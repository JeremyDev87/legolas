import assert from "node:assert/strict";
import test from "node:test";

import { buildReleasePlan } from "../scripts/release-plan.mjs";
import { createReleaseFixture } from "./helpers/release-fixture.mjs";

test("release plan resolves stable tags to latest", async () => {
  const repoRoot = await createReleaseFixture();

  const plan = await buildReleasePlan({
    repoRoot,
    eventName: "push",
    githubRef: "refs/tags/v0.1.0",
    githubRefName: "v0.1.0",
    requestedReleaseTag: "v0.1.0",
  });

  assert.equal(plan.packageName, "@jeremyfellaz/legolas");
  assert.equal(plan.distTag, "latest");
  assert.equal(plan.isPrerelease, false);
});

test("release plan resolves prerelease tags to next", async () => {
  const repoRoot = await createReleaseFixture({ packageVersion: "0.1.1-beta.2" });

  const plan = await buildReleasePlan({
    repoRoot,
    eventName: "workflow_dispatch",
    githubRef: "refs/heads/master",
    githubRefName: "master",
    requestedReleaseTag: "v0.1.1-beta.2",
  });

  assert.equal(plan.packageVersion, "0.1.1-beta.2");
  assert.equal(plan.distTag, "next");
  assert.equal(plan.isPrerelease, true);
});
