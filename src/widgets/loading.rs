//! Collapsible panels and shimmer placeholders.
//!
//! Both animate: a collapsible grows its content open, and a shimmer sweeps a
//! highlight across whatever is waiting. The animation machinery is
//! [`crate::motion`], the same springs the rest of the library uses, so a
//! reduced-motion application gets the settled state immediately.

use crate::theme::Theme;
use iced::advanced::renderer::Renderer as _;
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, Clipboard, Shell};
use iced::time::Instant;
use iced::widget::{container, text};
use iced::{Color, Element, Length, Padding, Rectangle, Size as IcedSize, Vector};

/// A panel that shows or hides its content.
///
/// The content stays mounted while closed, so opening it has something to
/// measure — a panel built only when open would have to be laid out in the same
/// frame it appears, leaving nothing to animate.
///
/// ```
/// # use iced_kit::widgets::{collapsible, Collapsible};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view(open: bool) -> Element<'static, Message, Theme> {
/// collapsible(iced::widget::text("Hidden when closed"), open)
/// # }
/// ```
pub fn collapsible<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
    open: bool,
) -> Element<'a, Message, Theme> {
    crate::widgets::reveal::Reveal::new(content, open)
        .duration(crate::motion::DURATION_NORMAL)
        .into()
}

/// A collapsible panel under construction, with the reference's options.
///
/// [`collapsible`] covers the common case; this adds padding and a duration.
///
/// ```
/// # use iced_kit::widgets::Collapsible;
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view(open: bool) -> Element<'static, Message, Theme> {
/// Collapsible::new(iced::widget::text("Body"), open)
///     .padding(12)
///     .duration(std::time::Duration::from_millis(150))
///     .into()
/// # }
/// ```
#[must_use = "a Collapsible does nothing unless it is turned into an Element"]
pub struct Collapsible<'a, Message> {
    content: Element<'a, Message, Theme>,
    open: bool,
    padding: Padding,
    duration: std::time::Duration,
}

impl<'a, Message: Clone + 'a> Collapsible<'a, Message> {
    /// Creates a panel around `content`.
    pub fn new(content: impl Into<Element<'a, Message, Theme>>, open: bool) -> Self {
        Self {
            content: content.into(),
            open,
            padding: Padding::ZERO,
            duration: crate::motion::DURATION_NORMAL,
        }
    }

    /// Sets the padding inside the panel.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets how long the panel takes to open or close.
    pub fn duration(mut self, duration: std::time::Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Turns the panel into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        crate::widgets::reveal::Reveal::new(self.content, self.open)
            .padding(self.padding)
            .duration(self.duration)
            .into()
    }
}

impl<'a, Message: Clone + 'a> From<Collapsible<'a, Message>> for Element<'a, Message, Theme> {
    fn from(collapsible: Collapsible<'a, Message>) -> Self {
        collapsible.into_element()
    }
}

/// How long one pulse of a breathing wash takes.
const BREATHE_PERIOD: std::time::Duration = std::time::Duration::from_millis(1600);

/// The weakest a breathing wash gets. Not zero: the wash must stay visible or
/// the content would appear to flicker in and out.
const BREATHE_MIN: f32 = 0.0;

/// The strongest a breathing wash gets. Kept low so the content stays legible
/// under it.
const BREATHE_MAX: f32 = 0.35;

/// How far a shimmer's highlight spreads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShimmerSpread {
    /// A share of the element's own width.
    Relative(f32),
    /// A width in logical pixels.
    Absolute(f32),
}

impl Default for ShimmerSpread {
    fn default() -> Self {
        Self::Relative(0.6)
    }
}

impl ShimmerSpread {
    /// The highlight's width for an element `width` wide.
    #[must_use]
    pub fn width(self, width: f32) -> f32 {
        match self {
            Self::Relative(share) => width * share.clamp(0.05, 4.0),
            Self::Absolute(pixels) => pixels.max(1.0),
        }
    }
}

/// The settings shared by every shimmer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShimmerStyle {
    /// How long one sweep takes.
    pub duration: std::time::Duration,
    /// The color of the moving highlight.
    pub highlight: Option<Color>,
    /// How wide the highlight is.
    pub spread: ShimmerSpread,
    /// Whether the sweep runs right to left.
    pub reverse: bool,
}

impl Default for ShimmerStyle {
    fn default() -> Self {
        Self {
            duration: std::time::Duration::from_millis(1600),
            highlight: None,
            spread: ShimmerSpread::default(),
            reverse: false,
        }
    }
}

