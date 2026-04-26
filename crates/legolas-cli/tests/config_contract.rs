mod support;

use std::{fs, path::Path};

use assert_cmd::Command;
use serde_json::Value;
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

fn parse_json_stdout(output: &std::process::Output) -> Value {
    serde_json::from_str(&stdout(output)).expect("parse json stdout")
}

#[test]
fn explicit_config_path_applies_relative_scan_path_and_command_defaults() {
    let config_path =
        support::fixture_path("tests/fixtures/config/explicit-path/custom.config.json");
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");

    let visualize_output = run_cli(&["visualize", "--config", &config_path.display().to_string()]);
    let visualize_expected = run_cli(&[
        "visualize",
        &basic_app.display().to_string(),
        "--limit",
        "2",
    ]);

    assert!(visualize_output.status.success());
    assert_eq!(stdout(&visualize_output), stdout(&visualize_expected));
    assert_eq!(stderr(&visualize_output), "");

    let optimize_output = run_cli(&["optimize", "--config", &config_path.display().to_string()]);
    let optimize_expected = run_cli(&["optimize", &basic_app.display().to_string(), "--top", "2"]);

    assert!(optimize_output.status.success());
    assert_eq!(stdout(&optimize_output), stdout(&optimize_expected));
    assert_eq!(stderr(&optimize_output), "");
}

#[test]
fn discovered_config_applies_defaults_from_current_project_root() {
    let project_root = support::fixture_path("tests/fixtures/config/cli-override");
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");

    let visualize_output = run_cli_in_dir(&project_root, &["visualize"]);
    let visualize_expected = run_cli(&[
        "visualize",
        &basic_app.display().to_string(),
        "--limit",
        "2",
    ]);

    assert!(visualize_output.status.success());
    assert_eq!(stdout(&visualize_output), stdout(&visualize_expected));
    assert_eq!(stderr(&visualize_output), "");

    let optimize_output = run_cli_in_dir(&project_root, &["optimize"]);
    let optimize_expected = run_cli(&["optimize", &basic_app.display().to_string(), "--top", "2"]);

    assert!(optimize_output.status.success());
    assert_eq!(stdout(&optimize_output), stdout(&optimize_expected));
    assert_eq!(stderr(&optimize_output), "");
}

#[test]
fn cli_flags_override_config_defaults() {
    let project_root = support::fixture_path("tests/fixtures/config/cli-override");
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");

    let visualize_output = run_cli_in_dir(&project_root, &["visualize", "--limit", "1"]);
    let visualize_expected = run_cli(&[
        "visualize",
        &basic_app.display().to_string(),
        "--limit",
        "1",
    ]);

    assert!(visualize_output.status.success());
    assert_eq!(stdout(&visualize_output), stdout(&visualize_expected));
    assert_eq!(stderr(&visualize_output), "");

    let optimize_output = run_cli_in_dir(&project_root, &["optimize", "--top", "1"]);
    let optimize_expected = run_cli(&["optimize", &basic_app.display().to_string(), "--top", "1"]);

    assert!(optimize_output.status.success());
    assert_eq!(stdout(&optimize_output), stdout(&optimize_expected));
    assert_eq!(stderr(&optimize_output), "");
}

#[test]
fn malformed_discovered_config_fails_with_dedicated_error() {
    let fixture = support::fixture_path("tests/fixtures/config/invalid-json");
    let output = run_cli(&["visualize", &fixture.display().to_string()]);

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(normalize(&stderr(&output)).starts_with(&format!(
        "legolas: malformed config {}/tests/fixtures/config/invalid-json/legolas.config.json:",
        normalize(&support::workspace_root().display().to_string())
    )));
}

#[test]
fn help_and_version_do_not_touch_invalid_discovered_config() {
    let invalid_root = support::fixture_path("tests/fixtures/config/invalid-json");

    let help_output = run_cli_in_dir(&invalid_root, &[]);
    assert!(help_output.status.success());
    assert_eq!(
        support::normalize_cli_output(&stdout(&help_output)),
        support::read_oracle("cli/help.txt")
    );
    assert_eq!(stderr(&help_output), "");

    let version_output = run_cli_in_dir(&invalid_root, &["--version"]);
    assert!(version_output.status.success());
    assert_eq!(
        support::normalize_cli_output(&stdout(&version_output)),
        support::expected_version_output()
    );
    assert_eq!(stderr(&version_output), "");
}

#[test]
fn config_warning_uses_stderr_without_failing_the_command() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("legolas.config.json");
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    fs::write(
        &config_path,
        format!(
            r#"{{
  "scan": {{ "path": "{}" }},
  "visualize": {{ "limit": 2, "theme": "wide" }},
  "extra": true
}}
"#,
            normalize(&basic_app.display().to_string())
        ),
    )
    .expect("write config");

    let output = run_cli(&["visualize", "--config", &config_path.display().to_string()]);
    let expected = run_cli(&[
        "visualize",
        &basic_app.display().to_string(),
        "--limit",
        "2",
    ]);

    assert!(output.status.success());
    assert_eq!(stdout(&output), stdout(&expected));
    let normalized_stderr = normalize(&stderr(&output));
    assert!(normalized_stderr.contains("legolas: config warning:"));
    assert!(normalized_stderr.contains("visualize.theme: unknown config key ignored"));
    assert!(normalized_stderr.contains("extra: unknown config key ignored"));
}

