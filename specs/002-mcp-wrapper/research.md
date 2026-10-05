# Research: Obsidian Link Resolver MCP Wrapper

**Date**: 2026-10-04
## Optional Operational Diagnostics (User Story 6)

**Decision**: Add a `--diagnostics` launch flag to the MCP server; diagnostics are off unless explicitly requested. When enabled, initialize `structured-logger` 1.0.5 with `default-features = false`, an explicit `INFO` level, its synchronous JSON writer, and the fixed MCP diagnostics target routed to stderr. Route the default writer to `std::io::sink()` so unrelated dependency logs cannot leak to stderr. Do not initialize the global logger when the flag is absent. Use structured key-values for the fixed event names and failure categories defined in [the data model](data-model.md#diagnostic-configuration-and-event) and MCP contract. A resolver `error` outcome is logged as a completed request status, distinct from an MCP/request-processing failure. Use a monotonically increasing process-local request sequence where correlation is needed. Record fixed categories rather than raw error text. Never record link text, context or vault paths, note contents, or tool arguments. Keep diagnostics separate from stdout protocol messages and MCP result objects.

**Rationale**: The spec requires opt-in operational visibility without changing normal results or exposing request content. `structured-logger` supplies a synchronous JSON writer, structured key-value fields, and target-specific writers, avoiding a project-maintained logging format. Explicit target routing and a sink fallback keep logs from MCP/runtime dependencies out of stderr. Disable its default `log-panic` feature because that feature writes panic messages and backtraces, which can contain sensitive data. Initialize only after parsing `--diagnostics` and use static messages/targets plus allow-listed structured values.

**Alternatives considered**:
- Always-on diagnostics: conflicts with the disabled-by-default requirement and adds noise to routine sessions.
- Reuse the CLI's repeatable `-v` flag: less explicit for a long-running server, where one boolean switch cleanly expresses the opt-in requirement.
- Direct `serde_json` plus stderr writes: avoids the `log` facade dependency, but requires custom serialization and write synchronization that the chosen backend already provides.
- A general tracing stack: may be appropriate for larger services, but is not needed for this compact event set.

**Validation**: Use a child-process test because the `log` facade is a process-global singleton. With diagnostics disabled, assert no optional logger records are emitted. With diagnostics enabled, parse stderr JSON-lines and verify lifecycle, request outcomes/failures, and cache outcomes; assert the fixed logger envelope and event schema, and confirm raw request data and paths are absent. Verify no unrelated target reaches stderr. Independently capture stdout and compare MCP protocol responses with diagnostics disabled/enabled to prove response parity and channel isolation. Error records use only a fixed category/stage, not raw errors that may contain paths or note text.

**Sources**:
- [`structured-logger` 1.0.5 crate documentation](https://docs.rs/structured-logger/1.0.5/structured_logger/)
- [Builder configuration](https://docs.rs/structured-logger/1.0.5/structured_logger/struct.Builder.html) and [synchronous JSON writer](https://docs.rs/structured-logger/1.0.5/structured_logger/json/index.html)
- [`structured-logger` 1.0.5 feature flags](https://docs.rs/crate/structured-logger/1.0.5/features)

## MCP SDK and Transport

**Decision**: Use the official Rust MCP SDK, `rmcp` 3.5.0, for a standalone server binary using stdio transport and one typed resolver tool. Keep enabled SDK features limited to server, tool/schema, and stdio support; do not add an HTTP listener.

**Rationale**: The feature requires a local client-launched server, and MCP stdio transport is designed for a subprocess communicating over stdin/stdout. The SDK supplies protocol framing, lifecycle, tool discovery/calls, and structured tool results, avoiding a project-maintained JSON-RPC implementation. The current library already exposes `resolve_with_vault`, so the adapter can preserve the existing resolver as its source of truth. Keep status outcomes such as unresolved and ambiguous in tool result content rather than converting them into protocol errors.

**Alternatives considered**:
- Hand-implement JSON-RPC over stdio using existing Serde dependencies: fewer dependencies, but transfers protocol framing, lifecycle, compatibility, and schema maintenance to this project.
- Streamable HTTP: appropriate for remote/network services, but outside this local stdio installation requirement and adds transport/security surface.
- A second community SDK: no requirement identified that justifies moving away from the official Rust SDK.

**Compatibility notes**: `rmcp` 3.5.0 declares Rust 1.88 MSRV, below the repository's CI/release Rust 1.98.1 pin. Its feature table names `server`, `macros`, `schemars`, and `transport-io` for server tools, typed tool macros, schema generation, and server-side stdio. Do not enable HTTP transport. Pin through `Cargo.lock` and verify the selected feature combination and stdio lifecycle against supported clients during implementation. The MCP output schema must remain aligned with `specs/001-link-resolver/contracts/result.schema.json`; existing result types currently derive Serde, not JSON Schema.

**Sources**:
- [Official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [`rmcp` 3.5.0 documentation](https://docs.rs/rmcp/3.5.0/rmcp/)
- [`rmcp` 3.5.0 feature flags](https://docs.rs/crate/rmcp/3.5.0/features)
- [MCP tools specification](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- [MCP transport specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports)
- [MCP versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)

## Vault Index Cache and Freshness

**Decision**: Cache a `Vault` snapshot per canonical vault root for the lifetime of the MCP server process. Watch filesystem changes as invalidation hints, coalesce event bursts, and rebuild before the next resolution after invalidation. Also run a background full rescan for each cached root at least every 60 seconds, independently of request arrival, as a missed-event fallback. If watcher setup is unavailable, periodic rescans remain active. If rebuilding fails, return an error instead of resolving from known-stale index data.

**Rationale**: `detect_root` currently canonicalizes and enumerates the vault on each call, while `resolve_with_vault` already accepts a prebuilt index. The root-selection and enumeration boundary can be split so requests can resolve/validate a root without rewalking the vault. A watcher avoids per-request enumeration; a periodic background scan bounds staleness when events are lost or unsupported, including during idle periods. Checking the deadline only when a request arrives is insufficient: a request just before that deadline could otherwise return stale data. A root directory timestamp is not sufficient to detect arbitrary nested changes. The resolver reads target note content during each resolution, so edits to note contents do not require rebuilding the name/path index.

**Alternatives considered**:
- Watcher only: efficient but events can be unavailable or lost on network filesystems, WSL-mounted paths, resource-limited systems, and during large event bursts.
- Full rescan on every call: simple and correct but directly violates the repeated-call cache requirement.
- Metadata-only polling: root metadata misses nested changes; robust recursive metadata comparison still requires walking the tree and relies on platform-dependent timestamp behavior.
- Watcher without fallback: lower scan cost but cannot bound stale state after missed notifications.
- Expiry checks only on incoming requests: avoids background work, but cannot guarantee the 60-second freshness bound independently of request timing.

**Behavior and validation**: Cache keys are canonical absolute roots and context paths outside explicit roots are rejected consistently with current behavior. Test that repeated clean calls reuse an index; create/delete/rename operations become visible after invalidation; periodic refresh recovers from a missed event; separate roots never share entries; and refresh failures do not return stale success. Use deterministic cache invalidation tests rather than timing-sensitive OS event assertions, plus cross-platform watcher smoke coverage. A note-body edit must affect heading/emplacement results without an index rebuild.

**Portability**: `notify` documents that network filesystems (including some WSL-mounted paths) may emit no events, native watching can be unavailable in Docker-on-macOS environments, FSEvents may not observe unowned files, Linux watcher limits can be exhausted, and large trees can lose events. These limitations make the request-independent periodic scan a correctness mechanism, not an optional optimization. `PollWatcher` is a documented alternative if a later implementation needs event polling for a specific backend, but the periodic full scan already bounds cache staleness without making that backend the default.

**Dependency notes**: `notify` 8.2.0 declares Rust 1.77 MSRV, compatible with the pinned 1.98.1 toolchain. Prefer its platform backend without an additional debouncer dependency; coalesce with a dirty flag. New dependency/tool requirements must be represented in the devcontainer and CI per Constitution VII.

**Sources**:
- [`notify` known problems and portability](https://docs.rs/notify/latest/notify/#known-problems)
- [`notify` 8.2.0 manifest](https://docs.rs/crate/notify/8.2.0/source/Cargo.toml)
- [Rust `Metadata::modified` behavior](https://doc.rust-lang.org/std/fs/struct.Metadata.html#method.modified)
- Repository: [vault index](../../src/vault.rs), [library API](../../src/lib.rs), [resolver](../../src/resolve.rs)

## Distribution and Client Setup

**Decision**: Publish a distinct `obsidian-link-resolver-mcp` executable in the existing GitHub release assets for the current supported targets: Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64. Add explicit `--component mcp` selection to the existing installer flows while retaining CLI-only behavior as the default. Document a client-launched stdio server configuration and vault selection.

**Rationale**: Existing release workflows already cross-compile native executables for those targets, and both installers already select assets by OS/architecture. A separate binary does not disturb the current CLI argument contract. The current PowerShell installer recognizes Windows ARM64, but the release workflow does not build that target; do not imply that MCP Windows ARM64 is supported until an artifact exists.

**Alternatives considered**:
- Add a server mode to the current CLI executable: risks changing required positional arguments and the one-shot CLI contract.
- Require source builds: conflicts with the requirement for ordinary users to install prebuilt binaries.
- Introduce a package-manager launcher: adds a distribution/runtime path when existing native release/install mechanisms are available.

**Implementation implications**: Extend `Cargo.toml`, CI, tag-release tests/builds/staging, both installers, and README client configuration docs. Release tests and the existing benchmark gate must pass before MCP assets are staged/published. Installer defaults remain compatible; MCP installation is explicitly selectable and installs the MCP executable name required by client configuration.

**Sources**: Repository [release workflow](../../.github/workflows/release.yml), [CI workflow](../../.github/workflows/ci.yml), [Bash installer](../../scripts/install.sh), [PowerShell installer](../../scripts/install.ps1), and [README](../../README.md).
