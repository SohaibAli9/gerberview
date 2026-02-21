//! Linear stroke widening for D01 interpolation.
//!
//! This module converts a line segment into thick triangle geometry using the
//! currently selected aperture.

use std::f64::consts::{FRAC_PI_2, PI};

use gerber_types::{Aperture, Polygon, Rectangular};

use crate::error::GeometryError;

use super::aperture::flash_aperture;
use super::types::{GeometryBuilder, Point};

const CIRCLE_ENDCAP_SEGMENTS: u32 = 16;

/// Expand a linear D01 draw command into renderable triangles.
///
/// The segment body is emitted as a quad. For circular apertures, rounded
/// semicircle endcaps are added at both ends.
///
/// # Errors
///
/// Returns an error when aperture parameters are invalid or when the aperture
/// type is not supported for stroke widening.
pub fn draw_linear(
    builder: &mut GeometryBuilder,
    from: Point,
    to: Point,
    aperture: &Aperture,
) -> Result<(), GeometryError> {
    let Some(stroke_width) = resolve_stroke_width(builder, aperture)? else {
        return Ok(());
    };

    let delta_x = to.x - from.x;
    let delta_y = to.y - from.y;
    let segment_length_sq = delta_x.mul_add(delta_x, delta_y * delta_y);
    if segment_length_sq <= f64::EPSILON {
        return handle_zero_length_segment(builder, from, aperture);
    }

    let segment_length = segment_length_sq.sqrt();
    let inverse_length = 1.0 / segment_length;
    let direction_x = delta_x * inverse_length;
    let direction_y = delta_y * inverse_length;
    let normal_x = -direction_y;
    let normal_y = direction_x;
    let half_width = stroke_width / 2.0;

    let start_left = Point {
        x: normal_x.mul_add(half_width, from.x),
        y: normal_y.mul_add(half_width, from.y),
    };
    let start_right = Point {
        x: (-normal_x).mul_add(half_width, from.x),
        y: (-normal_y).mul_add(half_width, from.y),
    };
    let end_right = Point {
        x: (-normal_x).mul_add(half_width, to.x),
        y: (-normal_y).mul_add(half_width, to.y),
    };
    let end_left = Point {
        x: normal_x.mul_add(half_width, to.x),
        y: normal_y.mul_add(half_width, to.y),
    };

    push_segment_body(builder, start_left, start_right, end_right, end_left);

    if matches!(aperture, Aperture::Circle(_)) {
        let direction_angle = direction_y.atan2(direction_x);
        push_semi_circle(
            builder,
            from,
            half_width,
            direction_angle + FRAC_PI_2,
            direction_angle + PI + FRAC_PI_2,
            CIRCLE_ENDCAP_SEGMENTS,
        );
        push_semi_circle(
            builder,
            to,
            half_width,
            direction_angle - FRAC_PI_2,
            direction_angle + FRAC_PI_2,
            CIRCLE_ENDCAP_SEGMENTS,
        );
    }

    Ok(())
}

fn handle_zero_length_segment(
    builder: &mut GeometryBuilder,
    position: Point,
    aperture: &Aperture,
) -> Result<(), GeometryError> {
    if matches!(aperture, Aperture::Circle(_)) {
        return flash_aperture(builder, aperture, position);
    }

    builder.warn("zero-length linear draw with non-circular aperture; skipping".to_string());
    Ok(())
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
            "aperture macro `{name}` is not supported by draw_linear"
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
        builder.warn(format!("{label} is zero; skipping stroke widening"));
        return Ok(None);
    }

    Ok(Some(normalized))
}

fn push_segment_body(
    builder: &mut GeometryBuilder,
    start_left: Point,
    start_right: Point,
    end_right: Point,
    end_left: Point,
) {
    let a = builder.push_vertex(start_left.x, start_left.y);
    let b = builder.push_vertex(start_right.x, start_right.y);
    let c = builder.push_vertex(end_right.x, end_right.y);
    let d = builder.push_vertex(end_left.x, end_left.y);
    builder.push_quad(a, b, c, d);
}

