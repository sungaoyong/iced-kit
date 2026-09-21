//! Area charts.
//!
//! Ported from `gpui-kit`'s `AreaChart` (Apache-2.0), rebuilt on iced's canvas.
//!
//! # Overlaid or stacked
//!
//! By default each series fills from its own line down to the baseline, which is
//! what an overlaid area chart shows: where the bands cross is meaningful.
//! [`AreaChart::stacked`] instead accumulates the series, so each band fills
//! from the previous series' line. That turns the top edge into a total, which
//! is what a stacked chart is read for — at the cost of hiding where individual
//! series cross.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use crate::widgets::plot::axis::{AXIS_GAP, LABEL_SIZE};
use crate::widgets::plot::shape::Interpolation;
use crate::widgets::plot::tooltip::{
    draw_hover_dot, draw_tooltip, focus_at, Focus, TooltipContent,
};
use crate::widgets::plot::{
    axis, format_tick, grid_color, label_color, series_color, shape, AxisInset, AxisLabelSide,
    AxisText, PlotAxis, Scale, ScaleLinear, ScalePoint,
};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Stroke};
use iced::{mouse, Color, Element, Font, Length, Pixels, Point, Rectangle};

/// One labelled series in an area chart.
#[derive(Debug, Clone)]
#[must_use = "a AreaSeries does nothing unless it is given to a chart"]
pub struct AreaSeries {
    name: String,
    values: Vec<f64>,
    tone: Option<Tone>,
    color: Option<Color>,
    /// How opaque the fill is, from 0 to 1.
    opacity: f32,
}

impl AreaSeries {
    /// Creates a series from its values.
    pub fn new(name: impl Into<String>, values: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            values,
            tone: None,
            color: None,
            // Translucent by default: overlapping bands must stay readable,
            // and an opaque area would hide every series under it.
            opacity: 0.35,
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

    /// Sets the fill's opacity, from 0 to 1.
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
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

    /// The fill's opacity.
    #[must_use]
    pub fn fill_opacity(&self) -> f32 {
        self.opacity
    }
}

/// An area chart.
///
/// The chart holds several independent display toggles, which is what the lint
/// below flags; grouping them into a sub-struct would only add indirection for
/// callers.
#[must_use = "an AreaChart does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct AreaChart {
    labels: Vec<String>,
    series: Vec<AreaSeries>,
    interpolation: Interpolation,
    stacked: bool,
    show_grid: bool,
    show_x_axis: bool,
    show_tooltip: bool,
    force_zero: bool,
    stroke_width: f32,
    height: f32,
    tone: Tone,
    value_suffix: Option<String>,
}

impl AreaChart {
    /// Creates an area chart from labelled series.
    pub fn new(labels: Vec<String>, series: Vec<AreaSeries>) -> Self {
        Self {
            labels,
            series,
            interpolation: Interpolation::Linear,
            stacked: false,
            show_grid: true,
            show_x_axis: true,
            show_tooltip: true,
            // Like a line chart, the axis includes zero so a small change in a
            // large value does not read as a dramatic one.
            force_zero: true,
            stroke_width: 2.0,
            height: 220.0,
            tone: Tone::Primary,
            value_suffix: None,
        }
    }

    /// Creates a single-series chart, labelling by index.
    pub fn single(name: impl Into<String>, values: Vec<f64>) -> Self {
        let labels = (0..values.len()).map(|index| index.to_string()).collect();

        Self::new(labels, vec![AreaSeries::new(name, values)])
    }

    /// Sets how the outline connects its points.
    pub fn interpolation(mut self, interpolation: Interpolation) -> Self {
        self.interpolation = interpolation;
        self
    }

    /// Accumulates the series instead of overlaying them.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Turns the grid lines on or off.
    pub fn grid(mut self, grid: bool) -> Self {
        self.show_grid = grid;
        self
    }

    /// Turns the category labels on or off.
    pub fn x_axis(mut self, visible: bool) -> Self {
        self.show_x_axis = visible;
        self
    }

