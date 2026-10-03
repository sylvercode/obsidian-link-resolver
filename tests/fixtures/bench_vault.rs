//! Synthetic benchmark vault generation for the warm-run latency gate.

use std::fs;
use std::path::{Path, PathBuf};

const BENCH_ROOT: &str = "tests/fixtures/bench-vault";
const TARGET_LINK: &str = "[[Target Note#Target Heading]]";

/// Ensure the synthetic benchmark vault exists and return its key paths.
///
/// The generated corpus contains a `.obsidian/` directory, a context note,
/// a uniquely named target note with a heading target, and approximately
/// 5,000 additional markdown notes distributed across nested directories so the
/// benchmark exercises realistic vault enumeration behavior.
pub fn ensure_bench_vault() -> Result<(PathBuf, PathBuf, &'static str), String> {
    let root = PathBuf::from(BENCH_ROOT);
    let sentinel = root.join(".obsidian").join(".bench-vault-ready");

    if !sentinel.is_file() {
        generate_bench_vault(&root, &sentinel)?;
    }

    let context_path = root.join("Context Note.md");
    Ok((root, context_path, TARGET_LINK))
}

fn generate_bench_vault(root: &Path, sentinel: &Path) -> Result<(), String> {
    fs::create_dir_all(root.join(".obsidian"))
        .map_err(|error| format!("failed to create benchmark vault metadata directory: {error}"))?;
    fs::create_dir_all(root.join("bulk"))
        .map_err(|error| format!("failed to create benchmark vault corpus directory: {error}"))?;

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
            format!(
                "failed to create benchmark shard directory '{}': {error}",
                shard_dir.display()
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

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create parent directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    fs::write(path, contents)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}
