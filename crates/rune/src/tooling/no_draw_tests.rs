//! `--no-draw`: a draw-free run writes the trees a drawn run writes, and refuses what it can't do.

use super::*;
use super::tests::HEADLESS;

/// Runs a headless case through `run` twice: first draw-free, with `--no-draw`, each tree
/// (`--tree` and every `--tree-at`) beside the drawn one's and any `--snap` left out, then as
/// given, so the test reads the drawn run's outputs and side effects. Asserts each pair of trees
/// matches byte for byte: skipping the drawing never changes a tree.
pub(super) fn drawn_and_draw_free(args: &[&str], mut run: impl FnMut(Vec<String>)) {
    let mut trees = Vec::new();
    let mut free = vec!["--no-draw".to_owned()];
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--snap" => i += 1,
            "--tree" => {
                let tree = args[i + 1].to_owned();
                free.extend(["--tree".to_owned(), format!("{tree}.no-draw.json")]);
                trees.push(tree);
                i += 1;
            }
            "--tree-at" => {
                let (at, tree) = args[i + 1].split_once('=').expect("--tree-at AT=PATH");
                free.extend(["--tree-at".to_owned(), format!("{at}={tree}.no-draw.json")]);
                trees.push(tree.to_owned());
                i += 1;
            }
            arg => free.push(arg.to_owned()),
        }
        i += 1;
    }
    assert!(!trees.is_empty(), "a case that writes a tree");
    run(free);
    run(args.iter().map(|&a| a.to_owned()).collect());
    for tree in trees {
        let (drawn, draw_free) = (std::fs::read_to_string(&tree).unwrap(), std::fs::read_to_string(format!("{tree}.no-draw.json")).unwrap());
        if let Some((n, (a, b))) = drawn.lines().zip(draw_free.lines()).enumerate().find(|(_, (a, b))| a != b) {
            panic!("--no-draw changed {tree} of {args:?} at line {}: drawn {a:?}, draw-free {b:?}", n + 1);
        }
        assert_eq!(drawn.len(), draw_free.len(), "--no-draw changed the length of {tree} for {args:?}");
    }
}

#[test]
fn a_draw_free_run_refuses_every_pixel_output_and_a_window() {
    let parsed = Args::parse_from(["--no-draw", "--tree", "t.json", "--tree-at", "mark:a=a.json", "--chrome", "--keys", "--trace", "t.jsonl", "--frames", "30"].map(String::from).into());
    assert!(parsed.no_draw, "--no-draw parses");
    assert_eq!(parsed.check(), Ok(()), "a tree run takes --no-draw with its other flags");
    assert!(!Args::parse_from(vec!["--tree".into(), "t.json".into()]).no_draw, "off by default");

    let tree = || Args { no_draw: true, tree: Some("t.json".into()), ..Default::default() };
    let pixels = [
        ("--snap", Args { snap: Some("a.png".into()), ..tree() }),
        ("--out", Args { out: Some("frames".into()), ..tree() }),
        ("--video", Args { video: Some("a.mp4".into()), ..tree() }),
        ("--strip", Args { strip: Some("strip.png".into()), ..tree() }),
        ("--first-pixels", Args { first_pixels: true, ..tree() }),
    ];
    for (flag, args) in pixels {
        let error = args.check().unwrap_err();
        assert!(error.contains("--no-draw") && error.contains(flag), "{flag}: {error}");
    }
    let windowed = [Args { no_draw: true, ..Default::default() }, Args { no_draw: true, native: true, script: Some("s.json".into()), ..Default::default() }];
    for args in windowed {
        assert!(args.check().unwrap_err().contains("--no-draw is for headless runs"), "{args:?}");
    }
}

struct Ui;

impl crate::Compositor<()> for Ui {
    fn render(&mut self, f: &mut crate::Frame<'_, ()>) { f.draw_ui(); }
}

#[test]
fn a_draw_free_run_refuses_an_app_with_a_compositor() {
    let _turn = HEADLESS.lock().unwrap_or_else(|e| e.into_inner());
    let app = || crate::App::new(AppSpec::new("comp", (40.0, 30.0), || (), |_, _| crate::el())).compositor(Ui);
    let args = |extra: &[&str]| {
        let mut args = vec!["--no-draw", "--tree", "never-written.json"];
        args.extend_from_slice(extra);
        Args::parse_from(args.into_iter().map(String::from).collect())
    };
    assert_eq!(try_headless(app(), args(&[])), Err(NO_DRAW_COMPOSITOR.to_owned()));
    let dir = std::env::temp_dir().join(format!("rune-no-draw-compositor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let backdrop = dir.join("backdrop.png");
    ::image::RgbaImage::new(80, 60).save(&backdrop).unwrap();
    let error = try_headless(app(), args(&["--desktop", backdrop.to_str().unwrap()])).unwrap_err();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(error, format!("{NO_DRAW_COMPOSITOR} (window \"main\")"), "a --desktop window with a compositor");
}
