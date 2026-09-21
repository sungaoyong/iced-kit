//! Split buttons: an action paired with a menu trigger.

use crate::theme::catalog::{ButtonVariant, Corners};
use crate::theme::{Size, Theme};
use crate::widgets::button::Button;
use iced::{Element, Length};

/// Which corners the action half of a split button rounds.
const ACTION_CORNERS: Corners = Corners {
    top_left: true,
    top_right: false,
    bottom_right: false,
    bottom_left: true,
};

/// Which corners the trigger half of a split button rounds.
const TRIGGER_CORNERS: Corners = Corners {
    top_left: false,
    top_right: true,
    bottom_right: true,
    bottom_left: false,
};

/// A button with an attached menu trigger, drawn as one control.
///
/// The two halves share a variant and size, and only the outer edges are
/// rounded, so they read as a single split control rather than as two buttons
/// that happen to be adjacent.
///
/// # Opening the menu
///
/// The trigger reports a press rather than holding a menu. iced has no
/// window-level z-order, so a menu is positioned by the application and drawn
/// through the [`Layer`](crate::widgets::overlay::Layer). This control reports
/// the intent to toggle, and the application owns the open flag and the menu's
/// placement — the same division of labour as every other iced component.
///
/// ```
/// # use iced_kit::widgets::button::{Button, DropdownButton};
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Save, ToggleMenu }
/// # fn view() -> iced::Element<'static, Message, Theme> {
/// DropdownButton::new(Button::new("Save").primary().on_press(Message::Save))
///     .on_toggle(Message::ToggleMenu)
///     .into()
/// # }
/// ```
#[must_use = "a DropdownButton does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct DropdownButton<'a, Message> {
    action: Option<Button<'a, Message>>,
    variant: Option<ButtonVariant>,
    size: Option<Size>,
    outline: bool,
    compact: bool,
    selected: bool,
    disabled: bool,
    /// Held by the application, so the trigger stays visibly pressed while the
    /// menu is up.
    open: bool,
    width: Option<Length>,
    on_toggle: Option<Message>,
    tooltip: Option<String>,
}

impl<'a, Message: Clone + 'a> DropdownButton<'a, Message> {
    /// Creates a split button from its action half.
    pub fn new(action: Button<'a, Message>) -> Self {
        Self {
            action: Some(action),
            variant: None,
            size: None,
            outline: false,
            compact: false,
            selected: false,
            disabled: false,
            open: false,
            width: None,
            on_toggle: None,
            tooltip: None,
        }
    }

    /// Creates a split button whose only content is the menu trigger.
    ///
    /// Use this when the trigger stands alone — an overflow menu, a settings
    /// button — with no paired action beside it.
    pub fn trigger_only() -> Self {
        Self {
            action: None,
            variant: None,
            size: None,
            outline: false,
            compact: false,
            selected: false,
            disabled: false,
            open: false,
            width: None,
            on_toggle: None,
            tooltip: None,
        }
    }

    /// Sets the variant both halves use.
    ///
    /// A variant set here wins over the action button's own; without it, the
    /// action keeps what it was given and the trigger copies it.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Sets the size both halves use.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Draws both halves as outlines.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Tightens both halves' padding.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Draws both halves in their selected state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Makes the whole control inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Marks the menu as open, so the trigger stays visibly pressed.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sets the control's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the message the trigger emits.
    ///
    /// It is emitted on every press, so the application toggles its open flag
    /// rather than setting it — a second press closes the menu.
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.on_toggle = Some(message);
        self
    }

    /// Attaches a tooltip to the trigger half.
    ///
    /// The trigger's content is a bare caret, which announces nothing useful on
    /// its own.
    pub fn trigger_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// The variant both halves draw with, falling back to the action's own.
    #[must_use]
    pub fn effective_variant(&self) -> ButtonVariant {
        self.variant
            .or_else(|| self.action.as_ref().map(|action| action.class().variant))
            .unwrap_or_default()
    }

    /// The size both halves draw at, falling back to the action's own.
    #[must_use]
    pub fn effective_size(&self) -> Size {
        self.size
            .or_else(|| self.action.as_ref().map(|action| action.class().size))
            .unwrap_or_default()
    }

    /// Converts the split button into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            action,
            variant,
            size,
            outline,
            compact,
            selected,
            disabled,
            open,
            width,
            on_toggle,
            tooltip,
        } = self;

        // Resolved before the action is consumed, so the trigger can share
        // whatever the action turned out to be using.
        let variant = variant
            .or_else(|| action.as_ref().map(|action| action.class().variant))
            .unwrap_or_default();
        let size = size
            .or_else(|| action.as_ref().map(|action| action.class().size))
            .unwrap_or_default();

        let has_action = action.is_some();
        let mut halves: Vec<Element<'a, Message, Theme>> = Vec::new();

        if let Some(action) = action {
            let mut action = action.corners(ACTION_CORNERS).disabled(disabled);

            // The outer control is authoritative, so its variant and size win
            // over whatever the action half was built with.
            action = action.variant(variant).size(size);

            if outline {
                action = action.outline();
            }
            if compact {
                action = action.compact();
            }
            if selected {
                action = action.selected(true);
            }

            halves.push(action.into_element());
        }

        // The trigger keeps its handler even while the menu is open, so the
        // same message closes it.
        let mut trigger = Button::icon_only()
            .dropdown_caret()
            .corners(if has_action {
                TRIGGER_CORNERS
            } else {
                Corners::ALL
            })
            .variant(variant)
            .size(size)
            .selected(selected || open)
            .disabled(disabled);

        if outline {
            trigger = trigger.outline();
        }
        if compact {
            trigger = trigger.compact();
        }
        if let Some(tooltip) = tooltip {
            trigger = trigger.tooltip(tooltip);
        }
        if let Some(on_toggle) = on_toggle {
            trigger = trigger.on_press(on_toggle);
        }

        halves.push(trigger.into_element());

        let row: Element<'a, Message, Theme> = iced::widget::Row::with_children(halves)
            .align_y(iced::Alignment::Center)
            .into();

        let mut container = iced::widget::container(row);
        if let Some(width) = width {
            container = container.width(width);
        }

        container.into()
    }
}

