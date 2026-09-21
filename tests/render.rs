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
    accordion, addon, alert, avatar, avatar_with_name, badge, code, divider, empty_state,
    group_button, heading, input_group, kbd, list, muted_text, number_input, otp_input, pagination,
    paragraph, password, progress, ring_progress, shortcut, skeleton, skeleton_list_item,
    spinner_styled, tag, text_input, tooltip, AccordionSection, AddonAlignment, AvatarLabel,
    Drawer, DrawerSide, Dropdown, Heading, ListItem, MenuItem, Modal, SkeletonShape, SpinnerStyle,
    TitleBar, Tone, VirtualList, VirtualListState, WindowControl,
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
fn assert_renders<'a>(name: &str, content: impl Into<Element<'a, Message, Theme>>, dark: bool) {
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
        .cancel("Cancel", Message::Close)
        .destructive("Delete", Message::Noop),
    );

    assert_renders("modal", overlay::layer(content, open), false);
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
                        .item(
                            SettingItem::new("Accent")
                                .field(SettingField::select(
                                    vec![
                                        ("blue".to_owned(), "Blue".to_owned()),
                                        ("violet".to_owned(), "Violet".to_owned()),
                                    ],
                                    Some("blue".to_owned()),
                                    |_| Message::Noop,
                                )),
                        ),
                )
                .group(
                    SettingGroup::new()
                        .title("Typography")
                        .item(
                            SettingItem::new("Font size")
                                .field(
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
                SettingGroup::new()
                    .title("Startup")
                    .item(SettingItem::new("Launch at login").field(SettingField::switch(
                        true,
                        |_| Message::Noop,
                    ))),
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
                SettingGroup::new()
                    .title("Updates")
                    .item(SettingItem::new("Automatic updates").field(SettingField::switch(
                        true,
                        |_| Message::Noop,
                    ))),
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
                    .item(SettingItem::new("Unsafe mode").field(SettingField::switch(
                        false,
                        |_| Message::Noop,
                    )))
                    .item(
                        SettingItem::new("Buffer size")
                            .field(SettingField::number(64.0, 1.0..=512.0, |_| Message::Noop)),
                    ),
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
        SettingField, SettingGroup, SettingItem, SettingPage, Settings, SettingsEvent, SettingsState,
    };

    fn app(state: &SettingsState, dark: bool) -> Element<'_, Message, Theme> {
        Settings::<Message>::new(state)
            .on_event(|_event| Message::Noop)
            .page(
                SettingPage::new("Appearance").group(
                    SettingGroup::new()
                        .title("Theme")
                        .item(SettingItem::new("Dark mode").field(SettingField::switch(
                            dark,
                            |_| Message::Noop,
                        ))),
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

        sim.snapshot(&Theme::light()).expect("the panel must render");
        sim.snapshot(&Theme::dark()).expect("the panel must render dark");
    }

    #[test]
    fn applying_a_page_selection_moves_the_panel() {
        let mut state = SettingsState::new();
        state.apply(SettingsEvent::SelectPage(1));

        assert_eq!(state.selected_page(), 1);

        // And the panel still renders on the page that was selected.
        let mut sim = simulator(&state, false);
        sim.snapshot(&Theme::light()).expect("the panel must render");
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
                    SettingGroup::new()
                        .title("Theme")
                        .item(SettingItem::new("Dark mode").field(SettingField::switch(
                            true,
                            |_| Message::Noop,
                        ))),
                )
                .group(
                    SettingGroup::new()
                        .title("Typography")
                        .item(
                            SettingItem::new("Font size")
                                .field(SettingField::number(14.0, 8.0..=72.0, |_| Message::Noop)),
                        ),
                ),
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
                icon_button::<Message>().icon("✕").primary().on_press(Message::Noop),
                icon_button::<Message>().icon("＋").primary().on_press(Message::Noop),
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
                icon_button::<Message>().icon("✕").outline().on_press(Message::Noop),
                icon_button::<Message>().icon("＋").outline().on_press(Message::Noop),
                icon_button::<Message>().icon("⊙").outline().on_press(Message::Noop),
            ]
            .spacing(8),
        ]
        .spacing(16),
        false,
    );
}

/// The dropdown caret at every size, beside the label it has to line up with.
///
/// A caret is the one piece of a button that is not text, so it is the piece
/// most likely to drift: an earlier version drew a `▾` on the text baseline,
/// which read small and rode high. This pins the sizes and the alignment.
#[test]
fn dropdown_carets_render_at_every_size() {
    use iced_kit::widgets::button::Button;

    assert_renders(
        "dropdown_carets",
        column![
            Button::<Message>::new("Menu")
                .dropdown_caret()
                .size(Size::Xs)
                .on_press(Message::Noop),
            Button::<Message>::new("Menu")
                .dropdown_caret()
                .size(Size::Sm)
                .on_press(Message::Noop),
            Button::<Message>::new("Menu")
                .dropdown_caret()
                .size(Size::Md)
                .on_press(Message::Noop),
            Button::<Message>::new("Menu")
                .dropdown_caret()
                .size(Size::Lg)
                .on_press(Message::Noop),
        ]
        .spacing(8),
        false,
    );
}
