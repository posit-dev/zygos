//! Declarative vector fill patterns: a tile of elements repeated across a
//! fill region.
//!
//! A [`FillPattern`] is pure vocabulary, like [`patterns::shape`](crate::patterns::shape):
//! zygos defines the tile, the consumer resolves shape names, shapes text,
//! and performs the tiling. All sizes are unitless multiples of
//! [`FillPattern::tile_rect`] height — the same convention
//! [`LinetypeStep`](crate::LinetypeStep) uses for line widths — so one
//! pattern scales as a coherent unit across contexts, and a resize never
//! changes the relative weight of text versus shapes.
//!
//! Patterns flow through channels as [`Brush::Pattern`](crate::Brush)
//! inside [`Value::Brush`](crate::Value::Brush), alongside solid, gradient
//! and image fills. Rendering — bbox coverage, clip + replay per tile, and
//! per-backend emission — lives in the consumer.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use peniko::kurbo::{Affine, BezPath, PathEl, Point, Rect, Stroke, Vec2};

use crate::brush::Brush;
use crate::value::canonical_f64_bits;

/// A vector fill pattern: a tile of declarative elements repeated across
/// a fill region.
///
/// The tiling is fully determined by `tile_rect`, `spacing` and
/// `transform` — like a gradient's `GradientKind` coordinates, the value
/// is self-contained about where it lands; the consumer needs no external
/// placement state. Elements may overflow the tile; the consumer clips to
/// `tile_rect` (grid's pattern-viewport model).
#[derive(Clone, Debug)]
pub struct FillPattern {
    /// The tile's contents, in tile coordinates.
    pub elements: Vec<PatternElement>,
    /// The tile bounds, in the pattern's own coordinate space. Elements
    /// outside it are clipped.
    pub tile_rect: Rect,
    /// Tile pitch. Smaller than `tile_rect` overlaps tiles; larger leaves
    /// gaps. Both components must be positive and finite.
    pub spacing: Vec2,
    /// Rotate / shear / offset the whole tiling in user space.
    pub transform: Affine,
}

/// What a pattern element is painted with.
///
/// The same indirection [`patterns::shape`](crate::patterns::shape)'s
/// `ShapeStyle` plays: zygos carries the *role*, the consumer binds the
/// paint. Roles are bound before the scene boundary — backends only ever
/// see fully-bound patterns and never learn roles exist.
#[derive(Clone, Debug)]
pub enum PatternPaint {
    /// The primary paint the consumer supplies — typically its fill
    /// channel's resolved brush. The natural default for hand-written
    /// elements.
    Foreground,
    /// The secondary paint the consumer supplies, conventionally a
    /// backdrop behind the foreground strokes; consumers commonly
    /// default it to transparent.
    Background,
    /// Paint fixed by the pattern itself, for genuinely colored tiles
    /// (a flag stamp, a logo) that must keep per-element paint.
    Fixed(Brush),
}

impl From<Brush> for PatternPaint {
    fn from(b: Brush) -> Self {
        PatternPaint::Fixed(b)
    }
}

/// One drawable element of a pattern tile. All sizes are unitless
/// multiples of the tile height — `size: 0.8` means 80% of
/// [`FillPattern::tile_rect`] height, so text legibility is the tile
/// author's choice, made indirectly through the tile size. Consumers
/// wanting an absolute pt size compute the fraction themselves (a 9pt
/// stamp in an 18pt tile is `size: 0.5`).
///
/// Paint is a [`PatternPaint`] role; a `Background` element is legal on
/// any variant (e.g. a shape stamped in the backdrop color).
#[derive(Clone, Debug)]
pub enum PatternElement {
    /// Filled and/or stroked path geometry, like `patterns::shape`.
    Path {
        /// The geometry, in tile coordinates.
        path: BezPath,
        /// Fill paint, if any (`None` = not filled, orthogonal to *how*
        /// it is painted).
        fill: Option<PatternPaint>,
        /// Stroke style and paint, if any. Boxed to keep the enum small
        /// (`Stroke` + `Brush` outweigh the other variants combined).
        stroke: Option<Box<(Stroke, PatternPaint)>>,
    },
    /// A named shape stamped at a point — the same indirection as
    /// `LinetypeStep::Marker`; the consumer resolves the name against
    /// `patterns::shape` at draw time.
    Shape {
        /// Shape name, resolved by the consumer.
        name: Arc<str>,
        /// Anchor point in tile coordinates.
        at: Point,
        /// Shape size, as a multiple of tile height.
        size: f64,
        /// Paint for the shape.
        paint: PatternPaint,
    },
    /// A text stamp: the string and its style *names*, not shaped glyphs.
    /// The consumer shapes; zygos never measures anything.
    Text {
        /// The text to stamp.
        content: Arc<str>,
        /// Baseline origin in tile coordinates.
        at: Point,
        /// Font size, as a multiple of tile height.
        size: f64,
        /// Font family name; `None` = consumer default.
        family: Option<Arc<str>>,
        /// Paint for the glyphs.
        paint: PatternPaint,
    },
}

