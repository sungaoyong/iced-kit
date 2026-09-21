//! Toggle buttons and segmented toggle groups.
//!
//! These are the pressed-state controls: a single button that stays latched, or
//! a row of them joined like a [`ButtonGroup`](crate::widgets::button::ButtonGroup).
//! They are distinct from [`switch`](crate::widgets::switch), which is the
//! sliding on/off control.

use crate::theme::catalog::{ButtonClass, ButtonRounded, ButtonState, ButtonVariant, Corners};
use crate::theme::{Size, Theme};
use crate::widgets::button::group::{member_corners, ButtonGroupLayout};
use crate::widgets::button::icon::Icon;
use crate::widgets::overlay::{tooltip_at, TooltipPosition};
use iced::widget::button as iced_button;
use iced::{Element, Length};

/// How a toggle is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToggleVariant {
    /// No surface until hovered or pressed, which is what upstream defaults to.
    #[default]
    Ghost,
    /// A bordered surface at rest.
    Outline,
}

impl ToggleVariant {
    /// The button variant this toggle style resolves to.
    fn button_variant(self) -> ButtonVariant {
        match self {
            // A ghost toggle is transparent until it matters, so it takes the
            // ghost palette.
            Self::Ghost => ButtonVariant::Ghost,
            Self::Outline => ButtonVariant::Default,
        }
    }
}

