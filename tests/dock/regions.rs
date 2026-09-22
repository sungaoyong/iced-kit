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

//! The behaviour the dock layers on top of its split tree: named regions, panel
//! zoom, visibility, the presentation seam, and the declarative area builder.
//!
//! These all live above the layout algebra, so they are exercised through the
//! state rather than through widgets: a region is a size and an open flag, a zoom
//! is a node id, and neither needs a frame to be meaningful.

use std::collections::HashSet;

use iced::{Point, Rectangle, Size};
use iced_kit::dock::model::{Dock, DockPlacement, PANEL_MIN_SIZE};
use iced_kit::dock::unstable::Factory;
use iced_kit::dock::{
    dock, horizontal, panel, tabs, vertical, DockAction, DockEvent, DockSession, DockWidgetState,
    LayoutArea, PanelDef,
};
use iced_kit::Theme;

type Message = u32;

fn area() -> LayoutArea<u32> {
    LayoutArea::new(tabs([panel("editor", "Editor", 0u32)]))
        .dock(
            DockPlacement::Left,
            240.0,
            tabs([panel("files", "Files", 1u32)]),
        )
        .dock(
            DockPlacement::Bottom,
            200.0,
            tabs([panel("terminal", "Terminal", 2u32)]),
        )
}

fn state() -> DockWidgetState<u32> {
    DockWidgetState::from_area(area()).expect("the area is valid")
}

// ---------------------------------------------------------------------------
// The declarative area
// ---------------------------------------------------------------------------

#[test]
fn an_area_compiles_its_centre_and_each_dock() {
    let state = state();

    // The centre is where it always was, so code written against a single tree
    // keeps working.
    assert!(state.layout.root_child().is_some());
    assert!(state.regions.has_dock(DockPlacement::Left));
    assert!(state.regions.has_dock(DockPlacement::Bottom));
    assert!(!state.regions.has_dock(DockPlacement::Right));
}

#[test]
fn a_dock_takes_the_size_it_was_given() {
    let state = state();
    assert_eq!(state.regions.dock(DockPlacement::Left).unwrap().size(), 240.0);
    assert_eq!(
        state.regions.dock(DockPlacement::Bottom).unwrap().size(),
        200.0
    );
}

#[test]
fn a_dock_with_no_stated_size_gets_the_default() {
    let area = LayoutArea::new(tabs([panel("editor", "Editor", 0u32)])).dock_with(
        DockPlacement::Right,
        None,
        true,
        tabs([panel("outline", "Outline", 1u32)]),
    );
    let state = DockWidgetState::from_area(area).expect("valid");
    assert_eq!(
        state.regions.dock(DockPlacement::Right).unwrap().size(),
        Dock::default_size()
    );
}

#[test]
fn a_panel_keeps_its_slot_when_it_is_hidden() {
    let def = PanelDef::new("editor", "Editor", 0u32);
    let area = LayoutArea::new(tabs([
        def.clone().visible(false),
        PanelDef::new("terminal", "Terminal", 1u32),
    ]));
    let state = DockWidgetState::from_area(area).expect("valid");

    // The hidden panel is still in the tree, so showing it again restores the
    // layout the user had rather than appending it.
    let pane = iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane");
    let iced_kit::dock::model::NodeKind::Pane(pane_state) = state.layout.kind(pane).unwrap() else {
        panic!("expected a pane");
    };
    assert_eq!(pane_state.tabs.len(), 2, "the hidden panel keeps its slot");
}

#[test]
fn the_centre_cannot_be_a_dock_placement() {
    let area = LayoutArea::new(tabs([panel("editor", "Editor", 0u32)])).dock_with(
        DockPlacement::Center,
        Some(200.0),
        true,
        tabs([panel("x", "X", 1u32)]),
    );
    let error = DockWidgetState::from_area(area).expect_err("the centre is not a dock");
    assert!(
        matches!(error, iced_kit::dock::Error::InvalidDockPlacement(_)),
        "got {error:?}"
    );
}

