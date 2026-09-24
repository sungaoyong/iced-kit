//! Overlays: modals, toasts, tooltips and dropdown menus.
//!
//! # The layering model
//!
//! iced has no window-level z-order, so every overlay is a child of a
//! [`Stack`](iced::widget::stack()). [`layer`] is the single place that assembles
//! them, in paint order:
//!
//! 1. the application content
//! 2. open dropdowns (so they float above the page)
//! 3. a drawer, which leaves the page visible beside it
//! 4. the modal backdrop and dialog (so a modal opened from a drawer covers it)
//! 5. toasts (always on top, since they report outcomes)
//!
//! Applications call [`layer`] once, wrapping their root view.
//!
//! ```
//! # use iced_kit::widgets::overlay::{self, Layer};
//! # use iced_kit::Theme;
//! # #[derive(Clone, Debug)] enum Message { Close }
//! # fn view(content: iced::Element<'static, Message, Theme>) -> iced::Element<'static, Message, Theme> {
//! // Pass whatever is open this frame; `Layer::default()` means nothing is.
//! overlay::layer(content, Layer::default())
//! # }
//! ```

pub mod dialog;
pub mod drawer;
pub mod dropdown;
mod enter;
pub mod menu;
pub mod popover;
pub mod toast;
pub mod tooltip;

use crate::theme::Theme;
use iced::widget::{container, stack, Space, Stack};
use iced::{Alignment, Color, Element, Length};

pub use dialog::{
    dialog_actions, dialog_close, dialog_description, dialog_title, header_with_icon, AlertDialog,
    AlertTone, Dialog, DialogButtonProps, DialogContent, DialogFooter, DialogHeader, DialogWidth,
    Modal,
};
pub use drawer::{
    drawer_actions, drawer_header, sheet, Drawer, DrawerSide, DrawerSize, HoverCard,
    HoverCardPlacement, SHEET_TOP_INSET,
};
pub use dropdown::{Dropdown, DropdownAlign, MenuItem};
pub use enter::EnterFrom;
pub use menu::OpenFlag;
pub use popover::{ContextMenu, Popover, PopoverPlacement};
pub use toast::{Toast, ToastKind, ToastPlacement, Toasts};
pub use tooltip::{
    tooltip, tooltip_at, tooltip_at_with_shortcut, tooltip_bubble, tooltip_bubble_with_shortcut,
    tooltip_with_shortcut, TooltipPosition,
};

pub(crate) use enter::Enter;

/// A downward chevron, for the trigger of a menu.
///
/// It is drawn as a glyph in the bundled icon font rather than as an SVG so it
/// inherits the surrounding text color without extra plumbing: a caret sits
/// inside a button whose color depends on its variant, and iced's SVG widget
/// would ignore that color.
///
/// # Why it is boxed and centered
///
/// A bare glyph advances by its own metrics and sits on the text baseline, which
/// for a chevron means it reads as small as its advance width and rides high
/// against the label beside it. Centering it in a square of the control's icon
/// size is what lines it up with that label.
///
/// The square is a step larger than an icon slot: the chevron's strokes are
/// thin, so at the icon size exactly it would look lighter than the text it
/// sits beside.
#[must_use]
pub fn caret<'a, Message: 'a>(size: crate::theme::Size) -> Element<'a, Message, Theme> {
    let box_side = (size.icon_size() * 1.25).round();

    container(
        crate::widgets::Icon::new(crate::icons::IconName::ChevronDown)
            .size(box_side)
            .into_element(size),
    )
    .width(Length::Fixed(box_side))
    .height(Length::Fixed(box_side))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .into()
}

/// Everything that can be open over the application content this frame.
///
/// Construct it with [`Layer::default`] and set only what is open, so adding a
/// new overlay kind does not break existing call sites.
#[must_use = "a Layer does nothing unless it is given to `layer`"]
pub struct Layer<'a, Message> {
    /// Menus anchored to a point or a widget.
    dropdowns: Vec<Element<'a, Message, Theme>>,
    /// A modal dialog, which blocks the content behind it.
    ///
    /// Held as an element rather than as a [`Modal`] so an [`AlertDialog`] or a
    /// hand-assembled dialog can be layered too; each brings its own backdrop.
    modal: Option<Element<'a, Message, Theme>>,
    /// A drawer, which slides in from an edge and leaves the page visible.
    drawer: Option<Drawer<'a, Message>>,
    /// Transient notifications, stacked in the corner.
    toasts: Toasts<'a, Message>,
}

impl<'a, Message: Clone + 'a> Default for Layer<'a, Message> {
    fn default() -> Self {
        Self {
            dropdowns: Vec::new(),
            modal: None,
            drawer: None,
            toasts: Toasts::new(),
        }
    }
}

