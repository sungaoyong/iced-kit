//! A ribbon built from the generic `ribbon` component: a strip of tabs over a
//! row of titled groups, each packing large, small, dropdown, and grid tools.
//!
//! The ribbon is pure data rebuilt every frame; which tab is active and which
//! dropdown is open live in a [`RibbonState`] the app owns. A ▾ reports the id
//! it wants opened and the app hosts that panel through [`Layer`], anchored
//! with [`trigger`] — the same division of labour as the combobox and picker
//! panels. Nothing here is CAD-specific: the tools are named icons that send
//! plain messages.
//!
//! Run with `cargo run --example ribbon`.

use iced::widget::{column, container, row, stack, text};
use iced::{Alignment, Element, Length, Rectangle, Task};
use iced_kit::icons::IconName;
use iced_kit::widgets::button;
use iced_kit::widgets::overlay::{self, popover_dismiss_area, Dropdown, Layer, MenuItem};
use iced_kit::widgets::ribbon::{
    CollapseMode, Ribbon, RibbonGroup, RibbonItem, RibbonState, RibbonTab, RibbonTool,
};
use iced_kit::Theme;

/// The gap between a trigger's edge and the panel dropped beneath it.
const PANEL_GAP: f32 = 6.0;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .run()
}

/// The example's message. A tool carries the label it should report.
#[derive(Debug, Clone)]
enum Message {
    Noop,
    ToggleTheme,
    /// A tab was picked.
    TabSelected(usize),
    /// A ▾ asked to open (or close) the panel named by its id.
    DropdownToggled(String),
    /// Where the ribbon sits, reported by `trigger` before a press lands.
    RibbonAnchored(Rectangle),
    /// A press outside the hosted panel.
    CloseDropdown,
    /// A density mode was picked from the selector, pinning every group.
    CollapseSet(CollapseMode),
    /// A tool or a panel item was activated.
    Ran(&'static str),
}

/// The example's state: the theme, the ribbon's own view state, the anchor the
/// hosted panel drops from, and the last thing that ran.
struct App {
    dark: bool,
    ribbon: RibbonState,
    anchor: Option<Rectangle>,
    status: String,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                dark: false,
                ribbon: RibbonState::new(),
                anchor: None,
                status: String::from("Pick a tab or press a tool."),
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        String::from(if self.dark {
            "Ribbon — iced-kit (dark)"
        } else {
            "Ribbon — iced-kit (light)"
        })
    }

    fn theme(&self) -> Theme {
        if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop => {}
            Message::ToggleTheme => self.dark = !self.dark,
            Message::TabSelected(index) => self.ribbon.select(index),
            Message::DropdownToggled(id) => self.ribbon.toggle_dropdown(&id),
            Message::RibbonAnchored(rectangle) => self.anchor = Some(rectangle),
            Message::CloseDropdown => self.ribbon.close_dropdown(),
            Message::CollapseSet(mode) => self.ribbon.set_collapse_mode(mode),
            Message::Ran(label) => {
                self.status = format!("Ran “{label}”");
                self.ribbon.close_dropdown();
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        // The ribbon reports each pressed ▾'s own bounds through
        // `on_dropdown_anchor`, so the hosted panel drops beneath that button
        // rather than at one fixed corner of the ribbon.
        let ribbon = self.build_ribbon();

        let page = container(
            column![
                self.header(),
                self.mode_selector(),
                ribbon,
                self.status_line()
            ]
            .spacing(16)
            .padding(16)
            .width(Length::Fill)
            .align_x(Alignment::Start),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let mut open = Layer::new();
        if let (Some(anchor), Some(id)) = (self.anchor, self.ribbon.open_dropdown.as_deref()) {
            let panel = stack![
                popover_dismiss_area(Message::CloseDropdown),
                Dropdown::new(panel_items(id))
                    .anchor(anchor.x, anchor.y + anchor.height + PANEL_GAP),
            ];
            open = open.dropdown(panel);
        }

        overlay::layer(page, open)
    }

    /// The header row: a title and the theme toggle.
    fn header(&self) -> Element<'_, Message, Theme> {
        let toggle = if self.dark { "Light mode" } else { "Dark mode" };
        row![
            text("Ribbon").size(20),
            container(text("")).width(Length::Fill),
            button(toggle).secondary().on_press(Message::ToggleTheme),
        ]
        .align_y(Alignment::Center)
        .into()
    }

    /// The line that reports the last tool that ran.
    fn status_line(&self) -> Element<'_, Message, Theme> {
        container(text(&self.status).size(14)).into()
    }

    /// A row of buttons pinning every group to one density. *Auto* lets the
    /// ribbon degrade the row from the right as the window narrows; the rest
    /// force a single level so each can be inspected at any width.
    fn mode_selector(&self) -> Element<'_, Message, Theme> {
        let buttons = CollapseMode::ALL
            .iter()
            .fold(row![].spacing(6), |row, mode| {
                let active = self.ribbon.collapse_mode == *mode;
                let btn = button(mode.label());
                let btn = if active { btn.primary() } else { btn.ghost() };
                row.push(btn.on_press(Message::CollapseSet(*mode)))
            });
        row![text("Density").size(13), buttons]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
    }

    /// The ribbon itself, rebuilt from data every frame.
    fn build_ribbon(&self) -> Ribbon<'_, Message> {
        Ribbon::new()
            .tab(home_tab())
            .tab(modify_tab())
            .tab(view_tab(&self.ribbon))
            .state(&self.ribbon)
            .on_select(Message::TabSelected)
            .on_dropdown_toggle(Message::DropdownToggled)
            .on_dropdown_anchor(Message::RibbonAnchored)
    }
}

