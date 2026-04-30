let machine = null;
let snapshotCounter = 0;
let wasmCorePromise = null;

self.addEventListener("message", async (event) => {
  const command = event.data;
  try {
    switch (command.type) {
      case "CreateMachine":
        machine = await createMachine();
        post({
          type: "MachineCreated",
          state: machine.state,
          detail: `${machine.mode} profile ready`,
        });
        postDebugState();
        break;
      case "Step":
        requireMachine();
        stepMachine(machine);
        postDebugState();
        break;
      case "RunDemo":
        requireMachine();
        runDemo(machine);
        postDebugState();
        break;
      case "Pause":
        requireMachine();
        machine.state = "paused";
        post({ type: "Paused", state: machine.state, detail: "paused at worker boundary" });
        postDebugState();
        break;
      case "Snapshot":
        requireMachine();
        post({ type: "SnapshotReady", snapshot: captureSnapshot(machine) });
        postDebugState();
        break;
      case "Restore":
        machine = await restoreSnapshot(command.snapshot);
        post({ type: "Paused", state: machine.state, detail: "snapshot restored" });
        postSerialState(machine);
        postDebugState();
        break;
      default:
        throw new Error(`unknown command ${command.type}`);
    }
  } catch (error) {
    post({ type: "Fault", detail: String(error.message ?? error) });
  }
});

async function createMachine() {
  const wasmCore = await loadWasmCore();
  if (wasmCore) {
    wasmCore.bxr_machine_create_demo();
  }
  return {
    profile: "bxr-minimal-x64-v1",
    state: "paused",
    serial: "",
    mode: wasmCore ? "wasm" : "js-fallback",
    wasmCore,
  };
}

function stepMachine(target) {
  if (target.wasmCore) {
    const ok = target.wasmCore.bxr_machine_step();
    syncFromWasm(target);
    postSerialState(target);
    post({
      type: target.state === "halted" ? "Stopped" : "Paused",
      state: target.state,
      detail: ok ? "stepped one instruction" : "step did not advance",
    });
    return;
  }

  target.serial += "step\\n";
  target.state = "paused";
  postSerialState(target);
  post({ type: "Paused", state: target.state, detail: "fallback step" });
}

function runDemo(target) {
  target.state = "running";
  post({ type: "Running", state: target.state, detail: "running demo guest" });
  if (target.wasmCore) {
    const steps = target.wasmCore.bxr_machine_run_until_halt(64);
    syncFromWasm(target);
    postSerialState(target);
    post({
      type: "Stopped",
      state: target.state,
      detail: `${target.mode} halted after ${steps} steps`,
    });
    return;
  }

  target.serial += runFallbackDemo();
  postSerialState(target);
  target.state = "halted";
  post({ type: "Stopped", state: target.state, detail: `${target.mode} demo halted` });
}

function captureSnapshot(target) {
  if (target.wasmCore) {
    target.wasmCore.bxr_machine_snapshot_capture();
  }
  return {
    id: `snap-${++snapshotCounter}`,
    profile: target.profile,
    state: target.state,
    serial: target.serial,
    mode: target.mode,
  };
}

async function restoreSnapshot(snapshot) {
  if (!snapshot || snapshot.profile !== "bxr-minimal-x64-v1") {
    throw new Error("invalid or unsupported snapshot");
  }
  const wasmCore = await loadWasmCore();
  if (wasmCore && snapshot.mode === "wasm") {
    wasmCore.bxr_machine_snapshot_restore();
    const restored = {
      profile: snapshot.profile,
      state: "paused",
      serial: "",
      mode: "wasm",
      wasmCore,
    };
    syncFromWasm(restored);
    return restored;
  }
  return {
    profile: snapshot.profile,
    state: "paused",
    serial: snapshot.serial,
    mode: snapshot.mode ?? "js-fallback",
    wasmCore: null,
  };
}

function requireMachine() {
  if (!machine) {
    throw new Error("machine has not been created");
  }
}

function post(message) {
  self.postMessage(message);
}

async function loadWasmCore() {
  wasmCorePromise ??= instantiateWasmCore();
  return wasmCorePromise;
}

async function instantiateWasmCore() {
  try {
    const response = await fetch("/wasm/bxr_wasm.wasm");
    if (!response.ok) {
      return null;
    }
    const module = await WebAssembly.instantiateStreaming(response, {});
    return module.instance.exports;
  } catch {
    return null;
  }
}

function runWasmDemo(exports) {
  const steps = exports.bxr_demo_steps();
  const halted = exports.bxr_demo_halted();
  const rax = exports.bxr_demo_rax();
  const serial = [];
  const len = exports.bxr_demo_serial_len();
  for (let i = 0; i < len; i += 1) {
    serial.push(exports.bxr_demo_serial_byte(i));
  }
  const text = String.fromCharCode(...serial);
  return `BXR wasm core serial: ${text}\\nsteps=${steps} halted=${halted} rax=${rax}\\n`;
}

function runFallbackDemo() {
  return "BXR JS fallback demo says hello through serial\\n";
}

function syncFromWasm(target) {
  const exports = target.wasmCore;
  target.state = stateName(exports.bxr_machine_state_code());
  target.serial = serialFromWasm(exports);
}

function serialFromWasm(exports) {
  const serial = [];
  const len = exports.bxr_machine_serial_len();
  for (let i = 0; i < len; i += 1) {
    serial.push(exports.bxr_machine_serial_byte(i));
  }
  return String.fromCharCode(...serial);
}

function postSerialState(target) {
  post({ type: "SerialState", text: target.serial });
}

function postDebugState() {
  if (!machine) {
    return;
  }
  post({ type: "DebugState", debug: readDebugState(machine) });
}

