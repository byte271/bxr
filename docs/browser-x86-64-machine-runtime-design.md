# Browser-Native x86-64 Machine Runtime Design

Status: architecture proposal

Working name: BXR, short for Browser x86 Runtime. The name is provisional; the technical identity is the important part.

## 1. Overall Concept

BXR is a browser-native x86-64 machine runtime: a serious virtual machine platform implemented for the web that can load a machine description, boot real guest software, run it in an isolated browser process, pause instantly, snapshot the full machine state, restore or branch from that state, and share reproducible machine sessions as browser artifacts. It treats the browser not as a thin demo shell around an emulator, but as the host operating environment for a new class of portable, inspectable, snapshot-first machines.

The problem it solves is that machine emulation is still too often packaged as a local tool with brittle setup, opaque state, hard-to-share disk images, and UI/debugger experiences bolted on after the fact. Browser runtimes solve distribution, collaboration, and reproducibility: a machine can become a link, a workspace, a test artifact, a classroom object, a debugger session, or a bug report. This matters because OS development, low-level teaching, reverse engineering, software preservation, reproducible research, and secure sandboxing all benefit from machines that are easy to launch, pause, inspect, clone, and exchange.

Browser-based machine runtimes are interesting because the browser already supplies isolation, deployment, storage, graphics, input, workers, structured cloning, sandboxed networking, and a UI platform. The cost is that the browser is not a traditional hypervisor: there is no native x86 execution, no raw sockets, no kernel-level threads, no direct disk, no privileged timer, and no arbitrary native JIT. A good architecture accepts those constraints and turns them into design principles.

Key distinctions:

- A classic emulator models hardware so software written for another machine can run. Its architecture is often hardware-inventory first.
- A modern runtime treats execution, memory, devices, scheduling, persistence, introspection, and host integration as first-class subsystems with stable contracts.
- A browser-native machine platform is a runtime plus a browser product surface: machine packages, state sharing, storage, UI, debugging, and browser security boundaries are part of the design.
- A compatibility layer maps guest OS or application APIs onto host APIs. BXR should not start as a Linux or Windows compatibility layer; it should run actual guest machine code.
- A virtual machine traditionally relies on hardware virtualization when host and guest ISA match. BXR cannot use VT-x or AMD-V in the browser; it must emulate or translate x86-64.
- Dynamic binary translation, or DBT, translates guest instruction blocks into a host-executable form. In BXR the practical host forms are WebAssembly, a custom micro-op bytecode, or carefully generated JavaScript. DBT is an execution technique, not the whole project.

## 2. Design Goal

The exact goal is to build a clean, browser-first x86-64 machine runtime that can boot real x86-64 software, expose a stable virtual machine model, provide instant pause/resume and snapshot workflows, and evolve from a correct interpreter to a hybrid interpreter/translator without rewriting the system.

Success means:

- A user can open a browser page, load a packaged machine, boot it, interact with serial/framebuffer output, pause it, save a snapshot, close the page, reopen it, and restore the same machine state.
- A developer can inspect CPU registers, memory, disassembly, device state, pending interrupts, and snapshot history from a real debugger UI.
- A contributor can work on the CPU decoder, MMU, device model, storage backend, UI, or tests without understanding the whole codebase.
- The runtime has a documented virtual machine spec and versioned snapshot format.
- Compatibility and performance improve by adding modules, not by contaminating every layer with special cases.

The first public version should boot at least a small x86-64 kernel and a minimal Linux path to a serial console or initramfs shell, support deterministic single-core execution, implement page-granular snapshots, persist disk/state data in browser storage, and include developer tools useful enough for debugging early boot failures.

The project should not try too early to boot Windows, emulate arbitrary PC chipsets, provide a full desktop, support SMP, implement AVX, emulate every obscure x86 corner, or build a broad device zoo. Respect comes from correctness boundaries, a clean machine spec, strong tests, honest compatibility claims, visible debugging tools, reproducible demos, and a roadmap that shows how the architecture grows without collapsing.

What makes the project special is the combination: x86-64 first, browser first, snapshot first, modern virtual hardware, modular core, research-grade observability, and a product surface built around sharing and inspecting live machines.

## 3. Architectural Philosophy

The architecture should be organized around stable contracts:

- CPU core executes instructions and raises architectural events.
- MMU translates virtual addresses and mediates memory access.
- Physical memory stores bytes and tracks dirty pages.
- Bus maps memory, I/O ports, and devices.
- Devices expose small interfaces for reads, writes, interrupts, timers, and snapshot state.
- Scheduler coordinates CPU execution, device deadlines, rendering, I/O, and browser messages.
- Snapshot system serializes architectural state and page deltas.
- Browser shell owns UI, persistence policy, import/export, and worker orchestration.

Keep the system clean by making the virtual machine model explicit. Do not let every guest OS invent its own hidden machine. Define named machine profiles such as `bxr-minimal-x64-v1`, `bxr-linux-direct-v1`, and later `bxr-pc-compat-v1`. A profile declares CPU feature bits, memory map, boot protocol, interrupt controller, timers, and devices. Compatibility exceptions belong in profile definitions or device modules, not scattered through the CPU.

Avoid legacy clutter by separating "must emulate for architectural correctness" from "historical PC hardware." x86-64 privilege modes, paging, exceptions, interrupts, CPUID, MSRs, and flags are architectural. ISA VGA, floppy controllers, PS/2 quirks, ancient DMA, and BIOS services are compatibility choices. Implement them only when a profile needs them.

Generic parts should include scheduling, snapshots, storage chunks, page tables over physical memory, trace logging, debugger protocols, test harnesses, and browser messaging. x86-specific parts include instruction decode, register files, flags, control registers, page-walk semantics, exceptions, CPUID, MSRs, and x86 memory ordering.

