//! Aperture macro evaluation.
//!
//! Evaluates aperture macro primitives (`Circle`, `VectorLine`, `CenterLine`,
//! `Outline`, `Polygon`) with exposure flags and arithmetic expression evaluation.

use std::collections::HashMap;

use gerber_types::{
    ApertureMacro, CenterLinePrimitive, CirclePrimitive, MacroBoolean, MacroContent, MacroDecimal,
    MacroInteger, OutlinePrimitive, PolygonPrimitive, VectorLinePrimitive,
};

use crate::error::GeometryError;

use super::types::{GeometryBuilder, Point};

const CIRCLE_SEGMENTS: u32 = 32;
const BC_GBR_024: &str = "BC-GBR-024: division by zero in macro expression; evaluating to 0";
const BC_GBR_025_DEEP: &str = "BC-GBR-025: expression nesting >20 levels";
const BC_GBR_025_WARN: &str = "BC-GBR-025: expression nesting >10 levels";

const MAX_NEST_WARN: u32 = 10;
const MAX_NEST_ERROR: u32 = 20;

/// Resolves aperture macro parameters from `MacroDecimal` to `f64`.
///
/// Parameters are resolved in order; each resolved value populates the
/// variable context ($1, $2, ...) for resolving subsequent parameters
/// that may reference them.
///
/// # Errors
///
/// Returns an error when a parameter cannot be resolved (e.g. undefined
/// variable reference).
pub fn resolve_macro_params(
    builder: &mut GeometryBuilder,
    params: Option<&[MacroDecimal]>,
) -> Result<Vec<f64>, GeometryError> {
    let Some(params) = params else {
        return Ok(Vec::new());
    };
    let mut vars: HashMap<u32, f64> = HashMap::new();
    let mut resolved = Vec::with_capacity(params.len());
    for (i, p) in params.iter().enumerate() {
        let v = resolve_decimal(builder, p, &vars)?;
        let key = u32::try_from(i).map_or(0, |n| n + 1);
        if key > 0 {
            vars.insert(key, v);
        }
        resolved.push(v);
    }
    Ok(resolved)
}

/// Evaluates an aperture macro at the given position.
///
/// Builds variable context from `params` ($1 = params[0], etc.), processes
/// variable definitions and primitives, and adds geometry to the builder.
///
/// # Errors
///
/// Returns an error for unsupported primitives (Moire, Thermal) or invalid
/// macro content.
pub fn evaluate_macro(
    builder: &mut GeometryBuilder,
    macro_def: &ApertureMacro,
    params: &[f64],
    position: Point,
) -> Result<(), GeometryError> {
    let mut vars: HashMap<u32, f64> = HashMap::new();
    for (i, &v) in params.iter().enumerate() {
        let key = u32::try_from(i).map_or(0, |n| n + 1);
        if key > 0 {
            vars.insert(key, v);
        }
    }

    for content in &macro_def.content {
        match content {
            MacroContent::VariableDefinition(vd) => {
                let val = evaluate_expression(builder, &vd.expression, &vars)?;
                vars.insert(vd.number, val);
            }
            MacroContent::Circle(c) => eval_circle(builder, c, &vars, position)?,
            MacroContent::VectorLine(vl) => eval_vector_line(builder, vl, &vars, position)?,
            MacroContent::CenterLine(cl) => eval_center_line(builder, cl, &vars, position)?,
            MacroContent::Outline(o) => eval_outline(builder, o, &vars, position)?,
            MacroContent::Polygon(p) => eval_polygon(builder, p, &vars, position)?,
            MacroContent::Moire(_) | MacroContent::Thermal(_) => {
                return Err(GeometryError::UnsupportedFeature(
                    "moire and thermal primitives not supported".to_string(),
                ));
            }
            MacroContent::Comment(_) => {}
        }
    }

    Ok(())
}

