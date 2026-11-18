//! TextArea (`@opencode/ui` forms/textarea), with a grip that drags it taller.

use rune::{El, TextEdit, TextStyle, Tx, col, el, icon, text_input};

use crate::text_input::{EditActs, kit_rich_style, name_field, rich_forwarders, shell};
use crate::{Kit, State};

/// A multi-line field (`Textarea`): 80 points tall at least, 13/1.35 type, 8 points of padding,
/// each of which the app may set.
/// Return breaks the line, as in a `<textarea>`; Cmd/Ctrl+Return calls [`TextArea::on_submit`].
pub struct TextArea {
    edit: TextEdit,
    key: String,
    label: Option<String>,
    placeholder: String,
    placeholder_ink: Option<rune::Color>,
    width: Option<f32>,
    rows: usize,
    grow: Option<(usize, usize)>,
    max_height: Option<f32>,
    min_height: Option<f32>,
    font: Option<TextStyle>,
    padding: (f32, f32),
    invalid: bool,
    state: State,
    edits: EditActs,
    resizable: bool,
    resize: Option<ResizeDrag>,
}

/// A `<textarea>`'s `min-height` in the kit's stylesheet.
const TEXTAREA_MIN_HEIGHT: f32 = 80.0;

/// The grip's drag, given the height the field shows now, when known, and its least height.
type ResizeDrag = Box<dyn FnOnce(El, Option<f32>, f32) -> El>;

/// Drags the field's [`TextEdit::dragged_height`] from where the press found it, never below
/// `least`; a cancelled drag puts back where it started.
fn resize_drag<S: 'static>(lens: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static) -> ResizeDrag {
    Box::new(move |grip: El, shown: Option<f32>, least: f32| {
        grip.on_drag(move |s: &mut S, d: &mut rune::Drag, _: &mut Tx| {
            let edit = lens(s);
            match d.phase {
                rune::Phase::Down => d.data = edit.dragged_height().or(shown).unwrap_or(least),
                rune::Phase::Move => edit.set_dragged_height(Some((d.data + d.dy).max(least).round())),
                rune::Phase::Up if d.cancelled() => edit.set_dragged_height(Some(d.data)),
                rune::Phase::Up => {}
            }
        })
    })
}

fn shared_lens<S, F: for<'a> Fn(&'a mut S) -> &'a mut TextEdit>(f: std::rc::Rc<F>) -> impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit {
    move |s| f(s)
}

