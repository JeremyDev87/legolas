use crate::argv::ReportLanguage;
use legolas_core::{
    boundaries::BoundaryWarning, budget::BudgetEvaluation, rank_actions,
    report_summary::build_report_summary, ActionDifficulty, Analysis, DuplicateImpactScope,
    FindingConfidence, FindingEvidence, FindingMetadata, RecommendedFix,
};
use std::collections::BTreeMap;

pub fn format_scan_report_for_language(analysis: &Analysis, language: ReportLanguage) -> String {
    match language {
        ReportLanguage::Ko => format_scan_report_ko(analysis),
        ReportLanguage::En => format_scan_report(analysis),
    }
}

pub fn format_scan_report(analysis: &Analysis) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "Legolas scan for {}",
        analysis.package_summary.name
    ));
    lines.push(format!("Project root: {}", analysis.project_root));
    lines.push(format!("Mode: {}", analysis.metadata.mode));
    lines.push(format!(
        "Frameworks: {}",
        if analysis.frameworks.is_empty() {
            "none detected".to_string()
        } else {
            analysis.frameworks.join(", ")
        }
    ));
    lines.push(format!("Package manager: {}", analysis.package_manager));
    lines.push(format!(
        "Scanned {} source files and {} imported packages",
        analysis.source_summary.files_scanned, analysis.source_summary.imported_packages
    ));
    append_workspace_summaries(&mut lines, analysis);
    lines.push(String::new());
    append_boundary_warnings(&mut lines, &analysis.boundary_warnings);
    append_english_summary(&mut lines, analysis);
    append_warnings(&mut lines, &analysis.warnings);
    lines.push(String::new());

    append_top_actions_en(&mut lines, analysis, 5);

    lines.push(String::new());
    lines.push("Heaviest known dependencies:".to_string());
    append_section(
        &mut lines,
        &analysis.heavy_dependencies,
        |item, _| {
            let import_text = if item.imported_by.is_empty() {
                "declared but not detected in source".to_string()
            } else {
                format!("imported in {} file(s)", item.imported_by.len())
            };
            with_evidence(
                format!(
                    "- {} ({} KB){}: {} {}.",
                    item.name,
                    item.estimated_kb,
                    confidence_bracket(&item.finding),
                    item.rationale,
                    import_text
                ),
                &item.finding,
                "  ",
            )
        },
        "- none",
    );

    lines.push(String::new());
    lines.push("Duplicate package versions:".to_string());
    append_section(
        &mut lines,
        &analysis.duplicate_packages,
        |item, _| {
            with_detail_lines(
                format!(
                    "- {}{}: {} ({} KB {})",
                    item.name,
                    confidence_bracket(&item.finding),
                    item.versions.join(", "),
                    item.estimated_extra_kb,
                    duplicate_scope_summary_en(item.impact_scope)
                ),
                &duplicate_origin_lines(item),
                &item.finding,
                "  ",
            )
        },
        "- none",
    );

    lines.push(String::new());
    lines.push("Lazy-load candidates:".to_string());
    append_section(
        &mut lines,
        &analysis.lazy_load_candidates,
        |item, _| with_evidence(lazy_load_summary(item), &item.finding, "  "),
        "- none",
    );

    lines.push(String::new());
    lines.push("Tree-shaking warnings:".to_string());
    append_section(
        &mut lines,
        &analysis.tree_shaking_warnings,
        |item, _| {
            with_evidence(
                format!(
                    "- {}{}: {}",
                    item.package_name,
                    confidence_bracket(&item.finding),
                    item.message
                ),
                &item.finding,
                "  ",
            )
        },
        "- none",
    );

    lines.push(String::new());
    lines.push("Unused dependency candidates:".to_string());
    append_section(
        &mut lines,
        &analysis
            .unused_dependency_candidates
            .iter()
            .take(10)
            .collect::<Vec<_>>(),
        |item, _| format!("- {}@{}", item.name, item.version_range),
        "- none",
    );

    if !analysis.bundle_artifacts.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Detected bundle artifacts: {}",
            analysis.bundle_artifacts.join(", ")
        ));
    }

    lines.join("\n")
}

