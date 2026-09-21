//! Chart scales: the mapping between data and screen coordinates.
//!
//! Ported from `gpui-kit`'s `plot/scale` (Apache-2.0). The design is d3's: a
//! scale owns a *domain* (the data range) and a *range* (the pixel span), and
//! converts between them.
//!
//! Each scale answers two questions, and the second is what makes tooltips
//! possible:
//!
//! - [`Scale::tick`] — where does this data value sit on screen?
//! - [`Scale::least_index_with_domain`] — which data value is nearest this
//!   screen position?
//!
//! # Note on floating-point comparison
//!
//! Categorical scales key their domain by value, which requires `Hash + Eq`.
//! That excludes `f32`, so a chart with numeric categories should format them
//! to a string first. Numeric axes use [`ScaleLinear`], which does not need
//! hashing.

// The scale constructors take their domain and range by value: every call site
// builds them inline (`vec![min, max]`), so accepting a slice would only make
// callers borrow a temporary. This matches upstream's API.
#![allow(clippy::needless_pass_by_value)]

use std::collections::HashMap;
use std::hash::Hash;

/// A mapping between a data domain and a pixel range.
pub trait Scale<T> {
    /// The position this value maps to, or `None` if the domain is degenerate.
    fn tick(&self, value: &T) -> Option<f32>;

    /// The index of the domain value nearest `tick`, when unknown.
    ///
    /// Used by categorical scales, which can recover an index from a position
    /// without the caller supplying the domain.
    fn least_index(&self, tick: f32) -> usize {
        let _ = tick;
        0
    }

    /// The index of the domain value nearest `tick`, and that value's position.
    ///
    /// This is the hit test a tooltip performs: the cursor gives a pixel
    /// position, and the scale reports which datum is under it.
    fn least_index_with_domain(&self, tick: f32, domain: &[T]) -> (usize, f32) {
        let _ = (tick, domain);
        (0, 0.0)
    }
}

/// A linear scale, for continuous numeric axes.
///
/// The domain is the data's extent and the range is the pixel span. A reversed
/// range (larger pixel value first) inverts the axis, which is how a y-axis
/// gets its upward orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[must_use = "a ScaleLinear does nothing unless a chart draws with it"]
pub struct ScaleLinear<T> {
    domain_len: usize,
    domain_start: T,
    domain_diff: T,
    range_start: f32,
    range_diff: f32,
}

/// The numeric types a [`ScaleLinear`] accepts.
///
/// Kept as a local trait rather than depending on `num-traits`: the operations
/// needed here are addition, subtraction and conversion to `f64`, and requiring
/// an external crate for those would be a dependency for three methods.
pub trait Numeric: Copy + PartialOrd {
    /// The additive identity.
    const ZERO: Self;

    /// Adds two values.
    #[must_use]
    fn add(self, other: Self) -> Self;

    /// Subtracts `other` from `self`.
    #[must_use]
    fn sub(self, other: Self) -> Self;

    /// Converts to `f64`.
    fn to_f64(self) -> f64;
}

/// Types whose conversion to `f64` is exact, so `From` is available.
macro_rules! impl_numeric_exact {
    ($($ty:ty),*) => {
        $(
            impl Numeric for $ty {
                const ZERO: Self = 0 as Self;

                fn add(self, other: Self) -> Self {
                    self + other
                }

                fn sub(self, other: Self) -> Self {
                    self - other
                }

                fn to_f64(self) -> f64 {
                    f64::from(self)
                }
            }
        )*
    };
}

/// Types wide enough that a conversion to `f64` can lose precision.
///
/// A cast is the only option here — `f64::from` does not exist for them — and
/// the loss is acceptable for chart coordinates, which are sub-pixel anyway.
macro_rules! impl_numeric_wide {
    ($($ty:ty),*) => {
        $(
            impl Numeric for $ty {
                const ZERO: Self = 0 as Self;

                fn add(self, other: Self) -> Self {
                    self + other
                }

                fn sub(self, other: Self) -> Self {
                    self - other
                }

                #[allow(clippy::cast_precision_loss)]
                fn to_f64(self) -> f64 {
                    self as f64
                }
            }
        )*
    };
}

impl_numeric_exact!(u8, u16, u32, i8, i16, i32, f32);
impl_numeric_wide!(u64, i64, usize, isize, f64);

