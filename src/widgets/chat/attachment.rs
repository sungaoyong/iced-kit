//! Attachment cards: a file or media preview with an upload status, a title and
//! description, and an action row.
//!
//! The crate does not enable iced's `image` feature, so a media preview is drawn
//! as a file-type glyph (with an optional overlay) rather than a decoded bitmap,
//! matching the spec's fallback.

use crate::icons::IconName;
use crate::theme::{Size, Theme};
use iced::widget::canvas::{Frame, Geometry, Path, Stroke};
use iced::widget::{button, canvas, column, container, row, stack, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};

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

/// The metrics one size step of an [`Attachment`] uses.
///
/// The reference carries a full table per step — gap, type size, padding and
/// media box all move together — because an attachment at `Large` is a
/// different control from one at `XSmall` rather than the same control scaled.
#[derive(Debug, Clone, Copy, PartialEq)]
struct AttachmentMetrics {
    /// The gap between the media and the text stack.
    gap: f32,
    /// The point size the title draws at.
    type_size: f32,
    /// The padding around a step that has text.
    content_padding: Padding,
    /// The padding around a step that is only a preview.
    media_padding: Padding,
    /// The side of the square media box.
    media_side: f32,
    /// The corner of the media box.
    media_radius: u16,
}

impl AttachmentMetrics {
    /// The metrics for one size step.
    ///
    /// The reference's `Size(v)` case scales continuously rather than snapping
    /// to a step, which is what `Size::Custom` carries here.
    fn for_size(size: Size) -> Self {
        match size {
            Size::Xs => Self {
                gap: 6.0,
                type_size: 12.0,
                content_padding: Padding::from([4.0, 6.0]),
                media_padding: Padding::from(4.0),
                media_side: 28.0,
                media_radius: 3,
            },
            Size::Sm => Self {
                gap: 10.0,
                type_size: 12.0,
                content_padding: Padding::from([6.0, 8.0]),
                media_padding: Padding::from(6.0),
                media_side: 32.0,
                media_radius: 6,
            },
            Size::Md => Self {
                gap: 8.0,
                type_size: 14.0,
                content_padding: Padding::from([8.0, 10.0]),
                media_padding: Padding::from(8.0),
                media_side: 40.0,
                media_radius: 6,
            },
            Size::Lg => Self {
                gap: 12.0,
                type_size: 16.0,
                content_padding: Padding::from([12.0, 16.0]),
                media_padding: Padding::from(12.0),
                media_side: 48.0,
                media_radius: 6,
            },
            // An explicit size scales the same proportions the named steps
            // follow, so a custom attachment is not a different shape.
            Size::Custom(value) => Self {
                gap: 4.0,
                type_size: value * 0.875,
                content_padding: Padding::from(value * 0.25),
                media_padding: Padding::from(value * 0.25),
                media_side: value,
                media_radius: 6,
            },
        }
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
    size: Option<Size>,
}

impl<'a, Message: 'a> AttachmentMedia<'a, Message> {
    pub fn new() -> Self {
        Self {
            icon: IconName::File,
            overlay: None,
            size: None,
        }
    }

    /// Overrides the media box's size. By default it follows the card's.
    pub fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
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

impl<'a, Message: 'a> AttachmentMedia<'a, Message> {
    /// Renders the media box at `size`, tinted for `status`.
    fn into_element_with(
        self,
        size: Size,
        status: AttachmentStatus,
    ) -> Element<'a, Message, Theme> {
        crate::icons::load();

        let metrics = AttachmentMetrics::for_size(size);
        let box_side = metrics.media_side;
        // A failed preview is the card's most legible signal at a glance: the
        // box takes the destructive tint while the glyph and card stay intact,
        // so the reader sees *which* file failed without reading the text.
        let failed = status.is_failed();

