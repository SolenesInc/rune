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
    let markers = |rt: &Runtime<Disclosed>| rt.elements().into_iter().filter(|e| e.key.as_deref() == Some("marker")).map(|e| e.role).collect::<Vec<_>>();
    assert_eq!(markers(&rt), ["icon"], "the marker is drawn, not set in a font that may lack it");

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

const DRAWN: &str = "# A \"title\"\n\nHello **world** &amp; [a link](https://x.dev) -- it's `code`.\n\n- one\n- two\n\n  second paragraph\n  - nested\n\n- [ ] a task\n\n> quoted\n\n> [!NOTE]\n> noted\n\n<details>\n<summary>More</summary>\n\nhidden body\n\n</details>\n\n| a | b |\n|---|---|\n| c | d |\n\n```rust\nfn main() {}\n```\n\n```diff\n-old\n+new\n```\n";

fn drawn_view(src: &String, _: &mut Cx<String>) -> El {
    col().w_full().child(Markdown::new(src.clone()).smart_punctuation(true).selectable(true).build(&Kit::neutral(true)).key("md"))
}

#[test]
fn each_selectable_text_a_document_draws_is_drawn_text_found_by_its_path() {
    let mut rt = Runtime::new(DRAWN.to_owned(), drawn_view, (480.0, 2000.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let first = rt.elements().into_iter().find(|e| e.path.ends_with("md/b1")).expect("the first paragraph");
    rt.pointer_down(first.rect[0] + 2.0, first.rect[1] + 4.0);
    rt.pointer_up(first.rect[0] + 2.0, first.rect[1] + 4.0);
    rt.key(rune::KeyEvent { key: rune::Key::Command('a'), mods: rune::Mods { cmd: true, ctrl: cfg!(not(target_os = "macos")), ..rune::Mods::default() } });
    rt.frame(1.0 / 60.0);
    let drawn = Markdown::new(DRAWN).smart_punctuation(true).drawn(&Kit::neutral(true));
    let sel = rt.selection().expect("everything selected");
    let elements = rt.elements();
    let mut seen = vec![0; drawn.texts().len()];
    for piece in sel.texts().iter().filter(|t| t.len() > 0) {
        let within = piece.path().split_once("md/").map(|(_, p)| p).unwrap_or_else(|| panic!("{} is outside the document", piece.path()));
        let i = drawn.at(within).unwrap_or_else(|| panic!("no drawn text at {within}"));
        let element = elements.iter().find(|e| e.path == piece.path()).expect("the piece's element");
        let unmarked = |s: &str| s.chars().filter(|c| !('\u{2066}'..='\u{2069}').contains(c)).collect::<String>();
        assert_eq!(unmarked(element.text.as_deref().unwrap_or_default()), unmarked(&drawn.texts()[i].text), "{within}");
        assert_eq!(piece.authored(), 0..drawn.texts()[i].text.len(), "{within}: the whole text, as authored");
        seen[i] += 1;
    }
    assert!(seen.iter().all(|&n| n == 1), "each drawn text is one selected text: {seen:?} of {:#?}", drawn.texts().iter().map(|t| &t.text).collect::<Vec<_>>());
}

#[test]
fn drawn_text_maps_selected_bytes_to_the_source_they_came_from() {
    let src = "Hello **world** &amp; it's here.";
    let drawn = Markdown::new(src).smart_punctuation(true).drawn(&Kit::neutral(true));
    let [para] = drawn.texts() else { panic!("one paragraph") };
    assert_eq!(para.text, "Hello world & it\u{2019}s here.");
    let world = para.text.find("world").unwrap();
    assert_eq!(para.source(world..world + 5), [8..13]);
    assert_eq!(para.source(0..world + 5), [0..6, 8..13]);
    let amp = para.text.find('&').unwrap();
    assert_eq!(para.source(amp..amp + 1), [16..21], "an entity maps as a whole");
    let quote = para.text.find('\u{2019}').unwrap();
    assert_eq!(&src[para.source(quote..quote + 3)[0].clone()], "'", "a curly quote maps to its straight one");
}

#[test]
fn drawn_blocks_span_their_lines_and_texts_say_where_they_sit() {
    let src = "# Title\n\n```\ncode\n```\n\n- a\n- b\n";
    let drawn = Markdown::new(src).drawn(&Kit::neutral(true));
    let blocks: Vec<&str> = drawn.blocks().iter().map(|r| &src[r.clone()]).collect();
    assert_eq!(blocks, ["# Title", "```\ncode\n```", "- a\n- b"]);
    let at: Vec<&[DrawnStep]> = drawn.texts().iter().map(DrawnText::at).collect();
    use DrawnStep::*;
    assert_eq!(at, [&[Block(0)][..], &[Block(1)], &[Block(2), Item(0), Block(0)], &[Block(2), Item(1), Block(0)]]);
}

#[derive(Default)]
struct Pressed { heard: Vec<String> }

const PRESSED: &str = "Mark this [link](https://x.dev) passage.\n\n```\nlet mark = 1;\n```\n";

fn pressed_view(_: &Pressed, _: &mut Cx<Pressed>) -> El {
    let this = PRESSED.find("this").unwrap()..PRESSED.find(" passage").unwrap() + 8;
    let mark = PRESSED.find("mark =").unwrap()..PRESSED.find("mark =").unwrap() + 4;
    col().w_full().child(Markdown::new(PRESSED).selectable(true)
        .highlights([(this, Highlight::new().key("n1")), (mark, Highlight::new().key("n2"))])
        .on_link(|s: &mut Pressed, url| s.heard.push(url.to_owned()))
        .on_highlight(|s: &mut Pressed, key, _| s.heard.push(key.to_owned()))
        .build(&Kit::neutral(true)))
}

/// Centres of the glyphs drawn inside the element whose path ends `end`, in reading order.
fn glyphs_in(rt: &Runtime<Pressed>, end: &str) -> Vec<(f32, f32)> {
    let e = rt.elements().into_iter().find(|e| e.path.ends_with(end)).unwrap_or_else(|| panic!("{end}"));
    let mut out: Vec<(f32, f32)> = rt.instances.iter()
        .filter(|i| i.params[1] == rune::render::KIND_GLYPH)
        .map(|i| (i.rect[0] + i.rect[2] / 2.0, i.rect[1] + i.rect[3] / 2.0))
        .filter(|&(x, y)| x >= e.rect[0] && x <= e.rect[0] + e.rect[2] && y >= e.rect[1] && y <= e.rect[1] + e.rect[3])
        .collect();
    out.sort_by(|a, b| ((a.1 / 10.0).round(), a.0).partial_cmp(&((b.1 / 10.0).round(), b.0)).unwrap());
    out
}

fn click(rt: &mut Runtime<Pressed>, (x, y): (f32, f32)) {
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.frame(1.0 / 60.0);
    rt.pointer_up(x, y);
    rt.frame(2.0 / 60.0);
}

#[test]
fn a_highlight_is_a_button_over_its_glyphs_beside_the_links_inside_it() {
    let mut rt = Runtime::new(Pressed::default(), pressed_view, (480.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let para = glyphs_in(&rt, "b0");
    // "Mark" is glyphs 0..4, "this" 4..8, "link" 8..12, "passage." 12..20.
    click(&mut rt, para[1]);
    assert!(rt.state.heard.is_empty(), "outside the mark nothing is pressed");
    click(&mut rt, para[5]);
    click(&mut rt, para[9]);
    click(&mut rt, para[14]);
    let code = glyphs_in(&rt, "b1");
    // "let" is glyphs 0..3, "mark" 3..7.
    click(&mut rt, code[4]);
    assert_eq!(rt.state.heard, ["n1", "https://x.dev", "n1", "n2"], "the link inside the mark opens as a link");
    assert!(names(&rt, "button").iter().any(|n| n == "this "), "a mark is a button named by its words: {:?}", names(&rt, "button"));
    let (x0, y0) = (para[5].0 - 2.0, para[5].1);
    let (x1, y1) = para[16];
    rt.pointer_move(x0, y0);
    rt.pointer_down(x0, y0);
    rt.frame(3.0 / 60.0);
    for k in 1..=6 {
        rt.pointer_move(x0 + (x1 - x0) * k as f32 / 6.0, y0);
        rt.frame((3 + k) as f64 / 60.0);
    }
    rt.pointer_up(x1, y1);
    rt.frame(10.0 / 60.0);
    assert!(rt.selected_text().is_some_and(|t| t.starts_with("his link pass")), "a drag from inside a mark selects: {:?}", rt.selected_text());
    assert_eq!(rt.state.heard.len(), 4, "and presses nothing");
}

#[test]
fn a_struck_highlight_draws_a_line_through_its_words_in_prose_and_in_code() {
    const SRC: &str = "Cut these words here.\n\n```\nlet cut = 1;\n```\n";
    fn struck(src: &String, _: &mut Cx<String>) -> El {
        let at = |s: &str| SRC.find(s).unwrap()..SRC.find(s).unwrap() + s.len();
        let marks = vec![(vec![at("these words")], Highlight::new().strike().key("cut")), (vec![at("cut")], Highlight::new().strike()), (vec![at("here")], Highlight::new())];
        col().w_full().child(Markdown::new(src.clone()).highlights(marks).build(&Kit::neutral(true)))
    }
    let mut rt = Runtime::new(SRC.to_owned(), struck, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let strikes = |shown: &str| {
        rt.elements().into_iter().find(|e| e.text.as_deref() == Some(shown)).map(|e| e.strikes).unwrap_or_else(|| panic!("no text {shown:?}"))
    };
    assert_eq!(strikes("Cut these words here."), vec![[4, 15]]);
    assert_eq!(strikes("let cut = 1;"), vec![[4, 7]]);
    let els = rt.elements();
    assert!(els.iter().any(|e| e.key.as_deref() == Some("cut") && e.role == "mark"), "the keyed mark is its own element");
    assert_eq!(els.iter().filter(|e| !e.strikes.is_empty()).count(), 2, "only the texts carry their strikes, not the marks inside them");
}
