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
    route_context::{classify_route_context, RouteContextKind},
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
    let next_client_surface_enabled = context
        .frameworks
        .iter()
        .any(|framework| framework == "Next.js");

    for (package_name, record) in &context.source_analysis.by_package {
        if !is_server_only_package(package_name) {
            continue;
        }

        let Some(client_file) = record.files.iter().find(|file| {
            is_client_surface(context.project_root, file, next_client_surface_enabled)
        }) else {
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

    for (rsc_file, specifier) in collect_server_only_rsc_imports(
        context.project_root,
        context.frameworks,
        next_client_surface_enabled,
        context.source_analysis,
    ) {
        let key = format!("{rsc_file}:{specifier}:rsc");
        if seen.insert(key) {
            warnings.push(build_rsc_boundary_warning(&specifier, &rsc_file));
        }
    }

    for (client_file, specifier) in
        collect_node_prefix_client_imports(context.project_root, next_client_surface_enabled)
    {
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

fn collect_server_only_rsc_imports(
    project_root: &Path,
    frameworks: &[String],
    next_client_surface_enabled: bool,
    source_analysis: &SourceAnalysis,
) -> Vec<(String, String)> {
    let Some(record) = source_analysis.by_package.get("server-only") else {
        return Vec::new();
    };

    record
        .files
        .iter()
        .filter(|file| is_rsc_surface(project_root, frameworks, file, next_client_surface_enabled))
        .map(|file| (file.clone(), String::from("server-only")))
        .collect()
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

fn build_rsc_boundary_warning(specifier: &str, rsc_file: &str) -> BoundaryWarning {
    BoundaryWarning {
        message: format!(
            "RSC surface `{rsc_file}` imports the server-only `{specifier}` module."
        ),
        recommendation:
            "Keep server-only guards in server-only utilities and avoid importing them directly from RSC entrypoints."
                .to_string(),
        finding: FindingMetadata::new(
            "boundary:rsc-server-only",
            FindingAnalysisSource::SourceImport,
        )
        .with_confidence(FindingConfidence::High)
        .with_action_priority(1)
        .with_evidence([FindingEvidence::new("source-file")
            .with_file(rsc_file)
            .with_specifier(specifier)
            .with_detail("RSC surface imports a server-only module")]),
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

fn is_client_surface(
    project_root: &Path,
    relative_path: &str,
    next_client_surface_enabled: bool,
) -> bool {
    if Path::new(relative_path)
        .components()
        .any(|component| matches!(component, Component::Normal(segment) if segment == "client"))
    {
        return true;
    }

    next_client_surface_enabled && has_use_client_directive(project_root.join(relative_path))
}

fn is_rsc_surface(
    project_root: &Path,
    frameworks: &[String],
    relative_path: &str,
    next_client_surface_enabled: bool,
) -> bool {
    if !next_client_surface_enabled || is_client_surface(project_root, relative_path, true) {
        return false;
    }

    matches!(
        classify_route_context(project_root, frameworks, Path::new(relative_path)),
        RouteContextKind::RoutePage
            | RouteContextKind::RouteLayout
            | RouteContextKind::AdminSurface
    )
}

fn collect_node_prefix_client_imports(
    project_root: &Path,
    next_client_surface_enabled: bool,
) -> Vec<(String, String)> {
    let mut matches = Vec::new();
    collect_node_prefix_client_imports_inner(
        project_root,
        project_root,
        next_client_surface_enabled,
        &mut matches,
    );
    matches
}

fn collect_node_prefix_client_imports_inner(
    project_root: &Path,
    current: &Path,
    next_client_surface_enabled: bool,
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
            collect_node_prefix_client_imports_inner(
                project_root,
                &path,
                next_client_surface_enabled,
                matches,
            );
            continue;
        }

        if !is_supported_source_file(&path) {
            continue;
        }

        let Some(relative_path) = path.strip_prefix(project_root).ok().map(to_posix) else {
            continue;
        };
        if !is_client_surface(project_root, &relative_path, next_client_surface_enabled) {
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

fn has_use_client_directive(path: impl AsRef<Path>) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };

    has_directive_prologue_entry(&contents, "use client")
}

fn has_directive_prologue_entry(contents: &str, directive: &str) -> bool {
    let mut in_block_comment = false;

    for line in contents.lines() {
        let mut remainder = line;

        while let Some(fragment) = leading_directive_fragment(remainder, &mut in_block_comment) {
            let Some((literal, rest)) = parse_directive_literal(fragment) else {
                return false;
            };
            if literal == directive {
                return true;
            }

            remainder = rest;
        }
    }

    false
}

fn leading_directive_fragment<'a>(line: &'a str, in_block_comment: &mut bool) -> Option<&'a str> {
    let mut fragment = line.trim().trim_start_matches('\u{feff}');

    loop {
        if fragment.is_empty() {
            return None;
        }

        if *in_block_comment {
            let block_end = fragment.find("*/")?;
            *in_block_comment = false;
            fragment = fragment[block_end + 2..].trim_start();
            continue;
        }

        if fragment.starts_with("//") {
            return None;
        }

        if fragment.starts_with("/*") {
            if let Some(block_end) = fragment.find("*/") {
                fragment = fragment[block_end + 2..].trim_start();
                continue;
            }

            *in_block_comment = true;
            return None;
        }

        return Some(fragment);
    }
}

fn parse_directive_literal(fragment: &str) -> Option<(&str, &str)> {
    let mut chars = fragment.char_indices();
    let (_, quote) = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let closing = fragment[1..].find(quote)? + 1;
    let literal = &fragment[1..closing];
    let rest = fragment[closing + quote.len_utf8()..].trim_start();
    if rest.is_empty() || rest.starts_with("//") || rest.starts_with("/*") {
        return Some((literal, ""));
    }

    let rest = rest.strip_prefix(';')?.trim_start();
    Some((literal, rest))
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

#[cfg(test)]
mod tests {
    use super::has_directive_prologue_entry;

    #[test]
    fn directive_prologue_accepts_same_line_block_comment_prefix() {
        let contents = "/* eslint-disable */ \"use client\";\nimport fs from \"node:fs\";";
        assert!(has_directive_prologue_entry(contents, "use client"));
    }

    #[test]
    fn directive_prologue_accepts_trailing_comment_after_directive() {
        let contents = "\"use client\"; // keep client-only behavior";
        assert!(has_directive_prologue_entry(contents, "use client"));
    }

    #[test]
    fn directive_prologue_accepts_target_after_use_strict() {
        let contents = "\"use strict\"; \"use client\";\nimport fs from \"node:fs\";";
        assert!(has_directive_prologue_entry(contents, "use client"));
    }

    #[test]
    fn directive_prologue_accepts_utf8_bom_prefix() {
        let contents = "\u{feff}\"use client\";\nimport fs from \"node:fs\";";
        assert!(has_directive_prologue_entry(contents, "use client"));
    }

    #[test]
    fn directive_prologue_stops_after_non_directive_code() {
        let contents = "const mode = \"client\";\n\"use client\";";
        assert!(!has_directive_prologue_entry(contents, "use client"));
    }
}