fn resolve_decimal(
    builder: &mut GeometryBuilder,
    d: &MacroDecimal,
    vars: &HashMap<u32, f64>,
) -> Result<f64, GeometryError> {
    match d {
        MacroDecimal::Value(v) => Ok(*v),
        MacroDecimal::Variable(n) => vars
            .get(n)
            .copied()
            .ok_or_else(|| GeometryError::MacroError(format!("undefined variable ${n}"))),
        MacroDecimal::Expression(s) => evaluate_expression(builder, s, vars),
    }
}

fn resolve_boolean(
    builder: &mut GeometryBuilder,
    b: &MacroBoolean,
    vars: &HashMap<u32, f64>,
) -> Result<bool, GeometryError> {
    match b {
        MacroBoolean::Value(v) => Ok(*v),
        MacroBoolean::Variable(n) => {
            let v = vars
                .get(n)
                .copied()
                .ok_or_else(|| GeometryError::MacroError(format!("undefined variable ${n}")))?;
            Ok(v != 0.0)
        }
        MacroBoolean::Expression(s) => {
            let v = evaluate_expression(builder, s, vars)?;
            Ok(v != 0.0)
        }
    }
}

fn resolve_integer(
    builder: &mut GeometryBuilder,
    i: &MacroInteger,
    vars: &HashMap<u32, f64>,
) -> Result<u32, GeometryError> {
    match i {
        MacroInteger::Value(v) => Ok(*v),
        MacroInteger::Variable(n) => {
            let v = vars
                .get(n)
                .copied()
                .ok_or_else(|| GeometryError::MacroError(format!("undefined variable ${n}")))?;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Ok(v as u32)
        }
        MacroInteger::Expression(s) => {
            let v = evaluate_expression(builder, s, vars)?;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Ok(v as u32)
        }
    }
}

fn evaluate_expression(
    builder: &mut GeometryBuilder,
    expr: &str,
    vars: &HashMap<u32, f64>,
) -> Result<f64, GeometryError> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Ok(0.0);
    }
    let tokens = tokenize(expr)?;
    eval_with_nesting(builder, &tokens, vars, 0)
}

