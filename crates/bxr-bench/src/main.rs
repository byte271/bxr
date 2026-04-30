#![forbid(unsafe_code)]

use bxr_core::{Machine, MachineRunState};
use bxr_memory::PAGE_SIZE;
use std::time::Instant;

const NOPS: usize = 16 * 1024;
const HOT_LOOP_STEPS: usize = 20 * 1024;

fn main() {
    run_nop_sled();
    run_hot_loop();
}

fn run_nop_sled() {
    let mut machine = Machine::new_minimal(PAGE_SIZE * 8).expect("machine");
    let mut program = vec![0x90; NOPS];
    program.push(0xf4);
    machine.cpu.registers.set_rip(0x1000);
    machine.load_program(0x1000, &program).expect("program");

    let started = Instant::now();
    let steps = machine.run_until_halt(NOPS + 8).expect("run");
    let elapsed = started.elapsed();
    print_result("nop-sled", &machine, steps, elapsed);
}

fn run_hot_loop() {
    let mut machine = Machine::new_minimal(PAGE_SIZE * 8).expect("machine");
    machine.cpu.registers.set_rip(0x1000);
    machine
        .load_program(
            0x1000,
            &[
                0x90, // nop
                0xe9, 0xfa, 0xff, 0xff, 0xff, // jmp rel32 back to 0x1000
            ],
        )
        .expect("program");

    let started = Instant::now();
    let steps = machine.run_until_halt(HOT_LOOP_STEPS).expect("run");
    let elapsed = started.elapsed();
    print_result("hot-loop", &machine, steps, elapsed);
}

fn print_result(scenario: &str, machine: &Machine, steps: usize, elapsed: std::time::Duration) {
    let seconds = elapsed.as_secs_f64();
    let steps_per_second = if seconds == 0.0 {
        0.0
    } else {
        steps as f64 / seconds
    };

    println!("scenario={scenario}");
    println!("steps={steps}");
    println!("halted={}", machine.run_state == MachineRunState::Halted);
    println!("elapsed_ns={}", elapsed.as_nanos());
    println!("steps_per_second={steps_per_second:.2}");
    println!("trace_events={}", machine.trace.events().len());
    println!("trace_capacity=256");
    let cache = machine.decode_cache_stats();
    println!("decode_cache_entries={}", cache.entries);
    println!("decode_cache_hits={}", cache.hits);
    println!("decode_cache_misses={}", cache.misses);
    println!("decode_cache_invalidations={}", cache.invalidations);
}
