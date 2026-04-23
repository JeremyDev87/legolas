mod support;

use legolas_cli::reporters::text::format_optimize_report;
use legolas_core::Analysis;

fn load_analysis() -> Analysis {
    serde_json::from_str(&support::read_oracle("basic-app/scan.json")).expect("parse analysis")
}

#[test]
fn optimize_report_matches_ranked_action_oracle() {
    let analysis = load_analysis();

    assert_eq!(
        format!("{}\n", format_optimize_report(&analysis, 5)),
        support::read_oracle("basic-app/optimize-ranked.txt")
    );
}

#[test]
fn optimize_report_only_renders_safe_fix_hints_for_high_confidence_actions() {
    let analysis = load_analysis();
    let report = format_optimize_report(&analysis, 5);

    assert!(report.contains(
        "1. Review chart.js upfront bundle weight [hard | high confidence | ~160 KB]\n   recommended fix: lazy-load - Register only the chart primitives you use and lazy load dashboard surfaces."
    ));
    assert!(report.contains(
        "4. Review lodash upfront bundle weight [hard | high confidence | ~72 KB]\n   recommended fix: narrow-import - Use per-method imports or switch to lodash-es when the toolchain supports it."
    ));
    assert!(!report.contains(
        "2. Lazy load chart.js [medium | low confidence | ~120 KB]\n   recommended fix:"
    ));
    assert!(!report.contains(
        "5. Lazy load react-icons [medium | low confidence | ~68 KB]\n   recommended fix:"
    ));
}
