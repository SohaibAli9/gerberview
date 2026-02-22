//! Arc tessellation and stroke widening for G02/G03 interpolation.
//!
//! This module converts circular interpolation commands into centerline points,
//! then widens each line segment using [`super::stroke::draw_linear`].

use std::f64::consts::TAU;

use gerber_types::{Aperture, Polygon, Rectangular};

use crate::error::GeometryError;

use super::stroke::draw_linear;
use super::types::{GeometryBuilder, Point};

const MIN_ARC_SEGMENTS: u32 = 16;
const MIN_SEGMENT_LENGTH_FLOOR: f64 = 0.01;
const RADIUS_MISMATCH_TOLERANCE: f64 = 1e-4;
const POINT_EQUALITY_EPSILON: f64 = 1e-9;

/// Default max segment length for arc tessellation in region boundaries,
/// where no stroke width is available to derive segment density.
pub const DEFAULT_REGION_ARC_SEGMENT_LENGTH: f64 = 0.1;

/// Arc sweep direction for G02/G03 interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcDirection {
    /// Clockwise interpolation (G02).
    Clockwise,
    /// Counter-clockwise interpolation (G03).
    CounterClockwise,
}

/// Gerber arc quadrant mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcQuadrantMode {
    /// Single-quadrant mode (G74), unsupported in MVP.
    SingleQuadrant,
    /// Multi-quadrant mode (G75), supported.
    MultiQuadrant,
}

/// Expand a circular interpolation command into widened triangle geometry.
///
/// The function computes arc center/sweep, tessellates the centerline into
/// multiple points, then widens each segment using [`draw_linear`].
///
/// # Errors
///
/// Returns an error when aperture parameters are invalid or when the aperture
/// type is unsupported for stroke widening.
pub fn draw_arc(
    builder: &mut GeometryBuilder,
    from: Point,
    to: Point,
    center_offset: Point,
    direction: ArcDirection,
    quadrant_mode: ArcQuadrantMode,
    aperture: &Aperture,
) -> Result<(), GeometryError> {
    let Some(stroke_width) = resolve_stroke_width(builder, aperture)? else {
        return Ok(());
    };

    let max_seg = max_segment_length_from_stroke(stroke_width);
    let Some(points) = arc_centerline_points(
        builder,
        from,
        to,
        center_offset,
        direction,
        quadrant_mode,
        max_seg,
    ) else {
        return Ok(());
    };

    emit_stroked_polyline(builder, &points, aperture)
}

/// Tessellate an arc into a series of centerline points.
///
/// `max_segment_length` controls tessellation density — shorter segments
/// produce smoother arcs. For stroked arcs, derive this from the stroke width
/// via `max_segment_length_from_stroke`. For region boundaries, use
/// [`DEFAULT_REGION_ARC_SEGMENT_LENGTH`].
///
/// Returns `None` if the arc is degenerate or uses unsupported single-quadrant mode.
pub(crate) fn arc_centerline_points(
    builder: &mut GeometryBuilder,
    from: Point,
    to: Point,
    center_offset: Point,
    direction: ArcDirection,
    quadrant_mode: ArcQuadrantMode,
    max_segment_length: f64,
) -> Option<Vec<Point>> {
    if matches!(quadrant_mode, ArcQuadrantMode::SingleQuadrant) {
        builder.warn("single-quadrant arc mode (G74) is not supported; skipping arc".to_string());
        return None;
    }

    let center = Point {
        x: from.x + center_offset.x,
        y: from.y + center_offset.y,
    };

    let radius_start = distance(from, center);
    if radius_start <= f64::EPSILON {
        builder.warn("arc has zero radius; skipping arc".to_string());
        return None;
    }

    let start_angle = (from.y - center.y).atan2(from.x - center.x);
    let (radius, sweep) = if points_approx_equal(from, to) {
        if center_offset_is_zero(center_offset) {
            builder.warn("arc start equals end with zero center offset; skipping arc".to_string());
            return None;
        }

        let full_sweep = match direction {
            ArcDirection::Clockwise => -TAU,
            ArcDirection::CounterClockwise => TAU,
        };
        (radius_start, full_sweep)
    } else {
        let radius_end = distance(to, center);
        let radius = resolve_radius(builder, radius_start, radius_end);
        if radius <= f64::EPSILON {
            builder.warn("arc has near-zero resolved radius; skipping arc".to_string());
            return None;
        }

        let end_angle = (to.y - center.y).atan2(to.x - center.x);
        let sweep = compute_sweep(start_angle, end_angle, direction);
        (radius, sweep)
    };

    let arc_length = sweep.abs() * radius;
    let segments = segment_count_for_arc(arc_length, max_segment_length);
    let points = tessellate_centerline(center, radius, start_angle, sweep, segments);
    Some(points)
}

