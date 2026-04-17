import { promises as fs } from "node:fs";
import path from "node:path";

const ROOT_MARKERS = ["package.json", "pnpm-lock.yaml", "package-lock.json", "yarn.lock", "bun.lock", "bun.lockb"];

export async function findProjectRoot(inputPath = process.cwd()) {
  const resolved = path.resolve(inputPath);
  let current = await normalizeToDirectory(resolved);
  const initialDirectory = current;

  while (true) {
    for (const marker of ROOT_MARKERS) {
      if (await exists(path.join(current, marker))) {
        return current;
      }
    }

    const parent = path.dirname(current);
    if (parent === current) {
      return initialDirectory;
    }
    current = parent;
  }
}

export async function readJsonIfExists(filePath) {
  const contents = await readTextIfExists(filePath);
  return contents ? JSON.parse(contents) : null;
}

export async function readTextIfExists(filePath) {
  try {
    return await fs.readFile(filePath, "utf8");
  } catch (error) {
    if (error && typeof error === "object" && "code" in error && error.code === "ENOENT") {
      return null;
    }
    throw error;
  }
}

export async function exists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function normalizeToDirectory(targetPath) {
  try {
    const stats = await fs.stat(targetPath);
    return stats.isDirectory() ? targetPath : path.dirname(targetPath);
  } catch (error) {
    if (error && typeof error === "object" && "code" in error && error.code === "ENOENT") {
      throw new Error(`path not found: ${targetPath}`);
    }
    throw error;
  }
}
