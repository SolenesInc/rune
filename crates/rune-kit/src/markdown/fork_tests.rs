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
    assert_eq!(rt.state().flipped, [at]);
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
    assert!(rt.state().heard.is_empty(), "outside the mark nothing is pressed");
    click(&mut rt, para[5]);
    click(&mut rt, para[9]);
    click(&mut rt, para[14]);
    let code = glyphs_in(&rt, "b1");
    // "let" is glyphs 0..3, "mark" 3..7.
    click(&mut rt, code[4]);
    assert_eq!(rt.state().heard, ["n1", "https://x.dev", "n1", "n2"], "the link inside the mark opens as a link");
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
    assert_eq!(rt.state().heard.len(), 4, "and presses nothing");
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

#[cfg(feature = "mermaid")]
fn source_framed_view(src: &String, _: &mut Cx<String>) -> El {
    let md = Markdown::new(src.clone()).diagram_source_frame(|source, d| {
        col().child(text(format!("fence {}", d.index)).key("why")).child(source)
    });
    col().w_full().child(md.build(&Kit::neutral(true)))
}

#[cfg(feature = "mermaid")]
#[test]
fn a_diagram_that_does_not_draw_comes_to_the_frame_as_its_source() {
    let src = "```mermaid\ngraph LR\n  A --> B\n```\n\n```mermaid\nkanban\n  Todo\n```\n";
    let mut rt = Runtime::new(src.to_owned(), source_framed_view, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let els = rt.elements();
    let within = |block: &str, key: &str| {
        let b = els.iter().find(|e| e.key.as_deref() == Some(block)).unwrap_or_else(|| panic!("{block}"));
        els.iter().any(|e| e.key.as_deref() == Some(key) && e.path.starts_with(&b.path) && e.path != b.path)
    };
    assert!(within("b0", "card") && !within("b0", "why"), "a drawn diagram isn't framed as source");
    assert!(within("b1", "source") && within("b1", "why"), "a fence shown as its source comes to the frame with it");
}

#[test]
fn list_items_and_their_markers_and_deep_headings_take_their_own_font_and_colours() {
    let (marker, deep) = (rune::oklch(0.6, 0.2, 260.0), rune::oklch(0.9, 0.0, 0.0));
    let small = TextStyle { size: 11.0, line_height: 14.0, ..Kit::neutral(true).ty.body };
    fn styled(src: &String, _: &mut Cx<String>) -> El {
        let k = Kit::neutral(true);
        let style = MarkdownStyle::from_kit(&k)
            .list_font(TextStyle { size: 11.0, line_height: 14.0, ..k.ty.body })
            .list_marker(rune::oklch(0.6, 0.2, 260.0))
            .deep_heading(rune::oklch(0.9, 0.0, 0.0));
        col().w_full().child(Markdown::new(src.clone()).render(&style))
    }
    let mut rt = Runtime::new("Above.\n\n- One\n- Two\n\n#### Deep\n".to_owned(), styled, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let elements = rt.elements();
    let shown = |t: &str| elements.iter().find(|e| e.text.as_deref() == Some(t)).unwrap_or_else(|| panic!("no text {t:?}"));
    assert_eq!(shown("•").text_color, Some(marker));
    assert_eq!(shown("Deep").text_color, Some(deep));
    assert_eq!(shown("One").rect[3], small.line_height);
    assert!(shown("Above.").rect[3] > small.line_height, "a paragraph outside a list keeps the body font");
}

#[test]
fn alerts_show_their_icon_before_their_title() {
    let rt = laid_out("> [!WARNING]\n> Careful.\n\n> [!TIP]\n> Try.\n");
    let icons: Vec<_> = rt.elements().into_iter().filter(|e| e.key.as_deref() == Some("icon")).collect();
    assert_eq!(icons.len(), 2);
    let elements = rt.elements();
    let title = elements.iter().find(|e| e.text.as_deref() == Some("Warning")).expect("the title");
    assert!(icons[0].rect[0] + icons[0].rect[2] <= title.rect[0], "the icon stands before the title");
    assert_eq!(icons[0].text_color, title.text_color);
}

#[test]
fn an_app_draws_an_image_with_no_picture_in_its_own_words() {
    fn framed(src: &String, _: &mut Cx<String>) -> El {
        col().w_full().child(
            Markdown::new(src.clone())
                .images(|_| None)
                .missing_image_frame(|_, image| text(format!("[missing image: {} at {}]", image.alt, image.src)))
                .build(&Kit::neutral(true)),
        )
    }
    let mut rt = Runtime::new("![A chart](chart.png)\n\n![](plain.png)\n".to_owned(), framed, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let texts: Vec<_> = rt.elements().into_iter().filter_map(|e| e.text).collect();
    assert!(texts.iter().any(|t| t == "[missing image: A chart at chart.png]"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "[missing image:  at plain.png]"), "an image without alt text comes too: {texts:?}");
    assert!(!texts.iter().any(|t| t == "A chart"), "the default glyph and alt text are replaced: {texts:?}");
}

#[test]
fn each_heading_level_takes_its_own_colour() {
    fn styled(src: &String, _: &mut Cx<String>) -> El {
        let style = MarkdownStyle::from_kit(&Kit::neutral(true))
            .heading_text(2, rune::oklch(0.8, 0.0, 0.0))
            .heading_text(3, rune::oklch(0.7, 0.0, 0.0));
        col().w_full().child(Markdown::new(src.clone()).render(&style))
    }
    let mut rt = Runtime::new("# One\n\n## Two\n\n### Three\n\nBody.\n".to_owned(), styled, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let elements = rt.elements();
    let ink = |t: &str| elements.iter().find(|e| e.text.as_deref() == Some(t)).unwrap_or_else(|| panic!("no text {t:?}")).text_color;
    assert_eq!(ink("Two"), Some(rune::oklch(0.8, 0.0, 0.0)));
    assert_eq!(ink("Three"), Some(rune::oklch(0.7, 0.0, 0.0)));
    assert_eq!(ink("One"), ink("Body."), "a level left alone keeps the text colour");
}

#[test]
fn an_alerts_bar_runs_the_whole_height_of_its_box() {
    let rt = laid_out("> [!IMPORTANT]\n> One line changes behavior.\n");
    let elements = rt.elements();
    let alert = elements.iter().find(|e| e.role == "note").expect("the alert");
    let title = elements.iter().find(|e| e.text.as_deref() == Some("Important")).expect("the title");
    let bar = elements
        .iter()
        .find(|e| e.rect[2] == 3.0 && e.fill == title.text_color.expect("the title's colour"))
        .expect("the bar");
    assert_eq!((bar.rect[1], bar.rect[3]), (alert.rect[1], alert.rect[3]), "the bar spans the padding too");
    assert!(title.rect[1] >= alert.rect[1] + 8.0, "the title sits inside the padding");
}

struct Styled {
    src: String,
    style: MarkdownStyle,
}

fn styled_view(s: &Styled, _: &mut Cx<Styled>) -> El {
    col().w_full().child(Markdown::new(s.src.clone()).render(&s.style))
}

fn styled(src: &str, style: impl FnOnce(MarkdownStyle) -> MarkdownStyle) -> Runtime<Styled> {
    let state = Styled { src: src.to_owned(), style: style(MarkdownStyle::from_kit(&Kit::neutral(true))) };
    let mut rt = Runtime::new(state, styled_view, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn shown<'a>(elements: &'a [rune::ElementInfo], t: &str) -> &'a rune::ElementInfo {
    elements.iter().find(|e| e.text.as_deref() == Some(t)).unwrap_or_else(|| panic!("no text {t:?}"))
}

#[test]
fn a_finished_task_draws_dimmed_and_struck_through_and_its_nested_list_does_not() {
    let done = rune::oklch(0.6, 0.0, 0.0);
    let src = "- [x] Finished\n  - inside\n- [ ] Open\n";
    let rt = styled(src, |s| s.task_done(done));
    let elements = rt.elements();
    assert_eq!(shown(&elements, "Finished").text_color, Some(done));
    assert_eq!(shown(&elements, "Finished").strikes, [[0, "Finished".len() as u32]]);
    assert!(shown(&elements, "inside").strikes.is_empty(), "a list inside the item keeps its own look");
    assert!(shown(&elements, "Open").strikes.is_empty());
    assert_ne!(shown(&elements, "Open").text_color, Some(done));
    let plain = styled(src, |s| s);
    assert!(shown(&plain.elements(), "Finished").strikes.is_empty(), "by default a finished task reads as an open one");
}

#[test]
fn a_task_checkbox_keeps_its_ring_checked_and_its_text_the_gap_away() {
    let (ring, mark) = (rune::oklch(0.7, 0.15, 145.0), rune::oklch(0.7, 0.15, 145.0));
    let rt = styled("- [x] Finished\n- [ ] Open\n", |s| s.task_checked(Color::TRANSPARENT, ring, mark).task_ring(2.0, 11.0));
    let elements = rt.elements();
    let boxes: Vec<_> = elements.iter().filter(|e| e.key.as_deref() == Some("control")).collect();
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes[0].fill, Color::TRANSPARENT, "a checked box stays a ring");
    let marks: Vec<_> = elements.iter().filter(|e| e.key.as_deref() == Some("mark")).collect();
    assert_eq!(marks[0].text_color, Some(mark));
    let text = shown(&elements, "Finished");
    assert_eq!(text.rect[0], boxes[0].rect[0] + boxes[0].rect[2] + 11.0);
}

#[test]
fn rules_table_rules_and_code_outlines_take_the_line_width() {
    let src = "Above.\n\n---\n\n| a |\n|---|\n| b |\n| c |\n";
    let thick = styled(src, |s| s.lines(1.0));
    let hairline = styled(src, |s| s);
    let heights = |rt: &Runtime<Styled>| -> Vec<f32> {
        let mut h: Vec<f32> = rt.elements().iter().filter(|e| e.rect[3] > 0.0 && e.rect[3] <= 1.0 && e.rect[2] > 100.0).map(|e| e.rect[3]).collect();
        h.sort_by(f32::total_cmp);
        h
    };
    assert!(!heights(&thick).is_empty() && heights(&thick).iter().all(|&h| h == 1.0), "{:?}", heights(&thick));
    assert!(heights(&hairline).iter().all(|&h| h == 0.5), "{:?}", heights(&hairline));
}

#[test]
fn blocks_space_by_the_margins_the_style_gives_them() {
    let src = "# Title\n\nText.\n\n### Code\n\n```\nlet x = 1;\n```\n\n- item\n\n---\n\nAfter.\n";
    let rt = styled(src, |s| {
        s.heading_margin(1, 21.0, 14.0)
            .heading_margin(3, 21.0, 7.0)
            .paragraph_margin(14.0)
            .code_margin(17.5, 17.5)
            .list_margin(5.25, 5.25)
            .rule_margin(28.0, 28.0)
            .heading_margins_collapse(true)
    });
    let elements = rt.elements();
    let top = |t: &str| shown(&elements, t).rect[1];
    let bottom = |t: &str| shown(&elements, t).rect[1] + shown(&elements, t).rect[3];
    assert_eq!(top("Text.") - bottom("Title"), 14.0);
    assert_eq!(top("Code") - bottom("Text."), 21.0);
    let code = elements.iter().find(|e| e.role == "code").expect("the code block");
    assert_eq!(code.rect[1] - bottom("Code"), 17.5, "a heading's bottom margin collapses with the code's top");
    let rule = elements.iter().find(|e| e.rect[3] == 0.5 && e.rect[1] > bottom("item")).expect("the rule");
    assert_eq!(rule.rect[1] - bottom("item"), 28.0);
    assert_eq!(top("After.") - (rule.rect[1] + rule.rect[3]), 28.0);

    let default = styled(src, |s| s);
    let elements = default.elements();
    let code = elements.iter().find(|e| e.role == "code").expect("the code block");
    let heading = shown(&elements, "Code");
    assert_eq!(code.rect[1] - (heading.rect[1] + heading.rect[3]), 8.0, "by default a heading's bottom margin alone sets the gap");
}

#[test]
fn a_quote_can_be_italic_and_its_words_lean() {
    let src = "> Every word of this quote.\n";
    let upright = styled(src, |s| s);
    let italic = styled(src, |s| s.quote_italic(true));
    let width = |rt: &Runtime<Styled>| shown(&rt.elements(), "Every word of this quote.").rect[2];
    let ink = |rt: &Runtime<Styled>| rt.instances.iter().filter(|i| i.params[1] == rune::render::KIND_GLYPH).map(|i| (i.rect, i.params)).collect::<Vec<_>>();
    assert!(width(&upright) != width(&italic) || ink(&upright).iter().zip(ink(&italic)).any(|(a, b)| *a != b), "italic draws other glyphs");
}

#[test]
fn inline_code_keeps_its_size_whatever_text_it_sits_in() {
    let src = "Some `code` here.\n\n- an `item`\n";
    let rt = styled(src, |s| s.list_font(TextStyle { size: 11.0, line_height: 16.0, ..Kit::neutral(true).ty.body }).inline_code_size(12.0));
    let scaled = styled(src, |s| s.list_font(TextStyle { size: 11.0, line_height: 16.0, ..Kit::neutral(true).ty.body }));
    let width = |rt: &Runtime<Styled>| {
        let e = rt.elements().into_iter().find(|e| e.text.as_deref().is_some_and(|t| t.contains("item"))).expect("the item");
        let xs: Vec<f32> = rt.instances.iter()
            .filter(|i| i.params[1] == rune::render::KIND_GLYPH)
            .filter(|i| i.rect[1] >= e.rect[1] && i.rect[1] + i.rect[3] <= e.rect[1] + e.rect[3])
            .map(|i| i.rect[0] + i.rect[2])
            .collect();
        xs.into_iter().fold(0.0f32, f32::max)
    };
    assert!(width(&rt) > width(&scaled), "the list's inline code grows to its own size");
}

#[test]
fn ordered_list_numbers_can_set_figures_at_one_width() {
    let src = "1. one\n2. two\n";
    let marks = |rt: &Runtime<Styled>| -> Vec<[f32; 4]> {
        rt.instances.iter().filter(|i| i.params[1] == rune::render::KIND_GLYPH).map(|i| i.rect).filter(|r| r[0] < 30.0).collect()
    };
    let tabular = styled(src, |s| s.list_markers_outside(true).list_tabular_figures(true));
    let proportional = styled(src, |s| s.list_markers_outside(true));
    assert_ne!(marks(&tabular), marks(&proportional), "the numbers set at another width");
}

#[test]
fn an_outside_marker_stands_the_gap_away_from_its_text() {
    let src = "- one\n";
    let bullet_end = |rt: &Runtime<Styled>| {
        let column = shown(&rt.elements(), "•").rect;
        rt.instances.iter()
            .filter(|i| i.params[1] == rune::render::KIND_GLYPH)
            .map(|i| i.rect)
            .filter(|r| r[0] >= column[0] && r[0] + r[2] <= column[0] + column[2] && r[1] >= column[1] && r[1] <= column[1] + column[3])
            .map(|r| r[0] + r[2])
            .fold(0.0f32, f32::max)
    };
    let near = bullet_end(&styled(src, |s| s.list_markers_outside(true).list_marker_gap(4.0)));
    let far = bullet_end(&styled(src, |s| s.list_markers_outside(true).list_marker_gap(12.0)));
    assert_eq!(near - far, 8.0, "the marker moves left by what the gap grows");
}

#[test]
fn an_inline_code_chip_pads_the_fonts_ascent_and_descent_by_its_vertical_pad() {
    let chip_bg = rune::oklch(0.5, 0.2, 140.0);
    let chip = |pad: Option<f32>| {
        let rt = styled("Some `code` here.\n", |s| {
            let s = s.inline_code_bg(chip_bg);
            match pad { Some(p) => s.inline_code_pad_y(p), None => s }
        });
        let fill = chip_bg.premul(1.0);
        rt.instances.iter().find(|i| i.params[1] == rune::render::KIND_RECT && i.fill == fill).expect("the code chip").rect
    };
    let (balanced, bare, padded) = (chip(None), chip(Some(0.0)), chip(Some(2.0)));
    assert_ne!(bare, balanced, "a vertical pad leaves the cap height for the font's ascent and descent");
    let grown = padded[3] - bare[3];
    assert!((3.0..=5.0).contains(&grown), "two points above and below: {bare:?} {padded:?}");
    assert!((padded[1] - (bare[1] - 2.0)).abs() <= 1.0, "half the room is above: {bare:?} {padded:?}");
}

#[test]
fn a_prose_measure_centers_top_level_prose_at_the_measure_while_tables_code_and_front_matter_take_the_column() {
    const MEASURE: f32 = 760.0;
    let src = "---\ntitle: Plan\n---\n# Heading\n\nA paragraph long enough to wrap if it had to, and then some more words after that.\n\n- an item\n\n> a quote\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n\n---\n\n![Wide](wide.png)\n";
    fn view((src, column): &(String, f32), _: &mut Cx<(String, f32)>) -> El {
        thread_local! {
            static WIDE: rune::Image = {
                let cache = rune::ImageCache::new(std::sync::Arc::new(|| {}));
                let image = cache.rgba(1200, 8, [40u8, 80, 120, 255].repeat(1200 * 8));
                image.wait().unwrap();
                image
            };
        }
        let picture = WIDE.with(|p| p.clone());
        let style = MarkdownStyle::from_kit(&Kit::neutral(true)).table_full_width(true).text_width(*column).prose_measure(MEASURE);
        let md = Markdown::new(src.clone()).style(style).images(move |_| Some(picture.clone()));
        col().w_full().child(md.build(&Kit::neutral(true)))
    }
    let widths = |column: f32| {
        let mut rt = Runtime::new((src.to_owned(), column), view, (column, 1200.0), Env::new(true, 1.0));
        rt.frame(0.0);
        let elements = rt.elements();
        (0..parse(src).len())
            .map(|i| {
                let key = format!("b{i}");
                let e = elements.iter().find(|e| e.key.as_deref() == Some(key.as_str())).unwrap_or_else(|| panic!("no {key}"));
                (e.rect[0].round(), e.rect[2].round())
            })
            .collect::<Vec<_>>()
    };
    // Front matter, heading, paragraph, list, quote, table, code, rule and a picture 1200 wide.
    let prose = ((1600.0 - MEASURE) / 2.0, MEASURE);
    let column = (0.0, 1600.0);
    assert_eq!(widths(1600.0), [column, prose, prose, prose, prose, column, column, column, (200.0, 1200.0)]);
    assert_eq!(widths(500.0), [(0.0, 500.0); 9]);
}
