import test from "node:test";
import assert from "node:assert/strict";
import { execFile as execFileCallback } from "node:child_process";
import { readFile } from "node:fs/promises";
import { promisify } from "node:util";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { cliPath, normalizeCliOutput, parityFixtureRoot, projectRoot, readOracle } from "../test-support/parity-oracles.js";

const execFile = promisify(execFileCallback);
const localProjectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("cli prints version without requiring a command", async () => {
  const packageJson = JSON.parse(await readFile(path.join(localProjectRoot, "package.json"), "utf8"));
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
      assert.equal(error.stdout, "");
      assert.equal(normalizeCliOutput(error.stderr), "legolas: --limit expects a positive integer\n");
      return true;
    }
  );

  await assert.rejects(
    execFile(process.execPath, [cliPath, "optimize", ".", "--top", "NaN"], {
      cwd: projectRoot
    }),
    (error) => {
      assert.equal(error.code, 1);
      assert.equal(error.stdout, "");
      assert.equal(normalizeCliOutput(error.stderr), "legolas: --top expects a positive integer\n");
      return true;
    }
  );
});

test("cli matches the checked-in help and version oracles", async () => {
  const help = await execFile(process.execPath, [cliPath, "help"], {
    cwd: projectRoot
  });
  const version = await execFile(process.execPath, [cliPath, "--version"], {
    cwd: projectRoot
  });

  assert.equal(normalizeCliOutput(help.stdout), await readOracle("cli", "help.txt"));
  assert.equal(normalizeCliOutput(version.stdout), await readOracle("cli", "version.txt"));
  assert.equal(help.stderr, "");
  assert.equal(version.stderr, "");
});

test("cli matches the checked-in text report oracles for the parity fixture", async () => {
  const commands = [
    { args: ["scan", parityFixtureRoot], oracle: ["basic-app", "scan.txt"] },
    { args: ["visualize", parityFixtureRoot], oracle: ["basic-app", "visualize.txt"] },
    { args: ["optimize", parityFixtureRoot], oracle: ["basic-app", "optimize.txt"] }
  ];

  for (const command of commands) {
    const { stdout, stderr } = await execFile(process.execPath, [cliPath, ...command.args], {
      cwd: projectRoot
    });

    assert.equal(normalizeCliOutput(stdout), await readOracle(...command.oracle));
    assert.equal(stderr, "");
  }
});

test("cli matches the checked-in validation error oracles", async () => {
  const errorCases = [
    {
      args: ["visualize", parityFixtureRoot, "--limit", "nope"],
      oracle: ["errors", "visualize-limit.txt"]
    },
    {
      args: ["optimize", parityFixtureRoot, "--top", "NaN"],
      oracle: ["errors", "optimize-top.txt"]
    }
  ];

  for (const errorCase of errorCases) {
    try {
      await execFile(process.execPath, [cliPath, ...errorCase.args], {
        cwd: projectRoot
      });
      assert.fail(`Expected command to fail: ${errorCase.args.join(" ")}`);
    } catch (error) {
      assert.equal(error.code, 1);
      assert.equal(error.stdout, "");
      assert.equal(normalizeCliOutput(error.stderr), await readOracle(...errorCase.oracle));
    }
  }
});
