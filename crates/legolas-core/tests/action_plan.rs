use legolas_core::{
    apply_action_plan, rank_actions, ActionDifficulty, Analysis, DuplicatePackage,
    FindingAnalysisSource, FindingConfidence, FindingMetadata, HeavyDependency, LazyLoadCandidate,
    RecommendedFix, TreeShakingWarning,
};
use serde_json::json;

#[test]
fn rank_actions_sorts_by_savings_confidence_and_difficulty() {
    let analysis = Analysis {
        heavy_dependencies: vec![heavy_dependency(
            "heavy-dependency:medium-hard",
            "chart.js",
            100,
            FindingConfidence::Medium,
        )],
        lazy_load_candidates: vec![lazy_load_candidate(
            "lazy-load:high-medium",
            "chart.js",
            100,
            FindingConfidence::High,
        )],
        tree_shaking_warnings: vec![tree_shaking_warning(
            "tree-shaking:high-easy",
            "lodash",
            100,
            FindingConfidence::High,
        )],
        duplicate_packages: vec![duplicate_package(
            "duplicate-package:low-medium",
            "react",
            150,
            FindingConfidence::Low,
        )],
        ..Default::default()
    };

    let actions = rank_actions(&analysis);

    assert_eq!(
        actions
            .iter()
            .map(|item| (
                item.action_priority,
                item.finding_id.as_str(),
                item.estimated_savings_kb,
                item.confidence,
                item.difficulty,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                1,
                "duplicate-package:low-medium",
                150,
                FindingConfidence::Low,
                ActionDifficulty::Medium,
            ),
            (
                2,
                "tree-shaking:high-easy",
                100,
                FindingConfidence::High,
                ActionDifficulty::Easy,
            ),
            (
                3,
                "lazy-load:high-medium",
                100,
                FindingConfidence::High,
                ActionDifficulty::Medium,
            ),
            (
                4,
                "heavy-dependency:medium-hard",
                100,
                FindingConfidence::Medium,
                ActionDifficulty::Hard,
            ),
        ]
    );
}

