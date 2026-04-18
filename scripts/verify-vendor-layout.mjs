import { readdir, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = fileURLToPath(new URL("..", import.meta.url));
const { options, positionals } = parseArgs(process.argv.slice(2));
const vendorRoot = path.resolve(
  options.vendorDir ?? process.env.LEGOLAS_VENDOR_DIR ?? path.join(projectRoot, "vendor"),
);

const vendorEntries = await readdir(vendorRoot, { withFileTypes: true }).catch((error) => {
  throw new Error(`Unable to read vendor root ${vendorRoot}: ${error.message}`);
});
const tripleDirectories = vendorEntries
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name)
  .sort();
const expectedTriples = positionals.length > 0 ? [...positionals].sort() : tripleDirectories;

if (expectedTriples.length === 0) {
  throw new Error(`No staged vendor binaries found under ${vendorRoot}.`);
}

for (const expectedTriple of expectedTriples) {
  if (!tripleDirectories.includes(expectedTriple)) {
    throw new Error(`Expected vendor triple ${expectedTriple} under ${vendorRoot}.`);
  }

  const tripleRoot = path.join(vendorRoot, expectedTriple);
  const expectedBinary = binaryNameForTriple(expectedTriple);
  const entries = await readdir(tripleRoot, { withFileTypes: true });
  const files = entries.filter((entry) => entry.isFile()).map((entry) => entry.name).sort();
  const nestedDirectories = entries.filter((entry) => entry.isDirectory()).map((entry) => entry.name);

  if (nestedDirectories.length > 0) {
    throw new Error(
      `Vendor triple ${expectedTriple} should not contain nested directories: ${nestedDirectories.join(", ")}`,
    );
  }

  if (files.length !== 1 || files[0] !== expectedBinary) {
    throw new Error(
      `Vendor triple ${expectedTriple} must contain exactly ${expectedBinary}, found: ${files.join(", ") || "<empty>"}`,
    );
  }

  const binaryPath = path.join(tripleRoot, expectedBinary);
  const binaryStat = await stat(binaryPath);
  if (!binaryStat.isFile()) {
    throw new Error(`Expected staged binary at ${binaryPath}.`);
  }
}

console.log(`Validated ${expectedTriples.length} staged vendor ${expectedTriples.length === 1 ? "binary" : "binaries"}.`);

function binaryNameForTriple(targetTriple) {
  return targetTriple.includes("windows") ? "legolas.exe" : "legolas";
}

function parseArgs(argv) {
  const options = {};
  const positionals = [];

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];

    if (token === "--vendor-dir") {
      const next = argv[index + 1];
      if (!next) {
        throw new Error(`${token} expects a value.`);
      }

      options.vendorDir = next;
      index += 1;
      continue;
    }

    if (token.startsWith("--vendor-dir=")) {
      options.vendorDir = token.slice("--vendor-dir=".length);
      continue;
    }

    positionals.push(token);
  }

  return { options, positionals };
}