fn format_scan_report_ko(analysis: &Analysis) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "Legolas scan for {}",
        analysis.package_summary.name
    ));
    lines.push(format!("프로젝트 루트: {}", analysis.project_root));
    lines.push(format!("모드: {}", analysis.metadata.mode));
    lines.push(format!(
        "프레임워크: {}",
        if analysis.frameworks.is_empty() {
            "감지 안 됨".to_string()
        } else {
            analysis.frameworks.join(", ")
        }
    ));
    lines.push(format!("패키지 매니저: {}", analysis.package_manager));
    lines.push(format!(
        "스캔: 소스 파일 {}개, import 패키지 {}개",
        analysis.source_summary.files_scanned, analysis.source_summary.imported_packages
    ));
    append_workspace_summaries_ko(&mut lines, analysis);
    lines.push(String::new());
    append_boundary_warnings_ko(&mut lines, &analysis.boundary_warnings);
    append_korean_summary(&mut lines, analysis);
    append_warnings_ko(&mut lines, &analysis.warnings);
    lines.push(String::new());

    append_top_actions_ko(&mut lines, analysis, 5);

    lines.push(String::new());
    lines.push("가장 무거운 의존성:".to_string());
    append_section(
        &mut lines,
        &analysis.heavy_dependencies,
        |item, _| {
            let import_text = if item.imported_by.is_empty() {
                "소스에서 import가 감지되지 않음".to_string()
            } else {
                format!("{}개 파일에서 import됨", item.imported_by.len())
            };
            with_evidence(
                format!(
                    "- {} ({} KB){}: {} {}.",
                    item.name,
                    item.estimated_kb,
                    confidence_bracket_ko(&item.finding),
                    item.rationale,
                    import_text
                ),
                &item.finding,
                "  ",
            )
        },
        "- 없음",
    );

    lines.push(String::new());
    lines.push("중복 패키지 버전:".to_string());
    append_section(
        &mut lines,
        &analysis.duplicate_packages,
        |item, _| {
            with_detail_lines(
                format!(
                    "- {}{}: {} ({} KB {})",
                    item.name,
                    confidence_bracket_ko(&item.finding),
                    item.versions.join(", "),
                    item.estimated_extra_kb,
                    duplicate_scope_summary_ko(item.impact_scope)
                ),
                &duplicate_origin_lines(item),
                &item.finding,
                "  ",
            )
        },
        "- 없음",
    );

    lines.push(String::new());
    lines.push("지연 로딩 후보:".to_string());
    append_section(
        &mut lines,
        &analysis.lazy_load_candidates,
        |item, _| with_evidence(lazy_load_summary_ko(item), &item.finding, "  "),
        "- 없음",
    );

    lines.push(String::new());
    lines.push("트리셰이킹 경고:".to_string());
    append_section(
        &mut lines,
        &analysis.tree_shaking_warnings,
        |item, _| {
            with_evidence(
                format!(
                    "- {}{}: {}",
                    item.package_name,
                    confidence_bracket_ko(&item.finding),
                    item.message
                ),
                &item.finding,
                "  ",
            )
        },
        "- 없음",
    );

    lines.push(String::new());
    lines.push("미사용 의존성 후보:".to_string());
    append_section(
        &mut lines,
        &analysis
            .unused_dependency_candidates
            .iter()
            .take(10)
            .collect::<Vec<_>>(),
        |item, _| format!("- {}@{}", item.name, item.version_range),
        "- 없음",
    );

    if !analysis.bundle_artifacts.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "감지된 번들 아티팩트: {}",
            analysis.bundle_artifacts.join(", ")
        ));
    }

    lines.join("\n")
}

pub fn format_visualization_report_for_language(
    analysis: &Analysis,
    limit: usize,
    language: ReportLanguage,
) -> String {
    match language {
        ReportLanguage::Ko => format_visualization_report_ko(analysis, limit),
        ReportLanguage::En => format_visualization_report(analysis, limit),
    }
}

pub fn format_visualization_report(analysis: &Analysis, limit: usize) -> String {
    let mut lines = Vec::new();
    let normalized_limit = limit.max(1);
    let heavy_dependencies = analysis
        .heavy_dependencies
        .iter()
        .take(normalized_limit)
        .map(|item| BarItem {
            label: item.name.clone(),
            value: item.estimated_kb,
        })
        .collect::<Vec<_>>();
    let duplicates = analysis
        .duplicate_packages
        .iter()
        .take(normalized_limit)
        .map(|item| BarItem {
            label: item.name.clone(),
            value: item.estimated_extra_kb,
        })
        .collect::<Vec<_>>();

    lines.push(format!(
        "Legolas visualize for {}",
        analysis.package_summary.name
    ));
    append_warnings(&mut lines, &analysis.warnings);
    lines.push(String::new());
    lines.push("Estimated dependency weight".to_string());
    lines.push(render_bars(if heavy_dependencies.is_empty() {
        vec![BarItem {
            label: "none".to_string(),
            value: 0,
        }]
    } else {
        heavy_dependencies
    }));
    lines.push(String::new());
    lines.push("Duplicate package pressure".to_string());
    lines.push(render_bars(if duplicates.is_empty() {
        vec![BarItem {
            label: "none".to_string(),
            value: 0,
        }]
    } else {
        duplicates
    }));

    lines.join("\n")
}

fn format_visualization_report_ko(analysis: &Analysis, limit: usize) -> String {
    let mut lines = Vec::new();
    let normalized_limit = limit.max(1);
    let heavy_dependencies = analysis
        .heavy_dependencies
        .iter()
        .take(normalized_limit)
        .map(|item| BarItem {
            label: item.name.clone(),
            value: item.estimated_kb,
        })
        .collect::<Vec<_>>();
    let duplicates = analysis
        .duplicate_packages
        .iter()
        .take(normalized_limit)
        .map(|item| BarItem {
            label: item.name.clone(),
            value: item.estimated_extra_kb,
        })
        .collect::<Vec<_>>();

    lines.push(format!(
        "Legolas visualize for {}",
        analysis.package_summary.name
    ));
    append_warnings_ko(&mut lines, &analysis.warnings);
    lines.push(String::new());
    lines.push("추정 의존성 무게".to_string());
    lines.push(render_bars(if heavy_dependencies.is_empty() {
        vec![BarItem {
            label: "없음".to_string(),
            value: 0,
        }]
    } else {
        heavy_dependencies
    }));
    lines.push(String::new());
    lines.push("중복 패키지 압력".to_string());
    lines.push(render_bars(if duplicates.is_empty() {
        vec![BarItem {
            label: "없음".to_string(),
            value: 0,
        }]
    } else {
        duplicates
    }));

    lines.join("\n")
}

pub fn format_optimize_report_for_language(
    analysis: &Analysis,
    top: usize,
    language: ReportLanguage,
) -> String {
    match language {
        ReportLanguage::Ko => format_optimize_report_ko(analysis, top),
        ReportLanguage::En => format_optimize_report(analysis, top),
    }
}

