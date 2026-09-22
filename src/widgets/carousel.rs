//! Carousels.
//!
//! [`carousel`] browses related content a slide at a time in a viewport that
//! snaps. Like the rest of the kit it is stateless: the caller owns which slide
//! is selected and receives a message when that changes.
//!
//! The selection moves four ways — the previous and next controls, the arrow
//! keys, the mouse wheel, and a drag. All four settle on the same snap point and
//! all four report through the same `on_select` callback, so an application sees
//! one kind of change however the user got there.

use std::rc::Rc;
use std::time::Duration;

use crate::icons::IconName;
use crate::theme::catalog::ButtonRounded;
use crate::theme::{Size, Theme};
use crate::widgets::{icon_button, Icon};
use iced::advanced::layout::Layout;
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{tree, Operation};
use iced::advanced::{layout, mouse, renderer, Clipboard, Shell, Widget};
use iced::keyboard::key::{Key, Named};
use iced::time::Instant;
use iced::widget::{button, container, row, Space};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Length, Point, Rectangle,
    Size as IcedSize, Vector,
};

/// Which way a carousel's slides advance.
///
/// The axis also decides which arrow keys navigate: a horizontal carousel uses
/// Left and Right, a vertical one uses Up and Down. The other pair is left alone
/// so it keeps scrolling whatever encloses the carousel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CarouselAxis {
    /// Slides advance from left to right.
    #[default]
    Horizontal,
    /// Slides advance from top to bottom.
    Vertical,
}

impl CarouselAxis {
    /// Returns whether slides advance along the horizontal axis.
    #[must_use]
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal)
    }

    /// Returns the previous and next chevron for this axis.
    fn chevrons(self) -> (IconName, IconName) {
        match self {
            Self::Horizontal => (IconName::ChevronLeft, IconName::ChevronRight),
            Self::Vertical => (IconName::ChevronUp, IconName::ChevronDown),
        }
    }
}

/// The selection a [`carousel`] renders, owned by the application.
///
/// The application keeps this value, hands it back on every frame, and replaces
/// it when [`carousel`] reports a new slide. Keeping the selection outside the
/// widget is what lets the widget stay stateless, and what lets an application
/// drive the carousel from its own state: a "next slide" control elsewhere in
/// the window is another call to [`Self::select_next`].
///
/// Keep the slide count here equal to the number of slides given to
/// [`carousel`]; the selection is expressed in those terms.
///
/// ```
/// # use iced_kit::widgets::carousel::{CarouselAxis, CarouselState};
/// let state = CarouselState::new(4)
///     .with_selected_index(1)
///     .with_axis(CarouselAxis::Vertical)
///     .with_looping(true);
///
/// assert_eq!(state.selected_index(), Some(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "a CarouselState does nothing unless it is given to `carousel`"]
pub struct CarouselState {
    item_count: usize,
    selected_index: Option<usize>,
    axis: CarouselAxis,
    looping: bool,
}

impl CarouselState {
    /// Creates state for `item_count` slides, selecting the first.
    ///
    /// An empty carousel has no selection rather than a selection of zero,
    /// which is what keeps the controls disabled and the arrow keys inert.
    pub fn new(item_count: usize) -> Self {
        Self {
            item_count,
            selected_index: (item_count > 0).then_some(0),
            axis: CarouselAxis::default(),
            looping: false,
        }
    }

    /// Sets the selected slide, clamped to the available range.
    pub fn with_selected_index(mut self, index: usize) -> Self {
        self.selected_index = self.clamp_index(index);
        self
    }

    /// Sets the axis the slides advance along.
    pub fn with_axis(mut self, axis: CarouselAxis) -> Self {
        self.axis = axis;
        self
    }

    /// Enables wrapping from the last slide to the first and back.
    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Returns the number of slides.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Returns the selected slide, or `None` when the carousel is empty.
    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Returns the axis the slides advance along.
    #[must_use]
    pub fn axis(&self) -> CarouselAxis {
        self.axis
    }

    /// Returns whether navigation wraps at the ends.
    #[must_use]
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Returns the slide a step backwards would select.
    ///
    /// This is `None` at the first slide unless looping is on, which is exactly
    /// the condition under which the previous control is disabled.
    #[must_use]
    pub fn previous_index(&self) -> Option<usize> {
        self.navigation_index(false)
    }

    /// Returns the slide a step forwards would select.
    #[must_use]
    pub fn next_index(&self) -> Option<usize> {
        self.navigation_index(true)
    }

    /// Returns whether a previous slide can be selected.
    #[must_use]
    pub fn has_previous(&self) -> bool {
        self.previous_index().is_some()
    }

    /// Returns whether a next slide can be selected.
    #[must_use]
    pub fn has_next(&self) -> bool {
        self.next_index().is_some()
    }

    /// Selects a slide, clamped to the available range.
    pub fn select_index(&mut self, index: usize) {
        self.selected_index = self.clamp_index(index);
    }

    /// Selects the previous slide, wrapping when looping is enabled.
    ///
    /// Returns whether the selection moved.
    pub fn select_previous(&mut self) -> bool {
        match self.previous_index() {
            Some(index) => {
                self.selected_index = Some(index);
                true
            }
            None => false,
        }
    }

    /// Selects the next slide, wrapping when looping is enabled.
    ///
    /// Returns whether the selection moved.
    pub fn select_next(&mut self) -> bool {
        match self.next_index() {
            Some(index) => {
                self.selected_index = Some(index);
                true
            }
            None => false,
        }
    }

