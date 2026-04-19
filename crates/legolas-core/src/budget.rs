use serde::{Deserialize, Serialize};

use crate::{
    config::{BudgetRules, BudgetThresholds},
    models::Analysis,
};

const POTENTIAL_KB_SAVED_KEY: &str = "potentialKbSaved";
const DUPLICATE_PACKAGE_COUNT_KEY: &str = "duplicatePackageCount";
const DYNAMIC_IMPORT_COUNT_KEY: &str = "dynamicImportCount";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum BudgetStatus {
    #[default]
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetRuleResult {
    pub key: String,
    pub actual: usize,
    pub warn_at: usize,
    pub fail_at: usize,
    pub status: BudgetStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetEvaluation {
    pub overall_status: BudgetStatus,
    pub rules: Vec<BudgetRuleResult>,
}

impl BudgetEvaluation {
    pub fn has_failures(&self) -> bool {
        self.overall_status == BudgetStatus::Fail
    }
}

pub fn evaluate_budget(analysis: &Analysis, overrides: Option<&BudgetRules>) -> BudgetEvaluation {
    let rules = resolved_rules(overrides);
    let results = vec![
        evaluate_max_rule(
            POTENTIAL_KB_SAVED_KEY,
            analysis.impact.potential_kb_saved,
            rules
                .potential_kb_saved
                .as_ref()
                .expect("starter rule exists"),
        ),
        evaluate_max_rule(
            DUPLICATE_PACKAGE_COUNT_KEY,
            analysis.duplicate_packages.len(),
            rules
                .duplicate_package_count
                .as_ref()
                .expect("starter rule exists"),
        ),
        evaluate_min_rule(
            DYNAMIC_IMPORT_COUNT_KEY,
            analysis.source_summary.dynamic_imports,
            rules
                .dynamic_import_count
                .as_ref()
                .expect("starter rule exists"),
        ),
    ];
    let overall_status = results
        .iter()
        .map(|item| item.status)
        .max()
        .unwrap_or_default();

    BudgetEvaluation {
        overall_status,
        rules: results,
    }
}

fn resolved_rules(overrides: Option<&BudgetRules>) -> BudgetRules {
    overrides
        .map(BudgetRules::merged_with_starter_defaults)
        .unwrap_or_else(BudgetRules::starter_defaults)
}

fn evaluate_max_rule(key: &str, actual: usize, thresholds: &BudgetThresholds) -> BudgetRuleResult {
    BudgetRuleResult {
        key: key.to_string(),
        actual,
        warn_at: thresholds.warn_at,
        fail_at: thresholds.fail_at,
        status: evaluate_max_status(actual, thresholds),
    }
}

fn evaluate_min_rule(key: &str, actual: usize, thresholds: &BudgetThresholds) -> BudgetRuleResult {
    BudgetRuleResult {
        key: key.to_string(),
        actual,
        warn_at: thresholds.warn_at,
        fail_at: thresholds.fail_at,
        status: evaluate_min_status(actual, thresholds),
    }
}

fn evaluate_max_status(actual: usize, thresholds: &BudgetThresholds) -> BudgetStatus {
    if actual >= thresholds.fail_at {
        return BudgetStatus::Fail;
    }

    if actual >= thresholds.warn_at {
        return BudgetStatus::Warn;
    }

    BudgetStatus::Pass
}

fn evaluate_min_status(actual: usize, thresholds: &BudgetThresholds) -> BudgetStatus {
    if actual <= thresholds.fail_at {
        return BudgetStatus::Fail;
    }

    if actual <= thresholds.warn_at {
        return BudgetStatus::Warn;
    }

    BudgetStatus::Pass
}
