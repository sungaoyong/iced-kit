//! Motion: easing curves, a spring solver, and the progress helper that
//! components animate with.
//!
//! iced already ships the clock this module builds on: [`iced::animation`] holds
//! a duration, an easing, a delay and a repetition count, and interpolates
//! between two states at a given [`Instant`]. What it does not ship is anything
//! that uses it — no widget in iced animates — so this module supplies the three
//! pieces a component needs on top:
//!
//! - The three curves the design language is specified in. iced's own easing
//!   names are a different set of control points from the ones this crate's
//!   palette uses, so the curves are solved here rather than borrowed.
//! - [`Spring`], for values whose target changes faster than the motion
//!   completes. A duration-based transition restarts its easing from wherever
//!   the value is, which is continuous in position but not in velocity, so a
//!   reversal reads as a snap; a spring carries velocity across the change.
//! - [`Progress`], a single value moving between two states, which is the shape
//!   every component here animates.
//!
//! # What an animation may touch
//!
//! iced's renderer has no alpha. Its [`Renderer`] trait exposes clipping
//! (`with_layer`), a linear transform (`with_translation`), and `fill_quad` —
//! there is no opacity anywhere in the graphics stack, and the `Style` a widget
//! passes to its children carries only a text color. So a component in this
//! crate can move and size what it draws, and it can fade the colors it
//! constructs itself (backgrounds, borders, shadows, scrims), but it cannot fade
//! an arbitrary `Element` the caller handed it. Enter animations therefore
//! translate and clip; where a fade is wanted it is applied to the component's
//! own chrome.
//!
//! [`Renderer`]: iced::advanced::Renderer

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use iced::animation::{Animation, Easing};
use iced::time::Instant;

mod presence;

pub use presence::{Presence, DURATION_EXIT};

/// No time at all: the value is adopted on the spot.
pub const DURATION_INSTANT: Duration = Duration::ZERO;

/// A color or a small state change, at 120ms.
pub const DURATION_FAST: Duration = Duration::from_millis(120);

/// The default for a value that moves or grows, at 180ms.
pub const DURATION_NORMAL: Duration = Duration::from_millis(180);

/// A large surface arriving or leaving, at 280ms.
pub const DURATION_SLOW: Duration = Duration::from_millis(280);

/// How far a small element travels as it arrives, in logical pixels.
pub const DISTANCE_SHORT: f32 = 4.0;

/// How far a surface travels as it arrives, in logical pixels.
pub const DISTANCE_MEDIUM: f32 = 8.0;

/// Whether the application has asked for reduced motion.
static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);

/// Returns whether motion should be skipped.
///
/// When this is set, every animation in the crate adopts its target
/// immediately instead of easing towards it, and stops asking for frames.
///
/// iced exposes no way to read the operating system's reduced-motion
/// preference — there is no accessibility API in 0.14 — so this is a flag the
/// application sets, having read the preference itself however it can. It
/// defaults to `false`, which is the behavior of an application that never
/// mentions it.
#[must_use]
pub fn reduce_motion() -> bool {
    REDUCE_MOTION.load(Ordering::Relaxed)
}

/// Asks every animation in the crate to skip its motion.
///
/// See [`reduce_motion`] for why the application has to be the one to decide.
pub fn set_reduce_motion(reduce: bool) {
    REDUCE_MOTION.store(reduce, Ordering::Relaxed);
}

/// The easing that applies to something arriving.
///
/// A strong ease-out: the value covers most of its distance early and settles
/// gently, which is what makes an appearance read as responsive rather than
/// slow. The control points are `cubic-bezier(0.16, 1, 0.3, 1)`.
#[must_use]
pub fn ease_enter(t: f32) -> f32 {
    cubic_bezier(0.16, 1.0, 0.3, 1.0, t)
}

/// The easing that applies to something leaving.
///
/// `cubic-bezier(0.4, 0, 1, 1)`: it starts at full speed, so a dismissal does
/// not linger.
#[must_use]
pub fn ease_exit(t: f32) -> f32 {
    cubic_bezier(0.4, 0.0, 1.0, 1.0, t)
}

/// The easing that applies to a value moving within the interface.
///
/// `cubic-bezier(0.2, 0, 0, 1)`: symmetric enough that a value moving either way
/// looks the same, which is what keeps a slider, an indicator or a progress bar
/// from appearing to favor one direction.
#[must_use]
pub fn ease_move(t: f32) -> f32 {
    cubic_bezier(0.2, 0.0, 0.0, 1.0, t)
}

