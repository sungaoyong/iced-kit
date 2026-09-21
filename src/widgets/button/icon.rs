//! The icon a button can carry.
//!
//! An icon has to be colored explicitly: iced's own `svg` widget reads its
//! color from the theme, not from the enclosing button, so an icon drawn
//! through that widget would ignore the button's variant. An icon drawn here
//! goes through a small private wrapper that takes the color from the
//! renderer's inherited style instead, which is exactly what the button sets.

use crate::theme::{Size, Theme};
use iced::advanced::widget::tree;
use iced::advanced::{layout, mouse, renderer, svg, Clipboard, Widget};
use iced::widget::container;
use iced::{Element, Length, Rectangle, Size as IcedSize};

/// What an icon is drawn from.
#[derive(Debug, Clone, PartialEq)]
pub enum IconSource {
    /// A vector image, tinted with the surrounding text color.
    Svg(svg::Handle),
    /// A text glyph, such as an emoji or a symbol from an icon font.
    Glyph(String),
}

impl From<svg::Handle> for IconSource {
    fn from(handle: svg::Handle) -> Self {
        Self::Svg(handle)
    }
}

impl From<&str> for IconSource {
    fn from(glyph: &str) -> Self {
        Self::Glyph(glyph.to_owned())
    }
}

impl From<String> for IconSource {
    fn from(glyph: String) -> Self {
        Self::Glyph(glyph)
    }
}

impl From<svg::Handle> for Icon {
    fn from(handle: svg::Handle) -> Self {
        Self::new(handle)
    }
}

impl From<&str> for Icon {
    fn from(glyph: &str) -> Self {
        Self::new(glyph)
    }
}

impl From<String> for Icon {
    fn from(glyph: String) -> Self {
        Self::new(glyph)
    }
}

impl From<&String> for Icon {
    fn from(glyph: &String) -> Self {
        Self::new(glyph.as_str())
    }
}

/// An icon placed beside a button's label, or standing in for it.
#[must_use = "an Icon does nothing unless it is given to a Button"]
#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    source: IconSource,
    size: Option<f32>,
}

impl Icon {
    /// Creates an icon from an SVG handle or a text glyph.
    pub fn new(source: impl Into<IconSource>) -> Self {
        Self {
            source: source.into(),
            size: None,
        }
    }

    /// Sets an explicit side length, overriding the size's own icon metric.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// The side length this icon draws at, given a control size.
    #[must_use]
    pub fn resolved_size(&self, control: Size) -> f32 {
        self.size.unwrap_or_else(|| control.icon_size())
    }

    /// Renders the icon, taking its color from the enclosing control.
    pub fn into_element<'a, Message: 'a>(self, control: Size) -> Element<'a, Message, Theme> {
        let side = self.resolved_size(control);

        match self.source {
            IconSource::Svg(handle) => TintedIcon { handle, side }.into(),
            IconSource::Glyph(glyph) => {
                // A glyph advances by its own metrics, so it is centered in a
                // square box to keep icon-only buttons from shifting.
                //
                // The line height is wrapped in `Pixels` because iced reads a
                // bare `f32` as a *relative* multiple of the font size, which
                // would inflate this box to hundreds of pixels and push the
                // whole button apart.
                container(
                    iced::widget::text(glyph)
                        .size(side)
                        .line_height(iced::Pixels(side)),
                )
                .width(Length::Fixed(side))
                .height(Length::Fixed(side))
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into()
            }
        }
    }
}

/// An SVG drawn in the color the surrounding widget set.
///
/// iced's `svg::Svg` takes its color from the theme's `svg::Catalog`, which
/// cannot know which button variant it is inside. This widget reads
/// `renderer::Style::text_color` instead — the same value a `text` widget
/// inherits — so an icon and the label beside it always agree.
#[derive(Debug, Clone, PartialEq)]
struct TintedIcon {
    handle: svg::Handle,
    side: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for TintedIcon
where
    Renderer: iced::advanced::Renderer + iced::advanced::svg::Renderer,
{
    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(Length::Fixed(self.side), Length::Fixed(self.side))
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fixed(self.side), Length::Fixed(self.side))
    }

    fn draw(
        &self,
        _tree: &tree::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !bounds.intersects(viewport) {
            return;
        }

        renderer.draw_svg(
            svg::Svg {
                handle: self.handle.clone(),
                color: Some(style.text_color),
                rotation: iced::Radians(0.0),
                opacity: 1.0,
            },
            bounds,
            *viewport,
        );
    }
}

impl<'a, Message, Theme, Renderer> From<TintedIcon> for Element<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer + iced::advanced::svg::Renderer + 'a,
{
    fn from(icon: TintedIcon) -> Self {
        Element::new(icon)
    }
}

