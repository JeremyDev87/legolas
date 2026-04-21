mod support;

use std::fs;

use legolas_core::{
    analyze_project, FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata,
    RecommendedFix,
};
use tempfile::tempdir;

#[test]
fn analyze_project_populates_heavy_dependency_evidence() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/findings/evidence-app",
    ))
    .expect("analyze evidence fixture");
    let heavy_dependency = analysis
        .heavy_dependencies
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js heavy dependency");

    assert_finding_metadata(
        &heavy_dependency.finding,
        FindingMetadata::new(
            "heavy-dependency:chart.js",
            FindingAnalysisSource::SourceImport,
        )
        .with_confidence(FindingConfidence::High)
        .with_evidence([FindingEvidence::new("source-file")
            .with_file("src/AdminDashboard.tsx")
            .with_specifier("chart.js")
            .with_detail(
                "static import; Charting code is often only needed on a subset of screens.",
            )]),
    );
    assert_eq!(heavy_dependency.finding.action_priority, Some(1));
    assert_recommended_fix(
        heavy_dependency.finding.recommended_fix.as_ref(),
        "lazy-load",
        &["src/AdminDashboard.tsx"],
        None,
    );
}

#[test]
fn analyze_project_populates_lazy_load_candidate_evidence() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/findings/evidence-app",
    ))
    .expect("analyze evidence fixture");
    let candidate = analysis
        .lazy_load_candidates
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js lazy-load candidate");

    assert_finding_metadata(
        &candidate.finding,
        FindingMetadata::new("lazy-load:chart.js", FindingAnalysisSource::Heuristic)
            .with_confidence(FindingConfidence::Medium)
            .with_evidence([FindingEvidence::new("source-file")
                .with_file("src/AdminDashboard.tsx")
                .with_specifier("chart.js")
                .with_detail("route-like UI surface matched `admin` keyword")]),
    );
    assert_eq!(candidate.finding.action_priority, Some(2));
    assert_recommended_fix(
        candidate.finding.recommended_fix.as_ref(),
        "lazy-load",
        &["src/AdminDashboard.tsx"],
        None,
    );
}

#[test]
fn analyze_project_populates_tree_shaking_warning_evidence() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/findings/evidence-app",
    ))
    .expect("analyze evidence fixture");
    let warning = analysis
        .tree_shaking_warnings
        .iter()
        .find(|item| item.key == "lodash-root-import")
        .expect("lodash tree-shaking warning");

    assert_finding_metadata(
        &warning.finding,
        FindingMetadata::new(
            "tree-shaking:lodash-root-import",
            FindingAnalysisSource::SourceImport,
        )
        .with_confidence(FindingConfidence::High)
        .with_evidence([FindingEvidence::new("source-file")
            .with_file("src/AdminDashboard.tsx")
            .with_specifier("lodash")
            .with_detail("root package import")]),
    );
    assert!(warning.finding.action_priority.is_some());
    assert_recommended_fix(
        warning.finding.recommended_fix.as_ref(),
        "narrow-import",
        &["src/AdminDashboard.tsx"],
        Some("lodash-es"),
    );
}

#[test]
fn analyze_project_limits_lazy_load_evidence_to_candidate_files() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "lazy-evidence-app",
  "dependencies": {
    "chart.js": "^4.4.1"
  }
}"#,
    );
    write_file(
        root,
        "src/Dashboard.tsx",
        "import { Chart } from \"chart.js\";\nexport const Dashboard = Chart;\n",
    );
    write_file(
        root,
        "src/utils/shared.ts",
        "import { Chart } from \"chart.js\";\nexport const shared = Chart;\n",
    );

    let analysis = analyze_project(root).expect("analyze lazy evidence project");
    let candidate = analysis
        .lazy_load_candidates
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js lazy-load candidate");

    assert_eq!(candidate.files, vec!["src/Dashboard.tsx".to_string()]);
    assert_finding_metadata(
        &candidate.finding,
        FindingMetadata::new("lazy-load:chart.js", FindingAnalysisSource::Heuristic)
            .with_confidence(FindingConfidence::Medium)
            .with_evidence([FindingEvidence::new("source-file")
                .with_file("src/Dashboard.tsx")
                .with_specifier("chart.js")
                .with_detail("route-like UI surface matched `dashboard` keyword")]),
    );
    assert_eq!(candidate.finding.action_priority, Some(2));
    assert_recommended_fix(
        candidate.finding.recommended_fix.as_ref(),
        "lazy-load",
        &["src/Dashboard.tsx"],
        None,
    );
}

fn write_file(root: &std::path::Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    let parent = path.parent().expect("fixture file parent");
    fs::create_dir_all(parent).expect("create fixture directory");
    fs::write(path, contents).expect("write fixture file");
}

fn assert_finding_metadata(actual: &FindingMetadata, expected: FindingMetadata) {
    assert_eq!(actual.finding_id, expected.finding_id);
    assert_eq!(actual.analysis_source, expected.analysis_source);
    assert_eq!(actual.confidence, expected.confidence);
    assert_eq!(actual.evidence, expected.evidence);
}

fn assert_recommended_fix(
    actual: Option<&RecommendedFix>,
    expected_kind: &str,
    expected_target_files: &[&str],
    expected_replacement: Option<&str>,
) {
    let actual = actual.expect("recommended fix");
    assert_eq!(actual.kind, expected_kind);
    assert_eq!(
        actual.target_files,
        expected_target_files
            .iter()
            .map(|file| (*file).to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(actual.replacement.as_deref(), expected_replacement);
}
