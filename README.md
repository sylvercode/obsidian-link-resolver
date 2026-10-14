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

## MCP Server Install / Update

The MCP server has its own installer; the commands below install or update
`obsidian-link-resolver-mcp` only. The CLI installer above remains CLI-only.

MCP release executables are published for Linux and macOS (x86_64 and
aarch64), and Windows (x86_64). Assets are named
`obsidian-link-resolver-mcp-<tag>-<os>-<arch>` with an `.exe` suffix on Windows.
The installers require a release that includes these MCP assets; older
CLI-only releases are not supported.

### Bash (Linux/macOS)

Install or update the latest release:

```bash
curl -fsSL https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.sh | bash
```

Install a pinned version or choose an installation directory:

```bash
curl -fsSL https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.sh | bash -s -- --version 2.0.0
```

To inspect the script first or set an installation directory:

```bash
curl -fsSLO https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.sh
bash install-mcp.sh --version 2.0.0 --install-dir "$HOME/.local/bin"
```

### PowerShell 7+ (Windows/Linux/macOS)

Install or update the latest release, or pin a version:

```powershell
irm https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.ps1 | iex
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.ps1))) -Version 2.0.0
```

To inspect the script first or set an installation directory:

```powershell
iwr https://raw.githubusercontent.com/sylvercode/obsidian-link-resolver/main/scripts/install-mcp.ps1 -OutFile install-mcp.ps1
pwsh ./install-mcp.ps1 -Version 2.0.0 -InstallDir "$HOME/.local/bin"
```

### GitHub Copilot in VS Code

Add a server entry to `.vscode/mcp.json` in the workspace, replacing the vault
path with an absolute path on this machine:

```json
{
	"servers": {
		"obsidian-link-resolver": {
			"type": "stdio",
			"command": "obsidian-link-resolver-mcp",
			"args": ["--vault", "/absolute/path/to/vault"]
		}
	}
}
```

Use a Windows path such as `C:\\Users\\you\\Documents\\Obsidian`. Add another
`"--vault", "/absolute/path/to/another-vault"` pair to `args` to allow more
vaults. VS Code starts the local process and connects to it over stdio; keep
stdout reserved for MCP messages.

### Claude Desktop

Add the server to Claude Desktop's `claude_desktop_config.json` (`%APPDATA%\\Claude\\claude_desktop_config.json` on Windows, or
`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
	"mcpServers": {
		"obsidian-link-resolver": {
			"command": "obsidian-link-resolver-mcp",
			"args": ["--vault", "/absolute/path/to/vault"]
		}
	}
}
```

Replace the vault path with an absolute path and repeat the `--vault` argument
pair for each additional allowed vault. Claude Desktop launches the local
server and communicates through stdio. Restart the client after updating its
configuration.

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

Version 2.0.0 moves CLI argument types out of the resolver library. Consumers
that used `obsidian_link_resolver::cli` should see [MIGRATION.md](MIGRATION.md).

## Benchmark Corpus

`cargo bench -p obsidian-link-resolver` uses a synthetic ~5,000-note vault generated under
`tests/fixtures/bench-vault/` on demand. The corpus is produced by the bench
helper module in `tests/fixtures/bench_vault.rs` and is designed for the warm
run latency gate described in the feature quickstart.
