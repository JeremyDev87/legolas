use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    boundaries::Phase8SeedContext,
    lockfiles::parse_duplicate_packages,
    package_intelligence::get_package_intel,
    workspace::{normalize_path, read_json_if_exists, read_text_if_exists},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSummary {
    pub name: String,
    pub path: String,
    pub imported_packages: usize,
    pub heavy_dependencies: usize,
    pub duplicate_packages: usize,
    pub potential_kb_saved: usize,
}

#[derive(Debug, Clone)]
struct WorkspaceDescriptor {
    name: String,
    root: PathBuf,
}

pub fn collect_workspace_summaries(context: &Phase8SeedContext<'_>) -> Vec<WorkspaceSummary> {
    let descriptors = match discover_workspace_descriptors(context.project_root) {
        Ok(descriptors) => descriptors,
        Err(_) => return Vec::new(),
    };

    let mut summaries = descriptors
        .into_iter()
        .filter_map(|descriptor| build_workspace_summary(context, descriptor))
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| left.path.cmp(&right.path));
    summaries
}

fn discover_workspace_descriptors(project_root: &Path) -> Result<Vec<WorkspaceDescriptor>, ()> {
    let mut roots = BTreeSet::new();

    for pattern in workspace_patterns(project_root) {
        let discovered = expand_workspace_pattern(project_root, &pattern)?;
        roots.extend(discovered);
    }

    let mut descriptors = Vec::new();
    for root in roots {
        if root == project_root {
            continue;
        }

        let Some(manifest) = read_json_if_exists::<Value, _>(root.join("package.json"))
            .ok()
            .flatten()
        else {
            continue;
        };

        let name = manifest
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                root.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("workspace")
                    .to_string()
            });

        descriptors.push(WorkspaceDescriptor { name, root });
    }

    Ok(descriptors)
}

fn workspace_patterns(project_root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    if let Some(manifest) = read_json_if_exists::<Value, _>(project_root.join("package.json"))
        .ok()
        .flatten()
    {
        patterns.extend(json_workspace_patterns(&manifest));
    }

    if let Some(contents) = read_text_if_exists(project_root.join("pnpm-workspace.yaml"))
        .ok()
        .flatten()
    {
        patterns.extend(pnpm_workspace_patterns(&contents));
    }

    patterns
}

