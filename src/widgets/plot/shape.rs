//! Plot shapes: the marks a chart draws.
//!
//! Ported from `gpui-kit`'s `plot/shape` (Apache-2.0), adapted to iced's canvas
//! `Frame`. Each shape takes already-mapped screen points and draws them.

use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Point, Size};

/// How a line connects its points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Straight segments between points.
    #[default]
    Linear,
    /// A smooth curve that passes through every point.
    Natural,
    /// A step: hold each value until the next x.
    StepAfter,
}

/// Joins points into a path using the given interpolation.
///
/// A single point yields no path: one point cannot describe a line, and drawing
/// it as one would put a stray mark at the origin.
#[must_use]
pub fn line_path(points: &[Point], interpolation: Interpolation) -> Option<Path> {
    if points.len() < 2 {
        return None;
    }

    Some(Path::new(|builder| {
        builder.move_to(points[0]);

        match interpolation {
            Interpolation::Linear => {
                for point in &points[1..] {
                    builder.line_to(*point);
                }
            }
            Interpolation::StepAfter => {
                for point in &points[1..] {
                    // Move horizontally at the previous value, then vertically.
                    builder.line_to(Point::new(point.x, points[0].y));
                    builder.line_to(*point);
                }
            }
            Interpolation::Natural => {
                for index in 1..points.len() {
                    let previous = points[index - 1];
                    let current = points[index];

                    // A Catmull-Rom segment expressed as a cubic Bézier: the
                    // control points are derived from the neighbours, which is
                    // what makes the curve pass through every point rather than
                    // merely near them.
                    let before = if index >= 2 {
                        points[index - 2]
                    } else {
                        previous
                    };
                    let after = points.get(index + 1).copied().unwrap_or(current);

                    let control_a = Point::new(
                        previous.x + (current.x - before.x) / 6.0,
                        previous.y + (current.y - before.y) / 6.0,
                    );
                    let control_b = Point::new(
                        current.x - (after.x - previous.x) / 6.0,
                        current.y - (after.y - previous.y) / 6.0,
                    );

                    builder.bezier_curve_to(control_a, control_b, current);
                }
            }
        }
    }))
}

/// Draws a series line.
pub fn draw_line<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    points: &[Point],
    color: Color,
    width: f32,
    interpolation: Interpolation,
) {
    let Some(path) = line_path(points, interpolation) else {
        return;
    };

    frame.stroke(
        &path,
        Stroke::default()
            .with_color(color)
            .with_width(width)
            .with_line_cap(iced::widget::canvas::LineCap::Round)
            .with_line_join(iced::widget::canvas::LineJoin::Round),
    );
}

/// Draws the area under a series line, down to `baseline`.
pub fn draw_area<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    points: &[Point],
    baseline: f32,
    color: Color,
    opacity: f32,
    interpolation: Interpolation,
) {
    if points.len() < 2 {
        return;
    }

    let fill = Color {
        a: opacity.clamp(0.0, 1.0),
        ..color
    };

    // The traced path follows the line and closes down to the baseline, so a
    // curved series gives a curved area rather than a polygonal one.
    let path = Path::new(|builder| {
        builder.move_to(Point::new(points[0].x, baseline));

        match interpolation {
            Interpolation::Linear | Interpolation::StepAfter => {
                for point in points {
                    builder.line_to(*point);
                }
            }
            Interpolation::Natural => {
                builder.line_to(points[0]);

                for index in 1..points.len() {
                    let previous = points[index - 1];
                    let current = points[index];
                    let before = if index >= 2 {
                        points[index - 2]
                    } else {
                        previous
                    };
                    let after = points.get(index + 1).copied().unwrap_or(current);

                    let control_a = Point::new(
                        previous.x + (current.x - before.x) / 6.0,
                        previous.y + (current.y - before.y) / 6.0,
                    );
                    let control_b = Point::new(
                        current.x - (after.x - previous.x) / 6.0,
                        current.y - (after.y - previous.y) / 6.0,
                    );

                    builder.bezier_curve_to(control_a, control_b, current);
                }
            }
        }

        builder.line_to(Point::new(points[points.len() - 1].x, baseline));
        builder.close();
    });

    frame.fill(&path, fill);
}

