# Obsidian Link Resolver

`obsidian-link-resolver` resolves an Obsidian link against a vault and a
context file, then returns the target file and, when applicable, the 1-based
line range for the target heading or block.

## Install / Update

### Bash (Linux/macOS)

Update if present, otherwise install the latest release:

```bash
curl -fsSL https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install.sh | bash
```

Install a pinned version:

```bash
curl -fsSL https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install.sh | bash -s -- --version 1.2.3
```

### PowerShell (Windows/Linux/macOS)

Update if present, otherwise install the latest release:

```powershell
irm https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install.ps1 | iex
```

Install a pinned version:

```powershell
irm https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install.ps1 | % { iex "$($_) -Version 1.2.3" }
```

### Manual fallback (no pipe-to-shell)

If you prefer to inspect first:

```bash
curl -fsSLO https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install.sh
bash install.sh --help
```

```powershell
iwr https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install.ps1 -OutFile install.ps1
pwsh ./install.ps1 -Mode install
```

## Usage

```bash
obsidian-link-resolver '[[Project Plan#Milestones]]' \
	--context tests/fixtures/vault/notes/a.md
```

Required input:

- `<LINK>`: the raw Obsidian link string.
- `--context <FILE>`: the markdown file that contains the link.

Shell tip: wrap links in single quotes when possible (`'[[#^id|Alias]]'`).
If a shell-escaped alias separator (`\|`) reaches the binary, it is treated
the same as `|`.

Optional flags:

- `--vault <DIR>`: use an explicit vault root instead of auto-detecting one.
- `--format <json|human>`: choose machine-readable or human-readable output.
- `--emplacement`: include the heading stack and section range for note targets.
- `-v` / `--verbose`: emit diagnostics to stderr.

## Exit Codes

The CLI maps each outcome to a stable exit code:

| Exit | Status | Meaning |
|------|--------|---------|
| `0` | `resolved` | The link resolved successfully. |
| `1` | `error` | The vault could not be determined or another fatal error occurred. |
| `2` | `unresolved` | The target note or path was not found. |
| `3` | `sub_target_not_found` | The note exists but the heading or block target does not. |
| `4` | `ambiguous` | More than one note matches the requested name. |

## Output Shape

Machine mode (`--format json`) emits one compact JSON record on stdout. The
record is deterministic and vault-relative:

- `target_path` and every `candidates` entry are normalized with forward-slash
	separators and are never absolute paths.
- `target_range` is the canonical location for heading, block, and
	structured-block targets.
- `display_text`, `is_embed`, and `emplacement` are informational and do not
	change how the link resolves.

Plain-file and attachment targets set `target_range` to `null`. Heading and
block targets use a 1-based inclusive `{begin,end}` range for the target line
or section.

## Structured Emplacement

When `--emplacement` is enabled and the target lands in a note, the result can
include a structured emplacement payload:

- `heading_stack`: outermost-to-innermost headings that contain the target.
- `section`: the line range for the containing section.
- `structured_block`: the enclosing quote, callout, table, list, code, or math
	block when the target falls inside one.

Attachments never include emplacement data.

## Integration Surfaces

The same result contract is exposed through three surfaces:

- CLI: the process interface documented above.
- JSON schema: `specs/001-link-resolver/contracts/result.schema.json`.
- C ABI / FFI: the `cdylib` surface documented in
	`specs/001-link-resolver/contracts/ffi.md`.

The FFI session API loads a vault once and reuses the index across many
consecutive resolutions.

## Benchmark Corpus

`cargo bench` uses a synthetic ~5,000-note vault generated under
`tests/fixtures/bench-vault/` on demand. The corpus is produced by the bench
helper module in `tests/fixtures/bench_vault.rs` and is designed for the warm
run latency gate described in the feature quickstart.
