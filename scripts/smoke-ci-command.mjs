import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const mode = process.argv[2];
const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const targetPath = path.join(workspaceRoot, "tests/fixtures/parity/basic-app");

if (mode !== "cargo" && mode !== "launcher") {
  console.error('usage: node ./scripts/smoke-ci-command.mjs <cargo|launcher>');
  process.exit(1);
}

const invocation =
  mode === "cargo"
    ? {
        cmd: "cargo",
        args: ["run", "-q", "-p", "legolas-cli", "--", "ci", targetPath],
      }
    : {
        cmd: process.execPath,
        args: [path.join(workspaceRoot, "bin/legolas.js"), "ci", targetPath],
      };

const result = spawnSync(invocation.cmd, invocation.args, {
  cwd: workspaceRoot,
  encoding: "utf8",
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

if (result.status !== 1) {
  console.error(`expected ci smoke to exit 1, got ${result.status ?? "null"}`);
  if (result.stdout) {
    process.stderr.write(result.stdout);
  }
  if (result.stderr) {
    process.stderr.write(result.stderr);
  }
  process.exit(1);
}

if (!result.stdout.includes("Legolas CI for basic-parity-app")) {
  console.error("expected ci smoke stdout summary for basic-parity-app");
  process.stderr.write(result.stdout);
  process.exit(1);
}

if (!result.stderr.includes("CI gate failed:")) {
  console.error("expected ci smoke stderr prefix");
  process.stderr.write(result.stderr);
  process.exit(1);
}

process.stdout.write(result.stdout);
process.stderr.write(result.stderr);
