//! The vector geometry the visualizers draw.
//!
//! Every function here is a pure map from analyzed audio to `kurbo` shapes.
//! Nothing in this module knows about colors, signals or scenes, which is what
//! makes each visualizer's drawing testable as a shape rather than as pixels.

use kurbo::{BezPath, Point, Rect, RoundedRect, Vec2};

/// The drawable rectangle of a `width` x `height` surface.
///
/// Returns `None` for a surface with no area, which is the one case where a
/// visualizer has nothing to lay out at all.
#[must_use]
pub fn surface_rect(width: f32, height: f32) -> Option<Rect> {
    let (width, height) = (f64::from(width), f64::from(height));
    (width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0)
        .then(|| Rect::new(0.0, 0.0, width, height))
}

/// A smooth curve through `samples`, centred vertically in `area`.
///
/// `amplitude` is the fraction of `area`'s height a full-scale sample deflects
/// by. The curve is a Catmull-Rom spline rather than a polyline: an
/// oscilloscope trace is a continuous signal, and joining sample points with
/// straight segments would draw the sampling grid instead of the waveform.
///
/// More samples than the surface has pixels are reduced by taking the extreme
/// of each column's samples, so a peak survives being drawn on a narrow
/// surface instead of being skipped between two grid points.
#[must_use]
pub fn waveform_path(samples: &[f32], area: Rect, amplitude: f64) -> BezPath {
    let columns = column_count(area.width(), samples.len());
    if columns < 2 {
        return BezPath::new();
    }
    let centre = area.center().y;
    let span = area.height() * amplitude;
    let last = index_to_f64(columns - 1);
    let points: Vec<Point> = (0..columns)
        .map(|column| {
            let x = (index_to_f64(column) / last).mul_add(area.width(), area.x0);
            let value = f64::from(column_extreme(samples, columns, column));
            Point::new(x, value.mul_add(-span, centre))
        })
        .collect();
    catmull_rom(&points, false)
}

/// One rounded rectangle per band, growing from the bottom of `area`.
///
/// `amplitude` multiplies each band before it is measured against the area's
/// height, so a level of `1.0` fills the area at an amplitude of `1.0`.
/// `gap_ratio` is the share of each band's slot left empty between bars, and
/// `corner_ratio` the share of half a bar's width used as its corner radius.
#[must_use]
pub fn bar_rects(
    bands: &[f32],
    area: Rect,
    amplitude: f64,
    gap_ratio: f64,
    corner_ratio: f64,
) -> Vec<RoundedRect> {
    if bands.is_empty() {
        return Vec::new();
    }
    let slot = area.width() / index_to_f64(bands.len());
    let gap = slot * gap_ratio.clamp(0.0, 0.9);
    let bar_width = slot - gap;
    let radius = bar_width * 0.5 * corner_ratio.clamp(0.0, 1.0);
    // A bar shorter than its own corners cannot be drawn as a rounded rect, so
    // that is the height a silent band rests at: a row of caps on the baseline.
    let floor = (radius * 2.0).min(area.height());
    bands
        .iter()
        .enumerate()
        .map(|(index, band)| {
            let x0 = index_to_f64(index).mul_add(slot, area.x0) + gap / 2.0;
            let level = (f64::from(*band) * amplitude).clamp(0.0, 1.0);
            let height = (level * area.height()).max(floor);
            RoundedRect::new(x0, area.y1 - height, x0 + bar_width, area.y1, radius)
        })
        .collect()
}

/// A closed ring centred in `area` whose radius is modulated by `bands`.
///
/// The first band sits at twelve o'clock and the rest run clockwise. Bands
/// wrap around, so the curve is smooth across the seam rather than showing
/// where the spectrum was cut.
///
/// `inner_ratio` is the share of the available radius the ring rests at when a
/// band is silent, and `amplitude` multiplies each band before it is measured
/// against the radius that is left.
#[must_use]
pub fn radial_path(bands: &[f32], area: Rect, inner_ratio: f64, amplitude: f64) -> BezPath {
    if bands.len() < 3 {
        return BezPath::new();
    }
    let centre = area.center();
    let radius = area.width().min(area.height()) / 2.0;
    let inner = radius * inner_ratio.clamp(0.0, 1.0);
    let span = radius - inner;
    let count = index_to_f64(bands.len());
    let points: Vec<Point> = bands
        .iter()
        .enumerate()
        .map(|(index, band)| {
            let angle = (index_to_f64(index) / count)
                .mul_add(core::f64::consts::TAU, -core::f64::consts::FRAC_PI_2);
            let level = (f64::from(*band) * amplitude).clamp(0.0, 1.0);
            let distance = level.mul_add(span, inner);
            centre + Vec2::new(angle.cos(), angle.sin()) * distance
        })
        .collect();
    catmull_rom(&points, true)
}

