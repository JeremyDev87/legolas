mod support;

use std::fs;

use legolas_core::{
    analyze_project, baseline::BASELINE_SCHEMA_VERSION, diff_analysis, BaselineFindingMetric,
    BaselineSnapshot,
};

#[test]
fn baseline_diff_detects_new_package_and_tree_shaking_warning() {
    let previous: BaselineSnapshot = serde_json::from_str(
        &fs::read_to_string(support::fixture_path(
            "tests/fixtures/baseline/previous-scan.json",
        ))
        .expect("read previous baseline"),
    )
    .expect("parse previous baseline");
    let current_analysis =
        analyze_project(support::fixture_path("tests/fixtures/baseline/current-app"))
            .expect("analyze current app");
    let current = BaselineSnapshot::from_analysis(&current_analysis);
    let diff = diff_analysis(&previous, &current_analysis);

    assert_eq!(current.project_name, "baseline-app");
    assert_eq!(current.package_manager, "npm");
    assert_eq!(
        diff.added_heavy_dependency_names,
        vec!["lodash".to_string()]
    );
    assert_eq!(diff.removed_heavy_dependency_names, Vec::<String>::new());
    assert_eq!(
        diff.worsened_heavy_dependency_names,
        vec!["chart.js".to_string()]
    );
    assert_eq!(
        diff.added_tree_shaking_warning_keys,
        vec!["lodash-root-import".to_string()]
    );
    assert_eq!(diff.removed_tree_shaking_warning_keys, Vec::<String>::new());
    assert_eq!(
        diff.worsened_tree_shaking_warning_keys,
        Vec::<String>::new()
    );
    assert_eq!(diff.added_duplicate_package_keys, Vec::<String>::new());
    assert_eq!(diff.removed_duplicate_package_keys, Vec::<String>::new());
    assert_eq!(diff.worsened_duplicate_package_keys, Vec::<String>::new());
    assert_eq!(diff.added_lazy_load_candidate_keys, Vec::<String>::new());
    assert_eq!(diff.removed_lazy_load_candidate_keys, Vec::<String>::new());
    assert_eq!(diff.worsened_lazy_load_candidate_keys, Vec::<String>::new());
    assert_eq!(diff.dependency_count_previous, 1);
    assert_eq!(diff.dependency_count_current, 2);
    assert_eq!(diff.source_file_count_previous, 1);
    assert_eq!(diff.source_file_count_current, 1);
    assert_eq!(diff.dynamic_import_count_previous, 0);
    assert_eq!(diff.dynamic_import_count_current, 0);
    assert!(!diff.is_empty());
}

#[test]
fn baseline_snapshot_round_trips_as_json() {
    let snapshot = BaselineSnapshot {
        schema_version: BASELINE_SCHEMA_VERSION,
        project_name: "baseline-app".to_string(),
        package_manager: "npm".to_string(),
        dependency_count: 1,
        dev_dependency_count: 0,
        source_file_count: 1,
        dynamic_import_count: 1,
        potential_kb_saved: 42,
        heavy_dependency_names: vec!["chart.js".to_string()],
        tree_shaking_warning_keys: Vec::new(),
        duplicate_package_keys: vec!["duplicate-package:react".to_string()],
        lazy_load_candidate_keys: vec!["lazy-load:chart.js".to_string()],
        boundary_warning_keys: vec![
            "boundary:server-client:fs:src/client/App.tsx:node:fs".to_string()
        ],
        unused_dependency_candidate_names: vec!["lodash".to_string()],
        heavy_dependency_metrics: vec![BaselineFindingMetric {
            key: "chart.js".to_string(),
            primary_metric: 66,
            secondary_metric: Some(1),
        }],
        tree_shaking_warning_metrics: Vec::new(),
        duplicate_package_metrics: vec![BaselineFindingMetric {
            key: "duplicate-package:react".to_string(),
            primary_metric: 12,
            secondary_metric: Some(2),
        }],
        lazy_load_candidate_metrics: vec![BaselineFindingMetric {
            key: "lazy-load:chart.js".to_string(),
            primary_metric: 48,
            secondary_metric: Some(1),
        }],
        warnings: Vec::new(),
    };

    let encoded = serde_json::to_string_pretty(&snapshot).expect("serialize snapshot");
    let decoded: BaselineSnapshot = serde_json::from_str(&encoded).expect("deserialize snapshot");

    assert_eq!(decoded, snapshot);
}

