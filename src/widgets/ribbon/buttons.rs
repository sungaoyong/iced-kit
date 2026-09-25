//! Rendering of individual ribbon items into elements.
//!
//! Each [`RibbonItem`] variant maps to one button face. The faces share
//! [`tool_style`](super::style::tool_style) so a hover, a press, and a selected
//! ring look the same whether the button is a large tool or a small dropdown
//! arrow. Icons go through [`Icon::into_element`], which inherits the button's
//! text color — the same rule that keeps an icon and its label matching in the
//! rest of the library.

use super::model::{RibbonGroup, RibbonItem, RibbonTool};
use super::style::{
    group_label, tool_style, BUTTON_GAP, GROUP_PADDING, LARGE_ICON, LARGE_W, ROW_H, SMALL_ICON,
    SMALL_W, TOOL_BAR_H,
};
use crate::icons::IconName;
use crate::theme::{Size, Theme};
use crate::widgets::button::Icon;
use crate::widgets::overlay::{tooltip_at, trigger, TooltipPosition};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length, Padding, Rectangle};
use std::rc::Rc;

/// The dropdown-and-toggle context a small or large dropdown needs to draw its
/// open state, emit its toggle, and report the bounds of the button that owns
/// it.
pub(crate) struct DropdownCtx<'a, 'i, Message> {
    pub(crate) open: &'i Option<String>,
    pub(crate) on_toggle: Option<&'i (dyn Fn(String) -> Message + 'a)>,
    /// Reports a pressed dropdown button's window bounds, so the application can
    /// anchor that button's hosted panel to it rather than to the whole ribbon.
    /// An [`Rc`] so every dropdown on the tab can share one callback.
    pub(crate) on_anchor: Option<&'i Rc<dyn Fn(Rectangle) -> Message + 'a>>,
}

impl<'a, Message> DropdownCtx<'a, '_, Message> {
    /// Wraps a dropdown button so a press on it first reports its own bounds
    /// (through [`trigger`], ahead of the button's toggle message), letting the
    /// application anchor the panel to the button rather than the whole ribbon.
    /// Without an anchor callback the button is returned untouched.
    fn anchored<ElementLike>(&self, btn: ElementLike) -> Element<'a, Message, Theme>
    where
        Message: Clone + 'a,
        ElementLike: Into<Element<'a, Message, Theme>>,
    {
        match self.on_anchor {
            Some(rc) => {
                let rc = Rc::clone(rc);
                trigger(btn, move |bounds| rc(bounds)).into()
            }
            None => btn.into(),
        }
    }
}

/// Turns one ribbon item into its element.
pub(crate) fn render_item<'a, Message: Clone + 'a>(
    item: &RibbonItem<Message>,
    ctx: &DropdownCtx<'a, '_, Message>,
) -> Element<'a, Message, Theme> {
    match item {
        RibbonItem::Tool(tool) => tool_button(tool, Face::SmallIcon),
        RibbonItem::LabeledTool(tool) => tool_button(tool, Face::SmallLabeled),
        RibbonItem::LargeTool(tool) => tool_button(tool, Face::Large),
        RibbonItem::Dropdown { id, icon, .. } => small_dropdown(id, icon, None, ctx),
        RibbonItem::LabeledDropdown {
            id, icon, label, ..
        } => small_dropdown(id, icon, label.as_deref(), ctx),
        RibbonItem::LargeDropdown {
            id, icon, label, ..
        } => large_dropdown(id, icon, label.as_deref(), ctx),
        RibbonItem::ToolGrid { columns } => tool_grid(columns),
    }
}

