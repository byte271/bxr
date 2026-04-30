import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

const root = resolve(".");
const isWindows = process.platform === "win32";

const checks = [
  {
    name: "rustfmt",
    command: "cargo",
    args: ["fmt", "--all", "--check"],
  },
  {
    name: "workspace tests",
    command: "cargo",
    args: ["test", "--workspace", "--target-dir", ".verify-target"],
    env: { CARGO_INCREMENTAL: "0" },
  },
  {
    name: "clippy",
    command: "cargo",
    args: ["clippy", "--workspace", "--target-dir", ".verify-target", "--", "-D", "warnings"],
  },
  {
    name: "build wasm",
    command: "npm",
    args: ["run", "build:wasm"],
  },
  {
    name: "syntax: build-wasm",
    command: "node",
    args: ["--check", "scripts/build-wasm.mjs"],
  },
  {
    name: "syntax: web server",
    command: "node",
    args: ["--check", "web/server.mjs"],
  },
  {
    name: "syntax: browser main",
    command: "node",
    args: ["--check", "web/src/main.js"],
  },
  {
    name: "syntax: machine worker",
    command: "node",
    args: ["--check", "web/src/machine.worker.js"],
  },
  {
    name: "wasm smoke",
    command: "node",
    args: ["scripts/wasm-smoke.mjs"],
  },
];

for (const check of checks) {
  console.log(`\n==> ${check.name}`);
  const result = spawnSync(check.command, check.args, {
    cwd: root,
    env: { ...process.env, ...check.env },
    shell: isWindows,
    stdio: "inherit",
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

console.log("\nquality gate passed");
