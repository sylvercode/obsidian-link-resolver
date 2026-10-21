//! Contract tests for MCP release targets, gates, and published assets.

const RELEASE_WORKFLOW: &str = include_str!("../../../../.github/workflows/release.yml");

/// Supported Rust targets included in each MCP release.
const RELEASE_TARGETS: [&str; 5] = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
];

/// A moved tag must not change the source commit used by a release run.
#[test]
fn release_checkouts_use_the_triggering_commit() {
    assert_release_checkouts_use_the_triggering_commit(RELEASE_WORKFLOW);
}

fn assert_release_checkouts_use_the_triggering_commit(workflow: &str) {
    let workflow = workflow.replace("\r\n", "\n");
    let checkouts: Vec<_> = workflow
        .split("- uses: actions/checkout@")
        .skip(1)
        .collect();
    assert_eq!(checkouts.len(), 3, "all build jobs must check out source");
    for checkout in checkouts {
        let step = checkout.split("\n      - ").next().unwrap();
        assert!(
            step.contains("\n        with:\n          ref: ${{ github.sha }}"),
            "each release checkout must use the immutable triggering commit"
        );
    }
}

/// Release validation must complete all feature gates before staging assets.
#[test]
fn release_gates_precede_component_asset_staging() {
    let staging = RELEASE_WORKFLOW
        .find("Stage release assets")
        .expect("release workflow should stage component assets");
    for gate in [
        "Check published MCP contract compatibility",
        "cargo test --workspace",
        "cargo bench -p obsidian-link-resolver-mcp --bench mcp_warm",
        "cargo bench -p obsidian-link-resolver-mcp --bench mcp_cold_start",
    ] {
        let gate_position = RELEASE_WORKFLOW
            .find(gate)
            .unwrap_or_else(|| panic!("release workflow is missing gate: {gate}"));
        assert!(
            gate_position < staging,
            "release gate must precede asset staging: {gate}"
        );
    }
}

/// The release maps every supported target to distinct CLI, MCP, and FFI assets.
#[test]
fn release_maps_all_targets_to_distinct_component_assets() {
    assert_release_maps_all_targets_to_distinct_component_assets(RELEASE_WORKFLOW);
}

