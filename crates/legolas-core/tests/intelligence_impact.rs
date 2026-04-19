use std::{fs, path::PathBuf};

use legolas_core::{
    impact::estimate_impact,
    models::{DuplicatePackage, HeavyDependency, Impact, LazyLoadCandidate, TreeShakingWarning},
    package_intelligence::{get_package_intel, package_intelligence_entries, PackageIntel},
};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct SnapshotPackageIntel {
    package_name: String,
    estimated_kb: usize,
    category: String,
    rationale: String,
    recommendation: String,
}

#[test]
fn get_package_intel_matches_registry_snapshot() {
    let rust_entries = package_intelligence_entries();
    let rust_keys: Vec<&str> = rust_entries.iter().map(|(key, _)| *key).collect();
    let snapshot_entries = load_package_intelligence_snapshot();
    let snapshot_keys: Vec<&str> = snapshot_entries
        .iter()
        .map(|entry| entry.package_name.as_str())
        .collect();

    assert_eq!(rust_keys, snapshot_keys);
    assert_eq!(snapshot_entries.len(), rust_entries.len());

    for ((rust_key, rust_intel), snapshot_entry) in rust_entries.iter().zip(snapshot_entries) {
        assert_eq!(*rust_key, snapshot_entry.package_name);
        assert_eq!(get_package_intel(rust_key), Some(*rust_intel));
        assert_eq!(snapshot(rust_key, *rust_intel), snapshot_entry);
    }
}

#[test]
fn get_package_intel_requires_an_exact_package_key_match() {
    assert_eq!(get_package_intel("lodash/fp"), None);
    assert_eq!(get_package_intel("@mui/icons-material/AccessAlarm"), None);
    assert_eq!(get_package_intel("unknown-package"), None);
}

#[test]
fn estimate_impact_matches_snapshot_directional_formula() {
    let case = load_impact_case("directional_formula");
    let impact = estimate_impact_for_payload(&case.payload);

    assert_eq!(impact, case.expected);
    assert_eq!(impact.potential_kb_saved, 435);
    assert_eq!(impact.estimated_lcp_improvement_ms, 914);
    assert_eq!(impact.confidence, "directional");
    assert_eq!(
        impact.summary,
        "High impact: the project has clear opportunities to reduce initial payload size."
    );
}

#[test]
fn estimate_impact_matches_snapshot_for_unsorted_inputs() {
    let case = load_impact_case("unsorted_inputs");
    let impact = estimate_impact_for_payload(&case.payload);

    assert_eq!(impact, case.expected);
    assert_eq!(impact.potential_kb_saved, 345);
    assert_eq!(impact.estimated_lcp_improvement_ms, 725);
}

#[test]
fn estimate_impact_preserves_fractional_rounding_behavior() {
    let impact = estimate_impact_for_payload(&load_impact_case("fractional_rounding").payload);

    assert_eq!(impact, load_impact_case("fractional_rounding").expected);
    assert_eq!(impact.potential_kb_saved, 5);
    assert_eq!(impact.estimated_lcp_improvement_ms, 11);
    assert_eq!(impact.confidence, "directional");
    assert_eq!(
        impact.summary,
        "Low impact: obvious bundle issues are limited in the current scan."
    );

    let threshold_impact =
        estimate_impact_for_payload(&load_impact_case("threshold_targeted").payload);

    assert_eq!(
        threshold_impact,
        load_impact_case("threshold_targeted").expected
    );
    assert_eq!(threshold_impact.potential_kb_saved, 40);
    assert_eq!(threshold_impact.estimated_lcp_improvement_ms, 84);
    assert_eq!(
        threshold_impact.summary,
        "Targeted impact: a handful of focused optimizations should pay off."
    );
}

#[test]
fn estimate_impact_uses_the_js_summary_thresholds() {
    let cases = [
        (
            "summary_low_zero",
            0,
            "low",
            "Low impact: obvious bundle issues are limited in the current scan.",
        ),
        (
            "summary_low_39",
            39,
            "directional",
            "Low impact: obvious bundle issues are limited in the current scan.",
        ),
        (
            "summary_targeted_40",
            40,
            "directional",
            "Targeted impact: a handful of focused optimizations should pay off.",
        ),
        (
            "summary_medium_120",
            120,
            "directional",
            "Medium impact: there are several meaningful bundle wins available.",
        ),
        (
            "summary_high_300",
            300,
            "directional",
            "High impact: the project has clear opportunities to reduce initial payload size.",
        ),
    ];

    for (case_name, potential_kb_saved, expected_confidence, expected_summary) in cases {
        let case = load_impact_case(case_name);
        let impact = estimate_impact_for_payload(&case.payload);

        assert_eq!(impact, case.expected);
        assert_eq!(impact.potential_kb_saved, potential_kb_saved);
        assert_eq!(impact.confidence, expected_confidence);
        assert_eq!(impact.summary, expected_summary);
    }
}

