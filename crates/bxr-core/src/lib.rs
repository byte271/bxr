#![forbid(unsafe_code)]

use bxr_devices::{Device, SerialDevice};
use bxr_memory::{MemoryError, PhysicalMemory, PAGE_SIZE};
use bxr_snapshot::{ChunkKind, ChunkRef, SnapshotManifest};
use bxr_x86::decode::{decode_one, DecodeError, Instruction, MAX_INSTRUCTION_LEN};
use bxr_x86::{
    translate, AccessType, CpuState, ExecuteError, ExecuteOutcome, Gpr, MmuError, PageTableMemory,
    PrivilegeLevel, TranslateRequest, Width, X86PortIo, X86StackMemory,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineProfile {
    pub id: &'static str,
    pub cpuid_profile: &'static str,
    pub default_ram_bytes: usize,
    pub devices: &'static [&'static str],
}

pub const MINIMAL_X64_V1: MachineProfile = MachineProfile {
    id: "bxr-minimal-x64-v1",
    cpuid_profile: "bxr-x64-conservative-v1",
    default_ram_bytes: 128 * 1024 * 1024,
    devices: &["serial0", "virtual-clock0"],
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachineRunState {
    Paused,
    Running,
    Halted,
    Faulted,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VirtualClock {
    ticks: u64,
}

impl VirtualClock {
    pub const fn ticks(self) -> u64 {
        self.ticks
    }

    fn advance_instruction(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachineError {
    Memory(MemoryError),
    Decode(DecodeError),
    Execute(ExecuteError),
    Mmu(MmuError),
    AlreadyHalted,
}

impl From<MemoryError> for MachineError {
    fn from(error: MemoryError) -> Self {
        Self::Memory(error)
    }
}

impl From<DecodeError> for MachineError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error)
    }
}

impl From<ExecuteError> for MachineError {
    fn from(error: ExecuteError) -> Self {
        Self::Execute(error)
    }
}

impl From<MmuError> for MachineError {
    fn from(error: MmuError) -> Self {
        Self::Mmu(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepReport {
    pub rip_before: u64,
    pub instruction: Instruction,
    pub outcome: ExecuteOutcome,
}

const TRACE_CAPACITY: usize = 256;
const DECODE_CACHE_CAPACITY: usize = 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub sequence: u64,
    pub rip_before: u64,
    pub rip_after: u64,
    pub virtual_ticks_after: u64,
    pub instruction_len: u8,
    pub instruction_bytes: [u8; MAX_INSTRUCTION_LEN],
    pub operation_code: u32,
    pub outcome_code: u32,
    pub rax_after: u64,
    pub rsp_after: u64,
    pub serial_len_after: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TraceLog {
    next_sequence: u64,
    events: Vec<TraceEvent>,
}

impl TraceLog {
    pub fn events(&self) -> &[TraceEvent] {
        &self.events
    }

    fn push(&mut self, mut event: TraceEvent) {
        event.sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        if self.events.len() == TRACE_CAPACITY {
            self.events.remove(0);
        }
        self.events.push(event);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DecodeCacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub invalidations: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodeCacheEntry {
    physical_rip: u64,
    page_index: usize,
    page_generation: u32,
    instruction: Instruction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodeCache {
    slots: Vec<Option<DecodeCacheEntry>>,
    entries: usize,
    hits: u64,
    misses: u64,
    invalidations: u64,
}

impl Default for DecodeCache {
    fn default() -> Self {
        Self {
            slots: vec![None; DECODE_CACHE_CAPACITY],
            entries: 0,
            hits: 0,
            misses: 0,
            invalidations: 0,
        }
    }
}

impl DecodeCache {
    fn stats(&self) -> DecodeCacheStats {
        DecodeCacheStats {
            entries: self.entries,
            hits: self.hits,
            misses: self.misses,
            invalidations: self.invalidations,
        }
    }

    fn get(
        &mut self,
        physical_rip: u64,
        page_index: usize,
        page_generation: u32,
    ) -> Option<Instruction> {
        let slot_index = self.slot_index(physical_rip);
        let entry = self.slots[slot_index].as_ref()?;
        if entry.physical_rip != physical_rip {
            return None;
        }
        if entry.page_index == page_index && entry.page_generation == page_generation {
            self.hits = self.hits.wrapping_add(1);
            return Some(entry.instruction.clone());
        }

        self.invalidations = self.invalidations.wrapping_add(1);
        self.slots[slot_index] = None;
        self.entries = self.entries.saturating_sub(1);
        None
    }

    fn record_miss(&mut self) {
        self.misses = self.misses.wrapping_add(1);
    }

    fn insert(
        &mut self,
        physical_rip: u64,
        page_index: usize,
        page_generation: u32,
        instruction: Instruction,
    ) {
        let slot_index = self.slot_index(physical_rip);
        if self.slots[slot_index].is_none() {
            self.entries += 1;
        }
        self.slots[slot_index] = Some(DecodeCacheEntry {
            physical_rip,
            page_index,
            page_generation,
            instruction,
        });
    }

    fn invalidate_range(&mut self, start_addr: u64, len: usize) {
        if len == 0 || self.entries == 0 {
            return;
        }

        let Some(start) = usize::try_from(start_addr).ok() else {
            let removed = self.entries as u64;
            self.clear_entries();
            self.invalidations = self.invalidations.wrapping_add(removed);
            return;
        };
        let Some(end) = start.checked_add(len) else {
            let removed = self.entries as u64;
            self.clear_entries();
            self.invalidations = self.invalidations.wrapping_add(removed);
            return;
        };

        let mut removed = 0_u64;
        for slot in &mut self.slots {
            let remove = match slot.as_ref() {
                Some(entry) => match usize::try_from(entry.physical_rip) {
                    Ok(entry_start) => {
                        let entry_end =
                            entry_start.saturating_add(usize::from(entry.instruction.len));
                        ranges_overlap(start, end, entry_start, entry_end)
                    }
                    Err(_) => true,
                },
                None => false,
            };
            if remove {
                *slot = None;
                removed = removed.wrapping_add(1);
            }
        }
        self.entries = self.entries.saturating_sub(removed as usize);
        self.invalidations = self.invalidations.wrapping_add(removed);
    }

    fn clear_entries(&mut self) {
        self.slots.fill(None);
        self.entries = 0;
    }

    fn slot_index(&self, physical_rip: u64) -> usize {
        physical_rip as usize % self.slots.len()
    }
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start < b_end && b_start < a_end
}

struct MachinePageTableMemory<'a> {
    memory: &'a PhysicalMemory,
}

impl PageTableMemory for MachinePageTableMemory<'_> {
    fn read_u64_phys(&self, addr: u64) -> Result<u64, MmuError> {
        self.memory
            .read_u64_le(addr)
            .map_err(|_| MmuError::PhysicalRead { addr })
    }
}

struct MachineBus<'a> {
    memory: &'a mut PhysicalMemory,
    serial: &'a mut SerialDevice,
    controls: bxr_x86::ControlRegisters,
}

impl X86StackMemory for MachineBus<'_> {
    fn read_u64_le(&mut self, addr: u64) -> Result<u64, ExecuteError> {
        let mut bytes = [0; 8];
        for (offset, byte) in bytes.iter_mut().enumerate() {
            let virtual_addr = addr.wrapping_add(offset as u64);
            let physical_addr = self
                .translate(virtual_addr, AccessType::Read)
                .map_err(ExecuteError::Address)?;
            *byte = self
                .memory
                .read_u8(physical_addr)
                .map_err(|_| ExecuteError::MemoryRead {
                    addr: virtual_addr,
                    width: Width::U8,
                })?;
        }
        Ok(u64::from_le_bytes(bytes))
    }

    fn write_u64_le(&mut self, addr: u64, value: u64) -> Result<(), ExecuteError> {
        for (offset, byte) in value.to_le_bytes().iter().copied().enumerate() {
            let virtual_addr = addr.wrapping_add(offset as u64);
            let physical_addr = self
                .translate(virtual_addr, AccessType::Write)
                .map_err(ExecuteError::Address)?;
            self.memory
                .write_u8(physical_addr, byte)
                .map_err(|_| ExecuteError::MemoryWrite {
                    addr: virtual_addr,
                    width: Width::U8,
                })?;
        }
        Ok(())
    }
}

impl MachineBus<'_> {
    fn translate(&self, virtual_addr: u64, access: AccessType) -> Result<u64, MmuError> {
        let page_tables = MachinePageTableMemory {
            memory: &*self.memory,
        };
        Ok(translate(
            self.controls,
            &page_tables,
            TranslateRequest {
                virtual_addr,
                access,
                privilege: PrivilegeLevel::Supervisor,
            },
        )?
        .physical_addr)
    }
}

impl X86PortIo for MachineBus<'_> {
    fn write_port_u8(&mut self, port: u16, value: u8) -> Result<(), ExecuteError> {
        match port {
            SerialDevice::DEBUG_CONSOLE_PORT | SerialDevice::COM1_DATA_PORT => {
                self.serial.write_byte(value);
                Ok(())
            }
            _ => Err(ExecuteError::PortWrite {
                port,
                width: Width::U8,
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineSnapshot {
    pub manifest: SnapshotManifest,
    pub profile: MachineProfile,
    pub cpu: CpuState,
    pub memory: PhysicalMemory,
    pub serial: SerialDevice,
    pub virtual_clock: VirtualClock,
    pub run_state: MachineRunState,
    pub trace: TraceLog,
}

#[derive(Clone, Debug)]
pub struct Machine {
    pub profile: MachineProfile,
    pub cpu: CpuState,
    pub memory: PhysicalMemory,
    pub serial: SerialDevice,
    pub virtual_clock: VirtualClock,
    pub run_state: MachineRunState,
    pub trace: TraceLog,
    decode_cache: DecodeCache,
}

impl Machine {
    pub fn new_minimal(ram_bytes: usize) -> Result<Self, bxr_memory::MemoryError> {
        Ok(Self {
            profile: MINIMAL_X64_V1,
            cpu: CpuState::default(),
            memory: PhysicalMemory::new(ram_bytes)?,
            serial: SerialDevice::default(),
            virtual_clock: VirtualClock::default(),
            run_state: MachineRunState::Paused,
            trace: TraceLog::default(),
            decode_cache: DecodeCache::default(),
        })
    }

    pub fn pause(&mut self) {
        if self.run_state == MachineRunState::Running {
            self.run_state = MachineRunState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.run_state == MachineRunState::Paused {
            self.run_state = MachineRunState::Running;
        }
    }

    pub fn snapshot_manifest(&self, created_by: impl Into<String>) -> SnapshotManifest {
        let mut manifest = SnapshotManifest::new(self.profile.id, created_by);
        manifest.add_chunk(ChunkRef::from_bytes(
            ChunkKind::Cpu,
            &self.cpu_chunk_bytes(),
        ));
        manifest.add_chunk(ChunkRef::from_bytes(
            ChunkKind::MemoryPage,
            self.memory.as_bytes(),
        ));
        manifest.add_chunk(ChunkRef::from_bytes(
            ChunkKind::Device,
            &self.serial.snapshot().payload,
        ));
        manifest.add_chunk(ChunkRef::from_bytes(
            ChunkKind::Scheduler,
            &self.virtual_clock.ticks().to_le_bytes(),
        ));
        manifest
    }

    pub fn capture_snapshot(&self, created_by: impl Into<String>) -> MachineSnapshot {
        MachineSnapshot {
            manifest: self.snapshot_manifest(created_by),
            profile: self.profile,
            cpu: self.cpu.clone(),
            memory: self.memory.clone(),
            serial: self.serial.clone(),
            virtual_clock: self.virtual_clock,
            run_state: self.run_state,
            trace: self.trace.clone(),
        }
    }

    pub fn restore_snapshot(snapshot: MachineSnapshot) -> Self {
        Self {
            profile: snapshot.profile,
            cpu: snapshot.cpu,
            memory: snapshot.memory,
            serial: snapshot.serial,
            virtual_clock: snapshot.virtual_clock,
            run_state: snapshot.run_state,
            trace: snapshot.trace,
            decode_cache: DecodeCache::default(),
        }
    }

    pub fn load_program(&mut self, addr: u64, bytes: &[u8]) -> Result<(), MachineError> {
        self.memory.write(addr, bytes)?;
        self.memory.mark_executable_range(addr, bytes.len())?;
        self.decode_cache.invalidate_range(addr, bytes.len());
        Ok(())
    }

    pub fn fetch_instruction(&mut self) -> Result<Instruction, MachineError> {
        let rip = self.cpu.registers.rip();
        let cache_key = self.decode_cache_key(rip)?;
        if let Some((physical_rip, page_index, page_generation)) = cache_key {
            if let Some(instruction) =
                self.decode_cache
                    .get(physical_rip, page_index, page_generation)
            {
                return Ok(instruction);
            }
        }

        self.decode_cache.record_miss();
        let mut bytes = [0; MAX_INSTRUCTION_LEN];
        for (offset, byte) in bytes.iter_mut().enumerate() {
            let virtual_addr = rip.wrapping_add(offset as u64);
            let physical_addr = self.translate_address(virtual_addr, AccessType::Execute)?;
            *byte = self.memory.read_u8(physical_addr)?;
        }
        let instruction = decode_one(&bytes)?;
        if let Some((physical_rip, page_index, page_generation)) = cache_key {
            self.decode_cache.insert(
                physical_rip,
                page_index,
                page_generation,
                instruction.clone(),
            );
        }
        Ok(instruction)
    }

    pub fn decode_cache_stats(&self) -> DecodeCacheStats {
        self.decode_cache.stats()
    }

    pub fn step(&mut self) -> Result<StepReport, MachineError> {
        if self.run_state == MachineRunState::Halted || self.cpu.halted {
            return Err(MachineError::AlreadyHalted);
        }

        let report = self.step_inner();
        if report.is_err() {
            if let Err(error) = &report {
                self.record_fault_address(error);
            }
            self.run_state = MachineRunState::Faulted;
        }
        report
    }

    fn step_inner(&mut self) -> Result<StepReport, MachineError> {
        let rip_before = self.cpu.registers.rip();
        let instruction = self.fetch_instruction()?;
        let outcome = {
            let mut bus = MachineBus {
                memory: &mut self.memory,
                serial: &mut self.serial,
                controls: self.cpu.controls,
            };
            self.cpu.execute_decoded_with_bus(&instruction, &mut bus)?
        };
        self.virtual_clock.advance_instruction();
        if outcome == ExecuteOutcome::Halted {
            self.run_state = MachineRunState::Halted;
        }

        let report = StepReport {
            rip_before,
            instruction,
            outcome,
        };
        self.record_trace(&report);
        Ok(report)
    }

    pub fn run_until_halt(&mut self, max_steps: usize) -> Result<usize, MachineError> {
        self.resume();
        let mut steps = 0;
        while self.run_state == MachineRunState::Running && steps < max_steps {
            self.step()?;
            steps += 1;
        }
        if self.run_state == MachineRunState::Running {
            self.pause();
        }
        Ok(steps)
    }

    pub fn translate_address(
        &mut self,
        virtual_addr: u64,
        access: AccessType,
    ) -> Result<u64, MachineError> {
        let page_tables = MachinePageTableMemory {
            memory: &self.memory,
        };
        match translate(
            self.cpu.controls,
            &page_tables,
            TranslateRequest {
                virtual_addr,
                access,
                privilege: PrivilegeLevel::Supervisor,
            },
        ) {
            Ok(translation) => Ok(translation.physical_addr),
            Err(error) => {
                if let MmuError::PageFault(page_fault) = error {
                    self.cpu.controls.set_page_fault_address(page_fault.addr);
                }
                Err(MachineError::Mmu(error))
            }
        }
    }

    fn decode_cache_key(
        &mut self,
        virtual_rip: u64,
    ) -> Result<Option<(u64, usize, u32)>, MachineError> {
        let physical_rip = self.translate_address(virtual_rip, AccessType::Execute)?;
        let Some(physical_rip_usize) = usize::try_from(physical_rip).ok() else {
            return Ok(None);
        };
        if physical_rip_usize % PAGE_SIZE + MAX_INSTRUCTION_LEN > PAGE_SIZE {
            return Ok(None);
        }

        let Some(page_index) = self.memory.page_index_for_addr(physical_rip) else {
            return Ok(None);
        };
        let Some(page_generation) = self.memory.page_generation(page_index) else {
            return Ok(None);
        };
        Ok(Some((physical_rip, page_index, page_generation)))
    }

    fn record_fault_address(&mut self, error: &MachineError) {
        match error {
            MachineError::Mmu(MmuError::PageFault(page_fault))
            | MachineError::Execute(ExecuteError::Address(MmuError::PageFault(page_fault))) => {
                self.cpu.controls.set_page_fault_address(page_fault.addr);
            }
            _ => {}
        }
    }

    fn record_trace(&mut self, report: &StepReport) {
        self.trace.push(TraceEvent {
            sequence: 0,
            rip_before: report.rip_before,
            rip_after: self.cpu.registers.rip(),
            virtual_ticks_after: self.virtual_clock.ticks(),
            instruction_len: report.instruction.len,
            instruction_bytes: report.instruction.bytes,
            operation_code: report.instruction.operation_code(),
            outcome_code: match report.outcome {
                ExecuteOutcome::Continue => 1,
                ExecuteOutcome::Halted => 2,
            },
            rax_after: self.cpu.registers.read(bxr_x86::Gpr::Rax),
            rsp_after: self.cpu.registers.read(bxr_x86::Gpr::Rsp),
            serial_len_after: self.serial.output().len(),
        });
    }

    fn cpu_chunk_bytes(&self) -> Vec<u8> {
        const GPRS: [Gpr; 16] = [
            Gpr::Rax,
            Gpr::Rcx,
            Gpr::Rdx,
            Gpr::Rbx,
            Gpr::Rsp,
            Gpr::Rbp,
            Gpr::Rsi,
            Gpr::Rdi,
            Gpr::R8,
            Gpr::R9,
            Gpr::R10,
            Gpr::R11,
            Gpr::R12,
            Gpr::R13,
            Gpr::R14,
            Gpr::R15,
        ];

        let mut bytes = Vec::with_capacity(16 * 8 + 8 * 7 + 1);
        for gpr in GPRS {
            bytes.extend_from_slice(&self.cpu.registers.read(gpr).to_le_bytes());
        }
        bytes.extend_from_slice(&self.cpu.registers.rip().to_le_bytes());
        bytes.extend_from_slice(&self.cpu.rflags.bits().to_le_bytes());
        bytes.extend_from_slice(&self.cpu.controls.cr0.to_le_bytes());
        bytes.extend_from_slice(&self.cpu.controls.cr2.to_le_bytes());
        bytes.extend_from_slice(&self.cpu.controls.cr3.to_le_bytes());
        bytes.extend_from_slice(&self.cpu.controls.cr4.to_le_bytes());
        bytes.extend_from_slice(&self.cpu.controls.efer.to_le_bytes());
        bytes.push(u8::from(self.cpu.halted));
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bxr_memory::PAGE_SIZE;
    use bxr_x86::decode::Operation;
    use bxr_x86::{ControlRegisters, Flag, Gpr};

    #[test]
    fn minimal_machine_starts_paused() {
        let machine = Machine::new_minimal(PAGE_SIZE).unwrap();
        assert_eq!(machine.profile.id, "bxr-minimal-x64-v1");
        assert_eq!(machine.run_state, MachineRunState::Paused);
        assert_eq!(machine.virtual_clock.ticks(), 0);
    }

    #[test]
    fn resume_and_pause_transitions_are_explicit() {
        let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
        machine.resume();
        assert_eq!(machine.run_state, MachineRunState::Running);
        machine.pause();
        assert_eq!(machine.run_state, MachineRunState::Paused);
    }

    #[test]
    fn steps_program_loaded_in_guest_memory() {
        let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
        machine.cpu.registers.set_rip(0x100);
        machine
            .load_program(
                0x100,
                &[
                    0x48, 0xb8, 0x2a, 0, 0, 0, 0, 0, 0, 0, // mov rax, 42
                    0x48, 0x05, 0x01, 0, 0, 0,    // add rax, 1
                    0xf4, // hlt
                ],
            )
            .unwrap();

        let steps = machine.run_until_halt(8).unwrap();

        assert_eq!(steps, 3);
        assert_eq!(machine.run_state, MachineRunState::Halted);
        assert_eq!(machine.cpu.registers.read(Gpr::Rax), 43);
        assert!(!machine.cpu.rflags.get(Flag::Zero));
        assert_eq!(machine.virtual_clock.ticks(), 3);
        assert_eq!(machine.trace.events().len(), 3);
        assert_eq!(machine.trace.events()[0].rip_before, 0x100);
        assert_eq!(machine.trace.events()[0].virtual_ticks_after, 1);
        assert_eq!(machine.trace.events()[2].outcome_code, 2);
    }

    #[test]
    fn reports_decode_errors_from_guest_memory() {
        let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
        machine.cpu.registers.set_rip(0x20);
        machine.load_program(0x20, &[0x0f, 0xff]).unwrap();

        assert_eq!(
            machine.step(),
            Err(MachineError::Decode(DecodeError::UnsupportedOpcode {
                opcode: 0xff,
                offset: 1
            }))
        );
        assert_eq!(machine.run_state, MachineRunState::Faulted);
    }

    #[test]
    fn machine_fetch_uses_long_mode_page_tables() {
        let mut machine = Machine::new_minimal(PAGE_SIZE * 16).unwrap();
        machine.memory.write_u64_le(0x1000, 0x2000 | 0b111).unwrap();
        machine.memory.write_u64_le(0x2000, 0x3000 | 0b111).unwrap();
        machine
            .memory
            .write_u64_le(0x3000 + 2 * 8, 0x4000 | 0b111)
            .unwrap();
        machine.memory.write_u64_le(0x4000, 0x8000 | 0b111).unwrap();
        machine.cpu.controls = ControlRegisters {
            cr0: ControlRegisters::CR0_PG,
            cr3: 0x1000,
            cr4: ControlRegisters::CR4_PAE,
            efer: ControlRegisters::EFER_LME | ControlRegisters::EFER_LMA,
            ..ControlRegisters::default()
        };
        machine.cpu.registers.set_rip(0x400000);
        machine.load_program(0x8000, &[0xf4]).unwrap();

        let steps = machine.run_until_halt(4).unwrap();

        assert_eq!(steps, 1);
        assert_eq!(machine.run_state, MachineRunState::Halted);
    }

    #[test]
    fn fetch_instruction_reuses_decode_cache() {
        let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
        machine.cpu.registers.set_rip(0x100);
        machine.load_program(0x100, &[0x90, 0xf4]).unwrap();

        let first = machine.fetch_instruction().unwrap();
        let first_stats = machine.decode_cache_stats();
        let second = machine.fetch_instruction().unwrap();
        let second_stats = machine.decode_cache_stats();

        assert_eq!(first.operation, Operation::Nop);
        assert_eq!(second.operation, Operation::Nop);
        assert_eq!(first_stats.entries, 1);
        assert_eq!(first_stats.misses, 1);
        assert_eq!(first_stats.hits, 0);
        assert_eq!(second_stats.entries, 1);
        assert_eq!(second_stats.misses, 1);
        assert_eq!(second_stats.hits, 1);
    }

    #[test]
    fn executable_write_invalidates_decode_cache() {
        let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
        machine.cpu.registers.set_rip(0x100);
        machine.load_program(0x100, &[0x90, 0xf4]).unwrap();
        assert_eq!(
            machine.fetch_instruction().unwrap().operation,
            Operation::Nop
        );
        assert_eq!(machine.decode_cache_stats().entries, 1);

        machine.load_program(0x100, &[0xf4]).unwrap();
        let after_write = machine.decode_cache_stats();
        let fetched = machine.fetch_instruction().unwrap();
        let after_fetch = machine.decode_cache_stats();

        assert_eq!(after_write.entries, 0);
        assert_eq!(after_write.invalidations, 1);
        assert_eq!(fetched.operation, Operation::Hlt);
        assert_eq!(after_fetch.entries, 1);
        assert_eq!(after_fetch.misses, 2);
    }

    #[test]
    fn page_fault_records_cr2_and_faults_machine() {
        let mut machine = Machine::new_minimal(PAGE_SIZE * 4).unwrap();
        machine.cpu.controls = ControlRegisters {
            cr0: ControlRegisters::CR0_PG,
            cr3: 0x1000,
            cr4: ControlRegisters::CR4_PAE,
            efer: ControlRegisters::EFER_LME | ControlRegisters::EFER_LMA,
            ..ControlRegisters::default()
        };
        machine.cpu.registers.set_rip(0x400000);

        assert!(matches!(machine.step(), Err(MachineError::Mmu(_))));
        assert_eq!(machine.cpu.controls.cr2, 0x400000);
        assert_eq!(machine.run_state, MachineRunState::Faulted);
    }

    #[test]
    fn machine_snapshot_round_trips_execution_state() {
        let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
        machine.cpu.registers.set_rip(0x100);
        machine
            .load_program(
                0x100,
                &[
                    0x48, 0xb8, b'A', 0, 0, 0, 0, 0, 0, 0, // mov rax, 'A'
                    0xe6, 0xe9, // out 0xe9, al
                    0xf4, // hlt
                ],
            )
            .unwrap();
        machine.step().unwrap();
        let snapshot = machine.capture_snapshot("test");

        machine.run_until_halt(8).unwrap();
        assert_eq!(machine.serial.output(), b"A");

        let mut restored = Machine::restore_snapshot(snapshot);
        assert_eq!(restored.decode_cache_stats().entries, 0);
        restored.run_until_halt(8).unwrap();

        assert_eq!(restored.serial.output(), b"A");
        assert_eq!(restored.run_state, MachineRunState::Halted);
        assert_eq!(restored.cpu.registers.rip(), 0x10d);
        assert_eq!(restored.virtual_clock.ticks(), 3);
        assert_eq!(restored.trace.events().len(), 3);
    }

    #[test]
    fn snapshot_manifest_records_content_chunks() {
        let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
        let before = machine.snapshot_manifest("test");

        machine.cpu.registers.set_rip(0x100);
        machine.load_program(0x100, &[0x90, 0xf4]).unwrap();
        machine.step().unwrap();
        let after = machine.snapshot_manifest("test");

        assert_eq!(before.chunks.len(), 4);
        assert_eq!(
            before
                .chunks
                .iter()
                .map(|chunk| chunk.kind)
                .collect::<Vec<_>>(),
            vec![
                ChunkKind::Cpu,
                ChunkKind::MemoryPage,
                ChunkKind::Device,
                ChunkKind::Scheduler
            ]
        );
        assert_ne!(before.chunks[0].hash, after.chunks[0].hash);
        assert_ne!(before.chunks[1].hash, after.chunks[1].hash);
        assert_ne!(before.chunks[3].hash, after.chunks[3].hash);
    }
}
