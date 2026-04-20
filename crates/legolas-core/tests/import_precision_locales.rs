mod support;

use std::{collections::BTreeMap, fs, path::Path};

use legolas_core::{
    import_scanner::{collect_source_files, scan_imports},
    FindingAnalysisSource, FindingConfidence, TreeShakingWarning,
};
use tempfile::tempdir;

#[test]
fn scan_imports_warns_on_multiple_static_locale_subpath_imports_per_package() {
    let root = support::fixture_path("tests/fixtures/import-precision/locales");
    let files = collect_source_files(&root).expect("collect fixture source files");

    let analysis = scan_imports(&root, &files).expect("scan fixture imports");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["date-fns", "dayjs", "moment"]
    );

    let warnings = analysis
        .tree_shaking_warnings
        .into_iter()
        .map(|warning| (format!("{}:{}", warning.key, warning.package_name), warning))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        warnings.keys().cloned().collect::<Vec<_>>(),
        vec![
            "import.locale-bundle:date-fns".to_string(),
            "import.locale-bundle:dayjs".to_string(),
            "import.locale-bundle:moment".to_string(),
        ]
    );

    assert_locale_warning(
        warnings
            .get("import.locale-bundle:date-fns")
            .expect("date-fns locale warning"),
        "date-fns",
    );
    assert_locale_warning(
        warnings
            .get("import.locale-bundle:dayjs")
            .expect("dayjs locale warning"),
        "dayjs",
    );
    assert_locale_warning(
        warnings
            .get("import.locale-bundle:moment")
            .expect("moment locale warning"),
        "moment",
    );
}

#[test]
fn scan_imports_does_not_warn_on_single_or_dynamic_locale_imports() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "single-locale-imports",
  "private": true,
  "dependencies": {
    "dayjs": "1.11.11",
    "moment": "2.30.1"
  }
}"#,
    );
    write_file(
        root,
        "src/DateView.tsx",
        r#"import "moment/locale/ko";
import "dayjs/locale/ko";

export function loadFrenchLocale() {
  return import("dayjs/locale/fr");
}
"#,
    );

    let files = collect_source_files(root).expect("collect source files");
    let analysis = scan_imports(root, &files).expect("scan locale imports");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["dayjs", "moment"]
    );
    assert!(analysis.tree_shaking_warnings.is_empty());
}

fn assert_locale_warning(warning: &TreeShakingWarning, expected_package_name: &str) {
    let expected_finding_id = format!("tree-shaking:import.locale-bundle:{expected_package_name}");

    assert_eq!(warning.key, "import.locale-bundle");
    assert_eq!(warning.package_name, expected_package_name);
    assert_eq!(warning.files, vec!["src/DateView.tsx".to_string()]);
    assert_eq!(
        warning.finding.analysis_source,
        Some(FindingAnalysisSource::SourceImport)
    );
    assert_eq!(warning.finding.confidence, Some(FindingConfidence::High));
    assert_eq!(
        warning.finding.finding_id.as_deref(),
        Some(expected_finding_id.as_str())
    );
    assert_eq!(warning.finding.evidence.len(), 1);
    assert_eq!(
        warning.finding.evidence[0].file.as_deref(),
        Some("src/DateView.tsx")
    );
    assert_eq!(
        warning.finding.evidence[0].specifier.as_deref(),
        Some(expected_package_name)
    );
    assert_eq!(
        warning.finding.evidence[0].detail.as_deref(),
        Some("static locale import bundle")
    );
}

fn write_file(root: &Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, contents).expect("write fixture file");
}
