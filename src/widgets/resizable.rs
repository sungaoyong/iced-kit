//! Resizable split panes.
//!
//! iced's [`pane_grid`] already provides the hard part — draggable splitters
//! with drag, resize and drop handling. This module supplies the design tokens
//! for the splitter chrome and a smaller API for the common case: a number of
//! panes separated by draggable dividers, with no drag-to-rearrange.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use iced::widget::pane_grid::{self, Axis, Configuration, Direction, Pane, Split};
use iced::{Background, Color, Element};

pub use iced::widget::pane_grid::{DragEvent, Pane as PaneId, ResizeEvent, State, Target};

/// Renders the content of one pane.
pub type PaneView<'a, Message> = Box<dyn Fn(Pane, &usize) -> Element<'a, Message, Theme> + 'a>;

/// How the panes are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitAxis {
    /// Panes sit side by side, divided by vertical splitters.
    #[default]
    Horizontal,
    /// Panes stack, divided by horizontal splitters.
    Vertical,
}

impl From<SplitAxis> for Axis {
    fn from(axis: SplitAxis) -> Self {
        match axis {
            SplitAxis::Horizontal => Self::Vertical,
            SplitAxis::Vertical => Self::Horizontal,
        }
    }
}

/// A group of panes separated by draggable splitters.
///
/// # Usage
///
/// ```
/// use iced::widget::text;
/// use iced_kit::widgets::resizable::{Resizable, ResizeEvent, SplitAxis, State};
/// use iced_kit::Theme;
/// use iced::Element;
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     Resized(ResizeEvent),
/// }
///
/// struct App {
///     panes: State<usize>,
/// }
///
/// fn view(app: &App) -> Element<'_, Message, Theme> {
///     Resizable::new(&app.panes)
///         .axis(SplitAxis::Horizontal)
///         // The handler receives the resize event and produces a message.
///         .on_resize(Message::Resized)
///         .pane(|_pane, index: &usize| text(format!("Pane {index}")).into())
///         .into_element()
/// }
/// ```
#[must_use = "a Resizable does nothing unless it is turned into an Element"]
pub struct Resizable<'a, Message> {
    state: &'a State<usize>,
    axis: SplitAxis,
    split: Option<Split>,
    ratio: f32,
    spacing: f32,
    min_size: f32,
    on_resize: Option<Box<dyn Fn(ResizeEvent) -> Message + 'a>>,
    on_drag: Option<Box<dyn Fn(DragEvent) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> Resizable<'a, Message> {
    /// Creates a split group over the given pane state.
    pub fn new(state: &'a State<usize>) -> Self {
        Self {
            state,
            axis: SplitAxis::default(),
            split: None,
            ratio: 0.5,
            spacing: 6.0,
            min_size: min_pane_size(),
            on_resize: None,
            on_drag: None,
        }
    }

    /// Creates a state for `count` panes along `axis`, divided at `ratio`.
    ///
    /// This is the common case: an application that only needs a splitter
    /// between a sidebar and a body does not have to build a layout tree.
    ///
    /// Each split takes `ratio` of the remaining space, so a ratio of 0.3 with
    /// three panes gives a narrow first pane and two even ones.
    #[must_use]
    pub fn split_state(count: usize, axis: SplitAxis, ratio: f32) -> (State<usize>, Vec<Pane>) {
        let count = count.max(1);
        let ratio = ratio.clamp(0.05, 0.95);

        // A `Configuration` tree carries the split ratios; `State::split` takes
        // the new pane's payload instead and cannot express a ratio.
        let configuration = nested_config(count, axis.into(), ratio, 0);
        let state = State::with_configuration(configuration);

        let panes = state.iter().map(|(pane, _)| *pane).collect();

        (state, panes)
    }

    /// Creates a state from an explicit layout tree.
    ///
    /// Use this when the panes need a nested arrangement rather than a single
    /// chain of splits.
    pub fn from_configuration(config: impl Into<Configuration<usize>>) -> State<usize> {
        State::with_configuration(config)
    }

    /// Sets the split axis.
    pub fn axis(mut self, axis: SplitAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Sets which split a drag applies to, when the layout has more than one.
    pub fn split(mut self, split: Split) -> Self {
        self.split = Some(split);
        self
    }

    /// Sets the ratio the split is restored to, for a layout shown without a
    /// stored state.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.clamp(0.05, 0.95);
        self
    }

    /// Sets the gap between panes.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.max(0.0);
        self
    }

    /// Sets the smallest a pane may be dragged to.
    ///
    /// The pane count times this value must fit the container, or the layout is
    /// over-constrained: iced clamps the panes it cannot fit and one may be
    /// pushed out of view. Lower this for a small split area.
    pub fn min_size(mut self, min_size: f32) -> Self {
        self.min_size = min_size.max(1.0);
        self
    }

    /// Reports a completed resize.
    pub fn on_resize(mut self, f: impl Fn(ResizeEvent) -> Message + 'a) -> Self {
        self.on_resize = Some(Box::new(f));
        self
    }

    /// Reports a pane drag, when panes are rearrangeable.
    pub fn on_drag(mut self, f: impl Fn(DragEvent) -> Message + 'a) -> Self {
        self.on_drag = Some(Box::new(f));
        self
    }

    /// Supplies the content of each pane, keyed by its index.
    pub fn pane(
        self,
        view: impl Fn(Pane, &usize) -> Element<'a, Message, Theme> + 'a,
    ) -> PaneGrid<'a, Message> {
        PaneGrid {
            inner: self,
            view: Box::new(view),
        }
    }
}

