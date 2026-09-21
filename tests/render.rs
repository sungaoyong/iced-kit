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

use iced::widget::{column, container};
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
    accordion, alert, avatar, avatar_with_name, badge, code, divider, empty_state, heading, kbd,
    list, muted_text, number_input, otp_input, pagination, paragraph, progress, ring_progress,
    shortcut, skeleton, skeleton_list_item, spinner_styled, tag, text_input, tooltip,
    AccordionSection, AvatarLabel, Drawer, DrawerSide, Dropdown, Heading, ListItem, MenuItem,
    Modal, SkeletonShape, SpinnerStyle, TitleBar, Tone, VirtualList, VirtualListState,
    WindowControl,
};
use iced_kit::Theme;

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
                .icon("◆")
                .subtitle("untitled")
                .control(WindowControl::Minimize, Message::Noop)
                .control(WindowControl::Maximize, Message::Noop)
                .control(WindowControl::Close, Message::Close),
            TitleBar::new("Primary tone")
                .tone(Tone::Primary)
                .control(WindowControl::Close, Message::Close),
            TitleBar::new("Warning tone")
                .tone(Tone::Warning)
                .icon("⚠")
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
            .icon("◆")
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
