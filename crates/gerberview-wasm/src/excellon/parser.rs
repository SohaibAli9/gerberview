//! Excellon drill parser.

use std::collections::HashMap;

use crate::error::GeometryError;

use super::types::{DrillHole, ExcellonResult, ExcellonUnits, ToolDefinition};

const DEFAULT_INTEGER_DIGITS: u8 = 2;
const DEFAULT_DECIMAL_DIGITS: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZeroSuppression {
    Leading,
    Trailing,
}

#[derive(Debug)]
struct ParserState {
    units: ExcellonUnits,
    integer_digits: u8,
    decimal_digits: u8,
    suppression: ZeroSuppression,
    tools: HashMap<u32, f64>,
    current_tool: Option<u32>,
    holes: Vec<DrillHole>,
    warnings: Vec<String>,
    declared_units: bool,
    in_header: bool,
}

impl Default for ParserState {
    fn default() -> Self {
        Self {
            units: ExcellonUnits::Imperial,
            integer_digits: DEFAULT_INTEGER_DIGITS,
            decimal_digits: DEFAULT_DECIMAL_DIGITS,
            suppression: ZeroSuppression::Leading,
            tools: HashMap::new(),
            current_tool: None,
            holes: Vec::new(),
            warnings: Vec::new(),
            declared_units: false,
            in_header: false,
        }
    }
}

/// Parse an Excellon drill file and return extracted holes, tools, and metadata.
///
/// # Errors
///
/// Returns [`GeometryError::ParseError`] if the input is empty, not valid UTF-8,
/// or contains invalid numeric fields in commands that must be parsed.
pub fn parse(data: &[u8]) -> Result<ExcellonResult, GeometryError> {
    if data.is_empty() {
        return Err(GeometryError::ParseError("empty input".to_string()));
    }

    let content = std::str::from_utf8(data)
        .map_err(|err| GeometryError::ParseError(format!("invalid UTF-8 input: {err}")))?;

    let mut state = ParserState::default();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        let normalized = line.to_ascii_uppercase();

        if normalized == "M48" {
            state.in_header = true;
            continue;
        }

        if normalized == "%" {
            state.in_header = false;
            continue;
        }

        if normalized == "M30" {
            break;
        }

        if state.in_header {
            parse_header_line(&normalized, &mut state)?;
        } else {
            parse_body_line(&normalized, &mut state)?;
        }
    }

    let mut tools: Vec<ToolDefinition> = state
        .tools
        .into_iter()
        .map(|(number, diameter)| ToolDefinition { number, diameter })
        .collect();
    tools.sort_by_key(|tool| tool.number);

    Ok(ExcellonResult {
        holes: state.holes,
        tools,
        units: state.units,
        warnings: state.warnings,
    })
}

fn parse_header_line(line: &str, state: &mut ParserState) -> Result<(), GeometryError> {
    if apply_units_directive(line, state) {
        return Ok(());
    }

    if let Some((tool_number, diameter)) = parse_tool_definition(line)? {
        register_tool(state, tool_number, diameter);
    }

    Ok(())
}

fn parse_body_line(line: &str, state: &mut ParserState) -> Result<(), GeometryError> {
    if apply_units_directive(line, state) {
        return Ok(());
    }

    if is_routing_command(line) {
        return Ok(());
    }

    if let Some((tool_number, diameter)) = parse_tool_definition(line)? {
        register_tool(state, tool_number, diameter);
        return Ok(());
    }

    if let Some(tool_number) = parse_tool_selection(line)? {
        if state.tools.contains_key(&tool_number) {
            state.current_tool = Some(tool_number);
        } else {
            state.current_tool = None;
            state
                .warnings
                .push(format!("tool T{tool_number} selected but not defined"));
        }
        return Ok(());
    }

    if let Some((x, y)) = parse_xy_coordinates(line, state)? {
        if let Some(tool_number) = state.current_tool {
            if let Some(diameter) = state.tools.get(&tool_number).copied() {
                state.holes.push(DrillHole { x, y, diameter });
            } else {
                state.warnings.push(format!(
                    "hole at ({x}, {y}) skipped: selected tool T{tool_number} is undefined"
                ));
            }
        } else {
            state
                .warnings
                .push(format!("hole at ({x}, {y}) skipped: no tool selected"));
        }
    }

    Ok(())
}

