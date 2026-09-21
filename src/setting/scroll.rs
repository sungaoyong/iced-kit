//! Scrolling the settings content to a group.
//!
//! iced reports a widget's bounds during a layout traversal, and the group's `y`
//! within the content is exactly the scroll offset that brings it to the top of
//! the viewport. Reading that value is the whole job, but it has to happen in two
//! passes, because a traversal reaches a scrollable *before* the content inside
//! it: at the moment the scrollable is visited, the group's position is not yet
//! known.
//!
//! [`operation::then`] is iced's own answer to that ordering. The first pass
//! finds the group and reports its offset; the second pass is built from that
//! value and performs the scroll, reusing iced's [`scroll_to`] rather than
//! reimplementing it.

use iced::advanced::widget::operation::scrollable::AbsoluteOffset;
use iced::advanced::widget::operation::{scrollable, Operation, Outcome};
use iced::advanced::widget::Id;
use iced::Rectangle;

use super::panel::content_id;

/// An operation that scrolls the settings content so `target` is at the top.
///
/// If the group is absent — a search hid it, or its page is not the one being
/// drawn — this does nothing rather than scrolling somewhere arbitrary.
pub(crate) fn scroll_to_group<T: Send + 'static>(target: Id) -> impl Operation<T> {
    iced::advanced::widget::operation::then(FindContainer::new(target), |offset: f32| {
        scrollable::scroll_to::<T>(
            content_id(),
            AbsoluteOffset {
                x: None,
                y: Some(offset),
            },
        )
    })
}

/// The first pass: reports the `y` of the container with the given id.
struct FindContainer {
    target: Id,
    offset: Option<f32>,
}

impl FindContainer {
    fn new(target: Id) -> Self {
        Self {
            target,
            offset: None,
        }
    }
}

impl Operation<f32> for FindContainer {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<f32>)) {
        // The target is somewhere in the subtree, so the traversal continues.
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.target) {
            // Bounds are reported in the content's own space, so this is already
            // the offset that brings the group to the top of the viewport.
            self.offset = Some(bounds.y.max(0.0));
        }
    }

    fn finish(&self) -> Outcome<f32> {
        match self.offset {
            Some(offset) => Outcome::Some(offset),
            // Reporting `None` stops the chain, so a missing group leaves the
            // scroll untouched.
            None => Outcome::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FindContainer;
    use iced::advanced::widget::operation::Operation;
    use iced::advanced::widget::Id;
    use iced::Rectangle;

    /// Runs the operation's container callback over the visited containers and
    /// reports what it found.
    ///
    /// `Id::new` takes `'static` data, so the ids are `String`s rather than
    /// borrowed `&str`s.
    fn offset_found(target: &'static str, visited: &[(&'static str, f32)]) -> Option<f32> {
        let mut op = FindContainer::new(Id::new(target));

        for (id, y) in visited {
            op.container(
                Some(&Id::new(id)),
                Rectangle {
                    x: 0.0,
                    y: *y,
                    width: 100.0,
                    height: 10.0,
                },
            );
        }

        op.offset
    }

    #[test]
    fn the_targets_offset_is_reported() {
        let found = offset_found(
            "group",
            &[("header", 0.0), ("group", 120.0), ("other", 300.0)],
        );

        assert_eq!(found, Some(120.0));
    }

    #[test]
    fn an_absent_target_reports_nothing() {
        let found = offset_found("group", &[("header", 0.0), ("other", 300.0)]);
        assert_eq!(found, None);
    }

    #[test]
    fn an_unidentified_container_is_ignored() {
        // A container with no id must not match a target, or the first unlabelled
        // wrapper in the tree would be scrolled to.
        let mut op = FindContainer::new(Id::new("group"));
        op.container(
            None,
            Rectangle {
                x: 0.0,
                y: 999.0,
                width: 1.0,
                height: 1.0,
            },
        );

        assert_eq!(op.offset, None);
    }

    #[test]
    fn a_negative_offset_is_clamped_to_the_top() {
        // Scrolling above the content would leave a blank strip.
        let found = offset_found("group", &[("group", -40.0)]);
        assert_eq!(found, Some(0.0));
    }

    #[test]
    fn the_last_matching_container_wins() {
        // Ids are meant to be unique; if a caller reuses one, the deepest match
        // is the more specific answer.
        let found = offset_found("dup", &[("dup", 10.0), ("dup", 90.0)]);
        assert_eq!(found, Some(90.0));
    }

    #[test]
    fn a_missing_target_stops_the_chain_without_scrolling() {
        // `Outcome::None` is what tells iced's `then` not to run the follow-up
        // scroll, which is what keeps a hidden group from scrolling to the top.
        let op = FindContainer::new(Id::new("absent"));

        assert!(matches!(
            Operation::<f32>::finish(&op),
            iced::advanced::widget::operation::Outcome::None
        ));
    }
}
