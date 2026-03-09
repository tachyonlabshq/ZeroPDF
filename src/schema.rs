use serde::Serialize;

pub const TOOL_SCHEMA_VERSION: &str = "1.0.0";
pub const MIN_COMPATIBLE_TOOL_SCHEMA_VERSION: &str = "1.0.0";
pub const AGENT_TASK_STATE_VERSION: &str = "1.0";
pub const SKILL_API_CONTRACT_VERSION: &str = "2026.03";

#[derive(Debug, Clone, Serialize)]
pub struct SchemaInfo {
    pub status: String,
    pub tool_schema_version: String,
    pub minimum_compatible_tool_schema_version: String,
    pub agent_task_state_version: String,
    pub skill_api_contract_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillApiContract {
    pub status: String,
    pub contract_version: String,
    pub tool_schema_version: String,
    pub minimum_compatible_tool_schema_version: String,
    pub agent_task_state_version: String,
    pub stable_commands: Vec<String>,
    pub stable_mcp_tools: Vec<String>,
    pub operational_notes: Vec<String>,
}

pub fn schema_info() -> SchemaInfo {
    SchemaInfo {
        status: "success".to_string(),
        tool_schema_version: TOOL_SCHEMA_VERSION.to_string(),
        minimum_compatible_tool_schema_version: MIN_COMPATIBLE_TOOL_SCHEMA_VERSION.to_string(),
        agent_task_state_version: AGENT_TASK_STATE_VERSION.to_string(),
        skill_api_contract_version: SKILL_API_CONTRACT_VERSION.to_string(),
    }
}

pub fn skill_api_contract() -> SkillApiContract {
    SkillApiContract {
        status: "success".to_string(),
        contract_version: SKILL_API_CONTRACT_VERSION.to_string(),
        tool_schema_version: TOOL_SCHEMA_VERSION.to_string(),
        minimum_compatible_tool_schema_version:
            MIN_COMPATIBLE_TOOL_SCHEMA_VERSION.to_string(),
        agent_task_state_version: AGENT_TASK_STATE_VERSION.to_string(),
        stable_commands: vec![
            "inspect".to_string(),
            "extract-text".to_string(),
            "search-text".to_string(),
            "extract-region".to_string(),
            "add-note".to_string(),
            "highlight-text".to_string(),
            "scan-agent-comments".to_string(),
            "sync-agent-tasks".to_string(),
            "set-agent-task-status".to_string(),
            "resolve-agent-task-context".to_string(),
            "doctor".to_string(),
            "init".to_string(),
            "schema-info".to_string(),
            "skill-api-contract".to_string(),
            "mcp-stdio".to_string(),
        ],
        stable_mcp_tools: vec![
            "inspect_pdf".to_string(),
            "extract_text".to_string(),
            "search_text".to_string(),
            "extract_region".to_string(),
            "add_note".to_string(),
            "highlight_text".to_string(),
            "scan_agent_comments".to_string(),
            "sync_agent_tasks".to_string(),
            "update_agent_task_status".to_string(),
            "resolve_agent_task_context".to_string(),
            "doctor_environment".to_string(),
            "schema_info".to_string(),
            "skill_api_contract".to_string(),
        ],
        operational_notes: vec![
            "Page numbers are 1-based in the public CLI and MCP contract.".to_string(),
            "Bounding boxes use a top-left origin in extracted responses; annotation writes are converted back to PDF coordinates internally.".to_string(),
            "scan-agent-comments detects @agent and @Agent in standard PDF annotation contents.".to_string(),
            "Highlight-driven tasks prefer text intersecting the highlighted region; note-style annotations fall back to nearby page text windows.".to_string(),
            "Task state is persisted in a sidecar JSON file so repeated scans do not mutate the source PDF.".to_string(),
        ],
    }
}
