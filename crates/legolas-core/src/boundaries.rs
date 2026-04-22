use std::{
    path::{Component, Path},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};

use crate::{
    findings::{FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata},
    import_scanner::SourceAnalysis,
};

static SERVER_ONLY_PACKAGES: OnceLock<Vec<&'static str>> = OnceLock::new();

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

    for (package_name, record) in &context.source_analysis.by_package {
        if !is_server_only_package(package_name) {
            continue;
        }

        let Some(client_file) = record.files.iter().find(|file| is_client_surface(file)) else {
            continue;
        };

        warnings.push(build_boundary_warning(package_name, client_file));
    }

    warnings
}

fn build_boundary_warning(package_name: &str, client_file: &str) -> BoundaryWarning {
    BoundaryWarning {
        message: format!(
            "Client surface `{client_file}` imports the Node-only `{package_name}` module."
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
            .with_specifier(package_name)
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
