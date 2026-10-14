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
    for target in RELEASE_TARGETS {
        assert!(
            RELEASE_WORKFLOW.contains(target),
            "release workflow is missing target {target}"
        );
    }

    for asset_family in [
        "obsidian-link-resolver-${RELEASE_TAG}",
        "obsidian-link-resolver-mcp-${RELEASE_TAG}",
        "libobsidian_link_resolver-${RELEASE_TAG}",
    ] {
        assert!(
            RELEASE_WORKFLOW.contains(asset_family),
            "release workflow is missing asset family {asset_family}"
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
