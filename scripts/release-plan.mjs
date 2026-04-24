import path from "node:path";
import { fileURLToPath } from "node:url";
import { appendFile } from "node:fs/promises";
import { assertReleaseWorkflowContext } from "./assert-release-workflow-context.mjs";
import { resolveReleasePlan } from "./lib/release.mjs";

export async function buildReleasePlan({
  repoRoot = process.cwd(),
  eventName,
  githubRef,
  githubRefName,
  requestedReleaseTag,
}) {
  const context = await assertReleaseWorkflowContext({
    repoRoot,
    eventName,
    githubRef,
    githubRefName,
    requestedReleaseTag,
  });
  const plan = resolveReleasePlan(context.releaseTag);

  return {
    ...context,
    distTag: plan.npmDistTag,
    isPrerelease: plan.isPrerelease,
  };
}

async function writeGithubOutputs(plan) {
  if (!process.env.GITHUB_OUTPUT) {
    return;
  }

  await appendFile(
    process.env.GITHUB_OUTPUT,
    [
      `package_name=${plan.packageName}`,
      `package_version=${plan.packageVersion}`,
      `cargo_version=${plan.cargoVersion}`,
      `release_tag=${plan.releaseTag}`,
      `dist_tag=${plan.distTag}`,
      `is_prerelease=${plan.isPrerelease}`,
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
    const plan = await buildReleasePlan({
      repoRoot: repoRootArg || process.cwd(),
      eventName,
      githubRef,
      githubRefName,
      requestedReleaseTag,
    });
    await writeGithubOutputs(plan);
    console.log(
      `Validated release plan for ${plan.releaseTag} (dist-tag: ${plan.distTag}, prerelease: ${plan.isPrerelease}).`,
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error(`Release plan validation failed: ${message}`);
    process.exitCode = 1;
  }
}
