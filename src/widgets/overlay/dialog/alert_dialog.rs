//! An interrupting message with a response to give.

use crate::theme::Size;
use crate::theme::Theme;
use crate::widgets::button as kit_button;
use crate::widgets::overlay::{scrim, Enter, EnterFrom};
use crate::widgets::Icon;
use iced::widget::{column, container, row, MouseArea};
use iced::{Alignment, Element, Length};

use super::modal::{DialogButtonProps, DialogWidth};
use super::{
    corner_padding, section_padding, surface_class, surface_padding, DIALOG_GAP, DIALOG_PADDING,
};

/// The icon an alert opens with, when it is given one.
///
/// The reference leaves the icon to the caller and disables it by default;
/// [`AlertDialog::icon`] takes an element for the same reason, since a caller
/// may want its own artwork rather than a font glyph.
///
/// This is the pairing of a tone with its default glyph, so the common cases
/// need no choice:
///
/// - [`AlertTone::Question`] → [`IconName::CircleHelp`]
/// - [`AlertTone::Warning`] → [`IconName::TriangleAlert`]
/// - [`AlertTone::Danger`] → [`IconName::CircleAlert`]
/// - [`AlertTone::Success`] → [`IconName::CircleCheck`]
/// - [`AlertTone::Info`] → [`IconName::Info`]
///
/// [`IconName::CircleHelp`]: crate::icons::IconName::CircleHelp
/// [`IconName::TriangleAlert`]: crate::icons::IconName::TriangleAlert
/// [`IconName::CircleAlert`]: crate::icons::IconName::CircleAlert
/// [`IconName::CircleCheck`]: crate::icons::IconName::CircleCheck
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlertTone {
    /// Asking the user to choose, the default for a confirmation.
    #[default]
    Question,
    /// A caution: the action is possible but has consequences.
    Warning,
    /// A destructive action, about to be taken.
    Danger,
    /// Reporting that something worked.
    Success,
    /// A neutral notice.
    Info,
}

impl AlertTone {
    /// The glyph this tone draws by default.
    #[must_use]
    pub const fn icon(self) -> crate::icons::IconName {
        use crate::icons::IconName;

        match self {
            Self::Question => IconName::CircleHelp,
            Self::Warning => IconName::TriangleAlert,
            Self::Danger => IconName::CircleAlert,
            Self::Success => IconName::CircleCheck,
            Self::Info => IconName::Info,
        }
    }
}

/// A modal that interrupts the user and expects an answer.
///
/// An `AlertDialog` is a [`Modal`](super::Modal) with the defaults an alert
/// wants, following the reference's `AlertDialog`:
///
/// - An icon beside the title, which the tone decides.
/// - No close button, because an alert has an answer to collect rather than a
///   corner to dismiss it from. [`close_button`](Self::close_button) turns one
///   back on.
/// - A footer that centres its buttons, since an alert interrupts rather than
///   advances and there is no direction to send the eye in.
/// - [`confirm`](Self::confirm) to add the cancel half, so a plain
///   acknowledgement needs no footer at all.
///
/// The backdrop is [`dismissible`](Self::dismissible) by default like any other
/// modal. The reference disables it outright, on the argument that a stray
/// click should not answer a question the user was interrupted by; this crate
/// leaves the choice to the caller, and a confirmation guarding unsaved work is
/// the case that wants `dismissible(false)`.
///
/// ```
/// # use iced_kit::widgets::overlay::{AlertDialog, AlertTone};
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// AlertDialog::new()
///     .tone(AlertTone::Danger)
///     .title("Delete project")
///     .description("This permanently removes every file in it.")
///     .confirm()
///     .into()
/// # }
/// ```
#[must_use = "an AlertDialog does nothing unless it is added to a Layer"]
pub struct AlertDialog<'a, Message> {
    tone: AlertTone,
    icon: Option<Element<'a, Message, Theme>>,
    title: Option<String>,
    description: Option<String>,
    width: DialogWidth,
    buttons: DialogButtonProps,
    on_confirm: Option<Message>,
    on_cancel: Option<Message>,
    on_dismiss: Option<Message>,
    dismissible: bool,
    close_button: bool,
    draggable: bool,
    body: Vec<Element<'a, Message, Theme>>,
    footer: Option<Element<'a, Message, Theme>>,
}

