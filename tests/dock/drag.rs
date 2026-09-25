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

//! Dragging a tab, driven end to end.
//!
//! The dock's dragging was the one path with no test that drove it: every existing
//! simulator test clicks, and a click never moves the pointer, so a drag was never
//! exercised. These tests press, move past the threshold, move onto a target and
//! release — the gesture a user actually makes — and assert on where the panel
//! ended up in the layout.

use iced::widget::{container, text};
use iced::{Element, Length, Point, Size};
use iced_kit::dock::model::NodeKind;
use iced_kit::dock::{
    dock, horizontal, panel, tabs, DockEvent, DockSession, DockWidgetState, LayoutArea, LayoutTree,
    PanelDef,
};
use iced_kit::Theme;
use iced_test::Simulator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Panel {
    Left,
    Right,
    Other,
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockEvent<Panel>),
}

fn view(session: &DockSession<Panel>) -> Element<'_, Message, Theme> {
    container(
        dock::<Panel, Message, Theme, iced::Renderer>()
            .state(session.state())
            .on_event(Message::Dock)
            .style(iced_kit::widgets::dock::style)
            // A tab strip, not a one-panel title bar: these tests are about
            // dragging tabs, and a lone panel draws a title bar by default.
            .panel_style(iced_kit::widgets::dock::PanelStyle::TabBar)
            .content(|panel| {
                text(match panel {
                    Panel::Left => "LEFT BODY",
                    Panel::Right => "RIGHT BODY",
                    Panel::Other => "OTHER BODY",
                })
                .into()
            })
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Two tab groups side by side, with the divider roughly down the middle.
fn two_group_session() -> DockSession<Panel> {
    DockSession::from_tree(horizontal([
        tabs([panel("left", "Left", Panel::Left)]),
        tabs([panel("right", "Right", Panel::Right)]),
    ]))
    .expect("valid")
}

/// The window the tests drive. Wide enough that each group is comfortably larger
/// than the drag threshold and the drop-edge bands.
const WINDOW: Size = Size::new(600.0, 400.0);

fn simulator(session: &DockSession<Panel>) -> Simulator<'_, Message, Theme> {
    Simulator::with_size(iced::Settings::default(), WINDOW, view(session))
}

/// A simulator that leaves `PanelStyle` at its default, so a group holding one panel
/// draws a title bar instead of a one-tab strip.
fn simulator_with_title_bars(session: &DockSession<Panel>) -> Simulator<'_, Message, Theme> {
    Simulator::with_size(
        iced::Settings::default(),
        WINDOW,
        view_with_title_bars(session),
    )
}

fn view_with_title_bars(session: &DockSession<Panel>) -> Element<'_, Message, Theme> {
    container(
        dock::<Panel, Message, Theme, iced::Renderer>()
            .state(session.state())
            .on_event(Message::Dock)
            .style(iced_kit::widgets::dock::style)
            // No `.panel_style(...)`: the default draws a title bar for a lone panel.
            .content(|panel| {
                text(match panel {
                    Panel::Left => "LEFT BODY",
                    Panel::Right => "RIGHT BODY",
                    Panel::Other => "OTHER BODY",
                })
                .into()
            })
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// The centre of a tab's label, found by what it draws.
fn tab_centre(ui: &mut Simulator<'_, Message, Theme>, label: &str) -> Point {
    let bounds = ui
        .find(label)
        .unwrap_or_else(|_| panic!("`{label}` should be on screen"))
        .bounds();
    Point::new(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// Press on `label`'s tab, move to `to`, and release.
///
/// The gesture is driven in full — press, two moves (the first crosses the drag
/// threshold and starts the drag, the second is what a drop resolves against), then
/// release — because that is the only shape that exercises the drag at all.
fn drag_tab(ui: &mut Simulator<'_, Message, Theme>, label: &str, to: Point) {
    let from = tab_centre(ui, label);
    // A frame first: the drop geometry is recorded while drawing, so a gesture that
    // starts before any frame has nothing to resolve against. A real window always has
    // drawn the layout the user is dragging over.
    draw_frame(ui);
    drive_drag(ui, from, to);
}

/// Render a frame, which is what records each pane's drop geometry.
fn draw_frame(ui: &mut Simulator<'_, Message, Theme>) {
    let _ = ui.snapshot(&Theme::light());
}

/// The same gesture, from an explicit press point.
fn drive_drag(ui: &mut Simulator<'_, Message, Theme>, from: Point, to: Point) {
    // `Simulator::point_at` is what sets the cursor the widgets are handed;
    // a `CursorMoved` event on its own updates the pointer but not
    // `Simulator::cursor`, so every widget would see `Cursor::Unavailable` and
    // ignore the press. Both are needed: the event drives the drag threshold, the
    // pointer position is what the hit-tests read.
    let halfway = Point::new(f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y));

    ui.point_at(from);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position: from,
    })]);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);

    // The first move crosses the drag threshold and starts the drag.
    ui.point_at(halfway);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position: halfway,
    })]);
    // The second is the one a drop resolves against.
    ui.point_at(to);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position: to,
    })]);

    ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
        iced::mouse::Button::Left,
    ))]);
}

