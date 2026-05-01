# BXR Architecture

BXR is a browser-native x86-64 machine runtime. It combines an emulator core, browser worker runtime, snapshot model, debugger surface, and future machine-package format. It should not be described as a hypervisor, container, compatibility layer, or full PC emulator until those claims are actually implemented.

## Current Architecture

- Rust owns CPU state, instruction decode/execute, memory, devices, snapshots, and the Wasm ABI.
- The browser main thread owns UI only.
- The machine worker owns guest execution and state transitions.
- The current machine profile is `bxr-minimal-x64-v1`: single vCPU, direct boot, RAM, serial output, deterministic virtual clock, and snapshot-capable state.
- WebAssembly is the browser execution target; JavaScript is orchestration glue, not the semantic CPU core.

## Hard Boundaries

- The main thread must never execute guest instructions.
- UI code must read machine state through worker/Wasm APIs, not by reaching into internal memory layouts.
- CPU feature bits must expose only implemented or safely trapping behavior.
- Snapshots must capture architectural state and must recreate derived caches after restore.
- Device complexity belongs behind bus/device contracts, not scattered through CPU or UI code.

## Browser Platform Requirements

- Required now: WebAssembly, Web Workers, secure local serving, structured clone/transfer, and HTTP headers suitable for Wasm assets.
- Required for shared memory/SMP: `SharedArrayBuffer`, Atomics, cross-origin isolation, COOP, and COEP.
- Strongly recommended for storage: OPFS for chunk/disk data and IndexedDB for manifest metadata.
- Strongly recommended for display: OffscreenCanvas once framebuffer rendering exists.
- Optional later: WebGPU for display acceleration or visualization, and WebRTC/WebSocket relays for networking.

If workers are unavailable, the runtime cannot keep the UI responsive. If cross-origin isolation is unavailable, SMP and shared-memory rendering are not credible. If OPFS is unavailable, full disk and large snapshot persistence fall back to weaker browser storage paths.

## Determinism

The current virtual clock advances one tick per successfully executed instruction. That is a useful first invariant, not a complete determinism model. A serious deterministic runtime also needs:

- deterministic device deadlines
- deterministic interrupt delivery
- deterministic input event logs
- explicit entropy sources
- stable snapshot quiescence points
- replayable storage and network events

Same machine package plus same input log should produce the same CPU state, serial output, trace order, virtual ticks, and snapshot chunk hashes for the implemented feature set.

## CPU Execution

The interpreter is the executable specification. Optimizations should be layered in this order:

1. interpreter with exact tests
2. decode cache and software TLB
3. micro-op IR
4. block executor
5. optional Wasm trace compiler

Correctness is mandatory for exposed instruction semantics, flags, memory accesses, control flow, page faults, exceptions, interrupts, syscalls, self-modifying code, and CPUID/MSR behavior. Speed work is only acceptable when it can be disabled and differentially tested against the interpreter.

## Machine And Devices

The minimal profile should stay small. Later profiles must add the mandatory advanced capabilities explicitly:

- `bxr-linux-direct-*` for direct Linux boot
- `bxr-pc-firmware-*` for BIOS/UEFI and legacy PC compatibility
- `bxr-smp-*` for multicore execution and memory-ordering guarantees
- `bxr-storage-*` for OPFS-backed disks and snapshot overlays
- `bxr-net-*` for browser-safe virtual networking
- `bxr-simd-*` for SSE/AVX state and instruction coverage

These tracks are required for the long-term project, but each needs independent tests and compatibility claims.

## Debugger And Observability

The first debugger should expose registers, RFLAGS, control registers, current instruction bytes, memory around RIP, serial output, trace events, snapshot availability, decode-cache counters, and virtual ticks. Before claiming kernel-development usefulness, it also needs page-table inspection, breakpoints, watchpoints, exception/interrupt traces, stack view, and snapshot lineage.

## Public Claim

Current claim: BXR is an early browser-native x86-64 runtime prototype with a working Rust/Wasm/worker execution path.

Do not claim: Linux compatibility, full x86-64 compatibility, full PC emulation, SMP, BIOS/UEFI, networking, full disk emulation, or AVX/SIMD breadth until the corresponding profile, tests, and docs exist.