impl ShimmerStyle {
    /// Creates the default style.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets how long one sweep takes.
    #[must_use]
    pub fn duration(mut self, duration: std::time::Duration) -> Self {
        self.duration = duration.max(std::time::Duration::from_millis(1));
        self
    }

    /// Sets the highlight color. The default is the surface color, which reads
    /// as a light sweeping over the muted surface.
    #[must_use]
    pub fn highlight(mut self, color: Color) -> Self {
        self.highlight = Some(color);
        self
    }

    /// Sets how wide the highlight is.
    #[must_use]
    pub fn spread(mut self, spread: impl Into<ShimmerSpread>) -> Self {
        self.spread = spread.into();
        self
    }

    /// Runs the sweep right to left.
    #[must_use]
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Where the highlight's leading edge sits at `elapsed` into a sweep, as a
    /// fraction of the position it must travel.
    ///
    /// The highlight starts fully off one edge and ends fully off the other, so
    /// a sweep has no pause at either end.
    #[must_use]
    pub fn progress(&self, elapsed: std::time::Duration) -> f32 {
        let period = self.duration.as_secs_f32().max(0.001);
        let phase = (elapsed.as_secs_f32() % period) / period;

        if self.reverse {
            1.0 - phase
        } else {
            phase
        }
    }
}

impl From<f32> for ShimmerSpread {
    fn from(value: f32) -> Self {
        Self::Relative(value)
    }
}

/// A block of text that shimmers while it waits to be replaced by real content.
///
/// ```
/// # use iced_kit::widgets::{shimmer_text, ShimmerStyle};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// shimmer_text("Loading…")
/// # }
/// ```
pub fn shimmer_text<'a, Message: 'a>(
    content: impl text::IntoFragment<'a>,
) -> Element<'a, Message, Theme> {
    let element: Element<'a, Message, Theme> = text(content).into();

    Shimmer::new(element).into()
}

/// Wraps an element in a shimmer.
pub fn shimmer<'a, Message: 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
) -> Shimmer<'a, Message> {
    Shimmer::new(content)
}

/// A shimmer over any element.
///
/// ```
/// # use iced_kit::widgets::{shimmer, ShimmerStyle};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// shimmer(iced::widget::text("Loading…")).into()
/// # }
/// ```
#[must_use = "a Shimmer does nothing unless it is turned into an Element"]
pub struct Shimmer<'a, Message> {
    content: Element<'a, Message, Theme>,
    style: ShimmerStyle,
}

impl<'a, Message: 'a> Shimmer<'a, Message> {
    /// Wraps `content` in a shimmer.
    pub fn new(content: impl Into<Element<'a, Message, Theme>>) -> Self {
        Self {
            content: content.into(),
            style: ShimmerStyle::default(),
        }
    }

    /// Sets the shimmer's style.
    pub fn style(mut self, style: ShimmerStyle) -> Self {
        self.style = style;
        self
    }

    /// Turns the shimmer into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        ShimmerShell {
            content: self.content,
            style: self.style,
        }
        .into()
    }
}

impl<'a, Message: 'a> From<Shimmer<'a, Message>> for Element<'a, Message, Theme> {
    fn from(shimmer: Shimmer<'a, Message>) -> Self {
        shimmer.into_element()
    }
}

/// The shimmer's state between frames.
#[derive(Debug, Clone, Copy)]
struct ShimmerState {
    /// The frame the sweep was last advanced to.
    last: Option<Instant>,
    /// When the current sweep began.
    started: Option<Instant>,
    /// The elapsed time within the current sweep, for the draw.
    elapsed: std::time::Duration,
}

impl Default for ShimmerState {
    fn default() -> Self {
        Self {
            last: None,
            started: None,
            elapsed: std::time::Duration::ZERO,
        }
    }
}