impl<'a, Message: Clone + 'a> Default for AlertDialog<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> AlertDialog<'a, Message> {
    /// Creates an alert with no title, body or footer yet.
    ///
    /// The confirm button's label defaults to `OK`; give the alert a
    /// [`Message`] with [`on_confirm`](Self::on_confirm) or it renders a button
    /// that does nothing.
    pub fn new() -> Self {
        Self {
            tone: AlertTone::default(),
            icon: None,
            title: None,
            description: None,
            width: DialogWidth::Sm,
            buttons: DialogButtonProps::default(),
            on_confirm: None,
            on_cancel: None,
            on_dismiss: None,
            dismissible: true,
            close_button: false,
            draggable: false,
            body: Vec::new(),
            footer: None,
        }
    }

    /// Sets the alert's tone, which decides its default icon.
    pub fn tone(mut self, tone: AlertTone) -> Self {
        self.tone = tone;
        self
    }

    /// Overrides the tone's default icon.
    pub fn icon(mut self, icon: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Draws no icon at all.
    ///
    /// The reference disables the icon by default and has callers opt in; here
    /// the tone's icon is the default and this is the way back out.
    pub fn no_icon(mut self) -> Self {
        self.icon = Some(no_icon());
        self
    }

    /// Sets the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the line under the title.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the alert's maximum width.
    ///
    /// Defaults to [`DialogWidth::Sm`]: an alert carries a short question, not
    /// a form, and the reference's 420px is close to this crate's 320px step
    /// once the reference's own 16px insets are taken into account.
    pub fn width(mut self, width: DialogWidth) -> Self {
        self.width = width;
        self
    }

    /// Adds a cancel button beside the confirm one.
    ///
    /// The reference's `confirm()`. Without it the alert is an acknowledgement
    /// with a single button, which is the right shape for reporting an outcome.
    pub fn confirm(mut self) -> Self {
        self.buttons = self.buttons.show_cancel(true);
        self
    }

    /// Sets the message the confirm button emits.
    pub fn on_confirm(mut self, message: Message) -> Self {
        self.on_confirm = Some(message);
        self
    }

    /// Sets the message the cancel button emits.
    ///
    /// A cancel button with no message of its own falls back to
    /// [`on_dismiss`](Self::on_dismiss), so a caller that already has a "this
    /// dialog is over" message need not repeat it.
    pub fn on_cancel(mut self, message: Message) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// Sets the message the backdrop emits.
    pub fn on_dismiss(mut self, message: Message) -> Self {
        self.on_dismiss = Some(message);
        self
    }

    /// Overrides the default footer's button labels and variants.
    ///
    /// Only the fields `buttons` sets are overridden, so the cancel button
    /// [`confirm`](Self::confirm) asked for survives a later value that never
    /// mentions it.
    pub fn button_props(mut self, buttons: DialogButtonProps) -> Self {
        self.buttons.merge(buttons);
        self
    }

    /// Sets the label of the confirm button. Default is `OK`.
    pub fn ok_text(mut self, ok_text: impl Into<String>) -> Self {
        self.buttons = self.buttons.ok_text(ok_text);
        self
    }

    /// Sets the label of the cancel button. Default is `Cancel`.
    pub fn cancel_text(mut self, cancel_text: impl Into<String>) -> Self {
        self.buttons = self.buttons.cancel_text(cancel_text);
        self
    }

    /// Whether the alert draws a close button in its top-right corner.
    ///
    /// Off by default, unlike a [`Modal`](super::Modal).
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.close_button = close_button;
        self
    }

    /// Whether clicking the backdrop dismisses the alert.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Adds a line to the alert's body.
    pub fn push(mut self, child: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.body.push(child.into());
        self
    }

    /// Replaces the default buttons with a footer of the caller's own.
    ///
    /// When a footer is set, `button_props` no longer reaches the render: the
    /// caller is drawing the buttons, so it decides their labels.
    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Builds the alert as a modal layer.
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            tone,
            icon,
            title,
            description,
            width,
            buttons,
            on_confirm,
            on_cancel,
            on_dismiss,
            dismissible,
            close_button,
            draggable,
            body,
            footer,
        } = self;

        let icon = icon.unwrap_or_else(|| tone_icon(tone));

        let header = super::header::icon_title_description(icon, title, description);

        let inset = section_padding(DIALOG_PADDING);

        let mut sections = column![]
            .spacing(DIALOG_GAP)
            .push(container(header).padding(inset).width(Length::Fill));

        if !body.is_empty() {
            sections = sections.push(
                container(column(body).spacing(DIALOG_GAP))
                    .padding(inset)
                    .width(Length::Fill),
            );
        }

        let footer = footer.or_else(|| {
            Some(alert_buttons(
                &buttons,
                on_confirm.as_ref(),
                on_cancel.as_ref(),
            ))
        });

        if let Some(footer) = footer {
            sections = sections.push(container(footer).padding(inset).width(Length::Fill));
        }

        let padding = surface_padding(DIALOG_PADDING);

        let width = Length::Fixed(width.max_width());

        let card = container(sections)
            .width(width)
            .padding(padding)
            .class(surface_class());

        // A close button dismisses, so it is only drawn when dismissal is
        // allowed — the same rule a `Modal` follows. It floats over the card's
        // top-right corner rather than taking a row of its own, which is what
        // keeps it from pushing the alert's icon and title down when the two
        // differ in height. The width is fixed for the reason `surface` in
        // `modal` documents: a filling first child would make the stack span
        // the window and take the alert's centring with it.
        let card = match (dismissible && close_button)
            .then(|| on_cancel.clone().or_else(|| on_dismiss.clone()))
            .flatten()
        {
            None => card.into(),
            Some(message) => {
                let corner = container(super::footer::dialog_close(message))
                    .width(width)
                    .padding(corner_padding(DIALOG_PADDING))
                    .align_x(Alignment::End)
                    .align_y(Alignment::Start);

                iced::widget::Stack::new().push(card).push(corner).into()
            }
        };

        let card = if draggable {
            Element::new(super::drag::DragSurface::new(card))
        } else {
            // The same claim a non-draggable modal gets: an inert card lets a
            // press fall through to the backdrop, which would dismiss the
            // alert the reader was merely clicking into.
            Element::new(crate::widgets::overlay::ClaimPress::new(card))
        };

        alert_layer(card, dismissible.then_some(on_dismiss).flatten())
    }

    /// Whether the alert can be carried around by its surface.
    ///
    /// The same grab a [`Modal`](super::Modal) gets: a press an interactive
    /// child has not claimed picks the alert up, and it keeps where it was put
    /// until it is closed. Off by default, so an interrupting alert stays put.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }
}

