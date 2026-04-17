use std::path::Path;

use serde_json::Value;

use crate::{error::Result, LegolasError};

pub fn detect_frameworks(_project_root: &Path, _manifest: &Value) -> Result<Vec<String>> {
    Err(LegolasError::NotImplemented("detect_frameworks"))
}

pub fn detect_package_manager(_project_root: &Path, _manifest: &Value) -> Result<String> {
    Err(LegolasError::NotImplemented("detect_package_manager"))
}
