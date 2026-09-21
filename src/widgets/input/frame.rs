//! The frame a form control is drawn inside.
//!
//! A field's border is not the control's own: the value, a prefix, a suffix, a
//! clear button and a spinner all share one box, and the box is what draws the
//! border and the focus ring. iced's `text_input` can only draw a border around
//! the text region it owns, which is why `gpui-kit` has an `InputGroup` and this
//! crate has a frame.
//!
//! # Reading focus
//!
//! A widget cannot ask iced "is my child focused?" through the public API, so
//! the frame reads the control's own state out of the widget tree — the same
//! downcast `iced_widget`'s own `combo_box` performs to decide whether to show
//! its selection. The frame therefore remembers the control's child index and
//! which kind of control it is.
//!
//! The layout is delegated to a [`Row`](iced::widget::row) rather than
//! re-implemented: laying out fill-width children, aligning them and routing
//! keyboard traversal is iced's job, and a hand-rolled flex container would only
//! get it subtly wrong.

use crate::theme::catalog::{FieldAppearance, FieldState};
use crate::theme::{Size, Theme};
use iced::advanced::text::highlighter::PlainText;
use iced::advanced::widget::tree;
use iced::advanced::{layout, mouse, overlay, renderer, text, Clipboard, Layout, Shell, Widget};
use iced::widget::{column, row, text_editor, text_input};
use iced::{Border, Element, Event, Length, Padding, Rectangle, Vector};

/// Which control the frame wraps, so it knows where to read focus from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlKind {
    /// A single-line [`text_input`].
    Single,
    /// A multi-line [`text_editor`].
    Multi,
}

/// The height a frame asks for.
///
/// A single-line field is exactly as tall as its size step, so its value is
/// centred against the border. A field carrying block addons has to grow to fit
/// them, and a multi-line editor brings a height of its own, so both take the
/// height their content needs instead.
fn frame_height(kind: ControlKind, has_block: bool, size: Size) -> Length {
    match (kind, has_block) {
        (ControlKind::Single, false) => Length::Fixed(size.height()),
        _ => Length::Shrink,
    }
}

/// How a frame aligns its inline slots against the control.
///
/// A single line of text is centred against the border, so its addons are
/// centred too. A multi-line area has several lines, and centring an addon
/// against the block would leave it floating halfway down beside no particular
/// line, so it sits at the top instead.
fn cross_alignment(kind: ControlKind) -> iced::Alignment {
    match kind {
        ControlKind::Single => iced::Alignment::Center,
        ControlKind::Multi => iced::Alignment::Start,
    }
}

/// The child indexes leading from a frame to its control.
///
/// A plain field's control is a direct child of the row. With a block addon the
/// body becomes a column whose middle child is that row, so the path grows a
/// leading index.
fn control_path(control_index: usize, has_block: bool) -> Vec<usize> {
    if has_block {
        vec![1, control_index]
    } else {
        vec![control_index]
    }
}

/// The frame around a form control.
pub(crate) struct FieldFrame<'a, Message, Renderer = iced::Renderer> {
    size: Size,
    /// What the caller knows about the field's state. Focus is not among it:
    /// only the control knows that, and the frame reads it from the tree.
    invalid: bool,
    disabled: bool,
    kind: ControlKind,
    /// Slots before the control. A field's prefix does not appear and disappear
    /// between rebuilds, so this list is fixed for the lifetime of one frame and
    /// the control's index is therefore stable.
    leading: Vec<Element<'a, Message, Theme, Renderer>>,
    control: Element<'a, Message, Theme, Renderer>,
    /// Slots after the control. These may come and go — a clear button appears
    /// when the field is filled — and appending or dropping the last children
    /// never shifts the control's index, so they need no stability.
    trailing: Vec<Element<'a, Message, Theme, Renderer>>,
    /// Addons stacked above the control's row, which turn the frame into a
    /// column. A group uses these for a label or a unit spanning the width.
    block_leading: Vec<Element<'a, Message, Theme, Renderer>>,
    /// Addons stacked below the control's row.
    block_trailing: Vec<Element<'a, Message, Theme, Renderer>>,
    padding: Padding,
    width: Length,
    radius: f32,
}

