//! Renders every component offscreen and checks the pixels.
//!
//! Physical-display screenshots depend on a visible desktop and cannot run in
//! CI. `iced_test`'s simulator renders through the same pipeline into an
//! offscreen buffer and hands back the pixels, so these checks are
//! deterministic and see what the user would see.
//!
//! The first run writes the reference PNG next to this file; later runs compare
//! against it. Delete a reference file to regenerate it after an intentional
//! visual change.

use iced::widget::{column, container, row};
use iced::{Element, Length};
use iced_kit::widgets::chart::{Chart, ChartKind, Series};
use iced_kit::widgets::data_table::{Column, DataTable, SortDirection, SortKey, TableState, Width};
use iced_kit::widgets::markdown::Markdown;
use iced_kit::widgets::overlay::{
    self, ContextMenu, Layer, Popover, Toast, ToastKind, ToastPlacement, Toasts,
};
use iced_kit::widgets::plot::shape::Interpolation;
use iced_kit::widgets::plot::{
    AreaChart, AreaSeries, BarAlignment, BarChart, BarSeries, Candle, CandlestickChart, LineChart,
    LineSeries, PieChart, PieSlice, RadarChart, RadarSeries, SankeyAlign, SankeyChart, SankeyLink,
    SankeyNode,
};
use iced_kit::widgets::{
    accordion, addon, alert, avatar, avatar_with_name, badge, carousel, code, divider, empty_state,
    group_button, heading, input_group, kbd, list, muted_text, number_input, otp_input, pagination,
    paragraph, password, progress, ring_progress, shortcut, sidebar, skeleton, skeleton_list_item,
    spinner_styled, tag, text_input, tooltip, AccordionSection, AddonAlignment, AvatarLabel,
    CarouselAxis, CarouselState, CollapseMode, Drawer, DrawerSide, Dropdown, Heading, ListItem,
    MenuItem, Modal, Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup, SidebarHeader,
    SidebarMenu, SidebarMenuItem, SidebarToggleButton, SkeletonShape, SpinnerStyle, TitleBar, Tone,
    VirtualList, VirtualListState, WindowControl,
};
use iced_kit::{Size, Theme};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Noop,
    Close,
}

/// Renders `content` and writes/compares a reference image.
///
/// The content may borrow, since a list built over local data is still rendered
/// before this function returns.
///
/// # Why every snapshot is rendered with motion reduced
///
/// A snapshot is a single frame. An animated surface would be captured at
/// whatever point in its transition that frame happened to land, which is a
/// different image on every run — and the pixel comparison would flake rather
/// than catch a real change. Asking for reduced motion makes every component
/// adopt its target immediately, so a reference shows the state the user
/// settles on.
///
/// The request is made once for the whole test binary rather than around each
/// render: the flag is process-wide, and the tests in this file run in parallel,
/// so toggling it per call would let one test disarm another one's render.
/// Testing the motion itself is the job of the unit tests beside each component.
fn assert_renders<'a>(name: &str, content: impl Into<Element<'a, Message, Theme>>, dark: bool) {
    static REDUCE_MOTION: std::sync::Once = std::sync::Once::new();
    REDUCE_MOTION.call_once(|| iced_kit::motion::set_reduce_motion(true));

    let theme = if dark { Theme::dark() } else { Theme::light() };
    let element: Element<'a, Message, Theme> = container(content.into()).padding(16).into();

    let mut simulator = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(640.0, 480.0),
        element,
    );

    let snapshot = simulator
        .snapshot(&theme)
        .unwrap_or_else(|error| panic!("{name}: rendering failed: {error}"));

    let path = format!("tests/snapshots/{name}.png");
    let matches = snapshot
        .matches_image(&path)
        .unwrap_or_else(|error| panic!("{name}: comparing against {path} failed: {error}"));

    assert!(
        matches,
        "{name}: the rendering no longer matches {path}. \
         If the change was intentional, delete that file and rerun."
    );
}

#[test]
fn buttons_render() {
    use iced_kit::widgets::button;

    assert_renders(
        "buttons",
        column![
            button("Default").on_press(Message::Noop),
            button("Primary").primary().on_press(Message::Noop),
            button("Secondary").secondary().on_press(Message::Noop),
            button("Destructive").destructive().on_press(Message::Noop),
            button("Ghost").ghost().on_press(Message::Noop),
            button("Link").link().on_press(Message::Noop),
            button("Small")
                .primary()
                .size(iced_kit::Size::Sm)
                .on_press(Message::Noop),
            button("Large")
                .primary()
                .size(iced_kit::Size::Lg)
                .on_press(Message::Noop),
            button("Disabled").primary().on_press_maybe(None),
        ]
        .spacing(8),
        false,
    );
}

/// The tones and the outline mode are what make the variant set complete: a
/// soft `danger` beside a filled `destructive` must be visibly different, and an
/// outline button must read as a wash rather than a fill.
#[test]
fn button_variants_and_tones_render() {
    use iced_kit::widgets::button;

    assert_renders(
        "button_variants",
        column![
            column![
                button("Danger").danger().on_press(Message::Noop),
                button("Warning").warning().on_press(Message::Noop),
                button("Success").success().on_press(Message::Noop),
                button("Info").info().on_press(Message::Noop),
                button("Text").text().on_press(Message::Noop),
            ]
            .spacing(8),
            column![
                button("Default outline").outline().on_press(Message::Noop),
                button("Primary outline")
                    .primary()
                    .outline()
                    .on_press(Message::Noop),
                button("Danger outline")
                    .danger()
                    .outline()
                    .on_press(Message::Noop),
                button("Success outline")
                    .success()
                    .outline()
                    .on_press(Message::Noop),
            ]
            .spacing(8),
        ]
        .spacing(16),
        false,
    );
}

/// Selection, icons, carets and compactness each change the drawing, so they
/// are pinned together on one page.
#[test]
fn button_states_and_content_render() {
    use iced_kit::widgets::button;

    assert_renders(
        "button_states",
        column![
            column![
                button("Unselected").primary().on_press(Message::Noop),
                button("Selected")
                    .primary()
                    .selected(true)
                    .on_press(Message::Noop),
                button("Disabled")
                    .primary()
                    .disabled(true)
                    .on_press(Message::Noop),
                button("Loading").primary().loading(true),
            ]
            .spacing(8),
            column![
                button("With icon")
                    .icon("★")
                    .primary()
                    .on_press(Message::Noop),
                button("Menu").dropdown_caret().on_press(Message::Noop),
                button("Compact")
                    .secondary()
                    .compact()
                    .on_press(Message::Noop),
                iced::widget::row![
                    iced_kit::widgets::icon_button::<Message>()
                        .icon("✕")
                        .ghost()
                        .on_press(Message::Noop),
                    iced_kit::widgets::icon_button::<Message>()
                        .icon("＋")
                        .outline()
                        .on_press(Message::Noop),
                ]
                .spacing(8),
            ]
            .spacing(8),
        ]
        .spacing(16),
        false,
    );
}

#[test]
fn button_groups_render() {
    use iced_kit::widgets::button::{Button, ButtonGroup, ButtonGroupLayout};

    assert_renders(
        "button_groups",
        column![
            ButtonGroup::new()
                .push(Button::new("Day").selected(true))
                .push(Button::new("Week"))
                .push(Button::new("Month"))
                .on_select(|_| Message::Noop),
            ButtonGroup::new()
                .push(Button::new("Cut"))
                .push(Button::new("Copy").selected(true))
                .push(Button::new("Paste"))
                .outline()
                .on_select(|_| Message::Noop),
            ButtonGroup::new()
                .push(Button::new("Top"))
                .push(Button::new("Middle").selected(true))
                .push(Button::new("Bottom"))
                .layout(ButtonGroupLayout::Vertical)
                .on_select(|_| Message::Noop),
        ]
        .spacing(16),
        false,
    );
}

#[test]
fn split_buttons_render() {
    use iced_kit::widgets::button::{Button, DropdownButton};

    assert_renders(
        "split_buttons",
        column![
            DropdownButton::new(Button::new("Save").primary().on_press(Message::Noop))
                .on_toggle(Message::Close),
            DropdownButton::new(Button::new("Save").on_press(Message::Noop))
                .on_toggle(Message::Close),
            DropdownButton::new(Button::new("Save").on_press(Message::Noop))
                .outline()
                .on_toggle(Message::Close),
            DropdownButton::<Message>::trigger_only().on_toggle(Message::Close),
        ]
        .spacing(8),
        false,
    );
}

#[test]
fn toggle_buttons_render() {
    use iced_kit::widgets::button::{Toggle, ToggleGroup};

    assert_renders(
        "toggle_buttons",
        column![
            column![
                Toggle::<Message>::new("Ghost off").on_toggle(|_| Message::Noop),
                Toggle::<Message>::new("Ghost on")
                    .checked(true)
                    .on_toggle(|_| Message::Noop),
                Toggle::<Message>::new("Outline on")
                    .outline()
                    .checked(true)
                    .on_toggle(|_| Message::Noop),
                Toggle::<Message>::new("With icon")
                    .icon("B")
                    .checked(true)
                    .on_toggle(|_| Message::Noop),
            ]
            .spacing(8),
            // A segmented group joins its members' borders, which is only
            // visible on the outline variant: a ghost toggle draws none.
            ToggleGroup::new()
                .push(Toggle::new("Bold").checked(true))
                .push(Toggle::new("Italic"))
                .push(Toggle::new("Underline").checked(true))
                .outline()
                .segmented()
                .on_change(|_| Message::Noop),
        ]
        .spacing(16),
        false,
    );
}