#[test]
fn apply_action_plan_writes_priority_and_recommended_fix_to_matching_findings() {
    let mut analysis = Analysis {
        heavy_dependencies: vec![heavy_dependency_with_recommendation(
            "heavy-dependency:lodash",
            "lodash",
            72,
            FindingConfidence::High,
            "Use per-method imports or switch to lodash-es when the toolchain supports it.",
        )],
        tree_shaking_warnings: vec![tree_shaking_warning(
            "tree-shaking:lodash-root-import",
            "lodash",
            26,
            FindingConfidence::High,
        )],
        ..Default::default()
    };

    let actions = apply_action_plan(&mut analysis);

    assert_eq!(
        actions
            .iter()
            .map(|item| (item.action_priority, item.finding_id.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (1, "heavy-dependency:lodash"),
            (2, "tree-shaking:lodash-root-import"),
        ]
    );
    assert_eq!(
        analysis.heavy_dependencies[0].finding.action_priority,
        Some(1)
    );
    assert_eq!(
        analysis.tree_shaking_warnings[0].finding.action_priority,
        Some(2)
    );

    let fix = analysis.heavy_dependencies[0]
        .finding
        .recommended_fix
        .as_ref()
        .expect("heavy dependency fix");
    assert_eq!(fix.kind, "narrow-import");
    assert_eq!(fix.target_files, vec!["src/App.tsx".to_string()]);
    assert_eq!(fix.replacement.as_deref(), Some("lodash-es"));

    let tree_fix = analysis.tree_shaking_warnings[0]
        .finding
        .recommended_fix
        .as_ref()
        .expect("tree shaking fix");
    assert_eq!(tree_fix.kind, "narrow-import");
    assert_eq!(tree_fix.replacement.as_deref(), Some("lodash-es"));
}

#[test]
fn rank_actions_maps_fix_kind_and_replacement_from_recommendation_shape() {
    let analysis = Analysis {
        heavy_dependencies: vec![
            heavy_dependency_with_recommendation(
                "heavy-dependency:chart.js",
                "chart.js",
                160,
                FindingConfidence::High,
                "Register only the chart primitives you use and lazy load dashboard surfaces.",
            ),
            heavy_dependency_with_recommendation(
                "heavy-dependency:react-icons",
                "react-icons",
                90,
                FindingConfidence::High,
                "Import narrowly from specific icon files or migrate to a more tree-shakable icon set.",
            ),
            heavy_dependency_with_recommendation(
                "heavy-dependency:lodash",
                "lodash",
                72,
                FindingConfidence::High,
                "Use per-method imports or switch to lodash-es when the toolchain supports it.",
            ),
            heavy_dependency_with_recommendation(
                "heavy-dependency:moment",
                "moment",
                67,
                FindingConfidence::High,
                "Prefer date-fns, Day.js, or the platform Intl APIs where practical.",
            ),
        ],
        ..Default::default()
    };

    let actions = rank_actions(&analysis);

    let chart = actions
        .iter()
        .find(|item| item.finding_id == "heavy-dependency:chart.js")
        .expect("chart.js action");
    assert_eq!(
        chart.recommended_fix.as_ref().map(|fix| fix.kind.as_str()),
        Some("lazy-load")
    );
    assert_eq!(
        chart
            .recommended_fix
            .as_ref()
            .and_then(|fix| fix.replacement.as_deref()),
        None
    );

    let react_icons = actions
        .iter()
        .find(|item| item.finding_id == "heavy-dependency:react-icons")
        .expect("react-icons action");
    assert_eq!(
        react_icons
            .recommended_fix
            .as_ref()
            .map(|fix| fix.kind.as_str()),
        Some("narrow-import")
    );
    assert_eq!(
        react_icons
            .recommended_fix
            .as_ref()
            .and_then(|fix| fix.replacement.as_deref()),
        None
    );

    let lodash = actions
        .iter()
        .find(|item| item.finding_id == "heavy-dependency:lodash")
        .expect("lodash action");
    assert_eq!(
        lodash.recommended_fix.as_ref().map(|fix| fix.kind.as_str()),
        Some("narrow-import")
    );
    assert_eq!(
        lodash
            .recommended_fix
            .as_ref()
            .and_then(|fix| fix.replacement.as_deref()),
        Some("lodash-es")
    );

    let moment = actions
        .iter()
        .find(|item| item.finding_id == "heavy-dependency:moment")
        .expect("moment action");
    assert_eq!(moment.recommended_fix, None);
}

#[test]
fn rank_actions_dedupes_by_finding_id_not_recommended_fix_text() {
    let analysis = Analysis {
        tree_shaking_warnings: vec![
            tree_shaking_warning(
                "tree-shaking:lodash-root-import",
                "lodash",
                26,
                FindingConfidence::High,
            ),
            tree_shaking_warning(
                "tree-shaking:lodash-root-import",
                "lodash",
                24,
                FindingConfidence::High,
            ),
        ],
        ..Default::default()
    };

    let actions = rank_actions(&analysis);

    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].finding_id, "tree-shaking:lodash-root-import");
    assert_eq!(actions[0].estimated_savings_kb, 26);
}

#[test]
fn apply_action_plan_only_exposes_safe_fix_hints_for_high_confidence_findings() {
    let mut analysis = Analysis {
        heavy_dependencies: vec![heavy_dependency_with_recommendation(
            "heavy-dependency:chart.js",
            "chart.js",
            160,
            FindingConfidence::Low,
            "Register only the chart primitives you use and lazy load dashboard surfaces.",
        )],
        lazy_load_candidates: vec![lazy_load_candidate(
            "lazy-load:chart.js",
            "chart.js",
            120,
            FindingConfidence::High,
        )],
        duplicate_packages: vec![duplicate_package(
            "duplicate-package:lodash",
            "lodash",
            18,
            FindingConfidence::High,
        )],
        ..Default::default()
    };

    apply_action_plan(&mut analysis);

    assert_eq!(analysis.heavy_dependencies[0].finding.recommended_fix, None);
    assert_eq!(
        analysis.lazy_load_candidates[0]
            .finding
            .recommended_fix
            .as_ref()
            .map(|fix| fix.kind.as_str()),
        Some("lazy-load")
    );
    assert_eq!(
        analysis.duplicate_packages[0]
            .finding
            .recommended_fix
            .as_ref()
            .map(|fix| fix.kind.as_str()),
        Some("dedupe-package")
    );
}

