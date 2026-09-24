//! Avatars: a user's initials on a tinted circle.

use crate::theme::{Size, Theme};
use iced::advanced::layout::{self, Limits, Node};
use iced::advanced::widget::{tree, Operation};
use iced::advanced::{mouse, Clipboard, Shell, Widget};
use iced::widget::{container, text};
use iced::{Color, Element, Length, Point, Rectangle, Size as IcedSize, Vector};

/// The shape of an [`avatar`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AvatarShape {
    /// A circle, the familiar user-avatar shape.
    #[default]
    Circle,
    /// A rounded square.
    Square,
}

/// Builds an avatar showing a person's (or team's) initials.
///
/// Up to two initials are rendered; extra characters are dropped so a long
/// display name cannot overflow the circle.
///
/// ```
/// # use iced_kit::widgets::avatar;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// avatar("Ada Lovelace", 40)
/// # }
/// ```
pub fn avatar<'a, Message: 'a>(
    name: impl text::IntoFragment<'a>,
    diameter: u16,
) -> Element<'a, Message, Theme> {
    let label = initials(name.into_fragment().as_ref());
    avatar_with_label(label, diameter)
}

/// Builds an avatar from an explicit label rather than a display name.
///
/// Use this when the initials are already known, so no name has to be parsed.
pub fn avatar_with_label<'a, Message: 'a>(
    label: impl text::IntoFragment<'a>,
    diameter: u16,
) -> Element<'a, Message, Theme> {
    Avatar::new(label.into_fragment().into_owned())
        .diameter(diameter)
        .into_element()
}

/// An avatar under construction.
///
/// [`avatar`] and [`avatar_with_label`] cover the common case; this builder
/// adds the shape and the muted tint a group's overflow chip wears.
#[must_use = "an Avatar does nothing unless it is turned into an Element"]
pub struct Avatar {
    label: String,
    diameter: u16,
    shape: AvatarShape,
    muted: bool,
}

impl Avatar {
    /// Creates an avatar showing `label` as its text.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            diameter: 40,
            shape: AvatarShape::default(),
            muted: false,
        }
    }

    /// Sets the avatar's diameter in logical pixels.
    pub fn diameter(mut self, diameter: u16) -> Self {
        self.diameter = diameter.max(1);
        self
    }

    /// Sets the avatar's shape.
    pub fn shape(mut self, shape: AvatarShape) -> Self {
        self.shape = shape;
        self
    }

    /// Draws the avatar in the muted surface color.
    ///
    /// A group's overflow chip wears this so it reads as a count rather than as
    /// another member.
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// Converts the avatar into an [`Element`].
    pub fn into_element<'a, Message: 'a>(self) -> Element<'a, Message, Theme> {
        let diameter = self.diameter;
        let shape = self.shape;
        let muted = self.muted;
        let font_size = (f32::from(diameter) * 0.4).max(9.0);
        let radius = match shape {
            AvatarShape::Circle => f32::from(diameter) / 2.0,
            AvatarShape::Square => f32::from(diameter) * 0.28,
        };

        container(
            text(self.label)
                .size(font_size)
                .line_height(iced::Pixels(font_size * 1.2))
                .font(iced::Font {
                    weight: iced::font::Weight::Medium,
                    ..iced::Font::DEFAULT
                }),
        )
        .width(Length::Fixed(f32::from(diameter)))
        .height(Length::Fixed(f32::from(diameter)))
        .center_x(Length::Fixed(f32::from(diameter)))
        .center_y(Length::Fixed(f32::from(diameter)))
        .class(Box::new(move |theme: &Theme| {
            let colors = theme.colors();
            // The primary token carries the identity tint, so a custom palette
            // recolors avatars along with the rest of the UI. The muted chip
            // takes the quiet surface instead.
            let tint = if muted { colors.muted } else { colors.primary };

            container::Style {
                background: Some(iced::Background::Color(Color { a: 0.15, ..tint })),
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: radius.into(),
                },
                text_color: Some(if muted {
                    colors.muted_foreground
                } else {
                    readable_on_tint(tint, colors.background)
                }),
                ..container::Style::default()
            }
        }) as container::StyleFn<'a, Theme>)
        .into()
    }
}