/// The widget that draws a shimmer over its child.
struct ShimmerShell<'a, Message> {
    content: Element<'a, Message, Theme>,
    style: ShimmerStyle,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for ShimmerShell<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ShimmerState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ShimmerState::default())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> IcedSize<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<ShimmerState>();

            // A reduced-motion application keeps the sweep still rather than
            // asking for a frame every tick.
            if crate::motion::reduce_motion() {
                state.last = Some(*now);
                return;
            }

            let started = *state.started.get_or_insert(*now);
            state.elapsed = now.duration_since(started);
            state.last = Some(*now);

            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !bounds.intersects(viewport) {
            return;
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );

        // The highlight is painted over the content rather than masked into it:
        // iced has no alpha, so the sweep is a translucent band of the surface
        // color, which reads as a light moving across the muted placeholder.
        if crate::motion::reduce_motion() {
            return;
        }

        let state = tree.state.downcast_ref::<ShimmerState>();
        let width = bounds.width;
        if width <= 0.0 {
            return;
        }

        let highlight_width = self.style.spread.width(width);
        let travel = width + highlight_width;
        let progress = self.style.progress(state.elapsed);
        // The band enters off one edge and leaves off the other.
        let x = bounds.x - highlight_width + travel * progress;

        let color = self
            .style
            .highlight
            .unwrap_or_else(|| theme.colors().surface);

        renderer.fill_quad(
            iced::advanced::renderer::Quad {
                bounds: Rectangle {
                    x,
                    y: bounds.y,
                    width: highlight_width,
                    height: bounds.height,
                },
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                shadow: iced::Shadow::default(),
                snap: false,
            },
            Color { a: 0.55, ..color },
        );
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<ShimmerShell<'a, Message>> for Element<'a, Message, Theme> {
    fn from(shimmer: ShimmerShell<'a, Message>) -> Self {
        Element::new(shimmer)
    }
}

/// Pulses its content, for something waiting that has nothing to shimmer.
///
/// iced's renderer has no alpha for a subtree, so this cannot dim what it
/// wraps. What it does instead is draw a soft, muted wash over the content and
/// pulse that: the content stays legible, and the movement is what says work is
/// still in progress. A caller with text to sweep should use [`shimmer`]
/// instead, which is the more informative effect.
pub fn breathe<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message, Theme>>,
) -> Element<'a, Message, Theme> {
    Breathing {
        content: content.into(),
    }
    .into()
}

/// The widget that pulses a wash over its content.
struct Breathing<'a, Message> {
    content: Element<'a, Message, Theme>,
}

/// The phase a breathing wash is at, and when it was last advanced.
#[derive(Debug, Clone, Copy, Default)]
struct BreathingState {
    /// Where in the cycle the wash is, in radians.
    phase: f32,
    /// The frame the phase was last advanced to.
    last: Option<Instant>,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Breathing<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<BreathingState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(BreathingState::default())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> IcedSize<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        // A reduced-motion application shows the settling state and stops
        // asking for frames, so the wash simply stays where it is.
        if crate::motion::reduce_motion() {
            return;
        }

        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<BreathingState>();
            let elapsed = state
                .last
                .map_or(std::time::Duration::ZERO, |last| now.duration_since(last));
            state.last = Some(*now);

            let period = BREATHE_PERIOD.as_secs_f32().max(0.001);
            state.phase = (state.phase + elapsed.as_secs_f32() / period * std::f32::consts::TAU)
                .rem_euclid(std::f32::consts::TAU);

            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );

        if !bounds.intersects(viewport) || crate::motion::reduce_motion() {
            return;
        }

        // A sine keeps the wash's strength continuous, so the pulse has no
        // seam where the cycle restarts.
        let phase = tree.state.downcast_ref::<BreathingState>().phase;
        let strength = (phase.sin() * 0.5 + 0.5) * (BREATHE_MAX - BREATHE_MIN) + BREATHE_MIN;

        renderer.fill_quad(
            iced::advanced::renderer::Quad {
                bounds,
                border: iced::Border {
                    radius: 0.0.into(),
                    ..iced::Border::default()
                },
                shadow: iced::Shadow::default(),
                snap: true,
            },
            Color {
                a: strength,
                ..theme.colors().surface
            },
        );
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message: 'a> From<Breathing<'a, Message>> for Element<'a, Message, Theme> {
    fn from(breathing: Breathing<'a, Message>) -> Self {
        Element::new(breathing)
    }
}

/// A muted block with a shimmer, for a value that has not arrived.
///
/// This is a shimmer over a skeleton: the skeleton gives the placeholder its
/// shape, and the shimmer is what says it is still loading.
pub fn shimmer_block<'a, Message: Clone + 'a>(
    width: Length,
    height: f32,
) -> Element<'a, Message, Theme> {
    let block: Element<'a, Message, Theme> = container(iced::widget::Space::new())
        .width(width)
        .height(Length::Fixed(height))
        .class(Box::new(|theme: &Theme| container::Style {
            background: Some(iced::Background::Color(theme.colors().muted)),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: f32::from(theme.radius().sm).into(),
            },
            ..container::Style::default()
        }) as container::StyleFn<'a, Theme>)
        .into();

    Shimmer::new(block).into_element()
}

