//! Line charts.
//!
//! Ported from `gpui-kit`'s `LineChart` (Apache-2.0), rebuilt on iced's canvas.
//!
//! # Why this is a canvas program rather than a widget tree
//!
//! A line chart's geometry — where each point lands, which segment the cursor is
//! nearest, how the highlight eases — is computed from the data and the bounds.
//! Expressing that as nested `Element`s would mean recomputing it during layout
//! and losing the ability to hit-test against the real drawn positions. A canvas
//! program gets the bounds and the cursor directly, which is what makes hover
//! work with no extra machinery.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use crate::widgets::plot::axis::{AXIS_GAP, LABEL_SIZE};
use crate::widgets::plot::shape::Interpolation;
use crate::widgets::plot::tooltip::{
    draw_hover_dot, draw_tooltip, focus_at, Focus, TooltipContent,
};
use crate::widgets::plot::{
    axis, format_tick, grid_color, series_color, shape, AxisInset, AxisLabelSide, AxisText,
    PlotAxis, Scale, ScaleLinear, ScalePoint,
};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Font, Length, Pixels, Point, Rectangle};

/// One labelled series in a line chart.
#[derive(Debug, Clone)]
#[must_use = "a LineSeries does nothing unless it is given to a LineChart"]
pub struct LineSeries {
    name: String,
    values: Vec<f64>,
    tone: Option<Tone>,
    color: Option<Color>,
}

impl LineSeries {
    /// Creates a series from its values.
    pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
            tone: None,
            color: None,
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

/// A line chart.
#[must_use = "a LineChart does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct LineChart {
    labels: Vec<String>,
    series: Vec<LineSeries>,
    interpolation: Interpolation,
    show_points: bool,
    show_grid: bool,
    show_x_axis: bool,
    show_tooltip: bool,
    force_zero: bool,
    stroke_width: f32,
    height: f32,
    tone: Tone,
    value_suffix: Option<String>,
}

impl LineChart {
    /// Creates a line chart from labelled series.
    ///
    /// Every series is plotted against the same category labels; a series
    /// shorter than the labels stops early rather than wrapping.
    pub fn new(labels: Vec<String>, series: Vec<LineSeries>) -> Self {
        Self {
            labels,
            series,
            interpolation: Interpolation::Linear,
            show_points: false,
            show_grid: true,
            show_x_axis: true,
            show_tooltip: true,
            // A line chart's y-axis includes zero, which keeps a small change in
            // a large value from reading as a dramatic one. `force_zero(false)`
            // opts into the zoomed view when the trend is the point.
            force_zero: true,
            stroke_width: 2.0,
            height: 220.0,
            tone: Tone::Primary,
            value_suffix: None,
        }
    }

    /// Creates a single-series chart from one run of values.
    ///
    /// The labels default to the index, which is what a chart of a bare
    /// measurement over time wants.
    pub fn single(name: impl Into<String>, values: Vec<f64>) -> Self {
        let labels = (0..values.len()).map(|index| index.to_string()).collect();

        Self::new(labels, vec![LineSeries::new(name, values)])
    }

    /// Sets how the line connects its points.
    pub fn interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Draws a dot at every data point.
    pub fn points(mut self, points: bool) -> Self {
        self.show_points = points;
        self
    }

    /// Turns the grid lines on or off.
    pub fn grid(mut self, grid: bool) -> Self {
        self.show_grid = grid;
        self
    }

    /// Turns the category axis labels on or off.
    pub fn x_axis(mut self, visible: bool) -> Self {
        self.show_x_axis = visible;
        self
    }

    /// Turns the hover tooltip and crosshair on or off.
    pub fn tooltip(mut self, tooltip: bool) -> Self {
        self.show_tooltip = tooltip;
        self
    }

    /// Whether the y-axis is anchored at zero.
    pub fn force_zero(mut self, force: bool) -> Self {
        self.force_zero = force;
        self
    }

    /// Sets the line width.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width.max(0.5);
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

