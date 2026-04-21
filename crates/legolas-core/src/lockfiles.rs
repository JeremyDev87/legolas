use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};

use crate::{
    error::Result,
    models::{DuplicateOrigin, DuplicatePackage},
    workspace::{exists, read_json_if_exists, read_text_if_exists},
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DuplicateAnalysis {
    pub duplicates: Vec<DuplicatePackage>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockfileKind {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

#[derive(Debug, Clone)]
struct Lockfile {
    kind: LockfileKind,
    name: &'static str,
    file_path: PathBuf,
}

type VersionsByName = BTreeMap<String, Vec<String>>;
type OriginsByName = BTreeMap<String, Vec<DuplicateOrigin>>;

#[derive(Debug, Default)]
struct DuplicateData {
    versions_by_name: VersionsByName,
    origins_by_name: OriginsByName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencyRef {
    name: String,
    selector: String,
}

#[derive(Debug, Clone)]
struct DependencyNode {
    name: String,
    version: String,
    dependencies: Vec<DependencyRef>,
}

#[derive(Debug, Clone)]
struct YarnNode {
    name: String,
    version: String,
    dependencies: Vec<DependencyRef>,
    descriptors: Vec<String>,
    resolution: Option<String>,
}

static PNPM_ENTRY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^ {2}'?(\S.*?)'?:\s*$").expect("valid pnpm entry regex"));
static DESCRIPTOR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(@[^/]+/[^@]+|[^@]+)@(.+)$").expect("valid descriptor regex"));
static YARN_VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^ {2}version "(.*)"$"#).expect("valid yarn version regex"));
static YARN_BERRY_VERSION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^ {2}version:\s+"?([^"]+)"?$"#).expect("valid yarn berry version regex")
});
static YARN_RESOLUTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^ {2}resolution(?::)?\s+"?([^"]+)"?$"#).expect("valid yarn resolution regex")
});
static SPLIT_HEADER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r",\s*").expect("valid yarn header split regex"));

pub fn parse_duplicate_packages<P: AsRef<Path>>(
    project_root: P,
    package_manager: &str,
) -> Result<DuplicateAnalysis> {
    let project_root = project_root.as_ref();
    let preferred_lockfile = normalize_package_manager(package_manager);
    let ordered_lockfiles = prioritize_lockfiles(lockfiles(project_root), preferred_lockfile);
    let mut existing_lockfiles = Vec::new();

    for lockfile in ordered_lockfiles {
        if exists(&lockfile.file_path)? {
            existing_lockfiles.push(lockfile);
        }
    }

    if existing_lockfiles.is_empty() {
        return Ok(DuplicateAnalysis::default());
    }

    let selected_lockfile = existing_lockfiles.remove(0);
    let duplicates = read_lockfile(&selected_lockfile)?;
    let warnings =
        build_lockfile_warnings(&selected_lockfile, &existing_lockfiles, package_manager);

    Ok(DuplicateAnalysis {
        duplicates,
        warnings,
    })
}

fn lockfiles(project_root: &Path) -> Vec<Lockfile> {
    vec![
        Lockfile {
            kind: LockfileKind::Npm,
            name: "npm",
            file_path: project_root.join("package-lock.json"),
        },
        Lockfile {
            kind: LockfileKind::Pnpm,
            name: "pnpm",
            file_path: project_root.join("pnpm-lock.yaml"),
        },
        Lockfile {
            kind: LockfileKind::Yarn,
            name: "yarn",
            file_path: project_root.join("yarn.lock"),
        },
        Lockfile {
            kind: LockfileKind::Bun,
            name: "bun",
            file_path: project_root.join("bun.lock"),
        },
        Lockfile {
            kind: LockfileKind::Bun,
            name: "bun",
            file_path: project_root.join("bun.lockb"),
        },
    ]
}

fn prioritize_lockfiles(
    lockfiles: Vec<Lockfile>,
    preferred_lockfile: Option<&'static str>,
) -> Vec<Lockfile> {
    let Some(preferred_lockfile) = preferred_lockfile else {
        return lockfiles;
    };

    let (preferred, remaining): (Vec<_>, Vec<_>) = lockfiles
        .into_iter()
        .partition(|lockfile| lockfile.name == preferred_lockfile);

    preferred.into_iter().chain(remaining).collect()
}

