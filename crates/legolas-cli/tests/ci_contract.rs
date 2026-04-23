mod support;

use std::{fs, path::Path};

use assert_cmd::Command;
use legolas_core::{analyze_project, diff_analysis, BaselineSnapshot};
use serde_json::json;
use tempfile::TempDir;

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(args)
        .output()
        .expect("run command")
}

fn run_cli_in_dir(current_dir: &Path, args: &[&str]) -> std::process::Output {
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

fn normalize(value: &str) -> String {
    value.replace('\\', "/")
}

fn triggered_finding(
    finding_id: &str,
    analysis_source: &str,
    confidence: &str,
) -> serde_json::Value {
    json!({
        "findingId": finding_id,
        "analysisSource": analysis_source,
        "confidence": confidence
    })
}

fn potential_kb_saved_findings() -> Vec<serde_json::Value> {
    vec![
        triggered_finding("heavy-dependency:chart.js", "source-import", "high"),
        triggered_finding("heavy-dependency:react-icons", "source-import", "high"),
        triggered_finding("heavy-dependency:lodash", "source-import", "high"),
        triggered_finding("duplicate-package:lodash", "lockfile-trace", "high"),
        triggered_finding("lazy-load:chart.js", "heuristic", "low"),
        triggered_finding("lazy-load:react-icons", "heuristic", "low"),
        triggered_finding("lazy-load:lodash", "heuristic", "low"),
        triggered_finding("tree-shaking:lodash-root-import", "source-import", "high"),
        triggered_finding(
            "tree-shaking:react-icons-root-import",
            "source-import",
            "high",
        ),
    ]
}

fn dynamic_import_findings() -> Vec<serde_json::Value> {
    vec![
        triggered_finding("lazy-load:chart.js", "heuristic", "low"),
        triggered_finding("lazy-load:react-icons", "heuristic", "low"),
        triggered_finding("lazy-load:lodash", "heuristic", "low"),
    ]
}

fn workspace_summaries() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "admin-app",
            "path": "apps/admin",
            "importedPackages": 3,
            "heavyDependencies": 2,
            "duplicatePackages": 0,
            "potentialKbSaved": 42
        }),
        json!({
            "name": "storefront-app",
            "path": "apps/storefront",
            "importedPackages": 2,
            "heavyDependencies": 1,
            "duplicatePackages": 0,
            "potentialKbSaved": 13
        }),
    ]
}

fn regression_baseline_diff() -> serde_json::Value {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let baseline_path = support::fixture_path("tests/fixtures/baseline/previous-scan.json");
    let baseline: BaselineSnapshot =
        serde_json::from_str(&fs::read_to_string(baseline_path).expect("read baseline fixture"))
            .expect("parse baseline fixture");
    let analysis = analyze_project(&current_app).expect("analyze current regression fixture");

    serde_json::to_value(diff_analysis(&baseline, &analysis)).expect("serialize baseline diff")
}

#[test]
fn ci_fail_returns_exit_code_one_and_fixed_failure_prefix() {
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = run_cli(&["ci", &basic_app.display().to_string()]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stdout(&output),
        "\
Legolas CI for basic-parity-app

Gate result: FAIL
Overall status: Fail
Rule statuses: potentialKbSaved=Fail, duplicatePackageCount=Pass, dynamicImportCount=Fail
"
    );
    assert_eq!(
        stderr(&output),
        "CI gate failed: overall status Fail (failing rules: potentialKbSaved, dynamicImportCount)\n"
    );
}

