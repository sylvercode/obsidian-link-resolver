use clap::Parser;
use obsidian_link_resolver::cli::{CliArgs, OutputFormat};

fn main() {
    let args = CliArgs::parse();
    let result = obsidian_link_resolver::resolve(
        &args.link,
        &args.context,
        args.vault.as_deref(),
        args.emplacement,
    );

    match args.format {
        OutputFormat::Json => match result.to_json() {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("failed to serialize result: {error}");
                std::process::exit(1);
            }
        },
        OutputFormat::Human => println!("{}", result.to_human_string()),
    }

    if (result.status == obsidian_link_resolver::output::Status::Error || args.verbose > 0)
        && result.reason.is_some()
    {
        if let Some(reason) = &result.reason {
            eprintln!("{reason}");
        }
    }

    std::process::exit(result.exit_code());
}
