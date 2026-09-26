//! Tabs.
//!
//! [`tabs`] renders a horizontal strip of selectable labels. It is stateless:
//! the caller owns the selection and receives a message when it changes.
//!
//! The selected tab is marked by an underline that slides between tabs rather
//! than jumping, which is what shows the selection as one thing moving through
//! the strip instead of two unrelated states.
//!
//! # The variants
//!
//! [`tabs`] returns a [`TabStrip`] builder whose [`TabStrip::variant`] selects
//! how the strip marks its selection, matching the reference's `TabVariant`:
//!
//! - [`TabVariant::Underline`] — a rule that slides under the selected label.
//!   The default, and the only variant the indicator overlay is used for.
//! - [`TabVariant::Tab`] — a bordered folder-tab look: the selected tab takes
//!   the page surface and a top border, sitting flush with the strip's edge.
//! - [`TabVariant::Outline`] — every tab is outlined; the selected one is
//!   outlined in the primary color.
//! - [`TabVariant::Pill`] — the selected tab is a filled primary pill.
//! - [`TabVariant::Segmented`] — a muted track with the selected tab as a
//!   raised background-colored segment.
//!
//! The four non-underline variants do not slide: their selection is painted by
//! the tab itself, so there is nothing for an overlay to interpolate.

use crate::theme::{Size, Theme};
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Shell};
use iced::time::Instant;
use iced::widget::{button, container, row, text};
use iced::{Color, Element, Event, Length, Padding, Rectangle, Size as IcedSize};

/// The thickness of the underline marking the selected tab, in logical pixels.
const INDICATOR_THICKNESS: f32 = 2.0;

/// How a tab strip marks its selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabVariant {
    /// A sliding rule under the selected label. The default.
    #[default]
    Underline,
    /// Bordered tabs with the selected one taking the page surface.
    Tab,
    /// Every tab outlined; the selected one in the primary color.
    Outline,
    /// The selected tab is a filled primary pill.
    Pill,
    /// A muted track with the selected tab raised on it.
    Segmented,
}

impl TabVariant {
    /// The name this variant is read from a config by.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Underline => "underline",
            Self::Tab => "tab",
            Self::Outline => "outline",
            Self::Pill => "pill",
            Self::Segmented => "segmented",
        }
    }

    /// The variant a name refers to, for a caller reading one from a config.
    ///
    /// An unrecognized name falls back to the default rather than failing: a
    /// variant is presentation, and a config that names one this version does
    /// not know is better drawn plainly than refused.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "tab" => Self::Tab,
            "outline" => Self::Outline,
            "pill" => Self::Pill,
            "segmented" => Self::Segmented,
            _ => Self::Underline,
        }
    }

    /// Whether the selection is drawn by a sliding overlay.
    ///
    /// Only the underline variant needs one; the others paint the selection as
    /// part of the tab, so an overlay would be a second, contradictory marker.
    fn slides(self) -> bool {
        matches!(self, Self::Underline)
    }

    /// The strip's own background and padding.
    fn strip(self, theme: &Theme) -> (Option<Color>, Padding) {
        let colors = theme.colors();

        match self {
            Self::Segmented => (
                Some(colors.muted),
                Padding {
                    top: 2.0,
                    right: 2.0,
                    bottom: 2.0,
                    left: 2.0,
                },
            ),
            _ => (None, Padding::ZERO),
        }
    }

    /// The gap between adjacent tabs.
    fn gap(self) -> f32 {
        match self {
            Self::Underline | Self::Tab => 0.0,
            Self::Outline | Self::Pill => 4.0,
            Self::Segmented => 2.0,
        }
    }

    /// The corner radius of a tab.
    fn radius(self, theme: &Theme) -> f32 {
        match self {
            Self::Underline | Self::Tab => 0.0,
            Self::Outline | Self::Pill => f32::from(theme.radius().full),
            Self::Segmented => f32::from(theme.radius().md),
        }
    }
}

