use std::{fs, path::PathBuf};

use legolas_cli::{
    argv::{self, Command},
    reporters::text::{
        format_budget_report, format_optimize_report, format_scan_report,
        format_visualization_report,
    },
};
use legolas_core::{
    analyze_project,
    budget::evaluate_budget,
    config::{load_config_file, load_discovered_config, LoadedConfig},
    LegolasError, Result,
};

const HELP_TEXT: &str = r#"Legolas
Slim bundles with precision.

Usage:
  legolas scan [path] [--config file] [--json]
  legolas visualize [path] [--config file] [--limit 10]
  legolas optimize [path] [--config file] [--top 5]
  legolas budget [path] [--config file] [--json]
  legolas help

Examples:
  legolas scan .
  legolas scan --config ./legolas.config.json
  legolas visualize ./apps/storefront --limit 12
  legolas optimize --top 7
  legolas budget ./apps/storefront --json
"#;

fn main() {
    if let Err(error) = run() {
        eprintln!("legolas: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let parsed = argv::parse_argv(std::env::args().skip(1))?;

    if parsed.version {
        println!("{}", read_package_version()?);
        return Ok(());
    }

    if parsed.help || parsed.command.is_none() || matches!(parsed.command, Some(Command::Help)) {
        print!("{HELP_TEXT}");
        return Ok(());
    }

    let command = parsed.command.clone().expect("command already checked");
    if let Command::Unknown(command) = command {
        return Err(LegolasError::CliUsage(format!(
            "unknown command \"{command}\""
        )));
    }
    validate_command_flags(&command, &parsed)?;

    let loaded_config = resolve_loaded_config(&parsed)?;
    emit_config_warnings(loaded_config.as_ref(), parsed.json);
    let target_path = resolve_target_path(&parsed, loaded_config.as_ref())?;
    let analysis = analyze_project(&target_path)?;
    let budget_evaluation = matches!(command, Command::Budget).then(|| {
        evaluate_budget(
            &analysis,
            loaded_config
                .as_ref()
                .and_then(|item| item.config.budget_rules.as_ref()),
        )
    });

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
            _ => println!("{}", serde_json::to_string_pretty(&analysis)?),
        }
        return Ok(());
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
        Command::Help | Command::Unknown(_) => unreachable!("handled above"),
    };

    println!("{output}");
    Ok(())
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

fn emit_config_warnings(config: Option<&LoadedConfig>, json_mode: bool) {
    if json_mode {
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

fn validate_command_flags(command: &Command, parsed: &argv::CliArgs) -> Result<()> {
    if matches!(command, Command::Budget) {
        if parsed.limit.is_some() {
            return Err(LegolasError::CliUsage(
                "unknown flag \"--limit\"".to_string(),
            ));
        }

        if parsed.top.is_some() {
            return Err(LegolasError::CliUsage("unknown flag \"--top\"".to_string()));
        }
    }

    Ok(())
}