impl FillPattern {
    /// A pattern with no tiling transform (`Affine::IDENTITY`).
    pub fn new(elements: Vec<PatternElement>, tile_rect: Rect, spacing: Vec2) -> Self {
        debug_assert!(
            spacing.x.is_finite() && spacing.y.is_finite() && spacing.x > 0.0 && spacing.y > 0.0,
            "FillPattern spacing must be positive and finite"
        );
        Self {
            elements,
            tile_rect,
            spacing,
            transform: Affine::IDENTITY,
        }
    }

    /// Builder: set the tiling transform.
    #[must_use]
    pub fn with_transform(mut self, transform: Affine) -> Self {
        self.transform = transform;
        self
    }
}

// ─── Named built-ins ─────────────────────────────────────────────────────────
//
// A small shared nomenclature — the same role `linetype_pattern` plays for
// dash patterns. The definitions carry geometry only; the caller supplies
// the paint, so a name never bakes in a color. There is deliberately no
// pattern scale yet: these exist so consumers resolve the same names to
// the same tilings.

/// Resolve a named fill pattern, painted with `paint`.
///
/// Names (case-insensitive):
/// - `"stripe"` / `"stripe-dense"` / `"stripe-sparse"` — diagonal lines at
///   45°, at one line per tile / half-tile / two tiles.
/// - `"crosshatch"` — diagonal lines in both directions.
/// - `"circle"` — a circle per tile (resolved by the consumer against
///   [`patterns::shape`](crate::patterns::shape)).
/// - `"dot"` — a small circle per tile.
///
/// Every element carries the one supplied [`PatternPaint`]: pass a
/// `Brush` (wrapped in [`PatternPaint::Fixed`] via `Into`) for a
/// self-colored definition, or `PatternPaint::Foreground` /
/// `Background` for a consumer-colored one. Line patterns stroke with
/// it; shape patterns fill with it. All geometry lives in a unit tile
/// (`Rect::new(0., 0., 1., 1.)`), with line widths and shape sizes as
/// multiples of tile height per the module convention. Returns `None`
/// for unknown names.
pub fn fill_pattern(name: &str, paint: impl Into<PatternPaint>) -> Option<FillPattern> {
    let paint = paint.into();
    fn line(
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        width: f64,
        paint: &PatternPaint,
    ) -> PatternElement {
        let mut path = BezPath::new();
        path.move_to((x0, y0));
        path.line_to((x1, y1));
        PatternElement::Path {
            path,
            fill: None,
            stroke: Some(Box::new((Stroke::new(width), paint.clone()))),
        }
    }
    // Stripes are horizontal lines; the 45° rotation rides on the
    // pattern's transform, so the tiling stays axis-aligned and seamless
    // (the line sits on the tile edge shared with the next row).
    let stripe = |spacing: f64, paint: &PatternPaint| {
        FillPattern::new(
            vec![line(0.0, 0.0, 1.0, 0.0, 0.125, paint)],
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Vec2::new(spacing, spacing),
        )
        .with_transform(Affine::rotate(std::f64::consts::FRAC_PI_4))
    };
    let shape = |shape_name: &str, size: f64, paint: &PatternPaint| {
        FillPattern::new(
            vec![PatternElement::Shape {
                name: Arc::from(shape_name),
                at: Point::new(0.5, 0.5),
                size,
                paint: paint.clone(),
            }],
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Vec2::new(1.0, 1.0),
        )
    };
    match name.to_lowercase().as_str() {
        "stripe" => Some(stripe(1.0, &paint)),
        "stripe-dense" => Some(stripe(0.5, &paint)),
        "stripe-sparse" => Some(stripe(2.0, &paint)),
        "crosshatch" => Some(
            FillPattern::new(
                vec![
                    line(0.0, 0.0, 1.0, 0.0, 0.125, &paint),
                    line(0.0, 0.0, 0.0, 1.0, 0.125, &paint),
                ],
                Rect::new(0.0, 0.0, 1.0, 1.0),
                Vec2::new(1.0, 1.0),
            )
            .with_transform(Affine::rotate(std::f64::consts::FRAC_PI_4)),
        ),
        "circle" => Some(shape("circle", 0.5, &paint)),
        "dot" => Some(shape("circle", 0.125, &paint)),
        _ => None,
    }
}

