use serde::{Deserialize, Serialize};

use crate::boundaries::Phase8SeedContext;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub name: String,
    pub path: String,
    pub imported_packages: usize,
    pub heavy_dependencies: usize,
    pub duplicate_packages: usize,
    pub potential_kb_saved: usize,
}

pub fn collect_workspace_summaries(_context: &Phase8SeedContext<'_>) -> Vec<WorkspaceSummary> {
    Vec::new()
}
