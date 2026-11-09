//! Break-label specs — a rename table plus a template minilanguage,
//! carried as plain data alongside a scale.
//!
//! A [`LabelSpec`] answers one question — *what text does this break
//! get?* — in a form that survives re-breaking. Because the spec is
//! keyed on [`Value`] and holds its template unparsed-as-data, a
//! consumer can re-render labels every time the break set changes
//! (viewport resize, domain update) instead of folding the formatting
//! into a static string map the first time breaks are computed.
//!
//! Resolution order for a break (mirrors ggplot2's `labels` semantics):
//!
//! 1. **Rename** — an entry equal to the break under [`Value::key_eq`]
//!    wins: `Some(label)` renames, a suppressed entry drops the label
//!    (`None`).
//! 2. **Template** — every other break is expanded through the
//!    template (default `{}`).
//!
//! ## Template syntax
//!
//! Placeholders are `{...}` inside otherwise literal text:
//!
//! - `{}` — the value's default rendering (numbers share a decimal
//!   precision across the break set; see [`shared_precision`])
//! - `{:UPPER}` / `{:lower}` / `{:Title}` — case transforms
//! - `{:abbr N}` — truncate to N characters with an ellipsis
//! - `{:bytes [N]}` — base-1024 sizes (`1.5 KiB`), N significant
//!   digits (default 3)
//! - `{:duration [unit] [N]}` — composite span (`1h 30m`) from a
//!   [`Value::Duration`] (µs) or a number read in `unit` (`ns`, `us`,
//!   `ms`, `s` — the default, `m`, `h`, `d`). N is how many unit parts
//!   to show, largest first (default 2): `{:duration 3}` gives
//!   `1h 30m 15s`
//! - `{:ordinal}` — `1st`, `2nd`, `3rd`, …
//! - `{:time <fmt>}` — calendar formatting with a strftime-style
//!   subset: `%Y %y %m %d %e %H %I %M %S %p %P %b %B %h %a %A %j %s
//!   %t %n %u %w %U %W %V %G %%` (`%V`/`%G` = ISO 8601 week and
//!   week-year), composites `%F %T %R %D %r`, fractional seconds `%f`
//!   (9-digit ns), `%.f`, `%.3f` / `%.6f` / `%.9f`, and the `%-` / `%_`
//!   / `%0` padding modifiers. Applies to the temporal [`Value`]
//!   variants directly and to [`Value::String`]s holding ISO dates /
//!   datetimes / times.
//! - `{:num <fmt>}` — number formatting with a printf-style subset
//!   (`%[flags][width][.precision][length]d i u o x X f F e E g G s`,
//!   plus `%%`). Flags include the POSIX `'` thousands separator
//!   (`{:num %',.2f}` → `1,234.50`) and two zygos extensions: `!`
//!   for Unicode scientific notation (`{:num %!e}` → `1.23 × 10⁴`)
//!   and `%S` for SI-prefixed notation with precision as significant
//!   digits (`{:num %.2S}` → `1.2M`). Length modifiers (`%ld`,
//!   `%lld`) are accepted and ignored since the value is always an
//!   f64.
//! - `{:num_align [fmt]}` — `{:num fmt}` (or the default rendering
//!   when bare) with figure-space (U+2007) padding across the break
//!   set so stacked decimal points align; a table-display concern
//!   that assumes tabular figures in the font
//!
//! Labels are presentation, so negative numbers render with U+2212
//! MINUS SIGN rather than a hyphen, throughout (plain `{}`, `{:num}`,
//! scientific exponents).
//!
//! Placeholder keywords are ASCII-case-insensitive, matching SQL
//! keyword handling (`{:NUM …}`, `{:upper}`, `{:TITLE}` all work);
//! the `{:time}` / `{:num}` payloads stay case-sensitive. A `{:num}`
//! body must start with the printf `%` introducer — one without it is
//! a malformed spec and degrades to `{}` rather than printing the
//! spec text as the label. Other unknown placeholders degrade to `{}`
//! too, so a newer consumer's template read by an older one still
//! renders something sensible. The language is deliberately
//! keyword-first (`:time …`, `:num …`) so new placeholder families
//! are additive.
//!
//! This module is std-only: the placeholder languages are hand-rolled
//! subsets. Locale-aware names (month names beyond English, digit
//! systems) are *not* here — that's the consumer's `LabelFormatter`'s
//! job, guided by [`Locale`](crate::Locale), exactly as before.

use crate::value::{Date, DateTime, Time, Value};

// ─── LabelSpec ───────────────────────────────────────────────────────────────

/// One rename-table entry: what to do with a break's label.
#[derive(Clone, Debug, PartialEq)]
enum Rename {
    /// Replace the label with this text.
    Label(String),
    /// Render no label for this break (the tick itself is the
    /// consumer's call).
    Suppress,
}

/// A break-label spec: a [`Value`]-keyed rename table plus a fallback
/// [`Template`].
///
/// Plain data with no rendering dependencies — construct it at
/// scale-configuration time, keep it next to the consumer's scale
/// bundle, and call [`labels`](Self::labels) against whatever break
/// set the scale currently produces.
#[derive(Clone, Debug, Default)]
pub struct LabelSpec {
    rename: Vec<(Value, Rename)>,
    template: Option<Template>,
}

impl LabelSpec {
    /// An empty spec: no renames, default `{}` formatting.
    pub const fn new() -> Self {
        Self {
            rename: Vec::new(),
            template: None,
        }
    }

    /// Rename the break equal to `value` (per [`Value::key_eq`]) to
    /// `label`. Re-adding an existing key replaces its entry.
    pub fn rename(&mut self, value: Value, label: impl Into<String>) -> &mut Self {
        self.set(value, Rename::Label(label.into()))
    }

    /// Suppress the label of the break equal to `value`
    /// (ggplot2's `na.value`-style blank for one specific break).
    pub fn suppress(&mut self, value: Value) -> &mut Self {
        self.set(value, Rename::Suppress)
    }

    fn set(&mut self, value: Value, rename: Rename) -> &mut Self {
        match self.rename.iter_mut().find(|(v, _)| v.key_eq(&value)) {
            Some(slot) => slot.1 = rename,
            None => self.rename.push((value, rename)),
        }
        self
    }

    /// Set the fallback template applied to every break with no rename
    /// entry. Parsing is infallible — unknown placeholders degrade to
    /// `{}`.
    pub fn set_template(&mut self, template: &str) -> &mut Self {
        self.template = Some(Template::parse(template));
        self
    }

    /// The fallback template, if one was set.
    pub fn template(&self) -> Option<&Template> {
        self.template.as_ref()
    }

    /// Look up `value` in the rename table.
    ///
    /// - `None` — no entry; the break falls through to the template.
    /// - `Some(None)` — suppressed.
    /// - `Some(Some(label))` — renamed.
    pub fn lookup(&self, value: &Value) -> Option<Option<&str>> {
        self.rename
            .iter()
            .find(|(v, _)| v.key_eq(value))
            .map(|(_, r)| match r {
                Rename::Label(l) => Some(l.as_str()),
                Rename::Suppress => None,
            })
    }

    /// The rename table in insertion order, each key paired with its
    /// entry in [`lookup`](Self::lookup)'s shape: `Some(label)` renames,
    /// `None` suppresses. For consumers that persist the spec or
    /// match keys under their own equivalence on top of
    /// [`Value::key_eq`].
    pub fn renames(&self) -> impl Iterator<Item = (&Value, Option<&str>)> + '_ {
        self.rename.iter().map(|(v, r)| {
            (
                v,
                match r {
                    Rename::Label(l) => Some(l.as_str()),
                    Rename::Suppress => None,
                },
            )
        })
    }

    /// Number of entries in the rename table.
    pub fn rename_len(&self) -> usize {
        self.rename.len()
    }

    /// `true` if the rename table is empty.
    pub fn rename_is_empty(&self) -> bool {
        self.rename.is_empty()
    }

    /// Label for a single break. `None` means suppressed (by an
    /// explicit entry or because the value is [`Value::Null`]).
    ///
    /// Numeric precision is value-local here; prefer
    /// [`labels`](Self::labels) for a break set so plain-`{}` numbers
    /// share a decimal precision.
    pub fn label(&self, value: &Value) -> Option<String> {
        self.label_inner(value, None)
    }

    /// Labels for a whole break set, one entry per break, aligned by
    /// index. Numbers rendered through a plain `{}` share a single
    /// decimal precision derived from the set
    /// ([`shared_precision`]), so `[54.5, 55, 55.5]` renders
    /// `["54.5", "55.0", "55.5"]` — and because this runs against the
    /// *current* break set on every call, a re-break (resize,
    /// re-domain) re-derives it.
    pub fn labels(&self, breaks: &[Value]) -> Vec<Option<String>> {
        let has_plain = self.template.as_ref().is_none_or(Template::has_plain);
        let precision = if has_plain {
            let unrenamed: Vec<Value> = breaks
                .iter()
                .filter(|v| self.lookup(v).is_none())
                .cloned()
                .collect();
            shared_precision(&unrenamed)
        } else {
            None
        };
        let mut labels: Vec<Option<String>> = breaks
            .iter()
            .map(|v| self.label_inner(v, precision))
            .collect();
        // Labels rendered through `{:num_align …}` get figure-space
        // padding so stacked decimal points line up. Set-level like
        // `shared_precision`; renamed labels are text and excluded.
        if self.template.as_ref().is_some_and(Template::has_num_align) {
            let idx: Vec<usize> = breaks
                .iter()
                .enumerate()
                .filter(|(_, v)| self.lookup(v).is_none())
                .map(|(i, _)| i)
                .collect();
            let mut subset: Vec<Option<String>> = idx.iter().map(|&i| labels[i].clone()).collect();
            decimal_align(&mut subset);
            for (i, label) in idx.into_iter().zip(subset) {
                labels[i] = label;
            }
        }
        labels
    }

    fn label_inner(&self, value: &Value, precision: Option<usize>) -> Option<String> {
        match self.lookup(value) {
            Some(label) => label.map(str::to_string),
            None => match &self.template {
                Some(t) => t.format(value, precision),
                None => default_label(value, precision),
            },
        }
    }
}

/// Structural equality: the same rename entries in the same order
/// (keys compared with [`Value::key_eq`], since [`Value`] has no
/// `PartialEq`) and templates parsed from the same source.
impl PartialEq for LabelSpec {
    fn eq(&self, other: &Self) -> bool {
        self.rename.len() == other.rename.len()
            && self
                .rename
                .iter()
                .zip(&other.rename)
                .all(|((a, ra), (b, rb))| a.key_eq(b) && ra == rb)
            && self.template == other.template
    }
}

// ─── Template ────────────────────────────────────────────────────────────────

/// A parsed label template: literal text interleaved with
/// [`Placeholder`]s. Parse once with [`Template::parse`], render per
/// break with [`Template::format`].
#[derive(Clone, Debug)]
pub struct Template {
    source: String,
    segments: Vec<Segment>,
}

