//! A docking workspace, exercising every part of the dock.
//!
//! Run with:
//!
//! ```text
//! cargo run --example dock --features dock
//! ```
//!
//! # What to try
//!
//! * **Drag a tab** onto another group's content area to move it there, or onto an
//!   edge of it to split that group.
//! * **Drag a tab onto the tab strip** of another group to insert it at a position.
//! * **Drag a divider** between panes, or the inner edge of an edge dock, to resize.
//! * **Click a dock's toggle** in a tab bar — an open dock draws the collapse icon,
//!   a closed one the expand icon — to put it away. A closed bottom dock keeps its
//!   strip, so it can be clicked back open.
//! * **Zoom a panel** with its toolbar button, or through the tab bar's ⋯ menu.
//!   Zoom fills the window with that panel; restoring puts the layout back.
//! * **Hide a panel** with the toolbar's eye button. A hidden panel keeps its tab
//!   slot, so showing it again restores the order.
//! * **Save and Load** the whole workspace with the buttons at the top. The status
//!   line shows what the dock reported and where each dock sits.
//!
//! # What this shows beyond the gallery
//!
//! The gallery demonstrates `Resizable`, the simpler two-pane case. This is the dock
//! proper: three collapsible regions, a panel presentation supplying titles, a
//! toolbar and a menu, `PanelStyle::Auto` so a lone panel draws a title bar instead
//! of a one-tab strip, runtime panel visibility, and a capture/restore round trip.

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use iced_kit::icons::IconName;
use iced_kit::theme::Theme;
use iced_kit::widgets::dock::{
    self, DockAreaState, DockEvent, DockPlacement, DockSession, LayoutArea, PanelControl,
    PanelDef, PanelStyle, PanelPresentation,
};
use iced_kit::widgets::{icon_button, muted_text};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(App::theme)
        .title("iced-kit — dock")
        .window_size((1180.0, 760.0))
        .run()
}

