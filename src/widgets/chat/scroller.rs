//! A virtualized, tail-following transcript scroller.
//!
//! The state machine lives here ([`MessageScrollerState`]); the widget that
//! renders it ([`MessageScroller`]) is added alongside it. The state is plain,
//! app-owned data — the same ownership model as
//! [`VirtualListState`](crate::widgets::virtual_list::VirtualListState) — so the
//! scroller stays stateless and testable.

use crate::theme::Theme;
use crate::widgets::virtual_list::VirtualList;
use crate::widgets::virtual_list::VirtualListState;
use iced::widget::{column, container, stack, text, Space};
use iced::{Alignment, Background, Color, Element, Length, Padding};

/// Distance (px) from the bottom within which the scroller re-sticks to the tail.
const FOLLOW_THRESHOLD: f32 = 80.0;

/// App-owned scroller state: scroll/measure ([`VirtualListState`]), whether the
/// view is pinned to the newest message, and the last-seen item count.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MessageScrollerState {
    list: VirtualListState,
    follow_tail: bool,
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

    /// Jumps to the very bottom and pins the view there.
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

        // The fade and the button sit above the content, hug the bottom edge,
        // and only appear once the reader has scrolled away from the tail.
        if scrolled_up {
            if let Some(color) = bottom_fade {
                layers.push(fade_overlay(color, width, height));
            }
            if jump_button {
                layers.push(jump_overlay(jump_label, on_jump, width, height));
            }
        }

        stack(layers).width(width).height(height).into()
    }
}

impl<'a, T: 'a, Message: Clone + 'a> From<MessageScroller<'a, T, Message>>
    for Element<'a, Message, Theme>
{
    fn from(scroller: MessageScroller<'a, T, Message>) -> Self {
        scroller.into_element()
    }
}

/// Builds a translucent gradient (as stacked bands) hugging the bottom edge.
fn fade_overlay<'a, Message: 'a>(
    color: Color,
    width: Length,
    height: Length,
) -> Element<'a, Message, Theme> {
    const BANDS: usize = 4;
    const FADE_HEIGHT: f32 = 48.0;

    let mut bands = column![].spacing(0).width(Length::Fill);
    for index in 0..BANDS {
        // Ramp the alpha up toward the bottom so content dissolves into `color`.
        let ratio = (index + 1) as f32 / BANDS as f32;
        let band = Color {
            a: color.a * ratio,
            ..color
        };
        bands = bands.push(
            container(
                Space::new()
                    .width(Length::Fill)
                    .height(Length::Fixed(FADE_HEIGHT / BANDS as f32)),
            )
            .style(move |_: &Theme| iced::widget::container::Style {
                background: Some(Background::Color(band)),
                ..Default::default()
            }),
        );
    }

    container(bands)
        .width(width)
        .height(height)
        .align_x(Alignment::Center)
        .align_y(Alignment::End)
        .into()
}

/// Builds the floating jump-to-bottom button, anchored to the bottom centre.
fn jump_overlay<'a, Message: Clone + 'a>(
    label: String,
    on_jump: Option<Message>,
    width: Length,
    height: Length,
) -> Element<'a, Message, Theme> {
    let mut button = iced::widget::button(text(label).size(12.0)).class(
        Box::new(|theme: &Theme, _status| {
            let c = theme.colors();
            iced::widget::button::Style {
                background: Some(Background::Color(c.primary)),
                text_color: c.primary_foreground,
                border: iced::Border {
                    radius: f32::from(theme.radius().full).into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }) as iced::widget::button::StyleFn<'a, Theme>,
    );

    if let Some(message) = on_jump {
        button = button.on_press(message);
    }

    container(button)
        .width(width)
        .height(height)
        .align_x(Alignment::Center)
        .align_y(Alignment::End)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 12.0,
            left: 0.0,
        })
        .into()
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
                let el: Element<'_, Msg, Theme> = MessageScroller::new(
                    &items,
                    &st,
                    |item, _| iced::widget::text(item.clone()).size(14).into(),
                )
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
        let el: Element<'_, Msg, Theme> =
            message_scroller(&items, &st, |item, _| iced::widget::text(item.clone()).into())
                .jump_button(true)
                .with_bottom_fade(Some(iced::Color::BLACK))
                .into();
        drop(el);
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
