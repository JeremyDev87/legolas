use std::path::Path;

use legolas_core::{
    boundaries::{collect_boundary_warnings, Phase8SeedContext},
    import_scanner::SourceAnalysis,
    workspaces::collect_workspace_summaries,
    Analysis,
};

#[test]
fn seed_workspace_hooks_default_to_empty_results() {
    let project_root = Path::new("/tmp/legolas-seed-workspace");
    let package_manager = "npm";
    let frameworks = Vec::new();
    let bundle_artifacts = Vec::new();
    let source_analysis = SourceAnalysis::default();

    let context = Phase8SeedContext {
        project_root,
        package_manager,
        frameworks: &frameworks,
        bundle_artifacts: &bundle_artifacts,
        source_analysis: &source_analysis,
        source_file_count: 0,
        imported_package_count: 0,
        dynamic_import_count: 0,
    };

    assert!(collect_boundary_warnings(&context).is_empty());
    assert!(collect_workspace_summaries(&context).is_empty());
    assert!(Analysis::default().boundary_warnings.is_empty());
    assert!(Analysis::default().workspace_summaries.is_empty());
}
