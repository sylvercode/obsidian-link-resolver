# Quickstart & Validation: MCP Wrapper

This guide validates the MCP server end-to-end. It references the [MCP contract](contracts/mcp.md), [data model](data-model.md), and the existing [resolver result schema](../001-link-resolver/contracts/result.schema.json).

## Prerequisites

- Rust toolchain 1.98.1+ and this repository checkout.
- A fixture or local Obsidian vault containing a `.obsidian/` directory and markdown notes.
- For interactive validation, GitHub Copilot in VS Code or Claude Desktop with local stdio-server support.

The devcontainer and CI/release toolchains must include any dependencies added by the implementation.

## Client Validation Record

Validate against the latest stable release of each supported client available when validation is performed. Record the validation date, exact client version, operating system, and result for each configuration in this file. A later client release does not invalidate or rewrite an existing result; rerun validation and add an updated record when confirming support for that release.

| Client | Exact version | Validation date | Operating system | Result |
|---|---|---|---|---|
| GitHub Copilot in VS Code | VS Code 1.141.2; Copilot extension not detected | 2026-10-14 | Debian GNU/Linux 13 dev container | Not validated: `code --list-extensions --show-versions` did not report GitHub Copilot; no interactive discovery or tool call was performed. |
| Claude Desktop | Unavailable in this environment | 2026-10-14 | Debian GNU/Linux 13 dev container | Not validated: Claude Desktop is not installed or available; no interactive discovery or tool call was performed. |

These are environment-validation notes, not successful client-compatibility claims. Replace them with successful results that include exact client and extension versions after validating each client.

## Install / Update

The dedicated MCP installer installs only `obsidian-link-resolver-mcp`.

Install or update the latest release with Bash, or pin a release version:

```bash
curl -fsSL https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.sh | bash
curl -fsSL https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.sh | bash -s -- --version 2.0.0
```

For a manual fallback, inspect and run the downloaded script:

```bash
curl -fsSLO https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.sh
bash install-mcp.sh --version 2.0.0 --install-dir "$HOME/.local/bin"
```

PowerShell 7+ supports the same latest, pinned, and manual installation paths:

```powershell
irm https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.ps1 | iex
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.ps1))) -Version 2.0.0
iwr https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.ps1 -OutFile install-mcp.ps1
pwsh ./install-mcp.ps1 -Version 2.0.0 -InstallDir "$HOME/.local/bin"
```

The existing `install.sh` and `install.ps1` remain CLI-only. Neither installer accepts a component-selection parameter.

## Build and Run

User-facing installation instructions for the published binary must match the existing README's latest-version install/update, pinned-version, and manual-fallback patterns. Provide separate parameterless MCP installer commands alongside the existing CLI installer, preserving its CLI-only behavior, then show local-stdio registration for GitHub Copilot in VS Code and Claude Desktop.

```bash
cargo build --release -p obsidian-link-resolver-mcp
target/release/obsidian-link-resolver-mcp --help
```

Register the installed executable in the client's standard local stdio-server configuration. The command is `obsidian-link-resolver-mcp`; pass `--vault /absolute/path/to/vault` to configure an allowed vault root, repeating `--vault` for each permitted vault. At least one root is required. A per-call `vault_root` can select only one of those configured roots, and `context_path` must be inside the selected root. The process uses stdin/stdout for MCP only. Do not wrap or redirect its stdout.

## Tool Scenarios

The scenarios below are the normative MCP conformance corpus for SC-001, SC-003, SC-006, and SC-008. Use `tests/fixtures/vault/` for repeatable cases, with `context_path` set to an existing note in that vault; include a fixture case for each listed link/outcome scenario and compare resolver outcomes with the underlying resolver for identical inputs. Test tool discovery and a valid call in the documented GitHub Copilot in VS Code and Claude Desktop configurations (2/2 configurations). Record exact client versions, validation date, operating system, and result in the Client Validation Record above.

| Scenario | Request | Expected result |
|---|---|---|
| Discover tool | MCP `tools/list` | One `resolve_obsidian_link` tool with required link/context fields and emplacement guidance. |
| Resolve heading | `link: "[[Project Plan#Milestones]]"` | Same outcome and every returned result field as `obsidian-link-resolver` for identical inputs, including `target_path` and range when present. |
| Path-qualified reference | Link containing a vault-relative note path | Same outcome and path interpretation as the underlying resolver on each supported release target. |
| Vault-relative context | `context_path` is a vault-relative path to an existing note | Same outcome and path interpretation as the underlying resolver on each supported release target. |
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
| Determinism boundary | Repeat an identical call while fixture contents are unchanged, then mutate a relevant note and refresh the cache | Primary result records are identical before the mutation; a later result may reflect the changed vault contents after invalidation or periodic refresh. |
| Contract compatibility | Validate the proposed public MCP contract under canonical requirement FR-012 and success criterion SC-012 | The compatibility check passes within the same shared package major version; any breaking change requires a new major version and migration notes. The first published MCP contract establishes the baseline if no earlier one exists. |

