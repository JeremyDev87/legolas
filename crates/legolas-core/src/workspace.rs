use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

use serde::de::DeserializeOwned;

use crate::{error::Result, LegolasError};

const ROOT_MARKERS: [&str; 6] = [
    "package.json",
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
];

pub fn find_project_root<P: AsRef<Path>>(input_path: P) -> Result<PathBuf> {
    let resolved = resolve_absolute(input_path.as_ref())?;
    let mut current = normalize_to_directory(&resolved)?;
    let initial_directory = current.clone();

    loop {
        for marker in ROOT_MARKERS {
            if exists(current.join(marker))? {
                return Ok(current);
            }
        }

        let Some(parent) = current.parent() else {
            return Ok(initial_directory);
        };
        let parent = parent.to_path_buf();

        if parent == current {
            return Ok(initial_directory);
        }

        current = parent;
    }
}

pub fn find_discovered_config_path<P: AsRef<Path>>(input_path: P) -> Result<Option<PathBuf>> {
    let project_root = find_project_root(input_path)?;
    let config_path = project_root.join("legolas.config.json");

    match fs::metadata(&config_path) {
        Ok(_) => Ok(Some(config_path)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn read_text_if_exists<P: AsRef<Path>>(file_path: P) -> Result<Option<String>> {
    match fs::read_to_string(file_path.as_ref()) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn read_json_if_exists<T, P>(file_path: P) -> Result<Option<T>>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    match read_text_if_exists(file_path)? {
        Some(contents) if contents.is_empty() => Ok(None),
        Some(contents) => Ok(Some(serde_json::from_str(&contents)?)),
        None => Ok(None),
    }
}

pub fn exists<P: AsRef<Path>>(file_path: P) -> Result<bool> {
    match fs::metadata(file_path.as_ref()) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

fn normalize_to_directory(target_path: &Path) -> Result<PathBuf> {
    match fs::metadata(target_path) {
        Ok(stats) => {
            if stats.is_dir() {
                Ok(target_path.to_path_buf())
            } else {
                Ok(target_path.parent().unwrap_or(target_path).to_path_buf())
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Err(LegolasError::PathNotFound(
            target_path.display().to_string(),
        )),
        Err(error) => Err(error.into()),
    }
}

fn resolve_absolute(input_path: &Path) -> Result<PathBuf> {
    let absolute = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(input_path)
    };

    Ok(normalize_path(&absolute))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }

    normalized
}
