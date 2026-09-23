---

description: "Task list for Obsidian Link Resolver CLI implementation"
---

# Tasks: Obsidian Link Resolver CLI

**Input**: Design documents from `/specs/001-link-resolver/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: INCLUDED. Test-First is NON-NEGOTIABLE per Constitution Principle III; the plan orders every module tests-first (unit, contract, integration, FFI, performance).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Single Rust crate at repository root (plan.md "Structure Decision"): library core in `src/` (compiled as `rlib` + `cdylib`) plus a `bin` target; tests in `tests/`, benches in `benches/`, generated C header in `include/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, toolchain, and build/release automation

**Documentation Gate (Principle IX)**: Config files (`Cargo.toml`, `cbindgen.toml`, `build.rs`, `rustfmt.toml`) are not Rust symbols; no Rustdoc required. However, `.github/workflows/` YAML files MUST include inline comments explaining each major step. Verify all comments are present before phase completion.

- [x] T001 Create the Rust project directory structure per plan.md: `src/`, `tests/{contract,ffi,integration,unit,fixtures}/`, `benches/`, `include/`, `.devcontainer/`, `.github/workflows/`
- [x] T002 Create `Cargo.toml` declaring a library target with `crate-type = ["rlib", "cdylib"]` and a `bin` target `obsidian-link-resolver`; dependencies `clap` (v4, derive), `serde` (derive), `serde_json`, `walkdir`; dev-dependencies `assert_cmd`, `predicates`, `criterion`; build-dependency `cbindgen`
- [x] T003 [P] Create `cbindgen.toml` configuring C header generation from the `extern "C"` surface (C language, `OlrSession`/`OlrStatus`/`olr_*` symbols) per contracts/ffi.md
- [x] T004 [P] Create `build.rs` that runs `cbindgen` to generate `include/obsidian_link_resolver.h` from the crate's `extern "C"` surface
- [x] T005 [P] Create `.devcontainer/devcontainer.json` declaring the full toolchain: Rust stable 1.98.1, cross-compilation targets for Linux (x64+arm64), macOS (x64+arm64), Windows (x64), and `cbindgen` (Constitution Principle VII)
- [x] T006 [P] Add `rustfmt.toml` and a clippy lint configuration for consistent formatting and linting
- [x] T007 [P] Create `.github/workflows/ci.yml` as authoritative build/test automation running `cargo fmt --check`, `cargo clippy`, and `cargo test` (unit, contract, integration, FFI) on push/PR (Constitution Principle VIII)
- [x] T008 [P] Create `.github/workflows/release.yml` triggered on `vX.Y.Z` tags that runs the full test suite plus the warm-run p50 ≤100 ms gate, then cross-builds and publishes the CLI binary, `cdylib` shared library, and generated C header for all six platform targets (SC-005, SC-008, Constitution Principle VIII)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Domain types, library API skeleton, exit-code mapping, and the shared fixture vault that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T009 Create `src/lib.rs` with module declarations (`cli`, `ffi`, `link`, `vault`, `note`, `resolve`, `output`) and the public `resolve` entry-point signature (Link string + context path + vault option + emplacement flag → `ResolutionTarget`) reused by CLI, FFI, and tests
- [x] T010 [P] Define the `Link` entity in `src/link.rs`: fields `raw: String`, `style: LinkStyle` (enum `Wikilink | Markdown`), `is_embed: bool`, `folder_path: Option<String>`, `note_name: Option<String>`, `heading_path: Vec<String>`, `block_id: Option<String>`, `display_text: Option<String>` (data-model.md Entity: Link)
- [x] T011 [P] Define the `Vault`, `NoteIndexEntry`, and `ContextFile` entities in `src/vault.rs`: `Vault { root: String, source: VaultSource(explicit|detected), entries: Vec<NoteIndexEntry> }`, `NoteIndexEntry { rel_path: String, name: String, is_markdown: bool }`, `ContextFile { path: String }` (data-model.md Entities: Vault, NoteIndexEntry, ContextFile)
- [x] T012 [P] Define the `ResolutionTarget` result struct and `Status` enum in `src/output.rs` with serde attributes matching contracts/result.schema.json: `status` enum exactly `resolved | unresolved | sub_target_not_found | ambiguous | error` (five mutually exclusive outcomes, FR-010); optional fields `target_path: Option<String>`, `display_text: Option<String>`, `is_embed: bool`, `candidates: Option<Vec<String>>`, `reason: Option<String>`, `target_range: Option<LineRange>`; fixed field order, inapplicable fields omitted/null
- [x] T013 Implement the exit-code mapping in `src/output.rs`: `resolved→0`, `error→1`, `unresolved→2`, `sub_target_not_found→3`, `ambiguous→4` (contracts/cli.md exit-status contract, FR-017)
- [x] T014 [P] Create the fixture vault under `tests/fixtures/vault/` with a `.obsidian/` directory and notes + one attachment covering every documented link form: plain, display-text, heading, nested heading (`Design#API#Auth`), block id (`^abc123`), same-file, embed, markdown-style, path-qualified (`folder/sub/Note`), duplicate headings, and `assets/diagram.png` (quickstart.md Fixture vault; SC-001, SC-004)