pub fn format_optimize_report(analysis: &Analysis, top: usize) -> String {
    let mut lines = Vec::new();
    let actions = build_actions(analysis)
        .into_iter()
        .take(top.max(1))
        .collect::<Vec<_>>();

    lines.push(format!(
        "Legolas optimize for {}",
        analysis.package_summary.name
    ));
    append_warnings(&mut lines, &analysis.warnings);
    lines.push(String::new());
    append_section(
        &mut lines,
        &actions,
        render_action_line,
        "1. No high-confidence optimization candidates were found.",
    );
    lines.push(String::new());
    lines.push(format!(
        "Projected savings: ~{} KB, with {} confidence.",
        analysis.impact.potential_kb_saved, analysis.impact.confidence
    ));

    lines.join("\n")
}

fn format_optimize_report_ko(analysis: &Analysis, top: usize) -> String {
    let mut lines = Vec::new();
    let actions = build_actions_for_language(analysis, ReportLanguage::Ko)
        .into_iter()
        .take(top.max(1))
        .collect::<Vec<_>>();

    lines.push(format!(
        "Legolas optimize for {}",
        analysis.package_summary.name
    ));
    append_warnings_ko(&mut lines, &analysis.warnings);
    lines.push(String::new());
    lines.push("Top next actions:".to_string());
    append_section(
        &mut lines,
        &actions,
        render_action_line_ko,
        "1. 신뢰도 높은 최적화 후보를 찾지 못했습니다.",
    );
    lines.push(String::new());
    lines.push(format!(
        "예상 정리 여지: 약 {} KB, 신뢰도 {}.",
        analysis.impact.potential_kb_saved, analysis.impact.confidence
    ));

    lines.join("\n")
}

pub fn format_budget_report_for_language(
    analysis: &Analysis,
    evaluation: &BudgetEvaluation,
    language: ReportLanguage,
) -> String {
    match language {
        ReportLanguage::Ko => format_budget_report_ko(analysis, evaluation),
        ReportLanguage::En => format_budget_report(analysis, evaluation),
    }
}

pub fn format_budget_report(analysis: &Analysis, evaluation: &BudgetEvaluation) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "Legolas budget for {}",
        analysis.package_summary.name
    ));
    append_warnings(&mut lines, &analysis.warnings);
    append_workspace_summaries(&mut lines, analysis);
    lines.push(String::new());
    lines.push(format!("Overall status: {:?}", evaluation.overall_status));
    lines.push(String::new());
    lines.push("Rule results:".to_string());
    append_section(
        &mut lines,
        &evaluation.rules,
        |item, _| {
            format!(
                "- {}: {:?} (actual: {}, warnAt: {}, failAt: {})",
                item.key, item.status, item.actual, item.warn_at, item.fail_at
            )
        },
        "- none",
    );

    lines.join("\n")
}

fn format_budget_report_ko(analysis: &Analysis, evaluation: &BudgetEvaluation) -> String {
    let mut lines = Vec::new();

    lines.push(format!(
        "Legolas budget for {}",
        analysis.package_summary.name
    ));
    append_warnings_ko(&mut lines, &analysis.warnings);
    append_workspace_summaries_ko(&mut lines, analysis);
    lines.push(String::new());
    lines.push(format!("전체 상태: {:?}", evaluation.overall_status));
    lines.push(String::new());
    lines.push("규칙 결과:".to_string());
    append_section(
        &mut lines,
        &evaluation.rules,
        |item, _| {
            format!(
                "- {}: {:?} (actual: {}, warnAt: {}, failAt: {})",
                item.key, item.status, item.actual, item.warn_at, item.fail_at
            )
        },
        "- 없음",
    );

    lines.join("\n")
}

pub fn format_ci_report_for_language(
    analysis: &Analysis,
    evaluation: &BudgetEvaluation,
    language: ReportLanguage,
) -> String {
    match language {
        ReportLanguage::Ko => format_ci_report_ko(analysis, evaluation),
        ReportLanguage::En => format_ci_report(analysis, evaluation),
    }
}

pub fn format_ci_report(analysis: &Analysis, evaluation: &BudgetEvaluation) -> String {
    let mut lines = Vec::new();

    lines.push(format!("Legolas CI for {}", analysis.package_summary.name));
    append_warnings(&mut lines, &analysis.warnings);
    append_workspace_summaries(&mut lines, analysis);
    lines.push(String::new());
    lines.push(format!(
        "Gate result: {}",
        match evaluation.overall_status {
            legolas_core::budget::BudgetStatus::Pass => "PASS",
            legolas_core::budget::BudgetStatus::Warn => "WARN",
            legolas_core::budget::BudgetStatus::Fail => "FAIL",
        }
    ));
    lines.push(format!("Overall status: {:?}", evaluation.overall_status));
    lines.push(format!(
        "Rule statuses: {}",
        evaluation
            .rules
            .iter()
            .map(|item| format!("{}={:?}", item.key, item.status))
            .collect::<Vec<_>>()
            .join(", ")
    ));

    lines.join("\n")
}

