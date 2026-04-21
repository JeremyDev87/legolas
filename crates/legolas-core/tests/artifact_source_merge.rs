mod support;

use legolas_core::{
    artifacts::{
        detect::parse_artifact_file, merge_artifact_source_signals, ArtifactChunkSignal,
        ArtifactSignalKind, ArtifactSourceSignal,
    },
    import_scanner::{collect_source_files, scan_imports_with_aliases},
    FindingEvidence, HeavyDependency,
};

#[test]
fn merges_source_and_artifact_signals_for_heavy_packages() {
    let fixture = support::fixture_path("tests/fixtures/artifacts/merge-app");
    let source_files = collect_source_files(&fixture).expect("collect source files");
    let source_analysis =
        scan_imports_with_aliases(&fixture, &source_files, None).expect("scan imports");
    let artifact_summary =
        parse_artifact_file(&fixture.join("stats.json")).expect("parse artifact summary");

    let actual = merge_artifact_source_signals(
        &artifact_summary,
        &source_analysis,
        &[
            heavy_dependency("chart.js"),
            heavy_dependency("lodash"),
            heavy_dependency("react-icons"),
        ],
    );

    assert_eq!(
        actual,
        vec![
            ArtifactSourceSignal {
                package_name: "chart.js".to_string(),
                kind: ArtifactSignalKind::ArtifactSource,
                source_files: vec!["src/AdminPage.tsx".to_string()],
                chunks: vec![chunk(
                    "admin",
                    &["dashboard"],
                    &["dist/admin.css", "dist/admin.js", "dist/admin.js.map"],
                    6_200,
                )],
                artifact_bytes: 6_200,
            },
            ArtifactSourceSignal {
                package_name: "lodash".to_string(),
                kind: ArtifactSignalKind::Artifact,
                source_files: vec![],
                chunks: vec![chunk(
                    "vendor",
                    &["dashboard"],
                    &["dist/vendor.css", "dist/vendor.js", "dist/vendor.js.map"],
                    5_100,
                )],
                artifact_bytes: 5_100,
            },
            ArtifactSourceSignal {
                package_name: "react-icons".to_string(),
                kind: ArtifactSignalKind::Source,
                source_files: vec!["src/AdminPage.tsx".to_string()],
                chunks: vec![],
                artifact_bytes: 0,
            },
        ]
    );
}

#[test]
fn merged_signal_evidence_orders_source_files_before_artifact_chunks() {
    let fixture = support::fixture_path("tests/fixtures/artifacts/merge-app");
    let source_files = collect_source_files(&fixture).expect("collect source files");
    let source_analysis =
        scan_imports_with_aliases(&fixture, &source_files, None).expect("scan imports");
    let artifact_summary =
        parse_artifact_file(&fixture.join("stats.json")).expect("parse artifact summary");

    let actual = merge_artifact_source_signals(
        &artifact_summary,
        &source_analysis,
        &[
            heavy_dependency("chart.js"),
            heavy_dependency("lodash"),
            heavy_dependency("react-icons"),
        ],
    );

    assert_eq!(
        actual[0].evidence(),
        vec![
            FindingEvidence::new("source-file")
                .with_file("src/AdminPage.tsx")
                .with_specifier("chart.js")
                .with_detail("package imported in source analysis"),
            FindingEvidence::new("artifact-chunk")
                .with_file("dist/admin.js")
                .with_specifier("chart.js")
                .with_detail(
                    "artifact chunk `admin` contributes 6200 bytes; entrypoints: dashboard"
                ),
        ]
    );
    assert_eq!(
        actual[1].evidence(),
        vec![FindingEvidence::new("artifact-chunk")
            .with_file("dist/vendor.js")
            .with_specifier("lodash")
            .with_detail("artifact chunk `vendor` contributes 5100 bytes; entrypoints: dashboard"),]
    );
    assert_eq!(
        actual[2].evidence(),
        vec![FindingEvidence::new("source-file")
            .with_file("src/AdminPage.tsx")
            .with_specifier("react-icons")
            .with_detail("package imported in source analysis"),]
    );
}

#[test]
fn merged_signal_evidence_prefers_code_assets_over_maps_or_css() {
    let signal = ArtifactSourceSignal {
        package_name: "chart.js".to_string(),
        kind: ArtifactSignalKind::Artifact,
        source_files: vec![],
        chunks: vec![chunk(
            "admin",
            &["dashboard"],
            &["dist/admin.css", "dist/admin.js", "dist/admin.js.map"],
            6_200,
        )],
        artifact_bytes: 6_200,
    };

    assert_eq!(
        signal.evidence(),
        vec![FindingEvidence::new("artifact-chunk")
            .with_file("dist/admin.js")
            .with_specifier("chart.js")
            .with_detail("artifact chunk `admin` contributes 6200 bytes; entrypoints: dashboard"),]
    );
}

fn heavy_dependency(name: &str) -> HeavyDependency {
    HeavyDependency {
        name: name.to_string(),
        ..HeavyDependency::default()
    }
}

fn chunk(name: &str, entrypoints: &[&str], files: &[&str], bytes: usize) -> ArtifactChunkSignal {
    ArtifactChunkSignal {
        name: name.to_string(),
        entrypoints: entrypoints
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        files: files.iter().map(|value| (*value).to_string()).collect(),
        bytes,
    }
}
