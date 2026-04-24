import assert from "node:assert/strict";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

export const releaseWiringRequiredFiles = {
  packageManifest: "package.json",
  cargoManifest: "crates/legolas-cli/Cargo.toml",
  ciWorkflow: ".github/workflows/ci.yml",
  releaseWorkflow: ".github/workflows/release.yml",
  releaseCandidateWorkflow: ".github/workflows/release-candidate.yml",
  manualReleaseBumpWorkflow: ".github/workflows/manual-release-bump.yml",
  releaseContextAssertion: "scripts/assert-release-workflow-context.mjs",
  releasePlan: "scripts/release-plan.mjs",
  releaseHelpers: "scripts/lib/release.mjs",
  releaseBumpScript: "scripts/bump-release-version.mjs",
  packedInstallSmoke: "scripts/smoke-packed-install.mjs",
  releaseContextTest: "test/release-workflow-context.test.js",
  releasePlanTest: "test/release-plan.test.js",
  releaseBumpTest: "test/bump-release-version.test.js",
  githubActionWiringTest: "test/github-action-wiring.test.js",
};

export async function validateReleaseWiring(repoRoot = process.cwd()) {
  const fileContents = await readRequiredFiles(repoRoot);
  const packageManifest = JSON.parse(fileContents.packageManifest);
  validatePackageManifest(packageManifest);
  validateCargoManifest(fileContents.cargoManifest, packageManifest.version);
  validateCiWorkflow(fileContents.ciWorkflow);
  validateReleaseWorkflow(fileContents.releaseWorkflow);
  validateReleaseCandidateWorkflow(fileContents.releaseCandidateWorkflow);
  validateManualReleaseBumpWorkflow(fileContents.manualReleaseBumpWorkflow);
  validateReleaseContextAssertion(fileContents.releaseContextAssertion);
  validateReleasePlanScript(fileContents.releasePlan);
  validateReleaseHelpers(fileContents.releaseHelpers);
  validateReleaseBumpScript(fileContents.releaseBumpScript);
  validatePackedInstallSmoke(fileContents.packedInstallSmoke);
  validateTests(fileContents.releaseContextTest, fileContents.releasePlanTest, fileContents.releaseBumpTest, fileContents.githubActionWiringTest);

  return {
    checkedFiles: Object.values(releaseWiringRequiredFiles),
    packageName: packageManifest.name,
  };
}

async function readRequiredFiles(repoRoot) {
  const entries = await Promise.all(
    Object.entries(releaseWiringRequiredFiles).map(async ([key, relativePath]) => {
      const absolutePath = path.join(repoRoot, relativePath);
      return [key, await readFile(absolutePath, "utf8")];
    }),
  );
  return Object.fromEntries(entries);
}

function validatePackageManifest(packageManifest) {
  assert.equal(packageManifest.name, "@jeremyfellaz/legolas");
  assert.equal(packageManifest.publishConfig?.access, "public");
  assert.equal(
    packageManifest.scripts?.["test:release-contract"],
    "node --test test/release-workflow-context.test.js test/release-plan.test.js test/bump-release-version.test.js test/github-action-wiring.test.js",
  );
  assert.equal(packageManifest.scripts?.["pack:smoke"], "node ./scripts/smoke-packed-install.mjs");
}

function validateCargoManifest(cargoManifestText, expectedVersion) {
  assertContains(cargoManifestText, 'name = "legolas-cli"', "cargo manifest package name");
  const versionMatch = cargoManifestText.match(/^version\s*=\s*"([^"]+)"$/m);

  assert.ok(versionMatch?.[1], "cargo manifest version field is missing");
  assert.equal(
    versionMatch[1],
    expectedVersion,
    `cargo manifest version ${versionMatch[1]} must match package.json version ${expectedVersion}`,
  );
}

function validateCiWorkflow(ciWorkflowText) {
  assertContains(ciWorkflowText, "workflow_dispatch:", "ci manual dispatch trigger");
  assertContains(ciWorkflowText, "Release Contract", "ci release contract job");
  assertContains(ciWorkflowText, "node ./scripts/validate-release-wiring.mjs", "ci release wiring validation");
  assertContains(ciWorkflowText, "npm run test:release-contract", "ci release contract tests");
  assertContains(ciWorkflowText, "npm run pack:smoke", "ci packed install smoke");
}