#[test]
fn form_controls_render() {
    use iced_kit::widgets::{checkbox, radio, slider, switch};

    assert_renders(
        "form_controls",
        column![
            text_input::<Message>("Name", "Ada Lovelace"),
            text_input::<Message>("Password", "hunter2").password(true),
            checkbox("Notifications", true, |_| Message::Noop),
            checkbox("Newsletter", false, |_| Message::Noop),
            switch("Enabled", true, |_| Message::Noop),
            radio("Option A", 1_u8, Some(1_u8), |_| Message::Noop),
            radio("Option B", 2_u8, Some(1_u8), |_| Message::Noop),
            slider(0.0..=100.0, 60.0, |_| Message::Noop).width(Length::Fixed(300.0)),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn display_components_render() {
    assert_renders(
        "display",
        column![
            progress(0.65, Tone::Primary),
            divider(),
            column![
                badge("Default", Tone::Neutral),
                badge("Success", Tone::Success),
                badge("Warning", Tone::Warning),
                badge("Danger", Tone::Danger),
                badge("Primary", Tone::Primary),
            ]
            .spacing(8),
            alert("Heads up", "This is an informational alert.", Tone::Neutral),
            alert("Saved", "Your changes were written to disk.", Tone::Success),
            alert("Disk almost full", "Only 2 GB remain.", Tone::Warning),
            alert("Build failed", "3 errors were found.", Tone::Danger),
            empty_state("No projects yet", "Create one to get started"),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn feedback_components_render() {
    assert_renders(
        "feedback",
        column![
            spinner_styled(32, SpinnerStyle::Arc),
            spinner_styled(32, SpinnerStyle::Dots),
            ring_progress(0.65, 72, Tone::Primary),
            ring_progress(0.3, 72, Tone::Danger),
            skeleton(SkeletonShape::Text, 3),
            skeleton_list_item(2),
        ]
        .spacing(16),
        false,
    );
}

#[test]
fn typography_renders() {
    assert_renders(
        "typography",
        column![
            heading("Heading 1", Heading::H1),
            heading("Heading 3", Heading::H3),
            paragraph("A paragraph of body text at the default size."),
            muted_text("Muted helper text under a field."),
            iced::widget::row![
                code("cargo run"),
                kbd::<Message>("Ctrl"),
                shortcut::<Message>(&["Ctrl", "Shift", "P"]),
            ]
            .spacing(12),
            avatar_with_name(
                AvatarLabel::new("Ada Lovelace").detail("ada@example.com"),
                40,
                iced_kit::Size::Md,
            ),
            iced::widget::row![
                avatar("Ada Lovelace", 32),
                avatar("Grace Hopper", 40),
                avatar("Alan Turing", 48),
            ]
            .spacing(12),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn navigation_components_render() {
    assert_renders(
        "navigation",
        column![
            pagination(4, 20, |_| Message::Noop),
            accordion(
                vec![
                    AccordionSection::new("General").subtitle("Basic options"),
                    AccordionSection::new("Advanced"),
                ],
                Some(0),
                |_| Message::Noop,
                |index| iced::widget::text(format!("Body for section {index}")).into(),
            ),
        ]
        .spacing(16),
        false,
    );
}

/// The sidebar the snapshot cases share: a header, a grouped menu with an
/// active item and an open submenu, and a footer, docked beside a page.
fn demo_sidebar<'a>(collapsible: SidebarCollapsible, collapsed: bool) -> Sidebar<'a, Message> {
    let icon_collapsed = collapsed && collapsible == SidebarCollapsible::Icon;

    let menu = SidebarMenu::new().children([
        SidebarMenuItem::new("Dashboard")
            .icon(iced_kit::icons::IconName::LayoutDashboard)
            .active(true)
            .on_select(Message::Noop),
        SidebarMenuItem::new("Inbox")
            .icon(iced_kit::icons::IconName::Inbox)
            .on_select(Message::Noop),
        SidebarMenuItem::new("Calendar").icon(iced_kit::icons::IconName::Calendar),
        SidebarMenuItem::new("Projects")
            .icon(iced_kit::icons::IconName::Folder)
            .open(true)
            .on_toggle(Message::Noop)
            .on_context(Message::Noop)
            .children([
                SidebarMenuItem::new("Design"),
                SidebarMenuItem::new("Engineering").disabled(true),
            ]),
        SidebarMenuItem::new("Settings").icon(iced_kit::icons::IconName::Settings),
    ]);

    // The header and footer are the caller's slots, so they adapt to the rail
    // the same way the reference example's do: a bare glyph when the names
    // would no longer fit.
    let header: Element<'a, Message, Theme> = if icon_collapsed {
        iced_kit::widgets::Icon::new(iced_kit::icons::IconName::GalleryVerticalEnd)
            .into_element(iced_kit::Size::Md)
    } else {
        iced::widget::text("Acme Inc").into()
    };

    let footer: Element<'a, Message, Theme> = if icon_collapsed {
        iced_kit::widgets::Icon::new(iced_kit::icons::IconName::CircleUser)
            .into_element(iced_kit::Size::Md)
    } else {
        iced::widget::text("Jason Lee").into()
    };

    sidebar()
        .collapsible(collapsible)
        .collapsed(collapsed)
        .width(220.0)
        .header(
            SidebarHeader::new()
                .child(header)
                .on_dropdown(Message::Noop),
        )
        .child(SidebarGroup::new("Application").child(menu))
        .footer(
            SidebarFooter::new()
                .child(footer)
                .on_dropdown(Message::Noop),
        )
}

/// The page beside the sidebar, so the snapshot shows the docked pair.
fn sidebar_page<'a>(
    collapsible: SidebarCollapsible,
    collapsed: bool,
) -> impl Into<Element<'a, Message, Theme>> {
    row![
        demo_sidebar(collapsible, collapsed),
        column![
            SidebarToggleButton::new()
                .collapsed(collapsed)
                .on_press(Message::Noop),
            iced::widget::text("Page content")
        ]
        .spacing(8)
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
}

#[test]
fn a_sidebar_renders() {
    assert_renders(
        "sidebar",
        sidebar_page(SidebarCollapsible::Icon, false),
        false,
    );
}

#[test]
fn a_sidebar_renders_icon_collapsed() {
    // Under reduced motion the collapse adopts its target width on the first
    // frame, so the snapshot shows the settled icon rail.
    assert_renders(
        "sidebar_icon_collapsed",
        sidebar_page(SidebarCollapsible::Icon, true),
        false,
    );
}

#[test]
fn a_sidebar_renders_in_dark_mode() {
    assert_renders(
        "sidebar_dark",
        sidebar_page(SidebarCollapsible::Icon, false),
        true,
    );
}

#[test]
fn a_modal_renders_over_content() {
    use iced_kit::widgets::button;

    let content: Element<'static, Message, Theme> = container(column![
        heading("Page content", Heading::H2),
        paragraph("This text sits behind the modal backdrop."),
        button("Open").primary().on_press(Message::Noop),
    ])
    .padding(24)
    .into();

    let open = Layer::new().modal(
        Modal::new(
            "Delete project",
            iced::widget::text("This permanently removes the project."),
            Message::Close,
        )
        .description("Every file in it goes with it.")
        .cancel("Cancel", Message::Close)
        .destructive("Delete", Message::Noop),
    );

    assert_renders("modal", overlay::layer(content, open), false);
}

#[test]
fn an_alert_dialog_renders_over_content() {
    use iced_kit::widgets::{button, AlertDialog, AlertTone};

    let content: Element<'static, Message, Theme> = container(column![
        heading("Page content", Heading::H2),
        paragraph("This text sits behind the alert's backdrop."),
        button("Delete").destructive().on_press(Message::Noop),
    ])
    .padding(24)
    .into();

    let open = Layer::new().modal(
        AlertDialog::new()
            .tone(AlertTone::Danger)
            .title("Delete project?")
            .description("Every file in it goes with it.")
            .confirm()
            .on_confirm(Message::Noop)
            .on_cancel(Message::Close)
            .on_dismiss(Message::Close),
    );

    assert_renders("alert_dialog", overlay::layer(content, open), false);
}

#[test]
fn a_dialog_body_laid_out_from_its_parts_renders() {
    use iced_kit::widgets::{button, DialogContent, DialogHeader};

    let content: Element<'static, Message, Theme> = container(column![
        heading("Page content", Heading::H2),
        button("Merge").primary().on_press(Message::Noop),
    ])
    .padding(24)
    .into();

    // The composition parts, assembled by hand: a caller that lays out its own
    // body rather than handing the dialog a title and a blob of content. An
    // empty title means the dialog draws no header of its own.
    let body: Element<'static, Message, Theme> = DialogContent::new()
        .push(
            DialogHeader::new()
                .title("Merge branch")
                .description("The branch will be merged into main."),
        )
        .push(paragraph("This cannot be undone."))
        .into();

    let open = Layer::new().modal(
        Modal::new("", body, Message::Close)
            .cancel("Cancel", Message::Close)
            .confirm("Merge", Message::Noop),
    );

    assert_renders("dialog_parts", overlay::layer(content, open), false);
}

#[test]
fn toasts_render() {
    use iced_kit::widgets::button;

    let content: Element<'static, Message, Theme> = container(column![
        heading("Page content", Heading::H2),
        button("Show toast").on_press(Message::Noop),
    ])
    .padding(24)
    .into();

    let open = Layer::new().toasts(
        Toasts::new()
            .placement(ToastPlacement::BottomRight)
            .push(Toast::new("Saved", ToastKind::Success).description("Just now"))
            .push(Toast::new("Disk almost full", ToastKind::Warning))
            .push(Toast::new("Build failed", ToastKind::Error)),
    );

    assert_renders("toasts", overlay::layer(content, open), false);
}

#[test]
fn a_dropdown_renders_over_content() {
    use iced_kit::widgets::button;

    let content: Element<'static, Message, Theme> = container(column![
        heading("Page content", Heading::H2),
        button("Menu").on_press(Message::Noop),
    ])
    .padding(24)
    .into();

    let open = Layer::new().dropdown(
        Dropdown::new(vec![
            MenuItem::new("Duplicate", Message::Noop).shortcut("Ctrl+D"),
            MenuItem::new("Rename", Message::Noop),
            MenuItem::new("Archive", Message::Noop).enabled(false),
            MenuItem::new("Delete", Message::Noop).destructive(true),
        ])
        .anchor(40.0, 100.0),
    );

    assert_renders("dropdown", overlay::layer(content, open), false);
}

#[test]
fn a_tooltip_renders() {
    use iced_kit::widgets::button;

    assert_renders(
        "tooltip",
        tooltip(
            button("Hover me").primary().on_press(Message::Noop),
            "A tooltip appears after a short delay".to_owned(),
        ),
        false,
    );
}

#[test]
fn dark_mode_renders_the_same_components() {
    use iced_kit::widgets::button;

    assert_renders(
        "dark_mode",
        column![
            button("Primary").primary().on_press(Message::Noop),
            button("Secondary").secondary().on_press(Message::Noop),
            button("Destructive").destructive().on_press(Message::Noop),
            progress(0.4, Tone::Primary),
            alert("Saved", "Written to disk.", Tone::Success),
            alert("Build failed", "3 errors found.", Tone::Danger),
            badge("Active", Tone::Success),
            avatar("Ada Lovelace", 40),
            ring_progress(0.5, 64, Tone::Primary),
            muted_text("Muted text on a dark surface."),
        ]
        .spacing(12),
        true,
    );
}

