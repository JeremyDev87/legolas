use std::path::PathBuf;

use legolas_core::{LegolasError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Scan,
    Visualize,
    Optimize,
    Budget,
    Help,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CliArgs {
    pub command: Option<Command>,
    pub target_path: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
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
                let next = tokens
                    .get(index + 1)
                    .ok_or_else(|| LegolasError::CliUsage("--config expects a path".to_string()))?;

                if next.starts_with('-') {
                    return Err(LegolasError::CliUsage(
                        "--config expects a path".to_string(),
                    ));
                }

                parsed.config_path = Some(resolve_path_token(next)?);
                index += 1;
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

fn finalize_numeric_flag(
    command: Option<&Command>,
    pending_value: Option<PendingNumericValue>,
    token: &str,
) -> Result<Option<usize>> {
    let Some(pending_value) = pending_value else {
        return Ok(None);
    };

    if matches!(command, Some(Command::Budget)) {
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
