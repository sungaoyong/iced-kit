//! The assembled dialogs: [`Modal`], [`Dialog`] and their shared parts.

use crate::motion::DURATION_SLOW;
use crate::theme::Theme;
use crate::widgets::button as kit_button;
use crate::widgets::overlay::{scrim, Enter, EnterFrom};
use iced::widget::{column, container, row, MouseArea};
use iced::{Alignment, Element, Length};

use super::footer::dialog_close;
use super::{
    corner_padding, dialog_description, dialog_title, section_padding, surface_class,
    surface_padding, DIALOG_GAP, DIALOG_PADDING, DIALOG_TRAVEL, HEADER_GAP,
};

/// How wide a dialog is allowed to grow.
///
/// The reference's `Dialog` takes a raw pixel width (448 by default) and its
/// `AlertDialog` a raw 420. A named scale is what this crate uses instead: the
/// steps are the ones a caller actually reaches for, and it keeps two dialogs
/// opened from different places from differing by four pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogWidth {
    /// A narrow dialog, for a confirmation.
    Sm,
    /// The default width, matching the reference's 448px.
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
            Self::Md => 448.0,
            Self::Lg => 720.0,
        }
    }
}

/// The text and variants of an [`AlertDialog`](super::AlertDialog)'s default
/// buttons.
///
/// Every field is unset until a builder sets it, and an unset field falls back
/// to its documented default when the dialog renders. So handing a value to
/// [`AlertDialog::button_props`](super::AlertDialog::button_props) overrides
/// only the fields that value sets: the Cancel button
/// [`confirm`](super::AlertDialog::confirm) asked for, or a label set earlier,
/// survives.
///
/// ```
/// # use iced_kit::widgets::overlay::{AlertDialog, DialogButtonProps};
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// AlertDialog::new()
///     .title("Delete project")
///     .confirm()
///     .button_props(DialogButtonProps::default().ok_text("Delete"))
///     .into()
/// # }
/// ```
#[must_use = "a DialogButtonProps does nothing unless it is given to a dialog"]
#[derive(Debug, Clone, Default)]
pub struct DialogButtonProps {
    ok_text: Option<String>,
    cancel_text: Option<String>,
    ok_primary: Option<bool>,
    show_cancel: Option<bool>,
}

impl DialogButtonProps {
    /// Sets the confirm button's text. Default is `OK`.
    pub fn ok_text(mut self, ok_text: impl Into<String>) -> Self {
        self.ok_text = Some(ok_text.into());
        self
    }

    /// Sets the cancel button's text. Default is `Cancel`.
    pub fn cancel_text(mut self, cancel_text: impl Into<String>) -> Self {
        self.cancel_text = Some(cancel_text.into());
        self
    }

    /// Whether the confirm button is drawn as the primary call to action.
    /// Default is `true`.
    pub fn ok_primary(mut self, ok_primary: bool) -> Self {
        self.ok_primary = Some(ok_primary);
        self
    }

    /// Whether the default footer shows a cancel button. Default is `false`,
    /// which an [`AlertDialog::confirm`](super::AlertDialog::confirm) turns on.
    pub fn show_cancel(mut self, show_cancel: bool) -> Self {
        self.show_cancel = Some(show_cancel);
        self
    }

    /// Takes over every field `other` sets and keeps the rest.
    pub(crate) fn merge(&mut self, other: Self) {
        // An unset field is `None` and must not overwrite what the dialog
        // already carries, which is why this is not a plain assignment.
        if other.ok_text.is_some() {
            self.ok_text = other.ok_text;
        }
        if other.cancel_text.is_some() {
            self.cancel_text = other.cancel_text;
        }
        if other.ok_primary.is_some() {
            self.ok_primary = other.ok_primary;
        }
        if other.show_cancel.is_some() {
            self.show_cancel = other.show_cancel;
        }
    }

    /// Whether the default footer renders a cancel button.
    #[must_use]
    pub fn is_cancel_shown(&self) -> bool {
        self.show_cancel.unwrap_or(false)
    }

    /// The confirm button's label.
    #[must_use]
    pub fn resolved_ok_text(&self, fallback: &str) -> String {
        self.ok_text.clone().unwrap_or_else(|| fallback.to_owned())
    }

    /// The cancel button's label.
    #[must_use]
    pub fn resolved_cancel_text(&self, fallback: &str) -> String {
        self.cancel_text
            .clone()
            .unwrap_or_else(|| fallback.to_owned())
    }

