// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use iced::advanced;
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::time::Duration;
use iced::widget::overlay::menu;
use crate::widgets::overlay::menu as kit_menu;
use iced::widget::text::{LineHeight, Shaping};
use iced::widget::{self, button, container, svg, text as iced_text};
use iced::{Background, Element, Event, Length, Rectangle, Size, Vector};

use crate::dock::model::{Axis, Layout as ModelLayout, NodeId, NodeKind, Pane};
use crate::dock::style::{Catalog, DockStyle, PaneContent, StyleFn};
use crate::dock::widget::action::DockAction;
use crate::dock::panel::PanelPresentation;
use crate::dock::widget::area::DockArea;
use crate::dock::widget::controls as controls_mod;
use crate::dock::widget::event::{action_to_event, DockEvent};
use crate::dock::widget::split::SplitContainer;
use crate::dock::widget::state::{dispatch_action, DockWidgetState};
use crate::dock::widget::tab_dock::{TabDock, TabInfo};

/// Vertical attachment edge for the optional tab-bar scrollbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabBarScrollbarAttachment {
    /// Render the scrollbar flush with the top edge of the tab bar.
    #[default]
    Top,
    /// Render the scrollbar flush with the bottom edge of the tab bar.
    Bottom,
}

/// Persistent tree state shared across frames. Holds the dock layout state
/// and a cached theme reference for the layout pass (which doesn't receive `&Theme`).
struct DockTreeHolder<K, Theme>
where
    Theme: Catalog,
{
    dock_state: Rc<RefCell<DockWidgetState<K>>>,
    resolved_theme: Rc<RefCell<Option<Theme>>>,
}

/// The top-level docking widget.
///
/// `Dock` renders a full split/tab layout from a [`DockWidgetState`] and
/// rebuilds its internal element tree each layout pass (following the same
/// pattern as iced's `responsive()` widget).
///
/// Use the [`dock()`] free function to obtain a [`DockBuilder`] for
/// ergonomic construction.
///
/// # Type parameters
///
/// * `'a` — View lifetime (matches the application's `view(&self)` borrow).
/// * `K` — Content key type stored in each panel (e.g. an enum of panel kinds).
/// * `Message` — The application message type.
/// * `Theme` — The iced theme (must implement [`Catalog`]).
/// * `Renderer` — The iced renderer.
// The metric fields are all named `tab_bar_*` / `dock_*` on purpose: they are the
// names the builder setters expose, so a reader can match a field to its setter
// without a lookup table.
#[allow(clippy::struct_field_names)]
pub struct Dock<'a, K, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer + advanced::text::Renderer<Font = iced::Font> + advanced::svg::Renderer,
{
    content: Box<dyn Fn(K) -> PaneContent<'a, Message, Theme, Renderer> + 'a>,
    modified: Option<ModifiedFn<'a, K>>,
    tooltip: Option<TooltipFn<'a, K>>,
    on_event: Rc<dyn Fn(DockEvent<K>) -> Message>,
    external_state: Option<Rc<RefCell<DockWidgetState<K>>>>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    root: Element<'a, Message, Theme, Renderer>,
    tab_bar_height: f32,
    tab_bar_spacing: f32,
    tab_bar_padding: [f32; 2],
    tab_text_size: f32,
    tab_font: Option<Renderer::Font>,
    tab_line_height: Option<LineHeight>,
    tab_text_shaping: Option<Shaping>,
    tab_padding: [f32; 2],
    tab_accent_height: f32,
    close_button_size: f32,
    close_button_margin_right: f32,
    close_button_padding: [f32; 2],
    splitter_size: f32,
    splitter_gap: f32,
    pane_padding: f32,
    scrollbar_height: f32,
    scrollbar_thumb_min_width: f32,
    insert_marker_width: f32,
    separator_height: f32,
    min_pane_width: f32,
    min_pane_height: f32,
    drag_threshold: f32,
    drop_edge_fraction: f32,
    tab_bar_scrollbar_fade_duration: Duration,
    tab_bar_scrollbar_animated: bool,
    tab_bar_show_scrollbar: bool,
    tab_bar_scrollbar_attachment: TabBarScrollbarAttachment,
    tab_tooltip_delay: Duration,
    focus_frame_groups: Option<HashSet<String>>,
    close_icon: Option<Rc<dyn Fn() -> Element<'static, Message, Theme, Renderer>>>,
    overflow_icon: Option<Rc<dyn Fn() -> Element<'static, Message, Theme, Renderer>>>,
    on_close_requested: Option<Rc<dyn Fn(K) -> Message>>,
    /// Width of an edge dock's resize grab area.
    ///
    /// Named rather than `handle_width` so it reads as a setting at the call site:
    /// an application writing `.dock_handle_width(8.0)` should not have to remember
    /// what kind of handle it is.
    dock_handle_width: f32,
    /// What each panel adds to its own chrome.
    presentation: Rc<dyn PanelPresentation<K, Message, Theme, Renderer>>,
    /// How a group presents itself: a title bar for a lone panel, or always tabs.
    panel_style: crate::dock::panel::PanelStyle,
    /// Whether tab bars offer the affordance that collapses a neighbouring dock.
    toggle_button_visible: bool,
    /// Panes that belong to a closed dock, and so draw collapsed.
    ///
    /// Recomputed on every build, because which region owns a pane is not something
    /// a pane can answer on its own.
    collapsed_panes: Rc<RefCell<HashSet<NodeId>>>,
    /// Panes that are the only group of their region.
    ///
    /// A panel that is alone there cannot be dragged out, so its title is not a drag
    /// source. Also recomputed per build, for the same reason as `collapsed_panes`.
    alone_panes: Rc<RefCell<HashSet<NodeId>>>,
}

