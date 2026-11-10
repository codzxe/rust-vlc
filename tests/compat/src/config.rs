//! The runner's config (schema `compat-matrix/config/2`).
//!
//! Everything user-visible is here, never in code (the C1 rule, doc 07
//! §9): corpus roots and media extensions, the regenerable-corpus
//! manifest + cache dir, the oracle record + live VLC invocation, the
//! f32 tolerance thresholds (the ratified policy values, cited), the
//! lane step commands, the probe-format/codec → feature-matrix row
//! mapping, and the report paths.
//!
//! Schema history: `/1` → `/2` (2026-11-10, issue #53 CI wiring): the
//! native-decode lane kind gained the REQUIRED `platform` field — the
//! feature-matrix column this host's sweep is evidence for. The 8.B1
//! run hard-coded `linux` (the lane that ran); the 3-OS CI lanes need
//! the attribution explicit, or a windows-runner sweep would propose
//! `linux`-cell transitions from windows evidence. The schema string
//! is bumped because old configs (no `platform`) must be refused, not
//! silently defaulted.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::ToolError;

/// The whole config file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Schema discriminator (`compat-matrix/config/2`).
    pub schema: String,
    /// The workspace root the relative paths resolve against
    /// (captured at load time, not serialized).
    #[serde(skip)]
    pub root: PathBuf,
    /// The corpus definition.
    pub corpus: Corpus,
    /// The oracle definition (records + optional live VLC).
    pub oracle: Oracle,
    /// The platform lanes this run records.
    pub lanes: Vec<Lane>,
    /// Probe-format id → feature-matrix row ids (the "format list").
    pub formats: Vec<FormatRow>,
    /// Codec descriptor → feature-matrix row ids.
    pub codecs: Vec<CodecRow>,
    /// Where the reports land.
    pub reports: Reports,
    /// The feature-matrix claim SNAPSHOT this run diffs against
    /// (the runner's view; `xtask compat-matrix apply-transitions`
    /// re-checks every `from` against the live file — a stale
    /// snapshot can only produce rejected proposals, never a bad
    /// write).
    pub claims: Vec<MatrixClaim>,
    /// Subprocess budgets (seconds) for regen/oracle/lane steps.
    pub budgets: Budgets,
}

/// Committed corpus roots + the regenerable set.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    /// Roots of committed media fixtures, workspace-relative. Each is
    /// walked recursively for `media_extensions` files.
    pub committed_roots: Vec<String>,
    /// The malformed/error-contract root (its rows expect a typed
    /// refusal or concealed damage, never a crash).
    pub malformed_root: String,
    /// File extensions that count as media (case-insensitive).
    pub media_extensions: Vec<String>,
    /// Suffixes to skip while walking (structure maps, oracle hashes).
    pub exclude_suffixes: Vec<String>,
    /// The regenerable corpus (manifest-anchored, ffmpeg-produced).
    pub regen: RegenCorpus,
}

/// The regenerable corpus family (the corpus-big/D-11 pattern: the
/// manifest is committed, the bytes are regenerated on demand).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegenCorpus {
    /// The committed manifest (argvs + sha256 + provenance).
    pub manifest: String,
    /// Where regenerated files land (workspace-relative; must be a
    /// git-ignored path — the config pins it, the runner never writes
    /// inside tracked trees).
    pub cache_dir: String,
    /// The ffmpeg binary.
    pub ffmpeg: String,
}

/// The oracle side.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Oracle {
    /// The committed 2026-11-09 oracle run record (cross-reference).
    pub record: String,
    /// The committed f32 tolerance decision record.
    pub f32_record: String,
    /// The ratified f32 thresholds (RMS/peak) — PASS iff both hold.
    pub f32_tolerance: F32Tolerance,
    /// The live pinned VLC 3.0.23 invocation, when available. `None`
    /// disables the live oracle (cells then cite the committed record
    /// only, labeled as such — never inferred).
    pub vlc: Option<VlcInvocation>,
    /// Rows to compare against the oracle (manifest rels of the
    /// lossless-comparable family).
    pub comparable_rels: Vec<String>,
}

/// The ratified f32 tolerance policy values.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct F32Tolerance {
    /// Max per-row RMS deviation.
    pub rms_max: f64,
    /// Max per-row peak deviation.
    pub peak_max: f64,
}

/// A live VLC invocation (the pinned 3.0.23, extracted locally).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VlcInvocation {
    /// The vlc binary path (absolute or workspace-relative).
    pub bin: String,
    /// Extra environment for the child (e.g. LD_LIBRARY_PATH,
    /// VLC_PLUGIN_PATH of the extracted tree).
    pub env: Vec<(String, String)>,
    /// The expected version line (asserted before use; a mismatched
    /// oracle is refused, not silently used).
    pub version_line: String,
}

