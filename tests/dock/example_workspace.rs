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

//! The `dock` example, driven as a user would drive it.
//!
//! The other drag and region tests build the smallest layout that shows a feature.
//! This one builds the *example's* workspace — a centre split plus all three edge
//! docks, with a presentation supplying titles and toolbars, and the crate's
//! recommended metrics — because the difference between that and a two-group centre
//! is exactly where the bugs were: a title bar instead of a strip, three regions
//! instead of one, and an application that tracks its own visibility.

use iced::widget::{column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Point, Size};
use iced_kit::dock::model::NodeKind;
use iced_kit::dock::{
    horizontal, tabs, DockEvent, DockPlacement, DockSession, LayoutArea, PanelControl, PanelDef,
    PanelPresentation, PanelStyle,
};
use iced_kit::widgets::dock as dock_kit;
use iced_kit::Theme;
use iced_test::Simulator;

/// The example's panel keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Panel {
    Files,
    Search,
    Editor,
    Preview,
    Terminal,
    Problems,
}

impl Panel {
    /// The string id this panel is registered under, which is what the session
    /// indexes it by.
    #[allow(dead_code)]
    fn id(self) -> &'static str {
        match self {
            Self::Files => "files",
            Self::Search => "search",
            Self::Editor => "editor",
            Self::Preview => "preview",
            Self::Terminal => "terminal",
            Self::Problems => "problems",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Files => "Explorer",
            Self::Search => "Search",
            Self::Editor => "main.rs",
            Self::Preview => "Preview",
            Self::Terminal => "Terminal",
            Self::Problems => "Problems",
        }
    }

    /// The body's marker text, which the tests find to locate a pane on screen.
    fn marker(self) -> &'static str {
        match self {
            Self::Files => "BODY FILES",
            Self::Search => "BODY SEARCH",
            Self::Editor => "BODY EDITOR",
            Self::Preview => "BODY PREVIEW",
            Self::Terminal => "BODY TERMINAL",
            Self::Problems => "BODY PROBLEMS",
        }
    }
}

/// Only `Dock` is ever produced here: the tests drive the session directly for
/// visibility, so the button's own message is never dispatched.
#[derive(Debug, Clone)]
enum Message {
    #[allow(dead_code)]
    Dock(DockEvent<Panel>),
    #[allow(dead_code)]
    TogglePanelVisible(Panel),
}

/// The example's workspace.
fn workspace() -> LayoutArea<Panel> {
    LayoutArea::new(horizontal([
        tabs([PanelDef::new("editor", "main.rs", Panel::Editor)]),
        tabs([PanelDef::new("preview", "Preview", Panel::Preview)]),
    ]))
    .dock(
        DockPlacement::Left,
        240.0,
        tabs([
            PanelDef::new("files", "Explorer", Panel::Files),
            PanelDef::new("search", "Search", Panel::Search),
        ])
        .active("files"),
    )
    .dock(
        DockPlacement::Right,
        250.0,
        tabs([PanelDef::new("problems", "Problems", Panel::Problems)]),
    )
    .dock(
        DockPlacement::Bottom,
        180.0,
        tabs([PanelDef::new("terminal", "Terminal", Panel::Terminal)]),
    )
}

/// The example's presentation: a title row with an icon button in it, a toolbar,
/// and a menu.
struct Panels {
    hidden: Vec<Panel>,
}

impl PanelPresentation<Panel, Message, Theme> for Panels {
    fn title(&self, panel: Panel) -> Option<Element<'static, Message, Theme>> {
        // Deliberately a *row containing a button*, as the example's is: a title that
        // is more than a bare `text` is the case a drag source has to survive.
        Some(
            row![
                iced_kit::widgets::icon_button::<Message>()
                    .icon("▤")
                    .ghost()
                    .compact(),
                text(panel.title()).size(13),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
        )
    }