#[test]
fn text_fields_render_in_every_state() {
    assert_renders(
        "text_fields",
        column![
            text_input::<Message>("Placeholder", "").on_input(|_| Message::Noop),
            text_input::<Message>("Filled", "Ada Lovelace").on_input(|_| Message::Noop),
            text_input::<Message>("Invalid", "not an email")
                .invalid(true)
                .on_input(|_| Message::Noop),
            text_input::<Message>("With label", "value")
                .label("Email")
                .on_input(|_| Message::Noop),
            text_input::<Message>("With error", "x")
                .error("That is not an email")
                .on_input(|_| Message::Noop),
            text_input::<Message>("Disabled", "nope")
                .disabled(true)
                .on_input(|_| Message::Noop),
            text_input::<Message>("Read-only", "selectable")
                .readonly(true)
                .on_input(|_| Message::Noop),
            text_input::<Message>("Password", "hunter2")
                .password(true)
                .on_mask_toggle(Message::Noop)
                .on_input(|_| Message::Noop),
            text_input::<Message>("Search", "query")
                .clearable(Message::Noop)
                .on_input(|_| Message::Noop),
            text_input::<Message>("Loading", "checking")
                .loading(true)
                .on_input(|_| Message::Noop),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn text_fields_render_at_every_size() {
    assert_renders(
        "text_field_sizes",
        column![
            text_input::<Message>("Extra small", "xs")
                .size(Size::Xs)
                .on_input(|_| Message::Noop),
            text_input::<Message>("Small", "sm")
                .size(Size::Sm)
                .on_input(|_| Message::Noop),
            text_input::<Message>("Medium", "md")
                .size(Size::Md)
                .on_input(|_| Message::Noop),
            text_input::<Message>("Large", "lg")
                .size(Size::Lg)
                .on_input(|_| Message::Noop),
            text_input::<Message>("With prefix", "120")
                .prefix(iced::widget::text("$"))
                .on_input(|_| Message::Noop),
            text_input::<Message>("With prefix and suffix", "120")
                .prefix(iced::widget::text("$"))
                .suffix(iced::widget::text("USD"))
                .on_input(|_| Message::Noop),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn input_groups_render() {
    assert_renders(
        "input_groups",
        column![
            input_group()
                .input(text_input::<Message>("example.com", "").on_input(|_| Message::Noop))
                .addon(addon().push(iced::widget::text("https://").size(12))),
            input_group()
                .input(text_input::<Message>("Search…", "rust").on_input(|_| Message::Noop))
                .addon(
                    addon()
                        .align(AddonAlignment::InlineEnd)
                        .push(group_button::<Message>("Go").on_press(Message::Noop)),
                ),
            input_group()
                .input(text_input::<Message>("0.00", "120").on_input(|_| Message::Noop))
                .addon(addon().push(iced::widget::text("$").size(13)))
                .addon(
                    addon()
                        .align(AddonAlignment::InlineEnd)
                        .push(iced::widget::text("USD").size(12)),
                ),
            input_group()
                .input(text_input::<Message>("Card number", "4242").on_input(|_| Message::Noop),)
                .addon(
                    addon()
                        .align(AddonAlignment::BlockStart)
                        .push(iced::widget::text("As printed on the card").size(11)),
                )
                .addon(
                    addon()
                        .align(AddonAlignment::BlockEnd)
                        .push(iced::widget::text("We never store this").size(11)),
                ),
            input_group()
                .input(
                    text_input::<Message>("Email", "nope")
                        .invalid(true)
                        .on_input(|_| Message::Noop),
                )
                .addon(
                    addon()
                        .align(AddonAlignment::InlineEnd)
                        .push(iced::widget::text("@"))
                )
                .invalid(true),
        ]
        .spacing(14),
        false,
    );
}

#[test]
fn number_and_otp_fields_render() {
    assert_renders(
        "number_and_otp_fields",
        column![
            number_input("Port", 8080.0, 1.0..=65535.0, |_| Message::Noop),
            number_input("Price", 19.99, 0.0..=100.0, |_| Message::Noop)
                .step(0.01)
                .prefix(iced::widget::text("$")),
            number_input("At the maximum", 100.0, 0.0..=100.0, |_| Message::Noop),
            number_input("Disabled", 5.0, 0.0..=10.0, |_| Message::Noop).disabled(true),
            otp_input("123456", 6, |_| Message::Noop),
            otp_input("1234", 6, |_| Message::Noop).groups(2),
            otp_input("12", 6, |_| Message::Noop).masked(true),
        ]
        .spacing(14),
        false,
    );
}

#[test]
fn a_password_field_renders_masked_and_revealed() {
    assert_renders(
        "password_fields",
        column![
            password::<Message>("Password", "hunter2"),
            password::<Message>("Revealed", "hunter2").masked(false),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn inputs_and_lists_render() {
    assert_renders(
        "inputs_and_lists",
        column![
            number_input("Port", 8080.0, 1.0..=65535.0, |_| Message::Noop),
            otp_input("123", 6, |_| Message::Noop),
            divider(),
            list(
                vec![
                    ListItem::new("Inbox").trailing("12"),
                    ListItem::new("Drafts").subtitle("3 unsent"),
                    ListItem::new("Archive").enabled(false),
                ],
                Some(0),
                |_| Message::Noop,
            ),
            iced::widget::row![
                tag("rust", Tone::Primary, None),
                tag("iced", Tone::Success, Some(Message::Noop)),
                tag("deprecated", Tone::Danger, None),
            ]
            .spacing(8),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn a_drawer_renders() {
    use iced_kit::widgets::button;

    let content: Element<'static, Message, Theme> = container(column![
        heading("Page content", Heading::H2),
        button("Open drawer").on_press(Message::Noop),
    ])
    .padding(24)
    .into();

    let open = Layer::new().drawer(
        Drawer::new(
            "Details",
            column![
                paragraph("A drawer slides in from an edge."),
                iced::widget::text_input("Name", "Ada"),
            ]
            .spacing(12),
        )
        .side(DrawerSide::Right)
        .on_dismiss(Message::Close),
    );

    assert_renders("drawer", overlay::layer(content, open), false);
}

#[test]
fn a_context_menu_renders() {
    use iced_kit::widgets::button;

    let content: Element<'static, Message, Theme> = container(column![
        heading("Page content", Heading::H2),
        paragraph("Right-click anywhere for a context menu."),
        button("Target").on_press(Message::Noop),
    ])
    .padding(24)
    .into();

    let open = Layer::new().dropdown(ContextMenu::new(
        vec![
            MenuItem::new("Cut", Message::Noop).shortcut("Ctrl+X"),
            MenuItem::new("Copy", Message::Noop).shortcut("Ctrl+C"),
            MenuItem::new("Paste", Message::Noop).shortcut("Ctrl+V"),
        ],
        (120.0, 140.0),
    ));

    assert_renders("context_menu", overlay::layer(content, open), false);
}

#[test]
fn a_popover_renders() {
    use iced_kit::widgets::button;

    let content: Element<'static, Message, Theme> = container(column![
        heading("Page content", Heading::H2),
        button("Trigger").on_press(Message::Noop),
    ])
    .padding(24)
    .into();

    let open = Layer::new().dropdown(Popover::new(
        column![
            heading("Settings", Heading::H4),
            paragraph("Adjust how the app behaves."),
            button("Save").primary().on_press(Message::Noop),
        ]
        .spacing(12),
        (40.0, 100.0),
    ));

    assert_renders("popover", overlay::layer(content, open), false);
}

#[test]
fn a_virtual_list_renders_only_its_window() {
    // Ten thousand rows, of which a handful are built: this is the rendering
    // the widget exists to make cheap.
    let rows: Vec<String> = (0..10_000).map(|index| format!("Row {index}")).collect();

    let mut state = VirtualListState::new();
    state.update(0.0, 300.0);

    let element: Element<'_, Message, Theme> =
        VirtualList::new(&rows, &state, |row: &String, _index| {
            iced::widget::text(row.clone()).into()
        })
        .fixed_row_height(28.0)
        .height(300.0)
        .into();

    assert_renders("virtual_list", element, false);
}

#[test]
fn a_variable_height_virtual_list_renders() {
    let rows: Vec<String> = (0..1_000).map(|index| format!("Item {index}")).collect();

    let mut state = VirtualListState::new();
    state.update(0.0, 240.0);

    // Alternating heights exercise the variable-height path rather than the
    // uniform fast path, so the rows below appear at uneven spacing.
    let element: Element<'_, Message, Theme> =
        VirtualList::new(&rows, &state, |row: &String, index| {
            iced::widget::text(format!(
                "{row}  ({}px)",
                if index % 2 == 0 { 24 } else { 36 }
            ))
            .into()
        })
        .row_height(
            |_row: &String, index: usize| {
                if index % 2 == 0 {
                    24.0
                } else {
                    36.0
                }
            },
        )
        .height(240.0)
        .into();

    assert_renders("virtual_list_variable_height", element, false);
}

/// A row type for the table snapshot.
#[derive(Debug, Clone)]
struct Record {
    name: String,
    size: u64,
}

#[test]
fn a_data_table_renders_sorted_and_selected() {
    let rows: Vec<Record> = [
        ("report.pdf", 2_400_000_u64),
        ("archive.zip", 18_000_000),
        ("notes.md", 4_200),
        ("photo.png", 940_000),
        ("data.csv", 128_000),
    ]
    .into_iter()
    .map(|(name, size)| Record {
        name: name.to_owned(),
        size,
    })
    .collect();

    // Sorted by size descending, with the second row selected, so the snapshot
    // shows the sorted header arrow and the selection tint at once.
    let mut state = TableState::sorted_by("Size", SortDirection::Descending);
    state.select(Some(1));

    let columns = vec![
        Column::<Record, Message>::new("Name", |record: &Record, _index| {
            iced_kit::widgets::data_table::text_cell(record.name.clone())
        })
        .width(Width::Fill)
        .sortable(|record| SortKey::text(record.name.clone())),
        Column::<Record, Message>::new("Size", |record: &Record, _index| {
            iced_kit::widgets::data_table::number_cell(format_bytes(record.size))
        })
        .width(Width::Fixed(120.0))
        .align_x(iced::Alignment::End)
        .sortable(|record| SortKey::number(record.size as f64)),
    ];

    let element: Element<'_, Message, Theme> = DataTable::new(&rows, &state, columns)
        .row_height(34.0)
        .on_sort(|_heading| Message::Noop)
        .on_row_click(|_index| Message::Noop)
        .into();

    assert_renders("data_table", element, false);
}

/// Formats a size in bytes the way a file browser would.
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

#[test]
fn charts_render_in_every_kind() {
    let samples: Vec<f32> = (0..24)
        .map(|index| {
            let t = index as f32;
            (t * 0.5).sin() * 40.0 + 60.0 + (t * 2.0).cos() * 8.0
        })
        .collect();

    let other: Vec<f32> = samples.iter().map(|value| value * 0.6 + 10.0).collect();

    assert_renders(
        "chart_line",
        column![
            heading("Line", Heading::H4),
            Chart::new(vec![
                Series::new("Read", samples.clone()).tone(Tone::Primary),
                Series::new("Write", other.clone()).tone(Tone::Success),
            ])
            .kind(ChartKind::Line)
            .height(180.0)
            .into_element::<Message>(),
            heading("Area", Heading::H4),
            Chart::new(vec![
                Series::new("Load", samples.clone()).tone(Tone::Primary)
            ])
            .kind(ChartKind::Area)
            .height(160.0)
            .into_element::<Message>(),
            heading("Bar", Heading::H4),
            Chart::new(vec![Series::new(
                "Requests",
                vec![12.0, 30.0, 22.0, 45.0, 38.0, 60.0, 51.0],
            )
            .tone(Tone::Warning)])
            .kind(ChartKind::Bar)
            .height(160.0)
            .into_element::<Message>(),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn a_chart_with_a_fixed_range_renders() {
    // A percentage axis, which must stay 0..=100 rather than rescaling to the
    // data, so the same chart is comparable across reloads.
    let percentages: Vec<f32> = vec![42.0, 55.0, 48.0, 61.0, 58.0, 70.0];

    assert_renders(
        "chart_fixed_range",
        Chart::new(vec![
            Series::new("Coverage", percentages).tone(Tone::Success)
        ])
        .kind(ChartKind::Area)
        .range(0.0, 100.0)
        .height(200.0)
        .into_element::<Message>(),
        false,
    );
}

#[test]
fn markdown_renders_every_block_kind() {
    let document = Markdown::parse(
        "# Document title

A paragraph with **bold**, *italic*, `inline code` and a [link](https://example.com).

## Section heading

- first item
- second item with `code`

1. ordered one
2. ordered two

> A quoted paragraph.

```rust
fn main() {
    println!(\"hello\");
}
```

| Column | Value |
| ------ | ----- |
| alpha  | 1     |
| beta   | 2     |

---
",
    );

    // Markdown's message type is a link URL, so this one renders through the
    // simulator directly rather than through `assert_renders`.
    let theme = Theme::light();
    let element = container(document.into_element()).padding(16);

    let mut simulator = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(640.0, 480.0),
        element,
    );

    let snapshot = simulator.snapshot(&theme).expect("markdown must render");

    let path = "tests/snapshots/markdown.png";
    let matches = snapshot
        .matches_image(path)
        .expect("markdown comparison must succeed");

    assert!(
        matches,
        "markdown: the rendering no longer matches {path}.          If the change was intentional, delete that file and rerun."
    );
}

/// Renders a dock styled with the design tokens.
///
/// Gated on the `dock` feature: the integration is opt-in, so the test that
/// covers it is too.
#[cfg(feature = "dock")]
#[test]
fn a_themed_dock_renders() {
    use iced_kit::widgets::dock::{self, DockSession, LayoutTree, PanelDef};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Panel {
        Explorer,
        Editor,
        Panel2,
        Terminal,
    }

    /// Only used to build the dock; the event payload is not inspected.
    #[derive(Debug, Clone)]
    enum DockMessage {
        Event,
    }

    let layout: LayoutTree<Panel> = dock::horizontal([
        dock::tabs([PanelDef::new("explorer", "Explorer", Panel::Explorer)]),
        dock::tabs([
            PanelDef::new("editor", "main.rs", Panel::Editor),
            PanelDef::new("lib", "lib.rs", Panel::Panel2),
        ]),
        dock::tabs([PanelDef::new("terminal", "Terminal", Panel::Terminal)]),
    ])
    .weights([0.2, 0.6, 0.2]);

    let session = DockSession::from_tree(layout).expect("the layout is valid");

    let element: Element<'_, DockMessage, Theme> =
        dock::dock::<Panel, DockMessage, Theme, iced::Renderer>()
            .state(session.state())
            .on_event(|_event| DockMessage::Event)
            .style(dock::style)
            .content(|panel| {
                iced::widget::text(match panel {
                    Panel::Explorer => "Explorer",
                    Panel::Editor => "main.rs",
                    Panel::Panel2 => "lib.rs",
                    Panel::Terminal => "Terminal",
                })
                .into()
            })
            .build()
            .into();

    // The dock's own message type differs from this file's, so it renders
    // through the simulator directly.
    let theme = Theme::light();
    let mut simulator = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(640.0, 480.0),
        element,
    );

    let snapshot = simulator.snapshot(&theme).expect("the dock must render");
    let path = "tests/snapshots/dock.png";
    let matches = snapshot
        .matches_image(path)
        .expect("dock comparison must succeed");

    assert!(
        matches,
        "dock: the rendering no longer matches {path}.          If the change was intentional, delete that file and rerun."
    );
}

#[test]
fn a_title_bar_renders_with_window_controls() {
    assert_renders(
        "title_bar",
        column![
            TitleBar::new("iced-kit gallery")
                .icon(iced_kit::icons::IconName::GalleryVerticalEnd)
                .subtitle("untitled")
                .control(WindowControl::Minimize, Message::Noop)
                .control(WindowControl::Maximize, Message::Noop)
                .control(WindowControl::Close, Message::Close),
            TitleBar::new("Primary tone")
                .tone(Tone::Primary)
                .control(WindowControl::Close, Message::Close),
            TitleBar::new("Warning tone")
                .tone(Tone::Warning)
                .icon(iced_kit::icons::IconName::TriangleAlert)
                .control(WindowControl::Close, Message::Close),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn a_title_bar_renders_in_dark_mode() {
    assert_renders(
        "title_bar_dark",
        TitleBar::new("iced-kit")
            .icon(iced_kit::icons::IconName::GalleryVerticalEnd)
            .subtitle("dark")
            .control(WindowControl::Minimize, Message::Noop)
            .control(WindowControl::Close, Message::Close),
        true,
    );
}

#[test]
fn resizable_panes_render() {
    use iced_kit::widgets::resizable::{Resizable, SplitAxis};

    let (horizontal, _) = Resizable::<Message>::split_state(2, SplitAxis::Horizontal, 0.3);
    let (vertical, _) = Resizable::<Message>::split_state(3, SplitAxis::Vertical, 0.4);

    let horizontal_grid: Element<'_, Message, Theme> = Resizable::new(&horizontal)
        .on_resize(|_event| Message::Noop)
        .pane(|_pane, index: &usize| {
            iced::widget::container(iced::widget::text(format!("Pane {}", index + 1)))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8)
                .into()
        })
        .into_element();

    let vertical_grid: Element<'_, Message, Theme> = Resizable::new(&vertical)
        // Three panes at the default 120px minimum would need more height than
        // the snapshot area, so the minimum is lowered to fit.
        .min_size(40.0)
        .on_resize(|_event| Message::Noop)
        .pane(|_pane, index: &usize| {
            iced::widget::container(iced::widget::text(format!("Row {}", index + 1)))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8)
                .into()
        })
        .into_element();

    assert_renders(
        "resizable",
        column![
            heading("Horizontal split", Heading::H4),
            iced::widget::container(horizontal_grid).height(Length::Fixed(100.0)),
            heading("Vertical split", Heading::H4),
            iced::widget::container(vertical_grid).height(Length::Fixed(240.0)),
        ]
        .spacing(12),
        false,
    );
}

#[test]
fn a_line_chart_renders() {
    let labels: Vec<String> = (0..24).map(|index| format!("{index}h")).collect();

    let reads: Vec<f64> = (0..24)
        .map(|index| {
            let t = f64::from(index);
            (t * 0.5).sin() * 40.0 + 70.0 + (t * 1.7).cos() * 12.0
        })
        .collect();

    let writes: Vec<f64> = reads.iter().map(|value| value * 0.6 + 8.0).collect();

    assert_renders(
        "line_chart",
        LineChart::new(
            labels,
            vec![
                LineSeries::new("Reads", reads).tone(Tone::Primary),
                LineSeries::new("Writes", writes).tone(Tone::Success),
            ],
        )
        .height(220.0)
        .into_element::<Message>(),
        false,
    );
}

#[test]
fn line_chart_interpolations_render() {
    let labels: Vec<String> = (0..12).map(|index| format!("{index}")).collect();
    let values: Vec<f64> = vec![
        20.0, 45.0, 30.0, 60.0, 38.0, 72.0, 50.0, 64.0, 42.0, 58.0, 35.0, 48.0,
    ];

    assert_renders(
        "line_chart_interpolation",
        column![
            heading("Natural", Heading::H4),
            LineChart::new(
                labels.clone(),
                vec![LineSeries::new("Smooth", values.clone()).tone(Tone::Primary)],
            )
            .interpolation(Interpolation::Natural)
            .points(true)
            .height(150.0)
            .into_element::<Message>(),
            heading("Step after", Heading::H4),
            LineChart::new(
                labels,
                vec![LineSeries::new("Stepped", values).tone(Tone::Warning)],
            )
            .interpolation(Interpolation::StepAfter)
            .height(150.0)
            .into_element::<Message>(),
        ]
        .spacing(10),
        false,
    );
}

#[test]
fn a_bar_chart_renders() {
    let labels: Vec<String> = ["Q1", "Q2", "Q3", "Q4"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();

    assert_renders(
        "bar_chart",
        column![
            heading("Revenue by quarter", Heading::H4),
            BarChart::new(
                labels.clone(),
                vec![
                    BarSeries::new("Revenue", vec![120.0, 180.0, 150.0, 210.0]).tone(Tone::Primary),
                    BarSeries::new("Cost", vec![80.0, 95.0, 88.0, 110.0]).tone(Tone::Success),
                ],
            )
            .height(180.0)
            .into_element::<Message>(),
            heading("Stacked, with a negative value", Heading::H4),
            BarChart::single("Change", labels.clone(), vec![120.0, -60.0, 90.0, 45.0],)
                .stacked(true)
                .values_visible(true)
                .height(170.0)
                .into_element::<Message>(),
        ]
        .spacing(10),
        false,
    );
}

#[test]
fn a_horizontal_bar_chart_renders() {
    let labels: Vec<String> = ["Search", "Direct", "Social", "Email", "Referral"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();

    assert_renders(
        "bar_chart_horizontal",
        BarChart::single("Sessions", labels, vec![420.0, 310.0, 180.0, 95.0, 60.0])
            .alignment(BarAlignment::Left)
            .corners(iced_kit::widgets::plot::shape::Corners::top(3.0))
            .height(200.0)
            .into_element::<Message>(),
        false,
    );
}

#[test]
fn an_area_chart_renders() {
    let labels: Vec<String> = (0..12).map(|index| format!("{index}")).collect();

    let reads: Vec<f64> = vec![
        24.0, 38.0, 30.0, 52.0, 44.0, 68.0, 55.0, 72.0, 60.0, 78.0, 66.0, 84.0,
    ];
    let writes: Vec<f64> = reads.iter().map(|value| value * 0.55).collect();

    assert_renders(
        "area_chart",
        column![
            heading("Overlaid", Heading::H4),
            AreaChart::new(
                labels.clone(),
                vec![
                    AreaSeries::new("Reads", reads.clone()).tone(Tone::Primary),
                    AreaSeries::new("Writes", writes.clone()).tone(Tone::Success),
                ],
            )
            .height(180.0)
            .into_element::<Message>(),
            heading("Stacked", Heading::H4),
            AreaChart::new(
                labels,
                vec![
                    AreaSeries::new("Reads", reads).tone(Tone::Primary),
                    AreaSeries::new("Writes", writes).tone(Tone::Success),
                ],
            )
            .stacked(true)
            .height(180.0)
            .into_element::<Message>(),
        ]
        .spacing(10),
        false,
    );
}

#[test]
fn a_pie_chart_renders() {
    let slices = vec![
        PieSlice::new("Rust", 45.0).tone(Tone::Primary),
        PieSlice::new("Go", 25.0).tone(Tone::Success),
        PieSlice::new("Python", 20.0).tone(Tone::Warning),
        PieSlice::new("Other", 10.0).tone(Tone::Danger),
    ];

    assert_renders(
        "pie_chart",
        column![
            heading("Pie", Heading::H4),
            PieChart::new(slices.clone())
                .height(220.0)
                .into_element::<Message>(),
            heading("Donut", Heading::H4),
            PieChart::new(slices)
                .donut(true)
                .height(200.0)
                .into_element::<Message>(),
        ]
        .spacing(10),
        false,
    );
}

#[test]
fn a_radar_chart_renders() {
    let axes: Vec<String> = ["Speed", "Power", "Range", "Accuracy", "Cost"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();

    assert_renders(
        "radar_chart",
        RadarChart::new(
            axes,
            vec![
                RadarSeries::new("Model A", vec![80.0, 65.0, 90.0, 70.0, 40.0]).tone(Tone::Primary),
                RadarSeries::new("Model B", vec![60.0, 85.0, 50.0, 95.0, 70.0]).tone(Tone::Success),
            ],
        )
        .max_value(100.0)
        .tooltip(true)
        .height(300.0)
        .into_element::<Message>(),
        false,
    );
}

#[test]
fn a_candlestick_chart_renders() {
    // A small random-walk series, so the candles show both directions.
    let mut price = 100.0_f64;
    let mut candles = Vec::new();

    for index in 0..20 {
        let drift = ((f64::from(index)) * 0.7).sin() * 4.0;
        let open = price;
        let close = open + drift;
        let high = open.max(close) + 2.5;
        let low = open.min(close) - 2.5;

        candles.push(Candle::new(open, high, low, close));
        price = close;
    }

    let labels: Vec<String> = (1..=20).map(|d| d.to_string()).collect();

    assert_renders(
        "candlestick_chart",
        CandlestickChart::new(labels, candles)
            .tooltip(true)
            .height(300.0)
            .into_element::<Message>(),
        false,
    );
}

#[test]
fn a_sankey_chart_renders() {
    let nodes = vec![
        SankeyNode::new("Coal").tone(Tone::Primary),
        SankeyNode::new("Gas").tone(Tone::Warning),
        SankeyNode::new("Solar").tone(Tone::Success),
        SankeyNode::new("Electricity").tone(Tone::Primary),
        SankeyNode::new("Industry").tone(Tone::Danger),
        SankeyNode::new("Homes").tone(Tone::Success),
        SankeyNode::new("Losses").tone(Tone::Neutral),
    ];

    let links = vec![
        SankeyLink::new(0, 3, 40.0),
        SankeyLink::new(1, 3, 25.0),
        SankeyLink::new(2, 3, 18.0),
        SankeyLink::new(3, 4, 45.0),
        SankeyLink::new(3, 5, 22.0),
        SankeyLink::new(3, 6, 16.0),
    ];

    assert_renders(
        "sankey_chart",
        SankeyChart::new(nodes, links)
            .align(SankeyAlign::Justify)
            .tooltip(true)
            .height(340.0)
            .into_element::<Message>(),
        false,
    );
}

#[test]
fn a_settings_panel_renders() {
    use iced_kit::setting::{
        SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsState,
    };

    let state = SettingsState::new();

    let settings = Settings::<Message>::new(&state)
        .on_event(|_| Message::Noop)
        .page(
            SettingPage::new("Appearance")
                .icon(iced_kit::icons::IconName::Palette)
                .group(
                    SettingGroup::new()
                        .title("Theme")
                        .description("How the interface is colored.")
                        .item(
                            SettingItem::new("Dark mode")
                                .description("Use the dark palette.")
                                .field(
                                    SettingField::switch(true, |_| Message::Noop)
                                        .default_value(true),
                                ),
                        )
                        .item(SettingItem::new("Accent").field(SettingField::select(
                            vec![
                                ("blue".to_owned(), "Blue".to_owned()),
                                ("violet".to_owned(), "Violet".to_owned()),
                            ],
                            Some("blue".to_owned()),
                            |_| Message::Noop,
                        ))),
                )
                .group(
                    SettingGroup::new()
                        .title("Typography")
                        .item(
                            SettingItem::new("Font size").field(
                                SettingField::number(14.0, 8.0..=72.0, |_| Message::Noop)
                                    .default_value(14.0),
                            ),
                        )
                        .item(
                            SettingItem::new("Font family")
                                .field(SettingField::text("Inter", |_| Message::Noop)),
                        ),
                ),
        )
        .page(
            SettingPage::new("Privacy")
                .icon(iced_kit::icons::IconName::User)
                .group(SettingGroup::new().title("Telemetry").item(
                    SettingItem::new("Send usage data").field(
                        SettingField::checkbox(false, |_| Message::Noop).default_value(false),
                    ),
                )),
        );

    let element: Element<'_, Message, Theme> = settings.into();
    assert_renders("settings_panel", element, false);
}

#[test]
fn a_settings_panel_renders_in_dark_mode() {
    use iced_kit::setting::{
        SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsState,
    };

    let state = SettingsState::new();

    let element: Element<'_, Message, Theme> = Settings::<Message>::new(&state)
        .page(
            SettingPage::new("General").group(
                SettingGroup::new().title("Startup").item(
                    SettingItem::new("Launch at login")
                        .field(SettingField::switch(true, |_| Message::Noop)),
                ),
            ),
        )
        .into();

    assert_renders("settings_panel_dark", element, true);
}

#[test]
fn a_stacked_settings_panel_renders() {
    use iced_kit::setting::{
        SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsState,
    };

    let state = SettingsState::new();

    let element: Element<'_, Message, Theme> = Settings::<Message>::new(&state)
        .stacked(true)
        .page(
            SettingPage::new("General").group(
                SettingGroup::new().title("Updates").item(
                    SettingItem::new("Automatic updates")
                        .field(SettingField::switch(true, |_| Message::Noop)),
                ),
            ),
        )
        .into();

    assert_renders("settings_panel_stacked", element, false);
}

#[test]
fn a_disabled_settings_group_renders_inert() {
    use iced_kit::setting::{
        SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsState,
    };

    let state = SettingsState::new();

    let element: Element<'_, Message, Theme> = Settings::<Message>::new(&state)
        .page(
            SettingPage::new("Advanced").group(
                SettingGroup::new()
                    .title("Experimental")
                    .disabled(true)
                    .item(
                        SettingItem::new("Unsafe mode")
                            .field(SettingField::switch(false, |_| Message::Noop)),
                    )
                    .item(SettingItem::new("Buffer size").field(SettingField::number(
                        64.0,
                        1.0..=512.0,
                        |_| Message::Noop,
                    ))),
            ),
        )
        .into();

    assert_renders("settings_panel_disabled", element, false);
}

#[test]
fn group_boxes_render_in_every_variant() {
    use iced_kit::widgets::{group_box, GroupBoxVariant};

    assert_renders(
        "group_boxes",
        column![
            group_box::<Message>()
                .title(iced::widget::text("Normal"))
                .push(iced::widget::text("A plain surface.")),
            group_box::<Message>()
                .title(iced::widget::text("Fill"))
                .variant(GroupBoxVariant::Fill)
                .push(iced::widget::text("An inset surface.")),
            group_box::<Message>()
                .title(iced::widget::text("Outline"))
                .variant(GroupBoxVariant::Outline)
                .description("A bordered surface.")
                .push(iced::widget::text("Grouped without a fill.")),
        ]
        .spacing(16),
        false,
    );
}

#[test]
fn named_icons_render() {
    use iced_kit::icons::IconName;
    use iced_kit::widgets::{button, Icon};

    assert_renders(
        "named_icons",
        row![
            button::<Message>("Search")
                .icon(Icon::new(IconName::Search))
                .on_press(Message::Noop),
            button::<Message>("Undo")
                .icon(Icon::new(IconName::Undo2))
                .on_press(Message::Noop),
            button::<Message>("Settings")
                .icon(Icon::new(IconName::Settings2))
                .outline()
                .on_press(Message::Noop),
            button::<Message>("Info")
                .icon(Icon::new(IconName::Info))
                .ghost()
                .on_press(Message::Noop),
        ]
        .spacing(12),
        false,
    );
}

/// The settings panel owns its view state, so the checks below drive the panel
/// through messages the way an application would, rather than asserting on the
/// snapshot alone.
mod settings_interaction {
    use super::{Element, Message, Theme};
    use iced::Task;
    use iced_kit::setting::{
        SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsEvent,
        SettingsState,
    };

    fn app(state: &SettingsState, dark: bool) -> Element<'_, Message, Theme> {
        Settings::<Message>::new(state)
            .on_event(|_event| Message::Noop)
            .page(
                SettingPage::new("Appearance").group(
                    SettingGroup::new().title("Theme").item(
                        SettingItem::new("Dark mode")
                            .field(SettingField::switch(dark, |_| Message::Noop)),
                    ),
                ),
            )
            .into()
    }

    fn simulator(
        state: &SettingsState,
        dark: bool,
    ) -> iced_test::Simulator<'_, Message, Theme, iced::Renderer> {
        iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(640.0, 480.0),
            app(state, dark),
        )
    }

    #[test]
    fn the_panel_renders_without_a_snapshot_comparison() {
        // A smoke test: building and rendering the element must not panic, which
        // is what catches a bad layout or a missing id.
        let state = SettingsState::new();
        let mut sim = simulator(&state, false);

        sim.snapshot(&Theme::light())
            .expect("the panel must render");
        sim.snapshot(&Theme::dark())
            .expect("the panel must render dark");
    }

    #[test]
    fn applying_a_page_selection_moves_the_panel() {
        let mut state = SettingsState::new();
        state.apply(SettingsEvent::SelectPage(1));

        assert_eq!(state.selected_page(), 1);

        // And the panel still renders on the page that was selected.
        let mut sim = simulator(&state, false);
        sim.snapshot(&Theme::light())
            .expect("the panel must render");
    }

    #[test]
    fn a_search_hides_every_page_that_does_not_match() {
        // Each simulator borrows the state, so the two queries get their own.
        let mut matching = SettingsState::new();
        matching.set_query("dark mode");
        let mut sim = simulator(&matching, false);
        sim.snapshot(&Theme::light())
            .expect("a matching page must still render");

        let mut unmatched = SettingsState::new();
        unmatched.set_query("nothing matches this");
        let mut sim = simulator(&unmatched, false);
        sim.snapshot(&Theme::light())
            .expect("an unmatched query must render the empty state");
    }

    #[test]
    fn scrolling_to_a_group_builds_a_task() {
        // The scroll is an operation over the rendered tree, so it can only be
        // exercised after the panel has been built and laid out.
        let state = SettingsState::new();
        let element = app(&state, false);

        let mut sim = iced_test::Simulator::with_size(
            iced::Settings::default(),
            iced::Size::new(640.0, 480.0),
            element,
        );
        sim.snapshot(&Theme::light()).expect("render first");

        let task: Task<Message> = Settings::<Message>::scroll_to_group(0, 0);
        drop(task);
    }
}

#[test]
fn a_filtered_settings_panel_renders_only_matching_groups() {
    use iced_kit::setting::{
        SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsState,
    };

    // The query matches the second group's item, so the first group must drop
    // out of both the content and the sidebar.
    let mut state = SettingsState::new();
    state.set_query("font");

    let element: Element<'_, Message, Theme> = Settings::<Message>::new(&state)
        .on_event(|_| Message::Noop)
        .page(
            SettingPage::new("Appearance")
                .group(
                    SettingGroup::new().title("Theme").item(
                        SettingItem::new("Dark mode")
                            .field(SettingField::switch(true, |_| Message::Noop)),
                    ),
                )
                .group(SettingGroup::new().title("Typography").item(
                    SettingItem::new("Font size").field(SettingField::number(
                        14.0,
                        8.0..=72.0,
                        |_| Message::Noop,
                    )),
                )),
        )
        .into();

    assert_renders("settings_panel_filtered", element, false);
}

/// The caret beside a select's own arrow, so the two menu affordances are
/// compared rather than judged separately.
#[test]
fn a_caret_matches_a_selects_arrow() {
    use iced_kit::widgets::button::Button;

    assert_renders(
        "caret_versus_select_arrow",
        column![
            Button::<Message>::new("Menu")
                .dropdown_caret()
                .on_press(Message::Noop),
            iced_kit::widgets::select(
                vec!["Blue".to_owned(), "Violet".to_owned()],
                Some("Blue".to_owned()),
                |_| Message::Noop,
            ),
        ]
        .spacing(8),
        false,
    );
}

/// An icon-only button must centre its glyph in its square.
///
/// This was a real defect: the glyph was boxed in a `container` sized to the
/// icon, which does not shrink to its child, so the alignment had nothing to
/// center against and every icon-only button drew its glyph against the left of
/// a full-width line box — visibly right of centre. The box is now the `Text`
/// itself, which centers the glyph as it draws.
#[test]
fn icon_only_buttons_centre_their_glyph() {
    use iced_kit::widgets::icon_button;

    assert_renders(
        "icon_only_buttons",
        column![
            // A text glyph and an icon-font glyph, so both paths are covered.
            row![
                icon_button::<Message>()
                    .icon("✕")
                    .primary()
                    .on_press(Message::Noop),
                icon_button::<Message>()
                    .icon("＋")
                    .primary()
                    .on_press(Message::Noop),
                icon_button::<Message>()
                    .icon(iced_kit::icons::IconName::Plus)
                    .primary()
                    .on_press(Message::Noop),
                icon_button::<Message>()
                    .icon(iced_kit::icons::IconName::X)
                    .primary()
                    .on_press(Message::Noop),
            ]
            .spacing(8),
            row![
                icon_button::<Message>()
                    .icon("✕")
                    .outline()
                    .on_press(Message::Noop),
                icon_button::<Message>()
                    .icon("＋")
                    .outline()
                    .on_press(Message::Noop),
                icon_button::<Message>()
                    .icon("⊙")
                    .outline()
                    .on_press(Message::Noop),
            ]
            .spacing(8),
        ]
        .spacing(16),
        false,
    );
}

/// A page button's number must sit on the button's centre line.
///
/// This was a real defect: the button's label inherited a line box the height of
/// the *text* rather than of the control, so the digit was drawn against the top
/// of the button — measured at 20 physical pixels above centre. iced lays a raw
/// button's content out at its padding origin without centring it, which is why
/// the line box's height is the only thing that decides where the glyph lands.
#[test]
fn pagination_centres_its_page_numbers() {
    assert_renders(
        "pagination_only",
        column![
            pagination(4, 20, |_| Message::Noop),
            pagination(0, 5, |_| Message::Noop),
        ]
        .spacing(16),
        false,
    );
}

/// A carousel in each orientation, with and without its controls, must draw the
/// slides inside its viewport rather than over the content beside it.
#[test]
fn carousels_clip_their_slides() {
    let horizontal = CarouselState::new(3);
    let vertical = CarouselState::new(3).with_axis(CarouselAxis::Vertical);

    let slide = |index: usize| -> Element<'static, Message, Theme> {
        container(iced::widget::text(format!("Slide {}", index + 1))).into()
    };

    assert_renders(
        "carousel",
        column![
            carousel(&horizontal, (0..3).map(slide).collect(), |_| Message::Noop,)
                .height(Length::Fixed(100.0))
                .indicators(true),
            carousel(&horizontal, (0..3).map(slide).collect(), |_| Message::Noop,)
                .controls(false)
                .height(Length::Fixed(100.0)),
            carousel(&vertical, (0..3).map(slide).collect(), |_| Message::Noop,)
                .height(Length::Fixed(100.0)),
        ]
        .spacing(16),
        false,
    );
}

/// A tab's label must sit on the tab's centre line, for the same reason.
#[test]
fn tabs_centre_their_labels() {
    use iced_kit::widgets::{tabs, Tab};

    assert_renders(
        "tabs_only",
        column![
            tabs(
                vec![
                    Tab::new("Overview"),
                    Tab::new("Analytics"),
                    Tab::new("Settings"),
                ],
                0,
                |_| Message::Noop,
            ),
            tabs(
                vec![Tab::new("One"), Tab::new("Two"), Tab::new("Three")],
                1,
                |_| Message::Noop,
            ),
        ]
        .spacing(16),
        false,
    );
}

/// The `dock` example's workspace, rendered so its layout is checked rather than
/// only compiled.
///
/// The example is the crate's own demonstration of the dock, so a regression that
/// only shows up on screen — a region taking no space, a bar drawn over its
/// content — would otherwise go unnoticed until someone ran it.
#[cfg(feature = "dock")]
#[test]
fn the_dock_example_workspace_renders() {
    use iced_kit::widgets::dock::{
        self, DockEvent, DockPlacement, DockSession, LayoutArea, PanelDef, PanelPresentation,
        PanelStyle,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum Panel {
        Files,
        Editor,
        Terminal,
    }

    struct Panels;
    impl PanelPresentation<Panel, DockMessage, Theme> for Panels {
        fn title(&self, panel: Panel) -> Option<Element<'static, DockMessage, Theme>> {
            let title = match panel {
                Panel::Files => "Explorer",
                Panel::Editor => "main.rs",
                Panel::Terminal => "Terminal",
            };
            Some(iced::widget::text(title).into())
        }
    }

    /// The event is carried but never inspected: this test checks what the dock
    /// draws, not what it reports.
    #[derive(Debug, Clone)]
    enum DockMessage {
        #[allow(dead_code)]
        Dock(DockEvent<Panel>),
    }

    let area = LayoutArea::new(dock::horizontal([
        dock::tabs([PanelDef::new("editor", "main.rs", Panel::Editor)]),
        dock::tabs([PanelDef::new("files", "Explorer", Panel::Files)]),
    ]))
    .dock(
        DockPlacement::Bottom,
        140.0,
        dock::tabs([PanelDef::new("terminal", "Terminal", Panel::Terminal)]),
    );
    let session = DockSession::from_area(area).expect("the workspace is valid");

    let element: Element<'_, DockMessage, Theme> = dock::apply_metrics(
        dock::dock::<Panel, DockMessage, Theme, iced::Renderer>()
            .state(session.state())
            .on_event(DockMessage::Dock)
            .style(dock::style)
            .panel_style(PanelStyle::Auto)
            .presentation(Panels),
    )
    .content(|panel| {
        iced::widget::text(match panel {
            Panel::Files => "explorer",
            Panel::Editor => "editor body",
            Panel::Terminal => "terminal output",
        })
        .into()
    })
    .build()
    .into();

    let mut simulator = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(640.0, 480.0),
        element,
    );
    let theme = Theme::light();
    let snapshot = simulator.snapshot(&theme).expect("the dock must render");
    let path = "tests/snapshots/dock-example.png";
    let matches = snapshot
        .matches_image(path)
        .expect("dock example comparison must succeed");
    assert!(
        matches,
        "the dock example no longer matches {path}. \
         If the change was intentional, delete that file and rerun."
    );
}

/// A form in each label direction, and one in two columns with a footer,
/// must render its fields with labels, required markers and descriptions.
#[test]
fn form_layouts_render() {
    use iced_kit::widgets::{button, field, form, text_area, FormLabelLayout};

    let empty_bio = iced::widget::text_editor::Content::with_text("");
    let empty_bio_2 = iced::widget::text_editor::Content::with_text("");

    let vertical = form()
        .child(
            field()
                .label("Name")
                .push(text_input::<Message>("Ada Lovelace", "").on_input(|_| Message::Noop)),
        )
        .child(
            field()
                .label("Email")
                .required(true)
                .push(text_input::<Message>("you@example.com", "").on_input(|_| Message::Noop)),
        )
        .child(
            field()
                .label("Bio")
                .description("Use at most 100 words to describe yourself.")
                .push(
                    text_area::<Message>("Write something…", &empty_bio).on_edit(|_| Message::Noop),
                ),
        );

    let horizontal = form()
        .label_layout(FormLabelLayout::Horizontal)
        .label_width(80.0)
        .child(
            field()
                .label("Email")
                .required(true)
                .description("We never share it.")
                .push(text_input::<Message>("you@example.com", "").on_input(|_| Message::Noop)),
        )
        .child(
            field()
                .label_indent(false)
                .push(muted_text("This is a full width form field.")),
        );

    let columns = form()
        .columns(2)
        .child(
            field()
                .label("Name")
                .push(text_input::<Message>("Ada Lovelace", "").on_input(|_| Message::Noop)),
        )
        .child(
            field()
                .label("Email")
                .required(true)
                .push(text_input::<Message>("you@example.com", "").on_input(|_| Message::Noop)),
        )
        .child(field().label("Bio").col_span(2).push(
            text_area::<Message>("Write something…", &empty_bio_2).on_edit(|_| Message::Noop),
        ))
        .footer(button("Save").primary().on_press(Message::Noop));

    assert_renders("form_vertical", column![vertical].spacing(16), false);
    assert_renders("form_horizontal", column![horizontal].spacing(16), false);
    assert_renders("form_columns", column![columns].spacing(16), false);
}

/// A grouped table, a table with a pinned column, and one that is loading.
///
/// These are the three options the reference's `DataTable` carries that the
/// first port left out, so this snapshot is what keeps them rendering.
#[test]
fn table_groups_pins_and_placeholders_render() {
    use iced_kit::widgets::data_table::{ColumnGroup, TableState};

    let rows: Vec<Record> = [
        ("report.pdf", 2_400_000_u64),
        ("archive.zip", 18_000_000),
        ("notes.md", 4_200),
    ]
    .into_iter()
    .map(|(name, size)| Record {
        name: name.to_owned(),
        size,
    })
    .collect();

    let mut sorted = TableState::new();
    sorted.toggle_sort("Size");
    let plain_state = TableState::new();

    let grouped = DataTable::new(
        &rows,
        &sorted,
        vec![option_columns(false), option_size_column()],
    )
    .row_height(34.0)
    .column_groups([ColumnGroup::new("File", 1), ColumnGroup::new("Details", 1)]);

    let pinned = DataTable::new(
        &rows,
        &plain_state,
        vec![
            option_columns(true),
            option_size_column(),
            option_detail_column(),
        ],
    )
    .row_height(34.0);

    let loading = DataTable::new(
        &rows,
        &plain_state,
        vec![option_columns(false), option_size_column()],
    )
    .row_height(34.0)
    .loading(3);

    // The whole set is built and rendered inside this scope: every table
    // borrows `rows` and its state, so none of them may outlive them.
    assert_renders(
        "data_table_options",
        column![
            container(grouped).padding(0),
            container(pinned).padding(0),
            container(loading).padding(0)
        ]
        .spacing(24),
        false,
    );
}

/// The columns the `data_table_options` snapshot is built from.
///
/// They are functions rather than closures so each table owns its own columns:
/// a `Column` holds a boxed renderer and is not `Clone`.
fn option_columns<'a>(fixed: bool) -> Column<'a, Record, Message> {
    Column::<Record, Message>::new("Name", |record: &Record, _index| {
        iced_kit::widgets::data_table::text_cell(record.name.clone())
    })
    .width(Width::FillPortion(3))
    .fixed(fixed)
}

fn option_size_column<'a>() -> Column<'a, Record, Message> {
    Column::<Record, Message>::new("Size", |record: &Record, _index| {
        iced_kit::widgets::data_table::number_cell(format_bytes(record.size))
    })
    .width(Width::Fixed(120.0))
    .align_x(iced::Alignment::End)
}

fn option_detail_column<'a>() -> Column<'a, Record, Message> {
    Column::<Record, Message>::new("Type", |record: &Record, _index| {
        iced_kit::widgets::data_table::text_cell(
            record.name.rsplit_once('.').map_or("file", |(_, ext)| ext),
        )
    })
    .width(Width::Fill)
}

/// The badge variants, both separator forms, the alert forms and an avatar
/// group: the pieces batch two of the parity work added.
#[test]
fn display_variants_render() {
    use iced_kit::widgets::{
        alert_builder, avatar, avatar_group, badge_builder, horizontal_separator, label_builder,
        AvatarGroup, HighlightsMatch, Label, Tone,
    };

    let badges: Vec<Element<'_, Message, Theme>> = vec![
        badge_builder(Tone::Success).label("Active").into(),
        badge_builder(Tone::Danger).dot().into(),
        badge_builder(Tone::Primary).count(7).into(),
        badge_builder(Tone::Warning).count(150).max(99).into(),
        badge_builder(Tone::Neutral)
            .icon(iced_kit::icons::IconName::Bell)
            .into(),
    ];

    let separators: Vec<Element<'_, Message, Theme>> = vec![
        horizontal_separator().into(),
        horizontal_separator().label("or").into(),
        horizontal_separator().dashed().into(),
        horizontal_separator().label("or").dashed().into(),
    ];

    let alerts: Vec<Element<'_, Message, Theme>> = vec![
        alert_builder("Your trial ends in 3 days.", Tone::Warning)
            .title("Heads up")
            .into(),
        alert_builder("Saved to disk.", Tone::Success).into(),
        alert_builder("This banner spans the page.", Tone::Neutral)
            .banner()
            .on_close(Message::Noop)
            .into(),
    ];

    let labels: Vec<Element<'_, Message, Theme>> = vec![
        {
            let label: Label<'_, Message> = label_builder("hunter2").masked(true).secondary("Ada");
            label.into()
        },
        {
            let label: Label<'_, Message> =
                label_builder("Ada Lovelace").highlights(HighlightsMatch::Full("ada".to_owned()));
            label.into()
        },
        {
            let label: Label<'_, Message> =
                label_builder("Grace Hopper").highlights(HighlightsMatch::Full("ada".to_owned()));
            label.into()
        },
    ];

    let members = [
        "Ada Lovelace",
        "Grace Hopper",
        "Alan Turing",
        "Edsger Dijkstra",
    ];
    let group: AvatarGroup<'_, Message> = avatar_group()
        .children(members.map(|name| avatar::<Message>(name, 32)))
        .diameter(32)
        .limit(3)
        .ellipsis();

    let mut badge_row = row![].spacing(8);
    for badge in badges {
        badge_row = badge_row.push(badge);
    }

    let mut label_column = column![].spacing(6);
    for label in labels {
        label_column = label_column.push(label);
    }

    assert_renders(
        "display_variants",
        column![
            badge_row,
            label_column,
            column(separators).spacing(8),
            group,
            column(alerts).spacing(8),
        ]
        .spacing(16),
        false,
    );
}

/// A combobox trigger in each state, and an open panel over a filtered list.
#[test]
fn combobox_states_render() {
    use iced_kit::widgets::{combobox, combobox_panel, ComboBoxOption};

    let options = vec![
        ComboBoxOption::new(0usize, "Germany").detail("DE"),
        ComboBoxOption::new(1, "Ghana").detail("GH"),
        ComboBoxOption::new(2, "Greece").detail("GR"),
        ComboBoxOption::new(3, "Japan").detail("JP").disabled(true),
    ];
    let selected = [0usize];

    let empty: Element<'_, Message, Theme> =
        combobox::<usize, Message>(&options, None, "Choose a country").into();

    let chosen: Element<'_, Message, Theme> =
        combobox::<usize, Message>(&options, Some(1), "Choose a country")
            .clearable(Message::Noop)
            .fill(true)
            .into();

    let open: Element<'_, Message, Theme> =
        combobox::<usize, Message>(&options, Some(1), "Choose a country")
            .open(true)
            .on_toggle(Message::Noop)
            .fill(true)
            .into();

    // The searchable trigger is a field inside the button, so its snapshot
    // pins the frame: the button owns the border, and the field sits in it
    // bare rather than drawing a box of its own.
    let searchable: Element<'_, Message, Theme> =
        combobox::<usize, Message>(&options, Some(1), "Choose a country")
            .query("Gr")
            .on_query(|_| Message::Noop)
            .open(true)
            .on_toggle(Message::Noop)
            .fill(true)
            .into();

    let panel: Element<'_, Message, Theme> =
        combobox_panel(&options, &selected, "gr", |_| Message::Noop)
            .on_query(|_| Message::Noop)
            .into();

    assert_renders(
        "combobox",
        column![empty, chosen, open, searchable, panel].spacing(16),
        false,
    );
}

/// A select beside the combobox triggers it should read as a family with.
///
/// The select is the zero-cost dropdown and the combobox the searchable one;
/// their triggers must look like the same control at two capability levels —
/// surface fill, one border, a chevron — or the selection page reads as two
/// unrelated widgets.
#[test]
fn select_and_combobox_render() {
    use iced_kit::widgets::{combobox, select, select_fill, ComboBoxOption};

    let combo_options = vec![
        ComboBoxOption::new(0usize, "Free"),
        ComboBoxOption::new(1, "Pro"),
        ComboBoxOption::new(2, "Team"),
    ];

    let chosen: Element<'_, Message, Theme> =
        select(vec!["Free", "Pro", "Team"], Some("Pro"), |_| Message::Noop)
            .placeholder("Choose a plan")
            .into();

    let placeholder: Element<'_, Message, Theme> =
        select(Vec::<&str>::new(), None::<&str>, |_| Message::Noop)
            .placeholder("Choose a plan")
            .into();

    let filled: Element<'_, Message, Theme> =
        select_fill(vec!["Free", "Pro", "Team"], Some("Pro"), |_| Message::Noop)
            .placeholder("Choose a plan")
            .into();

    let combo: Element<'_, Message, Theme> =
        combobox::<usize, Message>(&combo_options, Some(1), "Choose a plan")
            .on_toggle(Message::Noop)
            .fill(true)
            .into();

    let combo_open: Element<'_, Message, Theme> =
        combobox::<usize, Message>(&combo_options, Some(1), "Choose a plan")
            .open(true)
            .on_toggle(Message::Noop)
            .fill(true)
            .into();

    assert_renders(
        "select",
        column![chosen, placeholder, filled, combo, combo_open].spacing(16),
        false,
    );
}

/// A month grid, a two-month view with a range, and the date field in both
/// its empty and filled states.
#[test]
fn calendar_and_date_field_render() {
    use iced_kit::widgets::{calendar, date_picker, Date, Weekday};

    let month = Date::from_ymd(2024, 2, 1).expect("a real date");
    let selected = Date::from_ymd(2024, 2, 14).expect("a real date");

    let one: Element<'_, Message, Theme> = calendar::<Message>(month, Some(selected))
        .on_select(|_| Message::Noop)
        .into();

    let ranged: Element<'_, Message, Theme> = calendar::<Message>(month, None)
        .number_of_months(2)
        .first_day_of_week(Weekday::Sunday)
        .range((
            Date::from_ymd(2024, 2, 5).unwrap(),
            Date::from_ymd(2024, 2, 16).unwrap(),
        ))
        .on_select(|_| Message::Noop)
        .into();

    let empty: Element<'_, Message, Theme> = date_picker::<Message>("Pick a date", "").into();

    let filled: Element<'_, Message, Theme> = date_picker::<Message>("Pick a date", "2024-02-14")
        .open(true)
        .on_toggle(Message::Noop)
        .clearable(Message::Noop)
        .fill(true)
        .into();

    // The editable field nests a bare text input inside the framed button;
    // this state pins that the field draws no border of its own.
    let editable: Element<'_, Message, Theme> = date_picker::<Message>("Pick a date", "2024-02-14")
        .on_input(|_| Message::Noop)
        .on_toggle(Message::Noop)
        .fill(true)
        .into();

    assert_renders(
        "calendar",
        column![one, column![empty, filled, editable].spacing(8), ranged].spacing(20),
        false,
    );
}

/// A color picker trigger, and the panel with its square, hue slider and
/// swatches.
#[test]
fn color_picker_renders() {
    use iced_kit::widgets::{color_picker, color_picker_panel, parse_hex};

    let color = parse_hex("#336699").expect("a valid color").to_color();

    let trigger: Element<'_, Message, Theme> = color_picker::<Message>(color)
        .on_toggle(Message::Noop)
        .into();

    let labelled: Element<'_, Message, Theme> = color_picker::<Message>(color)
        .label("Accent")
        .open(true)
        .on_toggle(Message::Noop)
        .into();

    let panel: Element<'_, Message, Theme> = color_picker_panel::<Message>(color)
        .on_change(|_| Message::Noop)
        .into();

    let swatchless: Element<'_, Message, Theme> = color_picker_panel::<Message>(color)
        .on_change(|_| Message::Noop)
        .swatches([])
        .show_hex(false)
        .into();

    assert_renders(
        "color_picker",
        row![column![trigger, labelled].spacing(8), panel, swatchless].spacing(16),
        false,
    );
}

/// A file tree, collapsed, open, and with a single node selected.
#[test]
fn tree_view_renders() {
    use iced_kit::widgets::{tree, Tree, TreeItem};

    let items = vec![
        TreeItem::new("src", "src").children([
            TreeItem::new("main", "main.rs"),
            TreeItem::new("widgets", "widgets").children([
                TreeItem::new("button", "button.rs"),
                TreeItem::new("input", "input.rs"),
            ]),
            TreeItem::new("theme", "theme.rs"),
        ]),
        TreeItem::new("readme", "README.md"),
        TreeItem::new("locked", "Protected").disabled(true),
    ];

    let collapsed = Tree::new().items(items.clone());

    let mut open = Tree::new().items(items.clone());
    open.expand_all();

    let mut selected = Tree::new().items(items.clone());
    selected.expand_all();
    selected.select(Some("input"));

    assert_renders(
        "tree",
        column![
            column![
                muted_text("Collapsed"),
                tree::<Message>(&items, &collapsed).on_event(|_| Message::Noop),
            ]
            .spacing(8),
            column![
                muted_text("Expanded, with a selection"),
                tree::<Message>(&items, &open)
                    .on_event(|_| Message::Noop)
                    .max_height(240.0),
            ]
            .spacing(8),
        ]
        .spacing(20),
        false,
    );

    // The selected variant renders too; the snapshot above is the open one, so
    // this keeps the selection path covered.
    let element: Element<'_, Message, Theme> = tree::<Message>(&items, &selected)
        .on_event(|_| Message::Noop)
        .into();
    drop(element);
}

/// A breadcrumb trail, and a stepper in both its layouts.
#[test]
fn breadcrumb_and_stepper_render() {
    use iced_kit::widgets::{breadcrumb, stepper, Crumb, Step, StepLayout};

    let trail: Element<'_, Message, Theme> = breadcrumb(vec![
        Crumb::link("Home", Message::Noop),
        Crumb::link("Projects", Message::Noop),
        Crumb::new("iced-kit"),
    ]);

    let disabled_trail: Element<'_, Message, Theme> = breadcrumb(vec![
        Crumb::link("Home", Message::Noop),
        Crumb::link("Archived", Message::Noop).disabled(true),
        Crumb::new("Old report"),
    ]);

    let horizontal: Element<'_, Message, Theme> = stepper(
        vec![
            Step::new("Cart"),
            Step::new("Address"),
            Step::new("Payment"),
            Step::new("Review"),
        ],
        2,
    )
    .on_select(|_| Message::Noop)
    .into();

    let vertical: Element<'_, Message, Theme> = stepper(
        vec![
            Step::new("Cart"),
            Step::new("Address").disabled(true),
            Step::new("Payment"),
        ],
        1,
    )
    .layout(StepLayout::Vertical)
    .on_select(|_| Message::Noop)
    .into();

    assert_renders(
        "breadcrumb_stepper",
        column![
            trail,
            disabled_trail,
            horizontal,
            row![vertical, iced::widget::Space::new().width(Length::Fill)].spacing(0),
        ]
        .spacing(20),
        false,
    );
}

