use std::{
    cmp::Ordering,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};

use crate::{
    error::Result,
    models::DuplicatePackage,
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

static PNPM_ENTRY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^ {2,}'?(@?[^:'\s][^:]*?)'?:\s*$").expect("valid pnpm entry regex"));
static DESCRIPTOR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(@[^/]+/[^@]+|[^@]+)@(.+)$").expect("valid descriptor regex"));
static YARN_ALIAS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@npm:(@[^/]+/[^@]+|[^@]+)@").expect("valid yarn alias regex"));
static YARN_VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^ {2}version "(.*)"$"#).expect("valid yarn version regex"));
static YARN_BERRY_VERSION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^ {2}version:\s+"?([^"]+)"?$"#).expect("valid yarn berry version regex")
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

fn collect_from_package_lock(package_lock: Option<Value>) -> VersionsByName {
    let mut versions_by_name = BTreeMap::new();
    let Some(package_lock) = package_lock else {
        return versions_by_name;
    };

    let Some(package_lock_object) = package_lock.as_object() else {
        return versions_by_name;
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

                let Some(start_index) = package_path.rfind("node_modules/") else {
                    continue;
                };
                let package_name = &package_path[start_index + "node_modules/".len()..];
                add_version(&mut versions_by_name, package_name, version);
            }

            return versions_by_name;
        }
    }

    if let Some(dependencies) = package_lock_object
        .get("dependencies")
        .and_then(Value::as_object)
    {
        collect_from_package_lock_dependencies(dependencies, &mut versions_by_name);
    }

    versions_by_name
}

fn collect_from_package_lock_dependencies(
    dependencies: &Map<String, Value>,
    versions_by_name: &mut VersionsByName,
) {
    for (name, metadata) in dependencies {
        if let Some(version) = metadata
            .get("version")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            add_version(versions_by_name, name, version);
        }

        if let Some(child_dependencies) = metadata.get("dependencies").and_then(Value::as_object) {
            collect_from_package_lock_dependencies(child_dependencies, versions_by_name);
        }
    }
}

fn collect_from_pnpm_lock(content: &str) -> VersionsByName {
    let mut versions_by_name = BTreeMap::new();
    let mut inside_packages = false;

    for raw_line in content.split('\n') {
        let line = raw_line.trim_end_matches('\r');

        if line == "packages:" || line == "snapshots:" {
            inside_packages = true;
            continue;
        }

        if inside_packages && starts_with_ascii_alpha(line) {
            inside_packages = false;
        }

        if !inside_packages {
            continue;
        }

        let Some(captures) = PNPM_ENTRY_RE.captures(line) else {
            continue;
        };
        let mut descriptor = captures[1].to_string();
        if let Some(stripped) = descriptor.strip_prefix('/') {
            descriptor = stripped.to_string();
        }

        let Some((name, version)) = split_descriptor(&descriptor) else {
            continue;
        };
        add_version(&mut versions_by_name, &name, &version);
    }

    versions_by_name
}

fn collect_from_yarn_lock(content: &str) -> VersionsByName {
    let mut versions_by_name = BTreeMap::new();
    let mut current_package_names: Vec<String> = Vec::new();
    let mut current_version: Option<String> = None;

    for raw_line in content.split('\n') {
        let line = raw_line.trim_end_matches('\r');

        if line.trim().is_empty() {
            flush_current_yarn_entry(
                &mut versions_by_name,
                &current_package_names,
                current_version.as_deref(),
            );
            current_package_names.clear();
            current_version = None;
            continue;
        }

        if !line.starts_with(' ') {
            flush_current_yarn_entry(
                &mut versions_by_name,
                &current_package_names,
                current_version.as_deref(),
            );
            current_package_names = parse_yarn_header(line);
            current_version = None;
            continue;
        }

        if let Some(captures) = YARN_VERSION_RE.captures(line) {
            current_version = Some(captures[1].to_string());
            continue;
        }

        if let Some(captures) = YARN_BERRY_VERSION_RE.captures(line) {
            current_version = Some(captures[1].to_string());
        }
    }

    flush_current_yarn_entry(
        &mut versions_by_name,
        &current_package_names,
        current_version.as_deref(),
    );
    versions_by_name
}

fn parse_yarn_header(line: &str) -> Vec<String> {
    let trimmed = line.strip_suffix(':').unwrap_or(line);

    SPLIT_HEADER_RE
        .split(trimmed)
        .map(str::trim)
        .map(strip_wrapping_quotes)
        .filter_map(extract_yarn_package_name)
        .collect()
}

fn flush_current_yarn_entry(
    versions_by_name: &mut VersionsByName,
    package_names: &[String],
    version: Option<&str>,
) {
    let Some(version) = version else {
        return;
    };
    if package_names.is_empty() {
        return;
    }

    for package_name in package_names {
        add_version(versions_by_name, package_name, version);
    }
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

fn extract_yarn_package_name(descriptor: &str) -> Option<String> {
    if let Some(captures) = YARN_ALIAS_RE.captures(descriptor) {
        return Some(captures[1].to_string());
    }

    split_descriptor(descriptor).map(|(name, _)| name)
}

fn summarize_duplicates(versions_by_name: VersionsByName) -> Vec<DuplicatePackage> {
    let mut results = Vec::new();

    for (name, mut versions) in versions_by_name {
        if versions.len() < 2 {
            continue;
        }

        versions.sort_by(|left, right| compare_versions(left, right));
        let estimated_extra_kb = usize::max((versions.len().saturating_sub(1)) * 18, 18);

        results.push(DuplicatePackage {
            name,
            count: versions.len(),
            versions,
            estimated_extra_kb,
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

fn add_version(versions_by_name: &mut VersionsByName, name: &str, version: &str) {
    let versions = versions_by_name.entry(name.to_string()).or_default();
    if !versions.iter().any(|existing| existing == version) {
        versions.push(version.to_string());
    }
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
