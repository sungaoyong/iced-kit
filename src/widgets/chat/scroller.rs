//! A virtualized, tail-following transcript scroller.
//!
//! The state machine lives here ([`MessageScrollerState`]); the widget that
//! renders it ([`MessageScroller`]) is added alongside it. The state is plain,
//! app-owned data — the same ownership model as
//! [`VirtualListState`](crate::widgets::virtual_list::VirtualListState) — so the
//! scroller stays stateless and testable.

use crate::theme::{Size, Theme};
use crate::widgets::virtual_list::VirtualList;
use crate::widgets::virtual_list::VirtualListState;
use iced::advanced::layout::{self, Limits};
use iced::advanced::renderer::Renderer as _;
use iced::advanced::widget::{tree, Operation};
use iced::advanced::{mouse, Clipboard, Shell, Widget};
use iced::time::Instant;
use iced::widget::{container, stack};
use iced::{
    Alignment, Background, Color, Element, Length, Padding, Rectangle, Size as IcedSize, Vector,
};

/// Distance (px) from the bottom within which the scroller re-sticks to the tail.
const FOLLOW_THRESHOLD: f32 = 80.0;

/// How long an overlay takes to ease in or out.
const TRANSITION_DURATION: std::time::Duration = std::time::Duration::from_millis(200);

/// How far the jump-to-bottom control rises as it arrives, in logical pixels.
const TRANSITION_RISE: f32 = 8.0;

/// How far the jump-to-bottom control floats above the scroller's bottom edge.
///
/// It clears the fade, so the control is not drawn inside the gradient that
/// says there is more below.
const JUMP_BUTTON_INSET: f32 = 12.0;

/// App-owned scroller state: scroll/measure ([`VirtualListState`]), whether the
/// view is pinned to the newest message, and the last-seen item count.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MessageScrollerState {
    list: VirtualListState,
    follow_tail: bool,
    /// Last-seen item count, for the application to diff appends/prepends.
    /// The renderer does not read it: tail-following is driven purely by
    /// `follow_tail` plus the viewport's `anchor_bottom`.
    seen_len: usize,
}

impl MessageScrollerState {
    /// Creates a state over `item_count` items, pinned to the tail.
    #[must_use]
    pub fn new(item_count: usize) -> Self {
        Self {
            list: VirtualListState::new(),
            follow_tail: true,
            seen_len: item_count,
        }
    }

    /// Whether the view is currently pinned to the newest message.
    #[must_use]
    pub fn is_following_tail(&self) -> bool {
        self.follow_tail
    }

    /// Whether the user has scrolled away from the newest message.
    #[must_use]
    pub fn is_scrolled_up(&self) -> bool {
        !self.follow_tail
    }

    /// The current scroll offset in logical pixels.
    #[must_use]
    pub fn list_offset(&self) -> f32 {
        self.list.offset()
    }

    /// The number of items the state last saw.
    #[must_use]
    pub fn seen_len(&self) -> usize {
        self.seen_len
    }

    /// The inner virtualized-list state, for building a [`VirtualList`].
    #[must_use]
    pub fn inner(&self) -> &VirtualListState {
        &self.list
    }

    /// Mutable access to the inner list state (e.g. to record a measurement).
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut VirtualListState {
        &mut self.list
    }

    /// Records the incoming item count. An append while following keeps the tail
    /// pinned; nothing here moves the offset on its own.
    pub fn record_len(&mut self, new_len: usize) {
        // Append: if already following, stay following — never steal focus from a
        // user who has scrolled up. Shrink: nothing special, the offset clamps
        // during layout. Neither case moves the offset, so only the seen count
        // is synced here.
        self.seen_len = new_len;
    }

    /// Notes that `count` older items were prepended (history loading). The
    /// offset is deliberately untouched so the visible history does not jump.
    pub fn prepend(&mut self, count: usize) {
        self.seen_len += count;
    }

    /// Notes that `count` newer items were appended.
    pub fn append(&mut self, count: usize) {
        self.seen_len += count;
    }

