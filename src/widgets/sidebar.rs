//! Sidebar.
//!
//! A vertical navigation panel docked to one side of the window, with a
//! header, a scrollable body of [`SidebarGroup`]s and [`SidebarMenu`]s, and a
//! footer. It is stateless: the caller owns whether the sidebar is collapsed,
//! which item is active and which submenus are open, and receives messages
//! when any of them change — so the same sidebar can be driven by application
//! state, keyboard shortcuts or persistence.
//!
//! # Collapsing
//!
//! [`SidebarCollapsible`] follows the shadcn/ui modes. `Icon` narrows the
//! sidebar to an icon rail, `Offcanvas` slides it out and releases its layout
//! width, and `None` keeps it expanded whatever the collapsed flag says.
//!
//! # Why a sidebar does not open its own menus
//!
//! Right-clicking an item and pressing the header or footer report an intent
//! through [`SidebarMenuItem::on_context`] and
//! [`SidebarHeader::on_dropdown`]. iced has no window-level z-order, so a
//! floating menu is positioned by the application and drawn through the
//! [`Layer`](crate::widgets::overlay::Layer) — the same division of labour as
//! the [`DropdownButton`](crate::widgets::DropdownButton) trigger, and the
//! reason `on_context` carries a plain message rather than the menu contents:
//! the application owns both the open flag and the placement, and rebuilds the
//! menu beside its other view code.

use crate::icons::IconName;
use crate::theme::{Size, Theme};
use crate::widgets::button::{Button, Icon};
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Shell};
// The wrapper widgets below are written against the concrete `iced::Renderer`,
// so the core renderer trait must be in scope for `with_layer` to resolve.
use iced::advanced::Renderer as _;
use iced::time::Instant;
use iced::widget::{button as iced_button, column, container, mouse_area, row, scrollable, text};
use iced::{alignment, Color, Element, Event, Length, Padding, Rectangle, Size as IcedSize};

/// The width the sidebar is drawn at, in logical pixels.
const DEFAULT_WIDTH: f32 = 255.0;

/// The width of an icon-collapsed sidebar, in logical pixels.
const COLLAPSED_WIDTH: f32 = 48.0;

/// The way a [`Sidebar`] behaves when it is collapsed.
///
/// This follows the shadcn/ui sidebar modes:
/// - [`SidebarCollapsible::Icon`] collapses the sidebar to icon width.
/// - [`SidebarCollapsible::Offcanvas`] slides the sidebar out and releases its
///   layout width.
/// - [`SidebarCollapsible::None`] keeps the sidebar expanded and ignores the
///   collapsed state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidebarCollapsible {
    /// Collapse the sidebar to icon width.
    #[default]
    Icon,
    /// Collapse the sidebar completely out of the layout.
    Offcanvas,
    /// Disable sidebar collapse.
    None,
}

impl From<bool> for SidebarCollapsible {
    fn from(collapsible: bool) -> Self {
        if collapsible {
            Self::Icon
        } else {
            Self::None
        }
    }
}

/// The side of the window a [`Sidebar`] docks to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SidebarSide {
    /// The sidebar sits on the left edge.
    #[default]
    Left,
    /// The sidebar sits on the right edge.
    Right,
}

impl SidebarSide {
    /// Whether this is [`SidebarSide::Left`].
    #[must_use]
    pub fn is_left(self) -> bool {
        self == Self::Left
    }

    /// Whether this is [`SidebarSide::Right`].
    #[must_use]
    pub fn is_right(self) -> bool {
        self == Self::Right
    }
}

/// The wrapper's layout for one frame, derived from the sidebar's state.
///
/// A pure function of the sidebar's builders, so the three collapsing modes
/// and their edge cases are testable without a renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SidebarWrapperLayout {
    /// No wrapper: the sidebar renders as it is.
    None,
    /// A fixed-width wrapper with no animation, used by `Offcanvas` when the
    /// sidebar has no pixel width to animate.
    Static { width: f32 },
    /// A wrapper that animates between the current width and `target_width`.
    Animated { target_width: f32 },
}

/// Everything the sidebar's rendering needs to know about collapsing.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SidebarLayout {
    /// The sidebar is collapsed to an icon rail.
    icon_collapsed: bool,
    /// The sidebar is slid fully out of the layout.
    offcanvas_collapsed: bool,
    /// Whether the visible remainder hugs the content edge while an
    /// `Offcanvas` sidebar is mid-slide: a left sidebar's content sticks to
    /// the right of the clip box and a right sidebar's to the left, so the
    /// border that faces the page stays put.
    ///
    /// Computed for parity with the reference layout, whose wrapper slides the
    /// child under a clip. This port animates the layout width instead —
    /// iced's layers clip absolutely, so a slide cannot be composed with the
    /// scrollable body — and a wrapper that bounds its content has nothing
    /// left over to align.
    align_child_to_end: bool,
    wrapper: SidebarWrapperLayout,
}

impl SidebarLayout {
    fn new(
        collapsible: SidebarCollapsible,
        collapsed: bool,
        expanded_width: Option<f32>,
        side: SidebarSide,
    ) -> Self {
        let collapsed = collapsed && collapsible != SidebarCollapsible::None;
        let wrapper = match collapsible {
            SidebarCollapsible::None => SidebarWrapperLayout::None,
            SidebarCollapsible::Icon => match expanded_width {
                Some(expanded_width) => SidebarWrapperLayout::Animated {
                    target_width: if collapsed {
                        COLLAPSED_WIDTH
                    } else {
                        expanded_width
                    },
                },
                None => SidebarWrapperLayout::None,
            },
            SidebarCollapsible::Offcanvas => match (expanded_width, collapsed) {
                (Some(_), true) => SidebarWrapperLayout::Animated { target_width: 0.0 },
                (Some(expanded_width), false) => SidebarWrapperLayout::Animated {
                    target_width: expanded_width,
                },
                (None, true) => SidebarWrapperLayout::Static { width: 0.0 },
                (None, false) => SidebarWrapperLayout::None,
            },
        };
        let align_child_to_end = match collapsible {
            SidebarCollapsible::Offcanvas => side.is_left(),
            _ => side.is_right(),
        };

        Self {
            icon_collapsed: collapsed && collapsible == SidebarCollapsible::Icon,
            offcanvas_collapsed: collapsed && collapsible == SidebarCollapsible::Offcanvas,
            align_child_to_end,
            wrapper,
        }
    }
}

/// An element a [`Sidebar`] can host in its body, and that knows how to draw
/// itself when the sidebar has collapsed to an icon rail.
///
/// [`SidebarGroup`] and [`SidebarMenu`] implement it; the sidebar pushes the
/// collapsed flag down through [`SidebarItem::into_element`] so a group can
/// hide its heading and a menu item can centre its icon.
pub trait SidebarItem<'a, Message>: 'a {
    /// Converts the item into an element.
    ///
    /// `collapsed` is the sidebar's own icon-collapsed state, pushed down
    /// through the tree; a group re-pushes it to the items it hosts.
    fn into_element(self: Box<Self>, collapsed: bool) -> Element<'a, Message, Theme>;
}