    fn toolbar(&self, panel: Panel) -> Vec<Element<'static, Message, Theme>> {
        let hidden = self.hidden.contains(&panel);
        vec![iced_kit::widgets::icon_button::<Message>()
            .icon(if hidden { "◌" } else { "◉" })
            .ghost()
            .compact()
            .on_press(Message::TogglePanelVisible(panel))
            .into()]
    }

    fn menu(&self, panel: Panel) -> Vec<iced_kit::widgets::dock::MenuEntry<Message>> {
        // Deliberately longer than the button that opens them and carrying a
        // hint, which is the case a fixed-width menu wrapped.
        vec![
            iced_kit::widgets::dock::MenuEntry::new(
                format!("Copy {} path", panel.title()),
                Message::TogglePanelVisible(Panel::Files),
            )
            .shortcut("Ctrl+C"),
            iced_kit::widgets::dock::MenuEntry::new(
                format!("Close {} path", panel.title()),
                Message::TogglePanelVisible(Panel::Files),
            ),
            iced_kit::widgets::dock::MenuEntry::new(
                "Reload",
                Message::TogglePanelVisible(Panel::Files),
            ),
        ]
    }

    fn zoom_control(&self, _panel: Panel) -> Option<PanelControl> {
        Some(PanelControl::Both)
    }

    fn inner_padding(&self, _panel: Panel) -> bool {
        false
    }
}

fn view<'a>(session: &'a DockSession<Panel>, hidden: &'a [Panel]) -> Element<'a, Message, Theme> {
    let presentation = Panels {
        hidden: hidden.to_vec(),
    };
    let builder = dock_kit::dock::<Panel, Message, Theme, iced::Renderer>()
        .state(session.state())
        .on_event(Message::Dock)
        .style(iced_kit::widgets::dock::style)
        .panel_style(PanelStyle::Auto)
        .presentation(presentation);

    container(
        dock_kit::apply_metrics(builder)
            .content(|panel| {
                scrollable(column![text(panel.marker())].spacing(2))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            })
            .build(),
    )
    .padding(0)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

const WINDOW: Size = Size::new(1180.0, 760.0);

/// The centre of whatever draws `label`.
fn centre_of(ui: &mut Simulator<'_, Message, Theme>, label: &str) -> Point {
    let bounds = ui
        .find(label)
        .unwrap_or_else(|_| panic!("`{label}` should be on screen"))
        .bounds();
    Point::new(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// The gesture a user makes: point, press, move, move, release.
///
/// A frame is drawn first, because the drop geometry is recorded while drawing: a
/// gesture that begins before any frame has nothing to resolve against, and a real
/// window has always drawn the layout the user is dragging over.
fn drag(ui: &mut Simulator<'_, Message, Theme>, from: Point, to: Point) {
    draw_once(ui);
    let halfway = Point::new(f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y));
    ui.point_at(from);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position: from,
    })]);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);
    for position in [halfway, to] {
        ui.point_at(position);
        ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position,
        })]);
    }
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
        iced::mouse::Button::Left,
    ))]);
}

/// The on-screen centre of a pane, from the bounds the dock recorded while drawing.
///
/// A drop is resolved against the *pane's* rectangle, not against whatever widget
/// happens to be found inside it: a body's text sits near the pane's top, which lands
/// in the Top edge band and splits rather than merges. Asking the session for the
/// pane's own bounds is what makes the drop land where the test means it to.
fn pane_centre(
    ui: &mut Simulator<'_, Message, Theme>,
    session: &DockSession<Panel>,
    id: &str,
) -> Point {
    draw_once(ui);
    let pane = pane_of(session, id).unwrap_or_else(|| panic!("`{id}` should have a pane"));
    let state = session.state();
    let bounds = state
        .borrow()
        .pane_bounds
        .iter()
        .find(|(node, _)| *node == pane)
        .map_or_else(
            || panic!("`{id}`'s pane should have been drawn"),
            |(_, bounds)| *bounds,
        );
    Point::new(
        bounds.x + bounds.width / 2.0,
        bounds.y + bounds.height / 2.0,
    )
}

/// The pane holding a panel, resolved through the session so a dock panel is found.
fn pane_of(session: &DockSession<Panel>, id: &str) -> Option<iced_kit::dock::model::NodeId> {
    session.pane_for_panel(id)
}

/// Render a frame, so the bounds the dock records while drawing exist.
///
/// `pane_bounds` is filled in `draw`, and a fresh simulator has not drawn yet.
fn draw_once(ui: &mut Simulator<'_, Message, Theme>) {
    let _ = ui.snapshot(&Theme::light());
}