fn format_ci_report_ko(analysis: &Analysis, evaluation: &BudgetEvaluation) -> String {
    let mut lines = Vec::new();

    lines.push(format!("Legolas CI for {}", analysis.package_summary.name));
    append_warnings_ko(&mut lines, &analysis.warnings);
    append_workspace_summaries_ko(&mut lines, analysis);
    lines.push(String::new());
    lines.push(format!(
        "게이트 결과: {}",
        match evaluation.overall_status {
            legolas_core::budget::BudgetStatus::Pass => "PASS",
            legolas_core::budget::BudgetStatus::Warn => "WARN",
            legolas_core::budget::BudgetStatus::Fail => "FAIL",
        }
    ));
    lines.push(format!("전체 상태: {:?}", evaluation.overall_status));
    lines.push(format!(
        "규칙 상태: {}",
        evaluation
            .rules
            .iter()
            .map(|item| format!("{}={:?}", item.key, item.status))
            .collect::<Vec<_>>()
            .join(", ")
    ));

    lines.join("\n")
}

fn append_workspace_summaries(lines: &mut Vec<String>, analysis: &Analysis) {
    if analysis.workspace_summaries.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push("Workspace summaries:".to_string());
    append_section(
        lines,
        &analysis.workspace_summaries,
        |item, _| {
            format!(
                "- {} ({}): {} imported packages, {} heavy dependencies, {} duplicate packages, ~{} KB potential saved",
                item.name,
                item.path,
                item.imported_packages,
                item.heavy_dependencies,
                item.duplicate_packages,
                item.potential_kb_saved
            )
        },
        "- none",
    );
}

fn append_workspace_summaries_ko(lines: &mut Vec<String>, analysis: &Analysis) {
    if analysis.workspace_summaries.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push("워크스페이스 요약:".to_string());
    append_section(
        lines,
        &analysis.workspace_summaries,
        |item, _| {
            format!(
                "- {} ({}): import 패키지 {}개, 무거운 의존성 {}개, 중복 패키지 {}개, 약 {} KB 정리 여지",
                item.name,
                item.path,
                item.imported_packages,
                item.heavy_dependencies,
                item.duplicate_packages,
                item.potential_kb_saved
            )
        },
        "- 없음",
    );
}

#[derive(Clone)]
struct BarItem {
    label: String,
    value: usize,
}

#[derive(Clone)]
struct ActionLine {
    headline: String,
    details: Vec<String>,
    evidence: Vec<String>,
}

fn build_actions(analysis: &Analysis) -> Vec<ActionLine> {
    build_actions_for_language(analysis, ReportLanguage::En)
}

fn build_actions_for_language(analysis: &Analysis, language: ReportLanguage) -> Vec<ActionLine> {
    let ranked = build_ranked_actions(analysis, language);
    if !ranked.is_empty() {
        return ranked;
    }

    build_legacy_actions(analysis, language)
}