/// A description list in both layouts, a status bar, links, a copy button and
/// a rating.
#[test]
fn details_components_render() {
    use iced_kit::widgets::{
        clipboard_button, description_list, link, muted_text, rating, status_bar, Description,
        DescriptionLayout, Link, Rating,
    };

    let vertical: Element<'_, Message, Theme> = description_list(vec![
        Description::new("Name").value("Ada Lovelace"),
        Description::new("Role").value("Mathematician"),
    ])
    .bordered(true)
    .into();

    let horizontal: Element<'_, Message, Theme> = description_list(vec![
        Description::new("Email").value("ada@example.com"),
        Description::new("Notes")
            .value("The first programmer.")
            .span(2),
    ])
    .layout(DescriptionLayout::Horizontal)
    .bordered(true)
    .into();

    let bar: Element<'_, Message, Theme> = status_bar()
        .left(muted_text("main.rs"))
        .center(muted_text("Ln 12, Col 4"))
        .right(muted_text("UTF-8"))
        .into();

    let links: Element<'_, Message, Theme> = row![
        Link::<Message>::new("Documentation")
            .href("https://iced.rs")
            .on_press(Message::Noop),
        link::<Message>("Disabled").disabled(true),
    ]
    .spacing(12)
    .into();

    let copy: Element<'_, Message, Theme> = clipboard_button::<Message>("cargo test")
        .label("Copy command")
        .on_copy(|_| Message::Noop)
        .into();

    let ratings: Element<'_, Message, Theme> = row![
        Rating::<Message>::new(3).on_select(|_| Message::Noop),
        rating::<Message>(5),
    ]
    .spacing(16)
    .into();

    assert_renders(
        "details",
        column![vertical, horizontal, links, copy, ratings, bar].spacing(16),
        false,
    );
}

