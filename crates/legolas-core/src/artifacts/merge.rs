use std::collections::{BTreeMap, BTreeSet};

use crate::{
    artifacts::ArtifactSummary, findings::FindingEvidence, import_scanner::SourceAnalysis,
    models::HeavyDependency,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactSignalKind {
    Source,
    Artifact,
    ArtifactSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactChunkSignal {
    pub name: String,
    pub entrypoints: Vec<String>,
    pub files: Vec<String>,
    pub bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactSourceSignal {
    pub package_name: String,
    pub kind: ArtifactSignalKind,
    pub source_files: Vec<String>,
    pub chunks: Vec<ArtifactChunkSignal>,
    pub artifact_bytes: usize,
}

impl ArtifactSourceSignal {
    pub fn evidence(&self) -> Vec<FindingEvidence> {
        let mut evidence = self
            .source_files
            .iter()
            .map(|file| {
                FindingEvidence::new("source-file")
                    .with_file(file.clone())
                    .with_specifier(self.package_name.clone())
                    .with_detail("package imported in source analysis")
            })
            .collect::<Vec<_>>();

        evidence.extend(self.chunks.iter().map(|chunk| {
            let mut item = FindingEvidence::new("artifact-chunk")
                .with_specifier(self.package_name.clone())
                .with_detail(chunk_detail(chunk));
            if let Some(file) = preferred_chunk_file(&chunk.files) {
                item = item.with_file(file.clone());
            }
            item
        }));

        evidence
    }
}

pub fn merge_artifact_source_signals(
    artifact_summary: &ArtifactSummary,
    source_analysis: &SourceAnalysis,
    heavy_dependencies: &[HeavyDependency],
) -> Vec<ArtifactSourceSignal> {
    let heavy_packages = heavy_dependencies
        .iter()
        .map(|item| item.name.as_str())
        .collect::<BTreeSet<_>>();
    let chunk_lookup = artifact_summary
        .chunks
        .iter()
        .map(|chunk| (chunk.name.as_str(), chunk))
        .collect::<BTreeMap<_, _>>();
    let mut packages = source_analysis
        .by_package
        .iter()
        .filter(|(package_name, _)| heavy_packages.contains(package_name.as_str()))
        .map(|(package_name, record)| {
            (
                package_name.clone(),
                PackageAccumulator::new(package_name.clone(), record.files.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for module in &artifact_summary.modules {
        let Some(package_name) = module.package_name.as_deref() else {
            continue;
        };
        if !heavy_packages.contains(package_name) {
            continue;
        }

        let source_files = source_analysis
            .by_package
            .get(package_name)
            .map(|record| record.files.clone())
            .unwrap_or_default();
        let package = packages
            .entry(package_name.to_string())
            .or_insert_with(|| PackageAccumulator::new(package_name.to_string(), source_files));

        package.artifact_bytes += module.bytes;

        for chunk_name in &module.chunks {
            let chunk = chunk_lookup.get(chunk_name.as_str());
            let entry = package
                .chunks
                .entry(chunk_name.clone())
                .or_insert_with(|| ChunkAccumulator::from_summary(chunk_name, chunk.copied()));
            entry.bytes += module.bytes;
        }
    }

    packages
        .into_values()
        .map(PackageAccumulator::into_signal)
        .collect()
}

#[derive(Debug, Clone)]
struct PackageAccumulator {
    package_name: String,
    source_files: Vec<String>,
    chunks: BTreeMap<String, ChunkAccumulator>,
    artifact_bytes: usize,
}

impl PackageAccumulator {
    fn new(package_name: String, mut source_files: Vec<String>) -> Self {
        sort_and_dedup(&mut source_files);
        Self {
            package_name,
            source_files,
            chunks: BTreeMap::new(),
            artifact_bytes: 0,
        }
    }

    fn into_signal(self) -> ArtifactSourceSignal {
        let kind = if self.chunks.is_empty() {
            ArtifactSignalKind::Source
        } else if self.source_files.is_empty() {
            ArtifactSignalKind::Artifact
        } else {
            ArtifactSignalKind::ArtifactSource
        };

        ArtifactSourceSignal {
            package_name: self.package_name,
            kind,
            source_files: self.source_files,
            chunks: self
                .chunks
                .into_values()
                .map(ChunkAccumulator::into_signal)
                .collect(),
            artifact_bytes: self.artifact_bytes,
        }
    }
}

#[derive(Debug, Clone)]
struct ChunkAccumulator {
    name: String,
    entrypoints: Vec<String>,
    files: Vec<String>,
    bytes: usize,
}

impl ChunkAccumulator {
    fn from_summary(chunk_name: &str, summary: Option<&crate::artifacts::ArtifactChunk>) -> Self {
        let mut entrypoints = summary
            .map(|chunk| chunk.entrypoints.clone())
            .unwrap_or_default();
        let mut files = summary.map(|chunk| chunk.files.clone()).unwrap_or_default();
        sort_and_dedup(&mut entrypoints);
        sort_and_dedup(&mut files);

        Self {
            name: chunk_name.to_string(),
            entrypoints,
            files,
            bytes: 0,
        }
    }

    fn into_signal(self) -> ArtifactChunkSignal {
        ArtifactChunkSignal {
            name: self.name,
            entrypoints: self.entrypoints,
            files: self.files,
            bytes: self.bytes,
        }
    }
}

fn sort_and_dedup(values: &mut Vec<String>) {
    values.sort_unstable();
    values.dedup();
}

fn preferred_chunk_file(files: &[String]) -> Option<&String> {
    files
        .iter()
        .find(|file| is_code_asset(file))
        .or_else(|| files.iter().find(|file| !file.ends_with(".map")))
        .or_else(|| files.first())
}

fn is_code_asset(file: &str) -> bool {
    matches!(file.rsplit('.').next(), Some("js" | "mjs" | "cjs" | "jsx"))
}

fn chunk_detail(chunk: &ArtifactChunkSignal) -> String {
    if chunk.entrypoints.is_empty() {
        format!(
            "artifact chunk `{}` contributes {} bytes",
            chunk.name, chunk.bytes
        )
    } else {
        format!(
            "artifact chunk `{}` contributes {} bytes; entrypoints: {}",
            chunk.name,
            chunk.bytes,
            chunk.entrypoints.join(", ")
        )
    }
}