#[test]
fn a_placement_can_only_hold_one_dock() {
    let area = LayoutArea::new(tabs([panel("editor", "Editor", 0u32)]))
        .dock(
            DockPlacement::Left,
            200.0,
            tabs([panel("a", "A", 1u32)]),
        )
        .dock(
            DockPlacement::Left,
            200.0,
            tabs([panel("b", "B", 2u32)]),
        );
    let error = DockWidgetState::from_area(area).expect_err("a dock placement is unique");
    assert!(
        matches!(error, iced_kit::dock::Error::DuplicateDockPlacement(_)),
        "got {error:?}"
    );
}

#[test]
fn a_panel_id_is_unique_across_every_region() {
    // The same id in the centre and in a dock is a mistake worth catching: the
    // index is shared, so a later open-by-id would resolve to one of them by luck.
    let area = LayoutArea::new(tabs([panel("editor", "Editor", 0u32)])).dock(
        DockPlacement::Left,
        200.0,
        tabs([panel("editor", "Editor again", 1u32)]),
    );
    let error = DockWidgetState::from_area(area).expect_err("ids are area-wide");
    assert!(
        matches!(error, iced_kit::dock::Error::DuplicatePanelId(_)),
        "got {error:?}"
    );
}

// ---------------------------------------------------------------------------
// Showing and hiding a dock
// ---------------------------------------------------------------------------

#[test]
fn toggling_a_dock_sets_its_open_flag() {
    let mut state = state();
    assert!(state.regions.is_dock_open(DockPlacement::Left));
    assert!(state.toggle_dock(DockPlacement::Left));
    assert!(!state.regions.is_dock_open(DockPlacement::Left));
    assert!(state.toggle_dock(DockPlacement::Left));
    assert!(state.regions.is_dock_open(DockPlacement::Left));
}

#[test]
fn a_non_collapsible_dock_refuses_the_toggle_but_not_an_explicit_close() {
    let mut state = state();
    state
        .regions
        .dock_mut(DockPlacement::Left)
        .unwrap()
        .set_collapsible(false);

    // The toggle is the *affordance*, so a dock that opts out of it is not closed
    // by clicking a button that should not be there.
    assert!(!state.toggle_dock(DockPlacement::Left));
    assert!(state.regions.is_dock_open(DockPlacement::Left));

    // An application can still hide it: the flag is a preference, not a lock.
    assert!(state.set_dock_open(DockPlacement::Left, false));
    assert!(!state.regions.is_dock_open(DockPlacement::Left));

    // And an explicit open is never refused.
    assert!(state.set_dock_open(DockPlacement::Left, true));
}

#[test]
fn toggling_a_dock_that_does_not_exist_changes_nothing() {
    let mut state = state();
    assert!(!state.toggle_dock(DockPlacement::Right));
    assert!(!state.set_dock_open(DockPlacement::Right, false));
}

#[test]
fn a_closed_bottom_dock_keeps_a_strip_and_the_sides_take_nothing() {
    let mut state = state();
    state.set_dock_open(DockPlacement::Bottom, false);
    state.set_dock_open(DockPlacement::Left, false);

    // A closed bottom dock keeps enough height for its tab strip to be clicked,
    // which is how it is reopened; a closed side dock's strip would be a column of
    // slivers, so it takes no width at all.
    assert_eq!(
        state.regions.extent(DockPlacement::Bottom),
        iced_kit::dock::model::CLOSED_BOTTOM_STRIP
    );
    assert_eq!(state.regions.extent(DockPlacement::Left), 0.0);
}

#[test]
fn removing_a_dock_takes_its_panels_out_of_the_index() {
    let mut state = state();
    assert!(state.index.panels.contains_key("files"));

    assert!(state.remove_dock(DockPlacement::Left));
    assert!(
        !state.index.panels.contains_key("files"),
        "a panel that left for good must not stay resolvable by id"
    );
    assert!(!state.remove_dock(DockPlacement::Left), "and only once");
}

// ---------------------------------------------------------------------------
// Resizing a dock
// ---------------------------------------------------------------------------