/// What a button shows in place of its icon while it is loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadingIcon {
    /// The rotating arc that matches the rest of the library.
    #[default]
    Spinner,
    /// No indicator: the button is merely dimmed.
    None,
}

/// Renders a loading indicator at the size an icon of this control would take.
///
/// The spinner is drawn here rather than reusing the theme's [`spinner`] widget
/// for the same reason the SVG icon is: a canvas program reads its color from
/// the theme, so it would paint the theme's primary into a primary-filled
/// button and vanish. Inheriting the renderer's text color keeps the indicator
/// visible on every variant.
///
/// [`spinner`]: crate::widgets::spinner
pub(crate) fn loading_indicator<'a, Message: 'a>(
    control: Size,
    icon: LoadingIcon,
) -> Element<'a, Message, Theme> {
    match icon {
        LoadingIcon::None => iced::widget::Space::new()
            .width(Length::Fixed(control.icon_size()))
            .height(Length::Fixed(control.icon_size()))
            .into(),
        LoadingIcon::Spinner => SpinnerIcon {
            side: control.icon_size(),
        }
        .into(),
    }
}

/// A rotating arc, drawn in the color the surrounding control set.
///
/// The arc's rotation, accumulated from the redraw timestamps.
///
/// It is a phase in radians rather than a function of the wall clock, matching
/// the standalone spinner: a clock-derived angle would render differently on
/// every run, which a snapshot test cannot pin down. Starting at zero also
/// makes the first frame reproducible.
#[derive(Debug, Default)]
struct SpinnerState {
    phase: f32,
    last: Option<iced::time::Instant>,
}

impl SpinnerState {
    /// Advances the phase to the given instant.
    fn advance(&mut self, now: iced::time::Instant) {
        if let Some(last) = self.last {
            let delta = now.duration_since(last).as_secs_f32();
            // A long stall — a window dragged, a debugger paused — would
            // otherwise jump the arc forward by seconds at once.
            let turn = std::f32::consts::TAU;
            self.phase = (self.phase + delta.min(0.1) * turn) % turn;
        }

        self.last = Some(now);
    }
}

/// A rotating arc drawn where an icon would go.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SpinnerIcon {
    side: f32,
}

impl SpinnerIcon {
    /// The fraction of the circle the arc covers.
    const SWEEP: f32 = std::f32::consts::PI * 1.5;
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for SpinnerIcon
where
    Renderer: iced::advanced::Renderer + iced::advanced::graphics::geometry::Renderer,
{
    fn size(&self) -> IcedSize<Length> {
        IcedSize::new(Length::Fixed(self.side), Length::Fixed(self.side))
    }

    fn layout(
        &mut self,
        _tree: &mut tree::Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, Length::Fixed(self.side), Length::Fixed(self.side))
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<SpinnerState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(SpinnerState::default())
    }

    fn update(
        &mut self,
        tree: &mut tree::Tree,
        event: &iced::Event,
        _layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        // Each redraw advances the phase and asks for the next frame, which is
        // what animates the arc without the application owning a timer.
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            tree.state.downcast_mut::<SpinnerState>().advance(*now);
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &tree::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        if !bounds.intersects(viewport) {
            return;
        }

        let phase = tree.state.downcast_ref::<SpinnerState>().phase;

        // `Frame::new` builds a canvas whose origin is the widget's own
        // top-left, so the center is expressed in that local space and the whole
        // geometry is translated into place below. Using the absolute
        // `bounds.center()` instead would draw the arc at the window's corner.
        let center = iced::Point::new(self.side / 2.0, self.side / 2.0);
        let radius = (self.side / 2.0 - 1.5).max(1.0);
        // iced's `Arc` takes standard math angles, counter-clockwise from three
        // o'clock, so the accumulated phase is rotated a quarter turn here.
        let start = phase - std::f32::consts::FRAC_PI_2;
        let arc = iced::widget::canvas::Path::new(|builder| {
            builder.arc(iced::widget::canvas::path::Arc {
                center,
                radius,
                start_angle: iced::Radians(start),
                end_angle: iced::Radians(start + Self::SWEEP),
            });
        });

        let mut frame = iced::widget::canvas::Frame::new(renderer, bounds.size());
        frame.stroke(
            &arc,
            iced::widget::canvas::Stroke::default()
                .with_color(style.text_color)
                .with_width((radius * 0.28).max(1.5)),
        );

        // A canvas widget translates its geometry by its own position before
        // handing it to the renderer; a raw widget has to do the same, or the
        // arc lands at the window's origin.
        renderer.with_translation(iced::Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(frame.into_geometry());
        });
    }
}

impl<'a, Message, Theme, Renderer> From<SpinnerIcon> for Element<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer + iced::advanced::graphics::geometry::Renderer + 'a,
{
    fn from(icon: SpinnerIcon) -> Self {
        Element::new(icon)
    }
}