/// One tab in a [`tabs`] strip.
#[derive(Debug, Clone)]
#[must_use = "a Tab does nothing unless it is given to `tabs`"]
pub struct Tab {
    label: String,
    enabled: bool,
    icon: Option<crate::icons::IconName>,
    prefix: Option<String>,
    suffix: Option<String>,
}

impl Tab {
    /// Creates an enabled tab with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            icon: None,
            prefix: None,
            suffix: None,
        }
    }

    /// Enables or disables the tab.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Draws an icon before the label.
    pub fn icon(mut self, icon: crate::icons::IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Draws a short run of text before the icon, for a count or a status dot.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Draws a short run of text after the label, for a count or a badge.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Returns the tab's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether the tab can be selected.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Custom colors for a tab strip, for a strip embedded in a tinted surface —
/// a ribbon's title bar, say — where the global theme's colors do not apply.
///
/// Every field is optional; a `None` field falls back to the global theme, so
/// a caller only overrides the roles it needs to recolour.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TabStripColors {
    /// Text color of an unselected tab.
    pub text: Option<Color>,
    /// Text color of the selected tab.
    pub text_selected: Option<Color>,
    /// Text color of a hovered, unselected tab.
    pub text_hover: Option<Color>,
    /// Background painted behind the selected tab in the [`Tab`] variant.
    pub selected_background: Option<Color>,
    /// Border color of the selected tab in the [`Tab`] variant.
    pub selected_border: Option<Color>,
    /// The sliding indicator's color (the [`Underline`](TabVariant::Underline)
    /// variant only).
    pub indicator: Option<Color>,
}

/// A tab strip under construction.
///
/// [`tabs`] returns this, so the variant and size are set on the builder:
///
/// ```
/// # use iced_kit::widgets::{tabs, Tab, TabVariant};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Selected(usize) }
/// # fn view(current: usize) -> Element<'static, Message, Theme> {
/// tabs(vec![Tab::new("General"), Tab::new("Advanced")], current, Message::Selected)
///     .variant(TabVariant::Pill)
///     .into()
/// # }
/// ```
#[must_use = "a TabStrip does nothing unless it is turned into an Element"]
pub struct TabStrip<'a, Message> {
    tabs: Vec<Tab>,
    selected: usize,
    on_select: Box<dyn Fn(usize) -> Message + 'a>,
    variant: TabVariant,
    size: Size,
    colors: Option<TabStripColors>,
}

impl<'a, Message: Clone + 'a> TabStrip<'a, Message> {
    /// Sets how the strip marks its selection.
    pub fn variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the size step, which scales the tab height, padding and text.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Overrides the strip's colors, for a strip embedded in a tinted surface
    /// such as a ribbon's title bar.
    pub fn colors(mut self, colors: TabStripColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Turns the strip into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        build_strip(self)
    }
}

impl<'a, Message: Clone + 'a> From<TabStrip<'a, Message>> for Element<'a, Message, Theme> {
    fn from(strip: TabStrip<'a, Message>) -> Self {
        strip.into_element()
    }
}

/// Builds a horizontal tab strip.
///
/// `on_select` receives the index of the tab that was clicked. A disabled tab
/// never produces a message.
///
/// ```
/// # use iced_kit::widgets::{tabs, Tab};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Selected(usize) }
/// # fn view(current: usize) -> Element<'static, Message, Theme> {
/// tabs(
///     vec![Tab::new("General"), Tab::new("Advanced")],
///     current,
///     Message::Selected,
/// )
/// .into()
/// # }
/// ```
pub fn tabs<'a, Message: Clone + 'a>(
    tabs: Vec<Tab>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> TabStrip<'a, Message> {
    TabStrip {
        tabs,
        selected,
        on_select: Box::new(on_select),
        variant: TabVariant::default(),
        size: Size::Md,
        colors: None,
    }
}

