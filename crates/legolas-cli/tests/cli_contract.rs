mod support;

use assert_cmd::Command;
use serde_json::json;

#[test]
fn prints_version_without_a_command() {
    for args in [
        vec!["--version"],
        vec!["-v"],
        vec!["budget", "--version", "--top", "1"],
        vec!["ci", "--version", "--limit", "1"],
    ] {
        let output = Command::cargo_bin("legolas-cli")
            .expect("build binary")
            .args(args)
            .output()
            .expect("run version");

        assert!(output.status.success());
        assert_eq!(
            support::normalize_cli_output(&String::from_utf8(output.stdout).expect("stdout")),
            support::read_oracle("cli/version.txt")
        );
        assert_eq!(String::from_utf8(output.stderr).expect("stderr"), "");
    }
}

#[test]
fn prints_help_for_empty_command_and_help_variants() {
    for args in [
        Vec::<&str>::new(),
        vec!["help"],
        vec!["--help"],
        vec!["-h"],
        vec!["budget", "--help", "--limit", "1"],
        vec!["ci", "--help", "--top", "1"],
    ] {
        let output = Command::cargo_bin("legolas-cli")
            .expect("build binary")
            .args(args)
            .output()
            .expect("run help");

        assert!(output.status.success());
        assert_eq!(
            support::normalize_cli_output(&String::from_utf8(output.stdout).expect("stdout")),
            support::read_oracle("cli/help.txt")
        );
        assert_eq!(String::from_utf8(output.stderr).expect("stderr"), "");
    }
}

#[test]
fn matches_scan_visualize_and_optimize_oracles() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let cases = [
        (
            vec!["scan".to_string(), fixture.display().to_string()],
            "basic-app/scan.txt",
        ),
        (
            vec!["visualize".to_string(), fixture.display().to_string()],
            "basic-app/visualize.txt",
        ),
        (
            vec!["optimize".to_string(), fixture.display().to_string()],
            "basic-app/optimize.txt",
        ),
        (
            vec!["budget".to_string(), fixture.display().to_string()],
            "basic-app/budget.txt",
        ),
    ];

    for (args, oracle) in cases {
        let output = Command::cargo_bin("legolas-cli")
            .expect("build binary")
            .args(args)
            .output()
            .expect("run command");

        assert!(output.status.success(), "expected success for {oracle}");
        assert_eq!(
            support::normalize_cli_output(&String::from_utf8(output.stdout).expect("stdout")),
            support::read_oracle(oracle)
        );
        assert_eq!(String::from_utf8(output.stderr).expect("stderr"), "");
    }
}

#[test]
fn matches_scan_json_oracle() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(["scan", &fixture.display().to_string(), "--json"])
        .output()
        .expect("run scan --json");

    assert!(output.status.success());
    assert_eq!(
        support::normalize_analysis_json_output(&String::from_utf8(output.stdout).expect("stdout")),
        support::normalize_analysis_json_output(&support::read_oracle("basic-app/scan.json"))
    );
    assert_eq!(String::from_utf8(output.stderr).expect("stderr"), "");
}

#[test]
fn monorepo_scan_outputs_workspace_summaries_in_text_and_json() {
    let fixture = support::fixture_path("tests/fixtures/monorepo/pnpm-workspace");

    let text_output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(["scan", &fixture.display().to_string()])
        .output()
        .expect("run monorepo scan");
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8(text_output.stdout).expect("stdout");
    assert!(text_stdout.contains("Workspace summaries:"));
    assert!(text_stdout.contains("admin-app (apps/admin): 3 imported packages, 2 heavy dependencies, 0 duplicate packages, ~42 KB potential saved"));
    assert!(text_stdout.contains("storefront-app (apps/storefront): 2 imported packages, 1 heavy dependencies, 0 duplicate packages, ~13 KB potential saved"));
    assert_eq!(String::from_utf8(text_output.stderr).expect("stderr"), "");

    let json_output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(["scan", &fixture.display().to_string(), "--json"])
        .output()
        .expect("run monorepo scan --json");
    assert!(json_output.status.success());
    let analysis = support::normalize_analysis_json_output(
        &String::from_utf8(json_output.stdout).expect("stdout"),
    );
    assert_eq!(
        analysis["workspaceSummaries"],
        json!([
            {
                "name": "admin-app",
                "path": "apps/admin",
                "importedPackages": 3,
                "heavyDependencies": 2,
                "duplicatePackages": 0,
                "potentialKbSaved": 42
            },
            {
                "name": "storefront-app",
                "path": "apps/storefront",
                "importedPackages": 2,
                "heavyDependencies": 1,
                "duplicatePackages": 0,
                "potentialKbSaved": 13
            }
        ])
    );
    assert_eq!(String::from_utf8(json_output.stderr).expect("stderr"), "");
}

