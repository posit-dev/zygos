//! Out-of-bounds handling for values outside a scale's continuous domain.

/// Policy for values falling outside a scale's input domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OobPolicy {
    /// Drop the value (maps to NaN / is filtered by the consumer).
    #[default]
    Censor,
    /// Keep the value as-is, extending beyond the domain.
    Keep,
    /// Clamp the value to the nearest domain edge.
    Squish,
}

impl OobPolicy {
    /// Parse a policy name ("censor", "keep", "squish", "none" -> Keep), case-insensitive.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "censor" => Some(Self::Censor),
            "keep" | "none" => Some(Self::Keep),
            "squish" => Some(Self::Squish),
            _ => None,
        }
    }

    /// The canonical lowercase name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Censor => "censor",
            Self::Keep => "keep",
            Self::Squish => "squish",
        }
    }

    /// Apply the policy: returns None for censored out-of-range values,
    /// Some(clamped) for squish, Some(original) for keep.
    ///
    /// Values exactly on `min` or `max` are in range. NaN passes through
    /// as `Some(NaN)` under every policy.
    pub fn apply(self, value: f64, min: f64, max: f64) -> Option<f64> {
        if value.is_nan() || (value >= min && value <= max) {
            return Some(value);
        }
        match self {
            Self::Censor => None,
            Self::Keep => Some(value),
            Self::Squish => Some(if value < min { min } else { max }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn censor_drops_out_of_range() {
        assert_eq!(OobPolicy::Censor.apply(-1.0, 0.0, 10.0), None);
        assert_eq!(OobPolicy::Censor.apply(11.0, 0.0, 10.0), None);
    }

    #[test]
    fn keep_preserves_value() {
        assert_eq!(OobPolicy::Keep.apply(-1.0, 0.0, 10.0), Some(-1.0));
        assert_eq!(OobPolicy::Keep.apply(11.0, 0.0, 10.0), Some(11.0));
    }

    #[test]
    fn squish_clamps_to_edges() {
        assert_eq!(OobPolicy::Squish.apply(-1.0, 0.0, 10.0), Some(0.0));
        assert_eq!(OobPolicy::Squish.apply(11.0, 0.0, 10.0), Some(10.0));
    }

    #[test]
    fn in_range_and_boundaries_unchanged() {
        for p in [OobPolicy::Censor, OobPolicy::Keep, OobPolicy::Squish] {
            assert_eq!(p.apply(5.0, 0.0, 10.0), Some(5.0));
            assert_eq!(p.apply(0.0, 0.0, 10.0), Some(0.0));
            assert_eq!(p.apply(10.0, 0.0, 10.0), Some(10.0));
        }
    }

    #[test]
    fn nan_passes_through() {
        for p in [OobPolicy::Censor, OobPolicy::Keep, OobPolicy::Squish] {
            assert!(p.apply(f64::NAN, 0.0, 10.0).unwrap().is_nan());
        }
    }

    #[test]
    fn names_parse_and_round_trip() {
        assert_eq!(OobPolicy::from_name("CENSOR"), Some(OobPolicy::Censor));
        assert_eq!(OobPolicy::from_name("Squish"), Some(OobPolicy::Squish));
        assert_eq!(OobPolicy::from_name("none"), Some(OobPolicy::Keep));
        assert_eq!(OobPolicy::from_name("bogus"), None);
        for p in [OobPolicy::Censor, OobPolicy::Keep, OobPolicy::Squish] {
            assert_eq!(OobPolicy::from_name(p.name()), Some(p));
        }
        assert_eq!(OobPolicy::default(), OobPolicy::Censor);
    }
}
