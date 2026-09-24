//! Popup menus: the component behind a dock panel's `...` button, a
//! dropdown trigger, or any other anchor that opens a list of commands.
//!
//! The feature set follows `gpui-kit`'s popup menu — nested submenus,
//! checkable and disabled entries, separators that collapse at the edges,
//! keyboard navigation and the three ways a menu goes away (a choice is made,
//! a press lands outside, `Escape`) — ported onto iced's overlay model.
//!
//! # The layering model
//!
//! A menu is an [`overlay::Element`], which a widget hosts from its own
//! `Widget::overlay` — the way the dock's panel menu does. The open state is
//! an [`OpenFlag`], a shared flag the host keeps in its tree state and the
//! menu writes when it dismisses itself, because the overlay borrows the
//! host's state for as long as it is open and cannot hand a message back
//! mid-frame. A submenu is a nested overlay: the runtime updates the deepest
//! menu first and only lets a parent see an event the child ignored, which is
//! what makes `Escape` close one level at a time and a click inside the parent
//! fold the submenu without folding the parent.
//!
//! # What the host owns
//!
//! The host owns the [`OpenFlag`] and a [`State`] per menu. When the flag
//! reads closed, the host resets its state before deciding not to build the
//! overlay, so a reopened menu never inherits a selection or an open submenu
//! from the previous session:
//!
//! ```
//! # use iced_kit::widgets::overlay::menu::{self, Item, OpenFlag};
//! # #[derive(Clone, Debug)] enum Message { Copy, Delete }
//! # let mut state = menu::State::new();
//! # let open = OpenFlag::closed();
//! # open.open();
//! # let items = vec![
//! #     Item::action("Copy", Message::Copy),
//! #     Item::separator(),
//! #     Item::action("Delete", Message::Delete).destructive(true),
//! # ];
//! # let on_select = std::rc::Rc::new(|_: &[usize]| Message::Copy);
//! # let class: menu::StyleFn<'static, iced_kit::Theme> =
//! #     <iced_kit::Theme as menu::Catalog>::default();
//! if !open.is_open() {
//!     state.reset();
//! }
//! let menu: menu::Menu<'_, '_, Message, iced_kit::Theme> =
//!     menu::Menu::new(&mut state, &items, Some(on_select), open.clone(), &class);
//! # let trigger = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(20.0, 20.0));
//! # let viewport = iced::Rectangle::with_size(iced::Size::new(800.0, 600.0));
//! // Handed to the hosting widget's `Widget::overlay` each frame it is open.
//! let element = menu.overlay::<iced::Renderer>(trigger, viewport);
//! # drop(element);
//! ```
//!
//! # Entries without messages
//!
//! Most applications build entries with [`Item::action`], which carries the
//! message selecting it publishes. A host whose actions are cheaper to build
//! when chosen than when the menu opens — the dock, whose actions are
//! dispatched by being turned into messages — builds [`Item::command`]
//! entries instead and resolves the choice itself through the `on_select`
//! callback, which receives the path of the chosen entry (one index per menu
//! level, root first).

use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::text as adv_text;
use iced::advanced::text::paragraph;
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::key::Named;
use iced::mouse::{self, Cursor};
use iced::widget::text::{layout as text_layout, Format, Shaping};
use iced::{Color, Event, Pixels, Point, Rectangle, Size, Vector};

use crate::icons::IconName;

/// How far a popup keeps from the window edge, in logical pixels.
///
/// `gpui-kit` pads its popup layers by the same amount, so a menu pinned to
/// the edge sits a shadow's width inside the glass rather than flush with it.
const EDGE_MARGIN: f32 = 8.0;

/// How far a submenu overlaps the row that opened it.
///
/// A small overlap keeps the pointer travelling in a straight line from the
/// row into the submenu; without it, the gap between the two surfaces is
/// dead space a diagonal move can cross, folding the submenu mid-trip.
const SUBMENU_OVERLAP: f32 = 4.0;

/// The height of one selectable row.
const ROW_HEIGHT: f32 = 28.0;

/// The height of a separator, its line plus the air above and below.
const SEPARATOR_HEIGHT: f32 = 9.0;

/// The air between the menu's border and its rows.
const MENU_PADDING: f32 = 4.0;

/// The horizontal air inside a row, between the border and the content.
const ROW_PADDING: f32 = 8.0;

/// The gap between the icon slot and the label, and between label and hint.
const CONTENT_GAP: f32 = 8.0;

/// The narrowest a menu can come out, whatever its rows measure.
///
/// A menu holding only `Cut` would otherwise be a sliver; a minimum keeps a
/// short menu reading as a menu.
const MIN_WIDTH: f32 = 140.0;

/// The widest a menu grows before a long label is ellipsized rather than
/// widening the menu across the window.
///
/// `gpui-kit` caps its popups at the same figure.
const MAX_WIDTH: f32 = 420.0;

/// One entry in a menu.
///
/// Construct one per row: [`Item::action`] for a command that carries its
/// own message, [`Item::command`] for one the host resolves itself,
/// [`Item::submenu`] for a nested menu, [`Item::label`] for a non-interactive
/// row, and [`Item::separator`] for a rule between groups.
#[derive(Debug, Clone)]
pub enum Item<Message> {
    /// A command the pointer or keyboard can choose.
    Command(Command<Message>),
    /// A nested menu that opens beside this row.
    Submenu(Submenu<Message>),
    /// A row of text that answers to nothing.
    Label(String),
    /// A horizontal rule between groups of rows.
    ///
    /// Rules at the edges of a menu and runs of consecutive rules collapse:
    /// a rule exists to divide groups, and there is no group on one side.
    Separator,
}

/// A command row: what it says, and what choosing it does.
#[derive(Debug, Clone)]
pub struct Command<Message> {
    label: String,
    message: Option<Message>,
    icon: Option<IconName>,
    shortcut: Option<String>,
    checked: Option<bool>,
    enabled: bool,
    destructive: bool,
}

/// A row that opens a nested menu.
#[derive(Debug, Clone)]
pub struct Submenu<Message> {
    label: String,
    icon: Option<IconName>,
    enabled: bool,
    items: Vec<Item<Message>>,
}

impl<Message> Item<Message> {
    /// Creates a command that publishes `message` when chosen.
    pub fn action(label: impl Into<String>, message: Message) -> Self {
        Self::Command(Command {
            label: label.into(),
            message: Some(message),
            icon: None,
            shortcut: None,
            checked: None,
            enabled: true,
            destructive: false,
        })
    }

    /// Creates a command whose outcome the host resolves.
    ///
    /// Choosing it calls `on_select` with the entry's path — the route a
    /// host takes when building the message is cheaper done at choice time,
    /// as the dock's are.
    pub fn command(label: impl Into<String>) -> Self {
        Self::Command(Command {
            label: label.into(),
            message: None,
            icon: None,
            shortcut: None,
            checked: None,
            enabled: true,
            destructive: false,
        })
    }

    /// Creates a row that opens a nested menu.
    pub fn submenu(label: impl Into<String>, items: Vec<Self>) -> Self {
        Self::Submenu(Submenu {
            label: label.into(),
            icon: None,
            enabled: true,
            items,
        })
    }

    /// Creates a row of text that answers to nothing.
    pub fn label(label: impl Into<String>) -> Self {
        Self::Label(label.into())
    }

    /// Creates a horizontal rule.
    pub fn separator() -> Self {
        Self::Separator
    }

    /// Sets the leading icon glyph of this row.
    ///
    /// On a separator or label this does nothing.
    #[must_use]
    pub fn icon(mut self, icon: IconName) -> Self {
        match &mut self {
            Self::Command(command) => command.icon = Some(icon),
            Self::Submenu(submenu) => submenu.icon = Some(icon),
            _ => {}
        }
        self
    }