fn assert_release_maps_all_targets_to_distinct_component_assets(workflow: &str) {
    let workflow = workflow.replace("\r\n", "\n");
    let expected_mappings = [
        (
            RELEASE_TARGETS[0],
            r#"cp target/release/obsidian-link-resolver "dist/obsidian-link-resolver-${RELEASE_TAG}-linux-x86_64""#,
            r#"cp target/release/obsidian-link-resolver-mcp "dist/obsidian-link-resolver-mcp-${RELEASE_TAG}-linux-x86_64""#,
            r#"cp target/release/libobsidian_link_resolver.so "dist/libobsidian_link_resolver-${RELEASE_TAG}-linux-x86_64.so""#,
        ),
        (
            RELEASE_TARGETS[1],
            r#"cp target/aarch64-unknown-linux-gnu/release/obsidian-link-resolver "dist/obsidian-link-resolver-${RELEASE_TAG}-linux-aarch64""#,
            r#"cp target/aarch64-unknown-linux-gnu/release/obsidian-link-resolver-mcp "dist/obsidian-link-resolver-mcp-${RELEASE_TAG}-linux-aarch64""#,
            r#"cp target/aarch64-unknown-linux-gnu/release/libobsidian_link_resolver.so "dist/libobsidian_link_resolver-${RELEASE_TAG}-linux-aarch64.so""#,
        ),
        (
            RELEASE_TARGETS[2],
            r#"cp "target/${TARGET}/release/obsidian-link-resolver" "dist/obsidian-link-resolver-${RELEASE_TAG}-darwin-${ARCH}""#,
            r#"cp "target/${TARGET}/release/obsidian-link-resolver-mcp" "dist/obsidian-link-resolver-mcp-${RELEASE_TAG}-darwin-${ARCH}""#,
            r#"cp "target/${TARGET}/release/libobsidian_link_resolver.dylib" "dist/libobsidian_link_resolver-${RELEASE_TAG}-darwin-${ARCH}.dylib""#,
        ),
        (
            RELEASE_TARGETS[3],
            r#"cp "target/${TARGET}/release/obsidian-link-resolver" "dist/obsidian-link-resolver-${RELEASE_TAG}-darwin-${ARCH}""#,
            r#"cp "target/${TARGET}/release/obsidian-link-resolver-mcp" "dist/obsidian-link-resolver-mcp-${RELEASE_TAG}-darwin-${ARCH}""#,
            r#"cp "target/${TARGET}/release/libobsidian_link_resolver.dylib" "dist/libobsidian_link_resolver-${RELEASE_TAG}-darwin-${ARCH}.dylib""#,
        ),
        (
            RELEASE_TARGETS[4],
            r#"cp target/x86_64-pc-windows-msvc/release/obsidian-link-resolver.exe "dist/obsidian-link-resolver-${RELEASE_TAG}-windows-x86_64.exe""#,
            r#"cp target/x86_64-pc-windows-msvc/release/obsidian-link-resolver-mcp.exe "dist/obsidian-link-resolver-mcp-${RELEASE_TAG}-windows-x86_64.exe""#,
            r#"cp target/x86_64-pc-windows-msvc/release/obsidian_link_resolver.dll "dist/libobsidian_link_resolver-${RELEASE_TAG}-windows-x86_64.dll""#,
        ),
    ];

    for (target, cli_mapping, mcp_mapping, ffi_mapping) in expected_mappings {
        assert!(
            workflow.contains(target),
            "release workflow is missing target {target}"
        );
        for mapping in [cli_mapping, mcp_mapping, ffi_mapping] {
            assert!(
                workflow.contains(mapping),
                "release workflow is missing mapping for {target}: {mapping}"
            );
        }
    }

    for (target, runner, arch) in [
        (RELEASE_TARGETS[2], "macos-15-intel", "x86_64"),
        (RELEASE_TARGETS[3], "macos-15", "aarch64"),
    ] {
        let matrix_entry =
            format!("target: {target}\n            runner: {runner}\n            arch: {arch}");
        assert!(
            workflow.contains(&matrix_entry),
            "release workflow has an invalid native runner mapping for {target}"
        );
    }

    for required in [
        "cargo build --workspace --release --target ${{ matrix.target }}",
        "actions/download-artifact@v4",
        "merge-multiple: true",
        "needs: [release, build-macos, build-windows]",
    ] {
        assert!(
            workflow.contains(required),
            "release workflow is missing native asset pipeline requirement: {required}"
        );
    }

    for artifact in [
        "mcp-contract.json",
        "obsidian_link_resolver-${RELEASE_TAG}.h",
    ] {
        assert!(
            workflow.contains(artifact),
            "release workflow is missing published artifact {artifact}"
        );
    }
}

/// Windows builds use the native MSVC toolchain without a MinGW dependency.
#[test]
fn windows_workflows_use_native_msvc() {
    let ci = include_str!("../../../../.github/workflows/ci.yml").replace("\r\n", "\n");
    let release = RELEASE_WORKFLOW.replace("\r\n", "\n");
    assert!(ci.contains("target: x86_64-pc-windows-msvc\n            runner: windows-2022"));
    let windows_job = release
        .split("\n  build-windows:\n")
        .nth(1)
        .expect("release workflow must have a native Windows build job")
        .split("\n  build-macos:\n")
        .next()
        .unwrap();
    for required in [
        "runs-on: windows-2022",
        "targets: x86_64-pc-windows-msvc",
        "cargo build --workspace --release --target x86_64-pc-windows-msvc",
        "name: release-assets-windows",
    ] {
        assert!(windows_job.contains(required), "missing {required}");
    }
    for workflow in [&ci, &release] {
        for obsolete in ["windows-gnu", "WINDOWS_GNU_LINKER", "mingw", "msys2"] {
            assert!(!workflow.contains(obsolete), "obsolete setup: {obsolete}");
        }
    }
}

/// Artifact uploads in CI and releases must use the supported v4 action.
#[test]
fn artifact_uploads_use_v4() {
    for workflow in [
        include_str!("../../../../.github/workflows/ci.yml"),
        RELEASE_WORKFLOW,
    ] {
        let uploads: Vec<_> = workflow
            .lines()
            .filter_map(|line| line.trim().split_once("uses: actions/upload-artifact@"))
            .map(|(_, version)| version)
            .collect();
        assert!(!uploads.is_empty(), "workflow must retain its artifacts");
        assert!(uploads.iter().all(|version| *version == "v4"));
    }
}

/// Workflow contracts must hold regardless of the checkout's line endings.
#[test]
fn release_contract_checks_accept_lf_and_crlf() {
    let lf = RELEASE_WORKFLOW.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    for workflow in [&lf, &crlf] {
        assert_release_checkouts_use_the_triggering_commit(workflow);
        assert_release_maps_all_targets_to_distinct_component_assets(workflow);
    }
}