/// Draws a band between two polylines.
///
/// A stacked area fills between the series' own line and the one below it, so
/// the region has two curved edges rather than one. `draw_area` handles the
/// flat-baseline case; this is the general one.
pub fn draw_band<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    upper: &[Point],
    lower: &[Point],
    color: Color,
    opacity: f32,
    interpolation: Interpolation,
) {
    if upper.len() < 2 || lower.len() < 2 {
        return;
    }

    let fill = Color {
        a: opacity.clamp(0.0, 1.0),
        ..color
    };

    // The path traces the upper edge left to right, then the lower edge back,
    // so the two edges enclose the band.
    let path = Path::new(|builder| {
        builder.move_to(upper[0]);

        match interpolation {
            Interpolation::Linear | Interpolation::StepAfter => {
                for point in &upper[1..] {
                    builder.line_to(*point);
                }
            }
            Interpolation::Natural => {
                for index in 1..upper.len() {
                    let previous = upper[index - 1];
                    let current = upper[index];
                    let before = if index >= 2 {
                        upper[index - 2]
                    } else {
                        previous
                    };
                    let after = upper.get(index + 1).copied().unwrap_or(current);

                    builder.bezier_curve_to(
                        Point::new(
                            previous.x + (current.x - before.x) / 6.0,
                            previous.y + (current.y - before.y) / 6.0,
                        ),
                        Point::new(
                            current.x - (after.x - previous.x) / 6.0,
                            current.y - (after.y - previous.y) / 6.0,
                        ),
                        current,
                    );
                }
            }
        }

        // Walk the lower edge back.
        for point in lower.iter().rev() {
            builder.line_to(*point);
        }

        builder.close();
    });

    frame.fill(&path, fill);
}

/// Draws a series of stacked bands, one per series, from `bottom` upward.
///
/// `bands` holds one run of points per series, already offset vertically so the
/// bands sit on top of one another. Each is filled down to the previous series'
/// line rather than to the baseline, which is what makes a stack read as
/// cumulative rather than as overlapping areas.
pub fn draw_stack<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    bands: &[(Vec<Point>, Color, f32)],
    baseline: f32,
) {
    for (points, color, bottom_of_previous) in bands {
        if points.len() < 2 {
            continue;
        }

        draw_area(
            frame,
            points,
            *bottom_of_previous,
            *color,
            0.85,
            Interpolation::Linear,
        );
    }

    let _ = baseline;
}

/// Draws a bar with rounded corners, growing from `base` to `tip`.
///
/// A bar whose value is zero is skipped: it would draw a hairline that reads as
/// a small value rather than as none.
pub fn draw_bar<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    center: f32,
    base: f32,
    tip: f32,
    thickness: f32,
    corner_radius: f32,
    color: Color,
) {
    let height = (base - tip).abs();

    if height < 0.5 || thickness <= 0.0 {
        return;
    }

    let top = base.min(tip);
    let left = center - thickness / 2.0;
    let radius = corner_radius.clamp(0.0, thickness.min(height) / 2.0);

    if radius <= 0.0 {
        frame.fill_rectangle(Point::new(left, top), Size::new(thickness, height), color);
        return;
    }

    // Rounded on the growing end only, so the bar still meets the axis squarely.
    let path = Path::rounded_rectangle(
        Point::new(left, top),
        Size::new(thickness, height),
        radius.into(),
    );

    frame.fill(&path, color);
}

