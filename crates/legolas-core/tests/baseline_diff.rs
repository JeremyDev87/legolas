mod support;

use std::fs;

use legolas_core::{analyze_project, diff_analysis, BaselineSnapshot};

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
        diff.added_tree_shaking_warning_keys,
        vec!["lodash-root-import".to_string()]
    );
    assert_eq!(diff.removed_tree_shaking_warning_keys, Vec::<String>::new());
    assert_eq!(diff.dependency_count_previous, 1);
    assert_eq!(diff.dependency_count_current, 2);
    assert_eq!(diff.source_file_count_previous, 1);
    assert_eq!(diff.source_file_count_current, 1);
    assert!(!diff.is_empty());
}

#[test]
fn baseline_snapshot_round_trips_as_json() {
    let snapshot = BaselineSnapshot {
        schema_version: 1,
        project_name: "baseline-app".to_string(),
        package_manager: "npm".to_string(),
        dependency_count: 1,
        dev_dependency_count: 0,
        source_file_count: 1,
        heavy_dependency_names: vec!["chart.js".to_string()],
        tree_shaking_warning_keys: Vec::new(),
        warnings: Vec::new(),
    };

    let encoded = serde_json::to_string_pretty(&snapshot).expect("serialize snapshot");
    let decoded: BaselineSnapshot = serde_json::from_str(&encoded).expect("deserialize snapshot");

    assert_eq!(decoded, snapshot);
}
