// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.
//
// Keeps upstream's lint allowances, declared in its own `Cargo.toml`, rather
// than rewriting test code this project did not author.
#![allow(
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::default_trait_access,
    clippy::similar_names,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    clippy::type_complexity,
    clippy::unused_self
)]

//! A drag, driven the way `iced` itself drives one.
//!
//! The simulator's convenience methods are not enough here, because they settle the
//! interface between calls: they run `update`, then `layout`, then draw, for every
//! event. A real window does the same thing *inside* a single `update` — when a
//! widget calls `shell.invalidate_layout()`, `iced` re-runs `layout()` before the
//! next event is delivered (see `UserInterface::update`).
//!
//! That distinction is the whole point of this file. A dock that rebuilds its widget
//! tree during a drag loses the drag's own state when `Tree::diff` sees a different
//! tag and replaces the subtree — a failure that a per-event-settled simulator hides
//! completely, because the state it discards is rebuilt before the next assertion.
//!
//! So these tests drive `Dock` directly and re-run `layout()` after every event,
//! matching `iced`'s own sequence.

use std::collections::HashMap;

use iced::advanced::layout;
use iced::advanced::widget::Tree;
use iced::advanced::Shell;
use iced::{Element, Event, Point, Rectangle, Size};
use iced_kit::widgets::dock as dock_kit;
use iced_kit::dock::{horizontal, panel, tabs, DockEvent, DockSession};
use iced_kit::Theme;
use iced_test::renderer::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Panel {
    Left,
    Right,
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockEvent<Panel>),
}

type DockWidget<'a> = iced_kit::dock::Dock<'a, Panel, Message, Theme, Renderer>;

/// The window these tests drive.
const WINDOW: Size = Size::new(600.0, 400.0);

/// A two-group centre, so a tab can be dragged from one into the other.
fn session() -> DockSession<Panel> {
    DockSession::from_tree(horizontal([
        tabs([panel("left", "Left", Panel::Left)]),
        tabs([panel("right", "Right", Panel::Right)]),
    ]))
    .expect("valid")
}

fn view(session: &DockSession<Panel>) -> DockWidget<'_> {
    dock_kit::dock::<Panel, Message, Theme, Renderer>()
        .state(session.state())
        .on_event(Message::Dock)
        .style(iced_kit::widgets::dock::style)
        // A strip of tabs rather than a one-panel title bar, so the drag source is
        // the tab strip — the path a `PanelStyle::Auto` workspace uses for any group
        // holding more than one panel.
        .panel_style(iced_kit::widgets::dock::PanelStyle::TabBar)
        .content(|panel| {
            iced::widget::text(match panel {
                Panel::Left => "LEFT BODY",
                Panel::Right => "RIGHT BODY",
            })
            .into()
        })
        .build()
}

/// A driver that runs the sequence `iced` runs: deliver one event, then re-layout
/// if the widget asked for it, then deliver the next.
struct Driver<'a> {
    /// Wrapped in an `Element`, because `Tree::new` takes one and the `Dock` is
    /// addressed as a widget from inside it.
    root: Element<'a, Message, Theme, Renderer>,
    tree: Tree,
    /// The layout the current frame is being delivered against.
    current: layout::Node,
    renderer: Renderer,
    messages: Vec<Message>,
    /// Every layout pass, so a test can assert the rebuild happened.
    layouts: usize,
}

impl<'a> Driver<'a> {
    fn new(widget: DockWidget<'a>) -> Self {
        let renderer = iced_test::futures::futures::executor::block_on(
            <Renderer as iced::advanced::renderer::Headless>::new(
                iced::Font::with_name("Fira Sans"),
                iced::Pixels(16.0),
                None,
            ),
        )
        .expect("a headless renderer");
        let root: Element<'a, Message, Theme, Renderer> = widget.into();
        let tree = Tree::new(&root);
        Self {
            root,
            tree,
            current: layout::Node::new(WINDOW),
            renderer,
            messages: Vec::new(),
            layouts: 0,
        }
    }

