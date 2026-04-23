use crate::{
    findings::{FindingConfidence, FindingMetadata},
    models::RecommendedFix,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixHintKind {
    DynamicImport,
    SubpathImport,
    RouteSplit,
    DedupeResolution,
}

impl FixHintKind {
    fn as_str(self) -> &'static str {
        match self {
            FixHintKind::DynamicImport => "dynamic-import",
            FixHintKind::SubpathImport => "subpath-import",
            FixHintKind::RouteSplit => "route-split",
            FixHintKind::DedupeResolution => "dedupe-resolution",
        }
    }
}

pub fn dynamic_import_fix_hint(
    finding: &FindingMetadata,
    title: impl Into<String>,
    target_files: Vec<String>,
) -> Option<RecommendedFix> {
    high_confidence_fix_hint(
        FixHintKind::DynamicImport,
        finding,
        title,
        target_files,
        None,
    )
}

pub fn subpath_import_fix_hint(
    finding: &FindingMetadata,
    title: impl Into<String>,
    target_files: Vec<String>,
    replacement: Option<String>,
) -> Option<RecommendedFix> {
    high_confidence_fix_hint(
        FixHintKind::SubpathImport,
        finding,
        title,
        target_files,
        replacement,
    )
}

pub fn route_split_fix_hint(
    finding: &FindingMetadata,
    title: impl Into<String>,
    target_files: Vec<String>,
) -> Option<RecommendedFix> {
    high_confidence_fix_hint(FixHintKind::RouteSplit, finding, title, target_files, None)
}

pub fn dedupe_resolution_fix_hint(
    finding: &FindingMetadata,
    title: impl Into<String>,
) -> Option<RecommendedFix> {
    high_confidence_fix_hint(
        FixHintKind::DedupeResolution,
        finding,
        title,
        Vec::new(),
        None,
    )
}

pub fn high_confidence_fix_hint(
    kind: FixHintKind,
    finding: &FindingMetadata,
    title: impl Into<String>,
    target_files: Vec<String>,
    replacement: Option<String>,
) -> Option<RecommendedFix> {
    if finding.confidence != Some(FindingConfidence::High) {
        return None;
    }

    let target_files = normalized_files(target_files);
    if target_files.is_empty() && !matches!(kind, FixHintKind::DedupeResolution) {
        return None;
    }

    Some(RecommendedFix {
        kind: kind.as_str().to_string(),
        title: title.into(),
        target_files,
        replacement,
    })
}

fn normalized_files(files: Vec<String>) -> Vec<String> {
    let mut files = files;
    files.sort();
    files.dedup();
    files
}
