//! Attachment cards: a file or media preview with an upload status, a title and
//! description, and an action row.
//!
//! The crate does not enable iced's `image` feature, so a media preview is drawn
//! as a file-type glyph (with an optional overlay) rather than a decoded bitmap,
//! matching the spec's fallback.

use crate::icons::IconName;
use crate::theme::{Size, Theme};
use iced::widget::{column, container, row, text};
use iced::{Alignment, Background, Border, Element, Padding};

/// The lifecycle stage of an [`Attachment`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AttachmentStatus {
    /// Selected and waiting to be uploaded.
    Pending,
    /// Currently uploading.
    Uploading,
    /// Uploaded and being processed.
    Processing,
    /// Upload or processing failed.
    Failed,
    /// Ready.
    #[default]
    Complete,
}

impl AttachmentStatus {
    #[must_use]
    pub fn is_pending(self) -> bool {
        self == Self::Pending
    }
    #[must_use]
    pub fn is_uploading(self) -> bool {
        self == Self::Uploading
    }
    #[must_use]
    pub fn is_processing(self) -> bool {
        self == Self::Processing
    }
    #[must_use]
    pub fn is_failed(self) -> bool {
        self == Self::Failed
    }
    #[must_use]
    pub fn is_complete(self) -> bool {
        self == Self::Complete
    }
    #[must_use]
    pub fn is_in_progress(self) -> bool {
        matches!(self, Self::Uploading | Self::Processing)
    }
}

/// Which way an [`Attachment`] lays out its media and text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AttachmentAxis {
    /// Media beside text.
    #[default]
    Horizontal,
    /// Media above text.
    Vertical,
}

/// The media preview of an [`Attachment`].
#[must_use = "an AttachmentMedia does nothing unless it is given to an Attachment"]
pub struct AttachmentMedia<'a, Message> {
    icon: IconName,
    overlay: Option<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> AttachmentMedia<'a, Message> {
    pub fn new() -> Self {
        Self {
            icon: IconName::File,
            overlay: None,
        }
    }

    /// Sets the file-type glyph drawn as the preview.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// Adds an element drawn on top of the preview (a progress badge, say).
    pub fn overlay(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.overlay = Some(el.into());
        self
    }
}

impl<'a, Message: 'a> Default for AttachmentMedia<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<AttachmentMedia<'a, Message>> for Element<'a, Message, Theme> {
    fn from(media: AttachmentMedia<'a, Message>) -> Element<'a, Message, Theme> {
        crate::icons::load();
        let glyph: Element<'a, Message, Theme> = container(
            text(crate::icons::glyph(media.icon))
                .font(crate::icons::font())
                .size(24.0)
                .class(Box::new(|theme: &Theme| text::Style {
                    color: Some(theme.colors().muted_foreground),
                }) as iced::widget::text::StyleFn<'a, Theme>),
        )
        .width(48.0)
        .height(48.0)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .class(Box::new(|theme: &Theme| container::Style {
            background: Some(Background::Color(theme.colors().muted)),
            border: Border {
                radius: f32::from(theme.radius().md).into(),
                ..Default::default()
            },
            ..Default::default()
        }) as iced::widget::container::StyleFn<'a, Theme>)
        .into();

        match media.overlay {
            Some(overlay) => iced::widget::stack(vec![glyph, overlay]).into(),
            None => glyph,
        }
    }
}

/// The title line of an [`Attachment`]; shimmers while in progress.
#[must_use = "an AttachmentTitle does nothing unless it is given to an Attachment"]
pub struct AttachmentTitle<'a, Message> {
    text: String,
    status: AttachmentStatus,
    _marker: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message: 'a> AttachmentTitle<'a, Message> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            status: AttachmentStatus::Complete,
            _marker: std::marker::PhantomData,
        }
    }

    /// Marks the title with a status so it shimmers while uploading.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = status;
        self
    }
}