fn read_lockfile(lockfile: &Lockfile) -> Result<Vec<DuplicatePackage>> {
    match lockfile.kind {
        LockfileKind::Npm => {
            let package_lock: Option<Value> = read_json_if_exists(&lockfile.file_path)?;
            Ok(summarize_duplicates(collect_from_package_lock(
                package_lock,
            )))
        }
        LockfileKind::Pnpm => {
            let content = read_text_if_exists(&lockfile.file_path)?.unwrap_or_default();
            Ok(summarize_duplicates(collect_from_pnpm_lock(&content)))
        }
        LockfileKind::Yarn => {
            let content = read_text_if_exists(&lockfile.file_path)?.unwrap_or_default();
            Ok(summarize_duplicates(collect_from_yarn_lock(&content)))
        }
        LockfileKind::Bun => Ok(Vec::new()),
    }
}

fn collect_from_package_lock(package_lock: Option<Value>) -> DuplicateData {
    let mut data = DuplicateData::default();
    let Some(package_lock) = package_lock else {
        return data;
    };

    let Some(package_lock_object) = package_lock.as_object() else {
        return data;
    };

    if let Some(packages) = package_lock_object
        .get("packages")
        .and_then(Value::as_object)
    {
        if !packages.is_empty() {
            for (package_path, metadata) in packages {
                let Some(version) = metadata
                    .get("version")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                else {
                    continue;
                };

                let package_chain = package_path_chain(package_path);
                let Some(package_name) = package_chain.last() else {
                    continue;
                };
                add_version(&mut data.versions_by_name, package_name, version);
                add_origin(
                    &mut data.origins_by_name,
                    package_name,
                    build_duplicate_origin(version, &package_chain),
                );
            }

            return data;
        }
    }

    if let Some(dependencies) = package_lock_object
        .get("dependencies")
        .and_then(Value::as_object)
    {
        collect_from_package_lock_dependencies(
            dependencies,
            &mut data.versions_by_name,
            &mut data.origins_by_name,
            &mut Vec::new(),
        );
    }

    data
}

fn collect_from_package_lock_dependencies(
    dependencies: &Map<String, Value>,
    versions_by_name: &mut VersionsByName,
    origins_by_name: &mut OriginsByName,
    path: &mut Vec<String>,
) {
    for (name, metadata) in dependencies {
        path.push(name.clone());

        if let Some(version) = metadata
            .get("version")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            add_version(versions_by_name, name, version);
            add_origin(origins_by_name, name, build_duplicate_origin(version, path));
        }

        if let Some(child_dependencies) = metadata.get("dependencies").and_then(Value::as_object) {
            collect_from_package_lock_dependencies(
                child_dependencies,
                versions_by_name,
                origins_by_name,
                path,
            );
        }

        path.pop();
    }
}

fn collect_from_pnpm_lock(content: &str) -> DuplicateData {
    let nodes = parse_pnpm_nodes(content);
    let adjacency = build_pnpm_adjacency(&nodes);

    collect_from_dependency_graph(nodes, adjacency)
}

fn parse_pnpm_nodes(content: &str) -> BTreeMap<String, DependencyNode> {
    let mut nodes = BTreeMap::new();
    let mut inside_packages = false;
    let mut current_node_id: Option<String> = None;
    let mut inside_dependencies = false;

    for raw_line in content.split('\n') {
        let line = raw_line.trim_end_matches('\r');

        if line == "packages:" || line == "snapshots:" {
            inside_packages = true;
            current_node_id = None;
            inside_dependencies = false;
            continue;
        }

        if inside_packages && starts_with_ascii_alpha(line) {
            inside_packages = false;
            current_node_id = None;
            inside_dependencies = false;
        }

        if !inside_packages {
            continue;
        }

        if let Some(captures) = PNPM_ENTRY_RE.captures(line) {
            let mut descriptor = captures[1].to_string();
            if let Some(stripped) = descriptor.strip_prefix('/') {
                descriptor = stripped.to_string();
            }

            let Some((name, version)) = split_pnpm_descriptor(&descriptor) else {
                current_node_id = None;
                inside_dependencies = false;
                continue;
            };

            let node = nodes
                .entry(descriptor.clone())
                .or_insert_with(|| DependencyNode {
                    name,
                    version,
                    dependencies: Vec::new(),
                });
            current_node_id = Some(descriptor);
            inside_dependencies = false;

            if node.name.is_empty() {
                current_node_id = None;
            }
            continue;
        }

        if line == "    dependencies:" || line == "    optionalDependencies:" {
            inside_dependencies = true;
            continue;
        }

        if line.starts_with("    ")
            && !line.starts_with("      ")
            && line.trim_end().ends_with(':')
            && line != "    dependencies:"
            && line != "    optionalDependencies:"
        {
            inside_dependencies = false;
            continue;
        }

        if !inside_dependencies {
            continue;
        }

        let Some(node_id) = current_node_id.as_ref() else {
            continue;
        };
        let Some((name, selector)) = parse_yaml_dependency_line(line, 6) else {
            continue;
        };
        let Some(node) = nodes.get_mut(node_id) else {
            continue;
        };
        let dependency = DependencyRef { name, selector };
        if !node
            .dependencies
            .iter()
            .any(|existing| existing == &dependency)
        {
            node.dependencies.push(dependency);
        }
    }

    nodes
}

