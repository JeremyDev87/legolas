#[allow(dead_code)]
mod support;

use std::{fs, path::PathBuf, sync::Mutex};

use legolas_core::{
    workspace::{load_alias_config, AliasRule, AliasTarget},
    LegolasError,
};
use tempfile::tempdir;

static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn load_alias_config_reads_tsconfig_paths_deterministically() {
    let project_root = support::fixture_path("tests/fixtures/aliases/tsconfig-paths");
    let loaded = load_alias_config(&project_root)
        .expect("load alias config")
        .expect("tsconfig.json should be discovered");

    assert_eq!(loaded.path, project_root.join("tsconfig.json"));
    assert_eq!(loaded.config.base_url, Some(project_root.clone()));
    assert_eq!(
        loaded.config.rules,
        vec![
            AliasRule {
                pattern: "@shared".to_string(),
                specifier_prefix: "@shared".to_string(),
                replacement_targets: vec![
                    AliasTarget {
                        pattern: "src/shared/index.ts".to_string(),
                        replacement_prefix: "src/shared/index.ts".to_string(),
                        path_candidate: project_root.join("src/shared/index.ts"),
                    },
                    AliasTarget {
                        pattern: "src/shared/fallback.ts".to_string(),
                        replacement_prefix: "src/shared/fallback.ts".to_string(),
                        path_candidate: project_root.join("src/shared/fallback.ts"),
                    },
                ],
                wildcard: false,
            },
            AliasRule {
                pattern: "@/*".to_string(),
                specifier_prefix: "@/".to_string(),
                replacement_targets: vec![AliasTarget {
                    pattern: "src/*".to_string(),
                    replacement_prefix: "src/".to_string(),
                    path_candidate: project_root.join("src"),
                }],
                wildcard: true,
            },
        ]
    );
}

#[test]
fn load_alias_config_reads_jsconfig_paths_when_tsconfig_is_absent() {
    let project_root = support::fixture_path("tests/fixtures/aliases/jsconfig-paths");
    let loaded = load_alias_config(&project_root)
        .expect("load alias config")
        .expect("jsconfig.json should be discovered");

    assert_eq!(loaded.path, project_root.join("jsconfig.json"));
    assert_eq!(loaded.config.base_url, Some(project_root.join("src")));
    assert_eq!(
        loaded.config.rules,
        vec![
            AliasRule {
                pattern: "#env".to_string(),
                specifier_prefix: "#env".to_string(),
                replacement_targets: vec![AliasTarget {
                    pattern: "config/env.js".to_string(),
                    replacement_prefix: "config/env.js".to_string(),
                    path_candidate: project_root.join("src/config/env.js"),
                }],
                wildcard: false,
            },
            AliasRule {
                pattern: "~/*".to_string(),
                specifier_prefix: "~/".to_string(),
                replacement_targets: vec![
                    AliasTarget {
                        pattern: "components/*".to_string(),
                        replacement_prefix: "components/".to_string(),
                        path_candidate: project_root.join("src/components"),
                    },
                    AliasTarget {
                        pattern: "fallback/*".to_string(),
                        replacement_prefix: "fallback/".to_string(),
                        path_candidate: project_root.join("src/fallback"),
                    },
                ],
                wildcard: true,
            },
        ]
    );
}

#[test]
fn load_alias_config_accepts_jsonc_comments_and_trailing_commas() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("tsconfig.json");
    fs::write(
        &config_path,
        r#"{
  // common tsconfig comment
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*",],
    },
  },
}
"#,
    )
    .expect("write jsonc tsconfig");

    let loaded = load_alias_config(temp_dir.path())
        .expect("load jsonc alias config")
        .expect("tsconfig.json should be discovered");

    assert_eq!(loaded.path, config_path);
    assert_eq!(loaded.config.base_url, Some(temp_dir.path().to_path_buf()));
    assert_eq!(
        loaded.config.rules,
        vec![AliasRule {
            pattern: "@/*".to_string(),
            specifier_prefix: "@/".to_string(),
            replacement_targets: vec![AliasTarget {
                pattern: "src/*".to_string(),
                replacement_prefix: "src/".to_string(),
                path_candidate: temp_dir.path().join("src"),
            }],
            wildcard: true,
        }]
    );
}