impl<'a, Message: 'a> From<AttachmentTitle<'a, Message>> for Element<'a, Message, Theme> {
    fn from(title: AttachmentTitle<'a, Message>) -> Self {
        if title.status.is_in_progress() {
            crate::widgets::shimmer_text(title.text)
        } else {
            text(title.text)
                .size(Size::Sm.text().size)
                .class(Box::new(|theme: &Theme| text::Style {
                    color: Some(theme.colors().foreground),
                }) as iced::widget::text::StyleFn<'a, Theme>)
                .into()
        }
    }
}

/// The description line of an [`Attachment`].
#[must_use = "an AttachmentDescription does nothing unless it is given to an Attachment"]
pub struct AttachmentDescription<'a, Message> {
    text: String,
    status: AttachmentStatus,
    _marker: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message: 'a> AttachmentDescription<'a, Message> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            status: AttachmentStatus::Complete,
            _marker: std::marker::PhantomData,
        }
    }

    /// Marks the description with a status (failed descriptions read destructive).
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = status;
        self
    }
}

impl<'a, Message: 'a> From<AttachmentDescription<'a, Message>> for Element<'a, Message, Theme> {
    fn from(desc: AttachmentDescription<'a, Message>) -> Self {
        text(desc.text)
            .size(Size::Xs.text().size)
            .class(Box::new(move |theme: &Theme| text::Style {
                color: Some(if desc.status.is_failed() {
                    theme.colors().destructive
                } else {
                    theme.colors().muted_foreground
                }),
            }) as iced::widget::text::StyleFn<'a, Theme>)
            .into()
    }
}

/// The title + description stack of an [`Attachment`].
#[must_use = "an AttachmentContent does nothing unless it is given to an Attachment"]
pub struct AttachmentContent<'a, Message> {
    title: Option<AttachmentTitle<'a, Message>>,
    description: Option<AttachmentDescription<'a, Message>>,
}

impl<'a, Message: 'a> AttachmentContent<'a, Message> {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
        }
    }

    pub fn title(mut self, title: AttachmentTitle<'a, Message>) -> Self {
        self.title = Some(title);
        self
    }

    pub fn description(mut self, description: AttachmentDescription<'a, Message>) -> Self {
        self.description = Some(description);
        self
    }
}

impl<'a, Message: 'a> Default for AttachmentContent<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<AttachmentContent<'a, Message>> for Element<'a, Message, Theme> {
    fn from(content: AttachmentContent<'a, Message>) -> Self {
        let mut col = column![].spacing(2);
        if let Some(title) = content.title {
            col = col.push(title);
        }
        if let Some(description) = content.description {
            col = col.push(description);
        }
        col.into()
    }
}

/// The action row of an [`Attachment`] (open, remove, retry…).
#[must_use = "an AttachmentActions does nothing unless it is given to an Attachment"]
pub struct AttachmentActions<'a, Message> {
    children: Vec<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> AttachmentActions<'a, Message> {
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, Message: 'a> Default for AttachmentActions<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

