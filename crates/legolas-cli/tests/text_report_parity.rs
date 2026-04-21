mod support;

use legolas_cli::reporters::text::{
    format_optimize_report, format_scan_report, format_visualization_report,
};
use legolas_core::{
    Analysis, DuplicatePackage, FindingAnalysisSource, FindingEvidence, FindingMetadata,
    HeavyDependency, Impact, LazyLoadCandidate, Metadata, PackageSummary, SourceSummary,
};

fn load_analysis() -> Analysis {
    serde_json::from_str(&support::read_oracle("basic-app/scan.json")).expect("parse analysis")
}

fn assert_report_matches_oracle(actual: String, oracle: &str) {
    assert_eq!(format!("{actual}\n"), support::read_oracle(oracle));
}

#[test]
fn matches_scan_visualize_and_optimize_oracles() {
    let analysis = load_analysis();

    assert_report_matches_oracle(format_scan_report(&analysis), "basic-app/scan.txt");
    assert_report_matches_oracle(
        format_visualization_report(&analysis, 10),
        "basic-app/visualize.txt",
    );
    assert_report_matches_oracle(
        format_optimize_report(&analysis, 5),
        "basic-app/optimize.txt",
    );
}

#[test]
fn scan_and_optimize_reports_render_compact_evidence_lines() {
    let analysis = load_analysis();

    let scan = format_scan_report(&analysis);
    assert!(scan.contains(
        "- chart.js (160 KB) [high confidence]: Charting code is often only needed on a subset of screens. imported in 1 file(s)."
    ));
    assert!(scan.contains(
        "- chart.js [medium confidence]: chart.js is statically imported in UI surfaces that usually tolerate lazy loading. Estimated win 120 KB."
    ));
    assert!(scan.contains(
        "  evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens."
    ));
    assert!(
        scan.contains("  evidence: src/Dashboard.tsx | specifier: lodash | root package import")
    );

    let optimize = format_optimize_report(&analysis, 5);
    assert!(optimize.contains(
        "1. Review chart.js upfront bundle weight [hard | high confidence | ~160 KB]\n   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces.\n   targets: src/Dashboard.tsx\n   evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens."
    ));
    assert!(optimize.contains(
        "4. Review lodash upfront bundle weight [hard | high confidence | ~72 KB]\n   recommended fix: narrow-import - Use per-method imports or switch to lodash-es when the toolchain supports it.\n   targets: src/Dashboard.tsx\n   replacement: lodash-es\n   evidence: src/Dashboard.tsx | specifier: lodash | static import; Root lodash imports are a classic source of tree-shaking misses."
    ));
}

#[test]
fn scan_and_optimize_reports_only_render_the_first_evidence_line_per_finding() {
    let mut analysis = base_analysis("multi-evidence-app");
    analysis.heavy_dependencies = vec![HeavyDependency {
        name: "chart.js".to_string(),
        estimated_kb: 160,
        rationale: "Charting code is often only needed on a subset of screens.".to_string(),
        recommendation:
            "Register only the chart primitives you use and lazy load dashboard surfaces."
                .to_string(),
        imported_by: vec!["src/Admin.tsx".to_string(), "src/Reports.tsx".to_string()],
        finding: FindingMetadata::new(
            "heavy-dependency:chart.js",
            FindingAnalysisSource::SourceImport,
        )
        .with_evidence([
            FindingEvidence::new("source-file")
                .with_file("src/Admin.tsx")
                .with_specifier("chart.js")
                .with_detail("first evidence detail"),
            FindingEvidence::new("source-file")
                .with_file("src/Reports.tsx")
                .with_specifier("chart.js")
                .with_detail("second evidence detail"),
        ]),
        ..HeavyDependency::default()
    }];

    let scan = format_scan_report(&analysis);
    assert!(
        scan.contains("  evidence: src/Admin.tsx | specifier: chart.js | first evidence detail")
    );
    assert!(!scan.contains("second evidence detail"));

    let optimize = format_optimize_report(&analysis, 1);
    assert!(
        optimize.contains(
            "1. Review chart.js upfront bundle weight [hard | low confidence | ~160 KB]\n   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces.\n   targets: src/Admin.tsx, src/Reports.tsx\n   evidence: src/Admin.tsx | specifier: chart.js | first evidence detail"
        )
    );
    assert!(!optimize.contains("second evidence detail"));
}