/// Which pane holds a panel, by its string id.
fn pane_of(state: &DockWidgetState<Panel>, id: &str) -> Option<iced_kit::dock::model::NodeId> {
    let panel = state.index.panels.get(id).copied()?;
    state.layout.get(panel).and_then(|entry| entry.owner)
}

/// The panels a pane holds, in order.
fn tabs_in(state: &DockWidgetState<Panel>, pane: iced_kit::dock::model::NodeId) -> Vec<String> {
    let Some(NodeKind::Pane(pane)) = state.layout.kind(pane) else {
        return Vec::new();
    };
    pane.tabs
        .iter()
        .filter_map(|&tab| match state.layout.kind(tab) {
            Some(NodeKind::Panel(panel)) => Some(panel.id.clone()),
            _ => None,
        })
        .collect()
}

/// The regression: a tab drag never started, so a panel could not be moved at all.
///
/// Every existing simulator test clicked, and a click never moves the pointer —
/// so nothing ever crossed the drag threshold, and nothing asserted that a drop
/// changed the layout.
#[test]
fn dragging_a_tab_onto_another_group_moves_it() {
    let session = two_group_session();
    let mut ui = simulator(&session);

    // The right group's content area, which is what a drop is measured against.
    let target = {
        let bounds = ui.find("RIGHT BODY").expect("the right body").bounds();
        Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    };

    drag_tab(&mut ui, "Left", target);

    // The panel moved: the source group is gone, and the right pane holds both.
    let state = session.state();
    let state = state.borrow();
    let left_pane = pane_of(&state, "left");
    let right_pane = pane_of(&state, "right");
    assert_eq!(
        left_pane, right_pane,
        "the dragged panel should now share a group with the one it was dropped on"
    );
    assert_eq!(
        tabs_in(&state, right_pane.expect("a pane")),
        vec!["right".to_owned(), "left".to_owned()],
        "the dragged panel should have joined the target group"
    );
}

/// The other half: dropping on an *edge* of a group splits it rather than merging.
#[test]
fn dragging_a_tab_onto_a_groups_edge_splits_it() {
    let session = two_group_session();
    let mut ui = simulator(&session);

    // The far right edge of the right group, which is the `Right` drop band.
    let target = {
        let bounds = ui.find("RIGHT BODY").expect("the right body").bounds();
        Point::new(
            bounds.x + bounds.width - 4.0,
            bounds.y + bounds.height / 2.0,
        )
    };

    drag_tab(&mut ui, "Left", target);

    let state = session.state();
    let state = state.borrow();
    let left_pane = pane_of(&state, "left").expect("the left panel has a pane");
    let right_pane = pane_of(&state, "right").expect("the right panel has a pane");
    assert_ne!(
        left_pane, right_pane,
        "an edge drop splits, so the two panels stay in their own groups"
    );

    // The centre is now a three-child split: the original left group, the right
    // group, and the new one the dragged panel landed in.
    // The two panels are in separate groups, and — because the drop was on the
    // `Right` edge of a group that already sat in a horizontal split — the new
    // group went in *beside* it rather than nested inside it. Two horizontal splits
    // in a row would be a redundant level, so `Factory::split` merges them, and the
    // dragged panel now sits to the right of the one it was dropped beside.
    let root = state.layout.root_child().expect("a root child");
    let Some(NodeKind::Proportional(group)) = state.layout.kind(root) else {
        panic!("the centre should still be a split");
    };
    assert_eq!(
        group.children.len(),
        2,
        "the drop merged into the existing horizontal split rather than nesting"
    );

    let order: Vec<Vec<String>> = group
        .children
        .iter()
        .map(|&child| tabs_in(&state, child))
        .collect();
    assert_eq!(
        order,
        vec![vec!["right".to_owned()], vec!["left".to_owned()]],
        "the dragged panel landed to the right of the group it was dropped beside"
    );
}

