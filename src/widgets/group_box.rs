//! Group boxes: a titled surface that groups related content.
//!
//! This is the box the settings panel draws each group of settings in, and it is
//! useful anywhere a run of controls needs to read as one unit.
//!
//! # The three variants
//!
//! [`GroupBox::variant`] is the reference's `GroupBoxVariant`, and it decides how
//! much surface the box claims:
//!
//! - [`GroupBoxVariant::Normal`] — a plain surface. This is what a settings group
//!   uses, because the panel behind it already supplies the page background.
//! - [`GroupBoxVariant::Fill`] — a muted surface, for a group nested inside
//!   another box that must read as inset from it.
//! - [`GroupBoxVariant::Outline`] — a transparent fill with a visible border, for
//!   grouping without adding another surface.
//!
//! # Usage
//!
//! ```
//! use iced_kit::widgets::{group_box, label};
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! # #[derive(Clone, Debug)] enum Message {}
//! # fn view() -> Element<'static, Message, Theme> {
//! group_box()
//!     .title("Appearance")
//!     .description("How the app looks.")
//!     .push(label("Theme"))
//!     .into()
//! # }
//! ```

use crate::theme::{catalog, Size, Theme};
use iced::widget::{column, container, text};
use iced::{Element, Length, Padding};

/// How much surface a [`GroupBox`] claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroupBoxVariant {
    /// A plain surface with no border until its title asks for one.
    #[default]
    Normal,
    /// A muted, filled surface, for a group inset inside another.
    Fill,
    /// A transparent surface with a visible border.
    Outline,
}

impl GroupBoxVariant {
    /// The variant a name refers to, for a caller reading one from a config.
    ///
    /// An unrecognized name falls back to [`GroupBoxVariant::Normal`] rather than
    /// failing: a variant is presentation, and a config that names one this
    /// version does not know is better drawn plainly than refused.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "fill" => Self::Fill,
            "outline" => Self::Outline,
            _ => Self::Normal,
        }
    }

    /// The name this variant is read from a config by.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Fill => "fill",
            Self::Outline => "outline",
        }
    }
}

/// A titled surface that groups related content.
///
/// Build it with [`group_box`], or with [`GroupBox::new`]. A group box owns the
/// elements it was given, and iced's `Element` is neither `Clone` nor `Debug`,
/// so a builder here cannot be copied or printed the way a plain settings struct
/// can.
#[must_use = "a GroupBox does nothing unless it is turned into an Element"]
pub struct GroupBox<'a, Message> {
    title: Option<Element<'a, Message, Theme>>,
    description: Option<String>,
    variant: GroupBoxVariant,
    spacing: f32,
    padding: Option<Padding>,
    content: Vec<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> GroupBox<'a, Message> {
    /// Creates an empty group box.
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            variant: GroupBoxVariant::default(),
            spacing: 16.0,
            padding: None,
            content: Vec::new(),
        }
    }

    /// Sets the title drawn above the content.
    ///
    /// The title takes any element rather than only text, because a settings
    /// group heads itself with a title and a description stacked together.
    pub fn title(mut self, title: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets a secondary line drawn under the title.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets how much surface the box claims.
    pub fn variant(mut self, variant: GroupBoxVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the gap between the box's children.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    /// Sets the padding inside the box's border.
    ///
    /// The default suits a standalone box. A settings group passes a smaller
    /// vertical inset, because its own list already spaces its items.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Adds a child.
    pub fn push(mut self, child: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.content.push(child.into());
        self
    }

    /// Appends children.
    pub fn extend<I>(mut self, children: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Element<'a, Message, Theme>>,
    {
        self.content.extend(children.into_iter().map(Into::into));
        self
    }

    /// Converts the box into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            title,
            description,
            variant,
            spacing,
            padding,
            content,
        } = self;

        // The title block is built first so a group whose title is an empty
        // string still gets its own row rather than collapsing into the content.
        let heading = title.map(|title| {
            let mut block = column![title].spacing(4);

            if let Some(description) = description {
                block = block.push(
                    text(description)
                        .size(Size::Sm.text().size)
                        .width(Length::Fill),
                );
            }

            Element::from(block)
        });

        let body = {
            let mut list = column![].spacing(spacing);

            if let Some(heading) = heading {
                list = list.push(heading);
            }

            list = list.extend(content);

            list
        };

        let class: container::StyleFn<'a, Theme> = match variant {
            GroupBoxVariant::Normal => Box::new(catalog::card),
            GroupBoxVariant::Fill => Box::new(catalog::muted),
            GroupBoxVariant::Outline => Box::new(outline),
        };

        container(body)
            .padding(padding.unwrap_or_else(|| Padding::from(16)))
            .width(Length::Fill)
            .class(class)
            .into()
    }
}

impl<'a, Message: 'a> Default for GroupBox<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<GroupBox<'a, Message>> for Element<'a, Message, Theme> {
    fn from(box_: GroupBox<'a, Message>) -> Self {
        box_.into_element()
    }
}

/// A transparent surface with a visible border.
fn outline(theme: &Theme) -> container::Style {
    let colors = theme.colors();

    container::Style {
        background: None,
        border: iced::Border {
            color: colors.border,
            width: 1.0,
            radius: f32::from(theme.radius().lg).into(),
        },
        text_color: Some(colors.foreground),
        ..container::Style::default()
    }
}

/// Builds a [`GroupBox`].
///
/// ```
/// # use iced_kit::widgets::group_box;
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// group_box().push(iced::widget::text("Grouped")).into()
/// # }
/// ```
pub fn group_box<'a, Message: 'a>() -> GroupBox<'a, Message> {
    GroupBox::new()
}

#[cfg(test)]
mod tests {
    use super::{group_box, GroupBox, GroupBoxVariant};

    #[test]
    fn variants_round_trip_through_their_names() {
        for variant in [
            GroupBoxVariant::Normal,
            GroupBoxVariant::Fill,
            GroupBoxVariant::Outline,
        ] {
            assert_eq!(GroupBoxVariant::from_name(variant.as_str()), variant);
        }
    }

    #[test]
    fn an_unknown_variant_name_falls_back_to_normal() {
        assert_eq!(
            GroupBoxVariant::from_name("card"),
            GroupBoxVariant::Normal,
            "an unknown name must draw plainly rather than fail"
        );
    }

    #[test]
    fn variant_names_are_case_insensitive() {
        assert_eq!(GroupBoxVariant::from_name("FILL"), GroupBoxVariant::Fill);
        assert_eq!(
            GroupBoxVariant::from_name("Outline"),
            GroupBoxVariant::Outline
        );
    }

    #[test]
    fn the_default_variant_is_normal() {
        assert_eq!(GroupBoxVariant::default(), GroupBoxVariant::Normal);
        assert_eq!(GroupBox::<()>::new().variant, GroupBoxVariant::Normal);
    }

    #[test]
    fn pushing_children_accumulates_them() {
        let box_: GroupBox<'_, ()> = group_box()
            .push(iced::widget::text("a"))
            .push(iced::widget::text("b"));
        assert_eq!(box_.content.len(), 2);
    }

    #[test]
    fn extending_appends_many_children() {
        let box_: GroupBox<'_, ()> = group_box()
            .extend([iced::widget::text("a"), iced::widget::text("b")])
            .push(iced::widget::text("c"));
        assert_eq!(box_.content.len(), 3);
    }

    #[test]
    fn a_new_box_starts_empty_with_a_default_variant() {
        let box_: GroupBox<'_, ()> = group_box();
        assert!(box_.content.is_empty());
        assert!(box_.title.is_none());
        assert!(box_.description.is_none());
    }
}