/// A [`Resizable`] with its pane content, ready to become an [`Element`].
#[must_use = "a PaneGrid does nothing unless it is turned into an Element"]
pub struct PaneGrid<'a, Message> {
    inner: Resizable<'a, Message>,
    view: PaneView<'a, Message>,
}

impl<'a, Message: Clone + 'a> PaneGrid<'a, Message> {
    /// Converts the grid into an [`Element`].
    #[must_use]
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self { inner, view } = self;

        let Resizable {
            state,
            axis: _axis,
            split: _split,
            ratio: _ratio,
            spacing,
            min_size,
            on_resize,
            on_drag,
        } = inner;

        // `pane_grid` hands the view closure the pane id, its data, and whether
        // it is currently maximized; this wrapper exposes the first two.
        let mut grid = iced::widget::pane_grid(state, move |pane, index: &usize, _maximized| {
            view(pane, index).into()
        })
        .spacing(spacing)
        .min_size(min_size)
        .style(splitter_style);

        // A drag is only reported when the application asked for it, so a plain
        // split layout stays non-rearrangeable.
        if let Some(on_drag) = on_drag {
            grid = grid.on_drag(on_drag);
        }

        if let Some(on_resize) = on_resize {
            // A generous grab area, so a thin splitter is still easy to catch.
            grid = grid.on_resize(6.0, on_resize);
        }

        grid.into()
    }
}

impl<'a, Message: Clone + 'a> From<PaneGrid<'a, Message>> for Element<'a, Message, Theme> {
    fn from(grid: PaneGrid<'a, Message>) -> Self {
        grid.into_element()
    }
}

/// Builds a nested split configuration for `count` panes.
///
/// The first pane is split off at `ratio` of the whole, and the remainder is
/// split again, which yields a chain of splitters rather than a lopsided tree.
fn nested_config(
    count: usize,
    axis: iced::widget::pane_grid::Axis,
    ratio: f32,
    index: usize,
) -> Configuration<usize> {
    if count <= 1 {
        return Configuration::Pane(index);
    }

    Configuration::Split {
        axis,
        ratio,
        a: Box::new(Configuration::Pane(index)),
        b: Box::new(nested_config(count - 1, axis, ratio, index + 1)),
    }
}

/// The appearance of splitters between panes.
///
/// An idle splitter is invisible: a visible line between every pane would make
/// a dense layout look busy. It appears on hover and while dragging, which is
/// when the affordance matters.
fn splitter_style(theme: &Theme) -> pane_grid::Style {
    let colors = theme.colors();
    let tone = Tone::Primary.accent(theme);

    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: Background::Color(Color { a: 0.15, ..tone }),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: f32::from(theme.radius().sm).into(),
            },
        },
        picked_split: pane_grid::Line {
            color: tone,
            width: 2.0,
        },
        hovered_split: pane_grid::Line {
            color: colors.ring,
            width: 2.0,
        },
    }
}

/// The width of a splitter's visible line, in logical pixels.
#[must_use]
pub fn splitter_width() -> f32 {
    2.0
}

/// The smallest a pane may be dragged to, in logical pixels.
///
/// Below roughly this width a pane's own controls stop fitting, so a drag that
/// would go further is stopped. Note that the pane count times this value has to
/// fit the split area; see [`Resizable::min_size`].
#[must_use]
pub fn min_pane_size() -> f32 {
    120.0
}

