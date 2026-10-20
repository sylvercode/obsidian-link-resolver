# MCP Link Generation Brainstorm

Date: 2026-10-18

Status: Brainstorm report only. No implementation is authorized by this document.

## Intent And Value

Add a read-only MCP tool that returns Obsidian-compatible link text for an LLM
or user to insert into a note. The caller chooses what to reference; the tool
handles link formatting and, when requested, checks the destination.

An LLM can usually produce plausible link syntax. A dedicated tool is useful
because plausible syntax does not establish that a link resolves to the intended
note, heading, or block. Deterministic generation and explicit validation would
complement the existing resolution tool.

## Agreed Requirements

- Accept a contextual source file, a target file, and an optional heading or
  block ID. Support whole-note links as well as heading and block references.
- Support Wikilinks and Obsidian-compatible Markdown links.
- Select the format per generation request, defaulting to Wikilinks. The user's
  phrase "weak link" was interpreted as "Wikilink" during the conversation.
- Do not read Obsidian settings to determine the format. The caller supplies
  preferences, avoiding a dependency on Obsidian's settings schema.
- Support existing targets whose destination must be validated.
- Also support future targets that an LLM or user will create later, including
  future notes, headings, or block IDs.
- Strictly return link text and validation details when needed. Never create or
  modify files, insert block IDs, or perform the caller's editing task.

## Recommended Behavior, Not Yet Approved

Use an explicit existing-target versus future-target intention. A missing
existing target should not silently become an accepted future target.

For existing targets, generate a candidate and verify that it resolves to the
requested file and subtarget, rather than merely accepting any successful
resolution. Return a clear failure if validation fails; do not present the
candidate as a validated link.

For future targets, validate representable syntax and allowed path boundaries,
but clearly distinguish those checks from destination validation. A correctly
formatted future link is not a promise that its destination exists or will be
created. This also covers an existing note with a heading or block planned for
later creation.

Require exact vault-relative target paths to avoid guessing among identically
named notes. Allow the source to be a planned file identified by its intended
path, since the original use case includes links inserted into a new file.
These path choices are recommendations, not explicit user decisions.

## Existing Code Findings

- [MCP adapter](../crates/obsidian-link-resolver-mcp/src/mcp.rs): registers
  `resolve_obsidian_link`, accepts source context and an optional allowed vault
  root, uses the shared vault cache, and returns structured resolver results.
  There is no generation tool in this adapter today.
- The adapter's `select_allowed_vault` canonicalizes the context path. A source
  file that does not exist currently fails root selection. Planned source files
  therefore need deliberate handling rather than direct reuse of this check.
- [Link parser](../crates/obsidian-link-resolver/src/link.rs): represents
  Wikilink and Markdown styles, display text, heading paths, block IDs, embeds,
  and same-file references. Heading and block selectors are mutually exclusive.
  This is parsing infrastructure, not an existing link renderer.
- The Markdown parser percent-decodes destinations before parsing them. A
  generator needs explicit escaping and encoding rules, including rejection of
  inputs that cannot be faithfully represented. Those rules are not settled by
  the presence of a parser.
- [Resolution pipeline](../crates/obsidian-link-resolver/src/resolve.rs):
  already checks note lookup, ambiguous names, headings, and block IDs. Outcomes
  include `resolved`, `unresolved`, `sub_target_not_found`, `ambiguous`, and
  `error`. Heading and block lookup uses the first matching target, so successful
  resolution does not establish that duplicate subtargets are uniquely identified.
- [Vault handling](../crates/obsidian-link-resolver/src/vault.rs): provides
  indexed vault-relative paths and root-selection infrastructure. Generation
  must preserve the MCP's launch-time allowed-vault boundary, including for
  nonexistent paths and paths affected by symlinks.

These findings suggest reusing the parser, resolver, and shared cache for
existing-target checks. They do not demonstrate full Obsidian compatibility for
generated output, source-relative Markdown paths, or special-character cases.
No runtime tests or Obsidian application checks were performed for this report.

## Possible Tool Shape

Working name: `generate_obsidian_link`, alongside `resolve_obsidian_link`.

Possible inputs are source path, exact target path, optional heading or block ID,
format with a Wikilink default, explicit target intention, and vault selection
when necessary. An optional display label is a useful candidate, not an agreed
requirement. Field names and the final schema remain undecided.

The response should make the insertion text easy to consume and distinguish
validated existing destinations from unverified future destinations. Validation
failures should explain the relevant problem without implying success. Exact
response statuses and MCP error conventions remain to be designed.

## Open Decisions

- Confirm exact vault-relative target paths and support for nonexistent sources.
- Choose generated path style: vault-qualified, source-relative, or another
  explicit policy. Do not infer this from Obsidian settings.
- Decide display-label support and the default label for Markdown output.
- Decide whether nested heading selectors are exposed and how duplicate
  headings or block IDs affect the validation claim.
- Define representable names, escaping, encoding, and failure behavior for
  special characters in each format.
- Decide whether same-file shorthand, embeds, or attachments belong in the
  initial scope; they were not requested explicitly.

## Suggested Future Verification

If implementation is approved later, check that existing-target output resolves
back to the intended destination, both formats handle agreed escaping cases,
future targets remain explicitly unverified, planned sources are supported as
agreed, and no path can escape an allowed vault. Include independent
Obsidian-compatible examples so parser/renderer round trips are not the sole
evidence of correctness. Verify that generation never writes vault files and
that existing resolver behavior remains unchanged.

This report records the discussion and read-only code inspection. It is not a
completed design, implementation plan, or authorization to change product code.
