use legolas_core::{
    budget::BudgetEvaluation, rank_actions, ActionDifficulty, Analysis, FindingConfidence,
    FindingEvidence, FindingMetadata, RecommendedFix,
};
use std::collections::BTreeMap;

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
    lines.push(format!(
        "Potential payload reduction: ~{} KB",
        analysis.impact.potential_kb_saved
    ));
    lines.push(format!(
        "Estimated LCP improvement: ~{} ms",
        analysis.impact.estimated_lcp_improvement_ms
    ));
    lines.push(analysis.impact.summary.clone());
    append_warnings(&mut lines, &analysis.warnings);
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
                    "- {}{}: {} ({} KB avoidable)",
                    item.name,
                    confidence_bracket(&item.finding),
                    item.versions.join(", "),
                    item.estimated_extra_kb
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
    let ranked = build_ranked_actions(analysis);
    if !ranked.is_empty() {
        return ranked;
    }

    build_legacy_actions(analysis)
}

fn build_ranked_actions(analysis: &Analysis) -> Vec<ActionLine> {
    let contexts = build_action_contexts(analysis);

    rank_actions(analysis)
        .into_iter()
        .map(|action| {
            let context = contexts.get(&action.finding_id);
            ActionLine {
                headline: format!(
                    "{} [{} | {} confidence | ~{} KB]",
                    context
                        .map(|item| item.headline.as_str())
                        .unwrap_or(action.finding_id.as_str()),
                    difficulty_label(action.difficulty),
                    confidence_label(action.confidence),
                    action.estimated_savings_kb
                ),
                details: recommended_fix_details(action.recommended_fix.as_ref()),
                evidence: context
                    .map(|item| item.evidence.clone())
                    .unwrap_or_default(),
            }
        })
        .collect()
}

fn build_legacy_actions(analysis: &Analysis) -> Vec<ActionLine> {
    let mut actions = Vec::new();

    for dependency in analysis.heavy_dependencies.iter().take(3) {
        if dependency.imported_by.is_empty() {
            actions.push(ActionLine {
                headline: format!(
                    "Remove or justify {}; it is declared but not imported in scanned source files.",
                    dependency.name
                ),
                details: Vec::new(),
                evidence: display_evidence_lines(&dependency.finding),
            });
            continue;
        }

        actions.push(ActionLine {
            headline: format!("Review {}: {}", dependency.name, dependency.recommendation),
            details: Vec::new(),
            evidence: display_evidence_lines(&dependency.finding),
        });
    }

    for duplicate in analysis.duplicate_packages.iter().take(3) {
        actions.push(ActionLine {
            headline: format!(
                "Deduplicate {} versions ({}) to recover roughly {} KB.",
                duplicate.name,
                duplicate.versions.join(", "),
                duplicate.estimated_extra_kb
            ),
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
            headline: format!(
                "Lazy load {} in {} to target roughly {} KB of deferred code.",
                candidate.name, file, candidate.estimated_savings_kb
            ),
            details: Vec::new(),
            evidence: display_evidence_lines(&candidate.finding),
        });
    }

    for warning in analysis.tree_shaking_warnings.iter().take(2) {
        actions.push(ActionLine {
            headline: format!(
                "Clean up {} imports: {}",
                warning.package_name, warning.recommendation
            ),
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

#[derive(Clone)]
struct ActionContext {
    headline: String,
    evidence: Vec<String>,
}

fn build_action_contexts(analysis: &Analysis) -> BTreeMap<String, ActionContext> {
    let mut contexts = BTreeMap::new();

    for dependency in &analysis.heavy_dependencies {
        insert_action_context(
            &mut contexts,
            dependency.finding.finding_id.as_ref(),
            format!("Review {} upfront bundle weight", dependency.name),
            &dependency.finding,
        );
    }

    for duplicate in &analysis.duplicate_packages {
        insert_action_context(
            &mut contexts,
            duplicate.finding.finding_id.as_ref(),
            format!(
                "Deduplicate {} versions ({})",
                duplicate.name,
                duplicate.versions.join(", ")
            ),
            &duplicate.finding,
        );
    }

    for candidate in &analysis.lazy_load_candidates {
        insert_action_context(
            &mut contexts,
            candidate.finding.finding_id.as_ref(),
            format!("Lazy load {}", candidate.name),
            &candidate.finding,
        );
    }

    for warning in &analysis.tree_shaking_warnings {
        insert_action_context(
            &mut contexts,
            warning.finding.finding_id.as_ref(),
            format!("Clean up {} imports", warning.package_name),
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

fn confidence_label(confidence: FindingConfidence) -> &'static str {
    confidence_display(confidence)
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

fn recommended_fix_details(recommended_fix: Option<&RecommendedFix>) -> Vec<String> {
    let Some(recommended_fix) = recommended_fix else {
        return Vec::new();
    };

    let mut details = vec![format!(
        "recommended fix: {} - {}",
        recommended_fix.kind, recommended_fix.title
    )];

    if !recommended_fix.target_files.is_empty() {
        details.push(format!(
            "targets: {}",
            recommended_fix.target_files.join(", ")
        ));
    }

    if let Some(replacement) = recommended_fix.replacement.as_deref() {
        details.push(format!("replacement: {replacement}"));
    }

    details
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
