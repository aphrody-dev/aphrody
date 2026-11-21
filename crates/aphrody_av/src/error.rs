use std::fmt;

/// Every failure of the crate. Unsupported combinations name the exact codec, container or option
/// that is refused, never a generic failure.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum AvError {
    /// A valid request that this build cannot do (codec, container or option combination).
    Unsupported(String),
    /// The input is not what it claims to be, or is truncated or corrupt.
    Malformed(String),
    /// The request itself is wrong (bad option value, missing stream).
    InvalidArgument(String),
    /// Reading or writing a file failed.
    Io(String),
    /// A path is outside the allowed roots, or a limit was exceeded.
    Denied(String),
}

pub type AvResult<T> = Result<T, AvError>;

impl AvError {
    pub fn unsupported(message: impl Into<String>) -> Self {
        AvError::Unsupported(message.into())
    }
    pub fn malformed(message: impl Into<String>) -> Self {
        AvError::Malformed(message.into())
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        AvError::InvalidArgument(message.into())
    }
    /// Stable numeric code for the C ABI.
    pub fn code(&self) -> i32 {
        match self {
            AvError::Unsupported(_) => 1,
            AvError::Malformed(_) => 2,
            AvError::InvalidArgument(_) => 3,
            AvError::Io(_) => 4,
            AvError::Denied(_) => 5,
        }
    }
    pub fn message(&self) -> &str {
        match self {
            AvError::Unsupported(m)
            | AvError::Malformed(m)
            | AvError::InvalidArgument(m)
            | AvError::Io(m)
            | AvError::Denied(m) => m,
        }
    }
}

impl fmt::Display for AvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            AvError::Unsupported(_) => "unsupported",
            AvError::Malformed(_) => "malformed input",
            AvError::InvalidArgument(_) => "invalid argument",
            AvError::Io(_) => "i/o error",
            AvError::Denied(_) => "denied",
        };
        write!(f, "{label}: {}", self.message())
    }
}

impl std::error::Error for AvError {}

impl From<std::io::Error> for AvError {
    fn from(error: std::io::Error) -> Self {
        AvError::Io(error.to_string())
    }
}
