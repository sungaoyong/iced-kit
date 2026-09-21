//! Matching a search query against settings.

use crate::setting::{SettingGroup, SettingItem, SettingPage};

/// Whether `query` matches any of `haystacks`.
///
/// Matching is case-insensitive and substring-based, so `dark` finds "Dark
/// mode" without the user typing the whole heading. An empty query matches
/// everything, which is what makes clearing the search restore the full list.
#[must_use]
pub fn matches(query: &str, haystacks: &[&str]) -> bool {
    if query.is_empty() {
        return true;
    }

    let query = query.to_lowercase();

    haystacks
        .iter()
        .any(|haystack| haystack.to_lowercase().contains(&query))
}

/// Whether an item matches a query.
///
/// An item with no field is a section heading; it matches only when the query is
/// empty or names it directly, so a search does not drag a heading in without
/// the settings under it.
#[must_use]
pub(crate) fn item_matches<'a, Message: 'a>(item: &SettingItem<'a, Message>, query: &str) -> bool {
    let mut haystacks: Vec<&str> = vec![item.title()];

    if let Some(description) = item.description_text() {
        haystacks.push(description);
    }

    for keyword in &item.keywords {
        haystacks.push(keyword);
    }

    matches(query, &haystacks)
}

/// Whether a group has any matching item.
///
/// An empty query matches unconditionally, including a group with no items:
/// clearing the search has to restore exactly what the application declared,
/// rather than hiding groups that happen to be empty.
#[must_use]
pub(crate) fn group_matches<'a, Message: 'a>(group: &SettingGroup<'a, Message>, query: &str) -> bool {
    query.is_empty() || group.items.iter().any(|item| item_matches(item, query))
}

/// Whether a page has any matching group.
///
/// Like [`group_matches`], an empty query matches unconditionally.
#[must_use]
pub(crate) fn page_matches<'a, Message: 'a>(page: &SettingPage<'a, Message>, query: &str) -> bool {
    query.is_empty()
        || page
            .groups
            .iter()
            .any(|group| group_matches(group, query))
}

#[cfg(test)]
mod tests {
    use super::{group_matches, item_matches, matches, page_matches};
    use crate::setting::{SettingField, SettingGroup, SettingItem, SettingPage};

    #[derive(Debug, Clone)]
    enum Msg {
        /// The value is deliberately unread: these tests only need a message
        /// that carries the payload a field would report.
        Bool(#[allow(dead_code)] bool),
    }

    fn item(title: &str) -> SettingItem<'static, Msg> {
        SettingItem::new(title.to_owned())
            .field(SettingField::switch(true, Msg::Bool))
    }

    #[test]
    fn an_empty_query_matches_everything() {
        assert!(matches("", &["anything"]));
        assert!(matches("", &[]));
    }

    #[test]
    fn matching_ignores_case() {
        assert!(matches("dark", &["Dark mode"]));
        assert!(matches("DARK", &["Dark mode"]));
        assert!(matches("DaRk", &["Dark mode"]));
    }

    #[test]
    fn matching_is_substring_based() {
        assert!(matches("appear", &["Appearance settings"]));
        assert!(!matches("disappear", &["Appearance settings"]));
    }

    #[test]
    fn any_haystack_can_match() {
        assert!(matches("mfa", &["Two-factor auth", "mfa"]));
        assert!(matches("mfa", &["mfa", "Two-factor auth"]));
        assert!(!matches("mfa", &["Two-factor auth", "security"]));
    }

    #[test]
    fn an_item_matches_on_title_description_or_keyword() {
        let item = SettingItem::<Msg>::new("Two-factor auth")
            .description("Require a code at sign-in.")
            .keywords(["mfa"])
            .field(SettingField::switch(true, Msg::Bool));

        assert!(item_matches(&item, "two-factor"), "title");
        assert!(item_matches(&item, "sign-in"), "description");
        assert!(item_matches(&item, "mfa"), "keyword");
        assert!(!item_matches(&item, "password"));
    }

    #[test]
    fn a_group_matches_when_any_item_does() {
        let group = SettingGroup::<Msg>::new()
            .item(item("Theme"))
            .item(item("Language"));

        assert!(group_matches(&group, "lang"));
        assert!(!group_matches(&group, "telemetry"));
    }

    #[test]
    fn a_page_matches_when_any_group_does() {
        let page = SettingPage::<Msg>::new("General")
            .group(SettingGroup::new().item(item("Theme")))
            .group(SettingGroup::new().item(item("Language")));

        assert!(page_matches(&page, "theme"));
        assert!(!page_matches(&page, "telemetry"));
    }

    #[test]
    fn an_empty_group_matches_nothing_but_an_empty_query() {
        let group = SettingGroup::<Msg>::new();

        assert!(group_matches(&group, ""), "an empty query shows everything");
        assert!(
            !group_matches(&group, "anything"),
            "a group with no items cannot match a real query"
        );
    }
}
