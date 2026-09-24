//! Message rows and groups: an avatar, a header, a content surface (usually a
//! [`Bubble`]), and a footer, aligned to the leading or trailing edge.
//!
//! The chat struct is named `Message`, which would collide with the crate-wide
//! convention of naming the widget message type parameter `Message`. To keep both
//! the public name and the impl headers valid, the type parameter is `M` in this
//! module only.

use super::bubble::{Bubble, MessageAlignment};
use crate::theme::{Size, Theme};
use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length, Padding};

/// The horizontal inset a message's header and footer lines carry by default.
///
/// It matches a bubble's own horizontal padding, so the small muted lines up
/// with the surface it annotates.
const INSET: f32 = 12.0;

/// A vertical stack of consecutive messages from the same sender.
#[must_use = "a MessageGroup does nothing unless it is turned into an Element"]
pub struct MessageGroup<'a, M> {
    children: Vec<Element<'a, M, Theme>>,
}

impl<'a, M: 'a> MessageGroup<'a, M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Adds a message (or any element) to the group.
    pub fn child(mut self, el: impl Into<Element<'a, M, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, M: 'a> Default for MessageGroup<'a, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, M: 'a> From<MessageGroup<'a, M>> for Element<'a, M, Theme> {
    fn from(group: MessageGroup<'a, M>) -> Self {
        column(group.children).spacing(2).width(Length::Fill).into()
    }
}

/// The avatar slot of a [`Message`].
#[must_use = "a MessageAvatar does nothing unless it is given to a Message"]
pub struct MessageAvatar<'a, M> {
    children: Vec<Element<'a, M, Theme>>,
}

impl<'a, M: 'a> MessageAvatar<'a, M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Adds an element to the avatar slot.
    pub fn child(mut self, el: impl Into<Element<'a, M, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, M: 'a> Default for MessageAvatar<'a, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, M: 'a> From<MessageAvatar<'a, M>> for Element<'a, M, Theme> {
    fn from(slot: MessageAvatar<'a, M>) -> Self {
        column(slot.children).spacing(4).into()
    }
}

/// The header line of a [`Message`], usually an author name.
#[must_use = "a MessageHeader does nothing unless it is given to a Message"]
pub struct MessageHeader<'a, M> {
    children: Vec<Element<'a, M, Theme>>,
    content_inset: Option<bool>,
}

impl<'a, M: 'a> MessageHeader<'a, M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            content_inset: None,
        }
    }

    /// Inset the line horizontally to match a bubble's own padding.
    ///
    /// Set explicitly this wins over the message's own choice, which is what
    /// lets a caller pin the alignment even beside a surface-less bubble.
    pub fn content_inset(mut self, content_inset: bool) -> Self {
        self.content_inset = Some(content_inset);
        self
    }

    /// Adds a text run to the header.
    pub fn text(mut self, content: impl text::IntoFragment<'a>) -> Self {
        self.children.push(
            text(content)
                .size(Size::Xs.text().size)
                .class(Box::new(|theme: &Theme| text::Style {
                    color: Some(theme.colors().muted_foreground),
                }) as iced::widget::text::StyleFn<'a, Theme>)
                .into(),
        );
        self
    }

    /// Adds an arbitrary element to the header.
    pub fn child(mut self, el: impl Into<Element<'a, M, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, M: 'a> Default for MessageHeader<'a, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, M: 'a> MessageHeader<'a, M> {
    /// Resolves the inset against the message's choice and renders.
    ///
    /// An explicit setting always wins; the inherited value only fills in a
    /// gap, which is why this is `unwrap_or` rather than an overwrite.
    fn into_element_with(self, inherited_inset: bool) -> Element<'a, M, Theme> {
        meta_row(self.children, self.content_inset.unwrap_or(inherited_inset))
    }
}

impl<'a, M: 'a> From<MessageHeader<'a, M>> for Element<'a, M, Theme> {
    fn from(header: MessageHeader<'a, M>) -> Self {
        header.into_element_with(true)
    }
}

/// The footer line of a [`Message`], usually a timestamp or status.
#[must_use = "a MessageFooter does nothing unless it is given to a Message"]
pub struct MessageFooter<'a, M> {
    children: Vec<Element<'a, M, Theme>>,
    content_inset: Option<bool>,
}

