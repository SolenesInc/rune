//! [`MarkdownStyle`]: how markdown looks, and its builders.

use rune::{Icon, Spring};

use super::*;
use crate::palette::tint;

/// How markdown looks. [`MarkdownStyle::from_kit`] takes every colour and font from the kit's
/// tokens, [`MarkdownStyle::neutral`] needs no kit; each builder
/// overrides one thing.
#[derive(Clone, Debug)]
pub struct MarkdownStyle {
    pub(super) text: Color,
    pub(super) muted: Color,
    /// List items' text and markers (`None`: the body font).
    pub(super) list_font: Option<TextStyle>,
    /// List markers' colour (`None`: [`MarkdownStyle::muted`]).
    pub(super) list_marker: Option<Color>,
    /// Each heading level's colour, levels 1 to 3 and then 4 and deeper (`None`: the text
    /// colour for levels 1 to 3, [`MarkdownStyle::muted`] deeper).
    pub(super) heading_text: [Option<Color>; 4],
    pub(super) accent: Color,
    pub(super) link_underline: Color,
    pub(super) code_bg: Color,
    pub(super) code_block_fill: Color,
    pub(super) code_block_border: Color,
    pub(super) quote_rule: Color,
    pub(super) quote: QuoteLook,
    /// GFM alerts' colours: note, tip, important, warning, caution.
    pub(super) alerts: [Color; 5],
    pub(super) rule: Color,
    pub(super) body: TextStyle,
    /// Levels 1 to 3, then 4 and deeper.
    pub(super) headings: [TextStyle; 4],
    pub(super) code_block: TextStyle,
    /// Code blocks' text where the highlighter leaves it (the body text colour by default).
    pub(super) code_text: Option<Color>,
    /// Code blocks no highlighter colours (no language, or one it doesn't know): code_text by default.
    pub(super) plain_code_text: Option<Color>,
    /// Shell fences' own look (see [`MarkdownStyle::shell_code`]).
    pub(super) shell: Option<ShellLook>,
    pub(super) link_hover: Option<Color>,
    pub(super) list_outside: bool,
    /// Nested lists' indent: bullets, then numbers (`None`: [`MarkdownStyle::list`]'s).
    pub(super) list_nested: Option<[f32; 2]>,
    pub(super) table_align: bool,
    pub(super) inline_code_scale: f32,
    pub(super) inline_code_weight: f32,
    pub(super) inline_code_pad: f32,
    pub(super) code_block_pad: [f32; 2],
    pub(super) list_indent: f32,
    pub(super) list_gap: f32,
    pub(super) strong_weight: f32,
    pub(super) table_outline: Option<Color>,
    pub(super) table_rule: Color,
    pub(super) table_head_fill: Color,
    pub(super) table_head_text: Color,
    pub(super) table_head_rule: Option<Color>,
    pub(super) table_head_weight: Option<f32>,
    pub(super) table_min_column: f32,
    pub(super) table_tabular: bool,
    /// Cells' font before tabular figures and the header's weight (`None`: the body's).
    pub(super) table_font: Option<TextStyle>,
    /// Cell padding (x, y), and whether the first column has none on its left.
    pub(super) table_pad: [f32; 2],
    pub(super) table_flush_left: bool,
    pub(super) table_frame: Option<TableFrame>,
    pub(super) table_body: Option<TableBody>,
    /// Header cell padding (x, y) when it differs from the body's.
    pub(super) table_head_pad: Option<[f32; 2]>,
    pub(super) table_full_width: bool,
    /// Space above and below a table (collapsed with its neighbours' like CSS margins).
    pub(super) table_margin: [f32; 2],
    /// Space above and below a code block, a diff and a diagram (collapsed like `table_margin`).
    pub(super) code_margin: [f32; 2],
    /// A wide table's scroll bar thumb, at rest and under the pointer (`None`: no bar).
    pub(super) table_bar: Option<[Color; 2]>,
    /// A wide code block's scroll bar (`None`: no bar).
    pub(super) code_bar: Option<crate::ScrollBarStyle>,
    /// See [`MarkdownStyle::caret`].
    pub(super) caret: Option<Color>,
    /// The text column's width, when the app knows it (see [`MarkdownStyle::text_width`]).
    pub(super) text_width: Option<f32>,
    /// The widest a top-level prose block runs (see [`MarkdownStyle::prose_measure`]).
    pub(super) prose_measure: Option<f32>,
    /// How far a wide code block's sides fade while it scrolls sideways.
    pub(super) wide_fade: f32,
    /// The icon before a link to a web page: its size and gap (`None`: no icon).
    pub(super) link_icon: Option<[f32; 2]>,
    pub(super) diff_add: Color,
    pub(super) diff_remove: Color,
    pub(super) diff_add_text: Color,
    pub(super) diff_remove_text: Color,
    pub(super) diff_hues: [Color; 2],
    pub(super) diff_hunk: Color,
    pub(super) syntax: SyntaxTheme,
    pub(super) highlight: bool,
    pub(super) selection: Color,
    pub(super) highlight_bg: Color,
    pub(super) highlight_fg: Option<Color>,
    pub(super) highlight_ring: Option<(Color, f32)>,
    pub(super) highlight_current_bg: Color,
    pub(super) highlight_current_fg: Option<Color>,
    pub(super) highlight_current_ring: Option<(Color, f32)>,
    pub(super) task: TaskPaint,
    pub(crate) diagram: DiagramPaint,
    /// The width of rules, table rules and code blocks' outlines.
    pub(super) line: f32,
    pub(super) margins: Margins,
    /// Whether ordered lists' numbers set figures at one width.
    pub(super) list_tabular: bool,
    /// The room between an outside marker and its item's text (`None`: a space's width).
    pub(super) list_marker_gap: Option<f32>,
    /// Inline code's size in points, whatever text it sits in (`None`: `inline_code_scale` of it).
    pub(super) inline_code_size: Option<f32>,
    /// Inline code chip's room above and below the mono font's ascent and descent (`None`: the
    /// chip balances on the cap height).
    pub(super) inline_code_pad_y: Option<f32>,
    /// Table body cells' text (`None`: the body text colour).
    pub(super) table_text: Option<Color>,
}

