//! Pie charts.
//!
//! Ported from `gpui-kit`'s `PieChart` (Apache-2.0), rebuilt on iced's canvas.
//!
//! # Donut mode
//!
//! An inner radius turns the pie into a donut. The hole is not only decorative:
//! it gives the labels somewhere to sit, and a thin ring is easier to compare by
//! arc length than a full pie is by angle.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use crate::widgets::plot::series_color;
use crate::widgets::plot::shape;
use crate::widgets::plot::tooltip::{draw_tooltip, TooltipContent};
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{mouse, Color, Element, Font, Length, Pixels, Point, Rectangle};

/// The fraction of the outer radius a hovered slice lifts by.
const HOVER_LIFT: f32 = 0.06;

/// How much the un-hovered slices fade, from 0 (no fade) to 1 (invisible).
const HOVER_DIM: f32 = 0.55;

/// One slice of a pie chart.
#[derive(Debug, Clone)]
#[must_use = "a Slice does nothing unless it is given to a chart"]
pub struct Slice {
    label: String,
    value: f64,
    tone: Option<Tone>,
    color: Option<Color>,
}

impl Slice {
    /// Creates a slice with a label and a value.
    pub fn new(label: impl Into<String>, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            tone: None,
            color: None,
        }
    }

    /// Sets the slice's color tone.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Sets an explicit color, overriding the tone.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The slice's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The slice's value.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }
}

/// A pie chart.
#[must_use = "a PieChart does nothing unless it is turned into an Element"]
pub struct PieChart {
    slices: Vec<Slice>,
    inner_radius_ratio: f32,
    outer_radius_ratio: f32,
    pad_angle: f32,
    show_labels: bool,
    show_percentages: bool,
    show_tooltip: bool,
    start_angle: f32,
    height: f32,
}

impl PieChart {
    /// Creates a pie chart from its slices.
    pub fn new(slices: Vec<Slice>) -> Self {
        Self {
            slices,
            // No hole by default; `donut(true)` or `inner_radius` opens one.
            inner_radius_ratio: 0.0,
            // Leaves room for the outer labels, which sit outside the ring.
            outer_radius_ratio: 0.72,
            // A small gap between slices is what makes neighbouring colours read
            // as separate slices rather than one band.
            pad_angle: 0.015,
            show_labels: true,
            show_percentages: true,
            show_tooltip: true,
            // Twelve o'clock, read clockwise.
            start_angle: 0.0,
            height: 240.0,
        }
    }

    /// Turns the pie into a donut with a hole of `ratio` of the radius.
    pub fn donut(mut self, donut: bool) -> Self {
        self.inner_radius_ratio = if donut { 0.58 } else { 0.0 };
        self
    }

    /// Sets the inner radius as a fraction of the outer radius.
    pub fn inner_radius(mut self, ratio: f32) -> Self {
        self.inner_radius_ratio = ratio.clamp(0.0, 0.95);
        self
    }

    /// Sets the outer radius as a fraction of the available half-extent.
    pub fn outer_radius(mut self, ratio: f32) -> Self {
        self.outer_radius_ratio = ratio.clamp(0.1, 1.0);
        self
    }

    /// Sets the gap between slices, in radians.
    pub fn pad_angle(mut self, radians: f32) -> Self {
        self.pad_angle = radians.clamp(0.0, 0.5);
        self
    }

    /// Turns the slice labels on or off.
    pub fn labels(mut self, visible: bool) -> Self {
        self.show_labels = visible;
        self
    }

    /// Turns the percentages appended to labels on or off.
    pub fn percentages(mut self, visible: bool) -> Self {
        self.show_percentages = visible;
        self
    }

    /// Turns the hover tooltip on or off.
    pub fn tooltip(mut self, tooltip: bool) -> Self {
        self.show_tooltip = tooltip;
        self
    }

    /// Sets the angle the first slice starts at, in radians clockwise from
    /// twelve o'clock.
    pub fn start_angle(mut self, radians: f32) -> Self {
        self.start_angle = radians;
        self
    }

    /// Sets the chart's height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(1.0);
        self
    }

    /// Moves the builder into the canvas program that draws it.
    fn into_program(self) -> PieChartProgram {
        PieChartProgram {
            slices: self.slices,
            inner_radius_ratio: self.inner_radius_ratio,
            outer_radius_ratio: self.outer_radius_ratio,
            pad_angle: self.pad_angle,
            show_labels: self.show_labels,
            show_percentages: self.show_percentages,
            show_tooltip: self.show_tooltip,
            start_angle: self.start_angle,
        }
    }

    /// Converts the chart into an [`Element`].
    #[must_use]
    pub fn into_element<Message: 'static>(self) -> Element<'static, Message, Theme> {
        let height = self.height;

        Canvas::new(self.into_program())
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .into()
    }
}