/// A centripetal Catmull-Rom spline through `points`, as cubic Béziers.
///
/// The spline passes through every point and is continuous in its first
/// derivative, which is what makes it the right curve for data: it interpolates
/// the measurements instead of merely being pulled toward them.
#[must_use]
pub fn catmull_rom(points: &[Point], closed: bool) -> BezPath {
    let mut path = BezPath::new();
    if points.len() < 2 {
        return path;
    }
    path.move_to(points[0]);
    let last = points.len() - 1;
    let segments = if closed { points.len() } else { last };
    for segment in 0..segments {
        let start = points[segment % points.len()];
        let end = points[(segment + 1) % points.len()];
        let (first, second) = controls(
            before(points, segment, closed),
            start,
            end,
            after(points, segment, closed),
        );
        path.curve_to(first, second, end);
    }
    if closed {
        path.close_path();
    }
    path
}

/// The two Bézier controls of the segment from `start` to `end`.
///
/// `before` and `after` are the neighbouring knots, which is what gives the
/// segment the tangent the whole spline has at its ends.
fn controls(before: Point, start: Point, end: Point, after: Point) -> (Point, Point) {
    let span = end - start;
    let entering = knot(start - before);
    let leaving = knot(after - end);
    let across = knot(span);
    if across <= 0.0 {
        return (start, end);
    }
    // Barry-Goldman's non-uniform tangents, written in terms of the segment so
    // that a coincident neighbour degenerates to the straight-line tangent
    // rather than dividing by its own zero-length chord.
    let start_tangent = if entering > 0.0 {
        span + ((start - before) / entering - (end - before) / (entering + across)) * across
    } else {
        span
    };
    let end_tangent = if leaving > 0.0 {
        span + ((after - end) / leaving - (after - start) / (across + leaving)) * across
    } else {
        span
    };
    (start + start_tangent / 3.0, end - end_tangent / 3.0)
}

/// How far along the curve a chord of this length carries the parameter.
///
/// The square root is the centripetal parameterization: unlike the uniform
/// spline, it provably never cusps or loops, and it keeps the curve from
/// bulging past a short segment between two long ones — which is exactly the
/// shape a spectrum makes when one band peaks between two quiet ones.
fn knot(chord: Vec2) -> f64 {
    chord.hypot().sqrt()
}

/// The point before segment `segment`, which sets the tangent at its start.
///
/// An open spline clamps at its ends, which makes the tangent there point
/// straight at the neighbour; a closed one wraps.
const fn before(points: &[Point], segment: usize, closed: bool) -> Point {
    let len = points.len();
    if closed {
        points[(segment + len - 1) % len]
    } else {
        points[segment.saturating_sub(1)]
    }
}

/// The point after segment `segment`, which sets the tangent at its end.
fn after(points: &[Point], segment: usize, closed: bool) -> Point {
    let len = points.len();
    if closed {
        points[(segment + 2) % len]
    } else {
        points[(segment + 2).min(len - 1)]
    }
}

/// How many points a curve over `width` pixels is worth drawing `len` samples at.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is a finite positive pixel count already bounded by the sample count"
)]
fn column_count(width: f64, len: usize) -> usize {
    if len < 2 || !width.is_finite() || width <= 0.0 {
        return 0;
    }
    let pixels = width.ceil().min(index_to_f64(len));
    (pixels as usize).clamp(2, len)
}

/// The sample of the greatest magnitude in `column`'s share of `samples`.
fn column_extreme(samples: &[f32], columns: usize, column: usize) -> f32 {
    let start = column * samples.len() / columns;
    let end = ((column + 1) * samples.len() / columns).max(start + 1);
    samples[start..end.min(samples.len())]
        .iter()
        .copied()
        .fold(0.0_f32, |extreme, sample| {
            if sample.abs() > extreme.abs() {
                sample
            } else {
                extreme
            }
        })
}

