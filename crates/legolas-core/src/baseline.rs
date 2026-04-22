use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::Analysis;

pub const BASELINE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineSnapshot {
    pub schema_version: u32,
    pub project_name: String,
    pub package_manager: String,
    pub dependency_count: usize,
    pub dev_dependency_count: usize,
    pub source_file_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heavy_dependency_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tree_shaking_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl Default for BaselineSnapshot {
    fn default() -> Self {
        Self {
            schema_version: BASELINE_SCHEMA_VERSION,
            project_name: String::new(),
            package_manager: String::new(),
            dependency_count: 0,
            dev_dependency_count: 0,
            source_file_count: 0,
            heavy_dependency_names: Vec::new(),
            tree_shaking_warning_keys: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl BaselineSnapshot {
    pub fn from_analysis(analysis: &Analysis) -> Self {
        Self {
            schema_version: BASELINE_SCHEMA_VERSION,
            project_name: analysis.package_summary.name.clone(),
            package_manager: analysis.package_manager.clone(),
            dependency_count: analysis.package_summary.dependency_count,
            dev_dependency_count: analysis.package_summary.dev_dependency_count,
            source_file_count: analysis.source_summary.files_scanned,
            heavy_dependency_names: unique_sorted(
                analysis
                    .heavy_dependencies
                    .iter()
                    .map(|item| item.name.clone()),
            ),
            tree_shaking_warning_keys: unique_sorted(
                analysis
                    .tree_shaking_warnings
                    .iter()
                    .map(|item| item.key.clone()),
            ),
            warnings: unique_sorted(analysis.warnings.iter().cloned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineDiff {
    pub schema_version: u32,
    pub project_name_previous: String,
    pub project_name_current: String,
    pub package_manager_previous: String,
    pub package_manager_current: String,
    pub dependency_count_previous: usize,
    pub dependency_count_current: usize,
    pub dev_dependency_count_previous: usize,
    pub dev_dependency_count_current: usize,
    pub source_file_count_previous: usize,
    pub source_file_count_current: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_heavy_dependency_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_heavy_dependency_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_tree_shaking_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_tree_shaking_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_warnings: Vec<String>,
}

impl BaselineDiff {
    pub fn is_empty(&self) -> bool {
        self.project_name_previous == self.project_name_current
            && self.package_manager_previous == self.package_manager_current
            && self.dependency_count_previous == self.dependency_count_current
            && self.dev_dependency_count_previous == self.dev_dependency_count_current
            && self.source_file_count_previous == self.source_file_count_current
            && self.added_heavy_dependency_names.is_empty()
            && self.removed_heavy_dependency_names.is_empty()
            && self.added_tree_shaking_warning_keys.is_empty()
            && self.removed_tree_shaking_warning_keys.is_empty()
            && self.added_warnings.is_empty()
            && self.removed_warnings.is_empty()
    }
}

pub fn diff_baselines(previous: &BaselineSnapshot, current: &BaselineSnapshot) -> BaselineDiff {
    BaselineDiff {
        schema_version: BASELINE_SCHEMA_VERSION,
        project_name_previous: previous.project_name.clone(),
        project_name_current: current.project_name.clone(),
        package_manager_previous: previous.package_manager.clone(),
        package_manager_current: current.package_manager.clone(),
        dependency_count_previous: previous.dependency_count,
        dependency_count_current: current.dependency_count,
        dev_dependency_count_previous: previous.dev_dependency_count,
        dev_dependency_count_current: current.dev_dependency_count,
        source_file_count_previous: previous.source_file_count,
        source_file_count_current: current.source_file_count,
        added_heavy_dependency_names: diff_added(
            previous.heavy_dependency_names.as_slice(),
            current.heavy_dependency_names.as_slice(),
        ),
        removed_heavy_dependency_names: diff_removed(
            previous.heavy_dependency_names.as_slice(),
            current.heavy_dependency_names.as_slice(),
        ),
        added_tree_shaking_warning_keys: diff_added(
            previous.tree_shaking_warning_keys.as_slice(),
            current.tree_shaking_warning_keys.as_slice(),
        ),
        removed_tree_shaking_warning_keys: diff_removed(
            previous.tree_shaking_warning_keys.as_slice(),
            current.tree_shaking_warning_keys.as_slice(),
        ),
        added_warnings: diff_added(previous.warnings.as_slice(), current.warnings.as_slice()),
        removed_warnings: diff_removed(previous.warnings.as_slice(), current.warnings.as_slice()),
    }
}

pub fn diff_analysis(previous: &BaselineSnapshot, current: &Analysis) -> BaselineDiff {
    diff_baselines(previous, &BaselineSnapshot::from_analysis(current))
}

fn unique_sorted<I>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn diff_added(previous: &[String], current: &[String]) -> Vec<String> {
    let previous = previous.iter().cloned().collect::<BTreeSet<_>>();
    current
        .iter()
        .filter(|item| !previous.contains(item.as_str()))
        .cloned()
        .collect()
}

fn diff_removed(previous: &[String], current: &[String]) -> Vec<String> {
    let current = current.iter().cloned().collect::<BTreeSet<_>>();
    previous
        .iter()
        .filter(|item| !current.contains(item.as_str()))
        .cloned()
        .collect()
}
