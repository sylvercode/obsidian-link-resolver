use libloading::Library;
use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[repr(C)]
struct OlrSession;

type OlrVersionFn = unsafe extern "C" fn() -> *mut c_char;
type OlrSessionOpenFn = unsafe extern "C" fn(*const c_char, *mut i32) -> *mut OlrSession;
type OlrResolveFn = unsafe extern "C" fn(
    *mut OlrSession,
    *const c_char,
    *const c_char,
    i32,
    *mut i32,
) -> *mut c_char;
type OlrStringFreeFn = unsafe extern "C" fn(*mut c_char);
type OlrSessionCloseFn = unsafe extern "C" fn(*mut OlrSession);

fn library_file_path() -> PathBuf {
    let filename = format!(
        "{}obsidian_link_resolver{}",
        env::consts::DLL_PREFIX,
        env::consts::DLL_SUFFIX
    );

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join(filename)
}

fn ensure_cdylib_built() -> PathBuf {
    let library_path = library_file_path();
    if library_path.exists() {
        return library_path;
    }

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["build", "--lib"])
        .status()
        .expect("failed to invoke cargo build --lib for the CDYLIB");

    assert!(
        status.success(),
        "cargo build --lib failed before loading the CDYLIB"
    );

    let built_path = library_file_path();
    assert!(
        built_path.exists(),
        "cdylib should be built at {}",
        built_path.display()
    );
    built_path
}

fn load_library() -> Library {
    let library_path = ensure_cdylib_built();
    unsafe { Library::new(library_path).expect("cdylib should be built") }
}

#[test]
fn ffi_session_can_load_and_resolve_many_consecutive_links() {
    let library = load_library();
    let olr_version: OlrVersionFn = unsafe { *library.get(b"olr_version\0").unwrap() };
    let olr_session_open: OlrSessionOpenFn =
        unsafe { *library.get(b"olr_session_open\0").unwrap() };
    let olr_resolve: OlrResolveFn = unsafe { *library.get(b"olr_resolve\0").unwrap() };
    let olr_string_free: OlrStringFreeFn = unsafe { *library.get(b"olr_string_free\0").unwrap() };
    let olr_session_close: OlrSessionCloseFn =
        unsafe { *library.get(b"olr_session_close\0").unwrap() };

    let version_ptr = unsafe { olr_version() };
    let version = unsafe { CStr::from_ptr(version_ptr) }.to_str().unwrap();
    assert!(!version.is_empty());
    unsafe { olr_string_free(version_ptr) };

    let fixture_root = format!("{}/tests/fixtures/vault", env!("CARGO_MANIFEST_DIR"));
    let vault_root = CString::new(format!("{}/", fixture_root)).unwrap();
    let context_path = format!("{}/notes/a.md", fixture_root);

    let mut out_status = 0_i32;
    let session = unsafe { olr_session_open(vault_root.as_ptr(), &mut out_status) };
    assert!(!session.is_null(), "session should be opened");

    for _ in 0..3 {
        let link = CString::new("[[Project Plan#Milestones]]").unwrap();
        let context = CString::new(context_path.clone()).unwrap();
        let mut status = 0_i32;
        let json_ptr =
            unsafe { olr_resolve(session, link.as_ptr(), context.as_ptr(), 0, &mut status) };

        assert!(!json_ptr.is_null(), "resolve should return a JSON string");
        let json = unsafe { CStr::from_ptr(json_ptr) }.to_str().unwrap();
        assert!(json.contains("\"status\":\"resolved\""));
        unsafe { olr_string_free(json_ptr) };
        assert_eq!(status, 0);
    }

    unsafe { olr_session_close(session) };
}

#[test]
fn ffi_session_reuses_cached_index_after_open() {
    let library = load_library();
    let olr_session_open: OlrSessionOpenFn =
        unsafe { *library.get(b"olr_session_open\0").unwrap() };
    let olr_resolve: OlrResolveFn = unsafe { *library.get(b"olr_resolve\0").unwrap() };
    let olr_string_free: OlrStringFreeFn = unsafe { *library.get(b"olr_string_free\0").unwrap() };
    let olr_session_close: OlrSessionCloseFn =
        unsafe { *library.get(b"olr_session_close\0").unwrap() };

    let temp = tempdir().unwrap();
    let vault_root = temp.path().join("vault");
    std::fs::create_dir_all(vault_root.join(".obsidian")).unwrap();
    std::fs::create_dir_all(vault_root.join("notes")).unwrap();
    std::fs::write(vault_root.join("notes").join("ctx.md"), "# Context\n").unwrap();
    std::fs::write(vault_root.join("notes").join("existing.md"), "# Existing\n").unwrap();

    let vault_root_cstr = CString::new(vault_root.to_string_lossy().into_owned()).unwrap();
    let context_cstr = CString::new(
        vault_root
            .join("notes")
            .join("ctx.md")
            .to_string_lossy()
            .into_owned(),
    )
    .unwrap();

    let mut open_status = 0_i32;
    let session = unsafe { olr_session_open(vault_root_cstr.as_ptr(), &mut open_status) };
    assert!(!session.is_null(), "session should be opened");
    assert_eq!(open_status, 0);

    let missing_link = CString::new("[[newly-added]]").unwrap();
    let mut first_status = 0_i32;
    let first_json_ptr = unsafe {
        olr_resolve(
            session,
            missing_link.as_ptr(),
            context_cstr.as_ptr(),
            0,
            &mut first_status,
        )
    };
    assert!(!first_json_ptr.is_null());
    let first_json = unsafe { CStr::from_ptr(first_json_ptr) }
        .to_str()
        .unwrap()
        .to_string();
    unsafe { olr_string_free(first_json_ptr) };
    assert_eq!(first_status, 2);
    assert!(first_json.contains("\"status\":\"unresolved\""));

    std::fs::write(vault_root.join("newly-added.md"), "# Added later\n").unwrap();

    let mut second_status = 0_i32;
    let second_json_ptr = unsafe {
        olr_resolve(
            session,
            missing_link.as_ptr(),
            context_cstr.as_ptr(),
            0,
            &mut second_status,
        )
    };
    assert!(!second_json_ptr.is_null());
    let second_json = unsafe { CStr::from_ptr(second_json_ptr) }
        .to_str()
        .unwrap()
        .to_string();
    unsafe { olr_string_free(second_json_ptr) };

    // The session should keep using the index created at open time.
    assert_eq!(second_status, 2);
    assert!(second_json.contains("\"status\":\"unresolved\""));

    unsafe { olr_session_close(session) };
}

#[test]
fn ffi_session_open_rejects_invalid_utf8_root() {
    let library = load_library();
    let olr_session_open: OlrSessionOpenFn =
        unsafe { *library.get(b"olr_session_open\0").unwrap() };

    let invalid_root = CString::from_vec_with_nul(vec![0xFF, 0x00]).unwrap();

    let mut out_status = 0_i32;
    let session = unsafe { olr_session_open(invalid_root.as_ptr(), &mut out_status) };

    assert!(session.is_null(), "invalid UTF-8 root should fail open");
    assert_eq!(
        out_status, 1,
        "invalid UTF-8 root should return error status"
    );
}
