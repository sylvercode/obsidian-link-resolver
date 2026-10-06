# Quickstart & Validation: MCP Wrapper

This guide validates the MCP server end-to-end. It references the [MCP contract](contracts/mcp.md), [data model](data-model.md), and the existing [resolver result schema](../001-link-resolver/contracts/result.schema.json).

## Prerequisites

- Rust toolchain 1.98.1+ and this repository checkout.
- A fixture or local Obsidian vault containing a `.obsidian/` directory and markdown notes.
- For interactive validation, GitHub Copilot in VS Code or Claude Desktop with local stdio-server support.

The devcontainer and CI/release toolchains must include any dependencies added by the implementation.

## Build and Run

User-facing installation instructions for the published binary must match the existing README's latest-version install/update, pinned-version, and manual-fallback patterns. Provide separate MCP installer commands alongside the existing CLI installer, preserving its CLI-only default, then show local-stdio registration for GitHub Copilot in VS Code and Claude Desktop.

```bash
cargo build --release --bin obsidian-link-resolver-mcp
target/release/obsidian-link-resolver-mcp --help
```

Register the installed executable in the client's standard local stdio-server configuration. The command is `obsidian-link-resolver-mcp`; optionally pass `--vault /absolute/path/to/vault` as a default. The process uses stdin/stdout for MCP only. Do not wrap or redirect its stdout. A per-call `vault_root` overrides the launch default.

## Tool Scenarios

The scenarios below are the normative MCP conformance corpus for SC-001, SC-003, SC-006, and SC-009. Use `tests/fixtures/vault/` for repeatable cases, with `context_path` set to an existing note in that vault; include a fixture case for each listed link/outcome scenario and compare resolver outcomes with the underlying resolver for identical inputs. Test tool discovery and a valid call in the documented GitHub Copilot in VS Code and Claude Desktop configurations (2/2 configurations).

| Scenario | Request | Expected result |
|---|---|---|
| Discover tool | MCP `tools/list` | One `resolve_obsidian_link` tool with required link/context fields and emplacement guidance. |
| Resolve heading | `link: "[[Project Plan#Milestones]]"` | Same `target_path`, range, and `resolved` status as `obsidian-link-resolver` for identical inputs. |
| Same-file link | `link: "[[#Overview]]"` | Resolves relative to `context_path`. |
| Resolve block | Link to a fixture block ID | Same block target and location as the underlying resolver for identical inputs. |
| Emplacement | Same note link with `with_emplacement: true` | Existing heading stack, section range, and structured block fields where applicable. |
| Unresolved | Link to a missing note | Normal tool result with the existing unresolved status and reason fields, not a protocol error. |
| Missing sub-target | Link to a missing heading or block in an existing note | Normal tool result with the existing sub-target-not-found status and reason fields, not a protocol error. |
| Ambiguous | Link to a duplicated note name | Normal tool result with the existing ambiguous status and candidate fields, not a protocol error. |
| Resolver error | Deterministic resolver failure case | Normal tool result preserving the resolver's error outcome and reason fields. |
| Attachment | Link to a fixture attachment | Resolves according to existing attachment semantics without emplacement. |
| Cache reuse | Call repeatedly against one vault | One index is reused; no full directory enumeration per call. |
| Cache refresh | Add, remove, or rename a note | A watcher invalidates before the next resolution; with the watcher event suppressed, a deterministic timer test verifies the background full scan refreshes the index within 60 seconds even when no requests arrive during the interval. |
| Multiple vaults | Resolve with two canonical roots in one process | Each root uses its own cache entry; the cache corpus also covers same-root reuse, refresh, and invalidation during an in-flight resolution. |
| Change during resolution | Invalidate a vault root after a resolution starts and before it completes | The in-flight resolution may complete using its selected index generation; the next resolution for that root refreshes first and uses the latest state available at refresh time. |
| Diagnostics disabled | Start the server without `--diagnostics`; initialize and make successful, unresolved, and failing requests | No optional diagnostic records appear on stderr; MCP responses remain the contract-defined results/errors. |
| Diagnostics enabled | Start with `--diagnostics`; exercise server startup, request outcomes/failure, cache reuse and cache refresh | Parse the `structured-logger` JSON-lines on stderr and verify the fixed logger envelope, event names, and failure categories. A resolver `error` is logged as `request_completed` with status `error`. Assert link text, context/vault paths, note contents, serialized arguments, and raw error strings are absent. |
| Protocol isolation | Run equivalent MCP calls with diagnostics off and on while capturing stdout | stdout contains only MCP messages and the same tool result objects in both runs; diagnostics appear only on stderr. |

## Automated Validation

Run the focused MCP tests first during implementation, then the repository gates:

```bash
cargo test mcp
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

The focused suite verifies tool discovery/input schema, stdio initialize/list/call, structured and text result parity, all five resolver statuses, cache reuse, event invalidation, request-independent periodic refresh, refresh failure behavior, separation between vault roots, and diagnostics default/redaction/channel isolation. Use child-process tests to capture stderr and stdout because the `log` facade logger is process-global; verify unrelated targets are sent to the configured sink. Filesystem-event correctness tests should inject invalidation deterministically; the periodic scheduler should use a controllable clock so the 60-second bound can be tested without a wall-clock delay. The cross-platform watcher smoke test checks registration without depending on event timing.

## Release Validation

The tag release workflow must run the full tests and warm-run benchmark gate before building/staging the MCP binary for Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64. The benchmark measures end-to-end latency from tool-call receipt to response completion for 100 consecutive warm-cache calls on the approximately 5,000-note fixture, excluding startup and initial index construction; record the runner and fixture version and enforce the ≤100 ms p50 target. Verify the MCP installer commands refer to the MCP release asset and that the existing CLI installer still installs only the CLI by default. Windows ARM64 is not in the current release matrix.
