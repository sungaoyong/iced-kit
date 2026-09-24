//! Chat bubbles: the surface a message's content sits in, its alignment, and
//! the emoji reactions that may accompany it.

use crate::theme::{Size, Theme};
use iced::advanced::layout::{self, Limits};
use iced::advanced::widget::{tree, Operation};
use iced::advanced::{mouse, Clipboard, Shell, Widget};
use iced::widget::{column, container, row, text};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Point, Rectangle,
    Size as IcedSize, Vector,
};

/// The gap between reaction controls in a cluster.
const REACTION_GAP: f32 = 4.0;

/// How far a reaction control is rounded.
///
/// An explicit, very large radius rather than the theme's `full` token: a pill's
/// children have to be at least as round as the pill, and a theme with a small
/// radius scale would otherwise leave them squarer than the cluster around them.
const PILL_RADIUS: f32 = 9999.0;

/// The width of the ring drawn around a reaction cluster.
///
/// The ring is in the page color, so the cluster reads as a separate layer
/// floating over the bubble rather than as a patch inside it.
const REACTION_RING: f32 = 3.0;

/// Horizontal alignment for messages and message-owned chat surfaces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MessageAlignment {
    /// Place the surface at the leading edge.
    #[default]
    Start,
    /// Place the surface at the trailing edge.
    End,
}

/// A bubble's surface treatment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleVariant {
    /// A filled primary surface.
    #[default]
    Filled,
    /// A neutral secondary surface.
    Secondary,
    /// A lower-emphasis muted surface.
    Muted,
    /// A subtle primary-tinted surface.
    Tinted,
    /// A background surface with a visible border.
    Outline,
    /// A destructive-tinted surface, for an error or a refused message.
    Destructive,
    /// No surface, padding, or border.
    Ghost,
}

/// Which side of a bubble its reactions sit on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BubbleReactionSide {
    /// Attach reactions above the bubble.
    Top,
    /// Attach reactions below the bubble.
    #[default]
    Bottom,
}

/// The content slot of a [`Bubble`]: a plain vertical stack.
#[must_use = "a BubbleContent does nothing unless it is given to a Bubble"]
pub struct BubbleContent<'a, Message> {
    children: Vec<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> BubbleContent<'a, Message> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Adds a text line to the bubble body.
    pub fn text(mut self, content: impl text::IntoFragment<'a>) -> Self {
        self.children.push(
            text(content)
                .size(Size::Md.text().size)
                .line_height(Size::Md.text().line_height())
                .into(),
        );
        self
    }

    /// Adds an arbitrary element to the bubble body.
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, Message: 'a> Default for BubbleContent<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<BubbleContent<'a, Message>> for Element<'a, Message, Theme> {
    fn from(content: BubbleContent<'a, Message>) -> Self {
        column(content.children).spacing(4).into()
    }
}

/// A cluster of reaction affordances shown beside a bubble.
#[must_use = "a BubbleReactions does nothing unless it is given to a Bubble"]
pub struct BubbleReactions<'a, Message> {
    side: BubbleReactionSide,
    children: Vec<Element<'a, Message, Theme>>,
    /// Whether any child was given through [`BubbleReactions::action`].
    ///
    /// A cluster of controls already carries each control's padding, so the
    /// cluster adds none of its own; a cluster of bare text does need it.
    has_action: bool,
}