/// Builds a sidebar: a collapsible panel with a header, a scrollable body and
/// a footer.
///
/// ```
/// # use iced_kit::icons::IconName;
/// # use iced_kit::widgets::{sidebar, SidebarCollapsible, SidebarGroup, SidebarMenu, SidebarMenuItem};
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Picked, Toggled }
/// # fn view() -> iced::Element<'static, Message, Theme> {
/// sidebar::<Message>()
///     .collapsible(SidebarCollapsible::Icon)
///     .collapsed(false)
///     .child(
///         SidebarGroup::new("Application").child(
///             SidebarMenu::new().children([
///                 SidebarMenuItem::new("Dashboard")
///                     .icon(IconName::LayoutDashboard)
///                     .on_select(Message::Picked),
///                 SidebarMenuItem::new("Inbox").icon(IconName::Inbox),
///             ]),
///         ),
///     )
///     .into()
/// # }
/// ```
#[must_use = "a Sidebar does nothing unless it is turned into an Element"]
pub struct Sidebar<'a, Message> {
    items: Vec<Box<dyn SidebarItem<'a, Message> + 'a>>,
    header: Option<Element<'a, Message, Theme>>,
    footer: Option<Element<'a, Message, Theme>>,
    side: SidebarSide,
    collapsible: SidebarCollapsible,
    collapsed: bool,
    width: f32,
    min_width: f32,
    on_resize: Option<Box<dyn Fn(f32) -> Message + 'a>>,
}

/// The narrowest a [`Sidebar`] can be dragged by default, in logical pixels.
///
/// Below roughly this width the labels and icons stop fitting, so a drag that
/// would go further is stopped; [`Sidebar::min_width`] overrides it.
const DEFAULT_MIN_WIDTH: f32 = 180.0;

/// Builds a [`Sidebar`], matching the crate's module-plus-constructor
/// convention.
pub fn sidebar<'a, Message: Clone + 'a>() -> Sidebar<'a, Message> {
    Sidebar::new()
}

impl<'a, Message: Clone + 'a> Sidebar<'a, Message> {
    /// Creates an expanded left sidebar, 255 logical pixels wide.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            header: None,
            footer: None,
            side: SidebarSide::Left,
            collapsible: SidebarCollapsible::Icon,
            collapsed: false,
            width: DEFAULT_WIDTH,
            min_width: DEFAULT_MIN_WIDTH,
            on_resize: None,
        }
    }

    /// Sets the side of the window the sidebar docks to.
    pub fn side(mut self, side: SidebarSide) -> Self {
        self.side = side;
        self
    }

    /// Sets how the sidebar collapses.
    ///
    /// Passing `true` keeps the previous behavior and maps to
    /// [`SidebarCollapsible::Icon`]. Passing `false` maps to
    /// [`SidebarCollapsible::None`].
    pub fn collapsible(mut self, collapsible: impl Into<SidebarCollapsible>) -> Self {
        self.collapsible = collapsible.into();
        self
    }

    /// Sets whether the sidebar is collapsed.
    ///
    /// What collapsing means is decided by [`Sidebar::collapsible`].
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Sets the sidebar's expanded width, in logical pixels.
    ///
    /// The width stays the caller's: store it in the application state and
    /// feed what [`Sidebar::on_resize`] reports back into this builder.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Lets the user drag the sidebar's inner edge to change its width.
    ///
    /// The handle sits on the edge that faces the page and uses the same
    /// interaction as the [`Resizable`](crate::widgets::Resizable) split
    /// panes: a 6-pixel grab strip, a muted line on hover, the accent line
    /// while dragging. Every drag movement reports the new width, which the
    /// application stores and passes back through [`Sidebar::width`].
    ///
    /// The handle is off while the sidebar is icon-collapsed or slid out, and
    /// a drag can never squeeze the content beside the sidebar below the
    /// layout's remaining space. Resizing needs the animated wrapper, so
    /// [`SidebarCollapsible::None`] — which has no wrapper — ignores this.
    pub fn on_resize(mut self, on_resize: impl Fn(f32) -> Message + 'a) -> Self {
        self.on_resize = Some(Box::new(on_resize));
        self
    }

    /// Sets the smallest width a drag can produce, in logical pixels.
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width.max(1.0);
        self
    }

    /// Sets the header slot, usually a [`SidebarHeader`].
    pub fn header(mut self, header: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.header = Some(header.into());
        self
    }

    /// Sets the footer slot, usually a [`SidebarFooter`].
    pub fn footer(mut self, footer: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Adds an item to the sidebar's body.
    pub fn child(mut self, child: impl SidebarItem<'a, Message> + 'a) -> Self {
        self.items.push(Box::new(child));
        self
    }

    /// Adds multiple items to the sidebar's body.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl SidebarItem<'a, Message> + 'a>,
    ) -> Self {
        self.items
            .extend(children.into_iter().map(|child| Box::new(child) as _));
        self
    }

    /// Converts the sidebar into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        // The width builder stands in for the reference implementation's
        // `Styled` chain, so the expanded width is always known in pixels. The
        // no-width paths of `SidebarLayout` — a sidebar whose width is a
        // relative length cannot animate it — remain in the ported function
        // for its own tests, but this composition cannot reach them.
        let Self {
            items,
            header,
            footer,
            side,
            collapsible,
            collapsed,
            width,
            min_width,
            on_resize,
        } = self;

        let layout = SidebarLayout::new(collapsible, collapsed, Some(width), side);

        // The drag handle rides on the animated wrapper and is off whenever
        // there is no expanded edge to grab: a rail, or a sidebar slid out.
        let on_resize = if layout.icon_collapsed || layout.offcanvas_collapsed {
            None
        } else {
            on_resize
        };
        let anchored_left = side.is_left();

        let sidebar = Self::surface(items, header, footer, side, layout.icon_collapsed);

        match layout.wrapper {
            SidebarWrapperLayout::None => clip(sidebar, width).into(),
            SidebarWrapperLayout::Static { width } => {
                // Without a pixel width to animate, an offcanvas collapse is
                // adopted at once: the content is not built at all, which is
                // what releases the layout.
                let content: Element<'a, Message, Theme> = if layout.offcanvas_collapsed {
                    Element::new(Space)
                } else {
                    sidebar
                };

                clip(content, width).into()
            }
            SidebarWrapperLayout::Animated { target_width } => {
                SidebarShell::new(sidebar, target_width, anchored_left, min_width, on_resize).into()
            }
        }
    }

    /// Builds the themed surface: header, scrollable body and footer, with the
    /// border that faces the page.
    ///
    /// The surface is always built at its target width — a wrapper animates
    /// the clip around it, which is what keeps the transition from re-flowing
    /// the sidebar's own content on every frame.
    fn surface(
        items: Vec<Box<dyn SidebarItem<'a, Message> + 'a>>,
        header: Option<Element<'a, Message, Theme>>,
        footer: Option<Element<'a, Message, Theme>>,
        side: SidebarSide,
        icon_collapsed: bool,
    ) -> Element<'a, Message, Theme> {
        let gaps = if icon_collapsed { 8.0 } else { 0.0 };
        let inset_x = if icon_collapsed { 8.0 } else { 12.0 };
        let inset_y = if icon_collapsed { 8.0 } else { 12.0 };

        let mut body = column![]
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(gaps);

        if let Some(header) = header {
            body = body.push(container(header).width(Length::Fill).padding(Padding {
                top: inset_y,
                right: inset_x,
                bottom: 0.0,
                left: inset_x,
            }));
        }

        let items: Vec<Element<'a, Message, Theme>> = items
            .into_iter()
            .map(|item| item.into_element(icon_collapsed))
            .collect();

        let list = column(items)
            .spacing(12.0)
            .width(Length::Fill)
            .padding(Padding {
                top: inset_y,
                right: inset_x,
                bottom: inset_y,
                left: inset_x,
            });

        body = body.push(
            scrollable(container(list).width(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        );

        if let Some(footer) = footer {
            body = body.push(container(footer).width(Length::Fill).padding(Padding {
                top: 0.0,
                right: inset_x,
                bottom: inset_y,
                left: inset_x,
            }));
        }

        // The border sits on the edge that faces the page, so a left sidebar
        // draws it on its right and a right sidebar on its left. iced borders
        // surround a control on every side at once, so the one-sided rule is a
        // hairline column inside the surface instead.
        let hairline = container(Space)
            .width(Length::Fixed(1.0))
            .height(Length::Fill)
            .class(Box::new(sidebar_hairline) as container::StyleFn<'a, Theme>);

        let surface = match side {
            SidebarSide::Left => row![body, hairline],
            SidebarSide::Right => row![hairline, body],
        }
        .width(Length::Fill)
        .height(Length::Fill);

        container(surface)
            .width(Length::Fill)
            .height(Length::Fill)
            .class(Box::new(sidebar_surface) as container::StyleFn<'a, Theme>)
            .into()
    }
}

impl Default for Sidebar<'_, ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<Sidebar<'a, Message>> for Element<'a, Message, Theme> {
    fn from(sidebar: Sidebar<'a, Message>) -> Self {
        sidebar.into_element()
    }
}

/// The surface colors of the sidebar: its background and the text on it.
fn sidebar_surface(theme: &Theme) -> container::Style {
    let colors = theme.colors();

    container::Style {
        background: Some(iced::Background::Color(colors.sidebar)),
        text_color: Some(colors.sidebar_foreground),
        ..container::Style::default()
    }
}

/// The one-sided border between the sidebar and the page.
fn sidebar_hairline(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme.colors().sidebar_border)),
        ..container::Style::default()
    }
}

