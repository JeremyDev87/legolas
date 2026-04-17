use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::{error::Result, LegolasError};

pub fn find_project_root<P: AsRef<Path>>(_input_path: P) -> Result<PathBuf> {
    Err(LegolasError::NotImplemented("find_project_root"))
}

pub fn read_text_if_exists<P: AsRef<Path>>(_file_path: P) -> Result<Option<String>> {
    Err(LegolasError::NotImplemented("read_text_if_exists"))
}

pub fn read_json_if_exists<T, P>(_file_path: P) -> Result<Option<T>>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    Err(LegolasError::NotImplemented("read_json_if_exists"))
}

pub fn exists<P: AsRef<Path>>(_file_path: P) -> Result<bool> {
    Err(LegolasError::NotImplemented("exists"))
}
