use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use obsidian_link_resolver::output::{ResolutionTarget, Status};
use obsidian_link_resolver::resolve_with_vault;
use obsidian_link_resolver::vault::{enumerate_vault, Vault, VaultSource};

#[path = "../tests/fixtures/bench_vault.rs"]
mod bench_vault;

fn warm_run_resolve(c: &mut Criterion) {
    let (vault_root, context_path, link) =
        bench_vault::ensure_bench_vault().expect("benchmark vault should be generated");
    let entries = enumerate_vault(vault_root.to_string_lossy().as_ref())
        .expect("benchmark vault should enumerate successfully");
    let vault = Vault {
        root: vault_root.to_string_lossy().into_owned(),
        source: VaultSource::Explicit,
        entries,
    };

    let warmed_result =
        resolve_with_vault(link, context_path.to_string_lossy().as_ref(), &vault, false);
    assert!(
        matches!(warmed_result.status, Status::Resolved),
        "benchmark warm-up should resolve successfully: {:?}",
        warmed_result
    );

    c.bench_function("warm_run_resolve_heading", |bench| {
        bench.iter(|| {
            let result: ResolutionTarget = resolve_with_vault(
                black_box(link),
                black_box(context_path.to_string_lossy().as_ref()),
                black_box(&vault),
                false,
            );
            black_box(result)
        });
    });
}

criterion_group!(benches, warm_run_resolve);
criterion_main!(benches);