function validateReleaseWorkflow(releaseText) {
  assertContains(releaseText, 'push:\n    tags:\n      - "v*"', "release tag push trigger");
  assertContains(releaseText, "workflow_dispatch:", "release manual trigger");
  assertContains(releaseText, "release_tag:", "release workflow dispatch tag input");
  assertContains(releaseText, "validate-release-context:", "release context validation job");
  assertContains(releaseText, "Build release plan", "release plan step");
  assertContains(releaseText, "node ./scripts/release-plan.mjs", "release plan command");
  assertContains(releaseText, "format('refs/tags/{0}', inputs.release_tag)", "release exact tag rerun checkout");
  assertContains(releaseText, "fetch-depth: 1", "release shallow checkout");
  assertContains(releaseText, "release_commit_sha: ${{ steps.capture_release_commit.outputs.release_commit_sha }}", "release commit sha output");
  assertContains(releaseText, "Verify release commit is reachable from master", "release master reachability gate");
  assertContains(releaseText, "compare/${RELEASE_COMMIT_SHA}...master", "release master compare check");
  assertContains(releaseText, "assertReleaseCommitReachableFromMaster", "release master reachability assertion");
  assertContains(releaseText, "ref: ${{ needs.validate-release-context.outputs.release_commit_sha }}", "release downstream sha checkout");
  assertContains(releaseText, "actions/upload-artifact@v4", "release binary artifact upload");
  assertContains(releaseText, "actions/download-artifact@v4", "release binary artifact download");
  assertContains(releaseText, "npm run pack:smoke", "release packed install smoke");
  assertContains(releaseText, '--tag "$RELEASE_DIST_TAG"', "release npm dist-tag publish");
  assertContains(releaseText, "gh release upload", "release asset upload");
  assertContains(releaseText, "gh release view \"$TAG_NAME\" --json databaseId --jq .databaseId", "release REST release id lookup");
  assert.ok(
    !releaseText.includes("gh release view \"$TAG_NAME\" --json id --jq .id"),
    "release workflow must not pass GraphQL release node IDs to REST release endpoints",
  );
  assert.ok(!releaseText.includes("fetch-depth: 0"), "release workflow should not use full-history checkout");
}

function validateReleaseCandidateWorkflow(releaseCandidateText) {
  assertContains(releaseCandidateText, "workflow_dispatch:", "release candidate manual trigger");
  assertContains(releaseCandidateText, "target_sha:", "release candidate target sha input");
  assertContains(releaseCandidateText, "TARGET_SHA_INPUT: ${{ inputs.target_sha }}", "release candidate target sha env handoff");
  assertContains(releaseCandidateText, "target_sha must be a full 40-character commit SHA", "release candidate target sha validation");
  assertContains(releaseCandidateText, "Ensure release tag does not already exist", "release candidate tag guard");
  assertContains(releaseCandidateText, "node ./scripts/validate-release-wiring.mjs", "release candidate release wiring validation");
  assertContains(releaseCandidateText, "npm run test:release-contract", "release candidate release tests");
  assertContains(releaseCandidateText, "cargo build --release -p legolas-cli", "release candidate release build");
  assertContains(releaseCandidateText, "node ./scripts/verify-vendor-layout.mjs", "release candidate vendor verification");
  assertContains(releaseCandidateText, "npm run pack:check", "release candidate pack check");
  assertContains(releaseCandidateText, "npm run pack:smoke", "release candidate packed install smoke");
  assertContains(releaseCandidateText, "node ./scripts/smoke-ci-command.mjs launcher", "release candidate packaged launcher smoke");
  assert.ok(
    !releaseCandidateText.includes('"${{ inputs.target_sha }}"'),
    "release candidate workflow should not interpolate target_sha directly inside bash",
  );
}