impl<Message: 'static> From<PieChart> for Element<'static, Message, Theme> {
    fn from(chart: PieChart) -> Self {
        chart.into_element()
    }
}

/// One slice's resolved geometry, for hit-testing and drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SliceArc {
    start: f32,
    end: f32,
    fraction: f64,
}

/// The canvas program that draws a pie chart.
///
/// Mirrors the builder's display toggles.
#[allow(clippy::struct_excessive_bools)]
struct PieChartProgram {
    slices: Vec<Slice>,
    inner_radius_ratio: f32,
    outer_radius_ratio: f32,
    pad_angle: f32,
    show_labels: bool,
    show_percentages: bool,
    show_tooltip: bool,
    start_angle: f32,
}

impl PieChartProgram {
    /// The total of every finite, positive slice value.
    ///
    /// Negative values are ignored rather than subtracted: a slice of negative
    /// size has no meaning on a pie, and silently shrinking the whole would
    /// misreport every other slice's share.
    fn total(&self) -> f64 {
        self.slices
            .iter()
            .map(|slice| slice.value)
            .filter(|value| value.is_finite() && *value > 0.0)
            .sum()
    }

    /// Resolves each slice's angular span, in radians clockwise from the start.
    fn arcs(&self) -> Vec<SliceArc> {
        let total = self.total();

        if total <= 0.0 {
            return Vec::new();
        }

        let mut cursor = self.start_angle;
        let mut arcs = Vec::with_capacity(self.slices.len());

        for slice in &self.slices {
            let value = if slice.value.is_finite() && slice.value > 0.0 {
                slice.value
            } else {
                0.0
            };

            let sweep = std::f32::consts::TAU * (value / total) as f32;
            // The gap is taken off the end so slices stay adjacent at the start.
            let padded = (sweep - self.pad_angle).max(0.0);

            arcs.push(SliceArc {
                start: cursor,
                end: cursor + padded,
                fraction: value / total,
            });

            cursor += sweep;
        }

        arcs
    }

    /// The geometry shared by drawing and hit-testing.
    fn geometry(&self, bounds: Rectangle) -> Option<PieGeometry> {
        let total = self.total();

        if total <= 0.0 {
            return None;
        }

        // A pie is square by nature, so the radius comes from the smaller
        // dimension; the rest is centring.
        let half_extent = (bounds.width.min(bounds.height) / 2.0).max(1.0);
        let outer = half_extent * self.outer_radius_ratio;
        let inner = outer * self.inner_radius_ratio;

        Some(PieGeometry {
            center: Point::new(bounds.width / 2.0, bounds.height / 2.0),
            inner,
            outer,
            arcs: self.arcs(),
        })
    }

    /// The color a slice is drawn in.
    fn color_of(&self, index: usize, theme: &Theme) -> Color {
        let slice = &self.slices[index];

        slice.color.unwrap_or_else(|| match slice.tone {
            Some(tone) => tone.accent(theme),
            None => series_color(theme, index),
        })
    }

    /// Builds the tooltip for a hovered slice.
    fn tooltip_content(&self, index: usize, theme: &Theme) -> TooltipContent {
        let slice = &self.slices[index];
        let total = self.total();
        let share = if total > 0.0 {
            slice.value / total * 100.0
        } else {
            0.0
        };

        TooltipContent::new(slice.label.clone())
            .row(
                "Value",
                format_share(slice.value),
                self.color_of(index, theme),
            )
            .row(
                "Share",
                format!("{share:.1}%"),
                theme.colors().muted_foreground,
            )
    }
}

/// Resolved pie geometry: the centre, the two radii, and the slice spans.
struct PieGeometry {
    center: Point,
    inner: f32,
    outer: f32,
    arcs: Vec<SliceArc>,
}

impl PieGeometry {
    /// The slice containing an angle, measured from the chart's start angle.
    fn slice_at(&self, angle: f32) -> Option<usize> {
        // Wrapping keeps a negative angle (above twelve o'clock) comparable.
        let normalized = angle.rem_euclid(std::f32::consts::TAU);

        self.arcs.iter().position(|arc| {
            let start = arc.start.rem_euclid(std::f32::consts::TAU);
            let end = arc.end.rem_euclid(std::f32::consts::TAU);

            // An arc that wraps past twelve o'clock spans two ranges.
            if start <= end {
                normalized >= start && normalized < end
            } else {
                normalized >= start || normalized < end
            }
        })
    }
}