fn build_pnpm_adjacency(nodes: &BTreeMap<String, DependencyNode>) -> BTreeMap<String, Vec<String>> {
    let mut descriptor_index = BTreeMap::new();
    let mut by_name_version: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();

    for (id, node) in nodes {
        descriptor_index.insert(id.clone(), id.clone());
        by_name_version
            .entry((node.name.clone(), node.version.clone()))
            .or_default()
            .push(id.clone());
    }

    let mut adjacency = BTreeMap::new();

    for (id, node) in nodes {
        let mut targets: Vec<String> = Vec::new();

        for dependency in &node.dependencies {
            let Some(target_id) =
                resolve_pnpm_dependency(dependency, &descriptor_index, &by_name_version)
            else {
                continue;
            };

            if !targets.iter().any(|existing| existing == &target_id) {
                targets.push(target_id);
            }
        }

        adjacency.insert(id.clone(), targets);
    }

    adjacency
}

fn collect_from_yarn_lock(content: &str) -> DuplicateData {
    let nodes = parse_yarn_nodes(content);
    let adjacency = build_yarn_adjacency(&nodes);
    let graph_nodes = nodes
        .into_iter()
        .map(|(id, node)| {
            (
                id,
                DependencyNode {
                    name: node.name,
                    version: node.version,
                    dependencies: Vec::new(),
                },
            )
        })
        .collect();

    collect_from_dependency_graph(graph_nodes, adjacency)
}

fn parse_yarn_nodes(content: &str) -> BTreeMap<String, YarnNode> {
    let mut nodes = BTreeMap::new();
    let mut current_descriptors: Vec<String> = Vec::new();
    let mut current_package_names: Vec<String> = Vec::new();
    let mut current_version: Option<String> = None;
    let mut current_resolution: Option<String> = None;
    let mut current_dependencies: Vec<DependencyRef> = Vec::new();
    let mut inside_dependencies = false;

    for raw_line in content.split('\n') {
        let line = raw_line.trim_end_matches('\r');

        if line.trim().is_empty() {
            flush_current_yarn_entry(
                &mut nodes,
                &current_descriptors,
                &current_package_names,
                current_version.as_deref(),
                current_resolution.as_deref(),
                &current_dependencies,
            );
            current_descriptors.clear();
            current_package_names.clear();
            current_version = None;
            current_resolution = None;
            current_dependencies.clear();
            inside_dependencies = false;
            continue;
        }

        if !line.starts_with(' ') {
            flush_current_yarn_entry(
                &mut nodes,
                &current_descriptors,
                &current_package_names,
                current_version.as_deref(),
                current_resolution.as_deref(),
                &current_dependencies,
            );
            current_descriptors = parse_yarn_header_descriptors(line);
            current_package_names = current_descriptors
                .iter()
                .filter_map(|descriptor| extract_yarn_package_name(descriptor))
                .collect();
            current_version = None;
            current_resolution = None;
            current_dependencies.clear();
            inside_dependencies = false;
            continue;
        }

        if line == "  dependencies:" || line == "  optionalDependencies:" {
            inside_dependencies = true;
            continue;
        }

        if line.starts_with("  ")
            && !line.starts_with("    ")
            && line.trim_end().ends_with(':')
            && line != "  dependencies:"
            && line != "  optionalDependencies:"
        {
            inside_dependencies = false;
        }

        if inside_dependencies {
            if let Some(dependency) = parse_yaml_dependency_line(line, 4)
                .map(|(name, selector)| DependencyRef { name, selector })
            {
                if !current_dependencies
                    .iter()
                    .any(|existing| existing == &dependency)
                {
                    current_dependencies.push(dependency);
                }
                continue;
            }
        }

        if let Some(captures) = YARN_VERSION_RE.captures(line) {
            current_version = Some(captures[1].to_string());
            continue;
        }

        if let Some(captures) = YARN_BERRY_VERSION_RE.captures(line) {
            current_version = Some(captures[1].to_string());
            continue;
        }

        if let Some(captures) = YARN_RESOLUTION_RE.captures(line) {
            current_resolution = Some(normalize_yarn_descriptor(&captures[1]));
        }
    }

    flush_current_yarn_entry(
        &mut nodes,
        &current_descriptors,
        &current_package_names,
        current_version.as_deref(),
        current_resolution.as_deref(),
        &current_dependencies,
    );

    nodes
}

