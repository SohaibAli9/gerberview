//! Core geometry types and the geometry conversion pipeline.

pub mod aperture;
pub mod arc;
pub mod macro_eval;
pub mod polarity;
pub mod region;
pub mod step_repeat;
pub mod stroke;
pub mod types;

pub use aperture::*;
pub use arc::*;
pub use macro_eval::*;
pub use polarity::*;
pub use region::*;
pub use step_repeat::*;
pub use stroke::*;
pub use types::*;

use std::collections::HashMap;

use gerber_parser::GerberDoc;
use gerber_types::{
    Command, CoordinateFormat, CoordinateMode, CoordinateOffset, Coordinates, ExtendedCode,
    FunctionCode, Operation, Unit, ZeroOmission,
};

use crate::error::GeometryError;

const DEFAULT_FORMAT: (u8, u8) = (2, 6);
const MM_PER_INCH: f64 = 25.4;

/// Converts a parsed Gerber document into renderable layer geometry.
///
/// Walks the command list, maintains interpreter state, and dispatches to
/// geometry sub-modules for flashes, strokes, arcs, regions, step-repeat,
/// and aperture macros.
///
/// # Errors
///
/// Returns [`GeometryError`] when conversion fails fatally (e.g. invalid
/// aperture reference, degenerate geometry).
#[allow(clippy::too_many_lines)]
pub fn convert(doc: &GerberDoc) -> Result<LayerGeometry, GeometryError> {
    let format = doc.format_specification.unwrap_or_else(|| {
        CoordinateFormat::new(
            ZeroOmission::Leading,
            CoordinateMode::Absolute,
            DEFAULT_FORMAT.0,
            DEFAULT_FORMAT.1,
        )
    });
    let units = doc.units;

    let mut state = types::GerberState {
        current_point: types::Point { x: 0.0, y: 0.0 },
        current_aperture: None,
        interpolation_mode: types::InterpolationMode::Linear,
        polarity: types::Polarity::Dark,
        region_mode: false,
        region_points: Vec::new(),
        units,
        format: Some(format),
    };

    let mut builder = types::GeometryBuilder::new();
    let mut polarity_tracker = polarity::PolarityTracker::new();
    let mut arc_quadrant_mode = ArcQuadrantMode::MultiQuadrant;

    let mut macros: HashMap<String, gerber_types::ApertureMacro> = HashMap::new();
    let mut sr_stack: Vec<(types::GeometryBuilder, u32, u32, f64, f64)> = Vec::new();
    let mut command_count: u32 = 0;

    for cmd_result in &doc.commands {
        let cmd = match cmd_result {
            Ok(c) => c,
            Err(e) => {
                builder.warn(format!("parse error: {e:?}"));
                continue;
            }
        };

        command_count = command_count.saturating_add(1);

        if matches!(
            cmd,
            Command::ExtendedCode(ExtendedCode::StepAndRepeat(
                gerber_types::StepAndRepeat::Close,
            ))
        ) {
            if let Some((block_builder, repeat_x, repeat_y, distance_x, distance_y)) =
                sr_stack.pop()
            {
                let block_geom = block_builder.build();
                let parent_builder = if let Some((ref mut pb, ..)) = sr_stack.last_mut() {
                    pb
                } else {
                    &mut builder
                };
                step_repeat::apply_step_repeat(
                    parent_builder,
                    &block_geom,
                    repeat_x,
                    repeat_y,
                    distance_x,
                    distance_y,
                )?;
            } else {
                builder.warn("SR close without matching open; ignoring".to_string());
            }
            continue;
        }

        let builder_ref: &mut GeometryBuilder =
            if let Some((ref mut b, _, _, _, _)) = sr_stack.last_mut() {
                b
            } else {
                &mut builder
            };

        match cmd {
            Command::ExtendedCode(ExtendedCode::CoordinateFormat(cf)) => {
                state.format = Some(*cf);
            }
            Command::ExtendedCode(ExtendedCode::Unit(u)) => {
                state.units = Some(*u);
            }
            Command::ExtendedCode(ExtendedCode::LoadPolarity(gerber_types::Polarity::Dark)) => {
                polarity_tracker.set_polarity(types::Polarity::Dark, builder_ref);
            }
            Command::ExtendedCode(ExtendedCode::LoadPolarity(gerber_types::Polarity::Clear)) => {
                polarity_tracker.set_polarity(types::Polarity::Clear, builder_ref);
            }
            Command::ExtendedCode(ExtendedCode::StepAndRepeat(
                gerber_types::StepAndRepeat::Open {
                    repeat_x,
                    repeat_y,
                    distance_x,
                    distance_y,
                },
            )) => {
                sr_stack.push((
                    types::GeometryBuilder::new(),
                    *repeat_x,
                    *repeat_y,
                    *distance_x,
                    *distance_y,
                ));
            }
            Command::ExtendedCode(ExtendedCode::ApertureMacro(am)) => {
                macros.insert(am.name.clone(), am.clone());
            }
            Command::FunctionCode(FunctionCode::GCode(gerber_types::GCode::InterpolationMode(
                mode,
            ))) => {
                state.interpolation_mode = match mode {
                    gerber_types::InterpolationMode::Linear => types::InterpolationMode::Linear,
                    gerber_types::InterpolationMode::ClockwiseCircular => {
                        types::InterpolationMode::ClockwiseArc
                    }
                    gerber_types::InterpolationMode::CounterclockwiseCircular => {
                        types::InterpolationMode::CounterClockwiseArc
                    }
                };
            }
            Command::FunctionCode(FunctionCode::GCode(gerber_types::GCode::RegionMode(true))) => {
                state.region_mode = true;
                state.region_points.clear();
            }
            Command::FunctionCode(FunctionCode::GCode(gerber_types::GCode::RegionMode(false))) => {
                region::fill_region(builder_ref, &state.region_points)?;
                state.region_mode = false;
                state.region_points.clear();
            }
            Command::FunctionCode(FunctionCode::GCode(gerber_types::GCode::QuadrantMode(
                gerber_types::QuadrantMode::Single,
            ))) => {
                builder_ref.warn(
                    "G74 single-quadrant arc mode not supported; using multi-quadrant".to_string(),
                );
                arc_quadrant_mode = arc::ArcQuadrantMode::SingleQuadrant;
            }
            Command::FunctionCode(FunctionCode::GCode(gerber_types::GCode::QuadrantMode(
                gerber_types::QuadrantMode::Multi,
            ))) => {
                arc_quadrant_mode = arc::ArcQuadrantMode::MultiQuadrant;
            }
            Command::FunctionCode(FunctionCode::DCode(gerber_types::DCode::SelectAperture(n))) => {
                state.current_aperture = Some(*n);
            }
            Command::FunctionCode(FunctionCode::DCode(gerber_types::DCode::Operation(
                Operation::Move(Some(ref c)),
            ))) => {
                let pt = coords_to_point(c, &state);
                state.current_point = pt;
            }
            Command::FunctionCode(FunctionCode::DCode(gerber_types::DCode::Operation(
                Operation::Flash(Some(ref c)),
            ))) => {
                let pt = coords_to_point(c, &state);
                if let Some(dcode) = state.current_aperture {
                    if let Some(aperture) = doc.apertures.get(&dcode) {
                        match aperture {
                            gerber_types::Aperture::Macro(name, params) => {
                                if let Some(macro_def) = macros.get(name) {
                                    let resolved = macro_eval::resolve_macro_params(
                                        builder_ref,
                                        params.as_deref(),
                                    )?;
                                    macro_eval::evaluate_macro(
                                        builder_ref,
                                        macro_def,
                                        &resolved,
                                        pt,
                                    )?;
                                } else {
                                    builder_ref.warn(format!(
                                        "aperture macro `{name}` not defined; skipping flash"
                                    ));
                                }
                            }
                            _ => {
                                aperture::flash_aperture(builder_ref, aperture, pt)?;
                            }
                        }
                    } else {
                        builder_ref.warn(format!("aperture D{dcode} not defined; skipping flash"));
                    }
                } else {
                    builder_ref.warn("flash without selected aperture; skipping".to_string());
                }
            }
            Command::FunctionCode(FunctionCode::DCode(gerber_types::DCode::Operation(
                Operation::Interpolate(Some(ref c), ref offset),
            ))) => {
                let target = coords_to_point(c, &state);

                if state.region_mode {
                    state.region_points.push(target);
                } else if let Some(aperture) =
                    state.current_aperture.and_then(|d| doc.apertures.get(&d))
                {
                    match state.interpolation_mode {
                        types::InterpolationMode::Linear => {
                            stroke::draw_linear(
                                builder_ref,
                                state.current_point,
                                target,
                                aperture,
                            )?;
                        }
                        types::InterpolationMode::ClockwiseArc
                        | types::InterpolationMode::CounterClockwiseArc => {
                            let center_offset = offset_to_point(offset.as_ref(), &state);
                            let direction = match state.interpolation_mode {
                                types::InterpolationMode::CounterClockwiseArc => {
                                    arc::ArcDirection::CounterClockwise
                                }
                                types::InterpolationMode::ClockwiseArc
                                | types::InterpolationMode::Linear => arc::ArcDirection::Clockwise,
                            };
                            arc::draw_arc(
                                builder_ref,
                                state.current_point,
                                target,
                                center_offset,
                                direction,
                                arc_quadrant_mode,
                                aperture,
                            )?;
                        }
                    }
                } else {
                    builder_ref.warn("interpolate without selected aperture; skipping".to_string());
                }

                state.current_point = target;
            }
            _ => {}
        }
    }

    let ranges: Vec<polarity::ClearRange> = polarity_tracker.finish(&builder);
    let mut geom = builder.build();
    geom.command_count = command_count;
    apply_clear_ranges(&mut geom, ranges);

    Ok(geom)
}

fn coords_to_point(coords: &Coordinates, state: &types::GerberState) -> types::Point {
    let x = coords.x.map_or(state.current_point.x, f64::from);
    let y = coords.y.map_or(state.current_point.y, f64::from);

    let scale = unit_scale(state.units);
    types::Point {
        x: x * scale,
        y: y * scale,
    }
}

fn offset_to_point(offset: Option<&CoordinateOffset>, state: &types::GerberState) -> types::Point {
    let Some(off) = offset else {
        return types::Point { x: 0.0, y: 0.0 };
    };

    let x = off.x.map_or(0.0, f64::from);
    let y = off.y.map_or(0.0, f64::from);

    let scale = unit_scale(state.units);
    types::Point {
        x: x * scale,
        y: y * scale,
    }
}

#[allow(clippy::missing_const_for_fn)]
fn unit_scale(units: Option<Unit>) -> f64 {
    match units {
        Some(Unit::Inches) => MM_PER_INCH,
        Some(Unit::Millimeters) | None => 1.0,
    }
}
