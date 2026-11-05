//! Built-in shape definitions as kurbo path data with rendering metadata.
//!
//! A shape scale maps domain values to shape *names* — this module is the
//! shared vocabulary renderers resolve those names against. Each [`Shape`]
//! bundles everything a renderer needs in one definition: kurbo
//! [`BezPath`] geometry in the unit reference frame (roughly `[-1, 1]`,
//! y-down), a fill/stroke [`ShapeStyle`], and an anchor point, so a
//! consumer registers or serializes it without any additional matching.
//!
//! Renderers (e.g. hephaestus) consume [`Shape::paths`] directly; a
//! backend that needs a serialized form (e.g. SVG path data) gets it
//! from kurbo (`BezPath::to_svg`) without zygos mediating.
//!
//! Geometry and metadata are ported from hephaestus's `shape_builtin`;
//! this module is the canonical definition both consumers share.

use std::f64::consts::{FRAC_PI_2, PI, SQRT_2};

use peniko::kurbo::{BezPath, Point};

/// Outer radius of the built-in `circle` in shape-local coordinates.
/// Every other closed point shape is area-matched to a circle of this
/// radius. Chrome that mirrors a marker (e.g. a legend key) references
/// this constant so legend and panel markers can't drift.
pub const REFERENCE_RADIUS: f64 = 0.8;

/// How a shape's paths are painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeStyle {
    /// Fill the path interiors.
    Fill,
    /// Stroke the path outlines.
    Stroke,
}

/// A named shape: geometry plus the metadata a renderer needs to draw it.
#[derive(Debug, Clone)]
pub struct Shape {
    name: &'static str,
    paths: Vec<BezPath>,
    style: ShapeStyle,
    anchor: Point,
}

impl Shape {
    fn new(name: &'static str, paths: Vec<BezPath>, style: ShapeStyle, anchor: Point) -> Self {
        Shape {
            name,
            paths,
            style,
            anchor,
        }
    }

    /// The name the shape is registered under (see [`NAMES`]).
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The shape's geometry in the unit reference frame.
    pub fn paths(&self) -> &[BezPath] {
        &self.paths
    }

    /// Whether the paths are filled or stroked.
    pub fn style(&self) -> ShapeStyle {
        self.style
    }

    /// The anchor point in shape-local coordinates — the attachment
    /// point line-ending terminators align to, and the reference point
    /// for legend key placement.
    pub fn anchor(&self) -> Point {
        self.anchor
    }
}

/// Canonical names of every built-in shape, in registration order.
pub const NAMES: &[&str] = &[
    // Point shapes
    "circle",
    "square",
    "diamond",
    "triangle-up",
    "triangle-down",
    "star",
    "cross",
    "plus",
    "asterisk",
    "bowtie",
    "hline",
    "vline",
    "square-cross",
    "circle-plus",
    "square-plus",
    // Pointed arrowheads (tip at origin, body in -x)
    "arrow-open",
    "arrow-closed",
    "arrow-stealth",
    "arrow-latex",
    "arrow-thin",
    "arrow-wedge",
    // Tail-style
    "arrow-fishtail",
    "arrow-fork",
    "arrow-feather",
    // Symmetric terminators
    "arrow-bar",
    "arrow-bracket",
    "arrow-cross",
    "arrow-dot",
    "arrow-square",
    "arrow-diamond",
];

/// The point shapes (as opposed to line-ending terminators) — the
/// subset of [`NAMES`] a point glyph channel cycles through, in
/// canonical registration order.
pub const POINT_SHAPE_NAMES: &[&str] = &[
    "circle",
    "square",
    "diamond",
    "triangle-up",
    "triangle-down",
    "star",
    "cross",
    "plus",
    "asterisk",
    "bowtie",
    "hline",
    "vline",
    "square-cross",
    "circle-plus",
    "square-plus",
];

/// Construct the built-in shape registered under `name`
/// (case-insensitive). Returns `None` for unknown names.
pub fn builtin_shape(name: &str) -> Option<Shape> {
    Some(match name.to_lowercase().as_str() {
        "circle" => circle(),
        "square" => square(),
        "diamond" => diamond(),
        "triangle-up" => triangle_up(),
        "triangle-down" => triangle_down(),
        "star" => star(),
        "cross" => cross(),
        "plus" => plus(),
        "asterisk" => asterisk(),
        "bowtie" => bowtie(),
        "hline" => hline(),
        "vline" => vline(),
        "square-cross" => square_cross(),
        "circle-plus" => circle_plus(),
        "square-plus" => square_plus(),
        "arrow-open" => arrow_open(),
        "arrow-closed" => arrow_closed(),
        "arrow-stealth" => arrow_stealth(),
        "arrow-latex" => arrow_latex(),
        "arrow-thin" => arrow_thin(),
        "arrow-wedge" => arrow_wedge(),
        "arrow-fishtail" => arrow_fishtail(),
        "arrow-fork" => arrow_fork(),
        "arrow-feather" => arrow_feather(),
        "arrow-bar" => arrow_bar(),
        "arrow-bracket" => arrow_bracket(),
        "arrow-cross" => arrow_cross(),
        "arrow-dot" => arrow_dot(),
        "arrow-square" => arrow_square(),
        "arrow-diamond" => arrow_diamond(),
        _ => return None,
    })
}

