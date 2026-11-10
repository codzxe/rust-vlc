//! The soak scenario manifest loader (card 8.C1, issue #54 / P8-3) —
//! the config plane of the scenario suite.
//!
//! Every duration, cycle count, target, budget, threshold and fixture
//! list the suite uses is read from a TOML manifest selected at run
//! time (`SOAK_SCENARIOS`, default `tests/soak/scenarios.toml`), so
//! the CI-short smoke, the bounded sandbox profile and the 4 h nightly
//! tier are three FILES, not three code paths. The shape contract is
//! `tests/soak/scenarios.schema.json` (JSON Schema, draft 2020-12);
//! this loader enforces the same contract in Rust with loud
//! refusals — unknown keys, wrong types, out-of-range values and
//! duplicate ids are errors, never defaults (the `SOAK_MINUTES`
//! dirty-world discipline: garbage never silently becomes 2 minutes).
//!
//! # The TOML subset (the in-tree precedent)
//!
//! `playback-engine`'s manifest is Lead-owned and dependency-free by
//! C5, so the loader is the workspace's established line-scanner
//! style (the `tests/soak/scenario.rs` expectation parser and xtask's
//! `toml_lite`): bare keys, `[defaults]`, `[[scenario]]` array
//! tables, quoted strings, integers, floats, booleans, and
//! SINGLE-LINE string arrays. Multi-line arrays are refused loudly
//! (the boundaries.toml reader's rule). Full-line `#` comments only.
//!
//! # Errors
//!
//! Every refusal is a human sentence naming the file, the key and the
//! problem — a broken manifest means a broken test setup, and the
//! sentence is the debug session.

use std::collections::HashMap;
use std::path::Path;

use testkit::Must;

/// The manifest shape version this loader accepts (the
/// boundaries.toml `version` discipline: exactly one shape, bump on
/// incompatible change — the JSON schema carries the same const).
const SCHEMA_VERSION: i64 = 1;

// ---------------------------------------------------------------------------
// The model
// ---------------------------------------------------------------------------

/// One parsed manifest: the defaults plus the scenario list, in file
/// order (execution order).
#[derive(Debug, Clone)]
pub struct Manifest {
    /// The manifest's own path, as loaded (the report records which
    /// file drove the run).
    pub source: String,
    /// The suite-wide defaults (per-scenario `leak_ceiling_mb_per_h`
    /// may override the ceiling only).
    pub defaults: Defaults,
    /// The scenarios, in file order.
    pub scenarios: Vec<ScenarioSpec>,
}

/// The suite-wide defaults — every knob the runners read that is not
/// per-scenario. Mirrors `[defaults]` in the schema one-to-one.
#[derive(Debug, Clone, Copy)]
pub struct Defaults {
    /// Memory-sampling cadence (ms).
    pub sample_ms: u64,
    /// The warmup share of each scenario, excluded from slope fits.
    pub warmup_fraction: f64,
    /// Liveness floor on completed cycles (D-6).
    pub min_cycles: u64,
    /// Liveness ceiling on the state-skip share of ops (D-6).
    pub max_skip_rate: f64,
    /// The asserted live-bytes slope ceiling (MB/h).
    pub leak_ceiling_mb_per_h: f64,
    /// The design target (MB/h) — reported always, asserted only
    /// where `assert_target` is set.
    pub target_mb_per_h: f64,
    /// Whether `target_mb_per_h` is an asserted budget this run.
    pub assert_target: bool,
    /// Count-driven scenarios: allowed VmRSS growth, post-warmup
    /// baseline → final (MB).
    pub rss_growth_budget_mb: f64,
    /// Count-driven scenarios: allowed open-fd growth (Linux
    /// `/proc/self/fd`; reported not-measurable elsewhere).
    pub fd_growth_budget: u64,
    /// Count-driven scenarios: allowed live-bytes growth after the
    /// final close, post-warmup baseline (bytes).
    pub live_growth_budget_bytes: u64,
}

