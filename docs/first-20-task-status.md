# First 20 Task Status

This tracks the first 20 implementation tasks from the architecture proposal.

## Completed

1. Pick project name and write charter: `BXR` in `README.md`.
2. Define `bxr-minimal-x64-v1`: `docs/specs/machine-profile-v1.md`.
3. Define snapshot manifest v0/v1 draft: `docs/specs/snapshot-format-v1.md`.
4. Create Rust workspace and web shell: root `Cargo.toml`, `crates/*`, `web/*`.
5. Implement register file: `crates/bxr-x86/src/registers.rs`.
6. Implement decoder scaffold with opcode metadata: `crates/bxr-x86/src/decode/mod.rs`.
7. Implement basic moves/arithmetic/branches/stack: `mov`, `add`, `jmp`, `push`, `pop`, `ret`, `hlt`.
8. Add exact flag tests for implemented arithmetic: `crates/bxr-x86/src/flags.rs`.
9. Add native CPU test harness: Rust unit tests in `bxr-x86`.
10. Build tiny x86-64 test program path: direct-boot smoke tests execute guest bytes from memory.
11. Implement direct boot loader for tiny kernel/program: `crates/bxr-boot`.
12. Implement serial device: `crates/bxr-devices`, plus `out 0xe9, al` wiring.
13. Compile core to Wasm: `cargo check -p bxr-wasm --target wasm32-unknown-unknown`.
14. Create machine worker protocol: `docs/specs/worker-protocol-v1.md` and `web/src/machine.worker.js`.
15. Render serial terminal in browser UI: `web/index.html` and `web/src/main.js`; the worker can call the Rust/Wasm demo exports when `web/wasm/bxr_wasm.wasm` is built.
16. Add page-backed physical memory with dirty tracking: `crates/bxr-memory`.
17. Implement CR0/CR3/CR4/EFER and page walks: `crates/bxr-x86/src/system.rs` and `mmu.rs`.
18. Implement page fault detection path: MMU page-fault errors set `CR2` in machine tests.
19. Add pause/resume: `Machine::pause`, `Machine::resume`, `run_until_halt`.
20. Implement first CPU/RAM/device snapshot round trip: `MachineSnapshot` in `bxr-core`.

## Still Shallow

- The browser shell is Wasm-backed for a tiny persistent built-in machine, not a general machine loader yet.
- The snapshot is in-memory clone state, not the final manifest/chunk/export format.
- The page walker does not yet update accessed/dirty bits.
- There is no interrupt/IDT delivery, privilege transition, or Linux boot protocol yet.
- The instruction set is intentionally tiny.

## Design Milestone Follow-Up

- Browser integration release: partially started with worker-owned machine controls, serial terminal, debugger panels, and snapshot controls.
- Debugger v1: partially started with RIP, selected GPRs, RFLAGS bits, and CR0/CR2/CR3/CR4/EFER display.
- Snapshot UI: partially started with capture/restore against the current in-memory Wasm machine.
- Still missing from the design milestone: import/export bundles, drag/drop machine packages, persistent OPFS snapshots, real guest package loading, and full debugger memory/disassembly views.

## Research-Grade Additions

- Execution trace ring added to `bxr-core`.
- Browser debugger now shows current instruction bytes, memory around RIP, and recent trace events.
- Persistent Wasm ABI exposes trace, memory, current-instruction, register, flag, control, serial, and snapshot state.
- Native benchmark harness added as `cargo run -p bxr-bench --release`.
- Page-generation-backed decode cache added with Rust tests, Wasm ABI counters, browser debugger display, and benchmark metrics.
- Repeatable local quality gate added as `npm run quality`.
- Rust toolchain pinned with `rust-toolchain.toml`; GitHub Actions runs the local quality gate.
- Reproducible guest demo corpus added under `tests/guest-programs` and covered by native tests.
- Deterministic virtual clock added to machine state, snapshots, Wasm ABI, smoke tests, and browser debugger metadata.
- Snapshot manifests now include content-addressed chunk metadata for CPU, memory, device, and scheduler state.
- Release architecture and roadmap docs consolidated into `docs/architecture.md` and `docs/roadmap.md`.
