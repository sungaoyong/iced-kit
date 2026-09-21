//! Radar charts.
//!
//! Ported from `gpui-kit`'s `RadarChart` (Apache-2.0), rebuilt on iced's canvas.
//!
//! # Reading a radar chart
//!
//! Each axis is a dimension and each series is a polygon through its values on
//! those axes. The shape's *area* is what the eye compares, so the axes must be
//! on a common scale — which is why [`RadarChart::max_value`] exists: a fixed
//! maximum keeps two charts comparable, and an auto maximum would let a series
//! that barely varies fill the ring.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use crate::widgets::plot::series_color;
use crate::widgets::plot::shape;
use crate::widgets::plot::tooltip::{draw_tooltip, TooltipContent};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Font, Length, Pixels, Point, Rectangle};

/// One series of a radar chart: a value per axis.
#[derive(Debug, Clone)]
#[must_use = "a RadarSeries does nothing unless it is given to a chart"]
pub struct RadarSeries {
    name: String,
    values: Vec<f64>,
    tone: Option<Tone>,
    color: Option<Color>,
    show_dots: bool,
}

impl RadarSeries {
    /// Creates a series from one value per axis.
    ///
    /// A series with fewer values than axes stops early rather than wrapping;
    /// missing axes are treated as zero.
    pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
            tone: None,
            color: None,
            show_dots: true,
        }
    }

    /// Sets the series' color tone.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Sets an explicit color, overriding the tone.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Turns the vertex dots on or off.
    pub fn dots(mut self, dots: bool) -> Self {
        self.show_dots = dots;
        self
    }

    /// The series' name, shown in the tooltip.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The series' values.
    #[must_use]
    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

/// A radar chart.
#[must_use = "a RadarChart does nothing unless it is turned into an Element"]
pub struct RadarChart {
    axes: Vec<String>,
    series: Vec<RadarSeries>,
    max_value: Option<f64>,
    grid_levels: usize,
    outer_radius_ratio: f32,
    show_axis_labels: bool,
    show_tooltip: bool,
    stroke_width: f32,
    height: f32,
    tone: Tone,
}

impl RadarChart {
    /// Creates a radar chart from its axis labels and series.
    pub fn new(axes: Vec<String>, series: Vec<RadarSeries>) -> Self {
        Self {
            axes,
            series,
            // Auto by default; `max_value` pins it when charts must compare.
            max_value: None,
            grid_levels: 4,
            outer_radius_ratio: 0.7,
            show_axis_labels: true,
            show_tooltip: false,
            stroke_width: 2.0,
            height: 260.0,
            tone: Tone::Primary,
        }
    }

    /// Creates a single-series chart.
    pub fn single(name: impl Into<String>, axes: Vec<String>, values: Vec<f64>) -> Self {
        Self::new(axes, vec![RadarSeries::new(name, values)])
    }

    /// Fixes the value at the outer ring.
    ///
    /// Worth setting whenever more than one radar chart is shown: an auto
    /// maximum rescales each chart independently, so two shapes of different
    /// sizes can look identical.
    pub fn max_value(mut self, max: f64) -> Self {
        if max.is_finite() && max > 0.0 {
            self.max_value = Some(max);
        }
        self
    }

    /// Sets how many grid rings are drawn.
    pub fn grid_levels(mut self, levels: usize) -> Self {
        self.grid_levels = levels.clamp(1, 10);
        self
    }

    /// Sets the outer radius as a fraction of the available half-extent.
    pub fn outer_radius(mut self, ratio: f32) -> Self {
        self.outer_radius_ratio = ratio.clamp(0.2, 1.0);
        self
    }

    /// Turns the axis labels on or off.
    pub fn axis_labels(mut self, visible: bool) -> Self {
        self.show_axis_labels = visible;
        self
    }

    /// Turns the hover tooltip on or off.
    pub fn tooltip(mut self, tooltip: bool) -> Self {
        self.show_tooltip = tooltip;
        self
    }

