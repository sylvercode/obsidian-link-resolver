//! C-compatible ABI/FFI boundary for in-process embedding.
//!
//! This module exposes the link resolver as a C-compatible shared library (`cdylib`),
//! allowing host processes in other languages (e.g., .NET, Node.js, C/C++, Python) to
//! load a vault index once and run many consecutive resolutions without per-call process spawn.
//!
//! ## Overview
//!
//! The FFI surface provides three main entry points:
//! - `olr_version()`: Query the library version
//! - `olr_session_open()`: Load a vault and create a persistent session handle
//! - `olr_resolve()`: Resolve a link using the loaded vault (can be called many times per session)
//! - `olr_session_close()`: Release the session and free resources
//! - `olr_string_free()`: Free a string allocated by the library
//!
//! All strings are NUL-terminated UTF-8, and the caller owns the returned pointers.
//! Detailed signatures and semantics are documented in `contracts/ffi.md`
//! and the generated C header `include/obsidian_link_resolver.h` (auto-generated via `cbindgen`).

/// Get the semantic version string of the library.
///
/// # Returns
///
/// A NUL-terminated UTF-8 string encoding the version (e.g., `"0.1.0"`).
/// The pointer is valid for the lifetime of the program; do not free it.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