Correctness, speed, and maintainability should be balanced by tiering. The interpreter is the executable specification. The micro-op IR is the shared optimization boundary. Fast paths are caches over proven semantics, not replacements for them. Every optimized path must have a fallback and differential tests against the interpreter.

To avoid becoming a messy emulator codebase:

- No device reaches directly into CPU internals except through an interrupt/event API.
- No UI reaches into WebAssembly memory except through typed debug/export APIs.
- No boot path mutates CPU internals through ad hoc hooks after initial machine construction.
- No translator emits code without linking back to decoded instruction metadata.
- No CPUID feature is exposed before its architectural behavior is implemented or safely trapped.
- No snapshot format depends on in-memory Rust struct layout.

## 4. Machine Model

The browser presents a versioned virtual x86-64 machine, not "whatever the emulator currently does." The first machine should be intentionally small:

- One x86-64 CPU, initially single-core.
- Contiguous guest RAM with configurable size, starting with 128 MiB to 1 GiB.
- Architectural x86-64 virtual memory with 4-level paging first; 5-level paging later.
- Local APIC subset or a simpler interrupt controller profile, depending on the boot path.
- One monotonic virtual timer and one real-time clock.
- Serial console.
- Linear framebuffer after the first serial milestones.
- Keyboard and pointer input mapped through a simple virtual input device.
- Optional block storage backed by OPFS.
- Optional network device backed by WebSocket/WebRTC relay later.
- Snapshot and restore state for every architectural subsystem.

Minimal hardware first:

- CPU with integer, branch, stack, system, paging, interrupt, and selected SIMD state.
- RAM.
- Direct kernel loader.
- Serial output.
- Timer interrupt.
- Basic interrupt delivery.
- Snapshot-capable memory.

Postpone:

- Full BIOS.
- Full UEFI.
- VGA text/graphics compatibility.
- PCI enumeration beyond devices required by the Linux path.
- ACPI except minimal tables when needed.
- Sound.
- SMP.
- AVX.
- Windows-oriented device compatibility.

Emulate exactly:

- Instruction semantics for implemented instructions.
- Register widths and partial-register behavior.
- RFLAGS for control flow, arithmetic, string ops, and interrupts.
- Exceptions, faults, and traps with correct priority for implemented cases.
- Page-table walks, privilege checks, NX, user/supervisor, writable, present, accessed/dirty behavior where needed.
- Control registers, EFER, segment behavior in long mode, IDT/GDT/TSS effects required by x86-64 kernels.
- Interruptibility rules around `sti`, `hlt`, faults, and external interrupts.

Simplify:

- Hide unimplemented CPU features in CPUID.
- Use a modern paravirtual or minimal machine profile instead of pretending to be a full PC.
- Start with direct boot rather than BIOS.
- Prefer serial console over VGA.
- Prefer initrd over a writable root disk early.
- Recreate TLBs, translation caches, and renderer caches after restore instead of snapshotting them.

Core state areas:

- CPU: general registers, RIP, RFLAGS, segment registers/descriptors, control registers, debug registers later, MSRs, FPU/SIMD state, pending exceptions, halted/interrupted state.
- Memory: guest RAM pages, dirty bitmap, memory map, MMIO regions.
- Timers/interrupts: virtual time, pending IRQ lines, APIC/PIC state, scheduled deadlines.
- Display/input: framebuffer metadata and device queues, not browser DOM state.
- Storage: immutable base image references plus writable overlay chunks.
- Network: device queue state; live sockets are recreated.
- Boot: boot profile ID, kernel/initrd/disk artifact references, boot parameters.

## 5. CPU Design

The CPU should be built as a tiered execution engine:

1. Decoder: bytes to instruction records.
2. Semantic lowering: instruction records to micro-ops.
3. Reference interpreter: executes micro-ops precisely.
4. Block executor: caches decoded basic blocks and executes micro-op blocks with reduced dispatch.
5. Optional trace compiler: turns hot stable traces into WebAssembly functions.

The first implementation should be a Rust interpreter compiled to WebAssembly. It should decode x86-64 into a compact instruction representation, lower to micro-ops, and execute those micro-ops with precise exception points. A pure interpreter is slower than DBT, but it is the correct first core because browser DBT has high compile latency, security restrictions, and invalidation complexity. The interpreter becomes the oracle for later translators.

The recommended browser approach is hybrid:

- Phase 1: cached decode plus micro-op interpreter.
- Phase 2: specialized block interpreter with direct-threaded style dispatch where Wasm performance allows.
- Phase 3: hot trace selection and WebAssembly trace compilation behind a feature flag.
- Phase 4: profile-guided trace reuse, page invalidation, and snapshot-aware code cache warming.

Avoid JavaScript code generation as the main JIT. It conflicts with content security policies, is brittle across engines, and makes correctness harder. Dynamic WebAssembly generation is more principled, but still expensive and should be tiered carefully. A trace compiler should compile only stable hot blocks, keep metadata for precise traps, and invalidate on writes to executable guest pages.

Instruction decode requirements:

- Support x86 variable-length instructions up to 15 bytes.
- Parse legacy prefixes, REX, mandatory prefixes, opcode maps, ModR/M, SIB, displacement, immediate, VEX/EVEX later.
- Normalize operand forms into an internal representation.
- Preserve original bytes for disassembly, debugging, invalidation, and trace mapping.
- Reject or raise `#UD` for unsupported encodings instead of silently misexecuting.

Flags:

- Implement exact flags for all exposed instructions.
- Use lazy flags internally for speed: store operation kind and operands, materialize RFLAGS only when read by `jcc`, `setcc`, `cmovcc`, `pushf`, interrupts, or debug views.
- Never expose an instruction before its flags are tested.

Registers:

- Represent GPRs as 64-bit values with exact 8/16/32/64-bit writes, including zero-extension on 32-bit writes and high-byte register behavior.
- Model RIP-relative addressing directly.
- Maintain segment base/limit/attributes even though long mode ignores most segmentation, because FS/GS base and privilege transitions matter.