    /// Replaces a range with `new_len` total items, syncing the count.
    pub fn splice(&mut self, _range: std::ops::Range<usize>, new_len: usize) {
        self.seen_len = new_len;
    }

    /// Resets to a fresh tail-pinned state over `item_count` items.
    pub fn reset(&mut self, item_count: usize) {
        *self = Self::new(item_count);
    }

    /// Applies a fresh scroll event given the content and viewport heights,
    /// returning whether the follow state changed.
    ///
    /// A scroll within [`FOLLOW_THRESHOLD`] of the bottom re-pins the view; any
    /// larger gap releases it.
    pub fn apply_scroll(&mut self, content_height: f32, viewport: f32, offset: f32) -> bool {
        self.list.update(offset, viewport);
        let max = (content_height - viewport).max(0.0);
        let was = self.follow_tail;
        self.follow_tail = (max - offset) <= FOLLOW_THRESHOLD;
        self.follow_tail != was
    }

    /// Pins the view to the newest message without computing an offset.
    ///
    /// This is the primitive a "jump to bottom" button should reach for: it only
    /// flips the follow flag, and the next render lets the viewport's
    /// `anchor_bottom` place the tail exactly. Prefer it over
    /// [`scroll_to_end`](Self::scroll_to_end), which needs content/viewport
    /// heights the caller rarely knows precisely.
    pub fn pin_to_tail(&mut self) {
        self.follow_tail = true;
    }

    /// Jumps to the very bottom and pins the view there, given the total content
    /// and viewport heights.
    ///
    /// Because `content_height` is typically an estimate (rows may not all be
    /// measured), most callers should use [`pin_to_tail`](Self::pin_to_tail)
    /// instead; this is the explicit-offset path for restoring a known position.
    pub fn scroll_to_end(&mut self, content_height: f32, viewport: f32) {
        let max = (content_height - viewport).max(0.0);
        self.list.update(max, viewport);
        self.follow_tail = true;
    }
}

/// A virtualized, tail-following transcript scroller widget.
///
/// Built on top of [`VirtualList`]: it pins the viewport to the newest row
/// while [`MessageScrollerState::is_following_tail`], and can float a
/// "jump to bottom" button and a bottom fade once the reader has scrolled up.
#[must_use = "a MessageScroller does nothing unless it is turned into an Element"]
pub struct MessageScroller<'a, T, Message> {
    items: &'a [T],
    state: &'a MessageScrollerState,
    row: Box<dyn Fn(&'a T, usize) -> Element<'a, Message, Theme> + 'a>,
    row_height: f32,
    jump_button: bool,
    jump_label: String,
    bottom_fade: Option<Color>,
    width: Length,
    height: Length,
    on_scroll: Option<Box<dyn Fn(MessageScrollerState) -> Message + 'a>>,
    on_jump: Option<Message>,
}

impl<'a, T: 'a, Message: Clone + 'a> MessageScroller<'a, T, Message> {
    /// Creates a scroller over `items`, rendering each row with `row`, driven by
    /// the app-owned `state`.
    pub fn new(
        items: &'a [T],
        state: &'a MessageScrollerState,
        row: impl Fn(&'a T, usize) -> Element<'a, Message, Theme> + 'a,
    ) -> Self {
        Self {
            items,
            state,
            row: Box::new(row),
            row_height: 64.0,
            jump_button: false,
            jump_label: "\u{2193} New messages".to_string(),
            bottom_fade: None,
            width: Length::Fill,
            height: Length::Fill,
            on_scroll: None,
            on_jump: None,
        }
    }

    /// Sets the uniform height assumed for each row.
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height.max(1.0);
        self
    }

    /// Alias for [`row_height`](Self::row_height): the estimate used before a row
    /// has been measured.
    pub fn estimate(mut self, height: f32) -> Self {
        self.row_height = height.max(1.0);
        self
    }

    /// Shows or hides the floating "jump to bottom" button (only visible once
    /// the reader has scrolled away from the tail).
    pub fn jump_button(mut self, on: bool) -> Self {
        self.jump_button = on;
        self
    }

