mod support;

use legolas_core::analyze_project;

#[test]
fn analyze_project_matches_the_parity_oracle() {
    let analysis = analyze_project(support::fixture_path("tests/fixtures/parity/basic-app"))
        .expect("analyze parity fixture");
    let actual = support::normalize_analysis_for_oracle(&analysis);
    let expected = support::read_oracle("basic-app/scan.json");

    assert_eq!(actual, expected);
}
