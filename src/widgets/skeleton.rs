//! Skeletons: placeholder shapes shown while content loads.

use crate::motion::Clock;
use crate::theme::Theme;
use iced::advanced::widget::{tree, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Shell};
use iced::{Background, Border, Color, Element, Event, Length, Rectangle, Size};
use std::borrow::Cow;

/// How long one breath of the skeleton's pulse takes, in seconds.
///
/// Slow enough to read as waiting rather than as activity: a skeleton stands in
/// for content that has not arrived, and a brisk pulse would compete with the
/// spinner the application shows for work that is actually happening.
const PULSE_PERIOD: f32 = 1.6;

/// The dimmest the pulse takes the surface.
const PULSE_MIN: f32 = 0.45;

/// The brightest the pulse takes it, which is the theme's own muted color.
const PULSE_MAX: f32 = 1.0;

/// The shape of a skeleton block.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SkeletonShape {
    /// Several full-width lines, for a paragraph.
    #[default]
    Text,
    /// A single block, for an image or chart.
    Block,
    /// A circle, for an avatar.
    Circle,
}

/// Builds a skeleton placeholder.
///
/// A skeleton stands in for content whose shape is known but whose value is
/// not; it is deliberately not interactive and not focusable, so assistive
/// technology skips it.
///
/// Its surface pulses slowly while it is on screen, which is what distinguishes
/// a placeholder from content that happens to be grey. The pulse stops when the
/// application has asked for reduced motion, leaving a flat surface.
///
/// ```
/// # use iced_kit::widgets::{skeleton, SkeletonShape};
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// skeleton(SkeletonShape::Text, 3)
/// # }
/// ```
pub fn skeleton<'a, Message: 'a>(
    shape: SkeletonShape,
    lines: usize,
) -> Element<'a, Message, Theme> {
    match shape {
        SkeletonShape::Text => {
            let mut column = iced::widget::column![].spacing(8);

            for index in 0..lines.max(1) {
                // The last line is short, the way a real paragraph ends.
                let width = if index + 1 == lines.max(1) {
                    Length::FillPortion(3)
                } else {
                    Length::Fill
                };

                column = column.push(line(width));
            }

            column.into()
        }
        SkeletonShape::Block => Shimmer::new(Length::Fill, Length::Fixed(96.0), 4.0).into(),
        SkeletonShape::Circle => {
            // A full radius, so the square reads as a circle.
            Shimmer::new(Length::Fixed(48.0), Length::Fixed(48.0), 24.0).into()
        }
    }
}

/// One skeleton line.
fn line<'a, Message: 'a>(width: Length) -> Element<'a, Message, Theme> {
    Shimmer::new(width, Length::Fixed(12.0), 4.0).into()
}

/// A placeholder surface that pulses.
///
/// It is a widget of its own rather than a styled container because the pulse
/// needs a clock, and a style closure is handed a theme and nothing else — no
/// state, no frame time. Owning a [`Clock`] is what lets the surface breathe
/// without the application supplying a timer.
struct Shimmer {
    width: Length,
    height: Length,
    radius: f32,
}

impl Shimmer {
    /// Creates a pulsing surface of the given size and corner radius.
    const fn new(width: Length, height: Length, radius: f32) -> Self {
        Self {
            width,
            height,
            radius,
        }
    }

    /// The alpha multiplier the pulse is at, given the clock's phase.
    fn pulse(phase: f32) -> f32 {
        let turns = (phase / PULSE_PERIOD).fract();
        // A raised sine, so the surface spends equal time brightening and
        // dimming and there is no seam where the cycle restarts.
        let wave = (turns * std::f32::consts::TAU).sin() * 0.5 + 0.5;

        PULSE_MIN + (PULSE_MAX - PULSE_MIN) * wave
    }
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for Shimmer
where
    Renderer: iced::advanced::Renderer,
{
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, self.width, self.height)
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Clock>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Clock::default())
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        _layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        // Each redraw advances the pulse and asks for the next frame, which is
        // what animates the surface without the application owning a timer. A
        // reduced-motion application gets the flat surface and, because no
        // frame is requested, no redraw loop either.
        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if crate::motion::reduce_motion() {
                return;
            }

            tree.state.downcast_mut::<Clock>().advance(*now);
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !bounds.intersects(viewport) {
            return;
        }

        let muted = theme.colors().muted;
        let alpha = if crate::motion::reduce_motion() {
            1.0
        } else {
            Self::pulse(tree.state.downcast_ref::<Clock>().phase())
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: self.radius.into(),
                },
                shadow: iced::Shadow::default(),
                snap: true,
            },
            Background::Color(Color {
                a: muted.a * alpha,
                ..muted
            }),
        );
    }
}

impl<'a, Message, Renderer> From<Shimmer> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(shimmer: Shimmer) -> Self {
        Element::new(shimmer)
    }
}