fn build_ranked_actions(analysis: &Analysis, language: ReportLanguage) -> Vec<ActionLine> {
    let contexts = build_action_contexts(analysis, language);

    rank_actions(analysis)
        .into_iter()
        .map(|action| {
            let context = contexts.get(&action.finding_id);
            ActionLine {
                headline: action_headline(
                    context
                        .map(|item| item.headline.as_str())
                        .unwrap_or(action.finding_id.as_str()),
                    action.difficulty,
                    action.confidence,
                    action.estimated_savings_kb,
                    language,
                ),
                details: recommended_fix_details_for_language(
                    action.recommended_fix.as_ref(),
                    language,
                ),
                evidence: context
                    .map(|item| item.evidence.clone())
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn build_legacy_actions(analysis: &Analysis, language: ReportLanguage) -> Vec<ActionLine> {
    let mut actions = Vec::new();

    for dependency in analysis.heavy_dependencies.iter().take(3) {
        if dependency.imported_by.is_empty() {
            actions.push(ActionLine {
                headline: match language {
                    ReportLanguage::Ko => format!(
                        "{} 의존성을 제거하거나 필요성을 설명하세요. 스캔한 소스에서 import가 감지되지 않았습니다.",
                        dependency.name
                    ),
                    ReportLanguage::En => format!(
                        "Remove or justify {}; it is declared but not imported in scanned source files.",
                        dependency.name
                    ),
                },
                details: Vec::new(),
                evidence: display_evidence_lines(&dependency.finding),
            });
            continue;
        }

        actions.push(ActionLine {
            headline: match language {
                ReportLanguage::Ko => {
                    format!("{} 검토: {}", dependency.name, dependency.recommendation)
                }
                ReportLanguage::En => {
                    format!("Review {}: {}", dependency.name, dependency.recommendation)
                }
            },
            details: Vec::new(),
            evidence: display_evidence_lines(&dependency.finding),
        });
    }

    for duplicate in analysis.duplicate_packages.iter().take(3) {
        actions.push(ActionLine {
            headline: duplicate_action_headline(duplicate, language),
            details: Vec::new(),
            evidence: display_evidence_lines(&duplicate.finding),
        });
    }

    for candidate in analysis.lazy_load_candidates.iter().take(3) {
        let file = candidate
            .files
            .first()
            .map(String::as_str)
            .unwrap_or("undefined");
        actions.push(ActionLine {
            headline: match language {
                ReportLanguage::Ko => format!(
                    "{}를 {}에서 지연 로딩해 약 {} KB 지연 로딩 여지를 검토하세요.",
                    candidate.name, file, candidate.estimated_savings_kb
                ),
                ReportLanguage::En => format!(
                    "Lazy load {} in {} to target roughly {} KB of deferred code.",
                    candidate.name, file, candidate.estimated_savings_kb
                ),
            },
            details: Vec::new(),
            evidence: display_evidence_lines(&candidate.finding),
        });
    }

    for warning in analysis.tree_shaking_warnings.iter().take(2) {
        actions.push(ActionLine {
            headline: match language {
                ReportLanguage::Ko => format!(
                    "{} import 정리: {}",
                    warning.package_name, warning.recommendation
                ),
                ReportLanguage::En => format!(
                    "Clean up {} imports: {}",
                    warning.package_name, warning.recommendation
                ),
            },
            details: Vec::new(),
            evidence: display_evidence_lines(&warning.finding),
        });
    }

    dedupe_actions(actions)
}

fn render_bars(items: Vec<BarItem>) -> String {
    let max_value = items
        .iter()
        .map(|item| item.value)
        .max()
        .unwrap_or(1)
        .max(1);

    items
        .into_iter()
        .map(|item| {
            let bar_length = if item.value == 0 {
                0
            } else {
                (((item.value as f64 / max_value as f64) * 24.0).round() as usize).max(1)
            };
            let bar = "█".repeat(bar_length);
            format!("{:<24} {:<24} {} KB", item.label, bar, item.value)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn append_section<T, F>(lines: &mut Vec<String>, items: &[T], render_item: F, fallback_line: &str)
where
    F: Fn(&T, usize) -> String,
{
    if items.is_empty() {
        lines.push(fallback_line.to_string());
        return;
    }

    for (index, item) in items.iter().enumerate() {
        lines.push(render_item(item, index));
    }
}

fn dedupe_actions(items: Vec<ActionLine>) -> Vec<ActionLine> {
    let mut deduped = Vec::new();

    for item in items {
        if !deduped
            .iter()
            .any(|existing: &ActionLine| existing.headline == item.headline)
        {
            deduped.push(item);
        }
    }

    deduped
}

fn with_evidence(summary: String, finding: &FindingMetadata, indent: &str) -> String {
    with_detail_lines(summary, &[], finding, indent)
}

fn lazy_load_summary(item: &legolas_core::LazyLoadCandidate) -> String {
    if item.reason.contains("route-aware") && !item.files.is_empty() {
        if item.files.len() == 1 {
            return format!(
                "- {}{}: route surface {} statically imports {} and usually tolerates lazy loading. Estimated win {} KB.",
                item.name,
                confidence_bracket(&item.finding),
                item.files[0],
                item.name,
                item.estimated_savings_kb
            );
        }

        return format!(
            "- {}{}: route surfaces {} statically import {} and usually tolerate lazy loading. Estimated win {} KB.",
            item.name,
            confidence_bracket(&item.finding),
            item.files.join(", "),
            item.name,
            item.estimated_savings_kb
        );
    }

    format!(
        "- {}{}: {}. Estimated win {} KB.",
        item.name,
        confidence_bracket(&item.finding),
        item.reason,
        item.estimated_savings_kb
    )
}

fn with_detail_lines(
    summary: String,
    details: &[String],
    finding: &FindingMetadata,
    indent: &str,
) -> String {
    let mut lines = vec![summary];
    lines.extend(details.iter().map(|detail| format!("{indent}{detail}")));

    for evidence in display_evidence_lines(finding) {
        lines.push(format!("{indent}evidence: {evidence}"));
    }

    lines.join("\n")
}

fn confidence_bracket(finding: &FindingMetadata) -> String {
    finding
        .confidence
        .map(|confidence| format!(" [{}]", confidence_phrase(confidence)))
        .unwrap_or_default()
}

fn confidence_bracket_ko(finding: &FindingMetadata) -> String {
    finding
        .confidence
        .map(|confidence| format!(" [{} 신뢰도]", confidence_label_ko(confidence)))
        .unwrap_or_default()
}

#[derive(Clone)]
struct ActionContext {
    headline: String,
    evidence: Vec<String>,
}

fn build_action_contexts(
    analysis: &Analysis,
    language: ReportLanguage,
) -> BTreeMap<String, ActionContext> {
    let mut contexts = BTreeMap::new();

    for dependency in &analysis.heavy_dependencies {
        insert_action_context(
            &mut contexts,
            dependency.finding.finding_id.as_ref(),
            match language {
                ReportLanguage::Ko => format!("{} 초기 번들 무게 검토", dependency.name),
                ReportLanguage::En => format!("Review {} upfront bundle weight", dependency.name),
            },
            &dependency.finding,
        );
    }

    for duplicate in &analysis.duplicate_packages {
        insert_action_context(
            &mut contexts,
            duplicate.finding.finding_id.as_ref(),
            duplicate_context_headline(duplicate, language),
            &duplicate.finding,
        );
    }

    for candidate in &analysis.lazy_load_candidates {
        insert_action_context(
            &mut contexts,
            candidate.finding.finding_id.as_ref(),
            match language {
                ReportLanguage::Ko => format!("{} 지연 로딩", candidate.name),
                ReportLanguage::En => format!("Lazy load {}", candidate.name),
            },
            &candidate.finding,
        );
    }

    for warning in &analysis.tree_shaking_warnings {
        insert_action_context(
            &mut contexts,
            warning.finding.finding_id.as_ref(),
            match language {
                ReportLanguage::Ko => format!("{} import 정리", warning.package_name),
                ReportLanguage::En => format!("Clean up {} imports", warning.package_name),
            },
            &warning.finding,
        );
    }

    contexts
}

fn insert_action_context(
    contexts: &mut BTreeMap<String, ActionContext>,
    finding_id: Option<&String>,
    headline: String,
    finding: &FindingMetadata,
) {
    let Some(finding_id) = finding_id else {
        return;
    };

    contexts.insert(
        finding_id.clone(),
        ActionContext {
            headline,
            evidence: display_evidence_lines(finding),
        },
    );
}

fn difficulty_label(difficulty: ActionDifficulty) -> &'static str {
    match difficulty {
        ActionDifficulty::Easy => "easy",
        ActionDifficulty::Medium => "medium",
        ActionDifficulty::Hard => "hard",
    }
}

fn difficulty_label_ko(difficulty: ActionDifficulty) -> &'static str {
    match difficulty {
        ActionDifficulty::Easy => "쉬움",
        ActionDifficulty::Medium => "보통",
        ActionDifficulty::Hard => "어려움",
    }
}

fn confidence_label(confidence: FindingConfidence) -> &'static str {
    confidence_display(confidence)
}

fn confidence_label_ko(confidence: FindingConfidence) -> &'static str {
    match confidence {
        FindingConfidence::Low => "낮음",
        FindingConfidence::Medium => "중간",
        FindingConfidence::High => "높음",
    }
}

fn confidence_phrase(confidence: FindingConfidence) -> &'static str {
    match confidence {
        FindingConfidence::Low => "low confidence",
        FindingConfidence::Medium => "medium confidence",
        FindingConfidence::High => "high confidence",
    }
}

fn confidence_display(confidence: FindingConfidence) -> &'static str {
    match confidence {
        FindingConfidence::Low => "low",
        FindingConfidence::Medium => "medium",
        FindingConfidence::High => "high",
    }
}

fn recommended_fix_details_for_language(
    recommended_fix: Option<&RecommendedFix>,
    language: ReportLanguage,
) -> Vec<String> {
    let Some(recommended_fix) = recommended_fix else {
        return Vec::new();
    };

    let mut details = vec![match language {
        ReportLanguage::Ko => format!(
            "권장 수정: {} - {}",
            recommended_fix.kind, recommended_fix.title
        ),
        ReportLanguage::En => format!(
            "recommended fix: {} - {}",
            recommended_fix.kind, recommended_fix.title
        ),
    }];

    if !recommended_fix.target_files.is_empty() {
        details.push(match language {
            ReportLanguage::Ko => format!("대상: {}", recommended_fix.target_files.join(", ")),
            ReportLanguage::En => format!("targets: {}", recommended_fix.target_files.join(", ")),
        });
    }

    if let Some(replacement) = recommended_fix.replacement.as_deref() {
        details.push(match language {
            ReportLanguage::Ko => format!("대체 후보: {replacement}"),
            ReportLanguage::En => format!("replacement: {replacement}"),
        });
    }

    details
}

fn action_headline(
    base: &str,
    difficulty: ActionDifficulty,
    confidence: FindingConfidence,
    estimated_savings_kb: usize,
    language: ReportLanguage,
) -> String {
    match language {
        ReportLanguage::Ko => format!(
            "{} [난이도: {} | 신뢰도: {} | ~{} KB]",
            base,
            difficulty_label_ko(difficulty),
            confidence_label_ko(confidence),
            estimated_savings_kb
        ),
        ReportLanguage::En => format!(
            "{} [{} | {} confidence | ~{} KB]",
            base,
            difficulty_label(difficulty),
            confidence_label(confidence),
            estimated_savings_kb
        ),
    }
}

fn render_action_line(item: &ActionLine, index: usize) -> String {
    let mut lines = vec![format!("{}. {}", index + 1, item.headline)];

    for detail in &item.details {
        lines.push(format!("   {detail}"));
    }

    for evidence in &item.evidence {
        lines.push(format!("   evidence: {evidence}"));
    }

    lines.join("\n")
}

fn render_action_line_ko(item: &ActionLine, index: usize) -> String {
    let mut lines = vec![format!("{}. {}", index + 1, item.headline)];

    for detail in &item.details {
        lines.push(format!("   {detail}"));
    }

    for evidence in &item.evidence {
        lines.push(format!("   evidence: {evidence}"));
    }

    lines.join("\n")
}

fn display_evidence_lines(finding: &FindingMetadata) -> Vec<String> {
    let lines = finding
        .evidence
        .iter()
        .map(format_evidence)
        .collect::<Vec<_>>();
    if finding
        .evidence
        .iter()
        .any(|evidence| evidence.kind == "artifact-chunk")
    {
        lines
    } else {
        lines.into_iter().take(1).collect()
    }
}

fn format_evidence(evidence: &FindingEvidence) -> String {
    let mut parts = Vec::new();

    if let Some(file) = evidence.file.as_deref() {
        parts.push(file.to_string());
    }
    if let Some(specifier) = evidence.specifier.as_deref() {
        parts.push(format!("specifier: {specifier}"));
    }
    if let Some(detail) = evidence.detail.as_deref() {
        parts.push(detail.to_string());
    }

    if parts.is_empty() {
        evidence.kind.clone()
    } else {
        parts.join(" | ")
    }
}

fn duplicate_origin_lines(item: &legolas_core::DuplicatePackage) -> Vec<String> {
    item.origins
        .iter()
        .map(|origin| {
            format!(
                "origin: {} via {}",
                origin.version,
                format_origin_chain(origin)
            )
        })
        .collect()
}

fn format_origin_chain(origin: &legolas_core::DuplicateOrigin) -> String {
    let mut chain = origin.via_chain.clone();
    if chain.is_empty() {
        chain.push(origin.root_requester.clone());
    } else if chain.first() != Some(&origin.root_requester) {
        chain.insert(0, origin.root_requester.clone());
    }

    chain.join(" -> ")
}

fn append_korean_summary(lines: &mut Vec<String>, analysis: &Analysis) {
    let summary = build_report_summary(analysis);
    let has_confirmed_initial_payload_savings = summary.confirmed_initial_payload_kb_saved > 0;

    lines.push(format!("판정: {}", verdict_label_ko(&summary.verdict_key)));
    if has_confirmed_initial_payload_savings {
        lines.push(format!(
            "확정 초기 페이로드 절감: 약 {} KB (LCP 약 {} ms 개선 추정)",
            summary.confirmed_initial_payload_kb_saved, summary.estimated_lcp_improvement_ms
        ));
    } else {
        lines.push(
            "확정 초기 페이로드 절감: 미확정 (중복 의존성 압력만으로는 초기 페이로드/LCP 절감을 확정하지 않음)"
                .to_string(),
        );
    }
    lines.push(format!(
        "방향성 정리 여지: 약 {} KB",
        summary.directional_opportunity_kb
    ));
}

fn append_english_summary(lines: &mut Vec<String>, analysis: &Analysis) {
    let summary = build_report_summary(analysis);
    let has_confirmed_initial_payload_savings = summary.confirmed_initial_payload_kb_saved > 0;

    lines.push(format!(
        "Verdict: {}",
        verdict_label_en(&summary.verdict_key)
    ));
    if has_confirmed_initial_payload_savings {
        lines.push(format!(
            "Confirmed initial payload savings: ~{} KB (estimated LCP improvement ~{} ms)",
            summary.confirmed_initial_payload_kb_saved, summary.estimated_lcp_improvement_ms
        ));
    } else {
        lines.push(
            "Confirmed initial payload savings: not confirmed (duplicate dependency pressure alone does not confirm initial payload or LCP savings)"
                .to_string(),
        );
    }
    lines.push(format!(
        "Directional cleanup opportunity: ~{} KB",
        summary.directional_opportunity_kb
    ));
}

fn append_top_actions_ko(lines: &mut Vec<String>, analysis: &Analysis, limit: usize) {
    let actions = build_actions_for_language(analysis, ReportLanguage::Ko)
        .into_iter()
        .take(limit.max(1))
        .collect::<Vec<_>>();

    lines.push("Top next actions:".to_string());
    append_section(
        lines,
        &actions,
        render_action_line_ko,
        "1. 신뢰도 높은 다음 조치를 찾지 못했습니다.",
    );
}

fn append_top_actions_en(lines: &mut Vec<String>, analysis: &Analysis, limit: usize) {
    let actions = build_actions_for_language(analysis, ReportLanguage::En)
        .into_iter()
        .take(limit.max(1))
        .collect::<Vec<_>>();

    lines.push("Top next actions:".to_string());
    append_section(
        lines,
        &actions,
        render_action_line,
        "1. No high-confidence optimization candidates were found.",
    );
}

fn verdict_label_ko(verdict_key: &str) -> &'static str {
    match verdict_key {
        "high-impact" => "큰 폭 절감 가능",
        "medium-impact" => "의미 있는 절감 가능",
        "targeted-impact" => "국소 개선 가능",
        "low-impact" => "명확한 절감 근거 제한적",
        _ => "추가 확인 필요",
    }
}