fn apply_units_directive(line: &str, state: &mut ParserState) -> bool {
    let (units, suffix) = if let Some(rest) = line.strip_prefix("METRIC") {
        (ExcellonUnits::Metric, rest)
    } else if let Some(rest) = line.strip_prefix("INCH") {
        (ExcellonUnits::Imperial, rest)
    } else {
        return false;
    };

    if state.declared_units && state.units != units {
        state
            .warnings
            .push("mixed unit declarations detected; last declaration wins".to_string());
    }

    state.units = units;
    state.declared_units = true;

    if suffix.contains(",TZ") {
        state.suppression = ZeroSuppression::Trailing;
    } else if suffix.contains(",LZ") {
        state.suppression = ZeroSuppression::Leading;
    }

    true
}

fn register_tool(state: &mut ParserState, tool_number: u32, diameter: f64) {
    if diameter <= 0.0 {
        state.warnings.push(format!(
            "tool T{tool_number} has zero or negative diameter and was skipped"
        ));
        return;
    }

    if state.tools.contains_key(&tool_number) {
        state.warnings.push(format!(
            "duplicate tool definition for T{tool_number}; last definition wins"
        ));
    }

    state.tools.insert(tool_number, diameter);
}

fn parse_tool_definition(line: &str) -> Result<Option<(u32, f64)>, GeometryError> {
    let Some(after_t) = line.strip_prefix('T') else {
        return Ok(None);
    };

    let Some((tool_raw, diameter_raw)) = after_t.split_once('C') else {
        return Ok(None);
    };

    if tool_raw.is_empty() || diameter_raw.is_empty() {
        return Err(GeometryError::ParseError(format!(
            "invalid tool definition `{line}`"
        )));
    }

    if !tool_raw.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(GeometryError::ParseError(format!(
            "invalid tool number in `{line}`"
        )));
    }

    let tool_number = parse_u32(tool_raw, "tool number")?;
    let diameter = parse_f64(diameter_raw, "tool diameter")?;

    Ok(Some((tool_number, diameter)))
}

fn parse_tool_selection(line: &str) -> Result<Option<u32>, GeometryError> {
    if !line.starts_with('T') || line.contains('C') {
        return Ok(None);
    }

    let Some(tool_raw) = line.strip_prefix('T') else {
        return Ok(None);
    };

    if tool_raw.is_empty() {
        return Ok(None);
    }

    if !tool_raw.chars().all(|ch| ch.is_ascii_digit()) {
        return Ok(None);
    }

    parse_u32(tool_raw, "selected tool number").map(Some)
}

fn parse_xy_coordinates(
    line: &str,
    state: &ParserState,
) -> Result<Option<(f64, f64)>, GeometryError> {
    let Some(after_x) = line.strip_prefix('X') else {
        return Ok(None);
    };

    let Some((x_raw, y_raw)) = after_x.split_once('Y') else {
        return Ok(None);
    };

    if x_raw.is_empty() || y_raw.is_empty() {
        return Err(GeometryError::ParseError(format!(
            "invalid coordinate command `{line}`"
        )));
    }

    let x = parse_coordinate(
        x_raw,
        state.integer_digits,
        state.decimal_digits,
        state.suppression,
    )?;
    let y = parse_coordinate(
        y_raw,
        state.integer_digits,
        state.decimal_digits,
        state.suppression,
    )?;

    Ok(Some((x, y)))
}

fn parse_coordinate(
    raw: &str,
    integer_digits: u8,
    decimal_digits: u8,
    suppression: ZeroSuppression,
) -> Result<f64, GeometryError> {
    if raw.contains('.') {
        return parse_f64(raw, "coordinate");
    }

    let (sign, digits) = split_sign(raw);
    if digits.is_empty() {
        return Err(GeometryError::ParseError(
            "empty coordinate value".to_string(),
        ));
    }

    if !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(GeometryError::ParseError(format!(
            "invalid coordinate value `{raw}`"
        )));
    }

    // Many modern drill files use integer coordinates (e.g., X5Y10) in TZ mode.
    // Preserve those values directly when they fit within the integer precision.
    if digits.len() <= usize::from(integer_digits) {
        let value = parse_f64(digits, "coordinate")?;
        return Ok(sign * value);
    }

    let normalized =
        normalize_implicit_decimal(digits, integer_digits, decimal_digits, suppression);
    let value = parse_f64(&normalized, "coordinate")?;
    Ok(sign * value)
}

