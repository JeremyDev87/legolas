mod support;

use std::collections::BTreeSet;

use assert_cmd::Command;
use serde_json::json;

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(args)
        .output()
        .expect("run command")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr")
}

#[test]
fn scan_sarif_preserves_rule_ids_locations_and_metadata() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = run_cli(&["scan", &fixture.display().to_string(), "--sarif"]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let sarif = support::normalize_sarif_output(&stdout(&output));
    assert_eq!(
        sarif["$schema"],
        json!("https://json.schemastore.org/sarif-2.1.0.json")
    );
    assert_eq!(sarif["version"], json!("2.1.0"));
    assert_eq!(sarif["runs"][0]["properties"]["command"], json!("scan"));

    let results = sarif["runs"][0]["results"]
        .as_array()
        .expect("results array");
    assert_eq!(results.len(), 9);

    let rule_ids_from_results = results
        .iter()
        .map(|result| result["ruleId"].as_str().expect("rule id").to_string())
        .collect::<BTreeSet<_>>();
    let rule_ids_from_driver = sarif["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .expect("driver rules")
        .iter()
        .map(|rule| rule["id"].as_str().expect("rule id").to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(rule_ids_from_results, rule_ids_from_driver);

    let chart_js = results
        .iter()
        .find(|result| result["ruleId"] == "heavy-dependency:chart.js")
        .expect("chart.js result");
    assert_eq!(chart_js["level"], json!("warning"));
    assert_eq!(
        chart_js["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        json!("src/Dashboard.tsx")
    );
    assert_eq!(
        chart_js["properties"]["analysisSource"],
        json!("source-import")
    );
    assert_eq!(chart_js["properties"]["confidence"], json!("high"));
    assert_eq!(chart_js["properties"]["actionPriority"], json!(1));
    assert_eq!(
        chart_js["properties"]["recommendedFix"]["kind"],
        json!("lazy-load")
    );
    assert_eq!(
        chart_js["properties"]["evidence"][0],
        json!({
            "kind": "source-file",
            "file": "src/Dashboard.tsx",
            "specifier": "chart.js",
            "detail": "static import; Charting code is often only needed on a subset of screens."
        })
    );

    let duplicate = results
        .iter()
        .find(|result| result["ruleId"] == "duplicate-package:lodash")
        .expect("duplicate result");
    assert!(duplicate.get("locations").is_none());
    assert_eq!(
        duplicate["properties"]["analysisSource"],
        json!("lockfile-trace")
    );
    assert_eq!(duplicate["properties"]["confidence"], json!("high"));
    assert_eq!(duplicate["properties"]["evidence"], json!([]));
    assert_eq!(
        duplicate["properties"]["recommendedFix"]["kind"],
        json!("dedupe-package")
    );
}

#[test]
fn ci_sarif_uses_triggered_findings_and_preserves_failure_exit_code() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = run_cli(&["ci", &fixture.display().to_string(), "--sarif"]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "CI gate failed: overall status Fail (failing rules: potentialKbSaved, dynamicImportCount)\n"
    );

    let sarif = support::normalize_sarif_output(&stdout(&output));
    assert_eq!(sarif["runs"][0]["properties"]["command"], json!("ci"));
    assert_eq!(sarif["runs"][0]["properties"]["passed"], json!(false));
    assert_eq!(
        sarif["runs"][0]["properties"]["overallStatus"],
        json!("Fail")
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["rules"][0]["key"],
        json!("potentialKbSaved")
    );

    let rule_ids = sarif["runs"][0]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|result| result["ruleId"].as_str().expect("rule id").to_string())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        rule_ids,
        BTreeSet::from([
            "duplicate-package:lodash".to_string(),
            "heavy-dependency:chart.js".to_string(),
            "heavy-dependency:lodash".to_string(),
            "heavy-dependency:react-icons".to_string(),
            "lazy-load:chart.js".to_string(),
            "lazy-load:lodash".to_string(),
            "lazy-load:react-icons".to_string(),
            "tree-shaking:lodash-root-import".to_string(),
            "tree-shaking:react-icons-root-import".to_string(),
        ])
    );
}

#[test]
fn ci_sarif_excludes_non_budget_findings() {
    let fixture = support::fixture_path("tests/fixtures/boundaries/rsc-server-only");

    let scan_output = run_cli(&["scan", &fixture.display().to_string(), "--sarif"]);
    assert!(scan_output.status.success());
    let scan = support::normalize_sarif_output(&stdout(&scan_output));
    let scan_results = scan["runs"][0]["results"].as_array().expect("scan results");
    assert_eq!(scan_results.len(), 1);
    assert_eq!(scan_results[0]["ruleId"], json!("boundary:rsc-server-only"));

    let ci_output = run_cli(&["ci", &fixture.display().to_string(), "--sarif"]);
    assert!(!ci_output.status.success());
    assert_eq!(ci_output.status.code(), Some(1));
    assert_eq!(
        stderr(&ci_output),
        "CI gate failed: overall status Fail (failing rules: dynamicImportCount)\n"
    );
    let ci = support::normalize_sarif_output(&stdout(&ci_output));
    assert_eq!(ci["runs"][0]["results"], json!([]));
    assert_eq!(ci["runs"][0]["properties"]["passed"], json!(false));
    assert_eq!(ci["runs"][0]["properties"]["overallStatus"], json!("Fail"));
    assert_eq!(
        ci["runs"][0]["properties"]["rules"][2]["key"],
        json!("dynamicImportCount")
    );
}

#[test]
fn scan_sarif_exposes_analysis_warnings_even_without_findings() {
    let fixture = support::fixture_path("tests/fixtures/workspace/multi-lockfiles");
    let output = run_cli(&["scan", &fixture.display().to_string(), "--sarif"]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let sarif = support::normalize_sarif_output(&stdout(&output));
    assert_eq!(sarif["runs"][0]["results"], json!([]));
    assert_eq!(
        sarif["runs"][0]["properties"]["warnings"],
        json!([
            "Multiple lockfiles detected. Duplicate analysis used pnpm-lock.yaml based on package manager \"pnpm@9.0.0\" and ignored package-lock.json."
        ])
    );
}

#[test]
fn regression_only_ci_sarif_includes_regression_envelope() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let baseline_path = support::fixture_path("tests/fixtures/baseline/previous-scan.json");
    let output = run_cli(&[
        "ci",
        &current_app.display().to_string(),
        "--baseline",
        &baseline_path.display().to_string(),
        "--regression-only",
        "--sarif",
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let sarif = support::normalize_sarif_output(&stdout(&output));
    assert_eq!(sarif["runs"][0]["properties"]["passed"], json!(true));
    assert_eq!(
        sarif["runs"][0]["properties"]["overallStatus"],
        json!("Warn")
    );
    assert_eq!(
        sarif["runs"][0]["properties"]["regression"]["mode"],
        json!("regression-only")
    );
    assert!(
        sarif["runs"][0]["properties"]["regression"]["baselineDiff"]["schemaVersion"].is_number()
    );
}