fn eval_with_nesting(
    builder: &mut GeometryBuilder,
    tokens: &[Token],
    vars: &HashMap<u32, f64>,
    depth: u32,
) -> Result<f64, GeometryError> {
    let (val, rest) = parse_additive(builder, tokens, vars, depth)?;
    if rest.is_empty() {
        Ok(val)
    } else {
        Err(GeometryError::MacroError(
            "unexpected tokens in expression".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    Variable(u32),
    Op(char),
    LParen,
    RParen,
}

fn tokenize(expr: &str) -> Result<Vec<Token>, GeometryError> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => {}
            '+' | '-' | '/' => tokens.push(Token::Op(c)),
            'x' | 'X' => tokens.push(Token::Op('x')),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '$' => {
                let mut num = String::new();
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    if let Some(d) = chars.next() {
                        num.push(d);
                    }
                }
                let n: u32 = num.parse().map_err(|_| {
                    GeometryError::MacroError("invalid variable after $".to_string())
                })?;
                tokens.push(Token::Variable(n));
            }
            '0'..='9' | '.' => {
                let mut num = String::from(c);
                while let Some(&p) = chars.peek() {
                    match p {
                        '0'..='9' | '.' | 'e' | 'E' => {
                            chars.next();
                            num.push(p);
                        }
                        '+' | '-' => {
                            if num.ends_with('e') || num.ends_with('E') {
                                chars.next();
                                num.push(p);
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
                let v: f64 = num
                    .parse()
                    .map_err(|_| GeometryError::MacroError(format!("invalid number: {num}")))?;
                tokens.push(Token::Number(v));
            }
            _ => {
                return Err(GeometryError::MacroError(format!(
                    "unexpected character in expression: {c}"
                )));
            }
        }
    }

    Ok(tokens)
}

fn parse_additive<'a>(
    builder: &mut GeometryBuilder,
    tokens: &'a [Token],
    vars: &HashMap<u32, f64>,
    depth: u32,
) -> Result<(f64, &'a [Token]), GeometryError> {
    let (mut left, mut rest) = parse_multiplicative(builder, tokens, vars, depth)?;

    while let Some((op, tail)) = match rest.first() {
        Some(Token::Op('+')) => rest.get(1..).map(|t| ('+', t)),
        Some(Token::Op('-')) => rest.get(1..).map(|t| ('-', t)),
        _ => None,
    } {
        let (right, new_rest) = parse_multiplicative(builder, tail, vars, depth)?;
        left = match op {
            '+' => left + right,
            '-' => left - right,
            _ => left,
        };
        rest = new_rest;
    }

    Ok((left, rest))
}

fn parse_multiplicative<'a>(
    builder: &mut GeometryBuilder,
    tokens: &'a [Token],
    vars: &HashMap<u32, f64>,
    depth: u32,
) -> Result<(f64, &'a [Token]), GeometryError> {
    let (mut left, mut rest) = parse_unary(builder, tokens, vars, depth)?;

    while let Some((op, tail)) = match rest.first() {
        Some(Token::Op('x')) => rest.get(1..).map(|t| ('*', t)),
        Some(Token::Op('/')) => rest.get(1..).map(|t| ('/', t)),
        _ => None,
    } {
        let (right, new_rest) = parse_unary(builder, tail, vars, depth)?;
        left = match op {
            '*' => left * right,
            '/' => {
                if right.abs() < f64::EPSILON {
                    builder.warn(BC_GBR_024.to_string());
                    0.0
                } else {
                    left / right
                }
            }
            _ => left,
        };
        rest = new_rest;
    }

    Ok((left, rest))
}

fn parse_unary<'a>(
    builder: &mut GeometryBuilder,
    tokens: &'a [Token],
    vars: &HashMap<u32, f64>,
    depth: u32,
) -> Result<(f64, &'a [Token]), GeometryError> {
    let tail = tokens.get(1..).map_or(&[] as &[Token], |s| s);
    match tokens.first() {
        Some(Token::Op('+')) => parse_unary(builder, tail, vars, depth),
        Some(Token::Op('-')) => {
            let (v, rest) = parse_unary(builder, tail, vars, depth)?;
            Ok((-v, rest))
        }
        Some(Token::LParen) => {
            let new_depth = depth + 1;
            if new_depth > MAX_NEST_ERROR {
                builder.warn(BC_GBR_025_DEEP.to_string());
            } else if new_depth > MAX_NEST_WARN {
                builder.warn(BC_GBR_025_WARN.to_string());
            }
            let (v, rest) = parse_additive(builder, tail, vars, new_depth)?;
            match rest.first() {
                Some(Token::RParen) => Ok((v, rest.get(1..).map_or(&[] as &[Token], |s| s))),
                _ => Err(GeometryError::MacroError("missing ')'".to_string())),
            }
        }
        Some(Token::Number(n)) => Ok((*n, tail)),
        Some(Token::Variable(n)) => {
            let v = vars
                .get(n)
                .copied()
                .ok_or_else(|| GeometryError::MacroError(format!("undefined variable ${n}")))?;
            Ok((v, tail))
        }
        _ => Err(GeometryError::MacroError(
            "expected number, variable, or '('".to_string(),
        )),
    }
}

fn eval_circle(
    builder: &mut GeometryBuilder,
    c: &CirclePrimitive,
    vars: &HashMap<u32, f64>,
    position: Point,
) -> Result<(), GeometryError> {
    let exposure = resolve_boolean(builder, &c.exposure, vars)?;
    let diameter = resolve_decimal(builder, &c.diameter, vars)?;
    let (cx, cy) = (
        resolve_decimal(builder, &c.center.0, vars)?,
        resolve_decimal(builder, &c.center.1, vars)?,
    );
    let angle = c
        .angle
        .as_ref()
        .map(|a| resolve_decimal(builder, a, vars))
        .transpose()?
        .unwrap_or(0.0);

    let radius = diameter / 2.0;
    if radius <= 0.0 {
        return Ok(());
    }

    let idx_start = builder.index_count();
    let (rx, ry) = rotate_point(cx, cy, angle);
    let px = position.x + rx;
    let py = position.y + ry;
    builder.push_ngon(px, py, radius, CIRCLE_SEGMENTS);
    let idx_end = builder.index_count();

    if !exposure {
        builder.record_clear_range(idx_start, idx_end);
    }

    Ok(())
}