/// Converts an index into the coordinate it is used as, without a lossy cast.
///
/// # Panics
///
/// Panics for indices past `u32::MAX`, which no sample window or band table
/// reaches.
fn index_to_f64(index: usize) -> f64 {
    f64::from(u32::try_from(index).expect("geometry indices fit in u32"))
}

#[cfg(test)]
mod tests {
    use kurbo::{PathEl, Rect, Shape as _};

    use super::{bar_rects, radial_path, surface_rect, waveform_path};

    #[expect(
        clippy::cast_possible_truncation,
        reason = "a sine's value is inside f32's range by definition"
    )]
    fn sine(len: usize, cycles: f64) -> Vec<f32> {
        (0..len)
            .map(|index| {
                let phase = core::f64::consts::TAU * cycles * super::index_to_f64(index)
                    / super::index_to_f64(len);
                phase.sin() as f32
            })
            .collect()
    }

    #[test]
    fn an_empty_surface_has_no_rectangle() {
        assert!(surface_rect(0.0, 10.0).is_none());
        assert!(surface_rect(10.0, f32::NAN).is_none());
        assert_eq!(surface_rect(4.0, 2.0), Some(Rect::new(0.0, 0.0, 4.0, 2.0)));
    }

    #[test]
    fn the_waveform_is_a_curve_that_spans_its_area() {
        let area = Rect::new(0.0, 0.0, 64.0, 32.0);
        let path = waveform_path(&sine(256, 3.0), area, 0.4);
        let elements: Vec<_> = path.elements().to_vec();
        assert!(matches!(elements.first(), Some(PathEl::MoveTo(_))));
        assert!(
            elements
                .iter()
                .skip(1)
                .all(|element| matches!(element, PathEl::CurveTo(..))),
            "the trace must be cubic segments, not a polyline"
        );
        let bounds = path.bounding_box();
        assert!((bounds.x0 - area.x0).abs() < 1.0 && (bounds.x1 - area.x1).abs() < 1.0);
        assert!(
            bounds.height() > area.height() * 0.5,
            "the sine should swing"
        );
    }

    #[test]
    fn a_flat_signal_draws_a_line_down_the_middle() {
        let area = Rect::new(0.0, 0.0, 40.0, 20.0);
        let bounds = waveform_path(&[0.0; 128], area, 0.4).bounding_box();
        assert!((bounds.center().y - area.center().y).abs() < 1e-9);
        assert!(bounds.height() < 1e-9);
    }

    #[test]
    fn bars_differ_in_height_and_stay_inside_their_area() {
        let area = Rect::new(0.0, 0.0, 90.0, 60.0);
        let bars = bar_rects(&[0.1, 0.5, 1.0], area, 1.0, 0.3, 1.0);
        assert_eq!(bars.len(), 3);
        let heights: Vec<f64> = bars.iter().map(|bar| bar.rect().height()).collect();
        assert!(heights[0] < heights[1] && heights[1] < heights[2]);
        for bar in &bars {
            let rect = bar.rect();
            assert!(rect.y1 <= area.y1 + 1e-9 && rect.y0 >= area.y0 - 1e-9);
            assert!(rect.x0 >= area.x0 - 1e-9 && rect.x1 <= area.x1 + 1e-9);
        }
    }

    #[test]
    fn the_radial_form_is_closed_and_stays_within_its_circle() {
        let area = Rect::new(0.0, 0.0, 100.0, 100.0);
        let path = radial_path(&[0.2, 0.8, 0.4, 1.0, 0.0, 0.6], area, 0.4, 1.0);
        assert!(matches!(path.elements().last(), Some(PathEl::ClosePath)));
        let bounds = path.bounding_box();
        assert!(area.contains(bounds.origin()) && bounds.width() > 40.0);
    }

    #[test]
    fn too_few_bands_draw_nothing_rather_than_a_degenerate_ring() {
        assert!(radial_path(&[0.5, 0.5], Rect::new(0.0, 0.0, 10.0, 10.0), 0.4, 1.0).is_empty());
    }
}