/// The "Home" tab: a clipboard group with a large dropdown, a draw group of
/// large tools, and a styles group of labelled small tools.
fn home_tab() -> RibbonTab<Message> {
    RibbonTab::new("Home")
        .group(
            RibbonGroup::new("Clipboard")
                // A large dropdown: a full-height icon over "Paste" with a ▾.
                .item(RibbonItem::large_dropdown(
                    "paste",
                    IconName::ClipboardPaste,
                    "Paste",
                    vec![],
                ))
                .item(RibbonItem::tool(
                    RibbonTool::named(IconName::Copy)
                        .label("Copy")
                        .on_press(Message::Ran("Copy")),
                ))
                .item(RibbonItem::tool(
                    RibbonTool::named(IconName::Scissors)
                        .label("Cut")
                        .on_press(Message::Ran("Cut")),
                )),
        )
        .group(
            RibbonGroup::new("Draw")
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Spline)
                        .label("Line")
                        .selected(true)
                        .on_press(Message::Ran("Line")),
                ))
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Shapes)
                        .label("Shapes")
                        .on_press(Message::Ran("Shapes")),
                ))
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Highlighter)
                        .label("Sketch")
                        .disabled(true),
                )),
        )
        .group(
            RibbonGroup::new("Styles")
                .item(RibbonItem::labeled(
                    RibbonTool::named(IconName::Bold)
                        .label("Bold")
                        .on_press(Message::Ran("Bold")),
                ))
                .item(RibbonItem::labeled(
                    RibbonTool::named(IconName::Italic)
                        .label("Italic")
                        .on_press(Message::Ran("Italic")),
                ))
                .item(RibbonItem::labeled(
                    RibbonTool::named(IconName::Underline)
                        .label("Underline")
                        .on_press(Message::Ran("Underline")),
                ))
                .item(RibbonItem::dropdown("layers", IconName::Layers, vec![])),
        )
}

/// The "Modify" tab: large edit tools beside a two-by-two grid of small tools.
fn modify_tab() -> RibbonTab<Message> {
    RibbonTab::new("Modify")
        .group(
            RibbonGroup::new("Edit")
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Copy)
                        .label("Duplicate")
                        .on_press(Message::Ran("Duplicate")),
                ))
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Trash2)
                        .label("Delete")
                        .on_press(Message::Ran("Delete")),
                )),
        )
        .group(
            // A grid: explicit columns of small, icon-only buttons.
            RibbonGroup::new("Transform").item(RibbonItem::grid(vec![
                vec![
                    RibbonTool::named(IconName::Move).on_press(Message::Ran("Move")),
                    RibbonTool::named(IconName::Eraser).on_press(Message::Ran("Erase")),
                ],
                vec![
                    RibbonTool::named(IconName::Group).on_press(Message::Ran("Group")),
                    RibbonTool::named(IconName::Pencil).on_press(Message::Ran("Edit")),
                ],
            ])),
        )
}

/// The "View" tab: a set of toggles whose selected ring follows the state, so
/// the tab visibly remembers the current tool.
fn view_tab(state: &RibbonState) -> RibbonTab<Message> {
    let active = state.active;
    RibbonTab::new("View")
        .group(
            RibbonGroup::new("Show")
                .item(RibbonItem::tool(
                    RibbonTool::named(IconName::Grid3x3)
                        .label("Grid")
                        .selected(true)
                        .on_press(Message::Ran("Toggle grid")),
                ))
                .item(RibbonItem::tool(
                    RibbonTool::named(IconName::Layers)
                        .label("Panels")
                        .on_press(Message::Ran("Toggle panels")),
                ))
                .item(RibbonItem::tool(
                    RibbonTool::named(IconName::Settings)
                        .label("Guides")
                        .on_press(Message::Ran("Toggle guides")),
                )),
        )
        .group(
            RibbonGroup::new("About").item(RibbonItem::labeled(
                RibbonTool::named(IconName::Info)
                    // A hint that this tab is the active one, purely cosmetic.
                    .label(if active > 0 { "Current tab" } else { "Info" })
                    .on_press(Message::Ran("About")),
            )),
        )
}

/// The floating panel hosted for an open dropdown id. The ribbon reports only
/// the id; the app decides what its ▾ actually contains.
fn panel_items(id: &str) -> Vec<MenuItem<'_, Message>> {
    match id {
        "paste" => vec![
            MenuItem::new("Keep Source Formatting", Message::Ran("Paste: keep source"))
                .shortcut("Ctrl+V"),
            MenuItem::new("Merge Formatting", Message::Ran("Paste: merge")),
            MenuItem::new("Keep Text Only", Message::Ran("Paste: text only"))
                .shortcut("Ctrl+Shift+V"),
        ],
        "layers" => vec![
            MenuItem::new("Bring to Front", Message::Ran("Bring to front")),
            MenuItem::new("Send to Back", Message::Ran("Send to back")),
            MenuItem::new("Lock Layer", Message::Ran("Lock layer")).enabled(false),
        ],
        other => vec![MenuItem::new(format!("{other} tools"), Message::Noop).enabled(false)],
    }
}