/// Lays out a strip once its options are settled.
fn build_strip<'a, Message: Clone + 'a>(
    strip: TabStrip<'a, Message>,
) -> Element<'a, Message, Theme> {
    let TabStrip {
        tabs,
        selected,
        on_select,
        variant,
        size,
        colors,
    } = strip;

    let text_style = size.text();
    let height = tab_height(variant, size);
    let count = tabs.len();

    // The strip's own gap and padding, and the tab's, are read once here rather
    // than per tab: they are decided by the variant and size, not by the tab.
    let gap = variant.gap();
    let seg_padding = variant.strip(&Theme::light()).1;
    let seg_padding = Padding {
        top: seg_padding.top.max(0.0),
        ..seg_padding
    };

    let strip_row =
        tabs.into_iter()
            .enumerate()
            .fold(row![].spacing(gap), |strip_row, (index, tab)| {
                let is_selected = index == selected;

                // The line box is the tab's own height, not the text's. A raw
                // iced button lays its content out at its padding origin without
                // centring it, so a text-height line box leaves the label against
                // the top of the tab. Sizing the box to the control is what puts
                // the baseline where the eye expects it.
                let label = text(tab.label.clone())
                    .size(text_style.size)
                    .line_height(iced::Pixels(height.max(text_style.line_height)));

                // Icon, prefix and suffix sit on the tab's centre line beside
                // the label, spaced by the size step's gap.
                let content: Element<'_, Message, Theme> = {
                    let mut parts = row![].spacing(size.gap()).align_y(iced::Alignment::Center);

                    if let Some(prefix) = tab.prefix.clone() {
                        parts = parts.push(
                            text(prefix)
                                .size(text_style.size)
                                .line_height(iced::Pixels(height.max(text_style.line_height))),
                        );
                    }

                    if let Some(icon) = tab.icon {
                        parts = parts.push(crate::widgets::Icon::new(icon).into_element(size));
                    }

                    parts = parts.push(label);

                    if let Some(suffix) = tab.suffix.clone() {
                        parts = parts.push(
                            text(suffix)
                                .size(text_style.size)
                                .line_height(iced::Pixels(height.max(text_style.line_height))),
                        );
                    }

                    parts.into()
                };

                // Every variant pads its tab by the size step; the underline's
                // rule sits outside the label either way.
                let horizontal_padding = match size {
                    Size::Xs => 8.0,
                    Size::Sm => 10.0,
                    Size::Lg => 16.0,
                    _ => 12.0,
                };

                let colors_for_tab = colors;
                let mut widget = button(content)
                    .padding(Padding {
                        top: 0.0,
                        right: horizontal_padding,
                        bottom: 0.0,
                        left: horizontal_padding,
                    })
                    .height(Length::Fixed(height))
                    .class(Box::new(move |theme: &Theme, status| {
                        tab_style(theme, status, is_selected, variant, size, colors_for_tab)
                    }) as button::StyleFn<'a, Theme>);

                if tab.enabled {
                    widget = widget.on_press(on_select(index));
                }

                strip_row.push(widget)
            });

    // The segmented variant draws a track behind its tabs; the others are
    // transparent, so no container is built for them at all.
    let strip: Element<'a, Message, Theme> = match variant.strip(&Theme::light()).0 {
        None => strip_row.into(),
        Some(_) => container(strip_row)
            .padding(seg_padding)
            .class(Box::new(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(theme.colors().muted)),
                border: iced::Border {
                    radius: f32::from(theme.radius().lg).into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            }) as container::StyleFn<'a, Theme>)
            .into(),
    };

    if !variant.slides() {
        // Nothing needs interpolating, so the row is the whole strip.
        return strip;
    }

    // A stale index selects nothing, so the indicator is simply absent rather
    // than pointing at a tab that is not there.
    let selected = (selected < count).then_some(selected);
    let indicator_color = colors.and_then(|colors| colors.indicator);

    TabStripWidget::new(strip, selected, INDICATOR_THICKNESS, indicator_color).into()
}

