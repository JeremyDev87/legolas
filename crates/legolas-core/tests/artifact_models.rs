use legolas_core::artifacts::{ArtifactChunk, ArtifactModuleContribution, ArtifactSummary};

#[test]
fn artifact_namespace_exposes_models_and_parser_seams() {
    #[allow(unused_imports)]
    use legolas_core::artifacts::{detect, esbuild, rollup, webpack, ArtifactSummary};

    let _ = std::mem::size_of::<ArtifactSummary>();
}

#[test]
fn artifact_summary_normalization_produces_a_deterministic_json_shape() {
    let summary = ArtifactSummary {
        bundler: "webpack".to_string(),
        entrypoints: vec!["main".to_string(), "admin".to_string(), "main".to_string()],
        chunks: vec![
            ArtifactChunk {
                name: "vendor".to_string(),
                entrypoints: vec!["main".to_string(), "admin".to_string(), "main".to_string()],
                files: vec![
                    "dist/vendor.js".to_string(),
                    "dist/vendor.css".to_string(),
                    "dist/vendor.js".to_string(),
                ],
                initial: true,
                bytes: 64_000,
            },
            ArtifactChunk {
                name: "admin".to_string(),
                entrypoints: vec!["admin".to_string()],
                files: vec!["dist/admin.js".to_string()],
                initial: false,
                bytes: 12_000,
            },
        ],
        modules: vec![
            ArtifactModuleContribution {
                id: "src/admin.tsx".to_string(),
                package_name: None,
                chunks: vec![
                    "vendor".to_string(),
                    "admin".to_string(),
                    "admin".to_string(),
                ],
                bytes: 5_000,
            },
            ArtifactModuleContribution {
                id: "node_modules/react/index.js".to_string(),
                package_name: Some("react".to_string()),
                chunks: vec!["vendor".to_string(), "vendor".to_string()],
                bytes: 7_000,
            },
        ],
        total_bytes: 76_000,
    }
    .normalized();

    let actual = serde_json::to_string_pretty(&summary).expect("serialize artifact summary");
    let expected = r#"{
  "bundler": "webpack",
  "entrypoints": [
    "admin",
    "main"
  ],
  "chunks": [
    {
      "name": "admin",
      "entrypoints": [
        "admin"
      ],
      "files": [
        "dist/admin.js"
      ],
      "initial": false,
      "bytes": 12000
    },
    {
      "name": "vendor",
      "entrypoints": [
        "admin",
        "main"
      ],
      "files": [
        "dist/vendor.css",
        "dist/vendor.js"
      ],
      "initial": true,
      "bytes": 64000
    }
  ],
  "modules": [
    {
      "id": "node_modules/react/index.js",
      "packageName": "react",
      "chunks": [
        "vendor"
      ],
      "bytes": 7000
    },
    {
      "id": "src/admin.tsx",
      "packageName": null,
      "chunks": [
        "admin",
        "vendor"
      ],
      "bytes": 5000
    }
  ],
  "totalBytes": 76000
}"#;

    assert_eq!(actual, expected);
}