/// A convenience constructor for a two-pane split.
///
/// Returns the state and both pane ids, which is all a sidebar-plus-body layout
/// needs.
#[must_use]
pub fn split_pair(ratio: f32) -> (State<usize>, [Pane; 2]) {
    let (state, panes) = Resizable::<()>::split_state(2, SplitAxis::Horizontal, ratio);

    (state, [panes[0], panes[1]])
}

/// Whether a direction is one of the four the pane grid recognises.
#[must_use]
pub fn is_direction(direction: Direction, axis: SplitAxis) -> bool {
    match axis {
        SplitAxis::Horizontal => matches!(direction, Direction::Left | Direction::Right),
        SplitAxis::Vertical => matches!(direction, Direction::Up | Direction::Down),
    }
}

#[cfg(test)]
mod tests {
    use super::{split_pair, splitter_style, splitter_width, Resizable, SplitAxis};
    use crate::theme::Theme;
    use iced::widget::pane_grid::{Configuration, Direction, Node, Split, State};

    /// Finds the first split in a layout tree, for asserting on the structure.
    fn first_split(node: &Node) -> Option<Split> {
        match node {
            Node::Split { id, .. } => Some(*id),
            Node::Pane(_) => None,
        }
    }

    /// The handlers only need to produce a message; the event payloads are
    /// never inspected, so they are discarded.
    #[derive(Debug, Clone)]
    enum Message {
        Resized,
        Dragged,
    }

    /// Adapts a resize event into [`Message`].
    fn on_resize(_event: iced::widget::pane_grid::ResizeEvent) -> Message {
        Message::Resized
    }

    /// Adapts a drag event into [`Message`].
    fn on_drag(_event: iced::widget::pane_grid::DragEvent) -> Message {
        Message::Dragged
    }

    #[test]
    fn a_two_pane_split_state_is_created() {
        let (state, panes) = Resizable::<Message>::split_state(2, SplitAxis::Horizontal, 0.3);

        assert_eq!(state.len(), 2);
        assert_eq!(panes.len(), 2);
        assert_ne!(panes[0], panes[1]);
    }

    #[test]
    fn a_single_pane_split_state_has_no_splitter() {
        let (state, panes) = Resizable::<Message>::split_state(1, SplitAxis::Horizontal, 0.5);

        assert_eq!(state.len(), 1);
        assert_eq!(panes.len(), 1);
        assert!(first_split(state.layout()).is_none(), "nothing to drag");
    }

    #[test]
    fn a_zero_pane_request_still_produces_one_pane() {
        // A grid with no panes would render nothing and look broken.
        let (state, panes) = Resizable::<Message>::split_state(0, SplitAxis::Vertical, 0.5);

        assert_eq!(state.len(), 1);
        assert_eq!(panes.len(), 1);
    }

    #[test]
    fn splitting_many_panes_produces_one_splitter_per_seam() {
        let (state, panes) = Resizable::<Message>::split_state(4, SplitAxis::Vertical, 0.5);

        assert_eq!(state.len(), 4);
        assert_eq!(panes.len(), 4);
        assert!(first_split(state.layout()).is_some());
    }

    #[test]
    fn every_pane_carries_a_distinct_payload_in_order() {
        // A nested configuration that reused an index would silently render one
        // pane's content twice and leave another empty.
        for count in 1..=6 {
            let (state, _) = Resizable::<Message>::split_state(count, SplitAxis::Vertical, 0.4);

            let mut payloads: Vec<usize> = state.iter().map(|(_, data)| *data).collect();
            payloads.sort_unstable();

            let expected: Vec<usize> = (0..count).collect();
            assert_eq!(payloads, expected, "count = {count}");
        }
    }

    #[test]
    fn a_nested_layout_never_nests_deeper_than_its_pane_count() {
        // The tree is built by splitting off one pane at a time, so a layout of
        // `count` panes has exactly `count - 1` splitters.
        for count in 1..=5 {
            let (state, _) = Resizable::<Message>::split_state(count, SplitAxis::Horizontal, 0.5);

            assert_eq!(count_splits(state.layout()), count - 1, "count = {count}");
        }
    }

    /// Counts the split nodes in a layout tree.
    fn count_splits(node: &Node) -> usize {
        match node {
            Node::Split { a, b, .. } => 1 + count_splits(a) + count_splits(b),
            Node::Pane(_) => 0,
        }
    }