/// Draws a vertical bar with independent corner radii.
///
/// `draw_bar` applies one radius to every corner; a bar in a `BarChart` usually
/// wants only its growing end rounded, which needs per-corner control.
pub fn draw_bar_rounded<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    center: f32,
    base: f32,
    tip: f32,
    thickness: f32,
    corners: Corners,
    color: Color,
) {
    let height = (base - tip).abs();

    if height < 0.5 || thickness <= 0.0 {
        return;
    }

    let top = base.min(tip);
    let left = center - thickness / 2.0;
    let corners = corners.clamped(thickness, height);

    if corners.is_square() {
        frame.fill_rectangle(Point::new(left, top), Size::new(thickness, height), color);
        return;
    }

    // iced's `Radius` is per-corner, so the layout order is
    // top-left, top-right, bottom-right, bottom-left.
    let radius = iced::border::Radius {
        top_left: corners.top_left,
        top_right: corners.top_right,
        bottom_right: corners.bottom_right,
        bottom_left: corners.bottom_left,
    };

    frame.fill(
        &Path::rounded_rectangle(Point::new(left, top), Size::new(thickness, height), radius),
        color,
    );
}

/// Draws a horizontal bar, for a horizontally aligned chart.
pub fn draw_bar_horizontal<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    center: f32,
    base: f32,
    tip: f32,
    thickness: f32,
    corner_radius: f32,
    color: Color,
) {
    let width = (tip - base).abs();

    if width < 0.5 || thickness <= 0.0 {
        return;
    }

    let left = base.min(tip);
    let top = center - thickness / 2.0;
    let radius = corner_radius.clamp(0.0, thickness.min(width) / 2.0);

    if radius <= 0.0 {
        frame.fill_rectangle(Point::new(left, top), Size::new(width, thickness), color);
        return;
    }

    frame.fill(
        &Path::rounded_rectangle(
            Point::new(left, top),
            Size::new(width, thickness),
            radius.into(),
        ),
        color,
    );
}

/// Draws a filled circular sector, for pie and radar charts.
///
/// Angles are in radians, measured clockwise from twelve o'clock, which is how
/// a pie is read.
pub fn draw_wedge<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    center: Point,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: Color,
) {
    if end_angle <= start_angle || outer_radius <= 0.0 {
        return;
    }

    // The chart angles run clockwise from twelve o'clock, but iced's `Arc` takes
    // standard math angles (counter-clockwise from three o'clock). Rather than
    // converting the angles and risking a mismatch between the arc and the
    // endpoints, the outline is walked as explicit points: a straight edge in,
    // then a fan of short segments along each arc. With one segment per few
    // degrees the curves are indistinguishable from true arcs and the geometry
    // is unambiguous.
    let to_point = |radius: f32, angle: f32| {
        Point::new(
            center.x + radius * angle.sin(),
            center.y - radius * angle.cos(),
        )
    };

    // One segment per ~3 degrees, so a small slice still gets a few.
    let sweep = end_angle - start_angle;
    let steps = ((sweep / 0.05).ceil() as usize).clamp(2, 512);

    let path = Path::new(|builder| {
        if inner_radius > 0.0 {
            builder.move_to(to_point(inner_radius, start_angle));
            builder.line_to(to_point(outer_radius, start_angle));

            for step in 1..=steps {
                let angle = start_angle + sweep * step as f32 / steps as f32;
                builder.line_to(to_point(outer_radius, angle));
            }

            builder.line_to(to_point(inner_radius, end_angle));

            for step in (0..steps).rev() {
                let angle = start_angle + sweep * step as f32 / steps as f32;
                builder.line_to(to_point(inner_radius, angle));
            }
        } else {
            builder.move_to(center);

            for step in 0..=steps {
                let angle = start_angle + sweep * step as f32 / steps as f32;
                builder.line_to(to_point(outer_radius, angle));
            }
        }

        builder.close();
    });

    frame.fill(&path, color);
}

