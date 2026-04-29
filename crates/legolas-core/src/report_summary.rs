use crate::{
    action_plan::rank_actions,
    models::{
        Analysis, DuplicateImpactScope, ReportSummary, ReportSummaryAction,
        ReportSummaryGroupSummary, ReportSummaryTextSource,
    },
};

const TOP_ACTION_LIMIT: usize = 5;

pub fn build_report_summary(analysis: &Analysis) -> ReportSummary {
    let group_summaries = build_group_summaries(analysis);
    let confirmed_initial_payload_kb_saved = group_summaries
        .iter()
        .map(|group| group.confirmed_initial_payload_kb_saved)
        .sum();
    let directional_opportunity_kb = group_summaries
        .iter()
        .map(|group| group.directional_opportunity_kb)
        .sum();
    let verdict_key = verdict_key(confirmed_initial_payload_kb_saved).to_string();

    ReportSummary {
        text_source: text_source_for_verdict(&verdict_key),
        verdict_key,
        confirmed_initial_payload_kb_saved,
        directional_opportunity_kb,
        estimated_lcp_improvement_ms: (confirmed_initial_payload_kb_saved as f64 * 2.1).round()
            as usize,
        top_actions: rank_actions(analysis)
            .into_iter()
            .take(TOP_ACTION_LIMIT)
            .map(|action| ReportSummaryAction {
                action_priority: action.action_priority,
                finding_id: action.finding_id,
                estimated_savings_kb: action.estimated_savings_kb,
                confidence: action.confidence,
                difficulty: action.difficulty,
            })
            .collect(),
        group_summaries,
    }
}

fn build_group_summaries(analysis: &Analysis) -> Vec<ReportSummaryGroupSummary> {
    let heavy_dependency_kb = analysis
        .heavy_dependencies
        .iter()
        .take(5)
        .map(|item| item.estimated_kb as f64 * 0.18)
        .sum::<f64>()
        .round() as usize;
    let production_duplicate_kb = analysis
        .duplicate_packages
        .iter()
        .filter(|item| item.impact_scope == DuplicateImpactScope::ProductionLikely)
        .map(|item| item.estimated_extra_kb)
        .sum();
    let all_duplicate_kb = analysis
        .duplicate_packages
        .iter()
        .map(|item| item.estimated_extra_kb)
        .sum();
    let lazy_load_kb = analysis
        .lazy_load_candidates
        .iter()
        .map(|item| item.estimated_savings_kb)
        .sum();
    let tree_shaking_kb = analysis
        .tree_shaking_warnings
        .iter()
        .map(|item| item.estimated_kb)
        .sum();

    vec![
        group_summary(
            "heavy-dependencies",
            analysis.heavy_dependencies.len(),
            heavy_dependency_kb,
            heavy_dependency_kb,
        ),
        group_summary(
            "duplicate-packages",
            analysis.duplicate_packages.len(),
            production_duplicate_kb,
            all_duplicate_kb,
        ),
        group_summary(
            "lazy-load-candidates",
            analysis.lazy_load_candidates.len(),
            lazy_load_kb,
            lazy_load_kb,
        ),
        group_summary(
            "tree-shaking-warnings",
            analysis.tree_shaking_warnings.len(),
            tree_shaking_kb,
            tree_shaking_kb,
        ),
    ]
}

fn group_summary(
    key: &str,
    finding_count: usize,
    confirmed_initial_payload_kb_saved: usize,
    directional_opportunity_kb: usize,
) -> ReportSummaryGroupSummary {
    ReportSummaryGroupSummary {
        key: key.to_string(),
        finding_count,
        confirmed_initial_payload_kb_saved,
        directional_opportunity_kb,
    }
}

fn verdict_key(confirmed_initial_payload_kb_saved: usize) -> &'static str {
    if confirmed_initial_payload_kb_saved >= 300 {
        return "high-impact";
    }

    if confirmed_initial_payload_kb_saved >= 120 {
        return "medium-impact";
    }

    if confirmed_initial_payload_kb_saved >= 40 {
        return "targeted-impact";
    }

    "low-impact"
}

fn text_source_for_verdict(verdict_key: &str) -> ReportSummaryTextSource {
    ReportSummaryTextSource {
        title_key: format!("report.summary.{verdict_key}.title"),
        body_key: format!("report.summary.{verdict_key}.body"),
    }
}
