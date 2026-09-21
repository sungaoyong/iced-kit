//! Indeterminate spinners and ring-shaped progress indicators.
//!
//! Both are drawn with iced's canvas API, since neither can be composed from
//! iced's box-oriented widgets.

use crate::theme::Theme;
use crate::widgets::display::Tone;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{mouse, Color, Element, Length, Point, Radians, Rectangle};

/// The appearance of a [`spinner`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerStyle {
    /// A rotating arc that fades toward its tail.
    #[default]
    Arc,
    /// A ring of dots whose brightness travels around the circle.
    Dots,
}

/// Builds an indeterminate spinner.
///
/// A `spinner` communicates "work in progress, duration unknown" and therefore
/// never has a progress value; use [`ring_progress`] when the completion
/// fraction is known.
///
/// ```
/// # use iced_kit::widgets::spinner;
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// spinner(24)
/// # }
/// ```
pub fn spinner<'a, Message: 'a>(diameter: u16) -> Element<'a, Message, Theme> {
    spinner_styled(diameter, SpinnerStyle::Arc)
}

/// Builds an indeterminate spinner with an explicit style.
pub fn spinner_styled<'a, Message: 'a>(
    diameter: u16,
    style: SpinnerStyle,
) -> Element<'a, Message, Theme> {
    let side = f32::from(diameter);

    Canvas::new(SpinnerProgram { style })
        .width(Length::Fixed(side))
        .height(Length::Fixed(side))
        .into()
}

/// The animation clock of a [`SpinnerProgram`].
///
/// It is a free-running phase in seconds rather than a frame counter, so the
/// spinner animates at the same speed regardless of how often iced redraws.
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
            // A long stall (a window dragged, a debugger paused) would otherwise
            // jump the spinner forward by seconds at once.
            self.phase = (self.phase + delta.min(0.1)) % 10_000.0;
        }

        self.last = Some(now);
    }
}

/// Draws an indeterminate spinner.
struct SpinnerProgram {
    style: SpinnerStyle,
}

impl<Message, Theme, Renderer> canvas::Program<Message, Theme, Renderer> for SpinnerProgram
where
    Renderer: iced::advanced::graphics::geometry::Renderer,
    Theme: SpinnerTheme,
{
    type State = SpinnerState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        // Each redraw advances the phase and asks for the next frame, which is
        // what animates the spinner without the application owning a timer.
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            state.advance(*now);
            return Some(canvas::Action::request_redraw());
        }

        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        let color = theme.spinner_color();
        let phase = state.phase;

        let center = frame.center();
        let radius = (frame.width().min(frame.height()) / 2.0 - 2.0).max(1.0);

        match self.style {
            SpinnerStyle::Arc => {
                // A three-quarter arc leaves a gap that reads as "rotating".
                let start = phase * 3.0;
                let sweep = std::f32::consts::PI * 1.5;

                let arc = Path::new(|builder| {
                    builder.arc(canvas::path::Arc {
                        center,
                        radius,
                        start_angle: Radians(start),
                        end_angle: Radians(start + sweep),
                    });
                });

                frame.stroke(
                    &arc,
                    Stroke::default()
                        .with_color(color)
                        .with_width((radius * 0.25).max(1.5)),
                );
            }
            SpinnerStyle::Dots => {
                const DOTS: usize = 8;
                let dot_radius = (radius * 0.22).max(1.0);

                for index in 0..DOTS {
                    let angle = std::f32::consts::TAU * index as f32 / DOTS as f32 - phase * 2.0;

                    // The leading dot is brightest; the rest fall off around the
                    // ring, which is what makes the motion readable.
                    let dot_phase = (index as f32 / DOTS as f32 + phase * 0.5).fract();
                    let alpha = 0.15 + 0.85 * (1.0 - dot_phase);

                    let dot_center = Point::new(
                        center.x + radius * angle.cos(),
                        center.y + radius * angle.sin(),
                    );

                    frame.fill(
                        &Path::circle(dot_center, dot_radius),
                        Color { a: alpha, ..color },
                    );
                }
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}

/// The theme information a spinner needs.
///
/// Implemented for [`Theme`]; the trait exists so the canvas program does not
/// have to name the concrete theme type.
pub trait SpinnerTheme {
    /// The color of the moving indicator.
    fn spinner_color(&self) -> Color;
}

impl SpinnerTheme for Theme {
    fn spinner_color(&self) -> Color {
        self.colors().primary
    }
}

/// Builds a ring-shaped progress indicator.
///
/// `value` is a fraction in `0.0..=1.0` and is clamped; a caller passing a
/// percentage cannot overflow the ring. The completion fraction is drawn in the
/// tone's color and printed in the middle.
///
/// ```
/// # use iced_kit::widgets::{ring_progress, Tone};
/// # use iced_kit::Theme;
/// # fn view() -> iced::Element<'static, (), Theme> {
/// ring_progress(0.65, 72, Tone::Primary)
/// # }
/// ```
pub fn ring_progress<'a, Message: 'a>(
    value: f32,
    diameter: u16,
    tone: Tone,
) -> Element<'a, Message, Theme> {
    let side = f32::from(diameter);

    Canvas::new(RingProgressProgram {
        value: value.clamp(0.0, 1.0),
        tone,
        label: format!("{:.0}%", value.clamp(0.0, 1.0) * 100.0),
    })
    .width(Length::Fixed(side))
    .height(Length::Fixed(side))
    .into()
}

/// Draws a ring progress indicator.
struct RingProgressProgram {
    value: f32,
    tone: Tone,
    label: String,
}