fn heavy_dependency(estimated_kb: usize) -> HeavyDependency {
    HeavyDependency {
        estimated_kb,
        ..HeavyDependency::default()
    }
}

fn duplicate_package(estimated_extra_kb: usize) -> DuplicatePackage {
    DuplicatePackage {
        estimated_extra_kb,
        ..DuplicatePackage::default()
    }
}

fn lazy_load_candidate(estimated_savings_kb: usize) -> LazyLoadCandidate {
    LazyLoadCandidate {
        estimated_savings_kb,
        ..LazyLoadCandidate::default()
    }
}

fn tree_shaking_warning(estimated_kb: usize) -> TreeShakingWarning {
    TreeShakingWarning {
        estimated_kb,
        ..TreeShakingWarning::default()
    }
}

fn snapshot(package_name: &str, intel: PackageIntel) -> SnapshotPackageIntel {
    SnapshotPackageIntel {
        package_name: package_name.to_string(),
        estimated_kb: intel.estimated_kb,
        category: intel.category.to_string(),
        rationale: intel.rationale.to_string(),
        recommendation: intel.recommendation.to_string(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImpactCaseSnapshot {
    name: String,
    payload: ImpactPayloadSnapshot,
    expected: Impact,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImpactPayloadSnapshot {
    heavy_dependencies: Vec<EstimatedKbSnapshot>,
    duplicate_packages: Vec<EstimatedExtraKbSnapshot>,
    lazy_load_candidates: Vec<EstimatedSavingsKbSnapshot>,
    tree_shaking_warnings: Vec<EstimatedKbSnapshot>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EstimatedKbSnapshot {
    estimated_kb: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EstimatedExtraKbSnapshot {
    estimated_extra_kb: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EstimatedSavingsKbSnapshot {
    estimated_savings_kb: usize,
}

fn load_package_intelligence_snapshot() -> Vec<SnapshotPackageIntel> {
    let contents = fs::read_to_string(
        workspace_root().join("tests/oracles/package-intelligence/registry.json"),
    )
    .expect("read package intelligence snapshot");

    serde_json::from_str(&contents).expect("deserialize package intelligence snapshot")
}

fn load_impact_case(case_name: &str) -> ImpactCaseSnapshot {
    load_impact_cases()
        .into_iter()
        .find(|entry| entry.name == case_name)
        .unwrap_or_else(|| panic!("missing impact case snapshot: {case_name}"))
}

fn load_impact_cases() -> Vec<ImpactCaseSnapshot> {
    let contents = fs::read_to_string(workspace_root().join("tests/oracles/impact/cases.json"))
        .expect("read impact case snapshot");

    serde_json::from_str(&contents).expect("deserialize impact case snapshot")
}

fn estimate_impact_for_payload(payload: &ImpactPayloadSnapshot) -> Impact {
    let heavy_dependencies = payload
        .heavy_dependencies
        .iter()
        .map(|entry| heavy_dependency(entry.estimated_kb))
        .collect::<Vec<_>>();
    let duplicate_packages = payload
        .duplicate_packages
        .iter()
        .map(|entry| duplicate_package(entry.estimated_extra_kb))
        .collect::<Vec<_>>();
    let lazy_load_candidates = payload
        .lazy_load_candidates
        .iter()
        .map(|entry| lazy_load_candidate(entry.estimated_savings_kb))
        .collect::<Vec<_>>();
    let tree_shaking_warnings = payload
        .tree_shaking_warnings
        .iter()
        .map(|entry| tree_shaking_warning(entry.estimated_kb))
        .collect::<Vec<_>>();

    estimate_impact(
        &heavy_dependencies,
        &duplicate_packages,
        &lazy_load_candidates,
        &tree_shaking_warnings,
    )
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}
