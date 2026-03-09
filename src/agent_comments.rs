use crate::errors::ZeroPdfError;
use crate::pdf_ops::{canonical_pdf_path, clip_text, extract_near_bbox};
use crate::schema::AGENT_TASK_STATE_VERSION;
use anyhow::{Context, Result};
use lopdf::{Document, Object};
use pdfplumber::{AnnotationType, BBox, Pdf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCommentTask {
    pub task_id: String,
    pub task_status: String,
    pub page_number: usize,
    pub annotation_subtype: String,
    pub author: Option<String>,
    pub raw_text: String,
    pub instruction: String,
    pub context_text: String,
    pub bbox: TaskBoundingBox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBoundingBox {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCommentScanReport {
    pub status: String,
    pub pdf_path: String,
    pub scanned_page_count: usize,
    pub task_count: usize,
    pub tasks: Vec<AgentCommentTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStateRecord {
    pub task_id: String,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStateFile {
    pub version: String,
    pub pdf_path: String,
    pub tasks: BTreeMap<String, TaskStateRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncAgentTasksResponse {
    pub status: String,
    pub pdf_path: String,
    pub state_path: String,
    pub task_count: usize,
    pub states: Vec<TaskStateRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentTaskStatusResponse {
    pub status: String,
    pub pdf_path: String,
    pub state_path: String,
    pub task_id: String,
    pub task_status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveAgentTaskContextResponse {
    pub status: String,
    pub pdf_path: String,
    pub task_id: String,
    pub task_status: String,
    pub page_number: usize,
    pub annotation_subtype: String,
    pub instruction: String,
    pub context_text: String,
    pub bbox: TaskBoundingBox,
    pub note: Option<String>,
}

pub fn scan_agent_comments(path: impl AsRef<Path>) -> Result<AgentCommentScanReport> {
    let path = path.as_ref();
    let pdf = Pdf::open_file(path, None).map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
    let mut tasks = Vec::new();

    for page_idx in 0..pdf.page_count() {
        let page = pdf
            .page(page_idx)
            .map_err(|e| ZeroPdfError::Pdf(e.to_string()))?;
        for annot in page.annots() {
            let Some(contents) = annot.contents.as_ref() else {
                continue;
            };
            if !contains_agent_trigger(contents) {
                continue;
            }
            let instruction = normalize_instruction(contents);
            let context_text =
                build_context_for_annotation(path, page_idx + 1, annot.bbox, &annot.annot_type)?;
            tasks.push(AgentCommentTask {
                task_id: build_task_id(
                    path,
                    page_idx + 1,
                    &annot.raw_subtype,
                    annot.bbox,
                    &instruction,
                )?,
                task_status: "pending".to_string(),
                page_number: page_idx + 1,
                annotation_subtype: annot.raw_subtype.clone(),
                author: annot.author.clone(),
                raw_text: contents.clone(),
                instruction,
                context_text: clip_text(&context_text, 1200).0,
                bbox: TaskBoundingBox {
                    left: annot.bbox.x0,
                    top: annot.bbox.top,
                    right: annot.bbox.x1,
                    bottom: annot.bbox.bottom,
                },
            });
        }
    }

    Ok(AgentCommentScanReport {
        status: "success".to_string(),
        pdf_path: path.to_string_lossy().to_string(),
        scanned_page_count: pdf.page_count(),
        task_count: tasks.len(),
        tasks,
    })
}

pub fn sync_agent_tasks(
    path: impl AsRef<Path>,
    state_path: Option<&Path>,
) -> Result<SyncAgentTasksResponse> {
    let path = path.as_ref();
    let scan = scan_agent_comments(path)?;
    let state_path = resolve_state_path(path, state_path)?;
    let mut state = load_state_file(path, &state_path)?;

    for task in &scan.tasks {
        let entry = state
            .tasks
            .entry(task.task_id.clone())
            .or_insert(TaskStateRecord {
                task_id: task.task_id.clone(),
                status: "pending".to_string(),
                note: None,
            });
        if entry.status.trim().is_empty() {
            entry.status = "pending".to_string();
        }
    }

    state
        .tasks
        .retain(|task_id, _| scan.tasks.iter().any(|task| &task.task_id == task_id));
    persist_state_file(&state_path, &state)?;

    Ok(SyncAgentTasksResponse {
        status: "success".to_string(),
        pdf_path: path.to_string_lossy().to_string(),
        state_path: state_path.to_string_lossy().to_string(),
        task_count: state.tasks.len(),
        states: state.tasks.values().cloned().collect(),
    })
}

pub fn set_agent_task_status(
    path: impl AsRef<Path>,
    task_id: &str,
    status: &str,
    note: Option<String>,
    state_path: Option<&Path>,
) -> Result<UpdateAgentTaskStatusResponse> {
    let path = path.as_ref();
    let state_path = resolve_state_path(path, state_path)?;
    let mut state = load_state_file(path, &state_path)?;
    let Some(entry) = state.tasks.get_mut(task_id) else {
        return Err(ZeroPdfError::TaskNotFound {
            task_id: task_id.to_string(),
        }
        .into());
    };
    entry.status = status.to_string();
    entry.note = note.clone();
    persist_state_file(&state_path, &state)?;

    Ok(UpdateAgentTaskStatusResponse {
        status: "success".to_string(),
        pdf_path: path.to_string_lossy().to_string(),
        state_path: state_path.to_string_lossy().to_string(),
        task_id: task_id.to_string(),
        task_status: status.to_string(),
        note,
    })
}

pub fn resolve_agent_task_context(
    path: impl AsRef<Path>,
    task_id: &str,
    state_path: Option<&Path>,
) -> Result<ResolveAgentTaskContextResponse> {
    let path = path.as_ref();
    let scan = scan_agent_comments(path)?;
    let task = scan
        .tasks
        .iter()
        .find(|task| task.task_id == task_id)
        .cloned()
        .ok_or_else(|| ZeroPdfError::TaskNotFound {
            task_id: task_id.to_string(),
        })?;
    let state_path = resolve_state_path(path, state_path)?;
    let state = load_state_file(path, &state_path)?;
    let persisted = state.tasks.get(task_id);

    Ok(ResolveAgentTaskContextResponse {
        status: "success".to_string(),
        pdf_path: path.to_string_lossy().to_string(),
        task_id: task.task_id,
        task_status: persisted
            .map(|entry| entry.status.clone())
            .unwrap_or_else(|| task.task_status.clone()),
        page_number: task.page_number,
        annotation_subtype: task.annotation_subtype,
        instruction: task.instruction,
        context_text: task.context_text,
        bbox: task.bbox,
        note: persisted.and_then(|entry| entry.note.clone()),
    })
}

fn contains_agent_trigger(text: &str) -> bool {
    text.to_ascii_lowercase().contains("@agent")
}

fn normalize_instruction(text: &str) -> String {
    let lowered = text.to_ascii_lowercase();
    if let Some(pos) = lowered.find("@agent") {
        text[pos + "@agent".len()..].trim().to_string()
    } else {
        text.trim().to_string()
    }
}

fn build_context_for_annotation(
    path: &Path,
    page_number: usize,
    bbox: BBox,
    annot_type: &AnnotationType,
) -> Result<String> {
    let padding = match annot_type {
        AnnotationType::Highlight | AnnotationType::Underline | AnnotationType::StrikeOut => 8.0,
        _ => 72.0,
    };
    extract_near_bbox(path, page_number, bbox, padding)
}

fn build_task_id(
    path: &Path,
    page_number: usize,
    subtype: &str,
    bbox: BBox,
    instruction: &str,
) -> Result<String> {
    let canonical = canonical_pdf_path(path)?;
    let mut digest = Sha256::new();
    digest.update(canonical.to_string_lossy().as_bytes());
    digest.update(page_number.to_string().as_bytes());
    digest.update(subtype.as_bytes());
    digest.update(
        format!(
            "{:.2}:{:.2}:{:.2}:{:.2}",
            bbox.x0, bbox.top, bbox.x1, bbox.bottom
        )
        .as_bytes(),
    );
    digest.update(instruction.as_bytes());
    let hash = format!("{:x}", digest.finalize());
    Ok(format!("pdf-task-{}", &hash[..16]))
}

fn resolve_state_path(pdf_path: &Path, state_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = state_path {
        return Ok(path.to_path_buf());
    }
    let canonical = canonical_pdf_path(pdf_path)?;
    let mut digest = Sha256::new();
    digest.update(canonical.to_string_lossy().as_bytes());
    let file_hash = format!("{:x}", digest.finalize());
    Ok(PathBuf::from(".zeropdf")
        .join("agent-tasks")
        .join(format!("{}.json", &file_hash[..16])))
}

fn load_state_file(pdf_path: &Path, state_path: &Path) -> Result<TaskStateFile> {
    if !state_path.exists() {
        return Ok(TaskStateFile {
            version: AGENT_TASK_STATE_VERSION.to_string(),
            pdf_path: pdf_path.to_string_lossy().to_string(),
            tasks: BTreeMap::new(),
        });
    }

    let content = fs::read_to_string(state_path)
        .with_context(|| format!("failed to read {}", state_path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", state_path.display()))
        .map_err(|e| ZeroPdfError::State(e.to_string()).into())
}

fn persist_state_file(state_path: &Path, state: &TaskStateFile) -> Result<()> {
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(state)?;
    fs::write(state_path, format!("{body}\n"))
        .with_context(|| format!("failed to write {}", state_path.display()))?;
    Ok(())
}

#[allow(dead_code)]
fn scan_agent_comments_lopdf(path: impl AsRef<Path>) -> Result<Vec<(usize, String)>> {
    let doc = Document::load(path.as_ref())?;
    let mut output = Vec::new();
    for (page_number, page_id) in doc.get_pages() {
        for annotation in doc.get_page_annotations(page_id)? {
            if let Ok(Object::String(contents, _)) = annotation.get_deref(b"Contents", &doc) {
                output.push((
                    page_number as usize,
                    String::from_utf8_lossy(contents).to_string(),
                ));
            }
        }
    }
    Ok(output)
}