/// One `[[scenario]]` row.
#[derive(Debug, Clone)]
pub struct ScenarioSpec {
    /// The stable scenario id (report keys; unique).
    pub id: String,
    /// The runner kind.
    pub kind: Kind,
    /// Real wall duration (minutes). Required for the duration kinds;
    /// an optional wall cap for the count kinds.
    pub duration_min: Option<f64>,
    /// The cycle count. Required for the count kinds.
    pub cycles: Option<u64>,
    /// Fixture paths under `tests/fixtures/` (rotation or playlist
    /// order; existence-checked when the plans are built).
    pub fixtures: Vec<String>,
    /// Engine polls between seeks (seek-storm) or the bounded advance
    /// after play (reopen-stress).
    pub advance_polls: u64,
    /// Per-scenario override of the leak ceiling (MB/h).
    pub leak_ceiling_mb_per_h: Option<f64>,
}

/// The runner kinds (the schema's `kind` enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Duration-driven: open → play → advance to Ended → close, over
    /// the fixture rotation (the pure playout path).
    PlayLoop,
    /// Duration-driven: a tight seek rotation with small advances;
    /// reopen when the session leaves the playable states.
    SeekStorm,
    /// Duration-driven: strict sequential playlist passes with
    /// wrap-around counting.
    PlaylistCycle,
    /// Count-driven: N rapid open → settle → close cycles (no
    /// play-to-end), fd/RSS/live growth detection.
    OpenCloseChurn,
    /// Count-driven: N full open → play → close cycles (the 8.C1
    /// reopen-cycle stress), fd/RSS/live growth detection.
    ReopenStress,
}

impl Kind {
    /// Parse the wire name (the schema enum, verbatim).
    fn parse(text: &str) -> Result<Kind, String> {
        match text {
            "play-loop" => Ok(Kind::PlayLoop),
            "seek-storm" => Ok(Kind::SeekStorm),
            "playlist-cycle" => Ok(Kind::PlaylistCycle),
            "open-close-churn" => Ok(Kind::OpenCloseChurn),
            "reopen-stress" => Ok(Kind::ReopenStress),
            other => Err(format!(
                "unknown scenario kind {other:?} (legal: play-loop, seek-storm, \
                 playlist-cycle, open-close-churn, reopen-stress)"
            )),
        }
    }

    /// The stable report label.
    #[must_use = "report label"]
    pub fn label(self) -> &'static str {
        match self {
            Kind::PlayLoop => "play-loop",
            Kind::SeekStorm => "seek-storm",
            Kind::PlaylistCycle => "playlist-cycle",
            Kind::OpenCloseChurn => "open-close-churn",
            Kind::ReopenStress => "reopen-stress",
        }
    }

    /// Whether the runner is driven by `duration_min` (vs `cycles`).
    #[must_use = "driver axis"]
    pub fn is_duration_driven(self) -> bool {
        matches!(self, Kind::PlayLoop | Kind::SeekStorm | Kind::PlaylistCycle)
    }
}

// ---------------------------------------------------------------------------
// The raw value model (load-time only)
// ---------------------------------------------------------------------------

/// One parsed TOML-subset value (before typed extraction).
#[derive(Debug, Clone)]
enum RawValue {
    /// A quoted string.
    Str(String),
    /// An integer.
    Int(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
    /// A single-line array of quoted strings.
    StrArray(Vec<String>),
}

/// Which section a `key = value` line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    /// Above any table (only `schema` is legal here).
    Top,
    /// `[defaults]`.
    Defaults,
    /// The `n`-th `[[scenario]]` row.
    Scenario(usize),
}

// ---------------------------------------------------------------------------
// The loader
// ---------------------------------------------------------------------------