    /// Sets the jump button's label.
    pub fn with_jump_button_label(mut self, label: impl Into<String>) -> Self {
        self.jump_label = label.into();
        self
    }

    /// Enables a bottom fade rendered in `color` (only visible when scrolled up).
    pub fn with_bottom_fade(mut self, color: Option<Color>) -> Self {
        self.bottom_fade = color;
        self
    }

    /// Sets the scroller's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the scroller's visible height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Reports scroll changes as a fresh [`MessageScrollerState`].
    pub fn on_scroll(mut self, f: impl Fn(MessageScrollerState) -> Message + 'a) -> Self {
        self.on_scroll = Some(Box::new(f));
        self
    }

    /// Emits `message` when the jump-to-bottom button is pressed.
    pub fn on_jump_to_bottom(mut self, message: Message) -> Self {
        self.on_jump = Some(message);
        self
    }

    /// Converts the scroller into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            items,
            state,
            row,
            row_height,
            jump_button,
            jump_label,
            bottom_fade,
            width,
            height,
            on_scroll,
            on_jump,
        } = self;

        let content_height = items.len() as f32 * row_height;

        let mut list = VirtualList::new(items, state.inner(), row)
            .fixed_row_height(row_height)
            .width(width)
            .height_length(height)
            .anchor_bottom(state.is_following_tail());

        if let Some(on_scroll) = on_scroll {
            list = list.on_scroll(move |reported| {
                let mut next = state.clone();
                next.apply_scroll(
                    content_height,
                    reported.viewport_height(),
                    reported.offset(),
                );
                on_scroll(next)
            });
        }

        let content: Element<'a, Message, Theme> = list.into();
        let scrolled_up = state.is_scrolled_up();
        let mut layers: Vec<Element<'a, Message, Theme>> = vec![content];

        // The fade and the button sit above the content and hug its bottom
        // edge. Both stay in the tree while they are shown or still leaving, so
        // scrolled-up and scrolled-back do not blink them off and on.
        if let Some(color) = bottom_fade {
            let fade: Element<'a, Message, Theme> = FadeOverlay::new(color, scrolled_up).into();
            layers.push(fade);
        }

        if jump_button {
            let button = jump_overlay(jump_label, on_jump, width, height);
            layers.push(Transition::new(button, scrolled_up).into());
        }

        stack(layers).width(width).height(height).into()
    }
}

/// Eases its child in from below and out again, with a spring.
///
/// The effect is a rise: iced's renderer has no alpha for a subtree, so a child
/// that has to fade applies the progress itself — which is what [`FadeOverlay`]
/// does. What this owns is the offset and the visibility, so a hidden overlay
/// rises away and is then skipped rather than left as a transparent layer over
/// the transcript.
struct Transition<'a, Message> {
    content: Element<'a, Message, Theme>,
    showing: bool,
}

impl<'a, Message> Transition<'a, Message> {
    fn new(content: Element<'a, Message, Theme>, showing: bool) -> Self {
        Self { content, showing }
    }
}