/// Clips `content` to a fixed width without animating, for the sidebar modes
/// that adopt their width on the spot.
fn clip<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
    width: f32,
) -> Clip<'a, Message> {
    Clip {
        content: content.into(),
        width: width.max(0.0),
    }
}

/// A fixed-width clipping wrapper.
struct Clip<'a, Message> {
    content: Element<'a, Message, Theme>,
    width: f32,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Clip<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Clip<()>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(Length::Shrink, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // The wrapper's width is the content's: a wider width would let
        // `Fill` children run past the clip, which iced cannot cut off (its
        // layers clip absolutely, so a nested layer escapes this one).
        let child_limits = layout::Limits::new(
            IcedSize::new(0.0, limits.min().height),
            IcedSize::new(self.width.min(limits.max().width), limits.max().height),
        );

        let content =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &child_limits);

        layout::Node::with_children(
            IcedSize::new(self.width, content.size().height),
            vec![content],
        )
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // Only the visible part answers for the content, so a control inside
        // the clipped part never reacts to a click aimed at the page.
        let bounds = layout.bounds();
        let reachable = cursor.position_over(bounds).is_some();

        if let Some(inner) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                inner,
                if reachable {
                    cursor
                } else {
                    mouse::Cursor::Unavailable
                },
                renderer,
                clipboard,
                shell,
                &bounds
                    .intersection(viewport)
                    .unwrap_or_else(|| Rectangle::with_size(IcedSize::ZERO)),
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.position_over(layout.bounds()).is_none() {
            return mouse::Interaction::None;
        }

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if bounds.width <= 0.0 || !bounds.intersects(viewport) {
            return;
        }

        // The layer is the clip: the content is drawn at its own width and cut
        // off at the wrapper's.
        renderer.with_layer(bounds, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout.children().next().unwrap(),
                cursor,
                viewport,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next()?,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<Clip<'a, Message>> for Element<'a, Message, Theme> {
    fn from(clip: Clip<'a, Message>) -> Self {
        Element::new(clip)
    }
}

/// The zero-width stand-in for a sidebar whose content is not built.
struct Space;

impl<Message> Widget<Message, Theme, iced::Renderer> for Space {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Space>()
    }

    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(Length::Fixed(0.0), Length::Shrink)
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(IcedSize::new(0.0, limits.min().height))
    }

    fn draw(
        &self,
        _tree: &tree::Tree,
        _renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        // Nothing occupies this space, so nothing is drawn in it.
    }
}

impl<'a, Message: 'a> From<Space> for Element<'a, Message, Theme> {
    fn from(space: Space) -> Self {
        Element::new(space)
    }
}

/// The spring-driven wrapper around an animating sidebar.
///
/// The wrapper's width is the animated value, and the sidebar is laid out
/// *inside* that width — the content genuinely narrows for the duration of the
/// transition and is at its target width once the spring settles. The
/// reference implementation instead animates a clip around content that keeps
/// its target width, so the body slides out whole; that strategy does not
/// survive the port. iced's layers clip absolutely: a nested layer (and the
/// sidebar's scrollable body is one) records its own clip at its own width and
/// escapes the wrapper's, so a slide would paint straight over the page.
/// Animating the layout width is how everything else in iced narrows, and it
/// keeps the resting geometry exact.
///
/// Once an `Offcanvas` sidebar has settled at zero the wrapper stops laying
/// out, drawing and updating its content, which is the benefit the reference
/// gets from unmounting the child after its hide timer fires. The one cost
/// iced cannot avoid is building the element: the view is stateless and
/// cannot know the settled width at build time.
struct SidebarShell<'a, Message> {
    content: Element<'a, Message, Theme>,
    target_width: f32,
    /// Whether the sidebar is anchored at the row's left edge, which decides
    /// which way a drag widens it.
    anchored_left: bool,
    min_width: f32,
    on_resize: Option<Box<dyn Fn(f32) -> Message + 'a>>,
}

/// The shell's width between frames.
#[derive(Debug, Clone, Copy, Default)]
struct ShellState {
    /// The animated width, in logical pixels.
    width: crate::motion::SpringState,
    /// The frame the spring was last advanced to.
    last: Option<Instant>,
    /// The width drag in progress, if any.
    drag: Option<DragState>,
    /// The row width a drag may grow into, captured at layout time.
    available: f32,
}

impl ShellState {
    fn new(target_width: f32) -> Self {
        Self {
            // A spring created at its target is already settled: a sidebar
            // that starts collapsed is hidden on its first frame rather than
            // sliding out of nothing, and one that starts expanded does not
            // slide in.
            width: crate::motion::SpringState::new(target_width),
            ..Self::default()
        }
    }
}

/// Where a drag started, and the width it started from.
#[derive(Debug, Clone, Copy)]
struct DragState {
    press_x: f32,
    press_width: f32,
    /// The last width reported to the application, so a clamp that pins the
    /// cursor does not publish the same width on every mouse move.
    published: f32,
}