    /// Selects the first slide.
    pub fn select_first(&mut self) {
        self.selected_index = self.clamp_index(0);
    }

    /// Selects the last slide.
    pub fn select_last(&mut self) {
        let last = self.item_count.checked_sub(1);
        self.selected_index = last.and_then(|last| self.clamp_index(last));
    }

    /// Changes the number of slides, clamping the selection.
    pub fn set_item_count(&mut self, item_count: usize) {
        self.item_count = item_count;
        self.selected_index = match self.selected_index {
            Some(index) if item_count > 0 => Some(index.min(item_count - 1)),
            Some(_) => None,
            // An empty carousel has no selection to keep, and a non-empty one
            // selects the first slide rather than nothing.
            None => (item_count > 0).then_some(0),
        };
    }

    /// Clamps `index` into range, or returns `None` for an empty carousel.
    fn clamp_index(&self, index: usize) -> Option<usize> {
        (self.item_count > 0).then(|| index.min(self.item_count - 1))
    }

    /// Returns the slide a step in `forward` would select, honouring looping.
    fn navigation_index(&self, forward: bool) -> Option<usize> {
        let current = self.selected_index?;
        let last = self.item_count.checked_sub(1)?;

        if forward {
            if current < last {
                Some(current + 1)
            } else if self.looping {
                Some(0)
            } else {
                None
            }
        } else if current > 0 {
            Some(current - 1)
        } else if self.looping {
            Some(last)
        } else {
            None
        }
    }
}

impl Default for CarouselState {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Builds a carousel over `slides`.
///
/// The viewport snaps to slide boundaries; `per_view` slides are shown at once.
/// The controls are disabled at the ends of the range unless the state enables
/// looping. When the selection changes, `on_select` produces the message that
/// reports it.
///
/// ```
/// # use iced_kit::widgets::carousel::{carousel, CarouselState};
/// # use iced_kit::Theme;
/// # use iced::{Element, Length};
/// # #[derive(Clone, Debug)] enum Message { Went(usize) }
/// # fn view(state: &CarouselState) -> Element<'_, Message, Theme> {
/// carousel(
///     state,
///     vec![
///         iced::widget::text("One").into(),
///         iced::widget::text("Two").into(),
///         iced::widget::text("Three").into(),
///     ],
///     Message::Went,
/// )
/// .height(Length::Fixed(160.0))
/// .into()
/// # }
/// ```
pub fn carousel<'a, Message: Clone + 'a>(
    state: &'a CarouselState,
    slides: Vec<Element<'a, Message, Theme>>,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Carousel<'a, Message> {
    Carousel::new(state, slides, on_select)
}

/// A themed carousel.
#[must_use = "a Carousel does nothing unless it is turned into an Element"]
pub struct Carousel<'a, Message> {
    state: &'a CarouselState,
    slides: Vec<Element<'a, Message, Theme>>,
    on_select: Rc<dyn Fn(usize) -> Message + 'a>,
    width: Length,
    height: Length,
    gap: f32,
    per_view: usize,
    controls: bool,
    indicators: bool,
    size: Size,
    inset: f32,
}

impl<'a, Message: Clone + 'a> Carousel<'a, Message> {
    /// Creates a carousel over `slides`.
    pub fn new(
        state: &'a CarouselState,
        slides: Vec<Element<'a, Message, Theme>>,
        on_select: impl Fn(usize) -> Message + 'a,
    ) -> Self {
        Self {
            state,
            slides,
            on_select: Rc::new(on_select),
            width: Length::Fill,
            height: Length::Fill,
            gap: 16.0,
            per_view: 1,
            controls: true,
            indicators: false,
            size: Size::Md,
            inset: 12.0,
        }
    }

    /// Sets the carousel's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the carousel's height.
    ///
    /// A carousel fills the space it is given, so a fixed height is the usual
    /// way to stop it taking the whole window.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    /// Sets the space between slides.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    /// Sets how many slides the viewport shows at once.
    ///
    /// Whole slides only: snapping to a boundary the user cannot see is worse
    /// than snapping to one they can.
    pub fn per_view(mut self, per_view: usize) -> Self {
        self.per_view = per_view.max(1);
        self
    }

    /// Shows or hides the previous and next controls.
    pub fn controls(mut self, controls: bool) -> Self {
        self.controls = controls;
        self
    }

    /// Shows or hides the dot indicator.
    ///
    /// The indicator is off by default: dots stop being readable past a handful
    /// of slides, and only the caller knows how many to expect.
    pub fn indicators(mut self, indicators: bool) -> Self {
        self.indicators = indicators;
        self
    }

    /// Sets the size of the controls.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets the inset between the controls and the viewport edge.
    pub fn inset(mut self, inset: f32) -> Self {
        self.inset = inset.max(0.0);
        self
    }

    /// Converts the carousel into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let axis = self.state.axis();
        let mut controls = Vec::new();

        if self.controls {
            let (previous, next) = axis.chevrons();

            // Each control reports the slide it leads to, so pressing one is
            // the same transition as an arrow key. The target is resolved here,
            // at build time, because that is the only moment the selection is
            // known; a control with no target is simply never pressable.
            controls.push(control(
                previous,
                self.state.previous_index(),
                self.size,
                &self.on_select,
            ));
            controls.push(control(
                next,
                self.state.next_index(),
                self.size,
                &self.on_select,
            ));
        }

