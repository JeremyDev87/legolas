use std::{
    fs,
    path::{Component, Path},
    sync::OnceLock,
};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    findings::{FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata},
    import_scanner::SourceAnalysis,
};

static SERVER_ONLY_PACKAGES: OnceLock<Vec<&'static str>> = OnceLock::new();
static NODE_PREFIX_IMPORT_PATTERN: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BoundaryWarning {
    pub message: String,
    pub recommendation: String,
    #[serde(flatten, default, skip_serializing_if = "FindingMetadata::is_empty")]
    pub finding: FindingMetadata,
}

#[derive(Debug, Clone, Copy)]
pub struct Phase8SeedContext<'a> {
    pub project_root: &'a Path,
    pub package_manager: &'a str,
    pub frameworks: &'a [String],
    pub bundle_artifacts: &'a [String],
    pub source_analysis: &'a SourceAnalysis,
    pub source_file_count: usize,
    pub imported_package_count: usize,
    pub dynamic_import_count: usize,
}

pub fn collect_boundary_warnings(context: &Phase8SeedContext<'_>) -> Vec<BoundaryWarning> {
    let mut warnings = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for (package_name, record) in &context.source_analysis.by_package {
        if !is_server_only_package(package_name) {
            continue;
        }

        let Some(client_file) = record.files.iter().find(|file| is_client_surface(file)) else {
            continue;
        };

        let key = format!("{client_file}:{package_name}");
        if seen.insert(key) {
            warnings.push(build_boundary_warning(
                package_name,
                package_name,
                client_file,
            ));
        }
    }

    for (client_file, specifier) in collect_node_prefix_client_imports(context.project_root) {
        let Some(package_name) = specifier.strip_prefix("node:") else {
            continue;
        };
        if !is_server_only_package(package_name) {
            continue;
        }

        let key = format!("{client_file}:{package_name}");
        if seen.insert(key) {
            warnings.push(build_boundary_warning(
                package_name,
                &specifier,
                &client_file,
            ));
        }
    }

    warnings
}

fn build_boundary_warning(
    package_name: &str,
    raw_specifier: &str,
    client_file: &str,
) -> BoundaryWarning {
    BoundaryWarning {
        message: format!(
            "Client surface `{client_file}` imports the Node-only `{raw_specifier}` module."
        ),
        recommendation: "Keep Node-only work on the server and pass browser-safe data into the client component."
            .to_string(),
        finding: FindingMetadata::new(
            format!("boundary:server-client:{package_name}"),
            FindingAnalysisSource::SourceImport,
        )
        .with_confidence(FindingConfidence::High)
        .with_action_priority(1)
        .with_evidence([FindingEvidence::new("source-file")
            .with_file(client_file)
            .with_specifier(raw_specifier)
            .with_detail("client surface imports a Node-only module")]),
    }
}

fn is_server_only_package(package_name: &str) -> bool {
    server_only_packages().contains(&package_name)
}

fn server_only_packages() -> &'static [&'static str] {
    SERVER_ONLY_PACKAGES
        .get_or_init(|| {
            vec![
                "child_process",
                "crypto",
                "dns",
                "fs",
                "module",
                "net",
                "os",
                "path",
                "readline",
                "server-only",
                "stream",
                "tls",
                "worker_threads",
            ]
        })
        .as_slice()
}

fn is_client_surface(relative_path: &str) -> bool {
    Path::new(relative_path)
        .components()
        .any(|component| matches!(component, Component::Normal(segment) if segment == "client"))
}

fn collect_node_prefix_client_imports(project_root: &Path) -> Vec<(String, String)> {
    let mut matches = Vec::new();
    collect_node_prefix_client_imports_inner(project_root, project_root, &mut matches);
    matches
}

fn collect_node_prefix_client_imports_inner(
    project_root: &Path,
    current: &Path,
    matches: &mut Vec<(String, String)>,
) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if matches!(
                name,
                ".git"
                    | "node_modules"
                    | "dist"
                    | "build"
                    | ".next"
                    | ".turbo"
                    | "coverage"
                    | ".output"
                    | "test"
                    | "tests"
                    | "__tests__"
            ) {
                continue;
            }
            collect_node_prefix_client_imports_inner(project_root, &path, matches);
            continue;
        }

        if !is_supported_source_file(&path) {
            continue;
        }

        let Some(relative_path) = path.strip_prefix(project_root).ok().map(to_posix) else {
            continue;
        };
        if !is_client_surface(&relative_path) {
            continue;
        }

        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        for specifier in node_prefix_import_specifiers(&contents) {
            matches.push((relative_path.clone(), specifier));
        }
    }
}

fn is_supported_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("js" | "jsx" | "ts" | "tsx" | "cjs" | "cts" | "mjs" | "mts" | "vue" | "svelte")
    )
}

fn node_prefix_import_specifiers(contents: &str) -> Vec<String> {
    node_prefix_import_pattern()
        .captures_iter(contents)
        .filter_map(|captures| captures.get(1).map(|value| value.as_str().to_string()))
        .collect()
}

fn node_prefix_import_pattern() -> &'static Regex {
    NODE_PREFIX_IMPORT_PATTERN.get_or_init(|| {
        Regex::new(r#"["'](node:[^"']+)["']"#).expect("valid node prefix import regex")
    })
}

fn to_posix(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(segment) => Some(segment.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
