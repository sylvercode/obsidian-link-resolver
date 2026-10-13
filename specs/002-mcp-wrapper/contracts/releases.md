# Shared-Version Release Contract

This contract defines one versioned GitHub release containing distinct assets for the library/FFI, CLI binary, and MCP server.

## Version and Trigger

- The Cargo workspace version (`[workspace.package.version]`) is authoritative for the release.
- A valid `vX.Y.Z` tag matching that workspace version triggers the release workflow.
- All assets published for the tag use the same synchronized semantic version.
- Crates are organized as separate workspace members under `crates/`, but release together under this single `vX.Y.Z` tag. No per-component release tags are used.

## Release Gates and Assets

Before publishing, the workflow MUST run the full workspace test suite (`cargo test --workspace`) and the MCP warm-run performance gate. Failure of either gate MUST prevent publication.

After the gates pass, the workflow publishes three distinct component asset families under the same GitHub release:

- **Library/FFI**: generated C header and supported native library artifacts.
- **CLI**: existing `obsidian-link-resolver` binary for supported release targets.
- **MCP**: `obsidian-link-resolver-mcp` binary for supported release targets.

Asset names MUST distinguish component, version/tag, operating system, and architecture as needed. A release MUST NOT replace component-specific assets with one undifferentiated binary or package. GitHub Releases are the publication channel; crates.io publication is not part of this feature.

## Installer Selection

Installers MUST retain latest, pinned-version, and manual-fallback behavior. Users can select the CLI, MCP, or library/FFI component as applicable. The existing default installation remains CLI-only; requesting one component MUST NOT install the other component's executable.

Latest and pinned component selection resolves the same shared `vX.Y.Z` release and downloads only the requested component's asset. The MCP install/update path is distinct from the CLI path and MUST NOT change the CLI installer default.

## Compatibility and Validation

Existing executable names remain `obsidian-link-resolver` and `obsidian-link-resolver-mcp`. Validation MUST confirm:

1. A valid `vX.Y.Z` tag triggers the release workflow and its full test/performance gates.
2. Library/FFI, CLI, and MCP assets are all published with the same version and distinct names.
3. A failed gate blocks publication of every asset.
4. CLI, MCP, and library/FFI installer selection downloads only the selected component's assets.
5. The no-option CLI installer remains CLI-only.
