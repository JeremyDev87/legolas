#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { missingBinaryMessage, resolveCurrentHostSupport } from "./platform-support.js";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const hostSupport = resolveCurrentHostSupport();

if (!hostSupport.supported) {
  exitMissingBinary(hostSupport.hostKey);
}

const binaryPath = path.join(projectRoot, "vendor", hostSupport.targetTriple, hostSupport.binaryName);

if (!existsSync(binaryPath)) {
  exitMissingBinary(hostSupport.hostKey);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: "inherit"
});

if (result.error) {
  console.error(`legolas: ${result.error.message}`);
  process.exit(1);
}

if (typeof result.status === "number") {
  process.exit(result.status);
}

if (result.signal) {
  process.kill(process.pid, result.signal);
}

process.exit(1);

function exitMissingBinary(hostKey) {
  console.error(missingBinaryMessage(hostKey));
  process.exit(1);
}
