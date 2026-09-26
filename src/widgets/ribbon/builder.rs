//! The ribbon builder and its view assembly.
//!
//! [`Ribbon`] turns a [`Vec`](super::model::RibbonTab) of tabs into the two
//! bands a ribbon is made of: a [`tabs`](crate::widgets::tabs) strip along the
//! top, and the active tab's groups laid out left to right beneath it. Each
//! group is built at all four densities and handed to
//! [`CollapseGroups`](super::collapse::CollapseGroups), which picks how far to
//! degrade them against the width it is offered — see [`collapse`](super::collapse).
//!
//! On top of that core, the builder supports the four alignment features:
//! minimized mode, a quick access bar, galleries, and contextual tabs.

use super::buttons::{
    render_group_collapsed, render_group_param, render_group_single, render_group_tight,
    render_quick_access_bar, DropdownCtx,
};
use super::collapse::{CollapseGroups, CollapseMode, GroupSlots};
use super::model::{ContextualTab, QuickAccessBar, RibbonLayout, RibbonState, RibbonTab};
use super::style::{band_style, tool_style};
use super::theme::RibbonTheme;
use crate::theme::Theme;
use crate::widgets::{tabs as tab_strip, Tab, TabVariant};
use iced::widget::{button, column, container, text};
use iced::{Element, Length, Padding, Rectangle};
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
    layout: RibbonLayout,
    minimized: bool,
    contextual_tabs: Vec<ContextualTab>,
    quick_access_bar: Option<QuickAccessBar<Message>>,
    ribbon_theme: RibbonTheme,
    show_tab_strip: bool,
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
            layout: RibbonLayout::default(),
            minimized: false,
            contextual_tabs: Vec::new(),
            quick_access_bar: None,
            ribbon_theme: RibbonTheme::default(),
            show_tab_strip: true,
            on_select: None,
            on_dropdown_toggle: None,
            on_dropdown_anchor: None,
            width: Length::Fill,
        }
    }

    /// Whether the ribbon draws its own tab strip.
    ///
    /// A caller that embeds the tabs inside its own title bar — a ribbon in the
    /// SARibbon style, where the application button, the quick access bar, the
    /// tabs and the window controls share one tinted row — turns this off,
    /// renders [`Ribbon::tab_strip_element`] inside that bar itself, and shows
    /// only the group band here.
    pub fn show_tab_strip(mut self, show: bool) -> Self {
        self.show_tab_strip = show;
        self
    }

    /// Builds the ribbon's tab strip as a standalone element.
    ///
    /// The strip is *not* tinted by the [`RibbonTheme`]: a caller that places it
    /// on a tinted surface recolors it itself through
    /// [`TabStripColors`](crate::widgets::TabStripColors), whose roles it knows
    /// better than the ribbon does. Requires [`Self::on_select`] to have been
    /// set; without it the strip would have nothing to report.
    pub fn tab_strip_element(&self) -> Option<Element<'_, Message, Theme>> {
        let on_select = self.on_select.as_ref()?;
        let labels = self
            .tabs
            .iter()
            .map(|tab| Tab::new(tab.title.clone()))
            .collect();
        Some(
            tab_strip(labels, self.active, move |index| on_select(index))
                .variant(TabVariant::Tab)
                .into_element(),
        )
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

    /// Reads the active tab, the open dropdown, the minimized state, the
    /// contextual tab, the ribbon theme, and the layout style from a
    /// [`RibbonState`].
    pub fn state(mut self, state: &RibbonState) -> Self {
        self.active = state.active;
        self.open_dropdown.clone_from(&state.open_dropdown);
        self.collapse_mode = match state.layout {
            RibbonLayout::LooseThreeRow => super::collapse::CollapseMode::Full,
            RibbonLayout::CompactThreeRow => super::collapse::CollapseMode::Compact,
            _ => state.collapse_mode,
        };
        self.minimized = state.minimized;
        self.contextual_tabs.clone_from(&state.contextual_tabs);
        self.ribbon_theme = state.ribbon_theme;
        self.layout = state.layout;
        self
    }

    /// Sets the groups' layout style directly, overriding any [`RibbonState`]
    /// value.
    pub fn layout(mut self, layout: RibbonLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Sets the active tab directly, overriding any [`RibbonState`].
    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
        self
    }

    /// Sets the minimized state directly, overriding any [`RibbonState`].
    pub fn minimized(mut self, minimized: bool) -> Self {
        self.minimized = minimized;
        self
    }

    /// Sets the contextual tabs directly, overriding any [`RibbonState`]
    /// value.
    pub fn contextual_tabs(mut self, tabs: Vec<ContextualTab>) -> Self {
        self.contextual_tabs = tabs;
        self
    }

    /// Sets the quick access bar shown above the tab strip.
    pub fn quick_access_bar(mut self, bar: QuickAccessBar<Message>) -> Self {
        self.quick_access_bar = Some(bar);
        self
    }

    /// Sets the ribbon's visual theme, overriding any [`RibbonState`] value.
    pub fn ribbon_theme(mut self, theme: RibbonTheme) -> Self {
        self.ribbon_theme = theme;
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
            collapse_mode: _,
            layout,
            minimized,
            contextual_tabs,
            quick_access_bar,
            ribbon_theme,
            show_tab_strip,
            on_select,
            on_dropdown_toggle,
            on_dropdown_anchor,
            width,
        } = self;

        let strip = if show_tab_strip {
            on_select.map(|on_select| {
                let labels = tabs.iter().map(|tab| Tab::new(tab.title.clone())).collect();
                tab_strip(labels, active, on_select)
                    .variant(TabVariant::Tab)
                    .into_element()
            })
        } else {
            // The caller renders the tabs itself (inside its own title bar, say),
            // so this ribbon contributes only the group band.
            None
        };

        let groups = tabs
            .get(active)
            .map_or(&[][..], |tab| tab.groups.as_slice());
        let toggle = on_dropdown_toggle.as_ref().map(Box::as_ref);
        let ctx = DropdownCtx {
            open: &open_dropdown,
            on_toggle: toggle,
            on_anchor: on_dropdown_anchor.as_ref(),
            ribbon_theme,
        };

        // Build each group once per density; `CollapseGroups` measures them and
        // chooses how far to degrade against the width it is offered. Slot 0 —
        // the level every fixed layout pins to — follows the layout style: the
        // single-row layouts lay their items out in one row, and the two-row
        // styles stack their columns two to a column instead of three.
        let slots: Vec<GroupSlots<'a, Message>> = groups
            .iter()
            .map(|group| {
                [
                    if layout.is_single_row() {
                        render_group_single(group, &ctx)
                    } else {
                        render_group_param(
                            group,
                            &ctx,
                            layout.rows_per_column(),
                            layout.keeps_large_items(),
                        )
                    },
                    render_group_param(group, &ctx, 3, false),
                    render_group_collapsed(group, &ctx),
                    render_group_tight(group, &ctx),
                ]
            })
            .collect();

        // Assemble the layers: quick access bar on top, then the tab strip,
        // then the group band (hidden when minimized).
        let mut layers: Vec<Element<'a, Message, Theme>> = Vec::new();

        if let Some(qab) = quick_access_bar {
            layers.push(render_quick_access_bar(&qab, ribbon_theme));
        }

        if let Some(strip) = strip {
            layers.push(strip);
        }

        if !minimized {
            layers.push(
                container(CollapseGroups::new(slots, layout))
                    .width(width)
                    .class(band_style(ribbon_theme))
                    .into(),
            );
        }

        // Contextual tabs: each rendered as a small labeled strip to the
        // right of the regular tabs, visually distinguished by its accent color.
        for ct in &contextual_tabs {
            let ct_el = render_contextual_tab(
                ct,
                &open_dropdown,
                toggle,
                on_dropdown_anchor.as_ref(),
                ribbon_theme,
            );
            layers.push(ct_el);
        }

        if layers.is_empty() {
            // No tabs and not minimized: still show an empty band.
            container(CollapseGroups::new(Vec::new(), layout))
                .width(width)
                .class(band_style(ribbon_theme))
                .into()
        } else if layers.len() == 1 {
            layers.pop().unwrap_or_else(|| {
                container(CollapseGroups::new(Vec::new(), layout))
                    .width(width)
                    .class(band_style(ribbon_theme))
                    .into()
            })
        } else {
            layers
                .into_iter()
                .fold(column![].spacing(2), |col, el| col.push(el))
                .into()
        }
    }
}

/// Renders a contextual tab as a small labeled strip.
fn render_contextual_tab<'a, Message: Clone + 'a>(
    ct: &ContextualTab,
    open_dropdown: &Option<String>,
    on_toggle: Option<&(dyn Fn(String) -> Message + 'a)>,
    on_anchor: Option<&Rc<dyn Fn(Rectangle) -> Message + 'a>>,
    ribbon_theme: RibbonTheme,
) -> Element<'a, Message, Theme> {
    let is_open = open_dropdown.as_deref() == Some(ct.name.as_str());
    let ct_color = ct.color;
    let label = text(ct.name.clone())
        .size(13)
        .class(Box::new(move |_theme: &Theme| {
            let color = ct_color.unwrap_or(ribbon_theme.accent());
            iced::widget::text::Style { color: Some(color) }
        }) as text::StyleFn<'a, Theme>);

    let mut btn = button(label)
        .class(tool_style(is_open, ribbon_theme))
        .padding(Padding::from([4.0, 10.0]));

    if let Some(toggle) = on_toggle {
        btn = btn.on_press(toggle(ct.name.clone()));
    }

    DropdownCtx {
        open: open_dropdown,
        on_toggle,
        on_anchor,
        ribbon_theme,
    }
    .anchored(btn)
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