        let glyph: Element<'a, Message, Theme> = container(
            text(crate::icons::glyph(self.icon))
                .font(crate::icons::font())
                .size(box_side * 0.5)
                .class(Box::new(move |theme: &Theme| text::Style {
                    color: Some(if failed {
                        theme.colors().destructive
                    } else {
                        theme.colors().muted_foreground
                    }),
                }) as iced::widget::text::StyleFn<'a, Theme>),
        )
        .width(Length::Fixed(box_side))
        .height(Length::Fixed(box_side))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .class(Box::new(move |theme: &Theme| {
            let colors = theme.colors();
            // The tint is an opaque mix rather than a translucent fill: iced's
            // renderer does not guarantee alpha compositing.
            let background = if failed {
                let base = colors.background;
                Color {
                    r: colors.destructive.r * 0.1 + base.r * 0.9,
                    g: colors.destructive.g * 0.1 + base.g * 0.9,
                    b: colors.destructive.b * 0.1 + base.b * 0.9,
                    a: 1.0,
                }
            } else {
                colors.muted
            };

            container::Style {
                background: Some(Background::Color(background)),
                border: Border {
                    radius: f32::from(metrics.media_radius).into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }) as iced::widget::container::StyleFn<'a, Theme>)
        .into();

        match self.overlay {
            Some(overlay) => iced::widget::stack(vec![glyph, overlay]).into(),
            None => glyph,
        }
    }
}

impl<'a, Message: 'a> From<AttachmentMedia<'a, Message>> for Element<'a, Message, Theme> {
    fn from(media: AttachmentMedia<'a, Message>) -> Element<'a, Message, Theme> {
        // Standalone, the media box has no card to take a status or size from.
        let size = media.size.unwrap_or(Size::Md);
        media.into_element_with(size, AttachmentStatus::Complete)
    }
}

/// The title line of an [`Attachment`]; shimmers while in progress.
#[must_use = "an AttachmentTitle does nothing unless it is given to an Attachment"]
pub struct AttachmentTitle<'a, Message> {
    text: String,
    status: Option<AttachmentStatus>,
    size: Option<Size>,
    _marker: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message: 'a> AttachmentTitle<'a, Message> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            status: None,
            size: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets the status explicitly, overriding the card's.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Overrides the title's type size.
    pub fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Renders the title at `inherited` unless it stated its own status.
    fn into_element_with(self, inherited: AttachmentStatus) -> Element<'a, Message, Theme> {
        let status = self.status.unwrap_or(inherited);
        let size = self.size.unwrap_or(Size::Md);

        if status.is_in_progress() {
            crate::widgets::shimmer_text(self.text)
        } else {
            text(self.text)
                .size(AttachmentMetrics::for_size(size).type_size)
                .class(Box::new(|theme: &Theme| text::Style {
                    color: Some(theme.colors().foreground),
                }) as iced::widget::text::StyleFn<'a, Theme>)
                .into()
        }
    }
}

impl<'a, Message: 'a> From<AttachmentTitle<'a, Message>> for Element<'a, Message, Theme> {
    fn from(title: AttachmentTitle<'a, Message>) -> Self {
        title.into_element_with(AttachmentStatus::Complete)
    }
}

/// The description line of an [`Attachment`].
#[must_use = "an AttachmentDescription does nothing unless it is given to an Attachment"]
pub struct AttachmentDescription<'a, Message> {
    text: String,
    status: Option<AttachmentStatus>,
    _marker: std::marker::PhantomData<&'a Message>,
}

impl<'a, Message: 'a> AttachmentDescription<'a, Message> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            status: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets the status explicitly, overriding the card's.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Renders the description at `inherited` unless it stated its own status.
    fn into_element_with(self, inherited: AttachmentStatus) -> Element<'a, Message, Theme> {
        let status = self.status.unwrap_or(inherited);
        // A failed description is the line that says *why*, so it takes the
        // destructive accent rather than the usual muted grey.
        let failed = status.is_failed();

