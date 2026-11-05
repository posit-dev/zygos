//! Shape palettes: curated name lists for point glyph channels.
//! The shapes these names resolve to are defined in
//! [`crate::patterns::shape`].

/// Closed shapes - shapes with enclosed area/fill
pub const SHAPES_CLOSED: &[&str] = &[
    "circle",
    "square",
    "diamond",
    "triangle-up",
    "triangle-down",
    "star",
    "square-cross",
    "circle-plus",
    "square-plus",
];

/// Stroke shapes - shapes made of strokes/lines (no enclosed area)
pub const SHAPES_OPEN: &[&str] = &[
    "cross",    // X shape
    "plus",     // + shape
    "asterisk", // * shape (6-pointed)
    "bowtie",   // two triangles pointing inward
    "hline",    // horizontal line
    "vline",    // vertical line
];

/// Default point shapes - combined palette ordered by distinguishability
/// (closed shapes first, then open).
pub const SHAPES: &[&str] = &[
    // closed shapes first
    "circle",
    "square",
    "diamond",
    "triangle-up",
    "triangle-down",
    "star",
    "square-cross",
    "circle-plus",
    "square-plus",
    // Open shapes
    "cross",
    "plus",
    "asterisk",
    "bowtie",
    "hline",
    "vline",
];

/// Look up a shape palette by name.
pub fn get_shape_palette(name: &str) -> Option<&'static [&'static str]> {
    match name.to_lowercase().as_str() {
        "shapes" => Some(SHAPES),
        "shapes_closed" | "closed" | "default" => Some(SHAPES_CLOSED),
        "shapes_open" | "open" => Some(SHAPES_OPEN),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patterns::shape::{builtin_shape, POINT_SHAPE_NAMES};

    #[test]
    fn test_get_shape_palette() {
        assert_eq!(get_shape_palette("shapes").unwrap().len(), 15);
        assert_eq!(get_shape_palette("default").unwrap().len(), 9);
        assert_eq!(get_shape_palette("open").unwrap().len(), 6);
        assert!(get_shape_palette("SHAPES").is_some());
        assert!(get_shape_palette("unknown").is_none());
    }

    #[test]
    fn test_shape_palette_names_have_definitions() {
        // Every palette entry resolves to a shape definition, and the
        // point-shape registry covers exactly the same set (order may
        // differ — the registry lists point shapes in canonical order).
        for name in SHAPES {
            assert!(
                builtin_shape(name).is_some(),
                "palette shape '{name}' has no definition"
            );
            assert!(
                POINT_SHAPE_NAMES.contains(name),
                "'{name}' missing from POINT_SHAPE_NAMES"
            );
        }
        assert_eq!(SHAPES.len(), POINT_SHAPE_NAMES.len());
    }
}
