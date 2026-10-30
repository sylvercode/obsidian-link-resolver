# Data Model: MCP Server Info Tool

**Feature**: [MCP Server Info Tool](spec.md)  
**Date**: 2026-10-29

All entities are in-memory, read-only, and serialized with `serde` into the tool's structured content. There is no persistence and no state transition.

## ServerInfo

The complete response of `get_server_info`.

| Field | Type | Required | Rules |
|---|---|---:|---|
| `version` | string | yes | Semantic version of the MCP server build (`MAJOR.MINOR.PATCH`, optional pre-release/build suffix). Sourced from the crate's `CARGO_PKG_VERSION`; identical to the handshake `serverInfo.version`. |
| `vaults` | array of `ConfiguredVault` | yes | Ordered as supplied at startup, after canonicalization and de-duplication. Never empty (startup fails without at least one vault). |

**Invariants**

- Deterministic for the lifetime of a process: two calls yield byte-identical JSON (no timestamps, counters, or health data).
- No other fields. Additions in future features are made by adding fields to `ServerInfo` or `ConfiguredVault`, never by changing existing shapes.

## ConfiguredVault

One vault root accepted at startup.

| Field | Type | Required | Rules |
|---|---|---:|---|
| `path` | string | yes | Canonical absolute path exactly as returned by the operating system (`fs::canonicalize`): symlinks and `.`/`..` resolved; Windows extended-length prefix `\\?\` preserved; no trailing-separator or case rewriting. Non-ASCII and whitespace preserved. |

**Invariants**

- Each distinct canonical path appears at most once in `ServerInfo.vaults`.
- Represents startup configuration, not live state: a vault removed after startup is still listed.

## Startup De-duplication Rule (derived from FR-002)

Input: the ordered `--vault` arguments.

1. Canonicalize each argument; fail startup if any cannot be accessed or is not a directory (existing behavior).
2. Walk the canonical list in order; keep an entry only if no earlier kept entry is equal to it.
3. The resulting list is `ResolverServer::allowed_vaults` and the exact source of `ServerInfo.vaults`.

Example: `--vault /v/a --vault /v/b --vault /v/a --vault /v/link-to-a` (where `link-to-a` is a symlink to `a`) yields `[{"path":"/v/a"},{"path":"/v/b"}]`.

## Handshake Identity (derived from FR-008)

Not a tool entity, but a related contract value emitted in the MCP `initialize` result:

| Field | Value |
|---|---|
| `serverInfo.name` | `obsidian-link-resolver-mcp` |
| `serverInfo.version` | Same string as `ServerInfo.version` |

## Relationship to Existing Entities

- `ServerInfo.vaults` is a projection of `ResolverServer.allowed_vaults: Vec<PathBuf>`; no new runtime state is introduced.
- The `VaultCache` and resolver `Vault` index are not read, so no indexing side effect occurs.
- `ResolveRequest` and `LinkGenerationRequest` are unchanged.
