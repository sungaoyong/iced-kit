//! Tabs.
//!
//! [`tabs`] renders a horizontal strip of selectable labels. It is stateless:
//! the caller owns the selection and receives a message when it changes.
//!
//! The selected tab is marked by an underline that slides between tabs rather
//! than jumping, which is what shows the selection as one thing moving through
//! the strip instead of two unrelated states.

use crate::theme::{Size, Theme};
use iced::advanced::widget::{tree, Operation, Widget};
use iced::advanced::{layout, mouse, renderer, Clipboard, Shell};
use iced::time::Instant;
use iced::widget::{button, row, text};
use iced::{Color, Element, Event, Length, Padding, Rectangle, Size as IcedSize};

/// The thickness of the underline marking the selected tab, in logical pixels.
const INDICATOR_THICKNESS: f32 = 2.0;

/// One tab in a [`tabs`] strip.
#[derive(Debug, Clone)]
#[must_use = "a Tab does nothing unless it is given to `tabs`"]
pub struct Tab {
    label: String,
    enabled: bool,
}

impl Tab {
    /// Creates an enabled tab with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
        }
    }

    /// Enables or disables the tab.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns the tab's label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Builds a horizontal tab strip.
///
/// `on_select` receives the index of the tab that was clicked. A disabled tab
/// never produces a message.
///
/// ```
/// # use iced_kit::widgets::{tabs, Tab};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message { Selected(usize) }
/// # fn view(current: usize) -> Element<'static, Message, Theme> {
/// tabs(
///     vec![Tab::new("General"), Tab::new("Advanced")],
///     current,
///     Message::Selected,
/// )
/// .into()
/// # }
/// ```
pub fn tabs<'a, Message: Clone + 'a>(
    tabs: Vec<Tab>,
    selected: usize,
    on_select: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message, Theme> {
    let text_style = Size::Md.text();
    let height = Size::Md.height() + 4.0;
    let count = tabs.len();

    let strip =
        tabs.into_iter()
            .enumerate()
            .fold(row![].spacing(0), |row, (index, tab)| {
                let is_selected = index == selected;

                // The line box is the tab's own height, not the text's. A raw
                // iced button lays its content out at its padding origin without
                // centring it, so a text-height line box leaves the label against
                // the top of the tab. Sizing the box to the control is what puts
                // the baseline where the eye expects it.
                let label = text(tab.label)
                    .size(text_style.size)
                    .line_height(iced::Pixels(height.max(text_style.line_height)));

                let mut widget = button(label)
                    .padding(Padding {
                        top: 0.0,
                        right: 12.0,
                        bottom: 0.0,
                        left: 12.0,
                    })
                    .height(Length::Fixed(height))
                    .class(Box::new(move |theme: &Theme, status| {
                        tab_style(theme, status, is_selected)
                    }) as button::StyleFn<'a, Theme>);

                if tab.enabled {
                    widget = widget.on_press(on_select(index));
                }

                row.push(widget)
            });

    let strip: Element<'a, Message, Theme> = strip.into();

    // A stale index selects nothing, so the indicator is simply absent rather
    // than pointing at a tab that is not there.
    let selected = (selected < count).then_some(selected);

    TabStrip::new(strip, selected, INDICATOR_THICKNESS).into()
}

/// A row of tabs with a sliding underline.
///
/// It wraps the row rather than being one, so the indicator can be drawn on top
/// of the tabs at a position interpolated between them. iced has nothing that
/// draws an underline under a flex item, and the position is only knowable once
/// the row has been laid out.
struct TabStrip<'a, Message, Renderer = iced::Renderer> {
    row: Element<'a, Message, Theme, Renderer>,
    /// The selected tab, or `None` when nothing is selected.
    selected: Option<usize>,
    thickness: f32,
}

impl<'a, Message, Renderer> TabStrip<'a, Message, Renderer> {
    fn new(
        row: Element<'a, Message, Theme, Renderer>,
        selected: Option<usize>,
        thickness: f32,
    ) -> Self {
        Self {
            row,
            selected,
            thickness,
        }
    }
}

/// The indicator's state between frames.
#[derive(Debug, Clone, Copy, Default)]
struct StripState {
    /// The indicator's left edge and width, each spring-driven so the underline
    /// glides between tabs. Two springs rather than one because tabs differ in
    /// width: a single value would make the underline jump in size.
    left: crate::motion::SpringState,
    width: crate::motion::SpringState,
    /// The frame the springs were last advanced to.
    last: Option<Instant>,
    /// Whether the indicator has ever been placed, so the first frame does not
    /// slide in from the left edge.
    primed: bool,
}

impl<Message, Renderer> Widget<Message, Theme, Renderer> for TabStrip<'_, Message, Renderer>
where
    Message: Clone,
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<StripState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(StripState::default())
    }

    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.row)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(std::slice::from_ref(&self.row));
    }

    fn size(&self) -> IcedSize<Length> {
        self.row.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.row
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
        self.row
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
        // The tabs themselves take the events; the strip only draws over them.
        self.row.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        // Where the underline belongs is only knowable from the laid-out tabs,
        // because how wide a label ends up depends on the font.
        let Some(target) = self
            .selected
            .and_then(|selected| selected_tab_bounds(layout, selected))
        else {
            return;
        };

        let state = tree.state.downcast_mut::<StripState>();

        if !state.primed {
            // The first frame places the underline instead of sliding it in
            // from the left edge, which would read as the strip selecting
            // itself on startup.
            state.left.set(target.x);
            state.width.set(target.width);
            state.primed = true;
        }

        // Every frame steps towards the selected tab rather than only the frame
        // the selection changed on. A spring is retargeted by being told its
        // target again, so this is also what carries a half-finished slide
        // through a second click: the underline is caught in mid-flight and
        // turned around instead of restarting.
        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let elapsed = state
                .last
                .map_or(std::time::Duration::ZERO, |last| now.duration_since(last));
            state.last = Some(*now);

            // A reduced-motion application places the underline on its tab
            // rather than gliding to it.
            let spring = if crate::motion::reduce_motion() {
                crate::motion::Spring::new(std::time::Duration::ZERO)
            } else {
                crate::motion::Spring::new(crate::motion::DURATION_NORMAL).with_epsilon(0.05)
            };

            state.left.step(target.x, spring, elapsed);
            state.width.step(target.width, spring, elapsed);

            if !state.left.is_settled(target.x, spring)
                || !state.width.is_settled(target.width, spring)
            {
                shell.request_redraw();
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.row.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
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

        if !bounds.intersects(viewport) {
            return;
        }

        self.row.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );

        let state = tree.state.downcast_ref::<StripState>();

        // Nothing has been selected yet, so there is no underline to draw.
        if !state.primed {
            return;
        }

        let width = state.width.value().max(0.0);
        if width <= 0.0 {
            return;
        }

        let indicator = Rectangle {
            x: state.left.value(),
            y: bounds.y + bounds.height - self.thickness,
            width,
            height: self.thickness,
        };

        renderer.fill_quad(
            renderer::Quad {
                bounds: indicator,
                border: iced::Border {
                    // Squared ends, so the underline reads as a rule under the
                    // label rather than as a pill.
                    radius: f32::from(theme.radius().sm).into(),
                    ..iced::Border::default()
                },
                shadow: iced::Shadow::default(),
                // Not snapped: the indicator spends its time between tabs, and
                // pixel-snapping a moving quad makes it stutter.
                snap: false,
            },
            theme.colors().primary,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.row.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Renderer> From<TabStrip<'a, Message, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(strip: TabStrip<'a, Message, Renderer>) -> Self {
        Element::new(strip)
    }
}