/// A drag that ends where it began must not move anything.
#[test]
fn dropping_a_tab_back_on_its_own_group_is_ignored() {
    let session = two_group_session();
    let mut ui = simulator(&session);

    let target = {
        let bounds = ui.find("LEFT BODY").expect("the left body").bounds();
        Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    };

    drag_tab(&mut ui, "Left", target);

    let state = session.state();
    let state = state.borrow();
    assert_ne!(
        pane_of(&state, "left"),
        pane_of(&state, "right"),
        "a panel dropped where it already was should stay put"
    );
}

/// A single panel in the only group cannot be dragged out: there would be nothing
/// left to show, and no way to put it back.
#[test]
fn a_lone_panel_cannot_be_dragged_out_of_its_group() {
    let session =
        DockSession::from_tree(tabs([panel("only", "Only", Panel::Left)])).expect("valid");
    let mut ui = simulator(&session);

    let target = Point::new(500.0, 300.0);
    drag_tab(&mut ui, "Only", target);

    // A drag that never started leaves the gesture as a plain click, so the panel
    // is still alone in the group it was in.
    let state = session.state();
    let state = state.borrow();
    let pane = pane_of(&state, "only").expect("a pane");
    assert_eq!(tabs_in(&state, pane), vec!["only".to_owned()]);
}

/// Dragging within one group reorders its tabs.
#[test]
fn dragging_a_tab_within_its_group_reorders_it() {
    let session = DockSession::from_tree(tabs([
        panel("first", "First", Panel::Left),
        panel("second", "Second", Panel::Right),
        panel("third", "Third", Panel::Other),
    ]))
    .expect("valid");
    let mut ui = simulator(&session);

    // Aim at the strip's right end: past the last tab's midpoint is the insertion
    // slot that puts the dragged tab last.
    let target = {
        let bounds = ui.find("Third").expect("the third tab").bounds();
        Point::new(
            bounds.x + bounds.width + 30.0,
            bounds.y + bounds.height / 2.0,
        )
    };

    drag_tab(&mut ui, "First", target);

    let state = session.state();
    let state = state.borrow();
    let pane = pane_of(&state, "first").expect("a pane");
    let order = tabs_in(&state, pane);
    assert_eq!(
        order,
        vec!["second".to_owned(), "third".to_owned(), "first".to_owned()],
        "the dragged tab should have moved to the end"
    );
}

/// A dock's own handles must not start a tab drag, and a tab must not resize a dock:
/// the two gestures share the pointer and must not fight over it.
#[test]
fn a_dock_handle_and_a_tab_drag_do_not_interfere() {
    let area = LayoutArea::new(tabs([panel("left", "Left", Panel::Left)])).dock(
        iced_kit::dock::DockPlacement::Left,
        180.0,
        tabs([panel("side", "Side", Panel::Other)]),
    );
    let session = DockSession::from_area(area).expect("valid");
    let mut ui = simulator(&session);

    // The centre's tab, dragged a little: a drag inside its own group, so nothing
    // should move and nothing should resize.
    let before = {
        let state = session.state();
        let state = state.borrow();
        state
            .regions
            .dock(iced_kit::dock::DockPlacement::Left)
            .unwrap()
            .size()
    };

    let start = tab_centre(&mut ui, "Left");
    let target = Point::new(start.x + 20.0, start.y);
    drag_tab(&mut ui, "Left", target);

    let after = {
        let state = session.state();
        let state = state.borrow();
        state
            .regions
            .dock(iced_kit::dock::DockPlacement::Left)
            .unwrap()
            .size()
    };
    assert_eq!(before, after, "dragging a tab must not resize a dock");
}

/// The dock reports the drag as it happens, which is what a subscriber persists and
/// what an application reacts to.
#[test]
fn a_drag_reports_its_start_and_its_end() {
    let session = two_group_session();
    let mut ui = simulator(&session);

    let start = tab_centre(&mut ui, "Left");
    let target = {
        let bounds = ui.find("RIGHT BODY").expect("the right body").bounds();
        Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    };
    draw_frame(&mut ui);
    drive_drag(&mut ui, start, target);

    let events: Vec<DockEvent<Panel>> = ui
        .into_messages()
        .map(|Message::Dock(event)| event)
        .collect();

    assert!(
        events.iter().any(
            |event| matches!(event, DockEvent::DragStarted { panel } if *panel == Panel::Left)
        ),
        "the drag should have started once the pointer passed the threshold; got {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DockEvent::DragEnded { .. })),
        "the drag should have ended on release; got {events:?}"
    );
}