impl<'a, K, Message, Theme, Renderer> Dock<'a, K, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + kit_menu::Catalog
        + menu::Catalog
        + svg::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer
        + advanced::text::Renderer<Font = iced::Font>
        + advanced::svg::Renderer
        + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    for<'c> <Theme as svg::Catalog>::Class<'c>: From<svg::StyleFn<'c, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    /// Override the dock chrome style with a closure.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self
    where
        <Theme as Catalog>::Class<'static>: From<StyleFn<'static, Theme>>,
    {
        self.class = Rc::new((Box::new(style) as StyleFn<'static, Theme>).into());
        self
    }

    /// Sets the style class of the [`Dock`].
    #[must_use]
    pub fn class(mut self, class: <Theme as Catalog>::Class<'static>) -> Self {
        self.class = Rc::new(class);
        self
    }

    /// Attach shared widget state so the dock reads layout from an external
    /// [`DockWidgetState`] (typically obtained from [`DockSession::state`](crate::dock::DockSession::state)).
    #[must_use]
    pub fn with_state(mut self, state: Rc<RefCell<DockWidgetState<K>>>) -> Self {
        apply_focus_frame_groups(&state, self.focus_frame_groups.as_ref());
        self.external_state = Some(state);
        self
    }

    /// Fade duration for the tab-bar scrollbar when its visibility changes.
    ///
    /// Default is 0.5 seconds.
    #[must_use]
    pub fn tab_bar_scrollbar_fade_duration(mut self, duration: Duration) -> Self {
        self.tab_bar_scrollbar_fade_duration = duration;
        self
    }

    /// Whether the tab-bar scrollbar fades out when it hides.
    ///
    /// When `false`, the scrollbar snaps visible and hidden instantly.
    /// Default is `true`.
    #[must_use]
    pub fn tab_bar_scrollbar_animated(mut self, animated: bool) -> Self {
        self.tab_bar_scrollbar_animated = animated;
        self
    }

    /// Whether overflowing tab bars show a horizontal scrollbar thumb.
    ///
    /// When `false`, tabs can still be scrolled with the mouse wheel (and Shift+wheel).
    /// Default is `true`.
    #[must_use]
    pub fn tab_bar_show_scrollbar(mut self, show: bool) -> Self {
        self.tab_bar_show_scrollbar = show;
        self
    }

    /// Vertical edge used to attach the optional tab-bar scrollbar.
    ///
    /// Default is [`TabBarScrollbarAttachment::Top`].
    #[must_use]
    pub fn tab_bar_scrollbar_attachment(mut self, attachment: TabBarScrollbarAttachment) -> Self {
        self.tab_bar_scrollbar_attachment = attachment;
        self
    }

    /// Replace the default close-button SVG with a custom element.
    ///
    /// The closure is called once per closeable tab to produce the icon element
    /// rendered inside the close button. The button wrapper (on_press, padding,
    /// hover background) is unchanged.
    #[must_use]
    pub fn close_icon(
        mut self,
        f: impl Fn() -> Element<'static, Message, Theme, Renderer> + 'static,
    ) -> Self {
        self.close_icon = Some(Rc::new(f));
        self
    }

    /// Replace the default chevron SVG in the overflow button with a custom element.
    ///
    /// The closure is called each frame the overflow button is drawn. The
    /// button background and separator are still drawn by the dock; only the
    /// icon glyph is replaced.
    #[must_use]
    pub fn overflow_icon(
        mut self,
        f: impl Fn() -> Element<'static, Message, Theme, Renderer> + 'static,
    ) -> Self {
        self.overflow_icon = Some(Rc::new(f));
        self
    }

    fn wrap_action(
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        on_event: &Rc<dyn Fn(DockEvent<K>) -> Message>,
        on_close_requested: Option<&Rc<dyn Fn(K) -> Message>>,
        action: DockAction,
    ) -> Message {
        if let DockAction::Tab(crate::dock::widget::action::TabAction::Close { panel }) = &action {
            if let Some(hook) = on_close_requested {
                let state = holder.borrow();
                if let Some(NodeKind::Panel(p)) = state.layout.kind(*panel) {
                    let key = p.content;
                    drop(state);
                    return (hook)(key);
                }
            }
        }
        let mut state = holder.borrow_mut();
        // A dock action's outcome lives in the region rather than the tree, so it
        // is read back after the action is applied: `action_to_event` only sees
        // the tree, and would have to guess otherwise.
        let dock_placement = match &action {
            DockAction::ToggleDock { placement } | DockAction::DockResize { placement, .. } => {
                Some(*placement)
            }
            _ => None,
        };
        let pane_for_zoom = match &action {
            DockAction::ToggleZoom { pane } => Some(*pane),
            _ => None,
        };
        let event = action_to_event(&state.layout, &action).unwrap_or(DockEvent::LayoutChanged);
        dispatch_action(&mut state, action);
        let event = match event {
            DockEvent::LayoutChanged => match (dock_placement, pane_for_zoom) {
                (Some(placement), _) => state.regions.dock(placement).map_or(event, |dock| {
                    DockEvent::DockToggled {
                        placement,
                        open: dock.is_open(),
                    }
                }),
                (None, Some(pane)) => DockEvent::ZoomChanged {
                    zoomed: state.is_zoomed(pane),
                    panel: state
                        .layout
                        .kind(pane)
                        .and_then(|k| match k {
                            NodeKind::Pane(p) => p.active.or_else(|| p.tabs.first().copied()),
                            _ => None,
                        })
                        .and_then(|p| match state.layout.kind(p) {
                            Some(NodeKind::Panel(panel)) => Some(panel.content),
                            _ => None,
                        }),
                },
                _ => event,
            },
            other => other,
        };
        (on_event)(event)
    }

    fn build_node(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        theme_cell: &Rc<RefCell<Option<Theme>>>,
        layout: &ModelLayout<K>,
        node: NodeId,
    ) -> Option<Element<'a, Message, Theme, Renderer>> {
        match layout.kind(node)? {
            NodeKind::Proportional(pg) => {
                let children: Vec<_> = pg
                    .children
                    .iter()
                    .filter_map(|&c| self.build_node(holder, theme_cell, layout, c))
                    .collect();
                if children.is_empty() {
                    return None;
                }
                let h = Rc::clone(holder);
                let on_ev = Rc::clone(&self.on_event);
                let on_close = self.on_close_requested.as_ref().map(Rc::clone);
                let on_split = Rc::new(move |action: DockAction| {
                    Self::wrap_action(&h, &on_ev, on_close.as_ref(), action)
                });
                let drag_holder = Rc::clone(holder);
                let drag_active: Rc<dyn Fn() -> bool> =
                    Rc::new(move || drag_holder.borrow().drag.is_some());
                Some(
                    SplitContainer::new(
                        node,
                        pg.axis,
                        pg.proportions.clone(),
                        children,
                        on_split,
                        drag_active,
                        Rc::clone(&self.class),
                        self.splitter_size,
                        self.splitter_gap,
                        self.min_pane_width,
                        self.min_pane_height,
                    )
                    .into(),
                )
            }
            NodeKind::Pane(p) => self.build_pane(holder, theme_cell, layout, node, p),
            NodeKind::Panel(_) => None,
            NodeKind::Root(_) => {
                let c = layout.root_child()?;
                self.build_node(holder, theme_cell, layout, c)
            }
        }
    }

    fn build_pane(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        theme_cell: &Rc<RefCell<Option<Theme>>>,
        layout: &ModelLayout<K>,
        pane_id: NodeId,
        pane: &Pane,
    ) -> Option<Element<'a, Message, Theme, Renderer>> {
        if pane.tabs.is_empty() {
            if !pane.persistent {
                return None;
            }
            let class = Rc::clone(&self.class);
            let frame_holder = Rc::clone(holder);
            return Some(
                container(widget::space::Space::new())
                    .style(move |t: &Theme| {
                        let ds = Catalog::style(t, &class);
                        let is_focused = frame_holder.borrow().focus_frame_pane == Some(pane_id);
                        let border = if is_focused {
                            ds.window.focused_border.unwrap_or(ds.window.border)
                        } else {
                            ds.window.border
                        };
                        container::Style {
                            background: Some(Background::Color(ds.window.background)),
                            border,
                            ..Default::default()
                        }
                    })
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
        }
        // The pane's tabs, the visible set: a hidden panel keeps its node and its
        // slot but is left out of the strip, which is what "hidden" means to a
        // user. Both halves are consulted: the declared `visible` flag from the
        // layout, and the runtime set the application toggles through
        // `DockWidgetState::set_panel_visible`.
        let hidden = holder.borrow().hidden.clone();
        let tabs: Vec<TabInfo> = pane
            .tabs
            .iter()
            .filter_map(|&id| {
                let e = layout.get(id)?;
                match &e.kind {
                    NodeKind::Panel(m) if m.visible && !hidden.contains(&m.id) => Some(TabInfo {
                        id,
                        title: m.title.clone(),
                        tab_name: m.tab_name.clone(),
                        can_close: m.can_close,
                        can_drag: m.can_drag,
                        can_zoom: m.can_zoom,
                        is_modified: self.modified.as_ref().is_some_and(|f| f(m.content)),
                        tooltip: self.tooltip.as_ref().and_then(|f| f(m.content)),
                    }),
                    _ => None,
                }
            })
            .collect();

        // What the group *displays* is the stored active tab when it is one of the
        // visible ones, otherwise the first visible tab takes over. Resolving
        // through the raw `active` alone is what let a hidden panel keep the
        // stage: its tab left the strip, but its body went on filling the pane,
        // and the panel that should have replaced it drew nothing. Taking simply
        // the first visible tab would be wrong the other way — a click on a later
        // tab activates it, and the pane must show what was clicked.
        let displayed = pane
            .active
            .filter(|&active| tabs.iter().any(|tab| tab.id == active))
            .or_else(|| tabs.first().map(|tab| tab.id));
        let Some(displayed) = displayed else {
            // Every tab is hidden. A pane with nothing to draw does not *draw an
            // empty frame* — it does not exist, the same rule an emptied pane
            // follows. Returning an element here would keep the pane's slot in
            // the split, so hiding a group's last panel would leave a blank
            // region holding space its neighbours never receive.
            return None;
        };
        let content_key: K = match layout.get(displayed).map(|e| &e.kind) {
            Some(NodeKind::Panel(m)) => m.content,
            _ => return None,
        };

        let pane_content = (self.content)(content_key);
        let pane_class = pane_content
            .style
            .map_or_else(|| Rc::clone(&self.class), Rc::new);
        let content = pane_content.element;

        let h = Rc::clone(holder);
        let on_ev = Rc::clone(&self.on_event);
        let on_close = self.on_close_requested.as_ref().map(Rc::clone);
        let on_tab = Rc::new(move |action: DockAction| {
            Self::wrap_action(&h, &on_ev, on_close.as_ref(), action)
        });

        // Whether a lone panel gets a title bar rather than a strip of tabs. A
        // panel with its own chrome declines, and is then just its content.
        let lone_panel = tabs.len() == 1;
        let wants_title = lone_panel
            && self.presentation.title_bar(content_key)
            && matches!(self.panel_style, crate::dock::panel::PanelStyle::Auto);

        // The title is a drag source, because a group holding one panel otherwise
        // has no way to be picked up at all — and most groups in a workspace hold
        // one panel.
        let title = wants_title.then(|| {
            let title = self.build_title(content_key, &tabs);
            let on_title_drag = {
                let h = Rc::clone(holder);
                let on_ev = Rc::clone(&self.on_event);
                let on_close = self.on_close_requested.as_ref().map(Rc::clone);
                Rc::new(move |action: DockAction| {
                    Self::wrap_action(&h, &on_ev, on_close.as_ref(), action)
                })
            };
            // A panel that is the only one in the only group cannot be dragged out:
            // there would be nothing left to show, and no way to put it back.
            let alone = holder.borrow().layout.root_child().is_none() || self.is_only_group(pane_id);
            let draggable = tabs
                .first()
                .is_some_and(|tab| tab.can_drag)
                && !alone;
            crate::dock::widget::title_drag::TitleDrag::new(
                pane_id,
                displayed,
                draggable,
                title,
                on_title_drag,
                self.drag_threshold,
                self.drop_edge_fraction,
            )
            .into()
        });

        let leading = self.build_dock_toggles(pane_id, holder);
        let trailing = self.build_trailing(pane_id, content_key, holder, &tabs);

        let inner_padding = if tabs.len() > 1 {
            // A strip of tabs already separates the content from the chrome, and
            // padding on top of that would sink the content for no reason.
            false
        } else {
            self.presentation.inner_padding(content_key)
        };

        Some(
            TabDock::new(
                Rc::clone(holder),
                pane_id,
                tabs,
                displayed,
                content,
                title,
                leading,
                trailing,
                on_tab,
                pane_class,
                Rc::clone(theme_cell),
                inner_padding,
                self.is_pane_collapsed(pane_id),
                self.tab_bar_height,
                self.tab_bar_spacing,
                self.tab_bar_padding,
                self.tab_text_size,
                self.tab_font,
                self.tab_line_height,
                self.tab_text_shaping,
                self.tab_padding,
                self.tab_accent_height,
                self.close_button_size,
                self.close_button_margin_right,
                self.close_button_padding,
                self.pane_padding,
                self.scrollbar_height,
                self.scrollbar_thumb_min_width,
                self.insert_marker_width,
                self.separator_height,
                self.drag_threshold,
                self.drop_edge_fraction,
                self.tab_bar_scrollbar_fade_duration,
                self.tab_bar_scrollbar_animated,
                self.tab_bar_show_scrollbar,
                self.tab_bar_scrollbar_attachment,
                self.tab_tooltip_delay,
                self.close_icon.clone(),
                self.overflow_icon.clone(),
            )
            .into(),
        )
    }

    /// Whether `pane` is the only group in its region, and holds one panel.
    ///
    /// Dragging such a panel out would leave the region empty — nothing on screen,
    /// and no target to drag it back to — so the gesture is refused for it. This is
    /// the title-bar counterpart of the rule the tab strip applies to the last tab of
    /// the last group.
    fn is_only_group(&self, pane: NodeId) -> bool {
        // The panel's owning tree is found by walking, because a pane does not know
        // which region it belongs to.
        let holder = &self;
        let _ = holder;
        let _ = pane;
        // Filled in by `collapsed_panes`' sibling: the set of panes that are alone.
        self.alone_panes.borrow().contains(&pane)
    }

    /// Whether a pane belongs to a dock that is closed, and so draws collapsed.
    fn is_pane_collapsed(&self, pane: NodeId) -> bool {
        let holder = &self;
        let _ = holder;
        let _ = pane;
        // Filled in by the caller through `collapsed_panes`: a pane cannot tell
        // which region owns it without walking every tree, which the area has
        // already done.
        self.collapsed_panes.borrow().contains(&pane)
    }

    /// The title element for a lone panel: the panel's own element when it has
    /// one, styled text otherwise.
    fn build_title(&self, key: K, tabs: &[TabInfo]) -> Element<'a, Message, Theme, Renderer> {
        if let Some(element) = self.presentation.title(key) {
            return element;
        }
        // No title element of its own, so the panel's title string is drawn in the
        // title bar's own color rather than a tab's.
        let label = tabs
            .first()
            .map_or_else(String::new, |tab| tab.label().to_owned());
        let class = Rc::clone(&self.class);
        let text_size = self.tab_text_size;
        iced::widget::text(label)
            .size(text_size)
            .style(move |theme: &Theme| iced::widget::text::Style {
                color: Some(Catalog::style(theme, &class).title.text_color),
            })
            .into()
    }

    /// The dock toggles this group offers, if it is the one that carries them.
    ///
    /// Only one group per dock shows the affordance, and a zoomed or
    /// non-collapsible dock shows none — otherwise every tab bar in a dock would
    /// carry a duplicate button, and a dock that refuses to close would offer a
    /// button that does nothing.
    fn build_dock_toggles(
        &self,
        pane_id: NodeId,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
    ) -> Vec<Element<'a, Message, Theme, Renderer>> {
        if !self.toggle_button_visible {
            return Vec::new();
        }
        let state = holder.borrow();
        if state.regions.zoomed().is_some() {
            return Vec::new();
        }
        let mut controls = Vec::new();
        for placement in crate::dock::model::DockPlacement::DOCKS {
            if !state.regions.has_dock(placement) || !state.regions.is_dock_collapsible(placement) {
                continue;
            }
            if !self.is_toggle_group(placement, pane_id, &state) {
                continue;
            }
            let open = state.regions.is_dock_open(placement);
            let Some(icon) = controls_mod::dock_toggle_icon(placement, open) else {
                continue;
            };
            drop(state);
            let h = Rc::clone(holder);
            let on_ev = Rc::clone(&self.on_event);
            let on_close = self.on_close_requested.as_ref().map(Rc::clone);
            let on_event = Rc::new(move |action: DockAction| {
                Self::wrap_action(&h, &on_ev, on_close.as_ref(), action)
            });
            controls.push(
                controls_mod::ToggleButton::new(
                    iced::advanced::widget::Id::from(format!("dock-toggle:{}", placement.name())),
                    icon,
                    DockAction::ToggleDock { placement },
                    on_event,
                    Rc::clone(&self.class),
                    self.control_size(),
                    // An open dock shows its button lit, so the state is readable
                    // without hovering.
                    open,
                )
                .into(),
            );
            return controls;
        }
        controls
    }

    /// Whether `pane` is the group a dock's toggle button belongs to.
    ///
    /// The designated group is the top-most one on the dock's own side of the
    /// centre, which mirrors where the dock itself sits: a left dock's button
    /// belongs in the top-left group, a bottom dock's in the left-most group of
    /// the bottom region.
    fn is_toggle_group(
        &self,
        placement: crate::dock::model::DockPlacement,
        pane: NodeId,
        state: &DockWidgetState<K>,
    ) -> bool {
        use crate::dock::model::DockPlacement;
        let tree = match placement {
            DockPlacement::Bottom => state
                .regions
                .region(DockPlacement::Bottom)
                .map(|r| &r.tree),
            DockPlacement::Left | DockPlacement::Right | DockPlacement::Center => {
                Some(&state.layout)
            }
        };
        let Some(tree) = tree else {
            return false;
        };
        let designated = match placement {
            DockPlacement::Right => right_top_group(tree),
            DockPlacement::Left | DockPlacement::Bottom => left_top_group(tree),
            DockPlacement::Center => None,
        };
        designated == Some(pane)
    }

    fn control_size(&self) -> f32 {
        self.tab_bar_height.min(24.0)
    }

    /// The trailing controls for a group: the panel's toolbar, then zoom, then
    /// the menu.
    fn build_trailing(
        &self,
        pane_id: NodeId,
        key: K,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        tabs: &[TabInfo],
    ) -> Vec<Element<'a, Message, Theme, Renderer>> {
        let mut controls = Vec::new();
        if self.is_collapsed_in(holder, pane_id) {
            // A collapsed group is a way back in, not a place to work, so its
            // chrome carries nothing that acts on the hidden panel.
            return controls;
        }
        let active = self.active_of(tabs).and_then(|id| tabs.iter().find(|tab| tab.id == id));
        let can_zoom = active.is_some_and(|tab| tab.can_zoom);
        let zoomed = holder.borrow().regions.zoomed().is_some();
        let control = self.presentation.zoom_control(key);

        for (index, button) in self.presentation.toolbar(key).into_iter().enumerate() {
            let _ = index;
            controls.push(controls_mod::fit_to_bar(button, self.control_size()));
        }

        if can_zoom && control.is_some_and(crate::dock::panel::PanelControl::toolbar_visible) {
            let h = Rc::clone(holder);
            let on_ev = Rc::clone(&self.on_event);
            let on_close = self.on_close_requested.as_ref().map(Rc::clone);
            let on_event = Rc::new(move |action: DockAction| {
                Self::wrap_action(&h, &on_ev, on_close.as_ref(), action)
            });
            controls.push(
                controls_mod::ToggleButton::new(
                    iced::advanced::widget::Id::from(format!("dock-zoom:{}", pane_id.as_u64())),
                    if zoomed {
                        controls_mod::DockIcon::Restore
                    } else {
                        controls_mod::DockIcon::Maximize
                    },
                    DockAction::ToggleZoom { pane: pane_id },
                    on_event,
                    Rc::clone(&self.class),
                    self.control_size(),
                    zoomed,
                )
                .into(),
            );
        }

        if control.is_some_and(crate::dock::panel::PanelControl::menu_visible) {
            let entries: Vec<controls_mod::MenuEntry<Message>> = self.presentation.menu(key);
            let h = Rc::clone(holder);
            let on_ev = Rc::clone(&self.on_event);
            let on_close = self.on_close_requested.as_ref().map(Rc::clone);
            let on_event = Rc::new(move |action: DockAction| {
                Self::wrap_action(&h, &on_ev, on_close.as_ref(), action)
            });
            let can_close = active.is_some_and(|tab| tab.can_close);
            controls.push(
                controls_mod::PanelMenu::new(
                    // Unique per pane: iced keys element state by id, so a shared
                    // literal would collapse two groups' menus into one and make
                    // only one of them reachable.
                    iced::advanced::widget::Id::from(format!("dock-panel-menu:{}", pane_id.as_u64())),
                    pane_id,
                    // The panel the entry acts on is the one displayed, not the
                    // pane itself: `Close` names a *panel* node, and a pane id
                    // here would close nothing — the state would refuse to remove
                    // a pane as if it were a tab.
                    active.map(|tab| tab.id),
                    zoomed,
                    can_zoom,
                    can_close,
                    entries,
                    on_event,
                    Rc::clone(&self.class),
                    self.control_size(),
                )
                .into(),
            );
        }

        controls
    }

    /// The tab the group displays: the first one it offers.
    ///
    /// The tabs list is already the *visible* set — a hidden panel never reaches
    /// it — so the first entry is what is on screen, and a hidden active tab
    /// falls back to it without a special case.
    fn active_of(&self, tabs: &[TabInfo]) -> Option<NodeId> {
        tabs.first().map(|tab| tab.id)
    }

    fn is_collapsed_in(&self, holder: &Rc<RefCell<DockWidgetState<K>>>, pane: NodeId) -> bool {
        holder.borrow().collapsed_panes.contains(&pane)
    }

    fn build_root_element(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        theme_cell: &Rc<RefCell<Option<Theme>>>,
    ) -> Element<'a, Message, Theme, Renderer> {
        // Which panes belong to a closed dock has to be known before the groups
        // are built, because a collapsed group draws no content and no controls.
        // It cannot be answered by a pane: ownership is a property of the region,
        // so the walk happens here, once, over every tree.
        {
            let state = holder.borrow();
            let mut collapsed = std::collections::HashSet::new();
            let mut alone = std::collections::HashSet::new();

            // The centre, then every dock: a region's only group cannot relinquish
            // its last panel by dragging it elsewhere.
            let regions: Vec<&ModelLayout<K>> = std::iter::once(&state.layout)
                .chain(state.regions.iter().map(|(_, region)| &region.tree))
                .collect();
            for (index, tree) in regions.iter().enumerate() {
                let is_dock = index > 0;
                let placement = if is_dock {
                    state.regions.iter().nth(index - 1).map(|(p, _)| p)
                } else {
                    None
                };
                if let Some(placement) = placement {
                    if state.regions.dock(placement).is_some_and(|d| !d.is_open()) {
                        for pane in panes_in_tree(tree) {
                            collapsed.insert(pane);
                        }
                    }
                }
                let panes = panes_in_tree(tree);
                if panes.len() <= 1 {
                    alone.extend(panes);
                }
            }

            *self.collapsed_panes.borrow_mut() = collapsed;
            *self.alone_panes.borrow_mut() = alone;
        }
        let state = holder.borrow();

        // A zoomed panel is the whole area, so nothing else is built for it: the
        // docks would be laid out and then thrown away.
        let zoomed = state
            .regions
            .zoomed()
            .and_then(|node| self.build_zoomed(holder, theme_cell, node));

        let center = self.build_region(holder, theme_cell, &state.layout);
        let dock = |placement| {
            state.regions.region(placement).map_or_else(
                || Element::new(widget::space::Space::new()),
                |region| self.build_region(holder, theme_cell, &region.tree),
            )
        };
        let left = dock(crate::dock::model::DockPlacement::Left);
        let bottom = dock(crate::dock::model::DockPlacement::Bottom);
        let right = dock(crate::dock::model::DockPlacement::Right);
        drop(state);

        let h = Rc::clone(holder);
        let on_ev = Rc::clone(&self.on_event);
        let on_close = self.on_close_requested.as_ref().map(Rc::clone);
        let on_event = Rc::new(move |action: DockAction| {
            Self::wrap_action(&h, &on_ev, on_close.as_ref(), action)
        });

        DockArea::new(
            Rc::clone(holder),
            [center, left, bottom, right],
            zoomed,
            on_event,
            Rc::clone(&self.class),
            Rc::clone(theme_cell),
            self.dock_handle_width,
        )
        .into()
    }

    /// The root of one region's tree, or an empty element when the region has
    /// nothing to draw.
    fn build_region(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        theme_cell: &Rc<RefCell<Option<Theme>>>,
        tree: &ModelLayout<K>,
    ) -> Element<'a, Message, Theme, Renderer> {
        tree.root_child()
            .and_then(|root| self.build_node(holder, theme_cell, tree, root))
            .unwrap_or_else(|| Element::new(widget::space::Space::new()))
    }

    /// The single pane that fills the area while zoomed.
    ///
    /// The zoomed node lives in whichever region owns it, so the tree is looked
    /// up by node rather than assumed to be the centre.
    fn build_zoomed(
        &self,
        holder: &Rc<RefCell<DockWidgetState<K>>>,
        theme_cell: &Rc<RefCell<Option<Theme>>>,
        node: crate::dock::model::NodeId,
    ) -> Option<Element<'a, Message, Theme, Renderer>> {
        let state = holder.borrow();
        if state.layout.contains_key(node) {
            return self.build_node(holder, theme_cell, &state.layout, node);
        }
        for (_, region) in state.regions.iter() {
            if region.tree.contains_key(node) {
                return self.build_node(holder, theme_cell, &region.tree, node);
            }
        }
        None
    }

    fn rebuild_root(&mut self, tree: &mut Tree) {
        let holder = tree.state.downcast_ref::<DockTreeHolder<K, Theme>>();
        let dock_state = Rc::clone(&holder.dock_state);
        let theme_cell = Rc::clone(&holder.resolved_theme);
        self.root = self.build_root_element(&dock_state, &theme_cell);
        tree.diff_children(std::slice::from_ref(&self.root));
    }
}