/// A group at *full* density: its items packed into columns — a large item owns
/// a whole-height column, small items stack three to a column — over its bottom
/// label.
pub(crate) fn render_group<'a, Message: Clone + 'a>(
    group: &RibbonGroup<Message>,
    ctx: &DropdownCtx<'a, '_, Message>,
) -> Element<'a, Message, Theme> {
    let mut columns: Vec<Element<'a, Message, Theme>> = Vec::new();
    let mut small: Vec<Element<'a, Message, Theme>> = Vec::new();

    for item in &group.items {
        if item.is_large() {
            flush_small(&mut small, &mut columns);
            columns.push(render_item(item, ctx));
        } else {
            small.push(render_item(item, ctx));
            if small.len() == 3 {
                flush_small(&mut small, &mut columns);
            }
        }
    }
    flush_small(&mut small, &mut columns);

    let mut tools = row![]
        .spacing(BUTTON_GAP)
        .height(Length::Fill)
        .align_y(Alignment::Start);
    for column in columns {
        tools = tools.push(column);
    }

    column![tools, group_label(&group.title)]
        .align_x(Alignment::Center)
        .spacing(2)
        .padding(GROUP_PADDING)
        .height(Length::Fixed(TOOL_BAR_H))
        .into()
}

/// Wraps the small items gathered so far into one column and appends it.
fn flush_small<'a, Message: 'a>(
    small: &mut Vec<Element<'a, Message, Theme>>,
    columns: &mut Vec<Element<'a, Message, Theme>>,
) {
    if small.is_empty() {
        return;
    }
    let column = small
        .drain(..)
        .fold(column![].spacing(BUTTON_GAP), |column, item| {
            column.push(item)
        });
    columns.push(column.into());
}

/// A group at *compact* density: the same packing as [`render_group`], but every
/// item forced to its small, icon-only face so the group narrows. It keeps its
/// bottom label, and its full three-row height.
pub(crate) fn render_group_compact<'a, Message: Clone + 'a>(
    group: &RibbonGroup<Message>,
    ctx: &DropdownCtx<'a, '_, Message>,
) -> Element<'a, Message, Theme> {
    let mut columns: Vec<Element<'a, Message, Theme>> = Vec::new();
    let mut small: Vec<Element<'a, Message, Theme>> = Vec::new();
    for item in &group.items {
        small.push(render_item_compact(item, ctx));
        if small.len() == 3 {
            flush_small(&mut small, &mut columns);
        }
    }
    flush_small(&mut small, &mut columns);

    let mut tools = row![]
        .spacing(BUTTON_GAP)
        .height(Length::Fill)
        .align_y(Alignment::Start);
    for column in columns {
        tools = tools.push(column);
    }
    column![tools, group_label(&group.title)]
        .align_x(Alignment::Center)
        .spacing(2)
        .padding(GROUP_PADDING)
        .height(Length::Fixed(TOOL_BAR_H))
        .into()
}

/// Renders one item as a small, icon-only button — the face a compact group
/// uses for every item regardless of its full-size footprint.
fn render_item_compact<'a, Message: Clone + 'a>(
    item: &RibbonItem<Message>,
    ctx: &DropdownCtx<'a, '_, Message>,
) -> Element<'a, Message, Theme> {
    match item {
        RibbonItem::Tool(tool) | RibbonItem::LabeledTool(tool) | RibbonItem::LargeTool(tool) => {
            tool_button(tool, Face::SmallIcon)
        }
        RibbonItem::Dropdown { id, icon, .. }
        | RibbonItem::LabeledDropdown { id, icon, .. }
        | RibbonItem::LargeDropdown { id, icon, .. } => small_dropdown(id, icon, None, ctx),
        RibbonItem::ToolGrid { columns } => tool_grid(columns),
    }
}

/// A group at *collapsed* density: a single button bearing the group's title.
/// Pressing it reports the title as a dropdown id, so the application hosts the
/// group's tools in a panel — the same contract as any other ribbon dropdown.
pub(crate) fn render_group_collapsed<'a, Message: Clone + 'a>(
    group: &RibbonGroup<Message>,
    ctx: &DropdownCtx<'a, '_, Message>,
) -> Element<'a, Message, Theme> {
    let id = group.title.clone();
    let is_open = ctx.open.as_deref() == Some(id.as_str());
    let mut btn = button(text(group.title.clone()).size(13.0))
        .class(tool_style(is_open))
        .height(Length::Fixed(ROW_H))
        .padding(Padding::from([1.0, 8.0]));
    if let Some(toggle) = ctx.on_toggle {
        btn = btn.on_press(toggle(id));
    }
    ctx.anchored(btn)
}

