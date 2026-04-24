use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use legolas_cli::{
    argv::{self, Command},
    reporters::{
        sarif::{ci_sarif_output, scan_sarif_output},
        text::{
            format_budget_report, format_ci_report, format_optimize_report, format_scan_report,
            format_visualization_report,
        },
    },
};
use legolas_core::{
    analyze_project,
    baseline::{boundary_warning_key, diff_analysis, BaselineSnapshot},
    budget::{evaluate_budget, BudgetEvaluation},
    config::{load_config_file, load_discovered_config, LoadedConfig},
    impact::estimate_impact,
    LegolasError, Result,
};
use serde_json::{json, Map, Value};

const ANALYSIS_SCHEMA_VERSION: &str = "legolas.analysis.v1";
const BUDGET_SCHEMA_VERSION: &str = "legolas.budget.v1";
const CI_SCHEMA_VERSION: &str = "legolas.ci.v1";

const HELP_TEXT: &str = r#"Legolas
Slim bundles with precision.

Usage:
  legolas scan [path] [--config file] [--json | --sarif] [--write-baseline file] [--baseline file --regression-only]
  legolas visualize [path] [--config file] [--limit 10]
  legolas optimize [path] [--config file] [--top 5] [--json] [--baseline file --regression-only]
  legolas budget [path] [--config file] [--json] [--baseline file --regression-only]
  legolas ci [path] [--config file] [--json | --sarif] [--baseline file --regression-only]
  legolas help

Examples:
  legolas scan .
  legolas scan ./apps/storefront --sarif
  legolas scan ./apps/storefront --write-baseline ./baseline.json --json
  legolas scan ./apps/storefront --baseline ./baseline.json --regression-only --json
  legolas scan --config ./legolas.config.json
  legolas visualize ./apps/storefront --limit 12
  legolas optimize ./apps/storefront --top 7 --baseline ./baseline.json --regression-only
  legolas budget ./apps/storefront --baseline ./baseline.json --regression-only --json
  legolas ci ./apps/storefront --baseline ./baseline.json --regression-only --sarif
"#;

