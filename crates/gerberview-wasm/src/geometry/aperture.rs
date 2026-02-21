//! Aperture flash geometry expansion.
//!
//! This module converts Gerber D03 aperture flashes into triangle geometry
//! using the shared [`GeometryBuilder`].

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use gerber_types::{Aperture, Circle, Polygon, Rectangular};

use crate::error::GeometryError;

use super::types::{GeometryBuilder, Point};

const CIRCLE_SEGMENTS: u32 = 32;
const OBROUND_ENDCAP_SEGMENTS: u32 = 16;

/// Expand a flashed aperture at `position` into renderable triangles.
///
/// Supports standard Gerber apertures: circle, rectangle, obround, and polygon.
///
/// # Errors
///
/// Returns an error when the aperture variant is not supported by this module
/// or when aperture parameters are invalid (for example, non-finite values).
pub fn flash_aperture(
    builder: &mut GeometryBuilder,
    aperture: &Aperture,
    position: Point,
) -> Result<(), GeometryError> {
    match aperture {
        Aperture::Circle(circle) => flash_circle(builder, circle, position),
        Aperture::Rectangle(rectangle) => flash_rectangle(builder, rectangle, position),
        Aperture::Obround(obround) => flash_obround(builder, obround, position),
        Aperture::Polygon(polygon) => flash_polygon(builder, polygon, position),
        Aperture::Macro(name, _) => Err(GeometryError::UnsupportedFeature(format!(
            "aperture macro `{name}` is not supported by flash_aperture"
        ))),
    }
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
        builder.warn(format!("{label} is zero; skipping aperture flash"));
        return Ok(None);
    }

    Ok(Some(normalized))
}

fn flash_circle(
    builder: &mut GeometryBuilder,
    circle: &Circle,
    position: Point,
) -> Result<(), GeometryError> {
    let Some(diameter) = normalize_dimension(builder, circle.diameter, "circle diameter")? else {
        return Ok(());
    };

    let radius = diameter / 2.0;
    builder.push_ngon(position.x, position.y, radius, CIRCLE_SEGMENTS);
    Ok(())
}

fn flash_rectangle(
    builder: &mut GeometryBuilder,
    rectangle: &Rectangular,
    position: Point,
) -> Result<(), GeometryError> {
    let Some(width) = normalize_dimension(builder, rectangle.x, "rectangle width")? else {
        return Ok(());
    };
    let Some(height) = normalize_dimension(builder, rectangle.y, "rectangle height")? else {
        return Ok(());
    };

    push_centered_rectangle(builder, position, width, height);
    Ok(())
}

fn flash_obround(
    builder: &mut GeometryBuilder,
    obround: &Rectangular,
    position: Point,
) -> Result<(), GeometryError> {
    let Some(width) = normalize_dimension(builder, obround.x, "obround width")? else {
        return Ok(());
    };
    let Some(height) = normalize_dimension(builder, obround.y, "obround height")? else {
        return Ok(());
    };

    if (width - height).abs() <= f64::EPSILON {
        builder.push_ngon(position.x, position.y, width / 2.0, CIRCLE_SEGMENTS);
        return Ok(());
    }

    if width > height {
        let radius = height / 2.0;
        let body_width = width - height;
        let half_body = body_width / 2.0;

        if body_width > f64::EPSILON {
            push_centered_rectangle(builder, position, body_width, height);
        }

        push_semi_circle(
            builder,
            Point {
                x: position.x - half_body,
                y: position.y,
            },
            radius,
            FRAC_PI_2,
            3.0 * FRAC_PI_2,
            OBROUND_ENDCAP_SEGMENTS,
        );

        push_semi_circle(
            builder,
            Point {
                x: position.x + half_body,
                y: position.y,
            },
            radius,
            -FRAC_PI_2,
            FRAC_PI_2,
            OBROUND_ENDCAP_SEGMENTS,
        );
    } else {
        let radius = width / 2.0;
        let body_height = height - width;
        let half_body = body_height / 2.0;

        if body_height > f64::EPSILON {
            push_centered_rectangle(builder, position, width, body_height);
        }

        push_semi_circle(
            builder,
            Point {
                x: position.x,
                y: position.y + half_body,
            },
            radius,
            0.0,
            PI,
            OBROUND_ENDCAP_SEGMENTS,
        );

        push_semi_circle(
            builder,
            Point {
                x: position.x,
                y: position.y - half_body,
            },
            radius,
            PI,
            TAU,
            OBROUND_ENDCAP_SEGMENTS,
        );
    }

    Ok(())
}