**Documentation Gate (Principle IX)**: All domain types and entities in `src/lib.rs`, `src/link.rs`, `src/vault.rs`, `src/output.rs` MUST include Rustdoc comments (`///` for types and fields) explaining purpose, constraints, and field semantics. The public `resolve` entry point in `src/lib.rs` MUST include a `///` doc comment with purpose, parameters, return value, and error conditions. Verify with `cargo doc --open` and ensure no missing-docs warnings before phase completion.

**Checkpoint**: Domain types, library skeleton, exit-code contract, and fixture vault are ready — user story implementation can now begin

---

## Phase 3: User Story 1 - Resolve a link to its target file and location (Priority: P1) 🎯 MVP

**Goal**: Turn an Obsidian link string plus its context file path into a concrete target: the target file path and, for heading/block links, the 1-based line where the target begins. Distinguish resolved / unresolved / sub-target-not-found / ambiguous / error outcomes via exit codes.

**Independent Test**: Against the fixture vault, provide a context file path and a link string; assert the returned `target_path` and `target_range` match the known location, and that unresolved/sub-target/ambiguous inputs yield the correct status and exit code.

### Tests for User Story 1 (write FIRST, ensure they FAIL before implementation) ⚠️

- [X] T015 [P] [US1] CLI contract test for resolved outcomes (plain, heading, block, same-file) asserting single-line JSON on stdout and exit 0 in `tests/contract/resolve_cli.rs` (quickstart scenarios 1–4)
- [X] T016 [P] [US1] CLI contract test for `error` (exit 1: context file outside any vault / vault-undetermined, `status:"error"` + `reason` on stdout, diagnostics on stderr — FR-004a), `unresolved` (exit 2), and `sub_target_not_found` (exit 3, `target_path` + `reason` present) in `tests/contract/resolve_errors.rs` (quickstart scenarios 5, 6; FR-017, FR-019, SC-002)
- [X] T017 [P] [US1] Integration test over the fixture vault for scenarios 1–6, 10 (display-text), 11 (embed), 12 (markdown-style), 13 (attachment), 14 (path-qualified), 16 (explicit `--vault`) in `tests/integration/us1_resolution.rs`
- [X] T018 [P] [US1] Unit tests for link parsing (wikilink/markdown styles, embed flag, display-text split on `|`, folder prefix, `#` heading path, `^` block id, self-reference, whitespace trimming, and the rule that a link cannot target both a heading and a block id) in `tests/unit/link_parse.rs`; include a case where heading/block reference text differs only by case and still matches (FR-005b)
- [X] T019 [P] [US1] Unit tests for vault detection and name resolution (explicit root; `.obsidian` ancestor walk; vault-undetermined error; bare vs path-qualified match; case-insensitive; zero→unresolved, one→resolved, ≥two→ambiguous with candidates) in `tests/unit/name_resolution.rs`

**Documentation Gate (Principle IX)**: All functions and public types in `src/link.rs`, `src/vault.rs`, `src/note.rs`, `src/resolve.rs` MUST include Rustdoc comments (`///` for functions and types) explaining purpose, parameters, return values, and error/edge cases. The key public functions are `vault::detect_root`, `vault::enumerate_vault`, `vault::resolve_name`, `note::scan_note`, `note::find_target_line`, `resolve::resolve_link`. Verify with `cargo doc --open` and `cargo clippy -- -W missing-docs` before advancing to User Story 2.

