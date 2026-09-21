//! Selects (dropdowns).

use crate::theme::{Size, Theme};
use iced::widget::{pick_list as iced_pick_list, pick_list::Handle};
use iced::{Length, Padding};

/// Builds a themed select.
///
/// `options` is anything that borrows as a slice of `T`; `selected` is the
/// current value, if any. The selected item's `Display` output is what the
/// closed control shows.
///
/// ```
/// # use iced_kit::widgets::select;
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, PartialEq, Debug)] enum Fruit { Apple, Pear }
/// # impl std::fmt::Display for Fruit {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         write!(f, "{self:?}")
/// #     }
/// # }
/// # #[derive(Clone, Debug)] enum Message { Picked(Fruit) }
/// # fn view(current: Option<Fruit>) -> Element<'static, Message, Theme> {
/// select(vec![Fruit::Apple, Fruit::Pear], current, Message::Picked)
///     .placeholder("Choose a fruit")
///     .into()
/// # }
/// ```
/// Because `selected` is a plain `Option<T>` rather than iced's more general
/// `Option<V>`, passing `None` never needs a type annotation.
pub fn select<'a, T, L, Message>(
    options: L,
    selected: Option<T>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> iced_pick_list::PickList<'a, T, L, T, Message, Theme>
where
    T: ToString + PartialEq + Clone + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
    Message: Clone + 'a,
{
    let size = Size::Md;

    iced_pick_list(options, selected, on_selected)
        .text_size(size.text().size)
        .padding(Padding {
            top: 0.0,
            right: 8.0,
            bottom: 0.0,
            left: size.padding(),
        })
        .handle(Handle::Arrow {
            size: Some(iced::Pixels(14.0)),
        })
}

/// Builds a full-width themed select, for use in a form column.
pub fn select_fill<'a, T, L, Message>(
    options: L,
    selected: Option<T>,
    on_selected: impl Fn(T) -> Message + 'a,
) -> iced_pick_list::PickList<'a, T, L, T, Message, Theme>
where
    T: ToString + PartialEq + Clone + 'a,
    L: std::borrow::Borrow<[T]> + 'a,
    Message: Clone + 'a,
{
    select(options, selected, on_selected).width(Length::Fill)
}

#[cfg(test)]
mod tests {
    use super::{select, select_fill};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Fruit {
        Apple,
        Pear,
    }

    impl std::fmt::Display for Fruit {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Apple => write!(f, "Apple"),
                Self::Pear => write!(f, "Pear"),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Picked(Fruit),
    }

    #[test]
    fn a_select_renders_with_and_without_a_selection() {
        let options = vec![Fruit::Apple, Fruit::Pear];

        let empty: iced::Element<'_, Message, Theme> =
            select(options.clone(), None, Message::Picked).into();
        drop(empty);

        let chosen: iced::Element<'_, Message, Theme> =
            select(options, Some(Fruit::Apple), Message::Picked).into();
        drop(chosen);
    }

    #[test]
    fn a_select_renders_when_the_options_are_empty() {
        let empty: Vec<Fruit> = Vec::new();
        let element: iced::Element<'_, Message, Theme> =
            select(empty, None, Message::Picked).into();
        drop(element);
    }

    #[test]
    fn a_fill_select_renders() {
        let element: iced::Element<'_, Message, Theme> =
            select_fill(vec![Fruit::Apple], None, Message::Picked).into();
        drop(element);
    }
}
