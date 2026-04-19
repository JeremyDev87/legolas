use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingAnalysisSource {
    Heuristic,
    SourceImport,
    LockfileTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FindingEvidence {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl FindingEvidence {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            ..Self::default()
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_specifier(mut self, specifier: impl Into<String>) -> Self {
        self.specifier = Some(specifier.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FindingMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finding_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis_source: Option<FindingAnalysisSource>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<FindingEvidence>,
}

impl FindingMetadata {
    pub fn new(finding_id: impl Into<String>, analysis_source: FindingAnalysisSource) -> Self {
        Self {
            finding_id: Some(finding_id.into()),
            analysis_source: Some(analysis_source),
            evidence: Vec::new(),
        }
    }

    pub fn with_evidence<I>(mut self, evidence: I) -> Self
    where
        I: IntoIterator<Item = FindingEvidence>,
    {
        self.evidence = evidence.into_iter().collect();
        self
    }

    pub fn push_evidence(&mut self, evidence: FindingEvidence) {
        self.evidence.push(evidence);
    }

    pub fn is_empty(&self) -> bool {
        self.finding_id.is_none() && self.analysis_source.is_none() && self.evidence.is_empty()
    }
}