fn normalize_implicit_decimal(
    digits: &str,
    integer_digits: u8,
    decimal_digits: u8,
    suppression: ZeroSuppression,
) -> String {
    let decimal_count = usize::from(decimal_digits);
    if decimal_count == 0 {
        return digits.to_string();
    }

    let total_digits = usize::from(integer_digits) + decimal_count;
    let expanded = if digits.len() < total_digits {
        match suppression {
            ZeroSuppression::Leading => format!("{digits:0>total_digits$}"),
            ZeroSuppression::Trailing => format!("{digits:0<total_digits$}"),
        }
    } else {
        digits.to_string()
    };

    let split_index = expanded.len().saturating_sub(decimal_count);
    let (int_part, frac_part) = expanded.split_at(split_index);
    format!("{int_part}.{frac_part}")
}

fn split_sign(raw: &str) -> (f64, &str) {
    match (raw.strip_prefix('-'), raw.strip_prefix('+')) {
        (Some(rest), _) => (-1.0, rest),
        (None, Some(rest)) => (1.0, rest),
        (None, None) => (1.0, raw),
    }
}

fn parse_u32(raw: &str, label: &str) -> Result<u32, GeometryError> {
    raw.parse::<u32>()
        .map_err(|err| GeometryError::ParseError(format!("invalid {label} `{raw}`: {err}")))
}

fn parse_f64(raw: &str, label: &str) -> Result<f64, GeometryError> {
    raw.parse::<f64>()
        .map_err(|err| GeometryError::ParseError(format!("invalid {label} `{raw}`: {err}")))
}

