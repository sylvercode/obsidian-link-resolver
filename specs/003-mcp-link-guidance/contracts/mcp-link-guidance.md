# MCP Contract: Link Guidance

## Existing Tool

The contract remains `resolve_obsidian_link`. Its name, input schema, result schema, and resolver behavior do not change.

### Link guidance

The discoverable tool description identifies these representative forms:

- Wikilink: `[[Note]]`
- Wikilink heading: `[[Note#Heading]]`
- Wikilink block reference: `[[Note#^blockid]]`
- Markdown link: `[text](Note.md)`
- Markdown heading link: `[text](Note.md#Heading)`

The caller passes the link exactly as it appears in the Markdown source and supplies the file containing it as `context_path`. The caller does not convert one link form into another. A folder of Markdown files can be used without the Obsidian application, but the server can access only directories configured at startup.

### Initialization

Server initialization gives a concise statement of the MCP's resolver purpose. Detailed syntax and call guidance stays on the resolver tool.

### Boundaries

- The caller supplies a link and context; the server does not discover links or search documents.
- No new MCP tool or skill is added.
- Existing parsing and resolution outcomes remain authoritative.
- Existing request fields and result fields remain unchanged.
