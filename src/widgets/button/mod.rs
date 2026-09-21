//! Buttons.
//!
//! [`Button`] wraps iced's own button so that variant, size, outline, selection
//! and loading state are driven by the [`Theme`]'s tokens instead of per-call
//! styling.
//!
//! # Compared to a stock iced button
//!
//! iced's `button` is deliberately bare: it carries a message and a style
//! closure. This wrapper adds the vocabulary `gpui-kit` uses — named variants, a
//! size scale, an icon slot, an outline mode, a selected state — while still
//! resolving down to one iced button, so it costs nothing at runtime and
//! composes with the rest of iced's widget set.

mod group;
mod split;
mod toggle;

#[doc(hidden)]
pub mod icon;

pub use group::{member_corners, ButtonGroup, ButtonGroupLayout};
pub use icon::{Icon, IconSource, LoadingIcon};
pub use split::DropdownButton;
pub use toggle::{Toggle, ToggleGroup, ToggleVariant};

use crate::theme::catalog::{ButtonClass, ButtonRounded, ButtonVariant, Corners};
use crate::theme::{Size, Theme};
use crate::widgets::button::icon::loading_indicator;
use crate::widgets::overlay::{tooltip_at, TooltipPosition};
use iced::widget::{button as iced_button, row, text};
use iced::{Element, Font, Length, Padding};

/// The label shown while a button is loading and has nothing else to show.
///
/// A button with an icon puts its spinner in the icon's place and keeps its
/// label, so the control does not change width mid-action. Only a button with
/// no content at all falls back to this placeholder.
const LOADING_LABEL: &str = "…";

/// Builds a [`Button`] with the given label.
///
/// ```
/// # use iced_kit::widgets::button;
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Save }
/// # fn view() -> iced::Element<'static, Message, Theme> {
/// button::<Message>("Save").primary().on_press(Message::Save).into()
/// # }
/// ```
pub fn button<'a, Message: Clone + 'a>(label: impl text::IntoFragment<'a>) -> Button<'a, Message> {
    Button::new(label)
}

/// Builds a button showing only an icon.
///
/// Without a label the button sizes to a square, which is what makes a row of
/// icon buttons line up.
///
/// ```
/// # use iced_kit::widgets::{button, icon_button};
/// # use iced_kit::Theme;
/// # #[derive(Clone, Debug)] enum Message { Close }
/// # fn view() -> iced::Element<'static, Message, Theme> {
/// icon_button::<Message>()
///     .icon("✕")
///     .ghost()
///     .on_press(Message::Close)
///     .into()
/// # }
/// ```
pub fn icon_button<'a, Message: Clone + 'a>() -> Button<'a, Message> {
    Button::icon_only()
}

/// A themed button.
#[must_use = "a Button does nothing unless it is turned into an Element"]
#[allow(clippy::struct_excessive_bools)]
pub struct Button<'a, Message> {
    label: Option<text::Fragment<'a>>,
    icon: Option<Icon>,
    /// Content appended after the label, for a caller that needs more than a
    /// string — a badge, a shortcut hint, a count.
    children: Vec<Element<'a, Message, Theme>>,
    variant: ButtonVariant,
    size: Size,
    loading: bool,
    loading_icon: LoadingIcon,
    disabled: bool,
    outline: bool,
    selected: bool,
    toggled: Option<bool>,
    compact: bool,
    rounded: ButtonRounded,
    corners: Corners,
    dropdown_caret: bool,
    tooltip: Option<(String, TooltipPosition)>,
    font: Option<Font>,
    width: Option<Length>,
    height: Option<Length>,
    padding: Option<Padding>,
    on_press: Option<Message>,
}

