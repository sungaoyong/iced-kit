//! The enter transition shared by the overlay surfaces.
//!
//! iced cannot fade an arbitrary `Element`: its renderer exposes clipping and a
//! linear translation and nothing else, and the `Style` a widget hands its
//! children carries only a text color. There is no opacity anywhere in the
//! graphics stack, so a surface arriving cannot fade in.
//!
//! What it can do is move. [`Enter`] slides its content into place over the
//! theme's motion tokens and drives the frames itself, so a surface gets a
//! transition without the application owning a timer.
//!
//! # Where the time comes from
//!
//! The travelled fraction is read from the `now` carried by
//! [`Event::Window(RedrawRequested)`], never from the wall clock. `draw` is
//! handed no frame time at all, so the fraction is sampled in `update` and
//! cached for `draw` to use — which also keeps a rendering reproducible: a
//! value read from `Instant::now()` would differ on every run, and a snapshot
//! could not pin it down.
//!
//! [`Event::Window(RedrawRequested)`]: iced::window::Event::RedrawRequested

use std::time::Duration;

use crate::motion::{Progress, DISTANCE_MEDIUM, DURATION_NORMAL};
use crate::theme::Theme;
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Shell};
use iced::time::Instant;
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// Which way an entering surface travels from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnterFrom {
    /// No travel: the surface is simply present.
    #[default]
    None,
    /// Travelling downwards into place, for a surface that drops below its
    /// trigger.
    Above,
    /// Travelling upwards into place, for a surface that rises above its
    /// trigger.
    Below,
    /// Travelling rightwards into place, for a surface pinned to the left edge.
    Left,
    /// Travelling leftwards into place, for a surface pinned to the right edge.
    Right,
}

impl EnterFrom {
    /// The offset the surface starts at, given how far it travels.
    fn offset(self, distance: f32) -> Vector {
        match self {
            Self::None => Vector::ZERO,
            Self::Above => Vector::new(0.0, -distance),
            Self::Below => Vector::new(0.0, distance),
            Self::Left => Vector::new(-distance, 0.0),
            Self::Right => Vector::new(distance, 0.0),
        }
    }
}

/// The transition's own state.
///
/// `travelled` is sampled in `update` and cached, because `draw` is handed no
/// frame time.
#[derive(Debug, Clone)]
struct EnterState {
    progress: Progress,
    /// How far the content has travelled, from `0.0` to `1.0`.
    travelled: f32,
    /// The frame the transition started on, which only the first redraw sets.
    started_at: Option<Instant>,
}

impl EnterState {
    /// Starts in place, so a surface rendered before any frame arrives — a
    /// single-pass snapshot, say — is drawn where it belongs.
    fn new() -> Self {
        Self {
            progress: Progress::new(true),
            travelled: 1.0,
            started_at: None,
        }
    }

    /// Begins the transition, if it has not begun already.
    ///
    /// It is deliberately started by the caller rather than by the state's
    /// construction: the state is built during `state()`, which has no frame
    /// time, and a transition needs one.
    fn begin(&mut self, now: Instant, duration: Duration) {
        if self.started_at.is_some() {
            return;
        }

        self.started_at = Some(now);
        self.progress = Progress::with_timing(false, duration, crate::motion::ease_enter);
        self.progress.set_target(true, now);
        self.travelled = 0.0;
    }
}

/// Wraps content in an enter transition.
///
/// The transition begins on the first frame the surface is drawn, so a surface
/// that appears slides in on its own. A surface already on screen when the
/// application starts does not animate: its state is created by that first
/// frame, which is the same frame the transition would begin on, and the two
/// cancel out.
///
/// For a surface that also has to animate as it leaves, the application owns a
/// [`Presence`](crate::motion::Presence) and keeps passing the content while it
/// reports that it should still be drawn; the surface reads its progress to know
/// how far through the exit it is. The exit cannot live here, because by the
/// time a surface is closed the application has already stopped building it.
pub(crate) struct Enter<'a, Message, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    from: EnterFrom,
    distance: f32,
    duration: Duration,
    /// Whether to run the transition at all. A surface whose appearance the
    /// application did not ask for — the first frame of the window, say — is
    /// placed rather than animated.
    animate: bool,
    /// The application's presence, when this surface also has to animate as it
    /// leaves. Its progress drives the travel instead of the local clock.
    presence: Option<crate::motion::Presence>,
}