impl<T: Numeric> ScaleLinear<T> {
    /// Creates a scale mapping `domain` onto `range`.
    ///
    /// The domain is reduced to its extent, so a single value collapses the
    /// scale and every lookup returns `None` rather than dividing by zero.
    pub fn new(domain: Vec<T>, range: Vec<f32>) -> Self {
        let (domain_start, domain_end) = extent(&domain).unwrap_or((T::ZERO, T::ZERO));
        let (range_start, range_end) = range_extent(&range).unwrap_or((0.0, 0.0));

        Self {
            domain_len: domain.len(),
            domain_start,
            domain_diff: domain_end.sub(domain_start),
            range_start,
            range_diff: range_end - range_start,
        }
    }

    /// Creates a scale from explicit endpoints.
    pub fn from_bounds(domain: (T, T), range: (f32, f32)) -> Self {
        Self {
            domain_len: 2,
            domain_start: domain.0,
            domain_diff: domain.1.sub(domain.0),
            range_start: range.0,
            range_diff: range.1 - range.0,
        }
    }

    /// The number of values the domain was built from.
    #[must_use]
    pub fn domain_len(&self) -> usize {
        self.domain_len
    }

    /// Whether the domain has no extent, so the scale cannot map anything.
    #[must_use]
    pub fn is_degenerate(&self) -> bool {
        self.domain_diff.to_f64().abs() < f64::EPSILON
    }
}

impl<T: Numeric> Scale<T> for ScaleLinear<T> {
    fn tick(&self, value: &T) -> Option<f32> {
        if self.is_degenerate() {
            return None;
        }

        let ratio = value.sub(self.domain_start).to_f64() / self.domain_diff.to_f64();

        Some((ratio as f32) * self.range_diff + self.range_start)
    }

    fn least_index(&self, tick: f32) -> usize {
        self.least_index_with_domain(tick, &[]).0
    }

    fn least_index_with_domain(&self, tick: f32, domain: &[T]) -> (usize, f32) {
        if self.domain_len == 0 || domain.is_empty() || self.is_degenerate() {
            return (0, 0.0);
        }

        // Invert `tick` to recover the domain value under this pixel position,
        // then pick the domain entry closest to it.
        let ratio = if self.range_diff.abs() < f32::EPSILON {
            0.0
        } else {
            (tick - self.range_start) / self.range_diff
        };

        let target = self.domain_start.to_f64() + f64::from(ratio) * self.domain_diff.to_f64();

        let mut best_index = 0;
        let mut best_distance = f64::INFINITY;

        for (index, value) in domain.iter().enumerate() {
            let distance = (value.to_f64() - target).abs();

            if distance < best_distance {
                best_distance = distance;
                best_index = index;
            }
        }

        let position = self.tick(&domain[best_index]).unwrap_or(tick);
        (best_index, position)
    }
}

/// A band scale, for category axes where each category owns a slot.
///
/// Bands have width: a bar chart draws a bar inside each band. The padding
/// controls the gap between bands and at the ends of the axis.
#[derive(Debug, Clone)]
#[must_use = "a ScaleBand does nothing unless a chart draws with it"]
pub struct ScaleBand<T> {
    indices: HashMap<T, usize>,
    range_start: f32,
    range_diff: f32,
    avg_width: f32,
    padding_inner: f32,
    padding_outer: f32,
}

impl<T: Eq + Hash> ScaleBand<T> {
    /// Creates a band scale over the given categories.
    pub fn new(domain: Vec<T>, range: Vec<f32>) -> Self {
        let mut indices = HashMap::with_capacity(domain.len());

        for value in domain {
            let next = indices.len();
            indices.entry(value).or_insert(next);
        }

        let len = indices.len() as f32;
        let (range_start, range_end) = range_extent(&range).unwrap_or((0.0, 0.0));
        let range_diff = range_end - range_start;

        Self {
            indices,
            range_start,
            range_diff,
            avg_width: if len == 0.0 { 0.0 } else { range_diff / len },
            padding_inner: 0.0,
            padding_outer: 0.0,
        }
    }

    /// Sets the gap between adjacent bands, as a fraction of the slot.
    pub fn padding_inner(mut self, padding_inner: f32) -> Self {
        self.padding_inner = padding_inner.clamp(0.0, 0.95);
        self
    }

    /// Sets the gap at each end of the axis, as a fraction of the slot.
    pub fn padding_outer(mut self, padding_outer: f32) -> Self {
        self.padding_outer = padding_outer.clamp(0.0, 0.95);
        self
    }

