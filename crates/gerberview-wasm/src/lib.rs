#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::indexing_slicing)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

//! `GerberView` WASM module — Gerber/Excellon parsing and geometry conversion.

pub mod error;
pub mod excellon;
pub mod geometry;

use std::cell::RefCell;
use std::io::{BufReader, Cursor};

use wasm_bindgen::prelude::*;

use crate::geometry::types::saturate_u32;
use crate::geometry::{GeometryBuilder, LayerGeometry, LayerMeta};

thread_local! {
    static LAST_GEOMETRY: RefCell<Option<LayerGeometry>> = const { RefCell::new(None) };
}

fn store_geometry(geom: LayerGeometry) {
    LAST_GEOMETRY.with(|g| {
        *g.borrow_mut() = Some(geom);
    });
}

/// Initialize the WASM module. Sets up the panic hook for debugging.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Smoke-test export. Returns 42.
#[allow(clippy::missing_const_for_fn)]
#[wasm_bindgen]
pub fn ping() -> u32 {
    42
}

/// Parse a Gerber RS-274X file from raw bytes and generate renderable geometry.
///
/// Returns `LayerMeta` as a `JsValue` via `serde-wasm-bindgen`.
/// Geometry buffers are stored internally; retrieve with
/// [`get_positions`] and [`get_indices`].
///
/// # Errors
///
/// Returns a descriptive error string if parsing fails fatally.
#[wasm_bindgen]
pub fn parse_gerber(data: &[u8]) -> Result<JsValue, JsValue> {
    let meta = parse_gerber_internal(data).map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&meta).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Internal parse logic shared between the wasm export and native tests.
#[doc(hidden)]
pub fn parse_gerber_internal(data: &[u8]) -> Result<LayerMeta, String> {
    if data.is_empty() {
        return Err("empty input".to_string());
    }

    let reader = BufReader::new(Cursor::new(data));

    let doc = match gerber_parser::parse(reader) {
        Ok(doc) => doc,
        Err((doc, _parse_err)) => doc,
    };

    let geom = geometry::convert(&doc).map_err(|e| e.to_string())?;

    let meta = LayerMeta {
        bounds: geom.bounds,
        vertex_count: geom.vertex_count,
        index_count: saturate_u32(geom.indices.len()),
        command_count: geom.command_count,
        warning_count: saturate_u32(geom.warnings.len()),
        warnings: geom.warnings.clone(),
    };

    store_geometry(geom);

    Ok(meta)
}

/// Parse an Excellon drill file from raw bytes and generate renderable geometry.
///
/// Returns `LayerMeta` as a `JsValue` via `serde-wasm-bindgen`.
/// Geometry buffers are stored internally; retrieve with
/// [`get_positions`] and [`get_indices`].
///
/// # Errors
///
/// Returns a descriptive error string if parsing fails.
#[wasm_bindgen]
pub fn parse_excellon(data: &[u8]) -> Result<JsValue, JsValue> {
    let meta = parse_excellon_internal(data).map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&meta).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Internal parse logic shared between the wasm export and native tests.
#[doc(hidden)]
pub fn parse_excellon_internal(data: &[u8]) -> Result<LayerMeta, String> {
    let result = excellon::parser::parse(data).map_err(|err| err.to_string())?;

    let mut builder = GeometryBuilder::new();
    for warning in &result.warnings {
        builder.warn(warning.clone());
    }

    for hole in &result.holes {
        builder.push_ngon(hole.x, hole.y, hole.diameter / 2.0, 32);
    }

    let mut geom = builder.build();
    geom.command_count = saturate_u32(result.holes.len());

    let meta = LayerMeta {
        bounds: geom.bounds,
        vertex_count: geom.vertex_count,
        index_count: saturate_u32(geom.indices.len()),
        command_count: geom.command_count,
        warning_count: saturate_u32(geom.warnings.len()),
        warnings: geom.warnings.clone(),
    };

    store_geometry(geom);

    Ok(meta)
}

/// Retrieve the position buffer for the last parsed layer.
///
/// Returns a copy of the interleaved `[x0, y0, x1, y1, ...]` positions.
/// Returns an empty array if no layer has been parsed yet.
#[wasm_bindgen]
pub fn get_positions() -> Vec<f32> {
    LAST_GEOMETRY.with(|g| {
        g.borrow()
            .as_ref()
            .map_or_else(Vec::new, |geom| geom.positions.clone())
    })
}

