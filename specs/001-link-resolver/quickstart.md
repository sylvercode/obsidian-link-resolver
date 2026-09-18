# Quickstart & Validation: Obsidian Link Resolver CLI

This guide proves the feature works end-to-end. It references the contracts in
[contracts/cli.md](contracts/cli.md) and [contracts/result.schema.json](contracts/result.schema.json)
and the entities in [data-model.md](data-model.md) rather than duplicating them.

## Prerequisites

- Rust toolchain 1.83+ (`rustup`, `cargo`).
- A checkout of this repository.

## Build

```bash
cargo build --release
# binary at target/release/obsidian-link-resolver
```

## Fixture vault

A sample vault lives under `tests/fixtures/vault/` and contains, at minimum, a
`.obsidian/` directory plus notes and an attachment that cover every documented
link form (plain, aliased, heading, nested heading, block, same-file, embed,
markdown-style, path-qualified, duplicate headings, and an attachment). It is the
corpus behind SC-001 and SC-004.

## Validation scenarios

Run each command from the repository root. Expected outcomes are stated as the
result `status` and exit code (see the exit-status contract in
[contracts/cli.md](contracts/cli.md)); full field shapes are validated against
[contracts/result.schema.json](contracts/result.schema.json).

| # | Scenario (spec ref) | Command (abbreviated) | Expected `status` | Exit |
|---|---------------------|-----------------------|-------------------|------|
| 1 | Plain note link (US1 AS1) | `... '[[Project Plan]]' --context notes/a.md` | `resolved`, `target_line` null | 0 |
| 2 | Heading link (US1 AS2) | `... '[[Project Plan#Milestones]]' --context notes/a.md` | `resolved`, `target_line` at heading | 0 |
| 3 | Block link (US1 AS3) | `... '[[Project Plan#^abc123]]' --context notes/a.md` | `resolved`, `target_line` at block | 0 |
| 4 | Same-file link (US1 AS4) | `... '[[#Overview]]' --context notes/a.md` | `resolved`, target = context file | 0 |
| 5 | Broken link (US1 AS5) | `... '[[No Such Note]]' --context notes/a.md` | `unresolved` + reason | 2 |
| 6 | Sub-target missing (Edge) | `... '[[Project Plan#Nope]]' --context notes/a.md` | `sub_target_not_found` + `target_path` | 3 |
| 7 | Ambiguous name (Edge) | `... '[[Dup]]' --context notes/a.md` | `ambiguous` + `candidates` | 4 |
| 8 | Nested emplacement (US2 AS1) | `... '[[Design#API#Auth]]' --context notes/a.md --emplacement` | `resolved` + heading stack `Design,API,Auth` | 0 |
| 9 | No-heading emplacement (US2 AS2) | `... '[[Flat]]' --context notes/a.md --emplacement` | `resolved`, empty stack, whole-file section | 0 |
| 10 | Aliased link (Edge) | `... '[[Project Plan\|Plan]]' --context notes/a.md` | `resolved`, `alias` echoed | 0 |
| 11 | Embed (Edge) | `... '![[Note#Section]]' --context notes/a.md` | `resolved`, `is_embed` true | 0 |
| 12 | Markdown-style link (US1 / FR-003) | `... '[t](Some%20Note.md#Heading)' --context notes/a.md` | `resolved` | 0 |
| 13 | Attachment (Edge / FR-014) | `... '![[diagram.png]]' --context notes/a.md` | `resolved`, `target_line` null, no emplacement | 0 |
| 14 | Path-qualified (Edge / FR-005a) | `... '[[folder/sub/Note#H]]' --context notes/a.md` | `resolved` to that path | 0 |
| 15 | Vault undetermined (US1 / FR-004a) | `... '[[X]]' --context /tmp/outside.md` | `error` + reason | 1 |
| 16 | Explicit vault root (Edge) | `... '[[X]]' --context notes/a.md --vault tests/fixtures/vault` | uses root as-is | 0 |

Full command form for scenario 2:

```bash
target/release/obsidian-link-resolver '[[Project Plan#Milestones]]' \
  --context tests/fixtures/vault/notes/a.md --format json
# → single-line JSON on stdout, exit 0
```

## Determinism check (SC-003)

```bash
A=$(target/release/obsidian-link-resolver '[[Project Plan#Milestones]]' --context tests/fixtures/vault/notes/a.md)
B=$(target/release/obsidian-link-resolver '[[Project Plan#Milestones]]' --context tests/fixtures/vault/notes/a.md)
[ "$A" = "$B" ] && echo "deterministic OK"
```

## Stream separation check (FR-018)

```bash
# stdout carries only the result; diagnostics go to stderr
target/release/obsidian-link-resolver '[[Project Plan]]' \
  --context tests/fixtures/vault/notes/a.md -v 1>result.json 2>diag.log
```

## Performance check (SC-005)

```bash
cargo bench            # criterion warm-run benchmark on a ~5,000-note vault
# assert reported warm-run median ≤ 100 ms; CI fails on regression
```

## Automated equivalents

- Contract tests (`tests/contract/`) assert the stdout JSON validates against
  `result.schema.json` and that exit codes match the table above.
- Integration tests (`tests/integration/`) run scenarios 1–16 over the fixture
  vault.
- The `criterion` bench (`benches/resolve.rs`) enforces the latency budget.