/// Samples a cubic Bézier easing curve, in CSS's control-point form.
///
/// The curve runs from `(0, 0)` to `(1, 1)` with the two given control points.
/// `t` is elapsed progress along the x axis, not the curve's own parameter, so
/// the curve is first solved for the parameter whose x is `t` and only then
/// sampled for y. Skipping that solve is the classic mistake: it makes a curve
/// read much slower than the same control points do in CSS, because x is not
/// linear in the parameter.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }

    if t >= 1.0 {
        return 1.0;
    }

    let s = solve_for_x(x1, x2, t);

    // Clamped because a curve with a control point outside the unit square —
    // and the antisymmetric ones used here are inside it, but a caller's need
    // not be — can otherwise return a value outside the range its own endpoints
    // promise.
    bezier_axis(y1, y2, s).clamp(0.0, 1.0)
}

/// One axis of a cubic Bézier from 0 to 1 with the given control values.
fn bezier_axis(p1: f32, p2: f32, s: f32) -> f32 {
    let inverse = 1.0 - s;

    3.0 * inverse * inverse * s * p1 + 3.0 * inverse * s * s * p2 + s * s * s
}

/// The derivative of [`bezier_axis`], for the Newton step below.
fn bezier_axis_slope(p1: f32, p2: f32, s: f32) -> f32 {
    let inverse = 1.0 - s;

    3.0 * inverse * inverse * p1 + 6.0 * inverse * s * (p2 - p1) + 3.0 * s * s * (1.0 - p2)
}

/// Finds the curve parameter whose x is `x`.
///
/// Newton's method converges in a few steps for the shallow curves an easing
/// uses, and bisection finishes the job when it cannot: the derivative vanishes
/// at the endpoints when a control point sits on them, which is exactly the case
/// `cubic-bezier(0, 0, ...)` describes.
fn solve_for_x(x1: f32, x2: f32, x: f32) -> f32 {
    let mut s = x;

    for _ in 0..8 {
        let error = bezier_axis(x1, x2, s) - x;

        if error.abs() < 1e-6 {
            return s;
        }

        let slope = bezier_axis_slope(x1, x2, s);

        if slope.abs() < 1e-6 {
            break;
        }

        s -= error / slope;
    }

    let mut low = 0.0_f32;
    let mut high = 1.0_f32;
    let mut s = x.clamp(0.0, 1.0);

    for _ in 0..32 {
        let value = bezier_axis(x1, x2, s);

        if (value - x).abs() < 1e-6 {
            return s;
        }

        if value < x {
            low = s;
        } else {
            high = s;
        }

        s = f32::midpoint(low, high);
    }

    s
}

/// A physical spring, for values that are retargeted while they are still
/// moving.
///
/// A transition restarts its easing from the value sampled when the target
/// changed, which is continuous in position but not in velocity — so a value
/// reversed mid-flight visibly changes speed at the moment it turns around. A
/// spring carries velocity across the change, so it decelerates and turns.
///
/// Prefer a spring where the target changes faster than the motion completes: a
/// value following a pointer, a panel toggled again mid-slide, an indicator
/// chasing rapid selection. Prefer a duration for a target that is set once and
/// runs to completion, which is steadier and cheaper.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    response: Duration,
    damping: f32,
    epsilon: f32,
}

impl Spring {
    /// Builds a spring that reaches its target in about `response` without
    /// overshooting it.
    ///
    /// `response` is not a duration with an end that is scheduled; it is the
    /// period one full oscillation would take, which sets the scale the motion
    /// is felt at. The damping of one means the value approaches the target and
    /// stops, which is what almost every value in an interface wants: an
    /// opacity, a measured height, or anything bounded by the geometry around it
    /// has nowhere to overshoot to.
    #[must_use]
    pub const fn new(response: Duration) -> Self {
        Self {
            response,
            damping: 1.0,
            epsilon: 0.001,
        }
    }

