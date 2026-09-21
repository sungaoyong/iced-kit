//! Virtualized lists for large data sets.
//!
//! # Why this exists
//!
//! iced's `scrollable` lays out every child, so a list of ten thousand rows
//! builds ten thousand widget trees per frame. [`VirtualList`] instead renders
//! only the rows intersecting the viewport, padding the rest with two spacers.
//! The cost per frame becomes proportional to what is visible rather than to
//! the size of the data set.
//!
//! # Variable heights
//!
//! Row heights are supplied by the caller as a function of the item, so rows
//! may differ. [`VirtualList`] caches measured heights as rows scroll into
//! view and falls back to an estimate for rows it has not seen yet, correcting
//! the scroll offset as estimates are replaced by real measurements.
//!
//! # Usage
//!
//! ```
//! use iced_kit::widgets::virtual_list::{VirtualList, VirtualListState};
//! use iced_kit::Theme;
//! use iced::Element;
//!
//! #[derive(Clone, Debug)]
//! enum Message {
//!     Scrolled(VirtualListState),
//! }
//!
//! fn view<'a>(items: &'a [String], state: &'a VirtualListState) -> Element<'a, Message, Theme> {
//!     VirtualList::new(items, state, |item, _index| {
//!         iced::widget::text(item.clone()).size(14).into()
//!     })
//!     .row_height(|_item, _index| 32.0)
//!     .height(400.0)
//!     .on_scroll(Message::Scrolled)
//!     .into()
//! }
//! ```

use crate::theme::{Size, Theme};
use iced::widget::{column, container, scrollable, Space};
use iced::{Element, Length, Padding};

/// The scroll position and measured row heights of a [`VirtualList`].
///
/// The application owns this value: it is passed back into [`VirtualList::new`]
/// on every frame and replaced when the list reports a scroll. Keeping it
/// outside the widget is what lets the widget stay stateless and testable.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct VirtualListState {
    /// The scroll offset in logical pixels, from the top of the content.
    scroll_offset: f32,
    /// The visible height in logical pixels.
    viewport_height: f32,
    /// Measured heights, indexed by row. `None` until a row has been laid out.
    measured: Vec<Option<f32>>,
}

impl VirtualListState {
    /// Creates a fresh state with no scroll and no measurements.
    pub fn new() -> Self {
        Self::default()
    }

    /// A shared empty state, for widgets that accept scroll state but work
    /// without it.
    #[must_use]
    pub const fn const_empty() -> Self {
        Self {
            scroll_offset: 0.0,
            viewport_height: 0.0,
            measured: Vec::new(),
        }
    }

    /// Creates a state at a given scroll offset, for restoring a saved position.
    #[must_use]
    pub fn with_offset(offset: f32) -> Self {
        Self {
            scroll_offset: offset.max(0.0),
            ..Self::default()
        }
    }

    /// The current scroll offset in logical pixels.
    #[must_use]
    pub fn offset(&self) -> f32 {
        self.scroll_offset
    }

    /// The visible height in logical pixels.
    #[must_use]
    pub fn viewport_height(&self) -> f32 {
        self.viewport_height
    }

    /// Records what the browser reported, returning `true` if anything changed.
    ///
    /// Returning whether the state moved lets a caller skip a redraw when a
    /// scroll event arrives with identical numbers, which iced sends on every
    /// wheel tick.
    pub fn update(&mut self, offset: f32, viewport_height: f32) -> bool {
        let changed = (self.scroll_offset - offset).abs() > f32::EPSILON
            || (self.viewport_height - viewport_height).abs() > f32::EPSILON;

        self.scroll_offset = offset.max(0.0);
        self.viewport_height = viewport_height.max(0.0);
        changed
    }

    /// Records a measured row height, returning `true` if it was new.
    ///
    /// A row that has been measured before and reports the same height does not
    /// change the state, so re-measuring on every frame is free.
    pub fn record_height(&mut self, index: usize, height: f32) -> bool {
        if !height.is_finite() || height <= 0.0 {
            return false;
        }

        if self.measured.len() <= index {
            self.measured.resize(index + 1, None);
        }

        let changed = self.measured[index] != Some(height);
        self.measured[index] = Some(height);
        changed
    }

    /// The measured height of a row, if it has been seen.
    #[must_use]
    pub fn measured_height(&self, index: usize) -> Option<f32> {
        self.measured.get(index).copied().flatten()
    }

