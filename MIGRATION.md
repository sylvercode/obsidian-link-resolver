# Migration Notes

## 1.x to 2.0.0

The public `obsidian_link_resolver::cli` module has been removed from the
resolver library. CLI parsing types (`CliArgs` and `OutputFormat`) now belong
to the `obsidian-link-resolver-cli` executable crate and are not part of the
library API.

Applications that used the resolver library should continue to call the
library's resolution API and use the result types from
`obsidian_link_resolver::output`. Applications that depended on the CLI
argument types should instead invoke the `obsidian-link-resolver` executable
or maintain their own argument parsing.