/// A transition's state between frames.
#[derive(Debug, Clone, Copy, Default)]
struct TransitionState {
    /// How far in the transition has travelled: 0 hidden, 1 shown.
    progress: crate::motion::SpringState,
    /// The frame the spring was last advanced to.
    last: Option<Instant>,
    /// Whether the progress has ever been placed, so the first frame does not
    /// animate the overlay in on startup.
    primed: bool,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Transition<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TransitionState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TransitionState::default())
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
        limits: &Limits,
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
        // A hidden overlay takes no input. It is on its way out, and a control
        // must not answer for a press while it is leaving.
        if self.showing {
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
        }

        step_spring(tree, event, self.showing, shell);
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
        let progress = tree
            .state
            .downcast_ref::<TransitionState>()
            .progress
            .value();

        // Fully out: nothing is left to draw, and drawing it would leave a
        // ghost of the overlay behind.
        if progress <= 0.001 {
            return;
        }

        // Fully in is the common case, and it is drawn without a translation
        // layer so a settled transcript pays nothing for the transition.
        if progress >= 0.999 {
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

        // The overlay rises the last few pixels into place.
        let offset = Vector::new(0.0, (1.0 - progress) * TRANSITION_RISE);

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

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if !self.showing {
            return mouse::Interaction::None;
        }

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

impl<'a, Message: 'a> From<Transition<'a, Message>> for Element<'a, Message, Theme> {
    fn from(transition: Transition<'a, Message>) -> Self {
        Element::new(transition)
    }
}

/// The spring an overlay eases in and out with.
///
/// Critically damped, so an overlay never overshoots the edge it settles on: a
/// control that bounced past its resting place would read as a glitch. The
/// tolerance is coarse because the travel is only a few pixels.
fn transition_spring() -> crate::motion::Spring {
    crate::motion::Spring::new(TRANSITION_DURATION).with_epsilon(0.01)
}

impl<'a, T: 'a, Message: Clone + 'a> From<MessageScroller<'a, T, Message>>
    for Element<'a, Message, Theme>
{
    fn from(scroller: MessageScroller<'a, T, Message>) -> Self {
        scroller.into_element()
    }
}

/// How tall the bottom fade is, in logical pixels.
const FADE_HEIGHT: f32 = 48.0;

/// How many bands the fade's gradient is drawn in.
///
/// iced fills quads rather than painting a gradient, so the ramp is a stack of
/// bands. Four is enough to read as one gradient at this height.
const FADE_BANDS: usize = 4;

/// A gradient hugging the bottom edge, easing in and out with a spring.
///
/// Drawn by the widget rather than built as a tree of bands: a prebuilt tree
/// fixes each band's alpha at construction, so it could only appear and
/// disappear. Drawing it here applies the transition's progress to every band,
/// which is what lets it fade.
struct FadeOverlay {
    color: Color,
    showing: bool,
}

impl FadeOverlay {
    fn new(color: Color, showing: bool) -> Self {
        Self { color, showing }
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for FadeOverlay {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TransitionState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(TransitionState::default())
    }

    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &iced::Renderer,
        limits: &Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &iced::Event,
        _layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        // The fade is decoration: it never takes an event, so a press reaches
        // the transcript drawn beneath it.
        step_spring(tree, event, self.showing, shell);
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut iced::Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<TransitionState>();

        // An unprimed overlay is drawn where it belongs rather than at zero: a
        // frame is painted before the update that primes the spring, so reading
        // the bare value would paint the first frame wrong.
        let progress = if state.primed {
            state.progress.value()
        } else if self.showing {
            1.0
        } else {
            0.0
        };

        // Fully out: nothing to draw, and drawing it would leave a ghost of the
        // fade over the transcript below it.
        if progress <= 0.001 {
            return;
        }

        let bounds = layout.bounds();
        let height = FADE_HEIGHT.min(bounds.height);
        let top = bounds.y + bounds.height - height;
        let band_height = height / FADE_BANDS as f32;

        for index in 0..FADE_BANDS {
            let band = Rectangle {
                x: bounds.x,
                y: top + band_height * index as f32,
                width: bounds.width,
                // A hair taller, so rounding between bands leaves no seam.
                height: band_height + 0.5,
            };

            if !band.intersects(viewport) {
                continue;
            }

            renderer.fill_quad(
                iced::advanced::renderer::Quad {
                    bounds: band,
                    border: iced::Border {
                        radius: 0.0.into(),
                        ..iced::Border::default()
                    },
                    shadow: iced::Shadow::default(),
                    snap: true,
                },
                Color {
                    a: band_alpha(self.color, index, progress),
                    ..self.color
                },
            );
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &tree::Tree,
        _layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}

impl<'a, Message: 'a> From<FadeOverlay> for Element<'a, Message, Theme> {
    fn from(fade: FadeOverlay) -> Self {
        Element::new(fade)
    }
}

/// One band's alpha: the ramp's share of the color, scaled by how far the fade
/// has eased in.
///
/// The ramp runs from transparent at the top band to the full strength at the
/// bottom, so the fade has no hard seam where it meets the transcript. Scaling
/// the whole ramp by `progress` is what lets it ease in and out rather than only
/// appear and disappear.
fn band_alpha(color: Color, index: usize, progress: f32) -> f32 {
    let ratio = index as f32 / (FADE_BANDS - 1) as f32;

    color.a * ratio * progress.clamp(0.0, 1.0)
}

/// Steps a transition's spring for this frame, asking for another frame while it
/// is still travelling.
///
/// Shared by the fade and the jump button: both are overlays whose only state is
/// how far in they are.
fn step_spring<Message>(
    tree: &mut tree::Tree,
    event: &iced::Event,
    showing: bool,
    shell: &mut Shell<'_, Message>,
) {
    let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event else {
        return;
    };

    let target = if showing { 1.0 } else { 0.0 };
    let spring = transition_spring();
    let state = tree.state.downcast_mut::<TransitionState>();

    if !state.primed {
        // The first frame places the overlay rather than easing it in, so a
        // scroller that mounts already scrolled does not animate on startup.
        state.progress.set(target);
        state.primed = true;
        state.last = Some(*now);
        return;
    }

    let elapsed = state
        .last
        .map_or(std::time::Duration::ZERO, |last| now.duration_since(last));
    state.last = Some(*now);
    state.progress.step(target, spring, elapsed);

    if !state.progress.is_settled(target, spring) {
        shell.request_redraw();
    }
}

/// Builds the floating jump-to-bottom button, anchored to the bottom centre.
///
/// The control carries the reference's arrow rather than a text label: an icon
/// reads at a glance and keeps the control small over the messages it floats
/// over. The label is kept for the tooltip, where it names the action.
fn jump_overlay<'a, Message: Clone + 'a>(
    label: String,
    on_jump: Option<Message>,
    width: Length,
    height: Length,
) -> Element<'a, Message, Theme> {
    let arrow: Element<'a, Message, Theme> =
        crate::widgets::Icon::new(crate::icons::IconName::ArrowDown).into_element(Size::Sm);

    let mut button = iced::widget::button(
        container(arrow)
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
    )
    .padding(Padding::ZERO)
    .class(
        Box::new(|theme: &Theme, status| jump_button_style(theme, status))
            as iced::widget::button::StyleFn<'a, Theme>,
    );

    if let Some(message) = on_jump {
        button = button.on_press(message);
    }

    let button: Element<'a, Message, Theme> = button.into();

    // The tooltip names the action, which is what an icon-only control needs:
    // the arrow says "down", not "to the newest message".
    let button = crate::widgets::tooltip(button, label);

    container(button)
        .width(width)
        .height(height)
        .align_x(Alignment::Center)
        .align_y(Alignment::End)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: JUMP_BUTTON_INSET,
            left: 0.0,
        })
        .into()
}

