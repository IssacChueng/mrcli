use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Instant, SystemTime};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    Init,
    QueryList,
    QueryEdit,
    QueryRunning,
    ResultView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    Insert,
    Command,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub default_connection: Option<String>,
    pub connections: Vec<ConnectionProfile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionProfile {
    pub name: String,
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DraftEntry {
    pub file_name: String,
    pub path: PathBuf,
    pub content: String,
    pub modified_at: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ResultKind {
    Query,
    Command,
    Error,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub kind: ResultKind,
    pub elapsed_ms: u128,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub affected_rows: Option<u64>,
    pub error_message: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct PendingExecution {
    pub started_at: Instant,
    pub connection_name: String,
    pub sql: String,
}

pub struct ExecutionHandle {
    pub receiver: Receiver<ExecutionResult>,
}

#[derive(Debug, Clone)]
pub struct DatabaseMetadata {
    pub tables: Vec<TableMetadata>,
}

#[derive(Debug, Clone)]
pub struct TableMetadata {
    pub name: String,
    pub columns: Vec<ColumnMetadata>,
}

#[derive(Debug, Clone)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: String,
}

pub struct MetadataHandle {
    pub receiver: Receiver<Result<DatabaseMetadata, String>>,
}

impl ExecutionResult {
    pub fn error(message: impl Into<String>, elapsed_ms: u128) -> Self {
        Self {
            kind: ResultKind::Error,
            elapsed_ms,
            columns: Vec::new(),
            rows: Vec::new(),
            affected_rows: None,
            error_message: Some(message.into()),
            truncated: false,
        }
    }
}