Privilege, exceptions, interrupts:

- Implement rings 0 and 3 as soon as Linux userland is a goal.
- Implement exception delivery through IDT, stack switching through TSS where needed, and correct error codes.
- Page faults must report address in CR2 and deliver correct present/write/user/instruction-fetch bits.
- Faults and traps must be precise: the runtime must know whether RIP advances.
- External interrupts should be delivered at instruction boundaries, with correct masking by IF and interrupt shadow.

x86-64 mode and paging:

- Support real mode only as much as required by boot-sector tests and compatibility later.
- Make long mode the primary target.
- Implement CR0, CR3, CR4, EFER, PAE, long mode enablement, NX, and canonical address checks.
- Start with 4 KiB and 2 MiB pages; 1 GiB pages later.
- Cache virtual to physical translations in a software TLB keyed by ASID-like generation, CR3, privilege, access type, and page permissions.
- Invalidate on CR3 writes, INVLPG, page-table writes detected through dirty tracking, and self-modifying code events.

Memory ordering:

- Single-core phase can execute guest memory sequentially.
- Atomic instructions still need correct locked semantics relative to devices and future SMP.
- For SMP, browser workers and WebAssembly shared memory do not automatically give x86 TSO semantics at useful speed. Treat exact SMP memory ordering as a later research milestone.

Self-modifying code:

- Track executable physical pages.
- Mark code-cache entries by physical page generation.
- On write to an executable page, increment generation and invalidate affected blocks/traces.
- For correctness, interpreter fallback must always handle changed bytes immediately.

CPUID:

- Expose a small honest feature set.
- Set hypervisor bit.
- Hide AVX, XSAVE, AES, BMI, and advanced features until implemented.
- Expose SSE/SSE2 only when userland compatibility requires and tests pass.
- Version the CPUID profile as part of the machine profile.

FPU/SIMD:

- Phase 1 can defer x87/SSE for custom kernels and early kernel boot if CPUID hides features.
- x86-64 Linux userland realistically needs SSE2 soon. Implement XMM registers and SSE/SSE2 before claiming broad user program support.
- x87 can arrive after integer/SSE basics, but many older binaries and libc paths still touch it.
- AVX should be later because VEX decode, YMM state, XSAVE, and performance all broaden the state surface.

Exact now versus delayed:

- Exact now: integer ops, control flow, stack, flags, paging, exceptions, interrupts, CPUID honesty, system registers needed by boot.
- Delayed: broad SIMD, debug registers, performance counters, obscure string optimizations, VMX, SMM, full ACPI, AVX, machine-check behavior.

## 6. Browser Execution Model

The browser runtime should be worker-centered:

- Main thread: UI, command routing, layout, user input capture, high-level state display.
- Machine worker: CPU core, memory, MMU, device scheduler, deterministic state transitions.
- Render worker: OffscreenCanvas rendering, framebuffer conversion, optional WebGPU pipeline.
- Storage worker: OPFS disk images, chunk store, snapshot persistence.
- Network worker: packet relay adapters through browser-supported transports.

Rust compiled to WebAssembly should own CPU, MMU, physical memory, core devices, snapshot serialization, and testable machine state. TypeScript should own browser orchestration, worker lifecycle, UI, import/export, IndexedDB metadata, OPFS handles, and compatibility checks. JavaScript should be glue, not a semantic core.

Use SharedArrayBuffer only when needed:

- For single-core, normal WebAssembly.Memory may be faster and simpler.
- For renderer sharing, a SharedArrayBuffer framebuffer ring can avoid copies if cross-origin isolation is available.
- For SMP later, shared memory becomes mandatory.

SharedArrayBuffer requires secure context and cross-origin isolation. That means the public app must ship with COOP/COEP headers and a fallback mode for environments that cannot provide them. OPFS is appropriate for block storage and snapshot chunks, especially in workers where synchronous access handles can avoid promise overhead. OffscreenCanvas lets rendering move off the main thread. WebGPU is useful for future display scaling, texture conversion, and compute experiments, but it should be optional because support is not universal.

Responsiveness rules:

- CPU execution runs in time slices, for example 1 to 5 ms of guest work per scheduler quantum.
- The machine worker posts progress, interrupts, and framebuffer dirty regions without blocking the UI.
- UI commands are asynchronous and idempotent: pause, resume, step, snapshot, restore, import, export.
- Long operations such as snapshot compression run incrementally or in a separate worker.
- Rendering is dirty-rectangle or frame-rate limited, not tied to every guest write.
- Storage writes are batched through a write-back cache with explicit flush points for snapshots.

The runtime should feel smooth because the main thread never executes guest instructions, never compresses memory, never parses disk images, and never blocks on large OPFS reads.

## 7. Boot System

Boot should be layered:

1. Direct flat-binary boot for CPU bring-up.
2. Direct x86-64 kernel boot for tiny project kernels.
3. Linux direct boot using the Linux x86 boot protocol, initrd, and kernel command line.
4. Disk-backed bootloader path for compatibility.
5. Optional BIOS/UEFI profile later.

The best first boot path is not BIOS or UEFI. It is a direct kernel loader that constructs a known machine state and jumps into a tiny test kernel. This reaches "first boot success" quickly: CPU state, memory, long mode, serial output, interrupts, and snapshots can be validated without writing a fake PC around them.

The first public Linux path should use direct kernel boot with an initrd. It avoids a block device at first and lets the runtime prove it can handle long mode, paging, traps, timers, and serial console. A later bootloader path can load GRUB or Limine from a disk image once storage and more PC compatibility are ready.

BIOS should be postponed. UEFI should be treated as a compatibility package, not the core identity. A small custom boot adapter is cleaner and easier to maintain than embedding a full firmware stack before the CPU and snapshot systems are mature.