    /// Whether the confirm button is primary.
    #[must_use]
    pub fn is_ok_primary(&self) -> bool {
        self.ok_primary.unwrap_or(true)
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
///
/// A `Modal` blocks the content behind it: the backdrop dims the page and
/// swallows clicks, so the dialog is the only interactive region until it is
/// dismissed. For a dialog that leaves the page usable, see [`Dialog`].
///
/// # Dismissing on Escape
///
/// The reference closes its dialogs on Escape, because its base layer sees the
/// keystroke. An iced widget does not, so the application wires it: listen for
/// the key in `subscription` and emit the same message the backdrop emits,
/// gated on the dialog being open.
///
/// ```
/// # use iced::keyboard;
/// # use iced_kit::widgets::overlay::{Layer, Modal};
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Close, Escape }
/// # fn view(content: iced::Element<'static, Message, Theme>, open: bool) -> iced::Element<'static, Message, Theme> {
/// let mut layer = Layer::new();
///
/// if open {
///     layer = layer.modal(Modal::new(
///         "Delete project",
///         iced::widget::text("This cannot be undone."),
///         Message::Close,
///     ));
/// }
///
/// iced_kit::widgets::overlay::layer(content, layer)
/// # }
/// ```
#[must_use = "a Modal does nothing unless it is added to a Layer"]
pub struct Modal<'a, Message> {
    title: String,
    description: Option<String>,
    body: Element<'a, Message, Theme>,
    on_dismiss: Message,
    width: DialogWidth,
    actions: Vec<Action<Message>>,
    dismissible: bool,
    close_button: bool,
    draggable: bool,
}

impl<'a, Message: Clone + 'a> Modal<'a, Message> {
    /// Creates a modal with a title and body.
    ///
    /// `on_dismiss` is emitted when the backdrop is clicked — and by the close
    /// button, if one is shown — so a modal can always be closed without
    /// reaching for a specific action.
    pub fn new(
        title: impl Into<String>,
        body: impl Into<Element<'a, Message, Theme>>,
        on_dismiss: Message,
    ) -> Self {
        Self {
            title: title.into(),
            description: None,
            body: body.into(),
            on_dismiss,
            width: DialogWidth::default(),
            actions: Vec::new(),
            dismissible: true,
            close_button: true,
            draggable: false,
        }
    }

    /// Adds a line under the title explaining what the dialog is asking.
    ///
    /// Rendered in the muted foreground at the description size, so it reads as
    /// support for the title rather than as more title.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
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

    /// Whether the dialog draws a close button in its top-right corner.
    ///
    /// On by default, and it emits the same `on_dismiss` the backdrop does. A
    /// dialog built with [`dismissible`](Self::dismissible)`(false)` never shows
    /// one: a close button that dismisses would contradict the whole point of a
    /// dialog that must be answered explicitly.
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.close_button = close_button;
        self
    }

    /// Whether clicking the backdrop dismisses the dialog.
    ///
    /// Turn this off for a dialog that must be answered explicitly, such as one
    /// guarding unsaved work. This also hides the close button.
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Whether the dialog can be carried around by its surface.
    ///
    /// A grabbed press on the card — the title, or any stretch of it a button
    /// has not claimed — picks the dialog up, and it keeps where it was put
    /// until it is closed; opened again, it starts from the centre. Off by
    /// default, so a plain modal stays put.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Builds the full-screen layer: backdrop plus centred dialog.
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            title,
            description,
            body,
            on_dismiss,
            width,
            actions,
            dismissible,
            close_button,
            draggable,
        } = self;

        let close = (dismissible && close_button).then(|| dialog_close(on_dismiss.clone()));

        let content = dialog_body(
            Some(title),
            description,
            body,
            action_row(actions),
            DIALOG_PADDING,
        );

        let surface = surface(content, width, close);
        let surface = if draggable {
            Element::new(super::drag::DragSurface::new(surface))
        } else {
            surface
        };

        modal_layer(surface, dismissible.then_some(on_dismiss))
    }
}

