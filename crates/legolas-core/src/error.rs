use thiserror::Error;

pub type Result<T> = std::result::Result<T, LegolasError>;

#[derive(Debug, Error)]
pub enum LegolasError {
    #[error("path not found: {0}")]
    PathNotFound(String),
    #[error("package.json not found near {0}")]
    PackageJsonMissing(String),
    #[error("{0}")]
    CliUsage(String),
    #[error("unsupported lockfile: {0}")]
    UnsupportedLockfile(String),
    #[error("malformed config {path}: {message}")]
    MalformedConfig { path: String, message: String },
    #[error("unsupported config shape in {path} at {key_path}: {message}")]
    UnsupportedConfigShape {
        path: String,
        key_path: String,
        message: String,
    },
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    JsonParse(#[from] serde_json::Error),
}
