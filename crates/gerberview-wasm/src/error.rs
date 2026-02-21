//! Error types for the geometry and parsing pipeline.

use thiserror::Error;

/// Errors that can occur during Gerber geometry conversion.
#[derive(Debug, Error)]
pub enum GeometryError {
    /// An aperture definition is missing or invalid.
    #[error("invalid aperture: {0}")]
    InvalidAperture(String),

    /// A geometry operation produced degenerate output.
    #[error("degenerate geometry: {0}")]
    DegenerateGeometry(String),

    /// A Gerber feature is not yet supported.
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// An arc interpolation failed.
    #[error("arc error: {0}")]
    ArcError(String),

    /// A region contour is invalid.
    #[error("region error: {0}")]
    RegionError(String),

    /// An aperture macro evaluation failed.
    #[error("macro error: {0}")]
    MacroError(String),

    /// A Gerber file could not be parsed.
    #[error("parse error: {0}")]
    ParseError(String),
}