// ─── Path helpers ────────────────────────────────────────────────────

fn polygon(points: &[(f64, f64)]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(points[0]);
    for &p in &points[1..] {
        path.line_to(p);
    }
    path.close_path();
    path
}

fn polyline(points: &[(f64, f64)]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(points[0]);
    for &p in &points[1..] {
        path.line_to(p);
    }
    path
}

fn segment(a: (f64, f64), b: (f64, f64)) -> BezPath {
    polyline(&[a, b])
}

/// Circle as four cubic Bézier segments (the standard κ approximation,
/// max radial error ~0.03% of r — visually indistinguishable from a
/// true arc and far cleaner than a polygon).
fn circle_path(r: f64) -> BezPath {
    let k = r * 0.552_284_749_830_793_6; // (4/3)·(√2 − 1)·r
    let mut path = BezPath::new();
    path.move_to((r, 0.0));
    path.curve_to((r, k), (k, r), (0.0, r));
    path.curve_to((-k, r), (-r, k), (-r, 0.0));
    path.curve_to((-r, -k), (-k, -r), (0.0, -r));
    path.curve_to((k, -r), (r, -k), (r, 0.0));
    path.close_path();
    path
}

fn fill_one(name: &'static str, path: BezPath, anchor: Point) -> Shape {
    Shape::new(name, vec![path], ShapeStyle::Fill, anchor)
}

fn fill_many(name: &'static str, paths: Vec<BezPath>, anchor: Point) -> Shape {
    Shape::new(name, paths, ShapeStyle::Fill, anchor)
}

fn stroke(name: &'static str, paths: Vec<BezPath>, anchor: Point) -> Shape {
    Shape::new(name, paths, ShapeStyle::Stroke, anchor)
}

// ─── Point shapes ────────────────────────────────────────────────────

/// Bézier circle at [`REFERENCE_RADIUS`]. Area ≈ 2.01 — reference for
/// area-equalization of the other closed point shapes.
fn circle() -> Shape {
    fill_one(
        "circle",
        circle_path(REFERENCE_RADIUS),
        Point::new(-REFERENCE_RADIUS, 0.0),
    )
}

/// Square, half-side 0.71. Area-matched to the circle.
fn square() -> Shape {
    let s = 0.71;
    fill_one(
        "square",
        polygon(&[(-s, -s), (s, -s), (s, s), (-s, s)]),
        Point::new(-s, 0.0),
    )
}

/// Diamond (square rotated 45°), half-diagonal 0.89. Area-matched.
fn diamond() -> Shape {
    let d = 0.89;
    fill_one(
        "diamond",
        polygon(&[(0.0, -d), (d, 0.0), (0.0, d), (-d, 0.0)]),
        Point::new(-d, 0.0),
    )
}

/// Triangle pointing up, circumradius 0.92.
fn triangle_up() -> Shape {
    let r = 0.92;
    let h = r * 0.75;
    fill_one(
        "triangle-up",
        polygon(&[(0.0, -r), (r, h), (-r, h)]),
        Point::new(-r, 0.0),
    )
}

/// Triangle pointing down, circumradius 0.92.
fn triangle_down() -> Shape {
    let r = 0.92;
    let h = r * 0.75;
    fill_one(
        "triangle-down",
        polygon(&[(-r, -h), (r, -h), (0.0, r)]),
        Point::new(-r, 0.0),
    )
}

/// 5-point star, outer radius 0.95, inner 0.38.
fn star() -> Shape {
    let outer = 0.95;
    let inner = outer * 0.4;
    let mut pts = Vec::with_capacity(10);
    for i in 0..10 {
        let angle = -FRAC_PI_2 + PI * (i as f64) / 5.0;
        let r = if i % 2 == 0 { outer } else { inner };
        pts.push((r * angle.cos(), r * angle.sin()));
    }
    fill_one("star", polygon(&pts), Point::new(-outer, 0.0))
}

