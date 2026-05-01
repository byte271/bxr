import { readFile } from "node:fs/promises";

const wasmPath = new URL("../web/wasm/bxr_wasm.wasm", import.meta.url);
const bytes = await readFile(wasmPath);
const { instance } = await WebAssembly.instantiate(bytes, {});
const exports = instance.exports;

const requiredExports = [
  "bxr_abi_version",
  "bxr_machine_create_demo",
  "bxr_machine_step",
  "bxr_machine_run_until_halt",
  "bxr_machine_snapshot_capture",
  "bxr_machine_snapshot_restore",
  "bxr_machine_serial_len",
  "bxr_machine_trace_len",
  "bxr_machine_virtual_ticks",
  "bxr_machine_decode_cache_entries",
  "bxr_machine_decode_cache_hits",
  "bxr_machine_decode_cache_misses",
  "bxr_machine_decode_cache_invalidations",
];

for (const name of requiredExports) {
  if (typeof exports[name] !== "function") {
    throw new Error(`missing Wasm export: ${name}`);
  }
}

if (exports.bxr_abi_version() !== 1) {
  throw new Error("unexpected ABI version");
}
if (exports.bxr_machine_create_demo() !== 1) {
  throw new Error("failed to create demo machine");
}
if (exports.bxr_machine_step() !== 1) {
  throw new Error("failed to step demo machine");
}
if (exports.bxr_machine_virtual_ticks() !== 1n) {
  throw new Error("virtual clock did not advance after one instruction");
}
if (exports.bxr_machine_decode_cache_entries() < 1) {
  throw new Error("decode cache did not record the first decoded instruction");
}
if (exports.bxr_machine_decode_cache_misses() < 1n) {
  throw new Error("decode cache miss counter did not advance");
}
if (exports.bxr_machine_snapshot_capture() !== 1) {
  throw new Error("failed to capture snapshot");
}

const remainingSteps = exports.bxr_machine_run_until_halt(64);
if (remainingSteps !== 10) {
  throw new Error(`unexpected remaining step count: ${remainingSteps}`);
}
if (exports.bxr_machine_serial_len() !== 5) {
  throw new Error("demo serial output did not complete");
}
if (exports.bxr_machine_trace_len() === 0) {
  throw new Error("trace log did not record execution");
}
if (exports.bxr_machine_snapshot_restore() !== 1) {
  throw new Error("failed to restore snapshot");
}
if (exports.bxr_machine_virtual_ticks() !== 1n) {
  throw new Error("restored machine did not preserve virtual time");
}
if (exports.bxr_machine_decode_cache_entries() !== 0) {
  throw new Error("restored machine should rebuild transient decode cache");
}

console.log("wasm smoke ok: execution, trace, snapshot, and decode-cache ABI passed");