/// A file/media attachment card.
#[must_use = "an Attachment does nothing unless it is turned into an Element"]
pub struct Attachment<'a, Message> {
    status: AttachmentStatus,
    axis: AttachmentAxis,
    media: Option<AttachmentMedia<'a, Message>>,
    content: Option<AttachmentContent<'a, Message>>,
    actions: Option<AttachmentActions<'a, Message>>,
    on_click: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<'a, Message: 'a> Attachment<'a, Message> {
    pub fn new() -> Self {
        Self {
            status: AttachmentStatus::default(),
            axis: AttachmentAxis::default(),
            media: None,
            content: None,
            actions: None,
            on_click: None,
        }
    }

    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = status;
        self
    }

    pub fn axis(mut self, axis: AttachmentAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn media(mut self, media: AttachmentMedia<'a, Message>) -> Self {
        self.media = Some(media);
        self
    }

    pub fn content(mut self, content: AttachmentContent<'a, Message>) -> Self {
        self.content = Some(content);
        self
    }

    pub fn actions(mut self, actions: AttachmentActions<'a, Message>) -> Self {
        self.actions = Some(actions);
        self
    }

    /// Makes the whole card a button reporting `f()` when pressed.
    pub fn on_click(mut self, f: impl Fn() -> Message + 'a) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl<'a, Message: 'a> Default for Attachment<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<Attachment<'a, Message>> for Element<'a, Message, Theme> {
    fn from(attachment: Attachment<'a, Message>) -> Self {
        let status = attachment.status;

        let mut parts: Vec<Element<'a, Message, Theme>> = Vec::new();
        if let Some(media) = attachment.media {
            parts.push(media.into());
        }
        if let Some(content) = attachment.content {
            parts.push(content.into());
        }

        let mut card: Element<'a, Message, Theme> = match attachment.axis {
            AttachmentAxis::Horizontal => row(parts).spacing(10).align_y(Alignment::Center).into(),
            AttachmentAxis::Vertical => column(parts).spacing(6).into(),
        };

        if let Some(actions) = attachment.actions {
            if !actions.children.is_empty() {
                card = column![card, row(actions.children).spacing(4)]
                    .spacing(6)
                    .into();
            }
        }

        let surface: Element<'a, Message, Theme> = container(card)
            .padding(Padding::from([10.0, 12.0]))
            .class(Box::new(move |theme: &Theme| container::Style {
                background: Some(Background::Color(theme.colors().surface)),
                border: Border {
                    color: if status.is_failed() {
                        theme.colors().destructive
                    } else {
                        theme.colors().border
                    },
                    width: 1.0,
                    radius: f32::from(theme.radius().md).into(),
                },
                ..Default::default()
            }) as iced::widget::container::StyleFn<'a, Theme>)
            .into();

        match attachment.on_click {
            Some(f) => iced::widget::button(surface).on_press(f()).into(),
            None => surface,
        }
    }
}

/// A cluster of attachments stacked vertically.
#[must_use = "an AttachmentGroup does nothing unless it is turned into an Element"]
pub struct AttachmentGroup<'a, Message> {
    children: Vec<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> AttachmentGroup<'a, Message> {
    pub fn new() -> Self {
        Self { children: Vec::new() }
    }

    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, Message: 'a> Default for AttachmentGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<AttachmentGroup<'a, Message>> for Element<'a, Message, Theme> {
    fn from(group: AttachmentGroup<'a, Message>) -> Self {
        column(group.children).spacing(6).into()
    }
}

/// Builds an [`Attachment`] with a title and a default file glyph.
pub fn attachment<'a, Message: 'a>(title: impl Into<String>) -> Attachment<'a, Message> {
    Attachment::new()
        .media(AttachmentMedia::new())
        .content(AttachmentContent::new().title(AttachmentTitle::new(title)))
}

/// Starts an [`AttachmentGroup`].
pub fn attachment_group<'a, Message: 'a>() -> AttachmentGroup<'a, Message> {
    AttachmentGroup::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg {
        Open,
    }

    #[test]
    fn status_predicates_are_exclusive() {
        let s = AttachmentStatus::Uploading;
        assert!(s.is_uploading() && s.is_in_progress());
        assert!(!s.is_complete() && !s.is_failed() && !s.is_pending());
        assert!(AttachmentStatus::Processing.is_in_progress());
        assert!(AttachmentStatus::Failed.is_failed());
        assert!(AttachmentStatus::Complete.is_complete());
        assert!(AttachmentStatus::Pending.is_pending());
    }

    #[test]
    fn renders_every_status() {
        for st in [
            AttachmentStatus::Pending,
            AttachmentStatus::Uploading,
            AttachmentStatus::Processing,
            AttachmentStatus::Failed,
            AttachmentStatus::Complete,
        ] {
            let el: Element<'_, Msg, Theme> = attachment("report.pdf")
                .status(st)
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new("report.pdf").status(st))
                        .description(AttachmentDescription::new("1.2 MB").status(st)),
                )
                .into();
            drop(el);
        }
    }

    #[test]
    fn clickable_attachment_renders() {
        let close: Element<'_, Msg, Theme> = text("x").into();
        let el: Element<'_, Msg, Theme> = attachment("photo.png")
            .media(AttachmentMedia::new().icon(IconName::File))
            .actions(AttachmentActions::new().child(close))
            .on_click(|| Msg::Open)
            .into();
        drop(el);
    }

    #[test]
    fn group_and_vertical_axis_render() {
        let el: Element<'_, Msg, Theme> = attachment_group()
            .child(attachment("a").axis(AttachmentAxis::Vertical))
            .child(attachment("b"))
            .into();
        drop(el);
    }
}