impl<'a, Message: Clone + 'a> From<DropdownButton<'a, Message>> for Element<'a, Message, Theme> {
    fn from(button: DropdownButton<'a, Message>) -> Self {
        button.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::DropdownButton;
    use crate::theme::catalog::ButtonVariant;
    use crate::theme::Size;
    use crate::widgets::button::Button;
    use crate::Theme;
    use iced::Element;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Saved,
        Toggled,
    }

    /// An unset variant or size leaves the action half's own to supply both, so
    /// a caller can style the split from either level.
    #[test]
    fn the_action_half_supplies_the_defaults() {
        let split = DropdownButton::new(
            Button::new("Save")
                .danger()
                .size(Size::Sm)
                .on_press(Message::Saved),
        );

        assert_eq!(split.effective_variant(), ButtonVariant::Danger);
        assert_eq!(split.effective_size(), Size::Sm);
    }

    /// An explicit outer value wins over the action half's, so the two halves
    /// cannot disagree about their variant.
    #[test]
    fn the_outer_variant_overrides_the_action_half() {
        let split = DropdownButton::new(Button::new("Save").danger().on_press(Message::Saved))
            .variant(ButtonVariant::Primary)
            .size(Size::Lg);

        assert_eq!(split.effective_variant(), ButtonVariant::Primary);
        assert_eq!(split.effective_size(), Size::Lg);
    }

    #[test]
    fn both_forms_render() {
        let split: Element<'_, Message, Theme> =
            DropdownButton::new(Button::new("Save").primary().on_press(Message::Saved))
                .on_toggle(Message::Toggled)
                .trigger_tooltip("More actions")
                .open(true)
                .into();
        drop(split);

        let trigger_only: Element<'_, Message, Theme> = DropdownButton::<Message>::trigger_only()
            .on_toggle(Message::Toggled)
            .into();
        drop(trigger_only);
    }

    #[test]
    fn a_disabled_split_renders() {
        let element: Element<'_, Message, Theme> =
            DropdownButton::new(Button::new("Save").on_press(Message::Saved))
                .on_toggle(Message::Toggled)
                .disabled(true)
                .into();
        drop(element);
    }
}