/// The laid-out bounds of the tab at `selected`.
///
/// The strip's `layout` returns the row's own node rather than a wrapper around
/// it, so `layout`'s children are the tabs themselves and the index is the tab
/// index. Descending into a tab first — which this once did — looks for the
/// `selected`th child of a single tab, which does not exist; the lookup then
/// fails, the underline is never placed, and the strip silently stops
/// following the selection.
///
/// It is a free function so it can be tested against a real laid-out row.
fn selected_tab_bounds(layout: layout::Layout<'_>, selected: usize) -> Option<Rectangle> {
    layout.children().nth(selected).map(|tab| tab.bounds())
}

/// The appearance of a single tab.
fn tab_style(theme: &Theme, status: button::Status, is_selected: bool) -> button::Style {
    let colors = theme.colors();
    let hovered = matches!(status, button::Status::Hovered);
    let disabled = matches!(status, button::Status::Disabled);

    let text_color = if is_selected {
        colors.foreground
    } else if disabled {
        // A disabled tab is dimmer than an ordinary unselected one, so it does
        // not read as merely available.
        iced::Color {
            a: colors.muted_foreground.a * 0.6,
            ..colors.muted_foreground
        }
    } else {
        colors.muted_foreground
    };

    button::Style {
        // A disabled tab must not highlight on hover.
        background: (hovered && !disabled).then_some(iced::Background::Color(colors.accent)),
        text_color,
        // The underline is drawn by the strip above the row rather than as a
        // border here, because it slides between tabs and so cannot belong to
        // one of them.
        border: iced::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{tabs, Tab};
    use crate::theme::Theme;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Selected(usize),
    }

    #[test]
    fn a_tab_strip_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(
            vec![Tab::new("One"), Tab::new("Two"), Tab::new("Three")],
            0,
            Message::Selected,
        );
        drop(element);
    }

    #[test]
    fn a_disabled_tab_still_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(
            vec![Tab::new("Enabled"), Tab::new("Disabled").enabled(false)],
            0,
            Message::Selected,
        );
        drop(element);
    }

    #[test]
    fn an_empty_strip_renders() {
        let element: iced::Element<'_, Message, Theme> = tabs(vec![], 0, Message::Selected);
        drop(element);
    }

    #[test]
    fn a_selection_index_beyond_the_strip_renders_no_selection() {
        // A stale index must not panic; it simply selects nothing.
        let element: iced::Element<'_, Message, Theme> =
            tabs(vec![Tab::new("Only")], 99, Message::Selected);
        drop(element);
    }

    /// The selected tab is distinguished by its text color; the underline that
    /// makes it a tab rather than a button is drawn by the strip.
    #[test]
    fn tab_style_marks_the_selected_tab() {
        use iced::widget::button::Status;

        let theme = Theme::light();
        let selected = super::tab_style(&theme, Status::Active, true);
        let unselected = super::tab_style(&theme, Status::Active, false);

        assert_eq!(selected.text_color, theme.colors().foreground);
        assert_eq!(unselected.text_color, theme.colors().muted_foreground);
        assert_ne!(selected.text_color, unselected.text_color);
    }

    /// A tab must not draw an underline of its own, or there would be two: the
    /// strip draws the one that slides.
    #[test]
    fn no_tab_draws_its_own_underline() {
        use iced::widget::button::Status;

        let theme = Theme::light();

        for is_selected in [true, false] {
            for status in [Status::Active, Status::Hovered, Status::Disabled] {
                let style = super::tab_style(&theme, status, is_selected);

                assert_eq!(
                    style.border.width, 0.0,
                    "the strip owns the underline, not the tab"
                );
            }
        }
    }

    /// A fresh strip must place the underline rather than slide it in from the
    /// left edge, which would read as the strip selecting itself on startup.
    #[test]
    fn a_fresh_indicator_is_not_yet_placed() {
        let state = super::StripState::default();

        assert!(!state.primed);
        assert_eq!(state.left.value(), 0.0);
        assert_eq!(state.width.value(), 0.0);
    }

    /// The underline must target the tab at the selected index, and its bounds
    /// must be that tab's — this is the lookup that decides where the underline
    /// is drawn.
    ///
    /// This is a regression test. The lookup once descended into the first tab
    /// before indexing, looking for the `selected`th child of a single tab
    /// rather than the `selected`th tab. Every index but zero then failed to
    /// resolve, so the underline stopped following the selection — and because
    /// the failure also stopped the springs being stepped, the underline froze
    /// where it was rather than disappearing, which is what made it look like
    /// the *first* tab was still selected.
    #[test]
    fn the_underline_targets_the_selected_tab() {
        use iced::advanced::layout;
        use iced::{Point, Rectangle, Size};

        // Three tabs of different widths, as a real strip's are: a tab is as
        // wide as its label plus its padding.
        let tabs = [
            Rectangle::new(Point::new(0.0, 0.0), Size::new(60.0, 28.0)),
            Rectangle::new(Point::new(60.0, 0.0), Size::new(80.0, 28.0)),
            Rectangle::new(Point::new(140.0, 0.0), Size::new(70.0, 28.0)),
        ];

        let children: Vec<layout::Node> = tabs
            .iter()
            .map(|tab| layout::Node::new(tab.size()).move_to(tab.position()))
            .collect();

        // The row's own node, which is what the strip's `layout` returns.
        let node = layout::Node::with_children(Size::new(210.0, 28.0), children);
        let root = layout::Layout::new(&node);

        for (index, expected) in tabs.iter().enumerate() {
            assert_eq!(
                super::selected_tab_bounds(root, index),
                Some(*expected),
                "the underline must sit under tab {index}, \
                 or it stops following the selection"
            );
        }

        assert_eq!(
            super::selected_tab_bounds(root, tabs.len()),
            None,
            "an index past the last tab selects nothing"
        );
    }

    /// A strip with no tabs, or a stale selection, has no tab to mark, so the
    /// strip reports no selection rather than panicking on the missing one.
    #[test]
    fn a_strip_with_no_selectable_tab_marks_nothing() {
        let empty: iced::Element<'_, Message, Theme> = tabs(vec![], 0, Message::Selected);
        drop(empty);

        let stale: iced::Element<'_, Message, Theme> =
            tabs(vec![Tab::new("Only")], 99, Message::Selected);
        drop(stale);
    }

    #[test]
    fn tab_labels_are_preserved() {
        let tab = Tab::new("General");
        assert_eq!(tab.label(), "General");
        assert!(tab.enabled);
    }
}
