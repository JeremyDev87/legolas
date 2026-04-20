mod support;

use std::{
    fs,
    path::{Path, PathBuf},
};

use legolas_core::{
    import_scanner::{collect_source_files, scan_imports, ImportedPackageRecord},
    FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata, TreeShakingWarning,
};
use tempfile::tempdir;

#[test]
fn scan_imports_ignores_import_like_text_in_comments() {
    let root = support::fixture_path("tests/fixtures/scanner/comments");
    let files = collect_source_files(&root).expect("collect comment regression files");

    assert_eq!(to_posix_paths(&root, &files), vec!["Commented.tsx"]);

    let analysis = scan_imports(&root, &files).expect("scan comment regression fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_in_raw_template_strings() {
    let root = support::fixture_path("tests/fixtures/scanner/templates");
    let files = collect_source_files(&root).expect("collect template regression files");

    assert_eq!(to_posix_paths(&root, &files), vec!["Template.tsx"]);

    let analysis = scan_imports(&root, &files).expect("scan template regression fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_counts_dynamic_imports_inside_template_interpolations() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const rendered = `${true ? import(\"chart.js/auto\") : \"\"}`;\n",
    );

    let files = collect_source_files(root).expect("collect interpolation regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files).expect("scan interpolation regression fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["chart.js"]
    );
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["src/App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["src/App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_counts_dynamic_imports_inside_template_interpolations_with_regex_literals() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const rendered = `${/}/.test(value) ? import(\"chart.js/auto\") : \"\"}`;\n",
    );

    let files = collect_source_files(root).expect("collect interpolation regex regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files).expect("scan interpolation regex regression fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["chart.js"]
    );
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["src/App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["src/App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_counts_dynamic_imports_inside_template_interpolations_with_returned_regex_literals()
{
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const rendered = `${(() => { return /}/.test(value) ? import(\"chart.js/auto\") : \"\"; })()}`;\n",
    );

    let files =
        collect_source_files(root).expect("collect interpolation return regex regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan interpolation return regex regression fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["chart.js"]
    );
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["src/App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["src/App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_ignores_import_like_text_inside_template_interpolation_regex_literals() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const rendered = `${/import(\"chart.js\\/auto\")/.test(value) ? \"hit\" : \"miss\"}`;\n",
    );

    let files =
        collect_source_files(root).expect("collect interpolation regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan interpolation regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_imports_inside_unterminated_template_interpolations() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const broken = `${(() => import(\"chart.js/auto\"))()\n",
    );

    let files =
        collect_source_files(root).expect("collect unterminated template interpolation files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan unterminated template interpolation fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_statement_regex_after_control_headers() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "export const ok = true;\n",
            "if (ok) /import(\"chart.js\\/auto\")/.test(value);\n",
            "if (ok) /=import(\"chart.js\\/auto\")/.test(value);\n",
            "do /import(\"chart.js\\/auto\")/.test(value); while (false);\n",
            "if (ok) { safe(); } else /import(\"chart.js\\/auto\")/.test(value);\n",
        ),
    );

    let files =
        collect_source_files(root).expect("collect control-header regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan control-header regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_statement_regex_after_control_headers_with_comments(
) {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const ok = true;\nif (ok) /**//import(\"chart.js\\/auto\")/.test(value);\n",
    );

    let files = collect_source_files(root)
        .expect("collect control-header comment regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files)
        .expect("scan control-header comment regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_statement_regex_after_block_boundaries() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "if (flag) { doThing(); }\n/import(\"chart.js\\/auto\")/.test(value);\n",
    );

    let files =
        collect_source_files(root).expect("collect block-boundary regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan block-boundary regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_statement_regex_after_function_declarations() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "function foo() {}\n/import(\"chart.js\\/auto\")/.test(value);\n",
    );

    let files = collect_source_files(root)
        .expect("collect function-declaration regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan function-declaration regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_statement_regex_after_class_declarations() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "class Foo {}\n/import(\"chart.js\\/auto\")/.test(value);\n",
    );

    let files =
        collect_source_files(root).expect("collect class-declaration regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan class-declaration regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_statement_regex_after_declaration_adornments() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "class Foo extends Bar {}\n",
            "/import(\"chart.js\\/auto\")/.test(value);\n",
            "function foo<T>() {}\n",
            "/import(\"chart.js\\/auto\")/.test(value);\n",
            "function* bar() {}\n",
            "/import(\"chart.js\\/auto\")/.test(value);\n",
        ),
    );

    let files = collect_source_files(root)
        .expect("collect declaration adornment regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files)
        .expect("scan declaration adornment regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_statement_regex_after_catch_blocks() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "try { risky(); } catch {}\n/import(\"chart.js\\/auto\")/.test(value);\n",
    );

    let files = collect_source_files(root).expect("collect catch-block regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan catch-block regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_statement_regex_after_for_await_headers() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "async function main() {\n",
            "  for await (const item of items) {}\n",
            "  /import(\"chart.js\\/auto\")/.test(item);\n",
            "}\n",
        ),
    );

    let files = collect_source_files(root).expect("collect for-await regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files).expect("scan for-await regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_inside_for_of_headers() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "for (const item of /import(\"chart.js\\/auto\")/.exec(value) ?? []) {\n",
            "  console.log(item);\n",
            "}\n",
        ),
    );

    let files = collect_source_files(root).expect("collect for-of regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files).expect("scan for-of regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_preserves_dynamic_imports_after_object_literal_statements_followed_by_division() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "const objectLiteral = {}\n/import(\"chart\") / 2;\n",
    );

    let files =
        collect_source_files(root).expect("collect object-literal division regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files).expect("scan object-literal division fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert!(analysis.by_package.contains_key("chart"));
}

#[test]
fn scan_imports_ignores_import_like_text_after_arrow_function_statements() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "const render = () => {}\n",
            "/import(\"chart.js\\/auto\")/.test(render);\n",
        ),
    );

    let files =
        collect_source_files(root).expect("collect arrow-function regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files).expect("scan arrow-function regex fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_ignores_import_like_text_after_arrow_function_assignments() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "exports.loader = () => {}\n",
            "/import(\"chart.js\\/auto\")/.test(exports.loader);\n",
        ),
    );

    let files =
        collect_source_files(root).expect("collect arrow-assignment regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan arrow-assignment regex false-positive fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_preserves_dynamic_imports_after_function_expression_assignments_followed_by_division(
) {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "exports.fallback = function () {}\n/import(\"chart\") / 2;\n",
    );

    let files = collect_source_files(root)
        .expect("collect function-expression assignment division regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files)
        .expect("scan function-expression assignment division regression fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert!(analysis.by_package.contains_key("chart"));
}

#[test]
fn scan_imports_ignores_import_like_text_after_labeled_block_statements() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "label: {}\n",
            "/import(\"chart.js\\/auto\")/.test(label);\n",
        ),
    );

    let files =
        collect_source_files(root).expect("collect labeled-block regex false-positive files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files).expect("scan labeled-block regex fixture");

    assert!(analysis.by_package.is_empty());
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
}

#[test]
fn scan_imports_preserves_dynamic_imports_after_function_bodies_followed_by_division() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const value = (() => { return 1; }) / import(\"chart.js/auto\") / 2;\n",
    );

    let files =
        collect_source_files(root).expect("collect function-body division regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan function-body division regression fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["src/App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["src/App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_preserves_dynamic_imports_after_url_strings_followed_by_division() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const value = \"https://cdn.example.com\" / import(\"chart.js/auto\") / 2;\n",
    );

    let files = collect_source_files(root).expect("collect url-string division regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files).expect("scan url-string division regression fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["src/App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["src/App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_preserves_dynamic_imports_after_url_template_strings_followed_by_division() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const value = `https://cdn.example.com` / import(\"chart.js/auto\") / 2;\n",
    );

    let files =
        collect_source_files(root).expect("collect url-template-string division regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan url-template-string division regression fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["src/App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["src/App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_preserves_dynamic_imports_after_object_literal_exports_followed_by_division() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        "export const value = {}\n/import(\"chart.js/auto\") / 2;\n",
    );

    let files = collect_source_files(root)
        .expect("collect object-literal export division regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan object-literal export division regression fixture");

    assert_eq!(analysis.dynamic_import_count, 1);
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["src/App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["src/App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_preserves_dynamic_imports_after_semicolon_less_object_literal_returns_and_exports_followed_by_division(
) {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "function demo() {\n",
            "  return {}\n",
            "  / import(\"chart\") / 2;\n",
            "}\n",
            "function regexChainDemo() {\n",
            "  return {}\n",
            "  / import(\"chart\") / /2/.test(value);\n",
            "}\n",
            "export default {}\n",
            "/ import(\"chart\") / /2/.test(value);\n",
        ),
    );

    let files = collect_source_files(root)
        .expect("collect semicolon-less object literal division regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis = scan_imports(root, &files)
        .expect("scan semicolon-less object literal division regression fixture");

    assert_eq!(analysis.dynamic_import_count, 3);
    assert!(analysis.by_package.contains_key("chart"));
}

#[test]
fn scan_imports_preserves_dynamic_imports_after_postfix_increment_and_decrement() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/App.tsx",
        concat!(
            "let count = 0;\n",
            "export const plus = count++ / import(\"chart.js/auto\") / 2;\n",
            "export const minus = count-- / import(\"chart.js/auto\") / 2;\n",
        ),
    );

    let files =
        collect_source_files(root).expect("collect postfix increment division regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/App.tsx"]);

    let analysis =
        scan_imports(root, &files).expect("scan postfix increment division regression fixture");

    assert_eq!(analysis.dynamic_import_count, 2);
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["src/App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["src/App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_emits_tree_shaking_warnings_for_export_from_root_packages() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/index.ts",
        "export { default as fp } from \"lodash\";\n",
    );

    let files = collect_source_files(root).expect("collect export tree-shaking regression files");

    assert_eq!(to_posix_paths(root, &files), vec!["src/index.ts"]);

    let analysis = scan_imports(root, &files).expect("scan export tree-shaking regression fixture");

    assert_eq!(
        analysis.by_package.get("lodash"),
        Some(&ImportedPackageRecord {
            name: "lodash".to_string(),
            files: vec!["src/index.ts".to_string()],
            static_files: vec!["src/index.ts".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(analysis.dynamic_import_count, 0);
    assert_eq!(
        analysis.tree_shaking_warnings,
        vec![TreeShakingWarning {
            key: "lodash-root-import".to_string(),
            package_name: "lodash".to_string(),
            message: "Root lodash imports often keep more code than expected in client bundles."
                .to_string(),
            recommendation: "Prefer per-method imports or lodash-es.".to_string(),
            estimated_kb: 26,
            files: vec!["src/index.ts".to_string()],
            finding: FindingMetadata::new(
                "tree-shaking:lodash-root-import",
                FindingAnalysisSource::SourceImport,
            )
            .with_confidence(FindingConfidence::High)
            .with_evidence([FindingEvidence::new("source-file")
                .with_file("src/index.ts")
                .with_specifier("lodash")
                .with_detail("root package import")]),
        }]
    );
}

#[test]
fn scan_imports_tracks_export_from_reexports_without_type_only_exports() {
    let root = support::fixture_path("tests/fixtures/scanner/reexport");
    let files = collect_source_files(&root).expect("collect reexport regression files");

    assert_eq!(to_posix_paths(&root, &files), vec!["index.ts"]);

    let analysis = scan_imports(&root, &files).expect("scan reexport regression fixture");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["@scope/runtime", "chart.js", "dayjs"]
    );
    assert_eq!(analysis.dynamic_import_count, 0);
    assert!(analysis.tree_shaking_warnings.is_empty());
    assert_eq!(
        analysis.by_package.get("@scope/runtime"),
        Some(&ImportedPackageRecord {
            name: "@scope/runtime".to_string(),
            files: vec!["index.ts".to_string()],
            static_files: vec!["index.ts".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["index.ts".to_string()],
            static_files: vec!["index.ts".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("dayjs"),
        Some(&ImportedPackageRecord {
            name: "dayjs".to_string(),
            files: vec!["index.ts".to_string()],
            static_files: vec!["index.ts".to_string()],
            dynamic_files: Vec::new(),
        })
    );
}

#[test]
fn scan_imports_counts_nested_dynamic_imports() {
    let root = support::fixture_path("tests/fixtures/scanner/nested-dynamic");
    let files = collect_source_files(&root).expect("collect nested dynamic regression files");

    assert_eq!(to_posix_paths(&root, &files), vec!["App.tsx"]);

    let analysis = scan_imports(&root, &files).expect("scan nested dynamic regression fixture");

    assert_eq!(analysis.dynamic_import_count, 2);
    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["chart.js", "mapbox-gl"]
    );
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec!["App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["App.tsx".to_string()],
        })
    );
    assert_eq!(
        analysis.by_package.get("mapbox-gl"),
        Some(&ImportedPackageRecord {
            name: "mapbox-gl".to_string(),
            files: vec!["App.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["App.tsx".to_string()],
        })
    );
}

#[test]
fn scan_imports_reads_script_blocks_with_embedded_closing_tags_in_strings_and_comments() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "src/Widget.vue",
        concat!(
            "<script>\n",
            "const html = \"</script>\";\n",
            "import { reactive } from \"vue\";\n",
            "</script>\n",
        ),
    );
    write_file(
        root,
        "src/Panel.svelte",
        concat!(
            "<script>\n",
            "// </script>\n",
            "import dayjs from \"dayjs\";\n",
            "</script>\n",
        ),
    );

    let files = collect_source_files(root).expect("collect embedded closing-tag regression files");

    assert_eq!(
        to_posix_paths(root, &files),
        vec!["src/Panel.svelte", "src/Widget.vue"]
    );

    let analysis = scan_imports(root, &files).expect("scan embedded closing-tag regression files");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["dayjs", "vue"]
    );
    assert_eq!(analysis.dynamic_import_count, 0);
    assert_eq!(
        analysis.by_package.get("dayjs"),
        Some(&ImportedPackageRecord {
            name: "dayjs".to_string(),
            files: vec!["src/Panel.svelte".to_string()],
            static_files: vec!["src/Panel.svelte".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("vue"),
        Some(&ImportedPackageRecord {
            name: "vue".to_string(),
            files: vec!["src/Widget.vue".to_string()],
            static_files: vec!["src/Widget.vue".to_string()],
            dynamic_files: Vec::new(),
        })
    );
}

#[test]
fn scan_imports_reads_both_vue_script_blocks() {
    let root = support::fixture_path("tests/fixtures/scanner/vue-multiscript");
    let files = collect_source_files(&root).expect("collect vue multiscript regression files");

    assert_eq!(to_posix_paths(&root, &files), vec!["Widget.vue"]);

    let analysis = scan_imports(&root, &files).expect("scan vue multiscript regression fixture");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["@scope/runtime", "vue"]
    );
    assert_eq!(analysis.dynamic_import_count, 0);
    assert_eq!(
        analysis.by_package.get("@scope/runtime"),
        Some(&ImportedPackageRecord {
            name: "@scope/runtime".to_string(),
            files: vec!["Widget.vue".to_string()],
            static_files: vec!["Widget.vue".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("vue"),
        Some(&ImportedPackageRecord {
            name: "vue".to_string(),
            files: vec!["Widget.vue".to_string()],
            static_files: vec!["Widget.vue".to_string()],
            dynamic_files: Vec::new(),
        })
    );
}

#[test]
fn scan_imports_reads_svelte_context_and_instance_scripts() {
    let root = support::fixture_path("tests/fixtures/scanner/svelte-context");
    let files = collect_source_files(&root).expect("collect svelte context regression files");

    assert_eq!(to_posix_paths(&root, &files), vec!["Panel.svelte"]);

    let analysis = scan_imports(&root, &files).expect("scan svelte context regression fixture");

    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec!["@sveltejs/kit", "dayjs"]
    );
    assert_eq!(analysis.dynamic_import_count, 0);
    assert_eq!(
        analysis.by_package.get("@sveltejs/kit"),
        Some(&ImportedPackageRecord {
            name: "@sveltejs/kit".to_string(),
            files: vec!["Panel.svelte".to_string()],
            static_files: vec!["Panel.svelte".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("dayjs"),
        Some(&ImportedPackageRecord {
            name: "dayjs".to_string(),
            files: vec!["Panel.svelte".to_string()],
            static_files: vec!["Panel.svelte".to_string()],
            dynamic_files: Vec::new(),
        })
    );
}

fn to_posix_paths(root: &Path, files: &[PathBuf]) -> Vec<String> {
    let mut relative_paths = files
        .iter()
        .map(|file| {
            file.strip_prefix(root)
                .expect("source file should stay under fixture root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    relative_paths.sort();
    relative_paths
}

fn write_file(root: &Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    let parent = path.parent().expect("fixture file parent");
    fs::create_dir_all(parent).expect("create fixture directory");
    fs::write(path, contents).expect("write fixture file");
}
