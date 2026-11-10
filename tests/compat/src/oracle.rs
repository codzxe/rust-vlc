//! The oracle side: the pinned VLC 3.0.23 differential.
//!
//! Method (documented, reused — not reinvented):
//!
//! * **Integer PCM** (`u8`, `s16`): the **MD5-prefix method** of
//!   `docs/corpus/vlc-diff-report-20261023.md` — VLC's `afile` dump is
//!   a bit-exact PREFIX of the decoded payload; the shortfall is the
//!   unflushed tail. A row is *bit-exact* iff
//!   `md5(lumen_pcm[..vlc_bytes]) == vlc_md5`.
//! * **`pcm_f32le`**: the **ratified tolerance path** (Lead decision
//!   2026-11-09, `docs/corpus/vlc-f32-tolerance-20261109.json`) —
//!   per-row RMS ≤ 1e-4 AND peak ≤ 1e-3 over the head-aligned
//!   VLC-delivered window; thresholds come from the config.
//!
//! The live oracle is the pinned Debian build of VLC 3.0.23, version
//! asserted before any comparison (a mismatched oracle is refused).
//! When no live VLC is available — not configured, or the configured
//! binary absent on this host (the windows/macos CI lanes) — the cells
//! cite the committed 2026-11-09 record, labeled `committed-record`,
//! never inferred; a PRESENT but version-mismatched oracle stays a
//! hard refusal.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ToolError;
use crate::config::{Config, VlcInvocation};
use crate::decode::DecodeDump;
use crate::proc::{self, ProcRecord};

/// One oracle comparison row.
#[derive(Debug, Clone, Serialize)]
pub struct OracleRow {
    /// The corpus file (cache-relative id).
    pub rel: String,
    /// The VLC `afile` sample format used.
    pub audiofile_format: String,
    /// Lumen's decoded PCM bytes.
    pub lumen_bytes: u64,
    /// Lumen's PCM MD5 (lowercase hex).
    pub lumen_md5: String,
    /// Lumen's emitted sample format (`u8`/`s16`/`f32`).
    pub lumen_sample: String,
    /// VLC's dump bytes (live run) — the committed record's value when
    /// the live oracle is off.
    pub vlc_bytes: u64,
    /// VLC's dump MD5.
    pub vlc_md5: String,
    /// Where the VLC side came from.
    pub vlc_source: VlcSource,
    /// The verdict.
    pub verdict: OracleVerdict,
    /// f32 rows: the measured deviations (samples compared).
    pub deviation: Option<Deviation>,
}

/// Where the VLC side of a row came from.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VlcSource {
    /// This run executed the pinned VLC (the subprocess record rides
    /// the run metadata).
    LiveRun,
    /// The committed 2026-11-09 record's values were APPLIED to this
    /// row — the prefix method over its (vlc_md5, vlc_bytes), or its
    /// own measured f32 deviations echoed against the config's
    /// thresholds (no live VLC configured).
    CommittedRecord,
    /// Neither side: no comparison happened, so nothing from any VLC
    /// side is cited — Lumen refused the row, or no live oracle ran
    /// and the committed record was inapplicable (a fresh
    /// realization) or absent (review finding P3-5: these rows used
    /// to say `committed-record` with empty vlc_md5/vlc_bytes, though
    /// no VLC side had been consulted).
    None,
}

/// The measured f32 deviation.
#[derive(Debug, Clone, Serialize)]
pub struct Deviation {
    /// Sample instants compared.
    pub samples: u64,
    /// RMS deviation.
    pub rms: f64,
    /// Peak (max abs) deviation.
    pub peak: f64,
    /// The ratified thresholds (from the config, echoed for audit).
    pub rms_max: f64,
    pub peak_max: f64,
}

/// The per-row verdict.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "verdict")]
pub enum OracleVerdict {
    /// md5(lumen[..vlc_bytes]) == vlc_md5 (the documented prefix
    /// method; tail = lumen - vlc bytes unflushed on VLC's side).
    BitExactPrefix { unflushed_tail_bytes: u64 },
    /// The f32 tolerance path PASSED (RMS and peak within policy).
    TolerancePass,
    /// The f32 tolerance path FAILED (a genuine parity gap, recorded).
    ToleranceFail,
    /// The bytes disagree beyond the prefix method (integer row).
    NotBitExact,
    /// Lumen could not decode the row (the run's refusal is the
    /// record; no oracle claim is made).
    LumenRefused { reason: String },
    /// The live oracle could not produce a dump (timeout/nonzero).
    OracleFailed { reason: String },
    /// No live oracle and no committed record entry: nothing claimed.
    NoComparison,
}

