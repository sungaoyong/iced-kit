// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! A title bar's title, as a drag source.
//!
//! A group holding one panel draws a *title* rather than a strip of tabs. For
//! dragging that must not matter: a workspace where most groups hold a single panel
//! is the normal case, so a title that cannot be dragged leaves almost every panel
//! stuck where it is. In the `dock` example it left four groups out of five unable to
//! move, which reads as "dragging does not work at all".
//!
//! The threshold, the events and the session are the ones a tab drag uses, so a drop
//! cannot tell which of the two picked the panel up. That is deliberate — the title is
//! another way to pick a panel up, not another gesture — and it is why this widget
//! publishes through [`TabAction`] rather than inventing its own action.

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::svg as adv_svg;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::dock::model::NodeId;
use crate::dock::style::Catalog;
use crate::dock::widget::action::{DockAction, TabAction};
use crate::dock::widget::compose;

#[derive(Debug, Default)]
struct TitleDragState {
    /// The pointer went down on the title, so a drag may start.
    pending: bool,
    /// Where the press landed, to measure the threshold against.
    start: Option<iced::Point>,
    /// Whether the threshold was crossed, so a drag is live.
    dragging: bool,
}

/// Wraps an element so pressing it and moving picks the panel up.
///
/// `draggable` is `false` for the only panel of the only group, where a drag would
/// leave nothing on screen and no way to put it back. A widget that cannot be dragged
/// still draws and still passes events through, so it behaves like a plain element.
pub struct TitleDrag<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
{
    pane: NodeId,
    panel: NodeId,
    draggable: bool,
    /// The element dragged: the panel's title.
    title: Element<'a, Message, Theme, Renderer>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
    /// Movement, in logical pixels, before a press becomes a drag.
    threshold: f32,
    /// Fraction of a pane's edge that counts as a drop band, forwarded so the drop
    /// the session resolves uses the same geometry a tab drag would.
    drop_edge_fraction: f32,
}

impl<'a, Message, Theme, Renderer> TitleDrag<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer + adv_svg::Renderer + iced::advanced::text::Renderer + 'static,
{
    pub(crate) fn new(
        pane: NodeId,
        panel: NodeId,
        draggable: bool,
        title: Element<'a, Message, Theme, Renderer>,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        threshold: f32,
        drop_edge_fraction: f32,
    ) -> Self {
        Self {
            pane,
            panel,
            draggable,
            title,
            on_event,
            threshold,
            drop_edge_fraction,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TitleDrag<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer + adv_svg::Renderer + iced::advanced::text::Renderer + 'static,
{
    fn tag(&self) -> Tag {
        Tag::of::<TitleDragState>()
    }

    fn state(&self) -> State {
        State::new(TitleDragState::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.title)]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.is_empty() {
            tree.children.push(Tree::new(&self.title));
            return;
        }
        tree.children[0].diff(&self.title);
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let node = compose::child_layout(&mut self.title, &mut tree.children[0], renderer, limits);
        let size = node.size();
        layout::Node::with_children(size, vec![node])
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
        if let Some(title_layout) = layout.children().next() {
            compose::child_draw(
                &self.title,
                &tree.children[0],
                renderer,
                theme,
                style,
                title_layout,
                cursor,
                viewport,
            );
        }
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
        // The title is a real element, so it gets the event first: an application that
        // puts a button in its title keeps a working button.
        if let Some(title_layout) = layout.children().next() {
            compose::child_update(
                &mut self.title,
                &mut tree.children[0],
                event,
                title_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        if !self.draggable {
            return;
        }

        let state = tree.state.downcast_mut::<TitleDragState>();
        let threshold = self.threshold;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. }) => {
                if let Some(position) = cursor.position_over(layout.bounds()) {
                    state.pending = true;
                    state.start = Some(position);
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // Measured from the press, so a click on the title stays a click and
                // only a real movement picks the panel up.
                if state.pending && !state.dragging {
                    if let (Some(start), Some(position)) = (state.start, cursor.position()) {
                        let dx = position.x - start.x;
                        let dy = position.y - start.y;
                        if (dx * dx + dy * dy).sqrt() >= threshold {
                            state.dragging = true;
                            state.pending = false;
                            shell.publish((self.on_event)(DockAction::Tab(
                                TabAction::DragStarted {
                                    source_pane: self.pane,
                                    source_panel: self.panel,
                                    drop_edge_fraction: self.drop_edge_fraction,
                                },
                            )));
                            shell.capture_event();
                            shell.request_redraw();
                        }
                    }
                }
                if state.dragging {
                    if let Some(position) = cursor.position() {
                        shell.publish((self.on_event)(DockAction::Tab(TabAction::DragMoved {
                            cursor: position,
                        })));
                        // Claimed the way a tab drag claims it, so the panes under the
                        // pointer do not read the same movement as a hover.
                        shell.capture_event();
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerLifted { .. }) => {
                state.pending = false;
                state.start = None;
                if state.dragging {
                    state.dragging = false;
                    match cursor.position() {
                        Some(position) => {
                            shell.publish((self.on_event)(DockAction::Tab(TabAction::DragEnded {
                                cursor: position,
                            })));
                        }
                        // The pointer left the window mid-drag, so there is nowhere
                        // for the panel to land.
                        None => {
                            shell.publish(
                                (self.on_event)(DockAction::Tab(TabAction::DragCancelled)),
                            );
                        }
                    }
                    // The layout changed under the session, so the tree has to be
                    // rebuilt before the next frame draws it.
                    shell.invalidate_layout();
                    shell.invalidate_widgets();
                    shell.capture_event();
                    shell.request_redraw();
                }
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
        if self.draggable && cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Grab;
        }
        if let Some(title_layout) = layout.children().next() {
            return compose::child_mouse_interaction(
                &self.title,
                &tree.children[0],
                title_layout,
                cursor,
                viewport,
                renderer,
            );
        }
        mouse::Interaction::None
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(title_layout) = layout.children().next() {
            compose::child_operate(
                &mut self.title,
                &mut tree.children[0],
                title_layout,
                renderer,
                operation,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let title_layout = layout.children().next()?;
        let title_tree = tree.children.first_mut()?;
        self.title
            .as_widget_mut()
            .overlay(title_tree, title_layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<TitleDrag<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer + adv_svg::Renderer + iced::advanced::text::Renderer + 'static,
{
    fn from(drag: TitleDrag<'a, Message, Theme, Renderer>) -> Self {
        Element::new(drag)
    }
}