        // The number of slides is taken from the slides themselves, not from the
        // state's item count: a caller whose two disagree must still get a
        // viewport that treats its controls as controls.
        let slide_count = self.slides.len();
        let children = self.slides.into_iter().chain(controls).collect();

        let viewport: Element<'a, Message, Theme> = CarouselViewport {
            state: self.state,
            children,
            slide_count,
            on_select: self.on_select.clone(),
            axis,
            width: self.width,
            height: self.height,
            gap: self.gap,
            per_view: self.per_view,
            size: self.size,
            inset: self.inset,
        }
        .into();

        if !self.indicators {
            return viewport;
        }

        let dots = (0..self.state.item_count()).fold(
            row![].spacing(Size::Xs.gap()).align_y(Alignment::Center),
            |dots, index| {
                let selected = self.state.selected_index() == Some(index);
                dots.push(dot(selected, (self.on_select)(index)))
            },
        );

        iced::widget::column![
            viewport,
            container(dots).width(Length::Fill).center_x(Length::Fill)
        ]
        .spacing(Size::Xs.gap())
        .width(self.width)
        .into()
    }
}

impl<'a, Message: Clone + 'a> From<Carousel<'a, Message>> for Element<'a, Message, Theme> {
    fn from(carousel: Carousel<'a, Message>) -> Self {
        carousel.into_element()
    }
}

/// Builds one previous or next control.
fn control<'a, Message: Clone + 'a>(
    chevron: IconName,
    target: Option<usize>,
    size: Size,
    on_select: &Rc<dyn Fn(usize) -> Message + 'a>,
) -> Element<'a, Message, Theme> {
    // The side is stated rather than left to the layout: a control is laid out
    // in a fixed box of its own size, and a button that still claims fill width
    // inside that box would be drawn stretched across the viewport instead.
    let inset = size.height();

    let control = icon_button()
        .icon(Icon::new(chevron))
        .size(size)
        .outline()
        .width(Length::Fixed(inset))
        .height(Length::Fixed(inset))
        .rounded(ButtonRounded::Custom(inset / 2.0));

    match target {
        Some(target) => control.on_press(on_select(target)).into(),
        None => control.into(),
    }
}

/// Builds one indicator dot.
///
/// The selected dot is drawn larger as well as darker, so the indicator still
/// reads when the theme's primary and border colours are close together.
fn dot<'a, Message: Clone + 'a>(selected: bool, message: Message) -> Element<'a, Message, Theme> {
    let side = if selected { 8.0 } else { 6.0 };

    button(Space::new().width(side).height(side))
        .padding(0)
        .width(Length::Fixed(side))
        .height(Length::Fixed(side))
        .on_press(message)
        .class(Box::new(move |theme: &Theme, _status| button::Style {
            background: Some(Background::Color(if selected {
                theme.colors().primary
            } else {
                theme.colors().border
            })),
            text_color: Color::TRANSPARENT,
            border: Border {
                radius: (side / 2.0).into(),
                ..Border::default()
            },
            shadow: iced::Shadow::default(),
            snap: true,
        }) as button::StyleFn<'a, Theme>)
        .into()
}

/// How far the pointer must move before a press counts as a drag, in logical
/// pixels.
///
/// Below this the press is a click, and a click inside a slide belongs to that
/// slide's own widgets.
const DRAG_THRESHOLD: f32 = 4.0;

/// How quickly a released drag settles onto its snap point, per second.
///
/// A rate rather than a duration, so the easing does not change with the frame
/// rate. It is a plain exponential approach rather than a [`Spring`] because the
/// track is also moved directly by a drag: a spring would need the velocity it
/// was carrying when the pointer took over, and a drag overwrites the offset
/// without knowing it. Approaching by position alone stays correct however a
/// gesture ends.
///
/// [`Spring`]: crate::motion::Spring
const SETTLE_RATE: f32 = 14.0;

/// The drag in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
struct DragState {
    /// Where the pointer was when the drag began, along the carousel's axis.
    origin: f32,
    /// The scroll offset at that moment.
    scroll: f32,
    /// Whether the pointer has moved far enough to count as a drag.
    committed: bool,
}

/// The viewport's own state.
///
/// The scroll offset lives here rather than in [`CarouselState`] because it is
/// presentation: which slide is selected belongs to the application, while how
/// far the track has eased towards it changes on every frame of an animation.
#[derive(Debug, Clone, Copy, Default)]
struct ViewportState {
    /// Where the track is drawn.
    scroll: f32,
    /// Where the track is heading.
    target: f32,
    /// One slide's size along the carousel's axis.
    extent: f32,
    /// The space between slides.
    gap: f32,
    /// The furthest the track may scroll.
    max_scroll: f32,
    /// The drag in progress, if any.
    drag: Option<DragState>,
    /// Whether the key bindings act on this carousel.
    focused: bool,
    /// When the last frame was drawn, for the settle animation.
    last_frame: Option<Instant>,
    /// Whether a layout has ever run, so the first frame places the track
    /// instead of animating it in from zero.
    primed: bool,
}

impl ViewportState {
    /// The distance one slide advances the track by.
    fn stride(&self) -> f32 {
        self.extent + self.gap
    }

    /// The scroll offset that puts `index` at the start of the viewport.
    fn offset_for(&self, index: usize) -> f32 {
        if self.stride() <= 0.0 {
            return 0.0;
        }

        (index as f32 * self.stride()).clamp(0.0, self.max_scroll)
    }