impl<'a, Message: 'a, Renderer: text::Renderer + 'a> FieldFrame<'a, Message, Renderer> {
    /// Wraps `control` in a frame of the given size and radius.
    pub(crate) fn new(
        kind: ControlKind,
        control: Element<'a, Message, Theme, Renderer>,
        size: Size,
        radius: f32,
    ) -> Self {
        Self {
            size,
            invalid: false,
            disabled: false,
            kind,
            leading: Vec::new(),
            control,
            trailing: Vec::new(),
            block_leading: Vec::new(),
            block_trailing: Vec::new(),
            padding: Padding {
                top: 0.0,
                right: size.input_padding(),
                bottom: 0.0,
                left: size.input_padding(),
            },
            width: Length::Fill,
            radius,
        }
    }

    /// Sets what the caller knows about the field's state.
    pub(crate) fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Sets whether the field is inert.
    pub(crate) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Appends a slot before the control.
    pub(crate) fn leading(
        mut self,
        slot: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.leading.push(slot.into());
        self
    }

    /// Appends a slot after the control.
    pub(crate) fn trailing(
        mut self,
        slot: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.trailing.push(slot.into());
        self
    }

    /// Appends an addon above the control's row.
    pub(crate) fn block_leading(
        mut self,
        slot: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.block_leading.push(slot.into());
        self
    }

    /// Appends an addon below the control's row.
    pub(crate) fn block_trailing(
        mut self,
        slot: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        self.block_trailing.push(slot.into());
        self
    }

    /// Overrides the frame's inner padding.
    pub(crate) fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Overrides the frame's width.
    pub(crate) fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Consumes the builder into the widget that draws it.
    pub(crate) fn into_framed(self) -> Framed<'a, Message, Renderer> {
        let Self {
            size,
            invalid,
            disabled,
            kind,
            leading,
            control,
            trailing,
            block_leading,
            block_trailing,
            padding,
            width,
            radius,
        } = self;

        let gap = match size {
            Size::Xs => 4.0,
            _ => 6.0,
        };

        let cross_alignment = cross_alignment(kind);

        // The control sits after the leading slots. Since a prefix is fixed for
        // the life of a frame, this index is stable across rebuilds, which is
        // what lets the frame find the control's state again.
        let control_index = leading.len();

        let mut inline: Vec<Element<'a, Message, Theme, Renderer>> = leading;
        inline.push(control);
        inline.extend(trailing);

        let has_block = !block_leading.is_empty() || !block_trailing.is_empty();

        // Without block addons the body is the row itself. With them it becomes
        // a column holding the addons around that row.
        let body: Element<'a, Message, Theme, Renderer> = if has_block {
            // The frame's padding lands on the column, so the block addons are
            // inset like any other content. The row inside it would then start
            // at the column's own origin and the value would sit flush against
            // the border, so it takes the horizontal inset itself and the column
            // carries only the vertical one.
            let mut inner = row(inline).spacing(gap).align_y(cross_alignment);

            if padding.left > 0.0 || padding.right > 0.0 {
                inner = inner.padding(Padding {
                    top: 0.0,
                    right: padding.right,
                    bottom: 0.0,
                    left: padding.left,
                });
            }

            let column_padding = Padding {
                top: padding.top,
                right: 0.0,
                bottom: padding.bottom,
                left: 0.0,
            };

            let mut children: Vec<Element<'a, Message, Theme, Renderer>> = block_leading;
            children.push(inner.into());
            children.extend(block_trailing);

            column(children).spacing(gap).padding(column_padding).into()
        } else {
            row(inline).spacing(gap).align_y(cross_alignment).into()
        };

        let control_path = control_path(control_index, has_block);

        let height = frame_height(self.kind, has_block, self.size);

        Framed {
            body,
            invalid,
            disabled,
            kind,
            control_path,
            padding,
            padded_body: has_block,
            width,
            height,
            radius,
        }
    }
}

/// The widget a [`FieldFrame`] becomes: a row with the frame painted behind it.
///
/// Kept separate from [`FieldFrame`] so the builder can be consumed into the
/// widget without the widget holding builder state it no longer needs.
///
/// The frame resolves its colors in `draw` rather than at build time: the theme
/// is only known then, so resolving earlier would bake in `Theme::default()` and
/// paint every dark-mode field with the light palette.
pub(crate) struct Framed<'a, Message, Renderer = iced::Renderer> {
    body: Element<'a, Message, Theme, Renderer>,
    invalid: bool,
    disabled: bool,
    kind: ControlKind,
    /// The child indexes leading from the frame to the control, so the frame can
    /// find its state. A plain field's control is a direct child; a group's sits
    /// inside a row inside a column.
    control_path: Vec<usize>,
    padding: Padding,
    /// Whether the body is a column that carries its own padding.
    padded_body: bool,
    width: Length,
    height: Length,
    radius: f32,
}

