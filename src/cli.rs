//! Command-line interface argument parsing and configuration.
//!
//! This module handles parsing command-line arguments for the CLI binary,
//! validating them, and producing a [`CliArgs`] structure that controls
//! the resolver's behavior.

use clap::{Parser, ValueEnum};

/// Output mode selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Compact machine-readable JSON.
    Json,
    /// Human-readable summary text.
    Human,
}

/// Parsed command-line arguments for the CLI.
///
/// Represents all command-line options provided by the user, validated and ready for use
/// by the main resolver pipeline.
///
/// # Fields
///
/// - `link`: The Obsidian link string to resolve (positional argument, required)
/// - `context`: Absolute path of the file containing the link (`--context`, required)
/// - `vault`: Optional explicit vault root path (`--vault`, defaults to auto-detect)
/// - `format`: Output format ("json" or "human"; default "json")
/// - `verbose`: Verbosity level (`-v` is 1, `-vv` is 2, etc.; repeatable flag)
/// - `emplacement`: Whether to include structured heading stack and section ranges in the result (`--emplacement`)
///
/// # Arguments
///
/// - `<LINK>` (positional): The link string to resolve
/// - `--context <FILE>`: Absolute path to the file containing the link
/// - `--vault <DIR>`: Optional vault root; if omitted, auto-detected from the context file
/// - `--format <FORMAT>`: Output format ("json" or "human"; default "json")
/// - `-v, --verbose`: Increase diagnostic verbosity (repeatable; only affects stderr, not stdout)
/// - `--emplacement`: Include structured emplacement (heading stack + section ranges) if applicable
/// - `-h, --help`: Display help and exit
/// - `-V, --version`: Display version and exit
#[derive(Debug, Clone, Parser)]
#[command(name = "obsidian-link-resolver", version, about = "Resolve Obsidian links to target files and locations")]
pub struct CliArgs {
    /// The Obsidian link string to resolve (positional argument, required).
    #[arg(value_name = "LINK")]
    pub link: String,
    /// Absolute path of the file containing the link (`--context`, required).
    #[arg(long)]
    pub context: String,
    /// Optional explicit vault root path (`--vault`); if `None`, vault is auto-detected.
    #[arg(long)]
    pub vault: Option<String>,
    /// Output format: "json" (compact machine-readable) or "human" (interactive).
    /// Defaults to "json" if not specified.
    #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,
    /// Verbosity level: 0 (default) to N (repeatable `-v`/`--verbose` flags).
    /// Only affects diagnostics printed to stderr.
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// If `true`, include the structured emplacement (heading stack and section ranges) in the result.
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub emplacement: bool,
}
