# FFI / C-ABI Contract: Obsidian Link Resolver

This contract defines the C-compatible ABI/FFI boundary that lets host
applications embed the resolver in-process (FR-021, FR-021a) without spawning a
process per call. It is a stable interface governed by the constitution's
versioning policy (breaking ABI changes are MAJOR). The result payload crossing
this boundary is the same record defined by
[result.schema.json](result.schema.json) — there is a single source of truth for
the result shape shared with the [CLI contract](cli.md).

## Library artifact

- Built as a Rust `cdylib`: `libobsidian_link_resolver.so` (Linux),
  `libobsidian_link_resolver.dylib` (macOS), `obsidian_link_resolver.dll` (Windows).
- A C header `include/obsidian_link_resolver.h` is generated from the Rust
  `extern "C"` surface with `cbindgen`.
- No runtime dependency is required to load the library (Principle I, FR-021b).

## Types

| C type | Meaning |
|--------|---------|
| `OlrSession` | Opaque struct; a loaded vault index reused across resolutions. Callers hold only `OlrSession*` and never dereference it. |
| `OlrStatus` (int32) | Numeric outcome mirroring the CLI exit-status contract: `0` resolved, `1` error, `2` unresolved, `3` sub_target_not_found, `4` ambiguous. |

## Functions

All strings are NUL-terminated UTF-8. The library owns every string it returns;
the caller MUST release it with `olr_string_free`. All pointer parameters are
non-null unless stated; functions return an error status rather than crashing on
null/invalid UTF-8 input.

```c
// Semantic version of the ABI/library, e.g. "0.1.0". Caller-owned via olr_string_free.
const char* olr_version(void);

// Open a resolver session for a vault.
//   vault_root: explicit vault root, or NULL to auto-detect from context files (FR-004).
// Returns NULL on failure (e.g. vault could not be determined); *out_status carries the reason code.
OlrSession* olr_session_open(const char* vault_root, int32_t* out_status);

// Resolve one link within a context file against an open session.
//   link:        raw Obsidian link string (e.g. "[[Project Plan#Milestones]]").
//   context_path: path to the markdown file that contains the link.
//   with_emplacement: non-zero to include structured emplacement (FR-008).
//   out_status:  set to the OlrStatus outcome (mirrors exit codes).
// Returns a caller-owned UTF-8 JSON string validating against result.schema.json,
// byte-for-byte identical to the CLI --format json output for the same inputs (FR-016, SC-003).
char* olr_resolve(OlrSession* session,
                  const char* link,
                  const char* context_path,
                  int32_t with_emplacement,
                  int32_t* out_status);

// Release a string previously returned by the library.
void olr_string_free(char* s);

// Close a session and free its vault index.
void olr_session_close(OlrSession* session);
```

## Usage contract

1. `olr_session_open` once per vault root; reuse the returned session for many
   consecutive `olr_resolve` calls (FR-021a, SC-007). The vault index is loaded
   once and reused; no per-call process spawn occurs.
2. Each `olr_resolve` returns the primary result as a compact single-line JSON
   string (same fields/order as [result.schema.json](result.schema.json)) and
   writes the numeric outcome to `out_status`.
3. Free each returned string with `olr_string_free`; close the session with
   `olr_session_close`. Failing to free leaks memory owned by the library.
4. Determinism: no timestamps or other non-deterministic fields appear in the
   returned record (FR-016). Identical inputs produce identical JSON strings.

## Cross-technology consumption (informative)

These illustrate that the ABI is consumable from the required stacks (SC-007);
ready-made bindings are out of scope for v1.

- **.NET**: `[DllImport]`/`LibraryImport` P/Invoke against the `cdylib`, marshalling
  `const char*`/`char*` as UTF-8 pointers and calling `olr_string_free` to release
  returned strings.
- **Node.js**: an N-API native addon or an FFI package (e.g. `ffi`/koffi) that
  binds the same four functions and frees returned strings via `olr_string_free`.

## Versioning

- The function set, their signatures, and `OlrStatus` values are the ABI surface.
- Additive, backward-compatible functions are MINOR; signature or semantics
  changes are MAJOR. `olr_version` reports the library's semantic version.
