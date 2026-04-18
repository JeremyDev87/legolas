mod support;

use std::{fs, process::Command};

use legolas_core::{
    lockfiles::{parse_duplicate_packages, DuplicateAnalysis},
    DuplicatePackage,
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn parses_npm_v3_duplicates_from_the_packages_section() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/npm-v3");

    let analysis = parse_duplicate_packages(&fixture, "npm").expect("parse npm v3 lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![
                duplicate("@scope/pkg", &["1.0.0", "1.2.0"]),
                duplicate("lodash", &["4.17.20", "4.17.21"]),
            ],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_npm_v1_duplicates_from_dependency_trees_and_sorts_versions_naturally() {
    let temp_dir = tempdir().expect("create temp dir");
    let package_lock_path = temp_dir.path().join("package-lock.json");

    fs::write(
        &package_lock_path,
        r#"{
  "name": "npm-v1-fixture",
  "lockfileVersion": 1,
  "dependencies": {
    "alpha": {
      "version": "1.0.0",
      "dependencies": {
        "shared": {
          "version": "1.2.10"
        },
        "left-pad": {
          "version": "1.0.1"
        }
      }
    },
    "beta": {
      "version": "2.0.0",
      "dependencies": {
        "shared": {
          "version": "1.2.2"
        }
      }
    },
    "shared": {
      "version": "1.2.0"
    },
    "left-pad": {
      "version": "1.0.0"
    }
  }
}"#,
    )
    .expect("write npm v1 fixture");

    let analysis = parse_duplicate_packages(temp_dir.path(), "unknown").expect("parse npm v1");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![
                DuplicatePackage {
                    name: "shared".to_string(),
                    versions: vec![
                        "1.2.0".to_string(),
                        "1.2.2".to_string(),
                        "1.2.10".to_string(),
                    ],
                    count: 3,
                    estimated_extra_kb: 36,
                },
                duplicate("left-pad", &["1.0.0", "1.0.1"]),
            ],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_pnpm_duplicates_from_packages_and_snapshots_sections() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/pnpm-basic");

    let analysis = parse_duplicate_packages(&fixture, "pnpm@9.0.0").expect("parse pnpm lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate("lodash", &["4.17.20", "4.17.21"])],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_yarn_berry_entries_with_version_colons() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/yarn-berry");

    let analysis = parse_duplicate_packages(&fixture, "yarn@4.1.1").expect("parse yarn berry");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate("lodash", &["4.17.20", "4.17.21"])],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_yarn_aliases_as_the_underlying_package_name() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/yarn-alias");

    let analysis = parse_duplicate_packages(&fixture, "yarn").expect("parse yarn alias lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate("react", &["18.2.0", "18.3.1"])],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_yarn_entries_with_one_sided_quotes_like_the_js_parser() {
    let temp_dir = tempdir().expect("create temp dir");
    let yarn_lock_path = temp_dir.path().join("yarn.lock");

    fs::write(
        &yarn_lock_path,
        r#""lodash@npm:^4.17.20:
  version "4.17.20"

lodash@npm:^4.17.21:
  version "4.17.21"
"#,
    )
    .expect("write yarn lockfile");

    let analysis = parse_duplicate_packages(temp_dir.path(), "yarn").expect("parse yarn lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate("lodash", &["4.17.20", "4.17.21"])],
            warnings: vec![],
        }
    );
}

#[test]
fn warns_when_bun_lockfile_is_selected_because_parsing_is_not_supported() {
    let temp_dir = tempdir().expect("create temp dir");
    let bun_lockb_path = temp_dir.path().join("bun.lockb");

    fs::write(&bun_lockb_path, [0_u8, 255, 42]).expect("write bun lockb fixture");

    let analysis = parse_duplicate_packages(temp_dir.path(), "bun@1.1.8").expect("parse bun lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![],
            warnings: vec![
                "Detected bun.lockb, but duplicate analysis does not yet parse Bun lockfiles, so results may be incomplete.".to_string(),
            ],
        }
    );
}

#[test]
fn prefers_text_bun_lock_when_both_bun_lockfiles_exist() {
    let temp_dir = tempdir().expect("create temp dir");
    let bun_lock_path = temp_dir.path().join("bun.lock");
    let bun_lockb_path = temp_dir.path().join("bun.lockb");

    fs::write(&bun_lock_path, "placeholder bun text lock").expect("write bun lock fixture");
    fs::write(&bun_lockb_path, [0_u8, 255, 42]).expect("write bun lockb fixture");

    let analysis = parse_duplicate_packages(temp_dir.path(), "bun@1.2.0").expect("parse bun lockfiles");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![],
            warnings: vec![
                "Multiple lockfiles detected. Duplicate analysis used bun.lock based on package manager \"bun@1.2.0\" and ignored bun.lockb.".to_string(),
                "Detected bun.lock, but duplicate analysis does not yet parse Bun lockfiles, so results may be incomplete.".to_string(),
            ],
        }
    );
}

