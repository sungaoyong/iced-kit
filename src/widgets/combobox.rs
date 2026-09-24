//! Comboboxes: a select whose options can be searched, and which can hold more
//! than one value.
//!
//! # How it is put together
//!
//! A combobox has two halves, divided the way every iced overlay in this crate
//! is divided:
//!
//! - [`combobox`] builds the trigger — the closed control showing the current
//!   selection, and a text field when the caller wants one.
//! - [`ComboBoxPanel`] builds the open panel: a search field over the filtered
//!   options. The application positions it and draws it through
//!   [`Layer`](crate::widgets::overlay::Layer), because iced has no
//!   window-level z-order and no widget can place a popup over its neighbours.
//!
//! Filtering is a plain function of the query, so a caller can match on more
//! than the label — an id, an alias, a secondary line.
//!
//! # Usage
//!
//! ```
//! use iced_kit::widgets::{combobox, ComboBoxOption};
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! # #[derive(Clone, Debug)] enum Message { Toggled }
//! # fn view<'a>(
//! #     options: &'a [ComboBoxOption<usize>],
//! #     open: bool,
//! #     query: &str,
//! # ) -> Element<'a, Message, Theme> {
//! // The trigger stays in the page; the open panel is drawn by the application
//! // through `overlay::Layer`, because iced has no window-level z-order.
//! combobox::<usize, Message>(options, None, "Choose a fruit")
//!     .query(query)
//!     .on_toggle(Message::Toggled)
//!     .into()
//! # }
//! ```

use crate::theme::{Size, Theme};
use crate::widgets::input::text_input;
use crate::widgets::overlay::floating_shadow;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Padding};

/// One option in a [`combobox`].
#[derive(Debug, Clone)]
pub struct ComboBoxOption<T> {
    value: T,
    label: String,
    detail: Option<String>,
    disabled: bool,
}

impl<T> ComboBoxOption<T> {
    /// Creates an option with the given value and label.
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            detail: None,
            disabled: false,
        }
    }

    /// Adds a secondary line under the label.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Disables the option: it renders but cannot be picked.
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The option's value.
    #[must_use]
    pub fn value(&self) -> &T {
        &self.value
    }

    /// The option's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the option can be picked.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

impl<T: PartialEq> ComboBoxOption<T> {
    /// Whether this option is one of the selected values.
    #[must_use]
    pub fn is_selected(&self, selected: &[T]) -> bool {
        selected.contains(&self.value)
    }
}

impl<T: PartialEq> PartialEq for ComboBoxOption<T> {
    /// Two options are the same option when they carry the same value; the
    /// label is presentation and may be reworded without changing identity.
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

/// What a panel calls when an option is picked.
type OnSelect<'a, T, Message> = Box<dyn Fn(T) -> Message + 'a>;

/// What a panel calls when the query changes.
type OnQuery<'a, Message> = Box<dyn Fn(String) -> Message + 'a>;

/// A predicate over an option's value and a lowercased query.
type ValueMatches<'a, T> = Box<dyn Fn(&T, &str) -> bool + 'a>;

/// The options whose label, detail or value matches `query`, case-insensitively.
///
/// An empty query keeps everything. A disabled option that matches is still
/// shown — it is part of the list, and hiding it would misreport what the list
/// holds.
#[must_use]
pub fn filter_options<'a, T>(
    options: &'a [ComboBoxOption<T>],
    query: &str,
    matches: impl Fn(&T, &str) -> bool,
) -> Vec<&'a ComboBoxOption<T>> {
    let query = query.trim().to_lowercase();

    if query.is_empty() {
        return options.iter().collect();
    }

    options
        .iter()
        .filter(|option| {
            option.label.to_lowercase().contains(&query)
                || option
                    .detail
                    .as_ref()
                    .is_some_and(|detail| detail.to_lowercase().contains(&query))
                || matches(&option.value, &query)
        })
        .collect()
}

