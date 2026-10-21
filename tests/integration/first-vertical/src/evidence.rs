//! The evidence assembly (2.E1; doc 10 §6.4 transcripts + §7 ledger).
//!
//! One place builds every artifact under `evidence/` so the committed
//! files and the currency checks cannot disagree about what
//! "regenerated" means (the codegen.rs precedent):
//!
//! - `evidence/transcripts/<id>.json` — one per scenario, the full
//!   command script + wire event log + observables (doc 10 §6.4).
//! - `evidence/final-frame.png` — the last presented frame of a whole
//!   run (the headless video-render evidence).
//! - `evidence/ledger.json` — the `evidence/1` envelope (doc 10 §7.2)
//!   with the additive per-scenario table and artifact digests
//!   `[PROPOSAL: additive extensions to the §7.2 envelope]`.
//!
//! The normal gate only READS this directory (the currency checks in
//! `tests/evidence.rs`); only the `--ignored` regeneration run writes
//! it — a gate run never dirties the tree.

use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::harness::ScenarioRun;
use crate::json::Json;
use crate::scenarios;

/// The evidence directory (this crate's `evidence/`).
#[must_use]
pub fn evidence_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("evidence")
}

/// One scenario's transcript path.
#[must_use]
pub fn transcript_path(id: &str) -> PathBuf {
    evidence_dir()
        .join("transcripts")
        .join(format!("{id}.json"))
}

/// The final-frame PNG path.
#[must_use]
pub fn final_frame_path() -> PathBuf {
    evidence_dir().join("final-frame.png")
}

/// The ledger path.
#[must_use]
pub fn ledger_path() -> PathBuf {
    evidence_dir().join("ledger.json")
}

/// ISO-8601 UTC for "now" (machine-written per doc 10 §7.2).
#[must_use]
pub fn iso_utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    iso_utc(secs)
}

/// Unix seconds → `YYYY-MM-DDTHH:MM:SSZ` (the proleptic Gregorian
/// civil calendar — Howard Hinnant's `civil_from_days`, integer-exact,
/// valid past year 9999 only by the formatter's grace).
#[must_use]
pub fn iso_utc(secs: u64) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let rem = secs % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // civil_from_days: era-shifted March-based year computation.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// The current `git` HEAD commit (the tree the evidence was
/// regenerated from), or an honest placeholder when git is absent.
#[must_use]
pub fn git_head() -> String {
    match ProcessCommand::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "(git unavailable)".to_string(),
    }
}

/// The doc 10 §7.1 mock-vs-real labels for the paths this suite
/// exercises — the honest columns of the ledger.
#[must_use]
pub fn hardware_labels() -> Json {
    Json::obj(vec![
        (
            "audio_sink",
            Json::obj(vec![
                ("class", Json::str("mock")),
                (
                    "what",
                    Json::str(
                        "null adapter — no audio hardware exists in this sandbox; samples are \
                         mixed by audio-engine and rendered into the null device's ring, whose \
                         sample counter drives the audio-master clock (a moving position after \
                         `play` IS the audio limb being exercised end to end)",
                    ),
                ),
                (
                    "real_path",
                    Json::str(
                        "the typed OS-backend stubs in audio-output (card 2.C3; real backends \
                         land in platform-* per doc 01 §2.7 behind ADR-005 — the cpal P0 \
                         record docs/dependencies/records/cpal-0.18.2.md)",
                    ),
                ),
            ]),
        ),
        (
            "video_render",
            Json::obj(vec![
                ("class", Json::str("mock")),
                (
                    "what",
                    Json::str(
                        "software compositor, headless — no GPU exists in this sandbox; frames \
                         stage through the measured CPU path and the PNG is encoded by the \
                         std-only stored-deflate encoder",
                    ),
                ),
                (
                    "real_path",
                    Json::str(
                        "the wgpu backend is typed + shape-pinned but deferred per the 2.D1 \
                         record docs/dependencies/records/wgpu-30.0.1.md (admission gated on \
                         the libloading disposition)",
                    ),
                ),
            ]),
        ),
        (
            "clock",
            Json::obj(vec![
                ("class", Json::str("mock")),
                (
                    "what",
                    Json::str(
                        "virtual time — the host's bounded `advance` pumps are the only time \
                         mover (the poll-driven single-writer model; no wall-clock sleeps \
                         anywhere, hence the byte-determinism proofs)",
                    ),
                ),
                (
                    "real_path",
                    Json::str("the wall-clock session thread is card 3.B3"),
                ),
            ]),
        ),
        (
            "ci_3os_green",
            Json::obj(vec![
                ("status", Json::str("PENDING")),
                (
                    "why",
                    Json::str(
                        "the workflow files exist (.github/workflows-disabled/ci.yml, \
                         actionlint-validated, no runner has executed them in this sandbox); \
                         this suite is a workspace member so the CI `cargo test --workspace` \
                         3-OS matrix covers it once the workflows activate on the first push \
                         with a workflow-scoped token — 2.E2-V's to-run list owns the verdict",
                    ),
                ),
            ]),
        ),
    ])
}

