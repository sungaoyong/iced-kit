//! A gallery of every `iced-kit` component, with a light/dark toggle.
//!
//! Run with `cargo run --example gallery`.

use iced::widget::{column, container, row, scrollable, stack, text};
use iced::{Alignment, Color, Element, Length, Padding, Task};
use iced_kit::i18n::{FluentTranslator, I18n};
use iced_kit::icons::IconName;
use iced_kit::motion::Presence;
use iced_kit::prelude::*;
use iced_kit::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsEvent, SettingsState,
};
use iced_kit::widgets::overlay::{self, Layer, Toast, ToastKind, ToastPlacement, Toasts};
use iced_kit::widgets::plot::{
    AreaChart, AreaSeries, BarAlignment, BarChart, Candle, CandlestickChart, LineChart, LineSeries,
    PieChart, PieSlice, RadarChart, RadarSeries, SankeyAlign, SankeyChart, SankeyLink, SankeyNode,
};
use iced_kit::widgets::{
    accordion, addon, alert, avatar, avatar_with_name, carousel, code, empty_state, group_button,
    heading, icon_button, input_group, kbd, muted_text, number_input, otp_input, pagination,
    paragraph, ring_progress, shortcut, sidebar, skeleton, skeleton_list_item, spinner_styled,
    text_area, text_input, tooltip, AccordionSection, AddonAlignment, AlertDialog, AlertTone,
    AvatarLabel, Button, ButtonGroup, CarouselAxis, CarouselState, DialogContent, DialogFooter,
    DialogHeader, Dropdown, DropdownButton, Heading, Icon, MenuItem, Modal, SidebarCollapsible,
    SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem, SidebarToggleButton,
    SkeletonShape, SpinnerStyle, Toggle, ToggleGroup, Tone,
};
use iced_kit::widgets::{group_box, GroupBoxVariant};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .run()
}

/// The gap between a hosted panel and the trigger it dropped from.
const PANEL_GAP: f32 = 8.0;

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

/// A page of the gallery, shown one at a time in the demo pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Buttons,
    Form,
    FormLayout,
    NewComponents,
    Chat,
    InputGroups,
    Selection,
    Display,
    Feedback,
    Overlays,
    Tabs,
    Ribbon,
    Navigation,
    Carousel,
    Data,
    Settings,
    GroupBoxes,
    Sidebar,
    Shell,
}

impl Section {
    /// Every page, in the order the nav lists them.
    const ALL: &'static [Section] = &[
        Section::Buttons,
        Section::Form,
        Section::FormLayout,
        Section::NewComponents,
        Section::Chat,
        Section::InputGroups,
        Section::Selection,
        Section::Display,
        Section::Feedback,
        Section::Overlays,
        Section::Tabs,
        Section::Ribbon,
        Section::Navigation,
        Section::Carousel,
        Section::Data,
        Section::Settings,
        Section::GroupBoxes,
        Section::Sidebar,
        Section::Shell,
    ];

    /// The name the nav shows.
    fn label(self) -> &'static str {
        match self {
            Section::Buttons => "Buttons",
            Section::Form => "Form fields",
            Section::FormLayout => "Form layout",
            Section::NewComponents => "Combos, dates & color",
            Section::Chat => "Chat",
            Section::InputGroups => "Input groups",
            Section::Selection => "Selection",
            Section::Display => "Display",
            Section::Feedback => "Feedback",
            Section::Overlays => "Overlays",
            Section::Tabs => "Tabs & carousel",
            Section::Ribbon => "Ribbon",
            Section::Navigation => "Navigation & data",
            Section::Carousel => "Carousel",
            Section::Data => "Data",
            Section::Settings => "Settings",
            Section::GroupBoxes => "Group boxes",
            Section::Sidebar => "Sidebar",
            Section::Shell => "Shell",
        }
    }
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
    /// The form layout demo's bio, edited through the text area.
    bio: iced::widget::text_editor::Content,
    /// The new-components demo's file tree, expanded once at construction.
    file_tree: iced_kit::widgets::Tree,
    /// The tree's items, kept beside its state so the borrow lives as long as
    /// the demo does.
    file_items: Vec<iced_kit::widgets::TreeItem>,
    /// The combobox demo's options, kept for the same reason.
    country_options: Vec<iced_kit::widgets::ComboBoxOption<usize>>,
    /// Which page the demo pane shows.
    selected: Section,
    /// The ribbon demo's view state and the anchor of its hosted dropdown.
    ribbon: RibbonState,
    ribbon_anchor: Option<iced::Rectangle>,
    /// The split components' open panel, and the window-space rectangle of
    /// the trigger that opened it. Each component reports an intent and the
    /// gallery hosts its panel, the same division of labour as every overlay
    /// in the crate; the anchor comes from `trigger`, which reports where the
    /// pressed trigger sits.
    open_panel: Option<PanelKind>,
    panel_anchor: Option<iced::Rectangle>,
    /// The combobox's query, carried by its trigger's text field.
    combo_query: String,
    /// The countries the combobox shows as picked.
    picked_countries: Vec<usize>,
    /// The date the picker's field shows, as typed or picked.
    picked_date: String,
    /// The color the picker's swatch shows.
    picked_color: Color,
    /// The chat demo's transcript and its scroll state, both caller-owned.
    chat: MessageScrollerState,
    chat_lines: Vec<String>,
    notifications: bool,
    newsletter: bool,
    plan: Plan,
    volume: f32,
    progress: f32,
    tab: usize,
    saved: Option<&'static str>,
    /// Each overlay is tracked separately so the gallery can show one at a time.
    modal_open: bool,
    /// The alert dialog and the hand-assembled one, likewise one at a time.
    alert_open: bool,
    assembled_open: bool,
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
    /// The carousel demo's selection, in both orientations.
    carousel: CarouselState,
    carousel_vertical: CarouselState,
    /// The English translator for the calendar widget.
    i18n: I18n<FluentTranslator>,
    /// A counter that only exists to give the spinner something to be busy with.
    busy: bool,
    port: f64,
    otp: String,
    list_selection: usize,
    drawer_open: bool,
    /// Keeps the drawer mounted while its exit is drawn.
    drawer_presence: Presence,
    /// The Overlays section's anchored demos: where the pressed trigger
    /// sits, reported by `trigger`. The sidebar item's context menu keeps a
    /// hand-placed point, because a sidebar item is not reachable from
    /// outside the widget to be wrapped.
    dropdown_anchor: Option<iced::Rectangle>,
    menu_at: Option<iced::Rectangle>,
    popover_anchor: Option<iced::Rectangle>,
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
    /// The sidebar demo's state, all of it owned by the caller.
    sidebar_collapsed: bool,
    sidebar_collapsible: SidebarCollapsible,
    sidebar_active: &'static str,
    sidebar_projects_open: bool,
    sidebar_menu_at: Option<(f32, f32)>,
    /// The draggable widths of the sidebar and settings demos.
    sidebar_width: f32,
    settings_sidebar_width: f32,
    /// Split state for the resizable demo.
    #[cfg(feature = "dock")]
    splits: iced::widget::pane_grid::State<usize>,
    /// The settings demo's panel state, and the values its fields edit.
    settings: SettingsState,
    settings_dark_mode: bool,
    settings_autosave: bool,
    settings_font_size: f64,
    settings_font_family: String,
    settings_accent: String,
    settings_telemetry: bool,
    settings_launch_at_login: bool,
}

