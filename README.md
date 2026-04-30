# BXR: Browser x86 Runtime

BXR is the seed of a browser-native x86-64 machine runtime: a snapshot-first virtual machine platform designed to boot real software inside the browser while staying modular, inspectable, and contributor-friendly.

This repository is at milestone zero/one. The current code establishes the first compiled core crates:

- `bxr-x86`: x86-64 register, flag, width, decoder, and first execution primitives.
- `bxr-memory`: page-backed physical memory with dirty tracking and executable-page generations.
- `bxr-devices`: first device traits plus a serial console/debug output device.
- `bxr-snapshot`: versioned snapshot manifest data structures.
- `bxr-core`: minimal machine profile and machine state shell.
- `bxr-boot`: direct x86-64 boot-state helper.
- `bxr-wasm`: dependency-free browser/Wasm ABI with a persistent demo machine.
- `web`: no-dependency browser shell with worker-owned machine controls, serial output, debugger panels, and snapshot controls.

The design document lives at [`design.md`](design.md), which points to the canonical architecture proposal.

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
