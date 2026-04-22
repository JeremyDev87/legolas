mod support;

use std::{fs, path::Path};

use assert_cmd::Command;
use legolas_core::{analyze_project, BaselineSnapshot};
use tempfile::tempdir;

fn run_cli(args: Vec<String>) -> std::process::Output {
    Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(args)
        .output()
        .expect("run command")
}

fn run_cli_in_dir(current_dir: &Path, args: Vec<String>) -> std::process::Output {
    Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .current_dir(current_dir)
        .args(args)
        .output()
        .expect("run command in directory")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr")
}

fn baseline_fixture_path() -> String {
    support::fixture_path("tests/fixtures/baseline/previous-scan.json")
        .display()
        .to_string()
}

fn write_temp_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, contents).expect("write temp file");
}

fn write_temp_baseline(path: &Path, project_root: &Path) {
    let analysis = analyze_project(project_root).expect("analyze project for baseline");
    let snapshot = BaselineSnapshot::from_analysis(&analysis);
    write_temp_snapshot(path, &snapshot);
}

fn write_temp_snapshot(path: &Path, snapshot: &BaselineSnapshot) {
    fs::write(
        path,
        serde_json::to_string_pretty(snapshot).expect("serialize baseline snapshot"),
    )
    .expect("write baseline snapshot");
}

fn setup_dynamic_import_regression_fixture(
) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let temp_dir = tempdir().expect("create temp dir");
    let project_root = temp_dir.path().join("app");
    let baseline_path = project_root.join("baseline.json");

    write_temp_file(
        &project_root.join("package.json"),
        r#"{
  "name": "dynamic-import-regression-app",
  "dependencies": {
    "chart.js": "^4.4.1"
  }
}"#,
    );
    write_temp_file(
        &project_root.join("src/routes/Dashboard.tsx"),
        "import { Chart } from \"chart.js\";\nexport default function Dashboard() { return Chart; }\n",
    );
    write_temp_file(
        &project_root.join("src/load.ts"),
        "export const loadDashboard = () => import(\"./routes/Dashboard\");\n",
    );

    let analysis = analyze_project(&project_root).expect("analyze current fixture");
    assert_eq!(analysis.source_summary.dynamic_imports, 1);
    assert_eq!(analysis.impact.potential_kb_saved, 157);

    let mut baseline = BaselineSnapshot::from_analysis(&analysis);
    baseline.dynamic_import_count = 2;
    write_temp_snapshot(&baseline_path, &baseline);

    (temp_dir, project_root, baseline_path)
}

fn setup_dynamic_import_count_only_regression_fixture(
) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let temp_dir = tempdir().expect("create temp dir");
    let project_root = temp_dir.path().join("app");
    let baseline_path = project_root.join("baseline.json");

    write_temp_file(
        &project_root.join("package.json"),
        r#"{
  "name": "dynamic-import-count-only-regression-app",
  "dependencies": {}
}"#,
    );
    write_temp_file(
        &project_root.join("src/load.ts"),
        "export const loadSettings = () => import(\"./Settings\");\n",
    );
    write_temp_file(
        &project_root.join("src/Settings.ts"),
        "export const Settings = null;\n",
    );

    let baseline = BaselineSnapshot::from_analysis(
        &analyze_project(&project_root).expect("analyze baseline fixture"),
    );
    assert_eq!(baseline.dynamic_import_count, 1);
    write_temp_snapshot(&baseline_path, &baseline);

    write_temp_file(
        &project_root.join("src/load.ts"),
        "export const loadSettings = () => null;\n",
    );

    let current = analyze_project(&project_root).expect("analyze current fixture");
    assert_eq!(current.source_summary.dynamic_imports, 0);
    assert!(current.lazy_load_candidates.is_empty());

    (temp_dir, project_root, baseline_path)
}

