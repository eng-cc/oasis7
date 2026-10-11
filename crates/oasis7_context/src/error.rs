use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, Clone, Serialize)]
pub struct ContextError {
    pub code: String,
    pub message: String,
    pub effect: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_path: Option<PathBuf>,
}
impl ContextError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            effect: "none".into(),
            current_revision: None,
            draft_path: None,
        }
    }
    pub fn exit_code(&self) -> i32 {
        match self.code.as_str() {
            "CONTEXT_NOT_CONFIGURED" | "ENTRY_NOT_FOUND" => 3,
            "REVISION_CONFLICT" | "ALREADY_EXISTS" | "CURSOR_STALE" => 4,
            "UNSAFE_PATH" | "PERMISSION_DENIED" | "PLATFORM_UNSUPPORTED" => 5,
            "LOCK_BUSY" => 6,
            "LIMIT_EXCEEDED" => 7,
            "SNAPSHOT_INVALID" | "CRYPTO_ERROR" => 8,
            c if c.starts_with("GIT_") || c.starts_with("HISTORY_") => 9,
            "PERSISTENCE_UNCERTAIN" => 10,
            "IO_ERROR" | "INIT_INCOMPLETE" | "EDITOR_FAILED" => 11,
            _ => 2,
        }
    }
}
impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ContextError {}
impl From<std::io::Error> for ContextError {
    fn from(e: std::io::Error) -> Self {
        Self::new(
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                "PERMISSION_DENIED"
            } else {
                "IO_ERROR"
            },
            "filesystem operation failed",
        )
    }
}
pub type Result<T> = std::result::Result<T, ContextError>;
