use std::collections::{BTreeMap, BTreeSet};

use crate::{
    findings::{FindingConfidence, FindingMetadata},
    fix_hints::{
        dedupe_resolution_fix_hint, dynamic_import_fix_hint, route_split_fix_hint,
        subpath_import_fix_hint,
    },
    models::{
        ActionDifficulty, ActionPlanItem, Analysis, DuplicatePackage, HeavyDependency,
        LazyLoadCandidate, RecommendedFix, TreeShakingWarning,
    },
};

pub fn rank_actions(analysis: &Analysis) -> Vec<ActionPlanItem> {
    let mut actions = Vec::new();

    for item in &analysis.heavy_dependencies {
        push_action(
            &mut actions,
            &item.finding,
            item.estimated_kb,
            ActionDifficulty::Hard,
            heavy_dependency_fix(item),
        );
    }

    for item in &analysis.lazy_load_candidates {
        push_action(
            &mut actions,
            &item.finding,
            item.estimated_savings_kb,
            ActionDifficulty::Medium,
            lazy_load_fix(item),
        );
    }

    for item in &analysis.tree_shaking_warnings {
        push_action(
            &mut actions,
            &item.finding,
            item.estimated_kb,
            ActionDifficulty::Easy,
            tree_shaking_fix(item),
        );
    }

    for item in &analysis.duplicate_packages {
        push_action(
            &mut actions,
            &item.finding,
            item.estimated_extra_kb,
            ActionDifficulty::Medium,
            duplicate_package_fix(item),
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

fn heavy_dependency_fix(item: &HeavyDependency) -> Option<RecommendedFix> {
    match supported_heavy_dependency_fix_kind(&item.name, &item.recommendation) {
        Some(SupportedFixHintKind::DynamicImport) => dynamic_import_fix_hint(
            &item.finding,
            item.recommendation.clone(),
            item.imported_by.clone(),
        ),
        Some(SupportedFixHintKind::SubpathImport) => subpath_import_fix_hint(
            &item.finding,
            item.recommendation.clone(),
            item.imported_by.clone(),
            replacement_candidate("narrow-import", &item.name),
        ),
        Some(SupportedFixHintKind::RouteSplit) => route_split_fix_hint(
            &item.finding,
            item.recommendation.clone(),
            item.imported_by.clone(),
        ),
        None => None,
    }
}

fn lazy_load_fix(item: &LazyLoadCandidate) -> Option<RecommendedFix> {
    dynamic_import_fix_hint(
        &item.finding,
        item.recommendation.clone(),
        item.files.clone(),
    )
}

fn tree_shaking_fix(item: &TreeShakingWarning) -> Option<RecommendedFix> {
    subpath_import_fix_hint(
        &item.finding,
        item.recommendation.clone(),
        item.files.clone(),
        replacement_candidate("narrow-import", &item.package_name),
    )
}

fn duplicate_package_fix(item: &DuplicatePackage) -> Option<RecommendedFix> {
    dedupe_resolution_fix_hint(
        &item.finding,
        format!("Deduplicate {} to one installed version.", item.name),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupportedFixHintKind {
    DynamicImport,
    SubpathImport,
    RouteSplit,
}

fn supported_heavy_dependency_fix_kind(
    package_name: &str,
    recommendation: &str,
) -> Option<SupportedFixHintKind> {
    let normalized = recommendation.to_ascii_lowercase();

    if package_name == "moment" {
        return None;
    }

    if normalized.contains("server boundar") {
        return None;
    }

    if normalized.contains("lazy load")
        || normalized.contains("on demand")
        || normalized.contains("defer")
    {
        return Some(SupportedFixHintKind::DynamicImport);
    }

    if normalized.contains("route")
        || normalized.contains("split ")
        || normalized.contains("split-")
    {
        return Some(SupportedFixHintKind::RouteSplit);
    }

    if normalized.contains("import")
        || normalized.contains("modular")
        || normalized.contains("register only")
    {
        return Some(SupportedFixHintKind::SubpathImport);
    }

    None
}

fn replacement_candidate(kind: &str, package_name: &str) -> Option<String> {
    match (kind, package_name) {
        ("replace-package", "aws-sdk") => Some("AWS SDK v3".to_string()),
        ("replace-package", "moment") => Some("date-fns or Day.js".to_string()),
        ("narrow-import", "lodash") => Some("lodash-es".to_string()),
        _ => None,
    }
}
