//! A draggable width handle for fixed-width panels.
//!
//! This is the interaction the [`Resizable`](crate::widgets::Resizable) split
//! panes get from iced's `pane_grid`, extracted for a single seam: a thin grab
//! strip over a panel's content-side edge. The strip is invisible while idle —
//! the panel's own hairline or the layout's rule is the visible divider — and
//! lights up on hover and while dragging with the same colors the pane grid's
//! splitter uses.
//!
//! The handle is stateless in the crate's sense: the panel's width stays owned
//! by the caller, and every drag movement reports the new width as a message.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Shell};
// The wrapper is written against the concrete `iced::Renderer`, so the core
// renderer trait must be in scope for `fill_quad` to resolve.
use iced::advanced::Renderer as _;
use iced::{Color, Element, Event, Length, Rectangle, Size as IcedSize};

/// The width of the strip that catches a press, in logical pixels.
///
/// Mirrors the grab area the pane grid gives its splitters: a visible divider
/// is far too thin to aim at.
pub const GRAB_WIDTH: f32 = 6.0;

/// The width of the line drawn while the handle is hovered or dragged.
pub const LINE_WIDTH: f32 = 2.0;

/// The narrowest the rest of the row is allowed to become, in logical pixels.
///
/// This is the pane grid's `min_size` semantics for a two-pane seam: a drag
/// that would squeeze the content below this stops at
/// `available - MIN_CONTENT_AREA` instead.
pub const MIN_CONTENT_AREA: f32 = 240.0;

/// The width a panel dragged from `press_width` at `press_x` takes once the
/// cursor reaches `cursor_x`.
///
/// A panel anchored at the row's left edge grows rightwards, one anchored at
/// the right edge grows leftwards. The result is clamped to `[min, max]`, with
/// `max` raised to `min` so a degenerate range still yields `min` rather than
/// flipping the panel inside out.
#[must_use]
pub fn dragged_width(
    press_width: f32,
    press_x: f32,
    cursor_x: f32,
    anchored_left: bool,
    min: f32,
    max: f32,
) -> f32 {
    let delta = cursor_x - press_x;
    let width = if anchored_left {
        press_width + delta
    } else {
        press_width - delta
    };

    width.clamp(min, max.max(min))
}

/// The rectangle that catches a press for a panel of `bounds`.
#[must_use]
pub fn grab_rect(bounds: Rectangle, anchored_left: bool) -> Rectangle {
    if anchored_left {
        Rectangle {
            x: bounds.x + bounds.width - GRAB_WIDTH,
            width: GRAB_WIDTH,
            ..bounds
        }
    } else {
        Rectangle {
            width: GRAB_WIDTH,
            ..bounds
        }
    }
}

/// Wraps a fixed-width panel with a drag handle on its content-side edge.
pub(crate) struct ResizeEdge<'a, Message> {
    content: Element<'a, Message, Theme>,
    anchored_left: bool,
    min_width: f32,
    on_resize: Box<dyn Fn(f32) -> Message + 'a>,
}

/// The handle's state between frames.
#[derive(Debug, Default)]
struct EdgeState {
    /// The drag in progress, if any.
    drag: Option<DragState>,
    /// The row width the drag may grow into, captured at layout time.
    available: f32,
}

/// Where a drag started, and the width it started from.
#[derive(Debug, Clone, Copy)]
struct DragState {
    press_x: f32,
    press_width: f32,
    /// The last width reported to the application, so a clamp that pins the
    /// cursor does not publish the same width on every mouse move.
    published: f32,
}

impl<'a, Message: Clone + 'a> ResizeEdge<'a, Message> {
    pub(crate) fn new(
        content: impl Into<Element<'a, Message, Theme>>,
        anchored_left: bool,
        min_width: f32,
        on_resize: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            anchored_left,
            min_width: min_width.max(1.0),
            on_resize: Box::new(on_resize),
        }
    }

    fn edge_state(tree: &mut tree::Tree) -> &mut EdgeState {
        tree.state.downcast_mut::<EdgeState>()
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, iced::Renderer> for ResizeEdge<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<EdgeState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(EdgeState::default())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> IcedSize<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        Self::edge_state(tree).available = limits.max().width;

        // The panel decides its own width; the handle only rides on it.
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);

        layout::Node::with_children(content.size(), vec![content])
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let strip = grab_rect(bounds, self.anchored_left);
        let state = Self::edge_state(tree);

        // A press on the strip starts the drag and never reaches the panel
        // below, so a control at the panel's edge does not fire. The release
        // that ends the drag is swallowed for the same reason.
        let swallow = match event {
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                if let Some(position) = cursor.position_over(strip) {
                    state.drag = Some(DragState {
                        press_x: position.x,
                        press_width: bounds.width,
                        published: bounds.width,
                    });

                    true
                } else {
                    false
                }
            }
            Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                if let Some(drag) = state.drag.as_mut() {
                    let position = cursor.position().unwrap_or_default();
                    let max = (state.available - MIN_CONTENT_AREA).max(self.min_width);
                    let width = dragged_width(
                        drag.press_width,
                        drag.press_x,
                        position.x,
                        self.anchored_left,
                        self.min_width,
                        max,
                    );

                    if width != drag.published {
                        drag.published = width;
                        shell.publish((self.on_resize)(width));
                    }

                    true
                } else {
                    false
                }
            }
            Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                // The release that ends a drag is swallowed, so a control the
                // press never reached does not receive its release either.
                state.drag.take().is_some()
            }
            _ => false,
        };

        if swallow {
            return;
        }

        // The strip belongs to the handle: while the cursor is over it, the
        // panel below sees no pointer at all.
        let over_strip = cursor.position_over(strip).is_some();

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            if over_strip {
                mouse::Cursor::Unavailable
            } else {
                cursor
            },
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<EdgeState>();

        if state.drag.is_some()
            || cursor
                .position_over(grab_rect(layout.bounds(), self.anchored_left))
                .is_some()
        {
            return mouse::Interaction::ResizingHorizontally;
        }

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout.children().next().unwrap(),
            cursor,
            viewport,
        );

        let bounds = layout.bounds();
        let strip = grab_rect(bounds, self.anchored_left);
        let dragging = tree.state.downcast_ref::<EdgeState>().drag.is_some();
        let hovered = cursor.position_over(strip).is_some();

        if !dragging && !hovered {
            return;
        }

        // The same two-step coloring the pane grid's splitter uses: a muted
        // line while hovering, the accent while the drag is live.
        let color = if dragging {
            Tone::Primary.accent(theme)
        } else {
            theme.colors().ring
        };

        let line = Rectangle {
            x: if self.anchored_left {
                strip.x + strip.width - LINE_WIDTH
            } else {
                strip.x
            },
            width: LINE_WIDTH,
            ..strip
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: line,
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                shadow: iced::Shadow::default(),
                snap: true,
            },
            color,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next()?,
            renderer,
            viewport,
            translation,
        )
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let Some(inner) = layout.children().next() else {
            return;
        };

        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], inner, renderer, operation);
    }
}