/// Load and validate a manifest from `path`.
///
/// # Errors
/// A human sentence for every shape violation (missing file, unknown
/// key, wrong type, out-of-range value, duplicate id, wrong kind
/// axes) — garbage never silently becomes a default.
pub fn load(path: &Path) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("soak manifest {}: {error}", path.display()))?;
    let source = path.display().to_string();

    let mut top: HashMap<String, RawValue> = HashMap::new();
    let mut defaults_raw: HashMap<String, RawValue> = HashMap::new();
    let mut scenarios_raw: Vec<HashMap<String, RawValue>> = Vec::new();
    let mut section = Section::Top;

    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line_no = index + 1;
        if let Some(name) = line.strip_prefix("[[scenario]]") {
            if !name.trim().is_empty() {
                return Err(format!(
                    "{source}:{line_no}: trailing text after [[scenario]]: {name:?}"
                ));
            }
            scenarios_raw.push(HashMap::new());
            section = Section::Scenario(scenarios_raw.len().saturating_sub(1));
            continue;
        }
        if line.starts_with("[[") {
            return Err(format!(
                "{source}:{line_no}: unknown array table {line:?} (only [[scenario]] is legal)"
            ));
        }
        if let Some(name) = line.strip_prefix('[') {
            let name = name.strip_suffix(']').unwrap_or(name);
            section = match name.trim() {
                "defaults" => Section::Defaults,
                other => {
                    return Err(format!(
                        "{source}:{line_no}: unknown section [{other}] (only [defaults] is legal)"
                    ));
                }
            };
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!(
                "{source}:{line_no}: not a key = value line: {line:?}"
            ));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("{source}:{line_no}: empty key in {line:?}"));
        }
        let value = parse_value(value.trim(), &source, line_no)?;
        let target = match section {
            Section::Top => &mut top,
            Section::Defaults => &mut defaults_raw,
            Section::Scenario(index) => scenarios_raw
                .get_mut(index)
                .ok_or_else(|| format!("{source}:{line_no}: value before any [[scenario]]"))?,
        };
        if target.contains_key(key) {
            return Err(format!(
                "{source}:{line_no}: duplicate key {key:?} in the same section"
            ));
        }
        target.insert(key.to_string(), value);
    }

    // ---- top level: only `schema`, and it must be 1 ----
    for key in top.keys() {
        if key != "schema" {
            return Err(format!(
                "{source}: unknown top-level key {key:?} (only `schema` is legal)"
            ));
        }
    }
    let schema = raw_int(&top, "schema", &source, "<top>")?;
    if schema != SCHEMA_VERSION {
        return Err(format!(
            "{source}: schema = {schema} (this loader accepts only {SCHEMA_VERSION}; bump both \
             the loader and tests/soak/scenarios.schema.json together)"
        ));
    }

    let defaults = parse_defaults(&defaults_raw, &source)?;
    let mut scenarios = Vec::with_capacity(scenarios_raw.len());
    let mut seen_ids = Vec::new();
    for (index, raw) in scenarios_raw.iter().enumerate() {
        let spec = parse_scenario(raw, &source, index)?;
        if seen_ids.contains(&spec.id) {
            return Err(format!(
                "{source}: duplicate scenario id {:?} (ids are the report's keys; they must be \
                 unique)",
                spec.id
            ));
        }
        seen_ids.push(spec.id.clone());
        scenarios.push(spec);
    }
    if scenarios.is_empty() {
        return Err(format!(
            "{source}: no [[scenario]] rows — a suite with no scenarios is a ritual, not a \
             measurement (D-6)"
        ));
    }
    Ok(Manifest {
        source,
        defaults,
        scenarios,
    })
}