/// Assemble the `evidence/1` ledger for a set of green scenario runs
/// plus the artifact digests.
#[must_use]
pub fn build_ledger(runs: &[ScenarioRun], artifacts: &[(String, Vec<u8>)]) -> Json {
    let labels = crate::harness::Labels::detect();
    let short_sha = {
        let head = git_head();
        head.chars().take(9).collect::<String>()
    };
    let scenarios_table: Vec<Json> = runs
        .iter()
        .map(|run| {
            Json::obj(vec![
                ("id", Json::str(run.id)),
                (
                    "verdict",
                    Json::str(if run.all_passed() { "pass" } else { "fail" }),
                ),
                ("checks", Json::UInt(run.checks.len() as u64)),
                (
                    "claims",
                    Json::Arr(
                        run.checks
                            .iter()
                            .map(|check| Json::str(check.claim.clone()))
                            .collect(),
                    ),
                ),
            ])
        })
        .collect();
    let artifact_table: Vec<Json> = artifacts
        .iter()
        .map(|(path, bytes)| {
            Json::obj(vec![
                ("path", Json::str(path.clone())),
                ("bytes", Json::UInt(bytes.len() as u64)),
                ("sha256", Json::str(crate::digest::sha256_hex(bytes))),
            ])
        })
        .collect();
    Json::obj(vec![
        ("schema", Json::str("evidence/1")),
        ("run_id", Json::str(format!("2e1-local-{short_sha}"))),
        (
            "commit",
            Json::obj(vec![
                ("sha", Json::str(git_head())),
                (
                    "note",
                    Json::str(
                        "the tree the evidence was regenerated FROM; the artifact commit \
                         itself lands one commit later (a file cannot carry its own sha)",
                    ),
                ),
            ]),
        ),
        ("started_at", Json::str(iso_utc_now())),
        (
            "environment",
            Json::obj(vec![
                ("kind", Json::str(labels.environment_kind)),
                ("os", Json::str(labels.os)),
                ("hardware_class", Json::str(labels.hardware_class)),
                (
                    "devices",
                    Json::Arr(labels.devices.iter().map(|d| Json::str(*d)).collect()),
                ),
            ]),
        ),
        (
            "commands",
            Json::Arr(vec![Json::obj(vec![
                (
                    "cmd",
                    Json::str("cargo test -p first-vertical --test evidence -- --ignored regen"),
                ),
                ("exit", Json::UInt(0)),
                (
                    "note",
                    Json::str(
                        "in-process scenario runs through lumen_dispatch (the command layer); \
                         the full workspace gate result is recorded in the worklog entry",
                    ),
                ),
            ])]),
        ),
        (
            "labels",
            Json::Arr(vec![
                Json::str("mock"),
                Json::str("virtual-clock"),
                Json::str("null-audio"),
                Json::str("software-render"),
            ]),
        ),
        ("scenario_table", Json::Arr(scenarios_table)),
        ("artifacts", Json::Arr(artifact_table)),
        ("hardware_labels", hardware_labels()),
    ])
}