fn eval_vector_line(
    builder: &mut GeometryBuilder,
    vl: &VectorLinePrimitive,
    vars: &HashMap<u32, f64>,
    position: Point,
) -> Result<(), GeometryError> {
    let exposure = resolve_boolean(builder, &vl.exposure, vars)?;
    let width = resolve_decimal(builder, &vl.width, vars)?;
    let (sx, sy) = (
        resolve_decimal(builder, &vl.start.0, vars)?,
        resolve_decimal(builder, &vl.start.1, vars)?,
    );
    let (ex, ey) = (
        resolve_decimal(builder, &vl.end.0, vars)?,
        resolve_decimal(builder, &vl.end.1, vars)?,
    );
    let angle = resolve_decimal(builder, &vl.angle, vars)?;

    if width <= 0.0 {
        return Ok(());
    }

    let idx_start = builder.index_count();
    let (rsx, rsy) = rotate_point(sx, sy, angle);
    let (rex, rey) = rotate_point(ex, ey, angle);
    let x1 = position.x + rsx;
    let y1 = position.y + rsy;
    let x2 = position.x + rex;
    let y2 = position.y + rey;

    push_line_rect(builder, x1, y1, x2, y2, width);
    let idx_end = builder.index_count();

    if !exposure {
        builder.record_clear_range(idx_start, idx_end);
    }

    Ok(())
}

fn eval_center_line(
    builder: &mut GeometryBuilder,
    cl: &CenterLinePrimitive,
    vars: &HashMap<u32, f64>,
    position: Point,
) -> Result<(), GeometryError> {
    let exposure = resolve_boolean(builder, &cl.exposure, vars)?;
    let (w, h) = (
        resolve_decimal(builder, &cl.dimensions.0, vars)?,
        resolve_decimal(builder, &cl.dimensions.1, vars)?,
    );
    let (cx, cy) = (
        resolve_decimal(builder, &cl.center.0, vars)?,
        resolve_decimal(builder, &cl.center.1, vars)?,
    );
    let angle = resolve_decimal(builder, &cl.angle, vars)?;

    if w <= 0.0 || h <= 0.0 {
        return Ok(());
    }

    let idx_start = builder.index_count();
    let (rcx, rcy) = rotate_point(cx, cy, angle);
    let px = position.x + rcx;
    let py = position.y + rcy;
    push_centered_rect(builder, px, py, w, h, angle);
    let idx_end = builder.index_count();

    if !exposure {
        builder.record_clear_range(idx_start, idx_end);
    }

    Ok(())
}

fn eval_outline(
    builder: &mut GeometryBuilder,
    o: &OutlinePrimitive,
    vars: &HashMap<u32, f64>,
    position: Point,
) -> Result<(), GeometryError> {
    let exposure = resolve_boolean(builder, &o.exposure, vars)?;
    let angle = resolve_decimal(builder, &o.angle, vars)?;

    if o.points.len() < 3 {
        return Ok(());
    }

    let mut flat = Vec::with_capacity(o.points.len() * 2);
    for pt in &o.points {
        let x = resolve_decimal(builder, &pt.0, vars)?;
        let y = resolve_decimal(builder, &pt.1, vars)?;
        let (rx, ry) = rotate_point(x, y, angle);
        flat.push(position.x + rx);
        flat.push(position.y + ry);
    }

    let tri_indices = earclip::earcut::earcut(&flat, &[], 2);
    if tri_indices.is_empty() {
        return Ok(());
    }

    let idx_start = builder.index_count();
    let base = outline_emit_vertices(builder, &flat);
    outline_emit_triangles(builder, &tri_indices, base)?;
    let idx_end = builder.index_count();

    if !exposure {
        builder.record_clear_range(idx_start, idx_end);
    }

    Ok(())
}

