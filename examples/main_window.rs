//! A full main-window demo matching the SARibbon MainWindowExample layout.
//!
//! The window runs undecorated — its default title bar is replaced by the
//! ribbon's own, which drags the window, toggles maximize on a double press,
//! and carries the window controls. Run with `cargo run --example main_window`.

use std::time::{Duration, Instant};

use iced::widget::{column, container, mouse_area, row, scrollable, stack, text};
use iced::{Alignment, Color, Element, Length, Padding, Rectangle, Task};
use iced_kit::i18n::{FluentTranslator, I18n};
use iced_kit::icons::IconName;
use iced_kit::widgets::button::{self as kit_button, Button};
use iced_kit::widgets::details::StatusBar;
use iced_kit::widgets::overlay::{
    self, popover_dismiss_area, trigger, Dropdown, Layer, MenuItem, TooltipPosition,
};
use iced_kit::widgets::ribbon::{
    Ribbon, RibbonGallery, RibbonGalleryItem, RibbonGroup, RibbonItem, RibbonLayout, RibbonState,
    RibbonTab, RibbonTheme, RibbonTool,
};
use iced_kit::widgets::tabs::{tabs, Tab, TabStripColors, TabVariant};
use iced_kit::widgets::{combobox, combobox_panel, text_input, ComboBoxOption};
use iced_kit::{Size, Theme};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        // The ribbon draws its own title bar, so the window's default one is
        // not wanted: it would sit above the ribbon's and double every
        // affordance.
        .decorations(false)
        .run()
}

const PANEL_GAP: f32 = 6.0;

