//! Integration tests for Gerber parsing (IT-001 through IT-007, excluding IT-005).

use gerberview_wasm::parse_gerber_internal;
use std::time::Instant;

/// IT-001: Parse KiCad Gerber → non-empty command list.
#[test]
#[allow(clippy::expect_used)]
fn it_001_kicad_parse_non_empty_commands() {
    let data = include_bytes!("fixtures/kicad-sample/board-F_Cu.gbr");
    let result = parse_gerber_internal(data);
    assert!(
        result.is_ok(),
        "expected Ok, got Err: {:?}",
        result.as_ref().err()
    );
    let meta = result.as_ref().expect("assert!(result.is_ok()) above");
    assert!(
        meta.command_count > 0,
        "KiCad copper layer should have non-empty command list"
    );
}

/// IT-002: Parse KiCad copper layer → geometry with `vertex_count` > 0.
#[test]
#[allow(clippy::expect_used)]
fn it_002_kicad_parse_produces_geometry() {
    let data = include_bytes!("fixtures/kicad-sample/board-F_Cu.gbr");
    let result = parse_gerber_internal(data);
    assert!(
        result.is_ok(),
        "expected Ok, got Err: {:?}",
        result.as_ref().err()
    );
    let meta = result.as_ref().expect("assert!(result.is_ok()) above");
    assert!(
        meta.vertex_count > 0,
        "KiCad copper layer should produce non-empty geometry"
    );
}

/// IT-003: Parse → geometry → bounds within expected range (Arduino Uno ~68.6×53.4mm).
#[test]
#[allow(clippy::expect_used)]
fn it_003_arduino_bounds_within_expected_range() {
    let data = include_bytes!("fixtures/arduino-uno/arduino-uno.GTL");
    let result = parse_gerber_internal(data);
    assert!(
        result.is_ok(),
        "expected Ok, got Err: {:?}",
        result.as_ref().err()
    );
    let meta = result.as_ref().expect("assert!(result.is_ok()) above");
    let b = &meta.bounds;
    assert!(
        b.max_x >= b.min_x && b.max_y >= b.min_y,
        "bounds should be valid"
    );
    assert!(
        (b.max_x - b.min_x).abs() < 100.0 && (b.max_y - b.min_y).abs() < 100.0,
        "Arduino Uno bounds should be within ~100mm"
    );
}

/// IT-004: Parse all layers in KiCad sample → all produce geometry.
#[test]
#[allow(clippy::expect_used)]
fn it_004_kicad_all_layers_produce_geometry() {
    let layer_results: &[(&str, &[u8])] = &[
        (
            "board-F_Cu.gbr",
            include_bytes!("fixtures/kicad-sample/board-F_Cu.gbr").as_slice(),
        ),
        (
            "board-B_Cu.gbr",
            include_bytes!("fixtures/kicad-sample/board-B_Cu.gbr").as_slice(),
        ),
        (
            "board-F_Mask.gbr",
            include_bytes!("fixtures/kicad-sample/board-F_Mask.gbr").as_slice(),
        ),
        (
            "board-B_Mask.gbr",
            include_bytes!("fixtures/kicad-sample/board-B_Mask.gbr").as_slice(),
        ),
        (
            "board-F_SilkS.gbr",
            include_bytes!("fixtures/kicad-sample/board-F_SilkS.gbr").as_slice(),
        ),
        (
            "board-B_SilkS.gbr",
            include_bytes!("fixtures/kicad-sample/board-B_SilkS.gbr").as_slice(),
        ),
        (
            "board-Edge_Cuts.gbr",
            include_bytes!("fixtures/kicad-sample/board-Edge_Cuts.gbr").as_slice(),
        ),
    ];
    for (layer_name, data) in layer_results {
        let result = parse_gerber_internal(data);
        assert!(
            result.is_ok(),
            "layer {} should parse: {:?}",
            layer_name,
            result.as_ref().err()
        );
        let meta = result.as_ref().expect("assert!(result.is_ok()) above");
        assert!(
            meta.command_count > 0 || meta.vertex_count > 0,
            "layer {} should produce geometry",
            layer_name
        );
    }
}

/// IT-006: Large file parse time < 2000ms.
#[test]
#[allow(clippy::expect_used)]
fn it_006_large_file_parse_time_under_2000ms() {
    let data = include_bytes!("fixtures/kicad-sample/board-F_Cu.gbr");
    let start = Instant::now();
    let result = parse_gerber_internal(data);
    let elapsed = start.elapsed();
    assert!(
        result.is_ok(),
        "parse should succeed: {:?}",
        result.as_ref().err()
    );
    assert!(
        elapsed.as_millis() < 2000,
        "parse should complete in < 2000ms, took {}ms",
        elapsed.as_millis()
    );
}

/// IT-007: Parse malformed file → partial result + error, no panic.
#[test]
#[allow(clippy::expect_used)]
fn it_007_malformed_partial_result_no_panic() {
    let data = include_bytes!("fixtures/minimal/malformed.gbr");
    let result = parse_gerber_internal(data);
    assert!(
        result.is_ok(),
        "malformed file should yield partial Ok result, got Err: {:?}",
        result.as_ref().err()
    );
    let meta = result.as_ref().expect("assert!(result.is_ok()) above");
    assert!(
        meta.command_count > 0,
        "partial parse should yield commands from valid prefix"
    );
}