    /// Turns the hover tooltip on or off.
    pub fn tooltip(mut self, tooltip: bool) -> Self {
        self.show_tooltip = tooltip;
        self
    }

    /// Whether the value axis is anchored at zero.
    pub fn force_zero(mut self, force: bool) -> Self {
        self.force_zero = force;
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

    /// Appends a unit to tooltip values.
    pub fn value_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.value_suffix = Some(suffix.into());
        self
    }

    /// Moves the builder into the canvas program that draws it.
    fn into_program(self) -> AreaChartProgram {
        AreaChartProgram {
            labels: self.labels,
            series: self.series,
            interpolation: self.interpolation,
            stacked: self.stacked,
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

impl<Message: 'static> From<AreaChart> for Element<'static, Message, Theme> {
    fn from(chart: AreaChart) -> Self {
        chart.into_element()
    }
}

/// Where an area chart's plot area sits, and how its data maps onto it.
struct AreaLayout {
    /// The plot area, in frame-local pixels.
    area: Rectangle,
    /// Maps a category onto an x position.
    x: ScalePoint<String>,
    /// Maps a value onto a y position.
    y: ScaleLinear<f64>,
    /// The value-axis tick values, shared by the grid and the labels.
    ticks: Vec<f64>,
}

/// The canvas program that draws an area chart.
///
/// Mirrors the builder's display toggles.
#[allow(clippy::struct_excessive_bools)]
struct AreaChartProgram {
    labels: Vec<String>,
    series: Vec<AreaSeries>,
    interpolation: Interpolation,
    stacked: bool,
    show_grid: bool,
    show_x_axis: bool,
    show_tooltip: bool,
    force_zero: bool,
    stroke_width: f32,
    tone: Tone,
    value_suffix: Option<String>,
}

impl AreaChartProgram {
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

    /// The label for a category index, falling back to the index.
    fn label_of(&self, index: usize) -> String {
        self.labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| index.to_string())
    }

    /// The value range the y-axis covers.
    fn value_range(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        if self.stacked {
            // A stacked chart's axis covers the totals, not the parts, or the
            // top of every stack would be clipped.
            for index in 0..self.domain_len() {
                let total: f64 = self
                    .series
                    .iter()
                    .filter_map(|series| series.values.get(index))
                    .filter(|value| value.is_finite())
                    .sum();

                min = min.min(total);
                max = max.max(total);
            }
        } else {
            for value in self.series.iter().flat_map(|series| series.values.iter()) {
                if value.is_finite() {
                    min = min.min(*value);
                    max = max.max(*value);
                }
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

    /// The plot area, its scales, and the value-axis ticks.
    fn layout(&self, bounds: Rectangle) -> Option<AreaLayout> {
        let count = self.domain_len();

        if count == 0 {
            return None;
        }

        let labels: Vec<String> = (0..count).map(|index| self.label_of(index)).collect();
        let (min, max) = self.value_range();

        let ticks: Vec<f64> = (0..=4)
            .map(|index| min + (max - min) * f64::from(index) / 4.0)
            .collect();

        let mut sample_axis = PlotAxis::new().label_side(AxisLabelSide::Left);
        for value in &ticks {
            sample_axis = sample_axis.label(AxisText::new(
                format_tick(*value),
                Point::ORIGIN,
                Color::BLACK,
            ));
        }

        let inset = AxisInset::from_labels(&sample_axis, LABEL_SIZE, self.show_x_axis);

        // Frame-local coordinates: a `Frame` created with a size starts at
        // (0, 0) regardless of where the canvas sits in its parent.
        let area = Rectangle {
            x: inset.left,
            y: 4.0,
            width: (bounds.width - inset.left - 8.0).max(1.0),
            height: (bounds.height - inset.bottom - 8.0).max(1.0),
        };

        let x = ScalePoint::new(labels, vec![area.x, area.x + area.width]);
        // The y range is reversed: a larger value must sit higher on screen.
        let y = ScaleLinear::new(vec![min, max], vec![area.y + area.height, area.y]);

        Some(AreaLayout { area, x, y, ticks })
    }

    /// The per-category running totals for a stacked chart.
    ///
    /// Returns one row per series, each holding the value the *previous* series
    /// had accumulated at that category. Those are the lines a stacked band
    /// fills down to.
    fn stack_baselines(&self, count: usize) -> Vec<Vec<f64>> {
        let series_count = self.series.len();
        let mut baselines = vec![vec![0.0; count]; series_count];
        let mut running = vec![0.0_f64; count];

        for (series, baseline_row) in self.series.iter().zip(baselines.iter_mut()) {
            baseline_row.clone_from(&running);

            for (category, slot) in running.iter_mut().enumerate() {
                if let Some(value) = series.values.get(category) {
                    if value.is_finite() {
                        *slot += value;
                    }
                }
            }
        }

        baselines
    }

    /// Builds the tooltip for a hovered category.
    fn tooltip_content(&self, index: usize, theme: &Theme) -> TooltipContent {
        let mut content = TooltipContent::new(self.label_of(index));
        let mut total = 0.0_f64;

        for (series_index, series) in self.series.iter().enumerate() {
            let Some(value) = series.values.get(index) else {
                continue;
            };

            if value.is_finite() {
                total += value;
            }

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

        if self.stacked && self.series.len() > 1 {
            let formatted = match &self.value_suffix {
                Some(suffix) => format!("{}{suffix}", format_tick(total)),
                None => format_tick(total),
            };

            content = content.row("Total", formatted, theme.colors().foreground);
        }

        content
    }
}

impl<Message, Renderer> canvas::Program<Message, Theme, Renderer> for AreaChartProgram
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
        match event {
            iced::Event::Window(iced::window::Event::RedrawRequested(_)) => {
                let was_settling = state.is_settling();
                state.advance();

                if was_settling || state.is_settling() {
                    return Some(canvas::Action::request_redraw());
                }
            }
            // The hit test needs the scales, so it runs here where the state is
            // mutable; `draw` only reads the result.
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                let target = self.hit_test(bounds, cursor);
                let changed = state.target() != target;

                state.set_target(target);

                if changed {
                    return Some(canvas::Action::request_redraw());
                }
            }
            _ => {}
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
        let kit_theme: &crate::theme::Theme = theme;
        let colors = kit_theme.colors();

        let Some(layout) = self.layout(bounds) else {
            return vec![frame.into_geometry()];
        };

        let AreaLayout {
            area,
            x: x_scale,
            y: y_scale,
            ticks,
        } = layout;

        let count = self.domain_len();

        let _ = cursor;

        let grid = grid_color(kit_theme);
        let label = label_color(kit_theme);
        let axis_line = crate::widgets::plot::axis_color(kit_theme);

        // Grid lines and their labels.
        if self.show_grid {
            for value in &ticks {
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

        for value in &ticks {
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

        // The crosshair sits under the bands so it reads as a guide.
        if let Some(index) = state.current() {
            if let Some(x) = x_scale.tick(&self.label_of(index)) {
                axis::draw_dashed_line(
                    &mut frame,
                    Point::new(x, area.y),
                    Point::new(x, area.y + area.height),
                    Stroke::default().with_color(label).with_width(1.0),
                    3.0,
                    3.0,
                );
            }
        }

        let baselines = self.stacked.then(|| self.stack_baselines(count));

        // Bands are drawn back to front so the first series ends up on top,
        // which is the order a legend reads.
        for series_index in (0..self.series.len()).rev() {
            let series = &self.series[series_index];
            let color = self.color_of(series_index, kit_theme);

            let dimmed = state.current().is_some() && state.strength() > 0.0;
            let is_focused = state.current().is_some() && !dimmed;

            let (line_color, fill_opacity) = if dimmed && !is_focused {
                (Color { a: 0.3, ..color }, series.opacity * 0.4)
            } else {
                (color, series.opacity)
            };

            let upper: Vec<Point> = series
                .values
                .iter()
                .enumerate()
                .filter(|(_, value)| value.is_finite())
                .filter_map(|(index, value)| {
                    let x = x_scale.tick(&self.label_of(index))?;
                    let y = y_scale.tick(value)?;
                    Some(Point::new(x, y))
                })
                .collect();

            if upper.len() < 2 {
                continue;
            }

            // What the band fills down to: either the baseline, or the previous
            // series' line when stacked.
            if let Some(baselines) = &baselines {
                let lower: Vec<Point> = baselines[series_index]
                    .iter()
                    .enumerate()
                    .filter_map(|(index, value)| {
                        let x = x_scale.tick(&self.label_of(index))?;
                        let y = y_scale.tick(value)?;
                        Some(Point::new(x, y))
                    })
                    .collect();

                shape::draw_band(
                    &mut frame,
                    &upper,
                    &lower,
                    color,
                    fill_opacity,
                    self.interpolation,
                );
            } else {
                let baseline = y_scale.tick(&0.0).unwrap_or(area.y + area.height);

                shape::draw_area(
                    &mut frame,
                    &upper,
                    baseline,
                    color,
                    fill_opacity,
                    self.interpolation,
                );
            }

            if self.stroke_width > 0.0 {
                shape::draw_line(
                    &mut frame,
                    &upper,
                    line_color,
                    self.stroke_width,
                    self.interpolation,
                );
            }

            // The hovered dot, undimmed so the focused datum stands out.
            if let Some(index) = state.current() {
                if state.strength() > 0.0 {
                    if let Some(value) = series.values.get(index) {
                        if value.is_finite() {
                            if let (Some(x), Some(y)) =
                                (x_scale.tick(&self.label_of(index)), y_scale.tick(value))
                            {
                                draw_hover_dot(
                                    &mut frame,
                                    Point::new(x, y),
                                    color,
                                    state.strength(),
                                    colors.background,
                                );
                            }
                        }
                    }
                }
            }
        }

        axis::draw_axis_line(&mut frame, area, axis_line);

        // Category labels along the bottom.
        if self.show_x_axis {
            for index in 0..count {
                let text = self.label_of(index);
                let Some(x) = x_scale.tick(&text) else {
                    continue;
                };

                frame.fill_text(canvas::Text {
                    content: text,
                    position: Point::new(x, area.y + area.height + AXIS_GAP + 6.0),
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

                if let Some(x) = x_scale.tick(&self.label_of(index)) {
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
        if self.show_tooltip && cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

impl AreaChartProgram {
    /// Which category the cursor is over, if any.
    fn hit_test(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let count = self.domain_len();

        if count == 0 || !self.show_tooltip {
            return None;
        }

        let layout = self.layout(bounds)?;

        // The cursor reports absolute coordinates while the scales work in
        // frame-local ones, so the plot area is shifted into the same space.
        let absolute = Rectangle {
            x: bounds.x + layout.area.x,
            y: bounds.y + layout.area.y,
            width: layout.area.width,
            height: layout.area.height,
        };

        focus_at(cursor.position_over(bounds), absolute, |along| {
            layout.x.least_index(along - layout.area.x)
        })
        .filter(|index| *index < count)
    }
}

#[cfg(test)]
mod tests {
    use super::{AreaChart, AreaChartProgram, AreaSeries};
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

    fn program_of(chart: AreaChart) -> AreaChartProgram {
        chart.into_program()
    }

    #[test]
    fn an_area_chart_renders() {
        let element: iced::Element<'_, Message, Theme> =
            AreaChart::single("Load", vec![20.0, 45.0, 30.0, 60.0, 38.0, 72.0, 50.0, 64.0])
                .into_element();

        drop(element);
    }

    #[test]
    fn a_single_series_chart_labels_by_index() {
        let program = program_of(AreaChart::single("Load", vec![1.0, 2.0, 3.0]));

        assert_eq!(program.labels, vec!["0", "1", "2"]);
        assert_eq!(program.series[0].name(), "Load");
    }

    #[test]
    fn multi_series_charts_render_with_distinct_colors() {
        let program = program_of(AreaChart::new(
            labels(),
            vec![
                AreaSeries::new("Reads", vec![1.0, 2.0, 3.0, 4.0, 5.0]),
                AreaSeries::new("Writes", vec![5.0, 4.0, 3.0, 2.0, 1.0]),
                AreaSeries::new("Errors", vec![0.0, 1.0, 0.0, 2.0, 1.0]),
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

        let element: iced::Element<'_, Message, Theme> = AreaChart::new(
            labels(),
            vec![
                AreaSeries::new("A", vec![1.0, 2.0, 3.0, 4.0, 5.0]),
                AreaSeries::new("B", vec![5.0, 4.0, 3.0, 2.0, 1.0]),
            ],
        )
        .into_element();
        drop(element);
    }

    #[test]
    fn fills_are_translucent_by_default() {
        // An opaque fill would hide every series beneath it.
        let series = AreaSeries::new("Load", vec![1.0, 2.0]);
        assert!(series.fill_opacity() > 0.0 && series.fill_opacity() < 1.0);
    }

    #[test]
    fn fill_opacity_is_clamped() {
        for (input, expected) in [(-1.0_f32, 0.0_f32), (0.5, 0.5), (2.0, 1.0)] {
            let series = AreaSeries::new("Load", vec![1.0]).opacity(input);
            assert_eq!(series.fill_opacity(), expected);
        }
    }

    #[test]
    fn stack_baselines_accumulate_the_previous_series() {
        let program = program_of(
            AreaChart::new(
                labels(),
                vec![
                    AreaSeries::new("A", vec![10.0, 20.0, 30.0, 40.0, 50.0]),
                    AreaSeries::new("B", vec![1.0, 2.0, 3.0, 4.0, 5.0]),
                    AreaSeries::new("C", vec![100.0, 100.0, 100.0, 100.0, 100.0]),
                ],
            )
            .stacked(true),
        );

        let baselines = program.stack_baselines(5);

        // The first series sits on the axis.
        assert_eq!(baselines[0], vec![0.0; 5]);
        // The second sits on the first.
        assert_eq!(baselines[1], vec![10.0, 20.0, 30.0, 40.0, 50.0]);
        // The third sits on the sum of the first two.
        assert_eq!(baselines[2], vec![11.0, 22.0, 33.0, 44.0, 55.0]);
    }

    #[test]
    fn a_stacked_axis_covers_the_totals() {
        let grouped = program_of(AreaChart::new(
            labels(),
            vec![
                AreaSeries::new("A", vec![100.0; 5]),
                AreaSeries::new("B", vec![100.0; 5]),
            ],
        ));

        let stacked = program_of(
            AreaChart::new(
                labels(),
                vec![
                    AreaSeries::new("A", vec![100.0; 5]),
                    AreaSeries::new("B", vec![100.0; 5]),
                ],
            )
            .stacked(true),
        );

        assert_eq!(
            grouped.value_range().1,
            100.0,
            "overlaid bands share the axis"
        );
        assert_eq!(stacked.value_range().1, 200.0, "the stack totals 200");
    }

    #[test]
    fn overlaid_bands_default_to_the_zero_baseline() {
        let program = program_of(AreaChart::single("Load", vec![20.0, 40.0, 30.0]));

        assert!(!program.stacked);
        assert_eq!(program.value_range().0, 0.0);
    }

    #[test]
    fn the_axis_can_zoom_to_the_data() {
        let program =
            program_of(AreaChart::single("Load", vec![100.0, 120.0, 110.0]).force_zero(false));

        assert!(program.value_range().0 > 0.0);
    }

    #[test]
    fn every_interpolation_mode_renders() {
        for interpolation in [
            Interpolation::Linear,
            Interpolation::Natural,
            Interpolation::StepAfter,
        ] {
            let element: iced::Element<'_, Message, Theme> =
                AreaChart::single("Load", vec![1.0, 5.0, 3.0, 8.0])
                    .interpolation(interpolation)
                    .into_element();
            drop(element);
        }
    }

    #[test]
    fn a_flat_series_gets_a_non_degenerate_axis() {
        let program = program_of(AreaChart::single("Flat", vec![5.0, 5.0, 5.0]));
        let (min, max) = program.value_range();

        assert!(max > min, "a flat series still spans a range");
    }

    #[test]
    fn non_finite_values_are_excluded_from_the_range() {
        for values in [
            vec![f64::NAN, f64::NAN],
            vec![f64::INFINITY, f64::NEG_INFINITY],
            vec![f64::NAN, 1.0, f64::INFINITY],
        ] {
            let program = program_of(AreaChart::single("Bad", values));
            let (min, max) = program.value_range();

            assert!(min.is_finite() && max.is_finite(), "range must be finite");
            assert!(max > min);
        }
    }

    #[test]
    fn an_empty_chart_renders_without_panicking() {
        let no_data: iced::Element<'_, Message, Theme> =
            AreaChart::new(Vec::new(), Vec::new()).into_element();
        drop(no_data);

        let no_series: iced::Element<'_, Message, Theme> =
            AreaChart::new(labels(), Vec::new()).into_element();
        drop(no_series);

        let empty_series: iced::Element<'_, Message, Theme> =
            AreaChart::single("Empty", Vec::new()).into_element();
        drop(empty_series);
    }

    #[test]
    fn a_single_point_renders() {
        // One point cannot describe an area, so this exercises the guard.
        let element: iced::Element<'_, Message, Theme> =
            AreaChart::single("One", vec![42.0]).into_element();
        drop(element);
    }

    #[test]
    fn mismatched_series_lengths_render() {
        let short: iced::Element<'_, Message, Theme> =
            AreaChart::new(labels(), vec![AreaSeries::new("Short", vec![1.0, 2.0])]).into_element();
        drop(short);

        let long: iced::Element<'_, Message, Theme> = AreaChart::new(
            vec!["One".to_owned()],
            vec![AreaSeries::new("Long", vec![1.0, 2.0, 3.0, 4.0, 5.0])],
        )
        .into_element();
        drop(long);
    }

    #[test]
    fn a_stacked_chart_with_mismatched_lengths_renders() {
        // `stack_baselines` walks every series against the category count, so a
        // short series must not index out of bounds.
        let element: iced::Element<'_, Message, Theme> = AreaChart::new(
            labels(),
            vec![
                AreaSeries::new("Full", vec![1.0; 5]),
                AreaSeries::new("Short", vec![1.0, 2.0]),
                AreaSeries::new("Long", vec![1.0; 9]),
            ],
        )
        .stacked(true)
        .into_element();
        drop(element);
    }

    #[test]
    fn every_option_combination_renders() {
        let element: iced::Element<'_, Message, Theme> =
            AreaChart::single("Load", vec![1.0, 5.0, 3.0])
                .stacked(true)
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
    fn series_metadata_is_reported() {
        let series = AreaSeries::new("Load", vec![1.0, 2.0])
            .tone(Tone::Warning)
            .opacity(0.6);

        assert_eq!(series.name(), "Load");
        assert_eq!(series.values(), &[1.0, 2.0]);
        assert_eq!(series.fill_opacity(), 0.6);
    }

    #[test]
    fn a_series_with_an_explicit_color_keeps_it() {
        let program = program_of(AreaChart::new(
            labels(),
            vec![AreaSeries::new("A", vec![1.0; 5]).color(Color::WHITE)],
        ));

        assert_eq!(program.color_of(0, &Theme::light()), Color::WHITE);
    }
}