#[derive(Debug, Clone)]
enum Message {
    /// Does nothing; a disabled placeholder menu item needs some message.
    Noop,
    TabSelected(usize),
    DropdownToggled(String),
    RibbonAnchored(Rectangle),
    CloseDropdown,
    /// The application button was pressed, reporting its bounds so the menu
    /// drops from beneath it.
    AppMenuAnchored(Rectangle),
    /// The application button toggles its menu.
    AppMenuToggle,
    AppMenuAction(&'static str),
    ToggleMinimize,
    /// The right button group's pin: pinning the ribbon restores the band.
    PinRibbon,
    TogglePictureTools,
    ToggleTableTools,
    /// Adds a category to the ribbon, or removes it when it is already there.
    ToggleDeleteCategory,
    /// Adds a panel to the active category, or removes it when it is there.
    ToggleSizePanel,
    ThemeSelected(RibbonTheme),
    LayoutSelected(RibbonLayout),
    SearchChanged(String),
    ToggleWordWrap,
    ToggleRtl,
    AlignLeft,
    AlignCenter,
    AlignRight,
    Ran(&'static str),
    /// The title bar's drag area was pressed: begins a window drag, or
    /// toggles maximize when the press is the second of a double click.
    TitleBarPressed,
    WindowMinimize,
    WindowToggleMaximize,
    WindowClose,
    /// The window's id, learned from the window-event subscription once the
    /// runtime has told us which window this is.
    WindowId(iced::window::Id),
    /// The theme ComboBox's panel open state and trigger anchor.
    ThemeComboToggle,
    ThemeComboAnchored(Rectangle),
    /// The layout ComboBox's panel open state and trigger anchor.
    LayoutComboToggle,
    LayoutComboAnchored(Rectangle),
    /// Switches the application language ("en" or "zh-CN").
    SwitchLanguage(&'static str),
}

struct App {
    ribbon: RibbonState,
    anchor: Option<Rectangle>,
    app_menu_anchor: Option<Rectangle>,
    app_menu_open: bool,
    search: String,
    word_wrap: bool,
    rtl: bool,
    alignment: Alignment,
    status: String,
    /// Whether the dynamically-added "Delete" category is in the ribbon.
    delete_shown: bool,
    /// Whether the long-titled "Size(example long category)" category is
    /// shown, demonstrating overflow in the strip.
    size_shown: bool,
    /// The central area's operation log, oldest first.
    log: Vec<String>,
    /// When the title bar was last pressed, for double-click detection.
    last_title_press: Option<Instant>,
    /// This window's id, learned from the window-event subscription.
    window_id: Option<iced::window::Id>,
    /// The theme ComboBox's options, panel state, and trigger anchor.
    theme_options: Vec<ComboBoxOption<RibbonTheme>>,
    theme_combo_open: bool,
    /// Where the theme ComboBox's trigger sits, so its panel drops beneath it.
    theme_combo_anchor: Option<Rectangle>,
    /// The layout ComboBox's options, panel state, and trigger anchor.
    layout_options: Vec<ComboBoxOption<RibbonLayout>>,
    layout_combo_open: bool,
    /// Where the layout ComboBox's trigger sits.
    layout_combo_anchor: Option<Rectangle>,
    /// The currently-picked theme and layout, stored so the panels can
    /// reference a value that outlives the view call.
    theme_selected: RibbonTheme,
    layout_selected: RibbonLayout,
    /// The translation context, re-created on a language switch from the
    /// inline FTL in `Message::SwitchLanguage`.
    i18n: I18n<FluentTranslator>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let i18n = I18n::new(
            FluentTranslator::from_str("en", include_str!("../locales/en/main.ftl"))
                .expect("valid en ftl"),
        );

        (
            Self {
                ribbon: RibbonState::new(),
                anchor: None,
                app_menu_anchor: None,
                app_menu_open: false,
                search: String::new(),
                word_wrap: true,
                rtl: false,
                alignment: Alignment::Start,
                status: String::from("Ready"),
                delete_shown: true,
                size_shown: true,
                log: vec![String::from("Ready")],
                last_title_press: None,
                window_id: None,
                theme_options: theme_options(),
                theme_combo_open: false,
                theme_combo_anchor: None,
                layout_options: layout_options(),
                layout_combo_open: false,
                layout_combo_anchor: None,
                theme_selected: RibbonTheme::default(),
                layout_selected: RibbonLayout::default(),
                i18n,
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        String::from("SARibbon Demo — iced-kit")
    }

    /// Appends an entry to the central log, keeping the newest last.
    fn log(&mut self, entry: String) {
        self.status = entry.clone();
        self.log.push(entry);
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        // The window-event subscription reports every event with its window
        // id; the first one tells us which window this application owns, so
        // the title bar can drive it (drag, maximize, close).
        iced::window::events().map(|(id, _event)| Message::WindowId(id))
    }

    fn theme(&self) -> Theme {
        Theme::light()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Noop => {}
            Message::TabSelected(index) => self.ribbon.select(index),
            Message::DropdownToggled(id) => self.ribbon.toggle_dropdown(&id),
            Message::RibbonAnchored(rect) => self.anchor = Some(rect),
            Message::CloseDropdown => {
                self.ribbon.close_dropdown();
                self.app_menu_open = false;
                self.theme_combo_open = false;
                self.layout_combo_open = false;
            }
            Message::AppMenuAnchored(rect) => self.app_menu_anchor = Some(rect),
            Message::AppMenuToggle => {
                self.app_menu_open = !self.app_menu_open;
            }
            Message::AppMenuAction(action) => {
                self.log(format!("App menu: {action}"));
                self.app_menu_open = false;
                self.ribbon.close_dropdown();
            }
            Message::ToggleMinimize => self.ribbon.toggle_minimize(),
            Message::PinRibbon => {
                self.ribbon.set_minimize(false);
                self.log(String::from("Ribbon pinned open"));
            }
            Message::TogglePictureTools => {
                if self.ribbon.is_contextual_tab_shown("Picture Tools") {
                    self.ribbon.hide_contextual_tab("Picture Tools");
                    self.log("Context category hidden: Picture Tools".to_string());
                } else {
                    self.ribbon
                        .show_contextual_tab_color("Picture Tools", Color::from_rgb(0.8, 0.2, 0.2));
                    self.log("Context category shown: Picture Tools".to_string());
                }
            }
            Message::ToggleTableTools => {
                if self.ribbon.is_contextual_tab_shown("Table Tools") {
                    self.ribbon.hide_contextual_tab("Table Tools");
                    self.log("Context category hidden: Table Tools".to_string());
                } else {
                    self.ribbon
                        .show_contextual_tab_color("Table Tools", Color::from_rgb(0.2, 0.6, 0.3));
                    self.log("Context category shown: Table Tools".to_string());
                }
            }
            Message::ThemeSelected(theme) => {
                self.ribbon.set_ribbon_theme(theme);
                self.theme_selected = theme;
                self.theme_combo_open = false;
                self.log(format!("Theme: {}", theme.label()));
            }
            Message::LayoutSelected(layout) => {
                self.ribbon.set_layout(layout);
                self.layout_selected = layout;
                self.layout_combo_open = false;
                self.log(format!("Layout: {}", layout.label()));
            }
            Message::ToggleDeleteCategory => {
                self.delete_shown = !self.delete_shown;
                self.log(format!(
                    "Category Delete {}",
                    if self.delete_shown {
                        "added"
                    } else {
                        "removed"
                    }
                ));
            }
            Message::ToggleSizePanel => {
                self.size_shown = !self.size_shown;
                self.log(format!(
                    "Category Size(example long category) {}",
                    if self.size_shown { "added" } else { "removed" }
                ));
            }
            Message::ThemeComboToggle => {
                self.theme_combo_open = !self.theme_combo_open;
                if self.theme_combo_open {
                    self.layout_combo_open = false;
                    self.log(String::from("Theme selector opened"));
                }
            }
            Message::ThemeComboAnchored(rect) => self.theme_combo_anchor = Some(rect),
            Message::LayoutComboToggle => {
                self.layout_combo_open = !self.layout_combo_open;
                if self.layout_combo_open {
                    self.theme_combo_open = false;
                    self.log(String::from("Layout selector opened"));
                }
            }
            Message::LayoutComboAnchored(rect) => self.layout_combo_anchor = Some(rect),
            Message::SearchChanged(s) => self.search = s,
            Message::ToggleWordWrap => {
                self.word_wrap = !self.word_wrap;
                self.log(format!(
                    "Word wrap: {}",
                    if self.word_wrap { "On" } else { "Off" }
                ));
            }
            Message::ToggleRtl => {
                self.rtl = !self.rtl;
                self.log(format!("Layout: {}", if self.rtl { "RTL" } else { "LTR" }));
            }
            Message::AlignLeft => {
                self.alignment = Alignment::Start;
                self.log(String::from("Aligned: Left"));
            }
            Message::AlignCenter => {
                self.alignment = Alignment::Center;
                self.log(String::from("Aligned: Center"));
            }
            Message::AlignRight => {
                self.alignment = Alignment::End;
                self.log(String::from("Aligned: Right"));
            }
            Message::Ran(label) => {
                self.log(format!("Ran: {label}"));
                self.ribbon.close_dropdown();
            }
            Message::TitleBarPressed => {
                let Some(id) = self.window_id else {
                    return Task::none();
                };
                let now = Instant::now();
                let is_double = self
                    .last_title_press
                    .map(|last| now.duration_since(last) < Duration::from_millis(500))
                    .unwrap_or(false);
                self.last_title_press = Some(now);
                if is_double {
                    return iced::window::toggle_maximize(id);
                }
                return iced::window::drag(id);
            }
            Message::WindowMinimize => {
                let Some(id) = self.window_id else {
                    return Task::none();
                };
                return iced::window::minimize(id, true);
            }
            Message::WindowToggleMaximize => {
                let Some(id) = self.window_id else {
                    return Task::none();
                };
                return iced::window::toggle_maximize(id);
            }
            Message::WindowClose => {
                let Some(id) = self.window_id else {
                    return Task::none();
                };
                return iced::window::close(id);
            }
            Message::WindowId(id) => {
                self.window_id = Some(id);
            }
            Message::SwitchLanguage("en") => {
                self.i18n = I18n::new(
                    FluentTranslator::from_str("en", include_str!("../locales/en/main.ftl"))
                        .expect("valid en ftl"),
                );
            }
            Message::SwitchLanguage("zh-CN") => {
                self.i18n = I18n::new(
                    FluentTranslator::from_str("zh-CN", include_str!("../locales/zh-CN/main.ftl"))
                        .expect("valid zh-CN ftl"),
                );
            }
            Message::SwitchLanguage(_) => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message, Theme> {
        let content = container(
            column![
                self.build_title_bar(),
                self.build_ribbon(),
                self.build_content_area(),
                self.build_status_bar(),
            ]
            .spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let mut open = Layer::new();
        // Ribbon tool dropdowns.
        if let (Some(anchor), Some(id)) = (self.anchor, self.ribbon.open_dropdown.as_deref()) {
            let panel = stack![
                popover_dismiss_area(Message::CloseDropdown),
                Dropdown::new(panel_items(id, &self.i18n))
                    .anchor(anchor.x, anchor.y + anchor.height + PANEL_GAP),
            ];
            open = open.dropdown(panel);
        }
        // The application button's menu.
        if self.app_menu_open {
            if let Some(anchor) = self.app_menu_anchor {
                let panel = stack![
                    popover_dismiss_area(Message::CloseDropdown),
                    Dropdown::new(app_menu_items(&self.i18n))
                        .anchor(anchor.x, anchor.y + anchor.height),
                ];
                open = open.dropdown(panel);
            }
        }
        // The theme selector's panel: a searchable list of the ten built-in
        // styles, anchored beneath its trigger.
        if self.theme_combo_open {
            if let Some(anchor) = self.theme_combo_anchor {
                let panel = stack![
                    popover_dismiss_area(Message::CloseDropdown),
                    container(
                        combobox_panel(
                            &self.theme_options,
                            std::slice::from_ref(&self.theme_selected),
                            "",
                            Message::ThemeSelected,
                        )
                        .width(180.0)
                        .max_height(280.0)
                        .into_element(),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Start)
                    .align_y(Alignment::Start)
                    .padding(Padding {
                        top: (anchor.y + anchor.height + PANEL_GAP).max(0.0),
                        right: 0.0,
                        bottom: 0.0,
                        left: anchor.x.max(0.0),
                    }),
                ];
                open = open.dropdown(panel);
            }
        }
        // The layout selector's panel: the adaptive default plus the six
        // panel styles.
        if self.layout_combo_open {
            if let Some(anchor) = self.layout_combo_anchor {
                let panel = stack![
                    popover_dismiss_area(Message::CloseDropdown),
                    container(
                        combobox_panel(
                            &self.layout_options,
                            std::slice::from_ref(&self.layout_selected),
                            "",
                            Message::LayoutSelected,
                        )
                        .width(180.0)
                        .max_height(280.0)
                        .into_element(),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Start)
                    .align_y(Alignment::Start)
                    .padding(Padding {
                        top: (anchor.y + anchor.height + PANEL_GAP).max(0.0),
                        right: 0.0,
                        bottom: 0.0,
                        left: anchor.x.max(0.0),
                    }),
                ];
                open = open.dropdown(panel);
            }
        }

        overlay::layer(content, open)
    }

    // ── Title bar (SARibbon style: one tinted area holding everything) ───

    fn build_title_bar(&self) -> Element<'_, Message, Theme> {
        let i18n = &self.i18n;
        let rt = self.ribbon.ribbon_theme;
        let accent = rt.accent();

        // Row 1 — application button, quick access, title, window controls.
        // Every icon on the tinted bar shares one face: a bare glyph that
        // takes a translucent white wash on hover, so the bar reads as one
        // surface rather than a row of separate controls.

        let app_btn = trigger(
            Button::new(i18n.tr("app-button")).on_press(Message::AppMenuToggle),
            Message::AppMenuAnchored,
        );

        let qab_row = row![
            kit_button::icon_button()
                .icon(IconName::Save)
                .tooltip_at(i18n.tr("save-action"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::Ran("save")),
            kit_button::icon_button()
                .icon(IconName::Undo2)
                .tooltip_at(i18n.tr("undo-action"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::Ran("undo")),
            kit_button::icon_button()
                .icon(IconName::Redo2)
                .tooltip_at(i18n.tr("redo-action"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::Ran("redo")),
            kit_button::icon_button()
                .icon(IconName::Printer)
                .tooltip_at(i18n.tr("print-action"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::Ran("print")),
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        let win_controls = row![
            kit_button::icon_button()
                .icon(IconName::Minus)
                .tooltip_at(i18n.tr("minimize-window"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::WindowMinimize),
            kit_button::icon_button()
                .icon(IconName::Square)
                .tooltip_at(i18n.tr("maximize-window"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::WindowToggleMaximize),
            kit_button::icon_button()
                .icon(IconName::X)
                .tooltip_at(i18n.tr("close-window"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::WindowClose),
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        // The drag area covers everything but the window controls: pressing
        // it drags the window, and a double press toggles maximize — the
        // behaviour the default title bar provided, which this one replaces.
        let drag_area = mouse_area(row![
            app_btn,
            qab_row,
            container(text("")).width(Length::Fill),
            text(i18n.tr("window-title")).size(13),
        ])
        .on_press(Message::TitleBarPressed);

        let top_row = row![
            drag_area,
            container(text("")).width(Length::Fill),
            win_controls,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .padding(Padding::from([2.0, 8.0]));

        // Row 2 — the tabs (themed) + the right button group, still on the
        // tinted area, exactly where SARibbon puts them.
        let strip_colors = TabStripColors {
            text: Some(Color::WHITE),
            text_selected: Some(rt.tool_text()),
            text_hover: Some(Color::WHITE),
            selected_background: Some(rt.group_background()),
            selected_border: Some(rt.border()),
            indicator: Some(Color::WHITE),
        };

        let tab_strip = tabs(
            self.tab_labels()
                .iter()
                .map(|label| Tab::new(label.clone()))
                .collect(),
            self.ribbon.active,
            Message::TabSelected,
        )
        .variant(TabVariant::Tab)
        .size(Size::Sm)
        .colors(strip_colors);

        // The right button group: pin (un-minimize), help, and the two context
        // categories' show/hide toggles — SARibbon's createRightButtonGroup and
        // its two context categories, adapted.
        let right_group = row![
            kit_button::icon_button()
                .icon(if self.ribbon.minimized {
                    IconName::ChevronDown
                } else {
                    IconName::Pin
                })
                .tooltip_at(
                    i18n.tr(if self.ribbon.minimized {
                        "pin-ribbon"
                    } else {
                        "minimize-ribbon"
                    }),
                    TooltipPosition::Bottom,
                )
                .ghost()
                .on_press(if self.ribbon.minimized {
                    Message::PinRibbon
                } else {
                    Message::ToggleMinimize
                }),
            kit_button::icon_button()
                .icon(IconName::CircleHelp)
                .tooltip_at(i18n.tr("help-action"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::Ran("help")),
            kit_button::icon_button()
                .icon(if self.ribbon.is_contextual_tab_shown("Picture Tools") {
                    IconName::Eye
                } else {
                    IconName::EyeOff
                })
                .tooltip_at(i18n.tr("toggle-picture-tools"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::TogglePictureTools),
            kit_button::icon_button()
                .icon(if self.ribbon.is_contextual_tab_shown("Table Tools") {
                    IconName::Eye
                } else {
                    IconName::EyeOff
                })
                .tooltip_at(i18n.tr("toggle-table-tools"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::ToggleTableTools),
            kit_button::icon_button()
                .icon(if self.delete_shown {
                    IconName::Trash2
                } else {
                    IconName::Plus
                })
                .tooltip_at(i18n.tr("toggle-delete-category"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::ToggleDeleteCategory),
            kit_button::icon_button()
                .icon(if self.size_shown {
                    IconName::Ruler
                } else {
                    IconName::Plus
                })
                .tooltip_at(i18n.tr("toggle-size-category"), TooltipPosition::Bottom)
                .ghost()
                .on_press(Message::ToggleSizePanel),
        ]
        .spacing(2)
        .align_y(Alignment::Center);

        // The search box, styled for the tinted title bar: a translucent
        // surface with a light rule, rather than the page-field's white.
        let search = container(
            text_input(&i18n.tr("search"), &self.search)
                .on_input(Message::SearchChanged)
                .width(150.0),
        )
        .class(Box::new(move |theme: &Theme| {
            let colors = theme.colors();
            container::Style {
                background: Some(iced::Background::Color(Color {
                    a: 0.12,
                    ..colors.primary
                })),
                text_color: Some(Color::WHITE),
                border: iced::Border {
                    color: Color {
                        a: 0.35,
                        ..colors.muted_foreground
                    },
                    width: 1.0,
                    radius: f32::from(theme.radius().sm).into(),
                },
                ..container::Style::default()
            }
        }) as container::StyleFn<'_, Theme>);

        // The theme and layout selectors: one ComboBox each, replacing the
        // rows of buttons the title bar used to carry. Each carries a small
        // label so the two arrows read as labelled controls rather than
        // anonymous buttons. The triggers share the title bar's face — a
        // bare field that takes a translucent white wash on hover — rather
        // than the combobox's default white page-field.
        fn combo_style(
            theme: &Theme,
            status: iced::widget::button::Status,
        ) -> iced::widget::button::Style {
            let colors = theme.colors();
            iced::widget::button::Style {
                background: Some(iced::Background::Color(
                    if matches!(status, iced::widget::button::Status::Hovered) {
                        Color {
                            a: 0.18,
                            ..colors.primary
                        }
                    } else {
                        Color::TRANSPARENT
                    },
                )),
                text_color: Color::WHITE,
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: f32::from(theme.radius().sm).into(),
                },
                shadow: iced::Shadow::default(),
                snap: true,
            }
        }

        let theme_combo = trigger(
            row![
                text(i18n.tr("theme-label")).size(12),
                container(
                    combobox(
                        &self.theme_options,
                        Some(self.theme_selected),
                        i18n.tr("theme-label"),
                    )
                    .on_toggle(Message::ThemeComboToggle)
                    .class(Box::new(combo_style) as Box<_>),
                )
                .width(110.0),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            Message::ThemeComboAnchored,
        );

        let layout_combo = trigger(
            row![
                text(i18n.tr("layout-label")).size(12),
                container(
                    combobox(
                        &self.layout_options,
                        Some(self.layout_selected),
                        i18n.tr("layout-label"),
                    )
                    .on_toggle(Message::LayoutComboToggle)
                    .class(Box::new(combo_style) as Box<_>),
                )
                .width(110.0),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            Message::LayoutComboAnchored,
        );

        let tab_row = row![
            tab_strip,
            container(text("")).width(Length::Fill),
            layout_combo,
            theme_combo,
            search,
            right_group,
        ]
        .spacing(8)
        .align_y(Alignment::End)
        .padding(Padding::from([0.0, 8.0]));

        // Both rows share one tinted container, so the tabs read as part of
        // the title bar rather than as a separate strip beneath it.
        container(column![top_row, tab_row].spacing(0))
            .width(Length::Fill)
            .class(
                Box::new(move |_theme: &Theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(accent)),
                    text_color: Some(Color::WHITE),
                    ..iced::widget::container::Style::default()
                }) as iced::widget::container::StyleFn<'_, Theme>,
            )
            .into()
    }

    // ── Ribbon (group band only; the tabs live in the title bar) ────────

    /// The tab labels in strip order, with the dynamically-added categories
    /// included when they are shown.
    fn tab_labels(&self) -> Vec<String> {
        let i18n = &self.i18n;
        let mut labels = vec![
            i18n.tr("tab-home"),
            i18n.tr("tab-insert"),
            i18n.tr("tab-page-layout"),
            i18n.tr("tab-view"),
            i18n.tr("tab-developer"),
        ];
        if self.delete_shown {
            labels.push(i18n.tr("tab-delete"));
        }
        if self.size_shown {
            labels.push(i18n.tr("tab-size"));
        }
        labels
    }

    /// Builds the ribbon's tabs from the app's current state, so the dynamic
    /// categories can come and go without the ribbon noticing.
    fn build_ribbon_tabs(&self) -> Vec<RibbonTab<Message>> {
        let mut tabs = vec![
            self.home_tab(),
            self.insert_tab(),
            self.page_layout_tab(),
            self.view_tab(),
            self.developer_tab(),
        ];
        if self.delete_shown {
            tabs.push(self.delete_tab());
        }
        if self.size_shown {
            tabs.push(self.size_tab());
        }
        tabs
    }

    /// The "Delete" category: a panel that is added and removed at runtime,
    /// the reference's createCategoryDelete demonstration.
    fn delete_tab(&self) -> RibbonTab<Message> {
        let i18n = &self.i18n;
        RibbonTab::new(i18n.tr("tab-delete"))
            .group(
                RibbonGroup::new(i18n.tr("group-remove"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Trash2)
                            .label(i18n.tr("delete-row"))
                            .on_press(Message::Ran("delete-row")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Eraser)
                            .label(i18n.tr("clear"))
                            .on_press(Message::Ran("clear")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-add"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Plus)
                            .label(i18n.tr("insert"))
                            .on_press(Message::Ran("insert")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::FilePlus2)
                            .label(i18n.tr("new-item"))
                            .on_press(Message::Ran("new-item")),
                    )),
            )
    }

    /// The "Size" category, whose long title demonstrates the strip handling
    /// a category name too long to fit beside the others.
    fn size_tab(&self) -> RibbonTab<Message> {
        let i18n = &self.i18n;
        RibbonTab::new(i18n.tr("tab-size")).group(
            RibbonGroup::new(i18n.tr("group-sizing"))
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Expand)
                        .label(i18n.tr("stretch"))
                        .on_press(Message::Ran("stretch")),
                ))
                .item(RibbonItem::large(
                    RibbonTool::named(IconName::Shrink)
                        .label(i18n.tr("shrink"))
                        .on_press(Message::Ran("shrink")),
                ))
                .item(RibbonItem::tool(
                    RibbonTool::named(IconName::Move)
                        .label(i18n.tr("scale"))
                        .on_press(Message::Ran("scale")),
                )),
        )
    }

    fn build_ribbon(&self) -> Element<'_, Message, Theme> {
        Ribbon::new()
            .tabs(self.build_ribbon_tabs())
            .state(&self.ribbon)
            .show_tab_strip(false)
            .on_select(Message::TabSelected)
            .on_dropdown_toggle(Message::DropdownToggled)
            .on_dropdown_anchor(Message::RibbonAnchored)
            .into()
    }

    // ── Ribbon tabs ─────────────────────────────────────────────────────

    fn home_tab(&self) -> RibbonTab<Message> {
        let i18n = &self.i18n;
        RibbonTab::new(i18n.tr("tab-home"))
            .group(
                RibbonGroup::new(i18n.tr("group-clipboard"))
                    .item(RibbonItem::large_dropdown(
                        "paste",
                        IconName::ClipboardPaste,
                        i18n.tr("paste"),
                        vec![],
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Copy)
                            .label(i18n.tr("copy"))
                            .on_press(Message::Ran("copy")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Scissors)
                            .label(i18n.tr("cut"))
                            .on_press(Message::Ran("cut")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Paintbrush)
                            .label(i18n.tr("format-painter"))
                            .on_press(Message::Ran("format-painter")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-font"))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::Bold)
                            .label(i18n.tr("bold"))
                            .selected(true)
                            .on_press(Message::Ran("bold")),
                    ))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::Italic)
                            .label(i18n.tr("italic"))
                            .on_press(Message::Ran("italic")),
                    ))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::Underline)
                            .label(i18n.tr("underline"))
                            .on_press(Message::Ran("underline")),
                    ))
                    .item(RibbonItem::dropdown(
                        "font-color",
                        IconName::Palette,
                        vec![],
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-paragraph"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::AlignLeft)
                            .label(i18n.tr("align-left"))
                            .selected(self.alignment == Alignment::Start)
                            .on_press(Message::AlignLeft),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::AlignCenter)
                            .label(i18n.tr("align-center"))
                            .selected(self.alignment == Alignment::Center)
                            .on_press(Message::AlignCenter),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::AlignRight)
                            .label(i18n.tr("align-right"))
                            .selected(self.alignment == Alignment::End)
                            .on_press(Message::AlignRight),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::AlignJustify)
                            .label(i18n.tr("align-justify"))
                            .on_press(Message::Ran("align-justify")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-styles")).item(RibbonItem::gallery(
                    RibbonGallery::new("styles", i18n.tr("quick-styles"))
                        .item(
                            RibbonGalleryItem::new(IconName::Heading1)
                                .label(i18n.tr("heading-1"))
                                .on_press(Message::Ran("heading-1")),
                        )
                        .item(
                            RibbonGalleryItem::new(IconName::Heading2)
                                .label(i18n.tr("heading-2"))
                                .on_press(Message::Ran("heading-2")),
                        )
                        .item(
                            RibbonGalleryItem::new(IconName::Heading3)
                                .label(i18n.tr("heading-3"))
                                .on_press(Message::Ran("heading-3")),
                        )
                        .item(
                            RibbonGalleryItem::new(IconName::Text)
                                .label(i18n.tr("normal"))
                                .selected(true)
                                .on_press(Message::Ran("normal")),
                        )
                        .item(
                            RibbonGalleryItem::new(IconName::List)
                                .label(i18n.tr("list"))
                                .on_press(Message::Ran("list")),
                        )
                        .item(
                            RibbonGalleryItem::new(IconName::Quote)
                                .label(i18n.tr("quote"))
                                .on_press(Message::Ran("quote")),
                        )
                        .columns(4),
                )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-editing"))
                    .item(RibbonItem::dropdown("find", IconName::Search, vec![]))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Replace)
                            .label(i18n.tr("replace"))
                            .on_press(Message::Ran("replace")),
                    ))
                    .item(RibbonItem::dropdown(
                        "select",
                        IconName::MousePointer2,
                        vec![],
                    )),
            )
            // The reference's button types, adapted: a split action button whose
            // body runs the action while its ▾ opens a menu, a labelled
            // dropdown, and a disabled tool.
            .group(
                RibbonGroup::new(i18n.tr("group-popup-styles"))
                    .item(RibbonItem::action_dropdown(
                        "instant-popup",
                        IconName::Zap,
                        Message::Ran("instant-popup"),
                        vec![],
                    ))
                    .item(RibbonItem::action_dropdown(
                        "delayed-popup",
                        IconName::Timer,
                        Message::Ran("delayed-popup"),
                        vec![],
                    ))
                    .item(RibbonItem::labeled_dropdown(
                        "menu-button",
                        i18n.tr("menu"),
                        IconName::List,
                        vec![],
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Ban)
                            .label(i18n.tr("disabled"))
                            .disabled(true),
                    )),
            )
    }

    fn insert_tab(&self) -> RibbonTab<Message> {
        let i18n = &self.i18n;
        RibbonTab::new(i18n.tr("tab-insert"))
            .group(
                RibbonGroup::new(i18n.tr("group-tables"))
                    .item(RibbonItem::large_dropdown(
                        "insert-table",
                        IconName::Table,
                        i18n.tr("table"),
                        vec![],
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Grid3x3)
                            .label(i18n.tr("quick-table"))
                            .on_press(Message::Ran("quick-table")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-illustrations"))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::Image)
                            .label(i18n.tr("pictures"))
                            .on_press(Message::Ran("pictures")),
                    ))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::ChartColumn)
                            .label(i18n.tr("chart"))
                            .on_press(Message::Ran("chart")),
                    ))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::Shapes)
                            .label(i18n.tr("shapes"))
                            .on_press(Message::Ran("shapes")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-links"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Link)
                            .label(i18n.tr("link"))
                            .on_press(Message::Ran("link")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Bookmark)
                            .label(i18n.tr("bookmark"))
                            .on_press(Message::Ran("bookmark")),
                    )),
            )
    }

    fn page_layout_tab(&self) -> RibbonTab<Message> {
        let i18n = &self.i18n;
        RibbonTab::new(i18n.tr("tab-page-layout"))
            .group(
                RibbonGroup::new(i18n.tr("group-page-setup"))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::FileText)
                            .label(i18n.tr("margins"))
                            .on_press(Message::Ran("margins")),
                    ))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::RectangleHorizontal)
                            .label(i18n.tr("orientation"))
                            .on_press(Message::Ran("orientation")),
                    ))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::Ruler)
                            .label(i18n.tr("size"))
                            .on_press(Message::Ran("size")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-paragraph"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::IndentIncrease)
                            .label(i18n.tr("indent"))
                            .on_press(Message::Ran("indent")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Outdent)
                            .label(i18n.tr("outdent"))
                            .on_press(Message::Ran("outdent")),
                    )),
            )
    }

    fn view_tab(&self) -> RibbonTab<Message> {
        let i18n = &self.i18n;
        RibbonTab::new(i18n.tr("tab-view"))
            .group(
                RibbonGroup::new(i18n.tr("group-show"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Grid3x3)
                            .label(i18n.tr("gridlines"))
                            .on_press(Message::Ran("gridlines")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Ruler)
                            .label(i18n.tr("ruler"))
                            .on_press(Message::Ran("ruler")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::MapPin)
                            .label(i18n.tr("navigation"))
                            .on_press(Message::Ran("navigation")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-zoom"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::ZoomIn)
                            .label(i18n.tr("zoom-in"))
                            .on_press(Message::Ran("zoom-in")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::ZoomOut)
                            .label(i18n.tr("zoom-out"))
                            .on_press(Message::Ran("zoom-out")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Maximize2)
                            .label(i18n.tr("zoom-100"))
                            .on_press(Message::Ran("zoom-100")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-options"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::WrapText)
                            .label(i18n.tr("word-wrap"))
                            .selected(self.word_wrap)
                            .on_press(Message::ToggleWordWrap),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::ArrowLeftRight)
                            .label(i18n.tr("rtl"))
                            .selected(self.rtl)
                            .on_press(Message::ToggleRtl),
                    )),
            )
    }

    fn developer_tab(&self) -> RibbonTab<Message> {
        let i18n = &self.i18n;
        RibbonTab::new(i18n.tr("tab-developer"))
            .group(
                RibbonGroup::new(i18n.tr("group-code"))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::Code)
                            .label(i18n.tr("code"))
                            .on_press(Message::Ran("code")),
                    ))
                    .item(RibbonItem::large(
                        RibbonTool::named(IconName::Terminal)
                            .label(i18n.tr("terminal"))
                            .on_press(Message::Ran("terminal")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Bug)
                            .label(i18n.tr("debug"))
                            .on_press(Message::Ran("debug")),
                    )),
            )
            .group(
                RibbonGroup::new(i18n.tr("group-customize"))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Settings)
                            .label(i18n.tr("options"))
                            .on_press(Message::Ran("options")),
                    ))
                    .item(RibbonItem::tool(
                        RibbonTool::named(IconName::Palette)
                            .label(i18n.tr("colors"))
                            .on_press(Message::Ran("colors")),
                    )),
            )
    }

    // ── Content area ────────────────────────────────────────────────────

    fn build_content_area(&self) -> Element<'_, Message, Theme> {
        let i18n = &self.i18n;
        let heading = text(i18n.tr("log-heading")).size(16);

        // The operation log: every action the ribbon reports is appended here,
        // newest last, as the reference's example does.
        let entries = self
            .log
            .iter()
            .rev()
            .fold(column![].spacing(4).width(Length::Fill), |col, entry| {
                col.push(text(entry.clone()).size(13).width(Length::Fill))
            });

        let doc = container(
            column![heading, entries]
                .spacing(8)
                .padding(16)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        // The scrollable fills everything between the ribbon and the status
        // bar, so the log area reads as the window's body; its content
        // scrolling past the height draws a scrollbar.
        scrollable(doc).height(Length::Fill).into()
    }

    // ── Status bar ──────────────────────────────────────────────────────

    fn build_status_bar(&self) -> Element<'_, Message, Theme> {
        let i18n = &self.i18n;
        StatusBar::new()
            .left(text(i18n.tr("status-page")).size(12))
            .right(
                row![
                    text(self.ribbon.ribbon_theme.label()).size(12),
                    text("  |  ").size(12),
                    text(&self.status).size(12),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .into_element()
    }
}

