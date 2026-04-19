mod support;

use std::{
    fs,
    path::{Path, PathBuf},
};

use legolas_core::{
    import_scanner::{collect_source_files, scan_imports, ImportedPackageRecord},
    TreeShakingWarning,
};
use tempfile::tempdir;

#[test]
fn collect_source_files_skips_ignored_directories_and_non_source_files() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(root, "src/App.tsx", "export const App = () => null;");
    write_file(root, "src/helpers.mts", "export const answer = 42;");
    write_file(root, "src/legacy.cts", "module.exports = {};");
    write_file(root, "src/View.vue", "<script>export default {};</script>");
    write_file(
        root,
        "src/Panel.svelte",
        "<script>export let open = false;</script>",
    );
    write_file(root, "src/styles.css", "body {}");
    write_file(
        root,
        "node_modules/pkg/index.ts",
        "export const ignored = true;",
    );
    write_file(root, "tests/unit.test.ts", "export const ignored = true;");
    write_file(root, "__tests__/unit.ts", "export const ignored = true;");
    write_file(root, "build/output.jsx", "export const ignored = true;");
    write_file(
        root,
        ".git/hooks/pre-commit.js",
        "export const ignored = true;",
    );

    let files = collect_source_files(root).expect("collect source files");
    let relative_files = to_posix_paths(root, &files);

    assert_eq!(
        relative_files,
        vec![
            "src/App.tsx",
            "src/Panel.svelte",
            "src/View.vue",
            "src/helpers.mts",
            "src/legacy.cts",
        ]
    );
}

#[test]
fn scan_imports_matches_manual_scanner_parity_expectations() {
    let root = support::fixture_path("tests/fixtures/scanner");
    let files = collect_source_files(&root).expect("collect fixture source files");

    assert_eq!(
        to_posix_paths(&root, &files),
        vec![
            "basic/Dashboard.tsx",
            "dynamic/Dashboard.tsx",
            "false-positives/docs.ts",
            "jsx/View.jsx",
            "svelte/Panel.svelte",
            "type-only/types.ts",
            "vue/Widget.vue",
        ]
    );

    let analysis = scan_imports(&root, &files).expect("scan fixture imports");

    assert_eq!(analysis.dynamic_import_count, 2);
    assert_eq!(
        analysis.by_package.keys().cloned().collect::<Vec<_>>(),
        vec![
            "@mui/material",
            "@scope/runtime",
            "@sveltejs/kit",
            "chart.js",
            "dayjs",
            "lodash",
            "lucide-react",
            "mapbox-gl",
            "react-icons",
            "vue",
        ]
    );
    assert_eq!(
        analysis.imported_packages,
        analysis.by_package.values().cloned().collect::<Vec<_>>()
    );

    assert_eq!(
        analysis.by_package.get("@mui/material"),
        Some(&ImportedPackageRecord {
            name: "@mui/material".to_string(),
            files: vec!["basic/Dashboard.tsx".to_string()],
            static_files: vec!["basic/Dashboard.tsx".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("@scope/runtime"),
        Some(&ImportedPackageRecord {
            name: "@scope/runtime".to_string(),
            files: vec!["type-only/types.ts".to_string()],
            static_files: vec!["type-only/types.ts".to_string()],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec![
                "basic/Dashboard.tsx".to_string(),
                "vue/Widget.vue".to_string()
            ],
            static_files: vec![
                "basic/Dashboard.tsx".to_string(),
                "vue/Widget.vue".to_string()
            ],
            dynamic_files: vec!["vue/Widget.vue".to_string()],
        })
    );
    assert_eq!(
        analysis.by_package.get("mapbox-gl"),
        Some(&ImportedPackageRecord {
            name: "mapbox-gl".to_string(),
            files: vec!["dynamic/Dashboard.tsx".to_string()],
            static_files: Vec::new(),
            dynamic_files: vec!["dynamic/Dashboard.tsx".to_string()],
        })
    );
    assert_eq!(
        analysis.by_package.get("react-icons"),
        Some(&ImportedPackageRecord {
            name: "react-icons".to_string(),
            files: vec![
                "basic/Dashboard.tsx".to_string(),
                "vue/Widget.vue".to_string()
            ],
            static_files: vec![
                "basic/Dashboard.tsx".to_string(),
                "vue/Widget.vue".to_string()
            ],
            dynamic_files: Vec::new(),
        })
    );
    assert!(!analysis.by_package.contains_key("react"));
    assert!(!analysis.by_package.contains_key("fake-package"));
    assert!(!analysis.by_package.contains_key("@scope/"));

    assert_eq!(
        analysis.tree_shaking_warnings,
        vec![
            TreeShakingWarning {
                key: "lodash-root-import".to_string(),
                package_name: "lodash".to_string(),
                message:
                    "Root lodash imports often keep more code than expected in client bundles."
                        .to_string(),
                recommendation: "Prefer per-method imports or lodash-es.".to_string(),
                estimated_kb: 26,
                files: vec!["basic/Dashboard.tsx".to_string()],
                finding: Default::default(),
            },
            TreeShakingWarning {
                key: "react-icons-root-import".to_string(),
                package_name: "react-icons".to_string(),
                message: "Root react-icons imports can make tree shaking unreliable.".to_string(),
                recommendation: "Import from the specific icon pack path instead.".to_string(),
                estimated_kb: 22,
                files: vec![
                    "basic/Dashboard.tsx".to_string(),
                    "vue/Widget.vue".to_string()
                ],
                finding: Default::default(),
            },
            TreeShakingWarning {
                key: "namespace-ui-import".to_string(),
                package_name: "lucide-react".to_string(),
                message: "Namespace imports pull large symbol sets into a single module graph."
                    .to_string(),
                recommendation: "Import only the symbols you need from direct subpaths."
                    .to_string(),
                estimated_kb: 35,
                files: vec!["jsx/View.jsx".to_string()],
                finding: Default::default(),
            },
        ]
    );
}

fn write_file(root: &Path, relative_path: &str, contents: &str) {
    let path = root.join(relative_path);
    let parent = path.parent().expect("fixture file parent");
    fs::create_dir_all(parent).expect("create fixture directory");
    fs::write(path, contents).expect("write fixture file");
}

fn to_posix_paths(root: &Path, files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .map(|path| {
            path.strip_prefix(root)
                .expect("relative path")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}
