use legolas_core::{
    DuplicatePackage, FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata,
    HeavyDependency, LazyLoadCandidate, TreeShakingWarning,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

#[test]
fn finding_metadata_serializes_as_flat_camel_case_fields() {
    let payload = serde_json::to_value(HeavyDependency {
        name: "lodash".to_string(),
        version_range: "^4.17.21".to_string(),
        estimated_kb: 26,
        category: "utility".to_string(),
        rationale: "Large helper surface".to_string(),
        recommendation: "Prefer lodash-es.".to_string(),
        imported_by: vec!["src/App.tsx".to_string()],
        dynamic_imported_by: Vec::new(),
        import_count: 1,
        finding: FindingMetadata::new("heavy-dependency", FindingAnalysisSource::SourceImport)
            .with_confidence(FindingConfidence::High)
            .with_evidence([FindingEvidence::new("source-file")
                .with_file("src/App.tsx")
                .with_specifier("lodash")
                .with_detail("static import")]),
    })
    .expect("serialize heavy dependency");

    assert_eq!(payload["findingId"], json!("heavy-dependency"));
    assert_eq!(payload["analysisSource"], json!("source-import"));
    assert_eq!(payload["confidence"], json!("high"));
    assert_eq!(
        payload["evidence"],
        json!([{
            "kind": "source-file",
            "file": "src/App.tsx",
            "specifier": "lodash",
            "detail": "static import"
        }])
    );
}

#[test]
fn default_finding_metadata_keeps_legacy_shapes_deserializable() {
    assert_default_finding_round_trip::<HeavyDependency>(
        json!({
            "name": "lodash",
            "versionRange": "^4.17.21",
            "estimatedKb": 26,
            "category": "utility",
            "rationale": "Large helper surface",
            "recommendation": "Prefer lodash-es.",
            "importedBy": ["src/App.tsx"],
            "dynamicImportedBy": [],
            "importCount": 1
        }),
        |item| &item.finding,
    );

    assert_default_finding_round_trip::<DuplicatePackage>(
        json!({
            "name": "lodash",
            "versions": ["4.17.20", "4.17.21"],
            "count": 2,
            "estimatedExtraKb": 18
        }),
        |item| &item.finding,
    );

    assert_default_finding_round_trip::<LazyLoadCandidate>(
        json!({
            "name": "chart.js",
            "estimatedSavingsKb": 48,
            "recommendation": "Split the route bundle.",
            "files": ["src/routes/Admin.tsx"],
            "reason": "Admin route is statically imported."
        }),
        |item| &item.finding,
    );

    assert_default_finding_round_trip::<TreeShakingWarning>(
        json!({
            "key": "lodash-root-import",
            "packageName": "lodash",
            "message": "Root imports can keep extra code.",
            "recommendation": "Prefer per-method imports.",
            "estimatedKb": 26,
            "files": ["src/App.tsx"]
        }),
        |item| &item.finding,
    );
}

fn assert_default_finding_round_trip<T>(legacy_payload: Value, finding: fn(&T) -> &FindingMetadata)
where
    T: DeserializeOwned + Serialize,
{
    let item: T = serde_json::from_value(legacy_payload).expect("deserialize legacy payload");
    assert_eq!(finding(&item), &FindingMetadata::default());

    let serialized = serde_json::to_value(&item).expect("serialize with default finding");
    assert!(serialized.get("findingId").is_none());
    assert!(serialized.get("analysisSource").is_none());
    assert!(serialized.get("confidence").is_none());
    assert!(serialized.get("actionPriority").is_none());
    assert!(serialized.get("recommendedFix").is_none());
    assert!(serialized.get("evidence").is_none());
}