fn parse_yarn_header_descriptors(line: &str) -> Vec<String> {
    let trimmed = line.strip_suffix(':').unwrap_or(line);

    SPLIT_HEADER_RE
        .split(trimmed)
        .map(str::trim)
        .map(normalize_yarn_descriptor)
        .collect()
}

fn flush_current_yarn_entry(
    nodes: &mut BTreeMap<String, YarnNode>,
    descriptors: &[String],
    package_names: &[String],
    version: Option<&str>,
    resolution: Option<&str>,
    dependencies: &[DependencyRef],
) {
    let Some(version) = version else {
        return;
    };
    let Some(name) = package_names.first() else {
        return;
    };
    let Some(first_descriptor) = descriptors.first() else {
        return;
    };

    let id = resolution.unwrap_or(first_descriptor).to_string();
    nodes.insert(
        id,
        YarnNode {
            name: name.clone(),
            version: version.to_string(),
            dependencies: dependencies.to_vec(),
            descriptors: descriptors.to_vec(),
            resolution: resolution.map(str::to_string),
        },
    );
}

fn build_yarn_adjacency(nodes: &BTreeMap<String, YarnNode>) -> BTreeMap<String, Vec<String>> {
    let mut descriptor_index = BTreeMap::new();
    let mut by_name_version: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();

    for (id, node) in nodes {
        for descriptor in &node.descriptors {
            descriptor_index.insert(descriptor.clone(), id.clone());
        }
        if let Some(resolution) = node.resolution.as_ref() {
            descriptor_index.insert(resolution.clone(), id.clone());
        }
        by_name_version
            .entry((node.name.clone(), node.version.clone()))
            .or_default()
            .push(id.clone());
    }

    let mut adjacency = BTreeMap::new();

    for (id, node) in nodes {
        let mut targets = Vec::new();

        for dependency in &node.dependencies {
            let Some(target_id) =
                resolve_yarn_dependency(dependency, &descriptor_index, &by_name_version)
            else {
                continue;
            };

            if !targets.iter().any(|existing| existing == &target_id) {
                targets.push(target_id);
            }
        }

        adjacency.insert(id.clone(), targets);
    }

    adjacency
}

fn resolve_yarn_dependency(
    dependency: &DependencyRef,
    descriptor_index: &BTreeMap<String, String>,
    by_name_version: &BTreeMap<(String, String), Vec<String>>,
) -> Option<String> {
    let selector = strip_wrapping_quotes(&dependency.selector).to_string();
    for descriptor in yarn_dependency_lookup_keys(&dependency.name, &selector) {
        if let Some(target_id) = descriptor_index.get(&descriptor) {
            return Some(target_id.clone());
        }
    }

    let normalized = selector.strip_prefix("npm:").unwrap_or(&selector);

    if let Some((name, version)) = split_descriptor(normalized) {
        return unique_candidate(by_name_version.get(&(name, version)));
    }

    unique_candidate(by_name_version.get(&(dependency.name.clone(), normalized.to_string())))
}