/// The tabs a pane holds, wherever in the area it is.
fn tabs_in(session: &DockSession<Panel>, pane: iced_kit::dock::model::NodeId) -> Vec<String> {
    let state = session.state();
    let state = state.borrow();
    // Resolved through the state, not `state.layout`: a pane in a dock is not in the
    // centre tree, and reading the centre alone reports an empty group.
    let Some(pane_state) = state.pane(pane) else {
        return Vec::new();
    };
    let tree = state.tree_of(pane).expect("the pane's own tree");
    pane_state
        .tabs
        .iter()
        .filter_map(|&tab| match tree.kind(tab) {
            Some(NodeKind::Panel(panel)) => Some(panel.id.clone()),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Dragging, in the example's own workspace
// ---------------------------------------------------------------------------

/// The bug the example exposed: dragging a tab *in its own group* looked like
/// nothing happened.
///
/// A `PanelStyle::Auto` group that holds one panel draws a title bar, and dragging
/// that title is how such a panel is picked up. It has to work with the example's own
/// presentation — a title row holding a button — and with the three docks around it.
#[test]
fn a_tab_can_be_dragged_within_the_centre_group() {
    let session = DockSession::from_area(workspace()).expect("valid");
    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));

    // The preview panel's title, dragged onto the middle of the editor's pane. Each
    // centre pane holds one panel, so the panel's title is what is on screen.
    let title = {
        let bounds = ui.find("Preview").expect("the preview title").bounds();
        Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        )
    };
    let to = pane_centre(&mut ui, &session, "editor");
    drag(&mut ui, title, to);

    // The two centre panels are in one group now, so the centre has one pane holding
    // both, not two panes holding one each.
    let editor = pane_of(&session, "editor").expect("an editor pane");
    let preview = pane_of(&session, "preview").expect("a preview pane");
    assert_eq!(
        editor, preview,
        "the preview panel should have joined the editor's group"
    );
    assert_eq!(
        tabs_in(&session, editor).len(),
        2,
        "the joined group should hold both panels"
    );
}

/// A dock panel can be dragged into the centre, which is the other half of the same
/// gesture and the one a user tries first.
#[test]
fn a_dock_panel_can_be_dragged_into_the_centre() {
    let session = DockSession::from_area(workspace()).expect("valid");
    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));

    // The left dock holds two panels, so its bar is a strip: drag one of its tabs.
    let from = centre_of(&mut ui, "Search");
    println!("DBG search tab at {from:?}");
    let to = pane_centre(&mut ui, &session, "editor");
    println!("DBG editor pane at {to:?}");
    drag(&mut ui, from, to);

    let editor = pane_of(&session, "editor").expect("an editor pane");
    let search = pane_of(&session, "search").expect("a search pane");
    assert_eq!(
        editor, search,
        "the search tab should have joined the centre group"
    );
}

/// Dragging the left dock's two-tab strip reorders it, which is what a user does to
/// check the gesture works at all.
#[test]
fn dock_tabs_can_be_reordered() {
    let session = DockSession::from_area(workspace()).expect("valid");
    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));

    let pane = pane_of(&session, "files").expect("the left dock pane");
    assert_eq!(tabs_in(&session, pane), vec!["files", "search"]);

    // `files` is displayed and sits first; drag it past `search`, which is the
    // insertion slot *after* the last tab. Aiming at the midpoint of `search` would
    // land in the slot between the two, which is where `files` already is — a
    // deliberate no-op rather than a move.
    let from = centre_of(&mut ui, "Explorer");
    let to = {
        let bounds = ui.find("Search").expect("the search tab").bounds();
        Point::new(
            bounds.x + bounds.width + 20.0,
            bounds.y + bounds.height / 2.0,
        )
    };
    drag(&mut ui, from, to);

    assert_eq!(
        tabs_in(&session, pane),
        vec!["search", "files"],
        "the dragged tab should have moved past the other"
    );
}

/// A panel that is the only one in its region cannot be dragged out of it: there
/// would be nothing left to show, and nowhere to drag it back from.
#[test]
fn a_docks_only_panel_stays_put() {
    let session = DockSession::from_area(workspace()).expect("valid");
    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));

    let title = centre_of(&mut ui, "Problems");
    let to = pane_centre(&mut ui, &session, "editor");
    drag(&mut ui, title, to);

    assert!(
        pane_of(&session, "problems").is_some(),
        "the panel is still in the workspace"
    );
    let problems = pane_of(&session, "problems").expect("a pane");
    let editor = pane_of(&session, "editor").expect("a pane");
    assert_ne!(
        problems, editor,
        "the only panel of the right dock should not have left it"
    );
}

