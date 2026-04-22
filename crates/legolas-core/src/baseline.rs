use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    models::{DuplicatePackage, LazyLoadCandidate},
    Analysis,
};

pub const BASELINE_SCHEMA_VERSION: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineFindingMetric {
    pub key: String,
    pub primary_metric: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_metric: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineSnapshot {
    pub schema_version: u32,
    pub project_name: String,
    pub package_manager: String,
    pub dependency_count: usize,
    pub dev_dependency_count: usize,
    pub source_file_count: usize,
    #[serde(default)]
    pub dynamic_import_count: usize,
    #[serde(default)]
    pub potential_kb_saved: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heavy_dependency_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tree_shaking_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duplicate_package_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lazy_load_candidate_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundary_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unused_dependency_candidate_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heavy_dependency_metrics: Vec<BaselineFindingMetric>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tree_shaking_warning_metrics: Vec<BaselineFindingMetric>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duplicate_package_metrics: Vec<BaselineFindingMetric>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lazy_load_candidate_metrics: Vec<BaselineFindingMetric>,
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
            dynamic_import_count: 0,
            potential_kb_saved: 0,
            heavy_dependency_names: Vec::new(),
            tree_shaking_warning_keys: Vec::new(),
            duplicate_package_keys: Vec::new(),
            lazy_load_candidate_keys: Vec::new(),
            boundary_warning_keys: Vec::new(),
            unused_dependency_candidate_names: Vec::new(),
            heavy_dependency_metrics: Vec::new(),
            tree_shaking_warning_metrics: Vec::new(),
            duplicate_package_metrics: Vec::new(),
            lazy_load_candidate_metrics: Vec::new(),
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
            dynamic_import_count: analysis.source_summary.dynamic_imports,
            potential_kb_saved: analysis.impact.potential_kb_saved,
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
            duplicate_package_keys: unique_sorted(
                analysis
                    .duplicate_packages
                    .iter()
                    .map(duplicate_package_key),
            ),
            lazy_load_candidate_keys: unique_sorted(
                analysis
                    .lazy_load_candidates
                    .iter()
                    .map(lazy_load_candidate_key),
            ),
            boundary_warning_keys: unique_sorted(
                analysis.boundary_warnings.iter().map(boundary_warning_key),
            ),
            unused_dependency_candidate_names: unique_sorted(
                analysis
                    .unused_dependency_candidates
                    .iter()
                    .map(|item| item.name.clone()),
            ),
            heavy_dependency_metrics: unique_sorted_metrics(
                analysis
                    .heavy_dependencies
                    .iter()
                    .map(heavy_dependency_metric),
            ),
            tree_shaking_warning_metrics: unique_sorted_metrics(
                analysis
                    .tree_shaking_warnings
                    .iter()
                    .map(tree_shaking_warning_metric),
            ),
            duplicate_package_metrics: unique_sorted_metrics(
                analysis
                    .duplicate_packages
                    .iter()
                    .map(duplicate_package_metric),
            ),
            lazy_load_candidate_metrics: unique_sorted_metrics(
                analysis
                    .lazy_load_candidates
                    .iter()
                    .map(lazy_load_candidate_metric),
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
    pub dynamic_import_count_previous: usize,
    pub dynamic_import_count_current: usize,
    pub potential_kb_saved_previous: usize,
    pub potential_kb_saved_current: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_heavy_dependency_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_heavy_dependency_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worsened_heavy_dependency_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_tree_shaking_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_tree_shaking_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worsened_tree_shaking_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_duplicate_package_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_duplicate_package_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worsened_duplicate_package_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_lazy_load_candidate_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_lazy_load_candidate_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worsened_lazy_load_candidate_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_boundary_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_boundary_warning_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_unused_dependency_candidate_names: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_unused_dependency_candidate_names: Vec<String>,
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
            && self.dynamic_import_count_previous == self.dynamic_import_count_current
            && self.potential_kb_saved_previous == self.potential_kb_saved_current
            && self.added_heavy_dependency_names.is_empty()
            && self.removed_heavy_dependency_names.is_empty()
            && self.worsened_heavy_dependency_names.is_empty()
            && self.added_tree_shaking_warning_keys.is_empty()
            && self.removed_tree_shaking_warning_keys.is_empty()
            && self.worsened_tree_shaking_warning_keys.is_empty()
            && self.added_duplicate_package_keys.is_empty()
            && self.removed_duplicate_package_keys.is_empty()
            && self.worsened_duplicate_package_keys.is_empty()
            && self.added_lazy_load_candidate_keys.is_empty()
            && self.removed_lazy_load_candidate_keys.is_empty()
            && self.worsened_lazy_load_candidate_keys.is_empty()
            && self.added_boundary_warning_keys.is_empty()
            && self.removed_boundary_warning_keys.is_empty()
            && self.added_unused_dependency_candidate_names.is_empty()
            && self.removed_unused_dependency_candidate_names.is_empty()
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
        dynamic_import_count_previous: previous.dynamic_import_count,
        dynamic_import_count_current: current.dynamic_import_count,
        potential_kb_saved_previous: previous.potential_kb_saved,
        potential_kb_saved_current: current.potential_kb_saved,
        added_heavy_dependency_names: diff_added(
            previous.heavy_dependency_names.as_slice(),
            current.heavy_dependency_names.as_slice(),
        ),
        removed_heavy_dependency_names: diff_removed(
            previous.heavy_dependency_names.as_slice(),
            current.heavy_dependency_names.as_slice(),
        ),
        worsened_heavy_dependency_names: diff_worsened(
            previous.heavy_dependency_metrics.as_slice(),
            current.heavy_dependency_metrics.as_slice(),
        ),
        added_tree_shaking_warning_keys: diff_added(
            previous.tree_shaking_warning_keys.as_slice(),
            current.tree_shaking_warning_keys.as_slice(),
        ),
        removed_tree_shaking_warning_keys: diff_removed(
            previous.tree_shaking_warning_keys.as_slice(),
            current.tree_shaking_warning_keys.as_slice(),
        ),
        worsened_tree_shaking_warning_keys: diff_worsened(
            previous.tree_shaking_warning_metrics.as_slice(),
            current.tree_shaking_warning_metrics.as_slice(),
        ),
        added_duplicate_package_keys: diff_added(
            previous.duplicate_package_keys.as_slice(),
            current.duplicate_package_keys.as_slice(),
        ),
        removed_duplicate_package_keys: diff_removed(
            previous.duplicate_package_keys.as_slice(),
            current.duplicate_package_keys.as_slice(),
        ),
        worsened_duplicate_package_keys: diff_worsened(
            previous.duplicate_package_metrics.as_slice(),
            current.duplicate_package_metrics.as_slice(),
        ),
        added_lazy_load_candidate_keys: diff_added(
            previous.lazy_load_candidate_keys.as_slice(),
            current.lazy_load_candidate_keys.as_slice(),
        ),
        removed_lazy_load_candidate_keys: diff_removed(
            previous.lazy_load_candidate_keys.as_slice(),
            current.lazy_load_candidate_keys.as_slice(),
        ),
        worsened_lazy_load_candidate_keys: diff_worsened(
            previous.lazy_load_candidate_metrics.as_slice(),
            current.lazy_load_candidate_metrics.as_slice(),
        ),
        added_boundary_warning_keys: diff_added(
            previous.boundary_warning_keys.as_slice(),
            current.boundary_warning_keys.as_slice(),
        ),
        removed_boundary_warning_keys: diff_removed(
            previous.boundary_warning_keys.as_slice(),
            current.boundary_warning_keys.as_slice(),
        ),
        added_unused_dependency_candidate_names: diff_added(
            previous.unused_dependency_candidate_names.as_slice(),
            current.unused_dependency_candidate_names.as_slice(),
        ),
        removed_unused_dependency_candidate_names: diff_removed(
            previous.unused_dependency_candidate_names.as_slice(),
            current.unused_dependency_candidate_names.as_slice(),
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

fn unique_sorted_metrics<I>(values: I) -> Vec<BaselineFindingMetric>
where
    I: IntoIterator<Item = BaselineFindingMetric>,
{
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_by(|left, right| left.key.cmp(&right.key));
    values
}

fn heavy_dependency_metric(item: &crate::models::HeavyDependency) -> BaselineFindingMetric {
    BaselineFindingMetric {
        key: item.name.clone(),
        primary_metric: item.estimated_kb,
        secondary_metric: Some(item.import_count),
    }
}

fn tree_shaking_warning_metric(item: &crate::models::TreeShakingWarning) -> BaselineFindingMetric {
    BaselineFindingMetric {
        key: item.key.clone(),
        primary_metric: item.estimated_kb,
        secondary_metric: Some(item.files.len()),
    }
}

fn duplicate_package_key(item: &DuplicatePackage) -> String {
    item.finding
        .finding_id
        .clone()
        .unwrap_or_else(|| item.name.clone())
}

fn duplicate_package_metric(item: &DuplicatePackage) -> BaselineFindingMetric {
    BaselineFindingMetric {
        key: duplicate_package_key(item),
        primary_metric: item.estimated_extra_kb,
        secondary_metric: Some(item.count),
    }
}

fn lazy_load_candidate_key(item: &LazyLoadCandidate) -> String {
    item.finding
        .finding_id
        .clone()
        .unwrap_or_else(|| item.name.clone())
}

pub fn boundary_warning_key(item: &crate::boundaries::BoundaryWarning) -> String {
    let finding_id = item
        .finding
        .finding_id
        .as_deref()
        .unwrap_or("boundary-warning");
    let evidence = item.finding.evidence.first();

    match (
        evidence.and_then(|entry| entry.file.as_deref()),
        evidence.and_then(|entry| entry.specifier.as_deref()),
    ) {
        (Some(file), Some(specifier)) => format!("{finding_id}:{file}:{specifier}"),
        (Some(file), None) => format!("{finding_id}:{file}"),
        (None, Some(specifier)) => format!("{finding_id}:{specifier}"),
        (None, None) => item.message.clone(),
    }
}

fn lazy_load_candidate_metric(item: &LazyLoadCandidate) -> BaselineFindingMetric {
    BaselineFindingMetric {
        key: lazy_load_candidate_key(item),
        primary_metric: item.estimated_savings_kb,
        secondary_metric: Some(item.files.len()),
    }
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

fn diff_worsened(
    previous: &[BaselineFindingMetric],
    current: &[BaselineFindingMetric],
) -> Vec<String> {
    let previous_by_key = previous
        .iter()
        .map(|item| (&item.key, item))
        .collect::<std::collections::BTreeMap<_, _>>();

    current
        .iter()
        .filter_map(|item| {
            let previous = previous_by_key.get(&item.key)?;
            is_worsened_metric(previous, item).then(|| item.key.clone())
        })
        .collect()
}

fn is_worsened_metric(previous: &BaselineFindingMetric, current: &BaselineFindingMetric) -> bool {
    current.primary_metric > previous.primary_metric
        || (current.primary_metric == previous.primary_metric
            && current.secondary_metric.unwrap_or(0) > previous.secondary_metric.unwrap_or(0))
}
