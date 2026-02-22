//! Region fill triangulation for G36/G37 commands.
//!
//! Converts closed polygon boundaries into triangle geometry using the
//! `earclip` ear-clipping triangulation algorithm.

use crate::error::GeometryError;

use super::types::{GeometryBuilder, Point};

const POINT_EQUALITY_EPSILON: f64 = 1e-9;

/// Fill a closed polygon region by triangulating its boundary.
///
/// Boundary points are expected to be pre-tessellated (arc segments already
/// converted to line segments by the caller). The function auto-closes the
/// polygon when the last point does not coincide with the first.
///
/// # Errors
///
/// Returns [`GeometryError::RegionError`] if vertex index arithmetic overflows.
/// Degenerate boundaries (fewer than 3 points) are handled gracefully with a
/// warning and no geometry output.
pub fn fill_region(builder: &mut GeometryBuilder, boundary: &[Point]) -> Result<(), GeometryError> {
    if boundary.len() < 3 {
        builder.warn(format!(
            "region boundary has {} point(s); need at least 3; skipping region",
            boundary.len()
        ));
        return Ok(());
    }

    let needs_close = !points_approx_equal(
        boundary
            .first()
            .copied()
            .unwrap_or(Point { x: 0.0, y: 0.0 }),
        boundary.last().copied().unwrap_or(Point { x: 0.0, y: 0.0 }),
    );

    let effective_len = boundary.len() + usize::from(needs_close);
    let mut flat = Vec::with_capacity(effective_len * 2);
    for pt in boundary {
        flat.push(pt.x);
        flat.push(pt.y);
    }
    if needs_close {
        if let Some(first) = boundary.first() {
            flat.push(first.x);
            flat.push(first.y);
            builder.warn(
                "region boundary is not closed; auto-closing by appending first point".to_string(),
            );
        }
    }

    let indices = earclip::earcut::earcut(&flat, &[], 2);

    if indices.is_empty() {
        builder.warn("earclip produced no triangles for region; skipping".to_string());
        return Ok(());
    }

    let base_vertex = emit_vertices(builder, &flat);
    emit_triangles(builder, &indices, base_vertex)
}

/// Push all vertices from the flat coordinate buffer and return the first vertex index.
fn emit_vertices(builder: &mut GeometryBuilder, flat: &[f64]) -> u32 {
    let mut first: Option<u32> = None;
    let mut pairs = flat.chunks_exact(2);
    for pair in pairs.by_ref() {
        if let [x, y] = *pair {
            let idx = builder.push_vertex(x, y);
            if first.is_none() {
                first = Some(idx);
            }
        }
    }
    first.unwrap_or(0)
}

/// Convert earclip triangle indices (relative to the flat buffer) into
/// `GeometryBuilder` triangle calls using the base vertex offset.
fn emit_triangles(
    builder: &mut GeometryBuilder,
    indices: &[usize],
    base_vertex: u32,
) -> Result<(), GeometryError> {
    for tri in indices.chunks_exact(3) {
        if let [ia, ib, ic] = *tri {
            let a = offset_index(base_vertex, ia)?;
            let b = offset_index(base_vertex, ib)?;
            let c = offset_index(base_vertex, ic)?;
            builder.push_triangle(a, b, c);
        }
    }
    Ok(())
}

fn offset_index(base: u32, offset: usize) -> Result<u32, GeometryError> {
    let offset_u32 =
        u32::try_from(offset).map_err(|_| GeometryError::RegionError("index overflow".into()))?;
    base.checked_add(offset_u32)
        .ok_or_else(|| GeometryError::RegionError("vertex index overflow".into()))
}

