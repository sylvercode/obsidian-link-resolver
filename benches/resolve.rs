use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, Criterion};
use obsidian_link_resolver::output::{ResolutionTarget, Status};
use obsidian_link_resolver::resolve_with_vault;
use obsidian_link_resolver::vault::{enumerate_vault, Vault, VaultSource};

#[path = "../tests/fixtures/bench_vault.rs"]
mod bench_vault;

const WARM_RUN_P50_LIMIT: Duration = Duration::from_millis(100);

fn benchmark_vault() -> (String, &'static str, Vault) {
    let (vault_root, context_path, link) =
        bench_vault::ensure_bench_vault().expect("benchmark vault should be generated");
    let entries = enumerate_vault(vault_root.to_string_lossy().as_ref())
        .expect("benchmark vault should enumerate successfully");
    let vault = Vault {
        root: vault_root.to_string_lossy().into_owned(),
        source: VaultSource::Explicit,
        entries,
    };

    (context_path.to_string_lossy().into_owned(), link, vault)
}

fn warm_run_p50() -> Duration {
    let (context_path, link, vault) = benchmark_vault();
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
    let p50_index = samples.len() / 2;
    samples[p50_index]
}

#[test]
fn warm_run_latency_gate() {
    let p50 = warm_run_p50();
    assert!(
        p50 <= WARM_RUN_P50_LIMIT,
        "warm-run p50 latency gate failed: measured {p50:?}, limit is {WARM_RUN_P50_LIMIT:?}"
    );
}

fn warm_run_resolve(c: &mut Criterion) {
    let (context_path, link, vault) = benchmark_vault();
    let warmed_result = resolve_with_vault(link, &context_path, &vault, false);
    assert!(
        matches!(warmed_result.status, Status::Resolved),
        "benchmark warm-up should resolve successfully: {:?}",
        warmed_result
    );

    let p50 = warm_run_p50();
    assert!(
        p50 <= WARM_RUN_P50_LIMIT,
        "warm-run p50 latency gate failed: measured {p50:?}, limit is {WARM_RUN_P50_LIMIT:?}"
    );

    c.bench_function("warm_run_resolve_heading", |bench| {
        bench.iter(|| {
            let result: ResolutionTarget = resolve_with_vault(
                black_box(link),
                black_box(context_path.as_str()),
                black_box(&vault),
                false,
            );
            black_box(result)
        });
    });
}

criterion_group!(benches, warm_run_resolve);
criterion_main!(benches);