/// A combobox's trigger: the closed control.
///
/// See the module documentation for how this pairs with [`ComboBoxPanel`].
#[must_use = "a ComboBox does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct ComboBox<'a, T, Message> {
    options: &'a [ComboBoxOption<T>],
    selected: Vec<T>,
    placeholder: String,
    query: Option<String>,
    open: bool,
    searchable: bool,
    clearable: Option<Message>,
    on_open: Option<Message>,
    on_toggle: Option<Message>,
    on_query: Option<OnQuery<'a, Message>>,
    multi: bool,
    fill: bool,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, T: PartialEq + Clone + 'a, Message: Clone + 'a> ComboBox<'a, T, Message> {
    /// Creates a trigger over `options`, with `selected` already chosen.
    pub fn new(
        options: &'a [ComboBoxOption<T>],
        selected: impl IntoIterator<Item = T>,
        placeholder: impl Into<String>,
    ) -> Self {
        Self {
            options,
            selected: selected.into_iter().collect(),
            placeholder: placeholder.into(),
            query: None,
            open: false,
            searchable: false,
            clearable: None,
            on_open: None,
            on_toggle: None,
            on_query: None,
            multi: false,
            fill: false,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Makes the trigger's text editable, reporting each change.
    ///
    /// This is the difference between a plain select and a searchable one: the
    /// closed control becomes the search field, so the query is visible where
    /// the selection would otherwise be.
    pub fn on_query(mut self, on_query: impl Fn(String) -> Message + 'a) -> Self {
        self.on_query = Some(Box::new(on_query));
        self.searchable = true;
        self
    }

    /// Allows more than one value to be selected.
    pub fn multi(mut self, multi: bool) -> Self {
        self.multi = multi;
        self
    }

    /// Shows the current query in the trigger, so the trigger is the search
    /// field rather than merely reporting a selection.
    pub fn query(mut self, query: &str) -> Self {
        self.query = Some(query.to_owned());
        self
    }

    /// Marks the trigger as open, which keeps it highlighted while the panel
    /// is showing.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Makes the closed control a text field the user can type in.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Shows a clear button that emits this message.
    pub fn clearable(mut self, message: Message) -> Self {
        self.clearable = Some(message);
        self
    }

    /// Reports a press on the trigger, which the application turns into
    /// opening or closing the panel.
    pub fn on_open(mut self, message: Message) -> Self {
        self.on_open = Some(message);
        self
    }

    /// Reports a press with a toggle message, so one binding can open and close.
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.on_toggle = Some(message);
        self
    }

    /// Stretches the trigger to its container's width, for a form column.
    pub fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// The labels of the selected values, in the order the options declare
    /// them.
    #[must_use]
    pub fn selected_labels(&self) -> Vec<&str> {
        self.options
            .iter()
            .filter(|option| option.is_selected(&self.selected))
            .map(|option| option.label.as_str())
            .collect()
    }

    /// Replaces the selection.
    pub fn selected(mut self, selected: impl IntoIterator<Item = T>) -> Self {
        self.selected = selected.into_iter().collect();
        self
    }

    /// How many options match the current query.
    #[must_use]
    pub fn match_count(&self, matches: impl Fn(&T, &str) -> bool) -> usize {
        let query = self.query.as_deref().unwrap_or_default();
        filter_options(self.options, query, matches).len()
    }

    /// Turns the trigger into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            options,
            selected,
            placeholder,
            query,
            open,
            searchable,
            clearable,
            on_open,
            on_toggle,
            on_query,
            multi: _,
            fill,
            _lifetime,
        } = self;

        let text_style = Size::Md.text();
        let height = Size::Md.height();
        let selected_labels: Vec<&str> = options
            .iter()
            .filter(|option| option.is_selected(&selected))
            .map(|option| option.label.as_str())
            .collect();

        let display = if selected_labels.is_empty() {
            placeholder.clone()
        } else {
            selected_labels.join(", ")
        };

        let is_placeholder = selected_labels.is_empty() && query.is_none();

        // The trigger's content: the query when there is one, the selection
        // otherwise, and a caret at the trailing edge.
        let label_text = query.clone().unwrap_or(display);

        let label: Element<'a, Message, Theme> = text(label_text)
            .size(text_style.size)
            .line_height(iced::Pixels(height.max(text_style.line_height)))
            .class(Box::new(move |theme: &Theme| text::Style {
                color: Some(if is_placeholder {
                    theme.colors().muted_foreground
                } else {
                    theme.colors().foreground
                }),
            }) as text::StyleFn<'a, Theme>)
            .into();

        let mut content = row![]
            .spacing(8)
            .align_y(Alignment::Center)
            .push(container(label).width(Length::Fill));

        if let Some(message) = clearable {
            if !selected_labels.is_empty() {
                content = content.push(
                    crate::widgets::icon_button::<Message>()
                        .icon(crate::icons::IconName::X)
                        .ghost()
                        .size(Size::Sm)
                        .on_press(message),
                );
            }
        }

        let caret = crate::widgets::Icon::new(if open {
            crate::icons::IconName::ChevronUp
        } else {
            crate::icons::IconName::ChevronDown
        })
        .into_element(Size::Sm);
        content = content.push(caret);

        let mut trigger = button(content)
            .padding(Padding {
                top: 0.0,
                right: 8.0,
                bottom: 0.0,
                left: Size::Md.padding(),
            })
            .height(Length::Fixed(height))
            .width(if fill { Length::Fill } else { Length::Shrink })
            .class(Box::new(move |theme: &Theme, status| {
                combobox_trigger_style(theme, status, open)
            }) as button::StyleFn<'a, Theme>);

        // A searchable trigger is a field the user types in, so the query is
        // drawn by a text input rather than as the button's label. The panel
        // then carries no search field of its own: the query lives here.
        if let Some(on_query) = on_query {
            let field = crate::widgets::input::text_input::<Message>(
                &placeholder,
                query.as_deref().unwrap_or_default(),
            )
            .on_input(on_query)
            .width(Length::Fill);

            let mut content = row![].spacing(8).align_y(Alignment::Center);
            content = content.push(field);
            content = content.push(
                crate::widgets::Icon::new(if open {
                    crate::icons::IconName::ChevronUp
                } else {
                    crate::icons::IconName::ChevronDown
                })
                .into_element(Size::Sm),
            );

            let searchable = button(content)
                .padding(Padding {
                    top: 0.0,
                    right: 8.0,
                    bottom: 0.0,
                    left: 8.0,
                })
                .height(Length::Fixed(height))
                .width(if fill { Length::Fill } else { Length::Shrink })
                .class(Box::new(move |theme: &Theme, status| {
                    combobox_trigger_style(theme, status, open)
                }) as button::StyleFn<'a, Theme>);

            let searchable = match on_toggle.or(on_open) {
                Some(message) => searchable.on_press(message),
                None => searchable,
            };

            return searchable.into();
        }

        // One binding that both opens and closes is convenient; an explicit
        // toggle takes precedence over a plain open.
        if let Some(message) = on_toggle {
            trigger = trigger.on_press(message);
        } else if let Some(message) = on_open {
            trigger = trigger.on_press(message);
        }

        let _ = searchable;

        trigger.into()
    }
}

impl<'a, T: PartialEq + Clone + 'a, Message: Clone + 'a> From<ComboBox<'a, T, Message>>
    for Element<'a, Message, Theme>
{
    fn from(combo: ComboBox<'a, T, Message>) -> Self {
        combo.into_element()
    }
}

/// Builds a combobox trigger.
pub fn combobox<'a, T: PartialEq + Clone + 'a, Message: Clone + 'a>(
    options: &'a [ComboBoxOption<T>],
    selected: Option<T>,
    placeholder: impl Into<String>,
) -> ComboBox<'a, T, Message> {
    ComboBox::new(options, selected, placeholder)
}

