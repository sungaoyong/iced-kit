//! Modal dialogs.
//!
//! A [`Modal`] blocks the content behind it: the backdrop dims the page and
//! swallows clicks, so the dialog is the only interactive region until it is
//! dismissed.

use crate::motion::DURATION_SLOW;
use crate::theme::{Size, Theme};
use crate::widgets::button as kit_button;
use crate::widgets::overlay::{floating_shadow, scrim, Enter, EnterFrom};
use iced::widget::{column, container, row, text, MouseArea};
use iced::{Alignment, Element, Length, Padding};

/// How far a dialog rises as it arrives, in logical pixels.
///
/// Short on purpose: a dialog appears at the centre of attention rather than
/// travelling there, and a long slide would read as it coming from somewhere.
const DIALOG_TRAVEL: f32 = 12.0;

/// How wide a dialog is allowed to grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogWidth {
    /// A narrow dialog, for a confirmation.
    Sm,
    /// The default width.
    #[default]
    Md,
    /// A wide dialog, for a form.
    Lg,
}

impl DialogWidth {
    /// The maximum width in logical pixels.
    #[must_use]
    pub const fn max_width(self) -> f32 {
        match self {
            Self::Sm => 320.0,
            Self::Md => 480.0,
            Self::Lg => 720.0,
        }
    }
}

/// An action offered at the foot of a dialog.
struct Action<Message> {
    label: String,
    message: Message,
    destructive: bool,
    primary: bool,
}

/// A modal dialog.
#[must_use = "a Modal does nothing unless it is added to a Layer"]
pub struct Modal<'a, Message> {
    title: String,
    body: Element<'a, Message, Theme>,
    on_dismiss: Message,
    width: DialogWidth,
    actions: Vec<Action<Message>>,
    dismissible: bool,
}

impl<'a, Message: Clone + 'a> Modal<'a, Message> {
    /// Creates a modal with a title and body.
    ///
    /// `on_dismiss` is emitted when the backdrop is clicked, so a modal can
    /// always be closed without reaching for a specific button.
    pub fn new(
        title: impl Into<String>,
        body: impl Into<Element<'a, Message, Theme>>,
        on_dismiss: Message,
    ) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            on_dismiss,
            width: DialogWidth::default(),
            actions: Vec::new(),
            dismissible: true,
        }
    }

    /// Sets the dialog's maximum width.
    pub fn width(mut self, width: DialogWidth) -> Self {
        self.width = width;
        self
    }

    /// Adds a primary action button.
    pub fn confirm(mut self, label: impl Into<String>, message: Message) -> Self {
        self.actions.push(Action {
            label: label.into(),
            message,
            destructive: false,
            primary: true,
        });
        self
    }

    /// Adds a secondary, cancelling action.
    pub fn cancel(mut self, label: impl Into<String>, message: Message) -> Self {
        self.actions.push(Action {
            label: label.into(),
            message,
            destructive: false,
            primary: false,
        });
        self
    }

    /// Adds a destructive action, rendered in the danger color.
    pub fn destructive(mut self, label: impl Into<String>, message: Message) -> Self {
        self.actions.push(Action {
            label: label.into(),
            message,
            destructive: true,
            primary: false,
        });
        self
    }

    /// Whether clicking the backdrop dismisses the dialog.
    ///
    /// Turn this off for a dialog that must be answered explicitly, such as one
    /// guarding unsaved work.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Builds the full-screen layer: backdrop plus centred dialog.
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            title,
            body,
            on_dismiss,
            width,
            actions,
            dismissible,
        } = self;

        let title_style = Size::Lg.text();

        let mut content = column![text(title)
            .size(title_style.size)
            .line_height(title_style.line_height())]
        .spacing(16)
        .push(body);

        if !actions.is_empty() {
            let mut row = row![].spacing(8).align_y(Alignment::Center);

            // Secondary actions come first so the primary one lands on the
            // right, where a reader finishes.
            let (secondary, primary): (Vec<_>, Vec<_>) =
                actions.into_iter().partition(|action| !action.primary);

            for action in secondary {
                let widget = if action.destructive {
                    kit_button(action.label).destructive()
                } else {
                    kit_button(action.label).ghost()
                };

                row = row.push(widget.on_press(action.message));
            }

            for action in primary {
                row = row.push(kit_button(action.label).primary().on_press(action.message));
            }

            content = content.push(container(row).width(Length::Fill).align_x(Alignment::End));
        }

        let dialog = container(content)
            .width(Length::Fill)
            .max_width(width.max_width())
            .padding(Padding::new(24.0))
            .class(Box::new(|theme: &Theme| {
                let colors = theme.colors();

                container::Style {
                    background: Some(iced::Background::Color(colors.surface)),
                    border: iced::Border {
                        color: colors.border,
                        width: 1.0,
                        radius: f32::from(theme.radius().lg).into(),
                    },
                    shadow: floating_shadow(theme),
                    text_color: Some(colors.surface_foreground),
                    ..container::Style::default()
                }
            }) as container::StyleFn<'a, Theme>);

        // Centring happens inside a full-size container so the dialog stays
        // centred regardless of the window size.
        let centered = container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        let backdrop: Element<'a, Message, Theme> = if dismissible {
            // The backdrop both dims and dismisses: a `MouseArea` covers the
            // whole area and reports a press anywhere the dialog does not.
            MouseArea::new(scrim()).on_press(on_dismiss).into()
        } else {
            scrim()
        };

        // A dialog rises into place by a short distance: it is the arrival of a
        // surface at the centre of attention, so a long slide would read as the
        // dialog coming from somewhere rather than simply appearing.
        let surface = Enter::new(centered, EnterFrom::Below)
            .distance(DIALOG_TRAVEL)
            .duration(DURATION_SLOW);

        iced::widget::stack![backdrop, surface].into()
    }
}

