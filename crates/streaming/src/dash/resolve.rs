//! DASH **SegmentTemplate/SegmentList/SegmentBase URL resolution**
//! (card 6.C1b): the 6.C1a model ([`playlists::mpd`]) resolved into
//! concrete, fetchable segment URLs — template `$Identifier$`
//! substitution (with printf-style format tags), SegmentTimeline
//! `t+n×d` expansion, and RFC 3986 base folding over the MPD /
//! Period / AdaptationSet / Representation `<BaseURL>` chain.
//!
//! # Scope (the honest v0 line)
//!
//! * Pure function of the parsed model + the manifest URL: **no I/O,
//!   no clock** — the fetch/scheduling session is 6.C2
//!   ([`super::sched`]).
//! * Inheritance is attribute-wise (ISO/IEC 23009-1 §5.3): the
//!   effective `SegmentTemplate` of a representation is the nearest
//!   of Representation ⊕ AdaptationSet ⊕ Period, field by field
//!   (nearest defined value wins, including the timeline).
//! * BaseURL folding takes the **first** `<BaseURL>` per level (the
//!   spec's mirror list is a redundancy/throughput concern the
//!   session owns); each level folds onto the previous with the
//!   playlists crate's RFC 3986 join ([`playlists::resolve_uri`]).
//! * A SegmentTimeline `@r` is the **additional repeat count** (an
//!   `S` entry describes `r+1` segments; `r` defaults to 0). A
//!   negative `@r` (repeat until the next `@t` or the Period end,
//!   §5.3.9.6) resolves against the next entry's `@t` when present,
//!   else `Period@duration`/presentation duration, else refuses
//!   typed ([`DashResolveError::TimelineExtentUnknown`]) — never a
//!   guess.
//! * Budgets (D-1, and the fuzz harness's bounded-memory contract):
//!   [`MAX_PLAN_SEGMENTS`] segments per representation,
//!   [`MAX_PLAN_URL_BYTES`] URL bytes + struct cost per
//!   representation plan; both are typed refusals, never clamps.
//! * A representation with neither template, list, SegmentBase nor
//!   own `<BaseURL>` refuses typed ([`DashResolveError::NoAddressing`]);
//!   one with an own `<BaseURL>` but no segment descriptors is the
//!   single-media-resource shape (§5.3.7.2 v0 subset: one segment,
//!   number 1, duration `Period@duration` when known).

use playlists::{
    AdaptationSet, Mpd, Period, Representation, SegmentTemplate, SegmentTimelineEntry, resolve_uri,
};

/// Cap on segments produced for one representation (bounded memory —
/// the fuzz contract; real ladders are orders of magnitude below).
pub const MAX_PLAN_SEGMENTS: usize = 65_536;

/// Budget on the accumulated URL bytes + per-segment struct cost of
/// one representation's plan (a hostile 64 KiB template must not
/// multiply into gigabytes of minted URLs).
pub const MAX_PLAN_URL_BYTES: usize = 16 * 1024 * 1024;

/// Per-segment fixed cost charged against [`MAX_PLAN_URL_BYTES`]
/// (the plan Vec's own overhead, conservatively).
const SEGMENT_STRUCT_COST: usize = 64;

/// Typed refusal of segment-plan resolution (house style: stable
/// dotted codes `streaming.dash.*`, structured `Display`, bounded
/// strings — MP §12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashResolveError {
    /// `$Unknown$` in a template (`streaming.dash.unknown-identifier`).
    UnknownIdentifier {
        /// The identifier as written (bounded by the 64 KiB line cap).
        identifier: String,
    },
    /// A referenced value is absent (e.g. `$Number$` in an
    /// initialization template, `$RepresentationID$` on an id-less
    /// rung) (`streaming.dash.missing-value`).
    MissingValue {
        /// The identifier whose value is absent.
        identifier: &'static str,
    },
    /// Malformed format tag (unknown conversion, an over-long
    /// width) or an unbalanced `$` (`streaming.dash.bad-template`).
    BadTemplate {
        /// What was wrong, briefly (static text).
        detail: &'static str,
    },
    /// A negative timeline `@r` with no bounding `@t` or period
    /// duration (`streaming.dash.timeline-extent-unknown`).
    TimelineExtentUnknown,
    /// Timeline arithmetic overflowed (`streaming.dash.timeline-overflow`).
    TimelineOverflow,
    /// A plan budget was exceeded (`streaming.dash.plan-too-large`).
    PlanTooLarge {
        /// Which budget: `"segments"` or `"bytes"`.
        budget: &'static str,
    },
    /// No usable addressing for a representation
    /// (`streaming.dash.no-addressing`).
    NoAddressing,
    /// A required numeric template fact was absent (e.g.
    /// `@duration` with no timeline and no period end)
    /// (`streaming.dash.missing-fact`).
    MissingFact {
        /// The missing fact, e.g. `"SegmentTemplate@duration"`.
        fact: &'static str,
    },
}

