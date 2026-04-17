use std::path::PathBuf;

use legolas_core::Analysis;

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

pub fn fixture_path(relative_path: &str) -> PathBuf {
    workspace_root().join(relative_path)
}

pub fn read_oracle(relative_path: &str) -> String {
    std::fs::read_to_string(workspace_root().join("tests/oracles").join(relative_path))
        .expect("read oracle")
}

pub fn normalize_analysis_for_oracle(analysis: &Analysis) -> String {
    let mut normalized = analysis.clone();

    normalized.project_root = "<PROJECT_ROOT>".to_string();
    normalized.bundle_artifacts = normalized
        .bundle_artifacts
        .into_iter()
        .map(to_posix)
        .collect();
    normalized.heavy_dependencies = normalized
        .heavy_dependencies
        .into_iter()
        .map(|mut item| {
            item.imported_by = item.imported_by.into_iter().map(to_posix).collect();
            item.dynamic_imported_by = item.dynamic_imported_by.into_iter().map(to_posix).collect();
            item
        })
        .collect();
    normalized.lazy_load_candidates = normalized
        .lazy_load_candidates
        .into_iter()
        .map(|mut item| {
            item.files = item.files.into_iter().map(to_posix).collect();
            item
        })
        .collect();
    normalized.tree_shaking_warnings = normalized
        .tree_shaking_warnings
        .into_iter()
        .map(|mut item| {
            item.files = item.files.into_iter().map(to_posix).collect();
            item
        })
        .collect();
    normalized.warnings = normalized.warnings.into_iter().map(to_posix).collect();
    normalized.metadata.generated_at = "<GENERATED_AT>".to_string();

    format!(
        "{}\n",
        serde_json::to_string_pretty(&normalized).expect("serialize analysis")
    )
}

fn to_posix(value: String) -> String {
    value.replace('\\', "/")
}
