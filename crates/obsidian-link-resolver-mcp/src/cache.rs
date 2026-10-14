//! Process-scoped vault index cache with watcher and periodic invalidation.

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use obsidian_link_resolver::vault::{enumerate_vault, Vault, VaultSource};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;
use tokio::sync::{Mutex as AsyncMutex, Notify};
use tokio::time::Instant;

/// Maximum time an index may remain without a full filesystem rescan.
const FULL_RESCAN_INTERVAL: Duration = Duration::from_secs(60);

/// Maximum scan retries when watcher invalidations race with enumeration.
const MAX_REFRESH_ATTEMPTS: usize = 3;

/// One immutable vault index selected by a request or installed by a refresh.
#[derive(Debug)]
pub(crate) struct VaultGeneration {
    /// Monotonically increasing index generation for one canonical root.
    pub(crate) generation: u64,
    /// Complete resolver snapshot used by requests holding this generation.
    pub(crate) vault: Vault,
}

/// An in-progress cache entry and its watcher state for one canonical vault root.
struct CacheEntry {
    /// Canonical absolute path used as the cache identity.
    root: PathBuf,
    /// Mutable refresh state protected from watcher callbacks and requests.
    state: Mutex<EntryState>,
    /// Serializes scans so only one replacement is constructed at a time.
    refresh_lock: AsyncMutex<()>,
    /// Wakes deterministic tests when a background generation has been installed.
    refresh_completed: Notify,
    /// Retained watcher handle; dropping it disables filesystem notifications.
    watcher: Mutex<Option<RecommendedWatcher>>,
}

/// Mutable metadata for the current immutable vault generation.
struct EntryState {
    /// Current complete index, retained for already-started requests after invalidation.
    current: Option<Arc<VaultGeneration>>,
    /// Whether a watcher event or elapsed scan deadline requires rebuilding the index.
    dirty: bool,
    /// Sequence number incremented on every invalidation to detect changes during a scan.
    invalidation_sequence: u64,
    /// Monotonic completion time of the most recent successful full scan.
    last_full_scan: Option<Instant>,
    /// Test-only failure switch for exercising fail-closed refresh behavior.
    #[cfg(test)]
    fail_next_refresh: bool,
}

/// Per-process vault cache keyed by canonical absolute root paths.
pub(crate) struct VaultCache {
    /// Entries owned for the lifetime of the MCP server process.
    entries: Mutex<HashMap<PathBuf, Arc<CacheEntry>>>,
    /// Whether new entries should attempt recursive filesystem watching.
    watch_enabled: bool,
}

impl std::fmt::Debug for VaultCache {
    /// Format the cache without exposing vault paths or internal watcher details.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("VaultCache").finish_non_exhaustive()
    }
}

impl VaultCache {
    /// Create a cache that uses filesystem watchers when the platform permits.
    pub(crate) fn new() -> Self {
        Self::with_watcher(true)
    }

    /// Create a cache with watcher behavior selected by the caller.
    ///
    /// Watchers may be disabled by tests to verify the periodic-rescan fallback.
    fn with_watcher(watch_enabled: bool) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            watch_enabled,
        }
    }

    /// Create a cache that exercises the periodic fallback without filesystem events.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn without_watcher() -> Self {
        Self::with_watcher(false)
    }

    /// Select the current generation for a canonicalized root, refreshing when necessary.
    ///
    /// # Parameters
    ///
    /// - `root`: Existing directory identifying the vault to index.
    ///
    /// # Returns
    ///
    /// A stable generation for the caller, or a refresh error without stale data.
    pub(crate) async fn get_or_refresh(&self, root: &Path) -> Result<Arc<VaultGeneration>, String> {
        let canonical_root = fs::canonicalize(root)
            .map_err(|error| format!("failed to canonicalize vault root: {error}"))?;
        if !canonical_root.is_dir() {
            return Err("vault root is not a directory".to_owned());
        }
        let entry = self.entry(canonical_root);

        loop {
            if entry.needs_refresh() {
                entry.refresh().await?;
            }
            if let Some(generation) = entry.generation_if_fresh() {
                return Ok(generation);
            }
        }
    }

    /// Mark a cached root dirty after a deterministic invalidation or external event.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn invalidate(&self, root: &Path) {
        if let Some(entry) = self.find_entry(root) {
            entry.invalidate();
        }
    }

    /// Make the next refresh for a cached root fail without changing its filesystem.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn fail_next_refresh(&self, root: &Path) {
        if let Some(entry) = self.find_entry(root) {
            entry.state().fail_next_refresh = true;
        }
    }

    /// Report whether a root has an invalidation waiting to be refreshed.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn is_dirty(&self, root: &Path) -> bool {
        self.find_entry(root)
            .is_some_and(|entry| entry.state().dirty)
    }

    /// Report whether a recursive filesystem watcher is active for a root.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn watcher_available(&self, root: &Path) -> bool {
        self.find_entry(root).is_some_and(|entry| {
            entry
                .watcher
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_some()
        })
    }

    /// Return the last successfully installed generation without triggering a refresh.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn current_generation(&self, root: &Path) -> Option<u64> {
        self.find_entry(root).and_then(|entry| {
            entry
                .state()
                .current
                .as_ref()
                .map(|generation| generation.generation)
        })
    }

    /// Wait until a background scan installs a generation newer than the supplied one.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) async fn wait_for_generation_after(
        &self,
        root: &Path,
        generation: u64,
    ) -> Option<u64> {
        let entry = self.find_entry(root)?;
        loop {
            let notified = entry.refresh_completed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(current) = self
                .current_generation(root)
                .filter(|value| *value > generation)
            {
                return Some(current);
            }
            notified.await;
        }
    }

    /// Find or create one entry and start its watcher and periodic scanner.
    fn entry(&self, root: PathBuf) -> Arc<CacheEntry> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = entries.get(&root) {
            return Arc::clone(entry);
        }

        let entry = Arc::new(CacheEntry {
            root,
            state: Mutex::new(EntryState {
                current: None,
                dirty: true,
                invalidation_sequence: 0,
                last_full_scan: None,
                #[cfg(test)]
                fail_next_refresh: false,
            }),
            refresh_lock: AsyncMutex::new(()),
            refresh_completed: Notify::new(),
            watcher: Mutex::new(None),
        });
        if self.watch_enabled {
            entry.start_watcher();
        }
        Self::start_periodic_refresh(&entry);
        entries.insert(entry.root.clone(), Arc::clone(&entry));
        entry
    }

    /// Find a cache entry using a canonical root or its original absolute path.
    #[cfg(test)]
    #[allow(dead_code)]
    fn find_entry(&self, root: &Path) -> Option<Arc<CacheEntry>> {
        let key = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
            .map(Arc::clone)
    }

    /// Run a full scan at a fixed interval independently of incoming requests.
    fn start_periodic_refresh(entry: &Arc<CacheEntry>) {
        let weak_entry = Arc::downgrade(entry);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(FULL_RESCAN_INTERVAL).await;
                let Some(entry) = weak_entry.upgrade() else {
                    break;
                };
                let _ = entry.refresh().await;
            }
        });
    }
}