impl std::fmt::Display for DashResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DashResolveError::UnknownIdentifier { identifier } => write!(
                f,
                "streaming.dash.unknown-identifier: template references unknown identifier $${identifier}$"
            ),
            DashResolveError::MissingValue { identifier } => write!(
                f,
                "streaming.dash.missing-value: no value available for $${identifier}$"
            ),
            DashResolveError::BadTemplate { detail } => {
                write!(f, "streaming.dash.bad-template: {detail}")
            }
            DashResolveError::TimelineExtentUnknown => write!(
                f,
                "streaming.dash.timeline-extent-unknown: negative @r with no bounding @t or period duration"
            ),
            DashResolveError::TimelineOverflow => {
                write!(
                    f,
                    "streaming.dash.timeline-overflow: t+n*d exceeds the tick range"
                )
            }
            DashResolveError::PlanTooLarge { budget } => write!(
                f,
                "streaming.dash.plan-too-large: {budget} budget exceeded (bounded memory)"
            ),
            DashResolveError::NoAddressing => write!(
                f,
                "streaming.dash.no-addressing: representation has no template, list, base or BaseURL"
            ),
            DashResolveError::MissingFact { fact } => write!(
                f,
                "streaming.dash.missing-fact: {fact} absent where the shape requires it"
            ),
        }
    }
}

impl std::error::Error for DashResolveError {}

// ------------------------------------------------------- template math

/// One resolved media segment of a representation's plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentPlan {
    /// The segment's `@number` (startNumber-based; what `$Number$`
    /// substituted).
    pub number: u64,
    /// The segment's start tick in the template's timeline (what
    /// `$Time$` substituted; 0 when the shape carries no timeline).
    pub time: u64,
    /// The segment's duration in ticks (0 = unknown, e.g. the
    /// single-resource shape without a period duration).
    pub duration: u64,
    /// The resolved, absolute media URL.
    pub url: String,
}

/// One representation's resolved addressing (the 6.C1b output the
/// 6.C2 scheduler consumes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationPlan {
    /// The representation's `@id`, when present (selection logging).
    pub representation_id: Option<String>,
    /// The representation's `@bandwidth` (bits/s) — `None` rungs are
    /// unusable for ABR and are refused typed at selection time
    /// (6.C2's contract), never silently clamped here.
    pub bandwidth: Option<u64>,
    /// The timescale (ticks per second; spec default 1).
    pub timescale: u64,
    /// `presentationTimeOffset` (ticks).
    pub presentation_time_offset: u64,
    /// The initialization URL, when the shape has one.
    pub init_url: Option<String>,
    /// The media segments in presentation order.
    pub segments: Vec<SegmentPlan>,
}

/// The values available to a template substitution.
#[derive(Debug, Clone, Copy, Default)]
struct TemplateValues<'a> {
    representation_id: Option<&'a str>,
    bandwidth: Option<u64>,
    number: Option<u64>,
    time: Option<u64>,
}

