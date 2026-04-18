mod support;

use assert_cmd::Command;

#[test]
fn prints_version_without_a_command() {
    for args in [vec!["--version"], vec!["-v"]] {
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
    for args in [Vec::<&str>::new(), vec!["help"], vec!["--help"], vec!["-h"]] {
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
                "optimize".to_string(),
                fixture.display().to_string(),
                "--top".to_string(),
                "NaN".to_string(),
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
        assert_eq!(output.status.code(), Some(1), "expected exit code 1 for {oracle}");
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
                "optimize".to_string(),
                fixture.display().to_string(),
                "--top".to_string(),
            ],
            "legolas: --top expects a number\n",
        ),
        (
            vec!["--bogus".to_string()],
            "legolas: unknown flag \"--bogus\"\n",
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