impl<'a, Message: Clone + 'a> SidebarShell<'a, Message> {
    fn new(
        content: impl Into<Element<'a, Message, Theme>>,
        target_width: f32,
        anchored_left: bool,
        min_width: f32,
        on_resize: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    ) -> Self {
        Self {
            content: content.into(),
            target_width: target_width.max(0.0),
            anchored_left,
            min_width,
            on_resize,
        }
    }

    /// Whether the drag handle is live this frame.
    fn resizable(&self) -> bool {
        self.on_resize.is_some() && self.target_width > 0.0
    }

    /// The strip that catches a press, on the edge that faces the page.
    fn grab_rect(bounds: Rectangle, anchored_left: bool) -> Rectangle {
        crate::widgets::resize_edge::grab_rect(bounds, anchored_left)
    }

    /// Runs the width drag, reporting each new width to the application.
    ///
    /// Returns whether the event was consumed by the drag; a consumed press,
    /// move or release does not reach the sidebar's own controls, so a control
    /// under the strip never fires and the drag keeps running outside the
    /// sidebar's bounds.
    fn handle_drag(
        &self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let Some(on_resize) = self.on_resize.as_ref() else {
            return false;
        };

        let bounds = layout.bounds();
        let strip = Self::grab_rect(bounds, self.anchored_left);
        let state = tree.state.downcast_mut::<ShellState>();

        match event {
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                let Some(position) = cursor.position_over(strip) else {
                    return false;
                };

                state.drag = Some(DragState {
                    press_x: position.x,
                    press_width: bounds.width,
                    published: bounds.width,
                });

                true
            }
            Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                let Some(drag) = state.drag.as_mut() else {
                    return false;
                };

                let position = cursor.position().unwrap_or_default();
                // The page beside the sidebar keeps a floor, which is the same
                // deal the pane grid's `min_size` strikes for its panes.
                let max = (state.available - crate::widgets::resize_edge::MIN_CONTENT_AREA)
                    .max(self.min_width);
                let width = crate::widgets::resize_edge::dragged_width(
                    drag.press_width,
                    drag.press_x,
                    position.x,
                    self.anchored_left,
                    self.min_width,
                    max,
                );

                if width != drag.published {
                    drag.published = width;
                    // The width is adopted on the spot rather than eased
                    // towards: a drag must track the cursor, not chase it.
                    state.width.set(width);
                    shell.invalidate_layout();
                    shell.request_redraw();
                    shell.publish(on_resize(width));
                }

                true
            }
            Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                state.drag.take().is_some()
            }
            _ => false,
        }
    }

    /// The spring a width follows: critically damped, with a sub-pixel
    /// tolerance, so a retargeted collapse decelerates and turns instead of
    /// restarting.
    fn spring() -> crate::motion::Spring {
        if crate::motion::reduce_motion() {
            crate::motion::Spring::new(std::time::Duration::ZERO)
        } else {
            crate::motion::Spring::new(crate::motion::DURATION_NORMAL).with_epsilon(0.05)
        }
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, iced::Renderer> for SidebarShell<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ShellState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ShellState::new(self.target_width))
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(Length::Shrink, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<ShellState>();
        let spring = Self::spring();
        let width = state.width.value().max(0.0);

        // The row a drag may grow into, which only the layout pass knows.
        state.available = limits.max().width;

        // Settled out of the layout: the content costs nothing, which is what
        // releases the width an `Offcanvas` sidebar occupied.
        if self.target_width <= 0.0 && width <= 0.0 && state.width.is_settled(0.0, spring) {
            return layout::Node::new(IcedSize::new(0.0, limits.min().height));
        }

        // The animated width bounds the content: the sidebar narrows with the
        // wrapper instead of sliding under a clip (see the type's notes).
        let shown = width.min(self.target_width).min(limits.max().width);
        let child_limits = layout::Limits::new(
            IcedSize::new(0.0, limits.min().height),
            IcedSize::new(shown, limits.max().height),
        );

        let content =
            self.content
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &child_limits);

        layout::Node::with_children(IcedSize::new(shown, content.size().height), vec![content])
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let Some(inner) = layout.children().next() else {
            return;
        };

        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], inner, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<ShellState>();
            let spring = Self::spring();
            let elapsed = state
                .last
                .map_or(std::time::Duration::ZERO, |last| now.duration_since(last));
            state.last = Some(*now);

            state.width.step(self.target_width, spring, elapsed);

            // A width change re-lays the page out, so the frame has to be
            // re-laid out rather than merely redrawn.
            if !state.width.is_settled(self.target_width, spring) {
                shell.invalidate_layout();
                shell.request_redraw();
            }
        }

        // The grab strip wins over the sidebar's own controls: a drag that
        // starts on the edge consumes its press, its moves and its release.
        if self.resizable() && self.handle_drag(tree, event, layout, cursor, shell) {
            return;
        }

        // Only the visible remainder answers for the content: a control inside
        // the clipped part of a mid-slide sidebar must not react to a click
        // aimed at the page.
        let bounds = layout.bounds();
        let reachable = cursor.position_over(bounds).is_some();

        if let Some(inner) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                inner,
                if reachable {
                    cursor
                } else {
                    mouse::Cursor::Unavailable
                },
                renderer,
                clipboard,
                shell,
                &bounds
                    .intersection(viewport)
                    .unwrap_or_else(|| Rectangle::with_size(IcedSize::ZERO)),
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let strip = Self::grab_rect(bounds, self.anchored_left);
        let dragging = tree.state.downcast_ref::<ShellState>().drag.is_some();

        if dragging || cursor.position_over(strip).is_some() {
            return mouse::Interaction::ResizingHorizontally;
        }

        if bounds.width <= 0.0 || cursor.position_over(bounds).is_none() {
            return mouse::Interaction::None;
        }

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap(),
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if bounds.width <= 0.0 || !bounds.intersects(viewport) {
            return;
        }

        // The layer is the clip: the content is drawn at its target width and
        // cut off at the current one.
        renderer.with_layer(bounds, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout.children().next().unwrap(),
                cursor,
                viewport,
            );
        });

        // The handle announces itself with the same two-step coloring the
        // pane grid's splitter uses: a muted line on hover, the accent line
        // while the drag is live. Idle it stays invisible, and the surface's
        // own hairline is the divider.
        let strip = Self::grab_rect(bounds, self.anchored_left);
        let dragging = tree.state.downcast_ref::<ShellState>().drag.is_some();
        let hovered = cursor.position_over(strip).is_some();

        if !dragging && !hovered {
            return;
        }

        let color = if dragging {
            crate::widgets::display::Tone::Primary.accent(theme)
        } else {
            theme.colors().ring
        };

        let line = Rectangle {
            x: if self.anchored_left {
                strip.x + strip.width - crate::widgets::resize_edge::LINE_WIDTH
            } else {
                strip.x
            },
            width: crate::widgets::resize_edge::LINE_WIDTH,
            ..strip
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: line,
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                shadow: iced::Shadow::default(),
                snap: true,
            },
            color,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, iced::Renderer>> {
        // A hidden sidebar shows nothing, so neither does anything inside it.
        if layout.bounds().width <= 0.0 {
            return None;
        }

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next()?,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: Clone + 'a> From<SidebarShell<'a, Message>> for Element<'a, Message, Theme> {
    fn from(shell: SidebarShell<'a, Message>) -> Self {
        Element::new(shell)
    }
}

/// A titled section of a [`Sidebar`]'s body.
///
/// The heading is hidden when the sidebar is icon-collapsed, which is what
/// leaves the rail to the icons alone.
#[must_use = "a SidebarGroup does nothing unless it is given to a Sidebar"]
pub struct SidebarGroup<'a, Message> {
    label: String,
    children: Vec<Box<dyn SidebarItem<'a, Message> + 'a>>,
}

