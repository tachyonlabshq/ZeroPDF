# ZeroPDF Skill API Contract

## Scope

This document defines the stable contract for using ZeroPDF as an AI skill through CLI and MCP.

## Version Contract

- Contract version: `2026.03`
- Tool schema version: `1.0.0`
- Minimum compatible tool schema version: `1.0.0`
- Agent task state version: `1.0`
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

- Public page numbers are 1-based.
- Extracted bounding boxes use a top-left origin in responses.
- Annotation writes convert those boxes back to PDF coordinates internally.
- `scan-agent-comments` detects both `@agent` and `@Agent` inside standard PDF annotation contents.
- Highlight comments use the highlighted text region as primary context.
- Note-like annotations use the note rectangle plus a nearby text window fallback.
- Task state is persisted in a sidecar JSON file and does not mutate the original PDF.

## Publish Notes

- This repository is the canonical source of truth.
- Public install bundles should be exported from `ZeroPDF` rather than edited by hand.
