//! `--tree-at`: several trees from one run, each the tree a separate `--at X --tree` run writes.

use super::*;
use super::tests::HEADLESS;

fn args(v: &[&str]) -> Args { Args::parse_from(v.iter().map(|s| (*s).into()).collect()) }

#[test]
fn tree_at_parses_a_moment_and_a_path() {
    assert_eq!(parse_tree_at("1.5=a.json"), Ok((At::Seconds(1.5), "a.json".into())));
    assert_eq!(parse_tree_at("mark:done=out/done.json"), Ok((At::Mark("done".into()), "out/done.json".into())));
    assert_eq!(parse_tree_at("mark:step-1+0.5=a=b.json"), Ok((At::Mark("step-1+0.5".into()), "a=b.json".into())), "splits at the first =");
    for bad in ["a.json", "=a.json", "1.5=", "mark:=a.json", "soon=a.json", "-1=a.json", "inf=a.json"] {
        let error = parse_tree_at(bad).unwrap_err();
        assert!(error.starts_with("--tree-at is AT=PATH") && error.ends_with(bad), "{error}");
    }
    let a = args(&["--tree-at", "0.5=a.json", "--tree-at", "mark:end=b.json"]);
    assert!(a.headless() && a.wants_tree());
    assert_eq!(a.tree_at.len(), 2);
}

#[test]
fn two_trees_to_one_path_are_refused() {
    let mut a = Args { tree: Some("a.json".into()), ..Default::default() };
    a.tree_at = vec![(At::Seconds(0.5), "b.json".into())];
    assert_eq!(check_tree_paths(&a), Ok(()));
    a.tree_at.push((At::Seconds(1.0), "a.json".into()));
    assert_eq!(check_tree_paths(&a).unwrap_err(), "two trees would write a.json: give each --tree and --tree-at its own path");
    a.tree = None;
    a.tree_at.push((At::Seconds(2.0), "b.json".into()));
    assert!(check_tree_paths(&a).unwrap_err().contains("b.json"));
}

#[test]
fn tree_requests_resolve_to_frames_and_a_run_with_only_them_stops_at_the_latest() {
    let script = parse_script(json!([
        {"t": 0.5, "type": "tap", "x": 1, "y": 1},
        {"t": 1.0, "type": "mark", "name": "picked"},
        {"t": 2.0, "type": "mark", "name": "end"},
    ]));
    let timeline = |v: &[&str]| Timeline::of(&args(v), &script);
    let only = timeline(&["--tree-at", "mark:picked=a.json", "--tree-at", "0.25=b.json"]).unwrap();
    assert_eq!(only.trees, [(60, "a.json".into()), (15, "b.json".into())]);
    assert_eq!((only.snap_frame, only.last_frame), (60, 60), "stops at the latest request, not 1 s after the script");
    let with_tree = timeline(&["--tree", "t.json", "--tree-at", "mark:picked-0.5=a.json"]).unwrap();
    assert_eq!(with_tree.trees, [(180, "t.json".into()), (30, "a.json".into())], "--tree keeps its default moment");
    assert_eq!(with_tree.last_frame, 180);
    let later = timeline(&["--snap", "s.png", "--at", "0.5", "--tree-at", "mark:end+1=a.json"]).unwrap();
    assert_eq!((later.snap_frame, later.last_frame), (30, 180), "--at still sets the snapshot; the request extends the run");
    let dumped = timeline(&["--frames", "200", "--tree-at", "0.5=a.json"]).unwrap();
    assert_eq!(dumped.last_frame, 199, "dumped frames still run to their end");
    assert_eq!(timeline(&["--tree-at", "mark:gone=a.json"]).unwrap_err(), "no mark \"gone\" in the script (marks: picked, end)");
    let plain = timeline(&["--tree", "t.json"]).unwrap();
    assert_eq!((plain.snap_frame, plain.last_frame, plain.trees), (180, 180, vec![(180, "t.json".into())]));
}

/// A fitted window that grows a row on each tap, on its spring, and counts the taps in text.
fn growing() -> crate::App<usize> {
    let spec = AppSpec::new("grow", (300.0, 400.0), || 1usize, |rows, _| crate::col().key("root").w_full()
        .child(crate::el().key("more").size(50.0, 50.0).on_press(|rows: &mut usize| *rows += 1))
        .child(crate::text(format!("{rows} rows")).key("count"))
        .children((0..*rows).map(|i| crate::el().key(format!("row{i}")).size(100.0, 40.0))));
    crate::App::new(spec).fit_content_height()
}

const GROW_SCRIPT: &str = r#"[{"t": 0.1, "type": "tap", "key": "more"}, {"t": 0.2, "type": "mark", "name": "pressed"},
    {"t": 0.5, "type": "tap", "key": "more"}, {"t": 0.6, "type": "mark", "name": "again"}]"#;

