import { execFile as execFileCallback } from "node:child_process";
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import { normalizeAnalysisForOracle, normalizeCliOutput } from "./parity-oracle-normalization.js";

const execFile = promisify(execFileCallback);
const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const parityFixtureRoot = path.join(projectRoot, "tests", "fixtures", "parity", "basic-app");
const oracleRoot = path.join(projectRoot, "tests", "oracles");
const projectRootPlaceholder = "<PROJECT_ROOT>";
const generatedAtPlaceholder = "<GENERATED_AT>";
const rustCliPath = path.join(
  projectRoot,
  "target",
  "debug",
  process.platform === "win32" ? "legolas-cli.exe" : "legolas-cli"
);

async function main() {
  await buildRustCli();

  await fs.mkdir(path.join(oracleRoot, "basic-app"), { recursive: true });
  await fs.mkdir(path.join(oracleRoot, "cli"), { recursive: true });
  await fs.mkdir(path.join(oracleRoot, "errors"), { recursive: true });

  await writeOracle("basic-app/scan.txt", await runCli(["scan", parityFixtureRoot]));
  await writeOracle("basic-app/visualize.txt", await runCli(["visualize", parityFixtureRoot]));
  await writeOracle("basic-app/optimize.txt", await runCli(["optimize", parityFixtureRoot]));
  await writeOracle("cli/help.txt", await runCli(["help"]));
  await writeOracle("cli/version.txt", await runCli(["--version"]));
  await writeOracle("errors/visualize-limit.txt", await runCliError(["visualize", parityFixtureRoot, "--limit", "nope"]));
  await writeOracle("errors/optimize-top.txt", await runCliError(["optimize", parityFixtureRoot, "--top", "NaN"]));

  const analysis = JSON.parse(await runCli(["scan", parityFixtureRoot, "--json"]));
  const normalizedAnalysis = normalizeAnalysisForOracle(analysis, {
    projectRootPlaceholder,
    generatedAtPlaceholder
  });

  await writeOracle("basic-app/scan.json", `${JSON.stringify(normalizedAnalysis, null, 2)}\n`);

  console.log("Rust parity fixtures generated.");
}

async function runCli(args) {
  const { stdout, stderr } = await execFile(rustCliPath, args, { cwd: projectRoot });

  if (stderr) {
    throw new Error(`Expected stdout-only command for ${args.join(" ")}, received stderr: ${stderr}`);
  }

  return normalizeCliOutput(stdout, parityFixtureRoot, projectRootPlaceholder);
}

async function runCliError(args) {
  try {
    await execFile(rustCliPath, args, { cwd: projectRoot });
  } catch (error) {
    if (
      error &&
      typeof error === "object" &&
      "stderr" in error &&
      typeof error.stderr === "string" &&
      "stdout" in error &&
      typeof error.stdout === "string"
    ) {
      if (error.stdout.length > 0) {
        throw new Error(`Expected stderr-only failure for ${args.join(" ")}, received stdout: ${error.stdout}`);
      }

      return normalizeCliOutput(error.stderr, parityFixtureRoot, projectRootPlaceholder);
    }

    throw error;
  }

  throw new Error(`Expected command to fail: ${args.join(" ")}`);
}

async function buildRustCli() {
  await execFile("cargo", ["build", "-q", "-p", "legolas-cli"], {
    cwd: projectRoot
  });
}

async function writeOracle(relativePath, contents) {
  const destination = path.join(oracleRoot, relativePath);
  await fs.mkdir(path.dirname(destination), { recursive: true });
  await fs.writeFile(destination, contents);
}

main().catch((error) => {
  const message = error instanceof Error ? error.stack ?? error.message : String(error);
  console.error(message);
  process.exitCode = 1;
});
