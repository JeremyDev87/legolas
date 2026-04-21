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