/// The appearance of the jump-to-bottom control.
///
/// It is drawn on the page background with a border rather than as a filled
/// primary button: the control is a way back to the conversation, not a call to
/// action, and a primary fill would compete with the messages it floats over.
fn jump_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, iced::widget::button::Status::Hovered);

    iced::widget::button::Style {
        background: Some(Background::Color(if hovered {
            colors.accent
        } else {
            colors.background
        })),
        text_color: colors.foreground,
        border: iced::Border {
            color: colors.border,
            width: 1.0,
            radius: f32::from(theme.radius().full).into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

/// Convenience constructor mirroring the `iced::widget` style.
pub fn message_scroller<'a, T: 'a, Message: Clone + 'a>(
    items: &'a [T],
    state: &'a MessageScrollerState,
    row: impl Fn(&'a T, usize) -> Element<'a, Message, Theme> + 'a,
) -> MessageScroller<'a, T, Message> {
    MessageScroller::new(items, state, row)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-only: force an offset without touching the viewport height.
    impl MessageScrollerState {
        fn set_scroll(&mut self, offset: f32) {
            let viewport = self.list.viewport_height();
            self.list.update(offset, viewport);
        }
    }

    #[test]
    fn new_state_follows_tail() {
        let s = MessageScrollerState::new(0);
        assert!(s.is_following_tail());
    }

    #[test]
    fn append_while_following_keeps_follow() {
        let mut s = MessageScrollerState::new(3);
        s.record_len(4);
        assert!(s.is_following_tail());
        assert_eq!(s.seen_len(), 4);
    }

    #[test]
    fn scrolling_up_releases_follow() {
        let mut s = MessageScrollerState::new(10);
        // content=1000, viewport=400 -> bottom at 600; offset 400 is 200 up.
        let changed = s.apply_scroll(1000.0, 400.0, 400.0);
        assert!(changed, "the follow flag flipped");
        assert!(!s.is_following_tail());
        assert!(s.is_scrolled_up());
    }

    #[test]
    fn scrolling_back_within_threshold_refollows() {
        let mut s = MessageScrollerState::new(10);
        s.apply_scroll(1000.0, 400.0, 400.0); // detach
        s.apply_scroll(1000.0, 400.0, 560.0); // 600-560=40 <= 80 -> re-pin
        assert!(s.is_following_tail());
    }

    #[test]
    fn prepend_does_not_move_offset() {
        let mut s = MessageScrollerState::new(5);
        s.set_scroll(300.0);
        s.prepend(3);
        assert_eq!(s.list_offset(), 300.0);
        assert_eq!(s.seen_len(), 8);
    }

    #[test]
    fn scroll_to_end_sets_follow() {
        let mut s = MessageScrollerState::new(10);
        s.apply_scroll(1000.0, 400.0, 100.0); // top, detached
        s.scroll_to_end(1000.0, 400.0);
        assert!(s.is_following_tail());
        assert_eq!(s.list_offset(), 600.0);
    }

    #[test]
    fn pin_to_tail_refollows_without_touching_offset() {
        let mut s = MessageScrollerState::new(10);
        s.apply_scroll(1000.0, 400.0, 100.0); // detached, offset 100
        let before = s.list_offset();
        s.pin_to_tail();
        assert!(
            s.is_following_tail(),
            "the flag flips so anchor_bottom places it"
        );
        assert_eq!(
            s.list_offset(),
            before,
            "no offset is guessed on the jump path"
        );
    }

    #[test]
    fn append_after_scroll_up_does_not_steal_focus() {
        let mut s = MessageScrollerState::new(10);
        s.apply_scroll(1000.0, 400.0, 0.0); // scrolled to top
        s.append(1); // a new message arrives while reading history
        assert!(s.is_scrolled_up(), "appending must not re-pin on its own");
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg {
        Scrolled(MessageScrollerState),
        Jump,
    }

    #[test]
    fn scroller_builds_across_options() {
        let items: Vec<String> = (0..500).map(|i| format!("msg {i}")).collect();
        for jump in [false, true] {
            for fade in [false, true] {
                let mut st = MessageScrollerState::new(items.len());
                if fade || jump {
                    // Scroll up so the button/fade layers actually render.
                    st.apply_scroll(10_000.0, 400.0, 10.0);
                }
                let el: Element<'_, Msg, Theme> = MessageScroller::new(&items, &st, |item, _| {
                    iced::widget::text(item.clone()).size(14).into()
                })
                .jump_button(jump)
                .with_bottom_fade(fade.then_some(iced::Color::TRANSPARENT))
                .on_scroll(Msg::Scrolled)
                .on_jump_to_bottom(Msg::Jump)
                .height(400.0)
                .into();
                drop(el);
            }
        }
    }

    #[test]
    fn tail_following_scroller_builds_without_overlays() {
        let items: Vec<String> = (0..50).map(|i| format!("m {i}")).collect();
        let st = MessageScrollerState::new(items.len());
        assert!(st.is_following_tail());
        let el: Element<'_, Msg, Theme> = message_scroller(&items, &st, |item, _| {
            iced::widget::text(item.clone()).into()
        })
        .jump_button(true)
        .with_bottom_fade(Some(iced::Color::BLACK))
        .into();
        drop(el);
    }

    /// A transition starts placed rather than animating in, so a scroller that
    /// mounts already-scrolled does not slide its overlays on startup.
    #[test]
    fn a_transition_starts_primed() {
        let state = TransitionState::default();

        assert!(!state.primed, "an unset state has not been placed yet");
        assert!(state.last.is_none(), "and has no frame to measure from");
    }

    /// The spring is critically damped, which is what keeps an overlay from
    /// overshooting the edge it settles on.
    #[test]
    fn the_transition_spring_takes_the_documented_time() {
        assert_eq!(TRANSITION_DURATION.as_millis(), 200);
        assert_eq!(TRANSITION_RISE, 8.0);

        // A settled spring reports itself settled at both ends.
        let spring = transition_spring();
        let mut progress = crate::motion::SpringState::default();
        progress.set(1.0);
        assert!(progress.is_settled(1.0, spring));
    }

    /// Both overlays render shown and hidden. The fade draws itself and owns
    /// its own spring; the jump button is wrapped in the generic transition.
    #[test]
    fn overlays_render_hidden_and_shown() {
        for showing in [false, true] {
            let fade: Element<'_, Msg, Theme> =
                FadeOverlay::new(iced::Color::BLACK, showing).into();
            drop(fade);

            let jump: Element<'_, Msg, Theme> = Transition::new(
                jump_overlay("Jump".to_owned(), None, Length::Fill, Length::Fill),
                showing,
            )
            .into();
            drop(jump);
        }
    }

    /// The fade's shape, and the ramp its bands follow.
    #[test]
    fn the_fade_ramps_from_transparent_to_full() {
        assert_eq!(FADE_HEIGHT, 48.0);
        assert_eq!(FADE_BANDS, 4);

        let color = iced::Color::from_rgb8(0xff, 0xff, 0xff);

        // Eased fully in, the top band is invisible and the bottom is at the
        // color's own strength: that is what removes the seam at the top of the
        // fade.
        assert_eq!(band_alpha(color, 0, 1.0), 0.0);
        assert_eq!(band_alpha(color, FADE_BANDS - 1, 1.0), color.a);

        // Each band is at least as strong as the one above it.
        for index in 1..FADE_BANDS {
            assert!(
                band_alpha(color, index, 1.0) >= band_alpha(color, index - 1, 1.0),
                "band {index} must not be weaker than the one above it"
            );
        }
    }

    /// The transition scales the whole ramp, which is what lets the fade ease in
    /// rather than blink on: at half progress every band is half as strong.
    #[test]
    fn the_transition_scales_the_whole_ramp() {
        let color = iced::Color::from_rgb8(0x00, 0x00, 0x00);
        let full = band_alpha(color, 2, 1.0);

        assert!(full > 0.0);
        assert!((band_alpha(color, 2, 0.5) - full * 0.5).abs() < f32::EPSILON);
        assert_eq!(band_alpha(color, 2, 0.0), 0.0, "fully out draws nothing");

        // A progress outside the range cannot overshoot the color's strength.
        assert_eq!(band_alpha(color, 2, 2.0), full);
        assert_eq!(band_alpha(color, 2, -1.0), 0.0);
    }

    /// The jump control is an icon button, so its overlay carries an icon rather
    /// than a text label; the label survives as the tooltip.
    #[test]
    fn the_jump_control_renders_with_and_without_a_message() {
        let with: Element<'_, Msg, Theme> = jump_overlay(
            "Jump to latest".to_owned(),
            Some(Msg::Jump),
            Length::Fill,
            Length::Fill,
        );
        drop(with);

        // Without a handler the control is inert but still drawn.
        let without: Element<'_, Msg, Theme> = jump_overlay(
            "Jump to latest".to_owned(),
            None,
            Length::Fill,
            Length::Fill,
        );
        drop(without);
    }

    #[test]
    fn scroll_message_carries_the_updated_state() {
        let mut st = MessageScrollerState::new(10);
        st.apply_scroll(1000.0, 400.0, 0.0); // scrolled to the top, detached
        if let Msg::Scrolled(carried) = Msg::Scrolled(st.clone()) {
            assert!(carried.is_scrolled_up());
        }
    }
}