#[test]
fn baseline_diff_detects_worsened_existing_findings() {
    let previous = BaselineSnapshot {
        schema_version: BASELINE_SCHEMA_VERSION,
        project_name: "baseline-app".to_string(),
        package_manager: "npm".to_string(),
        dependency_count: 1,
        dev_dependency_count: 0,
        source_file_count: 1,
        dynamic_import_count: 2,
        potential_kb_saved: 120,
        heavy_dependency_names: vec!["lodash".to_string()],
        tree_shaking_warning_keys: Vec::new(),
        duplicate_package_keys: vec!["duplicate-package:react".to_string()],
        lazy_load_candidate_keys: vec!["lazy-load:chart.js".to_string()],
        boundary_warning_keys: Vec::new(),
        unused_dependency_candidate_names: Vec::new(),
        heavy_dependency_metrics: vec![BaselineFindingMetric {
            key: "lodash".to_string(),
            primary_metric: 72,
            secondary_metric: Some(1),
        }],
        tree_shaking_warning_metrics: Vec::new(),
        duplicate_package_metrics: vec![BaselineFindingMetric {
            key: "duplicate-package:react".to_string(),
            primary_metric: 12,
            secondary_metric: Some(1),
        }],
        lazy_load_candidate_metrics: vec![BaselineFindingMetric {
            key: "lazy-load:chart.js".to_string(),
            primary_metric: 48,
            secondary_metric: Some(1),
        }],
        warnings: Vec::new(),
    };
    let current = BaselineSnapshot {
        schema_version: BASELINE_SCHEMA_VERSION,
        project_name: "baseline-app".to_string(),
        package_manager: "npm".to_string(),
        dependency_count: 1,
        dev_dependency_count: 0,
        source_file_count: 1,
        dynamic_import_count: 1,
        potential_kb_saved: 156,
        heavy_dependency_names: vec!["lodash".to_string()],
        tree_shaking_warning_keys: Vec::new(),
        duplicate_package_keys: vec!["duplicate-package:react".to_string()],
        lazy_load_candidate_keys: vec!["lazy-load:chart.js".to_string()],
        boundary_warning_keys: Vec::new(),
        unused_dependency_candidate_names: Vec::new(),
        heavy_dependency_metrics: vec![BaselineFindingMetric {
            key: "lodash".to_string(),
            primary_metric: 72,
            secondary_metric: Some(2),
        }],
        tree_shaking_warning_metrics: Vec::new(),
        duplicate_package_metrics: vec![BaselineFindingMetric {
            key: "duplicate-package:react".to_string(),
            primary_metric: 20,
            secondary_metric: Some(2),
        }],
        lazy_load_candidate_metrics: vec![BaselineFindingMetric {
            key: "lazy-load:chart.js".to_string(),
            primary_metric: 64,
            secondary_metric: Some(2),
        }],
        warnings: Vec::new(),
    };

    let diff = legolas_core::diff_baselines(&previous, &current);

    assert_eq!(
        diff.worsened_heavy_dependency_names,
        vec!["lodash".to_string()]
    );
    assert_eq!(
        diff.worsened_duplicate_package_keys,
        vec!["duplicate-package:react".to_string()]
    );
    assert_eq!(
        diff.worsened_lazy_load_candidate_keys,
        vec!["lazy-load:chart.js".to_string()]
    );
    assert_eq!(diff.dynamic_import_count_previous, 2);
    assert_eq!(diff.dynamic_import_count_current, 1);
    assert_eq!(diff.potential_kb_saved_previous, 120);
    assert_eq!(diff.potential_kb_saved_current, 156);
}

