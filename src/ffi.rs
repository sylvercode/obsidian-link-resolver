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
use std::sync::Mutex;

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
    /// Internal session state used by the Rust implementation.
    inner: ResolverSession,
}

/// Cached session state for repeated in-process resolutions.
struct ResolverSession {
    /// Optional explicit vault root supplied when the session was opened.
    root: Option<String>,
    /// Lazily reused vault index bound to the active root/context.
    cached_vault: Mutex<Option<crate::vault::Vault>>,
}

impl ResolverSession {
    /// Create a new session and optionally pre-enumerate an explicit vault.
    ///
    /// # Parameters
    ///
    /// - `vault_root`: Optional explicit vault root path supplied by the caller.
    ///
    /// # Returns
    ///
    /// A ready resolver session with validated root and optional preloaded index.
    fn new(vault_root: Option<&str>) -> Result<Self, String> {
        let canonical_root = if let Some(root) = vault_root {
            let root_path = Path::new(root);
            if !root_path.exists() {
                return Err(format!("vault root '{root}' does not exist"));
            }
            if !root_path.is_dir() {
                return Err(format!("vault root '{root}' is not a directory"));
            }

            Some(
                std::fs::canonicalize(root_path)
                    .map_err(|error| format!("failed to read vault root '{root}': {error}"))?
                    .to_string_lossy()
                    .into_owned(),
            )
        } else {
            None
        };

        let cached_vault = if let Some(root) = canonical_root.as_deref() {
            Some(crate::vault::Vault {
                root: root.to_string(),
                source: crate::vault::VaultSource::Explicit,
                entries: crate::vault::enumerate_vault(root)?,
            })
        } else {
            None
        };

        Ok(Self {
            root: canonical_root,
            cached_vault: Mutex::new(cached_vault),
        })
    }

    /// Resolve one link using the session-scoped vault cache.
    ///
    /// # Parameters
    ///
    /// - `link`: Raw Obsidian link text.
    /// - `context_path`: Path to the file containing the link.
    /// - `with_emplacement`: Whether structured emplacement data should be included.
    ///
    /// # Returns
    ///
    /// The resolution output for the provided request.
    fn resolve(
        &self,
        link: &str,
        context_path: &str,
        with_emplacement: bool,
    ) -> crate::output::ResolutionTarget {
        let vault = match self.vault_for_context(context_path) {
            Ok(vault) => vault,
            Err(reason) => {
                return crate::output::ResolutionTarget {
                    status: crate::output::Status::Error,
                    target_path: None,
                    target_range: None,
                    is_embed: false,
                    display_text: None,
                    candidates: None,
                    reason: Some(reason),
                    emplacement: None,
                };
            }
        };

        crate::resolve_with_vault(link, context_path, &vault, with_emplacement)
    }

