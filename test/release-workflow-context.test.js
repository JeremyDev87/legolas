import assert from "node:assert/strict";
import test from "node:test";

import {
  assertReleaseCommitReachableFromMaster,
  assertReleaseWorkflowContext,
} from "../scripts/assert-release-workflow-context.mjs";
import { createReleaseFixture } from "./helpers/release-fixture.mjs";

test("release workflow context accepts matching tag pushes", async () => {
  const repoRoot = await createReleaseFixture();

  const summary = await assertReleaseWorkflowContext({
    repoRoot,
    eventName: "push",
    githubRef: "refs/tags/v0.1.0",
    githubRefName: "v0.1.0",
    requestedReleaseTag: "v0.1.0",
  });

  assert.equal(summary.packageName, "@jeremyfellaz/legolas");
  assert.equal(summary.packageVersion, "0.1.0");
  assert.equal(summary.cargoVersion, "0.1.0");
  assert.equal(summary.releaseTag, "v0.1.0");
});

test("release workflow context accepts workflow_dispatch when the selected tag is provided", async () => {
  const repoRoot = await createReleaseFixture({ packageVersion: "0.1.1-beta.2" });

  const summary = await assertReleaseWorkflowContext({
    repoRoot,
    eventName: "workflow_dispatch",
    githubRef: "refs/heads/master",
    githubRefName: "master",
    requestedReleaseTag: "v0.1.1-beta.2",
  });

  assert.equal(summary.packageVersion, "0.1.1-beta.2");
  assert.equal(summary.releaseTag, "v0.1.1-beta.2");
});

test("release workflow context rejects non-tag push refs", async () => {
  const repoRoot = await createReleaseFixture();

  await assert.rejects(
    assertReleaseWorkflowContext({
      repoRoot,
      eventName: "push",
      githubRef: "refs/heads/master",
      githubRefName: "master",
      requestedReleaseTag: "v0.1.0",
    }),
    /release workflow must run from a tag ref/,
  );
});

test("release workflow context rejects mismatched release tags", async () => {
  const repoRoot = await createReleaseFixture();

  await assert.rejects(
    assertReleaseWorkflowContext({
      repoRoot,
      eventName: "workflow_dispatch",
      githubRef: "refs/heads/master",
      githubRefName: "master",
      requestedReleaseTag: "v0.1.1",
    }),
    /does not match package\.json version/,
  );
});

test("release workflow context rejects cargo version mismatches", async () => {
  const repoRoot = await createReleaseFixture({ cargoVersion: "0.2.0" });

  await assert.rejects(
    assertReleaseWorkflowContext({
      repoRoot,
      eventName: "push",
      githubRef: "refs/tags/v0.1.0",
      githubRefName: "v0.1.0",
      requestedReleaseTag: "v0.1.0",
    }),
    /Cargo\.toml version 0\.2\.0 does not match package\.json version 0\.1\.0/,
  );
});

test("release commit reachability accepts master-contained compare statuses", () => {
  assert.deepEqual(
    assertReleaseCommitReachableFromMaster({
      releaseCommitSha: "abc123",
      comparisonStatus: "identical",
    }),
    {
      releaseCommitSha: "abc123",
      comparisonStatus: "identical",
    },
  );

  assert.deepEqual(
    assertReleaseCommitReachableFromMaster({
      releaseCommitSha: "abc123",
      comparisonStatus: "ahead",
    }),
    {
      releaseCommitSha: "abc123",
      comparisonStatus: "ahead",
    },
  );
});

test("release commit reachability rejects commits outside master", () => {
  assert.throws(
    () =>
      assertReleaseCommitReachableFromMaster({
        releaseCommitSha: "abc123",
        comparisonStatus: "diverged",
      }),
    /not reachable from origin\/master/,
  );
});
