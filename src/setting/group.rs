//! Setting groups: a titled box of related settings.

use crate::setting::{SettingItem, SettingLayout};
use crate::theme::{Size, Theme};
use crate::widgets::group_box::GroupBox;
use crate::widgets::GroupBoxVariant;
use iced::widget::{column, text};
use iced::{Element, Length};

/// A titled group of settings.
///
/// A group is the box in the settings column. Its title is optional, and its
/// items are the rows drawn inside it.
///
/// ```
/// # use iced_kit::setting::{SettingField, SettingGroup, SettingItem};
/// # #[derive(Clone, Debug)] enum Message { Sync(bool) }
/// SettingGroup::new()
///     .title("Sync")
///     .description("How often changes are uploaded.")
///     .item(SettingItem::new("Background sync").field(SettingField::switch(
///         true,
///         Message::Sync,
///     )));
/// ```
#[must_use = "a SettingGroup does nothing unless it is given to a SettingPage"]
pub struct SettingGroup<'a, Message> {
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) items: Vec<SettingItem<'a, Message>>,
    pub(crate) variant: Option<GroupBoxVariant>,
    pub(crate) disabled: bool,
}

impl<'a, Message: Clone + 'a> SettingGroup<'a, Message> {
    /// Creates an untitled group.
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            items: Vec::new(),
            variant: None,
            disabled: false,
        }
    }

    /// Sets the group's heading.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets a line under the group's heading.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds a setting to the group.
    pub fn item(mut self, item: SettingItem<'a, Message>) -> Self {
        self.items.push(item);
        self
    }

    /// Adds several settings to the group.
    pub fn items<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = SettingItem<'a, Message>>,
    {
        self.items.extend(items);
        self
    }

    /// Overrides the surface this group draws on.
    ///
    /// Left unset, the group uses the variant its panel was given.
    pub fn variant(mut self, variant: GroupBoxVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Draws every control in the group inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The group's title, if it has one.
    #[must_use]
    pub fn title_text(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Whether the group is drawn inert.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Whether any item in the group could be reset.
    #[must_use]
    pub fn is_resettable(&self) -> bool {
        self.items.iter().any(SettingItem::is_resettable)
    }

    /// Whether any item in the group differs from its defaults.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.items.iter().any(SettingItem::is_dirty)
    }

    /// Whether the group has no items to show.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Draws the group.
    ///
    /// This borrows rather than consumes for the same reason
    /// [`SettingItem::render`](crate::setting::SettingItem) does: a panel draws
    /// every frame, and iced's `Element` cannot be cloned.
    ///
    /// `query` filters the items; `inherit_disabled` comes from the panel and is
    /// OR-ed with the group's own flag; `size` is the panel's control size.
    pub(crate) fn render(
        &self,
        query: &str,
        inherit_disabled: bool,
        size: Size,
        variant: GroupBoxVariant,
    ) -> Element<'a, Message, Theme> {
        let disabled = self.disabled || inherit_disabled;

        let mut box_ = GroupBox::new()
            .variant(self.variant.unwrap_or(variant))
            .spacing(0.0);

        if let Some(title) = &self.title {
            let mut heading = column![text(title.clone())
                .size(Size::Md.text().size)
                .width(Length::Fill)]
            .spacing(4);

            if let Some(description) = &self.description {
                heading = heading.push(
                    text(description.clone())
                        .size(Size::Sm.text().size)
                        .width(Length::Fill),
                );
            }

            box_ = box_.title(heading);
        }

        // Filtering happens here rather than at the panel, so a group owns the
        // rule about which of its own items survive a search.
        box_ = box_.extend(
            self.items
                .iter()
                .filter(|item| crate::setting::search::item_matches(item, query))
                .map(|item| item.render(size, disabled, SettingLayout::default())),
        );

        box_.into_element()
    }
}

impl<'a, Message: Clone + 'a> Default for SettingGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SettingGroup;
    use crate::setting::{SettingField, SettingItem};
    use crate::widgets::GroupBoxVariant;

    #[derive(Debug, Clone)]
    enum Msg {
        /// The value is deliberately unread: these tests only need a message
        /// that carries the payload a field would report.
        Bool(#[allow(dead_code)] bool),
    }

    fn switch_item(title: &str, value: bool, default: Option<bool>) -> SettingItem<'static, Msg> {
        let field = SettingField::switch(value, Msg::Bool);
        let field = match default {
            Some(default) => field.default_value(default),
            None => field,
        };

        SettingItem::new(title.to_owned()).field(field)
    }

    #[test]
    fn a_new_group_is_untitled_and_empty() {
        let group = SettingGroup::<Msg>::new();

        assert!(group.is_empty());
        assert_eq!(group.title_text(), None);
        assert_eq!(group.variant, None);
    }

    #[test]
    fn items_accumulate_from_both_builders() {
        let group = SettingGroup::<Msg>::new()
            .item(switch_item("a", true, None))
            .items([switch_item("b", true, None), switch_item("c", true, None)]);

        assert_eq!(group.items.len(), 3);
    }

    #[test]
    fn a_group_with_no_resettable_item_is_not_resettable() {
        let group = SettingGroup::<Msg>::new().item(switch_item("a", true, None));

        assert!(!group.is_resettable());
        assert!(!group.is_dirty());
    }

    #[test]
    fn resettability_and_dirtiness_come_from_the_items() {
        let clean = SettingGroup::<Msg>::new().item(switch_item("a", true, Some(true)));
        assert!(clean.is_resettable());
        assert!(!clean.is_dirty());

        let dirty = SettingGroup::<Msg>::new().item(switch_item("a", false, Some(true)));
        assert!(dirty.is_resettable());
        assert!(dirty.is_dirty());
    }

    #[test]
    fn one_changed_item_makes_the_whole_group_dirty() {
        let group = SettingGroup::<Msg>::new()
            .item(switch_item("a", true, Some(true)))
            .item(switch_item("b", false, Some(true)));

        assert!(group.is_dirty(), "one dirty item dirties its group");
    }

    #[test]
    fn a_group_can_override_its_panel_variant() {
        let group = SettingGroup::<Msg>::new().variant(GroupBoxVariant::Outline);

        assert_eq!(group.variant, Some(GroupBoxVariant::Outline));
    }

    #[test]
    fn disabling_a_group_is_reported() {
        let group = SettingGroup::<Msg>::new().disabled(true);
        assert!(group.is_disabled());
    }
}
