// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! The area that arranges the centre and the edge docks, and the handles between
//! them.
//!
//! This is where a dock stops being a tree and starts being a window layout:
//! `[left dock] [centre column: centre, bottom dock] [right dock]`, with a resize
//! handle on the inner edge of each dock that has one.
//!
//! It owns two things the region model cannot:
//!
//! * **Placement.** How much of the area each dock takes, which is what makes a
//!   dock a column beside the centre rather than a block below it.
//! * **The drag on a handle.** Which is a gesture, so it belongs with the other
//!   gestures — and it is here rather than in a handle widget because the pointer
//!   leaves the handle immediately, so the move stream has to be read at the area.
//!
//! Children are positional and fixed: `[centre, left, bottom, right]`, always
//! four, with a dock that does not exist contributing a zero-sized spacer. That
//! matters because the child list is what iced matches tree state against: a
//! conditional list would shift every later child's state by one as a dock came
//! and went, which is exactly the bug a fixed list cannot have.

use std::cell::RefCell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::widget::overlay::menu;
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::dock::model::{Axis, DockPlacement};
use crate::dock::style::{Catalog, DockStyle};
use crate::dock::widget::action::DockAction;
use crate::dock::widget::compose;
use crate::dock::widget::state::DockWidgetState;

/// Index of a placement in the area's fixed child list.
const SLOT_CENTER: usize = 0;
const SLOT_LEFT: usize = 1;
const SLOT_BOTTOM: usize = 2;
const SLOT_RIGHT: usize = 3;

/// Index of the zoom slot, after the four regions.
const ZOOM_SLOT: usize = SLOT_RIGHT + 1;