/// Substitute `$Identifier$` / `$Identifier%tag$` / `$$` in a DASH
/// template string (ISO/IEC 23009-1 §5.3.9: the four standard
/// identifiers, printf-style format tags on the `%`-form, `$$` the
/// literal-dollar escape; any other identifier is an error).
fn substitute(template: &str, values: TemplateValues<'_>) -> Result<String, DashResolveError> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(dollar) = rest.find('$') {
        if dollar > 0 {
            out.push_str(&rest[..dollar]);
        }
        rest = &rest[dollar + 1..];
        // `$$` — the literal-dollar escape.
        if let Some(tail) = rest.strip_prefix('$') {
            out.push('$');
            rest = tail;
            continue;
        }
        // `$Identifier` … `$`, with an optional `%tag` before the
        // closing `$`. Identifier charset: [A-Za-z0-9_]+.
        let ident_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if ident_end == 0 {
            return Err(DashResolveError::BadTemplate {
                detail: "empty or non-identifier after '$'",
            });
        }
        let (identifier, after) = rest.split_at(ident_end);
        // The `%tag` form's closing `$` is the identifier's closer;
        // the plain form needs the next `$`. Both leave `rest` past
        // the closer.
        let (format, after_closer) = match after.strip_prefix('%') {
            Some(tagged) => {
                let close = tagged.find('$').ok_or(DashResolveError::BadTemplate {
                    detail: "unbalanced '$'",
                })?;
                (Some(&tagged[..close]), &tagged[close + 1..])
            }
            None => {
                let close = after.find('$').ok_or(DashResolveError::BadTemplate {
                    detail: "unbalanced '$'",
                })?;
                (None, &after[close + 1..])
            }
        };
        rest = after_closer;

        match identifier {
            "RepresentationID" => {
                let id = values
                    .representation_id
                    .ok_or(DashResolveError::MissingValue {
                        identifier: "RepresentationID",
                    })?;
                out.push_str(&apply_string_format(format, id)?);
            }
            "Bandwidth" => {
                let bw = values.bandwidth.ok_or(DashResolveError::MissingValue {
                    identifier: "Bandwidth",
                })?;
                out.push_str(&apply_int_format(format, bw)?);
            }
            "Number" => {
                let n = values.number.ok_or(DashResolveError::MissingValue {
                    identifier: "Number",
                })?;
                out.push_str(&apply_int_format(format, n)?);
            }
            "Time" => {
                let t = values
                    .time
                    .ok_or(DashResolveError::MissingValue { identifier: "Time" })?;
                out.push_str(&apply_int_format(format, t)?);
            }
            other => {
                return Err(DashResolveError::UnknownIdentifier {
                    identifier: other.to_owned(),
                });
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Apply a printf-style format tag (the `Some` text between `%` and
/// `$`; `None` = the plain form) to an integer value. Supported
/// conversions: `d u q x X o` (decimal ×3 / hex / HEX / octal);
/// width ≤ 2 digits (bounded output — a `$Number%099999d$` template
/// must not mint strings).
fn apply_int_format(format: Option<&str>, value: u64) -> Result<String, DashResolveError> {
    let format = match format {
        Some(f) => f,
        None => return Ok(value.to_string()),
    };
    let (zero_pad, width_digits, conv) = parse_format_tag(format)?;
    if conv == 's' {
        return Err(DashResolveError::BadTemplate {
            detail: "%s conversion on an integer identifier",
        });
    }
    let body = match conv {
        'x' => format!("{value:x}"),
        'X' => format!("{value:X}"),
        'o' => format!("{value:o}"),
        // d / u / q — all decimal for u64 values.
        _ => value.to_string(),
    };
    Ok(pad(body, zero_pad, width_digits))
}

/// Apply a format tag to the string identifier (`RepresentationID`):
/// only the plain form or `%s` is legal.
fn apply_string_format(format: Option<&str>, value: &str) -> Result<String, DashResolveError> {
    let format = match format {
        Some(f) => f,
        None => return Ok(value.to_owned()),
    };
    let (zero_pad, width_digits, conv) = parse_format_tag(format)?;
    if conv != 's' {
        return Err(DashResolveError::BadTemplate {
            detail: "integer conversion on $RepresentationID$",
        });
    }
    Ok(pad(value.to_owned(), zero_pad, width_digits))
}

/// Parse the `[0][width]conv` tag body (the text between `%` and
/// `$`) — returns `(zero_pad, width, conv)`.
fn parse_format_tag(format: &str) -> Result<(bool, usize, char), DashResolveError> {
    let body = format;
    let (zero_pad, body) = match body.strip_prefix('0') {
        Some(b) => (true, b),
        None => (false, body),
    };
    let digits_end = body
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(body.len());
    // No width digits (`d`, `0d`): plain conversion, width 0.
    let (width, conv) = if digits_end == 0 {
        (0, body.chars().next())
    } else {
        let width: usize =
            body[..digits_end]
                .parse()
                .map_err(|_| DashResolveError::BadTemplate {
                    detail: "format width not numeric",
                })?;
        (width, body[digits_end..].chars().next())
    };
    let conv = conv.ok_or(DashResolveError::BadTemplate {
        detail: "format tag has no conversion character",
    })?;
    if !matches!(conv, 'd' | 'u' | 'q' | 'x' | 'X' | 'o' | 's') {
        return Err(DashResolveError::BadTemplate {
            detail: "unknown format conversion",
        });
    }
    if width > 99 {
        // Width > 99 — refuse rather than mint huge padded strings.
        return Err(DashResolveError::BadTemplate {
            detail: "format width wider than 2 digits",
        });
    }
    Ok((zero_pad, width, conv))
}

/// Width-pad a body: zero-padding with a width, else right-justify
/// with spaces (the printf meanings).
fn pad(body: String, zero_pad: bool, width: usize) -> String {
    if body.len() >= width {
        return body;
    }
    if zero_pad {
        let mut out = "0".repeat(width - body.len());
        out.push_str(&body);
        out
    } else {
        let mut out = " ".repeat(width - body.len());
        out.push_str(&body);
        out
    }
}

// ----------------------------------------------------- timeline expansion

/// An expanded timeline row: the segment's start tick and duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimelineSlot {
    time: u64,
    duration: u64,
}

/// Expand a SegmentTimeline into `(time, duration)` slots (ISO/IEC
/// 23009-1 §5.3.9.6): each `S` describes `r+1` segments of duration
/// `d` starting at `t` (default: the previous entry's end); a
/// negative `@r` repeats until the next entry's `@t`, else until
/// `period_end` (ticks), else refuses — the extent is genuinely
/// unknowable.
fn expand_timeline(
    entries: &[SegmentTimelineEntry],
    period_end: Option<u64>,
    cap: usize,
) -> Result<Vec<TimelineSlot>, DashResolveError> {
    let mut slots = Vec::new();
    let mut cursor: u64 = 0;
    for (i, entry) in entries.iter().enumerate() {
        let start = entry.t.unwrap_or(cursor);
        let repeats: u64 = match entry.r {
            // r additional repeats (the entry describes r+1).
            Some(r) if r >= 0 => u64::try_from(r)
                .ok()
                .and_then(|base| base.checked_add(1))
                .ok_or(DashResolveError::TimelineOverflow)?,
            // Negative: until the next entry's @t …
            Some(_) => {
                let next_t = entries[i + 1..].iter().find_map(|e| e.t);
                match next_t {
                    Some(t) if t > start => {
                        let span = t - start;
                        span / entry.d + u64::from(span % entry.d != 0)
                    }
                    // … else the period end (ticks), …
                    _ => match period_end {
                        Some(end) if end > start => {
                            let span = end - start;
                            span / entry.d + u64::from(span % entry.d != 0)
                        }
                        // … else genuinely unknowable.
                        _ => return Err(DashResolveError::TimelineExtentUnknown),
                    },
                }
            }
            None => 1,
        };
        for n in 0..repeats {
            if slots.len() >= cap {
                return Err(DashResolveError::PlanTooLarge { budget: "segments" });
            }
            let off = match entry.d.checked_mul(n) {
                Some(off) => off,
                None => return Err(DashResolveError::TimelineOverflow),
            };
            let time = match start.checked_add(off) {
                Some(t) => t,
                None => return Err(DashResolveError::TimelineOverflow),
            };
            slots.push(TimelineSlot {
                time,
                duration: entry.d,
            });
        }
        cursor = match entry
            .d
            .checked_mul(repeats)
            .and_then(|span| start.checked_add(span))
        {
            Some(c) => c,
            None => return Err(DashResolveError::TimelineOverflow),
        };
    }
    Ok(slots)
}

// --------------------------------------------------------- base folding

/// Fold the BaseURL chain (first mirror per level) from the manifest
/// URL down to the representation, RFC 3986 at each step.
fn effective_base(chain: &[&str]) -> String {
    let mut base = String::new();
    for link in chain {
        base = resolve_uri(&base, link);
    }
    base
}

// ------------------------------------------------------ the plan itself

/// The effective (inheritance-merged) SegmentTemplate of a
/// representation: nearest-defined field wins across
/// Representation ⊕ AdaptationSet ⊕ Period.
fn effective_template(
    period: &Period,
    set: &AdaptationSet,
    rep: &Representation,
) -> Option<SegmentTemplate> {
    // The nearest level that HAS the element provides the base …
    let mut merged = rep
        .segment_template
        .clone()
        .or_else(|| set.segment_template.clone())
        .or_else(|| period.segment_template.clone())?;
    // … and every OUTER level's defined fields fill its absent ones.
    if let Some(outer) = &set.segment_template {
        merge_template(&mut merged, outer);
    }
    if let Some(outer) = &period.segment_template {
        merge_template(&mut merged, outer);
    }
    Some(merged)
}

/// Fill `into`'s absent fields from `from` (inheritance's fallback
/// direction: `into` is the nearer level).
fn merge_template(into: &mut SegmentTemplate, from: &SegmentTemplate) {
    if into.timescale.is_none() {
        into.timescale = from.timescale;
    }
    if into.duration.is_none() {
        into.duration = from.duration;
    }
    if into.start_number.is_none() {
        into.start_number = from.start_number;
    }
    if into.presentation_time_offset.is_none() {
        into.presentation_time_offset = from.presentation_time_offset;
    }
    if into.initialization.is_none() {
        into.initialization = from.initialization.clone();
    }
    if into.media.is_none() {
        into.media = from.media.clone();
    }
    if into.segment_timeline.is_none() {
        into.segment_timeline = from.segment_timeline.clone();
    }
}

/// Resolve one representation of one period into a
/// [`RepresentationPlan`] against `manifest_url`.
///
/// # Errors
/// [`DashResolveError`] — every refusal is typed (bad templates,
/// unknowable timelines, budget breaches, missing addressing).
pub fn plan_representation(
    mpd: &Mpd,
    period: &Period,
    set: &AdaptationSet,
    rep: &Representation,
    manifest_url: &str,
) -> Result<RepresentationPlan, DashResolveError> {
    // The BaseURL chain: manifest → MPD → Period → Set → Rep (first
    // mirror per level — redundancy policy is the session's).
    let mut chain: Vec<&str> = vec![manifest_url];
    if let Some(first) = mpd.base_urls.first() {
        chain.push(first.as_str());
    }
    if let Some(first) = period.base_urls.first() {
        chain.push(first.as_str());
    }
    if let Some(first) = set.base_urls.first() {
        chain.push(first.as_str());
    }
    if let Some(first) = rep.base_urls.first() {
        chain.push(first.as_str());
    }
    let base = effective_base(&chain);

    let id_values = TemplateValues {
        representation_id: rep.id.as_deref(),
        bandwidth: rep.bandwidth,
        number: None,
        time: None,
    };

    // ---- SegmentTemplate (the $-substitution shape).
    if let Some(template) = effective_template(period, set, rep) {
        let timescale = template.timescale.unwrap_or(1);
        let start_number = template.start_number.unwrap_or(1);
        let pto = template.presentation_time_offset.unwrap_or(0);
        let init_url = match template.initialization.as_deref() {
            Some(init) => {
                let substituted = substitute(init, id_values)?;
                Some(resolve_uri(&base, &substituted))
            }
            None => None,
        };
        let media = template
            .media
            .as_deref()
            .ok_or(DashResolveError::NoAddressing)?;
        // The slots: explicit timeline …
        let slots: Vec<TimelineSlot> = if let Some(timeline) = &template.segment_timeline {
            let period_end = period_duration_ticks(mpd, period, timescale);
            expand_timeline(&timeline.entries, period_end, MAX_PLAN_SEGMENTS)?
        } else {
            // … or the fixed-duration form (@duration covering the
            // whole period).
            let duration = template.duration.ok_or(DashResolveError::MissingFact {
                fact: "SegmentTemplate@duration",
            })?;
            let count = fixed_segment_count(mpd, period, timescale, duration)
                .ok_or(DashResolveError::TimelineExtentUnknown)?;
            let mut slots = Vec::new();
            for i in 0..count {
                if slots.len() >= MAX_PLAN_SEGMENTS {
                    return Err(DashResolveError::PlanTooLarge { budget: "segments" });
                }
                let time = i
                    .checked_mul(duration)
                    .ok_or(DashResolveError::TimelineOverflow)?;
                slots.push(TimelineSlot { time, duration });
            }
            slots
        };
        let mut segments = Vec::with_capacity(slots.len().min(MAX_PLAN_SEGMENTS));
        let mut bytes_budget = MAX_PLAN_URL_BYTES;
        for (i, slot) in slots.iter().enumerate() {
            let number = start_number
                .checked_add(i as u64)
                .ok_or(DashResolveError::TimelineOverflow)?;
            let substituted = substitute(
                media,
                TemplateValues {
                    number: Some(number),
                    time: Some(slot.time),
                    ..id_values
                },
            )?;
            bytes_budget = bytes_budget
                .checked_sub(substituted.len() + SEGMENT_STRUCT_COST)
                .ok_or(DashResolveError::PlanTooLarge { budget: "bytes" })?;
            segments.push(SegmentPlan {
                number,
                time: slot.time,
                duration: slot.duration,
                url: resolve_uri(&base, &substituted),
            });
        }
        return Ok(RepresentationPlan {
            representation_id: rep.id.clone(),
            bandwidth: rep.bandwidth,
            timescale,
            presentation_time_offset: pto,
            init_url,
            segments,
        });
    }

    // ---- SegmentList (the explicit URL list; nearest level wins
    // wholesale — no partial merge across list levels).
    if let Some(list) = rep.segment_list.as_ref().or(set.segment_list.as_ref()) {
        let timescale = list.timescale.unwrap_or(1);
        let start_number = list.start_number.unwrap_or(1);
        let pto = list.presentation_time_offset.unwrap_or(0);
        let init_url = list
            .initialization
            .as_deref()
            .map(|init| resolve_uri(&base, init));
        let duration = list.duration.unwrap_or(0);
        let mut segments = Vec::with_capacity(list.segment_urls.len().min(MAX_PLAN_SEGMENTS));
        let mut bytes_budget = MAX_PLAN_URL_BYTES;
        for (i, url) in list.segment_urls.iter().enumerate() {
            if segments.len() >= MAX_PLAN_SEGMENTS {
                return Err(DashResolveError::PlanTooLarge { budget: "segments" });
            }
            let media = url.media.as_deref().ok_or(DashResolveError::NoAddressing)?;
            bytes_budget = bytes_budget
                .checked_sub(media.len() + SEGMENT_STRUCT_COST)
                .ok_or(DashResolveError::PlanTooLarge { budget: "bytes" })?;
            let number = start_number
                .checked_add(i as u64)
                .ok_or(DashResolveError::TimelineOverflow)?;
            segments.push(SegmentPlan {
                number,
                time: duration
                    .checked_mul(i as u64)
                    .ok_or(DashResolveError::TimelineOverflow)?,
                duration,
                url: resolve_uri(&base, media),
            });
        }
        return Ok(RepresentationPlan {
            representation_id: rep.id.clone(),
            bandwidth: rep.bandwidth,
            timescale,
            presentation_time_offset: pto,
            init_url,
            segments,
        });
    }

    // ---- SegmentBase / single-resource shape: one segment at the
    // representation's own resolved BaseURL (a SegmentBase's
    // indexRange splits that resource — byte-range interpretation
    // is the fetch layer's, recorded not guessed here).
    let single_url = match rep.base_urls.first() {
        Some(own) => resolve_uri(&base, own),
        None => {
            if rep.segment_base.is_some() && chain.len() > 1 {
                // A SegmentBase with an inherited (non-manifest)
                // base: that base IS the media resource.
                base
            } else {
                return Err(DashResolveError::NoAddressing);
            }
        }
    };
    let timescale = rep
        .segment_base
        .as_ref()
        .and_then(|b| b.timescale)
        .unwrap_or(1);
    let pto = rep
        .segment_base
        .as_ref()
        .and_then(|b| b.presentation_time_offset)
        .unwrap_or(0);
    let duration_ticks = period_duration_ticks(mpd, period, timescale).unwrap_or(0);
    Ok(RepresentationPlan {
        representation_id: rep.id.clone(),
        bandwidth: rep.bandwidth,
        timescale,
        presentation_time_offset: pto,
        init_url: None,
        segments: vec![SegmentPlan {
            number: 1,
            time: 0,
            duration: duration_ticks,
            url: single_url,
        }],
    })
}

/// The Period's end tick (relative to the period start) under a
/// timescale: `Period@duration` when present, else
/// `MPD@mediaPresentationDuration − Period@start` (a missing `@start`
/// is the MPD's beginning — spec §5.3.2.1), else `None`.
fn period_duration_ticks(mpd: &Mpd, period: &Period, timescale: u64) -> Option<u64> {
    let seconds = period.duration.or_else(|| {
        mpd.media_presentation_duration
            .map(|total| total - period.start.unwrap_or(0.0))
    })?;
    let ticks_f = seconds * timescale as f64;
    if ticks_f.is_finite() && (0.0..=u64::MAX as f64).contains(&ticks_f) {
        Some(ticks_f as u64)
    } else {
        None
    }
}

/// The count of fixed-duration segments covering the period
/// (ceil-division on ticks; None when the extent is unknown).
fn fixed_segment_count(mpd: &Mpd, period: &Period, timescale: u64, duration: u64) -> Option<u64> {
    let end = period_duration_ticks(mpd, period, timescale)?;
    let count = end / duration + u64::from(end % duration != 0);
    if count == 0 { None } else { Some(count) }
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;
    use playlists::parse_mpd_bytes;

    fn fixture(name: &str) -> Vec<u8> {
        let path = format!(
            "{}/../playlists/tests/fixtures/dash/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => unreachable!("fixture {name}: {e}"),
        }
    }

    fn first_video_plan(bytes: &[u8]) -> RepresentationPlan {
        let parsed = match parse_mpd_bytes(bytes) {
            Ok(p) => p,
            Err(e) => unreachable!("parse: {e}"),
        };
        let period = &parsed.mpd.periods[0];
        let set = &period.adaptation_sets[0];
        match plan_representation(
            &parsed.mpd,
            period,
            set,
            &set.representations[0],
            "http://ex.test/vod/manifest.mpd",
        ) {
            Ok(p) => p,
            Err(e) => unreachable!("plan: {e}"),
        }
    }

    // -- $Identifier$ substitution math (spec-derived vectors). --

    #[test]
    fn substitutes_the_four_identifiers_plain() {
        let out = substitute(
            "s-$RepresentationID$-$Number$-$Time$-$Bandwidth$.m4s",
            TemplateValues {
                representation_id: Some("v0"),
                bandwidth: Some(500_000),
                number: Some(7),
                time: Some(61_440),
            },
        );
        assert_eq!(out.as_deref(), Ok("s-v0-7-61440-500000.m4s"));
    }

    #[test]
    fn format_tags_pad_and_convert() {
        let out = substitute(
            "chunk-$Number%05d$-$Time%03x$-$Bandwidth$.m4s",
            TemplateValues {
                bandwidth: Some(250),
                number: Some(42),
                time: Some(255),
                representation_id: None,
            },
        );
        assert_eq!(out.as_deref(), Ok("chunk-00042-0ff-250.m4s"));
        let oct = substitute(
            "$Time%o$",
            TemplateValues {
                time: Some(8),
                ..Default::default()
            },
        );
        assert_eq!(oct.as_deref(), Ok("10"));
        let sp = substitute(
            "$Number%6d$",
            TemplateValues {
                number: Some(42),
                ..Default::default()
            },
        );
        assert_eq!(sp.as_deref(), Ok("    42"));
        let strf = substitute(
            "$RepresentationID%3s$",
            TemplateValues {
                representation_id: Some("hi"),
                ..Default::default()
            },
        );
        assert_eq!(strf.as_deref(), Ok(" hi"));
    }

    #[test]
    fn dollar_escape_and_errors() {
        assert_eq!(
            substitute("a$$b.m4s", TemplateValues::default()).as_deref(),
            Ok("a$b.m4s")
        );
        assert_eq!(
            substitute("$Unknown$", TemplateValues::default()),
            Err(DashResolveError::UnknownIdentifier {
                identifier: "Unknown".to_owned()
            })
        );
        assert_eq!(
            substitute("$Number$", TemplateValues::default()),
            Err(DashResolveError::MissingValue {
                identifier: "Number"
            })
        );
        assert_eq!(
            substitute(
                "$Number%05d",
                TemplateValues {
                    number: Some(1),
                    ..Default::default()
                }
            ),
            Err(DashResolveError::BadTemplate {
                detail: "unbalanced '$'"
            })
        );
        assert_eq!(
            substitute(
                "$Number%05z$",
                TemplateValues {
                    number: Some(1),
                    ..Default::default()
                }
            ),
            Err(DashResolveError::BadTemplate {
                detail: "unknown format conversion"
            })
        );
        // Bounded output: absurd widths refuse rather than allocate.
        assert_eq!(
            substitute(
                "$Number%0999d$",
                TemplateValues {
                    number: Some(1),
                    ..Default::default()
                }
            ),
            Err(DashResolveError::BadTemplate {
                detail: "format width wider than 2 digits"
            })
        );
    }

    // -- Timeline t+n×d arithmetic. --

    #[test]
    fn timeline_expansion_t_d_r() {
        let entries = vec![
            SegmentTimelineEntry {
                t: Some(0),
                d: 10,
                r: None,
            },
            SegmentTimelineEntry {
                t: None,
                d: 5,
                r: Some(2),
            },
            SegmentTimelineEntry {
                t: Some(100),
                d: 1,
                r: Some(-1),
            },
        ];
        let slots = match expand_timeline(&entries, Some(110), 1000) {
            Ok(s) => s,
            Err(e) => unreachable!("expand: {e}"),
        };
        let got: Vec<(u64, u64)> = slots.iter().map(|s| (s.time, s.duration)).collect();
        // r=2 → three d=5 segments at 10,15,20 (t inherits the
        // previous end); negative r until the period end 110 → ten
        // d=1 segments from t=100.
        assert_eq!(
            got,
            vec![
                (0, 10),
                (10, 5),
                (15, 5),
                (20, 5),
                (100, 1),
                (101, 1),
                (102, 1),
                (103, 1),
                (104, 1),
                (105, 1),
                (106, 1),
                (107, 1),
                (108, 1),
                (109, 1),
            ]
        );
    }

    #[test]
    fn negative_r_bounded_by_next_t_or_period() {
        // No extent anywhere → typed refusal.
        let entries = vec![SegmentTimelineEntry {
            t: Some(0),
            d: 10,
            r: Some(-1),
        }];
        assert_eq!(
            expand_timeline(&entries, None, 10),
            Err(DashResolveError::TimelineExtentUnknown)
        );
        // Bounded by the next entry's @t (no period end needed):
        // span 35 @ d=10 → ceil 4 repeats, then the t=35 entry.
        let entries = vec![
            SegmentTimelineEntry {
                t: Some(0),
                d: 10,
                r: Some(-1),
            },
            SegmentTimelineEntry {
                t: Some(35),
                d: 10,
                r: None,
            },
        ];
        let slots = match expand_timeline(&entries, None, 100) {
            Ok(s) => s,
            Err(e) => unreachable!("expand: {e}"),
        };
        assert_eq!(slots.len(), 5);
        assert_eq!(slots[3].time, 30);
        assert_eq!(slots[4].time, 35);
    }

    #[test]
    fn timeline_overflow_and_caps_refuse_typed() {
        let entries = vec![SegmentTimelineEntry {
            t: Some(u64::MAX - 5),
            d: 10,
            r: None,
        }];
        assert_eq!(
            expand_timeline(&entries, None, 10),
            Err(DashResolveError::TimelineOverflow)
        );
        let entries = vec![SegmentTimelineEntry {
            t: Some(0),
            d: 1,
            r: Some(100),
        }];
        assert_eq!(
            expand_timeline(&entries, None, 10),
            Err(DashResolveError::PlanTooLarge { budget: "segments" })
        );
    }

    // -- The real ffmpeg fixtures (spec-conformant vectors). --

    #[test]
    fn ffmpeg_template_fixture_resolves() {
        let plan = first_video_plan(&fixture("ffmpeg_av_template.mpd"));
        assert_eq!(plan.representation_id.as_deref(), Some("0"));
        assert_eq!(plan.bandwidth, Some(500_000));
        assert_eq!(plan.timescale, 10_240);
        // init: init-stream0.m4s against the manifest directory.
        assert_eq!(
            plan.init_url.as_deref(),
            Some("http://ex.test/vod/init-stream0.m4s")
        );
        // media: chunk-stream0-00001.m4s (the %05d tag), one S entry
        // (t=0, d=61440) → one segment.
        assert_eq!(plan.segments.len(), 1);
        assert_eq!(
            plan.segments[0].url,
            "http://ex.test/vod/chunk-stream0-00001.m4s"
        );
        assert_eq!(plan.segments[0].number, 1);
        assert_eq!(plan.segments[0].time, 0);
        assert_eq!(plan.segments[0].duration, 61_440);
    }

    #[test]
    fn ffmpeg_template_fixture_audio_two_segments() {
        let bytes = fixture("ffmpeg_av_template.mpd");
        let parsed = match parse_mpd_bytes(&bytes) {
            Ok(p) => p,
            Err(e) => unreachable!("parse: {e}"),
        };
        let period = &parsed.mpd.periods[0];
        let set = &period.adaptation_sets[1]; // audio
        let plan = match plan_representation(
            &parsed.mpd,
            period,
            set,
            &set.representations[0],
            "http://ex.test/vod/manifest.mpd",
        ) {
            Ok(p) => p,
            Err(e) => unreachable!("plan: {e}"),
        };
        // S(t=0,d=220160) + S(d=44440) at t=220160 (timescale 44100).
        assert_eq!(plan.timescale, 44_100);
        assert_eq!(plan.segments.len(), 2);
        assert_eq!(plan.segments[1].time, 220_160);
        assert_eq!(plan.segments[1].duration, 44_440);
        assert_eq!(plan.segments[1].number, 2);
        assert_eq!(
            plan.segments[1].url,
            "http://ex.test/vod/chunk-stream2-00002.m4s"
        );
    }

    #[test]
    fn ffmpeg_list_fixture_resolves_explicit_urls() {
        let bytes = fixture("ffmpeg_av_list.mpd");
        let parsed = match parse_mpd_bytes(&bytes) {
            Ok(p) => p,
            Err(e) => unreachable!("parse: {e}"),
        };
        let period = &parsed.mpd.periods[0];
        let set = &period.adaptation_sets[0];
        let plan = match plan_representation(
            &parsed.mpd,
            period,
            set,
            &set.representations[0],
            "http://ex.test/vod/manifest.mpd",
        ) {
            Ok(p) => p,
            Err(e) => unreachable!("plan: {e}"),
        };
        // SegmentList numbering from startNumber; init from
        // Initialization@sourceURL.
        assert!(
            plan.init_url
                .as_deref()
                .is_some_and(|u| u.ends_with("init-stream0.m4s"))
        );
        assert!(!plan.segments.is_empty());
        assert_eq!(plan.segments[0].number, 1);
        assert!(
            plan.segments
                .iter()
                .all(|s| s.url.starts_with("http://ex.test/vod/"))
        );
    }

    #[test]
    fn hand_fixture_multi_level_baseurl_and_pto() {
        let bytes = fixture("hand_static_cenc.mpd");
        let parsed = match parse_mpd_bytes(&bytes) {
            Ok(p) => p,
            Err(e) => unreachable!("parse: {e}"),
        };
        // The hand fixture exercises set-level template inheritance
        // and BaseURL levels; every minted URL must carry them.
        let period = &parsed.mpd.periods[0];
        for set in &period.adaptation_sets {
            for rep in &set.representations {
                let plan = match plan_representation(
                    &parsed.mpd,
                    period,
                    set,
                    rep,
                    "http://ex.test/base/manifest.mpd",
                ) {
                    Ok(p) => p,
                    Err(e) => unreachable!("plan: {e}"),
                };
                for seg in &plan.segments {
                    assert!(
                        seg.url.starts_with("http://") || seg.url.starts_with("https://"),
                        "absolute URL after BaseURL folding: {}",
                        seg.url
                    );
                }
            }
        }
    }

    #[test]
    fn no_addressing_refuses_typed() {
        let rep = Representation {
            id: Some("x".to_owned()),
            bandwidth: Some(1),
            width: None,
            height: None,
            frame_rate: None,
            sar: None,
            mime_type: None,
            codecs: None,
            audio_sampling_rate: None,
            base_urls: Vec::new(),
            segment_template: None,
            segment_list: None,
            segment_base: None,
            content_protection: Vec::new(),
            descriptors: Vec::new(),
        };
        let set = AdaptationSet {
            id: None,
            content_type: Some("video".to_owned()),
            mime_type: None,
            codecs: None,
            lang: None,
            segment_alignment: None,
            bitstream_switching: None,
            start_with_sap: None,
            max_width: None,
            max_height: None,
            par: None,
            frame_rate: None,
            base_urls: Vec::new(),
            segment_template: None,
            segment_list: None,
            content_protection: Vec::new(),
            descriptors: Vec::new(),
            representations: vec![rep],
        };
        let period = Period {
            id: None,
            start: None,
            duration: None,
            bitstream_switching: None,
            base_urls: Vec::new(),
            segment_template: None,
            adaptation_sets: vec![set],
        };
        let mpd = Mpd {
            mpd_type: playlists::MpdType::Static,
            profiles: None,
            media_presentation_duration: None,
            minimum_update_period: None,
            max_segment_duration: None,
            min_buffer_time: None,
            time_shift_buffer_depth: None,
            availability_start_time: None,
            publish_time: None,
            id: None,
            base_urls: Vec::new(),
            periods: vec![period],
        };
        let p = &mpd.periods[0];
        assert_eq!(
            plan_representation(
                &mpd,
                p,
                &p.adaptation_sets[0],
                &p.adaptation_sets[0].representations[0],
                "http://ex.test/m.mpd"
            ),
            Err(DashResolveError::NoAddressing)
        );
    }

    #[test]
    fn template_inheritance_nearest_field_wins() {
        // Period-level template with duration+media+startNumber+init;
        // rep-level overrides only media — merged fields.
        let period_template = SegmentTemplate {
            timescale: Some(1000),
            duration: Some(2000),
            start_number: Some(5),
            presentation_time_offset: None,
            initialization: Some("i-$RepresentationID$.m4s".to_owned()),
            media: Some("p-$Number$.m4s".to_owned()),
            segment_timeline: None,
        };
        let rep_template = SegmentTemplate {
            timescale: None,
            duration: None,
            start_number: None,
            presentation_time_offset: None,
            initialization: None,
            media: Some("r-$Number$.m4s".to_owned()),
            segment_timeline: None,
        };
        let mut period = Period {
            id: None,
            start: None,
            duration: Some(4.0),
            bitstream_switching: None,
            base_urls: Vec::new(),
            segment_template: Some(period_template),
            adaptation_sets: Vec::new(),
        };
        let mut set = AdaptationSet {
            id: None,
            content_type: None,
            mime_type: None,
            codecs: None,
            lang: None,
            segment_alignment: None,
            bitstream_switching: None,
            start_with_sap: None,
            max_width: None,
            max_height: None,
            par: None,
            frame_rate: None,
            base_urls: Vec::new(),
            segment_template: None,
            segment_list: None,
            content_protection: Vec::new(),
            descriptors: Vec::new(),
            representations: Vec::new(),
        };
        let rep = Representation {
            id: Some("v".to_owned()),
            bandwidth: Some(100),
            width: None,
            height: None,
            frame_rate: None,
            sar: None,
            mime_type: None,
            codecs: None,
            audio_sampling_rate: None,
            base_urls: Vec::new(),
            segment_template: Some(rep_template),
            segment_list: None,
            segment_base: None,
            content_protection: Vec::new(),
            descriptors: Vec::new(),
        };
        set.representations.push(rep);
        period.adaptation_sets.push(set);
        let mpd = Mpd {
            mpd_type: playlists::MpdType::Static,
            profiles: None,
            media_presentation_duration: None,
            minimum_update_period: None,
            max_segment_duration: None,
            min_buffer_time: None,
            time_shift_buffer_depth: None,
            availability_start_time: None,
            publish_time: None,
            id: None,
            base_urls: Vec::new(),
            periods: vec![period],
        };
        let p = &mpd.periods[0];
        let plan = match plan_representation(
            &mpd,
            p,
            &p.adaptation_sets[0],
            &p.adaptation_sets[0].representations[0],
            "http://ex.test/x.mpd",
        ) {
            Ok(plan) => plan,
            Err(e) => unreachable!("plan: {e}"),
        };
        // duration 2000 ticks @ timescale 1000 = 2s; Period 4s → 2
        // segments numbered from the inherited startNumber 5.
        assert_eq!(plan.timescale, 1000);
        assert_eq!(plan.segments.len(), 2);
        assert_eq!(plan.segments[0].url, "http://ex.test/r-5.m4s");
        assert_eq!(plan.segments[1].url, "http://ex.test/r-6.m4s");
        assert_eq!(plan.segments[1].time, 2000);
        assert_eq!(plan.init_url.as_deref(), Some("http://ex.test/i-v.m4s"));
    }
}
