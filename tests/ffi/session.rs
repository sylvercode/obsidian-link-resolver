use libloading::Library;
use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::process::Command;

#[repr(C)]
struct OlrSession;

type OlrVersionFn = unsafe fn() -> *mut c_char;
type OlrSessionOpenFn = unsafe fn(*const c_char, *mut i32) -> *mut OlrSession;
type OlrResolveFn =
    unsafe fn(*mut OlrSession, *const c_char, *const c_char, i32, *mut i32) -> *mut c_char;
type OlrStringFreeFn = unsafe fn(*mut c_char);
type OlrSessionCloseFn = unsafe fn(*mut OlrSession);

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