#[test]
fn skips_empty_versions_in_package_lock_v3_entries() {
    let temp_dir = tempdir().expect("create temp dir");
    let package_lock_path = temp_dir.path().join("package-lock.json");

    fs::write(
        &package_lock_path,
        r#"{
  "name": "npm-v3-empty-version",
  "lockfileVersion": 3,
  "packages": {
    "node_modules/lodash": {
      "version": ""
    },
    "node_modules/example/node_modules/lodash": {
      "version": "4.17.21"
    }
  }
}"#,
    )
    .expect("write package lockfile");

    let analysis = parse_duplicate_packages(temp_dir.path(), "npm").expect("parse npm v3");

    assert_eq!(analysis, DuplicateAnalysis::default());
}

#[test]
fn skips_empty_versions_in_package_lock_v1_dependencies() {
    let temp_dir = tempdir().expect("create temp dir");
    let package_lock_path = temp_dir.path().join("package-lock.json");

    fs::write(
        &package_lock_path,
        r#"{
  "name": "npm-v1-empty-version",
  "lockfileVersion": 1,
  "dependencies": {
    "lodash": {
      "version": ""
    },
    "example": {
      "version": "1.0.0",
      "dependencies": {
        "lodash": {
          "version": "4.17.21"
        }
      }
    }
  }
}"#,
    )
    .expect("write package lockfile");

    let analysis = parse_duplicate_packages(temp_dir.path(), "npm").expect("parse npm v1");

    assert_eq!(analysis, DuplicateAnalysis::default());
}

#[test]
fn sorts_versions_like_js_locale_compare_numeric() {
    let input_versions = vec![
        "1",
        "01",
        "1.0.0",
        "1.0.00",
        "1.2.0",
        "1.02.0",
        "1.2.2",
        "1.2.10",
        "a1",
        "A1",
    ];
    let temp_dir = tempdir().expect("create temp dir");
    let package_lock_path = temp_dir.path().join("package-lock.json");
    let packages = input_versions
        .iter()
        .enumerate()
        .map(|(index, version)| {
            (
                format!("node_modules/pkg-{index}/node_modules/shared"),
                json!({ "version": version }),
            )
        })
        .collect::<serde_json::Map<_, _>>();

    fs::write(
        &package_lock_path,
        serde_json::to_string_pretty(&json!({
            "name": "npm-v3-version-sort",
            "lockfileVersion": 3,
            "packages": packages,
        }))
        .expect("serialize package lockfile"),
    )
    .expect("write package lockfile");

    let analysis = parse_duplicate_packages(temp_dir.path(), "npm").expect("parse npm v3");
    let expected_versions = load_js_sorted_versions(&input_versions);

    assert_eq!(
        analysis.duplicates,
        vec![DuplicatePackage {
            name: "shared".to_string(),
            versions: expected_versions,
            count: input_versions.len(),
            estimated_extra_kb: 162,
        }]
    );
}

#[test]
fn prefers_the_package_manager_lockfile_and_warns_about_the_rest() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/multi-lockfiles");

    let analysis =
        parse_duplicate_packages(&fixture, "pnpm@9.0.0").expect("parse preferred lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate("kleur", &["4.1.4", "4.1.5"])],
            warnings: vec![
                "Multiple lockfiles detected. Duplicate analysis used pnpm-lock.yaml based on package manager \"pnpm@9.0.0\" and ignored package-lock.json.".to_string(),
            ],
        }
    );
}

#[test]
fn falls_back_to_default_lockfile_priority_when_package_manager_is_unknown() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/multi-lockfiles");

    let analysis = parse_duplicate_packages(&fixture, "unknown").expect("parse default lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate("left-pad", &["1.0.0", "1.1.0"])],
            warnings: vec![
                "Multiple lockfiles detected. Duplicate analysis used package-lock.json and ignored pnpm-lock.yaml.".to_string(),
            ],
        }
    );
}

#[test]
fn returns_empty_results_when_no_supported_lockfile_exists() {
    let temp_dir = tempdir().expect("create temp dir");

    let analysis = parse_duplicate_packages(temp_dir.path(), "unknown").expect("parse empty dir");

    assert_eq!(analysis, DuplicateAnalysis::default());
}

fn duplicate(name: &str, versions: &[&str]) -> DuplicatePackage {
    DuplicatePackage {
        name: name.to_string(),
        versions: versions.iter().map(|value| (*value).to_string()).collect(),
        count: versions.len(),
        estimated_extra_kb: usize::max((versions.len().saturating_sub(1)) * 18, 18),
    }
}

fn load_js_sorted_versions(input_versions: &[&str]) -> Vec<String> {
    let payload = serde_json::to_string(input_versions).expect("serialize version payload");
    let script = format!(
        r#"
const versions = {payload};
versions.sort((left, right) => left.localeCompare(right, undefined, {{ numeric: true, sensitivity: "base" }}));
console.log(JSON.stringify(versions));
"#
    );

    let output = Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .expect("run node for JS version sort");

    assert!(
        output.status.success(),
        "node version sort exited unsuccessfully: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("parse JS-sorted versions")
}
