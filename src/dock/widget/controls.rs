// Ported from iced_dock (MIT, https://github.com/Fee0/iced_dock). See NOTICE.

//! The buttons the dock puts in a tab bar, and the panel menu one of them opens.
//!
//! These belong to the dock rather than to any panel: the dock toggles, which
//! collapse a neighbouring dock, and the zoom control. They are drawn by hand
//! rather than with `button`, because the tab strip draws its whole row into a
//! translated and clipped layer — a `button` brings its own layout pass and state
//! slot, which a bar that scrolls as one row cannot give it.
//!
//! The menu is separate: it is an overlay, so it needs a widget to host it, and
//! [`PanelMenu`] is that widget. It draws its own glyph and owns the open state.

use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::svg as adv_svg;
use iced::advanced::text as adv_text;
use iced::advanced::widget::tree::{State, Tag, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::widget::overlay::menu;
use iced::{Element, Event, Length, Rectangle, Size, Vector};

use crate::dock::model::DockPlacement;
use crate::dock::style::Catalog;
use crate::dock::widget::action::DockAction;

/// A glyph the dock can draw in one of its controls.
///
/// The glyphs come from the bundled icon font — the same set every other
/// component draws from, window controls included — so a dock button, a title
/// bar's maximize and a toolbar's icon share one weight, one optical size and
/// one grid. Drawing them as text is also what lets them recolor like text:
/// hover and press change the color, not the asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockIcon {
    /// Put a left dock away, folding it toward the left edge.
    PanelLeft,
    /// Bring a left dock back, unfolding it over the centre.
    PanelLeftOpen,
    /// Put a right dock away, folding it toward the right edge.
    PanelRight,
    /// Bring a right dock back, unfolding it over the centre.
    PanelRightOpen,
    /// Put a bottom dock away, folding it toward the bottom edge.
    PanelBottom,
    /// Bring a bottom dock back, unfolding it over the centre.
    PanelBottomOpen,
    /// Maximize the panel.
    Maximize,
    /// Restore the panel from maximized.
    Restore,
    /// Open the panel menu.
    Ellipsis,
}

impl DockIcon {
    /// The character this glyph draws, in the icon font.
    ///
    /// The mappings keep the dock in the same visual language as the window
    /// controls: the zoom pair draws exactly what a title bar's maximize draws
    /// — a square, and the overlapped pair a window restores to — and a dock
    /// toggle draws a chevron pointing the way the dock will move, which stays
    /// legible at the twelve-to-fourteen pixels a tab bar affords where a
    /// filled panel pictogram turns to ink.
    #[must_use]
    pub fn glyph(self) -> crate::icons::IconName {
        match self {
            // A left dock and a closed right dock both move the same way, so
            // they share a chevron; likewise a closed left dock and a right one.
            Self::PanelLeft | Self::PanelRightOpen => crate::icons::IconName::ChevronLeft,
            Self::PanelLeftOpen | Self::PanelRight => crate::icons::IconName::ChevronRight,
            Self::PanelBottom => crate::icons::IconName::ChevronDown,
            Self::PanelBottomOpen => crate::icons::IconName::ChevronUp,
            Self::Maximize => crate::icons::IconName::Square,
            Self::Restore => crate::icons::IconName::Copy,
            Self::Ellipsis => crate::icons::IconName::Ellipsis,
        }
    }
}

/// The glyph for a dock toggle, showing what clicking it will do.
///
/// An open dock draws the *collapsed* glyph and a closed one the expanded glyph,
/// so the button reads as the action rather than as the current state. `None` for
/// the centre, which is not a dock.
#[must_use]
pub fn dock_toggle_icon(placement: DockPlacement, open: bool) -> Option<DockIcon> {
    Some(match (placement, open) {
        (DockPlacement::Left, true) => DockIcon::PanelLeft,
        (DockPlacement::Left, false) => DockIcon::PanelLeftOpen,
        (DockPlacement::Right, true) => DockIcon::PanelRight,
        (DockPlacement::Right, false) => DockIcon::PanelRightOpen,
        (DockPlacement::Bottom, true) => DockIcon::PanelBottom,
        (DockPlacement::Bottom, false) => DockIcon::PanelBottomOpen,
        (DockPlacement::Center, _) => return None,
    })
}

/// A square hit target in a tab bar.
#[derive(Debug, Clone, Copy)]
pub struct ControlButton {
    /// Square the control occupies.
    pub bounds: Rectangle,
    /// The glyph drawn in it.
    pub icon: DockIcon,
}

