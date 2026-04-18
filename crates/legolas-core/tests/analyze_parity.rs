mod support;

use std::{fs, path::Path};

use legolas_core::analyze_project;
use regex::Regex;
use tempfile::tempdir;

#[test]
fn analyze_project_matches_the_parity_oracle() {
    let analysis = analyze_project(support::fixture_path("tests/fixtures/parity/basic-app"))
        .expect("analyze parity fixture");
    let actual = support::normalize_analysis_for_oracle(&analysis);
    let expected = support::read_oracle("basic-app/scan.json");

    assert_eq!(actual, expected);
}

#[test]
fn analyze_project_switches_to_artifact_assisted_mode_only_for_real_files() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "artifact-app",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#,
    );
    write_file(root, "src/App.ts", "export const App = () => null;\n");
    write_file(root, "dist/stats.json", "{}\n");

    let analysis = analyze_project(root).expect("analyze project");

    assert_eq!(analysis.metadata.mode, "artifact-assisted");
    assert_eq!(
        analysis.bundle_artifacts,
        vec!["dist/stats.json".to_string()]
    );
}

#[test]
fn analyze_project_unused_dependency_candidates_ignore_dev_dependencies() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "unused-deps-app",
  "dependencies": {
    "chart.js": "^4.4.1",
    "lodash": "^4.17.21"
  },
  "devDependencies": {
    "vite": "^5.2.0"
  }
}"#,
    );
    write_file(root, "src/App.ts", "import { chunk } from \"lodash\";\n");

    let analysis = analyze_project(root).expect("analyze project");

    assert_eq!(
        analysis
            .unused_dependency_candidates
            .iter()
            .map(|item| (item.name.as_str(), item.version_range.as_str()))
            .collect::<Vec<_>>(),
        vec![("chart.js", "^4.4.1")]
    );
}

#[test]
fn analyze_project_dedupes_dependencies_shadowed_by_optional_dependencies() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "optional-shadow-app",
  "dependencies": {
    "lodash": "^4.17.21"
  },
  "optionalDependencies": {
    "lodash": "^4.17.20"
  }
}"#,
    );
    write_file(root, "src/App.ts", "import _ from \"lodash\";\n");

    let analysis = analyze_project(root).expect("analyze project");

    assert_eq!(analysis.heavy_dependencies.len(), 1);
    assert_eq!(analysis.heavy_dependencies[0].name, "lodash");
    assert_eq!(analysis.heavy_dependencies[0].version_range, "^4.17.20");
    assert_eq!(analysis.impact.potential_kb_saved, 39);
}

#[test]
fn analyze_project_emits_iso_8601_generated_at_metadata() {
    let analysis = analyze_project(support::fixture_path("tests/fixtures/parity/basic-app"))
        .expect("analyze parity fixture");
    let iso8601_utc =
        Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$").expect("valid regex");

    assert!(iso8601_utc.is_match(&analysis.metadata.generated_at));
}

fn write_file(root: &Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    let parent = path.parent().expect("fixture file parent");
    fs::create_dir_all(parent).expect("create fixture directory");
    fs::write(path, contents).expect("write fixture file");
}
