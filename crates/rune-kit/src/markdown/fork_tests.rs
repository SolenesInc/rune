//! Tests of the SolenesInc fork's markdown patches (`FORK.md`).

use super::*;
use rune::{Cx, Env, Runtime, col};

fn view(src: &String, _: &mut Cx<String>) -> El {
    col().w_full().child(Markdown::new(src.clone()).build(&Kit::neutral(true)))
}

fn laid_out(src: &str) -> Runtime<String> {
    let mut rt = Runtime::new(src.to_owned(), view, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn names(rt: &Runtime<String>, role: &str) -> Vec<String> {
    fn walk(v: &serde_json::Value, role: &str, out: &mut Vec<String>) {
        if v.get("role").and_then(|r| r.as_str()) == Some(role) {
            out.push(v.get("name").and_then(|n| n.as_str()).unwrap_or_default().to_owned());
        }
        for c in v.get("children").and_then(|c| c.as_array()).into_iter().flatten() {
            walk(c, role, out);
        }
    }
    let mut out = Vec::new();
    walk(&rt.accessibility_json(), role, &mut out);
    out
}

#[test]
fn a_gfm_alert_parses_to_its_kind_without_its_marker_and_maps_to_source() {
    let src = "> [!WARNING]\n> Mind the gap.\n\n> [!note]\n> Lower case.\n\n> [!NOPE]\n> A quote.\n";
    let b = parse(src);
    assert!(matches!(&b[0], Block::Alert(Alert::Warning, inner) if matches!(&inner[0], Block::Para(p) if p.text == "Mind the gap.")), "{b:?}");
    assert!(matches!(&b[1], Block::Alert(Alert::Note, _)), "{b:?}");
    assert!(matches!(&b[2], Block::Quote(inner) if matches!(&inner[0], Block::Para(p) if p.text.starts_with("[!NOPE]"))), "{b:?}");
    let gap = src.find("gap").unwrap();
    assert_eq!(find(src, "gap", Case::Insensitive), vec![vec![gap..gap + 3]]);
}

#[test]
fn a_gfm_alert_shows_its_title_over_what_it_says() {
    let rt = laid_out("> [!CAUTION]\n> Hot.\n");
    assert_eq!(names(&rt, "note"), ["Caution"]);
    let labels = names(&rt, "label");
    assert!(labels.iter().any(|l| l == "Caution") && labels.iter().any(|l| l == "Hot."), "{labels:?}");
    assert!(!labels.iter().any(|l| l.contains("[!CAUTION]")), "{labels:?}");
}

#[test]
fn front_matter_is_its_own_block_that_an_app_can_frame() {
    let src = "---\ntitle: Plan\ntags: [a, b]\n---\n# Heading\n";
    let b = parse(src);
    assert!(matches!(&b[0], Block::FrontMatter(yaml) if yaml == "title: Plan\ntags: [a, b]"), "{b:?}");
    assert!(matches!(&b[1], Block::Heading(1, h) if h.text == "Heading"), "{b:?}");
    assert_eq!(parse("---\nstill text").len(), 2, "an unclosed fence is a rule and text");

    fn framed(src: &String, _: &mut Cx<String>) -> El {
        let card = |_: El, yaml: &str| text(format!("card {}", yaml.lines().count())).key("fm");
        col().w_full().child(Markdown::new(src.clone()).front_matter_frame(card).build(&Kit::neutral(true)))
    }
    let mut rt = Runtime::new(src.to_owned(), framed, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let labels = names(&rt, "label");
    assert!(labels.iter().any(|l| l == "card 2"), "{labels:?}");
    assert_eq!(names(&rt, "heading"), ["Heading"]);
    assert!(!labels.iter().any(|l| l.contains("title: Plan")), "{labels:?}");
}
