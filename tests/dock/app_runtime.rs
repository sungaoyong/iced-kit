// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.
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

//! The dock, driven through the sequence `iced` itself runs.
//!
//! `tests/dock/runtime_drag.rs` pins the widget protocol; this drives a whole
//! application the way a window does: lay out and draw once per frame, then deliver
//! that frame's events against the layout it produced.
//!
//! The distinction matters because the two are not the same program. A window draws
//! between frames, and the geometry a drop resolves against is recorded *while
//! drawing* — so a test that never draws has nothing to drop onto, and one that draws
//! only at the end never exercises the arrangement the user was looking at.

use iced::advanced::layout;
use iced::advanced::widget::Tree;
use iced::advanced::Shell;
use iced::{Event, Point, Rectangle, Size};
use iced_kit::dock::{horizontal, panel, tabs, DockEvent, DockPlacement, DockSession};
use iced_kit::widgets::dock as dock_kit;
use iced_test::renderer::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Panel {
    Files,
    Editor,
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockEvent<Panel>),
}

const WINDOW: Size = Size::new(900.0, 600.0);

fn view(session: &DockSession<Panel>) -> iced::Element<'_, Message, iced_kit::Theme, Renderer> {
    dock_kit::dock::<Panel, Message, iced_kit::Theme, Renderer>()
        .state(session.state())
        .on_event(Message::Dock)
        .style(dock_kit::style)
        // A strip rather than a one-panel title bar, so the drag source is the tab
        // strip: the path a group holding several panels takes.
        .panel_style(dock_kit::PanelStyle::TabBar)
        .content(|panel| {
            iced::widget::text(match panel {
                Panel::Files => "FILES BODY",
                Panel::Editor => "EDITOR BODY",
            })
            .into()
        })
        .build()
        .into()
}

/// One window's worth of state, driven frame by frame.
struct App<'a> {
    root: iced::Element<'a, Message, iced_kit::Theme, Renderer>,
    tree: Tree,
    renderer: Renderer,
    node: layout::Node,
    messages: Vec<Message>,
}

impl<'a> App<'a> {
    fn new(session: &'a DockSession<Panel>) -> Self {
        let renderer = iced_test::futures::futures::executor::block_on(
            <Renderer as iced::advanced::renderer::Headless>::new(
                iced::Font::with_name("Fira Sans"),
                iced::Pixels(16.0),
                None,
            ),
        )
        .expect("a headless renderer");
        let root = view(session);
        let tree = Tree::new(&root);
        Self {
            root,
            tree,
            renderer,
            node: layout::Node::new(WINDOW),
            messages: Vec::new(),
        }
    }

    /// A frame: lay out, then draw. That order is what leaves the drop geometry in
    /// place for the events of the next frame.
    fn frame(&mut self) {
        self.node = self.root.as_widget_mut().layout(
            &mut self.tree,
            &self.renderer,
            &layout::Limits::new(WINDOW, WINDOW),
        );
        let style = iced::advanced::renderer::Style::default();
        self.root.as_widget().draw(
            &self.tree,
            &mut self.renderer,
            &iced_kit::Theme::light(),
            &style,
            layout::Layout::new(&self.node),
            iced::mouse::Cursor::Unavailable,
            &Rectangle::with_size(WINDOW),
        );
    }

    fn event(&mut self, event: Event, cursor: Point) {
        let mut clipboard = iced::advanced::clipboard::Null;
        let mut shell = Shell::new(&mut self.messages);
        self.root.as_widget_mut().update(
            &mut self.tree,
            &event,
            layout::Layout::new(&self.node),
            iced::mouse::Cursor::Available(cursor),
            &self.renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );
        // `iced` re-lays-out inside the same `update` when a widget asks it to, which
        // a dock does whenever its layout tree changed.
        if shell.is_layout_invalid() {
            self.node = self.root.as_widget_mut().layout(
                &mut self.tree,
                &self.renderer,
                &layout::Limits::new(WINDOW, WINDOW),
            );
        }
    }

    fn press(&mut self, at: Point) {
        self.move_to(at);
        self.event(
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            at,
        );
    }

    fn move_to(&mut self, at: Point) {
        self.event(
            Event::Mouse(iced::mouse::Event::CursorMoved { position: at }),
            at,
        );
    }