/// A group at *tight* density: a small, icon-only button using the group's first
/// icon, toggling the same dropdown as [`render_group_collapsed`].
pub(crate) fn render_group_tight<'a, Message: Clone + 'a>(
    group: &RibbonGroup<Message>,
    ctx: &DropdownCtx<'a, '_, Message>,
) -> Element<'a, Message, Theme> {
    let id = group.title.clone();
    let is_open = ctx.open.as_deref() == Some(id.as_str());
    let icon = first_icon(group).unwrap_or_else(|| Icon::new(IconName::Menu));
    let icon_el = icon.size(SMALL_ICON).into_element::<Message>(Size::Sm);
    let mut btn = button(icon_el)
        .class(tool_style(is_open))
        .width(Length::Fixed(SMALL_W))
        .height(Length::Fixed(ROW_H))
        .padding(Padding::from(0.0));
    if let Some(toggle) = ctx.on_toggle {
        btn = btn.on_press(toggle(id));
    }
    ctx.anchored(btn)
}

/// The first icon a group shows, used as the tight button's face. Falls back to
/// `None` for an empty group.
fn first_icon<Message>(group: &RibbonGroup<Message>) -> Option<Icon> {
    group.items.iter().find_map(|item| match item {
        RibbonItem::Tool(tool) | RibbonItem::LabeledTool(tool) | RibbonItem::LargeTool(tool) => {
            Some(tool.icon.clone())
        }
        RibbonItem::Dropdown { icon, .. }
        | RibbonItem::LabeledDropdown { icon, .. }
        | RibbonItem::LargeDropdown { icon, .. } => Some(icon.clone()),
        RibbonItem::ToolGrid { columns } => columns
            .iter()
            .flatten()
            .next()
            .map(|tool| tool.icon.clone()),
    })
}

/// The three faces a plain tool button can take.
#[derive(Clone, Copy)]
enum Face {
    /// A square, icon-only button.
    SmallIcon,
    /// A single-row button with its label beside the icon.
    SmallLabeled,
    /// A full-height button with its label under the icon.
    Large,
}

/// Builds a plain tool button in one of the [`Face`] shapes.
fn tool_button<'a, Message: Clone + 'a>(
    tool: &RibbonTool<Message>,
    face: Face,
) -> Element<'a, Message, Theme> {
    let (icon_side, control) = match face {
        Face::Large => (LARGE_ICON, Size::Lg),
        Face::SmallIcon | Face::SmallLabeled => (SMALL_ICON, Size::Sm),
    };
    let icon = tool
        .icon
        .clone()
        .size(icon_side)
        .into_element::<Message>(control);

    let content: Element<'a, Message, Theme> = match face {
        Face::Large => {
            let mut col = column![icon].align_x(Alignment::Center).spacing(3);
            if let Some(label) = &tool.label {
                col = col.push(large_label(label));
            }
            col.into()
        }
        Face::SmallLabeled => {
            let mut r = row![icon].spacing(4).align_y(Alignment::Center);
            if let Some(label) = &tool.label {
                r = r.push(small_label(label));
            }
            r.into()
        }
        Face::SmallIcon => icon,
    };

    let mut btn = button(content).class(tool_style(tool.selected));
    btn = match face {
        Face::Large => btn
            .width(Length::Fixed(LARGE_W))
            .height(Length::Fixed(TOOL_BAR_H - 21.0))
            .padding(Padding::from(2.0)),
        Face::SmallIcon => btn
            .width(Length::Fixed(SMALL_W))
            .height(Length::Fixed(ROW_H))
            .padding(Padding::from(0.0)),
        Face::SmallLabeled => btn
            .height(Length::Fixed(ROW_H))
            .padding(Padding::from([1.0, 4.0])),
    };

    if let Some(message) = tool.on_press.clone() {
        btn = btn.on_press(message);
    }

    wrap_tooltip(btn.into(), tool)
}

