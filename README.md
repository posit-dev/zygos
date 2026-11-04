# zygos

Scale primitives for data visualization: values, ranges, transforms, and tick / break algorithms.

`zygos` maps a domain value to a visual output — a panel fraction, a colour, a pt size, a dash pattern. It is a leaf crate with a minimal dependency footprint (`std` + `peniko`), designed to be embedded by a plotting or rendering layer, which supplies its own scale aggregate and draws the axes and legends.

## Design

- **Free functions, no traits.** Every algorithm is a plain free function over enum tags (`ScaleTypeKind`, `TransformKind`) and POD config types. Dispatch is `match`; there is no runtime polymorphism.
- **Stateless.** All configuration is passed per call; scales are pure mappers shared across renders.
- **Rendering lives elsewhere.** This crate decides *what* ticks and breaks exist; consumers draw them.

## Scale types

| Kind | Behaviour |
|---|---|
| `Continuous` | Linear interpolation over a numeric domain; numeric or colour output. |
| `Discrete` | One-to-one lookup: `input[i]` → `output[i]`. |
| `Ordinal` | Ordered discrete domain interpolated through a continuous output range. |
| `Binned` | Explicit bin edges; data maps to bin centres, breaks land on edges. |
| `Identity` | Pass-through. |
| `Temporal` | Continuous mapping with calendar-aligned break generation (`Date` / `DateTime` / `Time` / `Duration`). |

Thirteen transforms are wired (`Identity`, `Log10`, `Log2`, `Log`, `Sqrt`, `Square`, `Exp10`, `Exp2`, `Exp`, `Asinh`, `PseudoLog`, `PseudoLog2`, `PseudoLog10`), each with forward, inverse, and allowed-domain handling. Break algorithms include Wilkinson extended, log-pretty, symlog, and calendar-aware temporal break/minor-break generation.

Colour output interpolates through a named `ColorSpace` (`Oklab` by default).

## Features

| Feature | Effect |
|---|---|
| `geom-wkt` / `geom-wkb` / `geom-geojson` | Dependency-free parsers for `Geometry` constructors. |
| `chrono` / `time` / `jiff` | `From` impls between the temporal newtypes and the named datetime library. |

## License

MIT — see [LICENSE.md](LICENSE.md).