fn verdict_label_en(verdict_key: &str) -> &'static str {
    match verdict_key {
        "high-impact" => "high impact",
        "medium-impact" => "meaningful impact",
        "targeted-impact" => "targeted impact",
        "low-impact" => "limited confirmed impact",
        _ => "needs review",
    }
}

fn duplicate_scope_summary_ko(scope: DuplicateImpactScope) -> &'static str {
    match scope {
        DuplicateImpactScope::ProductionLikely => "초기 페이로드 절감 후보",
        DuplicateImpactScope::DevOnly => "개발/테스트 의존성 중복 정리, dependency hygiene",
        DuplicateImpactScope::Unknown => "방향성 정리 여지",
    }
}

fn duplicate_scope_summary_en(scope: DuplicateImpactScope) -> &'static str {
    match scope {
        DuplicateImpactScope::ProductionLikely => "initial payload candidate",
        DuplicateImpactScope::DevOnly => {
            "development/test dependency duplication, dependency hygiene"
        }
        DuplicateImpactScope::Unknown => "directional cleanup opportunity",
    }
}

fn duplicate_context_headline(
    duplicate: &legolas_core::DuplicatePackage,
    language: ReportLanguage,
) -> String {
    match (language, duplicate.impact_scope) {
        (ReportLanguage::Ko, DuplicateImpactScope::DevOnly) => format!(
            "{} 개발/테스트 의존성 중복 정리 ({})",
            duplicate.name,
            duplicate.versions.join(", ")
        ),
        (ReportLanguage::Ko, _) => format!(
            "{} 버전 중복 정리 ({})",
            duplicate.name,
            duplicate.versions.join(", ")
        ),
        (ReportLanguage::En, DuplicateImpactScope::DevOnly) => format!(
            "Clean up {} development/test dependency duplication ({})",
            duplicate.name,
            duplicate.versions.join(", ")
        ),
        (ReportLanguage::En, _) => format!(
            "Deduplicate {} versions ({})",
            duplicate.name,
            duplicate.versions.join(", ")
        ),
    }
}