/// The space above and below blocks (see [`MarkdownStyle::heading_margin`] and its siblings).
#[derive(Clone, Copy, Debug)]
pub(super) struct Margins {
    pub(super) para: f32,
    /// Levels 1 to 3, then 4 and deeper.
    pub(super) headings: [[f32; 2]; 4],
    pub(super) list: [f32; 2],
    pub(super) quote: [f32; 2],
    pub(super) rule: [f32; 2],
    /// Whether the gap under a heading is its bottom margin collapsed with the next block's
    /// top, as CSS's, rather than its bottom margin alone.
    pub(super) collapse_under_heading: bool,
}

/// Colours, icon and motion of a GFM task list checkbox (matching [`crate::Checkbox`]).
#[derive(Clone, Copy, Debug)]
pub(super) struct TaskPaint {
    pub(super) fill_on: Color,
    pub(super) border_on: Color,
    pub(super) border_off: Color,
    pub(super) fill_off: Color,
    pub(super) hover: Color,
    /// The overlay a checked box takes under the pointer, so it keeps its fill.
    pub(super) hover_on: Color,
    pub(super) focus_ring: Color,
    pub(super) mark: Color,
    pub(super) check: &'static Icon,
    pub(super) radius: f32,
    pub(super) motion: Spring,
    /// The box's side, its left edge in the list's marker column, and its top below the
    /// item's (`None`: centred on the first line).
    pub(super) size: f32,
    pub(super) left: f32,
    pub(super) top: Option<f32>,
    /// The outline's width, and the room from the box to the item's text (`None`: the rest of
    /// the marker column).
    pub(super) ring: f32,
    pub(super) gap: Option<f32>,
    /// A checked item's text colour, struck through (`None`: drawn as an open item's).
    pub(super) done: Option<Color>,
}

/// A block quote's rule and text (see [`MarkdownStyle::quote`]).
#[derive(Clone, Copy, Debug)]
pub(super) struct QuoteLook {
    pub(super) width: f32,
    pub(super) pad: f32,
    pub(super) text: Option<Color>,
    pub(super) italic: bool,
}

/// Shell fences' frame and text (see [`MarkdownStyle::shell_code`]).
#[derive(Clone, Copy, Debug, Hash)]
pub(super) struct ShellLook {
    pub(super) fill: Color,
    pub(super) border: Color,
    pub(super) text: Color,
}

/// A table's rounded frame (see [`MarkdownStyle::table_frame`]).
#[derive(Clone, Copy, Debug)]
pub(super) struct TableFrame {
    pub(super) fill: Color,
    pub(super) ring: Color,
    pub(super) pad: f32,
}

/// The card a table's body rows sit on (see [`MarkdownStyle::table_body`]).
#[derive(Clone, Debug)]
pub(super) struct TableBody {
    pub(super) fill: Color,
    pub(super) radius: f32,
    pub(super) shadows: Vec<rune::Shadow>,
}

/// Colours and sizes diagrams draw with (see [`MarkdownStyle::diagram`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DiagramPaint {
    pub text: Color,
    pub muted: Color,
    pub surface: Color,
    pub surface_border: Color,
    pub node_fill: Color,
    pub node_border: Color,
    pub line: Color,
    pub cluster_fill: Color,
    pub note_fill: Color,
    pub note_border: Color,
    pub font: TextStyle,
    pub max_width: f32,
    pub min_scale: f32,
    pub hover: Color,
    pub hover_neighbours: bool,
}