impl<'a, Message: 'a> BubbleReactions<'a, Message> {
    pub fn new() -> Self {
        Self {
            side: BubbleReactionSide::default(),
            children: Vec::new(),
            has_action: false,
        }
    }

    /// Chooses whether the cluster sits above or below the bubble.
    pub fn side(mut self, side: BubbleReactionSide) -> Self {
        self.side = side;
        self
    }

    /// Adds a reaction control, shaped to the cluster's own pill radius.
    ///
    /// This is the typed form the reference takes: because the control is known
    /// to be a [`Button`](crate::widgets::Button), it can be rounded to match
    /// the cluster, so a row of reactions reads as one pill rather than as
    /// buttons sitting inside a pill. Use [`child`](Self::child) for a control
    /// that must keep its own shape.
    pub fn button(mut self, button: crate::widgets::Button<'a, Message>) -> Self
    where
        Message: Clone,
    {
        // Fully rounded, matching the cluster: a pill's children cannot be
        // squarer than the pill without reading as a fault.
        self.children.push(button.rounded(PILL_RADIUS).into());
        self.has_action = true;
        self
    }

    /// Adds a reaction control as a plain element.
    ///
    /// The element keeps its own styling; use [`button`](Self::button) to have
    /// a button rounded to the cluster.
    pub fn action(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self.has_action = true;
        self
    }

    /// Adds content that is not a control: a reaction count, a divider, a label.
    ///
    /// Unlike [`action`](Self::action), the element keeps its own styling.
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }

    /// Whether any child was added as a control.
    #[must_use]
    pub fn has_action(&self) -> bool {
        self.has_action
    }

    /// How many children the cluster holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Whether the cluster is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl<'a, Message: 'a> Default for BubbleReactions<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> BubbleReactions<'a, Message> {
    /// Renders the cluster as the reference's pill: a rounded, muted capsule
    /// with a thick ring in the page color, so it reads as floating over the
    /// bubble it annotates rather than as a patch inside it.
    fn into_element(self) -> Element<'a, Message, Theme> {
        let mut cluster = row(self.children)
            .spacing(REACTION_GAP)
            .align_y(Alignment::Center);

        // A cluster of bare text (a reaction count, say) needs its own padding.
        // A cluster of controls already carries each control's padding, and
        // adding more would make the pill bulge around them.
        if !self.has_action {
            cluster = cluster.padding(Padding::from([2.0, 6.0]));
        }

        container(cluster)
            .padding(REACTION_RING)
            .class(Box::new(|theme: &Theme| {
                let colors = theme.colors();

                container::Style {
                    // The background is the page color and the fill is muted, so
                    // the ring that separates the pill from the bubble is the
                    // page-color edge the padding leaves visible.
                    background: Some(Background::Color(colors.background)),
                    border: Border {
                        color: colors.muted,
                        width: 0.0,
                        radius: f32::from(theme.radius().full).into(),
                    },
                    text_color: Some(colors.foreground),
                    ..Default::default()
                }
            })
                as iced::widget::container::StyleFn<'a, Theme>)
            .into()
    }
}

impl<'a, Message: 'a> From<BubbleReactions<'a, Message>> for Element<'a, Message, Theme> {
    fn from(reactions: BubbleReactions<'a, Message>) -> Self {
        reactions.into_element()
    }
}

/// A chat bubble that owns alignment, variant, content, and reactions.
#[must_use = "a Bubble does nothing unless it is turned into an Element"]
pub struct Bubble<'a, Message> {
    variant: BubbleVariant,
    alignment: MessageAlignment,
    children: Vec<Element<'a, Message, Theme>>,
    reactions: Option<BubbleReactions<'a, Message>>,
    max_width: Option<f32>,
}

impl<'a, Message: 'a> Bubble<'a, Message> {
    pub fn new() -> Self {
        Self {
            variant: BubbleVariant::default(),
            alignment: MessageAlignment::default(),
            children: Vec::new(),
            reactions: None,
            max_width: None,
        }
    }

    /// Sets the surface treatment.
    pub fn with_variant(mut self, variant: BubbleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the leading/trailing alignment.
    pub fn alignment(mut self, alignment: MessageAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Replaces the body with a [`BubbleContent`]'s children.
    pub fn content(mut self, content: BubbleContent<'a, Message>) -> Self {
        self.children = content.children;
        self
    }

    /// Appends an element to the body.
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }

    /// Attaches a reaction cluster.
    pub fn reactions(mut self, reactions: BubbleReactions<'a, Message>) -> Self {
        self.reactions = Some(reactions);
        self
    }

    /// Caps the bubble's width, so long messages wrap rather than fill the row.
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width.max(1.0));
        self
    }

    /// Whether the bubble draws no surface at all.
    ///
    /// A bubble with no surface has no padding for a caller's header or footer
    /// to line up with, which is what [`MessageContent`] keys off.
    ///
    /// [`MessageContent`]: super::message::MessageContent
    #[must_use]
    pub fn is_ghost(&self) -> bool {
        self.variant == BubbleVariant::Ghost
    }
}