/// One recorded platform lane.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lane {
    /// Lane id (`linux-native`, `web-wasm`, `windows`, `macos`).
    pub id: String,
    /// How the lane's evidence is produced.
    pub kind: LaneKind,
    /// Human label for the report header.
    pub label: String,
}

/// What a lane's cells are sourced from.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum LaneKind {
    /// This run's own decode sweep (the host the runner executes on).
    /// `platform` is the feature-matrix column the sweep is evidence
    /// for (`linux` | `windows` | `macos` — `web` is the steps lane,
    /// never a native sweep): the gap rules compare against that
    /// column's claim and the outbox's transitions carry it. REQUIRED
    /// since config/2 — never defaulted (a wrong attribution would
    /// propose transitions for the wrong platform's cells).
    NativeDecode { platform: String },
    /// Command steps executed for real by this run (each recorded:
    /// exit code, duration, artifacts).
    Steps { steps: Vec<Step> },
    /// Externally blocked: no run happened and none is inferred. The
    /// reason + next step are cited verbatim in every cell.
    Blocked {
        reason: String,
        reason_ref: String,
        next_step: String,
        tracking: String,
    },
}

/// One real command step (recorded, never trusted blind).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// What this step proves (one line, shown in the report).
    pub purpose: String,
    /// The command argv (element 0 is the binary).
    pub argv: Vec<String>,
    /// The cwd (workspace-relative; empty = workspace root).
    pub cwd: String,
    /// Artifacts to record (workspace-relative globs are NOT
    /// supported; explicit paths only), with what to note (size).
    pub artifacts: Vec<String>,
    /// Wall-clock budget in seconds.
    pub timeout_s: u64,
}

/// One probe-format row of the matrix.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormatRow {
    /// The probe's format id (what `media-probe` names the container).
    pub probe_format: String,
    /// The feature-matrix row ids this maps to (empty = no row claims
    /// it — recorded honestly as out-of-claim).
    pub matrix_rows: Vec<String>,
    /// VLC's name for the family (report prose).
    pub vlc_name: String,
    /// File extensions whose valid-class corpus files belong to this
    /// row even when the probe refuses them outright (no signature):
    /// the honest attribution for claimed families the open path
    /// cannot even name (asf/wmv/wma today). Files that DO probe are
    /// matched by `probe_format` and never by extension.
    pub extensions: Vec<String>,
}

/// One codec row of the matrix.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodecRow {
    /// The codec descriptor prefix/family the tracks carry
    /// (`pcm_`, `aac`, `adpcm_`, ...).
    pub codec: String,
    /// Whether the descriptor matches by prefix (`pcm_`) or equality.
    pub prefix_match: bool,
    /// The feature-matrix row ids this maps to.
    pub matrix_rows: Vec<String>,
    /// The limb kind this family decodes on.
    pub kind: String,
}

/// One feature-matrix row's claimed statuses (the snapshot the
/// runner diffs against; all four platform columns recorded for
/// honesty, the gap rules act on the native lane's platform).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixClaim {
    /// The feature-matrix row id.
    pub row: String,
    /// The claimed windows status.
    pub windows: String,
    /// The claimed linux status.
    pub linux: String,
    /// The claimed macos status.
    pub macos: String,
    /// The claimed web status.
    pub web: String,
}

impl MatrixClaim {
    /// The claimed status of one platform column (`windows` | `linux`
    /// | `macos` | `web`). Callers pass the native lane's platform —
    /// never a hand-picked column — so a sweep's gap rules always
    /// judge the column the sweep is evidence for. The native
    /// platform is validated to windows/linux/macos at config load,
    /// so the catch-all arm is unreachable in practice and only
    /// exists to keep the match total without a panic (denied here).
    pub fn status_for(&self, platform: &str) -> &str {
        match platform {
            "windows" => &self.windows,
            "linux" => &self.linux,
            "macos" => &self.macos,
            "web" => &self.web,
            _ => &self.linux,
        }
    }
}

/// Report destinations.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reports {
    /// The human markdown table.
    pub markdown: String,
    /// The machine JSON (schema `compat-matrix/report/1`).
    pub json: String,
    /// The raw per-file run rows (JSONL, schema
    /// `compat-matrix/run-rows/1`) — the evidence the reports cite.
    pub raw_rows: String,
    /// The transitions outbox for `xtask compat-matrix
    /// apply-transitions` (schema `compat-matrix/transitions/1`).
    pub transitions: String,
}

/// Subprocess budgets.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    /// Per-file ffmpeg regeneration budget (seconds).
    pub regen_s: u64,
    /// Per-file VLC oracle budget (seconds).
    pub vlc_s: u64,
}

