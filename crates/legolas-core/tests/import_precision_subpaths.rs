mod support;

use std::{collections::BTreeMap, fs, path::Path};

use legolas_core::{
    import_scanner::{collect_source_files, scan_imports},
    FindingAnalysisSource, FindingConfidence, TreeShakingWarning,
};
use tempfile::tempdir;

#[test]
fn scan_imports_keeps_lodash_root_warning_without_flagging_safe_root_or_precise_subpaths() {
    let root = support::fixture_path("tests/fixtures/import-precision/subpaths");
    let files = collect_source_files(&root).expect("collect fixture source files");

    let analysis = scan_imports(&root, &files).expect("scan fixture imports");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["@mui/material", "date-fns", "lodash"]
    );

    let warnings = analysis
        .tree_shaking_warnings
        .into_iter()
        .map(|warning| (warning.package_name.clone(), warning))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(warnings.keys().cloned().collect::<Vec<_>>(), vec!["lodash"]);
    assert_root_barrel_warning(
        warnings.get("lodash").expect("lodash warning"),
        "lodash-root-import",
        "tree-shaking:lodash-root-import",
        "Prefer per-method imports or lodash-es.",
    );
}

#[test]
fn scan_imports_skips_tree_shaking_warnings_for_precise_subpaths_only() {
    let root = support::fixture_path("tests/fixtures/import-precision/subpaths-only");
    let files = collect_source_files(&root).expect("collect fixture source files");

    let analysis = scan_imports(&root, &files).expect("scan fixture imports");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["@mui/material", "date-fns", "lodash"]
    );
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_malformed_root_barrel_imports() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "malformed-root-barrels",
  "private": true
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        r#"import from "lodash";
import from "date-fns";
import from "@mui/material";
"#,
    );

    let files = collect_source_files(root).expect("collect source files");
    let analysis = scan_imports(root, &files).expect("scan malformed imports");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["@mui/material", "date-fns", "lodash"]
    );
    assert!(analysis.tree_shaking_warnings.is_empty());
}

fn assert_root_barrel_warning(
    warning: &TreeShakingWarning,
    expected_key: &str,
    expected_finding_id: &str,
    expected_recommendation: &str,
) {
    assert_eq!(warning.key, expected_key);
    assert_eq!(warning.files, vec!["src/App.tsx".to_string()]);
    assert_eq!(warning.recommendation, expected_recommendation);

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
        Some("src/App.tsx")
    );
    assert_eq!(
        warning.finding.evidence[0].specifier.as_deref(),
        Some(warning.package_name.as_str())
    );
    assert_eq!(
        warning.finding.evidence[0].detail.as_deref(),
        Some("root package import")
    );
}

fn write_file(root: &Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, contents).expect("write fixture file");
}