fn push_semi_circle(
    builder: &mut GeometryBuilder,
    center: Point,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    segments: u32,
) {
    let center_index = builder.push_vertex(center.x, center.y);
    let segment_count = segments.max(1);
    let angle_step = (end_angle - start_angle) / f64::from(segment_count);

    let mut previous_index: Option<u32> = None;
    for idx in 0..=segment_count {
        let angle = angle_step.mul_add(f64::from(idx), start_angle);
        let x = radius.mul_add(angle.cos(), center.x);
        let y = radius.mul_add(angle.sin(), center.y);
        let current_index = builder.push_vertex(x, y);
        if let Some(previous) = previous_index {
            builder.push_triangle(center_index, previous, current_index);
        }
        previous_index = Some(current_index);
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use gerber_types::{Circle, Rectangular};

    use super::*;

    const EPSILON: f64 = 1e-6;

    #[allow(clippy::needless_pass_by_value)]
    fn draw_and_build(
        from: Point,
        to: Point,
        aperture: Aperture,
    ) -> crate::geometry::LayerGeometry {
        let mut builder = GeometryBuilder::new();
        let result = draw_linear(&mut builder, from, to, &aperture);
        assert!(result.is_ok(), "expected draw_linear to succeed");
        builder.build()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_point(positions: &[f32], index: usize, expected_x: f64, expected_y: f64) {
        let base = index * 2;
        assert_close(f64::from(positions[base]), expected_x);
        assert_close(f64::from(positions[base + 1]), expected_y);
    }

    #[test]
    fn ut_str_001_horizontal_line_generates_quad() {
        let geom = draw_and_build(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Aperture::Rectangle(Rectangular::new(2.0, 2.0)),
        );

        assert_eq!(geom.vertex_count, 4);
        assert_eq!(geom.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_point(&geom.positions, 0, 0.0, 1.0);
        assert_point(&geom.positions, 1, 0.0, -1.0);
        assert_point(&geom.positions, 2, 10.0, -1.0);
        assert_point(&geom.positions, 3, 10.0, 1.0);
    }

    #[test]
    fn ut_str_002_vertical_line_generates_quad() {
        let geom = draw_and_build(
            Point { x: 0.0, y: 0.0 },
            Point { x: 0.0, y: 10.0 },
            Aperture::Rectangle(Rectangular::new(2.0, 2.0)),
        );

        assert_eq!(geom.vertex_count, 4);
        assert_eq!(geom.indices, vec![0, 1, 2, 0, 2, 3]);
        assert_point(&geom.positions, 0, -1.0, 0.0);
        assert_point(&geom.positions, 1, 1.0, 0.0);
        assert_point(&geom.positions, 2, 1.0, 10.0);
        assert_point(&geom.positions, 3, -1.0, 10.0);
    }

    #[test]
    fn ut_str_003_diagonal_line_quad_is_perpendicular_to_direction() {
        let geom = draw_and_build(
            Point { x: 0.0, y: 0.0 },
            Point { x: 3.0, y: 4.0 },
            Aperture::Rectangle(Rectangular::new(2.0, 2.0)),
        );

        assert_eq!(geom.vertex_count, 4);

        let x0 = f64::from(geom.positions[0]);
        let y0 = f64::from(geom.positions[1]);
        let x1 = f64::from(geom.positions[2]);
        let y1 = f64::from(geom.positions[3]);
        let edge_x = x1 - x0;
        let edge_y = y1 - y0;

        let direction_x = 3.0_f64 / 5.0_f64;
        let direction_y = 4.0_f64 / 5.0_f64;
        let dot = edge_x.mul_add(direction_x, edge_y * direction_y);
        assert_close(dot, 0.0);
    }

    #[test]
    fn ut_str_004_zero_length_line_with_circle_aperture_flashes_circle() {
        let geom = draw_and_build(
            Point { x: 5.0, y: 5.0 },
            Point { x: 5.0, y: 5.0 },
            Aperture::Circle(Circle::new(1.0)),
        );

        assert_eq!(geom.vertex_count, 32);
        assert_eq!(geom.indices.len(), 90);
        assert_close(geom.bounds.min_x, 4.5);
        assert_close(geom.bounds.max_x, 5.5);
        assert_close(geom.bounds.min_y, 4.5);
        assert_close(geom.bounds.max_y, 5.5);
    }

    #[test]
    fn ut_str_005_circular_aperture_adds_rounded_endcaps() {
        let geom = draw_and_build(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Aperture::Circle(Circle::new(2.0)),
        );

        assert!(geom.vertex_count > 4);
        assert!(geom.indices.len() > 6);
        assert_close(geom.bounds.min_x, -1.0);
        assert_close(geom.bounds.max_x, 11.0);
        assert_close(geom.bounds.min_y, -1.0);
        assert_close(geom.bounds.max_y, 1.0);
    }

    #[test]
    fn ut_str_006_rectangular_aperture_has_square_endcaps_only() {
        let geom = draw_and_build(
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Aperture::Rectangle(Rectangular::new(2.0, 2.0)),
        );

        assert_eq!(geom.vertex_count, 4);
        assert_eq!(geom.indices.len(), 6);
        assert_close(geom.bounds.min_x, 0.0);
        assert_close(geom.bounds.max_x, 10.0);
        assert_close(geom.bounds.min_y, -1.0);
        assert_close(geom.bounds.max_y, 1.0);
    }
}
