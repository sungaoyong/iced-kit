// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

use std::cell::RefCell;
use std::rc::Rc;

use crate::widgets::overlay::menu as kit_menu;
use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{self, Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::time::Duration;
use iced::touch;
use iced::widget::overlay::menu;
use iced::widget::text::{LineHeight, Shaping};
use iced::widget::{button, container, svg, text as iced_text};
use iced::{Border, Element, Event, Length, Rectangle, Size, Vector};

use crate::dock::manager::{DockManager, DragSession, DropZone, TabBarTarget};
use crate::dock::model::NodeId;
use crate::dock::style::{self, Catalog, DockStyle};
use crate::dock::widget::action::{DockAction, TabAction};
use crate::dock::widget::dock::TabBarScrollbarAttachment;
use crate::dock::widget::state::DockWidgetState;
use crate::dock::widget::tab_strip::{self, TabStrip};
use crate::dock::widget::{compose, controls};

fn drop_zone_rect(bounds: Rectangle, zone: DropZone, edge: f32) -> Rectangle {
    let w = bounds.width;
    let h = bounds.height;
    match zone {
        DropZone::Left => Rectangle {
            width: w * edge,
            ..bounds
        },
        DropZone::Right => Rectangle {
            x: bounds.x + w * (1.0 - edge),
            width: w * edge,
            ..bounds
        },
        DropZone::Top => Rectangle {
            height: h * edge,
            ..bounds
        },
        DropZone::Bottom => Rectangle {
            y: bounds.y + h * (1.0 - edge),
            height: h * edge,
            ..bounds
        },
        DropZone::Center => Rectangle {
            x: bounds.x + w * edge,
            y: bounds.y + h * edge,
            width: w * (1.0 - 2.0 * edge),
            height: h * (1.0 - 2.0 * edge),
        },
    }
}

fn pane_inset(pane_padding: f32, border_width: f32) -> f32 {
    pane_padding + border_width
}

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: NodeId,
    pub title: String,
    /// A short label for a group with no room for the full title.
    pub tab_name: Option<String>,
    pub can_close: bool,
    pub can_drag: bool,
    pub can_zoom: bool,
    pub is_modified: bool,
    pub tooltip: Option<String>,
}

impl TabInfo {
    /// The label a tab should show: the short name when there is one, the title
    /// otherwise.
    #[must_use]
    pub fn label(&self) -> &str {
        self.tab_name.as_deref().unwrap_or(&self.title)
    }
}

#[derive(Default)]
struct TabDockState;

fn tab_insert_is_noop(
    session: DragSession,
    pane_id: NodeId,
    index: usize,
    tabs: &[TabInfo],
) -> bool {
    if session.source_pane != pane_id {
        return false;
    }
    let Some(from) = tabs.iter().position(|t| t.id == session.source_panel) else {
        return false;
    };
    from == index || from + 1 == index
}

/// Index of the first control child, after the bar and the content.
const FIRST_CONTROL: usize = 2;

pub struct TabDock<'a, K, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
    Renderer: advanced::Renderer + advanced::svg::Renderer,
{
    dock_state: Rc<RefCell<DockWidgetState<K>>>,
    pane_id: NodeId,
    tabs: Vec<TabInfo>,
    active_tab: NodeId,
    /// The bar across the top. Either a scrolling tab strip, or a row holding a
    /// title and the dock's controls — composed by the dock, because the strip
    /// cannot host them: it draws its whole row into a translated, clipped layer.
    bar: Element<'a, Message, Theme, Renderer>,
    /// Whether `bar` is a tab strip.
    ///
    /// A title bar has no tabs to scroll, hover, drag or select, so every
    /// strip-specific step is skipped for it — and those steps downcast the bar's
    /// tree state, which would fail rather than quietly do nothing.
    is_strip: bool,
    content: Element<'a, Message, Theme, Renderer>,
    /// Controls before the bar: the dock toggles.
    leading: Vec<Element<'a, Message, Theme, Renderer>>,
    /// Controls after the bar: the panel's toolbar, then zoom and the menu.
    trailing: Vec<Element<'a, Message, Theme, Renderer>>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    theme: Rc<RefCell<Option<Theme>>>,
    tab_bar_height: f32,
    pane_padding: f32,
    drop_edge_fraction: f32,
    tab_bar_show_scrollbar: bool,
    /// Whether the group pads its content. `false` for a panel that draws its own
    /// edges and wants the whole pane.
    inner_padding: bool,
    /// Whether this group's tab started the drag that is in flight.
    ///
    /// Read from the session at the moment of the event rather than stored, and used
    /// to decide who ends the drag: every group is handed every pointer event, so
    /// without this the release would be acted on by all of them.
    is_drag_source: bool,
    /// Whether the group is collapsed — what every group of a closed dock is.
    is_collapsed: bool,
}

