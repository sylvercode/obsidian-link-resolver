# Research: Obsidian Link Resolver MCP Wrapper

**Date**: 2026-10-04

## MCP SDK and Transport

**Decision**: Use the official Rust MCP SDK, `rmcp` 3.5.0, for a standalone server binary using stdio transport and one typed resolver tool. Keep enabled SDK features limited to server, tool/schema, and stdio support; do not add an HTTP listener.

**Rationale**: The feature requires a local client-launched server, and MCP stdio transport is designed for a subprocess communicating over stdin/stdout. The SDK supplies protocol framing, lifecycle, tool discovery/calls, and structured tool results, avoiding a project-maintained JSON-RPC implementation. The current library already exposes `resolve_with_vault`, so the adapter can preserve the existing resolver as its source of truth. Keep status outcomes such as unresolved and ambiguous in tool result content rather than converting them into protocol errors.

**Alternatives considered**:
- Hand-implement JSON-RPC over stdio using existing Serde dependencies: fewer dependencies, but transfers protocol framing, lifecycle, compatibility, and schema maintenance to this project.
- Streamable HTTP: appropriate for remote/network services, but outside this local stdio installation requirement and adds transport/security surface.
- A second community SDK: no requirement identified that justifies moving away from the official Rust SDK.

**Compatibility notes**: `rmcp` 3.5.0 declares Rust 1.88 MSRV, below the repository's CI/release Rust 1.98.1 pin. It brings Tokio and schema support; pin through `Cargo.lock`. Verify the selected feature names and stdio lifecycle against supported clients during implementation. The MCP output schema must remain aligned with `specs/001-link-resolver/contracts/result.schema.json`; existing result types currently derive Serde, not JSON Schema.

**Sources**:
- [Official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [`rmcp` 3.5.0 documentation](https://docs.rs/rmcp/3.5.0/rmcp/)
- [MCP tools specification](https://modelcontextprotocol.io/specification/2026-07-28/server/tools)
- [MCP transport specification](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports)
- [MCP versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning)

## Vault Index Cache and Freshness

**Decision**: Cache a `Vault` snapshot per canonical vault root for the lifetime of the MCP server process. Watch filesystem changes as invalidation hints, coalesce event bursts, and rebuild before the next resolution after invalidation. Also perform a full rescan every 60 seconds as a missed-event fallback. If watcher setup is unavailable, periodic rescans remain active. If rebuilding fails, return an error instead of resolving from known-stale index data.

**Rationale**: `detect_root` currently canonicalizes and enumerates the vault on each call, while `resolve_with_vault` already accepts a prebuilt index. The root-selection and enumeration boundary can be split so requests can resolve/validate a root without rewalking the vault. A watcher avoids per-request enumeration; a periodic scan bounds staleness when events are lost or unsupported. A root directory timestamp is not sufficient to detect arbitrary nested changes. The resolver reads target note content during each resolution, so edits to note contents do not require rebuilding the name/path index.

**Alternatives considered**:
- Watcher only: efficient but events can be unavailable or lost on network filesystems, WSL-mounted paths, resource-limited systems, and during large event bursts.
- Full rescan on every call: simple and correct but directly violates the repeated-call cache requirement.
- Metadata-only polling: root metadata misses nested changes; robust recursive metadata comparison still requires walking the tree and relies on platform-dependent timestamp behavior.
- Watcher without fallback: lower scan cost but cannot bound stale state after missed notifications.

**Behavior and validation**: Cache keys are canonical absolute roots and context paths outside explicit roots are rejected consistently with current behavior. Test that repeated clean calls reuse an index; create/delete/rename operations become visible after invalidation; periodic refresh recovers from a missed event; separate roots never share entries; and refresh failures do not return stale success. Use deterministic cache invalidation tests rather than timing-sensitive OS event assertions, plus cross-platform watcher smoke coverage. A note-body edit must affect heading/emplacement results without an index rebuild.

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