/// X — two diagonal strokes through the origin.
fn cross() -> Shape {
    let c = 0.8 / SQRT_2;
    stroke(
        "cross",
        vec![segment((-c, -c), (c, c)), segment((-c, c), (c, -c))],
        Point::new(-c, 0.0),
    )
}

/// + — two axis-aligned strokes through the origin.
fn plus() -> Shape {
    stroke(
        "plus",
        vec![
            segment((-0.8, 0.0), (0.8, 0.0)),
            segment((0.0, -0.8), (0.0, 0.8)),
        ],
        Point::new(-0.8, 0.0),
    )
}

/// Asterisk — three line segments at 60° increments.
fn asterisk() -> Shape {
    let r: f64 = 0.8;
    let paths = (0..3)
        .map(|i| {
            let angle = (i as f64) * PI / 3.0;
            let (sin, cos) = angle.sin_cos();
            segment((-r * cos, -r * sin), (r * cos, r * sin))
        })
        .collect();
    stroke("asterisk", paths, Point::new(-r, 0.0))
}

/// Two triangles meeting at the origin (left and right).
fn bowtie() -> Shape {
    fill_many(
        "bowtie",
        vec![
            polygon(&[(-0.8, -0.8), (0.0, 0.0), (-0.8, 0.8)]),
            polygon(&[(0.8, -0.8), (0.0, 0.0), (0.8, 0.8)]),
        ],
        Point::new(-0.8, 0.0),
    )
}

/// Horizontal line segment from `(-0.8, 0)` to `(0.8, 0)`.
fn hline() -> Shape {
    stroke(
        "hline",
        vec![segment((-0.8, 0.0), (0.8, 0.0))],
        Point::new(-0.8, 0.0),
    )
}

/// Vertical line segment from `(0, -0.8)` to `(0, 0.8)`.
fn vline() -> Shape {
    stroke(
        "vline",
        vec![segment((0.0, -0.8), (0.0, 0.8))],
        Point::ORIGIN,
    )
}

/// Square divided into 4 triangles pointing inward (composite — 4 subpaths).
fn square_cross() -> Shape {
    let s = 0.71;
    let g = 0.12;
    fill_many(
        "square-cross",
        vec![
            polygon(&[(-s + g, -s), (s - g, -s), (0.0, -g)]),
            polygon(&[(s, -s + g), (s, s - g), (g, 0.0)]),
            polygon(&[(s - g, s), (-s + g, s), (0.0, g)]),
            polygon(&[(-s, s - g), (-s, -s + g), (-g, 0.0)]),
        ],
        Point::new(-s, 0.0),
    )
}

/// Circle divided into 4 quarter pieces by a `+`-shaped gap (composite — 4 subpaths).
fn circle_plus() -> Shape {
    let r: f64 = 0.8;
    let g: f64 = 0.12 / SQRT_2;
    let n = 8;
    let edge = (r * r - g * g).sqrt();
    let start_angle = (g / r).asin();
    let end_angle = FRAC_PI_2 - start_angle;
    let mut paths = Vec::with_capacity(4);
    for q in 0..4 {
        let base_angle = (q as f64) * FRAC_PI_2;
        let mut pts: Vec<(f64, f64)> = Vec::new();
        pts.push(match q {
            0 => (g, g),
            1 => (-g, g),
            2 => (-g, -g),
            _ => (g, -g),
        });
        pts.push(match q {
            0 => (edge, g),
            1 => (-g, edge),
            2 => (-edge, -g),
            _ => (g, -edge),
        });
        let arc_start = base_angle + start_angle;
        let arc_span = end_angle - start_angle;
        for i in 0..=n {
            let t = (i as f64) / (n as f64);
            let angle = arc_start + t * arc_span;
            pts.push((r * angle.cos(), r * angle.sin()));
        }
        pts.push(match q {
            0 => (g, edge),
            1 => (-edge, g),
            2 => (-g, -edge),
            _ => (edge, -g),
        });
        paths.push(polygon(&pts));
    }
    fill_many("circle-plus", paths, Point::new(-r, 0.0))
}

/// Square divided into 4 corner squares by a `+`-shaped gap (composite — 4 subpaths).
fn square_plus() -> Shape {
    let s = 0.71;
    let g = 0.12 / SQRT_2;
    fill_many(
        "square-plus",
        vec![
            polygon(&[(-s, -s), (-g, -s), (-g, -g), (-s, -g)]),
            polygon(&[(g, -s), (s, -s), (s, -g), (g, -g)]),
            polygon(&[(g, g), (s, g), (s, s), (g, s)]),
            polygon(&[(-s, g), (-g, g), (-g, s), (-s, s)]),
        ],
        Point::new(-s, 0.0),
    )
}