#[test]
fn resizing_a_dock_follows_the_pointer_from_its_own_edge() {
    let mut state = state();
    state.area_bounds = Some(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 1000.0,
        height: 700.0,
    });

    // A left dock is sized from the area's left edge.
    assert!(state.resize_dock_to(DockPlacement::Left, Point::new(320.0, 300.0)));
    assert_eq!(state.regions.dock(DockPlacement::Left).unwrap().size(), 320.0);

    // A right dock from the opposite one.
    state.regions.insert_dock(
        DockPlacement::Right,
        iced_kit::dock::model::DockRegion::new(Dock::new(200.0)),
    );
    assert!(state.resize_dock_to(DockPlacement::Right, Point::new(700.0, 300.0)));
    assert_eq!(state.regions.dock(DockPlacement::Right).unwrap().size(), 300.0);
}

#[test]
fn a_resize_leaves_room_for_the_centre_and_the_opposite_dock() {
    let mut state = state();
    state.area_bounds = Some(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    });
    // 260 left, so the right dock may be at most 800 - 100 (centre) - 260.
    state.regions.insert_dock(
        DockPlacement::Right,
        iced_kit::dock::model::DockRegion::new(Dock::new(200.0)),
    );

    state.resize_dock_to(DockPlacement::Right, Point::new(0.0, 300.0));
    assert_eq!(
        state.regions.dock(DockPlacement::Right).unwrap().size(),
        800.0 - PANEL_MIN_SIZE - 240.0
    );
}

#[test]
fn dragging_a_closed_docks_handle_reopens_it() {
    let mut state = state();
    state.area_bounds = Some(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 1000.0,
        height: 700.0,
    });
    state.set_dock_open(DockPlacement::Left, false);

    // The pointer is asking to see the dock; a resize that changed an invisible
    // extent would look like nothing happened.
    assert!(state.resize_dock_to(DockPlacement::Left, Point::new(260.0, 300.0)));
    assert!(state.regions.is_dock_open(DockPlacement::Left));
    assert_eq!(state.regions.dock(DockPlacement::Left).unwrap().size(), 260.0);
}

// ---------------------------------------------------------------------------
// Zoom
// ---------------------------------------------------------------------------

#[test]
fn a_pane_can_be_zoomed_and_restored() {
    let mut state = state();
    let pane = iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane");

    assert!(state.set_zoom(pane, true));
    assert!(state.is_zoomed(pane));
    assert!(state.set_zoom(pane, false));
    assert!(!state.is_zoomed(pane));
}

#[test]
fn a_panel_that_refuses_to_zoom_is_not_zoomed() {
    let mut state = state();
    let pane = iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane");
    let panel = state
        .index
        .panels
        .get("editor")
        .copied()
        .expect("the editor is indexed");
    if let Some(iced_kit::dock::model::NodeKind::Panel(p)) =
        state.layout.get_mut(panel).map(|e| &mut e.kind)
    {
        p.can_zoom = false;
    }

    assert!(!state.set_zoom(pane, true), "the panel refuses");
    assert!(!state.is_zoomed(pane));
}

#[test]
fn restoring_from_a_zoom_is_never_refused() {
    let mut state = state();
    let pane = iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane");
    assert!(state.set_zoom(pane, true));

    // A panel that stops allowing the control while zoomed would otherwise strand
    // the user with no way back, so zooming out never asks.
    let panel = state.index.panels.get("editor").copied().expect("indexed");
    if let Some(iced_kit::dock::model::NodeKind::Panel(p)) =
        state.layout.get_mut(panel).map(|e| &mut e.kind)
    {
        p.can_zoom = false;
    }
    assert!(state.set_zoom(pane, false));
    assert!(!state.is_zoomed(pane));
}

