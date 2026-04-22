use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{findings::FindingMetadata, import_scanner::SourceAnalysis};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BoundaryWarning {
    pub message: String,
    pub recommendation: String,
    #[serde(flatten, default, skip_serializing_if = "FindingMetadata::is_empty")]
    pub finding: FindingMetadata,
}

#[derive(Debug, Clone, Copy)]
pub struct Phase8SeedContext<'a> {
    pub project_root: &'a Path,
    pub package_manager: &'a str,
    pub frameworks: &'a [String],
    pub bundle_artifacts: &'a [String],
    pub source_analysis: &'a SourceAnalysis,
    pub source_file_count: usize,
    pub imported_package_count: usize,
    pub dynamic_import_count: usize,
}

pub fn collect_boundary_warnings(_context: &Phase8SeedContext<'_>) -> Vec<BoundaryWarning> {
    Vec::new()
}
