mod support;

use std::collections::BTreeMap;

use legolas_core::{
    import_scanner::{collect_source_files, scan_imports, ImportedPackageRecord},
    FindingAnalysisSource, FindingConfidence, TreeShakingWarning,
};

#[test]
fn scan_imports_warns_on_root_and_namespace_icon_pack_imports_only() {
    let root = support::fixture_path("tests/fixtures/import-precision/icons");
    let files = collect_source_files(&root).expect("collect fixture source files");

    let analysis = scan_imports(&root, &files).expect("scan fixture imports");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["@mui/icons-material", "react-icons"]
    );
    assert_eq!(
        analysis.by_package.get("@mui/icons-material"),
        Some(&ImportedPackageRecord {
            name: "@mui/icons-material".to_string(),
            files: vec!["src/IconPanel.tsx".to_string()],
            static_files: vec!["src/IconPanel.tsx".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("react-icons"),
        Some(&ImportedPackageRecord {
            name: "react-icons".to_string(),
            files: vec!["src/IconPanel.tsx".to_string()],
            static_files: vec!["src/IconPanel.tsx".to_string()],
            dynamic_files: Vec::new(),
        })
    );

    let warnings = analysis
        .tree_shaking_warnings
        .into_iter()
        .map(|warning| (format!("{}:{}", warning.key, warning.package_name), warning))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        warnings.keys().cloned().collect::<Vec<_>>(),
        vec![
            "mui-icons-namespace-import:@mui/icons-material".to_string(),
            "react-icons-pack-namespace-import:react-icons/fa".to_string(),
            "react-icons-pack-namespace-import:react-icons/fi".to_string(),
            "react-icons-root-import:react-icons".to_string(),
        ]
    );

    assert_warning(
        warnings
            .get("mui-icons-namespace-import:@mui/icons-material")
            .expect("mui icons namespace warning"),
        "mui-icons-namespace-import",
        "@mui/icons-material",
        "tree-shaking:mui-icons-namespace-import:@mui/icons-material",
        "icon pack namespace import",
    );
    assert_warning(
        warnings
            .get("react-icons-pack-namespace-import:react-icons/fa")
            .expect("react icons namespace warning"),
        "react-icons-pack-namespace-import",
        "react-icons/fa",
        "tree-shaking:react-icons-pack-namespace-import:react-icons/fa",
        "icon pack namespace import",
    );
    assert_warning(
        warnings
            .get("react-icons-pack-namespace-import:react-icons/fi")
            .expect("react icons fi namespace warning"),
        "react-icons-pack-namespace-import",
        "react-icons/fi",
        "tree-shaking:react-icons-pack-namespace-import:react-icons/fi",
        "icon pack namespace import",
    );
    assert_warning(
        warnings
            .get("react-icons-root-import:react-icons")
            .expect("react icons root warning"),
        "react-icons-root-import",
        "react-icons",
        "tree-shaking:react-icons-root-import",
        "root package import",
    );

    let finding_ids = warnings
        .values()
        .map(|warning| warning.finding.finding_id.as_deref().expect("finding id"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(finding_ids.len(), warnings.len());
}

fn assert_warning(
    warning: &TreeShakingWarning,
    expected_key: &str,
    expected_package_name: &str,
    expected_finding_id: &str,
    expected_detail: &str,
) {
    assert_eq!(warning.key, expected_key);
    assert_eq!(warning.package_name, expected_package_name);
    assert_eq!(warning.files, vec!["src/IconPanel.tsx".to_string()]);
    assert_eq!(
        warning.finding.analysis_source,
        Some(FindingAnalysisSource::SourceImport)
    );
    assert_eq!(warning.finding.confidence, Some(FindingConfidence::High));
    assert_eq!(
        warning.finding.finding_id.as_deref(),
        Some(expected_finding_id)
    );
    assert_eq!(warning.finding.evidence.len(), 1);
    assert_eq!(
        warning.finding.evidence[0].file.as_deref(),
        Some("src/IconPanel.tsx")
    );
    assert_eq!(
        warning.finding.evidence[0].specifier.as_deref(),
        Some(expected_package_name)
    );
    assert_eq!(
        warning.finding.evidence[0].detail.as_deref(),
        Some(expected_detail)
    );
}