    /// Return a vault view valid for the provided context path, refreshing cache if needed.
    ///
    /// # Parameters
    ///
    /// - `context_path`: Path to the context file requiring vault-scoped resolution.
    ///
    /// # Returns
    ///
    /// A vault index that can safely resolve links for the given context.
    fn vault_for_context(&self, context_path: &str) -> Result<crate::vault::Vault, String> {
        let context_abs = std::fs::canonicalize(context_path)
            .map_err(|error| format!("failed to read context path '{context_path}': {error}"))?;

        let mut cached = self
            .cached_vault
            .lock()
            .map_err(|_| "session cache lock poisoned".to_string())?;

        if let Some(vault) = cached.as_ref() {
            if context_abs.starts_with(Path::new(&vault.root)) {
                return Ok(vault.clone());
            }

            if self.root.is_some() {
                return Err(format!(
                    "context path '{context_path}' is outside the explicit vault root '{}', refusing to resolve links",
                    vault.root
                ));
            }
        }

        let detected = crate::vault::detect_root(context_path, self.root.as_deref())?;
        let out = detected.clone();
        *cached = Some(detected);
        Ok(out)
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

/// Write an FFI status code to an optional out-parameter.
///
/// # Parameters
///
/// - `out_status`: Optional pointer to writable status storage.
/// - `status`: Status code to write.
fn write_status(out_status: *mut OlrStatus, status: OlrStatus) {
    if !out_status.is_null() {
        unsafe {
            *out_status = status;
        }
    }
}

/// Convert a required C string pointer to a Rust `String`.
///
/// # Parameters
///
/// - `ptr`: Pointer to a NUL-terminated UTF-8 C string.
///
/// # Returns
///
/// `Some(String)` when the pointer is non-null and UTF-8 valid; otherwise `None`.
fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    let c_string = unsafe { CStr::from_ptr(ptr) };
    c_string.to_str().ok().map(str::to_owned)
}

/// Convert an optional C string pointer to `Option<String>`, validating UTF-8.
///
/// # Parameters
///
/// - `ptr`: Optional pointer to a NUL-terminated UTF-8 C string.
///
/// # Returns
///
/// `Ok(None)` for null pointers, `Ok(Some(String))` for valid UTF-8, or `Err(())` on invalid UTF-8.
fn cstr_to_optional_string(ptr: *const c_char) -> Result<Option<String>, ()> {
    if ptr.is_null() {
        return Ok(None);
    }

    let c_string = unsafe { CStr::from_ptr(ptr) };
    c_string
        .to_str()
        .map(str::to_owned)
        .map(Some)
        .map_err(|_| ())
}

/// Allocate a new caller-owned C string from a Rust string-like value.
///
/// # Parameters
///
/// - `value`: Source string content to encode as a C string.
///
/// # Returns
///
/// A heap-allocated C string pointer owned by the caller.
fn make_owned_string(value: impl AsRef<str>) -> *mut c_char {
    CString::new(value.as_ref())
        .unwrap_or_else(|_| CString::new("invalid utf-8").unwrap())
        .into_raw()
}

/// Return the library's semantic version as a caller-owned UTF-8 string.
///
/// # Safety
///
/// The returned pointer is caller-owned and must be released with `olr_string_free` when no
/// longer needed. No additional caller invariants are required beyond respecting the FFI ownership
/// contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn olr_version() -> *mut c_char {
    make_owned_string(version())
}

/// Open a resolver session for the given vault root.
///
/// If `vault_root` is `NULL`, the session defers vault detection to each resolution using the
/// context file path. If a specific vault root is provided, it is validated before the session is
/// returned. The returned session is opaque to callers and must be released with `olr_session_close`.
///
/// # Safety
///
/// `vault_root` must either be `NULL` or a valid pointer to a NUL-terminated UTF-8 string that
/// remains live for the duration of the call. `out_status` must either be `NULL` or point to
/// writable memory for the call result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn olr_session_open(
    vault_root: *const c_char,
    out_status: *mut OlrStatus,
) -> *mut OlrSession {
    let requested_root = match cstr_to_optional_string(vault_root) {
        Ok(root) => root,
        Err(()) => {
            write_status(out_status, OLR_STATUS_ERROR);
            return std::ptr::null_mut();
        }
    };

    match ResolverSession::new(requested_root.as_deref()) {
        Ok(session) => {
            write_status(out_status, OLR_STATUS_RESOLVED);
            Box::into_raw(Box::new(OlrSession { inner: session }))
        }
        Err(_reason) => {
            write_status(out_status, OLR_STATUS_ERROR);
            std::ptr::null_mut()
        }
    }
}

/// Resolve a single Obsidian link within a context file using the session's vault index.
///
/// # Safety
///
/// The caller must pass a valid `session` handle returned by `olr_session_open`, or `NULL` to
/// indicate invalid input. `link`, `context_path`, and `out_status` must each either be `NULL` or
/// point to valid, NUL-terminated UTF-8 strings / writable memory for the duration of the call.
/// The returned pointer is caller-owned and must be freed with `olr_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn olr_resolve(
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

    let result = (&*session)
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
///
/// # Safety
///
/// `ptr` must be either `NULL` or a pointer previously returned by the library via one of the
/// string-producing FFI functions. Passing any other pointer causes undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn olr_string_free(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    let _ = CString::from_raw(ptr);
}

/// Close and free an open resolver session.
///
/// # Safety
///
/// `session` must be either `NULL` or a pointer returned by `olr_session_open`. Passing any other
/// pointer or freeing the same session more than once is undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn olr_session_close(session: *mut OlrSession) {
    if session.is_null() {
        return;
    }

    drop(Box::from_raw(session));
}
