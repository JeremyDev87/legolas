use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    artifacts::{
        package_name_from_module_id, ArtifactChunk, ArtifactModuleContribution, ArtifactSummary,
    },
    LegolasError, Result,
};

pub fn parse_esbuild_metafile(value: &Value) -> Result<ArtifactSummary> {
    let outputs =
        value
            .get("outputs")
            .and_then(Value::as_object)
            .ok_or(LegolasError::NotImplemented(
                "unsupported artifact file shape",
            ))?;

    let mut summary = ArtifactSummary {
        bundler: "esbuild".to_string(),
        entrypoints: Vec::new(),
        chunks: Vec::new(),
        modules: Vec::new(),
        total_bytes: 0,
    };
    let mut modules = BTreeMap::new();

    for (output_path, output) in outputs {
        if !is_supported_chunk_output(output_path, output) {
            continue;
        }

        let bytes = output.get("bytes").and_then(Value::as_u64).unwrap_or(0) as usize;
        let entry_point = output
            .get("entryPoint")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        if let Some(entry_point) = entry_point.clone() {
            summary.entrypoints.push(entry_point);
        }

        summary.total_bytes += bytes;
        summary.chunks.push(ArtifactChunk {
            name: chunk_name(output_path),
            entrypoints: entry_point.into_iter().collect(),
            files: vec![output_path.clone()],
            initial: output.get("entryPoint").is_some(),
            bytes,
        });

        let Some(inputs) = output.get("inputs").and_then(Value::as_object) else {
            continue;
        };

        for (module_id, contribution) in inputs {
            let bytes = contribution
                .get("bytesInOutput")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            if bytes == 0 {
                continue;
            }

            let entry =
                modules
                    .entry(module_id.clone())
                    .or_insert_with(|| ArtifactModuleContribution {
                        id: module_id.clone(),
                        package_name: package_name_from_module_id(module_id),
                        chunks: Vec::new(),
                        bytes: 0,
                    });
            entry.chunks.push(chunk_name(output_path));
            entry.bytes += bytes;
        }
    }

    summary.modules = modules.into_values().collect();

    Ok(summary.normalized())
}

fn chunk_name(output_path: &str) -> String {
    std::path::Path::new(output_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(output_path)
        .to_string()
}

fn is_supported_chunk_output(output_path: &str, output: &Value) -> bool {
    output.get("inputs").is_some_and(Value::is_object)
        && matches!(
            std::path::Path::new(output_path)
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("js" | "mjs" | "cjs")
        )
}
