//! Polarity tracking for geometry conversion.
//!
//! Tracks dark vs clear polarity and records index ranges for clear-polarity
//! geometry so the renderer can apply background color.

use super::types::{GeometryBuilder, LayerGeometry, Polarity};

/// Index range for clear-polarity geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearRange {
    /// Start index (inclusive) in the triangle index buffer.
    pub index_start: u32,
    /// End index (exclusive) in the triangle index buffer.
    pub index_end: u32,
}

/// Tracks polarity state and records clear-polarity index ranges.
#[derive(Debug)]
pub struct PolarityTracker {
    /// Current polarity.
    polarity: Polarity,
    /// Start index of an open clear range, if any.
    clear_start: Option<u32>,
    /// Accumulated clear ranges.
    clear_ranges: Vec<ClearRange>,
}

impl PolarityTracker {
    /// Creates a new tracker with dark polarity.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            polarity: Polarity::Dark,
            clear_start: None,
            clear_ranges: Vec::new(),
        }
    }

    /// Returns the current polarity.
    #[must_use]
    pub const fn current(&self) -> Polarity {
        self.polarity
    }

    /// Updates polarity. When switching to clear, records the current index
    /// count as the start of a clear range. When switching to dark, closes
    /// the current clear range.
    pub fn set_polarity(&mut self, p: Polarity, builder: &GeometryBuilder) {
        if p == self.polarity {
            return;
        }

        let idx = builder.index_count();

        if p == Polarity::Clear {
            self.clear_start = Some(idx);
        } else if let Some(start) = self.clear_start.take() {
            if idx > start {
                self.clear_ranges.push(ClearRange {
                    index_start: start,
                    index_end: idx,
                });
            }
        }

        self.polarity = p;
    }

    /// Finishes tracking and returns all clear ranges.
    ///
    /// Closes any open clear range if still in clear polarity.
    #[must_use]
    pub fn finish(mut self, builder: &GeometryBuilder) -> Vec<ClearRange> {
        if self.polarity == Polarity::Clear {
            if let Some(start) = self.clear_start.take() {
                let idx = builder.index_count();
                if idx > start {
                    self.clear_ranges.push(ClearRange {
                        index_start: start,
                        index_end: idx,
                    });
                }
            }
        }
        self.clear_ranges
    }
}

impl Default for PolarityTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Applies clear ranges from a polarity tracker to a layer geometry.
///
/// Call after [`GeometryBuilder::build`] to merge tracker ranges with any
/// ranges already recorded by the builder (e.g. from macro primitives with
/// exposure off).
pub fn apply_clear_ranges(geom: &mut LayerGeometry, ranges: Vec<ClearRange>) {
    for r in ranges {
        geom.clear_ranges.push((r.index_start, r.index_end));
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn ut_pol_001_dark_polarity_produces_normal_geometry() {
        let mut builder = GeometryBuilder::new();
        let tracker = PolarityTracker::new();

        assert_eq!(tracker.current(), Polarity::Dark);

        builder.push_vertex(0.0, 0.0);
        builder.push_vertex(1.0, 0.0);
        builder.push_vertex(0.0, 1.0);
        builder.push_triangle(0, 1, 2);

        let ranges = tracker.finish(&builder);
        assert!(
            ranges.is_empty(),
            "dark polarity should produce no clear ranges"
        );
    }

    #[test]
    fn ut_pol_002_clear_polarity_produces_geometry_with_background_flag() {
        let mut builder = GeometryBuilder::new();
        let mut tracker = PolarityTracker::new();

        tracker.set_polarity(Polarity::Clear, &builder);

        builder.push_vertex(0.0, 0.0);
        builder.push_vertex(1.0, 0.0);
        builder.push_vertex(0.0, 1.0);
        builder.push_triangle(0, 1, 2);

        let ranges = tracker.finish(&builder);
        assert!(
            !ranges.is_empty(),
            "clear polarity should produce clear ranges"
        );
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].index_start, 0);
        assert_eq!(ranges[0].index_end, 3);
    }

    #[test]
    fn ut_pol_003_polarity_switch_mid_layer() {
        let mut builder = GeometryBuilder::new();
        let mut tracker = PolarityTracker::new();

        builder.push_vertex(0.0, 0.0);
        builder.push_vertex(1.0, 0.0);
        builder.push_vertex(0.0, 1.0);
        builder.push_triangle(0, 1, 2);

        tracker.set_polarity(Polarity::Clear, &builder);

        builder.push_vertex(2.0, 0.0);
        builder.push_vertex(3.0, 0.0);
        builder.push_vertex(2.0, 1.0);
        builder.push_triangle(3, 4, 5);

        tracker.set_polarity(Polarity::Dark, &builder);

        builder.push_vertex(4.0, 0.0);
        builder.push_vertex(5.0, 0.0);
        builder.push_vertex(4.0, 1.0);
        builder.push_triangle(6, 7, 8);

        let ranges = tracker.finish(&builder);
        assert_eq!(ranges.len(), 1);
        let r0 = ranges.first().expect("one range");
        assert_eq!(r0.index_start, 3);
        assert_eq!(r0.index_end, 6);
    }
}
