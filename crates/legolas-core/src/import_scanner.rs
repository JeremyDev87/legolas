use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{error::Result, models::TreeShakingWarning, LegolasError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportedPackageRecord {
    pub name: String,
    pub files: Vec<String>,
    pub static_files: Vec<String>,
    pub dynamic_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceAnalysis {
    pub by_package: BTreeMap<String, ImportedPackageRecord>,
    pub imported_packages: Vec<ImportedPackageRecord>,
    pub dynamic_import_count: usize,
    pub tree_shaking_warnings: Vec<TreeShakingWarning>,
}

pub fn collect_source_files<P: AsRef<Path>>(_project_root: P) -> Result<Vec<PathBuf>> {
    Err(LegolasError::NotImplemented("collect_source_files"))
}

pub fn scan_imports<P: AsRef<Path>>(
    _project_root: P,
    _source_files: &[PathBuf],
) -> Result<SourceAnalysis> {
    Err(LegolasError::NotImplemented("scan_imports"))
}
