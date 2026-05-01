#![forbid(unsafe_code)]

use bxr_core::{Machine, MachineRunState};
use bxr_memory::PAGE_SIZE;

const SERIAL_WASM_HEX: &str = include_str!("../../../tests/guest-programs/serial-wasm.hex");

#[test]
fn serial_wasm_corpus_program_runs_to_halt() {
    let program = parse_hex_corpus(SERIAL_WASM_HEX);
    let mut machine = Machine::new_minimal(PAGE_SIZE).unwrap();
    machine.cpu.registers.set_rip(0x100);
    machine.load_program(0x100, &program).unwrap();

    let steps = machine.run_until_halt(64).unwrap();

    assert_eq!(steps, 11);
    assert_eq!(machine.run_state, MachineRunState::Halted);
    assert_eq!(machine.serial.output(), b"WASM\n");
    assert_eq!(machine.virtual_clock.ticks(), steps as u64);
}

fn parse_hex_corpus(input: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in input.lines() {
        let line = line.split_once('#').map_or(line, |(code, _)| code);
        for token in line.split_whitespace() {
            bytes.push(u8::from_str_radix(token, 16).expect("valid hex byte"));
        }
    }
    bytes
}
