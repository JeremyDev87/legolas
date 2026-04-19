use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value};

use crate::{error::Result, workspace::find_discovered_config_path, LegolasError};

const UNKNOWN_KEY_WARNING: &str = "unknown config key ignored";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThresholdDirection {
    Max,
    Min,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LegolasConfig {
    pub command_defaults: CommandDefaults,
    pub budget_rules: Option<BudgetRules>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandDefaults {
    pub scan_path: Option<String>,
    pub visualize_limit: Option<usize>,
    pub optimize_top: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BudgetRules {
    pub potential_kb_saved: Option<BudgetThresholds>,
    pub duplicate_package_count: Option<BudgetThresholds>,
    pub dynamic_import_count: Option<BudgetThresholds>,
}

impl BudgetRules {
    pub fn starter_defaults() -> Self {
        Self {
            potential_kb_saved: Some(BudgetThresholds {
                warn_at: 40,
                fail_at: 80,
            }),
            duplicate_package_count: Some(BudgetThresholds {
                warn_at: 2,
                fail_at: 4,
            }),
            dynamic_import_count: Some(BudgetThresholds {
                warn_at: 1,
                fail_at: 0,
            }),
        }
    }

    pub fn merged_with_starter_defaults(&self) -> Self {
        let defaults = Self::starter_defaults();

        Self {
            potential_kb_saved: self
                .potential_kb_saved
                .clone()
                .or(defaults.potential_kb_saved),
            duplicate_package_count: self
                .duplicate_package_count
                .clone()
                .or(defaults.duplicate_package_count),
            dynamic_import_count: self
                .dynamic_import_count
                .clone()
                .or(defaults.dynamic_import_count),
        }
    }

    fn is_empty(&self) -> bool {
        self.potential_kb_saved.is_none()
            && self.duplicate_package_count.is_none()
            && self.dynamic_import_count.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetThresholds {
    pub warn_at: usize,
    pub fail_at: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWarning {
    pub key_path: String,
    pub message: String,
}

impl fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.key_path, self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub config: LegolasConfig,
    pub warnings: Vec<ConfigWarning>,
}

pub fn load_discovered_config<P: AsRef<Path>>(input_path: P) -> Result<Option<LoadedConfig>> {
    let Some(config_path) = find_discovered_config_path(input_path)? else {
        return Ok(None);
    };

    Ok(Some(load_config_file(config_path)?))
}

pub fn load_config_file<P: AsRef<Path>>(config_path: P) -> Result<LoadedConfig> {
    let config_path = config_path.as_ref();
    let raw_contents = fs::read_to_string(config_path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => {
            LegolasError::PathNotFound(config_path.display().to_string())
        }
        _ => error.into(),
    })?;
    let json_value = serde_json::from_str::<Value>(&raw_contents).map_err(|error| {
        LegolasError::MalformedConfig {
            path: config_path.display().to_string(),
            message: error.to_string(),
        }
    })?;
    let root = expect_object(&json_value, config_path, "$")?;
    let mut warnings = Vec::new();

    let config = parse_root(root, config_path, &mut warnings)?;

    Ok(LoadedConfig {
        path: config_path.to_path_buf(),
        config,
        warnings,
    })
}

fn parse_root(
    root: &Map<String, Value>,
    config_path: &Path,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<LegolasConfig> {
    warn_unknown_keys(
        root,
        &["scan", "visualize", "optimize", "budget"],
        "",
        warnings,
    );

    let mut command_defaults = CommandDefaults::default();

    if let Some(scan) = root.get("scan") {
        command_defaults.scan_path = parse_scan(scan, config_path, warnings)?;
    }

    if let Some(visualize) = root.get("visualize") {
        command_defaults.visualize_limit = parse_visualize(visualize, config_path, warnings)?;
    }

    if let Some(optimize) = root.get("optimize") {
        command_defaults.optimize_top = parse_optimize(optimize, config_path, warnings)?;
    }

    let budget_rules = match root.get("budget") {
        Some(budget) => parse_budget(budget, config_path, warnings)?,
        None => None,
    };

    Ok(LegolasConfig {
        command_defaults,
        budget_rules,
    })
}

fn parse_scan(
    value: &Value,
    config_path: &Path,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<Option<String>> {
    let scan = expect_object(value, config_path, "scan")?;
    warn_unknown_keys(scan, &["path"], "scan", warnings);

    match scan.get("path") {
        Some(path) => Ok(Some(
            expect_string(path, config_path, "scan.path")?.to_string(),
        )),
        None => Ok(None),
    }
}

fn parse_visualize(
    value: &Value,
    config_path: &Path,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<Option<usize>> {
    let visualize = expect_object(value, config_path, "visualize")?;
    warn_unknown_keys(visualize, &["limit"], "visualize", warnings);

    match visualize.get("limit") {
        Some(limit) => Ok(Some(expect_positive_usize(
            limit,
            config_path,
            "visualize.limit",
        )?)),
        None => Ok(None),
    }
}

fn parse_optimize(
    value: &Value,
    config_path: &Path,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<Option<usize>> {
    let optimize = expect_object(value, config_path, "optimize")?;
    warn_unknown_keys(optimize, &["top"], "optimize", warnings);

    match optimize.get("top") {
        Some(top) => Ok(Some(expect_positive_usize(
            top,
            config_path,
            "optimize.top",
        )?)),
        None => Ok(None),
    }
}

fn parse_budget(
    value: &Value,
    config_path: &Path,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<Option<BudgetRules>> {
    let budget = expect_object(value, config_path, "budget")?;
    warn_unknown_keys(budget, &["rules"], "budget", warnings);

    let Some(rules_value) = budget.get("rules") else {
        return Ok(None);
    };

    let rules_map = expect_object(rules_value, config_path, "budget.rules")?;
    warn_unknown_keys(
        rules_map,
        &[
            "potentialKbSaved",
            "duplicatePackageCount",
            "dynamicImportCount",
        ],
        "budget.rules",
        warnings,
    );

    let rules = BudgetRules {
        potential_kb_saved: parse_threshold_rule(
            rules_map.get("potentialKbSaved"),
            config_path,
            warnings,
            "budget.rules.potentialKbSaved",
            ThresholdDirection::Max,
        )?,
        duplicate_package_count: parse_threshold_rule(
            rules_map.get("duplicatePackageCount"),
            config_path,
            warnings,
            "budget.rules.duplicatePackageCount",
            ThresholdDirection::Max,
        )?,
        dynamic_import_count: parse_threshold_rule(
            rules_map.get("dynamicImportCount"),
            config_path,
            warnings,
            "budget.rules.dynamicImportCount",
            ThresholdDirection::Min,
        )?,
    };

    if rules.is_empty() {
        return Ok(None);
    }

    Ok(Some(rules))
}

fn parse_threshold_rule(
    value: Option<&Value>,
    config_path: &Path,
    warnings: &mut Vec<ConfigWarning>,
    key_path: &str,
    direction: ThresholdDirection,
) -> Result<Option<BudgetThresholds>> {
    let Some(value) = value else {
        return Ok(None);
    };

    let threshold = expect_object(value, config_path, key_path)?;
    warn_unknown_keys(threshold, &["warnAt", "failAt"], key_path, warnings);

    let warn_at = threshold
        .get("warnAt")
        .ok_or_else(|| unsupported_shape(config_path, &format!("{key_path}.warnAt"), "integer"))?;
    let fail_at = threshold
        .get("failAt")
        .ok_or_else(|| unsupported_shape(config_path, &format!("{key_path}.failAt"), "integer"))?;

    let warn_at = expect_usize(warn_at, config_path, &format!("{key_path}.warnAt"))?;
    let fail_at = expect_usize(fail_at, config_path, &format!("{key_path}.failAt"))?;

    validate_threshold_ordering(config_path, key_path, direction, warn_at, fail_at)?;

    Ok(Some(BudgetThresholds { warn_at, fail_at }))
}

fn expect_object<'a>(
    value: &'a Value,
    config_path: &Path,
    key_path: &str,
) -> Result<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| unsupported_shape(config_path, key_path, "object"))
}

fn expect_string<'a>(value: &'a Value, config_path: &Path, key_path: &str) -> Result<&'a str> {
    value
        .as_str()
        .ok_or_else(|| unsupported_shape(config_path, key_path, "string"))
}

fn expect_positive_usize(value: &Value, config_path: &Path, key_path: &str) -> Result<usize> {
    let number = expect_usize(value, config_path, key_path)?;
    if number == 0 {
        return Err(unsupported_shape(config_path, key_path, "positive integer"));
    }

    Ok(number)
}

fn expect_usize(value: &Value, config_path: &Path, key_path: &str) -> Result<usize> {
    let raw = value
        .as_u64()
        .ok_or_else(|| unsupported_shape(config_path, key_path, "integer"))?;

    usize::try_from(raw).map_err(|_| unsupported_shape(config_path, key_path, "integer"))
}

fn validate_threshold_ordering(
    config_path: &Path,
    key_path: &str,
    direction: ThresholdDirection,
    warn_at: usize,
    fail_at: usize,
) -> Result<()> {
    match direction {
        ThresholdDirection::Max if warn_at > fail_at => Err(unsupported_shape(
            config_path,
            key_path,
            "warnAt must be less than or equal to failAt for max rule",
        )),
        ThresholdDirection::Min if warn_at < fail_at => Err(unsupported_shape(
            config_path,
            key_path,
            "warnAt must be greater than or equal to failAt for min rule",
        )),
        _ => Ok(()),
    }
}

fn warn_unknown_keys(
    object: &Map<String, Value>,
    allowed_keys: &[&str],
    key_prefix: &str,
    warnings: &mut Vec<ConfigWarning>,
) {
    for key in object.keys() {
        if allowed_keys.contains(&key.as_str()) {
            continue;
        }

        warnings.push(ConfigWarning {
            key_path: join_key_path(key_prefix, key),
            message: UNKNOWN_KEY_WARNING.to_string(),
        });
    }
}

fn join_key_path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        return key.to_string();
    }

    format!("{prefix}.{key}")
}

fn unsupported_shape(config_path: &Path, key_path: &str, expected: &str) -> LegolasError {
    LegolasError::UnsupportedConfigShape {
        path: config_path.display().to_string(),
        key_path: key_path.to_string(),
        message: format!("expected {expected}"),
    }
}
