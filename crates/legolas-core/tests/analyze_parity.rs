mod support;

use std::{fs, path::Path, thread, time::Duration};

#[cfg(unix)]
use std::os::unix::fs::symlink as create_dir_symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_dir as create_dir_symlink;

use legolas_core::{
    analyze_project,
    artifacts::{
        detect::parse_artifact_file, ArtifactChunk, ArtifactModuleContribution, ArtifactSummary,
    },
    Analysis, LegolasError,
};
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
fn analyze_project_emits_relative_evidence_blocks_in_parity_fixture() {
    let analysis = analyze_project(support::fixture_path("tests/fixtures/parity/basic-app"))
        .expect("analyze parity fixture");

    let heavy = analysis
        .heavy_dependencies
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js heavy dependency");
    let heavy_evidence = heavy.finding.evidence.first().expect("heavy evidence");
    assert_eq!(heavy_evidence.file.as_deref(), Some("src/Dashboard.tsx"));
    assert_eq!(heavy_evidence.specifier.as_deref(), Some("chart.js"));
    assert_eq!(
        heavy_evidence.detail.as_deref(),
        Some("static import; Charting code is often only needed on a subset of screens.")
    );

    let lazy = analysis
        .lazy_load_candidates
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js lazy-load candidate");
    let lazy_evidence = lazy.finding.evidence.first().expect("lazy-load evidence");
    assert_eq!(lazy_evidence.file.as_deref(), Some("src/Dashboard.tsx"));
    assert_eq!(lazy_evidence.specifier.as_deref(), Some("chart.js"));
    assert_eq!(
        lazy_evidence.detail.as_deref(),
        Some("route-like UI surface matched `dashboard` keyword")
    );

    let warning = analysis
        .tree_shaking_warnings
        .iter()
        .find(|item| item.key == "lodash-root-import")
        .expect("lodash tree-shaking warning");
    let warning_evidence = warning
        .finding
        .evidence
        .first()
        .expect("tree-shaking evidence");
    assert_eq!(warning_evidence.file.as_deref(), Some("src/Dashboard.tsx"));
    assert_eq!(warning_evidence.specifier.as_deref(), Some("lodash"));
    assert_eq!(
        warning_evidence.detail.as_deref(),
        Some("root package import")
    );
}

#[test]
fn normalize_analysis_for_oracle_normalizes_artifact_summary_paths() {
    let normalized = support::normalize_analysis_for_oracle(&Analysis {
        project_root: r"C:\repo".to_string(),
        bundle_artifacts: vec![r"dist\stats.json".to_string()],
        artifact_summary: Some(ArtifactSummary {
            bundler: "webpack".to_string(),
            entrypoints: vec![r"src\main.ts".to_string()],
            chunks: vec![ArtifactChunk {
                name: "main".to_string(),
                entrypoints: vec![r"src\main.ts".to_string()],
                files: vec![r"dist\main.js".to_string()],
                initial: true,
                bytes: 9_000,
            }],
            modules: vec![ArtifactModuleContribution {
                id: r"src\main.ts".to_string(),
                package_name: None,
                chunks: vec!["main".to_string()],
                bytes: 1_400,
            }],
            total_bytes: 9_000,
        }),
        ..Analysis::default()
    });

    assert!(normalized.contains("\"dist/main.js\""));
    assert!(normalized.contains("\"src/main.ts\""));
    assert!(!normalized.contains('\\'));
}

#[test]
fn analyze_project_uses_parsed_artifact_summary_for_real_bundle_artifacts() {
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
    write_file(
        root,
        "dist/stats.json",
        include_str!("../../../tests/fixtures/artifacts/webpack-basic/stats.json"),
    );

    let analysis = analyze_project(root).expect("analyze project");
    let expected_summary = parse_artifact_file(&support::fixture_path(
        "tests/fixtures/artifacts/webpack-basic/stats.json",
    ))
    .expect("parse artifact fixture")
    .normalized();

    assert_eq!(analysis.metadata.mode, "artifact-assisted");
    assert_eq!(
        analysis.bundle_artifacts,
        vec!["dist/stats.json".to_string()]
    );
    assert_eq!(analysis.artifact_summary, Some(expected_summary));
    assert!(analysis.warnings.is_empty());
}

