//! Contract tests for MCP release targets, gates, and published assets.

const RELEASE_WORKFLOW: &str = include_str!("../../../../.github/workflows/release.yml");

/// Supported Rust targets included in each MCP release.
const RELEASE_TARGETS: [&str; 5] = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-gnu",
];

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
            r#"cp target/x86_64-pc-windows-gnu/release/obsidian-link-resolver.exe "dist/obsidian-link-resolver-${RELEASE_TAG}-windows-x86_64.exe""#,
            r#"cp target/x86_64-pc-windows-gnu/release/obsidian-link-resolver-mcp.exe "dist/obsidian-link-resolver-mcp-${RELEASE_TAG}-windows-x86_64.exe""#,
            r#"cp target/x86_64-pc-windows-gnu/release/obsidian_link_resolver.dll "dist/libobsidian_link_resolver-${RELEASE_TAG}-windows-x86_64.dll""#,
        ),
    ];

    for (target, cli_mapping, mcp_mapping, ffi_mapping) in expected_mappings {
        assert!(
            RELEASE_WORKFLOW.contains(target),
            "release workflow is missing target {target}"
        );
        for mapping in [cli_mapping, mcp_mapping, ffi_mapping] {
            assert!(
                RELEASE_WORKFLOW.contains(mapping),
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
            RELEASE_WORKFLOW.contains(&matrix_entry),
            "release workflow has an invalid native runner mapping for {target}"
        );
    }

    for required in [
        "cargo build --workspace --release --target ${{ matrix.target }}",
        "actions/download-artifact@v4",
        "merge-multiple: true",
        "needs: [release, build-macos]",
    ] {
        assert!(
            RELEASE_WORKFLOW.contains(required),
            "release workflow is missing native asset pipeline requirement: {required}"
        );
    }

    for artifact in [
        "mcp-contract.json",
        "obsidian_link_resolver-${RELEASE_TAG}.h",
    ] {
        assert!(
            RELEASE_WORKFLOW.contains(artifact),
            "release workflow is missing published artifact {artifact}"
        );
    }
}
