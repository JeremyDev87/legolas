use legolas_core::Analysis;

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
            format!(
                "- {} ({} KB): {} {}.",
                item.name, item.estimated_kb, item.rationale, import_text
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
            format!(
                "- {}: {} ({} KB avoidable)",
                item.name,
                item.versions.join(", "),
                item.estimated_extra_kb
            )
        },
        "- none",
    );

    lines.push(String::new());
    lines.push("Lazy-load candidates:".to_string());
    append_section(
        &mut lines,
        &analysis.lazy_load_candidates,
        |item, _| {
            format!(
                "- {}: {}. Estimated win {} KB.",
                item.name, item.reason, item.estimated_savings_kb
            )
        },
        "- none",
    );

    lines.push(String::new());
    lines.push("Tree-shaking warnings:".to_string());
    append_section(
        &mut lines,
        &analysis.tree_shaking_warnings,
        |item, _| format!("- {}: {}", item.package_name, item.message),
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
        |item, index| format!("{}. {}", index + 1, item),
        "1. No high-confidence optimization candidates were found.",
    );
    lines.push(String::new());
    lines.push(format!(
        "Projected savings: ~{} KB, with {} confidence.",
        analysis.impact.potential_kb_saved, analysis.impact.confidence
    ));

    lines.join("\n")
}

#[derive(Clone)]
struct BarItem {
    label: String,
    value: usize,
}

fn build_actions(analysis: &Analysis) -> Vec<String> {
    let mut actions = Vec::new();

    for dependency in analysis.heavy_dependencies.iter().take(3) {
        if dependency.imported_by.is_empty() {
            actions.push(format!(
                "Remove or justify {}; it is declared but not imported in scanned source files.",
                dependency.name
            ));
            continue;
        }

        actions.push(format!(
            "Review {}: {}",
            dependency.name, dependency.recommendation
        ));
    }

    for duplicate in analysis.duplicate_packages.iter().take(3) {
        actions.push(format!(
            "Deduplicate {} versions ({}) to recover roughly {} KB.",
            duplicate.name,
            duplicate.versions.join(", "),
            duplicate.estimated_extra_kb
        ));
    }

    for candidate in analysis.lazy_load_candidates.iter().take(3) {
        let file = candidate.files.first().cloned().unwrap_or_default();
        actions.push(format!(
            "Lazy load {} in {} to target roughly {} KB of deferred code.",
            candidate.name, file, candidate.estimated_savings_kb
        ));
    }

    for warning in analysis.tree_shaking_warnings.iter().take(2) {
        actions.push(format!(
            "Clean up {} imports: {}",
            warning.package_name, warning.recommendation
        ));
    }

    dedupe(actions)
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

fn dedupe(items: Vec<String>) -> Vec<String> {
    let mut deduped = Vec::new();

    for item in items {
        if !deduped.contains(&item) {
            deduped.push(item);
        }
    }

    deduped
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
