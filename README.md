# ZeroPDF

ZeroPDF is a Rust-based PDF skill and local MCP server for AI agents.

It is designed for OpenCode first and remains compatible with other agent runtimes that can call a local CLI or MCP stdio server. The project is part of the Zero- family of Rust-based office AI tools.

## Core capability set

- Fast PDF inspection and bounded text extraction
- Literal text search with page-local bounding boxes and context
- Region extraction by page and bounding box
- Standard PDF note and highlight annotation writes
- Annotation-native `@agent` workflow parsing
- Sidecar task-state sync for annotation-driven agent tasks
- MCP stdio adapter for OpenCode and similar local agent hosts
- Single canonical repo for source, packaging templates, and release artifacts

## Unique `@agent` workflow

ZeroPDF treats standard PDF annotations as an agent-native interface.

- A sticky note or highlight comment containing `@agent` or `@Agent` becomes a task.
- Highlight annotations use the highlighted text region as contextual scope.
- Note-like annotations use the annotation rectangle plus a nearby text window when no direct text is under the note.
- Task state lives in a sidecar JSON file, so repeated scans preserve status without mutating the original PDF.

## Current commands

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

## Build

```bash
cargo build --release
```

## Usage

Inspect a PDF:

```bash
./target/release/zeropdf inspect ./example.pdf --pretty
```

Extract text:

```bash
./target/release/zeropdf extract-text ./example.pdf --page 1 --max-chars 2500 --pretty
```

Search text and capture context:

```bash
./target/release/zeropdf search-text ./example.pdf "risk factors" --limit 5 --pretty
```

Extract a page region:

```bash
./target/release/zeropdf extract-region ./example.pdf 1 72 120 420 260 --margin 8 --pretty
```

Add a sticky note:

```bash
./target/release/zeropdf add-note ./example.pdf ./example-noted.pdf \
  --page 1 --x 80 --y 90 --width 18 --height 18 \
  --comment "@Agent summarize this paragraph" --author "Reviewer" --pretty
```

Highlight matching text and attach an agent comment:

```bash
./target/release/zeropdf highlight-text ./example.pdf ./example-highlighted.pdf \
  --page 1 --query "Target this sentence" \
  --comment "@Agent rewrite this with a firmer tone" --author "Reviewer" --pretty
```

Scan and resolve agent tasks:

```bash
./target/release/zeropdf scan-agent-comments ./example-highlighted.pdf --pretty
./target/release/zeropdf sync-agent-tasks ./example-highlighted.pdf --pretty
./target/release/zeropdf resolve-agent-task-context ./example-highlighted.pdf pdf-task-1234567890abcdef --pretty
```

Bootstrap a downstream OpenCode project:

```bash
./target/release/zeropdf init /path/to/project --binary /absolute/path/to/zeropdf --pretty
```

## MCP setup

After building, a local OpenCode MCP entry looks like this:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "zeropdf": {
      "type": "local",
      "command": ["/absolute/path/to/zeropdf", "mcp-stdio"],
      "enabled": true
    }
  }
}
```

## Validation and hardening checks

The current local validation flow is:

```bash
cargo check
cargo test
cargo build --release
cargo clippy --all-targets -- -D warnings
cargo audit
cargo deny check
```

## Distribution model

`ZeroPDF` is the source of truth. The same repo contains:

- Rust source under `src/`
- packaging templates under `distribution/public-package/`
- export/release scripts under `scripts/`
- packaged binaries and archives under `distribution/` after running the release tooling

Export an install-ready public bundle into a target directory:

```bash
python3 scripts/export_public_package.py --target-dir distribution/public-package
```

Build zip/tar release archives from the main repo:

```bash
python3 scripts/build_public_package_release.py --output-root distribution/public-package-releases
```