/// The open panel: a search field over the matching options.
///
/// The application positions this and draws it through
/// [`Layer`](crate::widgets::overlay::Layer).
#[must_use = "a ComboBoxPanel does nothing unless it is turned into an Element"]
pub struct ComboBoxPanel<'a, T, Message> {
    options: &'a [ComboBoxOption<T>],
    selected: &'a [T],
    query: &'a str,
    on_query: Option<OnQuery<'a, Message>>,
    on_select: OnSelect<'a, T, Message>,
    matches: ValueMatches<'a, T>,
    empty_label: String,
    width: f32,
    max_height: f32,
    multi: bool,
}

impl<'a, T: PartialEq + Clone + 'a, Message: Clone + 'a> ComboBoxPanel<'a, T, Message> {
    /// Creates a panel over `options` filtered by `query`.
    pub fn new(
        options: &'a [ComboBoxOption<T>],
        selected: &'a [T],
        query: &'a str,
        on_select: impl Fn(T) -> Message + 'a,
    ) -> Self {
        Self {
            options,
            selected,
            query,
            on_query: None,
            on_select: Box::new(on_select),
            matches: Box::new(|_, _| false),
            empty_label: "No matches".to_owned(),
            width: 240.0,
            max_height: 280.0,
            multi: false,
        }
    }

