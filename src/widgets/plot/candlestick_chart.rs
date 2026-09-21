//! Candlestick charts.
//!
//! Ported from `gpui-kit`'s `CandlestickChart` (Apache-2.0), rebuilt on iced's
//! canvas.
//!
//! # Bullish and bearish
//!
//! "Bullish" means the close is at or above the open: the period gained. The
//! convention differs by market — in much of Asia a rising candle is red — so
//! both colors are settable rather than fixed. [`Candle::bullish`] reports
//! which way a given candle reads.

use crate::theme::Theme;
use crate::widgets::plot::axis::{AXIS_GAP, LABEL_SIZE};
use crate::widgets::plot::shape;
use crate::widgets::plot::tooltip::{draw_tooltip, TooltipContent};
use crate::widgets::plot::{
    axis, format_tick, grid_color, label_color, AxisInset, AxisLabelSide, AxisText, PlotAxis,
    Scale, ScaleBand, ScaleLinear,
};
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Stroke};
use iced::{mouse, Color, Element, Font, Length, Pixels, Point, Rectangle};

/// One period's price range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle {
    /// The opening price.
    pub open: f64,
    /// The highest price in the period.
    pub high: f64,
    /// The lowest price in the period.
    pub low: f64,
    /// The closing price.
    pub close: f64,
}

impl Candle {
    /// Creates a candle from its four prices.
    #[must_use]
    pub fn new(open: f64, high: f64, low: f64, close: f64) -> Self {
        Self {
            open,
            high,
            low,
            close,
        }
    }

    /// Whether the period gained.
    ///
    /// A candle that opened and closed at the same price counts as bullish,
    /// which is the common convention: it did not fall.
    #[must_use]
    pub fn bullish(&self) -> bool {
        self.close >= self.open
    }

    /// Whether every price is a finite number.
    ///
    /// A candle with a non-finite price cannot be drawn; the chart skips it.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.open.is_finite()
            && self.high.is_finite()
            && self.low.is_finite()
            && self.close.is_finite()
    }
}

/// A candlestick chart.
///
/// The chart holds several independent display toggles, which is what the lint
/// below flags; grouping them into a sub-struct would only add indirection.
#[must_use = "a CandlestickChart does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct CandlestickChart {
    labels: Vec<String>,
    candles: Vec<Candle>,
    body_width_ratio: f32,
    bullish_color: Option<Color>,
    bearish_color: Option<Color>,
    show_grid: bool,
    show_x_axis: bool,
    show_value_axis: bool,
    show_tooltip: bool,
    height: f32,
}

impl CandlestickChart {
    /// Creates a chart from labelled candles.
    pub fn new(labels: Vec<String>, candles: Vec<Candle>) -> Self {
        Self {
            labels,
            candles,
            // Leaves a gap between candles, which is what separates them.
            body_width_ratio: 0.7,
            bullish_color: None,
            bearish_color: None,
            show_grid: true,
            show_x_axis: true,
            show_value_axis: true,
            show_tooltip: true,
            height: 260.0,
        }
    }

    /// Sets the body width as a fraction of the available slot.
    pub fn body_width_ratio(mut self, ratio: f32) -> Self {
        self.body_width_ratio = ratio.clamp(0.05, 1.0);
        self
    }

    /// Sets the color of a rising candle.
    ///
    /// Override this when the target market reads a rise as red rather than
    /// green.
    pub fn bullish(mut self, color: Color) -> Self {
        self.bullish_color = Some(color);
        self
    }

    /// Sets the color of a falling candle.
    pub fn bearish(mut self, color: Color) -> Self {
        self.bearish_color = Some(color);
        self
    }

    /// Turns the grid lines on or off.
    pub fn grid(mut self, grid: bool) -> Self {
        self.show_grid = grid;
        self
    }

    /// Turns the period labels on or off.
    pub fn x_axis(mut self, visible: bool) -> Self {
        self.show_x_axis = visible;
        self
    }

    /// Turns the price axis labels on or off.
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

