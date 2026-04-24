#!/usr/bin/env node

import crypto from "node:crypto";
import { realpathSync } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { assertReleaseUpgrade, resolveReleasePlan } from "./lib/release.mjs";

export const versionFilePaths = [
  "package.json",
  "crates/legolas-cli/Cargo.toml",
];

export function normalizeReleaseTag(input) {
  return input.startsWith("v") ? input : `v${input}`;
}

export function createManualBumpBranchName(input, baseRef = "master") {
  const normalizedTag = normalizeReleaseTag(input);
  const branchBase = baseRef
    .replace(/[./+]/g, "-")
    .replace(/[^0-9A-Za-z-]/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  const branchVersion = normalizedTag
    .slice(1)
    .replace(/[.+]/g, "-")
    .replace(/[^0-9A-Za-z-]/g, "-");
  const branchHash = crypto
    .createHash("sha256")
    .update(`${baseRef}\0${normalizedTag}`)
    .digest("hex")
    .slice(0, 8);

  return `codex/manual-bump-${branchBase}-v${branchVersion}-${branchHash}`;
}

export function updatePackageManifest(pkg, version) {
  return {
    ...pkg,
    version,
  };
}

export function updateCargoManifest(cargoManifest, version) {
  if (!/^version\s*=\s*"[^"]+"$/m.test(cargoManifest)) {
    throw new Error("Unable to find crates/legolas-cli/Cargo.toml version field");
  }

  return cargoManifest.replace(/^version\s*=\s*"[^"]+"$/m, `version = "${version}"`);
}

function readCargoVersion(cargoManifest) {
  const match = cargoManifest.match(/^version\s*=\s*"([^"]+)"$/m);

  if (!match?.[1]) {
    throw new Error("Unable to read crates/legolas-cli/Cargo.toml version");
  }

  return match[1];
}

export async function bumpReleaseVersion(input, repoRoot = process.cwd()) {
  const normalizedTag = normalizeReleaseTag(input);
  const plan = resolveReleasePlan(normalizedTag);
  const packageManifestPath = path.join(repoRoot, "package.json");
  const cargoManifestPath = path.join(repoRoot, "crates/legolas-cli/Cargo.toml");
  const packageManifest = JSON.parse(await fs.readFile(packageManifestPath, "utf8"));
  const cargoManifest = await fs.readFile(cargoManifestPath, "utf8");
  const cargoVersion = readCargoVersion(cargoManifest);

  if (cargoVersion !== packageManifest.version) {
    throw new Error(
      `crates/legolas-cli/Cargo.toml version ${cargoVersion} does not match package.json version ${packageManifest.version}`,
    );
  }

  assertReleaseUpgrade(packageManifest.version, plan.version);

  const nextPackageManifest = updatePackageManifest(packageManifest, plan.version);
  const nextCargoManifest = updateCargoManifest(cargoManifest, plan.version);

  await fs.writeFile(packageManifestPath, `${JSON.stringify(nextPackageManifest, null, 2)}\n`);
  await fs.writeFile(cargoManifestPath, nextCargoManifest);

  return {
    version: plan.version,
    tag: plan.tag,
    isPrerelease: plan.isPrerelease,
    manifestPaths: versionFilePaths.map((relativePath) => path.join(repoRoot, relativePath)),
  };
}

async function main() {
  const input = process.argv[2];
  const repoRoot = process.argv[3] ?? process.cwd();

  if (!input) {
    console.error("Usage: node ./scripts/bump-release-version.mjs <tag-or-version> [repo-root]");
    process.exit(1);
  }

  const result = await bumpReleaseVersion(input, repoRoot);
  console.log(`tag=${result.tag}`);
  console.log(`version=${result.version}`);
  console.log(`isPrerelease=${result.isPrerelease}`);
  console.log(`manifestCount=${result.manifestPaths.length}`);
}

function isDirectExecutionEntry() {
  if (!process.argv[1]) {
    return false;
  }

  try {
    return (
      realpathSync(fileURLToPath(import.meta.url))
      === realpathSync(path.resolve(process.argv[1]))
    );
  } catch {
    return false;
  }
}

const isDirectExecution = isDirectExecutionEntry();

if (isDirectExecution) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
