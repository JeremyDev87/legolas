use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    artifacts::{
        package_name_from_module_id, ArtifactChunk, ArtifactModuleContribution, ArtifactSummary,
    },
    LegolasError, Result,
};

pub fn parse_webpack_stats(value: &Value) -> Result<ArtifactSummary> {
    let entrypoints = value
        .get("entryPoints")
        .or_else(|| value.get("entrypoints"))
        .and_then(Value::as_object)
        .ok_or(LegolasError::NotImplemented(
            "unsupported artifact file shape",
        ))?;
    let chunks =
        value
            .get("chunks")
            .and_then(Value::as_array)
            .ok_or(LegolasError::NotImplemented(
                "unsupported artifact file shape",
            ))?;

    let mut entrypoint_assets = BTreeMap::new();
    for (entry_name, entry_value) in entrypoints {
        let assets = entry_value
            .get("assets")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|asset| asset.get("name").and_then(Value::as_str))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        entrypoint_assets.insert(entry_name.clone(), assets);
    }

    let mut summary = ArtifactSummary {
        bundler: "webpack".to_string(),
        entrypoints: entrypoint_assets.keys().cloned().collect(),
        chunks: Vec::new(),
        modules: Vec::new(),
        total_bytes: 0,
    };
    let mut modules_by_id = BTreeMap::new();

    for chunk in chunks {
        let files = chunk
            .get("files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let chunk_entrypoints = entrypoint_assets
            .iter()
            .filter(|(_, assets)| {
                assets
                    .iter()
                    .any(|asset| files.iter().any(|file| file == asset))
            })
            .map(|(entry_name, _)| entry_name.clone())
            .collect::<Vec<_>>();
        let name = chunk
            .get("names")
            .and_then(Value::as_array)
            .and_then(|names| names.first())
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                chunk
                    .get("id")
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "chunk".to_string())
            });
        let bytes = chunk.get("size").and_then(Value::as_u64).unwrap_or(0) as usize;
        summary.total_bytes += bytes;
        summary.chunks.push(ArtifactChunk {
            name: name.clone(),
            entrypoints: chunk_entrypoints,
            files,
            initial: chunk
                .get("initial")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            bytes,
        });

        let Some(modules) = chunk.get("modules").and_then(Value::as_array) else {
            continue;
        };

        for module in modules {
            let id = module
                .get("identifier")
                .or_else(|| module.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                continue;
            }

            let bytes = module.get("size").and_then(Value::as_u64).unwrap_or(0) as usize;
            if bytes == 0 {
                continue;
            }

            let entry =
                modules_by_id
                    .entry(id.clone())
                    .or_insert_with(|| ArtifactModuleContribution {
                        id: id.clone(),
                        package_name: package_name_from_module_id(&id),
                        chunks: Vec::new(),
                        bytes: 0,
                    });
            entry.chunks.push(name.clone());
            entry.bytes += bytes;
        }
    }

    summary.modules = modules_by_id.into_values().collect();

    Ok(summary.normalized())
}