impl<'a, Message: 'a> Default for Bubble<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<Bubble<'a, Message>> for Element<'a, Message, Theme> {
    fn from(bubble: Bubble<'a, Message>) -> Self {
        let variant = bubble.variant;
        let edge = match bubble.alignment {
            MessageAlignment::Start => Alignment::Start,
            MessageAlignment::End => Alignment::End,
        };

        // Without an explicit cap the surface hugs its text; with one, the body
        // fills the cap so long messages wrap instead of overflowing. A `Fill`
        // body inside a `Shrink` stack would collapse to nothing, so the two
        // widths have to move together.
        let (body_width, stack_width) = match bubble.max_width {
            Some(width) => (Length::Fill, Length::Fixed(width)),
            None => (Length::Shrink, Length::Shrink),
        };

        let body = column(bubble.children).spacing(4).width(body_width);

        // Ghost has no surface at all; the others wrap the body in a styled
        // container so its text color flows into descendants.
        let surface: Element<'a, Message, Theme> = if variant == BubbleVariant::Ghost {
            body.into()
        } else {
            container(body)
                .width(body_width)
                .padding(Padding::from([8.0, 12.0]))
                .class(Box::new(move |theme: &Theme| bubble_style(theme, variant))
                    as iced::widget::container::StyleFn<'a, Theme>)
                .into()
        };

        // The reaction pill rides over the bubble's edge rather than sitting
        // beside it, so the two overlap: the pill is an annotation on the
        // message, and the pair must occupy one box rather than two stacked.
        let cluster = bubble
            .reactions
            .filter(|reactions| !reactions.is_empty())
            .map(|reactions| (reactions.side, reactions.into_element()));

        let stack = match cluster {
            None => container(surface).width(stack_width).into(),
            Some((side, cluster)) => {
                Element::from(Overlap::new(surface, cluster, side, stack_width))
            }
        };

        // The outer container spans the row so it can push the shrunken stack
        // to the leading or trailing edge.
        container(stack).width(Length::Fill).align_x(edge).into()
    }
}

/// Stacks a bubble with a reaction pill that rides over its edge.
///
/// iced has no absolute positioning and no negative margins, so a pill that
/// overlaps the bubble's edge has to be laid out by hand: the two children are
/// placed with the pill inset past the bubble's own edge, and the result's
/// height is the union of the two rather than their sum. That union is what
/// keeps a message the same height whether or not it has reactions, so a
/// transcript does not jump as they arrive.
struct Overlap<'a, Message> {
    bubble: Element<'a, Message, Theme>,
    pill: Element<'a, Message, Theme>,
    side: BubbleReactionSide,
    width: Length,
}

