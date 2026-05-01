const terminal = document.querySelector("#terminal");
const state = document.querySelector("#state");
const detail = document.querySelector("#detail");
const registers = document.querySelector("#registers");
const flags = document.querySelector("#flags");
const controls = document.querySelector("#controls");
const instruction = document.querySelector("#instruction");
const cache = document.querySelector("#cache");
const memory = document.querySelector("#memory");
const trace = document.querySelector("#trace");
const snapshots = document.querySelector("#snapshots");
const worker = new Worker(new URL("./machine.worker.js", import.meta.url), {
  type: "module",
});

let lastSnapshot = null;

worker.addEventListener("message", (event) => {
  const message = event.data;
  switch (message.type) {
    case "MachineCreated":
    case "Paused":
    case "Running":
    case "Stopped":
      state.textContent = message.state;
      detail.textContent = message.detail ?? "";
      break;
    case "SerialOutput":
      terminal.textContent += message.text;
      break;
    case "SerialState":
      terminal.textContent = message.text;
      break;
    case "DebugState":
      renderDebug(message.debug);
      break;
    case "SnapshotReady":
      lastSnapshot = message.snapshot;
      state.textContent = "snapshot";
      detail.textContent = `snapshot ${message.snapshot.id}`;
      renderSnapshots(message.snapshot);
      break;
    case "Fault":
      state.textContent = "faulted";
      detail.textContent = message.detail;
      break;
    default:
      detail.textContent = `unknown event ${message.type}`;
  }
});

document.querySelector("#boot").addEventListener("click", () => {
  terminal.textContent = "";
  worker.postMessage({ type: "CreateMachine" });
});

document.querySelector("#step").addEventListener("click", () => {
  worker.postMessage({ type: "Step" });
});

document.querySelector("#run").addEventListener("click", () => {
  worker.postMessage({ type: "RunDemo" });
});

document.querySelector("#pause").addEventListener("click", () => {
  worker.postMessage({ type: "Pause" });
});

document.querySelector("#snapshot").addEventListener("click", () => {
  worker.postMessage({ type: "Snapshot" });
});

document.querySelector("#restore").addEventListener("click", () => {
  if (lastSnapshot) {
    terminal.textContent = "";
    worker.postMessage({ type: "Restore", snapshot: lastSnapshot });
  }
});

function renderDebug(debug) {
  if (!debug) {
    return;
  }
  renderPairs(registers, debug.registers);
  renderPairs(flags, debug.flags);
  renderPairs(controls, debug.controls);
  renderPairs(instruction, debug.instruction);
  renderPairs(cache, debug.cache);
  memory.textContent = debug.memory.join("\n");
  renderTrace(debug.trace);
  renderPairs(snapshots, {
    available: debug.snapshotAvailable ? "yes" : "no",
    mode: debug.mode,
    serial: `${debug.serialLength} bytes`,
    "virtual ticks": debug.virtualTicks,
  });
}

function renderTrace(entries) {
  trace.replaceChildren(
    ...entries.map((entry) => {
      const item = document.createElement("li");
      item.textContent = `${entry.sequence} ${entry.ripBefore} ${entry.operation} ${entry.bytes} -> ${entry.ripAfter}`;
      return item;
    }),
  );
}

function renderSnapshots(snapshot) {
  renderPairs(snapshots, {
    id: snapshot.id,
    mode: snapshot.mode,
    state: snapshot.state,
  });
}

function renderPairs(target, values) {
  target.replaceChildren(
    ...Object.entries(values).flatMap(([key, value]) => {
      const term = document.createElement("dt");
      term.textContent = key;
      const description = document.createElement("dd");
      description.textContent = String(value);
      return [term, description];
    }),
  );
}
