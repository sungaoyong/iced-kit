//! Setting pages: a named collection of groups.

use crate::setting::SettingGroup;
use crate::theme::{Size, Theme};
use crate::widgets::Icon;
use crate::icons::IconName;
use iced::widget::{column, container, row, text};
use iced::{Element, Length};

/// One page of settings.
///
/// A page is what the sidebar lists. Its groups are the boxes the page draws,
/// and the sidebar lists those too when there is more than one — which is what
/// makes a long page navigable without collapsing it.
///
/// ```
/// # use iced_kit::setting::{SettingField, SettingGroup, SettingItem, SettingPage};
/// # #[derive(Clone, Debug)] enum Message { Telemetry(bool) }
/// SettingPage::new("Privacy")
///     .icon(iced_kit::icons::IconName::User)
///     .description("What is shared and what stays here.")
///     .group(SettingGroup::new().title("Telemetry").item(
///         SettingItem::new("Send usage data").field(SettingField::switch(
///             false,
///             Message::Telemetry,
///         )),
///     ));
/// ```
#[must_use = "a SettingPage does nothing unless it is given to Settings"]
pub struct SettingPage<'a, Message> {
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    pub(crate) icon: Option<IconName>,
    pub(crate) groups: Vec<SettingGroup<'a, Message>>,
    pub(crate) resettable: bool,
}

impl<'a, Message: Clone + 'a> SettingPage<'a, Message> {
    /// Creates a page with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            icon: None,
            groups: Vec::new(),
            resettable: true,
        }
    }

    /// Sets the page's description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the icon shown beside the page's name in the sidebar.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Adds a group to the page.
    pub fn group(mut self, group: SettingGroup<'a, Message>) -> Self {
        self.groups.push(group);
        self
    }

    /// Adds several groups to the page.
    pub fn groups<I>(mut self, groups: I) -> Self
    where
        I: IntoIterator<Item = SettingGroup<'a, Message>>,
    {
        self.groups.extend(groups);
        self
    }

    /// Whether this page offers a "reset all" control.
    ///
    /// A page is resettable by default, but the control only appears while
    /// something on the page actually differs from its default — so a page whose
    /// settings have no defaults never shows one.
    pub fn resettable(mut self, resettable: bool) -> Self {
        self.resettable = resettable;
        self
    }

    /// The page's title.
    #[must_use]
    pub fn title_text(&self) -> &str {
        &self.title
    }

    /// The icon this page shows in the sidebar, if any.
    #[must_use]
    pub fn icon_name(&self) -> Option<IconName> {
        self.icon
    }

    /// Whether a reset control would have anything to do.
    ///
    /// Both conditions have to hold: the page has to allow resetting, and at
    /// least one of its groups has to hold a changed, resettable setting.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.resettable && self.groups.iter().any(SettingGroup::is_dirty)
    }

    /// Whether any group on the page offers a reset.
    #[must_use]
    pub fn is_resettable(&self) -> bool {
        self.resettable && self.groups.iter().any(SettingGroup::is_resettable)
    }

    /// Draws the page's header: its title, description and reset control.
    pub(crate) fn render_header(
        &self,
        query: &str,
        size: Size,
        on_reset: Option<Element<'a, Message, Theme>>,
    ) -> Element<'a, Message, Theme> {
        let mut heading = column![text(self.title.clone())
            .size((size.text().size + 4.0).max(16.0))
            .width(Length::Fill)]
        .spacing(4);

        if let Some(description) = &self.description {
            heading = heading.push(
                text(description.clone())
                    .size(Size::Sm.text().size)
                    .width(Length::Fill),
            );
        }

        let mut header = row![heading.width(Length::Fill)]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        // The reset control only appears while the page has changes to undo, so
        // it is offered only when the query does not hide the changed settings.
        if self.is_dirty() && !query.is_empty() {
            // Under an active search the changed setting may not be visible, so
            // the control is withheld rather than left to reset unseen things.
        } else if self.is_dirty() {
            if let Some(reset) = on_reset {
                header = header.push(reset);
            }
        }

        container(header)
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 16.0,
                right: 16.0,
                bottom: 0.0,
                left: 16.0,
            })
            .into()
    }

    /// Draws the groups this page shows for `query`.
    ///
    /// Each group is handed to `wrap` along with its index, which is how the
    /// panel gives a group an identity without the page having to know where it
    /// sits.
    pub(crate) fn render_groups<F>(
        &self,
        query: &str,
        size: Size,
        variant: crate::widgets::GroupBoxVariant,
        disabled: bool,
        wrap: F,
    ) -> Element<'a, Message, Theme>
    where
        F: Fn(usize, Element<'a, Message, Theme>) -> Element<'a, Message, Theme>,
    {
        let mut groups = column![].spacing(16.0).width(Length::Fill);

        for (group_ix, group) in self.groups.iter().enumerate() {
            if !crate::setting::search::group_matches(group, query) {
                continue;
            }

            groups = groups.push(wrap(group_ix, group.render(query, disabled, size, variant)));
        }

        container(groups)
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 16.0,
                right: 16.0,
                bottom: 16.0,
                left: 16.0,
            })
            .into()
    }

    /// Renders the page's icon, when it has one.
    pub(crate) fn render_icon(&self, size: Size) -> Option<Element<'a, Message, Theme>> {
        self.icon
            .map(|icon| Icon::new(icon).into_element(size))
    }
}