fn outline_emit_vertices(builder: &mut GeometryBuilder, flat: &[f64]) -> u32 {
    let mut first: Option<u32> = None;
    for pair in flat.chunks_exact(2) {
        if let [x, y] = *pair {
            let idx = builder.push_vertex(x, y);
            if first.is_none() {
                first = Some(idx);
            }
        }
    }
    first.unwrap_or(0)
}

fn outline_emit_triangles(
    builder: &mut GeometryBuilder,
    indices: &[usize],
    base: u32,
) -> Result<(), GeometryError> {
    for tri in indices.chunks_exact(3) {
        if let [ia, ib, ic] = *tri {
            let a = outline_offset(base, ia)?;
            let b = outline_offset(base, ib)?;
            let c = outline_offset(base, ic)?;
            builder.push_triangle(a, b, c);
        }
    }
    Ok(())
}

fn outline_offset(base: u32, offset: usize) -> Result<u32, GeometryError> {
    let offset_u32 = u32::try_from(offset)
        .map_err(|_| GeometryError::MacroError("outline index overflow".into()))?;
    base.checked_add(offset_u32)
        .ok_or_else(|| GeometryError::MacroError("outline vertex index overflow".into()))
}

fn eval_polygon(
    builder: &mut GeometryBuilder,
    p: &PolygonPrimitive,
    vars: &HashMap<u32, f64>,
    position: Point,
) -> Result<(), GeometryError> {
    let exposure = resolve_boolean(builder, &p.exposure, vars)?;
    let vertices = resolve_integer(builder, &p.vertices, vars)?;
    let (cx, cy) = (
        resolve_decimal(builder, &p.center.0, vars)?,
        resolve_decimal(builder, &p.center.1, vars)?,
    );
    let diameter = resolve_decimal(builder, &p.diameter, vars)?;
    let angle = resolve_decimal(builder, &p.angle, vars)?;

    if vertices < 3 || diameter <= 0.0 {
        return Ok(());
    }

    let idx_start = builder.index_count();
    let (rcx, rcy) = rotate_point(cx, cy, angle);
    let px = position.x + rcx;
    let py = position.y + rcy;
    let radius = diameter / 2.0;
    builder.push_ngon(px, py, radius, vertices);
    let idx_end = builder.index_count();

    if !exposure {
        builder.record_clear_range(idx_start, idx_end);
    }

    Ok(())
}

fn rotate_point(x: f64, y: f64, angle_deg: f64) -> (f64, f64) {
    let rad = angle_deg.to_radians();
    let c = rad.cos();
    let s = rad.sin();
    (x.mul_add(c, -(y * s)), x.mul_add(s, y * c))
}

fn push_line_rect(builder: &mut GeometryBuilder, x1: f64, y1: f64, x2: f64, y2: f64, width: f64) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = dx.hypot(dy);
    if len < f64::EPSILON {
        return;
    }
    let nx = -dy / len;
    let ny = dx / len;
    let hw = width / 2.0;

    let a = builder.push_vertex(nx.mul_add(hw, x1), ny.mul_add(hw, y1));
    let b = builder.push_vertex(nx.mul_add(-hw, x1), ny.mul_add(-hw, y1));
    let c = builder.push_vertex(nx.mul_add(-hw, x2), ny.mul_add(-hw, y2));
    let d = builder.push_vertex(nx.mul_add(hw, x2), ny.mul_add(hw, y2));
    builder.push_quad(a, b, c, d);
}