// ─── Pointed arrowheads (tip at origin, body in -x) ──────────────────

/// Open V: two strokes meeting at the tip. Anchor at the tip.
fn arrow_open() -> Shape {
    stroke(
        "arrow-open",
        vec![
            segment((-1.0, 0.5), (0.0, 0.0)),
            segment((0.0, 0.0), (-1.0, -0.5)),
        ],
        Point::ORIGIN,
    )
}

/// Filled isosceles triangle. Anchor at the back of the body.
fn arrow_closed() -> Shape {
    fill_one(
        "arrow-closed",
        polygon(&[(0.0, 0.0), (-1.0, 0.5), (-1.0, -0.5)]),
        Point::new(-1.0, 0.0),
    )
}

/// TikZ-style stealth: concave-back filled triangle. Anchor at notch apex.
fn arrow_stealth() -> Shape {
    fill_one(
        "arrow-stealth",
        polygon(&[(0.0, 0.0), (-1.0, 0.5), (-0.4, 0.0), (-1.0, -0.5)]),
        Point::new(-0.4, 0.0),
    )
}

/// LaTeX `\to`-style: slightly concave-back filled triangle.
fn arrow_latex() -> Shape {
    fill_one(
        "arrow-latex",
        polygon(&[(0.0, 0.0), (-1.0, 0.35), (-0.6, 0.0), (-1.0, -0.35)]),
        Point::new(-0.6, 0.0),
    )
}

/// Narrow filled triangle (~5:1 aspect).
fn arrow_thin() -> Shape {
    fill_one(
        "arrow-thin",
        polygon(&[(0.0, 0.0), (-1.0, 0.2), (-1.0, -0.2)]),
        Point::new(-1.0, 0.0),
    )
}

/// Asymmetric barb / half-arrow (top half only).
fn arrow_wedge() -> Shape {
    fill_one(
        "arrow-wedge",
        polygon(&[(0.0, 0.0), (-1.0, 0.5), (-1.0, 0.0)]),
        Point::new(-1.0, 0.0),
    )
}

// ─── Tail-style (open, opens away from line) ─────────────────────────

/// Two strokes opening outward — classic fletching/tail look.
fn arrow_fishtail() -> Shape {
    stroke(
        "arrow-fishtail",
        vec![
            segment((0.0, 0.0), (-1.0, 0.5)),
            segment((0.0, 0.0), (-1.0, -0.5)),
        ],
        Point::ORIGIN,
    )
}

/// Wider-angle Y.
fn arrow_fork() -> Shape {
    stroke(
        "arrow-fork",
        vec![
            segment((0.0, 0.0), (-0.7, 0.7)),
            segment((0.0, 0.0), (-0.7, -0.7)),
        ],
        Point::ORIGIN,
    )
}

/// Stylised fletching — three chevrons along the shaft (6 subpaths).
fn arrow_feather() -> Shape {
    let arm = 0.4;
    let halfh = 0.5;
    let offsets = [0.0, -0.3, -0.6];
    let mut paths = Vec::with_capacity(6);
    for &ox in &offsets {
        paths.push(segment((ox, 0.0), (ox - arm, halfh)));
        paths.push(segment((ox, 0.0), (ox - arm, -halfh)));
    }
    stroke("arrow-feather", paths, Point::ORIGIN)
}

// ─── Symmetric terminators ───────────────────────────────────────────

/// Perpendicular bar.
fn arrow_bar() -> Shape {
    stroke(
        "arrow-bar",
        vec![segment((0.0, -0.5), (0.0, 0.5))],
        Point::ORIGIN,
    )
}

/// Bar with two right-angle returns (`[`-shape).
fn arrow_bracket() -> Shape {
    stroke(
        "arrow-bracket",
        vec![polyline(&[
            (0.2, -0.5),
            (0.0, -0.5),
            (0.0, 0.5),
            (0.2, 0.5),
        ])],
        Point::ORIGIN,
    )
}

/// Perpendicular X.
fn arrow_cross() -> Shape {
    stroke(
        "arrow-cross",
        vec![
            segment((-0.5, -0.5), (0.5, 0.5)),
            segment((-0.5, 0.5), (0.5, -0.5)),
        ],
        Point::ORIGIN,
    )
}

/// Small filled circle terminator.
fn arrow_dot() -> Shape {
    fill_one("arrow-dot", circle_path(0.3), Point::ORIGIN)
}

/// Small filled square terminator.
fn arrow_square() -> Shape {
    fill_one(
        "arrow-square",
        polygon(&[(-0.3, -0.3), (0.3, -0.3), (0.3, 0.3), (-0.3, 0.3)]),
        Point::ORIGIN,
    )
}