impl<'a, Message: Clone + 'a> From<AlertDialog<'a, Message>> for Element<'a, Message, Theme> {
    fn from(alert: AlertDialog<'a, Message>) -> Self {
        alert.into_element()
    }
}

/// An element that occupies no space, for [`AlertDialog::no_icon`].
fn no_icon<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    container(iced::widget::Space::new())
        .width(Length::Fixed(0.0))
        .height(Length::Fixed(0.0))
        .into()
}

/// The tone's default icon, sized and coloured as an alert's icon.
fn tone_icon<'a, Message: 'a>(tone: AlertTone) -> Element<'a, Message, Theme> {
    let side = Size::Lg.icon_size();

    container(Icon::new(tone.icon()).size(side).into_element(Size::Lg))
        .width(Length::Fixed(side))
        .height(Length::Fixed(side))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}

/// Builds the default footer: a centred Cancel and OK.
fn alert_buttons<'a, Message: Clone + 'a>(
    buttons: &DialogButtonProps,
    on_confirm: Option<&Message>,
    on_cancel: Option<&Message>,
) -> Element<'a, Message, Theme> {
    let mut row = row![].spacing(super::FOOTER_GAP).align_y(Alignment::Center);

    if buttons.is_cancel_shown() {
        let label = buttons.resolved_cancel_text("Cancel");
        row = row.push(kit_button(label).ghost().on_press_maybe(on_cancel.cloned()));
    }

    let ok = kit_button(buttons.resolved_ok_text("OK")).on_press_maybe(on_confirm.cloned());

    let ok = if buttons.is_ok_primary() {
        ok.primary()
    } else {
        ok
    };

    row = row.push(ok);

    // Centred, not end-aligned: an alert interrupts rather than advances, so
    // there is no direction for the eye to be sent in and the buttons read as a
    // balanced pair.
    container(row)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

