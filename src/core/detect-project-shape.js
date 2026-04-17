import path from "node:path";

import { exists } from "./workspace.js";

const FRAMEWORK_MARKERS = [
  { name: "Next.js", packages: ["next"], files: ["next.config.js", "next.config.mjs", "next.config.ts"] },
  { name: "Vite", packages: ["vite"], files: ["vite.config.js", "vite.config.ts", "vite.config.mjs"] },
  { name: "Webpack", packages: ["webpack"], files: ["webpack.config.js", "webpack.config.ts"] },
  { name: "Rollup", packages: ["rollup"], files: ["rollup.config.js", "rollup.config.mjs", "rollup.config.ts"] },
  { name: "Astro", packages: ["astro"], files: ["astro.config.mjs", "astro.config.ts"] },
  { name: "Nuxt", packages: ["nuxt"], files: ["nuxt.config.ts", "nuxt.config.js"] },
  { name: "React", packages: ["react"] },
  { name: "Vue", packages: ["vue"] },
  { name: "Svelte", packages: ["svelte", "@sveltejs/kit"] }
];

export async function detectFrameworks(projectRoot, manifest) {
  const allDependencies = new Set([
    ...Object.keys(manifest.dependencies ?? {}),
    ...Object.keys(manifest.devDependencies ?? {})
  ]);

  const detected = [];

  for (const marker of FRAMEWORK_MARKERS) {
    const packageHit = marker.packages?.some((pkg) => allDependencies.has(pkg));
    const fileHit = marker.files
      ? await anyExists(marker.files.map((file) => path.join(projectRoot, file)))
      : false;

    if (packageHit || fileHit) {
      detected.push(marker.name);
    }
  }

  return detected;
}

export async function detectPackageManager(projectRoot, manifest) {
  const explicit = manifest.packageManager;
  if (typeof explicit === "string" && explicit.length > 0) {
    return explicit;
  }

  const checks = [
    { file: "pnpm-lock.yaml", name: "pnpm" },
    { file: "yarn.lock", name: "yarn" },
    { file: "package-lock.json", name: "npm" },
    { file: "bun.lockb", name: "bun" },
    { file: "bun.lock", name: "bun" }
  ];

  for (const check of checks) {
    if (await exists(path.join(projectRoot, check.file))) {
      return check.name;
    }
  }

  return "unknown";
}

async function anyExists(paths) {
  for (const currentPath of paths) {
    if (await exists(currentPath)) {
      return true;
    }
  }
  return false;
}
