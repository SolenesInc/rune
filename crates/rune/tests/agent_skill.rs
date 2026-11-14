//! The agent skill in `plugins/rune` names flags, files and README sections. Each must exist, so
//! the skill can't keep telling agents to use something that was renamed or removed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn skill() -> String {
    read("plugins/rune/skills/rune/SKILL.md")
}

/// The words in `text` that start with `prefix`, each cut at the first character `keep` refuses.
fn words(text: &str, prefix: &str, keep: fn(char) -> bool) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(at) = rest.find(prefix) {
        let boundary = rest[..at].chars().next_back().is_none_or(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '/');
        let word: String = rest[at..].chars().take_while(|&c| keep(c)).collect();
        rest = &rest[at + word.len().max(prefix.len())..];
        if boundary && word[prefix.len()..].starts_with(|c: char| c.is_ascii_alphanumeric()) {
            found.insert(word);
        }
    }
    found
}

#[test]
fn every_flag_the_skill_names_is_one_rune_parses() {
    // Flags of other tools the skill runs.
    let foreign = ["--format-version"];
    let parser = read("crates/rune/src/tooling/args.rs");
    let unknown: Vec<String> = words(&skill(), "--", |c| c.is_ascii_lowercase() || c == '-')
        .into_iter()
        .filter(|flag| !foreign.contains(&flag.as_str()) && !parser.contains(&format!("\"{flag}\"")))
        .collect();
    assert!(unknown.is_empty(), "the skill names flags Rune's headless tooling doesn't parse: {unknown:?}");
}

#[test]
fn every_path_the_skill_names_exists() {
    let skill = skill();
    let path = |c: char| c.is_ascii_alphanumeric() || "/._-*".contains(c);
    let mut missing = Vec::new();
    for prefix in ["crates/", "examples/", "demo/", "scripts/"] {
        for named in words(&skill, prefix, path) {
            let named = named.trim_end_matches('.');
            let exists = match named.split_once('*') {
                None => root().join(named).exists(),
                // `demo/kit/*.png`, `demo/settings-*.png`: the folder holds a file named that way.
                Some((start, ending)) => {
                    let (dir, start) = start.rsplit_once('/').unwrap();
                    std::fs::read_dir(root().join(dir)).is_ok_and(|files| {
                        files.flatten().map(|f| f.file_name().to_string_lossy().into_owned()).any(|f| f.starts_with(start) && f.ends_with(ending))
                    })
                }
            };
            if !exists {
                missing.push(named.to_string());
            }
        }
    }
    assert!(missing.is_empty(), "the skill names paths this repository doesn't have: {missing:?}");
}
