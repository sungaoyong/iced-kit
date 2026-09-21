//! Charts: line, bar and area, drawn with iced's canvas.
//!
//! # Scope
//!
//! This is a small plotting component for dashboards, not a general charting
//! library. It draws one or more series against a linear value axis with
//! optional grid lines and axis labels.
//!
//! # Usage
//!
//! ```
//! use iced_kit::widgets::chart::{Chart, ChartKind, Series};
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! fn view(samples: &[f32]) -> Element<'static, (), Theme> {
//!     Chart::new(vec![Series::new("Load", samples.to_vec())])
//!         .kind(ChartKind::Line)
//!         .height(200.0)
//!         .into()
//! }
//! ```

use crate::theme::Theme;
use crate::widgets::display::Tone;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Length, Point, Radians, Rectangle, Size};

/// How a series is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartKind {
    /// Connected points.
    #[default]
    Line,
    /// Vertical bars, one per sample.
    Bar,
    /// Connected points with the area beneath them filled.
    Area,
}

/// One data series.
#[derive(Debug, Clone)]
#[must_use = "a Series does nothing unless it is given to a Chart"]
pub struct Series {
    name: String,
    values: Vec<f32>,
    tone: Tone,
    color: Option<Color>,
}

impl Series {
    /// Creates a series from its values.
    pub fn new(name: impl Into<String>, values: Vec<f32>) -> Self {
        Self {
            name: name.into(),
            values,
            tone: Tone::Primary,
            color: None,
        }
    }

    /// Sets the series' color tone.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets an explicit color, overriding the tone.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The series' display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The series' values.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }
}

/// The value range shown on the vertical axis.
#[derive(Debug, Clone, Copy, PartialEq)]
enum AxisRange {
    /// Derive the range from the data.
    Auto,
    /// Use an explicit range.
    Fixed(f32, f32),
}

/// A chart.
#[must_use = "a Chart does nothing unless it is turned into an Element"]
pub struct Chart {
    series: Vec<Series>,
    kind: ChartKind,
    range: AxisRange,
    grid: bool,
    labels: bool,
    height: f32,
    padding: f32,
}

impl Chart {
    /// Creates a chart over the given series.
    pub fn new(series: Vec<Series>) -> Self {
        Self {
            series,
            kind: ChartKind::default(),
            range: AxisRange::Auto,
            grid: true,
            labels: true,
            height: 200.0,
            padding: 32.0,
        }
    }

    /// Sets how the series are drawn.
    pub fn kind(mut self, kind: ChartKind) -> Self {
        self.kind = kind;
        self
    }

    /// Fixes the vertical axis to a range.
    ///
    /// Useful for a percentage axis (0..=100) or for keeping several charts
    /// comparable, where an auto range would make each chart's shape differ.
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.range = AxisRange::Fixed(min, max);
        self
    }

    /// Turns grid lines on or off.
    pub fn grid(mut self, grid: bool) -> Self {
        self.grid = grid;
        self
    }

    /// Turns axis labels on or off.
    pub fn labels(mut self, labels: bool) -> Self {
        self.labels = labels;
        self
    }

    /// Sets the chart's height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(1.0);
        self
    }

    /// Sets the padding between the plot area and the chart's edge, in logical
    /// pixels. Must leave room for the labels.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding.max(0.0);
        self
    }

    /// Converts the chart into an [`Element`].
    ///
    /// A chart emits no messages, so the element is generic only in name; the
    /// message type is chosen by whatever it is placed inside.
    #[must_use]
    pub fn into_element<Message: 'static>(self) -> Element<'static, Message, Theme> {
        let program = ChartProgram {
            series: self.series,
            kind: self.kind,
            range: self.range,
            grid: self.grid,
            labels: self.labels,
            padding: self.padding,
        };

        Canvas::new(program)
            .width(Length::Fill)
            .height(Length::Fixed(self.height))
            .into()
    }
}

impl<Message: 'static> From<Chart> for Element<'static, Message, Theme> {
    fn from(chart: Chart) -> Self {
        chart.into_element()
    }
}

/// The canvas program that draws a chart.
struct ChartProgram {
    series: Vec<Series>,
    kind: ChartKind,
    range: AxisRange,
    grid: bool,
    labels: bool,
    padding: f32,
}

