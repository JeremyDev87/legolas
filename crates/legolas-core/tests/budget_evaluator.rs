use legolas_core::{
    budget::{evaluate_budget, BudgetStatus},
    config::{BudgetRules, BudgetThresholds},
    Analysis, DuplicatePackage,
};

#[test]
fn evaluate_budget_uses_starter_thresholds_when_no_override_is_present() {
    let evaluation = evaluate_budget(&analysis_with_metrics(39, 1, 2), None);

    assert_eq!(evaluation.overall_status, BudgetStatus::Pass);
    assert_eq!(
        rule_snapshot(&evaluation),
        vec![
            (
                "potentialKbSaved".to_string(),
                39,
                40,
                80,
                BudgetStatus::Pass,
            ),
            (
                "duplicatePackageCount".to_string(),
                1,
                2,
                4,
                BudgetStatus::Pass,
            ),
            (
                "dynamicImportCount".to_string(),
                2,
                1,
                0,
                BudgetStatus::Pass,
            ),
        ]
    );
    assert!(!evaluation.has_failures());
}

#[test]
fn evaluate_budget_merges_partial_overrides_with_starter_defaults() {
    let overrides = BudgetRules {
        potential_kb_saved: None,
        duplicate_package_count: Some(BudgetThresholds {
            warn_at: 1,
            fail_at: 2,
        }),
        dynamic_import_count: Some(BudgetThresholds {
            warn_at: 3,
            fail_at: 1,
        }),
    };
    let evaluation = evaluate_budget(&analysis_with_metrics(39, 2, 2), Some(&overrides));

    assert_eq!(evaluation.overall_status, BudgetStatus::Fail);
    assert_eq!(
        rule_snapshot(&evaluation),
        vec![
            (
                "potentialKbSaved".to_string(),
                39,
                40,
                80,
                BudgetStatus::Pass,
            ),
            (
                "duplicatePackageCount".to_string(),
                2,
                1,
                2,
                BudgetStatus::Fail,
            ),
            (
                "dynamicImportCount".to_string(),
                2,
                3,
                1,
                BudgetStatus::Warn,
            ),
        ]
    );
    assert!(evaluation.has_failures());
}

#[test]
fn evaluate_budget_locks_warn_and_fail_boundaries_for_each_rule() {
    let warn = evaluate_budget(&analysis_with_metrics(40, 2, 1), None);
    let fail = evaluate_budget(&analysis_with_metrics(80, 4, 0), None);

    assert_eq!(warn.overall_status, BudgetStatus::Warn);
    assert_eq!(
        rule_statuses(&warn),
        vec![BudgetStatus::Warn, BudgetStatus::Warn, BudgetStatus::Warn]
    );

    assert_eq!(fail.overall_status, BudgetStatus::Fail);
    assert_eq!(
        rule_statuses(&fail),
        vec![BudgetStatus::Fail, BudgetStatus::Fail, BudgetStatus::Fail]
    );
}

fn analysis_with_metrics(
    potential_kb_saved: usize,
    duplicate_package_count: usize,
    dynamic_import_count: usize,
) -> Analysis {
    Analysis {
        impact: legolas_core::Impact {
            potential_kb_saved,
            ..legolas_core::Impact::default()
        },
        duplicate_packages: (0..duplicate_package_count)
            .map(|index| DuplicatePackage {
                name: format!("pkg-{index}"),
                count: 2,
                ..DuplicatePackage::default()
            })
            .collect(),
        source_summary: legolas_core::SourceSummary {
            dynamic_imports: dynamic_import_count,
            ..legolas_core::SourceSummary::default()
        },
        ..Analysis::default()
    }
}

fn rule_snapshot(
    evaluation: &legolas_core::budget::BudgetEvaluation,
) -> Vec<(String, usize, usize, usize, BudgetStatus)> {
    evaluation
        .rules
        .iter()
        .map(|item| {
            (
                item.key.clone(),
                item.actual,
                item.warn_at,
                item.fail_at,
                item.status,
            )
        })
        .collect()
}

fn rule_statuses(evaluation: &legolas_core::budget::BudgetEvaluation) -> Vec<BudgetStatus> {
    evaluation.rules.iter().map(|item| item.status).collect()
}