impl<'a, Message: 'a> From<Avatar> for Element<'a, Message, Theme> {
    fn from(avatar: Avatar) -> Self {
        avatar.into_element()
    }
}

/// A label placed next to an avatar.
#[derive(Debug, Clone)]
#[must_use = "an AvatarLabel does nothing unless it is given to `avatar_with_name`"]
pub struct AvatarLabel {
    name: String,
    detail: Option<String>,
}

impl AvatarLabel {
    /// Creates a label with a name and an optional secondary line.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            detail: None,
        }
    }

    /// Sets the secondary line, such as an email address.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Builds an avatar with its name (and optional detail) beside it.
pub fn avatar_with_name<'a, Message: 'a>(
    label: AvatarLabel,
    diameter: u16,
    size: Size,
) -> Element<'a, Message, Theme> {
    let name_style = size.text();
    let detail_style = Size::Sm.text();

    let avatar_element: Element<'a, Message, Theme> =
        avatar_with_label(initials(&label.name), diameter);

    let mut column = iced::widget::column![text(label.name)
        .size(name_style.size)
        .line_height(name_style.line_height())]
    .spacing(2);

    if let Some(detail) = label.detail {
        column = column.push(
            text(detail)
                .size(detail_style.size)
                .line_height(detail_style.line_height())
                .class(Box::new(|theme: &Theme| text::Style {
                    color: Some(theme.colors().muted_foreground),
                }) as text::StyleFn<'a, Theme>),
        );
    }

    iced::widget::row![avatar_element, column]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
}

/// Picks a legible color for text drawn on a lightly tinted background.
///
/// The avatar background is the accent at 15% over the page background, which
/// is far lighter than the accent itself, so the accent cannot be used as-is.
fn readable_on_tint(accent: Color, background: Color) -> Color {
    const TINT: f32 = 0.15;

    let tinted = Color {
        r: accent.r * TINT + background.r * (1.0 - TINT),
        g: accent.g * TINT + background.g * (1.0 - TINT),
        b: accent.b * TINT + background.b * (1.0 - TINT),
        a: 1.0,
    };

    let luminance = 0.2126 * tinted.r + 0.7152 * tinted.g + 0.0722 * tinted.b;

    if luminance > 0.5 {
        // Darken the accent so it reads on a pale tint.
        Color {
            r: accent.r * 0.45,
            g: accent.g * 0.45,
            b: accent.b * 0.45,
            a: 1.0,
        }
    } else {
        // Lighten it for a dark tint.
        Color {
            r: accent.r + (1.0 - accent.r) * 0.55,
            g: accent.g + (1.0 - accent.g) * 0.55,
            b: accent.b + (1.0 - accent.b) * 0.55,
            a: 1.0,
        }
    }
}

/// Extracts up to two initials from a display name.
///
/// Splits on whitespace and takes the first character of the first and last
/// words, so "Ada Lovelace" yields "AL" and "Ada" yields "A". A name with no
/// alphabetic characters falls back to "?" so the avatar is never blank.
fn initials(name: &str) -> String {
    let mut words = name
        .split_whitespace()
        .filter_map(|word| word.chars().next());

    let first = match words.next() {
        Some(c) if c.is_alphanumeric() => c,
        _ => return "?".to_owned(),
    };

    // The last word is what distinguishes "Ada Lovelace" from "Ada B. Lovelace".
    let last = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .rfind(|c: &char| c.is_alphanumeric());

    match last {
        Some(last) if last != first || name.split_whitespace().count() > 1 => {
            let mut result = String::with_capacity(2);
            result.push(first.to_ascii_uppercase());
            result.push(last.to_ascii_uppercase());
            result
        }
        _ => first.to_ascii_uppercase().to_string(),
    }
}

