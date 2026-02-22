//! Core geometry types and the `GeometryBuilder` accumulator.

use serde::Serialize;

/// 2D point in board coordinate space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
}

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BoundingBox {
    /// Minimum X coordinate.
    pub min_x: f64,
    /// Minimum Y coordinate.
    pub min_y: f64,
    /// Maximum X coordinate.
    pub max_x: f64,
    /// Maximum Y coordinate.
    pub max_y: f64,
}

impl BoundingBox {
    /// Creates an empty bounding box that will expand with the first `update` call.
    pub const fn new() -> Self {
        Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }

    /// Expands the bounding box to include the given point.
    pub fn update(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self::new()
    }
}

/// Polarity state during geometry conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Dark polarity — adds material.
    Dark,
    /// Clear polarity — removes material.
    Clear,
}

/// Interpolation mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationMode {
    /// Linear interpolation (G01).
    Linear,
    /// Clockwise circular interpolation (G02).
    ClockwiseArc,
    /// Counter-clockwise circular interpolation (G03).
    CounterClockwiseArc,
}

/// Mutable state machine tracking the Gerber interpreter state
/// as commands are processed sequentially.
#[derive(Debug)]
pub struct GerberState {
    /// Current drawing position.
    pub current_point: Point,
    /// Currently selected aperture D-code.
    pub current_aperture: Option<i32>,
    /// Active interpolation mode.
    pub interpolation_mode: InterpolationMode,
    /// Whether region mode is active (G36/G37).
    pub region_mode: bool,
    /// Accumulated region boundary points.
    pub region_points: Vec<Point>,
    /// Unit specification from the file header.
    pub units: Option<gerber_types::Unit>,
    /// Coordinate format from the file header.
    pub format: Option<gerber_types::CoordinateFormat>,
}

impl Default for GerberState {
    fn default() -> Self {
        Self {
            current_point: Point { x: 0.0, y: 0.0 },
            current_aperture: None,
            interpolation_mode: InterpolationMode::Linear,
            region_mode: false,
            region_points: Vec::new(),
            units: None,
            format: None,
        }
    }
}

/// Output of the geometry pipeline for a single layer.
///
/// Positions are interleaved `[x0, y0, x1, y1, ...]` as `f32` for WebGL.
/// Indices reference into the positions array as a triangle list.
#[derive(Debug, Clone)]
pub struct LayerGeometry {
    /// Interleaved vertex positions `[x0, y0, x1, y1, ...]`.
    pub positions: Vec<f32>,
    /// Triangle-list indices into the positions array.
    pub indices: Vec<u32>,
    /// Axis-aligned bounding box of all vertices.
    pub bounds: BoundingBox,
    /// Number of Gerber commands processed.
    pub command_count: u32,
    /// Number of vertices (`positions.len() / 2`).
    pub vertex_count: u32,
    /// Warning messages generated during conversion.
    pub warnings: Vec<String>,
    /// Index ranges for clear-polarity geometry `(start, end)` pairs.
    pub clear_ranges: Vec<(u32, u32)>,
}

/// Metadata returned to JavaScript for a parsed layer.
#[derive(Debug, Clone, Serialize)]
pub struct LayerMeta {
    /// Axis-aligned bounding box.
    pub bounds: BoundingBox,
    /// Number of vertices.
    pub vertex_count: u32,
    /// Number of triangle indices.
    pub index_count: u32,
    /// Number of Gerber commands processed.
    pub command_count: u32,
    /// Number of warnings.
    pub warning_count: u32,
    /// Warning messages.
    pub warnings: Vec<String>,
}

/// Accumulator for building layer geometry incrementally.
///
/// Passed by mutable reference to geometry conversion functions.
/// Vertices and indices are collected in flat `Vec`s to minimize allocations.
#[derive(Debug)]
pub struct GeometryBuilder {
    positions: Vec<f32>,
    indices: Vec<u32>,
    bounds: BoundingBox,
    warnings: Vec<String>,
    /// Index ranges for clear-polarity geometry, populated by macro evaluator.
    clear_ranges: Vec<(u32, u32)>,
}