impl<'a, M: 'a> MessageFooter<'a, M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            content_inset: None,
        }
    }

    /// Inset the line horizontally to match a bubble's own padding.
    ///
    /// Set explicitly this wins over the message's own choice.
    pub fn content_inset(mut self, content_inset: bool) -> Self {
        self.content_inset = Some(content_inset);
        self
    }

    /// Adds a muted text run to the footer.
    pub fn text(mut self, content: impl text::IntoFragment<'a>) -> Self {
        self.children.push(
            text(content)
                .size(Size::Xs.text().size)
                .class(Box::new(|theme: &Theme| text::Style {
                    color: Some(theme.colors().muted_foreground),
                }) as iced::widget::text::StyleFn<'a, Theme>)
                .into(),
        );
        self
    }

    /// Adds an arbitrary element to the footer.
    pub fn child(mut self, el: impl Into<Element<'a, M, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, M: 'a> Default for MessageFooter<'a, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, M: 'a> MessageFooter<'a, M> {
    /// Resolves the inset against the message's choice and renders.
    fn into_element_with(self, inherited_inset: bool) -> Element<'a, M, Theme> {
        meta_row(self.children, self.content_inset.unwrap_or(inherited_inset))
    }
}

impl<'a, M: 'a> From<MessageFooter<'a, M>> for Element<'a, M, Theme> {
    fn from(footer: MessageFooter<'a, M>) -> Self {
        footer.into_element_with(true)
    }
}

/// Shared chrome for the header and footer lines.
///
/// Inset horizontally by a bubble's own padding so the small muted text lines
/// up with the surface it annotates. A message whose content is a surface-less
/// bubble passes `false`: there is no padding to line up with, and insetting
/// anyway would indent the line away from the text it belongs to.
fn meta_row<'a, M: 'a>(
    children: Vec<Element<'a, M, Theme>>,
    content_inset: bool,
) -> Element<'a, M, Theme> {
    let inset = if content_inset { INSET } else { 0.0 };

    container(row(children).spacing(4))
        .width(Length::Shrink)
        .padding(Padding::from([0.0, inset]))
        .into()
}

/// The content slot of a [`Message`]: a bubble plus any extra elements.
#[must_use = "a MessageContent does nothing unless it is given to a Message"]
pub struct MessageContent<'a, M> {
    bubble: Option<Element<'a, M, Theme>>,
    children: Vec<Element<'a, M, Theme>>,
    /// Whether the content is a bubble that draws no surface.
    surface_less: bool,
}

impl<'a, M: 'a> MessageContent<'a, M> {
    pub fn new() -> Self {
        Self {
            bubble: None,
            children: Vec::new(),
            surface_less: false,
        }
    }

    /// Sets the bubble this message shows.
    ///
    /// The bubble's variant decides whether the message's header and footer
    /// keep their horizontal inset: a surface-less bubble supplies no padding
    /// for them to line up with.
    pub fn bubble(mut self, bubble: Bubble<'a, M>) -> Self {
        self.surface_less |= bubble.is_ghost();
        self.bubble = Some(bubble.into());
        self
    }

    /// Adds an element beneath the bubble (e.g. an [`AttachmentGroup`]).
    ///
    /// [`AttachmentGroup`]: super::attachment::AttachmentGroup
    pub fn child(mut self, el: impl Into<Element<'a, M, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, M: 'a> Default for MessageContent<'a, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, M: 'a> From<MessageContent<'a, M>> for Element<'a, M, Theme> {
    fn from(content: MessageContent<'a, M>) -> Self {
        let mut col = column![].spacing(4).width(Length::Fill);
        if let Some(bubble) = content.bubble {
            col = col.push(bubble);
        }
        for el in content.children {
            col = col.push(el);
        }
        col.into()
    }
}

/// A single message row: avatar, header, content, and footer.
#[must_use = "a Message does nothing unless it is turned into an Element"]
pub struct Message<'a, M> {
    alignment: MessageAlignment,
    avatar: Option<Element<'a, M, Theme>>,
    header: Option<MessageHeader<'a, M>>,
    content: Option<MessageContent<'a, M>>,
    footer: Option<MessageFooter<'a, M>>,
}

