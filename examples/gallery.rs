//! A gallery of every `iced-kit` component, with a light/dark toggle.
//!
//! Run with `cargo run --example gallery`.

use iced::widget::{column, container, row, scrollable};
use iced::{Alignment, Element, Length, Task};
use iced_kit::prelude::*;
use iced_kit::widgets::overlay::{self, Layer, Toast, ToastKind, ToastPlacement, Toasts};
use iced_kit::widgets::plot::{
    AreaChart, AreaSeries, BarAlignment, BarChart, Candle, CandlestickChart, LineChart, LineSeries,
    PieChart, PieSlice, RadarChart, RadarSeries, SankeyAlign, SankeyChart, SankeyLink, SankeyNode,
};
use iced_kit::widgets::{
    accordion, addon, alert, avatar, avatar_with_name, code, empty_state, group_button, heading,
    icon_button, input_group, kbd, muted_text, number_input, otp_input, pagination, paragraph,
    ring_progress, shortcut, skeleton, skeleton_list_item, spinner_styled, text_area, text_input,
    tooltip, AccordionSection, AddonAlignment, AvatarLabel, Button, ButtonGroup, Dropdown,
    DropdownButton, Heading, MenuItem, Modal, SkeletonShape, SpinnerStyle, Toggle, ToggleGroup,
    Tone,
};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .run()
}

/// The Markdown document shown in the gallery.
const SAMPLE_DOCUMENT: &str = "# Markdown

Rendered with the theme's own type scale: **bold**, *italic*, `code` and a
[link](https://iced.rs).

- a list item
- another item

---

| Column | Value |
| ------ | ----- |
| alpha  | 1     |
";

/// Formats a byte count the way a file browser would.
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Formats a size as a short detail line for the virtualized list.
fn format_client(size: u64) -> String {
    format!("{} on disk", format_bytes(size))
}

/// The state of the gallery.
///
/// An example naturally tracks several independent toggles at once, which is
/// exactly what the lint below warns about; splitting them into a sub-struct
/// would only add indirection to a demo.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
struct App {
    dark: bool,
    name: String,
    email: String,
    password: String,
    password_visible: bool,
    search: String,
    amount: String,
    notes: iced::widget::text_editor::Content,
    notifications: bool,
    newsletter: bool,
    plan: Plan,
    volume: f32,
    progress: f32,
    tab: usize,
    saved: Option<&'static str>,
    /// Each overlay is tracked separately so the gallery can show one at a time.
    modal_open: bool,
    toasts: Vec<&'static str>,
    dropdown_open: bool,
    /// Which members of the demo button group are selected.
    group_selection: Vec<usize>,
    /// The segmented toggle demo's state, one flag per segment.
    bold: bool,
    italic: bool,
    underline: bool,
    page: usize,
    accordion_open: Option<usize>,
    /// A counter that only exists to give the spinner something to be busy with.
    busy: bool,
    port: f64,
    otp: String,
    list_selection: usize,
    drawer_open: bool,
    menu_at: Option<(f32, f32)>,
    popover_open: bool,
    /// Scroll state for the virtualized list and table.
    list_state: VirtualListState,
    table_state: TableState,
    /// A large data set, to demonstrate virtualization.
    rows: Vec<Row>,
    /// Parsed once and rendered every frame, which is why it lives in the state
    /// rather than being built inside `view`.
    document: MarkdownDocument,
    /// Messages from the title bar's window controls, for the demo.
    last_window_action: Option<&'static str>,
    /// Split state for the resizable demo.
    #[cfg(feature = "dock")]
    splits: iced::widget::pane_grid::State<usize>,
}

/// A row of the sample data set.
#[derive(Debug, Clone)]
struct Row {
    name: String,
    size: u64,
}

/// Which plan the user picked, for the radio group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Plan {
    #[default]
    Free,
    Pro,
    Team,
}

impl Plan {
    fn label(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Pro => "Pro",
            Self::Team => "Team",
        }
    }
}

impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    NameChanged(String),
    EmailChanged(String),
    PasswordChanged(String),
    TogglePasswordVisibility,
    ClearSearch,
    SearchChanged(String),
    AmountChanged(String),
    NotesEdited(iced::widget::text_editor::Action),
    NotificationsToggled(bool),
    NewsletterToggled(bool),
    PlanPicked(Plan),
    VolumeChanged(f32),
    Advance,
    TabSelected(usize),
    Save,
    OpenModal,
    CloseModal,
    ConfirmModal,
    ShowToast,
    DismissToast(usize),
    ToggleDropdown,
    GroupSelected(Vec<usize>),
    ToggleSplitMenu,
    TogglesChanged(Vec<bool>),
    PickedFromMenu(&'static str),
    PageSelected(usize),
    AccordionToggled(usize),
    ToggleBusy,
    PortChanged(f64),
    OtpChanged(String),
    ListPicked(usize),
    ToggleDrawer,
    OpenContextMenu,
    CloseContextMenu,
    TogglePopover,
    Scrolled(VirtualListState),
    Sorted(String),
    Minimize,
    Maximize,
    CloseWindow,
}