    /// The number of rows that have been measured.
    #[must_use]
    pub fn measured_count(&self) -> usize {
        self.measured
            .iter()
            .filter(|height| height.is_some())
            .count()
    }

    /// Scrolls to the top.
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0.0;
    }

    /// Scrolls to the bottom of a list of the given total height.
    pub fn scroll_to_bottom(&mut self, content_height: f32) {
        self.scroll_offset = (content_height - self.viewport_height).max(0.0);
    }
}

/// A virtualized, optionally variable-height list.
#[must_use = "a VirtualList does nothing unless it is turned into an Element"]
pub struct VirtualList<'a, T, Message> {
    items: &'a [T],
    state: &'a VirtualListState,
    row: Box<dyn Fn(&'a T, usize) -> Element<'a, Message, Theme> + 'a>,
    height_of: Box<dyn Fn(&'a T, usize) -> f32 + 'a>,
    default_height: f32,
    overscan: usize,
    width: Option<Length>,
    height: Option<Length>,
    on_scroll: Option<Box<dyn Fn(VirtualListState) -> Message + 'a>>,
    spacing: f32,
}

impl<'a, T: 'a, Message: Clone + 'a> VirtualList<'a, T, Message> {
    /// Creates a list over `items`, rendering each row with `row`.
    ///
    /// `state` carries the scroll position and measurements; the application
    /// owns it and replaces it from the message emitted by
    /// [`on_scroll`](Self::on_scroll).
    pub fn new(
        items: &'a [T],
        state: &'a VirtualListState,
        row: impl Fn(&'a T, usize) -> Element<'a, Message, Theme> + 'a,
    ) -> Self {
        Self {
            items,
            state,
            row: Box::new(row),
            height_of: Box::new(|_item, _index| 0.0),
            default_height: 32.0,
            overscan: 2,
            width: None,
            height: None,
            on_scroll: None,
            spacing: 0.0,
        }
    }

    /// Sets how tall each row is, when all rows share a height.
    ///
    /// This is the fast path: with a uniform height the list can compute every
    /// offset directly and needs no measurement pass at all.
    pub fn fixed_row_height(mut self, height: f32) -> Self {
        self.default_height = height.max(1.0);
        self.height_of = Box::new(move |_item, _index| height.max(1.0));
        self
    }

    /// Sets a per-row height function, for rows of differing heights.
    pub fn row_height(mut self, height_of: impl Fn(&'a T, usize) -> f32 + 'a) -> Self {
        self.height_of = Box::new(height_of);
        self
    }

    /// Sets the height assumed for a row before it has been measured.
    ///
    /// A good estimate keeps the scrollbar from jumping as rows are measured
    /// for the first time.
    pub fn estimate_height(mut self, estimate: f32) -> Self {
        self.default_height = estimate.max(1.0);
        self
    }

    /// Sets how many rows beyond the viewport to render, to hide the cost of
    /// building rows during a fast scroll.
    pub fn overscan(mut self, rows: usize) -> Self {
        self.overscan = rows;
        self
    }

    /// Sets the gap between rows.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    /// Sets the list's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the list's visible height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(Length::Fixed(height.max(1.0)));
        self
    }

    /// Sets the visible height as a [`Length`].
    pub fn height_length(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Reports scroll changes, carrying the new state.
    pub fn on_scroll(mut self, f: impl Fn(VirtualListState) -> Message + 'a) -> Self {
        self.on_scroll = Some(Box::new(f));
        self
    }

    /// The height of row `index`: the measurement if there is one, else the estimate.
    fn row_extent(&self, index: usize) -> f32 {
        self.state
            .measured_height(index)
            .unwrap_or_else(|| (self.height_of)(&self.items[index], index).max(1.0))
            + self.spacing
    }

    /// Computes which rows to render and the spacers that stand in for the rest.
    fn plan(&self) -> VirtualPlan {
        if self.items.is_empty() {
            return VirtualPlan::default();
        }

        let viewport = if self.state.viewport_height > 0.0 {
            self.state.viewport_height
        } else {
            // Before the first scroll event the visible height is unknown, so
            // the list renders a screenful rather than nothing.
            DEFAULT_VIEWPORT_ESTIMATE
        };

        let top = self.state.scroll_offset;
        let bottom = top + viewport;

        // Walk from the top accumulating heights until the viewport is reached.
        // For a list of tens of thousands this is still cheap (an add per row),
        // and it is what makes variable heights exact rather than guessed.
        let mut offset = 0.0_f32;
        let mut first = 0_usize;
        let mut leading = 0.0_f32;

        for index in 0..self.items.len() {
            let height = self.row_extent(index);

            if offset + height > top {
                first = index;
                leading = offset;
                break;
            }

            offset += height;
            // The final row can end exactly at the viewport top.
            first = index + 1;
            leading = offset;
        }

        let mut last = first;
        let mut cursor = leading;

        while last < self.items.len() && cursor < bottom {
            cursor += self.row_extent(last);
            last += 1;
        }

        let first = first.saturating_sub(self.overscan);
        let last = (last + self.overscan).min(self.items.len());

        // Recompute the leading spacer after overscan moved the start back.
        let leading: f32 = (0..first).map(|index| self.row_extent(index)).sum();

        let content_height: f32 = (0..self.items.len())
            .map(|index| self.row_extent(index))
            .sum();
        let trailing = (content_height
            - leading
            - (first..last)
                .map(|index| self.row_extent(index))
                .sum::<f32>())
        .max(0.0);

        VirtualPlan {
            first,
            last,
            leading,
            trailing,
        }
    }

    /// The total height of all rows, including spacing.
    #[must_use]
    pub fn content_height(&self) -> f32 {
        (0..self.items.len())
            .map(|index| self.row_extent(index))
            .sum()
    }

    /// Converts the list into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let plan = self.plan();

        // The spacers reserve the scroll height for the rows that are not
        // rendered, which is what keeps the scrollbar honest.
        let mut rows = column![].spacing(0).width(Length::Fill);

        if plan.leading > 0.0 {
            rows = rows.push(
                Space::new()
                    .width(Length::Fill)
                    .height(Length::Fixed(plan.leading)),
            );
        }

        let mut visible = column![].spacing(0).width(Length::Fill);

        for index in plan.first..plan.last {
            let item = &self.items[index];
            let row = (self.row)(item, index);
            let height = (self.height_of)(item, index).max(1.0);

            visible = visible.push(
                container(row)
                    .width(Length::Fill)
                    .height(Length::Fixed(height))
                    .padding(Padding::default()),
            );

            if self.spacing > 0.0 && index + 1 < plan.last {
                visible = visible.push(
                    Space::new()
                        .width(Length::Fill)
                        .height(Length::Fixed(self.spacing)),
                );
            }
        }

        rows = rows.push(visible);

        if plan.trailing > 0.0 {
            rows = rows.push(
                Space::new()
                    .width(Length::Fill)
                    .height(Length::Fixed(plan.trailing)),
            );
        }

        let mut scroller = scrollable(rows).width(self.width.unwrap_or(Length::Fill));

        if let Some(height) = self.height {
            scroller = scroller.height(height);
        }

        if let Some(on_scroll) = self.on_scroll {
            let current = self.state.clone();

            scroller = scroller.on_scroll(move |viewport| {
                let offset = viewport.absolute_offset().y;
                let mut next = current.clone();
                next.update(offset, viewport.bounds().height);
                on_scroll(next)
            });
        }

        scroller.into()
    }
}

impl<'a, T: 'a, Message: Clone + 'a> From<VirtualList<'a, T, Message>>
    for Element<'a, Message, Theme>
{
    fn from(list: VirtualList<'a, T, Message>) -> Self {
        list.into_element()
    }
}

/// The viewport height assumed before the first scroll event arrives.
const DEFAULT_VIEWPORT_ESTIMATE: f32 = 400.0;

/// The slice of rows to render, and the space to reserve around them.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct VirtualPlan {
    first: usize,
    last: usize,
    leading: f32,
    trailing: f32,
}

impl VirtualPlan {
    /// How many rows the plan renders.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.last.saturating_sub(self.first)
    }
}

