use serde_json::Value;
use std::path::PathBuf;

#[allow(dead_code)]
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

#[allow(dead_code)]
pub fn fixture_path(relative_path: &str) -> PathBuf {
    workspace_root().join(relative_path)
}

#[allow(dead_code)]
pub fn read_oracle(relative_path: &str) -> String {
    std::fs::read_to_string(workspace_root().join("tests/oracles").join(relative_path))
        .expect("read oracle")
}

#[allow(dead_code)]
pub fn normalize_cli_output(output: &str) -> String {
    to_posix(output.to_string()).replace(
        &to_posix(
            fixture_path("tests/fixtures/parity/basic-app")
                .display()
                .to_string(),
        ),
        "<PROJECT_ROOT>",
    )
}

#[allow(dead_code)]
pub fn normalize_analysis_json_output(output: &str) -> Value {
    let mut analysis = serde_json::from_str::<Value>(output).expect("parse analysis json");
    normalize_analysis_value(&mut analysis);
    analysis
}

#[allow(dead_code)]
fn normalize_analysis_value(analysis: &mut Value) {
    replace_string_field(analysis, &["projectRoot"], "<PROJECT_ROOT>");
    replace_string_field(analysis, &["metadata", "generatedAt"], "<GENERATED_AT>");
    normalize_string_array(analysis, &["bundleArtifacts"]);
    normalize_string_array(analysis, &["warnings"]);
    normalize_object_array_string_field(analysis, &["heavyDependencies"], "importedBy");
    normalize_object_array_string_field(analysis, &["heavyDependencies"], "dynamicImportedBy");
    normalize_object_array_string_field(analysis, &["lazyLoadCandidates"], "files");
    normalize_object_array_string_field(analysis, &["treeShakingWarnings"], "files");
}

#[allow(dead_code)]
fn replace_string_field(root: &mut Value, path: &[&str], replacement: &str) {
    let Some(value) = get_path_mut(root, path) else {
        return;
    };

    if value.is_string() {
        *value = Value::String(replacement.to_string());
    }
}

#[allow(dead_code)]
fn normalize_string_array(root: &mut Value, path: &[&str]) {
    let Some(Value::Array(items)) = get_path_mut(root, path) else {
        return;
    };

    for item in items {
        if let Some(value) = item.as_str() {
            *item = Value::String(to_posix(value.to_string()));
        }
    }
}

#[allow(dead_code)]
fn normalize_object_array_string_field(root: &mut Value, path: &[&str], field: &str) {
    let Some(Value::Array(items)) = get_path_mut(root, path) else {
        return;
    };

    for item in items {
        let Some(array) = item.get_mut(field).and_then(Value::as_array_mut) else {
            continue;
        };

        for entry in array {
            if let Some(value) = entry.as_str() {
                *entry = Value::String(to_posix(value.to_string()));
            }
        }
    }
}

#[allow(dead_code)]
fn get_path_mut<'a>(value: &'a mut Value, path: &[&str]) -> Option<&'a mut Value> {
    let mut current = value;

    for segment in path {
        current = current.get_mut(*segment)?;
    }

    Some(current)
}

#[allow(dead_code)]
fn to_posix(value: String) -> String {
    value.replace('\\', "/")
}
