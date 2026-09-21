//! Chart tooltips and hover focus.
//!
//! Ported from `gpui-kit`'s `plot/tooltip` (Apache-2.0), adapted to iced's
//! canvas model.
//!
//! # How hover works here
//!
//! iced draws a canvas through a `Program` whose `update` receives every event
//! and whose `draw` receives the current cursor. That means a chart does not
//! need a separate hit-test pass: [`focus_at`] maps the cursor position onto a
//! datum during `draw`, and the same call drives both the highlight and the
//! tooltip.
//!
//! # Focus easing
//!
//! The focus value ramps rather than snapping, so a highlight fades in and out
//! instead of appearing. [`Focus`] carries that ramp; a chart stores it in its
//! canvas state and advances it once per redraw.

use crate::theme::{Size, Theme};
use iced::alignment::Horizontal;
use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Font, Padding, Pixels, Point, Rectangle};

/// The radius of the dot marking a hovered datum.
pub const HOVER_DOT_RADIUS: f32 = 4.0;

/// The radius of the halo behind a hovered dot, at full focus.
pub const HOVER_HALO_RADIUS: f32 = 10.0;

/// How much the focus advances per frame while easing in or out.
///
/// iced gives a canvas no clock, only redraws, so the ramp is per-frame rather
/// than per-millisecond. At 60fps this reaches full focus in about a sixth of a
/// second, which reads as immediate without being a jump.
pub const FOCUS_STEP: f32 = 0.1;

/// The hover focus of a chart: which datum is under the cursor, and how far the
/// highlight has faded in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Focus {
    target: Option<usize>,
    current: Option<usize>,
    /// How far the highlight has faded in, from 0 to 1.
    strength: f32,
}

impl Focus {
    /// Creates an unfocused state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The datum the cursor is over, if any.
    #[must_use]
    pub fn target(&self) -> Option<usize> {
        self.target
    }

    /// The datum currently highlighted, which lags the target while easing.
    #[must_use]
    pub fn current(&self) -> Option<usize> {
        self.current
    }

    /// How far the highlight has faded in, from 0 to 1.
    #[must_use]
    pub fn strength(&self) -> f32 {
        self.strength
    }

    /// Whether anything is highlighted at all.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.current.is_some() && self.strength > 0.0
    }

    /// Whether a redraw is still needed to finish easing.
    #[must_use]
    pub fn is_settling(&self) -> bool {
        let target_strength = if self.target.is_some() { 1.0 } else { 0.0 };

        (self.strength - target_strength).abs() > f32::EPSILON
            || (self.target.is_some() && self.current != self.target)
    }

    /// Sets which datum the cursor is over.
    pub fn set_target(&mut self, target: Option<usize>) {
        self.target = target;

        // Moving onto a datum from nothing starts from that datum rather than
        // sliding the highlight across from wherever it was, which would look
        // like the pointer travelling.
        if self.current.is_none() && target.is_some() {
            self.current = target;
        }

        if target.is_some() {
            self.current = target;
        }
    }

    /// Advances the ramp one frame.
    pub fn advance(&mut self) {
        let target_strength = if self.target.is_some() { 1.0 } else { 0.0 };

        if self.strength < target_strength {
            self.strength = (self.strength + FOCUS_STEP).min(1.0);
        } else if self.strength > target_strength {
            self.strength = (self.strength - FOCUS_STEP).max(0.0);

            // Once fully faded out, the remembered datum is released so the
            // next hover starts from nothing.
            if self.strength <= 0.0 {
                self.current = None;
            }
        }

        if let Some(target) = self.target {
            self.current = Some(target);
        }
    }
}

/// A tooltip's contents: a title and one or more labelled values.
#[derive(Debug, Clone, PartialEq)]
#[must_use = "a TooltipContent does nothing unless it is drawn"]
pub struct TooltipContent {
    /// The heading, usually the x-axis value.
    pub title: String,
    /// One row per series.
    pub rows: Vec<TooltipRow>,
}