fn emit_stroked_polyline(
    builder: &mut GeometryBuilder,
    points: &[Point],
    aperture: &Aperture,
) -> Result<(), GeometryError> {
    let mut iter = points.iter().copied();
    let Some(mut previous) = iter.next() else {
        return Ok(());
    };

    for current in iter {
        draw_linear(builder, previous, current, aperture)?;
        previous = current;
    }

    Ok(())
}

fn resolve_radius(builder: &mut GeometryBuilder, start_radius: f64, end_radius: f64) -> f64 {
    if (start_radius - end_radius).abs() > RADIUS_MISMATCH_TOLERANCE {
        builder.warn(format!(
            "arc radii mismatch ({start_radius} vs {end_radius}); using average radius"
        ));
        return (start_radius + end_radius) / 2.0;
    }

    start_radius
}

fn compute_sweep(start_angle: f64, end_angle: f64, direction: ArcDirection) -> f64 {
    let delta = end_angle - start_angle;
    match direction {
        ArcDirection::Clockwise => {
            if delta >= 0.0 {
                delta - TAU
            } else {
                delta
            }
        }
        ArcDirection::CounterClockwise => {
            if delta <= 0.0 {
                delta + TAU
            } else {
                delta
            }
        }
    }
}

fn distance(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx.mul_add(dx, dy * dy).sqrt()
}

fn points_approx_equal(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= POINT_EQUALITY_EPSILON && (a.y - b.y).abs() <= POINT_EQUALITY_EPSILON
}

fn center_offset_is_zero(center_offset: Point) -> bool {
    center_offset.x.abs() <= POINT_EQUALITY_EPSILON
        && center_offset.y.abs() <= POINT_EQUALITY_EPSILON
}

