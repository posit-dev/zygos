//! Brush vocabulary — shared paint types for fill channels.
//!
//! A *brush* is anything a renderer can paint a shape with: a solid
//! color, a gradient, or an image pattern. peniko's [`Brush`] is exactly
//! the right shape (`Solid(Color) | Gradient(Gradient) |
//! Image(ImageBrush)`), and every consumer already converts to it at the
//! scene boundary — so rather than defining a parallel type that
//! hephaestus and ggsql would each have to convert, this module
//! re-exports peniko's types as the shared vocabulary, the same role
//! [`LinetypeStep`](crate::LinetypeStep) plays for dash patterns.
//!
//! Brushes flow through zygos as [`Value::Brush`](crate::Value::Brush)
//! and [`DataColumn::Brush`](crate::DataColumn::Brush). They are opaque
//! to scales: constant channels carry them (the per-category raw fill
//! use case); a data-driven fill scale with gradient interpolation is a
//! separate, later design.

pub use peniko::{
    Blob, Brush, ColorStop, ColorStops, Extend, Gradient, GradientKind, ImageAlphaType, ImageBrush,
    ImageData, ImageFormat, ImageQuality, ImageSampler, InterpolationAlphaSpace,
    LinearGradientPosition, RadialGradientPosition, SweepGradientPosition,
};