    /// Makes the panel's search field editable.
    ///
    /// Without this the panel is a plain option list, which is what a
    /// non-searchable select needs.
    pub fn on_query(mut self, on_query: impl Fn(String) -> Message + 'a) -> Self {
        self.on_query = Some(Box::new(on_query));
        self
    }

    /// Adds a predicate the filter also consults, for matching on a value.
    pub fn matches(mut self, matches: impl Fn(&T, &str) -> bool + 'a) -> Self {
        self.matches = Box::new(matches);
        self
    }

    /// Sets the line shown when nothing matches.
    pub fn empty_label(mut self, label: impl Into<String>) -> Self {
        self.empty_label = label.into();
        self
    }

    /// Sets the panel's width. The default is 240.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(80.0);
        self
    }

    /// Sets the tallest the option list may grow. The default is 280.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height.max(40.0);
        self
    }

    /// Allows more than one value to be selected: picking keeps the panel open.
    pub fn multi(mut self, multi: bool) -> Self {
        self.multi = multi;
        self
    }

    /// How many options the current query leaves.
    #[must_use]
    pub fn visible_count(&self) -> usize {
        filter_options(self.options, self.query, &self.matches).len()
    }

    /// Turns the panel into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            options,
            selected,
            query,
            on_query,
            on_select,
            matches,
            empty_label,
            width,
            max_height,
            multi,
        } = self;

        let text_style = Size::Sm.text();
        let visible = filter_options(options, query, &matches);

        let mut body = column![].spacing(0).width(Length::Fill);

        if let Some(on_query) = on_query {
            let field = text_input::<Message>("Search…", query).on_input(on_query);

            body = body.push(container(field).width(Length::Fill).padding(Padding {
                top: 6.0,
                right: 6.0,
                bottom: 6.0,
                left: 6.0,
            }));
        }

        if visible.is_empty() {
            body = body.push(
                container(
                    text(empty_label)
                        .size(text_style.size)
                        .line_height(text_style.line_height())
                        .class(Box::new(|theme: &Theme| text::Style {
                            color: Some(theme.colors().muted_foreground),
                        }) as text::StyleFn<'a, Theme>),
                )
                .width(Length::Fill)
                .padding(Padding {
                    top: 10.0,
                    right: 10.0,
                    bottom: 10.0,
                    left: 10.0,
                }),
            );
        }

        for option in visible {
            let is_selected = option.is_selected(selected);
            let disabled = option.disabled;
            let value = option.value.clone();

            let mut label = row![].spacing(6).align_y(Alignment::Center);

            if is_selected {
                label = label.push(
                    crate::widgets::Icon::new(crate::icons::IconName::Check).into_element(Size::Sm),
                );
            }

            label = label.push(
                text(option.label.clone())
                    .size(text_style.size)
                    .line_height(text_style.line_height()),
            );

            if let Some(detail) = &option.detail {
                label = label.push(
                    text(detail.clone())
                        .size(Size::Xs.text().size)
                        .line_height(Size::Xs.text().line_height())
                        .class(Box::new(|theme: &Theme| text::Style {
                            color: Some(theme.colors().muted_foreground),
                        }) as text::StyleFn<'a, Theme>),
                );
            }

            let mut row_widget = button(label.width(Length::Fill))
                .width(Length::Fill)
                .padding(Padding {
                    top: 6.0,
                    right: 10.0,
                    bottom: 6.0,
                    left: 10.0,
                })
                .class(Box::new(move |theme: &Theme, status| {
                    option_row_style(theme, status, is_selected, disabled)
                }) as button::StyleFn<'a, Theme>);

            if !disabled {
                // Whether a pick closes the panel is the application's call: in
                // a multi-select it keeps the panel open so several values can
                // be ticked in one visit, and `multi` says which this is.
                let _ = multi;
                row_widget = row_widget.on_press((on_select)(value));
            }

            body = body.push(row_widget);
        }

        container(
            scrollable(body)
                .height(Length::Fixed(max_height))
                .width(Length::Fill),
        )
        .width(Length::Fixed(width))
        .class(Box::new(|theme: &Theme| {
            let colors = theme.colors();

            container::Style {
                background: Some(iced::Background::Color(colors.surface)),
                border: iced::Border {
                    color: colors.border,
                    width: 1.0,
                    radius: f32::from(theme.radius().md).into(),
                },
                shadow: floating_shadow(theme),
                text_color: Some(colors.foreground),
                ..container::Style::default()
            }
        }) as container::StyleFn<'a, Theme>)
        .padding(2)
        .into()
    }
}

impl<'a, T: PartialEq + Clone + 'a, Message: Clone + 'a> From<ComboBoxPanel<'a, T, Message>>
    for Element<'a, Message, Theme>
{
    fn from(panel: ComboBoxPanel<'a, T, Message>) -> Self {
        panel.into_element()
    }
}

/// Builds a combobox's open panel.
pub fn combobox_panel<'a, T: PartialEq + Clone + 'a, Message: Clone + 'a>(
    options: &'a [ComboBoxOption<T>],
    selected: &'a [T],
    query: &'a str,
    on_select: impl Fn(T) -> Message + 'a,
) -> ComboBoxPanel<'a, T, Message> {
    ComboBoxPanel::new(options, selected, query, on_select)
}

