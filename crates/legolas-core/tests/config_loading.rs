#[allow(dead_code)]
mod support;

use std::fs;

use legolas_core::{
    config::{
        load_config_file, load_discovered_config, BudgetRules, BudgetThresholds, CommandDefaults,
        ConfigWarning, LegolasConfig, LoadedConfig,
    },
    workspace::find_discovered_config_path,
    LegolasError,
};
use tempfile::tempdir;

#[test]
fn load_discovered_config_reads_known_keys_from_project_root() {
    let project_root = support::fixture_path("tests/fixtures/config/discovered");
    let package_json = project_root.join("package.json");
    let config_path = project_root.join("legolas.config.json");
    let loaded = load_discovered_config(&package_json)
        .expect("load discovered config")
        .expect("config should be discovered");

    assert_eq!(
        find_discovered_config_path(&package_json).expect("find discovered config path"),
        Some(config_path.clone())
    );
    assert_eq!(
        loaded,
        LoadedConfig {
            path: config_path,
            config: LegolasConfig {
                command_defaults: CommandDefaults {
                    scan_path: Some("src".to_string()),
                    visualize_limit: Some(12),
                    optimize_top: Some(7),
                },
                budget_rules: Some(BudgetRules {
                    potential_kb_saved: Some(BudgetThresholds {
                        warn_at: 40,
                        fail_at: 80,
                    }),
                    duplicate_package_count: Some(BudgetThresholds {
                        warn_at: 2,
                        fail_at: 4,
                    }),
                    dynamic_import_count: Some(BudgetThresholds {
                        warn_at: 1,
                        fail_at: 0,
                    }),
                }),
            },
            warnings: vec![],
        }
    );
    assert_eq!(
        load_discovered_config(&project_root)
            .expect("load discovered config from directory")
            .expect("config should be discovered from directory input"),
        loaded
    );
}

#[test]
fn load_discovered_config_returns_none_when_config_is_absent() {
    let project_root = support::fixture_path("tests/fixtures/workspace/vite-project");
    let package_json = project_root.join("package.json");

    assert_eq!(
        find_discovered_config_path(&package_json).expect("find discovered config path"),
        None
    );
    assert_eq!(
        load_discovered_config(&package_json).expect("load missing config"),
        None
    );
}

#[test]
fn load_config_file_collects_unknown_key_warnings_and_ignores_them() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("legolas.config.json");
    fs::write(
        &config_path,
        r#"{
  "visualize": { "limit": 9, "theme": "wide" },
  "budget": {
    "rules": {
      "potentialKbSaved": { "warnAt": 40, "failAt": 80, "note": true },
      "surpriseRule": { "warnAt": 1, "failAt": 2 }
    }
  },
  "extra": true
}
"#,
    )
    .expect("write config");

    let loaded = load_config_file(&config_path).expect("load config");
    let mut warnings = loaded
        .warnings
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    warnings.sort();

    assert_eq!(
        loaded.config,
        LegolasConfig {
            command_defaults: CommandDefaults {
                scan_path: None,
                visualize_limit: Some(9),
                optimize_top: None,
            },
            budget_rules: Some(BudgetRules {
                potential_kb_saved: Some(BudgetThresholds {
                    warn_at: 40,
                    fail_at: 80,
                }),
                duplicate_package_count: None,
                dynamic_import_count: None,
            }),
        }
    );
    assert_eq!(
        warnings,
        vec![
            ConfigWarning {
                key_path: "budget.rules.potentialKbSaved.note".to_string(),
                message: "unknown config key ignored".to_string(),
            }
            .to_string(),
            ConfigWarning {
                key_path: "budget.rules.surpriseRule".to_string(),
                message: "unknown config key ignored".to_string(),
            }
            .to_string(),
            ConfigWarning {
                key_path: "extra".to_string(),
                message: "unknown config key ignored".to_string(),
            }
            .to_string(),
            ConfigWarning {
                key_path: "visualize.theme".to_string(),
                message: "unknown config key ignored".to_string(),
            }
            .to_string(),
        ]
    );
}

#[test]
fn load_discovered_config_reports_malformed_json_with_config_path() {
    let project_root = support::fixture_path("tests/fixtures/config/invalid-json");
    let config_path = project_root.join("legolas.config.json");
    let error = load_discovered_config(&project_root).expect_err("malformed config should fail");

    match error {
        LegolasError::MalformedConfig { path, message } => {
            assert_eq!(path, config_path.display().to_string());
            assert!(message.contains("EOF") || message.contains("expected"));
        }
        other => panic!("expected malformed config error, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn find_discovered_config_path_surfaces_metadata_failures_instead_of_hiding_them() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    struct PermissionGuard {
        path: std::path::PathBuf,
        permissions: fs::Permissions,
    }

    impl Drop for PermissionGuard {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.path, self.permissions.clone());
        }
    }

    let project_root = tempdir().expect("create temp project root");
    let package_json = project_root.path().join("package.json");
    fs::write(&package_json, "{}").expect("write package.json");

    let sealed_root = tempdir().expect("create sealed root");
    let sealed_dir = sealed_root.path().join("sealed");
    fs::create_dir(&sealed_dir).expect("create sealed dir");
    fs::write(sealed_dir.join("legolas.config.json"), "{}").expect("write config file");
    symlink(
        sealed_dir.join("legolas.config.json"),
        project_root.path().join("legolas.config.json"),
    )
    .expect("create config symlink");

    let original_permissions = fs::metadata(&sealed_dir)
        .expect("read sealed dir metadata")
        .permissions();
    let _permission_guard = PermissionGuard {
        path: sealed_dir.clone(),
        permissions: original_permissions.clone(),
    };
    let mut sealed_permissions = original_permissions;
    sealed_permissions.set_mode(0o000);
    fs::set_permissions(&sealed_dir, sealed_permissions).expect("seal directory");

    if fs::metadata(project_root.path().join("legolas.config.json")).is_ok() {
        return;
    }

    let error = find_discovered_config_path(&package_json)
        .expect_err("metadata failure should not be hidden as missing config");

    match error {
        LegolasError::Io(io_error) => {
            assert_eq!(io_error.kind(), std::io::ErrorKind::PermissionDenied);
        }
        other => panic!("expected permission denied io error, got {other:?}"),
    }
}