fn placement_of(slot: usize) -> Option<DockPlacement> {
    match slot {
        SLOT_LEFT => Some(DockPlacement::Left),
        SLOT_BOTTOM => Some(DockPlacement::Bottom),
        SLOT_RIGHT => Some(DockPlacement::Right),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct DockAreaState {
    /// The grab area of each dock's handle, in area-local coordinates.
    handle_bounds: [Option<Rectangle>; 4],
    /// The dock whose handle the pointer is over.
    hovered: Option<DockPlacement>,
}

pub struct DockArea<'a, K, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
{
    dock_state: Rc<RefCell<DockWidgetState<K>>>,
    /// `[centre, left, bottom, right]`, always four.
    children: [Element<'a, Message, Theme, Renderer>; 4],
    /// The zoomed pane, drawn alone when a panel is maximized.
    zoomed: Option<Element<'a, Message, Theme, Renderer>>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    theme: Rc<RefCell<Option<Theme>>>,
    /// Width of a dock's resize grab area.
    handle_width: f32,
    /// Holds the zoom slot's tree state while nothing is zoomed.
    ///
    /// A real element rather than a skipped child, so the child list keeps its
    /// length: iced matches tree state against the child count, and a list that
    /// changed length would re-key everything below it.
    zoom_placeholder: Element<'a, Message, Theme, Renderer>,
}

impl<'a, K, Message, Theme, Renderer> DockArea<'a, K, Message, Theme, Renderer>
where
    K: 'static,
    Message: Clone + 'static,
    Theme: Catalog + menu::Catalog + Clone + PartialEq + 'static,
    Renderer: iced::advanced::Renderer + 'static,
{
    pub(crate) fn new(
        dock_state: Rc<RefCell<DockWidgetState<K>>>,
        children: [Element<'a, Message, Theme, Renderer>; 4],
        zoomed: Option<Element<'a, Message, Theme, Renderer>>,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        class: Rc<<Theme as Catalog>::Class<'static>>,
        theme: Rc<RefCell<Option<Theme>>>,
        handle_width: f32,
    ) -> Self {
        Self {
            dock_state,
            children,
            zoomed,
            on_event,
            class,
            theme,
            handle_width,
            zoom_placeholder: Element::new(iced::widget::Space::new()),
        }
    }

    fn style(&self) -> DockStyle {
        match self.theme.borrow().as_ref() {
            Some(theme) => Catalog::style(theme, &self.class),
            None => crate::dock::style::default(&iced::Theme::Dark),
        }
    }

    /// The placement a handle under `cursor` belongs to, if any.
    fn handle_under_cursor(state: &DockAreaState, layout: Layout<'_>, cursor: Cursor) -> Option<DockPlacement> {
        let position = cursor.position()?;
        let origin = layout.position();
        state.handle_bounds.iter().enumerate().find_map(|(slot, bounds)| {
            let bounds = (*bounds)? + Vector::new(origin.x, origin.y);
            bounds
                .contains(position)
                .then(|| placement_of(slot))
                .flatten()
        })
    }
}

impl<K, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for DockArea<'_, K, Message, Theme, Renderer>
where
    K: 'static,
    Message: Clone + 'static,
    Theme: Catalog + menu::Catalog + Clone + PartialEq + 'static,
    Renderer: iced::advanced::Renderer + 'static,
{
    fn tag(&self) -> Tag {
        Tag::of::<DockAreaState>()
    }

    fn state(&self) -> State {
        State::new(DockAreaState::default())
    }

    fn children(&self) -> Vec<Tree> {
        // The zoom slot is always present, even when nothing is zoomed, so the
        // child list never changes length. A list that grew and shrank would make
        // iced re-key every child's state whenever a panel was maximized, throwing
        // away the scroll offset of every tab bar in the dock.
        let mut trees: Vec<Tree> = self.children.iter().map(Tree::new).collect();
        trees.push(match &self.zoomed {
            Some(zoomed) => Tree::new(zoomed),
            None => Tree::new(&self.zoom_placeholder),
        });
        trees
    }

    fn diff(&self, tree: &mut Tree) {
        // Every child, including the zoom slot: `diff_children` walks the slice it
        // is given, so passing only the four regions would leave the fifth slot
        // unseeded while `children` claimed it exists.
        let mut all: Vec<&Element<'_, Message, Theme, Renderer>> =
            self.children.iter().collect();
        all.push(self.zoomed.as_ref().unwrap_or(&self.zoom_placeholder));
        tree.diff_children(&all);
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
        let max = limits.max();
        let state = tree.state.downcast_mut::<DockAreaState>();
        state.handle_bounds = [None; 4];
        self.dock_state.borrow_mut().area_bounds = Some(Rectangle {
            x: 0.0,
            y: 0.0,
            width: max.width,
            height: max.height,
        });

        // A zoomed panel is the whole area: the docks are not drawn around it,
        // because "maximize" means the panel is what you are looking at. They are
        // still laid out at zero size, so the node count matches the child count in
        // every state — iced matches tree state against that count.
        let zero = layout::Limits::new(Size::ZERO, Size::ZERO);
        let mut zoom_node = match self.zoomed.as_mut() {
            Some(zoomed) => compose::child_layout(
                zoomed,
                &mut tree.children[ZOOM_SLOT],
                renderer,
                &full_limits(max),
            ),
            None => compose::child_layout(
                &mut self.zoom_placeholder,
                &mut tree.children[ZOOM_SLOT],
                renderer,
                &zero,
            ),
        };

        if self.zoomed.is_some() {
            for slot in 0..4 {
                compose::child_layout(
                    &mut self.children[slot],
                    &mut tree.children[slot],
                    renderer,
                    &zero,
                );
            }
            zoom_node.move_to_mut((0.0, 0.0));
            return layout::Node::with_children(max, vec![zoom_node]);
        }

        let (rects, handles) = self.region_rects(max);
        let mut nodes = Vec::with_capacity(ZOOM_SLOT);
        for (slot, rect) in rects.iter().enumerate().take(4) {
            // Both bounds are the region's size, not `ZERO..size`: a region is a slot
            // that *is* that size, and a `Fill` child under a zero minimum collapses
            // to its content. A pane shrunk to a few dozen pixels still renders and
            // still hits its own tab bar, but its content area — which is what a drop
            // is measured against — is far too small to drop into.
            let extent = Size::new(rect.width, rect.height);
            let mut node = compose::child_layout(
                &mut self.children[slot],
                &mut tree.children[slot],
                renderer,
                &layout::Limits::new(extent, extent),
            );
            node.move_to_mut((rect.x, rect.y));
            nodes.push(node);
        }
        zoom_node.move_to_mut((0.0, 0.0));
        let _ = zoom_node;

        for (slot, handle) in handles.iter().enumerate() {
            state.handle_bounds[slot] = *handle;
        }

        layout::Node::with_children(max, nodes)
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
        let state = tree.state.downcast_ref::<DockAreaState>();
        let dock_style = self.style();
        let _ = theme;

        // The handles are the area's own chrome, so they are drawn here rather
        // than by a child: a child would have to be told about the drag, and the
        // drag is the area's.
        let resizing = self.dock_state.borrow().resizing_dock;
        let origin = layout.position();
        for (slot, bounds) in state.handle_bounds.iter().enumerate() {
            let Some(bounds) = *bounds else { continue };
            let placement = placement_of(slot);
            let dragging = placement.is_some() && resizing == placement;
            let hovered = placement.is_some() && state.hovered == placement;
            let color = if dragging {
                dock_style.control.handle.drag_color
            } else if hovered {
                dock_style.control.handle.hover_color
            } else {
                dock_style.control.handle.idle_color
            };
            if color.a <= 0.0 {
                continue;
            }
            let axis = placement.map_or(Axis::Horizontal, DockPlacement::axis);
            // The drag line is thin even though the grab area is wide: the extra
            // width is there to make the handle easy to grab, not to draw a band.
            let line = match axis {
                Axis::Horizontal => Rectangle {
                    x: bounds.x + origin.x + (bounds.width - 1.0) / 2.0,
                    y: bounds.y + origin.y,
                    width: 1.0,
                    height: bounds.height,
                },
                Axis::Vertical => Rectangle {
                    x: bounds.x + origin.x,
                    y: bounds.y + origin.y + (bounds.height - 1.0) / 2.0,
                    width: bounds.width,
                    height: 1.0,
                },
            };
            renderer.fill_quad(
                renderer::Quad {
                    bounds: line,
                    ..renderer::Quad::default()
                },
                color,
            );
        }

        let child_count = if self.zoomed.is_some() { 1 } else { 4 };
        for (slot, child_layout) in layout.children().take(child_count).enumerate() {
            let child_tree = if self.zoomed.is_some() {
                tree.children.get(ZOOM_SLOT)
            } else {
                tree.children.get(slot)
            };
            let element = if self.zoomed.is_some() {
                self.zoomed.as_ref()
            } else {
                self.children.get(slot)
            };
            if let (Some(child), Some(child_tree)) = (element, child_tree) {
                compose::child_draw(
                    child,
                    child_tree,
                    renderer,
                    theme,
                    style,
                    child_layout,
                    cursor,
                    viewport,
                );
            }
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
        // Forward to the regions first, so a press inside one of them is claimed
        // by the widget under the pointer before the area considers it a handle
        // drag. The handle rects sit between the regions, so the two cannot
        // overlap — but the ordering is what makes that true of the *whole*
        // handle band rather than of its centre line.
        let child_count = if self.zoomed.is_some() { 1 } else { 4 };
        for (slot, child_layout) in layout.children().take(child_count).enumerate() {
            let index = if self.zoomed.is_some() { ZOOM_SLOT } else { slot };
            let element = if self.zoomed.is_some() {
                self.zoomed.as_mut()
            } else {
                self.children.get_mut(slot)
            };
            let Some(child) = element else { continue };
            let Some(child_tree) = tree.children.get_mut(index) else {
                continue;
            };
            compose::child_update(
                child,
                child_tree,
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        if self.zoomed.is_some() {
            return;
        }

        let state = tree.state.downcast_mut::<DockAreaState>();
        let dragging = self.dock_state.borrow().resizing_dock;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. }) => {
                if let Some(placement) = Self::handle_under_cursor(state, layout, cursor) {
                    self.dock_state.borrow_mut().resizing_dock = Some(placement);
                    if state.hovered != Some(placement) {
                        state.hovered = Some(placement);
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(placement) = dragging {
                    if let Some(position) = cursor.position() {
                        shell.publish((self.on_event)(DockAction::DockResize {
                            placement,
                            cursor: position,
                        }));
                        shell.capture_event();
                        shell.request_redraw();
                    }
                } else {
                    let hovered = Self::handle_under_cursor(state, layout, cursor);
                    if hovered != state.hovered {
                        state.hovered = hovered;
                        shell.request_redraw();
                    }
                }
            }
            // A pointer lifted anywhere ends the drag, so the handle keeps no
            // claim on a pointer that has left it.
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerLifted { .. })
                if dragging.is_some() =>
            {
                self.dock_state.borrow_mut().resizing_dock = None;
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
        if self.zoomed.is_some() {
            return mouse::Interaction::None;
        }
        let state = tree.state.downcast_ref::<DockAreaState>();
        let dragging = self.dock_state.borrow().resizing_dock;
        if let Some(placement) = dragging {
            return interaction_for(placement.axis());
        }
        if let Some(placement) = Self::handle_under_cursor(state, layout, cursor) {
            return interaction_for(placement.axis());
        }

        let mut interaction = mouse::Interaction::None;
        for (slot, child_layout) in layout.children().take(4).enumerate() {
            if let (Some(child), Some(child_tree)) =
                (self.children.get(slot), tree.children.get(slot))
            {
                interaction = interaction.max(compose::child_mouse_interaction(
                    child,
                    child_tree,
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                ));
            }
        }
        interaction
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let child_count = if self.zoomed.is_some() { 1 } else { 4 };
        for (slot, child_layout) in layout.children().take(child_count).enumerate() {
            let index = if self.zoomed.is_some() { ZOOM_SLOT } else { slot };
            let element = if self.zoomed.is_some() {
                self.zoomed.as_mut()
            } else {
                self.children.get_mut(slot)
            };
            if let (Some(child), Some(child_tree)) = (element, tree.children.get_mut(index)) {
                compose::child_operate(child, child_tree, child_layout, renderer, operation);
            }
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
        let mut overlays = Vec::new();
        let zoomed = self.zoomed.is_some();

        // `split_at_mut` rather than indexing twice: the borrowed element and its
        // tree state have to be handed out together, and the borrow checker only
        // accepts that when the two slices are provably disjoint.
        if zoomed {
            let has_zoomed_tree = tree.children.len() > SLOT_RIGHT;
            if has_zoomed_tree {
                if let Some(child) = self.zoomed.as_mut() {
                    if let Some(child_layout) = layout.children().next() {
                        let (_, tail) = tree.children.split_at_mut(ZOOM_SLOT);
                        if let Some(element) = child.as_widget_mut().overlay(
                            &mut tail[0],
                            child_layout,
                            renderer,
                            viewport,
                            translation,
                        ) {
                            overlays.push(element);
                        }
                    }
                }
            }
        } else {
            // One iterator over each side, zipped, so every iteration hands out a
            // disjoint element and tree state — indexing inside the loop would
            // re-borrow the whole slice each time.
            let children = &mut self.children;
            for ((child, child_tree), child_layout) in children
                .iter_mut()
                .zip(tree.children.iter_mut())
                .zip(layout.children())
            {
                if let Some(element) = child.as_widget_mut().overlay(
                    child_tree,
                    child_layout,
                    renderer,
                    viewport,
                    translation,
                ) {
                    overlays.push(element);
                }
            }
        }

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

fn full_limits(max: Size) -> layout::Limits {
    layout::Limits::new(Size::ZERO, max)
}

fn interaction_for(axis: Axis) -> mouse::Interaction {
    match axis {
        Axis::Horizontal => mouse::Interaction::ResizingHorizontally,
        Axis::Vertical => mouse::Interaction::ResizingVertically,
    }
}

impl<K, Message, Theme, Renderer> DockArea<'_, K, Message, Theme, Renderer>
where
    K: 'static,
    Message: Clone + 'static,
    Theme: Catalog + menu::Catalog + Clone + PartialEq + 'static,
    Renderer: iced::advanced::Renderer + 'static,
{
    /// Where each region goes, and the grab area of each handle.
    ///
    /// The arrangement is `[left] [centre column: centre, bottom] [right]`, so a
    /// side dock takes the full height and the bottom dock spans what the sides
    /// leave. A handle sits on a dock's inner edge and only exists while the dock
    /// is open — a closed dock is not something to drag.
    fn region_rects(
        &self,
        max: Size,
    ) -> ([Rectangle; 4], [Option<Rectangle>; 4]) {
        let state = self.dock_state.borrow();
        let regions = &state.regions;
        let handle = self.handle_width;

        let left_open = regions.is_dock_open(DockPlacement::Left);
        let right_open = regions.is_dock_open(DockPlacement::Right);
        let bottom_open = regions.is_dock_open(DockPlacement::Bottom);

        // A dock whose every panel is hidden has nothing to draw, so it takes no
        // extent and offers no handle — the same give-back a closed dock makes.
        // Holding the space anyway would read as "hide did not work": the panel
        // is gone but the region it lived in stays on screen as a blank slab.
        // The dock's own open flag is untouched, so showing a panel again
        // restores it at the size the user dragged.
        let draws_left = left_open
            && regions
                .region(DockPlacement::Left)
                .is_none_or(|r| !state.region_is_empty(&r.tree));
        let draws_right = right_open
            && regions
                .region(DockPlacement::Right)
                .is_none_or(|r| !state.region_is_empty(&r.tree));
        let draws_bottom = bottom_open
            && regions
                .region(DockPlacement::Bottom)
                .is_none_or(|r| !state.region_is_empty(&r.tree));

        // The extent is asked per *drawn* state: `extent` alone answers the
        // open flag, so an open but empty dock would come back at full size.
        let dock_draws = |placement, draws| {
            if draws {
                regions.extent(placement)
            } else {
                0.0
            }
        };
        let left_w = dock_draws(DockPlacement::Left, draws_left);
        let right_w = dock_draws(DockPlacement::Right, draws_right);
        let bottom_h = dock_draws(DockPlacement::Bottom, draws_bottom);

        let left_grip = if draws_left { handle } else { 0.0 };
        let right_grip = if draws_right { handle } else { 0.0 };
        let bottom_grip = if draws_bottom { handle } else { 0.0 };

        let center_x = left_w + left_grip;
        let center_w = (max.width - left_w - left_grip - right_w - right_grip).max(0.0);
        let center_h = (max.height - bottom_h - bottom_grip).max(0.0);

        let mut rects = [Rectangle::default(); 4];
        rects[SLOT_CENTER] = Rectangle {
            x: center_x,
            y: 0.0,
            width: center_w,
            height: center_h,
        };
        rects[SLOT_LEFT] = Rectangle {
            x: 0.0,
            y: 0.0,
            width: left_w,
            height: max.height,
        };
        rects[SLOT_BOTTOM] = Rectangle {
            x: center_x,
            y: max.height - bottom_h,
            width: center_w,
            height: bottom_h,
        };
        rects[SLOT_RIGHT] = Rectangle {
            x: max.width - right_w,
            y: 0.0,
            width: right_w,
            height: max.height,
        };

        let mut handles = [None; 4];
        if draws_left {
            handles[SLOT_LEFT] = Some(Rectangle {
                x: left_w,
                y: 0.0,
                width: handle.min(center_w),
                height: max.height,
            });
        }
        if draws_right {
            handles[SLOT_RIGHT] = Some(Rectangle {
                x: max.width - right_w - handle,
                y: 0.0,
                width: handle.min(center_w),
                height: max.height,
            });
        }
        if draws_bottom {
            // Sits in the gap the centre leaves above the dock, so the handle is
            // inside the centre's column rather than over the dock's chrome.
            handles[SLOT_BOTTOM] = Some(Rectangle {
                x: center_x,
                y: max.height - bottom_h - handle,
                width: center_w,
                height: handle.min(center_h),
            });
        }

        (rects, handles)
    }
}

impl<'a, K, Message, Theme, Renderer> From<DockArea<'a, K, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    K: 'static,
    Message: Clone + 'static,
    Theme: Catalog + menu::Catalog + Clone + PartialEq + 'static,
    Renderer: iced::advanced::Renderer + 'static,
{
    fn from(area: DockArea<'a, K, Message, Theme, Renderer>) -> Self {
        Element::new(area)
    }
}