/// Parse one raw value (quoted string / int / float / bool /
/// single-line string array).
fn parse_value(text: &str, source: &str, line_no: usize) -> Result<RawValue, String> {
    let where_ = format!("{source}:{line_no}");
    if let Some(stripped) = text.strip_prefix('"') {
        let close = stripped
            .find('"')
            .ok_or_else(|| format!("{where_}: unterminated string {text:?}"))?;
        if !stripped[close + 1..].trim().is_empty() {
            return Err(format!(
                "{where_}: trailing text after the closing quote in {text:?}"
            ));
        }
        let inner = &stripped[..close];
        if inner.contains('"') {
            return Err(format!(
                "{where_}: embedded quotes are not supported in {text:?}"
            ));
        }
        return Ok(RawValue::Str(inner.to_string()));
    }
    if text.starts_with('[') {
        if !text.ends_with(']') {
            return Err(format!(
                "{where_}: multi-line arrays are not supported (the boundaries.toml reader \
                 rule): {text:?}"
            ));
        }
        let inner = text[1..text.len() - 1].trim();
        let mut items = Vec::new();
        if !inner.is_empty() {
            for item in inner.split(',') {
                let item = item.trim();
                let Some(stripped) = item.strip_prefix('"') else {
                    return Err(format!(
                        "{where_}: array items must be quoted strings: {item:?}"
                    ));
                };
                let close = stripped
                    .find('"')
                    .ok_or_else(|| format!("{where_}: unterminated array item {item:?}"))?;
                if !stripped[close + 1..].trim().is_empty() {
                    return Err(format!("{where_}: trailing text in array item {item:?}"));
                }
                items.push(stripped[..close].to_string());
            }
        }
        return Ok(RawValue::StrArray(items));
    }
    if text == "true" {
        return Ok(RawValue::Bool(true));
    }
    if text == "false" {
        return Ok(RawValue::Bool(false));
    }
    if let Ok(int) = text.parse::<i64>() {
        return Ok(RawValue::Int(int));
    }
    if let Ok(float) = text.parse::<f64>()
        && float.is_finite()
    {
        return Ok(RawValue::Float(float));
    }
    Err(format!(
        "{where_}: value {text:?} is not a quoted string, integer, float, boolean or \
         single-line string array"
    ))
}

/// Extract a required integer.
fn raw_int(
    map: &HashMap<String, RawValue>,
    key: &str,
    source: &str,
    section: &str,
) -> Result<i64, String> {
    match map.get(key) {
        Some(RawValue::Int(value)) => Ok(*value),
        Some(other) => Err(format!(
            "{source}: [{section}] {key} must be an integer, found {other:?}"
        )),
        None => Err(format!(
            "{source}: [{section}] {key} is required (the schema's required list)"
        )),
    }
}

/// Extract a required float.
fn raw_float(
    map: &HashMap<String, RawValue>,
    key: &str,
    source: &str,
    section: &str,
) -> Result<f64, String> {
    match map.get(key) {
        Some(RawValue::Float(value)) => Ok(*value),
        Some(RawValue::Int(value)) => Ok(*value as f64),
        Some(other) => Err(format!(
            "{source}: [{section}] {key} must be a number, found {other:?}"
        )),
        None => Err(format!(
            "{source}: [{section}] {key} is required (the schema's required list)"
        )),
    }
}

/// Extract a required boolean.
fn raw_bool(
    map: &HashMap<String, RawValue>,
    key: &str,
    source: &str,
    section: &str,
) -> Result<bool, String> {
    match map.get(key) {
        Some(RawValue::Bool(value)) => Ok(*value),
        Some(other) => Err(format!(
            "{source}: [{section}] {key} must be a boolean, found {other:?}"
        )),
        None => Err(format!(
            "{source}: [{section}] {key} is required (the schema's required list)"
        )),
    }
}

/// Extract a required string.
fn raw_str(
    map: &HashMap<String, RawValue>,
    key: &str,
    source: &str,
    section: &str,
) -> Result<String, String> {
    match map.get(key) {
        Some(RawValue::Str(value)) => Ok(value.clone()),
        Some(other) => Err(format!(
            "{source}: [{section}] {key} must be a quoted string, found {other:?}"
        )),
        None => Err(format!(
            "{source}: [{section}] {key} is required (the schema's required list)"
        )),
    }
}

/// Extract a required string array.
fn raw_str_array(
    map: &HashMap<String, RawValue>,
    key: &str,
    source: &str,
    section: &str,
) -> Result<Vec<String>, String> {
    match map.get(key) {
        Some(RawValue::StrArray(items)) => Ok(items.clone()),
        Some(other) => Err(format!(
            "{source}: [{section}] {key} must be a single-line string array, found {other:?}"
        )),
        None => Err(format!(
            "{source}: [{section}] {key} is required (the schema's required list)"
        )),
    }
}

