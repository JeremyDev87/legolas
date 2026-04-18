import { chmod, copyFile, mkdir, stat } from "node:fs/promises";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const projectRoot = fileURLToPath(new URL("..", import.meta.url));
const { options } = parseArgs(process.argv.slice(2));
const vendorRoot = path.resolve(
  options.vendorDir ?? process.env.LEGOLAS_VENDOR_DIR ?? path.join(projectRoot, "vendor"),
);
const hostTriple = readRustHostTriple();
const stagedBinaryName = stagedBinaryNameForTriple(hostTriple);
const sourceBinary = path.resolve(
  options.sourceBinary ??
    process.env.LEGOLAS_SOURCE_BINARY ??
    path.join(projectRoot, "target", "release", sourceBinaryNameForTriple(hostTriple)),
);
const stagedBinary = path.join(vendorRoot, hostTriple, stagedBinaryName);

await assertFileExists(sourceBinary, `Built binary not found at ${sourceBinary}`);
await mkdir(path.dirname(stagedBinary), { recursive: true });
await copyFile(sourceBinary, stagedBinary);

if (!hostTriple.includes("windows")) {
  await chmod(stagedBinary, 0o755);
}

console.log(`Staged ${sourceBinary} -> ${stagedBinary}`);

function readRustHostTriple() {
  const result = spawnSync("rustc", ["-vV"], {
    cwd: projectRoot,
    encoding: "utf8",
  });

  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout || "Failed to read rustc host triple.");
  }

  const hostLine = result.stdout
    .split(/\r?\n/)
    .find((line) => line.startsWith("host: "));

  if (!hostLine) {
    throw new Error("rustc -vV did not include a host triple.");
  }

  return hostLine.replace(/^host:\s*/, "").trim();
}

function sourceBinaryNameForTriple(targetTriple) {
  return targetTriple.includes("windows") ? "legolas-cli.exe" : "legolas-cli";
}

function stagedBinaryNameForTriple(targetTriple) {
  return targetTriple.includes("windows") ? "legolas.exe" : "legolas";
}

async function assertFileExists(filePath, message) {
  const fileStat = await stat(filePath).catch(() => null);

  if (!fileStat?.isFile()) {
    throw new Error(message);
  }
}

function parseArgs(argv) {
  const options = {};
  const positionals = [];

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];

    if (token === "--vendor-dir" || token === "--source-binary") {
      const next = argv[index + 1];
      if (!next) {
        throw new Error(`${token} expects a value.`);
      }

      if (token === "--vendor-dir") {
        options.vendorDir = next;
      } else {
        options.sourceBinary = next;
      }

      index += 1;
      continue;
    }

    if (token.startsWith("--vendor-dir=")) {
      options.vendorDir = token.slice("--vendor-dir=".length);
      continue;
    }

    if (token.startsWith("--source-binary=")) {
      options.sourceBinary = token.slice("--source-binary=".length);
      continue;
    }

    positionals.push(token);
  }

  return { options, positionals };
}
