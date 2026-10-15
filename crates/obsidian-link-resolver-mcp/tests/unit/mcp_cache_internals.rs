//! Unit tests for vault cache internals, loaded as a child of the cache module.

use super::VaultCache;
use std::future::{poll_fn, Future};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::task::Poll;

#[tokio::test]
/// A poisoned map retains existing entries and permits new roots after recovery.
async fn poisoned_map_preserves_entries() {
    let directory = tempfile::tempdir().expect("temporary vault should be created");
    let root = std::fs::canonicalize(directory.path()).expect("root should be canonicalized");
    let cache = VaultCache::without_watcher();
    let first = cache
        .get_or_refresh(&root)
        .await
        .expect("initial index should build");
    let entry = cache.entry(root.clone());

    assert!(catch_unwind(AssertUnwindSafe(|| {
        let _entries = cache
            .entries
            .lock()
            .expect("map should not be poisoned yet");
        panic!("poison the cache map");
    }))
    .is_err());
    assert!(cache.entries.is_poisoned());
    assert!(Arc::ptr_eq(&entry, &cache.entry(root.clone())));
    assert!(!cache.entries.is_poisoned());
    let reused = cache
        .get_or_refresh(&root)
        .await
        .expect("existing snapshot should remain usable");
    assert!(Arc::ptr_eq(&first, &reused));

    let other = tempfile::tempdir().expect("another vault should be created");
    cache
        .get_or_refresh(other.path())
        .await
        .expect("new roots should remain usable");
    assert_eq!(cache.entries().len(), 2);
}

#[tokio::test]
/// Poisoned state rejects cached snapshots until a full scan succeeds.
async fn poisoned_state_requires_successful_refresh() {
    let directory = tempfile::tempdir().expect("temporary vault should be created");
    let cache = VaultCache::without_watcher();
    let first = cache
        .get_or_refresh(directory.path())
        .await
        .expect("initial index should build");
    let entry = cache
        .find_entry(directory.path())
        .expect("entry should exist");
    let sequence = entry.state().invalidation_sequence;
    std::fs::write(directory.path().join("Added.md"), "# Added\n").expect("note should be written");

    assert!(catch_unwind(AssertUnwindSafe(|| {
        let mut state = entry
            .state
            .lock()
            .expect("state should not be poisoned yet");
        state.fail_next_refresh = true;
        panic!("poison the entry state");
    }))
    .is_err());
    assert!(entry.state.is_poisoned());
    assert!(entry.generation_if_fresh().is_none());
    assert!(!entry.state.is_poisoned());
    {
        let state = entry.state();
        assert!(state.dirty);
        assert!(state.last_full_scan.is_none());
        assert_eq!(state.invalidation_sequence, sequence.wrapping_add(1));
        assert!(Arc::ptr_eq(state.current.as_ref().unwrap(), &first));
    }

    let error = cache
        .get_or_refresh(directory.path())
        .await
        .expect_err("failed recovery must not return the old snapshot");
    assert_eq!(error, "injected vault index refresh failure");
    assert!(entry.generation_if_fresh().is_none());
    let recovered = cache
        .get_or_refresh(directory.path())
        .await
        .expect("a subsequent scan should recover");
    assert_eq!(recovered.generation, first.generation + 1);
    assert!(recovered
        .vault
        .entries
        .iter()
        .any(|entry| entry.rel_path == "Added.md"));
    assert!(first.vault.entries.is_empty());
    assert!(!entry.state().dirty);
    assert!(entry.state().last_full_scan.is_some());
}