impl<'a, Message, Renderer> Enter<'a, Message, Renderer> {
    /// Wraps `content` in an enter transition.
    pub(crate) fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        from: EnterFrom,
    ) -> Self {
        Self {
            content: content.into(),
            from,
            distance: DISTANCE_MEDIUM,
            duration: DURATION_NORMAL,
            animate: true,
            presence: None,
        }
    }

    /// Drives the travel from the application's presence rather than locally, so
    /// the surface slides back out as it leaves.
    ///
    /// The application keeps calling this while
    /// [`Presence::should_render`](crate::motion::Presence::should_render) is
    /// true, and stops once the exit has been drawn.
    pub(crate) fn presence(mut self, presence: &crate::motion::Presence) -> Self {
        self.presence = Some(presence.clone());
        self
    }

    /// Sets how far the surface travels, in logical pixels.
    pub(crate) fn distance(mut self, distance: f32) -> Self {
        self.distance = distance.max(0.0);
        self
    }

    /// Sets how long the transition takes.
    pub(crate) fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// The offset the content is drawn at for a given travelled fraction.
    fn offset(&self, travelled: f32) -> Vector {
        self.from.offset(self.distance * (1.0 - travelled))
    }
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for Enter<'_, Message, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<EnterState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(EnterState::new())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
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
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
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
        let state = tree.state.downcast_mut::<EnterState>();

        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            // An exit is driven by the application's presence, which knows how
            // far the surface has left; an entrance is driven here, because the
            // surface is the only thing that knows it has just appeared.
            if let Some(presence) = &self.presence {
                state.travelled = presence.progress(*now);

                if presence.is_animating(*now) {
                    shell.request_redraw();
                }
            } else if self.animate && !crate::motion::reduce_motion() {
                // The first frame is the surface's appearance, so it is also the
                // moment the transition starts. Later frames only sample it.
                state.begin(*now, self.duration);
                state.travelled = state.progress.value(*now);

                if state.progress.is_animating(*now) {
                    shell.request_redraw();
                }
            }
        }

        let offset = self.offset(state.travelled);

        // The content hit-tests where it is drawn, not where it will end up, so
        // a surface still sliding is not clickable ahead of itself.
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor - offset,
            renderer,
            clipboard,
            shell,
            &(*viewport - offset),
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
        let offset = self.offset(tree.state.downcast_ref::<EnterState>().travelled);

        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor - offset,
            &(*viewport - offset),
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
        let offset = self.offset(tree.state.downcast_ref::<EnterState>().travelled);

        if offset == Vector::ZERO {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );

            return;
        }

        renderer.with_translation(offset, |renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor - offset,
                &(*viewport - offset),
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        // An overlay drawn from inside the content — a menu, a tooltip — travels
        // with it, or it would sit at the surface's final position while the
        // surface itself is still sliding.
        let offset = self.offset(tree.state.downcast_ref::<EnterState>().travelled);

        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation + offset,
        )
    }
}

impl<'a, Message, Renderer> From<Enter<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(enter: Enter<'a, Message, Renderer>) -> Self {
        Element::new(enter)
    }
}

#[cfg(test)]
mod tests {
    use super::{Enter, EnterFrom, EnterState};
    use crate::theme::Theme;
    use std::time::Duration;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {}

    #[test]
    fn every_direction_offsets_its_content() {
        // A surface travelling from above starts above its final place, and so
        // on; the sign is what makes the motion read as arriving from the right
        // edge rather than from the wrong one.
        let above = EnterFrom::Above.offset(8.0);
        assert!(above.y < 0.0 && above.x == 0.0);

        let below = EnterFrom::Below.offset(8.0);
        assert!(below.y > 0.0 && below.x == 0.0);

        let left = EnterFrom::Left.offset(8.0);
        assert!(left.x < 0.0 && left.y == 0.0);

        let right = EnterFrom::Right.offset(8.0);
        assert!(right.x > 0.0 && right.y == 0.0);
    }

    #[test]
    fn no_direction_means_no_travel() {
        assert_eq!(EnterFrom::None.offset(8.0), iced::Vector::ZERO);
    }

    /// The offset must shrink to nothing, or a surface would come to rest
    /// displaced from where it belongs.
    #[test]
    fn a_finished_transition_has_no_offset() {
        let enter: Enter<'_, Message> = Enter::new(iced::widget::text("Body"), EnterFrom::Above);

        assert_eq!(enter.offset(0.0), iced::Vector::new(0.0, -8.0));
        assert_eq!(enter.offset(1.0), iced::Vector::ZERO);
        assert_eq!(enter.offset(0.5), iced::Vector::new(0.0, -4.0));
    }

    #[test]
    fn the_travel_distance_is_configurable() {
        let enter: Enter<'_, Message> =
            Enter::new(iced::widget::text("Body"), EnterFrom::Above).distance(20.0);

        assert_eq!(enter.offset(0.0), iced::Vector::new(0.0, -20.0));
    }

    #[test]
    fn a_surface_with_no_direction_never_moves() {
        let enter: Enter<'_, Message> = Enter::new(iced::widget::text("Body"), EnterFrom::None);

        for travelled in [0.0, 0.5, 1.0] {
            assert_eq!(enter.offset(travelled), iced::Vector::ZERO);
        }
    }

    /// A surface rendered before any frame arrives must be in place, or a
    /// single-pass snapshot would capture it mid-slide.
    #[test]
    fn a_fresh_state_is_already_in_place() {
        let state = EnterState::new();

        assert_eq!(state.travelled, 1.0);
        assert!(state.started_at.is_none());
    }

    /// Beginning a transition moves the surface to its starting offset, and
    /// beginning it twice must not restart it.
    #[test]
    fn a_transition_begins_once() {
        let mut state = EnterState::new();
        let start = std::time::Instant::now();
        let duration = Duration::from_millis(180);

        state.begin(start, duration);
        assert_eq!(state.travelled, 0.0);
        assert_eq!(state.started_at, Some(start));

        // A later frame must not rewind it.
        let later = start + Duration::from_millis(90);
        state.begin(later, duration);
        assert_eq!(
            state.started_at,
            Some(start),
            "a begun transition must not restart"
        );
    }

    #[test]
    fn every_direction_renders() {
        for from in [
            EnterFrom::None,
            EnterFrom::Above,
            EnterFrom::Below,
            EnterFrom::Left,
            EnterFrom::Right,
        ] {
            let element: iced::Element<'_, Message, Theme> =
                Enter::new(iced::widget::text("Body"), from).into();

            drop(element);
        }
    }
}