/// The panels this workspace can hold.
///
/// A `Copy` key rather than a trait object: the dock asks for a panel's chrome on
/// every frame, and a key costs nothing to pass around.
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
    /// Every panel key, so the tests can assert they are all covered. Only the
    /// tests need it — the application names the panels it uses directly — so it is
    /// allowed to look unused in a plain build.
    #[cfg_attr(not(test), allow(dead_code))]
    const ALL: [Self; 6] = [
        Self::Files,
        Self::Search,
        Self::Editor,
        Self::Preview,
        Self::Terminal,
        Self::Problems,
    ];

    /// The string id this panel is registered under, which is what the session
    /// indexes it by and what visibility is asked about.
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

    /// A short label, for a group with no room for the full title.
    fn short(self) -> &'static str {
        match self {
            Self::Files => "Files",
            Self::Search => "Find",
            Self::Editor => "main.rs",
            Self::Preview => "View",
            Self::Terminal => "Term",
            Self::Problems => "Issues",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Files => IconName::Folder,
            Self::Search => IconName::Search,
            Self::Editor => IconName::FileCode,
            Self::Preview => IconName::Eye,
            Self::Terminal => IconName::Terminal,
            Self::Problems => IconName::TriangleAlert,
        }
    }

    /// The body of the panel.
    ///
    /// A real application builds a widget here; the example draws something whose
    /// bounds are obvious, so a resize or a split is visible at a glance.
    fn body(self) -> Element<'static, Message, Theme> {
        let lines: Vec<Element<'static, Message, Theme>> = match self {
            Self::Files => ["src", "  main.rs", "  dock.rs", "Cargo.toml", "README.md"]
                .iter()
                .map(|line| text(*line).into())
                .collect(),
            Self::Search => ["dock.rs — 14 matches", "  panel.rs", "  region.rs"]
                .iter()
                .map(|line| text(*line).into())
                .collect(),
            Self::Editor => (1..=30)
                .map(|n| {
                    text(format!("{n:>3}  fn view(&self) -> Element<'_, Message> {{")).into()
                })
                .collect(),
            Self::Preview => vec![muted_text("Rendered preview").into()],
            Self::Terminal => vec![
                text("$ cargo test --features dock").into(),
                text("test result: ok. 995 passed").into(),
            ],
            Self::Problems => vec![muted_text("No problems detected").into()],
        };

        container(
            scrollable(column(lines).spacing(2))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

/// What the application adds to a panel's own chrome.
///
/// Every method has a default that draws nothing, so only the ones this example
/// wants are implemented — the rest say "the dock's default is fine".
///
/// The visibility the eye button reflects comes from the *session*, not from a list
/// kept here: the session is what the dock reads when it decides which tabs to draw,
/// so a second copy could disagree with it. That disagreement was the bug that made
/// the button look inert.
struct Panels {
    hidden: Vec<Panel>,
}

impl PanelPresentation<Panel, Message, Theme> for Panels {
    fn title(&self, panel: Panel) -> Option<Element<'static, Message, Theme>> {
        // An element rather than a string, which is what lets the icon be part of
        // the tab and the title bar.
        Some(
            row![
                icon_button::<Message>().icon(panel.icon()).ghost().compact(),
                text(panel.title()).size(13)
            ]
            .spacing(4)
            .align_y(Alignment::Center)
            .into(),
        )
    }

    /// The eye button, whose icon reflects the session's own record of visibility.
    fn toolbar(&self, panel: Panel) -> Vec<Element<'static, Message, Theme>> {
        let hidden = self.hidden.contains(&panel);
        let icon = if hidden { IconName::EyeOff } else { IconName::Eye };
        vec![
            icon_button::<Message>()
                .icon(icon)
                .ghost()
                .compact()
                .on_press(Message::TogglePanelVisible(panel))
                .into(),
        ]
    }

    fn menu(&self, panel: Panel) -> Vec<(String, Message)> {
        vec![
            (format!("Copy {} path", panel.short()), Message::Noop),
            (format!("Close {}", panel.short()), Message::Noop),
        ]
    }

    fn zoom_control(&self, _panel: Panel) -> Option<PanelControl> {
        // In both places, so the example shows each: the toolbar button and the menu
        // entry are the same action.
        Some(PanelControl::Both)
    }

    fn inner_padding(&self, _panel: Panel) -> bool {
        // `Panel::body` draws its own padding, so the group must not add more.
        false
    }

    fn title_bar(&self, _panel: Panel) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
enum Message {
    Dock(DockEvent<Panel>),
    TogglePanelVisible(Panel),
    /// A toolbar toggle: an *action*, dispatched through the session, not an
    /// event. An event only reports what already happened, so a button that
    /// sends one changes nothing — the dock reads the session, and the session
    /// is what has to be told.
    ToggleDock(DockPlacement),
    Save,
    Load,
    Reset,
    Noop,
}

struct App {
    session: DockSession<Panel>,
    /// The last thing the dock reported, so the example shows events arriving.
    last_event: String,
    /// The saved workspace. A real application would keep a file per project; one
    /// slot is enough to show the round trip.
    saved: Option<DockAreaState<Panel>>,
}

/// The workspace this example opens with.
///
/// A centre split of editor and preview, plus all three edge docks — the shape the
/// gallery's `Resizable` cannot express.
fn workspace() -> LayoutArea<Panel> {
    LayoutArea::new(dock::horizontal([
        dock::tabs([PanelDef::new("editor", "main.rs", Panel::Editor)]),
        dock::tabs([PanelDef::new("preview", "Preview", Panel::Preview)]),
    ]))
    .dock(
        DockPlacement::Left,
        240.0,
        dock::tabs([
            PanelDef::new("files", "Explorer", Panel::Files),
            PanelDef::new("search", "Search", Panel::Search),
        ])
        .active("files"),
    )
    .dock(
        DockPlacement::Right,
        250.0,
        dock::tabs([PanelDef::new("problems", "Problems", Panel::Problems)]),
    )
    .dock(
        DockPlacement::Bottom,
        180.0,
        dock::tabs([PanelDef::new("terminal", "Terminal", Panel::Terminal)]),
    )
}

