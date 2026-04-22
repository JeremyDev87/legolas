mod support;

use std::fs;

use legolas_core::{
    analyze_project,
    project_shape::detect_frameworks,
    route_context::{classify_route_context, RouteContextKind},
    FindingConfidence,
};
use serde_json::Value;
use tempfile::tempdir;

#[test]
fn classify_route_context_distinguishes_next_route_pages_from_shared_components() {
    let project_root = support::fixture_path("tests/fixtures/routes/next-app");
    let frameworks = load_frameworks(&project_root);

    assert_eq!(
        classify_route_context(
            &project_root,
            &frameworks,
            &project_root.join("app/dashboard/page.tsx")
        ),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(
            &project_root,
            &frameworks,
            &project_root.join("components/ChartPanel.tsx"),
        ),
        RouteContextKind::SharedComponent
    );
}

#[test]
fn classify_route_context_marks_next_layout_and_admin_surfaces() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "next-admin-app",
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "app/layout.tsx",
        "export default function Layout() { return null; }\n",
    );
    write_file(
        root,
        "app/admin/page.tsx",
        "export default function Page() { return null; }\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("app/layout.tsx")),
        RouteContextKind::RouteLayout
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("app/admin/page.tsx")),
        RouteContextKind::AdminSurface
    );
}

#[test]
fn classify_route_context_reads_next_special_route_files() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "next-special-files-app",
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "src/app/dashboard/loading.tsx",
        "export default function Loading() { return null; }\n",
    );
    write_file(
        root,
        "src/app/dashboard/error.tsx",
        "export default function Error() { return null; }\n",
    );
    write_file(
        root,
        "src/app/dashboard/template.tsx",
        "export default function Template() { return null; }\n",
    );
    write_file(
        root,
        "src/app/dashboard/not-found.tsx",
        "export default function NotFound() { return null; }\n",
    );
    write_file(
        root,
        "src/app/dashboard/default.tsx",
        "export default function Default() { return null; }\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/app/dashboard/loading.tsx")
        ),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/app/dashboard/error.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/app/dashboard/template.tsx")
        ),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/app/dashboard/not-found.tsx")
        ),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/app/dashboard/default.tsx")
        ),
        RouteContextKind::RoutePage
    );
}

#[test]
fn classify_route_context_reads_generic_pages_and_non_route_files() {
    let project_root = support::fixture_path("tests/fixtures/routes/vite-pages");
    let frameworks = load_frameworks(&project_root);

    assert_eq!(
        classify_route_context(
            &project_root,
            &frameworks,
            &project_root.join("src/pages/Settings.tsx"),
        ),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(
            &project_root,
            &frameworks,
            &project_root.join("src/lib/date.ts"),
        ),
        RouteContextKind::NonRoute
    );
}

#[test]
fn classify_route_context_keeps_generic_route_segments_even_when_names_overlap_support_dirs() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "generic-route-support-app",
  "dependencies": {
    "vite": "^6.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "src/pages/utils/date.ts",
        "export function formatDate() { return \"2026-04-21\"; }\n",
    );
    write_file(
        root,
        "routes/lib/helper.ts",
        "export const routeHelper = true;\n",
    );
    write_file(
        root,
        "src/pages/dashboard/index.tsx",
        "export default function DashboardPage() { return null; }\n",
    );
    write_file(
        root,
        "src/routes/lib.tsx",
        "export default function LibRoute() { return null; }\n",
    );
    write_file(
        root,
        "pages/components.tsx",
        "export default function ComponentsRoute() { return null; }\n",
    );
    write_file(
        root,
        "pages/components/index.tsx",
        "export default function ComponentsIndexRoute() { return null; }\n",
    );
    write_file(
        root,
        "src/pages/shared/index.tsx",
        "export default function SharedIndexRoute() { return null; }\n",
    );
    write_file(
        root,
        "pages/shared/Button.tsx",
        "export default function SharedButtonRoute() { return null; }\n",
    );
    write_file(
        root,
        "src/pages/components/ChartPanel.tsx",
        "export default function ComponentsChartRoute() { return null; }\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/pages/utils/date.ts")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("routes/lib/helper.ts")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/pages/dashboard/index.tsx")
        ),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/routes/lib.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("pages/components.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("pages/components/index.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/pages/shared/index.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("pages/shared/Button.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/pages/components/ChartPanel.tsx")
        ),
        RouteContextKind::RoutePage
    );
}