Clean boot module boundaries:

- `boot-loader` parses machine package metadata.
- `boot-x64-direct` initializes CPU registers, page tables, segments, and memory layout for project kernels.
- `boot-linux-x86` loads bzImage/initrd and constructs boot parameters.
- `boot-disk` eventually exposes disk images to a bootloader.
- Firmware modules, if added, are regular boot providers, not privileged global code.

## 8. Operating System Support

Support should grow through classes:

- Class 0: CPU self-tests and freestanding x86-64 test kernels.
- Class 1: tiny educational kernels with serial/framebuffer output.
- Class 2: Linux kernel direct boot to panic-free early init.
- Class 3: Linux plus initramfs userland and shell.
- Class 4: Linux with block storage and basic networking.
- Class 5: graphical Linux demos.
- Class 6: broader PC compatibility and other OS experiments.
- Class 7: Windows-class compatibility, only if the hardware profile becomes mature enough.

Booting a kernel means reaching its entry point with a plausible machine state. Running an OS means surviving interrupts, paging, syscalls, scheduler behavior, drivers, and device interaction. Running user programs means the kernel can enter ring 3, deliver signals/exceptions, manage page faults, and execute libc/ABI assumptions. Supporting a desktop means display, input, storage, timers, sound, and enough performance. Supporting real commercial software means years of compatibility work.

The realistic path is:

1. Boot a tiny kernel.
2. Boot Linux to early console.
3. Boot Linux with initrd.
4. Run BusyBox/static tools.
5. Add storage overlay and network.
6. Add framebuffer/simple graphical stack.
7. Expand CPU features and device compatibility.

Windows should not be an early goal. It pulls in UEFI/BIOS, ACPI, APIC complexity, PCI details, storage/network/display drivers, timing assumptions, and a much larger CPU feature surface.

## 9. Browser-Native Features

Use browser features as platform capabilities:

- WebAssembly: CPU core, MMU, devices, snapshot algorithms, compression libraries where appropriate.
- TypeScript: UI, worker protocol, app state, machine package management, storage orchestration.
- Web Workers: CPU and I/O isolation from the main thread.
- SharedArrayBuffer: shared framebuffer rings, future SMP memory, low-copy worker exchange where cross-origin isolation is available.
- OffscreenCanvas: render guest display off main thread.
- WebGPU: optional future display compositor, accelerated scaling, color conversion, and research compute paths.
- IndexedDB: metadata, machine catalog, snapshot manifests, content-addressed chunk indexes.
- OPFS: disk images, memory page chunks, large snapshot blobs, write-back overlays.
- URL state: small launch descriptors, machine package references, snapshot manifest IDs, not giant RAM dumps.
- Drag and drop: import kernels, initrds, disks, snapshots, and machine packages.
- Clipboard: serial console paste/copy and guest clipboard integration later.
- File System Access API: optional explicit export/import for browsers that support it.

Make it feel like a real browser app:

- Machine packages are typed artifacts with manifests.
- Snapshots show names, sizes, timestamps, base dependencies, and compatibility.
- Restore is a primary UI command, not a hidden save-state hack.
- Debugger panels are built into the product.
- Import/export validates versions and checksums.
- Browser capability checks explain missing isolation, storage, or worker support before launching.

## 10. Performance Strategy

The biggest bottlenecks are instruction dispatch, memory translation, bounds/MMIO checks, flags computation, host-to-guest crossings, device interrupts, rendering copies, snapshot compression, and dynamic translation compile latency.

Instruction execution:

- Cache decoded blocks by physical address and code page generation.
- Lower complex x86 instructions to compact micro-ops.
- Use lazy flags.
- Use specialized fast paths for common load/store/addressing modes.
- Fuse common pairs such as compare plus branch in the block executor.
- Keep cold exact paths separate from hot simple paths.

Memory access:

- Use software TLBs for instruction fetch and data access.
- Fast path RAM accesses after translation.
- Keep MMIO ranges out of hot RAM pages.
- Cache page permissions and host offsets.
- Use dirty bitmaps for snapshots and executable-page invalidation.
- Pool temporary buffers and avoid per-access allocations.

Device emulation:

- Use event deadlines instead of ticking every device every instruction.
- Batch IRQ delivery at instruction boundaries.
- Use ring buffers for serial/input/framebuffer events.
- Keep slow browser I/O behind async device backends with deterministic guest-visible completion events.

Rendering:

- Track framebuffer dirty pages or rectangles.
- Transfer only changed regions.
- Render in an OffscreenCanvas worker.
- Use WebGPU/WebGL only when it simplifies scaling/composition or future acceleration.

Snapshots:

- Page-granular dirty tracking.
- Immutable base plus delta pages.
- Content-addressed page chunks.
- Compression in worker.
- Lazy restore by mapping pages immediately and decompressing cold pages on demand where feasible.
- Store translation caches as recreatable, not authoritative.

Startup:

- Lazy-load optional devices and debugger panels.
- Compile Wasm core once and cache through normal browser mechanisms.
- Keep machine manifests small.
- Load kernel/initrd/disk chunks on demand.
- Restore snapshots before rebuilding trace caches.

The machine feels fast when pause/resume is instant, restore is visibly immediate, UI remains responsive, serial/framebuffer output streams smoothly, and long storage operations report progress. Raw MIPS matters, but perceived latency and state workflows are equally important.

## 11. Compatibility Strategy

Phase 1 compatibility should be explicit: custom x86-64 kernels, CPU tests, boot-sector experiments, and a narrow Linux direct-boot path. The first public version can claim "Linux initramfs shell on a documented machine profile" only when it passes repeatable boot tests.

Executing x86 code is not the same as booting a bootloader. Booting a bootloader is not the same as running a kernel. Running a kernel is not the same as running arbitrary user programs. Running command-line userland is not the same as desktop apps. Running Windows software is a separate multi-year compatibility project.

