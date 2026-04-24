import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";

import {
  bumpReleaseVersion,
  createManualBumpBranchName,
} from "../scripts/bump-release-version.mjs";
import { createReleaseFixture } from "./helpers/release-fixture.mjs";

test("bumpReleaseVersion updates package.json and cargo manifest together", async () => {
  const repoRoot = await createReleaseFixture();

  await bumpReleaseVersion("v0.1.1", repoRoot);

  const packageManifest = JSON.parse(
    await readFile(path.join(repoRoot, "package.json"), "utf8"),
  );
  const cargoManifest = await readFile(
    path.join(repoRoot, "crates/legolas-cli/Cargo.toml"),
    "utf8",
  );

  assert.equal(packageManifest.version, "0.1.1");
  assert.match(cargoManifest, /^version\s*=\s*"0\.1\.1"$/m);
});

test("createManualBumpBranchName is deterministic per tag and base", () => {
  assert.equal(
    createManualBumpBranchName("v0.1.1", "master"),
    createManualBumpBranchName("0.1.1", "master"),
  );
  assert.notEqual(
    createManualBumpBranchName("v0.1.1", "master"),
    createManualBumpBranchName("v0.1.1-beta.1", "master"),
  );
});