/// The committed oracle record's row shape (subset read).
#[derive(Debug, Deserialize)]
pub struct CommittedRow {
    /// The record's rel key (`media/0022-avi-av.avi`).
    pub rel: String,
    #[serde(default)]
    lumen_fresh_md5: Option<String>,
    vlc_md5: String,
    vlc_bytes: u64,
    /// f32 rows: the measured RMS deviation (the 2026-11-09 run
    /// computed it from the actual PCM on both sides).
    #[serde(default)]
    rms_dev: Option<f64>,
    /// f32 rows: the measured peak deviation.
    #[serde(default)]
    peak_dev: Option<f64>,
    /// f32 rows: the sample instants compared.
    #[serde(default)]
    samples_compared: Option<u64>,
}

/// The live-oracle state after the version assertion.
pub enum OracleState {
    /// The pinned VLC is present and its version line asserted.
    Live(LiveOracle),
    /// No live oracle: the recorded reason (cited verbatim in the
    /// report). Rows then cite the committed 2026-11-09 record —
    /// labeled `committed-record`, never inferred. An EXISTING but
    /// version-mismatched oracle is still a hard refusal, never a
    /// silent downgrade (see `assert_version`).
    Absent(String),
}

/// The live-oracle handle: an asserted VLC invocation.
pub struct LiveOracle {
    invocation: VlcInvocation,
    bin: PathBuf,
    /// The version assertion's record (run once, lazily).
    version_record: Option<ProcRecord>,
}

/// One VLC `afile` dump attempt's outcome: the usable dump, or the
/// recorded reason the oracle could not produce one (a row-level
/// fact, never a tool failure).
#[derive(Debug, Clone)]
pub enum VlcDump {
    /// The dump's bytes + its MD5 (lowercase hex).
    Ok { bytes: Vec<u8>, md5: String },
    /// VLC ran but produced no usable dump (reason verbatim).
    Failed { reason: String },
}

impl LiveOracle {
    /// Resolve the live oracle: present + version-asserted, or the
    /// recorded reason it is absent. A configured binary that does not
    /// exist on this host is an ABSENCE (the committed record is cited
    /// instead, labeled) — the 3-OS CI lanes run where the pinned
    /// Debian build is not installed. A configured binary whose
    /// version line does not match the pin is a hard ERROR (an unpinned
    /// oracle is refused, never silently used).
    pub fn assert_version(config: &Config) -> Result<OracleState, ToolError> {
        let Some(invocation) = config.oracle.vlc.clone() else {
            return Ok(OracleState::Absent("no live oracle configured".to_string()));
        };
        let bin = config.rel(&invocation.bin);
        if !bin.exists() {
            return Ok(OracleState::Absent(format!(
                "the configured live VLC binary {} does not exist on this host — the committed \
                 2026-11-09 record is cited instead (labeled committed-record, never inferred)",
                bin.display()
            )));
        }
        let mut oracle = LiveOracle {
            invocation,
            bin,
            version_record: None,
        };
        let argv = vec![oracle.bin.display().to_string(), "--version".to_string()];
        let record = proc::run_bounded(&argv, None, &oracle.invocation.env, 30)?;
        let version_ok = record
            .stdout_head
            .lines()
            .any(|line| line.contains(&oracle.invocation.version_line));
        if !version_ok {
            return Err(ToolError::config(format!(
                "the configured VLC's version line does not contain {:?} — refusing to use \
                 an unpinned oracle (stdout: {})",
                oracle.invocation.version_line, record.stdout_head
            )));
        }
        oracle.version_record = Some(record);
        Ok(OracleState::Live(oracle))
    }