#[test]
/// A scan started before poisoning cannot install its snapshot after recovery.
fn poisoned_state_rejects_in_flight_scan() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .expect("test runtime should build");
    runtime.block_on(async {
        let directory = tempfile::tempdir().expect("temporary vault should be created");
        let cache = VaultCache::without_watcher();
        let first = cache
            .get_or_refresh(directory.path())
            .await
            .expect("initial index should build");
        let entry = cache
            .find_entry(directory.path())
            .expect("entry should exist");
        let (release, wait_for_release) = std::sync::mpsc::channel();
        let (started, wait_for_start) = std::sync::mpsc::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            started
                .send(())
                .expect("test should receive the start signal");
            let _ = wait_for_release.recv();
        });
        wait_for_start
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("blocking worker should start");

        let refresh = entry.refresh(true);
        tokio::pin!(refresh);
        poll_fn(|context| {
            assert!(refresh.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        assert!(catch_unwind(AssertUnwindSafe(|| {
            let mut state = entry
                .state
                .lock()
                .expect("state should not be poisoned yet");
            state.fail_next_refresh = true;
            panic!("poison state during an in-flight scan");
        }))
        .is_err());
        assert!(entry.state.is_poisoned());
        release
            .send(())
            .expect("blocking worker should be released");
        blocker.await.expect("blocking worker should finish");

        assert_eq!(
            refresh.await.expect_err("invalidated scan must retry"),
            "injected vault index refresh failure"
        );
        assert!(!entry.state.is_poisoned());
        assert!(entry.generation_if_fresh().is_none());
        assert_eq!(
            cache.current_generation(directory.path()),
            Some(first.generation)
        );
        let recovered = cache
            .get_or_refresh(directory.path())
            .await
            .expect("a subsequent request should rebuild");
        assert_eq!(recovered.generation, first.generation + 1);
    });
}

#[tokio::test]
/// Concurrent cold and dirty requests share one refresh generation.
async fn concurrent_callers_coalesce_cold_and_dirty_refreshes() {
    let directory = tempfile::tempdir().expect("temporary vault should be created");
    std::fs::write(directory.path().join("Note.md"), "# Note\n").expect("note should be written");
    let root = std::fs::canonicalize(directory.path()).expect("root should be canonicalized");
    let cache = VaultCache::without_watcher();
    let entry = cache.entry(root.clone());

    for expected_generation in 1..=2 {
        let refresh_guard = entry.refresh_lock.lock().await;
        let first = cache.get_or_refresh(&root);
        let second = cache.get_or_refresh(&root);
        let third = cache.get_or_refresh(&root);
        tokio::pin!(first, second, third);
        poll_fn(|context| {
            assert!(first.as_mut().poll(context).is_pending());
            assert!(second.as_mut().poll(context).is_pending());
            assert!(third.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        drop(refresh_guard);

        let (first, second, third) = tokio::join!(first, second, third);
        let first = first.expect("first request should select a generation");
        let second = second.expect("second request should select a generation");
        let third = third.expect("third request should select a generation");
        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(&first, &third));
        assert_eq!(first.generation, expected_generation);
        assert_eq!(cache.current_generation(&root), Some(expected_generation));
        entry.invalidate();
    }
}

#[tokio::test]
/// A clean cache entry reuses its snapshot until an explicit invalidation.
async fn forced_refresh_rescans_a_fresh_generation() {
    let directory = tempfile::tempdir().expect("temporary vault should be created");
    let cache = VaultCache::without_watcher();
    let first = cache
        .get_or_refresh(directory.path())
        .await
        .expect("initial index should build");
    let entry = cache
        .find_entry(directory.path())
        .expect("entry should exist");
    std::fs::write(directory.path().join("Added.md"), "# Added\n").expect("note should be written");

    entry
        .refresh(false)
        .await
        .expect("fresh index should be reused");
    assert_eq!(
        cache.current_generation(directory.path()),
        Some(first.generation)
    );
    entry
        .refresh(true)
        .await
        .expect("forced scan should succeed");
    let refreshed = entry
        .generation_if_fresh()
        .expect("index should remain fresh");
    assert_eq!(refreshed.generation, first.generation + 1);
    assert!(refreshed
        .vault
        .entries
        .iter()
        .any(|entry| entry.rel_path == "Added.md"));
}