### Implementation for User Story 1

- [X] T020 [US1] Implement Obsidian link parsing in `src/link.rs`: parse wikilink `[[...]]` and markdown `[text](target)` styles, detect embed `![[...]]` / `![](...)`, split display text on `|`, extract optional `folder_path`, `note_name`, `heading_path` (split on `#`, may be nested), `block_id` (`^id`); trim whitespace around each segment; enforce that `heading_path` non-empty and `block_id` are mutually exclusive and that a self-reference (absent `note_name`) carries a heading path or block id (data-model.md parse rules; FR-002, FR-003)
- [X] T021 [US1] Implement vault root detection in `src/vault.rs`: use explicit caller root when given, else walk up from the context file's directory to the nearest ancestor containing an `.obsidian` directory, else return the `error` outcome "vault could not be determined" (FR-004, FR-004a)
- [X] T022 [US1] Implement vault enumeration in `src/vault.rs`: a single `walkdir` pass that records paths only (no body reads), populating `NoteIndexEntry { rel_path, name, is_markdown }` where `rel_path` is a vault-relative, forward-slash-normalized path and `name` is the `.md` file stem for notes or the full filename for attachments; results MUST NOT depend on filesystem enumeration order (candidates are sorted downstream) (research Decision 4; performance constraint; FR-006, FR-016, SC-003; addresses analysis U1/U2)
- [X] T023 [US1] Implement name resolution in `src/vault.rs`: match the note-name portion case-insensitively against each note's basename; a path-qualified name matches only the exact vault-relative path with no bare-name fallback; exactly one match → resolved, zero → `unresolved`, two or more → `ambiguous` with the full candidate list — each a vault-relative, forward-slash path — sorted ascending by vault-relative path using ordinal (byte-wise) comparison and with no silent selection (FR-005, FR-005a, FR-005b, FR-006, FR-010, FR-011, FR-016, SC-003; addresses analysis U1/U2)
- [X] T024 [US1] Implement the note line scanner in `src/note.rs`: scan a single note for ATX headings (`#`..`######`) and trailing block ids (`^id`), tracking fenced-code-block state so `#`/`^id`-looking lines inside code fences are ignored (research Decision 2)
- [X] T025 [US1] Implement heading/block target-line lookup in `src/note.rs`: match heading text and block ids case-insensitively (after trimming surrounding whitespace), consistent with Obsidian's default (FR-005b); for a heading reference select the first matching heading in document order and report the chosen line (FR-002a); for a nested path resolve left-to-right, first parent then first deeper-level child within that parent's section (FR-002b); for a block id select the first matching line (FR-002c); a missing or malformed sub-target yields `sub_target_not_found`
- [X] T026 [US1] Implement the core resolve pipeline in `src/resolve.rs`: combine parsed `Link` + `ContextFile` + `Vault` into a `ResolutionTarget`; handle same-file `[[#...]]` references against the context file, non-markdown attachments (`target_range = null`, no emplacement, FR-014), embed echo (FR-013), and display-text echo (FR-012); no heading/block → `target_range = null` with beginning-of-file target (FR-006, FR-007)
- [X] T027 [US1] Implement compact single-line JSON serialization of `ResolutionTarget` in `src/output.rs` (fixed field order, omit/null inapplicable fields, trailing newline), emitting `target_path` and every `candidates` entry as vault-relative, forward-slash-normalized paths (never absolute) per contracts/cli.md and contracts/result.schema.json (FR-006; addresses analysis U2)
- [X] T028 [US1] Implement clap argument parsing in `src/cli.rs`: positional `<LINK>` (required), `--context <FILE>` (required), `--vault <DIR>` (optional, default auto-detect), `--format <json|human>` (default `json`), `-v/--verbose` (repeatable, stderr only), plus `-h/--help` and `-V/--version` (contracts/cli.md Options)
- [X] T029 [US1] Implement the CLI entrypoint in `src/main.rs`: parse args, call `lib::resolve`, emit the primary result on stdout and diagnostics on stderr (FR-018), and map the outcome `status` to the process exit code via `src/output.rs` (FR-017)

**Checkpoint**: User Story 1 is fully functional — the CLI resolves any documented link form to a file+line target with correct outcomes and exit codes (MVP).