// ---------------------------------------------------------------------------
// Hiding a panel, the way the example offers it
// ---------------------------------------------------------------------------

/// The example's eye button hid nothing, because the example tracked visibility in
/// its own list while the dock filters on the session's hidden set. This drives the
/// real path: the toolbar button's message reaches the session, and the panel leaves
/// the strip.
#[test]
fn the_toolbar_button_hides_a_panel_through_the_session() {
    let session = DockSession::from_area(workspace()).expect("valid");
    let pane = pane_of(&session, "files").expect("the left dock pane");
    assert_eq!(tabs_in(&session, pane).len(), 2);

    // What the example's `update` should do for `TogglePanelVisible`.
    assert!(session.set_panel_visible("files", false));
    assert!(
        !session.state().borrow().is_panel_visible("files"),
        "the session records the panel as hidden"
    );

    // And showing it again puts it back in the same slot, not at the end.
    assert!(session.set_panel_visible("files", true));
    assert_eq!(
        tabs_in(&session, pane),
        vec!["files", "search"],
        "the panel returns to the slot it had"
    );
}

/// Hiding the panel a pane is *displaying* must not leave the pane blank: the group
/// falls back to the first panel it still has.
#[test]
fn hiding_the_displayed_panel_falls_back_to_another() {
    let session = DockSession::from_area(workspace()).expect("valid");
    let pane = pane_of(&session, "files").expect("the left dock pane");

    // The left dock displays `files`.
    {
        let state = session.state();
        let state = state.borrow();
        let pane_state = state.pane(pane).expect("a pane");
        let active = pane_state.active.expect("an active tab");
        let tree = state.tree_of(active).expect("the tab's tree");
        let Some(NodeKind::Panel(panel)) = tree.kind(active) else {
            panic!("expected a panel");
        };
        assert_eq!(panel.id, "files");
    }

    assert!(session.set_panel_visible("files", false));

    // The dock resolves the displayed tab through the visible set, so the drawn tab
    // is now `search` even though the stored active index still names `files`.
    let ui_state = session.state();
    let ui_state = ui_state.borrow();
    let displayed = ui_state
        .displayed_panel(pane)
        .expect("a panel is still shown");
    let tree = ui_state.tree_of(displayed).expect("the panel's tree");
    match tree.kind(displayed) {
        Some(NodeKind::Panel(panel)) => assert_eq!(panel.id, "search"),
        _ => panic!("expected a panel"),
    }
}

/// The eye button's own icon reflects the session, so the example cannot disagree
/// with the dock about whether a panel is hidden.
#[test]
fn the_toolbar_reports_the_sessions_visibility() {
    let session = DockSession::from_area(workspace()).expect("valid");
    assert!(session.state().borrow().is_panel_visible("files"));

    session.set_panel_visible("files", false);
    let state = session.state();
    let state = state.borrow();
    assert!(!state.is_panel_visible("files"));
    assert!(
        state.is_panel_visible("search"),
        "only the panel that was hidden is hidden"
    );
}

/// Hiding every panel of a dock leaves the dock with nothing to draw, which must not
/// panic or leave a stale tab behind.
#[test]
fn hiding_every_panel_of_a_dock_is_survivable() {
    let session = DockSession::from_area(workspace()).expect("valid");
    session.set_panel_visible("files", false);
    session.set_panel_visible("search", false);

    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));
    // The dock still renders: the left dock's bar is empty, and the rest is intact.
    ui.find("BODY EDITOR")
        .expect("the centre is unaffected by another dock's hidden panels");
    assert!(
        ui.find("Explorer").is_err(),
        "the hidden panel's title is gone"
    );
}

/// A hidden panel is not drawn, so its body cannot be found.
#[test]
fn a_hidden_panel_is_not_on_screen() {
    let session = DockSession::from_area(workspace()).expect("valid");
    assert!(session.set_panel_visible("search", false));

    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));
    assert!(
        ui.find("Search").is_err(),
        "the hidden panel's tab should not be drawn"
    );
    assert!(
        ui.find("Explorer").is_ok(),
        "the panel beside it is unaffected"
    );
}