impl MarkdownStyle {
    pub fn from_kit(k: &Kit) -> MarkdownStyle {
        let c = &k.c;
        let body = TextStyle { line_height: 22.4, ..k.ty.body };
        let heading = |size, lh| TextStyle { size, weight: 600.0, line_height: lh, ..body };
        MarkdownStyle {
            text: c.text_base,
            muted: c.text_muted,
            list_font: None,
            list_marker: None,
            heading_text: [None; 4],
            accent: c.text_accent,
            link_underline: k.alpha(c.text_accent, 0.4),
            code_bg: k.alpha(c.text_base, 0.08),
            code_block_fill: c.layer_01,
            code_block_border: c.border_muted,
            quote_rule: c.layer_03,
            quote: QuoteLook { width: 2.0, pad: 12.0, text: None, italic: false },
            alerts: alert_colors(k.dark),
            rule: c.border_base,
            body,
            headings: [heading(17.0, 24.0), heading(15.0, 24.0), heading(13.0, 20.0), heading(13.0, 20.0)],
            code_block: TextStyle { size: 13.0, line_height: 20.0, ..k.ty.mono },
            code_text: None,
            plain_code_text: None,
            shell: None,
            link_hover: None,
            list_outside: false,
            list_nested: None,
            table_align: true,
            inline_code_scale: 0.9,
            inline_code_weight: 400.0,
            inline_code_pad: 3.5,
            code_block_pad: [12.0, 12.0],
            list_indent: 32.0,
            list_gap: 4.0,
            strong_weight: 670.0,
            table_outline: Some(c.border_muted),
            table_rule: c.border_muted,
            table_head_fill: c.layer_01,
            table_head_text: c.text_base,
            table_head_rule: None,
            table_head_weight: None,
            table_min_column: 120.0,
            table_tabular: true,
            table_font: None,
            table_pad: [10.0, 6.0],
            table_flush_left: false,
            table_frame: None,
            table_body: None,
            table_head_pad: None,
            table_full_width: false,
            table_margin: [12.0, 16.0],
            code_margin: [12.0, 24.0],
            table_bar: None,
            code_bar: None,
            caret: None,
            text_width: None,
            prose_measure: None,
            wide_fade: 12.0,
            link_icon: None,
            diff_add: tint(c.layer_01, c.success.icon, if k.dark { 0.11 } else { 0.14 }),
            diff_remove: tint(c.layer_01, c.danger.icon, if k.dark { 0.11 } else { 0.14 }),
            diff_add_text: c.text_base,
            diff_remove_text: c.text_base,
            diff_hues: [c.success.icon, c.danger.icon],
            diff_hunk: c.text_muted,
            syntax: SyntaxTheme::from_kit(k),
            highlight: true,
            selection: k.selection(),
            highlight_bg: if k.dark { k.alpha(c.warning.icon, 0.24) } else { rune::oklcha(0.86, 0.15, 85.0, 0.42) },
            highlight_fg: None,
            highlight_ring: None,
            highlight_current_bg: if k.dark { k.alpha(c.warning.icon, 0.40) } else { rune::oklcha(0.82, 0.17, 75.0, 0.58) },
            highlight_current_fg: None,
            highlight_current_ring: Some((if k.dark { c.warning.icon } else { rune::oklch(0.66, 0.16, 70.0) }, 1.0)),
            task: TaskPaint {
                fill_on: c.bg_accent,
                border_on: c.bg_accent,
                border_off: c.border_weak,
                fill_off: Color::TRANSPARENT,
                hover: c.surface_hover,
                hover_on: k.on(c.contrast_hover, c.bg_accent),
                focus_ring: c.border_active,
                mark: c.knob_on,
                check: k.glyph.check,
                radius: k.radius.sm,
                motion: k.motion.control,
                size: 16.0,
                left: 8.0,
                top: None,
                ring: 1.0,
                gap: None,
                done: None,
            },
            diagram: DiagramPaint {
                text: c.text_base,
                muted: c.text_muted,
                surface: c.bg_base,
                surface_border: c.border_muted,
                node_fill: c.layer_01,
                node_border: k.alpha(c.text_base, 0.2),
                line: k.alpha(c.text_base, 0.42),
                cluster_fill: k.alpha(c.text_base, if k.dark { 0.04 } else { 0.025 }),
                note_fill: c.warning.bg,
                note_border: c.warning.border,
                font: TextStyle { size: 13.0, line_height: 18.0, ..k.ty.body },
                max_width: 680.0,
                min_scale: 0.6,
                hover: k.alpha(c.text_base, 0.9),
                hover_neighbours: false,
            },
            line: 0.5,
            margins: Margins {
                para: 12.0,
                headings: [[28.0, 12.0], [24.0, 10.0], [20.0, 8.0], [20.0, 8.0]],
                list: [8.0, 12.0],
                quote: [16.0, 16.0],
                rule: [32.0, 32.0],
                collapse_under_heading: false,
            },
            list_tabular: false,
            list_marker_gap: None,
            inline_code_size: None,
            inline_code_pad_y: None,
            table_text: None,
        }
    }
    /// Rune's neutral light or dark look ([`Kit::neutral`]'s tokens), for apps without a kit.
    pub fn neutral(dark: bool) -> MarkdownStyle {
        MarkdownStyle::from_kit(&Kit::neutral(dark))
    }
    /// Body, heading (levels 1 to 3 and the muted level 4+) and emphasis colour.
    pub fn text(mut self, c: Color) -> Self {
        self.text = c;
        self
    }
    /// Level 4+ headings and list markers, unless [`Self::deep_heading`] or
    /// [`Self::list_marker`] gives them their own.
    pub fn muted(mut self, c: Color) -> Self {
        self.muted = c;
        self
    }
    /// List items' paragraphs and their markers, as a page that sets `li` apart from `p` draws
    /// them (the body font by default).
    pub fn list_font(mut self, f: TextStyle) -> Self {
        self.list_font = Some(f);
        self
    }
    /// List markers' bullets and numbers, as CSS's `::marker { color }` sets them.
    pub fn list_marker(mut self, c: Color) -> Self {
        self.list_marker = Some(c);
        self
    }
    /// Level 4 and deeper headings: [`Self::heading_text`] at level 4.
    pub fn deep_heading(self, c: Color) -> Self {
        self.heading_text(4, c)
    }
    /// The headings of `level` (1 to 4, where 4 is every deeper level too), as a page that
    /// colours `h1`, `h2` and `h3` apart draws them.
    pub fn heading_text(mut self, level: u8, c: Color) -> Self {
        self.heading_text[usize::from(level.clamp(1, 4)) - 1] = Some(c);
        self
    }
    /// Link text; the underline is the same colour at 40% unless set with [`Self::link_underline`].
    pub fn link(mut self, c: Color) -> Self {
        self.accent = c;
        self.link_underline = c.alpha(0.4);
        self
    }
    pub fn link_underline(mut self, c: Color) -> Self {
        self.link_underline = c;
        self
    }
    /// The chip behind inline code.
    pub fn inline_code_bg(mut self, c: Color) -> Self {
        self.code_bg = c;
        self
    }
    pub fn code_block(mut self, fill: Color, border: Color) -> Self {
        self.code_block_fill = fill;
        self.code_block_border = border;
        self
    }
    /// Room (x, y) inside a code block's frame; 12 by 12 by default.
    pub fn code_block_padding(mut self, x: f32, y: f32) -> Self {
        self.code_block_pad = [x, y];
        self
    }
    /// The column a list's markers sit centred in (32 by default), and the gap between its
    /// items (4).
    pub fn list(mut self, indent: f32, gap: f32) -> Self {
        self.list_indent = indent;
        self.list_gap = gap;
        self
    }
    /// GFM alerts' colours, for their rule and title: note, tip, important, warning, caution.
    pub fn alerts(mut self, colors: [Color; 5]) -> Self {
        self.alerts = colors;
        self
    }

