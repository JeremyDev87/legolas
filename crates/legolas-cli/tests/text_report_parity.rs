mod support;

use legolas_cli::argv::ReportLanguage;
use legolas_cli::reporters::text::{
    format_budget_report_for_language, format_optimize_report, format_optimize_report_for_language,
    format_scan_report, format_scan_report_for_language, format_visualization_report,
    format_visualization_report_for_language,
};
use legolas_core::{
    budget::evaluate_budget, Analysis, DuplicateImpactScope, DuplicateOrigin, DuplicatePackage,
    FindingAnalysisSource, FindingConfidence, FindingEvidence, FindingMetadata, HeavyDependency,
    Impact, LazyLoadCandidate, Metadata, PackageSummary, SourceSummary,
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

    assert_report_matches_oracle(
        format_scan_report_for_language(&analysis, ReportLanguage::Ko),
        "basic-app/scan.txt",
    );
    assert_report_matches_oracle(
        format_visualization_report_for_language(&analysis, 10, ReportLanguage::Ko),
        "basic-app/visualize.txt",
    );
    assert_report_matches_oracle(
        format_optimize_report_for_language(&analysis, 5, ReportLanguage::Ko),
        "basic-app/optimize.txt",
    );
    assert_report_matches_oracle(
        format_budget_report_for_language(
            &analysis,
            &evaluate_budget(&analysis, None),
            ReportLanguage::Ko,
        ),
        "basic-app/budget.txt",
    );
}

#[test]
fn english_language_wrappers_keep_existing_output() {
    let analysis = load_analysis();
    let scan = format_scan_report_for_language(&analysis, ReportLanguage::En);

    assert_eq!(scan, format_scan_report(&analysis));
    assert!(scan.contains("Verdict: high impact"));
    assert!(scan.contains(
        "Confirmed initial payload savings: ~348 KB (estimated LCP improvement ~731 ms)"
    ));
    assert!(scan.contains("Directional cleanup opportunity: ~366 KB"));
    assert!(scan.contains("Top next actions:"));
    assert_eq!(
        format_visualization_report_for_language(&analysis, 10, ReportLanguage::En),
        format_visualization_report(&analysis, 10)
    );
    assert_eq!(
        format_optimize_report_for_language(&analysis, 5, ReportLanguage::En),
        format_optimize_report(&analysis, 5)
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
        "- chart.js [low confidence]: chart.js is statically imported in UI surfaces that usually tolerate lazy loading. Estimated win 120 KB."
    ));
    assert!(scan.contains(
        "  evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens."
    ));
    assert!(
        scan.contains("  evidence: src/Dashboard.tsx | specifier: lodash | root package import")
    );
    assert!(scan.contains("  origin: 4.17.20 via lodash"));
    assert!(scan.contains("  origin: 4.17.21 via lodash"));

    let optimize = format_optimize_report(&analysis, 5);
    assert!(optimize.contains(
        "1. Review chart.js upfront bundle weight [hard | high confidence | ~160 KB]\n   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces.\n   targets: src/Dashboard.tsx\n   evidence: src/Dashboard.tsx | specifier: chart.js | static import; Charting code is often only needed on a subset of screens."
    ));
    assert!(optimize.contains(
        "4. Review lodash upfront bundle weight [hard | high confidence | ~72 KB]\n   recommended fix: narrow-import - Use per-method imports or switch to lodash-es when the toolchain supports it.\n   targets: src/Dashboard.tsx\n   replacement: lodash-es\n   evidence: src/Dashboard.tsx | specifier: lodash | static import; Root lodash imports are a classic source of tree-shaking misses."
    ));
    assert!(!optimize.contains(
        "2. Lazy load chart.js [medium | low confidence | ~120 KB]\n   recommended fix:"
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
            "1. Review chart.js upfront bundle weight [hard | low confidence | ~160 KB]\n   evidence: src/Admin.tsx | specifier: chart.js | first evidence detail"
        )
    );
    assert!(!optimize.contains("recommended fix:"));
    assert!(!optimize.contains("second evidence detail"));
}

#[test]
fn scan_and_optimize_reports_render_all_evidence_lines_for_artifact_assisted_findings() {
    let mut analysis = base_analysis("artifact-evidence-app");
    analysis.heavy_dependencies = vec![HeavyDependency {
        name: "chart.js".to_string(),
        estimated_kb: 160,
        rationale: "Charting code is often only needed on a subset of screens.".to_string(),
        recommendation:
            "Register only the chart primitives you use and lazy load dashboard surfaces."
                .to_string(),
        imported_by: vec!["src/Admin.tsx".to_string()],
        finding: FindingMetadata::new(
            "heavy-dependency:chart.js",
            FindingAnalysisSource::ArtifactSource,
        )
        .with_confidence(FindingConfidence::High)
        .with_evidence([
            FindingEvidence::new("source-file")
                .with_file("src/Admin.tsx")
                .with_specifier("chart.js")
                .with_detail("source evidence detail"),
            FindingEvidence::new("artifact-chunk")
                .with_file("dist/admin.js")
                .with_specifier("chart.js")
                .with_detail("artifact chunk `admin` contributes 6200 bytes"),
        ]),
        ..HeavyDependency::default()
    }];

    let scan = format_scan_report(&analysis);
    assert!(
        scan.contains("  evidence: src/Admin.tsx | specifier: chart.js | source evidence detail")
    );
    assert!(scan.contains(
        "  evidence: dist/admin.js | specifier: chart.js | artifact chunk `admin` contributes 6200 bytes"
    ));

    let optimize = format_optimize_report(&analysis, 1);
    assert!(optimize
        .contains("   evidence: src/Admin.tsx | specifier: chart.js | source evidence detail"));
    assert!(optimize.contains(
        "   evidence: dist/admin.js | specifier: chart.js | artifact chunk `admin` contributes 6200 bytes"
    ));
}

