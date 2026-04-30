# Research Observability

BXR's research value depends on making execution inspectable. The current implementation establishes the first observability spine:

- A bounded execution trace ring in `bxr-core`.
- Instruction bytes preserved in trace events.
- Operation class codes exposed to the browser.
- RIP-before/RIP-after per traced instruction.
- Register/control/flag views through the Wasm ABI.
- A memory window around RIP in the browser debugger.
- A native benchmark harness via `cargo run -p bxr-bench --release`.
- A page-generation-backed decode cache with hit, miss, entry, and invalidation counters.

## Current Trace Event

Each trace event stores:

- sequence number
- RIP before execution
- RIP after execution
- instruction length
- instruction bytes
- operation code
- outcome code
- RAX and RSP after execution
- serial byte length after execution

This is intentionally compact and stable enough for browser display, test assertions, and later export. It is not yet a full record/replay stream.

## Decode Cache Observability

The decode cache is deliberately small and conservative:

- It caches only instructions that fit entirely inside one physical page.
- Each entry records the physical RIP, page index, executable-page generation, and decoded instruction.
- Writes through `Machine::load_program` invalidate overlapping cached instructions.
- Snapshot restore starts with an empty cache; decoded instructions are recreated from architectural memory.
- Browser, Wasm, and native benchmark surfaces expose entries, hits, misses, and invalidations.

This is the first optimization hook that behaves like the future block and trace caches should behave: fast paths are derived from memory, checked against page generations, observable, and disposable.

The benchmark harness currently includes two useful extremes:

- `nop-sled`: mostly cold sequential decode; useful for measuring dispatch and decode-cache miss overhead.
- `hot-loop`: two-instruction loop; useful for measuring cache reuse and branch-heavy interpreter overhead.

## Research Direction

Next high-value steps:

1. Add fault trace events for decode, MMU, and execution failures.
2. Add page-table walk traces for page-fault diagnosis.
3. Add invalidation reason codes and per-page hotness counters.
4. Add deterministic trace export/import for bug reports.
5. Add reference-run comparison once a reference harness exists.
6. Add browser performance marks around worker run, snapshot, and restore commands.