    /// Sets the damping ratio.
    ///
    /// Below one the value passes its target and comes back. That is only
    /// appropriate where overshoot is the intended effect — a value that is
    /// meant to feel physical — because it also overshoots out of whatever
    /// bounds the value is drawn within.
    #[must_use]
    pub const fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }

    /// Sets the period the spring is scaled to.
    #[must_use]
    pub const fn with_response(mut self, response: Duration) -> Self {
        self.response = response;
        self
    }

    /// Sets how close to the target counts as arrived, in the target's own
    /// units.
    ///
    /// A value measured in pixels should coarsen this — a tenth of a pixel —
    /// so the motion ends when the travel left is sub-pixel rather than running
    /// frames that change nothing anyone can see. A normalized value keeps the
    /// default.
    #[must_use]
    pub const fn with_epsilon(mut self, epsilon: f32) -> Self {
        self.epsilon = epsilon;
        self
    }

    /// Returns the period the spring is scaled to.
    #[must_use]
    pub const fn response(self) -> Duration {
        self.response
    }

    /// Returns the damping ratio.
    #[must_use]
    pub const fn damping(self) -> f32 {
        self.damping
    }

    /// Returns the settling tolerance.
    #[must_use]
    pub const fn epsilon(self) -> f32 {
        self.epsilon
    }

    /// The undamped angular frequency the response corresponds to.
    fn angular_frequency(self) -> f32 {
        let seconds = self.response.as_secs_f32();

        if seconds <= 0.0 {
            // A spring with no period has no motion to integrate; the caller
            // adopts the target instead of dividing by zero.
            return 0.0;
        }

        std::f32::consts::TAU / seconds
    }
}

/// The position and velocity of a value driven by a [`Spring`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpringState {
    value: f32,
    velocity: f32,
}

impl SpringState {
    /// Starts a value at rest.
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
        }
    }

    /// Returns the current value.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.value
    }

    /// Returns the current velocity, in value units per second.
    #[must_use]
    pub const fn velocity(self) -> f32 {
        self.velocity
    }

    /// Puts the value somewhere at rest, discarding any momentum.
    pub fn set(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
    }

    /// Advances the value towards `target`.
    ///
    /// The step is the closed-form solution of a damped harmonic oscillator
    /// rather than a numerical integration, so the value reached after a given
    /// elapsed time does not depend on how many frames were drawn along the
    /// way. A dropped frame changes how often the result is painted, not the
    /// result.
    pub fn step(&mut self, target: f32, spring: Spring, elapsed: Duration) {
        let angular_frequency = spring.angular_frequency();

        if angular_frequency <= 0.0 {
            self.set(target);
            return;
        }

        let damping = spring.damping.max(0.0);
        let t = elapsed.as_secs_f32().max(0.0);
        let displacement = self.value - target;
        let velocity = self.velocity;

        let (offset, speed) = if damping < 1.0 {
            // Under-damped: the value passes the target and oscillates back.
            let damped = angular_frequency * (1.0 - damping * damping).sqrt();
            let decay = (-damping * angular_frequency * t).exp();
            let (sin, cos) = (damped * t).sin_cos();
            let amplitude = displacement;
            let slope = (velocity + damping * angular_frequency * displacement) / damped;

            (
                decay * (amplitude * cos + slope * sin),
                decay
                    * ((slope * damped - damping * angular_frequency * amplitude) * cos
                        - (amplitude * damped + damping * angular_frequency * slope) * sin),
            )
        } else if (damping - 1.0).abs() < f32::EPSILON {
            // Critically damped: the fastest approach that does not overshoot.
            let decay = (-angular_frequency * t).exp();
            let slope = velocity + angular_frequency * displacement;

            (
                decay * (displacement + slope * t),
                decay * (slope - angular_frequency * (displacement + slope * t)),
            )
        } else {
            // Over-damped: two real roots, no oscillation, slower than critical.
            let root = angular_frequency * (damping * damping - 1.0).sqrt();
            let slow = -damping * angular_frequency + root;
            let fast = -damping * angular_frequency - root;
            let first = (velocity - fast * displacement) / (slow - fast);
            let second = displacement - first;
            let (slow_decay, fast_decay) = ((slow * t).exp(), (fast * t).exp());

            (
                first * slow_decay + second * fast_decay,
                first * slow * slow_decay + second * fast * fast_decay,
            )
        };

        self.value = target + offset;
        self.velocity = speed;

        // Snap once the motion is smaller than anyone can see, so a settled
        // value compares equal to its target and stops requesting frames.
        if self.is_settled(target, spring) {
            self.set(target);
        }
    }

    /// Returns whether the value has arrived and come to rest.
    ///
    /// Both the distance left and the speed are tested: a value passing through
    /// its target at speed is momentarily on it, and stopping there would drop
    /// the overshoot the spring exists to produce.
    #[must_use]
    pub fn is_settled(self, target: f32, spring: Spring) -> bool {
        (self.value - target).abs() < spring.epsilon
            && self.velocity.abs() < spring.epsilon * spring.angular_frequency()
    }
}