impl<'a, K, Message, Theme, Renderer> TabDock<'a, K, Message, Theme, Renderer>
where
    K: 'static,
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
    Renderer: advanced::Renderer + advanced::text::Renderer + advanced::svg::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    for<'c> <Theme as svg::Catalog>::Class<'c>: From<svg::StyleFn<'c, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    /// Build the group's bar: a strip of tabs, or a single panel's title.
    ///
    /// The argument list is long because a group is where every configurable metric
    /// of a tab bar meets. They are threaded through rather than gathered into a
    /// struct so that `DockBuilder` stays the one place a setting is named.
    #[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
    pub(crate) fn new(
        dock_state: Rc<RefCell<DockWidgetState<K>>>,
        pane_id: NodeId,
        tabs: Vec<TabInfo>,
        active_tab: NodeId,
        content: Element<'a, Message, Theme, Renderer>,
        title: Option<Element<'a, Message, Theme, Renderer>>,
        leading: Vec<Element<'a, Message, Theme, Renderer>>,
        trailing: Vec<Element<'a, Message, Theme, Renderer>>,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        class: Rc<<Theme as Catalog>::Class<'static>>,
        theme: Rc<RefCell<Option<Theme>>>,
        inner_padding: bool,
        is_collapsed: bool,
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
        pane_padding: f32,
        scrollbar_height: f32,
        scrollbar_thumb_min_width: f32,
        insert_marker_width: f32,
        separator_height: f32,
        drag_threshold: f32,
        drop_edge_fraction: f32,
        tab_bar_scrollbar_fade_duration: Duration,
        tab_bar_scrollbar_animated: bool,
        tab_bar_show_scrollbar: bool,
        tab_bar_scrollbar_attachment: TabBarScrollbarAttachment,
        tab_tooltip_delay: Duration,
        close_icon: Option<Rc<dyn Fn() -> Element<'static, Message, Theme, Renderer>>>,
        overflow_icon: Option<Rc<dyn Fn() -> Element<'static, Message, Theme, Renderer>>>,
    ) -> Self {
        let is_strip = title.is_none();
        // The bar is the strip or the title *itself*, never a row wrapping either.
        // A wrapper would take over tree child 0, and the strip's own state — its
        // scroll offset, its hovered tab — would then be looked up in the wrong
        // place, which fails the downcast rather than degrading quietly. The
        // controls are laid out beside the bar by this widget instead, as children
        // of their own.
        let bar: Element<'a, Message, Theme, Renderer> = match title {
            Some(title) => title,
            None => TabStrip::new(
                pane_id,
                tabs.clone(),
                active_tab,
                Rc::clone(&on_event),
                Rc::clone(&class),
                Rc::clone(&theme),
                tab_bar_height,
                tab_bar_spacing,
                tab_bar_padding,
                tab_text_size,
                tab_font,
                tab_line_height,
                tab_text_shaping,
                tab_padding,
                tab_accent_height,
                close_button_size,
                close_button_margin_right,
                close_button_padding,
                scrollbar_height,
                scrollbar_thumb_min_width,
                insert_marker_width,
                separator_height,
                drag_threshold,
                drop_edge_fraction,
                tab_bar_scrollbar_fade_duration,
                tab_bar_scrollbar_animated,
                tab_bar_show_scrollbar,
                tab_bar_scrollbar_attachment,
                tab_tooltip_delay,
                close_icon,
                overflow_icon,
            )
            .into(),
        };
        Self {
            dock_state,
            pane_id,
            tabs,
            active_tab,
            bar,
            is_strip,
            content,
            leading,
            trailing,
            on_event,
            class,
            theme,
            tab_bar_height,
            pane_padding,
            drop_edge_fraction,
            tab_bar_show_scrollbar,
            inner_padding,
            is_collapsed,
            is_drag_source: false,
        }
    }
}

