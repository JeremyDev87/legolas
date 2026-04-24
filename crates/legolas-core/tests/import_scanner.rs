mod support;

use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::symlink as create_dir_symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_dir as create_dir_symlink;

use legolas_core::{
    import_scanner::{
        collect_source_files, scan_imports, scan_imports_with_aliases, ImportedPackageRecord,
    },
    workspace::load_alias_config,
    FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata, TreeShakingWarning,
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
    for directory in [
        "target",
        ".cache",
        ".parcel-cache",
        ".vite",
        ".svelte-kit",
        ".nuxt",
        ".yarn",
        ".pnpm-store",
        "vendor",
        "tmp",
        "temp",
    ] {
        write_file(
            root,
            &format!("{directory}/generated.tsx"),
            "export const ignored = true;",
        );
    }
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
            "comments/Commented.tsx",
            "dynamic/Dashboard.tsx",
            "false-positives/docs.ts",
            "jsx/View.jsx",
            "nested-dynamic/App.tsx",
            "reexport/index.ts",
            "svelte/Panel.svelte",
            "svelte-context/Panel.svelte",
            "templates/Template.tsx",
            "type-only/types.ts",
            "vue/Widget.vue",
            "vue-multiscript/Widget.vue",
        ]
    );

    let analysis = scan_imports(&root, &files).expect("scan fixture imports");

    assert_eq!(analysis.dynamic_import_count, 6);
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
            files: vec![
                "reexport/index.ts".to_string(),
                "type-only/types.ts".to_string(),
                "vue-multiscript/Widget.vue".to_string(),
            ],
            static_files: vec![
                "reexport/index.ts".to_string(),
                "type-only/types.ts".to_string(),
                "vue-multiscript/Widget.vue".to_string(),
            ],
            dynamic_files: Vec::new(),
        })
    );
    assert_eq!(
        analysis.by_package.get("chart.js"),
        Some(&ImportedPackageRecord {
            name: "chart.js".to_string(),
            files: vec![
                "basic/Dashboard.tsx".to_string(),
                "nested-dynamic/App.tsx".to_string(),
                "reexport/index.ts".to_string(),
                "vue/Widget.vue".to_string()
            ],
            static_files: vec![
                "basic/Dashboard.tsx".to_string(),
                "reexport/index.ts".to_string(),
                "vue/Widget.vue".to_string()
            ],
            dynamic_files: vec![
                "nested-dynamic/App.tsx".to_string(),
                "vue/Widget.vue".to_string()
            ],
        })
    );
    assert_eq!(
        analysis.by_package.get("mapbox-gl"),
        Some(&ImportedPackageRecord {
            name: "mapbox-gl".to_string(),
            files: vec![
                "dynamic/Dashboard.tsx".to_string(),
                "nested-dynamic/App.tsx".to_string()
            ],
            static_files: Vec::new(),
            dynamic_files: vec![
                "dynamic/Dashboard.tsx".to_string(),
                "nested-dynamic/App.tsx".to_string()
            ],
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
                finding: FindingMetadata::new(
                    "tree-shaking:lodash-root-import",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::High)
                .with_evidence([FindingEvidence::new("source-file")
                    .with_file("basic/Dashboard.tsx")
                    .with_specifier("lodash")
                    .with_detail("root package import")]),
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
                finding: FindingMetadata::new(
                    "tree-shaking:react-icons-root-import",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::High)
                .with_evidence([
                    FindingEvidence::new("source-file")
                        .with_file("basic/Dashboard.tsx")
                        .with_specifier("react-icons")
                        .with_detail("root package import"),
                    FindingEvidence::new("source-file")
                        .with_file("vue/Widget.vue")
                        .with_specifier("react-icons")
                        .with_detail("root package import"),
                ]),
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
                finding: FindingMetadata::new(
                    "tree-shaking:namespace-ui-import",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::High)
                .with_evidence([FindingEvidence::new("source-file")
                    .with_file("jsx/View.jsx")
                    .with_specifier("lucide-react")
                    .with_detail("namespace import")]),
            },
        ]
    );
}

#[test]
fn scan_imports_excludes_tsconfig_backed_local_aliases_from_package_usage() {
    let root = support::fixture_path("tests/fixtures/aliases/tsconfig-paths");
    let files = collect_source_files(&root).expect("collect tsconfig fixture source files");
    let alias_config = load_alias_config(&root)
        .expect("load tsconfig alias config")
        .expect("tsconfig alias config should exist");

    assert_eq!(
        to_posix_paths(&root, &files),
        vec![
            "src/App.tsx",
            "src/components/Button.tsx",
            "src/shared/fallback.ts",
            "src/shared/index.ts",
        ]
    );

    let legacy = scan_imports(&root, &files).expect("scan imports without alias config");
    let alias_aware = scan_imports_with_aliases(&root, &files, Some(&alias_config.config))
        .expect("scan imports with alias config");

    assert!(legacy.by_package.contains_key("components"));
    assert!(legacy.by_package.contains_key("chart.js"));
    assert!(!alias_aware.by_package.contains_key("components"));
    assert!(alias_aware.by_package.contains_key("chart.js"));
    assert_eq!(alias_aware.imported_packages.len(), 1);
    assert_eq!(alias_aware.dynamic_import_count, 0);
}