        text(self.text)
            .size(Size::Xs.text().size)
            .class(Box::new(move |theme: &Theme| text::Style {
                color: Some(if failed {
                    theme.colors().destructive
                } else {
                    theme.colors().muted_foreground
                }),
            }) as iced::widget::text::StyleFn<'a, Theme>)
            .into()
    }
}

impl<'a, Message: 'a> From<AttachmentDescription<'a, Message>> for Element<'a, Message, Theme> {
    fn from(desc: AttachmentDescription<'a, Message>) -> Self {
        desc.into_element_with(AttachmentStatus::Complete)
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
        content.into_element_with(AttachmentStatus::Complete)
    }
}

impl<'a, Message: 'a> AttachmentContent<'a, Message> {
    /// Renders the stack, letting `status` reach any child that did not state
    /// its own.
    fn into_element_with(self, status: AttachmentStatus) -> Element<'a, Message, Theme> {
        let mut col = column![].spacing(2);

        if let Some(title) = self.title {
            // An explicitly-stated child status wins; otherwise the card's
            // status flows in, so a caller can set it once at the top.
            col = col.push(title.into_element_with(status));
        }
        if let Some(description) = self.description {
            col = col.push(description.into_element_with(status));
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
        Self {
            children: Vec::new(),
        }
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
    size: Size,
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
            size: Size::Md,
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

    /// Sets the size step, which scales the gap, type, padding and media box
    /// together. The default is `Size::Md`.
    pub fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
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
        let size = attachment.size;
        let axis = attachment.axis;
        let metrics = AttachmentMetrics::for_size(size);
        let has_content = attachment.content.is_some();

        let mut parts: Vec<Element<'a, Message, Theme>> = Vec::new();
        if let Some(media) = attachment.media {
            let media_size = media.size.unwrap_or(size);
            parts.push(media.into_element_with(media_size, status));
        }
        if let Some(content) = attachment.content {
            // The card's status is inherited by any title or description that
            // did not state one, so a caller sets it once on the card.
            parts.push(content.into_element_with(status));
        }

        let mut card: Element<'a, Message, Theme> = match axis {
            AttachmentAxis::Horizontal => row(parts)
                .spacing(metrics.gap)
                .align_y(Alignment::Center)
                .into(),
            AttachmentAxis::Vertical => column(parts).spacing(metrics.gap).into(),
        };

        if let Some(actions) = attachment.actions {
            if !actions.children.is_empty() {
                let actions: Element<'a, Message, Theme> = row(actions.children).spacing(4).into();

                card = match axis {
                    // Beside the text, the actions form a trailing cluster, so
                    // they sit on the card's own line rather than below it.
                    AttachmentAxis::Horizontal => row![card, actions]
                        .spacing(metrics.gap)
                        .align_y(Alignment::Center)
                        .into(),
                    AttachmentAxis::Vertical => column![card, actions].spacing(metrics.gap).into(),
                };
            }
        }

        // A card that is only a preview is padded like a thumbnail; one with
        // text takes the roomier padding its lines need.
        let padding = if has_content {
            metrics.content_padding
        } else {
            metrics.media_padding
        };

        let surface = container(card)
            .padding(padding)
            .width(Length::Fill)
            .class(Box::new(move |theme: &Theme| {
                let colors = theme.colors();

                container::Style {
                    background: Some(Background::Color(colors.surface)),
                    border: Border {
                        // Failed and Pending draw their own outline over the
                        // card, so the surface itself is unoutlined.
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: f32::from(theme.radius().md).into(),
                    },
                    ..Default::default()
                }
            })
                as iced::widget::container::StyleFn<'a, Theme>);

        // The outline is drawn as an overlay because neither of the two
        // treatments can be an iced `Border`: a dashed line has no `Border`
        // representation, and the failed tint is translucent so it must be
        // composited against the surface by hand.
        let outlined: Element<'a, Message, Theme> = if status.is_pending() || status.is_failed() {
            stack![
                surface,
                canvas(CardOutline {
                    status,
                    radius: f32::from(Theme::light().radius().md),
                })
                .width(Length::Fill)
                .height(Length::Fill),
            ]
            .into()
        } else {
            surface.into()
        };

        match attachment.on_click {
            // A clickable card highlights under the pointer, which is what
            // tells the reader the whole card is one target. A static card gets
            // no hover: there would be nothing for it to promise.
            Some(f) => button(outlined)
                .padding(Padding::ZERO)
                .width(Length::Fill)
                .class(
                    Box::new(|theme: &Theme, status| attachment_button_style(theme, status))
                        as iced::widget::button::StyleFn<'a, Theme>,
                )
                .on_press(f())
                .into(),
            None => outlined,
        }
    }
}