impl TooltipContent {
    /// Creates a tooltip with a title and no rows.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    /// Adds a labelled value.
    pub fn row(mut self, label: impl Into<String>, value: impl Into<String>, color: Color) -> Self {
        self.rows.push(TooltipRow {
            label: label.into(),
            value: value.into(),
            color,
        });
        self
    }

    /// Whether the tooltip has anything to show.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_empty() && self.rows.is_empty()
    }
}

/// One row of a tooltip.
#[derive(Debug, Clone, PartialEq)]
pub struct TooltipRow {
    /// The series name.
    pub label: String,
    /// The formatted value.
    pub value: String,
    /// The series color, shown as a swatch.
    pub color: Color,
}

/// Draws a tooltip near `anchor`, kept inside `bounds`.
///
/// The box is flipped to the other side of the anchor when it would overflow,
/// which is what keeps a tooltip for a point near the right edge readable.
pub fn draw_tooltip<Renderer>(
    frame: &mut Frame<Renderer>,
    content: &TooltipContent,
    anchor: Point,
    bounds: Rectangle,
    theme: &Theme,
) where
    Renderer:
        iced::advanced::graphics::geometry::Renderer + iced::advanced::text::Renderer<Font = Font>,
{
    if content.is_empty() {
        return;
    }

    let colors = theme.colors();
    let text_style = Size::Sm.text();

    let padding = 8.0;
    let row_height = text_style.line_height + 4.0;
    let swatch = 8.0;

    // Width from the longest row, estimated per character because iced measures
    // text only during layout.
    let widest = content
        .rows
        .iter()
        .map(|row| row.label.chars().count() + row.value.chars().count() + 4)
        .chain(std::iter::once(content.title.chars().count()))
        .max()
        .unwrap_or(8);

    let char_width = text_style.size * 0.58;
    let width = (widest as f32 * char_width + padding * 2.0 + swatch + 6.0).clamp(80.0, 260.0);
    let height = padding * 2.0 + row_height * (content.rows.len() + 1) as f32;

    let offset = 12.0;

    let mut x = anchor.x + offset;
    let mut y = anchor.y - height - offset;

    // Flip rather than clip: a tooltip half outside the plot is unreadable.
    if x + width > bounds.x + bounds.width {
        x = anchor.x - width - offset;
    }
    if y < bounds.y {
        y = anchor.y + offset;
    }

    x = x.clamp(bounds.x, (bounds.x + bounds.width - width).max(bounds.x));
    y = y.clamp(bounds.y, (bounds.y + bounds.height - height).max(bounds.y));

    let box_rect = Rectangle {
        x,
        y,
        width,
        height,
    };

    // The surface, with a border so it reads as a layer above the plot.
    frame.fill_rectangle(
        Point::new(box_rect.x, box_rect.y),
        iced::Size::new(box_rect.width, box_rect.height),
        Color {
            a: 0.97,
            ..colors.surface
        },
    );
    frame.stroke(
        &Path::rectangle(
            Point::new(box_rect.x, box_rect.y),
            iced::Size::new(box_rect.width, box_rect.height),
        ),
        Stroke::default().with_color(colors.border).with_width(1.0),
    );

    let mut cursor_y = y + padding + row_height / 2.0;

    frame.fill_text(iced::widget::canvas::Text {
        content: content.title.clone(),
        position: Point::new(x + padding, cursor_y),
        color: colors.foreground,
        size: Pixels(text_style.size),
        align_x: Horizontal::Left.into(),
        align_y: iced::alignment::Vertical::Center,
        font: Font::DEFAULT,
        ..iced::widget::canvas::Text::default()
    });

    cursor_y += row_height;

    for row in &content.rows {
        frame.fill_rectangle(
            Point::new(x + padding, cursor_y - swatch / 2.0),
            iced::Size::new(swatch, swatch),
            row.color,
        );

        frame.fill_text(iced::widget::canvas::Text {
            content: row.label.clone(),
            position: Point::new(x + padding + swatch + 6.0, cursor_y),
            color: colors.muted_foreground,
            size: Pixels(text_style.size - 1.0),
            align_x: Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center,
            font: Font::DEFAULT,
            ..iced::widget::canvas::Text::default()
        });

        frame.fill_text(iced::widget::canvas::Text {
            content: row.value.clone(),
            position: Point::new(x + width - padding, cursor_y),
            color: colors.foreground,
            size: Pixels(text_style.size - 1.0),
            align_x: Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            font: Font::DEFAULT,
            ..iced::widget::canvas::Text::default()
        });

        cursor_y += row_height;
    }
}

