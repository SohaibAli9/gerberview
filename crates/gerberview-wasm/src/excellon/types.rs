//! Excellon drill file types.

/// A single drill hole from Excellon parsing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrillHole {
    /// X coordinate of the hole center.
    pub x: f64,
    /// Y coordinate of the hole center.
    pub y: f64,
    /// Diameter of the drill hole.
    pub diameter: f64,
}

/// Excellon tool definition from the file header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolDefinition {
    /// Tool number (T1, T2, etc.).
    pub number: u32,
    /// Drill diameter.
    pub diameter: f64,
}

/// Unit system for Excellon files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExcellonUnits {
    /// Metric (millimeters).
    Metric,
    /// Imperial (inches).
    Imperial,
}

/// Result of Excellon parsing for a single file.
#[derive(Debug, Clone)]
pub struct ExcellonResult {
    /// All drill holes extracted from the file.
    pub holes: Vec<DrillHole>,
    /// Tool definitions from the file header.
    pub tools: Vec<ToolDefinition>,
    /// Unit system specified in the file.
    pub units: ExcellonUnits,
    /// Parser warnings encountered while processing the file.
    pub warnings: Vec<String>,
}