/// Clicking an eye button hides that panel, through the path the application uses.
///
/// This is the reported bug: the button toggled a flag the *example* kept, while the
/// dock read the session's own hidden set — so the icon changed and nothing else did.
/// The toolbar for a single-panel group sits in its title bar, so the click below
/// lands on the centre's editor.
#[test]
fn clicking_an_eye_button_hides_that_panel() {
    let session = DockSession::from_area(workspace()).expect("valid");

    // The eye button draws `◉` while its panel is visible, so the selector finds it
    // without the test having to know where the toolbar sits.
    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));
    let _ = ui.click("◉");

    let clicked: Vec<Panel> = ui
        .into_messages()
        .filter_map(|message| match message {
            Message::TogglePanelVisible(panel) => Some(panel),
            Message::Dock(_) => None,
        })
        .collect();
    assert_eq!(
        clicked,
        vec![Panel::Editor],
        "the eye button in the editor's title bar should dispatch for the editor"
    );

    // What the application's handler does with that message.
    for panel in &clicked {
        session.set_panel_visible(panel.id(), false);
    }
    assert!(
        !session.is_panel_visible("editor"),
        "the message should have reached the session"
    );

    // And the dock leaves the panel out, which it could not do while the application
    // kept its own copy of the flag.
    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));
    assert!(
        ui.find("main.rs").is_err(),
        "the hidden panel's title is gone"
    );
    assert!(ui.find("BODY PREVIEW").is_ok(), "the pane beside it stays");
}

/// Hiding the panel a pane is *displaying* must swap the content on screen, not
/// only the strip.
///
/// This is the reported bug: the hidden panel's tab left the strip, but the pane
/// went on drawing the hidden panel's *body* — the widget resolved what to display
/// from the stored active tab alone, never asking whether it was visible. The panel
/// that should have taken over drew nothing, so the hide read as "the eye button
/// changed the icon and nothing else happened".
#[test]
fn hiding_the_displayed_panel_swaps_the_body() {
    // A group of two, displaying the *second*: the fallback has a tab to fall
    // back to, and the displayed one is not merely the first tab.
    let session = DockSession::from_area(LayoutArea::new(
        tabs([
            PanelDef::new("files", "Explorer", Panel::Files),
            PanelDef::new("search", "Search", Panel::Search),
        ])
        .active("search"),
    ))
    .expect("valid");
    let pane = pane_of(&session, "search").expect("the pane");

    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));
    let _ = ui.snapshot(&Theme::light());
    assert!(ui.find("BODY SEARCH").is_ok(), "search is displayed");
    assert_eq!(
        session.state().borrow().pane(pane).and_then(|p| p.active),
        session.state().borrow().index.panels.get("search").copied(),
        "search is the stored active"
    );

    // Hide the displayed panel, the way the eye button does.
    assert!(session.set_panel_visible("search", false));

    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));
    let _ = ui.snapshot(&Theme::light());
    assert!(
        ui.find("BODY SEARCH").is_err(),
        "the hidden panel's body must not stay on screen"
    );
    assert!(
        ui.find("BODY FILES").is_ok(),
        "the panel that takes over must be drawn"
    );

    // Showing it again puts it back, because the stored active still names it.
    assert!(session.set_panel_visible("search", true));
    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));
    let _ = ui.snapshot(&Theme::light());
    assert!(ui.find("BODY SEARCH").is_ok(), "search is displayed again");
    assert!(
        ui.find("BODY FILES").is_err(),
        "the panel that stood in steps back down"
    );
}

/// A group whose every panel is hidden draws an empty frame — not the last body
/// it happened to hold, and not a panic.
#[test]
fn hiding_every_panel_of_a_group_clears_its_body() {
    let session = DockSession::from_area(
        LayoutArea::new(tabs([PanelDef::new("editor", "main.rs", Panel::Editor)])).dock(
            DockPlacement::Bottom,
            180.0,
            tabs([PanelDef::new("terminal", "Terminal", Panel::Terminal)]),
        ),
    )
    .expect("valid");

    // Hide the bottom dock's only panel: its region has nothing left to draw.
    assert!(session.set_panel_visible("terminal", false));

    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));
    let _ = ui.snapshot(&Theme::light());
    assert!(
        ui.find("BODY TERMINAL").is_err(),
        "the only panel is hidden, so its body is gone"
    );
    assert!(ui.find("BODY EDITOR").is_ok(), "the centre is unaffected");
}

