use crate::findings::FindingMetadata;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub project_root: String,
    pub package_manager: String,
    pub frameworks: Vec<String>,
    pub bundle_artifacts: Vec<String>,
    pub package_summary: PackageSummary,
    pub source_summary: SourceSummary,
    pub heavy_dependencies: Vec<HeavyDependency>,
    pub duplicate_packages: Vec<DuplicatePackage>,
    pub lazy_load_candidates: Vec<LazyLoadCandidate>,
    pub tree_shaking_warnings: Vec<TreeShakingWarning>,
    pub unused_dependency_candidates: Vec<UnusedDependencyCandidate>,
    pub warnings: Vec<String>,
    pub impact: Impact,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PackageSummary {
    pub name: String,
    pub dependency_count: usize,
    pub dev_dependency_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceSummary {
    pub files_scanned: usize,
    pub imported_packages: usize,
    pub dynamic_imports: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HeavyDependency {
    pub name: String,
    pub version_range: String,
    pub estimated_kb: usize,
    pub category: String,
    pub rationale: String,
    pub recommendation: String,
    pub imported_by: Vec<String>,
    pub dynamic_imported_by: Vec<String>,
    pub import_count: usize,
    #[serde(flatten, default, skip_serializing_if = "FindingMetadata::is_empty")]
    pub finding: FindingMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatePackage {
    pub name: String,
    pub versions: Vec<String>,
    pub count: usize,
    pub estimated_extra_kb: usize,
    #[serde(flatten, default, skip_serializing_if = "FindingMetadata::is_empty")]
    pub finding: FindingMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LazyLoadCandidate {
    pub name: String,
    pub estimated_savings_kb: usize,
    pub recommendation: String,
    pub files: Vec<String>,
    pub reason: String,
    #[serde(flatten, default, skip_serializing_if = "FindingMetadata::is_empty")]
    pub finding: FindingMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TreeShakingWarning {
    pub key: String,
    pub package_name: String,
    pub message: String,
    pub recommendation: String,
    pub estimated_kb: usize,
    pub files: Vec<String>,
    #[serde(flatten, default, skip_serializing_if = "FindingMetadata::is_empty")]
    pub finding: FindingMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UnusedDependencyCandidate {
    pub name: String,
    pub version_range: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Impact {
    pub potential_kb_saved: usize,
    pub estimated_lcp_improvement_ms: usize,
    pub confidence: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub mode: String,
    pub generated_at: String,
}