#[test]
fn analyze_project_prefers_the_latest_parsed_bundle_artifact_when_multiple_exist() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "multi-artifact-app",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#,
    );
    write_file(root, "src/App.ts", "export const App = () => null;\n");
    write_file(
        root,
        "dist/stats.json",
        include_str!("../../../tests/fixtures/artifacts/webpack-basic/stats.json"),
    );
    thread::sleep(Duration::from_millis(1100));
    write_file(
        root,
        "dist/meta.json",
        include_str!("../../../tests/fixtures/artifacts/esbuild-basic/meta.json"),
    );

    let analysis = analyze_project(root).expect("analyze project");
    let expected_summary = parse_artifact_file(&support::fixture_path(
        "tests/fixtures/artifacts/esbuild-basic/meta.json",
    ))
    .expect("parse artifact fixture")
    .normalized();

    assert_eq!(analysis.metadata.mode, "artifact-assisted");
    assert_eq!(
        analysis.bundle_artifacts,
        vec!["dist/stats.json".to_string(), "dist/meta.json".to_string()]
    );
    assert_eq!(analysis.artifact_summary, Some(expected_summary));
    assert_eq!(
        analysis.warnings,
        vec![
            "Multiple bundle artifacts were parsed; artifactSummary selected `dist/meta.json` by latest modification time.".to_string()
        ]
    );
}

#[test]
fn analyze_project_falls_back_to_heuristic_mode_when_known_artifact_shape_is_unsupported() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "unsupported-artifact-app",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#,
    );
    write_file(root, "src/App.ts", "export const App = () => null;\n");
    write_file(root, "dist/stats.json", "{}\n");

    let analysis = analyze_project(root).expect("analyze project");

    assert_eq!(analysis.metadata.mode, "heuristic");
    assert_eq!(
        analysis.bundle_artifacts,
        vec!["dist/stats.json".to_string()]
    );
    assert!(analysis.artifact_summary.is_none());
    assert_eq!(
        analysis.warnings,
        vec![
            "Bundle artifact `dist/stats.json` could not be parsed: not implemented: unsupported artifact file shape".to_string()
        ]
    );
}

#[test]
fn analyze_project_falls_back_to_heuristic_mode_when_known_artifact_json_is_malformed() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "malformed-artifact-app",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#,
    );
    write_file(root, "src/App.ts", "export const App = () => null;\n");
    write_file(root, "dist/stats.json", "{\n");

    let analysis = analyze_project(root).expect("analyze project");

    assert_eq!(analysis.metadata.mode, "heuristic");
    assert_eq!(
        analysis.bundle_artifacts,
        vec!["dist/stats.json".to_string()]
    );
    assert!(analysis.artifact_summary.is_none());
    assert_eq!(analysis.warnings.len(), 1);
    assert!(analysis.warnings[0].contains("Bundle artifact `dist/stats.json` could not be parsed:"));
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

#[test]
fn analyze_project_uses_alias_config_to_ignore_local_alias_imports() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "alias-analysis-app",
  "dependencies": {
    "chart.js": "^4.4.1",
    "components": "^1.0.0"
  }
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "components/*": ["src/components/*"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "import { Button } from \"components/Button\";\nimport \"chart.js/auto\";\nexport const App = Button;\n",
    );
    write_file(
        root,
        "src/components/Button.tsx",
        "export const Button = 'button';\n",
    );

    let analysis = analyze_project(root).expect("analyze alias-aware project");

    assert_eq!(analysis.source_summary.imported_packages, 1);
    assert_eq!(
        analysis
            .unused_dependency_candidates
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["components"]
    );
}

