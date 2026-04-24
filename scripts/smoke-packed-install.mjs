import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = fileURLToPath(new URL("..", import.meta.url));
const packageManifest = JSON.parse(
  await readFile(path.join(projectRoot, "package.json"), "utf8"),
);
const smokeRoot = await mkdtemp(path.join(os.tmpdir(), "legolas-packed-install-"));
const packDir = path.join(smokeRoot, "pack");
const installDir = path.join(smokeRoot, "install");
const cacheDir = path.join(smokeRoot, "npm-cache");
const scanFixturePath = path.join(projectRoot, "tests", "fixtures", "parity", "basic-app");

await mkdir(packDir, { recursive: true });
await mkdir(installDir, { recursive: true });
await writeFile(
  path.join(installDir, "package.json"),
  `${JSON.stringify({ private: true, name: "legolas-packed-install-smoke" }, null, 2)}\n`,
);

const pack = run(npmCommand(), [
  "pack",
  "--json",
  "--pack-destination",
  packDir,
  "--cache",
  cacheDir,
]);
const [packResult] = JSON.parse(pack.stdout);
const tarballPath = path.isAbsolute(packResult.filename)
  ? packResult.filename
  : path.join(packDir, packResult.filename);

await assertFile(tarballPath, "npm pack did not produce a tarball");

run(npmCommand(), [
  "install",
  "--no-audit",
  "--no-fund",
  "--cache",
  cacheDir,
  tarballPath,
], { cwd: installDir });

const binPath = path.join(
  installDir,
  "node_modules",
  ".bin",
  process.platform === "win32" ? "legolas.cmd" : "legolas",
);
await assertFile(binPath, "installed package did not expose node_modules/.bin/legolas");

const version = runInstalledBinary(binPath, ["--version"], { cwd: installDir });
const actualVersion = version.stdout.trim();

if (actualVersion !== packageManifest.version) {
  throw new Error(
    `installed legolas --version returned ${actualVersion}, expected ${packageManifest.version}`,
  );
}

const scan = runInstalledBinary(binPath, ["scan", scanFixturePath], { cwd: installDir });

if (!scan.stdout.includes("Legolas scan for basic-parity-app")) {
  throw new Error(`installed legolas scan did not report the basic fixture:\n${scan.stdout}`);
}

if (!scan.stdout.includes("Scanned 1 source files")) {
  throw new Error(`installed legolas scan did not report the expected source count:\n${scan.stdout}`);
}

if (scan.stderr !== "") {
  throw new Error(`installed legolas scan wrote unexpected stderr:\n${scan.stderr}`);
}

console.log(`Verified packed install legolas --version ${actualVersion} and scan basic-parity-app.`);

async function assertFile(filePath, message) {
  const fileStat = await stat(filePath).catch(() => null);

  if (!fileStat?.isFile()) {
    throw new Error(`${message}: ${filePath}`);
  }
}

function npmCommand() {
  return process.platform === "win32" ? "npm.cmd" : "npm";
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? projectRoot,
    encoding: "utf8",
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(" ")} failed with ${result.status}\n${result.stdout}${result.stderr}`,
    );
  }

  return result;
}

function runInstalledBinary(binPath, args, options = {}) {
  if (process.platform !== "win32") {
    return run(binPath, args, options);
  }

  return run(npmCommand(), ["exec", "--", "legolas", ...args], options);
}