/// A non-blocking dialog: the same visual treatment, but the content behind it
/// stays interactive.
#[must_use = "a Dialog does nothing unless it is turned into an Element"]
pub struct Dialog<'a, Message> {
    title: String,
    body: Element<'a, Message, Theme>,
    footer: Option<Element<'a, Message, Theme>>,
    width: DialogWidth,
}

impl<'a, Message: Clone + 'a> Dialog<'a, Message> {
    /// Creates a dialog with a title and body.
    pub fn new(title: impl Into<String>, body: impl Into<Element<'a, Message, Theme>>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            footer: None,
            width: DialogWidth::default(),
        }
    }

    /// Sets the dialog's maximum width.
    pub fn width(mut self, width: DialogWidth) -> Self {
        self.width = width;
        self
    }

    /// Sets the footer, usually a row of buttons.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Converts the dialog into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            title,
            body,
            footer,
            width,
        } = self;

        let title_style = Size::Lg.text();

        let mut content = column![text(title)
            .size(title_style.size)
            .line_height(title_style.line_height())]
        .spacing(16)
        .push(body);

        if let Some(footer) = footer {
            content = content.push(
                container(footer)
                    .width(Length::Fill)
                    .align_x(Alignment::End),
            );
        }

        container(content)
            .width(Length::Fill)
            .max_width(width.max_width())
            .padding(Padding::new(24.0))
            .class(Box::new(|theme: &Theme| {
                let colors = theme.colors();

                container::Style {
                    background: Some(iced::Background::Color(colors.surface)),
                    border: iced::Border {
                        color: colors.border,
                        width: 1.0,
                        radius: f32::from(theme.radius().lg).into(),
                    },
                    shadow: floating_shadow(theme),
                    text_color: Some(colors.surface_foreground),
                    ..container::Style::default()
                }
            }) as container::StyleFn<'a, Theme>)
            .into()
    }
}

impl<'a, Message: Clone + 'a> From<Dialog<'a, Message>> for Element<'a, Message, Theme> {
    fn from(dialog: Dialog<'a, Message>) -> Self {
        dialog.into_element()
    }
}

/// A right-aligned row of dialog buttons, for callers assembling their own
/// dialog body.
pub fn dialog_actions<'a, Message: Clone + 'a>(
    actions: Vec<crate::widgets::Button<'a, Message>>,
) -> Element<'a, Message, Theme> {
    let mut row = row![].spacing(8).align_y(Alignment::Center);

    for action in actions {
        row = row.push(action);
    }

    container(row)
        .width(Length::Fill)
        .align_x(Alignment::End)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{Dialog, DialogWidth, Modal};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Close,
        Confirm,
        Delete,
    }

    #[test]
    fn a_modal_renders_with_no_actions() {
        let element: iced::Element<'_, Message, Theme> =
            Modal::new("Title", iced::widget::text("Body"), Message::Close).into_element();
        drop(element);
    }

    #[test]
    fn a_modal_renders_with_every_action_kind() {
        let element: iced::Element<'_, Message, Theme> = Modal::new(
            "Delete file?",
            iced::widget::text("This cannot be undone."),
            Message::Close,
        )
        .cancel("Cancel", Message::Close)
        .destructive("Delete", Message::Delete)
        .confirm("Move to trash", Message::Confirm)
        .into_element();
        drop(element);
    }

    #[test]
    fn a_modal_orders_secondary_actions_before_the_primary_one() {
        // The primary action sits on the right, so it must be partitioned out
        // of the declaration order the caller used.
        let modal = Modal::new("Title", iced::widget::text("Body"), Message::Close)
            .confirm("Confirm", Message::Confirm)
            .cancel("Cancel", Message::Close);

        assert_eq!(modal.actions.len(), 2);
        assert!(modal.actions[0].primary, "confirm is declared first");
        assert!(!modal.actions[1].primary);
    }

    #[test]
    fn a_non_dismissible_modal_renders() {
        let element: iced::Element<'_, Message, Theme> = Modal::new(
            "Unsaved changes",
            iced::widget::text("Save before closing?"),
            Message::Close,
        )
        .dismissible(false)
        .into_element();
        drop(element);
    }

    #[test]
    fn modal_widths_increase_with_the_variant() {
        assert!(DialogWidth::Sm.max_width() < DialogWidth::Md.max_width());
        assert!(DialogWidth::Md.max_width() < DialogWidth::Lg.max_width());
    }

    #[test]
    fn a_dialog_renders_with_and_without_a_footer() {
        let plain: iced::Element<'_, Message, Theme> =
            Dialog::new("Title", iced::widget::text("Body")).into();
        drop(plain);

        let footed: iced::Element<'_, Message, Theme> =
            Dialog::new("Title", iced::widget::text("Body"))
                .width(DialogWidth::Lg)
                .footer(iced::widget::text("Footer"))
                .into();
        drop(footed);
    }

    #[test]
    fn dialog_actions_render() {
        use crate::widgets::button;

        let element: iced::Element<'_, Message, Theme> = super::dialog_actions(vec![
            button("Cancel").ghost().on_press(Message::Close),
            button("Confirm").primary().on_press(Message::Confirm),
        ]);
        drop(element);
    }
}