fn max_segment_length_from_stroke(stroke_width: f64) -> f64 {
    (stroke_width * 0.25).max(MIN_SEGMENT_LENGTH_FLOOR)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn segment_count_for_arc(arc_length: f64, max_segment_length: f64) -> u32 {
    let raw = (arc_length / max_segment_length).ceil();
    if !raw.is_finite() || raw <= 0.0 {
        return MIN_ARC_SEGMENTS;
    }

    let estimated = raw as u32;
    estimated.max(MIN_ARC_SEGMENTS)
}

fn tessellate_centerline(
    center: Point,
    radius: f64,
    start_angle: f64,
    sweep: f64,
    segments: u32,
) -> Vec<Point> {
    let segment_count = segments.max(1);
    let mut points = Vec::new();
    for step in 0..=segment_count {
        let t = f64::from(step) / f64::from(segment_count);
        let angle = sweep.mul_add(t, start_angle);
        points.push(Point {
            x: radius.mul_add(angle.cos(), center.x),
            y: radius.mul_add(angle.sin(), center.y),
        });
    }
    points
}

fn resolve_stroke_width(
    builder: &mut GeometryBuilder,
    aperture: &Aperture,
) -> Result<Option<f64>, GeometryError> {
    match aperture {
        Aperture::Circle(circle) => {
            normalize_dimension(builder, circle.diameter, "circle diameter")
        }
        Aperture::Rectangle(rectangle) => {
            normalize_rect_like_width(builder, rectangle, "rectangle")
        }
        Aperture::Obround(obround) => normalize_rect_like_width(builder, obround, "obround"),
        Aperture::Polygon(polygon) => resolve_polygon_width(builder, polygon),
        Aperture::Macro(name, _) => Err(GeometryError::UnsupportedFeature(format!(
            "aperture macro `{name}` is not supported by draw_arc"
        ))),
    }
}

fn resolve_polygon_width(
    builder: &mut GeometryBuilder,
    polygon: &Polygon,
) -> Result<Option<f64>, GeometryError> {
    if polygon.vertices < 3 {
        return Err(GeometryError::InvalidAperture(format!(
            "polygon has {} vertices; expected at least 3",
            polygon.vertices
        )));
    }

    normalize_dimension(builder, polygon.diameter, "polygon diameter")
}

fn normalize_rect_like_width(
    builder: &mut GeometryBuilder,
    dimensions: &Rectangular,
    shape_name: &str,
) -> Result<Option<f64>, GeometryError> {
    let width_label = format!("{shape_name} width");
    let height_label = format!("{shape_name} height");

    let Some(width) = normalize_dimension(builder, dimensions.x, &width_label)? else {
        return Ok(None);
    };
    let Some(height) = normalize_dimension(builder, dimensions.y, &height_label)? else {
        return Ok(None);
    };

    Ok(Some(width.min(height)))
}

fn normalize_dimension(
    builder: &mut GeometryBuilder,
    value: f64,
    label: &str,
) -> Result<Option<f64>, GeometryError> {
    if !value.is_finite() {
        return Err(GeometryError::InvalidAperture(format!(
            "{label} must be finite, got {value}"
        )));
    }

    let mut normalized = value;
    if normalized < 0.0 {
        builder.warn(format!(
            "{label} is negative ({normalized}); using absolute value"
        ));
        normalized = normalized.abs();
    }

    if normalized <= f64::EPSILON {
        builder.warn(format!("{label} is zero; skipping arc tessellation"));
        return Ok(None);
    }

    Ok(Some(normalized))
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use std::f64::consts::PI;

    use gerber_types::{Circle, Rectangular};

    use super::*;

    const EPSILON: f64 = 1e-6;

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected}, got {actual}, tolerance {tolerance}"
        );
    }

    #[allow(clippy::needless_pass_by_value)]
    fn build_arc(
        from: Point,
        to: Point,
        center_offset: Point,
        direction: ArcDirection,
        mode: ArcQuadrantMode,
        aperture: Aperture,
    ) -> crate::geometry::LayerGeometry {
        let mut builder = GeometryBuilder::new();
        let result = draw_arc(
            &mut builder,
            from,
            to,
            center_offset,
            direction,
            mode,
            &aperture,
        );
        assert!(result.is_ok(), "expected draw_arc to succeed");
        builder.build()
    }

    #[test]
    fn ut_arc_001_cw_quarter_arc_points_stay_on_radius() {
        let mut builder = GeometryBuilder::new();
        let points = arc_centerline_points(
            &mut builder,
            Point { x: 0.0, y: 5.0 },
            Point { x: 5.0, y: 0.0 },
            Point { x: 0.0, y: -5.0 },
            ArcDirection::Clockwise,
            ArcQuadrantMode::MultiQuadrant,
            0.25,
        )
        .unwrap_or_default();

        assert!(!points.is_empty(), "expected tessellated points");
        for point in points {
            let radius = distance(point, Point { x: 0.0, y: 0.0 });
            assert_close(radius, 5.0, RADIUS_MISMATCH_TOLERANCE);
        }
    }

    #[test]
    fn ut_arc_002_ccw_sweep_is_positive_and_cw_is_negative() {
        let start = 0.0;
        let end = PI / 2.0;
        let cw = compute_sweep(start, end, ArcDirection::Clockwise);
        let ccw = compute_sweep(start, end, ArcDirection::CounterClockwise);
        assert!(cw < 0.0, "expected clockwise sweep to be negative");
        assert!(ccw > 0.0, "expected counter-clockwise sweep to be positive");
    }

    #[test]
    fn ut_arc_003_semicircle_spans_half_circle() {
        let mut builder = GeometryBuilder::new();
        let points = arc_centerline_points(
            &mut builder,
            Point { x: 5.0, y: 0.0 },
            Point { x: -5.0, y: 0.0 },
            Point { x: -5.0, y: 0.0 },
            ArcDirection::CounterClockwise,
            ArcQuadrantMode::MultiQuadrant,
            0.25,
        )
        .unwrap_or_default();

        assert!(!points.is_empty(), "expected tessellated points");
        let mut max_y = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        for point in points {
            max_y = max_y.max(point.y);
            min_y = min_y.min(point.y);
        }
        assert!(max_y > 4.9, "expected upper semicircle to reach near y=5");
        assert!(
            min_y >= -EPSILON,
            "expected points to stay on upper half-plane"
        );
    }

    #[test]
    fn ut_arc_004_full_circle_when_start_equals_end_and_center_offset_non_zero() {
        let mut builder = GeometryBuilder::new();
        let points = arc_centerline_points(
            &mut builder,
            Point { x: 5.0, y: 0.0 },
            Point { x: 5.0, y: 0.0 },
            Point { x: -5.0, y: 0.0 },
            ArcDirection::CounterClockwise,
            ArcQuadrantMode::MultiQuadrant,
            0.25,
        )
        .unwrap_or_default();

        assert!(!points.is_empty(), "expected full-circle points");
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for point in points {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }
        assert!(min_x <= -4.9 && max_x >= 4.9);
        assert!(min_y <= -4.9 && max_y >= 4.9);
    }

    #[test]
    fn ut_arc_005_small_arc_uses_minimum_segment_count() {
        let angle = (0.5_f64).to_radians();
        let radius = 10.0;
        let from = Point { x: radius, y: 0.0 };
        let to = Point {
            x: radius * angle.cos(),
            y: radius * angle.sin(),
        };
        let mut builder = GeometryBuilder::new();
        let points = arc_centerline_points(
            &mut builder,
            from,
            to,
            Point { x: -radius, y: 0.0 },
            ArcDirection::CounterClockwise,
            ArcQuadrantMode::MultiQuadrant,
            0.25,
        )
        .unwrap_or_default();

        let minimum = usize::try_from(MIN_ARC_SEGMENTS).unwrap_or(usize::MAX);
        assert!(points.len() >= minimum.saturating_add(1));
    }

    #[test]
    fn ut_arc_006_draw_arc_emits_stroke_geometry() {
        let geom = build_arc(
            Point { x: 5.0, y: 0.0 },
            Point { x: 0.0, y: 5.0 },
            Point { x: -5.0, y: 0.0 },
            ArcDirection::CounterClockwise,
            ArcQuadrantMode::MultiQuadrant,
            Aperture::Rectangle(Rectangular::new(1.0, 1.0)),
        );

        assert!(geom.vertex_count > 0, "expected widened arc vertices");
        assert!(!geom.indices.is_empty(), "expected widened arc indices");
    }

    #[test]
    fn ut_arc_007_zero_radius_arc_skips_with_warning() {
        let geom = build_arc(
            Point { x: 1.0, y: 1.0 },
            Point { x: 2.0, y: 2.0 },
            Point { x: 0.0, y: 0.0 },
            ArcDirection::CounterClockwise,
            ArcQuadrantMode::MultiQuadrant,
            Aperture::Circle(Circle::new(1.0)),
        );

        assert_eq!(geom.vertex_count, 0);
        assert_eq!(geom.indices.len(), 0);
        assert!(
            geom.warnings
                .iter()
                .any(|warning| warning.contains("zero radius")),
            "expected zero-radius warning"
        );
    }

    #[test]
    fn bc_gbr_015_single_quadrant_warns_and_multi_quadrant_draws() {
        let mut single_builder = GeometryBuilder::new();
        let single_result = draw_arc(
            &mut single_builder,
            Point { x: 5.0, y: 0.0 },
            Point { x: 0.0, y: 5.0 },
            Point { x: -5.0, y: 0.0 },
            ArcDirection::CounterClockwise,
            ArcQuadrantMode::SingleQuadrant,
            &Aperture::Circle(Circle::new(1.0)),
        );
        assert!(
            single_result.is_ok(),
            "single-quadrant path should not error"
        );
        let single_geom = single_builder.build();
        assert_eq!(single_geom.vertex_count, 0);
        assert!(
            single_geom
                .warnings
                .iter()
                .any(|warning| warning.contains("single-quadrant arc mode")),
            "expected single-quadrant warning"
        );

        let multi_geom = build_arc(
            Point { x: 5.0, y: 0.0 },
            Point { x: 0.0, y: 5.0 },
            Point { x: -5.0, y: 0.0 },
            ArcDirection::CounterClockwise,
            ArcQuadrantMode::MultiQuadrant,
            Aperture::Circle(Circle::new(1.0)),
        );
        assert!(multi_geom.vertex_count > 0, "expected geometry in G75 mode");
    }
}