#[cfg(test)]
mod tests {
    use super::{
        collapsible, shimmer, shimmer_block, shimmer_text, Collapsible, Shimmer, ShimmerSpread,
        ShimmerStyle,
    };
    use crate::theme::Theme;
    use iced::Length;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    #[test]
    fn a_collapsible_records_its_options() {
        let panel: Collapsible<'_, Message> = Collapsible::new(iced::widget::text("Body"), true)
            .padding(12)
            .duration(std::time::Duration::from_millis(150));

        assert!(panel.open);
        assert_eq!(panel.padding.top, 12.0);
        assert_eq!(panel.duration.as_millis(), 150);
    }

    #[test]
    fn collapsibles_render_open_and_closed() {
        let open: iced::Element<'_, Message, Theme> = collapsible(iced::widget::text("Body"), true);
        drop(open);

        let closed: iced::Element<'_, Message, Theme> =
            collapsible(iced::widget::text("Body"), false);
        drop(closed);

        let padded: iced::Element<'_, Message, Theme> =
            Collapsible::new(iced::widget::text("Body"), true)
                .padding(8)
                .into();
        drop(padded);
    }

    #[test]
    fn a_shimmer_style_records_its_settings() {
        let style = ShimmerStyle::new()
            .duration(std::time::Duration::from_millis(800))
            .highlight(iced::Color::WHITE)
            .spread(ShimmerSpread::Absolute(40.0))
            .reverse(true);

        assert_eq!(style.duration.as_millis(), 800);
        assert!(style.highlight.is_some());
        assert_eq!(style.spread, ShimmerSpread::Absolute(40.0));
        assert!(style.reverse);

        // A zero duration would divide by zero in `progress`, so it is floored.
        assert!(
            ShimmerStyle::new()
                .duration(std::time::Duration::ZERO)
                .duration
                .as_millis()
                >= 1
        );
    }

    #[test]
    fn a_spread_measures_itself_against_the_element() {
        assert_eq!(ShimmerSpread::Relative(0.5).width(200.0), 100.0);
        assert_eq!(ShimmerSpread::Absolute(40.0).width(200.0), 40.0);

        // A share outside the sensible range is clamped: a highlight that
        // covered nothing, or the whole element, would not read as a sweep.
        assert_eq!(ShimmerSpread::Relative(0.0).width(200.0), 10.0);
        assert_eq!(ShimmerSpread::Relative(9.0).width(200.0), 800.0);

        assert_eq!(ShimmerSpread::default(), ShimmerSpread::Relative(0.6));
        assert_eq!(ShimmerSpread::from(0.3), ShimmerSpread::Relative(0.3));
    }

    #[test]
    fn a_sweep_runs_from_one_edge_to_the_other() {
        let style = ShimmerStyle::new().duration(std::time::Duration::from_millis(1000));

        assert_eq!(style.progress(std::time::Duration::ZERO), 0.0);
        assert_eq!(style.progress(std::time::Duration::from_millis(500)), 0.5);
        // The sweep wraps rather than stopping at the end.
        assert_eq!(style.progress(std::time::Duration::from_millis(1000)), 0.0);
        assert!((style.progress(std::time::Duration::from_millis(1500)) - 0.5).abs() < 0.001);
    }

    #[test]
    fn a_reversed_sweep_runs_the_other_way() {
        let style = ShimmerStyle::new()
            .duration(std::time::Duration::from_millis(1000))
            .reverse(true);

        // A reversed sweep starts at the far edge. At the end of the period the
        // phase wraps to zero, so the reversed progress is back at the far edge
        // rather than at the near one: the wrap is the start of the next sweep.
        assert_eq!(style.progress(std::time::Duration::ZERO), 1.0);
        assert_eq!(style.progress(std::time::Duration::from_millis(500)), 0.5);
        assert_eq!(style.progress(std::time::Duration::from_millis(1000)), 1.0);
        // Just before the wrap it has reached the near edge.
        let almost = style.progress(std::time::Duration::from_millis(999));
        assert!(
            almost < 0.01,
            "a reversed sweep ends at the near edge: {almost}"
        );
    }

    #[test]
    fn the_default_style_has_every_field_set() {
        let style = ShimmerStyle::default();

        assert_eq!(style.duration, std::time::Duration::from_millis(1600));
        assert_eq!(style.highlight, None);
        assert!(!style.reverse);
    }

    #[test]
    fn shimmers_render_in_every_form() {
        let elements: Vec<iced::Element<'_, Message, Theme>> = vec![
            shimmer_text::<Message>("Loading…"),
            shimmer(iced::widget::text("Loading…")).into(),
            Shimmer::new(iced::widget::text("Loading…"))
                .style(ShimmerStyle::new().reverse(true).spread(0.3))
                .into(),
            shimmer_block::<Message>(Length::Fixed(100.0), 16.0),
        ];

        for element in elements {
            drop(element);
        }
    }
}