/// Regenerate every artifact from a fresh run: run all scenarios,
/// refuse to write when any check fails (evidence only commits green
/// runs — a failed regen is a test failure to fix, not evidence to
/// record), then write transcripts + the final frame + the ledger.
///
/// # Errors
/// [`String`] naming the failing scenario/IO step.
pub fn regen_all() -> Result<String, String> {
    let mut runs = Vec::new();
    for id in scenarios::SCENARIO_IDS {
        let run = scenarios::by_id(id)?;
        if !run.all_passed() {
            let failed: Vec<&str> = run
                .checks
                .iter()
                .filter(|check| !check.passed)
                .map(|check| check.claim.as_str())
                .collect();
            return Err(format!(
                "scenario `{id}` failed its checks — refusing to write \
                 evidence: {failed:?}"
            ));
        }
        runs.push(run);
    }
    let transcripts_dir = evidence_dir().join("transcripts");
    std::fs::create_dir_all(&transcripts_dir)
        .map_err(|error| format!("mkdir {}: {error}", transcripts_dir.display()))?;
    let mut artifacts: Vec<(String, Vec<u8>)> = Vec::new();
    for run in &runs {
        let path = transcript_path(run.id);
        let bytes = run.transcript_bytes();
        std::fs::write(&path, bytes.as_bytes())
            .map_err(|error| format!("write {}: {error}", path.display()))?;
        artifacts.push((
            format!("evidence/transcripts/{}.json", run.id),
            bytes.into_bytes(),
        ));
    }
    // The final frame: the run-to-end run's PNG bytes, verbatim.
    let png = final_png_of(&runs).ok_or_else(|| {
        "the run-to-end scenario produced no screenshot — cannot write \
         the final-frame evidence"
            .to_string()
    })?;
    std::fs::write(final_frame_path(), &png)
        .map_err(|error| format!("write final-frame.png: {error}"))?;
    artifacts.push(("evidence/final-frame.png".to_string(), png));
    let ledger = build_ledger(&runs, &artifacts);
    let ledger_bytes = ledger.render();
    std::fs::write(ledger_path(), ledger_bytes.as_bytes())
        .map_err(|error| format!("write ledger.json: {error}"))?;
    Ok(format!(
        "regenerated {} transcripts + final-frame.png + ledger.json \
         ({} artifacts, all scenarios green)",
        runs.len(),
        artifacts.len()
    ))
}

/// The final-frame PNG out of an assembled run set (the run-to-end
/// run's `png` bytes; its transcript carries the digest/extent record
/// of the same bytes).
fn final_png_of(runs: &[ScenarioRun]) -> Option<Vec<u8>> {
    runs.iter()
        .find(|run| run.id == "run-to-end")
        .and_then(|run| run.png.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The civil-calendar conversion against known instants (the
    /// 2026-10-19 epoch was taken from the python3 `datetime` oracle
    /// at recording time — the first draft's from-memory value was
    /// wrong and the vector caught it).
    #[test]
    fn iso_utc_known_instants() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        // 2000-02-29 (the leap day) 12:00:00 UTC.
        assert_eq!(iso_utc(951_825_600), "2000-02-29T12:00:00Z");
        // 2026-10-19 00:00:00 UTC.
        assert_eq!(iso_utc(1_792_368_000), "2026-10-19T00:00:00Z");
        // 2024-12-31 23:59:59 UTC (the year boundary).
        assert_eq!(iso_utc(1_735_689_599), "2024-12-31T23:59:59Z");
    }

    /// Monotonicity across a day boundary.
    #[test]
    fn iso_utc_monotonic() {
        let a = iso_utc(1_735_689_599);
        let b = iso_utc(1_735_689_600);
        assert!(a < b, "{a} then {b}");
    }

    /// The artifact paths land under this crate's evidence dir.
    #[test]
    fn artifact_paths_are_stable() {
        assert!(transcript_path("open").ends_with("evidence/transcripts/open.json"));
        assert!(final_frame_path().ends_with("evidence/final-frame.png"));
        assert!(ledger_path().ends_with("evidence/ledger.json"));
    }
}
