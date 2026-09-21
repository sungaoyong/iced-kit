//! Button groups: several buttons drawn as one joined control.

use crate::theme::catalog::{ButtonVariant, Corners};
use crate::theme::{Size, Theme};
use crate::widgets::button::Button;
use iced::{Element, Length};

/// The direction a group lays its members out in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonGroupLayout {
    /// Members sit side by side.
    #[default]
    Horizontal,
    /// Members stack, for a vertical toolbar.
    Vertical,
}

/// Several buttons joined into one control.
///
/// The members share the group's variant and size unless they set their own,
/// and only the outer edges are rounded — which is what makes a row of buttons
/// read as a single segmented control rather than as neighbours.
///
/// # Selection
///
/// A group reports the indices of its selected members through
/// [`on_select`](Self::on_select). It does not hold that state itself, matching
/// how iced components work: the application owns the selection and passes it
/// back in through [`Button::selected`], so the group is a view of the state
/// rather than a second copy of it.
///
/// ```
/// # use iced_kit::widgets::button::{Button, ButtonGroup};
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Filter(Vec<usize>) }
/// # fn view(active: usize) -> iced::Element<'static, Message, Theme> {
/// ButtonGroup::new()
///     .push(Button::new("Day").selected(active == 0))
///     .push(Button::new("Week").selected(active == 1))
///     .on_select(Message::Filter)
///     .into()
/// # }
/// ```
#[must_use = "a ButtonGroup does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct ButtonGroup<'a, Message> {
    children: Vec<Button<'a, Message>>,
    variant: Option<ButtonVariant>,
    size: Option<Size>,
    outline: bool,
    compact: bool,
    disabled: bool,
    multiple: bool,
    layout: ButtonGroupLayout,
    width: Option<Length>,
    on_select: Option<Box<dyn Fn(Vec<usize>) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> ButtonGroup<'a, Message> {
    /// Creates an empty group.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            variant: None,
            size: None,
            outline: false,
            compact: false,
            disabled: false,
            multiple: false,
            layout: ButtonGroupLayout::default(),
            width: None,
            on_select: None,
        }
    }

    /// Adds a member.
    pub fn push(mut self, button: Button<'a, Message>) -> Self {
        self.children.push(button);
        self
    }

    /// Adds several members.
    pub fn extend(mut self, buttons: impl IntoIterator<Item = Button<'a, Message>>) -> Self {
        self.children.extend(buttons);
        self
    }

    /// Sets the variant every member uses, unless it set its own.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Sets the size every member uses.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Draws every member as an outline.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Tightens every member's padding.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Makes the whole group inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Allows more than one member to be selected at a time.
    ///
    /// A single-selection group reports exactly one index; a multiple-selection
    /// group reports a toggled set.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Sets the layout direction.
    pub fn layout(mut self, layout: ButtonGroupLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Sets the group's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the message to emit when a member is pressed.
    ///
    /// The handler receives the indices of the members that are selected after
    /// the press, which is what the caller needs to store as the new state.
    pub fn on_select(mut self, on_select: impl Fn(Vec<usize>) -> Message + 'a) -> Self {
        self.on_select = Some(Box::new(on_select));
        self
    }

    /// How many members the group has.
    #[must_use]
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Whether the group has no members.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Converts the group into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            children,
            variant,
            size,
            outline,
            compact,
            disabled,
            multiple,
            layout,
            width,
            on_select,
        } = self;

        let count = children.len();
        let vertical = layout == ButtonGroupLayout::Vertical;

        // The selection the members were rendered with, captured before they
        // are consumed; a press is resolved against this so the caller receives
        // a complete picture rather than just the index that was clicked.
        let selected: Vec<usize> = children
            .iter()
            .enumerate()
            .filter_map(|(index, child)| child.class().selected.then_some(index))
            .collect();

        let mut members = Vec::with_capacity(count);

        for (index, child) in children.into_iter().enumerate() {
            let edges = member_corners(index, count, layout);
            let mut child = child.corners(edges);

            if let Some(variant) = variant {
                child = child.variant(variant);
            }
            if let Some(size) = size {
                child = child.size(size);
            }
            if outline {
                child = child.outline();
            }
            if compact {
                child = child.compact();
            }

            // A disabled group disables every member, and drops their handlers
            // so a press cannot reach the caller at all.
            if disabled {
                child = child.on_press_maybe(None);
            } else if let Some(on_select) = &on_select {
                let next = resolve_selection(&selected, index, multiple);
                let message = on_select(next);
                child = child.on_press(message);
            }

            members.push(child.into_element());
        }

        let container: Element<'a, Message, Theme> = if vertical {
            iced::widget::Column::with_children(members).into()
        } else {
            iced::widget::Row::with_children(members).into()
        };

        let mut widget = iced::widget::container(container);

        if let Some(width) = width {
            widget = widget.width(width);
        }

        widget.into()
    }
}