/// A free-running phase, advanced by the redraw timestamps.
///
/// An indeterminate animation — a spinner, a shimmer — has no target and no
/// end; it is a phase that keeps moving for as long as it is drawn. This holds
/// that phase and the frame time it was last advanced to.
///
/// The phase accumulates elapsed time rather than counting frames, so the
/// animation runs at the same speed whatever rate iced redraws at. Starting at
/// zero also makes the first frame reproducible, which a clock-derived angle
/// would not be — a snapshot test cannot pin down a value read from the wall
/// clock.
#[derive(Debug, Default)]
pub(crate) struct Clock {
    phase: f32,
    last: Option<Instant>,
}

impl Clock {
    /// Advances the phase to `now` and returns it.
    pub(crate) fn advance(&mut self, now: Instant) -> f32 {
        if let Some(last) = self.last {
            let delta = now.duration_since(last).as_secs_f32();
            // A long stall — a window dragged, a debugger paused — would
            // otherwise jump the animation forward by seconds at once.
            self.phase = (self.phase + delta.min(0.1)) % 10_000.0;
        }

        self.last = Some(now);
        self.phase
    }

    /// Returns the phase without advancing it.
    pub(crate) fn phase(&self) -> f32 {
        self.phase
    }
}

/// A value moving between two states, animated.
///
/// This is the shape nearly every component needs: something appears or
/// disappears, opens or closes, is selected or is not, and the transition is
/// what makes the change legible. The value runs from `0.0` when the state is
/// off to `1.0` when it is on, so it can be read directly as a fraction, an
/// opacity, or a distance travelled.
///
/// # Why the timing is fixed at construction
///
/// iced's [`Animation`] takes its duration and easing through consuming
/// builders, so retiming one means rebuilding it — and rebuilding loses the
/// position it had reached, which would show as a jump. The timing is therefore
/// chosen where the value is created and left alone; a component that wants
/// something other than [`DURATION_NORMAL`] and [`ease_move`] passes it there.
///
/// [`Spring`], by contrast, takes its timing per step, so a spring-driven value
/// can follow the theme from frame to frame. Use a spring when that matters.
#[derive(Debug, Clone)]
pub(crate) struct Progress {
    animation: Animation<bool>,
    target: bool,
}

impl Progress {
    /// Starts a value at rest in the given state.
    pub(crate) fn new(target: bool) -> Self {
        Self::with_timing(target, DURATION_NORMAL, ease_move)
    }

    /// Starts a value at rest with explicit timing.
    ///
    /// The duration is used as given. Whether motion is wanted at all is the
    /// caller's decision — a widget consults [`reduce_motion`] where it starts a
    /// transition, so this stays a plain constructor with no global state in it.
    pub(crate) fn with_timing(target: bool, duration: Duration, easing: fn(f32) -> f32) -> Self {
        Self {
            animation: Animation::new(target)
                .duration(duration)
                .easing(Easing::Custom(easing)),
            target,
        }
    }

    /// Moves the value towards `target`, starting the motion at `at`.
    ///
    /// A target change while the value is still moving continues from wherever
    /// it has reached rather than restarting from the previous endpoint, which
    /// is what keeps a rapid toggle from producing a jump.
    pub(crate) fn set_target(&mut self, target: bool, at: Instant) {
        if self.target == target {
            return;
        }

        self.target = target;
        self.animation.go_mut(target, at);
    }

    /// Returns how far the value has travelled, from `0.0` to `1.0`.
    #[must_use]
    pub(crate) fn value(&self, at: Instant) -> f32 {
        self.animation.interpolate(0.0, 1.0, at).clamp(0.0, 1.0)
    }

    /// Returns whether the value is still moving.
    ///
    /// A component uses this to keep asking for frames. It goes false once the
    /// motion ends, so nothing redraws forever.
    #[must_use]
    pub(crate) fn is_animating(&self, at: Instant) -> bool {
        self.target != self.animation.value() || self.animation.is_animating(at)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ease_enter, ease_exit, ease_move, reduce_motion, Spring, SpringState, DURATION_FAST,
    };
    use std::time::Duration;

    /// A curve must start at 0 and end at 1 exactly, or a value animated by it
    /// never quite arrives.
    #[test]
    fn the_curves_hold_their_endpoints() {
        for curve in [ease_enter, ease_exit, ease_move] {
            assert_eq!(curve(0.0), 0.0);
            assert_eq!(curve(1.0), 1.0);
            assert_eq!(curve(-1.0), 0.0, "a curve must clamp below its range");
            assert_eq!(curve(2.0), 1.0, "a curve must clamp above its range");
        }
    }

    #[test]
    fn the_curves_advance_without_going_backwards() {
        for curve in [ease_enter, ease_exit, ease_move] {
            let mut previous = 0.0;

            for step in 0..=100 {
                let value = curve(step as f32 / 100.0);

                assert!(
                    value >= previous,
                    "a curve must not reverse: {value} followed {previous}"
                );
                previous = value;
            }
        }
    }