    /// Sets the trailing keyboard hint of this row, e.g. `"Ctrl+C"`.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        if let Self::Command(command) = &mut self {
            command.shortcut = Some(shortcut.into());
        }
        self
    }

    /// Marks the row with (or without) a check.
    #[must_use]
    pub fn checked(mut self, checked: bool) -> Self {
        if let Self::Command(command) = &mut self {
            command.checked = Some(checked);
        }
        self
    }

    /// Enables or disables the row. A disabled row renders dimmed and
    /// answers to nothing.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        match &mut self {
            Self::Command(command) => command.enabled = enabled,
            Self::Submenu(submenu) => submenu.enabled = enabled,
            _ => {}
        }
        self
    }

    /// Renders the row in the danger color, for a destructive command.
    #[must_use]
    pub fn destructive(mut self, destructive: bool) -> Self {
        if let Self::Command(command) = &mut self {
            command.destructive = destructive;
        }
        self
    }

    /// Whether the keyboard or pointer can choose this row.
    fn is_selectable(&self) -> bool {
        match self {
            Self::Command(command) => command.enabled,
            Self::Submenu(submenu) => submenu.enabled,
            Self::Label(_) | Self::Separator => false,
        }
    }

    /// Whether the row carries a leading slot: an icon, a check, or both.
    ///
    /// Every row of a menu reserves the slot when any row carries one, so a
    /// label lines up with the labels beside it instead of starting where an
    /// icon would sit.
    fn has_leading_slot(&self) -> bool {
        match self {
            Self::Command(command) => command.icon.is_some() || command.checked.is_some(),
            Self::Submenu(submenu) => submenu.icon.is_some(),
            _ => false,
        }
    }
}

/// What a menu calls when a command row without a message of its own is
/// chosen. It receives the row's path: one index per menu level, root first.
pub type OnSelect<'a, Message> = Rc<dyn Fn(&[usize]) -> Message + 'a>;

/// The width `text` needs on one line, as the renderer will draw it.
///
/// iced's `fill_text` takes a `bounds` that acts as the wrap width and ignores
/// the `wrapping` field, so a label is only kept to one line by drawing it
/// into a box at least as wide as it measures. That makes the measurement —
/// not an estimate — the thing the menu's width has to be built from.
fn text_width<Renderer>(
    renderer: &Renderer,
    paragraph: &mut paragraph::Plain<Renderer::Paragraph>,
    text: &str,
    size: f32,
) -> f32
where
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    let limits = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, f32::INFINITY));
    let format = Format {
        width: iced::Length::Shrink,
        height: iced::Length::Shrink,
        size: Some(Pixels(size)),
        font: Some(iced::Font::DEFAULT),
        line_height: adv_text::LineHeight::Relative(1.0),
        shaping: Shaping::Basic,
        ..Format::default()
    };

    text_layout(paragraph, renderer, &limits, text, format)
        .size()
        .width
}

/// The width one row needs, measured: its icon slot, label and trailing
/// marks, plus the row's own padding.
///
/// `has_leading` widens every row that carries no icon or check of its own,
/// because the slot is reserved across the menu so labels line up.
fn row_width<Message, Renderer>(
    renderer: &Renderer,
    paragraph: &mut paragraph::Plain<Renderer::Paragraph>,
    item: &Item<Message>,
    metrics: Metrics,
    has_leading: bool,
) -> f32
where
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    let mut width = ROW_PADDING * 2.0;

    if item.has_leading_slot() || has_leading {
        width += metrics.icon_size + CONTENT_GAP;
    }

    match item {
        Item::Command(command) => {
            width += text_width(renderer, paragraph, &command.label, metrics.text_size);

            if let Some(shortcut) = &command.shortcut {
                width += CONTENT_GAP;
                width += text_width(renderer, paragraph, shortcut, metrics.shortcut_size);
            }
        }
        Item::Submenu(submenu) => {
            width += text_width(renderer, paragraph, &submenu.label, metrics.text_size);
            // The chevron every submenu row ends with.
            width += CONTENT_GAP + metrics.icon_size;
        }
        Item::Label(label) => {
            width += text_width(renderer, paragraph, label, metrics.text_size);
        }
        Item::Separator => {}
    }

    width
}

/// The text sizes a menu lays its rows out at.
///
/// An overlay lays out without a theme — `Overlay::layout` is handed a
/// renderer and the window's bounds, nothing else — so the sizes the rows are
/// measured with cannot come from the style at that point. The host resolves
/// the style once and passes these in, the way the tab strip hands its own
/// overflow menu a text size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// The size of a row's label.
    pub text_size: f32,
    /// The size of a keyboard hint.
    pub shortcut_size: f32,
    /// The size of an icon or check glyph.
    pub icon_size: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            text_size: 13.0,
            shortcut_size: 12.0,
            icon_size: crate::theme::Size::Sm.icon_size(),
        }
    }
}

impl From<&Style> for Metrics {
    fn from(style: &Style) -> Self {
        Self {
            text_size: style.text_size,
            shortcut_size: style.shortcut_size,
            icon_size: style.icon_size,
        }
    }
}

/// The width a menu needs to lay every row out on one line.
///
/// The measured rows, plus the air inside the border, floored at
/// [`MIN_WIDTH`] and capped at [`MAX_WIDTH`] so one long label cannot widen a
/// menu across the window.
fn measured_width<Message, Renderer>(
    renderer: &Renderer,
    items: &[Item<Message>],
    metrics: Metrics,
) -> f32
where
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    let mut paragraph = paragraph::Plain::default();
    let has_leading = any_leading_slot(items);

    let widest = effective_rows(items)
        .iter()
        .map(|ix| row_width(renderer, &mut paragraph, &items[*ix], metrics, has_leading))
        .fold(0.0_f32, f32::max);

    (widest + MENU_PADDING * 2.0).clamp(MIN_WIDTH, MAX_WIDTH)
}

/// The rows a menu actually draws, with the index each came from.
///
/// Separators collapse where they would divide nothing: at the very start or
/// end of the list, or in a run of two or more. The index is the row's place
/// in the host's item list, which is what selection reports.
fn effective_rows<Message>(items: &[Item<Message>]) -> Vec<usize> {
    let mut rows: Vec<usize> = Vec::new();

    for (ix, item) in items.iter().enumerate() {
        match item {
            Item::Separator => {
                let previous_is_separator = rows
                    .last()
                    .is_some_and(|ix| matches!(items[*ix], Item::Separator));
                if !previous_is_separator {
                    rows.push(ix);
                }
            }
            _ => rows.push(ix),
        }
    }

    while rows
        .last()
        .is_some_and(|ix| matches!(items[*ix], Item::Separator))
    {
        rows.pop();
    }

    rows
}

/// The height one row of the menu occupies.
fn row_height<Message>(item: &Item<Message>) -> f32 {
    match item {
        Item::Separator => SEPARATOR_HEIGHT,
        _ => ROW_HEIGHT,
    }
}

/// The menu's content height: every effective row, plus the air inside the
/// border.
fn content_height<Message>(items: &[Item<Message>]) -> f32 {
    let rows = effective_rows(items);
    let height: f32 = rows.iter().map(|ix| row_height(&items[*ix])).sum();
    height + MENU_PADDING * 2.0
}

/// Whether any effective row carries a leading slot.
fn any_leading_slot<Message>(items: &[Item<Message>]) -> bool {
    effective_rows(items)
        .iter()
        .any(|ix| items[*ix].has_leading_slot())
}

/// The y where row `ix` starts, relative to the content's top.
fn row_offset<Message>(items: &[Item<Message>], target: usize) -> f32 {
    effective_rows(items)
        .iter()
        .take_while(|ix| **ix != target)
        .map(|ix| row_height(&items[*ix]))
        .sum()
}

/// The local state of one menu level.
///
/// Each menu — the root as well as every open submenu — owns one. The state
/// lives in the host's tree for as long as the menu is built, which is what
/// keeps a selection and a scroll position across frames.
#[derive(Debug, Clone)]
pub struct State {
    /// The selected row, as an index into the host's item list.
    selected: Option<usize>,
    /// How far the rows are scrolled down, in logical pixels.
    scroll: f32,
    /// The open submenu: which row opened it, and its own state.
    submenu: Option<(usize, Box<State>)>,
}

impl State {
    /// Creates empty state for a menu.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the selection, the scroll and any open submenu.
    ///
    /// A host calls this when a menu closes, so the next opening starts
    /// where a fresh menu would.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Whether a submenu of this menu is open.
    #[must_use]
    pub fn has_submenu(&self) -> bool {
        self.submenu.is_some()
    }

    /// The row this level has selected, if any.
    ///
    /// The selection is what a hover or a keyboard move sets, and what the
    /// next `draw` highlights: reading it is how a test checks that a pointer
    /// landed where it looked like it landed.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            selected: None,
            scroll: 0.0,
            submenu: None,
        }
    }
}

/// The shared open flag of a menu.
///
/// The host keeps one in its tree state and consults it to decide whether to
/// build the overlay; the menu writes it when it dismisses itself — a choice
/// was made, a press landed outside, `Escape` was pressed. Sharing a flag
/// rather than passing messages is what lets the menu close without a round
/// trip through the application, and what lets a submenu deep inside a chain
/// fold the whole thing: every level holds a clone of the same flag.
#[derive(Debug, Clone)]
pub struct OpenFlag(Rc<Cell<bool>>);