#[test]
fn load_alias_config_preserves_fallback_targets_and_catch_all_rules() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("tsconfig.json");
    fs::write(
        &config_path,
        r##"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@ui": ["src/ui/index.ts", "src/ui/index.js"],
      "*": ["src/*", "generated/*"]
    }
  }
}
"##,
    )
    .expect("write fallback tsconfig");

    let loaded = load_alias_config(temp_dir.path())
        .expect("load alias config")
        .expect("tsconfig.json should be discovered");

    assert_eq!(loaded.path, config_path);
    assert_eq!(
        loaded.config.rules,
        vec![
            AliasRule {
                pattern: "@ui".to_string(),
                specifier_prefix: "@ui".to_string(),
                replacement_targets: vec![
                    AliasTarget {
                        pattern: "src/ui/index.ts".to_string(),
                        replacement_prefix: "src/ui/index.ts".to_string(),
                        path_candidate: temp_dir.path().join("src/ui/index.ts"),
                    },
                    AliasTarget {
                        pattern: "src/ui/index.js".to_string(),
                        replacement_prefix: "src/ui/index.js".to_string(),
                        path_candidate: temp_dir.path().join("src/ui/index.js"),
                    },
                ],
                wildcard: false,
            },
            AliasRule {
                pattern: "*".to_string(),
                specifier_prefix: "".to_string(),
                replacement_targets: vec![
                    AliasTarget {
                        pattern: "src/*".to_string(),
                        replacement_prefix: "src/".to_string(),
                        path_candidate: temp_dir.path().join("src"),
                    },
                    AliasTarget {
                        pattern: "generated/*".to_string(),
                        replacement_prefix: "generated/".to_string(),
                        path_candidate: temp_dir.path().join("generated"),
                    },
                ],
                wildcard: true,
            },
        ]
    );
}

#[test]
fn load_alias_config_preserves_raw_package_remap_targets() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("tsconfig.json");
    fs::write(
        &config_path,
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@react": ["react"],
      "@scope/*": ["@vendor/pkg/*"]
    }
  }
}
"#,
    )
    .expect("write package-remap tsconfig");

    let loaded = load_alias_config(temp_dir.path())
        .expect("load alias config")
        .expect("tsconfig.json should be discovered");

    assert_eq!(loaded.path, config_path);
    assert_eq!(
        loaded.config.rules,
        vec![
            AliasRule {
                pattern: "@scope/*".to_string(),
                specifier_prefix: "@scope/".to_string(),
                replacement_targets: vec![AliasTarget {
                    pattern: "@vendor/pkg/*".to_string(),
                    replacement_prefix: "@vendor/pkg/".to_string(),
                    path_candidate: temp_dir.path().join("@vendor/pkg"),
                }],
                wildcard: true,
            },
            AliasRule {
                pattern: "@react".to_string(),
                specifier_prefix: "@react".to_string(),
                replacement_targets: vec![AliasTarget {
                    pattern: "react".to_string(),
                    replacement_prefix: "react".to_string(),
                    path_candidate: temp_dir.path().join("react"),
                }],
                wildcard: false,
            },
        ]
    );
}

#[test]
fn load_alias_config_resolves_parent_segments_from_relative_project_roots() {
    struct CurrentDirGuard {
        original: PathBuf,
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original).expect("restore current dir");
        }
    }

    let _cwd_lock = CURRENT_DIR_LOCK.lock().expect("lock current dir");
    let original_dir = std::env::current_dir().expect("read current dir");
    let _current_dir_guard = CurrentDirGuard {
        original: original_dir,
    };

    let temp_dir = tempdir().expect("create temp dir");
    let project_root = temp_dir.path().join("project");
    let shared_root = temp_dir.path().join("shared");
    let source_root = temp_dir.path().join("src");
    fs::create_dir_all(&project_root).expect("create project root");
    fs::create_dir_all(&shared_root).expect("create shared root");
    fs::create_dir_all(&source_root).expect("create source root");
    let config_path = project_root.join("tsconfig.json");
    fs::write(
        &config_path,
        r#"{
  "compilerOptions": {
    "baseUrl": "../shared",
    "paths": {
      "@/*": ["../src/*"]
    }
  }
}
"#,
    )
    .expect("write relative-root tsconfig");

    std::env::set_current_dir(&project_root).expect("enter project root");

    let loaded = load_alias_config(".")
        .expect("load alias config from relative root")
        .expect("tsconfig.json should be discovered");

    assert_eq!(
        fs::canonicalize(&loaded.path).expect("canonicalize loaded config path"),
        fs::canonicalize(&config_path).expect("canonicalize expected config path")
    );
    assert_eq!(
        fs::canonicalize(
            loaded
                .config
                .base_url
                .as_ref()
                .expect("baseUrl should be resolved"),
        )
        .expect("canonicalize loaded baseUrl"),
        fs::canonicalize(&shared_root).expect("canonicalize expected shared root")
    );
    assert_eq!(loaded.config.rules.len(), 1);
    assert_eq!(loaded.config.rules[0].pattern, "@/*");
    assert_eq!(loaded.config.rules[0].specifier_prefix, "@/");
    assert!(loaded.config.rules[0].wildcard);
    assert_eq!(loaded.config.rules[0].replacement_targets.len(), 1);
    assert_eq!(
        fs::canonicalize(&loaded.config.rules[0].replacement_targets[0].path_candidate)
            .expect("canonicalize loaded replacement target"),
        fs::canonicalize(&source_root).expect("canonicalize expected source root")
    );
}

