# BXR Machine Profile v1 Draft

Profile ID: `bxr-minimal-x64-v1`

This profile is the first executable target for BXR. It is intentionally smaller than a PC and is designed for direct x86-64 boot, serial-first diagnostics, deterministic snapshots, and later Linux direct boot.

## Required Components

- One x86-64 vCPU.
- Guest physical memory, page size 4096 bytes.
- Direct x86-64 boot state.
- Serial console device, initially exposed through the debug output port `0x00e9` and COM1 data port `0x03f8`.
- Monotonic virtual timer placeholder.
- Snapshot-capable CPU, memory, and device state.

## Explicit Non-Goals

- BIOS.
- UEFI.
- SMP.
- VGA compatibility.
- PCI discovery.
- Sound.
- AVX.
- Windows compatibility.

## CPU Contract

The CPU exposes only implemented features. CPUID must be conservative. Unsupported instructions must raise a decode/undefined-instruction path instead of silently executing partial behavior.

The first system-register surface includes CR0, CR2, CR3, CR4, and EFER. Long-mode 4-level page walks are implemented as a foundation, but accessed/dirty bit updates and interrupt delivery are later work.

## Memory Contract

RAM is byte-addressable, little-endian, and page-backed. Writes mark dirty pages. Writes to pages marked executable advance the page generation so decode/translation caches can invalidate safely.

## Boot Contract

The first boot path sets a direct x86-64 entry point and stack pointer. Firmware compatibility is a later profile, not part of this one.