impl TextArea {
    /// The key of the editor inside, `"input"` unless set: what `Tx::focused` reports while it has
    /// focus and what `Tx::focus` takes, so an app with several fields tells them apart.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }
    /// Protect the model like a password field. Hidden non-empty values use a fixed one-line
    /// mask. Clicks and unshifted navigation move to the end; shifted navigation selects all.
    /// Typing/paste append, or replace the whole value if selected; Backspace deletes at the
    /// end. Reveal to edit an interior position. Enter inserts a newline; Cmd/Ctrl+Enter submits.
    pub fn secure(mut self) -> Self {
        self.edit.set_secure(true);
        self
    }
    /// Show the text without enabling undo, copy, cut, IME or accessibility text.
    /// Toggling preserves the model selection until a hidden edit or navigation occurs.
    pub fn revealed(mut self, revealed: bool) -> Self {
        self.edit.set_revealed(revealed);
        self.edits.visual(move |e| e.revealed(revealed));
        self
    }
    /// Mark misspelled words and offer their suggestions in the field's menu (`El::spellcheck`).
    pub fn spellcheck(mut self, on: bool) -> Self {
        self.edits.visual(move |e| e.spellcheck(on));
        self
    }
    pub fn new(edit: &TextEdit) -> TextArea {
        TextArea {
            key: "input".into(),
            edit: edit.clone(),
            label: None,
            placeholder: String::new(),
            placeholder_ink: None,
            width: Some(280.0),
            rows: 3,
            grow: None,
            max_height: None,
            min_height: None,
            font: None,
            padding: (8.0, 8.0),
            invalid: false,
            state: State::Rest,
            edits: EditActs::default(),
            resizable: false,
            resize: None,
        }
    }
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }
    /// The placeholder's ink, the palette's `text_faint` unless set; an invalid field's stays red.
    pub fn placeholder_ink(mut self, ink: rune::Color) -> Self {
        self.placeholder_ink = Some(ink);
        self
    }
    /// A grip in the bottom-right corner drags the field taller or back to its least height
    /// (`resize: vertical`). The dragged height lives in the field's [`TextEdit`]
    /// ([`TextEdit::dragged_height`]) and fixes the box, scrolling longer text inside; the grip
    /// shows once [`TextArea::on_edit`] gives the kit the edit to keep it in.
    pub fn resizable(mut self) -> Self {
        self.resizable = true;
        self
    }
    /// The accessible name, as [`TextInput::label`].
    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }
    pub fn width(mut self, w: Option<f32>) -> Self {
        self.width = w;
        self
    }
    pub fn rows(mut self, r: usize) -> Self {
        self.rows = r.max(1);
        self
    }
    /// Fit the text: at least `min_rows` lines tall, growing with each wrapped line up to
    /// `max_rows`, then scrolling inside with the caret kept in view. The height springs on
    /// the kit's control motion.
    pub fn auto_grow(mut self, min_rows: usize, max_rows: usize) -> Self {
        let min = min_rows.max(1);
        self.grow = Some((min, max_rows.max(min)));
        self
    }
    /// Cap the field's height and scroll overflowing text. With [`TextArea::auto_grow`],
    /// also stop growing at this many points of text.
    pub fn max_height(mut self, h: f32) -> Self {
        self.max_height = Some(h);
        self
    }
    /// The box's least height in points (`min-height`), where its rows ask for less: 80 unless
    /// set, and none for an [`TextArea::auto_grow`] area. A [`TextArea::resizable`] drag stops
    /// there too.
    pub fn min_height(mut self, h: f32) -> Self {
        self.min_height = Some(h);
        self
    }
    /// The text's type, 13/1.35 of the kit's UI face unless set; rows are its lines.
    pub fn font(mut self, style: TextStyle) -> Self {
        self.font = Some(style);
        self
    }
    /// The space between the box's edge and its text: `x` on the sides, `y` above and below.
    pub fn padding(mut self, x: f32, y: f32) -> Self {
        self.padding = (x, y);
        self
    }
    pub fn animate_edit_layout(mut self) -> Self {
        self.edits.handlers.push(Box::new(El::animate_edit_layout));
        self
    }
    /// See [`El::wrap_anywhere`](rune::El::wrap_anywhere).
    pub fn wrap_anywhere(mut self) -> Self {
        self.edits.visual(El::wrap_anywhere);
        self
    }
    pub fn invalid(mut self, i: bool) -> Self {
        self.invalid = i;
        self
    }
    pub fn state(mut self, s: State) -> Self {
        self.state = s;
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        if d {
            self.state = State::Disabled;
        }
        self
    }
    pub fn on_edit<S: 'static>(mut self, f: impl for<'a> Fn(&'a mut S) -> &'a mut TextEdit + 'static) -> Self {
        let lens = std::rc::Rc::new(f);
        self.edits.edit(shared_lens(lens.clone()));
        self.resize = Some(resize_drag(shared_lens(lens)));
        self
    }
    pub fn on_submit<S: 'static>(mut self, f: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.edits.submit(f);
        self
    }
    pub fn on_key<S: 'static>(mut self, f: impl Fn(&mut S, &rune::KeyEvent, &mut Tx) -> bool + 'static) -> Self {
        self.edits.key(f);
        self
    }
    pub fn on_change<S: 'static>(mut self, f: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.edits.change(f);
        self
    }
    /// Focus left the field ([`El::on_blur`]).
    pub fn on_blur<S: 'static>(mut self, f: impl Fn(&mut S, &mut Tx) + 'static) -> Self {
        self.edits.blur(f);
        self
    }
    rich_forwarders!();

    pub fn build(self, k: &Kit) -> El {
        let c = &k.c;
        let disabled = self.state == State::Disabled;
        let style = self.font.unwrap_or(TextStyle { line_height: (13.0f32 * 1.35 * 2.0).round() / 2.0, ..k.ty.ui });
        let (pad_x, pad_y) = self.padding;
        let inset = 2.0 * pad_y;
        let least = self.least_height(style.line_px());
        let stylesheet_min_height = self.stylesheet_min_height();
        let red = self.invalid && !disabled && k.invalid_text;
        let ink = if red { c.danger.fg } else { c.text_base };
        let hint = if red { c.danger.fg } else { self.placeholder_ink.unwrap_or(c.text_faint) };
        let field = text_input(&self.edit, style)
            .key(self.key.clone())
            .w_full()
            .color(ink)
            .rich_style(kit_rich_style(k))
            .placeholder(self.placeholder.clone(), hint)
            .caret(ink, k.selection())
            .return_breaks_line();
        let field = name_field(field, self.label.as_deref(), &self.placeholder, None, disabled);
        let field = self.edits.apply(field, disabled);
        let line = style.line_px();
        let mut e = col().role("textbox").px(pad_x).py(pad_y);
        let dragged = self.edit.dragged_height().filter(|_| self.resizable);
        e = if let Some(h) = dragged {
            let viewport = col()
                .key("scroll")
                .w_full()
                .grow(1.0)
                .shrink()
                .min_h(0.0)
                .scroll(rune::ScrollSpec { stick_bottom: false })
                .child(field.animate_edit_layout());
            e.h(h.max(least)).child(viewport)
        } else if let Some((min_rows, max_rows)) = self.grow {
            let viewport = rune::auto_grow(field, min_rows, max_rows);
            e.child(match self.max_height {
                Some(h) => viewport.max_h(h.min(max_rows as f32 * line).max(line)),
                None => viewport,
            })
        } else if let Some(h) = self.max_height {
            let max_inner = (h - inset).max(line);
            let min_inner = (self.rows as f32 * line).max(stylesheet_min_height - inset).min(max_inner);
            let viewport = col()
                .key("scroll")
                .w_full()
                .shrink()
                .min_h(min_inner)
                .max_h(max_inner)
                .scroll(rune::ScrollSpec { stick_bottom: false })
                .child(field.animate_edit_layout());
            e.min_h(min_inner + inset).child(viewport)
        } else {
            e.child(field)
        };
        e = match self.width {
            Some(w) => e.w(w),
            None => e.w_full(),
        };
        e = e.min_h(least);
        if let (true, false, Some(drag)) = (self.resizable, disabled, self.resize) {
            let box_key = format!("{}-resizable", self.key);
            let shown = if dragged.is_none() { rune::memo::layout_size(&box_key).map(|[_, h]| h) } else { None };
            let grip = el()
                .key(format!("{}-grip", self.key))
                .size(12.0, 12.0)
                .pin(None, None, Some(2.0), Some(2.0))
                .center()
                .cursor(rune::Cursor::ResizeRow)
                .child(icon(k.glyph.resize_grip, 10.0).color(c.icon_muted));
            e = e.key(box_key).child(drag(grip, shown, least));
        }
        shell(k, e, self.state, self.invalid && !disabled, self.edit.focused && !disabled)
    }

    /// The box's least height: what it shows with no text, which a drag never goes below.
    fn least_height(&self, line: f32) -> f32 {
        let inset = 2.0 * self.padding.1;
        match (self.grow, self.max_height) {
            (Some((min_rows, _)), _) => (min_rows as f32 * line + inset).max(self.min_height.unwrap_or(0.0)),
            (None, Some(h)) => {
                (self.rows as f32 * line).max(self.stylesheet_min_height() - inset).min((h - inset).max(line)) + inset
            }
            (None, None) => (self.rows as f32 * line + inset).max(self.stylesheet_min_height()),
        }
    }

    fn stylesheet_min_height(&self) -> f32 {
        self.min_height.unwrap_or(TEXTAREA_MIN_HEIGHT)
    }
}