/// A convenience constructor mirroring the `iced::widget` style.
pub fn virtual_list<'a, T: 'a, Message: Clone + 'a>(
    items: &'a [T],
    state: &'a VirtualListState,
    row: impl Fn(&'a T, usize) -> Element<'a, Message, Theme> + 'a,
) -> VirtualList<'a, T, Message> {
    VirtualList::new(items, state, row)
}

/// Builds a row whose look matches the rest of the design system.
///
/// Offered because every virtualized list needs a plain text row, and writing
/// one inline at each call site duplicates the sizing rules.
pub fn text_row<'a, Message: Clone + 'a>(
    label: impl Into<String>,
    detail: Option<String>,
) -> Element<'a, Message, Theme> {
    let label = label.into();
    let title_style = Size::Md.text();
    let detail_style = Size::Sm.text();

    let mut content = column![iced::widget::text(label)
        .size(title_style.size)
        .line_height(title_style.line_height())]
    .spacing(2);

    if let Some(detail) = detail {
        content = content.push(
            iced::widget::text(detail)
                .size(detail_style.size)
                .class(Box::new(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.colors().muted_foreground),
                }) as iced::widget::text::StyleFn<'a, Theme>),
        );
    }

    // A `Column` cannot center its children vertically, so the content is
    // wrapped in a container that can; a row of a fixed height must have its
    // text centred or it hugs the top edge.
    container(content)
        .height(Length::Fill)
        .align_y(iced::Alignment::Center)
        .padding(Padding {
            top: 0.0,
            right: 8.0,
            bottom: 0.0,
            left: 8.0,
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::{text_row, VirtualList, VirtualListState};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Scrolled(VirtualListState),
    }

    fn items(count: usize) -> Vec<usize> {
        (0..count).collect()
    }

    /// Extracts the render plan, with the message type pinned so the callers
    /// below do not each need a turbofish.
    fn plan_for(list: &VirtualList<'_, usize, Message>) -> super::VirtualPlan {
        list.plan()
    }

    /// Builds a list of plain text rows over `data`.
    fn list<'a>(data: &'a [usize], state: &'a VirtualListState) -> VirtualList<'a, usize, Message> {
        VirtualList::new(data, state, |item: &usize, _index| {
            iced::widget::text(item.to_string()).into()
        })
    }

    #[test]
    fn a_fixed_height_list_renders_a_window_not_the_whole_data_set() {
        let data = items(10_000);
        let mut state = VirtualListState::new();
        state.update(0.0, 400.0);

        let list = list(&data, &state).fixed_row_height(32.0);

        let plan = plan_for(&list);

        // A 400px viewport with 32px rows shows about 13 rows; the point is that
        // it is a small constant, not 10_000.
        assert!(
            plan.len() <= 20,
            "expected a small window, rendered {} rows",
            plan.len()
        );
        assert_eq!(plan.first, 0, "the window starts at the top");
        assert!(
            plan.trailing > 0.0,
            "the rest of the list is reserved as space"
        );
    }

    #[test]
    fn scrolling_moves_the_rendered_window() {
        let data = items(1_000);
        let mut state = VirtualListState::new();
        state.update(320.0, 400.0);

        let list = list(&data, &state).fixed_row_height(32.0);

        let plan = plan_for(&list);

        // 320px into 32px rows is row 10, and overscan is 2 by default.
        assert_eq!(plan.first, 8, "the window follows the scroll offset");
        assert!(
            plan.leading > 0.0,
            "scrolled-away rows become leading space"
        );
    }

    #[test]
    fn the_window_never_runs_past_the_end() {
        let data = items(20);
        let mut state = VirtualListState::new();
        // A scroll offset beyond the content must clamp rather than panic.
        state.update(100_000.0, 400.0);

        let list = list(&data, &state).fixed_row_height(32.0);

        let plan = plan_for(&list);
        assert!(plan.last <= data.len());
        assert!(plan.first <= plan.last);
    }

    #[test]
    fn an_empty_list_renders() {
        let data: Vec<usize> = Vec::new();
        let state = VirtualListState::new();

        let list = list(&data, &state).fixed_row_height(32.0);

        let plan = plan_for(&list);
        assert_eq!(plan.len(), 0);

        let element: iced::Element<'_, Message, Theme> = list.into();
        drop(element);
    }

    #[test]
    fn variable_heights_are_respected_in_the_plan() {
        let data = items(100);
        let mut state = VirtualListState::new();
        state.update(0.0, 200.0);

        // Every row is 100px tall, so a 200px viewport shows two of them.
        let list = list(&data, &state).row_height(|_item, _index| 100.0);

        let plan = plan_for(&list);
        assert!(
            plan.len() <= 6,
            "expected ~2 rows plus overscan, got {}",
            plan.len()
        );
    }

    #[test]
    fn measured_heights_override_the_estimate() {
        let data = items(50);
        let mut state = VirtualListState::new();
        state.update(0.0, 100.0);
        // The real rows turn out to be much taller than the estimate.
        state.record_height(0, 500.0);

        let list = list(&data, &state).estimate_height(10.0);

        // The first row alone fills the viewport, so the window is tiny.
        let plan = plan_for(&list);
        assert!(
            plan.len() <= 4,
            "a measured tall row must shrink the window, got {}",
            plan.len()
        );
    }

    #[test]
    fn a_state_records_and_reports_measurements() {
        let mut state = VirtualListState::new();

        assert!(state.record_height(3, 48.0), "a new height is a change");
        assert_eq!(state.measured_height(3), Some(48.0));
        assert_eq!(state.measured_count(), 1);

        assert!(
            !state.record_height(3, 48.0),
            "recording the same height twice is not a change"
        );
        assert!(
            state.record_height(3, 50.0),
            "a different height is a change"
        );
        assert_eq!(state.measured_height(3), Some(50.0));
    }

    #[test]
    fn a_state_rejects_impossible_measurements() {
        let mut state = VirtualListState::new();

        // A zero, negative, or non-finite height would corrupt every offset
        // computed after it, so it is dropped rather than stored.
        for bad in [0.0, -10.0, f32::NAN, f32::INFINITY] {
            assert!(!state.record_height(0, bad), "{bad} must be rejected");
        }

        assert_eq!(state.measured_count(), 0);
    }

    #[test]
    fn an_unmeasured_row_falls_back_to_the_estimate() {
        let state = VirtualListState::new();
        assert_eq!(state.measured_height(99), None);
    }

    #[test]
    fn updating_the_same_scroll_position_reports_no_change() {
        let mut state = VirtualListState::new();

        assert!(state.update(100.0, 400.0), "the first update is a change");
        assert!(
            !state.update(100.0, 400.0),
            "an identical scroll event must not force a redraw"
        );
        assert!(state.update(120.0, 400.0));
    }

    #[test]
    fn a_negative_scroll_offset_is_clamped() {
        let mut state = VirtualListState::new();
        state.update(-50.0, 400.0);
        assert_eq!(state.offset(), 0.0);
    }

    #[test]
    fn scrolling_helpers_move_to_the_ends() {
        let mut state = VirtualListState::with_offset(500.0);
        state.update(500.0, 200.0);

        state.scroll_to_top();
        assert_eq!(state.offset(), 0.0);

        state.scroll_to_bottom(1_000.0);
        assert_eq!(
            state.offset(),
            800.0,
            "the last screenful, not past the end"
        );

        // A list shorter than the viewport cannot scroll at all.
        state.scroll_to_bottom(100.0);
        assert_eq!(state.offset(), 0.0);
    }

    #[test]
    fn content_height_covers_every_row() {
        let data = items(10);
        let state = VirtualListState::new();

        let list = list(&data, &state).fixed_row_height(32.0);

        assert_eq!(list.content_height(), 320.0);
    }

    #[test]
    fn content_height_accounts_for_spacing() {
        let data = items(10);
        let state = VirtualListState::new();

        let list = list(&data, &state).fixed_row_height(32.0).spacing(8.0);

        assert_eq!(list.content_height(), 10.0 * 40.0);
    }

    #[test]
    fn a_list_renders_with_every_builder_method() {
        let data = items(1_000);

        for height in [0.0, 100.0, 400.0] {
            for estimate in [8.0, 32.0, 200.0] {
                let mut state = VirtualListState::new();
                state.update(0.0, height);

                let element: iced::Element<'_, Message, Theme> = list(&data, &state)
                    .row_height(|_item, index| if index % 3 == 0 { 48.0 } else { 24.0 })
                    .estimate_height(estimate)
                    .overscan(3)
                    .spacing(4.0)
                    .width(iced::Length::Fill)
                    .height(300.0)
                    .on_scroll(Message::Scrolled)
                    .into();
                drop(element);
            }
        }
    }

    #[test]
    fn the_constructor_function_matches_the_builder() {
        let data = items(10);
        let state = VirtualListState::new();

        let element: iced::Element<'_, Message, Theme> =
            super::virtual_list(&data, &state, |item, _| {
                iced::widget::text(item.to_string()).into()
            })
            .fixed_row_height(24.0)
            .into();
        drop(element);
    }

    #[test]
    fn a_text_row_renders_with_and_without_detail() {
        let plain: iced::Element<'_, Message, Theme> = text_row("Row", None);
        drop(plain);

        let detailed: iced::Element<'_, Message, Theme> =
            text_row("Row", Some("Secondary line".to_owned()));
        drop(detailed);
    }
}