impl GeometryBuilder {
    /// Creates an empty builder.
    pub const fn new() -> Self {
        Self {
            positions: Vec::new(),
            indices: Vec::new(),
            bounds: BoundingBox::new(),
            warnings: Vec::new(),
            clear_ranges: Vec::new(),
        }
    }

    /// Adds a vertex and returns its index.
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn push_vertex(&mut self, x: f64, y: f64) -> u32 {
        let idx = self.positions.len() / 2;
        self.positions.push(x as f32);
        self.positions.push(y as f32);
        self.bounds.update(x, y);
        idx as u32
    }

    /// Adds a triangle from three vertex indices.
    pub fn push_triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.push(a);
        self.indices.push(b);
        self.indices.push(c);
    }

    /// Adds a quad as two triangles `(a, b, c)` and `(a, c, d)`.
    pub fn push_quad(&mut self, a: u32, b: u32, c: u32, d: u32) {
        self.push_triangle(a, b, c);
        self.push_triangle(a, c, d);
    }

    /// Adds an N-gon centered at `(cx, cy)` with the given `radius` and `segments`.
    ///
    /// Vertices are placed on a circle and fan-triangulated from the first vertex.
    /// Returns the index of the first vertex. `segments` should be >= 3 for
    /// meaningful polygons.
    pub fn push_ngon(&mut self, cx: f64, cy: f64, radius: f64, segments: u32) -> u32 {
        let first = self.push_vertex(cx + radius, cy);

        for i in 1..segments {
            let angle = 2.0 * std::f64::consts::PI * f64::from(i) / f64::from(segments);
            self.push_vertex(
                radius.mul_add(angle.cos(), cx),
                radius.mul_add(angle.sin(), cy),
            );
        }

        for i in 1..segments.saturating_sub(1) {
            self.push_triangle(first, first + i, first + i + 1);
        }

        first
    }

    /// Records a warning message.
    pub fn warn(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    /// Records an index range for clear-polarity geometry.
    ///
    /// Used by aperture macro evaluator when a primitive has exposure off.
    pub fn record_clear_range(&mut self, start: u32, end: u32) {
        if end > start {
            self.clear_ranges.push((start, end));
        }
    }

    /// Returns the current number of triangle indices.
    #[must_use]
    pub fn index_count(&self) -> u32 {
        u32::try_from(self.indices.len()).unwrap_or(u32::MAX)
    }

    /// Returns the current number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> u32 {
        u32::try_from(self.positions.len() / 2).unwrap_or(u32::MAX)
    }

    /// Consumes the builder and produces a [`LayerGeometry`].
    ///
    /// `command_count` is set to 0; the caller should update it as needed.
    /// `clear_ranges` is initialized empty; the caller may populate it from a
    /// [`super::polarity::PolarityTracker`].
    pub fn build(self) -> LayerGeometry {
        let vertex_count = u32::try_from(self.positions.len() / 2).unwrap_or(u32::MAX);
        LayerGeometry {
            positions: self.positions,
            indices: self.indices,
            bounds: self.bounds,
            command_count: 0,
            vertex_count,
            warnings: self.warnings,
            clear_ranges: self.clear_ranges,
        }
    }
}

