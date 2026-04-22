mod support;

use legolas_core::{
    analyze_project, FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata,
};

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

fn assert_finding_metadata(actual: &FindingMetadata, expected: FindingMetadata) {
    assert_eq!(actual.finding_id, expected.finding_id);
    assert_eq!(actual.analysis_source, expected.analysis_source);
    assert_eq!(actual.confidence, expected.confidence);
    assert_eq!(actual.action_priority, expected.action_priority);
    assert_eq!(actual.evidence, expected.evidence);
}
