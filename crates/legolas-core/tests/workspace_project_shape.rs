#[allow(dead_code)]
mod support;

use std::fs;

use legolas_core::{
    project_shape::{detect_frameworks, detect_package_manager},
    workspace::{exists, find_project_root, read_json_if_exists, read_text_if_exists},
};
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn find_project_root_normalizes_file_inputs_and_ascends_to_nearest_marker() {
    let file_input = support::fixture_path("tests/fixtures/workspace/file-input/example.ts");
    let expected_root = support::workspace_root();

    assert_eq!(
        find_project_root(&file_input).expect("find project root"),
        expected_root
    );
}

#[test]
fn find_project_root_returns_a_stable_missing_path_error() {
    let missing = support::fixture_path("tests/fixtures/workspace/file-input/missing.ts");
    let error = find_project_root(&missing).expect_err("missing path should fail");

    assert_eq!(
        error.to_string(),
        format!("path not found: {}", missing.display())
    );
}

#[test]
fn read_helpers_treat_missing_entries_as_optional() {
    let project_root = support::fixture_path("tests/fixtures/workspace/vite-project");
    let manifest: Value = read_json_if_exists(project_root.join("package.json"))
        .expect("read manifest")
        .expect("package.json should exist");

    assert_eq!(manifest["name"], "workspace-vite-project");
    assert_eq!(
        read_text_if_exists(project_root.join("missing.txt")).expect("read missing text"),
        None
    );
    assert!(!exists(project_root.join("missing.txt")).expect("check missing file"));
    assert!(exists(project_root.join("vite.config.ts")).expect("check config file"));
}

#[test]
fn read_json_if_exists_treats_empty_files_as_missing() {
    let temp_dir = tempdir().expect("create temp dir");
    let empty_json = temp_dir.path().join("package-lock.json");
    fs::write(&empty_json, "").expect("write empty json");

    let parsed: Option<Value> = read_json_if_exists(&empty_json).expect("read empty json");

    assert_eq!(parsed, None);
}

#[cfg(unix)]
#[test]
fn exists_swallows_non_not_found_filesystem_errors() {
    use std::{os::unix::fs::PermissionsExt, path::PathBuf};

    struct PermissionGuard {
        path: PathBuf,
        permissions: fs::Permissions,
    }

    impl Drop for PermissionGuard {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.path, self.permissions.clone());
        }
    }

    let temp_dir = tempdir().expect("create temp dir");
    let sealed_dir = temp_dir.path().join("sealed");
    fs::create_dir(&sealed_dir).expect("create sealed dir");
    fs::write(sealed_dir.join("package.json"), "{}").expect("write package.json");

    let original_permissions = fs::metadata(&sealed_dir)
        .expect("read metadata")
        .permissions();
    let _permission_guard = PermissionGuard {
        path: sealed_dir.clone(),
        permissions: original_permissions.clone(),
    };
    let mut sealed_permissions = original_permissions.clone();
    sealed_permissions.set_mode(0o000);
    fs::set_permissions(&sealed_dir, sealed_permissions).expect("seal directory");

    if fs::metadata(sealed_dir.join("package.json")).is_ok() {
        return;
    }

    let result = exists(sealed_dir.join("package.json")).expect("exists should not fail");

    assert!(!result);
}

#[test]
fn detect_frameworks_matches_js_order_and_config_only_hits() {
    let project_root = support::fixture_path("tests/fixtures/workspace/vite-project");
    let manifest: Value = read_json_if_exists(project_root.join("package.json"))
        .expect("read manifest")
        .expect("package.json should exist");

    assert_eq!(
        detect_frameworks(&project_root, &manifest).expect("detect frameworks"),
        vec!["Vite".to_string(), "React".to_string()]
    );
}

#[test]
fn detect_package_manager_prefers_manifest_before_lockfiles() {
    let project_root = support::fixture_path("tests/fixtures/workspace/multi-lockfiles");
    let manifest_path = project_root.join("package.json");
    let mut manifest: Value = read_json_if_exists(&manifest_path)
        .expect("read manifest")
        .expect("package.json should exist");

    assert_eq!(
        detect_package_manager(&project_root, &manifest).expect("detect package manager"),
        "pnpm@9.0.0"
    );

    manifest
        .as_object_mut()
        .expect("manifest object")
        .remove("packageManager");

    assert_eq!(
        detect_package_manager(&project_root, &manifest).expect("detect fallback package manager"),
        "pnpm"
    );
}

#[test]
fn find_project_root_handles_package_and_lockfile_entry_paths() {
    let project_root = support::fixture_path("tests/fixtures/workspace/multi-lockfiles");

    for entry in ["package.json", "package-lock.json", "pnpm-lock.yaml"] {
        assert_eq!(
            find_project_root(project_root.join(entry)).expect("find project root from file entry"),
            project_root
        );
    }
}
