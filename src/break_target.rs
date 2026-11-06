//! Tick-density specification: a count, or a desired physical spacing.
//!
//! [`BreakTarget::Spacing`] says "about one major tick every N pt" and
//! stays unresolved until break time, when the consumer — the only one
//! who knows the rendered axis length — hands its extent to
//! [`resolve_target`]. Resolution yields a *count*; the pretty
//! algorithm (Wilkinson for continuous scales, the calendar-interval
//! picker for temporal ones) still chooses the actual positions, so the
//! spacing is approximate by design.

use crate::breaks::DEFAULT_BREAK_COUNT;

/// How a scale's major-tick density is specified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BreakTarget {
    /// A tick count, passed to the break algorithm as-is (target, not
    /// guarantee — Wilkinson typically lands within ±2).
    Count(usize),
    /// Desired physical spacing between adjacent major ticks, in pt.
    /// Resolved to a count at break time against the axis extent.
    Spacing(f64),
}

/// Resolve a [`BreakTarget`] to a tick count. `axis_len_pt` is the
/// rendered axis extent in pt, supplied by the consumer; only
/// `Spacing` consults it. Degenerate inputs — non-positive or
/// non-finite spacing, or no usable axis length — fall back to
/// [`DEFAULT_BREAK_COUNT`] rather than erroring, so a legend or an
/// unlaid-out axis still gets sensible ticks.
pub fn resolve_target(target: BreakTarget, axis_len_pt: f64) -> usize {
    match target {
        BreakTarget::Count(n) => n,
        BreakTarget::Spacing(spacing)
            if spacing.is_finite() && spacing > 0.0 && axis_len_pt.is_finite() =>
        {
            (axis_len_pt / spacing).round().max(2.0) as usize
        }
        BreakTarget::Spacing(_) => DEFAULT_BREAK_COUNT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_passes_through_untouched() {
        assert_eq!(resolve_target(BreakTarget::Count(7), 400.0), 7);
        // Zero is the caller's problem; the count algorithms clamp it.
        assert_eq!(resolve_target(BreakTarget::Count(0), 400.0), 0);
    }

    #[test]
    fn spacing_resolves_against_axis_length() {
        // 400 pt axis, 56.7 pt (~2 cm) spacing → ~7 ticks.
        assert_eq!(resolve_target(BreakTarget::Spacing(56.7), 400.0), 7);
        assert_eq!(resolve_target(BreakTarget::Spacing(100.0), 400.0), 4);
    }

    #[test]
    fn spacing_rounds_to_nearest_count() {
        assert_eq!(resolve_target(BreakTarget::Spacing(30.0), 100.0), 3);
        assert_eq!(resolve_target(BreakTarget::Spacing(29.0), 100.0), 3);
    }

    #[test]
    fn spacing_floors_at_two() {
        assert_eq!(resolve_target(BreakTarget::Spacing(10_000.0), 100.0), 2);
    }

    #[test]
    fn degenerate_spacing_falls_back_to_default_count() {
        for spacing in [0.0, -10.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                resolve_target(BreakTarget::Spacing(spacing), 400.0),
                DEFAULT_BREAK_COUNT
            );
        }
    }

    #[test]
    fn degenerate_axis_length_falls_back_to_default_count() {
        for len in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                resolve_target(BreakTarget::Spacing(50.0), len),
                DEFAULT_BREAK_COUNT
            );
        }
    }
}