Hard compatibility areas:

- x87: old binaries and libc paths.
- SSE/SSE2: baseline for x86-64 userland ABI in practice.
- AVX/XSAVE: large state and decode surface; hide until implemented.
- RFLAGS: subtle and heavily tested by real code.
- Syscalls: handled by the guest kernel, but instructions such as `syscall`, `sysret`, `iretq`, and MSRs must be exact.
- ABI details: signal delivery, TLS via FS/GS, page faults, stack alignment, SIMD state.
- Self-modifying code: invalidation and precise execution.
- Memory maps: kernels depend on e820-like maps, initrd placement, ACPI/boot params later.
- Device assumptions: Linux drivers expect discoverable devices and correct interrupts.
- Timing assumptions: kernels calibrate timers and may behave poorly under unstable virtual time.

Choose exactness for CPU architectural state, faults, paging, and interrupts. Choose practicality for device inventory, firmware, rarely used CPU extensions, and legacy hardware. The rule is simple: if exposing it can cause guest software to depend on it, either implement it correctly or hide it.

## 12. Multicore and Concurrency

Multicore should be designed for but not shipped early. In a browser, multicore means multiple guest vCPUs running in multiple Web Workers over shared guest memory, synchronized with Atomics, shared interrupt controller state, and message/ring protocols for device events.

The path from single-core to SMP:

1. Define CPU state as per-vCPU from day one.
2. Make physical memory independent of CPU state.
3. Make interrupts route through explicit controller state.
4. Make devices safe under a central scheduler first.
5. Add paused lockstep dual-vCPU tests.
6. Add experimental one-worker-per-vCPU execution.
7. Add APIC/IPI and TLB shootdown.
8. Address x86 memory ordering.

Risks:

- Browser scheduling is not real-time.
- Shared memory requires cross-origin isolation.
- Exact x86 TSO over WebAssembly/JS shared memory may need conservative fences or serialized memory operations.
- Data races in the emulator can corrupt guest state.
- Deterministic replay becomes harder.
- Snapshotting a live multicore machine requires stop-the-world coordination.

Do not implement SMP early. Build all state boundaries so SMP can be added without rewriting the core, then ship it as an experimental profile once single-core Linux and snapshots are solid.

## 13. Snapshot and State System

Snapshots should be a primary architectural feature, not an afterthought. A snapshot is a versioned machine-state manifest plus content-addressed chunks.

Must save:

- Machine profile ID and runtime version compatibility.
- CPU state for every vCPU.
- FPU/SIMD state for implemented features.
- Control registers, MSRs, descriptor tables, interrupt state.
- RAM pages or references to base pages plus deltas.
- Device states and queues.
- Virtual time and scheduled events.
- Storage overlay state.
- Boot artifact references and hashes.
- Snapshot lineage metadata.

Can recreate:

- Software TLBs.
- Decoded block caches.
- Wasm trace code.
- Renderer textures.
- UI layout.
- Open network sockets.
- OPFS access handles.

Make snapshots fast:

- Dirty-page tracking from the first memory implementation.
- Copy-on-write page tables during pause.
- Background compression.
- Incremental manifests.
- Quiescent snapshot points at instruction boundaries.
- Device `snapshot_prepare` hooks to flush deterministic state.

Make snapshots small:

- Deduplicate identical pages.
- Delta against base images and parent snapshots.
- Compress cold pages.
- Store sparse zero pages as metadata.
- Hash chunks for reuse.

Restore instantly:

- Load manifest first.
- Reconstruct CPU/device structs immediately.
- Map page references lazily.
- Prioritize hot pages: stacks, current code pages, page tables, framebuffer.
- Rebuild caches after resume.

Supported workflows:

- Pause: stop scheduler at a precise boundary.
- Resume: continue from same state.
- Rewind: restore parent snapshot.
- Branch: create child snapshot from any parent.
- Clone: duplicate manifest plus copy-on-write overlays.
- Share: export manifest and chunks as a bundle or publish to a content store.
- Import: verify hashes, profile version, and artifact availability before restore.

Snapshots can become the project's main advantage because they turn machines into manipulable browser objects: bug reports, assignments, boot traces, reproducible failures, forks, demos, and research artifacts.

## 14. Device Model

Device interface:

```text
Device {
  id
  reset(machine_profile)
  mmio_read(addr, size) -> value/event
  mmio_write(addr, size, value) -> event
  pio_read(port, size) -> value/event
  pio_write(port, size, value) -> event
  next_deadline() -> virtual_time?
  on_time(virtual_time) -> events
  irq_lines() -> line states
  snapshot() -> bytes
  restore(bytes)
}
```

The bus owns address routing. Devices do not own threads unless their browser backend requires it. The guest-visible device model stays deterministic even when the host backend is asynchronous.

Implement first:

- Serial console, compatible enough with 16550A or a documented paravirtual serial device.
- Virtual timer.
- Interrupt controller sufficient for the boot profile.
- Linear framebuffer or simple boot framebuffer after serial.
- Keyboard/mouse input queue after framebuffer.
- Initrd loader before block disk.

Postpone:

- Sound.
- Full PCI.
- Virtio-net until a relay design exists.
- Virtio-blk until snapshot overlays are solid.
- VGA compatibility.
- Full RTC edge cases.
- USB.

Recommended device path:

- `console0`: serial first for deterministic boot logs.
- `timer0`: monotonic virtual timer.
- `irq0`: minimal interrupt controller, then APIC-compatible profile.
- `fb0`: simple linear framebuffer with dirty tracking.
- `input0`: keyboard/pointer event queues.
- `blk0`: OPFS-backed block overlay.
- `net0`: packet rings to relay transport.
- `rtc0`: wall-clock snapshot-aware time source.