impl App {
    fn new() -> (Self, iced::Task<Message>) {
        let session = DockSession::from_area(workspace()).expect("the workspace is valid");
        (
            Self {
                session,
                last_event: "ready — drag a tab, a divider, or a dock's toggle".to_owned(),
                saved: None,
            },
            iced::Task::none(),
        )
    }

    /// iced calls this with the application, so the signature is fixed even though
    /// this example has one theme.
    #[allow(clippy::unused_self)]
    fn theme(&self) -> Theme {
        Theme::light()
    }

    /// The presentation, told which panels the *session* considers hidden.
    fn presentation(&self) -> Panels {
        Panels {
            hidden: Panel::ALL
                .into_iter()
                .filter(|panel| !self.session.is_panel_visible(panel.id()))
                .collect(),
        }
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Dock(event) => {
                // The widget has already applied the layout change; this is where a
                // real application would persist, log or react to it.
                self.last_event = describe(&event);
            }
            Message::TogglePanelVisible(panel) => {
                // The session is told, because the session is what the dock reads when
                // it decides which tabs to draw. Toggling a flag kept here instead
                // would only change the button's own icon.
                let now_visible = !self.session.is_panel_visible(panel.id());
                self.session.set_panel_visible(panel.id(), now_visible);
                self.last_event = format!(
                    "{} {}",
                    if now_visible { "showed" } else { "hid" },
                    panel.short()
                );
            }
            Message::ToggleDock(placement) => {
                // Dispatched as an action, the same way the dock's own toggle
                // button does it: the session applies it and the dock redraws
                // from the session.
                let state = self.session.state();
                let was_open = state.borrow().regions.is_dock_open(placement);
                self.session
                    .dispatch(dock::DockAction::ToggleDock { placement });
                self.last_event = format!(
                    "{} the {} dock",
                    if was_open { "closed" } else { "opened" },
                    placement_name(placement)
                );
            }
            Message::Save => {
                let saved = self.session.capture(Some(1));
                self.last_event = format!("saved {} dock(s) plus the centre", saved.docks.len());
                self.saved = Some(saved);
            }
            Message::Load => match self.saved.take() {
                Some(saved) => match self.session.restore(&saved) {
                    Ok(()) => {
                        self.saved = Some(saved);
                        self.last_event = String::from("loaded the saved workspace");
                    }
                    Err(error) => self.last_event = format!("load failed: {error}"),
                },
                None => {
                    self.last_event =
                        String::from("nothing saved yet — press Save first");
                }
            },
            Message::Reset => match self.session.set_area(workspace()) {
                Ok(()) => {
                    self.saved = None;
                    // Every panel the workspace declares, shown again.
                    for panel in Panel::ALL {
                        self.session.set_panel_visible(panel.id(), true);
                    }
                    self.last_event = String::from("reset to the opening workspace");
                }
                Err(error) => self.last_event = format!("reset failed: {error}"),
            },
            Message::Noop => {}
        }
        iced::Task::none()
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        // `apply_metrics` wraps the builder rather than chaining onto it, so it is
        // applied around the settings it does not touch.
        let builder = dock::dock::<Panel, Message, Theme, iced::Renderer>()
            .state(self.session.state())
            .on_event(Message::Dock)
            // The token-derived style, so the dock matches the rest of the crate.
            .style(dock::style)
            // A lone panel draws a title bar rather than a one-tab strip, which is
            // what an editor wants; two or more draw the strip.
            .panel_style(PanelStyle::Auto)
            .presentation(self.presentation());

        // The crate's recommended metrics, so the dock's chrome lines up with the
        // buttons above it.
        let dock: Element<'_, Message, Theme> = dock::apply_metrics(builder)
            .content(Panel::body)
            .build()
            .into();