    /// How many categories the scale holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Whether the scale holds no categories.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// The usable width of one band.
    ///
    /// Deliberately uncapped: an earlier version clamped this at 30px, which
    /// silently made `padding_inner` a no-op for any chart with few categories,
    /// because both values collapsed to the cap. A caller wanting narrower bars
    /// sets `padding_inner` or constrains the plot area — the scale should
    /// report the geometry it was asked for.
    #[must_use]
    pub fn band_width(&self) -> f32 {
        self.avg_width * (1.0 - self.padding_inner)
    }

    /// The distance between the starts of two adjacent bands.
    ///
    /// Includes the inner padding, so successive `tick` positions are one
    /// `step` apart.
    #[must_use]
    pub fn step(&self) -> f32 {
        if self.len() <= 1 {
            self.range_diff
        } else {
            self.avg_width
        }
    }

    /// The position of a category, measured from the range's start.
    fn offset(&self, value: &T) -> Option<f32> {
        let index = *self.indices.get(value)? as f32;
        let outer = self.padding_outer * self.avg_width;

        Some(self.range_start + outer + index * self.avg_width)
    }
}

impl<T: Eq + Hash> Scale<T> for ScaleBand<T> {
    fn tick(&self, value: &T) -> Option<f32> {
        self.offset(value)
    }

    fn least_index(&self, tick: f32) -> usize {
        if self.avg_width.abs() < f32::EPSILON {
            return 0;
        }

        let outer = self.padding_outer * self.avg_width;
        let index = ((tick - self.range_start - outer) / self.avg_width).floor();

        index.max(0.0) as usize
    }

    fn least_index_with_domain(&self, tick: f32, domain: &[T]) -> (usize, f32) {
        let index = self.least_index(tick).min(domain.len().saturating_sub(1));

        let position = domain
            .get(index)
            .and_then(|value| self.offset(value))
            .unwrap_or(tick);

        (index, position)
    }
}

/// A point scale, for category axes where categories are points rather than slots.
///
/// A line chart's x-axis: values sit at evenly spaced positions with no width,
/// and the first and last sit on the range's edges.
#[derive(Debug, Clone)]
#[must_use = "a ScalePoint does nothing unless a chart draws with it"]
pub struct ScalePoint<T> {
    indices: HashMap<T, usize>,
    range_start: f32,
    range_diff: f32,
    len: usize,
}

impl<T: Eq + Hash> ScalePoint<T> {
    /// Creates a point scale over the given categories.
    pub fn new(domain: Vec<T>, range: Vec<f32>) -> Self {
        let mut indices = HashMap::with_capacity(domain.len());

        for value in domain {
            let next = indices.len();
            indices.entry(value).or_insert(next);
        }

        let len = indices.len();
        let (range_start, range_end) = range_extent(&range).unwrap_or((0.0, 0.0));

        Self {
            indices,
            range_start,
            range_diff: range_end - range_start,
            len,
        }
    }

    /// How many categories the scale holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// Whether the scale holds no categories.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// The distance between two adjacent points.
    #[must_use]
    pub fn step(&self) -> f32 {
        if self.len <= 1 {
            0.0
        } else {
            self.range_diff / (self.len - 1) as f32
        }
    }
}

impl<T: Eq + Hash> Scale<T> for ScalePoint<T> {
    fn tick(&self, value: &T) -> Option<f32> {
        let index = *self.indices.get(value)? as f32;

        // A single point sits centred; otherwise points span the whole range.
        if self.len <= 1 {
            Some(self.range_start + self.range_diff / 2.0)
        } else {
            Some(self.range_start + self.step() * index)
        }
    }

    fn least_index(&self, tick: f32) -> usize {
        let step = self.step();

        if step.abs() < f32::EPSILON {
            return 0;
        }

        ((tick - self.range_start) / step).round().max(0.0) as usize
    }

    fn least_index_with_domain(&self, tick: f32, domain: &[T]) -> (usize, f32) {
        let index = self.least_index(tick).min(domain.len().saturating_sub(1));

        let position = domain.get(index).and_then(|v| self.tick(v)).unwrap_or(tick);

        (index, position)
    }
}

/// A scale mapping categories onto colours from a cyclic palette.
#[derive(Debug, Clone)]
#[must_use = "a ScaleOrdinal does nothing unless a chart draws with it"]
pub struct ScaleOrdinal<T> {
    indices: HashMap<T, usize>,
}