/// The row the pointer is over is the row that lights up, driven through the
/// whole dock.
///
/// A screenshot showed this going wrong: the pointer had travelled a whole
/// menu's height before the highlight caught up, because the hover path fed
/// the menu a point that had already been made relative to it, and the row
/// lookup subtracted the menu's top again.
///
/// The menu's own tests build a level directly, so they miss the layers a
/// real click passes through — the dock's bar, then the control, then the
/// overlay — which is where the coordinates are translated. This drives the
/// real thing and reads the highlight back out of the pixels.
#[test]
fn the_row_under_the_pointer_is_the_row_that_lights_up() {
    let session = DockSession::from_area(workspace()).expect("valid");
    let mut ui = Simulator::with_size(iced::Settings::default(), WINDOW, view(&session, &[]));

    let button = {
        draw_once(&mut ui);
        let pane = pane_of(&session, "problems").expect("a pane");
        let id = format!("dock-panel-menu:{}", pane.as_u64());
        ui.find(iced::advanced::widget::Id::from(id))
            .expect("the menu button")
            .bounds()
    };
    let trigger = button.center();

    ui.point_at(trigger);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);
    draw_once(&mut ui);

    // A point on the third row, well inside the menu: the row band is 28 tall
    // and the menu body starts 4 below the button.
    let target_row = 2usize;
    let pointer = Point::new(
        trigger.x - 60.0,
        button.y + button.height + 4.0 + 28.0 * target_row as f32 + 14.0,
    );
    ui.point_at(pointer);
    ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position: pointer,
    })]);
    draw_once(&mut ui);

    let label = "target/menu_hover_probe";
    let path = format!("{label}-wgpu.png");
    let _ = std::fs::remove_file(&path);
    ui.snapshot(&Theme::light())
        .expect("the dock should render")
        .matches_image(label)
        .expect("the frame should be written");

    let bytes = std::fs::read(&path).expect("the frame");
    let decoder = png::Decoder::new(bytes.as_slice());
    let mut reader = decoder.read_info().expect("a PNG");
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).expect("decode");
    pixels.truncate(info.buffer_size());
    let _ = std::fs::remove_file(&path);

    let width = info.width as usize;
    let height = info.height as usize;
    let scale = (width as f32 / WINDOW.width).round();

    // The accent wash a selected row is painted with: a light grey, off both
    // the white surface and the dark text.
    let is_highlight = |x: usize, y: usize| {
        let i = (y * width + x) * 4;
        let [r, g, b] = [pixels[i], pixels[i + 1], pixels[i + 2]];
        (240..=250).contains(&r) && (240..=250).contains(&g) && (240..=250).contains(&b)
    };

    // Sample a column inside the menu's body, clear of its 1px border and its
    // rounded corners, and of the icon column's glyphs.
    let column = ((trigger.x - 70.0) * scale) as usize;
    let mut bands: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for y in 0..height {
        if is_highlight(column, y) {
            start = start.or(Some(y));
        } else if let Some(from) = start.take() {
            bands.push((from, y - 1));
        }
    }
    if let Some(from) = start {
        bands.push((from, height - 1));
    }

    // The tallest band is the selected row; the others are whatever chrome
    // happens to share the wash's tone.
    let (top, bottom) = bands
        .iter()
        .copied()
        .max_by_key(|(from, to)| to - from)
        .expect("a highlighted row should be drawn");

    let (top, bottom) = (top as f32 / scale, bottom as f32 / scale);
    let row_top = button.y + button.height + 4.0 + 28.0 * target_row as f32;

    assert!(
        (top - row_top).abs() <= 2.0 && (bottom - (row_top + 28.0)).abs() <= 2.0,
        "the highlight should cover row {target_row} ({row_top:.1}..{:.1}): it drew {top:.1}..{bottom:.1}",
        row_top + 28.0
    );
    assert!(
        pointer.y >= top && pointer.y <= bottom,
        "the pointer at y={:.1} should sit inside the highlighted band {top:.1}..{bottom:.1}",
        pointer.y
    );
}
