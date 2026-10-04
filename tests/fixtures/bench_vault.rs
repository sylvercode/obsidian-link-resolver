//! Synthetic benchmark vault generation for the warm-run latency gate.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Root directory where the generated benchmark vault is created.
const BENCH_ROOT: &str = "tests/fixtures/bench-vault";
/// Canonical link resolved repeatedly by benchmark and latency-gate tests.
const TARGET_LINK: &str = "[[Target Note#Target Heading]]";

/// Ensure the synthetic benchmark vault exists and return its key paths.
///
/// The generated corpus contains a `.obsidian/` directory, a context note,
/// a uniquely named target note with a heading target, and approximately
/// 5,000 additional markdown notes distributed across nested directories so the
/// benchmark exercises realistic vault enumeration behavior.
///
/// # Returns
///
/// A tuple `(vault_root, context_path, target_link)` used by latency benchmarks.
pub fn ensure_bench_vault() -> io::Result<(PathBuf, PathBuf, &'static str)> {
    let root = PathBuf::from(BENCH_ROOT);
    let metadata_dir = root.join(".obsidian");
    let sentinel = metadata_dir.join(".bench-vault-ready");
    let lock_path = metadata_dir.join(".bench-vault.lock");

    if !sentinel.is_file() {
        fs::create_dir_all(&metadata_dir).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "failed to create benchmark vault metadata directory '{}': {error}",
                    metadata_dir.display()
                ),
            )
        })?;

        let lock_created = match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(_) => true,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
            Err(error) => return Err(error),
        };

        if !lock_created {
            while lock_path.exists() && !sentinel.is_file() {
                std::thread::yield_now();
            }
        }

        if !sentinel.is_file() {
            generate_bench_vault(&root, &sentinel)?;
        }

        if lock_path.exists() {
            let _ = fs::remove_file(&lock_path);
        }
    }

    let context_path = root.join("Context Note.md");
    Ok((root, context_path, TARGET_LINK))
}

/// Generate the benchmark vault corpus and write the readiness sentinel.
///
/// # Parameters
///
/// - `root`: Root directory where the synthetic vault is created.
/// - `sentinel`: File path written last to indicate generation completed.
///
/// # Returns
///
/// `Ok(())` after successful generation, or an `io::Error` if setup fails.
fn generate_bench_vault(root: &Path, sentinel: &Path) -> io::Result<()> {
    fs::create_dir_all(root.join(".obsidian")).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to create benchmark vault metadata directory: {error}"),
        )
    })?;
    fs::create_dir_all(root.join("bulk")).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to create benchmark vault corpus directory: {error}"),
        )
    })?;

    write_file(
        &root.join("Context Note.md"),
        &format!("# Context Note\n\nThis note holds the benchmark link target.\n\n{TARGET_LINK}\n"),
    )?;
    write_file(
        &root.join("Target Note.md"),
        "# Target Note\n\nWarm-run benchmark target note.\n\n## Target Heading\n\nThe benchmark resolves to this heading.\n",
    )?;

    for shard in 0..100 {
        let shard_dir = root.join("bulk").join(format!("{shard:02}"));
        fs::create_dir_all(&shard_dir).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "failed to create benchmark shard directory '{}': {error}",
                    shard_dir.display()
                ),
            )
        })?;

        for offset in 0..50 {
            let note_number = shard * 50 + offset;
            let note_path = shard_dir.join(format!("bench-note-{note_number:04}.md"));
            if note_path.is_file() {
                continue;
            }

            let note_contents = format!(
                "# Bench Note {note_number}\n\nGenerated corpus note {note_number}.\n\n## Detail {note_number}\n\nPayload line {note_number}.\n"
            );
            write_file(&note_path, &note_contents)?;
        }
    }

    write_file(sentinel, "ready\n")?;
    Ok(())
}

/// Write one file, ensuring all parent directories exist first.
///
/// # Parameters
///
/// - `path`: Destination file path to write.
/// - `contents`: File contents to persist.
///
/// # Returns
///
/// `Ok(())` on success, or an `io::Error` with contextual path details.
fn write_file(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "failed to create parent directory '{}': {error}",
                    parent.display()
                ),
            )
        })?;
    }
    fs::write(path, contents).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to write '{}': {error}", path.display()),
        )
    })
}
