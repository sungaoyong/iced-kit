//! Chat bubbles: the surface a message's content sits in, its alignment, and
//! the emoji reactions that may accompany it.

use crate::theme::{Size, Theme};
use iced::widget::{column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};

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

        let mut stack = column![surface].spacing(4).width(stack_width);
        if let Some(reactions) = bubble.reactions {
            if !reactions.children.is_empty() {
                let cluster: Element<'a, Message, Theme> =
                    row(reactions.children).spacing(4).into();
                stack = match reactions.side {
                    BubbleReactionSide::Top => column![cluster, stack].align_x(edge),
                    BubbleReactionSide::Bottom => column![stack, cluster].align_x(edge),
                }
                .spacing(4)
                .width(stack_width);
            }
        }

        // The outer container spans the row so it can push the shrunken stack
        // to the leading or trailing edge.
        container(stack).width(Length::Fill).align_x(edge).into()
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
}