impl<'a, Message: Clone + 'a> SidebarGroup<'a, Message> {
    /// Creates a group with the given heading.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            children: Vec::new(),
        }
    }

    /// Adds an item to the group.
    pub fn child(mut self, child: impl SidebarItem<'a, Message> + 'a) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Adds multiple items to the group.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl SidebarItem<'a, Message> + 'a>,
    ) -> Self {
        self.children
            .extend(children.into_iter().map(|child| Box::new(child) as _));
        self
    }
}

impl<'a, Message: Clone + 'a> SidebarItem<'a, Message> for SidebarGroup<'a, Message> {
    fn into_element(self: Box<Self>, collapsed: bool) -> Element<'a, Message, Theme> {
        let mut group = column![].width(Length::Fill).spacing(4.0);

        if !collapsed {
            group = group.push(
                container(text(self.label).size(Size::Xs.text().size))
                    .width(Length::Fill)
                    .height(Length::Fixed(32.0))
                    .padding(Padding::new(8.0))
                    .align_y(alignment::Vertical::Center)
                    .class(Box::new(group_label) as container::StyleFn<'a, Theme>),
            );
        }

        for child in self.children {
            group = group.push(child.into_element(collapsed));
        }

        group.into()
    }
}

impl<'a, Message: Clone + 'a> From<SidebarGroup<'a, Message>> for Element<'a, Message, Theme> {
    fn from(group: SidebarGroup<'a, Message>) -> Self {
        Box::new(group).into_element(false)
    }
}

/// The heading of a [`SidebarGroup`], dimmed against the items below it.
fn group_label(theme: &Theme) -> container::Style {
    let colors = theme.colors();

    container::Style {
        text_color: Some(Color {
            a: colors.sidebar_foreground.a * 0.7,
            ..colors.sidebar_foreground
        }),
        ..container::Style::default()
    }
}

/// A list of menu items inside a [`SidebarGroup`].
#[must_use = "a SidebarMenu does nothing unless it is given to a Sidebar or a SidebarGroup"]
pub struct SidebarMenu<'a, Message> {
    items: Vec<SidebarMenuItem<'a, Message>>,
}

impl<'a, Message: Clone + 'a> SidebarMenu<'a, Message> {
    /// Creates an empty menu.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Adds an item to the menu.
    pub fn child(mut self, child: impl Into<SidebarMenuItem<'a, Message>>) -> Self {
        self.items.push(child.into());
        self
    }

    /// Adds multiple items to the menu.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<SidebarMenuItem<'a, Message>>>,
    ) -> Self {
        self.items = children.into_iter().map(Into::into).collect();
        self
    }
}

impl Default for SidebarMenu<'_, ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> SidebarItem<'a, Message> for SidebarMenu<'a, Message> {
    fn into_element(self: Box<Self>, collapsed: bool) -> Element<'a, Message, Theme> {
        let mut menu = column![].width(Length::Fill).spacing(8.0);

        for item in self.items {
            menu = menu.push(Box::new(item).into_element(collapsed));
        }

        menu.into()
    }
}

impl<'a, Message: Clone + 'a> From<SidebarMenu<'a, Message>> for Element<'a, Message, Theme> {
    fn from(menu: SidebarMenu<'a, Message>) -> Self {
        Box::new(menu).into_element(false)
    }
}

/// One entry in a [`SidebarMenu`].
///
/// An item with [`SidebarMenuItem::children`] is a submenu: a caret button
/// appears after its label, and the children are drawn under the item while it
/// is open and the sidebar is not collapsed.
#[must_use = "a SidebarMenuItem does nothing unless it is given to a SidebarMenu"]
pub struct SidebarMenuItem<'a, Message> {
    label: String,
    icon: Option<Icon>,
    active: bool,
    disabled: bool,
    children: Vec<SidebarMenuItem<'a, Message>>,
    open: bool,
    on_toggle: Option<Message>,
    on_select: Option<Message>,
    on_context: Option<Message>,
    suffix: Option<Element<'a, Message, Theme>>,
    tooltip_text: Option<String>,
}

impl<'a, Message: Clone + 'a> SidebarMenuItem<'a, Message> {
    /// Creates an inactive, enabled item with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            active: false,
            disabled: false,
            children: Vec::new(),
            open: false,
            on_toggle: None,
            on_select: None,
            on_context: None,
            suffix: None,
            tooltip_text: None,
        }
    }

    /// Sets the icon shown before the label.
    ///
    /// The icon is what the item shows when the sidebar is icon-collapsed, so
    /// an item without one has nothing to show there.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Marks the item as the current selection.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Disables the item. A disabled item renders but emits nothing.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Adds children, which turns the item into a submenu.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<SidebarMenuItem<'a, Message>>>,
    ) -> Self {
        self.children = children.into_iter().map(Into::into).collect();
        self
    }

    /// Sets whether the submenu is open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Sets the message the caret button emits, toggling the submenu.
    ///
    /// Without it a submenu has no caret and stays at
    /// [`SidebarMenuItem::open`].
    pub fn on_toggle(mut self, message: Message) -> Self {
        self.on_toggle = Some(message);
        self
    }

    /// Sets the message emitted when the item is pressed.
    ///
    /// A disabled item emits nothing.
    pub fn on_select(mut self, message: Message) -> Self {
        self.on_select = Some(message);
        self
    }

    /// Sets the message emitted when the item is right-clicked.
    ///
    /// The menu itself — its items and where it appears — is the application's,
    /// drawn through the [`Layer`](crate::widgets::overlay::Layer).
    pub fn on_context(mut self, message: Message) -> Self {
        self.on_context = Some(message);
        self
    }

    /// Sets an element drawn after the label, such as a badge or a switch.
    pub fn suffix(mut self, suffix: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Overrides the label the item shows in its collapsed-state tooltip.
    ///
    /// Without it the tooltip shows the item's own label.
    pub fn tooltip_text(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip_text = Some(tooltip.into());
        self
    }

    /// Whether the item is a submenu.
    fn is_submenu(&self) -> bool {
        !self.children.is_empty()
    }

    /// The tooltip the item shows when the sidebar is icon-collapsed.
    ///
    /// Only an item with an icon has something to hover, and only when it is
    /// actually collapsed.
    fn collapsed_tooltip(&self, collapsed: bool) -> Option<String> {
        if collapsed && self.icon.is_some() {
            Some(
                self.tooltip_text
                    .clone()
                    .unwrap_or_else(|| self.label.clone()),
            )
        } else {
            None
        }
    }

    /// The message pressing the item emits, if it may be pressed at all.
    fn select_message(&self) -> Option<&Message> {
        if self.disabled {
            None
        } else {
            self.on_select.as_ref()
        }
    }
}

impl<'a, Message: Clone + 'a> SidebarItem<'a, Message> for SidebarMenuItem<'a, Message> {
    fn into_element(self: Box<Self>, collapsed: bool) -> Element<'a, Message, Theme> {
        let is_submenu = self.is_submenu();
        let is_open = self.open && !collapsed;
        let active = self.active;
        let disabled = self.disabled;
        let hoverable = !active && !disabled;
        let on_select = self.select_message().cloned();
        let on_context = self.on_context.clone();
        let on_toggle = self.on_toggle.clone();
        // Read before any field is moved out of the item.
        let tooltip = self.collapsed_tooltip(collapsed);

        // The rail shows the icon alone, centred; a label would not fit, and
        // the tooltip carries it instead.
        let icon = self
            .icon
            .as_ref()
            .map(|icon| icon.clone().into_element(Size::Md));

