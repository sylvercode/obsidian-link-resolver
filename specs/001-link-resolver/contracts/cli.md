# CLI Contract: Obsidian Link Resolver

This contract is the stable interface that agents, MCP servers, and skill
wrappers depend on. Changes here are governed by the constitution's versioning
policy (breaking output/exit-code changes are MAJOR).

## Invocation

```text
obsidian-link-resolver <LINK> --context <CONTEXT_FILE> [OPTIONS]
```

- `<LINK>` (positional, required): the raw Obsidian link string, e.g. `[[Project Plan#Milestones]]`.
- `--context <CONTEXT_FILE>` (required): path to the markdown file that contains the link.

### Options

| Flag | Values | Default | Purpose |
|------|--------|---------|---------|
| `--vault <DIR>` | path | (auto-detect) | Explicit vault root; when omitted, auto-detect via nearest ancestor `.obsidian` (FR-004) |
| `--format <MODE>` | `json` \| `human` | `json` | Output mode; `json` is compact single-line (FR-015) |
| `--emplacement` | flag | off | Include structured emplacement (heading stack + ranges) (FR-008) |
| `-v`, `--verbose` | flag (repeatable) | off | Diagnostic logging to stderr only (Principle V) |
| `-h`, `--help` | flag | — | Usage to stdout, exit 0 |
| `-V`, `--version` | flag | — | Semantic version to stdout, exit 0 |

## Streams

- **stdout**: exactly one primary result record (single-line JSON in `json` mode; formatted text in `human` mode). Nothing else. (FR-018)
- **stderr**: diagnostics, verbose logs, and error messages only. (FR-018)

## Exit status contract (FR-017, SC-002)

| Code | Meaning | Result `status` |
|------|---------|-----------------|
| `0` | Link fully resolved | `resolved` |
| `1` | Usage/error: bad arguments, vault could not be determined, or I/O failure (FR-004a) | `error` |
| `2` | Target note not found in vault | `unresolved` |
| `3` | Note found but heading/block sub-target missing | `sub_target_not_found` |
| `4` | Note name matches multiple candidates | `ambiguous` |

Callers MUST be able to branch on outcome using the exit code alone, without
parsing stdout.

## Machine-mode output (`--format json`)

- Single line, compact (no extra whitespace), UTF-8, trailing newline.
- Fixed field order; optional/inapplicable fields are omitted or `null` per
  [result.schema.json](result.schema.json).
- No timestamps or other non-deterministic fields in the primary record
  (FR-016, SC-003) — identical inputs produce byte-for-byte identical stdout.
- `target_path` and every `candidates` entry are **vault-relative** paths with
  forward-slash (`/`) separators, never absolute filesystem paths (FR-006).
- For `ambiguous`, `candidates` is sorted ascending by vault-relative path using
  ordinal (byte-wise) comparison, so the list order is deterministic (FR-011).
- Line numbers are 1-based (FR-020).

### Examples

Plain resolved link (`[[Project Plan]]`):

```json
{"status":"resolved","target_path":"Project Plan.md","target_range":null,"is_embed":false}
```

Heading link with emplacement requested (`[[Design#API]]` under `# Design`):

```json
{"status":"resolved","target_path":"Design.md","target_range":{"begin":12,"end":25},"is_embed":false,"emplacement":{"heading_stack":[{"text":"Design","level":1,"begin":1,"end":40},{"text":"API","level":2,"begin":12,"end":25}],"section":{"begin":12,"end":25}}}
```

Ambiguous link:

```json
{"status":"ambiguous","candidates":["a/Note.md","b/Note.md"],"reason":"note name 'Note' matches multiple notes"}
```

Sub-target missing:

```json
{"status":"sub_target_not_found","target_path":"Project Plan.md","reason":"heading 'Milestones' not found in 'Project Plan.md'"}
```

Unresolved:

```json
{"status":"unresolved","reason":"note 'Missing Note' not found in vault"}
```

Attachment (`![[diagram.png]]`):

```json
{"status":"resolved","target_path":"assets/diagram.png","target_range":null,"is_embed":true}
```

## Human-mode output (`--format human`)

Non-contractual, for interactive use. Prints a readable summary of the same
fields to stdout. Field values match machine mode; formatting may change
between MINOR versions.
