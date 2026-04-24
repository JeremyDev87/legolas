import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  bumpReleaseVersion,
  createManualBumpBranchName,
} from "../scripts/bump-release-version.mjs";
import { createReleaseFixture } from "./helpers/release-fixture.mjs";

const projectRoot = fileURLToPath(new URL("..", import.meta.url));

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

test("bump module can be imported from node eval with tag argv", () => {
  const output = execFileSync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      "import { createManualBumpBranchName } from './scripts/bump-release-version.mjs'; console.log(createManualBumpBranchName(process.argv[1]));",
      "v0.1.1",
    ],
    { cwd: projectRoot, encoding: "utf8" },
  );

  assert.match(output, /^codex\/manual-bump-master-v0-1-1-[0-9a-f]{8}\n$/);
});
