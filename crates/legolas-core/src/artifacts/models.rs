use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSummary {
    pub bundler: String,
    pub entrypoints: Vec<String>,
    pub chunks: Vec<ArtifactChunk>,
    pub modules: Vec<ArtifactModuleContribution>,
    pub total_bytes: usize,
}

impl ArtifactSummary {
    pub fn normalize(&mut self) {
        sort_and_dedup(&mut self.entrypoints);

        for chunk in &mut self.chunks {
            chunk.normalize();
        }
        self.chunks.sort_unstable_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(left.initial.cmp(&right.initial))
                .then(left.bytes.cmp(&right.bytes))
                .then(left.entrypoints.cmp(&right.entrypoints))
                .then(left.files.cmp(&right.files))
        });

        for module in &mut self.modules {
            module.normalize();
        }
        self.modules.sort_unstable_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then(left.package_name.cmp(&right.package_name))
                .then(left.bytes.cmp(&right.bytes))
                .then(left.chunks.cmp(&right.chunks))
        });
    }

    pub fn normalized(mut self) -> Self {
        self.normalize();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactChunk {
    pub name: String,
    pub entrypoints: Vec<String>,
    pub files: Vec<String>,
    pub initial: bool,
    pub bytes: usize,
}

impl ArtifactChunk {
    pub fn normalize(&mut self) {
        sort_and_dedup(&mut self.entrypoints);
        sort_and_dedup(&mut self.files);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactModuleContribution {
    pub id: String,
    pub package_name: Option<String>,
    pub chunks: Vec<String>,
    pub bytes: usize,
}

impl ArtifactModuleContribution {
    pub fn normalize(&mut self) {
        sort_and_dedup(&mut self.chunks);
    }
}

fn sort_and_dedup(values: &mut Vec<String>) {
    values.sort_unstable();
    values.dedup();
}