impl OpenFlag {
    /// Creates a closed flag.
    #[must_use]
    pub fn closed() -> Self {
        Self(Rc::new(Cell::new(false)))
    }

    /// Opens the menu.
    pub fn open(&self) {
        self.0.set(true);
    }

    /// Closes the menu.
    pub fn close(&self) {
        self.0.set(false);
    }

    /// Whether the menu is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.0.get()
    }

    /// Toggles the menu, returning whether it is open now.
    pub fn toggle(&self) -> bool {
        let open = !self.0.get();
        self.0.set(open);
        open
    }
}

impl Default for OpenFlag {
    fn default() -> Self {
        Self::closed()
    }
}

/// The appearance of a menu.
#[derive(Debug, Clone)]
pub struct Style {
    /// The [`iced::Background`] of the menu surface.
    pub background: iced::Background,
    /// The [`iced::Border`] of the menu surface.
    pub border: iced::Border,
    /// The [`iced::Shadow`] cast by the menu.
    pub shadow: iced::Shadow,
    /// The text color of a row.
    pub text_color: Color,
    /// The text color of a secondary mark: a keyboard hint.
    pub muted_color: Color,
    /// The text color of a destructive row.
    pub destructive_color: Color,
    /// The background drawn behind the selected row.
    pub hovered_background: Color,
    /// The color of a separator.
    pub separator_color: Color,
    /// The size of a row's label.
    pub text_size: f32,
    /// The size of an icon or check glyph.
    pub icon_size: f32,
    /// The size of a keyboard hint.
    pub shortcut_size: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: iced::Background::Color(Color::WHITE),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            text_color: Color::BLACK,
            muted_color: Color::BLACK,
            destructive_color: Color::BLACK,
            hovered_background: Color::TRANSPARENT,
            separator_color: Color::TRANSPARENT,
            text_size: 13.0,
            icon_size: crate::theme::Size::Sm.icon_size(),
            shortcut_size: 12.0,
        }
    }
}

/// The theme catalog of a [`Menu`].
pub trait Catalog {
    /// The class of the catalog.
    type Class<'a>;

    /// The default class of the catalog.
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// A styling function for a [`Menu`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for crate::theme::Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme| {
            let colors = theme.colors();