impl<K, Message, Theme, Renderer> TabDock<'_, K, Message, Theme, Renderer>
where
    K: 'static,
    Theme: Catalog + Clone + menu::Catalog + kit_menu::Catalog + 'static,
    Renderer: advanced::Renderer + advanced::svg::Renderer,
{
    fn resolved_theme(&self) -> Option<Theme> {
        self.theme.borrow().clone()
    }

    fn layout_style(&self, theme: &Theme) -> DockStyle {
        Catalog::style(theme, &self.class)
    }

    fn layout_style_or_default(&self) -> DockStyle {
        match self.resolved_theme() {
            Some(t) => Catalog::style(&t, &self.class),
            None => style::default(&iced::Theme::Dark),
        }
    }

    fn is_dragging(&self, tree: &Tree) -> bool {
        self.dock_state.borrow().drag.is_some()
            || (self.is_strip && tab_strip::is_dragging::<Theme>(tree.children.first()))
    }

    /// Cursor handed to the pane content. A tab drag owns the pointer, so the
    /// content is blinded for its duration: widgets see "cursor not over me" and
    /// clear their hover state instead of freezing it. Blocking pointer *events*
    /// alone is not enough — `button` recomputes its status from the cursor on
    /// every `RedrawRequested`, and canvases hover straight out of `draw`.
    fn content_cursor(&self, cursor: Cursor) -> Cursor {
        if self.dock_state.borrow().drag.is_some() {
            Cursor::Unavailable
        } else {
            cursor
        }
    }

    fn register_drop_target(&self, bounds: Rectangle) {
        self.dock_state
            .borrow_mut()
            .drop_targets
            .push((self.pane_id, bounds));
    }

    fn register_tab_bar_target(&self, bounds: Rectangle, insert_x: Vec<f32>, scroll_offset: f32) {
        self.dock_state
            .borrow_mut()
            .tab_bar_targets
            .push(TabBarTarget {
                pane: self.pane_id,
                bounds,
                insert_x,
                scroll_offset,
            });
    }

    fn register_pane_bounds(&self, bounds: Rectangle) {
        self.dock_state
            .borrow_mut()
            .pane_bounds
            .push((self.pane_id, bounds));
    }
}