/// Small filled diamond terminator.
fn arrow_diamond() -> Shape {
    fill_one(
        "arrow-diamond",
        polygon(&[(0.0, -0.4), (0.4, 0.0), (0.0, 0.4), (-0.4, 0.0)]),
        Point::ORIGIN,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use peniko::kurbo::Shape as KurboShape;

    #[test]
    fn every_name_resolves_round_trip() {
        for name in NAMES {
            let shape = builtin_shape(name).unwrap_or_else(|| panic!("{name} should resolve"));
            assert_eq!(shape.name(), *name, "registered name should round-trip");
            assert!(!shape.paths().is_empty(), "{name} should have geometry");
        }
    }

    #[test]
    fn lookup_is_case_insensitive_and_rejects_unknown() {
        assert!(builtin_shape("CIRCLE").is_some());
        assert!(builtin_shape("Arrow-Stealth").is_some());
        assert!(builtin_shape("nope").is_none());
    }

    #[test]
    fn geometry_stays_in_unit_frame() {
        for name in NAMES {
            for path in builtin_shape(name).unwrap().paths() {
                let bb = KurboShape::bounding_box(path);
                assert!(
                    bb.min_x() >= -1.01
                        && bb.max_x() <= 1.01
                        && bb.min_y() >= -1.01
                        && bb.max_y() <= 1.01,
                    "{name} escapes the unit frame: {bb:?}"
                );
            }
        }
    }

    #[test]
    fn point_shapes_are_first_fifteen() {
        assert_eq!(POINT_SHAPE_NAMES.len(), 15);
        assert_eq!(POINT_SHAPE_NAMES[0], "circle");
        assert_eq!(POINT_SHAPE_NAMES[14], "square-plus");
    }

    #[test]
    fn styles_match_hephaestus() {
        // Closed point shapes fill; line shapes stroke.
        for name in [
            "circle",
            "square",
            "diamond",
            "triangle-up",
            "triangle-down",
            "star",
            "bowtie",
            "square-cross",
            "circle-plus",
            "square-plus",
        ] {
            assert_eq!(
                builtin_shape(name).unwrap().style(),
                ShapeStyle::Fill,
                "{name}"
            );
        }
        for name in ["cross", "plus", "asterisk", "hline", "vline"] {
            assert_eq!(
                builtin_shape(name).unwrap().style(),
                ShapeStyle::Stroke,
                "{name}"
            );
        }
    }

    #[test]
    fn anchors_match_hephaestus() {
        assert_eq!(
            builtin_shape("circle").unwrap().anchor(),
            Point::new(-0.8, 0.0)
        );
        assert_eq!(builtin_shape("vline").unwrap().anchor(), Point::ORIGIN);
        assert_eq!(
            builtin_shape("arrow-closed").unwrap().anchor(),
            Point::new(-1.0, 0.0)
        );
        assert_eq!(
            builtin_shape("arrow-stealth").unwrap().anchor(),
            Point::new(-0.4, 0.0)
        );
    }

    #[test]
    fn composite_shapes_have_multiple_subpaths() {
        assert_eq!(builtin_shape("square-cross").unwrap().paths().len(), 4);
        assert_eq!(builtin_shape("circle-plus").unwrap().paths().len(), 4);
        assert_eq!(builtin_shape("square-plus").unwrap().paths().len(), 4);
        assert_eq!(builtin_shape("arrow-feather").unwrap().paths().len(), 6);
    }

    #[test]
    fn circle_is_smooth_bezier_not_polygon() {
        let circle = builtin_shape("circle").unwrap();
        let els: Vec<_> = circle.paths()[0].elements().to_vec();
        assert_eq!(els.len(), 6, "move + 4 cubics + close");
        assert!(
            els.iter()
                .any(|el| matches!(el, peniko::kurbo::PathEl::CurveTo(..))),
            "circle should contain curve segments, not a polygon"
        );
        // Radius is preserved: the path's bbox spans ±REFERENCE_RADIUS.
        let bb = KurboShape::bounding_box(&circle.paths()[0]);
        assert!((bb.width() - 2.0 * REFERENCE_RADIUS).abs() < 1e-9);
    }

    #[test]
    fn closed_shapes_close_their_paths() {
        use peniko::kurbo::PathEl;
        let last = |name: &str| {
            let shape = builtin_shape(name).unwrap();
            shape.paths()[0].elements().last().copied().unwrap()
        };
        assert!(matches!(last("square"), PathEl::ClosePath));
        assert!(!matches!(last("hline"), PathEl::ClosePath));
    }
}