fn collect_from_dependency_graph(
    nodes: BTreeMap<String, DependencyNode>,
    adjacency: BTreeMap<String, Vec<String>>,
) -> DuplicateData {
    let mut data = DuplicateData::default();

    for node in nodes.values() {
        add_version(&mut data.versions_by_name, &node.name, &node.version);
    }

    if nodes.is_empty() {
        return data;
    }

    let mut indegree = nodes
        .keys()
        .cloned()
        .map(|id| (id, 0_usize))
        .collect::<BTreeMap<_, _>>();

    for targets in adjacency.values() {
        for target in targets {
            if let Some(count) = indegree.get_mut(target) {
                *count += 1;
            }
        }
    }

    let mut roots = indegree
        .into_iter()
        .filter_map(|(id, count)| (count == 0).then_some(id))
        .collect::<Vec<_>>();
    if roots.is_empty() {
        roots = nodes.keys().cloned().collect();
    }

    for root_id in roots {
        let Some(root_node) = nodes.get(&root_id) else {
            continue;
        };
        let mut visited = BTreeSet::new();
        let mut path = vec![root_node.name.clone()];
        walk_dependency_graph(
            &root_id,
            &nodes,
            &adjacency,
            &mut visited,
            &mut path,
            &mut data,
        );
    }

    data
}

fn walk_dependency_graph(
    current_id: &str,
    nodes: &BTreeMap<String, DependencyNode>,
    adjacency: &BTreeMap<String, Vec<String>>,
    visited: &mut BTreeSet<String>,
    path: &mut Vec<String>,
    data: &mut DuplicateData,
) {
    let Some(current_node) = nodes.get(current_id) else {
        return;
    };
    add_origin(
        &mut data.origins_by_name,
        &current_node.name,
        build_duplicate_origin(&current_node.version, path),
    );

    if !visited.insert(current_id.to_string()) {
        return;
    }

    if let Some(targets) = adjacency.get(current_id) {
        for target_id in targets {
            if visited.contains(target_id) {
                continue;
            }

            let Some(target_node) = nodes.get(target_id) else {
                continue;
            };

            path.push(target_node.name.clone());
            walk_dependency_graph(target_id, nodes, adjacency, visited, path, data);
            path.pop();
        }
    }

    visited.remove(current_id);
}

fn split_descriptor(descriptor: &str) -> Option<(String, String)> {
    let clean = descriptor.strip_prefix("npm:").unwrap_or(descriptor);
    let captures = DESCRIPTOR_RE.captures(clean)?;

    Some((
        captures[1].to_string(),
        captures[2]
            .split('(')
            .next()
            .unwrap_or_default()
            .to_string(),
    ))
}

fn split_pnpm_descriptor(descriptor: &str) -> Option<(String, String)> {
    let (name, version) = split_descriptor(descriptor)?;

    let Some(alias_target) = version.strip_prefix("npm:") else {
        return Some((name, version));
    };

    if let Some((target_name, target_version)) = split_descriptor(alias_target) {
        return Some((target_name, target_version));
    }

    Some((name, alias_target.to_string()))
}

fn extract_yarn_package_name(descriptor: &str) -> Option<String> {
    let (name, version) = split_descriptor(descriptor.trim_start_matches('/'))?;

    if let Some(alias_target) = version.strip_prefix("npm:") {
        if let Some((target_name, _)) = split_descriptor(alias_target) {
            return Some(target_name);
        }
    }

    Some(name)
}

fn normalize_yarn_descriptor(value: &str) -> String {
    strip_wrapping_quotes(value.trim())
        .trim_start_matches('/')
        .to_string()
}

fn summarize_duplicates(mut data: DuplicateData) -> Vec<DuplicatePackage> {
    let mut results = Vec::new();

    for (name, mut versions) in data.versions_by_name {
        if versions.len() < 2 {
            continue;
        }

        versions.sort_by(|left, right| compare_versions(left, right));
        let mut origins = data.origins_by_name.remove(&name).unwrap_or_default();
        origins.sort_by(|left, right| {
            compare_versions(&left.version, &right.version)
                .then_with(|| left.root_requester.cmp(&right.root_requester))
                .then_with(|| left.via_chain.cmp(&right.via_chain))
        });
        let estimated_extra_kb = usize::max((versions.len().saturating_sub(1)) * 18, 18);

        results.push(DuplicatePackage {
            name,
            count: versions.len(),
            versions,
            estimated_extra_kb,
            origins,
            finding: Default::default(),
        });
    }

    results.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    results
}