#[test]
fn scan_imports_excludes_jsconfig_backed_exact_aliases_from_package_usage() {
    let root = support::fixture_path("tests/fixtures/aliases/jsconfig-paths");
    let files = collect_source_files(&root).expect("collect jsconfig fixture source files");
    let alias_config = load_alias_config(&root)
        .expect("load jsconfig alias config")
        .expect("jsconfig alias config should exist");

    assert_eq!(
        to_posix_paths(&root, &files),
        vec!["src/config/env.js", "src/index.jsx"]
    );

    let legacy = scan_imports(&root, &files).expect("scan imports without alias config");
    let alias_aware = scan_imports_with_aliases(&root, &files, Some(&alias_config.config))
        .expect("scan imports with alias config");

    assert!(legacy.by_package.contains_key("env"));
    assert!(legacy.by_package.contains_key("react-icons"));
    assert!(!alias_aware.by_package.contains_key("env"));
    assert!(alias_aware.by_package.contains_key("react-icons"));
    assert_eq!(alias_aware.imported_packages.len(), 1);
}

#[test]
fn scan_imports_excludes_middle_wildcard_alias_patterns_from_package_usage() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    write_file(
        root,
        "package.json",
        r#"{
  "name": "middle-wildcard-alias-app",
  "private": true
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "components/*/public": ["src/components/*/index"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "import Button from \"components/button/public\";\nexport default Button;\n",
    );
    write_file(
        root,
        "src/components/button/index.ts",
        "const Button = 'button';\nexport default Button;\n",
    );

    let files = collect_source_files(root).expect("collect middle-wildcard source files");
    let alias_config = load_alias_config(root)
        .expect("load middle-wildcard alias config")
        .expect("middle-wildcard alias config should exist");

    let legacy = scan_imports(root, &files).expect("scan imports without alias config");
    let alias_aware = scan_imports_with_aliases(root, &files, Some(&alias_config.config))
        .expect("scan imports with alias config");

    assert!(legacy.by_package.contains_key("components"));
    assert!(alias_aware.by_package.is_empty());
    assert_eq!(alias_aware.imported_packages.len(), 0);
    assert_eq!(alias_aware.dynamic_import_count, 0);
}

#[test]
fn scan_imports_keeps_node_modules_package_remaps_counted_as_packages() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    write_file(
        root,
        "package.json",
        r#"{
  "name": "package-remap-app",
  "private": true
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "react": ["node_modules/preact/compat"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "import { h } from \"react\";\nexport const App = h;\n",
    );
    write_file(
        root,
        "node_modules/preact/compat/index.js",
        "export const h = () => null;\n",
    );

    let files = collect_source_files(root).expect("collect package-remap source files");
    let alias_config = load_alias_config(root)
        .expect("load package-remap alias config")
        .expect("package-remap alias config should exist");
    let alias_aware = scan_imports_with_aliases(root, &files, Some(&alias_config.config))
        .expect("scan imports with alias config");

    assert!(alias_aware.by_package.contains_key("react"));
    assert_eq!(alias_aware.imported_packages.len(), 1);
}

#[test]
fn scan_imports_counts_dynamic_entries_even_when_aliases_are_local() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    write_file(
        root,
        "package.json",
        r#"{
  "name": "dynamic-alias-app",
  "private": true
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "routes/*": ["src/routes/*"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "export async function load() {\n  await import(\"routes/dashboard\");\n  await import(\"./local\");\n}\n",
    );
    write_file(
        root,
        "src/routes/dashboard.tsx",
        "export default 'dashboard';\n",
    );
    write_file(root, "src/local.ts", "export default 'local';\n");

    let files = collect_source_files(root).expect("collect dynamic-alias source files");
    let alias_config = load_alias_config(root)
        .expect("load dynamic-alias alias config")
        .expect("dynamic-alias alias config should exist");
    let alias_aware = scan_imports_with_aliases(root, &files, Some(&alias_config.config))
        .expect("scan imports with alias config");

    assert_eq!(alias_aware.imported_packages.len(), 0);
    assert_eq!(alias_aware.dynamic_import_count, 2);
}

#[test]
fn scan_imports_keeps_symlinked_package_remaps_counted_as_packages() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();
    write_file(
        root,
        "package.json",
        r#"{
  "name": "symlink-package-remap-app",
  "private": true
}"#,
    );
    write_file(
        root,
        "tsconfig.json",
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "react": ["vendor/react"]
    }
  }
}"#,
    );
    write_file(
        root,
        "src/App.tsx",
        "import { h } from \"react\";\nexport const App = h;\n",
    );
    write_file(
        root,
        "node_modules/preact/compat/index.js",
        "export const h = () => null;\n",
    );
    fs::create_dir_all(root.join("vendor")).expect("create vendor dir");
    create_dir_symlink(
        root.join("node_modules/preact/compat"),
        root.join("vendor/react"),
    )
    .expect("create vendor symlink");

    let files = collect_source_files(root).expect("collect symlink-remap source files");
    let alias_config = load_alias_config(root)
        .expect("load symlink-remap alias config")
        .expect("symlink-remap alias config should exist");
    let alias_aware = scan_imports_with_aliases(root, &files, Some(&alias_config.config))
        .expect("scan imports with alias config");

    assert!(alias_aware.by_package.contains_key("react"));
    assert_eq!(alias_aware.imported_packages.len(), 1);
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
