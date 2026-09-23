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

use iced::widget::{container, text};
use iced::{Element, Length, Point, Size};
use iced_kit::dock::{dock, horizontal, panel, single, tabs, DockEvent, DockSession};
use iced_test::{simulator, Simulator};

#[derive(Debug, Clone)]
enum Message {
    Dock(DockEvent<u32>),
}

// ---------------------------------------------------------------------------
// Session helpers
// ---------------------------------------------------------------------------

fn two_tab_session() -> DockSession<u32> {
    DockSession::from_tree(
        tabs([
            panel("editor", "Editor", 0u32),
            panel("terminal", "Terminal", 1u32),
        ])
        .active("editor"),
    )
    .expect("valid layout")
}

fn three_tab_session() -> DockSession<u32> {
    DockSession::from_tree(
        tabs([
            panel("editor", "Editor", 0u32),
            panel("terminal", "Terminal", 1u32),
            panel("output", "Output", 2u32),
        ])
        .active("editor"),
    )
    .expect("valid layout")
}

fn split_session() -> DockSession<u32> {
    DockSession::from_tree(
        horizontal([
            tabs([
                panel("editor", "Editor", 0u32),
                panel("terminal", "Terminal", 1u32),
            ])
            .active("editor"),
            tabs([
                panel("explorer", "Explorer", 10u32),
                panel("search", "Search", 11u32),
            ])
            .active("explorer"),
        ])
        .weights([0.7, 0.3]),
    )
    .expect("valid layout")
}

fn overflow_session() -> DockSession<u32> {
    DockSession::from_tree(
        tabs([
            panel("file0", "File 0", 0u32),
            panel("file1", "File 1", 1u32),
            panel("file2", "File 2", 2u32),
            panel("file3", "File 3", 3u32),
            panel("file4", "File 4", 4u32),
            panel("file5", "File 5", 5u32),
        ])
        .active("file0"),
    )
    .expect("valid layout")
}

// ---------------------------------------------------------------------------
// View helpers
// ---------------------------------------------------------------------------

fn view(session: &DockSession<u32>) -> Element<'_, Message> {
    container(
        dock()
            .state(session.state())
            .on_event(Message::Dock)
            .content(|key| text(format!("Content {}", key)).into())
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn content_label(key: u32) -> &'static str {
    match key {
        0 => "Content:editor",
        1 => "Content:terminal",
        2 => "Content:output",
        10 => "Content:explorer",
        11 => "Content:search",
        _ => "Content:unknown",
    }
}

fn view_with_unique_content(session: &DockSession<u32>) -> Element<'_, Message> {
    container(
        dock()
            .state(session.state())
            .on_event(Message::Dock)
            .content(|key| text(content_label(key)).into())
            .build(),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn narrow_view(session: &DockSession<u32>) -> Simulator<'_, Message> {
    Simulator::with_size(Default::default(), Size::new(180.0, 240.0), view(session))
}

#[test]
fn clicking_inactive_tab_produces_tab_selected() {
    let session = two_tab_session();
    let mut ui = simulator(view(&session));

    let _ = ui.click("Terminal");

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.iter().any(|msg| matches!(
            msg,
            Message::Dock(DockEvent::TabSelected { panel, .. }) if *panel == 1
        )),
        "expected TabSelected for terminal (1), got: {messages:?}"
    );
}

#[test]
fn find_confirms_tab_labels_present() {
    let session = two_tab_session();
    let mut ui = simulator(view(&session));

    ui.find("Editor").expect("'Editor' tab should be visible");
    ui.find("Terminal")
        .expect("'Terminal' tab should be visible");
}

// ---------------------------------------------------------------------------
// Tab interaction
// ---------------------------------------------------------------------------