impl Default for GeometryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn push_vertex_adds_two_floats() {
        let mut b = GeometryBuilder::new();
        b.push_vertex(1.0, 2.0);
        let geom = b.build();
        assert_eq!(geom.positions.len(), 2);
    }

    #[test]
    fn push_three_vertices_six_floats() {
        let mut b = GeometryBuilder::new();
        b.push_vertex(1.0, 2.0);
        b.push_vertex(3.0, 4.0);
        b.push_vertex(5.0, 6.0);
        let geom = b.build();
        assert_eq!(geom.positions.len(), 6);
    }

    #[test]
    fn push_vertex_returns_sequential_indices() {
        let mut b = GeometryBuilder::new();
        assert_eq!(b.push_vertex(0.0, 0.0), 0);
        assert_eq!(b.push_vertex(1.0, 0.0), 1);
        assert_eq!(b.push_vertex(2.0, 0.0), 2);
    }

    #[test]
    fn push_triangle_adds_three_indices() {
        let mut b = GeometryBuilder::new();
        b.push_vertex(0.0, 0.0);
        b.push_vertex(1.0, 0.0);
        b.push_vertex(0.0, 1.0);
        b.push_triangle(0, 1, 2);
        let geom = b.build();
        assert_eq!(geom.indices.len(), 3);
        assert_eq!(geom.indices, vec![0, 1, 2]);
    }

    #[test]
    fn push_quad_adds_six_indices() {
        let mut b = GeometryBuilder::new();
        b.push_vertex(0.0, 0.0);
        b.push_vertex(1.0, 0.0);
        b.push_vertex(1.0, 1.0);
        b.push_vertex(0.0, 1.0);
        b.push_quad(0, 1, 2, 3);
        let geom = b.build();
        assert_eq!(geom.indices.len(), 6);
        assert_eq!(geom.indices, vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn push_ngon_four_creates_four_vertices() {
        let mut b = GeometryBuilder::new();
        b.push_ngon(0.0, 0.0, 1.0, 4);
        let geom = b.build();
        assert_eq!(geom.positions.len(), 8);
        assert_eq!(geom.vertex_count, 4);
    }

    #[test]
    fn push_ngon_four_vertices_on_unit_circle() {
        let mut b = GeometryBuilder::new();
        b.push_ngon(0.0, 0.0, 1.0, 4);
        let geom = b.build();
        let eps = 1e-6_f32;

        // v0 at (1, 0)
        assert!((geom.positions[0] - 1.0).abs() < eps);
        assert!((geom.positions[1]).abs() < eps);

        // v1 at (0, 1)
        assert!((geom.positions[2]).abs() < eps);
        assert!((geom.positions[3] - 1.0).abs() < eps);

        // v2 at (-1, 0)
        assert!((geom.positions[4] + 1.0).abs() < eps);
        assert!((geom.positions[5]).abs() < eps);

        // v3 at (0, -1)
        assert!((geom.positions[6]).abs() < eps);
        assert!((geom.positions[7] + 1.0).abs() < eps);
    }

    #[test]
    fn push_ngon_triangulation() {
        let mut b = GeometryBuilder::new();
        let first = b.push_ngon(0.0, 0.0, 1.0, 4);
        let geom = b.build();
        // 4-gon → 2 triangles → 6 indices
        assert_eq!(geom.indices.len(), 6);
        assert_eq!(
            geom.indices,
            vec![first, first + 1, first + 2, first, first + 2, first + 3]
        );
    }

    #[test]
    fn build_returns_correct_vertex_count() {
        let mut b = GeometryBuilder::new();
        b.push_vertex(0.0, 0.0);
        b.push_vertex(1.0, 1.0);
        b.push_vertex(2.0, 2.0);
        b.push_vertex(3.0, 3.0);
        b.push_vertex(4.0, 4.0);
        let geom = b.build();
        assert_eq!(geom.vertex_count, 5);
    }

    #[test]
    fn bounding_box_updates_on_push() {
        let mut b = GeometryBuilder::new();
        b.push_vertex(1.0, 2.0);
        b.push_vertex(-3.0, 4.0);
        let geom = b.build();
        assert!((geom.bounds.min_x - (-3.0)).abs() < f64::EPSILON);
        assert!((geom.bounds.min_y - 2.0).abs() < f64::EPSILON);
        assert!((geom.bounds.max_x - 1.0).abs() < f64::EPSILON);
        assert!((geom.bounds.max_y - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn warn_records_messages() {
        let mut b = GeometryBuilder::new();
        b.warn("first warning".to_string());
        b.warn("second warning".to_string());
        let geom = b.build();
        assert_eq!(geom.warnings.len(), 2);
        assert_eq!(geom.warnings[0], "first warning");
        assert_eq!(geom.warnings[1], "second warning");
    }

    #[test]
    fn empty_builder_builds_empty_geometry() {
        let geom = GeometryBuilder::new().build();
        assert_eq!(geom.positions.len(), 0);
        assert_eq!(geom.indices.len(), 0);
        assert_eq!(geom.vertex_count, 0);
        assert_eq!(geom.command_count, 0);
        assert!(geom.warnings.is_empty());
    }
}