#[allow(clippy::indexing_slicing)]
fn push_centered_rect(
    builder: &mut GeometryBuilder,
    center_x: f64,
    center_y: f64,
    width: f64,
    height: f64,
    angle_deg: f64,
) {
    let half_w = width / 2.0;
    let half_h = height / 2.0;
    let corners = [
        (-half_w, -half_h),
        (half_w, -half_h),
        (half_w, half_h),
        (-half_w, half_h),
    ];
    let rotated: [(f64, f64); 4] = corners.map(|(dx, dy)| rotate_point(dx, dy, angle_deg));
    let v0 = builder.push_vertex(center_x + rotated[0].0, center_y + rotated[0].1);
    let v1 = builder.push_vertex(center_x + rotated[1].0, center_y + rotated[1].1);
    let v2 = builder.push_vertex(center_x + rotated[2].0, center_y + rotated[2].1);
    let v3 = builder.push_vertex(center_x + rotated[3].0, center_y + rotated[3].1);
    builder.push_quad(v0, v1, v2, v3);
}

#[cfg(test)]
mod tests {
    use gerber_types::{CirclePrimitive, MacroBoolean, MacroDecimal, VariableDefinition};

    use super::*;

    fn make_macro_with_circle() -> ApertureMacro {
        ApertureMacro::new("CIRCLE").add_content(CirclePrimitive {
            exposure: MacroBoolean::Value(true),
            diameter: MacroDecimal::Value(2.0),
            center: (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
            angle: None,
        })
    }

    #[test]
    fn ut_mac_001_circle_primitive_produces_vertices() {
        let macro_def = make_macro_with_circle();
        let mut builder = GeometryBuilder::new();
        let result = evaluate_macro(&mut builder, &macro_def, &[], Point { x: 0.0, y: 0.0 });
        assert!(result.is_ok());
        let geom = builder.build();
        assert_eq!(geom.vertex_count, CIRCLE_SEGMENTS);
        assert!(!geom.indices.is_empty());
    }

    #[test]
    fn ut_mac_002_vector_line_primitive_produces_vertices() {
        let macro_def = ApertureMacro::new("LINE").add_content(VectorLinePrimitive {
            exposure: MacroBoolean::Value(true),
            width: MacroDecimal::Value(0.5),
            start: (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
            end: (MacroDecimal::Value(2.0), MacroDecimal::Value(0.0)),
            angle: MacroDecimal::Value(0.0),
        });
        let mut builder = GeometryBuilder::new();
        let result = evaluate_macro(&mut builder, &macro_def, &[], Point { x: 0.0, y: 0.0 });
        assert!(result.is_ok());
        let geom = builder.build();
        assert_eq!(geom.vertex_count, 4);
        assert_eq!(geom.indices.len(), 6);
    }

    #[test]
    fn ut_mac_003_outline_primitive_produces_polygon_geometry() {
        let points = vec![
            (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
            (MacroDecimal::Value(1.0), MacroDecimal::Value(0.0)),
            (MacroDecimal::Value(1.0), MacroDecimal::Value(1.0)),
            (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
        ];
        let macro_def = ApertureMacro::new("OUTLINE").add_content(OutlinePrimitive {
            exposure: MacroBoolean::Value(true),
            points,
            angle: MacroDecimal::Value(0.0),
        });
        let mut builder = GeometryBuilder::new();
        let result = evaluate_macro(&mut builder, &macro_def, &[], Point { x: 0.0, y: 0.0 });
        assert!(result.is_ok());
        let geom = builder.build();
        assert!(geom.vertex_count >= 3);
        assert!(geom.indices.len() >= 3);
    }

    #[test]
    fn ut_mac_003b_concave_outline_triangulates_correctly() {
        let points = vec![
            (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
            (MacroDecimal::Value(2.0), MacroDecimal::Value(0.0)),
            (MacroDecimal::Value(2.0), MacroDecimal::Value(2.0)),
            (MacroDecimal::Value(1.0), MacroDecimal::Value(1.0)),
            (MacroDecimal::Value(0.0), MacroDecimal::Value(2.0)),
            (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
        ];
        let macro_def = ApertureMacro::new("CONCAVE").add_content(OutlinePrimitive {
            exposure: MacroBoolean::Value(true),
            points,
            angle: MacroDecimal::Value(0.0),
        });
        let mut builder = GeometryBuilder::new();
        let result = evaluate_macro(&mut builder, &macro_def, &[], Point { x: 0.0, y: 0.0 });
        assert!(result.is_ok());
        let geom = builder.build();
        assert!(geom.vertex_count >= 5);
        assert!(
            geom.indices.len() >= 9,
            "concave pentagon needs at least 3 triangles"
        );
    }

    #[test]
    fn ut_mac_004_exposure_off_produces_clear_geometry() {
        let macro_def = ApertureMacro::new("CLEAR_CIRCLE").add_content(CirclePrimitive {
            exposure: MacroBoolean::Value(false),
            diameter: MacroDecimal::Value(1.0),
            center: (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
            angle: None,
        });
        let mut builder = GeometryBuilder::new();
        let result = evaluate_macro(&mut builder, &macro_def, &[], Point { x: 0.0, y: 0.0 });
        assert!(result.is_ok());
        let geom = builder.build();
        assert!(!geom.clear_ranges.is_empty());
    }

    #[test]
    fn ut_mac_005_arithmetic_expression_evaluation() {
        let macro_def = ApertureMacro::new("EXPR")
            .add_content(VariableDefinition::new(3, "$1x2+$2"))
            .add_content(CirclePrimitive {
                exposure: MacroBoolean::Value(true),
                diameter: MacroDecimal::Variable(3),
                center: (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
                angle: None,
            });
        let mut builder = GeometryBuilder::new();
        let result = evaluate_macro(
            &mut builder,
            &macro_def,
            &[3.0, 1.0],
            Point { x: 0.0, y: 0.0 },
        );
        assert!(result.is_ok());
        let geom = builder.build();
        assert!(geom.vertex_count > 0);
        let radius = 3.0f64.mul_add(2.0, 1.0);
        let expected_radius = radius / 2.0;
        let first_x = geom.positions.first().copied().unwrap_or(0.0);
        assert!((f64::from(first_x) - expected_radius).abs() < 1e-5);
    }

    #[test]
    fn bc_gbr_024_division_by_zero_evaluates_to_zero_with_warn() {
        let macro_def = ApertureMacro::new("DIVZERO")
            .add_content(VariableDefinition::new(1, "1/0"))
            .add_content(CirclePrimitive {
                exposure: MacroBoolean::Value(true),
                diameter: MacroDecimal::Variable(1),
                center: (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
                angle: None,
            });
        let mut builder = GeometryBuilder::new();
        let result = evaluate_macro(&mut builder, &macro_def, &[], Point { x: 0.0, y: 0.0 });
        assert!(result.is_ok());
        let geom = builder.build();
        assert!(geom.warnings.iter().any(|w| w.contains("BC-GBR-024")));
    }

    #[test]
    fn bc_gbr_025_deep_nesting_evaluates_with_warn() {
        let mut expr = String::from("1");
        for _ in 0..12 {
            expr = format!("({expr})");
        }
        let macro_def = ApertureMacro::new("DEEP")
            .add_content(VariableDefinition::new(1, &expr))
            .add_content(CirclePrimitive {
                exposure: MacroBoolean::Value(true),
                diameter: MacroDecimal::Variable(1),
                center: (MacroDecimal::Value(0.0), MacroDecimal::Value(0.0)),
                angle: None,
            });
        let mut builder = GeometryBuilder::new();
        let result = evaluate_macro(&mut builder, &macro_def, &[], Point { x: 0.0, y: 0.0 });
        assert!(result.is_ok());
        let geom = builder.build();
        assert!(geom.warnings.iter().any(|w| w.contains("BC-GBR-025")));
    }
}
