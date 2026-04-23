mod support;

use legolas_core::{
    fix_hints::{
        dedupe_resolution_fix_hint, dynamic_import_fix_hint, route_split_fix_hint,
        subpath_import_fix_hint,
    },
    FindingAnalysisSource, FindingConfidence, FindingMetadata,
};

#[test]
fn dynamic_import_fix_hint_requires_high_confidence_and_normalizes_target_files() {
    assert!(
        support::fixture_path("tests/fixtures/fix-hints/dynamic-import/src/Dashboard.tsx").exists()
    );
    assert!(support::fixture_path("tests/fixtures/fix-hints/subpath-import/src/App.tsx").exists());

    let fix = dynamic_import_fix_hint(
        &high_confidence_finding("dynamic-import"),
        "Split the dashboard code path.",
        vec![
            "tests/fixtures/fix-hints/subpath-import/src/App.tsx".to_string(),
            "tests/fixtures/fix-hints/dynamic-import/src/Dashboard.tsx".to_string(),
            "tests/fixtures/fix-hints/subpath-import/src/App.tsx".to_string(),
        ],
    )
    .expect("dynamic import fix hint");

    assert_eq!(fix.kind, "dynamic-import");
    assert_eq!(
        fix.target_files,
        vec![
            "tests/fixtures/fix-hints/dynamic-import/src/Dashboard.tsx".to_string(),
            "tests/fixtures/fix-hints/subpath-import/src/App.tsx".to_string(),
        ]
    );
    assert_eq!(fix.replacement, None);
}

#[test]
fn subpath_import_fix_hint_preserves_replacement() {
    let fix = subpath_import_fix_hint(
        &high_confidence_finding("subpath-import"),
        "Use package subpath imports.",
        vec![
            "tests/fixtures/fix-hints/subpath-import/src/App.tsx".to_string(),
            "tests/fixtures/fix-hints/dynamic-import/src/Dashboard.tsx".to_string(),
            "tests/fixtures/fix-hints/subpath-import/src/App.tsx".to_string(),
        ],
        Some("lodash-es".to_string()),
    )
    .expect("subpath import fix hint");

    assert_eq!(fix.kind, "subpath-import");
    assert_eq!(
        fix.target_files,
        vec![
            "tests/fixtures/fix-hints/dynamic-import/src/Dashboard.tsx".to_string(),
            "tests/fixtures/fix-hints/subpath-import/src/App.tsx".to_string(),
        ]
    );
    assert_eq!(fix.replacement.as_deref(), Some("lodash-es"));
}

#[test]
fn route_split_fix_hint_rejects_non_high_confidence_findings_and_empty_targets() {
    let low_confidence_fix = route_split_fix_hint(
        &low_confidence_finding("route-split"),
        "Split the route bundle.",
        vec!["tests/fixtures/fix-hints/dynamic-import/src/Dashboard.tsx".to_string()],
    );

    let empty_target_fix = route_split_fix_hint(
        &high_confidence_finding("route-split-empty"),
        "Split the route bundle.",
        Vec::new(),
    );

    assert!(low_confidence_fix.is_none());
    assert!(empty_target_fix.is_none());
}

#[test]
fn dedupe_resolution_fix_hint_allows_empty_target_files() {
    let fix = dedupe_resolution_fix_hint(
        &high_confidence_finding("dedupe-resolution"),
        "Deduplicate lodash to one installed version.",
    )
    .expect("dedupe resolution fix hint");

    assert_eq!(fix.kind, "dedupe-resolution");
    assert!(fix.target_files.is_empty());
    assert_eq!(fix.replacement, None);
}

fn high_confidence_finding(finding_id: &str) -> FindingMetadata {
    FindingMetadata::new(finding_id, FindingAnalysisSource::Heuristic)
        .with_confidence(FindingConfidence::High)
}

fn low_confidence_finding(finding_id: &str) -> FindingMetadata {
    FindingMetadata::new(finding_id, FindingAnalysisSource::Heuristic)
        .with_confidence(FindingConfidence::Low)
}