type ContentFn<'a, K, Message, Theme, Renderer> =
    Box<dyn Fn(K) -> PaneContent<'a, Message, Theme, Renderer> + 'a>;

type ModifiedFn<'a, K> = Box<dyn Fn(K) -> bool + 'a>;

type TooltipFn<'a, K> = Box<dyn Fn(K) -> Option<String> + 'a>;

/// Builder for constructing a [`Dock`] widget with ergonomic chained setters.
///
/// Obtained via [`dock()`]. At minimum, call [`content`](Self::content),
/// [`on_event`](Self::on_event), [`state`](Self::state), and [`build`](Self::build):
///
/// ```ignore
/// dock()
///     .state(session.state())
///     .on_event(Message::DockEvent)
///     .content(|key| view_panel(key))
///     .build()
/// ```
#[allow(clippy::struct_field_names)]
pub struct DockBuilder<'a, K, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer + advanced::text::Renderer<Font = iced::Font> + advanced::svg::Renderer,
{
    content: Option<ContentFn<'a, K, Message, Theme, Renderer>>,
    modified: Option<ModifiedFn<'a, K>>,
    tooltip: Option<TooltipFn<'a, K>>,
    on_event: Option<Rc<dyn Fn(DockEvent<K>) -> Message>>,
    shared_state: Option<Rc<RefCell<DockWidgetState<K>>>>,
    class: Option<Rc<<Theme as Catalog>::Class<'static>>>,
    tab_bar_height: f32,
    tab_bar_spacing: f32,
    tab_bar_padding: [f32; 2],
    tab_text_size: f32,
    tab_font: Option<Renderer::Font>,
    tab_line_height: Option<LineHeight>,
    tab_text_shaping: Option<Shaping>,
    tab_padding: [f32; 2],
    tab_accent_height: f32,
    close_button_size: f32,
    close_button_margin_right: f32,
    close_button_padding: [f32; 2],
    splitter_size: f32,
    splitter_gap: f32,
    pane_padding: f32,
    scrollbar_height: f32,
    scrollbar_thumb_min_width: f32,
    insert_marker_width: f32,
    separator_height: f32,
    min_pane_width: f32,
    min_pane_height: f32,
    drag_threshold: f32,
    drop_edge_fraction: f32,
    tab_bar_scrollbar_fade_duration: Duration,
    tab_bar_scrollbar_animated: bool,
    tab_bar_show_scrollbar: bool,
    tab_bar_scrollbar_attachment: TabBarScrollbarAttachment,
    tab_tooltip_delay: Duration,
    focus_frame_groups: Option<HashSet<String>>,
    close_icon: Option<Rc<dyn Fn() -> Element<'static, Message, Theme, Renderer>>>,
    overflow_icon: Option<Rc<dyn Fn() -> Element<'static, Message, Theme, Renderer>>>,
    on_close_requested: Option<Rc<dyn Fn(K) -> Message>>,
    /// Width of an edge dock's resize grab area.
    ///
    /// Named rather than `handle_width` so it reads as a setting at the call site:
    /// an application writing `.dock_handle_width(8.0)` should not have to remember
    /// what kind of handle it is.
    dock_handle_width: f32,
    /// What each panel adds to its own chrome.
    presentation: Rc<dyn PanelPresentation<K, Message, Theme, Renderer>>,
    /// How a group presents itself: a title bar for a lone panel, or always tabs.
    panel_style: crate::dock::panel::PanelStyle,
    /// Whether tab bars offer the affordance that collapses a neighbouring dock.
    toggle_button_visible: bool,
}