#[test]
fn scan_report_renders_all_duplicate_origin_lines() {
    let mut analysis = base_analysis("duplicate-app");
    analysis.duplicate_packages = vec![DuplicatePackage {
        name: "lodash".to_string(),
        versions: vec![
            "4.17.19".to_string(),
            "4.17.20".to_string(),
            "4.17.21".to_string(),
        ],
        count: 3,
        estimated_extra_kb: 36,
        impact_scope: DuplicateImpactScope::Unknown,
        origins: vec![
            origin("4.17.19", "shell", &["shell", "shared"]),
            origin("4.17.20", "admin", &["admin"]),
            origin("4.17.21", "docs", &["docs", "shared"]),
        ],
        finding: FindingMetadata::new(
            "duplicate-package:lodash",
            FindingAnalysisSource::LockfileTrace,
        ),
    }];

    let scan = format_scan_report(&analysis);
    assert!(scan
        .contains("- lodash: 4.17.19, 4.17.20, 4.17.21 (36 KB directional cleanup opportunity)"));
    assert!(scan.contains("  origin: 4.17.19 via shell -> shared"));
    assert!(scan.contains("  origin: 4.17.20 via admin"));
    assert!(scan.contains("  origin: 4.17.21 via docs -> shared"));
}

#[test]
fn scan_report_renders_route_aware_lazy_load_file_in_summary() {
    let mut analysis = base_analysis("route-aware-report");
    analysis.lazy_load_candidates = vec![LazyLoadCandidate {
        name: "chart.js".to_string(),
        estimated_savings_kb: 128,
        recommendation: "Load it lazily.".to_string(),
        files: vec!["app/reports/page.tsx".to_string()],
        reason:
            "chart.js is statically imported in route-aware UI surfaces that usually tolerate lazy loading"
                .to_string(),
        finding: FindingMetadata::new("lazy-load:chart.js", FindingAnalysisSource::Heuristic)
            .with_confidence(legolas_core::FindingConfidence::Medium)
            .with_evidence([FindingEvidence::new("route-file")
                .with_file("app/reports/page.tsx")
                .with_specifier("chart.js")
                .with_detail("route context classified `route-page`")]),
    }];

    let scan = format_scan_report(&analysis);
    assert!(scan.contains(
        "- chart.js [medium confidence]: route surface app/reports/page.tsx statically imports chart.js and usually tolerates lazy loading. Estimated win 128 KB."
    ));
    assert!(scan.contains(
        "  evidence: app/reports/page.tsx | specifier: chart.js | route context classified `route-page`"
    ));
}

