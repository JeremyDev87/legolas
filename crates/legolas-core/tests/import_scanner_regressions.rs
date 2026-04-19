mod support;

use std::path::{Path, PathBuf};

use legolas_core::import_scanner::{collect_source_files, scan_imports, ImportedPackageRecord};

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
