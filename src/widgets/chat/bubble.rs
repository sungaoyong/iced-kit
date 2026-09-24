//! Chat bubbles: the surface a message's content sits in, its alignment, and
//! the emoji reactions that may accompany it.

use crate::theme::Theme;
use iced::widget::{column, container, row, text};
use iced::{Alignment, Background, Border, Element, Length, Padding};

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
        Self { children: Vec::new() }
    }

    /// Adds a text line to the bubble body.
    pub fn text(mut self, content: impl text::IntoFragment<'a>) -> Self {
        self.children.push(text(content).into());
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
}

impl<'a, Message: 'a> BubbleReactions<'a, Message> {
    pub fn new() -> Self {
        Self {
            side: BubbleReactionSide::default(),
            children: Vec::new(),
        }
    }

    /// Chooses whether the cluster sits above or below the bubble.
    pub fn side(mut self, side: BubbleReactionSide) -> Self {
        self.side = side;
        self
    }

    /// Adds a reaction control (usually a small [`Button`](crate::widgets::Button)).
    pub fn action(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }

    /// Alias for [`action`](Self::action) for non-button children.
    pub fn child(mut self, el: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(el.into());
        self
    }
}

impl<'a, Message: 'a> Default for BubbleReactions<'a, Message> {
    fn default() -> Self {
        Self::new()
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
}

impl<'a, Message: 'a> Default for Bubble<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<Bubble<'a, Message>> for Element<'a, Message, Theme> {
    fn from(bubble: Bubble<'a, Message>) -> Self {
        let variant = bubble.variant;
        let body = column(bubble.children).spacing(4).width(Length::Fill);

        // Ghost has no surface at all; the others wrap the body in a styled
        // container so its text color flows into descendants.
        let surface: Element<'a, Message, Theme> = if variant == BubbleVariant::Ghost {
            body.into()
        } else {
            container(body)
                .padding(Padding::from([8.0, 12.0]))
                .class(Box::new(move |theme: &Theme| bubble_style(theme, variant))
                    as iced::widget::container::StyleFn<'a, Theme>)
                .into()
        };

        let mut stack = column![surface].spacing(4).width(Length::Shrink);
        if let Some(reactions) = bubble.reactions {
            if !reactions.children.is_empty() {
                let cluster: Element<'a, Message, Theme> =
                    row(reactions.children).spacing(4).into();
                let edge = match bubble.alignment {
                    MessageAlignment::Start => Alignment::Start,
                    MessageAlignment::End => Alignment::End,
                };
                stack = match reactions.side {
                    BubbleReactionSide::Top => column![cluster, stack].align_x(edge),
                    BubbleReactionSide::Bottom => column![stack, cluster].align_x(edge),
                }
                .spacing(4)
                .width(Length::Shrink);
            }
        }

        // A capped width lets long messages wrap instead of filling the row;
        // otherwise the stack shrinks to its content so the outer container can
        // push it to the leading or trailing edge.
        let sized = match bubble.max_width {
            Some(width) => stack.width(Length::Fixed(width)),
            None => stack,
        };

        container(sized)
            .width(Length::Fill)
            .align_x(match bubble.alignment {
                MessageAlignment::Start => Alignment::Start,
                MessageAlignment::End => Alignment::End,
            })
            .into()
    }
}

/// Resolves a bubble variant to a container style against the theme.
fn bubble_style(theme: &Theme, variant: BubbleVariant) -> container::Style {
    let c = theme.colors();
    let radius = f32::from(theme.radius().lg).into();
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
        BubbleVariant::Secondary => container::Style {
            background: Some(Background::Color(c.secondary)),
            text_color: Some(c.secondary_foreground),
            border: Border {
                radius,
                ..Default::default()
            },
            ..Default::default()
        },
        BubbleVariant::Muted => container::Style {
            background: Some(Background::Color(c.muted)),
            text_color: Some(c.muted_foreground),
            border: Border {
                radius,
                ..Default::default()
            },
            ..Default::default()
        },
        BubbleVariant::Tinted => container::Style {
            background: Some(Background::Color(iced::Color {
                a: 0.12,
                ..c.primary
            })),
            text_color: Some(c.foreground),
            border: Border {
                radius,
                ..Default::default()
            },
            ..Default::default()
        },
        BubbleVariant::Outline => container::Style {
            background: Some(Background::Color(c.surface)),
            text_color: Some(c.foreground),
            border: Border {
                color: c.border,
                width: 1.0,
                radius,
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
        Self { children: Vec::new() }
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
    Bubble::new().child(text(content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use iced::Element;

    #[derive(Clone, Debug)]
    enum Msg {}

    const ALL: [BubbleVariant; 6] = [
        BubbleVariant::Filled,
        BubbleVariant::Secondary,
        BubbleVariant::Muted,
        BubbleVariant::Tinted,
        BubbleVariant::Outline,
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
}
