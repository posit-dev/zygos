# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `discrete_map_at` and `ordinal_map_at`: `discrete_map` / `ordinal_map`
  with the domain scan already done. Callers with a cached value→index map
  (e.g. a `Scale` bundle) skip the O(|domain|) scan per lookup.

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

[Unreleased]: https://github.com/posit-dev/zygos/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/posit-dev/zygos/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/posit-dev/zygos/releases/tag/v0.1.0
