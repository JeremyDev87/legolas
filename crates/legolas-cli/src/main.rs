use std::{fs, path::PathBuf};

use legolas_cli::{
    argv::{self, Command},
    reporters::text::{format_optimize_report, format_scan_report, format_visualization_report},
};
use legolas_core::{analyze_project, LegolasError, Result};

const HELP_TEXT: &str = r#"Legolas
Slim bundles with precision.

Usage:
  legolas scan [path] [--json]
  legolas visualize [path] [--limit 10]
  legolas optimize [path] [--top 5]
  legolas help

Examples:
  legolas scan .
  legolas visualize ./apps/storefront --limit 12
  legolas optimize --top 7
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
        println!("{HELP_TEXT}");
        return Ok(());
    }

    let command = parsed.command.expect("command already checked");
    if let Command::Unknown(command) = command {
        return Err(LegolasError::CliUsage(format!(
            "unknown command \"{command}\""
        )));
    }

    let analysis = analyze_project(&parsed.target_path)?;

    if parsed.json {
        println!("{}", serde_json::to_string_pretty(&analysis)?);
        return Ok(());
    }

    let output = match command {
        Command::Scan => format_scan_report(&analysis),
        Command::Visualize => format_visualization_report(&analysis, parsed.limit.unwrap_or(10)),
        Command::Optimize => format_optimize_report(&analysis, parsed.top.unwrap_or(5)),
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