        let content: Element<'a, Message, Theme> = if collapsed {
            container(icon.unwrap_or_else(|| Element::new(Space)))
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Center)
                .into()
        } else {
            let label = text(self.label.clone())
                .size(Size::Sm.text().size)
                .line_height(Size::Sm.text().line_height())
                .wrapping(iced::widget::text::Wrapping::None)
                .width(Length::Fill)
                .font(if active {
                    // The reference marks the active item in a medium weight
                    // as well as by its background.
                    iced::Font {
                        weight: iced::font::Weight::Medium,
                        ..iced::Font::DEFAULT
                    }
                } else {
                    iced::Font::DEFAULT
                });

            let mut line = row![].spacing(8).align_y(alignment::Alignment::Center);

            if let Some(icon) = icon {
                line = line.push(icon);
            }

            line = line.push(label);

            if let Some(suffix) = self.suffix {
                line = line.push(suffix);
            }

            if is_submenu {
                if let Some(on_toggle) = on_toggle {
                    let caret = Button::icon_only()
                        .ghost()
                        .size(Size::Xs)
                        .icon(Icon::new(if is_open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        }))
                        .on_press(on_toggle);

                    line = line.push(caret);
                }
            }

            row![line]
                .width(Length::Fill)
                .height(Length::Fixed(28.0))
                .into()
        };

        let mut item: Element<'a, Message, Theme> = iced_button(content)
            .width(Length::Fill)
            .padding(Padding::new(8.0))
            .class(Box::new(move |theme: &Theme, status| {
                item_style(theme, status, active, disabled, hoverable)
            }) as iced_button::StyleFn<'a, Theme>)
            .on_press_maybe(on_select)
            .into();

        // The sensor covers the same row the button does — the button inside
        // fills the width, and the sensor shrink-wraps to it — so a
        // right-click anywhere on the item reports the intent.
        if let Some(on_context) = on_context {
            item = mouse_area(item).on_right_press(on_context).into();
        }

        if let Some(tooltip) = tooltip {
            item =
                crate::widgets::tooltip_at(item, tooltip, crate::widgets::TooltipPosition::Right);
        }

        let mut item = column![item].width(Length::Fill);

        if is_submenu && is_open {
            // The children indent under their parent, separated from it by a
            // hairline rule — one more thing iced borders cannot do on a
            // single side, so it is a column of its own.
            let mut children = column![].width(Length::Fill).spacing(4.0).padding(Padding {
                top: 2.0,
                right: 0.0,
                bottom: 2.0,
                left: 10.0,
            });

            for child in self.children {
                children = children.push(Box::new(child).into_element(collapsed));
            }

            let rule = container(Space)
                .width(Length::Fixed(1.0))
                .height(Length::Fill)
                .class(Box::new(sidebar_hairline) as container::StyleFn<'a, Theme>);

            item = item.push(
                container(
                    row![rule, children]
                        .width(Length::Fill)
                        .height(Length::Shrink),
                )
                .width(Length::Fill)
                .padding(Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 14.0,
                }),
            );
        }

        item.into()
    }
}

impl<'a, Message: Clone + 'a> From<SidebarMenuItem<'a, Message>> for Element<'a, Message, Theme> {
    fn from(item: SidebarMenuItem<'a, Message>) -> Self {
        Box::new(item).into_element(false)
    }
}

/// The appearance of a menu item.
fn item_style(
    theme: &Theme,
    status: iced_button::Status,
    active: bool,
    disabled: bool,
    hoverable: bool,
) -> iced_button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, iced_button::Status::Hovered);

    let (background, text_color) = if active {
        (
            Some(iced::Background::Color(colors.sidebar_accent)),
            colors.sidebar_accent_foreground,
        )
    } else if hovered && hoverable {
        // A hover washes the accent at four-fifths strength, so it reads as a
        // hint rather than as the selection the full accent marks.
        (
            Some(iced::Background::Color(Color {
                a: colors.sidebar_accent.a * 0.8,
                ..colors.sidebar_accent
            })),
            colors.sidebar_accent_foreground,
        )
    } else if disabled {
        (None, colors.muted_foreground)
    } else {
        (None, colors.sidebar_foreground)
    };

    iced_button::Style {
        background,
        text_color,
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: f32::from(theme.radius().md).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The slot at the top of a [`Sidebar`], usually an application switcher.
///
/// Hovering and selecting use the same accent as a menu item; the dropdown it
/// may open is the application's, reported through
/// [`SidebarHeader::on_dropdown`].
#[must_use = "a SidebarHeader does nothing unless it is given to a Sidebar"]
pub struct SidebarHeader<'a, Message> {
    child: Option<Element<'a, Message, Theme>>,
    selected: bool,
    on_dropdown: Option<Message>,
}

impl<'a, Message: Clone + 'a> SidebarHeader<'a, Message> {
    /// Creates an empty header.
    pub fn new() -> Self {
        Self {
            child: None,
            selected: false,
            on_dropdown: None,
        }
    }

    /// Sets the header's content.
    pub fn child(mut self, child: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Draws the header in its selected state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets the message emitted when the header is pressed, opening the
    /// application's dropdown.
    pub fn on_dropdown(mut self, message: Message) -> Self {
        self.on_dropdown = Some(message);
        self
    }
}

impl Default for SidebarHeader<'_, ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<SidebarHeader<'a, Message>> for Element<'a, Message, Theme> {
    fn from(header: SidebarHeader<'a, Message>) -> Self {
        let content: Element<'a, Message, Theme> = match header.child {
            Some(child) => row![child].width(Length::Fill).into(),
            None => Element::new(Space),
        };

        iced_button(content)
            .width(Length::Fill)
            .padding(Padding::new(8.0))
            .class(
                Box::new(move |theme: &Theme, status| slot_style(theme, status, header.selected))
                    as iced_button::StyleFn<'a, Theme>,
            )
            .on_press_maybe(header.on_dropdown)
            .into()
    }
}

/// The slot at the bottom of a [`Sidebar`], usually an account menu.
#[must_use = "a SidebarFooter does nothing unless it is given to a Sidebar"]
pub struct SidebarFooter<'a, Message> {
    child: Option<Element<'a, Message, Theme>>,
    selected: bool,
    on_dropdown: Option<Message>,
}

impl<'a, Message: Clone + 'a> SidebarFooter<'a, Message> {
    /// Creates an empty footer.
    pub fn new() -> Self {
        Self {
            child: None,
            selected: false,
            on_dropdown: None,
        }
    }

    /// Sets the footer's content.
    pub fn child(mut self, child: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.child = Some(child.into());
        self
    }

    /// Draws the footer in its selected state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets the message emitted when the footer is pressed, opening the
    /// application's dropdown.
    pub fn on_dropdown(mut self, message: Message) -> Self {
        self.on_dropdown = Some(message);
        self
    }
}

impl Default for SidebarFooter<'_, ()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<SidebarFooter<'a, Message>> for Element<'a, Message, Theme> {
    fn from(footer: SidebarFooter<'a, Message>) -> Self {
        let content: Element<'a, Message, Theme> = match footer.child {
            Some(child) => row![child].width(Length::Fill).into(),
            None => Element::new(Space),
        };

        iced_button(content)
            .width(Length::Fill)
            .padding(Padding::new(8.0))
            .class(
                Box::new(move |theme: &Theme, status| slot_style(theme, status, footer.selected))
                    as iced_button::StyleFn<'a, Theme>,
            )
            .on_press_maybe(footer.on_dropdown)
            .into()
    }
}