impl<'a, M: 'a> Message<'a, M> {
    pub fn new() -> Self {
        Self {
            alignment: MessageAlignment::default(),
            avatar: None,
            header: None,
            content: None,
            footer: None,
        }
    }

    /// Sets the leading/trailing edge the message sits on.
    pub fn alignment(mut self, alignment: MessageAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets the message's avatar (e.g. an [`Avatar`](crate::widgets::Avatar) element).
    pub fn avatar(mut self, avatar: impl Into<Element<'a, M, Theme>>) -> Self {
        self.avatar = Some(avatar.into());
        self
    }

    /// Sets the header from a text run.
    pub fn header(mut self, content: impl text::IntoFragment<'a>) -> Self {
        self.header = Some(MessageHeader::new().text(content));
        self
    }

    /// Sets a fully-built header.
    pub fn header_el(mut self, header: MessageHeader<'a, M>) -> Self {
        self.header = Some(header);
        self
    }

    /// Sets the message content.
    pub fn content(mut self, content: MessageContent<'a, M>) -> Self {
        self.content = Some(content);
        self
    }

    /// Sets the footer from a text run.
    pub fn footer(mut self, content: impl text::IntoFragment<'a>) -> Self {
        self.footer = Some(MessageFooter::new().text(content));
        self
    }

    /// Sets a fully-built footer.
    pub fn footer_el(mut self, footer: MessageFooter<'a, M>) -> Self {
        self.footer = Some(footer);
        self
    }
}

impl<'a, M: 'a> Default for Message<'a, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, M: 'a> From<Message<'a, M>> for Element<'a, M, Theme> {
    fn from(message: Message<'a, M>) -> Self {
        let edge = match message.alignment {
            MessageAlignment::Start => Alignment::Start,
            MessageAlignment::End => Alignment::End,
        };

        // A surface-less bubble supplies no padding, so the header and footer
        // must not inset themselves away from the text they annotate.
        let content_inset = !message
            .content
            .as_ref()
            .is_some_and(|content| content.surface_less);

        let mut body = column![].spacing(6).align_x(edge).width(Length::Fill);
        if let Some(header) = message.header {
            body = body.push(header.into_element_with(content_inset));
        }
        if let Some(content) = message.content {
            body = body.push(content);
        }
        if let Some(footer) = message.footer {
            body = body.push(footer.into_element_with(content_inset));
        }

        let mut parts = row![].spacing(8).align_y(Alignment::End);
        match message.alignment {
            MessageAlignment::Start => {
                if let Some(avatar) = message.avatar {
                    parts = parts.push(avatar);
                }
                parts = parts.push(body);
            }
            MessageAlignment::End => {
                parts = parts.push(body);
                if let Some(avatar) = message.avatar {
                    parts = parts.push(avatar);
                }
            }
        }

        container(parts.width(Length::Fill))
            .align_x(edge)
            .width(Length::Fill)
            .into()
    }
}

