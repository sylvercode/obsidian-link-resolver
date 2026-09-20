# Phase 1 Data Model: Obsidian Link Resolver CLI

Derived from the Key Entities in [spec.md](spec.md). These are conceptual domain
entities that map to Rust types in `src/`; field names shown in `snake_case`
match the intended JSON output where relevant (see [contracts/result.schema.json](contracts/result.schema.json)).

## Entity: Link (parsed Obsidian link)

The reference being resolved, produced by `link.rs` from the raw input string.

| Field | Type | Notes |
|-------|------|-------|
| `raw` | string | Original link text as provided |
| `style` | enum `wikilink` \| `markdown` | Link syntax family (FR-002, FR-003) |
| `is_embed` | bool | True for `![[...]]` / embed markdown; reported, does not change target (FR-013) |
| `folder_path` | string? | Optional vault-relative folder prefix before the note name (FR-005a) |
| `note_name` | string? | Target note name; absent for self-references `[[#...]]` (FR-002) |
| `heading_path` | string[] | Ordered heading segments (e.g. `Parent`, `Child`); empty if none (FR-002) |
| `block_id` | string? | Block id without leading `^`; mutually exclusive with `heading_path` |
| `alias` | string? | Display text after `\|`; informational only, never affects target (FR-012) |

**Validation / parse rules**:
- Exactly one of {`heading_path` non-empty, `block_id` present, neither} may hold; a link cannot target both a heading and a block id.
- A self-reference (`note_name` absent) MUST carry a heading path or block id.
- Whitespace around name/heading/block segments is trimmed before matching (spec Edge Cases; FR-005b).

## Entity: ContextFile

The markdown file that contains the link.

| Field | Type | Notes |
|-------|------|-------|
| `path` | string | Absolute path to the context file (input) |
| `vault` | Vault | The vault it belongs to (resolved) |

**Rules**: Used for same-file resolution (`[[#...]]`) and as the starting point for vault auto-detection (FR-004).

## Entity: Vault

The collection within which names are resolved.

| Field | Type | Notes |
|-------|------|-------|
| `root` | string | Absolute path to the vault root |
| `source` | enum `explicit` \| `detected` | How the root was determined (FR-004) |
| `entries` | NoteIndexEntry[] | Enumerated notes + attachments (paths only) |

**Rules**:
- `root` is an explicit caller value, else the nearest ancestor of the context file containing `.obsidian` (FR-004); if neither, resolution fails with a "vault undetermined" error (FR-004a).
- `entries` are gathered by a single walk that records paths only (no body reads) to preserve latency.

### Sub-entity: NoteIndexEntry

| Field | Type | Notes |
|-------|------|-------|
| `rel_path` | string | Vault-relative path (used for path-qualified matches) |
| `name` | string | File stem (note name) or full attachment filename |
| `is_markdown` | bool | Distinguishes notes from attachments (FR-014) |

## Entity: ResolutionTarget (primary result)

The output of resolving a `Link` in a `ContextFile`.

| Field | Type | Notes |
|-------|------|-------|
| `status` | enum `resolved` \| `unresolved` \| `sub_target_not_found` \| `ambiguous` \| `error` | Outcome (FR-010) |
| `target_path` | string? | Resolved file path, expressed as a **vault-relative** path with forward-slash (`/`) separators (never absolute); present for `resolved` and `sub_target_not_found` (FR-006) |
| `target_line` | int? | 1-based line where target begins; null when link has no heading/block or for attachments (FR-006, FR-007, FR-014, FR-020) |
| `is_embed` | bool | Echoed from the link (FR-013) |
| `alias` | string? | Echoed display text, informational (FR-012) |
| `candidates` | string[]? | Candidate vault-relative paths for `ambiguous`, sorted ascending by vault-relative path using ordinal (byte-wise) comparison for byte-for-byte determinism (FR-011, FR-016, SC-003) |
| `reason` | string? | Actionable reason for any non-`resolved` outcome (FR-019) |
| `emplacement` | StructuredEmplacement? | Present only when requested and target lands inside a note (FR-008) |

**State/outcome transitions** (mutually exclusive terminal states):
- Note not found anywhere → `unresolved` (exit 2).
- Note found, but requested heading/block absent → `sub_target_not_found` with `target_path` set (exit 3).
- Note name matches multiple candidates after shortest-path preference → `ambiguous` (exit 4).
- Note (and sub-target, if any) found → `resolved` (exit 0).
- Bad input / vault undetermined / I/O → `error` (exit 1).

**Determinism**: No timestamps or other non-deterministic fields appear in this record; `target_path`/`candidates` are vault-relative forward-slash paths and the `candidates` list is ordinally sorted, so identical inputs yield byte-for-byte identical output (FR-006, FR-011, FR-016, SC-003).

## Entity: StructuredEmplacement

Positional context of a target within its note (FR-008, FR-009).

| Field | Type | Notes |
|-------|------|-------|
| `heading_stack` | HeadingRef[] | Ordered outermost→innermost containing headings; empty if none (spec US2 AS2) |
| `section` | LineRange | Begin/end line range of the target's own section |

### Sub-entity: HeadingRef

| Field | Type | Notes |
|-------|------|-------|
| `text` | string | Heading text |
| `level` | int | 1–6 (ATX heading level) |
| `begin` | int | 1-based line of the heading |
| `end` | int | 1-based last line of the heading's section (line before next heading of equal/higher level, or EOF) (FR-009) |

### Sub-entity: LineRange

| Field | Type | Notes |
|-------|------|-------|
| `begin` | int | 1-based inclusive start |
| `end` | int | 1-based inclusive end |

**Rules**:
- For a note with no headings, `heading_stack` is empty and `section` spans the whole file (spec US2 AS2).
- A resolved heading's section spans from its heading line up to (but excluding) the next heading of equal or higher level (spec US2 AS3; FR-009).
- Attachments have `target_line = null` and no `emplacement` (FR-014).

## Entity: ResolverSession (FFI/ABI boundary handle)

An opaque, in-process handle used by host applications embedding the resolver
through the C-compatible ABI/FFI boundary (FR-021, FR-021a). Not part of the JSON
result record; it models the state carried across consecutive in-process calls.
See the C-ABI signatures in [contracts/ffi.md](contracts/ffi.md).

| Field | Type | Notes |
|-------|------|-------|
| `vault` | Vault | Vault index loaded once and reused across resolutions (FR-021a) |
| `handle` | opaque pointer | Passed back to the host as `*mut OlrSession`; never dereferenced by callers |

**Lifecycle / rules**:
- Opened once per vault root (`olr_session_open`), reused for many consecutive `olr_resolve` calls without per-call process spawn (FR-021a, SC-007).
- Each `olr_resolve` takes a link string, a context-file path, and an emplacement flag, and returns the same compact JSON `ResolutionTarget` record (as a UTF-8 string) that the CLI emits in `--format json` (single source of truth: [contracts/result.schema.json](contracts/result.schema.json)); the numeric outcome mirrors the CLI exit-status contract.
- Result strings are owned by the library and released by the host via `olr_string_free`; the session is released via `olr_session_close`. No non-deterministic fields cross the boundary (FR-016).
- The handle is thread-compatible but not required to be thread-safe for v1; hosts serialize calls per session unless documented otherwise.
