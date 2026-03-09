use crate::agent_comments::{
    resolve_agent_task_context, scan_agent_comments, set_agent_task_status, sync_agent_tasks,
};
use crate::annotations::{AddNoteRequest, HighlightTextRequest, add_note, highlight_text};
use crate::doctor::doctor_environment;
use crate::errors::{ErrorCode, classify_error};
use crate::pdf_ops::{
    ExtractTextRequest, RegionRequest, extract_region, extract_text, inspect_pdf, search_text,
};
use crate::schema::{schema_info, skill_api_contract};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct McpRequest {
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpErrorPayload>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpErrorPayload {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct McpToolDescriptor {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct ToolsCallParams {
    name: String,
    arguments: Option<Value>,
}

pub fn run_mcp_stdio(pretty: bool) -> Result<()> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let stream = serde_json::Deserializer::from_reader(stdin.lock()).into_iter::<Value>();

    for parsed in stream {
        let response = match parsed {
            Ok(raw) => match serde_json::from_value::<McpRequest>(raw) {
                Ok(request) => handle_mcp_request(request),
                Err(err) => McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(McpErrorPayload {
                        code: -32700,
                        message: format!("invalid MCP request JSON: {err}"),
                        data: None,
                    }),
                },
            },
            Err(err) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(McpErrorPayload {
                    code: -32700,
                    message: format!("invalid MCP request stream: {err}"),
                    data: None,
                }),
            },
        };

        let serialized = if pretty {
            serde_json::to_string_pretty(&response)?
        } else {
            serde_json::to_string(&response)?
        };
        writeln!(stdout, "{serialized}").context("failed to write MCP response")?;
        stdout.flush().context("failed to flush MCP stdout")?;
    }

    Ok(())
}

pub fn handle_mcp_request(request: McpRequest) -> McpResponse {
    let result = match request.method.as_str() {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "zeropdf", "version": "0.1.0" }
        })),
        "notifications/initialized" | "notifications/cancelled" => Ok(json!(null)),
        "tools/list" => Ok(json!({ "tools": list_tools() })),
        "tools/call" => call_tool(request.params.clone()),
        unsupported => Err(anyhow!("unsupported method '{unsupported}'")),
    };

    match result {
        Ok(value) => McpResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(value),
            error: None,
        },
        Err(err) => {
            let diagnostic = classify_error("mcp", &err);
            let code = match diagnostic.code {
                ErrorCode::InvalidArguments
                | ErrorCode::InvalidBounds
                | ErrorCode::InvalidPageNumber => -32602,
                _ => -32000,
            };
            McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpErrorPayload {
                    code,
                    message: diagnostic.message,
                    data: Some(json!({
                        "operation": diagnostic.operation,
                        "hint": diagnostic.hint,
                        "details": diagnostic.details,
                    })),
                }),
            }
        }
    }
}

