//! Showing and hiding something that animates as it leaves.
//!
//! An enter transition can be owned by the component that draws the surface: the
//! surface exists, so it can animate itself into place. A leave transition
//! cannot work that way. By the time the application has decided a dialog is
//! closed, it has also stopped passing the dialog's content — so there is
//! nothing left to draw while the exit runs.
//!
//! [`Presence`] is the piece the application owns to bridge that gap. The
//! application keeps it in its own state, tells it when something opens or
//! closes, and keeps supplying the content while it reports
//! [`should_render`](Presence::should_render). The surface itself reads
//! [`progress`](Presence::progress) to know how far through its exit it is.
//!
//! ```
//! # use iced_kit::motion::Presence;
//! # use iced::time::Instant;
//! # #[derive(Debug, Clone)]
//! # enum Message { Toggle, Dismissed, Frame }
//! # struct App { presence: Presence, open: bool }
//! # impl App {
//! fn update(&mut self, message: Message) {
//!     match message {
//!         Message::Toggle => {
//!             self.open = !self.open;
//!             self.presence.show(self.open, Instant::now());
//!         }
//!         Message::Dismissed => self.presence.dismiss(Instant::now()),
//!         // Ask for a frame while the exit runs, so it is drawn to completion.
//!         Message::Frame => {}
//!     }
//! }
//!
//! fn visible(&self) -> bool {
//!     self.open || self.presence.should_render()
//! }
//! # }
//! ```
//!
//! # Why the application has to own it
//!
//! The alternative would be for the component to hold the content it is
//! animating out. It cannot: iced widgets are rebuilt every frame from the
//! application's state, and the retained `tree::State` a widget is allowed has
//! to be `'static` — it cannot hold an `Element`, which borrows from the
//! application. So the application keeps the content and asks this whether it is
//! still needed.

use std::time::{Duration, Instant};

use crate::motion::{Progress, DURATION_NORMAL};

/// How long an exit takes.
///
/// Shorter than a [[`DURATION_NORMAL`]] entrance: a surface that is going away
/// has already been read, so waiting for it is waiting for nothing.
pub const DURATION_EXIT: Duration = Duration::from_millis(140);

/// Tracks whether something should still be drawn as it leaves.
///
/// The application owns this value, hands it the current time when the surface
/// opens or closes, and asks it whether to keep rendering.
#[derive(Debug, Clone)]
pub struct Presence {
    progress: Progress,
    /// Whether the surface is showing.
    showing: bool,
    /// The duration a transition takes, so a caller can retime it.
    duration: Duration,
}

impl Presence {
    /// Creates a presence that is hidden and not animating.
    pub fn new() -> Self {
        Self {
            progress: Progress::new(false),
            showing: false,
            duration: DURATION_NORMAL,
        }
    }

    /// Creates a presence that is already showing, for something that is open
    /// when the application starts.
    pub fn visible() -> Self {
        Self {
            progress: Progress::new(true),
            showing: true,
            duration: DURATION_NORMAL,
        }
    }

    /// Sets how long the transition takes.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Shows the surface, or starts it leaving.
    ///
    /// Call this from the application's update when the surface is opened or
    /// closed. Calling it with the state it already has changes nothing, so it
    /// is safe to call on every frame.
    pub fn show(&mut self, showing: bool, at: Instant) {
        if self.showing == showing {
            return;
        }

        // The transition is rebuilt rather than retimed, because iced's
        // animation takes its timing through consuming builders. Rebuilding
        // restarts from a standstill, which is correct here: an exit that has
        // been replaced by an entrance should begin its arrival from nothing
        // rather than carry the momentum of the exit it interrupted.
        self.progress = Progress::with_timing(
            self.showing,
            if showing {
                self.duration
            } else {
                DURATION_EXIT
            },
            if showing {
                crate::motion::ease_enter
            } else {
                crate::motion::ease_exit
            },
        );
        self.progress.set_target(showing, at);
        self.showing = showing;
    }

    /// Starts the surface leaving.
    ///
    /// This is the same as [`Self::show`] with `false`, and reads better at a
    /// dismissal site.
    pub fn dismiss(&mut self, at: Instant) {
        self.show(false, at);
    }