    /// The slide nearest the current scroll offset.
    fn nearest_index(&self, item_count: usize) -> Option<usize> {
        if item_count == 0 {
            return None;
        }

        if self.stride() <= 0.0 {
            return Some(0);
        }

        let index = (self.scroll / self.stride()).round().max(0.0) as usize;
        Some(index.min(item_count - 1))
    }

    /// Moves the track one step towards its target.
    ///
    /// Returns whether the track is still moving, which is what keeps the
    /// widget asking for frames.
    fn settle(&mut self, delta: Duration) -> bool {
        let difference = self.target - self.scroll;

        if difference.abs() < 0.5 {
            let moved = self.scroll != self.target;
            self.scroll = self.target;
            return moved;
        }

        // An exponential approach: the distance left shrinks by the same
        // fraction each second, which eases out smoothly without overshooting
        // and without depending on how often iced redraws. It reads only the
        // position, which is what keeps it correct after a drag has moved the
        // track directly.
        let step = 1.0 - (-SETTLE_RATE * delta.as_secs_f32()).exp();
        self.scroll += difference * step;
        true
    }
}

impl Focusable for ViewportState {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
    }

    fn unfocus(&mut self) {
        self.focused = false;
    }
}

/// The snapping viewport.
///
/// It is a widget of its own rather than a composition of stock ones because
/// nothing in iced scrolls by snapping, and because the track has to be
/// translated while drawing: a layout-time offset would invalidate the layout on
/// every frame of an animation.
///
/// The slides come first in `children` and the controls follow them, which is
/// what `slide_count` marks.
struct CarouselViewport<'a, Message, Renderer = iced::Renderer> {
    state: &'a CarouselState,
    children: Vec<Element<'a, Message, Theme, Renderer>>,
    slide_count: usize,
    on_select: Rc<dyn Fn(usize) -> Message + 'a>,
    axis: CarouselAxis,
    width: Length,
    height: Length,
    gap: f32,
    per_view: usize,
    size: Size,
    inset: f32,
}