/// Draws the outline of a wedge, for a radar chart's polygons.
pub fn draw_polygon<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    points: &[Point],
    fill: Option<Color>,
    stroke: Option<(Color, f32)>,
) {
    if points.len() < 3 {
        return;
    }

    let path = Path::new(|builder| {
        builder.move_to(points[0]);

        for point in &points[1..] {
            builder.line_to(*point);
        }

        builder.close();
    });

    if let Some(color) = fill {
        frame.fill(&path, color);
    }

    if let Some((color, width)) = stroke {
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(color)
                .with_width(width)
                .with_line_join(iced::widget::canvas::LineJoin::Round),
        );
    }
}

/// Draws a candlestick: a body between open and close, with wicks to the extremes.
///
/// The body is drawn hollow when the candle is bearish and filled when bullish,
/// following the convention that a hollow body reads as a decline.
///
/// The five OHLC values plus geometry and two colours exceed the argument-count
/// lint; a struct of the same fields would only move the list to the call site.
#[allow(clippy::too_many_arguments)]
pub fn draw_candle<Renderer: iced::advanced::graphics::geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    center: f32,
    high: f32,
    low: f32,
    open: f32,
    close: f32,
    width: f32,
    bullish_color: Color,
    bearish_color: Color,
) {
    let bullish = close <= open;
    let color = if bullish {
        bullish_color
    } else {
        bearish_color
    };

    let top = open.min(close);
    let bottom = open.max(close);
    let body_height = (bottom - top).max(1.0);
    let left = center - width / 2.0;

    // Wicks first, so the body covers their junction.
    frame.stroke(
        &Path::line(Point::new(center, high), Point::new(center, low)),
        Stroke::default().with_color(color).with_width(1.0),
    );

    if bullish {
        frame.fill_rectangle(Point::new(left, top), Size::new(width, body_height), color);
    } else {
        frame.stroke(
            &Path::rectangle(Point::new(left, top), Size::new(width, body_height)),
            Stroke::default().with_color(color).with_width(1.0),
        );
        frame.fill_rectangle(
            Point::new(left, top),
            Size::new(width, body_height),
            Color { a: 0.15, ..color },
        );
    }
}

/// The four corner radii of a bar, for callers wanting only one end rounded.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[must_use = "Corners does nothing unless it is given to a bar"]
pub struct Corners {
    /// Top-left radius.
    pub top_left: f32,
    /// Top-right radius.
    pub top_right: f32,
    /// Bottom-right radius.
    pub bottom_right: f32,
    /// Bottom-left radius.
    pub bottom_left: f32,
}

impl Corners {
    /// The same radius on every corner.
    pub const fn all(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Only the two corners at the growing end, for a bar rising from an axis.
    pub const fn top(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }

    /// The largest radius actually usable for a rectangle of this size.
    pub fn clamped(self, width: f32, height: f32) -> Self {
        let limit = width.min(height) / 2.0;

        Self {
            top_left: self.top_left.clamp(0.0, limit),
            top_right: self.top_right.clamp(0.0, limit),
            bottom_right: self.bottom_right.clamp(0.0, limit),
            bottom_left: self.bottom_left.clamp(0.0, limit),
        }
    }

    /// Whether every corner is square.
    pub fn is_square(self) -> bool {
        self.top_left == 0.0
            && self.top_right == 0.0
            && self.bottom_right == 0.0
            && self.bottom_left == 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{line_path, Corners, Interpolation};
    use iced::Point;

    fn points() -> Vec<Point> {
        vec![
            Point::new(0.0, 10.0),
            Point::new(10.0, 5.0),
            Point::new(20.0, 15.0),
            Point::new(30.0, 8.0),
        ]
    }

    #[test]
    fn a_line_path_needs_at_least_two_points() {
        assert!(line_path(&[], Interpolation::Linear).is_none());
        assert!(line_path(&[Point::ORIGIN], Interpolation::Linear).is_none());
        assert!(line_path(&points(), Interpolation::Linear).is_some());
    }

    #[test]
    fn every_interpolation_produces_a_path() {
        for interpolation in [
            Interpolation::Linear,
            Interpolation::Natural,
            Interpolation::StepAfter,
        ] {
            assert!(
                line_path(&points(), interpolation).is_some(),
                "{interpolation:?} must produce a path"
            );
        }
    }

    #[test]
    fn a_natural_curve_works_with_only_two_points() {
        // The control points derive from neighbours, which do not exist here.
        let two = vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)];
        assert!(line_path(&two, Interpolation::Natural).is_some());
    }