impl Default for App {
    fn default() -> Self {
        Self {
            dark: false,
            name: String::new(),
            email: String::new(),
            password: String::new(),
            password_visible: false,
            search: String::new(),
            amount: String::new(),
            notes: iced::widget::text_editor::Content::new(),
            notifications: false,
            newsletter: false,
            plan: Plan::default(),
            volume: 50.0,
            progress: 0.0,
            tab: 0,
            saved: None,
            modal_open: false,
            toasts: Vec::new(),
            dropdown_open: false,
            // The middle option starts active, so the group shows both states.
            group_selection: vec![1],
            bold: true,
            italic: false,
            underline: true,
            page: 0,
            accordion_open: None,
            busy: false,
            port: 8080.0,
            otp: String::new(),
            list_selection: 0,
            drawer_open: false,
            menu_at: None,
            popover_open: false,
            list_state: VirtualListState::new(),
            table_state: TableState::new(),
            rows: Vec::new(),
            document: MarkdownDocument::new(),
            last_window_action: None,
            #[cfg(feature = "dock")]
            splits: Resizable::<Message>::split_state(3, SplitAxis::Horizontal, 0.25).0,
        }
    }
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // A set large enough that rendering every row would be visibly slow, so
        // the virtualization is doing real work.
        let rows: Vec<Row> = (0..5_000)
            .map(|index| Row {
                name: format!("item-{index:04}.dat"),
                size: (index as u64 * 7919) % 20_000_000,
            })
            .collect();