    pub(super) fn alert(&self, alert: Alert) -> Color {
        self.alerts[alert as usize]
    }

    pub fn quote_rule(mut self, c: Color) -> Self {
        self.quote_rule = c;
        self
    }
    /// A block quote's rule, `width` points wide in `rule`, `pad` points before its text, and
    /// the text's colour (2, 12 and the body text colour by default).
    pub fn quote(mut self, rule: Color, width: f32, pad: f32, text: Color) -> Self {
        self.quote_rule = rule;
        self.quote = QuoteLook { width: width.max(0.0), pad: pad.max(0.0), text: Some(text), ..self.quote };
        self
    }
    /// Whether a block quote's text is italic, as CSS's `blockquote { font-style: italic }` (off
    /// by default).
    pub fn quote_italic(mut self, on: bool) -> Self {
        self.quote.italic = on;
        self
    }
    /// The colour of a link under the pointer, underlined in it (the link colour by default).
    pub fn link_hover(mut self, c: Color) -> Self {
        self.link_hover = Some(c);
        self
    }
    /// Code blocks' text where the highlighter leaves it uncoloured (the body text colour by
    /// default).
    pub fn code_block_text(mut self, c: Color) -> Self {
        self.code_text = Some(c);
        self
    }
    /// The text of plain-text code blocks: a fence with no language, or `text`, `txt`, `plain`
    /// or `plaintext` (`code_block_text`'s colour by default).
    pub fn plain_code_text(mut self, c: Color) -> Self {
        self.plain_code_text = Some(c);
        self
    }
    /// Fences in a shell language (`bash`, `sh`, `shell`, `zsh`, `fish`, `console`,
    /// `terminal`) on `fill` with a `border`, their text all in `text` with no syntax colours,
    /// as a web page that styles a command apart from code.
    pub fn shell_code(mut self, fill: Color, border: Color, text: Color) -> Self {
        self.shell = Some(ShellLook { fill, border, text });
        self
    }
    /// List markers end where the item's text starts, numbers and bullets alike, as CSS's
    /// `list-style-position: outside` puts them, rather than centred in the marker column.
    pub fn list_markers_outside(mut self, on: bool) -> Self {
        self.list_outside = on;
        self
    }
    /// The marker column of a list inside a list item: `bullets` for bulleted lists and
    /// `numbers` for numbered ones (the top level's [`Self::list`] indent by default).
    pub fn list_nested_indent(mut self, bullets: f32, numbers: f32) -> Self {
        self.list_nested = Some([bullets.max(0.0), numbers.max(0.0)]);
        self
    }
    /// Task list checkboxes: `size` points square with corner `radius`, their left edge `left`
    /// points into the marker column and their top `top` below the item's (`None`: centred on
    /// its first line). Checked, they fill `fill_on` with a `mark` in it; unchecked, they fill
    /// `fill_off` inside a `border_off` outline. 16 points, 8 in and centred by default.
    #[allow(clippy::too_many_arguments)]
    pub fn task_checkbox(mut self, size: f32, radius: f32, left: f32, top: Option<f32>, fill_on: Color, mark: Color, fill_off: Color, border_off: Color) -> Self {
        let t = &mut self.task;
        (t.size, t.radius, t.left, t.top) = (size.max(0.0), radius.max(0.0), left.max(0.0), top);
        (t.fill_on, t.border_on, t.mark, t.fill_off, t.border_off) = (fill_on, fill_on, mark, fill_off, border_off);
        t.hover_on = t.hover;
        self
    }
    /// A checked task checkbox: its fill, outline and mark, for a box that stays a ring with a
    /// check in it rather than filling ([`Self::task_checkbox`] gives the outline the fill).
    pub fn task_checked(mut self, fill: Color, border: Color, mark: Color) -> Self {
        (self.task.fill_on, self.task.border_on, self.task.mark) = (fill, border, mark);
        self.task.hover_on = self.task.hover;
        self
    }
    /// Task checkboxes' outline width (1 by default) and the room from the box to the item's
    /// text (by default the rest of the list's marker column, so the text lines up with other
    /// items').
    pub fn task_ring(mut self, width: f32, gap: f32) -> Self {
        (self.task.ring, self.task.gap) = (width.max(0.0), Some(gap.max(0.0)));
        self
    }
    /// A checked task's text in `text`, struck through, as a page that dims finished tasks
    /// draws it. Lists inside the item keep their own look.
    pub fn task_done(mut self, text: Color) -> Self {
        self.task.done = Some(text);
        self
    }
    pub fn rule(mut self, c: Color) -> Self {
        self.rule = c;
        self
    }
    /// The width of rules, table rules and code blocks' outlines: 0.5, a hairline, by default; 1
    /// for a web page's `1px` borders.
    pub fn lines(mut self, width: f32) -> Self {
        self.line = super::table::pts(width);
        self
    }
    /// The space below a paragraph (12 by default; none above).
    pub fn paragraph_margin(mut self, bottom: f32) -> Self {
        self.margins.para = super::table::pts(bottom);
        self
    }
    /// The space above and below a heading of `level` (1 to 4, where 4 is every deeper level
    /// too): 28 and 12, 24 and 10, then 20 and 8 by default.
    pub fn heading_margin(mut self, level: u8, top: f32, bottom: f32) -> Self {
        self.margins.headings[usize::from(level.clamp(1, 4)) - 1] = [super::table::pts(top), super::table::pts(bottom)];
        self
    }
    /// The space above and below a list (8 and 12 by default).
    pub fn list_margin(mut self, top: f32, bottom: f32) -> Self {
        self.margins.list = [super::table::pts(top), super::table::pts(bottom)];
        self
    }
    /// The space above and below a block quote or an alert (16 and 16 by default).
    pub fn quote_margin(mut self, top: f32, bottom: f32) -> Self {
        self.margins.quote = [super::table::pts(top), super::table::pts(bottom)];
        self
    }
    /// The space above and below a rule (32 and 32 by default).
    pub fn rule_margin(mut self, top: f32, bottom: f32) -> Self {
        self.margins.rule = [super::table::pts(top), super::table::pts(bottom)];
        self
    }
    /// Whether the gap under a heading collapses its bottom margin with the next block's top,
    /// the larger winning, as a page without `h2 + * { margin-top: 0 }` spaces it. Off by
    /// default: the heading's bottom margin alone sets the gap.
    pub fn heading_margins_collapse(mut self, on: bool) -> Self {
        self.margins.collapse_under_heading = on;
        self
    }
    /// Whether ordered lists' numbers set figures at one width (OpenType `tnum`), as CSS's
    /// `font-variant-numeric: tabular-nums` (off by default).
    pub fn list_tabular_figures(mut self, on: bool) -> Self {
        self.list_tabular = on;
        self
    }
    /// The room between a [`Self::list_markers_outside`] marker and its item's text, as a
    /// browser leaves it (a space's width by default).
    pub fn list_marker_gap(mut self, points: f32) -> Self {
        self.list_marker_gap = Some(super::table::pts(points));
        self
    }
    /// Inline code at `points`, whatever the size of the text around it, as a page that sets
    /// `code { font-size }` in `rem` (by default [`Self::inline_code_font`]'s multiple of it).
    pub fn inline_code_size(mut self, points: f32) -> Self {
        self.inline_code_size = (points.is_finite() && points > 0.0).then_some(points);
        self
    }
    /// Table body cells' text colour (the body text colour by default), as a page that sets
    /// `td { color }` apart from `p`.
    pub fn table_text(mut self, c: Color) -> Self {
        self.table_text = Some(c);
        self
    }
    /// Paragraphs, list items and list markers.
    pub fn body_font(mut self, f: TextStyle) -> Self {
        self.body = f;
        self
    }
    /// The font of heading `level` (1 to 4; level 5 and deeper share 4). Level 3 sets 4 too,
    /// so a style that sets levels 1 to 3 keeps 4 and deeper in level 3's font.
    pub fn heading_font(mut self, level: u8, f: TextStyle) -> Self {
        let i = (level.clamp(1, 4) - 1) as usize;
        self.headings[i] = f;
        if i == 2 {
            self.headings[3] = f;
        }
        self
    }
    /// Code blocks and diffs. Inline code is set in the app's mono slot, so keep this on
    /// `Family::Mono` for the two to match; for the platform's monospaced font (SF Mono) in
    /// both, name `ui-monospace` in the mono slot of `AppSpec::families`.
    pub fn code_block_font(mut self, f: TextStyle) -> Self {
        self.code_block = f;
        self
    }
    /// Inline code's size as a multiple of the surrounding text, and its weight (the family is
    /// always the mono slot's).
    pub fn inline_code_font(mut self, scale: f32, weight: f32) -> Self {
        self.inline_code_scale = scale;
        self.inline_code_weight = weight;
        self
    }
    /// Room in points between inline code and the edges of its chip. Up to twice a narrow
    /// space's width; what's left of it is the gap between the chip and the text around it.
    pub fn inline_code_pad(mut self, pad: f32) -> Self {
        self.inline_code_pad = pad;
        self
    }
    /// Room in points above and below inline code inside its chip, as CSS `code { padding }`
    /// sets it: the chip then runs from the mono font's ascent to its descent and this much
    /// further. Without it the chip balances on the cap height.
    pub fn inline_code_pad_y(mut self, pad: f32) -> Self {
        self.inline_code_pad_y = (pad.is_finite() && pad >= 0.0).then_some(pad);
        self
    }
    pub fn strong_weight(mut self, w: f32) -> Self {
        self.strong_weight = w;
        self
    }
    /// The width of the column the document lays out in, when the app knows it. With
    /// [`Markdown::scroll_wide`], tables then size their columns as a browser's automatic table
    /// layout does: in proportion to their content when it fits, between their narrowest and
    /// widest when it nearly does, and at their narrowest (each column's longest word), scrolling
    /// sideways, when even that is wider than the text. Lists and quotes take their indent off it.
    pub fn text_width(mut self, points: f32) -> Self {
        self.text_width = (points.is_finite() && points > 0.0).then_some(points);
        self
    }
    /// The widest a top-level paragraph, heading, list, quote or alert runs, centered in the
    /// column, as a web page's `max-width` and `margin-inline: auto` on those blocks keep a
    /// reading measure while tables, code, diagrams, front matter and rules take the whole
    /// column and a picture narrower than the column sits centered (none by default). A
    /// top-level list, quote or alert starts where the measure starts and runs to the column's
    /// end: its lines, code and diagrams keep the measure, and its pictures and tables run on to
    /// the column's end.
    pub fn prose_measure(mut self, points: f32) -> Self {
        self.prose_measure = (points.is_finite() && points > 0.0).then_some(points);
        self
    }
    /// A document still arriving ([`Markdown::streaming`]) ends in a caret of `color`: a bar
    /// after the last word of its last paragraph, heading, list item, quote or code line,
    /// which moves as words arrive and goes when the stream ends. It is never copied, and it
    /// doesn't fade in with the words. A document that ends in a table, a picture or a rule
    /// shows none. None by default.
    pub fn caret(mut self, color: Color) -> Self {
        self.caret = Some(color);
        self
    }
    /// A code block that scrolls sideways ([`Markdown::scroll_wide`]) gets a scroll bar in
    /// `style` along its bottom edge, as a web page's `pre { overflow-x: auto }` does: an overlay
    /// bar draws over the block's bottom padding, and a classic one (the system showing classic
    /// bars, or an [`ScrollBarStyle::always`](crate::ScrollBarStyle::always) bar) adds its strip
    /// under the padding while the code overflows ([`ScrollBarStyle::room`](crate::ScrollBarStyle::room)).
    /// None by default.
    pub fn code_scroll_bar(mut self, style: crate::ScrollBarStyle) -> Self {
        self.code_bar = Some(style);
        self
    }
    /// How far the sides of a code block that scrolls sideways ([`Markdown::scroll_wide`]) fade
    /// out, in points: 12 by default, 0 for a hard edge as a web page's `overflow: auto` clips.
    pub fn wide_fade(mut self, points: f32) -> Self {
        self.wide_fade = if points.is_finite() { points.max(0.0) } else { 0.0 };
        self
    }
    /// The space above and below a code block, a diff and a diagram (12 and 24 points by
    /// default). Like the other blocks' margins, it collapses with the space a neighbour wants,
    /// taking the larger.
    pub fn code_margin(mut self, top: f32, bottom: f32) -> Self {
        self.code_margin = [super::table::pts(top), super::table::pts(bottom)];
        self
    }
    /// The tints behind a diff's added and removed lines (full width).
    pub fn diff(mut self, added: Color, removed: Color) -> Self {
        self.diff_add = added;
        self.diff_remove = removed;
        self
    }
    /// The text colours of a diff's added and removed lines (the body text colour by default).
    pub fn diff_text(mut self, added: Color, removed: Color) -> Self {
        self.diff_add_text = added;
        self.diff_remove_text = removed;
        self
    }
    /// Tints added and removed lines' text `amount` (0 to 1) of the way toward the theme's green
    /// and red, from the text colour set so far; around 0.5 reads clearly in both themes.
    pub fn diff_text_tint(mut self, amount: f32) -> Self {
        let t = amount.clamp(0.0, 1.0);
        self.diff_add_text = tint(self.text, self.diff_hues[0], t);
        self.diff_remove_text = tint(self.text, self.diff_hues[1], t);
        self
    }
    /// The text colour of a diff's `@@` hunk headers and file header lines.
    pub fn diff_hunk(mut self, c: Color) -> Self {
        self.diff_hunk = c;
        self
    }
    /// The colour of one kind of token in highlighted code blocks.
    pub fn syntax(mut self, kind: Syntax, c: Color) -> Self {
        self.syntax = self.syntax.with(kind, c);
        self
    }
    /// Every token colour at once (by default [`SyntaxTheme::from_kit`]).
    pub fn syntax_theme(mut self, theme: SyntaxTheme) -> Self {
        self.syntax = theme;
        self
    }
    /// Whether code blocks are coloured (on by default): fences in a language the highlighter
    /// knows (see [`Markdown::highlight`]). Off, every code block draws in the text colour.
    pub fn syntax_highlight(mut self, on: bool) -> Self {
        self.highlight = on;
        self
    }
    /// The highlight of [`Markdown::selectable`] text (the kit's field selection by default).
    pub fn selection(mut self, c: Color) -> Self {
        self.selection = c;
        self
    }
    /// Diagram nodes (and sequence participants): their fill and outline, and the colour of
    /// edges, arrows and lifelines.
    pub fn diagram(mut self, node_fill: Color, node_border: Color, line: Color) -> Self {
        self.diagram.node_fill = node_fill;
        self.diagram.node_border = node_border;
        self.diagram.line = line;
        self
    }
    /// The card a diagram sits on, and the tint behind flowchart subgraphs. Keep `fill` opaque:
    /// translucent lines and outlines are flattened over it so they don't brighten where they
    /// overlap. For no card, pass the colour behind the diagram with a clear `border`.
    pub fn diagram_surface(mut self, fill: Color, border: Color, subgraph: Color) -> Self {
        self.diagram.surface = fill;
        self.diagram.surface_border = border;
        self.diagram.cluster_fill = subgraph;
        self
    }
    /// Sequence diagram notes.
    pub fn diagram_note(mut self, fill: Color, border: Color) -> Self {
        self.diagram.note_fill = fill;
        self.diagram.note_border = border;
        self
    }
    /// The font of diagram labels (13 points of the body face by default).
    pub fn diagram_font(mut self, f: TextStyle) -> Self {
        self.diagram.font = f;
        self
    }
    /// The widest a diagram draws, in points (680 by default). A left-to-right flowchart that
    /// would have to shrink below 78% to fit is laid out top-down instead, when that comes out
    /// narrower; anything still wider is drawn smaller, down to 60%.
    pub fn diagram_max_width(mut self, points: f32) -> Self {
        self.diagram.max_width = points.max(120.0);
        self
    }
    /// The least a diagram wider than [`Self::diagram_max_width`] shrinks to (0.6 by default).
    /// Past it, the card shows the middle and its sides fade out. `0.0` always fits the whole
    /// diagram in, however small that draws it.
    pub fn diagram_min_scale(mut self, k: f32) -> Self {
        self.diagram.min_scale = k.clamp(0.0, 1.0);
        self
    }
    /// What lights up while the pointer is over a flowchart or state diagram node: its edges and
    /// their labels in `color` (the text colour by default; transparent turns it off), and with
    /// `neighbours` its outline and the outlines of the nodes they reach. Pointing at an edge or
    /// its label lights that edge and its label.
    pub fn diagram_hover(mut self, color: Color, neighbours: bool) -> Self {
        self.diagram.hover = color;
        self.diagram.hover_neighbours = neighbours;
        self
    }
    /// Background fill of standard [`Markdown::highlights`] matches.
    pub fn highlight_bg(mut self, c: Color) -> Self {
        self.highlight_bg = c;
        self
    }
    /// Optional glyph foreground colour of standard [`Markdown::highlights`] matches.
    pub fn highlight_fg(mut self, c: Option<Color>) -> Self {
        self.highlight_fg = c;
        self
    }
    /// Optional inset border ring `(color, width_pt)` of standard [`Markdown::highlights`] matches.
    pub fn highlight_ring(mut self, ring: Option<(Color, f32)>) -> Self {
        self.highlight_ring = ring;
        self
    }
    /// Background fill of [`Highlight::current`] matches.
    pub fn highlight_current_bg(mut self, c: Color) -> Self {
        self.highlight_current_bg = c;
        self
    }
    /// Optional glyph foreground colour of [`Highlight::current`] matches.
    pub fn highlight_current_fg(mut self, c: Option<Color>) -> Self {
        self.highlight_current_fg = c;
        self
    }
    /// Optional inset border ring `(color, width_pt)` of [`Highlight::current`] matches.
    pub fn highlight_current_ring(mut self, ring: Option<(Color, f32)>) -> Self {
        self.highlight_current_ring = ring;
        self
    }
}