fn list_tools() -> Vec<McpToolDescriptor> {
    vec![
        McpToolDescriptor {
            name: "inspect_pdf".to_string(),
            description: "Inspect PDF metadata, page sizes, text previews, and annotation counts."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_path"],
                "properties": {
                    "pdf_path": { "type": "string" }
                }
            }),
        },
        McpToolDescriptor {
            name: "extract_text".to_string(),
            description: "Extract bounded page text and optional word boxes from a PDF."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_path"],
                "properties": {
                    "pdf_path": { "type": "string" },
                    "page": { "type": "integer" },
                    "page_start": { "type": "integer" },
                    "page_end": { "type": "integer" },
                    "max_chars_per_page": { "type": "integer" },
                    "include_words": { "type": "boolean" }
                }
            }),
        },
        McpToolDescriptor {
            name: "search_text".to_string(),
            description:
                "Search literal text in a PDF and return page-local bounding boxes with context."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_path", "query"],
                "properties": {
                    "pdf_path": { "type": "string" },
                    "query": { "type": "string" },
                    "page": { "type": "integer" },
                    "limit": { "type": "integer" }
                }
            }),
        },
        McpToolDescriptor {
            name: "extract_region".to_string(),
            description: "Extract text within a page bounding box.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_path", "page_number", "left", "top", "right", "bottom"],
                "properties": {
                    "pdf_path": { "type": "string" },
                    "page_number": { "type": "integer" },
                    "left": { "type": "number" },
                    "top": { "type": "number" },
                    "right": { "type": "number" },
                    "bottom": { "type": "number" },
                    "margin": { "type": "number" }
                }
            }),
        },
        McpToolDescriptor {
            name: "add_note".to_string(),
            description: "Add a standard PDF sticky-note annotation to a document copy."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["input_pdf_path", "output_pdf_path", "page_number", "x", "y", "width", "height", "comment"],
                "properties": {
                    "input_pdf_path": { "type": "string" },
                    "output_pdf_path": { "type": "string" },
                    "page_number": { "type": "integer" },
                    "x": { "type": "number" },
                    "y": { "type": "number" },
                    "width": { "type": "number" },
                    "height": { "type": "number" },
                    "comment": { "type": "string" },
                    "author": { "type": "string" }
                }
            }),
        },
        McpToolDescriptor {
            name: "highlight_text".to_string(),
            description: "Find text in a PDF and add a highlight annotation with a comment."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["input_pdf_path", "output_pdf_path", "query", "comment"],
                "properties": {
                    "input_pdf_path": { "type": "string" },
                    "output_pdf_path": { "type": "string" },
                    "query": { "type": "string" },
                    "page_number": { "type": "integer" },
                    "match_index": { "type": "integer" },
                    "comment": { "type": "string" },
                    "author": { "type": "string" }
                }
            }),
        },
        McpToolDescriptor {
            name: "scan_agent_comments".to_string(),
            description: "Parse PDF annotations tagged with @agent and recover page-local context."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_path"],
                "properties": { "pdf_path": { "type": "string" } }
            }),
        },
        McpToolDescriptor {
            name: "sync_agent_tasks".to_string(),
            description: "Persist scanned PDF @agent tasks into a sidecar state file.".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_path"],
                "properties": {
                    "pdf_path": { "type": "string" },
                    "state_path": { "type": "string" }
                }
            }),
        },
        McpToolDescriptor {
            name: "update_agent_task_status".to_string(),
            description: "Update a persisted PDF task state to pending, running, done, or error."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_path", "task_id", "status"],
                "properties": {
                    "pdf_path": { "type": "string" },
                    "task_id": { "type": "string" },
                    "status": { "type": "string" },
                    "note": { "type": "string" },
                    "state_path": { "type": "string" }
                }
            }),
        },
        McpToolDescriptor {
            name: "resolve_agent_task_context".to_string(),
            description:
                "Resolve one PDF @agent task into instruction, bbox, context, and persisted status."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["pdf_path", "task_id"],
                "properties": {
                    "pdf_path": { "type": "string" },
                    "task_id": { "type": "string" },
                    "state_path": { "type": "string" }
                }
            }),
        },
        McpToolDescriptor {
            name: "doctor_environment".to_string(),
            description: "Check basic ZeroPDF runtime prerequisites.".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        McpToolDescriptor {
            name: "schema_info".to_string(),
            description: "Return stable ZeroPDF schema versions.".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        McpToolDescriptor {
            name: "skill_api_contract".to_string(),
            description: "Return the stable ZeroPDF skill contract.".to_string(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
    ]
}

fn call_tool(params: Option<Value>) -> Result<Value> {
    let params = params.ok_or_else(|| anyhow!("tools/call requires params"))?;
    let parsed: ToolsCallParams =
        serde_json::from_value(params).context("failed to parse tools/call params")?;
    let args = parsed.arguments.unwrap_or_else(|| json!({}));
    let args_obj = args
        .as_object()
        .ok_or_else(|| anyhow!("tool arguments must be an object"))?;

    let result = match parsed.name.as_str() {
        "inspect_pdf" => {
            let pdf_path = required_string(args_obj, "pdf_path")?;
            serde_json::to_value(inspect_pdf(pdf_path)?)?
        }
        "extract_text" => {
            let request = ExtractTextRequest {
                page: optional_usize(args_obj, &["page"])?,
                page_start: optional_usize(args_obj, &["page_start"])?,
                page_end: optional_usize(args_obj, &["page_end"])?,
                max_chars_per_page: optional_usize(args_obj, &["max_chars_per_page"])?
                    .unwrap_or(4000),
                include_words: optional_bool(args_obj, &["include_words"]).unwrap_or(false),
            };
            let pdf_path = required_string(args_obj, "pdf_path")?;
            serde_json::to_value(extract_text(pdf_path, &request)?)?
        }
        "search_text" => {
            let pdf_path = required_string(args_obj, "pdf_path")?;
            let query = required_string(args_obj, "query")?;
            serde_json::to_value(search_text(
                pdf_path,
                query,
                optional_usize(args_obj, &["page", "page_number"])?,
                optional_usize(args_obj, &["limit"])?.unwrap_or(10),
            )?)?
        }
        "extract_region" => {
            let pdf_path = required_string(args_obj, "pdf_path")?;
            let page_number = required_usize(args_obj, "page_number")?;
            let region = RegionRequest {
                left: required_f64(args_obj, "left")?,
                top: required_f64(args_obj, "top")?,
                right: required_f64(args_obj, "right")?,
                bottom: required_f64(args_obj, "bottom")?,
            };
            serde_json::to_value(extract_region(
                pdf_path,
                page_number,
                region,
                optional_f64(args_obj, &["margin"]).unwrap_or(0.0),
            )?)?
        }
        "add_note" => {
            let input_pdf_path = required_string(args_obj, "input_pdf_path")?;
            let output_pdf_path = required_string(args_obj, "output_pdf_path")?;
            let request = AddNoteRequest {
                page_number: required_usize(args_obj, "page_number")?,
                x: required_f64(args_obj, "x")?,
                y: required_f64(args_obj, "y")?,
                width: required_f64(args_obj, "width")?,
                height: required_f64(args_obj, "height")?,
                comment: required_string(args_obj, "comment")?.to_string(),
                author: optional_string(args_obj, &["author"]).map(str::to_string),
                color: None,
                icon: None,
            };
            serde_json::to_value(add_note(input_pdf_path, output_pdf_path, &request)?)?
        }
        "highlight_text" => {
            let input_pdf_path = required_string(args_obj, "input_pdf_path")?;
            let output_pdf_path = required_string(args_obj, "output_pdf_path")?;
            let request = HighlightTextRequest {
                page_number: optional_usize(args_obj, &["page_number", "page"])?,
                query: required_string(args_obj, "query")?.to_string(),
                match_index: optional_usize(args_obj, &["match_index"])?.unwrap_or(0),
                comment: required_string(args_obj, "comment")?.to_string(),
                author: optional_string(args_obj, &["author"]).map(str::to_string),
                color: None,
            };
            serde_json::to_value(highlight_text(input_pdf_path, output_pdf_path, &request)?)?
        }
        "scan_agent_comments" => {
            let pdf_path = required_string(args_obj, "pdf_path")?;
            serde_json::to_value(scan_agent_comments(pdf_path)?)?
        }
        "sync_agent_tasks" => {
            let pdf_path = required_string(args_obj, "pdf_path")?;
            let state_path = optional_string(args_obj, &["state_path"]).map(PathBuf::from);
            serde_json::to_value(sync_agent_tasks(pdf_path, state_path.as_deref())?)?
        }
        "update_agent_task_status" => {
            let pdf_path = required_string(args_obj, "pdf_path")?;
            let task_id = required_string(args_obj, "task_id")?;
            let status = required_string(args_obj, "status")?;
            let state_path = optional_string(args_obj, &["state_path"]).map(PathBuf::from);
            let note = optional_string(args_obj, &["note"]).map(str::to_string);
            serde_json::to_value(set_agent_task_status(
                pdf_path,
                task_id,
                status,
                note,
                state_path.as_deref(),
            )?)?
        }
        "resolve_agent_task_context" => {
            let pdf_path = required_string(args_obj, "pdf_path")?;
            let task_id = required_string(args_obj, "task_id")?;
            let state_path = optional_string(args_obj, &["state_path"]).map(PathBuf::from);
            serde_json::to_value(resolve_agent_task_context(
                pdf_path,
                task_id,
                state_path.as_deref(),
            )?)?
        }
        "doctor_environment" => serde_json::to_value(doctor_environment()?)?,
        "schema_info" => serde_json::to_value(schema_info())?,
        "skill_api_contract" => serde_json::to_value(skill_api_contract())?,
        other => return Err(anyhow!("unsupported tool '{other}'")),
    };

    Ok(result)
}

fn required_string<'a>(args: &'a serde_json::Map<String, Value>, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing string argument '{key}'"))
}

fn required_usize(args: &serde_json::Map<String, Value>, key: &str) -> Result<usize> {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .ok_or_else(|| anyhow!("missing integer argument '{key}'"))
}

fn required_f64(args: &serde_json::Map<String, Value>, key: &str) -> Result<f64> {
    args.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("missing numeric argument '{key}'"))
}

fn optional_string<'a>(args: &'a serde_json::Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| args.get(*key).and_then(Value::as_str))
}

fn optional_usize(args: &serde_json::Map<String, Value>, keys: &[&str]) -> Result<Option<usize>> {
    for key in keys {
        if let Some(value) = args.get(*key) {
            return value
                .as_u64()
                .map(|number| Some(number as usize))
                .ok_or_else(|| anyhow!("argument '{key}' must be an integer"));
        }
    }
    Ok(None)
}

fn optional_f64(args: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| args.get(*key).and_then(Value::as_f64))
}

fn optional_bool(args: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| args.get(*key).and_then(Value::as_bool))
}