fn points_approx_equal(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= POINT_EQUALITY_EPSILON && (a.y - b.y).abs() <= POINT_EQUALITY_EPSILON
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn fill_and_build(boundary: &[Point]) -> crate::geometry::LayerGeometry {
        let mut builder = GeometryBuilder::new();
        let result = fill_region(&mut builder, boundary);
        assert!(result.is_ok(), "expected fill_region to succeed");
        builder.build()
    }

    fn triangle_count(geom: &crate::geometry::LayerGeometry) -> usize {
        geom.indices.len() / 3
    }

    // --- UT-REG-001: Square region produces 2 triangles ---

    #[test]
    fn ut_reg_001_square_region_produces_two_triangles() {
        let boundary = &[
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ];
        let geom = fill_and_build(boundary);
        assert_eq!(triangle_count(&geom), 2);
        assert_eq!(geom.indices.len(), 6);
    }

    // --- UT-REG-002: L-shaped region produces >= 4 triangles ---

    #[test]
    fn ut_reg_002_l_shape_produces_at_least_four_triangles() {
        let boundary = &[
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 0.0 },
            Point { x: 2.0, y: 1.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 1.0, y: 2.0 },
            Point { x: 0.0, y: 2.0 },
        ];
        let geom = fill_and_build(boundary);
        assert!(
            triangle_count(&geom) >= 4,
            "expected >= 4 triangles for L-shape, got {}",
            triangle_count(&geom)
        );
        assert!(geom.vertex_count > 0);
    }

    // --- UT-REG-003: Triangle region produces exactly 1 triangle ---

    #[test]
    fn ut_reg_003_triangle_region_produces_one_triangle() {
        let boundary = &[
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.5, y: 1.0 },
        ];
        let geom = fill_and_build(boundary);
        assert_eq!(triangle_count(&geom), 1);
        assert_eq!(geom.indices.len(), 3);
    }

    // --- UT-REG-004: Concave polygon produces valid triangulation ---

    #[test]
    fn ut_reg_004_concave_arrow_polygon_produces_valid_triangulation() {
        let boundary = &[
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 1.0 },
            Point { x: 0.0, y: 2.0 },
            Point { x: 0.5, y: 1.0 },
        ];
        let geom = fill_and_build(boundary);
        assert!(triangle_count(&geom) >= 2);
        assert!(geom.vertex_count > 0);
        assert!(geom.warnings.is_empty() || geom.warnings.iter().all(|w| !w.contains("skip")));
    }

    // --- UT-REG-005: 2-point degenerate boundary skips with warning (BC-GBR-016) ---

    #[test]
    fn ut_reg_005_two_point_boundary_skips_with_warning() {
        let boundary = &[Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 1.0 }];
        let geom = fill_and_build(boundary);
        assert_eq!(geom.vertex_count, 0);
        assert_eq!(geom.indices.len(), 0);
        assert!(geom.warnings.iter().any(|w| w.contains("2 point(s)")));
    }

    // --- UT-REG-006: 1-point degenerate boundary skips with warning (BC-GBR-016) ---

    #[test]
    fn ut_reg_006_one_point_boundary_skips_with_warning() {
        let boundary = &[Point { x: 5.0, y: 5.0 }];
        let geom = fill_and_build(boundary);
        assert_eq!(geom.vertex_count, 0);
        assert_eq!(geom.indices.len(), 0);
        assert!(geom.warnings.iter().any(|w| w.contains("1 point(s)")));
    }

    // --- UT-REG-007: Self-intersecting bowtie best-effort (BC-GBR-018) ---

    #[test]
    fn ut_reg_007_self_intersecting_bowtie_best_effort() {
        let boundary = &[
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 2.0 },
            Point { x: 2.0, y: 0.0 },
            Point { x: 0.0, y: 2.0 },
        ];
        let mut builder = GeometryBuilder::new();
        let result = fill_region(&mut builder, boundary);
        assert!(
            result.is_ok(),
            "self-intersecting region must not panic or error"
        );
        let geom = builder.build();
        assert!(
            triangle_count(&geom) >= 1,
            "earclip should produce best-effort triangles for bowtie"
        );
    }

    // --- UT-REG-008: Pre-tessellated arc boundary triangulates normally ---

    #[test]
    fn ut_reg_008_arc_boundary_pretessellated_triangulates() {
        let segments: u32 = 8;
        let radius = 5.0;
        let capacity = usize::try_from(segments).unwrap_or(8) + 2;
        let mut boundary = Vec::with_capacity(capacity);
        for i in 0..=segments {
            let angle = std::f64::consts::PI * f64::from(i) / f64::from(segments);
            boundary.push(Point {
                x: radius * angle.cos(),
                y: radius * angle.sin(),
            });
        }
        boundary.push(Point { x: -radius, y: 0.0 });

        let geom = fill_and_build(&boundary);
        assert!(geom.vertex_count > 0, "expected vertices from arc boundary");
        assert!(
            triangle_count(&geom) >= 1,
            "expected triangles from arc boundary"
        );
    }

    // --- BC-GBR-016: Empty boundary (0 points) ---

    #[test]
    fn bc_gbr_016_empty_boundary_skips_with_warning() {
        let geom = fill_and_build(&[]);
        assert_eq!(geom.vertex_count, 0);
        assert_eq!(geom.indices.len(), 0);
        assert!(geom.warnings.iter().any(|w| w.contains("0 point(s)")));
    }

    // --- BC-GBR-017: Unclosed polygon auto-closes ---

    #[test]
    fn bc_gbr_017_unclosed_polygon_auto_closes() {
        let boundary = &[
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 0.0, y: 1.0 },
        ];
        let mut builder = GeometryBuilder::new();
        let result = fill_region(&mut builder, boundary);
        assert!(result.is_ok());
        let geom = builder.build();
        assert!(
            geom.warnings.iter().any(|w| w.contains("auto-closing")),
            "expected auto-close warning"
        );
        assert!(triangle_count(&geom) >= 2);
    }

    // --- BC-GBR-018: Self-intersecting region produces non-empty best-effort ---

    #[test]
    fn bc_gbr_018_self_intersecting_region_best_effort() {
        let boundary = &[
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.0, y: 1.0 },
        ];
        let mut builder = GeometryBuilder::new();
        let result = fill_region(&mut builder, boundary);
        assert!(result.is_ok(), "must not error on self-intersecting region");
        let geom = builder.build();
        assert!(geom.vertex_count > 0, "best-effort should produce vertices");
    }
}