    /// Returns whether the surface is showing, whether or not it has finished
    /// arriving.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.showing
    }

    /// Returns whether the surface should be drawn this frame.
    ///
    /// This is `true` while the surface is showing and also while it is leaving,
    /// which is what lets the application stop supplying it only once its exit
    /// has been drawn. It becomes `false` once the exit has finished.
    ///
    /// The application must keep rendering while this is true; it does not have
    /// to ask for frames itself, because the surface that draws the exit drives
    /// them.
    #[must_use]
    pub fn should_render(&self) -> bool {
        self.showing || self.progress.is_animating(Instant::now())
    }

    /// Returns how far the surface has arrived, from `0.0` to `1.0`.
    ///
    /// A surface scales its own chrome by this — an offset, an alpha, a shadow —
    /// so that its exit is visible. It is `1.0` while the surface is fully
    /// present.
    #[must_use]
    pub fn progress(&self, at: Instant) -> f32 {
        self.progress.value(at)
    }

    /// Returns whether a transition is running.
    #[must_use]
    pub fn is_animating(&self, at: Instant) -> bool {
        self.progress.is_animating(at)
    }
}

impl Default for Presence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Presence, DURATION_EXIT};
    use std::time::Instant;

    /// A fresh presence must not ask to be drawn, or every application would
    /// render a surface it never opened.
    #[test]
    fn a_fresh_presence_is_hidden() {
        let presence = Presence::new();

        assert!(!presence.is_visible());
        assert!(!presence.should_render());
    }

    /// A presence created as visible must be fully arrived, so something open at
    /// startup is not animated open again.
    #[test]
    fn a_visible_presence_starts_arrived() {
        let presence = Presence::visible();

        assert!(presence.is_visible());
        assert!(presence.should_render());
        assert_eq!(presence.progress(std::time::Instant::now()), 1.0);
    }

    /// The surface must keep being drawn while it leaves, which is the whole
    /// point of the type: the application has already closed it by then.
    #[test]
    fn a_leaving_surface_is_still_drawn() {
        let mut presence = Presence::visible();
        let now = std::time::Instant::now();

        presence.dismiss(now);

        assert!(!presence.is_visible(), "the surface is closed");
        assert!(
            presence.should_render(),
            "but its exit is still being drawn"
        );
    }

    /// Once the exit has run, the surface must stop being drawn — otherwise the
    /// application would render it forever.
    #[test]
    fn a_finished_exit_stops_being_drawn() {
        let mut presence = Presence::visible();
        let now = std::time::Instant::now();

        presence.dismiss(now);

        let after = now + DURATION_EXIT + std::time::Duration::from_millis(50);

        assert!(
            !should_render_at(&presence, after),
            "a finished exit must stop asking to be drawn"
        );
        assert_eq!(presence.progress(after), 0.0);
    }

    /// The progress must run from 1 down to 0 as the surface leaves, so a
    /// surface can scale its offset and alpha by it.
    #[test]
    fn the_progress_falls_as_the_surface_leaves() {
        let mut presence = Presence::visible();
        let now = std::time::Instant::now();

        presence.dismiss(now);

        let start = presence.progress(now);
        let middle = presence.progress(now + DURATION_EXIT / 2);
        let end = presence.progress(now + DURATION_EXIT);

        assert_eq!(start, 1.0, "an exit starts from fully present");
        assert!(
            middle < start && middle > end,
            "the progress must fall through the middle: {start}, {middle}, {end}"
        );
        assert_eq!(end, 0.0, "an exit ends fully gone");
    }

    /// Showing a surface that is already showing must not restart its entrance,
    /// or a redraw would re-trigger the animation every frame.
    #[test]
    fn showing_an_already_visible_surface_changes_nothing() {
        let mut presence = Presence::visible();
        let now = std::time::Instant::now();

        presence.show(true, now);

        assert!(presence.is_visible());
        assert_eq!(presence.progress(now), 1.0);
        assert!(!presence.is_animating(now), "nothing was started");
    }

    /// A dismissal is just a hide, so it must say so.
    #[test]
    fn dismissing_hides_the_surface() {
        let mut presence = Presence::visible();
        presence.dismiss(std::time::Instant::now());

        assert!(!presence.is_visible());
    }

    #[test]
    fn the_exit_is_shorter_than_an_entrance() {
        assert!(
            DURATION_EXIT < crate::motion::DURATION_NORMAL,
            "a surface on its way out should not keep the reader waiting"
        );
    }

    /// Returns whether the surface should be drawn at a given instant.
    ///
    /// Split out so the behavior can be tested at a chosen time rather than
    /// against the wall clock.
    fn should_render_at(presence: &Presence, at: Instant) -> bool {
        presence.is_visible() || presence.is_animating(at)
    }
}
