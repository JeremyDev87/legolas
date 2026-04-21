use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    artifacts::{
        package_name_from_module_id, ArtifactChunk, ArtifactModuleContribution, ArtifactSummary,
    },
    LegolasError, Result,
};

pub fn parse_rollup_metadata(value: &Value) -> Result<ArtifactSummary> {
    let outputs =
        value
            .get("outputs")
            .and_then(Value::as_array)
            .ok_or(LegolasError::NotImplemented(
                "unsupported artifact file shape",
            ))?;

    let mut summary = ArtifactSummary {
        bundler: "rollup".to_string(),
        entrypoints: Vec::new(),
        chunks: Vec::new(),
        modules: Vec::new(),
        total_bytes: 0,
    };
    let mut modules_by_id = BTreeMap::new();

    for output in outputs {
        if output.get("type").and_then(Value::as_str) != Some("chunk") {
            continue;
        }

        let name = output
            .get("name")
            .and_then(Value::as_str)
            .or_else(|| output.get("file").and_then(Value::as_str))
            .unwrap_or("chunk")
            .to_string();
        let file = output
            .get("file")
            .and_then(Value::as_str)
            .unwrap_or("dist/chunk.js")
            .to_string();
        let is_entry = output
            .get("isEntry")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if is_entry {
            summary.entrypoints.push(name.clone());
        }

        let modules = output
            .get("modules")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let chunk_bytes = modules
            .values()
            .map(|module| {
                module
                    .get("renderedLength")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            })
            .sum::<u64>() as usize;
        summary.total_bytes += chunk_bytes;
        summary.chunks.push(ArtifactChunk {
            name: name.clone(),
            entrypoints: is_entry.then_some(name.clone()).into_iter().collect(),
            files: vec![file],
            initial: output
                .get("isDynamicEntry")
                .and_then(Value::as_bool)
                .map(|value| !value)
                .unwrap_or(true),
            bytes: chunk_bytes,
        });

        for (module_id, module) in modules {
            let bytes = module
                .get("renderedLength")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            if bytes == 0 {
                continue;
            }

            let entry = modules_by_id.entry(module_id.clone()).or_insert_with(|| {
                ArtifactModuleContribution {
                    id: module_id.clone(),
                    package_name: package_name_from_module_id(&module_id),
                    chunks: Vec::new(),
                    bytes: 0,
                }
            });
            entry.chunks.push(name.clone());
            entry.bytes += bytes;
        }
    }

    summary.modules = modules_by_id.into_values().collect();

    Ok(summary.normalized())
}