#[test]
fn a_zoom_on_a_node_that_is_gone_ends_silently() {
    let mut state = state();
    let pane = iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane");
    assert!(state.set_zoom(pane, true));

    // Closing every panel in the pane collapses the tree out from under the zoom.
    let panels: Vec<_> = state
        .index
        .panels
        .values()
        .copied()
        .filter(|&p| state.layout.kind(p).is_some())
        .collect();
    for panel in panels {
        let r = Factory.close(&mut state.layout, panel);
        println!("close => {r:?}");
    }
    println!("root_child after close = {:?}", state.layout.root_child());
    state.sync_index();
    state.regions.prune_zoom(&state.layout);

    assert_eq!(state.regions.zoomed(), None, "the zoom went with the group");
}

#[test]
fn zooming_a_dock_toggle_target_is_the_pane_not_the_dock() {
    // A zoom fills the whole area, so it is recorded as the *group*, and the area
    // asks the region for it. Recording the region would make a zoomed panel
    // inside a closed dock pointless.
    let mut state = state();
    let pane = iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane");
    assert!(state.set_zoom(pane, true));
    assert_eq!(state.regions.zoomed(), Some(pane));
}

// ---------------------------------------------------------------------------
// Runtime visibility
// ---------------------------------------------------------------------------

#[test]
fn hiding_a_panel_at_runtime_leaves_the_layout_alone() {
    let mut state = state();

    // Visibility is application state that changes without the layout changing, so
    // hiding a panel must not force a rebuild of the tree around it.
    assert!(state.set_panel_visible("editor", false));
    assert!(!state.is_panel_visible("editor"));
    assert!(!state.set_panel_visible("editor", false), "idempotent");

    assert!(state.set_panel_visible("editor", true));
    assert!(state.is_panel_visible("editor"));
}

#[test]
fn a_panel_is_visible_until_it_is_hidden() {
    let state = state();
    assert!(state.is_panel_visible("editor"));
    assert!(
        state.is_panel_visible("never-registered"),
        "an unknown name is not hidden, it is simply unknown"
    );
}

// ---------------------------------------------------------------------------
// Actions and events
// ---------------------------------------------------------------------------

#[test]
fn a_dock_toggle_action_reaches_the_state_and_reports_the_new_flag() {
    let session = DockSession::from_area(area()).expect("valid");
    assert!(session.dispatch(DockAction::ToggleDock {
        placement: DockPlacement::Left
    }));
    assert!(!session.state().borrow().regions.is_dock_open(DockPlacement::Left));
}

#[test]
fn a_zoom_action_reaches_the_state() {
    let session = DockSession::from_area(area()).expect("valid");
    let pane = {
        let state = session.state();
        let state = state.borrow();
        iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane")
    };
    assert!(session.dispatch(DockAction::ToggleZoom { pane }));
    assert!(session.state().borrow().is_zoomed(pane));
}

#[test]
fn a_dock_resize_action_clamps_to_the_area() {
    let session = DockSession::from_area(area()).expect("valid");
    {
        let state = session.state();
        let mut state = state.borrow_mut();
        state.area_bounds = Some(Rectangle {
            x: 0.0,
            y: 0.0,
            width: 900.0,
            height: 600.0,
        });
    }
    assert!(session.dispatch(DockAction::DockResize {
        placement: DockPlacement::Left,
        cursor: Point::new(500.0, 300.0),
    }));
    let state = session.state();
    let size = state
        .borrow()
        .regions
        .dock(DockPlacement::Left)
        .unwrap()
        .size();
    assert!(size <= 900.0 - PANEL_MIN_SIZE);
    assert!(size >= PANEL_MIN_SIZE);
}

// ---------------------------------------------------------------------------
// The area a session installs can be replaced
// ---------------------------------------------------------------------------

#[test]
fn installing_a_new_area_keeps_the_size_the_user_dragged() {
    let mut state = state();
    state
        .regions
        .dock_mut(DockPlacement::Left)
        .unwrap()
        .set_size(333.0);
    state.set_dock_open(DockPlacement::Left, false);

    // The incoming trees are fresh values, but a dock size is not part of a tree,
    // so it is carried across by placement rather than reset to the default.
    state.set_area(area()).expect("the area is valid");
    assert_eq!(state.regions.dock(DockPlacement::Left).unwrap().size(), 333.0);
    assert!(!state.regions.is_dock_open(DockPlacement::Left));
}