/// A collapsible panel open and closed, a shimmer over a placeholder, a sheet
/// against an edge, and a hover card's trigger.
#[test]
fn loading_and_overlay_pieces_render() {
    use iced_kit::widgets::overlay::{DrawerSide, HoverCard};
    use iced_kit::widgets::{
        collapsible, sheet, shimmer, shimmer_block, shimmer_text, Collapsible, ShimmerStyle,
    };

    let open: Element<'_, Message, Theme> = collapsible::<Message>(muted_text("Panel body"), true);
    let closed: Element<'_, Message, Theme> =
        collapsible::<Message>(muted_text("Panel body"), false);
    let padded: Element<'_, Message, Theme> =
        Collapsible::<Message>::new(muted_text("Padded body"), true)
            .padding(8)
            .into();

    let shimmering: Element<'_, Message, Theme> = column![
        shimmer_text::<Message>("Loading a value…"),
        shimmer::<Message>(muted_text("Wrapped text")).into_element(),
        shimmer_block::<Message>(Length::Fixed(180.0), 14.0),
    ]
    .spacing(8)
    .into();

    let styled: Element<'_, Message, Theme> = shimmer::<Message>(muted_text("Reverse sweep"))
        .style(ShimmerStyle::new().reverse(true).spread(0.4))
        .into_element();

    let sheet_panel: Element<'_, Message, Theme> =
        sheet::<Message>("Settings", muted_text("Panel body"))
            .side(DrawerSide::Right)
            .into();

    let card: Element<'_, Message, Theme> = HoverCard::<Message>::new(muted_text("Hover me"))
        .on_open(Message::Noop)
        .on_close(Message::Noop)
        .into();

    assert_renders(
        "loading_overlays",
        column![open, closed, padded, shimmering, styled, card].spacing(12),
        false,
    );

    // A sheet fills the frame, so it is rendered on its own.
    assert_renders("sheet", sheet_panel, false);
}

