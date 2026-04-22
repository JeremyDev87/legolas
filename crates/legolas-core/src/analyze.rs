use std::{
    cmp::Reverse,
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

use crate::{
    action_plan::apply_action_plan,
    artifacts::{
        detect::parse_artifact_file, detect::KNOWN_ARTIFACT_FILES, merge_artifact_source_signals,
        ArtifactSummary,
    },
    boundaries::{collect_boundary_warnings, Phase8SeedContext},
    confidence::{score_duplicate_package, score_heavy_dependency, score_lazy_load_candidate},
    error::Result,
    findings::{FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata},
    impact::estimate_impact,
    import_scanner::{
        collect_source_files, scan_imports_with_aliases, ImportedPackageRecord, SourceAnalysis,
    },
    lockfiles::parse_duplicate_packages,
    models::{
        Analysis, HeavyDependency, LazyLoadCandidate, Metadata, PackageSummary, SourceSummary,
        TreeShakingWarning, UnusedDependencyCandidate,
    },
    package_intelligence::get_package_intel,
    project_shape::{detect_frameworks, detect_package_manager},
    route_context::{classify_route_context, RouteContextKind},
    workspace::{find_project_root, load_alias_config, read_json_if_exists},
    workspaces::collect_workspace_summaries,
    LegolasError,
};

static CANDIDATE_FILES_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(modal|chart|editor|map|viewer|dashboard|settings|admin|page|route|dialog|drawer|popover)")
        .expect("valid candidate files regex")
});

