import assert from "node:assert/strict";
import path from "node:path";
import { appendFile, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const expectedPackageName = "@jeremyfellaz/legolas";
const RELEASE_TAG_PATTERN = /^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$/;
const MASTER_REACHABLE_COMPARE_STATUSES = new Set(["ahead", "identical"]);

export function assertReleaseCommitReachableFromMaster({
  releaseCommitSha,
  comparisonStatus,
}) {
  assert.ok(releaseCommitSha, "release commit sha is required");
  assert.ok(
    MASTER_REACHABLE_COMPARE_STATUSES.has(comparisonStatus),
    `Tagged commit ${releaseCommitSha} is not reachable from origin/master (compare status: ${comparisonStatus || "<empty>"})`,
  );

  return {
    releaseCommitSha,
    comparisonStatus,
  };
}

async function readPackageManifest(repoRoot) {
  const packageJsonPath = path.join(repoRoot, "package.json");
  return JSON.parse(await readFile(packageJsonPath, "utf8"));
}

async function readCargoCliVersion(repoRoot) {
  const cargoManifestPath = path.join(repoRoot, "crates/legolas-cli/Cargo.toml");
  const cargoManifest = await readFile(cargoManifestPath, "utf8");
  const match = cargoManifest.match(/^version\s*=\s*"([^"]+)"$/m);

  assert.ok(match?.[1], "missing crates/legolas-cli/Cargo.toml version");
  return match[1];
}

export async function assertReleaseWorkflowContext({
  repoRoot = process.cwd(),
  eventName,
  githubRef,
  githubRefName,
  requestedReleaseTag,
}) {
  assert.ok(
    eventName === "push" || eventName === "workflow_dispatch",
    `unsupported release workflow event: ${eventName}`,
  );

  if (eventName === "push") {
    assert.ok(
      githubRef?.startsWith("refs/tags/"),
      `release workflow must run from a tag ref, received ${githubRef}`,
    );
  }

  const releaseTag = eventName === "push" ? githubRefName || githubRef?.slice("refs/tags/".length) : requestedReleaseTag;
  assert.ok(releaseTag, "release tag is required");
  assert.match(releaseTag, RELEASE_TAG_PATTERN, `release tag must look like v1.2.3 or v1.2.3-beta.1, received ${releaseTag}`);

  if (eventName === "push") {
    assert.equal(
      githubRefName || githubRef.slice("refs/tags/".length),
      releaseTag,
      `selected ref ${githubRefName || githubRef.slice("refs/tags/".length)} does not match release tag ${releaseTag}`,
    );
  }

  const packageManifest = await readPackageManifest(repoRoot);
  assert.equal(
    packageManifest.name,
    expectedPackageName,
    `package.json name must be @jeremyfellaz/legolas, received ${packageManifest.name}`,
  );

  const packageVersion = packageManifest.version;
  const cargoVersion = await readCargoCliVersion(repoRoot);
  const expectedReleaseTag = `v${packageVersion}`;

  assert.equal(
    releaseTag,
    expectedReleaseTag,
    `release tag ${releaseTag} does not match package.json version ${expectedReleaseTag}`,
  );
  assert.equal(
    cargoVersion,
    packageVersion,
    `crates/legolas-cli/Cargo.toml version ${cargoVersion} does not match package.json version ${packageVersion}`,
  );

  return {
    eventName,
    packageName: packageManifest.name,
    packageVersion,
    cargoVersion,
    releaseTag,
  };
}

async function writeGithubOutputs(summary) {
  if (!process.env.GITHUB_OUTPUT) {
    return;
  }

  await appendFile(
    process.env.GITHUB_OUTPUT,
    [
      `package_name=${summary.packageName}`,
      `package_version=${summary.packageVersion}`,
      `cargo_version=${summary.cargoVersion}`,
      `release_tag=${summary.releaseTag}`,
    ].join("\n") + "\n",
    "utf8",
  );
}

const isDirectExecution =
  process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);

if (isDirectExecution) {
  const [eventName, githubRef, githubRefName, requestedReleaseTag, repoRootArg] =
    process.argv.slice(2);

  try {
    const summary = await assertReleaseWorkflowContext({
      repoRoot: repoRootArg || process.cwd(),
      eventName,
      githubRef,
      githubRefName,
      requestedReleaseTag,
    });
    await writeGithubOutputs(summary);
    console.log(`Validated release workflow context for ${summary.releaseTag}.`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Release workflow context validation failed: ${message}`);
    process.exitCode = 1;
  }
}