/// A button that stays pressed.
///
/// # Checked versus selected
///
/// A toggle's `checked` state is its own business: it is the value the control
/// represents, and the caller owns it, passing it back in on each render.
#[must_use = "a Toggle does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct Toggle<'a, Message> {
    label: Option<iced::widget::text::Fragment<'a>>,
    icon: Option<Icon>,
    checked: bool,
    variant: ToggleVariant,
    size: Size,
    disabled: bool,
    corners: Corners,
    tooltip: Option<(String, TooltipPosition)>,
    width: Option<Length>,
    /// Emits the new checked state when the toggle is pressed.
    on_change: Option<Box<dyn Fn(bool) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> Toggle<'a, Message> {
    /// Creates a toggle with the given label.
    pub fn new(label: impl iced::widget::text::IntoFragment<'a>) -> Self {
        Self {
            label: Some(label.into_fragment()),
            icon: None,
            checked: false,
            variant: ToggleVariant::default(),
            size: Size::Md,
            disabled: false,
            corners: Corners::ALL,
            tooltip: None,
            width: None,
            on_change: None,
        }
    }

    /// Creates a toggle with no label, for a toolbar of icon toggles.
    pub fn icon_only() -> Self {
        Self {
            label: None,
            ..Self::new("")
        }
    }

    /// Sets the checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Adds a leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets how the toggle is drawn.
    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Draws the toggle with a bordered surface at rest.
    pub fn outline(self) -> Self {
        self.variant(ToggleVariant::Outline)
    }

    /// Sets the toggle's size.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Makes the toggle inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the toggle's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the message to emit with the new checked state.
    pub fn on_toggle(mut self, on_toggle: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_change = Some(Box::new(on_toggle));
        self
    }

    /// Attaches a tooltip.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some((tooltip.into(), TooltipPosition::default()));
        self
    }

    /// Overrides the corner mask, for a toggle that is one segment of a group.
    pub fn corners(mut self, corners: Corners) -> Self {
        self.corners = corners;
        self
    }

    /// Whether the toggle is currently checked.
    #[must_use]
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Converts the toggle into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            label,
            icon,
            checked,
            variant,
            size,
            disabled,
            corners,
            tooltip,
            width,
            on_change,
        } = self;

        let class = ButtonClass {
            variant: variant.button_variant(),
            size,
            outline: variant == ToggleVariant::Outline,
            // A checked toggle is the pressed state, so it draws the selection
            // palette; the same value also drives the accessibility metadata.
            selected: checked,
            loading: false,
            disabled,
            rounded: ButtonRounded::default(),
            corners,
        };

        let text_style = size.text();
        // The line box matches the control, which is what centres the glyph:
        // iced anchors a paragraph by its own glyph bounds, so a line box sized
        // to the text alone would leave the label sitting against the top of a
        // taller toggle.
        let line_height = iced::Pixels(size.height().max(text_style.line_height));
        let content: Element<'a, Message, Theme> = match (icon, label) {
            (Some(icon), label) => {
                let mut parts: Vec<Element<'a, Message, Theme>> = vec![icon.into_element(size)];
                if let Some(label) = label.filter(|label| !label.is_empty()) {
                    parts.push(
                        iced::widget::text(label)
                            .size(text_style.size)
                            .line_height(line_height)
                            .into(),
                    );
                }
                let parts = if parts.len() == 1 {
                    parts.pop().expect("just checked the length")
                } else {
                    iced::widget::Row::with_children(parts)
                        .spacing(size.gap())
                        .align_y(iced::Alignment::Center)
                        .into()
                };
                parts
            }
            (None, Some(label)) => iced::widget::text(label)
                .size(text_style.size)
                .line_height(line_height)
                .into(),
            (None, None) => iced::widget::Space::new()
                .width(Length::Fixed(size.height()))
                .into(),
        };

        let padding = if variant == ToggleVariant::Ghost && !checked {
            4.0
        } else {
            size.padding() * 0.6
        };

        let mut widget = iced_button(content)
            .padding(iced::Padding {
                top: 0.0,
                right: padding,
                bottom: 0.0,
                left: padding,
            })
            .class(Box::new(move |theme: &Theme, status| {
                let state = match status {
                    // iced reports `Disabled` only for a button with no
                    // handler, so the toggle's own flag is consulted first.
                    _ if disabled => ButtonState::Disabled,
                    iced_button::Status::Active => {
                        if checked {
                            ButtonState::Selected
                        } else {
                            ButtonState::Normal
                        }
                    }
                    iced_button::Status::Hovered => ButtonState::Hovered,
                    iced_button::Status::Pressed => ButtonState::Active,
                    iced_button::Status::Disabled => ButtonState::Disabled,
                };

                let appearance = class.appearance(theme, state);

                iced_button::Style {
                    background: appearance.background,
                    text_color: appearance.text_color,
                    border: appearance.border,
                    shadow: appearance.shadow,
                    snap: true,
                }
            }) as iced_button::StyleFn<'a, Theme>);

        if let Some(width) = width {
            widget = widget.width(width);
        } else {
            widget = widget.height(Length::Fixed(size.height()));
        }

        // A checked toggle is latched, so the same control unlatches it: the
        // new value is the negation of the one the caller passed in.
        if let Some(on_change) = on_change {
            if !disabled {
                widget = widget.on_press(on_change(!checked));
            }
        }

        let element: Element<'a, Message, Theme> = widget.into();

        match tooltip {
            Some((label, position)) => tooltip_at(element, label, position),
            None => element,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Toggle<'a, Message>> for Element<'a, Message, Theme> {
    fn from(toggle: Toggle<'a, Message>) -> Self {
        toggle.into_element()
    }
}

/// A row of toggles treated as one control.
///
/// In its segmented form the borders of adjacent members are joined, so the
/// group reads as a single multi-position switch.
///
/// ```
/// # use iced_kit::widgets::button::{Toggle, ToggleGroup};
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Changed(Vec<bool>) }
/// # fn view(state: [bool; 3]) -> iced::Element<'static, Message, Theme> {
/// ToggleGroup::new()
///     .push(Toggle::new("Bold").checked(state[0]))
///     .push(Toggle::new("Italic").checked(state[1]))
///     .on_change(|next| Message::Changed(next))
///     .into()
/// # }
/// ```
#[must_use = "a ToggleGroup does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct ToggleGroup<'a, Message> {
    items: Vec<Toggle<'a, Message>>,
    variant: ToggleVariant,
    size: Option<Size>,
    disabled: bool,
    segmented: bool,
    width: Option<Length>,
    /// Shared rather than owned so each member's handler can hold a reference
    /// to the same callback: a `Box` cannot be cloned into every member.
    on_change: Option<std::rc::Rc<dyn Fn(Vec<bool>) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> ToggleGroup<'a, Message> {
    /// Creates an empty group.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            variant: ToggleVariant::default(),
            size: None,
            disabled: false,
            segmented: false,
            width: None,
            on_change: None,
        }
    }

    /// Adds a member.
    pub fn push(mut self, toggle: Toggle<'a, Message>) -> Self {
        self.items.push(toggle);
        self
    }

    /// Adds several members.
    pub fn extend(mut self, toggles: impl IntoIterator<Item = Toggle<'a, Message>>) -> Self {
        self.items.extend(toggles);
        self
    }

    /// Sets the variant every member uses.
    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Draws every member with a bordered surface at rest.
    ///
    /// This is the form a segmented control is usually built in: a ghost member
    /// draws no border, so there is nothing for adjacent members to share.
    pub fn outline(self) -> Self {
        self.variant(ToggleVariant::Outline)
    }

    /// Sets the size every member uses.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Makes the whole group inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Joins the members' borders into one segmented control.
    ///
    /// Without this the members are spaced apart as independent toggles.
    pub fn segmented(mut self) -> Self {
        self.segmented = true;
        self
    }

    /// Sets the group's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the message to emit with the new checked state of every member.
    pub fn on_change(mut self, on_change: impl Fn(Vec<bool>) -> Message + 'a) -> Self {
        self.on_change = Some(std::rc::Rc::new(on_change));
        self
    }

    /// How many members the group has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the group has no members.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Converts the group into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            items,
            variant,
            size,
            disabled,
            segmented,
            width,
            on_change,
        } = self;

        let count = items.len();
        let states: Vec<bool> = items.iter().map(Toggle::is_checked).collect();

        let mut members: Vec<Element<'a, Message, Theme>> = Vec::with_capacity(count);

        for (index, item) in items.into_iter().enumerate() {
            let mut item = item.variant(variant).disabled(disabled);

            if let Some(size) = size {
                item = item.size(size);
            }

            // Only a segmented group joins its borders, and only the outer
            // edges of the run are rounded.
            if segmented {
                item = item.corners(member_corners(index, count, ButtonGroupLayout::Horizontal));
            }

            if let Some(on_change) = on_change.clone() {
                // Each member's message is the state that pressing *it*
                // produces, computed against the states the caller rendered —
                // so a press reports the whole group, not just its own flip.
                let next = flip(&states, index);
                item = item.on_toggle(move |_| on_change(next.clone()));
            }

            members.push(item.into_element());
        }

        let row = iced::widget::Row::with_children(members)
            // A segmented run has no gaps: the members' borders are meant to
            // touch, which is what makes them read as one control.
            .spacing(if segmented { 0.0 } else { 8.0 })
            .align_y(iced::Alignment::Center);

        let mut container = iced::widget::container(row);
        if let Some(width) = width {
            container = container.width(width);
        }

        container.into()
    }
}

