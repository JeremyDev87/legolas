import test from "node:test";
import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";

import { analyzeProject } from "../src/core/analyze-project.js";

test("analyzeProject reports heavy dependencies, duplicates, and tree-shaking warnings", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-project-"));

  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "fixture-app",
      dependencies: {
        react: "^18.3.0",
        lodash: "^4.17.21",
        "chart.js": "^4.4.1",
        "react-icons": "^5.2.1"
      },
      devDependencies: {
        vite: "^5.2.0"
      }
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "pnpm-lock.yaml"),
    [
      "lockfileVersion: '9.0'",
      "",
      "packages:",
      "  lodash@4.17.21:",
      "    resolution: {integrity: sha512-foo}",
      "  lodash@4.17.20:",
      "    resolution: {integrity: sha512-bar}",
      "  chart.js@4.4.1:",
      "    resolution: {integrity: sha512-baz}"
    ].join("\n")
  );

  await fs.writeFile(
    path.join(tempRoot, "src", "Dashboard.tsx"),
    [
      "import { useEffect } from 'react';",
      "import _ from 'lodash';",
      "import { FaUser } from 'react-icons';",
      "import { Chart } from 'chart.js';",
      "",
      "export function Dashboard() {",
      "  useEffect(() => {",
      "    console.log(_.shuffle([1, 2, 3]), Chart, FaUser);",
      "  }, []);",
      "  return null;",
      "}"
    ].join("\n")
  );

  const analysis = await analyzeProject(tempRoot);

  assert.equal(analysis.packageSummary.name, "fixture-app");
  assert.ok(analysis.frameworks.includes("Vite"));
  assert.ok(analysis.heavyDependencies.some((item) => item.name === "chart.js"));
  assert.ok(analysis.duplicatePackages.some((item) => item.name === "lodash"));
  assert.ok(analysis.lazyLoadCandidates.some((item) => item.name === "chart.js"));
  assert.ok(analysis.treeShakingWarnings.some((item) => item.packageName === "lodash"));
});

test("analyzeProject returns a stable missing-package error for file inputs outside a package root", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-orphan-file-"));
  const orphanFile = path.join(tempRoot, "example.ts");

  await fs.writeFile(orphanFile, "export const value = 1;\n");

  await assert.rejects(
    analyzeProject(orphanFile),
    /package\.json not found near .*legolas-orphan-file-/
  );
});

test("analyzeProject ignores import-like text inside strings and comments", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-false-positive-"));

  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "string-fixture",
      dependencies: {
        lodash: "^4.17.21"
      }
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "src", "docs.ts"),
    [
      "// import fake from 'lodash';",
      "const guide = \"import fake from 'lodash'\";",
      "const template = `require('lodash') and import('lodash')`;",
      "export const value = guide + template;"
    ].join("\n")
  );

  const analysis = await analyzeProject(tempRoot);

  assert.equal(analysis.sourceSummary.importedPackages, 0);
  assert.equal(analysis.treeShakingWarnings.length, 0);
  assert.ok(analysis.unusedDependencyCandidates.some((item) => item.name === "lodash"));
});

test("analyzeProject tracks dynamic imports as package usage", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-dynamic-import-"));

  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "dynamic-fixture",
      dependencies: {
        "chart.js": "^4.4.1"
      }
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "src", "Dashboard.tsx"),
    "export async function loadChart() { return import('chart.js'); }\n"
  );

  const analysis = await analyzeProject(tempRoot);

  assert.equal(analysis.sourceSummary.dynamicImports, 1);
  assert.ok(analysis.lazyLoadCandidates.length === 0);
  assert.ok(analysis.unusedDependencyCandidates.every((item) => item.name !== "chart.js"));
  assert.ok(analysis.heavyDependencies.some((item) => item.name === "chart.js"));
});

test("analyzeProject ignores type-only imports and exports for runtime usage", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-type-only-"));

  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "types-fixture",
      dependencies: {
        lodash: "^4.17.21"
      }
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "src", "types.ts"),
    [
      "import type { DebouncedFunc } from 'lodash';",
      "export type { DebouncedFunc as Debounced } from 'lodash';",
      "export type OnlyType = DebouncedFunc<string>;"
    ].join("\n")
  );

  const analysis = await analyzeProject(tempRoot);

  assert.equal(analysis.sourceSummary.importedPackages, 0);
  assert.equal(analysis.treeShakingWarnings.length, 0);
  assert.ok(analysis.unusedDependencyCandidates.some((item) => item.name === "lodash"));
});

