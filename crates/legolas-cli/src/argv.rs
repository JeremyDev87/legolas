use std::path::PathBuf;

use legolas_core::{LegolasError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Scan,
    Visualize,
    Optimize,
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

pub fn parse_argv<I, S>(args: I) -> Result<CliArgs>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let tokens = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let mut parsed = CliArgs::default();
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
                let next = tokens
                    .get(index + 1)
                    .ok_or_else(|| LegolasError::CliUsage(format!("{token} expects a number")))?;

                if next.starts_with('-') {
                    return Err(LegolasError::CliUsage(format!("{token} expects a number")));
                }

                let parsed_value = next.parse::<usize>().map_err(|_| {
                    LegolasError::CliUsage(format!("{token} expects a positive integer"))
                })?;

                if parsed_value < 1 {
                    return Err(LegolasError::CliUsage(format!(
                        "{token} expects a positive integer"
                    )));
                }

                match token.as_str() {
                    "--limit" => parsed.limit = Some(parsed_value),
                    "--top" => parsed.top = Some(parsed_value),
                    _ => {}
                }

                index += 1;
            }
            _ => {
                return Err(LegolasError::CliUsage(format!("unknown flag \"{token}\"")));
            }
        }

        index += 1;
    }

    Ok(parsed)
}

fn parse_command(token: &str) -> Command {
    match token {
        "scan" => Command::Scan,
        "visualize" => Command::Visualize,
        "optimize" => Command::Optimize,
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