impl<K, Message, Theme, Renderer> Default for DockBuilder<'_, K, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer + advanced::text::Renderer<Font = iced::Font> + advanced::svg::Renderer,
{
    fn default() -> Self {
        Self {
            content: None,
            modified: None,
            tooltip: None,
            on_event: None,
            shared_state: None,
            class: None,
            tab_bar_height: 30.0,
            tab_bar_spacing: 0.0,
            tab_bar_padding: [0.0, 0.0],
            tab_text_size: 12.0,
            tab_font: None,
            tab_line_height: None,
            tab_text_shaping: None,
            tab_padding: [0.0, 10.0],
            tab_accent_height: 2.0,
            close_button_size: 20.0,
            close_button_margin_right: 6.0,
            close_button_padding: [0.0, 0.0],
            splitter_size: 0.5,
            splitter_gap: 10.0,
            pane_padding: 0.0,
            scrollbar_height: 6.0,
            scrollbar_thumb_min_width: 24.0,
            insert_marker_width: 3.0,
            separator_height: 1.0,
            min_pane_width: 80.0,
            min_pane_height: 80.0,
            drag_threshold: 6.0,
            drop_edge_fraction: 0.2,
            tab_bar_scrollbar_fade_duration: Duration::from_millis(500),
            tab_bar_scrollbar_animated: true,
            tab_bar_show_scrollbar: false,
            tab_bar_scrollbar_attachment: TabBarScrollbarAttachment::Top,
            tab_tooltip_delay: Duration::from_millis(500),
            focus_frame_groups: None,
            close_icon: None,
            overflow_icon: None,
            on_close_requested: None,
            dock_handle_width: 6.0,
            presentation: Rc::new(crate::dock::panel::PlainPanels),
            panel_style: crate::dock::panel::PanelStyle::Auto,
            toggle_button_visible: true,
        }
    }
}

