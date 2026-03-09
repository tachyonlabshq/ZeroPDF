use anyhow::Error;
use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZeroPdfError {
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("invalid page number {page_number}; document has {page_count} pages")]
    InvalidPageNumber {
        page_number: usize,
        page_count: usize,
    },
    #[error("invalid bounding box: {0}")]
    InvalidBounds(String),
    #[error("PDF error: {0}")]
    Pdf(String),
    #[error("annotation write failed: {0}")]
    AnnotationWrite(String),
    #[error("text match '{query}' not found")]
    QueryNotFound { query: String },
    #[error("agent task '{task_id}' not found")]
    TaskNotFound { task_id: String },
    #[error("state file error: {0}")]
    State(String),
    #[error("path error: {0}")]
    Path(String),
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidArguments,
    InvalidPageNumber,
    InvalidBounds,
    InvalidPdf,
    AnnotationWrite,
    QueryNotFound,
    TaskNotFound,
    StateError,
    Io,
    Internal,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub operation: String,
    pub code: ErrorCode,
    pub message: String,
    pub hint: Option<String>,
    pub details: Option<String>,
}

pub fn classify_error(operation: &str, err: &Error) -> Diagnostic {
    if let Some(zero_err) = err.downcast_ref::<ZeroPdfError>() {
        let (code, hint) = match zero_err {
            ZeroPdfError::InvalidArguments(_) => (
                ErrorCode::InvalidArguments,
                Some("check the command arguments and required flags".to_string()),
            ),
            ZeroPdfError::InvalidPageNumber { .. } => (
                ErrorCode::InvalidPageNumber,
                Some("use a 1-based page number within the document page count".to_string()),
            ),
            ZeroPdfError::InvalidBounds(_) => (
                ErrorCode::InvalidBounds,
                Some(
                    "pass bounds as left, top, right, bottom with positive width and height"
                        .to_string(),
                ),
            ),
            ZeroPdfError::Pdf(_) => (
                ErrorCode::InvalidPdf,
                Some(
                    "verify the file is a readable, unencrypted PDF or provide a different source"
                        .to_string(),
                ),
            ),
            ZeroPdfError::AnnotationWrite(_) => (
                ErrorCode::AnnotationWrite,
                Some(
                    "try writing to a new output path and ensure the PDF is not corrupted"
                        .to_string(),
                ),
            ),
            ZeroPdfError::QueryNotFound { .. } => (
                ErrorCode::QueryNotFound,
                Some("adjust the search query or page scope".to_string()),
            ),
            ZeroPdfError::TaskNotFound { .. } => (
                ErrorCode::TaskNotFound,
                Some(
                    "run scan-agent-comments or sync-agent-tasks first to discover valid task ids"
                        .to_string(),
                ),
            ),
            ZeroPdfError::State(_) => (
                ErrorCode::StateError,
                Some("check the sidecar state path and file permissions".to_string()),
            ),
            ZeroPdfError::Path(_) => (
                ErrorCode::Io,
                Some("verify the path exists and is writable".to_string()),
            ),
        };
        return Diagnostic {
            operation: operation.to_string(),
            code,
            message: zero_err.to_string(),
            hint,
            details: None,
        };
    }

    Diagnostic {
        operation: operation.to_string(),
        code: ErrorCode::Internal,
        message: err.to_string(),
        hint: Some("inspect the diagnostic details and retry with a narrower input".to_string()),
        details: Some(format!("{err:?}")),
    }
}

pub fn normalize_path(path: &std::path::Path) -> Result<PathBuf, ZeroPdfError> {
    path.canonicalize()
        .map_err(|e| ZeroPdfError::Path(format!("{} ({})", path.display(), e)))
}
