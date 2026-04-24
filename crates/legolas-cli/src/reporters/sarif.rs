use std::collections::{BTreeMap, BTreeSet};

use legolas_core::{
    boundaries::BoundaryWarning, budget::BudgetEvaluation, Analysis, BaselineDiff,
    DuplicatePackage, FindingConfidence, FindingEvidence, FindingMetadata, HeavyDependency,
    LazyLoadCandidate, TreeShakingWarning,
};
use serde_json::{json, Map, Value};

const SARIF_SCHEMA_URL: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const SARIF_VERSION: &str = "2.1.0";
const LEGOLAS_INFO_URL: &str = "https://github.com/JeremyDev87/legolas";

pub fn scan_sarif_output(analysis: &Analysis) -> Value {
    sarif_output(
        collect_scan_records(analysis),
        base_run_properties("scan", analysis),
    )
}

pub fn ci_sarif_output(
    analysis: &Analysis,
    evaluation: &BudgetEvaluation,
    regression_diff: Option<&BaselineDiff>,
) -> Value {
    let triggered_finding_ids = evaluation
        .rules
        .iter()
        .flat_map(|rule| {
            rule.triggered_findings
                .iter()
                .map(|finding| finding.finding_id.clone())
        })
        .collect::<BTreeSet<_>>();

    let records = collect_scan_records(analysis)
        .into_iter()
        .filter(|record| triggered_finding_ids.contains(&record.rule_id))
        .collect::<Vec<_>>();

    sarif_output(
        records,
        ci_run_properties(analysis, evaluation, regression_diff),
    )
}

#[derive(Debug, Clone)]
struct SarifRecord {
    rule_id: String,
    message: String,
    short_description: String,
    level: &'static str,
    locations: Vec<Value>,
    properties: Map<String, Value>,
}

fn collect_scan_records(analysis: &Analysis) -> Vec<SarifRecord> {
    let mut records = Vec::new();

    records.extend(
        analysis
            .boundary_warnings
            .iter()
            .filter_map(boundary_warning_record),
    );
    records.extend(
        analysis
            .heavy_dependencies
            .iter()
            .filter_map(heavy_dependency_record),
    );
    records.extend(
        analysis
            .duplicate_packages
            .iter()
            .filter_map(duplicate_package_record),
    );
    records.extend(
        analysis
            .lazy_load_candidates
            .iter()
            .filter_map(lazy_load_candidate_record),
    );
    records.extend(
        analysis
            .tree_shaking_warnings
            .iter()
            .filter_map(tree_shaking_warning_record),
    );

    records
}

fn boundary_warning_record(item: &BoundaryWarning) -> Option<SarifRecord> {
    finding_record(
        &item.finding,
        item.message.clone(),
        item.message.clone(),
        Some(item.recommendation.as_str()),
        [],
    )
}

fn heavy_dependency_record(item: &HeavyDependency) -> Option<SarifRecord> {
    finding_record(
        &item.finding,
        format!(
            "{} ({} KB): {}",
            item.name, item.estimated_kb, item.rationale
        ),
        format!("Review {} upfront bundle weight", item.name),
        Some(item.recommendation.as_str()),
        [
            ("name", json!(item.name)),
            ("versionRange", json!(item.version_range)),
            ("estimatedKb", json!(item.estimated_kb)),
            ("category", json!(item.category)),
            ("importCount", json!(item.import_count)),
            ("importedBy", json!(item.imported_by)),
            ("dynamicImportedBy", json!(item.dynamic_imported_by)),
        ],
    )
}

fn duplicate_package_record(item: &DuplicatePackage) -> Option<SarifRecord> {
    finding_record(
        &item.finding,
        format!(
            "{} duplicated across {} ({} KB avoidable)",
            item.name,
            item.versions.join(", "),
            item.estimated_extra_kb
        ),
        format!("Deduplicate {}", item.name),
        None,
        [
            ("name", json!(item.name)),
            ("versions", json!(item.versions)),
            ("count", json!(item.count)),
            ("estimatedExtraKb", json!(item.estimated_extra_kb)),
            ("origins", json!(item.origins)),
        ],
    )
}

fn lazy_load_candidate_record(item: &LazyLoadCandidate) -> Option<SarifRecord> {
    finding_record(
        &item.finding,
        format!(
            "{} can be lazy loaded ({} KB): {}",
            item.name, item.estimated_savings_kb, item.reason
        ),
        format!("Lazy load {}", item.name),
        Some(item.recommendation.as_str()),
        [
            ("name", json!(item.name)),
            ("estimatedSavingsKb", json!(item.estimated_savings_kb)),
            ("files", json!(item.files)),
            ("reason", json!(item.reason)),
        ],
    )
}