#[cfg(test)]
mod tests {
    use super::SettingPage;
    use crate::setting::{SettingField, SettingGroup, SettingItem};
    use crate::icons::IconName;

    #[derive(Debug, Clone)]
    enum Msg {
        /// The value is deliberately unread: these tests only need a message
        /// that carries the payload a field would report.
        Bool(#[allow(dead_code)] bool),
    }

    fn item(title: &str, value: bool, default: Option<bool>) -> SettingItem<'static, Msg> {
        let field = SettingField::switch(value, Msg::Bool);
        let field = match default {
            Some(default) => field.default_value(default),
            None => field,
        };

        SettingItem::new(title.to_owned()).field(field)
    }

    #[test]
    fn a_new_page_is_resettable_and_empty() {
        let page = SettingPage::<Msg>::new("General");

        assert_eq!(page.title_text(), "General");
        assert!(page.groups.is_empty());
        assert!(page.resettable);
        assert!(page.icon_name().is_none());
    }

    #[test]
    fn groups_accumulate_from_both_builders() {
        let page = SettingPage::<Msg>::new("General")
            .group(SettingGroup::new().item(item("a", true, None)))
            .groups([
                SettingGroup::new().item(item("b", true, None)),
                SettingGroup::new().item(item("c", true, None)),
            ]);

        assert_eq!(page.groups.len(), 3);
    }

    #[test]
    fn a_page_with_no_changes_offers_nothing_to_reset() {
        let page = SettingPage::<Msg>::new("General")
            .group(SettingGroup::new().item(item("a", true, Some(true))));

        assert!(page.is_resettable(), "the item could be reset");
        assert!(
            !page.is_dirty(),
            "but nothing has changed, so no control should appear"
        );
    }

    #[test]
    fn a_changed_setting_makes_the_page_dirty() {
        let page = SettingPage::<Msg>::new("General")
            .group(SettingGroup::new().item(item("a", false, Some(true))));

        assert!(page.is_dirty());
    }

    #[test]
    fn a_page_that_opted_out_never_offers_a_reset() {
        let page = SettingPage::<Msg>::new("Advanced")
            .resettable(false)
            .group(SettingGroup::new().item(item("a", false, Some(true))));

        assert!(!page.is_resettable());
        assert!(
            !page.is_dirty(),
            "opting out must suppress the reset even when a setting changed"
        );
    }

    #[test]
    fn a_page_whose_settings_have_no_defaults_is_not_resettable() {
        let page = SettingPage::<Msg>::new("General")
            .group(SettingGroup::new().item(item("a", true, None)));

        assert!(!page.is_resettable());
        assert!(!page.is_dirty());
    }

    #[test]
    fn a_page_can_carry_an_icon() {
        let page = SettingPage::<Msg>::new("Privacy").icon(IconName::User);
        let icon = page.icon_name().expect("the page should carry its icon");
        assert_eq!(
            crate::icons::glyph(icon),
            crate::icons::glyph(IconName::User)
        );
    }
}