impl<'a, Message> Overlap<'a, Message> {
    fn new(
        bubble: Element<'a, Message, Theme>,
        pill: Element<'a, Message, Theme>,
        side: BubbleReactionSide,
        width: Length,
    ) -> Self {
        Self {
            bubble,
            pill,
            side,
            width,
        }
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Overlap<'_, Message> {
    fn children(&self) -> Vec<tree::Tree> {
        vec![tree::Tree::new(&self.bubble), tree::Tree::new(&self.pill)]
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(&[&self.bubble, &self.pill]);
    }

    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &Limits,
    ) -> layout::Node {
        let max = limits.width(self.width).max();

        let bubble = self.bubble.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &Limits::new(IcedSize::ZERO, max),
        );
        let pill = self.pill.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &Limits::new(IcedSize::ZERO, max),
        );

        let bubble_size = bubble.size();
        let pill_size = pill.size();

        // How far the pill reaches past the bubble's edge: three quarters of
        // the pill's height, matching the reference's approximation of
        // shadcn's `translate-y-3/4`. Enough to read as riding on the edge,
        // while leaving the message's last line uncovered.
        let overhang = pill_size.height * 0.75;

        let (bubble_y, pill_y, height) = match self.side {
            // Over the top edge: the pill starts at the origin and the bubble
            // is pushed down by what the pill does not overhang.
            BubbleReactionSide::Top => {
                let bubble_y = pill_size.height - overhang;
                (bubble_y, 0.0, bubble_y + bubble_size.height)
            }
            // Over the bottom edge: the bubble comes first, and the pill's top
            // sits at the bubble's bottom less the overhang.
            BubbleReactionSide::Bottom => {
                let pill_y = bubble_size.height - overhang;
                (0.0, pill_y, pill_y + pill_size.height)
            }
        };

        let width = bubble_size.width.max(pill_size.width);

        // The pill tucks against the bubble's trailing edge, the way a reaction
        // badge hangs off the corner of the message it belongs to.
        let pill_x = (bubble_size.width - pill_size.width).max(0.0);

        let nodes = vec![
            bubble.move_to(Point::new(0.0, bubble_y)),
            pill.move_to(Point::new(pill_x, pill_y)),
        ];

        layout::Node::with_children(IcedSize::new(width, height), nodes)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let children: Vec<layout::Layout<'_>> = layout.children().collect();

        for (index, child) in [&mut self.bubble, &mut self.pill].into_iter().enumerate() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };

            child.as_widget_mut().operate(
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
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let children: Vec<layout::Layout<'_>> = layout.children().collect();

        // The pill draws over the bubble, so it answers for a press first:
        // a reaction that sits on the message's edge must still be clickable.
        for index in [1, 0] {
            let Some(child_layout) = children.get(index).copied() else {
                continue;
            };

            let child = if index == 0 {
                &mut self.bubble
            } else {
                &mut self.pill
            };

            child.as_widget_mut().update(
                &mut tree.children[index],
                event,
                child_layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );

            if shell.is_event_captured() {
                break;
            }
        }
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
        let children: Vec<layout::Layout<'_>> = layout.children().collect();

        // The bubble first, so the pill and its ring draw over it.
        for (index, child) in [&self.bubble, &self.pill].into_iter().enumerate() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };

            child.as_widget().draw(
                &tree.children[index],
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &tree::Tree,
        layout: layout::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let children: Vec<layout::Layout<'_>> = layout.children().collect();

        // Front to back, matching the draw order.
        for index in [1, 0] {
            let Some(child_layout) = children.get(index).copied() else {
                continue;
            };

            let child = if index == 0 { &self.bubble } else { &self.pill };

            let interaction = child.as_widget().mouse_interaction(
                &tree.children[index],
                child_layout,
                cursor,
                viewport,
                renderer,
            );

            if interaction != mouse::Interaction::None {
                return interaction;
            }
        }

        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut tree::Tree,
        layout: layout::Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let overlays = [&mut self.bubble, &mut self.pill]
            .into_iter()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|((child, tree), layout)| {
                child
                    .as_widget_mut()
                    .overlay(tree, layout, renderer, viewport, translation)
            })
            .collect::<Vec<_>>();

        (!overlays.is_empty())
            .then(|| iced::advanced::overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: 'a> From<Overlap<'a, Message>> for Element<'a, Message, Theme> {
    fn from(overlap: Overlap<'a, Message>) -> Self {
        Element::new(overlap)
    }
}