#[test]
fn merge_app_scan_json_exposes_additive_artifact_contract() {
    let fixture = support::fixture_path("tests/fixtures/artifacts/merge-app");
    let output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(["scan", &fixture.display().to_string(), "--json"])
        .output()
        .expect("run merge-app scan --json");

    assert!(output.status.success());
    let analysis =
        support::normalize_analysis_json_output(&String::from_utf8(output.stdout).expect("stdout"));
    let heavy_dependencies = analysis["heavyDependencies"]
        .as_array()
        .expect("heavy dependencies array");

    let chart_js = heavy_dependencies
        .iter()
        .find(|item| item["name"] == "chart.js")
        .expect("chart.js heavy dependency");
    assert_eq!(chart_js["analysisSource"], json!("artifact-source"));
    let chart_js_evidence = chart_js["evidence"].as_array().expect("chart.js evidence");
    assert_eq!(chart_js_evidence.len(), 2);
    assert_eq!(chart_js_evidence[0]["kind"], json!("source-file"));
    assert_eq!(chart_js_evidence[0]["file"], json!("src/AdminPage.tsx"));
    assert_eq!(chart_js_evidence[0]["specifier"], json!("chart.js"));
    assert_eq!(chart_js_evidence[1]["kind"], json!("artifact-chunk"));
    assert_eq!(chart_js_evidence[1]["file"], json!("dist/admin.js"));
    assert_eq!(chart_js_evidence[1]["specifier"], json!("chart.js"));
    assert_eq!(
        chart_js_evidence[1]["detail"],
        json!("artifact chunk `admin` contributes 6200 bytes; entrypoints: dashboard")
    );

    let lodash = heavy_dependencies
        .iter()
        .find(|item| item["name"] == "lodash")
        .expect("lodash heavy dependency");
    assert_eq!(lodash["analysisSource"], json!("artifact"));
    assert_eq!(
        lodash["evidence"],
        json!([
            {
                "kind": "artifact-chunk",
                "file": "dist/vendor.js",
                "specifier": "lodash",
                "detail": "artifact chunk `vendor` contributes 5100 bytes; entrypoints: dashboard"
            }
        ])
    );

    let react_icons = heavy_dependencies
        .iter()
        .find(|item| item["name"] == "react-icons")
        .expect("react-icons heavy dependency");
    assert_eq!(react_icons["analysisSource"], json!("source-import"));
    let react_icons_evidence = react_icons["evidence"]
        .as_array()
        .expect("react-icons evidence");
    assert_eq!(react_icons_evidence.len(), 1);
    assert_eq!(react_icons_evidence[0]["kind"], json!("source-file"));
    assert_eq!(react_icons_evidence[0]["file"], json!("src/AdminPage.tsx"));
    assert_eq!(react_icons_evidence[0]["specifier"], json!("react-icons"));
}

#[test]
fn normalize_analysis_json_output_normalizes_artifact_summary_paths() {
    let normalized = support::normalize_analysis_json_output(
        r#"{
  "projectRoot": "C:\\repo",
  "bundleArtifacts": ["dist\\stats.json"],
  "artifactSummary": {
    "bundler": "webpack",
    "entrypoints": ["src\\main.ts"],
    "chunks": [
      {
        "name": "main",
        "entrypoints": ["src\\main.ts"],
        "files": ["dist\\main.js"],
        "initial": true,
        "bytes": 9000
      }
    ],
    "modules": [
      {
        "id": "src\\main.ts",
        "packageName": null,
        "chunks": ["main"],
        "bytes": 1400
      }
    ],
    "totalBytes": 9000
  },
  "packageSummary": {"name":"demo","dependencyCount":0,"devDependencyCount":0},
  "sourceSummary": {"filesScanned":0,"importedPackages":0,"dynamicImports":0},
  "heavyDependencies": [],
  "duplicatePackages": [],
  "lazyLoadCandidates": [],
  "treeShakingWarnings": [],
  "unusedDependencyCandidates": [],
  "warnings": [],
  "impact": {"potentialKbSaved":0,"estimatedLcpImprovementMs":0,"confidence":"directional","summary":"n/a"},
  "metadata": {"mode":"artifact-assisted","generatedAt":"2026-01-01T00:00:00.000Z"}
}"#,
    );

    assert_eq!(normalized["bundleArtifacts"][0], "dist/stats.json");
    assert_eq!(
        normalized["artifactSummary"]["entrypoints"][0],
        "src/main.ts"
    );
    assert_eq!(
        normalized["artifactSummary"]["chunks"][0]["files"][0],
        "dist/main.js"
    );
    assert_eq!(
        normalized["artifactSummary"]["modules"][0]["id"],
        "src/main.ts"
    );
}