impl<'a, Message: Clone + 'a> From<ResizeEdge<'a, Message>> for Element<'a, Message, Theme> {
    fn from(edge: ResizeEdge<'a, Message>) -> Self {
        Element::new(edge)
    }
}

#[cfg(test)]
mod tests {
    use super::{dragged_width, grab_rect, EdgeState, GRAB_WIDTH, MIN_CONTENT_AREA};
    use crate::theme::Theme;
    use iced::{Point, Rectangle};

    #[derive(Debug, Clone)]
    #[allow(dead_code, reason = "the payload documents what a drag reports")]
    enum Message {
        Resized(f32),
    }

    #[test]
    fn a_left_anchored_panel_grows_rightwards() {
        let width = dragged_width(220.0, 100.0, 140.0, true, 120.0, 480.0);

        assert_eq!(width, 260.0);
    }

    #[test]
    fn a_right_anchored_panel_grows_leftwards() {
        let width = dragged_width(220.0, 300.0, 260.0, false, 120.0, 480.0);

        assert_eq!(width, 260.0);
    }

    #[test]
    fn a_drag_is_clamped_to_the_floor() {
        let width = dragged_width(220.0, 100.0, -500.0, true, 120.0, 480.0);

        assert_eq!(width, 120.0);
    }

    #[test]
    fn a_drag_is_clamped_to_the_ceiling() {
        let width = dragged_width(220.0, 100.0, 5_000.0, true, 120.0, 480.0);

        assert_eq!(width, 480.0);
    }

    #[test]
    fn a_degenerate_range_yields_the_floor() {
        // A tiny window can push the ceiling below the floor; the panel must
        // still land on one width instead of oscillating.
        let width = dragged_width(220.0, 100.0, 0.0, true, 300.0, 100.0);

        assert_eq!(width, 300.0);
    }

    #[test]
    fn the_grab_strip_sits_on_the_content_side_edge() {
        let bounds = Rectangle::new(Point::new(16.0, 16.0), iced::Size::new(220.0, 400.0));

        let left = grab_rect(bounds, true);
        assert_eq!(left.x, 16.0 + 220.0 - GRAB_WIDTH);
        assert_eq!(left.width, GRAB_WIDTH);

        let right = grab_rect(bounds, false);
        assert_eq!(right.x, 16.0);
        assert_eq!(right.width, GRAB_WIDTH);
    }

    #[test]
    fn a_fresh_edge_state_is_not_dragging() {
        let state = EdgeState::default();

        assert!(state.drag.is_none());
        assert_eq!(state.available, 0.0);
    }

    #[test]
    fn a_resizable_edge_renders() {
        let panel: iced::Element<'_, Message, Theme> =
            iced::widget::container(iced::widget::text("Panel"))
                .width(iced::Length::Fixed(220.0))
                .into();

        let edge: iced::Element<'_, Message, Theme> =
            super::ResizeEdge::new(panel, true, 120.0, Message::Resized).into();

        drop(edge);
    }

    #[test]
    fn the_content_floor_leaves_room_for_the_page() {
        // A 640px row must not let the panel grow past 640 - 240.
        let available = 640.0;
        let max = (available - MIN_CONTENT_AREA).max(120.0);

        let width = dragged_width(220.0, 0.0, 10_000.0, true, 120.0, max);

        assert_eq!(width, available - MIN_CONTENT_AREA);
    }

    #[test]
    fn a_theme_supplies_both_handle_colors() {
        let theme = Theme::light();
        let hover = theme.colors().ring;
        let drag = super::Tone::Primary.accent(&theme);

        assert_ne!(hover, drag, "hover and drag must be distinguishable");
    }
}
