# ZeroPDF Skill for OpenCode

ZeroPDF is the public install bundle for the Rust-based PDF skill used by AI agents.

## Current capability set

- PDF inspection, bounded text extraction, and literal search
- Region extraction by page-local bounding box
- Standard sticky-note and highlight annotation writes
- `@agent` parsing from PDF comments and highlight annotations
- Sidecar task-state sync and task-context resolution
- MCP stdio adapter for local agent runtimes

## Contract

- Contract version: `{{CONTRACT_VERSION}}`
- Tool schema version: `{{TOOL_SCHEMA_VERSION}}`
- Minimum compatible tool schema version: `{{MIN_COMPATIBLE_VERSION}}`

## Installation

Clone or download this repository and point OpenCode at the matching binary in `bin/`.

### macOS (Apple Silicon)

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "zeropdf": {
      "type": "local",
      "command": ["/path/to/ZeroPDF/distribution/public-package/bin/zeropdf-macos-arm64", "mcp-stdio"],
      "enabled": true
    }
  }
}
```

### macOS (Intel)

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "zeropdf": {
      "type": "local",
      "command": ["/path/to/ZeroPDF/distribution/public-package/bin/zeropdf-macos-x64", "mcp-stdio"],
      "enabled": true
    }
  }
}
```

### Windows (x64)

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "zeropdf": {
      "type": "local",
      "command": ["C:\\path\\to\\ZeroPDF\\distribution\\public-package\\bin\\zeropdf-windows-x64.exe", "mcp-stdio"],
      "enabled": true
    }
  }
}
```

### Windows (ARM64)

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "zeropdf": {
      "type": "local",
      "command": ["C:\\path\\to\\ZeroPDF\\distribution\\public-package\\bin\\zeropdf-windows-arm64.exe", "mcp-stdio"],
      "enabled": true
    }
  }
}
```

## Binary inventory

- `bin/zeropdf-macos-arm64`
- `bin/zeropdf-macos-x64`
- `bin/zeropdf-windows-x64.exe`
- `bin/zeropdf-windows-arm64.exe`

## Notes

- `scan-agent-comments` is the entry point for annotation-native AI workflows.
- Highlight comments tagged with `@agent` recover the highlighted text as context.
- Sticky notes tagged with `@agent` recover nearby page text as fallback context.
- This package should be exported from the main `ZeroPDF` repo rather than edited manually.
