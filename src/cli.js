import { analyzeProject } from "./core/analyze-project.js";
import { formatOptimizeReport, formatScanReport, formatVisualizationReport } from "./reporters/text.js";

const HELP_TEXT = `Legolas
Slim bundles with precision.

Usage:
  legolas scan [path] [--json]
  legolas visualize [path] [--limit 10]
  legolas optimize [path] [--top 5]
  legolas help

Examples:
  legolas scan .
  legolas visualize ./apps/storefront --limit 12
  legolas optimize --top 7
`;

export async function runCli(argv) {
  const { command, targetPath, flags } = parseArgv(argv);

  if (flags.version) {
    console.log("0.1.0");
    return;
  }

  if (!command || command === "help" || flags.help) {
    console.log(HELP_TEXT);
    return;
  }

  if (!["scan", "visualize", "optimize"].includes(command)) {
    throw new Error(`unknown command "${command}"`);
  }

  const analysis = await analyzeProject(targetPath);

  if (flags.json) {
    console.log(JSON.stringify(analysis, null, 2));
    return;
  }

  if (command === "scan") {
    console.log(formatScanReport(analysis));
    return;
  }

  if (command === "visualize") {
    console.log(formatVisualizationReport(analysis, Number(flags.limit ?? 10)));
    return;
  }

  console.log(formatOptimizeReport(analysis, Number(flags.top ?? 5)));
}

function parseArgv(argv) {
  let command = null;
  let targetPath = process.cwd();
  const flags = {};

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];

    if (!command && !token.startsWith("-")) {
      command = token;
      continue;
    }

    if (!token.startsWith("-")) {
      targetPath = token;
      continue;
    }

    if (token === "--help" || token === "-h") {
      flags.help = true;
      continue;
    }

    if (token === "--version" || token === "-v") {
      flags.version = true;
      continue;
    }

    if (token === "--json") {
      flags.json = true;
      continue;
    }

    if (token === "--limit" || token === "--top") {
      const next = argv[index + 1];
      if (!next || next.startsWith("-")) {
        throw new Error(`${token} expects a number`);
      }
      const parsedValue = Number(next);
      if (!Number.isInteger(parsedValue) || parsedValue < 1) {
        throw new Error(`${token} expects a positive integer`);
      }
      flags[token.replace(/^--/, "")] = parsedValue;
      index += 1;
      continue;
    }

    throw new Error(`unknown flag "${token}"`);
  }

  return { command, targetPath, flags };
}
