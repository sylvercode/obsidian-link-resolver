//! Deterministic tests for cached vault generations.

#[path = "../../src/cache.rs"]
mod cache;

use cache::VaultCache;
use obsidian_link_resolver::output::Status;
use obsidian_link_resolver::resolve_with_vault;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::oneshot;

/// Create a small vault with one note and return its temporary directory.
fn make_vault() -> TempDir {
    let directory = tempfile::tempdir().expect("temporary vault should be created");
    fs::create_dir(directory.path().join(".obsidian"))
        .expect("vault metadata directory should be created");
    write_note(directory.path(), "Context.md");
    directory
}

/// Create a markdown note beneath a vault root.
fn write_note(root: &Path, relative_path: &str) {
    let note = root.join(relative_path);
    fs::create_dir_all(note.parent().expect("note should have a parent"))
        .expect("note directory should be created");
    fs::write(note, "# Note\n").expect("note should be written");
}

/// Return the paths in one cached generation.
fn indexed_paths(generation: &cache::VaultGeneration) -> Vec<String> {
    generation
        .vault
        .entries
        .iter()
        .map(|entry| entry.rel_path.clone())
        .collect()
}

/// Repeated requests for equivalent canonical roots reuse one generation.
#[tokio::test]
async fn canonical_roots_reuse_the_same_generation() {
    let directory = make_vault();
    let root = directory.path();
    let relative_root = root.join(".");
    let cache = VaultCache::new();

    let first = cache
        .get_or_refresh(root)
        .await
        .expect("first vault index should build");
    let second = cache
        .get_or_refresh(&relative_root)
        .await
        .expect("canonical alias should resolve");

    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(first.generation, second.generation);
}

/// Multiple invalidations before a lookup coalesce into one index replacement.
#[tokio::test]
async fn dirty_state_coalesces_invalidation_bursts() {
    let directory = make_vault();
    let cache = VaultCache::new();
    let first = cache
        .get_or_refresh(directory.path())
        .await
        .expect("initial index should build");

    write_note(directory.path(), "Added.md");
    cache.invalidate(directory.path());
    cache.invalidate(directory.path());
    cache.invalidate(directory.path());
    let refreshed = cache
        .get_or_refresh(directory.path())
        .await
        .expect("dirty index should refresh");

    assert_eq!(refreshed.generation, first.generation + 1);
    assert!(indexed_paths(&refreshed).contains(&"Added.md".to_owned()));
}

/// Distinct canonical roots never share a cached generation.
#[tokio::test]
async fn separate_roots_have_isolated_generations() {
    let first_root = make_vault();
    let second_root = make_vault();
    write_note(second_root.path(), "Second.md");
    let cache = VaultCache::new();

    let first = cache
        .get_or_refresh(first_root.path())
        .await
        .expect("first root should index");
    let second = cache
        .get_or_refresh(second_root.path())
        .await
        .expect("second root should index");

    assert!(!Arc::ptr_eq(&first, &second));
    assert!(indexed_paths(&first).contains(&"Context.md".to_owned()));
    assert!(!indexed_paths(&first).contains(&"Second.md".to_owned()));
    assert!(indexed_paths(&second).contains(&"Second.md".to_owned()));
}

/// Refresh swaps complete snapshots while existing requests retain their generation.
#[tokio::test]
async fn refresh_atomically_replaces_the_index_generation() {
    let directory = make_vault();
    let cache = VaultCache::new();
    let selected = cache
        .get_or_refresh(directory.path())
        .await
        .expect("initial index should build");
    write_note(directory.path(), "Added.md");
    cache.invalidate(directory.path());

    let refreshed = cache
        .get_or_refresh(directory.path())
        .await
        .expect("replacement index should build");

    assert!(!Arc::ptr_eq(&selected, &refreshed));
    assert!(!indexed_paths(&selected).contains(&"Added.md".to_owned()));
    assert!(indexed_paths(&refreshed).contains(&"Added.md".to_owned()));
}

/// Refresh failures return an error rather than a previously cached generation.
#[tokio::test]
async fn failed_refresh_never_returns_the_stale_generation() {
    let directory = make_vault();
    let root = directory.path().to_path_buf();
    let cache = VaultCache::new();
    let selected = cache
        .get_or_refresh(&root)
        .await
        .expect("initial index should build");
    cache.invalidate(&root);
    cache.fail_next_refresh(&root);

    assert!(cache.get_or_refresh(&root).await.is_err());
    assert_eq!(cache.current_generation(&root), Some(selected.generation));
    assert!(cache.is_dirty(&root));
}

/// In-flight work may finish on its selected generation after invalidation.
#[tokio::test]
async fn in_flight_generation_remains_stable_after_invalidation() {
    let directory = make_vault();
    let root: PathBuf = directory.path().to_path_buf();
    let cache = VaultCache::new();
    let selected = cache
        .get_or_refresh(&root)
        .await
        .expect("initial index should build");
    let selected_for_request = Arc::clone(&selected);
    let (started_sender, started_receiver) = oneshot::channel();
    let (resume_sender, resume_receiver) = oneshot::channel();
    let in_flight = tokio::spawn(async move {
        started_sender
            .send(())
            .expect("request start should be observed");
        resume_receiver
            .await
            .expect("in-flight request should resume");
        indexed_paths(&selected_for_request)
    });
    started_receiver
        .await
        .expect("request should select its generation before invalidation");
    write_note(&root, "Added.md");
    cache.invalidate(&root);

    resume_sender
        .send(())
        .expect("in-flight request should be released");
    assert!(!in_flight
        .await
        .expect("in-flight request should complete")
        .contains(&"Added.md".to_owned()));
    let next = cache
        .get_or_refresh(&root)
        .await
        .expect("next request should refresh before selection");
    assert!(indexed_paths(&next).contains(&"Added.md".to_owned()));
    assert!(next.generation > selected.generation);
}

/// A refreshed generation resolves headings added to an existing note.
#[tokio::test]
async fn changed_note_contents_are_visible_after_refresh() {
    let directory = make_vault();
    let target_path = directory.path().join("Target.md");
    fs::write(&target_path, "# Original\n").expect("target note should be written");
    let context_path = directory.path().join("Context.md");
    let cache = VaultCache::new();
    let _initial = cache
        .get_or_refresh(directory.path())
        .await
        .expect("initial index should build");

    fs::write(&target_path, "# Updated\n\nUpdated body.\n").expect("target note should be changed");
    cache.invalidate(directory.path());
    let refreshed = cache
        .get_or_refresh(directory.path())
        .await
        .expect("updated index should build");
    let result = resolve_with_vault(
        "[[Target#Updated]]",
        context_path.to_string_lossy().as_ref(),
        &refreshed.vault,
        false,
    );

    assert_eq!(result.status, Status::Resolved);
}
