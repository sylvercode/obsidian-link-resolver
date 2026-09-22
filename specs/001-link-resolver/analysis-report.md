# Specification Analysis Report

Analysis of `spec.md`, `plan.md`, and `tasks.md`, cross-checked against the constitution and supporting contracts (`contracts/cli.md`, `contracts/result.schema.json`, `data-model.md`). **Read-only analysis — no artifacts were modified.**

Feature: `001-link-resolver` | Date: 2026-09-19

## Findings

| ID | Category | Severity | Location(s) | Summary | Recommendation |
|----|----------|----------|-------------|---------|----------------|
| U1 | Underspecification / Inconsistency | HIGH | spec.md FR-011/FR-016, tasks.md T023/T036 | `ambiguous` candidate list has no stated deterministic ordering, yet SC-003/FR-016 demand byte-for-byte identical output. `walkdir` enumeration order (T022) is not guaranteed stable across runs/filesystems, so unsorted candidates can break determinism. | Add a requirement/task to sort `candidates` (e.g., by vault-relative path) and make note enumeration order deterministic. |
| U2 | Underspecification | MEDIUM | spec.md FR-006, contracts/cli.md examples, tasks.md T027 | `target_path` is shown as vault-relative in every example (`Project Plan.md`, `assets/diagram.png`) but no requirement states relative-vs-absolute. `data-model.md` calls context/vault paths absolute. Absolute paths would defeat cross-run/portable determinism. | State explicitly that `target_path`/`candidates` are vault-relative; add a parse/serialize note in T023/T027. |
| I1 | Inconsistency | MEDIUM | spec.md FR-014, data-model.md, tasks.md T026/T034 | FR-014 says attachments resolve "with ... an empty heading stack" (implies an emplacement object present), but plan/data-model/tasks say attachments get **no** `emplacement` object at all. | Align wording: either "no emplacement" everywhere or define an empty-stack object; pick one and update FR-014. |
| G1 | Coverage Gap | MEDIUM | tasks.md T044, T014 | SC-005 warm-run p50 gate is benchmarked "against a ~5,000-note vault," but no task creates/generates that large vault. T014 builds only the small link-form fixture. | Add a task to generate/provision the ~5,000-note benchmark vault used by T044/T008. |
| G2 | Coverage Gap | LOW | spec.md FR-021b | "MUST NOT introduce hard deps precluding .NET/Node integration" has no explicit task or CI gate; validated only implicitly by design. | Optional: add a checklist/CI note asserting no such dependency is introduced. |
| G3 | Coverage Gap | LOW | spec.md SC-006 | "Integrator can wrap the CLI in an MCP/skill" has no dedicated task; covered only indirectly by schema/determinism tests (T037). | Acceptable; optionally note SC-006 as satisfied-by T037 for traceability. |
| D1 | Duplication | LOW | spec.md FR-005 / FR-005b | Case-insensitive matching is stated in FR-005 and again as its own FR-005b. | Keep FR-005b as the canonical statement; reference it from FR-005 instead of restating. |
| A1 | Ambiguity | LOW | plan.md Summary, tasks.md Phase 3 goal | Prose enumerates "resolved / unresolved / sub-target-not-found / ambiguous" via exit codes, omitting the fifth outcome `error` in that phrasing. | Add `error` to those enumerations for consistency with FR-010's five-outcome set. |

## Coverage Summary

All 21 base functional requirements plus sub-requirements map to at least one task. Representative mapping:

| Requirement Key | Has Task? | Task IDs | Notes |
|-----------------|-----------|----------|-------|
| FR-001..FR-003 (input, wikilink/markdown parse) | Yes | T018, T020, T028 | |
| FR-002a/b/c (dup heading, nested, block id) | Yes | T024, T025, T014 | |
| FR-004 / FR-004a (vault detect / undetermined) | Yes | T019, T021 | |
| FR-005 / FR-005a / FR-005b (name resolution) | Yes | T019, T023 | See U1 (candidate ordering) |
| FR-006 / FR-007 (target path + line) | Yes | T025, T026 | See U2 (path relativity) |
| FR-008 / FR-009 (emplacement, section end) | Yes | T031, T033, T035 | |
| FR-010 / FR-011 / FR-017 (5 outcomes, exit codes) | Yes | T012, T013, T029 | |
| FR-012..FR-014 (display text, embed, attachment) | Yes | T026 | See I1 |
| FR-015 / FR-016 (machine+human, determinism) | Yes | T027, T039, T040, T036 | See U1 |
| FR-018 / FR-019 / FR-020 (streams, reason, 1-based) | Yes | T016, T029, T012 | |
| FR-021 / FR-021a (integration surfaces, FFI session) | Yes | T038, T041, T042 | |
| FR-021b (no precluding hard deps) | Partial | — | G2: no explicit task |
| SC-001..SC-005, SC-007, SC-008 | Yes | T017, T036, T044, T008, T046, T038 | SC-005 needs large vault (G1) |
| SC-006 (MCP/skill wrap) | Partial | T037 (indirect) | G3 |

## Constitution Alignment Issues

None. All eight principles (Performance, CLI/Output, Test-First, Integration Testing, Observability/Versioning, Interoperability, Reproducible Devcontainer, CI/Release Gating) are explicitly addressed by plan constraints and tasks (T005–T008, T044). No MUST violations detected.

## Unmapped Tasks

None. Every task (T001–T048) traces to a requirement, success criterion, or constitution principle.

## Metrics

- **Total Requirements**: 29 (21 FR + 8 sub-requirements) + 8 Success Criteria
- **Total Tasks**: 48 (T001–T048)
- **Coverage**: ~97% (28/29 FR with >=1 task; FR-021b design-only). All SC covered (SC-006 indirectly).
- **Ambiguity Count**: 1 (A1)
- **Duplication Count**: 1 (D1)
- **Critical Issues Count**: 0 CRITICAL; 1 HIGH (U1)

## Next Actions

- No CRITICAL issues — implementation is not blocked.
- Recommended before implementation: resolve **U1** (candidate ordering / determinism) and **U2** (path relativity), since both directly affect the FR-016/SC-003 byte-for-byte determinism guarantee shared by the FFI and CLI contracts.
- Suggested remediation:
  - Edit `spec.md` to state candidate ordering (FR-011) and `target_path` relativity (FR-006).
  - Edit `tasks.md` to add a large-vault fixture task (G1) and a candidate-sort step in T023.
  - Optionally reflect the attachment-emplacement shape (I1) in the schema/data-model.
