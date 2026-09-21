//! Bar charts.
//!
//! Ported from `gpui-kit`'s `BarChart` (Apache-2.0), rebuilt on iced's canvas.
//!
//! # Negative values
//!
//! Bars grow from zero, not from the axis floor. A chart whose data spans zero
//! therefore shows bars on both sides of a shared baseline, which is what makes
//! a change of sign readable.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use crate::widgets::plot::axis::{AXIS_GAP, LABEL_SIZE};
use crate::widgets::plot::shape::Corners;
use crate::widgets::plot::tooltip::{draw_tooltip, focus_at, Focus, TooltipContent};
use crate::widgets::plot::{
    axis, format_tick, grid_color, label_color, series_color, shape, AxisInset, AxisLabelSide,
    AxisText, PlotAxis, Scale, ScaleBand, ScaleLinear,
};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Stroke};
use iced::{mouse, Color, Element, Font, Length, Pixels, Point, Rectangle};

/// Which way a bar chart's bars grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarAlignment {
    /// Vertical bars growing upward from a bottom baseline.
    #[default]
    Bottom,
    /// Vertical bars growing downward from a top baseline.
    Top,
    /// Horizontal bars growing rightward from a left baseline.
    Left,
    /// Horizontal bars growing leftward from a right baseline.
    Right,
}

impl BarAlignment {
    /// Whether this alignment lays bars out horizontally.
    #[must_use]
    pub const fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// One labelled series in a bar chart.
#[derive(Debug, Clone)]
#[must_use = "a BarSeries does nothing unless it is given to a BarChart"]
pub struct BarSeries {
    name: String,
    values: Vec<f64>,
    tone: Option<Tone>,
    color: Option<Color>,
}

impl BarSeries {
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

/// A bar chart.
///
/// The chart holds several independent display toggles, which is what the lint
/// below flags; grouping them into a sub-struct would only add indirection for
/// callers.
#[must_use = "a BarChart does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct BarChart {
    labels: Vec<String>,
    series: Vec<BarSeries>,
    alignment: BarAlignment,
    corners: Corners,
    show_grid: bool,
    show_x_axis: bool,
    show_value_axis: bool,
    show_tooltip: bool,
    show_values: bool,
    grouped: bool,
    inner_padding: f32,
    height: f32,
    tone: Tone,
}

impl BarChart {
    /// Creates a bar chart from labelled series.
    ///
    /// Series are drawn side by side within each category's band, which is what
    /// makes a comparison across series possible at a glance.
    pub fn new(labels: Vec<String>, series: Vec<BarSeries>) -> Self {
        Self {
            labels,
            series,
            alignment: BarAlignment::default(),
            // A slight radius stops the bars reading as blocks of colour.
            corners: Corners::top(4.0),
            show_grid: true,
            show_x_axis: true,
            show_value_axis: true,
            show_tooltip: true,
            show_values: false,
            grouped: true,
            inner_padding: 0.25,
            height: 220.0,
            tone: Tone::Primary,
        }
    }

    /// Creates a single-series chart.
    pub fn single(name: impl Into<String>, labels: Vec<String>, values: Vec<f64>) -> Self {
        Self::new(labels, vec![BarSeries::new(name, values)])
    }

    /// Sets which way the bars grow.
    pub fn alignment(mut self, alignment: BarAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets the corner rounding of each bar.
    pub fn corners(mut self, corners: Corners) -> Self {
        self.corners = corners;
        self
    }

    /// Stacks the series on top of one another instead of side by side.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.grouped = !stacked;
        self
    }

    /// Sets the gap between bands, as a fraction of the slot.
    pub fn padding(mut self, padding: f32) -> Self {
        self.inner_padding = padding.clamp(0.0, 0.9);
        self
    }

