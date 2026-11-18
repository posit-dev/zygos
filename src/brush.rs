//! Brush vocabulary — shared paint types for fill channels.
//!
//! A *brush* is anything a renderer can paint a shape with: a solid
//! color, a gradient, an image pattern, or a vector fill pattern. Zygos
//! defines its own [`Brush`] enum rather than re-exporting peniko's,
//! which is closed and has no pattern variant. The first three variants
//! wrap peniko's types unchanged, so conversion back for scene APIs is a
//! mechanical match via [`Brush::as_peniko`]; `Pattern` never converts —
//! the consumer renders it through its own tiling machinery.
//!
//! Brushes flow through zygos as [`Value::Brush`](crate::Value::Brush)
//! and [`DataColumn::Brush`](crate::DataColumn::Brush). They are opaque
//! to scales: constant channels carry them (the per-category raw fill
//! use case); a data-driven fill scale with gradient interpolation is a
//! separate, later design.
//!
//! `Brush` is neither `Eq` nor `Hash` (float fields throughout); use
//! [`Brush::key_eq`] / [`Brush::key_hash`] for the canonicalised
//! diff-friendly comparison, the same convention as
//! [`Value::key_eq`](crate::Value::key_eq).

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::color::Color;
use crate::patterns::fill::{pattern_key_eq, pattern_key_hash, FillPattern};
use crate::value::{canonical_f32_bits, canonical_f64_bits};

pub use peniko::{
    Blob, ColorStop, ColorStops, Extend, Gradient, GradientKind, ImageAlphaType, ImageBrush,
    ImageData, ImageFormat, ImageQuality, ImageSampler, InterpolationAlphaSpace,
    LinearGradientPosition, RadialGradientPosition, SweepGradientPosition,
};
// kurbo's Stroke is part of the pattern-element vocabulary; re-export it
// beside the peniko types so consumers name one type.
pub use peniko::kurbo::Stroke;

/// Anything a renderer can paint a shape with.
///
/// `Solid` / `Gradient` / `Image` wrap peniko's types unchanged;
/// [`as_peniko`](Self::as_peniko) hands them back for scene APIs.
/// `Pattern` is zygos-side declarative vocabulary — see
/// [`FillPattern`](crate::patterns::fill::FillPattern) — and converts to
/// nothing; the consumer tiles it itself.
#[derive(Clone, Debug)]
pub enum Brush {
    /// A solid color.
    Solid(Color),
    /// A gradient.
    Gradient(Gradient),
    /// An image pattern.
    Image(ImageBrush),
    /// A declarative vector fill pattern. Carried by `Arc` so a pattern
    /// repeated across many rows shares its definition.
    Pattern(Arc<FillPattern>),
}

impl Brush {
    /// Hand back the peniko brush for scene APIs, if this brush is one
    /// peniko can express. Patterns return `None` — the consumer tiles
    /// them through its own machinery.
    pub fn as_peniko(&self) -> Option<peniko::Brush> {
        match self {
            Brush::Solid(c) => Some(peniko::Brush::Solid(*c)),
            Brush::Gradient(g) => Some(peniko::Brush::Gradient(g.clone())),
            Brush::Image(i) => Some(peniko::Brush::Image(i.clone())),
            Brush::Pattern(_) => None,
        }
    }

    /// Structural equality for diff/lookup keys (float fields rule out
    /// `Eq`). Solids compare by color; gradients by kind coordinates,
    /// stops, extend, and interpolation parameters; images by
    /// [`Blob`] id plus dimensions, format, alpha type, and sampler
    /// settings; patterns structurally (see
    /// [`pattern_key_eq`]), with `Arc` pointer identity as a fast path.
    /// NaN / -0 are canonicalised in every float comparison. Mirrors
    /// [`key_hash`](Self::key_hash).
    pub fn key_eq(&self, other: &Brush) -> bool {
        match (self, other) {
            (Brush::Solid(x), Brush::Solid(y)) => x == y,
            (Brush::Gradient(x), Brush::Gradient(y)) => gradient_key_eq(x, y),
            (Brush::Image(x), Brush::Image(y)) => image_key_eq(x, y),
            (Brush::Pattern(x), Brush::Pattern(y)) => Arc::ptr_eq(x, y) || pattern_key_eq(x, y),
            _ => false,
        }
    }

    /// Deterministic hash. Mirrors [`key_eq`](Self::key_eq).
    pub fn key_hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Brush::Solid(c) => {
                let [r, g, b, a] = c.components;
                canonical_f32_bits(r).hash(state);
                canonical_f32_bits(g).hash(state);
                canonical_f32_bits(b).hash(state);
                canonical_f32_bits(a).hash(state);
            }
            Brush::Gradient(g) => gradient_key_hash(g, state),
            Brush::Image(i) => image_key_hash(i, state),
            Brush::Pattern(p) => pattern_key_hash(p, state),
        }
    }
}