pub fn analyze_project<P: AsRef<Path>>(input_path: P) -> Result<Analysis> {
    let project_root = find_project_root(input_path)?;
    let manifest: Value = read_json_if_exists(project_root.join("package.json"))?
        .ok_or_else(|| LegolasError::PackageJsonMissing(project_root.display().to_string()))?;

    let package_manager = detect_package_manager(&project_root, &manifest)?;
    let frameworks = detect_frameworks(&project_root, &manifest)?;
    let alias_config = load_alias_config(&project_root)?;
    let source_files = collect_source_files(&project_root)?;
    let source_analysis = scan_imports_with_aliases(
        &project_root,
        &source_files,
        alias_config.as_ref().map(|loaded| &loaded.config),
    )?;
    let mut duplicate_analysis = parse_duplicate_packages(&project_root, &package_manager)?;
    enrich_duplicate_package_findings(&mut duplicate_analysis.duplicates);
    let heavy_dependencies = build_heavy_dependency_report(&manifest, &source_analysis);
    let lazy_load_candidates = build_lazy_load_candidates(
        &project_root,
        &frameworks,
        &source_analysis,
        &heavy_dependencies,
    );
    let tree_shaking_warnings = build_tree_shaking_warnings(&source_analysis);
    let artifact_assist = collect_artifact_assist(&project_root)?;
    let phase8_context = Phase8SeedContext {
        project_root: &project_root,
        package_manager: &package_manager,
        frameworks: &frameworks,
        bundle_artifacts: &artifact_assist.bundle_artifacts,
        source_analysis: &source_analysis,
        source_file_count: source_files.len(),
        imported_package_count: source_analysis.imported_packages.len(),
        dynamic_import_count: source_analysis.dynamic_import_count,
    };
    let boundary_warnings = collect_boundary_warnings(&phase8_context);
    let workspace_summaries = collect_workspace_summaries(&phase8_context);
    let mut heavy_dependencies = heavy_dependencies;
    if let Some(artifact_summary) = artifact_assist.artifact_summary.as_ref() {
        let merged_signals =
            merge_artifact_source_signals(artifact_summary, &source_analysis, &heavy_dependencies);
        apply_merged_artifact_signals(&mut heavy_dependencies, &merged_signals);
    }
    let impact = estimate_impact(
        &heavy_dependencies,
        &duplicate_analysis.duplicates,
        &lazy_load_candidates,
        &tree_shaking_warnings,
    );

    let mut analysis = Analysis {
        project_root: project_root.to_string_lossy().to_string(),
        package_manager,
        frameworks,
        bundle_artifacts: artifact_assist.bundle_artifacts.clone(),
        artifact_summary: artifact_assist.artifact_summary.clone(),
        package_summary: build_package_summary(&manifest),
        source_summary: SourceSummary {
            files_scanned: source_files.len(),
            imported_packages: source_analysis.imported_packages.len(),
            dynamic_imports: source_analysis.dynamic_import_count,
        },
        boundary_warnings,
        workspace_summaries,
        heavy_dependencies,
        duplicate_packages: duplicate_analysis.duplicates,
        lazy_load_candidates,
        tree_shaking_warnings,
        unused_dependency_candidates: build_unused_dependency_candidates(
            &manifest,
            &source_analysis,
        ),
        warnings: merge_warnings(duplicate_analysis.warnings, artifact_assist.warnings),
        impact,
        metadata: Metadata {
            mode: if artifact_assist.artifact_summary.is_none() {
                "heuristic".to_string()
            } else {
                "artifact-assisted".to_string()
            },
            generated_at: generated_at_string(),
        },
    };
    apply_action_plan(&mut analysis);

    Ok(analysis)
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
            name: name.clone(),
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
            finding: build_heavy_dependency_finding(&name, intel.rationale, import_info),
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
    project_root: &Path,
    frameworks: &[String],
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

        let classified_static_files = imported_package
            .static_files
            .iter()
            .map(|file| {
                (
                    file.clone(),
                    classify_route_context(project_root, frameworks, Path::new(file)),
                )
            })
            .collect::<Vec<_>>();
        let route_context_files = classified_static_files
            .iter()
            .filter(|(_, route_kind)| is_lazy_load_route_context(*route_kind))
            .map(|(file, route_kind)| (file.clone(), *route_kind))
            .collect::<Vec<_>>();
        let has_shared_component_import = classified_static_files
            .iter()
            .any(|(_, route_kind)| *route_kind == RouteContextKind::SharedComponent);
        let heuristic_files = classified_static_files
            .iter()
            .filter(|(file, route_kind)| {
                *route_kind != RouteContextKind::SharedComponent
                    && CANDIDATE_FILES_PATTERN.is_match(file)
            })
            .map(|(file, _)| file.clone())
            .collect::<Vec<_>>();

        if !route_context_files.is_empty() && has_shared_component_import {
            continue;
        }

        let split_friendly_files = if route_context_files.is_empty() {
            heuristic_files.clone()
        } else {
            route_context_files
                .iter()
                .map(|(file, _)| file.clone())
                .collect::<Vec<_>>()
        };

        if split_friendly_files.is_empty() || !imported_package.dynamic_files.is_empty() {
            continue;
        }

        let reason = lazy_load_reason(&imported_package.name, &route_context_files);
        candidates.push(LazyLoadCandidate {
            name: imported_package.name.clone(),
            estimated_savings_kb: lazy_load_estimated_savings_kb(
                heavy.estimated_kb,
                &route_context_files,
            ),
            recommendation: heavy.recommendation.clone(),
            files: split_friendly_files,
            reason,
            finding: build_lazy_load_finding(
                &imported_package.name,
                &heuristic_files,
                &route_context_files,
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

fn enrich_duplicate_package_findings(duplicates: &mut [crate::models::DuplicatePackage]) {
    for duplicate in duplicates {
        duplicate.finding = FindingMetadata::new(
            format!("duplicate-package:{}", duplicate.name),
            FindingAnalysisSource::LockfileTrace,
        )
        .with_confidence(score_duplicate_package());
    }
}

fn build_heavy_dependency_finding(
    package_name: &str,
    rationale: &str,
    import_info: Option<&ImportedPackageRecord>,
) -> FindingMetadata {
    let analysis_source = import_info
        .filter(|item| !item.files.is_empty())
        .map(|_| FindingAnalysisSource::SourceImport)
        .unwrap_or(FindingAnalysisSource::Heuristic);
    let mut evidence = Vec::new();

    if let Some(import_info) = import_info {
        evidence.extend(
            import_info
                .static_files
                .iter()
                .map(|file| {
                    FindingEvidence::new("source-file")
                        .with_file(file.clone())
                        .with_specifier(package_name.to_string())
                        .with_detail(format!("static import; {rationale}"))
                })
                .collect::<Vec<_>>(),
        );
        evidence.extend(
            import_info
                .dynamic_files
                .iter()
                .map(|file| {
                    FindingEvidence::new("source-file")
                        .with_file(file.clone())
                        .with_specifier(package_name.to_string())
                        .with_detail(format!("dynamic import; {rationale}"))
                })
                .collect::<Vec<_>>(),
        );
    }

    FindingMetadata::new(format!("heavy-dependency:{package_name}"), analysis_source)
        .with_confidence(score_heavy_dependency(import_info))
        .with_evidence(evidence)
}

fn apply_merged_artifact_signals(
    heavy_dependencies: &mut [HeavyDependency],
    merged_signals: &[crate::artifacts::ArtifactSourceSignal],
) {
    let signals_by_package = merged_signals
        .iter()
        .map(|signal| (signal.package_name.as_str(), signal))
        .collect::<BTreeMap<_, _>>();

    for dependency in heavy_dependencies {
        let Some(signal) = signals_by_package.get(dependency.name.as_str()) else {
            continue;
        };

        let analysis_source = match signal.kind {
            crate::artifacts::ArtifactSignalKind::Source => continue,
            crate::artifacts::ArtifactSignalKind::Artifact => FindingAnalysisSource::Artifact,
            crate::artifacts::ArtifactSignalKind::ArtifactSource => {
                FindingAnalysisSource::ArtifactSource
            }
        };

        dependency.finding.analysis_source = Some(analysis_source);
        dependency.finding.evidence.extend(
            signal
                .evidence()
                .into_iter()
                .skip(signal.source_files.len()),
        );
    }
}

fn build_lazy_load_finding(
    package_name: &str,
    heuristic_files: &[String],
    route_context_files: &[(String, RouteContextKind)],
) -> FindingMetadata {
    let evidence = if route_context_files.is_empty() {
        heuristic_files
            .iter()
            .map(|file| {
                FindingEvidence::new("source-file")
                    .with_file(file.clone())
                    .with_specifier(package_name.to_string())
                    .with_detail(lazy_load_surface_detail(file))
            })
            .collect::<Vec<_>>()
    } else {
        route_context_files
            .iter()
            .map(|(file, route_kind)| {
                FindingEvidence::new("source-file")
                    .with_file(file.clone())
                    .with_specifier(package_name.to_string())
                    .with_detail(route_context_surface_detail(*route_kind))
            })
            .collect::<Vec<_>>()
    };

    FindingMetadata::new(
        format!("lazy-load:{package_name}"),
        FindingAnalysisSource::Heuristic,
    )
    .with_confidence(lazy_load_candidate_confidence(route_context_files))
    .with_evidence(evidence)
}

fn lazy_load_reason(
    package_name: &str,
    route_context_files: &[(String, RouteContextKind)],
) -> String {
    if route_context_files.is_empty() {
        format!(
            "{} is statically imported in UI surfaces that usually tolerate lazy loading",
            package_name
        )
    } else {
        format!(
            "{} is statically imported in route-aware UI surfaces that usually tolerate lazy loading",
            package_name
        )
    }
}

fn lazy_load_estimated_savings_kb(
    estimated_kb: usize,
    route_context_files: &[(String, RouteContextKind)],
) -> usize {
    let multiplier = if route_context_files.is_empty() {
        0.75
    } else {
        0.80
    };

    (estimated_kb as f64 * multiplier).round() as usize
}

fn lazy_load_candidate_confidence(
    route_context_files: &[(String, RouteContextKind)],
) -> FindingConfidence {
    if route_context_files.is_empty() {
        FindingConfidence::Low
    } else {
        score_lazy_load_candidate()
    }
}

fn lazy_load_surface_detail(file: &str) -> String {
    let normalized = file.to_ascii_lowercase();

    CANDIDATE_FILES_PATTERN
        .find(&normalized)
        .map(|matched| {
            format!(
                "route-like UI surface matched `{}` keyword",
                matched.as_str()
            )
        })
        .unwrap_or_else(|| "route-like UI surface heuristic".to_string())
}

fn route_context_surface_detail(route_kind: RouteContextKind) -> String {
    let label = match route_kind {
        RouteContextKind::RoutePage => "route-page",
        RouteContextKind::RouteLayout => "route-layout",
        RouteContextKind::AdminSurface => "admin-surface",
        RouteContextKind::SharedComponent => "shared-component",
        RouteContextKind::NonRoute => "non-route",
    };

    format!("route context classified `{label}`")
}

fn is_lazy_load_route_context(route_kind: RouteContextKind) -> bool {
    matches!(
        route_kind,
        RouteContextKind::RoutePage
            | RouteContextKind::RouteLayout
            | RouteContextKind::AdminSurface
    )
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

#[derive(Debug, Default)]
struct ArtifactAssist {
    bundle_artifacts: Vec<String>,
    artifact_summary: Option<ArtifactSummary>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct ParsedArtifactCandidate {
    relative_path: String,
    modified_at: SystemTime,
    summary: ArtifactSummary,
}

fn collect_artifact_assist(project_root: &Path) -> Result<ArtifactAssist> {
    let mut assist = ArtifactAssist::default();
    let mut parsed_candidates = Vec::new();

    for relative_path in KNOWN_ARTIFACT_FILES {
        let absolute_path = project_root.join(relative_path);
        let Ok(metadata) = fs::metadata(&absolute_path) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }

        assist.bundle_artifacts.push(relative_path.to_string());

        match parse_artifact_file(&absolute_path) {
            Ok(summary) => parsed_candidates.push(ParsedArtifactCandidate {
                relative_path: relative_path.to_string(),
                modified_at: metadata.modified().unwrap_or(UNIX_EPOCH),
                summary: summary.normalized(),
            }),
            Err(error) => assist.warnings.push(format!(
                "Bundle artifact `{relative_path}` could not be parsed: {error}"
            )),
        }
    }

    if let Some(selected) = select_artifact_summary(&parsed_candidates) {
        if parsed_candidates.len() > 1 {
            assist.warnings.push(format!(
                "Multiple bundle artifacts were parsed; artifactSummary selected `{}` by latest modification time.",
                selected.relative_path
            ));
        }
        assist.artifact_summary = Some(selected.summary.clone());
    }

    Ok(assist)
}

fn select_artifact_summary(
    candidates: &[ParsedArtifactCandidate],
) -> Option<&ParsedArtifactCandidate> {
    candidates.iter().max_by(|left, right| {
        left.modified_at
            .cmp(&right.modified_at)
            .then_with(|| right.relative_path.cmp(&left.relative_path))
    })
}

fn merge_warnings(primary: Vec<String>, secondary: Vec<String>) -> Vec<String> {
    primary.into_iter().chain(secondary).collect()
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