#[inline]
fn f32_bits(x: f32) -> u32 {
    if x == 0.0 { 0 } else { x.to_bits() }
}

impl std::hash::Hash for MarkdownStyle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let MarkdownStyle {
            text,
            muted,
            list_font,
            list_marker,
            heading_text,
            accent,
            link_underline,
            code_bg,
            code_block_fill,
            code_block_border,
            quote_rule,
            quote,
            alerts,
            rule,
            body,
            headings,
            code_block,
            code_text,
            plain_code_text,
            shell,
            link_hover,
            list_outside,
            list_nested,
            table_align,
            inline_code_scale,
            inline_code_weight,
            inline_code_pad,
            code_block_pad,
            list_indent,
            list_gap,
            strong_weight,
            table_outline,
            table_rule,
            table_head_fill,
            table_head_text,
            table_head_rule,
            table_head_weight,
            table_min_column,
            table_tabular,
            table_font,
            table_pad,
            table_flush_left,
            ref table_frame,
            ref table_body,
            table_head_pad,
            table_full_width,
            table_margin,
            code_margin,
            table_bar,
            code_bar,
            caret,
            text_width,
            prose_measure,
            wide_fade,
            link_icon,
            diff_add,
            diff_remove,
            diff_add_text,
            diff_remove_text,
            diff_hues,
            diff_hunk,
            syntax,
            highlight,
            selection,
            highlight_bg,
            highlight_fg,
            highlight_ring,
            highlight_current_bg,
            highlight_current_fg,
            highlight_current_ring,
            task,
            diagram,
            line,
            margins,
            list_tabular,
            list_marker_gap,
            inline_code_size,
            inline_code_pad_y,
            table_text,
        } = *self;
        (text, muted, list_marker, heading_text, accent, link_underline, code_bg, code_block_fill, code_block_border, quote_rule, rule).hash(state);
        (body, list_font, headings, code_block, (code_text, plain_code_text), shell, link_hover, list_outside, list_nested.map(|n| n.map(f32_bits)), table_align).hash(state);
        (f32_bits(quote.width), f32_bits(quote.pad), quote.text, quote.italic, alerts).hash(state);
        let Margins { para, headings: heading_margins, list, quote: quote_margin, rule: rule_margin, collapse_under_heading } = margins;
        (f32_bits(line), f32_bits(para), heading_margins.map(|m| m.map(f32_bits)), [list, quote_margin, rule_margin].map(|m| m.map(f32_bits)), collapse_under_heading).hash(state);
        (list_tabular, list_marker_gap.map(f32_bits), inline_code_size.map(f32_bits), inline_code_pad_y.map(f32_bits), table_text).hash(state);
        [inline_code_scale, inline_code_weight, inline_code_pad, code_block_pad[0], code_block_pad[1], list_indent, list_gap, strong_weight, table_min_column]
            .map(f32_bits)
            .hash(state);
        (table_outline, table_rule, table_head_fill, table_head_text, table_head_rule, table_head_weight.map(f32_bits), table_tabular, table_font).hash(state);
        ([f32_bits(table_pad[0]), f32_bits(table_pad[1])], table_flush_left, table_head_pad.map(|p| p.map(f32_bits)), table_full_width, table_margin.map(f32_bits), code_margin.map(f32_bits)).hash(state);
        table_frame.map(|TableFrame { fill, ring, pad }| (fill, ring, f32_bits(pad))).hash(state);
        code_bar.map(|bar| format!("{bar:?}")).hash(state);
        caret.hash(state);
        (table_bar, text_width.map(f32_bits), prose_measure.map(f32_bits), f32_bits(wide_fade), link_icon.map(|l| l.map(f32_bits))).hash(state);
        match table_body {
            None => 0u8.hash(state),
            Some(TableBody { fill, radius, shadows }) => {
                1u8.hash(state);
                (fill, f32_bits(*radius), shadows.len()).hash(state);
                for rune::Shadow { x, y, blur, spread, color, inset } in shadows {
                    ([f32_bits(*x), f32_bits(*y), f32_bits(*blur), f32_bits(*spread)], color, inset).hash(state);
                }
            }
        }
        (diff_add, diff_remove, diff_add_text, diff_remove_text, diff_hues, diff_hunk, syntax, highlight, selection).hash(state);
        (
            highlight_bg,
            highlight_fg,
            highlight_ring.map(|(c, w)| (c, f32_bits(w))),
            highlight_current_bg,
            highlight_current_fg,
            highlight_current_ring.map(|(c, w)| (c, f32_bits(w))),
        )
            .hash(state);
        let TaskPaint { fill_on, border_on, border_off, fill_off, hover, hover_on, focus_ring, mark, check, radius, motion, size, left, top, ring, gap, done } = task;
        (fill_on, border_on, border_off, fill_off, hover, hover_on, focus_ring, mark, check as *const Icon as usize, f32_bits(radius), f32_bits(motion.duration), f32_bits(motion.bounce)).hash(state);
        (f32_bits(size), f32_bits(left), top.map(f32_bits), f32_bits(ring), gap.map(f32_bits), done).hash(state);
        let DiagramPaint {
            text,
            muted,
            surface,
            surface_border,
            node_fill,
            node_border,
            line,
            cluster_fill,
            note_fill,
            note_border,
            font,
            max_width,
            min_scale,
            hover,
            hover_neighbours,
        } = diagram;
        (text, muted, surface, surface_border, node_fill, node_border, line, cluster_fill, note_fill, note_border).hash(state);
        (font, f32_bits(max_width), f32_bits(min_scale), hover, hover_neighbours).hash(state);
    }
}

/// GitHub's alert colours: note, tip, important, warning, caution.
fn alert_colors(dark: bool) -> [Color; 5] {
    let hexes = if dark {
        ["#4493f8", "#3fb950", "#ab7df8", "#d29922", "#f85149"]
    } else {
        ["#0969da", "#1a7f37", "#8250df", "#9a6700", "#cf222e"]
    };
    hexes.map(|h| rune::color::hex(h).unwrap_or_default())
}
