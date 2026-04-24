import assert from "node:assert/strict";
import { cp, mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import {
  releaseWiringRequiredFiles,
  validateReleaseWiring,
} from "../scripts/validate-release-wiring.mjs";
import { createReleaseFixture } from "./helpers/release-fixture.mjs";

async function copyReleaseWiringSupportFiles(repoRoot) {
  for (const relativePath of Object.values(releaseWiringRequiredFiles)) {
    if (relativePath === "package.json" || relativePath === "crates/legolas-cli/Cargo.toml") {
      continue;
    }

    const targetPath = path.join(repoRoot, relativePath);
    await mkdir(path.dirname(targetPath), { recursive: true });
    await cp(path.join(process.cwd(), relativePath), targetPath);
  }
}

test("release wiring validation passes for the checked-in GitHub automation files", async () => {
  const summary = await validateReleaseWiring(process.cwd());

  assert.equal(summary.checkedFiles.length, 14);
  assert.equal(summary.packageName, "@jeremyfellaz/legolas");
});

test("release wiring validation accepts bumped manifest versions", async () => {
  const repoRoot = await createReleaseFixture({ packageVersion: "0.1.1" });
  await copyReleaseWiringSupportFiles(repoRoot);

  const summary = await validateReleaseWiring(repoRoot);

  assert.equal(summary.checkedFiles.length, 14);
  assert.equal(summary.packageName, "@jeremyfellaz/legolas");
});

test("manual bump workflow validates the actual bump PR head for release candidate dispatch", async () => {
  const workflow = await readFile(".github/workflows/manual-release-bump.yml", "utf8");

  assert.match(workflow, /name: Resolve release candidate target/);
  assert.match(workflow, /refs\/remotes\/origin\/\$\{\{ steps\.meta\.outputs\.branch \}\}/);
  assert.match(workflow, /target_sha="\$\{\{ steps\.candidate_target\.outputs\.sha \}\}"/);
});

test("manual bump workflow dispatches a dispatch-enabled CI workflow", async () => {
  const ciWorkflow = await readFile(".github/workflows/ci.yml", "utf8");
  const manualBumpWorkflow = await readFile(".github/workflows/manual-release-bump.yml", "utf8");

  assert.match(ciWorkflow, /workflow_dispatch:/);
  assert.match(manualBumpWorkflow, /gh workflow run ci\.yml --ref/);
});

test("release workflow rejects tags whose commit is outside master", async () => {
  const workflow = await readFile(".github/workflows/release.yml", "utf8");

  assert.match(workflow, /name: Verify release commit is reachable from master/);
  assert.match(workflow, /compare\/\$\{RELEASE_COMMIT_SHA\}\.\.\.master/);
  assert.match(workflow, /assertReleaseCommitReachableFromMaster/);
});