impl ControlButton {
    #[must_use]
    pub fn new(bounds: Rectangle, icon: DockIcon) -> Self {
        Self { bounds, icon }
    }
}

/// Draw one of the dock's control buttons.
///
/// The font bound is `iced::Font` because the glyph is named by family — the
/// icon font registers itself process-wide, and every iced renderer draws text
/// in an `iced::Font` — so the button needs no per-renderer font plumbing.
pub fn draw_control(
    renderer: &mut impl adv_text::Renderer<Font = iced::Font>,
    button: ControlButton,
    style: &crate::dock::style::ControlStyle,
    hovered: bool,
    pressed: bool,
) {
    if hovered || pressed {
        let background = if pressed {
            // Pressing deepens the wash, so the click reads before its effect
            // lands.
            style.text_color.scale_alpha(0.12)
        } else {
            style.hovered_background
        };
        if background.a > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: button.bounds,
                    border: iced::Border {
                        radius: style.border_radius.into(),
                        ..iced::Border::default()
                    },
                    ..renderer::Quad::default()
                },
                background,
            );
        }
    }

    let color = if hovered || pressed {
        style.hovered_text
    } else {
        style.text_color
    };
    let size = style.glyph_size.min(button.bounds.width).min(button.bounds.height);
    // The glyph is text in the icon font, so it recolors through the text
    // pipeline the way every other icon in the library does — the same path a
    // title bar's window controls take.
    crate::icons::load();
    renderer.fill_text(
        adv_text::Text {
            content: crate::icons::glyph(button.icon.glyph()).to_string(),
            bounds: button.bounds.size(),
            size: iced::Pixels(size),
            line_height: adv_text::LineHeight::Relative(1.0),
            font: crate::icons::font(),
            align_x: adv_text::Alignment::Center,
            align_y: iced::alignment::Vertical::Center,
            shaping: adv_text::Shaping::Basic,
            wrapping: adv_text::Wrapping::None,
        },
        // Centered alignment anchors the text *at* this point, the contract
        // iced's own checkbox relies on: the point is the centre, not the
        // top-left corner.
        button.bounds.center(),
        color,
        Rectangle::INFINITE,
    );
}

/// The width a row of `count` controls occupies, gaps included.
///
/// Used to reserve the controls' space in the bar, so a bar crowded by buttons
/// scrolls its tabs rather than letting them run underneath.
#[must_use]
pub fn controls_width(count: usize, style: &crate::dock::style::ControlStyle) -> f32 {
    if count == 0 {
        return 0.0;
    }
    let count = count as f32;
    style.size * count + style.gap * (count - 1.0)
}

/// State of the panel menu hosted by a [`PanelMenu`].
///
/// The menu's own data lives here rather than in the widget because the overlay
/// borrows it for as long as the menu is open, and the widget is rebuilt every
/// frame. Anything the overlay reads has to outlive the frame that opened it.
pub struct PanelMenuState<Message, Theme>
where
    Theme: menu::Catalog,
{
    open: bool,
    menu: menu::State,
    /// Index into the labels the pointer is over.
    hovered: Option<usize>,
    /// Bounds of the button, so a click outside can close the menu.
    button_bounds: Rectangle,
    /// The labels the menu was last built with.
    labels: Vec<String>,
    /// What each label does, in the same order as `labels`.
    ///
    /// Kept as sources rather than as ready-made messages, because building a
    /// message from a dock action *is* dispatching it: a message built while
    /// laying the menu out would fire the action on the frame it was measured.
    sources: Vec<MenuSource<Message>>,
    /// The menu's style class, owned here for the same reason as the labels.
    class: <Theme as menu::Catalog>::Class<'static>,
}

impl<Message, Theme> Default for PanelMenuState<Message, Theme>
where
    Theme: menu::Catalog,
{
    fn default() -> Self {
        Self {
            open: false,
            menu: menu::State::default(),
            hovered: None,
            button_bounds: Rectangle::default(),
            labels: Vec::new(),
            sources: Vec::new(),
            class: <Theme as menu::Catalog>::default(),
        }
    }
}

/// Where a menu entry's outcome comes from.
#[derive(Debug, Clone)]
pub enum MenuSource<Message> {
    /// The panel supplied the message itself, so it is already known.
    Panel(Message),
    /// The dock owns this entry. The action is dispatched through the widget's
    /// event handler when the entry is chosen, not when the menu is built.
    Dock(DockAction),
}

/// One entry the panel menu offers.
pub struct MenuEntry<Message> {
    /// The label.
    pub label: String,
    /// The message selecting it produces.
    pub message: Message,
}