/// Which of the split components' panels is open.
///
/// One at a time: opening a panel closes whichever one was open, the way a
/// popover family behaves — two floating surfaces at once reads as a fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelKind {
    Combo,
    Date,
    Color,
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
    /// A press the demo does not act on: some sections show what controls
    /// look like rather than wiring every one to state.
    Noop,
    /// The demo pane's page selection.
    SectionPicked(Section),
    /// The file tree's events: the gallery holds the tree's state, so the
    /// disclosure and selection changes come back through here to be applied.
    TreeEvent(iced_kit::widgets::TreeEvent),
    /// The split components' open intents.
    ToggleCombo,
    ToggleDatePicker,
    ToggleColorPicker,
    /// Where a pressed trigger sits, published by `trigger` the moment a
    /// press lands on it. It arrives ahead of the toggle message, so the
    /// panel the toggle opens is anchored to the press that opened it.
    PanelAnchor(iced::Rectangle),
    /// A press outside an open panel: the popover closes instead of the
    /// control that was clicked.
    ClosePanel,
    /// The combobox's query, and the country picked from its panel.
    ComboQuery(String),
    CountryPicked(usize),
    /// The date field's text, and a date picked from the calendar.
    DateText(String),
    DatePicked(iced_kit::widgets::date::Date),
    /// The color picker's swatch, changed from its panel.
    ColorChanged(Color),
    NameChanged(String),
    EmailChanged(String),
    PasswordChanged(String),
    TogglePasswordVisibility,
    ClearSearch,
    SearchChanged(String),
    AmountChanged(String),
    NotesEdited(iced::widget::text_editor::Action),
    BioEdited(iced::widget::text_editor::Action),
    NotificationsToggled(bool),
    NewsletterToggled(bool),
    PlanPicked(Plan),
    VolumeChanged(f32),
    Advance,
    TabSelected(usize),
    /// The ribbon demo's tab pick, its dropdown open intent, and the anchor
    /// its trigger reports. A press outside the hosted panel closes it.
    RibbonTabSelected(usize),
    RibbonDropdown(String),
    RibbonAnchor(iced::Rectangle),
    RibbonCollapse(CollapseMode),
    CloseRibbonPanel,
    Save,
    OpenModal,
    CloseModal,
    ConfirmModal,
    OpenAlert,
    CloseAlert,
    OpenAssembled,
    CloseAssembled,
    ShowToast,
    DismissToast(usize),
    ToggleDropdown,
    /// The dropdown menu's catcher was pressed: a click outside the menu
    /// closes it instead of reaching the page beneath.
    CloseDropdown,
    /// Where the Overlays section's pressed triggers sit.
    DropdownAnchor(iced::Rectangle),
    ContextMenuAnchor(iced::Rectangle),
    PopoverAnchor(iced::Rectangle),
    GroupSelected(Vec<usize>),
    ToggleSplitMenu,
    TogglesChanged(Vec<bool>),
    PickedFromMenu(&'static str),
    PageSelected(usize),
    /// Navigation and search inside the settings panel.
    Settings(SettingsEvent),
    /// The settings panel asked for a reset; the index is the page it came from.
    SettingsReset(usize),
    SettingsDarkMode(bool),
    SettingsAutosave(bool),
    SettingsFontSize(f64),
    SettingsFontFamily(String),
    SettingsAccent(String),
    SettingsTelemetry(bool),
    SettingsLaunchAtLogin(bool),
    AccordionToggled(usize),
    /// The carousel demos report the slide they moved to.
    CarouselSelected(usize),
    CarouselVerticalSelected(usize),
    ToggleBusy,
    PortChanged(f64),
    OtpChanged(String),
    ListPicked(usize),
    ToggleDrawer,
    /// The drawer's backdrop was pressed. It is deliberately not the toggle:
    /// a press that lands while the drawer's exit is still being drawn must
    /// not flip the drawer back open.
    DrawerDismissed,
    /// The drawer's exit has finished drawing. The message itself does
    /// nothing; the view rebuild around it is what removes the drawer — and
    /// its backdrop — from the screen.
    DrawerExitDrawn,
    CloseContextMenu,
    TogglePopover,
    Scrolled(VirtualListState),
    Sorted(String),
    Minimize,
    Maximize,
    CloseWindow,
    /// The sidebar demo: collapse toggle, mode switcher, item selection, the
    /// submenu caret and the item's right-click intent.
    SidebarToggled,
    SidebarModePicked(SidebarCollapsible),
    SidebarPicked(&'static str),
    SidebarSubmenuToggled,
    SidebarContextOpened,
    /// The sidebar and the settings panel report a new width while dragged.
    SidebarResized(f32),
    SettingsSidebarResized(f32),
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
            bio: iced::widget::text_editor::Content::new(),
            file_items: file_tree_items(),
            chat: MessageScrollerState::new(chat_lines().len()),
            chat_lines: chat_lines(),
            country_options: vec![
                iced_kit::widgets::ComboBoxOption::new(0, "Germany"),
                iced_kit::widgets::ComboBoxOption::new(1, "Ghana"),
                iced_kit::widgets::ComboBoxOption::new(2, "Japan").disabled(true),
            ],
            selected: Section::Buttons,
            ribbon: RibbonState::new(),
            ribbon_anchor: None,
            open_panel: None,
            panel_anchor: None,
            combo_query: String::new(),
            picked_countries: vec![2],
            picked_date: "2024-02-14".to_owned(),
            picked_color: Color::from_rgb8(0x33, 0x66, 0x99),
            file_tree: {
                let mut tree = iced_kit::widgets::Tree::new().items(file_tree_items());
                tree.expand_all();
                tree
            },
            notifications: false,
            newsletter: false,
            plan: Plan::default(),
            volume: 50.0,
            progress: 0.0,
            tab: 0,
            saved: None,
            modal_open: false,
            alert_open: false,
            assembled_open: false,
            toasts: Vec::new(),
            dropdown_open: false,
            // The middle option starts active, so the group shows both states.
            group_selection: vec![1],
            bold: true,
            italic: false,
            underline: true,
            page: 0,
            accordion_open: None,
            // Looping is on so the demo shows the wrap-around, which is the
            // part of the behaviour a still screenshot cannot otherwise convey.
            carousel: CarouselState::new(4).with_looping(true),
            carousel_vertical: CarouselState::new(3).with_axis(CarouselAxis::Vertical),
            busy: false,
            port: 8080.0,
            otp: String::new(),
            list_selection: 0,
            drawer_open: false,
            drawer_presence: Presence::new(),
            menu_at: None,
            dropdown_anchor: None,
            popover_anchor: None,
            popover_open: false,
            list_state: VirtualListState::new(),
            table_state: TableState::new(),
            rows: Vec::new(),
            document: MarkdownDocument::new(),
            last_window_action: None,
            sidebar_collapsed: false,
            sidebar_collapsible: SidebarCollapsible::Icon,
            sidebar_active: "Dashboard",
            sidebar_projects_open: true,
            sidebar_menu_at: None,
            sidebar_width: 240.0,
            settings_sidebar_width: 220.0,
            #[cfg(feature = "dock")]
            splits: Resizable::<Message>::split_state(3, SplitAxis::Horizontal, 0.25).0,
            settings: SettingsState::new(),
            settings_dark_mode: true,
            settings_autosave: true,
            settings_font_size: 14.0,
            settings_font_family: "Inter".to_owned(),
            settings_accent: "blue".to_owned(),
            settings_telemetry: false,
            settings_launch_at_login: false,
            i18n: I18n::new(
                FluentTranslator::from_str("zh-CN", include_str!("../locales/zh-CN/main.ftl"))
                    .expect("valid zh-CN ftl"),
            ),
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
            // DrawerExitDrawn is here on purpose: its body does nothing, and
            // the view rebuild around any message is what drops the drawer
            // — exit already drawn — from the screen.
            Message::Noop | Message::DrawerExitDrawn => {}
            Message::SectionPicked(section) => {
                self.selected = section;
                // A panel belongs to the page that opened it.
                self.open_panel = None;
            }
            Message::TreeEvent(event) => match event {
                // The tree is the caller's: these are the state changes the
                // component read back, applied here so the next frame draws
                // them.
                iced_kit::widgets::TreeEvent::Toggled(id, _) => self.file_tree.toggle(&id),
                iced_kit::widgets::TreeEvent::Selected(id) => self.file_tree.select(Some(&id)),
            },
            // The anchor message precedes the toggle in the same batch, so a
            // panel opening always reads the bounds of the press that opened
            // it, and one closing wastes the update.
            Message::PanelAnchor(anchor) => self.panel_anchor = Some(anchor),
            Message::ToggleCombo => {
                self.open_panel =
                    (self.open_panel != Some(PanelKind::Combo)).then_some(PanelKind::Combo);
            }
            Message::ToggleDatePicker => {
                self.open_panel =
                    (self.open_panel != Some(PanelKind::Date)).then_some(PanelKind::Date);
            }
            Message::ToggleColorPicker => {
                self.open_panel =
                    (self.open_panel != Some(PanelKind::Color)).then_some(PanelKind::Color);
            }
            Message::ClosePanel => self.open_panel = None,
            Message::ComboQuery(query) => self.combo_query = query,
            Message::CountryPicked(value) => {
                self.picked_countries = vec![value];
                self.open_panel = None;
            }
            Message::DateText(text) => self.picked_date = text,
            Message::DatePicked(date) => {
                self.picked_date = date.to_string();
                self.open_panel = None;
            }
            Message::ColorChanged(color) => self.picked_color = color,
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
            Message::BioEdited(action) => {
                self.bio.perform(action);
            }
            Message::NotificationsToggled(value) => self.notifications = value,
            Message::NewsletterToggled(value) => self.newsletter = value,
            Message::PlanPicked(plan) => self.plan = plan,
            Message::VolumeChanged(value) => self.volume = value,
            Message::Advance => {
                self.progress = (self.progress + 0.1).min(1.0);
            }
            Message::TabSelected(index) => self.tab = index,
            Message::RibbonAnchor(anchor) => self.ribbon_anchor = Some(anchor),
            Message::RibbonTabSelected(index) => self.ribbon.select(index),
            Message::RibbonDropdown(id) => self.ribbon.toggle_dropdown(&id),
            Message::RibbonCollapse(mode) => self.ribbon.set_collapse_mode(mode),
            Message::CloseRibbonPanel => self.ribbon.close_dropdown(),
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
            Message::OpenAlert => self.alert_open = true,
            Message::CloseAlert => self.alert_open = false,
            Message::OpenAssembled => self.assembled_open = true,
            Message::CloseAssembled => self.assembled_open = false,
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
            Message::CloseDropdown => self.dropdown_open = false,
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
            Message::CarouselSelected(index) => self.carousel.select_index(index),
            Message::CarouselVerticalSelected(index) => {
                self.carousel_vertical.select_index(index);
            }
            Message::Settings(event) => {
                self.settings.apply(event);

                // Choosing a group asks to scroll to it. The position is only
                // known once iced has laid the page out, so it comes back as a
                // task rather than being applied here.
                return match self.settings.take_pending_scroll() {
                    Some((page, group)) => Settings::<Message>::scroll_to_group(page, group),
                    None => Task::none(),
                };
            }
            Message::SettingsReset(_page) => {
                // The panel cannot reset values it does not own, so it reports
                // which page asked and the application restores its own
                // defaults. One page needs one restore here because the demo
                // keeps every setting in the same struct.
                self.settings_dark_mode = true;
                self.settings_autosave = true;
                self.settings_font_size = 14.0;
                "Inter".clone_into(&mut self.settings_font_family);
                "blue".clone_into(&mut self.settings_accent);
                self.settings_telemetry = false;
                self.settings_launch_at_login = false;
            }
            Message::SettingsDarkMode(value) => self.settings_dark_mode = value,
            Message::SettingsAutosave(value) => self.settings_autosave = value,
            Message::SettingsFontSize(value) => self.settings_font_size = value,
            Message::SettingsFontFamily(value) => self.settings_font_family = value,
            Message::SettingsAccent(value) => self.settings_accent = value,
            Message::SettingsTelemetry(value) => self.settings_telemetry = value,
            Message::SettingsLaunchAtLogin(value) => self.settings_launch_at_login = value,
            Message::ToggleBusy => self.busy = !self.busy,
            Message::PortChanged(value) => self.port = value,
            Message::OtpChanged(value) => self.otp = value,
            Message::ListPicked(index) => self.list_selection = index,
            Message::ToggleDrawer => {
                self.drawer_open = !self.drawer_open;

                // The presence has to be told, or the drawer would stop being
                // drawn the instant it closed rather than sliding out.
                self.drawer_presence
                    .show(self.drawer_open, std::time::Instant::now());
            }
            Message::DrawerDismissed => {
                // Only a drawer that is open can be dismissed. A press that
                // lands while its exit is still being drawn would otherwise
                // re-open it — two clicks in quick succession, and the drawer
                // looks like it refuses to close.
                if self.drawer_open {
                    self.drawer_open = false;
                    self.drawer_presence.show(false, std::time::Instant::now());
                }
            }
            // The anchor arrives first in the same batch as the toggle, so an
            // opening press always carries where it happened.
            Message::DropdownAnchor(anchor) => self.dropdown_anchor = Some(anchor),
            Message::ContextMenuAnchor(anchor) => self.menu_at = Some(anchor),
            Message::PopoverAnchor(anchor) => self.popover_anchor = Some(anchor),
            Message::CloseContextMenu => {
                self.menu_at = None;
                self.sidebar_menu_at = None;
            }
            Message::TogglePopover => self.popover_open = !self.popover_open,
            Message::Scrolled(state) => {
                self.list_state = state;
            }
            Message::Sorted(heading) => self.table_state.toggle_sort(heading),
            Message::Minimize => self.last_window_action = Some("Minimize"),
            Message::Maximize => self.last_window_action = Some("Maximize"),
            Message::CloseWindow => self.last_window_action = Some("Close"),
            Message::SidebarToggled => self.sidebar_collapsed = !self.sidebar_collapsed,
            Message::SidebarModePicked(mode) => self.sidebar_collapsible = mode,
            Message::SidebarPicked(label) => {
                self.sidebar_active = label;
                self.last_window_action = Some(label);
            }
            Message::SidebarSubmenuToggled => {
                self.sidebar_projects_open = !self.sidebar_projects_open;
            }
            Message::SidebarContextOpened => self.sidebar_menu_at = Some((260.0, 300.0)),
            Message::SidebarResized(width) => self.sidebar_width = width,
            Message::SettingsSidebarResized(width) => self.settings_sidebar_width = width,
        }

        Task::none()
    }

    /// The left-hand rail: one row per page, the current one marked.
    fn nav(&self) -> Element<'_, Message, Theme> {
        let items = Section::ALL
            .iter()
            .map(|section| ListItem::new(section.label()))
            .collect();

        let selected = Section::ALL
            .iter()
            .position(|section| *section == self.selected);

        container(scrollable(
            column![
                heading("Components", Heading::H4),
                list(items, selected, |index| {
                    Message::SectionPicked(Section::ALL[index.min(Section::ALL.len() - 1)])
                }),
            ]
            .spacing(8),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .class(Box::new(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(theme.colors().muted)),
            border: iced::Border::default(),
            ..container::Style::default()
        }) as container::StyleFn<'_, Theme>)
        .into()
    }

    /// The page the demo pane shows.
    fn selected_section(&self) -> Element<'_, Message, Theme> {
        match self.selected {
            Section::Buttons => self.buttons_section(),
            Section::Form => self.form_section(),
            Section::FormLayout => self.form_layout_section(),
            Section::NewComponents => self.new_components_section(),
            Section::Chat => self.chat_section(),
            Section::InputGroups => self.input_group_section(),
            Section::Selection => self.selection_section(),
            Section::Display => self.display_section(),
            Section::Feedback => self.feedback_section(),
            Section::Overlays => self.overlay_section(),
            Section::Tabs => self.tabs_section(),
            Section::Ribbon => self.ribbon_section(),
            Section::Navigation => self.navigation_section(),
            Section::Carousel => self.carousel_section(),
            Section::Data => self.data_section(),
            Section::Settings => self.settings_section(),
            Section::GroupBoxes => Self::group_box_section(),
            Section::Sidebar => self.sidebar_section(),
            Section::Shell => self.shell_section(),
        }
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        let page = container(
            column![
                self.header(),
                row![
                    // The nav is a fixed rail; the pane takes the rest and
                    // scrolls its own page, so a long demo never pushes the
                    // nav off the side.
                    container(self.nav()).width(Length::Fixed(220.0)),
                    container(scrollable(self.selected_section()))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(24),
                ]
                .width(Length::Fill)
                .height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        // Every overlay is assembled in one place, in paint order: dropdowns
        // under the modal, toasts on top.
        let mut open = Layer::new();

        // The ribbon's open dropdown, hosted below the ribbon band the same
        // way the split components' panels are: the trigger reports the
        // anchor, the catcher closes it on an outside press.
        if self.selected == Section::Ribbon {
            if let (Some(anchor), Some(id)) =
                (self.ribbon_anchor, self.ribbon.open_dropdown.as_deref())
            {
                open = open.dropdown(stack![
                    popover_dismiss_area(Message::CloseRibbonPanel),
                    Dropdown::new(ribbon_panel_items(id))
                        .anchor(anchor.x, anchor.y + anchor.height + PANEL_GAP),
                ]);
            }
        }

        // The split components' panels live here rather than inside their
        // triggers: iced has no window-level z-order, so the application
        // positions and hosts them. The anchor is the pressed trigger's own
        // rectangle, reported by `trigger`, so each panel drops just below
        // the control that opened it. An open panel is wrapped with a
        // dismiss catcher under it, so a press anywhere else closes the
        // popover instead of reaching the page beneath.
        if self.selected == Section::NewComponents {
            if let Some(anchor) = self.panel_anchor {
                match self.open_panel {
                    Some(PanelKind::Combo) => {
                        open = open.dropdown(anchored_panel(
                            combobox_panel(
                                &self.country_options,
                                &self.picked_countries,
                                &self.combo_query,
                                Message::CountryPicked,
                            )
                            .width(260.0)
                            .max_height(200.0)
                            .into(),
                            anchor,
                            Message::ClosePanel,
                        ));
                    }

                    Some(PanelKind::Date) => {
                        let month = iced_kit::widgets::date::parse(&self.picked_date)
                            .unwrap_or_else(|| {
                                iced_kit::widgets::date::Date::from_ymd(2024, 2, 14).unwrap()
                            })
                            .first_of_month();
                        let selected = iced_kit::widgets::date::parse(&self.picked_date);

                        let card = container(
                            calendar::<Message>(month, selected, &self.i18n)
                                .on_select(Message::DatePicked)
                                .number_of_months(1),
                        )
                        .padding(8)
                        .class(Box::new(|theme: &Theme| container::Style {
                            background: Some(iced::Background::Color(theme.colors().surface)),
                            border: iced::Border {
                                color: theme.colors().border,
                                width: 1.0,
                                radius: f32::from(theme.radius().md).into(),
                            },
                            ..container::Style::default()
                        }) as container::StyleFn<'_, Theme>);

                        // The calendar brings its own card, so it goes through
                        // `Popover`, which styles for it; it is still placed
                        // from the trigger's rectangle, and still dismissed on
                        // a press outside it.
                        open = open.dropdown(stack![
                            popover_dismiss_area(Message::ClosePanel),
                            Popover::new(card, (anchor.x, anchor.y + anchor.height + PANEL_GAP),)
                                .placement(PopoverPlacement::BottomStart)
                        ]);
                    }

                    Some(PanelKind::Color) => {
                        open = open.dropdown(anchored_panel(
                            color_picker_panel::<Message>(self.picked_color)
                                .on_change(Message::ColorChanged)
                                .width(240.0)
                                .into(),
                            anchor,
                            Message::ClosePanel,
                        ));
                    }

                    None => {}
                }
            }
        }

        if self.dropdown_open {
            if let Some(anchor) = self.dropdown_anchor {
                // The catcher under the menu is what makes a press outside it
                // close the menu instead of reaching the page beneath.
                open = open.dropdown(stack![
                    popover_dismiss_area(Message::CloseDropdown),
                    Dropdown::new(vec![
                        MenuItem::new("Duplicate", Message::PickedFromMenu("Duplicated"))
                            .shortcut("Ctrl+D"),
                        MenuItem::new("Rename", Message::PickedFromMenu("Renamed")).shortcut("F2"),
                        MenuItem::new("Archive", Message::PickedFromMenu("Archived"))
                            .enabled(false),
                        MenuItem::new("Delete", Message::PickedFromMenu("Deleted"))
                            .destructive(true),
                    ])
                    .anchor(anchor.x, anchor.y + anchor.height + PANEL_GAP),
                ]);
            }
        }

        if let Some(anchor) = self.menu_at {
            open = open.dropdown(stack![
                popover_dismiss_area(Message::CloseContextMenu),
                ContextMenu::new(
                    vec![
                        MenuItem::new("Cut", Message::CloseContextMenu).shortcut("Ctrl+X"),
                        MenuItem::new("Copy", Message::CloseContextMenu).shortcut("Ctrl+C"),
                        MenuItem::new("Archive", Message::CloseContextMenu).enabled(false),
                        MenuItem::new("Delete", Message::CloseContextMenu).destructive(true),
                    ],
                    (anchor.x, anchor.y + anchor.height + PANEL_GAP),
                ),
            ]);
        }

        // The sidebar item's right-click intent lands here: the menu's items
        // and its placement are the application's, not the sidebar's.
        if let Some(at) = self.sidebar_menu_at {
            open = open.dropdown(stack![
                popover_dismiss_area(Message::CloseContextMenu),
                ContextMenu::new(
                    vec![
                        MenuItem::new("Open", Message::CloseContextMenu),
                        MenuItem::new("Rename", Message::CloseContextMenu).shortcut("F2"),
                        MenuItem::new("Delete", Message::CloseContextMenu).destructive(true),
                    ],
                    at,
                ),
            ]);
        }

        if self.popover_open {
            if let Some(anchor) = self.popover_anchor {
                open = open.dropdown(stack![
                    popover_dismiss_area(Message::TogglePopover),
                    Popover::new(
                        column![
                            heading("Quick settings", Heading::H4),
                            paragraph("Adjust how the gallery behaves."),
                            button("Close")
                                .secondary()
                                .size(Size::Sm)
                                .on_press(Message::TogglePopover),
                        ]
                        .spacing(12),
                        (anchor.x, anchor.y + anchor.height + PANEL_GAP),
                    ),
                ]);
            }
        }

        // The drawer is kept mounted while it leaves, so its exit is drawn. The
        // presence is what tells the gallery to keep supplying it.
        if self.drawer_open || self.drawer_presence.should_render() {
            open = open.drawer(
                Drawer::new(
                    "",
                    column![
                        paragraph("A drawer slides in from an edge and leaves the page visible."),
                        text_input::<Message>("Name", &self.name),
                    ]
                    .spacing(12),
                )
                // The header carries the close control. A drawer is only
                // dismissable from the backdrop beside it — a press on the
                // drawer itself stays with the drawer — so without a visible
                // way out, closing took two clicks.
                .header(overlay::drawer_header("Details", Message::DrawerDismissed))
                .side(DrawerSide::Right)
                .presence(&self.drawer_presence)
                .on_dismiss(Message::DrawerDismissed)
                .on_closed(Message::DrawerExitDrawn),
            );
        }

        if self.modal_open {
            open = open.modal(
                Modal::new(
                    "Delete project",
                    label("This permanently removes the project and all of its data."),
                    Message::CloseModal,
                )
                .description("Every file in it goes with it.")
                .cancel("Cancel", Message::CloseModal)
                .destructive("Delete", Message::ConfirmModal)
                // Grabbable by its surface, so the modal can be moved aside to
                // read what it is covering.
                .draggable(true),
            );
        }

        // An alert: the same surface, with the tone's icon, centred buttons and
        // no close button of its own.
        if self.alert_open {
            open = open.modal(
                AlertDialog::new()
                    .tone(AlertTone::Danger)
                    .title("Delete project?")
                    .description("Every file in it goes with it. This cannot be undone.")
                    .confirm()
                    .on_confirm(Message::CloseAlert)
                    .on_cancel(Message::CloseAlert)
                    .on_dismiss(Message::CloseAlert),
            );
        }

        // A dialog body assembled from the composition parts: the caller lays
        // out the header and the footer itself rather than handing the dialog a
        // title and a blob of content. The modal is given an empty title and no
        // actions, so it contributes only the surface and the backdrop.
        if self.assembled_open {
            let body: Element<'_, Message, Theme> = DialogContent::new()
                .spacing(16.0)
                .push(
                    DialogHeader::new()
                        .title("Merge branch")
                        .description("The branch will be merged into main."),
                )
                .push(label("This cannot be undone."))
                .push(Self::assembled_footer())
                .into();

            open = open.modal(
                Modal::new("", body, Message::CloseAssembled)
                    .width(DialogWidth::Lg)
                    .close_button(false),
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
        // The rail and the demo pane both inset themselves; the header spans
        // the window, so it carries its own margin to line up with them.
        .padding(Padding {
            top: 16.0,
            right: 24.0,
            bottom: 16.0,
            left: 24.0,
        })
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

    fn form_layout_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Form layout",
            form()
                .columns(2)
                .child(
                    field().label("Name").push(
                        text_input::<Message>("Ada Lovelace", &self.name)
                            .on_input(Message::NameChanged),
                    ),
                )
                .child(
                    field().label("Email").required(true).push(
                        text_input::<Message>("you@example.com", &self.email)
                            .on_input(Message::EmailChanged),
                    ),
                )
                .child(
                    field()
                        .label("Bio")
                        .description("Use at most 100 words to describe yourself.")
                        .col_span(2)
                        .push(
                            text_area::<Message>("Write something…", &self.bio)
                                .on_edit(Message::BioEdited),
                        ),
                )
                .child(
                    field()
                        .label_indent(false)
                        .col_span(2)
                        .push(muted_text("This is a full width form field.")),
                )
                .footer(row![button("Save").primary().on_press(Message::Save)].spacing(8)),
        )
    }

    /// The chat family: bubbles, message rows, markers, an attachment and a
    /// tail-following transcript.
    fn chat_section(&self) -> Element<'_, Message, Theme> {
        let thumbs: Element<'_, Message, Theme> = muted_text("👍 2").into();

        let received: Element<'_, Message, Theme> =
            message(bubble("How are the docs coming along?"))
                .avatar(avatar::<Message>("Ada Lovelace", 32))
                .header("Ada Lovelace")
                .footer("12:30")
                .into();

        let sent: Element<'_, Message, Theme> = message(
            bubble("Nearly there — one section left.")
                .with_variant(BubbleVariant::Secondary)
                .reactions(BubbleReactions::new().child(thumbs)),
        )
        .alignment(MessageAlignment::End)
        .header("Grace Hopper")
        .footer("Sent")
        .into();

        let in_a_slot: Element<'_, Message, Theme> = message(
            bubble("An avatar through a numbered slot.").with_variant(BubbleVariant::Muted),
        )
        .avatar(MessageAvatar::new().child(muted_text("AL")))
        .header_el(MessageHeader::new().text("System"))
        .into();

        let failed: Element<'_, Message, Theme> = attachment::<Message>("draft.pdf")
            .status(AttachmentStatus::Failed)
            .content(
                AttachmentContent::new()
                    .title(AttachmentTitle::new("draft.pdf"))
                    .description(AttachmentDescription::new("Upload failed — retry")),
            )
            .into();

        let scroller: Element<'_, Message, Theme> =
            MessageScroller::new(&self.chat_lines, &self.chat, |line, _| {
                muted_text(line.clone()).into()
            })
            .jump_button(true)
            .with_bottom_fade(Some(iced::Color::from_rgb8(0xfa, 0xfa, 0xfa)))
            .row_height(28.0)
            .height(220.0)
            .into();

        Self::section(
            "Chat",
            column![
                received,
                sent,
                in_a_slot,
                row![
                    marker::<Message>("— Today —"),
                    marker::<Message>("System note").with_variant(MarkerVariant::Border),
                ]
                .spacing(16),
                failed,
                scroller,
            ]
            .spacing(12),
        )
    }

    /// The batch of components added alongside the reference's remaining set:
    /// a searchable combobox, a date field, a colour picker, a stepper, a tree,
    /// a breadcrumb and a description list.
    fn new_components_section(&self) -> Element<'_, Message, Theme> {
        use iced_kit::widgets::{
            breadcrumb, color_picker, date_picker, description_list, stepper, tree, Crumb,
            Description, DescriptionLayout, Step, StepLayout,
        };

        Self::section(
            "New components",
            column![
                row![
                    // Each trigger only reports intent; the open panel is
                    // hosted further down, in the layer. `trigger` wraps the
                    // intent with the one thing the panel also needs: where
                    // the pressed trigger sits.
                    trigger(
                        combobox::<usize, Message>(
                            &self.country_options,
                            self.picked_countries.first().copied(),
                            "Country",
                        )
                        .query(&self.combo_query)
                        .on_query(Message::ComboQuery)
                        .open(self.open_panel == Some(PanelKind::Combo))
                        .on_toggle(Message::ToggleCombo)
                        .fill(true),
                        Message::PanelAnchor,
                    ),
                    trigger(
                        date_picker::<Message>("Pick a date", &self.picked_date)
                            .open(self.open_panel == Some(PanelKind::Date))
                            .on_input(Message::DateText)
                            .on_toggle(Message::ToggleDatePicker)
                            .fill(true),
                        Message::PanelAnchor,
                    ),
                ]
                .spacing(16),
                row![
                    trigger(
                        color_picker::<Message>(self.picked_color)
                            .open(self.open_panel == Some(PanelKind::Color))
                            .on_toggle(Message::ToggleColorPicker),
                        Message::PanelAnchor,
                    ),
                    rating::<Message>(4).on_select(|_| Message::Noop),
                    clipboard_button::<Message>("cargo test")
                        .label("Copy")
                        .on_copy(|_| Message::Noop),
                ]
                .spacing(16)
                .align_y(iced::Alignment::Center),
                breadcrumb(vec![
                    Crumb::link("Home", Message::Noop),
                    Crumb::link("Projects", Message::Noop),
                    Crumb::new("iced-kit"),
                ]),
                stepper(
                    vec![
                        Step::new("Cart"),
                        Step::new("Address"),
                        Step::new("Payment"),
                    ],
                    1,
                )
                .layout(StepLayout::Horizontal)
                .on_select(|_| Message::Noop),
                tree::<Message>(&self.file_items, &self.file_tree)
                    .on_event(Message::TreeEvent)
                    .max_height(140.0),
                description_list(vec![
                    Description::new("Version").value("0.1.0"),
                    Description::new("License").value("MIT OR Apache-2.0"),
                ])
                .layout(DescriptionLayout::Horizontal)
                .bordered(true),
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

    /// The footer for the hand-assembled dialog, built from the composition
    /// parts rather than from `Modal`'s own action list.
    fn assembled_footer() -> Element<'static, Message, Theme> {
        DialogFooter::new()
            .push(button("Cancel").on_press(Message::CloseAssembled))
            .push(button("Merge").primary().on_press(Message::CloseAssembled))
            .into()
    }

    fn overlay_section(&self) -> Element<'_, Message, Theme> {
        Self::section(
            "Overlays",
            column![
                row![
                    button("Open modal").primary().on_press(Message::OpenModal),
                    button("Alert dialog")
                        .destructive()
                        .on_press(Message::OpenAlert),
                    button("Assembled dialog")
                        .secondary()
                        .on_press(Message::OpenAssembled),
                    button("Open drawer")
                        .secondary()
                        .on_press(Message::ToggleDrawer),
                    button("Show toast")
                        .secondary()
                        .on_press(Message::ShowToast),
                    trigger(
                        button("Dropdown")
                            .secondary()
                            .on_press(Message::ToggleDropdown),
                        Message::DropdownAnchor,
                    ),
                    trigger(
                        button("Context menu").secondary().on_press(Message::Noop),
                        Message::ContextMenuAnchor,
                    ),
                    trigger(
                        button("Popover")
                            .secondary()
                            .on_press(Message::TogglePopover),
                        Message::PopoverAnchor,
                    ),
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

    /// A settings panel, with the page and field shapes the reference shows.
    fn settings_section(&self) -> Element<'_, Message, Theme> {
        let panel = Settings::<Message>::new(&self.settings)
            .on_event(Message::Settings)
            .on_reset(Message::SettingsReset)
            .sidebar_width(self.settings_sidebar_width)
            .on_sidebar_resize(Message::SettingsSidebarResized)
            .page(
                SettingPage::new("Appearance")
                    .icon(IconName::Palette)
                    .description("How the interface looks.")
                    .group(
                        SettingGroup::new()
                            .title("Theme")
                            .description("The palette the interface is drawn in.")
                            .item(
                                SettingItem::new("Dark mode")
                                    .description("Use the dark palette.")
                                    .keywords(["night", "color scheme"])
                                    .field(
                                        SettingField::switch(
                                            self.settings_dark_mode,
                                            Message::SettingsDarkMode,
                                        )
                                        .default_value(true),
                                    ),
                            )
                            .item(
                                SettingItem::new("Accent color").field(
                                    SettingField::select(
                                        vec![
                                            ("blue".to_owned(), "Blue".to_owned()),
                                            ("violet".to_owned(), "Violet".to_owned()),
                                            ("rose".to_owned(), "Rose".to_owned()),
                                        ],
                                        Some(self.settings_accent.clone()),
                                        Message::SettingsAccent,
                                    )
                                    .default_value("blue"),
                                ),
                            ),
                    )
                    .group(
                        SettingGroup::new()
                            .title("Typography")
                            .item(
                                SettingItem::new("Font size")
                                    .description("In points.")
                                    .field(
                                        SettingField::number(
                                            self.settings_font_size,
                                            8.0..=72.0,
                                            Message::SettingsFontSize,
                                        )
                                        .default_value(14.0),
                                    ),
                            )
                            .item(SettingItem::new("Font family").field(SettingField::text(
                                self.settings_font_family.clone(),
                                Message::SettingsFontFamily,
                            ))),
                    ),
            )
            .page(
                SettingPage::new("General").icon(IconName::Settings2).group(
                    SettingGroup::new()
                        .title("Startup")
                        .item(
                            SettingItem::new("Launch at login").field(SettingField::switch(
                                self.settings_launch_at_login,
                                Message::SettingsLaunchAtLogin,
                            )),
                        )
                        .item(
                            SettingItem::new("Autosave")
                                .description("Write changes as you make them.")
                                .field(
                                    SettingField::switch(
                                        self.settings_autosave,
                                        Message::SettingsAutosave,
                                    )
                                    .default_value(true),
                                ),
                        ),
                ),
            )
            .page(
                SettingPage::new("Privacy")
                    .icon(IconName::User)
                    .group(
                        SettingGroup::new().title("Telemetry").item(
                            SettingItem::new("Send usage data")
                                .description("Anonymous, and never sold.")
                                .keywords(["analytics", "tracking"])
                                .field(
                                    SettingField::checkbox(
                                        self.settings_telemetry,
                                        Message::SettingsTelemetry,
                                    )
                                    .default_value(false),
                                ),
                        ),
                    )
                    .group(
                        SettingGroup::new()
                            .title("Experimental")
                            .disabled(true)
                            .item(SettingItem::new("Differential sync").field(SettingField::<
                                Message,
                            >::switch(
                                false,
                                |_| Message::ToggleBusy,
                            ))),
                    ),
            );

        Self::section("Settings", container(panel).height(Length::Fixed(560.0)))
    }

    /// The three group box variants, standalone.
    fn group_box_section() -> Element<'static, Message, Theme> {
        let sample = |variant: GroupBoxVariant| {
            group_box::<Message>()
                .variant(variant)
                .title(label(variant.as_str().to_owned()).size(14))
                .push(text("Grouped content").size(13))
        };

        Self::section(
            "Group boxes",
            column![
                sample(GroupBoxVariant::Normal),
                sample(GroupBoxVariant::Fill),
                sample(GroupBoxVariant::Outline).description("A bordered surface."),
            ]
            .spacing(12),
        )
    }

    fn shell_section(&self) -> Element<'_, Message, Theme> {
        let action = self
            .last_window_action
            .map_or_else(|| "none yet".to_owned(), str::to_owned);

        let title_bar = TitleBar::new("iced-kit")
            .icon(IconName::GalleryVerticalEnd)
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

    fn sidebar_section(&self) -> Element<'_, Message, Theme> {
        let icon_collapsed =
            self.sidebar_collapsed && self.sidebar_collapsible == SidebarCollapsible::Icon;

        let mode_button = |label: &'static str, mode: SidebarCollapsible| -> Button<Message> {
            Button::new(label)
                .size(iced_kit::Size::Sm)
                .selected(self.sidebar_collapsible == mode)
                .on_press(Message::SidebarModePicked(mode))
        };

        // The app tile collapses with the rail, trading its fill for a bare
        // glyph — the same adaptation the reference example makes.
        let logo_side = if icon_collapsed { 24.0 } else { 32.0 };
        let logo =
            container(Icon::new(IconName::GalleryVerticalEnd).into_element(iced_kit::Size::Md))
                .width(Length::Fixed(logo_side))
                .height(Length::Fixed(logo_side))
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .class(
                    Box::new(move |theme: &Theme| iced::widget::container::Style {
                        background: (!icon_collapsed)
                            .then(|| iced::Background::Color(theme.colors().sidebar_primary)),
                        text_color: Some(theme.colors().sidebar_primary_foreground),
                        border: iced::Border {
                            radius: f32::from(theme.radius().md).into(),
                            ..iced::Border::default()
                        },
                        ..iced::widget::container::Style::default()
                    }) as iced::widget::container::StyleFn<'_, Theme>,
                );

        let mut header_row = row![logo].spacing(8).align_y(Alignment::Center);

        if !icon_collapsed {
            header_row = header_row.push(
                column![text("Acme Inc").size(14), muted_text("Enterprise")]
                    .spacing(2)
                    .width(Length::Fill),
            );
        }

        let mut footer_row = row![Icon::new(IconName::CircleUser).into_element(iced_kit::Size::Md)]
            .spacing(8)
            .align_y(Alignment::Center);

        if !icon_collapsed {
            footer_row = footer_row.push(text("Jason Lee").size(14));
        }

        let picked = |label: &'static str| self.sidebar_active == label;

        let menu = SidebarMenu::new().children([
            SidebarMenuItem::new("Dashboard")
                .icon(IconName::LayoutDashboard)
                .active(picked("Dashboard"))
                .on_select(Message::SidebarPicked("Dashboard")),
            SidebarMenuItem::new("Inbox")
                .icon(IconName::Inbox)
                .active(picked("Inbox"))
                .on_select(Message::SidebarPicked("Inbox")),
            SidebarMenuItem::new("Calendar")
                .icon(IconName::Calendar)
                .active(picked("Calendar"))
                .on_select(Message::SidebarPicked("Calendar")),
            SidebarMenuItem::new("Projects")
                .icon(IconName::Folder)
                .open(self.sidebar_projects_open)
                .on_toggle(Message::SidebarSubmenuToggled)
                .on_context(Message::SidebarContextOpened)
                .suffix(badge("3", Tone::Neutral))
                .children([
                    SidebarMenuItem::new("Design")
                        .active(picked("Design"))
                        .on_select(Message::SidebarPicked("Design")),
                    SidebarMenuItem::new("Engineering")
                        .active(picked("Engineering"))
                        .on_select(Message::SidebarPicked("Engineering")),
                    SidebarMenuItem::new("Marketing")
                        .active(picked("Marketing"))
                        .on_select(Message::SidebarPicked("Marketing")),
                ]),
            SidebarMenuItem::new("Settings")
                .icon(IconName::Settings)
                .active(picked("Settings"))
                .on_select(Message::SidebarPicked("Settings")),
        ]);

        let description = match self.sidebar_collapsible {
            SidebarCollapsible::Icon => {
                "Icon mode narrows the sidebar to a 48px rail; hovering an icon shows its name."
            }
            SidebarCollapsible::Offcanvas => {
                "Offcanvas mode slides the sidebar out and releases the width it occupied."
            }
            SidebarCollapsible::None => {
                "None mode keeps the sidebar expanded and ignores the collapsed flag."
            }
        };

        Self::section(
            "Sidebar",
            column![
                row![
                    SidebarToggleButton::new()
                        .collapsed(icon_collapsed)
                        .on_press(Message::SidebarToggled),
                    text("Collapsible modes").size(14),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                row![
                    text("Mode:").size(13),
                    mode_button("Icon", SidebarCollapsible::Icon),
                    mode_button("Offcanvas", SidebarCollapsible::Offcanvas),
                    mode_button("None", SidebarCollapsible::None),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                divider(),
                row![
                    sidebar()
                        .collapsible(self.sidebar_collapsible)
                        .collapsed(self.sidebar_collapsed)
                        .width(self.sidebar_width)
                        .min_width(200.0)
                        .on_resize(Message::SidebarResized)
                        .header(
                            SidebarHeader::new()
                                .child(header_row)
                                .on_dropdown(Message::SidebarContextOpened)
                        )
                        .child(SidebarGroup::new("Application").child(menu))
                        .footer(
                            SidebarFooter::new()
                                .child(footer_row)
                                .on_dropdown(Message::SidebarContextOpened)
                        ),
                    container(text(description).size(13))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(16),
                ]
                .width(Length::Fill)
                .height(Length::Fixed(380.0)),
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

    /// A carousel in both orientations, with the controls and the dot
    /// indicator, which is the whole of the surface a caller can configure.
    fn carousel_section(&self) -> Element<'_, Message, Theme> {
        let slide = |index: usize, height: f32| -> Element<'_, Message, Theme> {
            container(
                column![
                    heading(format!("Slide {}", index + 1), Heading::H3),
                    label("A snapping viewport, dragged or stepped through."),
                ]
                .spacing(4),
            )
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(height))
            .into()
        };

        Self::section(
            "Carousel",
            column![
                column![
                    label("Horizontal, looping, with indicators").size(13),
                    carousel(
                        &self.carousel,
                        (0..4).map(|index| slide(index, 120.0)).collect(),
                        Message::CarouselSelected,
                    )
                    .height(Length::Fixed(120.0))
                    .indicators(true),
                ]
                .spacing(8),
                divider(),
                column![
                    label("Two per view").size(13),
                    carousel(
                        &self.carousel,
                        (0..4).map(|index| slide(index, 110.0)).collect(),
                        Message::CarouselSelected,
                    )
                    .per_view(2)
                    .gap(12.0)
                    .height(Length::Fixed(110.0)),
                ]
                .spacing(8),
                divider(),
                column![
                    label("Vertical").size(13),
                    row![carousel(
                        &self.carousel_vertical,
                        (0..3).map(|index| slide(index, 90.0)).collect(),
                        Message::CarouselVerticalSelected,
                    )
                    .height(Length::Fixed(90.0))
                    .width(Length::Fixed(360.0)),],
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

    fn ribbon_section(&self) -> Element<'_, Message, Theme> {
        // The ribbon reports each pressed ▾'s own bounds through
        // `on_dropdown_anchor`, so the panel the gallery hosts drops beneath the
        // button that opened it, as for every other overlay.
        let ribbon = Ribbon::new()
            .tabs(ribbon_tabs())
            .state(&self.ribbon)
            .on_select(Message::RibbonTabSelected)
            .on_dropdown_toggle(Message::RibbonDropdown)
            .on_dropdown_anchor(Message::RibbonAnchor);

        // A row pinning every group to one density. Auto degrades the row from
        // the right as the pane narrows; the rest force a level to inspect.
        let density = CollapseMode::ALL
            .iter()
            .fold(row![].spacing(6), |row, mode| {
                let active = self.ribbon.collapse_mode == *mode;
                let btn = button(mode.label());
                let btn = if active { btn.primary() } else { btn.ghost() };
                row.push(btn.on_press(Message::RibbonCollapse(*mode)))
            });

        Self::section(
            "Ribbon",
            column![
                muted_text(
                    "A tab strip over titled groups of large, small, dropdown and grid tools."
                ),
                divider(),
                row![label("Density").size(13), density]
                    .spacing(10)
                    .align_y(Alignment::Center),
                ribbon,
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

/// Hosts a panel just below the trigger's rectangle, over a full-area
/// dismiss catcher.
///
/// The catcher is the layer under the panel: a press that misses the panel
/// closes the popover instead of reaching the page beneath it, and the stack
/// hands a press that lands on the panel to the panel alone.
fn anchored_panel<'a, Message: Clone + 'a>(
    panel: Element<'a, Message, Theme>,
    anchor: iced::Rectangle,
    dismiss: Message,
) -> Element<'a, Message, Theme> {
    stack![
        popover_dismiss_area(dismiss),
        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Start)
            .align_y(Alignment::Start)
            .padding(Padding {
                top: (anchor.y + anchor.height + PANEL_GAP).max(0.0),
                right: 0.0,
                bottom: 0.0,
                left: anchor.x.max(0.0),
            })
    ]
    .into()
}

/// The file tree the new-components demo shows.
fn file_tree_items() -> Vec<iced_kit::widgets::TreeItem> {
    use iced_kit::widgets::TreeItem;

    vec![TreeItem::new("src", "src").children([
        TreeItem::new("main", "main.rs"),
        TreeItem::new("widgets", "widgets").child(TreeItem::new("button", "button.rs")),
    ])]
}

/// A short transcript for the chat demo.
fn chat_lines() -> Vec<String> {
    [
        "Ada: How are the docs coming along?",
        "Grace: Nearly there — one section left.",
        "Ada: Take your time.",
        "Grace: The chat components are in.",
        "Ada: Bubbles, markers, attachments?",
        "Grace: All of it, and the scroller.",
        "Ada: Nice.",
        "Grace: Scroll up and the jump button appears.",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect()
}

/// The ribbon demo's tabs: a Home tab showing every item size, and a Modify
/// tab showing a grid. All data, no behaviour — the gallery owns the state.
fn ribbon_tabs() -> Vec<RibbonTab<Message>> {
    let home = RibbonTab::new("Home")
        .group(
            RibbonGroup::new("Clipboard")
                .item(RibbonItem::large_dropdown(
                    "paste",
                    IconName::ClipboardPaste,
                    "Paste",
                    vec![],
                ))
                .item(RibbonItem::tool(
                    RibbonTool::named(IconName::Copy).label("Copy"),
                ))
                .item(RibbonItem::tool(
                    RibbonTool::named(IconName::Scissors).label("Cut"),
                )),
        )
        .group(
            RibbonGroup::new("Draw")
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Spline)
                        .label("Line")
                        .selected(true),
                ))
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Shapes).label("Shapes"),
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
                    RibbonTool::named(IconName::Bold).label("Bold"),
                ))
                .item(RibbonItem::labeled(
                    RibbonTool::named(IconName::Italic).label("Italic"),
                ))
                .item(RibbonItem::dropdown("layers", IconName::Layers, vec![])),
        );

    let modify =
        RibbonTab::new("Modify").group(RibbonGroup::new("Transform").item(RibbonItem::grid(vec![
            vec![
                RibbonTool::named(IconName::Move),
                RibbonTool::named(IconName::Eraser),
            ],
            vec![
                RibbonTool::named(IconName::Group),
                RibbonTool::named(IconName::Pencil),
            ],
        ])));

    vec![home, modify]
}

/// The floating panel the gallery hosts for the ribbon's open dropdown id.
fn ribbon_panel_items(id: &str) -> Vec<MenuItem<'_, Message>> {
    match id {
        "paste" => vec![
            MenuItem::new("Keep Source Formatting", Message::CloseRibbonPanel),
            MenuItem::new("Merge Formatting", Message::CloseRibbonPanel),
            MenuItem::new("Keep Text Only", Message::CloseRibbonPanel),
        ],
        "layers" => vec![
            MenuItem::new("Bring to Front", Message::CloseRibbonPanel),
            MenuItem::new("Send to Back", Message::CloseRibbonPanel),
            MenuItem::new("Lock Layer", Message::CloseRibbonPanel).enabled(false),
        ],
        other => vec![MenuItem::new(other, Message::CloseRibbonPanel)],
    }
}
