//! Step-repeat geometry duplication.
//!
//! Duplicates vertex ranges with X/Y offsets for each grid position.

use crate::error::GeometryError;

use super::types::{GeometryBuilder, LayerGeometry};

const BC_GBR_020: &str = "BC-GBR-020: step-repeat with zero count in X or Y; skipping block";

/// Applies step-repeat by duplicating block geometry at each grid position.
///
/// For each `(ix, iy)` in `0..repeat_x` × `0..repeat_y`, adds a copy of
/// `block_geometry` offset by `(ix * step_x, iy * step_y)`.
///
/// # Errors
///
/// Returns an error if geometry is invalid.
///
/// # Boundary conditions
///
/// - BC-GBR-020: If `repeat_x == 0` or `repeat_y == 0`, warns and returns
///   `Ok(())` without adding geometry.
/// - BC-GBR-019: Nested step-repeat is flattened by applying this function
///   to already-step-repeated geometry.
pub fn apply_step_repeat(
    builder: &mut GeometryBuilder,
    block_geometry: &LayerGeometry,
    repeat_x: u32,
    repeat_y: u32,
    step_x: f64,
    step_y: f64,
) -> Result<(), GeometryError> {
    if repeat_x == 0 || repeat_y == 0 {
        builder.warn(BC_GBR_020.to_string());
        return Ok(());
    }

    let positions = &block_geometry.positions;
    let indices = &block_geometry.indices;
    let vertex_count = block_geometry.vertex_count as usize;

    for iy in 0..repeat_y {
        for ix in 0..repeat_x {
            let offset_x = f64::from(ix) * step_x;
            let offset_y = f64::from(iy) * step_y;

            let base = builder.vertex_count();

            for v in 0..vertex_count {
                let i = v * 2;
                let x = positions
                    .get(i)
                    .and_then(|a| positions.get(i + 1).map(|b| (a, b)));
                let Some((x, y)) = x else {
                    return Err(GeometryError::DegenerateGeometry(
                        "block has incomplete vertex data".to_string(),
                    ));
                };
                let x_val = f64::from(*x) + offset_x;
                let y_val = f64::from(*y) + offset_y;
                builder.push_vertex(x_val, y_val);
            }

            let base_u = base;
            for chunk in indices.chunks_exact(3) {
                let (Some(&a), Some(&b), Some(&c)) = (chunk.first(), chunk.get(1), chunk.get(2))
                else {
                    return Err(GeometryError::DegenerateGeometry(
                        "block has invalid index".to_string(),
                    ));
                };
                if a >= block_geometry.vertex_count
                    || b >= block_geometry.vertex_count
                    || c >= block_geometry.vertex_count
                {
                    return Err(GeometryError::DegenerateGeometry(
                        "block has invalid index".to_string(),
                    ));
                }
                builder.push_triangle(base_u + a, base_u + b, base_u + c);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_simple_block() -> LayerGeometry {
        let mut b = GeometryBuilder::new();
        b.push_vertex(0.0, 0.0);
        b.push_vertex(1.0, 0.0);
        b.push_vertex(0.0, 1.0);
        b.push_triangle(0, 1, 2);
        b.build()
    }

    #[test]
    fn ut_sr_001_two_by_three_produces_six_copies() {
        let block = make_simple_block();
        let block_vertices = block.vertex_count;
        let block_indices = block.indices.len();

        let mut builder = GeometryBuilder::new();
        let result = apply_step_repeat(&mut builder, &block, 2, 3, 10.0, 5.0);
        assert!(result.is_ok());

        let geom = builder.build();
        assert_eq!(geom.vertex_count, block_vertices * 6);
        assert_eq!(geom.indices.len(), block_indices * 6);
    }

    #[test]
    fn ut_sr_002_step_repeat_with_spacing_offsets_correctly() {
        let block = make_simple_block();

        let mut builder = GeometryBuilder::new();
        let result = apply_step_repeat(&mut builder, &block, 2, 2, 5.0, 3.0);
        assert!(result.is_ok());

        let geom = builder.build();
        let positions = &geom.positions;

        let eps = 1e-6_f32;
        let first_tri_first_vertex_x = positions.first().copied().unwrap_or(0.0);
        let first_tri_first_vertex_y = positions.get(1).copied().unwrap_or(0.0);
        assert!((first_tri_first_vertex_x - 0.0).abs() < eps);
        assert!((first_tri_first_vertex_y - 0.0).abs() < eps);

        let second_copy_first_vertex_idx = 3;
        let second_x = positions
            .get(second_copy_first_vertex_idx * 2)
            .copied()
            .unwrap_or(0.0);
        let second_y = positions
            .get(second_copy_first_vertex_idx * 2 + 1)
            .copied()
            .unwrap_or(0.0);
        assert!((second_x - 5.0).abs() < eps);
        assert!((second_y - 0.0).abs() < eps);
    }

    #[test]
    fn ut_sr_003_zero_count_in_x_skips_with_warn() {
        let block = make_simple_block();
        let mut builder = GeometryBuilder::new();

        let result = apply_step_repeat(&mut builder, &block, 0, 3, 1.0, 1.0);
        assert!(result.is_ok());

        let geom = builder.build();
        assert_eq!(geom.vertex_count, 0);
        assert!(geom.warnings.iter().any(|w| w.contains("BC-GBR-020")));
    }

    #[test]
    fn bc_gbr_019_nested_step_repeat_flattens() {
        let block = make_simple_block();

        let mut inner_builder = GeometryBuilder::new();
        let inner_result = apply_step_repeat(&mut inner_builder, &block, 2, 1, 2.0, 0.0);
        assert!(inner_result.is_ok(), "inner step-repeat should succeed");
        let inner_geom = inner_builder.build();

        let mut outer_builder = GeometryBuilder::new();
        let outer_result = apply_step_repeat(&mut outer_builder, &inner_geom, 1, 2, 0.0, 4.0);
        assert!(outer_result.is_ok(), "outer step-repeat should succeed");
        let outer_geom = outer_builder.build();

        assert_eq!(outer_geom.vertex_count, block.vertex_count * 4);
        assert_eq!(outer_geom.indices.len(), block.indices.len() * 4);
    }
}