    fn release(&mut self, at: Point) {
        self.event(
            Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
            at,
        );
    }
}

/// The pane holding a panel, by the panel's string id.
fn pane_of(session: &DockSession<Panel>, id: &str) -> Option<iced_kit::dock::model::NodeId> {
    session.pane_for_panel(id)
}

/// The rectangle a pane was last drawn in.
fn pane_bounds(session: &DockSession<Panel>, pane: iced_kit::dock::model::NodeId) -> Rectangle {
    session
        .state()
        .borrow()
        .pane_bounds
        .iter()
        .find(|(node, _)| *node == pane)
        .map(|(_, bounds)| *bounds)
        .expect("the pane was drawn")
}

/// The regression a real window hit: dragging a tab moved nothing.
///
/// The frame that handles the drop finishes by laying out the new arrangement, so a
/// drop has to resolve against the geometry the *earlier* frames recorded — the
/// arrangement the user was looking at — rather than against what its own release
/// produced.
#[test]
fn dragging_a_tab_between_groups_moves_it() {
    let session = DockSession::from_tree(horizontal([
        tabs([panel("files", "Files", Panel::Files)]),
        tabs([panel("editor", "Editor", Panel::Editor)]),
    ]))
    .expect("valid");
    let mut app = App::new(&session);
    app.frame();

    // The left group's tab is at the window's top-left; the right group's pane fills
    // the right half.
    let start = Point::new(30.0, 14.0);
    let target = pane_bounds(&session, pane_of(&session, "editor").expect("a pane")).center();

    assert_ne!(
        pane_of(&session, "files"),
        pane_of(&session, "editor"),
        "the two panels start in different groups"
    );

    // A drag spanning frames, each frame drawn as a window would.
    app.press(start);
    app.move_to(Point::new(300.0, 150.0));
    app.frame();
    app.move_to(target);
    app.frame();
    app.release(target);

    assert_eq!(
        pane_of(&session, "files"),
        pane_of(&session, "editor"),
        "the dragged panel should have joined the group it was dropped on"
    );
    assert!(
        app.messages
            .iter()
            .any(|message| matches!(message, Message::Dock(DockEvent::DragEnded { .. }))),
        "the release should have ended the drag; got {:?}",
        app.messages
    );
}

/// Dragging a tab onto another group's edge splits that group instead of merging.
#[test]
fn dragging_a_tab_onto_an_edge_splits_the_group() {
    let session = DockSession::from_tree(horizontal([
        tabs([panel("files", "Files", Panel::Files)]),
        tabs([panel("editor", "Editor", Panel::Editor)]),
    ]))
    .expect("valid");
    let mut app = App::new(&session);
    app.frame();

    let editor = pane_bounds(&session, pane_of(&session, "editor").expect("a pane"));
    let start = Point::new(30.0, 14.0);
    // Just inside the right group's right edge, which is the `Right` drop band.
    let edge = Point::new(editor.x + editor.width - 4.0, editor.center().y);

    app.press(start);
    app.move_to(Point::new(300.0, 150.0));
    app.frame();
    app.move_to(edge);
    app.frame();
    app.release(edge);

    assert_ne!(
        pane_of(&session, "files"),
        pane_of(&session, "editor"),
        "an edge drop splits, so the two panels keep their own groups"
    );
}

/// The dock's own toggle collapses its dock.
#[test]
fn the_dock_toggle_collapses_its_dock() {
    let area = dock_kit::LayoutArea::new(tabs([panel("files", "Files", Panel::Files)])).dock(
        DockPlacement::Left,
        220.0,
        tabs([panel("editor", "Editor", Panel::Editor)]),
    );
    let session = DockSession::from_area(area).expect("valid");
    let mut app = App::new(&session);
    app.frame();

    // The left dock's toggle is drawn in the centre's top-left group, at that group's
    // leading edge — immediately before the panel's own title.
    let centre = pane_bounds(&session, pane_of(&session, "files").expect("a pane"));
    let at = Point::new(centre.x + 12.0, centre.y + 16.0);

    app.press(at);
    app.release(at);

    assert!(
        !session
            .state()
            .borrow()
            .regions
            .is_dock_open(DockPlacement::Left),
        "the toggle should have collapsed the left dock"
    );
}
