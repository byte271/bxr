# BXR: Browser x86 Runtime

BXR is a browser-native x86-64 machine runtime: a snapshot-first virtual machine platform designed to run, inspect, pause, restore, and eventually share real machine sessions from the browser.

This repository is an early public prototype. It currently proves the core shape of the project: Rust machine components, a dependency-free WebAssembly ABI, a worker-owned browser shell, deterministic virtual ticks, snapshot metadata, and a small guest corpus. It does **not** yet claim Linux boot, full PC compatibility, SMP, BIOS/UEFI, networking, full disk emulation, or broad AVX/SIMD compatibility.

## Current Scope

The current implementation establishes:

- `bxr-x86`: x86-64 register, flag, width, decoder, and first execution primitives.
- `bxr-memory`: page-backed physical memory with dirty tracking and executable-page generations.
- `bxr-devices`: first device traits plus a serial console/debug output device.
- `bxr-snapshot`: versioned snapshot manifest data structures.
- `bxr-core`: minimal machine profile and machine state shell.
- `bxr-boot`: direct x86-64 boot-state helper.
- `bxr-wasm`: dependency-free browser/Wasm ABI with a persistent demo machine.
- `web`: no-dependency browser shell with worker-owned machine controls, serial output, debugger panels, and snapshot controls.

The project identity and release roadmap are tracked in [`design.md`](design.md).

## Required Long-Term Capabilities

BXR's long-term target includes SMP, BIOS/UEFI profiles, networking, a full disk/storage stack, and broad AVX/SIMD coverage. These are mandatory roadmap capabilities, but they are intentionally not represented as current support until the CPU, device, scheduler, browser isolation, and test infrastructure can validate them.

## Artifact Quality

- Rust is pinned by [`rust-toolchain.toml`](rust-toolchain.toml), including `rustfmt`, `clippy`, and the `wasm32-unknown-unknown` target.
- CI runs the same local gate as contributors: [`npm run quality`](package.json).
- The first reproducible guest corpus lives under [`tests/guest-programs`](tests/guest-programs). It is executed by the native test suite.
- Machine snapshots carry deterministic content-addressed chunk metadata for CPU, memory, serial device, and scheduler/virtual-time state.

## Build

```sh
cargo test --workspace
```

```sh
npm run build:wasm
npm run dev
```

```sh
cargo run -p bxr-bench --release
```

```sh
npm run quality
```

The browser shell runs at `http://127.0.0.1:8080` after `npm run dev`. Run `npm run build:wasm` first to copy the dependency-free Rust/Wasm demo module into `web/wasm/`; without that file the worker falls back to a JavaScript protocol demo.

The first implementation slice intentionally has no third-party dependencies. That keeps the early architecture easy to audit and makes every dependency decision explicit later.

## Documentation

- [`docs/architecture.md`](docs/architecture.md): current architecture and hard boundaries.
- [`docs/roadmap.md`](docs/roadmap.md): required capability tracks and release gates.
- [`docs/specs/`](docs/specs): versioned machine, snapshot, and worker protocol drafts.
- [`docs/first-20-task-status.md`](docs/first-20-task-status.md): current implementation status.