#[test]
fn analyze_project_surfaces_malformed_alias_configs_instead_of_falling_back() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    let config_path = root.join("tsconfig.json");

    write_file(
        root,
        "package.json",
        r#"{
  "name": "malformed-alias-app",
  "dependencies": {
    "lodash": "^4.17.21"
  }
}"#,
    );
    write_file(root, "src/App.ts", "import { chunk } from \"lodash\";\n");
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "paths": "@/src/*"
  }
}"#,
    );

    let error = analyze_project(root).expect_err("malformed alias config should fail");

    match error {
        LegolasError::UnsupportedConfigShape {
            path,
            key_path,
            message,
        } => {
            assert_eq!(path, config_path.display().to_string());
            assert_eq!(key_path, "compilerOptions.paths");
            assert_eq!(message, "expected object");
        }
        other => panic!("expected unsupported config shape error, got {other:?}"),
    }
}

#[test]
fn analyze_project_uses_middle_wildcard_alias_targets_to_ignore_local_imports() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "middle-wildcard-alias-analysis-app",
  "dependencies": {
    "components": "^1.0.0"
  }
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "components/*": ["src/components/*/index"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "import Button from \"components/button\";\nexport default Button;\n",
    );
    write_file(
        root,
        "src/components/button/index.ts",
        "const Button = 'button';\nexport default Button;\n",
    );

    let analysis = analyze_project(root).expect("analyze middle-wildcard alias project");

    assert_eq!(analysis.source_summary.imported_packages, 0);
    assert_eq!(analysis.source_summary.dynamic_imports, 0);
    assert_eq!(
        analysis
            .unused_dependency_candidates
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["components"]
    );
}

#[test]
fn analyze_project_keeps_node_modules_package_remaps_as_used_dependencies() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "package-remap-analysis-app",
  "dependencies": {
    "react": "^18.2.0"
  }
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "react": ["node_modules/preact/compat"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "import { h } from \"react\";\nexport const App = h;\n",
    );
    write_file(
        root,
        "node_modules/preact/compat/index.js",
        "export const h = () => null;\n",
    );

    let analysis = analyze_project(root).expect("analyze package-remap project");

    assert_eq!(analysis.source_summary.imported_packages, 1);
    assert_eq!(analysis.source_summary.dynamic_imports, 0);
    assert!(analysis.unused_dependency_candidates.is_empty());
}

#[test]
fn analyze_project_counts_dynamic_alias_entries_even_without_package_usage() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "dynamic-alias-analysis-app",
  "private": true
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "routes/*": ["src/routes/*"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "export async function load() {\n  await import(\"routes/dashboard\");\n  await import(\"./local\");\n}\n",
    );
    write_file(
        root,
        "src/routes/dashboard.tsx",
        "export default 'dashboard';\n",
    );
    write_file(root, "src/local.ts", "export default 'local';\n");

    let analysis = analyze_project(root).expect("analyze dynamic-alias project");

    assert_eq!(analysis.source_summary.imported_packages, 0);
    assert_eq!(analysis.source_summary.dynamic_imports, 2);
}

#[test]
fn analyze_project_keeps_symlinked_package_remaps_as_used_dependencies() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "symlink-package-remap-analysis-app",
  "dependencies": {
    "react": "^18.2.0"
  }
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "react": ["vendor/react"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "import { h } from \"react\";\nexport const App = h;\n",
    );
    write_file(
        root,
        "node_modules/preact/compat/index.js",
        "export const h = () => null;\n",
    );
    fs::create_dir_all(root.join("vendor")).expect("create vendor dir");
    create_dir_symlink(
        root.join("node_modules/preact/compat"),
        root.join("vendor/react"),
    )
    .expect("create vendor symlink");

    let analysis = analyze_project(root).expect("analyze symlink package-remap project");

    assert_eq!(analysis.source_summary.imported_packages, 1);
    assert!(analysis.unused_dependency_candidates.is_empty());
}

fn write_file(root: &Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    let parent = path.parent().expect("fixture file parent");
    fs::create_dir_all(parent).expect("create fixture directory");
    fs::write(path, contents).expect("write fixture file");
}