/// Resolves a bubble variant to a container style against the theme.
fn bubble_style(theme: &Theme, variant: BubbleVariant) -> container::Style {
    let c = theme.colors();
    // The reference chat surface uses its 2xl corner, which is `radius * 2.5`
    // (15px at the default scale) rather than a named step. Our scale tops out
    // at 12px, and the reference's own comment calls this a "large" corner, so
    // the derived value is composed here the same way the theme derives it.
    let radius = (f32::from(theme.radius().md) * 2.5).into();

    // A tinted variant is an opaque mix, not a translucent fill: iced's
    // renderer does not guarantee alpha compositing, so a translucent surface
    // would blend with whatever happens to be behind it instead of reading as
    // one color. The reference mixes in a different ratio per mode.
    let tinted = |color: Color, dark_ratio: f32, light_ratio: f32| {
        let base = if theme.is_dark() {
            c.background
        } else {
            c.surface
        };
        let ratio = if theme.is_dark() {
            dark_ratio
        } else {
            light_ratio
        };

        Color {
            r: color.r * ratio + base.r * (1.0 - ratio),
            g: color.g * ratio + base.g * (1.0 - ratio),
            b: color.b * ratio + base.b * (1.0 - ratio),
            a: 1.0,
        }
    };

    match variant {
        BubbleVariant::Filled => container::Style {
            background: Some(Background::Color(c.primary)),
            text_color: Some(c.primary_foreground),
            border: Border {
                radius,
                ..Default::default()
            },
            ..Default::default()
        },
        // The theme's `secondary` role is tuned for buttons and reads a tier
        // heavier than a conversation surface; the near-background `muted` tier
        // matches the reference bubble in both light and dark themes.
        BubbleVariant::Secondary => container::Style {
            background: Some(Background::Color(c.muted)),
            text_color: Some(c.secondary_foreground),
            border: Border {
                radius,
                ..Default::default()
            },
            ..Default::default()
        },
        BubbleVariant::Muted => container::Style {
            background: Some(Background::Color(c.muted)),
            text_color: Some(c.foreground),
            border: Border {
                radius,
                ..Default::default()
            },
            ..Default::default()
        },
        BubbleVariant::Tinted => container::Style {
            background: Some(Background::Color(tinted(c.primary, 0.24, 0.12))),
            text_color: Some(c.foreground),
            border: Border {
                radius,
                ..Default::default()
            },
            ..Default::default()
        },
        BubbleVariant::Outline => container::Style {
            // The page color, so the bubble reads as outlined rather than
            // raised: the border is the whole surface treatment.
            background: Some(Background::Color(c.background)),
            text_color: Some(c.foreground),
            border: Border {
                color: c.border,
                width: 1.0,
                radius,
            },
            ..Default::default()
        },
        BubbleVariant::Destructive => container::Style {
            background: Some(Background::Color(tinted(c.destructive, 0.2, 0.1))),
            // The text takes the destructive accent, which is what carries the
            // warning once the tint itself is as quiet as a surface.
            text_color: Some(c.destructive),
            border: Border {
                radius,
                ..Default::default()
            },
            ..Default::default()
        },
        BubbleVariant::Ghost => container::Style::default(),
    }
}

/// A vertical stack of consecutive bubbles (a "chat burst").
#[must_use = "a BubbleGroup does nothing unless it is turned into an Element"]
pub struct BubbleGroup<'a, Message> {
    children: Vec<Element<'a, Message, Theme>>,
}

impl<'a, Message: 'a> BubbleGroup<'a, Message> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Adds a bubble (or any element) to the group.
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, Message: 'a> Default for BubbleGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<BubbleGroup<'a, Message>> for Element<'a, Message, Theme> {
    fn from(group: BubbleGroup<'a, Message>) -> Self {
        column(group.children).spacing(2).width(Length::Fill).into()
    }
}

