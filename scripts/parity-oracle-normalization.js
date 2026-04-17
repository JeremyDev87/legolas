export function normalizeCliOutput(output, parityFixtureRoot, projectRootPlaceholder = "<PROJECT_ROOT>") {
  return toPosix(output).split(toPosix(parityFixtureRoot)).join(projectRootPlaceholder);
}

export function normalizeAnalysisForOracle(
  analysis,
  {
    projectRootPlaceholder = "<PROJECT_ROOT>",
    generatedAtPlaceholder = "<GENERATED_AT>"
  } = {}
) {
  return {
    ...analysis,
    projectRoot: projectRootPlaceholder,
    bundleArtifacts: analysis.bundleArtifacts.map(toPosix),
    heavyDependencies: analysis.heavyDependencies.map((item) => ({
      ...item,
      importedBy: item.importedBy.map(toPosix),
      dynamicImportedBy: item.dynamicImportedBy.map(toPosix)
    })),
    lazyLoadCandidates: analysis.lazyLoadCandidates.map((item) => ({
      ...item,
      files: item.files.map(toPosix)
    })),
    treeShakingWarnings: analysis.treeShakingWarnings.map((item) => ({
      ...item,
      files: item.files.map(toPosix)
    })),
    warnings: analysis.warnings.map(toPosix),
    metadata: {
      ...analysis.metadata,
      generatedAt: generatedAtPlaceholder
    }
  };
}

export function toPosix(value) {
  return String(value).replaceAll("\\", "/");
}
