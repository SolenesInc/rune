//! The accessibility tree the runtime builds, and the actions it runs.

use accesskit::{Action as Ak, ActionData, ActionRequest, Role, TreeId};
use serde_json::Value;

use crate::{
    Cx, El, Env, GrowthSpec, LiveRegion, Runtime, ScrollSpec, TextEdit, col, row, text, text_input,
};

fn find<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if v["key"] == key {
        return Some(v);
    }
    v["children"].as_array()?.iter().find_map(|c| find(c, key))
}

fn count(v: &Value) -> usize {
    1 + v["children"]
        .as_array()
        .map_or(0, |c| c.iter().map(count).sum())
}

fn states(v: &Value) -> Vec<&str> {
    v["states"]
        .as_array()
        .map_or(Vec::new(), |s| s.iter().filter_map(Value::as_str).collect())
}

fn actions(v: &Value) -> Vec<&str> {
    v["actions"]
        .as_array()
        .map_or(Vec::new(), |s| s.iter().filter_map(Value::as_str).collect())
}

fn request<S>(rt: &Runtime<S>, key: &str, action: Ak, data: Option<ActionData>) -> ActionRequest {
    let tree = rt.accessibility_tree();
    let (id, _) = tree
        .nodes
        .iter()
        .find(|(_, n)| n.author_id() == Some(key))
        .expect("node keyed so");
    ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node: *id,
        data,
    }
}

#[derive(Default)]
struct Form {
    presses: u32,
    on: bool,
    open: bool,
    level: i32,
    name: TextEdit,
    menu: u32,
}

fn form(s: &Form, _: &mut Cx<Form>) -> El {
    col()
        .w_full()
        .gap(8.0)
        .child(text("Settings").key("title").role("heading").level(1))
        .child(
            row()
                .key("save")
                .role("button")
                .focusable(true)
                .on_press(|s: &mut Form| s.presses += 1)
                .on_secondary_press(|s: &mut Form, _, _| s.menu += 1)
                .child(text("Save")),
        )
        .child(
            row()
                .key("close")
                .role("button")
                .label("Close window")
                .focusable(true)
                .on_press(|s: &mut Form| s.presses += 10)
                .child(crate::el().key("x").role("icon").size(12.0, 12.0)),
        )
        .child(
            row()
                .key("wifi")
                .role("switch")
                .checked(s.on)
                .focusable(true)
                .on_press(|s: &mut Form| s.on = !s.on)
                .child(text("Wi-Fi")),
        )
        .child(
            row()
                .key("more")
                .role("button")
                .expanded(s.open)
                .focusable(true)
                .on_press(|s: &mut Form| s.open = !s.open)
                .child(text("More")),
        )
        .child(
            crate::el()
                .key("volume")
                .role("slider")
                .range_value(f64::from(s.level), 0.0, 10.0)
                .range_step(1.0)
                .label("Volume")
                .size(100.0, 12.0)
                .on_step(|s: &mut Form, d, _| s.level = (s.level + d).clamp(0, 10)),
        )
        .child(
            text_input(&s.name, crate::tok::type_::BODY)
                .key("name")
                .single_line()
                .w(200.0)
                .on_edit(|s: &mut Form| &mut s.name),
        )
        .child(
            row()
                .key("off")
                .role("button")
                .disabled(true)
                .child(text("Unavailable")),
        )
        .child(text("Saved").key("status").role("status"))
        .child(text("Decoration").key("deco").a11y_hidden())
}