fn flash_polygon(
    builder: &mut GeometryBuilder,
    polygon: &Polygon,
    position: Point,
) -> Result<(), GeometryError> {
    let Some(diameter) = normalize_dimension(builder, polygon.diameter, "polygon diameter")? else {
        return Ok(());
    };

    if polygon.vertices < 3 {
        return Err(GeometryError::InvalidAperture(format!(
            "polygon has {} vertices; expected at least 3",
            polygon.vertices
        )));
    }

    let rotation_degrees = polygon.rotation.unwrap_or(0.0);
    if !rotation_degrees.is_finite() {
        return Err(GeometryError::InvalidAperture(format!(
            "polygon rotation must be finite, got {rotation_degrees}"
        )));
    }

    let sides = u32::from(polygon.vertices);
    let radius = diameter / 2.0;
    let rotation = rotation_degrees.to_radians();

    let mut first_index: Option<u32> = None;
    for i in 0..sides {
        let angle = rotation + TAU * f64::from(i) / f64::from(sides);
        let x = radius.mul_add(angle.cos(), position.x);
        let y = radius.mul_add(angle.sin(), position.y);
        let index = builder.push_vertex(x, y);
        if first_index.is_none() {
            first_index = Some(index);
        }
    }

    let Some(first) = first_index else {
        return Err(GeometryError::DegenerateGeometry(
            "polygon produced no vertices".to_string(),
        ));
    };

    for i in 1..sides.saturating_sub(1) {
        let b = add_index(first, i)?;
        let c = add_index(first, i + 1)?;
        builder.push_triangle(first, b, c);
    }

    Ok(())
}

fn add_index(base: u32, offset: u32) -> Result<u32, GeometryError> {
    base.checked_add(offset).ok_or_else(|| {
        GeometryError::DegenerateGeometry("vertex index overflow while triangulating".to_string())
    })
}

fn push_centered_rectangle(builder: &mut GeometryBuilder, center: Point, width: f64, height: f64) {
    let half_width = width / 2.0;
    let half_height = height / 2.0;

    let a = builder.push_vertex(center.x - half_width, center.y - half_height);
    let b = builder.push_vertex(center.x + half_width, center.y - half_height);
    let c = builder.push_vertex(center.x + half_width, center.y + half_height);
    let d = builder.push_vertex(center.x - half_width, center.y + half_height);
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
    let step = (end_angle - start_angle) / f64::from(segment_count);

    let mut previous_index: Option<u32> = None;
    for i in 0..=segment_count {
        let angle = start_angle + step * f64::from(i);
        let x = radius.mul_add(angle.cos(), center.x);
        let y = radius.mul_add(angle.sin(), center.y);
        let index = builder.push_vertex(x, y);
        if let Some(previous) = previous_index {
            builder.push_triangle(center_index, previous, index);
        }
        previous_index = Some(index);
    }
}

#[cfg(test)]
mod tests {
    use gerber_types::{Circle, Polygon, Rectangular};

    use super::*;

    const EPSILON: f64 = 1e-6;

    #[allow(clippy::needless_pass_by_value)]
    fn flash_and_build(aperture: Aperture, position: Point) -> crate::geometry::LayerGeometry {
        let mut builder = GeometryBuilder::new();
        let result = flash_aperture(&mut builder, &aperture, position);
        assert!(result.is_ok(), "expected aperture flash to succeed");
        builder.build()
    }

    #[test]
    fn ut_apr_001_circle_aperture_generates_ngon_vertices() {
        let geom = flash_and_build(Aperture::Circle(Circle::new(1.0)), Point { x: 0.0, y: 0.0 });
        assert_eq!(geom.vertex_count, 32);
        assert_eq!(geom.indices.len(), 90);
    }

    #[test]
    fn ut_apr_002_circle_vertices_are_expected_distance_from_center() {
        let geom = flash_and_build(Aperture::Circle(Circle::new(2.0)), Point { x: 5.0, y: 3.0 });
        for pair in geom.positions.chunks_exact(2) {
            if let [x, y] = pair {
                let dx = f64::from(*x) - 5.0;
                let dy = f64::from(*y) - 3.0;
                let distance = dx.hypot(dy);
                assert!((distance - 1.0).abs() < EPSILON);
            }
        }
    }

