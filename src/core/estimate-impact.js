export function estimateImpact({ heavyDependencies, duplicatePackages, lazyLoadCandidates, treeShakingWarnings }) {
  const heavyKb = heavyDependencies.slice(0, 5).reduce((sum, item) => sum + item.estimatedKb * 0.18, 0);
  const duplicateKb = duplicatePackages.reduce((sum, item) => sum + item.estimatedExtraKb, 0);
  const lazyKb = lazyLoadCandidates.reduce((sum, item) => sum + item.estimatedSavingsKb, 0);
  const shakingKb = treeShakingWarnings.reduce((sum, item) => sum + item.estimatedKb, 0);

  const potentialKbSaved = Math.round(heavyKb + duplicateKb + lazyKb + shakingKb);
  const estimatedLcpImprovementMs = Math.round(potentialKbSaved * 2.1);

  return {
    potentialKbSaved,
    estimatedLcpImprovementMs,
    confidence: potentialKbSaved > 0 ? "directional" : "low",
    summary: summarizeImpact(potentialKbSaved)
  };
}

function summarizeImpact(potentialKbSaved) {
  if (potentialKbSaved >= 300) {
    return "High impact: the project has clear opportunities to reduce initial payload size.";
  }

  if (potentialKbSaved >= 120) {
    return "Medium impact: there are several meaningful bundle wins available.";
  }

  if (potentialKbSaved >= 40) {
    return "Targeted impact: a handful of focused optimizations should pay off.";
  }

  return "Low impact: obvious bundle issues are limited in the current scan.";
}
