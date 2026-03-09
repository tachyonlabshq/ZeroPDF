# ZeroPDF Roadmap

## Phase 1 - Foundation and Contract
- [ ] Initialize the Rust workspace as the canonical `ZeroPDF` source repository.
  - Create a single-binary CLI/MCP application with a stable crate layout (`src/lib.rs`, `src/main.rs`, domain modules, tests, scripts, distribution templates).
  - Establish release-oriented build defaults (`lto`, stripped symbols, panic abort) and schema/version constants for downstream agents.
- [ ] Define the stable CLI and MCP contract.
  - Publish JSON-friendly commands and MCP tool names for extraction, annotation, agent-task scanning, task-state sync, and environment diagnostics.
  - Keep argument names deterministic and alias-friendly so OpenCode and other agent runtimes can call tools without custom adapters.
- [ ] Document repository conventions and packaging rules.
  - Make `ZeroPDF` the sole source of truth for source code, docs, release templates, and packaged binaries.
  - Reserve generated install bundles for `distribution/` so release artifacts can be validated and reproduced from the main repo.

## Phase 2 - Core PDF Intelligence
- [ ] Implement document inspection and text extraction.
  - Parse PDF metadata, page geometry, page text, and word-level bounding boxes using a pure Rust-first stack suitable for AI agents.
  - Provide bounded extraction modes so agent calls can request whole pages, page windows, or compact previews without runaway output.
- [ ] Add structured query primitives for downstream agents.
  - Support search-by-text, page slicing, and rectangle/bounding-box queries.
  - Normalize outputs into concise JSON with stable field names for metadata, snippets, and geometric context.
- [ ] Handle malformed and sparse PDFs predictably.
  - Classify corrupted files, encrypted files, empty pages, image-only pages, and missing metadata with actionable error codes.
  - Add safe clipping and fallback messaging so agent loops fail clearly rather than returning ambiguous partial payloads.

## Phase 3 - Annotation-Native Agent Workflow
- [ ] Add write support for standard PDF annotations.
  - Create sticky-note/text annotations and highlight annotations with optional author, color, icon, and page targeting.
  - Preserve existing pages and annotations while appending new agent-facing annotations non-destructively.
- [ ] Build the `@Agent` interaction model.
  - Detect annotation comments that contain `@agent` or `@Agent` and normalize the instruction text for downstream execution.
  - Treat highlight comments as task anchors and use the highlighted region as first-class context.
- [ ] Resolve contextual ranges for annotation-driven tasks.
  - For highlight annotations, return the highlighted text or text intersecting the highlight bounding region.
  - For sticky notes and other note-like annotations, return the text within or nearest the annotation rectangle, plus page-local context windows.
- [ ] Persist task state outside the PDF.
  - Add sidecar task-state sync so repeated scans preserve status transitions (`pending`, `running`, `done`, `error`) without mutating the source PDF.
  - Expose task-context resolution APIs that combine live annotation data with persisted state.

## Phase 4 - Agent Integration and MCP
- [ ] Implement an MCP stdio adapter tuned for OpenCode.
  - Expose the stable tool list and structured error responses through JSON-RPC.
  - Keep tool outputs concise, schema-stable, and compatible with other agents that expect local MCP servers.
- [ ] Add project bootstrap and environment doctor commands.
  - Generate a local `opencode.json` MCP entry and project-scoped helper files for easier install/configuration.
  - Validate binary pathing, writable temp/output directories, and optional helper/runtime dependencies.
- [ ] Publish the skill contract.
  - Document compatible commands, MCP tools, schema versions, and operational notes in a compact `SKILL.md` suitable for packaged distribution.

## Phase 5 - Quality, Fixtures, and Edge Cases
- [ ] Build a representative PDF fixture corpus.
  - Include PDFs with native text, multi-page layout, image-only pages, existing annotations, overlapping highlights, empty comments, and malformed annotation payloads.
  - Add fixtures that explicitly exercise `@agent` notes, sticky notes, highlight comments, and mixed-case triggers.
- [ ] Add regression and golden tests.
  - Verify extraction, annotation creation, task scanning, task-state sync, and MCP tool behavior against stable snapshots.
  - Cover edge cases such as duplicate matches, multi-line highlights, missing quad points, and comments with no nearby text.
- [ ] Add integration checks for packaging and distribution.
  - Ensure export scripts produce a complete public package with docs, config, scripts, and binaries.
  - Validate generated release manifests and checksums.

## Phase 6 - Security and Hardening
- [ ] Add security policy gates.
  - Run `cargo audit`, `cargo deny`, clippy, formatting, and test suites in a repeatable local/CI flow.
  - Review dependency choices for unsafe surface area and minimize nonessential runtime dependencies.
- [ ] Harden file handling and annotation writes.
  - Validate page indices, bounding boxes, output paths, and user-supplied strings before mutating PDFs.
  - Ensure annotation insertion cannot silently corrupt the original document structure.
- [ ] Prepare operational security docs.
  - Document artifact checksums, release verification, and recommended local execution posture for agent hosts.

## Phase 7 - Distribution and Release Automation
- [ ] Add export tooling for install-ready public bundles.
  - Materialize `README.md`, `SKILL.md`, `mcp.json`, `bin/`, and any helper assets directly from the main repo.
  - Keep templates version-driven so public-package output cannot drift from the canonical source.
- [ ] Build multi-platform binaries and release archives.
  - Package macOS ARM64, macOS x64, Windows x64, and Windows ARM64 binaries when toolchains are available.
  - Emit zip/tar archives, checksum files, and a machine-readable manifest from the main repo.
- [ ] Prepare GitHub release readiness.
  - Add workflows/docs for artifact assembly and verification.
  - Finalize repository metadata, changelog, and release notes so `tachyonlabshq/ZeroPDF` is publishable without a second mirror repo.
