//! Tick-density specification: a count, or a desired physical spacing.
//!
//! The spacing variants say "about one major tick every N pt" and stay
//! unresolved until break time, when the consumer — the only one who
//! knows the rendered axis length — hands its extent to
//! [`resolve_target`]. Resolution yields a *count*; the pretty
//! algorithm (Wilkinson for continuous scales, the calendar-interval
//! picker for temporal ones) still chooses the actual positions, so the
//! spacing is approximate by design.
//!
//! Three flavors, differing only in how `axis_len / spacing` rounds to
//! a count: [`BreakTarget::Spacing`] rounds to nearest (N pt on
//! average), [`BreakTarget::MaxSpacing`] rounds up ("<N pt" — the gap
//! never exceeds N, erring towards one more break), and
//! [`BreakTarget::MinSpacing`] rounds down (">N pt" — the gap never
//! dips below N, erring towards one fewer).

use crate::breaks::DEFAULT_BREAK_COUNT;

/// How a scale's major-tick density is specified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BreakTarget {
    /// A tick count, passed to the break algorithm as-is (target, not
    /// guarantee — Wilkinson typically lands within ±2).
    Count(usize),
    /// Desired physical spacing between adjacent major ticks, in pt.
    /// Resolved to a count at break time against the axis extent,
    /// rounding to the nearest count.
    Spacing(f64),
    /// Maximum physical spacing between adjacent major ticks, in pt
    /// ("<N pt"). Resolves by rounding the count *up*, so the resolved
    /// gap never exceeds N — errs towards one more break.
    MaxSpacing(f64),
    /// Minimum physical spacing between adjacent major ticks, in pt
    /// (">N pt"). Resolves by rounding the count *down*, so the
    /// resolved gap never dips below N — errs towards one fewer break.
    MinSpacing(f64),
}

/// Resolve a [`BreakTarget`] to a tick count. `axis_len_pt` is the
/// rendered axis extent in pt, supplied by the consumer; only the
/// spacing variants consult it. Degenerate inputs — non-positive or
/// non-finite spacing, or no usable axis length — fall back to
/// [`DEFAULT_BREAK_COUNT`] rather than erroring, so a legend or an
/// unlaid-out axis still gets sensible ticks. The resolved count floors
/// at 2, which can breach a [`BreakTarget::MinSpacing`] guarantee on a
/// very short axis.
pub fn resolve_target(target: BreakTarget, axis_len_pt: f64) -> usize {
    let (spacing, round): (f64, fn(f64) -> f64) = match target {
        BreakTarget::Count(n) => return n,
        BreakTarget::Spacing(s) => (s, f64::round),
        BreakTarget::MaxSpacing(s) => (s, f64::ceil),
        BreakTarget::MinSpacing(s) => (s, f64::floor),
    };
    if spacing.is_finite() && spacing > 0.0 && axis_len_pt.is_finite() {
        round(axis_len_pt / spacing).max(2.0) as usize
    } else {
        DEFAULT_BREAK_COUNT
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
        assert_eq!(resolve_target(BreakTarget::Spacing(28.0), 100.0), 4);
    }

    #[test]
    fn max_spacing_rounds_up() {
        // "<N pt": the resolved gap never exceeds N.
        assert_eq!(resolve_target(BreakTarget::MaxSpacing(45.0), 100.0), 3);
        assert_eq!(resolve_target(BreakTarget::MaxSpacing(40.0), 100.0), 3);
        assert_eq!(resolve_target(BreakTarget::MaxSpacing(30.0), 100.0), 4);
        assert_eq!(resolve_target(BreakTarget::MaxSpacing(29.0), 100.0), 4);
    }

    #[test]
    fn min_spacing_rounds_down() {
        // ">N pt": the resolved gap never dips below N.
        assert_eq!(resolve_target(BreakTarget::MinSpacing(45.0), 100.0), 2);
        assert_eq!(resolve_target(BreakTarget::MinSpacing(40.0), 100.0), 2);
        assert_eq!(resolve_target(BreakTarget::MinSpacing(30.0), 100.0), 3);
        assert_eq!(resolve_target(BreakTarget::MinSpacing(29.0), 100.0), 3);
    }

    #[test]
    fn spacing_floors_at_two() {
        assert_eq!(resolve_target(BreakTarget::Spacing(10_000.0), 100.0), 2);
        assert_eq!(resolve_target(BreakTarget::MaxSpacing(10_000.0), 100.0), 2);
        assert_eq!(resolve_target(BreakTarget::MinSpacing(10_000.0), 100.0), 2);
    }

    #[test]
    fn degenerate_spacing_falls_back_to_default_count() {
        for target in [
            BreakTarget::Spacing(0.0),
            BreakTarget::MaxSpacing(-10.0),
            BreakTarget::MinSpacing(f64::NAN),
            BreakTarget::MaxSpacing(f64::INFINITY),
        ] {
            assert_eq!(resolve_target(target, 400.0), DEFAULT_BREAK_COUNT);
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
