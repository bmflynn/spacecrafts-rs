#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("json syntax: {0}")]
    SerdeJSON(#[from] serde_json::Error),
    #[error("invalid: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
