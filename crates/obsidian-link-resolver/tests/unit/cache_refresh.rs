//! Watcher and periodic-refresh coverage for the vault cache.

use super::VaultCache;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tempfile::TempDir;

/// Create a small temporary vault suitable for cache refresh tests.
fn make_vault() -> TempDir {
    let directory = tempfile::tempdir().expect("temporary vault should be created");
    fs::create_dir(directory.path().join(".obsidian"))
        .expect("vault metadata directory should be created");
    write_note(directory.path(), "Context.md");
    directory
}

/// Write a note at a vault-relative path.
fn write_note(root: &Path, relative_path: &str) {
    let note = root.join(relative_path);
    fs::create_dir_all(note.parent().expect("note should have a parent"))
        .expect("note directory should be created");
    fs::write(note, "# Note\n").expect("note should be written");
}

/// A filesystem change marks an active cached root dirty before another request.
#[tokio::test]
async fn watcher_invalidates_before_the_next_generation_is_selected() {
    let directory = make_vault();
    let cache = VaultCache::new();
    let first = cache
        .get_or_refresh(directory.path())
        .await
        .expect("initial index should build");
    assert!(cache.watcher_available(directory.path()));
    write_note(directory.path(), "Watcher Added.md");

    tokio::time::timeout(
        Duration::from_secs(5),
        cache.wait_until_dirty(directory.path()),
    )
    .await
    .expect("watcher should report the filesystem change")
    .expect("cached root should remain present");

    let refreshed = cache
        .get_or_refresh(directory.path())
        .await
        .expect("next selection should refresh the dirty index");
    assert!(refreshed.generation > first.generation);
    assert!(refreshed
        .vault
        .entries
        .iter()
        .any(|entry| entry.rel_path == "Watcher Added.md"));
}

/// Periodic rescans work without a watcher or incoming requests.
#[tokio::test(start_paused = true)]
async fn periodic_refresh_recovers_missed_events_with_watcher_unavailable() {
    let directory = make_vault();
    let cache = VaultCache::without_watcher();
    let first = cache
        .get_or_refresh(directory.path())
        .await
        .expect("initial index should build");
    assert!(!cache.watcher_available(directory.path()));
    tokio::task::yield_now().await;
    write_note(directory.path(), "Periodic Added.md");

    tokio::time::advance(Duration::from_secs(60)).await;
    let current_generation = tokio::time::timeout(
        Duration::from_secs(5),
        cache.wait_for_generation_after(directory.path(), first.generation),
    )
    .await
    .expect("background refresh should finish within the fallback window")
    .expect("cached root should remain present");
    assert!(current_generation > first.generation);
    let refreshed = cache
        .get_or_refresh(directory.path())
        .await
        .expect("periodic generation should be available without a request-triggered walk");
    assert!(refreshed
        .vault
        .entries
        .iter()
        .any(|entry| entry.rel_path == "Periodic Added.md"));
}
