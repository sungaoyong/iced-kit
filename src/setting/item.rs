//! Setting items: one labelled row in a settings group.

use crate::setting::field::SettingField;
use crate::theme::{Size, Theme};
use iced::widget::{column, container, row, text};
use iced::{Element, Length};

/// How an item lays its label and control out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingLayout {
    /// The label on the left, the control on the right. This is the settings
    /// look, and it is what makes a long list scannable.
    #[default]
    Horizontal,
    /// The label above the control, which gives a wide control the full width.
    Vertical,
}

/// One labelled setting.
///
/// An item is a title, an optional description, and a [`SettingField`]. The
/// title is what the search matches on, so it is worth writing in the words a
/// user would look for.
///
/// ```
/// # use iced_kit::setting::{SettingField, SettingItem};
/// # #[derive(Clone, Debug)] enum Message { Notify(bool) }
/// SettingItem::new("Notifications")
///     .description("Show a banner when a build finishes.")
///     .keywords(["alert", "banner"])
///     .field(SettingField::switch(true, Message::Notify));
/// ```
#[must_use = "a SettingItem does nothing unless it is given to a SettingGroup"]
pub struct SettingItem<'a, Message> {
    title: String,
    description: Option<String>,
    pub(crate) keywords: Vec<String>,
    layout: SettingLayout,
    disabled: bool,
    field: Option<SettingField<'a, Message>>,
}

impl<'a, Message: 'a> SettingItem<'a, Message> {
    /// Creates a setting with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            keywords: Vec::new(),
            layout: SettingLayout::default(),
            disabled: false,
            field: None,
        }
    }

    /// Sets the secondary line under the title.
    ///
    /// A description is searched as well as the title, so it can carry the words
    /// a user is likely to type without cluttering the visible heading.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds words that match this item in a search without being drawn.
    ///
    /// This is how a setting titled "Enable two-factor auth" becomes reachable by
    /// typing `MFA`.
    pub fn keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Sets how the label and control are laid out.
    pub fn layout(mut self, layout: SettingLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Draws the item with reduced emphasis and an inert control.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the control this setting manipulates.
    pub fn field(mut self, field: SettingField<'a, Message>) -> Self {
        self.field = Some(field);
        self
    }

    /// The item's title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The item's description, if it has one.
    #[must_use]
    pub fn description_text(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Whether the item is drawn inert.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

impl<'a, Message: 'a> SettingItem<'a, Message> {
    /// Whether a reset would change this item.
    #[must_use]
    pub fn is_resettable(&self) -> bool {
        self.field.as_ref().is_some_and(SettingField::is_resettable)
    }

    /// Whether this item differs from its defaults.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.field.as_ref().is_some_and(SettingField::is_dirty)
    }

    /// Whether this item draws a control that wants a narrow column.
    #[must_use]
    pub fn prefers_narrow_column(&self) -> bool {
        self.field
            .as_ref()
            .is_some_and(|field| field.kind().prefers_narrow_column())
    }
}

impl<'a, Message: Clone + 'a> SettingItem<'a, Message> {
    /// Draws the row.
    ///
    /// This borrows rather than consumes, so a panel can render an item without
    /// owning it: iced's `Element` is not `Clone`, so a consumed item could not
    /// be drawn again on the next frame.
    pub(crate) fn render(
        &self,
        size: Size,
        inherited_disabled: bool,
        layout: SettingLayout,
    ) -> Element<'a, Message, Theme> {
        let Self {
            title,
            description,
            keywords: _,
            layout: own_layout,
            disabled,
            field,
        } = self;

        // A vertical layout forced by the panel wins over the item's own, because
        // a narrow column cannot fit a horizontal row without squeezing the
        // label into a few characters per line.
        let layout = if layout == SettingLayout::Vertical {
            SettingLayout::Vertical
        } else {
            *own_layout
        };
        let disabled = *disabled || inherited_disabled;

        let mut label = column![text(title.clone())
            .size(size.text().size)
            .width(Length::Fill)]
        .spacing(4);

        if let Some(description) = description {
            label = label.push(
                text(description.clone())
                    .size(Size::Sm.text().size)
                    .width(Length::Fill),
            );
        }

        let control = match field {
            Some(field) => field.render(disabled, size),
            // An item with no field is a section heading inside a group: it shows
            // its title and description and nothing else.
            None => super::field::empty_field(),
        };

        let body: Element<'a, Message, Theme> = match layout {
            SettingLayout::Horizontal => row![
                container(label).width(Length::FillPortion(3)),
                container(control).width(Length::FillPortion(2)),
            ]
            .spacing(size.gap() * 2.0)
            .align_y(iced::Alignment::Start)
            .into(),
            SettingLayout::Vertical => column![label, control].spacing(8).into(),
        };

        container(body)
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 8.0,
                right: 0.0,
                bottom: 8.0,
                left: 0.0,
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::{SettingItem, SettingLayout};
    use crate::setting::SettingField;

    #[derive(Debug, Clone)]
    enum Msg {
        /// The value is deliberately unread: these tests only need a message
        /// that carries the payload a field would report.
        Bool(#[allow(dead_code)] bool),
    }

    #[test]
    fn an_item_holds_its_title_description_and_keywords() {
        let item = SettingItem::<Msg>::new("Notifications")
            .description("Banner on completion.")
            .keywords(["alert", "banner"]);

        assert_eq!(item.title(), "Notifications");
        assert_eq!(item.description_text(), Some("Banner on completion."));
        assert_eq!(item.keywords.len(), 2);
    }

    #[test]
    fn the_default_layout_is_horizontal() {
        assert_eq!(
            SettingItem::<Msg>::new("x").layout,
            SettingLayout::Horizontal
        );
    }

    #[test]
    fn an_item_without_a_field_is_not_resettable() {
        // This is the section-heading case: nothing to reset.
        let item = SettingItem::<Msg>::new("Section");
        assert!(!item.is_resettable());
        assert!(!item.is_dirty());
    }

    #[test]
    fn resettability_follows_the_field() {
        let with_default = SettingItem::<Msg>::new("x")
            .field(SettingField::switch(true, Msg::Bool).default_value(true));
        assert!(with_default.is_resettable());
        assert!(!with_default.is_dirty());

        let changed = SettingItem::<Msg>::new("x")
            .field(SettingField::switch(false, Msg::Bool).default_value(true));
        assert!(changed.is_resettable());
        assert!(changed.is_dirty(), "a changed field makes its item dirty");

        let bare = SettingItem::<Msg>::new("x").field(SettingField::switch(true, Msg::Bool));
        assert!(!bare.is_resettable());
    }

    #[test]
    fn a_boolean_control_asks_for_a_narrow_column() {
        let item = SettingItem::<Msg>::new("x").field(SettingField::switch(true, Msg::Bool));
        assert!(item.prefers_narrow_column());

        let text_item = SettingItem::<Msg>::new("x").field(SettingField::text("a", |s| {
            let _ = s;
            Msg::Bool(true)
        }));
        assert!(!text_item.prefers_narrow_column());
    }

    #[test]
    fn disabling_an_item_is_reported() {
        let item = SettingItem::<Msg>::new("x").disabled(true);
        assert!(item.is_disabled());
    }
}
