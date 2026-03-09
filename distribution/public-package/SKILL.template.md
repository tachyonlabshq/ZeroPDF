# ZeroPDF Skill API Contract

## Scope

This document defines the stable contract for using ZeroPDF as an AI skill through CLI and MCP.

## Version Contract

- Contract version: `{{CONTRACT_VERSION}}`
- Tool schema version: `{{TOOL_SCHEMA_VERSION}}`
- Minimum compatible tool schema version: `{{MIN_COMPATIBLE_VERSION}}`
- Agent task state version: `{{TASK_STATE_VERSION}}`
- Compatibility model: additive fields in minor versions, breaking changes only in major versions.

## Stable Commands

- `inspect`
- `extract-text`
- `search-text`
- `extract-region`
- `add-note`
- `highlight-text`
- `scan-agent-comments`
- `sync-agent-tasks`
- `set-agent-task-status`
- `resolve-agent-task-context`
- `doctor`
- `init`
- `schema-info`
- `skill-api-contract`
- `mcp-stdio`

## Stable MCP Tools

- `inspect_pdf`
- `extract_text`
- `search_text`
- `extract_region`
- `add_note`
- `highlight_text`
- `scan_agent_comments`
- `sync_agent_tasks`
- `update_agent_task_status`
- `resolve_agent_task_context`
- `doctor_environment`
- `schema_info`
- `skill_api_contract`

## Operational Notes

- Page numbers are 1-based in the public contract.
- Bounding boxes in responses use a top-left origin.
- Annotation writes convert response coordinates back to native PDF coordinates.
- `scan-agent-comments` treats both `@agent` and `@Agent` as task triggers.
- Highlight tasks prefer the highlighted region as context and note-like annotations fall back to nearby text windows.

## Publish Notes

- This package ships compiled binaries.
- Export this file from the main `ZeroPDF` repository instead of editing it directly in the public bundle.