    /// Draw the current tree, registering whatever `draw` records.
    fn draw(&mut self, node: &layout::Node) {
        let style = iced::advanced::renderer::Style::default();
        self.root.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            &Theme::light(),
            &style,
            layout::Layout::new(node),
            iced::mouse::Cursor::Unavailable,
            &Rectangle::with_size(WINDOW),
        );
    }

    /// One layout pass, returning the node so a caller can read bounds.
    fn layout(&mut self) -> layout::Node {
        self.layouts += 1;
        self.current = self.root.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(WINDOW, WINDOW),
        );
        self.current.clone()
    }

    /// Deliver one event the way `iced` does: update, and re-layout if the widget
    /// invalidated the layout during that update.
    /// Start a frame: lay out and draw, so the geometry this frame's events resolve
    /// against is in place. A real window does this once per frame, not per event.
    fn frame(&mut self) {
        let node = self.layout();
        self.draw(&node);
    }

    fn event(&mut self, event: Event, cursor: Point) -> layout::Node {
        // Within one frame every event is delivered against the layout the frame
        // began with; nothing is re-laid-out or re-drawn here.
        let node = self.current.clone();
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut shell = Shell::new(&mut self.messages);
        self.root.as_widget_mut().update(
            &mut self.tree,
            &event,
            layout::Layout::new(&node),
            iced::mouse::Cursor::Available(cursor),
            &self.renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );
        // This is the step the simulator hides: `iced` re-lays-out here, before the
        // next event, and a widget that rebuilt its tree has just changed the tags
        // the tree is diffed against.
        if shell.is_layout_invalid() {
            // `iced` does exactly this: re-run `layout` inside the same `update`, so a
            // widget that rebuilt its tree has already changed the tags the tree is
            // diffed against by the time the next event arrives.
            self.layout()
        } else {
            node
        }
    }
}

/// Which pane a panel is in, so a test can tell whether it moved.
fn pane_map(session: &DockSession<Panel>) -> HashMap<String, iced_kit::dock::model::NodeId> {
    let state = session.state();
    let state = state.borrow();
    let mut map = HashMap::new();
    for (id, &panel) in &state.index.panels {
        if let Some(pane) = state.pane_of_panel(panel) {
            map.insert(id.clone(), pane);
        }
    }
    let _ = &state;
    map
}

/// The regression this file exists for: a drag has to survive the layout pass that
/// its own `DragStarted` triggers.
///
/// `Dock::update` calls `shell.invalidate_layout()` whenever the layout tree changed,
/// which a drag does on its first movement. `iced` then re-runs `layout()` before the
/// next event arrives — and `Dock::layout` rebuilds the whole element tree through
/// `rebuild_root`. If that rebuild changes a widget's tag, `Tree::diff` replaces its
/// state, and the drag in flight disappears between two events.
#[test]
fn a_drag_survives_the_layout_pass_its_own_start_triggers() {
    let session = session();
    let mut driver = Driver::new(view(&session));

    // The gesture: press on the left tab, move twice, release. Every step goes
    // through `Driver::event`, which re-lays-out as `iced` would.
    let start = Point::new(30.0, 15.0);
    let target = Point::new(450.0, 200.0);
    let halfway = Point::new(
        f32::midpoint(start.x, target.x),
        f32::midpoint(start.y, target.y),
    );

    driver.frame();
    driver.event(
        Event::Mouse(iced::mouse::Event::CursorMoved { position: start }),
        start,
    );
    driver.event(
        Event::Mouse(iced::mouse::Event::ButtonPressed(
            iced::mouse::Button::Left,
        )),
        start,
    );
    driver.event(
        Event::Mouse(iced::mouse::Event::CursorMoved {
            position: halfway,
        }),
        halfway,
    );
    driver.event(
        Event::Mouse(iced::mouse::Event::CursorMoved { position: target }),
        target,
    );
    driver.event(
        Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
        target,
    );

    let events: Vec<DockEvent<Panel>> = driver
        .messages
        .iter()
        .filter_map(|message| match message {
            Message::Dock(event) => Some(event.clone()),
            #[allow(unreachable_patterns)]
            _ => None,
        })
        .collect();

    assert!(
        events
            .iter()
            .any(|event| matches!(event, DockEvent::DragStarted { .. })),
        "the drag should have started; got {events:?}"
    );
    // The heart of it: the moves and the release have to arrive *after* the layout
    // rebuild that the start triggered, not be swallowed by it.
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DockEvent::DragEnded { .. })),
        "the release should have ended the drag; got {events:?}"
    );

    // And the panel moved, which is what the user is trying to do.
    let map = pane_map(&session);
    assert_eq!(
        map.get("left"),
        map.get("right"),
        "the dragged panel should share a pane with the one it was dropped on"
    );
}
