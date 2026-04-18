use crate::models::{
    DuplicatePackage, HeavyDependency, Impact, LazyLoadCandidate, TreeShakingWarning,
};

pub fn estimate_impact(
    heavy_dependencies: &[HeavyDependency],
    duplicate_packages: &[DuplicatePackage],
    lazy_load_candidates: &[LazyLoadCandidate],
    tree_shaking_warnings: &[TreeShakingWarning],
) -> Impact {
    let heavy_kb: f64 = heavy_dependencies
        .iter()
        .take(5)
        .map(|item| item.estimated_kb as f64 * 0.18)
        .sum();
    let duplicate_kb: usize = duplicate_packages
        .iter()
        .map(|item| item.estimated_extra_kb)
        .sum();
    let lazy_kb: usize = lazy_load_candidates
        .iter()
        .map(|item| item.estimated_savings_kb)
        .sum();
    let shaking_kb: usize = tree_shaking_warnings
        .iter()
        .map(|item| item.estimated_kb)
        .sum();

    let potential_kb_saved =
        (heavy_kb + duplicate_kb as f64 + lazy_kb as f64 + shaking_kb as f64).round() as usize;
    let estimated_lcp_improvement_ms = (potential_kb_saved as f64 * 2.1).round() as usize;

    Impact {
        potential_kb_saved,
        estimated_lcp_improvement_ms,
        confidence: if potential_kb_saved > 0 {
            "directional".to_string()
        } else {
            "low".to_string()
        },
        summary: summarize_impact(potential_kb_saved).to_string(),
    }
}

fn summarize_impact(potential_kb_saved: usize) -> &'static str {
    if potential_kb_saved >= 300 {
        return "High impact: the project has clear opportunities to reduce initial payload size.";
    }

    if potential_kb_saved >= 120 {
        return "Medium impact: there are several meaningful bundle wins available.";
    }

    if potential_kb_saved >= 40 {
        return "Targeted impact: a handful of focused optimizations should pay off.";
    }

    "Low impact: obvious bundle issues are limited in the current scan."
}
