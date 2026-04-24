import os from "node:os";
import path from "node:path";
import { mkdtemp, mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const projectRoot = fileURLToPath(new URL("../..", import.meta.url));

export async function createReleaseFixture({
  packageName,
  packageVersion = "0.1.0",
  cargoVersion,
} = {}) {
  const fixtureRoot = await mkdtemp(path.join(os.tmpdir(), "legolas-release-fixture-"));
  await mkdir(path.join(fixtureRoot, "crates/legolas-cli"), { recursive: true });

  const packageManifest = JSON.parse(
    await readFile(path.join(projectRoot, "package.json"), "utf8"),
  );
  if (packageName) {
    packageManifest.name = packageName;
  }
  packageManifest.version = packageVersion;

  let cargoManifest = await readFile(
    path.join(projectRoot, "crates/legolas-cli/Cargo.toml"),
    "utf8",
  );
  cargoManifest = cargoManifest.replace(
    /^version\s*=\s*"[^"]+"$/m,
    `version = "${cargoVersion ?? packageVersion}"`,
  );

  await writeFile(
    path.join(fixtureRoot, "package.json"),
    `${JSON.stringify(packageManifest, null, 2)}\n`,
  );
  await writeFile(path.join(fixtureRoot, "crates/legolas-cli/Cargo.toml"), cargoManifest);

  return fixtureRoot;
}