    /// Prints each bar's value above it.
    pub fn values_visible(mut self, visible: bool) -> Self {
        self.show_values = visible;
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

    /// Turns the value axis labels on or off.
    pub fn value_axis(mut self, visible: bool) -> Self {
        self.show_value_axis = visible;
        self
    }

    /// Turns the hover tooltip on or off.
    pub fn tooltip(mut self, tooltip: bool) -> Self {
        self.show_tooltip = tooltip;
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
    fn into_program(self) -> BarChartProgram {
        BarChartProgram {
            labels: self.labels,
            series: self.series,
            alignment: self.alignment,
            corners: self.corners,
            show_grid: self.show_grid,
            show_x_axis: self.show_x_axis,
            show_value_axis: self.show_value_axis,
            show_tooltip: self.show_tooltip,
            show_values: self.show_values,
            grouped: self.grouped,
            inner_padding: self.inner_padding,
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

impl<Message: 'static> From<BarChart> for Element<'static, Message, Theme> {
    fn from(chart: BarChart) -> Self {
        chart.into_element()
    }
}

/// The canvas program that draws a bar chart.
///
/// Mirrors the builder's display toggles.
#[allow(clippy::struct_excessive_bools)]
struct BarChartProgram {
    labels: Vec<String>,
    series: Vec<BarSeries>,
    alignment: BarAlignment,
    corners: Corners,
    show_grid: bool,
    show_x_axis: bool,
    show_value_axis: bool,
    show_tooltip: bool,
    show_values: bool,
    grouped: bool,
    inner_padding: f32,
    tone: Tone,
}

impl BarChartProgram {
    /// The value range, including zero so bars have a baseline to grow from.
    fn value_range(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        if self.grouped {
            for value in self.series.iter().flat_map(|series| series.values.iter()) {
                if value.is_finite() {
                    min = min.min(*value);
                    max = max.max(*value);
                }
            }
        } else {
            // A stacked chart's axis covers the stack totals, not the parts.
            for index in 0..self.category_count() {
                let total: f64 = self
                    .series
                    .iter()
                    .filter_map(|series| series.values.get(index))
                    .filter(|value| value.is_finite())
                    .sum();

                min = min.min(total.min(0.0));
                max = max.max(total.max(0.0));
            }
        }

        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }

        // Bars are read as lengths from zero, so zero is always on the axis.
        min = min.min(0.0);
        max = max.max(0.0);

        if (max - min).abs() < f64::EPSILON {
            return (0.0, 1.0);
        }

        (min, max)
    }

    /// How many categories the chart spans.
    fn category_count(&self) -> usize {
        self.labels.len().max(
            self.series
                .iter()
                .map(|s| s.values.len())
                .max()
                .unwrap_or(0),
        )
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

    /// The label for a category index.
    fn label_of(&self, index: usize) -> String {
        self.labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| index.to_string())
    }
}

impl<Message, Renderer> canvas::Program<Message, Theme, Renderer> for BarChartProgram
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
            // A redraw advances the easing ramp and asks for the frames the ramp
            // still needs.
            iced::Event::Window(iced::window::Event::RedrawRequested(_)) => {
                let was_settling = state.is_settling();
                state.advance();

                if was_settling || state.is_settling() {
                    return Some(canvas::Action::request_redraw());
                }
            }
            // Moving the pointer changes which band is hovered. The hit test
            // needs the scales, so it is resolved here, where the state is
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

        let count = self.category_count();

        if count == 0 || self.series.is_empty() {
            return vec![frame.into_geometry()];
        }

        let labels: Vec<String> = (0..count).map(|index| self.label_of(index)).collect();
        let (min, max) = self.value_range();

        let tick_values: Vec<f64> = (0..=4)
            .map(|index| min + (max - min) * f64::from(index) / 4.0)
            .collect();

        // The gutters come from the same helper the hit test uses, so the drawn
        // bands and the hovered bands always agree.
        let (left_gutter, bottom_gutter) = self.gutters();

        let area = Rectangle {
            x: left_gutter,
            y: 8.0,
            width: (bounds.width - left_gutter - 8.0).max(1.0),
            height: (bounds.height - bottom_gutter - 8.0).max(1.0),
        };

        // The band axis runs along the categories; the value axis runs across.
        let horizontal = self.alignment.is_horizontal();

        let (band_range, value_range) = if horizontal {
            (
                vec![area.y, area.y + area.height],
                vec![area.x, area.x + area.width],
            )
        } else {
            (
                vec![area.x, area.x + area.width],
                vec![area.y + area.height, area.y],
            )
        };

        let band = ScaleBand::new(labels.clone(), band_range).padding_inner(self.inner_padding);
        let value_scale = ScaleLinear::new(vec![min, max], value_range.clone());

        // The hover target was resolved in `update`, where the state is
        // mutable; `draw` only reads it.
        let _ = cursor;

        // Zero's position on the value axis; bars grow from here.
        let baseline = value_scale.tick(&0.0).unwrap_or(value_range[0]);
        let zero_is_visible = min <= 0.0 && max >= 0.0;

        let grid = grid_color(kit_theme);
        let label_color = label_color(kit_theme);

        // Grid lines at the value ticks.
        if self.show_grid {
            for value in &tick_values {
                let Some(position) = value_scale.tick(value) else {
                    continue;
                };

                let (from, to) = if horizontal {
                    (
                        Point::new(position, area.y),
                        Point::new(position, area.y + area.height),
                    )
                } else {
                    (
                        Point::new(area.x, position),
                        Point::new(area.x + area.width, position),
                    )
                };

                axis::draw_dashed_line(
                    &mut frame,
                    from,
                    to,
                    Stroke::default().with_color(grid).with_width(1.0),
                    4.0,
                    3.0,
                );
            }
        }

        // Value axis labels.
        if self.show_value_axis {
            for value in &tick_values {
                let Some(position) = value_scale.tick(value) else {
                    continue;
                };

                let (position, align) = if horizontal {
                    (
                        Point::new(position, area.y + area.height + AXIS_GAP + 6.0),
                        iced::alignment::Horizontal::Center,
                    )
                } else {
                    (
                        Point::new(area.x - AXIS_GAP, position),
                        iced::alignment::Horizontal::Right,
                    )
                };

                frame.fill_text(canvas::Text {
                    content: format_tick(*value),
                    position,
                    color: label_color,
                    size: Pixels(LABEL_SIZE),
                    align_x: align.into(),
                    align_y: iced::alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }

        // Category labels along the band axis.
        if self.show_x_axis {
            for (index, label_text) in labels.iter().enumerate() {
                let Some(position) = band.tick(label_text) else {
                    continue;
                };

                let (position, align) = if horizontal {
                    (
                        Point::new(area.x - AXIS_GAP, position + band.band_width() / 2.0),
                        iced::alignment::Horizontal::Right,
                    )
                } else {
                    (
                        Point::new(
                            position + band.band_width() / 2.0,
                            area.y + area.height + AXIS_GAP + 6.0,
                        ),
                        iced::alignment::Horizontal::Center,
                    )
                };

                let _ = index;

                frame.fill_text(canvas::Text {
                    content: label_text.clone(),
                    position,
                    color: label_color,
                    size: Pixels(LABEL_SIZE),
                    align_x: align.into(),
                    align_y: iced::alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }

        // The bars.
        let series_count = self.series.len();
        let band_width = band.band_width();
        let focus_strength = state.strength();
        let focused_category = state.current();

        for (index, label_text) in labels.iter().enumerate() {
            let Some(band_start) = band.tick(label_text) else {
                continue;
            };

            let dimmed = focused_category.is_some() && focus_strength > 0.0;
            let is_focused = focused_category == Some(index);

            // A stacked chart draws every series in the same slot; a grouped one
            // divides the band between them.
            let (slot_start, slot_width) = if self.grouped {
                let slot = band_width / series_count as f32;

                (band_start, slot)
            } else {
                (band_start, band_width)
            };

            // Running total for the stacked case.
            let mut stack_positive = 0.0_f64;
            let mut stack_negative = 0.0_f64;

            for (series_index, series) in self.series.iter().enumerate() {
                let Some(value) = series.values.get(index).copied() else {
                    continue;
                };

                if !value.is_finite() {
                    continue;
                }

                let color = self.color_of(series_index, kit_theme);
                let color = if dimmed && !is_focused {
                    Color { a: 0.3, ..color }
                } else {
                    color
                };

                let thickness = if self.grouped { slot_width } else { band_width };

                let center = if self.grouped {
                    slot_start + slot_width * (series_index as f32 + 0.5)
                } else {
                    band_start + band_width / 2.0
                };

                // The bar's far end, in data terms: the running total for a
                // stack, or the value itself for a group.
                let (from_value, to_value) = if self.grouped {
                    (0.0, value)
                } else if value >= 0.0 {
                    let from = stack_positive;
                    stack_positive += value;
                    (from, stack_positive)
                } else {
                    let from = stack_negative;
                    stack_negative += value;
                    (from, stack_negative)
                };

                // Both ends must be finite for a bar to be drawable; a stack
                // that overflows would otherwise draw a bar across the chart.
                let (Some(from), Some(to)) =
                    (value_scale.tick(&from_value), value_scale.tick(&to_value))
                else {
                    continue;
                };

                // Corners round the growing end, so the baseline stays square.
                let corners = if self.grouped {
                    self.corners.clamped(thickness, (to - from).abs())
                } else {
                    // Only the outermost segment of a stack gets rounded.
                    let is_last = if value >= 0.0 {
                        (stack_positive - value).abs() < f64::EPSILON
                            || series_index + 1 == series_count
                    } else {
                        series_index + 1 == series_count
                    };

                    if is_last {
                        self.corners.clamped(thickness, (to - from).abs())
                    } else {
                        Corners::default()
                    }
                };

                let _ = baseline;

                if horizontal {
                    shape::draw_bar_horizontal(
                        &mut frame,
                        center,
                        from,
                        to,
                        thickness,
                        corners.top_left.max(corners.top_right),
                        color,
                    );
                } else {
                    shape::draw_bar_rounded(
                        &mut frame, center, from, to, thickness, corners, color,
                    );
                }

                // A value printed at the bar's end, for charts where the exact
                // number matters more than the shape.
                if self.show_values && (!dimmed || is_focused) {
                    let text_position = if horizontal {
                        Point::new(to + 4.0, center)
                    } else {
                        Point::new(center, to - 8.0)
                    };

                    frame.fill_text(canvas::Text {
                        content: format_tick(value),
                        position: text_position,
                        color: colors.foreground,
                        size: Pixels(LABEL_SIZE),
                        align_x: if horizontal {
                            iced::alignment::Horizontal::Left.into()
                        } else {
                            iced::alignment::Horizontal::Center.into()
                        },
                        align_y: iced::alignment::Vertical::Center,
                        font: Font::DEFAULT,
                        ..canvas::Text::default()
                    });
                }
            }
        }

        // The baseline, drawn on top of the bars so the zero line stays visible.
        if zero_is_visible {
            let (from, to) = if horizontal {
                (
                    Point::new(baseline, area.y),
                    Point::new(baseline, area.y + area.height),
                )
            } else {
                (
                    Point::new(area.x, baseline),
                    Point::new(area.x + area.width, baseline),
                )
            };

            frame.stroke(
                &iced::widget::canvas::Path::line(from, to),
                Stroke::default()
                    .with_color(colors.muted_foreground)
                    .with_width(1.0),
            );
        }

        axis::draw_axis_line(
            &mut frame,
            area,
            crate::widgets::plot::axis_color(kit_theme),
        );

        // The tooltip.
        if let (Some(index), true) = (state.current(), focus_strength > 0.0) {
            let content = self.tooltip_content(index, kit_theme);
            // Centred in the plot area: the bands move, the plot does not, so
            // one anchor keeps the tooltip from jumping between categories.
            let anchor = Point::new(area.x + area.width / 2.0, area.y + area.height / 2.0);

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

impl BarChartProgram {
    /// Which category band the cursor is over, if any.
    ///
    /// `update` calls this to set the hover target; the geometry is recomputed
    /// rather than cached, which is cheap and keeps the two call sites from
    /// disagreeing about where the bands are.
    fn hit_test(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let count = self.category_count();

        if count == 0 || !self.show_tooltip {
            return None;
        }

        let horizontal = self.alignment.is_horizontal();
        let (left_gutter, bottom_gutter) = self.gutters();

        let area = Rectangle {
            x: left_gutter,
            y: 8.0,
            width: (bounds.width - left_gutter - 8.0).max(1.0),
            height: (bounds.height - bottom_gutter - 8.0).max(1.0),
        };

        let absolute_area = Rectangle {
            x: bounds.x + area.x,
            y: bounds.y + area.y,
            width: area.width,
            height: area.height,
        };

        let labels: Vec<String> = (0..count).map(|index| self.label_of(index)).collect();

        let absolute_band_range = if horizontal {
            vec![area.y + bounds.y, area.y + area.height + bounds.y]
        } else {
            vec![area.x + bounds.x, area.x + area.width + bounds.x]
        };

        let band = ScaleBand::new(labels, absolute_band_range).padding_inner(self.inner_padding);

        focus_at(cursor.position_over(bounds), absolute_area, |along| {
            band.least_index(along)
        })
        .filter(|index| *index < count)
    }

    /// The gutter each axis needs, in frame-local pixels.
    ///
    /// Which gutter holds the category labels depends on the alignment: a
    /// vertical chart labels its categories along the bottom, a horizontal one
    /// labels them down the left. Measuring only the value labels left the
    /// horizontal case with too little room, so its category names were clipped.
    fn gutters(&self) -> (f32, f32) {
        let (min, max) = self.value_range();
        let tick_values: Vec<f64> = (0..=4)
            .map(|index| min + (max - min) * f64::from(index) / 4.0)
            .collect();

        let horizontal = self.alignment.is_horizontal();
        let count = self.category_count();

        // The value labels always sit beside the value axis: on the left for a
        // vertical chart, along the bottom for a horizontal one.
        let mut value_axis = PlotAxis::new().label_side(AxisLabelSide::Left);
        for value in &tick_values {
            value_axis = value_axis.label(AxisText::new(
                format_tick(*value),
                Point::ORIGIN,
                Color::BLACK,
            ));
        }

        let value_inset = AxisInset::from_labels(&value_axis, LABEL_SIZE, false);

        // The category labels sit in the other gutter.
        let mut category_axis = PlotAxis::new();
        for index in 0..count {
            category_axis = category_axis.label(AxisText::new(
                self.label_of(index),
                Point::ORIGIN,
                Color::BLACK,
            ));
        }

        let category_inset = AxisInset::from_labels(&category_axis, LABEL_SIZE, false);

        let (left, bottom) = if horizontal {
            (
                if self.show_x_axis {
                    category_inset.left
                } else {
                    8.0
                },
                if self.show_value_axis {
                    LABEL_SIZE * 1.6
                } else {
                    8.0
                },
            )
        } else {
            (
                if self.show_value_axis {
                    value_inset.left
                } else {
                    8.0
                },
                if self.show_x_axis {
                    LABEL_SIZE * 1.6
                } else {
                    8.0
                },
            )
        };

        (left.max(8.0), bottom.max(8.0))
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

            content = content.row(
                series.name.clone(),
                format_tick(*value),
                self.color_of(series_index, theme),
            );
        }

        // A stack's total is the number the reader wants, so it gets its own row.
        if !self.grouped && self.series.len() > 1 {
            content = content.row("Total", format_tick(total), theme.colors().foreground);
        }

        content
    }
}

#[cfg(test)]
mod tests {
    use super::{BarAlignment, BarChart, BarSeries};
    use crate::theme::Theme;
    use crate::widgets::display::Tone;
    use crate::widgets::plot::shape::Corners;
    use iced::Color;

    /// Only the type matters; no handler is ever invoked in these tests.
    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    fn labels() -> Vec<String> {
        ["Q1", "Q2", "Q3", "Q4"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    fn program_of(chart: BarChart) -> super::BarChartProgram {
        chart.into_program()
    }

    #[test]
    fn a_bar_chart_renders() {
        let element: iced::Element<'_, Message, Theme> =
            BarChart::single("Revenue", labels(), vec![120.0, 180.0, 150.0, 210.0]).into_element();

        drop(element);
    }

    #[test]
    fn a_bar_chart_renders_in_every_alignment() {
        for alignment in [
            BarAlignment::Bottom,
            BarAlignment::Top,
            BarAlignment::Left,
            BarAlignment::Right,
        ] {
            let element: iced::Element<'_, Message, Theme> =
                BarChart::single("Revenue", labels(), vec![120.0, 180.0, 150.0, 210.0])
                    .alignment(alignment)
                    .into_element();
            drop(element);
        }
    }

    #[test]
    fn only_left_and_right_are_horizontal() {
        assert!(BarAlignment::Left.is_horizontal());
        assert!(BarAlignment::Right.is_horizontal());
        assert!(!BarAlignment::Bottom.is_horizontal());
        assert!(!BarAlignment::Top.is_horizontal());
    }

    #[test]
    fn grouped_and_stacked_both_render() {
        let grouped: iced::Element<'_, Message, Theme> = BarChart::new(
            labels(),
            vec![
                BarSeries::new("Revenue", vec![120.0, 180.0, 150.0, 210.0]),
                BarSeries::new("Cost", vec![80.0, 90.0, 85.0, 100.0]),
            ],
        )
        .stacked(false)
        .into_element();
        drop(grouped);

        let stacked: iced::Element<'_, Message, Theme> = BarChart::new(
            labels(),
            vec![
                BarSeries::new("Revenue", vec![120.0, 180.0, 150.0, 210.0]),
                BarSeries::new("Cost", vec![80.0, 90.0, 85.0, 100.0]),
            ],
        )
        .stacked(true)
        .into_element();
        drop(stacked);
    }

    #[test]
    fn the_axis_always_includes_zero() {
        // Bars are read as lengths from zero; an axis excluding it would make
        // the bar heights meaningless.
        let program = program_of(BarChart::single(
            "All positive",
            labels(),
            vec![100.0, 120.0, 110.0, 130.0],
        ));

        assert_eq!(program.value_range().0, 0.0);
    }

    #[test]
    fn negative_values_extend_the_axis_below_zero() {
        let program = program_of(BarChart::single(
            "Change",
            labels(),
            vec![-40.0, 25.0, -10.0, 60.0],
        ));

        let (min, max) = program.value_range();
        assert!(min < 0.0, "the axis reaches the negative values");
        assert!(max > 0.0);
    }

    #[test]
    fn a_stacked_axis_covers_the_totals_not_the_parts() {
        // Using the largest part instead of the largest total would clip the
        // top of every stack.
        let grouped = program_of(
            BarChart::new(
                labels(),
                vec![
                    BarSeries::new("A", vec![100.0, 100.0, 100.0, 100.0]),
                    BarSeries::new("B", vec![100.0, 100.0, 100.0, 100.0]),
                ],
            )
            .stacked(false),
        );

        let stacked = program_of(
            BarChart::new(
                labels(),
                vec![
                    BarSeries::new("A", vec![100.0, 100.0, 100.0, 100.0]),
                    BarSeries::new("B", vec![100.0, 100.0, 100.0, 100.0]),
                ],
            )
            .stacked(true),
        );

        assert_eq!(
            grouped.value_range().1,
            100.0,
            "grouped bars share the axis"
        );
        assert_eq!(stacked.value_range().1, 200.0, "the stack totals 200");
    }

    #[test]
    fn multi_series_get_distinct_colors() {
        let program = program_of(BarChart::new(
            labels(),
            vec![
                BarSeries::new("A", vec![1.0, 2.0, 3.0, 4.0]),
                BarSeries::new("B", vec![4.0, 3.0, 2.0, 1.0]),
                BarSeries::new("C", vec![2.0, 2.0, 2.0, 2.0]),
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
    }

    #[test]
    fn every_option_combination_renders() {
        let element: iced::Element<'_, Message, Theme> =
            BarChart::single("Revenue", labels(), vec![120.0, 180.0, 150.0, 210.0])
                .corners(Corners::all(2.0))
                .padding(0.4)
                .values_visible(true)
                .grid(false)
                .x_axis(false)
                .value_axis(false)
                .tooltip(false)
                .height(300.0)
                .tone(Tone::Success)
                .into_element();

        drop(element);
    }

    #[test]
    fn an_empty_chart_renders_without_panicking() {
        let no_data: iced::Element<'_, Message, Theme> =
            BarChart::new(Vec::new(), Vec::new()).into_element();
        drop(no_data);

        let no_series: iced::Element<'_, Message, Theme> =
            BarChart::new(labels(), Vec::new()).into_element();
        drop(no_series);

        let empty_values: iced::Element<'_, Message, Theme> =
            BarChart::new(labels(), vec![BarSeries::new("Empty", Vec::new())]).into_element();
        drop(empty_values);
    }

    #[test]
    fn a_flat_series_gets_a_usable_axis() {
        let program = program_of(BarChart::single("Flat", labels(), vec![0.0, 0.0, 0.0, 0.0]));

        let (min, max) = program.value_range();
        assert!(max > min, "an all-zero chart still spans a range");
    }

    #[test]
    fn non_finite_values_are_excluded() {
        let program = program_of(BarChart::single(
            "Bad",
            labels(),
            vec![f64::NAN, 10.0, f64::INFINITY, 20.0],
        ));

        let (min, max) = program.value_range();
        assert!(min.is_finite() && max.is_finite());
        assert!(max >= 20.0);
    }

    #[test]
    fn mismatched_series_lengths_render() {
        // Shorter and longer series are both common caller mistakes.
        let short: iced::Element<'_, Message, Theme> =
            BarChart::new(labels(), vec![BarSeries::new("Short", vec![1.0, 2.0])]).into_element();
        drop(short);

        let long: iced::Element<'_, Message, Theme> = BarChart::new(
            vec!["One".to_owned()],
            vec![BarSeries::new("Long", vec![1.0, 2.0, 3.0, 4.0, 5.0])],
        )
        .into_element();
        drop(long);
    }

    #[test]
    fn a_series_with_an_explicit_color_keeps_it() {
        let program = program_of(BarChart::new(
            labels(),
            vec![BarSeries::new("A", vec![1.0, 2.0, 3.0, 4.0]).color(Color::WHITE)],
        ));

        assert_eq!(program.color_of(0, &Theme::light()), Color::WHITE);
    }

    #[test]
    fn series_metadata_is_reported() {
        let series = BarSeries::new("Revenue", vec![1.0, 2.0]).tone(Tone::Warning);

        assert_eq!(series.name(), "Revenue");
        assert_eq!(series.values(), &[1.0, 2.0]);
    }
}