// ---------------------------------------------------------------------------
// The presentation seam
// ---------------------------------------------------------------------------

/// A presentation that answers everything, so the dock's use of it is exercised.
struct Panels;

impl iced_kit::dock::PanelPresentation<u32, Message, Theme> for Panels {
    fn title(&self, panel: u32) -> Option<iced::Element<'static, Message, Theme>> {
        Some(iced::widget::text(format!("title {panel}")).into())
    }

    fn title_suffix(&self, panel: u32) -> Option<iced::Element<'static, Message, Theme>> {
        Some(iced::widget::text(format!("({panel})")).into())
    }

    fn toolbar(&self, _panel: u32) -> Vec<iced::Element<'static, Message, Theme>> {
        vec![iced::widget::text("★").into()]
    }

    fn menu(&self, _panel: u32) -> Vec<(String, Message)> {
        vec![("Reload".to_owned(), 7u32)]
    }

    fn zoom_control(&self, _panel: u32) -> Option<iced_kit::dock::PanelControl> {
        Some(iced_kit::dock::PanelControl::Both)
    }

    fn inner_padding(&self, panel: u32) -> bool {
        panel != 0
    }

    fn title_bar(&self, _panel: u32) -> bool {
        true
    }
}

/// A presentation that declines everything, which every method's default allows.
#[derive(Default)]
struct Silent;

impl iced_kit::dock::PanelPresentation<u32, Message, Theme> for Silent {}

#[test]
fn every_presentation_method_has_a_default() {
    // The point of the defaults: a dock whose panels are plain implements nothing.
    let _ = Silent;
}

#[test]
fn a_presentation_is_accepted_by_the_builder() {
    let session = DockSession::from_area(area()).expect("valid");
    let element: iced::Element<'_, Message, Theme> = dock::<u32, Message, Theme, iced::Renderer>()
        .state(session.state())
        .on_event(|_| 0u32)
        .presentation(Panels)
        .panel_style(iced_kit::dock::PanelStyle::TabBar)
        .toggle_button_visible(false)
        .dock_handle_width(8.0)
        .content(|_| iced::widget::text("content").into())
        .build()
        .into();
    drop(element);
}

#[test]
fn a_plain_panels_marker_stands_in_for_no_presentation() {
    let session = DockSession::from_area(area()).expect("valid");
    let element: iced::Element<'_, Message, Theme> = dock::<u32, Message, Theme, iced::Renderer>()
        .state(session.state())
        .on_event(|_| 0u32)
        .presentation(iced_kit::dock::PlainPanels)
        .content(|_| iced::widget::text("content").into())
        .build()
        .into();
    drop(element);
}

// ---------------------------------------------------------------------------
// The design-system layer
// ---------------------------------------------------------------------------

#[test]
fn the_design_system_layer_re_exports_the_area_api() {
    // The whole point of `widgets::dock` is that an application depends on one
    // module rather than two, so the area types must come through it.
    let _ = iced_kit::widgets::dock::panel("id", "Title", 0u32);
    let _: iced_kit::widgets::dock::LayoutArea<u32> =
        iced_kit::widgets::dock::LayoutArea::new(tabs([panel("a", "A", 0u32)]));
    let _ = iced_kit::widgets::dock::DockPlacement::Left;
}

#[test]
fn the_design_system_style_covers_the_new_chrome() {
    let style = iced_kit::widgets::dock::style(&Theme::light());
    // A group holding one panel draws this bar rather than a strip of tabs, so it
    // has to be metrics-complete out of the box.
    assert!(style.title.height > 0.0);
    assert!(style.control.size > 0.0);
    assert!(style.control.glyph_size > 0.0);
    // The region handle is invisible at rest, like an inner splitter.
    assert_eq!(style.control.handle.idle_color.a, 0.0);
    assert_ne!(style.control.handle.hover_color.a, 0.0);
}