/// Load and validate the config at `path`.
pub fn load(path: &Path) -> Result<Config, ToolError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| ToolError::io(format!("reading config {}", path.display()), e))?;
    let mut config: Config = serde_json::from_str(&text)
        .map_err(|e| ToolError::config(format!("parsing {}: {e}", path.display())))?;
    if config.schema != "compat-matrix/config/2" {
        return Err(ToolError::config(format!(
            "unsupported schema {:?} (want \"compat-matrix/config/2\" — see the config.rs \
             schema-history note)",
            config.schema
        )));
    }
    if config.corpus.media_extensions.is_empty() {
        return Err(ToolError::config("corpus.media_extensions is empty"));
    }
    if config.formats.is_empty() || config.codecs.is_empty() {
        return Err(ToolError::config("formats/codecs mapping is empty"));
    }
    if config.lanes.is_empty() {
        return Err(ToolError::config("lanes is empty"));
    }
    let native = config
        .lanes
        .iter()
        .filter(|lane| matches!(lane.kind, LaneKind::NativeDecode { .. }))
        .count();
    if native != 1 {
        return Err(ToolError::config(format!(
            "exactly one native-decode lane is required (found {native}) — the runner decodes \
             on exactly one host per run"
        )));
    }
    for lane in &config.lanes {
        if let LaneKind::NativeDecode { platform } = &lane.kind
            && !matches!(platform.as_str(), "windows" | "linux" | "macos")
        {
            return Err(ToolError::config(format!(
                "lane {:?}: native-decode platform must be one of windows/linux/macos (got \
                 {platform:?} — `web` is the steps lane, never a native sweep)",
                lane.id
            )));
        }
    }
    let mut lane_ids: Vec<&str> = config.lanes.iter().map(|lane| lane.id.as_str()).collect();
    lane_ids.sort_unstable();
    let before = lane_ids.len();
    lane_ids.dedup();
    if lane_ids.len() != before {
        return Err(ToolError::config("duplicate lane ids"));
    }
    config.root = crate::find_workspace_root()?;
    Ok(config)
}

impl Config {
    /// Resolve a workspace-relative path from this config.
    pub fn rel(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal VALID config/2 document (every required field, no
    /// unknown keys — `deny_unknown_fields` is part of what these
    /// tests protect).
    fn valid_json(schema: &str, native_platform: &str) -> String {
        format!(
            r#"{{
  "schema": "{schema}",
  "corpus": {{
    "committed_roots": ["tests/fixtures/media"],
    "malformed_root": "tests/fixtures/malformed",
    "media_extensions": [".avi"],
    "exclude_suffixes": [],
    "regen": {{
      "manifest": "tests/compat/regen-corpus-manifest.json",
      "cache_dir": "tests/compat/.regen-cache",
      "ffmpeg": "ffmpeg"
    }}
  }},
  "oracle": {{
    "record": "docs/corpus/vlc-oracle-run-20261109.json",
    "f32_record": "docs/corpus/vlc-f32-tolerance-20261109.json",
    "f32_tolerance": {{ "rms_max": 0.0001, "peak_max": 0.001 }},
    "vlc": null,
    "comparable_rels": []
  }},
  "lanes": [
    {{
      "id": "linux-native",
      "label": "Linux native decode",
      "kind": {{ "type": "native-decode", "platform": "{native_platform}" }}
    }}
  ],
  "formats": [
    {{
      "probe_format": "avi",
      "matrix_rows": ["container-avi"],
      "vlc_name": "AVI",
      "extensions": ["avi"]
    }}
  ],
  "codecs": [
    {{
      "codec": "pcm_",
      "prefix_match": true,
      "matrix_rows": ["codec-audio-pcm"],
      "kind": "audio"
    }}
  ],
  "reports": {{
    "markdown": "docs/corpus/out.md",
    "json": "docs/corpus/out.json",
    "raw_rows": "docs/corpus/out.raw-rows.jsonl",
    "transitions": "docs/corpus/out-transitions.json"
  }},
  "claims": [],
  "budgets": {{ "regen_s": 60, "vlc_s": 60 }}
}}"#
        )
    }

    /// A per-load unique id for the temp-file name. Process-wide on
    /// purpose: the test harness runs these tests on parallel threads,
    /// so a thread-local counter would hand two threads the same
    /// first name and race them onto one temp file.
    fn unique_id() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Write a config to a unique temp file and load it.
    fn load_json(text: &str) -> Result<Config, ToolError> {
        let path = std::env::temp_dir().join(format!(
            "compat-matrix-config-test-{}-{}.json",
            std::process::id(),
            unique_id()
        ));
        std::fs::write(&path, text)
            .map_err(|e| ToolError::io("writing the test config".to_string(), e))?;
        let result = load(&path);
        let _ = std::fs::remove_file(&path);
        result
    }