        column![self.toolbar(), dock, self.status()]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// The bar above the dock.
    ///
    /// In a real application these would live in the window's own title bar; here
    /// they double as a legend for the example.
    fn toolbar(&self) -> Element<'_, Message, Theme> {
        let state = self.session.state();
        let toggles: Vec<Element<'_, Message, Theme>> = {
            let state = state.borrow();
            DockPlacement::DOCKS
                .iter()
                .map(|placement| {
                    let open = state.regions.is_dock_open(*placement);
                    let label = format!(
                        "{} {}",
                        if open { "Hide" } else { "Show" },
                        placement_name(*placement)
                    );
                    button(text(label))
                        .on_press(Message::ToggleDock(*placement))
                        .into()
                })
                .collect()
        };

        container(
            row![
                text("Dock").size(14),
                button(text("Save")).on_press(Message::Save),
                button(text("Load")).on_press(Message::Load),
                button(text("Reset")).on_press(Message::Reset),
                row(toggles).spacing(6),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .into()
    }

    /// The line under the dock: what just happened, and where each dock sits.
    fn status(&self) -> Element<'_, Message, Theme> {
        let state = self.session.state();
        let state = state.borrow();

        let mut docks = Vec::new();
        for (placement, _) in state.regions.iter() {
            if let Some(dock) = state.regions.dock(placement) {
                docks.push(format!(
                    "{} {:.0}px{}",
                    placement_name(placement),
                    dock.size(),
                    if dock.is_open() { "" } else { " (closed)" }
                ));
            }
        }
        if let Some(zoomed) = state.regions.zoomed() {
            let _ = zoomed;
            docks.push("zoomed".to_owned());
        }

        let hidden: Vec<&str> = Panel::ALL
            .into_iter()
            .filter(|panel| !self.session.is_panel_visible(panel.id()))
            .map(Panel::short)
            .collect();
        let hidden = if hidden.is_empty() {
            String::new()
        } else {
            format!("  ·  hidden: {}", hidden.join(", "))
        };

        container(
            row![
                muted_text(format!("last event: {}", self.last_event)),
                muted_text(format!("  ·  {}", docks.join("  ·  "))),
                muted_text(hidden),
            ]
            .spacing(4),
        )
        .padding([6, 10])
        .width(Length::Fill)
        .into()
    }
}

fn placement_name(placement: DockPlacement) -> &'static str {
    match placement {
        DockPlacement::Center => "centre",
        DockPlacement::Left => "left",
        DockPlacement::Right => "right",
        DockPlacement::Bottom => "bottom",
    }
}

