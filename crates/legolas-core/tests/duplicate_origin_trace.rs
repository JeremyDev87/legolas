mod support;

use legolas_core::{
    lockfiles::{parse_duplicate_packages, DuplicateAnalysis},
    DuplicateOrigin, DuplicatePackage,
};

#[test]
fn parses_npm_origin_traces_from_package_paths() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/origins-npm");

    let analysis = parse_duplicate_packages(&fixture, "npm").expect("parse npm lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate_with_origins(
                "shared-lib",
                &["1.0.0", "2.0.0"],
                &[
                    origin("1.0.0", "app-shell", &["app-shell", "widget-core"]),
                    origin("2.0.0", "admin-shell", &["admin-shell"]),
                ],
            )],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_pnpm_origin_traces_from_dependency_graphs() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/origins-pnpm");

    let analysis = parse_duplicate_packages(&fixture, "pnpm@9.0.0").expect("parse pnpm lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate_with_origins(
                "shared-lib",
                &["1.0.0", "2.0.0"],
                &[
                    origin("1.0.0", "app-shell", &["app-shell", "widget-core"]),
                    origin("2.0.0", "admin-shell", &["admin-shell"]),
                ],
            )],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_pnpm_origin_traces_with_peer_scoped_variants() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/origins-pnpm-peer");

    let analysis = parse_duplicate_packages(&fixture, "pnpm@9.0.0").expect("parse pnpm lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate_with_origins(
                "shared-lib",
                &["1.0.0", "2.0.0"],
                &[
                    origin("1.0.0", "app-shell", &["app-shell", "widget-core"]),
                    origin("2.0.0", "admin-shell", &["admin-shell", "widget-core"]),
                ],
            )],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_yarn_origin_traces_from_dependency_graphs() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/origins-yarn");

    let analysis = parse_duplicate_packages(&fixture, "yarn@4.1.1").expect("parse yarn lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate_with_origins(
                "shared-lib",
                &["1.0.0", "2.0.0"],
                &[
                    origin("1.0.0", "app-shell", &["app-shell", "widget-core"]),
                    origin("2.0.0", "admin-shell", &["admin-shell"]),
                ],
            )],
            warnings: vec![],
        }
    );
}

#[test]
fn parses_yarn_origin_traces_with_peer_scoped_variants() {
    let fixture = support::fixture_path("tests/fixtures/lockfiles/origins-yarn-peer");

    let analysis = parse_duplicate_packages(&fixture, "yarn@4.1.1").expect("parse yarn lockfile");

    assert_eq!(
        analysis,
        DuplicateAnalysis {
            duplicates: vec![duplicate_with_origins(
                "shared-lib",
                &["1.0.0", "2.0.0"],
                &[
                    origin("1.0.0", "app-shell", &["app-shell", "widget-core"]),
                    origin("2.0.0", "admin-shell", &["admin-shell", "widget-core"]),
                ],
            )],
            warnings: vec![],
        }
    );
}

fn duplicate_with_origins(
    name: &str,
    versions: &[&str],
    origins: &[DuplicateOrigin],
) -> DuplicatePackage {
    DuplicatePackage {
        name: name.to_string(),
        versions: versions.iter().map(|value| (*value).to_string()).collect(),
        count: versions.len(),
        estimated_extra_kb: usize::max((versions.len().saturating_sub(1)) * 18, 18),
        origins: origins.to_vec(),
        finding: Default::default(),
    }
}

fn origin(version: &str, root_requester: &str, via_chain: &[&str]) -> DuplicateOrigin {
    DuplicateOrigin {
        version: version.to_string(),
        root_requester: root_requester.to_string(),
        via_chain: via_chain.iter().map(|value| (*value).to_string()).collect(),
    }
}