fn duplicate_action_headline(
    duplicate: &legolas_core::DuplicatePackage,
    language: ReportLanguage,
) -> String {
    match (language, duplicate.impact_scope) {
        (ReportLanguage::Ko, DuplicateImpactScope::DevOnly) => format!(
            "{} 개발/테스트 의존성 중복({})을 정리하세요. 약 {} KB의 dependency hygiene 여지입니다.",
            duplicate.name,
            duplicate.versions.join(", "),
            duplicate.estimated_extra_kb
        ),
        (ReportLanguage::Ko, _) => format!(
            "{} 버전 중복({})을 정리해 약 {} KB의 방향성 정리 여지를 확인하세요.",
            duplicate.name,
            duplicate.versions.join(", "),
            duplicate.estimated_extra_kb
        ),
        (ReportLanguage::En, DuplicateImpactScope::DevOnly) => format!(
            "Clean up {} development/test dependency duplication ({}) for roughly {} KB of dependency hygiene.",
            duplicate.name,
            duplicate.versions.join(", "),
            duplicate.estimated_extra_kb
        ),
        (ReportLanguage::En, _) => format!(
            "Deduplicate {} versions ({}) to recover roughly {} KB.",
            duplicate.name,
            duplicate.versions.join(", "),
            duplicate.estimated_extra_kb
        ),
    }
}