/// Two templates are equal when parsed from the same source — the
/// segments are a pure function of it.
impl PartialEq for Template {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

#[derive(Clone, Debug)]
enum Segment {
    Literal(String),
    Placeholder(Placeholder),
}

/// A single `{...}` placeholder in a [`Template`].
#[derive(Clone, Debug, PartialEq)]
pub enum Placeholder {
    /// `{}` — the value's default rendering, sharing numeric precision
    /// across the break set.
    Plain,
    /// `{:UPPER}` — uppercase the default rendering.
    Upper,
    /// `{:lower}` — lowercase the default rendering.
    Lower,
    /// `{:Title}` — Title Case the default rendering.
    Title,
    /// `{:time <fmt>}` — strftime-subset calendar formatting.
    Time(String),
    /// `{:num <fmt>}` — printf-subset number formatting.
    Number(String),
    /// `{:num_align [fmt]}` — `{:num <fmt>}` (or the default rendering
    /// when bare), with figure-space (U+2007) padding applied across
    /// the break set so stacked decimal points align; see
    /// [`decimal_align`]. The padding is set-level, so it happens in
    /// [`LabelSpec::labels`] only — [`LabelSpec::label`] renders a lone
    /// value with nothing to align to. Tabular figures in the font are
    /// the user's call: with proportional figures the padding can't
    /// rescue alignment.
    NumAlign(String),
    /// `{:abbr N}` — truncate the default rendering to N characters
    /// (including the ellipsis).
    Abbr(usize),
    /// `{:bytes [N]}` — base-1024 sizes (`1.5 KiB`); N is significant
    /// digits, default 3.
    Bytes(Option<usize>),
    /// `{:duration [unit] [N]}` — composite span (`1h 30m`). Numbers
    /// are read in `unit` (`ns`, `us` / `µs`, `ms`, `s` — the default,
    /// `m`, `h`, `d`); [`Value::Duration`]s are always µs and ignore
    /// the unit. `N` is the number of unit parts shown, largest first
    /// (default 2).
    Duration(Option<f64>, Option<usize>),
    /// `{:ordinal}` — `1st`, `2nd`, `3rd`, …
    Ordinal,
}

fn parse_placeholder(inner: &str) -> Placeholder {
    let Some(body) = inner.strip_prefix(':') else {
        return Placeholder::Plain;
    };
    // Keywords are ASCII-case-insensitive, matching SQL keyword
    // handling: `{:NUM …}`, `{:upper}`, `{:TITLE}` all work. The
    // payload keeps its original case — strftime/printf bodies are
    // case-sensitive.
    let lower = body.to_ascii_lowercase();
    match lower.as_str() {
        "upper" => Placeholder::Upper,
        "lower" => Placeholder::Lower,
        "title" => Placeholder::Title,
        "ordinal" => Placeholder::Ordinal,
        _ if lower == "duration" || lower.starts_with("duration ") => {
            // µs per unit of bare-number input.
            let factor = |unit: &str| match unit {
                "" | "s" => Some(1_000_000.0),
                "ns" => Some(0.001),
                "us" | "\u{b5}s" => Some(1.0),
                "ms" => Some(1_000.0),
                "m" => Some(60_000_000.0),
                "h" => Some(3_600_000_000.0),
                "d" => Some(86_400_000_000.0),
                _ => None,
            };
            // Args are a unit and/or a part count, in either order.
            let mut unit = None;
            let mut parts = None;
            let mut ok = true;
            for tok in lower["duration".len()..].split_whitespace() {
                if let Ok(n) = tok.parse::<usize>() {
                    if parts.is_some() {
                        ok = false;
                        break;
                    }
                    parts = Some(n);
                } else if unit.is_none() {
                    match factor(tok) {
                        Some(f) => unit = Some(f),
                        None => {
                            ok = false;
                            break;
                        }
                    }
                } else {
                    ok = false;
                    break;
                }
            }
            if ok {
                Placeholder::Duration(unit, parts)
            } else {
                Placeholder::Plain
            }
        }
        _ if lower.starts_with("abbr ") => match lower["abbr ".len()..].trim().parse::<usize>() {
            Ok(n) => Placeholder::Abbr(n),
            Err(_) => Placeholder::Plain,
        },
        _ if lower == "bytes" || lower.starts_with("bytes ") => {
            match lower["bytes".len()..].trim() {
                "" => Placeholder::Bytes(None),
                p => match p.parse::<usize>() {
                    Ok(n) => Placeholder::Bytes(Some(n)),
                    Err(_) => Placeholder::Plain,
                },
            }
        }
        _ if lower == "num_align" => Placeholder::NumAlign(String::new()),
        _ if lower.starts_with("num_align ") => {
            let payload = &body["num_align ".len()..];
            // Same rule as `{:num}`: the body must start with the
            // printf `%` introducer or the spec degrades to `{}`.
            if !payload.trim_start().starts_with('%') {
                return Placeholder::Plain;
            }
            Placeholder::NumAlign(payload.to_string())
        }
        _ => {
            for (keyword, is_num) in [("time ", false), ("num ", true)] {
                if let Some(payload) = lower.strip_prefix(keyword) {
                    let payload = &body[body.len() - payload.len()..];
                    // `{:num}` requires the printf `%` introducer; a
                    // body without it is a malformed spec, not a
                    // literal, so degrade to Plain rather than print
                    // the spec text as the label.
                    if is_num && !payload.trim_start().starts_with('%') {
                        return Placeholder::Plain;
                    }
                    return if is_num {
                        Placeholder::Number(payload.to_string())
                    } else {
                        Placeholder::Time(payload.to_string())
                    };
                }
            }
            // Unknown placeholders degrade to Plain: forward-compatible
            // with future placeholder families.
            Placeholder::Plain
        }
    }
}

impl Template {
    /// Parse a template string. Infallible: an unmatched `{` is
    /// literal text, and unknown placeholders degrade to `{}`.
    pub fn parse(source: &str) -> Self {
        let mut segments = Vec::new();
        let mut rest = source;
        while let Some(open) = rest.find('{') {
            if open > 0 {
                segments.push(Segment::Literal(rest[..open].to_string()));
            }
            let after = &rest[open + 1..];
            match after.find('}') {
                Some(close) => {
                    segments.push(Segment::Placeholder(parse_placeholder(&after[..close])));
                    rest = &after[close + 1..];
                }
                None => {
                    segments.push(Segment::Literal(rest[open..].to_string()));
                    rest = "";
                }
            }
        }
        if !rest.is_empty() {
            segments.push(Segment::Literal(rest.to_string()));
        }
        Self {
            source: source.to_string(),
            segments,
        }
    }

    /// The string the template was parsed from.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// `true` if any segment is a plain `{}` (or the template has no
    /// placeholders at all) — the case where numeric breaks share a
    /// precision.
    pub fn has_plain(&self) -> bool {
        !self
            .segments
            .iter()
            .any(|s| matches!(s, Segment::Placeholder(_)))
            || self
                .segments
                .iter()
                .any(|s| matches!(s, Segment::Placeholder(Placeholder::Plain)))
    }

    /// `true` if any segment is `{:num_align …}` — the case where
    /// rendered labels get figure-space decimal alignment across the
    /// set ([`decimal_align`]).
    pub fn has_num_align(&self) -> bool {
        self.segments
            .iter()
            .any(|s| matches!(s, Segment::Placeholder(Placeholder::NumAlign(_))))
    }

    /// Render the template for one value. Returns `None` for
    /// [`Value::Null`]. A template with no placeholders renders its
    /// literal text for every value.
    pub fn format(&self, value: &Value, precision: Option<usize>) -> Option<String> {
        if value.is_null() {
            return None;
        }
        let mut out = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(s) => out.push_str(s),
                Segment::Placeholder(p) => out.push_str(&apply_placeholder(p, value, precision)),
            }
        }
        Some(out)
    }
}

fn apply_placeholder(p: &Placeholder, value: &Value, precision: Option<usize>) -> String {
    match p {
        Placeholder::Plain => default_label(value, precision).unwrap_or_default(),
        Placeholder::Upper => default_label(value, None)
            .unwrap_or_default()
            .to_uppercase(),
        Placeholder::Lower => default_label(value, None)
            .unwrap_or_default()
            .to_lowercase(),
        Placeholder::Title => title_case(&default_label(value, None).unwrap_or_default()),
        Placeholder::Time(fmt) => format_temporal(value, fmt),
        Placeholder::Number(spec) => match value.as_number() {
            Some(n) => printf_format(spec, n)
                .unwrap_or_else(|_| default_label(value, None).unwrap_or_default()),
            None => default_label(value, None).unwrap_or_default(),
        },
        Placeholder::NumAlign(spec) if spec.is_empty() => {
            default_label(value, precision).unwrap_or_default()
        }
        Placeholder::NumAlign(spec) => match value.as_number() {
            Some(n) => printf_format(spec, n)
                .unwrap_or_else(|_| default_label(value, None).unwrap_or_default()),
            None => default_label(value, None).unwrap_or_default(),
        },
        Placeholder::Abbr(max) => abbreviate(&default_label(value, None).unwrap_or_default(), *max),
        Placeholder::Bytes(sig) => match value.as_number() {
            Some(n) => {
                let body = bytes_notation(n.abs(), sig.unwrap_or(3));
                if n.is_sign_negative() {
                    format!("\u{2212}{body}")
                } else {
                    body
                }
            }
            None => default_label(value, None).unwrap_or_default(),
        },
        Placeholder::Duration(unit, parts) => {
            let us = match value {
                // Typed durations are µs; the unit arg doesn't apply.
                Value::Duration(us) => Some(*us as f64),
                Value::Number(n) => Some(*n * unit.unwrap_or(1_000_000.0)),
                _ => None,
            };
            match us {
                Some(us) => duration_notation(us, parts.unwrap_or(2)),
                None => default_label(value, None).unwrap_or_default(),
            }
        }
        Placeholder::Ordinal => match value.as_number() {
            Some(n) if n.fract() == 0.0 && n.is_finite() => ordinal(n as i64),
            _ => default_label(value, None).unwrap_or_default(),
        },
    }
}

/// Truncate `s` to at most `max` characters including an ellipsis.
fn abbreviate(s: &str, max: usize) -> String {
    if max == 0 || s.chars().count() <= max {
        return s.to_string();
    }
    let keep = max.saturating_sub(1).max(1);
    format!("{}\u{2026}", s.chars().take(keep).collect::<String>())
}

/// Base-1024 size notation: `1536` → `1.5 KiB`. `sig` significant
/// digits, trailing zeros stripped.
fn bytes_notation(n: f64, sig: usize) -> String {
    const UNITS: [&str; 9] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
    let unit = if n >= 1.0 {
        (n.log(1024.0).floor() as usize).min(UNITS.len() - 1)
    } else {
        0
    };
    let scaled = n / 1024f64.powi(unit as i32);
    if unit == 0 {
        // Bytes are whole; fractions only appear below 1 B.
        let body = if scaled.fract() == 0.0 {
            format!("{}", scaled as i64)
        } else {
            significant(scaled, sig)
        };
        return format!("{body} B");
    }
    format!("{} {}", significant(scaled, sig), UNITS[unit])
}