    /// Dump one file's first audio track with VLC (`afile`), answer
    /// the raw dump's bytes + md5. The inner `Result` distinguishes
    /// "VLC ran but the dump is unusable" (recorded as a row) from
    /// tool-level failures.
    pub fn dump(
        &self,
        input: &Path,
        format: &str,
        out: &Path,
        timeout_s: u64,
    ) -> Result<VlcDump, ToolError> {
        let argv = vec![
            self.bin.display().to_string(),
            "-I".to_string(),
            "dummy".to_string(),
            "--play-and-exit".to_string(),
            "--vout".to_string(),
            "none".to_string(),
            "--aout".to_string(),
            "afile".to_string(),
            "--no-audiofile-wav".to_string(),
            "--audiofile-format".to_string(),
            format.to_string(),
            "--audiofile-file".to_string(),
            out.display().to_string(),
            input.display().to_string(),
        ];
        let record = proc::run_bounded(&argv, None, &self.invocation.env, timeout_s)?;
        if !record.outcome.starts_with("exit 0") {
            return Ok(VlcDump::Failed {
                reason: format!(
                    "vlc {}: {}",
                    record.outcome,
                    record.stderr_tail.chars().take(400).collect::<String>()
                ),
            });
        }
        let bytes = std::fs::read(out)
            .map_err(|e| ToolError::io(format!("reading vlc dump {}", out.display()), e))?;
        let mut md5 = bench_vs_vlc::md5::Md5::new();
        md5.update(&bytes);
        Ok(VlcDump::Ok {
            bytes,
            md5: md5.finalize_hex(),
        })
    }
}

/// The prefix comparison (integer PCM): md5 of the Lumen payload
/// truncated to the VLC dump's length must equal the VLC dump's md5.
pub fn prefix_verdict(lumen: &[u8], vlc_md5: &str, vlc_bytes: u64) -> OracleVerdict {
    if vlc_bytes > lumen.len() as u64 {
        return OracleVerdict::NotBitExact;
    }
    let mut md5 = bench_vs_vlc::md5::Md5::new();
    md5.update(&lumen[..vlc_bytes as usize]);
    if md5.finalize_hex() == vlc_md5 {
        OracleVerdict::BitExactPrefix {
            unflushed_tail_bytes: lumen.len() as u64 - vlc_bytes,
        }
    } else {
        OracleVerdict::NotBitExact
    }
}

/// The f32 tolerance comparison over the head-aligned VLC window.
pub fn f32_verdict(
    lumen: &[u8],
    vlc: &[u8],
    rms_max: f64,
    peak_max: f64,
) -> (OracleVerdict, Deviation) {
    // Head-aligned window: compare min(len) bytes as f32 LE samples.
    let comparable = lumen.len().min(vlc.len());
    let samples = comparable / 4;
    let mut sum_sq = 0.0f64;
    let mut peak = 0.0f64;
    let mut count = 0u64;
    for index in 0..samples {
        let offset = index * 4;
        let l = f32::from_le_bytes([
            lumen[offset],
            lumen[offset + 1],
            lumen[offset + 2],
            lumen[offset + 3],
        ]);
        let r = f32::from_le_bytes([
            vlc[offset],
            vlc[offset + 1],
            vlc[offset + 2],
            vlc[offset + 3],
        ]);
        let delta = f64::from(l) - f64::from(r);
        sum_sq += delta * delta;
        peak = peak.max(delta.abs());
        count += 1;
    }
    let rms = if count > 0 {
        (sum_sq / count as f64).sqrt()
    } else {
        0.0
    };
    let deviation = Deviation {
        samples: count,
        rms,
        peak,
        rms_max,
        peak_max,
    };
    let verdict = if rms <= rms_max && peak <= peak_max {
        OracleVerdict::TolerancePass
    } else {
        OracleVerdict::ToleranceFail
    };
    (verdict, deviation)
}