/// Retrieve the index buffer for the last parsed layer.
///
/// Returns a copy of the triangle-list indices.
/// Returns an empty array if no layer has been parsed yet.
#[wasm_bindgen]
pub fn get_indices() -> Vec<u32> {
    LAST_GEOMETRY.with(|g| {
        g.borrow()
            .as_ref()
            .map_or_else(Vec::new, |geom| geom.indices.clone())
    })
}

/// Retrieve the clear-polarity index ranges for the last parsed layer.
///
/// Returns a flattened `[start0, end0, start1, end1, ...]` array of index
/// ranges that should be rendered with background color (clear polarity).
/// Returns an empty array if no layer has been parsed or there are no clear ranges.
#[wasm_bindgen]
pub fn get_clear_ranges() -> Vec<u32> {
    LAST_GEOMETRY.with(|g| {
        g.borrow().as_ref().map_or_else(Vec::new, |geom| {
            let mut flat = Vec::with_capacity(geom.clear_ranges.len() * 2);
            for &(start, end) in &geom.clear_ranges {
                flat.push(start);
                flat.push(end);
            }
            flat
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_returns_42() {
        assert_eq!(ping(), 42);
    }

    #[test]
    fn parse_gerber_valid_fixture() {
        let data = include_bytes!("../tests/fixtures/minimal/rectangle.gbr");
        let result = parse_gerber_internal(data);
        assert!(
            result.is_ok(),
            "expected Ok, got Err: {:?}",
            result.as_ref().err()
        );
        let Some(meta) = result.ok() else {
            return;
        };
        assert!(
            meta.command_count > 0,
            "expected commands from valid Gerber"
        );
        assert!(
            meta.vertex_count > 0,
            "expected non-empty geometry from valid Gerber"
        );
    }

    #[test]
    fn parse_gerber_empty_bytes() {
        let result = parse_gerber_internal(&[]);
        assert!(result.is_err(), "empty input should return Err");
    }

    #[test]
    fn parse_gerber_garbage_bytes() {
        let garbage: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0x02, 0x03];
        let result = parse_gerber_internal(garbage);
        // gerber_parser may return a partial doc with zero commands
        if let Ok(meta) = result {
            assert_eq!(meta.vertex_count, 0);
        }
    }

    #[test]
    fn parse_gerber_malformed_fixture() {
        let data = include_bytes!("../tests/fixtures/minimal/malformed.gbr");
        let result = parse_gerber_internal(data);
        assert!(
            result.is_ok(),
            "expected Ok for partial parse, got Err: {:?}",
            result.as_ref().err()
        );
        let Some(meta) = result.ok() else {
            return;
        };
        assert!(
            meta.command_count > 0,
            "partial parse should yield commands"
        );
    }

    #[test]
    fn parse_excellon_fixture() {
        let data = include_bytes!("../tests/fixtures/minimal/drill.drl");
        let result = parse_excellon_internal(data);
        assert!(
            result.is_ok(),
            "expected Ok, got Err: {:?}",
            result.as_ref().err()
        );
        let Some(meta) = result.ok() else {
            return;
        };
        assert!(meta.vertex_count > 0, "expected generated drill geometry");
        assert_eq!(meta.command_count, 5, "expected five drill commands");
    }

    #[test]
    fn get_buffers_empty_without_parse() {
        LAST_GEOMETRY.with(|g| {
            *g.borrow_mut() = None;
        });
        let positions = get_positions();
        let indices = get_indices();
        let clear_ranges = get_clear_ranges();
        assert!(positions.is_empty(), "no parse yet => empty positions");
        assert!(indices.is_empty(), "no parse yet => empty indices");
        assert!(
            clear_ranges.is_empty(),
            "no parse yet => empty clear ranges"
        );
    }

    #[test]
    fn get_clear_ranges_returns_flattened_pairs() {
        let mut geom = LayerGeometry {
            positions: vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0],
            indices: vec![0, 1, 2],
            bounds: geometry::BoundingBox::new(),
            command_count: 1,
            vertex_count: 3,
            warnings: Vec::new(),
            clear_ranges: vec![(0, 3), (6, 12)],
        };
        geom.bounds.update(0.0, 0.0);
        geom.bounds.update(1.0, 1.0);
        store_geometry(geom);
        let ranges = get_clear_ranges();
        assert_eq!(ranges, vec![0, 3, 6, 12]);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn wasm_ping_returns_42() {
        assert_eq!(ping(), 42);
    }
}