/// Builds a [`Message`] that shows `bubble` as its content.
pub fn message<'a, M: 'a>(bubble: Bubble<'a, M>) -> Message<'a, M> {
    Message::new().content(MessageContent::new().bubble(bubble))
}

/// Starts a [`MessageGroup`].
pub fn message_group<'a, M: 'a>() -> MessageGroup<'a, M> {
    MessageGroup::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::widgets::chat::bubble;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg {}

    #[test]
    fn message_renders_both_alignments() {
        for a in [MessageAlignment::Start, MessageAlignment::End] {
            let av: Element<'_, Msg, Theme> = text("AB").into();
            let el: Element<'_, Msg, Theme> = message(bubble("hi").alignment(a))
                .alignment(a)
                .avatar(av)
                .header("Assistant")
                .footer("12:30")
                .into();
            drop(el);
        }
    }

    #[test]
    fn message_group_stacks_children() {
        let el: Element<'_, Msg, Theme> = message_group()
            .child(message(bubble("one")))
            .child(message(bubble("two")))
            .into();
        drop(el);
    }

    #[test]
    fn content_carries_bubble_and_extra_children() {
        let extra: Element<'_, Msg, Theme> = text("attachment").into();
        let el: Element<'_, Msg, Theme> = Message::new()
            .content(MessageContent::new().bubble(bubble("text")).child(extra))
            .into();
        drop(el);
    }

    #[test]
    fn sub_containers_render_standalone() {
        let h: Element<'_, Msg, Theme> = MessageHeader::new().text("Name").into();
        drop(h);
        let f: Element<'_, Msg, Theme> = MessageFooter::new().text("now").into();
        drop(f);
        let a: Element<'_, Msg, Theme> = MessageAvatar::new().child(text("AB")).into();
        drop(a);
    }

    /// The horizontal padding a meta line resolves to.
    ///
    /// This mirrors `meta_row`'s choice so the inheritance rule can be asserted
    /// without a window: the drawn value is a constant, and the rule is which
    /// input decides it.
    fn resolved_inset(inset: Option<bool>, inherited: bool) -> f32 {
        if inset.unwrap_or(inherited) {
            INSET
        } else {
            0.0
        }
    }

    /// A ghost bubble has no padding, so a header or footer beside it must not
    /// inset itself: the line would otherwise sit 12px away from the text it
    /// annotates.
    ///
    /// This is the reference's `test_ghost_bubble_inherits_message_slot_insets`.
    #[test]
    fn a_ghost_bubble_removes_the_slot_inset() {
        use crate::widgets::chat::bubble::BubbleVariant;

        let ghost: Bubble<'_, Msg> = Bubble::new().with_variant(BubbleVariant::Ghost);
        assert!(ghost.is_ghost(), "the Ghost variant must report itself");

        let surfaced: Bubble<'_, Msg> = Bubble::new().with_variant(BubbleVariant::Filled);
        assert!(!surfaced.is_ghost());

        // Content built over a ghost bubble asks for no inset.
        let ghost_content = MessageContent::new().bubble(ghost);
        assert!(
            ghost_content.surface_less,
            "a ghost bubble must mark its content surface-less"
        );

        let surfaced_content = MessageContent::new().bubble(surfaced);
        assert!(!surfaced_content.surface_less);

        // So the inherited inset is off for the ghost case and on otherwise.
        assert_eq!(resolved_inset(None, !ghost_content.surface_less), 0.0);
        assert_eq!(resolved_inset(None, !surfaced_content.surface_less), INSET);
    }

    /// An explicit setting always wins over the inherited one, in both
    /// directions: a caller can inset beside a ghost bubble, or drop the inset
    /// beside a surfaced one.
    #[test]
    fn an_explicit_inset_overrides_the_inherited_one() {
        // Explicitly inset, though the ghost bubble would have said not to.
        assert_eq!(resolved_inset(Some(true), false), INSET);

        // Explicitly flush, though a surfaced bubble would have said to inset.
        assert_eq!(resolved_inset(Some(false), true), 0.0);
    }

    #[test]
    fn header_and_footer_carry_an_inset_setting() {
        let header = MessageHeader::<Msg>::new().content_inset(false);
        assert_eq!(header.content_inset, Some(false));

        let footer = MessageFooter::<Msg>::new().content_inset(true);
        assert_eq!(footer.content_inset, Some(true));

        // Unstated by default, which is what lets the message decide.
        assert_eq!(MessageHeader::<Msg>::new().content_inset, None);
        assert_eq!(MessageFooter::<Msg>::new().content_inset, None);
    }

    /// The whole chain end to end: a message whose content is a ghost bubble
    /// renders, and one with a surfaced bubble renders, and their headers are
    /// built through different inset paths.
    #[test]
    fn messages_render_with_and_without_a_surfaced_bubble() {
        use crate::widgets::chat::bubble::BubbleVariant;

        for variant in [BubbleVariant::Filled, BubbleVariant::Ghost] {
            let av: Element<'_, Msg, Theme> = text("AB").into();
            let el: Element<'_, Msg, Theme> = message(bubble("hi").with_variant(variant))
                .avatar(av)
                .header("Assistant")
                .footer("12:30")
                .into();
            drop(el);
        }

        // A header that states its own inset still renders through the same path.
        let pinned: Element<'_, Msg, Theme> = Message::new()
            .header_el(MessageHeader::new().text("Name").content_inset(false))
            .footer_el(MessageFooter::new().text("now"))
            .content(MessageContent::new().bubble(bubble("x")))
            .into();
        drop(pinned);
    }
}