/// Extract an optional integer.
fn raw_opt_int(
    map: &HashMap<String, RawValue>,
    key: &str,
    source: &str,
    section: &str,
) -> Result<Option<i64>, String> {
    match map.get(key) {
        Some(RawValue::Int(value)) => Ok(Some(*value)),
        Some(other) => Err(format!(
            "{source}: [{section}] {key} must be an integer, found {other:?}"
        )),
        None => Ok(None),
    }
}

/// Extract an optional float (an integer literal is accepted).
fn raw_opt_float(
    map: &HashMap<String, RawValue>,
    key: &str,
    source: &str,
    section: &str,
) -> Result<Option<f64>, String> {
    match map.get(key) {
        Some(RawValue::Float(value)) => Ok(Some(*value)),
        Some(RawValue::Int(value)) => Ok(Some(*value as f64)),
        Some(other) => Err(format!(
            "{source}: [{section}] {key} must be a number, found {other:?}"
        )),
        None => Ok(None),
    }
}

/// The legal `[defaults]` keys (the schema's `additionalProperties:
/// false` set — `pub` for the binary root's loader/schema drift
/// check).
pub const DEFAULTS_KEYS: [&str; 10] = [
    "sample_ms",
    "warmup_fraction",
    "min_cycles",
    "max_skip_rate",
    "leak_ceiling_mb_per_h",
    "target_mb_per_h",
    "assert_target",
    "rss_growth_budget_mb",
    "fd_growth_budget",
    "live_growth_budget_bytes",
];

/// Parse the `[defaults]` table with range checks (the schema's
/// minimum/maximum/exclusiveMinimum, mirrored).
fn parse_defaults(raw: &HashMap<String, RawValue>, source: &str) -> Result<Defaults, String> {
    for key in raw.keys() {
        if !DEFAULTS_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "{source}: [defaults] unknown key {key:?} (legal: {})",
                DEFAULTS_KEYS.join(", ")
            ));
        }
    }
    let sample_ms = raw_int(raw, "sample_ms", source, "defaults")?;
    if !(1..=600_000).contains(&sample_ms) {
        return Err(format!(
            "{source}: [defaults] sample_ms = {sample_ms} is outside [1, 600000]"
        ));
    }
    let warmup_fraction = raw_float(raw, "warmup_fraction", source, "defaults")?;
    if !(0.0..=0.5).contains(&warmup_fraction) || warmup_fraction <= 0.0 {
        return Err(format!(
            "{source}: [defaults] warmup_fraction = {warmup_fraction} is outside (0, 0.5]"
        ));
    }
    let min_cycles = raw_int(raw, "min_cycles", source, "defaults")?;
    if min_cycles < 1 {
        return Err(format!(
            "{source}: [defaults] min_cycles = {min_cycles} must be >= 1"
        ));
    }
    let max_skip_rate = raw_float(raw, "max_skip_rate", source, "defaults")?;
    if !(0.0..=1.0).contains(&max_skip_rate) || max_skip_rate <= 0.0 {
        return Err(format!(
            "{source}: [defaults] max_skip_rate = {max_skip_rate} is outside (0, 1]"
        ));
    }
    let leak_ceiling_mb_per_h = raw_float(raw, "leak_ceiling_mb_per_h", source, "defaults")?;
    if leak_ceiling_mb_per_h <= 0.0 {
        return Err(format!(
            "{source}: [defaults] leak_ceiling_mb_per_h = {leak_ceiling_mb_per_h} must be > 0"
        ));
    }
    let target_mb_per_h = raw_float(raw, "target_mb_per_h", source, "defaults")?;
    if target_mb_per_h <= 0.0 {
        return Err(format!(
            "{source}: [defaults] target_mb_per_h = {target_mb_per_h} must be > 0"
        ));
    }
    let rss_growth_budget_mb = raw_float(raw, "rss_growth_budget_mb", source, "defaults")?;
    if rss_growth_budget_mb <= 0.0 {
        return Err(format!(
            "{source}: [defaults] rss_growth_budget_mb = {rss_growth_budget_mb} must be > 0"
        ));
    }
    let fd_growth_budget = raw_int(raw, "fd_growth_budget", source, "defaults")?;
    if fd_growth_budget < 0 {
        return Err(format!(
            "{source}: [defaults] fd_growth_budget = {fd_growth_budget} must be >= 0"
        ));
    }
    let live_growth_budget_bytes = raw_int(raw, "live_growth_budget_bytes", source, "defaults")?;
    if live_growth_budget_bytes < 0 {
        return Err(format!(
            "{source}: [defaults] live_growth_budget_bytes = {live_growth_budget_bytes} must be \
             >= 0"
        ));
    }
    Ok(Defaults {
        sample_ms: sample_ms.unsigned_abs(),
        warmup_fraction,
        min_cycles: min_cycles.unsigned_abs(),
        max_skip_rate,
        leak_ceiling_mb_per_h,
        target_mb_per_h,
        assert_target: raw_bool(raw, "assert_target", source, "defaults")?,
        rss_growth_budget_mb,
        fd_growth_budget: fd_growth_budget.unsigned_abs(),
        live_growth_budget_bytes: live_growth_budget_bytes.unsigned_abs(),
    })
}