#[test]
fn json_mode_suppresses_config_warnings_to_keep_machine_output_clean() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("legolas.config.json");
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    fs::write(
        &config_path,
        format!(
            r#"{{
  "scan": {{ "path": "{}" }},
  "visualize": {{ "limit": 2, "theme": "wide" }},
  "extra": true
}}
"#,
            normalize(&basic_app.display().to_string())
        ),
    )
    .expect("write config");

    let output = run_cli(&[
        "scan",
        "--config",
        &config_path.display().to_string(),
        "--json",
    ]);
    let expected = run_cli(&["scan", &basic_app.display().to_string(), "--json"]);

    assert!(output.status.success());
    assert_eq!(
        support::normalize_analysis_json_output(&stdout(&output)),
        support::normalize_analysis_json_output(&stdout(&expected))
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn sarif_mode_suppresses_config_warnings_to_keep_machine_output_clean() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("legolas.config.json");
    let basic_app = support::fixture_path("tests/fixtures/parity/basic-app");
    fs::write(
        &config_path,
        format!(
            r#"{{
  "scan": {{ "path": "{}" }},
  "visualize": {{ "limit": 2, "theme": "wide" }},
  "extra": true
}}
"#,
            normalize(&basic_app.display().to_string())
        ),
    )
    .expect("write config");

    let output = run_cli(&[
        "scan",
        "--config",
        &config_path.display().to_string(),
        "--sarif",
    ]);
    let expected = run_cli(&["scan", &basic_app.display().to_string(), "--sarif"]);

    assert!(output.status.success());
    assert_eq!(
        support::normalize_sarif_output(&stdout(&output)),
        support::normalize_sarif_output(&stdout(&expected))
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn explicit_config_path_applies_scan_ignore_patterns_to_scan_json() {
    let temp_dir = tempdir().expect("create temp dir");
    let root = temp_dir.path();
    let config_path = root.join("legolas.config.json");
    write_ignore_config_fixture(root, &config_path);

    let output = run_cli(&[
        "scan",
        &root.display().to_string(),
        "--config",
        &config_path.display().to_string(),
        "--json",
    ]);

    assert!(output.status.success());
    assert_eq!(stderr(&output), "");
    let json = parse_json_stdout(&output);
    assert_eq!(json["sourceSummary"]["filesScanned"], 1);
    assert_eq!(json["sourceSummary"]["dynamicImports"], 0);
    assert!(!stdout(&output).contains("generated/Ignored.tsx"));
}

#[test]
fn discovered_config_applies_scan_ignore_patterns_across_commands() {
    let temp_dir = tempdir().expect("create temp dir");
    let root = temp_dir.path();
    let config_path = root.join("legolas.config.json");
    write_ignore_config_fixture(root, &config_path);

    let scan_output = run_cli_in_dir(root, &["scan", "--json"]);
    assert!(scan_output.status.success());
    assert_eq!(
        parse_json_stdout(&scan_output)["sourceSummary"]["filesScanned"],
        1
    );
    assert_eq!(
        parse_json_stdout(&scan_output)["sourceSummary"]["dynamicImports"],
        0
    );

    let visualize_output = run_cli_in_dir(root, &["visualize"]);
    assert!(visualize_output.status.success());
    assert_eq!(stderr(&visualize_output), "");

    let budget_output = run_cli_in_dir(root, &["budget", "--json"]);
    assert!(budget_output.status.success());
    let budget = parse_json_stdout(&budget_output);
    let dynamic_rule = budget["rules"]
        .as_array()
        .expect("budget rules array")
        .iter()
        .find(|rule| rule["key"] == "dynamicImportCount")
        .expect("dynamic import budget rule");
    assert_eq!(dynamic_rule["actual"], 0);

    let ci_output = run_cli_in_dir(root, &["ci", "--json"]);
    assert_eq!(ci_output.status.code(), Some(1));
    let ci = parse_json_stdout(&ci_output);
    let dynamic_rule = ci["rules"]
        .as_array()
        .expect("ci rules array")
        .iter()
        .find(|rule| rule["key"] == "dynamicImportCount")
        .expect("dynamic import ci rule");
    assert_eq!(dynamic_rule["actual"], 0);
}

fn write_ignore_config_fixture(root: &Path, config_path: &Path) {
    fs::write(
        root.join("package.json"),
        r#"{
  "name": "ignore-config-fixture",
  "dependencies": {
    "chart.js": "^4.0.0"
  }
}
"#,
    )
    .expect("write package.json");
    write_file(root, "src/App.tsx", "export const App = () => null;\n");
    write_file(
        root,
        "generated/Ignored.tsx",
        "export const loadChart = () => import(\"chart.js\");\n",
    );
    fs::write(
        config_path,
        r#"{
  "scan": {
    "ignorePatterns": ["generated/**"]
  }
}
"#,
    )
    .expect("write config");
}

fn write_file(root: &Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    let parent = path.parent().expect("fixture file parent");
    fs::create_dir_all(parent).expect("create fixture directory");
    fs::write(path, contents).expect("write fixture file");
}