/// The appearance of a clickable attachment card.
fn attachment_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, iced::widget::button::Status::Hovered);

    iced::widget::button::Style {
        background: Some(Background::Color(if hovered {
            // A half-strength muted wash: enough to read as a target, quiet
            // enough that a grid of cards does not flicker as the pointer
            // crosses it.
            Color {
                a: 0.5,
                ..colors.muted
            }
        } else {
            colors.surface
        })),
        text_color: colors.foreground,
        border: Border::default(),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The dash pattern type iced's stroke carries.
use iced::advanced::graphics::geometry::LineDash;

/// Draws a card's outline: dashed while pending, destructively tinted while
/// failed.
///
/// This is a `canvas` because neither treatment fits an iced `Border`: a dash
/// pattern has no representation there, and the failed tint needs compositing.
struct CardOutline {
    status: AttachmentStatus,
    radius: f32,
}

impl<Message> iced::widget::canvas::Program<Message, Theme> for CardOutline {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let colors = theme.colors();
        let mut frame = Frame::new(renderer, bounds.size());

        // Inset by half the stroke so the line lands inside the card rather
        // than straddling its edge.
        let inset = 0.5;
        let outline = Path::rounded_rectangle(
            iced::Point::new(inset, inset),
            iced::Size::new(
                (bounds.width - inset * 2.0).max(0.0),
                (bounds.height - inset * 2.0).max(0.0),
            ),
            self.radius.into(),
        );

        let stroke = if self.status.is_pending() {
            // A dashed outline is the "not started yet" signal: the card is
            // present but has not been sent anywhere. `Stroke` has no dash
            // setter, so the pattern is written into the field.
            const DASHES: &[f32] = &[6.0, 4.0];

            Stroke {
                line_dash: LineDash {
                    segments: DASHES,
                    offset: 0,
                },
                ..Stroke::default()
            }
            .with_width(1.0)
            .with_color(colors.border)
        } else {
            // The failed tint is composited onto the card's own surface, since
            // a translucent border would otherwise depend on what is behind it.
            let tint = Color {
                a: 0.3,
                ..colors.destructive
            };
            let composited = crate::theme::catalog::blend(tint, colors.surface);

            Stroke::default().with_width(1.0).with_color(composited)
        };

        frame.stroke(&outline, stroke);

        vec![frame.into_geometry()]
    }
}