#[test]
fn baseline_diff_detects_added_boundary_and_unused_dependency_regressions() {
    let previous = BaselineSnapshot {
        schema_version: BASELINE_SCHEMA_VERSION,
        project_name: "baseline-app".to_string(),
        package_manager: "npm".to_string(),
        dependency_count: 2,
        dev_dependency_count: 0,
        source_file_count: 1,
        dynamic_import_count: 0,
        potential_kb_saved: 0,
        heavy_dependency_names: Vec::new(),
        tree_shaking_warning_keys: Vec::new(),
        duplicate_package_keys: Vec::new(),
        lazy_load_candidate_keys: Vec::new(),
        boundary_warning_keys: vec![
            "boundary:server-client:fs:src/client/existing.ts:node:fs".to_string()
        ],
        unused_dependency_candidate_names: vec!["lodash".to_string()],
        heavy_dependency_metrics: Vec::new(),
        tree_shaking_warning_metrics: Vec::new(),
        duplicate_package_metrics: Vec::new(),
        lazy_load_candidate_metrics: Vec::new(),
        warnings: Vec::new(),
    };
    let current = BaselineSnapshot {
        schema_version: BASELINE_SCHEMA_VERSION,
        project_name: "baseline-app".to_string(),
        package_manager: "npm".to_string(),
        dependency_count: 2,
        dev_dependency_count: 0,
        source_file_count: 1,
        dynamic_import_count: 0,
        potential_kb_saved: 0,
        heavy_dependency_names: Vec::new(),
        tree_shaking_warning_keys: Vec::new(),
        duplicate_package_keys: Vec::new(),
        lazy_load_candidate_keys: Vec::new(),
        boundary_warning_keys: vec![
            "boundary:server-client:fs:src/client/existing.ts:node:fs".to_string(),
            "boundary:server-client:path:src/client/new.ts:node:path".to_string(),
        ],
        unused_dependency_candidate_names: vec!["chart.js".to_string(), "lodash".to_string()],
        heavy_dependency_metrics: Vec::new(),
        tree_shaking_warning_metrics: Vec::new(),
        duplicate_package_metrics: Vec::new(),
        lazy_load_candidate_metrics: Vec::new(),
        warnings: Vec::new(),
    };

    let diff = legolas_core::diff_baselines(&previous, &current);

    assert_eq!(
        diff.added_boundary_warning_keys,
        vec!["boundary:server-client:path:src/client/new.ts:node:path".to_string()]
    );
    assert_eq!(diff.removed_boundary_warning_keys, Vec::<String>::new());
    assert_eq!(
        diff.added_unused_dependency_candidate_names,
        vec!["chart.js".to_string()]
    );
    assert_eq!(
        diff.removed_unused_dependency_candidate_names,
        Vec::<String>::new()
    );
}

#[test]
fn baseline_diff_detects_added_boundary_warning_for_same_package_in_new_file() {
    let previous = BaselineSnapshot {
        schema_version: BASELINE_SCHEMA_VERSION,
        project_name: "baseline-app".to_string(),
        package_manager: "npm".to_string(),
        dependency_count: 0,
        dev_dependency_count: 0,
        source_file_count: 1,
        dynamic_import_count: 0,
        potential_kb_saved: 0,
        heavy_dependency_names: Vec::new(),
        tree_shaking_warning_keys: Vec::new(),
        duplicate_package_keys: Vec::new(),
        lazy_load_candidate_keys: Vec::new(),
        boundary_warning_keys: vec![
            "boundary:server-client:fs:src/client/existing.ts:node:fs".to_string()
        ],
        unused_dependency_candidate_names: Vec::new(),
        heavy_dependency_metrics: Vec::new(),
        tree_shaking_warning_metrics: Vec::new(),
        duplicate_package_metrics: Vec::new(),
        lazy_load_candidate_metrics: Vec::new(),
        warnings: Vec::new(),
    };
    let current = BaselineSnapshot {
        schema_version: BASELINE_SCHEMA_VERSION,
        project_name: "baseline-app".to_string(),
        package_manager: "npm".to_string(),
        dependency_count: 0,
        dev_dependency_count: 0,
        source_file_count: 2,
        dynamic_import_count: 0,
        potential_kb_saved: 0,
        heavy_dependency_names: Vec::new(),
        tree_shaking_warning_keys: Vec::new(),
        duplicate_package_keys: Vec::new(),
        lazy_load_candidate_keys: Vec::new(),
        boundary_warning_keys: vec![
            "boundary:server-client:fs:src/client/existing.ts:node:fs".to_string(),
            "boundary:server-client:fs:src/client/new.ts:node:fs".to_string(),
        ],
        unused_dependency_candidate_names: Vec::new(),
        heavy_dependency_metrics: Vec::new(),
        tree_shaking_warning_metrics: Vec::new(),
        duplicate_package_metrics: Vec::new(),
        lazy_load_candidate_metrics: Vec::new(),
        warnings: Vec::new(),
    };

    let diff = legolas_core::diff_baselines(&previous, &current);

    assert_eq!(
        diff.added_boundary_warning_keys,
        vec!["boundary:server-client:fs:src/client/new.ts:node:fs".to_string()]
    );
    assert_eq!(diff.removed_boundary_warning_keys, Vec::<String>::new());
}