fn lazy_load_summary_ko(item: &legolas_core::LazyLoadCandidate) -> String {
    if item.reason.contains("route-aware") && !item.files.is_empty() {
        if item.files.len() == 1 {
            return format!(
                "- {}{}: route surface {}에서 {}를 정적 import합니다. 지연 로딩 검토 여지 {} KB.",
                item.name,
                confidence_bracket_ko(&item.finding),
                item.files[0],
                item.name,
                item.estimated_savings_kb
            );
        }

        return format!(
            "- {}{}: route surfaces {}에서 {}를 정적 import합니다. 지연 로딩 검토 여지 {} KB.",
            item.name,
            confidence_bracket_ko(&item.finding),
            item.files.join(", "),
            item.name,
            item.estimated_savings_kb
        );
    }

    format!(
        "- {}{}: {}. 지연 로딩 검토 여지 {} KB.",
        item.name,
        confidence_bracket_ko(&item.finding),
        item.reason,
        item.estimated_savings_kb
    )
}

fn append_warnings(lines: &mut Vec<String>, warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push("Warnings:".to_string());
    for warning in warnings {
        lines.push(format!("- {warning}"));
    }
}

fn append_warnings_ko(lines: &mut Vec<String>, warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push("경고:".to_string());
    for warning in warnings {
        lines.push(format!("- {warning}"));
    }
}

fn append_boundary_warnings(lines: &mut Vec<String>, warnings: &[BoundaryWarning]) {
    if warnings.is_empty() {
        return;
    }

    lines.push("Boundary warnings:".to_string());
    for warning in warnings {
        lines.push(format!("- {}", warning.message));
        lines.push(format!("  recommendation: {}", warning.recommendation));
        for evidence in display_evidence_lines(&warning.finding) {
            lines.push(format!("  evidence: {evidence}"));
        }
    }
    lines.push(String::new());
}

fn append_boundary_warnings_ko(lines: &mut Vec<String>, warnings: &[BoundaryWarning]) {
    if warnings.is_empty() {
        return;
    }

    lines.push("경계 경고:".to_string());
    for warning in warnings {
        lines.push(format!("- {}", warning.message));
        lines.push(format!("  권장: {}", warning.recommendation));
        for evidence in display_evidence_lines(&warning.finding) {
            lines.push(format!("  evidence: {evidence}"));
        }
    }
    lines.push(String::new());
}

#[cfg(test)]
mod tests {
    use super::format_scan_report;
    use legolas_core::{
        boundaries::BoundaryWarning, Analysis, FindingAnalysisSource, FindingConfidence,
        FindingEvidence, FindingMetadata, Impact, Metadata, PackageSummary, SourceSummary,
    };

    #[test]
    fn format_scan_report_renders_boundary_warnings() {
        let analysis = Analysis {
            package_summary: PackageSummary {
                name: "boundary-app".to_string(),
                ..Default::default()
            },
            source_summary: SourceSummary {
                files_scanned: 1,
                imported_packages: 1,
                ..Default::default()
            },
            impact: Impact {
                summary: "summary".to_string(),
                ..Default::default()
            },
            metadata: Metadata {
                mode: "heuristic".to_string(),
                generated_at: "2026-04-24T00:00:00Z".to_string(),
            },
            boundary_warnings: vec![BoundaryWarning {
                message: "RSC surface `app/page.tsx` imports the server-only `server-only` module."
                    .to_string(),
                recommendation:
                    "Keep server-only guards in server-only utilities and avoid importing them directly from RSC entrypoints."
                        .to_string(),
                finding: FindingMetadata::new(
                    "boundary:rsc-server-only",
                    FindingAnalysisSource::SourceImport,
                )
                .with_confidence(FindingConfidence::High)
                .with_action_priority(1)
                .with_evidence([FindingEvidence::new("source-file")
                    .with_file("app/page.tsx")
                    .with_specifier("server-only")
                    .with_detail("RSC surface imports a server-only module")]),
            }],
            ..Default::default()
        };

        let report = format_scan_report(&analysis);

        assert!(report.contains("Boundary warnings:"));
        assert!(report
            .contains("RSC surface `app/page.tsx` imports the server-only `server-only` module."));
        assert!(report.contains("recommendation: Keep server-only guards in server-only utilities and avoid importing them directly from RSC entrypoints."));
        assert!(report.contains("evidence: app/page.tsx | specifier: server-only | RSC surface imports a server-only module"));
    }
}
