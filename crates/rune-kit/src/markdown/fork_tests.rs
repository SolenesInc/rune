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

fn names<S>(rt: &Runtime<S>, role: &str) -> Vec<String> {
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

#[test]
fn inline_html_breaks_lines_and_styles_text_and_drops_other_tags() {
    let src = "| a |\n|---|\n| one<br>two |\n\nSay <b>bold</b>, <kbd>Esc</kbd>, <s>old</s> and <name> here.\n";
    let b = parse(src);
    let Block::Table(t) = &b[0] else { panic!("{b:?}") };
    assert_eq!(t.rows[0][0].text, "one\ntwo");
    let Block::Para(p) = &b[1] else { panic!("{b:?}") };
    assert_eq!(p.text, "Say bold, Esc, old and  here.");
    let styled: Vec<(&str, Style)> = p.spans.iter().map(|&(a, z, s)| (&p.text[a..z], s)).collect();
    assert_eq!(styled, [("bold", Style::Strong), ("Esc", Style::Code), ("old", Style::Strike)]);
    let two = src.find("two").unwrap();
    assert_eq!(find(src, "one two", Case::Insensitive).len(), 0, "a break is not a space");
    assert_eq!(find(src, "two", Case::Insensitive), vec![vec![two..two + 3]]);
    assert_eq!(parse("a </b> b").first(), parse("a  b").first(), "an unmatched closing tag ends nothing");
}

#[test]
fn text_links_link_what_the_app_finds_in_prose_and_leave_code_and_links_alone() {
    fn view(_: &(), _: &mut Cx<()>) -> El {
        let src = "# Plan s-1\n\nBlocked on **s-1** and [s-1](other), see `s-1`.\n\n```\ns-1\n```\n\n> - s-1\n\n| a |\n|---|\n| s-1 |";
        Markdown::new(src)
            .on_link(|_: &mut (), _| {})
            .text_links(|run| run.match_indices("s-1").map(|(at, w)| (at..at + w.len(), "seed:s-1".to_owned())).collect())
            .render(&MarkdownStyle::neutral(true))
    }
    let mut rt = Runtime::new((), view, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let links: Vec<String> = rt.elements().into_iter().filter(|e| e.role == "link").filter_map(|e| e.key).collect();
    assert_eq!(links, ["seed:s-1", "seed:s-1", "other", "seed:s-1", "seed:s-1"], "heading, bold, the written link, quoted list, table cell");
}

#[test]
fn text_links_skip_ranges_that_overlap_leave_the_run_or_cut_a_character() {
    fn view(_: &(), _: &mut Cx<()>) -> El {
        Markdown::new("é abc")
            .on_link(|_: &mut (), _| {})
            .text_links(|_| vec![(0..1, "cut".into()), (3..5, "first".into()), (4..6, "overlap".into()), (5..99, "out".into()), (5..5, "empty".into())])
            .render(&MarkdownStyle::neutral(true))
    }
    let mut rt = Runtime::new((), view, (480.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let links: Vec<String> = rt.elements().into_iter().filter(|e| e.role == "link").filter_map(|e| e.key).collect();
    assert_eq!(links, ["first"]);
}

#[test]
fn a_details_block_holds_its_summary_and_markdown_body_and_maps_to_source() {
    let src = "Intro\n\n<details>\n<summary>More <b>here</b></summary>\n\n- one\n- two\n\n</details>\n\n<details open><summary>Open</summary>\n\nShown\n\n</details>\n\nAfter\n";
    let b = parse(src);
    assert_eq!(b.len(), 4, "{b:?}");
    let Block::Details { summary, open: false, body } = &b[1] else { panic!("{b:?}") };
    assert_eq!(summary.text, "More here");
    assert!(matches!(&body[0], Block::List { items, .. } if items.len() == 2), "{body:?}");
    assert!(matches!(&b[2], Block::Details { summary, open: true, .. } if summary.text == "Open"), "{b:?}");
    assert!(matches!(&b[3], Block::Para(p) if p.text == "After"), "{b:?}");
    let two = src.find("two").unwrap();
    assert_eq!(find(src, "two", Case::Insensitive), vec![vec![two..two + 3]]);
    assert!(matches!(parse("<details><summary>Cut</summary>\n\nstill arriving").as_slice(), [Block::Details { body, .. }] if body.len() == 1), "an unclosed block holds the rest");
}

struct Disclosed {
    src: String,
    flipped: Vec<usize>,
}

fn disclosed(d: &Disclosed, _: &mut Cx<Disclosed>) -> El {
    let flipped = d.flipped.clone();
    let md = Markdown::new(d.src.clone()).details(
        move |at, open| open != flipped.contains(&at),
        |d: &mut Disclosed, at, _open, _| {
            match d.flipped.iter().position(|&f| f == at) {
                Some(i) => {
                    d.flipped.remove(i);
                }
                None => d.flipped.push(at),
            }
        },
    );
    col().w_full().child(md.build(&Kit::neutral(true)))
}

#[test]
fn a_details_block_opens_and_closes_from_its_summary_as_the_app_says() {
    let src = "<details>\n<summary>More</summary>\n\nHidden body\n\n</details>\n";
    let at = src.find("<details>").unwrap();
    let mut rt = Runtime::new(Disclosed { src: src.into(), flipped: Vec::new() }, disclosed, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let summary = || |rt: &Runtime<Disclosed>| rt.elements().into_iter().find(|e| e.role == "button" && e.label.as_deref() == Some("More")).expect("the summary");
    assert_eq!(summary()(&rt).expanded, Some(false));
    assert!(!names(&rt, "label").iter().any(|l| l == "Hidden body"));

    let r = summary()(&rt).rect;
    let (x, y) = (r[0] + 4.0, r[1] + r[3] * 0.5);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.flipped, [at]);
    assert_eq!(summary()(&rt).expanded, Some(true));
    assert!(names(&rt, "label").iter().any(|l| l == "Hidden body"));

    let plain = laid_out(src);
    assert!(names(&plain, "label").iter().any(|l| l == "Hidden body"), "without a handler the body shows");
}

#[test]
fn a_table_cell_that_holds_only_an_image_is_known_by_its_row_and_column() {
    let src = "| ![Before](a.png) | ![After](b.png \"later\") |\n|---|---|\n| ![Shot](shot.png) | text ![inline](c.png) |\n";
    let b = parse(src);
    let Block::Table(t) = &b[0] else { panic!("{b:?}") };
    let at: Vec<_> = t.pictures.iter().map(|p| (p.row, p.column, p.src.as_str(), p.title.as_str())).collect();
    assert_eq!(at, [(None, 0, "a.png", ""), (None, 1, "b.png", "later"), (Some(0), 0, "shot.png", "")]);
    assert_eq!(t.rows[0][1].text, "text inline", "an image beside text stays its alt text");
}

#[test]
fn a_table_cell_image_shows_its_picture_at_its_size_and_its_alt_text_until_then() {
    fn shown(src: &String, _: &mut Cx<String>) -> El {
        thread_local! {
            static PICTURE: rune::Image = {
                let cache = rune::ImageCache::new(std::sync::Arc::new(|| {}));
                let image = cache.rgba(64, 32, [40u8, 80, 120, 255].repeat(64 * 32));
                image.wait().unwrap();
                image
            };
        }
        let picture = PICTURE.with(|p| p.clone());
        let md = Markdown::new(src.clone()).images(move |src| (src == "shot.png").then(|| picture.clone()));
        col().w_full().child(md.build(&Kit::neutral(true)))
    }
    let src = "| Shot | Gone |\n|---|---|\n| ![A shot](shot.png) | ![Missing](gone.png) |\n";
    let mut rt = Runtime::new(src.to_owned(), shown, (480.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let images: Vec<_> = rt.elements().into_iter().filter(|e| e.role == "image").collect();
    let labels: Vec<_> = images.iter().map(|e| e.label.as_deref().unwrap_or_default()).collect();
    assert_eq!(labels, ["A shot", "Missing"]);
    assert_eq!(images[0].rect[2], 64.0, "the picture at its own width");
}

#[test]
fn smart_punctuation_sets_prose_and_maps_each_piece_to_its_source() {
    let src = "\"Ship it...\" -- pages 3--4 :tada: `a--b \"c\"` <kbd>--x</kbd>\n";
    let smart = Reading { autolink: true, smart: true };
    let (blocks, _, _, _, _, maps) = parse_doc(src, smart);
    let [Block::Para(p)] = blocks.as_slice() else { panic!("{blocks:?}") };
    let code = |s: &str| format!("{}{s}{}", parser::OPEN, parser::CLOSE);
    assert_eq!(p.text, format!("“Ship it…” -- pages 3–4 🎉 {} {}", code("a--b \"c\""), "--x"));
    let [BlockMap::Inline(map)] = maps.as_slice() else { panic!("{maps:?}") };
    let source_of = |shown: &str| {
        let at = p.text.find(shown).unwrap();
        let (s, out) = map.0.iter().find(|(_, out)| out.contains(&at)).unwrap_or_else(|| panic!("{shown:?} in {map:?}"));
        if s.len() == out.len() { &src[s.start + at - out.start..s.start + at - out.start + shown.len()] } else { &src[s.clone()] }
    };
    assert_eq!((source_of("…"), source_of("“"), source_of("–"), source_of("🎉")), ("...", "\"", "--", ":tada:"));
    let (plain, ..) = parse_doc(src, Reading::LINKED);
    assert!(matches!(plain.as_slice(), [Block::Para(p)] if p.text.starts_with("\"Ship it...\"")), "off by default: {plain:?}");
}
