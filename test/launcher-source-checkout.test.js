import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageManifest = JSON.parse(readFileSync(path.join(projectRoot, "package.json"), "utf8"));
const sourceCheckoutLauncher = path.join(projectRoot, "bin", "legolas.js");

test("source checkout launcher falls back to cargo when vendor binary is absent", () => {
  const result = spawnSync(process.execPath, ["./bin/legolas.js", "--version"], {
    cwd: projectRoot,
    encoding: "utf8"
  });

  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim().split(/\r?\n/).at(-1), packageManifest.version);
});

test("source checkout launcher preserves caller cwd for relative target paths", () => {
  const fixtureRoot = path.join(projectRoot, "tests", "fixtures", "parity", "basic-app");
  const result = spawnSync(process.execPath, [sourceCheckoutLauncher, "scan", "."], {
    cwd: fixtureRoot,
    encoding: "utf8"
  });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /Legolas scan for basic-parity-app/);
  assert.match(result.stdout, /Project root: .*tests\/fixtures\/parity\/basic-app/);
  assert.match(result.stdout, /Scanned 1 source files/);
});
