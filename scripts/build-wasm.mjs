import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const targetDir = ".wasm-target";
const cargo = spawnSync(
  "cargo",
  ["build", "-p", "bxr-wasm", "--target", "wasm32-unknown-unknown", "--target-dir", targetDir],
  {
    cwd: resolve("."),
    env: { ...process.env, CARGO_INCREMENTAL: "0" },
    shell: process.platform === "win32",
    stdio: "inherit",
  },
);

if (cargo.status !== 0) {
  process.exit(cargo.status ?? 1);
}

const source = resolve(targetDir, "wasm32-unknown-unknown", "debug", "bxr_wasm.wasm");
const destination = resolve("web", "wasm", "bxr_wasm.wasm");
mkdirSync(dirname(destination), { recursive: true });
copyFileSync(source, destination);
console.log(`copied ${source} -> ${destination}`);