/// Structural equality for [`FillPattern`]s, canonicalising NaN / -0 in
/// every float comparison (rect, spacing, transform coefficients, and
/// element geometry). Two patterns differing only in `transform` are
/// different fills. Mirrors [`pattern_key_hash`].
pub fn pattern_key_eq(a: &FillPattern, b: &FillPattern) -> bool {
    fn f64_eq(x: f64, y: f64) -> bool {
        canonical_f64_bits(x) == canonical_f64_bits(y)
    }
    fn rect_eq(x: &Rect, y: &Rect) -> bool {
        f64_eq(x.x0, y.x0) && f64_eq(x.x1, y.x1) && f64_eq(x.y0, y.y0) && f64_eq(x.y1, y.y1)
    }
    fn vec2_eq(x: &Vec2, y: &Vec2) -> bool {
        f64_eq(x.x, y.x) && f64_eq(x.y, y.y)
    }
    fn affine_eq(x: &Affine, y: &Affine) -> bool {
        let (xc, yc) = (x.as_coeffs(), y.as_coeffs());
        xc.iter().zip(yc.iter()).all(|(u, v)| f64_eq(*u, *v))
    }
    rect_eq(&a.tile_rect, &b.tile_rect)
        && vec2_eq(&a.spacing, &b.spacing)
        && affine_eq(&a.transform, &b.transform)
        && a.elements.len() == b.elements.len()
        && a.elements
            .iter()
            .zip(b.elements.iter())
            .all(|(x, y)| element_key_eq(x, y))
}

/// Deterministic hash for a [`FillPattern`]. Mirrors [`pattern_key_eq`].
pub fn pattern_key_hash<H: Hasher>(p: &FillPattern, state: &mut H) {
    let r = &p.tile_rect;
    for v in [r.x0, r.x1, r.y0, r.y1] {
        canonical_f64_bits(v).hash(state);
    }
    canonical_f64_bits(p.spacing.x).hash(state);
    canonical_f64_bits(p.spacing.y).hash(state);
    for c in p.transform.as_coeffs() {
        canonical_f64_bits(c).hash(state);
    }
    (p.elements.len() as u64).hash(state);
    for el in &p.elements {
        element_key_hash(el, state);
    }
}

