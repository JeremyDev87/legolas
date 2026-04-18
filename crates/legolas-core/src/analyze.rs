use std::{
    cmp::Reverse,
    fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use crate::{
    error::Result,
    impact::estimate_impact,
    import_scanner::{collect_source_files, scan_imports, SourceAnalysis},
    lockfiles::parse_duplicate_packages,
    models::{
        Analysis, HeavyDependency, LazyLoadCandidate, Metadata, PackageSummary, SourceSummary,
        TreeShakingWarning, UnusedDependencyCandidate,
    },
    package_intelligence::get_package_intel,
    project_shape::{detect_frameworks, detect_package_manager},
    workspace::{find_project_root, read_json_if_exists},
    LegolasError,
};

static CANDIDATE_FILES_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(modal|chart|editor|map|viewer|dashboard|settings|admin|page|route|dialog|drawer|popover)")
        .expect("valid candidate files regex")
});

const KNOWN_BUNDLE_ARTIFACTS: [&str; 5] = [
    "stats.json",
    "dist/stats.json",
    "build/stats.json",
    "meta.json",
    "dist/meta.json",
];

pub fn analyze_project<P: AsRef<Path>>(input_path: P) -> Result<Analysis> {
    let project_root = find_project_root(input_path)?;
    let manifest: Value = read_json_if_exists(project_root.join("package.json"))?
        .ok_or_else(|| LegolasError::PackageJsonMissing(project_root.display().to_string()))?;

    let package_manager = detect_package_manager(&project_root, &manifest)?;
    let frameworks = detect_frameworks(&project_root, &manifest)?;
    let source_files = collect_source_files(&project_root)?;
    let source_analysis = scan_imports(&project_root, &source_files)?;
    let duplicate_analysis = parse_duplicate_packages(&project_root, &package_manager)?;
    let heavy_dependencies = build_heavy_dependency_report(&manifest, &source_analysis);
    let lazy_load_candidates = build_lazy_load_candidates(&source_analysis, &heavy_dependencies);
    let tree_shaking_warnings = build_tree_shaking_warnings(&source_analysis);
    let bundle_artifacts = detect_bundle_artifacts(&project_root)?;
    let impact = estimate_impact(
        &heavy_dependencies,
        &duplicate_analysis.duplicates,
        &lazy_load_candidates,
        &tree_shaking_warnings,
    );

    Ok(Analysis {
        project_root: project_root.to_string_lossy().to_string(),
        package_manager,
        frameworks,
        bundle_artifacts: bundle_artifacts.clone(),
        package_summary: build_package_summary(&manifest),
        source_summary: SourceSummary {
            files_scanned: source_files.len(),
            imported_packages: source_analysis.imported_packages.len(),
            dynamic_imports: source_analysis.dynamic_import_count,
        },
        heavy_dependencies,
        duplicate_packages: duplicate_analysis.duplicates,
        lazy_load_candidates,
        tree_shaking_warnings,
        unused_dependency_candidates: build_unused_dependency_candidates(
            &manifest,
            &source_analysis,
        ),
        warnings: duplicate_analysis.warnings,
        impact,
        metadata: Metadata {
            mode: if bundle_artifacts.is_empty() {
                "heuristic".to_string()
            } else {
                "artifact-assisted".to_string()
            },
            generated_at: generated_at_string(),
        },
    })
}

fn build_package_summary(manifest: &Value) -> PackageSummary {
    PackageSummary {
        name: manifest
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown-project")
            .to_string(),
        dependency_count: manifest
            .get("dependencies")
            .and_then(Value::as_object)
            .map(|entries| entries.len())
            .unwrap_or(0),
        dev_dependency_count: manifest
            .get("devDependencies")
            .and_then(Value::as_object)
            .map(|entries| entries.len())
            .unwrap_or(0),
    }
}

