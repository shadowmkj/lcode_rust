use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Authentication error: Missing session token or CSRF token")]
    Auth,
    #[error("Serialization error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("GraphQL error: {0}")]
    GraphQL(String),
    #[error("System error")]
    System,
    #[error("Other: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