/// A small dropdown: an icon (and optional label) with a trailing ▾, all in one
/// button that toggles the panel named `id`.
fn small_dropdown<'a, Message: Clone + 'a>(
    id: &str,
    icon: &Icon,
    label: Option<&str>,
    ctx: &DropdownCtx<'a, '_, Message>,
) -> Element<'a, Message, Theme> {
    let is_open = ctx.open.as_deref() == Some(id);
    let icon_el = icon
        .clone()
        .size(SMALL_ICON)
        .into_element::<Message>(Size::Sm);
    let arrow = caret::<Message>(10.0);

    let mut r = row![icon_el].spacing(4).align_y(Alignment::Center);
    if let Some(label) = label {
        r = r.push(small_label(label));
    }
    r = r.push(arrow);

    let mut btn = button(r)
        .class(tool_style(is_open))
        .height(Length::Fixed(ROW_H))
        .padding(Padding::from([1.0, 3.0]));
    if let Some(toggle) = ctx.on_toggle {
        btn = btn.on_press(toggle(id.to_owned()));
    }
    ctx.anchored(btn)
}

/// A large dropdown: a full-height icon-over-label button with a ▾ beneath.
fn large_dropdown<'a, Message: Clone + 'a>(
    id: &str,
    icon: &Icon,
    label: Option<&str>,
    ctx: &DropdownCtx<'a, '_, Message>,
) -> Element<'a, Message, Theme> {
    let is_open = ctx.open.as_deref() == Some(id);
    let icon_el = icon
        .clone()
        .size(LARGE_ICON)
        .into_element::<Message>(Size::Lg);

    let mut col = column![icon_el].align_x(Alignment::Center).spacing(3);
    if let Some(label) = label {
        col = col.push(large_label(label));
    }
    col = col.push(caret::<Message>(9.0));

    let mut btn = button(col)
        .class(tool_style(is_open))
        .width(Length::Fixed(LARGE_W))
        .height(Length::Fixed(TOOL_BAR_H - 21.0))
        .padding(Padding::from(2.0));
    if let Some(toggle) = ctx.on_toggle {
        btn = btn.on_press(toggle(id.to_owned()));
    }
    ctx.anchored(btn)
}

/// A grid of small icon-only buttons, laid out in the caller's columns.
fn tool_grid<'a, Message: Clone + 'a>(
    columns: &[Vec<RibbonTool<Message>>],
) -> Element<'a, Message, Theme> {
    columns
        .iter()
        .fold(row![].spacing(BUTTON_GAP), |grid, column_tools| {
            let col = column_tools
                .iter()
                .fold(column![].spacing(BUTTON_GAP), |col, tool| {
                    col.push(tool_button(tool, Face::SmallIcon))
                });
            grid.push(col)
        })
        .into()
}

/// The small caret glyph used on a dropdown's ▾ strip.
fn caret<'a, Message: 'a>(side: f32) -> Element<'a, Message, Theme> {
    Icon::new(IconName::ChevronDown)
        .size(side)
        .into_element::<Message>(Size::Xs)
}

/// A large button's label: small text, allowed to wrap to two lines.
fn large_label<'a, Message: 'a>(label: &str) -> Element<'a, Message, Theme> {
    text(label.to_string())
        .size(11.0)
        .width(Length::Fixed(LARGE_W - 4.0))
        .align_x(Alignment::Center)
        .into()
}

/// A single-row button's label.
fn small_label<'a, Message: 'a>(label: &str) -> Element<'a, Message, Theme> {
    text(label.to_string()).size(13.0).into()
}

/// Attaches a tooltip, if the tool carries one.
fn wrap_tooltip<'a, Message: 'a>(
    element: Element<'a, Message, Theme>,
    tool: &RibbonTool<Message>,
) -> Element<'a, Message, Theme> {
    match &tool.tooltip {
        Some(tip) => tooltip_at(element, tip.clone(), TooltipPosition::Bottom),
        None => element,
    }
}