function readDebugState(target) {
  if (!target.wasmCore) {
    return {
      mode: target.mode,
      serialLength: target.serial.length,
      snapshotAvailable: false,
      registers: { rip: "0x0", rax: "0x0", rsp: "0x0" },
      flags: {},
      controls: {},
      instruction: {},
      cache: {},
      memory: [],
      trace: [],
    };
  }

  const exports = target.wasmCore;
  const rflags = exports.bxr_machine_rflags();
  const rip = exports.bxr_machine_rip();
  return {
    mode: target.mode,
    serialLength: Number(exports.bxr_machine_serial_len()),
    snapshotAvailable: exports.bxr_machine_snapshot_available() === 1,
    registers: {
      rip: hex(rip),
      rax: hex(exports.bxr_machine_gpr(0)),
      rbx: hex(exports.bxr_machine_gpr(3)),
      rcx: hex(exports.bxr_machine_gpr(1)),
      rdx: hex(exports.bxr_machine_gpr(2)),
      rsp: hex(exports.bxr_machine_gpr(4)),
      rbp: hex(exports.bxr_machine_gpr(5)),
      r8: hex(exports.bxr_machine_gpr(8)),
      r15: hex(exports.bxr_machine_gpr(15)),
    },
    flags: decodeFlags(rflags),
    controls: {
      cr0: hex(exports.bxr_machine_control(0)),
      cr2: hex(exports.bxr_machine_control(2)),
      cr3: hex(exports.bxr_machine_control(3)),
      cr4: hex(exports.bxr_machine_control(4)),
      efer: hex(exports.bxr_machine_control(0x0efe)),
    },
    instruction: readInstruction(exports),
    cache: readDecodeCache(exports),
    memory: readMemoryWindow(exports, rip),
    trace: readTrace(exports),
  };
}

function stateName(code) {
  switch (Number(code)) {
    case 1:
      return "paused";
    case 2:
      return "running";
    case 3:
      return "halted";
    case 4:
      return "faulted";
    default:
      return "idle";
  }
}

function hex(value) {
  const bigint = typeof value === "bigint" ? value : BigInt(value);
  return `0x${bigint.toString(16).padStart(16, "0")}`;
}

function decodeFlags(value) {
  const flags = typeof value === "bigint" ? value : BigInt(value);
  return {
    cf: bit(flags, 0),
    pf: bit(flags, 2),
    af: bit(flags, 4),
    zf: bit(flags, 6),
    sf: bit(flags, 7),
    if: bit(flags, 9),
    df: bit(flags, 10),
    of: bit(flags, 11),
  };
}

function bit(value, index) {
  return (value & (1n << BigInt(index))) !== 0n ? "1" : "0";
}

function readInstruction(exports) {
  const len = Number(exports.bxr_machine_current_instruction_len());
  const code = Number(exports.bxr_machine_current_instruction_code());
  const bytes = [];
  for (let i = 0; i < len; i += 1) {
    bytes.push(byteHex(exports.bxr_machine_current_instruction_byte(i)));
  }
  return {
    op: operationName(code),
    len,
    bytes: bytes.join(" "),
    translated: hex(exports.bxr_machine_translate_execute(exports.bxr_machine_rip())),
  };
}

function readDecodeCache(exports) {
  return {
    entries: Number(exports.bxr_machine_decode_cache_entries()),
    hits: exports.bxr_machine_decode_cache_hits().toString(),
    misses: exports.bxr_machine_decode_cache_misses().toString(),
    invalidations: exports.bxr_machine_decode_cache_invalidations().toString(),
  };
}

function readMemoryWindow(exports, rip) {
  const lines = [];
  const start = rip > 16n ? rip - 16n : 0n;
  for (let row = 0; row < 4; row += 1) {
    const addr = start + BigInt(row * 16);
    const bytes = [];
    for (let col = 0; col < 16; col += 1) {
      const value = exports.bxr_machine_memory_byte(addr + BigInt(col));
      bytes.push(value > 0xff ? "??" : byteHex(value));
    }
    lines.push(`${hex(addr)}  ${bytes.join(" ")}`);
  }
  return lines;
}

function readTrace(exports) {
  const len = Number(exports.bxr_machine_trace_len());
  const start = Math.max(0, len - 12);
  const entries = [];
  for (let i = start; i < len; i += 1) {
    const bytes = [];
    const byteLen = Number(exports.bxr_machine_trace_instruction_len(i));
    for (let j = 0; j < byteLen; j += 1) {
      bytes.push(byteHex(exports.bxr_machine_trace_instruction_byte(i, j)));
    }
    entries.push({
      sequence: exports.bxr_machine_trace_sequence(i).toString(),
      ripBefore: hex(exports.bxr_machine_trace_rip_before(i)),
      ripAfter: hex(exports.bxr_machine_trace_rip_after(i)),
      operation: operationName(Number(exports.bxr_machine_trace_operation_code(i))),
      outcome: exports.bxr_machine_trace_outcome_code(i) === 2 ? "halt" : "continue",
      bytes: bytes.join(" "),
    });
  }
  return entries;
}

function operationName(code) {
  switch (code) {
    case 1:
      return "nop";
    case 2:
      return "ret";
    case 3:
      return "int3";
    case 4:
      return "hlt";
    case 5:
      return "syscall";
    case 6:
      return "mov-imm";
    case 7:
      return "push";
    case 8:
      return "pop";
    case 9:
      return "add-acc-imm";
    case 10:
      return "jmp-rel32";
    case 11:
      return "out-imm8-al";
    default:
      return "unknown";
  }
}

function byteHex(value) {
  return Number(value).toString(16).padStart(2, "0");
}