fn build_heavy_dependency_report(
    manifest: &Value,
    source_analysis: &SourceAnalysis,
) -> Vec<HeavyDependency> {
    let mut heavy_dependencies = Vec::new();

    for (name, version_range) in merged_dependency_entries(manifest) {
        let Some(intel) = get_package_intel(&name) else {
            continue;
        };

        let import_info = source_analysis.by_package.get(&name);
        heavy_dependencies.push(HeavyDependency {
            name,
            version_range,
            estimated_kb: intel.estimated_kb,
            category: intel.category.to_string(),
            rationale: intel.rationale.to_string(),
            recommendation: intel.recommendation.to_string(),
            imported_by: import_info
                .map(|item| item.files.clone())
                .unwrap_or_default(),
            dynamic_imported_by: import_info
                .map(|item| item.dynamic_files.clone())
                .unwrap_or_default(),
            import_count: import_info.map(|item| item.files.len()).unwrap_or(0),
        });
    }

    heavy_dependencies.sort_by_key(|item| Reverse(item.estimated_kb));
    heavy_dependencies
}

fn merged_dependency_entries(manifest: &Value) -> Vec<(String, String)> {
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

fn build_lazy_load_candidates(
    source_analysis: &SourceAnalysis,
    heavy_dependencies: &[HeavyDependency],
) -> Vec<LazyLoadCandidate> {
    let heavy_by_name = heavy_dependencies
        .iter()
        .map(|item| (item.name.as_str(), item))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut candidates = Vec::new();

    for imported_package in &source_analysis.imported_packages {
        let Some(heavy) = heavy_by_name.get(imported_package.name.as_str()) else {
            continue;
        };

        let split_friendly_files = imported_package
            .static_files
            .iter()
            .filter(|file| CANDIDATE_FILES_PATTERN.is_match(file))
            .cloned()
            .collect::<Vec<_>>();

        if split_friendly_files.is_empty() || !imported_package.dynamic_files.is_empty() {
            continue;
        }

        candidates.push(LazyLoadCandidate {
            name: imported_package.name.clone(),
            estimated_savings_kb: (heavy.estimated_kb as f64 * 0.75).round() as usize,
            recommendation: heavy.recommendation.clone(),
            files: split_friendly_files,
            reason: format!(
                "{} is statically imported in UI surfaces that usually tolerate lazy loading",
                imported_package.name
            ),
        });
    }

    candidates.sort_by_key(|item| Reverse(item.estimated_savings_kb));
    candidates
}

fn build_tree_shaking_warnings(source_analysis: &SourceAnalysis) -> Vec<TreeShakingWarning> {
    let mut warnings = source_analysis.tree_shaking_warnings.clone();
    warnings.sort_by_key(|item| Reverse(item.estimated_kb));
    warnings
}

fn build_unused_dependency_candidates(
    manifest: &Value,
    source_analysis: &SourceAnalysis,
) -> Vec<UnusedDependencyCandidate> {
    let used_packages = source_analysis
        .imported_packages
        .iter()
        .map(|item| item.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut candidates = Vec::new();

    if let Some(entries) = manifest.get("dependencies").and_then(Value::as_object) {
        for (name, version_range) in entries {
            let Some(version_range) = version_range.as_str() else {
                continue;
            };

            if used_packages.contains(name.as_str()) {
                continue;
            }

            candidates.push(UnusedDependencyCandidate {
                name: name.clone(),
                version_range: version_range.to_string(),
            });
        }
    }

    candidates.sort_by(|left, right| left.name.cmp(&right.name));
    candidates
}

fn detect_bundle_artifacts(project_root: &Path) -> Result<Vec<String>> {
    let mut detected = Vec::new();

    for relative_path in KNOWN_BUNDLE_ARTIFACTS {
        let absolute_path = project_root.join(relative_path);
        if let Ok(metadata) = fs::metadata(&absolute_path) {
            if metadata.is_file() {
                detected.push(relative_path.to_string());
            }
        }
    }

    Ok(detected)
}

fn generated_at_string() -> String {
    format_iso8601_utc(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default(),
    )
}

fn format_iso8601_utc(duration: Duration) -> String {
    let total_seconds = duration.as_secs() as i64;
    let milliseconds = duration.subsec_millis();
    let days = total_seconds.div_euclid(86_400);
    let seconds_of_day = total_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milliseconds:03}Z")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    // Convert Unix days to Gregorian UTC using Howard Hinnant's civil date algorithm.
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let normalized_year = year + if month <= 2 { 1 } else { 0 };

    (normalized_year as i32, month as u32, day as u32)
}
