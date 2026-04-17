import path from "node:path";

import { exists, readJsonIfExists, readTextIfExists } from "./workspace.js";

export async function parseDuplicatePackages(projectRoot, packageManager = "unknown") {
  const packageLockPath = path.join(projectRoot, "package-lock.json");
  const pnpmLockPath = path.join(projectRoot, "pnpm-lock.yaml");
  const yarnLockPath = path.join(projectRoot, "yarn.lock");
  const normalizedPackageManager = normalizePackageManager(packageManager);
  const lockfiles = [
    {
      name: "npm",
      filePath: packageLockPath,
      read: async () => {
        const packageLock = await readJsonIfExists(packageLockPath);
        return summarizeDuplicates(collectFromPackageLock(packageLock));
      }
    },
    {
      name: "pnpm",
      filePath: pnpmLockPath,
      read: async () => {
        const content = await readTextIfExists(pnpmLockPath);
        return summarizeDuplicates(collectFromPnpmLock(content ?? ""));
      }
    },
    {
      name: "yarn",
      filePath: yarnLockPath,
      read: async () => {
        const content = await readTextIfExists(yarnLockPath);
        return summarizeDuplicates(collectFromYarnLock(content ?? ""));
      }
    }
  ];
  const orderedLockfiles = prioritizeLockfiles(lockfiles, normalizedPackageManager);
  const existingLockfiles = [];

  for (const lockfile of orderedLockfiles) {
    if (await exists(lockfile.filePath)) {
      existingLockfiles.push(lockfile);
    }
  }

  if (existingLockfiles.length === 0) {
    return {
      duplicates: [],
      warnings: []
    };
  }

  const [selectedLockfile, ...ignoredLockfiles] = existingLockfiles;

  return {
    duplicates: await selectedLockfile.read(),
    warnings: buildLockfileWarnings(selectedLockfile, ignoredLockfiles, packageManager)
  };
}

function collectFromPackageLock(packageLock) {
  const versionsByName = new Map();

  if (!packageLock) {
    return versionsByName;
  }

  const packages = packageLock.packages ?? {};
  if (Object.keys(packages).length > 0) {
    for (const [packagePath, metadata] of Object.entries(packages)) {
      if (!packagePath.includes("node_modules/") || !metadata?.version) {
        continue;
      }

      const startIndex = packagePath.lastIndexOf("node_modules/") + "node_modules/".length;
      const packageName = packagePath.slice(startIndex);
      addVersion(versionsByName, packageName, metadata.version);
    }

    return versionsByName;
  }

  collectFromPackageLockDependencies(packageLock.dependencies ?? {}, versionsByName);
  return versionsByName;
}

function collectFromPnpmLock(content) {
  const versionsByName = new Map();
  const lines = content.split(/\r?\n/);
  let insidePackages = false;

  for (const line of lines) {
    if (line === "packages:" || line === "snapshots:") {
      insidePackages = true;
      continue;
    }

    if (insidePackages && /^[A-Za-z]/.test(line)) {
      insidePackages = false;
    }

    if (!insidePackages) {
      continue;
    }

    const match = line.match(/^ {2,}'?(@?[^:'\s][^:]*?)'?:\s*$/);
    if (!match) {
      continue;
    }

    let descriptor = match[1];
    if (descriptor.startsWith("/")) {
      descriptor = descriptor.slice(1);
    }

    const parsed = splitDescriptor(descriptor);
    if (!parsed) {
      continue;
    }

    addVersion(versionsByName, parsed.name, parsed.version);
  }

  return versionsByName;
}

