//! Skeletons: placeholder shapes shown while content loads.

use crate::theme::Theme;
use iced::widget::container;
use iced::{Color, Element, Length};
use std::borrow::Cow;

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
        SkeletonShape::Block => {
            let block: Element<'a, Message, Theme> =
                container(iced::widget::Space::new().height(Length::Fixed(96.0)))
                    .width(Length::Fill)
                    .class(Box::new(shimmer) as container::StyleFn<'a, Theme>)
                    .into();

            block
        }
        SkeletonShape::Circle => {
            let circle: Element<'a, Message, Theme> = container(
                iced::widget::Space::new()
                    .width(Length::Fixed(48.0))
                    .height(Length::Fixed(48.0)),
            )
            .width(Length::Fixed(48.0))
            .height(Length::Fixed(48.0))
            .class(Box::new(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(theme.colors().muted)),
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 24.0.into(),
                },
                ..container::Style::default()
            }) as container::StyleFn<'a, Theme>)
            .into();

            circle
        }
    }
}

/// One skeleton line.
fn line<'a, Message: 'a>(width: Length) -> Element<'a, Message, Theme> {
    container(iced::widget::Space::new().height(Length::Fixed(12.0)))
        .width(width)
        .height(Length::Fixed(12.0))
        .class(Box::new(shimmer) as container::StyleFn<'a, Theme>)
        .into()
}

/// The shared skeleton surface: a muted fill with rounded corners.
fn shimmer(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme.colors().muted)),
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
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
    use super::{skeleton, skeleton_list_item, skeleton_table, SkeletonRow, SkeletonShape};
    use crate::theme::Theme;

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
