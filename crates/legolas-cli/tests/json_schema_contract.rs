mod support;

use assert_cmd::Command;
use serde_json::{json, Value};

const ANALYSIS_SCHEMA_VERSION: &str = "legolas.analysis.v1";
const BUDGET_SCHEMA_VERSION: &str = "legolas.budget.v1";
const CI_SCHEMA_VERSION: &str = "legolas.ci.v1";

#[test]
fn scan_json_matches_analysis_schema_doc() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(["scan", &fixture.display().to_string(), "--json"])
        .output()
        .expect("run scan --json");

    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).expect("stdout");
    let value = serde_json::from_str::<Value>(&output).expect("parse scan json");
    let schema = read_schema("analysis.v1.schema.json");

    assert_eq!(value["schemaVersion"], json!(ANALYSIS_SCHEMA_VERSION));
    assert_matches_schema(&value, &schema, "$");
}

#[test]
fn optimize_json_matches_analysis_schema_doc() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(["optimize", &fixture.display().to_string(), "--json"])
        .output()
        .expect("run optimize --json");

    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).expect("stdout");
    let value = serde_json::from_str::<Value>(&output).expect("parse optimize json");
    let schema = read_schema("analysis.v1.schema.json");

    assert_eq!(value["schemaVersion"], json!(ANALYSIS_SCHEMA_VERSION));
    assert_matches_schema(&value, &schema, "$");
}

#[test]
fn budget_json_matches_budget_schema_doc() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(["budget", &fixture.display().to_string(), "--json"])
        .output()
        .expect("run budget --json");

    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).expect("stdout");
    let value = serde_json::from_str::<Value>(&output).expect("parse budget json");
    let schema = read_schema("budget.v1.schema.json");

    assert_eq!(value["schemaVersion"], json!(BUDGET_SCHEMA_VERSION));
    assert_matches_schema(&value, &schema, "$");
}

#[test]
fn ci_json_matches_ci_schema_doc() {
    let fixture = support::fixture_path("tests/fixtures/parity/basic-app");
    let output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args(["ci", &fixture.display().to_string(), "--json"])
        .output()
        .expect("run ci --json");

    assert!(!output.status.success());
    let output = String::from_utf8(output.stdout).expect("stdout");
    let value = serde_json::from_str::<Value>(&output).expect("parse ci json");
    let schema = read_schema("ci.v1.schema.json");

    assert_eq!(value["schemaVersion"], json!(CI_SCHEMA_VERSION));
    assert_matches_schema(&value, &schema, "$");
}

#[test]
fn regression_only_ci_json_matches_ci_schema_doc() {
    let fixture = support::fixture_path("tests/fixtures/baseline/current-app");
    let baseline = support::fixture_path("tests/fixtures/baseline/previous-scan.json");
    let output = Command::cargo_bin("legolas-cli")
        .expect("build binary")
        .args([
            "ci",
            &fixture.display().to_string(),
            "--baseline",
            &baseline.display().to_string(),
            "--regression-only",
            "--json",
        ])
        .output()
        .expect("run regression ci --json");

    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).expect("stdout");
    let value = serde_json::from_str::<Value>(&output).expect("parse regression ci json");
    let schema = read_schema("ci.v1.schema.json");

    assert_eq!(value["schemaVersion"], json!(CI_SCHEMA_VERSION));
    assert_matches_schema(&value, &schema, "$");
}

fn read_schema(relative_path: &str) -> Value {
    let schema_path = support::workspace_root()
        .join("docs")
        .join("schema")
        .join(relative_path);
    let contents = std::fs::read_to_string(&schema_path).expect("read schema");

    serde_json::from_str(&contents).expect("parse schema")
}

fn assert_matches_schema(value: &Value, schema: &Value, path: &str) {
    if let Some(expected) = schema.get("const") {
        assert_eq!(value, expected, "{path}: const mismatch");
    }

    if let Some(expected) = schema.get("enum").and_then(Value::as_array) {
        assert!(
            expected.iter().any(|item| item == value),
            "{path}: expected one of {expected:?}, got {value:?}"
        );
    }

    let Some(expected_type) = schema.get("type").and_then(Value::as_str) else {
        return;
    };

    match expected_type {
        "object" => {
            let object = value.as_object().unwrap_or_else(|| {
                panic!("{path}: expected object, got {value:?}");
            });
            let schema_object = schema
                .as_object()
                .expect("object schema should be an object");
            let required = schema_object
                .get("required")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let properties = schema_object
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let allow_additional_properties = schema_object
                .get("additionalProperties")
                .and_then(Value::as_bool)
                .unwrap_or(properties.is_empty());

            for required_key in required {
                let required_key = required_key.as_str().expect("required key string");
                assert!(
                    object.contains_key(required_key),
                    "{path}: missing required key `{required_key}`"
                );
            }

            if !allow_additional_properties {
                for key in object.keys() {
                    assert!(
                        properties.contains_key(key),
                        "{path}: unexpected key `{key}`"
                    );
                }
            }

            for (key, property_schema) in properties {
                if let Some(property_value) = object.get(&key) {
                    assert_matches_schema(
                        property_value,
                        &property_schema,
                        &format!("{path}.{key}"),
                    );
                }
            }
        }
        "array" => {
            let items = value.as_array().unwrap_or_else(|| {
                panic!("{path}: expected array, got {value:?}");
            });
            let item_schema = schema.get("items").expect("array schema must define items");

            for (index, item) in items.iter().enumerate() {
                assert_matches_schema(item, item_schema, &format!("{path}[{index}]"));
            }
        }
        "string" => {
            assert!(value.is_string(), "{path}: expected string, got {value:?}");
        }
        "integer" => {
            assert!(
                value.as_i64().is_some() || value.as_u64().is_some(),
                "{path}: expected integer, got {value:?}"
            );
        }
        "boolean" => {
            assert!(
                value.is_boolean(),
                "{path}: expected boolean, got {value:?}"
            );
        }
        _ => panic!("{path}: unsupported schema type `{expected_type}`"),
    }
}