fn element_key_eq(a: &PatternElement, b: &PatternElement) -> bool {
    fn f64_eq(x: f64, y: f64) -> bool {
        canonical_f64_bits(x) == canonical_f64_bits(y)
    }
    fn point_eq(x: &Point, y: &Point) -> bool {
        f64_eq(x.x, y.x) && f64_eq(x.y, y.y)
    }
    fn opt_paint_eq(x: &Option<PatternPaint>, y: &Option<PatternPaint>) -> bool {
        match (x, y) {
            (None, None) => true,
            (Some(u), Some(v)) => paint_key_eq(u, v),
            _ => false,
        }
    }
    fn stroke_eq(x: &Stroke, y: &Stroke) -> bool {
        f64_eq(x.width, y.width)
            && x.join == y.join
            && x.miter_limit == y.miter_limit
            && x.start_cap == y.start_cap
            && x.end_cap == y.end_cap
            && f64_eq(x.dash_offset, y.dash_offset)
            && x.dash_pattern.len() == y.dash_pattern.len()
            && x.dash_pattern
                .iter()
                .zip(y.dash_pattern.iter())
                .all(|(u, v)| f64_eq(*u, *v))
    }
    match (a, b) {
        (
            PatternElement::Path {
                path: pa,
                fill: fa,
                stroke: sa,
            },
            PatternElement::Path {
                path: pb,
                fill: fb,
                stroke: sb,
            },
        ) => {
            path_key_eq(pa, pb)
                && opt_paint_eq(fa, fb)
                && match (sa, sb) {
                    (None, None) => true,
                    (Some(sa), Some(sb)) => stroke_eq(&sa.0, &sb.0) && paint_key_eq(&sa.1, &sb.1),
                    _ => false,
                }
        }
        (
            PatternElement::Shape {
                name: na,
                at: aa,
                size: sa,
                paint: ba,
            },
            PatternElement::Shape {
                name: nb,
                at: ab,
                size: sb,
                paint: bb,
            },
        ) => **na == **nb && point_eq(aa, ab) && f64_eq(*sa, *sb) && paint_key_eq(ba, bb),
        (
            PatternElement::Text {
                content: ca,
                at: aa,
                size: sa,
                family: fa,
                paint: ba,
            },
            PatternElement::Text {
                content: cb,
                at: ab,
                size: sb,
                family: fb,
                paint: bb,
            },
        ) => {
            **ca == **cb && point_eq(aa, ab) && f64_eq(*sa, *sb) && fa == fb && paint_key_eq(ba, bb)
        }
        _ => false,
    }
}

fn element_key_hash<H: Hasher>(el: &PatternElement, state: &mut H) {
    fn point_hash<H2: Hasher>(p: &Point, state: &mut H2) {
        canonical_f64_bits(p.x).hash(state);
        canonical_f64_bits(p.y).hash(state);
    }
    fn stroke_hash<H2: Hasher>(s: &Stroke, state: &mut H2) {
        canonical_f64_bits(s.width).hash(state);
        std::mem::discriminant(&s.join).hash(state);
        std::mem::discriminant(&s.start_cap).hash(state);
        std::mem::discriminant(&s.end_cap).hash(state);
        canonical_f64_bits(s.miter_limit).hash(state);
        canonical_f64_bits(s.dash_offset).hash(state);
        (s.dash_pattern.len() as u64).hash(state);
        for d in s.dash_pattern.iter() {
            canonical_f64_bits(*d).hash(state);
        }
    }
    std::mem::discriminant(el).hash(state);
    match el {
        PatternElement::Path { path, fill, stroke } => {
            path_key_hash(path, state);
            fill.is_some().hash(state);
            if let Some(p) = fill {
                paint_key_hash(p, state);
            }
            stroke.is_some().hash(state);
            if let Some(sb) = stroke {
                stroke_hash(&sb.0, state);
                paint_key_hash(&sb.1, state);
            }
        }
        PatternElement::Shape {
            name,
            at,
            size,
            paint,
        } => {
            (**name).hash(state);
            point_hash(at, state);
            canonical_f64_bits(*size).hash(state);
            paint_key_hash(paint, state);
        }
        PatternElement::Text {
            content,
            at,
            size,
            family,
            paint,
        } => {
            (**content).hash(state);
            point_hash(at, state);
            canonical_f64_bits(*size).hash(state);
            family.hash(state);
            paint_key_hash(paint, state);
        }
    }
}

/// Structural equality for [`PatternPaint`]: roles by discriminant,
/// `Fixed` by [`Brush::key_eq`]. Note a `Foreground` element and a
/// `Fixed` element that a consumer would bind identically still compare
/// unequal — the key describes the *definition*, not the resolution.
fn paint_key_eq(a: &PatternPaint, b: &PatternPaint) -> bool {
    match (a, b) {
        (PatternPaint::Foreground, PatternPaint::Foreground) => true,
        (PatternPaint::Background, PatternPaint::Background) => true,
        (PatternPaint::Fixed(x), PatternPaint::Fixed(y)) => x.key_eq(y),
        _ => false,
    }
}

/// Deterministic hash for [`PatternPaint`]. Mirrors [`paint_key_eq`].
fn paint_key_hash<H: Hasher>(p: &PatternPaint, state: &mut H) {
    std::mem::discriminant(p).hash(state);
    if let PatternPaint::Fixed(b) = p {
        b.key_hash(state);
    }
}

