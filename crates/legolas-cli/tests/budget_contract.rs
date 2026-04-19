mod support;

use std::{fs, path::Path};

use assert_cmd::Command;
use serde_json::json;
use tempfile::tempdir;

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

#[test]
fn budget_text_output_uses_built_in_starter_thresholds() {
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = run_cli(&["budget", &basic_app.display().to_string()]);

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "\
Legolas budget for basic-parity-app

Overall status: Fail

Rule results:
- potentialKbSaved: Fail (actual: 366, warnAt: 40, failAt: 80)
- duplicatePackageCount: Pass (actual: 1, warnAt: 2, failAt: 4)
- dynamicImportCount: Fail (actual: 0, warnAt: 1, failAt: 0)
"
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn budget_json_output_has_a_stable_shape() {
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = run_cli(&["budget", &basic_app.display().to_string(), "--json"]);

    assert!(output.status.success());
    assert_eq!(
        support::normalize_budget_json_output(&stdout(&output)),
        json!({
            "overallStatus": "Fail",
            "rules": [
                {
                    "key": "potentialKbSaved",
                    "actual": 366,
                    "warnAt": 40,
                    "failAt": 80,
                    "status": "Fail"
                },
                {
                    "key": "duplicatePackageCount",
                    "actual": 1,
                    "warnAt": 2,
                    "failAt": 4,
                    "status": "Pass"
                },
                {
                    "key": "dynamicImportCount",
                    "actual": 0,
                    "warnAt": 1,
                    "failAt": 0,
                    "status": "Fail"
                }
            ]
        })
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn budget_uses_config_threshold_overrides_and_starter_fallbacks_together() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("legolas.config.json");
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    fs::write(
        &config_path,
        format!(
            r#"{{
  "scan": {{ "path": "{}" }},
  "budget": {{
    "rules": {{
      "potentialKbSaved": {{ "warnAt": 400, "failAt": 500 }},
      "duplicatePackageCount": {{ "warnAt": 1, "failAt": 2 }}
    }}
  }}
}}
"#,
            normalize(&basic_app.display().to_string())
        ),
    )
    .expect("write config");

    let output = run_cli(&[
        "budget",
        "--config",
        &config_path.display().to_string(),
        "--json",
    ]);

    assert!(output.status.success());
    assert_eq!(
        support::normalize_budget_json_output(&stdout(&output)),
        json!({
            "overallStatus": "Fail",
            "rules": [
                {
                    "key": "potentialKbSaved",
                    "actual": 366,
                    "warnAt": 400,
                    "failAt": 500,
                    "status": "Pass"
                },
                {
                    "key": "duplicatePackageCount",
                    "actual": 1,
                    "warnAt": 1,
                    "failAt": 2,
                    "status": "Warn"
                },
                {
                    "key": "dynamicImportCount",
                    "actual": 0,
                    "warnAt": 1,
                    "failAt": 0,
                    "status": "Fail"
                }
            ]
        })
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn budget_uses_discovered_config_from_project_root() {
    let temp_dir = tempdir().expect("create temp dir");
    let project_root = temp_dir.path();
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    fs::write(
        project_root.join("package.json"),
        "{\"name\":\"budget-discovered\"}",
    )
    .expect("write package.json");
    fs::write(
        project_root.join("legolas.config.json"),
        format!(
            r#"{{
  "scan": {{ "path": "{}" }},
  "budget": {{
    "rules": {{
      "potentialKbSaved": {{ "warnAt": 40, "failAt": 80 }},
      "duplicatePackageCount": {{ "warnAt": 2, "failAt": 4 }},
      "dynamicImportCount": {{ "warnAt": 1, "failAt": 0 }}
    }}
  }}
}}
"#,
            normalize(&basic_app.display().to_string())
        ),
    )
    .expect("write config");
    let output = run_cli_in_dir(project_root, &["budget", "--json"]);

    assert!(output.status.success());
    assert_eq!(
        support::normalize_budget_json_output(&stdout(&output)),
        json!({
            "overallStatus": "Fail",
            "rules": [
                {
                    "key": "potentialKbSaved",
                    "actual": 366,
                    "warnAt": 40,
                    "failAt": 80,
                    "status": "Fail"
                },
                {
                    "key": "duplicatePackageCount",
                    "actual": 1,
                    "warnAt": 2,
                    "failAt": 4,
                    "status": "Pass"
                },
                {
                    "key": "dynamicImportCount",
                    "actual": 0,
                    "warnAt": 1,
                    "failAt": 0,
                    "status": "Fail"
                }
            ]
        })
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn budget_rejects_command_specific_numeric_flags() {
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    let cases = [
        (
            vec![
                "budget".to_string(),
                basic_app.display().to_string(),
                "--limit".to_string(),
                "1".to_string(),
            ],
            "legolas: unknown flag \"--limit\"\n",
        ),
        (
            vec![
                "budget".to_string(),
                basic_app.display().to_string(),
                "--top".to_string(),
                "1".to_string(),
            ],
            "legolas: unknown flag \"--top\"\n",
        ),
        (
            vec![
                "budget".to_string(),
                basic_app.display().to_string(),
                "--top".to_string(),
                "NaN".to_string(),
            ],
            "legolas: unknown flag \"--top\"\n",
        ),
        (
            vec![
                "budget".to_string(),
                basic_app.display().to_string(),
                "--limit".to_string(),
                "-1".to_string(),
            ],
            "legolas: unknown flag \"--limit\"\n",
        ),
        (
            vec![
                "--top".to_string(),
                "NaN".to_string(),
                "budget".to_string(),
                basic_app.display().to_string(),
            ],
            "legolas: unknown flag \"--top\"\n",
        ),
        (
            vec![
                "--limit".to_string(),
                "-1".to_string(),
                "budget".to_string(),
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
            .expect("run invalid budget command");

        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(stdout(&output), "");
        assert_eq!(stderr(&output), expected_stderr);
    }
}