/// The application button's menu, in the style of Office's File menu.
fn app_menu_items(i18n: &I18n<FluentTranslator>) -> Vec<MenuItem<'static, Message>> {
    vec![
        MenuItem::new(i18n.tr("menu-new"), Message::AppMenuAction("new")).shortcut("Ctrl+N"),
        MenuItem::new(i18n.tr("menu-open"), Message::AppMenuAction("open")).shortcut("Ctrl+O"),
        MenuItem::new(i18n.tr("menu-save"), Message::AppMenuAction("save")).shortcut("Ctrl+S"),
        MenuItem::new(i18n.tr("menu-save-as"), Message::AppMenuAction("save-as")),
        MenuItem::new(i18n.tr("menu-print"), Message::AppMenuAction("print")).shortcut("Ctrl+P"),
        MenuItem::new(i18n.tr("menu-language"), Message::SwitchLanguage("en")),
        MenuItem::new("中文 (zh-CN)", Message::SwitchLanguage("zh-CN")),
        MenuItem::new(i18n.tr("menu-exit"), Message::AppMenuAction("exit")),
    ]
}

fn panel_items(id: &str, i18n: &I18n<FluentTranslator>) -> Vec<MenuItem<'static, Message>> {
    match id {
        "paste" => vec![
            MenuItem::new(
                i18n.tr("keep-source-formatting"),
                Message::Ran("paste-keep-source"),
            )
            .shortcut("Ctrl+V"),
            MenuItem::new(i18n.tr("merge-formatting"), Message::Ran("paste-merge")),
            MenuItem::new(i18n.tr("keep-text-only"), Message::Ran("paste-text-only"))
                .shortcut("Ctrl+Shift+V"),
        ],
        "find" => vec![
            MenuItem::new(i18n.tr("find"), Message::Ran("find")).shortcut("Ctrl+F"),
            MenuItem::new(
                i18n.tr("find-and-replace"),
                Message::Ran("find-and-replace"),
            )
            .shortcut("Ctrl+H"),
        ],
        "select" => vec![
            MenuItem::new(i18n.tr("select-all"), Message::Ran("select-all")).shortcut("Ctrl+A"),
            MenuItem::new(i18n.tr("select-objects"), Message::Ran("select-objects")),
        ],
        "font-color" => vec![
            MenuItem::new(i18n.tr("font-color-auto"), Message::Ran("font-color-auto")),
            MenuItem::new(
                i18n.tr("font-color-black"),
                Message::Ran("font-color-black"),
            ),
            MenuItem::new(i18n.tr("font-color-red"), Message::Ran("font-color-red")),
        ],
        "insert-table" => vec![
            MenuItem::new(i18n.tr("insert-table"), Message::Ran("insert-table")),
            MenuItem::new(i18n.tr("draw-table"), Message::Ran("draw-table")),
            MenuItem::new(i18n.tr("quick-table"), Message::Ran("quick-table")),
        ],
        "instant-popup" => vec![
            MenuItem::new(i18n.tr("insert-row"), Message::Ran("instant-insert-row")),
            MenuItem::new(
                i18n.tr("insert-column"),
                Message::Ran("instant-insert-column"),
            ),
        ],
        "delayed-popup" => vec![
            MenuItem::new(i18n.tr("sort-ascending"), Message::Ran("delayed-sort-asc")),
            MenuItem::new(
                i18n.tr("sort-descending"),
                Message::Ran("delayed-sort-desc"),
            ),
        ],
        "menu-button" => vec![
            MenuItem::new(i18n.tr("apply-style"), Message::Ran("menu-apply-style")),
            MenuItem::new(i18n.tr("modify-style"), Message::Ran("menu-modify-style")),
            MenuItem::new(
                i18n.tr("clear-formatting"),
                Message::Ran("menu-clear-formatting"),
            ),
        ],
        other => vec![MenuItem::new(format!("{other}"), Message::Noop).enabled(false)],
    }
}

/// The theme selector's options, one per built-in SARibbon style.
fn theme_options() -> Vec<ComboBoxOption<RibbonTheme>> {
    RibbonTheme::ALL
        .iter()
        .map(|theme| ComboBoxOption::new(*theme, theme.label()))
        .collect()
}

/// The layout selector's options: the adaptive default plus the reference's
/// six panel styles.
fn layout_options() -> Vec<ComboBoxOption<RibbonLayout>> {
    RibbonLayout::ALL
        .iter()
        .map(|layout| ComboBoxOption::new(*layout, layout.label()))
        .collect()
}