fn package_path_chain(package_path: &str) -> Vec<String> {
    package_path
        .split("node_modules/")
        .skip(1)
        .map(|segment| segment.trim_matches('/'))
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn build_duplicate_origin(version: &str, path: &[String]) -> DuplicateOrigin {
    let root_requester = path.first().cloned().unwrap_or_default();

    DuplicateOrigin {
        version: version.to_string(),
        root_requester: root_requester.clone(),
        via_chain: via_chain_from_path(path),
    }
}

fn via_chain_from_path(path: &[String]) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }

    let mut via_chain = if path.len() == 1 {
        vec![path[0].clone()]
    } else {
        path[..path.len() - 1].to_vec()
    };
    via_chain.truncate(6);
    via_chain
}

fn add_origin(origins_by_name: &mut OriginsByName, name: &str, origin: DuplicateOrigin) {
    let origins = origins_by_name.entry(name.to_string()).or_default();
    if !origins.iter().any(|existing| existing == &origin) {
        origins.push(origin);
    }
}

fn add_version(versions_by_name: &mut VersionsByName, name: &str, version: &str) {
    let versions = versions_by_name.entry(name.to_string()).or_default();
    if !versions.iter().any(|existing| existing == version) {
        versions.push(version.to_string());
    }
}

fn parse_yaml_dependency_line(line: &str, min_indent: usize) -> Option<(String, String)> {
    let indent = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    if indent < min_indent {
        return None;
    }

    let trimmed = line.trim();
    let (raw_name, raw_selector) = trimmed.split_once(':')?;
    let name = strip_wrapping_quotes(raw_name.trim()).to_string();
    let selector = strip_wrapping_quotes(raw_selector.trim()).to_string();

    if name.is_empty() || selector.is_empty() {
        return None;
    }

    Some((name, selector))
}

fn resolve_pnpm_dependency_target(
    dependency_name: &str,
    selector: &str,
) -> Option<(String, String)> {
    let trimmed = strip_wrapping_quotes(selector).trim();
    let candidate = trimmed.split_whitespace().next().unwrap_or_default();
    if candidate.is_empty()
        || candidate.starts_with("link:")
        || candidate.starts_with("file:")
        || candidate.starts_with("workspace:")
    {
        return None;
    }

    let normalized = candidate
        .split('(')
        .next()
        .unwrap_or_default()
        .trim_end_matches(',')
        .trim();

    if let Some(alias_target) = normalized.strip_prefix("npm:") {
        if let Some((name, version)) = split_descriptor(alias_target) {
            return Some((name, version));
        }

        return Some((dependency_name.to_string(), alias_target.to_string()));
    }

    Some((dependency_name.to_string(), normalized.to_string()))
}

fn resolve_pnpm_dependency(
    dependency: &DependencyRef,
    descriptor_index: &BTreeMap<String, String>,
    by_name_version: &BTreeMap<(String, String), Vec<String>>,
) -> Option<String> {
    let selector = strip_wrapping_quotes(&dependency.selector).trim();
    let candidate = selector
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_end_matches(',')
        .trim();
    if candidate.is_empty()
        || candidate.starts_with("link:")
        || candidate.starts_with("file:")
        || candidate.starts_with("workspace:")
    {
        return None;
    }

    for descriptor in pnpm_dependency_lookup_keys(&dependency.name, candidate) {
        if let Some(target_id) = descriptor_index.get(&descriptor) {
            return Some(target_id.clone());
        }
    }

    let (resolved_name, resolved_version) =
        resolve_pnpm_dependency_target(&dependency.name, candidate)?;

    unique_candidate(by_name_version.get(&(resolved_name, resolved_version)))
}

fn pnpm_dependency_lookup_keys(dependency_name: &str, selector: &str) -> Vec<String> {
    let mut keys = vec![format!("{dependency_name}@{selector}")];

    if let Some(alias_target) = selector.strip_prefix("npm:") {
        keys.push(alias_target.to_string());
    }

    dedupe_strings(keys)
}

fn yarn_dependency_lookup_keys(dependency_name: &str, selector: &str) -> Vec<String> {
    let mut keys = vec![
        format!("{dependency_name}@{selector}"),
        selector.to_string(),
    ];

    if !selector.starts_with("npm:") {
        keys.push(format!("{dependency_name}@npm:{selector}"));
        keys.push(format!("npm:{selector}"));
    }

    dedupe_strings(keys)
}

fn unique_candidate(candidates: Option<&Vec<String>>) -> Option<String> {
    let candidates = candidates?;

    (candidates.len() == 1).then(|| candidates[0].clone())
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();

    for value in values {
        if !unique.iter().any(|existing| existing == &value) {
            unique.push(value);
        }
    }

    unique
}