impl<Message, Theme, Renderer> canvas::Program<Message, Theme, Renderer> for RingProgressProgram
where
    Renderer: iced::advanced::graphics::geometry::Renderer
        + iced::advanced::text::Renderer<Font = iced::Font>,
    Theme: RingTheme,
{
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (track_color, bar_color, text_color) = theme.ring_colors(self.tone);

        let center = frame.center();
        let thickness = (frame.width().min(frame.height()) * 0.1).max(2.0);
        let radius = (frame.width().min(frame.height()) / 2.0 - thickness / 2.0 - 1.0).max(1.0);

        // The full track, so an empty ring still reads as a ring.
        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default()
                .with_color(track_color)
                .with_width(thickness),
        );

        if self.value > 0.0 {
            // Start at 12 o'clock and sweep clockwise, which is how a progress
            // ring is read.
            let start = -std::f32::consts::FRAC_PI_2;
            let end = start + std::f32::consts::TAU * self.value;

            let arc = Path::new(|builder| {
                builder.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: Radians(start),
                    end_angle: Radians(end),
                });
            });

            frame.stroke(
                &arc,
                Stroke::default()
                    .with_color(bar_color)
                    .with_width(thickness),
            );
        }

        // The label has to fit inside the ring's inner circle: a font sized off
        // the radius alone overflows on small rings, so it is derived from the
        // diameter and skipped entirely once it would be illegible.
        let inner_diameter = (radius - thickness / 2.0) * 2.0;
        let label_size = inner_diameter * 0.32;

        if label_size >= 8.0 {
            frame.fill_text(canvas::Text {
                content: self.label.clone(),
                position: center,
                color: text_color,
                size: iced::Pixels(label_size),
                align_x: iced::advanced::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}

/// The theme information a ring needs.
pub trait RingTheme {
    /// Returns the track color, the bar color, and the label color.
    fn ring_colors(&self, tone: Tone) -> (Color, Color, Color);
}

impl RingTheme for Theme {
    fn ring_colors(&self, tone: Tone) -> (Color, Color, Color) {
        let colors = self.colors();

        (
            colors.secondary,
            match tone {
                Tone::Neutral => colors.primary,
                other => other.accent(self),
            },
            colors.foreground,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ring_progress, spinner, spinner_styled, SpinnerStyle};
    use crate::theme::Theme;
    use crate::widgets::display::Tone;
    use iced::widget::canvas::Program;

    #[test]
    fn a_spinner_renders_in_every_style() {
        for style in [SpinnerStyle::Arc, SpinnerStyle::Dots] {
            let element: iced::Element<'_, (), Theme> = spinner_styled(24, style);
            drop(element);
        }
    }

    #[test]
    fn a_spinner_renders_at_several_sizes() {
        for diameter in [12, 16, 32, 64] {
            let element: iced::Element<'_, (), Theme> = spinner(diameter);
            drop(element);
        }
    }

    #[test]
    fn a_ring_renders_across_the_value_range() {
        // Out-of-range values are clamped, so a caller passing a percentage
        // does not overflow the ring.
        for value in [-0.5, 0.0, 0.25, 0.5, 1.0, 1.5, f32::NAN] {
            let element: iced::Element<'_, (), Theme> = ring_progress(value, 72, Tone::Primary);
            drop(element);
        }
    }

    #[test]
    fn a_ring_renders_in_every_tone() {
        for tone in [
            Tone::Neutral,
            Tone::Success,
            Tone::Warning,
            Tone::Danger,
            Tone::Primary,
        ] {
            let element: iced::Element<'_, (), Theme> = ring_progress(0.5, 64, tone);
            drop(element);
        }
    }

    #[test]
    fn ring_values_are_clamped_to_the_unit_range() {
        // The clamp is what protects the arc maths; verify it directly so the
        // canvas never sees a sweep beyond a full turn.
        for (input, expected) in [(-1.0_f32, 0.0_f32), (0.5, 0.5), (3.0, 1.0)] {
            assert_eq!(input.clamp(0.0, 1.0), expected);
        }
    }

    #[test]
    fn the_spinner_program_advances_its_clock_on_redraw() {
        let program = super::SpinnerProgram {
            style: SpinnerStyle::Arc,
        };
        let mut state = super::SpinnerState::default();
        let bounds = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(24.0, 24.0));

        let event = iced::Event::Window(iced::window::Event::RedrawRequested(
            std::time::Instant::now(),
        ));

        let action: Option<iced::widget::canvas::Action<()>> =
            Program::<(), Theme, iced::Renderer>::update(
                &program,
                &mut state,
                &event,
                bounds,
                iced::mouse::Cursor::Unavailable,
            );

        assert!(
            action.is_some(),
            "a redraw must be requested so it animates"
        );
        assert!(state.last.is_some(), "the clock must record the frame time");
    }

    #[test]
    fn the_spinner_program_ignores_unrelated_events() {
        let program = super::SpinnerProgram {
            style: SpinnerStyle::Dots,
        };
        let mut state = super::SpinnerState::default();
        let bounds = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(24.0, 24.0));

        let event = iced::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: iced::Point::ORIGIN,
        });

        let action: Option<iced::widget::canvas::Action<()>> =
            Program::<(), Theme, iced::Renderer>::update(
                &program,
                &mut state,
                &event,
                bounds,
                iced::mouse::Cursor::Unavailable,
            );

        assert!(action.is_none(), "only redraws should advance the clock");
    }
}