/// A row of overlapping avatars, with an overflow chip when there are more
/// than the limit.
///
/// The avatars stack rather than sit side by side, which is what keeps a long
/// membership row from reading as a list. iced has no negative margins, so
/// [`Overlap`] does that layout.
///
/// ```
/// # use iced_kit::widgets::{avatar, avatar_group};
/// # use iced_kit::Theme;
/// # use iced::Element;
/// # #[derive(Clone, Debug)] enum Message {}
/// # fn view() -> Element<'static, Message, Theme> {
/// let members = ["Ada Lovelace", "Grace Hopper", "Alan Turing", "Edsger Dijkstra"];
/// avatar_group()
///     .children(members.map(|name| avatar::<Message>(name, 32)))
///     .limit(3)
///     .ellipsis()
///     .into()
/// # }
/// ```
#[must_use = "an AvatarGroup does nothing unless it is turned into an Element"]
pub struct AvatarGroup<'a, Message> {
    avatars: Vec<Element<'a, Message, Theme>>,
    diameter: u16,
    limit: usize,
    ellipsis: bool,
}

impl<'a, Message: 'a> AvatarGroup<'a, Message> {
    /// Creates an empty group.
    pub fn new() -> Self {
        Self {
            avatars: Vec::new(),
            diameter: 40,
            limit: 3,
            ellipsis: false,
        }
    }

    /// Adds an avatar.
    pub fn child(mut self, avatar: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.avatars.push(avatar.into());
        self
    }

    /// Adds several avatars.
    pub fn children(
        mut self,
        avatars: impl IntoIterator<Item = impl Into<Element<'a, Message, Theme>>>,
    ) -> Self {
        self.avatars.extend(avatars.into_iter().map(Into::into));
        self
    }

    /// Sets the diameter the group's avatars are drawn at, which also decides
    /// how far they overlap. Default 40.
    pub fn diameter(mut self, diameter: u16) -> Self {
        self.diameter = diameter.max(1);
        self
    }

    /// Sets how many avatars are drawn before the overflow chip. Default 3.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    /// Draws the overflow chip when the group is over its limit.
    ///
    /// Without this, avatars past the limit are simply dropped, which silently
    /// misreports how many there are.
    pub fn ellipsis(mut self) -> Self {
        self.ellipsis = true;
        self
    }

    /// How many avatars the group is hiding, if it is over its limit.
    #[must_use]
    pub fn hidden(&self) -> usize {
        self.avatars.len().saturating_sub(self.limit)
    }

    /// Converts the group into an [`Element`].
    pub fn into_element(self) -> Element<'a, Message, Theme> {
        let Self {
            avatars,
            diameter,
            limit,
            ellipsis,
        } = self;

        let hidden = avatars.len().saturating_sub(limit);
        // Each avatar begins before the previous one ends, which is what makes
        // the row read as a stack rather than as a list.
        let step = f32::from(diameter) * 0.7;

        // The trailing avatar is the one closest to the reader's eye: iced
        // paints in order, so a later child draws over an earlier one, and the
        // rightmost should be the one on top of the stack.
        let mut chips: Vec<Element<'a, Message, Theme>> = avatars.into_iter().take(limit).collect();

        if ellipsis && hidden > 0 {
            chips.push(
                Avatar::new(format!("+{hidden}"))
                    .diameter(diameter)
                    .muted(true)
                    .into_element(),
            );
        }

        Overlap::new(chips, step).into()
    }
}

impl<'a, Message: 'a> Default for AvatarGroup<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<AvatarGroup<'a, Message>> for Element<'a, Message, Theme> {
    fn from(group: AvatarGroup<'a, Message>) -> Self {
        group.into_element()
    }
}

/// Builds a row of overlapping avatars.
pub fn avatar_group<'a, Message: 'a>() -> AvatarGroup<'a, Message> {
    AvatarGroup::new()
}