/// The appearance of a header or a footer slot.
fn slot_style(theme: &Theme, status: iced_button::Status, selected: bool) -> iced_button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, iced_button::Status::Hovered);
    let highlighted = selected || hovered;

    iced_button::Style {
        background: highlighted.then_some(iced::Background::Color(colors.sidebar_accent)),
        text_color: if highlighted {
            colors.sidebar_accent_foreground
        } else {
            colors.sidebar_foreground
        },
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: f32::from(theme.radius().md).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// The button that collapses and expands a [`Sidebar`].
///
/// Its icon is chosen from the side the sidebar docks to and the state it is
/// in, so the arrow always points the way the panel will go.
#[must_use = "a SidebarToggleButton does nothing unless it is turned into an Element"]
pub struct SidebarToggleButton<Message> {
    side: SidebarSide,
    collapsed: bool,
    on_press: Option<Message>,
}

impl<Message: Clone> SidebarToggleButton<Message> {
    /// Creates an expanded left-sidebar toggle.
    pub fn new() -> Self {
        Self {
            side: SidebarSide::Left,
            collapsed: false,
            on_press: None,
        }
    }

    /// Sets which side of the window the toggled sidebar docks to.
    pub fn side(mut self, side: SidebarSide) -> Self {
        self.side = side;
        self
    }

    /// Sets the collapsed state the button displays.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Sets the message emitted when the button is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The glyph the button draws, from its side and state.
    #[must_use]
    pub fn icon_name(&self) -> IconName {
        match (self.side, self.collapsed) {
            (SidebarSide::Left, true) => IconName::PanelLeftOpen,
            (SidebarSide::Left, false) => IconName::PanelLeftClose,
            (SidebarSide::Right, true) => IconName::PanelRightOpen,
            (SidebarSide::Right, false) => IconName::PanelRightClose,
        }
    }
}

impl<Message: Clone> Default for SidebarToggleButton<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: Clone + 'a> From<SidebarToggleButton<Message>> for Element<'a, Message, Theme> {
    fn from(toggle: SidebarToggleButton<Message>) -> Self {
        Button::icon_only()
            .ghost()
            .size(Size::Sm)
            .icon(Icon::new(toggle.icon_name()))
            .on_press_maybe(toggle.on_press)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        sidebar, ShellState, Sidebar, SidebarCollapsible, SidebarFooter, SidebarGroup,
        SidebarHeader, SidebarLayout, SidebarMenu, SidebarMenuItem, SidebarSide,
        SidebarToggleButton, SidebarWrapperLayout, COLLAPSED_WIDTH,
    };
    use crate::icons::IconName;
    use crate::theme::Theme;
    use crate::widgets::sidebar::SidebarShell;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Picked,
        Toggled,
    }

    fn layout(
        collapsible: SidebarCollapsible,
        collapsed: bool,
        expanded_width: Option<f32>,
        side: SidebarSide,
    ) -> SidebarLayout {
        SidebarLayout::new(collapsible, collapsed, expanded_width, side)
    }

    #[test]
    fn bool_collapsible_should_remain_backward_compatible() {
        assert_eq!(SidebarCollapsible::from(true), SidebarCollapsible::Icon);
        assert_eq!(SidebarCollapsible::from(false), SidebarCollapsible::None);
    }

    #[test]
    fn icon_collapsed_should_use_icon_width_and_icon_rendering() {
        let layout = layout(
            SidebarCollapsible::Icon,
            true,
            Some(240.0),
            SidebarSide::Left,
        );

        assert!(layout.icon_collapsed);
        assert!(!layout.offcanvas_collapsed);
        assert!(!layout.align_child_to_end);
        assert_eq!(
            layout.wrapper,
            SidebarWrapperLayout::Animated {
                target_width: COLLAPSED_WIDTH,
            }
        );
    }

    #[test]
    fn icon_expanded_should_use_expanded_width() {
        let layout = layout(
            SidebarCollapsible::Icon,
            false,
            Some(240.0),
            SidebarSide::Left,
        );

        assert!(!layout.icon_collapsed);
        assert!(!layout.offcanvas_collapsed);
        assert_eq!(
            layout.wrapper,
            SidebarWrapperLayout::Animated {
                target_width: 240.0,
            }
        );
    }

    #[test]
    fn icon_expanded_with_non_pixel_width_should_keep_original_layout() {
        let layout = layout(SidebarCollapsible::Icon, false, None, SidebarSide::Left);

        assert!(!layout.icon_collapsed);
        assert!(!layout.offcanvas_collapsed);
        assert_eq!(layout.wrapper, SidebarWrapperLayout::None);
    }

    #[test]
    fn none_should_ignore_collapsed_state() {
        let layout = layout(
            SidebarCollapsible::None,
            true,
            Some(240.0),
            SidebarSide::Right,
        );

        assert!(!layout.icon_collapsed);
        assert!(!layout.offcanvas_collapsed);
        assert!(layout.align_child_to_end);
        assert_eq!(layout.wrapper, SidebarWrapperLayout::None);
    }

    #[test]
    fn offcanvas_collapsed_with_pixel_width_should_animate_to_zero() {
        let layout = layout(
            SidebarCollapsible::Offcanvas,
            true,
            Some(240.0),
            SidebarSide::Left,
        );

        assert!(!layout.icon_collapsed);
        assert!(layout.offcanvas_collapsed);
        assert!(layout.align_child_to_end);
        assert_eq!(
            layout.wrapper,
            SidebarWrapperLayout::Animated { target_width: 0.0 }
        );
    }

    #[test]
    fn offcanvas_expanded_with_pixel_width_should_use_expanded_width() {
        let layout = layout(
            SidebarCollapsible::Offcanvas,
            false,
            Some(240.0),
            SidebarSide::Left,
        );

        assert!(!layout.icon_collapsed);
        assert!(!layout.offcanvas_collapsed);
        assert_eq!(
            layout.wrapper,
            SidebarWrapperLayout::Animated {
                target_width: 240.0,
            }
        );
    }

    #[test]
    fn offcanvas_collapsed_with_non_pixel_width_should_statically_release_layout() {
        let layout = layout(SidebarCollapsible::Offcanvas, true, None, SidebarSide::Left);

        assert!(!layout.icon_collapsed);
        assert!(layout.offcanvas_collapsed);
        assert_eq!(layout.wrapper, SidebarWrapperLayout::Static { width: 0.0 });
    }

    #[test]
    fn offcanvas_expanded_with_non_pixel_width_should_keep_original_layout() {
        let layout = layout(
            SidebarCollapsible::Offcanvas,
            false,
            None,
            SidebarSide::Left,
        );

        assert!(!layout.icon_collapsed);
        assert!(!layout.offcanvas_collapsed);
        assert_eq!(layout.wrapper, SidebarWrapperLayout::None);
    }

    #[test]
    fn offcanvas_should_anchor_child_toward_the_content_edge() {
        let left = layout(
            SidebarCollapsible::Offcanvas,
            true,
            Some(240.0),
            SidebarSide::Left,
        );
        let right = layout(
            SidebarCollapsible::Offcanvas,
            true,
            Some(240.0),
            SidebarSide::Right,
        );

        assert!(left.align_child_to_end);
        assert!(!right.align_child_to_end);
    }

    #[test]
    fn collapsed_icon_item_uses_label_as_tooltip() {
        let item = SidebarMenuItem::<Message>::new("Projects").icon(IconName::Folder);

        assert_eq!(item.collapsed_tooltip(true).as_deref(), Some("Projects"));
    }

    #[test]
    fn expanded_or_iconless_item_has_no_collapsed_tooltip() {
        let expanded = SidebarMenuItem::<Message>::new("Projects").icon(IconName::Folder);
        let iconless = SidebarMenuItem::<Message>::new("Projects");

        assert!(expanded.collapsed_tooltip(false).is_none());
        assert!(iconless.collapsed_tooltip(true).is_none());
    }

    #[test]
    fn a_tooltip_override_replaces_the_label() {
        let item = SidebarMenuItem::<Message>::new("Projects")
            .icon(IconName::Folder)
            .tooltip_text("All projects");

        assert_eq!(
            item.collapsed_tooltip(true).as_deref(),
            Some("All projects")
        );
    }

    #[test]
    fn a_disabled_item_emits_nothing() {
        let disabled = SidebarMenuItem::new("Projects")
            .on_select(Message::Picked)
            .disabled(true);

        assert_eq!(disabled.select_message(), None);

        let enabled = SidebarMenuItem::new("Projects").on_select(Message::Picked);

        assert_eq!(enabled.select_message(), Some(&Message::Picked));

        let unhandled = SidebarMenuItem::<Message>::new("Projects");

        assert_eq!(unhandled.select_message(), None);
    }

    #[test]
    fn the_toggle_button_picks_the_glyph_its_side_and_state_ask_for() {
        let collapsed_left = SidebarToggleButton::<Message>::new()
            .side(SidebarSide::Left)
            .collapsed(true);

        // The icon enum is re-exported from a crate that derives no
        // `PartialEq`, so two names are compared by the glyphs they draw —
        // the same comparison an `IconSource` makes.
        assert_eq!(
            crate::icons::glyph(collapsed_left.icon_name()),
            crate::icons::glyph(IconName::PanelLeftOpen)
        );

        let expanded_right = SidebarToggleButton::<Message>::new().side(SidebarSide::Right);

        assert_eq!(
            crate::icons::glyph(expanded_right.icon_name()),
            crate::icons::glyph(IconName::PanelRightClose)
        );
    }

    #[test]
    fn a_fresh_shell_starts_settled_at_its_target() {
        let expanded = ShellState::new(255.0);
        let collapsed = ShellState::new(0.0);

        assert_eq!(expanded.width.value(), 255.0);
        assert_eq!(collapsed.width.value(), 0.0);

        // A sidebar that starts offcanvas-collapsed is hidden on its first
        // frame, which is what keeps it from sliding out of nothing.
        assert!(collapsed
            .width
            .is_settled(0.0, SidebarShell::<Message>::spring()));
    }

    #[test]
    fn a_sidebar_renders_in_every_collapsing_mode() {
        let menu = || {
            SidebarMenu::new().children([
                SidebarMenuItem::new("Dashboard")
                    .icon(IconName::LayoutDashboard)
                    .active(true)
                    .on_select(Message::Picked),
                SidebarMenuItem::new("Projects")
                    .icon(IconName::Folder)
                    .open(true)
                    .on_toggle(Message::Toggled)
                    .on_context(Message::Picked)
                    .children([
                        SidebarMenuItem::new("Design"),
                        SidebarMenuItem::new("Engineering").disabled(true),
                    ]),
            ])
        };

        for collapsible in [
            SidebarCollapsible::Icon,
            SidebarCollapsible::Offcanvas,
            SidebarCollapsible::None,
        ] {
            for collapsed in [true, false] {
                let element: iced::Element<'_, Message, Theme> = sidebar()
                    .collapsible(collapsible)
                    .collapsed(collapsed)
                    .width(240.0)
                    .header(
                        SidebarHeader::<Message>::new()
                            .child(iced::widget::text("Acme Inc"))
                            .on_dropdown(Message::Toggled),
                    )
                    .child(SidebarGroup::new("Application").child(menu()))
                    .footer(
                        SidebarFooter::<Message>::new()
                            .child(iced::widget::text("Jason Lee"))
                            .on_dropdown(Message::Toggled),
                    )
                    .into();

                drop(element);
            }
        }
    }

    #[test]
    fn an_empty_sidebar_renders() {
        let element: iced::Element<'_, Message, Theme> = Sidebar::new().into();

        drop(element);
    }

    #[test]
    fn a_sidebar_docks_to_either_side() {
        for side in [SidebarSide::Left, SidebarSide::Right] {
            let element: iced::Element<'_, Message, Theme> = sidebar()
                .side(side)
                .child(SidebarMenu::<Message>::new().child(SidebarMenuItem::new("Only")))
                .into();

            drop(element);
        }
    }

    #[test]
    fn a_group_and_a_menu_render_standalone() {
        let group: iced::Element<'_, Message, Theme> = SidebarGroup::new("General").into();
        let menu: iced::Element<'_, Message, Theme> = SidebarMenu::<Message>::new().into();

        drop(group);
        drop(menu);
    }

    #[test]
    fn a_sidebar_defaults_to_a_draggable_floor() {
        let sidebar = Sidebar::<Message>::new();

        assert_eq!(sidebar.min_width, super::DEFAULT_MIN_WIDTH);
        assert!(sidebar.on_resize.is_none(), "resize is opt-in");

        let narrowed = Sidebar::<Message>::new().min_width(140.0);

        assert_eq!(narrowed.min_width, 140.0);
        assert_eq!(Sidebar::<Message>::new().min_width(0.0).min_width, 1.0);
    }

    #[test]
    fn a_resizable_sidebar_renders_where_a_handle_can_exist() {
        let menu = || {
            SidebarMenu::new().child(
                SidebarMenuItem::new("Dashboard")
                    .icon(IconName::LayoutDashboard)
                    .on_select(Message::Picked),
            )
        };

        // Expanded in both collapsible modes, the edge is draggable.
        for collapsible in [SidebarCollapsible::Icon, SidebarCollapsible::Offcanvas] {
            let element: iced::Element<'_, Message, Theme> = sidebar()
                .collapsible(collapsible)
                .on_resize(|_| Message::Toggled)
                .child(SidebarGroup::new("Application").child(menu()))
                .into();

            drop(element);
        }

        // A right-docked sidebar drags from its other edge.
        let element: iced::Element<'_, Message, Theme> = sidebar()
            .side(SidebarSide::Right)
            .on_resize(|_| Message::Toggled)
            .child(menu())
            .into();

        drop(element);
    }

    #[test]
    fn a_submenu_without_a_toggle_stays_at_its_open_flag() {
        let item =
            SidebarMenuItem::<Message>::new("Projects").children([SidebarMenuItem::new("Design")]);

        assert!(item.is_submenu());
        assert!(item.on_toggle.is_none());
    }

    #[test]
    fn items_of_mixed_kinds_compose_into_one_sidebar() {
        let element: iced::Element<'_, Message, Theme> = sidebar()
            .child(SidebarGroup::<Message>::new("Groups"))
            .child(SidebarMenu::<Message>::new())
            .child(SidebarMenuItem::<Message>::new("Bare"))
            .into();

        drop(element);
    }
}