fn tree_shaking_warning_record(item: &TreeShakingWarning) -> Option<SarifRecord> {
    finding_record(
        &item.finding,
        format!("{}: {}", item.package_name, item.message),
        format!("Review {} tree shaking", item.package_name),
        Some(item.recommendation.as_str()),
        [
            ("key", json!(item.key)),
            ("packageName", json!(item.package_name)),
            ("estimatedKb", json!(item.estimated_kb)),
            ("files", json!(item.files)),
        ],
    )
}

fn finding_record<I>(
    finding: &FindingMetadata,
    message: String,
    short_description: String,
    recommendation: Option<&str>,
    extra_properties: I,
) -> Option<SarifRecord>
where
    I: IntoIterator<Item = (&'static str, Value)>,
{
    let rule_id = finding.finding_id.clone()?;
    let mut properties = base_properties(finding);

    if let Some(recommendation) = recommendation {
        properties.insert("recommendation".to_string(), json!(recommendation));
    }

    for (key, value) in extra_properties {
        properties.insert(key.to_string(), value);
    }

    Some(SarifRecord {
        rule_id,
        message,
        short_description,
        level: sarif_level(finding.confidence),
        locations: locations_from_evidence(&finding.evidence),
        properties,
    })
}

fn base_properties(finding: &FindingMetadata) -> Map<String, Value> {
    let mut properties = Map::new();

    if let Some(analysis_source) = finding.analysis_source {
        properties.insert("analysisSource".to_string(), json!(analysis_source));
    }
    if let Some(confidence) = finding.confidence {
        properties.insert("confidence".to_string(), json!(confidence));
    }
    if let Some(action_priority) = finding.action_priority {
        properties.insert("actionPriority".to_string(), json!(action_priority));
    }
    if let Some(recommended_fix) = &finding.recommended_fix {
        properties.insert("recommendedFix".to_string(), json!(recommended_fix));
    }

    properties.insert("evidence".to_string(), json!(finding.evidence));
    properties
}

fn locations_from_evidence(evidence: &[FindingEvidence]) -> Vec<Value> {
    let mut seen = BTreeSet::new();
    let mut locations = Vec::new();

    for item in evidence {
        let Some(file) = item.file.as_ref() else {
            continue;
        };

        if seen.insert(file.clone()) {
            locations.push(json!({
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": file
                    }
                }
            }));
        }
    }

    locations
}

fn sarif_level(confidence: Option<FindingConfidence>) -> &'static str {
    match confidence {
        Some(FindingConfidence::High) => "warning",
        Some(FindingConfidence::Medium) | Some(FindingConfidence::Low) | None => "note",
    }
}

fn base_run_properties(command: &str, analysis: &Analysis) -> Map<String, Value> {
    let mut properties = Map::new();
    properties.insert("command".to_string(), json!(command));

    if !analysis.warnings.is_empty() {
        properties.insert("warnings".to_string(), json!(analysis.warnings));
    }

    properties
}

fn ci_run_properties(
    analysis: &Analysis,
    evaluation: &BudgetEvaluation,
    regression_diff: Option<&BaselineDiff>,
) -> Map<String, Value> {
    let mut properties = base_run_properties("ci", analysis);
    properties.insert("passed".to_string(), json!(!evaluation.has_failures()));
    properties.insert(
        "overallStatus".to_string(),
        json!(evaluation.overall_status),
    );
    properties.insert("rules".to_string(), json!(evaluation.rules));

    if let Some(diff) = regression_diff {
        properties.insert(
            "regression".to_string(),
            json!({
                "mode": "regression-only",
                "baselineDiff": diff,
            }),
        );
    }

    properties
}

fn sarif_output(records: Vec<SarifRecord>, run_properties: Map<String, Value>) -> Value {
    let mut rules = BTreeMap::new();
    let results = records
        .iter()
        .map(|record| {
            rules.entry(record.rule_id.clone()).or_insert_with(|| {
                json!({
                    "id": record.rule_id,
                    "shortDescription": {
                        "text": record.short_description
                    }
                })
            });

            let mut result = Map::new();
            result.insert("ruleId".to_string(), json!(record.rule_id));
            result.insert("level".to_string(), json!(record.level));
            result.insert(
                "message".to_string(),
                json!({
                    "text": record.message
                }),
            );
            result.insert(
                "properties".to_string(),
                Value::Object(record.properties.clone()),
            );
            if !record.locations.is_empty() {
                result.insert(
                    "locations".to_string(),
                    Value::Array(record.locations.clone()),
                );
            }

            Value::Object(result)
        })
        .collect::<Vec<_>>();

    json!({
        "$schema": SARIF_SCHEMA_URL,
        "version": SARIF_VERSION,
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "legolas",
                        "informationUri": LEGOLAS_INFO_URL,
                        "rules": rules.into_values().collect::<Vec<_>>()
                    }
                },
                "properties": run_properties,
                "results": results
            }
        ]
    })
}
