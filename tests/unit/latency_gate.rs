use std::time::{Duration, Instant};

use obsidian_link_resolver::output::Status;
use obsidian_link_resolver::resolve_with_vault;
use obsidian_link_resolver::vault::{enumerate_vault, Vault, VaultSource};

#[path = "../fixtures/bench_vault.rs"]
mod bench_vault;

/// Verifies warm-run median latency stays within the documented 100ms performance gate.
#[test]
fn warm_run_p50_latency_gate() {
    let (vault_root, context_path, link) =
        bench_vault::ensure_bench_vault().expect("benchmark vault should be generated");
    let entries = enumerate_vault(vault_root.to_string_lossy().as_ref())
        .expect("benchmark vault should enumerate successfully");
    let vault = Vault {
        root: vault_root.to_string_lossy().into_owned(),
        source: VaultSource::Explicit,
        entries,
    };

    let context_path = context_path.to_string_lossy().into_owned();
    let mut samples = Vec::with_capacity(31);

    for _ in 0..31 {
        let started = Instant::now();
        let result = resolve_with_vault(link, &context_path, &vault, false);
        let elapsed = started.elapsed();

        assert!(
            matches!(result.status, Status::Resolved),
            "benchmark warm-up should resolve successfully: {:?}",
            result
        );
        samples.push(elapsed);
    }

    samples.sort();
    let p50 = samples[samples.len() / 2];
    assert!(
        p50 <= Duration::from_millis(100),
        "warm-run p50 latency gate failed: measured {p50:?}, limit is 100ms"
    );
}