impl From<peniko::Brush> for Brush {
    fn from(b: peniko::Brush) -> Self {
        match b {
            peniko::Brush::Solid(c) => Brush::Solid(c),
            peniko::Brush::Gradient(g) => Brush::Gradient(g),
            peniko::Brush::Image(i) => Brush::Image(i),
        }
    }
}
impl From<Color> for Brush {
    fn from(c: Color) -> Self {
        Brush::Solid(c)
    }
}
impl From<Gradient> for Brush {
    fn from(g: Gradient) -> Self {
        Brush::Gradient(g)
    }
}
impl From<ImageBrush> for Brush {
    fn from(i: ImageBrush) -> Self {
        Brush::Image(i)
    }
}
impl From<FillPattern> for Brush {
    fn from(p: FillPattern) -> Self {
        Brush::Pattern(Arc::new(p))
    }
}
impl From<Arc<FillPattern>> for Brush {
    fn from(p: Arc<FillPattern>) -> Self {
        Brush::Pattern(p)
    }
}

fn gradient_key_eq(a: &Gradient, b: &Gradient) -> bool {
    fn point_eq(a: peniko::kurbo::Point, b: peniko::kurbo::Point) -> bool {
        canonical_f64_bits(a.x) == canonical_f64_bits(b.x)
            && canonical_f64_bits(a.y) == canonical_f64_bits(b.y)
    }
    let kind_eq = match (&a.kind, &b.kind) {
        (GradientKind::Linear(x), GradientKind::Linear(y)) => {
            point_eq(x.start, y.start) && point_eq(x.end, y.end)
        }
        (GradientKind::Radial(x), GradientKind::Radial(y)) => {
            point_eq(x.start_center, y.start_center)
                && point_eq(x.end_center, y.end_center)
                && canonical_f32_bits(x.start_radius) == canonical_f32_bits(y.start_radius)
                && canonical_f32_bits(x.end_radius) == canonical_f32_bits(y.end_radius)
        }
        (GradientKind::Sweep(x), GradientKind::Sweep(y)) => {
            point_eq(x.center, y.center)
                && canonical_f32_bits(x.start_angle) == canonical_f32_bits(y.start_angle)
                && canonical_f32_bits(x.end_angle) == canonical_f32_bits(y.end_angle)
        }
        _ => false,
    };
    kind_eq
        && a.extend == b.extend
        && a.interpolation_cs == b.interpolation_cs
        && a.hue_direction == b.hue_direction
        && a.interpolation_alpha_space == b.interpolation_alpha_space
        && a.stops.len() == b.stops.len()
        && a.stops.iter().zip(b.stops.iter()).all(|(x, y)| {
            canonical_f32_bits(x.offset) == canonical_f32_bits(y.offset)
                && x.color.cs == y.color.cs
                && x.color
                    .components
                    .iter()
                    .zip(y.color.components.iter())
                    .all(|(u, v)| canonical_f32_bits(*u) == canonical_f32_bits(*v))
        })
}

fn gradient_key_hash<H: Hasher>(g: &Gradient, state: &mut H) {
    fn point_hash<H2: Hasher>(p: peniko::kurbo::Point, state: &mut H2) {
        canonical_f64_bits(p.x).hash(state);
        canonical_f64_bits(p.y).hash(state);
    }
    std::mem::discriminant(&g.kind).hash(state);
    match &g.kind {
        GradientKind::Linear(l) => {
            point_hash(l.start, state);
            point_hash(l.end, state);
        }
        GradientKind::Radial(r) => {
            point_hash(r.start_center, state);
            point_hash(r.end_center, state);
            canonical_f32_bits(r.start_radius).hash(state);
            canonical_f32_bits(r.end_radius).hash(state);
        }
        GradientKind::Sweep(s) => {
            point_hash(s.center, state);
            canonical_f32_bits(s.start_angle).hash(state);
            canonical_f32_bits(s.end_angle).hash(state);
        }
    }
    std::mem::discriminant(&g.extend).hash(state);
    std::mem::discriminant(&g.interpolation_cs).hash(state);
    std::mem::discriminant(&g.hue_direction).hash(state);
    std::mem::discriminant(&g.interpolation_alpha_space).hash(state);
    (g.stops.len() as u64).hash(state);
    for stop in g.stops.iter() {
        canonical_f32_bits(stop.offset).hash(state);
        std::mem::discriminant(&stop.color.cs).hash(state);
        for c in stop.color.components {
            canonical_f32_bits(c).hash(state);
        }
    }
}

fn image_key_eq(a: &ImageBrush, b: &ImageBrush) -> bool {
    fn data_eq(a: &ImageData, b: &ImageData) -> bool {
        a.data.id() == b.data.id()
            && a.format == b.format
            && a.alpha_type == b.alpha_type
            && a.width == b.width
            && a.height == b.height
    }
    fn sampler_eq(a: &ImageSampler, b: &ImageSampler) -> bool {
        a.x_extend == b.x_extend
            && a.y_extend == b.y_extend
            && a.quality == b.quality
            && canonical_f32_bits(a.alpha) == canonical_f32_bits(b.alpha)
    }
    data_eq(&a.image, &b.image) && sampler_eq(&a.sampler, &b.sampler)
}

fn image_key_hash<H: Hasher>(i: &ImageBrush, state: &mut H) {
    i.image.data.id().hash(state);
    std::mem::discriminant(&i.image.format).hash(state);
    std::mem::discriminant(&i.image.alpha_type).hash(state);
    i.image.width.hash(state);
    i.image.height.hash(state);
    std::mem::discriminant(&i.sampler.x_extend).hash(state);
    std::mem::discriminant(&i.sampler.y_extend).hash(state);
    std::mem::discriminant(&i.sampler.quality).hash(state);
    canonical_f32_bits(i.sampler.alpha).hash(state);
}