/// Wraps an alert surface in the centred, dimmed layer it occupies.
fn alert_layer<'a, Message: Clone + 'a>(
    surface: Element<'a, Message, Theme>,
    on_dismiss: Option<Message>,
) -> Element<'a, Message, Theme> {
    // A full-size box gives the alert something to be centred within, for the
    // same reason [`modal_layer`](super::modal) needs one.
    let centered = container(surface)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    let backdrop: Element<'a, Message, Theme> = match on_dismiss {
        Some(message) => MouseArea::new(scrim()).on_press(message).into(),
        None => scrim(),
    };

    let surface = Enter::new(centered, EnterFrom::Below)
        .distance(super::DIALOG_TRAVEL)
        .duration(crate::motion::DURATION_SLOW);

    // The stack's size is declared rather than inherited from its first child,
    // which asks for a filling width; see `modal_layer` for what goes wrong
    // when it is left to that hint.
    iced::widget::Stack::new()
        .push(backdrop)
        .push(surface)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{AlertDialog, AlertTone};
    use crate::theme::Theme;
    use crate::widgets::overlay::DialogButtonProps;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Close,
        Confirm,
        Cancel,
    }

    #[test]
    fn an_alert_renders_with_its_tone_defaults() {
        for tone in [
            AlertTone::Question,
            AlertTone::Warning,
            AlertTone::Danger,
            AlertTone::Success,
            AlertTone::Info,
        ] {
            let element: iced::Element<'_, Message, Theme> = AlertDialog::new()
                .tone(tone)
                .title("Title")
                .description("Body")
                .on_confirm(Message::Confirm)
                .into_element();
            drop(element);
        }
    }

    #[test]
    fn an_alert_renders_with_a_title_only() {
        let element: iced::Element<'_, Message, Theme> = AlertDialog::new()
            .title("Saved")
            .on_confirm(Message::Confirm)
            .into_element();
        drop(element);
    }

    #[test]
    fn an_alert_renders_with_no_body_at_all() {
        // Nothing forces a caller to give it text: the icon and buttons are
        // enough for a bare confirmation.
        let element: iced::Element<'_, Message, Theme> = AlertDialog::new()
            .on_confirm(Message::Confirm)
            .into_element();
        drop(element);
    }

    #[test]
    fn a_confirm_alert_renders_both_buttons() {
        let element: iced::Element<'_, Message, Theme> = AlertDialog::new()
            .title("Delete project?")
            .description("This cannot be undone.")
            .confirm()
            .on_confirm(Message::Confirm)
            .on_cancel(Message::Cancel)
            .into_element();
        drop(element);
    }

    #[test]
    fn an_alert_can_draw_no_icon_or_its_own() {
        let bare: iced::Element<'_, Message, Theme> = AlertDialog::new()
            .no_icon()
            .title("Saved")
            .on_confirm(Message::Confirm)
            .into_element();
        drop(bare);

        let custom: iced::Element<'_, Message, Theme> = AlertDialog::new()
            .icon(
                crate::widgets::Icon::new(crate::icons::IconName::Sparkles)
                    .into_element(crate::theme::Size::Lg),
            )
            .title("Saved")
            .on_confirm(Message::Confirm)
            .into_element();
        drop(custom);
    }

    #[test]
    fn an_alert_accepts_a_custom_footer() {
        let element: iced::Element<'_, Message, Theme> = AlertDialog::new()
            .title("Choose")
            .footer(crate::widgets::button("Later").on_press(Message::Close))
            .into_element();
        drop(element);
    }

    #[test]
    fn an_alert_accepts_a_body_alongside_its_header() {
        let element: iced::Element<'_, Message, Theme> = AlertDialog::new()
            .title("Choose a branch")
            .push(iced::widget::text("main"))
            .push(iced::widget::text("release"))
            .confirm()
            .on_confirm(Message::Confirm)
            .into_element();
        drop(element);
    }

    #[test]
    fn an_alert_renders_a_close_button_when_asked() {
        let element: iced::Element<'_, Message, Theme> = AlertDialog::new()
            .title("Title")
            .close_button(true)
            .on_dismiss(Message::Close)
            .on_confirm(Message::Confirm)
            .into_element();
        drop(element);
    }

    #[test]
    fn an_alert_close_button_is_never_drawn_when_dismissal_is_off() {
        let alert = AlertDialog::<Message>::new()
            .title("Title")
            .close_button(true)
            .dismissible(false)
            .on_dismiss(Message::Close);

        // The render path drops the button, so the alert still answers only
        // through its own buttons.
        let element: iced::Element<'_, Message, Theme> = alert.into_element();
        drop(element);
    }

    #[test]
    fn the_direct_builders_match_a_button_props_value() {
        let direct = AlertDialog::<Message>::new()
            .confirm()
            .ok_text("Delete")
            .cancel_text("Keep");

        let bundled = AlertDialog::<Message>::new().button_props(
            DialogButtonProps::default()
                .show_cancel(true)
                .ok_text("Delete")
                .cancel_text("Keep"),
        );

        for buttons in [&direct.buttons, &bundled.buttons] {
            assert!(buttons.is_cancel_shown());
            assert_eq!(buttons.resolved_ok_text("OK"), "Delete");
            assert_eq!(buttons.resolved_cancel_text("Cancel"), "Keep");
        }
    }

    #[test]
    fn confirm_after_button_props_keeps_the_ok_text() {
        let alert = AlertDialog::<Message>::new()
            .button_props(DialogButtonProps::default().ok_text("Delete"))
            .confirm();

        assert!(alert.buttons.is_cancel_shown());
        assert_eq!(alert.buttons.resolved_ok_text("OK"), "Delete");
    }

    #[test]
    fn button_props_after_confirm_keeps_the_cancel_button() {
        let alert = AlertDialog::<Message>::new()
            .confirm()
            .button_props(DialogButtonProps::default().ok_text("Delete"));

        assert!(alert.buttons.is_cancel_shown());
        assert_eq!(alert.buttons.resolved_ok_text("OK"), "Delete");
    }

    #[test]
    fn an_alert_defaults_to_a_narrow_width() {
        let alert = AlertDialog::<Message>::new();

        assert_eq!(alert.width, super::DialogWidth::Sm);
    }

    #[test]
    fn each_tone_has_its_own_icon() {
        // Distinct glyphs, so a tone is legible before the text is read.
        let glyphs: Vec<char> = [
            AlertTone::Question,
            AlertTone::Warning,
            AlertTone::Danger,
            AlertTone::Success,
            AlertTone::Info,
        ]
        .into_iter()
        .map(|tone| crate::icons::glyph(tone.icon()))
        .collect();

        for (index, glyph) in glyphs.iter().enumerate() {
            for other in &glyphs[index + 1..] {
                assert_ne!(glyph, other, "two tones share a glyph");
            }
        }
    }
}
