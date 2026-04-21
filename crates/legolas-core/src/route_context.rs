use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteContextKind {
    RoutePage,
    RouteLayout,
    AdminSurface,
    SharedComponent,
    NonRoute,
}

pub fn classify_route_context(
    project_root: &Path,
    frameworks: &[String],
    path: &Path,
) -> RouteContextKind {
    let normalized = normalize_relative_path(project_root, path);
    let lower = normalized.to_ascii_lowercase();

    if is_shared_component_path(&lower, frameworks) {
        return RouteContextKind::SharedComponent;
    }

    if is_next_app_route_path(&lower, frameworks) {
        if is_admin_surface_path(&lower) {
            return RouteContextKind::AdminSurface;
        }

        if file_stem(&lower) == Some("layout") {
            return RouteContextKind::RouteLayout;
        }

        return RouteContextKind::RoutePage;
    }

    if is_generic_route_path(&lower) {
        if is_admin_surface_path(&lower) {
            return RouteContextKind::AdminSurface;
        }

        return RouteContextKind::RoutePage;
    }

    if is_admin_surface_path(&lower) {
        return RouteContextKind::AdminSurface;
    }

    RouteContextKind::NonRoute
}

fn normalize_relative_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn is_next_app_route_path(path: &str, frameworks: &[String]) -> bool {
    has_framework(frameworks, "Next.js")
        && is_next_app_root_path(path)
        && has_supported_source_extension(path)
        && is_next_app_route_stem(path)
}

fn is_generic_route_path(path: &str) -> bool {
    has_supported_source_extension(path)
        && !is_generic_route_special_case(path)
        && (path.starts_with("pages/")
            || path.starts_with("routes/")
            || path.starts_with("src/pages/")
            || path.starts_with("src/routes/"))
}

fn is_shared_component_path(path: &str, frameworks: &[String]) -> bool {
    has_supported_source_extension(path)
        && (path.starts_with("components/")
            || path.starts_with("src/components/")
            || path.starts_with("shared/")
            || path.starts_with("src/shared/")
            || is_nested_collocated_shared_component_path(path, frameworks))
}

fn is_admin_surface_path(path: &str) -> bool {
    has_supported_source_extension(path)
        && (path.starts_with("admin/")
            || path.starts_with("src/admin/")
            || path.contains("/admin/")
            || (is_generic_route_root_path(path) && file_stem(path) == Some("admin")))
}

fn has_supported_source_extension(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some(
            "js" | "jsx"
                | "ts"
                | "tsx"
                | "cjs"
                | "cjsx"
                | "cts"
                | "ctsx"
                | "mjs"
                | "mjsx"
                | "mts"
                | "mtsx"
                | "vue"
                | "svelte"
                | "astro"
        )
    )
}

fn file_stem(path: &str) -> Option<&str> {
    Path::new(path).file_stem().and_then(|stem| stem.to_str())
}

fn is_next_app_root_path(path: &str) -> bool {
    next_app_relative_segments(path).is_some()
}

fn is_next_app_route_stem(path: &str) -> bool {
    matches!(
        file_stem(path),
        Some("page" | "layout" | "loading" | "error" | "template" | "not-found" | "default")
    )
}

fn is_generic_route_special_case(path: &str) -> bool {
    path.starts_with("pages/api/")
        || path.starts_with("src/pages/api/")
        || file_stem(path).is_some_and(|stem| matches!(stem, "_app" | "_document" | "_error"))
}

fn path_segments(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn is_generic_route_root_path(path: &str) -> bool {
    generic_route_relative_segments(path).is_some()
}

fn is_nested_collocated_shared_component_path(path: &str, frameworks: &[String]) -> bool {
    if has_framework(frameworks, "Next.js")
        && next_app_relative_segments(path).is_some_and(|segments| {
            has_nested_collocated_support_segment(&segments, is_next_app_route_stem(path))
        })
    {
        return true;
    }

    generic_route_relative_segments(path).is_some_and(|segments| {
        has_nested_collocated_support_segment(&segments, file_stem(path) == Some("index"))
    })
}

fn has_nested_collocated_support_segment(segments: &[&str], is_route_leaf: bool) -> bool {
    if is_route_leaf || segments.len() < 3 {
        return false;
    }

    segments[..segments.len() - 1]
        .iter()
        .enumerate()
        .any(|(index, segment)| index > 0 && matches!(*segment, "components" | "shared"))
}

fn next_app_relative_segments(path: &str) -> Option<Vec<&str>> {
    let segments = path_segments(path);

    match segments.as_slice() {
        ["app", rest @ ..] if !rest.is_empty() => Some(rest.to_vec()),
        ["src", "app", rest @ ..] if !rest.is_empty() => Some(rest.to_vec()),
        _ => None,
    }
}

fn generic_route_relative_segments(path: &str) -> Option<Vec<&str>> {
    let segments = path_segments(path);

    match segments.as_slice() {
        ["pages", rest @ ..] if !rest.is_empty() => Some(rest.to_vec()),
        ["routes", rest @ ..] if !rest.is_empty() => Some(rest.to_vec()),
        ["src", "pages", rest @ ..] if !rest.is_empty() => Some(rest.to_vec()),
        ["src", "routes", rest @ ..] if !rest.is_empty() => Some(rest.to_vec()),
        _ => None,
    }
}

fn has_framework(frameworks: &[String], expected: &str) -> bool {
    frameworks
        .iter()
        .any(|framework| framework.eq_ignore_ascii_case(expected))
}