#[test]
fn ci_warn_keeps_exit_code_zero() {
    let project = dynamic_import_project("ci-warn-app", 1);
    let output = run_cli_in_dir(project.path(), &["ci"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "\
Legolas CI for ci-warn-app

Gate result: WARN
Overall status: Warn
Rule statuses: potentialKbSaved=Pass, duplicatePackageCount=Pass, dynamicImportCount=Warn
"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn ci_pass_keeps_exit_code_zero() {
    let project = dynamic_import_project("ci-pass-app", 2);
    let output = run_cli_in_dir(project.path(), &["ci"]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "\
Legolas CI for ci-pass-app

Gate result: PASS
Overall status: Pass
Rule statuses: potentialKbSaved=Pass, duplicatePackageCount=Pass, dynamicImportCount=Pass
"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn ci_json_output_uses_machine_readable_gate_shape() {
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = run_cli(&["ci", &basic_app.display().to_string(), "--json"]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        support::normalize_ci_json_output(&stdout(&output)),
        json!({
            "schemaVersion": "legolas.ci.v1",
            "passed": false,
            "overallStatus": "Fail",
            "rules": [
                {
                    "key": "potentialKbSaved",
                    "actual": 366,
                    "warnAt": 40,
                    "failAt": 80,
                    "status": "Fail",
                    "triggeredFindings": potential_kb_saved_findings()
                },
                {
                    "key": "duplicatePackageCount",
                    "actual": 1,
                    "warnAt": 2,
                    "failAt": 4,
                    "status": "Pass",
                    "triggeredFindings": []
                },
                {
                    "key": "dynamicImportCount",
                    "actual": 0,
                    "warnAt": 1,
                    "failAt": 0,
                    "status": "Fail",
                    "triggeredFindings": dynamic_import_findings()
                }
            ]
        })
    );
    assert_eq!(
        stderr(&output),
        "CI gate failed: overall status Fail (failing rules: potentialKbSaved, dynamicImportCount)\n"
    );
}

#[test]
fn regression_only_ci_json_includes_regression_envelope() {
    let current_app = support::fixture_path("tests/fixtures/baseline/current-app");
    let baseline_path = support::fixture_path("tests/fixtures/baseline/previous-scan.json");
    let output = run_cli(&[
        "ci",
        &current_app.display().to_string(),
        "--baseline",
        &baseline_path.display().to_string(),
        "--regression-only",
        "--json",
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");

    let ci = support::normalize_ci_json_output(&stdout(&output));
    assert_eq!(ci["passed"], json!(true));
    assert_eq!(ci["overallStatus"], json!("Warn"));
    assert_eq!(
        ci["regression"],
        json!({
            "mode": "regression-only",
            "baselineDiff": regression_baseline_diff()
        })
    );
}

#[test]
fn ci_json_output_includes_workspace_summaries_for_monorepos() {
    let workspace = support::fixture_path("tests/fixtures/monorepo/pnpm-workspace");
    let output = run_cli(&["ci", &workspace.display().to_string(), "--json"]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    let ci = support::normalize_ci_json_output(&stdout(&output));
    assert_eq!(ci["workspaceSummaries"], json!(workspace_summaries()));
    assert_eq!(ci["overallStatus"], json!("Fail"));
    assert_eq!(ci["passed"], json!(false));
}

#[test]
fn ci_rejects_command_specific_numeric_flags() {
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    let cases = [
        (
            vec![
                "ci".to_string(),
                basic_app.display().to_string(),
                "--limit".to_string(),
                "1".to_string(),
            ],
            "legolas: unknown flag \"--limit\"\n",
        ),
        (
            vec![
                "ci".to_string(),
                basic_app.display().to_string(),
                "--top".to_string(),
                "1".to_string(),
            ],
            "legolas: unknown flag \"--top\"\n",
        ),
        (
            vec![
                "--limit".to_string(),
                "-1".to_string(),
                "ci".to_string(),
                basic_app.display().to_string(),
            ],
            "legolas: unknown flag \"--limit\"\n",
        ),
    ];

    for (args, expected_stderr) in cases {
        let output = Command::cargo_bin("legolas-cli")
            .expect("build binary")
            .args(args)
            .output()
            .expect("run invalid ci command");

        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stdout(&output), "");
        assert_eq!(stderr(&output), expected_stderr);
    }
}

#[test]
fn malformed_config_fails_before_ci_gate_policy() {
    let fixture = support::fixture_path("tests/fixtures/config/invalid-json");
    let output = run_cli(&["ci", &fixture.display().to_string()]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(normalize(&stderr(&output)).starts_with(&format!(
        "legolas: malformed config {}/tests/fixtures/config/invalid-json/legolas.config.json:",
        normalize(&support::workspace_root().display().to_string())
    )));
}

#[test]
fn ci_suppresses_config_warnings_to_keep_failure_prefix_stable() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("legolas.config.json");
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    fs::write(
        &config_path,
        format!(
            r#"{{
  "scan": {{ "path": "{}" }},
  "budget": {{
    "rules": {{
      "potentialKbSaved": {{ "warnAt": 40, "failAt": 80, "note": "ignored" }}
    }}
  }},
  "extra": true
}}
"#,
            normalize(&basic_app.display().to_string())
        ),
    )
    .expect("write config");

    let output = run_cli(&["ci", "--config", &config_path.display().to_string()]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stdout(&output),
        "\
Legolas CI for basic-parity-app

Gate result: FAIL
Overall status: Fail
Rule statuses: potentialKbSaved=Fail, duplicatePackageCount=Pass, dynamicImportCount=Fail
"
    );
    assert_eq!(
        stderr(&output),
        "CI gate failed: overall status Fail (failing rules: potentialKbSaved, dynamicImportCount)\n"
    );
}

#[test]
fn ci_text_output_includes_workspace_summaries_for_monorepos() {
    let workspace = support::fixture_path("tests/fixtures/monorepo/pnpm-workspace");
    let output = run_cli(&["ci", &workspace.display().to_string()]);

    assert!(!output.status.success());
    let stdout = stdout(&output);
    assert!(stdout.contains("Workspace summaries:"));
    assert!(stdout.contains("admin-app (apps/admin): 3 imported packages, 2 heavy dependencies, 0 duplicate packages, ~42 KB potential saved"));
    assert!(stdout.contains("storefront-app (apps/storefront): 2 imported packages, 1 heavy dependencies, 0 duplicate packages, ~13 KB potential saved"));
}

fn dynamic_import_project(name: &str, dynamic_imports: usize) -> TempDir {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).expect("create src dir");

    let dependency_entries = (0..dynamic_imports)
        .map(|entry| match entry {
            0 => "\"left-pad\":\"^1.3.0\"".to_string(),
            1 => "\"is-odd\":\"^3.0.1\"".to_string(),
            other => format!("\"ci-dyn-{other}\":\"^1.0.0\""),
        })
        .collect::<Vec<_>>()
        .join(",");
    fs::write(
        temp_dir.path().join("package.json"),
        format!(r#"{{"name":"{name}","dependencies":{{{dependency_entries}}}}}"#),
    )
    .expect("write package.json");

    let mut index = String::new();
    for entry in 0..dynamic_imports {
        let package_name = match entry {
            0 => "left-pad".to_string(),
            1 => "is-odd".to_string(),
            other => format!("ci-dyn-{other}"),
        };
        index.push_str(&format!("void import(\"{package_name}\");\n"));
    }
    fs::write(src_dir.join("index.js"), index).expect("write entry module");

    temp_dir
}