impl<'a, Message: Clone + 'a> Default for ToggleGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<ToggleGroup<'a, Message>> for Element<'a, Message, Theme> {
    fn from(group: ToggleGroup<'a, Message>) -> Self {
        group.into_element()
    }
}

/// The whole group's state after pressing the member at `index`.
///
/// The result is computed from the states the caller rendered, so the members
/// do not have to be built in any particular order for the reported vector to
/// be right.
fn flip(states: &[bool], index: usize) -> Vec<bool> {
    let mut next = states.to_vec();
    if let Some(state) = next.get_mut(index) {
        *state = !*state;
    }
    next
}

#[cfg(test)]
mod tests {
    use super::{Toggle, ToggleGroup, ToggleVariant};
    use crate::theme::Size;
    use crate::Theme;
    use iced::Element;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Toggled(bool),
        Changed(Vec<bool>),
    }

    #[test]
    fn a_toggle_renders_in_both_states_and_variants() {
        for variant in [ToggleVariant::Ghost, ToggleVariant::Outline] {
            for checked in [true, false] {
                let element: Element<'_, Message, Theme> = Toggle::new("Bold")
                    .variant(variant)
                    .checked(checked)
                    .on_toggle(Message::Toggled)
                    .into();
                drop(element);
            }
        }
    }

    #[test]
    fn a_toggle_renders_with_an_icon_and_every_size() {
        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
            let element: Element<'_, Message, Theme> = Toggle::new("Bold")
                .icon("B")
                .size(size)
                .on_toggle(Message::Toggled)
                .into();
            drop(element);

            let icon_only: Element<'_, Message, Theme> =
                Toggle::<Message>::icon_only().icon("B").size(size).into();
            drop(icon_only);
        }
    }

    /// A checked toggle latches, so pressing it must report the negation of the
    /// state the caller passed in — not the state itself. A toggle that
    /// reported its current value would never switch off.
    #[test]
    fn pressing_a_toggle_reports_the_negation_of_its_state() {
        use std::cell::Cell;

        for state in [true, false] {
            let reported = Cell::new(None);
            let element: Element<'_, Message, Theme> = Toggle::new("Bold")
                .checked(state)
                .on_toggle(|next| {
                    reported.set(Some(next));
                    Message::Toggled(next)
                })
                .into();
            drop(element);

            assert_eq!(
                reported.get(),
                Some(!state),
                "a toggle checked({state}) must report {}",
                !state
            );
        }
    }

    #[test]
    fn a_disabled_toggle_renders_and_drops_its_handler() {
        let element: Element<'_, Message, Theme> = Toggle::new("Bold")
            .checked(true)
            .disabled(true)
            .on_toggle(Message::Toggled)
            .into();
        drop(element);
    }

    #[test]
    fn a_group_renders_segmented_and_spaced() {
        for segmented in [true, false] {
            let mut group = ToggleGroup::new()
                .push(Toggle::new("Bold").checked(true))
                .push(Toggle::new("Italic"))
                .push(Toggle::new("Underline"));

            if segmented {
                group = group.segmented();
            }

            let element: Element<'_, Message, Theme> = group.on_change(Message::Changed).into();
            drop(element);
        }
    }

    #[test]
    fn an_empty_group_renders() {
        assert!(ToggleGroup::<Message>::new().is_empty());

        let element: Element<'_, Message, Theme> = ToggleGroup::<Message>::new().into();
        drop(element);
    }

    /// Every member's reported state has to reflect the one press, with the
    /// rest passed through unchanged — a group that reported only its own flip
    /// would erase the caller's other choices.
    #[test]
    fn a_group_reports_the_full_state_after_a_press() {
        use std::cell::RefCell;

        let reported: RefCell<Vec<Vec<bool>>> = RefCell::new(Vec::new());
        let element: Element<'_, Message, Theme> = ToggleGroup::new()
            .push(Toggle::new("Bold").checked(true))
            .push(Toggle::new("Italic"))
            .push(Toggle::new("Underline").checked(true))
            .on_change(|next| {
                reported.borrow_mut().push(next.clone());
                Message::Changed(next)
            })
            .into();
        drop(element);

        // One entry per member, each showing only that member flipped and the
        // rest passed through untouched.
        let reported = reported.into_inner();
        assert_eq!(reported.len(), 3);
        assert_eq!(reported[0], vec![false, false, true]);
        assert_eq!(reported[1], vec![true, true, true]);
        assert_eq!(reported[2], vec![true, false, false]);
    }
}