#[test]
fn load_alias_config_accepts_file_and_nested_directory_inputs() {
    let temp_dir = tempdir().expect("create temp dir");
    let project_root = temp_dir.path().join("alias-project");
    let nested_dir = project_root.join("src/nested");
    let file_input = nested_dir.join("App.tsx");
    fs::create_dir_all(&nested_dir).expect("create nested source dir");
    fs::write(
        project_root.join("package.json"),
        "{\n  \"name\": \"alias-project\"\n}\n",
    )
    .expect("write package.json");
    fs::write(
        project_root.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  }
}
"#,
    )
    .expect("write tsconfig");
    fs::write(&file_input, "export const App = () => null;\n").expect("write source file");

    let from_directory = load_alias_config(project_root.join("src"))
        .expect("load alias config from directory input")
        .expect("tsconfig should be discovered from directory input");
    let from_file = load_alias_config(&file_input)
        .expect("load alias config from file input")
        .expect("tsconfig should be discovered from file input");

    for loaded in [from_directory, from_file] {
        assert_eq!(loaded.path, project_root.join("tsconfig.json"));
        assert_eq!(loaded.config.base_url, Some(project_root.clone()));
        assert_eq!(
            loaded.config.rules,
            vec![AliasRule {
                pattern: "@/*".to_string(),
                specifier_prefix: "@/".to_string(),
                replacement_targets: vec![AliasTarget {
                    pattern: "src/*".to_string(),
                    replacement_prefix: "src/".to_string(),
                    path_candidate: project_root.join("src"),
                }],
                wildcard: true,
            }]
        );
    }
}

#[test]
fn load_alias_config_returns_none_when_project_has_no_supported_config_file() {
    let project_root = support::fixture_path("tests/fixtures/workspace/vite-project");

    assert_eq!(
        load_alias_config(&project_root).expect("load alias config"),
        None
    );
}

#[test]
fn load_alias_config_keeps_missing_and_malformed_config_states_distinct() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("tsconfig.json");
    fs::write(
        &config_path,
        r#"{
  "compilerOptions": {
    "paths": "@/src/*"
  }
}
"#,
    )
    .expect("write malformed tsconfig");

    let error = load_alias_config(temp_dir.path()).expect_err("malformed config should fail");

    match error {
        LegolasError::UnsupportedConfigShape {
            path,
            key_path,
            message,
        } => {
            assert_eq!(path, config_path.display().to_string());
            assert_eq!(key_path, "compilerOptions.paths");
            assert_eq!(message, "expected object");
        }
        other => panic!("expected unsupported config shape error, got {other:?}"),
    }
}

#[test]
fn load_alias_config_rejects_unterminated_block_comments_in_jsonc() {
    let temp_dir = tempdir().expect("create temp dir");
    let config_path = temp_dir.path().join("tsconfig.json");
    fs::write(
        &config_path,
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  }
} /* unterminated
"#,
    )
    .expect("write malformed jsonc tsconfig");

    let error = load_alias_config(temp_dir.path()).expect_err("unterminated block comment");

    match error {
        LegolasError::MalformedConfig { path, message } => {
            assert_eq!(path, config_path.display().to_string());
            assert_eq!(message, "unterminated block comment");
        }
        other => panic!("expected malformed config error, got {other:?}"),
    }
}
