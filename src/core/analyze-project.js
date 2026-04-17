import { promises as fs } from "node:fs";
import path from "node:path";

import { detectFrameworks, detectPackageManager } from "./detect-project-shape.js";
import { estimateImpact } from "./estimate-impact.js";
import { collectSourceFiles, scanImports } from "./scan-imports.js";
import { findProjectRoot, readJsonIfExists, readTextIfExists } from "./workspace.js";
import { getPackageIntel } from "./package-intelligence.js";
import { parseDuplicatePackages } from "./parse-lockfiles.js";

export async function analyzeProject(inputPath = process.cwd()) {
  const projectRoot = await findProjectRoot(inputPath);
  const manifest = await readJsonIfExists(path.join(projectRoot, "package.json"));

  if (!manifest) {
    throw new Error(`package.json not found near ${projectRoot}`);
  }

  const packageManager = await detectPackageManager(projectRoot, manifest);
  const frameworks = await detectFrameworks(projectRoot, manifest);
  const sourceFiles = await collectSourceFiles(projectRoot);
  const sourceAnalysis = await scanImports(projectRoot, sourceFiles);
  const duplicateAnalysis = await parseDuplicatePackages(projectRoot, packageManager);
  const duplicatePackages = duplicateAnalysis.duplicates;

  const heavyDependencies = buildHeavyDependencyReport(manifest, sourceAnalysis);
  const lazyLoadCandidates = buildLazyLoadCandidates(sourceAnalysis, heavyDependencies);
  const treeShakingWarnings = buildTreeShakingWarnings(sourceAnalysis);
  const bundleArtifacts = await detectBundleArtifacts(projectRoot);
  const impact = estimateImpact({
    heavyDependencies,
    duplicatePackages,
    lazyLoadCandidates,
    treeShakingWarnings
  });

  return {
    projectRoot,
    packageManager,
    frameworks,
    bundleArtifacts,
    packageSummary: buildPackageSummary(manifest),
    sourceSummary: {
      filesScanned: sourceFiles.length,
      importedPackages: sourceAnalysis.importedPackages.length,
      dynamicImports: sourceAnalysis.dynamicImportCount
    },
    heavyDependencies,
    duplicatePackages,
    lazyLoadCandidates,
    treeShakingWarnings,
    unusedDependencyCandidates: buildUnusedDependencyCandidates(manifest, sourceAnalysis),
    warnings: duplicateAnalysis.warnings,
    impact,
    metadata: {
      mode: bundleArtifacts.length > 0 ? "artifact-assisted" : "heuristic",
      generatedAt: new Date().toISOString()
    }
  };
}

function buildPackageSummary(manifest) {
  const dependencies = Object.keys(manifest.dependencies ?? {});
  const devDependencies = Object.keys(manifest.devDependencies ?? {});

  return {
    name: manifest.name ?? "unknown-project",
    dependencyCount: dependencies.length,
    devDependencyCount: devDependencies.length
  };
}

function buildHeavyDependencyReport(manifest, sourceAnalysis) {
  const dependencyEntries = {
    ...(manifest.dependencies ?? {}),
    ...(manifest.optionalDependencies ?? {})
  };

  return Object.entries(dependencyEntries)
    .map(([name, range]) => {
      const intel = getPackageIntel(name);
      if (!intel) {
        return null;
      }

      const importInfo = sourceAnalysis.byPackage.get(name);
      return {
        name,
        versionRange: range,
        estimatedKb: intel.estimatedKb,
        category: intel.category,
        rationale: intel.rationale,
        recommendation: intel.recommendation,
        importedBy: importInfo ? sortSet(importInfo.files) : [],
        dynamicImportedBy: importInfo ? sortSet(importInfo.dynamicFiles) : [],
        importCount: importInfo ? importInfo.files.size : 0
      };
    })
    .filter(Boolean)
    .sort((left, right) => right.estimatedKb - left.estimatedKb);
}

function buildLazyLoadCandidates(sourceAnalysis, heavyDependencies) {
  const heavyByName = new Map(heavyDependencies.map((item) => [item.name, item]));
  const candidateFilesPattern = /(modal|chart|editor|map|viewer|dashboard|settings|admin|page|route|dialog|drawer|popover)/i;
  const candidates = [];

  for (const importedPackage of sourceAnalysis.importedPackages) {
    const heavy = heavyByName.get(importedPackage.name);
    if (!heavy) {
      continue;
    }

    const staticFiles = sortSet(importedPackage.staticFiles);
    const dynamicFiles = sortSet(importedPackage.dynamicFiles);
    const splitFriendlyFiles = staticFiles.filter((file) => candidateFilesPattern.test(file));

    if (splitFriendlyFiles.length === 0 || dynamicFiles.length > 0) {
      continue;
    }

    candidates.push({
      name: importedPackage.name,
      estimatedSavingsKb: Math.round(heavy.estimatedKb * 0.75),
      recommendation: heavy.recommendation,
      files: splitFriendlyFiles,
      reason: `${importedPackage.name} is statically imported in UI surfaces that usually tolerate lazy loading`
    });
  }

  return candidates.sort((left, right) => right.estimatedSavingsKb - left.estimatedSavingsKb);
}

function buildTreeShakingWarnings(sourceAnalysis) {
  return sourceAnalysis.treeShakingWarnings
    .map((warning) => ({
      ...warning,
      files: sortSet(warning.files)
    }))
    .sort((left, right) => right.estimatedKb - left.estimatedKb);
}

function buildUnusedDependencyCandidates(manifest, sourceAnalysis) {
  const dependencies = Object.entries(manifest.dependencies ?? {});
  const usedPackages = new Set(sourceAnalysis.importedPackages.map((item) => item.name));

  return dependencies
    .filter(([name]) => !usedPackages.has(name))
    .map(([name, versionRange]) => ({
      name,
      versionRange
    }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

async function detectBundleArtifacts(projectRoot) {
  const knownArtifacts = [
    "stats.json",
    "dist/stats.json",
    "build/stats.json",
    "meta.json",
    "dist/meta.json"
  ];

  const detected = [];

  for (const relativePath of knownArtifacts) {
    const absolutePath = path.join(projectRoot, relativePath);
    try {
      const stats = await fs.stat(absolutePath);
      if (stats.isFile()) {
        detected.push(relativePath);
      }
    } catch {}
  }

  return detected;
}

function sortSet(values) {
  return [...values].sort((left, right) => left.localeCompare(right));
}
