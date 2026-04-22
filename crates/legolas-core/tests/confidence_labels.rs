mod support;

use std::fs;

use legolas_core::{analyze_project, FindingConfidence};
use tempfile::tempdir;

#[test]
fn analyze_project_marks_direct_import_findings_as_high_confidence() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/confidence/high-confidence",
    ))
    .expect("analyze high confidence fixture");

    let heavy_dependency = analysis
        .heavy_dependencies
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js heavy dependency");
    let tree_shaking_warning = analysis
        .tree_shaking_warnings
        .iter()
        .find(|item| item.key == "lodash-root-import")
        .expect("lodash tree-shaking warning");

    assert_eq!(
        heavy_dependency.finding.confidence,
        Some(FindingConfidence::High)
    );
    assert_eq!(
        tree_shaking_warning.finding.confidence,
        Some(FindingConfidence::High)
    );
}

#[test]
fn analyze_project_marks_filename_heuristic_lazy_load_candidates_as_low_confidence() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/confidence/high-confidence",
    ))
    .expect("analyze high confidence fixture");
    let candidate = analysis
        .lazy_load_candidates
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js lazy-load candidate");

    assert_eq!(candidate.finding.confidence, Some(FindingConfidence::Low));
}

#[test]
fn analyze_project_marks_route_aware_lazy_load_candidates_as_medium_confidence() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    fs::write(
        root.join("package.json"),
        r#"{
  "name": "route-aware-confidence-app",
  "dependencies": {
    "chart.js": "^4.4.1",
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}"#,
    )
    .expect("write package.json");
    fs::create_dir_all(root.join("app/reports")).expect("create route dir");
    fs::write(
        root.join("app/reports/page.tsx"),
        "import { Chart } from \"chart.js\";\nexport default function ReportsPage() { return Chart; }\n",
    )
    .expect("write route page");

    let analysis = analyze_project(root).expect("analyze route-aware confidence fixture");
    let candidate = analysis
        .lazy_load_candidates
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js lazy-load candidate");

    assert_eq!(
        candidate.finding.confidence,
        Some(FindingConfidence::Medium)
    );
}

#[test]
fn analyze_project_marks_manifest_only_heavy_dependencies_as_low_confidence() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/confidence/low-confidence",
    ))
    .expect("analyze low confidence fixture");
    let heavy_dependency = analysis
        .heavy_dependencies
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js heavy dependency");

    assert_eq!(heavy_dependency.import_count, 0);
    assert_eq!(
        heavy_dependency.finding.confidence,
        Some(FindingConfidence::Low)
    );
}

#[test]
fn analyze_project_marks_lockfile_duplicate_findings_as_high_confidence() {
    let analysis = analyze_project(support::fixture_path("tests/fixtures/parity/basic-app"))
        .expect("analyze parity fixture");
    let duplicate_package = analysis
        .duplicate_packages
        .iter()
        .find(|item| item.name == "lodash")
        .expect("lodash duplicate package");

    assert_eq!(
        duplicate_package.finding.confidence,
        Some(FindingConfidence::High)
    );
}