impl<T: Eq + Hash> ScaleOrdinal<T> {
    /// Creates an ordinal scale over the given categories.
    pub fn new(domain: Vec<T>) -> Self {
        let mut indices = HashMap::with_capacity(domain.len());

        for value in domain {
            let next = indices.len();
            indices.entry(value).or_insert(next);
        }

        Self { indices }
    }

    /// The palette index for a category, cycling once the palette is exhausted.
    #[must_use]
    pub fn index_of(&self, value: &T) -> Option<usize> {
        self.indices.get(value).copied()
    }

    /// The palette entry for a category, wrapping around `palette_len`.
    #[must_use]
    pub fn cycle(&self, value: &T, palette_len: usize) -> Option<usize> {
        if palette_len == 0 {
            return None;
        }

        self.index_of(value).map(|index| index % palette_len)
    }
}

/// The smallest and largest values present.
fn extent<T: Numeric>(values: &[T]) -> Option<(T, T)> {
    let mut iter = values.iter().copied();

    let first = iter.next()?;
    let mut min = first;
    let mut max = first;

    for value in iter {
        if value < min {
            min = value;
        }
        if value > max {
            max = value;
        }
    }

    Some((min, max))
}

/// The first and last finite values, in the order given.
///
/// Order matters: a y-axis maps larger data values to smaller pixel values, so
/// the range is `[bottom, top]` and collapsing it to min/max would silently
/// un-invert the axis.
fn range_extent(values: &[f32]) -> Option<(f32, f32)> {
    let mut finite = values.iter().copied().filter(|v| v.is_finite());

    let first = finite.next()?;

    // `DoubleEndedIterator::last` would walk the iterator twice; taking the
    // remaining values in order is one pass.
    let mut last = first;
    for value in finite {
        last = value;
    }

    Some((first, last))
}

#[cfg(test)]
mod tests {
    use super::{Numeric, Scale, ScaleBand, ScaleLinear, ScaleOrdinal, ScalePoint};

    #[test]
    fn a_linear_scale_maps_the_domain_onto_the_range() {
        let scale = ScaleLinear::new(vec![0.0, 100.0], vec![0.0, 200.0]);

        assert_eq!(scale.tick(&0.0), Some(0.0));
        assert_eq!(scale.tick(&50.0), Some(100.0));
        assert_eq!(scale.tick(&100.0), Some(200.0));
    }

    #[test]
    fn a_reversed_range_inverts_the_axis() {
        // A y-axis needs larger values higher up, which means a descending range.
        let scale = ScaleLinear::new(vec![0.0, 100.0], vec![300.0, 0.0]);

        assert_eq!(scale.tick(&0.0), Some(300.0));
        assert_eq!(scale.tick(&100.0), Some(0.0));
    }

    #[test]
    fn a_degenerate_domain_yields_no_positions() {
        // A single value has no extent, so there is nothing to map.
        let scale = ScaleLinear::new(vec![5.0, 5.0], vec![0.0, 100.0]);

        assert!(scale.is_degenerate());
        assert_eq!(scale.tick(&5.0), None);
    }

    #[test]
    fn an_empty_domain_does_not_panic() {
        let scale = ScaleLinear::new(Vec::<f32>::new(), vec![0.0, 100.0]);

        assert_eq!(scale.tick(&1.0), None);
        assert!(scale.is_degenerate());
    }

    #[test]
    fn a_linear_scale_recovers_the_nearest_datum_from_a_position() {
        // This is the hit test a tooltip performs.
        let domain = vec![0.0, 10.0, 20.0, 30.0];
        let scale = ScaleLinear::new(domain.clone(), vec![0.0, 300.0]);

        let (index, position) = scale.least_index_with_domain(95.0, &domain);
        assert_eq!(index, 1, "95px is nearest the second datum");
        assert!((position - 100.0).abs() < 0.01);

        let (index, _) = scale.least_index_with_domain(305.0, &domain);
        assert_eq!(index, 3, "a position past the end clamps to the last datum");
    }

    #[test]
    fn a_linear_scale_hit_test_survives_an_empty_domain_argument() {
        let scale = ScaleLinear::new(vec![0.0, 10.0], vec![0.0, 100.0]);

        // An empty domain would otherwise index out of bounds.
        assert_eq!(scale.least_index_with_domain(50.0, &[]), (0, 0.0));
    }