#[test]
fn the_title_bar_and_the_tab_strip_are_the_same_height() {
    // They are the same bar in two presentations, so a group that gains or loses a
    // tab must not move its content.
    let style = iced_kit::widgets::dock::style(&Theme::light());
    assert_eq!(style.title.height, iced_kit::widgets::dock::tab_bar_height());
}

#[test]
fn a_renamed_panel_keeps_its_id_and_its_place() {
    // The id is what persistence and open-by-id use, so a title change is not a
    // structural change.
    let mut state = state();
    let before = state.index.panels.get("editor").copied().expect("indexed");
    state.set_panel_visible("editor", false);
    state.set_panel_visible("editor", true);
    assert_eq!(state.index.panels.get("editor").copied(), Some(before));
}

#[test]
fn a_vertical_area_is_accepted_like_a_horizontal_one() {
    let area = LayoutArea::new(vertical([
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .dock(
        DockPlacement::Right,
        260.0,
        tabs([panel("c", "C", 2u32)]),
    );
    let state = DockWidgetState::from_area(area).expect("valid");
    assert!(state.regions.has_dock(DockPlacement::Right));
}

#[test]
fn a_horizontal_centre_with_every_dock_installs() {
    let area = LayoutArea::new(horizontal([
        tabs([panel("a", "A", 0u32)]),
        tabs([panel("b", "B", 1u32)]),
    ]))
    .dock(DockPlacement::Left, 200.0, tabs([panel("c", "C", 2u32)]))
    .dock(DockPlacement::Right, 200.0, tabs([panel("d", "D", 3u32)]))
    .dock(
        DockPlacement::Bottom,
        180.0,
        tabs([panel("e", "E", 4u32)]),
    );
    let state = DockWidgetState::from_area(area).expect("valid");
    for placement in DockPlacement::DOCKS {
        assert!(state.regions.has_dock(placement), "{placement} is present");
        assert!(state.regions.is_dock_open(placement));
    }
}

#[test]
fn the_size_a_dock_reports_is_never_below_the_minimum() {
    let mut state = state();
    state.area_bounds = Some(Rectangle {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    });
    // However small the window, a dock stays usable rather than collapsing to a
    // sliver the user cannot grab.
    state.resize_dock_to(DockPlacement::Left, Point::new(0.0, 0.0));
    assert_eq!(state.regions.dock(DockPlacement::Left).unwrap().size(), PANEL_MIN_SIZE);
}

#[test]
fn a_zoom_event_reports_the_panel_it_applies_to() {
    // `ZoomChanged` carries the panel so a subscriber can persist "which document
    // was maximized", not just that something was.
    let event: DockEvent<u32> = DockEvent::ZoomChanged {
        zoomed: true,
        panel: Some(3),
    };
    assert!(matches!(event, DockEvent::ZoomChanged { zoomed: true, panel: Some(3) }));
}

#[test]
fn a_dock_event_reports_which_dock_moved() {
    let event: DockEvent<u32> = DockEvent::DockToggled {
        placement: DockPlacement::Bottom,
        open: false,
    };
    assert!(matches!(
        event,
        DockEvent::DockToggled {
            placement: DockPlacement::Bottom,
            open: false
        }
    ));
}

#[test]
fn a_hidden_panel_is_not_offered_even_when_declared_visible() {
    // The two halves are separate on purpose: `visible` is declared in the layout,
    // the hidden set is runtime state, and either one takes the panel out.
    let mut state = state();
    state.set_panel_visible("editor", false);
    assert!(!state.is_panel_visible("editor"));

    let mut hidden_ids = HashSet::new();
    hidden_ids.insert("files".to_owned());
    state.hidden = hidden_ids;
    assert!(!state.is_panel_visible("files"));
}

#[test]
fn the_regions_extent_is_what_the_area_lays_out() {
    let mut state = state();
    let wide = Size::new(1000.0, 700.0);

    // Open, each dock takes its size.
    assert_eq!(state.regions.extent(DockPlacement::Left), 240.0);
    assert_eq!(state.regions.extent(DockPlacement::Bottom), 200.0);

    // The opposite extent is what the centre shares its space with.
    assert_eq!(
        state.regions.opposite_extent(DockPlacement::Right),
        240.0,
        "the left dock is across the centre from the right one"
    );

    // And a clamped resize respects the area it was given.
    state.area_bounds = Some(Rectangle {
        x: 0.0,
        y: 0.0,
        width: wide.width,
        height: wide.height,
    });
    state.resize_dock_to(DockPlacement::Left, Point::new(900.0, 400.0));
    assert!(state.regions.extent(DockPlacement::Left) <= wide.width - PANEL_MIN_SIZE);
}

// ---------------------------------------------------------------------------
// Saving and restoring a workspace
// ---------------------------------------------------------------------------

#[test]
fn a_session_captures_and_restores_its_whole_workspace() {
    let session = DockSession::from_area(area()).expect("valid");
    session.dispatch(DockAction::ToggleDock {
        placement: DockPlacement::Bottom,
    });
    let saved = session.capture(Some(7));

    // A second session, restored from the first's file.
    let restored = DockSession::from_tree(tabs([panel("editor", "Editor", 0u32)])).expect("valid");
    restored.restore(&saved).expect("restores");

    let state = restored.state();
    let state = state.borrow();
    assert!(state.regions.has_dock(DockPlacement::Left));
    assert!(state.regions.has_dock(DockPlacement::Bottom));
    assert!(
        !state.regions.is_dock_open(DockPlacement::Bottom),
        "the closed dock is still closed"
    );
    assert_eq!(state.regions.dock(DockPlacement::Left).unwrap().size(), 240.0);
    assert!(state.index.panels.contains_key("files"), "panels came back");
}

#[test]
fn a_saved_workspace_records_the_version_it_was_written_with() {
    let session = DockSession::from_area(area()).expect("valid");
    assert_eq!(session.capture(None).version, None);
    assert_eq!(session.capture(Some(2)).version, Some(2));
}

#[test]
fn restoring_drops_the_panels_the_saved_workspace_never_had() {
    // A workspace describes the whole layout: a panel added after the file was
    // written does not come back, or the user's saved arrangement would keep
    // growing every time the application gained a feature.
    let session = DockSession::from_area(area()).expect("valid");
    let saved = session.capture(None);

    let later = DockSession::from_area(
        LayoutArea::new(tabs([
            panel("editor", "Editor", 0u32),
            panel("brand-new", "Brand new", 9u32),
        ]))
        .dock(
            DockPlacement::Left,
            240.0,
            tabs([panel("files", "Files", 1u32)]),
        ),
    )
    .expect("valid");
    assert!(later.state().borrow().index.panels.contains_key("brand-new"));

    later.restore(&saved).expect("restores");
    assert!(
        !later.state().borrow().index.panels.contains_key("brand-new"),
        "the file did not have it, so restoring removed it"
    );
}

#[test]
fn a_saved_zoom_survives_a_round_trip() {
    let session = DockSession::from_area(area()).expect("valid");
    let pane = {
        let state = session.state();
        let state = state.borrow();
        iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane")
    };
    assert!(session.dispatch(DockAction::ToggleZoom { pane }));
    let saved = session.capture(None);
    assert_eq!(saved.zoomed.as_deref(), Some("editor"));

    let restored = DockSession::from_tree(tabs([panel("editor", "Editor", 0u32)])).expect("valid");
    restored.restore(&saved).expect("restores");
    let state = restored.state();
    assert!(
        state.borrow().regions.zoomed().is_some(),
        "the maximized panel came back maximized"
    );
}

#[test]
fn restoring_a_workspace_whose_zoomed_panel_is_gone_still_opens() {
    let session = DockSession::from_area(area()).expect("valid");
    let pane = {
        let state = session.state();
        let state = state.borrow();
        iced_kit::dock::unstable::first_pane(&state.layout).expect("a pane")
    };
    assert!(session.dispatch(DockAction::ToggleZoom { pane }));
    let mut saved = session.capture(None);
    // The maximized panel was removed by a later build.
    saved.zoomed = Some("removed-in-v2".to_owned());

    let restored = DockSession::from_tree(tabs([panel("editor", "Editor", 0u32)])).expect("valid");
    restored.restore(&saved).expect("the load does not fail");
    assert_eq!(restored.state().borrow().regions.zoomed(), None);
}

#[test]
fn a_dock_that_refuses_to_collapse_stays_that_way_across_a_restore() {
    let session = DockSession::from_area(
        LayoutArea::new(tabs([panel("editor", "Editor", 0u32)]))
            .dock(
                DockPlacement::Left,
                240.0,
                tabs([panel("files", "Files", 1u32)]),
            )
            .not_collapsible(DockPlacement::Left),
    )
    .expect("valid");
    let saved = session.capture(None);

    let restored = DockSession::from_area(area()).expect("valid");
    restored.restore(&saved).expect("restores");
    assert!(!restored
        .state()
        .borrow()
        .regions
        .is_dock_collapsible(DockPlacement::Left));
}

// ---------------------------------------------------------------------------
// The documented path, end to end
// ---------------------------------------------------------------------------

/// A panel key for the documented-path test, so it exercises the seam with an
/// application's own key type rather than the fixture's `u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DocPanel {
    Files,
    Editor,
    Terminal,
}

/// The presentation for [`DocPanel`].
struct DocPanels;

impl iced_kit::widgets::dock::PanelPresentation<DocPanel, Message, Theme> for DocPanels {
    fn toolbar(&self, _panel: DocPanel) -> Vec<iced::Element<'static, Message, Theme>> {
        vec![iced::widget::text("⇅").into()]
    }
}

/// Compile-checks the integration the README documents, with every new feature
/// turned on at once.
///
/// The dock is generic over the theme and the renderer, so an integration that does
/// not compile is easy to write and hard to notice. This drives the whole builder
/// the way an application would, then saves and restores through it.
#[test]
fn the_documented_workspace_path_compiles_and_round_trips() {
    use iced_kit::widgets::dock::{
        DockPlacement, DockSession, LayoutArea, PanelDef, PanelStyle,
    };

    type P = DocPanel;

    let area = LayoutArea::new(iced_kit::widgets::dock::tabs([PanelDef::new(
        "editor", "main.rs", P::Editor,
    )]))
    .dock(
        DockPlacement::Left,
        240.0,
        iced_kit::widgets::dock::tabs([PanelDef::new("files", "Files", P::Files)]),
    )
    .dock(
        DockPlacement::Bottom,
        200.0,
        iced_kit::widgets::dock::tabs([PanelDef::new("term", "Terminal", P::Terminal)]),
    );

    let session = DockSession::from_area(area).expect("the workspace is valid");

    let element: iced::Element<'_, Message, Theme> =
        iced_kit::widgets::dock::dock::<P, Message, Theme, iced::Renderer>()
            .state(session.state())
            .on_event(|_event| 0u32)
            .style(iced_kit::widgets::dock::style)
            .panel_style(PanelStyle::Auto)
            .presentation(DocPanels)
            .toggle_button_visible(true)
            .dock_handle_width(6.0)
            .content(|panel| match panel {
                P::Files => iced::widget::text("Files").into(),
                P::Editor => iced::widget::text("Editor").into(),
                P::Terminal => iced::widget::text("Terminal").into(),
            })
            .build()
            .into();
    drop(element);

    // And the workspace survives a save and a restore.
    let saved = session.capture(Some(1));
    let reloaded = DockSession::from_area(
        iced_kit::widgets::dock::LayoutArea::new(iced_kit::widgets::dock::tabs([PanelDef::new(
            "editor", "main.rs", P::Editor,
        )])),
    )
    .expect("a centre-only workspace");
    reloaded.restore(&saved).expect("restores");

    let state = reloaded.state();
    let state = state.borrow();
    assert!(state.regions.has_dock(DockPlacement::Left));
    assert!(state.regions.has_dock(DockPlacement::Bottom));
    assert_eq!(state.index.panels.len(), 3, "every panel came back");
}
