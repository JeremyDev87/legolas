use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{error::Result, models::Analysis, LegolasError};

pub fn analyze_project<P: AsRef<Path>>(input_path: P) -> Result<Analysis> {
    let repo_root = workspace_root();
    let output = Command::new("node")
        .arg(repo_root.join("bin/legolas.js"))
        .arg("scan")
        .arg(input_path.as_ref())
        .arg("--json")
        .current_dir(&repo_root)
        .output()?;

    if !output.status.success() {
        return Err(LegolasError::CliUsage(extract_error_message(
            &output.stderr,
        )));
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn extract_error_message(stderr: &[u8]) -> String {
    let message = String::from_utf8_lossy(stderr).trim().to_string();
    message
        .strip_prefix("legolas: ")
        .map(ToOwned::to_owned)
        .unwrap_or(message)
}