#[test]
fn classify_route_context_reads_src_app_and_root_routes_layouts() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "route-shapes-app",
  "dependencies": {
    "next": "^15.0.0",
    "astro": "^5.0.0"
  }
}"#,
    );
    write_file(
        root,
        "src/app/reports/page.tsx",
        "export default function ReportsPage() { return null; }\n",
    );
    write_file(
        root,
        "pages/index.tsx",
        "export default function HomePage() { return null; }\n",
    );
    write_file(
        root,
        "routes/admin.tsx",
        "export default function AdminPage() { return null; }\n",
    );
    write_file(
        root,
        "routes/ops.mtsx",
        "export default function OpsPage() { return null; }\n",
    );
    write_file(
        root,
        "src/pages/blog.astro",
        "---\nconst title = \"blog\";\n---\n<h1>{title}</h1>\n",
    );
    write_file(
        root,
        "pages/_app.tsx",
        "export default function App() { return null; }\n",
    );
    write_file(
        root,
        "pages/api/health.cts",
        "export default function handler() { return null; }\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/app/reports/page.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("pages/index.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("routes/admin.tsx")),
        RouteContextKind::AdminSurface
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("routes/ops.mtsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/pages/blog.astro")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("pages/_app.tsx")),
        RouteContextKind::NonRoute
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("pages/api/health.cts")),
        RouteContextKind::NonRoute
    );
}

#[test]
fn classify_route_context_prefers_shared_component_over_admin_filename() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "shared-admin-app",
  "dependencies": {
    "vite": "^6.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "src/components/AdminPanel.tsx",
        "export function AdminPanel() { return null; }\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/components/AdminPanel.tsx")
        ),
        RouteContextKind::SharedComponent
    );
}

#[test]
fn classify_route_context_detects_collocated_shared_components_inside_route_trees() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "collocated-components-app",
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "app/dashboard/components/ChartPanel.tsx",
        "export function ChartPanel() { return null; }\n",
    );
    write_file(
        root,
        "src/pages/dashboard/components/ChartPanel.tsx",
        "export function DashboardChartPanel() { return null; }\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("app/dashboard/components/ChartPanel.tsx")
        ),
        RouteContextKind::SharedComponent
    );
    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/pages/dashboard/components/ChartPanel.tsx")
        ),
        RouteContextKind::SharedComponent
    );
}

#[test]
fn classify_route_context_does_not_treat_partial_admin_names_as_admin_surfaces() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "admin-name-heuristic-app",
  "dependencies": {
    "vite": "^6.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "src/pages/administrator.tsx",
        "export default function AdministratorPage() { return null; }\n",
    );
    write_file(
        root,
        "src/components/MyAdminPanel.tsx",
        "export function MyAdminPanel() { return null; }\n",
    );
    write_file(
        root,
        "src/pages/admin.tsx",
        "export default function AdminPage() { return null; }\n",
    );
    write_file(
        root,
        "src/lib/admin.tsx",
        "export const adminHelper = true;\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/pages/administrator.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(
            root,
            &frameworks,
            &root.join("src/components/MyAdminPanel.tsx")
        ),
        RouteContextKind::SharedComponent
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/pages/admin.tsx")),
        RouteContextKind::AdminSurface
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/lib/admin.tsx")),
        RouteContextKind::NonRoute
    );
}

#[test]
fn classify_route_context_does_not_treat_route_segments_named_components_or_shared_as_shared_components(
) {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "route-segment-name-app",
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "app/components/page.tsx",
        "export default function ComponentsPage() { return null; }\n",
    );
    write_file(
        root,
        "app/shared/page.tsx",
        "export default function SharedPage() { return null; }\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("app/components/page.tsx")),
        RouteContextKind::RoutePage
    );
    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("app/shared/page.tsx")),
        RouteContextKind::RoutePage
    );
}

#[test]
fn analyze_project_uses_route_context_for_lazy_load_candidates_without_keyword_matches() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "route-aware-candidate-app",
  "dependencies": {
    "chart.js": "^4.4.1",
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "app/reports/page.tsx",
        "import { Chart } from \"chart.js\";\nexport default function ReportsPage() { return Chart; }\n",
    );

    let analysis = analyze_project(root).expect("analyze route-aware candidate project");
    let candidate = analysis
        .lazy_load_candidates
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js lazy-load candidate");

    assert_eq!(candidate.files, vec!["app/reports/page.tsx".to_string()]);
    assert_eq!(
        candidate.reason,
        "chart.js is statically imported in route-aware UI surfaces that usually tolerate lazy loading"
    );
    assert_eq!(candidate.estimated_savings_kb, 128);
    assert_eq!(
        candidate.finding.confidence,
        Some(FindingConfidence::Medium)
    );
    let evidence = candidate
        .finding
        .evidence
        .first()
        .expect("lazy-load evidence");
    assert_eq!(evidence.file.as_deref(), Some("app/reports/page.tsx"));
    assert_eq!(evidence.specifier.as_deref(), Some("chart.js"));
    assert_eq!(
        evidence.detail.as_deref(),
        Some("route context classified `route-page`")
    );
}

