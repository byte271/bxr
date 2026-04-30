# World-Class Criteria

BXR should earn respect by making every claim measurable. "World-class" for this project means a browser-native machine runtime with a clear virtual hardware contract, repeatable correctness checks, visible performance data, and research hooks that are useful before the runtime is broadly compatible.

## Engineering Bar

- Correctness first: optimized paths must preserve interpreter-visible behavior and have tests that cover invalidation, faults, and snapshot restore.
- Browser first: the main thread must stay responsive; execution, storage, and future rendering belong in isolated workers.
- Snapshot first: architectural state is saved; transient caches are rebuilt after restore.
- Observable by default: execution traces, register state, memory windows, cache counters, and benchmark output are surfaced through stable APIs.
- Conservative compatibility: CPUID and machine profiles expose only behavior the runtime actually implements.
- Dependency discipline: early core stays no-dependency unless a dependency has a clear architectural payoff.
- Contributor trust: one command should verify format, tests, static analysis, Wasm build, browser syntax, and the Wasm ABI smoke path.

## Current World-Class Signals

- Rust workspace split by responsibility: CPU/decoder, memory, devices, core machine, boot, snapshot, Wasm ABI, and benchmark harness.
- `bxr-minimal-x64-v1` is documented as a small direct-boot profile instead of an implicit PC clone.
- Physical memory tracks dirty pages and executable-page generations.
- The MMU has a long-mode page-walk path and records page-fault addresses in CR2.
- The browser worker owns the machine and the UI reads state through explicit messages.
- The debugger surface shows registers, flags, controls, instruction bytes, memory around RIP, trace events, snapshots, and decode-cache counters.
- The native benchmark prints steps/second, trace occupancy, and decode-cache counters for cold sequential decode and hot-loop reuse.
- `npm run quality` runs the project quality gate.

## Near-Term Criteria To Reach Public-Project Credibility

- Add CPU reference tests for every implemented instruction, including exact flags and fault behavior.
- Add a deterministic fixture format for small guest programs.
- Add import/export of machine snapshots without depending on Rust struct layout.
- Add a software TLB with the same generation/invalidation model as the decode cache.
- Add a block executor behind a feature flag after the interpreter tests are stronger.
- Add Linux direct-boot fixtures before adding broad PC hardware.
- Add browser performance marks for worker run, snapshot, restore, and debug-state reads.

## Hard Rules For Future Optimization

- No cache survives snapshot restore unless the snapshot format explicitly owns it.
- No translated block may execute without a recorded source RIP, instruction bytes, and page generation.
- No guest-writable executable page may keep stale decoded or translated code after a write.
- No new CPU feature bit may be exposed before unsupported behavior traps clearly.
- No browser UI claim should be accepted without a worker-level state source or smoke test.