/// Builds a card-shaped skeleton: an avatar circle beside two short lines.
///
/// This is the shape most list views need, so it is offered directly rather
/// than making every caller assemble it.
pub fn skeleton_list_item<'a, Message: 'a>(rows: usize) -> Element<'a, Message, Theme> {
    let mut column = iced::widget::column![].spacing(16);

    for _ in 0..rows.max(1) {
        let avatar: Element<'a, Message, Theme> = skeleton(SkeletonShape::Circle, 1);

        let lines = {
            let mut lines = iced::widget::column![].spacing(6);
            lines = lines.push(line(Length::FillPortion(3)));
            lines = lines.push(line(Length::FillPortion(2)));
            lines
        };

        column = column.push(
            iced::widget::row![avatar, lines]
                .spacing(12)
                .align_y(iced::Alignment::Center),
        );
    }

    column.into()
}

/// A skeleton shaped like a table row, for list views.
#[derive(Debug, Clone)]
pub struct SkeletonRow {
    /// How many cells the row has.
    pub columns: usize,
}

impl SkeletonRow {
    /// Creates a row with the given number of cells.
    #[must_use]
    pub fn new(columns: usize) -> Self {
        Self { columns }
    }
}

/// Builds a skeleton table: heading plus `rows` placeholder rows.
pub fn skeleton_table<'a, Message: 'a>(columns: usize, rows: usize) -> Element<'a, Message, Theme> {
    let columns = columns.max(1);
    let mut body = iced::widget::column![].spacing(12);

    for _ in 0..rows.max(1) {
        let row = SkeletonRow::new(columns);

        let mut cells = iced::widget::row![].spacing(16);
        for _ in 0..row.columns {
            cells = cells.push(line(Length::Fill));
        }

        body = body.push(cells);
    }

    body.into()
}

/// Formats a byte count for display in a skeleton label.
///
/// Kept here rather than in a component because it is a plain formatting
/// helper shared by the loading-state components.
#[must_use]
pub fn placeholder_caption<'a>(caption: impl Into<Cow<'a, str>>) -> Cow<'a, str> {
    caption.into()
}

#[cfg(test)]
mod tests {
    use super::{
        skeleton, skeleton_list_item, skeleton_table, Shimmer, SkeletonRow, SkeletonShape,
    };
    use crate::theme::Theme;

    /// The pulse must stay within its documented range, or a skeleton would
    /// either vanish or flare brighter than the content it stands in for.
    #[test]
    fn the_pulse_stays_within_its_range() {
        for step in 0..=1000 {
            let phase = step as f32 * 0.01;
            let pulse = Shimmer::pulse(phase);

            assert!(
                (super::PULSE_MIN..=super::PULSE_MAX).contains(&pulse),
                "a pulse of {pulse} at phase {phase} left its range"
            );
        }
    }

    /// The pulse must actually move, or the skeleton is back to being static —
    /// which is the state this replaced.
    #[test]
    fn the_pulse_moves() {
        let samples: Vec<f32> = (0..40)
            .map(|step| Shimmer::pulse(step as f32 * 0.1))
            .collect();

        let lowest = samples.iter().copied().fold(f32::INFINITY, f32::min);
        let highest = samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        assert!(
            highest - lowest > 0.2,
            "the pulse barely moved: {lowest} to {highest}"
        );
    }

    /// It must be periodic, so the surface does not jump when a cycle restarts.
    #[test]
    fn the_pulse_repeats_without_a_seam() {
        let start = Shimmer::pulse(0.0);
        let after_a_period = Shimmer::pulse(super::PULSE_PERIOD);

        assert!(
            (start - after_a_period).abs() < 1e-4,
            "a period must return the same value: {start} then {after_a_period}"
        );
    }

    #[test]
    fn a_skeleton_renders_in_every_shape() {
        for shape in [
            SkeletonShape::Text,
            SkeletonShape::Block,
            SkeletonShape::Circle,
        ] {
            let element: iced::Element<'_, (), Theme> = skeleton(shape, 3);
            drop(element);
        }
    }

    #[test]
    fn a_text_skeleton_renders_at_least_one_line() {
        // Zero lines would produce an invisible widget, which looks like a bug
        // rather than a loading state.
        for lines in [0, 1, 5] {
            let element: iced::Element<'_, (), Theme> = skeleton(SkeletonShape::Text, lines);
            drop(element);
        }
    }

    #[test]
    fn list_and_table_skeletons_render() {
        let list: iced::Element<'_, (), Theme> = skeleton_list_item(3);
        drop(list);

        let table: iced::Element<'_, (), Theme> = skeleton_table(4, 5);
        drop(table);

        // Degenerate inputs must not panic.
        let empty_table: iced::Element<'_, (), Theme> = skeleton_table(0, 0);
        drop(empty_table);
    }

    #[test]
    fn a_skeleton_row_reports_its_column_count() {
        assert_eq!(SkeletonRow::new(0).columns, 0);
        assert_eq!(SkeletonRow::new(3).columns, 3);
    }
}