    #[test]
    fn ut_apr_003_rectangle_aperture_generates_expected_corners() {
        let geom = flash_and_build(
            Aperture::Rectangle(Rectangular::new(2.0, 1.0)),
            Point { x: 0.0, y: 0.0 },
        );
        assert_eq!(geom.vertex_count, 4);
        assert_eq!(
            geom.positions,
            vec![-1.0_f32, -0.5_f32, 1.0_f32, -0.5_f32, 1.0_f32, 0.5_f32, -1.0_f32, 0.5_f32,]
        );
    }

    #[test]
    fn ut_apr_004_rectangle_aperture_generates_two_triangles() {
        let geom = flash_and_build(
            Aperture::Rectangle(Rectangular::new(2.0, 1.0)),
            Point { x: 0.0, y: 0.0 },
        );
        assert_eq!(geom.indices, vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn ut_apr_005_obround_horizontal_builds_expected_bounds() {
        let geom = flash_and_build(
            Aperture::Obround(Rectangular::new(3.0, 1.0)),
            Point { x: 0.0, y: 0.0 },
        );
        assert!(geom.vertex_count > 4);
        assert!((geom.bounds.min_x + 1.5).abs() < EPSILON);
        assert!((geom.bounds.max_x - 1.5).abs() < EPSILON);
        assert!((geom.bounds.min_y + 0.5).abs() < EPSILON);
        assert!((geom.bounds.max_y - 0.5).abs() < EPSILON);
    }

    #[test]
    fn ut_apr_006_obround_vertical_builds_expected_bounds() {
        let geom = flash_and_build(
            Aperture::Obround(Rectangular::new(1.0, 3.0)),
            Point { x: 0.0, y: 0.0 },
        );
        assert!(geom.vertex_count > 4);
        assert!((geom.bounds.min_x + 0.5).abs() < EPSILON);
        assert!((geom.bounds.max_x - 0.5).abs() < EPSILON);
        assert!((geom.bounds.min_y + 1.5).abs() < EPSILON);
        assert!((geom.bounds.max_y - 1.5).abs() < EPSILON);
    }

    #[test]
    fn ut_apr_007_polygon_aperture_generates_rotation() {
        let geom = flash_and_build(
            Aperture::Polygon(Polygon::new(2.0, 6).with_rotation(30.0)),
            Point { x: 0.0, y: 0.0 },
        );
        assert_eq!(geom.vertex_count, 6);
        let mut pairs = geom.positions.chunks_exact(2);
        let first_pair = pairs.next();
        assert!(first_pair.is_some(), "expected first polygon vertex");
        if let Some([x, y]) = first_pair {
            assert!((f64::from(*x) - (30.0_f64.to_radians().cos())).abs() < EPSILON);
            assert!((f64::from(*y) - (30.0_f64.to_radians().sin())).abs() < EPSILON);
        }
    }

    #[test]
    fn ut_apr_008_zero_diameter_circle_skips_with_warning() {
        let mut builder = GeometryBuilder::new();
        let result = flash_aperture(
            &mut builder,
            &Aperture::Circle(Circle::new(0.0)),
            Point { x: 0.0, y: 0.0 },
        );
        assert!(result.is_ok());
        let geom = builder.build();
        assert_eq!(geom.vertex_count, 0);
        assert!(geom.warnings.iter().any(|msg| msg.contains("zero")));
    }

    #[test]
    fn ut_apr_009_negative_rectangle_dimensions_use_abs_with_warning() {
        let mut builder = GeometryBuilder::new();
        let result = flash_aperture(
            &mut builder,
            &Aperture::Rectangle(Rectangular::new(-2.0, -1.0)),
            Point { x: 0.0, y: 0.0 },
        );
        assert!(result.is_ok());
        let geom = builder.build();
        assert_eq!(geom.vertex_count, 4);
        assert!((geom.bounds.min_x + 1.0).abs() < EPSILON);
        assert!((geom.bounds.max_x - 1.0).abs() < EPSILON);
        assert!((geom.bounds.min_y + 0.5).abs() < EPSILON);
        assert!((geom.bounds.max_y - 0.5).abs() < EPSILON);
        assert!(geom.warnings.iter().any(|msg| msg.contains("negative")));
    }
}
