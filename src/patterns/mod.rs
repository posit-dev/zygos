//! Definitions of the named shape and linetype patterns scales map to.
//!
//! Shape and linetype *names* are core scale vocabulary — a shape scale
//! maps to [`OutputRange::Strings`](crate::OutputRange::Strings) of
//! names, and [`LinetypeStep`](crate::LinetypeStep) /
//! [`OutputRange::Linetypes`](crate::OutputRange::Linetypes) carry dash
//! patterns — so their definitions live here, always compiled. The
//! curated *palettes* built on these names (and the large color tables)
//! live in the feature-gated [`palettes`](crate::palettes) module.

pub mod linetype;
pub mod shape;