/// The legal `[[scenario]]` keys (the schema's
/// `additionalProperties: false` set — `pub` for the binary root's
/// loader/schema drift check).
pub const SCENARIO_KEYS: [&str; 7] = [
    "id",
    "kind",
    "duration_min",
    "cycles",
    "fixtures",
    "advance_polls",
    "leak_ceiling_mb_per_h",
];

/// The scenario-id pattern (the schema's `pattern`): lowercase
/// alphanumerics and hyphens, starting alphanumeric.
fn id_is_well_formed(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return false;
    }
    if id.len() > 64 {
        return false;
    }
    id.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parse one `[[scenario]]` row with kind-axes and range checks.
fn parse_scenario(
    raw: &HashMap<String, RawValue>,
    source: &str,
    index: usize,
) -> Result<ScenarioSpec, String> {
    let section = format!("scenario {index}");
    for key in raw.keys() {
        if !SCENARIO_KEYS.contains(&key.as_str()) {
            return Err(format!(
                "{source}: [[scenario]] #{index} unknown key {key:?} (legal: {})",
                SCENARIO_KEYS.join(", ")
            ));
        }
    }
    let id = raw_str(raw, "id", source, &section)?;
    if !id_is_well_formed(&id) {
        return Err(format!(
            "{source}: [[scenario]] #{index} id {id:?} must match ^[a-z0-9][a-z0-9-]*$ (1-64 \
             chars) — ids are the report's stable keys"
        ));
    }
    let kind = Kind::parse(&raw_str(raw, "kind", source, &section)?)?;
    let duration_min = raw_opt_float(raw, "duration_min", source, &section)?;
    if let Some(minutes) = duration_min
        && (!(0.0..=10_080.0).contains(&minutes) || minutes <= 0.0)
    {
        return Err(format!(
            "{source}: [[scenario]] {id:?} duration_min = {minutes} is outside (0, 10080] (a \
             week; the SOAK_MINUTES discipline)"
        ));
    }
    let cycles = raw_opt_int(raw, "cycles", source, &section)?;
    if let Some(count) = cycles
        && count < 1
    {
        return Err(format!(
            "{source}: [[scenario]] {id:?} cycles = {count} must be >= 1"
        ));
    }
    let fixtures = raw_str_array(raw, "fixtures", source, &section)?;
    if fixtures.is_empty() {
        return Err(format!(
            "{source}: [[scenario]] {id:?} fixtures is empty — a scenario with nothing to play \
             is a ritual, not a measurement (D-6)"
        ));
    }
    let advance_polls = raw_opt_int(raw, "advance_polls", source, &section)?.unwrap_or(1);
    if !(0..=1024).contains(&advance_polls) {
        return Err(format!(
            "{source}: [[scenario]] {id:?} advance_polls = {advance_polls} is outside [0, 1024]"
        ));
    }
    let leak_ceiling_mb_per_h = raw_opt_float(raw, "leak_ceiling_mb_per_h", source, &section)?;
    if let Some(ceiling) = leak_ceiling_mb_per_h
        && ceiling <= 0.0
    {
        return Err(format!(
            "{source}: [[scenario]] {id:?} leak_ceiling_mb_per_h = {ceiling} must be > 0"
        ));
    }
    // The kind axes (the schema's per-kind `required` rule).
    if kind.is_duration_driven() {
        if duration_min.is_none() {
            return Err(format!(
                "{source}: [[scenario]] {id:?} kind {} requires duration_min",
                kind.label()
            ));
        }
    } else if cycles.is_none() {
        return Err(format!(
            "{source}: [[scenario]] {id:?} kind {} requires cycles",
            kind.label()
        ));
    }
    Ok(ScenarioSpec {
        id,
        kind,
        duration_min,
        cycles: cycles.map(i64::unsigned_abs),
        fixtures,
        advance_polls: advance_polls.unsigned_abs(),
        leak_ceiling_mb_per_h,
    })
}