#[test]
fn analyze_project_skips_candidates_when_shared_component_keeps_static_import_alive() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "shared-import-route-app",
  "dependencies": {
    "chart.js": "^4.4.1",
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "app/reports/page.tsx",
        "import { Chart } from \"chart.js\";\nexport default function ReportsPage() { return Chart; }\n",
    );
    write_file(
        root,
        "src/components/ChartPanel.tsx",
        "import { Chart } from \"chart.js\";\nexport function ChartPanel() { return Chart; }\n",
    );

    let analysis = analyze_project(root).expect("analyze shared-import route project");

    assert!(
        analysis
            .lazy_load_candidates
            .iter()
            .all(|item| item.name != "chart.js"),
        "shared component static import should suppress route-aware chart.js candidate"
    );
}

#[test]
fn analyze_project_keeps_index_routes_inside_support_named_segments_as_candidates() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "support-named-route-app",
  "dependencies": {
    "chart.js": "^4.4.1",
    "vite": "^6.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "pages/components/index.tsx",
        "import { Chart } from \"chart.js\";\nexport default function ComponentsPage() { return Chart; }\n",
    );

    let analysis = analyze_project(root).expect("analyze support-named route project");
    let candidate = analysis
        .lazy_load_candidates
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js lazy-load candidate");

    assert_eq!(
        candidate.files,
        vec!["pages/components/index.tsx".to_string()]
    );
    let evidence = candidate
        .finding
        .evidence
        .first()
        .expect("lazy-load evidence");
    assert_eq!(evidence.file.as_deref(), Some("pages/components/index.tsx"));
    assert_eq!(
        evidence.detail.as_deref(),
        Some("route context classified `route-page`")
    );
}

#[test]
fn analyze_project_keeps_direct_route_files_inside_support_named_segments_as_candidates() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "support-named-direct-route-app",
  "dependencies": {
    "chart.js": "^4.4.1",
    "vite": "^6.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "pages/shared/Button.tsx",
        "import { Chart } from \"chart.js\";\nexport default function SharedButtonPage() { return Chart; }\n",
    );

    let analysis = analyze_project(root).expect("analyze direct support-named route project");
    let candidate = analysis
        .lazy_load_candidates
        .iter()
        .find(|item| item.name == "chart.js")
        .expect("chart.js lazy-load candidate");

    assert_eq!(candidate.files, vec!["pages/shared/Button.tsx".to_string()]);
    assert_eq!(
        candidate.reason,
        "chart.js is statically imported in route-aware UI surfaces that usually tolerate lazy loading"
    );
    assert_eq!(candidate.estimated_savings_kb, 128);
    let evidence = candidate
        .finding
        .evidence
        .first()
        .expect("lazy-load evidence");
    assert_eq!(evidence.file.as_deref(), Some("pages/shared/Button.tsx"));
    assert_eq!(
        evidence.detail.as_deref(),
        Some("route context classified `route-page`")
    );
}

#[test]
fn classify_route_context_does_not_treat_next_non_entry_admin_named_file_as_admin_surface() {
    let temp = tempdir().expect("create temp dir");
    let root = temp.path();

    write_file(
        root,
        "package.json",
        r#"{
  "name": "next-non-entry-admin-app",
  "dependencies": {
    "next": "^15.0.0",
    "react": "^19.0.0"
  }
}"#,
    );
    write_file(
        root,
        "src/app/admin.tsx",
        "export const adminHelper = true;\n",
    );

    let frameworks = load_frameworks(root);

    assert_eq!(
        classify_route_context(root, &frameworks, &root.join("src/app/admin.tsx")),
        RouteContextKind::NonRoute
    );
}

fn load_frameworks(project_root: &std::path::Path) -> Vec<String> {
    let manifest =
        fs::read_to_string(project_root.join("package.json")).expect("read package.json");
    let manifest: Value = serde_json::from_str(&manifest).expect("parse package.json");
    detect_frameworks(project_root, &manifest).expect("detect frameworks")
}

fn write_file(root: &std::path::Path, relative_path: &str, contents: &str) {
    let target = root.join(relative_path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(target, contents).expect("write file");
}