/// The height of a tab at this variant and size.
///
/// Ported from the reference's `TabVariant::height`, whose underline tabs are
/// shorter because the rule takes the difference.
fn tab_height(variant: TabVariant, size: Size) -> f32 {
    match variant {
        // The underline's rule sits below the tab, so its box is taller by
        // that much at every step.
        TabVariant::Underline => match size {
            Size::Xs => 26.0,
            Size::Sm => 30.0,
            Size::Lg => 44.0,
            _ => 36.0,
        },
        _ => match size {
            Size::Xs => 20.0,
            Size::Sm => 24.0,
            Size::Lg => 36.0,
            _ => 32.0,
        },
    }
}

/// A row of tabs with a sliding underline.
///
/// It wraps the row rather than being one, so the indicator can be drawn on top
/// of the tabs at a position interpolated between them. iced has nothing that
/// draws an underline under a flex item, and the position is only knowable once
/// the row has been laid out.
struct TabStripWidget<'a, Message, Renderer = iced::Renderer> {
    row: Element<'a, Message, Theme, Renderer>,
    /// The selected tab, or `None` when nothing is selected.
    selected: Option<usize>,
    thickness: f32,
    /// The indicator's color, overriding the theme's primary.
    indicator: Option<Color>,
}

impl<'a, Message, Renderer> TabStripWidget<'a, Message, Renderer> {
    fn new(
        row: Element<'a, Message, Theme, Renderer>,
        selected: Option<usize>,
        thickness: f32,
        indicator: Option<Color>,
    ) -> Self {
        Self {
            row,
            selected,
            thickness,
            indicator,
        }
    }
}

/// The indicator's state between frames.
#[derive(Debug, Clone, Copy, Default)]
struct StripState {
    /// The indicator's left edge and width, each spring-driven so the underline
    /// glides between tabs. Two springs rather than one because tabs differ in
    /// width: a single value would make the underline jump in size.
    left: crate::motion::SpringState,
    width: crate::motion::SpringState,
    /// The frame the springs were last advanced to.
    last: Option<Instant>,
    /// Whether the indicator has ever been placed, so the first frame does not
    /// slide in from the left edge.
    primed: bool,
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for TabStripWidget<'_, Message, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<StripState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(StripState::default())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.row)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.row));
    }

    fn size(&self) -> IcedSize<Length> {
        self.row.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.row
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.row
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // The tabs themselves take the events; the strip only draws over them.
        self.row.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        // Where the underline belongs is only knowable from the laid-out tabs,
        // because how wide a label ends up depends on the font.
        let Some(target) = self
            .selected
            .and_then(|selected| selected_tab_bounds(layout, selected))
        else {
            return;
        };

        let state = tree.state.downcast_mut::<StripState>();

        if !state.primed {
            // The first frame places the underline instead of sliding it in
            // from the left edge, which would read as the strip selecting
            // itself on startup.
            state.left.set(target.x);
            state.width.set(target.width);
            state.primed = true;
        }

        // Every frame steps towards the selected tab rather than only the frame
        // the selection changed on. A spring is retargeted by being told its
        // target again, so this is also what carries a half-finished slide
        // through a second click: the underline is caught in mid-flight and
        // turned around instead of restarting.
        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let elapsed = state
                .last
                .map_or(std::time::Duration::ZERO, |last| now.duration_since(last));
            state.last = Some(*now);

            // A reduced-motion application places the underline on its tab
            // rather than gliding to it.
            let spring = if crate::motion::reduce_motion() {
                crate::motion::Spring::new(std::time::Duration::ZERO)
            } else {
                crate::motion::Spring::new(crate::motion::DURATION_NORMAL).with_epsilon(0.05)
            };

            state.left.step(target.x, spring, elapsed);
            state.width.step(target.width, spring, elapsed);

            if !state.left.is_settled(target.x, spring)
                || !state.width.is_settled(target.width, spring)
            {
                shell.request_redraw();
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.row.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !bounds.intersects(viewport) {
            return;
        }

        self.row.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );

        let state = tree.state.downcast_ref::<StripState>();

        // Nothing has been selected yet, so there is no underline to draw.
        if !state.primed {
            return;
        }

        let width = state.width.value().max(0.0);
        if width <= 0.0 {
            return;
        }

        let indicator = Rectangle {
            x: state.left.value(),
            y: bounds.y + bounds.height - self.thickness,
            width,
            height: self.thickness,
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: indicator,
                border: iced::Border {
                    // Squared ends, so the underline reads as a rule under the
                    // label rather than as a pill.
                    radius: f32::from(theme.radius().sm).into(),
                    ..iced::Border::default()
                },
                shadow: iced::Shadow::default(),
                // Not snapped: the indicator spends its time between tabs, and
                // pixel-snapping a moving quad makes it stutter.
                snap: false,
            },
            self.indicator.unwrap_or(theme.colors().primary),
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.row.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Renderer> From<TabStripWidget<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(strip: TabStripWidget<'a, Message, Renderer>) -> Self {
        Element::new(strip)
    }
}