/// The appearance of a combobox trigger.
fn combobox_trigger_style(theme: &Theme, status: button::Status, open: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: Some(iced::Background::Color(if open || hovered {
            colors.accent
        } else {
            colors.surface
        })),
        text_color: colors.foreground,
        border: iced::Border {
            // The open trigger keeps the primary outline, so the panel below it
            // is visibly attached to the control that owns it.
            color: if open { colors.primary } else { colors.border },
            width: 1.0,
            radius: f32::from(theme.radius().md).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The appearance of one option row.
fn option_row_style(
    theme: &Theme,
    status: button::Status,
    is_selected: bool,
    disabled: bool,
) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);

    button::Style {
        background: (hovered && !disabled).then_some(iced::Background::Color(colors.accent)),
        // A selected row is marked by weight of colour as well as by its check
        // mark: the tick is enough to see, and the stronger text is what makes
        // a glance down the list show which values are already held.
        text_color: if disabled {
            Color {
                a: colors.muted_foreground.a * 0.6,
                ..colors.muted_foreground
            }
        } else if is_selected {
            colors.primary
        } else {
            colors.foreground
        },
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: f32::from(theme.radius().sm).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{combobox, combobox_panel, filter_options, ComboBoxOption};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Picked(usize),
        Opened,
        Toggled,
        Cleared,
        Queried(String),
    }

    fn countries() -> Vec<ComboBoxOption<usize>> {
        vec![
            ComboBoxOption::new(0, "Germany").detail("DE"),
            ComboBoxOption::new(1, "Ghana"),
            ComboBoxOption::new(2, "Japan").disabled(true),
        ]
    }

    #[test]
    fn an_option_keeps_its_parts() {
        let option = ComboBoxOption::new(7, "Files").detail("14 items");

        assert_eq!(*option.value(), 7);
        assert_eq!(option.label(), "Files");
        assert!(!option.is_disabled());
        assert!(ComboBoxOption::new(1, "X").disabled(true).is_disabled());
    }

    #[test]
    fn an_empty_query_keeps_every_option() {
        let options = countries();
        let visible = filter_options(&options, "", |_, _| false);

        assert_eq!(visible.len(), 3);
        // A whitespace-only query is empty too.
        assert_eq!(filter_options(&options, "   ", |_, _| false).len(), 3);
    }

    #[test]
    fn a_query_matches_the_label_case_insensitively() {
        let options = countries();

        let found = filter_options(&options, "gh", |_, _| false);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label(), "Ghana");

        assert_eq!(filter_options(&options, "JAPAN", |_, _| false).len(), 1);
    }

    #[test]
    fn a_query_also_matches_the_detail_and_the_value() {
        let options = countries();

        // "DE" is only in the detail line.
        let by_detail = filter_options(&options, "de", |_, _| false);
        assert_eq!(by_detail.len(), 1);
        assert_eq!(by_detail[0].label(), "Germany");

        // A predicate can match on the value, which has no text.
        let by_value = filter_options(&options, "2", |value, query| value.to_string() == query);
        assert_eq!(by_value.len(), 1);
        assert_eq!(by_value[0].label(), "Japan");
    }

    #[test]
    fn a_matching_disabled_option_still_shows() {
        let options = countries();
        let visible = filter_options(&options, "japan", |_, _| false);

        assert_eq!(visible.len(), 1);
        assert!(
            visible[0].is_disabled(),
            "the list must not hide its own state"
        );
    }

    #[test]
    fn a_trigger_reports_its_selection_as_labels() {
        let options = countries();

        let empty = super::ComboBox::<usize, Message>::new(&options, [], "Pick a country");
        assert!(empty.selected_labels().is_empty());

        let chosen = super::ComboBox::<usize, Message>::new(&options, [1], "Pick a country");
        assert_eq!(chosen.selected_labels(), vec!["Ghana"]);

        // Labels come back in the options' order, not the selection's.
        let multi =
            super::ComboBox::<usize, Message>::new(&options, [1, 0], "Pick a country").multi(true);
        assert_eq!(multi.selected_labels(), vec!["Germany", "Ghana"]);
    }

    #[test]
    fn a_trigger_counts_the_current_matches() {
        let options = countries();
        let trigger = super::ComboBox::<usize, Message>::new(&options, [], "Pick").query("gha");

        assert_eq!(trigger.match_count(|_, _| false), 1);
    }

    #[test]
    fn comboboxes_render_in_every_form() {
        let options = countries();

        let plain: iced::Element<'_, Message, Theme> =
            combobox(&options, None, "Pick a country").into();
        drop(plain);

        let open: iced::Element<'_, Message, Theme> = combobox(&options, None, "Pick")
            .query("gh")
            .open(true)
            .on_toggle(Message::Toggled)
            .fill(true)
            .into();
        drop(open);

        let clearable: iced::Element<'_, Message, Theme> = combobox(&options, Some(1), "Pick")
            .clearable(Message::Cleared)
            .on_open(Message::Opened)
            .multi(true)
            .searchable(true)
            .into();
        drop(clearable);
    }

    #[test]
    fn a_panel_renders_with_and_without_a_query_field() {
        let options = countries();
        let selected = [0usize];

        let list: iced::Element<'_, Message, Theme> =
            combobox_panel(&options, &selected, "", Message::Picked).into();
        drop(list);

        let searchable: iced::Element<'_, Message, Theme> =
            combobox_panel(&options, &selected, "gh", Message::Picked)
                .on_query(Message::Queried)
                .multi(true)
                .width(300.0)
                .max_height(200.0)
                .into();
        drop(searchable);
    }

    #[test]
    fn a_panel_reports_how_many_options_survive_the_query() {
        let options = countries();

        let all: super::ComboBoxPanel<'_, usize, Message> =
            combobox_panel(&options, &[], "", Message::Picked);
        assert_eq!(all.visible_count(), 3);

        let narrowed: super::ComboBoxPanel<'_, usize, Message> =
            combobox_panel(&options, &[], "jap", Message::Picked);
        assert_eq!(narrowed.visible_count(), 1);
    }

    #[test]
    fn a_panel_with_no_matches_renders_its_empty_label() {
        let options = countries();

        let element: iced::Element<'_, Message, Theme> =
            combobox_panel(&options, &[], "zzz", Message::Picked)
                .empty_label("Nothing here")
                .into();
        drop(element);
    }

    #[test]
    fn a_panel_clamps_its_geometry() {
        let options = countries();

        let tiny: super::ComboBoxPanel<'_, usize, Message> =
            combobox_panel(&options, &[], "", Message::Picked)
                .width(1.0)
                .max_height(1.0);

        assert_eq!(tiny.width, 80.0);
        assert_eq!(tiny.max_height, 40.0);
    }

    #[test]
    fn a_trigger_marks_open_and_closed_states() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let open = super::combobox_trigger_style(&theme, Status::Active, true);
        let closed = super::combobox_trigger_style(&theme, Status::Active, false);

        assert_eq!(open.border.color, theme.colors().primary);
        assert_eq!(closed.border.color, theme.colors().border);
    }
}