/// Lays elements out in a row where each begins `step` pixels after the last.
///
/// A row cannot express this: `spacing` only ever separates its children, and
/// the whole point of a stack of avatars is that they overlap. Each child keeps
/// its own size and the row is only as tall as the tallest.
struct Overlap<'a, Message> {
    children: Vec<Element<'a, Message, Theme>>,
    step: f32,
}

impl<'a, Message> Overlap<'a, Message> {
    fn new(children: Vec<Element<'a, Message, Theme>>, step: f32) -> Self {
        Self {
            children,
            step: step.max(0.0),
        }
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Overlap<'_, Message> {
    fn children(&self) -> Vec<tree::Tree> {
        self.children.iter().map(tree::Tree::new).collect()
    }

    fn diff(&self, tree: &mut tree::Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut tree::Tree,
        renderer: &iced::Renderer,
        limits: &Limits,
    ) -> Node {
        let count = self.children.len();
        let mut nodes = Vec::with_capacity(count);
        let mut x = 0.0;
        let mut height: f32 = 0.0;

        for (index, child) in self.children.iter_mut().enumerate() {
            let node = child.as_widget_mut().layout(
                &mut tree.children[index],
                renderer,
                &Limits::new(IcedSize::ZERO, limits.max()),
            );

            let size = node.size();
            height = height.max(size.height);
            nodes.push(node.move_to(Point::new(x, 0.0)));

            // The last child advances nothing: there is no one after it to
            // make room for, and its width is what the row ends up being.
            x += if index + 1 == count {
                size.width
            } else {
                self.step
            };
        }

        Node::with_children(IcedSize::new(x, height), nodes)
    }

    fn operate(
        &mut self,
        tree: &mut tree::Tree,
        layout: layout::Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let children: Vec<layout::Layout<'_>> = layout.children().collect();

        for (index, child) in self.children.iter_mut().enumerate() {
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

        // Back to front, so the element drawn on top is the one that answers
        // for a click: the stack's own z-order decides who is reachable.
        for index in (0..self.children.len()).rev() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };

            self.children[index].as_widget_mut().update(
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

        // In order, so each avatar is painted over the one before it.
        for (index, child) in self.children.iter().enumerate() {
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

        for index in (0..self.children.len()).rev() {
            let Some(child_layout) = children.get(index).copied() else {
                break;
            };

            let interaction = self.children[index].as_widget().mouse_interaction(
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
        let overlays = self
            .children
            .iter_mut()
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

#[cfg(test)]
mod tests {
    use super::{
        avatar, avatar_group, avatar_with_label, avatar_with_name, initials, Avatar, AvatarGroup,
        AvatarLabel, AvatarShape,
    };
    use crate::theme::Theme;

    #[test]
    fn initials_use_the_first_and_last_words() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("Grace Brewster Murray Hopper"), "GH");
        assert_eq!(initials("Ada"), "A");
    }

    #[test]
    fn initials_are_uppercased() {
        assert_eq!(initials("ada lovelace"), "AL");
    }

    #[test]
    fn initials_ignore_extra_whitespace() {
        assert_eq!(initials("  Ada   Lovelace  "), "AL");
    }

    #[test]
    fn a_nameless_avatar_falls_back_rather_than_going_blank() {
        assert_eq!(initials(""), "?");
        assert_eq!(initials("   "), "?");
        assert_eq!(initials("123"), "1");
    }

    #[test]
    fn an_avatar_renders_at_several_sizes() {
        for diameter in [16, 24, 40, 96] {
            let element: iced::Element<'_, (), crate::theme::Theme> =
                avatar("Ada Lovelace", diameter);
            drop(element);
        }
    }

    #[test]
    fn a_labelled_avatar_renders_with_and_without_a_detail_line() {
        let simple: iced::Element<'_, (), crate::theme::Theme> =
            avatar_with_name(AvatarLabel::new("Ada Lovelace"), 40, crate::theme::Size::Md);
        drop(simple);

        let detailed: iced::Element<'_, (), crate::theme::Theme> = avatar_with_name(
            AvatarLabel::new("Ada Lovelace").detail("ada@example.com"),
            40,
            crate::theme::Size::Lg,
        );
        drop(detailed);
    }

    #[test]
    fn a_precomputed_label_avatar_renders() {
        let element: iced::Element<'_, (), crate::theme::Theme> = avatar_with_label("AL", 32);
        drop(element);
    }

    #[test]
    fn tinted_text_is_legible_in_both_palettes() {
        use crate::theme::Theme;
        use iced::theme::Base;

        let contrast = |a: iced::Color, b: iced::Color| {
            let lum = |c: iced::Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
            let (hi, lo) = {
                let (x, y) = (lum(a), lum(b));
                if x > y {
                    (x, y)
                } else {
                    (y, x)
                }
            };
            (hi + 0.05) / (lo + 0.05)
        };

        for theme in [Theme::light(), Theme::dark()] {
            let base = theme.base();
            let tinted = super::readable_on_tint(theme.colors().primary, base.background_color);
            let ratio = contrast(tinted, base.background_color);

            assert!(
                ratio >= 3.0,
                "avatar initials need enough contrast to read, got {ratio:.2}"
            );
        }
    }

    #[test]
    fn an_avatar_builder_records_its_settings() {
        let built = Avatar::new("AL")
            .diameter(32)
            .shape(AvatarShape::Square)
            .muted(true);

        assert_eq!(built.diameter, 32);
        assert_eq!(built.shape, AvatarShape::Square);
        assert!(built.muted);
        // A zero diameter would divide into a zero radius, so it is floored.
        assert_eq!(Avatar::new("AL").diameter(0).diameter, 1);
    }

    #[test]
    fn an_avatar_renders_at_every_shape() {
        for shape in [AvatarShape::Circle, AvatarShape::Square] {
            let element: iced::Element<'_, (), Theme> = Avatar::new("AL").shape(shape).into();
            drop(element);
        }
    }

    #[test]
    fn an_avatar_group_records_its_settings() {
        let group: AvatarGroup<'_, ()> = avatar_group()
            .child(avatar::<()>("Ada Lovelace", 32))
            .child(avatar::<()>("Grace Hopper", 32))
            .diameter(32)
            .limit(1)
            .ellipsis();

        assert_eq!(group.avatars.len(), 2);
        assert_eq!(group.diameter, 32);
        assert_eq!(group.limit, 1);
        assert!(group.ellipsis);
        // One avatar over the limit is one hidden.
        assert_eq!(group.hidden(), 1);
    }

    /// A limit of zero would draw nothing at all, so it is floored at one.
    #[test]
    fn a_group_limit_is_at_least_one() {
        let group: AvatarGroup<'_, ()> = avatar_group().limit(0);
        assert_eq!(group.limit, 1);
        assert_eq!(AvatarGroup::<'_, ()>::default().limit, 3);
    }

    #[test]
    fn a_group_within_its_limit_hides_nothing() {
        let group: AvatarGroup<'_, ()> = avatar_group()
            .children([
                avatar::<()>("Ada Lovelace", 32),
                avatar::<()>("Grace Hopper", 32),
            ])
            .limit(3);

        assert_eq!(group.hidden(), 0);
    }

    #[test]
    fn an_avatar_group_renders_with_and_without_the_overflow_chip() {
        let members = [
            "Ada Lovelace",
            "Grace Hopper",
            "Alan Turing",
            "Edsger Dijkstra",
        ];

        for ellipsis in [false, true] {
            let group: AvatarGroup<'_, ()> = avatar_group()
                .children(members.map(|name| avatar::<()>(name, 32)))
                .limit(3);

            let group = if ellipsis { group.ellipsis() } else { group };
            let element: iced::Element<'_, (), Theme> = group.into();
            drop(element);
        }

        // An empty group is a valid, if pointless, row.
        let empty: iced::Element<'_, (), Theme> = avatar_group::<()>().into();
        drop(empty);
    }
}