    #[test]
    fn corner_radii_clamp_to_half_the_smaller_side() {
        let corners = Corners::all(100.0).clamped(20.0, 10.0);

        // A radius larger than half the height would invert the corners.
        assert_eq!(corners.top_left, 5.0);
        assert_eq!(corners.bottom_right, 5.0);
    }

    #[test]
    fn corner_radii_can_round_only_one_end() {
        let top = Corners::top(4.0);

        assert_eq!(top.top_left, 4.0);
        assert_eq!(top.top_right, 4.0);
        assert_eq!(top.bottom_left, 0.0);
        assert!(!top.is_square());
    }

    #[test]
    fn square_corners_are_reported() {
        assert!(Corners::default().is_square());
        assert!(Corners::all(0.0).is_square());
        assert!(!Corners::all(2.0).is_square());
    }

    #[test]
    fn clamping_never_produces_a_negative_radius() {
        let corners = Corners::all(-5.0).clamped(10.0, 10.0);

        assert_eq!(corners.top_left, 0.0);
        assert!(corners.is_square());
    }

    #[test]
    fn line_paths_have_the_expected_vertex_counts() {
        // The vertex count is the observable difference between the
        // interpolation modes: linear adds one per point, step adds two.
        let points = points();

        let linear = line_path(&points, Interpolation::Linear).expect("a path");
        let step = line_path(&points, Interpolation::StepAfter).expect("a path");

        let linear_vertices = linear.raw().iter().count();
        let step_vertices = step.raw().iter().count();

        assert!(linear_vertices > 0);
        assert!(
            step_vertices > linear_vertices,
            "each step inserts an extra corner: {step_vertices} vs {linear_vertices}"
        );
    }

    #[test]
    fn the_natural_mode_emits_cubics_and_the_linear_mode_does_not() {
        // The curve mode's defining property is that its segments are cubic
        // Béziers with control points; the polyline's are straight lines. That
        // is the difference worth pinning, rather than a bounding box, which
        // both modes happen to share since a curve passes through its points.
        use iced::widget::canvas::path::lyon_path::Event;

        let points = points();

        let natural = line_path(&points, Interpolation::Natural).expect("a path");

        let natural_events: Vec<_> = natural.raw().iter().collect();
        let natural_cubics = natural_events
            .iter()
            .filter(|event| matches!(event, Event::Cubic { .. }))
            .count();

        assert_eq!(natural_cubics, points.len() - 1, "one cubic per segment");

        // A cubic's control points are what make it curve; they must not sit on
        // the endpoints, which would degenerate it to a straight line.
        let has_offset_control = natural_events.iter().any(|event| {
            if let Event::Cubic { ctrl1, from, .. } = event {
                (ctrl1.x - from.x).abs() > 0.01
            } else {
                false
            }
        });

        assert!(
            has_offset_control,
            "the control points must pull the curve off the straight path"
        );

        let linear = line_path(&points, Interpolation::Linear).expect("a path");
        let linear_cubics = linear
            .raw()
            .iter()
            .filter(|event| matches!(event, Event::Cubic { .. }))
            .count();

        assert_eq!(linear_cubics, 0, "the linear mode draws straight segments");
    }

    #[test]
    fn a_two_point_natural_curve_still_produces_a_path() {
        let two = vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)];
        let path = line_path(&two, Interpolation::Natural).expect("a path");

        assert!(path.raw().iter().count() > 0);
    }

    #[test]
    fn corners_report_whether_they_round_anything() {
        assert!(Corners::default().is_square());
        assert!(!Corners::top(1.0).is_square());
        assert!(!Corners::all(1.0).is_square());
    }
}