impl CacheEntry {
    /// Lock entry state while recovering safely from a prior poisoned lock.
    fn state(&self) -> MutexGuard<'_, EntryState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Record an invalidation without replacing the current immutable generation.
    fn invalidate(&self) {
        let mut state = self.state();
        state.invalidation_sequence = state.invalidation_sequence.wrapping_add(1);
        state.dirty = true;
    }

    /// Check whether the current generation is absent, dirty, or past its scan deadline.
    fn needs_refresh(&self) -> bool {
        let mut state = self.state();
        if state
            .last_full_scan
            .is_some_and(|last_scan| last_scan.elapsed() >= FULL_RESCAN_INTERVAL)
        {
            state.dirty = true;
        }
        state.dirty || state.current.is_none()
    }

    /// Return a cloned generation only while the state remains clean and current.
    fn generation_if_fresh(&self) -> Option<Arc<VaultGeneration>> {
        let mut state = self.state();
        if state
            .last_full_scan
            .is_some_and(|last_scan| last_scan.elapsed() >= FULL_RESCAN_INTERVAL)
        {
            state.dirty = true;
        }
        if state.dirty {
            None
        } else {
            state.current.as_ref().map(Arc::clone)
        }
    }

    /// Build and atomically install a complete index if no event raced with the scan.
    async fn refresh(&self) -> Result<(), String> {
        let _refresh_guard = self.refresh_lock.lock().await;
        for _ in 0..MAX_REFRESH_ATTEMPTS {
            #[cfg(test)]
            {
                let mut state = self.state();
                if state.fail_next_refresh {
                    state.fail_next_refresh = false;
                    return Err("injected vault index refresh failure".to_owned());
                }
            }
            let observed_sequence = self.state().invalidation_sequence;
            let root = self.root.clone();
            let root_for_walk = root.clone();
            let entries = tokio::task::spawn_blocking(move || {
                enumerate_vault(root_for_walk.to_string_lossy().as_ref())
            })
            .await
            .map_err(|_| "vault index refresh task failed".to_owned())??;
            let vault = Vault {
                root: root.to_string_lossy().into_owned(),
                source: VaultSource::Explicit,
                entries,
            };

            let mut state = self.state();
            if state.invalidation_sequence != observed_sequence {
                state.dirty = true;
                continue;
            }
            let generation = state
                .current
                .as_ref()
                .map_or(1, |current| current.generation + 1);
            state.current = Some(Arc::new(VaultGeneration { generation, vault }));
            state.last_full_scan = Some(Instant::now());
            state.dirty = false;
            drop(state);
            self.refresh_completed.notify_waiters();
            return Ok(());
        }

        self.state().dirty = true;
        Err("vault changed repeatedly during index refresh".to_owned())
    }

    /// Register a recursive watcher whose callback only marks this entry dirty.
    fn start_watcher(self: &Arc<Self>) {
        let weak_entry: Weak<Self> = Arc::downgrade(self);
        let watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if event.is_ok_and(|event| !matches!(event.kind, EventKind::Access(_))) {
                if let Some(entry) = weak_entry.upgrade() {
                    entry.invalidate();
                }
            }
        })
        .and_then(|mut watcher| {
            watcher.watch(&self.root, RecursiveMode::Recursive)?;
            Ok(watcher)
        });

        if let Ok(watcher) = watcher {
            *self
                .watcher
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(watcher);
        }
    }
}
