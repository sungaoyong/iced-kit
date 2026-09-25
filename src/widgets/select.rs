//! Selects (dropdowns).
//!
//! A [`select`] is the zero-cost dropdown: a themed `pick_list` whose menu
//! iced draws and hosts itself, so a caller needs no open flag, no anchor and
//! no panel. It has no search and no multi-select — that is the combobox's
//! job. [`searchable_select`] and [`searchable_select_panel`] are thin aliases
//! over that combobox, kept here so the two capability levels read as one
//! family; the trigger styles are the same either way.

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

    // The caret is the icon font's chevron at the combobox trigger's icon
    // size, and the insets are the field insets those triggers use — a
    // select and a combobox side by side should read as one system. The
    // font has to be registered before the glyph can draw.
    crate::icons::load();

    iced_pick_list(options, selected, on_selected)
        .text_size(size.text().size)
        // A pick_list's height is its text line, which reads shorter than
        // every other form control beside it. The line box carries the
        // control's height token instead — the same trick the combobox
        // trigger's label uses, and pick_list centers the value in it.
        .text_line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(
            size.height(),
        )))
        .padding(Padding {
            top: 0.0,
            right: size.input_padding(),
            bottom: 0.0,
            left: size.input_padding(),
        })
        .handle(Handle::Static(iced_pick_list::Icon {
            font: crate::icons::font(),
            code_point: crate::icons::glyph(crate::icons::IconName::ChevronDown),
            size: Some(iced::Pixels(Size::Sm.icon_size())),
            line_height: iced::widget::text::LineHeight::Absolute(iced::Pixels(
                Size::Sm.icon_size(),
            )),
            shaping: iced::widget::text::Shaping::Basic,
        }))
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
/// A select that filters its options as the user types.
///
/// iced's own `pick_list` draws a menu it owns and cannot search, so a
/// searchable select is assembled from the pieces in this module: a trigger
/// whose text is the query, and a panel over the matching options. The
/// application owns the query, the open flag and the panel's placement — the
/// same division of labour as [`combobox`](crate::widgets::combobox).
///
/// ```
/// # use iced_kit::widgets::{searchable_select, ComboBoxOption};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Queried(String), Toggled }
/// # fn view<'a>(
/// #     options: &'a [ComboBoxOption<usize>],
/// #     query: &str,
/// #     open: bool,
/// # ) -> Element<'a, Message, Theme> {
/// searchable_select::<usize, Message>(options)
///     .query(query)
///     .open(open)
///     .on_query(Message::Queried)
///     .on_toggle(Message::Toggled)
///     .into()
/// # }
/// ```
pub fn searchable_select<'a, T: PartialEq + Clone + 'a, Message: Clone + 'a>(
    options: &'a [crate::widgets::combobox::ComboBoxOption<T>],
) -> crate::widgets::combobox::ComboBox<'a, T, Message> {
    crate::widgets::combobox::ComboBox::new(options, [], "Select…")
        .searchable(true)
        .fill(true)
}

/// The panel a searchable select drops: the search field and the options.
///
/// Pair it with [`searchable_select`], the way a combobox pairs its trigger
/// with its panel.
pub fn searchable_select_panel<'a, T: PartialEq + Clone + 'a, Message: Clone + 'a>(
    options: &'a [crate::widgets::combobox::ComboBoxOption<T>],
    selected: &'a [T],
    query: &'a str,
    on_select: impl Fn(T) -> Message + 'a,
) -> crate::widgets::combobox::ComboBoxPanel<'a, T, Message> {
    crate::widgets::combobox::ComboBoxPanel::new(options, selected, query, on_select)
}

#[cfg(test)]
mod searchable_tests {
    use super::{searchable_select, searchable_select_panel};
    use crate::theme::Theme;
    use crate::widgets::combobox::ComboBoxOption;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Picked(usize),
        Queried(String),
        Toggled,
    }

    #[test]
    fn a_searchable_select_renders() {
        let options = vec![
            ComboBoxOption::new(0, "Apple"),
            ComboBoxOption::new(1, "Pear"),
        ];

        let trigger: iced::Element<'_, Message, Theme> =
            searchable_select::<usize, Message>(&options)
                .query("ap")
                .open(true)
                .on_query(Message::Queried)
                .on_toggle(Message::Toggled)
                .into();
        drop(trigger);

        let panel: iced::Element<'_, Message, Theme> =
            searchable_select_panel(&options, &[], "ap", Message::Picked)
                .on_query(Message::Queried)
                .into();
        drop(panel);
    }

    #[test]
    fn a_searchable_select_filters_what_it_shows() {
        let options = vec![
            ComboBoxOption::new(0, "Apple"),
            ComboBoxOption::new(1, "Pear"),
        ];

        let all = searchable_select::<usize, Message>(&options);
        assert_eq!(all.match_count(|_, _| false), 2);

        let narrowed = searchable_select::<usize, Message>(&options).query("pea");
        assert_eq!(narrowed.match_count(|_, _| false), 1);
    }
}
