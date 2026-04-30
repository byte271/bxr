// This crate is the browser ABI boundary. It intentionally uses `#[no_mangle]`
// exports so JavaScript can call a dependency-free WebAssembly module. Core
// emulator crates still inherit `unsafe_code = forbid`.

use bxr_core::{Machine, MachineRunState};
use bxr_memory::PAGE_SIZE;
use bxr_x86::{AccessType, Gpr};
use std::cell::RefCell;

thread_local! {
    static MACHINE: RefCell<Option<Machine>> = const { RefCell::new(None) };
    static SNAPSHOT: RefCell<Option<bxr_core::MachineSnapshot>> = const { RefCell::new(None) };
}

pub fn runtime_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn minimal_profile_id() -> &'static str {
    bxr_core::MINIMAL_X64_V1.id
}

#[no_mangle]
pub extern "C" fn bxr_abi_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn bxr_profile_id_code() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn bxr_demo_steps() -> u32 {
    run_demo().map(|result| result.steps as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_demo_halted() -> u32 {
    run_demo()
        .map(|result| u32::from(result.halted))
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_demo_rax() -> u32 {
    run_demo().map(|result| result.rax as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_demo_serial_len() -> u32 {
    run_demo()
        .map(|result| result.serial_len as u32)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_demo_serial_byte(index: u32) -> u32 {
    run_demo()
        .and_then(|result| result.serial.get(index as usize).copied())
        .map(u32::from)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_create_demo() -> u32 {
    let Some(machine) = create_demo_machine() else {
        return 0;
    };
    MACHINE.with(|slot| {
        *slot.borrow_mut() = Some(machine);
    });
    SNAPSHOT.with(|slot| {
        *slot.borrow_mut() = None;
    });
    1
}

#[no_mangle]
pub extern "C" fn bxr_machine_step() -> u32 {
    MACHINE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(machine) = slot.as_mut() else {
            return 0;
        };
        machine.step().map(|_| 1).unwrap_or(0)
    })
}

#[no_mangle]
pub extern "C" fn bxr_machine_run_until_halt(max_steps: u32) -> u32 {
    MACHINE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(machine) = slot.as_mut() else {
            return 0;
        };
        machine.run_until_halt(max_steps as usize).unwrap_or(0) as u32
    })
}

#[no_mangle]
pub extern "C" fn bxr_machine_state_code() -> u32 {
    with_machine(|machine| match machine.run_state {
        MachineRunState::Paused => 1,
        MachineRunState::Running => 2,
        MachineRunState::Halted => 3,
        MachineRunState::Faulted => 4,
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_gpr(index: u32) -> u64 {
    with_machine(|machine| {
        gpr_from_index(index)
            .map(|gpr| machine.cpu.registers.read(gpr))
            .unwrap_or(0)
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_rip() -> u64 {
    with_machine(|machine| machine.cpu.registers.rip()).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_rflags() -> u64 {
    with_machine(|machine| machine.cpu.rflags.bits()).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_control(index: u32) -> u64 {
    with_machine(|machine| match index {
        0 => machine.cpu.controls.cr0,
        2 => machine.cpu.controls.cr2,
        3 => machine.cpu.controls.cr3,
        4 => machine.cpu.controls.cr4,
        0x0efe => machine.cpu.controls.efer,
        _ => 0,
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_serial_len() -> u32 {
    with_machine(|machine| machine.serial.output().len() as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_serial_byte(index: u32) -> u32 {
    with_machine(|machine| {
        machine
            .serial
            .output()
            .get(index as usize)
            .copied()
            .map(u32::from)
            .unwrap_or(0)
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_snapshot_capture() -> u32 {
    MACHINE.with(|machine_slot| {
        let machine_slot = machine_slot.borrow();
        let Some(machine) = machine_slot.as_ref() else {
            return 0;
        };
        let snapshot = machine.capture_snapshot("bxr-wasm");
        SNAPSHOT.with(|snapshot_slot| {
            *snapshot_slot.borrow_mut() = Some(snapshot);
        });
        1
    })
}

#[no_mangle]
pub extern "C" fn bxr_machine_snapshot_restore() -> u32 {
    let snapshot = SNAPSHOT.with(|slot| slot.borrow().clone());
    let Some(snapshot) = snapshot else {
        return 0;
    };
    MACHINE.with(|slot| {
        *slot.borrow_mut() = Some(Machine::restore_snapshot(snapshot));
    });
    1
}

#[no_mangle]
pub extern "C" fn bxr_machine_snapshot_available() -> u32 {
    SNAPSHOT.with(|slot| u32::from(slot.borrow().is_some()))
}

#[no_mangle]
pub extern "C" fn bxr_machine_current_instruction_len() -> u32 {
    with_current_instruction(|instruction| u32::from(instruction.len)).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_current_instruction_code() -> u32 {
    with_current_instruction(|instruction| instruction.operation_code()).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_current_instruction_byte(index: u32) -> u32 {
    with_current_instruction(|instruction| {
        if index as usize >= usize::from(instruction.len) {
            return 0;
        }
        u32::from(instruction.bytes[index as usize])
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_memory_byte(physical_addr: u64) -> u32 {
    with_machine(|machine| {
        machine
            .memory
            .read_u8(physical_addr)
            .map(u32::from)
            .unwrap_or(0x100)
    })
    .unwrap_or(0x100)
}

#[no_mangle]
pub extern "C" fn bxr_machine_translate_execute(virtual_addr: u64) -> u64 {
    MACHINE.with(|slot| {
        let machine = slot.borrow();
        let Some(machine) = machine.as_ref() else {
            return u64::MAX;
        };
        let mut probe = machine.clone();
        probe
            .translate_address(virtual_addr, AccessType::Execute)
            .unwrap_or(u64::MAX)
    })
}

#[no_mangle]
pub extern "C" fn bxr_machine_trace_len() -> u32 {
    with_machine(|machine| machine.trace.events().len() as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_trace_sequence(index: u32) -> u64 {
    with_trace_event(index, |event| event.sequence).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_trace_rip_before(index: u32) -> u64 {
    with_trace_event(index, |event| event.rip_before).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_trace_rip_after(index: u32) -> u64 {
    with_trace_event(index, |event| event.rip_after).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_trace_operation_code(index: u32) -> u32 {
    with_trace_event(index, |event| event.operation_code).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_trace_outcome_code(index: u32) -> u32 {
    with_trace_event(index, |event| event.outcome_code).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_trace_instruction_len(index: u32) -> u32 {
    with_trace_event(index, |event| u32::from(event.instruction_len)).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_trace_instruction_byte(index: u32, byte_index: u32) -> u32 {
    with_trace_event(index, |event| {
        if byte_index as usize >= usize::from(event.instruction_len) {
            return 0;
        }
        u32::from(event.instruction_bytes[byte_index as usize])
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_decode_cache_entries() -> u32 {
    with_machine(|machine| machine.decode_cache_stats().entries as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_decode_cache_hits() -> u64 {
    with_machine(|machine| machine.decode_cache_stats().hits).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_decode_cache_misses() -> u64 {
    with_machine(|machine| machine.decode_cache_stats().misses).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn bxr_machine_decode_cache_invalidations() -> u64 {
    with_machine(|machine| machine.decode_cache_stats().invalidations).unwrap_or(0)
}

struct DemoResult {
    steps: usize,
    halted: bool,
    rax: u64,
    serial: Vec<u8>,
    serial_len: usize,
}

fn run_demo() -> Option<DemoResult> {
    let mut machine = create_demo_machine()?;
    let steps = machine.run_until_halt(64).ok()?;
    let serial = machine.serial.output().to_vec();
    let serial_len = serial.len();
    Some(DemoResult {
        steps,
        halted: machine.run_state == MachineRunState::Halted,
        rax: machine.cpu.registers.read(Gpr::Rax),
        serial,
        serial_len,
    })
}

fn create_demo_machine() -> Option<Machine> {
    let mut machine = Machine::new_minimal(PAGE_SIZE).ok()?;
    machine.cpu.registers.set_rip(0x100);
    machine
        .load_program(0x100, &demo_program_bytes(b"WASM\n"))
        .ok()?;
    Some(machine)
}

fn demo_program_bytes(text: &[u8]) -> Vec<u8> {
    let mut program = Vec::with_capacity(text.len() * 12 + 1);
    for byte in text {
        program.extend_from_slice(&[0x48, 0xb8, *byte, 0, 0, 0, 0, 0, 0, 0]);
        program.extend_from_slice(&[0xe6, 0xe9]);
    }
    program.push(0xf4);
    program
}

fn with_machine<T>(f: impl FnOnce(&Machine) -> T) -> Option<T> {
    MACHINE.with(|slot| slot.borrow().as_ref().map(f))
}

fn with_current_instruction<T>(f: impl FnOnce(&bxr_x86::decode::Instruction) -> T) -> Option<T> {
    MACHINE.with(|slot| {
        let machine = slot.borrow();
        let mut probe = machine.as_ref()?.clone();
        let instruction = probe.fetch_instruction().ok()?;
        Some(f(&instruction))
    })
}

fn with_trace_event<T>(index: u32, f: impl FnOnce(&bxr_core::TraceEvent) -> T) -> Option<T> {
    with_machine(|machine| machine.trace.events().get(index as usize).map(f)).flatten()
}

fn gpr_from_index(index: u32) -> Option<Gpr> {
    Some(match index {
        0 => Gpr::Rax,
        1 => Gpr::Rcx,
        2 => Gpr::Rdx,
        3 => Gpr::Rbx,
        4 => Gpr::Rsp,
        5 => Gpr::Rbp,
        6 => Gpr::Rsi,
        7 => Gpr::Rdi,
        8 => Gpr::R8,
        9 => Gpr::R9,
        10 => Gpr::R10,
        11 => Gpr::R11,
        12 => Gpr::R12,
        13 => Gpr::R13,
        14 => Gpr::R14,
        15 => Gpr::R15,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_demo_runs_core_machine() {
        assert_eq!(bxr_abi_version(), 1);
        assert_eq!(bxr_demo_steps(), 11);
        assert_eq!(bxr_demo_halted(), 1);
        assert_eq!(bxr_demo_rax(), u32::from(b'\n'));
        assert_eq!(bxr_demo_serial_len(), 5);
        assert_eq!(bxr_demo_serial_byte(0), u32::from(b'W'));
    }

    #[test]
    fn persistent_machine_steps_and_snapshots() {
        assert_eq!(bxr_machine_create_demo(), 1);
        assert_eq!(bxr_machine_state_code(), 1);
        assert_eq!(bxr_machine_step(), 1);
        assert_eq!(bxr_machine_gpr(0), u64::from(b'W'));
        assert_eq!(bxr_machine_decode_cache_entries(), 1);
        assert_eq!(bxr_machine_decode_cache_misses(), 1);
        assert_eq!(bxr_machine_snapshot_capture(), 1);
        assert_eq!(bxr_machine_snapshot_available(), 1);
        assert_eq!(bxr_machine_run_until_halt(64), 10);
        assert_eq!(bxr_machine_state_code(), 3);
        assert_eq!(bxr_machine_serial_len(), 5);
        assert_eq!(bxr_machine_snapshot_restore(), 1);
        assert_eq!(bxr_machine_state_code(), 1);
        assert_eq!(bxr_machine_serial_len(), 0);
        assert_eq!(bxr_machine_rip(), 0x10a);
        assert_eq!(bxr_machine_decode_cache_entries(), 0);
        assert_eq!(bxr_machine_current_instruction_len(), 2);
        assert_eq!(bxr_machine_current_instruction_code(), 11);
        assert_eq!(bxr_machine_current_instruction_byte(0), 0xe6);
        assert_eq!(bxr_machine_memory_byte(0x100), 0x48);
        assert_eq!(bxr_machine_translate_execute(0x100), 0x100);
        assert_eq!(bxr_machine_trace_len(), 1);
        assert_eq!(bxr_machine_trace_operation_code(0), 6);
        assert_eq!(bxr_machine_trace_instruction_len(0), 10);
        assert_eq!(bxr_machine_trace_instruction_byte(0, 0), 0x48);
    }

    #[test]
    fn instruction_byte_bounds_are_zero() {
        assert_eq!(bxr_machine_create_demo(), 1);
        assert_eq!(
            bxr_machine_current_instruction_byte(bxr_x86::decode::MAX_INSTRUCTION_LEN as u32),
            0
        );
    }
}