    #[test]
    fn a_linear_scale_accepts_integer_domains() {
        let scale = ScaleLinear::new(vec![0u32, 10, 20], vec![0.0, 100.0]);

        assert_eq!(scale.tick(&10), Some(50.0));

        let (index, _) = scale.least_index_with_domain(90.0, &[0u32, 10, 20]);
        assert_eq!(index, 2);
    }

    /// A range that does not start at zero is the normal case for a chart: the
    /// plot area sits to the right of the y-axis labels.
    ///
    /// This was broken: the band and point scales computed positions relative to
    /// the origin and ignored where their range began, so a chart whose plot
    /// area started at x=24 drew its bars and points over the axis labels.
    #[test]
    fn a_band_scale_offsets_by_its_range_start() {
        let scale = ScaleBand::new(vec!["a", "b"], vec![100.0, 300.0]);

        assert_eq!(scale.tick(&"a"), Some(100.0));
        assert_eq!(scale.tick(&"b"), Some(200.0));
    }

    #[test]
    fn a_point_scale_offsets_by_its_range_start() {
        let scale = ScalePoint::new(vec!["a", "b", "c"], vec![100.0, 300.0]);

        assert_eq!(scale.tick(&"a"), Some(100.0));
        assert_eq!(scale.tick(&"b"), Some(200.0));
        assert_eq!(scale.tick(&"c"), Some(300.0));
    }

    #[test]
    fn a_single_point_scale_offsets_by_its_range_start() {
        let scale = ScalePoint::new(vec!["only"], vec![100.0, 300.0]);
        assert_eq!(scale.tick(&"only"), Some(200.0));
    }

    #[test]
    fn the_reverse_mapping_also_accounts_for_the_range_start() {
        // The forward and reverse mappings must agree, or a tooltip highlights
        // a different datum than the one under the cursor.
        let domain = vec!["a", "b", "c"];
        let scale = ScaleBand::new(domain.clone(), vec![100.0, 400.0]);

        let (index, position) = scale.least_index_with_domain(200.0, &domain);
        assert_eq!(index, 1);
        assert_eq!(position, 200.0);
        assert_eq!(scale.tick(&"b"), Some(position));

        let point = ScalePoint::new(domain.clone(), vec![100.0, 300.0]);
        let (index, position) = point.least_index_with_domain(200.0, &domain);
        assert_eq!(index, 1);
        assert_eq!(point.tick(&"b"), Some(position));
    }

    #[test]
    fn a_band_scale_gives_each_category_a_slot() {
        let scale = ScaleBand::new(vec!["a", "b", "c"], vec![0.0, 300.0]);

        assert_eq!(scale.len(), 3);
        assert_eq!(scale.tick(&"a"), Some(0.0));
        assert_eq!(scale.tick(&"b"), Some(100.0));
        assert_eq!(scale.tick(&"c"), Some(200.0));

        assert!(scale.band_width() > 0.0);
        assert_eq!(scale.step(), 100.0);
    }

    #[test]
    fn a_band_scale_width_is_uncapped_so_padding_still_bites() {
        // With only two categories each slot is half the range, which is far
        // wider than a bar should be; the caller narrows it with padding. A
        // hard cap here would make that padding do nothing.
        let tight = ScaleBand::new(vec!["a", "b"], vec![0.0, 200.0]);
        let padded = ScaleBand::new(vec!["a", "b"], vec![0.0, 200.0]).padding_inner(0.5);

        assert_eq!(tight.band_width(), 100.0);
        assert_eq!(padded.band_width(), 50.0);
    }

    #[test]
    fn a_band_scale_ignores_duplicate_categories() {
        // A repeated label would otherwise shift every later band.
        let scale = ScaleBand::new(vec!["a", "a", "b"], vec![0.0, 200.0]);

        assert_eq!(scale.len(), 2);
        assert_eq!(scale.tick(&"a"), Some(0.0));
        assert_eq!(scale.tick(&"b"), Some(100.0));
    }

    #[test]
    fn a_band_scale_returns_none_for_an_unknown_category() {
        let scale = ScaleBand::new(vec!["a"], vec![0.0, 100.0]);
        assert_eq!(scale.tick(&"zzz"), None);
    }

    #[test]
    fn a_band_scale_recovers_a_category_from_a_position() {
        let domain = vec!["a", "b", "c"];
        let scale = ScaleBand::new(domain.clone(), vec![0.0, 300.0]);

        let (index, position) = scale.least_index_with_domain(150.0, &domain);
        assert_eq!(index, 1);
        assert_eq!(position, 100.0);

        // Positions outside the range clamp rather than indexing out of bounds.
        let (index, _) = scale.least_index_with_domain(-50.0, &domain);
        assert_eq!(index, 0);
        let (index, _) = scale.least_index_with_domain(9_999.0, &domain);
        assert_eq!(index, 2);
    }