#[test]
fn scan_report_covers_empty_section_fallbacks() {
    let analysis = base_analysis("empty-app");

    assert_eq!(
        format_scan_report(&analysis),
        concat!(
            "Legolas scan for empty-app\n",
            "Project root: <PROJECT_ROOT>\n",
            "Mode: heuristic\n",
            "Frameworks: none detected\n",
            "Package manager: npm\n",
            "Scanned 0 source files and 0 imported packages\n",
            "\n",
            "Potential payload reduction: ~0 KB\n",
            "Estimated LCP improvement: ~0 ms\n",
            "Low impact: obvious bundle issues are limited in the current scan.\n",
            "\n",
            "Heaviest known dependencies:\n",
            "- none\n",
            "\n",
            "Duplicate package versions:\n",
            "- none\n",
            "\n",
            "Lazy-load candidates:\n",
            "- none\n",
            "\n",
            "Tree-shaking warnings:\n",
            "- none\n",
            "\n",
            "Unused dependency candidates:\n",
            "- none"
        )
    );
}

#[test]
fn optimize_and_visualize_reports_clamp_zero_limits_and_cover_lazy_load_fallback() {
    let empty_analysis = base_analysis("empty-app");
    assert_eq!(
        format_visualization_report(&empty_analysis, 0),
        format_visualization_report(&empty_analysis, 1)
    );

    let mut visualize_analysis = base_analysis("visualize-app");
    visualize_analysis.heavy_dependencies = vec![HeavyDependency {
        name: "react-icons".to_string(),
        estimated_kb: 90,
        ..HeavyDependency::default()
    }];
    visualize_analysis.duplicate_packages = vec![DuplicatePackage {
        name: "react".to_string(),
        versions: vec!["18.2.0".to_string(), "18.3.1".to_string()],
        estimated_extra_kb: 20,
        ..DuplicatePackage::default()
    }];
    let zero_limit_visualize = format_visualization_report(&visualize_analysis, 0);
    assert_eq!(
        zero_limit_visualize,
        format_visualization_report(&visualize_analysis, 1)
    );
    assert!(zero_limit_visualize.contains("react-icons"));
    assert!(zero_limit_visualize.contains("react"));

    let mut optimize_analysis = base_analysis("edge-app");
    optimize_analysis.impact = Impact {
        potential_kb_saved: 42,
        estimated_lcp_improvement_ms: 88,
        confidence: "directional".to_string(),
        summary: "Targeted impact: a handful of focused optimizations should pay off.".to_string(),
    };
    optimize_analysis.lazy_load_candidates = vec![LazyLoadCandidate {
        name: "chart.js".to_string(),
        estimated_savings_kb: 68,
        recommendation: "Load it lazily.".to_string(),
        files: Vec::new(),
        reason: "chart.js is statically imported in UI surfaces that usually tolerate lazy loading"
            .to_string(),
        ..LazyLoadCandidate::default()
    }];

    assert_eq!(
        format_optimize_report(&optimize_analysis, 0),
        concat!(
            "Legolas optimize for edge-app\n",
            "\n",
            "1. Lazy load chart.js in undefined to target roughly 68 KB of deferred code.\n",
            "\n",
            "Projected savings: ~42 KB, with directional confidence."
        )
    );
    assert_eq!(
        format_optimize_report(&optimize_analysis, 0),
        format_optimize_report(&optimize_analysis, 1)
    );
}

fn base_analysis(name: &str) -> Analysis {
    Analysis {
        project_root: "<PROJECT_ROOT>".to_string(),
        package_manager: "npm".to_string(),
        package_summary: PackageSummary {
            name: name.to_string(),
            ..PackageSummary::default()
        },
        source_summary: SourceSummary::default(),
        impact: Impact {
            potential_kb_saved: 0,
            estimated_lcp_improvement_ms: 0,
            confidence: "low".to_string(),
            summary: "Low impact: obvious bundle issues are limited in the current scan."
                .to_string(),
        },
        metadata: Metadata {
            mode: "heuristic".to_string(),
            generated_at: "<GENERATED_AT>".to_string(),
        },
        ..Analysis::default()
    }
}
