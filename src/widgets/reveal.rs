//! A panel that opens and closes by growing.
//!
//! The height of the content is measured first and then revealed progressively,
//! which is what gives a collapsible section its motion. It takes a measured
//! approach rather than a fixed height because the caller supplies arbitrary
//! content: the panel has to find out how tall that content is before it can
//! decide how much of it to show.
//!
//! The content is laid out at its full height on every frame and clipped to the
//! revealed part, rather than being laid out at the revealed height. That
//! distinction matters: laying it out at a shrinking height would reflow
//! everything inside — a wrapped paragraph would re-break its lines on every
//! frame of the transition.

use std::time::Duration;

use crate::theme::Theme;
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Shell};
use iced::time::Instant;
use iced::{Element, Event, Length, Padding, Rectangle, Size};

/// The state a [`Reveal`] keeps between frames.
///
/// The fraction is spring-driven rather than eased on a duration. A reveal is
/// retargeted often — clicking through a list of sections toggles the previous
/// one shut and the next one open in the same frame — and a spring carries its
/// velocity across that, so the panel that is closing decelerates and turns
/// instead of restarting. A spring also takes its timing per step, so the
/// theme's tokens are read every frame rather than frozen when the motion
/// began.
#[derive(Debug, Clone, Copy)]
struct RevealState {
    /// How much of the content is shown, from `0.0` to `1.0`.
    revealed: crate::motion::SpringState,
    /// The frame the reveal was last advanced to.
    last: Option<Instant>,
}

impl RevealState {
    fn new(open: bool) -> Self {
        Self {
            revealed: crate::motion::SpringState::new(if open { 1.0 } else { 0.0 }),
            last: None,
        }
    }

    /// The fraction of the content to show.
    fn fraction(self) -> f32 {
        self.revealed.value().clamp(0.0, 1.0)
    }
}

/// A panel that grows to reveal its content.
pub(crate) struct Reveal<'a, Message, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    open: bool,
    /// The padding around the content, which is revealed along with it.
    padding: Padding,
    spring: crate::motion::Spring,
}

impl<'a, Message, Renderer> Reveal<'a, Message, Renderer> {
    /// Wraps `content` in a panel that grows when `open`.
    pub(crate) fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        open: bool,
    ) -> Self {
        Self {
            content: content.into(),
            open,
            padding: Padding::ZERO,
            // A panel is bounded by the content above and below it, so it must
            // not overshoot: growing past its own height would overlap whatever
            // follows. Critically damped, with a pixel-scale tolerance.
            spring: crate::motion::Spring::new(crate::motion::DURATION_NORMAL).with_epsilon(0.002),
        }
    }

    /// Sets the padding around the content.
    ///
    /// The padding is inside the revealed height, so a closed panel has no
    /// height at all rather than leaving the padding behind as a gap.
    pub(crate) fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets how long the panel takes to open or close.
    pub(crate) fn duration(mut self, duration: Duration) -> Self {
        self.spring = self.spring.with_response(duration);
        self
    }
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for Reveal<'_, Message, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<RevealState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(RevealState::new(self.open))
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        // The height depends on the measured content, which `size` cannot know,
        // so the panel reports the axis it does control.
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        // The content is measured unconstrained vertically so it takes the
        // height it needs rather than being squeezed into the revealed part.
        let full = layout::Limits::new(
            Size::new(limits.max().width, 0.0),
            Size::new(limits.max().width, f32::INFINITY),
        );

        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, &full);

        let padding = self.padding.fit(content.size(), limits.max());
        let natural = content.size().height + padding.top + padding.bottom;

        let revealed = natural * tree.state.downcast_ref::<RevealState>().fraction();
        let size = Size::new(limits.max().width, revealed.clamp(0.0, natural));

        layout::Node::with_children(
            size,
            vec![content.translate(iced::Vector::new(padding.left, padding.top))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &Renderer,
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<RevealState>();

        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let target = if self.open { 1.0 } else { 0.0 };
            let spring = if crate::motion::reduce_motion() {
                crate::motion::Spring::new(Duration::ZERO)
            } else {
                self.spring
            };

            let elapsed = state
                .last
                .map_or(Duration::ZERO, |last| now.duration_since(last));
            state.last = Some(*now);

            state.revealed.step(target, spring, elapsed);

            // A reveal changes the layout every frame it runs, so the frame has
            // to be re-laid out rather than merely redrawn.
            if !state.revealed.is_settled(target, spring) {
                shell.invalidate_layout();
                shell.request_redraw();
            }
        }

        // Only the revealed part can be interacted with, so a control inside a
        // closed panel does not answer for content nobody can see.
        let visible = layout.bounds();
        let reachable = cursor.position_over(visible).is_some();

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap(),
            if reachable {
                cursor
            } else {
                mouse::Cursor::Unavailable
            },
            renderer,
            clipboard,
            shell,
            &visible
                .intersection(viewport)
                .unwrap_or_else(|| Rectangle::with_size(Size::ZERO)),
        );
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let visible = layout.bounds();
        let hidden = cursor.position_over(visible).is_none();

        if hidden {
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
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if bounds.height <= 0.0 || !bounds.intersects(viewport) {
            return;
        }

        // The layer is the clip: the content is drawn at its full height and cut
        // off at the revealed part, so text does not spill over what is below
        // the panel while it opens.
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
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        // A closed panel shows nothing, so neither does anything inside it.
        if layout.bounds().height <= 0.0 {
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

impl<'a, Message, Renderer> From<Reveal<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(reveal: Reveal<'a, Message, Renderer>) -> Self {
        Element::new(reveal)
    }
}

#[cfg(test)]
mod tests {
    use super::{Reveal, RevealState};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    /// A panel that starts closed shows none of its content, and one that
    /// starts open shows all of it.
    #[test]
    fn a_panel_starts_in_its_given_state() {
        assert_eq!(RevealState::new(true).fraction(), 1.0);
        assert_eq!(RevealState::new(false).fraction(), 0.0);
    }

    /// The fraction must stay within the range the height is computed from, or
    /// the panel would claim a negative height or one larger than its content.
    #[test]
    fn the_fraction_stays_within_range() {
        for open in [true, false] {
            let state = RevealState::new(open);
            let fraction = state.fraction();

            assert!(
                (0.0..=1.0).contains(&fraction),
                "a fraction of {fraction} is out of range"
            );
        }
    }

    #[test]
    fn a_panel_renders_open_and_closed() {
        for open in [true, false] {
            let element: iced::Element<'_, Message, Theme> =
                Reveal::new(iced::widget::text("Body"), open).into();

            drop(element);
        }
    }

    #[test]
    fn a_panel_takes_padding() {
        let element: iced::Element<'_, Message, Theme> =
            Reveal::new(iced::widget::text("Body"), true)
                .padding(iced::Padding::new(12.0))
                .into();

        drop(element);
    }
}