impl<'a, Message: Clone + 'a> Layer<'a, Message> {
    /// Creates an empty layer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an open dropdown to the layer.
    pub fn dropdown(mut self, dropdown: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.dropdowns.push(dropdown.into());
        self
    }

    /// Sets the modal dialog.
    ///
    /// Takes anything that becomes an element, so an [`AlertDialog`] — or a
    /// dialog body a caller assembled from
    /// [`DialogHeader`] and its siblings — is layered the same way a
    /// [`Modal`] is. The element is expected to bring its own backdrop; the
    /// assembled dialogs all do.
    pub fn modal(mut self, modal: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.modal = Some(modal.into());
        self
    }

    /// Sets the drawer.
    pub fn drawer(mut self, drawer: Drawer<'a, Message>) -> Self {
        self.drawer = Some(drawer);
        self
    }

    /// Sets the toast stack.
    pub fn toasts(mut self, toasts: Toasts<'a, Message>) -> Self {
        self.toasts = toasts;
        self
    }

    /// Whether anything is currently open.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dropdowns.is_empty()
            && self.modal.is_none()
            && self.drawer.is_none()
            && self.toasts.is_empty()
    }
}

/// Wraps application `content` in the overlay layer.
#[must_use]
pub fn layer<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
    open: Layer<'a, Message>,
) -> Element<'a, Message, Theme> {
    let Layer {
        dropdowns,
        modal,
        drawer,
        toasts,
    } = open;

    // A fast path when nothing is open: skipping the `Stack` avoids paying for
    // its layout pass on every frame of an idle application.
    let any_dropdown = !dropdowns.is_empty();

    if !any_dropdown && modal.is_none() && drawer.is_none() && toasts.is_empty() {
        return content.into();
    }

    // `Stack` sizes its overlay children from the base layer, so the content is
    // forced to the full area; otherwise a shrink-wrapping content view would
    // leave the overlays with a tiny area to occupy.
    let base = container(content).width(Length::Fill).height(Length::Fill);

    let mut layers: Stack<'a, Message, Theme> = stack![base];

    for dropdown in dropdowns {
        // A dropdown is positioned by its own wrapper; the layer only gives it
        // the full area to place itself within.
        layers = layers.push(
            container(dropdown)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Start)
                .align_y(Alignment::Start),
        );
    }

    // The drawer sits under the modal: a modal opened from a drawer must cover it.
    if let Some(drawer) = drawer {
        layers = layers.push(drawer.into_element());
    }

    if let Some(modal) = modal {
        layers = layers.push(modal);
    }

    if !toasts.is_empty() {
        layers = layers.push(toasts.into_element());
    }

    layers.into()
}

/// The full-area scrim drawn behind a modal.
///
/// It is a separate widget so a modal's backdrop can be styled or reused by
/// other blocking overlays.
#[must_use]
pub fn scrim<'a, Message: 'a>() -> Element<'a, Message, Theme> {
    container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .class(Box::new(|theme: &Theme| container::Style {
            // A translucent black dims the page in both palettes; using the
            // foreground color would lighten it in dark mode.
            background: Some(iced::Background::Color(Color::from_rgba(
                0.0,
                0.0,
                0.0,
                if theme.is_dark() { 0.6 } else { 0.4 },
            ))),
            ..container::Style::default()
        }) as container::StyleFn<'a, Theme>)
        .into()
}

/// Shared shadow used by every floating surface (menus, dialogs, toasts).
pub(crate) fn floating_shadow(theme: &Theme) -> iced::Shadow {
    iced::Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, if theme.is_dark() { 0.5 } else { 0.15 }),
        offset: iced::Vector::new(0.0, 8.0),
        blur_radius: 24.0,
    }
}

#[cfg(test)]
mod tests {
    use super::{layer, scrim, Layer};
    use crate::theme::Theme;
    use crate::widgets::{button, Modal};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Close,
        Confirm,
    }

    #[test]
    fn an_empty_layer_returns_the_content_untouched() {
        let content: iced::Element<'_, Message, Theme> = button("Hi").into();
        let layered = layer(content, Layer::default());

        // Rendered as-is, with no stack in between.
        let _: iced::Element<'_, Message, Theme> = layered;
    }

    #[test]
    fn a_layer_reports_whether_it_is_empty() {
        let empty: Layer<'_, Message> = Layer::new();
        assert!(empty.is_empty());

        let with_modal = Layer::new().modal(Modal::new(
            "Title",
            iced::widget::text("Body"),
            Message::Close,
        ));
        assert!(!with_modal.is_empty());
    }

    #[test]
    fn a_modal_layer_renders() {
        let content: iced::Element<'_, Message, Theme> = button("Open").into();
        let open = Layer::new().modal(
            Modal::new(
                "Confirm",
                iced::widget::text("Are you sure?"),
                Message::Close,
            )
            .confirm("Yes", Message::Confirm),
        );

        let element: iced::Element<'_, Message, Theme> = layer(content, open);
        drop(element);
    }

    #[test]
    fn a_scrim_renders() {
        let element: iced::Element<'_, Message, Theme> = scrim();
        drop(element);
    }

    #[test]
    fn a_toast_layer_renders() {
        use super::{Toast, ToastKind};

        let content: iced::Element<'_, Message, Theme> = button("Hi").into();
        let open = Layer::new()
            .toasts(crate::widgets::Toasts::new().push(Toast::new("Saved", ToastKind::Success)));

        let element: iced::Element<'_, Message, Theme> = layer(content, open);
        drop(element);
    }
}