fn json_workspace_patterns(manifest: &Value) -> Vec<String> {
    let Some(workspaces) = manifest.get("workspaces") else {
        return Vec::new();
    };

    match workspaces {
        Value::Array(entries) => entries
            .iter()
            .filter_map(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
        Value::Object(map) => map
            .get("packages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn pnpm_workspace_patterns(contents: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut in_packages = false;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if !raw_line.starts_with(' ') && line.starts_with("packages:") {
            in_packages = true;
            continue;
        }

        if in_packages {
            if let Some(item) = line.strip_prefix("- ") {
                let cleaned = item.trim().trim_matches('"').trim_matches('\'');
                if !cleaned.is_empty() {
                    patterns.push(cleaned.to_string());
                }
                continue;
            }

            if !raw_line.starts_with(' ') {
                in_packages = false;
            }
        }
    }

    patterns
}

fn expand_workspace_pattern(project_root: &Path, pattern: &str) -> Result<BTreeSet<PathBuf>, ()> {
    let mut matches = BTreeSet::new();
    let normalized_pattern = pattern.trim().trim_start_matches("./").replace('\\', "/");
    if normalized_pattern.is_empty() {
        return Ok(matches);
    }

    let segments = normalized_pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    expand_pattern_segments(project_root, project_root, &segments, 0, &mut matches)?;
    Ok(matches)
}

fn expand_pattern_segments(
    project_root: &Path,
    current: &Path,
    segments: &[&str],
    index: usize,
    matches: &mut BTreeSet<PathBuf>,
) -> Result<(), ()> {
    if index == segments.len() {
        if current != project_root && current.join("package.json").is_file() {
            matches.insert(normalize_path(current));
        }
        return Ok(());
    }

    let segment = segments[index];
    if segment.contains('*') {
        let entries = fs::read_dir(current).map_err(|_| ())?;
        for entry in entries {
            let entry = entry.map_err(|_| ())?;
            if !entry.file_type().map_err(|_| ())?.is_dir() {
                continue;
            }

            let candidate = entry.file_name();
            let candidate = candidate.to_string_lossy();
            if !glob_component_matches(segment, &candidate) {
                continue;
            }

            expand_pattern_segments(project_root, &entry.path(), segments, index + 1, matches)?;
        }
        return Ok(());
    }

    let next = current.join(segment);
    if next.is_dir() {
        expand_pattern_segments(project_root, &next, segments, index + 1, matches)?;
    }

    Ok(())
}

fn glob_component_matches(pattern: &str, candidate: &str) -> bool {
    let escaped = regex::escape(pattern).replace(r"\*", ".*");
    Regex::new(&format!("^{escaped}$"))
        .map(|regex| regex.is_match(candidate))
        .unwrap_or(false)
}

fn build_workspace_summary(
    context: &Phase8SeedContext<'_>,
    descriptor: WorkspaceDescriptor,
) -> Option<WorkspaceSummary> {
    let relative_path = descriptor
        .root
        .strip_prefix(context.project_root)
        .unwrap_or(&descriptor.root);
    let path = to_posix(relative_path);
    if path.is_empty() || path == "." {
        return None;
    }

    let imported_packages = context
        .source_analysis
        .imported_packages
        .iter()
        .filter(|record| {
            record
                .files
                .iter()
                .any(|file| file_belongs_to_workspace(file, &path))
        })
        .count();

    let manifest = read_json_if_exists::<Value, _>(descriptor.root.join("package.json"))
        .ok()
        .flatten()?;

    let (heavy_dependencies, heavy_kb_saved) = summarize_workspace_dependencies(&manifest);
    let duplicate_analysis =
        parse_duplicate_packages(&descriptor.root, context.package_manager).unwrap_or_default();
    let duplicate_packages = duplicate_analysis.duplicates.len();
    let duplicate_kb_saved = duplicate_analysis
        .duplicates
        .iter()
        .map(|item| item.estimated_extra_kb)
        .sum::<usize>();
    let potential_kb_saved = heavy_kb_saved + duplicate_kb_saved;

    Some(WorkspaceSummary {
        name: descriptor.name,
        path,
        imported_packages,
        heavy_dependencies,
        duplicate_packages,
        potential_kb_saved,
    })
}

fn summarize_workspace_dependencies(manifest: &Value) -> (usize, usize) {
    let mut heavy_dependencies = 0;
    let mut potential_kb_saved = 0usize;

    for (name, _version_range) in workspace_dependency_entries(manifest) {
        let Some(intel) = get_package_intel(&name) else {
            continue;
        };

        heavy_dependencies += 1;
        potential_kb_saved += (intel.estimated_kb as f64 * 0.18).round() as usize;
    }

    (heavy_dependencies, potential_kb_saved)
}

fn workspace_dependency_entries(manifest: &Value) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    for field in ["dependencies", "optionalDependencies"] {
        let Some(values) = manifest.get(field).and_then(Value::as_object) else {
            continue;
        };

        for (name, version_range) in values {
            let Some(version_range) = version_range.as_str() else {
                continue;
            };

            if let Some((_, existing_range)) = entries
                .iter_mut()
                .find(|(existing_name, _)| existing_name == name)
            {
                *existing_range = version_range.to_string();
                continue;
            }

            entries.push((name.clone(), version_range.to_string()));
        }
    }

    entries
}

fn file_belongs_to_workspace(file: &str, workspace_path: &str) -> bool {
    if workspace_path.is_empty() || workspace_path == "." {
        return false;
    }

    file == workspace_path
        || file
            .strip_prefix(workspace_path)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn to_posix(path: &Path) -> String {
    let normalized = normalize_path(path);
    normalized.to_string_lossy().replace('\\', "/")
}
