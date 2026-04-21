use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::{
    artifacts::{
        esbuild::parse_esbuild_metafile, rollup::parse_rollup_metadata,
        webpack::parse_webpack_stats, ArtifactSummary,
    },
    LegolasError, Result,
};

pub const KNOWN_ARTIFACT_FILES: [&str; 5] = [
    "stats.json",
    "dist/stats.json",
    "build/stats.json",
    "meta.json",
    "dist/meta.json",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactParserKind {
    Esbuild,
    Rollup,
    Webpack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedArtifact {
    pub relative_path: String,
    pub parser: ArtifactParserKind,
}

pub fn detect_known_artifacts(project_root: &Path) -> Result<Vec<DetectedArtifact>> {
    let mut detected = Vec::new();

    for relative_path in KNOWN_ARTIFACT_FILES {
        let absolute_path = project_root.join(relative_path);
        if !is_file(&absolute_path) {
            continue;
        }

        let value = match read_json(&absolute_path) {
            Ok(value) => value,
            Err(LegolasError::JsonParse(_)) => continue,
            Err(error) => return Err(error),
        };
        let Some(parser) = detect_parser_kind(&absolute_path, &value) else {
            continue;
        };

        detected.push(DetectedArtifact {
            relative_path: relative_path.to_string(),
            parser,
        });
    }

    Ok(detected)
}

pub fn parse_artifact_file(path: &Path) -> Result<ArtifactSummary> {
    let value = read_json(path)?;
    parse_artifact_value(path, &value)
}

pub fn parse_artifact_value(path: &Path, value: &Value) -> Result<ArtifactSummary> {
    match detect_parser_kind(path, value) {
        Some(ArtifactParserKind::Esbuild) => parse_esbuild_metafile(value),
        Some(ArtifactParserKind::Rollup) => parse_rollup_metadata(value),
        Some(ArtifactParserKind::Webpack) => parse_webpack_stats(value),
        None => Err(LegolasError::NotImplemented(
            "unsupported artifact file shape",
        )),
    }
}

pub fn detect_parser_kind(path: &Path, value: &Value) -> Option<ArtifactParserKind> {
    let file_name = path.file_name()?.to_str()?;

    if file_name.eq_ignore_ascii_case("stats.json") && looks_like_webpack_stats(value) {
        return Some(ArtifactParserKind::Webpack);
    }

    if !file_name.eq_ignore_ascii_case("meta.json") {
        return None;
    }

    if value.get("inputs").is_some() && value.get("outputs").is_some() {
        return Some(ArtifactParserKind::Esbuild);
    }

    if value
        .get("outputs")
        .is_some_and(|outputs| outputs.is_array())
    {
        return Some(ArtifactParserKind::Rollup);
    }

    None
}

fn looks_like_webpack_stats(value: &Value) -> bool {
    webpack_entrypoints(value).is_some() && value.get("chunks").is_some_and(Value::is_array)
}

fn webpack_entrypoints(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    value
        .get("entryPoints")
        .or_else(|| value.get("entrypoints"))
        .and_then(Value::as_object)
}

fn is_file(path: &PathBuf) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

fn read_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}