#[test]
fn scan_report_renders_all_route_aware_lazy_load_files_in_summary() {
    let mut analysis = base_analysis("route-aware-multi-report");
    analysis.lazy_load_candidates = vec![LazyLoadCandidate {
        name: "chart.js".to_string(),
        estimated_savings_kb: 128,
        recommendation: "Load it lazily.".to_string(),
        files: vec![
            "app/reports/page.tsx".to_string(),
            "app/settings/page.tsx".to_string(),
        ],
        reason:
            "chart.js is statically imported in route-aware UI surfaces that usually tolerate lazy loading"
                .to_string(),
        finding: FindingMetadata::new("lazy-load:chart.js", FindingAnalysisSource::Heuristic)
            .with_confidence(legolas_core::FindingConfidence::Medium)
            .with_evidence([
                FindingEvidence::new("route-file")
                    .with_file("app/reports/page.tsx")
                    .with_specifier("chart.js")
                    .with_detail("route context classified `route-page`"),
                FindingEvidence::new("route-file")
                    .with_file("app/settings/page.tsx")
                    .with_specifier("chart.js")
                    .with_detail("route context classified `route-page`"),
            ]),
    }];

    let scan = format_scan_report(&analysis);
    assert!(scan.contains(
        "- chart.js [medium confidence]: route surfaces app/reports/page.tsx, app/settings/page.tsx statically import chart.js and usually tolerate lazy loading. Estimated win 128 KB."
    ));
    assert!(scan.contains(
        "  evidence: app/reports/page.tsx | specifier: chart.js | route context classified `route-page`"
    ));
    assert!(!scan.contains("app/settings/page.tsx | specifier: chart.js"));
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
            "Verdict: limited confirmed impact\n",
            "Confirmed initial payload savings: not confirmed (duplicate dependency pressure alone does not confirm initial payload or LCP savings)\n",
            "Directional cleanup opportunity: ~0 KB\n",
            "\n",
            "Top next actions:\n",
            "1. No high-confidence optimization candidates were found.\n",
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
fn korean_scan_report_does_not_confirm_lcp_savings_for_duplicate_only_pressure() {
    let mut analysis = base_analysis("duplicate-only-app");
    analysis.duplicate_packages = vec![DuplicatePackage {
        name: "vitest".to_string(),
        versions: vec!["1.6.0".to_string(), "2.0.0".to_string()],
        count: 2,
        estimated_extra_kb: 48,
        impact_scope: DuplicateImpactScope::Unknown,
        finding: FindingMetadata::new(
            "duplicate-package:vitest",
            FindingAnalysisSource::LockfileTrace,
        )
        .with_confidence(FindingConfidence::Medium),
        ..DuplicatePackage::default()
    }];

    let report = format_scan_report_for_language(&analysis, ReportLanguage::Ko);

    assert!(report.contains("판정:"));
    assert!(report.contains("확정 초기 페이로드 절감: 미확정"));
    assert!(!report.contains("LCP 약"));
    assert!(report.contains("방향성 정리 여지: 약 48 KB"));
    assert!(report.contains("Top next actions:"));
}

#[test]
fn korean_scan_report_renders_dev_only_duplicates_as_dependency_hygiene() {
    let mut analysis = base_analysis("dev-duplicate-app");
    analysis.duplicate_packages = vec![DuplicatePackage {
        name: "vitest".to_string(),
        versions: vec!["1.6.0".to_string(), "2.0.0".to_string()],
        count: 2,
        estimated_extra_kb: 48,
        impact_scope: DuplicateImpactScope::DevOnly,
        finding: FindingMetadata::new(
            "duplicate-package:vitest",
            FindingAnalysisSource::LockfileTrace,
        )
        .with_confidence(FindingConfidence::Medium),
        ..DuplicatePackage::default()
    }];

    let report = format_scan_report_for_language(&analysis, ReportLanguage::Ko);

    assert!(report.contains("vitest 개발/테스트 의존성 중복 정리"));
    assert!(report.contains("개발/테스트 의존성 중복 정리"));
    assert!(report.contains("dependency hygiene"));
    assert!(report.contains("확정 초기 페이로드 절감: 미확정"));
}

#[test]
fn english_scan_report_renders_dev_only_duplicates_as_dependency_hygiene() {
    let mut analysis = base_analysis("dev-duplicate-app");
    analysis.duplicate_packages = vec![DuplicatePackage {
        name: "vitest".to_string(),
        versions: vec!["1.6.0".to_string(), "2.0.0".to_string()],
        count: 2,
        estimated_extra_kb: 48,
        impact_scope: DuplicateImpactScope::DevOnly,
        finding: FindingMetadata::new(
            "duplicate-package:vitest",
            FindingAnalysisSource::LockfileTrace,
        )
        .with_confidence(FindingConfidence::Medium),
        ..DuplicatePackage::default()
    }];

    let report = format_scan_report_for_language(&analysis, ReportLanguage::En);

    assert!(report.contains("development/test dependency duplication"));
    assert!(report.contains("dependency hygiene"));
    assert!(!report.contains("48 KB avoidable"));
    assert!(report.contains("Confirmed initial payload savings: not confirmed"));
}

#[test]
fn scan_report_confirms_production_likely_duplicate_savings_without_other_findings() {
    let mut analysis = base_analysis("production-duplicate-app");
    analysis.duplicate_packages = vec![DuplicatePackage {
        name: "lodash".to_string(),
        versions: vec!["4.17.20".to_string(), "4.17.21".to_string()],
        count: 2,
        estimated_extra_kb: 48,
        impact_scope: DuplicateImpactScope::ProductionLikely,
        finding: FindingMetadata::new(
            "duplicate-package:lodash",
            FindingAnalysisSource::LockfileTrace,
        )
        .with_confidence(FindingConfidence::High),
        ..DuplicatePackage::default()
    }];

    let ko_report = format_scan_report_for_language(&analysis, ReportLanguage::Ko);
    let en_report = format_scan_report_for_language(&analysis, ReportLanguage::En);

    assert!(ko_report.contains("확정 초기 페이로드 절감: 미확정"));
    assert!(!ko_report.contains("확정 초기 페이로드 절감: 약 48 KB"));
    assert!(ko_report.contains("초기 페이로드 절감 후보"));
    assert!(en_report.contains("Confirmed initial payload savings: not confirmed"));
    assert!(!en_report.contains("Confirmed initial payload savings: ~48 KB"));
    assert!(en_report.contains("initial payload candidate"));
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

fn origin(version: &str, root_requester: &str, via_chain: &[&str]) -> DuplicateOrigin {
    DuplicateOrigin {
        version: version.to_string(),
        root_requester: root_requester.to_string(),
        via_chain: via_chain.iter().map(|value| (*value).to_string()).collect(),
    }
}