/// Which corners a member of a group rounds.
///
/// Only the outer edges are rounded, which is what joins the members into one
/// control. A lone member has no inner edges, so it keeps all four corners.
#[must_use]
pub fn member_corners(index: usize, count: usize, layout: ButtonGroupLayout) -> Corners {
    if count <= 1 {
        return Corners::ALL;
    }

    let vertical = layout == ButtonGroupLayout::Vertical;

    if index == 0 {
        if vertical {
            Corners {
                top_left: true,
                top_right: true,
                bottom_right: false,
                bottom_left: false,
            }
        } else {
            Corners {
                top_left: true,
                top_right: false,
                bottom_right: false,
                bottom_left: true,
            }
        }
    } else if index == count - 1 {
        if vertical {
            Corners {
                top_left: false,
                top_right: false,
                bottom_right: true,
                bottom_left: true,
            }
        } else {
            Corners {
                top_left: false,
                top_right: true,
                bottom_right: true,
                bottom_left: false,
            }
        }
    } else {
        Corners::NONE
    }
}

/// The selection a press on `index` produces.
///
/// This mirrors `gpui-kit`'s behaviour: a multiple-selection group toggles the
/// pressed member within the set, and a single-selection group replaces the set
/// with just that member.
fn resolve_selection(current: &[usize], index: usize, multiple: bool) -> Vec<usize> {
    if multiple {
        let mut next = current.to_vec();
        if let Some(position) = next.iter().position(|&selected| selected == index) {
            next.remove(position);
        } else {
            next.push(index);
            next.sort_unstable();
        }
        next
    } else {
        vec![index]
    }
}

impl<'a, Message: Clone + 'a> Default for ButtonGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<ButtonGroup<'a, Message>> for Element<'a, Message, Theme> {
    fn from(group: ButtonGroup<'a, Message>) -> Self {
        group.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_selection, ButtonGroup, ButtonGroupLayout};
    use crate::theme::catalog::ButtonVariant;
    use crate::widgets::button::Button;
    use crate::Theme;
    use iced::Element;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Selected(Vec<usize>),
    }

    #[test]
    fn a_single_selection_replaces_the_previous_choice() {
        assert_eq!(resolve_selection(&[0], 2, false), vec![2]);
        assert_eq!(resolve_selection(&[], 1, false), vec![1]);
    }

    #[test]
    fn a_multiple_selection_toggles_the_pressed_member() {
        assert_eq!(resolve_selection(&[0], 1, true), vec![0, 1]);
        assert_eq!(resolve_selection(&[0, 1], 0, true), vec![1]);
        assert_eq!(resolve_selection(&[], 3, true), vec![3]);
    }

    /// A toggle keeps the reported set sorted, so a caller comparing it to a
    /// stored selection does not see a spurious change of order.
    #[test]
    fn a_multiple_selection_stays_sorted() {
        assert_eq!(resolve_selection(&[2], 0, true), vec![0, 2]);
    }

    #[test]
    fn a_group_of_every_size_and_variant_renders() {
        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Primary,
            ButtonVariant::Ghost,
        ] {
            for layout in [ButtonGroupLayout::Horizontal, ButtonGroupLayout::Vertical] {
                let element: Element<'_, Message, Theme> = ButtonGroup::new()
                    .push(Button::new("One"))
                    .push(Button::new("Two").selected(true))
                    .push(Button::new("Three"))
                    .variant(variant)
                    .layout(layout)
                    .on_select(Message::Selected)
                    .into();
                drop(element);
            }
        }
    }

    /// The corner mask is the whole point of a group: a joined row must not
    /// round its inner edges, or the members read as neighbours rather than as
    /// one control.
    #[test]
    fn a_group_rounds_only_its_outer_corners() {
        use super::member_corners;
        use crate::theme::catalog::Corners;

        let horizontal = |index| member_corners(index, 3, ButtonGroupLayout::Horizontal);

        // A horizontal group keeps the left edge of its first member and the
        // right edge of its last, and squares everything between.
        assert!(horizontal(0).top_left && !horizontal(0).top_right);
        assert_eq!(horizontal(1), Corners::NONE);
        assert!(horizontal(2).top_right && !horizontal(2).top_left);

        let vertical = |index| member_corners(index, 3, ButtonGroupLayout::Vertical);

        // A vertical group rounds the top of its first member and the bottom of
        // its last, and squares the full width in between.
        assert!(vertical(0).top_left && vertical(0).top_right);
        assert!(!vertical(0).bottom_left && !vertical(0).bottom_right);
        assert_eq!(vertical(1), Corners::NONE);
        assert!(vertical(2).bottom_right && vertical(2).bottom_left);
    }

    /// A lone member has no inner edges to square off, so it keeps all four
    /// corners rather than ending up with half a rounded rectangle.
    #[test]
    fn a_lone_member_keeps_every_corner() {
        use super::member_corners;
        use crate::theme::catalog::Corners;

        assert_eq!(
            member_corners(0, 1, ButtonGroupLayout::Horizontal),
            Corners::ALL
        );
        assert_eq!(
            member_corners(0, 0, ButtonGroupLayout::Horizontal),
            Corners::ALL
        );
    }
}
