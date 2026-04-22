use std::path::PathBuf;

use legolas_core::{LegolasError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Scan,
    Visualize,
    Optimize,
    Budget,
    Ci,
    Help,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CliArgs {
    pub command: Option<Command>,
    pub target_path: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub baseline_path: Option<PathBuf>,
    pub write_baseline_path: Option<PathBuf>,
    pub regression_only: bool,
    pub json: bool,
    pub limit: Option<usize>,
    pub top: Option<usize>,
    pub help: bool,
    pub version: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingNumericValue {
    MissingOrFlag,
    MissingOrFlagBeforeCommand,
    Raw(String),
}

pub fn parse_argv<I, S>(args: I) -> Result<CliArgs>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let tokens = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let mut parsed = CliArgs::default();
    let mut pending_limit = None;
    let mut pending_top = None;
    let mut index = 0;

    while index < tokens.len() {
        let token = &tokens[index];

        if parsed.command.is_none() && !token.starts_with('-') {
            parsed.command = Some(parse_command(token));
            index += 1;
            continue;
        }

        if !token.starts_with('-') {
            parsed.target_path = Some(resolve_path_token(token)?);
            index += 1;
            continue;
        }

        match token.as_str() {
            "--help" | "-h" => {
                parsed.help = true;
            }
            "--version" | "-v" => {
                parsed.version = true;
            }
            "--json" => {
                parsed.json = true;
            }
            "--config" => {
                parsed.config_path = Some(parse_path_flag(&tokens, index, "--config")?);
                index += 1;
            }
            "--baseline" => {
                parsed.baseline_path = Some(parse_path_flag(&tokens, index, "--baseline")?);
                index += 1;
            }
            "--write-baseline" => {
                parsed.write_baseline_path =
                    Some(parse_path_flag(&tokens, index, "--write-baseline")?);
                index += 1;
            }
            "--regression-only" => {
                parsed.regression_only = true;
            }
            "--limit" | "--top" => {
                let command_known = parsed.command.is_some();
                let pending_value = match tokens.get(index + 1) {
                    Some(next) if !next.starts_with('-') => {
                        index += 1;
                        PendingNumericValue::Raw(next.clone())
                    }
                    Some(next) if command_known && looks_like_signed_integer_token(next) => {
                        index += 1;
                        PendingNumericValue::Raw(next.clone())
                    }
                    Some(next) if !command_known && next.starts_with('-') => {
                        index += 1;
                        PendingNumericValue::MissingOrFlagBeforeCommand
                    }
                    _ => PendingNumericValue::MissingOrFlag,
                };

                match token.as_str() {
                    "--limit" => pending_limit = Some(pending_value),
                    "--top" => pending_top = Some(pending_value),
                    _ => unreachable!("validated numeric flag"),
                }
            }
            _ => {
                return Err(LegolasError::CliUsage(format!("unknown flag \"{token}\"")));
            }
        }

        index += 1;
    }

    if parsed.help || parsed.version {
        return Ok(parsed);
    }

    validate_baseline_flags(&parsed)?;

    parsed.limit = finalize_numeric_flag(parsed.command.as_ref(), pending_limit, "--limit")?;
    parsed.top = finalize_numeric_flag(parsed.command.as_ref(), pending_top, "--top")?;

    Ok(parsed)
}

fn parse_command(token: &str) -> Command {
    match token {
        "scan" => Command::Scan,
        "visualize" => Command::Visualize,
        "optimize" => Command::Optimize,
        "budget" => Command::Budget,
        "ci" => Command::Ci,
        "help" => Command::Help,
        other => Command::Unknown(other.to_string()),
    }
}

fn resolve_path_token(token: &str) -> Result<PathBuf> {
    let path = PathBuf::from(token);
    if path.is_absolute() {
        return Ok(path);
    }

    Ok(std::env::current_dir()?.join(path))
}

fn parse_path_flag(tokens: &[String], index: usize, flag: &str) -> Result<PathBuf> {
    let next = tokens
        .get(index + 1)
        .ok_or_else(|| LegolasError::CliUsage(format!("{flag} expects a path")))?;

    if next.starts_with('-') {
        return Err(LegolasError::CliUsage(format!("{flag} expects a path")));
    }

    resolve_path_token(next)
}

fn validate_baseline_flags(parsed: &CliArgs) -> Result<()> {
    if parsed.baseline_path.is_none()
        && parsed.write_baseline_path.is_none()
        && !parsed.regression_only
    {
        return Ok(());
    }

    let command_supports = matches!(
        parsed.command,
        Some(Command::Scan) | Some(Command::Optimize) | Some(Command::Budget) | Some(Command::Ci)
    );

    if !command_supports {
        let flag = if parsed.baseline_path.is_some() {
            "--baseline"
        } else if parsed.write_baseline_path.is_some() {
            "--write-baseline"
        } else {
            "--regression-only"
        };

        return Err(LegolasError::CliUsage(format!("unknown flag \"{flag}\"")));
    }

    if parsed.write_baseline_path.is_some() && !matches!(parsed.command, Some(Command::Scan)) {
        return Err(LegolasError::CliUsage(
            "unknown flag \"--write-baseline\"".to_string(),
        ));
    }

    if parsed.baseline_path.is_some() && !parsed.regression_only {
        return Err(LegolasError::CliUsage(
            "--baseline requires --regression-only".to_string(),
        ));
    }

    Ok(())
}

fn finalize_numeric_flag(
    command: Option<&Command>,
    pending_value: Option<PendingNumericValue>,
    token: &str,
) -> Result<Option<usize>> {
    let Some(pending_value) = pending_value else {
        return Ok(None);
    };

    if matches!(command, Some(Command::Budget) | Some(Command::Ci)) {
        return Err(LegolasError::CliUsage(format!("unknown flag \"{token}\"")));
    }

    match pending_value {
        PendingNumericValue::MissingOrFlag | PendingNumericValue::MissingOrFlagBeforeCommand => {
            Err(LegolasError::CliUsage(format!("{token} expects a number")))
        }
        PendingNumericValue::Raw(raw) => {
            let parsed_value = raw.parse::<usize>().map_err(|_| {
                LegolasError::CliUsage(format!("{token} expects a positive integer"))
            })?;

            if parsed_value < 1 {
                return Err(LegolasError::CliUsage(format!(
                    "{token} expects a positive integer"
                )));
            }

            Ok(Some(parsed_value))
        }
    }
}

fn looks_like_signed_integer_token(token: &str) -> bool {
    token
        .strip_prefix('-')
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()))
}