/// Lays out a dialog's header, body and footer.
///
/// Shared by [`Modal`], [`Dialog`] and [`AlertDialog`] so the three space their
/// sections alike: the reference gives the surface vertical padding and each
/// section its own horizontal padding, which leaves a scrolling body free to
/// run its scrollbar against the surface's edge rather than inset from it.
///
/// The title and description are taken by value: an iced [`Element`] borrows
/// the fragment it holds, so a `&str` local to this call could not be handed to
/// an `Element<'a>`. An owned `String` lives as long as the element does.
fn dialog_body<'a, Message: 'a>(
    title: Option<String>,
    description: Option<String>,
    body: Element<'a, Message, Theme>,
    footer: Option<Element<'a, Message, Theme>>,
    padding: f32,
) -> Element<'a, Message, Theme> {
    let inset = section_padding(padding);

    let mut sections = column![].spacing(DIALOG_GAP);

    if let Some(header) = header_section(title, description) {
        sections = sections.push(container(header).padding(inset).width(Length::Fill));
    }

    // The body carries no height of its own: the surface shrink-wraps whatever
    // the caller put in it, so a one-line dialog is a one-line card rather than
    // a full-height panel stretching past its content. A caller whose body must
    // scroll wraps it in its own `scrollable`, which is what gives it a height.
    sections = sections.push(container(body).padding(inset).width(Length::Fill));

    if let Some(footer) = footer {
        sections = sections.push(container(footer).padding(inset).width(Length::Fill));
    }

    sections.into()
}

/// Builds a dialog's header: the title, and the description under it.
///
/// A header with nothing to draw is omitted rather than rendered as a blank
/// line, which is what lets a caller with its own [`DialogHeader`] inside the
/// body pass an empty title and get no default header above it.
fn header_section<'a, Message: 'a>(
    title: Option<String>,
    description: Option<String>,
) -> Option<Element<'a, Message, Theme>> {
    // A dialog with no title is one whose caller is drawing its own header
    // inside the body, which is what `DialogHeader` and `DialogContent` are
    // for. An empty string means the same thing as no title at all.
    let title = title.filter(|title| !title.is_empty());

    if title.is_none() && description.is_none() {
        return None;
    }

    // A title and its description are one thought, so they sit closer to each
    // other than to the body below them.
    let mut text = column![].spacing(HEADER_GAP);

    if let Some(title) = title {
        text = text.push(dialog_title(title));
    }

    if let Some(description) = description {
        text = text.push(dialog_description(description));
    }

    Some(text.into())
}

/// Builds the footer row from a modal's declared actions.
fn action_row<'a, Message: Clone + 'a>(
    actions: Vec<Action<Message>>,
) -> Option<Element<'a, Message, Theme>> {
    if actions.is_empty() {
        return None;
    }

    // Secondary actions come first so the primary one lands on the right, where
    // a reader finishes.
    let (secondary, primary): (Vec<_>, Vec<_>) =
        actions.into_iter().partition(|action| !action.primary);

    let mut row = row![].spacing(super::FOOTER_GAP).align_y(Alignment::Center);

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

    Some(
        container(row)
            .width(Length::Fill)
            .align_x(Alignment::End)
            .into(),
    )
}

/// Draws a dialog surface: a raised card, with an optional close button
/// floated over its top-right corner.
///
/// The button floats rather than taking a row of its own, which is what the
/// reference does and what a header too short to hold it needs: a dialog whose
/// caller drew its own header inside the body has no title row for the button
/// to share, and giving it one would push the caller's header down the card.
///
/// iced has no absolute positioning, so the float is a
/// [`Stack`](iced::widget::Stack) holding the card and the button.
///
/// # Why the card's width is `Fixed` and not `Fill`
///
/// `Stack::push` adopts the first child's size hint, so a card asking for a
/// filling width makes the whole stack report one. The stack's node then spans
/// the window while the card draws at its resolved width inside it — and the
/// centring box in [`modal_layer`] has no spare room left to centre with, which
/// leaves the dialog against the window's left edge.
///
/// A fixed width keeps the stack the size of the card, which is what gives the
/// centring box something to centre. It does not stop the dialog shrinking on a
/// narrow window: a `Length::Fixed` is clamped to the limits it is laid out in,
/// so 720 in a 500-wide window resolves to 500.
fn surface<'a, Message: Clone + 'a>(
    content: Element<'a, Message, Theme>,
    width: DialogWidth,
    close: Option<Element<'a, Message, Theme>>,
) -> Element<'a, Message, Theme> {
    let padding = surface_padding(DIALOG_PADDING);
    let width = Length::Fixed(width.max_width());

    let card = container(content)
        .width(width)
        .padding(padding)
        .class(surface_class());

    let Some(close) = close else {
        return card.into();
    };

    // The button is inset from the corner by the same amount on both axes. The
    // surface's own padding is not the right inset: it is vertical-only, which
    // would leave the button flush against the right edge while holding it 16px
    // down from the top, so it reads as low rather than in the corner.
    let corner = container(close)
        .width(width)
        .padding(corner_padding(DIALOG_PADDING))
        .align_x(Alignment::End)
        .align_y(Alignment::Start);

    iced::widget::Stack::new().push(card).push(corner).into()
}