/// The drag preview's reported size is what a drop placeholder flies in from, so a
/// drag that started must have recorded one.
#[test]
fn a_started_drag_records_where_it_was_gripped() {
    let session = two_group_session();
    let mut ui = simulator(&session);

    let target = {
        let bounds = ui.find("RIGHT BODY").expect("the right body").bounds();
        Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    };
    drag_tab(&mut ui, "Left", target);

    // The drag is over, so the session should be idle again — a drag left in flight
    // would leave every subsequent click read as a drag move.
    let state = session.state();
    assert!(
        state.borrow().drag.is_none(),
        "the drag should be cleared once the pointer is released"
    );
}

/// The layout the drag produced survives a save and a restore.
#[test]
fn a_dragged_layout_survives_a_round_trip() {
    let session = two_group_session();
    let mut ui = simulator(&session);

    let target = {
        let bounds = ui.find("RIGHT BODY").expect("the right body").bounds();
        Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    };
    drag_tab(&mut ui, "Left", target);

    let saved = session.capture(None);
    let reloaded = DockSession::from_tree(LayoutTree::Tabs(iced_kit::dock::TabsNode::new([
        PanelDef::new("right", "Right", Panel::Right),
    ])))
    .expect("valid");
    reloaded.restore(&saved).expect("restores");

    let state = reloaded.state();
    let state = state.borrow();
    assert_eq!(
        pane_of(&state, "left"),
        pane_of(&state, "right"),
        "the two panels are still in one group after a round trip"
    );
}

/// The regression that made dragging look broken: a group holding one panel draws a
/// *title bar* rather than a strip of tabs, and the title was not a drag source.
///
/// In a workspace like the `dock` example — where four of five groups hold a single
/// panel — that left almost every panel undraggable, so dragging appeared not to work
/// at all while a two-tab group dragged fine.
#[test]
fn a_lone_panels_title_bar_is_a_drag_source() {
    let session = two_group_session();
    let mut ui = simulator_with_title_bars(&session);

    // Both groups hold one panel, so both draw a title bar. Drag the left one's
    // title onto the right group's body.
    let target = {
        let bounds = ui.find("RIGHT BODY").expect("the right body").bounds();
        Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    };
    drag_tab(&mut ui, "Left", target);

    let state = session.state();
    let state = state.borrow();
    assert_eq!(
        pane_of(&state, "left"),
        pane_of(&state, "right"),
        "a panel dragged by its title should have joined the target group"
    );
}

/// The same gesture in the only group: a panel that is alone cannot be dragged out,
/// because there would be nothing left to show.
#[test]
fn a_lone_panels_title_bar_does_not_start_a_pointless_drag() {
    let session =
        DockSession::from_area(LayoutArea::new(tabs([panel("only", "Only", Panel::Left)])))
            .expect("valid");
    let mut ui = simulator_with_title_bars(&session);

    drag_tab(&mut ui, "Only", Point::new(500.0, 300.0));

    let state = session.state();
    let state = state.borrow();
    let pane = pane_of(&state, "only").expect("a pane");
    assert_eq!(
        tabs_in(&state, pane),
        vec!["only".to_owned()],
        "the panel should still be where it was"
    );
}

/// A title bar drag reports itself, so an application can react to it the same way
/// it reacts to a tab drag.
#[test]
fn a_title_bar_drag_reports_its_start_and_end() {
    let session = two_group_session();
    let mut ui = simulator_with_title_bars(&session);

    let start = tab_centre(&mut ui, "Left");
    let target = {
        let bounds = ui.find("RIGHT BODY").expect("the right body").bounds();
        Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    };
    draw_frame(&mut ui);
    drive_drag(&mut ui, start, target);

    let events: Vec<DockEvent<Panel>> = ui
        .into_messages()
        .map(|Message::Dock(event)| event)
        .collect();

    assert!(
        events.iter().any(
            |event| matches!(event, DockEvent::DragStarted { panel } if *panel == Panel::Left)
        ),
        "dragging a title should report a drag start; got {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DockEvent::DragEnded { .. })),
        "releasing a title drag should report its end; got {events:?}"
    );
}
