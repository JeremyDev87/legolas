use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::workspace::{find_project_root, normalize_path, read_text_if_exists};
use crate::{error::Result, LegolasError};

const ALIAS_CONFIG_FILES: [&str; 2] = ["tsconfig.json", "jsconfig.json"];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AliasConfig {
    pub base_url: Option<PathBuf>,
    pub rules: Vec<AliasRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasRule {
    pub pattern: String,
    pub specifier_prefix: String,
    pub replacement_targets: Vec<AliasTarget>,
    pub wildcard: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasTarget {
    pub pattern: String,
    pub replacement_prefix: String,
    pub path_candidate: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedAliasConfig {
    pub path: PathBuf,
    pub config: AliasConfig,
}

pub fn load_alias_config<P: AsRef<Path>>(input_path: P) -> Result<Option<LoadedAliasConfig>> {
    let project_root = resolve_project_root(input_path.as_ref())?;

    for file_name in ALIAS_CONFIG_FILES {
        let config_path = project_root.join(file_name);
        let Some(raw_contents) = read_text_if_exists(&config_path)? else {
            continue;
        };

        let config = parse_alias_config(&config_path, &raw_contents)?;
        return Ok(Some(LoadedAliasConfig {
            path: config_path,
            config,
        }));
    }

    Ok(None)
}

fn resolve_project_root(input_path: &Path) -> Result<PathBuf> {
    let project_root = find_project_root(input_path)?;
    Ok(normalize_path(&project_root))
}

fn parse_alias_config(config_path: &Path, raw_contents: &str) -> Result<AliasConfig> {
    let normalized_contents = strip_jsonc(config_path, raw_contents)?;
    let json_value = serde_json::from_str::<Value>(&normalized_contents).map_err(|error| {
        LegolasError::MalformedConfig {
            path: config_path.display().to_string(),
            message: error.to_string(),
        }
    })?;
    let root = expect_object(&json_value, config_path, "$")?;
    let compiler_options = root
        .get("compilerOptions")
        .map(|value| expect_object(value, config_path, "compilerOptions"))
        .transpose()?;
    let config_dir = config_path.parent().unwrap_or(Path::new("."));
    let base_url = compiler_options
        .and_then(|options| options.get("baseUrl"))
        .map(|value| parse_base_url(value, config_path, config_dir))
        .transpose()?;
    let rules = compiler_options
        .and_then(|options| options.get("paths"))
        .map(|value| parse_paths(value, config_path, config_dir, base_url.as_deref()))
        .transpose()?
        .unwrap_or_default();

    Ok(AliasConfig { base_url, rules })
}

fn strip_jsonc(config_path: &Path, raw_contents: &str) -> Result<String> {
    let without_comments = strip_json_comments(config_path, raw_contents)?;
    Ok(strip_trailing_commas(&without_comments))
}

fn strip_json_comments(config_path: &Path, raw_contents: &str) -> Result<String> {
    let chars: Vec<char> = raw_contents.chars().collect();
    let mut cleaned = String::with_capacity(raw_contents.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;

    while index < chars.len() {
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        if line_comment {
            if matches!(current, '\n' | '\r') {
                line_comment = false;
                cleaned.push(current);
            }
            index += 1;
            continue;
        }

        if block_comment {
            if current == '*' && next == Some('/') {
                block_comment = false;
                index += 2;
                continue;
            }

            if matches!(current, '\n' | '\r') {
                cleaned.push(current);
            }
            index += 1;
            continue;
        }

        if in_string {
            cleaned.push(current);
            if escaped {
                escaped = false;
            } else if current == '\\' {
                escaped = true;
            } else if current == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if current == '"' {
            in_string = true;
            cleaned.push(current);
            index += 1;
            continue;
        }

        if current == '/' && next == Some('/') {
            line_comment = true;
            index += 2;
            continue;
        }

        if current == '/' && next == Some('*') {
            block_comment = true;
            index += 2;
            continue;
        }

        cleaned.push(current);
        index += 1;
    }

    if block_comment {
        return Err(LegolasError::MalformedConfig {
            path: config_path.display().to_string(),
            message: "unterminated block comment".to_string(),
        });
    }

    Ok(cleaned)
}

fn strip_trailing_commas(raw_contents: &str) -> String {
    let chars: Vec<char> = raw_contents.chars().collect();
    let mut cleaned = String::with_capacity(raw_contents.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < chars.len() {
        let current = chars[index];

        if in_string {
            cleaned.push(current);
            if escaped {
                escaped = false;
            } else if current == '\\' {
                escaped = true;
            } else if current == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if current == '"' {
            in_string = true;
            cleaned.push(current);
            index += 1;
            continue;
        }

        if current == ',' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }

            if matches!(chars.get(lookahead), Some('}') | Some(']')) {
                index += 1;
                continue;
            }
        }

        cleaned.push(current);
        index += 1;
    }

    cleaned
}

fn parse_base_url(value: &Value, config_path: &Path, config_dir: &Path) -> Result<PathBuf> {
    let base_url = expect_string(value, config_path, "compilerOptions.baseUrl")?;

    Ok(resolve_config_path(config_dir, base_url))
}

fn parse_paths(
    value: &Value,
    config_path: &Path,
    config_dir: &Path,
    base_url: Option<&Path>,
) -> Result<Vec<AliasRule>> {
    let paths = expect_object(value, config_path, "compilerOptions.paths")?;
    let target_root = base_url.unwrap_or(config_dir);
    let mut rules = Vec::new();

    for (pattern, targets) in paths {
        let key_path = format!("compilerOptions.paths[{pattern}]");
        let alias_pattern = parse_rule_pattern(pattern, config_path, &key_path)?;
        let target_patterns = parse_targets(targets, config_path, &key_path)?;
        let replacement_targets = target_patterns
            .into_iter()
            .map(|target_pattern| {
                if alias_pattern.wildcard != target_pattern.wildcard {
                    return unsupported_shape(
                        config_path,
                        &key_path,
                        "expected alias key and target entries to use the same wildcard shape",
                    );
                }

                Ok(AliasTarget {
                    pattern: target_pattern.original.clone(),
                    replacement_prefix: target_pattern.prefix.clone(),
                    path_candidate: resolve_config_path(target_root, &target_pattern.prefix),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        rules.push(AliasRule {
            pattern: alias_pattern.original,
            specifier_prefix: alias_pattern.prefix,
            replacement_targets,
            wildcard: alias_pattern.wildcard,
        });
    }

    // Keep more-specific rules first so later matchers can use this order directly.
    rules.sort_by(|left, right| {
        right
            .pattern
            .chars()
            .filter(|character| *character != '*')
            .count()
            .cmp(
                &left
                    .pattern
                    .chars()
                    .filter(|character| *character != '*')
                    .count(),
            )
            .then_with(|| {
                right
                    .specifier_prefix
                    .len()
                    .cmp(&left.specifier_prefix.len())
            })
            .then_with(|| left.wildcard.cmp(&right.wildcard))
            .then_with(|| left.pattern.cmp(&right.pattern))
    });

    Ok(rules)
}

fn parse_targets(value: &Value, config_path: &Path, key_path: &str) -> Result<Vec<ParsedPattern>> {
    let targets = expect_array(value, config_path, key_path)?;
    if targets.is_empty() {
        return unsupported_shape(
            config_path,
            key_path,
            "expected at least one target entry in alias path array",
        );
    }

    targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let target_key_path = format!("{key_path}[{index}]");
            let target = expect_string(target, config_path, &target_key_path)?;

            parse_rule_pattern(target, config_path, &target_key_path)
        })
        .collect()
}

fn parse_rule_pattern(value: &str, config_path: &Path, key_path: &str) -> Result<ParsedPattern> {
    if value.is_empty() {
        return unsupported_shape(config_path, key_path, "expected non-empty alias pattern");
    }

    let wildcard_count = value.matches('*').count();
    if wildcard_count == 0 {
        return Ok(ParsedPattern {
            original: value.to_string(),
            prefix: value.to_string(),
            wildcard: false,
        });
    }

    if wildcard_count != 1 {
        return unsupported_shape(
            config_path,
            key_path,
            "expected exact value or a single * wildcard pattern",
        );
    }

    let (prefix, _) = value
        .split_once('*')
        .expect("single wildcard patterns always split");

    Ok(ParsedPattern {
        original: value.to_string(),
        prefix: prefix.to_string(),
        wildcard: true,
    })
}

fn resolve_config_path(root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);

    if path.is_absolute() {
        normalize_path(path)
    } else {
        normalize_path(&root.join(path))
    }
}

fn expect_object<'a>(
    value: &'a Value,
    config_path: &Path,
    key_path: &str,
) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| LegolasError::UnsupportedConfigShape {
            path: config_path.display().to_string(),
            key_path: key_path.to_string(),
            message: "expected object".to_string(),
        })
}

fn expect_array<'a>(
    value: &'a Value,
    config_path: &Path,
    key_path: &str,
) -> Result<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| LegolasError::UnsupportedConfigShape {
            path: config_path.display().to_string(),
            key_path: key_path.to_string(),
            message: "expected array".to_string(),
        })
}

fn expect_string<'a>(value: &'a Value, config_path: &Path, key_path: &str) -> Result<&'a str> {
    value
        .as_str()
        .ok_or_else(|| LegolasError::UnsupportedConfigShape {
            path: config_path.display().to_string(),
            key_path: key_path.to_string(),
            message: "expected string".to_string(),
        })
}

fn unsupported_shape<T>(config_path: &Path, key_path: &str, message: &str) -> Result<T> {
    Err(LegolasError::UnsupportedConfigShape {
        path: config_path.display().to_string(),
        key_path: key_path.to_string(),
        message: message.to_string(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedPattern {
    original: String,
    prefix: String,
    wildcard: bool,
}
