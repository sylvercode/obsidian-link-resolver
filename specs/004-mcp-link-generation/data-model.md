# Data Model: MCP Link Generation

## LinkGenerationRequest

One MCP tool call asking for link text.

| Field | Type | Required | Rules |
|---|---|---:|---|
| `context_path` | string | yes | Existing or planned source note path; absolute or working-directory-relative, constrained to a configured vault. |
| `target_path` | string | yes | Exact vault-relative file path; note or supported attachment. |
| `intent` | `existing` \| `future` | yes | Caller-selected; missing existing targets never become future targets implicitly. |
| `format` | `wikilink` \| `markdown` | no | Defaults to `wikilink`. |
| `output` | `link` \| `embed` | no | Defaults to `link`; embeds support note and attachment targets. |
| `same_file` | boolean | no | Defaults to false. When true, target must be the contextual note and a heading or block selector is required. |
| `heading` | string | no | Optional note heading path; mutually exclusive with `block_id`. |
| `block_id` | string | no | Optional block identifier; mutually exclusive with `heading`. |
| `label` | string | no | Optional visible label for either format; omitted Markdown labels are derived from the target and requested subtarget, while Wikilinks use their target-derived display. |
| `vault_root` | string | no | Optional selector that must match one startup-configured root. |

### Validation

- Reject empty required strings, unknown fields, simultaneous `heading` and `block_id`, invalid linkable filenames, and same-file requests without a subtarget.
- Resolve `target_path` beneath the selected configured root and reject traversal or symlink escapes.
- For a planned `context_path`, verify the canonical existing prefix remains inside the root and that the missing tail is safe; the planned file itself is not created.
- Attachments use the existing resolver's supported extension/name rules and cannot have heading or block selectors.
- Markdown destinations use vault-relative paths and are percent-encoded according to Obsidian's link syntax.

## Destination

The requested vault entry, with:

- `path`: canonical vault-relative path using `/` separators.
- `kind`: `note` for Markdown or `attachment` for another supported file type.
- `subtarget`: none, heading path, or block ID; attachments permit none only.
- `intent`: `existing` or `future`.

A same-file shorthand request still identifies the destination by its exact `path`; the output omits that path only after confirming it is the contextual note.

## GenerationResult

| Field | Type | Meaning |
|---|---|---|
| `link_text` | optional string | Insertion candidate when the request is representable and remains within the configured root. |
| `status` | `validated` \| `unverified` \| `rejected` | Overall caller-facing outcome. |
| `syntax_valid` | boolean | Whether the candidate is representable in the selected syntax. |
| `path_allowed` | boolean | Whether source/target paths remain inside a configured root. |
| `destination_validation` | `validated` \| `unverified` \| `failed` | Explicit destination-check result; future targets are never reported as validated. |
| `target_path` | optional string | Canonical vault-relative target when known. |
| `format` | `wikilink` \| `markdown` | Selected output format. |
| `output` | `link` \| `embed` | Selected insertion form. |
| `reason` | optional string | Concise explanation for unverified or rejected results. |

### State Rules

- Existing target + exact resolver match => `status=validated`, `destination_validation=validated`.
- Future target + valid syntax + allowed paths => `status=unverified`, `destination_validation=unverified`.
- Invalid syntax/path or an existing destination/subtarget mismatch => `status=rejected`; never claim validation.
- Generation is read-only: no file creation, block-ID insertion, or note modification.
