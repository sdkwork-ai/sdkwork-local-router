use std::{fmt, io};

pub type LocalApiProxyNativeResult<T> = std::result::Result<T, LocalApiProxyNativeError>;

#[derive(Debug)]
pub enum LocalApiProxyNativeError {
    Io(io::Error),
    Serde(serde_json::Error),
    Sqlite(rusqlite::Error),
    ValidationFailed(String),
    InvalidOperation(String),
    NotFound(String),
    Conflict(String),
    Timeout(String),
    Internal(String),
}

impl fmt::Display for LocalApiProxyNativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Serde(error) => write!(f, "serde error: {error}"),
            Self::Sqlite(error) => write!(f, "sqlite error: {error}"),
            Self::ValidationFailed(reason) => write!(f, "validation failed: {reason}"),
            Self::InvalidOperation(reason) => write!(f, "invalid operation: {reason}"),
            Self::NotFound(resource) => write!(f, "not found: {resource}"),
            Self::Conflict(reason) => write!(f, "conflict: {reason}"),
            Self::Timeout(reason) => write!(f, "timeout: {reason}"),
            Self::Internal(reason) => write!(f, "internal error: {reason}"),
        }
    }
}

impl std::error::Error for LocalApiProxyNativeError {}

impl From<io::Error> for LocalApiProxyNativeError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for LocalApiProxyNativeError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<rusqlite::Error> for LocalApiProxyNativeError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sqlite(value)
    }
}
