//! Integration tests for geometry conversion.

use gerberview_wasm::{geometry, get_indices, get_positions, parse_gerber_internal};
use std::io::{BufReader, Cursor};

/// Parse KiCad copper layer → geometry with valid positions.len() == vertex_count * 2, all indices valid.
#[test]
#[allow(clippy::expect_used)]
fn kicad_copper_geometry_invariants() {
    let data = include_bytes!("fixtures/kicad-sample/board-F_Cu.gbr");
    let result = parse_gerber_internal(data);
    let meta = result.as_ref().expect("parse should succeed");
    assert!(meta.vertex_count > 0, "should produce geometry");

    let positions = get_positions();
    let indices = get_indices();

    assert_eq!(
        positions.len(),
        meta.vertex_count as usize * 2,
        "positions length should match vertex_count * 2"
    );

    let max_idx = positions.len() / 2;
    for idx in &indices {
        assert!(
            (*idx as usize) < max_idx,
            "index {} out of bounds for {} vertices",
            idx,
            max_idx
        );
    }
}

/// Parse minimal rectangle → non-empty geometry (convert_rectangle_produces_geometry).
#[test]
#[allow(clippy::expect_used)]
fn convert_rectangle_produces_geometry() {
    let data = include_bytes!("fixtures/minimal/rectangle.gbr");
    let reader = BufReader::new(Cursor::new(data.as_slice()));
    let doc = match gerber_parser::parse(reader) {
        Ok(d) | Err((d, _)) => d,
    };
    let geom = geometry::convert(&doc).expect("convert should succeed");
    assert!(
        geom.vertex_count > 0,
        "rectangle fixture should produce non-empty geometry"
    );
    assert_eq!(
        geom.positions.len(),
        geom.vertex_count as usize * 2,
        "positions length should match vertex_count * 2"
    );
}

/// Parse minimal circle → non-empty geometry.
#[test]
#[allow(clippy::expect_used)]
fn convert_circle_produces_geometry() {
    let data = include_bytes!("fixtures/minimal/circle.gbr");
    let reader = BufReader::new(Cursor::new(data.as_slice()));
    let doc = match gerber_parser::parse(reader) {
        Ok(d) | Err((d, _)) => d,
    };
    let geom = geometry::convert(&doc).expect("convert should succeed");
    assert!(
        geom.vertex_count > 0,
        "circle fixture should produce non-empty geometry"
    );
}

/// Parse minimal region → non-empty geometry.
#[test]
#[allow(clippy::expect_used)]
fn convert_region_produces_geometry() {
    let data = include_bytes!("fixtures/minimal/region.gbr");
    let reader = BufReader::new(Cursor::new(data.as_slice()));
    let doc = match gerber_parser::parse(reader) {
        Ok(d) | Err((d, _)) => d,
    };
    let geom = geometry::convert(&doc).expect("convert should succeed");
    assert!(
        geom.vertex_count > 0,
        "region fixture should produce non-empty geometry"
    );
}