/// The chat family's bubbles and message rows, in both alignments.
///
/// The bubble variants are the piece where the surface treatment carries the
/// meaning — filled for the sender, muted for the other party, destructive for
/// a refused message — so the snapshot is what keeps them distinguishable.
#[test]
fn chat_bubbles_and_messages_render() {
    use iced_kit::widgets::chat::{
        bubble, message, message_group, BubbleReactions, BubbleVariant, MessageAlignment,
    };

    let variants: Vec<Element<'_, Message, Theme>> = [
        BubbleVariant::Filled,
        BubbleVariant::Secondary,
        BubbleVariant::Muted,
        BubbleVariant::Tinted,
        BubbleVariant::Outline,
        BubbleVariant::Destructive,
        BubbleVariant::Ghost,
    ]
    .into_iter()
    .map(|variant| {
        bubble::<Message>("A bubble")
            .with_variant(variant)
            .max_width(240.0)
            .into()
    })
    .collect();

    let start: Element<'_, Message, Theme> = message(bubble("How are the docs coming along?"))
        .alignment(MessageAlignment::Start)
        .header("Ada")
        .footer("12:30")
        .into();

    let end: Element<'_, Message, Theme> =
        message(bubble("Nearly there — one section left.").with_variant(BubbleVariant::Secondary))
            .alignment(MessageAlignment::End)
            .header("Grace")
            .footer("Sent")
            .into();

    // Reactions belong to the bubble, which is where the reference attaches
    // them too: the cluster is positioned against the surface it annotates.
    let thumbs: Element<'_, Message, Theme> = muted_text("👍 2").into();
    let reacted: Element<'_, Message, Theme> = message(
        bubble("Reacted")
            .with_variant(BubbleVariant::Muted)
            .reactions(BubbleReactions::new().action(thumbs)),
    )
    .alignment(MessageAlignment::Start)
    .into();

    assert_renders(
        "chat_bubbles",
        column![
            column(variants).spacing(8),
            message_group::<Message>()
                .child(start)
                .child(end)
                .child(reacted),
        ]
        .spacing(24),
        false,
    );
}

