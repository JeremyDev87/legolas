use std::{path::PathBuf, process::Command};

use legolas_core::{
    impact::estimate_impact,
    models::{DuplicatePackage, HeavyDependency, Impact, LazyLoadCandidate, TreeShakingWarning},
    package_intelligence::{get_package_intel, package_intelligence_entries, PackageIntel},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct JsPackageIntel {
    estimated_kb: usize,
    category: String,
    rationale: String,
    recommendation: String,
}

#[test]
fn get_package_intel_matches_every_js_registry_entry() {
    let rust_entries = package_intelligence_entries();
    let rust_keys: Vec<&str> = rust_entries.iter().map(|(key, _)| *key).collect();
    let js_entries = load_js_package_intelligence();
    let js_keys: Vec<&str> = js_entries.iter().map(|(key, _)| key.as_str()).collect();

    assert_eq!(rust_keys, js_keys);
    assert_eq!(js_entries.len(), rust_entries.len());

    for ((rust_key, rust_intel), (js_key, js_intel)) in rust_entries.iter().zip(js_entries) {
        assert_eq!(*rust_key, js_key);
        assert_eq!(get_package_intel(rust_key), Some(*rust_intel));
        assert_eq!(snapshot(*rust_intel), js_intel);
    }
}

#[test]
fn get_package_intel_requires_an_exact_package_key_match() {
    assert_eq!(get_package_intel("lodash/fp"), None);
    assert_eq!(get_package_intel("@mui/icons-material/AccessAlarm"), None);
    assert_eq!(get_package_intel("unknown-package"), None);
}

#[test]
fn estimate_impact_matches_the_js_directional_formula() {
    let heavy_dependencies = vec![
        heavy_dependency(600),
        heavy_dependency(500),
        heavy_dependency(400),
        heavy_dependency(300),
        heavy_dependency(200),
        heavy_dependency(100),
    ];
    let duplicate_packages = vec![duplicate_package(15), duplicate_package(5)];
    let lazy_load_candidates = vec![lazy_load_candidate(30)];
    let tree_shaking_warnings = vec![tree_shaking_warning(25)];

    let impact = estimate_impact(
        &heavy_dependencies,
        &duplicate_packages,
        &lazy_load_candidates,
        &tree_shaking_warnings,
    );
    let js_impact = load_js_impact(&impact_payload(
        &[600, 500, 400, 300, 200, 100],
        &[15, 5],
        &[30],
        &[25],
    ));

    assert_eq!(impact, js_impact);
    assert_eq!(impact.potential_kb_saved, 435);
    assert_eq!(impact.estimated_lcp_improvement_ms, 914);
    assert_eq!(impact.confidence, "directional");
    assert_eq!(
        impact.summary,
        "High impact: the project has clear opportunities to reduce initial payload size."
    );
}

#[test]
fn estimate_impact_matches_js_oracle_for_unsorted_inputs() {
    let impact = estimate_impact(
        &[
            heavy_dependency(100),
            heavy_dependency(200),
            heavy_dependency(300),
            heavy_dependency(400),
            heavy_dependency(500),
            heavy_dependency(600),
        ],
        &[duplicate_package(15), duplicate_package(5)],
        &[lazy_load_candidate(30)],
        &[tree_shaking_warning(25)],
    );
    let js_impact = load_js_impact(&impact_payload(
        &[100, 200, 300, 400, 500, 600],
        &[15, 5],
        &[30],
        &[25],
    ));

    assert_eq!(impact, js_impact);
    assert_eq!(impact.potential_kb_saved, 345);
    assert_eq!(impact.estimated_lcp_improvement_ms, 725);
}

#[test]
fn estimate_impact_preserves_fractional_rounding_behavior() {
    let impact = estimate_impact(&[heavy_dependency(25)], &[], &[], &[]);
    let js_impact = load_js_impact(&impact_payload(&[25], &[], &[], &[]));

    assert_eq!(impact, js_impact);
    assert_eq!(impact.potential_kb_saved, 5);
    assert_eq!(impact.estimated_lcp_improvement_ms, 11);
    assert_eq!(impact.confidence, "directional");
    assert_eq!(
        impact.summary,
        "Low impact: obvious bundle issues are limited in the current scan."
    );

    let threshold_impact = estimate_impact(&[heavy_dependency(222)], &[], &[], &[]);
    let js_threshold_impact = load_js_impact(&impact_payload(&[222], &[], &[], &[]));

    assert_eq!(threshold_impact, js_threshold_impact);
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
            0,
            "low",
            "Low impact: obvious bundle issues are limited in the current scan.",
        ),
        (
            39,
            "directional",
            "Low impact: obvious bundle issues are limited in the current scan.",
        ),
        (
            40,
            "directional",
            "Targeted impact: a handful of focused optimizations should pay off.",
        ),
        (
            120,
            "directional",
            "Medium impact: there are several meaningful bundle wins available.",
        ),
        (
            300,
            "directional",
            "High impact: the project has clear opportunities to reduce initial payload size.",
        ),
    ];

    for (potential_kb_saved, expected_confidence, expected_summary) in cases {
        let duplicate_packages = if potential_kb_saved == 0 {
            Vec::new()
        } else {
            vec![duplicate_package(potential_kb_saved)]
        };

        let impact = estimate_impact(&[], &duplicate_packages, &[], &[]);
        let js_impact = load_js_impact(&impact_payload(
            &[],
            &duplicate_packages
                .iter()
                .map(|item| item.estimated_extra_kb)
                .collect::<Vec<_>>(),
            &[],
            &[],
        ));

        assert_eq!(impact, js_impact);
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

fn snapshot(intel: PackageIntel) -> JsPackageIntel {
    JsPackageIntel {
        estimated_kb: intel.estimated_kb,
        category: intel.category.to_string(),
        rationale: intel.rationale.to_string(),
        recommendation: intel.recommendation.to_string(),
    }
}

fn load_js_package_intelligence() -> Vec<(String, JsPackageIntel)> {
    let repo_root = workspace_root();
    let script = r#"
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";

const repoRoot = process.argv[1];
const sourcePath = path.join(repoRoot, "src/core/package-intelligence.js");
const source = fs.readFileSync(sourcePath, "utf8").replace(
  "export function getPackageIntel",
  "function getPackageIntel"
);
const context = {};

vm.runInNewContext(
  `${source}\nresult = Object.entries(PACKAGE_INTELLIGENCE);`,
  context
);

console.log(JSON.stringify(context.result));
"#;

    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .arg(repo_root.display().to_string())
        .current_dir(&repo_root)
        .output()
        .expect("run node for JS package intelligence");

    assert!(
        output.status.success(),
        "node exited unsuccessfully: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("deserialize JS package intelligence")
}

fn load_js_impact(payload: &serde_json::Value) -> Impact {
    let repo_root = workspace_root();
    let payload_json = serde_json::to_string(payload).expect("serialize JS impact payload");
    let script = r#"
import path from "node:path";
import { pathToFileURL } from "node:url";

const repoRoot = process.argv[1];
const payload = JSON.parse(process.argv[2]);
const moduleUrl = pathToFileURL(path.join(repoRoot, "src/core/estimate-impact.js")).href;
const { estimateImpact } = await import(moduleUrl);

console.log(JSON.stringify(estimateImpact(payload)));
"#;

    let output = Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script)
        .arg(repo_root.display().to_string())
        .arg(payload_json)
        .current_dir(&repo_root)
        .output()
        .expect("run node for JS impact");

    assert!(
        output.status.success(),
        "node exited unsuccessfully: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("deserialize JS impact")
}

fn impact_payload(
    heavy_dependencies: &[usize],
    duplicate_packages: &[usize],
    lazy_load_candidates: &[usize],
    tree_shaking_warnings: &[usize],
) -> serde_json::Value {
    json!({
        "heavyDependencies": heavy_dependencies
            .iter()
            .map(|estimated_kb| json!({ "estimatedKb": estimated_kb }))
            .collect::<Vec<_>>(),
        "duplicatePackages": duplicate_packages
            .iter()
            .map(|estimated_extra_kb| json!({ "estimatedExtraKb": estimated_extra_kb }))
            .collect::<Vec<_>>(),
        "lazyLoadCandidates": lazy_load_candidates
            .iter()
            .map(|estimated_savings_kb| json!({ "estimatedSavingsKb": estimated_savings_kb }))
            .collect::<Vec<_>>(),
        "treeShakingWarnings": tree_shaking_warnings
            .iter()
            .map(|estimated_kb| json!({ "estimatedKb": estimated_kb }))
            .collect::<Vec<_>>(),
    })
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}