            Style {
                background: iced::Background::Color(colors.surface),
                border: iced::Border {
                    color: colors.border,
                    width: 1.0,
                    radius: f32::from(theme.radius().md).into(),
                },
                shadow: super::floating_shadow(theme),
                text_color: colors.foreground,
                muted_color: colors.muted_foreground,
                destructive_color: colors.destructive,
                hovered_background: colors.accent,
                separator_color: colors.border,
                ..Style::default()
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

impl Catalog for iced::Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|theme: &iced::Theme| {
            let palette = theme.extended_palette();

            Style {
                background: iced::Background::Color(palette.background.weak.color),
                border: iced::Border {
                    color: palette.background.strong.color,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                text_color: palette.background.weak.text,
                muted_color: palette.secondary.weak.color,
                destructive_color: palette.danger.strong.color,
                hovered_background: palette.primary.weak.color,
                separator_color: palette.background.strong.color,
                ..Style::default()
            }
        })
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

/// A popup menu.
///
/// Built from the host's state, the host's items and the shared
/// [`OpenFlag`], and turned into an [`overlay::Element`] anchored to the
/// trigger's rectangle in window coordinates:
///
/// ```
/// # use iced_kit::widgets::overlay::menu::{self, Item, OpenFlag};
/// # #[derive(Clone, Debug)] enum Message { Copy }
/// # let mut state = menu::State::new();
/// # let open = OpenFlag::closed();
/// # open.open();
/// # let items = vec![Item::action("Copy", Message::Copy)];
/// # let on_select = std::rc::Rc::new(|_: &[usize]| Message::Copy);
/// # let class: menu::StyleFn<'static, iced_kit::Theme> =
/// #     <iced_kit::Theme as menu::Catalog>::default();
/// # let trigger = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(20.0, 20.0));
/// # let viewport = iced::Rectangle::with_size(iced::Size::new(800.0, 600.0));
/// let menu: menu::Menu<'_, '_, Message, iced_kit::Theme> =
///     menu::Menu::new(&mut state, &items, Some(on_select), open, &class).width(180.0);
/// let element = menu.overlay::<iced::Renderer>(trigger, viewport);
/// # drop(element);
/// ```
#[must_use = "a Menu does nothing unless it is turned into an overlay"]
pub struct Menu<'a, 'b, Message, Theme = crate::theme::Theme>
where
    Theme: Catalog,
    'b: 'a,
{
    state: &'a mut State,
    items: &'a [Item<Message>],
    on_select: Option<OnSelect<'a, Message>>,
    open: OpenFlag,
    class: &'a <Theme as Catalog>::Class<'b>,
    /// The text sizes the rows measure and draw at.
    metrics: Metrics,
    /// The floor under the measured width.
    width: f32,
    max_height: Option<f32>,
}

impl<'a, 'b, Message, Theme> Menu<'a, 'b, Message, Theme>
where
    Message: Clone + 'a,
    'b: 'a,
    Theme: Catalog + 'a,
{
    /// Creates a menu from its state, items, style class and open flag.
    ///
    /// The class is lent from the host — a host keeps one in tree state as
    /// [`Class<'static>`][Catalog] — so a menu can be styled per host the way
    /// the dock's panel menu is.
    ///
    /// `on_select` resolves an [`Item::command`] — a row built without a
    /// message of its own — into one, receiving the path of the chosen row
    /// (one index per menu level, root first). A menu built entirely of
    /// [`Item::action`] rows passes [`Option::None`].
    pub fn new(
        state: &'a mut State,
        items: &'a [Item<Message>],
        on_select: Option<OnSelect<'a, Message>>,
        open: OpenFlag,
        class: &'a <Theme as Catalog>::Class<'b>,
    ) -> Self {
        Self {
            state,
            items,
            on_select,
            open,
            class,
            metrics: Metrics::default(),
            width: MIN_WIDTH,
            max_height: None,
        }
    }

    /// Sets the text sizes the rows are measured and drawn at.
    ///
    /// A host that styles its menu takes these from the same style, so the
    /// width the labels are measured at is the width they are drawn at. The
    /// default matches [`Style::default`].
    pub fn metrics(mut self, metrics: Metrics) -> Self {
        self.metrics = metrics;
        self
    }

    /// Sets the narrowest the menu may be, in logical pixels.
    ///
    /// A menu is as wide as its widest row needs — labels are never wrapped
    /// or clipped to fit a guessed width — and this is the floor under that,
    /// so a menu of `Cut` and `Paste` still reads as a menu.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the tallest the menu may grow, in logical pixels. Rows beyond it
    /// scroll. The default caps a menu at half the window's height.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Turns the menu into an overlay element anchored to `trigger`.
    ///
    /// The menu drops below the trigger when there is room, rises above it
    /// when there is not, and — when the trigger sits so near the window's
    /// right edge that the menu would leave the glass — opens leftward from
    /// the trigger's right edge, so a menu on the rightmost dock panel reads
    /// instead of clipping out of the window.
    pub fn overlay<Renderer>(
        self,
        trigger: Rectangle,
        viewport: Rectangle,
    ) -> overlay::Element<'a, Message, Theme, Renderer>
    where
        Renderer: adv_text::Renderer<Font = iced::Font> + 'a,
    {
        overlay::Element::new(Box::new(Overlay {
            root: true,
            trigger,
            viewport,
            min_width: self.width,
            max_height: self.max_height,
            metrics: self.metrics,
            state: self.state,
            items: self.items,
            prefix: Rc::from(Vec::new().into_boxed_slice()),
            on_select: self.on_select,
            open: self.open,
            class: self.class,
            _renderer: std::marker::PhantomData,
        }))
    }
}

/// One level of an open menu: the root, or a submenu.
struct Overlay<'a, 'b, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: adv_text::Renderer<Font = iced::Font>,
    'b: 'a,
{
    /// Whether this level is the root of its menu.
    ///
    /// Only the root closes on a press outside and on a final `Escape`; a
    /// submenu cannot fold itself, it is folded by its parent.
    root: bool,
    /// The window-space rectangle the menu anchors to: the trigger button for
    /// the root, the row that opened it for a submenu.
    trigger: Rectangle,
    viewport: Rectangle,
    /// The floor under the width the rows measure out to.
    min_width: f32,
    max_height: Option<f32>,
    /// The text sizes the rows measure and draw at.
    metrics: Metrics,
    state: &'a mut State,
    items: &'a [Item<Message>],
    /// The path of this level within the host's items, root first.
    prefix: Rc<[usize]>,
    on_select: Option<OnSelect<'a, Message>>,
    open: OpenFlag,
    class: &'a <Theme as Catalog>::Class<'b>,
    /// The overlay is generic over the renderer only because its draw uses
    /// the text pipeline; nothing stores one.
    _renderer: std::marker::PhantomData<Renderer>,
}

impl<Message, Theme, Renderer> Overlay<'_, '_, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: Catalog,
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    /// The rows this level draws, resolved against the host's items.
    fn rows(&self) -> Vec<usize> {
        effective_rows(self.items)
    }

    /// The menu's height for this frame: the content, capped by the host's
    /// ceiling and the window.
    fn view_height(&self, window_height: f32) -> f32 {
        let ceiling = self
            .max_height
            .unwrap_or_else(|| (window_height * 0.5).min(450.0));
        content_height(self.items).min(ceiling)
    }

    /// The window-space rectangle of row `ix` this frame.
    fn row_bounds(&self, ix: usize, menu_bounds: Rectangle) -> Rectangle {
        let y = menu_bounds.y + MENU_PADDING + row_offset(self.items, ix) - self.state.scroll;
        Rectangle {
            x: menu_bounds.x,
            y,
            width: menu_bounds.width,
            height: row_height(&self.items[ix]),
        }
    }

    /// The row a point within the menu falls on, if any.
    ///
    /// `position` is relative to the menu's own top-left corner — the form
    /// [`Cursor::position_in`] returns. Taking it that way, rather than taking
    /// a window-space point and subtracting the menu's origin here, is what
    /// keeps the two coordinate spaces from being confused: there is no way
    /// for a caller to pass an absolute point and have it silently counted
    /// twice.
    fn row_at(&self, position: Point) -> Option<usize> {
        let local_y = position.y - MENU_PADDING + self.state.scroll;
        let mut offset = 0.0;

        for ix in self.rows() {
            let height = row_height(&self.items[ix]);
            if local_y >= offset && local_y < offset + height {
                return Some(ix);
            }
            offset += height;
        }

        None
    }

    /// The indices a keyboard move may land on, in order.
    fn selectable(&self) -> Vec<usize> {
        (0..self.items.len())
            .filter(|ix| self.items[*ix].is_selectable())
            .collect()
    }

    /// Moves the selection one step among the selectable rows, wrapping at
    /// the ends, and scrolls it into view.
    fn move_selection(&mut self, step: isize, menu_bounds: Rectangle) {
        let selectable = self.selectable();
        let Some(fallback) = selectable.first().copied() else {
            return;
        };

        let position = self
            .state
            .selected
            .and_then(|selected| selectable.iter().position(|ix| *ix == selected));

        let next = match position {
            Some(position) => {
                let count = isize::try_from(selectable.len())
                    .expect("a menu cannot have more rows than isize holds");
                let current =
                    isize::try_from(position).expect("the index of a row is within the list");
                let next = usize::try_from((current + step).rem_euclid(count))
                    .expect("a wrapped index is not negative");
                selectable[next]
            }
            // Nothing selected yet: down starts at the top, up at the bottom.
            None => {
                if step < 0 {
                    *selectable.last().expect("at least one row")
                } else {
                    fallback
                }
            }
        };

        self.state.selected = Some(next);
        self.scroll_to(next, menu_bounds);
    }

    /// Scrolls the selected row to where it can be read.
    fn scroll_to(&mut self, ix: usize, menu_bounds: Rectangle) {
        let view = menu_bounds.height - MENU_PADDING * 2.0;
        let top = row_offset(self.items, ix);
        let bottom = top + row_height(&self.items[ix]);
        let visible_top = self.state.scroll;
        let visible_bottom = self.state.scroll + view;

        if top < visible_top {
            self.state.scroll = top;
        } else if bottom > visible_bottom {
            self.state.scroll = bottom - view;
        }
    }

    /// Lines the open submenu up with the selection: a submenu opens when its
    /// row is selected and folds when the selection moves off it, which is
    /// how a pointer and the arrow keys both walk a menu.
    fn reconcile_submenu(&mut self) {
        let selected = self.state.selected;

        let keep = match (selected, &self.state.submenu) {
            (Some(selected), Some((ix, _))) => selected == *ix,
            _ => false,
        };

        if keep {
            return;
        }

        match selected.and_then(|ix| self.items.get(ix)) {
            Some(Item::Submenu(_)) => {
                self.state.submenu = selected.map(|ix| (ix, Box::new(State::new())));
            }
            _ => self.state.submenu = None,
        }
    }

    /// Chooses a row: publishes its message or resolves it through
    /// `on_select`, then folds the whole menu.
    fn confirm(&mut self, ix: usize, shell: &mut Shell<'_, Message>) {
        let path: Vec<usize> = self.prefix.iter().copied().chain([ix]).collect();
        let message = match &self.items[ix] {
            Item::Command(command) => command
                .message
                .clone()
                .or_else(|| self.on_select.as_ref().map(|on_select| on_select(&path))),
            _ => None,
        };

        if let Some(message) = message {
            shell.publish(message);
        }

        self.open.close();
        shell.capture_event();
        shell.request_redraw();
    }

    /// The horizontal origin for a menu of `width`: left of the trigger's
    /// right edge when the trigger sits near the window's right edge — the
    /// menu opens inward rather than off the glass — and the trigger's left
    /// edge otherwise.
    fn horizontal_origin(&self, width: f32, window_width: f32) -> f32 {
        let room_right = window_width - EDGE_MARGIN - self.trigger.x;

        let x = if self.root {
            if width <= room_right {
                self.trigger.x
            } else {
                self.trigger.x + self.trigger.width - width
            }
        } else {
            let right = self.trigger.x + self.trigger.width - SUBMENU_OVERLAP;
            let left = self.trigger.x - width + SUBMENU_OVERLAP;

            if right + width <= window_width - EDGE_MARGIN {
                right
            } else if left >= EDGE_MARGIN {
                left
            } else {
                window_width - EDGE_MARGIN - width
            }
        };

        x.clamp(0.0, (window_width - width).max(0.0))
    }

    /// The vertical origin and the height the menu may take: below the
    /// trigger when the menu fits there, above it when it fits there, and on
    /// the roomier side — capped to it — when it fits nowhere.
    fn vertical_placement(&self, height: f32, window_height: f32) -> (f32, f32) {
        if self.root {
            let space_below = window_height - EDGE_MARGIN - (self.trigger.y + self.trigger.height);
            let space_above = self.trigger.y - EDGE_MARGIN;

            if height <= space_below {
                (self.trigger.y + self.trigger.height, height)
            } else if height <= space_above {
                (self.trigger.y - height, height)
            } else if space_below >= space_above {
                let capped = space_below.max(MENU_PADDING * 2.0);
                (self.trigger.y + self.trigger.height, capped)
            } else {
                let capped = space_above.max(MENU_PADDING * 2.0);
                (self.trigger.y - capped, capped)
            }
        } else {
            // A submenu hangs from its row, pulled back inside the window
            // when the row sits low.
            let capped = height
                .min(window_height - EDGE_MARGIN * 2.0)
                .max(MENU_PADDING * 2.0);
            let y = self
                .trigger
                .y
                .min(window_height - EDGE_MARGIN - capped)
                .max(EDGE_MARGIN);
            (y, capped)
        }
    }
}

impl<Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'_, '_, Message, Theme, Renderer>
where
    Message: Clone,
    Theme: Catalog,
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        // The width comes from what the rows measure, so every label is drawn
        // on one line rather than wrapped to fit a guessed box. The host's
        // floor and the window's room both cap it.
        let measured = measured_width(renderer, self.items, self.metrics).max(self.min_width);
        let width = measured.min((bounds.width - EDGE_MARGIN * 2.0).max(0.0));
        let content = self.view_height(bounds.height);
        let (y, height) = self.vertical_placement(content, bounds.height);
        let x = self.horizontal_origin(width, bounds.width);

