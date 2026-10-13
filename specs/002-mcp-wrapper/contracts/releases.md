# Shared-Version Release Contract

This contract defines one versioned GitHub release containing distinct assets for the library/FFI, CLI binary, and MCP server.

## Version and Trigger

- The Cargo workspace version (`[workspace.package.version]`) is authoritative for the release.
- A valid `vX.Y.Z` tag matching that workspace version triggers the release workflow.
- All assets published for the tag use the same synchronized semantic version.
- Crates are organized as separate workspace members under `crates/`, but release together under this single `vX.Y.Z` tag. No per-component release tags are used.

## Release Gates and Assets

Before staging or publishing any component assets, the workflow MUST pass all four release gates:

1. **Workspace tests**: Run the full workspace test suite (`cargo test --workspace`).
2. **Warm-run performance**: Run the MCP warm-run benchmark and enforce its p50 threshold.
3. **Cold-start baseline (SC-014)**: Run the MCP cold-start benchmark for 100 fresh server processes and produce the required machine-readable report. Until a latency budget is established, this gate records a baseline without a p50 failure threshold.
4. **Contract compatibility (FR-012)**: Validate the MCP contract against the latest published contract in the same major version; breaking changes MUST use a new major version and include migration notes.

Failure of any gate MUST prevent staging and publication of every component asset.

After the gates pass, the workflow publishes three distinct component asset families under the same GitHub release:

- **Library/FFI**: generated C header and supported native library artifacts.
- **CLI**: existing `obsidian-link-resolver` binary for supported release targets.
- **MCP**: `obsidian-link-resolver-mcp` binary for supported release targets.

Asset names MUST distinguish component, version/tag, operating system, and architecture as needed. A release MUST NOT replace component-specific assets with one undifferentiated binary or package. GitHub Releases are the publication channel; crates.io publication is not part of this feature.

## Installer Commands

Installers MUST retain latest, pinned-version, and manual-fallback behavior. The existing CLI installer installs only the CLI binary. A separate MCP installer command installs only the MCP server binary and MUST NOT require a component-selection parameter. Library/FFI assets are published separately and are not installed by either executable installer.

The CLI and MCP installer commands resolve the same shared `vX.Y.Z` release and download only their respective executable assets. The MCP install/update path is distinct from the CLI path and MUST NOT change the CLI installer behavior.

## Compatibility and Validation

Existing executable names remain `obsidian-link-resolver` and `obsidian-link-resolver-mcp`. Validation MUST confirm:

1. A valid `vX.Y.Z` tag triggers the release workflow and its full test/performance gates.
2. Library/FFI, CLI, and MCP assets are all published with the same version and distinct names.
3. A failed gate blocks publication of every asset.
4. The CLI installer downloads only the CLI asset, and the separate MCP installer downloads only the MCP asset without a component-selection parameter.
5. Library/FFI assets are published separately and are not installed by either executable installer.