#[test]
fn matches_validation_error_oracles() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let cases = [
        (
            vec![
                "visualize".to_string(),
                fixture.display().to_string(),
                "--limit".to_string(),
                "nope".to_string(),
            ],
            "errors/visualize-limit.txt",
        ),
        (
            vec![
                "visualize".to_string(),
                fixture.display().to_string(),
                "--limit".to_string(),
                "-1".to_string(),
            ],
            "errors/visualize-limit.txt",
        ),
        (
            vec![
                "optimize".to_string(),
                fixture.display().to_string(),
                "--top".to_string(),
                "NaN".to_string(),
            ],
            "errors/optimize-top.txt",
        ),
        (
            vec![
                "optimize".to_string(),
                fixture.display().to_string(),
                "--top".to_string(),
                "-1".to_string(),
            ],
            "errors/optimize-top.txt",
        ),
    ];

    for (args, oracle) in cases {
        let output = Command::cargo_bin("legolas-cli")
            .expect("build binary")
            .args(args)
            .output()
            .expect("run command");

        assert!(!output.status.success(), "expected failure for {oracle}");
        assert_eq!(
            output.status.code(),
            Some(1),
            "expected exit code 1 for {oracle}"
        );
        assert_eq!(String::from_utf8(output.stdout).expect("stdout"), "");
        assert_eq!(
            support::normalize_cli_output(&String::from_utf8(output.stderr).expect("stderr")),
            support::read_oracle(oracle)
        );
    }
}

#[test]
fn matches_missing_number_and_unknown_flag_contracts() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let cases = [
        (
            vec![
                "visualize".to_string(),
                fixture.display().to_string(),
                "--limit".to_string(),
            ],
            "legolas: --limit expects a number\n",
        ),
        (
            vec![
                "--limit".to_string(),
                "-1".to_string(),
                "visualize".to_string(),
                fixture.display().to_string(),
            ],
            "legolas: --limit expects a number\n",
        ),
        (
            vec![
                "optimize".to_string(),
                fixture.display().to_string(),
                "--top".to_string(),
            ],
            "legolas: --top expects a number\n",
        ),
        (
            vec![
                "--top".to_string(),
                "-1".to_string(),
                "optimize".to_string(),
                fixture.display().to_string(),
            ],
            "legolas: --top expects a number\n",
        ),
        (
            vec!["--bogus".to_string()],
            "legolas: unknown flag \"--bogus\"\n",
        ),
        (
            vec!["scan".to_string(), "--config".to_string()],
            "legolas: --config expects a path\n",
        ),
        (
            vec![
                "visualize".to_string(),
                fixture.display().to_string(),
                "--baseline".to_string(),
                fixture.join("baseline.json").display().to_string(),
            ],
            "legolas: unknown flag \"--baseline\"\n",
        ),
        (
            vec![
                "visualize".to_string(),
                fixture.display().to_string(),
                "--write-baseline".to_string(),
                fixture.join("baseline.json").display().to_string(),
            ],
            "legolas: unknown flag \"--write-baseline\"\n",
        ),
        (
            vec![
                "visualize".to_string(),
                fixture.display().to_string(),
                "--regression-only".to_string(),
            ],
            "legolas: unknown flag \"--regression-only\"\n",
        ),
    ];

    for (args, expected_stderr) in cases {
        let output = Command::cargo_bin("legolas-cli")
            .expect("build binary")
            .args(args)
            .output()
            .expect("run invalid command");

        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(String::from_utf8(output.stdout).expect("stdout"), "");
        assert_eq!(
            String::from_utf8(output.stderr).expect("stderr"),
            expected_stderr
        );
    }
}