/// The padding a tooltip reserves, exposed for callers sizing their own.
#[must_use]
pub fn tooltip_padding() -> Padding {
    Padding::new(8.0)
}

/// Draws the ring that grows out of a hovered dot.
pub fn draw_halo<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    center: Point,
    color: Color,
    focus: f32,
) {
    if focus <= 0.0 {
        return;
    }

    let radius = HOVER_HALO_RADIUS * focus.clamp(0.0, 1.0);

    frame.fill(
        &Path::circle(center, radius),
        Color {
            a: 0.25 * focus.clamp(0.0, 1.0),
            ..color
        },
    );
}

/// Draws a hovered datum's dot, with its halo.
pub fn draw_hover_dot<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    center: Point,
    color: Color,
    focus: f32,
    surface: Color,
) {
    if focus <= 0.0 {
        return;
    }

    draw_halo(frame, center, color, focus);

    // The dot itself is drawn in the surface colour with a colored ring, so it
    // stays visible against a line of the same colour.
    frame.fill(
        &Path::circle(center, HOVER_DOT_RADIUS),
        Color {
            a: focus,
            ..surface
        },
    );
    frame.stroke(
        &Path::circle(center, HOVER_DOT_RADIUS),
        Stroke::default()
            .with_color(Color { a: focus, ..color })
            .with_width(2.0),
    );
}

/// Maps a cursor position onto the nearest datum index along one axis.
///
/// Returns `None` when the cursor is outside `area`, which is what stops a
/// tooltip appearing while the pointer is over the axis labels.
#[must_use]
pub fn focus_at(
    cursor: Option<Point>,
    area: Rectangle,
    scale: impl Fn(f32) -> usize,
) -> Option<usize> {
    let position = cursor?;

    if !area.contains(position) {
        return None;
    }

    let along = if area.width >= area.height {
        position.x - area.x
    } else {
        position.y - area.y
    };

    Some(scale(along))
}

#[cfg(test)]
mod tests {
    use super::{
        focus_at, tooltip_padding, Focus, TooltipContent, FOCUS_STEP, HOVER_DOT_RADIUS,
        HOVER_HALO_RADIUS,
    };
    use iced::{Color, Point, Rectangle};

    fn area() -> Rectangle {
        Rectangle {
            x: 10.0,
            y: 20.0,
            width: 200.0,
            height: 100.0,
        }
    }

    #[test]
    fn a_fresh_focus_is_inactive() {
        let focus = Focus::new();

        assert_eq!(focus.target(), None);
        assert_eq!(focus.current(), None);
        assert_eq!(focus.strength(), 0.0);
        assert!(!focus.is_active());
    }

    #[test]
    fn setting_a_target_starts_the_ramp() {
        let mut focus = Focus::new();
        focus.set_target(Some(3));

        assert_eq!(focus.target(), Some(3));
        assert_eq!(focus.current(), Some(3));

        // The ramp has not advanced yet, so nothing is highlighted.
        assert!(!focus.is_active());
        assert!(focus.is_settling());

        for _ in 0..20 {
            focus.advance();
        }

        assert!((focus.strength() - 1.0).abs() < f32::EPSILON);
        assert!(focus.is_active());
        assert!(!focus.is_settling(), "the ramp has settled");
    }

