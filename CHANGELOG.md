# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2026-11-05

### Added

- `auto_temporal_interval(min, max, unit, n)`: the `TemporalInterval`
  `temporal_breaks_from_f64` picks for the same arguments, exposed so
  consumers deriving minor breaks externally can subdivide the interval
  the majors actually stepped by.
- `integer_breaks` and `graticule_breaks`: integer-constrained evenly
  spaced breaks (nice 1-2-5 steps ≥ 1, or exact linear spacing rounded
  to integers), and lat/long graticule lines from a fixed step ladder
  with ±90 / ±180 boundary inclusion. Moved from ggsql.
- `Closed` (`Left` / `Right`) for binned scales: which side of a bin
  owns an exact edge value.

### Changed

- **Breaking:** `binned_map` and `binned_band_width_at` take a `closed:
  Closed` parameter. `Closed::Left` (the default) reproduces the
  previous `[e_i, e_{i+1})` behaviour.
- **Breaking:** `OobPolicy::apply` no longer passes NaN through under
  every policy: `Censor` now drops NaN (`None`), while `Keep` and
  `Squish` return `Some(NaN)` — matching `f64::clamp`.
- `temporal_breaks_from_f64` is now re-exported at the crate root.

## [0.2.0] - 2026-11-05

### Added

- `patterns` module (always compiled): the named shape and linetype
  definitions scales map to.
  - `patterns::shape`: the 29 built-in shapes (15 point shapes plus
    line-ending terminators) as kurbo `BezPath` geometry with style and
    anchor metadata (`Shape`, `builtin_shape`, `NAMES`,
    `POINT_SHAPE_NAMES`). Canonical definitions shared by hephaestus
    and ggsql.
  - `patterns::linetype`: named and ggplot2-style hex linetype patterns
    resolved to `LinetypeStep` sequences (`linetype_pattern`). Moved
    from ggsql.
- `palettes` module behind the `palettes` feature flag (default off) —
  curated palettes, with the ~300KB of color tables opt-in:
  - `palettes::color`: the named palette tables (categorical,
    ColorBrewer, viridis family, Crameri scientific colour maps) holding
    `Color` values directly (no hex-string parsing) and
    `get_color_palette`. Moved from ggsql.
  - `palettes::shape`: the shape name palettes (`SHAPES` /
    `SHAPES_CLOSED` / `SHAPES_OPEN`, `get_shape_palette`).
  - `palettes::linetype`: the `LINETYPES` palette with
    `get_linetype_palette`, and `generate_linetype_sequential`.
- `color::ramp`: multi-stop color interpolation with exact stop
  reproduction.
- `oob`: `OobPolicy` (censor / keep / squish) for values outside a scale's
  continuous domain.
- `discrete_map_at` and `ordinal_map_at`: `discrete_map` / `ordinal_map`
  with the domain scan already done. Callers with a cached value→index map
  (e.g. a `Scale` bundle) skip the O(|domain|) scan per lookup.

### Changed

- **Breaking:** `LinetypeStep::Dash` / `Gap` lengths are now unitless
  multiples of the line width (ggplot2 convention), documented as resolved
  to pt/px by the consumer, rather than pt. A marker occupies one line
  width of arc length.

### Fixed

- `expand_range_transformed` / `expand_range_transformed_selective`: an end
  whose pad leaves the transform's image no longer folds back inside the
  data range (sqrt padded below 0 came back as a positive square, hiding
  data) or returns NaN (exp padded below a 0 output) — such an end is left
  unpadded.
- `continuous_breaks` / `continuous_minor_breaks` for `Asinh` and the
  baseless `PseudoLog` now place breaks on the base-10 1-2-5 pattern
  instead of powers of e, which labelled as 2.718….

## [0.1.0] - 2026-11-04

Initial release, extracted from [hephaestus](https://github.com/posit-dev/hephaestus)
as a standalone leaf crate.

### Added

- Scale types as a closed `ScaleTypeKind` enum — `Continuous`, `Discrete`,
  `Ordinal`, `Binned`, `Identity`, `Temporal` — with per-kind free functions
  for mapping, breaks, minor breaks, and band widths.
- Thirteen transforms (`Identity`, `Log10`, `Log2`, `Log`, `Sqrt`, `Square`,
  `Exp10`, `Exp2`, `Exp`, `Asinh`, `PseudoLog`, `PseudoLog2`, `PseudoLog10`)
  with forward, inverse, and allowed-domain dispatch, and transform-aware
  domain expansion (`expand_range_transformed`).
- Tick selection: Wilkinson extended breaks, evenly-spaced linear breaks,
  log pretty / minor breaks, sqrt breaks, and symlog breaks for the Asinh /
  PseudoLog family.
- Calendar-aware temporal breaks and minor breaks for `Date`, `DateTime`,
  `Time`, and `Duration`, placed on an epoch-anchored grid (quarter-aligned
  months, Monday weeks, :00/:15/:30/:45 quarters of an hour) with minors
  nesting inside majors by construction.
- `InputRange` (continuous / discrete), `OutputRange` (numbers, strings,
  colours, linetypes), and `Direction` for reversal without backwards domains.
- Colour interpolation through a named `ColorSpace` (`Oklab` by default).
- The `Value` enum scales map, including temporal newtypes and
  `Value::Geometry`, with optional hand-rolled WKT / WKB / GeoJSON parsers
  behind the `geom-wkt` / `geom-wkb` / `geom-geojson` features.
- Optional datetime-library interop (`From` impls for the temporal newtypes)
  behind the `chrono`, `time`, and `jiff` features.
- `Locale` as a tag carried alongside break labels.

[Unreleased]: https://github.com/posit-dev/zygos/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/posit-dev/zygos/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/posit-dev/zygos/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/posit-dev/zygos/releases/tag/v0.1.0