    #[test]
    fn an_out_of_range_ratio_is_clamped() {
        // A ratio of 0 or 1 would collapse a pane to nothing and make the
        // splitter impossible to grab again.
        for ratio in [-1.0, 0.0, 1.0, 2.0, f32::NAN] {
            let (state, _) = Resizable::<Message>::split_state(2, SplitAxis::Horizontal, ratio);
            assert_eq!(state.len(), 2, "ratio {ratio} must still build a grid");
        }
    }

    #[test]
    fn the_split_pair_helper_returns_both_panes() {
        let (state, panes) = split_pair(0.25);

        assert_eq!(state.len(), 2);
        assert_ne!(panes[0], panes[1]);
    }

    #[test]
    fn a_state_can_be_built_from_a_configuration() {
        let config: Configuration<usize> = Configuration::Split {
            axis: iced::widget::pane_grid::Axis::Vertical,
            ratio: 0.4,
            a: Box::new(Configuration::Pane(0)),
            b: Box::new(Configuration::Pane(1)),
        };

        let state = Resizable::<Message>::from_configuration(config);
        assert_eq!(state.len(), 2);
    }

    #[test]
    fn a_grid_renders_with_handlers() {
        let (state, _) = Resizable::<Message>::split_state(2, SplitAxis::Horizontal, 0.3);

        let element: iced::Element<'_, Message, Theme> = Resizable::new(&state)
            .axis(SplitAxis::Horizontal)
            .ratio(0.35)
            .spacing(8.0)
            .min_size(100.0)
            .on_resize(on_resize)
            .on_drag(on_drag)
            .pane(|_pane, index: &usize| iced::widget::text(format!("Pane {index}")).into())
            .into_element();

        drop(element);
    }

    #[test]
    fn a_grid_renders_without_handlers() {
        let (state, _) = Resizable::<Message>::split_state(3, SplitAxis::Vertical, 0.5);

        let element: iced::Element<'_, Message, Theme> = Resizable::new(&state)
            .pane(|_pane, index: &usize| iced::widget::text(format!("{index}")).into())
            .into_element();

        drop(element);
    }

    #[test]
    fn a_single_pane_grid_renders() {
        let (state, _) = Resizable::<Message>::split_state(1, SplitAxis::Horizontal, 0.5);

        let element: iced::Element<'_, Message, Theme> = Resizable::new(&state)
            .on_resize(on_resize)
            .pane(|_pane, _index| iced::widget::text("Only").into())
            .into_element();

        drop(element);
    }

    #[test]
    fn a_restored_state_renders() {
        let state = State::<usize>::with_configuration(Configuration::Pane(0));

        let element: iced::Element<'_, Message, Theme> = Resizable::new(&state)
            .pane(|_pane, _index| iced::widget::text("Restored").into())
            .into_element();

        drop(element);
    }

    #[test]
    fn an_idle_splitter_is_invisible_and_reacts_on_hover() {
        let theme = Theme::light();
        let style = splitter_style(&theme);

        assert_eq!(style.hovered_split.width, splitter_width());
        assert_ne!(style.hovered_split.color, style.picked_split.color);
        // The hover highlight is a translucent wash, not an opaque fill.
        match style.hovered_region.background {
            iced::Background::Color(color) => assert_eq!(color.a, 0.15),
            iced::Background::Gradient(_) => {
                panic!("the hover highlight must be a solid color at low alpha")
            }
        }
    }

    #[test]
    fn split_axes_map_onto_iced_axes() {
        use iced::widget::pane_grid::Axis;

        // A horizontal split means panes side by side, which iced models as a
        // vertical axis.
        assert_eq!(Axis::from(SplitAxis::Horizontal), Axis::Vertical);
        assert_eq!(Axis::from(SplitAxis::Vertical), Axis::Horizontal);
    }

    #[test]
    fn direction_checks_match_the_axis() {
        assert!(super::is_direction(Direction::Left, SplitAxis::Horizontal));
        assert!(super::is_direction(Direction::Right, SplitAxis::Horizontal));
        assert!(!super::is_direction(Direction::Up, SplitAxis::Horizontal));

        assert!(super::is_direction(Direction::Up, SplitAxis::Vertical));
        assert!(!super::is_direction(Direction::Left, SplitAxis::Vertical));
    }

    #[test]
    fn a_layout_node_reports_its_first_split() {
        let (state, _) = Resizable::<Message>::split_state(2, SplitAxis::Horizontal, 0.5);

        match state.layout() {
            Node::Split { .. } => assert!(first_split(state.layout()).is_some()),
            Node::Pane(_) => panic!("two panes must produce a split"),
        }
    }
}