Avoid overbuilding by implementing only devices required by the next OS milestone. Keep each device snapshotable and testable in isolation.

## 15. Debugger and Developer Tools

The debugger should be a first-class browser UI, not a console afterthought.

First version tools:

- CPU register view.
- RFLAGS decoded view.
- Control register and mode view.
- Current instruction disassembly.
- Memory viewer by virtual and physical address.
- Page-table walk inspector.
- Serial console.
- Pause/resume/step.
- Breakpoints by virtual/physical address.
- Snapshot list and restore.
- Basic trace log around exceptions and interrupts.

Later tools:

- Stack view with symbol support.
- Watchpoints.
- Execution timeline.
- Device state inspectors.
- Trace search.
- Kernel symbol loading.
- ELF/DWARF awareness.
- GDB remote protocol.
- Reverse stepping via snapshots.
- Record/replay.
- Differential trace comparison against reference runs.

This can be useful to OS developers because boot failures become inspectable in-browser. It can be useful to emulator researchers because decode, micro-op lowering, translation caches, and invalidation are visible. It can be useful to reverse engineers because snapshots, breakpoints, memory inspection, and shareable traces make experiments reproducible.

## 16. Testing Strategy

CPU tests:

- Decoder golden tests for opcode bytes to instruction records.
- Generated operand-form tests for ModR/M, SIB, displacement, immediates, prefixes, REX, and invalid encodings.
- Instruction semantic tests against a native/reference runner for exact outputs and flags.
- Randomized small-program differential tests against QEMU, Bochs, hardware harnesses, or another reference where legally and practically available.
- Exception-priority tests.

Flags:

- Per-instruction flag matrix.
- Edge cases around carry, overflow, sign, zero, parity, auxiliary carry.
- Lazy-flags materialization tests.

Paging:

- Page-walk tests for present/write/user/NX/accessed/dirty.
- Canonical address tests.
- CR3 and INVLPG invalidation tests.
- User/kernel privilege tests.
- Page fault error-code tests.

Interrupts:

- IDT delivery tests.
- Error code and stack-frame tests.
- `iretq`, `syscall`, `sysret`, `sti`, `cli`, `hlt` tests.
- Timer interrupt integration tests.

Boot behavior:

- Tiny kernel smoke tests.
- Serial transcript golden tests.
- Linux boot checkpoints.
- Panic/fault trace capture.

Snapshots:

- Round-trip CPU/device/memory state.
- Snapshot during halted CPU, running CPU, pending IRQ, dirty framebuffer, and disk writes.
- Restore then continue deterministic transcript.
- Branch and clone lineage tests.

Device behavior:

- Per-device register tests.
- Interrupt line tests.
- Storage overlay persistence tests.
- Framebuffer dirty-region tests.

Long-running stability:

- Soak tests in browser automation.
- Memory leak checks.
- Snapshot/restore loops.
- Boot loops.
- Random pause/resume under load.

Trust over time comes from CI that runs native Rust tests, Wasm tests, browser integration tests, boot smoke tests, snapshot regression tests, and benchmark trend checks. Booting Linux is evidence, not a substitute for instruction-level tests.

## 17. Implementation Language and Tooling

Recommended stack:

- Rust for CPU, decoder, MMU, memory, devices, snapshots, and core tests.
- TypeScript for browser app, workers, UI, package manifests, storage orchestration, and debugger shell.
- WebAssembly as the core runtime target.
- JavaScript only as generated glue.

Rust is the best fit because it gives memory safety, high-performance data structures, good testing, strong WebAssembly support, and contributor familiarity in systems projects. C++ can be fast but raises memory-safety and toolchain complexity. Zig is attractive for systems work but has a smaller web/Wasm ecosystem. TypeScript is excellent for the browser surface but should not own x86 semantics. Plain JavaScript should not be used for the core.

Toolchain:

- Cargo workspace.
- `wasm32-unknown-unknown` target.
- `wasm-bindgen` or a minimal ABI bridge for TS integration.
- Vite or equivalent for browser development.
- Playwright for browser integration tests.
- Rust unit tests and property tests.
- Fuzzing for decoder and instruction semantics.
- Criterion or custom benchmark harness for native microbenchmarks.
- Browser performance traces for Wasm and UI profiling.
- Snapshot corpus tests in CI.

Build and release:

- Native core tests run fast on every PR.
- Wasm build validates browser ABI.
- Browser tests boot tiny kernels.
- Release bundles include Wasm core, TS app, docs, sample machine packages, and reproducible test images.
- Contributor docs explain machine profiles, decoder tables, adding instructions, adding devices, and writing boot tests.

## 18. Repository Layout

Proposed layout:

```text
bxr/
  Cargo.toml
  package.json
  README.md
  docs/
    architecture/
      machine-model.md
      cpu-core.md
      snapshots.md
      browser-runtime.md
      device-model.md
    specs/
      machine-profile-v1.md
      snapshot-format-v1.md
      worker-protocol-v1.md
    roadmap.md
  crates/
    bxr-core/
      src/lib.rs
      src/machine.rs
      src/scheduler.rs
    bxr-x86/
      src/cpu/
      src/decode/
      src/ir/
      src/execute/
      src/mmu/
      src/cpuid.rs
      src/msr.rs
    bxr-memory/
      src/phys.rs
      src/tlb.rs
      src/dirty.rs
    bxr-devices/
      src/bus.rs
      src/serial.rs
      src/timer.rs
      src/irq.rs
      src/framebuffer.rs
      src/block.rs
    bxr-boot/
      src/direct_x64.rs
      src/linux_x86.rs
      src/disk.rs
    bxr-snapshot/
      src/manifest.rs
      src/chunk.rs
      src/restore.rs
    bxr-wasm/
      src/lib.rs
      src/bridge.rs
    bxr-test-kernels/
      src/
  web/
    src/
      app/
      workers/
        machine.worker.ts
        render.worker.ts
        storage.worker.ts
      debugger/
      storage/
      protocol/
      ui/
    public/
  tests/
    decode/
    instruction/
    paging/
    interrupts/
    boot/
    snapshots/
    browser/
  benches/
    decode/
    execute/
    memory/
    snapshot/
    browser/
  tools/
    gen-decode-tables/
    make-machine-package/
    run-reference-tests/
    inspect-snapshot/
  examples/
    tiny-kernel/
    linux-initrd/
    machine-packages/
```

