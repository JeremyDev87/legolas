mod support;

use legolas_core::{
    analyze_project,
    boundaries::Phase8SeedContext,
    import_scanner::SourceAnalysis,
    workspaces::{collect_workspace_summaries, WorkspaceSummary},
};
use tempfile::tempdir;

#[test]
fn analyze_project_collects_workspace_summaries_for_pnpm_monorepos() {
    let analysis = analyze_project(support::fixture_path(
        "tests/fixtures/monorepo/pnpm-workspace",
    ))
    .expect("analyze pnpm workspace fixture");

    assert_eq!(
        analysis.workspace_summaries,
        vec![
            WorkspaceSummary {
                name: "admin-app".to_string(),
                path: "apps/admin".to_string(),
                imported_packages: 3,
                heavy_dependencies: 2,
                duplicate_packages: 0,
                potential_kb_saved: 42,
            },
            WorkspaceSummary {
                name: "storefront-app".to_string(),
                path: "apps/storefront".to_string(),
                imported_packages: 2,
                heavy_dependencies: 1,
                duplicate_packages: 0,
                potential_kb_saved: 13,
            },
        ]
    );
}

#[test]
fn collect_workspace_summaries_returns_empty_without_workspace_patterns() {
    let temp_dir = tempdir().expect("create temp dir");
    std::fs::write(
        temp_dir.path().join("package.json"),
        r#"{
  "name": "solo-app"
}"#,
    )
    .expect("write package.json");

    let frameworks = Vec::new();
    let bundle_artifacts = Vec::new();
    let source_analysis = SourceAnalysis::default();
    let context = Phase8SeedContext {
        project_root: temp_dir.path(),
        package_manager: "pnpm",
        frameworks: &frameworks,
        bundle_artifacts: &bundle_artifacts,
        source_analysis: &source_analysis,
        source_file_count: 0,
        imported_package_count: 0,
        dynamic_import_count: 0,
    };

    assert!(collect_workspace_summaries(&context).is_empty());
}