    #[test]
    fn a_band_scale_padding_narrows_the_bands() {
        let tight = ScaleBand::new(vec!["a", "b"], vec![0.0, 200.0]);
        let padded = ScaleBand::new(vec!["a", "b"], vec![0.0, 200.0]).padding_inner(0.5);

        assert!(padded.band_width() < tight.band_width());
    }

    #[test]
    fn a_band_scale_clamps_absurd_padding() {
        // A padding of 1.0 would leave zero-width bands, which cannot be drawn
        // or hit-tested.
        let scale = ScaleBand::new(vec!["a", "b"], vec![0.0, 200.0]).padding_inner(5.0);

        assert!(scale.band_width() > 0.0);
    }

    #[test]
    fn an_empty_band_scale_is_empty_and_safe() {
        let scale = ScaleBand::new(Vec::<&str>::new(), vec![0.0, 100.0]);

        assert!(scale.is_empty());
        assert_eq!(scale.tick(&"a"), None);
        assert_eq!(scale.least_index(50.0), 0);
    }

    #[test]
    fn a_point_scale_spans_the_range_edges() {
        let scale = ScalePoint::new(vec!["a", "b", "c"], vec![0.0, 200.0]);

        assert_eq!(scale.tick(&"a"), Some(0.0));
        assert_eq!(scale.tick(&"b"), Some(100.0));
        assert_eq!(scale.tick(&"c"), Some(200.0));
        assert_eq!(scale.step(), 100.0);
    }

    #[test]
    fn a_single_point_sits_centred() {
        let scale = ScalePoint::new(vec!["only"], vec![0.0, 200.0]);

        assert_eq!(scale.tick(&"only"), Some(100.0));
        assert_eq!(scale.step(), 0.0);
    }

    #[test]
    fn a_point_scale_recovers_the_nearest_category() {
        let domain = vec!["a", "b", "c"];
        let scale = ScalePoint::new(domain.clone(), vec![0.0, 200.0]);

        let (index, position) = scale.least_index_with_domain(96.0, &domain);
        assert_eq!(index, 1);
        assert_eq!(position, 100.0);

        let (index, _) = scale.least_index_with_domain(-20.0, &domain);
        assert_eq!(index, 0);
        let (index, _) = scale.least_index_with_domain(500.0, &domain);
        assert_eq!(index, 2);
    }

    #[test]
    fn an_empty_point_scale_is_safe() {
        let scale = ScalePoint::new(Vec::<&str>::new(), vec![0.0, 100.0]);

        assert!(scale.is_empty());
        assert_eq!(scale.least_index(50.0), 0);
        assert_eq!(scale.least_index_with_domain(50.0, &[]), (0, 50.0));
    }

    #[test]
    fn an_ordinal_scale_cycles_through_a_palette() {
        let scale = ScaleOrdinal::new(vec!["a", "b", "c"]);

        assert_eq!(scale.cycle(&"a", 2), Some(0));
        assert_eq!(scale.cycle(&"b", 2), Some(1));
        assert_eq!(scale.cycle(&"c", 2), Some(0), "the palette wraps");
    }

    #[test]
    fn an_ordinal_scale_with_an_empty_palette_yields_nothing() {
        let scale = ScaleOrdinal::new(vec!["a"]);

        assert_eq!(scale.cycle(&"a", 0), None);
        assert_eq!(scale.cycle(&"zzz", 4), None);
    }

    #[test]
    fn numeric_conversions_round_trip_for_every_supported_type() {
        assert_eq!(Numeric::to_f64(7u8), 7.0);
        assert_eq!(Numeric::to_f64(-7i32), -7.0);
        assert_eq!(Numeric::to_f64(7usize), 7.0);
        assert_eq!(Numeric::to_f64(7.5f32), 7.5);
    }

    #[test]
    fn a_linear_scale_ignores_non_finite_range_values() {
        let scale = ScaleLinear::new(vec![0.0, 10.0], vec![0.0, f32::NAN, 100.0]);

        // A NaN in the range would otherwise poison every position.
        let position = scale.tick(&5.0).expect("the scale still maps");
        assert!(position.is_finite(), "got {position}");
    }
}
