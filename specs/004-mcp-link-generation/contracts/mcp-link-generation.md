# MCP Contract: Link Generation

## Tool

`generate_obsidian_link`

The tool returns formatted link text and separately reports syntax, allowed-path, and destination-validation outcomes. It is additive; `resolve_obsidian_link` retains its existing schema and behavior.

## Input

All fields are strict: reject unknown properties and invalid types.

| Property | Type | Required | Default / constraints |
|---|---|---:|---|
| `context_path` | string | yes | Source note path; may identify a planned, not-yet-created file if its path is safely within a configured vault. |
| `target_path` | string | yes | Exact vault-relative note or supported attachment path. |
| `intent` | string enum | yes | `existing` or `future`. |
| `format` | string enum | no | `wikilink` or `markdown`; default `wikilink`. |
| `output` | string enum | no | `link` or `embed`; default `link`. |
| `same_file` | boolean | no | Default `false`; when true, requires a same-note heading or block target. |
| `heading` | string | no | Mutually exclusive with `block_id`; note targets only. |
| `block_id` | string | no | Mutually exclusive with `heading`; note targets only. |
| `label` | string | no | Optional visible text for either format; when omitted, Markdown text is derived from target/subtarget and Wikilinks use their target-derived display. |
| `vault_root` | string | no | Optional root, which must match a root configured at server startup. |

## Output

Successful MCP transport responses contain structured content with these fields:

| Field | Type | Description |
|---|---|---|
| `link_text` | string or null | Candidate insertion text when representable; never implies validation by itself. |
| `status` | `validated` \| `unverified` \| `rejected` | Overall result. |
| `syntax_valid` | boolean | Candidate syntax can be represented faithfully. |
| `path_allowed` | boolean | All requested paths satisfy the startup-configured root boundary. |
| `destination_validation` | `validated` \| `unverified` \| `failed` | Exact destination result, distinct from syntax/path checks. |
| `target_path` | string or null | Canonical vault-relative path when known. |
| `format` | `wikilink` \| `markdown` | Selected format. |
| `output` | `link` \| `embed` | Selected form. |
| `reason` | string or null | Concise explanation when not validated. |

Argument deserialization errors remain MCP tool errors. Valid requests with missing, ambiguous, or mismatched existing destinations return a structured `rejected` result, not a validated candidate. Future requests return `unverified` only after syntax and path checks succeed.

## Rendering Rules

- Both formats use vault-relative paths with `/` separators.
- Note Wikilinks may omit `.md`; Markdown destinations retain the extension and percent-encode destination characters per Obsidian's internal-link documentation.
- A heading or block selector is appended to the destination; headings and block IDs are mutually exclusive.
- Same-file shorthand omits the note path only when the requested target is the contextual note.
- Wikilinks use `[[target]]` or `[[target|label]]`; embeds prefix `!`. Markdown links use `[label](target)`; Markdown embeds use `![label](target)`. A supplied `label` is used in either format.
- Attachment targets retain the full filename and use the existing resolver's supported-extension rules.
- Any unfaithful escaping/encoding or unsupported name is rejected without claiming validation.

## Compatibility and Verification

- Existing `resolve_obsidian_link` name, input, output, and status behavior remain unchanged.
- Contract tests assert the new tool's discovered schema, defaults, invalid inputs, structured result fields, and exact syntax.
- Stdio integration tests assert exact-destination validation, future-target separation, attachment behavior, embeds, same-file shorthand, and path-boundary rejection.

## References

- [Obsidian Internal links](https://obsidian.md/help/links)
- [Obsidian Embed files](https://obsidian.md/help/embeds)
- [Obsidian Accepted file formats](https://obsidian.md/help/file-formats)
- Local result semantics: `crates/obsidian-link-resolver/src/output.rs`
