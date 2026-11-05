//! Curated named palettes, behind the `palettes` feature.
//!
//! One submodule per aesthetic:
//!
//! - [`color`] — named color palette tables (categorical, ColorBrewer,
//!   viridis family, Crameri scientific colour maps) — the bulk of this
//!   feature's compiled size.
//! - [`shape`] — the shape name palettes (`SHAPES` and friends).
//! - [`linetype`] — the linetype name palette and the sequential
//!   linetype generator.
//!
//! The shape and linetype *definitions* these palettes draw on live in
//! the always-compiled [`patterns`](crate::patterns) module.

pub mod color;
pub mod linetype;
pub mod shape;