        // The rows scroll within the view; never park the scroll past the
        // last row when the window shrank since the last frame.
        let view = height - MENU_PADDING * 2.0;
        let max_scroll = (content - MENU_PADDING * 2.0 - view).max(0.0);
        self.state.scroll = self.state.scroll.clamp(0.0, max_scroll);

        layout::Node::new(Size::new(width, height)).move_to(Point::new(x, y))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let menu_bounds = layout.bounds();

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. }) => {
                let Some(point) = cursor.position() else {
                    return;
                };

                if menu_bounds.contains(point) {
                    // A press inside the menu answers to the row it landed
                    // on — a choice, or a submenu folding open — and is
                    // swallowed either way, so nothing under the menu
                    // answers to it.
                    if let Some(ix) =
                        self.row_at(Point::new(point.x - menu_bounds.x, point.y - menu_bounds.y))
                    {
                        match &self.items[ix] {
                            Item::Submenu(_) if self.items[ix].is_selectable() => {
                                let already = self
                                    .state
                                    .submenu
                                    .as_ref()
                                    .is_some_and(|(open, _)| *open == ix);
                                if already {
                                    self.state.submenu = None;
                                } else {
                                    self.state.selected = Some(ix);
                                    self.state.submenu = Some((ix, Box::new(State::new())));
                                }
                            }
                            item if item.is_selectable() => {
                                self.confirm(ix, shell);
                            }
                            _ => {}
                        }
                    }

                    shell.capture_event();
                    shell.request_redraw();
                } else if self.root {
                    // A press outside the menu closes it, and is swallowed so
                    // it does not also press what happens to sit under the
                    // point — a menu is a question, and the answer comes
                    // before whatever else the pointer wanted.
                    self.open.close();
                    shell.capture_event();
                    shell.request_redraw();
                }
                // A submenu leaves an outside press to its parent, which
                // folds it or passes the press further up.
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.is_over(menu_bounds) {
                    let lines = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => *y * ROW_HEIGHT,
                        mouse::ScrollDelta::Pixels { y, .. } => *y,
                    };

                    // The wheel belongs to the menu while the pointer is over
                    // it, scrollable or not: the page scrolling under an open
                    // menu reads as the menu tearing loose.
                    let content = content_height(self.items) - MENU_PADDING * 2.0;
                    let view = menu_bounds.height - MENU_PADDING * 2.0;
                    let max_scroll = (content - view).max(0.0);
                    self.state.scroll = (self.state.scroll + lines).clamp(0.0, max_scroll);
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Touch(iced::touch::Event::FingerMoved { .. }) => {
                // `position_in` is the menu-local form `row_at` takes. Passing
                // an absolute point here instead is what left the highlight a
                // whole menu behind the pointer: `row_at` measures from the
                // menu's own top, so an absolute point gets its height counted
                // twice.
                let Some(point) = cursor.position_in(menu_bounds) else {
                    return;
                };

                if let Some(ix) = self.row_at(point) {
                    if self.items[ix].is_selectable() && self.state.selected != Some(ix) {
                        self.state.selected = Some(ix);
                        self.reconcile_submenu();
                        shell.request_redraw();
                    }
                }
            }
            Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) => {
                // A level with an open submenu yields the arrow keys to it —
                // the deepest menu walks first. It keeps only `Escape` and
                // `Left`, which fold its own submenu.
                let child_open = self.state.submenu.is_some();

                let iced::keyboard::Key::Named(named) = key else {
                    return;
                };
                let named = *named;

                match named {
                    Named::Escape => {
                        if child_open {
                            self.state.submenu = None;
                        } else if self.root {
                            self.open.close();
                            self.state.reset();
                        } else {
                            // A submenu cannot fold itself; its parent,
                            // which sees this event after the child ignores
                            // it, does the folding.
                            return;
                        }
                        shell.capture_event();
                        shell.request_redraw();
                    }
                    Named::ArrowUp | Named::ArrowDown if !child_open => {
                        let step = if named == Named::ArrowUp { -1isize } else { 1 };
                        self.move_selection(step, menu_bounds);
                        self.reconcile_submenu();
                        shell.capture_event();
                        shell.request_redraw();
                    }
                    Named::ArrowLeft if child_open => {
                        self.state.submenu = None;
                        shell.capture_event();
                        shell.request_redraw();
                    }
                    Named::ArrowRight if !child_open => {
                        let opens = self
                            .state
                            .selected
                            .is_some_and(|ix| matches!(self.items.get(ix), Some(Item::Submenu(_))));
                        if opens {
                            self.reconcile_submenu();
                            shell.capture_event();
                            shell.request_redraw();
                        }
                    }
                    Named::Enter if !child_open => {
                        if let Some(ix) = self.state.selected {
                            match self.items.get(ix) {
                                Some(Item::Submenu(_)) if self.items[ix].is_selectable() => {
                                    self.state.submenu = Some((ix, Box::new(State::new())));
                                    shell.capture_event();
                                    shell.request_redraw();
                                }
                                Some(item) if item.is_selectable() => {
                                    self.confirm(ix, shell);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
    ) {
        let bounds = layout.bounds();
        let style = Catalog::style(theme, self.class);
        let rows = self.rows();
        let has_leading = any_leading_slot(self.items);
        let mut paragraph = paragraph::Plain::default();

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: style.border,
                shadow: style.shadow,
                ..renderer::Quad::default()
            },
            style.background,
        );

        // The runtime clips this level to its own bounds, so rows scrolled
        // out of view draw nowhere without culling.
        for ix in rows {
            let row = self.row_bounds(ix, bounds);
            let row_top = row.y - bounds.y;

            if row_bottom_of(row_top, row.height) < 0.0 || row_top > bounds.height {
                continue;
            }

            match &self.items[ix] {
                Item::Separator => {
                    let line = Rectangle {
                        x: bounds.x + MENU_PADDING,
                        y: row.y + SEPARATOR_HEIGHT / 2.0 - 0.5,
                        width: bounds.width - MENU_PADDING * 2.0,
                        height: 1.0,
                    };
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: line,
                            ..renderer::Quad::default()
                        },
                        style.separator_color,
                    );
                }
                Item::Label(label) => {
                    draw_label(renderer, &style, row, label, style.muted_color);
                }
                Item::Command(command) => {
                    let selected = self.state.selected == Some(ix) && command.enabled;
                    draw_selectable_row(
                        renderer,
                        &mut paragraph,
                        &style,
                        row,
                        selected,
                        has_leading,
                        command.label.clone(),
                        command.icon,
                        command.checked,
                        command.shortcut.clone(),
                        command.destructive,
                        None,
                    );
                }
                Item::Submenu(submenu) => {
                    let selected = self.state.selected == Some(ix);
                    draw_selectable_row(
                        renderer,
                        &mut paragraph,
                        &style,
                        row,
                        selected,
                        has_leading,
                        submenu.label.clone(),
                        submenu.icon,
                        None,
                        None,
                        false,
                        Some(crate::icons::IconName::ChevronRight),
                    );
                }
            }
        }

        // A scrollbar when the rows outrun the view.
        let content = content_height(self.items) - MENU_PADDING * 2.0;
        let view = bounds.height - MENU_PADDING * 2.0;
        if content > view && view > 0.0 {
            let track = bounds.height - MENU_PADDING * 2.0;
            let thumb_height = (view * view / content).max(16.0);
            let progress = self.state.scroll / (content - view);
            let thumb_y = bounds.y + MENU_PADDING + progress * (track - thumb_height);
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x + bounds.width - 4.0,
                        y: thumb_y,
                        width: 3.0,
                        height: thumb_height,
                    },
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..iced::Border::default()
                    },
                    ..renderer::Quad::default()
                },
                style.separator_color,
            );
        }
    }

    /// The open submenu, as an overlay nested inside this one.
    ///
    /// The runtime lays a nested overlay out with the window's bounds and
    /// draws it after — above — its parent, and routes events to the deepest
    /// level first. Building it here, per frame, is what keeps the submenu
    /// pinned to its row as either of them moves.
    fn overlay<'b>(
        &'b mut self,
        layout: Layout<'b>,
        _renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let menu_bounds = layout.bounds();

        let ix = self.state.submenu.as_ref().map(|(ix, _)| *ix)?;
        let trigger = self.row_bounds(ix, menu_bounds);

        let items = self.items;
        let Item::Submenu(submenu) = items.get(ix)? else {
            return None;
        };

        let mut prefix = (*self.prefix).to_vec();
        prefix.push(ix);

        let (_, child_state) = self.state.submenu.as_mut()?;

        Some(overlay::Element::new(Box::new(Overlay {
            root: false,
            trigger,
            viewport: self.viewport,
            min_width: self.min_width,
            max_height: self.max_height,
            metrics: self.metrics,
            state: &mut *child_state,
            items: &submenu.items,
            prefix: Rc::from(prefix.into_boxed_slice()),
            on_select: self.on_select.clone(),
            open: self.open.clone(),
            class: self.class,
            _renderer: std::marker::PhantomData,
        })))
    }
}

