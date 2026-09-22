use obsidian_link_resolver::link::{parse_link, LinkStyle};

#[test]
fn parses_wikilinks_markdown_links_and_rejects_invalid_sub_targets() {
    let link = parse_link("![[folder/sub/Note#Heading#Child|Alias]]").expect("link should parse");
    assert_eq!(link.raw, "![[folder/sub/Note#Heading#Child|Alias]]");
    assert_eq!(link.style, LinkStyle::Wikilink);
    assert!(link.is_embed);
    assert_eq!(link.folder_path.as_deref(), Some("folder/sub"));
    assert_eq!(link.note_name.as_deref(), Some("Note"));
    assert_eq!(link.heading_path, vec!["Heading", "Child"]);
    assert!(link.block_id.is_none());
    assert_eq!(link.alias.as_deref(), Some("Alias"));

    let markdown = parse_link("[Display](Some%20Note.md#Heading)").expect("markdown link should parse");
    assert_eq!(markdown.style, LinkStyle::Markdown);
    assert_eq!(markdown.alias.as_deref(), Some("Display"));
    assert_eq!(markdown.note_name.as_deref(), Some("Some Note.md"));
    assert_eq!(markdown.heading_path, vec!["Heading"]);

    let same_file = parse_link("[[#Overview]]").expect("same-file link should parse");
    assert!(same_file.note_name.is_none());
    assert_eq!(same_file.heading_path, vec!["Overview"]);

    assert!(parse_link("[[Note#Heading#^abc123]]").is_err());
}
