//! One step in a linetype pattern, the unit dash patterns are built from.

use std::sync::Arc;

/// One step in a linetype pattern.
///
/// Patterns are even-length sequences where even-indexed entries are
/// `Dash` or `Marker` (something to draw at the cursor) and odd-indexed
/// entries are `Gap` (an unconditional advance).
///
/// `PartialEq` follows f64's IEEE semantics for `Dash` / `Gap`
/// (NaN ≠ NaN); use [`Value::key_eq`](crate::value::Value::key_eq) /
/// `key_hash` for the canonicalised diff-friendly comparison.
#[derive(Clone, Debug, PartialEq)]
pub enum LinetypeStep {
    /// Stroke a segment of this length along the line, then advance the
    /// cursor by the same amount. Lengths are unitless multiples of the
    /// line's width (ggplot2 convention); the consumer resolves them to
    /// pt / px when it draws.
    Dash(f64),
    /// Stamp the named shape at the current cursor. The marker is
    /// assumed to occupy one line width of arc length so the next gap
    /// measures clear space starting from the marker's trailing edge.
    Marker(Arc<str>),
    /// Advance the cursor by this many line widths without drawing.
    Gap(f64),
}

impl LinetypeStep {
    /// `true` if this is a `Marker` step.
    pub fn is_marker(&self) -> bool {
        matches!(self, LinetypeStep::Marker(_))
    }
}
