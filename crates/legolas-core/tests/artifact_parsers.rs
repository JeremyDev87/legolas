mod support;

use legolas_core::artifacts::detect::{
    detect_known_artifacts, detect_parser_kind, parse_artifact_file, ArtifactParserKind,
};
use legolas_core::artifacts::{
    esbuild::parse_esbuild_metafile, rollup::parse_rollup_metadata, webpack::parse_webpack_stats,
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn detect_known_artifacts_routes_each_fixture_to_the_expected_parser() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root.join("dist/stats.json"),
        support::fixture_path("tests/fixtures/artifacts/webpack-basic/stats.json"),
    );
    write_file(
        root.join("dist/meta.json"),
        support::fixture_path("tests/fixtures/artifacts/esbuild-basic/meta.json"),
    );
    write_file(
        root.join("meta.json"),
        support::fixture_path("tests/fixtures/artifacts/vite-basic/meta.json"),
    );

    let actual = detect_known_artifacts(root).expect("detect artifacts");

    assert_eq!(
        actual,
        vec![
            legolas_core::artifacts::detect::DetectedArtifact {
                relative_path: "dist/stats.json".to_string(),
                parser: ArtifactParserKind::Webpack,
            },
            legolas_core::artifacts::detect::DetectedArtifact {
                relative_path: "meta.json".to_string(),
                parser: ArtifactParserKind::Rollup,
            },
            legolas_core::artifacts::detect::DetectedArtifact {
                relative_path: "dist/meta.json".to_string(),
                parser: ArtifactParserKind::Esbuild,
            },
        ]
    );
}

#[test]
fn parse_artifact_file_reads_esbuild_fixture() {
    let summary = parse_artifact_file(&support::fixture_path(
        "tests/fixtures/artifacts/esbuild-basic/meta.json",
    ))
    .expect("parse esbuild artifact");

    assert_eq!(summary.bundler, "esbuild");
    assert_eq!(summary.entrypoints, vec!["src/main.ts".to_string()]);
    assert_eq!(summary.total_bytes, 13_200);
    assert_eq!(summary.chunks.len(), 2);
    assert_eq!(
        summary.modules,
        vec![
            module(
                "node_modules/react/index.js",
                Some("react"),
                &["main", "vendor"],
                5_400
            ),
            module("src/main.ts", None, &["main"], 1_200),
        ]
    );
}

#[test]
fn parse_esbuild_metafile_ignores_sourcemaps_and_non_js_assets() {
    let summary = parse_esbuild_metafile(&json!({
        "outputs": {
            "dist/main.js": {
                "bytes": 8_200,
                "entryPoint": "src/main.ts",
                "inputs": {
                    "src/main.ts": {
                        "bytesInOutput": 1_200
                    },
                    "node_modules/react/index.js": {
                        "bytesInOutput": 3_200
                    }
                }
            },
            "dist/main.js.map": {
                "bytes": 4_000,
                "inputs": {
                    "src/main.ts": {
                        "bytesInOutput": 1_200
                    }
                }
            },
            "dist/styles.css": {
                "bytes": 1_500,
                "inputs": {
                    "src/styles.css": {
                        "bytesInOutput": 1_500
                    }
                }
            },
            "dist/logo.svg": {
                "bytes": 700,
                "inputs": {
                    "src/logo.svg": {
                        "bytesInOutput": 700
                    }
                }
            }
        }
    }))
    .expect("parse esbuild metafile");

    assert_eq!(summary.total_bytes, 8_200);
    assert_eq!(summary.chunks.len(), 1);
    assert_eq!(
        summary.modules,
        vec![
            module(
                "node_modules/react/index.js",
                Some("react"),
                &["main"],
                3_200
            ),
            module("src/main.ts", None, &["main"], 1_200),
        ]
    );
}

#[test]
fn parse_artifact_file_reads_webpack_fixture() {
    let summary = parse_artifact_file(&support::fixture_path(
        "tests/fixtures/artifacts/webpack-basic/stats.json",
    ))
    .expect("parse webpack artifact");

    assert_eq!(summary.bundler, "webpack");
    assert_eq!(summary.entrypoints, vec!["main".to_string()]);
    assert_eq!(summary.total_bytes, 14_000);
    assert_eq!(summary.chunks.len(), 2);
    assert_eq!(
        summary.modules,
        vec![
            module(
                "node_modules/react/index.js",
                Some("react"),
                &["vendors"],
                4_500
            ),
            module("src/index.tsx", None, &["main"], 1_400),
        ]
    );
}

#[test]
fn parse_artifact_file_reads_rollup_fixture() {
    let summary = parse_artifact_file(&support::fixture_path(
        "tests/fixtures/artifacts/vite-basic/meta.json",
    ))
    .expect("parse rollup artifact");

    assert_eq!(summary.bundler, "rollup");
    assert_eq!(summary.entrypoints, vec!["main".to_string()]);
    assert_eq!(summary.total_bytes, 6_400);
    assert_eq!(summary.chunks.len(), 2);
    assert_eq!(
        summary.modules,
        vec![
            module(
                "/workspace/node_modules/vue/dist/vue.runtime.esm-bundler.js",
                Some("vue"),
                &["main", "vendor"],
                5_300,
            ),
            module("/workspace/src/main.ts", None, &["main"], 1_100),
        ]
    );
}