/// Compare one row LIVE (Lumen payload + the VLC dump's bytes, both in
/// memory): integer PCM via the MD5-prefix method, f32 via the
/// ratified tolerance path computed fresh.
pub fn compare_row_live(
    rel: &str,
    audiofile_format: &str,
    dump: &DecodeDump,
    vlc_bytes: &[u8],
    vlc_md5: &str,
    tolerance: &crate::config::F32Tolerance,
) -> OracleRow {
    let lumen_bytes = dump.pcm.len() as u64;
    let (verdict, deviation) = if audiofile_format == "float32" {
        let (verdict, deviation) =
            f32_verdict(&dump.pcm, vlc_bytes, tolerance.rms_max, tolerance.peak_max);
        (verdict, Some(deviation))
    } else {
        (
            prefix_verdict(&dump.pcm, vlc_md5, vlc_bytes.len() as u64),
            None,
        )
    };
    OracleRow {
        rel: rel.to_string(),
        audiofile_format: audiofile_format.to_string(),
        lumen_bytes,
        lumen_md5: dump.md5_hex.clone(),
        lumen_sample: dump.sample.to_string(),
        vlc_bytes: vlc_bytes.len() as u64,
        vlc_md5: vlc_md5.to_string(),
        vlc_source: VlcSource::LiveRun,
        verdict,
        deviation,
    }
}

/// Compare one row against the COMMITTED 2026-11-09 record (no live
/// VLC): integer PCM via the MD5-prefix method over the record's
/// (vlc_md5, vlc_bytes) pair — the prefix truncation needs only the
/// length, not the bytes; f32 via the record's own measured RMS/peak
/// (the record documents they were computed from the actual PCM on
/// both sides), echoed against the config's thresholds. Applicable
/// only when the fresh realization demonstrably equals the record's
/// (the caller checked `lumen_fresh_md5`).
pub fn compare_row_committed(
    rel: &str,
    audiofile_format: &str,
    dump: &DecodeDump,
    parts: &CommittedParts,
    tolerance: &crate::config::F32Tolerance,
) -> OracleRow {
    let (verdict, deviation) = if audiofile_format == "float32" {
        match (parts.rms_dev, parts.peak_dev) {
            (Some(rms), Some(peak)) => {
                let dev = Deviation {
                    samples: parts.samples_compared.unwrap_or(0),
                    rms,
                    peak,
                    rms_max: tolerance.rms_max,
                    peak_max: tolerance.peak_max,
                };
                let verdict = if rms <= tolerance.rms_max && peak <= tolerance.peak_max {
                    OracleVerdict::TolerancePass
                } else {
                    OracleVerdict::ToleranceFail
                };
                (verdict, Some(dev))
            }
            _ => (OracleVerdict::NoComparison, None),
        }
    } else {
        (
            prefix_verdict(&dump.pcm, &parts.vlc_md5, parts.vlc_bytes),
            None,
        )
    };
    OracleRow {
        rel: rel.to_string(),
        audiofile_format: audiofile_format.to_string(),
        lumen_bytes: dump.pcm.len() as u64,
        lumen_md5: dump.md5_hex.clone(),
        lumen_sample: dump.sample.to_string(),
        vlc_bytes: parts.vlc_bytes,
        vlc_md5: parts.vlc_md5.clone(),
        vlc_source: VlcSource::CommittedRecord,
        verdict,
        deviation,
    }
}

/// Load the committed oracle rows (rel → row).
pub fn committed_rows(config: &Config) -> Result<BTreeMapOrHashMap, ToolError> {
    let path = config.rel(&config.oracle.record);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| ToolError::io(format!("reading {}", path.display()), e))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| ToolError::config(format!("parsing {}: {e}", path.display())))?;
    let mut map = BTreeMap::new();
    let Some(rows) = value.get("rows").and_then(|rows| rows.as_array()) else {
        return Err(ToolError::config(format!(
            "{}: no rows array",
            path.display()
        )));
    };
    for row in rows {
        let parsed: CommittedRow = serde_json::from_value(row.clone())
            .map_err(|e| ToolError::config(format!("parsing a row of {}: {e}", path.display())))?;
        map.insert(parsed.rel.clone(), parsed);
    }
    Ok(map)
}

/// The public alias for the committed rows map.
pub type BTreeMapOrHashMap = BTreeMap<String, CommittedRow>;

/// The committed record's comparable fields, when the row exists.
#[derive(Debug, Clone)]
pub struct CommittedParts {
    /// VLC's dump MD5 (the 2026-11-09 realization's bytes).
    pub vlc_md5: String,
    /// VLC's dump bytes.
    pub vlc_bytes: u64,
    /// Lumen's decode MD5 of the same realization (cross-check:
    /// equal iff the fresh bytes match the recorded ones).
    pub lumen_fresh_md5: Option<String>,
    /// f32 rows: the recorded RMS deviation of that realization.
    pub rms_dev: Option<f64>,
    /// f32 rows: the recorded peak deviation.
    pub peak_dev: Option<f64>,
    /// f32 rows: the recorded compared-sample count.
    pub samples_compared: Option<u64>,
}