    /// Appends a unit to tooltip values, such as `"%"` or `" ms"`.
    pub fn value_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.value_suffix = Some(suffix.into());
        self
    }

    /// Moves the builder into the canvas program that draws it.
    ///
    /// The program owns the chart's geometry; the builder only collects
    /// options. Keeping the two apart means the computations live in one place
    /// rather than being duplicated for the tests to reach.
    fn into_program(self) -> LineChartProgram {
        LineChartProgram {
            labels: self.labels,
            series: self.series,
            interpolation: self.interpolation,
            show_points: self.show_points,
            show_grid: self.show_grid,
            show_x_axis: self.show_x_axis,
            show_tooltip: self.show_tooltip,
            force_zero: self.force_zero,
            stroke_width: self.stroke_width,
            tone: self.tone,
            value_suffix: self.value_suffix,
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

impl<Message: 'static> From<LineChart> for Element<'static, Message, Theme> {
    fn from(chart: LineChart) -> Self {
        chart.into_element()
    }
}

/// Where a line chart's plot area sits, and how its data maps onto it.
struct LineLayout {
    /// The plot area, in frame-local pixels.
    area: Rectangle,
    /// Maps a category onto an x position.
    x: ScalePoint<String>,
    /// Maps a value onto a y position.
    y: ScaleLinear<f64>,
    /// The value-axis tick values, shared by the grid and the labels.
    ticks: Vec<f64>,
}

/// The canvas program that draws a line chart.
///
/// Mirrors the builder's display toggles.
#[allow(clippy::struct_excessive_bools)]
struct LineChartProgram {
    labels: Vec<String>,
    series: Vec<LineSeries>,
    interpolation: Interpolation,
    show_points: bool,
    show_grid: bool,
    show_x_axis: bool,
    show_tooltip: bool,
    force_zero: bool,
    stroke_width: f32,
    tone: Tone,
    value_suffix: Option<String>,
}

impl LineChartProgram {
    /// The value range, mirroring `LineChart::value_range`.
    fn value_range(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for value in self.series.iter().flat_map(|series| series.values.iter()) {
            if value.is_finite() {
                min = min.min(*value);
                max = max.max(*value);
            }
        }

        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }

        if self.force_zero {
            min = min.min(0.0);
            max = max.max(0.0);
        }

        if (max - min).abs() < f64::EPSILON {
            let pad = if max.abs() < 1.0 {
                1.0
            } else {
                max.abs() * 0.1
            };
            return (min - pad, max + pad);
        }

        (min, max)
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

    /// The number of categories the chart spans.
    fn domain_len(&self) -> usize {
        self.labels.len().max(
            self.series
                .iter()
                .map(|s| s.values.len())
                .max()
                .unwrap_or(0),
        )
    }

    /// The plot area, and the scales mapping data onto it.
    fn layout(&self, bounds: Rectangle) -> Option<LineLayout> {
        let count = self.domain_len();

        if count == 0 {
            return None;
        }

        let labels: Vec<String> = (0..count)
            .map(|index| {
                self.labels
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| index.to_string())
            })
            .collect();

        // The y labels are needed before the plot area exists, so their width is
        // estimated rather than measured. The tick values are returned so the
        // drawing pass labels exactly the lines it draws.
        let (min, max) = self.value_range();
        let tick_values: Vec<f64> = (0..=4)
            .map(|index| min + (max - min) * f64::from(index) / 4.0)
            .collect();

        let mut sample_axis = PlotAxis::new().label_side(AxisLabelSide::Left);
        for value in &tick_values {
            sample_axis = sample_axis.label(AxisText::new(
                format_tick(*value),
                Point::ORIGIN,
                Color::BLACK,
            ));
        }

        let inset = AxisInset::from_labels(&sample_axis, LABEL_SIZE, self.show_x_axis);

        // Frame-local coordinates: a `Frame` created with a size starts at
        // (0, 0) regardless of where the canvas sits in its parent.
        let width = bounds.width;
        let height = bounds.height;

        let area = Rectangle {
            x: inset.left,
            y: 4.0,
            width: (width - inset.left - 8.0).max(1.0),
            height: (height - inset.bottom - 8.0).max(1.0),
        };

        // The y range is reversed: a larger value must sit higher on screen.
        let x_scale = ScalePoint::new(labels, vec![area.x, area.x + area.width]);
        let y_scale = ScaleLinear::new(vec![min, max], vec![area.y + area.height, area.y]);

        Some(LineLayout {
            area,
            x: x_scale,
            y: y_scale,
            ticks: tick_values,
        })
    }
}

impl<Message, Renderer> canvas::Program<Message, Theme, Renderer> for LineChartProgram
where
    Renderer:
        iced::advanced::graphics::geometry::Renderer + iced::advanced::text::Renderer<Font = Font>,
{
    type State = Focus;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        // The hover target is resolved during `draw`, where the scales exist.
        // `update` only advances the easing ramp and asks for the frames that
        // the ramp needs.
        if let iced::Event::Window(iced::window::Event::RedrawRequested(_)) = event {
            let settling = state.is_settling();
            state.advance();

            if settling || state.is_settling() {
                return Some(canvas::Action::request_redraw());
            }
        }

        // Moving the pointer changes the target, so a redraw is needed to
        // resolve which datum is under it.
        if let iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) = event {
            let _ = (bounds, cursor);
            return Some(canvas::Action::request_redraw());
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());

        // `Canvas::new` in `into_element` fixes this program's theme to this
        // crate's `Theme`, so the parameter is known here without a bound.
        let kit_theme: &crate::theme::Theme = theme;
        let colors = kit_theme.colors();

        let Some(LineLayout {
            area,
            x: x_scale,
            y: y_scale,
            ticks: tick_values,
        }) = self.layout(bounds)
        else {
            return vec![frame.into_geometry()];
        };

        let count = self.domain_len();

        // Resolve the hovered datum before drawing so the crosshair and the
        // tooltip agree on it.
        // The cursor reports absolute coordinates while the scales work in
        // frame-local ones, so the plot area is shifted into the same space for
        // the hit test.
        let absolute_area = Rectangle {
            x: bounds.x + area.x,
            y: bounds.y + area.y,
            width: area.width,
            height: area.height,
        };

        let hovered = self.show_tooltip.then(|| {
            focus_at(cursor.position_over(bounds), absolute_area, |along| {
                x_scale.least_index(along)
            })
            .filter(|index| *index < count)
        });

        let target = hovered.flatten();

        let grid = grid_color(kit_theme);
        let label = crate::widgets::plot::label_color(kit_theme);
        let axis_line = crate::widgets::plot::axis_color(kit_theme);

        // Grid lines behind everything else, at the y-axis tick positions.
        if self.show_grid {
            for value in &tick_values {
                let Some(y) = y_scale.tick(value) else {
                    continue;
                };

                axis::draw_dashed_line(
                    &mut frame,
                    Point::new(area.x, y),
                    Point::new(area.x + area.width, y),
                    Stroke::default().with_color(grid).with_width(1.0),
                    4.0,
                    3.0,
                );
            }
        }

        // The y labels sit in the gutter the inset reserved, so they never
        // overlap the series.
        for value in &tick_values {
            let Some(y) = y_scale.tick(value) else {
                continue;
            };

            frame.fill_text(canvas::Text {
                content: format_tick(*value),
                position: Point::new(area.x - AXIS_GAP, y),
                color: label,
                size: Pixels(LABEL_SIZE),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                font: Font::DEFAULT,
                ..canvas::Text::default()
            });
        }

        // The crosshair sits under the series so it reads as a guide.
        if let Some(index) = target {
            if let Some(x) = x_scale.tick(&self.labels_or_index(index)) {
                let cross = Stroke::default().with_color(label).with_width(1.0);
                axis::draw_dashed_line(
                    &mut frame,
                    Point::new(x, area.y),
                    Point::new(x, area.y + area.height),
                    cross,
                    3.0,
                    3.0,
                );
            }
        }

        // The series themselves.
        for (series_index, series) in self.series.iter().enumerate() {
            let color = self.color_of(series_index, kit_theme);
            let dimmed = target.is_some() && state.strength() > 0.0;

            let color = if dimmed {
                Color { a: 0.35, ..color }
            } else {
                color
            };

            let points: Vec<Point> = series
                .values
                .iter()
                .enumerate()
                .filter(|(_, value)| value.is_finite())
                .filter_map(|(index, value)| {
                    let x = x_scale.tick(&self.labels_or_index(index))?;
                    let y = y_scale.tick(value)?;
                    Some(Point::new(x, y))
                })
                .collect();

            if points.len() >= 2 {
                shape::draw_line(
                    &mut frame,
                    &points,
                    color,
                    self.stroke_width,
                    self.interpolation,
                );
            }

            if self.show_points {
                for point in &points {
                    frame.fill(&Path::circle(*point, 2.5), color);
                }
            }

            // The hovered dot is drawn undimmed, so the focused datum stands out
            // from the faded rest.
            if let Some(index) = state.current() {
                if state.strength() > 0.0 {
                    if let Some(value) = series.values.get(index) {
                        if value.is_finite() {
                            if let (Some(x), Some(y)) = (
                                x_scale.tick(&self.labels_or_index(index)),
                                y_scale.tick(value),
                            ) {
                                let full = self.color_of(series_index, kit_theme);
                                draw_hover_dot(
                                    &mut frame,
                                    Point::new(x, y),
                                    full,
                                    state.strength(),
                                    colors.background,
                                );
                            }
                        }
                    }
                }
            }
        }

        // The axes' own lines, drawn last so they sit above the grid.
        axis::draw_axis_line(&mut frame, area, axis_line);

        // Category labels along the bottom.
        if self.show_x_axis {
            for index in 0..count {
                let label_text = self.labels_or_index(index);
                let Some(x) = x_scale.tick(&label_text) else {
                    continue;
                };

                frame.fill_text(canvas::Text {
                    content: label_text,
                    position: Point::new(x, area.y + area.height + 12.0),
                    color: label,
                    size: Pixels(LABEL_SIZE),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }

        // The tooltip, on top of everything.
        if let Some(index) = state.current() {
            if state.strength() > 0.0 {
                let content = self.tooltip_content(index, kit_theme);

                if let Some(x) = x_scale.tick(&self.labels_or_index(index)) {
                    draw_tooltip(
                        &mut frame,
                        &content,
                        Point::new(x, area.y + area.height / 2.0),
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
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        // A chart with a tooltip is interactive, so the pointer should say so.
        if self.show_tooltip && cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

impl LineChartProgram {
    /// The label for a category index, falling back to the index.
    fn labels_or_index(&self, index: usize) -> String {
        self.labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| index.to_string())
    }

    /// Builds the tooltip for a hovered category.
    fn tooltip_content(&self, index: usize, theme: &Theme) -> TooltipContent {
        let mut content = TooltipContent::new(self.labels_or_index(index));

        for (series_index, series) in self.series.iter().enumerate() {
            let Some(value) = series.values.get(index) else {
                continue;
            };

            let formatted = match &self.value_suffix {
                Some(suffix) => format!("{}{suffix}", format_tick(*value)),
                None => format_tick(*value),
            };

            content = content.row(
                series.name.clone(),
                formatted,
                self.color_of(series_index, theme),
            );
        }

        content
    }
}

#[cfg(test)]
mod tests {
    use super::{LineChart, LineSeries};
    use crate::theme::Theme;
    use crate::widgets::display::Tone;
    use crate::widgets::plot::shape::Interpolation;
    use iced::Color;

    /// Only the type matters; no handler is ever invoked in these tests.
    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    fn labels() -> Vec<String> {
        ["Mon", "Tue", "Wed", "Thu", "Fri"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    /// Moves a builder into its program, which owns the chart's computations.
    ///
    /// `into_element` is the only public conversion, so the program is reached
    /// the same way the canvas would reach it.
    fn program_of(chart: super::LineChart) -> super::LineChartProgram {
        chart.into_program()
    }

    #[test]
    fn a_line_chart_renders() {
        let element: iced::Element<'_, Message, Theme> = LineChart::new(
            labels(),
            vec![LineSeries::new("Reads", vec![10.0, 25.0, 18.0, 32.0, 28.0])],
        )
        .into_element();

        drop(element);
    }

    #[test]
    fn a_single_series_chart_labels_by_index() {
        let chart = LineChart::single("Load", vec![1.0, 2.0, 3.0]);

        let program = program_of(chart);
        assert_eq!(program.labels, vec!["0", "1", "2"]);
        assert_eq!(program.series.len(), 1);
        assert_eq!(program.series[0].name(), "Load");
    }

    #[test]
    fn every_interpolation_mode_renders() {
        for interpolation in [
            Interpolation::Linear,
            Interpolation::Natural,
            Interpolation::StepAfter,
        ] {
            let element: iced::Element<'_, Message, Theme> =
                LineChart::single("Load", vec![1.0, 5.0, 3.0, 8.0])
                    .interpolation(interpolation)
                    .into_element();
            drop(element);
        }
    }

    #[test]
    fn a_chart_renders_with_every_option_toggled() {
        let element: iced::Element<'_, Message, Theme> =
            LineChart::single("Load", vec![1.0, 5.0, 3.0])
                .points(true)
                .grid(false)
                .x_axis(false)
                .tooltip(false)
                .force_zero(false)
                .stroke_width(3.0)
                .height(300.0)
                .tone(Tone::Success)
                .value_suffix(" ms")
                .into_element();
        drop(element);
    }

    #[test]
    fn multi_series_charts_render_with_distinct_colors() {
        let chart = LineChart::new(
            labels(),
            vec![
                LineSeries::new("Reads", vec![1.0, 2.0, 3.0, 4.0, 5.0]),
                LineSeries::new("Writes", vec![5.0, 4.0, 3.0, 2.0, 1.0]),
                LineSeries::new("Errors", vec![0.0, 1.0, 0.0, 2.0, 1.0]),
            ],
        );

        let theme = Theme::light();
        let program = program_of(chart);
        let colors: Vec<Color> = (0..3)
            .map(|index| program.color_of(index, &theme))
            .collect();

        // Three series in one color would be unreadable.
        for (i, left) in colors.iter().enumerate() {
            for (j, right) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(left, right, "series {i} and {j} share a colour");
                }
            }
        }

        let element: iced::Element<'_, Message, Theme> = LineChart::new(
            labels(),
            vec![LineSeries::new("Reads", vec![1.0, 2.0, 3.0, 4.0, 5.0])],
        )
        .into_element();
        drop(element);
    }

    #[test]
    fn a_series_with_an_explicit_color_keeps_it() {
        let chart = LineChart::new(
            labels(),
            vec![LineSeries::new("Load", vec![1.0, 2.0, 3.0, 4.0, 5.0]).color(Color::WHITE)],
        );

        assert_eq!(program_of(chart).color_of(0, &Theme::light()), Color::WHITE);
    }

    #[test]
    fn the_y_axis_includes_zero_by_default() {
        // Anchoring at zero is what stops a small change in a large value from
        // reading as a dramatic one.
        let program = program_of(LineChart::single("High", vec![100.0, 105.0, 103.0]));
        assert_eq!(program.value_range().0, 0.0);
    }

    #[test]
    fn a_zoomed_axis_can_exclude_zero() {
        let program =
            program_of(LineChart::single("High", vec![100.0, 105.0, 103.0]).force_zero(false));
        assert!(program.value_range().0 > 0.0, "the axis zooms to the data");
    }

    #[test]
    fn a_flat_series_gets_a_non_degenerate_axis() {
        // Without padding the line would sit exactly on the axis and the chart
        // would look empty.
        let program = program_of(LineChart::single("Flat", vec![5.0, 5.0, 5.0]));
        let (min, max) = program.value_range();

        assert!(max > min, "a flat series still spans a range");
        assert!(min <= 5.0 && max >= 5.0);
    }

    #[test]
    fn non_finite_values_are_excluded_from_the_range() {
        for values in [
            vec![f64::NAN, f64::NAN],
            vec![f64::INFINITY, f64::NEG_INFINITY],
            vec![f64::NAN, 1.0, f64::INFINITY],
        ] {
            let program = program_of(LineChart::single("Bad", values));
            let (min, max) = program.value_range();

            assert!(min.is_finite() && max.is_finite(), "range must be finite");
            assert!(max > min, "range must have extent");
        }
    }

    #[test]
    fn an_empty_chart_renders_without_panicking() {
        let empty: iced::Element<'_, Message, Theme> =
            LineChart::new(Vec::new(), Vec::new()).into_element();
        drop(empty);

        let no_values: iced::Element<'_, Message, Theme> =
            LineChart::new(labels(), Vec::new()).into_element();
        drop(no_values);

        let empty_series: iced::Element<'_, Message, Theme> =
            LineChart::single("Empty", Vec::new()).into_element();
        drop(empty_series);
    }

    #[test]
    fn a_single_point_renders() {
        // One point cannot form a line, so this exercises the degenerate path.
        let element: iced::Element<'_, Message, Theme> =
            LineChart::single("One", vec![42.0]).into_element();
        drop(element);
    }

    #[test]
    fn a_series_shorter_than_the_labels_renders() {
        // Mismatched lengths are a common caller mistake; the chart must not
        // index out of bounds.
        let element: iced::Element<'_, Message, Theme> =
            LineChart::new(labels(), vec![LineSeries::new("Short", vec![1.0, 2.0])]).into_element();
        drop(element);
    }

    #[test]
    fn a_series_longer_than_the_labels_renders() {
        let element: iced::Element<'_, Message, Theme> = LineChart::new(
            vec!["One".to_owned()],
            vec![LineSeries::new("Long", vec![1.0, 2.0, 3.0, 4.0, 5.0])],
        )
        .into_element();
        drop(element);
    }

    #[test]
    fn series_metadata_is_reported() {
        let series = LineSeries::new("Reads", vec![1.0, 2.0]).tone(Tone::Danger);

        assert_eq!(series.name(), "Reads");
        assert_eq!(series.values(), &[1.0, 2.0]);
        assert_eq!(series.tone, Some(Tone::Danger));
    }
}