impl<'a, Message: Clone + 'a> Button<'a, Message> {
    /// Creates a button with the given label.
    pub fn new(label: impl text::IntoFragment<'a>) -> Self {
        Self::with_label(Some(label.into_fragment()))
    }

    /// Creates a button with no label.
    pub fn icon_only() -> Self {
        Self::with_label(None)
    }

    fn with_label(label: Option<text::Fragment<'a>>) -> Self {
        Self {
            label,
            icon: None,
            children: Vec::new(),
            variant: ButtonVariant::Default,
            size: Size::Md,
            loading: false,
            loading_icon: LoadingIcon::default(),
            disabled: false,
            outline: false,
            selected: false,
            toggled: None,
            compact: false,
            rounded: ButtonRounded::default(),
            corners: Corners::ALL,
            dropdown_caret: false,
            tooltip: None,
            font: None,
            width: None,
            height: None,
            padding: None,
            on_press: None,
        }
    }

    /// Renders the button with the primary variant.
    pub fn primary(self) -> Self {
        self.variant(ButtonVariant::Primary)
    }

    /// Renders the button with the secondary variant.
    pub fn secondary(self) -> Self {
        self.variant(ButtonVariant::Secondary)
    }

    /// Renders the button as a soft destructive action: a red wash, red text.
    pub fn danger(self) -> Self {
        self.variant(ButtonVariant::Danger)
    }

    /// Renders the button as a filled destructive action.
    ///
    /// This is the right choice for a genuinely irreversible action such as a
    /// delete; [`danger`](Self::danger) is the softer tint.
    pub fn destructive(self) -> Self {
        self.variant(ButtonVariant::Destructive)
    }

    /// Renders the button with the warning variant.
    pub fn warning(self) -> Self {
        self.variant(ButtonVariant::Warning)
    }

    /// Renders the button with the success variant.
    pub fn success(self) -> Self {
        self.variant(ButtonVariant::Success)
    }

    /// Renders the button with the info variant.
    pub fn info(self) -> Self {
        self.variant(ButtonVariant::Info)
    }

    /// Renders the button as a ghost: transparent until hovered.
    pub fn ghost(self) -> Self {
        self.variant(ButtonVariant::Ghost)
    }

    /// Renders the button as a link, underlined in every state.
    pub fn link(self) -> Self {
        self.variant(ButtonVariant::Link)
    }

    /// Renders the button as plain text, with no padding or border.
    pub fn text(self) -> Self {
        self.variant(ButtonVariant::Text)
    }

    /// Sets the button's variant explicitly.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the button's size.
    pub fn size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }

    /// Draws the button as an outline: an accent wash rather than an accent
    /// fill.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Draws the button in its selected state.
    ///
    /// Use this for a button that reflects a choice — the active filter, the
    /// current tab. A selection is a stronger signal than a hover, so a
    /// selected button stays distinguishable while the pointer moves elsewhere.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets the pressed state announced to assistive technology.
    ///
    /// This is metadata only; use [`selected`](Self::selected) for the styling.
    /// Call this on a real toggle button, so a screen reader announces it as
    /// pressed rather than as an ordinary push button.
    pub fn toggled(mut self, toggled: bool) -> Self {
        self.toggled = Some(toggled);
        self
    }

    /// Makes the button inert and dims it.
    ///
    /// A loading button never emits its message: firing an action twice while
    /// the first call is still in flight is almost never what the caller wants.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Sets what the button shows in place of its icon while loading.
    pub fn loading_icon(mut self, icon: LoadingIcon) -> Self {
        self.loading_icon = icon;
        self
    }

    /// Makes the button inert and draws it in the disabled style.
    ///
    /// Distinct from leaving off a press handler, which also renders the
    /// disabled style: this keeps the handler but suppresses it, which is what
    /// a control that is temporarily unavailable needs.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Tightens the padding, for a dense toolbar.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Sets the border radius.
    pub fn rounded(mut self, rounded: impl Into<ButtonRounded>) -> Self {
        self.rounded = rounded.into();
        self
    }

    /// Shows a caret after the label, for a button that opens a menu.
    pub fn dropdown_caret(mut self) -> Self {
        self.dropdown_caret = true;
        self
    }

    /// Adds a leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Appends an arbitrary element after the label.
    pub fn push(mut self, child: impl Into<Element<'a, Message, Theme>>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Attaches a tooltip.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some((tooltip.into(), TooltipPosition::default()));
        self
    }

    /// Attaches a tooltip at an explicit position.
    pub fn tooltip_at(mut self, tooltip: impl Into<String>, position: TooltipPosition) -> Self {
        self.tooltip = Some((tooltip.into(), position));
        self
    }

    /// Sets the label's font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the button's width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// Sets the button's height, overriding the size's own.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Sets the button's padding, overriding the size's own.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Sets the message to emit when the button is pressed.
    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Sets the message to emit when the button is pressed, if any.
    pub fn on_press_maybe(mut self, message: Option<Message>) -> Self {
        self.on_press = message;
        self
    }

    /// Whether the button currently emits a message when pressed.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.on_press.is_some() && !self.loading && !self.disabled
    }

    /// Whether the button draws no padding, so it reads as inline content.
    #[must_use]
    pub fn is_borderless(&self) -> bool {
        self.variant.no_padding()
    }

    /// The class this button resolves its appearance from.
    ///
    /// Exposed so a composite control — a group, a split button — can give a
    /// member button the same palette as the control it belongs to.
    #[must_use]
    pub fn class(&self) -> ButtonClass {
        ButtonClass {
            variant: self.variant,
            size: self.size,
            outline: self.outline,
            selected: self.selected,
            loading: self.loading,
            disabled: self.disabled,
            rounded: self.rounded,
            corners: self.corners,
        }
    }

    /// Overrides the corner mask, for a button that is one member of a joined
    /// group.
    ///
    /// A group needs its outer edges rounded and its inner edges square, which
    /// is what makes the members read as a single control.
    pub fn corners(mut self, corners: Corners) -> Self {
        self.corners = corners;
        self
    }

    /// Whether the button is drawn in icon-only mode: a square, with no label
    /// or caller content beside the icon.
    #[must_use]
    pub fn is_icon_only(&self) -> bool {
        self.icon.is_some() && self.label.is_none() && self.children.is_empty()
    }

    /// The padding this button draws with.
    ///
    /// A horizontal inset with no vertical padding: the height comes from the
    /// height constraint, and the content centers itself within it. That keeps
    /// a row of buttons of different variants exactly the same height.
    #[must_use]
    pub fn resolved_padding(&self) -> Padding {
        if let Some(padding) = self.padding {
            return padding;
        }

        // A link or text button reads as inline content, so it takes no padding
        // at all rather than a smaller amount.
        if self.variant.no_padding() {
            return Padding::ZERO;
        }

        // A compact button narrows its inset but keeps its height, so it can
        // sit beside a full-size one without breaking the row's baseline.
        let inset = if self.compact {
            self.size.padding() * 0.6
        } else {
            self.size.padding()
        };

        Padding {
            top: 0.0,
            right: inset,
            bottom: 0.0,
            left: inset,
        }
    }

    /// The placeholder shown in place of content while loading.
    fn loading_placeholder(&self) -> Element<'a, Message, Theme> {
        text(LOADING_LABEL)
            .size(self.size.text().size)
            .line_height(self.size.text().line_height())
            .into()
    }

    /// Builds the label, underlined when the variant calls for it.
    ///
    /// The underline has to be drawn by the text itself: iced's
    /// `button::Style` has no underline field, so a `Link` variant that only set
    /// a flag would render exactly like `Text`. Rich text carries the
    /// decoration, and a single span is enough for one label.
    fn label_element(&self, label: &text::Fragment<'a>) -> Element<'a, Message, Theme> {
        let text_style = self.size.text();
        let line_height = self.resolved_line_height();

        if !self.variant.underline() {
            return text(label.clone())
                .size(text_style.size)
                .line_height(line_height)
                .font_maybe(self.font)
                .into();
        }

        // The link type is `()`: this label is decoration, not a clickable
        // span — the button itself carries the message.
        let span: text::Span<'a, (), Font> = text::Span::new(label.clone())
            .size(text_style.size)
            .line_height(line_height)
            .font_maybe(self.font)
            .underline(true);

        text::Rich::with_spans([span]).into()
    }

    /// The line box a label draws in.
    ///
    /// It is as tall as the control, which is what centres the glyph inside it.
    /// iced anchors a paragraph by its own glyph bounds — ascender through
    /// descender — so a line box sized to the text alone leaves the label
    /// sitting against the top of a taller control. Matching the control's
    /// height puts the baseline where the eye expects it, with no wrapper to
    /// align.
    ///
    /// The result is [`iced::Pixels`], not a bare `f32`: iced reads a plain
    /// number as a *multiple* of the font size, which would inflate the label's
    /// box to hundreds of pixels.
    #[must_use]
    pub fn resolved_line_height(&self) -> iced::Pixels {
        let height = match self.height {
            Some(Length::Fixed(px)) => px,
            _ => self.size.height(),
        };

        // A control shorter than its own text would otherwise clip it.
        iced::Pixels(height.max(self.size.text().line_height))
    }

    /// Builds the button's inner content row.
    ///
    /// The icon and the label are colored by the button's resolved
    /// `text_color`, which iced passes to its content as the renderer's
    /// inherited style — so nothing here has to know the variant.
    fn content(&mut self) -> Element<'a, Message, Theme> {
        let size = self.size;
        let loading = self.loading;

        // The icon slot: a caller icon, an icon replaced by a spinner while
        // loading, or nothing.
        let icon_slot: Option<Element<'a, Message, Theme>> = if loading {
            Some(loading_indicator(size, self.loading_icon))
        } else {
            self.icon.clone().map(|icon| icon.into_element(size))
        };

        let label: Option<Element<'a, Message, Theme>> =
            self.label.clone().map(|label| self.label_element(&label));

        let children = std::mem::take(&mut self.children);

        let mut parts: Vec<Element<'a, Message, Theme>> = Vec::new();
        if let Some(icon) = icon_slot {
            parts.push(icon);
        }
        if let Some(label) = label {
            parts.push(label);
        }
        parts.extend(children);

        if self.dropdown_caret {
            parts.push(crate::widgets::overlay::caret(size.text().size));
        }

        let content: Element<'a, Message, Theme> = match parts.len() {
            0 => self.loading_placeholder(),
            1 => parts.pop().expect("just checked the length"),
            // A row centres its children on each other, so an icon and a label
            // of different heights line up.
            _ => row(parts)
                .spacing(size.gap())
                .align_y(iced::Alignment::Center)
                .into(),
        };

        content
    }

    /// Converts the button into an [`Element`].
    pub fn into_element(mut self) -> Element<'a, Message, Theme> {
        let class = self.class();
        let padding = self.resolved_padding();
        let size = self.size;
        let width = self.width;
        let height = self.height;
        let tooltip = self.tooltip.take();
        // A loading button is inert: it never emits its message, even if the
        // caller left a handler on it. A disabled one keeps its handler but
        // suppresses it, so iced still reports `Disabled` and draws the
        // disabled style.
        let on_press = self
            .on_press
            .clone()
            .filter(|_| !self.loading && !self.disabled);

        let icon_only = self.is_icon_only();
        let content = self.content();

        let mut widget =
            iced_button(content)
                .padding(padding)
                .class(
                    Box::new(move |theme: &Theme, status| class.into_style_fn(theme)(&status))
                        as iced_button::StyleFn<'a, Theme>,
                );

        // An icon-only button is a square, so a toolbar of them lines up; a
        // labelled one takes its natural width.
        if icon_only {
            let edge = size.height();
            widget = widget
                .width(Length::Fixed(edge))
                .height(Length::Fixed(edge));
        } else {
            widget = widget.height(Length::Fixed(size.height()));
        }

        if let Some(width) = width {
            widget = widget.width(width);
        }
        if let Some(height) = height {
            widget = widget.height(height);
        }
        if let Some(message) = on_press {
            widget = widget.on_press(message);
        }

        let element: Element<'a, Message, Theme> = widget.into();

        match tooltip {
            Some((label, position)) => tooltip_at(element, label, position),
            None => element,
        }
    }
}