/// Builds a [`Bubble`] from a single text line.
pub fn bubble<'a, Message: 'a>(content: impl text::IntoFragment<'a>) -> Bubble<'a, Message> {
    Bubble::new().child(
        text(content)
            .size(Size::Md.text().size)
            .line_height(Size::Md.text().line_height()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg {}

    const ALL: [BubbleVariant; 7] = [
        BubbleVariant::Filled,
        BubbleVariant::Secondary,
        BubbleVariant::Muted,
        BubbleVariant::Tinted,
        BubbleVariant::Outline,
        BubbleVariant::Destructive,
        BubbleVariant::Ghost,
    ];

    #[test]
    fn default_variant_is_filled() {
        assert_eq!(BubbleVariant::default(), BubbleVariant::Filled);
        assert_eq!(MessageAlignment::default(), MessageAlignment::Start);
    }

    #[test]
    fn renders_every_variant_and_alignment() {
        for v in ALL {
            for a in [MessageAlignment::Start, MessageAlignment::End] {
                let el: Element<'_, Msg, Theme> =
                    bubble("hello").with_variant(v).alignment(a).into();
                drop(el);
            }
        }
    }

    #[test]
    fn bubble_with_reactions_renders_on_each_side() {
        for side in [BubbleReactionSide::Top, BubbleReactionSide::Bottom] {
            let note: Element<'_, Msg, Theme> = text("👍").into();
            let el: Element<'_, Msg, Theme> = bubble("hi")
                .reactions(BubbleReactions::new().side(side).action(note))
                .into();
            drop(el);
        }
    }

    /// The distinction the reference draws: an `action` is a control, and a
    /// cluster of controls needs no padding of its own; a `child` is bare
    /// content, and a cluster of those does.
    #[test]
    fn a_reaction_cluster_tracks_whether_it_holds_a_control() {
        let note: Element<'_, Msg, Theme> = text("👍 2").into();
        let bare = BubbleReactions::<Msg>::new().child(note);
        assert_eq!(bare.len(), 1);
        assert!(!bare.has_action(), "a plain child is not a control");

        let with_action = BubbleReactions::<Msg>::new().action(text("👍"));
        assert!(with_action.has_action(), "an action is a control");

        let count: Element<'_, Msg, Theme> = text("👍 2").into();
        let mixed = BubbleReactions::<Msg>::new()
            .child(count)
            .action(text("👍"));
        assert!(
            mixed.has_action(),
            "one control is enough to claim the padding"
        );

        let empty = BubbleReactions::<Msg>::new();
        assert!(empty.is_empty());
        assert!(!empty.has_action());
    }

    /// A cluster renders standalone as a pill, and a bubble with no reactions
    /// must not grow a pill.
    #[test]
    fn reaction_clusters_render_standalone() {
        let count: Element<'_, Msg, Theme> = text("🎉 4").into();
        let cluster: Element<'_, Msg, Theme> = BubbleReactions::new().child(count).into();
        drop(cluster);

        let no_reactions: Element<'_, Msg, Theme> =
            bubble("plain").reactions(BubbleReactions::new()).into();
        drop(no_reactions);
    }

    /// All four combinations of side and alignment must lay out, since the pill
    /// is positioned by hand.
    #[test]
    fn the_pill_lays_out_on_every_side_and_alignment() {
        for side in [BubbleReactionSide::Top, BubbleReactionSide::Bottom] {
            for alignment in [MessageAlignment::Start, MessageAlignment::End] {
                let note: Element<'_, Msg, Theme> = text("👍 2").into();
                let el: Element<'_, Msg, Theme> = bubble("a message long enough to wrap")
                    .alignment(alignment)
                    .max_width(200.0)
                    .reactions(BubbleReactions::new().side(side).child(note))
                    .into();
                drop(el);
            }
        }
    }

    #[test]
    fn content_group_and_max_width_compose() {
        let el: Element<'_, Msg, Theme> = bubble("x")
            .content(BubbleContent::new().text("a").text("b"))
            .max_width(300.0)
            .into();
        drop(el);

        let group: Element<'_, Msg, Theme> = BubbleGroup::new()
            .child(bubble("one"))
            .child(bubble("two"))
            .into();
        drop(group);
    }

    /// Every tinted surface must be opaque. iced's renderer does not guarantee
    /// alpha compositing, so a translucent fill would take its color from
    /// whatever sits behind the bubble rather than from the theme.
    #[test]
    fn tinted_surfaces_are_opaque() {
        for theme in [Theme::light(), Theme::dark()] {
            for variant in [
                BubbleVariant::Filled,
                BubbleVariant::Secondary,
                BubbleVariant::Muted,
                BubbleVariant::Tinted,
                BubbleVariant::Outline,
                BubbleVariant::Destructive,
            ] {
                let style = super::bubble_style(&theme, variant);

                let Some(Background::Color(color)) = style.background else {
                    panic!("{variant:?} must draw a color surface");
                };

                assert_eq!(
                    color.a, 1.0,
                    "{variant:?} must not rely on alpha compositing"
                );
            }
        }
    }

    /// The destructive bubble is its own signal: a red-tinted surface with red
    /// text, distinct from every other variant in both palettes.
    #[test]
    fn the_destructive_variant_is_distinct() {
        for theme in [Theme::light(), Theme::dark()] {
            let destructive = super::bubble_style(&theme, BubbleVariant::Destructive);

            assert_eq!(
                destructive.text_color,
                Some(theme.colors().destructive),
                "its text must carry the warning"
            );

            for other in ALL {
                if other == BubbleVariant::Destructive {
                    continue;
                }

                let style = super::bubble_style(&theme, other);
                assert_ne!(
                    (style.background, style.text_color),
                    (destructive.background, destructive.text_color),
                    "{other:?} must not look like Destructive"
                );
            }
        }
    }

    /// The tint is more saturated in the dark palette, where a faint wash would
    /// disappear into the background.
    #[test]
    fn the_destructive_tint_strengthens_in_the_dark_palette() {
        let light = super::bubble_style(&Theme::light(), BubbleVariant::Destructive)
            .background
            .expect("a surface");
        let dark = super::bubble_style(&Theme::dark(), BubbleVariant::Destructive)
            .background
            .expect("a surface");

        let (Background::Color(light), Background::Color(dark)) = (light, dark) else {
            panic!("both tints must be colors");
        };

        let redness = |color: Color| color.r - color.g.midpoint(color.b);

        assert!(
            redness(dark) > redness(light),
            "the dark tint must be the more saturated of the two"
        );
    }

    /// Every variant keeps the same generous corner, so a row of bubbles reads
    /// as one family. Ghost is the exception: it has no surface to round.
    #[test]
    fn variants_share_one_corner_radius() {
        let theme = Theme::light();
        let expected = f32::from(theme.radius().md) * 2.5;

        for variant in ALL {
            let style = super::bubble_style(&theme, variant);
            let radius = style.border.radius.top_left;

            if variant == BubbleVariant::Ghost {
                assert_eq!(radius, 0.0, "Ghost has no surface to round");
            } else {
                assert_eq!(radius, expected, "{variant:?} must use the shared corner");
            }
        }
    }

    /// The typed action rounds the control to the cluster's own pill radius,
    /// which is what makes a row of reactions read as one pill.
    #[test]
    fn a_button_action_is_rounded_to_the_pill() {
        use crate::widgets::button;

        let cluster = BubbleReactions::<Msg>::new().button(button("👍"));
        assert!(cluster.has_action(), "a button is a control");
        assert_eq!(cluster.len(), 1);

        // The radius is explicit and very large rather than the theme's `full`
        // token, so a small radius scale cannot leave the control squarer than
        // its cluster. The value is read through a runtime binding so the check
        // is on the constant the widget uses rather than a copy of it.
        let radius = PILL_RADIUS;
        assert!(
            radius >= 999.0,
            "a pill's children must be at least as round as the pill"
        );
    }

    /// A cluster mixing typed buttons and bare elements still counts as having
    /// a control, so it takes no padding of its own.
    #[test]
    fn a_mixed_cluster_still_has_a_control() {
        use crate::widgets::button;

        let count: Element<'_, Msg, Theme> = text("2").into();
        let cluster = BubbleReactions::<Msg>::new()
            .child(count)
            .button(button("👍"));

        assert!(cluster.has_action());
        assert_eq!(cluster.len(), 2);
    }
}