test("analyzeProject ignores import-like text inside JSX, Vue, and Svelte templates", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-template-text-"));

  await fs.mkdir(path.join(tempRoot, "src"), { recursive: true });
  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "template-fixture",
      dependencies: {
        lodash: "^4.17.21"
      }
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "src", "View.jsx"),
    [
      "export function View() {",
      "  return <div>import(\"lodash\")</div>;",
      "}"
    ].join("\n")
  );

  await fs.writeFile(
    path.join(tempRoot, "src", "Widget.vue"),
    "<template><div>require(\"lodash\")</div></template>\n"
  );

  await fs.writeFile(
    path.join(tempRoot, "src", "Panel.svelte"),
    "<div>import(\"lodash\")</div>\n"
  );

  const analysis = await analyzeProject(tempRoot);

  assert.equal(analysis.sourceSummary.importedPackages, 0);
  assert.equal(analysis.sourceSummary.dynamicImports, 0);
  assert.equal(analysis.treeShakingWarnings.length, 0);
  assert.ok(analysis.unusedDependencyCandidates.some((item) => item.name === "lodash"));
});

test("analyzeProject detects duplicate packages from Yarn Berry lockfiles", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-yarn-berry-"));

  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "berry-fixture",
      packageManager: "yarn@4.1.0"
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "yarn.lock"),
    [
      "\"lodash@npm:^4.17.20\":",
      "  version: 4.17.20",
      "  resolution: \"lodash@npm:4.17.20\"",
      "",
      "\"lodash@npm:^4.17.21\":",
      "  version: 4.17.21",
      "  resolution: \"lodash@npm:4.17.21\""
    ].join("\n")
  );

  const analysis = await analyzeProject(tempRoot);

  assert.ok(analysis.duplicatePackages.some((item) => item.name === "lodash"));
});

test("analyzeProject prefers the declared package manager lockfile when multiple locks exist", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-lockfile-priority-"));

  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "priority-fixture",
      packageManager: "pnpm@9.0.0"
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "package-lock.json"),
    JSON.stringify({
      lockfileVersion: 3,
      packages: {
        "": {}
      }
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "pnpm-lock.yaml"),
    [
      "lockfileVersion: '9.0'",
      "",
      "packages:",
      "  lodash@4.17.20:",
      "    resolution: {integrity: sha512-foo}",
      "  lodash@4.17.21:",
      "    resolution: {integrity: sha512-bar}"
    ].join("\n")
  );

  const analysis = await analyzeProject(tempRoot);

  assert.ok(analysis.duplicatePackages.some((item) => item.name === "lodash"));
});

test("analyzeProject detects duplicates in legacy npm lockfiles", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-npm-v1-"));

  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "legacy-npm-fixture"
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "package-lock.json"),
    JSON.stringify({
      lockfileVersion: 1,
      dependencies: {
        alpha: {
          version: "1.0.0",
          dependencies: {
            ms: {
              version: "2.0.0"
            }
          }
        },
        beta: {
          version: "1.0.0",
          dependencies: {
            ms: {
              version: "2.1.0"
            }
          }
        }
      }
    }, null, 2)
  );

  const analysis = await analyzeProject(tempRoot);

  assert.ok(analysis.duplicatePackages.some((item) => item.name === "ms"));
});

test("analyzeProject warns when duplicate analysis ignores other lockfiles", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-lockfile-warning-"));

  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "warning-fixture",
      packageManager: "pnpm@9.0.0"
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "package-lock.json"),
    JSON.stringify({
      lockfileVersion: 3,
      packages: {
        "": {},
        "node_modules/lodash": {
          version: "4.17.20"
        },
        "node_modules/pkg-a/node_modules/lodash": {
          version: "4.17.21"
        }
      }
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "pnpm-lock.yaml"),
    [
      "lockfileVersion: '9.0'",
      "",
      "packages:",
      "  react@18.3.0:",
      "    resolution: {integrity: sha512-foo}"
    ].join("\n")
  );

  const analysis = await analyzeProject(tempRoot);

  assert.equal(analysis.duplicatePackages.length, 0);
  assert.ok(analysis.warnings.some((warning) => warning.includes("Multiple lockfiles detected")));
  assert.ok(analysis.warnings.some((warning) => warning.includes("package-lock.json")));
});

test("analyzeProject detects duplicates from Yarn alias descriptors", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "legolas-yarn-alias-"));

  await fs.writeFile(
    path.join(tempRoot, "package.json"),
    JSON.stringify({
      name: "yarn-alias-fixture",
      packageManager: "yarn@4.1.0"
    }, null, 2)
  );

  await fs.writeFile(
    path.join(tempRoot, "yarn.lock"),
    [
      "\"foo@npm:lodash@^4.17.20\":",
      "  version: 4.17.20",
      "  resolution: \"foo@npm:lodash@4.17.20\"",
      "",
      "\"lodash@npm:^4.17.21\":",
      "  version: 4.17.21",
      "  resolution: \"lodash@npm:4.17.21\""
    ].join("\n")
  );

  const analysis = await analyzeProject(tempRoot);

  assert.ok(analysis.duplicatePackages.some((item) => item.name === "lodash"));
});
