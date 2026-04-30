use crate::decode::{Instruction, Operation};
use crate::mmu::MmuError;
use crate::system::ControlRegisters;
use crate::{Gpr, RFlags, RegisterFile, Width};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CpuState {
    pub registers: RegisterFile,
    pub rflags: RFlags,
    pub controls: ControlRegisters,
    pub halted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecuteOutcome {
    Continue,
    Halted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecuteError {
    UnsupportedOperation(&'static str),
    MemoryRead { addr: u64, width: Width },
    MemoryWrite { addr: u64, width: Width },
    PortWrite { port: u16, width: Width },
    Address(MmuError),
}

pub trait X86StackMemory {
    fn read_u64_le(&mut self, addr: u64) -> Result<u64, ExecuteError>;
    fn write_u64_le(&mut self, addr: u64, value: u64) -> Result<(), ExecuteError>;
}

pub trait X86PortIo {
    fn write_port_u8(&mut self, port: u16, value: u8) -> Result<(), ExecuteError>;
}

pub trait X86SystemBus: X86StackMemory + X86PortIo {}

impl<T> X86SystemBus for T where T: X86StackMemory + X86PortIo {}

impl CpuState {
    pub fn execute_decoded(
        &mut self,
        instruction: &Instruction,
    ) -> Result<ExecuteOutcome, ExecuteError> {
        match instruction.operation {
            Operation::Nop => {
                self.advance(instruction);
                Ok(ExecuteOutcome::Continue)
            }
            Operation::MovImm { dst, width, imm } => {
                self.registers.write_width(dst, width, imm);
                self.advance(instruction);
                Ok(ExecuteOutcome::Continue)
            }
            Operation::AddAccumulatorImm { width, imm32 } => {
                let lhs = self.registers.read_width(Gpr::Rax, width);
                let rhs = add_immediate_operand(width, imm32);
                let result = lhs.wrapping_add(rhs);
                self.registers.write_width(Gpr::Rax, width, result);
                self.rflags.update_add(width, lhs, rhs, result);
                self.advance(instruction);
                Ok(ExecuteOutcome::Continue)
            }
            Operation::JmpRel32 { rel } => {
                let base = self
                    .registers
                    .rip()
                    .wrapping_add(u64::from(instruction.len));
                self.registers
                    .set_rip(base.wrapping_add_signed(i64::from(rel)));
                Ok(ExecuteOutcome::Continue)
            }
            Operation::Hlt => {
                self.halted = true;
                self.advance(instruction);
                Ok(ExecuteOutcome::Halted)
            }
            Operation::Ret => Err(ExecuteError::UnsupportedOperation(
                "ret requires stack memory",
            )),
            Operation::OutImm8Al { .. } => Err(ExecuteError::UnsupportedOperation(
                "out requires port I/O bus",
            )),
            Operation::Int3 => Err(ExecuteError::UnsupportedOperation(
                "int3 requires exception delivery",
            )),
            Operation::Syscall => Err(ExecuteError::UnsupportedOperation(
                "syscall requires privilege transition state",
            )),
            Operation::Push { .. } | Operation::Pop { .. } => Err(
                ExecuteError::UnsupportedOperation("push/pop require stack memory"),
            ),
        }
    }

    pub fn execute_decoded_with_memory(
        &mut self,
        instruction: &Instruction,
        memory: &mut impl X86StackMemory,
    ) -> Result<ExecuteOutcome, ExecuteError> {
        match instruction.operation {
            Operation::Push { src } => {
                let value = self.registers.read(src);
                let new_rsp = self.registers.read(Gpr::Rsp).wrapping_sub(8);
                memory.write_u64_le(new_rsp, value)?;
                self.registers.write_width(Gpr::Rsp, Width::U64, new_rsp);
                self.advance(instruction);
                Ok(ExecuteOutcome::Continue)
            }
            Operation::Pop { dst } => {
                let old_rsp = self.registers.read(Gpr::Rsp);
                let value = memory.read_u64_le(old_rsp)?;
                self.registers
                    .write_width(Gpr::Rsp, Width::U64, old_rsp.wrapping_add(8));
                self.registers.write_width(dst, Width::U64, value);
                self.advance(instruction);
                Ok(ExecuteOutcome::Continue)
            }
            Operation::Ret => {
                let old_rsp = self.registers.read(Gpr::Rsp);
                let target = memory.read_u64_le(old_rsp)?;
                self.registers
                    .write_width(Gpr::Rsp, Width::U64, old_rsp.wrapping_add(8));
                self.registers.set_rip(target);
                Ok(ExecuteOutcome::Continue)
            }
            _ => self.execute_decoded(instruction),
        }
    }

    pub fn execute_decoded_with_bus(
        &mut self,
        instruction: &Instruction,
        bus: &mut impl X86SystemBus,
    ) -> Result<ExecuteOutcome, ExecuteError> {
        match instruction.operation {
            Operation::OutImm8Al { port } => {
                let value = self.registers.read_width(Gpr::Rax, Width::U8) as u8;
                bus.write_port_u8(u16::from(port), value)?;
                self.advance(instruction);
                Ok(ExecuteOutcome::Continue)
            }
            _ => self.execute_decoded_with_memory(instruction, bus),
        }
    }

    fn advance(&mut self, instruction: &Instruction) {
        self.registers.advance_rip(instruction.len);
    }
}

fn add_immediate_operand(width: Width, imm32: u32) -> u64 {
    match width {
        Width::U64 => i64::from(imm32 as i32) as u64,
        Width::U32 => u64::from(imm32),
        Width::U16 | Width::U8 => unreachable!("decoder only emits U32/U64 accumulator adds"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::decode_one;
    use crate::Flag;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct TestStackMemory {
        words: BTreeMap<u64, u64>,
    }

    impl X86StackMemory for TestStackMemory {
        fn read_u64_le(&mut self, addr: u64) -> Result<u64, ExecuteError> {
            self.words
                .get(&addr)
                .copied()
                .ok_or(ExecuteError::MemoryRead {
                    addr,
                    width: Width::U64,
                })
        }

        fn write_u64_le(&mut self, addr: u64, value: u64) -> Result<(), ExecuteError> {
            self.words.insert(addr, value);
            Ok(())
        }
    }

    impl X86PortIo for TestStackMemory {
        fn write_port_u8(&mut self, port: u16, value: u8) -> Result<(), ExecuteError> {
            self.words
                .insert(0xffff_0000 | u64::from(port), u64::from(value));
            Ok(())
        }
    }

    #[test]
    fn executes_mov_rax_imm64() {
        let inst = decode_one(&[0x48, 0xb8, 0x78, 0x56, 0x34, 0x12, 0, 0, 0, 0]).unwrap();
        let mut cpu = CpuState::default();

        assert_eq!(cpu.execute_decoded(&inst), Ok(ExecuteOutcome::Continue));
        assert_eq!(cpu.registers.read(Gpr::Rax), 0x1234_5678);
        assert_eq!(cpu.registers.rip(), 10);
    }

    #[test]
    fn executes_add_rax_imm32_with_sign_extension() {
        let mov = decode_one(&[0x48, 0xb8, 1, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        let add = decode_one(&[0x48, 0x05, 0xff, 0xff, 0xff, 0xff]).unwrap();
        let mut cpu = CpuState::default();

        cpu.execute_decoded(&mov).unwrap();
        cpu.execute_decoded(&add).unwrap();

        assert_eq!(cpu.registers.read(Gpr::Rax), 0);
        assert!(cpu.rflags.get(Flag::Zero));
    }

    #[test]
    fn executes_relative_jump_from_next_rip() {
        let jmp = decode_one(&[0xe9, 0x05, 0, 0, 0]).unwrap();
        let mut cpu = CpuState::default();
        cpu.registers.set_rip(0x1000);

        cpu.execute_decoded(&jmp).unwrap();

        assert_eq!(cpu.registers.rip(), 0x100a);
    }

    #[test]
    fn executes_hlt() {
        let hlt = decode_one(&[0xf4]).unwrap();
        let mut cpu = CpuState::default();

        assert_eq!(cpu.execute_decoded(&hlt), Ok(ExecuteOutcome::Halted));
        assert!(cpu.halted);
        assert_eq!(cpu.registers.rip(), 1);
    }

    #[test]
    fn executes_push_to_stack_memory() {
        let push = decode_one(&[0x50]).unwrap();
        let mut cpu = CpuState::default();
        let mut memory = TestStackMemory::default();
        cpu.registers.write_width(Gpr::Rax, Width::U64, 0xfeed);
        cpu.registers.write_width(Gpr::Rsp, Width::U64, 0x1000);

        cpu.execute_decoded_with_memory(&push, &mut memory).unwrap();

        assert_eq!(cpu.registers.read(Gpr::Rsp), 0x0ff8);
        assert_eq!(memory.words.get(&0x0ff8), Some(&0xfeed));
        assert_eq!(cpu.registers.rip(), 1);
    }

    #[test]
    fn executes_pop_from_stack_memory() {
        let pop = decode_one(&[0x58]).unwrap();
        let mut cpu = CpuState::default();
        let mut memory = TestStackMemory::default();
        cpu.registers.write_width(Gpr::Rsp, Width::U64, 0x0ff8);
        memory.words.insert(0x0ff8, 0xfeed);

        cpu.execute_decoded_with_memory(&pop, &mut memory).unwrap();

        assert_eq!(cpu.registers.read(Gpr::Rax), 0xfeed);
        assert_eq!(cpu.registers.read(Gpr::Rsp), 0x1000);
        assert_eq!(cpu.registers.rip(), 1);
    }

    #[test]
    fn executes_ret_from_stack_memory() {
        let ret = decode_one(&[0xc3]).unwrap();
        let mut cpu = CpuState::default();
        let mut memory = TestStackMemory::default();
        cpu.registers.write_width(Gpr::Rsp, Width::U64, 0x0ff8);
        memory.words.insert(0x0ff8, 0x1234);

        cpu.execute_decoded_with_memory(&ret, &mut memory).unwrap();

        assert_eq!(cpu.registers.rip(), 0x1234);
        assert_eq!(cpu.registers.read(Gpr::Rsp), 0x1000);
    }

    #[test]
    fn executes_out_imm8_al() {
        let out = decode_one(&[0xe6, 0xe9]).unwrap();
        let mut cpu = CpuState::default();
        let mut bus = TestStackMemory::default();
        cpu.registers.write_width(Gpr::Rax, Width::U64, 0x1234);

        cpu.execute_decoded_with_bus(&out, &mut bus).unwrap();

        assert_eq!(bus.words.get(&(0xffff_0000 | 0xe9)), Some(&0x34));
        assert_eq!(cpu.registers.rip(), 2);
    }
}