fn is_routing_command(line: &str) -> bool {
    line.starts_with("G00")
        || line.starts_with("G01")
        || line.starts_with("G02")
        || line.starts_with("G03")
        || line.starts_with("G85")
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-9;

    #[test]
    fn ut_exc_001_parse_minimal_fixture_extracts_tools_and_holes() {
        let result = parse(include_bytes!("../../tests/fixtures/minimal/drill.drl"));
        assert!(result.is_ok(), "expected parser to accept minimal fixture");

        if let Ok(parsed) = result {
            assert_eq!(parsed.tools.len(), 2);
            assert_eq!(parsed.holes.len(), 5);

            let first = parsed.holes.first();
            assert!(first.is_some(), "first hole missing");
            if let Some(first) = first {
                assert!((first.x - 2.54).abs() < EPSILON);
                assert!((first.y - 2.54).abs() < EPSILON);
                assert!((first.diameter - 0.8).abs() < EPSILON);
            }
        }
    }

    #[test]
    fn ut_exc_002_metric_units_detected_from_minimal_fixture() {
        let result = parse(include_bytes!("../../tests/fixtures/minimal/drill.drl"));
        assert!(result.is_ok(), "expected parser to accept minimal fixture");

        if let Ok(parsed) = result {
            assert_eq!(parsed.units, ExcellonUnits::Metric);
        }
    }

    #[test]
    fn ut_exc_003_imperial_units_detected_from_arduino_fixture() {
        let result = parse(include_bytes!(
            "../../tests/fixtures/arduino-uno/arduino-uno.drl"
        ));
        assert!(result.is_ok(), "expected parser to accept arduino fixture");

        if let Ok(parsed) = result {
            assert_eq!(parsed.units, ExcellonUnits::Imperial);
        }
    }

    #[test]
    fn ut_exc_004_lz_coordinate_parsing() {
        let input = b"M48\nMETRIC,LZ\nT1C1.0\n%\nT1\nX1500Y2500\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "expected parser to accept LZ input");

        if let Ok(parsed) = result {
            assert_eq!(parsed.holes.len(), 1);
            let hole = parsed.holes.first();
            assert!(hole.is_some(), "hole missing");
            if let Some(hole) = hole {
                assert!((hole.x - 0.15).abs() < EPSILON);
                assert!((hole.y - 0.25).abs() < EPSILON);
            }
        }
    }

    #[test]
    fn ut_exc_005_tz_coordinate_parsing() {
        let input = b"M48\nMETRIC,TZ\nT1C1.0\n%\nT1\nX1500Y2500\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "expected parser to accept TZ input");

        if let Ok(parsed) = result {
            assert_eq!(parsed.holes.len(), 1);
            let hole = parsed.holes.first();
            assert!(hole.is_some(), "hole missing");
            if let Some(hole) = hole {
                assert!((hole.x - 15.0).abs() < EPSILON);
                assert!((hole.y - 25.0).abs() < EPSILON);
            }
        }
    }

    #[test]
    fn ut_exc_006_tool_reselection_uses_latest_selected_tool() {
        let result = parse(include_bytes!(
            "../../tests/fixtures/arduino-uno/arduino-uno.drl"
        ));
        assert!(result.is_ok(), "expected parser to accept arduino fixture");

        if let Ok(parsed) = result {
            assert_eq!(parsed.holes.len(), 15);
            let last = parsed.holes.last();
            assert!(last.is_some(), "last hole missing");
            if let Some(last) = last {
                assert!((last.diameter - 0.0394).abs() < EPSILON);
            }
        }
    }

    #[test]
    fn bc_exc_001_empty_input_returns_error() {
        let result = parse(&[]);
        assert!(result.is_err(), "empty input must return an error");
    }

    #[test]
    fn bc_exc_002_header_only_input_returns_zero_holes() {
        let input = b"M48\nT1C0.8\n%\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "header-only file should parse");

        if let Ok(parsed) = result {
            assert_eq!(parsed.tools.len(), 1);
            assert_eq!(parsed.holes.len(), 0);
        }
    }

    #[test]
    fn bc_exc_003_no_m48_header_uses_defaults() {
        let input = b"T1C0.8\nT1\nX10000Y20000\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "no-header file should parse using defaults");

        if let Ok(parsed) = result {
            assert_eq!(parsed.units, ExcellonUnits::Imperial);
            assert_eq!(parsed.holes.len(), 1);
            let hole = parsed.holes.first();
            assert!(hole.is_some(), "hole missing");
            if let Some(hole) = hole {
                assert!((hole.x - 1.0).abs() < EPSILON);
                assert!((hole.y - 2.0).abs() < EPSILON);
            }
        }
    }

    #[test]
    fn bc_exc_004_zero_diameter_tool_is_skipped_with_warning() {
        let input = b"M48\nMETRIC\nT1C0.0\nT2C0.8\n%\nT1\nX1.0Y1.0\nT2\nX2.0Y2.0\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "input should parse");

        if let Ok(parsed) = result {
            assert_eq!(parsed.tools.len(), 1);
            assert_eq!(parsed.holes.len(), 1);
            assert!(parsed
                .warnings
                .iter()
                .any(|warning| { warning.contains("zero or negative diameter") }));
        }
    }

    #[test]
    fn bc_exc_005_duplicate_tool_definition_last_wins_with_warning() {
        let input = b"M48\nMETRIC\nT1C0.8\nT1C1.0\n%\nT1\nX1.0Y1.0\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "input should parse");

        if let Ok(parsed) = result {
            assert_eq!(parsed.tools.len(), 1);
            assert_eq!(parsed.holes.len(), 1);

            let hole = parsed.holes.first();
            assert!(hole.is_some(), "hole missing");
            if let Some(hole) = hole {
                assert!((hole.diameter - 1.0).abs() < EPSILON);
            }

            assert!(parsed
                .warnings
                .iter()
                .any(|warning| { warning.contains("duplicate tool definition") }));
        }
    }

    #[test]
    fn bc_exc_006_hole_before_tool_selection_is_skipped_with_warning() {
        let input = b"M48\nMETRIC\nT1C0.8\n%\nX1.0Y1.0\nT1\nX2.0Y2.0\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "input should parse");

        if let Ok(parsed) = result {
            assert_eq!(parsed.holes.len(), 1);
            assert!(parsed
                .warnings
                .iter()
                .any(|warning| warning.contains("no tool selected")));
        }
    }

    #[test]
    fn bc_exc_007_mixed_units_last_declaration_wins_with_warning() {
        let input = b"M48\nMETRIC\nINCH\nT1C0.8\n%\nT1\nX1.0Y1.0\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "input should parse");

        if let Ok(parsed) = result {
            assert_eq!(parsed.units, ExcellonUnits::Imperial);
            assert!(parsed
                .warnings
                .iter()
                .any(|warning| warning.contains("mixed unit declarations")));
        }
    }

    #[test]
    fn bc_exc_008_routing_commands_are_ignored() {
        let input = b"M48\nMETRIC\nT1C0.8\n%\nT1\nG01X100Y200\nX1.0Y2.0\nG02X200Y300\nM30\n";
        let result = parse(input);
        assert!(result.is_ok(), "input should parse");

        if let Ok(parsed) = result {
            assert_eq!(parsed.holes.len(), 1);
            let hole = parsed.holes.first();
            assert!(hole.is_some(), "hole missing");
            if let Some(hole) = hole {
                assert!((hole.x - 1.0).abs() < EPSILON);
                assert!((hole.y - 2.0).abs() < EPSILON);
            }
        }
    }
}