/// The laid-out bounds of the tab at `selected`.
///
/// The strip's `layout` returns the row's own node rather than a wrapper around
/// it, so `layout`'s children are the tabs themselves and the index is the tab
/// index. Descending into a tab first — which this once did — looks for the
/// `selected`th child of a single tab, which does not exist; the lookup then
/// fails, the underline is never placed, and the strip silently stops
/// following the selection.
///
/// It is a free function so it can be tested against a real laid-out row.
fn selected_tab_bounds(layout: layout::Layout<'_>, selected: usize) -> Option<Rectangle> {
    layout.children().nth(selected).map(|tab| tab.bounds())
}

/// The appearance of a single tab, for the variant it belongs to.
///
/// `colors` overrides the global theme's roles for a strip embedded in a
/// tinted surface; `None` fields fall back to the theme.
fn tab_style(
    theme: &Theme,
    status: button::Status,
    is_selected: bool,
    variant: TabVariant,
    size: Size,
    colors: Option<TabStripColors>,
) -> button::Style {
    let theme_colors = theme.colors();
    let hovered =
        matches!(status, button::Status::Hovered) && !matches!(status, button::Status::Disabled);
    let disabled = matches!(status, button::Status::Disabled);

    let muted = || {
        colors
            .and_then(|c| c.text)
            .unwrap_or(theme_colors.muted_foreground)
    };
    let foreground = || {
        colors
            .and_then(|c| c.text_selected)
            .unwrap_or(theme_colors.foreground)
    };
    let hover_text = || {
        colors
            .and_then(|c| c.text_hover)
            .unwrap_or(theme_colors.foreground)
    };

    let radius = variant.radius(theme);

    // The underline's rule is drawn by the strip above the row rather than as a
    // border here: it slides between tabs, so it cannot belong to one of them.
    // Every other variant paints its own selection.
    let mut style = button::Style {
        background: None,
        text_color: muted(),
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    };

    if disabled {
        style.text_color = Color {
            a: muted().a * 0.6,
            ..muted()
        };
        // A disabled tab must not highlight on hover, but its selection still
        // shows: it is where the strip is, not an affordance.
        match variant {
            TabVariant::Underline | TabVariant::Tab => {}
            TabVariant::Outline => {
                style.border = iced::Border {
                    color: if is_selected {
                        colors
                            .and_then(|c| c.selected_border)
                            .unwrap_or(theme_colors.primary)
                    } else {
                        theme_colors.border
                    },
                    width: 1.0,
                    radius: radius.into(),
                };
            }
            TabVariant::Pill => {
                if is_selected {
                    style.background = Some(iced::Background::Color(fade(
                        colors
                            .and_then(|c| c.selected_background)
                            .unwrap_or(theme_colors.primary),
                        0.5,
                    )));
                    style.text_color = fade(theme_colors.primary_foreground, 0.5);
                }
            }
            TabVariant::Segmented => {
                if is_selected {
                    style.background = Some(iced::Background::Color(theme_colors.background));
                }
            }
        }
        return style;
    }

    match variant {
        TabVariant::Underline => {
            style.text_color = if is_selected { foreground() } else { muted() };
        }
        TabVariant::Tab => {
            style.text_color = if is_selected { foreground() } else { muted() };
            if is_selected {
                style.background = Some(iced::Background::Color(
                    colors
                        .and_then(|c| c.selected_background)
                        .unwrap_or(theme_colors.surface),
                ));
                style.border = iced::Border {
                    color: colors
                        .and_then(|c| c.selected_border)
                        .unwrap_or(theme_colors.border),
                    // Only the top and sides are ruled, so the tab reads as
                    // joined to the page below it.
                    width: 1.0,
                    radius: iced::border::Radius {
                        top_left: f32::from(theme.radius().md),
                        top_right: f32::from(theme.radius().md),
                        bottom_left: 0.0,
                        bottom_right: 0.0,
                    },
                };
            } else if hovered {
                style.text_color = hover_text();
            }
        }
        TabVariant::Outline => {
            style.text_color = if is_selected {
                colors
                    .and_then(|c| c.selected_border)
                    .unwrap_or(theme_colors.primary)
            } else {
                muted()
            };
            style.border = iced::Border {
                color: if is_selected {
                    colors
                        .and_then(|c| c.selected_border)
                        .unwrap_or(theme_colors.primary)
                } else {
                    theme_colors.border
                },
                width: 1.0,
                radius: radius.into(),
            };
            if hovered && !is_selected {
                style.background = Some(iced::Background::Color(theme_colors.accent));
            }
        }
        TabVariant::Pill => {
            if is_selected {
                style.background = Some(iced::Background::Color(
                    colors
                        .and_then(|c| c.selected_background)
                        .unwrap_or(theme_colors.primary),
                ));
                style.text_color = theme_colors.primary_foreground;
            } else {
                style.text_color = muted();
                if hovered {
                    style.background = Some(iced::Background::Color(theme_colors.secondary));
                    style.text_color = theme_colors.secondary_foreground;
                }
            }
        }
        TabVariant::Segmented => {
            style.text_color = if is_selected { foreground() } else { muted() };
            if is_selected {
                // The raised segment is the page color on the muted track.
                style.background = Some(iced::Background::Color(theme_colors.background));
            }
        }
    }

    // The size only reaches the style through the radius scale for segmented
    // tabs, whose inner radius is one step tighter than the track's.
    if variant == TabVariant::Segmented && matches!(size, Size::Xs | Size::Sm) {
        style.border.radius = iced::border::Radius {
            top_left: f32::from(theme.radius().sm),
            top_right: f32::from(theme.radius().sm),
            bottom_left: f32::from(theme.radius().sm),
            bottom_right: f32::from(theme.radius().sm),
        };
    }

    style
}

