//! Chart axes and grid lines.
//!
//! Ported from `gpui-kit`'s `plot/axis` and `plot/grid` (Apache-2.0), adapted to
//! iced's canvas `Frame`.
//!
//! An axis owns the labels along one edge and the grid lines perpendicular to
//! it. Labels are placed before the plot area is drawn so the plot can reserve
//! room for them.

use crate::theme::Theme;
use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Font, Pixels, Point, Size};

/// The gap between an axis label and the plot area, in logical pixels.
pub const AXIS_GAP: f32 = 6.0;

/// The font size of an axis label.
pub const LABEL_SIZE: f32 = 10.0;

/// Which side of the plot an axis's labels sit on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AxisLabelSide {
    /// Labels to the left of the plot, reading right-aligned.
    #[default]
    Left,
    /// Labels below the plot, reading centred.
    Bottom,
    /// Labels to the right of the plot, reading left-aligned.
    Right,
}

impl AxisLabelSide {
    /// The horizontal alignment a label on this side should use.
    pub fn align_x(self) -> iced::advanced::text::Alignment {
        match self {
            Self::Left => iced::advanced::text::Alignment::Right,
            Self::Right => iced::advanced::text::Alignment::Left,
            Self::Bottom => iced::advanced::text::Alignment::Center,
        }
    }

