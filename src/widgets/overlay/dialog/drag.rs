//! The drag a dialog moves by.
//!
//! A dialog is centred by its layer and, once grabbed, by nothing: a drag
//! displaces the surface from wherever the centring put it, and the surface
//! keeps the displacement for as long as it exists. The state lives in the
//! widget's tree slot — the same arrangement the carousel's scroll and the
//! dock's drag session use — so a dialog opened again starts from the centre
//! rather than from wherever it was left, with nothing asked of the
//! application.
//!
//! # Why the offset moves the layout rather than the drawing
//!
//! A drawn translation (the way [`Enter`](super::Enter) slides a surface in)
//! has to re-answer every hit-test by hand: the cursor, the viewport, each
//! child's idea of where it is. Translating the layout node instead moves the
//! surface *and* its hit-testing together, so buttons in a displaced dialog
//! keep working with no compensation anywhere.

use std::marker::PhantomData;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::Operation;
use iced::advanced::{widget::Widget, Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::{Element, Event, Point, Rectangle, Size, Vector};


/// How far the pointer travels before a press becomes a drag, in logical
/// pixels. The same threshold a tab drag uses, so every grab in the library
/// asks for the same nudge before it picks something up.
const DRAG_THRESHOLD: f32 = 6.0;

/// How much of a dragged surface must stay on the glass, in logical pixels:
/// the sliver a corner is allowed to shrink to before the clamp holds it
/// back. A dialog dragged mostly off-screen is a dialog lost.
const MIN_VISIBLE: f32 = 48.0;

/// Holds a displacement to what keeps the surface on the glass.
///
/// The rule is a corner's width of the surface, on every side: a dialog
/// dragged mostly out of the window is a dialog lost, and one pulled until
/// only a sliver shows reads as gone. Where the surface starts is the layer's
/// decision — a modal centres it — so the allowance is measured from the
/// middle, which is where a dialog sits before it is grabbed.
fn clamp_offset(offset: Vector, size: Size, view: Size) -> Vector {
    let x = (view.width - size.width) / 2.0;
    let y = (view.height - size.height) / 2.0;

    Vector::new(
        offset
            .x
            .clamp(MIN_VISIBLE - size.width - x, view.width - MIN_VISIBLE - x),
        offset
            .y
            .clamp(MIN_VISIBLE - size.height - y, view.height - MIN_VISIBLE - y),
    )
}

/// Wraps a dialog surface so pressing it and moving carries it elsewhere.
///
/// The whole surface is the handle: presses an interactive child does not
/// claim — the title, the body's prose — pick the dialog up, while a press a
/// button claimed does not, because the child is offered the event first.
pub struct DragSurface<'a, Message, Theme = crate::theme::Theme, Renderer = iced::Renderer>
where
    Renderer: renderer::Renderer,
{
    surface: Element<'a, Message, Theme, Renderer>,
    _renderer: PhantomData<Renderer>,
}

/// The grab state of one [`DragSurface`].
#[derive(Debug, Default, Clone, Copy)]
struct DragState {
    /// The pointer went down on the surface, so a drag may start.
    pending: bool,
    /// Where the press landed, to measure the threshold against.
    start: Option<Point>,
    /// Whether the threshold was crossed, so a drag is live.
    dragging: bool,
    /// The offset the surface had when this drag began, so successive drags
    /// add up rather than restart.
    base: Vector,
    /// Where the surface sits now, from its centred position.
    offset: Vector,
}

impl<'a, Message, Theme, Renderer> DragSurface<'a, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    /// Wraps a dialog surface.
    pub fn new(surface: Element<'a, Message, Theme, Renderer>) -> Self {
        Self {
            surface,
            _renderer: PhantomData,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for DragSurface<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn tag(&self) -> Tag {
        Tag::of::<DragState>()
    }

    fn state(&self) -> State {
        State::new(DragState::default())
    }

    fn children(&self) -> Vec<Tree> {
        // The surface's own subtree is created with it: a dialog built from a
        // `Stack` answers to its children the first time it is laid out.
        vec![Tree::new(&self.surface)]
    }

    fn diff(&self, tree: &mut Tree) {
        // One child: the surface itself. `diff_children` diffs it in full —
        // a push of a fresh `Tree` would leave the surface's own subtree
        // empty, and a dialog built from a `Stack` answers to its children
        // the first time it is laid out.
        tree.diff_children(std::slice::from_ref(&self.surface));
    }

    fn size(&self) -> Size<iced::Length> {
        self.surface.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let mut surface = self
            .surface
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);

        let state = tree.state.downcast_mut::<DragState>();
        let size = surface.size();
        let view = limits.max();

        state.offset = clamp_offset(state.offset, size, view);

        // The offset lives *inside* this node: the layer that centres the
        // dialog positions this node after the fact, and a translation on
        // the node itself would be moved over by that.
        surface.translate_mut(state.offset);

        layout::Node::with_children(size, vec![surface])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let Some(surface_layout) = layout.children().next() else {
            return;
        };
        self.surface.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            surface_layout,
            cursor,
            viewport,
        );
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // The surface is displaced within this widget, so its own layout —
        // not this widget's — is where its bounds and its children live. It
        // is offered the event first, so a press a button claimed never
        // becomes a grab and a displaced dialog's controls keep working.
        let Some(surface_layout) = layout.children().next() else {
            return;
        };

        self.surface.as_widget_mut().update(
            &mut tree.children[0],
            event,
            surface_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<DragState>();
        let bounds = surface_layout.bounds();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. }) => {
                        if !shell.is_event_captured()
                    && cursor
                        .position()
                        .is_some_and(|point| bounds.contains(point))
                {
                    state.pending = true;
                    state.start = cursor.position();
                    state.base = state.offset;
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(iced::touch::Event::FingerMoved { .. }) => {
                // Measured from the press, so a click on the title stays a
                // click and only a real movement picks the dialog up.
                if state.pending && !state.dragging {
                    if let (Some(start), Some(position)) =
                        (state.start, cursor.position())
                    {
                        let dx = position.x - start.x;
                        let dy = position.y - start.y;

                        if (dx * dx + dy * dy).sqrt() >= DRAG_THRESHOLD {
                            state.dragging = true;
                            state.pending = false;
                        }
                    }
                }

                if state.dragging {
                    if let (Some(start), Some(position)) =
                        (state.start, cursor.position())
                    {
                        state.offset = state.base
                            + Vector::new(position.x - start.x, position.y - start.y);
                        // The layout moves with the drag, so the next frame
                        // both draws and hit-tests where the surface now is.
                        shell.invalidate_layout();
                        shell.capture_event();
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerLifted { .. })
                if state.pending || state.dragging =>
            {
                state.pending = false;
                state.start = None;
                state.dragging = false;
                // The offset stays: a dialog released where it was dragged
                // stays where it was dragged.
                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(surface_layout) = layout.children().next() else {
            return mouse::Interaction::None;
        };

        let child = self.surface.as_widget().mouse_interaction(
            &tree.children[0],
            surface_layout,
            cursor,
            viewport,
            renderer,
        );

        if child == mouse::Interaction::None
            && cursor.is_over(surface_layout.bounds())
        {
            mouse::Interaction::Grab
        } else {
            child
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let Some(surface_layout) = layout.children().next() else {
            return;
        };

        self.surface
            .as_widget_mut()
            .operate(&mut tree.children[0], surface_layout, renderer, operation);
    }
}

#[cfg(test)]
mod tests {
    use super::{clamp_offset, MIN_VISIBLE};
    use iced::{Vector, Size};

    /// The window the drag tests render in, and a dialog card in it.
    const VIEW: Size = Size { width: 1280.0, height: 1000.0 };
    const CARD: Size = Size { width: 448.0, height: 164.8 };

    /// Where the card starts when the layer centres it.
    fn centre() -> Vector {
        Vector::new((VIEW.width - CARD.width) / 2.0, (VIEW.height - CARD.height) / 2.0)
    }

    #[test]
    fn a_small_drag_is_left_alone() {
        let centre = centre();
        let moved = clamp_offset(Vector::new(120.0, 60.0), CARD, VIEW);

        assert_eq!(moved, Vector::new(120.0, 60.0));

        // And the card is where the drag put it, not at the origin.
        let position = centre + moved;
        assert!((position.x - (centre.x + 120.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn a_long_drag_stops_with_a_corner_still_showing() {
        let centre = centre();

        // Dragged a thousand pixels each way in a 1280x1000 window: the
        // clamp, not the pull, decides where it stops.
        let moved = clamp_offset(Vector::new(-1000.0, -1000.0), CARD, VIEW);

        let top_left = centre + moved;
        let bottom_right = top_left + Vector::new(CARD.width, CARD.height);

        assert!(
            (bottom_right.x - MIN_VISIBLE).abs() < 0.01,
            "the card's right edge should hold a corner from the left edge: \
             it sits at {}",
            bottom_right.x
        );
        assert!(
            (bottom_right.y - MIN_VISIBLE).abs() < 0.01,
            "the card's bottom edge should hold a corner from the top edge: \
             it sits at {}",
            bottom_right.y
        );

        // The other direction holds the opposite corner.
        let moved = clamp_offset(Vector::new(1000.0, 1000.0), CARD, VIEW);
        let top_left = centre + moved;

        assert!((top_left.x - (VIEW.width - MIN_VISIBLE)).abs() < 0.01);
        assert!((top_left.y - (VIEW.height - MIN_VISIBLE)).abs() < 0.01);
    }

    #[test]
    fn the_clamp_is_symmetric_about_the_centre() {
        let left = clamp_offset(Vector::new(-5000.0, 0.0), CARD, VIEW);
        let right = clamp_offset(Vector::new(5000.0, 0.0), CARD, VIEW);

        assert!(
            (left.x + right.x).abs() < 0.01,
            "the two limits should mirror each other: {} against {}",
            left.x,
            right.x
        );
    }

    #[test]
    fn a_fresh_offset_is_the_centred_position() {
        // The zero offset means "wherever the layer put me", which is what a
        // dialog is built with — and what a reopened one gets, because the
        // state lives in the tree slot the closing dialog took with it.
        let moved = clamp_offset(Vector::new(0.0, 0.0), CARD, VIEW);
        assert_eq!(moved, Vector::new(0.0, 0.0));
    }

    #[test]
    fn a_card_bigger_than_the_window_still_leaves_a_corner() {
        let wide = Size::new(2000.0, 2000.0);
        let moved = clamp_offset(Vector::new(-9000.0, -9000.0), wide, VIEW);

        // The card is wider than the window, so its right edge can only be
        // pulled to the clamp's own limit, never past it.
        let position = Vector::new((VIEW.width - wide.width) / 2.0, (VIEW.height - wide.height) / 2.0)
            + moved;
        let bottom_right = position + Vector::new(wide.width, wide.height);

        assert!(bottom_right.x >= MIN_VISIBLE - 0.01);
        assert!(bottom_right.y >= MIN_VISIBLE - 0.01);
    }
}
