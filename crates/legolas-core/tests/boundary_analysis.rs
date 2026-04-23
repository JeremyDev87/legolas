mod support;

use std::fs;

use legolas_core::{
    analyze_project, FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata,
};
use tempfile::tempdir;

#[test]
fn analyze_project_emits_general_server_client_boundary_warning() {
    let analysis = analyze_project(support::fixture_path("tests/fixtures/boundaries/general"))
        .expect("analyze boundary fixture");

    assert_eq!(analysis.boundary_warnings.len(), 1);

    let warning = &analysis.boundary_warnings[0];
    assert_eq!(
        warning.message,
        "Client surface `src/client/App.tsx` imports the Node-only `node:fs` module."
    );
    assert_eq!(
        warning.recommendation,
        "Keep Node-only work on the server and pass browser-safe data into the client component."
    );
    assert_finding_metadata(
        &warning.finding,
        FindingMetadata::new(
            "boundary:server-client:fs",
            FindingAnalysisSource::SourceImport,
        )
        .with_confidence(FindingConfidence::High)
        .with_action_priority(1)
        .with_evidence([FindingEvidence::new("source-file")
            .with_file("src/client/App.tsx")
            .with_specifier("node:fs")
            .with_detail("client surface imports a Node-only module")]),
    );
}

#[test]
fn analyze_project_emits_next_use_client_boundary_warning() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/boundaries/next-use-client",
    ))
    .expect("analyze next boundary fixture");

    assert_eq!(analysis.boundary_warnings.len(), 1);

    let warning = &analysis.boundary_warnings[0];
    assert_eq!(
        warning.message,
        "Client surface `app/page.tsx` imports the Node-only `node:fs` module."
    );
    assert_eq!(
        warning.recommendation,
        "Keep Node-only work on the server and pass browser-safe data into the client component."
    );
    assert_finding_metadata(
        &warning.finding,
        FindingMetadata::new(
            "boundary:server-client:fs",
            FindingAnalysisSource::SourceImport,
        )
        .with_confidence(FindingConfidence::High)
        .with_action_priority(1)
        .with_evidence([FindingEvidence::new("source-file")
            .with_file("app/page.tsx")
            .with_specifier("node:fs")
            .with_detail("client surface imports a Node-only module")]),
    );
}

#[test]
fn analyze_project_emits_next_rsc_server_only_boundary_warning() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/boundaries/rsc-server-only",
    ))
    .expect("analyze rsc boundary fixture");

    assert_eq!(analysis.boundary_warnings.len(), 1);

    let warning = &analysis.boundary_warnings[0];
    assert_eq!(
        warning.message,
        "RSC surface `app/page.tsx` imports the server-only `server-only` module."
    );
    assert_eq!(
        warning.recommendation,
        "Keep server-only guards in server-only utilities and avoid importing them directly from RSC entrypoints."
    );
    assert_finding_metadata(
        &warning.finding,
        FindingMetadata::new(
            "boundary:rsc-server-only",
            FindingAnalysisSource::SourceImport,
        )
        .with_confidence(FindingConfidence::High)
        .with_action_priority(1)
        .with_evidence([FindingEvidence::new("source-file")
            .with_file("app/page.tsx")
            .with_specifier("server-only")
            .with_detail("RSC surface imports a server-only module")]),
    );
}

#[test]
fn analyze_project_does_not_emit_next_rsc_server_only_boundary_warning_for_server_utils() {
    let temp_dir = tempdir().expect("create temp dir");
    fs::create_dir_all(temp_dir.path().join("app/lib/server")).expect("create app/lib/server");
    fs::write(
        temp_dir.path().join("package.json"),
        r#"{
  "name": "boundary-rsc-component-app",
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}
"#,
    )
    .expect("write package.json");
    fs::write(
        temp_dir.path().join("app/lib/server/db.ts"),
        "import \"server-only\";\nexport function connect() {\n  return \"ok\";\n}\n",
    )
    .expect("write server utility");
    let analysis = analyze_project(temp_dir.path()).expect("analyze next app server utility");

    assert!(analysis.boundary_warnings.is_empty());
}

fn assert_finding_metadata(actual: &FindingMetadata, expected: FindingMetadata) {
    assert_eq!(actual.finding_id, expected.finding_id);
    assert_eq!(actual.analysis_source, expected.analysis_source);
    assert_eq!(actual.confidence, expected.confidence);
    assert_eq!(actual.action_priority, expected.action_priority);
    assert_eq!(actual.evidence, expected.evidence);
}