function collectFromYarnLock(content) {
  const versionsByName = new Map();
  const lines = content.split(/\r?\n/);
  let currentPackageNames = [];
  let currentVersion = null;

  for (const line of lines) {
    if (!line.trim()) {
      flushCurrentYarnEntry(versionsByName, currentPackageNames, currentVersion);
      currentPackageNames = [];
      currentVersion = null;
      continue;
    }

    if (!line.startsWith(" ")) {
      flushCurrentYarnEntry(versionsByName, currentPackageNames, currentVersion);
      currentPackageNames = line
        .replace(/:$/, "")
        .split(/,\s*/)
        .map((token) => token.replace(/^"|"$/g, ""))
        .map(extractYarnPackageName)
        .filter(Boolean);
      currentVersion = null;
      continue;
    }

    const versionMatch = line.match(/^ {2}version "(.*)"$/) ?? line.match(/^ {2}version:\s+"?([^"]+)"?$/);
    if (!versionMatch) {
      continue;
    }

    currentVersion = versionMatch[1];
  }

  flushCurrentYarnEntry(versionsByName, currentPackageNames, currentVersion);
  return versionsByName;
}

function splitDescriptor(descriptor) {
  const clean = descriptor.replace(/^npm:/, "");
  const match = clean.match(/^(@[^/]+\/[^@]+|[^@]+)@(.+)$/);
  if (!match) {
    return null;
  }

  return {
    name: match[1],
    version: match[2].split("(")[0]
  };
}

function extractYarnPackageName(descriptor) {
  const aliasMatch = descriptor.match(/@npm:(@[^/]+\/[^@]+|[^@]+)@/);
  if (aliasMatch) {
    return aliasMatch[1];
  }

  const parsedDescriptor = splitDescriptor(descriptor);
  return parsedDescriptor ? parsedDescriptor.name : null;
}

function summarizeDuplicates(versionsByName) {
  const results = [];

  for (const [name, versions] of versionsByName.entries()) {
    if (versions.size < 2) {
      continue;
    }

    const allVersions = [...versions].sort(compareVersions);
    const estimatedExtraKb = Math.max((allVersions.length - 1) * 18, 18);

    results.push({
      name,
      versions: allVersions,
      count: allVersions.length,
      estimatedExtraKb
    });
  }

  return results.sort((left, right) => {
    if (right.count !== left.count) {
      return right.count - left.count;
    }
    return left.name.localeCompare(right.name);
  });
}

function addVersion(versionsByName, name, version) {
  if (!versionsByName.has(name)) {
    versionsByName.set(name, new Set());
  }
  versionsByName.get(name).add(version);
}

function compareVersions(left, right) {
  return left.localeCompare(right, undefined, { numeric: true, sensitivity: "base" });
}

function collectFromPackageLockDependencies(dependencies, versionsByName) {
  for (const [name, metadata] of Object.entries(dependencies)) {
    if (metadata?.version) {
      addVersion(versionsByName, name, metadata.version);
    }

    if (metadata?.dependencies) {
      collectFromPackageLockDependencies(metadata.dependencies, versionsByName);
    }
  }
}

function normalizePackageManager(packageManager) {
  const normalized = String(packageManager).toLowerCase();

  if (normalized.startsWith("pnpm")) {
    return "pnpm";
  }

  if (normalized.startsWith("yarn")) {
    return "yarn";
  }

  if (normalized.startsWith("npm")) {
    return "npm";
  }

  return null;
}

function prioritizeLockfiles(lockfiles, preferredLockfile) {
  if (!preferredLockfile) {
    return lockfiles;
  }

  return [
    ...lockfiles.filter((lockfile) => lockfile.name === preferredLockfile),
    ...lockfiles.filter((lockfile) => lockfile.name !== preferredLockfile)
  ];
}

function flushCurrentYarnEntry(versionsByName, packageNames, version) {
  if (!version || packageNames.length === 0) {
    return;
  }

  for (const packageName of packageNames) {
    addVersion(versionsByName, packageName, version);
  }
}

function buildLockfileWarnings(selectedLockfile, ignoredLockfiles, packageManager) {
  if (ignoredLockfiles.length === 0) {
    return [];
  }

  const selectedName = path.basename(selectedLockfile.filePath);
  const ignoredNames = ignoredLockfiles.map((lockfile) => path.basename(lockfile.filePath));
  const packageManagerText = packageManager && packageManager !== "unknown"
    ? ` based on package manager "${packageManager}"`
    : "";

  return [
    `Multiple lockfiles detected. Duplicate analysis used ${selectedName}${packageManagerText} and ignored ${ignoredNames.join(", ")}.`
  ];
}
