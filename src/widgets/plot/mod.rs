//! Chart plotting infrastructure.
//!
//! Ported from `gpui-kit`'s `plot` module (Apache-2.0), adapted to iced's
//! canvas API. The pieces here are shared by every chart type:
//!
//! - [`scale`] maps between data values and screen positions, in both
//!   directions — the reverse direction is what makes tooltips possible.
//! - [`axis`] places axis labels and grid lines, and reserves the space they
//!   need.
//! - [`tooltip`] tracks which datum the cursor is over, eases the highlight,
//!   and draws the tooltip box.
//! - [`shape`] draws the marks themselves (lines, bars, areas, arcs).
//!
//! A chart type composes these: compute scales from the data, build axes, then
//! draw shapes inside the plot area they leave behind.
//!
//! See `NOTICE` at the repository root for attribution.

pub mod area_chart;
pub mod axis;
pub mod bar_chart;
pub mod candlestick_chart;
pub mod line_chart;
pub mod pie_chart;
pub mod radar_chart;
pub mod sankey_chart;
pub mod scale;
pub mod shape;
pub mod tooltip;

pub use area_chart::{AreaChart, AreaSeries};
pub use axis::{format_tick, ticks, AxisInset, AxisLabelSide, AxisText, PlotAxis};
pub use bar_chart::{BarAlignment, BarChart, BarSeries};
pub use candlestick_chart::{Candle, CandlestickChart};
pub use line_chart::{LineChart, LineSeries};
pub use pie_chart::{PieChart, Slice as PieSlice};
pub use radar_chart::{RadarChart, RadarSeries};
pub use sankey_chart::{SankeyAlign, SankeyChart, SankeyLink, SankeyNode, SankeyValueScale};
pub use scale::{Numeric, Scale, ScaleBand, ScaleLinear, ScaleOrdinal, ScalePoint};
pub use tooltip::{Focus, TooltipContent, TooltipRow};

use crate::theme::Theme;
use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle};

/// The inset a plot reserves for its axes by default.
pub const DEFAULT_INSET: f32 = 8.0;

/// The color a chart uses for its grid lines.
#[must_use]
pub fn grid_color(theme: &Theme) -> Color {
    theme.colors().border
}

/// The color a chart uses for its axis lines.
#[must_use]
pub fn axis_color(theme: &Theme) -> Color {
    theme.colors().border
}

/// The color a chart uses for its axis labels.
#[must_use]
pub fn label_color(theme: &Theme) -> Color {
    theme.colors().muted_foreground
}

/// Draws a plot's background and border, so a chart reads as a bounded area.
pub fn draw_plot_area<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    area: Rectangle,
    theme: &Theme,
) {
    let colors = theme.colors();

    frame.fill_rectangle(
        Point::new(area.x, area.y),
        iced::Size::new(area.width, area.height),
        Color {
            a: 0.35,
            ..colors.muted
        },
    );

    frame.stroke(
        &Path::rectangle(
            Point::new(area.x, area.y),
            iced::Size::new(area.width, area.height),
        ),
        Stroke::default().with_color(colors.border).with_width(1.0),
    );
}

/// The color palette a chart cycles through for multi-series data.
///
/// Series beyond the palette's length wrap, so an arbitrary number of series
/// can be drawn; the first entries are the semantic roles, since those are the
/// ones a reader most often distinguishes.
#[must_use]
pub fn series_palette(theme: &Theme) -> Vec<Color> {
    let colors = theme.colors();

    vec![
        colors.primary,
        Color::from_rgb8(0x0e, 0xa5, 0xe9),
        Color::from_rgb8(0x8b, 0x5c, 0xf6),
        Color::from_rgb8(0x14, 0xb8, 0xa6),
        Color::from_rgb8(0xf5, 0x9e, 0x0b),
        colors.destructive,
    ]
}

/// Picks the palette entry for a series index, wrapping as needed.
#[must_use]
pub fn series_color(theme: &Theme, index: usize) -> Color {
    let palette = series_palette(theme);

    if palette.is_empty() {
        return theme.colors().foreground;
    }

    palette[index % palette.len()]
}

/// Builds a colour-swatch legend: one entry per labelled colour.
///
/// A legend is a normal layout widget rather than part of a chart's canvas, so
/// it can be placed wherever the surrounding design needs it — beside a pie, or
/// under a line chart.
#[must_use]
pub fn legend<'a, Message: 'a>(entries: &'a [(String, Color)]) -> Element<'a, Message, Theme> {
    let style = crate::theme::Size::Sm.text();

    let mut row = iced::widget::row![].spacing(16);

    for (label, color) in entries {
        // The swatch is pinned to the text's line height so entries of
        // different lengths still align on their baselines.
        let swatch = iced::widget::container(iced::widget::Space::new())
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .class({
                let color = *color;
                Box::new(move |theme: &Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(color)),
                    border: iced::Border {
                        color: theme.colors().border,
                        width: 1.0,
                        radius: 5.0.into(),
                    },
                    ..iced::widget::container::Style::default()
                }) as iced::widget::container::StyleFn<'a, Theme>
            });

        row = row.push(
            iced::widget::row![
                swatch,
                iced::widget::text(label.clone())
                    .size(style.size)
                    .line_height(style.line_height()),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        );
    }

    row.wrap().into()
}

/// A placeholder for a chart with nothing to draw.
///
/// A chart with no data otherwise renders as blank space, which reads as a
/// rendering bug rather than as "there is nothing here".
#[must_use]
pub fn empty_plot<'a, Message: 'a>(height: f32) -> Element<'a, Message, Theme> {
    let style = crate::theme::Size::Sm.text();

    iced::widget::container(
        iced::widget::text("No data")
            .size(style.size)
            .class(Box::new(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.colors().muted_foreground),
            }) as iced::widget::text::StyleFn<'a, Theme>),
    )
    .width(Length::Fill)
    .height(Length::Fixed(height.max(1.0)))
    .center_x(Length::Fill)
    .center_y(Length::Fixed(height.max(1.0)))
    .into()
}

#[cfg(test)]
mod tests {
    use super::{series_color, series_palette};
    use crate::theme::Theme;

    #[test]
    fn the_palette_is_non_empty() {
        for theme in [Theme::light(), Theme::dark()] {
            assert!(!series_palette(&theme).is_empty());
        }
    }

    #[test]
    fn series_colors_wrap_for_many_series() {
        let theme = Theme::light();
        let palette_len = series_palette(&theme).len();

        // A series past the palette's length reuses an entry rather than
        // panicking or drawing nothing.
        assert_eq!(
            series_color(&theme, palette_len),
            series_color(&theme, 0),
            "the palette wraps"
        );
        assert_eq!(
            series_color(&theme, palette_len * 3 + 2),
            series_color(&theme, 2)
        );
    }

    #[test]
    fn the_light_and_dark_palettes_differ() {
        let light = series_palette(&Theme::light());
        let dark = series_palette(&Theme::dark());

        assert_ne!(light[0], dark[0]);
    }

    #[test]
    fn the_first_palette_entry_is_the_brand_color() {
        let theme = Theme::light();
        assert_eq!(series_color(&theme, 0), theme.colors().primary);
    }
}