## Automated Validation

Run the focused MCP tests first during implementation, then the repository gates:

```bash
cargo test -p obsidian-link-resolver-mcp
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo bench -p obsidian-link-resolver-mcp --bench mcp_warm
cargo bench -p obsidian-link-resolver-mcp --bench mcp_cold_start
pwsh -NoProfile -File scripts/check-mcp-contract-compatibility.ps1 specs/002-mcp-wrapper/contracts/mcp-contract.json published MIGRATION.md
```

The focused suite verifies tool discovery/input schema, stdio initialize/list/call, structured and text result parity, all five resolver statuses, path-qualified and vault-relative context path parity, cache reuse, event invalidation, request-independent periodic refresh, refresh failure behavior, separation between vault roots, and diagnostics default/redaction/channel isolation. Run path parity fixtures natively on `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-gnu`, configuring the MinGW linker for the Windows GNU target. Compare outcome categories for every case and compare target/location fields only when present in the direct resolver result. Determinism tests hold vault contents fixed; mutation tests separately verify that cache freshness can legitimately change subsequent results. Contract compatibility validation follows canonical requirement FR-012 and success criterion SC-012. Use child-process tests to capture stderr and stdout because the `log` facade logger is process-global; verify unrelated targets are sent to the configured sink and emitted diagnostics match the allow-listed schema in `contracts/mcp.md`. Filesystem-event correctness tests should inject invalidation deterministically; the periodic scheduler should use a controllable clock so the 60-second bound can be tested without a wall-clock delay. The cross-platform watcher smoke test checks registration without depending on event timing.

The warm-run benchmark is a performance gate in both pull-request CI and tag-release validation. Both workflows run it on GitHub-hosted `ubuntu-24.04`; it measures 100 consecutive end-to-end MCP calls against the deterministic, generated `bench-vault-v1` corpus from `tests/fixtures/bench_vault.rs`, excluding process startup and initial index construction. The p50 must be ≤100 ms. The benchmark emits a machine-readable report with runner image/OS/architecture, fixture version, sample count, and p50; the workflow retains that report as an artifact. Run the same command locally for a diagnostic measurement, but interpret the enforced threshold on the named CI/release runner.

The cold-start benchmark measures 100 fresh processes from launch through the first successful resolution, including startup and initial indexing. It records the p50 as a baseline without a failure threshold. Both benchmarks emit machine-readable reports containing runner image, OS, architecture, fixture version, sample count, and p50; CI and release workflows retain both reports as artifacts.

The compatibility command compares the candidate with the latest published MCP contract in the same major version. It requires an authenticated GitHub CLI session and repository access when using `published`. If no published MCP contract asset exists, the candidate establishes the initial baseline; breaking changes in later releases require a new major version and matching migration notes in `MIGRATION.md`.

The automated end-to-end scenarios are exercised by the MCP stdio, resolver-parity, cache-refresh, diagnostics, installer, and release-asset integration tests, along with the contract and determinism tests. CI runs the mocked installer integration tests on `ubuntu-24.04`; injected platform probes cover the supported asset mappings, but this does not test native shell or filesystem behavior on macOS or Windows. CI does not claim native installer coverage on those platforms. Interactive validation in the two supported clients remains outstanding as recorded above.

## Shared-Version Release Validation

The release model is defined in the [shared-version release contract](contracts/releases.md). Validate the workspace and shared-tag flow:

```bash
cargo test --workspace
cargo build --release -p obsidian-link-resolver-cli
cargo build --release -p obsidian-link-resolver-mcp
cargo build --release -p obsidian-link-resolver
```

Every `vX.Y.Z` tag release must run the full workspace test suite and MCP warm-run benchmark before publishing distinct library/FFI, CLI, and MCP asset families under the same tag. Run both gates on the same GitHub-hosted `ubuntu-24.04` runner label as pull-request CI. The benchmark measures 100 consecutive end-to-end warm-cache calls on `bench-vault-v1` (approximately 5,000 notes), excluding startup and initial indexing; emit and retain runner image/OS/architecture, fixture version, sample count, and p50, and enforce p50 ≤100 ms. A failed gate must prevent publication of all assets. Exercise latest and pinned installation using the existing CLI installer and the separate MCP installer; verify that each installs only its corresponding executable and that the MCP installer requires no component-selection parameter. Library/FFI assets are distributed separately. Windows ARM64 is not in the current release matrix.