impl<'a, Message: 'a, Renderer: text::Renderer> Framed<'a, Message, Renderer> {
    /// Reads whether the control currently holds focus.
    ///
    /// A widget cannot ask iced about a child's focus through the public API, so
    /// the control's state is read out of the tree — the same downcast
    /// `iced_widget`'s `combo_box` uses. A missing child, or one whose state is
    /// not the expected type, reads as unfocused rather than panicking: the frame
    /// is decoration, and it must not be able to take the window down.
    fn control_is_focused(&self, tree: &tree::Tree) -> bool {
        let mut node = tree;

        for index in &self.control_path {
            let Some(child) = node.children.get(*index) else {
                return false;
            };

            node = child;
        }

        // The tag is checked before the downcast because `downcast_ref` panics
        // on a stateless node, and a widget tree is rebuilt from application
        // state: a caller can swap the control for something else between two
        // frames, and the frame must not be able to take the window down when
        // it does.
        match self.kind {
            ControlKind::Single => {
                let expected = tree::Tag::of::<text_input::State<Renderer::Paragraph>>();

                if node.tag != expected {
                    return false;
                }

                node.state
                    .downcast_ref::<text_input::State<Renderer::Paragraph>>()
                    .is_focused()
            }
            ControlKind::Multi => {
                let expected = tree::Tag::of::<text_editor::State<PlainText>>();

                if node.tag != expected {
                    return false;
                }

                node.state
                    .downcast_ref::<text_editor::State<PlainText>>()
                    .is_focused()
            }
        }
    }

    /// The state the frame is drawn in, with focus and hover folded in.
    fn resolved_state(&self, focused: bool, hovered: bool) -> FieldState {
        FieldState {
            focused,
            hovered,
            invalid: self.invalid,
            disabled: self.disabled,
        }
    }
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for Framed<'_, Message, Renderer>
where
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        self.body.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.body.as_widget().state()
    }

    fn children(&self) -> Vec<tree::Tree> {
        self.body.as_widget().children()
    }

    fn diff(&self, tree: &mut tree::Tree) {
        self.body.as_widget().diff(tree);
    }

    fn size(&self) -> iced::Size<Length> {
        iced::Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // The body is laid out against a loose height and then centred in the
        // field. Constraining it to the exact height instead would stretch the
        // control to fill the box while its text stayed pinned to the top of its
        // own line box, which reads as a value sitting low in the field.
        //
        // A body that carries its own padding — the column a block addon builds
        // — is laid out at the full width and only centred vertically; a plain
        // row is inset here, so its limits have to be shrunk first or the
        // content would be laid out full width and then shifted off the edge.
        let outer = limits.width(self.width).height(self.height);

        if self.padded_body {
            let content =
                self.body
                    .as_widget_mut()
                    .layout(tree, renderer, &limits.width(self.width).loose());
            let size = outer.resolve(self.width, self.height, content.size());
            let content = content.align(iced::Alignment::Center, iced::Alignment::Center, size);

            return layout::Node::with_children(size, vec![content]);
        }

        let content = self.body.as_widget_mut().layout(
            tree,
            renderer,
            &limits.width(self.width).loose().shrink(self.padding),
        );

        let padding = self.padding.fit(content.size(), outer.max());
        let size = outer
            .shrink(padding)
            .resolve(self.width, self.height, content.size());

        let content = content
            .align(iced::Alignment::Center, iced::Alignment::Center, size)
            .move_to((padding.left, padding.top));

        layout::Node::with_children(size.expand(padding), vec![content])
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let Some(inner) = layout.children().next() else {
            return;
        };

        self.body
            .as_widget_mut()
            .operate(tree, inner, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(inner) = layout.children().next() else {
            return;
        };

        self.body.as_widget_mut().update(
            tree, event, inner, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(inner) = layout.children().next() else {
            return mouse::Interaction::default();
        };

        self.body
            .as_widget()
            .mouse_interaction(tree, inner, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !bounds.intersects(viewport) {
            return;
        }

        let state = self.resolved_state(self.control_is_focused(tree), cursor.is_over(bounds));
        let appearance = FieldAppearance::resolve(theme, state);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    color: appearance.border,
                    width: appearance.border_width,
                    radius: self.radius.into(),
                },
                // A field whose height is not a whole number of pixels would
                // otherwise blur its border, the same reason a button snaps.
                snap: true,
                ..renderer::Quad::default()
            },
            appearance.background,
        );

        let Some(inner) = layout.children().next() else {
            return;
        };

        // The frame owns the text color, because it is the frame that decides
        // whether the field is inert; the control inherits what it is told.
        self.body.as_widget().draw(
            tree,
            renderer,
            theme,
            &renderer::Style {
                text_color: appearance.text_color,
            },
            inner,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let inner = layout.children().next()?;

        self.body
            .as_widget_mut()
            .overlay(tree, inner, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a, Renderer> From<FieldFrame<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(frame: FieldFrame<'a, Message, Renderer>) -> Self {
        Element::new(frame.into_framed())
    }
}

#[cfg(test)]
mod tests {
    use super::{control_path, cross_alignment, frame_height, ControlKind};
    use crate::theme::catalog::{FieldAppearance, FieldState};
    use crate::theme::{Size, Theme};
    use iced::Length;

    #[test]
    fn a_single_line_field_is_exactly_one_size_step_tall() {
        // The value's line box and the frame's height have to agree, or the text
        // sits off-centre: too short and it is clipped, too tall and it floats.
        // This was a real defect: the frame let the control set the height while
        // the line box stayed at the text's own height.
        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
            assert_eq!(
                frame_height(ControlKind::Single, false, size),
                Length::Fixed(size.height()),
                "{size:?} should be exactly {:?} tall",
                size.height()
            );
        }
    }

    #[test]
    fn a_field_carrying_block_addons_is_not_pinned_to_one_size_step() {
        // Block addons stack above and below the value, so the frame has to grow
        // to fit them; pinning it would overlap them, which is what it did
        // before this was split out.
        assert_eq!(
            frame_height(ControlKind::Single, true, Size::Md),
            Length::Shrink
        );
    }

    #[test]
    fn a_multi_line_field_is_sized_by_its_content() {
        // A text area carries a height of its own, so the frame must keep out of
        // the way; forcing the size step would crop it to one line.
        assert_eq!(
            frame_height(ControlKind::Multi, false, Size::Md),
            Length::Shrink
        );
        assert_eq!(
            frame_height(ControlKind::Multi, true, Size::Md),
            Length::Shrink
        );
    }

    #[test]
    fn a_multi_line_field_aligns_its_addons_to_the_top() {
        // Centring an addon against a block of text leaves it floating beside no
        // particular line, which is what it looked like before this was split.
        assert_eq!(
            cross_alignment(ControlKind::Single),
            iced::Alignment::Center
        );
        assert_eq!(cross_alignment(ControlKind::Multi), iced::Alignment::Start);
    }

    #[test]
    fn the_control_keeps_its_index_however_many_trailing_slots_appear() {
        // The frame finds the control's focus state by index, so a trailing slot
        // appearing — a clear button, a spinner — must not shift it.
        for prefix_count in 0..4 {
            assert_eq!(
                control_path(prefix_count, false),
                vec![prefix_count],
                "a leading slot shifting the control is expected; trailing ones are not"
            );
        }

        assert_eq!(control_path(0, false), vec![0]);
        assert_eq!(control_path(0, true), vec![1, 0]);
        assert_eq!(control_path(2, true), vec![1, 2]);
    }

    #[test]
    fn an_invalid_field_keeps_its_error_border_while_focused() {
        // Focus is not new information when something is already wrong, and
        // replacing the error with a focus ring would hide the one signal the
        // user has to act on.
        let theme = Theme::light();

        let invalid_focused = FieldAppearance::resolve(
            &theme,
            FieldState {
                focused: true,
                hovered: false,
                invalid: true,
                disabled: false,
            },
        );
        let valid_focused = FieldAppearance::resolve(
            &theme,
            FieldState {
                focused: true,
                hovered: false,
                invalid: false,
                disabled: false,
            },
        );

        assert_eq!(invalid_focused.border, theme.colors().destructive);
        assert_eq!(valid_focused.border, theme.colors().ring);
        assert_ne!(invalid_focused.border, valid_focused.border);
    }

    #[test]
    fn an_invalid_field_keeps_its_error_border_even_while_disabled() {
        // Validation stays visible when editing is switched off; the caller's
        // result does not become less true because the field went inert.
        let theme = Theme::light();

        let disabled_invalid = FieldAppearance::resolve(
            &theme,
            FieldState {
                focused: false,
                hovered: false,
                invalid: true,
                disabled: true,
            },
        );

        let expected = theme.colors().destructive;

        assert_eq!(
            disabled_invalid.border.r, expected.r,
            "the border keeps the error hue, only dimmed"
        );
        assert_eq!(disabled_invalid.border.g, expected.g);
        assert_eq!(disabled_invalid.border.b, expected.b);
        assert!(disabled_invalid.border.a < expected.a, "but dimmed");
    }

    #[test]
    fn a_disabled_field_keeps_a_surface_and_dims_its_border() {
        let theme = Theme::light();

        let enabled = FieldAppearance::resolve(&theme, FieldState::default());
        let disabled = FieldAppearance::resolve(
            &theme,
            FieldState {
                disabled: true,
                ..FieldState::default()
            },
        );

        assert!(
            disabled.border.a < enabled.border.a,
            "a disabled border must be dimmed"
        );
        assert_eq!(disabled.text_color, theme.colors().muted_foreground);
        assert!(
            disabled.background.a > 0.0,
            "an empty field still has to read as a place a value could live"
        );
    }

    #[test]
    fn a_hovered_field_darkens_its_border_without_a_ring() {
        // Hover is a weaker signal than focus, so it changes the border's shade
        // rather than its weight; a hover that drew a ring would claim focus the
        // field does not have.
        let theme = Theme::light();

        let idle = FieldAppearance::resolve(&theme, FieldState::default());
        let hovered = FieldAppearance::resolve(
            &theme,
            FieldState {
                hovered: true,
                ..FieldState::default()
            },
        );

        assert_ne!(hovered.border, idle.border);
        assert_eq!(hovered.border_width, idle.border_width);
    }

    #[test]
    fn a_disabled_field_ignores_hover() {
        // A control that cannot be interacted with must not react to the
        // pointer, or it looks live.
        let theme = Theme::light();

        let disabled = FieldAppearance::resolve(
            &theme,
            FieldState {
                disabled: true,
                ..FieldState::default()
            },
        );
        let disabled_hovered = FieldAppearance::resolve(
            &theme,
            FieldState {
                disabled: true,
                hovered: true,
                ..FieldState::default()
            },
        );

        assert_eq!(disabled.border, disabled_hovered.border);
        assert_eq!(disabled.background, disabled_hovered.background);
    }

    #[test]
    fn a_focused_field_draws_a_heavier_ring_than_an_idle_one() {
        // The ring color is a mid-grey in both palettes, so the border's weight
        // is what actually carries the signal.
        let theme = Theme::light();

        let idle = FieldAppearance::resolve(&theme, FieldState::default());
        let focused = FieldAppearance::resolve(
            &theme,
            FieldState {
                focused: true,
                ..FieldState::default()
            },
        );

        assert!(focused.border_width > idle.border_width);
    }

    #[test]
    fn a_field_sits_on_a_different_surface_in_dark_mode() {
        // A field has to read as "type here" rather than as "press here", so in
        // dark mode it is lifted off the page and in light mode it is the page.
        let light = FieldAppearance::resolve(&Theme::light(), FieldState::default());
        let dark = FieldAppearance::resolve(&Theme::dark(), FieldState::default());

        assert_eq!(light.background, Theme::light().colors().background);
        assert_ne!(dark.background, Theme::dark().colors().background);
    }

    #[test]
    fn the_control_draws_no_border_of_its_own() {
        // The frame draws the border; a second one would double the line and
        // ignore the frame's radius.
        let appearance = FieldAppearance::resolve(&Theme::light(), FieldState::default());

        assert_eq!(appearance.into_text_input_style().border.width, 0.0);
        assert_eq!(appearance.into_text_editor_style().border.width, 0.0);
    }
}