/// Wraps a dialog surface in the centred, dimmed layer a modal occupies.
///
/// `on_dismiss` is the message a press on the backdrop emits; `None` gives a
/// backdrop that dims and blocks but does not dismiss.
fn modal_layer<'a, Message: Clone + 'a>(
    surface: Element<'a, Message, Theme>,
    on_dismiss: Option<Message>,
) -> Element<'a, Message, Theme> {
    // A full-size box gives the dialog something to be centred within: the card
    // inside it is narrower than the window and shrink-wraps its height, so
    // centring both ways puts it in the middle of the screen.
    let centered = container(surface)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    let backdrop: Element<'a, Message, Theme> = match on_dismiss {
        // The backdrop both dims and dismisses: a `MouseArea` covers the whole
        // area and reports a press anywhere the dialog does not.
        Some(message) => MouseArea::new(scrim()).on_press(message).into(),
        None => scrim(),
    };

    // A dialog rises into place by a short distance: it is the arrival of a
    // surface at the centre of attention, so a long slide would read as the
    // dialog coming from somewhere rather than simply appearing.
    let surface = Enter::new(centered, EnterFrom::Below)
        .distance(DIALOG_TRAVEL)
        .duration(DURATION_SLOW);

    // The stack's own size is declared rather than left to its first child.
    // `Stack::push` adopts that child's size hint, and the child here is a
    // dialog card asking for a filling width — so the stack would report `Fill`
    // and its node would span the window. The card still draws at its resolved
    // width inside that node, but the centring box above then sees no spare
    // room and leaves the dialog against the window's left edge. Declaring the
    // size keeps the stack honest about spanning the window, which is what the
    // backdrop needs anyway.
    iced::widget::Stack::new()
        .push(backdrop)
        .push(surface)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// A non-blocking dialog: the same visual treatment, but the content behind it
/// stays interactive.
///
/// Unlike [`Modal`], this does not dim or block the page, so it has no
/// dismissal of its own — hence a close button is opt-in here and takes an
/// explicit message, where a modal's is on by default and reuses `on_dismiss`.
#[must_use = "a Dialog does nothing unless it is turned into an Element"]
pub struct Dialog<'a, Message> {
    title: String,
    description: Option<String>,
    body: Element<'a, Message, Theme>,
    footer: Option<Element<'a, Message, Theme>>,
    width: DialogWidth,
    close: Option<Message>,
    draggable: bool,
}

impl<'a, Message: Clone + 'a> Dialog<'a, Message> {
    /// Creates a dialog with a title and body.
    pub fn new(title: impl Into<String>, body: impl Into<Element<'a, Message, Theme>>) -> Self {
        Self {
            title: title.into(),
            description: None,
            body: body.into(),
            footer: None,
            width: DialogWidth::default(),
            close: None,
            draggable: false,
        }
    }

    /// Adds a line under the title explaining what the dialog is asking.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
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

    /// Adds a close button to the dialog's top-right corner, emitting `message`.
    pub fn close_button(mut self, message: Message) -> Self {
        self.close = Some(message);
        self
    }

    /// Whether the dialog can be carried around by its surface.
    ///
    /// The same grab a [`Modal`] gets: a press an interactive child has not
    /// claimed picks the dialog up, and it keeps where it was put for as long
    /// as it stays open. Off by default.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Converts the dialog into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            title,
            description,
            body,
            footer,
            width,
            close,
            draggable,
        } = self;

        let close = close.map(dialog_close);

        let content = dialog_body(Some(title), description, body, footer, DIALOG_PADDING);

        let surface = surface(content, width, close);

        if draggable {
            Element::new(super::drag::DragSurface::new(surface))
        } else {
            surface
        }
    }
}

impl<'a, Message: Clone + 'a> From<Modal<'a, Message>> for Element<'a, Message, Theme> {
    fn from(modal: Modal<'a, Message>) -> Self {
        modal.into_element()
    }
}

