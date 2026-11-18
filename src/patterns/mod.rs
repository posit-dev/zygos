//! Definitions of the named shape and linetype patterns scales map to,
//! plus declarative vector fill patterns.
//!
//! Shape and linetype *names* are core scale vocabulary — a shape scale
//! maps to [`OutputRange::Strings`](crate::OutputRange::Strings) of
//! names, and [`LinetypeStep`](crate::LinetypeStep) /
//! [`OutputRange::Linetypes`](crate::OutputRange::Linetypes) carry dash
//! patterns — so their definitions live here, always compiled. The
//! curated *palettes* built on these names (and the large color tables)
//! live in the feature-gated [`palettes`](crate::palettes) module.
//!
//! [`fill`] holds [`FillPattern`](fill::FillPattern), the tile-of-
//! elements vocabulary behind [`Brush::Pattern`](crate::Brush) — pure
//! declarative geometry the consumer resolves, shapes text for, and
//! tiles.

pub mod fill;
pub mod linetype;
pub mod shape;