// ---------------------------------------------------------------------------
// Parser tests (pure — no fixture or engine dependency)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole-binary serializer (the counting allocator is
    /// process-global; a sibling test's allocations inside a measured
    /// window are pollution).
    fn serial() -> std::sync::MutexGuard<'static, ()> {
        testkit::alloc_count::serialize_tests()
    }

    /// Write a manifest body to a temp file and load it (the loader
    /// takes a path; tests exercise the real read + parse path).
    fn load_text(name: &str, body: &str) -> Result<Manifest, String> {
        let dir = std::env::temp_dir().join("lumen-soak-manifest-tests");
        std::fs::create_dir_all(&dir).must();
        let path = dir.join(name);
        std::fs::write(&path, body.as_bytes()).must();
        load(&path)
    }

    /// The minimal valid body (every required key, in-range values).
    const MINIMAL: &str = "schema = 1\n\
        \n\
        [defaults]\n\
        sample_ms = 250\n\
        warmup_fraction = 0.15\n\
        min_cycles = 3\n\
        max_skip_rate = 0.25\n\
        leak_ceiling_mb_per_h = 50.0\n\
        target_mb_per_h = 5.0\n\
        assert_target = false\n\
        rss_growth_budget_mb = 8.0\n\
        fd_growth_budget = 4\n\
        live_growth_budget_bytes = 524288\n\
        \n\
        [[scenario]]\n\
        id = \"play-loop\"\n\
        kind = \"play-loop\"\n\
        duration_min = 0.25\n\
        fixtures = [\"generated/valid/avi_mjpeg_pcm.avi\"]\n";

    #[test]
    fn minimal_manifest_parses_with_defaults() {
        let _serial = serial();
        let manifest = load_text("minimal.toml", MINIMAL).must();
        assert_eq!(manifest.scenarios.len(), 1);
        assert_eq!(manifest.defaults.sample_ms, 250);
        assert!(!manifest.defaults.assert_target);
        let scenario = &manifest.scenarios[0];
        assert_eq!(scenario.id, "play-loop");
        assert_eq!(scenario.kind, Kind::PlayLoop);
        assert!((scenario.duration_min.must() - 0.25).abs() < 1e-9);
        assert_eq!(scenario.advance_polls, 1, "the documented default");
    }

    #[test]
    fn unknown_keys_refuse_loudly() {
        let _serial = serial();
        let body = MINIMAL.replace(
            "duration_min = 0.25",
            "duration_min = 0.25\nbogus_key = true",
        );
        let error = load_text("unknown-key.toml", &body).err().must();
        assert!(
            error.contains("unknown key") && error.contains("bogus_key"),
            "the sentence names the key: {error}"
        );
    }

    #[test]
    fn unknown_section_and_top_level_refuse() {
        let _serial = serial();
        let error = load_text(
            "unknown-section.toml",
            &format!("schema = 1\n[profile]\nsample_ms = 1\n{MINIMAL}"),
        )
        .err()
        .must();
        assert!(error.contains("unknown section"), "{error}");

        let error = load_text("unknown-top.toml", &format!("profile = \"x\"\n{MINIMAL}"))
            .err()
            .must();
        assert!(error.contains("unknown top-level key"), "{error}");
    }

    #[test]
    fn schema_version_is_checked() {
        let _serial = serial();
        let body = MINIMAL.replacen("schema = 1", "schema = 2", 1);
        let error = load_text("schema-2.toml", &body).err().must();
        assert!(error.contains("accepts only 1"), "{error}");
    }

    #[test]
    fn duplicate_scenario_ids_refuse() {
        let _serial = serial();
        let body = format!(
            "{MINIMAL}\n[[scenario]]\nid = \"play-loop\"\nkind = \"play-loop\"\n\
             duration_min = 1.0\nfixtures = [\"generated/valid/avi_mjpeg_pcm.avi\"]\n"
        );
        let error = load_text("dup-id.toml", &body).err().must();
        assert!(error.contains("duplicate scenario id"), "{error}");
    }

    #[test]
    fn kind_axes_are_enforced() {
        let _serial = serial();
        // A duration kind without duration_min.
        let body = MINIMAL.replace("duration_min = 0.25\n", "");
        let error = load_text("no-duration.toml", &body).err().must();
        assert!(
            error.contains("requires duration_min"),
            "the sentence names the axis: {error}"
        );

        // A count kind without cycles.
        let body = format!(
            "{MINIMAL}\n[[scenario]]\nid = \"churn\"\nkind = \"open-close-churn\"\n\
             fixtures = [\"generated/valid/avi_mjpeg_pcm.avi\"]\n"
        );
        let error = load_text("no-cycles.toml", &body).err().must();
        assert!(
            error.contains("requires cycles"),
            "the sentence names the axis: {error}"
        );
    }

    #[test]
    fn unknown_kind_and_bad_id_refuse() {
        let _serial = serial();
        let body = MINIMAL.replace("kind = \"play-loop\"", "kind = \"teleport\"");
        let error = load_text("bad-kind.toml", &body).err().must();
        assert!(error.contains("unknown scenario kind"), "{error}");

        let body = MINIMAL.replace("id = \"play-loop\"", "id = \"Play Loop!\"");
        let error = load_text("bad-id.toml", &body).err().must();
        assert!(error.contains("must match"), "{error}");
    }

    #[test]
    fn out_of_range_values_refuse() {
        let _serial = serial();
        for (key, value) in [
            ("sample_ms", "0"),
            ("sample_ms", "700000"),
            ("warmup_fraction", "0.9"),
            ("min_cycles", "0"),
            ("leak_ceiling_mb_per_h", "0.0"),
        ] {
            let body = MINIMAL
                .lines()
                .map(|line| {
                    if line.starts_with(&format!("{key} = ")) {
                        format!("{key} = {value}")
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let error = load_text("range.toml", &body).err().must();
            assert!(
                error.contains(key),
                "the sentence names the key {key}: {error}"
            );
        }
    }

    #[test]
    fn multi_line_arrays_refuse() {
        let _serial = serial();
        let body = MINIMAL.replace(
            "fixtures = [\"generated/valid/avi_mjpeg_pcm.avi\"]",
            "fixtures = [\n  \"generated/valid/avi_mjpeg_pcm.avi\",\n]",
        );
        let error = load_text("multiline.toml", &body).err().must();
        assert!(error.contains("multi-line arrays"), "{error}");
    }

    #[test]
    fn empty_fixture_list_refuses() {
        let _serial = serial();
        let body = MINIMAL.replace(
            "fixtures = [\"generated/valid/avi_mjpeg_pcm.avi\"]",
            "fixtures = []",
        );
        let error = load_text("empty-fixtures.toml", &body).err().must();
        assert!(error.contains("fixtures is empty"), "{error}");
    }

    #[test]
    fn missing_file_refuses_with_the_path() {
        let _serial = serial();
        let error = load(std::path::Path::new(
            "/nonexistent/soak-manifest-does-not-exist.toml",
        ))
        .err()
        .must();
        assert!(error.contains("soak manifest"), "{error}");
    }
}