fn main() {
    match run() {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(error) => {
            eprintln!("legolas: {error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let parsed = argv::parse_argv(std::env::args().skip(1))?;

    if parsed.version {
        println!("{}", read_package_version()?);
        return Ok(0);
    }

    if parsed.help || parsed.command.is_none() || matches!(parsed.command, Some(Command::Help)) {
        print!("{HELP_TEXT}");
        return Ok(0);
    }

    let command = parsed.command.clone().expect("command already checked");
    if let Command::Unknown(command) = command {
        return Err(LegolasError::CliUsage(format!(
            "unknown command \"{command}\""
        )));
    }

    let loaded_config = resolve_loaded_config(&parsed)?;
    emit_config_warnings(
        &command,
        loaded_config.as_ref(),
        parsed.json || parsed.sarif,
    );
    let target_path = resolve_target_path(&parsed, loaded_config.as_ref())?;
    let analysis = analyze_project(&target_path)?;
    let baseline = resolve_baseline_snapshot(&parsed)?;
    let (output_analysis, regression_diff) = if parsed.regression_only {
        let Some(baseline) = baseline.as_ref() else {
            return Err(LegolasError::CliUsage(
                "--regression-only requires --baseline".to_string(),
            ));
        };

        (
            regression_only_analysis(&analysis, baseline),
            Some(diff_analysis(baseline, &analysis)),
        )
    } else {
        (analysis.clone(), None)
    };
    let mut budget_evaluation =
        resolve_budget_evaluation(&command, &output_analysis, loaded_config.as_ref());
    if let Some(diff) = regression_diff.as_ref() {
        budget_evaluation = budget_evaluation
            .map(|evaluation| filter_regression_budget_evaluation(evaluation, diff));
    }

    if let Some(write_baseline_path) = &parsed.write_baseline_path {
        write_baseline_snapshot(write_baseline_path, &analysis)?;
    }

    if parsed.sarif {
        let output = match command {
            Command::Scan => scan_sarif_output(&output_analysis),
            Command::Ci => ci_sarif_output(
                &output_analysis,
                budget_evaluation
                    .as_ref()
                    .expect("budget evaluation exists for ci command"),
                regression_diff.as_ref(),
            ),
            Command::Visualize | Command::Optimize | Command::Budget => {
                unreachable!("argv validation already restricts --sarif")
            }
            Command::Help | Command::Unknown(_) => unreachable!("handled above"),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);

        if matches!(command, Command::Ci)
            && budget_evaluation
                .as_ref()
                .expect("budget evaluation exists for ci command")
                .has_failures()
        {
            eprintln!(
                "{}",
                ci_failure_message(
                    budget_evaluation
                        .as_ref()
                        .expect("budget evaluation exists for ci command"),
                )
            );
            return Ok(1);
        }

        return Ok(0);
    }

    if parsed.json {
        let output = match command {
            Command::Budget => budget_json_output(
                &output_analysis,
                budget_evaluation
                    .as_ref()
                    .expect("budget evaluation exists for budget command"),
            ),
            Command::Ci => ci_json_output(
                &output_analysis,
                budget_evaluation
                    .as_ref()
                    .expect("budget evaluation exists for ci command"),
                regression_diff.as_ref(),
            ),
            _ => analysis_json_output(&output_analysis)?,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);

        if matches!(command, Command::Ci)
            && budget_evaluation
                .as_ref()
                .expect("budget evaluation exists for ci command")
                .has_failures()
        {
            eprintln!(
                "{}",
                ci_failure_message(
                    budget_evaluation
                        .as_ref()
                        .expect("budget evaluation exists for ci command"),
                )
            );
            return Ok(1);
        }

        return Ok(0);
    }

    let output = match command {
        Command::Scan => format_scan_report(&output_analysis),
        Command::Visualize => format_visualization_report(
            &output_analysis,
            resolve_visualize_limit(&parsed, loaded_config.as_ref()),
        ),
        Command::Optimize => format_optimize_report(
            &output_analysis,
            resolve_optimize_top(&parsed, loaded_config.as_ref()),
        ),
        Command::Budget => format_budget_report(
            &output_analysis,
            budget_evaluation
                .as_ref()
                .expect("budget evaluation exists for budget command"),
        ),
        Command::Ci => format_ci_report(
            &output_analysis,
            budget_evaluation
                .as_ref()
                .expect("budget evaluation exists for ci command"),
        ),
        Command::Help | Command::Unknown(_) => unreachable!("handled above"),
    };

    println!("{output}");
    if matches!(command, Command::Ci)
        && budget_evaluation
            .as_ref()
            .expect("budget evaluation exists for ci command")
            .has_failures()
    {
        eprintln!(
            "{}",
            ci_failure_message(
                budget_evaluation
                    .as_ref()
                    .expect("budget evaluation exists for ci command"),
            )
        );
        return Ok(1);
    }

    Ok(0)
}

fn resolve_baseline_snapshot(parsed: &argv::CliArgs) -> Result<Option<BaselineSnapshot>> {
    let Some(baseline_path) = &parsed.baseline_path else {
        return Ok(None);
    };

    let raw_baseline = fs::read_to_string(baseline_path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => {
            LegolasError::PathNotFound(baseline_path.display().to_string())
        }
        _ => error.into(),
    })?;
    let snapshot: BaselineSnapshot = serde_json::from_str(&raw_baseline).map_err(|error| {
        LegolasError::CliUsage(format!(
            "malformed baseline {}: {}",
            baseline_path.display(),
            error
        ))
    })?;

    if snapshot.schema_version != legolas_core::baseline::BASELINE_SCHEMA_VERSION {
        return Err(LegolasError::CliUsage(format!(
            "unsupported baseline schema version: {} (expected {}; regenerate with --write-baseline)",
            snapshot.schema_version,
            legolas_core::baseline::BASELINE_SCHEMA_VERSION
        )));
    }

    Ok(Some(snapshot))
}

fn write_baseline_snapshot(path: &Path, analysis: &legolas_core::Analysis) -> Result<()> {
    let snapshot = BaselineSnapshot::from_analysis(analysis);
    fs::write(path, serde_json::to_string_pretty(&snapshot)?).map_err(Into::into)
}

fn analysis_json_output(analysis: &legolas_core::Analysis) -> Result<Value> {
    let mut output = Map::new();
    output.insert("schemaVersion".to_string(), json!(ANALYSIS_SCHEMA_VERSION));
    output.extend(analysis_value_to_object(analysis)?);

    Ok(Value::Object(output))
}

fn budget_json_output(analysis: &legolas_core::Analysis, evaluation: &BudgetEvaluation) -> Value {
    let mut output = Map::new();
    output.insert("schemaVersion".to_string(), json!(BUDGET_SCHEMA_VERSION));
    output.insert(
        "overallStatus".to_string(),
        json!(evaluation.overall_status),
    );
    output.insert("rules".to_string(), json!(evaluation.rules));

    if !analysis.workspace_summaries.is_empty() {
        output.insert(
            "workspaceSummaries".to_string(),
            json!(analysis.workspace_summaries),
        );
    }

    Value::Object(output)
}

fn ci_json_output(
    analysis: &legolas_core::Analysis,
    evaluation: &BudgetEvaluation,
    regression_diff: Option<&legolas_core::BaselineDiff>,
) -> Value {
    let mut output = Map::new();
    output.insert("schemaVersion".to_string(), json!(CI_SCHEMA_VERSION));
    output.insert("passed".to_string(), json!(!evaluation.has_failures()));
    output.insert(
        "overallStatus".to_string(),
        json!(evaluation.overall_status),
    );
    output.insert("rules".to_string(), json!(evaluation.rules));

    if !analysis.workspace_summaries.is_empty() {
        output.insert(
            "workspaceSummaries".to_string(),
            json!(analysis.workspace_summaries),
        );
    }

    if let Some(diff) = regression_diff {
        output.insert(
            "regression".to_string(),
            json!({
                "mode": "regression-only",
                "baselineDiff": diff,
            }),
        );
    }

    Value::Object(output)
}

fn analysis_value_to_object(analysis: &legolas_core::Analysis) -> Result<Map<String, Value>> {
    let Value::Object(object) = serde_json::to_value(analysis)? else {
        unreachable!("serialized JSON output must be an object");
    };

    Ok(object)
}

fn read_package_version() -> Result<String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

fn resolve_loaded_config(parsed: &argv::CliArgs) -> Result<Option<LoadedConfig>> {
    if let Some(config_path) = &parsed.config_path {
        return Ok(Some(load_config_file(config_path)?));
    }

    let discovery_input = parsed
        .target_path
        .clone()
        .unwrap_or(std::env::current_dir()?);
    load_discovered_config(discovery_input)
}

fn emit_config_warnings(command: &Command, config: Option<&LoadedConfig>, json_mode: bool) {
    if json_mode || matches!(command, Command::Ci) {
        return;
    }

    let Some(config) = config else {
        return;
    };

    for warning in &config.warnings {
        eprintln!(
            "legolas: config warning: {}: {}",
            config.path.display(),
            warning
        );
    }
}

fn resolve_target_path(parsed: &argv::CliArgs, config: Option<&LoadedConfig>) -> Result<PathBuf> {
    if let Some(target_path) = &parsed.target_path {
        return Ok(target_path.clone());
    }

    if let Some(default_path) = config
        .and_then(|item| item.config.command_defaults.scan_path.as_deref())
        .map(|value| resolve_config_relative_path(config.expect("config exists"), value))
    {
        return Ok(default_path);
    }

    std::env::current_dir().map_err(Into::into)
}

fn resolve_visualize_limit(parsed: &argv::CliArgs, config: Option<&LoadedConfig>) -> usize {
    parsed
        .limit
        .or_else(|| config.and_then(|item| item.config.command_defaults.visualize_limit))
        .unwrap_or(10)
}

fn resolve_optimize_top(parsed: &argv::CliArgs, config: Option<&LoadedConfig>) -> usize {
    parsed
        .top
        .or_else(|| config.and_then(|item| item.config.command_defaults.optimize_top))
        .unwrap_or(5)
}

fn resolve_budget_evaluation(
    command: &Command,
    analysis: &legolas_core::Analysis,
    config: Option<&LoadedConfig>,
) -> Option<BudgetEvaluation> {
    matches!(command, Command::Budget | Command::Ci).then(|| {
        evaluate_budget(
            analysis,
            config.and_then(|item| item.config.budget_rules.as_ref()),
        )
    })
}

fn regression_only_analysis(
    analysis: &legolas_core::Analysis,
    baseline: &BaselineSnapshot,
) -> legolas_core::Analysis {
    let diff = diff_analysis(baseline, analysis);
    let potential_kb_saved_worsened =
        diff.potential_kb_saved_current > diff.potential_kb_saved_previous;
    let dynamic_import_count_decreased =
        diff.dynamic_import_count_current < diff.dynamic_import_count_previous;
    let added_heavy_dependencies = diff
        .added_heavy_dependency_names
        .into_iter()
        .chain(diff.worsened_heavy_dependency_names)
        .collect::<BTreeSet<_>>();
    let added_tree_shaking_warning_keys = diff
        .added_tree_shaking_warning_keys
        .into_iter()
        .chain(diff.worsened_tree_shaking_warning_keys)
        .collect::<BTreeSet<_>>();
    let added_duplicate_package_keys = diff
        .added_duplicate_package_keys
        .into_iter()
        .chain(diff.worsened_duplicate_package_keys)
        .collect::<BTreeSet<_>>();
    let added_lazy_load_candidate_keys = diff
        .added_lazy_load_candidate_keys
        .into_iter()
        .chain(diff.worsened_lazy_load_candidate_keys)
        .collect::<BTreeSet<_>>();
    let added_boundary_warning_keys = diff
        .added_boundary_warning_keys
        .into_iter()
        .collect::<BTreeSet<_>>();
    let added_unused_dependency_candidate_names = diff
        .added_unused_dependency_candidate_names
        .into_iter()
        .collect::<BTreeSet<_>>();
    let added_warnings = diff.added_warnings.into_iter().collect::<BTreeSet<_>>();
    let mut filtered = analysis.clone();

    filtered
        .heavy_dependencies
        .retain(|item| added_heavy_dependencies.contains(item.name.as_str()));
    filtered
        .tree_shaking_warnings
        .retain(|item| added_tree_shaking_warning_keys.contains(item.key.as_str()));
    filtered.duplicate_packages.retain(|item| {
        added_duplicate_package_keys.contains(regression_finding_key(
            item.finding.finding_id.as_deref(),
            item.name.as_str(),
        ))
    });
    filtered.lazy_load_candidates.retain(|item| {
        added_lazy_load_candidate_keys.contains(regression_finding_key(
            item.finding.finding_id.as_deref(),
            item.name.as_str(),
        ))
    });
    filtered
        .boundary_warnings
        .retain(|item| added_boundary_warning_keys.contains(boundary_warning_key(item).as_str()));
    filtered
        .warnings
        .retain(|warning| added_warnings.contains(warning.as_str()));
    filtered
        .unused_dependency_candidates
        .retain(|item| added_unused_dependency_candidate_names.contains(item.name.as_str()));

    if potential_kb_saved_worsened
        && filtered.heavy_dependencies.is_empty()
        && filtered.duplicate_packages.is_empty()
        && filtered.lazy_load_candidates.is_empty()
        && filtered.tree_shaking_warnings.is_empty()
    {
        filtered.heavy_dependencies = analysis.heavy_dependencies.clone();
        filtered.duplicate_packages = analysis.duplicate_packages.clone();
        filtered.lazy_load_candidates = analysis.lazy_load_candidates.clone();
        filtered.tree_shaking_warnings = analysis.tree_shaking_warnings.clone();
    }

    if dynamic_import_count_decreased && filtered.lazy_load_candidates.is_empty() {
        filtered.lazy_load_candidates = analysis.lazy_load_candidates.clone();
    }

    filtered.impact = estimate_impact(
        &filtered.heavy_dependencies,
        &filtered.duplicate_packages,
        &filtered.lazy_load_candidates,
        &filtered.tree_shaking_warnings,
    );

    filtered
}

fn regression_finding_key<'a>(finding_id: Option<&'a str>, fallback: &'a str) -> &'a str {
    finding_id.unwrap_or(fallback)
}

fn filter_regression_budget_evaluation(
    evaluation: BudgetEvaluation,
    diff: &legolas_core::BaselineDiff,
) -> BudgetEvaluation {
    let potential_kb_saved_worsened =
        diff.potential_kb_saved_current > diff.potential_kb_saved_previous;
    let duplicate_package_regressed = !diff.added_duplicate_package_keys.is_empty()
        || !diff.worsened_duplicate_package_keys.is_empty();
    let lazy_load_regressed = !diff.added_lazy_load_candidate_keys.is_empty()
        || !diff.worsened_lazy_load_candidate_keys.is_empty();
    let dynamic_import_count_decreased =
        diff.dynamic_import_count_current < diff.dynamic_import_count_previous;
    let rules = evaluation
        .rules
        .into_iter()
        .filter(|item| {
            if item.status == legolas_core::budget::BudgetStatus::Pass {
                return false;
            }

            match item.key.as_str() {
                "potentialKbSaved" => {
                    potential_kb_saved_worsened && !item.triggered_findings.is_empty()
                }
                "duplicatePackageCount" => {
                    duplicate_package_regressed && !item.triggered_findings.is_empty()
                }
                "dynamicImportCount" => {
                    dynamic_import_count_decreased
                        || (lazy_load_regressed && !item.triggered_findings.is_empty())
                }
                _ => !item.triggered_findings.is_empty(),
            }
        })
        .collect::<Vec<_>>();
    let overall_status = rules
        .iter()
        .map(|item| item.status)
        .max()
        .unwrap_or_default();

    BudgetEvaluation {
        overall_status,
        rules,
    }
}

fn resolve_config_relative_path(config: &LoadedConfig, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        return path;
    }

    config
        .path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(path)
}

fn ci_failure_message(evaluation: &BudgetEvaluation) -> String {
    let failing_rules = evaluation
        .rules
        .iter()
        .filter(|item| item.status == legolas_core::budget::BudgetStatus::Fail)
        .map(|item| item.key.as_str())
        .collect::<Vec<_>>();

    if failing_rules.is_empty() {
        return format!(
            "CI gate failed: overall status {:?}",
            evaluation.overall_status
        );
    }

    format!(
        "CI gate failed: overall status {:?} (failing rules: {})",
        evaluation.overall_status,
        failing_rules.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::{filter_regression_budget_evaluation, regression_only_analysis};
    use legolas_core::{
        budget::{evaluate_budget, BudgetStatus},
        config::{BudgetRules, BudgetThresholds},
        Analysis, BaselineSnapshot, DuplicatePackage, FindingAnalysisSource, FindingConfidence,
        FindingMetadata, Impact, LazyLoadCandidate, SourceSummary,
    };

    #[test]
    fn regression_only_budget_keeps_new_duplicate_package_failures() {
        let analysis = Analysis {
            source_summary: SourceSummary {
                dynamic_imports: 2,
                ..SourceSummary::default()
            },
            duplicate_packages: vec![DuplicatePackage {
                name: "react".to_string(),
                versions: vec!["17.0.0".to_string(), "18.0.0".to_string()],
                count: 2,
                estimated_extra_kb: 12,
                finding: FindingMetadata::new(
                    "duplicate-package:react",
                    FindingAnalysisSource::LockfileTrace,
                )
                .with_confidence(FindingConfidence::High),
                ..DuplicatePackage::default()
            }],
            impact: Impact::default(),
            ..Analysis::default()
        };
        let filtered = regression_only_analysis(
            &analysis,
            &BaselineSnapshot {
                duplicate_package_keys: Vec::new(),
                lazy_load_candidate_keys: Vec::new(),
                ..BaselineSnapshot::default()
            },
        );
        let diff = legolas_core::diff_analysis(
            &BaselineSnapshot {
                duplicate_package_keys: Vec::new(),
                lazy_load_candidate_keys: Vec::new(),
                ..BaselineSnapshot::default()
            },
            &analysis,
        );

        assert_eq!(filtered.duplicate_packages.len(), 1);

        let evaluation = filter_regression_budget_evaluation(
            evaluate_budget(
                &filtered,
                Some(&BudgetRules {
                    potential_kb_saved: Some(BudgetThresholds {
                        warn_at: usize::MAX,
                        fail_at: usize::MAX,
                    }),
                    duplicate_package_count: Some(BudgetThresholds {
                        warn_at: 1,
                        fail_at: 1,
                    }),
                    dynamic_import_count: Some(BudgetThresholds {
                        warn_at: 1,
                        fail_at: 0,
                    }),
                }),
            ),
            &diff,
        );

        assert_eq!(evaluation.overall_status, BudgetStatus::Fail);
        assert_eq!(evaluation.rules.len(), 1);
        assert_eq!(evaluation.rules[0].key, "duplicatePackageCount");
        assert_eq!(evaluation.rules[0].triggered_findings.len(), 1);
    }

    #[test]
    fn regression_only_budget_keeps_new_lazy_load_failures() {
        let analysis = Analysis {
            source_summary: SourceSummary::default(),
            lazy_load_candidates: vec![LazyLoadCandidate {
                name: "chart.js".to_string(),
                estimated_savings_kb: 48,
                recommendation: "Lazy-load chart routes.".to_string(),
                files: vec!["src/routes/dashboard.tsx".to_string()],
                reason: "Route-only import".to_string(),
                finding: FindingMetadata::new(
                    "lazy-load:chart.js",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::Medium),
            }],
            impact: Impact::default(),
            ..Analysis::default()
        };
        let filtered = regression_only_analysis(
            &analysis,
            &BaselineSnapshot {
                duplicate_package_keys: Vec::new(),
                lazy_load_candidate_keys: Vec::new(),
                ..BaselineSnapshot::default()
            },
        );
        let diff = legolas_core::diff_analysis(
            &BaselineSnapshot {
                duplicate_package_keys: Vec::new(),
                lazy_load_candidate_keys: Vec::new(),
                ..BaselineSnapshot::default()
            },
            &analysis,
        );

        assert_eq!(filtered.lazy_load_candidates.len(), 1);

        let evaluation = filter_regression_budget_evaluation(
            evaluate_budget(
                &filtered,
                Some(&BudgetRules {
                    potential_kb_saved: Some(BudgetThresholds {
                        warn_at: usize::MAX,
                        fail_at: usize::MAX,
                    }),
                    duplicate_package_count: Some(BudgetThresholds {
                        warn_at: usize::MAX,
                        fail_at: usize::MAX,
                    }),
                    dynamic_import_count: Some(BudgetThresholds {
                        warn_at: 1,
                        fail_at: 0,
                    }),
                }),
            ),
            &diff,
        );

        assert_eq!(evaluation.overall_status, BudgetStatus::Fail);
        assert_eq!(evaluation.rules.len(), 1);
        assert_eq!(evaluation.rules[0].key, "dynamicImportCount");
        assert_eq!(evaluation.rules[0].triggered_findings.len(), 1);
    }

    #[test]
    fn regression_only_keeps_worsened_existing_findings() {
        let analysis = Analysis {
            heavy_dependencies: vec![legolas_core::HeavyDependency {
                name: "lodash".to_string(),
                version_range: "^4.17.21".to_string(),
                estimated_kb: 72,
                category: "utility".to_string(),
                rationale: "same package".to_string(),
                recommendation: "narrow import".to_string(),
                imported_by: vec!["src/a.ts".to_string(), "src/b.ts".to_string()],
                dynamic_imported_by: Vec::new(),
                import_count: 2,
                finding: FindingMetadata::new(
                    "heavy-dependency:lodash",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::High),
            }],
            duplicate_packages: vec![DuplicatePackage {
                name: "react".to_string(),
                versions: vec!["17.0.0".to_string(), "18.0.0".to_string()],
                count: 2,
                estimated_extra_kb: 20,
                finding: FindingMetadata::new(
                    "duplicate-package:react",
                    FindingAnalysisSource::LockfileTrace,
                )
                .with_confidence(FindingConfidence::High),
                ..DuplicatePackage::default()
            }],
            lazy_load_candidates: vec![LazyLoadCandidate {
                name: "chart.js".to_string(),
                estimated_savings_kb: 64,
                recommendation: "Lazy-load chart routes.".to_string(),
                files: vec![
                    "src/routes/dashboard.tsx".to_string(),
                    "src/routes/report.tsx".to_string(),
                ],
                reason: "Route-only import".to_string(),
                finding: FindingMetadata::new(
                    "lazy-load:chart.js",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::Medium),
            }],
            impact: Impact {
                potential_kb_saved: 156,
                ..Impact::default()
            },
            ..Analysis::default()
        };
        let filtered = regression_only_analysis(
            &analysis,
            &BaselineSnapshot {
                heavy_dependency_names: vec!["lodash".to_string()],
                duplicate_package_keys: vec!["duplicate-package:react".to_string()],
                lazy_load_candidate_keys: vec!["lazy-load:chart.js".to_string()],
                heavy_dependency_metrics: vec![legolas_core::baseline::BaselineFindingMetric {
                    key: "lodash".to_string(),
                    primary_metric: 72,
                    secondary_metric: Some(1),
                }],
                duplicate_package_metrics: vec![legolas_core::baseline::BaselineFindingMetric {
                    key: "duplicate-package:react".to_string(),
                    primary_metric: 12,
                    secondary_metric: Some(1),
                }],
                lazy_load_candidate_metrics: vec![legolas_core::baseline::BaselineFindingMetric {
                    key: "lazy-load:chart.js".to_string(),
                    primary_metric: 48,
                    secondary_metric: Some(1),
                }],
                potential_kb_saved: 132,
                ..BaselineSnapshot::default()
            },
        );

        assert_eq!(filtered.heavy_dependencies.len(), 1);
        assert_eq!(filtered.duplicate_packages.len(), 1);
        assert_eq!(filtered.lazy_load_candidates.len(), 1);
    }

    #[test]
    fn regression_only_keeps_aggregate_regressions_when_finding_keys_are_unchanged() {
        let analysis = Analysis {
            source_summary: SourceSummary {
                dynamic_imports: 1,
                ..SourceSummary::default()
            },
            heavy_dependencies: vec![legolas_core::HeavyDependency {
                name: "chart.js".to_string(),
                version_range: "^4.4.1".to_string(),
                estimated_kb: 160,
                category: "charts".to_string(),
                rationale: "same package".to_string(),
                recommendation: "lazy-load".to_string(),
                imported_by: vec!["src/App.tsx".to_string()],
                dynamic_imported_by: Vec::new(),
                import_count: 1,
                finding: FindingMetadata::new(
                    "heavy-dependency:chart.js",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::High),
            }],
            lazy_load_candidates: vec![LazyLoadCandidate {
                name: "chart.js".to_string(),
                estimated_savings_kb: 48,
                recommendation: "Lazy-load chart routes.".to_string(),
                files: vec!["src/routes/dashboard.tsx".to_string()],
                reason: "Route-only import".to_string(),
                finding: FindingMetadata::new(
                    "lazy-load:chart.js",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::Medium),
            }],
            impact: Impact {
                potential_kb_saved: 72,
                ..Impact::default()
            },
            unused_dependency_candidates: vec![legolas_core::UnusedDependencyCandidate {
                name: "unused".to_string(),
                version_range: "^1.0.0".to_string(),
            }],
            ..Analysis::default()
        };
        let filtered = regression_only_analysis(
            &analysis,
            &BaselineSnapshot {
                dynamic_import_count: 2,
                potential_kb_saved: 48,
                heavy_dependency_names: vec!["chart.js".to_string()],
                lazy_load_candidate_keys: vec!["lazy-load:chart.js".to_string()],
                heavy_dependency_metrics: vec![legolas_core::baseline::BaselineFindingMetric {
                    key: "chart.js".to_string(),
                    primary_metric: 160,
                    secondary_metric: Some(1),
                }],
                lazy_load_candidate_metrics: vec![legolas_core::baseline::BaselineFindingMetric {
                    key: "lazy-load:chart.js".to_string(),
                    primary_metric: 48,
                    secondary_metric: Some(1),
                }],
                ..BaselineSnapshot::default()
            },
        );

        assert_eq!(filtered.heavy_dependencies.len(), 1);
        assert_eq!(filtered.lazy_load_candidates.len(), 1);
        assert_eq!(filtered.unused_dependency_candidates.len(), 1);
        assert_eq!(filtered.unused_dependency_candidates[0].name, "unused");
    }

    #[test]
    fn regression_only_budget_drops_potential_kb_saved_failures_when_only_dynamic_import_count_regresses(
    ) {
        let analysis = Analysis {
            source_summary: SourceSummary {
                dynamic_imports: 1,
                ..SourceSummary::default()
            },
            heavy_dependencies: vec![legolas_core::HeavyDependency {
                name: "chart.js".to_string(),
                version_range: "^4.4.1".to_string(),
                estimated_kb: 160,
                category: "charts".to_string(),
                rationale: "route-only import".to_string(),
                recommendation: "lazy-load".to_string(),
                imported_by: vec!["src/routes/Dashboard.tsx".to_string()],
                dynamic_imported_by: Vec::new(),
                import_count: 1,
                finding: FindingMetadata::new(
                    "heavy-dependency:chart.js",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::High),
            }],
            lazy_load_candidates: vec![LazyLoadCandidate {
                name: "chart.js".to_string(),
                estimated_savings_kb: 128,
                recommendation: "Lazy-load dashboard routes.".to_string(),
                files: vec!["src/routes/Dashboard.tsx".to_string()],
                reason: "Route-only import".to_string(),
                finding: FindingMetadata::new(
                    "lazy-load:chart.js",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::Medium),
            }],
            impact: Impact {
                potential_kb_saved: 157,
                ..Impact::default()
            },
            ..Analysis::default()
        };
        let baseline = BaselineSnapshot {
            dynamic_import_count: 2,
            potential_kb_saved: 157,
            heavy_dependency_names: vec!["chart.js".to_string()],
            lazy_load_candidate_keys: vec!["lazy-load:chart.js".to_string()],
            heavy_dependency_metrics: vec![legolas_core::baseline::BaselineFindingMetric {
                key: "chart.js".to_string(),
                primary_metric: 160,
                secondary_metric: Some(1),
            }],
            lazy_load_candidate_metrics: vec![legolas_core::baseline::BaselineFindingMetric {
                key: "lazy-load:chart.js".to_string(),
                primary_metric: 128,
                secondary_metric: Some(1),
            }],
            ..BaselineSnapshot::default()
        };
        let filtered = regression_only_analysis(&analysis, &baseline);
        let diff = legolas_core::diff_analysis(&baseline, &analysis);

        assert_eq!(filtered.lazy_load_candidates.len(), 1);
        assert_eq!(filtered.impact.potential_kb_saved, 128);

        let evaluation = filter_regression_budget_evaluation(
            evaluate_budget(
                &filtered,
                Some(&BudgetRules {
                    potential_kb_saved: Some(BudgetThresholds {
                        warn_at: 40,
                        fail_at: 80,
                    }),
                    duplicate_package_count: Some(BudgetThresholds {
                        warn_at: usize::MAX,
                        fail_at: usize::MAX,
                    }),
                    dynamic_import_count: Some(BudgetThresholds {
                        warn_at: 1,
                        fail_at: 0,
                    }),
                }),
            ),
            &diff,
        );

        assert_eq!(evaluation.overall_status, BudgetStatus::Warn);
        assert_eq!(evaluation.rules.len(), 1);
        assert_eq!(evaluation.rules[0].key, "dynamicImportCount");
        assert_eq!(evaluation.rules[0].status, BudgetStatus::Warn);
    }

    #[test]
    fn regression_only_budget_keeps_dynamic_import_failures_without_lazy_load_findings() {
        let analysis = Analysis {
            source_summary: SourceSummary {
                dynamic_imports: 0,
                ..SourceSummary::default()
            },
            impact: Impact::default(),
            ..Analysis::default()
        };
        let baseline = BaselineSnapshot {
            dynamic_import_count: 1,
            ..BaselineSnapshot::default()
        };
        let filtered = regression_only_analysis(&analysis, &baseline);
        let diff = legolas_core::diff_analysis(&baseline, &analysis);

        let evaluation = filter_regression_budget_evaluation(
            evaluate_budget(
                &filtered,
                Some(&BudgetRules {
                    potential_kb_saved: Some(BudgetThresholds {
                        warn_at: usize::MAX,
                        fail_at: usize::MAX,
                    }),
                    duplicate_package_count: Some(BudgetThresholds {
                        warn_at: usize::MAX,
                        fail_at: usize::MAX,
                    }),
                    dynamic_import_count: Some(BudgetThresholds {
                        warn_at: 1,
                        fail_at: 0,
                    }),
                }),
            ),
            &diff,
        );

        assert_eq!(evaluation.overall_status, BudgetStatus::Fail);
        assert_eq!(evaluation.rules.len(), 1);
        assert_eq!(evaluation.rules[0].key, "dynamicImportCount");
        assert_eq!(evaluation.rules[0].status, BudgetStatus::Fail);
        assert!(evaluation.rules[0].triggered_findings.is_empty());
    }
}
