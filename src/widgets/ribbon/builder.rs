//! The ribbon builder and its view assembly.
//!
//! [`Ribbon`] turns a [`Vec`](super::model::RibbonTab) of tabs into the two
//! bands a ribbon is made of: a [`tabs`](crate::widgets::tabs) strip along the
//! top, and the active tab's groups laid out left to right beneath it. Each
//! group is built at all four densities and handed to
//! [`CollapseGroups`](super::collapse::CollapseGroups), which picks how far to
//! degrade them against the width it is offered — see [`collapse`](super::collapse).

use super::buttons::{
    render_group, render_group_collapsed, render_group_compact, render_group_tight, DropdownCtx,
};
use super::collapse::{CollapseGroups, CollapseMode, GroupSlots};
use super::model::{RibbonState, RibbonTab};
use super::style::band_style;
use crate::theme::Theme;
use crate::widgets::{tabs as tab_strip, Tab, TabVariant};
use iced::widget::{column, container};
use iced::{Element, Length, Rectangle};
use std::rc::Rc;

/// Builds a [`Ribbon`] with no tabs yet.
///
/// ```
/// # use iced_kit::widgets::ribbon::{Ribbon, RibbonGroup, RibbonTab, RibbonItem, RibbonTool, RibbonState};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Selected(usize) }
/// # fn view(state: &RibbonState) -> Element<'static, Message, Theme> {
/// Ribbon::new()
///     .tab(RibbonTab::new("Home").group(
///         RibbonGroup::new("Draw")
///             .item(RibbonItem::large(RibbonTool::new("／").label("Line"))),
///     ))
///     .state(state)
///     .on_select(Message::Selected)
///     .into()
/// # }
/// ```
#[must_use = "a Ribbon does nothing unless it is turned into an Element"]
pub struct Ribbon<'a, Message> {
    tabs: Vec<RibbonTab<Message>>,
    active: usize,
    open_dropdown: Option<String>,
    collapse_mode: CollapseMode,
    on_select: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    on_dropdown_toggle: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_dropdown_anchor: Option<Rc<dyn Fn(Rectangle) -> Message + 'a>>,
    width: Length,
}

impl<'a, Message: Clone + 'a> Ribbon<'a, Message> {
    /// Creates an empty ribbon.
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
            open_dropdown: None,
            collapse_mode: CollapseMode::default(),
            on_select: None,
            on_dropdown_toggle: None,
            on_dropdown_anchor: None,
            width: Length::Fill,
        }
    }

    /// Appends a tab.
    pub fn tab(mut self, tab: RibbonTab<Message>) -> Self {
        self.tabs.push(tab);
        self
    }

    /// Appends several tabs.
    pub fn tabs(mut self, tabs: impl IntoIterator<Item = RibbonTab<Message>>) -> Self {
        self.tabs.extend(tabs);
        self
    }

    /// Reads the active tab, the open dropdown, and the density mode from a
    /// [`RibbonState`].
    pub fn state(mut self, state: &RibbonState) -> Self {
        self.active = state.active;
        self.open_dropdown.clone_from(&state.open_dropdown);
        self.collapse_mode = state.collapse_mode;
        self
    }

    /// Sets the active tab directly, overriding any [`RibbonState`].
    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
        self
    }

    /// Sets the message sent when a tab is picked, receiving its index.
    pub fn on_select(self, on_select: impl Fn(usize) -> Message + 'a) -> Self {
        Self {
            on_select: Some(Box::new(on_select)),
            ..self
        }
    }

    /// Sets the message sent when a dropdown's ▾ is pressed, receiving its id.
    ///
    /// The ribbon draws no floating panel of its own — iced has no window-level
    /// z-order — so the application hosts the open panel through
    /// [`Layer`](crate::widgets::overlay::Layer), exactly as it does for the
    /// combobox and date-picker panels.
    pub fn on_dropdown_toggle(self, on_dropdown_toggle: impl Fn(String) -> Message + 'a) -> Self {
        Self {
            on_dropdown_toggle: Some(Box::new(on_dropdown_toggle)),
            ..self
        }
    }

    /// Sets the message sent when a dropdown button is pressed, receiving that
    /// button's window bounds — reported by a [`trigger`](crate::widgets::overlay::trigger)
    /// wrapped around each dropdown, ahead of its toggle.
    ///
    /// Pair it with [`on_dropdown_toggle`](Self::on_dropdown_toggle): record the
    /// rectangle here and anchor the panel you host for the id to it, so each
    /// ▾ opens beneath its own button rather than at one fixed corner of the
    /// ribbon.
    pub fn on_dropdown_anchor(
        self,
        on_dropdown_anchor: impl Fn(Rectangle) -> Message + 'a,
    ) -> Self {
        Self {
            on_dropdown_anchor: Some(Rc::new(on_dropdown_anchor)),
            ..self
        }
    }

    /// Sets the ribbon's width. It defaults to [`Length::Fill`].
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets how the groups are sized when the row runs out of width. It defaults
    /// to [`CollapseMode::Auto`], which degrades groups from the right; the
    /// other modes pin every group to one density.
    pub fn collapse_mode(mut self, mode: CollapseMode) -> Self {
        self.collapse_mode = mode;
        self
    }

    /// Turns the ribbon into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            tabs,
            active,
            open_dropdown,
            collapse_mode,
            on_select,
            on_dropdown_toggle,
            on_dropdown_anchor,
            width,
        } = self;

        let strip = on_select.map(|on_select| {
            let labels = tabs.iter().map(|tab| Tab::new(tab.title.clone())).collect();
            tab_strip(labels, active, on_select)
                .variant(TabVariant::Tab)
                .into_element()
        });

        let groups = tabs
            .get(active)
            .map_or(&[][..], |tab| tab.groups.as_slice());
        let toggle = on_dropdown_toggle.as_ref().map(Box::as_ref);
        let ctx = DropdownCtx {
            open: &open_dropdown,
            on_toggle: toggle,
            on_anchor: on_dropdown_anchor.as_ref(),
        };

        // Build each group once per density; `CollapseGroups` measures them and
        // chooses how far to degrade against the width it is offered.
        let slots: Vec<GroupSlots<'a, Message>> = groups
            .iter()
            .map(|group| {
                [
                    render_group(group, &ctx),
                    render_group_compact(group, &ctx),
                    render_group_collapsed(group, &ctx),
                    render_group_tight(group, &ctx),
                ]
            })
            .collect();

        let band = container(CollapseGroups::new(slots, collapse_mode))
            .width(width)
            .class(band_style());

        match strip {
            Some(strip) => column![strip, band].into(),
            None => band.into(),
        }
    }
}

impl<'a, Message: Clone + 'a> Default for Ribbon<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<Ribbon<'a, Message>> for Element<'a, Message, Theme> {
    fn from(ribbon: Ribbon<'a, Message>) -> Self {
        ribbon.into_element()
    }
}

/// Builds a [`Ribbon`] with no tabs yet.
///
/// See [`Ribbon`] for the full builder.
pub fn ribbon<'a, Message: Clone + 'a>() -> Ribbon<'a, Message> {
    Ribbon::new()
}
