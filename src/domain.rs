use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Serialize)]
pub struct Error {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip)]
    pub exit: i32,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>, exit: i32) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            exit,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Self::new("IO_ERROR", "Filesystem operation failed", 7)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::SqliteFailure(ref x, _)
                if x.code == rusqlite::ErrorCode::DatabaseBusy
                    || x.code == rusqlite::ErrorCode::DatabaseLocked =>
            {
                Self::new("INDEX_BUSY", "Database writer is busy", 7)
            }
            _ => Self::new("DB_ERROR", "Database operation failed", 7),
        }
    }
}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        Self::new("INVALID_ARGUMENT", "Invalid JSON data", 2)
    }
}
pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
pub fn id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}
pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub path: String,
    pub file_hash: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    pub parent_symbol_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub file_hash: String,
    pub size_bytes: u64,
    pub language: String,
    pub parse_status: String,
    pub symbols: Vec<Symbol>,
}
pub fn envelope(command: &str, project: Option<&crate::project::Project>, data: Value) -> Value {
    json!({"schema_version":"1.0","command":command,"status":"ok","project_id":project.map(|p| &p.project_id),"workspace_id":project.map(|p| &p.workspace_id),"generation_id":null,"validation":{"mode":"matched","scope":[],"checked_at":chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),"workspace_atomic":false},"coverage":{"status":"complete","reasons":[]},"data":data,"truncation":{"truncated":false,"reasons":[]},"warnings":[],"errors":[]})
}
