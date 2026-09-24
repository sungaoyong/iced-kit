//! A virtualized, tail-following transcript scroller.
//!
//! The state machine lives here ([`MessageScrollerState`]); the widget that
//! renders it ([`MessageScroller`]) is added alongside it. The state is plain,
//! app-owned data — the same ownership model as
//! [`VirtualListState`](crate::widgets::virtual_list::VirtualListState) — so the
//! scroller stays stateless and testable.

use crate::widgets::virtual_list::VirtualListState;
use std::cmp::Ordering;

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
        match new_len.cmp(&self.seen_len) {
            // Append: if already following, stay following. Never steal focus
            // from a user who has scrolled up.
            Ordering::Greater => {}
            // Shrink: nothing special; the offset clamps during layout.
            Ordering::Less | Ordering::Equal => {}
        }
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