/// The bottom edge of a row, relative to the menu's top.
fn row_bottom_of(top: f32, height: f32) -> f32 {
    top + height
}

/// Draws one selectable row: the selection wash, the leading slot, the
/// label, the keyboard hint, and a submenu's chevron.
#[allow(clippy::too_many_arguments)]
fn draw_selectable_row<Renderer>(
    renderer: &mut Renderer,
    paragraph: &mut paragraph::Plain<Renderer::Paragraph>,
    style: &Style,
    row: Rectangle,
    selected: bool,
    has_leading: bool,
    label: String,
    icon: Option<IconName>,
    checked: Option<bool>,
    shortcut: Option<String>,
    destructive: bool,
    trailing_icon: Option<IconName>,
) where
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    if selected {
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: row.x + 2.0,
                    y: row.y + 1.0,
                    width: row.width - 4.0,
                    height: row.height - 2.0,
                },
                border: iced::Border {
                    radius: 4.0.into(),
                    ..iced::Border::default()
                },
                ..renderer::Quad::default()
            },
            style.hovered_background,
        );
    }

    let center = row.center_y();
    let text_color = if destructive {
        style.destructive_color
    } else {
        style.text_color
    };

    let mut x = row.x + ROW_PADDING;

    if has_leading {
        let slot = Rectangle {
            x,
            y: row.y,
            width: style.icon_size,
            height: row.height,
        };

        if let Some(checked) = checked {
            if checked {
                crate::icons::load();
                draw_glyph(
                    renderer,
                    crate::icons::IconName::Check,
                    slot.center(),
                    style.icon_size,
                    text_color,
                );
            }
        } else if let Some(icon) = icon {
            crate::icons::load();
            draw_glyph(renderer, icon, slot.center(), style.icon_size, text_color);
        }

        x += style.icon_size + CONTENT_GAP;
    }

    // The label takes what is left between the leading slot and the marks at
    // the far edge. The marks are measured, not estimated: the menu's own
    // width came from a measurement, and a guessed reservation here would
    // clip the label the width was chosen to fit.
    let trailing = trailing_icon.is_some();
    let reserved = trailing_area_width(renderer, paragraph, style, shortcut.as_deref(), trailing);
    let label_right = row.x + row.width - ROW_PADDING - reserved;
    let label_bounds = Rectangle {
        x,
        y: row.y,
        width: (label_right - x).max(0.0),
        height: row.height,
    };

    renderer.fill_text(
        adv_text::Text {
            content: label,
            bounds: label_bounds.size(),
            size: Pixels(style.text_size),
            line_height: adv_text::LineHeight::Relative(1.0),
            font: Renderer::Font::DEFAULT,
            align_x: adv_text::Alignment::Default,
            align_y: iced::alignment::Vertical::Center,
            shaping: adv_text::Shaping::Basic,
            wrapping: adv_text::Wrapping::None,
        },
        Point::new(label_bounds.x, center),
        text_color,
        Rectangle::INFINITE,
    );

    if let Some(shortcut) = shortcut {
        let hint_width = reserved
            - if trailing {
                style.icon_size + CONTENT_GAP
            } else {
                0.0
            };
        renderer.fill_text(
            adv_text::Text {
                content: shortcut,
                bounds: Size::new(hint_width.max(0.0), row.height),
                size: Pixels(style.shortcut_size),
                line_height: adv_text::LineHeight::Relative(1.0),
                font: Renderer::Font::DEFAULT,
                align_x: adv_text::Alignment::Right,
                align_y: iced::alignment::Vertical::Center,
                shaping: adv_text::Shaping::Basic,
                wrapping: adv_text::Wrapping::None,
            },
            Point::new(
                row.x + row.width
                    - ROW_PADDING
                    - if trailing {
                        style.icon_size + CONTENT_GAP
                    } else {
                        0.0
                    },
                center,
            ),
            style.muted_color,
            Rectangle::INFINITE,
        );
    }

    if let Some(trailing_icon) = trailing_icon {
        crate::icons::load();
        draw_glyph(
            renderer,
            trailing_icon,
            Point::new(
                row.x + row.width - ROW_PADDING - style.icon_size / 2.0,
                center,
            ),
            style.icon_size,
            style.text_color,
        );
    }
}

/// The width a row keeps clear at its right edge for a keyboard hint and a
/// submenu's chevron.
fn trailing_area_width<Renderer>(
    renderer: &Renderer,
    paragraph: &mut paragraph::Plain<Renderer::Paragraph>,
    style: &Style,
    shortcut: Option<&str>,
    trailing: bool,
) -> f32
where
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    let hint = shortcut.map_or(0.0, |hint| {
        text_width(renderer, paragraph, hint, style.shortcut_size) + CONTENT_GAP
    });
    let chevron = if trailing {
        style.icon_size + CONTENT_GAP
    } else {
        0.0
    };
    hint + chevron
}

/// Draws a non-interactive label row.
fn draw_label<Renderer>(
    renderer: &mut Renderer,
    style: &Style,
    row: Rectangle,
    label: &str,
    color: Color,
) where
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    let x = row.x + ROW_PADDING;

    renderer.fill_text(
        adv_text::Text {
            content: label.to_owned(),
            bounds: Size::new(row.width - ROW_PADDING * 2.0, row.height),
            size: Pixels(style.text_size),
            line_height: adv_text::LineHeight::Relative(1.0),
            font: Renderer::Font::DEFAULT,
            align_x: adv_text::Alignment::Default,
            align_y: iced::alignment::Vertical::Center,
            shaping: adv_text::Shaping::Basic,
            wrapping: adv_text::Wrapping::None,
        },
        Point::new(x, row.center_y()),
        color,
        Rectangle::INFINITE,
    );
}

