use obsidian_link_resolver::output::Status;
use obsidian_link_resolver::resolve_with_vault;
use obsidian_link_resolver::vault::{enumerate_vault, ContextFile, Vault, VaultSource};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn witm_vault_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp/WitM-DM")
}

fn is_external_markdown_target(target: &str) -> bool {
    let target = target.trim();
    let lower = target.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("obsidian://")
}

fn extract_wikilinks(contents: &str) -> Vec<String> {
    let bytes = contents.as_bytes();
    let mut index = 0;
    let mut links = Vec::new();

    while index + 1 < bytes.len() {
        let start = if bytes[index] == b'[' && bytes[index + 1] == b'[' {
            Some(index)
        } else if index + 2 < bytes.len()
            && bytes[index] == b'!'
            && bytes[index + 1] == b'['
            && bytes[index + 2] == b'['
        {
            Some(index)
        } else {
            None
        };

        let Some(start) = start else {
            index += 1;
            continue;
        };

        let mut end = start + 2;
        while end + 1 < bytes.len() {
            if bytes[end] == b']' && bytes[end + 1] == b']' {
                let slice_end = end + 2;
                let link = &contents[start..slice_end];
                let inner = if let Some(inner) = link.strip_prefix("![[") {
                    inner.strip_suffix("]]").unwrap_or("")
                } else {
                    link.strip_prefix("[[")
                        .and_then(|value| value.strip_suffix("]]"))
                        .unwrap_or("")
                };

                if !inner.contains("[[") && !inner.contains("]]") {
                    links.push(link.to_string());
                }
                index = slice_end;
                break;
            }
            end += 1;
        }

        if end + 1 >= bytes.len() {
            break;
        }
    }

    links
}

fn extract_markdown_links(contents: &str) -> Vec<String> {
    let bytes = contents.as_bytes();
    let mut links = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let mut open_bracket = index;
        let mut found = false;
        while open_bracket < bytes.len() {
            if bytes[open_bracket] == b'[' {
                found = true;
                break;
            }
            open_bracket += 1;
        }
        if !found {
            break;
        }

        let mut close_bracket = open_bracket + 1;
        while close_bracket < bytes.len() && bytes[close_bracket] != b']' {
            close_bracket += 1;
        }
        if close_bracket + 1 >= bytes.len() || bytes[close_bracket + 1] != b'(' {
            index = open_bracket + 1;
            continue;
        }

        let mut close_paren = close_bracket + 2;
        while close_paren < bytes.len() && bytes[close_paren] != b')' {
            close_paren += 1;
        }
        if close_paren >= bytes.len() {
            break;
        }

        let full_start = if open_bracket > 0 && bytes[open_bracket - 1] == b'!' {
            open_bracket - 1
        } else {
            open_bracket
        };
        let link = &contents[full_start..=close_paren];

        let target = &contents[close_bracket + 2..close_paren];
        if !is_external_markdown_target(target) {
            links.push(link.to_string());
        }

        index = close_paren + 1;
    }

    links
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

#[test]
fn resolves_internal_witm_links_without_error_status() {
    let vault_root = witm_vault_root();
    if !vault_root.exists() {
        eprintln!(
            "skipping WitM smoke test because vault is missing: {}",
            vault_root.display()
        );
        return;
    }

    let entries = enumerate_vault(vault_root.to_string_lossy().as_ref())
        .expect("WitM vault should enumerate successfully");
    let vault = Vault {
        root: vault_root.to_string_lossy().into_owned(),
        source: VaultSource::Explicit,
        entries,
    };

    let mut markdown_files: Vec<PathBuf> = WalkDir::new(&vault_root)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.into_path())
        .filter(|path| path.is_file() && is_markdown_file(path))
        .collect();
    markdown_files.sort();

    let mut checked_links = 0usize;
    let mut errors = Vec::new();

    for file in markdown_files {
        let contents = fs::read_to_string(&file).unwrap_or_default();
        let mut links = extract_wikilinks(&contents);
        links.extend(extract_markdown_links(&contents));
        let context = ContextFile {
            path: file.to_string_lossy().into_owned(),
        };

        for link in links {
            checked_links += 1;
            let result = resolve_with_vault(&link, &context.path, &vault, false);
            if result.status == Status::Error {
                errors.push(format!(
                    "{} :: {} :: {}",
                    context.path,
                    link,
                    result.reason.as_deref().unwrap_or("unknown error")
                ));
            }
        }
    }

    assert!(
        checked_links > 0,
        "WitM smoke test found no links to validate"
    );
    assert!(
        errors.is_empty(),
        "WitM smoke test found {} internal link(s) with error status:\n{}",
        errors.len(),
        errors.join("\n")
    );
}