/// A color at a fraction of its opacity.
fn fade(color: Color, opacity: f32) -> Color {
    Color {
        a: color.a * opacity,
        ..color
    }
}

#[cfg(test)]
mod tests {
    use super::{tabs, Tab};
    use crate::theme::{Size, Theme};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Selected(usize),
    }

    #[test]
    fn a_tab_strip_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(
            vec![Tab::new("One"), Tab::new("Two"), Tab::new("Three")],
            0,
            Message::Selected,
        )
        .into();
        drop(element);
    }

    #[test]
    fn a_disabled_tab_still_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(
            vec![Tab::new("Enabled"), Tab::new("Disabled").enabled(false)],
            0,
            Message::Selected,
        )
        .into();
        drop(element);
    }

    #[test]
    fn an_empty_strip_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(vec![], 0, Message::Selected).into();
        drop(element);
    }

    #[test]
    fn a_selection_index_beyond_the_strip_renders_no_selection() {
        // A stale index must not panic; it simply selects nothing.
        let element: iced::Element<'_, Message, Theme> =
            tabs(vec![Tab::new("Only")], 99, Message::Selected).into();
        drop(element);
    }

    /// The selected tab is distinguished by its text color; the underline that
    /// makes it a tab rather than a button is drawn by the strip.
    #[test]
    fn tab_style_marks_the_selected_tab() {
        use super::TabVariant;
        use iced::widget::button::Status;

        let theme = Theme::light();
        let selected = super::tab_style(
            &theme,
            Status::Active,
            true,
            TabVariant::Underline,
            Size::Md,
            None,
        );
        let unselected = super::tab_style(
            &theme,
            Status::Active,
            false,
            TabVariant::Underline,
            Size::Md,
            None,
        );

        assert_eq!(selected.text_color, theme.colors().foreground);
        assert_eq!(unselected.text_color, theme.colors().muted_foreground);
        assert_ne!(selected.text_color, unselected.text_color);
    }

    /// A tab must not draw an underline of its own, or there would be two: the
    /// strip draws the one that slides.
    #[test]
    fn no_underline_tab_draws_its_own_rule() {
        use super::TabVariant;
        use iced::widget::button::Status;

        let theme = Theme::light();

        for is_selected in [true, false] {
            for status in [Status::Active, Status::Hovered, Status::Disabled] {
                let style = super::tab_style(
                    &theme,
                    status,
                    is_selected,
                    TabVariant::Underline,
                    Size::Md,
                    None,
                );

                assert_eq!(
                    style.border.width, 0.0,
                    "the strip owns the underline, not the tab"
                );
            }
        }
    }

    /// Each painted variant must mark its selection itself, or the strip would
    /// show no selection at all: nothing interpolates for them.
    #[test]
    fn painted_variants_style_their_selection() {
        use super::TabVariant;
        use iced::widget::button::Status;

        for variant in [
            TabVariant::Outline,
            TabVariant::Pill,
            TabVariant::Segmented,
            TabVariant::Tab,
        ] {
            let theme = Theme::light();
            let selected = super::tab_style(&theme, Status::Active, true, variant, Size::Md, None);
            let unselected =
                super::tab_style(&theme, Status::Active, false, variant, Size::Md, None);

            assert_ne!(
                (
                    selected.background,
                    selected.text_color,
                    selected.border.color
                ),
                (
                    unselected.background,
                    unselected.text_color,
                    unselected.border.color
                ),
                "{variant:?} must distinguish its selected tab"
            );
        }
    }

    /// Only the underline variant needs the sliding overlay. A painted variant
    /// that still requested one would draw the selection twice.
    #[test]
    fn only_underline_slides() {
        use super::TabVariant;

        assert!(TabVariant::Underline.slides());
        for variant in [
            TabVariant::Tab,
            TabVariant::Outline,
            TabVariant::Pill,
            TabVariant::Segmented,
        ] {
            assert!(!variant.slides(), "{variant:?} paints its own selection");
        }
    }

    #[test]
    fn variant_names_round_trip() {
        use super::TabVariant;

        for variant in [
            TabVariant::Underline,
            TabVariant::Tab,
            TabVariant::Outline,
            TabVariant::Pill,
            TabVariant::Segmented,
        ] {
            assert_eq!(TabVariant::from_name(variant.as_str()), variant);
        }

        assert_eq!(TabVariant::from_name("nope"), TabVariant::Underline);
        assert_eq!(TabVariant::from_name("PILL"), TabVariant::Pill);
    }

    #[test]
    fn the_default_variant_is_underline() {
        assert_eq!(super::TabVariant::default(), super::TabVariant::Underline);
    }

    #[test]
    fn a_variant_scales_its_height_with_size() {
        use super::TabVariant;

        // Larger steps are taller, in every variant.
        for variant in [
            TabVariant::Underline,
            TabVariant::Tab,
            TabVariant::Outline,
            TabVariant::Pill,
            TabVariant::Segmented,
        ] {
            let small = super::tab_height(variant, Size::Sm);
            let medium = super::tab_height(variant, Size::Md);
            let large = super::tab_height(variant, Size::Lg);

            assert!(small < medium && medium < large, "{variant:?}");
        }
    }

    #[test]
    fn tabs_take_icons_prefixes_and_suffixes() {
        use crate::icons::IconName;

        let tab = Tab::new("Files")
            .icon(IconName::File)
            .prefix("1")
            .suffix("9+");

        assert_eq!(tab.label(), "Files");
        assert!(tab.is_enabled());
        assert!(tab.icon.is_some());
        assert_eq!(tab.prefix.as_deref(), Some("1"));
        assert_eq!(tab.suffix.as_deref(), Some("9+"));
    }

    #[test]
    fn a_strip_records_its_variant_and_size() {
        use super::TabVariant;

        let strip = tabs(vec![Tab::new("One")], 0, Message::Selected)
            .variant(TabVariant::Segmented)
            .size(Size::Lg);

        assert_eq!(strip.variant, TabVariant::Segmented);
        assert_eq!(strip.size, Size::Lg);
    }

    /// A fresh strip must place the underline rather than slide it in from the
    /// left edge, which would read as the strip selecting itself on startup.
    #[test]
    fn a_fresh_indicator_is_not_yet_placed() {
        let state = super::StripState::default();

        assert!(!state.primed);
        assert_eq!(state.left.value(), 0.0);
        assert_eq!(state.width.value(), 0.0);
    }

    /// The underline must target the tab at the selected index, and its bounds
    /// must be that tab's — this is the lookup that decides where the underline
    /// is drawn.
    ///
    /// This is a regression test. The lookup once descended into the first tab
    /// before indexing, looking for the `selected`th child of a single tab
    /// rather than the `selected`th tab. Every index but zero then failed to
    /// resolve, so the underline stopped following the selection — and because
    /// the failure also stopped the springs being stepped, the underline froze
    /// where it was rather than disappearing, which is what made it look like
    /// the *first* tab was still selected.
    #[test]
    fn the_underline_targets_the_selected_tab() {
        use iced::advanced::layout;
        use iced::{Point, Rectangle, Size};

        // Three tabs of different widths, as a real strip's are: a tab is as
        // wide as its label plus its padding.
        let tabs = [
            Rectangle::new(Point::new(0.0, 0.0), Size::new(60.0, 28.0)),
            Rectangle::new(Point::new(60.0, 0.0), Size::new(80.0, 28.0)),
            Rectangle::new(Point::new(140.0, 0.0), Size::new(70.0, 28.0)),
        ];

        let children: Vec<layout::Node> = tabs
            .iter()
            .map(|tab| layout::Node::new(tab.size()).move_to(tab.position()))
            .collect();

        // The row's own node, which is what the strip's `layout` returns.
        let node = layout::Node::with_children(Size::new(210.0, 28.0), children);
        let root = layout::Layout::new(&node);

        for (index, expected) in tabs.iter().enumerate() {
            assert_eq!(
                super::selected_tab_bounds(root, index),
                Some(*expected),
                "the underline must sit under tab {index}, \
                 or it stops following the selection"
            );
        }

        assert_eq!(
            super::selected_tab_bounds(root, tabs.len()),
            None,
            "an index past the last tab selects nothing"
        );
    }

    /// A strip with no tabs, or a stale selection, has no tab to mark, so the
    /// strip reports no selection rather than panicking on the missing one.
    #[test]
    fn a_strip_with_no_selectable_tab_marks_nothing() {
        let empty: iced::Element<'_, Message, Theme> = tabs(vec![], 0, Message::Selected).into();
        drop(empty);

        let stale: iced::Element<'_, Message, Theme> =
            tabs(vec![Tab::new("Only")], 99, Message::Selected).into();
        drop(stale);
    }

    #[test]
    fn tab_labels_are_preserved() {
        let tab = Tab::new("General");
        assert_eq!(tab.label(), "General");
        assert!(tab.enabled);
    }
}
