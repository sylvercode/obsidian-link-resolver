# Data Model: MCP Link Guidance

This feature adds no persisted or runtime domain entities. It documents the existing MCP request and two static metadata surfaces.

## Existing Resolver Request

| Field | Type | Meaning and constraints |
|---|---|---|
| `link` | String | The link text copied from the Markdown source. Callers pass it exactly as written; existing resolver parsing remains authoritative. |
| `context_path` | String | Path to the Markdown file containing the link, supplied as resolution context. The server must be configured to access its containing directory at startup. |
| `vault_root` | Optional string | Existing selector for an allowed root; it cannot expand the server's startup-configured access boundary. |
| `with_emplacement` | Optional boolean | Existing option for structured location detail; unrelated to link-guidance semantics. |

## Static Metadata

| Surface | Content responsibility |
|---|---|
| Resolver tool description | Identify the four representative Wikilink/Markdown forms; require verbatim source-link submission and source-file context; clarify that configured Markdown folders do not require the Obsidian app and access is limited to startup-configured directories. |
| Initialization instructions | Briefly state the MCP's resolver purpose without repeating detailed syntax guidance. |

## Relationships and State

The tool description explains how to provide the existing request. The initialization text orients the client to the same tool. There are no new relationships, validation states, or state transitions; access remains governed by the existing startup allow-list and resolution remains governed by the existing resolver.
