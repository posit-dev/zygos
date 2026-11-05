//! Linetype palettes: curated name lists and the sequential generator.
//! The patterns these names resolve to are defined in
//! [`crate::patterns::linetype`].

use std::sync::Arc;

use crate::value::LinetypeStep;

/// Default linetypes ordered by distinguishability.
/// Resolve names to patterns with
/// [`linetype_pattern`](crate::patterns::linetype::linetype_pattern).
pub const LINETYPES: &[&str] = &[
    "solid", "dashed", "dotted", "dotdash", "longdash", "twodash",
];

/// Look up a linetype palette by name.
pub fn get_linetype_palette(name: &str) -> Option<&'static [&'static str]> {
    match name.to_lowercase().as_str() {
        "categorical" | "default" => Some(LINETYPES),
        _ => None,
    }
}

/// Generate linetypes with evenly-spaced ink densities.
///
/// Creates `count` linetypes ranging from sparse (low ink) to solid (100%
/// ink, an empty pattern). Each non-solid linetype is a two-step
/// `[Dash(on), Gap(off)]` pattern with a cycle of 16 line widths, so ink
/// densities range from 6.25% to 93.75% before reaching solid.
///
/// # Examples
///
/// ```
/// use zygos::palettes::linetype::generate_linetype_sequential;
/// use zygos::LinetypeStep;
///
/// let types = generate_linetype_sequential(3);
/// // 6.25% ink, 50% ink, solid (empty pattern)
/// assert_eq!(&types[0][..], &[LinetypeStep::Dash(1.0), LinetypeStep::Gap(15.0)]);
/// assert_eq!(&types[1][..], &[LinetypeStep::Dash(8.0), LinetypeStep::Gap(8.0)]);
/// assert!(types[2].is_empty());
/// ```
pub fn generate_linetype_sequential(count: usize) -> Vec<Arc<[LinetypeStep]>> {
    /// Minimum ink density: 1/16 ≈ 6.25%
    const MIN_INK_RATIO: f64 = 1.0 / 16.0;
    /// Maximum ink density for non-solid: 15/16 ≈ 93.75%
    const MAX_INK_RATIO: f64 = 15.0 / 16.0;

    if count == 0 {
        return vec![];
    }
    if count == 1 {
        return vec![Arc::from(&[][..])]; // solid
    }

    let mut result = Vec::with_capacity(count);

    // Evenly spaced ink densities from MIN_INK_RATIO to 100%; last is solid
    for i in 0..count {
        let t = i as f64 / (count - 1) as f64;

        if i == count - 1 {
            result.push(Arc::from(&[][..])); // solid
        } else {
            let ink_pct = MIN_INK_RATIO + t * (MAX_INK_RATIO - MIN_INK_RATIO);

            // Pattern with cycle of 16 line widths: on / 16 = ink_pct
            let on = (ink_pct * 16.0).round() as u32;
            let on = on.clamp(1, 15);
            let off = 16 - on;
            let off = off.clamp(1, 15);

            result.push(Arc::from(vec![
                LinetypeStep::Dash(on as f64),
                LinetypeStep::Gap(off as f64),
            ]));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_linetype_palette() {
        assert_eq!(get_linetype_palette("default").unwrap().len(), 6);
        assert!(get_linetype_palette("CATEGORICAL").is_some());
        assert!(get_linetype_palette("unknown_lt").is_none());
    }

    #[test]
    fn test_linetype_palette_names_resolve() {
        for name in LINETYPES {
            assert!(
                crate::patterns::linetype::linetype_pattern(name).is_some(),
                "palette linetype '{name}' has no pattern"
            );
        }
    }

    #[test]
    fn test_generate_linetype_sequential() {
        assert!(generate_linetype_sequential(0).is_empty());
        assert!(generate_linetype_sequential(1)[0].is_empty()); // solid

        let three = generate_linetype_sequential(3);
        assert_eq!(
            &three[0][..],
            &[LinetypeStep::Dash(1.0), LinetypeStep::Gap(15.0)]
        );
        assert_eq!(
            &three[1][..],
            &[LinetypeStep::Dash(8.0), LinetypeStep::Gap(8.0)]
        );
        assert!(three[2].is_empty()); // solid

        let five = generate_linetype_sequential(5);
        assert_eq!(five.len(), 5);
        assert!(five[4].is_empty()); // solid
    }

    #[test]
    fn test_generate_linetype_sequential_valid_patterns() {
        // Every non-solid pattern is an alternating Dash/Gap sequence of
        // positive lengths, with ink density increasing along the sequence.
        for count in 2..=10 {
            let types = generate_linetype_sequential(count);
            let densities: Vec<f64> = types
                .iter()
                .map(|pattern| {
                    if pattern.is_empty() {
                        return 1.0; // solid
                    }
                    assert_eq!(pattern.len() % 2, 0, "pattern should alternate on/off");
                    let mut ink = 0.0;
                    let mut cycle = 0.0;
                    for (i, step) in pattern.iter().enumerate() {
                        match (i % 2, step) {
                            (0, LinetypeStep::Dash(v)) => {
                                ink += v;
                                cycle += v;
                            }
                            (1, LinetypeStep::Gap(v)) => cycle += v,
                            _ => panic!("steps should alternate Dash/Gap: {pattern:?}"),
                        }
                    }
                    ink / cycle
                })
                .collect();
            assert!(
                densities.windows(2).all(|w| w[0] < w[1]),
                "ink density should strictly increase: {densities:?}"
            );
        }
    }
}