    /// Sets the outline width.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width.max(0.0);
        self
    }

    /// Sets the chart's height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(1.0);
        self
    }

    /// Sets the default color tone for series without an explicit color.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Moves the builder into the canvas program that draws it.
    fn into_program(self) -> RadarChartProgram {
        RadarChartProgram {
            axes: self.axes,
            series: self.series,
            max_value: self.max_value,
            grid_levels: self.grid_levels,
            outer_radius_ratio: self.outer_radius_ratio,
            show_axis_labels: self.show_axis_labels,
            show_tooltip: self.show_tooltip,
            stroke_width: self.stroke_width,
            tone: self.tone,
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

impl<Message: 'static> From<RadarChart> for Element<'static, Message, Theme> {
    fn from(chart: RadarChart) -> Self {
        chart.into_element()
    }
}

/// The canvas program that draws a radar chart.
///
/// Mirrors the builder's display toggles.
#[allow(clippy::struct_excessive_bools)]
struct RadarChartProgram {
    axes: Vec<String>,
    series: Vec<RadarSeries>,
    max_value: Option<f64>,
    grid_levels: usize,
    outer_radius_ratio: f32,
    show_axis_labels: bool,
    show_tooltip: bool,
    stroke_width: f32,
    tone: Tone,
}

impl RadarChartProgram {
    /// The value at the outer ring.
    fn resolved_max(&self) -> f64 {
        if let Some(max) = self.max_value {
            return max;
        }

        let observed = self
            .series
            .iter()
            .flat_map(|series| series.values.iter())
            .filter(|value| value.is_finite())
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        // A chart with no positive data still needs a ring to draw.
        if !observed.is_finite() || observed <= 0.0 {
            1.0
        } else {
            observed
        }
    }

    /// The color a series is drawn in.
    fn color_of(&self, index: usize, theme: &Theme) -> Color {
        self.series[index]
            .color
            .unwrap_or_else(|| match self.series[index].tone {
                Some(tone) => tone.accent(theme),
                None if self.series.len() > 1 => series_color(theme, index),
                None => self.tone.accent(theme),
            })
    }

    /// The centre and outer radius for the given bounds.
    fn geometry(&self, bounds: Rectangle) -> (Point, f32) {
        let half_extent = (bounds.width.min(bounds.height) / 2.0).max(1.0);

        (
            Point::new(bounds.width / 2.0, bounds.height / 2.0),
            half_extent * self.outer_radius_ratio,
        )
    }

    /// The angle of an axis, measured clockwise from twelve o'clock.
    fn axis_angle(&self, index: usize) -> f32 {
        let count = self.axes.len().max(1);

        std::f32::consts::TAU * index as f32 / count as f32
    }

    /// Maps an axis index and value onto a point.
    fn point_at(&self, center: Point, radius: f32, axis: usize, value: f64) -> Point {
        let angle = self.axis_angle(axis);
        let max = self.resolved_max();
        let fraction = if max > 0.0 {
            (value / max).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        let r = radius * fraction;

        Point::new(center.x + r * angle.sin(), center.y - r * angle.cos())
    }

    /// The polygon for one series.
    fn polygon(&self, center: Point, radius: f32, series_index: usize) -> Vec<Point> {
        let series = &self.series[series_index];

        (0..self.axes.len())
            .map(|axis| {
                let value = series.values.get(axis).copied().unwrap_or(0.0);
                let value = if value.is_finite() { value } else { 0.0 };

                self.point_at(center, radius, axis, value)
            })
            .collect()
    }

    /// Builds the tooltip for a hovered axis.
    fn tooltip_content(&self, axis: usize, theme: &Theme) -> TooltipContent {
        let label = self
            .axes
            .get(axis)
            .cloned()
            .unwrap_or_else(|| axis.to_string());

        let mut content = TooltipContent::new(label);

        for (series_index, series) in self.series.iter().enumerate() {
            let value = series.values.get(axis).copied().unwrap_or(0.0);
            let value = if value.is_finite() { value } else { 0.0 };

            content = content.row(
                series.name.clone(),
                format_value(value),
                self.color_of(series_index, theme),
            );
        }

        content
    }
}

impl<Message, Renderer> canvas::Program<Message, Theme, Renderer> for RadarChartProgram
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

        let count = self.axes.len();

        if count == 0 || self.series.is_empty() {
            return vec![frame.into_geometry()];
        }

        let (center, radius) = self.geometry(bounds);
        let hovered = *state;

        // The grid: one polygon per level, plus the spokes.
        if self.grid_levels > 0 {
            for level in 1..=self.grid_levels {
                let fraction = level as f32 / self.grid_levels as f32;
                let points: Vec<Point> = (0..count)
                    .map(|axis| {
                        self.point_at(
                            center,
                            radius,
                            axis,
                            self.resolved_max() * f64::from(fraction),
                        )
                    })
                    .collect();

                shape::draw_polygon(&mut frame, &points, None, Some((colors.border, 1.0)));
            }
        }

        // Spokes from the centre to each axis.
        for axis in 0..count {
            let outer = self.point_at(center, radius, axis, self.resolved_max());

            frame.stroke(
                &Path::line(center, outer),
                Stroke::default().with_color(colors.border).with_width(1.0),
            );
        }

        // The axis labels sit just outside the outer ring.
        if self.show_axis_labels {
            for axis in 0..count {
                let angle = self.axis_angle(axis);
                let r = radius + 14.0;

                let anchor = Point::new(center.x + r * angle.sin(), center.y - r * angle.cos());

                // Labels past the horizontal extremes would run off the canvas,
                // so the edge ones are pulled back inside.
                let anchor = Point::new(
                    anchor.x.clamp(24.0, (bounds.width - 24.0).max(24.0)),
                    anchor.y,
                );

                let is_hovered = hovered == Some(axis);

                frame.fill_text(canvas::Text {
                    content: self.axes[axis].clone(),
                    position: anchor,
                    color: if is_hovered {
                        colors.foreground
                    } else {
                        colors.muted_foreground
                    },
                    size: Pixels(11.0),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }

        // The series polygons, drawn back to front.
        for series_index in (0..self.series.len()).rev() {
            let series = &self.series[series_index];
            let color = self.color_of(series_index, kit_theme);
            let points = self.polygon(center, radius, series_index);

            if points.len() < 3 {
                continue;
            }

            // A translucent fill is what makes several overlapping shapes
            // readable at once.
            shape::draw_polygon(
                &mut frame,
                &points,
                Some(Color { a: 0.22, ..color }),
                (self.stroke_width > 0.0).then_some((color, self.stroke_width)),
            );

            if series.show_dots {
                for point in &points {
                    frame.fill(&Path::circle(*point, 3.0), color);
                }
            }
        }

        // The hovered axis is marked with a dot on each series.
        if let Some(axis) = hovered {
            for series_index in 0..self.series.len() {
                let value = self.series[series_index]
                    .values
                    .get(axis)
                    .copied()
                    .unwrap_or(0.0);
                let value = if value.is_finite() { value } else { 0.0 };

                let point = self.point_at(center, radius, axis, value);

                frame.stroke(
                    &Path::circle(point, 5.0),
                    Stroke::default()
                        .with_color(self.color_of(series_index, kit_theme))
                        .with_width(2.0),
                );
            }

            if self.show_tooltip {
                let content = self.tooltip_content(axis, kit_theme);
                let angle = self.axis_angle(axis);
                let r = radius * 0.6;

                let anchor = Point::new(center.x + r * angle.sin(), center.y - r * angle.cos());

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

impl RadarChartProgram {
    /// Which axis the cursor is nearest, if it is close enough to one.
    fn hit_test(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        if !self.show_tooltip {
            return None;
        }

        let position = cursor.position_over(bounds)?;
        let (center, radius) = self.geometry(bounds);

        let dx = position.x - center.x;
        let dy = position.y - center.y;

        if (dx * dx + dy * dy).sqrt() > radius {
            return None;
        }

        // The axis whose spoke is closest in angle to the cursor.
        let count = self.axes.len();

        if count == 0 {
            return None;
        }

        let cursor_angle = dx.atan2(-dy).rem_euclid(std::f32::consts::TAU);
        let step = std::f32::consts::TAU / count as f32;

        // Rounding to the nearest spoke is what makes a whole wedge hoverable
        // rather than just the spoke line itself.
        Some(((cursor_angle / step).round() as usize) % count)
    }
}

/// Formats a value without trailing noise.
fn format_value(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::{RadarChart, RadarSeries};
    use crate::theme::Theme;
    use crate::widgets::display::Tone;
    use iced::{mouse, Color, Point, Rectangle};

    /// Only the type matters; no handler is ever invoked in these tests.
    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    fn axes() -> Vec<String> {
        ["Speed", "Power", "Range", "Accuracy", "Cost"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    fn program_of(chart: RadarChart) -> super::RadarChartProgram {
        chart.into_program()
    }

    #[test]
    fn a_radar_chart_renders() {
        let element: iced::Element<'_, Message, Theme> =
            RadarChart::single("Model A", axes(), vec![80.0, 65.0, 90.0, 70.0, 40.0])
                .into_element();

        drop(element);
    }

    #[test]
    fn multi_series_render_with_distinct_colors() {
        let program = program_of(RadarChart::new(
            axes(),
            vec![
                RadarSeries::new("A", vec![80.0, 65.0, 90.0, 70.0, 40.0]),
                RadarSeries::new("B", vec![60.0, 85.0, 50.0, 95.0, 70.0]),
                RadarSeries::new("C", vec![70.0, 70.0, 70.0, 70.0, 70.0]),
            ],
        ));

        let theme = Theme::light();
        let colors: Vec<Color> = (0..3)
            .map(|index| program.color_of(index, &theme))
            .collect();

        for (i, left) in colors.iter().enumerate() {
            for (j, right) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(left, right, "series {i} and {j} share a colour");
                }
            }
        }

        let element: iced::Element<'_, Message, Theme> = RadarChart::new(
            axes(),
            vec![
                RadarSeries::new("A", vec![80.0, 65.0, 90.0, 70.0, 40.0]),
                RadarSeries::new("B", vec![60.0, 85.0, 50.0, 95.0, 70.0]),
            ],
        )
        .into_element();
        drop(element);
    }

    #[test]
    fn the_auto_maximum_is_the_largest_value() {
        let program = program_of(RadarChart::single(
            "A",
            axes(),
            vec![80.0, 65.0, 90.0, 70.0, 40.0],
        ));

        assert_eq!(program.resolved_max(), 90.0);
    }

    #[test]
    fn a_fixed_maximum_overrides_the_data() {
        // Two charts with different peaks must share a scale, or their shapes
        // are not comparable.
        let program = program_of(
            RadarChart::single("A", axes(), vec![80.0, 65.0, 90.0, 70.0, 40.0]).max_value(100.0),
        );

        assert_eq!(program.resolved_max(), 100.0);
    }

    #[test]
    fn an_invalid_maximum_is_ignored() {
        for bad in [0.0, -10.0, f64::NAN, f64::INFINITY] {
            let program = program_of(RadarChart::single("A", axes(), vec![50.0]).max_value(bad));

            assert!(program.resolved_max() > 0.0, "{bad} must be rejected");
        }
    }

    #[test]
    fn a_chart_with_no_data_still_has_a_scale() {
        // A zero maximum would make every point collapse onto the centre.
        let program = program_of(RadarChart::new(
            axes(),
            vec![RadarSeries::new("A", vec![0.0; 5])],
        ));
        assert!(program.resolved_max() > 0.0);
    }

    #[test]
    fn a_value_at_the_maximum_lands_on_the_outer_ring() {
        let program = program_of(
            RadarChart::single("A", axes(), vec![100.0, 0.0, 0.0, 0.0, 0.0]).max_value(100.0),
        );

        let center = Point::new(100.0, 100.0);
        let point = program.point_at(center, 50.0, 0, 100.0);

        // Axis 0 is at twelve o'clock, so it lands directly above the centre.
        assert!((point.x - center.x).abs() < 0.001);
        assert!((point.y - (center.y - 50.0)).abs() < 0.001);
    }

    #[test]
    fn a_zero_value_lands_on_the_centre() {
        let program = program_of(RadarChart::single("A", axes(), vec![0.0; 5]).max_value(100.0));

        let center = Point::new(100.0, 100.0);
        let point = program.point_at(center, 50.0, 2, 0.0);

        assert!((point.x - center.x).abs() < 0.001);
        assert!((point.y - center.y).abs() < 0.001);
    }

    #[test]
    fn values_above_the_maximum_are_clamped_to_the_ring() {
        // A value past the ring would draw outside the chart.
        let program = program_of(
            RadarChart::single("A", axes(), vec![0.0, 0.0, 0.0, 0.0, 0.0]).max_value(50.0),
        );

        let center = Point::new(100.0, 100.0);
        let point = program.point_at(center, 50.0, 0, 500.0);

        let distance = ((point.x - center.x).powi(2) + (point.y - center.y).powi(2)).sqrt();
        assert!(distance <= 50.001, "distance was {distance}");
    }

    #[test]
    fn axes_are_evenly_spaced_around_the_circle() {
        let program = program_of(RadarChart::single("A", axes(), vec![1.0; 5]));

        let step = std::f32::consts::TAU / 5.0;

        for index in 0..5 {
            let expected = step * index as f32;
            assert!((program.axis_angle(index) - expected).abs() < 0.001);
        }
    }

    #[test]
    fn the_polygon_has_one_point_per_axis() {
        let program = program_of(RadarChart::single("A", axes(), vec![1.0; 5]));
        let polygon = program.polygon(Point::new(100.0, 100.0), 50.0, 0);

        assert_eq!(polygon.len(), 5);
    }

    #[test]
    fn a_series_shorter_than_the_axes_treats_missing_axes_as_zero() {
        let program = program_of(RadarChart::single("Short", axes(), vec![50.0, 50.0]));

        let polygon = program.polygon(Point::new(100.0, 100.0), 50.0, 0);
        assert_eq!(polygon.len(), 5, "every axis still gets a vertex");

        // The missing axes collapse onto the centre.
        for point in &polygon[2..] {
            assert!((point.x - 100.0).abs() < 0.001);
            assert!((point.y - 100.0).abs() < 0.001);
        }
    }

    #[test]
    fn non_finite_values_are_treated_as_zero() {
        let program = program_of(RadarChart::single(
            "Bad",
            axes(),
            vec![f64::NAN, f64::INFINITY, 50.0, f64::NEG_INFINITY, 0.0],
        ));

        let polygon = program.polygon(Point::new(100.0, 100.0), 50.0, 0);

        for point in polygon.iter().take(2) {
            assert!(point.x.is_finite() && point.y.is_finite());
        }
    }

    #[test]
    fn an_empty_chart_renders_without_panicking() {
        let no_axes: iced::Element<'_, Message, Theme> =
            RadarChart::new(Vec::new(), Vec::new()).into_element();
        drop(no_axes);

        let no_series: iced::Element<'_, Message, Theme> =
            RadarChart::new(axes(), Vec::new()).into_element();
        drop(no_series);

        let empty_values: iced::Element<'_, Message, Theme> =
            RadarChart::new(axes(), vec![RadarSeries::new("Empty", Vec::new())]).into_element();
        drop(empty_values);
    }

    #[test]
    fn too_few_axes_to_form_a_polygon_renders() {
        // A polygon needs three points; one or two axes cannot make one.
        for count in 1..=2 {
            let axes: Vec<String> = (0..count).map(|i| i.to_string()).collect();
            let element: iced::Element<'_, Message, Theme> =
                RadarChart::single("A", axes, vec![1.0; count]).into_element();
            drop(element);
        }
    }

    #[test]
    fn every_option_combination_renders() {
        let element: iced::Element<'_, Message, Theme> =
            RadarChart::single("A", axes(), vec![80.0, 65.0, 90.0, 70.0, 40.0])
                .max_value(100.0)
                .grid_levels(6)
                .outer_radius(0.9)
                .axis_labels(false)
                .tooltip(true)
                .stroke_width(3.0)
                .height(320.0)
                .tone(Tone::Success)
                .into_element();
        drop(element);
    }

    #[test]
    fn a_series_can_hide_its_dots() {
        let program = program_of(RadarChart::new(
            axes(),
            vec![RadarSeries::new("A", vec![1.0; 5]).dots(false)],
        ));

        assert!(!program.series[0].show_dots);

        let element: iced::Element<'_, Message, Theme> = RadarChart::new(
            axes(),
            vec![RadarSeries::new("A", vec![1.0; 5]).dots(false)],
        )
        .into_element();
        drop(element);
    }

    #[test]
    fn the_axis_hit_test_snaps_to_the_nearest_spoke() {
        let program = program_of(RadarChart::single("A", axes(), vec![1.0; 5]).tooltip(true));

        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };

        // Just right of centre is nearest the first spoke (twelve o'clock) by
        // angle; the test pins the snapping, not an exact pixel.
        let cursor = mouse::Cursor::Available(Point::new(105.0, 60.0));
        let index = program.hit_test(bounds, cursor);

        assert!(
            index.is_some_and(|i| i < 5),
            "a spoke is always within range"
        );
    }

    #[test]
    fn a_cursor_outside_the_ring_hovers_nothing() {
        let program = program_of(RadarChart::single("A", axes(), vec![1.0; 5]).tooltip(true));

        let bounds = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };

        let cursor = mouse::Cursor::Available(Point::new(0.0, 0.0));
        assert_eq!(program.hit_test(bounds, cursor), None);
    }

    #[test]
    fn a_series_with_an_explicit_color_keeps_it() {
        let program = program_of(RadarChart::new(
            axes(),
            vec![RadarSeries::new("A", vec![1.0; 5]).color(Color::WHITE)],
        ));

        assert_eq!(program.color_of(0, &Theme::light()), Color::WHITE);
    }

    #[test]
    fn series_metadata_is_reported() {
        let series = RadarSeries::new("Model A", vec![1.0, 2.0]).tone(Tone::Warning);

        assert_eq!(series.name(), "Model A");
        assert_eq!(series.values(), &[1.0, 2.0]);
    }
}