    /// An ease-out covers more than half its distance by the halfway point,
    /// while an ease-in covers less. This is what distinguishes them; a curve
    /// solver that skipped the x-solve would put them on the wrong side.
    #[test]
    fn the_curves_have_the_shape_their_names_claim() {
        assert!(
            ease_enter(0.5) > 0.5,
            "an entering value must lead its progress"
        );
        assert!(
            ease_exit(0.5) < 0.5,
            "a leaving value must lag its progress"
        );
    }

    /// Two springs with different damping must differ, or the parameter is
    /// being ignored.
    #[test]
    fn damping_changes_the_motion() {
        let firm = Spring::new(Duration::from_millis(200));
        let loose = firm.with_damping(0.4);

        let mut firm_state = SpringState::new(0.0);
        let mut loose_state = SpringState::new(0.0);

        let step = Duration::from_millis(10);
        let mut firm_peak: f32 = 0.0;
        let mut loose_peak: f32 = 0.0;

        for _ in 0..100 {
            firm_state.step(1.0, firm, step);
            loose_state.step(1.0, loose, step);

            firm_peak = firm_peak.max(firm_state.value());
            loose_peak = loose_peak.max(loose_state.value());
        }

        assert!(
            firm_peak <= 1.0 + f32::EPSILON,
            "a critically damped spring must not overshoot: reached {firm_peak}"
        );
        assert!(
            loose_peak > 1.0,
            "an under-damped spring must overshoot: only reached {loose_peak}"
        );
    }

    /// The whole point of solving the oscillator in closed form: the value at a
    /// given time does not depend on how many frames were drawn to get there.
    #[test]
    fn the_spring_does_not_depend_on_the_frame_rate() {
        let spring = Spring::new(Duration::from_millis(200));

        let mut coarse = SpringState::new(0.0);
        coarse.step(1.0, spring, Duration::from_millis(100));

        let mut fine = SpringState::new(0.0);
        for _ in 0..10 {
            fine.step(1.0, spring, Duration::from_millis(10));
        }

        let difference = (coarse.value() - fine.value()).abs();

        assert!(
            difference < 1e-4,
            "one long frame must reach the same value as many short ones, \
             but {} differed from {}",
            coarse.value(),
            fine.value()
        );
    }

    #[test]
    fn a_spring_settles_and_stops() {
        let spring = Spring::new(Duration::from_millis(180));

        let mut state = SpringState::new(0.0);
        let step = Duration::from_millis(16);

        for _ in 0..200 {
            state.step(1.0, spring, step);
        }

        assert_eq!(state.value(), 1.0, "the spring must reach its target");
        assert_eq!(state.velocity(), 0.0, "a settled value has no momentum");
        assert!(state.is_settled(1.0, spring));
    }

    /// A spring with no period has nothing to integrate; it must adopt the
    /// target rather than divide by zero.
    #[test]
    fn a_spring_with_no_response_adopts_the_target() {
        let spring = Spring::new(Duration::ZERO);

        let mut state = SpringState::new(0.0);
        state.step(1.0, spring, Duration::from_millis(16));

        assert_eq!(state.value(), 1.0);
    }

    /// A settled value must report settled at every damping, or a component
    /// would keep requesting frames forever.
    #[test]
    fn every_damping_settles_eventually() {
        for damping in [0.3, 0.7, 1.0, 1.5, 3.0] {
            let spring = Spring::new(Duration::from_millis(120)).with_damping(damping);
            let mut state = SpringState::new(0.0);
            let step = Duration::from_millis(16);
            let mut steps = 0;

            while !state.is_settled(1.0, spring) {
                state.step(1.0, spring, step);
                steps += 1;

                assert!(steps < 1000, "a spring of damping {damping} never settled");
            }
        }
    }

    #[test]
    fn the_spring_defaults_are_the_documented_ones() {
        let spring = Spring::new(DURATION_FAST);

        assert_eq!(spring.response(), DURATION_FAST);
        assert_eq!(spring.damping(), 1.0, "the default must not overshoot");
        assert_eq!(spring.epsilon(), 0.001);
    }

    /// The flag starts off, which is what an application that never mentions it
    /// gets. It is deliberately not toggled here: the flag is process-wide and
    /// the tests run in parallel, so flipping it would change what every other
    /// test in the binary renders.
    #[test]
    fn motion_is_not_reduced_unless_asked() {
        assert!(
            !reduce_motion(),
            "an application that never asks for reduced motion must get motion"
        );
    }
}