impl<'a, K, Message, Theme, Renderer> DockBuilder<'a, K, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + kit_menu::Catalog
        + menu::Catalog
        + svg::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer
        + advanced::text::Renderer<Font = iced::Font>
        + advanced::svg::Renderer
        + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    for<'c> <Theme as svg::Catalog>::Class<'c>: From<svg::StyleFn<'c, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    /// Set the content factory that maps a panel key `K` to its [`Element`].
    ///
    /// The closure may borrow from application state; it only needs to live
    /// as long as the view frame (`'a`).
    #[must_use]
    pub fn content(mut self, f: impl Fn(K) -> Element<'a, Message, Theme, Renderer> + 'a) -> Self {
        self.content = Some(Box::new(move |key| PaneContent::from(f(key))));
        self
    }

    /// Like [`content`](Self::content), but the closure returns [`PaneContent`]
    /// for per-pane style overrides.
    #[must_use]
    pub fn content_styled(
        mut self,
        f: impl Fn(K) -> PaneContent<'a, Message, Theme, Renderer> + 'a,
    ) -> Self {
        self.content = Some(Box::new(f));
        self
    }

    /// Set a predicate that marks tabs as modified (unsaved changes).
    ///
    /// Queried per tab each view pass, so the flag always reflects current
    /// application state — no imperative syncing. Modified tabs render a `*`
    /// title suffix and use [`TabStyle::modified_background`](crate::dock::style::TabStyle::modified_background)
    /// when set.
    #[must_use]
    pub fn modified(mut self, f: impl Fn(K) -> bool + 'a) -> Self {
        self.modified = Some(Box::new(f));
        self
    }

    /// Set a closure that provides a hover tooltip per tab (e.g. the full
    /// file path). Queried per tab each view pass; `None` disables the
    /// tooltip for that tab.
    ///
    /// The tooltip appears below the tab after
    /// [`tab_tooltip_delay`](Self::tab_tooltip_delay) and is styled via
    /// [`DockStyle::tooltip`](crate::dock::style::DockStyle::tooltip).
    #[must_use]
    pub fn tab_tooltip(mut self, f: impl Fn(K) -> Option<String> + 'a) -> Self {
        self.tooltip = Some(Box::new(f));
        self
    }

    /// Hover time before a tab tooltip appears. Default 500 ms.
    #[must_use]
    pub fn tab_tooltip_delay(mut self, delay: Duration) -> Self {
        self.tab_tooltip_delay = delay;
        self
    }

    /// Map observation [`DockEvent`] values to the application message type.
    ///
    /// The widget applies layout mutations before this callback; do not call
    /// [`DockSession::dispatch`](crate::dock::DockSession::dispatch) for widget-originated events.
    #[must_use]
    pub fn on_event(mut self, f: impl Fn(DockEvent<K>) -> Message + 'static) -> Self {
        self.on_event = Some(Rc::new(f));
        self
    }

    /// Attach shared widget state (typically obtained from
    /// [`DockSession::state`](crate::dock::DockSession::state)).
    #[must_use]
    pub fn state(mut self, state: Rc<RefCell<DockWidgetState<K>>>) -> Self {
        self.shared_state = Some(state);
        self
    }

    /// Override the dock chrome style with a closure.
    #[must_use]
    pub fn style(mut self, style: impl Fn(&Theme) -> DockStyle + 'static) -> Self
    where
        <Theme as Catalog>::Class<'static>: From<StyleFn<'static, Theme>>,
    {
        self.class = Some(Rc::new((Box::new(style) as StyleFn<'static, Theme>).into()));
        self
    }

    /// Sets the style class of the [`Dock`].
    #[must_use]
    pub fn class(mut self, class: <Theme as Catalog>::Class<'static>) -> Self {
        self.class = Some(Rc::new(class));
        self
    }

    /// Minimum width of each pane in horizontal split groups.
    ///
    /// Split drags stop when an adjacent pair would shrink a pane below this width.
    /// Default is `80.0`.
    #[must_use]
    pub fn min_pane_width(mut self, min_pane_width: f32) -> Self {
        self.min_pane_width = min_pane_width.max(1.0);
        self
    }

    /// Restrict the pane focus frame to panes tagged with one of these tab groups.
    ///
    /// Groups are the tags set with [`TabsNode::group`](crate::dock::TabsNode::group). When this is
    /// set:
    ///
    /// * Clicking a pane outside these groups leaves the frame on the last focused pane that
    ///   *is* in one of them — useful for keeping the frame on the active document group while
    ///   the user clicks around tool panes. Logical focus still moves, so
    ///   [`DockEvent::PaneFocused`](crate::dock::DockEvent::PaneFocused) still fires for tool panes
    ///   and [`DockSession::cycle_panel`](crate::dock::DockSession::cycle_panel) still acts on them.
    /// * [`DockSession::focus_adjacent`](crate::dock::DockSession::focus_adjacent) only moves focus
    ///   *to* panes in these groups; ineligible panes are skipped.
    ///
    /// By default every pane shows the frame while focused and is reachable by directional
    /// focus movement.
    ///
    /// ```ignore
    /// dock().focus_frame_groups(["documents"])
    /// ```
    #[must_use]
    pub fn focus_frame_groups(
        mut self,
        groups: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.focus_frame_groups = Some(groups.into_iter().map(Into::into).collect());
        self
    }

    /// Minimum height of each pane in vertical split groups.
    ///
    /// Split drags stop when an adjacent pair would shrink a pane below this height.
    /// Default is `80.0`.
    #[must_use]
    pub fn min_pane_height(mut self, min_pane_height: f32) -> Self {
        self.min_pane_height = min_pane_height.max(1.0);
        self
    }

    /// Minimum pointer movement before a tab label press becomes a dock drag.
    ///
    /// Default is `6.0`.
    #[must_use]
    pub fn drag_threshold(mut self, threshold: f32) -> Self {
        self.drag_threshold = threshold.max(0.0);
        self
    }

    /// Fraction of pane edge used for edge drop bands (0.0–0.5).
    ///
    /// Default is `0.2`.
    #[must_use]
    pub fn drop_edge_fraction(mut self, fraction: f32) -> Self {
        self.drop_edge_fraction = fraction.clamp(0.0, 0.5);
        self
    }

    /// Fade duration for the tab-bar scrollbar when its visibility changes.
    ///
    /// Default is 0.5 seconds.
    #[must_use]
    pub fn tab_bar_scrollbar_fade_duration(mut self, duration: Duration) -> Self {
        self.tab_bar_scrollbar_fade_duration = duration;
        self
    }

    /// Whether the tab-bar scrollbar fades out when it hides.
    ///
    /// When `false`, the scrollbar snaps visible and hidden instantly.
    /// Default is `true`.
    #[must_use]
    pub fn tab_bar_scrollbar_animated(mut self, animated: bool) -> Self {
        self.tab_bar_scrollbar_animated = animated;
        self
    }

    /// Whether overflowing tab bars show a horizontal scrollbar thumb.
    ///
    /// When `false`, tabs can still be scrolled with the mouse wheel (and Shift+wheel).
    /// Default is `true`.
    #[must_use]
    pub fn tab_bar_show_scrollbar(mut self, show: bool) -> Self {
        self.tab_bar_show_scrollbar = show;
        self
    }

    /// Vertical edge used to attach the optional tab-bar scrollbar.
    ///
    /// Default is [`TabBarScrollbarAttachment::Top`].
    #[must_use]
    pub fn tab_bar_scrollbar_attachment(mut self, attachment: TabBarScrollbarAttachment) -> Self {
        self.tab_bar_scrollbar_attachment = attachment;
        self
    }

    /// Height of the tab bar strip above each pane. Default `30.0`.
    #[must_use]
    pub fn tab_bar_height(mut self, h: f32) -> Self {
        self.tab_bar_height = h.max(1.0);
        self
    }

    /// Horizontal spacing between adjacent tabs. Default `0.0`.
    #[must_use]
    pub fn tab_bar_spacing(mut self, s: f32) -> Self {
        self.tab_bar_spacing = s.max(0.0);
        self
    }

    /// Outer padding of the tab bar: `[vertical, horizontal]`. Default `[0, 0]`.
    #[must_use]
    pub fn tab_bar_padding(mut self, p: [f32; 2]) -> Self {
        self.tab_bar_padding = p;
        self
    }

    /// Font size for tab labels. Default `12.0`.
    #[must_use]
    pub fn tab_text_size(mut self, s: f32) -> Self {
        self.tab_text_size = s.max(1.0);
        self
    }

    /// Font for tab labels. When unset, tab text uses the renderer's
    /// [`default_font`](iced::Settings::default_font) (same as plain `text()`).
    #[must_use]
    pub fn tab_font(mut self, font: impl Into<Renderer::Font>) -> Self {
        self.tab_font = Some(font.into());
        self
    }

    /// Line height for tab labels. When unset, uses the text widget default.
    #[must_use]
    pub fn tab_line_height(mut self, line_height: impl Into<LineHeight>) -> Self {
        self.tab_line_height = Some(line_height.into());
        self
    }

    /// Text shaping for tab labels (e.g. for RTL). When unset, uses the text widget default.
    #[must_use]
    pub fn tab_text_shaping(mut self, shaping: Shaping) -> Self {
        self.tab_text_shaping = Some(shaping);
        self
    }

    /// Inner padding of each tab label: `[vertical, horizontal]`. Default `[0, 10]`.
    #[must_use]
    pub fn tab_padding(mut self, p: [f32; 2]) -> Self {
        self.tab_padding = p;
        self
    }

    /// Height of the accent bar drawn under the active tab. Default `2.0`.
    #[must_use]
    pub fn tab_accent_height(mut self, h: f32) -> Self {
        self.tab_accent_height = h.max(0.0);
        self
    }

    /// Inner padding of the close button: `[vertical, horizontal]`. Default `[0, 0]`.
    #[must_use]
    pub fn close_button_padding(mut self, p: [f32; 2]) -> Self {
        self.close_button_padding = p;
        self
    }

    /// Square size of the close button. Default `20.0`.
    #[must_use]
    pub fn close_button_size(mut self, s: f32) -> Self {
        self.close_button_size = s.max(1.0);
        self
    }

    /// Space between the close button and the tab edge. Default `6.0`.
    #[must_use]
    pub fn close_button_margin(mut self, m: f32) -> Self {
        self.close_button_margin_right = m.max(0.0);
        self
    }

    /// Visual thickness of the splitter divider line. Default `0.5`.
    #[must_use]
    pub fn splitter_size(mut self, s: f32) -> Self {
        self.splitter_size = s.max(0.0);
        self
    }

    /// Extra gap between panes. Default `10.0`.
    #[must_use]
    pub fn splitter_gap(mut self, g: f32) -> Self {
        self.splitter_gap = g.max(0.0);
        self
    }

    /// Content padding inside each pane. Default `0.0`.
    #[must_use]
    pub fn pane_padding(mut self, p: f32) -> Self {
        self.pane_padding = p.max(0.0);
        self
    }

    /// Height of the scrollbar thumb in overflowing tab bars. Default `6.0`.
    #[must_use]
    pub fn scrollbar_height(mut self, h: f32) -> Self {
        self.scrollbar_height = h.max(1.0);
        self
    }

    /// Width of the vertical insertion marker during tab drag. Default `3.0`.
    #[must_use]
    pub fn insert_marker_width(mut self, w: f32) -> Self {
        self.insert_marker_width = w.max(1.0);
        self
    }

    /// Height of the separator line at the bottom of the tab bar. Default `1.0`.
    #[must_use]
    pub fn separator_height(mut self, h: f32) -> Self {
        self.separator_height = h.max(0.0);
        self
    }

    /// Replace the default close-button SVG with a custom element.
    ///
    /// The closure is called once per closeable tab to produce the icon element
    /// rendered inside the close button. The button wrapper (on_press, padding,
    /// hover background) is unchanged.
    #[must_use]
    pub fn close_icon(
        mut self,
        f: impl Fn() -> Element<'static, Message, Theme, Renderer> + 'static,
    ) -> Self {
        self.close_icon = Some(Rc::new(f));
        self
    }

    /// Replace the default chevron SVG in the overflow button with a custom element.
    ///
    /// The closure is called each frame the overflow button is drawn. The
    /// button background and separator are still drawn by the dock; only the
    /// icon glyph is replaced.
    #[must_use]
    pub fn overflow_icon(
        mut self,
        f: impl Fn() -> Element<'static, Message, Theme, Renderer> + 'static,
    ) -> Self {
        self.overflow_icon = Some(Rc::new(f));
        self
    }

    /// Width of an edge dock's resize grab area.
    ///
    /// Wider than the line it draws, so a thin divider is still easy to grab.
    /// Default `6.0`.
    #[must_use]
    pub fn dock_handle_width(mut self, width: f32) -> Self {
        self.dock_handle_width = width.max(1.0);
        self
    }

    /// Set what each panel adds to its own chrome: a title, a toolbar, a menu.
    ///
    /// Every method on the presentation has a default, so a dock whose panels are
    /// plain needs no implementation — see
    /// [`PlainPanels`](crate::dock::PlainPanels).
    #[must_use]
    pub fn presentation(
        mut self,
        presentation: impl PanelPresentation<K, Message, Theme, Renderer> + 'static,
    ) -> Self {
        self.presentation = Rc::new(presentation);
        self
    }

    /// How a group presents itself.
    ///
    /// [`PanelStyle::Auto`](crate::dock::PanelStyle::Auto), the default, draws a
    /// title bar for a group holding one panel and a strip of tabs for more;
    /// [`PanelStyle::TabBar`](crate::dock::PanelStyle::TabBar) always draws the
    /// strip.
    #[must_use]
    pub fn panel_style(mut self, style: crate::dock::panel::PanelStyle) -> Self {
        self.panel_style = style;
        self
    }

    /// Whether tab bars offer the affordance that collapses a neighbouring dock.
    ///
    /// Default `true`. A dock that is not collapsible never shows one, whatever
    /// this says, because the button would have nothing to do.
    #[must_use]
    pub fn toggle_button_visible(mut self, visible: bool) -> Self {
        self.toggle_button_visible = visible;
        self
    }

    /// Register a callback that fires when a tab close button is clicked,
    /// **before** the tab is removed from the layout. Return the application
    /// message to dispatch. If this hook is set the close action is suppressed;
    /// call [`crate::dock::DockSession::close_panel`] from within the message handler to
    /// perform the close programmatically.
    #[must_use]
    pub fn on_close_requested(mut self, f: impl Fn(K) -> Message + 'static) -> Self {
        self.on_close_requested = Some(Rc::new(f));
        self
    }

    /// # Panics
    ///
    /// Panics when [`on_event`](Self::on_event) was not set.
    #[must_use]
    pub fn build(self) -> Dock<'a, K, Message, Theme, Renderer> {
        if let Some(state) = self.shared_state.as_ref() {
            apply_focus_frame_groups(state, self.focus_frame_groups.as_ref());
        }
        let content: ContentFn<'a, K, Message, Theme, Renderer> = self
            .content
            .unwrap_or_else(|| Box::new(|_| PaneContent::new(widget::text("No content"))));
        let on_event: Rc<dyn Fn(DockEvent<K>) -> Message> = self
            .on_event
            .unwrap_or_else(|| Rc::new(|_| panic!("dock().on_event(...) required")));
        Dock {
            content,
            modified: self.modified,
            tooltip: self.tooltip,
            on_event,
            external_state: self.shared_state,
            class: self
                .class
                .unwrap_or_else(|| Rc::new(<Theme as Catalog>::default())),
            root: Element::new(widget::space::Space::new()),
            tab_bar_height: self.tab_bar_height,
            tab_bar_spacing: self.tab_bar_spacing,
            tab_bar_padding: self.tab_bar_padding,
            tab_text_size: self.tab_text_size,
            tab_font: self.tab_font,
            tab_line_height: self.tab_line_height,
            tab_text_shaping: self.tab_text_shaping,
            tab_padding: self.tab_padding,
            tab_accent_height: self.tab_accent_height,
            close_button_size: self.close_button_size,
            close_button_margin_right: self.close_button_margin_right,
            close_button_padding: self.close_button_padding,
            splitter_size: self.splitter_size,
            splitter_gap: self.splitter_gap,
            pane_padding: self.pane_padding,
            scrollbar_height: self.scrollbar_height,
            scrollbar_thumb_min_width: self.scrollbar_thumb_min_width,
            insert_marker_width: self.insert_marker_width,
            separator_height: self.separator_height,
            min_pane_width: self.min_pane_width,
            min_pane_height: self.min_pane_height,
            drag_threshold: self.drag_threshold,
            drop_edge_fraction: self.drop_edge_fraction,
            tab_bar_scrollbar_fade_duration: self.tab_bar_scrollbar_fade_duration,
            tab_bar_scrollbar_animated: self.tab_bar_scrollbar_animated,
            tab_bar_show_scrollbar: self.tab_bar_show_scrollbar,
            tab_bar_scrollbar_attachment: self.tab_bar_scrollbar_attachment,
            tab_tooltip_delay: self.tab_tooltip_delay,
            focus_frame_groups: self.focus_frame_groups.clone(),
            close_icon: self.close_icon,
            overflow_icon: self.overflow_icon,
            on_close_requested: self.on_close_requested,
            dock_handle_width: self.dock_handle_width,
            presentation: self.presentation,
            panel_style: self.panel_style,
            toggle_button_visible: self.toggle_button_visible,
            collapsed_panes: Rc::new(RefCell::new(HashSet::new())),
            alone_panes: Rc::new(RefCell::new(HashSet::new())),
        }
    }
}

/// Push the focus-frame group filter into shared dock state.
///
/// A no-op when the filter is unchanged, so calling it on every `view` pass is free.
fn apply_focus_frame_groups<K>(
    state: &Rc<RefCell<DockWidgetState<K>>>,
    groups: Option<&HashSet<String>>,
) {
    state.borrow_mut().set_focus_frame_groups(groups.cloned());
}

/// Create a [`DockBuilder`] for constructing a [`Dock`] widget.
///
/// This is the primary entry point for building a dock layout in your view
/// function. See [`DockBuilder`] for the full set of configuration options.
#[must_use]
pub fn dock<'a, K, Message, Theme, Renderer>() -> DockBuilder<'a, K, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + kit_menu::Catalog
        + menu::Catalog
        + svg::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer
        + advanced::text::Renderer<Font = iced::Font>
        + advanced::svg::Renderer
        + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    for<'c> <Theme as svg::Catalog>::Class<'c>: From<svg::StyleFn<'c, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    DockBuilder::default()
}

impl<K, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Dock<'_, K, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + kit_menu::Catalog
        + menu::Catalog
        + svg::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer
        + advanced::text::Renderer<Font = iced::Font>
        + advanced::svg::Renderer
        + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    for<'c> <Theme as svg::Catalog>::Class<'c>: From<svg::StyleFn<'c, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    fn tag(&self) -> Tag {
        Tag::of::<DockTreeHolder<K, Theme>>()
    }

    fn state(&self) -> State {
        let dock_state = self
            .external_state
            .clone()
            .unwrap_or_else(|| Rc::new(RefCell::new(DockWidgetState::default())));
        State::new(DockTreeHolder::<K, Theme> {
            dock_state,
            resolved_theme: Rc::new(RefCell::new(None)),
        })
    }

    fn diff(&self, _tree: &mut Tree) {
        // Deferred to layout(), following the responsive() pattern.
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fill,
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let holder = tree.state.downcast_ref::<DockTreeHolder<K, Theme>>();
        let dock_state = Rc::clone(&holder.dock_state);
        dock_state.borrow_mut().commit_layout();

        // The drop geometry is *not* cleared here. `layout` runs before `update` — and
        // `update` is where a drop resolves — so clearing here would leave `finish_drag`
        // reading an empty buffer on the very frame the user let go. It is cleared in
        // `draw` instead, next to the pass that refills it.
        self.rebuild_root(tree);

        let size = limits.max();
        if let Some(child_tree) = tree.children.first_mut() {
            let child_node = self
                .root
                .as_widget_mut()
                .layout(child_tree, renderer, limits);
            layout::Node::with_children(size, vec![child_node])
        } else {
            layout::Node::new(size)
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let holder = tree.state.downcast_ref::<DockTreeHolder<K, Theme>>();
        *holder.resolved_theme.borrow_mut() = Some(theme.clone());

        // A fresh frame's geometry. The children are drawn below and register theirs,
        // so by the time this returns the buffers describe the layout that is on
        // screen — and the events of the *next* frame resolve against exactly that.
        //
        // Cleared here rather than in `layout`: a layout pass can run in the middle of
        // the `update` that handles a drop, and emptying the buffers then would leave
        // the drop with nothing to resolve against.
        {
            let mut state = holder.dock_state.borrow_mut();
            state.pane_bounds.clear();
            state.drop_targets.clear();
            state.tab_bar_targets.clear();
        }

        let dock_style = Catalog::style(theme, &self.class);
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            dock_style.background.color,
        );


        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first() else {
            return;
        };
        self.root.as_widget().draw(
            child_tree,
            renderer,
            theme,
            style,
            child_layout,
            cursor,
            viewport,
        );

    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first_mut() else {
            return;
        };

        self.root.as_widget_mut().update(
            child_tree,
            event,
            child_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let dock_state = Rc::clone(
            &tree
                .state
                .downcast_ref::<DockTreeHolder<K, Theme>>()
                .dock_state,
        );

        if dock_state.borrow().layout_dirty {
            // Rebuild only in `layout()` (responsive pattern). Rebuilding here desyncs
            // the widget tree from the current layout and can panic nested containers
            // during `mouse_interaction` on the same frame as a tab switch.
            shell.invalidate_layout();
            shell.invalidate_widgets();
        }

        if dock_state.borrow().focus_dirty {
            dock_state.borrow_mut().focus_dirty = false;
            shell.request_redraw();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let holder = tree.state.downcast_ref::<DockTreeHolder<K, Theme>>();
        if holder.dock_state.borrow().drag.is_some() {
            return mouse::Interaction::Grab;
        }

        let Some(child_layout) = layout.children().next() else {
            return mouse::Interaction::default();
        };
        let Some(child_tree) = tree.children.first() else {
            return mouse::Interaction::default();
        };
        self.root.as_widget().mouse_interaction(
            child_tree,
            child_layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        let Some(child_layout) = layout.children().next() else {
            return;
        };
        let Some(child_tree) = tree.children.first_mut() else {
            return;
        };
        self.root
            .as_widget_mut()
            .operate(child_tree, child_layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let child_layout = layout.children().next()?;
        let child_tree = tree.children.first_mut()?;
        self.root
            .as_widget_mut()
            .overlay(child_tree, child_layout, renderer, viewport, translation)
    }
}

impl<'a, K, Message, Theme, Renderer> From<Dock<'a, K, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    K: Copy + 'static,
    Message: Clone + 'static,
    Theme: Catalog
        + button::Catalog
        + container::Catalog
        + iced_text::Catalog
        + kit_menu::Catalog
        + menu::Catalog
        + svg::Catalog
        + Clone
        + PartialEq
        + 'static,
    Renderer: advanced::Renderer
        + advanced::text::Renderer<Font = iced::Font>
        + advanced::svg::Renderer
        + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    for<'c> <Theme as svg::Catalog>::Class<'c>: From<svg::StyleFn<'c, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    fn from(widget: Dock<'a, K, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}

/// The group a left dock's toggle belongs to: the top-most one in the centre.
///
/// The button collapses the dock that sits against the centre's left edge, so it
/// belongs in the group nearest that edge — which is the first child of a
/// horizontal split, walked down to a leaf.
fn left_top_group<K>(layout: &ModelLayout<K>) -> Option<NodeId> {
    fn walk<K>(layout: &ModelLayout<K>, node: NodeId) -> Option<NodeId> {
        match layout.kind(node)? {
            NodeKind::Pane(_) => Some(node),
            NodeKind::Proportional(pg) => {
                let first = *pg.children.first()?;
                walk(layout, first)
            }
            NodeKind::Panel(_) | NodeKind::Root(_) => None,
        }
    }
    walk(layout, layout.root_child()?)
}

/// The group a right dock's toggle belongs to: the last child of a horizontal
/// split, the first of a vertical one.
///
/// A right dock's button sits on the right of the centre's top row, so it belongs
/// to the group against that edge. A vertical split has no right-hand side, so its
/// first child is the group a user would call the top one.
fn right_top_group<K>(layout: &ModelLayout<K>) -> Option<NodeId> {
    fn walk<K>(layout: &ModelLayout<K>, node: NodeId) -> Option<NodeId> {
        match layout.kind(node)? {
            NodeKind::Pane(_) => Some(node),
            NodeKind::Proportional(pg) => {
                let next = match pg.axis {
                    Axis::Horizontal => *pg.children.last()?,
                    Axis::Vertical => *pg.children.first()?,
                };
                walk(layout, next)
            }
            NodeKind::Panel(_) | NodeKind::Root(_) => None,
        }
    }
    walk(layout, layout.root_child()?)
}

/// Every pane node in a tree, in preorder.
fn panes_in_tree<K>(layout: &ModelLayout<K>) -> Vec<NodeId> {
    fn walk<K>(layout: &ModelLayout<K>, node: NodeId, found: &mut Vec<NodeId>) {
        match layout.kind(node) {
            Some(NodeKind::Pane(_)) => found.push(node),
            Some(NodeKind::Proportional(pg)) => {
                for &child in &pg.children {
                    walk(layout, child, found);
                }
            }
            Some(NodeKind::Panel(_) | NodeKind::Root(_)) | None => {}
        }
    }
    let mut found = Vec::new();
    if let Some(root) = layout.root_child() {
        walk(layout, root, &mut found);
    }
    found
}
