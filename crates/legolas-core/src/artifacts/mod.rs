pub mod models;

pub mod detect;
pub mod esbuild;
pub mod rollup;
pub mod webpack;

pub use models::{ArtifactChunk, ArtifactModuleContribution, ArtifactSummary};

pub(crate) fn package_name_from_module_id(module_id: &str) -> Option<String> {
    let normalized = module_id.replace('\\', "/");
    let (_, package_path) = normalized.rsplit_once("node_modules/")?;
    let mut segments = package_path
        .split('/')
        .filter(|segment| !segment.is_empty());
    let first = segments.next()?;
    if first.starts_with('@') {
        Some(format!("{}/{}", first, segments.next()?))
    } else {
        Some(first.to_string())
    }
}
