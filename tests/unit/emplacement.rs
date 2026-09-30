use obsidian_link_resolver::link::parse_link;
use obsidian_link_resolver::note::find_target_range;
use obsidian_link_resolver::output::LineRange;

#[test]
fn computes_heading_section_end_before_next_equal_or_higher_heading() {
    let contents =
        "# Root\n\n## Parent\n\n### Child\nchild content\n\n## Sibling\nsibling content\n";
    let link = parse_link("[[Note#Child]]").expect("link should parse");

    let range = find_target_range(contents, &link)
        .expect("target range lookup should succeed")
        .expect("heading target should produce a range");

    assert_eq!(range, LineRange { begin: 5, end: 7 });
}

#[test]
fn derives_enclosing_ranges_for_structured_block_targets() {
    let contents = "# Emplacement\n\n## Blocks\n\n> quoted line 1\n> quoted line 2\n\n^quote-block\n\n> [!note] callout line 1\n> callout line 2\n\n^callout-block\n\n| Col | Value |\n| --- | ----- |\n| one | two |\n\n^table-block\n\n- parent item\n  - child item ^list-block\n  continuation of child\n- sibling item\n\n```text\ncode line 1\ncode line 2\n```\n\n^code-block\n\n$$\nmath line 1\nmath line 2\n$$\n\n^math-block\n\n> outer quote\n>> middle quote\n>>> inner quote ^triple-quote-block\n";

    let cases = [
        (
            "[[Emplacement#^quote-block]]",
            LineRange { begin: 5, end: 6 },
        ),
        (
            "[[Emplacement#^callout-block]]",
            LineRange { begin: 10, end: 11 },
        ),
        (
            "[[Emplacement#^triple-quote-block]]",
            LineRange { begin: 40, end: 42 },
        ),
        (
            "[[Emplacement#^list-block]]",
            LineRange { begin: 21, end: 24 },
        ),
        (
            "[[Emplacement#^code-block]]",
            LineRange { begin: 26, end: 29 },
        ),
        (
            "[[Emplacement#^math-block]]",
            LineRange { begin: 33, end: 36 },
        ),
    ];

    for (raw_link, expected) in cases {
        let link = parse_link(raw_link).expect("link should parse");
        let range = find_target_range(contents, &link)
            .expect("target range lookup should succeed")
            .expect("structured block target should produce a range");
        assert_eq!(
            range, expected,
            "{raw_link} should resolve to the enclosing block"
        );
    }
}
