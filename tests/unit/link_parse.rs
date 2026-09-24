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
    assert_eq!(link.display_text.as_deref(), Some("Alias"));

    let markdown =
        parse_link("[Display](Some%20Note.md#Heading)").expect("markdown link should parse");
    assert_eq!(markdown.style, LinkStyle::Markdown);
    assert_eq!(markdown.display_text.as_deref(), Some("Display"));
    assert_eq!(markdown.note_name.as_deref(), Some("Some Note.md"));
    assert_eq!(markdown.heading_path, vec!["Heading"]);

    let utf8_markdown = parse_link("[Display](Some%20Note%20%C3%A9.md#Heading)")
        .expect("utf-8 markdown link should parse");
    assert_eq!(utf8_markdown.note_name.as_deref(), Some("Some Note é.md"));
    assert_eq!(utf8_markdown.heading_path, vec!["Heading"]);

    let same_file = parse_link("[[#Overview]]").expect("same-file link should parse");
    assert!(same_file.note_name.is_none());
    assert_eq!(same_file.heading_path, vec!["Overview"]);

    let same_file_block = parse_link("[[#^abc123]]").expect("same-file block link should parse");
    assert_eq!(same_file_block.block_id.as_deref(), Some("abc123"));

    let case_insensitive =
        parse_link("[[project plan#milestones]]").expect("case-insensitive match should parse");
    assert_eq!(case_insensitive.note_name.as_deref(), Some("project plan"));
    assert_eq!(case_insensitive.heading_path, vec!["milestones"]);

    assert!(parse_link("[[Note#Heading#^abc123]]").is_err());
}
