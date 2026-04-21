use std::collections::{BTreeMap, BTreeSet};

use crate::{
    findings::{FindingConfidence, FindingMetadata},
    models::{ActionDifficulty, ActionPlanItem, Analysis, RecommendedFix},
};

pub fn rank_actions(analysis: &Analysis) -> Vec<ActionPlanItem> {
    let mut actions = Vec::new();

    for item in &analysis.heavy_dependencies {
        push_action(
            &mut actions,
            &item.finding,
            item.estimated_kb,
            ActionDifficulty::Hard,
            Some(recommended_fix(
                recommended_fix_kind(&item.name, &item.recommendation),
                item.recommendation.clone(),
                item.imported_by.clone(),
                &item.name,
            )),
        );
    }

    for item in &analysis.lazy_load_candidates {
        push_action(
            &mut actions,
            &item.finding,
            item.estimated_savings_kb,
            ActionDifficulty::Medium,
            Some(recommended_fix(
                "lazy-load",
                item.recommendation.clone(),
                item.files.clone(),
                &item.name,
            )),
        );
    }

    for item in &analysis.tree_shaking_warnings {
        push_action(
            &mut actions,
            &item.finding,
            item.estimated_kb,
            ActionDifficulty::Easy,
            Some(recommended_fix(
                "narrow-import",
                item.recommendation.clone(),
                item.files.clone(),
                &item.package_name,
            )),
        );
    }

    for item in &analysis.duplicate_packages {
        push_action(
            &mut actions,
            &item.finding,
            item.estimated_extra_kb,
            ActionDifficulty::Medium,
            Some(RecommendedFix {
                kind: "dedupe-package".to_string(),
                title: format!("Deduplicate {} to one installed version.", item.name),
                target_files: Vec::new(),
                replacement: None,
            }),
        );
    }

    actions.sort_by(|left, right| {
        right
            .estimated_savings_kb
            .cmp(&left.estimated_savings_kb)
            .then(right.confidence.cmp(&left.confidence))
            .then(left.difficulty.cmp(&right.difficulty))
            .then(left.finding_id.cmp(&right.finding_id))
    });

    let mut seen = BTreeSet::new();
    let mut ranked = actions
        .into_iter()
        .filter(|item| seen.insert(item.finding_id.clone()))
        .collect::<Vec<_>>();

    for (index, item) in ranked.iter_mut().enumerate() {
        item.action_priority = index + 1;
    }

    ranked
}

pub fn apply_action_plan(analysis: &mut Analysis) -> Vec<ActionPlanItem> {
    let actions = rank_actions(analysis);
    let action_by_id = actions
        .iter()
        .map(|action| (action.finding_id.clone(), action.clone()))
        .collect::<BTreeMap<_, _>>();

    for item in &mut analysis.heavy_dependencies {
        apply_action_metadata(&mut item.finding, &action_by_id);
    }
    for item in &mut analysis.lazy_load_candidates {
        apply_action_metadata(&mut item.finding, &action_by_id);
    }
    for item in &mut analysis.tree_shaking_warnings {
        apply_action_metadata(&mut item.finding, &action_by_id);
    }
    for item in &mut analysis.duplicate_packages {
        apply_action_metadata(&mut item.finding, &action_by_id);
    }

    actions
}

fn push_action(
    actions: &mut Vec<ActionPlanItem>,
    finding: &FindingMetadata,
    estimated_savings_kb: usize,
    difficulty: ActionDifficulty,
    recommended_fix: Option<RecommendedFix>,
) {
    let Some(finding_id) = finding.finding_id.clone() else {
        return;
    };

    actions.push(ActionPlanItem {
        action_priority: 0,
        finding_id,
        estimated_savings_kb,
        confidence: finding.confidence.unwrap_or(FindingConfidence::Low),
        difficulty,
        recommended_fix,
    });
}

fn recommended_fix(
    kind: &str,
    title: String,
    target_files: Vec<String>,
    package_name: &str,
) -> RecommendedFix {
    RecommendedFix {
        kind: kind.to_string(),
        title,
        target_files: normalized_files(target_files),
        replacement: replacement_candidate(kind, package_name),
    }
}

fn normalized_files(files: Vec<String>) -> Vec<String> {
    let mut files = files;
    files.sort();
    files.dedup();
    files
}

fn apply_action_metadata(
    finding: &mut FindingMetadata,
    action_by_id: &BTreeMap<String, ActionPlanItem>,
) {
    finding.action_priority = None;
    finding.recommended_fix = None;

    let Some(action) = finding
        .finding_id
        .as_ref()
        .and_then(|finding_id| action_by_id.get(finding_id))
    else {
        return;
    };

    finding.action_priority = Some(action.action_priority);
    finding.recommended_fix = action.recommended_fix.clone();
}

fn recommended_fix_kind(package_name: &str, recommendation: &str) -> &'static str {
    let normalized = recommendation.to_ascii_lowercase();

    if package_name == "moment" {
        return "replace-package";
    }

    if normalized.contains("server boundar") {
        return "move-boundary";
    }

    if normalized.contains("lazy load")
        || normalized.contains("on demand")
        || normalized.contains("defer")
    {
        return "lazy-load";
    }

    if normalized.contains("route")
        || normalized.contains("split ")
        || normalized.contains("split-")
    {
        return "split-route";
    }

    if normalized.contains("import")
        || normalized.contains("modular")
        || normalized.contains("register only")
    {
        return "narrow-import";
    }

    "reduce-usage"
}

fn replacement_candidate(kind: &str, package_name: &str) -> Option<String> {
    match (kind, package_name) {
        ("replace-package", "aws-sdk") => Some("AWS SDK v3".to_string()),
        ("replace-package", "moment") => Some("date-fns or Day.js".to_string()),
        ("narrow-import", "lodash") => Some("lodash-es".to_string()),
        _ => None,
    }
}
