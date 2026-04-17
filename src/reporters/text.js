export function formatScanReport(analysis) {
  const lines = [];

  lines.push(`Legolas scan for ${analysis.packageSummary.name}`);
  lines.push(`Project root: ${analysis.projectRoot}`);
  lines.push(`Mode: ${analysis.metadata.mode}`);
  lines.push(`Frameworks: ${analysis.frameworks.length > 0 ? analysis.frameworks.join(", ") : "none detected"}`);
  lines.push(`Package manager: ${analysis.packageManager}`);
  lines.push(`Scanned ${analysis.sourceSummary.filesScanned} source files and ${analysis.sourceSummary.importedPackages} imported packages`);
  lines.push("");
  lines.push(`Potential payload reduction: ~${analysis.impact.potentialKbSaved} KB`);
  lines.push(`Estimated LCP improvement: ~${analysis.impact.estimatedLcpImprovementMs} ms`);
  lines.push(analysis.impact.summary);
  appendWarnings(lines, analysis.warnings);
  lines.push("");

  lines.push("Heaviest known dependencies:");
  appendSection(lines, analysis.heavyDependencies, (item) => {
    const importText = item.importedBy.length > 0 ? `imported in ${item.importedBy.length} file(s)` : "declared but not detected in source";
    return `- ${item.name} (${item.estimatedKb} KB): ${item.rationale} ${importText}.`;
  }, "- none");

  lines.push("");
  lines.push("Duplicate package versions:");
  appendSection(lines, analysis.duplicatePackages, (item) => `- ${item.name}: ${item.versions.join(", ")} (${item.estimatedExtraKb} KB avoidable)` , "- none");

  lines.push("");
  lines.push("Lazy-load candidates:");
  appendSection(lines, analysis.lazyLoadCandidates, (item) => `- ${item.name}: ${item.reason}. Estimated win ${item.estimatedSavingsKb} KB.`, "- none");

  lines.push("");
  lines.push("Tree-shaking warnings:");
  appendSection(lines, analysis.treeShakingWarnings, (item) => `- ${item.packageName}: ${item.message}`, "- none");

  lines.push("");
  lines.push("Unused dependency candidates:");
  appendSection(lines, analysis.unusedDependencyCandidates.slice(0, 10), (item) => `- ${item.name}@${item.versionRange}`, "- none");

  if (analysis.bundleArtifacts.length > 0) {
    lines.push("");
    lines.push(`Detected bundle artifacts: ${analysis.bundleArtifacts.join(", ")}`);
  }

  return lines.join("\n");
}

export function formatVisualizationReport(analysis, limit = 10) {
  const lines = [];
  const heavyDependencies = analysis.heavyDependencies.slice(0, Math.max(limit, 1));
  const duplicates = analysis.duplicatePackages.slice(0, Math.max(limit, 1));

  lines.push(`Legolas visualize for ${analysis.packageSummary.name}`);
  appendWarnings(lines, analysis.warnings);
  lines.push("");
  lines.push("Estimated dependency weight");
  lines.push(renderBars(
    heavyDependencies.length > 0
      ? heavyDependencies.map((item) => ({
          label: item.name,
          value: item.estimatedKb
        }))
      : [{ label: "none", value: 0 }]
  ));
  lines.push("");
  lines.push("Duplicate package pressure");
  lines.push(renderBars(
    duplicates.length > 0
      ? duplicates.map((item) => ({
          label: item.name,
          value: item.estimatedExtraKb
        }))
      : [{ label: "none", value: 0 }]
  ));

  return lines.join("\n");
}

export function formatOptimizeReport(analysis, top = 5) {
  const lines = [];
  const actions = buildActions(analysis).slice(0, Math.max(top, 1));

  lines.push(`Legolas optimize for ${analysis.packageSummary.name}`);
  appendWarnings(lines, analysis.warnings);
  lines.push("");
  appendSection(lines, actions, (item, index) => `${index + 1}. ${item}`, "1. No high-confidence optimization candidates were found.");
  lines.push("");
  lines.push(`Projected savings: ~${analysis.impact.potentialKbSaved} KB, with ${analysis.impact.confidence} confidence.`);

  return lines.join("\n");
}

function buildActions(analysis) {
  const actions = [];

  for (const dependency of analysis.heavyDependencies.slice(0, 3)) {
    if (dependency.importedBy.length === 0) {
      actions.push(`Remove or justify ${dependency.name}; it is declared but not imported in scanned source files.`);
      continue;
    }

    actions.push(`Review ${dependency.name}: ${dependency.recommendation}`);
  }

  for (const duplicate of analysis.duplicatePackages.slice(0, 3)) {
    actions.push(`Deduplicate ${duplicate.name} versions (${duplicate.versions.join(", ")}) to recover roughly ${duplicate.estimatedExtraKb} KB.`);
  }

  for (const candidate of analysis.lazyLoadCandidates.slice(0, 3)) {
    actions.push(`Lazy load ${candidate.name} in ${candidate.files[0]} to target roughly ${candidate.estimatedSavingsKb} KB of deferred code.`);
  }

  for (const warning of analysis.treeShakingWarnings.slice(0, 2)) {
    actions.push(`Clean up ${warning.packageName} imports: ${warning.recommendation}`);
  }

  return dedupe(actions);
}

function renderBars(items) {
  const maxValue = Math.max(...items.map((item) => item.value), 1);
  return items
    .map((item) => {
      const barLength = item.value === 0 ? 0 : Math.max(1, Math.round((item.value / maxValue) * 24));
      const bar = "█".repeat(barLength);
      return `${item.label.padEnd(24)} ${bar.padEnd(24)} ${item.value} KB`;
    })
    .join("\n");
}

function appendSection(lines, items, renderItem, fallbackLine) {
  if (!items || items.length === 0) {
    lines.push(fallbackLine);
    return;
  }

  items.forEach((item, index) => {
    lines.push(renderItem(item, index));
  });
}

function dedupe(items) {
  return [...new Set(items)];
}

function appendWarnings(lines, warnings) {
  if (!warnings || warnings.length === 0) {
    return;
  }

  lines.push("");
  lines.push("Warnings:");
  warnings.forEach((warning) => {
    lines.push(`- ${warning}`);
  });
}
