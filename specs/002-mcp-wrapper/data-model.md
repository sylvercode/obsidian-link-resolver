# Data Model: Obsidian Link Resolver MCP Wrapper

The MCP wrapper adapts the existing resolver; it does not introduce a second link-resolution model. The authoritative result fields and serialization semantics remain in [the resolver result schema](../001-link-resolver/contracts/result.schema.json).

## MCP Tool Request

One `resolve_obsidian_link` invocation.

| Field | Type | Required | Validation and meaning |
|---|---|---:|---|
| `link` | string | Yes | Non-empty raw Obsidian link text accepted by the existing resolver parser. |
| `context_path` | string | Yes | Non-empty path to the file containing the link; relative and absolute paths follow existing resolver behavior. |
| `vault_root` | string | No | Non-empty explicit vault root. Per-call value overrides a server launch-time default; if neither is supplied, existing nearest-`.obsidian` detection applies. |
| `with_emplacement` | boolean | No | Defaults to `false`; requests the existing structured heading/section/block result when available. Attachments retain existing no-emplacement behavior. |

The request is not persisted. It is passed to the resolver with no alternate parsing or matching rules.

## Resolver Capability

The existing Rust library capability parses a link, validates/detects its vault, resolves against the vault index, and reads the target note as needed. MCP requests delegate to `resolve_with_vault` after obtaining the cached index. Root selection, index enumeration, and resolution remain conceptually separate so cached requests can validate/select a root without rebuilding its index.

## Vault Index Cache

An in-memory cache entry is owned by the MCP server process and keyed by the canonical absolute vault root.

| Field | Type | Meaning |
|---|---|---|
| `root` | canonical path | Identity key; never returned in the public resolution record. |
| `vault` | `Vault` snapshot | Existing supported note/attachment entries used for name and path matching. |
| `last_full_scan` | monotonic instant | Completion marker used to schedule the next background full rescan, no more than 60 seconds later. |
| `dirty` | boolean | Set by relevant watcher events; causes a rebuild before the next resolution. |
| `watch_state` | internal state | Active watcher or unavailable; unavailability does not disable periodic refresh. |

### Cache State Transitions

- **Absent → Clean**: first request selects/canonicalizes the root, enumerates it, and registers a watcher when possible.
- **Clean → Dirty**: a relevant filesystem event is observed; event bursts coalesce into one dirty state.
- **Clean → Refreshing**: the per-vault background timer reaches its full-rescan deadline, independently of whether a request arrives.
- **Dirty/Refreshing → Clean**: enumeration succeeds and atomically replaces the cached snapshot and scan time.
- **Dirty/Refreshing → Error**: enumeration fails; the cache records the refresh failure, requests return a resolver `error` outcome, and the stale snapshot is not served. A later refresh retries.
- **Watcher Active → Watcher Unavailable**: watch registration/runtime failure; retain the cache and rely on periodic rescans.

Filesystem events are hints, not an authoritative operation log. A background full scan runs at least every 60 seconds even while watching and even when no requests arrive; this makes undetected index changes available to the next resolution within the specified bound. The cache exists for the server process lifetime; this is the defined active-session scope. Note file bodies are read on each resolution, so body edits are visible without a vault-index transition.

## MCP Tool Result

The result is the existing `ResolutionTarget` record. It has the five mutually exclusive statuses `resolved`, `unresolved`, `sub_target_not_found`, `ambiguous`, and `error`, and retains the existing optional target path/range, embed/display metadata, candidates, reason, and emplacement fields. The MCP response carries this record as structured content and its compact serialized JSON as text. Resolver statuses are successful tool calls with machine-readable outcomes; invalid protocol arguments or transport/lifecycle failures are MCP errors.

## Relationships

- Each tool request selects one context file and one effective vault root.
- Many requests in one server process may share one cache entry when their effective canonical roots match.
- Each request produces one resolver result from that request plus the selected `Vault` snapshot and current target note contents.
- The tool result schema is the existing result schema, not an MCP-specific fork.
