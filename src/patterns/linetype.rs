//! Linetype definitions: named patterns and ggplot2-style hex patterns,
//! resolved to [`LinetypeStep`] sequences — the crate's single linetype
//! representation, shared with [`Value::Linetype`](crate::Value::Linetype)
//! and [`OutputRange::Linetypes`](crate::OutputRange::Linetypes).
//!
//! Lengths are unitless multiples of the line's width (ggplot2
//! convention). Resolving them to pt / px is the consumer's job — this
//! crate defines the pattern, not the rendering.

use std::sync::Arc;

use crate::value::LinetypeStep;

/// Dashed: regular dashes `[6, 4]`.
pub const DASHED: &[LinetypeStep] = &[LinetypeStep::Dash(6.0), LinetypeStep::Gap(4.0)];
/// Dotted: dots `[1, 2]`.
pub const DOTTED: &[LinetypeStep] = &[LinetypeStep::Dash(1.0), LinetypeStep::Gap(2.0)];
/// Dotdash: alternating dots and dashes `[1, 2, 6, 2]`.
pub const DOTDASH: &[LinetypeStep] = &[
    LinetypeStep::Dash(1.0),
    LinetypeStep::Gap(2.0),
    LinetypeStep::Dash(6.0),
    LinetypeStep::Gap(2.0),
];
/// Longdash: longer dashes `[10, 4]`.
pub const LONGDASH: &[LinetypeStep] = &[LinetypeStep::Dash(10.0), LinetypeStep::Gap(4.0)];
/// Twodash: two-dash pattern `[6, 2, 2, 2]`.
pub const TWODASH: &[LinetypeStep] = &[
    LinetypeStep::Dash(6.0),
    LinetypeStep::Gap(2.0),
    LinetypeStep::Dash(2.0),
    LinetypeStep::Gap(2.0),
];

/// Parse a ggplot2-style hex string linetype pattern.
///
/// Format: Even number (2-8) of hex digits, each specifying on/off lengths.
/// Even-indexed digits become `Dash`, odd-indexed `Gap`.
/// Examples: "33" = [Dash(3), Gap(3)], "1343" = [Dash(1), Gap(3), Dash(4), Gap(3)]
fn parse_hex_linetype(s: &str) -> Option<Vec<LinetypeStep>> {
    let len = s.len();

    // Must be even length, 2-8 characters, all hex digits
    if !(2..=8).contains(&len) || len % 2 != 0 {
        return None;
    }

    // Parse each character as a hex digit
    let mut result = Vec::with_capacity(len);
    for (i, c) in s.chars().enumerate() {
        let digit = c.to_digit(16)?;
        if digit == 0 {
            return None; // ggplot2 requires non-zero digits
        }
        result.push(if i % 2 == 0 {
            LinetypeStep::Dash(digit as f64)
        } else {
            LinetypeStep::Gap(digit as f64)
        });
    }

    Some(result)
}