#[test]
fn apply_action_plan_clears_stale_metadata_before_reapplying() {
    let mut analysis = Analysis {
        tree_shaking_warnings: vec![tree_shaking_warning(
            "tree-shaking:lodash-root-import",
            "lodash",
            26,
            FindingConfidence::High,
        )],
        ..Default::default()
    };

    apply_action_plan(&mut analysis);
    assert_eq!(
        analysis.tree_shaking_warnings[0].finding.action_priority,
        Some(1)
    );
    assert!(analysis.tree_shaking_warnings[0]
        .finding
        .recommended_fix
        .is_some());

    analysis.tree_shaking_warnings[0].finding.finding_id = None;

    apply_action_plan(&mut analysis);

    assert_eq!(
        analysis.tree_shaking_warnings[0].finding.action_priority,
        None
    );
    assert_eq!(
        analysis.tree_shaking_warnings[0].finding.recommended_fix,
        None
    );
}

#[test]
fn finding_metadata_serializes_action_priority_and_recommended_fix_additively() {
    let metadata = FindingMetadata::new(
        "heavy-dependency:lodash",
        FindingAnalysisSource::SourceImport,
    )
    .with_confidence(FindingConfidence::High)
    .with_action_priority(1)
    .with_recommended_fix(RecommendedFix {
        kind: "narrow-import".to_string(),
        title: "Use per-method imports.".to_string(),
        target_files: vec!["src/App.tsx".to_string()],
        replacement: Some("lodash-es".to_string()),
    });

    let payload = serde_json::to_value(metadata).expect("serialize finding metadata");

    assert_eq!(
        payload,
        json!({
            "findingId": "heavy-dependency:lodash",
            "analysisSource": "source-import",
            "confidence": "high",
            "actionPriority": 1,
            "recommendedFix": {
                "kind": "narrow-import",
                "title": "Use per-method imports.",
                "targetFiles": ["src/App.tsx"],
                "replacement": "lodash-es"
            }
        })
    );
}

fn heavy_dependency(
    finding_id: &str,
    name: &str,
    estimated_kb: usize,
    confidence: FindingConfidence,
) -> HeavyDependency {
    heavy_dependency_with_recommendation(
        finding_id,
        name,
        estimated_kb,
        confidence,
        &format!("Reduce {name}."),
    )
}

fn heavy_dependency_with_recommendation(
    finding_id: &str,
    name: &str,
    estimated_kb: usize,
    confidence: FindingConfidence,
    recommendation: &str,
) -> HeavyDependency {
    HeavyDependency {
        name: name.to_string(),
        estimated_kb,
        recommendation: recommendation.to_string(),
        imported_by: vec!["src/App.tsx".to_string()],
        finding: finding(finding_id, confidence),
        ..Default::default()
    }
}

fn lazy_load_candidate(
    finding_id: &str,
    name: &str,
    estimated_savings_kb: usize,
    confidence: FindingConfidence,
) -> LazyLoadCandidate {
    LazyLoadCandidate {
        name: name.to_string(),
        estimated_savings_kb,
        recommendation: format!("Lazy load {name}."),
        files: vec!["src/Dashboard.tsx".to_string()],
        finding: finding(finding_id, confidence),
        ..Default::default()
    }
}

fn tree_shaking_warning(
    finding_id: &str,
    package_name: &str,
    estimated_kb: usize,
    confidence: FindingConfidence,
) -> TreeShakingWarning {
    TreeShakingWarning {
        package_name: package_name.to_string(),
        estimated_kb,
        recommendation: format!("Narrow {package_name} imports."),
        files: vec!["src/App.tsx".to_string()],
        finding: finding(finding_id, confidence),
        ..Default::default()
    }
}

fn duplicate_package(
    finding_id: &str,
    name: &str,
    estimated_extra_kb: usize,
    confidence: FindingConfidence,
) -> DuplicatePackage {
    DuplicatePackage {
        name: name.to_string(),
        estimated_extra_kb,
        finding: finding(finding_id, confidence),
        ..Default::default()
    }
}

fn finding(finding_id: &str, confidence: FindingConfidence) -> FindingMetadata {
    FindingMetadata::new(finding_id, FindingAnalysisSource::SourceImport)
        .with_confidence(confidence)
}