/// Draws one icon-font glyph centred at a point.
fn draw_glyph<Renderer>(
    renderer: &mut Renderer,
    icon: IconName,
    center: Point,
    size: f32,
    color: Color,
) where
    Renderer: adv_text::Renderer<Font = iced::Font>,
{
    renderer.fill_text(
        adv_text::Text {
            content: crate::icons::glyph(icon).to_string(),
            bounds: Size::new(f32::INFINITY, size),
            size: Pixels(size),
            line_height: adv_text::LineHeight::Relative(1.0),
            font: crate::icons::font(),
            align_x: adv_text::Alignment::Center,
            align_y: iced::alignment::Vertical::Center,
            shaping: adv_text::Shaping::Basic,
            wrapping: adv_text::Wrapping::None,
        },
        center,
        color,
        Rectangle::INFINITE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::renderer::Headless;
    use iced::{Font, Pixels};

    type Items = Vec<Item<()>>;

    /// A headless renderer, so a text width can be measured the way the real
    /// draw path measures it.
    fn headless_renderer() -> iced_test::renderer::Renderer {
        iced_test::futures::futures::executor::block_on(
            <iced_test::renderer::Renderer as Headless>::new(
                Font::with_name("Fira Sans"),
                Pixels(16.0),
                None,
            ),
        )
        .expect("headless renderer")
    }

    fn sample_items() -> Items {
        vec![
            Item::action("Copy", ()),
            Item::separator(),
            Item::separator(),
            Item::command("Zoom"),
            Item::label("Informational"),
            Item::separator(),
            Item::action("Delete", ()).destructive(true),
        ]
    }

    /// Builds a menu level for the geometry tests: the state is borrowed for
    /// the level's lifetime, the class outlives it as `'static`.
    fn level<'a>(
        state: &'a mut State,
        items: &'a Items,
        trigger: Rectangle,
        root: bool,
        class: &'static <crate::theme::Theme as Catalog>::Class<'static>,
    ) -> Overlay<'a, 'static, (), crate::theme::Theme, iced::Renderer> {
        Overlay {
            root,
            trigger,
            viewport: Rectangle::with_size(Size::new(400.0, 400.0)),
            min_width: 160.0,
            max_height: None,
            metrics: Metrics::default(),
            state,
            items,
            prefix: Rc::from(Vec::new().into_boxed_slice()),
            on_select: None,
            open: OpenFlag::closed(),
            class,
            _renderer: std::marker::PhantomData,
        }
    }

    fn class() -> &'static <crate::theme::Theme as Catalog>::Class<'static> {
        // Leaking a style table keeps the test helpers' lifetimes simple: a
        // class is a constant table, and a test builds a handful of them.
        Box::leak(Box::new(<crate::theme::Theme as Catalog>::default()))
    }

    #[test]
    fn edge_and_consecutive_separators_collapse() {
        let items = sample_items();
        let rows = effective_rows(&items);

        // The run of two after `Copy` collapses to one rule, and the trailing
        // one after `Delete` goes: a rule divides groups, and there is no
        // group on one side.
        assert_eq!(rows, vec![0, 1, 3, 4, 5, 6]);
    }

    #[test]
    fn a_lone_separator_collapses_entirely() {
        let items: Items = vec![Item::separator()];
        assert!(effective_rows(&items).is_empty());
    }

    #[test]
    fn content_height_sums_effective_rows_only() {
        let items = sample_items();
        let rows = effective_rows(&items);
        let expected: f32 =
            rows.iter().map(|ix| row_height(&items[*ix])).sum::<f32>() + MENU_PADDING * 2.0;

        assert!((content_height(&items) - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn disabled_rows_are_not_selectable() {
        let items: Items = vec![
            Item::action("Off", ()).enabled(false),
            Item::submenu("Closed", Vec::new()).enabled(false),
            Item::action("On", ()),
        ];

        assert!(!items[0].is_selectable());
        assert!(!items[1].is_selectable());
        assert!(items[2].is_selectable());
    }

    #[test]
    fn an_open_flag_round_trips() {
        let open = OpenFlag::closed();
        assert!(!open.is_open());

        open.open();
        assert!(open.is_open());

        assert!(!open.toggle(), "toggling an open menu closes it");
        assert!(open.toggle(), "toggling a closed menu opens it");
    }

    #[test]
    fn row_offset_counts_only_the_rows_before_it() {
        let items = sample_items();

        // `Zoom` sits one rule behind `Copy`: the run of two separators
        // counts once.
        let copy = row_offset(&items, 0);
        let zoom = row_offset(&items, 3);
        assert!((zoom - copy - (ROW_HEIGHT + SEPARATOR_HEIGHT)).abs() < f32::EPSILON);
    }

    #[test]
    fn the_horizontal_origin_flips_left_near_the_right_edge() {
        let items = sample_items();

        // A trigger with room to its right opens left-aligned to it.
        let mut state = State::new();
        let roomy = level(
            &mut state,
            &items,
            Rectangle::new(Point::new(40.0, 40.0), Size::new(20.0, 20.0)),
            true,
            class(),
        );
        assert_eq!(roomy.horizontal_origin(160.0, 400.0), 40.0);

        // A trigger near the right edge opens inward, from its right edge:
        // this is what keeps the rightmost dock panel's menu on the glass.
        let mut state = State::new();
        let cramped = level(
            &mut state,
            &items,
            Rectangle::new(Point::new(380.0, 40.0), Size::new(20.0, 20.0)),
            true,
            class(),
        );
        let x = cramped.horizontal_origin(160.0, 400.0);
        assert!((x - (380.0 + 20.0 - 160.0)).abs() < f32::EPSILON);

        // A submenu opens beside its row on the right when there is room.
        let mut state = State::new();
        let row = Rectangle::new(Point::new(100.0, 100.0), Size::new(160.0, 28.0));
        let submenu = level(&mut state, &items, row, false, class());
        let to_the_right = submenu.horizontal_origin(160.0, 600.0);
        assert!((to_the_right - (100.0 + 160.0 - SUBMENU_OVERLAP)).abs() < f32::EPSILON);

        // And beside it on the left when the right side has none.
        let mut state = State::new();
        let row = Rectangle::new(Point::new(350.0, 100.0), Size::new(160.0, 28.0));
        let submenu = level(&mut state, &items, row, false, class());
        let to_the_left = submenu.horizontal_origin(160.0, 600.0);
        assert!((to_the_left - (350.0 - 160.0 + SUBMENU_OVERLAP)).abs() < f32::EPSILON);
    }

    #[test]
    fn the_vertical_placement_prefers_below_then_above() {
        let items: Items = vec![
            Item::action("One", ()),
            Item::action("Two", ()),
            Item::action("Three", ()),
        ];
        let height = content_height(&items);
        assert!((height - (ROW_HEIGHT * 3.0 + MENU_PADDING * 2.0)).abs() < f32::EPSILON);

        // A trigger in the upper half opens downward, from its bottom edge.
        let mut state = State::new();
        let top = level(
            &mut state,
            &items,
            Rectangle::new(Point::new(40.0, 30.0), Size::new(20.0, 20.0)),
            true,
            class(),
        );
        let (y, capped) = top.vertical_placement(height, 400.0);
        assert!((y - 50.0).abs() < f32::EPSILON);
        assert!((capped - height).abs() < f32::EPSILON);

        // A trigger in the lower half opens upward: the menu rises above the
        // trigger instead of running off the bottom of the window.
        let mut state = State::new();
        let bottom = level(
            &mut state,
            &items,
            Rectangle::new(Point::new(40.0, 370.0), Size::new(20.0, 20.0)),
            true,
            class(),
        );
        let (y, capped) = bottom.vertical_placement(height, 400.0);
        assert!(y + capped <= 370.0 + f32::EPSILON);
        assert!((capped - height).abs() < f32::EPSILON);
    }

    #[test]
    fn a_submenu_opens_with_its_row_and_folds_off_it() {
        let items: Items = vec![
            Item::submenu("Export", vec![Item::action("As PDF", ())]),
            Item::action("Close", ()),
        ];

        let mut state = State::new();
        let mut menu = level(
            &mut state,
            &items,
            Rectangle::new(Point::new(40.0, 40.0), Size::new(20.0, 20.0)),
            true,
            class(),
        );

        menu.state.selected = Some(0);
        menu.reconcile_submenu();
        assert!(menu.state.has_submenu(), "selecting the row opens it");

        // Staying on the row keeps the submenu.
        menu.reconcile_submenu();
        assert!(menu.state.has_submenu());

        // Moving to a plain row folds it.
        menu.state.selected = Some(1);
        menu.reconcile_submenu();
        assert!(!menu.state.has_submenu());
    }

    #[test]
    fn keyboard_moves_skip_rows_that_cannot_answer() {
        let items: Items = vec![
            Item::label("Header"),
            Item::action("Off", ()).enabled(false),
            Item::separator(),
            Item::action("On", ()),
        ];

        let mut state = State::new();
        let mut menu = level(
            &mut state,
            &items,
            Rectangle::new(Point::new(40.0, 40.0), Size::new(20.0, 20.0)),
            true,
            class(),
        );

        let bounds = Rectangle::new(Point::ORIGIN, Size::new(160.0, 100.0));
        menu.move_selection(1, bounds);
        assert_eq!(
            menu.state.selected,
            Some(3),
            "down lands on the first row that answers"
        );

        // A lone answerable row keeps the selection no matter the steps.
        menu.move_selection(1, bounds);
        assert_eq!(menu.state.selected, Some(3));
    }

    #[test]
    fn scroll_follows_a_keyboard_move_past_the_fold() {
        // Ten rows stand 280px tall; the view shows two, so the last rows
        // need scrolling to be seen.
        let items: Items = (0..10)
            .map(|ix| Item::action(format!("Row {ix}"), ()))
            .collect();

        let mut state = State::new();
        let view = Size::new(160.0, MENU_PADDING * 2.0 + ROW_HEIGHT * 2.0);
        let mut menu = level(
            &mut state,
            &items,
            Rectangle::new(Point::new(40.0, 40.0), Size::new(20.0, 20.0)),
            true,
            class(),
        );

        let bounds = Rectangle::with_size(view);
        for _ in 0..10 {
            menu.move_selection(1, bounds);
        }
        assert_eq!(menu.state.selected, Some(9));

        let last_top = row_offset(&items, 9);
        let visible_top = menu.state.scroll;
        let visible_bottom = menu.state.scroll + view.height - MENU_PADDING * 2.0;
        assert!(
            last_top >= visible_top && last_top < visible_bottom,
            "the selected row scrolled into view"
        );
    }

    #[test]
    fn a_state_resets_to_fresh() {
        let items: Items = vec![Item::submenu("Export", vec![Item::action("As PDF", ())])];

        let mut state = State::new();
        let mut menu = level(
            &mut state,
            &items,
            Rectangle::new(Point::new(40.0, 40.0), Size::new(20.0, 20.0)),
            true,
            class(),
        );
        menu.state.selected = Some(0);
        menu.state.scroll = 40.0;
        menu.reconcile_submenu();
        assert!(menu.state.has_submenu());

        menu.state.reset();
        assert!(!menu.state.has_submenu());
        assert_eq!(menu.state.selected, None);
        assert_eq!(menu.state.scroll, 0.0);
    }

    #[test]
    fn a_menu_is_as_wide_as_its_longest_row_needs() {
        let renderer = headless_renderer();
        let metrics = Metrics::default();
        let short: Items = vec![Item::action("Cut", ()), Item::action("Copy", ())];
        let long: Items = vec![
            Item::action("Copy the whole project's path to the clipboard", ()),
            Item::action("Close the project", ()),
        ];

        let short_width = measured_width(&renderer, &short, metrics);
        let long_width = measured_width(&renderer, &long, metrics);

        assert!(
            long_width > short_width,
            "a menu with longer labels should be wider: {long_width} against {short_width}"
        );
        assert!(
            (short_width - MIN_WIDTH).abs() < 0.5,
            "a menu of two short words should sit at the floor: {short_width}"
        );

        // A long label is what sets the width, so the menu's own body has to
        // be at least as wide as that label plus the row's padding and the
        // body's own inset — that is what keeps it on one line.
        let mut paragraph = paragraph::Plain::default();
        let label = text_width(
            &renderer,
            &mut paragraph,
            "Copy the whole project's path to the clipboard",
            metrics.text_size,
        );
        assert!(
            long_width >= label + ROW_PADDING * 2.0 + MENU_PADDING * 2.0,
            "the menu ({long_width}) should fit its longest label ({label})              with the row's and the body's padding"
        );
    }

    #[test]
    fn a_shortcut_does_not_squeeze_the_label_out() {
        let renderer = headless_renderer();
        let metrics = Metrics::default();

        // The dock's menu, which is what wrapped: a long label and a hint.
        let items: Items = vec![
            Item::action("Copy Files path", ()).shortcut("Ctrl+C"),
            Item::action("Close Files", ()),
        ];

        let width = measured_width(&renderer, &items, metrics);

        let mut paragraph = paragraph::Plain::default();
        let label = text_width(
            &renderer,
            &mut paragraph,
            "Copy Files path",
            metrics.text_size,
        );
        let hint = text_width(&renderer, &mut paragraph, "Ctrl+C", metrics.shortcut_size);

        // A row with a shortcut has to fit its label, the hint, the gap
        // between them and the row's own padding. Neither row here carries an
        // icon, so no icon column is reserved.
        let needed = label + hint + CONTENT_GAP + ROW_PADDING * 2.0 + MENU_PADDING * 2.0;

        assert!(
            width + 0.5 >= needed,
            "the menu should fit the label and its hint: {width} against {needed}"
        );
    }

    #[test]
    fn a_menu_never_exceeds_the_maximum_width() {
        let renderer = headless_renderer();
        let metrics = Metrics::default();

        // A label far longer than any menu should be.
        let items: Items = vec![Item::action("x".repeat(400), ())];

        let width = measured_width(&renderer, &items, metrics);
        assert!(
            (width - MAX_WIDTH).abs() < 0.5,
            "a very long label should be capped at {MAX_WIDTH}: it came out {width}"
        );
    }

    #[test]
    fn a_menu_never_shrinks_below_the_minimum_width() {
        let renderer = headless_renderer();
        let metrics = Metrics::default();

        let items: Items = vec![Item::action("Cut", ())];

        let width = measured_width(&renderer, &items, metrics);
        assert!(
            (width - MIN_WIDTH).abs() < 0.5,
            "a one-word menu should be floored at {MIN_WIDTH}: it came out {width}"
        );
    }

    #[test]
    fn the_overlay_lays_out_wide_enough_for_its_labels() {
        // The bug a screenshot showed: a menu anchored to the dock's `...`
        // button came out as wide as the button — about 22px — so its labels
        // wrapped across three lines. The width the overlay resolves is what
        // decides that, and a label is only kept on one line by drawing it
        // into a box at least as wide as it measures.
        let renderer = headless_renderer();
        let metrics = Metrics::default();

        let items: Items = vec![
            Item::action("Copy Problems path", ()).shortcut("Ctrl+C"),
            Item::action("Close Problems path", ()),
        ];

        // A button at the right end of a bar, as the dock's control is.
        let trigger = Rectangle::new(Point::new(1160.0, 40.0), Size::new(22.0, 22.0));
        let viewport = Rectangle::with_size(Size::new(1180.0, 760.0));

        let mut state = State::new();
        let open = OpenFlag::closed();
        let class = class();

        let menu: Menu<'_, '_, (), crate::theme::Theme> =
            Menu::new(&mut state, &items, None, open, class).metrics(metrics);

        let mut overlay = menu.overlay::<iced::Renderer>(trigger, viewport);
        let node = overlay
            .as_overlay_mut()
            .layout(&renderer, Size::new(1180.0, 760.0));
        let bounds = node.bounds();

        // The widest label, drawn at the row's own size.
        let mut paragraph = paragraph::Plain::default();
        let label = text_width(
            &renderer,
            &mut paragraph,
            "Copy Problems path",
            metrics.text_size,
        );

        assert!(
            bounds.width >= label + ROW_PADDING * 2.0 + MENU_PADDING * 2.0,
            "the menu should be wide enough for its longest label:              {} against a label of {label}",
            bounds.width
        );

        // And it must stay on the glass: the trigger is inside the last 20
        // pixels of the window, so a menu opened from it has to flip inward.
        assert!(
            bounds.x >= 0.0 && bounds.x + bounds.width <= viewport.width,
            "the menu should stay inside the window: it spans {}..{} of {}",
            bounds.x,
            bounds.x + bounds.width,
            viewport.width
        );

        // The width is the label's, not the button's.
        assert!(
            bounds.width > trigger.width * 3.0,
            "the menu should be far wider than the button that opened it:              {} against {}",
            bounds.width,
            trigger.width
        );
    }

    /// The defect a screenshot showed: moving the pointer down a menu left
    /// the highlight behind, so the pointer had travelled a whole menu's
    /// height before the highlight caught up.
    ///
    /// The hover path fed `row_at` a position already made relative to the
    /// menu, and `row_at` subtracts the menu's top itself — so the menu's
    /// height was counted twice. This drives the real `update` to pin the
    /// contract: the row under the pointer is the row that gets selected.
    #[test]
    fn hovering_a_row_selects_the_row_the_pointer_is_over() {
        let renderer = headless_renderer();
        let items: Items = (0..5)
            .map(|ix| Item::action(format!("Row {ix}"), ()))
            .collect();

        let trigger = Rectangle::new(Point::new(100.0, 100.0), Size::new(20.0, 20.0));
        let viewport = Rectangle::with_size(Size::new(400.0, 400.0));

        for ix in 0..5 {
            let mut state = State::new();
            let open = OpenFlag::closed();
            let class = class();

            let menu: Menu<'_, '_, (), crate::theme::Theme> =
                Menu::new(&mut state, &items, None, open, class);

            // The menu's own frame, from the layout the runtime would give it.
            let mut element = menu.overlay::<iced::Renderer>(trigger, viewport);
            let node = element
                .as_overlay_mut()
                .layout(&renderer, Size::new(400.0, 400.0));
            let menu_bounds = node.bounds();

            // A window-space point inside the row, taken from the menu's top
            // edge and the row heights — not from `row_bounds`, so the test
            // does not assume the arithmetic it is checking.
            let point = Point::new(
                menu_bounds.x + menu_bounds.width / 2.0,
                menu_bounds.y + MENU_PADDING + ROW_HEIGHT * (ix as f32 + 0.5),
            );

            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            element.as_overlay_mut().update(
                &Event::Mouse(mouse::Event::CursorMoved { position: point }),
                Layout::new(&node),
                Cursor::Available(point),
                &renderer,
                &mut iced::advanced::clipboard::Null,
                &mut shell,
            );

            // The overlay borrows the state, so it goes before the state is
            // read back.
            drop(element);

            assert_eq!(
                state.selected(),
                Some(ix),
                "the pointer at y={} should select row {ix}",
                point.y
            );
        }
    }
}
