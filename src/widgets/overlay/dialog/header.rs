//! The header of a dialog: a title, and usually a line under it.

use crate::theme::{Size, Theme};
use iced::widget::column;
use iced::widget::row;
use iced::widget::text::IntoFragment;
use iced::{Alignment, Element, Length};

use super::dialog_description;
use super::dialog_title;
use super::HEADER_GAP;

/// The header section of a dialog.
///
/// The reference's `DialogHeader`: a vertical stack with a tight gap, holding a
/// title and a description. It is what a caller assembles when it wants the
/// standard header inside a dialog body it is laying out itself.
///
/// ```
/// # use iced_kit::widgets::overlay::DialogHeader;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// DialogHeader::new()
///     .title("Delete project")
///     .description("This cannot be undone.")
///     .into()
/// # }
/// ```
#[must_use = "a DialogHeader does nothing unless it is turned into an Element"]
pub struct DialogHeader<'a, Message> {
    content: Vec<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> Default for DialogHeader<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> DialogHeader<'a, Message> {
    /// Creates an empty header.
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    /// Appends the dialog's title.
    ///
    /// A convenience for the common case; a caller with its own title element
    /// uses [`push`](Self::push) instead.
    pub fn title(mut self, title: impl IntoFragment<'a>) -> Self {
        self.content.push(dialog_title(title).into());
        self
    }

    /// Appends the dialog's supporting line.
    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        self.content.push(dialog_description(description).into());
        self
    }

    /// Appends a child.
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

    /// Converts the header into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        column(self.content).spacing(HEADER_GAP).into()
    }
}

impl<'a, Message: 'a> From<DialogHeader<'a, Message>> for Element<'a, Message, Theme> {
    fn from(header: DialogHeader<'a, Message>) -> Self {
        header.into_element()
    }
}

/// Builds a header pairing an icon with a title and description.
///
/// This is the reference's `AlertDialog` header layout: the icon sits to the
/// left of the text and stays at the text's top edge, so a description that
/// wraps does not push the icon down the line. The text takes the remaining
/// width and is allowed to shrink, which is what lets a long description wrap
/// instead of overflowing the dialog.
///
/// A caller that wants this layout in its own dialog body uses it directly;
/// [`AlertDialog`](super::AlertDialog) is the assembled version.
///
/// ```
/// # use iced_kit::widgets::overlay::header_with_icon;
/// # use iced_kit::widgets::Icon;
/// # use iced_kit::icons::IconName;
/// # use iced_kit::Size;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// header_with_icon(
///     Icon::new(IconName::Info).into_element(Size::Lg),
///     "Saved",
///     Some("Your changes are safe."),
/// )
/// # }
/// ```
#[must_use]
pub fn header_with_icon<'a, Message: 'a>(
    icon: impl Into<Element<'a, Message, Theme>>,
    title: impl IntoFragment<'a>,
    description: Option<impl IntoFragment<'a>>,
) -> Element<'a, Message, Theme> {
    icon_title_description(
        icon.into(),
        Some(title.into_fragment().to_string()),
        description.map(|text| text.into_fragment().to_string()),
    )
}

/// The icon-plus-text header layout, over owned strings.
///
/// [`header_with_icon`] and [`AlertDialog`](super::AlertDialog) both land here,
/// so the two cannot drift apart. The title and description are owned because
/// the alert may have neither, and because an iced [`Element`] borrows the
/// fragment it holds — a `&str` local to the call could not be handed to an
/// `Element<'a>`.
pub(crate) fn icon_title_description<'a, Message: 'a>(
    icon: Element<'a, Message, Theme>,
    title: Option<String>,
    description: Option<String>,
) -> Element<'a, Message, Theme> {
    let gap = Size::Md.gap();

    // A title and its description are one thought, so they sit closer together
    // than the header sits to the body. A missing title leaves the description
    // as the whole header rather than reserving an empty line for it.
    let mut text_column = column![].spacing(4.0);

    if let Some(title) = title {
        text_column = text_column.push(dialog_title(title));
    }

    if let Some(description) = description {
        text_column = text_column.push(dialog_description(description));
    }

    row![
        icon,
        // The text takes what is left and may shrink: a flex item that may not
        // shrink refuses to wrap, preferring to overflow its parent, which
        // would turn a long description into a clipped line.
        iced::widget::container(text_column).width(Length::Fill),
    ]
    .spacing(gap)
    .align_y(Alignment::Start)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{header_with_icon, DialogHeader};
    use crate::icons::IconName;
    use crate::theme::{Size, Theme};
    use crate::widgets::Icon;
    use iced::Element;

    #[test]
    fn a_header_renders_with_a_title_and_description() {
        let element: Element<'_, (), Theme> = DialogHeader::new()
            .title("Delete project")
            .description("This cannot be undone.")
            .into();
        drop(element);
    }

    #[test]
    fn an_empty_header_renders() {
        let element: Element<'_, (), Theme> = DialogHeader::new().into();
        drop(element);
    }

    #[test]
    fn a_header_accepts_custom_children() {
        let element: Element<'_, (), Theme> = DialogHeader::new()
            .push(iced::widget::text("Anything"))
            .extend([iced::widget::text("and more")])
            .into();
        drop(element);
    }

    #[test]
    fn a_header_with_an_icon_renders_with_and_without_a_description() {
        let icon = || Icon::new(IconName::Info).into_element(Size::Lg);

        let described: Element<'_, (), Theme> =
            header_with_icon(icon(), "Saved", Some("Your changes are safe."));
        drop(described);

        let bare: Element<'_, (), Theme> = header_with_icon(icon(), "Saved", None::<&str>);
        drop(bare);
    }
}
