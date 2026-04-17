import test from "node:test";
import assert from "node:assert/strict";
import { execFile as execFileCallback } from "node:child_process";
import { readFile } from "node:fs/promises";
import { promisify } from "node:util";
import path from "node:path";
import { fileURLToPath } from "node:url";

const execFile = promisify(execFileCallback);
const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cliPath = path.join(projectRoot, "bin", "legolas.js");

test("cli prints version without requiring a command", async () => {
  const packageJson = JSON.parse(await readFile(path.join(projectRoot, "package.json"), "utf8"));
  const { stdout, stderr } = await execFile(process.execPath, [cliPath, "--version"], {
    cwd: projectRoot
  });

  assert.equal(stderr, "");
  assert.equal(stdout.trim(), packageJson.version);
});

test("cli rejects non-numeric visualization and optimize limits", async () => {
  await assert.rejects(
    execFile(process.execPath, [cliPath, "visualize", ".", "--limit", "nope"], {
      cwd: projectRoot
    }),
    (error) => {
      assert.equal(error.code, 1);
      assert.match(error.stderr, /--limit expects a positive integer/);
      return true;
    }
  );

  await assert.rejects(
    execFile(process.execPath, [cliPath, "optimize", ".", "--top", "NaN"], {
      cwd: projectRoot
    }),
    (error) => {
      assert.equal(error.code, 1);
      assert.match(error.stderr, /--top expects a positive integer/);
      return true;
    }
  );
});