/// A short description of an event, for the status line.
fn describe(event: &DockEvent<Panel>) -> String {
    match event {
        DockEvent::TabSelected { panel, .. } => format!("selected {}", panel.short()),
        DockEvent::TabClosed { panel } => format!("closed {}", panel.short()),
        DockEvent::PaneFocused { .. } => "focused a pane".to_owned(),
        DockEvent::SplitResized { pair_ratio, .. } => {
            format!("resized a divider to {:.0}%", pair_ratio * 100.0)
        }
        DockEvent::DragStarted { panel } => format!("started dragging {}", panel.short()),
        DockEvent::DragMoved { cursor } => {
            format!("dragging at {:.0}, {:.0}", cursor.x, cursor.y)
        }
        DockEvent::DragEnded { .. } => "dropped a tab".to_owned(),
        DockEvent::DragCancelled => "the drag was cancelled".to_owned(),
        DockEvent::ZoomChanged { zoomed, panel } => match (zoomed, panel) {
            (true, Some(panel)) => format!("maximized {}", panel.short()),
            _ => "restored the layout".to_owned(),
        },
        DockEvent::DockToggled { placement, open } => format!(
            "{} the {} dock",
            if *open { "opened" } else { "closed" },
            placement_name(*placement)
        ),
        DockEvent::DockResized { placement, size } => {
            format!("resized the {} dock to {size:.0}px", placement_name(*placement))
        }
        DockEvent::LayoutChanged => "the layout changed".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Point;

    /// The example is a usage check: if the documented integration stops compiling,
    /// or the workspace stops being valid, this fails before a window is opened.
    #[test]
    fn the_workspace_is_valid_and_holds_every_panel() {
        let session = DockSession::from_area(workspace()).expect("valid");
        let state = session.state();
        let state = state.borrow();

        for placement in DockPlacement::DOCKS {
            assert!(
                state.regions.has_dock(placement),
                "{placement:?} is in the workspace"
            );
            assert!(state.regions.is_dock_open(placement));
        }
        assert_eq!(state.index.panels.len(), Panel::ALL.len());
    }

    #[test]
    fn saving_and_loading_the_workspace_round_trips() {
        let session = DockSession::from_area(workspace()).expect("valid");
        session.dispatch(dock::DockAction::ToggleDock {
            placement: DockPlacement::Right,
        });
        let saved = session.capture(Some(1));

        let reloaded = DockSession::from_area(LayoutArea::new(dock::tabs([PanelDef::new(
            "editor",
            "main.rs",
            Panel::Editor,
        )])))
        .expect("valid");
        reloaded.restore(&saved).expect("restores");

        let state = reloaded.state();
        let state = state.borrow();
        assert!(state.regions.has_dock(DockPlacement::Left));
        assert!(state.regions.has_dock(DockPlacement::Bottom));
        assert!(
            !state.regions.is_dock_open(DockPlacement::Right),
            "the closed dock is still closed"
        );
        assert_eq!(state.index.panels.len(), Panel::ALL.len());
    }

    #[test]
    fn saving_keeps_the_dock_sizes_the_user_dragged() {
        let session = DockSession::from_area(workspace()).expect("valid");
        {
            let state = session.state();
            let mut state = state.borrow_mut();
            state.area_bounds = Some(iced::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 1200.0,
                height: 800.0,
            });
            assert!(state.resize_dock_to(DockPlacement::Left, iced::Point::new(320.0, 400.0)));
        }
        let saved = session.capture(None);

        let left = saved
            .docks
            .iter()
            .find(|d| d.placement == DockPlacement::Left)
            .expect("the left dock");
        assert_eq!(left.size, 320.0);
    }

    /// Every panel key has a title, a short label, an icon and a body, so no pane in
    /// the example can render blank, and the labels stay distinct.
    #[test]
    fn every_panel_is_fully_described() {
        for panel in Panel::ALL {
            assert!(!panel.title().is_empty());
            assert!(!panel.short().is_empty());
            let _ = panel.icon();
            let _ = panel.body();
        }

        let titles: std::collections::HashSet<&str> =
            Panel::ALL.iter().map(|p| p.title()).collect();
        let shorts: std::collections::HashSet<&str> =
            Panel::ALL.iter().map(|p| p.short()).collect();
        assert_eq!(titles.len(), Panel::ALL.len(), "titles are distinct");
        assert_eq!(shorts.len(), Panel::ALL.len(), "short labels are distinct");
    }

    /// The presentation's title, toolbar and menu all build, so the seam is
    /// exercised without a window.
    #[test]
    fn the_presentation_builds_its_chrome() {
        let panels = Panels { hidden: Vec::new() };
        for panel in Panel::ALL {
            assert!(panels.title(panel).is_some());
            let _ = panels.toolbar(panel);
            let _ = panels.menu(panel);
            assert!(panels.zoom_control(panel).is_some());
        }

        // A hidden panel's toolbar says so, which is what the eye button reflects.
        let hiding = Panels {
            hidden: vec![Panel::Terminal],
        };
        let _ = hiding.toolbar(Panel::Terminal);
    }

    /// Every event the dock can report has a description, so the status line never
    /// silently shows nothing.
    #[test]
    fn every_event_is_described() {
        let events = [
            DockEvent::TabSelected {
                pane: None,
                panel: Panel::Editor,
            },
            DockEvent::TabClosed {
                panel: Panel::Editor,
            },
            DockEvent::PaneFocused {
                pane: None,
                panel: None,
            },
            DockEvent::SplitResized {
                splitter_index: 0,
                pair_ratio: 0.5,
            },
            DockEvent::DragStarted {
                panel: Panel::Editor,
            },
            DockEvent::DragMoved {
                cursor: iced::Point::new(0.0, 0.0),
            },
            DockEvent::DragEnded {
                cursor: iced::Point::new(0.0, 0.0),
            },
            DockEvent::DragCancelled,
            DockEvent::ZoomChanged {
                zoomed: true,
                panel: Some(Panel::Editor),
            },
            DockEvent::DockToggled {
                placement: DockPlacement::Left,
                open: false,
            },
            DockEvent::DockResized {
                placement: DockPlacement::Left,
                size: 200.0,
            },
            DockEvent::LayoutChanged,
        ];
        for event in &events {
            assert!(!describe(event).is_empty(), "{event:?} has no description");
        }
    }

    /// Renders the example's own app, so its layout can be looked at.

    /// Renders the example's own app, so a change in its layout can be looked at.
    ///
    /// Written, not compared: the image is for a human to inspect, and comparing it
    /// would make every intentional style change a failing test.
    #[test]
    fn the_app_renders() {
        let (app, _) = App::new();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1180.0, 760.0),
            app.view(),
        );
        ui.snapshot(&Theme::light())
            .expect("the app renders")
            .matches_image("target/example-shots/dock.png")
            .expect("the screenshot is written");
    }

    /// Drags the preview panel's title into the editor's pane, drawing a frame
    /// between every pointer event.
    ///
    /// The frame matters. The dock rebuilds its widget tree between frames, so a
    /// burst of events inside a single frame exercises a different — and easier —
    /// path than a real window does.
    #[test]
    fn dragging_a_title_into_another_pane_moves_the_panel() {
        let (app, _) = App::new();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1180.0, 760.0),
            app.view(),
        );
        let _ = ui.snapshot(&Theme::light());

        let from = ui.find("Preview").expect("the preview title").bounds();
        let from = iced::Point::new(from.x + from.width / 2.0, from.y + from.height / 2.0);
        let to = pane_centre(&app, "editor");
        assert_ne!(
            pane_of(&app, "editor"),
            pane_of(&app, "preview"),
            "the two panels start in different panes"
        );

        drive_drag(&mut ui, from, to);

        assert_eq!(
            pane_of(&app, "editor"),
            pane_of(&app, "preview"),
            "the dragged panel should have joined the editor's pane"
        );

        ui.snapshot(&Theme::light())
            .expect("renders after the drag")
            .matches_image("target/example-shots/dock-after-drag.png")
            .expect("the screenshot is written");
    }

    /// Clicking the eye in a panel's title bar hides that panel, through the
    /// application's own `update`.
    #[test]
    fn the_eye_button_hides_its_panel() {
        let (mut app, _) = App::new();
        assert!(app.session.is_panel_visible("editor"), "it starts visible");

        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1180.0, 760.0),
            app.view(),
        );
        // A frame first: `pane_bounds` is filled while drawing, and a fresh
        // simulator has not drawn yet.
        let _ = ui.snapshot(&Theme::light());

        // The eye sits among the trailing controls of the editor's title bar, after
        // the panel's own toolbar and before the dock's zoom and menu.
        let pane = pane_of(&app, "editor").expect("the editor pane");
        let bounds = pane_bounds(&app, pane);
        let at = iced::Point::new(bounds.x + bounds.width - 60.0, bounds.y + 16.0);

        click_at(&mut ui, at);
        for message in ui.into_messages() {
            let _ = app.update(message);
        }

        assert!(
            !app.session.is_panel_visible("editor"),
            "the eye button should have hidden the editor"
        );
    }

    /// The dock's own controls are addressable by id and do their job.
    ///
    /// A control drawn as an icon has no text to be found by, which is why each
    /// carries an id — and why these are clicked by id rather than by a glyph.
    #[test]
    fn the_docks_controls_work() {
        // A dock toggle collapses its dock.
        let (mut app, _) = App::new();
        for message in click_id(&mut app, "dock-toggle:left") {
            let _ = app.update(message);
        }
        assert!(
            !app.session
                .state()
                .borrow()
                .regions
                .is_dock_open(DockPlacement::Left),
            "the toggle should have collapsed the left dock"
        );

        // The zoom control maximizes the panel.
        let (mut app, _) = App::new();
        let pane = pane_of(&app, "editor").expect("the editor pane");
        for message in click_id(&mut app, &format!("dock-zoom:{}", pane.as_u64())) {
            let _ = app.update(message);
        }
        assert!(
            app.session.state().borrow().regions.zoomed().is_some(),
            "the zoom control should have maximized the panel"
        );
    }

    /// Every control the dock draws can be found by id, so a test or an application
    /// can address it without knowing where it sits.
    #[test]
    fn the_docks_controls_are_addressable() {
        let (app, _) = App::new();
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1180.0, 760.0),
            app.view(),
        );
        let _ = ui.snapshot(&Theme::light());

        for id in [
            "dock-toggle:left".to_owned(),
            "dock-toggle:bottom".to_owned(),
            "dock-toggle:right".to_owned(),
        ] {
            assert!(
                ui.find(iced::advanced::widget::Id::from(id.clone()))
                    .is_ok(),
                "`{id}` should be addressable"
            );
        }

        // The per-pane controls, which is why their ids carry the pane.
        for panel in ["editor", "preview", "files", "terminal", "problems"] {
            let Some(pane) = pane_of(&app, panel) else {
                continue;
            };
            for prefix in ["dock-zoom", "dock-panel-menu"] {
                let id = format!("{prefix}:{}", pane.as_u64());
                assert!(
                    ui.find(iced::advanced::widget::Id::from(id.clone()))
                        .is_ok(),
                    "`{id}` should be addressable"
                );
            }
        }
    }

    // --- helpers ---------------------------------------------------------

    /// The pane holding a panel, by the panel's string id.
    fn pane_of(app: &App, id: &str) -> Option<iced_kit::dock::model::NodeId> {
        app.session.state().borrow().pane_for_panel_id(id)
    }

    /// The on-screen rectangle a pane was last drawn in.
    fn pane_bounds(app: &App, pane: iced_kit::dock::model::NodeId) -> iced::Rectangle {
        app.session
            .state()
            .borrow()
            .pane_bounds
            .iter()
            .find(|(node, _)| *node == pane)
            .map(|(_, bounds)| *bounds)
            .expect("the pane was drawn")
    }

    /// The centre of a pane, from the bounds the dock recorded while drawing.
    fn pane_centre(app: &App, id: &str) -> iced::Point {
        let pane = pane_of(app, id).unwrap_or_else(|| panic!("`{id}` has a pane"));
        let bounds = pane_bounds(app, pane);
        iced::Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0)
    }

    /// Point, press, move twice, release — with a frame drawn between each step.
    fn drive_drag(
        ui: &mut iced_test::Simulator<'_, Message, Theme>,
        from: iced::Point,
        to: iced::Point,
    ) {
        let halfway =
            iced::Point::new(f32::midpoint(from.x, to.x), f32::midpoint(from.y, to.y));

        ui.point_at(from);
        ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: from,
        })]);
        let _ = ui.snapshot(&Theme::light());
        ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
            iced::mouse::Button::Left,
        ))]);
        let _ = ui.snapshot(&Theme::light());

        // Two moves: the first crosses the drag threshold, the second is what the
        // drop resolves against.
        for position in [halfway, to] {
            ui.point_at(position);
            ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                position,
            })]);
            let _ = ui.snapshot(&Theme::light());
        }

        ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        ))]);
        let _ = ui.snapshot(&Theme::light());
    }

    /// Press and release at a point, with a frame between.
    fn click_at(ui: &mut iced_test::Simulator<'_, Message, Theme>, at: iced::Point) {
        ui.point_at(at);
        ui.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: at,
        })]);
        let _ = ui.snapshot(&Theme::light());
        ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
            iced::mouse::Button::Left,
        ))]);
        let _ = ui.snapshot(&Theme::light());
        ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        ))]);
        let _ = ui.snapshot(&Theme::light());
    }

    /// Click a control by id against a fresh render of `app`, returning the messages.
    fn click_id(app: &mut App, id: &str) -> Vec<Message> {
        let mut ui = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(1180.0, 760.0),
            app.view(),
        );
        let _ = ui.snapshot(&Theme::light());
        if let Ok(target) = ui.find(iced::advanced::widget::Id::from(id.to_owned())) {
            click_at(&mut ui, target.bounds().center());
        }
        ui.into_messages().collect()
    }
}