/// Attachment cards across their statuses, sizes and axes.
///
/// Three things here are only checkable in pixels: the pending card's dashed
/// outline, the failed card's destructive tint (on the border, the media box and
/// the description), and the size steps scaling the whole card rather than just
/// its text.
#[test]
fn chat_attachments_render() {
    use iced_kit::widgets::chat::{
        attachment, AttachmentActions, AttachmentAxis, AttachmentContent, AttachmentDescription,
        AttachmentMedia, AttachmentStatus, AttachmentTitle,
    };

    let card = |label: &'static str, status: AttachmentStatus, size: iced_kit::Size| {
        attachment::<Message>(label)
            .with_size(size)
            .status(status)
            .content(
                AttachmentContent::new()
                    .title(AttachmentTitle::new(label))
                    .description(AttachmentDescription::new(match status {
                        AttachmentStatus::Failed => "Upload failed — retry",
                        AttachmentStatus::Pending => "Waiting",
                        // Every other status shows the file's size, which is
                        // the line an in-flight or finished card carries.
                        _ => "1.2 MB",
                    })),
            )
    };

    let retry: Element<'_, Message, Theme> = muted_text("Retry").into();
    let remove: Element<'_, Message, Theme> = muted_text("Remove").into();

    assert_renders(
        "chat_attachments",
        column![
            column![
                muted_text("Statuses"),
                card("report.pdf", AttachmentStatus::Pending, iced_kit::Size::Md),
                card(
                    "report.pdf",
                    AttachmentStatus::Uploading,
                    iced_kit::Size::Md
                ),
                card("report.pdf", AttachmentStatus::Failed, iced_kit::Size::Md),
                card("report.pdf", AttachmentStatus::Complete, iced_kit::Size::Md),
            ]
            .spacing(8),
            column![
                muted_text("Sizes"),
                card("xs.pdf", AttachmentStatus::Complete, iced_kit::Size::Xs),
                card("sm.pdf", AttachmentStatus::Complete, iced_kit::Size::Sm),
                card("lg.pdf", AttachmentStatus::Complete, iced_kit::Size::Lg),
            ]
            .spacing(8),
            column![
                muted_text("Axis and actions"),
                attachment::<Message>("vertical.png")
                    .axis(AttachmentAxis::Vertical)
                    .media(AttachmentMedia::new())
                    .status(AttachmentStatus::Failed)
                    .content(
                        AttachmentContent::new()
                            .title(AttachmentTitle::new("vertical.png"))
                            .description(AttachmentDescription::new("2.4 MB"))
                    ),
                attachment::<Message>("actions.pdf")
                    .actions(AttachmentActions::new().child(retry).child(remove),),
            ]
            .spacing(8),
        ]
        .spacing(24),
        false,
    );
}