fn path_key_eq(a: &BezPath, b: &BezPath) -> bool {
    fn pt_eq(x: Point, y: Point) -> bool {
        canonical_f64_bits(x.x) == canonical_f64_bits(y.x)
            && canonical_f64_bits(x.y) == canonical_f64_bits(y.y)
    }
    let (ea, eb) = (a.elements(), b.elements());
    ea.len() == eb.len()
        && ea.iter().zip(eb.iter()).all(|(x, y)| match (x, y) {
            (PathEl::MoveTo(p), PathEl::MoveTo(q)) => pt_eq(*p, *q),
            (PathEl::LineTo(p), PathEl::LineTo(q)) => pt_eq(*p, *q),
            (PathEl::QuadTo(p1, p2), PathEl::QuadTo(q1, q2)) => pt_eq(*p1, *q1) && pt_eq(*p2, *q2),
            (PathEl::CurveTo(p1, p2, p3), PathEl::CurveTo(q1, q2, q3)) => {
                pt_eq(*p1, *q1) && pt_eq(*p2, *q2) && pt_eq(*p3, *q3)
            }
            (PathEl::ClosePath, PathEl::ClosePath) => true,
            _ => false,
        })
}

fn path_key_hash<H: Hasher>(path: &BezPath, state: &mut H) {
    fn pt_hash<H2: Hasher>(p: &Point, state: &mut H2) {
        canonical_f64_bits(p.x).hash(state);
        canonical_f64_bits(p.y).hash(state);
    }
    let els = path.elements();
    (els.len() as u64).hash(state);
    for el in els {
        std::mem::discriminant(el).hash(state);
        match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => pt_hash(p, state),
            PathEl::QuadTo(p1, p2) => {
                pt_hash(p1, state);
                pt_hash(p2, state);
            }
            PathEl::CurveTo(p1, p2, p3) => {
                pt_hash(p1, state);
                pt_hash(p2, state);
                pt_hash(p3, state);
            }
            PathEl::ClosePath => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    // ── Named built-ins ──

    #[test]
    fn named_patterns_resolve() {
        let brush = Brush::Solid(Color::BLACK);
        for name in [
            "stripe",
            "stripe-dense",
            "stripe-sparse",
            "crosshatch",
            "circle",
            "dot",
        ] {
            assert!(fill_pattern(name, brush.clone()).is_some(), "{name}");
        }
        assert!(fill_pattern("nonsense", brush.clone()).is_none());
        // Case-insensitive, like linetype_pattern.
        assert!(fill_pattern("STRIPE", brush).is_some());
    }

    #[test]
    fn stripe_density_changes_spacing_not_geometry() {
        let brush = || Brush::Solid(Color::BLACK);
        let base = fill_pattern("stripe", brush()).unwrap();
        let dense = fill_pattern("stripe-dense", brush()).unwrap();
        let sparse = fill_pattern("stripe-sparse", brush()).unwrap();
        assert_eq!(dense.spacing, Vec2::new(0.5, 0.5));
        assert_eq!(sparse.spacing, Vec2::new(2.0, 2.0));
        // Same tile, same transform — only the pitch differs.
        assert_eq!(base.tile_rect, dense.tile_rect);
        assert_eq!(base.transform, dense.transform);
    }

    #[test]
    fn crosshatch_has_two_strokes_and_rotates() {
        let p = fill_pattern("crosshatch", Brush::Solid(Color::BLACK)).unwrap();
        assert_eq!(p.elements.len(), 2);
        assert_ne!(p.transform, Affine::IDENTITY);
        for el in &p.elements {
            assert!(matches!(
                el,
                PatternElement::Path {
                    stroke: Some(_),
                    ..
                }
            ));
        }
    }

    #[test]
    fn dot_is_a_smaller_circle() {
        let brush = || Brush::Solid(Color::BLACK);
        let circle = fill_pattern("circle", brush()).unwrap();
        let dot = fill_pattern("dot", brush()).unwrap();
        let size = |p: &FillPattern| match &p.elements[0] {
            PatternElement::Shape { size, .. } => *size,
            _ => panic!("expected shape"),
        };
        assert!(size(&dot) < size(&circle));
        // Same name resolved either way — the consumer sees one shape.
        assert!(!pattern_key_eq(&circle, &dot));
    }

    #[test]
    fn roles_compare_by_discriminant_not_resolution() {
        let fixed = fill_pattern("stripe", Brush::Solid(Color::BLACK)).unwrap();
        let foreground = fill_pattern("stripe", PatternPaint::Foreground).unwrap();
        let background = fill_pattern("stripe", PatternPaint::Background).unwrap();
        // Same geometry, different paint roles → different definitions,
        // even where a consumer would bind Foreground to black.
        assert!(!pattern_key_eq(&fixed, &foreground));
        assert!(!pattern_key_eq(&foreground, &background));
        assert!(pattern_key_eq(
            &foreground,
            &fill_pattern("stripe", PatternPaint::Foreground).unwrap()
        ));

        // Hash consistency across the role forms.
        use std::collections::hash_map::DefaultHasher;
        let hash = |p: &FillPattern| {
            let mut h = DefaultHasher::new();
            pattern_key_hash(p, &mut h);
            h.finish()
        };
        assert_eq!(
            hash(&foreground),
            hash(&fill_pattern("stripe", PatternPaint::Foreground).unwrap())
        );
        assert_ne!(hash(&fixed), hash(&foreground));
    }

    fn stripe(transform: Affine) -> FillPattern {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((1.0, 1.0));
        FillPattern::new(
            vec![PatternElement::Path {
                path,
                fill: None,
                stroke: Some(Box::new((
                    Stroke::new(0.1),
                    PatternPaint::Fixed(Brush::Solid(Color::BLACK)),
                ))),
            }],
            Rect::new(0.0, 0.0, 1.0, 1.0),
            Vec2::new(1.0, 1.0),
        )
        .with_transform(transform)
    }

    #[test]
    fn pattern_key_eq_identity_and_structure() {
        let a = stripe(Affine::IDENTITY);
        let b = stripe(Affine::IDENTITY);
        assert!(pattern_key_eq(&a, &b));

        // Transform participates in the key.
        let rotated = stripe(Affine::rotate(0.5));
        assert!(!pattern_key_eq(&a, &rotated));
    }

    #[test]
    fn pattern_key_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        let hash = |p: &FillPattern| {
            let mut h = DefaultHasher::new();
            pattern_key_hash(p, &mut h);
            h.finish()
        };
        assert_eq!(
            hash(&stripe(Affine::IDENTITY)),
            hash(&stripe(Affine::IDENTITY))
        );
        assert_ne!(
            hash(&stripe(Affine::IDENTITY)),
            hash(&stripe(Affine::rotate(0.5)))
        );
    }

    #[test]
    fn text_and_shape_elements_compare() {
        let text = |s: f64| PatternElement::Text {
            content: Arc::from("•"),
            at: Point::new(0.5, 0.5),
            size: s,
            family: None,
            paint: PatternPaint::Fixed(Brush::Solid(Color::BLACK)),
        };
        assert!(element_key_eq(&text(0.8), &text(0.8)));
        assert!(!element_key_eq(&text(0.8), &text(0.5)));

        let shape = |n: &str| PatternElement::Shape {
            name: Arc::from(n),
            at: Point::new(0.0, 0.0),
            size: 0.5,
            paint: PatternPaint::Fixed(Brush::Solid(Color::BLACK)),
        };
        assert!(element_key_eq(&shape("circle"), &shape("circle")));
        assert!(!element_key_eq(&shape("circle"), &shape("square")));
    }

    #[test]
    fn nan_canonicalises_in_geometry() {
        let p = |x: f64| {
            FillPattern::new(
                vec![PatternElement::Shape {
                    name: Arc::from("circle"),
                    at: Point::new(x, 0.0),
                    size: 0.5,
                    paint: PatternPaint::Fixed(Brush::Solid(Color::BLACK)),
                }],
                Rect::new(0.0, 0.0, 1.0, 1.0),
                Vec2::new(1.0, 1.0),
            )
        };
        assert!(pattern_key_eq(&p(f64::NAN), &p(f64::NAN)));
        assert!(pattern_key_eq(&p(-0.0), &p(0.0)));
    }
}