function validateManualReleaseBumpWorkflow(manualReleaseBumpText) {
  assertContains(manualReleaseBumpText, "workflow_dispatch:", "manual bump workflow trigger");
  assertContains(manualReleaseBumpText, "dry_run:", "manual bump dry-run input");
  assertContains(manualReleaseBumpText, "INPUT_TAG: ${{ inputs.tag }}", "manual bump tag env handoff");
  assertContains(manualReleaseBumpText, 'tag="$INPUT_TAG"', "manual bump tag shell variable usage");
  assertContains(manualReleaseBumpText, "node ./scripts/bump-release-version.mjs", "manual bump script usage");
  assertContains(manualReleaseBumpText, "npm run test:release-contract", "manual bump release contract tests");
  assertContains(manualReleaseBumpText, "gh workflow run ci.yml --ref", "manual bump CI dispatch");
  assertContains(manualReleaseBumpText, "Resolve release candidate target", "manual bump candidate sha resolution");
  assertContains(manualReleaseBumpText, "steps.candidate_target.outputs.sha", "manual bump candidate target output");
  assertContains(manualReleaseBumpText, "gh workflow run release-candidate.yml --ref master -f target_sha=", "manual bump candidate dispatch");
  assertContains(manualReleaseBumpText, "skip-changelog", "manual bump skip-changelog label");
  assertContains(manualReleaseBumpText, "LEGOLAS_RELEASE_BOT_TOKEN", "manual bump dedicated PR token");
  assertContains(manualReleaseBumpText, "Unable to create a draft PR", "manual bump hard-fail PR creation");
  assert.ok(
    !manualReleaseBumpText.includes('tag="${{ inputs.tag }}"'),
    "manual bump workflow should not interpolate input tag directly inside bash",
  );
}

function validateReleaseContextAssertion(releaseContextAssertionText) {
  assertContains(releaseContextAssertionText, "release workflow must run from a tag ref", "release context tag gate");
  assertContains(releaseContextAssertionText, "assertReleaseCommitReachableFromMaster", "release context master reachability helper");
  assertContains(releaseContextAssertionText, "not reachable from origin/master", "release context master reachability error");
  assertContains(releaseContextAssertionText, "package.json name must be @jeremyfellaz/legolas", "release context package name gate");
  assertContains(releaseContextAssertionText, "crates/legolas-cli/Cargo.toml version", "release context cargo version gate");
  assertContains(releaseContextAssertionText, "GITHUB_OUTPUT", "release context GitHub output export");
}

function validateReleasePlanScript(releasePlanText) {
  assertContains(releasePlanText, "distTag", "release plan dist-tag logic");
  assertContains(releasePlanText, "isPrerelease", "release plan prerelease flag");
  assertContains(releasePlanText, "package_name", "release plan package output");
  assertContains(releasePlanText, "resolveReleasePlan", "release plan helper wiring");
}

function validateReleaseHelpers(releaseHelpersText) {
  assertContains(releaseHelpersText, 'npmDistTag: isPrerelease ? "next" : "latest"', "release helper dist-tag mapping");
  assertContains(releaseHelpersText, "compareReleaseVersions", "release helper version comparison");
  assertContains(releaseHelpersText, "assertReleaseUpgrade", "release helper upgrade assertion");
}

function validateReleaseBumpScript(releaseBumpScriptText) {
  assertContains(releaseBumpScriptText, "versionFilePaths", "release bump version path list");
  assertContains(releaseBumpScriptText, "createManualBumpBranchName", "release bump branch naming");
  assertContains(releaseBumpScriptText, "crates/legolas-cli/Cargo.toml", "release bump cargo manifest update");
  assertContains(releaseBumpScriptText, "assertReleaseUpgrade", "release bump upgrade guard");
}

function validatePackedInstallSmoke(packedInstallSmokeText) {
  assertContains(packedInstallSmokeText, "npm", "packed install smoke npm usage");
  assertContains(packedInstallSmokeText, "pack", "packed install smoke pack command");
  assertContains(packedInstallSmokeText, "install", "packed install smoke install command");
  assertContains(packedInstallSmokeText, "node_modules", "packed install smoke installed bin path");
  assertContains(packedInstallSmokeText, "--version", "packed install smoke version command");
  assertContains(packedInstallSmokeText, "exec", "packed install smoke Windows npm exec invocation");
}

function validateTests(releaseContextTest, releasePlanTest, releaseBumpTest, githubActionWiringTest) {
  assertContains(releaseContextTest, "assertReleaseWorkflowContext", "release context test coverage");
  assertContains(releasePlanTest, "buildReleasePlan", "release plan test coverage");
  assertContains(releaseBumpTest, "bumpReleaseVersion", "release bump test coverage");
  assertContains(githubActionWiringTest, "validateReleaseWiring", "release wiring test coverage");
}

function assertContains(text, expected, label) {
  assert.match(text, new RegExp(escapeRegExp(expected), "m"), `${label} is missing`);
}

function escapeRegExp(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

const isDirectExecution =
  process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);

if (isDirectExecution) {
  try {
    const summary = await validateReleaseWiring();
    console.log(`Validated release wiring in ${summary.checkedFiles.length} files.`);
    console.log(`Package name: ${summary.packageName}`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Release wiring validation failed: ${message}`);
    process.exitCode = 1;
  }
}
