# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `BreakTarget::MaxSpacing` / `BreakTarget::MinSpacing` — spacing
  variants with a rounding direction, backing consumer syntax like
  "<40 pt" / ">40 pt". `MaxSpacing` resolves `axis_len / spacing` by
  rounding the count up (the resolved gap never exceeds N pt, erring
  towards one more break); `MinSpacing` rounds down (the gap never
  dips below N pt, erring towards one fewer). The existing `Spacing`
  keeps rounding to nearest. The resolved count still floors at 2,
  which can breach a `MinSpacing` guarantee on a very short axis.
- `{:num_align [fmt]}` — a `:num`-family placeholder that pads its
  labels with figure spaces (U+2007) across the break set so stacked
  decimal points align, for table display with tabular-figure fonts.
  Bare `{:num_align}` uses the default number rendering;
  `{:num_align %.2f}` pins a printf format first. The padding is
  set-level, so it applies in `LabelSpec::labels`, not the value-local
  `LabelSpec::label`. The pass is also exposed directly as
  `decimal_align` (plus the `FIGURE_SPACE` constant) for consumers
  with their own formatters.
- `{:duration [unit] [N]}` — the duration placeholder accepts a part
  count alongside the unit (either order, e.g. `{:duration 3}` or
  `{:duration h 3}`), rendering the N largest units (`2h 30m 15s`)
  instead of always the two largest. Default remains 2, so existing
  templates are unaffected.

### Fixed

- `default_label` / `LabelSpec::label` with unpinned precision no
  longer print accumulated floating-point noise: values are snapped
  through a 12-significant-figure round trip before shortest-round-trip
  display, so `0.1 + 0.2` labels as `0.3` rather than
  `0.30000000000000004`. Zero, non-finite values and pinned-precision
  rendering are untouched. This restores the scrub hephaestus's
  deleted consumer-side formatter used to apply, which single-value
  and out-of-break-set labels had lost.

## [0.4.0] - 2026-11-06

### Added

- `label` module — break-label specs carried as plain data alongside a
  scale and re-rendered against whatever break set the scale currently
  produces (viewport resize, re-domain), instead of folding formatting
  into a static string map at first break computation. `LabelSpec` is a
  `Value`-keyed rename table (`rename` / `suppress`, matched by
  `Value::key_eq`) plus a fallback `Template`; `labels()` resolves
  rename-first, then expands the template, deriving a shared decimal
  precision across the current break set (`shared_precision`, after R's
  `scales::precision()`). `default_label` renders a bare `Value`.
  Template minilanguage, std-only and hand-rolled:
    - `{}`, `{:UPPER}` / `{:lower}` / `{:Title}` (keywords are
      ASCII-case-insensitive, SQL-style; payloads stay case-sensitive).
    - `{:num <fmt>}` — printf subset: conversions `d i u o x X f F e E
      g G s` and `%%`, flags `- + space 0 #`, width, precision, ignored
      length modifiers (`%ld`), and the POSIX `'` thousands separator
      (`,` accepted as an alias). Extensions: `%S` = SI-prefixed
      notation with precision as significant digits (`{:num %.2S}` →
      `1.2M`), and the `!` flag = Unicode scientific notation on
      `e/E/g/G` (`{:num %!.2e}` → `1.23 × 10³`). A `{:num}` body
      without the `%` introducer degrades to plain `{}`.
    - `{:time <fmt>}` — strftime subset over the temporal `Value`
      variants and ISO strings: `%Y %y %m %d %e %H %I %M %S %p %P %b %B
      %h %a %A %j %s %t %n %u %w %U %W %V %G %%`, composites
      `%F %T %R %D %r`, fractional seconds `%f` (9-digit ns), `%.f`,
      `%.3f` / `%.6f` / `%.9f`, and the `%-` / `%_` / `%0` padding
      modifiers. `%V` / `%G` are ISO 8601 week / week-year.
    - `{:abbr N}` (ellipsis truncation), `{:bytes [N]}` (base-1024 IEC
      units), `{:duration [unit]}` (composite spans like `1h 30m`;
      bare numbers read in `unit`, default seconds), `{:ordinal}`
      (`1st` / `2nd` / `3rd`).
    - Unknown placeholders degrade to `{}` for forward compatibility.
  Negative numbers render with U+2212 MINUS SIGN throughout (labels
  are presentation). Locale-aware names and separators remain the
  consumer formatter's job, as with `Locale`.
- `BreakTarget` and `resolve_target` (new `break_target` module):
  tick density as a count or as a desired physical spacing in pt.
  `BreakTarget::Spacing` stays unresolved until break time, when the
  consumer hands the rendered axis extent to `resolve_target`;
  resolution yields a count, so Wilkinson and the calendar-interval
  picker still choose the actual positions and the spacing is
  approximate. Degenerate spacing or an unusable axis length falls back
  to `DEFAULT_BREAK_COUNT`. Surfaced through
  `continuous_breaks_with_target` and `temporal_breaks_with_target`,
  which delegate to the existing transform-aware / calendar-aligned
  break paths after resolving.
- `Value::Relative` / `DataColumn::Relative`: a factor of a
  consumer-defined base quantity (e.g. `0.8` × a resolved font size),
  constructed via the `Rel` newtype / `rel()` constructor. Opaque to
  scales — `as_number` does not project it, so scale training rejects a
  relative value as non-numeric. Read back with `Value::as_relative`.

### Changed

- **Breaking:** `continuous_map`, `discrete_map`, `discrete_map_at`,
  `ordinal_map`, `ordinal_map_at`, and `binned_map` take a `na_value:
  &Value` parameter, returned for missing data — `Value::Null`, numeric
  NaN, and (discrete / ordinal) values not in the domain. Type
  mismatches and out-of-domain binned values still map to `Null`.
  Passing `&Value::Null` reproduces the previous behaviour.

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

[Unreleased]: https://github.com/posit-dev/zygos/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/posit-dev/zygos/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/posit-dev/zygos/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/posit-dev/zygos/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/posit-dev/zygos/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/posit-dev/zygos/releases/tag/v0.1.0
