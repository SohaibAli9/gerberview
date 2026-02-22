//! Integration tests for Excellon drill parsing.

use gerberview_wasm::parse_excellon_internal;

/// Parse Excellon drill fixture → correct hole count, positions.
#[test]
#[allow(clippy::expect_used)]
fn excellon_minimal_drill_hole_count_and_positions() {
    let data = include_bytes!("fixtures/minimal/drill.drl");
    let result = parse_excellon_internal(data);
    assert!(
        result.is_ok(),
        "expected Ok, got Err: {:?}",
        result.as_ref().err()
    );
    let meta = result.as_ref().expect("assert!(result.is_ok()) above");
    assert!(meta.vertex_count > 0, "expected generated drill geometry");
    assert_eq!(meta.command_count, 5, "expected five drill commands");
}

/// Parse Arduino drill file → bounds match expected.
#[test]
#[allow(clippy::expect_used)]
fn excellon_arduino_drill_bounds_match_expected() {
    let data = include_bytes!("fixtures/arduino-uno/arduino-uno.drl");
    let result = parse_excellon_internal(data);
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
        "Arduino drill bounds should be within ~100mm"
    );
}