    #[test]
    fn clearing_the_target_fades_out_and_releases_the_datum() {
        let mut focus = Focus::new();
        focus.set_target(Some(2));

        for _ in 0..20 {
            focus.advance();
        }
        assert!(focus.is_active());

        focus.set_target(None);
        assert_eq!(focus.target(), None);
        // The datum lingers while the highlight fades, so the fade has
        // something to draw.
        assert_eq!(focus.current(), Some(2));

        for _ in 0..20 {
            focus.advance();
        }

        assert_eq!(focus.current(), None, "the datum is released once faded");
        assert!(!focus.is_active());
    }

    #[test]
    fn moving_between_data_points_keeps_the_highlight_visible() {
        // Sliding along a line should not fade out and back in.
        let mut focus = Focus::new();
        focus.set_target(Some(1));

        for _ in 0..20 {
            focus.advance();
        }

        focus.set_target(Some(5));

        assert_eq!(focus.current(), Some(5));
        assert!(
            (focus.strength() - 1.0).abs() < f32::EPSILON,
            "the highlight stays at full strength while moving"
        );
    }

    /// Too large and the highlight jumps; too small and it lags the cursor.
    const _: () = assert!(FOCUS_STEP > 0.0 && FOCUS_STEP < 0.5);

    #[test]
    fn the_focus_strength_stays_within_range() {
        let mut focus = Focus::new();
        focus.set_target(Some(0));

        for _ in 0..100 {
            focus.advance();
        }
        assert!(focus.strength() <= 1.0);

        focus.set_target(None);
        for _ in 0..100 {
            focus.advance();
        }
        assert!(focus.strength() >= 0.0);
    }

    #[test]
    fn a_tooltip_takes_a_title_and_rows() {
        let tooltip = TooltipContent::new("January")
            .row("Read", "1.2k", Color::WHITE)
            .row("Write", "800", Color::BLACK);

        assert_eq!(tooltip.title, "January");
        assert_eq!(tooltip.rows.len(), 2);
        assert_eq!(tooltip.rows[0].label, "Read");
        assert_eq!(tooltip.rows[0].value, "1.2k");
        assert!(!tooltip.is_empty());
    }

    #[test]
    fn an_empty_tooltip_reports_itself() {
        // A title alone is content worth drawing, so it is not empty.
        assert!(!TooltipContent::new("January").is_empty());

        // An empty title with no rows is nothing to show.
        assert!(TooltipContent::new("").is_empty());

        let no_rows_no_title = TooltipContent {
            title: String::new(),
            rows: Vec::new(),
        };
        assert!(no_rows_no_title.is_empty());

        // A row without a title is still worth drawing.
        let row_only = TooltipContent::new("").row("Read", "1", Color::WHITE);
        assert!(!row_only.is_empty());
    }

    #[test]
    fn a_cursor_inside_the_area_maps_to_a_datum() {
        let inside = Point::new(60.0, 50.0);

        // 50px along the area maps to datum 2.
        let index = focus_at(Some(inside), area(), |along| (along / 25.0) as usize);
        assert_eq!(index, Some(2));
    }

    #[test]
    fn a_cursor_outside_the_area_yields_no_focus() {
        let outside = Point::new(0.0, 0.0);

        // A tooltip must not appear while the pointer is over the axis labels.
        assert_eq!(focus_at(Some(outside), area(), |_| 0), None);
        assert_eq!(focus_at(None, area(), |_| 0), None);
    }

    /// The halo is what draws the eye; the dot just marks the exact value.
    const _: () = assert!(HOVER_DOT_RADIUS < HOVER_HALO_RADIUS);

    #[test]
    fn tooltip_padding_is_positive() {
        let padding = tooltip_padding();
        assert!(padding.top > 0.0);
        assert!(padding.left > 0.0);
    }
}