fn compare_versions(left: &str, right: &str) -> Ordering {
    let mut left_index = 0;
    let mut right_index = 0;

    loop {
        match (
            next_natural_part(left, left_index),
            next_natural_part(right, right_index),
        ) {
            (Some((left_part, true, next_left)), Some((right_part, true, next_right))) => {
                left_index = next_left;
                right_index = next_right;
                let ordering = compare_digit_parts(left_part, right_part);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some((left_part, false, next_left)), Some((right_part, false, next_right))) => {
                left_index = next_left;
                right_index = next_right;
                let ordering = left_part
                    .to_ascii_lowercase()
                    .cmp(&right_part.to_ascii_lowercase());
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some((_, true, _)), Some((_, false, _))) => return Ordering::Less,
            (Some((_, false, _)), Some((_, true, _))) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (None, None) => return Ordering::Equal,
        }
    }
}

fn next_natural_part(value: &str, start: usize) -> Option<(&str, bool, usize)> {
    if start >= value.len() {
        return None;
    }

    let bytes = value.as_bytes();
    let is_digit = bytes[start].is_ascii_digit();
    let mut end = start + 1;

    while end < bytes.len() && bytes[end].is_ascii_digit() == is_digit {
        end += 1;
    }

    Some((&value[start..end], is_digit, end))
}

fn compare_digit_parts(left: &str, right: &str) -> Ordering {
    let left_trimmed = left.trim_start_matches('0');
    let right_trimmed = right.trim_start_matches('0');
    let left_normalized = if left_trimmed.is_empty() {
        "0"
    } else {
        left_trimmed
    };
    let right_normalized = if right_trimmed.is_empty() {
        "0"
    } else {
        right_trimmed
    };

    left_normalized
        .len()
        .cmp(&right_normalized.len())
        .then_with(|| left_normalized.cmp(right_normalized))
}

fn normalize_package_manager(package_manager: &str) -> Option<&'static str> {
    let normalized = package_manager.to_ascii_lowercase();

    if normalized.starts_with("bun") {
        return Some("bun");
    }

    if normalized.starts_with("pnpm") {
        return Some("pnpm");
    }

    if normalized.starts_with("yarn") {
        return Some("yarn");
    }

    if normalized.starts_with("npm") {
        return Some("npm");
    }

    None
}

fn build_lockfile_warnings(
    selected_lockfile: &Lockfile,
    ignored_lockfiles: &[Lockfile],
    package_manager: &str,
) -> Vec<String> {
    let mut warnings = Vec::new();

    if ignored_lockfiles.is_empty() {
        if selected_lockfile.kind == LockfileKind::Bun {
            warnings.push(unsupported_bun_lockfile_warning(selected_lockfile));
        }
        return warnings;
    }

    let selected_name = basename(&selected_lockfile.file_path);
    let ignored_names = ignored_lockfiles
        .iter()
        .map(|lockfile| basename(&lockfile.file_path))
        .collect::<Vec<_>>();
    let package_manager_text = if !package_manager.is_empty() && package_manager != "unknown" {
        format!(r#" based on package manager "{package_manager}""#)
    } else {
        String::new()
    };

    warnings.push(format!(
        "Multiple lockfiles detected. Duplicate analysis used {selected_name}{package_manager_text} and ignored {}.",
        ignored_names.join(", ")
    ));

    if selected_lockfile.kind == LockfileKind::Bun {
        warnings.push(unsupported_bun_lockfile_warning(selected_lockfile));
    }

    warnings
}

fn basename(file_path: &Path) -> String {
    file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

fn starts_with_ascii_alpha(value: &str) -> bool {
    value
        .chars()
        .next()
        .map(|character| character.is_ascii_alphabetic())
        .unwrap_or(false)
}

fn strip_wrapping_quotes(value: &str) -> &str {
    let trimmed_prefix = value.strip_prefix('"').unwrap_or(value);
    trimmed_prefix.strip_suffix('"').unwrap_or(trimmed_prefix)
}

fn unsupported_bun_lockfile_warning(lockfile: &Lockfile) -> String {
    format!(
        "Detected {}, but duplicate analysis does not yet parse Bun lockfiles, so results may be incomplete.",
        basename(&lockfile.file_path)
    )
}