impl<Message> MenuEntry<Message> {
    #[must_use]
    pub fn new(label: impl Into<String>, message: Message) -> Self {
        Self {
            label: label.into(),
            message,
        }
    }
}

pub struct PanelMenu<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
{
    /// The button's element id, so it can be addressed without its glyph.
    id: iced::advanced::widget::Id,
    pane: crate::dock::model::NodeId,
    /// The panel the built-in entries act on: the one displayed.
    ///
    /// `Close` names a *panel* node, so without this the entry would have only a
    /// pane to name and the state would refuse it.
    panel: Option<crate::dock::model::NodeId>,
    /// Whether the panel is currently zoomed, which decides the zoom entry.
    zoomed: bool,
    /// Whether the panel allows zooming at all, which decides whether the entry
    /// is offered.
    can_zoom: bool,
    /// Whether the panel may be closed.
    can_close: bool,
    /// The panel's own entries, drawn first.
    entries: Vec<MenuEntry<Message>>,
    on_event: Rc<dyn Fn(DockAction) -> Message>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    size: f32,
    icon: DockIcon,
    _marker: std::marker::PhantomData<(&'a (), Renderer)>,
}

impl<Message, Theme, Renderer> PanelMenu<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + menu::Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer
        + adv_svg::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>
        + 'static,
{
    pub(crate) fn new(
        id: iced::advanced::widget::Id,
        pane: crate::dock::model::NodeId,
        panel: Option<crate::dock::model::NodeId>,
        zoomed: bool,
        can_zoom: bool,
        can_close: bool,
        entries: Vec<MenuEntry<Message>>,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        class: Rc<<Theme as Catalog>::Class<'static>>,
        size: f32,
    ) -> Self {
        Self {
            id,
            pane,
            panel,
            zoomed,
            can_zoom,
            can_close,
            entries,
            on_event,
            class,
            size,
            icon: DockIcon::Ellipsis,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for PanelMenu<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + menu::Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer
        + adv_svg::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>
        + 'static,
{
    fn tag(&self) -> Tag {
        Tag::of::<PanelMenuState<Message, Theme>>()
    }

    fn state(&self) -> State {
        State::new(PanelMenuState::<Message, Theme>::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.size),
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        // The entries are assembled here rather than in `overlay`, because the
        // overlay borrows them for as long as the menu is open and this widget is
        // rebuilt every frame: whatever the overlay reads has to live in the tree
        // state, and this is the pass that fills it.
        //
        // The panel's own entries come first, then the dock's — zoom, then close.
        // A panel never has to implement either, and one that forgets to pass its
        // list back cannot drop them.
        let pane = self.pane;
        let mut labels: Vec<String> = self
            .entries
            .iter()
            .map(|entry| entry.label.clone())
            .collect();
        let mut sources: Vec<MenuSource<Message>> = self
            .entries
            .iter()
            .map(|entry| MenuSource::Panel(entry.message.clone()))
            .collect();
        if self.can_zoom {
            labels.push(if self.zoomed { "Restore" } else { "Maximize" }.to_owned());
            sources.push(MenuSource::Dock(DockAction::ToggleZoom { pane }));
        }
        if self.can_close {
            if let Some(panel) = self.panel {
                labels.push("Close".to_owned());
                sources.push(MenuSource::Dock(DockAction::Tab(
                    crate::dock::widget::action::TabAction::Close { panel },
                )));
            }
        }

        let state = tree.state.downcast_mut::<PanelMenuState<Message, Theme>>();
        state.labels = labels;
        state.sources = sources;
        // The hit square, recorded here because the node this returns fills the whole
        // bar: the button is a square centred in it, and `draw` and `update` both need
        // to agree on where that square is.
        let size = self.size.min(max.height);
        state.button_bounds = Rectangle {
            x: 0.0,
            y: (max.height - size) / 2.0,
            width: size,
            height: size,
        };
        layout::Node::new(Size::new(self.size, max.height))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<PanelMenuState<Message, Theme>>();
        let style = Catalog::style(theme, &self.class);
        let mut bounds = state.button_bounds;
        let origin = layout.position();
        bounds.x += origin.x;
        bounds.y += origin.y;
        let button = ControlButton::new(bounds, self.icon);
        let hovered = cursor.is_over(bounds);
        draw_control(renderer, button, &style.control, hovered || state.open, false);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let origin = layout.position();
        let state = tree.state.downcast_mut::<PanelMenuState<Message, Theme>>();
        let mut bounds = state.button_bounds;
        bounds.x += origin.x;
        bounds.y += origin.y;

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
            | Event::Touch(iced::touch::Event::FingerPressed { .. }) => {
                if let Some(point) = cursor.position() {
                    if bounds.contains(point) {
                        state.open = !state.open;
                        shell.capture_event();
                        shell.request_redraw();
                    }
                }
            }
            Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if state.open
                    && matches!(
                        key,
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                    ) =>
            {
                state.open = false;
                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<PanelMenuState<Message, Theme>>();
        let mut bounds = state.button_bounds;
        let origin = layout.position();
        bounds.x += origin.x;
        bounds.y += origin.y;
        if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(Some(&self.id), layout.bounds());
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_mut::<PanelMenuState<Message, Theme>>();
        if !state.open || state.labels.is_empty() {
            return None;
        }

        let origin = layout.position();
        let anchor = iced::Point::new(
            origin.x + state.button_bounds.x,
            origin.y + state.button_bounds.y + state.button_bounds.height,
        );

        // The entries live in the state, not here: the overlay borrows them for as
        // long as the menu is open, and this widget is rebuilt every frame.
        let on_event = Rc::clone(&self.on_event);
        // Cloned out rather than moved: the overlay outlives this call, and the
        // state must keep the entries for the frames that follow.
        let sources = Rc::new(state.sources.clone());
        let dispatch = Rc::new(move |source: &MenuSource<Message>| match source {
            MenuSource::Panel(message) => message.clone(),
            // Built only now, when the entry is chosen: building it while the menu
            // was laid out would have fired the action then.
            MenuSource::Dock(action) => (on_event)(action.clone()),
        });
        let labels = &state.labels;
        let menu = menu::Menu::new(
            &mut state.menu,
            labels,
            &mut state.hovered,
            move |selected: String| {
                // The option is a label, so find which slot it came from: two
                // entries may share a label, and what the entry does is what matters.
                let index = labels.iter().position(|label| label == &selected).unwrap_or(0);
                // An index the menu cannot have produced would mean the labels and
                // the sources disagree; falling back to the last entry keeps a
                // malformed menu from panicking inside an overlay.
                let source = sources
                    .get(index)
                    .or_else(|| sources.last())
                    .expect("the menu is only built with at least one entry");
                (dispatch)(source)
            },
            None,
            &state.class,
        )
        .width(self.size.max(140.0))
        .padding(4);

        Some(menu.overlay(anchor, *viewport, 0.0, iced::Length::Shrink))
    }
}

impl<'a, Message, Theme, Renderer> From<PanelMenu<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + menu::Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer
        + adv_svg::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>
        + 'static,
{
    fn from(menu: PanelMenu<'a, Message, Theme, Renderer>) -> Self {
        Element::new(menu)
    }
}

/// A plain control button as a widget, for the dock toggles.
///
/// Unlike [`PanelMenu`] it hosts no overlay: clicking it dispatches its action
/// directly, so it needs no state of its own.
pub struct ToggleButton<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>
where
    Theme: Catalog,
{
    /// The button's element id.
    ///
    /// A control drawn as an icon has no text to be found by, so an id is the only
    /// way to address it — which a test needs, and so does anything that drives the
    /// dock programmatically.
    id: iced::advanced::widget::Id,
    icon: DockIcon,
    action: DockAction,
    on_event: Option<Rc<dyn Fn(DockAction) -> Message>>,
    class: Rc<<Theme as Catalog>::Class<'static>>,
    size: f32,
    /// Whether the dock this toggles is currently open, for the hover tint.
    active: bool,
    _marker: std::marker::PhantomData<(&'a (), Renderer)>,
}

impl<Message, Theme, Renderer> ToggleButton<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer
        + adv_svg::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>
        + 'static,
{
    pub(crate) fn new(
        id: iced::advanced::widget::Id,
        icon: DockIcon,
        action: DockAction,
        on_event: Rc<dyn Fn(DockAction) -> Message>,
        class: Rc<<Theme as Catalog>::Class<'static>>,
        size: f32,
        active: bool,
    ) -> Self {
        Self {
            id,
            icon,
            action,
            on_event: Some(on_event),
            class,
            size,
            active,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ToggleButton<'_, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer
        + adv_svg::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>
        + 'static,
{
    fn tag(&self) -> Tag {
        Tag::of::<()>()
    }

    fn state(&self) -> State {
        State::new(())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(self.size),
            height: Length::Fill,
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let max = limits.max();
        layout::Node::new(Size::new(self.size, max.height))
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        let style = Catalog::style(theme, &self.class);
        let bounds = layout.bounds();
        let size = style.control.size.min(bounds.height);
        let button = ControlButton::new(
            Rectangle {
                x: bounds.x,
                y: bounds.y + (bounds.height - size) / 2.0,
                width: size,
                height: size,
            },
            self.icon,
        );
        let hovered = cursor.is_over(button.bounds);
        draw_control(renderer, button, &style.control, hovered || self.active, false);
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if !matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(iced::touch::Event::FingerPressed { .. })
        ) {
            return;
        }
        let bounds = layout.bounds();
        let Some(point) = cursor.position() else {
            return;
        };
        if !bounds.contains(point) {
            return;
        }
        if let Some(on_event) = &self.on_event {
            shell.publish((on_event)(self.action.clone()));
        }
        shell.capture_event();
        shell.request_redraw();
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        // Reported as a container with an id: that is the shape the finder reads,
        // so the control can be addressed by id rather than by the glyph it draws.
        operation.container(Some(&self.id), layout.bounds());
    }

    fn overlay<'b>(
        &'b mut self,
        _tree: &'b mut Tree,
        _layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        None
    }
}

impl<'a, Message, Theme, Renderer> From<ToggleButton<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'static,
    Theme: Catalog + Clone + PartialEq + 'static,
    Renderer: renderer::Renderer
        + adv_svg::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>
        + 'static,
{
    fn from(button: ToggleButton<'a, Message, Theme, Renderer>) -> Self {
        Element::new(button)
    }
}

/// Truncate an application-built control to a square of the bar's height.
///
/// A toolbar button is built by the application, which does not know how tall the
/// tab bar is. Squeezing it here is what keeps a row of application buttons the
/// same height as the dock's own zoom and menu buttons beside them.
pub(crate) fn fit_to_bar<'a, Message: 'a, Theme, Renderer>(
    element: Element<'a, Message, Theme, Renderer>,
    size: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Theme: iced::widget::container::Catalog + 'a,
    Renderer: renderer::Renderer + 'a,
{
    iced::widget::container(element)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .padding(0)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{dock_toggle_icon, DockIcon};
    use crate::dock::model::DockPlacement;
    use crate::icons::{self, IconName};

    #[test]
    fn every_dock_icon_resolves_to_a_font_glyph() {
        for icon in [
            DockIcon::PanelLeft,
            DockIcon::PanelLeftOpen,
            DockIcon::PanelRight,
            DockIcon::PanelRightOpen,
            DockIcon::PanelBottom,
            DockIcon::PanelBottomOpen,
            DockIcon::Maximize,
            DockIcon::Restore,
            DockIcon::Ellipsis,
        ] {
            let glyph = icons::glyph(icon.glyph());
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(&glyph),
                "{icon:?} resolved to {glyph:?}, which is not an icon-font glyph"
            );
        }
    }

    #[test]
    fn the_zoom_pair_matches_the_window_controls() {
        // The zoom button draws exactly what the matching window control draws,
        // so a maximized panel and the window around it read as one interface:
        // a maximize square straight from the title bar's own mapping, and the
        // overlapped pair a window restores to. Compared through the resolved
        // glyph, because the icon enum implements neither `PartialEq` nor `Eq`.
        assert_eq!(
            icons::glyph(DockIcon::Maximize.glyph()),
            icons::glyph(crate::widgets::title_bar::WindowControl::Maximize.icon())
        );
        assert_eq!(
            icons::glyph(DockIcon::Restore.glyph()),
            icons::glyph(IconName::Copy)
        );
    }

    #[test]
    fn a_toggle_points_the_way_the_dock_moves() {
        // An open dock folds toward its own edge; a closed one unfolds back
        // over the centre. The chevron says which, the way a window's own
        // collapse affordances do.
        use DockIcon as I;
        let expected = [
            (DockPlacement::Left, I::PanelLeft, I::PanelLeftOpen),
            (DockPlacement::Right, I::PanelRight, I::PanelRightOpen),
            (DockPlacement::Bottom, I::PanelBottom, I::PanelBottomOpen),
        ];
        for (placement, open, closed) in expected {
            assert_eq!(dock_toggle_icon(placement, true), Some(open));
            assert_eq!(dock_toggle_icon(placement, false), Some(closed));
            assert_ne!(
                icons::glyph(open.glyph()),
                icons::glyph(closed.glyph()),
                "{placement:?} must draw different glyphs open and closed"
            );
        }
        // The centre is not a dock, so it draws no toggle.
        assert!(dock_toggle_icon(DockPlacement::Center, true).is_none());
    }
}