/// Each moment's tree from one run, then from one `--at X --tree` run per moment.
fn compare(dir: &std::path::Path, base: &[&str], moments: &[&str], run: &dyn Fn(Vec<String>)) {
    let mut once: Vec<String> = base.iter().map(|s| (*s).into()).collect();
    for (i, at) in moments.iter().enumerate() {
        once.extend(["--tree-at".into(), format!("{at}={}", dir.join(format!("once-{i}.json")).display())]);
    }
    // `--tree` beside the requests keeps its own moment.
    once.extend(["--at".into(), moments[0].into(), "--tree".into(), dir.join("plain.json").display().to_string()]);
    run(once);
    let mut written = Vec::new();
    for (i, at) in moments.iter().enumerate() {
        let separate = dir.join(format!("separate-{i}.json"));
        let mut alone: Vec<String> = base.iter().map(|s| (*s).into()).collect();
        alone.extend(["--at".into(), (*at).into(), "--tree".into(), separate.display().to_string()]);
        run(alone);
        let (once, separate) = (std::fs::read(dir.join(format!("once-{i}.json"))).unwrap(), std::fs::read(separate).unwrap());
        assert!(once == separate, "{at}: one run's tree differs from a separate run's\n{}\n---\n{}",
            String::from_utf8_lossy(&once), String::from_utf8_lossy(&separate));
        written.push(once);
    }
    assert_eq!(std::fs::read(dir.join("plain.json")).unwrap(), written[0], "--tree with --at beside --tree-at");
    assert!(written.windows(2).all(|w| w[0] != w[1]), "the moments show different trees");
}

#[test]
fn one_run_writes_the_tree_a_separate_run_writes_at_each_moment() {
    let _turn = HEADLESS.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("rune-tree-at-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("script.json");
    std::fs::write(&script, GROW_SCRIPT).unwrap();
    // Mid-spring after the first tap, at a mark, at a mark's offset, and at a time.
    let moments = ["mark:pressed", "mark:pressed+0.05", "mark:again", "0.9"];
    let base = ["--script", script.to_str().unwrap(), "--scale", "1", "--fps", "60"];
    compare(&dir, &base, &moments, &|argv| headless(growing(), Args::parse_from(argv)));

    let only: Vec<String> = base.iter().map(|s| (*s).to_string()).chain(["--tree-at".into(), format!("mark:pressed={}", dir.join("only.json").display())]).collect();
    headless(growing(), Args::parse_from(only));
    assert_eq!(headless_run().unwrap().end(), 0.2, "a run with only --tree-at ends at its latest request");
    std::fs::remove_dir_all(dir).unwrap();
}

/// A `--desktop` window counting the keys it gets in text; each key also opens `other`.
struct Counter { n: u32, other: Option<crate::AppHandle<Counter>> }

fn counter(name: &'static str, other: Option<crate::AppHandle<Counter>>, options: crate::WindowOptions) -> (crate::AnyWindowSpec, crate::AppHandle<Counter>) {
    let spec = AppSpec {
        on_key: Some(|c: &mut Counter, _: &crate::KeyEvent, _: &mut crate::Tx| {
            c.n += 1;
            if let Some(other) = &c.other { other.open(); }
            true
        }),
        ..AppSpec::new("counter", (1.0, 1.0), || Counter { n: 0, other: None },
            |c, _| crate::col().w_full().h_full().child(crate::text(format!("{} keys", c.n)).key("count")))
    };
    let app = crate::App::new(spec).init(move || Counter { n: 0, other });
    let handle = app.handle();
    (crate::WindowSpec::from_app(name, app).options(options).boxed(), handle)
}

#[test]
fn one_desktop_run_writes_the_tree_a_separate_run_writes_at_each_moment() {
    let _turn = HEADLESS.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("rune-tree-at-desktop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let backdrop = dir.join("backdrop.png");
    ::image::RgbaImage::from_pixel(240, 160, ::image::Rgba([128, 128, 128, 255])).save(&backdrop).unwrap();
    let script = dir.join("script.json");
    std::fs::write(&script, r#"[{"t": 0.1, "type": "key", "name": "Enter"}, {"t": 0.2, "type": "mark", "name": "opened"},
        {"t": 0.3, "type": "key", "name": "Enter"}, {"t": 0.4, "type": "mark", "name": "twice"}]"#).unwrap();
    let base = ["--desktop", backdrop.to_str().unwrap(), "--script", script.to_str().unwrap(), "--scale", "2", "--fps", "10", "--window", "a"];
    let windows = || {
        let at = |x, y, w, visible| crate::WindowOptions { position: Some((x, y)), physical_size: Some((w, 60)), visible, ..Default::default() };
        let (b, handle) = counter("b", None, at(160, 40, 60, false));
        let (a, _) = counter("a", Some(handle), at(20, 20, 100, true));
        vec![a, b]
    };
    compare(&dir, &base, &["0", "mark:opened", "mark:twice"], &|argv| {
        crate::run_windows_with_args(crate::RunOptions::default(), argv, move |_| windows());
    });
    let tree: Value = serde_json::from_slice(&std::fs::read(dir.join("once-1.json")).unwrap()).unwrap();
    assert_eq!(tree["windows"][1]["visible"], json!(true), "the tree at a mark carries each window's frame: {tree}");
    std::fs::remove_dir_all(dir).unwrap();
}