    /// Moves the builder into the canvas program that draws it.
    fn into_program(self) -> CandlestickProgram {
        CandlestickProgram {
            labels: self.labels,
            candles: self.candles,
            body_width_ratio: self.body_width_ratio,
            bullish_color: self.bullish_color,
            bearish_color: self.bearish_color,
            show_grid: self.show_grid,
            show_x_axis: self.show_x_axis,
            show_value_axis: self.show_value_axis,
            show_tooltip: self.show_tooltip,
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

impl<Message: 'static> From<CandlestickChart> for Element<'static, Message, Theme> {
    fn from(chart: CandlestickChart) -> Self {
        chart.into_element()
    }
}

/// The canvas program that draws a candlestick chart.
///
/// Mirrors the builder's display toggles.
#[allow(clippy::struct_excessive_bools)]
struct CandlestickProgram {
    labels: Vec<String>,
    candles: Vec<Candle>,
    body_width_ratio: f32,
    bullish_color: Option<Color>,
    bearish_color: Option<Color>,
    show_grid: bool,
    show_x_axis: bool,
    show_value_axis: bool,
    show_tooltip: bool,
}

impl CandlestickProgram {
    /// How many periods the chart spans.
    fn len(&self) -> usize {
        self.candles.len().max(self.labels.len())
    }

    /// The label for a period index, falling back to the index.
    fn label_of(&self, index: usize) -> String {
        self.labels
            .get(index)
            .cloned()
            .unwrap_or_else(|| index.to_string())
    }

    /// The price range the axis covers.
    ///
    /// Taken from the highs and lows, not the opens and closes: a wick that
    /// leaves the plot area is the one thing a candlestick must not do.
    fn price_range(&self) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for candle in self.candles.iter().filter(|candle| candle.is_valid()) {
            min = min.min(candle.low);
            max = max.max(candle.high);
        }

        if !min.is_finite() || !max.is_finite() {
            return (0.0, 1.0);
        }

        // A little headroom keeps the extreme wicks off the frame edge.
        let pad = (max - min).abs() * 0.04;
        let pad = if pad > 0.0 {
            pad
        } else {
            max.abs() * 0.01 + 1.0
        };

        (min - pad, max + pad)
    }

    /// The bullish and bearish colors, from the builder or the theme.
    fn colors(&self, theme: &Theme) -> (Color, Color) {
        let colors = theme.colors();

        // The theme has one `destructive` role and no gain/loss pair, so the
        // defaults are derived: a gain reads as the primary color, a loss as
        // destructive. Both are overridable for markets that invert them.
        let bullish = self.bullish_color.unwrap_or(colors.primary);
        let bearish = self.bearish_color.unwrap_or(colors.destructive);

        (bullish, bearish)
    }

    /// The gutter each axis needs, in frame-local pixels.
    fn gutters(&self) -> (f32, f32) {
        let (min, max) = self.price_range();

        let ticks: Vec<f64> = (0..=4)
            .map(|index| min + (max - min) * f64::from(index) / 4.0)
            .collect();

        let mut value_axis = PlotAxis::new().label_side(AxisLabelSide::Left);
        for value in &ticks {
            value_axis = value_axis.label(AxisText::new(
                format_tick(*value),
                Point::ORIGIN,
                Color::BLACK,
            ));
        }

        let inset = AxisInset::from_labels(&value_axis, LABEL_SIZE, false);

        (
            if self.show_value_axis {
                inset.left
            } else {
                8.0
            },
            if self.show_x_axis {
                LABEL_SIZE * 1.6
            } else {
                8.0
            },
        )
    }

    /// Builds the tooltip for a hovered period.
    fn tooltip_content(&self, index: usize, theme: &Theme) -> TooltipContent {
        let mut content = TooltipContent::new(self.label_of(index));

        let Some(candle) = self.candles.get(index) else {
            return content;
        };

        let (bullish, bearish) = self.colors(theme);
        let color = if candle.bullish() { bullish } else { bearish };

        // The change is what the reader wants first, so it leads.
        let change = candle.close - candle.open;
        let percent = if candle.open.abs() > f64::EPSILON {
            change / candle.open * 100.0
        } else {
            0.0
        };

        content = content.row("Change", format!("{change:+.2} ({percent:+.1}%)"), color);

        content
            .row(
                "Open",
                format!("{:.2}", candle.open),
                theme.colors().muted_foreground,
            )
            .row(
                "High",
                format!("{:.2}", candle.high),
                theme.colors().muted_foreground,
            )
            .row(
                "Low",
                format!("{:.2}", candle.low),
                theme.colors().muted_foreground,
            )
            .row(
                "Close",
                format!("{:.2}", candle.close),
                theme.colors().muted_foreground,
            )
    }
}

impl<Message, Renderer> canvas::Program<Message, Theme, Renderer> for CandlestickProgram
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

        let count = self.len();

        if count == 0 {
            return vec![frame.into_geometry()];
        }

        let labels: Vec<String> = (0..count).map(|index| self.label_of(index)).collect();
        let (min, max) = self.price_range();

        let ticks: Vec<f64> = (0..=4)
            .map(|index| min + (max - min) * f64::from(index) / 4.0)
            .collect();

        let (left_gutter, bottom_gutter) = self.gutters();

        let area = Rectangle {
            x: left_gutter,
            y: 8.0,
            width: (bounds.width - left_gutter - 8.0).max(1.0),
            height: (bounds.height - bottom_gutter - 8.0).max(1.0),
        };

        let band = ScaleBand::new(labels.clone(), vec![area.x, area.x + area.width])
            .padding_inner(1.0 - self.body_width_ratio);
        // The price range is reversed: a higher price must sit higher on screen.
        let price = ScaleLinear::new(vec![min, max], vec![area.y + area.height, area.y]);

        let grid = grid_color(kit_theme);
        let label = label_color(kit_theme);

        // Grid lines and the price labels.
        if self.show_grid {
            for value in &ticks {
                let Some(y) = price.tick(value) else {
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

        if self.show_value_axis {
            for value in &ticks {
                let Some(y) = price.tick(value) else {
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
        }

        let hovered = *state;
        let (bullish_color, bearish_color) = self.colors(kit_theme);
        let slot = band.band_width();

        // The hovered period's highlight band, drawn under the candles.
        if let Some(index) = hovered {
            if let Some(x) = band.tick(&labels[index]) {
                frame.fill_rectangle(
                    Point::new(x, area.y),
                    iced::Size::new(slot, area.height),
                    Color {
                        a: 0.08,
                        ..colors.foreground
                    },
                );
            }
        }

        for (index, candle) in self.candles.iter().enumerate() {
            if !candle.is_valid() {
                continue;
            }

            let Some(x) = band.tick(&labels[index]) else {
                continue;
            };

            let center = x + slot / 2.0;

            let (Some(high), Some(low)) = (price.tick(&candle.high), price.tick(&candle.low))
            else {
                continue;
            };
            let (Some(open), Some(close)) = (price.tick(&candle.open), price.tick(&candle.close))
            else {
                continue;
            };

            shape::draw_candle(
                &mut frame,
                center,
                high,
                low,
                open,
                close,
                slot,
                bullish_color,
                bearish_color,
            );
        }

        // Period labels along the bottom.
        if self.show_x_axis {
            for label_text in &labels {
                let Some(x) = band.tick(label_text) else {
                    continue;
                };

                frame.fill_text(canvas::Text {
                    content: label_text.clone(),
                    position: Point::new(x + slot / 2.0, area.y + area.height + AXIS_GAP + 6.0),
                    color: label,
                    size: Pixels(LABEL_SIZE),
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    font: Font::DEFAULT,
                    ..canvas::Text::default()
                });
            }
        }

        axis::draw_axis_line(
            &mut frame,
            area,
            crate::widgets::plot::axis_color(kit_theme),
        );

        // The tooltip.
        if let (Some(index), true) = (hovered, self.show_tooltip) {
            let content = self.tooltip_content(index, kit_theme);

            let anchor = if let Some(candle) = self.candles.get(index) {
                let x = band
                    .tick(&labels[index])
                    .map_or(area.x + area.width / 2.0, |x| x + slot / 2.0);
                let y = price
                    .tick(&candle.close)
                    .unwrap_or(area.y + area.height / 2.0);

                Point::new(x, y)
            } else {
                Point::new(area.x + area.width / 2.0, area.y + area.height / 2.0)
            };

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

impl CandlestickProgram {
    /// Which period the cursor is over, if any.
    fn hit_test(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let count = self.len();

        if count == 0 || !self.show_tooltip {
            return None;
        }

        let (left_gutter, bottom_gutter) = self.gutters();

        let area = Rectangle {
            x: left_gutter,
            y: 8.0,
            width: (bounds.width - left_gutter - 8.0).max(1.0),
            height: (bounds.height - bottom_gutter - 8.0).max(1.0),
        };

        let absolute = Rectangle {
            x: bounds.x + area.x,
            y: bounds.y + area.y,
            width: area.width,
            height: area.height,
        };

        let labels: Vec<String> = (0..count).map(|index| self.label_of(index)).collect();

        let band = ScaleBand::new(
            labels,
            vec![area.x + bounds.x, area.x + area.width + bounds.x],
        )
        .padding_inner(1.0 - self.body_width_ratio);

        let position = cursor.position_over(bounds)?;

        if !absolute.contains(position) {
            return None;
        }

        let index = band.least_index(position.x - area.x - bounds.x);

        (index < count).then_some(index)
    }
}

/// The recommended body width ratio, exposed so callers can match their own
/// annotations to the built-in candles.
#[must_use]
pub fn default_body_width_ratio() -> f32 {
    0.7
}

/// Formats a price for an axis label.
#[must_use]
pub fn format_price(value: f64) -> String {
    format!("{value:.2}")
}

#[cfg(test)]
mod tests {
    use super::{default_body_width_ratio, format_price, Candle, CandlestickChart};
    use crate::theme::Theme;
    use iced::Color;

    /// Only the type matters; no handler is ever invoked in these tests.
    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    fn candles() -> Vec<Candle> {
        vec![
            Candle::new(100.0, 110.0, 95.0, 108.0),
            Candle::new(108.0, 112.0, 102.0, 104.0),
            Candle::new(104.0, 118.0, 103.0, 116.0),
            Candle::new(116.0, 120.0, 110.0, 112.0),
            Candle::new(112.0, 115.0, 105.0, 107.0),
        ]
    }

    fn labels() -> Vec<String> {
        ["Mon", "Tue", "Wed", "Thu", "Fri"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect()
    }

    fn program_of(chart: CandlestickChart) -> super::CandlestickProgram {
        chart.into_program()
    }

    #[test]
    fn a_candlestick_chart_renders() {
        let element: iced::Element<'_, Message, Theme> =
            CandlestickChart::new(labels(), candles()).into_element();

        drop(element);
    }

    #[test]
    fn a_rising_candle_is_bullish() {
        let candle = Candle::new(100.0, 110.0, 95.0, 108.0);
        assert!(candle.bullish());
    }

    #[test]
    fn a_falling_candle_is_bearish() {
        let candle = Candle::new(108.0, 112.0, 102.0, 104.0);
        assert!(!candle.bullish());
    }

    #[test]
    fn an_unchanged_candle_counts_as_bullish() {
        // The common convention: a period that did not fall did not fall.
        let candle = Candle::new(100.0, 105.0, 98.0, 100.0);
        assert!(candle.bullish());
    }

    #[test]
    fn the_price_range_covers_the_wicks() {
        // A wick leaving the plot area is the one thing a candlestick must not
        // do, so the axis has to reach the extremes.
        let program = program_of(CandlestickChart::new(labels(), candles()));
        let (min, max) = program.price_range();

        for candle in candles() {
            assert!(min < candle.low, "the low {} is inside {min}", candle.low);
            assert!(
                max > candle.high,
                "the high {} is inside {max}",
                candle.high
            );
        }
    }

    #[test]
    fn the_price_range_leaves_headroom() {
        // Wicks touching the frame edge read as clipped.
        let program = program_of(CandlestickChart::new(labels(), candles()));
        let (min, max) = program.price_range();

        let lowest = candles()
            .iter()
            .map(|c| c.low)
            .fold(f64::INFINITY, f64::min);
        let highest = candles()
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);

        assert!(min < lowest);
        assert!(max > highest);
    }

    #[test]
    fn a_flat_series_gets_a_usable_range() {
        // Every price identical would otherwise produce a zero-height axis.
        let flat = vec![Candle::new(100.0, 100.0, 100.0, 100.0); 5];
        let program = program_of(CandlestickChart::new(labels(), flat));
        let (min, max) = program.price_range();

        assert!(max > min, "a flat series still spans a range");
    }

    #[test]
    fn invalid_candles_are_excluded_from_the_range() {
        let mixed = vec![
            Candle::new(100.0, 110.0, 95.0, 108.0),
            Candle::new(f64::NAN, f64::NAN, f64::NAN, f64::NAN),
            Candle::new(104.0, 118.0, 103.0, 116.0),
        ];

        let program = program_of(CandlestickChart::new(labels(), mixed));
        let (min, max) = program.price_range();

        assert!(min.is_finite() && max.is_finite());
    }

    #[test]
    fn a_candle_reports_whether_its_prices_are_finite() {
        assert!(Candle::new(1.0, 2.0, 0.5, 1.5).is_valid());
        assert!(!Candle::new(f64::NAN, 2.0, 0.5, 1.5).is_valid());
        assert!(!Candle::new(1.0, f64::INFINITY, 0.5, 1.5).is_valid());
    }

    #[test]
    fn the_default_colors_differ() {
        // A chart whose gains and losses share a color is unreadable.
        let program = program_of(CandlestickChart::new(labels(), candles()));
        let (bullish, bearish) = program.colors(&Theme::light());

        assert_ne!(bullish, bearish);
    }

    #[test]
    fn the_colors_can_be_inverted_for_other_markets() {
        // In much of Asia a rising candle is red.
        let program = program_of(
            CandlestickChart::new(labels(), candles())
                .bullish(Color::from_rgb8(0xd4, 0x2c, 0x2c))
                .bearish(Color::from_rgb8(0x16, 0xa3, 0x4a)),
        );

        let (bullish, bearish) = program.colors(&Theme::light());

        assert_eq!(bullish, Color::from_rgb8(0xd4, 0x2c, 0x2c));
        assert_eq!(bearish, Color::from_rgb8(0x16, 0xa3, 0x4a));
    }

    #[test]
    fn an_empty_chart_renders_without_panicking() {
        let no_data: iced::Element<'_, Message, Theme> =
            CandlestickChart::new(Vec::new(), Vec::new()).into_element();
        drop(no_data);

        let labels_only: iced::Element<'_, Message, Theme> =
            CandlestickChart::new(labels(), Vec::new()).into_element();
        drop(labels_only);

        let candles_only: iced::Element<'_, Message, Theme> =
            CandlestickChart::new(Vec::new(), candles()).into_element();
        drop(candles_only);
    }

    #[test]
    fn a_single_candle_renders() {
        let element: iced::Element<'_, Message, Theme> = CandlestickChart::new(
            vec!["Only".to_owned()],
            vec![Candle::new(100.0, 110.0, 95.0, 108.0)],
        )
        .into_element();
        drop(element);
    }

    #[test]
    fn every_option_combination_renders() {
        let element: iced::Element<'_, Message, Theme> = CandlestickChart::new(labels(), candles())
            .body_width_ratio(0.4)
            .grid(false)
            .x_axis(false)
            .value_axis(false)
            .tooltip(false)
            .height(320.0)
            .into_element();
        drop(element);
    }

    #[test]
    fn the_body_width_ratio_is_clamped() {
        for bad in [0.0, -1.0, 5.0] {
            let chart = CandlestickChart::new(labels(), candles()).body_width_ratio(bad);
            assert!(chart.body_width_ratio > 0.0 && chart.body_width_ratio <= 1.0);
        }
    }

    #[test]
    fn the_default_body_width_leaves_a_gap() {
        // Candles that touch read as one band rather than a series.
        assert!(default_body_width_ratio() < 1.0);
        assert!(default_body_width_ratio() > 0.5);
    }

    #[test]
    fn prices_are_formatted_to_two_decimals() {
        assert_eq!(format_price(100.0), "100.00");
        assert_eq!(format_price(100.5), "100.50");
    }

    #[test]
    fn mismatched_label_and_candle_counts_render() {
        // More labels than candles, and the reverse, are both caller mistakes.
        let few_candles: iced::Element<'_, Message, Theme> =
            CandlestickChart::new(labels(), candles()[..2].to_vec()).into_element();
        drop(few_candles);

        let few_labels: iced::Element<'_, Message, Theme> =
            CandlestickChart::new(vec!["Mon".to_owned()], candles()).into_element();
        drop(few_labels);
    }
}