    /// The refusal text of a rejected load ("ok" when the config
    /// unexpectedly LOADED — the follow-up contains-assertions then
    /// fail with exactly that visible in their message).
    fn err_text(result: Result<Config, ToolError>) -> String {
        match result {
            Ok(_) => "ok (the config unexpectedly loaded)".to_string(),
            Err(err) => err.to_string(),
        }
    }

    /// Insert one more lane object at the HEAD of the lanes array
    /// (splice inside the array — inserting before a later key such
    /// as `codecs` would land at the JSON top level and die as a
    /// parse error before any validation under test runs).
    fn insert_lane(text: &str, lane: &str) -> String {
        const MARKER: &str = "\"lanes\": [";
        let Some(index) = text.find(MARKER) else {
            unreachable!("the {MARKER} marker is in valid_json");
        };
        let mut out = text.to_string();
        out.insert_str(index + MARKER.len(), &format!("\n    {lane},"));
        out
    }

    /// P3-2: the schema discriminator is refused loudly (an old /1
    /// config must never be silently defaulted).
    #[test]
    fn wrong_schema_is_refused() {
        let message = err_text(load_json(&valid_json("compat-matrix/config/1", "linux")));
        assert!(message.contains("unsupported schema"), "{message}");
        assert!(message.contains("compat-matrix/config/2"), "{message}");
    }

    /// P3-2: the native-decode platform is validated to
    /// windows/linux/macos (`web` is the steps lane, never a native
    /// sweep — the config/2 rule).
    #[test]
    fn web_native_platform_is_refused() {
        let message = err_text(load_json(&valid_json("compat-matrix/config/2", "web")));
        assert!(
            message.contains("native-decode platform must be one of windows/linux/macos"),
            "{message}"
        );
        assert!(message.contains("`web` is the steps lane"), "{message}");
    }

    /// P3-2: exactly one native-decode lane (the runner decodes on
    /// exactly one host per run).
    #[test]
    fn two_native_lanes_are_refused() {
        let extra = r#"{
      "id": "mac-native",
      "label": "macOS native decode",
      "kind": { "type": "native-decode", "platform": "macos" }
    }"#;
        let text = insert_lane(&valid_json("compat-matrix/config/2", "linux"), extra);
        let message = err_text(load_json(&text));
        assert!(
            message.contains("exactly one native-decode lane is required"),
            "{message}"
        );
    }

    /// P3-2: the structural validations — empty lanes, duplicate
    /// lane ids, empty media extensions.
    #[test]
    fn structural_rejections() {
        // Empty lanes.
        let mut empty_lanes = valid_json("compat-matrix/config/2", "linux");
        const LANE_OPEN: &str = "\"lanes\": [";
        let Some(open) = empty_lanes.find(LANE_OPEN) else {
            unreachable!("the {LANE_OPEN} marker is in valid_json");
        };
        let lane_start = open + LANE_OPEN.len();
        let Some(relative) = empty_lanes[lane_start..].find(']') else {
            unreachable!("the lanes array closes in valid_json");
        };
        empty_lanes.replace_range(lane_start..lane_start + relative, "");
        let message = err_text(load_json(&empty_lanes));
        assert!(message.contains("lanes is empty"), "{message}");

        // Duplicate lane ids (a second, steps lane with the same id).
        let extra = r#"{
      "id": "linux-native",
      "label": "duplicate",
      "kind": { "type": "steps", "steps": [] }
    }"#;
        let dup = insert_lane(&valid_json("compat-matrix/config/2", "linux"), extra);
        let message = err_text(load_json(&dup));
        assert!(message.contains("duplicate lane ids"), "{message}");

        // Empty media extensions.
        let no_ext = valid_json("compat-matrix/config/2", "linux").replace(
            "\"media_extensions\": [\".avi\"]",
            "\"media_extensions\": []",
        );
        let message = err_text(load_json(&no_ext));
        assert!(message.contains("media_extensions is empty"), "{message}");
    }

    /// P3-2 (control): the minimal valid document loads — schema/2,
    /// one linux native lane, workspace root resolved.
    #[test]
    fn minimal_valid_config_loads() {
        let config = match load_json(&valid_json("compat-matrix/config/2", "linux")) {
            Ok(config) => config,
            Err(err) => unreachable!("the minimal valid config must load: {err}"),
        };
        assert_eq!(config.schema, "compat-matrix/config/2");
        assert_eq!(config.lanes.len(), 1);
        assert!(
            config.root.join("docs").is_dir(),
            "the workspace root must have been resolved"
        );
    }
}