impl ChartProgram {
    /// Resolves the value range to draw.
    ///
    /// This is the single owner of the axis logic. An empty or all-NaN data set
    /// has no range, so it falls back to `0..=1` rather than producing a
    /// degenerate axis that divides by zero.
    fn resolved_range(&self) -> (f32, f32) {
        match self.range {
            AxisRange::Fixed(min, max) if max > min => (min, max),
            _ => {
                let mut min = f32::INFINITY;
                let mut max = f32::NEG_INFINITY;

                for value in self.series.iter().flat_map(|series| series.values.iter()) {
                    if value.is_finite() {
                        min = min.min(*value);
                        max = max.max(*value);
                    }
                }

                if !min.is_finite() || !max.is_finite() {
                    return (0.0, 1.0);
                }

                if (max - min).abs() < f32::EPSILON {
                    let pad = if max.abs() < 1.0 {
                        1.0
                    } else {
                        max.abs() * 0.1
                    };
                    return (min - pad, max + pad);
                }

                if self.kind == ChartKind::Bar {
                    min = min.min(0.0);
                    max = max.max(0.0);
                }

                (min, max)
            }
        }
    }
}

impl<Message, Theme, Renderer> canvas::Program<Message, Theme, Renderer> for ChartProgram
where
    Renderer: iced::advanced::graphics::geometry::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>,
    Theme: ChartTheme,
{
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        let colors = theme.chart_colors();

        let count = self
            .series
            .iter()
            .map(|series| series.values.len())
            .max()
            .unwrap_or(0);

        if count == 0 {
            return vec![frame.into_geometry()];
        }

        // The label gutter is reserved on the left and bottom.
        let gutter_left = if self.labels { self.padding } else { 4.0 };
        let gutter_bottom = if self.labels { 18.0 } else { 4.0 };

        let area = Rectangle {
            x: gutter_left,
            y: 4.0,
            width: (frame.width() - gutter_left - 8.0).max(1.0),
            height: (frame.height() - gutter_bottom - 4.0).max(1.0),
        };

        let range = self.resolved_range();

        if self.grid {
            for step in 0..=4 {
                let fraction = step as f32 / 4.0;
                let y = area.y + area.height * fraction;

                frame.stroke(
                    &Path::line(Point::new(area.x, y), Point::new(area.x + area.width, y)),
                    Stroke::default().with_color(colors.grid).with_width(1.0),
                );

                if self.labels {
                    let value = range.1 - (range.1 - range.0) * fraction;
                    frame.fill_text(canvas::Text {
                        content: format_label(value),
                        position: Point::new(area.x - 6.0, y),
                        color: colors.label,
                        size: iced::Pixels(10.0),
                        align_x: iced::advanced::text::Alignment::Right,
                        align_y: iced::alignment::Vertical::Center,
                        ..canvas::Text::default()
                    });
                }
            }
        }

        for (index, series) in self.series.iter().enumerate() {
            let color = series
                .color
                .unwrap_or_else(|| theme.series_color(series.tone, index));

            match self.kind {
                ChartKind::Bar => draw_bars(&mut frame, series, count, area, range, color),
                ChartKind::Line => draw_line(&mut frame, series, count, area, range, color, false),
                ChartKind::Area => draw_line(&mut frame, series, count, area, range, color, true),
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}

/// Draws a line, filling the area beneath it when `filled` is set.
fn draw_line<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    series: &Series,
    count: usize,
    area: Rectangle,
    range: (f32, f32),
    color: Color,
    filled: bool,
) {
    // Non-finite samples break a line into segments rather than being drawn, so
    // a gap in the data shows as a gap instead of a spike to zero.
    let segments = segments_for(series, count, area, range);

    for segment in &segments {
        if segment.len() < 2 {
            continue;
        }

        if filled {
            // A translucent wash under the line, so overlapping series stay
            // readable where they cross.
            let baseline = area.y + area.height;
            let path = Path::new(|builder| {
                builder.move_to(Point::new(segment[0].x, baseline));
                builder.line_to(segment[0]);

                for point in &segment[1..] {
                    builder.line_to(*point);
                }

                builder.line_to(Point::new(segment[segment.len() - 1].x, baseline));
                builder.close();
            });

            frame.fill(&path, Color { a: 0.18, ..color });
        }

        let line = Path::new(|builder| {
            builder.move_to(segment[0]);

            for point in &segment[1..] {
                builder.line_to(*point);
            }
        });

        frame.stroke(
            &line,
            Stroke::default()
                .with_color(color)
                .with_width(2.0)
                .with_line_cap(canvas::LineCap::Round)
                .with_line_join(canvas::LineJoin::Round),
        );
    }
}

/// Splits a series into runs of consecutive finite samples, mapped to points.
fn segments_for(
    series: &Series,
    count: usize,
    area: Rectangle,
    range: (f32, f32),
) -> Vec<Vec<Point>> {
    let (min, max) = range;
    let span = (max - min).max(f32::EPSILON);

    let mut segments: Vec<Vec<Point>> = Vec::new();
    let mut current: Vec<Point> = Vec::new();

    for (index, value) in series.values.iter().enumerate() {
        if !value.is_finite() {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }

        // A single sample sits centred rather than at the left edge.
        let x_fraction = if count <= 1 {
            0.5
        } else {
            index as f32 / (count - 1) as f32
        };
        let y_fraction = ((value - min) / span).clamp(0.0, 1.0);

        current.push(Point::new(
            area.x + area.width * x_fraction,
            area.y + area.height * (1.0 - y_fraction),
        ));
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

/// Draws bars for a series.
fn draw_bars<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    series: &Series,
    count: usize,
    area: Rectangle,
    range: (f32, f32),
    color: Color,
) {
    if count == 0 {
        return;
    }

    let baseline = if range.0 > 0.0 {
        area.y + area.height
    } else {
        // Bars grow from zero, which is where the axis crosses when the data
        // spans it.
        let span = (range.1 - range.0).max(f32::EPSILON);
        let zero_fraction = (0.0 - range.0) / span;
        area.y + area.height * (1.0 - zero_fraction.clamp(0.0, 1.0))
    };

    // Each sample gets an equal slot; the bar fills most of it, leaving a gap.
    let slot = area.width / count as f32;
    let bar_width = (slot * 0.7).max(1.0);

    for (index, value) in series.values.iter().enumerate() {
        if !value.is_finite() {
            continue;
        }

        let center_x = area.x + slot * (index as f32 + 0.5);
        let span = (range.1 - range.0).max(f32::EPSILON);
        let fraction = ((value - range.0) / span).clamp(0.0, 1.0);
        let y = area.y + area.height * (1.0 - fraction);

        let (top, height) = if y <= baseline {
            (y, baseline - y)
        } else {
            (baseline, y - baseline)
        };

        frame.fill_rectangle(
            Point::new(center_x - bar_width / 2.0, top),
            Size::new(bar_width, height.max(1.0)),
            color,
        );
    }
}

/// Formats an axis label without trailing noise.
fn format_label(value: f32) -> String {
    let magnitude = value.abs();

    if magnitude >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if magnitude >= 1_000.0 {
        format!("{:.1}k", value / 1_000.0)
    } else if magnitude >= 10.0 || value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// The theme information a chart needs.
pub trait ChartTheme {
    /// Returns the colors used for a chart's chrome.
    fn chart_colors(&self) -> ChartColors;
    /// Returns the color for the series at `index`, given its tone.
    fn series_color(&self, tone: Tone, index: usize) -> Color;
}

/// The chrome colors of a chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartColors {
    /// The grid line color.
    pub grid: Color,
    /// The axis label color.
    pub label: Color,
}

impl ChartTheme for Theme {
    fn chart_colors(&self) -> ChartColors {
        let colors = self.colors();

        ChartColors {
            grid: colors.border,
            label: colors.muted_foreground,
        }
    }

    fn series_color(&self, tone: Tone, index: usize) -> Color {
        let colors = self.colors();

        // A `Neutral` tone with several series would draw them all the same
        // color, so unnamed series cycle through the palette instead.
        match (tone, index) {
            (Tone::Neutral, 0) => colors.primary,
            (Tone::Neutral, 1) => Color::from_rgb8(0x0e, 0xa5, 0xe9),
            (Tone::Neutral, 2) => Color::from_rgb8(0x8b, 0x5c, 0xf6),
            (Tone::Neutral, _) => Color::from_rgb8(0x14, 0xb8, 0xa6),
            (other, _) => other.accent(self),
        }
    }
}

/// A legend entry for a chart series.
///
/// Charts do not draw their own legend; it is a normal layout widget so it can
/// be placed wherever the surrounding design needs it.
pub fn legend<'a, Message: 'a>(entries: &'a [(String, Tone)]) -> Element<'a, Message, Theme> {
    let style = crate::theme::Size::Sm.text();

    let mut content = iced::widget::row![].spacing(16);

    for (name, tone) in entries {
        content = content.push(
            iced::widget::row![
                iced::widget::container(iced::widget::Space::new())
                    .width(Length::Fixed(10.0))
                    .height(Length::Fixed(10.0))
                    .class(
                        Box::new(move |theme: &Theme| iced::widget::container::Style {
                            background: Some(iced::Background::Color(tone.accent(theme))),
                            border: iced::Border {
                                color: Color::TRANSPARENT,
                                width: 0.0,
                                radius: 5.0.into(),
                            },
                            ..iced::widget::container::Style::default()
                        }) as iced::widget::container::StyleFn<'a, Theme>
                    ),
                iced::widget::text(name.clone())
                    .size(style.size)
                    .line_height(style.line_height()),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );
    }

    content.wrap().into()
}

/// The stroke style a chart uses for a line, exposed for callers drawing their
/// own overlays on top of one.
#[must_use]
pub fn line_stroke(color: Color) -> Stroke<'static> {
    Stroke::default()
        .with_color(color)
        .with_width(2.0)
        .with_line_cap(canvas::LineCap::Round)
        .with_line_join(canvas::LineJoin::Round)
}

/// Draws a filled area under a polyline, for callers composing a custom chart.
pub fn area_path<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    points: &[Point],
    baseline_y: f32,
    color: Color,
) {
    if points.len() < 2 {
        return;
    }

    let path = Path::new(|builder| {
        builder.move_to(Point::new(points[0].x, baseline_y));
        builder.line_to(points[0]);

        for point in &points[1..] {
            builder.line_to(*point);
        }

        builder.line_to(Point::new(points[points.len() - 1].x, baseline_y));
        builder.close();
    });

    frame.fill(&path, color);
}

/// The number of degrees in a full turn, re-exported so chart code need not
/// import `Radians`' underlying constant.
#[must_use]
pub fn full_turn() -> Radians {
    Radians(std::f32::consts::TAU)
}

#[cfg(test)]
mod tests {
    use super::{format_label, Chart, ChartKind, ChartProgram, Series};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    #[test]
    fn a_chart_renders_in_every_kind() {
        let data = vec![1.0, 4.0, 2.0, 8.0, 5.0];

        for kind in [ChartKind::Line, ChartKind::Bar, ChartKind::Area] {
            let element: iced::Element<'static, Message, crate::theme::Theme> =
                Chart::new(vec![Series::new("Load", data.clone())])
                    .kind(kind)
                    .into_element();
            drop(element);
        }
    }

    #[test]
    fn a_chart_renders_with_several_series() {
        let element: iced::Element<'static, Message, crate::theme::Theme> = Chart::new(vec![
            Series::new("Read", vec![1.0, 2.0, 3.0]).tone(crate::widgets::Tone::Primary),
            Series::new("Write", vec![3.0, 1.0, 2.0]).tone(crate::widgets::Tone::Success),
            Series::new("Errors", vec![0.0, 1.0, 0.0]).tone(crate::widgets::Tone::Danger),
        ])
        .into_element();
        drop(element);
    }

    #[test]
    fn an_empty_chart_renders_without_panicking() {
        let element: iced::Element<'static, Message, crate::theme::Theme> =
            Chart::new(Vec::new()).into_element();
        drop(element);

        let empty_series: iced::Element<'static, Message, crate::theme::Theme> =
            Chart::new(vec![Series::new("Empty", Vec::new())]).into_element();
        drop(empty_series);
    }

    #[test]
    fn a_single_sample_chart_renders() {
        // One point cannot form a line, so this exercises the degenerate path.
        let element: iced::Element<'static, Message, crate::theme::Theme> =
            Chart::new(vec![Series::new("One", vec![42.0])]).into_element();
        drop(element);
    }

    #[test]
    fn a_flat_series_gets_a_non_degenerate_axis() {
        let program = ChartProgram {
            series: vec![Series::new("Flat", vec![5.0, 5.0, 5.0])],
            kind: ChartKind::Line,
            range: super::AxisRange::Auto,
            grid: true,
            labels: true,
            padding: 32.0,
        };

        let (min, max) = program.resolved_range();

        // A zero-height range would divide by zero when mapping points.
        assert!(max > min, "a flat series must still produce a range");
        assert!(
            min <= 5.0 && max >= 5.0,
            "the value must be inside the range"
        );
    }

    #[test]
    fn a_series_with_non_finite_values_gets_a_valid_range() {
        for values in [
            vec![f32::NAN, f32::NAN],
            vec![f32::INFINITY, f32::NEG_INFINITY],
            vec![f32::NAN, 1.0, f32::INFINITY],
        ] {
            let program = ChartProgram {
                series: vec![Series::new("Bad", values)],
                kind: ChartKind::Line,
                range: super::AxisRange::Auto,
                grid: true,
                labels: true,
                padding: 32.0,
            };

            let (min, max) = program.resolved_range();
            assert!(min.is_finite() && max.is_finite(), "range must be finite");
            assert!(max > min, "range must have height");
        }
    }

    #[test]
    fn an_explicit_range_is_used_verbatim() {
        let program = ChartProgram {
            series: vec![Series::new("Data", vec![1.0, 2.0, 3.0])],
            kind: ChartKind::Line,
            range: super::AxisRange::Fixed(0.0, 100.0),
            grid: true,
            labels: true,
            padding: 32.0,
        };

        // A fixed range is what keeps a percentage axis at 0..=100 rather than
        // rescaling to the data.
        assert_eq!(program.resolved_range(), (0.0, 100.0));
    }

    #[test]
    fn an_inverted_fixed_range_falls_back_to_the_data() {
        let program = ChartProgram {
            series: vec![Series::new("Data", vec![1.0, 2.0, 3.0])],
            kind: ChartKind::Line,
            range: super::AxisRange::Fixed(100.0, 0.0),
            grid: true,
            labels: true,
            padding: 32.0,
        };

        let (min, max) = program.resolved_range();
        assert!(max > min, "an inverted range must not be used as-is");
    }

    #[test]
    fn a_bar_chart_includes_zero_in_its_axis() {
        let program = ChartProgram {
            series: vec![Series::new("All positive", vec![10.0, 20.0, 30.0])],
            kind: ChartKind::Bar,
            range: super::AxisRange::Auto,
            grid: true,
            labels: true,
            padding: 32.0,
        };

        let (min, _) = program.resolved_range();
        // Bars are read as lengths from zero, so the axis must contain it even
        // when every value is positive.
        assert_eq!(min, 0.0);
    }

    #[test]
    fn a_line_chart_does_not_force_zero_into_the_axis() {
        let program = ChartProgram {
            series: vec![Series::new("High values", vec![100.0, 120.0, 110.0])],
            kind: ChartKind::Line,
            range: super::AxisRange::Auto,
            grid: true,
            labels: true,
            padding: 32.0,
        };

        let (min, _) = program.resolved_range();
        // Zooming in on a narrow band is what makes a line chart readable.
        assert!(min > 0.0, "a line chart should not be anchored at zero");
    }

    #[test]
    fn series_keep_their_metadata() {
        let series = Series::new("Load", vec![1.0, 2.0])
            .tone(crate::widgets::Tone::Warning)
            .color(iced::Color::WHITE);

        assert_eq!(series.name(), "Load");
        assert_eq!(series.values(), &[1.0, 2.0]);
        assert_eq!(series.color, Some(iced::Color::WHITE));
    }

    #[test]
    fn axis_labels_are_abbreviated_by_magnitude() {
        assert_eq!(format_label(0.0), "0");
        assert_eq!(format_label(5.0), "5");
        assert_eq!(format_label(1_500.0), "1.5k");
        assert_eq!(format_label(2_500_000.0), "2.5M");
        assert_eq!(format_label(0.25), "0.2");
    }

    #[test]
    fn chart_chrome_follows_the_theme() {
        use super::ChartTheme;
        use crate::theme::Theme;

        let light = Theme::light();
        let dark = Theme::dark();

        assert_eq!(light.chart_colors().grid, light.colors().border);
        assert_ne!(light.chart_colors().grid, dark.chart_colors().grid);
    }

    #[test]
    fn neutral_series_get_distinct_colors() {
        use super::ChartTheme;
        use crate::theme::Theme;
        use crate::widgets::Tone;

        let theme = Theme::light();
        let colors: Vec<_> = (0..4)
            .map(|index| theme.series_color(Tone::Neutral, index))
            .collect();

        // Four series drawn in one color would be indistinguishable.
        for (i, left) in colors.iter().enumerate() {
            for (j, right) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(left, right, "series {i} and {j} share a color");
                }
            }
        }
    }

    #[test]
    fn a_toned_series_uses_its_tone_color() {
        use super::ChartTheme;
        use crate::theme::Theme;
        use crate::widgets::Tone;

        let theme = Theme::light();
        assert_eq!(
            theme.series_color(Tone::Danger, 0),
            Tone::Danger.accent(&theme)
        );
    }

    #[test]
    fn a_chart_legend_renders() {
        use crate::widgets::Tone;

        let entries = vec![
            ("Read".to_owned(), Tone::Primary),
            ("Write".to_owned(), Tone::Success),
        ];

        let element: iced::Element<'_, Message, crate::theme::Theme> = super::legend(&entries);
        drop(element);

        let empty: iced::Element<'_, Message, crate::theme::Theme> = super::legend(&[]);
        drop(empty);
    }

    #[test]
    fn chart_geometry_helpers_are_usable() {
        let stroke = super::line_stroke(iced::Color::WHITE);
        assert_eq!(stroke.width, 2.0);
        assert_eq!(super::full_turn().0, std::f32::consts::TAU);
    }
}