/// Format with `sig` significant digits, trailing zeros stripped.
fn significant(v: f64, sig: usize) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let sig = sig.max(1);
    let decimals = (sig as i32 - 1 - v.abs().log10().floor() as i32).max(0) as usize;
    let s = format!("{v:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Composite span notation from microseconds: the largest `max_parts`
/// nonzero units — `5400 s` → `1h 30m`, `90.2 s` → `1m 30s`. Sub-second
/// spans use `ms` / `µs`; zero is `0s`.
fn duration_notation(us: f64, max_parts: usize) -> String {
    let max_parts = max_parts.max(1);
    const UNITS: [(&str, f64); 6] = [
        ("d", 86_400_000_000.0),
        ("h", 3_600_000_000.0),
        ("m", 60_000_000.0),
        ("s", 1_000_000.0),
        ("ms", 1_000.0),
        ("\u{b5}s", 1.0),
    ];
    if us == 0.0 {
        return "0s".to_string();
    }
    let neg = us < 0.0;
    let mut rem = us.abs();
    let mut parts = Vec::new();
    for (name, size) in UNITS {
        if rem >= size && parts.len() < max_parts {
            let count = (rem / size).floor();
            parts.push(format!("{}{name}", significant(count, 4)));
            rem -= count * size;
        }
    }
    let body = if parts.is_empty() {
        // Sub-microsecond: show what there is.
        format!("{}\u{b5}s", significant(rem, 3))
    } else {
        parts.join(" ")
    };
    if neg {
        format!("\u{2212}{body}")
    } else {
        body
    }
}

/// Ordinal rendering: `1st`, `2nd`, `3rd`, `4th`, with the 11/12/13
/// exception. Non-integers don't reach here (guarded by the caller).
fn ordinal(n: i64) -> String {
    let suffix = match n.rem_euclid(100) {
        11..=13 => "th",
        _ => match n.rem_euclid(10) {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        },
    };
    if n < 0 {
        format!("\u{2212}{}{suffix}", n.unsigned_abs())
    } else {
        format!("{n}{suffix}")
    }
}

fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ─── Default rendering ───────────────────────────────────────────────────────

/// The default rendering of a [`Value`] — what a plain `{}` (or a
/// spec with no template) produces.
///
/// `precision` pins the decimal count for numbers (from
/// [`shared_precision`]); `None` renders the shortest round-trip form,
/// so integers print without a trailing `.0`. Temporal values render
/// as ISO calendar text; [`Value::Null`], [`Value::Linetype`],
/// [`Value::Geometry`] and [`Value::Brush`] render as `None`.
pub fn default_label(value: &Value, precision: Option<usize>) -> Option<String> {
    match value {
        Value::Null | Value::Linetype(_) | Value::Geometry(_) | Value::Brush(_) => None,
        Value::Number(n) | Value::Relative(n) => Some(format_number(*n, precision)),
        Value::String(s) => Some(s.to_string()),
        Value::Bool(b) => Some(if *b { "true" } else { "false" }.to_string()),
        Value::Date(d) => Some(iso_date(Date::from_days(*d))),
        Value::DateTime(us) => Some(iso_datetime(DateTime::from_micros(*us))),
        Value::Time(ns) => Some(iso_time(Time::from_nanos(*ns))),
        Value::Duration(us) => Some(format_number(*us as f64 / 1_000_000.0, precision)),
        Value::Color(c) => {
            let rgba = c.to_rgba8();
            if rgba.a == 255 {
                Some(format!("#{:02x}{:02x}{:02x}", rgba.r, rgba.g, rgba.b))
            } else {
                Some(format!(
                    "#{:02x}{:02x}{:02x}{:02x}",
                    rgba.r, rgba.g, rgba.b, rgba.a
                ))
            }
        }
    }
}

/// Render a number, pinning decimals when `precision` is set.
fn format_number(n: f64, precision: Option<usize>) -> String {
    let s = match precision {
        Some(p) => format!("{n:.p$}"),
        None => format!("{}", snap_float_noise(n)),
    };
    // Labels are presentation: U+2212 reads better than a hyphen.
    unicode_minus(s)
}

/// Snap away accumulated floating-point error before shortest-round-trip
/// display. A 12-significant-figure round trip (`0.1 + 0.2` → `0.3`,
/// `0.6000000000001` → `0.6`) kills ULP noise while leaving any precision a
/// label could plausibly intend intact. Zero and non-finite values pass
/// through untouched.
fn snap_float_noise(n: f64) -> f64 {
    if n == 0.0 || !n.is_finite() {
        return n;
    }
    format!("{n:.11e}").parse().unwrap_or(n)
}

/// Swap a leading ASCII hyphen for U+2212 MINUS SIGN.
fn unicode_minus(s: String) -> String {
    match s.strip_prefix('-') {
        Some(rest) => format!("\u{2212}{rest}"),
        None => s,
    }
}

/// U+2007 FIGURE SPACE — a space exactly one tabular digit wide.
pub const FIGURE_SPACE: char = '\u{2007}';

/// Pad the numeric labels in `labels` with figure spaces (U+2007) so
/// their decimal points align when the labels are stacked — mainly a
/// table-display concern. Integer parts are left-padded to the widest
/// in the set, fraction parts right-padded to the widest; a label with
/// no `.` gets no trailing pad. A label counts as numeric when it is
/// an optional sign (`-`, `+`, U+2212) plus ASCII digits with at most
/// one `.` — anything else (units, exponents, text) is left alone, as
/// are `None` entries.
///
/// The padding assumes tabular figures in the rendering font: U+2007
/// is one *digit* wide, so with proportional figures the aligned
/// columns still drift. Choosing a tabular-figures font is the user's
/// call.
pub fn decimal_align(labels: &mut [Option<String>]) {
    let parsed: Vec<Option<(bool, usize, usize)>> = labels
        .iter()
        .map(|l| l.as_deref().and_then(split_decimal))
        .collect();
    let mut max_before = 0;
    let mut max_after = 0;
    for &(has_dot, before, after) in parsed.iter().flatten() {
        max_before = max_before.max(before);
        if has_dot {
            max_after = max_after.max(after);
        }
    }
    if max_before == 0 {
        return;
    }
    for (label, parsed) in labels.iter_mut().zip(parsed) {
        let (Some(s), Some((has_dot, before, after))) = (label.as_ref(), parsed) else {
            continue;
        };
        let mut padded = String::with_capacity(s.len() + max_before + max_after);
        for _ in before..max_before {
            padded.push(FIGURE_SPACE);
        }
        padded.push_str(s);
        if has_dot {
            for _ in after..max_after {
                padded.push(FIGURE_SPACE);
            }
        }
        *label = Some(padded);
    }
}

/// Split a plain decimal label into (`has_dot`, integer-part char
/// count, fraction char count); `None` for anything but an optional
/// sign (`-` / `+` / U+2212) plus ASCII digits with at most one `.`.
/// The sign counts towards the integer width — in tabular-figure
/// fonts the minus sign is digit-width, like the padding.
fn split_decimal(s: &str) -> Option<(bool, usize, usize)> {
    let (before, after) = s.split_once('.').unwrap_or((s, ""));
    let digits = before
        .strip_prefix(['-', '+', '\u{2212}'])
        .unwrap_or(before);
    if digits.is_empty()
        || !digits.chars().all(|c| c.is_ascii_digit())
        || !after.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some((
        s.contains('.'),
        before.chars().count(),
        after.chars().count(),
    ))
}

/// Decimal places to display a numeric break set consistently, after
/// R's `scales::precision()`: derive display precision from the
/// smallest inter-break difference so `[54.5, 55, 55.5]` prints as
/// `["54.5", "55.0", "55.5"]` rather than stripping `55`'s `.0`.
/// Returns `None` for fewer than two numeric values or all-integers.
pub fn shared_precision(breaks: &[Value]) -> Option<usize> {
    let tol = f64::EPSILON.sqrt();

    let mut numbers: Vec<f64> = breaks
        .iter()
        .filter_map(|v| match v {
            Value::Number(n) => Some(*n),
            _ => None,
        })
        .collect();

    if numbers.len() < 2 {
        return None;
    }

    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let smallest_diff = numbers
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .filter(|d| *d > 0.0)
        .fold(f64::INFINITY, f64::min);

    if smallest_diff == f64::INFINITY || smallest_diff < tol {
        return None;
    }

    let mut precision = 10f64.powf(smallest_diff.log10().floor() - 1.0);

    // If all values are multiples of 10*precision, coarsen by one level.
    if numbers
        .iter()
        .all(|&x| ((x / precision).round() % 10.0).abs() < tol)
    {
        precision *= 10.0;
    }

    let n_decimals = (-precision.log10().floor()) as isize;
    if n_decimals > 0 {
        Some(n_decimals.min(20) as usize)
    } else {
        None
    }
}

// ─── ISO calendar rendering ──────────────────────────────────────────────────

fn iso_date(d: Date) -> String {
    let (y, m, day) = d.to_ymd();
    format!("{y:04}-{m:02}-{day:02}")
}

fn iso_datetime(dt: DateTime) -> String {
    let (d, us) = dt.split();
    let (h, m, s, sub) = split_time_us(us);
    let mut out = format!("{} {h:02}:{m:02}:{s:02}", iso_date(d));
    if sub > 0 {
        out.push_str(&format!(".{sub:06}"));
    }
    out
}

fn iso_time(t: Time) -> String {
    let ns = t.to_nanos().rem_euclid(86_400_000_000_000);
    let (h, m, s, sub) = split_time_us(ns / 1_000);
    let mut out = format!("{h:02}:{m:02}:{s:02}");
    if sub > 0 {
        out.push_str(&format!(".{sub:06}"));
    }
    out
}

/// Microseconds-since-midnight → (h, m, s, sub-second µs).
fn split_time_us(us: i64) -> (i64, i64, i64, i64) {
    let h = us.div_euclid(3_600_000_000);
    let m = us.rem_euclid(3_600_000_000).div_euclid(60_000_000);
    let s = us.rem_euclid(60_000_000).div_euclid(1_000_000);
    let sub = us.rem_euclid(1_000_000);
    (h, m, s, sub)
}

// ─── `{:time …}` — strftime subset ───────────────────────────────────────────

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const WEEKDAYS_SHORT: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const WEEKDAYS_LONG: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Calendar parts extracted from a temporal value: the date (when the
/// value carries one) and the time-of-day (idem).
struct CalendarParts {
    date: Option<Date>,
    /// (h, m, s, sub-second ns), when the value carries a time.
    time: Option<(i64, i64, i64, i64)>,
}

fn calendar_parts(value: &Value) -> Option<CalendarParts> {
    match value {
        Value::Date(d) => Some(CalendarParts {
            date: Some(Date::from_days(*d)),
            time: None,
        }),
        Value::DateTime(us) => {
            let (d, day_us) = DateTime::from_micros(*us).split();
            let (h, m, s, sub) = split_time_us(day_us);
            Some(CalendarParts {
                date: Some(d),
                time: Some((h, m, s, sub * 1_000)),
            })
        }
        Value::Time(ns) => {
            let ns = ns.rem_euclid(86_400_000_000_000);
            let h = ns / 3_600_000_000_000;
            let m = ns.rem_euclid(3_600_000_000_000) / 60_000_000_000;
            let s = ns.rem_euclid(60_000_000_000) / 1_000_000_000;
            let sub = ns.rem_euclid(1_000_000_000);
            Some(CalendarParts {
                date: None,
                time: Some((h, m, s, sub)),
            })
        }
        // Strings holding ISO calendar text stay formattable, for
        // consumers whose discrete domains arrive as strings.
        Value::String(s) => parse_iso_string(s),
        _ => None,
    }
}

/// Parse the ISO shapes the temporal projections print:
/// `YYYY-MM-DD`, `YYYY-MM-DD[T ]HH:MM[:SS[.f]]`, `HH:MM[:SS[.f]]`.
fn parse_iso_string(s: &str) -> Option<CalendarParts> {
    let b = s.as_bytes();
    let num = |i: usize, len: usize| -> Option<i64> {
        b.get(i..i + len)
            .filter(|d| d.iter().all(u8::is_ascii_digit))
            .and_then(|d| std::str::from_utf8(d).ok())
            .and_then(|d| d.parse().ok())
    };
    let parse_time = |i: usize| -> Option<(i64, i64, i64, i64)> {
        let h = num(i, 2)?;
        if b.get(i + 2) != Some(&b':') {
            return None;
        }
        let m = num(i + 3, 2)?;
        let mut s = 0;
        let mut sub = 0;
        if b.get(i + 5) == Some(&b':') {
            s = num(i + 6, 2)?;
            if b.get(i + 8) == Some(&b'.') {
                let digits: String = b[i + 9..]
                    .iter()
                    .take_while(|c| c.is_ascii_digit())
                    .map(|c| *c as char)
                    .collect();
                if !digits.is_empty() {
                    let mut frac = digits;
                    frac.truncate(9);
                    while frac.len() < 9 {
                        frac.push('0');
                    }
                    sub = frac.parse().ok()?;
                }
            }
        }
        Some((h, m, s, sub))
    };

    // Date prefix?
    if b.len() >= 10 && b[4] == b'-' && b[7] == b'-' {
        let (y, m, d) = (num(0, 4)? as i32, num(5, 2)? as u8, num(8, 2)? as u8);
        let date = Date::from_ymd(y, m, d);
        if b.len() > 10 && (b[10] == b'T' || b[10] == b' ') {
            return parse_time(11).map(|time| CalendarParts {
                date: Some(date),
                time: Some(time),
            });
        }
        return Some(CalendarParts {
            date: Some(date),
            time: None,
        });
    }
    // Bare time?
    parse_time(0).map(|time| CalendarParts {
        date: None,
        time: Some(time),
    })
}

/// Field padding for strftime numeric tokens.
#[derive(Clone, Copy, PartialEq)]
enum Pad {
    /// Zero-pad (the default).
    Zero,
    /// `%_` — space-pad.
    Space,
    /// `%-` — no padding.
    None,
}

impl Pad {
    fn apply(self, v: i64, width: usize) -> String {
        match self {
            Pad::Zero => format!("{v:0width$}"),
            Pad::Space => format!("{v:width$}"),
            Pad::None => format!("{v}"),
        }
    }
}

/// ISO 8601 week-year and week number: weeks start Monday, week 1 is
/// the week containing the year's first Thursday. The Thursday of the
/// current week always falls in the ISO week-year, which resolves the
/// Dec-29/Jan-3 boundary without special cases.
fn iso_week(date: Date) -> (i32, i64) {
    let thursday = date.to_days() - date.day_of_week() as i32 + 3;
    let (gy, _, _) = Date::from_days(thursday).to_ymd();
    let week = ((thursday - Date::from_ymd(gy, 1, 1).to_days()) / 7 + 1) as i64;
    (gy, week)
}

/// Week of year with week 1 starting on the first `week_start`
/// (`Date::day_of_week()` ordinal: 0 = Monday, 6 = Sunday); days
/// before it are week 00. `%U` is Sunday-start, `%W` Monday-start.
fn week_of_year(date: Date, week_start: u8) -> i64 {
    let (y, _, _) = date.to_ymd();
    let doy = (date.to_days() - Date::from_ymd(y, 1, 1).to_days() + 1) as i64; // 1-based
    let dow = (date.day_of_week() as i64 - week_start as i64).rem_euclid(7);
    (doy - dow + 6) / 7
}

/// Expand a strftime-subset format against a temporal value.
///
/// Supported tokens: `%Y %y %m %d %e %H %I %M %S %f %p %P %b %B %h %a
/// %A %j %s %t %n %u %w %U %W %V %G %%` (`%V` / `%G` are the ISO 8601
/// week and week-year — week 1 contains the first Thursday; `%U` /
/// `%W` count from the first Sunday / Monday), the composites `%F` (%Y-%m-%d), `%T`
/// (%H:%M:%S), `%R` (%H:%M), `%D` (%m/%d/%y), `%r` (%I:%M:%S %p),
/// and the fractional-second forms `%f` (9-digit nanoseconds, chrono
/// semantics), `%.f` (dot + fraction, omitted on whole seconds) and
/// `%.3f` / `%.6f` / `%.9f` (dot + exactly N digits). Numeric tokens
/// accept the padding modifiers `%-` (no pad), `%_` (space pad) and
/// `%0` (zero pad, the default).
///
/// A token needing a component the value doesn't carry (e.g. `%H` on
/// a `Date`), a non-temporal value, or an unparsable string falls
/// back to the value's default rendering — matching the historical
/// behaviour of returning the input untouched.
fn format_temporal(value: &Value, fmt: &str) -> String {
    let Some(parts) = calendar_parts(value) else {
        return default_label(value, None).unwrap_or_default();
    };
    let fallback = |_: ()| default_label(value, None).unwrap_or_default();
    let ymd = parts.date.map(Date::to_ymd);
    let mut out = String::new();
    let mut chars = fmt.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let Some(mut token) = chars.next() else {
            out.push('%');
            break;
        };
        // Padding modifier.
        let mut pad = Pad::Zero;
        match token {
            '-' => {
                pad = Pad::None;
                token = chars.next().unwrap_or('\0');
            }
            '_' => {
                pad = Pad::Space;
                token = chars.next().unwrap_or('\0');
            }
            '0' => {
                token = chars.next().unwrap_or('\0');
            }
            _ => {}
        }
        // Fractional-second forms: `%.f`, `%.3f`, `%.6f`, `%.9f`.
        if token == '.' {
            let mut digits = String::new();
            let mut next = chars.next().unwrap_or('\0');
            while next.is_ascii_digit() {
                digits.push(next);
                next = chars.next().unwrap_or('\0');
            }
            if next != 'f' {
                out.push_str("%.");
                out.push_str(&digits);
                out.push(next);
                continue;
            }
            let Some((_, _, _, ns)) = parts.time else {
                return fallback(());
            };
            match digits.as_str() {
                // `%.f`: dot + fraction, nothing on whole seconds.
                "" => {
                    if ns != 0 {
                        let frac = format!("{ns:09}");
                        out.push('.');
                        out.push_str(frac.trim_end_matches('0'));
                    }
                }
                // `%.3f` / `%.6f` / `%.9f`: dot + exactly N digits.
                d => {
                    let n: usize = match d.parse() {
                        Ok(n) if n <= 9 => n,
                        _ => return fallback(()),
                    };
                    let frac = format!("{ns:09}");
                    out.push('.');
                    out.push_str(&frac[..n]);
                }
            }
            continue;
        }
        // Needs date / needs time — bail to the default rendering if
        // the component is absent.
        let needs_date = matches!(
            token,
            'Y' | 'y'
                | 'm'
                | 'd'
                | 'e'
                | 'b'
                | 'B'
                | 'h'
                | 'a'
                | 'A'
                | 'j'
                | 'F'
                | 'D'
                | 's'
                | 'u'
                | 'w'
                | 'U'
                | 'W'
                | 'V'
                | 'G'
        );
        let needs_time = matches!(
            token,
            'H' | 'I' | 'M' | 'S' | 'f' | 'p' | 'P' | 'T' | 'R' | 'r'
        );
        if (needs_date && ymd.is_none()) || (needs_time && parts.time.is_none()) {
            return fallback(());
        }
        let t = parts.time.unwrap_or((0, 0, 0, 0));
        match token {
            '%' => out.push('%'),
            't' => out.push('\t'),
            'n' => out.push('\n'),
            'Y' => out.push_str(&pad.apply(ymd.unwrap().0 as i64, 4)),
            'y' => out.push_str(&pad.apply(ymd.unwrap().0.rem_euclid(100) as i64, 2)),
            'm' => out.push_str(&pad.apply(ymd.unwrap().1 as i64, 2)),
            'd' => out.push_str(&pad.apply(ymd.unwrap().2 as i64, 2)),
            'e' => out.push_str(&Pad::Space.apply(ymd.unwrap().2 as i64, 2)),
            'b' | 'h' => out.push_str(MONTHS_SHORT[(ymd.unwrap().1 - 1) as usize]),
            'B' => out.push_str(MONTHS_LONG[(ymd.unwrap().1 - 1) as usize]),
            'a' => out.push_str(WEEKDAYS_SHORT[parts.date.unwrap().day_of_week() as usize]),
            'A' => out.push_str(WEEKDAYS_LONG[parts.date.unwrap().day_of_week() as usize]),
            'j' => {
                let (y, _, _) = ymd.unwrap();
                let doy = parts.date.unwrap().to_days() - Date::from_ymd(y, 1, 1).to_days() + 1;
                out.push_str(&pad.apply(doy as i64, 3));
            }
            'H' => out.push_str(&pad.apply(t.0, 2)),
            'I' => {
                let h12 = match t.0 % 12 {
                    0 => 12,
                    r => r,
                };
                out.push_str(&pad.apply(h12, 2));
            }
            'M' => out.push_str(&pad.apply(t.1, 2)),
            'S' => out.push_str(&pad.apply(t.2, 2)),
            'f' => out.push_str(&format!("{:09}", t.3)),
            'p' => out.push_str(if t.0 < 12 { "AM" } else { "PM" }),
            'P' => out.push_str(if t.0 < 12 { "am" } else { "pm" }),
            'F' => {
                let (y, m, d) = ymd.unwrap();
                out.push_str(&format!(
                    "{}-{}-{}",
                    pad.apply(y as i64, 4),
                    pad.apply(m as i64, 2),
                    pad.apply(d as i64, 2)
                ));
            }
            'D' => {
                let (y, m, d) = ymd.unwrap();
                out.push_str(&format!(
                    "{}/{}/{}",
                    pad.apply(m as i64, 2),
                    pad.apply(d as i64, 2),
                    pad.apply(y.rem_euclid(100) as i64, 2)
                ));
            }
            'T' => {
                out.push_str(&format!(
                    "{}:{}:{}",
                    pad.apply(t.0, 2),
                    pad.apply(t.1, 2),
                    pad.apply(t.2, 2)
                ));
            }
            'R' => {
                out.push_str(&format!("{}:{}", pad.apply(t.0, 2), pad.apply(t.1, 2)));
            }
            'r' => {
                let h12 = match t.0 % 12 {
                    0 => 12,
                    r => r,
                };
                out.push_str(&format!(
                    "{}:{}:{} {}",
                    pad.apply(h12, 2),
                    pad.apply(t.1, 2),
                    pad.apply(t.2, 2),
                    if t.0 < 12 { "AM" } else { "PM" }
                ));
            }
            's' => {
                // Unix seconds: midnight for a bare Date.
                let days = parts.date.unwrap().to_days() as i64;
                let secs = days * 86_400 + t.0 * 3_600 + t.1 * 60 + t.2;
                out.push_str(&secs.to_string());
            }
            'u' => out.push_str(&(parts.date.unwrap().day_of_week() as i64 + 1).to_string()),
            'w' => {
                out.push_str(&(((parts.date.unwrap().day_of_week() + 1) % 7) as i64).to_string())
            }
            'U' => out.push_str(&pad.apply(week_of_year(parts.date.unwrap(), 6), 2)),
            'W' => out.push_str(&pad.apply(week_of_year(parts.date.unwrap(), 0), 2)),
            'V' => out.push_str(&pad.apply(iso_week(parts.date.unwrap()).1, 2)),
            'G' => out.push_str(&pad.apply(iso_week(parts.date.unwrap()).0 as i64, 4)),
            // Unknown tokens pass through literally.
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
    out
}

// ─── `{:num …}` — printf subset ──────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
struct PrintfFlags {
    minus: bool,
    plus: bool,
    space: bool,
    zero: bool,
    hash: bool,
    /// `'` — POSIX thousands-grouping flag (`%'d` → `1,234,567`).
    grouping: bool,
    /// `!` — zygos extension: "pretty" rendering. On `%e` / `%E` (and
    /// the exponential branch of `%g` / `%G`) emits Unicode
    /// scientific notation (`1.23 × 10⁴`) instead of `e+04`.
    pretty: bool,
}

/// Format `n` with a printf-style spec string, which may mix literal
/// text with `%[flags][width][.precision][length]conv` verbs.
/// Supported conversions: `d i u o x X f F e E g G s` and `%%`;
/// flags include the POSIX `'` thousands separator and the `!`
/// extension (Unicode scientific notation on the exponential
/// conversions). Negative numbers render with U+2212. Length modifiers
/// (`h`, `hh`, `l`, `ll`, `L`, `z`, `t`, `j`) are accepted and
/// ignored — the value is always an f64 — so `%ld` / `%lld` from
/// printf muscle memory render as `%d` rather than failing. Returns
/// `Err(())` on an unsupported conversion or malformed verb.
fn printf_format(spec: &str, n: f64) -> Result<String, ()> {
    let b = spec.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'%' {
            out.push(b[i] as char);
            i += 1;
            continue;
        }
        i += 1;
        let mut flags = PrintfFlags::default();
        while i < b.len() {
            match b[i] {
                b'-' => flags.minus = true,
                b'+' => flags.plus = true,
                b' ' => flags.space = true,
                b'0' => flags.zero = true,
                b'#' => flags.hash = true,
                // `,` is not a POSIX flag but means grouping in
                // D3-style specs; accepting it costs nothing.
                b'\'' | b',' => flags.grouping = true,
                b'!' => flags.pretty = true,
                _ => break,
            }
            i += 1;
        }
        let mut width = 0usize;
        while i < b.len() && b[i].is_ascii_digit() {
            width = width * 10 + (b[i] - b'0') as usize;
            i += 1;
        }
        let mut precision = None;
        if i < b.len() && b[i] == b'.' {
            i += 1;
            let mut p = 0usize;
            while i < b.len() && b[i].is_ascii_digit() {
                p = p * 10 + (b[i] - b'0') as usize;
                i += 1;
            }
            precision = Some(p);
        }
        // Length modifiers — accepted, ignored (the value is an f64).
        while i < b.len() && matches!(b[i], b'h' | b'l' | b'L' | b'z' | b't' | b'j' | b'q') {
            i += 1;
        }
        if i >= b.len() {
            return Err(());
        }
        let conv = b[i] as char;
        i += 1;
        if conv == '%' {
            out.push('%');
            continue;
        }
        let mut body = render_conversion(n, conv, flags, precision)?;
        if flags.grouping && matches!(conv, 'd' | 'i' | 'u' | 'f' | 'F' | 'g' | 'G') {
            body = group_thousands(&body);
        }
        out.push_str(&pad(body, width, flags));
    }
    Ok(out)
}

/// Pad a rendered conversion body to `width`, honouring alignment and
/// zero-padding (which inserts after any sign / prefix).
fn pad(body: String, width: usize, flags: PrintfFlags) -> String {
    let len = body.chars().count();
    if len >= width {
        return body;
    }
    let fill = width - len;
    if flags.minus {
        let mut s = body;
        s.extend(std::iter::repeat_n(' ', fill));
        s
    } else if flags.zero {
        // Zero-fill goes after any sign and any "0x"/"0X" prefix.
        let mut split = 0;
        if let Some(first) = body.chars().next() {
            if matches!(first, '-' | '+' | ' ' | '\u{2212}') {
                split = first.len_utf8();
            }
        }
        if body[split..].starts_with("0x") || body[split..].starts_with("0X") {
            split += 2;
        }
        let (head, tail) = body.split_at(split);
        let mut s = String::with_capacity(width);
        s.push_str(head);
        s.extend(std::iter::repeat_n('0', fill));
        s.push_str(tail);
        s
    } else {
        let mut s = String::with_capacity(width);
        s.extend(std::iter::repeat_n(' ', fill));
        s.push_str(&body);
        s
    }
}

fn render_conversion(
    n: f64,
    conv: char,
    flags: PrintfFlags,
    precision: Option<usize>,
) -> Result<String, ()> {
    let sign = if n.is_sign_negative() {
        // U+2212 — labels are presentation.
        "\u{2212}"
    } else if flags.plus {
        "+"
    } else if flags.space {
        " "
    } else {
        ""
    };
    match conv {
        'd' | 'i' | 'u' | 'o' | 'x' | 'X' => {
            // Integer conversions truncate toward zero, as in C.
            let v = n as i64;
            let isign = if conv == 'u' {
                // %u: no sign — negatives wrap as in C.
                ""
            } else if v < 0 {
                "\u{2212}"
            } else if flags.plus {
                "+"
            } else if flags.space {
                " "
            } else {
                ""
            };
            let digits = match conv {
                'u' => format!("{}", v as u64),
                'd' | 'i' => format!("{}", v.unsigned_abs()),
                'o' => format!("{:o}", v.unsigned_abs()),
                'x' => format!("{:x}", v.unsigned_abs()),
                'X' => format!("{:X}", v.unsigned_abs()),
                _ => unreachable!(),
            };
            let digits = match precision {
                Some(p) if digits.len() < p => {
                    format!("{}{}", "0".repeat(p - digits.len()), digits)
                }
                _ => digits,
            };
            let prefix = match (conv, flags.hash) {
                ('o', true) if !digits.starts_with('0') => "0",
                ('x', true) if v != 0 => "0x",
                ('X', true) if v != 0 => "0X",
                _ => "",
            };
            Ok(format!("{isign}{prefix}{digits}"))
        }
        'f' | 'F' => {
            let p = precision.unwrap_or(6);
            Ok(format!("{sign}{:.p$}", n.abs()))
        }
        'e' | 'E' => {
            let p = precision.unwrap_or(6);
            let body = if flags.pretty {
                pretty_sci(n.abs(), p, false)
            } else {
                exp_notation(n.abs(), p, conv == 'E')
            };
            Ok(format!("{sign}{body}"))
        }
        'g' | 'G' => {
            let p = precision.unwrap_or(6).max(1);
            Ok(format!(
                "{sign}{}",
                g_notation(n.abs(), p, conv == 'G', flags.hash, flags.pretty)
            ))
        }
        's' => {
            // The value is always an f64 here, so `%s` stringifies it;
            // precision truncates, as for strings in C.
            let s = format_number(n, None);
            Ok(match precision {
                Some(p) => s.chars().take(p).collect(),
                None => s,
            })
        }
        'S' => {
            // zygos extension: SI-prefixed notation (`1.23M`).
            // Precision is significant digits, defaulting to 3.
            Ok(format!(
                "{sign}{}",
                si_notation(n.abs(), precision.unwrap_or(3))
            ))
        }
        _ => Err(()),
    }
}

/// Insert `,` every three digits in the integer part of a rendered
/// number (POSIX `'` flag). The integer part runs up to the first
/// `.`, `e`/`E` (exponent) or non-digit; anything else — signs,
/// `0x` prefixes, `inf` / `nan` — is left alone.
fn group_thousands(rendered: &str) -> String {
    let int_end = rendered.find(['.', 'e', 'E']).unwrap_or(rendered.len());
    let (int_part, rest) = rendered.split_at(int_end);
    let digit_start = int_part
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(int_part.len());
    let (head, digits) = int_part.split_at(digit_start);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return rendered.to_string();
    }
    let mut grouped = String::with_capacity(rendered.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{head}{grouped}{rest}")
}

/// Unicode scientific notation (`!` flag): `1.23 × 10⁴` — mantissa,
/// U+00D7, and a superscript exponent — with no markup, so it renders
/// in any writer. An exponent of 0 (or 0 itself) collapses to the
/// plain mantissa, since `× 10⁰` carries no information. `trim`
/// strips trailing mantissa zeros (the `%g` behaviour without `#`).
fn pretty_sci(n: f64, precision: usize, trim: bool) -> String {
    if n == 0.0 {
        return format!("{:.precision$}", 0.0);
    }
    let s = format!("{n:.precision$e}");
    let (mantissa, e) = s.split_once('e').unwrap_or((&s, "0"));
    let exp: i32 = e.parse().unwrap_or(0);
    let mantissa = if trim {
        mantissa.trim_end_matches('0').trim_end_matches('.')
    } else {
        mantissa
    };
    if exp == 0 {
        mantissa.to_string()
    } else {
        format!("{mantissa} \u{d7} 10{}", superscript(exp))
    }
}

/// Render an integer as Unicode superscripts (U+207B for the sign).
fn superscript(exp: i32) -> String {
    const DIGITS: [char; 10] = [
        '\u{2070}', '\u{b9}', '\u{b2}', '\u{b3}', '\u{2074}', '\u{2075}', '\u{2076}', '\u{2077}',
        '\u{2078}', '\u{2079}',
    ];
    let mut s = String::new();
    if exp < 0 {
        s.push('\u{207b}');
    }
    for c in exp.unsigned_abs().to_string().chars() {
        s.push(DIGITS[c.to_digit(10).unwrap_or(0) as usize]);
    }
    s
}

/// SI-prefixed notation (`%S`, a zygos extension — printf has no SI
/// conversion and `%s` is taken): `sig` significant digits, prefix
/// from `y z a f p n µ m` (none) `k M G T P E Z Y`, trailing zeros
/// stripped. `{:num %S}` on 1_234_567 → `1.23M`, on 0.0012 → `1.2m`.
fn si_notation(n: f64, sig: usize) -> String {
    const PREFIXES: [&str; 17] = [
        "y", "z", "a", "f", "p", "n", "\u{b5}", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y",
    ];
    if n == 0.0 {
        return "0".to_string();
    }
    let sig = sig.max(1);
    let exp10 = n.log10().floor() as i32;
    let mut si_exp = (exp10.div_euclid(3) * 3).clamp(-24, 24);
    let mut mantissa = n / 10f64.powi(si_exp);
    let decimals = (sig as i32 - 1 - mantissa.log10().floor() as i32).max(0) as usize;
    let mut s = format!("{mantissa:.decimals$}");
    // Rounding can push 999.9… to 1000 — step up a prefix.
    if s.starts_with("1000") && si_exp < 24 {
        si_exp += 3;
        mantissa /= 1_000.0;
        let decimals = (sig as i32 - 1).max(0) as usize;
        s = format!("{mantissa:.decimals$}");
    }
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    format!("{s}{}", PREFIXES[(si_exp / 3 + 8) as usize])
}

/// printf `%e`: one digit before the point, signed two-digit exponent.
fn exp_notation(n: f64, precision: usize, upper: bool) -> String {
    let s = format!("{n:.precision$e}");
    let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let e = if upper { 'E' } else { 'e' };
    format!(
        "{mantissa}{e}{}{:02}",
        if exp < 0 { '-' } else { '+' },
        exp.abs()
    )
}

/// printf `%g`: `%e` when the exponent falls outside `[-4, precision)`,
/// `%f` otherwise, trailing zeros stripped unless `#`. With `pretty`,
/// the exponential branch uses Unicode scientific notation.
fn g_notation(n: f64, precision: usize, upper: bool, hash: bool, pretty: bool) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    // Exponent the value would carry in %e form.
    let exp = format!("{n:.prec$e}", prec = precision - 1)
        .split_once('e')
        .and_then(|(_, e)| e.parse::<i32>().ok())
        .unwrap_or(0);
    let mut s = if exp < -4 || exp >= precision as i32 {
        if pretty {
            pretty_sci(n, precision - 1, !hash)
        } else {
            exp_notation(n, precision - 1, upper)
        }
    } else {
        let decimals = (precision as i32 - 1 - exp).max(0) as usize;
        format!("{n:.decimals$}")
    };
    if !hash {
        // Strip trailing zeros (and a dangling point) in the mantissa.
        let e_pos = s.find(['e', 'E']);
        let (mantissa, exponent) = match e_pos {
            Some(i) => s.split_at(i),
            None => (&s[..], ""),
        };
        let mut m = mantissa;
        if m.contains('.') {
            m = m.trim_end_matches('0');
            m = m.trim_end_matches('.');
        }
        s = format!("{m}{exponent}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Duration;
    use std::sync::Arc;

    fn s(v: &str) -> Value {
        Value::String(Arc::from(v))
    }

    fn labels(spec: &LabelSpec, breaks: &[Value]) -> Vec<String> {
        spec.labels(breaks)
            .into_iter()
            .map(|l| l.unwrap_or("<suppressed>".to_string()))
            .collect()
    }

    // ─── LabelSpec resolution ───

    #[test]
    fn rename_wins_over_template() {
        let mut spec = LabelSpec::new();
        spec.rename(s("A"), "Alpha").set_template("Category {}");
        let breaks = [s("A"), s("B"), s("C")];
        assert_eq!(
            labels(&spec, &breaks),
            ["Alpha", "Category B", "Category C"]
        );
    }

    #[test]
    fn suppress_drops_label() {
        let mut spec = LabelSpec::new();
        spec.suppress(s("internal"));
        let breaks = [s("internal"), s("other")];
        assert_eq!(labels(&spec, &breaks), ["<suppressed>", "other"]);
    }

    #[test]
    fn re_adding_key_replaces_entry() {
        let mut spec = LabelSpec::new();
        spec.rename(s("A"), "one");
        spec.rename(s("A"), "two");
        assert_eq!(spec.rename_len(), 1);
        assert_eq!(spec.label(&s("A")), Some("two".to_string()));
    }

    #[test]
    fn numeric_rename_matches_break_value_by_key_eq() {
        // 25 and 25.0 are the same Number bits; NaN keys match too.
        let mut spec = LabelSpec::new();
        spec.rename(Value::Number(25.0), "twenty-five");
        assert_eq!(
            spec.label(&Value::Number(25.0)),
            Some("twenty-five".to_string())
        );
        assert_eq!(spec.label(&Value::Number(26.0)), Some("26".to_string()));
    }

    #[test]
    fn renames_iterates_in_insertion_order() {
        let mut spec = LabelSpec::new();
        spec.rename(s("A"), "Alpha").suppress(Value::Number(1.0));
        let entries: Vec<(String, Option<&str>)> = spec
            .renames()
            .map(|(v, l)| (default_label(v, None).unwrap(), l))
            .collect();
        assert_eq!(
            entries,
            [("A".to_string(), Some("Alpha")), ("1".to_string(), None)]
        );
    }

    #[test]
    fn equality_is_structural() {
        let mut a = LabelSpec::new();
        a.rename(Value::Number(0.0), "zero").set_template("{} u");
        let mut b = LabelSpec::new();
        b.rename(Value::Number(-0.0), "zero").set_template("{} u");
        assert_eq!(a, b);
        b.set_template("{}");
        assert_ne!(a, b);
        assert_ne!(LabelSpec::new(), a);
    }

    #[test]
    fn null_is_suppressed_unless_renamed() {
        let mut spec = LabelSpec::new();
        assert_eq!(spec.label(&Value::Null), None);
        spec.rename(Value::Null, "(missing)");
        assert_eq!(spec.label(&Value::Null), Some("(missing)".to_string()));
    }

    // ─── Templates ───

    #[test]
    fn plain_placeholder() {
        let mut spec = LabelSpec::new();
        spec.set_template("{} units");
        let breaks = [Value::Number(0.0), Value::Number(25.0)];
        assert_eq!(labels(&spec, &breaks), ["0 units", "25 units"]);
    }

    #[test]
    fn case_placeholders() {
        let mut spec = LabelSpec::new();
        spec.set_template("{:UPPER}");
        assert_eq!(spec.label(&s("north")), Some("NORTH".to_string()));
        let mut spec = LabelSpec::new();
        spec.set_template("{:lower}");
        assert_eq!(spec.label(&s("HELLO")), Some("hello".to_string()));
        let mut spec = LabelSpec::new();
        spec.set_template("Region: {:Title}");
        assert_eq!(
            spec.label(&s("us east")),
            Some("Region: Us East".to_string())
        );
    }

    #[test]
    fn literal_template_repeats() {
        let mut spec = LabelSpec::new();
        spec.set_template("Constant Label");
        let breaks = [s("A"), s("B")];
        assert_eq!(labels(&spec, &breaks), ["Constant Label", "Constant Label"]);
    }

    #[test]
    fn keywords_are_case_insensitive() {
        let t = Template::parse("{:NUM %d}");
        assert_eq!(t.format(&Value::Number(42.0), None), Some("42".to_string()));
        let t = Template::parse("{:Time %Y}");
        let d = Value::Date(Date::from_ymd(2024, 6, 23).to_days());
        assert_eq!(t.format(&d, None), Some("2024".to_string()));
        let t = Template::parse("{:upper}");
        assert_eq!(t.format(&s("abc"), None), Some("ABC".to_string()));
        let t = Template::parse("{:title}");
        assert_eq!(
            t.format(&s("hello world"), None),
            Some("Hello World".to_string())
        );
        let t = Template::parse("{:LOWER}");
        assert_eq!(t.format(&s("ABC"), None), Some("abc".to_string()));
    }

    #[test]
    fn num_body_requires_percent_introducer() {
        // A malformed `{:num}` spec degrades to plain — the break
        // still shows its value, not the spec text.
        let t = Template::parse("{:num .2f}");
        assert_eq!(t.format(&Value::Number(1.5), None), Some("1.5".to_string()));
        let t = Template::parse("{:num 'd}");
        assert_eq!(t.format(&Value::Number(1.5), None), Some("1.5".to_string()));
    }

    #[test]
    fn abbr_placeholder() {
        let t = Template::parse("{:abbr 10}");
        assert_eq!(
            t.format(&s("new hampshire"), None),
            Some("new hamps\u{2026}".to_string())
        );
        // Short values pass through; numbers render then truncate.
        assert_eq!(t.format(&s("iowa"), None), Some("iowa".to_string()));
        let t = Template::parse("{:abbr 4}");
        assert_eq!(
            t.format(&Value::Number(123456.0), None),
            Some("123\u{2026}".to_string())
        );
    }

    #[test]
    fn bytes_placeholder() {
        let fmt = |spec: &str, v: f64| Template::parse(spec).format(&Value::Number(v), None);
        assert_eq!(fmt("{:bytes}", 1536.0), Some("1.5 KiB".to_string()));
        assert_eq!(fmt("{:bytes}", 2_000_000.0), Some("1.91 MiB".to_string()));
        assert_eq!(fmt("{:bytes}", 512.0), Some("512 B".to_string()));
        assert_eq!(fmt("{:bytes 2}", 2_000_000.0), Some("1.9 MiB".to_string()));
        assert_eq!(fmt("{:bytes}", 0.0), Some("0 B".to_string()));
        assert_eq!(
            fmt("{:bytes}", -1536.0),
            Some("\u{2212}1.5 KiB".to_string())
        );
    }

    #[test]
    fn duration_placeholder() {
        // Numbers read as seconds.
        let t = Template::parse("{:duration}");
        assert_eq!(
            t.format(&Value::Number(5400.0), None),
            Some("1h 30m".to_string())
        );
        assert_eq!(
            t.format(&Value::Number(90.2), None),
            Some("1m 30s".to_string())
        );
        assert_eq!(
            t.format(&Value::Number(0.15), None),
            Some("150ms".to_string())
        );
        assert_eq!(t.format(&Value::Number(0.0), None), Some("0s".to_string()));
        // Duration values (µs) work directly.
        let t = Template::parse("{:duration}");
        let d = Value::Duration(crate::value::Duration::from_seconds(93784).to_micros());
        assert_eq!(t.format(&d, None), Some("1d 2h".to_string()));
        // Negative: U+2212.
        assert_eq!(
            t.format(&Value::Number(-90.0), None),
            Some("\u{2212}1m 30s".to_string())
        );
        // Explicit units for bare numbers.
        let t = Template::parse("{:duration h}");
        assert_eq!(
            t.format(&Value::Number(1.5), None),
            Some("1h 30m".to_string())
        );
        let t = Template::parse("{:duration ms}");
        assert_eq!(
            t.format(&Value::Number(90_000.0), None),
            Some("1m 30s".to_string())
        );
        let t = Template::parse("{:duration d}");
        assert_eq!(t.format(&Value::Number(2.0), None), Some("2d".to_string()));
        // Typed durations ignore the unit (they're already µs).
        let t = Template::parse("{:duration h}");
        let d = Value::Duration(crate::value::Duration::from_seconds(5400).to_micros());
        assert_eq!(t.format(&d, None), Some("1h 30m".to_string()));
    }

    #[test]
    fn duration_part_count() {
        // Default is two parts; trailing units are dropped.
        let t = Template::parse("{:duration}");
        assert_eq!(
            t.format(&Value::Number(9015.5), None),
            Some("2h 30m".to_string())
        );
        // N parts, largest first.
        let t = Template::parse("{:duration 3}");
        assert_eq!(
            t.format(&Value::Number(9015.5), None),
            Some("2h 30m 15s".to_string())
        );
        let t = Template::parse("{:duration 6}");
        assert_eq!(
            t.format(&Value::Number(9015.5), None),
            Some("2h 30m 15s 500ms".to_string())
        );
        // Part count combines with a unit, in either order.
        let t = Template::parse("{:duration h 3}");
        assert_eq!(
            t.format(&Value::Number(2.5043), None),
            Some("2h 30m 15s".to_string())
        );
        let t = Template::parse("{:duration 3 h}");
        assert_eq!(
            t.format(&Value::Number(2.5043), None),
            Some("2h 30m 15s".to_string())
        );
        // Typed durations respect the part count.
        let t = Template::parse("{:duration 4}");
        let d = Value::Duration(crate::value::Duration::from_seconds(9015).to_micros());
        assert_eq!(t.format(&d, None), Some("2h 30m 15s".to_string()));
        // One part keeps the largest unit only.
        let t = Template::parse("{:duration 1}");
        assert_eq!(
            t.format(&Value::Number(9015.5), None),
            Some("2h".to_string())
        );
    }

    #[test]
    fn num_align_placeholder() {
        const FS: char = FIGURE_SPACE;
        let aligned = |template: &str, breaks: &[f64]| {
            let mut spec = LabelSpec::new();
            spec.set_template(template);
            spec.labels(&breaks.iter().map(|n| Value::Number(*n)).collect::<Vec<_>>())
        };
        // Bare: default rendering, decimal points aligned across the set.
        let labels = aligned("{:num_align}", &[1.5, 42.0, 300.25]);
        assert_eq!(
            labels,
            vec![
                Some(format!("{FS}{FS}1.5{FS}")),
                Some(format!("{FS}42")),
                Some("300.25".to_string()),
            ]
        );
        // With a printf body.
        let labels = aligned("{:num_align %.2f}", &[1.5, 42.0]);
        assert_eq!(
            labels,
            vec![Some(format!("{FS}1.50")), Some("42.00".to_string())]
        );
        // The U+2212 sign counts towards the integer width.
        let labels = aligned("{:num_align}", &[-1.5, 10.25]);
        assert_eq!(
            labels,
            vec![Some(format!("\u{2212}1.5{FS}")), Some("10.25".to_string())]
        );
        // A body without the `%` introducer degrades to `{}` (and is
        // not marked for alignment).
        let labels = aligned("{:num_align 2f}", &[1.5, 42.0]);
        assert_eq!(
            labels,
            vec![Some("1.5".to_string()), Some("42".to_string())]
        );
        // Renamed labels are text — excluded from the pass.
        let mut spec = LabelSpec::new();
        spec.set_template("{:num_align}");
        spec.rename(Value::Number(42.0), "answer");
        let labels = spec.labels(&[
            Value::Number(1.5),
            Value::Number(42.0),
            Value::Number(300.25),
        ]);
        assert_eq!(
            labels,
            vec![
                Some(format!("{FS}{FS}1.5{FS}")),
                Some("answer".to_string()),
                Some("300.25".to_string()),
            ]
        );
        // `label()` is value-local: nothing to align to, no padding.
        let mut spec = LabelSpec::new();
        spec.set_template("{:num_align}");
        assert_eq!(spec.label(&Value::Number(1.5)), Some("1.5".to_string()));
    }

    #[test]
    fn decimal_align_skips_non_numeric() {
        // Units, exponents and text are left alone.
        let mut labels = vec![
            Some("1.5 KiB".to_string()),
            Some("1e4".to_string()),
            Some("n/a".to_string()),
            None,
        ];
        decimal_align(&mut labels);
        assert_eq!(
            labels,
            vec![
                Some("1.5 KiB".to_string()),
                Some("1e4".to_string()),
                Some("n/a".to_string()),
                None,
            ]
        );
        // A lone numeric label still gets padded to itself — a no-op.
        let mut labels = vec![Some("12.5".to_string())];
        decimal_align(&mut labels);
        assert_eq!(labels, vec![Some("12.5".to_string())]);
    }

    #[test]
    fn ordinal_placeholder() {
        let t = Template::parse("{:ordinal}");
        let cases = [
            (1.0, "1st"),
            (2.0, "2nd"),
            (3.0, "3rd"),
            (4.0, "4th"),
            (11.0, "11th"),
            (12.0, "12th"),
            (13.0, "13th"),
            (21.0, "21st"),
            (112.0, "112th"),
        ];
        for (v, want) in cases {
            assert_eq!(
                t.format(&Value::Number(v), None),
                Some(want.to_string()),
                "ordinal of {v}"
            );
        }
        // Non-integers fall back to the plain rendering.
        assert_eq!(t.format(&Value::Number(1.5), None), Some("1.5".to_string()));
    }

    #[test]
    fn unknown_placeholder_degrades_to_plain() {
        let mut spec = LabelSpec::new();
        spec.set_template("{:future %q}");
        assert_eq!(spec.label(&s("x")), Some("x".to_string()));
    }

    #[test]
    fn unmatched_brace_is_literal() {
        let t = Template::parse("a { b");
        assert_eq!(t.format(&s("v"), None), Some("a { b".to_string()));
    }

    // ─── Shared precision ───

    #[test]
    fn shared_precision_mixed_decimals() {
        let breaks = [
            Value::Number(54.5),
            Value::Number(55.0),
            Value::Number(55.5),
        ];
        assert_eq!(labels(&LabelSpec::new(), &breaks), ["54.5", "55.0", "55.5"]);
    }

    #[test]
    fn shared_precision_all_integers() {
        let breaks = [
            Value::Number(10.0),
            Value::Number(20.0),
            Value::Number(30.0),
        ];
        assert_eq!(labels(&LabelSpec::new(), &breaks), ["10", "20", "30"]);
    }

    #[test]
    fn shared_precision_small_steps() {
        let breaks = [Value::Number(0.0), Value::Number(0.05), Value::Number(0.1)];
        assert_eq!(labels(&LabelSpec::new(), &breaks), ["0.00", "0.05", "0.10"]);
    }

    #[test]
    fn re_break_rederives_precision() {
        // The same spec re-rendered against a new break set adapts —
        // the property the whole design exists for.
        let spec = LabelSpec::new();
        let wide = [
            Value::Number(0.0),
            Value::Number(50.0),
            Value::Number(100.0),
        ];
        assert_eq!(labels(&spec, &wide), ["0", "50", "100"]);
        let fine = [Value::Number(0.0), Value::Number(0.5), Value::Number(1.0)];
        assert_eq!(labels(&spec, &fine), ["0.0", "0.5", "1.0"]);
    }

    // ─── `{:time …}` ───

    #[test]
    fn time_placeholder_on_typed_values() {
        let t = Template::parse("{:time %b %Y}");
        let jan = Value::Date(Date::from_ymd(2024, 1, 15).to_days());
        assert_eq!(t.format(&jan, None), Some("Jan 2024".to_string()));

        let t = Template::parse("{:time %Y-%m-%d %H:%M}");
        let dt =
            Value::DateTime(DateTime::from_ymd_hms_micros(2024, 1, 15, 10, 30, 0, 0).to_micros());
        assert_eq!(t.format(&dt, None), Some("2024-01-15 10:30".to_string()));

        let t = Template::parse("{:time %I:%M %p}");
        let tod = Value::Time(Time::from_hms_nanos(15, 4, 0, 0).to_nanos());
        assert_eq!(t.format(&tod, None), Some("03:04 PM".to_string()));
    }

    #[test]
    fn time_placeholder_on_iso_strings() {
        let t = Template::parse("{:time %b %Y}");
        assert_eq!(
            t.format(&s("2024-02-15"), None),
            Some("Feb 2024".to_string())
        );
        let t = Template::parse("{:time %H:%M}");
        assert_eq!(
            t.format(&s("2024-01-15T10:30:00"), None),
            Some("10:30".to_string())
        );
    }

    #[test]
    fn time_composite_tokens() {
        let dt = Value::DateTime(DateTime::from_ymd_hms_micros(2024, 3, 5, 9, 7, 3, 0).to_micros());
        let fmt = |f: &str| Template::parse(f).format(&dt, None);
        assert_eq!(fmt("{:time %F}"), Some("2024-03-05".to_string()));
        assert_eq!(fmt("{:time %T}"), Some("09:07:03".to_string()));
        assert_eq!(fmt("{:time %R}"), Some("09:07".to_string()));
        assert_eq!(fmt("{:time %D}"), Some("03/05/24".to_string()));
        assert_eq!(fmt("{:time %r}"), Some("09:07:03 AM".to_string()));
        assert_eq!(
            fmt("{:time %F %T}"),
            Some("2024-03-05 09:07:03".to_string())
        );
        assert_eq!(fmt("{:time %h}"), Some("Mar".to_string()));
        assert_eq!(fmt("{:time %p %P}"), Some("AM am".to_string()));
        assert_eq!(fmt("{:time %s}"), Some("1709629623".to_string()));
    }

    #[test]
    fn time_padding_modifiers() {
        let d = Value::Date(Date::from_ymd(2024, 3, 5).to_days());
        let fmt = |f: &str| Template::parse(f).format(&d, None);
        assert_eq!(fmt("{:time %-d}"), Some("5".to_string()));
        assert_eq!(fmt("{:time %_d}"), Some(" 5".to_string()));
        assert_eq!(fmt("{:time %0d}"), Some("05".to_string()));
        assert_eq!(fmt("{:time %-m/%-d/%y}"), Some("3/5/24".to_string()));
    }

    #[test]
    fn time_fractional_seconds() {
        // chrono semantics: %f is 9-digit nanoseconds.
        let dt =
            Value::DateTime(DateTime::from_ymd_hms_micros(2024, 1, 1, 0, 0, 5, 26_490).to_micros());
        let fmt = |f: &str| Template::parse(f).format(&dt, None);
        assert_eq!(fmt("{:time %S.%f}"), Some("05.026490000".to_string()));
        assert_eq!(fmt("{:time %S%.f}"), Some("05.02649".to_string()));
        assert_eq!(fmt("{:time %S%.3f}"), Some("05.026".to_string()));
        assert_eq!(fmt("{:time %S%.9f}"), Some("05.026490000".to_string()));
        // Whole seconds: `%.f` prints nothing.
        let whole =
            Value::DateTime(DateTime::from_ymd_hms_micros(2024, 1, 1, 0, 0, 5, 0).to_micros());
        assert_eq!(
            Template::parse("{:time %S%.f}").format(&whole, None),
            Some("05".to_string())
        );
        // Time (ns) keeps full precision.
        let t = Value::Time(Time::from_hms_nanos(0, 0, 5, 123_456_789).to_nanos());
        assert_eq!(
            Template::parse("{:time %S.%f}").format(&t, None),
            Some("05.123456789".to_string())
        );
    }

    #[test]
    fn time_week_tokens() {
        let fmt = |y, m, d, f: &str| {
            Template::parse(f)
                .format(&Value::Date(Date::from_ymd(y, m, d).to_days()), None)
                .unwrap()
        };
        // Weekday numbers: 2024-03-05 is a Tuesday.
        assert_eq!(fmt(2024, 3, 5, "{:time %u}"), "2");
        assert_eq!(fmt(2024, 3, 3, "{:time %u}"), "7"); // Sunday
        assert_eq!(fmt(2024, 3, 3, "{:time %w}"), "0");
        assert_eq!(fmt(2024, 3, 5, "{:time %w}"), "2");

        // ISO week: 2024-01-01 is the Monday of ISO week 1.
        assert_eq!(fmt(2024, 1, 1, "{:time %G-W%V}"), "2024-W01");
        // 2021-01-01 (Fri) is ISO week 53 of *2020*.
        assert_eq!(fmt(2021, 1, 1, "{:time %G-W%V}"), "2020-W53");
        // 2019-12-30 (Mon) is ISO week 1 of *2020*.
        assert_eq!(fmt(2019, 12, 30, "{:time %G-W%V}"), "2020-W01");
        // Mid-year spot check: 2024-03-05 is ISO week 10.
        assert_eq!(fmt(2024, 3, 5, "{:time %V}"), "10");

        // %U (Sunday-start): 2024-01-01..06 is week 00, first Sunday
        // Jan 7 opens week 01.
        assert_eq!(fmt(2024, 1, 1, "{:time %U}"), "00");
        assert_eq!(fmt(2024, 1, 7, "{:time %U}"), "01");
        // %W (Monday-start): 2024-01-01 is itself the first Monday.
        assert_eq!(fmt(2024, 1, 1, "{:time %W}"), "01");
        // 2023-01-01 (Sun) is week 00 under %W, week 01 under %U
        // (it *is* the first Sunday).
        assert_eq!(fmt(2023, 1, 1, "{:time %W}"), "00");
        assert_eq!(fmt(2023, 1, 1, "{:time %U}"), "01");
    }

    #[test]
    fn time_placeholder_falls_back() {
        let t = Template::parse("{:time %Y-%m-%d}");
        assert_eq!(
            t.format(&s("not-a-date"), None),
            Some("not-a-date".to_string())
        );
        // %H on a Date has no time component → default rendering.
        let t = Template::parse("{:time %H}");
        let d = Value::Date(Date::from_ymd(2024, 1, 15).to_days());
        assert_eq!(t.format(&d, None), Some("2024-01-15".to_string()));
    }

    // ─── `{:num …}` ───

    #[test]
    fn num_fixed_and_width() {
        let t = Template::parse("${:num %.2f}");
        assert_eq!(
            t.format(&Value::Number(25.5), None),
            Some("$25.50".to_string())
        );
        let t = Template::parse("{:num %8.2f}");
        assert_eq!(
            t.format(&Value::Number(3.5), None),
            Some("    3.50".to_string())
        );
        let t = Template::parse("{:num %-8.2f|}");
        assert_eq!(
            t.format(&Value::Number(3.5), None),
            Some("3.50    |".to_string())
        );
        let t = Template::parse("{:num %08.2f}");
        assert_eq!(
            t.format(&Value::Number(-3.5), None),
            Some("\u{2212}0003.50".to_string())
        );
        let t = Template::parse("{:num %+.1f}");
        assert_eq!(
            t.format(&Value::Number(3.5), None),
            Some("+3.5".to_string())
        );
    }

    #[test]
    fn num_integer_conversions() {
        let t = Template::parse("{:num %d}");
        assert_eq!(t.format(&Value::Number(42.9), None), Some("42".to_string()));
        let t = Template::parse("{:num %#x}");
        assert_eq!(
            t.format(&Value::Number(255.0), None),
            Some("0xff".to_string())
        );
        let t = Template::parse("{:num %05d}");
        assert_eq!(
            t.format(&Value::Number(42.0), None),
            Some("00042".to_string())
        );
    }

    #[test]
    fn num_scientific() {
        let t = Template::parse("{:num %.2e}");
        assert_eq!(
            t.format(&Value::Number(1234.5), None),
            Some("1.23e+03".to_string())
        );
        let t = Template::parse("{:num %g}");
        assert_eq!(t.format(&Value::Number(0.5), None), Some("0.5".to_string()));
        let t = Template::parse("{:num %g}");
        assert_eq!(
            t.format(&Value::Number(123456.0), None),
            Some("123456".to_string())
        );
        let t = Template::parse("{:num %G}");
        assert_eq!(
            t.format(&Value::Number(1234567.0), None),
            Some("1.23457E+06".to_string())
        );
    }

    #[test]
    fn num_length_modifiers_ignored() {
        let t = Template::parse("{:num %ld}");
        assert_eq!(t.format(&Value::Number(42.9), None), Some("42".to_string()));
        let t = Template::parse("{:num %lld}");
        assert_eq!(t.format(&Value::Number(42.9), None), Some("42".to_string()));
        let t = Template::parse("{:num %.2lf}");
        assert_eq!(
            t.format(&Value::Number(3.5), None),
            Some("3.50".to_string())
        );
    }

    #[test]
    fn num_unsigned_and_string() {
        let t = Template::parse("{:num %u}");
        assert_eq!(t.format(&Value::Number(42.9), None), Some("42".to_string()));
        let t = Template::parse("{:num %s}");
        assert_eq!(
            t.format(&Value::Number(42.5), None),
            Some("42.5".to_string())
        );
        let t = Template::parse("{:num %.2s}");
        assert_eq!(t.format(&Value::Number(42.5), None), Some("42".to_string()));
    }

    #[test]
    fn num_thousands_grouping() {
        let t = Template::parse("{:num %'d}");
        assert_eq!(
            t.format(&Value::Number(1234567.0), None),
            Some("1,234,567".to_string())
        );
        let t = Template::parse("{:num %',.2f}");
        assert_eq!(
            t.format(&Value::Number(1234.5), None),
            Some("1,234.50".to_string())
        );
        // Sign stays in front; short digit runs are untouched.
        let t = Template::parse("{:num %'d}");
        assert_eq!(
            t.format(&Value::Number(-1234.0), None),
            Some("\u{2212}1,234".to_string())
        );
        let t = Template::parse("{:num %'d}");
        assert_eq!(
            t.format(&Value::Number(123.0), None),
            Some("123".to_string())
        );
        // No grouping for hex / octal.
        let t = Template::parse("{:num %'x}");
        assert_eq!(
            t.format(&Value::Number(65535.0), None),
            Some("ffff".to_string())
        );
    }

    #[test]
    fn num_unicode_minus() {
        let t = Template::parse("{:num %.1f}");
        assert_eq!(
            t.format(&Value::Number(-1.5), None),
            Some("\u{2212}1.5".to_string())
        );
        let t = Template::parse("{:num %d}");
        assert_eq!(
            t.format(&Value::Number(-42.0), None),
            Some("\u{2212}42".to_string())
        );
        // Default rendering too, via plain {}.
        assert_eq!(
            Template::parse("{}").format(&Value::Number(-2.5), None),
            Some("\u{2212}2.5".to_string())
        );
        // Forced sign still uses ASCII + for positives.
        let t = Template::parse("{:num %+.1f}");
        assert_eq!(
            t.format(&Value::Number(1.5), None),
            Some("+1.5".to_string())
        );
    }

    #[test]
    fn num_pretty_scientific() {
        let t = Template::parse("{:num %!.2e}");
        assert_eq!(
            t.format(&Value::Number(1234.5), None),
            Some("1.23 \u{d7} 10\u{b3}".to_string())
        );
        let t = Template::parse("{:num %!.1e}");
        assert_eq!(
            t.format(&Value::Number(0.00012), None),
            Some("1.2 \u{d7} 10\u{207b}\u{2074}".to_string())
        );
        // Exponent 0 collapses to the mantissa.
        let t = Template::parse("{:num %!.2e}");
        assert_eq!(
            t.format(&Value::Number(4.2), None),
            Some("4.20".to_string())
        );
        // Negative mantissa: U+2212 up front.
        let t = Template::parse("{:num %!.1e}");
        assert_eq!(
            t.format(&Value::Number(-15000.0), None),
            Some("\u{2212}1.5 \u{d7} 10\u{2074}".to_string())
        );
        // %!g takes the pretty branch for large exponents and trims zeros.
        let t = Template::parse("{:num %!g}");
        assert_eq!(
            t.format(&Value::Number(1_000_000.0), None),
            Some("1 \u{d7} 10\u{2076}".to_string())
        );
        // Plain %e is unaffected.
        let t = Template::parse("{:num %.2e}");
        assert_eq!(
            t.format(&Value::Number(1234.5), None),
            Some("1.23e+03".to_string())
        );
    }

    #[test]
    fn num_si_prefixes() {
        let fmt = |spec: &str, v: f64| Template::parse(spec).format(&Value::Number(v), None);
        assert_eq!(fmt("{:num %S}", 1_234_567.0), Some("1.23M".to_string()));
        assert_eq!(fmt("{:num %S}", 1_234.0), Some("1.23k".to_string()));
        assert_eq!(fmt("{:num %S}", 0.0012), Some("1.2m".to_string()));
        assert_eq!(fmt("{:num %S}", 5e9), Some("5G".to_string()));
        assert_eq!(fmt("{:num %S}", 0.0), Some("0".to_string()));
        assert_eq!(fmt("{:num %S}", 42.0), Some("42".to_string()));
        // Precision = significant digits.
        assert_eq!(fmt("{:num %.1S}", 1_234_567.0), Some("1M".to_string()));
        assert_eq!(fmt("{:num %.4S}", 1_234_567.0), Some("1.235M".to_string()));
        // Rounding across a prefix boundary steps up.
        assert_eq!(fmt("{:num %.2S}", 999_900.0), Some("1M".to_string()));
        // Negatives get U+2212; sub-micro clamps at the smallest prefix.
        assert_eq!(fmt("{:num %S}", -1_500.0), Some("\u{2212}1.5k".to_string()));
        assert_eq!(fmt("{:num %S}", 1e-9), Some("1n".to_string()));
    }

    #[test]
    fn num_non_numeric_falls_back() {
        let t = Template::parse("{:num %.2f}");
        assert_eq!(t.format(&s("hello"), None), Some("hello".to_string()));
    }

    // ─── Default rendering ───

    #[test]
    fn default_label_variants() {
        assert_eq!(
            default_label(&Value::Bool(true), None),
            Some("true".to_string())
        );
        assert_eq!(
            default_label(&Value::Date(Date::from_ymd(2024, 3, 5).to_days()), None),
            Some("2024-03-05".to_string())
        );
        assert_eq!(
            default_label(
                &Value::Duration(Duration::from_seconds(90).to_micros()),
                None
            ),
            Some("90".to_string())
        );
    }

    #[test]
    fn default_label_snaps_float_noise() {
        // Unpinned precision snaps accumulated f64 error before display.
        assert_eq!(
            default_label(&Value::Number(0.1 + 0.2), None),
            Some("0.3".to_string())
        );
        assert_eq!(
            default_label(&Value::Number(0.6000000000001), None),
            Some("0.6".to_string())
        );
        // Zero and non-finite pass through untouched.
        assert_eq!(
            default_label(&Value::Number(0.0), None),
            Some("0".to_string())
        );
        assert_eq!(
            default_label(&Value::Number(f64::NAN), None),
            Some("NaN".to_string())
        );
        assert_eq!(
            default_label(&Value::Number(f64::INFINITY), None),
            Some("inf".to_string())
        );
        // Real precision up to 12 significant figures survives.
        assert_eq!(
            default_label(&Value::Number(1.23456789012), None),
            Some("1.23456789012".to_string())
        );
        // Pinned precision prints as pinned — no snap.
        assert_eq!(
            default_label(&Value::Number(0.1 + 0.2), Some(17)),
            Some("0.30000000000000004".to_string())
        );
    }
}