**Documentation Gate (Principle IX)**: All symbols created in US1 (link parsing, vault detection, name resolution, note scanning, target lookup, core resolve pipeline, CLI argument parsing, main entrypoint) MUST have Rustdoc comments. Verify with `cargo doc --all` and ensure no missing-docs warnings before advancing to US2.

---

## Phase 3.1: User Story 1 Delta - OR1-OR15 and Range Contract Alignment

**Purpose**: Address specification/plan deltas introduced after initial US1 completion: explicit OR1-OR14 coverage, OR15 structured-block boundaries, and target-range output requirements.

**Independent Test**: Re-run US1 scenarios plus new OR1-OR15-focused scenarios; verify deterministic vault-relative JSON output now includes correct `target_range` semantics for heading/block/structured-block targets and rejects non-linkable targets per Obsidian rules.

### Tests for Phase 3.1 (write FIRST, ensure they FAIL before implementation) ⚠️

- [x] T030 [P] [US1] Extend contract/error tests in `tests/contract/resolve_errors.rs` to cover OR1-OR5 invalid targets and traversal pruning: unsupported extensions, dot-prefixed path segments, names containing invalid characters (`*`, `"`, `/`, `\`, `<`, `>`, `:`, `|`, `?`, `#`, `%%`, `[`, `]`, `^`), and cases proving vault enumeration never descends into dot-prefixed directories at any depth, with deterministic non-resolved outcomes and actionable `reason` (FR-002f, FR-019)
- [x] T031 [P] [US1] Extend parser unit tests in `tests/unit/link_parse.rs` for markdown-path parity rules (URL-decoding, optional `.md` note extension, same-file `#`/`#^` references, and heading-vs-block mutual exclusion) and case-insensitive matching behavior across path/heading/block segments (FR-002d, FR-002e, FR-003, FR-005b)
- [x] T032 [P] [US1] Add target-range contract/integration tests in `tests/contract/resolve_cli.rs` and `tests/integration/us1_resolution.rs` validating `target_range` for heading/block/structured-block targets, including single-line ranges represented as `{begin:X,end:X}`, and validating `target_range` is null/absent for plain-file and attachment targets (FR-006, FR-007, FR-014, OR12, OR12.1, OR12.2, OR15)

### Implementation for Phase 3.1

- [x] T033 [US1] Update `ResolutionTarget` and serializers in `src/output.rs` to include `target_range: Option<LineRange>` with 1-based inclusive `begin`/`end`; keep field ordering deterministic and schema-compatible with `contracts/result.schema.json` (FR-006, FR-007, FR-016)
- [x] T034 [US1] Extend note scanning in `src/note.rs` to detect OR15 structured-block boundaries (quote/callout, table, list, fenced code, math) and structured block-id placements (standalone line, end of list item/structured block, final line of structured block) while preserving fenced-block parsing correctness (OR12, OR12.1, OR12.2, OR15)
- [x] T035 [US1] Update target lookup/resolution in `src/note.rs` and `src/resolve.rs` so heading/block targets return deterministic `target_range` (not line-only), choosing first matching heading/block in document order and applying nested-heading left-to-right narrowing before sub-target failure (FR-002a, FR-002b, FR-002c, FR-006)
- [x] T036 [US1] Enforce supported-target filtering, traversal pruning, and deterministic vault-relative normalization in `src/vault.rs` and `src/resolve.rs`: include only Obsidian-linkable notes/attachments, reject dotfile/dotfolder/invalid-character names, never descend into dot-prefixed directories during vault enumeration, and keep `target_path`/`candidates` portable with forward-slash separators (FR-002f, FR-006, FR-011, FR-016)

**Checkpoint**: User Story 1 is re-baselined to the updated OR1-OR15 and range semantics; downstream US2/US3 work can assume stable `target_range` and structured-block primitives.

**Documentation Gate (Principle IX)**: All symbols added/changed in `src/note.rs`, `src/output.rs`, `src/resolve.rs`, and `src/vault.rs` for OR/range semantics MUST include Rustdoc comments, including the structured-block boundary rules and `target_range` semantics.

---

## Phase 4: User Story 2 - Return the structured emplacement of the target (Priority: P2)

**Goal**: On request, return the ordered heading stack containing the target (outermost→innermost) with each heading's begin/end line range, enabling sharded reading of large notes.

**Independent Test**: Resolve a link into a note with nested headings with emplacement requested; assert the returned heading stack (e.g. `Design, API, Auth`) and each section's begin/end line range match the note structure, and that a note with no headings yields an empty stack spanning the whole file.

### Tests for User Story 2 (write FIRST, ensure they FAIL before implementation) ⚠️

- [ ] T037 [P] [US2] Integration test for emplacement scenarios 8 (nested `Design#API#Auth` stack), 9 (no-heading note → empty stack + whole-file section), and US2 AS4 structured-block coverage (table/quote/callout/list/code/math boundaries with list-item-in-parent-list context) in `tests/integration/us2_emplacement.rs`
- [ ] T038 [P] [US2] Unit tests in `tests/unit/emplacement.rs` for heading section-end computation (`end` is line before next heading of equal/higher level or EOF), enclosing structured-block boundary derivation, and list-item emplacement inside parent-list context (FR-008, FR-009, OR15)

**Documentation Gate (Principle IX)**: All new types and functions added in US2 (`StructuredEmplacement`, `HeadingRef`, `LineRange`, heading-stack and section-range computation functions) MUST include Rustdoc comments explaining structure, field semantics, and computation logic. Verify with `cargo doc --all` before advancing to US3.

### Implementation for User Story 2

- [ ] T039 [P] [US2] Expand emplacement output types in `src/output.rs` and `contracts/result.schema.json`: keep `HeadingRef { text, level (1–6), begin, end }`, `section: LineRange`, and add optional structured-block boundary payload for note targets inside list/table/quote/callout/fenced-code/math blocks, including list-item line context nested under the parent list block (FR-008, FR-009, OR15)
- [ ] T040 [US2] Implement heading-stack plus structured-block emplacement computation in `src/note.rs`: return outermost→innermost headings, section range, and enclosing structured-block boundaries for the resolved target line; for no-heading notes return empty stack with whole-file section (FR-008, FR-009)
- [ ] T041 [US2] Wire emplacement in `src/resolve.rs` to consume phase-3.1 `target_range`/block metadata, populate emplacement only when requested and target is a note, and keep attachment behavior as `emplacement = null/absent` (FR-014)
- [ ] T042 [US2] Add/verify `--emplacement` plumbing in `src/cli.rs`, `src/lib.rs`, and `src/output.rs` so JSON and human outputs include emplacement fields deterministically when requested (FR-008, FR-015, FR-016)

**Checkpoint**: User Stories 1 AND 2 both work independently — targets can be resolved with or without structured emplacement.

**Documentation Gate (Principle IX)**: All emplacement-related symbols MUST have Rustdoc comments. Verify with `cargo doc --all` and ensure no missing-docs warnings before advancing to US3.

---

## Phase 5: User Story 3 - Machine-friendly output for agents and wrappers (Priority: P3)

**Goal**: Provide a deterministic, compact machine-readable output mode alongside a human-readable mode, and expose an in-process C-ABI/FFI boundary so hosts (at minimum .NET and Node.js) can run many consecutive resolutions without a per-call process spawn.

**Independent Test**: Invoke the resolver in machine mode against known inputs and assert a single parseable record with stable field names and correct outcome; run identical inputs twice and assert byte-for-byte identical output; load the `cdylib` and drive one session through many resolutions, asserting the returned JSON matches the CLI output.

### Tests for User Story 3 (write FIRST, ensure they FAIL before implementation) ⚠️

- [ ] T043 [P] [US3] Contract test asserting machine-mode output is byte-for-byte identical across two runs on identical inputs, including records with `target_range` and emplacement/structured-block fields, in `tests/contract/determinism.rs` (SC-003, FR-016)
- [ ] T044 [P] [US3] Contract test asserting `ambiguous` lists sorted `candidates`, exits 4, and emitted stdout validates against the updated `contracts/result.schema.json` (including `target_range` and emplacement shapes) in `tests/contract/machine_output.rs` (US3 AS1, AS2; FR-011, FR-015)
- [ ] T045 [P] [US3] FFI contract test in `tests/ffi/session.rs`: load the `cdylib`, call `olr_session_open` once, run many consecutive `olr_resolve` calls reusing the loaded vault index, free each string via `olr_string_free`, close via `olr_session_close`, and assert each returned JSON (including range/emplacement fields) is byte-for-byte identical to CLI `--format json` output and `out_status` mirrors exit code (SC-007, FR-021a)

**Documentation Gate (Principle IX)**: All FFI symbols and related types in `src/ffi.rs` (opaque `OlrSession`, all `extern "C"` functions, `ResolverSession`) MUST have Rustdoc comments explaining purpose, C-ABI semantics, memory ownership (caller/callee), UTF-8 assumptions, and error handling. The generated `include/obsidian_link_resolver.h` MUST be reviewed to ensure C function signatures and docstrings are clear. Verify with `cargo doc --all` before advancing to Polish.

### Implementation for User Story 3

- [ ] T046 [US3] Harden deterministic serialization in `src/output.rs`: guarantee fixed field order for all primary record fields (`target_range`, emplacement payloads, candidates), exclude timestamps/non-deterministic values, and keep compact single-line output byte-stable for identical inputs (FR-016, SC-003)
- [ ] T047 [US3] Implement/finish human-readable mode in `src/output.rs` and mode selection in `src/cli.rs`; ensure human mode prints the same semantic values as JSON mode (including range/emplacement details) on stdout (FR-015, contracts/cli.md Human-mode output)
- [ ] T048 [US3] Implement the C-ABI/FFI surface in `src/ffi.rs`: opaque `OlrSession`, and `extern "C"` functions `olr_version`, `olr_session_open(vault_root, out_status)`, `olr_resolve(session, link, context_path, with_emplacement, out_status)`, `olr_string_free`, `olr_session_close`; all strings NUL-terminated UTF-8 and caller-owned, returning error status rather than crashing on null/invalid-UTF-8 input (contracts/ffi.md)
- [ ] T049 [US3] Implement `ResolverSession` state in `src/ffi.rs` so the vault index is loaded once per `olr_session_open` and reused across consecutive `olr_resolve` calls, and `olr_resolve` returns the same compact JSON string the CLI emits (including `target_range` and emplacement fields) with `out_status` mirroring exit code (FR-021, FR-021a)
- [ ] T050 [US3] Verify `build.rs`/`cbindgen` generates `include/obsidian_link_resolver.h` matching the `extern "C"` surface and signatures documented in `contracts/ffi.md`

**Checkpoint**: All three user stories are independently functional — resolution, structured emplacement, deterministic machine/human output, and the in-process FFI embedding surface.

**Documentation Gate (Principle IX)**: All FFI and output-related symbols MUST have Rustdoc comments. Verify with `cargo doc --all` and ensure no missing-docs warnings before Polish phase.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Performance gate, documentation, and final validation across all stories

**Documentation Gate (Principle IX)**: Final verification phase. Run `cargo doc --all` and fix any remaining missing-docs warnings. Run `cargo clippy -- -W missing-docs` and resolve all findings. Generate and review the full API documentation to ensure all public symbols have clear, complete Rustdoc comments with purpose, constraints, parameters, return values, and examples where helpful. This is a BLOCKING gate before release.

- [ ] T051 [P] Generate a synthetic ~5,000-note benchmark vault under `tests/fixtures/bench-vault/` (including an `.obsidian/` directory) via a reproducible helper script/module, providing the representative-vault corpus for the warm-run latency gate (SC-005; addresses analysis G1; consumed by T052 and the release gate in T008)
- [ ] T052 [P] Implement the `criterion` warm-run latency benchmark in `benches/resolve.rs` against the ~5,000-note benchmark vault from T051, asserting warm-run p50 ≤100 ms as the CI regression/release gate (SC-005)
- [ ] T053 [P] Write `README.md` usage documentation covering CLI invocation, exit-code contract, `target_range` semantics, structured emplacement (including structured-block boundaries), and the three integration surfaces (CLI protocol, JSON schema, C ABI/FFI)
- [ ] T054 [P] Run `cargo doc --all --no-deps` and review generated HTML docs; ensure all public types/functions/modules (including OR15/target-range/emplacement symbols) are documented and rendered correctly
- [ ] T055 Run `quickstart.md` validation scenarios end-to-end (including OR1-OR15 and structured-block emplacement cases) against fixture vault(s) and confirm each `status`/exit code and contract field expectations
- [ ] T056 [P] Run `cargo fmt --check` and `cargo clippy -- -D warnings` and resolve any findings
- [ ] T057 [P] Run `cargo clippy -- -W missing-docs` and resolve any missing documentation warnings across all modules
- [ ] T058 Review path handling for read-only operation, no traversal outside resolved vault root, and structured-block scanning safety on malformed markdown inputs (security hardening; Assumptions: read-only operation)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **User Stories (Phase 3, 3.1, 4, 5)**: All depend on Foundational completion
  - US1 (P1) has no dependency on other stories
  - Phase 3.1 extends US1 to updated OR1-OR15 and `target_range` semantics before US2 and is a mandatory re-baselining gate for the post-724676f requirement delta
  - US2 (P2) builds on Phase 3.1 structured-block scanning and resolve primitives
  - US3 (P3) builds on US1/US2 output+resolve contracts and FFI reuse
- **Polish (Phase 6)**: Depends on all targeted user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Starts after Foundational — independently testable MVP
- **Phase 3.1 (US1 Delta)**: Starts after initial US1 completion to align implementation with updated OR1-OR15 + range requirements
- **User Story 2 (P2)**: Starts after Phase 3.1 (reuses enriched `note.rs` scanner + `resolve.rs`) — independently testable via `--emplacement`
- **User Story 3 (P3)**: Starts after US1/Phase 3.1 (reuses `output.rs`/`resolve.rs`); emplacement in FFI/JSON benefits from US2 — independently testable via machine mode + FFI

### Within Each User Story

- Tests are written FIRST and must FAIL before implementation (Constitution Principle III)
- Link/vault types and parsing before the resolve pipeline
- Resolve pipeline before CLI/FFI adapters and output

### Parallel Opportunities

- All Setup tasks marked [P] (T003–T008) can run in parallel
- Foundational type-definition tasks T010, T011, T012, T014 marked [P] can run in parallel
- All test-authoring tasks within a story ([P]) can run in parallel before implementation
- Once Foundational completes, US1 starts first; once Phase 3.1 stabilizes scanner/output contracts, US2 and US3 can proceed in parallel where file boundaries do not overlap (note that both touch `note.rs`/`output.rs`/`resolve.rs`)

---

## Parallel Example: User Story 1

```bash
# Author all US1 tests together (they must fail first):
Task: "CLI contract test for resolved outcomes in tests/contract/resolve_cli.rs"
Task: "CLI contract test for unresolved/sub_target in tests/contract/resolve_errors.rs"
Task: "Integration test over fixture vault in tests/integration/us1_resolution.rs"
Task: "Unit tests for link parsing in tests/unit/link_parse.rs"
Task: "Unit tests for vault detection + name resolution in tests/unit/name_resolution.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test User Story 1 independently against the fixture vault
5. Ship as MVP — an agent can follow any Obsidian link to the right file and line

### Incremental Delivery

1. Setup + Foundational → foundation ready
2. Add US1 → test independently → MVP
3. Add Phase 3.1 delta alignment → test independently against OR1-OR15/range scenarios
4. Add US2 → structured emplacement (+ structured-block boundaries) → test independently
5. Add US3 → deterministic machine/human output + FFI embedding → test independently
6. Polish → performance gate, docs, quickstart validation

### Parallel Team Strategy

1. Team completes Setup + Foundational together
2. Once Foundational is done, US1 lands first, then Phase 3.1 reconciles spec/plan deltas; US2 and US3 then extend `note.rs`, `resolve.rs`, and `output.rs` in coordinated sequence

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps each task to its user story for traceability
- Each user story is independently completable and testable
- Verify tests fail before implementing (Constitution Principle III)
- Commit after each task or logical group
- Analysis follow-ups applied: candidate ordering + deterministic enumeration (U1) in T022/T023, vault-relative path serialization (U2) in T023/T027, and the ~5,000-note benchmark-vault fixture (G1) as T051.
- Phase 3.1 (T030–T036) captures the post-724676f requirement delta: OR1-OR15 explicit coverage, supported-target filtering, and canonical `target_range` semantics for heading/block/structured-block targets.
- FR-021b (no hard dependency precluding .NET/Node.js integration) is enforced by design and reviewed during T048–T050; the FFI contract test (T045) exercises the C-ABI surface both hosts rely on (analysis G2, satisfied-by).
- SC-006 (wrapping the CLI in an MCP server/skill) is satisfied by deterministic machine-mode contract + schema validation in T043/T044 (analysis G3, satisfied-by).