impl<K, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for TabDock<'_, K, Message, Theme, Renderer>
where
    K: 'static,
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
    Renderer: advanced::Renderer + advanced::text::Renderer + advanced::svg::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    for<'c> <Theme as svg::Catalog>::Class<'c>: From<svg::StyleFn<'c, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    fn tag(&self) -> Tag {
        Tag::of::<TabDockState>()
    }

    fn state(&self) -> State {
        State::new(TabDockState)
    }

    fn children(&self) -> Vec<Tree> {
        {
            let mut trees = vec![Tree::new(&self.bar), Tree::new(&self.content)];
            trees.extend(self.leading.iter().map(Tree::new));
            trees.extend(self.trailing.iter().map(Tree::new));
            trees
        }
    }

    fn diff(&self, tree: &mut Tree) {
        // Diffed in place rather than rebuilt: a tab strip keeps the press, hover
        // and drag state that spans the frames between a press and its release, and
        // a fresh `Tree` would wipe it every pass. Only the control tail is
        // rebuilt, and only when its length changes, because a panel gaining or
        // losing a toolbar button shifts every later child's state by one.
        if tree.children.len() < FIRST_CONTROL {
            tree.children.clear();
            tree.children.push(Tree::new(&self.bar));
            tree.children.push(Tree::new(&self.content));
        } else {
            tree.children[0].diff(&self.bar);
            tree.children[1].diff(&self.content);
        }

        let expected = FIRST_CONTROL + self.leading.len() + self.trailing.len();
        if tree.children.len() == expected {
            for (i, element) in self.leading.iter().enumerate() {
                tree.children[FIRST_CONTROL + i].diff(element);
            }
            for (i, element) in self.trailing.iter().enumerate() {
                tree.children[FIRST_CONTROL + self.leading.len() + i].diff(element);
            }
        } else {
            tree.children.truncate(FIRST_CONTROL);
            tree.children.extend(self.leading.iter().map(Tree::new));
            tree.children.extend(self.trailing.iter().map(Tree::new));
        }
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
        let style = self.layout_style_or_default();
        let max = limits.max();
        let inset = pane_inset(self.pane_padding, style.window.border.width);
        let inner_w = (max.width - 2.0 * inset).max(0.0);
        let inner_h = (max.height - 2.0 * inset).max(0.0);
        let bar_h = self.tab_bar_height;
        let control = style.control.clone();
        let leading_w = controls::controls_width(self.leading.len(), &control);
        let trailing_w = controls::controls_width(self.trailing.len(), &control);

        // The bar gets what the controls leave, so a crowded bar scrolls its tabs
        // rather than letting them run under a button.
        let bar_w = if self.is_strip {
            (inner_w - leading_w - trailing_w).max(0.0)
        } else {
            inner_w
        };
        let mut bar_node = compose::child_layout(
            &mut self.bar,
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(bar_w, bar_h)),
        );
        // A title bar is placed after the leading controls, because it holds the
        // panel's name and reads as belonging with them; a strip keeps the whole
        // width, with the controls drawn over the space it was told to leave.
        let bar_x = if self.is_strip {
            inset
        } else {
            inset + leading_w
        };
        bar_node.move_to_mut((bar_x, inset));

        // A collapsed group is its bar and nothing else: the content is laid out
        // at zero height and contributes none, which leaves the strip flush in the
        // space the dock keeps for it.
        // The panel's content is inset only when it asked to be: a panel that
        // draws its own edges — an editor, an image — wants the whole pane, and its
        // chrome already separates it from the bar.
        let content_pad = if self.inner_padding {
            self.pane_padding
        } else {
            0.0
        };
        let content_h = if self.is_collapsed {
            0.0
        } else {
            (inner_h - bar_h - 2.0 * content_pad).max(0.0)
        };
        let content_w = (inner_w - 2.0 * content_pad).max(0.0);
        // The content region's size, given as *both* bounds so a `Fill` child (or a
        // shrink-to-fit one like a bare `text`) still reports the whole region. Its
        // bounds are what a drop is measured against, so a collapsed box would leave
        // a pane that draws fine but cannot be dropped into.
        let content_extent = Size::new(content_w, content_h);
        let mut content_node = compose::child_layout(
            &mut self.content,
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(content_extent, content_extent),
        );
        content_node.move_to_mut((inset + content_pad, inset + bar_h + content_pad));
        let mut nodes = vec![bar_node, content_node];

        let size = control.size.min(bar_h);
        let mut x = inset;
        for (i, element) in self.leading.iter_mut().enumerate() {
            let child = &mut tree.children[FIRST_CONTROL + i];
            let mut node = compose::child_layout(
                element,
                child,
                renderer,
                &layout::Limits::new(Size::new(size, size), Size::new(size, size)),
            );
            node.move_to_mut((x, inset + (bar_h - size) / 2.0));
            x += size + control.gap;
            nodes.push(node);
        }

        let mut x = inset + inner_w - trailing_w;
        for (i, element) in self.trailing.iter_mut().enumerate() {
            let child = &mut tree.children[FIRST_CONTROL + self.leading.len() + i];
            let mut node = compose::child_layout(
                element,
                child,
                renderer,
                &layout::Limits::new(Size::new(size, size), Size::new(size, size)),
            );
            node.move_to_mut((x, inset + (bar_h - size) / 2.0));
            x += size + control.gap;
            nodes.push(node);
        }

        // Sized to the slot it was given, not to the union of its children: the group
        // is what the split laid out, and reporting a smaller box would make every
        // pane's own drop region smaller than the pane.
        layout::Node::with_children(Size::new(max.width, max.height), nodes)
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
        // Absolute bounds, which is what a drop is measured against.
        //
        // Registered while drawing rather than while laying out: a node's own
        // `bounds()` is relative to its parent, so two panes side by side both report
        // the same origin. `draw` is handed the resolved layout, and it runs after the
        // last layout pass of a frame — so what is recorded here is the arrangement the
        // user is looking at.
        self.register_pane_bounds(layout.bounds());
        if let Some(content_layout) = layout.children().nth(1) {
            self.register_drop_target(content_layout.bounds());
        }
        if self.is_strip {
            if let Some(bar_layout) = layout.children().next() {
                let (insert_x, scroll) = tab_strip::insert_slots::<Theme>(&tree.children[0]);
                if !insert_x.is_empty() {
                    self.register_tab_bar_target(bar_layout.bounds(), insert_x, scroll);
                }
            }
        }

        let dock_style = self.layout_style(theme);

        let pane_bounds = layout.bounds();
        let window = &dock_style.window;
        let is_focused = self.dock_state.borrow().focus_frame_pane == Some(self.pane_id);
        let border = if is_focused {
            window.focused_border.unwrap_or(window.border)
        } else {
            window.border
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: pane_bounds,
                border,
                ..renderer::Quad::default()
            },
            window.background,
        );

        if let Some(tab_layout) = layout.children().next() {
            compose::child_draw(
                &self.bar,
                &tree.children[0],
                renderer,
                theme,
                style,
                tab_layout,
                cursor,
                viewport,
            );
        }
        // The dock's controls are drawn after the bar and the content so their
        // glyphs are not covered by a tab that runs to the pane's edge.
        let controls: Vec<&Element<'_, Message, Theme, Renderer>> =
            self.leading.iter().chain(self.trailing.iter()).collect();
        for (i, element) in controls.iter().enumerate() {
            if let (Some(child_layout), Some(child_tree)) = (
                layout.children().nth(FIRST_CONTROL + i),
                tree.children.get(FIRST_CONTROL + i),
            ) {
                compose::child_draw(
                    element,
                    child_tree,
                    renderer,
                    theme,
                    style,
                    child_layout,
                    cursor,
                    viewport,
                );
            }
        }

        if let Some(content_layout) = layout.children().nth(1) {
            compose::child_draw(
                &self.content,
                &tree.children[1],
                renderer,
                theme,
                style,
                content_layout,
                self.content_cursor(cursor),
                viewport,
            );
        }

        let drag_session = self.dock_state.borrow().drag;
        let show_overlay = self.is_dragging(tree);

        if show_overlay {
            if let Some(content_layout) = layout.children().nth(1) {
                let bounds = content_layout.bounds();
                let show_content_overlay = drag_session.is_some_and(|s| {
                    s.tab_insert.is_none() && s.hover_target == Some(self.pane_id)
                });
                let zone = show_content_overlay
                    .then(|| {
                        drag_session
                            .and_then(|s| s.operation)
                            .and_then(|_| cursor.position())
                            .and_then(|point| {
                                DockManager::hit_test_drop_zone(
                                    bounds,
                                    point,
                                    self.drop_edge_fraction,
                                )
                            })
                            .or_else(|| {
                                let point = cursor.position_over(bounds)?;
                                DockManager::hit_test_drop_zone(
                                    bounds,
                                    point,
                                    self.drop_edge_fraction,
                                )
                            })
                    })
                    .flatten();

                if let Some(zone) = zone {
                    let blocked = zone == DropZone::Center
                        && drag_session.is_some_and(|s| {
                            let state = self.dock_state.borrow();
                            !DockManager.groups_compatible(
                                &state.layout,
                                s.source_panel,
                                self.pane_id,
                            )
                        });
                    let (highlight, outline) = if blocked {
                        (
                            dock_style.drop_overlay.blocked_color,
                            dock_style.drop_overlay.blocked_border_color,
                        )
                    } else {
                        (
                            dock_style.drop_overlay.color,
                            dock_style.drop_overlay.border_color,
                        )
                    };
                    let zone_bounds = drop_zone_rect(bounds, zone, self.drop_edge_fraction);
                    // Own layer: inside a single layer the renderer draws quads before meshes
                    // and text, so a plain fill_quad here ends up *under* canvas-based content.
                    renderer.with_layer(bounds, |renderer| {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: zone_bounds,
                                border: Border {
                                    width: dock_style.drop_overlay.border_width,
                                    color: outline,
                                    radius: 0.0.into(),
                                },
                                ..Default::default()
                            },
                            highlight,
                        );
                    });
                }
            }
        }
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
        // The controls get first refusal on every event: a press on a toggle must
        // not also reach the pane's content, which would focus it as a side effect.
        // A control that claims the event calls `capture_event` itself, and the
        // loops below honour that by not forwarding.
        {
            let control_count = self.leading.len() + self.trailing.len();
            for i in 0..control_count {
                let Some(child_layout) = layout.children().nth(FIRST_CONTROL + i) else {
                    continue;
                };
                let Some(child_tree) = tree.children.get_mut(FIRST_CONTROL + i) else {
                    continue;
                };
                let element = if i < self.leading.len() {
                    self.leading.get_mut(i)
                } else {
                    self.trailing.get_mut(i - self.leading.len())
                };
                let Some(element) = element else { continue };
                compose::child_update(
                    element,
                    child_tree,
                    event,
                    child_layout,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
        }

        let dragging = self.is_dragging(tree);

        // Whether *this* group started the drag in flight. A tab drag records its pane
        // in the session; a title drag does not, so one started from a title bar has
        // no owning group here and only the title itself ends it.
        self.is_drag_source = self
            .dock_state
            .borrow()
            .drag
            .is_some_and(|session| session.source_pane == self.pane_id);
        let is_picked = self.is_drag_source;

        // The bar is always given the events — a title bar's title is a drag source,
        // so it needs them as much as a strip does. Only the strip's own bookkeeping
        // is conditional: it downcasts the bar's tree state, which would fail on a
        // title bar rather than quietly do nothing.
        if let Some(tab_layout) = layout.children().next() {
            if self.is_strip {
                let suppress_hover = self.dock_state.borrow().drag.is_some();
                if tab_strip::set_suppress_hover::<Theme>(&mut tree.children[0], suppress_hover) {
                    shell.request_redraw();
                }
            }

            compose::child_update(
                &mut self.bar,
                &mut tree.children[0],
                event,
                tab_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            if self.is_strip {
                tab_strip::sync_hover_in_tree::<_, Theme>(
                    &mut tree.children[0],
                    tab_layout.bounds(),
                    cursor,
                    self.tab_bar_show_scrollbar,
                    shell,
                );

                let (marker_index, marker_blocked) = {
                    let state = self.dock_state.borrow();
                    let result = state.drag.and_then(|session| {
                        let (pane, index) = session.tab_insert?;
                        (pane == self.pane_id
                            && !tab_insert_is_noop(session, self.pane_id, index, &self.tabs))
                        .then_some((
                            index,
                            session.source_pane != self.pane_id
                                && !DockManager.groups_compatible(
                                    &state.layout,
                                    session.source_panel,
                                    self.pane_id,
                                ),
                        ))
                    });
                    result.map_or((None, false), |(index, blocked)| (Some(index), blocked))
                };
                if tab_strip::set_insert_marker_index::<Theme>(&mut tree.children[0], marker_index)
                {
                    shell.request_redraw();
                }
                if tab_strip::set_drag_blocked::<Theme>(&mut tree.children[0], marker_blocked) {
                    shell.request_redraw();
                }
            }
        }
        if let Some(content_layout) = layout.children().nth(1) {
            // Keep forwarding while a drag is in progress, but with a blinded cursor:
            // widgets must clear hover rather than keep the highlight they had when the
            // drag began, and a press with no cursor over them is a no-op.
            let content_cursor = self.content_cursor(cursor);
            if !is_picked {
                compose::child_update(
                    &mut self.content,
                    &mut tree.children[1],
                    event,
                    content_layout,
                    content_cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }

            if !dragging {
                match event {
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                    | Event::Touch(touch::Event::FingerPressed { .. }) => {
                        let bounds = content_layout.bounds();
                        if cursor.position_over(bounds).is_some() {
                            shell.capture_event();
                            shell.publish((self.on_event)(DockAction::PaneFocused {
                                pane: self.pane_id,
                                panel: Some(self.active_tab),
                            }));
                            shell.request_redraw();
                        }
                    }
                    _ => {}
                }
            }
        }

        // Only the group whose *tab* began the drag ends it here. The session flag is
        // no help: it says a drag is in flight, not who is driving it, and this widget
        // is handed every event for every pane — so ending on the flag alone would end
        // one drag several times, applying the same layout change over and over.
        if self.is_drag_source
            && matches!(
                event,
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
            )
            && self.dock_state.borrow().drag.is_some()
        {
            if let Some(pos) = cursor.position() {
                shell.publish((self.on_event)(DockAction::Tab(TabAction::DragEnded {
                    cursor: pos,
                })));
            } else {
                shell.publish((self.on_event)(DockAction::Tab(TabAction::DragCancelled)));
            }
            shell.invalidate_layout();
            shell.invalidate_widgets();
            shell.request_redraw();
        }

        if dragging && self.is_drag_source {
            if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
                if let Some(pos) = cursor.position() {
                    shell.publish((self.on_event)(DockAction::Tab(TabAction::DragMoved {
                        cursor: pos,
                    })));
                    shell.request_redraw();
                }
            }
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
        if self.dock_state.borrow().drag.is_some()
            || (self.is_strip && tab_strip::is_tab_drag_active::<Theme>(tree.children.first()))
        {
            return mouse::Interaction::Grab;
        }

        let mut interaction = mouse::Interaction::None;
        if let Some(tab_layout) = layout.children().next() {
            interaction = interaction.max(compose::child_mouse_interaction(
                &self.bar,
                &tree.children[0],
                tab_layout,
                cursor,
                viewport,
                renderer,
            ));
        }
        if let Some(content_layout) = layout.children().nth(1) {
            interaction = interaction.max(compose::child_mouse_interaction(
                &self.content,
                &tree.children[1],
                content_layout,
                cursor,
                viewport,
                renderer,
            ));
        }
        for (i, element) in self.leading.iter().chain(self.trailing.iter()).enumerate() {
            if let (Some(child_layout), Some(child_tree)) = (
                layout.children().nth(FIRST_CONTROL + i),
                tree.children.get(FIRST_CONTROL + i),
            ) {
                interaction = interaction.max(compose::child_mouse_interaction(
                    element,
                    child_tree,
                    child_layout,
                    cursor,
                    viewport,
                    renderer,
                ));
            }
        }
        interaction
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(tab_layout) = layout.children().next() {
            compose::child_operate(
                &mut self.bar,
                &mut tree.children[0],
                tab_layout,
                renderer,
                operation,
            );
        }
        if let Some(content_layout) = layout.children().nth(1) {
            compose::child_operate(
                &mut self.content,
                &mut tree.children[1],
                content_layout,
                renderer,
                operation,
            );
        }
        let control_count = self.leading.len() + self.trailing.len();
        for i in 0..control_count {
            let Some(child_layout) = layout.children().nth(FIRST_CONTROL + i) else {
                continue;
            };
            let Some(child_tree) = tree.children.get_mut(FIRST_CONTROL + i) else {
                continue;
            };
            let element = if i < self.leading.len() {
                self.leading.get_mut(i)
            } else {
                self.trailing.get_mut(i - self.leading.len())
            };
            if let Some(element) = element {
                compose::child_operate(element, child_tree, child_layout, renderer, operation);
            }
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut overlays = Vec::new();

        // One split up front, so every child's element and tree state are handed
        // out as a disjoint pair: the widget borrows come from `self`, the tree
        // slices from `children`, and they must not be re-derived inside the loop.
        // The tree is only as long as the last `children()`/`diff()` built it, and
        // an overlay can be asked for in the same frame an overlay was opened —
        // before the control children exist. Bail rather than slice past the end.
        if tree.children.len() < FIRST_CONTROL {
            return None;
        }
        let (head, controls) = tree.children.split_at_mut(FIRST_CONTROL);
        let (bar_tree, content_tree) = head.split_at_mut(1);

        if let Some(tab_layout) = layout.children().next() {
            if let Some(overlay) = self.bar.as_widget_mut().overlay(
                &mut bar_tree[0],
                tab_layout,
                renderer,
                viewport,
                translation,
            ) {
                overlays.push(overlay);
            }
        }
        if let Some(content_layout) = layout.children().nth(1) {
            if let Some(overlay) = self.content.as_widget_mut().overlay(
                &mut content_tree[0],
                content_layout,
                renderer,
                viewport,
                translation,
            ) {
                overlays.push(overlay);
            }
        }

        // The controls can host overlays of their own — the panel menu is one — so
        // they are asked too.
        for (i, (element, child_tree)) in self
            .leading
            .iter_mut()
            .chain(self.trailing.iter_mut())
            .zip(controls.iter_mut())
            .enumerate()
        {
            let Some(child_layout) = layout.children().nth(FIRST_CONTROL + i) else {
                continue;
            };
            if let Some(overlay) = element.as_widget_mut().overlay(
                child_tree,
                child_layout,
                renderer,
                viewport,
                translation,
            ) {
                overlays.push(overlay);
            }
        }
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, K, Message, Theme, Renderer> From<TabDock<'a, K, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    K: 'static,
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
    Renderer: advanced::Renderer + advanced::text::Renderer + advanced::svg::Renderer + 'static,
    <Theme as button::Catalog>::Class<'static>: From<button::StyleFn<'static, Theme>>,
    for<'c> <Theme as svg::Catalog>::Class<'c>: From<svg::StyleFn<'c, Theme>>,
    <Theme as container::Catalog>::Class<'static>: From<container::StyleFn<'static, Theme>>,
    for<'b> <Theme as iced_text::Catalog>::Class<'b>: From<iced_text::StyleFn<'b, Theme>>,
{
    fn from(widget: TabDock<'a, K, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}