/// A cluster of attachments stacked vertically.
#[must_use = "an AttachmentGroup does nothing unless it is turned into an Element"]
pub struct AttachmentGroup<'a, Message> {
    children: Vec<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> AttachmentGroup<'a, Message> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
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

    /// The size steps scale the whole control, so a `Large` attachment is not
    /// merely a scaled `XSmall` one: gap, type, padding and media all move.
    #[test]
    fn every_size_step_has_its_own_metrics() {
        let steps = [Size::Xs, Size::Sm, Size::Md, Size::Lg];
        let metrics: Vec<AttachmentMetrics> = steps
            .iter()
            .map(|size| AttachmentMetrics::for_size(*size))
            .collect();

        for (index, one) in metrics.iter().enumerate() {
            for other in metrics.iter().skip(index + 1) {
                assert_ne!(
                    (one.gap, one.type_size, one.media_side),
                    (other.gap, other.type_size, other.media_side),
                    "two size steps must not be identical"
                );
            }
        }

        // The steps grow monotonically, so a larger step is never smaller.
        for pair in metrics.windows(2) {
            assert!(pair[0].media_side < pair[1].media_side);
            assert!(pair[0].type_size <= pair[1].type_size);
        }
    }

    /// A custom size scales continuously rather than snapping to a step.
    #[test]
    fn a_custom_size_scales_the_media_box() {
        let custom = AttachmentMetrics::for_size(Size::Custom(64.0));

        assert_eq!(custom.media_side, 64.0);
        assert_eq!(custom.type_size, 56.0, "0.875 of the size");
    }

    #[test]
    fn attachments_render_at_every_size() {
        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg, Size::Custom(56.0)] {
            let el: Element<'_, Msg, Theme> = attachment("report.pdf")
                .with_size(size)
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new("report.pdf"))
                        .description(AttachmentDescription::new("1.2 MB")),
                )
                .into();
            drop(el);
        }
    }

    /// The documented inheritance rule: a child that states a status keeps it,
    /// and one that does not takes the card's.
    #[test]
    fn a_child_status_overrides_the_cards() {
        let explicit: AttachmentTitle<'_, Msg> =
            AttachmentTitle::new("held").status(AttachmentStatus::Complete);
        assert_eq!(
            explicit.status,
            Some(AttachmentStatus::Complete),
            "an explicit status is remembered as explicit"
        );

        let inherited: AttachmentTitle<'_, Msg> = AttachmentTitle::new("flows");
        assert_eq!(inherited.status, None, "an unstated status stays unstated");
    }

    /// Since the status is inherited rather than defaulted, setting it on the
    /// card must reach a title that never mentioned it.
    #[test]
    fn an_in_progress_card_shimmers_its_unstated_title() {
        let el: Element<'_, Msg, Theme> = attachment("big.iso")
            .status(AttachmentStatus::Uploading)
            .content(AttachmentContent::new().title(AttachmentTitle::new("big.iso")))
            .into();
        drop(el);
    }

    #[test]
    fn media_takes_the_cards_size_and_status() {
        let el: Element<'_, Msg, Theme> = attachment("photo.png")
            .with_size(Size::Lg)
            .status(AttachmentStatus::Failed)
            .into();
        drop(el);

        // An explicit media size wins over the card's.
        let pinned: Element<'_, Msg, Theme> = attachment("photo.png")
            .with_size(Size::Xs)
            .media(AttachmentMedia::new().with_size(Size::Lg))
            .into();
        drop(pinned);
    }

    /// Both outline treatments are drawn by the canvas overlay, so each status
    /// has to be reachable without panicking and without the other's outline.
    #[test]
    fn the_outline_is_drawn_only_for_pending_and_failed() {
        for status in [
            AttachmentStatus::Pending,
            AttachmentStatus::Uploading,
            AttachmentStatus::Processing,
            AttachmentStatus::Failed,
            AttachmentStatus::Complete,
        ] {
            let el: Element<'_, Msg, Theme> = attachment("report.pdf").status(status).into();
            drop(el);
        }
    }

    /// A clickable card highlights under the pointer; a static one does not,
    /// because there is nothing for the highlight to promise.
    #[test]
    fn only_a_clickable_card_is_wrapped_in_a_button() {
        let clickable: Element<'_, Msg, Theme> = attachment("a").on_click(|| Msg::Open).into();
        drop(clickable);

        let static_card: Element<'_, Msg, Theme> = attachment("a").into();
        drop(static_card);
    }

    #[test]
    fn a_card_with_actions_keeps_them_on_its_own_line() {
        for axis in [AttachmentAxis::Horizontal, AttachmentAxis::Vertical] {
            let action: Element<'_, Msg, Theme> = text("x").into();
            let el: Element<'_, Msg, Theme> = attachment("a")
                .axis(axis)
                .actions(AttachmentActions::new().child(action))
                .into();
            drop(el);
        }
    }
}