#[test]
fn regression_only_scan_json_filters_to_new_findings() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let output = run_cli(vec![
        "scan".to_string(),
        current_app.display().to_string(),
        "--baseline".to_string(),
        baseline_fixture_path(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let analysis = support::normalize_analysis_json_output(&stdout(&output));
    assert_eq!(analysis["packageSummary"]["name"], "baseline-app");
    assert_eq!(analysis["heavyDependencies"].as_array().unwrap().len(), 2);
    assert_eq!(analysis["heavyDependencies"][0]["name"], "chart.js");
    assert_eq!(analysis["heavyDependencies"][1]["name"], "lodash");
    assert_eq!(analysis["treeShakingWarnings"].as_array().unwrap().len(), 1);
    assert_eq!(
        analysis["treeShakingWarnings"][0]["key"],
        "lodash-root-import"
    );
    assert_eq!(analysis["duplicatePackages"], serde_json::json!([]));
}

#[test]
fn regression_only_scan_text_only_mentions_new_findings() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let output = run_cli(vec![
        "scan".to_string(),
        current_app.display().to_string(),
        "--baseline".to_string(),
        baseline_fixture_path(),
        "--regression-only".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let stdout = stdout(&output);
    assert!(stdout.contains("Legolas scan for baseline-app"));
    assert!(stdout.contains("- chart.js"));
    assert!(stdout.contains("- lodash"));
    assert!(stdout.contains("Tree-shaking warnings:"));
    assert!(stdout
        .contains("Root lodash imports often keep more code than expected in client bundles."));
}

#[test]
fn regression_only_budget_json_filters_pass_rules() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let output = run_cli(vec![
        "budget".to_string(),
        current_app.display().to_string(),
        "--baseline".to_string(),
        baseline_fixture_path(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let budget = support::normalize_budget_json_output(&stdout(&output));
    assert_eq!(budget["overallStatus"], serde_json::json!("Warn"));
    assert_eq!(budget["rules"].as_array().unwrap().len(), 1);
    assert_eq!(
        budget["rules"][0]["key"],
        serde_json::json!("potentialKbSaved")
    );
    assert_eq!(budget["rules"][0]["status"], serde_json::json!("Warn"));
    assert_eq!(
        budget["rules"][0]["triggeredFindings"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn regression_only_ci_json_filters_pass_rules() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let output = run_cli(vec![
        "ci".to_string(),
        current_app.display().to_string(),
        "--baseline".to_string(),
        baseline_fixture_path(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let ci = support::normalize_ci_json_output(&stdout(&output));
    assert_eq!(ci["passed"], serde_json::json!(true));
    assert_eq!(ci["overallStatus"], serde_json::json!("Warn"));
    assert_eq!(ci["rules"].as_array().unwrap().len(), 1);
    assert_eq!(ci["rules"][0]["key"], serde_json::json!("potentialKbSaved"));
    assert_eq!(ci["rules"][0]["status"], serde_json::json!("Warn"));
}

#[test]
fn regression_only_budget_json_drops_potential_kb_failures_for_dynamic_import_only_regressions() {
    let (_temp_dir, project_root, baseline_path) = setup_dynamic_import_regression_fixture();
    let output = run_cli(vec![
        "budget".to_string(),
        project_root.display().to_string(),
        "--baseline".to_string(),
        baseline_path.display().to_string(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let budget = support::normalize_budget_json_output(&stdout(&output));
    assert_eq!(budget["overallStatus"], serde_json::json!("Warn"));
    assert_eq!(budget["rules"].as_array().unwrap().len(), 1);
    assert_eq!(
        budget["rules"][0]["key"],
        serde_json::json!("dynamicImportCount")
    );
    assert_eq!(budget["rules"][0]["status"], serde_json::json!("Warn"));
}

#[test]
fn regression_only_ci_json_drops_potential_kb_failures_for_dynamic_import_only_regressions() {
    let (_temp_dir, project_root, baseline_path) = setup_dynamic_import_regression_fixture();
    let output = run_cli(vec![
        "ci".to_string(),
        project_root.display().to_string(),
        "--baseline".to_string(),
        baseline_path.display().to_string(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let ci = support::normalize_ci_json_output(&stdout(&output));
    assert_eq!(ci["passed"], serde_json::json!(true));
    assert_eq!(ci["overallStatus"], serde_json::json!("Warn"));
    assert_eq!(ci["rules"].as_array().unwrap().len(), 1);
    assert_eq!(
        ci["rules"][0]["key"],
        serde_json::json!("dynamicImportCount")
    );
    assert_eq!(ci["rules"][0]["status"], serde_json::json!("Warn"));
}

#[test]
fn regression_only_budget_json_keeps_dynamic_import_failures_without_lazy_candidates() {
    let (_temp_dir, project_root, baseline_path) =
        setup_dynamic_import_count_only_regression_fixture();
    let output = run_cli(vec![
        "budget".to_string(),
        project_root.display().to_string(),
        "--baseline".to_string(),
        baseline_path.display().to_string(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let budget = support::normalize_budget_json_output(&stdout(&output));
    assert_eq!(budget["overallStatus"], serde_json::json!("Fail"));
    assert_eq!(budget["rules"].as_array().unwrap().len(), 1);
    assert_eq!(
        budget["rules"][0]["key"],
        serde_json::json!("dynamicImportCount")
    );
    assert_eq!(budget["rules"][0]["status"], serde_json::json!("Fail"));
    assert_eq!(
        budget["rules"][0]["triggeredFindings"],
        serde_json::json!([])
    );
}

#[test]
fn regression_only_ci_json_keeps_dynamic_import_failures_without_lazy_candidates() {
    let (_temp_dir, project_root, baseline_path) =
        setup_dynamic_import_count_only_regression_fixture();
    let output = run_cli(vec![
        "ci".to_string(),
        project_root.display().to_string(),
        "--baseline".to_string(),
        baseline_path.display().to_string(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("CI gate failed: overall status Fail"));

    let ci = support::normalize_ci_json_output(&stdout(&output));
    assert_eq!(ci["passed"], serde_json::json!(false));
    assert_eq!(ci["overallStatus"], serde_json::json!("Fail"));
    assert_eq!(ci["rules"].as_array().unwrap().len(), 1);
    assert_eq!(
        ci["rules"][0]["key"],
        serde_json::json!("dynamicImportCount")
    );
    assert_eq!(ci["rules"][0]["status"], serde_json::json!("Fail"));
    assert_eq!(ci["rules"][0]["triggeredFindings"], serde_json::json!([]));
}

#[test]
fn write_baseline_persists_the_current_snapshot() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let temp_dir = tempdir().expect("create temp dir");
    let baseline_path = temp_dir.path().join("baseline.json");
    let output = run_cli(vec![
        "scan".to_string(),
        current_app.display().to_string(),
        "--write-baseline".to_string(),
        baseline_path.display().to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");
    assert!(baseline_path.exists());

    let current_analysis = analyze_project(&current_app).expect("analyze current app");
    let expected_snapshot = BaselineSnapshot::from_analysis(&current_analysis);
    let written_snapshot: BaselineSnapshot =
        serde_json::from_str(&fs::read_to_string(&baseline_path).expect("read written baseline"))
            .expect("parse written baseline");

    assert_eq!(written_snapshot, expected_snapshot);
}

#[test]
fn regression_only_requires_an_explicit_baseline() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let output = run_cli(vec![
        "scan".to_string(),
        current_app.display().to_string(),
        "--regression-only".to_string(),
    ]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "legolas: --regression-only requires --baseline\n"
    );
}

#[test]
fn baseline_flag_requires_regression_only() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let output = run_cli(vec![
        "scan".to_string(),
        current_app.display().to_string(),
        "--baseline".to_string(),
        baseline_fixture_path(),
    ]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "legolas: --baseline requires --regression-only\n"
    );
}

#[test]
fn help_allows_baseline_related_flags_without_usage_errors() {
    let output = run_cli(vec!["--help".to_string(), "--regression-only".to_string()]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");
    let stdout = stdout(&output);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--write-baseline file"));
    assert!(stdout.contains("--baseline file --regression-only"));
}

#[test]
fn write_baseline_is_rejected_for_budget_commands() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let temp_dir = tempdir().expect("create temp dir");
    let baseline_path = temp_dir.path().join("budget-baseline.json");
    let output = run_cli(vec![
        "budget".to_string(),
        current_app.display().to_string(),
        "--write-baseline".to_string(),
        baseline_path.display().to_string(),
    ]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "legolas: unknown flag \"--write-baseline\"\n"
    );
    assert!(!baseline_path.exists());
}

#[test]
fn legacy_baseline_schema_requires_regeneration() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let temp_dir = tempdir().expect("create temp dir");
    let legacy_baseline = temp_dir.path().join("legacy-baseline.json");
    fs::write(
        &legacy_baseline,
        r#"{
  "schemaVersion": 1,
  "projectName": "baseline-app",
  "packageManager": "npm",
  "dependencyCount": 1,
  "devDependencyCount": 0,
  "sourceFileCount": 1,
  "heavyDependencyNames": ["chart.js"],
  "treeShakingWarningKeys": [],
  "warnings": []
}"#,
    )
    .expect("write legacy baseline");

    let output = run_cli(vec![
        "scan".to_string(),
        current_app.display().to_string(),
        "--baseline".to_string(),
        legacy_baseline.display().to_string(),
        "--regression-only".to_string(),
    ]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("unsupported baseline schema version: 1"));
    assert!(stderr(&output).contains("regenerate with --write-baseline"));
}

#[test]
fn missing_baseline_file_is_reported_explicitly() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let missing_baseline = tempdir()
        .expect("create temp dir")
        .path()
        .join("missing.json");
    let output = run_cli(vec![
        "scan".to_string(),
        current_app.display().to_string(),
        "--baseline".to_string(),
        missing_baseline.display().to_string(),
        "--regression-only".to_string(),
    ]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).starts_with("legolas: path not found: "),
        "unexpected stderr: {}",
        stderr(&output)
    );
}

#[test]
fn malformed_baseline_file_is_reported_separately() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let temp_dir = tempdir().expect("create temp dir");
    let malformed_baseline = temp_dir.path().join("baseline.json");
    fs::write(&malformed_baseline, "{not-json").expect("write malformed baseline");

    let output = run_cli(vec![
        "scan".to_string(),
        current_app.display().to_string(),
        "--baseline".to_string(),
        malformed_baseline.display().to_string(),
        "--regression-only".to_string(),
    ]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("legolas: malformed baseline "));
}

#[test]
fn regression_only_works_in_an_existing_project_directory() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let output = run_cli_in_dir(
        &current_app,
        vec![
            "scan".to_string(),
            "--baseline".to_string(),
            baseline_fixture_path(),
            "--regression-only".to_string(),
        ],
    );

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");
}

#[test]
fn regression_only_scan_json_filters_boundary_warnings_to_new_keys() {
    let temp_dir = tempdir().expect("create temp dir");
    let project_root = temp_dir.path();
    let baseline_path = project_root.join("baseline.json");

    write_temp_file(
        &project_root.join("package.json"),
        r#"{
  "name": "boundary-regression-app",
  "dependencies": {}
}"#,
    );
    write_temp_file(
        &project_root.join("src/client/existing.ts"),
        "\"use client\";\nimport \"node:fs\";\n",
    );
    write_temp_baseline(&baseline_path, project_root);
    write_temp_file(
        &project_root.join("src/client/new.ts"),
        "\"use client\";\nimport \"node:path\";\n",
    );

    let output = run_cli(vec![
        "scan".to_string(),
        project_root.display().to_string(),
        "--baseline".to_string(),
        baseline_path.display().to_string(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let analysis = support::normalize_analysis_json_output(&stdout(&output));
    assert_eq!(analysis["boundaryWarnings"].as_array().unwrap().len(), 1);
    assert!(analysis["boundaryWarnings"][0]["message"]
        .as_str()
        .unwrap()
        .contains("node:path"));
    assert!(!analysis["boundaryWarnings"][0]["message"]
        .as_str()
        .unwrap()
        .contains("node:fs"));
}

#[test]
fn regression_only_scan_json_filters_boundary_warnings_by_file_for_same_package() {
    let temp_dir = tempdir().expect("create temp dir");
    let project_root = temp_dir.path();
    let baseline_path = project_root.join("baseline.json");

    write_temp_file(
        &project_root.join("package.json"),
        r#"{
  "name": "boundary-regression-file-app",
  "dependencies": {}
}"#,
    );
    write_temp_file(
        &project_root.join("src/client/existing.ts"),
        "\"use client\";\nimport \"node:fs\";\n",
    );
    write_temp_baseline(&baseline_path, project_root);
    write_temp_file(
        &project_root.join("src/client/new.ts"),
        "\"use client\";\nimport \"node:fs\";\n",
    );

    let output = run_cli(vec![
        "scan".to_string(),
        project_root.display().to_string(),
        "--baseline".to_string(),
        baseline_path.display().to_string(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let analysis = support::normalize_analysis_json_output(&stdout(&output));
    assert_eq!(analysis["boundaryWarnings"].as_array().unwrap().len(), 1);
    assert!(analysis["boundaryWarnings"][0]["message"]
        .as_str()
        .unwrap()
        .contains("src/client/new.ts"));
    assert!(analysis["boundaryWarnings"][0]["message"]
        .as_str()
        .unwrap()
        .contains("node:fs"));
}

#[test]
fn regression_only_scan_json_filters_unused_dependencies_to_new_candidates() {
    let temp_dir = tempdir().expect("create temp dir");
    let project_root = temp_dir.path();
    let baseline_path = project_root.join("baseline.json");

    write_temp_file(
        &project_root.join("package.json"),
        r#"{
  "name": "unused-regression-app",
  "dependencies": {
    "chart.js": "^4.4.1",
    "lodash": "^4.17.21"
  }
}"#,
    );
    write_temp_file(
        &project_root.join("src/App.ts"),
        "import \"chart.js/auto\";\nexport const App = null;\n",
    );
    write_temp_baseline(&baseline_path, project_root);
    write_temp_file(
        &project_root.join("src/App.ts"),
        "export const App = null;\n",
    );

    let output = run_cli(vec![
        "scan".to_string(),
        project_root.display().to_string(),
        "--baseline".to_string(),
        baseline_path.display().to_string(),
        "--regression-only".to_string(),
        "--json".to_string(),
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let analysis = support::normalize_analysis_json_output(&stdout(&output));
    assert_eq!(
        analysis["unusedDependencyCandidates"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        analysis["unusedDependencyCandidates"][0]["name"],
        serde_json::json!("chart.js")
    );
}