    /// Whether labels on this side are offset horizontally from their anchor.
    pub fn is_vertical_axis(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// One label on an axis.
#[derive(Debug, Clone, PartialEq)]
#[must_use = "an AxisText does nothing unless it is given to a PlotAxis"]
pub struct AxisText {
    /// The text to draw.
    pub text: String,
    /// Where it sits, in plot-local coordinates.
    pub position: Point,
    /// Its color.
    pub color: Color,
    /// Its horizontal alignment relative to `position`.
    pub align: iced::advanced::text::Alignment,
}

impl AxisText {
    /// Creates a label.
    pub fn new(text: impl Into<String>, position: Point, color: Color) -> Self {
        Self {
            text: text.into(),
            position,
            color,
            align: iced::advanced::text::Alignment::Center,
        }
    }

    /// Sets the label's horizontal alignment.
    pub fn align(mut self, align: iced::advanced::text::Alignment) -> Self {
        self.align = align;
        self
    }
}

/// An axis: the labels along one edge, and the grid lines perpendicular to it.
#[derive(Debug, Clone, Default)]
#[must_use = "a PlotAxis does nothing unless it is drawn into a Frame"]
pub struct PlotAxis {
    labels: Vec<AxisText>,
    label_side: AxisLabelSide,
    grid: bool,
    label_visible: bool,
}

impl PlotAxis {
    /// Creates an axis with grid lines and labels enabled.
    pub fn new() -> Self {
        Self {
            labels: Vec::new(),
            label_side: AxisLabelSide::default(),
            grid: true,
            label_visible: true,
        }
    }

    /// Sets where the labels sit.
    pub fn label_side(mut self, side: AxisLabelSide) -> Self {
        self.label_side = side;
        self
    }

    /// Adds a label.
    pub fn label(mut self, text: AxisText) -> Self {
        self.labels.push(text);
        self
    }

    /// Adds several labels.
    pub fn labels(mut self, labels: impl IntoIterator<Item = AxisText>) -> Self {
        self.labels.extend(labels);
        self
    }

    /// Turns grid lines on or off.
    pub fn grid(mut self, grid: bool) -> Self {
        self.grid = grid;
        self
    }

    /// Turns labels on or off.
    pub fn labels_visible(mut self, visible: bool) -> Self {
        self.label_visible = visible;
        self
    }

    /// The labels this axis holds.
    pub fn texts(&self) -> &[AxisText] {
        &self.labels
    }

    /// Whether this axis draws grid lines.
    #[must_use]
    pub fn draws_grid(&self) -> bool {
        self.grid
    }

    /// The grid line positions, along the axis's own direction.
    #[must_use]
    pub fn grid_positions(&self) -> Vec<f32> {
        self.labels
            .iter()
            .map(|label| {
                if self.label_side.is_vertical_axis() {
                    label.position.y
                } else {
                    label.position.x
                }
            })
            .collect()
    }

    /// Draws this axis's labels into `frame`.
    pub fn draw_labels<Renderer>(&self, frame: &mut Frame<Renderer>, theme: &Theme)
    where
        Renderer: iced::advanced::graphics::geometry::Renderer
            + iced::advanced::text::Renderer<Font = Font>,
    {
        if !self.label_visible {
            return;
        }

        let _ = theme;

        for label in &self.labels {
            frame.fill_text(iced::widget::canvas::Text {
                content: label.text.clone(),
                position: label.position,
                color: label.color,
                size: Pixels(LABEL_SIZE),
                align_x: label.align,
                align_y: iced::alignment::Vertical::Center,
                font: Font::DEFAULT,
                ..iced::widget::canvas::Text::default()
            });
        }
    }

    /// Draws this axis's grid lines across `area`.
    pub fn draw_grid<Renderer: iced::advanced::graphics::geometry::Renderer>(
        &self,
        frame: &mut Frame<Renderer>,
        area: iced::Rectangle,
        color: Color,
        dashed: bool,
    ) {
        if !self.grid {
            return;
        }

        for position in self.grid_positions() {
            let (from, to) = if self.label_side.is_vertical_axis() {
                (
                    Point::new(area.x, position),
                    Point::new(area.x + area.width, position),
                )
            } else {
                (
                    Point::new(position, area.y),
                    Point::new(position, area.y + area.height),
                )
            };

            let stroke = Stroke::default().with_color(color).with_width(1.0);

            if dashed {
                draw_dashed_line(frame, from, to, stroke, 4.0, 4.0);
            } else {
                frame.stroke(&Path::line(from, to), stroke);
            }
        }
    }
}

/// Draws a dashed line by emitting alternating segments.
///
/// iced's `Stroke` carries a dash *pattern* but the tiny-skia backend does not
/// honour it for canvas paths, so the segments are emitted explicitly. That also
/// keeps the dash phase stable across frames, which a backend-driven dash does
/// not guarantee.
pub fn draw_dashed_line<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    from: Point,
    to: Point,
    stroke: Stroke<'_>,
    dash: f32,
    gap: f32,
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = (dx * dx + dy * dy).sqrt();

    if length <= f32::EPSILON || dash <= 0.0 || gap < 0.0 {
        return;
    }

    let ux = dx / length;
    let uy = dy / length;
    let mut travelled = 0.0;

    while travelled < length {
        let segment_end = (travelled + dash).min(length);

        frame.stroke(
            &Path::line(
                Point::new(from.x + ux * travelled, from.y + uy * travelled),
                Point::new(from.x + ux * segment_end, from.y + uy * segment_end),
            ),
            stroke,
        );

        travelled = segment_end + gap;
    }
}

/// Draws a plot area's border.
pub fn draw_axis_line<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    area: iced::Rectangle,
    color: Color,
) {
    // Only the two primary edges: a full box around the plot would double up
    // with the grid's outermost lines.
    let bottom = Path::line(
        Point::new(area.x, area.y + area.height),
        Point::new(area.x + area.width, area.y + area.height),
    );
    let left = Path::line(
        Point::new(area.x, area.y),
        Point::new(area.x, area.y + area.height),
    );

    let stroke = Stroke::default().with_color(color).with_width(1.0);

    frame.stroke(&bottom, stroke);
    frame.stroke(&left, stroke);
}

/// The space labels need along each edge, so the plot can be inset.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AxisInset {
    /// Space reserved on the left, for y-axis labels.
    pub left: f32,
    /// Space reserved at the bottom, for x-axis labels.
    pub bottom: f32,
}

impl AxisInset {
    /// Estimates the inset from the longest label, given a label size.
    ///
    /// The estimate is per-character rather than measured: iced measures text
    /// only during layout, which has already happened by the time a canvas
    /// program draws. Over-reserving slightly is safer than clipping.
    ///
    /// `has_x_labels` reserves a strip along the bottom independently of what
    /// the y-axis holds. An earlier version only set it while iterating a
    /// label, so passing a y-axis (whose labels are on the left) reserved no
    /// bottom space at all and the category labels were clipped away.
    #[must_use]
    pub fn from_labels(axis: &PlotAxis, label_size: f32, has_x_labels: bool) -> Self {
        let mut left = 0.0_f32;
        let mut bottom = 0.0_f32;

        for label in axis.texts() {
            if axis.label_side.is_vertical_axis() {
                // Roughly 0.6em per character for the default sans face.
                left = left.max(label.text.chars().count() as f32 * label_size * 0.6);
            }

            if !axis.label_side.is_vertical_axis() {
                bottom = bottom.max(label_size * 1.6);
            }
        }

        if has_x_labels {
            bottom = bottom.max(label_size * 1.6);
        }

        Self {
            left: if left > 0.0 { left + AXIS_GAP } else { 0.0 },
            bottom,
        }
    }