        (
            Self {
                // Starting in dark mode is handy for eyeballing the dark
                // palette: `GALLERY_DARK=1 cargo run --example gallery`.
                dark: std::env::var("GALLERY_DARK").is_ok_and(|v| v == "1"),
                rows,
                volume: 60.0,
                progress: 0.35,
                port: 8080.0,
                otp: "123".to_owned(),
                document: MarkdownDocument::parse(SAMPLE_DOCUMENT),
                ..Self::default()
            },
            Task::none(),
        )
    }

    // `&self` is part of the callback signature iced expects, even though the
    // window title is constant here.
    #[allow(clippy::unused_self)]
    fn title(&self) -> String {
        "iced-kit gallery".to_owned()
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
            Message::ToggleTheme => self.dark = !self.dark,
            Message::NameChanged(value) => self.name = value,
            Message::EmailChanged(value) => self.email = value,
            Message::PasswordChanged(value) => self.password = value,
            Message::TogglePasswordVisibility => {
                self.password_visible = !self.password_visible;
            }
            Message::ClearSearch => self.search.clear(),
            Message::SearchChanged(value) => self.search = value,
            Message::AmountChanged(value) => self.amount = value,
            Message::NotesEdited(action) => {
                self.notes.perform(action);
            }
            Message::NotificationsToggled(value) => self.notifications = value,
            Message::NewsletterToggled(value) => self.newsletter = value,
            Message::PlanPicked(plan) => self.plan = plan,
            Message::VolumeChanged(value) => self.volume = value,
            Message::Advance => {
                self.progress = (self.progress + 0.1).min(1.0);
            }
            Message::TabSelected(index) => self.tab = index,
            Message::Save => {
                self.saved = Some(if self.dark {
                    "Saved (dark)"
                } else {
                    "Saved (light)"
                });
            }
            Message::OpenModal => self.modal_open = true,
            Message::CloseModal => self.modal_open = false,
            Message::ConfirmModal => {
                self.modal_open = false;
                self.toasts.push("Project deleted");
            }
            Message::ShowToast => {
                // Capped so a long session of clicking cannot grow without bound.
                if self.toasts.len() < 4 {
                    self.toasts.push("Changes saved successfully");
                }
            }
            Message::DismissToast(index) => {
                if index < self.toasts.len() {
                    self.toasts.remove(index);
                }
            }
            Message::ToggleDropdown => self.dropdown_open = !self.dropdown_open,
            Message::GroupSelected(selection) => self.group_selection = selection,
            Message::ToggleSplitMenu => {
                self.saved = Some("Split menu toggled");
            }
            Message::TogglesChanged(next) => {
                // The group reports every member's state; the gallery keeps one
                // flag each so the control stays a view of the state.
                if let [bold, italic, underline] = next.as_slice() {
                    self.bold = *bold;
                    self.italic = *italic;
                    self.underline = *underline;
                }
            }
            Message::PickedFromMenu(label) => {
                self.dropdown_open = false;
                self.saved = Some(label);
            }
            Message::PageSelected(page) => self.page = page,
            Message::AccordionToggled(index) => {
                // Clicking the open section closes it, which is what makes an
                // accordion feel like a toggle rather than a selector.
                self.accordion_open = (self.accordion_open != Some(index)).then_some(index);
            }
            Message::ToggleBusy => self.busy = !self.busy,
            Message::PortChanged(value) => self.port = value,
            Message::OtpChanged(value) => self.otp = value,
            Message::ListPicked(index) => self.list_selection = index,
            Message::ToggleDrawer => self.drawer_open = !self.drawer_open,
            Message::OpenContextMenu => self.menu_at = Some((120.0, 320.0)),
            Message::CloseContextMenu => self.menu_at = None,
            Message::TogglePopover => self.popover_open = !self.popover_open,
            Message::Scrolled(state) => {
                self.list_state = state;
            }
            Message::Sorted(heading) => self.table_state.toggle_sort(heading),
            Message::Minimize => self.last_window_action = Some("Minimize"),
            Message::Maximize => self.last_window_action = Some("Maximize"),
            Message::CloseWindow => self.last_window_action = Some("Close"),
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        let content = column![
            self.header(),
            self.buttons_section(),
            self.form_section(),
            self.input_group_section(),
            self.selection_section(),
            self.display_section(),
            self.feedback_section(),
            self.overlay_section(),
            self.tabs_section(),
            self.navigation_section(),
            self.data_section(),
            self.shell_section(),
        ]
        .spacing(24)
        .padding(24);

        let page = container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill);

        // Every overlay is assembled in one place, in paint order: dropdowns
        // under the modal, toasts on top.
        let mut open = Layer::new();

        if self.dropdown_open {
            open = open.dropdown(
                Dropdown::new(vec![
                    MenuItem::new("Duplicate", Message::PickedFromMenu("Duplicated"))
                        .shortcut("Ctrl+D"),
                    MenuItem::new("Rename", Message::PickedFromMenu("Renamed")).shortcut("F2"),
                    MenuItem::new("Archive", Message::PickedFromMenu("Archived")).enabled(false),
                    MenuItem::new("Delete", Message::PickedFromMenu("Deleted")).destructive(true),
                ])
                .anchor(620.0, 250.0),
            );
        }

        if let Some(at) = self.menu_at {
            open = open.dropdown(ContextMenu::new(
                vec![
                    MenuItem::new("Cut", Message::CloseContextMenu).shortcut("Ctrl+X"),
                    MenuItem::new("Copy", Message::CloseContextMenu).shortcut("Ctrl+C"),
                    MenuItem::new("Archive", Message::CloseContextMenu).enabled(false),
                    MenuItem::new("Delete", Message::CloseContextMenu).destructive(true),
                ],
                at,
            ));
        }

        if self.popover_open {
            open = open.dropdown(Popover::new(
                column![
                    heading("Quick settings", Heading::H4),
                    paragraph("Adjust how the gallery behaves."),
                    button("Close")
                        .secondary()
                        .size(Size::Sm)
                        .on_press(Message::TogglePopover),
                ]
                .spacing(12),
                (420.0, 240.0),
            ));
        }

        if self.drawer_open {
            open = open.drawer(
                Drawer::new(
                    "Details",
                    column![
                        paragraph("A drawer slides in from an edge and leaves the page visible."),
                        text_input::<Message>("Name", &self.name),
                    ]
                    .spacing(12),
                )
                .side(DrawerSide::Right)
                .on_dismiss(Message::ToggleDrawer),
            );
        }

        if self.modal_open {
            open = open.modal(
                Modal::new(
                    "Delete project",
                    label("This permanently removes the project and all of its data."),
                    Message::CloseModal,
                )
                .cancel("Cancel", Message::CloseModal)
                .destructive("Delete", Message::ConfirmModal),
            );
        }

        let mut toasts = Toasts::new().placement(ToastPlacement::BottomRight);

        for (index, text) in self.toasts.iter().enumerate() {
            toasts = toasts.push_dismissible(
                Toast::new(*text, ToastKind::Success).description("Just now"),
                Message::DismissToast(index),
            );
        }

        open = open.toasts(toasts);

        overlay::layer(page, open)
    }

    fn header(&self) -> Element<'_, Message, Theme> {
        let theme = if self.dark {
            Theme::dark()
        } else {
            Theme::light()
        };
        let primary = theme.colors().primary;

        row![
            column![
                label("iced-kit").size(24),
                label("A gpui-kit inspired component library for iced")
                    .size(14)
                    .color(theme.colors().muted_foreground),
            ]
            .spacing(4)
            .width(Length::Fill),
            button(if self.dark { "Light mode" } else { "Dark mode" })
                .variant(ButtonVariant::Secondary)
                .on_press(Message::ToggleTheme),
        ]
        .align_y(Alignment::Center)
        .push(container(label("●").color(primary)).padding(8))
        .into()
    }

    fn buttons_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Buttons",
            column![
                row![
                    button("Primary").primary().on_press(Message::Save),
                    button("Secondary").secondary().on_press(Message::Save),
                    button("Default").on_press(Message::Save),
                    button("Ghost").ghost().on_press(Message::Save),
                    button("Link").link().on_press(Message::Save),
                    button("Destructive").destructive().on_press(Message::Save),
                ]
                .spacing(8)
                .wrap(),
                row![
                    button("Small")
                        .primary()
                        .size(Size::Sm)
                        .on_press(Message::Save),
                    button("Medium")
                        .primary()
                        .size(Size::Md)
                        .on_press(Message::Save),
                    button("Large")
                        .primary()
                        .size(Size::Lg)
                        .on_press(Message::Save),
                    button("Loading").primary().loading(true),
                    button("Disabled").primary().on_press_maybe(None),
                ]
                .spacing(8)
                .wrap(),
                row![
                    button("Danger").danger().on_press(Message::Save),
                    button("Warning").warning().on_press(Message::Save),
                    button("Success").success().on_press(Message::Save),
                    button("Info").info().on_press(Message::Save),
                    button("Text").text().on_press(Message::Save),
                ]
                .spacing(8)
                .wrap(),
                row![
                    button("Outline").outline().on_press(Message::Save),
                    button("Primary outline")
                        .primary()
                        .outline()
                        .on_press(Message::Save),
                    button("Danger outline")
                        .danger()
                        .outline()
                        .on_press(Message::Save),
                    button("Selected")
                        .primary()
                        .selected(true)
                        .on_press(Message::Save),
                    button("Disabled state")
                        .primary()
                        .disabled(true)
                        .on_press(Message::Save),
                ]
                .spacing(8)
                .wrap(),
                row![
                    button("With icon")
                        .icon("★")
                        .primary()
                        .on_press(Message::Save),
                    button("Menu").dropdown_caret().on_press(Message::Save),
                    icon_button::<Message>()
                        .icon("✕")
                        .ghost()
                        .on_press(Message::Save),
                    icon_button::<Message>()
                        .icon("＋")
                        .outline()
                        .on_press(Message::Save),
                    button("Compact")
                        .secondary()
                        .compact()
                        .on_press(Message::Save),
                ]
                .spacing(8)
                .wrap(),
                column![
                    label("Button group").size(13),
                    ButtonGroup::new()
                        .push(Button::new("Day").selected(self.group_selection.contains(&0)))
                        .push(Button::new("Week").selected(self.group_selection.contains(&1)))
                        .push(Button::new("Month").selected(self.group_selection.contains(&2)))
                        .on_select(Message::GroupSelected),
                ]
                .spacing(6),
                column![
                    label("Split button").size(13),
                    row![
                        DropdownButton::new(Button::new("Save").primary().on_press(Message::Save))
                            .on_toggle(Message::ToggleSplitMenu),
                        DropdownButton::new(Button::new("Save").on_press(Message::Save))
                            .outline()
                            .on_toggle(Message::ToggleSplitMenu),
                    ]
                    .spacing(12),
                ]
                .spacing(6),
                column![
                    label("Segmented toggles").size(13),
                    ToggleGroup::new()
                        .push(Toggle::new("Bold").checked(self.bold))
                        .push(Toggle::new("Italic").checked(self.italic))
                        .push(Toggle::new("Underline").checked(self.underline))
                        .segmented()
                        .on_change(Message::TogglesChanged),
                ]
                .spacing(6),
                label(self.saved.unwrap_or(""))
                    .size(13)
                    .color(Theme::light().colors().muted_foreground),
            ]
            .spacing(12),
        )
    }

    fn form_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Form fields",
            column![
                row![
                    text_input::<Message>("Ada Lovelace", &self.name)
                        .label("Name")
                        .on_input(Message::NameChanged)
                        .width(220),
                    text_input::<Message>("you@example.com", &self.email)
                        .label("Email")
                        .on_input(Message::EmailChanged)
                        .width(220),
                    text_input::<Message>("••••••••", &self.password)
                        .label("Password")
                        .password(true)
                        .masked(self.password_visible)
                        .on_mask_toggle(Message::TogglePasswordVisibility)
                        .on_input(Message::PasswordChanged)
                        .width(220),
                ]
                .spacing(16)
                .wrap(),
                row![
                    text_input::<Message>("Search…", &self.search)
                        .label("Search")
                        .clearable(Message::ClearSearch)
                        .on_input(Message::SearchChanged)
                        .width(220),
                    text_input::<Message>("https://example.com", &self.email)
                        .label("Site")
                        .prefix(label("https://").size(13))
                        .on_input(Message::EmailChanged)
                        .width(260),
                    text_input::<Message>("Required", "")
                        .label("Validated")
                        .error("This field is required")
                        .width(220),
                ]
                .spacing(16)
                .wrap(),
                row![
                    number_input("Port", 8080.0, 1.0..=65535.0, Message::PortChanged),
                    otp_input(&self.otp, 6, Message::OtpChanged).groups(2),
                ]
                .spacing(16)
                .align_y(iced::Alignment::Start)
                .wrap(),
                column![text_area::<Message>("Write something…", &self.notes)
                    .label("Notes")
                    .on_edit(Message::NotesEdited),]
                .spacing(4),
            ]
            .spacing(16),
        )
    }

    fn input_group_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Input groups",
            column![
                input_group()
                    .input(
                        text_input::<Message>("example.com", &self.email)
                            .on_input(Message::EmailChanged),
                    )
                    .addon(addon().push(label("https://").size(13)))
                    .width(320),
                input_group()
                    .input(
                        text_input::<Message>("Search…", &self.search)
                            .on_input(Message::SearchChanged),
                    )
                    .addon(
                        addon()
                            .align(AddonAlignment::InlineEnd)
                            .push(group_button::<Message>("Go").on_press(Message::Save)),
                    )
                    .width(320),
                input_group()
                    .input(
                        text_input::<Message>("0.00", &self.amount)
                            .on_input(Message::AmountChanged),
                    )
                    .addon(addon().push(label("$").size(13)))
                    .addon(
                        addon()
                            .align(AddonAlignment::InlineEnd)
                            .push(label("USD").size(12))
                    )
                    .width(320),
                input_group()
                    .input(
                        text_input::<Message>("4242 4242 4242 4242", "")
                            .invalid(true)
                            .on_input(Message::NameChanged),
                    )
                    .addon(
                        addon()
                            .align(AddonAlignment::BlockStart)
                            .push(label("Card that expired last month").size(11)),
                    )
                    .addon(
                        addon()
                            .align(AddonAlignment::InlineEnd)
                            .push(label("@").size(12))
                    )
                    .invalid(true)
                    .width(320),
            ]
            .spacing(14),
        )
    }

    fn selection_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Selection",
            column![
                row![
                    checkbox(
                        "Notifications",
                        self.notifications,
                        Message::NotificationsToggled
                    ),
                    switch("Newsletter", self.newsletter, Message::NewsletterToggled),
                ]
                .spacing(24)
                .align_y(Alignment::Center),
                row![
                    column![
                        label("Plan").size(13),
                        row![
                            radio("Free", Plan::Free, Some(self.plan), Message::PlanPicked),
                            radio("Pro", Plan::Pro, Some(self.plan), Message::PlanPicked),
                            radio("Team", Plan::Team, Some(self.plan), Message::PlanPicked),
                        ]
                        .spacing(24),
                    ]
                    .spacing(4),
                    column![
                        label("Plan (select)").size(13),
                        select(
                            vec![Plan::Free, Plan::Pro, Plan::Team],
                            Some(self.plan),
                            Message::PlanPicked
                        )
                        .placeholder("Choose a plan")
                        .width(160),
                    ]
                    .spacing(4),
                    column![
                        label("Plan (select_fill)").size(13),
                        // `select_fill` expands to its container, so the
                        // wrapper bounds it.
                        iced::widget::container(select_fill(
                            vec![Plan::Free, Plan::Pro, Plan::Team],
                            Some(self.plan),
                            Message::PlanPicked,
                        ))
                        .width(Length::Fixed(180.0)),
                    ]
                    .spacing(4),
                ]
                .spacing(32)
                .wrap(),
                column![
                    label(format!("Volume: {:.0}%", self.volume)).size(13),
                    slider(0.0..=100.0, self.volume, Message::VolumeChanged)
                        .width(Length::Fixed(320.0)),
                ]
                .spacing(4),
                label(format!("Selected plan: {}", self.plan.label()))
                    .size(13)
                    .color(Theme::light().colors().muted_foreground),
            ]
            .spacing(16),
        )
    }

    fn display_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Display",
            column![
                row![
                    badge("Default", Tone::Neutral),
                    badge("Success", Tone::Success),
                    badge("Warning", Tone::Warning),
                    badge("Danger", Tone::Danger),
                    badge("Primary", Tone::Primary),
                ]
                .spacing(8),
                // Both divider orientations, side by side so the difference
                // between them is visible at a glance.
                row![label("left"), vertical_divider(), label("right"),]
                    .spacing(12)
                    .height(Length::Fixed(24.0))
                    .align_y(Alignment::Center),
                divider(),
                column![
                    label(format!("Progress: {:.0}%", self.progress * 100.0)).size(13),
                    progress(self.progress, Tone::Primary),
                    button("Advance")
                        .secondary()
                        .size(Size::Sm)
                        .on_press(Message::Advance),
                ]
                .spacing(8),
                divider(),
                alert(
                    "Heads up",
                    "This is what an informational alert looks like.",
                    Tone::Neutral,
                ),
                alert("Saved", "Your changes were written to disk.", Tone::Success),
                alert(
                    "Disk almost full",
                    "Only 2 GB of free space remains.",
                    Tone::Warning,
                ),
                alert(
                    "Build failed",
                    "3 errors were found in 2 files.",
                    Tone::Danger
                ),
                empty_state("No projects yet", "Create one to get started"),
            ]
            .spacing(16),
        )
    }

    fn feedback_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Feedback",
            column![
                row![
                    column![
                        label("Spinner (arc)").size(13),
                        spinner_styled(28, SpinnerStyle::Arc),
                    ]
                    .spacing(8)
                    .align_x(Alignment::Center),
                    column![
                        label("Spinner (dots)").size(13),
                        spinner_styled(28, SpinnerStyle::Dots),
                    ]
                    .spacing(8)
                    .align_x(Alignment::Center),
                    column![
                        label("Ring").size(13),
                        ring_progress(0.65, 64, Tone::Primary),
                    ]
                    .spacing(8)
                    .align_x(Alignment::Center),
                    column![
                        label("Ring (danger)").size(13),
                        ring_progress(0.3, 64, Tone::Danger),
                    ]
                    .spacing(8)
                    .align_x(Alignment::Center),
                    button(if self.busy { "Idle" } else { "Busy" })
                        .secondary()
                        .size(Size::Sm)
                        .on_press(Message::ToggleBusy),
                ]
                .spacing(32)
                .align_y(Alignment::Center)
                .wrap(),
                divider(),
                row![
                    if self.busy {
                        spinner_styled(16, SpinnerStyle::Arc)
                    } else {
                        label("Ready").into()
                    },
                    label(format!("Busy state: {}", self.busy)).size(13),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
                divider(),
                column![
                    label("Skeleton (text)").size(13),
                    skeleton(SkeletonShape::Text, 3)
                ]
                .spacing(8),
                column![
                    label("Skeleton (list item)").size(13),
                    skeleton_list_item(2)
                ]
                .spacing(8),
                column![label("Skeleton (table)").size(13), skeleton_table(3, 2)].spacing(8),
            ]
            .spacing(16),
        )
    }

    fn overlay_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Overlays",
            column![
                row![
                    button("Open modal").primary().on_press(Message::OpenModal),
                    button("Open drawer")
                        .secondary()
                        .on_press(Message::ToggleDrawer),
                    button("Show toast")
                        .secondary()
                        .on_press(Message::ShowToast),
                    button("Dropdown")
                        .secondary()
                        .on_press(Message::ToggleDropdown),
                    button("Context menu")
                        .secondary()
                        .on_press(Message::OpenContextMenu),
                    button("Popover")
                        .secondary()
                        .on_press(Message::TogglePopover),
                    tooltip(
                        button("Hover me").ghost().on_press(Message::Save),
                        "A tooltip appears after a short delay".to_owned(),
                    ),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .wrap(),
                label(format!("Open toasts: {}", self.toasts.len())).size(13),
            ]
            .spacing(12),
        )
    }

    fn shell_section(&self) -> Element<'_, Message, Theme> {
        let action = self
            .last_window_action
            .map_or_else(|| "none yet".to_owned(), str::to_owned);

        let title_bar = TitleBar::new("iced-kit")
            .icon("◆")
            .subtitle("shell")
            .control(WindowControl::Minimize, Message::Minimize)
            .control(WindowControl::Maximize, Message::Maximize)
            .control(WindowControl::Close, Message::CloseWindow);

        #[cfg(feature = "dock")]
        let panes: Element<'_, Message, Theme> = Resizable::new(&self.splits)
            .min_size(60.0)
            .on_resize(|_event| Message::Save)
            .pane(|_pane, index: &usize| {
                iced_kit::widgets::dock::pane_body(label(format!("Pane {}", index + 1))).into()
            })
            .into_element();

        #[cfg(not(feature = "dock"))]
        let panes: Element<'_, Message, Theme> = empty_state(
            "Split panes need the `dock` feature",
            "Run with --features dock to see them",
        );

        Self::section(
            "Shell",
            column![
                column![label("Title bar").size(13), title_bar],
                label(format!("Last window action: {action}")).size(13),
                divider(),
                column![label("Resizable panes").size(13), panes].spacing(8),
            ]
            .spacing(12),
        )
    }

    fn data_section(&self) -> Element<'_, Message, Theme> {
        let columns = vec![
            Column::<Row, Message>::new("Name", |row: &Row, _index| {
                iced_kit::widgets::data_table::text_cell(row.name.clone())
            })
            .width(Width::Fill)
            .sortable(|row| SortKey::text(row.name.clone())),
            Column::<Row, Message>::new("Size", |row: &Row, _index| {
                iced_kit::widgets::data_table::number_cell(format_bytes(row.size))
            })
            .width(Width::Fixed(120.0))
            .align_x(Alignment::End)
            .sortable(|row| SortKey::number(row.size as f64)),
        ];

        let samples: Vec<f64> = (0..32)
            .map(|index| {
                let t = f64::from(index);
                (t * 0.4).sin() * 30.0 + 55.0 + (t * 1.7).cos() * 6.0
            })
            .collect();

        let writes: Vec<f64> = samples.iter().map(|value| value * 0.55 + 12.0).collect();

        Self::section(
            "Data",
            column![
                column![
                    label(format!(
                        "Virtual table ({} rows, only the visible ones are built)",
                        self.rows.len()
                    ))
                    .size(13),
                    DataTable::new(&self.rows, &self.table_state, columns)
                        .row_height(30.0)
                        .max_height(240.0)
                        .on_sort(Message::Sorted)
                        .on_row_click(|_index| Message::Save),
                ]
                .spacing(8),
                divider(),
                column![
                    label("Virtual list (variable row heights)").size(13),
                    VirtualList::new(&self.rows, &self.list_state, |row: &Row, index| {
                        let detail = if index % 3 == 0 {
                            Some(format_client(row.size))
                        } else {
                            None
                        };
                        iced_kit::widgets::virtual_list::text_row(row.name.clone(), detail)
                    })
                    .row_height(|_row: &Row, index| if index % 3 == 0 { 46.0 } else { 28.0 })
                    .overscan(3)
                    .height(200.0)
                    .on_scroll(Message::Scrolled)
                    .into_element()
                ]
                .spacing(8),
                divider(),
                column![
                    label("Line").size(13),
                    LineChart::new(
                        (0..24).map(|index| format!("{index}h")).collect(),
                        vec![
                            LineSeries::new("Reads", samples.clone()).tone(Tone::Primary),
                            LineSeries::new("Writes", writes.clone()).tone(Tone::Success),
                        ],
                    )
                    .tooltip(false)
                    .height(170.0)
                    .into_element::<Message>(),
                ]
                .spacing(8),
                column![
                    label("Area (stacked)").size(13),
                    AreaChart::new(
                        (0..12).map(|index| format!("{index}")).collect(),
                        vec![
                            AreaSeries::new("Reads", samples.iter().take(12).copied().collect())
                                .tone(Tone::Primary),
                            AreaSeries::new("Writes", writes.iter().take(12).copied().collect())
                                .tone(Tone::Success),
                        ],
                    )
                    .stacked(true)
                    .tooltip(false)
                    .height(170.0)
                    .into_element::<Message>(),
                ]
                .spacing(8),
                column![
                    label("Bar (horizontal)").size(13),
                    BarChart::single(
                        "Sessions",
                        ["Search", "Direct", "Social", "Email", "Referral"]
                            .iter()
                            .map(|s| (*s).to_owned())
                            .collect(),
                        vec![420.0, 310.0, 180.0, 95.0, 60.0],
                    )
                    .alignment(BarAlignment::Left)
                    .tooltip(false)
                    .height(170.0)
                    .into_element::<Message>(),
                ]
                .spacing(8),
                row![
                    column![
                        label("Pie").size(13),
                        PieChart::new(vec![
                            PieSlice::new("Rust", 45.0).tone(Tone::Primary),
                            PieSlice::new("Go", 25.0).tone(Tone::Success),
                            PieSlice::new("Python", 20.0).tone(Tone::Warning),
                            PieSlice::new("Other", 10.0).tone(Tone::Danger),
                        ])
                        .tooltip(false)
                        .height(200.0)
                        .into_element::<Message>(),
                    ]
                    .spacing(8)
                    .width(Length::FillPortion(1)),
                    column![
                        label("Donut").size(13),
                        PieChart::new(vec![
                            PieSlice::new("Used", 68.0).tone(Tone::Primary),
                            PieSlice::new("Free", 32.0).tone(Tone::Neutral),
                        ])
                        .donut(true)
                        .labels(false)
                        .tooltip(false)
                        .height(200.0)
                        .into_element::<Message>(),
                    ]
                    .spacing(8)
                    .width(Length::FillPortion(1)),
                    column![
                        label("Radar").size(13),
                        RadarChart::new(
                            ["Speed", "Power", "Range", "Accuracy", "Cost"]
                                .iter()
                                .map(|s| (*s).to_owned())
                                .collect(),
                            vec![
                                RadarSeries::new("A", vec![80.0, 65.0, 90.0, 70.0, 40.0])
                                    .tone(Tone::Primary),
                                RadarSeries::new("B", vec![60.0, 85.0, 50.0, 95.0, 70.0])
                                    .tone(Tone::Success),
                            ],
                        )
                        .max_value(100.0)
                        .tooltip(false)
                        .height(200.0)
                        .into_element::<Message>(),
                    ]
                    .spacing(8)
                    .width(Length::FillPortion(1)),
                ]
                .spacing(12),
                column![
                    label("Candlestick").size(13),
                    CandlestickChart::new(
                        (1..=20).map(|d| d.to_string()).collect(),
                        (0..20)
                            .map(|index| {
                                let drift = ((f64::from(index)) * 0.7).sin() * 4.0;
                                let open = 100.0 + f64::from(index) * 0.4;
                                let close = open + drift;
                                Candle::new(
                                    open,
                                    open.max(close) + 2.5,
                                    open.min(close) - 2.5,
                                    close,
                                )
                            })
                            .collect(),
                    )
                    .tooltip(false)
                    .height(200.0)
                    .into_element::<Message>(),
                ]
                .spacing(8),
                column![
                    label("Sankey").size(13),
                    SankeyChart::new(
                        vec![
                            SankeyNode::new("Coal").tone(Tone::Primary),
                            SankeyNode::new("Gas").tone(Tone::Warning),
                            SankeyNode::new("Solar").tone(Tone::Success),
                            SankeyNode::new("Electricity").tone(Tone::Primary),
                            SankeyNode::new("Industry").tone(Tone::Danger),
                            SankeyNode::new("Homes").tone(Tone::Success),
                            SankeyNode::new("Losses").tone(Tone::Neutral),
                        ],
                        vec![
                            SankeyLink::new(0, 3, 40.0),
                            SankeyLink::new(1, 3, 25.0),
                            SankeyLink::new(2, 3, 18.0),
                            SankeyLink::new(3, 4, 45.0),
                            SankeyLink::new(3, 5, 22.0),
                            SankeyLink::new(3, 6, 16.0),
                        ],
                    )
                    .align(SankeyAlign::Justify)
                    .tooltip(false)
                    .height(240.0)
                    .into_element::<Message>(),
                ]
                .spacing(8),
                divider(),
                column![
                    label("Markdown").size(13),
                    self.document
                        .into_element()
                        .map(|_url: String| Message::Save),
                ]
                .spacing(8),
            ]
            .spacing(16),
        )
    }

    fn navigation_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Navigation & data",
            column![
                column![
                    label("Pagination").size(13),
                    pagination(self.page, 12, Message::PageSelected),
                ]
                .spacing(8),
                divider(),
                column![
                    label("Accordion").size(13),
                    accordion(
                        vec![
                            AccordionSection::new("General").subtitle("Basic options"),
                            AccordionSection::new("Advanced").subtitle("For experts"),
                            AccordionSection::new("Danger zone"),
                        ],
                        self.accordion_open,
                        Message::AccordionToggled,
                        |index| label(format!("Body content for section {index}.")).into(),
                    ),
                ]
                .spacing(8),
                divider(),
                column![
                    label("Avatars").size(13),
                    row![
                        avatar("Ada Lovelace", 32),
                        avatar("Grace Hopper", 40),
                        avatar("Alan Turing", 48),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                    avatar_with_name(
                        AvatarLabel::new("Ada Lovelace").detail("ada@example.com"),
                        40,
                        Size::Md,
                    ),
                    row![
                        avatar_with_label("AL", 28),
                        label("Precomputed initials").size(13),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(12),
                divider(),
                column![
                    label("Number input").size(13),
                    number_input("Port", self.port, 1.0..=65535.0, Message::PortChanged),
                ]
                .spacing(6),
                column![
                    label("One-time code").size(13),
                    otp_input(&self.otp, 6, Message::OtpChanged),
                ]
                .spacing(6),
                divider(),
                column![
                    label("List").size(13),
                    list(
                        vec![
                            ListItem::new("Inbox").trailing("12"),
                            ListItem::new("Drafts").subtitle("3 unsent"),
                            ListItem::new("Archive").enabled(false),
                        ],
                        Some(self.list_selection),
                        Message::ListPicked,
                    ),
                ]
                .spacing(6),
                column![
                    label("Tags").size(13),
                    row![
                        tag("rust", Tone::Primary, None),
                        tag("iced", Tone::Success, Some(Message::Save)),
                        tag("deprecated", Tone::Danger, None),
                    ]
                    .spacing(8),
                ]
                .spacing(6),
                divider(),
                column![
                    label("Typography").size(13),
                    heading("Heading 2", Heading::H2),
                    paragraph("A paragraph of body text at the default size."),
                    muted_text("Muted helper text under a field."),
                    row![
                        code("cargo run"),
                        kbd::<Message>("Ctrl"),
                        shortcut::<Message>(&["Ctrl", "Shift", "P"]),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .wrap(),
                ]
                .spacing(8),
            ]
            .spacing(16),
        )
    }

    fn tabs_section(&self) -> Element<'_, Message, Theme> {
        let body: Element<'_, Message, Theme> = match self.tab {
            0 => label("The first tab's content lives here.").into(),
            1 => label("The second tab's content lives here.").into(),
            _ => label("The third tab's content lives here.").into(),
        };

        Self::section(
            "Tabs",
            column![
                tabs(
                    vec![
                        Tab::new("General"),
                        Tab::new("Advanced"),
                        Tab::new("Disabled").enabled(false),
                    ],
                    self.tab,
                    Message::TabSelected,
                ),
                divider(),
                container(body).padding(8),
            ]
            .spacing(12),
        )
    }

    /// Wraps a section's content in a titled card.
    fn section<'a>(
        title: &'a str,
        content: impl Into<Element<'a, Message, Theme>>,
    ) -> Element<'a, Message, Theme> {
        card(column![label(title).size(18), divider(), content.into()].spacing(12))
            .width(Length::Fill)
            .into()
    }
}
