use std::{fs, path::PathBuf};

use legolas_cli::{
    argv::{self, Command},
    reporters::text::{
        format_budget_report, format_ci_report, format_optimize_report, format_scan_report,
        format_visualization_report,
    },
};
use legolas_core::{
    analyze_project,
    budget::{evaluate_budget, BudgetEvaluation},
    config::{load_config_file, load_discovered_config, LoadedConfig},
    LegolasError, Result,
};
use serde_json::json;

const HELP_TEXT: &str = r#"Legolas
Slim bundles with precision.

Usage:
  legolas scan [path] [--config file] [--json]
  legolas visualize [path] [--config file] [--limit 10]
  legolas optimize [path] [--config file] [--top 5]
  legolas budget [path] [--config file] [--json]
  legolas ci [path] [--config file] [--json]
  legolas help

Examples:
  legolas scan .
  legolas scan --config ./legolas.config.json
  legolas visualize ./apps/storefront --limit 12
  legolas optimize --top 7
  legolas budget ./apps/storefront --json
  legolas ci ./apps/storefront
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
    emit_config_warnings(&command, loaded_config.as_ref(), parsed.json);
    let target_path = resolve_target_path(&parsed, loaded_config.as_ref())?;
    let analysis = analyze_project(&target_path)?;
    let budget_evaluation = resolve_budget_evaluation(&command, &analysis, loaded_config.as_ref());

    if parsed.json {
        match command {
            Command::Budget => println!(
                "{}",
                serde_json::to_string_pretty(
                    budget_evaluation
                        .as_ref()
                        .expect("budget evaluation exists for budget command"),
                )?
            ),
            Command::Ci => println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "passed": !budget_evaluation
                        .as_ref()
                        .expect("budget evaluation exists for ci command")
                        .has_failures(),
                    "overallStatus": budget_evaluation
                        .as_ref()
                        .expect("budget evaluation exists for ci command")
                        .overall_status,
                    "rules": budget_evaluation
                        .as_ref()
                        .expect("budget evaluation exists for ci command")
                        .rules,
                }))?
            ),
            _ => println!("{}", serde_json::to_string_pretty(&analysis)?),
        }

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
        Command::Scan => format_scan_report(&analysis),
        Command::Visualize => format_visualization_report(
            &analysis,
            resolve_visualize_limit(&parsed, loaded_config.as_ref()),
        ),
        Command::Optimize => format_optimize_report(
            &analysis,
            resolve_optimize_top(&parsed, loaded_config.as_ref()),
        ),
        Command::Budget => format_budget_report(
            &analysis,
            budget_evaluation
                .as_ref()
                .expect("budget evaluation exists for budget command"),
        ),
        Command::Ci => format_ci_report(
            &analysis,
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

fn read_package_version() -> Result<String> {
    let package_json = fs::read_to_string(workspace_root().join("package.json"))?;
    let value = serde_json::from_str::<serde_json::Value>(&package_json)?;

    Ok(value
        .get("version")
        .and_then(|version| version.as_str())
        .unwrap_or("0.0.0")
        .to_string())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
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