    /// Insets `area` by the reserved space.
    pub fn apply(self, area: iced::Rectangle) -> iced::Rectangle {
        iced::Rectangle {
            x: area.x + self.left,
            y: area.y,
            width: (area.width - self.left).max(1.0),
            height: (area.height - self.bottom).max(1.0),
        }
    }
}

/// Formats an axis value without trailing noise.
///
/// Large magnitudes are abbreviated and whole numbers lose their decimal point,
/// which is what keeps an axis readable at a glance.
#[must_use]
pub fn format_tick(value: f64) -> String {
    let magnitude = value.abs();

    if magnitude >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if magnitude >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if magnitude >= 1_000.0 {
        format!("{:.1}k", value / 1_000.0)
    } else if value.fract() == 0.0 || magnitude >= 10.0 {
        // Whole numbers, and anything large enough that decimals are noise.
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

/// Chooses `count` evenly spaced tick values across `min..=max`.
///
/// Always includes both ends, so the axis states its full extent.
#[must_use]
pub fn ticks(min: f64, max: f64, count: usize) -> Vec<f64> {
    let count = count.max(2);

    if !min.is_finite() || !max.is_finite() || (max - min).abs() < f64::EPSILON {
        return vec![min];
    }

    let step = (max - min) / (count - 1) as f64;

    (0..count).map(|index| min + step * index as f64).collect()
}

/// The size a text label needs, for callers laying out their own chrome.
#[must_use]
pub fn label_size() -> Size {
    Size::new(LABEL_SIZE * 0.6, LABEL_SIZE * 1.2)
}

#[cfg(test)]
mod tests {
    use super::{format_tick, label_size, ticks, AxisInset, AxisLabelSide, AxisText, PlotAxis};
    use iced::{Color, Point};

    fn axis_with(labels: &[(&str, f32, f32)]) -> PlotAxis {
        let mut axis = PlotAxis::new();

        for (text, x, y) in labels {
            axis = axis.label(AxisText::new(*text, Point::new(*x, *y), Color::BLACK));
        }

        axis
    }

    #[test]
    fn an_axis_holds_its_labels_in_order() {
        let axis = axis_with(&[("a", 0.0, 0.0), ("b", 0.0, 10.0)]);

        assert_eq!(axis.texts().len(), 2);
        assert_eq!(axis.texts()[0].text, "a");
        assert_eq!(axis.texts()[1].text, "b");
    }

    #[test]
    fn grid_positions_follow_the_axis_direction() {
        // A vertical axis (labels on the left) lays its grid lines out by y; a
        // horizontal one lays them out by x.
        let vertical =
            axis_with(&[("0", 0.0, 5.0), ("1", 0.0, 25.0)]).label_side(AxisLabelSide::Left);
        assert_eq!(vertical.grid_positions(), vec![5.0, 25.0]);

        let horizontal =
            axis_with(&[("0", 5.0, 0.0), ("1", 25.0, 0.0)]).label_side(AxisLabelSide::Bottom);
        assert_eq!(horizontal.grid_positions(), vec![5.0, 25.0]);

        let right = axis_with(&[("0", 0.0, 7.0)]).label_side(AxisLabelSide::Right);
        assert_eq!(right.grid_positions(), vec![7.0]);
    }

    #[test]
    fn an_axis_can_have_its_grid_and_labels_turned_off() {
        let axis = axis_with(&[("0", 0.0, 0.0)])
            .grid(false)
            .labels_visible(false);

        assert!(!axis.draws_grid());
        assert!(!axis.grid_positions().is_empty(), "labels are still held");
    }

    #[test]
    fn label_alignment_matches_the_side() {
        use iced::advanced::text::Alignment;

        assert_eq!(AxisLabelSide::Left.align_x(), Alignment::Right);
        assert_eq!(AxisLabelSide::Right.align_x(), Alignment::Left);
        assert_eq!(AxisLabelSide::Bottom.align_x(), Alignment::Center);
    }

    #[test]
    fn only_the_left_and_right_sides_are_vertical_axes() {
        assert!(AxisLabelSide::Left.is_vertical_axis());
        assert!(AxisLabelSide::Right.is_vertical_axis());
        assert!(!AxisLabelSide::Bottom.is_vertical_axis());
    }

    #[test]
    fn an_inset_reserves_room_for_the_widest_label() {
        let axis = axis_with(&[("1000", 0.0, 0.0), ("5", 0.0, 10.0)]);
        let inset = AxisInset::from_labels(&axis, 10.0, false);

        // Four characters at 10px is wider than one, so the wider label decides.
        assert!(inset.left > 10.0 * 0.6 * 3.0);
    }

    #[test]
    fn an_inset_reserves_bottom_space_for_x_labels_even_from_a_y_axis() {
        // The regression: passing a left-side axis must still reserve the strip
        // the category labels need, or they are drawn outside the plot area and
        // clipped away.
        let axis = axis_with(&[("100", 0.0, 0.0)]).label_side(AxisLabelSide::Left);
        let inset = AxisInset::from_labels(&axis, 10.0, true);

        assert!(inset.left > 0.0, "the y labels still need room");
        assert!(inset.bottom > 0.0, "the x labels need room too");
    }

    #[test]
    fn an_inset_omits_bottom_space_when_there_are_no_x_labels() {
        let axis = axis_with(&[("100", 0.0, 0.0)]).label_side(AxisLabelSide::Left);
        let inset = AxisInset::from_labels(&axis, 10.0, false);

        assert!(inset.left > 0.0);
        assert_eq!(inset.bottom, 0.0);
    }

    #[test]
    fn an_inset_with_no_labels_reserves_nothing() {
        let axis = PlotAxis::new();
        let inset = AxisInset::from_labels(&axis, 10.0, false);

        assert_eq!(inset.left, 0.0);
        assert_eq!(inset.bottom, 0.0);
    }

    #[test]
    fn an_inset_shrinks_the_plot_area_without_collapsing_it() {
        let area = iced::Rectangle {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };

        let inset = AxisInset {
            left: 30.0,
            bottom: 20.0,
        };
        let inset_area = inset.apply(area);

        assert_eq!(inset_area.x, 30.0);
        assert_eq!(inset_area.width, 70.0);
        assert_eq!(inset_area.height, 30.0);

        // An inset larger than the area must not produce a negative size.
        let huge = AxisInset {
            left: 500.0,
            bottom: 500.0,
        };
        let clamped = huge.apply(area);

        assert!(clamped.width >= 1.0);
        assert!(clamped.height >= 1.0);
    }

    #[test]
    fn tick_labels_abbreviate_by_magnitude() {
        assert_eq!(format_tick(0.0), "0");
        assert_eq!(format_tick(42.0), "42");
        assert_eq!(format_tick(1_500.0), "1.5k");
        assert_eq!(format_tick(2_500_000.0), "2.5M");
        assert_eq!(format_tick(3_000_000_000.0), "3.0B");
        assert_eq!(format_tick(-1_500.0), "-1.5k");
        assert_eq!(format_tick(0.25), "0.25");
    }

    #[test]
    fn ticks_include_both_ends() {
        let values = ticks(0.0, 100.0, 5);

        assert_eq!(values.len(), 5);
        assert_eq!(values[0], 0.0);
        assert_eq!(values[4], 100.0);
        assert_eq!(values[2], 50.0, "the middle tick is centred");
    }

    #[test]
    fn ticks_handle_a_degenerate_range() {
        // A flat series has no range to divide, so a single tick stands in.
        assert_eq!(ticks(5.0, 5.0, 5), vec![5.0]);

        for (min, max) in [(f64::NAN, 1.0), (0.0, f64::INFINITY)] {
            let values = ticks(min, max, 4);
            assert!(!values.is_empty(), "a non-finite range still yields a tick");
        }
    }

    #[test]
    fn a_single_tick_request_still_yields_two_ends() {
        // One tick would give an axis with no extent, which cannot be drawn.
        let values = ticks(0.0, 10.0, 1);
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn a_label_size_is_positive() {
        let size = label_size();
        assert!(size.width > 0.0);
        assert!(size.height > 0.0);
    }
}