This scales because ISA-specific code lives in `bxr-x86`, generic runtime code lives in `bxr-core`, browser code lives in `web`, and specs live in `docs/specs`. Ownership is clear: CPU contributors do not need UI context; UI contributors do not need instruction semantics; device contributors implement the bus/device/snapshot contracts.

## 19. Project Roadmap

Milestone 0: charter and specs

- Define `bxr-minimal-x64-v1`.
- Define snapshot manifest v0.
- Define worker protocol v0.

Milestone 1: first instruction execution

- Rust x86 decoder skeleton.
- Register file and flags.
- Integer arithmetic, moves, branches, stack.
- Native tests.

Milestone 2: first long-mode test kernel

- Direct x64 boot state.
- Basic memory.
- Serial console.
- Tiny kernel prints text.

Milestone 3: first visible browser output

- Wasm bridge.
- Machine worker.
- Main-thread UI shell.
- Serial terminal in browser.

Milestone 4: paging and exceptions

- CR0/CR3/CR4/EFER.
- Page tables and page faults.
- IDT delivery.
- Interrupt/exception tests.

Milestone 5: first snapshot

- CPU/device/RAM serialization.
- Dirty-page tracking.
- Pause/resume.
- Snapshot restore in same session.

Milestone 6: persistent save/restore

- OPFS chunk store.
- IndexedDB manifests.
- Export/import bundle.
- Restore after page reload.

Milestone 7: first Linux direct boot

- Linux boot protocol.
- Initrd support.
- Kernel command line.
- Serial console transcript.

Milestone 8: performance pass 1

- Decode cache.
- Software TLB.
- Lazy flags.
- Block executor.
- Benchmarks.

Milestone 9: browser integration release

- Debugger v1.
- Snapshot UI.
- Drag/drop import.
- Machine package validation.
- Public demo.

Milestone 10: public release v0.1

- Tiny kernel demo.
- Linux initramfs demo if stable.
- Docs for contributors.
- CI and benchmark dashboard.

Later milestones:

- Framebuffer and input.
- OPFS block device.
- Virtio-style devices.
- SSE/SSE2 userland compatibility.
- Wasm trace compiler.
- Network relay.
- SMP experiment.
- Broader Linux distro support.
- Optional firmware/PC compatibility profile.

Do not start SMP, AVX, Windows, full UEFI, sound, or desktop polish before the CPU, MMU, boot, snapshot, and debugger foundations are working.

## 20. Differentiation From Existing Projects

BXR can be meaningfully different by choosing a new center of gravity. It should not be "a PC emulator ported to JavaScript." It should be a browser-native machine platform with a documented VM profile, a serious x86-64 core, a snapshot DAG, a strong debugger, and browser storage/sharing as first-class features.

It can be better than a classic emulator architecturally by:

- Treating snapshots as a primary data model.
- Using modern virtual devices instead of inheriting every old peripheral.
- Keeping boot profiles explicit.
- Separating CPU semantics from devices and browser UI.
- Building debugger and trace infrastructure early.
- Providing repeatable browser CI.
- Making machine states shareable.
- Using WebAssembly and workers intentionally.

It can surpass old ideas without pretending to be magic by being honest: browser x86-64 will not beat native virtualization on raw speed, and full PC compatibility is expensive. Its advantage is portability, inspectability, shareability, reproducibility, and integrated tooling.

Avoid copying old emulator architecture by refusing to let the legacy PC become the implicit design. Start with a modern minimal x86-64 machine, then add compatibility profiles as needed.

## 21. Risks and Hard Problems

Hardest problems:

- x86-64 correctness across decode, flags, faults, paging, and privilege transitions.
- Linux boot compatibility without letting PC legacy dominate.
- Browser performance under Wasm and worker constraints.
- Snapshot consistency across CPU, memory, devices, and storage.
- Self-modifying code and trace invalidation.
- SIMD and x86-64 userland ABI support.
- Multicore memory ordering.
- Deterministic testing against reference behavior.
- Browser storage quota and eviction behavior.
- Network limitations in browser sandboxes.

Most time will go into CPU correctness, MMU/exceptions, Linux boot, tests, and performance profiling. The project can go wrong if it overbuilds devices before the core works, exposes CPUID features too early, treats boot success as correctness proof, makes snapshots dependent on internal struct layouts, or tries to chase Windows/desktop demos before the machine model is trustworthy.

Simplify first:

- Single core.
- Direct boot.
- Serial console.
- Initrd over disk.
- No AVX.
- No full firmware.
- Narrow Linux config.

Perfection matters in CPU architectural state, exceptions, page faults, interrupts, snapshot restore, and exposed feature semantics. Perfection matters less initially in exact wall-clock timing, old devices, visual polish, and broad OS compatibility.

## 22. Required Output Summaries

### Executive Summary

BXR should be a browser-native x86-64 machine platform, not a toy emulator. Its first credible public form is a single-core, direct-boot x86-64 runtime with a Rust/WebAssembly CPU core, TypeScript browser shell, worker-based execution, serial console, snapshot persistence, debugger UI, and a narrow Linux/initramfs path. Its long-term identity is snapshot-first, inspectable, shareable machines in the browser.

### One-Page Architecture Overview

The system has four layers:

1. Core machine layer in Rust/Wasm: CPU, decoder, micro-op interpreter, MMU, memory, bus, devices, scheduler, snapshots.
2. Browser runtime layer in TypeScript: workers, command protocol, storage coordination, machine packages, import/export.
3. Product layer: debugger, terminal, display, snapshot browser, machine launcher, settings, diagnostics.
4. Research/optimization layer: block executor, trace compiler, code-cache invalidation, profiling, differential testing.

Execution flow:

```text
UI command -> machine worker -> Wasm machine core -> CPU/MMU/bus/devices
                                      |                  |
                                      |                  -> render/storage/network workers
                                      -> snapshot chunks/manifests -> OPFS/IndexedDB
```

The main thread stays interactive. The machine worker owns deterministic guest execution. Storage and rendering are isolated. Snapshots are page-based and versioned. Optimizations sit behind the interpreter and can be disabled for debugging.

### Module List

- `x86-decode`: byte stream to instruction records.
- `x86-ir`: semantic micro-ops.
- `x86-exec`: interpreter and block executor.
- `x86-mmu`: page walks, TLB, faults.
- `memory`: physical RAM, dirty tracking, executable page generations.
- `bus`: MMIO/PIO routing.
- `devices`: serial, timer, IRQ, framebuffer, input, block, network later.
- `boot`: direct x64, Linux direct, disk boot later.
- `snapshot`: manifests, chunks, compression, restore.
- `worker-runtime`: machine/render/storage/network workers.
- `browser-ui`: terminal, debugger, display, snapshot browser.
- `tests`: instruction, paging, interrupts, boot, snapshots, browser.
- `benchmarks`: execution, decode, memory, snapshot, rendering.

### Milestone List

1. Specs and machine profile.
2. Decoder and integer interpreter.
3. Tiny direct-boot kernel prints serial.
4. Browser worker runs machine.
5. Paging, exceptions, interrupts.
6. Pause/resume and in-memory snapshot.
7. OPFS/IndexedDB persistent snapshot.
8. Linux direct boot with initrd.
9. Debugger v1.
10. Decode cache, TLB, lazy flags.
11. Public v0.1.
12. Framebuffer/input.
13. Block storage overlay.
14. SSE/SSE2.
15. Trace compiler.
16. Network.
17. SMP experiment.
18. Broader compatibility profile.

### First 20 Tasks

1. Pick project name and write charter.
2. Define `bxr-minimal-x64-v1`.
3. Define snapshot manifest v0.
4. Create Rust workspace and TypeScript web shell.
5. Implement register file.
6. Implement decoder scaffold with opcode metadata.
7. Implement basic moves/arithmetic/branches/stack.
8. Add exact flag tests for implemented arithmetic.
9. Add native CPU test harness.
10. Build tiny x86-64 test kernel.
11. Implement direct boot loader for tiny kernel.
12. Implement serial device.
13. Compile core to Wasm.
14. Create machine worker protocol.
15. Render serial terminal in browser UI.
16. Add page-backed physical memory with dirty tracking.
17. Implement CR0/CR3/CR4/EFER and page walks.
18. Implement page fault delivery.
19. Add pause/resume.
20. Implement first CPU/RAM/device snapshot round trip.

### Major Technical Decisions

- Browser first, no native daemon required.
- x86-64 first, with honest CPUID feature exposure.
- Rust core compiled to WebAssembly.
- TypeScript browser shell.
- Worker-owned machine execution.
- Direct boot before BIOS/UEFI.
- Serial before framebuffer.
- Initrd before writable disk.
- Single-core before SMP.
- Interpreter before DBT.
- Micro-op IR as optimization boundary.
- Snapshot format versioned and independent of struct layout.
- OPFS for large chunks, IndexedDB for metadata.
- OffscreenCanvas for rendering, WebGPU optional.
- Compatibility profiles instead of implicit PC sprawl.

### Open Questions

- What exact Linux kernel config should define the first public Linux target?
- Should the first interrupt controller be APIC-compatible or a simpler paravirtual profile?
- Should block/network devices be virtio-pci, virtio-mmio with ACPI, or custom BXR devices first?
- How much real-mode support is needed before the first release?
- Which reference engines should be used for differential CPU tests?
- What compression format is best for browser snapshot chunks?
- How should storage quota exhaustion be surfaced to users?
- What minimum browser support matrix is acceptable?
- Should trace compilation target dynamic Wasm modules or a denser internal bytecode first?
- What is the exact security model for importing untrusted machine packages?

### Likely Future Research Directions

- Fast and precise dynamic WebAssembly trace compilation.
- Snapshot DAGs with content-addressed deduplication across users.
- Deterministic replay and reverse debugging.
- Browser-feasible x86 TSO modeling for SMP.
- Hybrid CPU verification using generated tests and symbolic execution.
- Page-hotness-guided lazy restore.
- WebGPU-assisted framebuffer and memory visualization.
- Collaborative shared machine sessions.
- Portable machine-state bundles for bug reports and classrooms.
- Capability-secure browser networking for guest machines.

## Browser Platform References

These browser constraints should be treated as engineering inputs:

- SharedArrayBuffer requires secure context and cross-origin isolation: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/SharedArrayBuffer
- WebAssembly.Memory can expose ArrayBuffer or SharedArrayBuffer and uses 64 KiB pages: https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/Memory/Memory
- OPFS provides origin-private storage, is available in workers, and supports synchronous worker access handles: https://developer.mozilla.org/docs/Web/API/File_System_API/Origin_private_file_system
- OffscreenCanvas is available in workers and is suitable for off-main-thread rendering: https://developer.mozilla.org/en-US/docs/Web/API/OffscreenCanvas/OffscreenCanvas
- WebGPU is secure-context only and not universally baseline, so it should be optional: https://developer.mozilla.org/en-US/docs/Web/API/WebGPU_API