/// Resolve a linetype specification to a [`LinetypeStep`] pattern.
///
/// Supports:
/// - Named linetypes: "solid", "dashed", "dotted", "dotdash", "longdash", "twodash"
/// - Hex string patterns: "33", "1343", "44", etc. (2-8 hex digits)
///
/// `solid` resolves to an empty pattern — the same convention
/// [`OutputRange::Linetypes`](crate::OutputRange::Linetypes) uses.
///
/// # Hex string patterns
/// A string of 2-8 hex digits (even count), where each digit specifies
/// the length of alternating on/off segments. For example:
/// - `"33"` = 3 line widths on, 3 off
/// - `"1343"` = 1 on, 3 off, 4 on, 3 off
/// - `"af"` = 10 on, 15 off (hex digits a-f supported)
pub fn linetype_pattern(name: &str) -> Option<Arc<[LinetypeStep]>> {
    // First try named linetypes (case-insensitive)
    match name.to_lowercase().as_str() {
        "solid" => return Some(Arc::from(&[][..])),
        "dashed" => return Some(Arc::from(DASHED)),
        "dotted" => return Some(Arc::from(DOTTED)),
        "dotdash" => return Some(Arc::from(DOTDASH)),
        "longdash" => return Some(Arc::from(LONGDASH)),
        "twodash" => return Some(Arc::from(TWODASH)),
        _ => {}
    }

    // Then try hex string pattern
    parse_hex_linetype(name).map(Arc::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(on_off: &[f64]) -> Vec<LinetypeStep> {
        on_off
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                if i % 2 == 0 {
                    LinetypeStep::Dash(v)
                } else {
                    LinetypeStep::Gap(v)
                }
            })
            .collect()
    }

    fn assert_pattern(name: &str, on_off: &[f64]) {
        let pattern = linetype_pattern(name).unwrap();
        assert_eq!(&pattern[..], &steps(on_off)[..], "pattern for {name:?}");
    }

    #[test]
    fn test_linetype_pattern_known() {
        assert!(linetype_pattern("solid").unwrap().is_empty());
        assert_pattern("dashed", &[6.0, 4.0]);
        assert_pattern("dotted", &[1.0, 2.0]);
        assert_pattern("dotdash", &[1.0, 2.0, 6.0, 2.0]);
        assert_pattern("longdash", &[10.0, 4.0]);
        assert_pattern("twodash", &[6.0, 2.0, 2.0, 2.0]);
    }

    #[test]
    fn test_linetype_pattern_case_insensitive() {
        assert!(linetype_pattern("SOLID").is_some());
        assert!(linetype_pattern("Dashed").is_some());
        assert!(linetype_pattern("DoTdAsH").is_some());
    }

    #[test]
    fn test_linetype_pattern_unknown() {
        assert!(linetype_pattern("unknown").is_none());
        assert!(linetype_pattern("").is_none());
    }

    #[test]
    fn test_hex_linetype_basic() {
        assert_pattern("33", &[3.0, 3.0]);
        assert_pattern("44", &[4.0, 4.0]);
        assert_pattern("13", &[1.0, 3.0]);
    }

    #[test]
    fn test_hex_linetype_four_digit() {
        assert_pattern("1343", &[1.0, 3.0, 4.0, 3.0]);
        assert_pattern("2262", &[2.0, 2.0, 6.0, 2.0]);
    }

    #[test]
    fn test_hex_linetype_six_and_eight_digit() {
        assert_pattern("123456", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_pattern("12345678", &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn test_hex_linetype_ggplot2_standards() {
        // ggplot2's standard dash-dot patterns
        assert_pattern("44", &[4.0, 4.0]); // dashed
        assert_pattern("13", &[1.0, 3.0]); // dotted
        assert_pattern("1343", &[1.0, 3.0, 4.0, 3.0]); // dotdash
        assert_pattern("73", &[7.0, 3.0]); // longdash
        assert_pattern("2262", &[2.0, 2.0, 6.0, 2.0]); // twodash
    }

    #[test]
    fn test_hex_linetype_with_letters() {
        // Hex digits a-f should work
        assert_pattern("af", &[10.0, 15.0]);
        assert_pattern("AF", &[10.0, 15.0]);
        assert_pattern("1a2b", &[1.0, 10.0, 2.0, 11.0]);
    }

    #[test]
    fn test_hex_linetype_invalid() {
        // Odd length
        assert!(linetype_pattern("123").is_none());
        // Too short
        assert!(linetype_pattern("1").is_none());
        // Too long (>8)
        assert!(linetype_pattern("1234567890").is_none());
        // Contains zero (invalid in ggplot2)
        assert!(linetype_pattern("10").is_none());
        assert!(linetype_pattern("01").is_none());
        // Non-hex characters
        assert!(linetype_pattern("gg").is_none());
        assert!(linetype_pattern("1x").is_none());
    }
}
