# BXR Roadmap

This roadmap treats SMP, BIOS/UEFI, networking, full disk/storage, and AVX/SIMD breadth as mandatory long-term capabilities. They are not removed from scope; they are gated so the public project remains honest and maintainable.

## Public Prototype Gate

- Clean public repository with no local runtime state.
- Pinned Rust toolchain.
- CI running `npm run quality`.
- Wasm build and smoke test.
- Small reproducible guest corpus.
- Clear current-support and unsupported-feature documentation.

## Correctness Foundation

- Instruction fixtures for every implemented opcode.
- Exact flag tests for arithmetic, branches, stack, and string operations as they are added.
- Page-walk, page-fault, CR2, NX, user/supervisor, and invalidation tests.
- Exception and interrupt delivery tests before real OS claims.
- Differential testing against a reference engine for the supported ISA subset.
- Fuzzing for decoder, execution, MMU, and snapshot restore.

## Snapshot And Storage Foundation

- Versioned manifest migration policy.
- Content-addressed chunk store.
- Page-delta snapshots.
- Snapshot quiescence protocol.
- OPFS chunk persistence.
- IndexedDB manifest index.
- Import/export bundle validation.
- Disk overlay snapshots.

## Boot And OS Track

- Tiny direct-boot x86-64 kernel to serial.
- Long-mode paging and page-fault checkpoint tests.
- Timer interrupt and minimal IRQ controller.
- Linux direct boot with initrd and serial console.
- Kernel boot transcript fixtures.
- Later BIOS and UEFI boot profiles.

## Required Advanced Capability Tracks

### SMP

Required before claiming multicore support:

- SharedArrayBuffer and cross-origin isolation checks.
- Wasm shared memory integration.
- Atomic operation semantics.
- x86 memory-ordering model.
- deterministic scheduling or replay story.
- deadlock/debug tooling for guest cores.

### BIOS/UEFI

Required before claiming firmware compatibility:

- separate firmware machine profile
- memory map and boot services contract
- ACPI/SMBIOS decisions
- device discovery model
- bootloader fixtures

Direct boot remains the first path; firmware support is required later, not the foundation.

### Networking

Required before claiming networking:

- virtual NIC model
- deterministic packet queue state
- browser-safe relay transport
- security model for untrusted guest traffic
- snapshot behavior for in-flight packets

### Full Disk Stack

Required before claiming disk support:

- OPFS-backed base image and writable overlay
- block cache and flush semantics
- snapshot-consistent disk state
- quota handling and eviction reporting
- import/export with chunk verification

### AVX/SIMD Breadth

Required before claiming broad x86-64 software compatibility:

- XMM/YMM architectural state
- SSE/SSE2 baseline before Linux userland claims
- VEX/EVEX decode plan
- XSAVE/XRSTOR and CPUID/MSR policy
- differential tests for SIMD instructions

## Release Milestones

1. `v0.1-prototype`: current minimal runtime, honest docs, CI, Wasm demo, guest corpus.
2. `v0.2-correctness`: stronger ISA/MMU/snapshot tests and deterministic scheduler draft.
3. `v0.3-boot`: tiny kernel boot and interrupt/fault debugging.
4. `v0.4-storage`: OPFS snapshot chunks and disk overlay prototype.
5. `v0.5-linux-direct`: Linux direct boot to serial/initrd shell.
6. `v0.6-debugger`: page-table inspector, breakpoints, trace export, snapshot lineage.
7. `v0.7-simd`: SSE/SSE2 baseline and expanded userland compatibility.
8. `v0.8-firmware-storage-net`: BIOS/UEFI profile work, full disk stack, and network prototype.
9. `v0.9-smp`: experimental multicore profile.
