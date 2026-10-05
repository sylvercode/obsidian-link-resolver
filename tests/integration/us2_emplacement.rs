use obsidian_link_resolver::output::{LineRange, Status};
use obsidian_link_resolver::resolve;
use std::path::PathBuf;

/// Return the absolute path to the shared fixture vault root.
///
/// # Returns
///
/// Absolute fixture vault path used by integration tests.
fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault")
}

/// Resolve a fixture-case link while controlling whether emplacement data is requested.
///
/// # Parameters
///
/// - `link`: Link string to resolve.
/// - `context_rel`: Context path relative to the fixture root.
/// - `with_emplacement`: Whether to request structured emplacement data.
///
/// # Returns
///
/// Resolver output for the test case.
fn resolve_case(
    link: &str,
    context_rel: &str,
    with_emplacement: bool,
) -> obsidian_link_resolver::output::ResolutionTarget {
    let fixture_root = fixture_root();
    let context = fixture_root.join(context_rel);

    resolve(
        link,
        context.to_string_lossy().as_ref(),
        None,
        with_emplacement,
    )
}

/// Verifies nested heading links produce the expected heading stack and section range.
#[test]
fn returns_nested_heading_stack_and_section_ranges() {
    let result = resolve_case("[[Design#API#Auth]]", "notes/a.md", true);
    let same_result = resolve_case("[[Design#Auth]]", "notes/a.md", true);

    assert_eq!(same_result, result);

    assert_eq!(result.status, Status::Resolved);
    assert_eq!(result.target_path.as_deref(), Some("Design.md"));
    assert_eq!(result.target_range, Some(LineRange { begin: 5, end: 7 }));

    let emplacement = result.emplacement.expect("emplacement should be present");
    assert_eq!(emplacement.heading_stack.len(), 3);
    assert_eq!(emplacement.heading_stack[0].text, "Design");
    assert_eq!(emplacement.heading_stack[0].level, 1);
    assert_eq!(emplacement.heading_stack[0].begin, 1);
    assert_eq!(emplacement.heading_stack[0].end, 7);
    assert_eq!(emplacement.heading_stack[1].text, "API");
    assert_eq!(emplacement.heading_stack[1].level, 2);
    assert_eq!(emplacement.heading_stack[1].begin, 3);
    assert_eq!(emplacement.heading_stack[1].end, 7);
    assert_eq!(emplacement.heading_stack[2].text, "Auth");
    assert_eq!(emplacement.heading_stack[2].level, 3);
    assert_eq!(emplacement.heading_stack[2].begin, 5);
    assert_eq!(emplacement.heading_stack[2].end, 7);
    assert_eq!(emplacement.section, LineRange { begin: 5, end: 7 });
}

/// Verifies whole-file emplacement is returned for notes without heading structure.
#[test]
fn returns_empty_stack_and_whole_file_section_for_flat_notes() {
    let result = resolve_case("[[Flat]]", "notes/a.md", true);

    assert_eq!(result.status, Status::Resolved);
    assert_eq!(result.target_path.as_deref(), Some("notes/Flat.md"));
    assert_eq!(result.target_range, None);

    let emplacement = result.emplacement.expect("emplacement should be present");
    assert!(emplacement.heading_stack.is_empty());
    assert_eq!(emplacement.section, LineRange { begin: 1, end: 3 });
}

/// Assert a link resolves to the expected structured block envelope in emplacement output.
///
/// # Parameters
///
/// - `raw_link`: Raw link value to resolve.
/// - `expected_range`: Expected canonical target range.
/// - `expected_kind`: Expected serialized structured block kind.
/// - `expected_block_id`: Expected block id on the top-level structured block.
/// - `expected_child_block_id`: Expected block id on a nested child node, when applicable.
fn assert_structured_block(
    raw_link: &str,
    expected_range: LineRange,
    expected_kind: &str,
    expected_block_id: Option<&str>,
    expected_child_block_id: Option<&str>,
) {
    let result = resolve_case(raw_link, "notes/a.md", true);
    assert_eq!(result.status, Status::Resolved);
    assert_eq!(result.target_path.as_deref(), Some("notes/emplacement.md"));
    assert_eq!(result.target_range, Some(expected_range.clone()));

    let emplacement = result
        .emplacement
        .as_ref()
        .expect("emplacement should be present");
    let blocks_heading = emplacement
        .heading_stack
        .iter()
        .find(|heading| heading.text == "Blocks")
        .expect("Blocks heading should be present");
    assert_eq!(blocks_heading.begin, 3);
    assert_eq!(blocks_heading.end, 43);

    let json = serde_json::to_value(result).expect("result should serialize");
    assert_eq!(
        json["emplacement"]["structured_block"]["kind"],
        expected_kind
    );
    assert_eq!(
        json["emplacement"]["structured_block"]["begin"],
        expected_range.begin
    );
    assert_eq!(
        json["emplacement"]["structured_block"]["end"],
        expected_range.end
    );

    match expected_block_id {
        Some(block_id) => {
            assert_eq!(
                json["emplacement"]["structured_block"]["block_id"],
                block_id
            );
        }
        None => {
            assert!(json["emplacement"]["structured_block"]["block_id"].is_null());
        }
    }

    match expected_child_block_id {
        Some(block_id) => {
            assert!(json["emplacement"]["structured_block"]["items"].is_array());
            assert_eq!(
                json["emplacement"]["structured_block"]["items"][1]["block_id"],
                block_id
            );
        }
        None => {
            if expected_block_id.is_none() {
                assert!(json["emplacement"]["structured_block"]["items"]
                    .as_array()
                    .is_some_and(|items| items.is_empty()));
            }
        }
    }
}

/// Verifies supported structured block kinds expose canonical begin/end boundaries.
#[test]
fn returns_structured_block_boundaries_for_supported_blocks() {
    assert_structured_block(
        "[[Emplacement#^quote-block]]",
        LineRange { begin: 5, end: 6 },
        "quote",
        Some("quote-block"),
        None,
    );
    assert_structured_block(
        "[[Emplacement#^callout-block]]",
        LineRange { begin: 10, end: 11 },
        "callout",
        Some("callout-block"),
        None,
    );
    assert_structured_block(
        "[[Emplacement#^table-block]]",
        LineRange { begin: 15, end: 17 },
        "table",
        Some("table-block"),
        None,
    );
    assert_structured_block(
        "[[Emplacement#^list-block]]",
        LineRange { begin: 21, end: 24 },
        "list",
        None,
        Some("list-block"),
    );
    assert_structured_block(
        "[[Emplacement#^code-block]]",
        LineRange { begin: 26, end: 29 },
        "code",
        Some("code-block"),
        None,
    );
    assert_structured_block(
        "[[Emplacement#^math-block]]",
        LineRange { begin: 33, end: 36 },
        "math",
        Some("math-block"),
        None,
    );
    assert_structured_block(
        "[[Emplacement#^triple-quote-block]]",
        LineRange { begin: 40, end: 42 },
        "quote",
        Some("triple-quote-block"),
        None,
    );
}
