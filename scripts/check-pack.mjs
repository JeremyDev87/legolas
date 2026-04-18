import { spawn } from "node:child_process";
import { readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
const projectRoot = fileURLToPath(new URL("..", import.meta.url));

const rootAllowlist = new Set(["LICENSE", "package.json"]);
const rootReadmePattern = /^README(\.[^.]+)?\.md$/i;
const packageDirs = ["bin", "vendor"];
const vendorReadmePath = "vendor/README.md";
const vendorBinaryPattern = /^vendor\/[^/]+\/legolas(?:\.exe)?$/;

const expectedFiles = new Set(await collectExpectedFiles());
const packedFiles = await collectPackedFiles();
const invalidVendorFiles = packedFiles.filter(isInvalidVendorFile);
const stagedVendorBinaries = packedFiles.filter((filePath) => vendorBinaryPattern.test(filePath));

const unexpectedFiles = packedFiles.filter((filePath) => !expectedFiles.has(filePath));
const missingFiles = [...expectedFiles].filter((filePath) => !packedFiles.includes(filePath));

if (
  unexpectedFiles.length > 0 ||
  missingFiles.length > 0 ||
  invalidVendorFiles.length > 0 ||
  stagedVendorBinaries.length === 0
) {
  console.error("Package contents validation failed.");

  if (unexpectedFiles.length > 0) {
    console.error("Unexpected files:");
    for (const filePath of unexpectedFiles) {
      console.error(`- ${filePath}`);
    }
  }

  if (missingFiles.length > 0) {
    console.error("Missing files:");
    for (const filePath of missingFiles) {
      console.error(`- ${filePath}`);
    }
  }

  if (invalidVendorFiles.length > 0) {
    console.error("Invalid vendor layout files:");
    for (const filePath of invalidVendorFiles) {
      console.error(`- ${filePath}`);
    }
  }

  if (stagedVendorBinaries.length === 0) {
    console.error("Missing staged vendor binary under vendor/<triple>/legolas[.exe].");
  }

  process.exit(1);
}

console.log(`Validated ${packedFiles.length} packaged files.`);

async function collectExpectedFiles() {
  const rootEntries = await readdir(projectRoot, { withFileTypes: true });
  const expected = new Set();

  for (const entry of rootEntries) {
    if (!entry.isFile()) {
      continue;
    }

    if (rootAllowlist.has(entry.name) || rootReadmePattern.test(entry.name)) {
      expected.add(entry.name);
    }
  }

  for (const directory of packageDirs) {
    const absolutePath = path.join(projectRoot, directory);
    const files = await walkFiles(absolutePath);

    for (const filePath of files) {
      expected.add(toPosixPath(path.relative(projectRoot, filePath)));
    }
  }

  return [...expected].sort();
}

async function walkFiles(directoryPath) {
  const entries = await readdir(directoryPath, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const absolutePath = path.join(directoryPath, entry.name);

    if (entry.isDirectory()) {
      files.push(...(await walkFiles(absolutePath)));
      continue;
    }

    if (entry.isFile()) {
      files.push(absolutePath);
    }
  }

  return files;
}

async function collectPackedFiles() {
  const stdout = await runPackCommand();

  const [packResult] = JSON.parse(stdout);

  if (!packResult?.files) {
    throw new Error("npm pack did not return a file list.");
  }

  return packResult.files
    .map((entry) => entry.path)
    .sort();
}

function toPosixPath(filePath) {
  return filePath.split(path.sep).join("/");
}

function isInvalidVendorFile(filePath) {
  return filePath.startsWith("vendor/") && filePath !== vendorReadmePath && !vendorBinaryPattern.test(filePath);
}

function runPackCommand() {
  return new Promise((resolve, reject) => {
    const child = process.platform === "win32"
      ? spawn("cmd.exe", ["/d", "/s", "/c", "npm pack --dry-run --json --cache ./.npm-cache"], { cwd: projectRoot })
      : spawn("npm", ["pack", "--dry-run", "--json", "--cache", "./.npm-cache"], { cwd: projectRoot });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });

    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });

    child.on("error", (error) => {
      reject(error);
    });

    child.on("close", (code) => {
      if (code === 0) {
        resolve(stdout);
        return;
      }

      reject(new Error(stderr || stdout || `npm pack exited with code ${code}`));
    });
  });
}
