# MCP Link Guidance Brainstorm

## Context

The MCP resolves Obsidian-style links in Markdown documentation. Users may have Markdown folders without using the Obsidian app or maintaining a formally configured Obsidian vault. The model needs to recognize supported link syntax in a Markdown file and know to pass that link to the resolver.

## Direction

Keep this as guidance in the existing MCP metadata, not as a new tool or skill. Explain the accepted link forms in the resolver tool description, where the model can see them while deciding how to call the tool. Keep server initialization instructions brief and focused on the MCP's purpose.

Suggested tool-description wording:

> Pass the link exactly as written in the Markdown file. Supported forms include Wikilinks such as `[[Note]]` and `[[Note#Heading]]`, and Markdown links such as `[text](Note.md)` and `[text](Note.md#Heading)`. Do not rewrite one form as the other.

## Scope

- The MCP resolves a link provided by the model, with the source Markdown file as context.
- A folder of Markdown files is sufficient; the Obsidian app is not required.
- The server can access only directories configured at startup with `--vault`.
- Link discovery, document search, new tools, and new skills are out of scope.
- This note records a brainstorm only; no implementation changes were requested.