fn form_rt() -> Runtime<Form> {
    let mut rt = Runtime::new(Form::default(), form, (400.0, 500.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt
}

#[test]
fn controls_report_role_name_value_states_and_actions() {
    let rt = form_rt();
    let tree = rt.accessibility_json();
    assert_eq!(tree["role"], "window");
    let title = find(&tree, "title").unwrap();
    assert_eq!(
        (
            title["role"].as_str(),
            title["name"].as_str(),
            title["level"].as_u64()
        ),
        (Some("heading"), Some("Settings"), Some(1))
    );
    let save = find(&tree, "save").unwrap();
    assert_eq!(
        (save["role"].as_str(), save["name"].as_str()),
        (Some("button"), Some("Save"))
    );
    assert!(
        actions(save).contains(&"click")
            && actions(save).contains(&"focus")
            && actions(save).contains(&"context-menu")
    );
    let close = find(&tree, "close").unwrap();
    assert_eq!(
        close["name"], "Close window",
        "an icon-only button takes its label"
    );
    let wifi = find(&tree, "wifi").unwrap();
    assert_eq!(
        (wifi["role"].as_str(), wifi["name"].as_str()),
        (Some("switch"), Some("Wi-Fi"))
    );
    assert!(states(wifi).contains(&"unchecked"));
    let more = find(&tree, "more").unwrap();
    assert!(states(more).contains(&"collapsed") && actions(more).contains(&"expand"));
    let volume = find(&tree, "volume").unwrap();
    assert_eq!(volume["role"], "slider");
    assert_eq!(volume["range"], serde_json::json!([0.0, 0.0, 10.0, 1.0]));
    assert!(actions(volume).contains(&"increment"));
    let name = find(&tree, "name").unwrap();
    assert_eq!(name["role"], "text-input");
    assert!(actions(name).contains(&"set-value"));
    let off = find(&tree, "off").unwrap();
    assert!(states(off).contains(&"disabled") && actions(off).is_empty());
    let status = find(&tree, "status").unwrap();
    assert!(states(status).contains(&"live-polite"));
    assert!(find(&tree, "deco").is_none(), "a11y_hidden leaves the tree");
    let b = save["bounds"].as_array().unwrap();
    assert!(
        b[2].as_f64().unwrap() > 0.0 && b[3].as_f64().unwrap() > 0.0,
        "bounds in points: {b:?}"
    );
}

#[test]
fn actions_run_the_same_handlers_as_pointer_and_keys() {
    let mut rt = form_rt();
    let click = request(&rt, "save", Ak::Click, None);
    assert!(rt.a11y_action(&click));
    rt.frame(0.0);
    assert_eq!(rt.state().presses, 1);
    let menu = request(&rt, "save", Ak::ShowContextMenu, None);
    assert!(rt.a11y_action(&menu));
    rt.frame(0.0);
    assert_eq!(rt.state().menu, 1);
    let toggle = request(&rt, "wifi", Ak::Click, None);
    rt.a11y_action(&toggle);
    rt.frame(0.0);
    assert!(rt.state().on);
    assert!(states(find(&rt.accessibility_json(), "wifi").unwrap()).contains(&"checked"));
    let expand = request(&rt, "more", Ak::Expand, None);
    assert!(rt.a11y_action(&expand));
    rt.frame(0.0);
    assert!(rt.state().open);
    let expand_again = request(&rt, "more", Ak::Expand, None);
    assert!(
        !rt.a11y_action(&expand_again),
        "expanding an open disclosure does nothing"
    );
    for _ in 0..3 {
        let up = request(&rt, "volume", Ak::Increment, None);
        rt.a11y_action(&up);
        rt.frame(0.0);
    }
    assert_eq!(rt.state().level, 3);
    let to_seven = request(
        &rt,
        "volume",
        Ak::SetValue,
        Some(ActionData::NumericValue(7.2)),
    );
    assert!(
        rt.a11y_action(&to_seven),
        "a slider with a step takes a number"
    );
    rt.frame(0.0);
    assert_eq!(
        rt.state().level,
        7,
        "set as steps through on_step, rounded to a step"
    );
    let set = request(
        &rt,
        "name",
        Ak::SetValue,
        Some(ActionData::Value("Ada".into())),
    );
    assert!(rt.a11y_action(&set));
    rt.frame(0.0);
    assert_eq!(rt.state().name.text, "Ada");
    let tree = rt.accessibility_json();
    let name = find(&tree, "name").unwrap();
    assert_eq!(name["value"], "Ada");
    assert!(
        states(name).contains(&"focused"),
        "editing focuses the field: {name}"
    );
    let off = request(&rt, "off", Ak::Click, None);
    assert!(!rt.a11y_action(&off), "a disabled control takes no action");
}

#[test]
fn focus_moves_through_the_tree() {
    let mut rt = form_rt();
    assert_eq!(rt.accessibility_tree().focus, super::a11y_tree::ROOT);
    let focus = request(&rt, "save", Ak::Focus, None);
    assert!(rt.a11y_action(&focus));
    rt.frame(0.0);
    let tree = rt.accessibility_tree();
    assert_eq!(
        tree.nodes
            .iter()
            .find(|(id, _)| *id == tree.focus)
            .and_then(|(_, n)| n.author_id()),
        Some("save")
    );
}

#[test]
fn updates_send_only_what_changed_and_nothing_while_inactive() {
    let mut rt = form_rt();
    assert!(rt.a11y_update().is_none(), "no client, no tree");
    rt.a11y_activate("Form");
    let full = rt.a11y_update().expect("first update after activation");
    assert!(full.tree.is_some() && full.nodes.len() > 10);
    assert!(rt.a11y_update().is_none(), "nothing changed");
    rt.frame(0.0);
    assert!(rt.a11y_update().is_none(), "an idle frame changes nothing");
    let toggle = request(&rt, "wifi", Ak::Click, None);
    rt.a11y_action(&toggle);
    rt.frame(0.0);
    let update = rt.a11y_update().expect("the switch changed");
    assert!(update.tree.is_none());
    assert!(
        update.nodes.len() <= 3,
        "only the switch and what moved: {:?}",
        update
            .nodes
            .iter()
            .map(|(_, n)| n.role())
            .collect::<Vec<_>>()
    );
    assert!(
        update
            .nodes
            .iter()
            .any(|(_, n)| n.author_id() == Some("wifi"))
    );
    rt.a11y_deactivate();
    rt.frame(0.0);
    assert!(rt.a11y_update().is_none());
}

/// A window closed and opened again keeps its runtime but gets a new adapter, which AccessKit
/// may already hold active (AT-SPI turned on): its first update must hold the whole tree, or
/// AccessKit panics building the tree from it.
#[test]
fn a_reopened_surface_gets_the_whole_tree() {
    let mut rt = form_rt();
    let mut first = crate::AccessibilityLink::new();
    first.activate();
    assert!(
        first
            .service(&mut rt, "Form")
            .expect("activated")
            .tree
            .is_some()
    );
    let toggle = request(&rt, "wifi", Ak::Click, None);
    rt.a11y_action(&toggle);
    rt.frame(0.0);
    assert!(
        first
            .service(&mut rt, "Form")
            .expect("the switch changed")
            .tree
            .is_none()
    );
    drop(first);
    let mut second = crate::AccessibilityLink::new();
    rt.frame(0.0);
    assert!(
        second.service(&mut rt, "Form").is_none(),
        "nothing before the new client asks"
    );
    second.activate();
    assert!(
        second
            .service(&mut rt, "Form")
            .expect("activated")
            .tree
            .is_some()
    );
}

#[derive(Default)]
struct List {
    rows: usize,
}

fn list(s: &List, _: &mut Cx<List>) -> El {
    col()
        .key("scroller")
        .w_full()
        .h(200.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .key("rows")
                .role("list")
                .growth(GrowthSpec::default())
                .virtual_rows(
                    s.rows,
                    20.0,
                    |_: &List, i| format!("row-{i}").into(),
                    |_: &List, i| {
                        row()
                            .role("listitem")
                            .h(20.0)
                            .child(text(format!("Entry {i}")))
                    },
                ),
        )
}

#[test]
fn virtual_lists_report_their_size_and_rows_their_place() {
    let mut rt = Runtime::new(
        List { rows: 10_000 },
        list,
        (300.0, 200.0),
        Env::new(true, 1.0),
    );
    for _ in 0..4 {
        rt.frame(0.0);
    }
    let tree = rt.accessibility_json();
    let rows = find(&tree, "rows").unwrap();
    assert_eq!(rows["setSize"], 10_000);
    let first = find(&tree, "row-0").unwrap();
    assert_eq!(
        (first["position"].as_u64(), first["name"].as_str()),
        (Some(1), Some("Entry 0"))
    );
    assert!(count(&tree) < 400, "only mounted rows: {}", count(&tree));
    let scroller = find(&tree, "scroller").unwrap();
    assert_eq!(scroller["role"], "scroll-view");
    assert!(states(scroller).contains(&"clips") && actions(scroller).contains(&"scroll-down"));
    let down = request(&rt, "scroller", Ak::ScrollDown, None);
    assert!(rt.a11y_action(&down));
    for _ in 0..4 {
        rt.frame(0.0);
    }
    let tree = rt.accessibility_json();
    let y = find(&tree, "scroller").unwrap()["scroll"][1]
        .as_f64()
        .unwrap();
    assert!(y > 100.0, "a page down scrolled: {y}");
}

#[test]
fn scrolling_changes_only_the_viewport_node() {
    let mut rt = Runtime::new(
        List { rows: 200 },
        list,
        (300.0, 200.0),
        Env::new(true, 1.0),
    );
    for _ in 0..4 {
        rt.frame(0.0);
    }
    rt.a11y_activate("List");
    rt.a11y_update();
    let down = request(
        &rt,
        "scroller",
        Ak::SetScrollOffset,
        Some(ActionData::SetScrollOffset(accesskit::Point {
            x: 0.0,
            y: 40.0,
        })),
    );
    rt.a11y_action(&down);
    rt.frame(0.0);
    let update = rt.a11y_update().expect("scrolled");
    let changed: Vec<_> = update
        .nodes
        .iter()
        .map(|(_, n)| n.author_id().unwrap_or(""))
        .collect();
    assert!(changed.contains(&"scroller"), "{changed:?}");
    assert!(
        update.nodes.iter().all(|(_, n)| n.role() != Role::Label),
        "rows keep their content-space bounds: {changed:?}"
    );
}

#[derive(Default)]
struct Field {
    edit: TextEdit,
}

fn field(s: &Field, _: &mut Cx<Field>) -> El {
    col().child(
        text_input(&s.edit, crate::tok::type_::BODY)
            .key("note")
            .w(300.0)
            .on_edit(|s: &mut Field| &mut s.edit),
    )
}

#[test]
fn text_fields_expose_lines_caret_and_selection() {
    let mut edit = TextEdit::new("hello world");
    edit.focused = true;
    edit.set_selection(0, 5);
    let mut rt = Runtime::new(Field { edit }, field, (400.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let tree = rt.accessibility_tree();
    let (_, note) = tree
        .nodes
        .iter()
        .find(|(_, n)| n.author_id() == Some("note"))
        .unwrap();
    assert_eq!(note.role(), Role::MultilineTextInput);
    let runs: Vec<_> = tree
        .nodes
        .iter()
        .filter(|(_, n)| n.role() == Role::TextRun)
        .collect();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].1.value(), Some("hello world"));
    assert_eq!(runs[0].1.character_lengths().len(), 11);
    let sel = note.text_selection().expect("a selection");
    assert_eq!(
        (sel.anchor.character_index, sel.focus.character_index),
        (0, 5)
    );
    let positions = runs[0].1.character_positions().unwrap_or_default();
    assert!(
        positions.windows(2).all(|w| w[1] > w[0]),
        "characters advance: {positions:?}"
    );
    // A reader moves the caret to the end of "world".
    let request = ActionRequest {
        action: Ak::SetTextSelection,
        target_tree: TreeId::ROOT,
        target_node: tree
            .nodes
            .iter()
            .find(|(_, n)| n.author_id() == Some("note"))
            .unwrap()
            .0,
        data: Some(ActionData::SetTextSelection(accesskit::TextSelection {
            anchor: accesskit::TextPosition {
                node: runs[0].0,
                character_index: 11,
            },
            focus: accesskit::TextPosition {
                node: runs[0].0,
                character_index: 11,
            },
        })),
    };
    assert!(rt.a11y_action(&request));
    rt.frame(0.0);
    assert_eq!(rt.state().edit.range(), (11, 11));
}

#[test]
fn secure_fields_never_expose_their_text() {
    let edit = TextEdit::new("hunter2").secure();
    let mut rt = Runtime::new(
        Field { edit },
        |s: &Field, _: &mut Cx<Field>| {
            col().child(
                text_input(&s.edit, crate::tok::type_::BODY)
                    .key("pw")
                    .single_line()
                    .label("Password")
                    .w(200.0)
                    .on_edit(|s: &mut Field| &mut s.edit),
            )
        },
        (400.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    let json = rt.accessibility_json().to_string();
    assert!(!json.contains("hunter2"), "{json}");
    let tree = rt.accessibility_json();
    let pw = find(&tree, "pw").unwrap();
    assert_eq!(
        (pw["role"].as_str(), pw["name"].as_str()),
        (Some("password-input"), Some("Password"))
    );
}

#[derive(Default)]
struct Linked {
    opened: Vec<String>,
}

#[test]
fn inline_links_are_their_own_nodes() {
    let mut rt = Runtime::new(
        Linked::default(),
        |_: &Linked, _: &mut Cx<Linked>| {
            col().child(
                crate::rich("Read the spec now.", vec![])
                    .key("para")
                    .w(300.0)
                    .link(crate::Link::new(9..13, "https://example.com/spec"))
                    .on_link(|s: &mut Linked, id, _| s.opened.push(id.to_owned())),
            )
        },
        (400.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    let para = find(&tree, "para").unwrap();
    assert_eq!(para["name"], "Read the spec now.");
    let link = find(&tree, "https://example.com/spec").unwrap();
    assert_eq!(
        (
            link["role"].as_str(),
            link["name"].as_str(),
            link["url"].as_str()
        ),
        (Some("link"), Some("spec"), Some("https://example.com/spec"))
    );
    let click = request(&rt, "https://example.com/spec", Ak::Click, None);
    assert!(rt.a11y_action(&click));
    rt.frame(0.0);
    assert_eq!(rt.state().opened, ["https://example.com/spec"]);
}

#[test]
fn modal_hides_the_background_and_live_regions_announce() {
    #[derive(Default)]
    struct M;
    let mut rt = Runtime::new(
        M,
        |_: &M, _: &mut Cx<M>| {
            col()
                .child(
                    row()
                        .key("behind")
                        .role("button")
                        .on_press(|_: &mut M| {})
                        .child(text("Behind")),
                )
                .child(
                    col()
                        .key("sheet")
                        .label("Confirm")
                        .modal(true, |_: &mut M| {})
                        .child(
                            text("Are you sure?")
                                .key("q")
                                .live_region(LiveRegion::Assertive),
                        ),
                )
        },
        (400.0, 300.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    assert!(find(&tree, "behind").is_none(), "behind a modal: {tree}");
    let sheet = find(&tree, "sheet").unwrap();
    assert_eq!(
        (sheet["role"].as_str(), sheet["name"].as_str()),
        (Some("dialog"), Some("Confirm"))
    );
    assert!(states(sheet).contains(&"modal"));
    assert!(
        states(find(&tree, "q").unwrap_or_else(|| panic!("{tree:#}"))).contains(&"live-assertive")
    );
}

struct Big {
    on: Vec<bool>,
}

fn big(s: &Big, _: &mut Cx<Big>) -> El {
    col()
        .key("big")
        .w_full()
        .h(600.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .children(s.on.iter().enumerate().map(|(i, on)| {
            row()
                .key(format!("r{i}"))
                .role("listitem")
                .h(24.0)
                .gap(8.0)
                .child(
                    row()
                        .key("check")
                        .role("checkbox")
                        .checked(*on)
                        .focusable(true)
                        .size(16.0, 16.0)
                        .on_press(move |s: &mut Big| s.on[i] = !s.on[i]),
                )
                .child(text(format!("Row {i}: the quick brown fox")))
        }))
}

/// What the tree costs: `cargo test -p rune --profile perf a11y_cost -- --ignored --nocapture`.
#[test]
#[ignore]
fn a11y_cost() {
    super::a11y_tree::CHECK_SENT.set(false);
    let median = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let mut rt = Runtime::new(
        Big {
            on: vec![false; 2000],
        },
        big,
        (800.0, 600.0),
        Env::new(true, 2.0),
    );
    rt.frame(0.0);
    let toggle = |rt: &mut Runtime<Big>, i: usize| {
        rt.update(|s, _| s.on[i] = !s.on[i]);
        rt.frame(0.0);
    };
    let time = |f: &mut dyn FnMut()| {
        let t = std::time::Instant::now();
        f();
        t.elapsed().as_secs_f64() * 1e6
    };
    let off: Vec<f64> = (0..30).map(|i| time(&mut || toggle(&mut rt, i))).collect();
    rt.a11y_activate("Big");
    let full: Vec<f64> = (0..10)
        .map(|_| {
            rt.a11y_activate("Big");
            time(&mut || {
                rt.a11y_update();
            })
        })
        .collect();
    let on: Vec<f64> = (0..30).map(|i| time(&mut || toggle(&mut rt, i))).collect();
    let incremental: Vec<f64> = (0..30)
        .map(|i| {
            toggle(&mut rt, i);
            time(&mut || {
                rt.a11y_update();
            })
        })
        .collect();
    let idle: Vec<f64> = (0..30)
        .map(|_| {
            time(&mut || {
                rt.a11y_update();
            })
        })
        .collect();
    rt.pointer_move(100.0, 100.0);
    let scroll: Vec<f64> = (1..=30)
        .map(|k| {
            rt.wheel_event(
                crate::WheelEvent::new(0.0, -24.0)
                    .precise(true)
                    .at(100.0, 100.0),
            );
            rt.frame(10.0 + k as f64 / 60.0);
            let commits = rt.commits;
            let mut sent = None;
            let t = time(&mut || {
                sent = rt.a11y_update().map(|u| u.nodes.len());
            });
            assert_eq!(rt.commits, commits);
            assert!(sent.is_some_and(|n| n > 0), "a scroll sends its viewports");
            t
        })
        .collect();
    let nodes = rt.accessibility_tree().nodes.len();
    println!(
        "a11y_cost elements={} nodes={nodes} commit_us={:.0} commit_active_us={:.0} full_us={:.0} incremental_us={:.0} scroll_us={:.1} idle_turn_us={:.2}",
        rt.nodes.len(),
        median(off),
        median(on),
        median(full),
        median(incremental),
        median(scroll),
        median(idle)
    );
}

#[test]
fn help_describes_unless_it_repeats_the_name_and_rows_point_at_their_active_descendant() {
    #[derive(Default)]
    struct H;
    let mut rt = Runtime::new(
        H,
        |_: &H, _: &mut Cx<H>| {
            col()
                .child(
                    row()
                        .key("save")
                        .role("button")
                        .label("Save")
                        .help("Save the file")
                        .on_press(|_: &mut H| {}),
                )
                .child(
                    row()
                        .key("trash")
                        .role("button")
                        .label("Delete")
                        .help("Moves it to the Trash")
                        .on_press(|_: &mut H| {}),
                )
                .child(
                    row()
                        .key("quiet")
                        .role("button")
                        .label("Quiet")
                        .help("Hush")
                        .description("")
                        .on_press(|_: &mut H| {}),
                )
                .child(
                    col()
                        .key("tree")
                        .role("tree")
                        .focusable(true)
                        .active_descendant("b")
                        .child(row().key("a").role("treeitem").child(text("Alpha")))
                        .child(
                            row()
                                .key("b")
                                .role("treeitem")
                                .focus_selected(true)
                                .child(text("Beta")),
                        ),
                )
                .child(
                    col().role("textbox").child(
                        text_input(&TextEdit::new(""), crate::tok::type_::BODY)
                            .key("field")
                            .label("Find"),
                    ),
                )
                .child(text("Bad").key("bad").invalid(true))
        },
        (400.0, 300.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    assert!(
        find(&tree, "save").unwrap()["description"].is_null(),
        "help that repeats the name: {tree}"
    );
    assert_eq!(
        find(&tree, "trash").unwrap()["description"],
        "Moves it to the Trash"
    );
    assert!(find(&tree, "quiet").unwrap()["description"].is_null());
    assert!(
        states(find(&tree, "b").unwrap()).contains(&"selected"),
        "a selected row outside a focus group: {tree}"
    );
    let whole = rt.accessibility_tree();
    let (_, t) = whole
        .nodes
        .iter()
        .find(|(_, n)| n.author_id() == Some("tree"))
        .unwrap();
    let (b, _) = whole
        .nodes
        .iter()
        .find(|(_, n)| n.author_id() == Some("b"))
        .unwrap();
    assert_eq!(t.active_descendant(), Some(*b));
    // The box around the field joins its parent; the field is the text box.
    let field_parent_is_root = tree["children"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["key"] == "field");
    assert!(field_parent_is_root, "{tree}");
    let (_, bad) = whole
        .nodes
        .iter()
        .find(|(_, n)| n.author_id() == Some("bad"))
        .unwrap();
    assert_eq!(bad.invalid(), Some(accesskit::Invalid::True));
}

fn nested(_: &(), _: &mut Cx<()>) -> El {
    let inner = col()
        .key("inner")
        .h(60.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(col().h(200.0))
        .child(text("Deep").key("deep").h(20.0));
    col()
        .key("outer")
        .h(200.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(col().h(600.0))
        .child(inner)
}

#[test]
fn scroll_into_view_reveals_through_every_scroll_view_around_the_element() {
    let mut rt = Runtime::new((), nested, (300.0, 200.0), Env::new(true, 1.0));
    for _ in 0..4 {
        rt.frame(0.0);
    }
    let shown = |rt: &Runtime<()>| {
        let e = rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("deep"))
            .unwrap();
        e.visible[3]
    };
    assert_eq!(shown(&rt), 0.0, "the row starts out of view");

    assert!(rt.accessibility_action("deep", Ak::ScrollIntoView, None));
    for _ in 0..4 {
        rt.frame(0.0);
    }

    assert_eq!(shown(&rt), 20.0, "the whole row shows in the window");
}

fn picker(_: &(), _: &mut Cx<()>) -> El {
    col()
        .key("roots")
        .role("listbox")
        .child(
            row()
                .role("option")
                .focusable(true)
                .on_press(|_: &mut ()| {})
                .child(text("Notebook")),
        )
        .child(
            row()
                .role("option")
                .focusable(true)
                .on_press(|_: &mut ()| {})
                .child(text("A")),
        )
}

#[test]
fn accessible_elements_name_an_unkeyed_element_by_the_role_and_name_the_tree_prints() {
    let mut rt = Runtime::new((), picker, (300.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);

    let option = rt
        .accessible_elements()
        .into_iter()
        .find(|e| e.role == "list-box-option" && e.name.as_deref() == Some("Notebook"))
        .expect("the option, as the tree names it");

    let element = rt
        .elements()
        .into_iter()
        .find(|e| e.path == option.path)
        .expect("its element");
    assert_eq!((element.role, element.key), ("option", None));
}

fn tall_section(_: &(), _: &mut Cx<()>) -> El {
    col()
        .h(200.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .key("section")
                .child(col().h(600.0))
                .child(row().role("button").h(20.0).child(text("Save"))),
        )
}

#[test]
fn an_action_at_a_path_scrolls_an_unkeyed_element_into_view_inside_a_keyed_section_taller_than_its_scroll_view()
 {
    let mut rt = Runtime::new((), tall_section, (300.0, 200.0), Env::new(true, 1.0));
    for _ in 0..4 {
        rt.frame(0.0);
    }
    let save = rt
        .accessible_elements()
        .into_iter()
        .find(|e| e.role == "button" && e.name.as_deref() == Some("Save"))
        .expect("the button")
        .path;
    let shown = |rt: &Runtime<()>| {
        rt.elements()
            .into_iter()
            .find(|e| e.path == save)
            .unwrap()
            .visible[3]
    };
    assert_eq!(shown(&rt), 0.0, "the button starts out of view");

    assert!(rt.accessibility_action_at(&save, Ak::ScrollIntoView, None));
    for _ in 0..4 {
        rt.frame(0.0);
    }

    assert_eq!(shown(&rt), 20.0, "the whole button shows");
}

#[test]
fn an_action_at_a_link_s_path_reaches_the_link() {
    let mut rt = Runtime::new(
        false,
        |_: &bool, _: &mut Cx<bool>| {
            text("Continue from the receipt.")
                .link(crate::Link::new(14..25, "s-receipt"))
                .on_link(|followed: &mut bool, _: &str, _: &mut crate::Tx| *followed = true)
        },
        (300.0, 40.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    let link = rt
        .accessible_elements()
        .into_iter()
        .find(|e| e.role == "link")
        .expect("the link");
    assert_eq!(link.name.as_deref(), Some("the receipt"));
    assert!(
        rt.elements().iter().any(|e| e.path == link.path),
        "{} is an element",
        link.path
    );

    assert!(rt.accessibility_action_at(&link.path, Ak::Click, None));
    rt.frame(0.016);
    assert!(*rt.state(), "the link was followed");
}

#[test]
fn contains_names_an_unkeyed_element_by_its_path() {
    let mut rt = Runtime::new((), tall_section, (300.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let spacer = rt
        .elements()
        .into_iter()
        .find(|e| e.key.is_none() && e.rect[3] == 600.0)
        .expect("the spacer");

    assert!(rt.contains(&spacer.path, 10.0, 10.0));
    assert!(
        !rt.contains(&spacer.path, 10.0, 250.0),
        "outside the window's clip"
    );
}

#[test]
fn an_element_that_holds_keys_is_the_focus_while_nothing_has_keyboard_focus() {
    struct S {
        doc: bool,
        field: TextEdit,
    }
    let mut rt = Runtime::new(
        S {
            doc: true,
            field: TextEdit::new(""),
        },
        |s: &S, _: &mut Cx<S>| {
            col()
                .child(
                    col()
                        .key("doc")
                        .role("document")
                        .label("Plan")
                        .holds_keys(s.doc)
                        .active_descendant("p2")
                        .child(text("One").key("p1"))
                        .child(text("Two").key("p2")),
                )
                .child(
                    text_input(&s.field, crate::tok::type_::BODY)
                        .key("field")
                        .label("Find")
                        .on_edit(|s: &mut S| &mut s.field),
                )
        },
        (400.0, 300.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    let focus = |rt: &mut Runtime<S>| {
        let id = rt.a11y_update().expect("the focus moved").focus;
        let tree = rt.accessibility_tree();
        tree.nodes
            .iter()
            .find(|(n, _)| *n == id)
            .and_then(|(_, n)| n.author_id().map(str::to_owned))
    };
    let tree = rt.accessibility_json();
    assert!(
        states(find(&tree, "doc").unwrap()).contains(&"focused"),
        "{tree}"
    );
    assert_eq!(find(&tree, "doc").unwrap()["activeDescendant"], "p2");
    rt.a11y_activate("Window");
    assert_eq!(focus(&mut rt).as_deref(), Some("doc"));

    rt.update(|s, _| s.field.focused = true);
    rt.frame(0.0);
    assert_eq!(
        focus(&mut rt).as_deref(),
        Some("field"),
        "keyboard focus comes first"
    );

    rt.update(|s, _| s.field.focused = false);
    rt.frame(0.0);
    assert_eq!(focus(&mut rt).as_deref(), Some("doc"));

    rt.update(|s, _| s.doc = false);
    rt.frame(0.0);
    assert_eq!(
        focus(&mut rt),
        None,
        "the window itself, once nothing holds keys"
    );
}

#[test]
fn an_editor_that_draws_its_own_text_is_a_text_area_with_lines_and_a_caret() {
    struct Doc {
        text: String,
        caret: usize,
    }
    let mut rt = Runtime::new(
        Doc {
            text: "# Plan\nship it\n".into(),
            caret: 2,
        },
        |s: &Doc, _: &mut Cx<Doc>| {
            col().child(
                col()
                    .key("editor")
                    .role("textbox")
                    .label("Note")
                    .focusable(true)
                    .a11y_text(s.text.as_str(), s.caret, s.caret, Vec::new())
                    .child(text(s.text.lines().next().unwrap_or("")).key("row")),
            )
        },
        (400.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    let editor = |rt: &Runtime<Doc>| {
        let tree = rt.accessibility_tree();
        let (_, n) = tree
            .nodes
            .iter()
            .find(|(_, n)| n.author_id() == Some("editor"))
            .expect("the editor is in the tree")
            .clone();
        let runs: Vec<(accesskit::NodeId, String, usize, Vec<u8>)> = n
            .children()
            .iter()
            .filter_map(|c| tree.nodes.iter().find(|(id, _)| id == c))
            .filter(|(_, r)| r.role() == Role::TextRun)
            .map(|(id, r)| {
                (
                    *id,
                    r.value().unwrap_or("").to_owned(),
                    r.character_lengths().len(),
                    r.word_starts().to_vec(),
                )
            })
            .collect();
        (n, runs)
    };
    let (node, runs) = editor(&rt);
    assert_eq!(node.role(), Role::MultilineTextInput);
    assert_eq!(node.label(), Some("Note"));
    assert_eq!(node.value(), Some("# Plan\nship it\n"));
    assert_eq!(
        runs.iter().map(|r| (r.1.as_str(), r.2)).collect::<Vec<_>>(),
        [("# Plan\n", 7), ("ship it\n", 8), ("", 0)]
    );
    assert_eq!(
        runs[1].3,
        [0, 5],
        "words start at each letter after a non-letter"
    );
    let at = |sel: &accesskit::TextSelection| (sel.focus.node, sel.focus.character_index);
    assert_eq!(at(node.text_selection().expect("a caret")), (runs[0].0, 2));

    rt.update(|s, _| s.caret = 12);
    rt.frame(0.0);
    let (node, runs) = editor(&rt);
    assert_eq!(
        at(node.text_selection().unwrap()),
        (runs[1].0, 5),
        "the caret moved to the second line"
    );

    rt.update(|s, _| {
        s.text.push('x');
        s.caret = s.text.len();
    });
    rt.frame(0.0);
    let (node, runs) = editor(&rt);
    assert_eq!(
        node.value(),
        Some("# Plan\nship it\nx"),
        "typing reaches the value"
    );
    assert_eq!(at(node.text_selection().unwrap()), (runs[2].0, 1));
}

/// The line and the word around the caret of the element keyed `key`, as a screen reader's
/// adapter reads them from the tree.
fn line_and_word_at_caret<S: 'static>(rt: &Runtime<S>, key: &str) -> (String, String) {
    let tree = accesskit_consumer::Tree::new(rt.accessibility_tree(), true);
    let state = tree.state();
    fn keyed<'a>(
        n: accesskit_consumer::NodeRef<'a>,
        key: &str,
    ) -> Option<accesskit_consumer::NodeRef<'a>> {
        if n.author_id() == Some(key) {
            return Some(n);
        }
        n.children().find_map(|c| keyed(c, key))
    }
    let node = keyed(state.root(), key).expect("keyed node");
    let caret = node.text_selection_focus().expect("a caret");
    let line = node
        .line_range_from_index(caret.to_line_index())
        .expect("the caret's line")
        .text();
    let mut word = node.document_range();
    word.set_end(caret.forward_to_word_end());
    word.set_start(caret.backward_to_word_start());
    (line, word.text().trim_end().to_owned())
}

#[test]
fn an_editor_that_draws_its_own_text_reads_each_wrapped_line_as_a_line() {
    struct Doc {
        caret: usize,
    }
    let mut rt = Runtime::new(
        Doc { caret: 2 },
        |s: &Doc, _: &mut Cx<Doc>| {
            col().child(
                col()
                    .key("editor")
                    .role("textbox")
                    .label("Note")
                    .focusable(true)
                    .a11y_text("alpha beta gamma\ndelta", s.caret, s.caret, vec![6, 11]),
            )
        },
        (400.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    assert_eq!(line_and_word_at_caret(&rt, "editor").0, "alpha ");
    rt.update(|s, _| s.caret = 8);
    rt.frame(0.0);
    assert_eq!(
        line_and_word_at_caret(&rt, "editor"),
        ("beta ".to_owned(), "beta".to_owned()),
        "the caret's drawn line, not its paragraph"
    );
    rt.update(|s, _| s.caret = 11);
    rt.frame(0.0);
    assert_eq!(
        line_and_word_at_caret(&rt, "editor").0,
        "gamma\n",
        "a caret at a wrap stands at the start of the line after it"
    );
}

#[test]
fn a_word_past_a_line_s_255th_character_is_read_as_that_word() {
    let words: Vec<String> = (0..100).map(|k| format!("w{k:03}")).collect();
    let line = words.join(" ");
    let caret = line.find("w070").unwrap() + 2;
    assert!(caret > 255);

    struct Doc {
        text: String,
        caret: usize,
    }
    let mut rt = Runtime::new(
        Doc {
            text: line.clone(),
            caret,
        },
        |s: &Doc, _: &mut Cx<Doc>| {
            col().child(
                col()
                    .key("editor")
                    .role("textbox")
                    .label("Note")
                    .focusable(true)
                    .a11y_text(s.text.as_str(), s.caret, s.caret, Vec::new()),
            )
        },
        (400.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    assert_eq!(
        line_and_word_at_caret(&rt, "editor"),
        (line.clone(), "w070".to_owned()),
        "drawn text"
    );

    let mut edit = TextEdit::new(line.as_str());
    edit.focused = true;
    edit.set_caret(caret);
    let mut rt = Runtime::new(
        Field { edit },
        |s: &Field, _: &mut Cx<Field>| {
            col().child(
                text_input(&s.edit, crate::tok::type_::BODY)
                    .key("note")
                    .single_line()
                    .w(4000.0)
                    .on_edit(|s: &mut Field| &mut s.edit),
            )
        },
        (4200.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    assert_eq!(
        line_and_word_at_caret(&rt, "note"),
        (line, "w070".to_owned()),
        "a field"
    );
}

#[test]
fn a_terminal_reads_its_screen_only_while_the_tree_is_read_and_again_when_it_says_it_changed() {
    struct Term {
        screen: String,
        reads: std::cell::Cell<u32>,
    }
    let mut rt = Runtime::new(
        Term {
            screen: "$ ".into(),
            reads: Default::default(),
        },
        |_: &Term, _: &mut Cx<Term>| {
            col().child(
                crate::el()
                    .key("term")
                    .role("terminal")
                    .label("Shell")
                    .holds_keys(true)
                    .size(200.0, 100.0)
                    .a11y_text_from(|s: &Term| {
                        s.reads.set(s.reads.get() + 1);
                        Some(crate::DrawnText {
                            text: s.screen.clone(),
                            anchor: s.screen.len(),
                            focus: s.screen.len(),
                            wraps: Vec::new(),
                        })
                    }),
            )
        },
        (400.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    assert_eq!(rt.state().reads.get(), 0, "no client, no screen text");

    rt.a11y_activate("Window");
    let full = rt.a11y_update().expect("first update after activation");
    let term = full
        .nodes
        .iter()
        .find(|(_, n)| n.author_id() == Some("term"))
        .map(|(_, n)| n.clone())
        .expect("the terminal is in the tree");
    assert_eq!(term.role(), Role::Terminal);
    assert_eq!(term.value(), Some("$ "));
    assert_eq!(
        full.nodes
            .iter()
            .find(|(id, _)| *id == full.focus)
            .and_then(|(_, n)| n.author_id()),
        Some("term"),
        "the terminal holds the keys"
    );

    let commits = rt.stats.commits;
    rt.state_mut().screen.push_str("echo hello\nhello\n$ ");
    rt.a11y_text_changed();
    rt.frame(1.0);
    assert_eq!(rt.stats.commits, commits, "output does not run the view");
    let update = rt.a11y_update().expect("the screen changed");
    let term = update
        .nodes
        .iter()
        .find(|(_, n)| n.author_id() == Some("term"))
        .map(|(_, n)| n.clone())
        .expect("the terminal is sent again");
    assert_eq!(term.value(), Some("$ echo hello\nhello\n$ "));
    let lines: Vec<&str> = update
        .nodes
        .iter()
        .filter(|(_, n)| n.role() == Role::TextRun)
        .filter_map(|(_, n)| n.value())
        .collect();
    assert_eq!(lines, ["$ echo hello\n", "hello\n", "$ "]);

    let reads = rt.state().reads.get();
    rt.frame(2.0);
    assert!(rt.a11y_update().is_none(), "nothing changed");
    assert_eq!(
        rt.state().reads.get(),
        reads,
        "an idle frame reads no screen text"
    );
}