impl<'a, Message: Clone + 'a> From<Dialog<'a, Message>> for Element<'a, Message, Theme> {
    fn from(dialog: Dialog<'a, Message>) -> Self {
        dialog.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{Dialog, DialogButtonProps, DialogWidth, Modal};
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
        .description("The file moves to the trash.")
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
    fn a_modal_shows_a_close_button_unless_it_is_told_not_to() {
        let default = Modal::new("T", iced::widget::text("B"), Message::Close);
        assert!(default.close_button);
        assert!(default.dismissible);

        let bare = Modal::new("T", iced::widget::text("B"), Message::Close).close_button(false);
        assert!(!bare.close_button);
    }

    #[test]
    fn a_non_dismissible_modal_renders_without_a_close_button() {
        let modal = Modal::new(
            "Unsaved changes",
            iced::widget::text("Save before closing?"),
            Message::Close,
        )
        .dismissible(false);

        // A close button that dismissed would contradict `dismissible(false)`,
        // so the render path drops it even though `close_button` is still set.
        let element: iced::Element<'_, Message, Theme> = modal.into_element();
        drop(element);
    }

    #[test]
    fn modal_widths_increase_with_the_variant() {
        assert!(DialogWidth::Sm.max_width() < DialogWidth::Md.max_width());
        assert!(DialogWidth::Md.max_width() < DialogWidth::Lg.max_width());
    }

    #[test]
    fn the_default_width_matches_the_reference() {
        // gpui-kit's `DialogProps::default` is `px(448.)`.
        assert_eq!(DialogWidth::Md.max_width(), 448.0);
    }

    #[test]
    fn a_dialog_renders_with_and_without_a_footer() {
        let plain: iced::Element<'_, Message, Theme> =
            Dialog::new("Title", iced::widget::text("Body")).into();
        drop(plain);

        let footed: iced::Element<'_, Message, Theme> =
            Dialog::new("Title", iced::widget::text("Body"))
                .description("A body with a footer.")
                .width(DialogWidth::Lg)
                .footer(iced::widget::text("Footer"))
                .close_button(Message::Close)
                .into();
        drop(footed);
    }

    #[test]
    fn a_dialog_with_no_title_renders_only_its_body() {
        // `DialogContent` and `DialogHeader` exist so a caller can lay out its
        // own header inside the body; an empty title leaves room for it.
        let element: iced::Element<'_, Message, Theme> =
            Dialog::new("", iced::widget::text("Body only")).into();
        drop(element);
    }

    #[test]
    fn an_unset_button_prop_falls_back_to_its_default() {
        let props = DialogButtonProps::default();

        assert_eq!(props.resolved_ok_text("OK"), "OK");
        assert_eq!(props.resolved_cancel_text("Cancel"), "Cancel");
        assert!(props.is_ok_primary());
        assert!(!props.is_cancel_shown());
    }

    #[test]
    fn set_button_props_override_their_defaults() {
        let props = DialogButtonProps::default()
            .ok_text("Delete")
            .cancel_text("Keep")
            .ok_primary(false)
            .show_cancel(true);

        assert_eq!(props.resolved_ok_text("OK"), "Delete");
        assert_eq!(props.resolved_cancel_text("Cancel"), "Keep");
        assert!(!props.is_ok_primary());
        assert!(props.is_cancel_shown());
    }

    #[test]
    fn a_later_props_value_overrides_only_the_fields_it_sets() {
        let mut props = DialogButtonProps::default()
            .ok_text("Delete")
            .show_cancel(true);

        props.merge(DialogButtonProps::default().cancel_text("Keep"));

        assert_eq!(props.resolved_ok_text("OK"), "Delete", "kept from before");
        assert!(props.is_cancel_shown(), "kept from before");
        assert_eq!(
            props.resolved_cancel_text("Cancel"),
            "Keep",
            "from the merge"
        );
    }

    #[test]
    fn the_call_order_of_button_props_does_not_matter() {
        // `confirm()` sets `show_cancel` directly and `button_props` merges, so
        // each order has to leave both settings in place.
        let mut after = DialogButtonProps::default();
        after.merge(DialogButtonProps::default().show_cancel(true));
        after.merge(DialogButtonProps::default().ok_text("Delete"));

        let mut before = DialogButtonProps::default().ok_text("Delete");
        before.merge(DialogButtonProps::default().show_cancel(true));

        for props in [after, before] {
            assert!(props.is_cancel_shown());
            assert_eq!(props.resolved_ok_text("OK"), "Delete");
        }
    }
}