impl<Message, Renderer> canvas::Program<Message, Theme, Renderer> for PieChartProgram
where
    Renderer:
        iced::advanced::graphics::geometry::Renderer + iced::advanced::text::Renderer<Font = Font>,
{
    type State = Option<usize>;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if let iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) = event {
            let target = self.hit_test(bounds, cursor);

            if *state != target {
                *state = target;
                return Some(canvas::Action::request_redraw());
            }
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        let kit_theme: &crate::theme::Theme = theme;
        let colors = kit_theme.colors();

        let Some(geometry) = self.geometry(bounds) else {
            return vec![frame.into_geometry()];
        };

        let hovered = *state;

        for (index, arc) in geometry.arcs.iter().enumerate() {
            if arc.end <= arc.start {
                continue;
            }

            let color = self.color_of(index, kit_theme);
            let is_hovered = hovered == Some(index);

            // The hovered slice lifts out of the ring and the rest fade behind
            // it, which is what makes the hover readable without a legend.
            let (outer, color) = if hovered.is_some() {
                if is_hovered {
                    (geometry.outer * (1.0 + HOVER_LIFT), color)
                } else {
                    (
                        geometry.outer,
                        Color {
                            a: 1.0 - HOVER_DIM,
                            ..color
                        },
                    )
                }
            } else {
                (geometry.outer, color)
            };

            shape::draw_wedge(
                &mut frame,
                geometry.center,
                geometry.inner,
                outer,
                arc.start,
                arc.end,
                color,
            );
        }

        // Labels sit outside the ring, at each slice's mid-angle.
        if self.show_labels {
            for (index, arc) in geometry.arcs.iter().enumerate() {
                if arc.end <= arc.start {
                    continue;
                }

                let mid = f32::midpoint(arc.start, arc.end);
                let is_hovered = hovered == Some(index);
                let alpha = if hovered.is_some() && !is_hovered {
                    1.0 - HOVER_DIM
                } else {
                    1.0
                };

                let radius = geometry.outer + 16.0;
                let anchor = Point::new(
                    geometry.center.x + radius * mid.sin(),
                    geometry.center.y - radius * mid.cos(),
                );

                // Labels on the left half read right-aligned so they grow away
                // from the chart rather than over it.
                let on_left = anchor.x < geometry.center.x;
                let align = if on_left {
                    iced::alignment::Horizontal::Right
                } else {
                    iced::alignment::Horizontal::Left
                };

                let label = if self.show_percentages {
                    format!("{} {:.0}%", self.slices[index].label, arc.fraction * 100.0)
                } else {
                    self.slices[index].label.clone()
                };

                frame.fill_text(canvas::Text {
                    content: label,
                    position: Point::new(anchor.x + if on_left { -4.0 } else { 4.0 }, anchor.y),
                    color: Color {
                        a: alpha,
                        ..colors.foreground
                    },
                    size: Pixels(11.0),
                    align_x: align.into(),
                    align_y: iced::alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }

        // The share in the middle of a donut, which is where the eye lands.
        if geometry.inner > 0.0 {
            let total = self.total();

            if total > 0.0 {
                let content = match hovered {
                    Some(index) => format!("{:.0}%", geometry.arcs[index].fraction * 100.0),
                    None => format_share(total),
                };

                frame.fill_text(canvas::Text {
                    content,
                    position: geometry.center,
                    color: colors.foreground,
                    size: Pixels(if hovered.is_some() { 22.0 } else { 18.0 }),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }

        // The tooltip.
        if let Some(index) = hovered {
            let content = self.tooltip_content(index, kit_theme);
            let arc = geometry.arcs[index];
            let mid = f32::midpoint(arc.start, arc.end);
            let radius = geometry.outer * 0.75;

            let anchor = Point::new(
                geometry.center.x + radius * mid.sin(),
                geometry.center.y - radius * mid.cos(),
            );

            draw_tooltip(
                &mut frame,
                &content,
                anchor,
                Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: bounds.width,
                    height: bounds.height,
                },
                kit_theme,
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.show_tooltip && cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

impl PieChartProgram {
    /// Which slice the cursor is over, if any.
    fn hit_test(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        if !self.show_tooltip {
            return None;
        }

        let position = cursor.position_over(bounds)?;
        let geometry = self.geometry(bounds)?;

        let dx = position.x - geometry.center.x;
        let dy = position.y - geometry.center.y;
        let distance = (dx * dx + dy * dy).sqrt();

        // Outside the ring, or inside the hole, is not over any slice.
        if distance > geometry.outer || distance < geometry.inner {
            return None;
        }

        // `atan2` measures counter-clockwise from three o'clock; the slices are
        // measured clockwise from twelve, so the angle is rotated.
        let angle = dx.atan2(-dy);

        geometry.slice_at(angle)
    }
}

/// Formats a value for a tooltip without trailing noise.
fn format_share(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

/// The fraction of the outer radius a sliced label sits at, exposed so callers
/// can align their own annotations with the built-in ones.
#[must_use]
pub fn label_radius_fraction() -> f32 {
    0.72
}

#[cfg(test)]
mod tests {
    use super::{PieChart, Slice};
    use crate::theme::Theme;
    use crate::widgets::display::Tone;
    use iced::Color;

    /// Only the type matters; no handler is ever invoked in these tests.
    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    fn slices() -> Vec<Slice> {
        vec![
            Slice::new("Rust", 45.0),
            Slice::new("Go", 25.0),
            Slice::new("Python", 20.0),
            Slice::new("Other", 10.0),
        ]
    }

    fn program_of(chart: PieChart) -> super::PieChartProgram {
        chart.into_program()
    }

    #[test]
    fn a_pie_chart_renders() {
        let element: iced::Element<'_, Message, Theme> = PieChart::new(slices()).into_element();
        drop(element);
    }

    #[test]
    fn a_donut_renders() {
        let chart = PieChart::new(slices()).donut(true);
        assert!(chart.inner_radius_ratio > 0.0);

        let element: iced::Element<'_, Message, Theme> = chart.into_element();
        drop(element);
    }

    #[test]
    fn the_slice_angles_are_proportional_to_the_values() {
        let program = program_of(PieChart::new(slices()));
        let arcs = program.arcs();

        assert_eq!(arcs.len(), 4);

        // 45 of 100 is 45% of a full turn; the small pad angle is taken off.
        let expected = std::f32::consts::TAU * 0.45 - program.pad_angle;
        assert!((arcs[0].end - arcs[0].start - expected).abs() < 0.001);
    }

    #[test]
    fn the_slices_cover_a_full_turn() {
        let program = program_of(PieChart::new(slices()));
        let arcs = program.arcs();

        let total: f32 = arcs.iter().map(|arc| arc.end - arc.start).sum();
        let expected = std::f32::consts::TAU - program.pad_angle * arcs.len() as f32;

        assert!(
            (total - expected).abs() < 0.01,
            "the arcs plus the gaps must cover the circle: {total} vs {expected}"
        );
    }

    #[test]
    fn slices_are_adjacent_at_their_start() {
        // A gap is taken off the end of each slice, so consecutive slices start
        // where the previous one would have ended without a gap.
        let program = program_of(PieChart::new(slices()));
        let arcs = program.arcs();

        let first_sweep = std::f32::consts::TAU * 0.45;
        assert!((arcs[1].start - first_sweep).abs() < 0.001);
    }

    #[test]
    fn the_fractions_sum_to_one() {
        let program = program_of(PieChart::new(slices()));
        let sum: f64 = program.arcs().iter().map(|arc| arc.fraction).sum();

        assert!((sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn zero_and_negative_values_are_dropped() {
        // A negative slice has no meaning on a pie, and silently shrinking the
        // whole would misreport every other slice's share.
        let program = program_of(PieChart::new(vec![
            Slice::new("Good", 50.0),
            Slice::new("Zero", 0.0),
            Slice::new("Negative", -30.0),
            Slice::new("Also good", 50.0),
        ]));

        assert_eq!(program.total(), 100.0, "only the positive values count");

        let arcs = program.arcs();
        assert_eq!(arcs[0].fraction, 0.5);
        assert_eq!(arcs[2].fraction, 0.0);
        assert_eq!(arcs[3].fraction, 0.5);
    }

    #[test]
    fn an_all_zero_chart_draws_nothing() {
        let program = program_of(PieChart::new(vec![
            Slice::new("A", 0.0),
            Slice::new("B", -1.0),
        ]));

        assert_eq!(program.total(), 0.0);
        assert!(program.arcs().is_empty());
        assert!(program
            .geometry(iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
            })
            .is_none());

        let element: iced::Element<'_, Message, Theme> =
            PieChart::new(vec![Slice::new("A", 0.0)]).into_element();
        drop(element);
    }

    #[test]
    fn non_finite_values_are_dropped() {
        let program = program_of(PieChart::new(vec![
            Slice::new("Good", 50.0),
            Slice::new("NaN", f64::NAN),
            Slice::new("Inf", f64::INFINITY),
        ]));

        assert_eq!(program.total(), 50.0);
        assert_eq!(program.arcs()[1].fraction, 0.0);
    }

    #[test]
    fn an_empty_chart_renders_without_panicking() {
        let element: iced::Element<'_, Message, Theme> = PieChart::new(Vec::new()).into_element();
        drop(element);
    }

    #[test]
    fn a_single_slice_covers_the_whole_pie() {
        let program = program_of(PieChart::new(vec![Slice::new("All", 42.0)]));
        let arcs = program.arcs();

        assert_eq!(arcs.len(), 1);
        assert_eq!(arcs[0].fraction, 1.0);
        assert!(
            (arcs[0].end - arcs[0].start).abs() > 6.0,
            "nearly a full turn"
        );
    }

    #[test]
    fn the_start_angle_rotates_every_slice() {
        let base = program_of(PieChart::new(slices()));
        let rotated = program_of(PieChart::new(slices()).start_angle(1.0));

        let base_arcs = base.arcs();
        let rotated_arcs = rotated.arcs();

        assert!((rotated_arcs[0].start - base_arcs[0].start - 1.0).abs() < 0.001);
    }

    #[test]
    fn a_slice_can_be_found_by_angle() {
        let program = program_of(PieChart::new(slices()));
        let geometry = program
            .geometry(iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
            })
            .expect("geometry");

        // 45% of the turn is the first slice, so a tenth of the way in is it.
        assert_eq!(geometry.slice_at(std::f32::consts::TAU * 0.05), Some(0));
        // Halfway round is the third slice (45% + 25% = 70%).
        assert_eq!(geometry.slice_at(std::f32::consts::TAU * 0.5), Some(1));
        // Near the end is the last slice.
        assert_eq!(geometry.slice_at(std::f32::consts::TAU * 0.95), Some(3));
    }

    #[test]
    fn an_angle_lookup_handles_a_negative_angle() {
        // The hit test produces angles from `atan2`, which range over -π..π.
        let program = program_of(PieChart::new(slices()));
        let geometry = program
            .geometry(iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
            })
            .expect("geometry");

        let wrapped = geometry.slice_at(-0.1);
        let plain = geometry.slice_at(std::f32::consts::TAU - 0.1);

        assert_eq!(wrapped, plain, "a negative angle wraps to the same slice");
    }

    #[test]
    fn the_inner_radius_leaves_a_hole() {
        let square = iced::Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };

        let solid = program_of(PieChart::new(slices()))
            .geometry(square)
            .expect("geometry");
        let donut = program_of(PieChart::new(slices()).donut(true))
            .geometry(square)
            .expect("geometry");

        assert_eq!(solid.inner, 0.0);
        assert!(donut.inner > 0.0);
        assert!(
            donut.inner < donut.outer,
            "the hole must fit inside the ring"
        );
    }

    #[test]
    fn the_inner_radius_is_clamped_below_the_outer_one() {
        let chart = PieChart::new(slices()).inner_radius(5.0);
        assert!(chart.inner_radius_ratio < 1.0);
    }

    #[test]
    fn the_radius_follows_the_smaller_dimension() {
        // A pie is square by nature; a wide canvas must not stretch it.
        let program = program_of(PieChart::new(slices()));

        let wide = program
            .geometry(iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 100.0,
            })
            .expect("geometry");
        let square = program
            .geometry(iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            })
            .expect("geometry");

        assert!((wide.outer - square.outer).abs() < 0.001);
    }

    #[test]
    fn every_option_combination_renders() {
        let element: iced::Element<'_, Message, Theme> = PieChart::new(slices())
            .donut(true)
            .pad_angle(0.05)
            .labels(false)
            .percentages(false)
            .tooltip(false)
            .outer_radius(0.9)
            .start_angle(0.5)
            .height(300.0)
            .into_element();
        drop(element);
    }

    #[test]
    fn slices_get_distinct_colors() {
        let program = program_of(PieChart::new(slices()));
        let theme = Theme::light();

        let colors: Vec<Color> = (0..4)
            .map(|index| program.color_of(index, &theme))
            .collect();

        for (i, left) in colors.iter().enumerate() {
            for (j, right) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(left, right, "slice {i} and {j} share a colour");
                }
            }
        }
    }

    #[test]
    fn a_slice_with_an_explicit_color_keeps_it() {
        let program = program_of(PieChart::new(
            vec![Slice::new("A", 1.0).color(Color::WHITE)],
        ));

        assert_eq!(program.color_of(0, &Theme::light()), Color::WHITE);
    }

    #[test]
    fn slice_metadata_is_reported() {
        let slice = Slice::new("Rust", 45.0).tone(Tone::Warning);

        assert_eq!(slice.label(), "Rust");
        assert_eq!(slice.value(), 45.0);
    }
}
