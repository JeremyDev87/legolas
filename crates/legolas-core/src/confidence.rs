use crate::{findings::FindingConfidence, import_scanner::ImportedPackageRecord};

pub fn score_heavy_dependency(import_info: Option<&ImportedPackageRecord>) -> FindingConfidence {
    if import_info.is_some_and(|item| !item.files.is_empty()) {
        FindingConfidence::High
    } else {
        FindingConfidence::Low
    }
}

pub fn score_duplicate_package() -> FindingConfidence {
    FindingConfidence::High
}

pub fn score_lazy_load_candidate() -> FindingConfidence {
    FindingConfidence::Medium
}

pub fn score_tree_shaking_warning() -> FindingConfidence {
    FindingConfidence::High
}