#[test]
fn clicking_active_tab_still_produces_select() {
    let session = two_tab_session();
    let mut ui = simulator(view(&session));

    let _ = ui.click("Editor");

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.iter().any(|msg| matches!(
            msg,
            Message::Dock(DockEvent::TabSelected { panel, .. }) if *panel == 0
        )),
        "expected TabSelected for editor (0), got: {messages:?}"
    );
}

#[test]
fn three_tabs_switch_twice() {
    let session = three_tab_session();

    // Step 1: click Terminal
    let mut ui = simulator(view(&session));
    let _ = ui.click("Terminal");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(
        msgs.iter().any(|msg| matches!(
            msg,
            Message::Dock(DockEvent::TabSelected { panel, .. }) if *panel == 1
        )),
        "step 1: expected TabSelected for terminal (1), got: {msgs:?}"
    );

    // Step 2: rebuild view, click Output
    let mut ui = simulator(view(&session));
    let _ = ui.click("Output");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(
        msgs.iter().any(|msg| matches!(
            msg,
            Message::Dock(DockEvent::TabSelected { panel, .. }) if *panel == 2
        )),
        "step 2: expected TabSelected for output (2), got: {msgs:?}"
    );
}

// ---------------------------------------------------------------------------
// Content verification
// ---------------------------------------------------------------------------

#[test]
fn active_content_is_rendered() {
    let session = two_tab_session();
    let mut ui = simulator(view_with_unique_content(&session));

    ui.find("Content:editor")
        .expect("active tab's content should be visible");
}

#[test]
fn inactive_content_is_not_rendered() {
    let session = two_tab_session();
    let mut ui = simulator(view_with_unique_content(&session));

    assert!(
        ui.find("Content:terminal").is_err(),
        "inactive tab's content should NOT be in the widget tree"
    );
}

// ---------------------------------------------------------------------------
// Multi-pane layout
// ---------------------------------------------------------------------------

#[test]
fn split_layout_renders_all_pane_tabs() {
    let session = split_session();
    let mut ui = simulator(view(&session));

    ui.find("Editor").expect("Editor tab visible");
    ui.find("Terminal").expect("Terminal tab visible");
    ui.find("Explorer").expect("Explorer tab visible");
    ui.find("Search").expect("Search tab visible");
}

#[test]
fn split_layout_click_tab_in_right_pane() {
    let session = split_session();
    let mut ui = simulator(view(&session));

    let _ = ui.click("Search");

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages.iter().any(|msg| matches!(
            msg,
            Message::Dock(DockEvent::TabSelected { panel, .. }) if *panel == 11
        )),
        "expected TabSelected for search (11), got: {messages:?}"
    );
}

// ---------------------------------------------------------------------------
// State mutation through shared Rc
// ---------------------------------------------------------------------------

