use std::path::PathBuf;

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

pub fn fixture_path(relative_path: &str) -> PathBuf {
    workspace_root().join(relative_path)
}

pub fn read_oracle(relative_path: &str) -> String {
    std::fs::read_to_string(workspace_root().join("tests/oracles").join(relative_path))
        .expect("read oracle")
}

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

fn to_posix(value: String) -> String {
    value.replace('\\', "/")
}