impl<Message: Clone, Renderer> CarouselViewport<'_, Message, Renderer> {
    /// The number of leading children that are slides.
    fn slide_count(&self) -> usize {
        self.slide_count.min(self.children.len())
    }

    /// The position of a control inside the viewport.
    ///
    /// A control is a square as tall as its size token, so where it goes follows
    /// from the viewport's own bounds; the layout gives it exactly these bounds.
    fn control_bounds(&self, index: usize, viewport: IcedSize) -> Rectangle {
        let side = self.size.height();
        let centred = |available: f32| ((available - side) / 2.0).max(0.0);

        match (self.axis, index) {
            // The first control goes before the slides, the second after.
            (CarouselAxis::Horizontal, 0) => Rectangle::new(
                Point::new(self.inset, centred(viewport.height)),
                IcedSize::new(side, side),
            ),
            (CarouselAxis::Horizontal, _) => Rectangle::new(
                Point::new(
                    (viewport.width - side - self.inset).max(0.0),
                    centred(viewport.height),
                ),
                IcedSize::new(side, side),
            ),
            (CarouselAxis::Vertical, 0) => Rectangle::new(
                Point::new(centred(viewport.width), self.inset),
                IcedSize::new(side, side),
            ),
            (CarouselAxis::Vertical, _) => Rectangle::new(
                Point::new(
                    centred(viewport.width),
                    (viewport.height - side - self.inset).max(0.0),
                ),
                IcedSize::new(side, side),
            ),
        }
    }

    /// The translation that moves the track's origin onto the viewport's.
    ///
    /// Positive when added to a track coordinate to get a viewport one.
    fn track_translation(&self, scroll: f32) -> Vector {
        match self.axis {
            CarouselAxis::Horizontal => Vector::new(scroll, 0.0),
            CarouselAxis::Vertical => Vector::new(0.0, scroll),
        }
    }

    /// The position of a point along the carousel's axis.
    fn along(&self, point: Point) -> f32 {
        if self.axis.is_horizontal() {
            point.x
        } else {
            point.y
        }
    }
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for CarouselViewport<'_, Message, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<ViewportState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(ViewportState::default())
    }

    fn children(&self) -> Vec<tree::Tree> {
        self.children.iter().map(tree::Tree::new).collect()
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let viewport = limits.width(self.width).height(self.height).resolve(
            self.width,
            self.height,
            IcedSize::ZERO,
        );

        let along = if self.axis.is_horizontal() {
            viewport.width
        } else {
            viewport.height
        };
        let across = if self.axis.is_horizontal() {
            viewport.height
        } else {
            viewport.width
        };

        let slides = self.slide_count();
        let controls = self.children.len() - slides;
        let gaps = self.gap * self.per_view.saturating_sub(1) as f32;
        let extent = ((along - gaps) / self.per_view as f32).max(1.0);
        let content = if slides == 0 {
            0.0
        } else {
            slides as f32 * extent + (slides - 1) as f32 * self.gap
        };

        {
            let state = tree.state.downcast_mut::<ViewportState>();
            state.extent = extent;
            state.gap = self.gap;
            state.max_scroll = (content - along).max(0.0);
            state.target = self
                .state
                .selected_index()
                .map_or(0.0, |index| state.offset_for(index));

            if state.primed {
                state.scroll = state.scroll.clamp(0.0, state.max_scroll);
            } else {
                state.scroll = state.target;
                state.primed = true;
            }
        }

        // Every slide is laid out at exactly one slide's box, so a slide cannot
        // grow the track by asking for more room than it was given, and every
        // snap point is the same distance from the next.
        let slide_limits =
            layout::Limits::new(IcedSize::new(extent, across), IcedSize::new(extent, across));

        let mut nodes: Vec<layout::Node> = Vec::with_capacity(self.children.len());

        for index in 0..slides {
            let node = self.children[index].as_widget_mut().layout(
                &mut tree.children[index],
                renderer,
                &slide_limits,
            );
            let offset = index as f32 * (extent + self.gap);

            nodes.push(if self.axis.is_horizontal() {
                node.move_to(Point::new(offset, 0.0))
            } else {
                node.move_to(Point::new(0.0, offset))
            });
        }

        for index in 0..controls {
            let bounds = self.control_bounds(index, viewport);
            let node = self.children[slides + index].as_widget_mut().layout(
                &mut tree.children[slides + index],
                renderer,
                &layout::Limits::new(bounds.size(), bounds.size()),
            );

            nodes.push(node.move_to(bounds.position()));
        }

        layout::Node::with_children(viewport, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        // Advertising the viewport as focusable is what puts it in the tab
        // order, so the arrow keys work without the application focusing it.
        operation.focusable(
            None,
            layout.bounds(),
            tree.state.downcast_mut::<ViewportState>(),
        );

        // Indexed rather than iterated: every child is visited, and a shared
        // iterator would advance past the end of the set.
        let children: Vec<Layout<'_>> = layout.children().collect();

        for index in 0..self.children.len() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };

            self.children[index].as_widget_mut().operate(
                &mut tree.children[index],
                child_layout,
                renderer,
                operation,
            );
        }
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if self.handle_event(tree, event, bounds, cursor, shell) {
            return;
        }

        let slides = self.slide_count();
        let scroll = tree.state.downcast_ref::<ViewportState>().scroll;

        // Slides hit-test in the track's own coordinates, so the cursor is moved
        // onto them rather than the other way round: their layout is not
        // re-run every frame, and a control inside a scrolled slide still
        // receives the events meant for it. Outside the viewport nothing is
        // reachable, so a slide that has scrolled out of sight cannot answer for
        // what is drawn over it.
        let over = cursor.is_over(bounds);
        let track_cursor = if over {
            cursor + self.track_translation(scroll)
        } else {
            mouse::Cursor::Unavailable
        };
        // The clip a slide is offered is the viewport expressed in the track's
        // own coordinates, which is what keeps a slide that has scrolled out of
        // sight from claiming events aimed at whatever is drawn over it.
        let track_clip = Rectangle::new(
            bounds.position() + self.track_translation(scroll),
            bounds.size(),
        );

        // Collected once, because the slides and the controls each need a
        // different cursor and clip and so cannot share one pass.
        let children: Vec<Layout<'_>> = layout.children().collect();

        // Visited back to front, because the controls are drawn over the slides:
        // the topmost widget is the one a click belongs to, so it has to be
        // offered the event before anything underneath can consume it.
        for index in (0..self.children.len()).rev() {
            let Some(child_layout) = children.get(index).copied() else {
                continue;
            };

            let (child_cursor, child_clip) = if index < slides {
                (track_cursor, &track_clip)
            } else {
                (cursor, viewport)
            };

            self.children[index].as_widget_mut().update(
                &mut tree.children[index],
                event,
                child_layout,
                child_cursor,
                renderer,
                clipboard,
                shell,
                child_clip,
            );

            if shell.is_event_captured() {
                return;
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let slides = self.slide_count();
        let scroll = tree.state.downcast_ref::<ViewportState>().scroll;
        let over = cursor.is_over(layout.bounds());
        let track_cursor = if over {
            cursor + self.track_translation(scroll)
        } else {
            mouse::Cursor::Unavailable
        };
        let children: Vec<Layout<'_>> = layout.children().collect();

        // Back to front, for the same reason as `update`: the controls are drawn
        // over the slides, so the control under the pointer is the one that names
        // the cursor.
        for index in (0..self.children.len()).rev() {
            let Some(child_layout) = children.get(index).copied() else {
                continue;
            };

            let child_cursor = if index < slides { track_cursor } else { cursor };

            let interaction = self.children[index].as_widget().mouse_interaction(
                &tree.children[index],
                child_layout,
                child_cursor,
                viewport,
                renderer,
            );

            if interaction != mouse::Interaction::None {
                return interaction;
            }
        }

        if !over {
            return mouse::Interaction::None;
        }

        if tree
            .state
            .downcast_ref::<ViewportState>()
            .drag
            .is_some_and(|drag| drag.committed)
        {
            mouse::Interaction::Grabbing
        } else {
            mouse::Interaction::Grab
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !bounds.intersects(viewport) {
            return;
        }

        let slides = self.slide_count();
        let scroll = tree.state.downcast_ref::<ViewportState>().scroll;
        // Collected once: the slides and the controls are drawn in two passes,
        // and an iterator shared between them would run off the end before the
        // second pass began.
        let children: Vec<Layout<'_>> = layout.children().collect();

        // The slides are drawn translated, so their cursor is offset the other
        // way for the same reason as in `update`.
        let track_cursor = cursor + self.track_translation(scroll);

        // The layer is the clip: everything the track draws outside the
        // viewport is cut off, which is what makes a slide appear at the edge
        // rather than spill over the content beside it.
        renderer.with_layer(bounds, |renderer| {
            renderer.with_translation(-self.track_translation(scroll), |renderer| {
                for (index, child) in self.children[..slides].iter().enumerate() {
                    let Some(child_layout) = children.get(index).copied() else {
                        break;
                    };

                    child.as_widget().draw(
                        &tree.children[index],
                        renderer,
                        theme,
                        style,
                        child_layout,
                        track_cursor,
                        viewport,
                    );
                }
            });
        });

        // The controls are drawn after the layer is closed, so the clip that
        // keeps the track inside the viewport does not also hide them.
        for (index, child) in self.children[slides..].iter().enumerate() {
            let position = slides + index;
            let Some(child_layout) = children.get(position).copied() else {
                break;
            };

            child.as_widget().draw(
                &tree.children[position],
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        // A menu or tooltip inside a slide is drawn above the clip the carousel
        // puts around its track, so the children are asked for their overlays
        // here; leaving this to the default implementation would drop them.
        iced::advanced::overlay::from_children(
            &mut self.children,
            tree,
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<Message: Clone, Renderer> CarouselViewport<'_, Message, Renderer> {
    /// Handles the input the viewport owns itself.
    ///
    /// Returns whether the event was consumed, in which case nothing is passed
    /// on to the slides: a drag that moved the track must not also press
    /// whatever it began on.
    fn handle_event(
        &self,
        tree: &mut tree::Tree,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let slides = self.slide_count();
        let state = tree.state.downcast_mut::<ViewportState>();

        match event {
            Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) if state.focused => {
                let target = match (self.axis, key) {
                    (CarouselAxis::Horizontal, Key::Named(Named::ArrowLeft))
                    | (CarouselAxis::Vertical, Key::Named(Named::ArrowUp)) => {
                        self.state.previous_index()
                    }
                    (CarouselAxis::Horizontal, Key::Named(Named::ArrowRight))
                    | (CarouselAxis::Vertical, Key::Named(Named::ArrowDown)) => {
                        self.state.next_index()
                    }
                    (_, Key::Named(Named::Home)) => (slides > 0).then_some(0),
                    (_, Key::Named(Named::End)) => slides.checked_sub(1),
                    _ => None,
                };

                if let Some(index) = target {
                    shell.publish((self.on_select)(index));
                    shell.capture_event();
                    return true;
                }

                false
            }
            // A wheel notch steps one slide. A gesture at the edge is left for
            // an enclosing scrollable, so a carousel inside a page does not
            // trap the wheel once it has nothing left to show.
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let notches = match delta {
                    mouse::ScrollDelta::Lines { x, y } => {
                        if self.axis.is_horizontal() {
                            f64::from(*x) + f64::from(*y)
                        } else {
                            f64::from(*y)
                        }
                    }
                    // A pixel delta arrives continuously from a trackpad, and
                    // treating each one as a step would race through the slides.
                    mouse::ScrollDelta::Pixels { .. } => 0.0,
                };

                let target = if notches < 0.0 {
                    self.state.next_index()
                } else if notches > 0.0 {
                    self.state.previous_index()
                } else {
                    None
                };

                if let Some(index) = target {
                    shell.publish((self.on_select)(index));
                    shell.capture_event();
                    return true;
                }

                false
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                // Focus follows the click, which is what makes the arrow keys
                // work on a carousel the user has just touched.
                state.focused = true;

                if let Some(position) = cursor.position_over(bounds) {
                    state.drag = Some(DragState {
                        origin: self.along(position),
                        scroll: state.scroll,
                        committed: false,
                    });
                }

                false
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let Some(drag) = state.drag else {
                    return false;
                };

                if drag.committed {
                    // The drag may have begun over a slide, in which case the
                    // position is in the track's coordinates.
                    let delta = self.along(*position) - drag.origin;
                    state.scroll = (drag.scroll - delta).clamp(0.0, state.max_scroll);
                    shell.request_redraw();
                    return true;
                }

                let delta = self.along(*position) - drag.origin;

                if delta.abs() < DRAG_THRESHOLD {
                    // Not yet a drag. Leaving the track alone here is what keeps
                    // a click inside a slide available to the slide.
                    return false;
                }

                state.drag = Some(DragState {
                    committed: true,
                    ..drag
                });
                state.scroll = (drag.scroll - delta).clamp(0.0, state.max_scroll);
                shell.request_redraw();

                true
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let Some(drag) = state.drag.take() else {
                    return false;
                };

                if !drag.committed {
                    return false;
                }

                // A release settles on the nearest snap point rather than where
                // the pointer happened to stop, so a half-finished flick ends
                // somewhere the user can name.
                if let Some(index) = state.nearest_index(slides) {
                    shell.publish((self.on_select)(index));
                }

                shell.capture_event();
                true
            }
            Event::Window(iced::window::Event::RedrawRequested(now)) => {
                let previous = state.last_frame.replace(*now);
                let delta = previous.map_or(Duration::ZERO, |last| now.duration_since(last));

                // A track under the pointer is where the pointer put it; pulling
                // it back towards the selected slide mid-drag would fight the
                // hand doing the dragging.
                let dragging = state.drag.is_some_and(|drag| drag.committed);

                if !dragging && state.settle(delta) {
                    shell.request_redraw();
                }

                false
            }
            Event::Window(iced::window::Event::Unfocused) => {
                state.focused = false;
                false
            }
            _ => false,
        }
    }
}

impl<'a, Message, Renderer> From<CarouselViewport<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(viewport: CarouselViewport<'a, Message, Renderer>) -> Self {
        Element::new(viewport)
    }
}

#[cfg(test)]
mod tests {
    use super::{carousel, CarouselAxis, CarouselState, ViewportState};
    use crate::icons::IconName;
    use crate::theme::Theme;
    use std::time::Duration;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Went(usize),
    }

    #[test]
    fn a_carousel_renders() {
        let state = CarouselState::new(3);
        let element: iced::Element<'_, Message, Theme> = carousel(
            &state,
            (0..3)
                .map(|index| iced::widget::text(format!("Slide {index}")).into())
                .collect(),
            Message::Went,
        )
        .into();

        drop(element);
    }

    /// An empty carousel must not divide by a slide count of zero or panic.
    #[test]
    fn an_empty_carousel_renders() {
        let state = CarouselState::new(0);
        let element: iced::Element<'_, Message, Theme> =
            carousel(&state, Vec::new(), Message::Went).into();

        drop(element);
    }

    /// State and slides must be able to disagree without the render panicking,
    /// and the controls must stay controls rather than being counted as slides.
    #[test]
    fn a_state_larger_than_the_slides_renders() {
        let state = CarouselState::new(9).with_selected_index(8);
        let element: iced::Element<'_, Message, Theme> = carousel(
            &state,
            vec![iced::widget::text("One").into()],
            Message::Went,
        )
        .controls(true)
        .into();

        drop(element);
    }

    #[test]
    fn an_empty_carousel_has_no_navigation() {
        let state = CarouselState::new(0);

        assert_eq!(state.selected_index(), None);
        assert_eq!(state.previous_index(), None);
        assert_eq!(state.next_index(), None);
        assert!(!state.has_previous());
        assert!(!state.has_next());
    }

    #[test]
    fn the_first_slide_has_no_previous_and_the_last_has_no_next() {
        let mut state = CarouselState::new(3);

        assert_eq!(state.selected_index(), Some(0));
        assert!(!state.has_previous());
        assert_eq!(state.next_index(), Some(1));

        state.select_last();

        assert_eq!(state.selected_index(), Some(2));
        assert_eq!(state.previous_index(), Some(1));
        assert!(!state.has_next());
    }

    #[test]
    fn looping_wraps_in_both_directions() {
        let mut state = CarouselState::new(3).with_looping(true);

        assert_eq!(state.previous_index(), Some(2));

        state.select_last();
        assert_eq!(state.next_index(), Some(0));

        assert!(state.select_next(), "looping past the end must move");
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn stepping_stops_at_the_ends_without_looping() {
        let mut state = CarouselState::new(2);

        assert!(
            !state.select_previous(),
            "there is nothing before the first"
        );
        assert_eq!(state.selected_index(), Some(0));

        assert!(state.select_next());
        assert_eq!(state.selected_index(), Some(1));
        assert!(!state.select_next(), "there is nothing after the last");
    }

    #[test]
    fn a_selection_out_of_range_is_clamped() {
        let state = CarouselState::new(3).with_selected_index(99);
        assert_eq!(state.selected_index(), Some(2));

        let mut state = CarouselState::new(3);
        state.select_index(99);
        assert_eq!(state.selected_index(), Some(2));
    }

    /// Shrinking the set must not leave the selection pointing past the end.
    #[test]
    fn shrinking_the_set_clamps_the_selection() {
        let mut state = CarouselState::new(5);
        state.select_last();
        assert_eq!(state.selected_index(), Some(4));

        state.set_item_count(2);
        assert_eq!(state.selected_index(), Some(1));

        state.set_item_count(0);
        assert_eq!(state.selected_index(), None);

        state.set_item_count(3);
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn the_axis_defaults_to_horizontal() {
        assert_eq!(CarouselState::new(1).axis(), CarouselAxis::Horizontal);
        assert!(CarouselAxis::Horizontal.is_horizontal());
        assert!(!CarouselAxis::Vertical.is_horizontal());
    }

    #[test]
    fn a_vertical_carousel_renders() {
        let state = CarouselState::new(2).with_axis(CarouselAxis::Vertical);
        let element: iced::Element<'_, Message, Theme> = carousel(
            &state,
            (0..2)
                .map(|index| iced::widget::text(format!("Slide {index}")).into())
                .collect(),
            Message::Went,
        )
        .height(iced::Length::Fixed(200.0))
        .into();

        drop(element);
    }

    #[test]
    fn a_carousel_renders_with_its_controls_hidden() {
        let state = CarouselState::new(3);
        let element: iced::Element<'_, Message, Theme> = carousel(
            &state,
            vec![iced::widget::text("One").into()],
            Message::Went,
        )
        .controls(false)
        .into();

        drop(element);
    }

    #[test]
    fn a_carousel_renders_its_indicators() {
        let state = CarouselState::new(4).with_selected_index(2);
        let element: iced::Element<'_, Message, Theme> = carousel(
            &state,
            (0..4)
                .map(|index| iced::widget::text(format!("Slide {index}")).into())
                .collect(),
            Message::Went,
        )
        .indicators(true)
        .into();

        drop(element);
    }

    #[test]
    fn a_carousel_shows_more_than_one_slide_at_a_time() {
        let state = CarouselState::new(6);
        let element: iced::Element<'_, Message, Theme> = carousel(
            &state,
            (0..6)
                .map(|index| iced::widget::text(format!("Slide {index}")).into())
                .collect(),
            Message::Went,
        )
        .per_view(3)
        .into();

        drop(element);
    }

    /// Every knob at once, since they share the geometry that decides the snap
    /// points.
    #[test]
    fn a_carousel_renders_with_an_axis_and_looping_and_indicators() {
        let state = CarouselState::new(4)
            .with_axis(CarouselAxis::Vertical)
            .with_selected_index(3)
            .with_looping(true);

        let element: iced::Element<'_, Message, Theme> = carousel(
            &state,
            (0..4)
                .map(|index| iced::widget::text(format!("Slide {index}")).into())
                .collect(),
            Message::Went,
        )
        .per_view(2)
        .gap(4.0)
        .inset(0.0)
        .size(crate::theme::Size::Lg)
        .indicators(true)
        .height(iced::Length::Fixed(180.0))
        .into();

        drop(element);
    }

    /// A scroll offset lands on the slide whose boundary it is nearest.
    #[test]
    fn a_scroll_offset_settles_on_the_nearest_slide() {
        let measured = ViewportState {
            extent: 100.0,
            gap: 0.0,
            max_scroll: 300.0,
            ..ViewportState::default()
        };

        for (scroll, expected) in [(0.0, 0), (49.0, 0), (51.0, 1), (199.0, 2)] {
            let state = ViewportState { scroll, ..measured };

            assert_eq!(
                state.nearest_index(4),
                Some(expected),
                "a scroll of {scroll} must settle on slide {expected}"
            );
        }
    }

    #[test]
    fn the_gap_is_part_of_the_distance_between_snap_points() {
        let state = ViewportState {
            extent: 100.0,
            gap: 20.0,
            max_scroll: 480.0,
            ..ViewportState::default()
        };

        assert_eq!(state.offset_for(0), 0.0);
        assert_eq!(state.offset_for(2), 240.0);
    }

    /// The track must not scroll past its own content.
    #[test]
    fn a_scroll_offset_is_clamped_to_the_content() {
        let state = ViewportState {
            extent: 100.0,
            gap: 0.0,
            max_scroll: 200.0,
            ..ViewportState::default()
        };

        assert_eq!(state.offset_for(9), 200.0);
    }

    /// A carousel of one slide, or none, has nothing to scroll.
    #[test]
    fn a_degenerate_viewport_does_not_move() {
        let state = ViewportState {
            extent: 100.0,
            gap: 0.0,
            max_scroll: 0.0,
            ..ViewportState::default()
        };

        assert_eq!(state.offset_for(5), 0.0);
        assert_eq!(state.nearest_index(1), Some(0));
        assert_eq!(
            state.nearest_index(0),
            None,
            "an empty carousel has no slides"
        );

        let unmeasured = ViewportState::default();
        assert_eq!(unmeasured.offset_for(3), 0.0);
        assert_eq!(unmeasured.nearest_index(3), Some(0));
    }

    /// The settle animation must reach its target and then stop asking for
    /// frames, or a carousel at rest would redraw the window forever.
    #[test]
    fn the_track_settles_and_then_stops() {
        let mut state = ViewportState {
            target: 100.0,
            ..ViewportState::default()
        };

        let frame = Duration::from_millis(16);
        let mut frames = 0;

        while state.settle(frame) {
            frames += 1;
            assert!(frames < 500, "the track must settle within a few seconds");
        }

        assert_eq!(
            state.scroll, state.target,
            "the track must reach its target"
        );
        assert!(
            !state.settle(frame),
            "a settled track must stop requesting frames"
        );
    }

    #[test]
    fn a_settled_track_is_drawn_where_it_already_was() {
        let mut state = ViewportState {
            scroll: 40.0,
            target: 40.0,
            ..ViewportState::default()
        };

        assert!(!state.settle(Duration::from_millis(16)));
        assert_eq!(state.scroll, 40.0);
    }

    /// The chevrons must point the way the slides move. They are compared by
    /// the glyph they resolve to, because the icon font's enum has no
    /// `PartialEq` of its own.
    #[test]
    fn the_axis_chooses_the_chevrons() {
        let glyph = |icon: IconName| crate::icons::glyph(icon);

        let (previous, next) = CarouselAxis::Horizontal.chevrons();
        assert_eq!(glyph(previous), glyph(IconName::ChevronLeft));
        assert_eq!(glyph(next), glyph(IconName::ChevronRight));

        let (previous, next) = CarouselAxis::Vertical.chevrons();
        assert_eq!(glyph(previous), glyph(IconName::ChevronUp));
        assert_eq!(glyph(next), glyph(IconName::ChevronDown));
    }

    #[test]
    fn a_state_defaults_to_an_empty_horizontal_carousel() {
        let state = CarouselState::default();

        assert_eq!(state.item_count(), 0);
        assert_eq!(state.selected_index(), None);
        assert_eq!(state.axis(), CarouselAxis::Horizontal);
        assert!(!state.is_looping());
    }

    /// A single-slide carousel has nothing to navigate to.
    #[test]
    fn a_single_slide_has_nowhere_to_go() {
        let state = CarouselState::new(1);

        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.previous_index(), None);
        assert_eq!(state.next_index(), None);

        let looping = CarouselState::new(1).with_looping(true);
        assert_eq!(
            looping.next_index(),
            Some(0),
            "looping around a single slide stays on it"
        );
    }

    #[test]
    fn the_dots_resolve_in_both_themes() {
        for theme in [Theme::light(), Theme::dark()] {
            let _ = theme.colors().primary;
            let _ = theme.colors().border;
        }
    }
}
