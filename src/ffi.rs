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

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;

/// Status code returned by the FFI entry points.
pub type OlrStatus = i32;

/// FFI status: resolved.
pub const OLR_STATUS_RESOLVED: OlrStatus = 0;
/// FFI status: error.
pub const OLR_STATUS_ERROR: OlrStatus = 1;
/// FFI status: unresolved.
pub const OLR_STATUS_UNRESOLVED: OlrStatus = 2;
/// FFI status: sub-target not found.
pub const OLR_STATUS_SUB_TARGET_NOT_FOUND: OlrStatus = 3;
/// FFI status: ambiguous.
pub const OLR_STATUS_AMBIGUOUS: OlrStatus = 4;

/// Opaque resolver session handle.
pub struct OlrSession {
    inner: ResolverSession,
}

/// Cached session state for repeated in-process resolutions.
struct ResolverSession {
    root: Option<String>,
}

impl ResolverSession {
    fn new(vault_root: Option<&str>) -> Result<Self, String> {
        if let Some(root) = vault_root {
            let root_path = Path::new(root);
            if !root_path.exists() {
                return Err(format!("vault root '{root}' does not exist"));
            }
            if !root_path.is_dir() {
                return Err(format!("vault root '{root}' is not a directory"));
            }
        }

        Ok(Self {
            root: vault_root.map(str::to_owned),
        })
    }

    fn resolve(
        &self,
        link: &str,
        context_path: &str,
        with_emplacement: bool,
    ) -> crate::output::ResolutionTarget {
        crate::resolve(link, context_path, self.root.as_deref(), with_emplacement)
    }
}

/// Get the semantic version string of the library.
///
/// # Returns
///
/// A NUL-terminated UTF-8 string encoding the version (e.g., `"0.1.0"`).
/// The pointer is valid for the lifetime of the program; do not free it.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn write_status(out_status: *mut OlrStatus, status: OlrStatus) {
    if !out_status.is_null() {
        unsafe {
            *out_status = status;
        }
    }
}

fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let c_string = unsafe { CStr::from_ptr(ptr) };
    c_string.to_str().ok().map(str::to_owned)
}

fn make_owned_string(value: impl AsRef<str>) -> *mut c_char {
    CString::new(value.as_ref())
        .unwrap_or_else(|_| CString::new("invalid utf-8").unwrap())
        .into_raw()
}

/// Return the library's semantic version as a caller-owned UTF-8 string.
#[unsafe(no_mangle)]
pub extern "C" fn olr_version() -> *mut c_char {
    make_owned_string(version())
}

/// Open a resolver session for the given vault root.
///
/// If `vault_root` is `NULL`, the session defers vault detection to each resolution using the
/// context file path. If a specific vault root is provided, it is validated before the session is
/// returned. The returned session is opaque to callers and must be released with `olr_session_close`.
#[unsafe(no_mangle)]
pub extern "C" fn olr_session_open(
    vault_root: *const c_char,
    out_status: *mut OlrStatus,
) -> *mut OlrSession {
    let requested_root = cstr_to_string(vault_root);

    match ResolverSession::new(requested_root.as_deref()) {
        Ok(session) => {
            write_status(out_status, OLR_STATUS_RESOLVED);
            Box::into_raw(Box::new(OlrSession { inner: session }))
        }
        Err(reason) => {
            let message = format!("{reason}");
            _ = message;
            write_status(out_status, OLR_STATUS_ERROR);
            std::ptr::null_mut()
        }
    }
}

/// Resolve a single Obsidian link within a context file using the session's vault index.
#[unsafe(no_mangle)]
pub extern "C" fn olr_resolve(
    session: *mut OlrSession,
    link: *const c_char,
    context_path: *const c_char,
    with_emplacement: i32,
    out_status: *mut OlrStatus,
) -> *mut c_char {
    if session.is_null() {
        write_status(out_status, OLR_STATUS_ERROR);
        return std::ptr::null_mut();
    }

    let link_str = match cstr_to_string(link) {
        Some(value) => value,
        None => {
            write_status(out_status, OLR_STATUS_ERROR);
            return std::ptr::null_mut();
        }
    };

    let context_str = match cstr_to_string(context_path) {
        Some(value) => value,
        None => {
            write_status(out_status, OLR_STATUS_ERROR);
            return std::ptr::null_mut();
        }
    };

    let result = unsafe { &*session }
        .inner
        .resolve(&link_str, &context_str, with_emplacement != 0);
    let out = result.to_json().unwrap_or_else(|_| {
        let fallback = crate::output::ResolutionTarget {
            status: crate::output::Status::Error,
            target_path: None,
            target_range: None,
            is_embed: false,
            display_text: None,
            candidates: None,
            reason: Some("failed to serialize result".to_string()),
            emplacement: None,
        };
        fallback.to_json().unwrap_or_else(|_| {
            "{\"status\":\"error\",\"reason\":\"failed to serialize result\"}".to_string()
        })
    });

    write_status(out_status, result.exit_code() as OlrStatus);
    make_owned_string(out)
}

/// Free a string allocated by the library in the FFI boundary.
#[unsafe(no_mangle)]
pub extern "C" fn olr_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

/// Close and free an open resolver session.
#[unsafe(no_mangle)]
pub extern "C" fn olr_session_close(session: *mut OlrSession) {
    if session.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(session));
    }
}
