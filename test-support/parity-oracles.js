import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  normalizeAnalysisForOracle as normalizeAnalysisForOracleBase,
  normalizeCliOutput as normalizeCliOutputBase
} from "../scripts/parity-oracle-normalization.js";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const oracleRoot = path.join(projectRoot, "tests", "oracles");
const parityFixtureRoot = path.join(projectRoot, "tests", "fixtures", "parity", "basic-app");
const cliPath = path.join(projectRoot, "bin", "legolas.js");

export { cliPath, parityFixtureRoot, projectRoot };

export async function readOracle(...relativeParts) {
  return readFile(path.join(oracleRoot, ...relativeParts), "utf8");
}

export function normalizeCliOutput(output) {
  return normalizeCliOutputBase(output, parityFixtureRoot);
}

export function normalizeAnalysisForOracle(analysis) {
  return normalizeAnalysisForOracleBase(analysis);
}
