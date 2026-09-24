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
use iced::{Alignment, Element, Length};

/// A vertical stack of consecutive messages from the same sender.
#[must_use = "a MessageGroup does nothing unless it is turned into an Element"]
pub struct MessageGroup<'a, M> {
    children: Vec<Element<'a, M, Theme>>,
}

impl<'a, M: 'a> MessageGroup<'a, M> {
    pub fn new() -> Self {
        Self { children: Vec::new() }
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
        Self { children: Vec::new() }
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
}

impl<'a, M: 'a> MessageHeader<'a, M> {
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    /// Adds a text run to the header.
    pub fn text(mut self, content: impl text::IntoFragment<'a>) -> Self {
        self.children.push(
            text(content)
                .size(Size::Sm.text().size)
                .class(Box::new(|theme: &Theme| text::Style {
                    color: Some(theme.colors().foreground),
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

impl<'a, M: 'a> From<MessageHeader<'a, M>> for Element<'a, M, Theme> {
    fn from(header: MessageHeader<'a, M>) -> Self {
        row(header.children).spacing(6).into()
    }
}

/// The footer line of a [`Message`], usually a timestamp or status.
#[must_use = "a MessageFooter does nothing unless it is given to a Message"]
pub struct MessageFooter<'a, M> {
    children: Vec<Element<'a, M, Theme>>,
}

impl<'a, M: 'a> MessageFooter<'a, M> {
    pub fn new() -> Self {
        Self { children: Vec::new() }
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

impl<'a, M: 'a> From<MessageFooter<'a, M>> for Element<'a, M, Theme> {
    fn from(footer: MessageFooter<'a, M>) -> Self {
        row(footer.children).spacing(6).into()
    }
}

/// The content slot of a [`Message`]: a bubble plus any extra elements.
#[must_use = "a MessageContent does nothing unless it is given to a Message"]
pub struct MessageContent<'a, M> {
    bubble: Option<Element<'a, M, Theme>>,
    children: Vec<Element<'a, M, Theme>>,
}

impl<'a, M: 'a> MessageContent<'a, M> {
    pub fn new() -> Self {
        Self {
            bubble: None,
            children: Vec::new(),
        }
    }

    /// Sets the bubble this message shows.
    pub fn bubble(mut self, bubble: Bubble<'a, M>) -> Self {
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
        let mut col = column![].spacing(4);
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

        let mut body = column![].spacing(4).width(Length::Fill);
        if let Some(header) = message.header {
            body = body.push(header);
        }
        if let Some(content) = message.content {
            body = body.push(content);
        }
        if let Some(footer) = message.footer {
            body = body.push(footer);
        }

        let mut parts = row![].spacing(8).align_y(Alignment::Start);
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

        container(parts.width(Length::Fill)).align_x(edge).width(Length::Fill).into()
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
}