#[test]
fn detect_parser_kind_distinguishes_esbuild_and_rollup_meta_files_by_root_shape() {
    let webpack = json!({
        "entryPoints": {},
        "chunks": []
    });
    let esbuild = json!({
        "inputs": {},
        "outputs": {}
    });
    let rollup = json!({
        "outputs": []
    });
    let invalid_stats = json!({});

    assert_eq!(
        detect_parser_kind(std::path::Path::new("dist/stats.json"), &webpack),
        Some(ArtifactParserKind::Webpack)
    );
    assert_eq!(
        detect_parser_kind(std::path::Path::new("dist/stats.json"), &invalid_stats),
        None
    );
    assert_eq!(
        detect_parser_kind(std::path::Path::new("dist/meta.json"), &esbuild),
        Some(ArtifactParserKind::Esbuild)
    );
    assert_eq!(
        detect_parser_kind(std::path::Path::new("meta.json"), &rollup),
        Some(ArtifactParserKind::Rollup)
    );
}

#[test]
fn detect_known_artifacts_ignores_stats_json_without_webpack_shape() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    std::fs::create_dir_all(root.join("dist")).expect("create dist dir");
    std::fs::write(root.join("dist/stats.json"), "{}\n").expect("write invalid stats");

    let actual = detect_known_artifacts(root).expect("detect artifacts");

    assert!(actual.is_empty());
}

#[test]
fn detect_known_artifacts_skips_malformed_json_and_keeps_valid_artifacts() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    std::fs::create_dir_all(root.join("dist")).expect("create dist dir");
    std::fs::write(root.join("dist/stats.json"), "{\n").expect("write malformed stats");
    write_file(
        root.join("dist/meta.json"),
        support::fixture_path("tests/fixtures/artifacts/esbuild-basic/meta.json"),
    );

    let actual = detect_known_artifacts(root).expect("detect artifacts");

    assert_eq!(
        actual,
        vec![legolas_core::artifacts::detect::DetectedArtifact {
            relative_path: "dist/meta.json".to_string(),
            parser: ArtifactParserKind::Esbuild,
        }]
    );
}

#[test]
fn parse_webpack_stats_merges_same_module_across_chunks() {
    let summary = parse_webpack_stats(&json!({
        "entryPoints": {
            "main": {
                "assets": [
                    { "name": "main.js" },
                    { "name": "vendors.js" }
                ]
            }
        },
        "chunks": [
            {
                "id": 1,
                "names": ["main"],
                "files": ["main.js"],
                "initial": true,
                "size": 3_000,
                "modules": [
                    {
                        "identifier": "node_modules/react/index.js",
                        "size": 1_000
                    }
                ]
            },
            {
                "id": 2,
                "names": ["vendors"],
                "files": ["vendors.js"],
                "initial": true,
                "size": 4_000,
                "modules": [
                    {
                        "identifier": "node_modules/react/index.js",
                        "size": 2_500
                    }
                ]
            }
        ]
    }))
    .expect("parse webpack stats");

    assert_eq!(
        summary.modules,
        vec![module(
            "node_modules/react/index.js",
            Some("react"),
            &["main", "vendors"],
            3_500,
        )]
    );
}

#[test]
fn nested_node_modules_paths_use_the_deepest_package_name() {
    let esbuild = parse_esbuild_metafile(&json!({
        "outputs": {
            "dist/main.js": {
                "bytes": 1_000,
                "entryPoint": "src/main.ts",
                "inputs": {
                    "/repo/node_modules/pkg/node_modules/dep/index.js": {
                        "bytesInOutput": 500
                    }
                }
            }
        }
    }))
    .expect("parse esbuild metafile");
    assert_eq!(
        esbuild.modules,
        vec![module(
            "/repo/node_modules/pkg/node_modules/dep/index.js",
            Some("dep"),
            &["main"],
            500,
        )]
    );

    let rollup = parse_rollup_metadata(&json!({
        "outputs": [
            {
                "type": "chunk",
                "name": "main",
                "file": "dist/main.js",
                "isEntry": true,
                "modules": {
                    "/repo/node_modules/pkg/node_modules/@scope/dep/index.js": {
                        "renderedLength": 700
                    }
                }
            }
        ]
    }))
    .expect("parse rollup metadata");
    assert_eq!(
        rollup.modules,
        vec![module(
            "/repo/node_modules/pkg/node_modules/@scope/dep/index.js",
            Some("@scope/dep"),
            &["main"],
            700,
        )]
    );

    let webpack = parse_webpack_stats(&json!({
        "entryPoints": {
            "main": {
                "assets": [{ "name": "main.js" }]
            }
        },
        "chunks": [
            {
                "id": 1,
                "names": ["main"],
                "files": ["main.js"],
                "initial": true,
                "size": 1_000,
                "modules": [
                    {
                        "identifier": "C:\\repo\\node_modules\\pkg\\node_modules\\dep\\index.js",
                        "size": 600
                    }
                ]
            }
        ]
    }))
    .expect("parse webpack stats");
    assert_eq!(
        webpack.modules,
        vec![module(
            "C:\\repo\\node_modules\\pkg\\node_modules\\dep\\index.js",
            Some("dep"),
            &["main"],
            600,
        )]
    );
}

fn module(
    id: &str,
    package_name: Option<&str>,
    chunks: &[&str],
    bytes: usize,
) -> legolas_core::artifacts::ArtifactModuleContribution {
    legolas_core::artifacts::ArtifactModuleContribution {
        id: id.to_string(),
        package_name: package_name.map(ToOwned::to_owned),
        chunks: chunks.iter().map(|chunk| chunk.to_string()).collect(),
        bytes,
    }
}

fn write_file(target: std::path::PathBuf, source: std::path::PathBuf) {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).expect("create parent dir");
    }
    std::fs::copy(source, target).expect("copy fixture");
}