/// Whether the committed record's values apply to a fresh decode:
/// Lumen's decode is deterministic, so the record's
/// `lumen_fresh_md5` equals the fresh bytes' MD5 iff this is the
/// same realization the record captured (a fresh random realization
/// or encoder drift makes the record's VLC side inapplicable — the
/// caller must then record `NoComparison`, never apply the record).
pub fn committed_record_applies(parts: &CommittedParts, fresh_md5: &str) -> bool {
    parts
        .lumen_fresh_md5
        .as_deref()
        .is_some_and(|md5| md5 == fresh_md5)
}

/// Expose the committed row's fields (the private type's read API).
pub fn committed_parts(map: &BTreeMapOrHashMap, rel: &str) -> Option<CommittedParts> {
    map.get(rel).map(|row| CommittedParts {
        vlc_md5: row.vlc_md5.clone(),
        vlc_bytes: row.vlc_bytes,
        lumen_fresh_md5: row.lumen_fresh_md5.clone(),
        rms_dev: row.rms_dev,
        peak_dev: row.peak_dev,
        samples_compared: row.samples_compared,
    })
}

/// The `compare-pcm` CLI (one-off, both files on disk).
pub fn compare_pcm_cli(args: &[String]) -> Result<std::process::ExitCode, ToolError> {
    let mut paths: Vec<String> = Vec::new();
    let mut sample = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--sample" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err(ToolError::usage("--sample requires {u8|s16|f32}"));
                };
                sample = value.clone();
            }
            other => paths.push(other.to_string()),
        }
        i += 1;
    }
    if paths.len() != 2 {
        return Err(ToolError::usage("compare-pcm takes exactly two files"));
    }
    let lumen = std::fs::read(Path::new(&paths[0]))
        .map_err(|e| ToolError::io(format!("reading {}", paths[0]), e))?;
    let oracle = std::fs::read(Path::new(&paths[1]))
        .map_err(|e| ToolError::io(format!("reading {}", paths[1]), e))?;
    let mut md5 = bench_vs_vlc::md5::Md5::new();
    md5.update(&oracle);
    let oracle_md5 = md5.finalize_hex();
    match sample.as_str() {
        "f32" => {
            // The ratified defaults are PRINTED, not decided, here.
            let (verdict, deviation) = f32_verdict(&lumen, &oracle, 1e-4, 1e-3);
            println!(
                "lumen {} B, oracle {} B (md5 {}): {:?} rms={} peak={} (policy: rms<=1e-4 \
                 AND peak<=1e-3 — from docs/corpus/vlc-f32-tolerance-20261109.json)",
                lumen.len(),
                oracle.len(),
                oracle_md5,
                verdict,
                deviation.rms,
                deviation.peak
            );
        }
        "u8" | "s16" => {
            let verdict = prefix_verdict(&lumen, &oracle_md5, oracle.len() as u64);
            println!(
                "lumen {} B, oracle {} B (md5 {}): {:?}",
                lumen.len(),
                oracle.len(),
                oracle_md5,
                verdict
            );
        }
        other => {
            return Err(ToolError::usage(format!(
                "unsupported --sample {other:?} (u8|s16|f32)"
            )));
        }
    }
    Ok(std::process::ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(pcm: &[u8]) -> String {
        let mut md5 = bench_vs_vlc::md5::Md5::new();
        md5.update(pcm);
        md5.finalize_hex()
    }

    fn dump(pcm: Vec<u8>) -> DecodeDump {
        DecodeDump {
            md5_hex: hex(&pcm),
            pcm,
            sample: "s16",
        }
    }

    fn f32_bytes(samples: &[f32]) -> Vec<u8> {
        samples.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    /// P3-2: the MD5-prefix method — a matching prefix with an
    /// unflushed tail is bit-exact; the tail is exactly the byte
    /// shortfall.
    #[test]
    fn prefix_verdict_bit_exact_with_tail() {
        let lumen: Vec<u8> = (0..100u8).collect();
        let vlc_bytes = 60u64;
        let verdict = prefix_verdict(&lumen, &hex(&lumen[..60]), vlc_bytes);
        assert_eq!(
            verdict,
            OracleVerdict::BitExactPrefix {
                unflushed_tail_bytes: 40
            }
        );
    }

    /// P3-2: equal lengths with matching md5 → zero tail.
    #[test]
    fn prefix_verdict_whole_payload() {
        let lumen: Vec<u8> = (0..64u8).collect();
        let verdict = prefix_verdict(&lumen, &hex(&lumen), lumen.len() as u64);
        assert_eq!(
            verdict,
            OracleVerdict::BitExactPrefix {
                unflushed_tail_bytes: 0
            }
        );
    }

    /// P3-2: a byte that disagrees inside the VLC window →
    /// NotBitExact, never a partial pass.
    #[test]
    fn prefix_verdict_rejects_mismatched_bytes() {
        let lumen: Vec<u8> = (0..100u8).collect();
        let mut corrupted = lumen.clone();
        corrupted[10] = corrupted[10].wrapping_add(1);
        assert_eq!(
            prefix_verdict(&lumen, &hex(&corrupted[..60]), 60),
            OracleVerdict::NotBitExact
        );
    }

    /// P3-2: an oracle window LONGER than the decode → NotBitExact
    /// (the prefix method cannot compare bytes Lumen never produced).
    #[test]
    fn prefix_verdict_rejects_oracle_longer_than_decode() {
        let lumen: Vec<u8> = (0..100u8).collect();
        assert_eq!(
            prefix_verdict(&lumen, &hex(&lumen), 101),
            OracleVerdict::NotBitExact
        );
    }

    /// P3-2: the f32 tolerance path — identical windows pass with
    /// zero deviation; the head-aligned window is min(len) samples.
    #[test]
    fn f32_verdict_identical_passes() {
        let lumen = f32_bytes(&[1.0, 0.5, -0.25, 0.0]);
        let vlc = f32_bytes(&[1.0, 0.5, -0.25]);
        let (verdict, dev) = f32_verdict(&lumen, &vlc, 1e-4, 1e-3);
        assert_eq!(verdict, OracleVerdict::TolerancePass);
        assert_eq!(dev.samples, 3); // min(4, 3) samples
        assert_eq!(dev.rms, 0.0);
        assert_eq!(dev.peak, 0.0);
    }

    /// P3-2: a deviation inside policy passes; the arithmetic is the
    /// plain RMS/peak over the compared samples. The offset is 2⁻¹⁷ —
    /// exactly representable in f32 — so every intermediate is exact:
    /// peak 2⁻¹⁷, RMS 2⁻¹⁸ over the four samples (a decimal offset
    /// like 1e-5 would round in f32 and break exact assertions).
    #[test]
    fn f32_verdict_small_deviation_passes() {
        let offset = 2.0f32.powi(-17); // a power of two — exact in f32
        let lumen = f32_bytes(&[1.0, 1.0, 1.0, 1.0]);
        let vlc = f32_bytes(&[1.0, 1.0, 1.0, 1.0 + offset]);
        let (verdict, dev) = f32_verdict(&lumen, &vlc, 1e-4, 1e-3);
        assert_eq!(verdict, OracleVerdict::TolerancePass);
        assert_eq!(dev.samples, 4);
        assert_eq!(dev.rms, f64::from(offset) / 2.0, "rms {}", dev.rms);
        assert_eq!(dev.peak, f64::from(offset), "peak {}", dev.peak);
    }

    /// P3-2: a deviation beyond policy FAILS (the genuine parity-gap
    /// class the 2026-11-10 run recorded on all six f32 rows). The
    /// offset 2⁻⁷ is exact in f32: peak 2⁻⁷, far beyond the 1e-3
    /// policy; RMS 2⁻⁸, beyond the 1e-4 policy.
    #[test]
    fn f32_verdict_out_of_policy_fails() {
        let offset = 2.0f32.powi(-7); // a power of two — exact in f32
        let lumen = f32_bytes(&[1.0, 1.0, 1.0, 1.0]);
        let vlc = f32_bytes(&[1.0, 1.0, 1.0, 1.0 + offset]);
        let (verdict, dev) = f32_verdict(&lumen, &vlc, 1e-4, 1e-3);
        assert_eq!(verdict, OracleVerdict::ToleranceFail);
        assert_eq!(dev.rms, f64::from(offset) / 2.0, "rms {}", dev.rms);
        assert_eq!(dev.peak, f64::from(offset), "peak {}", dev.peak);
    }

    /// P3-2: the committed-record applicability rule — the record's
    /// values apply only when its `lumen_fresh_md5` equals the fresh
    /// decode's (same realization), and only when it HAS one.
    #[test]
    fn committed_record_applicability() {
        let mut parts = CommittedParts {
            vlc_md5: "0".repeat(32),
            vlc_bytes: 10,
            lumen_fresh_md5: Some("abc".to_string()),
            rms_dev: None,
            peak_dev: None,
            samples_compared: None,
        };
        assert!(committed_record_applies(&parts, "abc"));
        assert!(!committed_record_applies(&parts, "abd"));
        parts.lumen_fresh_md5 = None;
        assert!(!committed_record_applies(&parts, "abc"));
    }

    /// P3-2: the committed-record comparison for an integer row —
    /// the prefix method over the record's (vlc_md5, vlc_bytes).
    #[test]
    fn compare_row_committed_integer_prefix() {
        let pcm: Vec<u8> = (0..200u8).collect();
        let d = dump(pcm.clone());
        let parts = CommittedParts {
            vlc_md5: hex(&pcm[..128]),
            vlc_bytes: 128,
            lumen_fresh_md5: Some(d.md5_hex.clone()),
            rms_dev: None,
            peak_dev: None,
            samples_compared: None,
        };
        let row = compare_row_committed("r.avi", "u8", &d, &parts, &tolerance());
        assert_eq!(row.vlc_source, VlcSource::CommittedRecord);
        assert_eq!(
            row.verdict,
            OracleVerdict::BitExactPrefix {
                unflushed_tail_bytes: 72
            }
        );
        // A record whose realization does not match must not be
        // applied by the caller (the applicability rule above).
        let wrong = CommittedParts {
            vlc_md5: hex(&pcm[..128]),
            vlc_bytes: 128,
            lumen_fresh_md5: Some("different".to_string()),
            ..parts.clone()
        };
        assert!(!committed_record_applies(&wrong, &d.md5_hex));
    }

    /// P3-2: the committed-record comparison for an f32 row — the
    /// record's own measured RMS/peak echoed against the configured
    /// thresholds; a record without them claims nothing.
    #[test]
    fn compare_row_committed_f32_echoes_the_record() {
        let d = dump(f32_bytes(&[0.5, 0.25]));
        let within = CommittedParts {
            vlc_md5: "0".repeat(32),
            vlc_bytes: 8,
            lumen_fresh_md5: Some(d.md5_hex.clone()),
            rms_dev: Some(1e-5),
            peak_dev: Some(2e-5),
            samples_compared: Some(2),
        };
        let row = compare_row_committed("r.avi", "float32", &d, &within, &tolerance());
        assert_eq!(row.verdict, OracleVerdict::TolerancePass);
        match row.deviation {
            Some(dev) => {
                assert_eq!(dev.samples, 2);
                assert!((dev.rms - 1e-5).abs() < 1e-12, "rms {}", dev.rms);
            }
            None => unreachable!("the echoed record carries its deviation"),
        }

        let beyond = CommittedParts {
            rms_dev: Some(1.0),
            peak_dev: Some(2.0),
            ..within.clone()
        };
        let row = compare_row_committed("r.avi", "float32", &d, &beyond, &tolerance());
        assert_eq!(row.verdict, OracleVerdict::ToleranceFail);

        let incomplete = CommittedParts {
            rms_dev: None,
            peak_dev: None,
            ..within
        };
        let row = compare_row_committed("r.avi", "float32", &d, &incomplete, &tolerance());
        assert_eq!(row.verdict, OracleVerdict::NoComparison);
    }

    fn tolerance() -> crate::config::F32Tolerance {
        crate::config::F32Tolerance {
            rms_max: 1e-4,
            peak_max: 1e-3,
        }
    }
}