impl<'a, Message: Clone + 'a> From<Button<'a, Message>> for Element<'a, Message, Theme> {
    fn from(button: Button<'a, Message>) -> Self {
        button.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{icon_button, Button};
    use crate::theme::catalog::{ButtonRounded, ButtonVariant, Corners};
    use crate::theme::{Size, Theme};
    use iced::{Element, Length};

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        Pressed,
    }

    #[test]
    fn a_button_renders_in_every_variant_and_size() {
        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonVariant::Danger,
            ButtonVariant::Warning,
            ButtonVariant::Success,
            ButtonVariant::Info,
            ButtonVariant::Ghost,
            ButtonVariant::Link,
            ButtonVariant::Text,
            ButtonVariant::Destructive,
        ] {
            for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
                let element: Element<'_, Message, Theme> = Button::new("Save")
                    .variant(variant)
                    .size(size)
                    .on_press(Message::Pressed)
                    .into();
                drop(element);
            }
        }
    }

    #[test]
    fn a_button_renders_outlined_and_selected() {
        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Primary,
            ButtonVariant::Ghost,
        ] {
            for selected in [true, false] {
                let element: Element<'_, Message, Theme> = Button::new("Save")
                    .variant(variant)
                    .outline()
                    .selected(selected)
                    .on_press(Message::Pressed)
                    .into();
                drop(element);
            }
        }
    }

    #[test]
    fn a_loading_button_renders_and_drops_its_press_handler() {
        let button = Button::new("Save")
            .primary()
            .loading(true)
            .on_press(Message::Pressed);

        assert!(!button.is_enabled());

        let element: Element<'_, Message, Theme> = button.into();
        drop(element);
    }

    #[test]
    fn an_enabled_button_keeps_its_handler() {
        let button = Button::new("Save").on_press(Message::Pressed);
        assert!(button.is_enabled());
    }

    #[test]
    fn a_button_without_a_handler_is_disabled_but_still_renders() {
        let button: Button<'_, Message> = Button::new("Save");
        assert!(!button.is_enabled());

        let element: Element<'_, Message, Theme> = button.into();
        drop(element);
    }

    #[test]
    fn builder_methods_are_chainable() {
        let button: Button<'_, Message> = Button::new("Delete")
            .destructive()
            .size(Size::Lg)
            .outline()
            .rounded(ButtonRounded::Large)
            .width(200)
            .on_press_maybe(Some(Message::Pressed));

        let class = button.class();
        assert_eq!(class.variant, ButtonVariant::Destructive);
        assert_eq!(class.size, Size::Lg);
        assert!(class.outline);
        assert_eq!(class.rounded, ButtonRounded::Large);
    }

    /// An icon, a dropdown caret and caller content each have to survive the
    /// trip into the content row; a button that silently dropped one would look
    /// fine in a screenshot of a plain button and wrong everywhere else.
    #[test]
    fn optional_content_renders_together() {
        let element: Element<'_, Message, Theme> = Button::new("Menu")
            .icon("☰")
            .dropdown_caret()
            .push(iced::widget::text("3"))
            .on_press(Message::Pressed)
            .into();
        drop(element);

        let icon_only: Element<'_, Message, Theme> = icon_button::<Message>()
            .icon("✕")
            .on_press(Message::Pressed)
            .into();
        drop(icon_only);
    }

    /// A borderless variant must take no padding at all, and a compact one must
    /// narrow its inset without changing height.
    #[test]
    fn padding_follows_the_variant_and_the_compact_flag() {
        let normal: Button<'_, Message> = Button::new("Save").size(Size::Md);
        let compact: Button<'_, Message> = Button::new("Save").size(Size::Md).compact();
        let link: Button<'_, Message> = Button::new("Save").link();

        assert_eq!(normal.resolved_padding().left, Size::Md.padding());
        assert!(compact.resolved_padding().left < normal.resolved_padding().left);
        assert_eq!(link.resolved_padding().left, 0.0);
        assert_eq!(link.resolved_padding().top, 0.0);
    }

    /// An icon-only button is square; a labelled one is only as tall as its
    /// size asks. This is what makes a toolbar of icons line up.
    #[test]
    fn an_icon_only_button_is_a_square() {
        let plain: Button<'_, Message> = Button::new("Save");
        let icon: Button<'_, Message> = icon_button::<Message>().icon("✕");
        let labelled_icon: Button<'_, Message> = Button::new("Save").icon("✕");

        assert!(!plain.is_icon_only());
        assert!(icon.is_icon_only());
        assert!(
            !labelled_icon.is_icon_only(),
            "a button with both an icon and a label is not icon-only"
        );
    }

    #[test]
    fn a_group_member_can_override_its_corners() {
        let squared: Button<'_, Message> = Button::new("Save").corners(Corners {
            top_left: true,
            top_right: false,
            bottom_right: false,
            bottom_left: true,
        });

        assert!(!squared.class().corners.top_right);
        assert!(squared.class().corners.top_left);
    }

    /// The label's line box has to be as tall as the control, or the glyph sits
    /// against the top of it.
    ///
    /// iced anchors a paragraph by its own glyph bounds — ascender through
    /// descender — so a line box sized to the text alone leaves the label high
    /// in a taller button. This was a visible defect (the label hugged the top
    /// edge while the bottom gap was four times the top), and a pixel-comparison
    /// snapshot cannot catch it: the snapshot only proves the render did not
    /// change, not that it was ever right.
    #[test]
    fn the_label_line_box_matches_the_control_height() {
        for size in [Size::Xs, Size::Sm, Size::Md, Size::Lg] {
            let button: Button<'_, Message> = Button::new("Save").size(size);
            assert_eq!(
                button.resolved_line_height().0,
                size.height(),
                "{size:?}: the label's line box must fill the control"
            );

            // And an explicit height overrides the size's own, so a tall button
            // centres its label too.
            let tall: Button<'_, Message> =
                Button::new("Save").size(size).height(Length::Fixed(64.0));
            assert_eq!(
                tall.resolved_line_height().0,
                64.0,
                "{size:?}: explicit height"
            );
        }
    }

    #[test]
    fn width_and_height_are_optional_overrides() {
        let natural: Button<'_, Message> = Button::new("Save");
        assert_eq!(natural.class().size, Size::Md);

        let sized: Button<'_, Message> = Button::new("Save")
            .width(Length::Fill)
            .height(Length::Fixed(48.0));
        let _: Element<'_, Message, Theme> = sized.into();
    }
}
