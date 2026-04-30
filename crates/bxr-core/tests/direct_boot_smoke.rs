use bxr_boot::{apply_direct_x64_boot_state, DirectX64BootState};
use bxr_core::{Machine, MachineRunState};
use bxr_memory::PAGE_SIZE;
use bxr_x86::Gpr;

#[test]
fn direct_boot_program_runs_until_hlt() {
    let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
    apply_direct_x64_boot_state(
        &mut machine.cpu.registers,
        &mut machine.cpu.rflags,
        DirectX64BootState {
            entry: 0x200,
            stack_top: 0x1000,
        },
    );

    machine
        .load_program(
            0x200,
            &[
                0x48, 0xb8, 0x34, 0x12, 0, 0, 0, 0, 0, 0, // mov rax, 0x1234
                0x48, 0x05, 0x10, 0, 0, 0,    // add rax, 0x10
                0xf4, // hlt
            ],
        )
        .unwrap();

    let steps = machine.run_until_halt(16).unwrap();

    assert_eq!(steps, 3);
    assert_eq!(machine.run_state, MachineRunState::Halted);
    assert_eq!(machine.cpu.registers.read(Gpr::Rax), 0x1244);
    assert_eq!(machine.cpu.registers.read(Gpr::Rsp), 0x1000);
}

#[test]
fn direct_boot_program_can_return_through_guest_stack() {
    let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
    apply_direct_x64_boot_state(
        &mut machine.cpu.registers,
        &mut machine.cpu.rflags,
        DirectX64BootState {
            entry: 0x200,
            stack_top: 0x1000,
        },
    );

    machine
        .load_program(
            0x200,
            &[
                0x48, 0xb8, 0x10, 0x02, 0, 0, 0, 0, 0, 0,    // mov rax, 0x210
                0x50, // push rax
                0xc3, // ret
                0x90, 0x90, 0x90, 0x90, // padding
                0xf4, // hlt at 0x210
            ],
        )
        .unwrap();

    let steps = machine.run_until_halt(16).unwrap();

    assert_eq!(steps, 4);
    assert_eq!(machine.run_state, MachineRunState::Halted);
    assert_eq!(machine.cpu.registers.rip(), 0x211);
    assert_eq!(machine.cpu.registers.read(Gpr::Rsp), 0x1000);
}

#[test]
fn direct_boot_program_can_write_serial_debug_port() {
    let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
    apply_direct_x64_boot_state(
        &mut machine.cpu.registers,
        &mut machine.cpu.rflags,
        DirectX64BootState {
            entry: 0x200,
            stack_top: 0x1000,
        },
    );

    machine
        .load_program(
            0x200,
            &[
                0x48, 0xb8, b'B', 0, 0, 0, 0, 0, 0, 0, // mov rax, 'B'
                0xe6, 0xe9, // out 0xe9, al
                0xf4, // hlt
            ],
        )
        .unwrap();

    let steps = machine.run_until_halt(16).unwrap();

    assert_eq!(steps, 3);
    assert_eq!(machine.run_state, MachineRunState::Halted);
    assert_eq!(machine.serial.output(), b"B");
}
