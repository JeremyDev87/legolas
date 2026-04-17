use std::path::Path;

use crate::{error::Result, models::DuplicatePackage, LegolasError};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DuplicateAnalysis {
    pub duplicates: Vec<DuplicatePackage>,
    pub warnings: Vec<String>,
}

pub fn parse_duplicate_packages<P: AsRef<Path>>(
    _project_root: P,
    _package_manager: &str,
) -> Result<DuplicateAnalysis> {
    Err(LegolasError::NotImplemented("parse_duplicate_packages"))
}