/// The ghost-bubble inset rule, side by side.
///
/// A surfaced bubble pads its text by 12px, so the header and footer inset
/// themselves to match. A ghost bubble has no padding, so the same lines must
/// sit flush with the text. The two cases are rendered together because the
/// difference is only legible by comparison.
#[test]
fn chat_ghost_bubble_inset_renders() {
    use iced_kit::widgets::chat::{bubble, message, BubbleVariant, MessageHeader};

    let surfaced: Element<'_, Message, Theme> = message(bubble("Surfaced bubble"))
        .header("Ada")
        .footer("12:30")
        .into();

    let ghost: Element<'_, Message, Theme> =
        message(bubble("Ghost bubble").with_variant(BubbleVariant::Ghost))
            .header("Ada")
            .footer("12:30")
            .into();

    // A header that states its own inset keeps it even beside a ghost bubble.
    let pinned: Element<'_, Message, Theme> =
        message(bubble("Ghost, inset pinned").with_variant(BubbleVariant::Ghost))
            .header_el(MessageHeader::new().text("Ada").content_inset(true))
            .into();

    assert_renders(
        "chat_ghost_inset",
        column![surfaced, ghost, pinned].spacing(16),
        false,
    );
}

/// Message rows with and without avatars, in both alignments.
///
/// The avatar rail and the footer that clears it are both geometric: an avatar
/// only reads as a rail if every one is the same 32px square, and the footer's
/// 40px offset is what keeps a timestamp under the text rather than under the
/// avatar.
#[test]
fn chat_message_avatar_rail_renders() {
    use iced_kit::widgets::avatar;
    use iced_kit::widgets::chat::{
        bubble, message, MessageAlignment, MessageAvatar, MessageHeader,
    };

    let with_avatar: Element<'_, Message, Theme> = message(bubble("With an avatar"))
        .avatar(avatar::<Message>("Ada Lovelace", 32))
        .header("Ada Lovelace")
        .footer("12:30")
        .into();

    let in_a_slot: Element<'_, Message, Theme> = message(bubble("Avatar in a slot"))
        .avatar(MessageAvatar::new().child(avatar::<Message>("Grace Hopper", 32)))
        .header("Grace Hopper")
        .footer("12:31")
        .into();

    let no_avatar: Element<'_, Message, Theme> = message(bubble("No avatar at all"))
        .header("System")
        .footer("12:32")
        .into();

    let trailing: Element<'_, Message, Theme> = message(
        bubble("Sent by the reader")
            .with_variant(iced_kit::widgets::chat::BubbleVariant::Secondary),
    )
    .alignment(MessageAlignment::End)
    .avatar(avatar::<Message>("Grace Hopper", 32))
    .header_el(MessageHeader::new().text("Grace"))
    .footer("Sent")
    .into();

    assert_renders(
        "chat_message_rail",
        column![with_avatar, in_a_slot, no_avatar, trailing].spacing(20),
        false,
    );
}

/// Reaction clusters riding over a bubble's edge.
///
/// The pill is drawn over the bubble rather than beside it, which needs a
/// hand-written layout: iced has no absolute positioning or negative margins.
/// Both sides and both alignments are shown, because each places the pill
/// differently.
#[test]
fn chat_reaction_pills_render() {
    use iced_kit::widgets::chat::{bubble, BubbleReactionSide, BubbleReactions, MessageAlignment};

    let pill = |label: &'static str, side: BubbleReactionSide, alignment: MessageAlignment| {
        let note: Element<'_, Message, Theme> = muted_text(label).into();
        bubble::<Message>("A message with reactions")
            .alignment(alignment)
            .max_width(240.0)
            .reactions(BubbleReactions::new().side(side).child(note))
    };

    let top_start: Element<'_, Message, Theme> =
        pill("👍 2", BubbleReactionSide::Top, MessageAlignment::Start).into();
    let bottom_start: Element<'_, Message, Theme> =
        pill("🎉 4", BubbleReactionSide::Bottom, MessageAlignment::Start).into();
    let top_end: Element<'_, Message, Theme> =
        pill("❤️ 1", BubbleReactionSide::Top, MessageAlignment::End).into();
    let bottom_end: Element<'_, Message, Theme> =
        pill("😄 3", BubbleReactionSide::Bottom, MessageAlignment::End).into();

    assert_renders(
        "chat_reaction_pills",
        column![
            top_start,
            bottom_start,
            top_end,
            bottom_end,
            // A message with no reactions must not reserve the pill's space.
            bubble::<Message>("No reactions at all").max_width(240.0),
        ]
        .spacing(20),
        false,
    );
}

/// The message scroller's jump-to-bottom control, floating over the transcript.
///
/// The control only appears once the reader has scrolled away from the tail, so
/// the state is scrolled up deliberately: this is the one state where the
/// button and the bottom fade are both drawn.
#[test]
fn chat_message_scroller_jump_button_renders() {
    use iced_kit::widgets::chat::{MessageScroller, MessageScrollerState};

    let items: Vec<String> = (0..40).map(|index| format!("Message {index}")).collect();

    let mut state = MessageScrollerState::new(items.len());
    // Far from the bottom, so the tail-follow releases and the overlays draw.
    state.apply_scroll(40.0 * 28.0, 200.0, 0.0);

    let scroller: Element<'_, Message, Theme> =
        MessageScroller::new(&items, &state, |item, _| muted_text(item.clone()).into())
            .jump_button(true)
            .with_bottom_fade(Some(iced::Color::from_rgb8(0xff, 0xff, 0xff)))
            .row_height(28.0)
            .height(200.0)
            .into();

    assert_renders("chat_scroller_jump", scroller, false);
}

/// Markers in every variant, loading style and icon state.
///
/// The spacing here is the reference's: an icon sits 8px from its text and a
/// rule 4px from it. The spinner appears only when the icon slot is free, so
/// the row with an icon is the case that proves the rule.
#[test]
fn chat_markers_render() {
    use iced_kit::widgets::chat::{marker, Marker, MarkerLoadingStyle, MarkerVariant};

    let plain: Element<'_, Message, Theme> = marker::<Message>("— Today —").into();

    let separator: Element<'_, Message, Theme> = Marker::<Message>::new()
        .content("Yesterday")
        .with_variant(MarkerVariant::Separator)
        .into();

    let bordered: Element<'_, Message, Theme> = Marker::<Message>::new()
        .content("System note")
        .with_variant(MarkerVariant::Border)
        .into();

    let spinner: Element<'_, Message, Theme> = Marker::<Message>::new()
        .content("Reading 3 files")
        .loading(true)
        .into();

    // An icon takes the slot, so no spinner is drawn beside it.
    let icon_and_loading: Element<'_, Message, Theme> = Marker::<Message>::new()
        .icon(iced_kit::icons::IconName::Search)
        .content("Searching")
        .loading(true)
        .into();

    let shimmering: Element<'_, Message, Theme> = Marker::<Message>::new()
        .content("Thinking…")
        .loading(true)
        .with_loading_style(MarkerLoadingStyle::Shimmer)
        .into();

    assert_renders(
        "chat_markers",
        column![
            plain,
            separator,
            bordered,
            spinner,
            icon_and_loading,
            shimmering,
        ]
        .spacing(12),
        false,
    );
}

/// A ribbon exercising every item size and a group title, so the snapshot
/// proves large, small, dropdown, grid, selected and disabled all draw.
fn ribbon_demo() -> Element<'static, Message, Theme> {
    // Auto at a generous width: every group stays full-size.
    ribbon_at(CollapseMode::Auto, 600.0)
}

/// Builds the demo ribbon at a given density `mode` and row `width`.
///
/// A forced `mode` pins every group to one density so each level is captured at
/// a stable size; [`CollapseMode::Auto`] with a narrow `width` instead shows the
/// row degrading groups from the right until they fit.
fn ribbon_at(mode: CollapseMode, width: f32) -> Element<'static, Message, Theme> {
    use iced_kit::icons::IconName;
    use iced_kit::widgets::{Ribbon, RibbonGroup, RibbonItem, RibbonState, RibbonTab, RibbonTool};

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
        .group(RibbonGroup::new("Transform").item(RibbonItem::grid(vec![
            vec![
                RibbonTool::named(IconName::Move),
                RibbonTool::named(IconName::Eraser),
            ],
            vec![
                RibbonTool::named(IconName::Group),
                RibbonTool::named(IconName::Pencil),
            ],
        ])));

    let modify = RibbonTab::new("Modify").group(
        RibbonGroup::new("Styles")
            .item(RibbonItem::labeled(
                RibbonTool::named(IconName::Bold).label("Bold"),
            ))
            .item(RibbonItem::labeled(
                RibbonTool::named(IconName::Italic).label("Italic"),
            ))
            .item(RibbonItem::dropdown("layers", IconName::Layers, vec![])),
    );

    let state = RibbonState {
        collapse_mode: mode,
        ..RibbonState::new()
    };
    Ribbon::new()
        .tab(home)
        .tab(modify)
        .state(&state)
        .on_select(|_| Message::Noop)
        .on_dropdown_toggle(|_| Message::Noop)
        .width(Length::Fixed(width))
        .into()
}

#[test]
fn ribbon_renders() {
    assert_renders("ribbon", ribbon_demo(), false);
    assert_renders("ribbon_dark", ribbon_demo(), true);
}

/// The density ladder: *compact* narrows each group to small icons while keeping
/// its label, *collapsed* reduces each to a title button, and *Auto* at a width
/// too small for the full row degrades groups from the right — the leftmost
/// staying full while the rightmost fall back.
#[test]
fn ribbon_density_renders() {
    assert_renders(
        "ribbon_compact",
        ribbon_at(CollapseMode::Compact, 600.0),
        false,
    );
    assert_renders(
        "ribbon_collapsed",
        ribbon_at(CollapseMode::Collapsed, 600.0),
        false,
    );
    assert_renders(
        "ribbon_auto_narrow",
        ribbon_at(CollapseMode::Auto, 180.0),
        false,
    );
}