#[test]
fn click_mutates_shared_session_state() {
    let session = two_tab_session();
    let mut ui = simulator(view(&session));

    let _ = ui.click("Terminal");
    let _ = ui.into_messages().count();

    let state = session.state();
    let state = state.borrow();
    let pane = state.focused_pane.expect("a pane should be focused");
    let active = state
        .layout
        .kind(pane)
        .and_then(|kind| match kind {
            iced_kit::dock::model::NodeKind::Pane(p) => p.active,
            _ => None,
        })
        .expect("pane should have an active tab");
    let active_id = state
        .index
        .panels
        .iter()
        .find_map(|(id, &node)| (node == active).then(|| id.clone()));
    assert_eq!(
        active_id.as_deref(),
        Some("terminal"),
        "shared state should reflect 'terminal' as active"
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn single_panel_layout_renders() {
    let session = DockSession::from_tree(single(panel("solo", "Solo", 0u32))).expect("valid");
    let mut ui = simulator(view_with_unique_content(&session));

    ui.find("Solo").expect("tab label should be visible");
    ui.find("Content:editor")
        .expect("content should be rendered");
}

#[test]
fn overflow_menu_selects_hidden_tab() {
    let session = overflow_session();
    let mut ui = narrow_view(&session);

    // The press that opens the panel menu lands where the bar crowds the
    // tabs, and the click that follows is meant for the hidden tab: a menu
    // that is open swallows the press that dismisses it, so the test closes
    // it on a neutral spot of the pane first — a press eaten by the menu must
    // not also press whatever sits under it.
    ui.point_at(Point::new(168.0, 15.0));
    let _ = ui.simulate(iced_test::simulator::click());

    ui.point_at(Point::new(90.0, 120.0));
    let _ = ui.simulate(iced_test::simulator::click());

    let _ = ui.click("File 5");
    let messages: Vec<_> = ui.into_messages().collect();

    assert!(
        messages.iter().any(|msg| matches!(
            msg,
            Message::Dock(DockEvent::TabSelected { panel, .. }) if *panel == 5
        )),
        "expected TabSelected for hidden tab file5 (5), got: {messages:?}"
    );

    let state = session.state();
    let state = state.borrow();
    let pane = state.focused_pane.expect("a pane should be focused");
    let active = state
        .layout
        .kind(pane)
        .and_then(|kind| match kind {
            iced_kit::dock::model::NodeKind::Pane(p) => p.active,
            _ => None,
        })
        .expect("pane should have an active tab");
    let active_id = state
        .index
        .panels
        .iter()
        .find_map(|(id, &node)| (node == active).then(|| id.clone()));
    assert_eq!(active_id.as_deref(), Some("file5"));
}

#[test]
fn panel_menu_flips_inward_selects_and_closes() {
    let session = overflow_session();
    let mut ui = narrow_view(&session);

    // The panel menu button sits at the right end of the bar, inside the
    // window's last 22 logical pixels: a menu anchored left of it would leave
    // the glass, so it must open inward for its rows to be reachable at all.
    ui.point_at(Point::new(168.0, 15.0));
    let _ = ui.simulate(iced_test::simulator::click());

    // The menu's first row is the zoom entry, flipped to start near the
    // window's left edge.
    ui.point_at(Point::new(99.0, 45.0));
    let _ = ui.simulate(iced_test::simulator::click());

    // A choice folds the menu, so pressing the button opens it again and the
    // same row answers a second time.
    ui.point_at(Point::new(168.0, 15.0));
    let _ = ui.simulate(iced_test::simulator::click());
    ui.point_at(Point::new(99.0, 45.0));
    let _ = ui.simulate(iced_test::simulator::click());

    let messages: Vec<_> = ui.into_messages().collect();
    let zooms = messages
        .iter()
        .filter(|msg| matches!(msg, Message::Dock(DockEvent::ZoomChanged { .. })))
        .count();
    assert_eq!(
        zooms, 2,
        "choosing the zoom row twice must zoom and restore, got: {messages:?}"
    );
}

#[test]
fn panel_menu_closes_on_a_press_outside() {
    let session = overflow_session();
    let mut ui = narrow_view(&session);

    ui.point_at(Point::new(168.0, 15.0));
    let _ = ui.simulate(iced_test::simulator::click());

    // A press on the pane content is swallowed by the open menu — it dismisses
    // without pressing what sat under the point.
    ui.point_at(Point::new(90.0, 120.0));
    let _ = ui.simulate(iced_test::simulator::click());

    // The menu is gone, so the button opens it again and the zoom row answers.
    ui.point_at(Point::new(168.0, 15.0));
    let _ = ui.simulate(iced_test::simulator::click());
    ui.point_at(Point::new(99.0, 45.0));
    let _ = ui.simulate(iced_test::simulator::click());

    let messages: Vec<_> = ui.into_messages().collect();
    let zooms = messages
        .iter()
        .filter(|msg| matches!(msg, Message::Dock(DockEvent::ZoomChanged { .. })))
        .count();
    assert_eq!(
        zooms, 1,
        "exactly one zoom must follow the dismissal and re-open, got: {messages:?}"
    );
}
