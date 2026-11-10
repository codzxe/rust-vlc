//! The full run: orchestration, matrix rendering, and the
//! transitions outbox (issue #53, cards 8.B1–8.B2).
//!
//! The flow is doc 10 §7.3's shape, honestly scoped to what THIS
//! host can run:
//!
//! 1. Assemble the corpus (committed fixture roots + the
//!    manifest-anchored regenerable set).
//! 2. Assert the live VLC oracle when configured (version-pinned;
//!    a mismatched oracle is refused, never silently used).
//! 3. Decode every corpus file through the real open/decode path
//!    ([`crate::decode`]), appending each row to the raw-rows JSONL
//!    as it happens (partial evidence survives a mid-run kill).
//! 4. Compare the lossless-comparable family against the oracle
//!    (live dump preferred; the committed 2026-11-09 record is used
//!    only where the fresh bytes demonstrably match its realization,
//!    always labeled `committed-record`).
//! 5. Execute the configured lane steps for real (the web/wasm lane)
//!    and record their outcomes.
//! 6. Aggregate the per-format × per-lane matrix, detect the gaps
//!    (claims the run contradicts), and render:
//!    the human markdown, the machine JSON, and the transitions
//!    outbox that `cargo xtask compat-matrix apply-transitions`
//!    (the parity tool's rule-enforcing writer) consumes.
//!
//! THIS TOOL NEVER EDITS `docs/feature-matrix.toml`. Windows/macOS
//! cells are `blocked` lane cells — reason + next step cited
//! verbatim, never inferred (the 3-OS runner blocker is user-owned).

use std::collections::BTreeMap;
use std::io::Write as _;

use serde::Serialize;

use crate::ToolError;
use crate::config::{Config, LaneKind, MatrixClaim, Step};
use crate::corpus::{self, CorpusClass, CorpusFile, RegenOutcome};
use crate::decode::{self, FileOutcome, FileRecord, TrackOutcome};
use crate::oracle::{self, OracleRow, OracleVerdict, VlcSource};
use crate::proc;

/// One lane's recorded evidence.
#[derive(Debug, Serialize)]
struct LaneRecord {
    /// The lane id (`linux-native`, `web-wasm`, ...).
    id: String,
    /// The human label.
    label: String,
    /// The lane kind tag (`native-decode` / `steps` / `blocked`).
    kind: String,
    /// Executed steps with their outcomes (steps lanes).
    steps: Vec<StepRecord>,
    /// The blocked-lane citation (blocked lanes).
    blocked: Option<BlockedCitation>,
}

/// One executed lane step.
#[derive(Debug, Serialize)]
struct StepRecord {
    /// What this step proves.
    purpose: String,
    /// The argv.
    argv: Vec<String>,
    /// Exit status / termination.
    outcome: String,
    /// Wall seconds.
    wall_s: f64,
    /// Recorded artifacts (path + size).
    artifacts: Vec<ArtifactRecord>,
}

/// One recorded artifact.
#[derive(Debug, Serialize)]
struct ArtifactRecord {
    /// Workspace-relative path.
    path: String,
    /// Size in bytes.
    bytes: u64,
}

/// The blocked-lane citation, verbatim from the config.
#[derive(Debug, Serialize)]
struct BlockedCitation {
    reason: String,
    reason_ref: String,
    next_step: String,
    tracking: String,
}

/// One matrix cell.
#[derive(Debug, Clone, Serialize)]
struct Cell {
    /// The lane id.
    lane: String,
    /// The cell status vocabulary (see the report legend).
    status: String,
    /// One-line detail (counts, refusal messages, citations).
    detail: String,
}

/// One rendered format row of the matrix.
#[derive(Debug, Serialize)]
struct FormatRowOut {
    /// The probe's format id.
    probe_format: String,
    /// VLC's name for the family.
    vlc_name: String,
    /// The feature-matrix row ids this row maps to.
    matrix_rows: Vec<String>,
    /// The claimed statuses (the config's claim snapshot).
    claims: Vec<ClaimOut>,
    /// The corpus files behind this row (valid classes).
    files: Vec<String>,
    /// Malformed/error-contract files of this format.
    malformed_files: usize,
    /// The cells, one per configured lane.
    cells: Vec<Cell>,
}

/// One rendered codec row of the matrix.
#[derive(Debug, Serialize)]
struct CodecRowOut {
    /// The codec family descriptor.
    codec: String,
    /// The limb kind this family decodes on (`audio`/`video`).
    kind: String,
    /// The feature-matrix row ids this row maps to.
    matrix_rows: Vec<String>,
    /// The claimed statuses.
    claims: Vec<ClaimOut>,
    /// The corpus files with tracks of this family.
    files: Vec<String>,
    /// The cells, one per configured lane.
    cells: Vec<Cell>,
}

/// One claim snapshot entry (rendered): ALL FOUR platform columns
/// are recorded for honesty — the gap rules act on the lane that ran
/// (linux), the report shows what the matrix claims everywhere.
#[derive(Debug, Clone, Serialize)]
struct ClaimOut {
    /// The feature-matrix row id.
    row: String,
    /// The claimed windows status.
    windows: String,
    /// The claimed linux status (the lane that ran).
    linux: String,
    /// The claimed macos status.
    macos: String,
    /// The claimed web status.
    web: String,
}

/// One proposed status transition (the outbox shape).
#[derive(Debug, Clone, Serialize)]
struct TransitionOut {
    /// The feature-matrix row id.
    row: String,
    /// The platform column (`linux` — the lane that ran).
    platform: String,
    /// The current claimed status (the applier verifies it).
    from: String,
    /// The proposed status.
    to: String,
    /// Why (cited from the run's evidence).
    reason: String,
    /// The concrete next step (downgrades only).
    next_step: String,
    /// The evidence artifact for the transition.
    evidence: String,
}

/// The whole machine report (schema `compat-matrix/report/1`).
#[derive(Debug, Serialize)]
struct RunReport {
    schema: String,
    generated: String,
    host: HostInfo,
    git_commit: Option<String>,
    corpus: CorpusSummary,
    lanes: Vec<LaneRecord>,
    oracle: OracleSummary,
    formats: Vec<FormatRowOut>,
    codecs: Vec<CodecRowOut>,
    counts: Counts,
    transitions: Vec<TransitionOut>,
    notes: Vec<String>,
}

/// The host this run executed on.
#[derive(Debug, Serialize)]
struct HostInfo {
    os: &'static str,
    arch: &'static str,
}

/// Corpus assembly summary.
#[derive(Debug, Serialize)]
struct CorpusSummary {
    committed_files: usize,
    malformed_files: usize,
    regen_files: usize,
    /// The regen manifest's provenance prose (cited verbatim).
    regen_provenance: String,
    regen_outcomes: Vec<(String, RegenOutcome)>,
}

/// The oracle-side summary.
#[derive(Debug, Serialize)]
struct OracleSummary {
    /// The asserted version line (live oracle), else the record citation.
    source: String,
    /// The ratified f32 tolerance decision record (cited).
    f32_record: String,
    /// The comparison rows.
    rows: Vec<OracleRow>,
    /// Cross-checks against the committed 2026-11-09 record.
    committed_cross_checks: Vec<String>,
}

/// The deliverable counts.
#[derive(Debug, Default, Serialize)]
struct Counts {
    files_total: usize,
    decoded: usize,
    decoded_partial: usize,
    refused_probe: usize,
    refused_demux: usize,
    refused_input: usize,
    demux_fatal: usize,
    malformed_typed: usize,
    oracle_bit_exact: usize,
    oracle_tolerance_pass: usize,
    oracle_tolerance_fail: usize,
    oracle_not_bit_exact: usize,
    oracle_no_comparison: usize,
    oracle_other: usize,
}

/// The whole run. See the module docs.
pub fn full_run(config: &Config) -> Result<(), ToolError> {
    let started = std::time::Instant::now();
    let generated = iso8601_now();

    // 0. Host + commit provenance.
    let git_commit = git_commit();

    // 1. Corpus assembly.
    let committed = corpus::collect_committed(config)?;
    let committed_count = committed
        .iter()
        .filter(|file| matches!(file.class, CorpusClass::Committed))
        .count();
    let malformed_count = committed
        .iter()
        .filter(|file| matches!(file.class, CorpusClass::Malformed))
        .count();
    let (regen_files, regen_outcomes) = corpus::ensure_regen_corpus(config)?;
    eprintln!(
        "compat-matrix: corpus assembled: {} committed + {} malformed + {} regen ({} regen failures)",
        committed_count,
        malformed_count,
        regen_files.len(),
        regen_outcomes
            .iter()
            .filter(|(_, o)| matches!(o, RegenOutcome::Failed { .. }))
            .count()
    );

    // 2. The live oracle (version-asserted before any use; a missing
    //    binary on this host is a recorded absence — committed-record
    //    rows, labeled — never an inference).
    let oracle_state = oracle::LiveOracle::assert_version(config)?;
    let live = match &oracle_state {
        oracle::OracleState::Live(oracle) => Some(oracle),
        oracle::OracleState::Absent(_) => None,
    };
    let committed_oracle = oracle::committed_rows(config)?;

    // 3. The decode sweep, raw rows appended incrementally.
    let raw_rows_path = config.rel(&config.reports.raw_rows);
    if let Some(parent) = raw_rows_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ToolError::io(format!("creating {}", parent.display()), e))?;
    }
    let mut raw_rows = std::fs::File::create(&raw_rows_path)
        .map_err(|e| ToolError::io(format!("creating {}", raw_rows_path.display()), e))?;

    let mut records: Vec<(CorpusFile, FileRecord)> = Vec::new();
    let mut dumps: BTreeMap<String, decode::DecodeDump> = BTreeMap::new();
    let all_files: Vec<CorpusFile> = committed.into_iter().chain(regen_files).collect();
    let mut counts = Counts {
        files_total: all_files.len(),
        ..Counts::default()
    };
    for file in &all_files {
        let (record, dump) =
            decode::decode_file(&file.rel, &file.path, file.dump_pcm).map_err(|message| {
                ToolError::Subprocess(format!(
                    "decode of {} failed at tool level: {message}",
                    file.rel
                ))
            })?;
        count_file(
            &record,
            matches!(file.class, CorpusClass::Malformed),
            &mut counts,
        );
        if let Some(dump) = dump {
            dumps.insert(file.rel.clone(), dump);
        }
        let mut line = serde_json::to_string(&record)
            .map_err(|e| ToolError::Subprocess(format!("serializing row {}: {e}", file.rel)))?;
        line.push('\n');
        raw_rows
            .write_all(line.as_bytes())
            .map_err(|e| ToolError::io(format!("writing {}", raw_rows_path.display()), e))?;
        records.push((file.clone(), record));
    }
    drop(raw_rows);
    eprintln!(
        "compat-matrix: decode sweep done: {} files ({} decoded, {} partial, {} refused, {} demux-fatal)",
        counts.files_total,
        counts.decoded,
        counts.decoded_partial,
        counts.refused_probe + counts.refused_demux + counts.refused_input,
        counts.demux_fatal
    );

    // 4. The oracle comparisons (the lossless-comparable family).
    let mut oracle_rows: Vec<OracleRow> = Vec::new();
    let mut cross_checks: Vec<String> = Vec::new();
    let dump_dir = config.rel(&config.corpus.regen.cache_dir).join("vlc-dumps");
    for file in &all_files {
        let Some(entry_format) = audiofile_format_for(config, file) else {
            continue;
        };
        let Some(dump) = dumps.get(&file.rel) else {
            // Lumen could not decode the row — record the refusal
            // honestly, no oracle claim.
            let record = records
                .iter()
                .find(|(f, _)| f.rel == file.rel)
                .map(|(_, r)| r);
            let reason = match record {
                Some(record) => outcome_message(&record.outcome),
                None => "no decoded payload".to_string(),
            };
            oracle_rows.push(OracleRow {
                rel: file.rel.clone(),
                audiofile_format: entry_format,
                lumen_bytes: 0,
                lumen_md5: String::new(),
                lumen_sample: String::new(),
                vlc_bytes: 0,
                vlc_md5: String::new(),
                // No VLC side was consulted — the decode never
                // happened, so no source is cited (P3-5).
                vlc_source: VlcSource::None,
                verdict: OracleVerdict::LumenRefused { reason },
                deviation: None,
            });
            counts.oracle_other += 1;
            continue;
        };
        let record_key = oracle_rel_for(config, file);
        let committed_parts = record_key
            .as_deref()
            .and_then(|key| oracle::committed_parts(&committed_oracle, key));
        // The committed record's values apply only when the fresh
        // bytes demonstrably equal its realization: Lumen's decode is
        // deterministic, so equal MD5 ⇒ equal input bytes.
        let committed_applicable = committed_parts
            .as_ref()
            .is_some_and(|parts| oracle::committed_record_applies(parts, &dump.md5_hex));
        if committed_parts.is_some() {
            if committed_applicable {
                cross_checks.push(format!(
                    "{}: fresh decode md5 == committed 2026-11-09 lumen_fresh_md5 (same \
                     realization) — the record's vlc_md5 is applicable",
                    file.rel
                ));
            } else {
                cross_checks.push(format!(
                    "{}: fresh realization (random source or encoder drift) — the committed \
                     record's md5s do not apply to these bytes",
                    file.rel
                ));
            }
        }
        let row = match &live {
            Some(live) => {
                std::fs::create_dir_all(&dump_dir)
                    .map_err(|e| ToolError::io(format!("creating {}", dump_dir.display()), e))?;
                let out = dump_dir.join(safe_name(&file.rel)).with_extension("pcm");
                match live.dump(&file.path, &entry_format, &out, config.budgets.vlc_s) {
                    Ok(oracle::VlcDump::Ok { bytes, md5 }) => {
                        let _ = std::fs::remove_file(&out);
                        oracle::compare_row_live(
                            &file.rel,
                            &entry_format,
                            dump,
                            &bytes,
                            &md5,
                            &config.oracle.f32_tolerance,
                        )
                    }
                    Ok(oracle::VlcDump::Failed { reason }) => {
                        oracle_rows.push(OracleRow {
                            rel: file.rel.clone(),
                            audiofile_format: entry_format,
                            lumen_bytes: dump.pcm.len() as u64,
                            lumen_md5: dump.md5_hex.clone(),
                            lumen_sample: dump.sample.to_string(),
                            vlc_bytes: 0,
                            vlc_md5: String::new(),
                            vlc_source: VlcSource::LiveRun,
                            verdict: OracleVerdict::OracleFailed { reason },
                            deviation: None,
                        });
                        counts.oracle_other += 1;
                        continue;
                    }
                    Err(err) => return Err(err),
                }
            }
            None => match committed_applicable {
                true => {
                    let parts = committed_parts.as_ref().ok_or_else(|| {
                        ToolError::Subprocess("committed parts vanished".to_string())
                    })?;
                    oracle::compare_row_committed(
                        &file.rel,
                        &entry_format,
                        dump,
                        parts,
                        &config.oracle.f32_tolerance,
                    )
                }
                false => {
                    oracle_rows.push(OracleRow {
                        rel: file.rel.clone(),
                        audiofile_format: entry_format,
                        lumen_bytes: dump.pcm.len() as u64,
                        lumen_md5: dump.md5_hex.clone(),
                        lumen_sample: dump.sample.to_string(),
                        vlc_bytes: 0,
                        vlc_md5: String::new(),
                        // The record was consulted for the cross-check
                        // but its values were NOT applied (fresh
                        // realization, or no entry) — no source is
                        // cited (P3-5).
                        vlc_source: VlcSource::None,
                        verdict: OracleVerdict::NoComparison,
                        deviation: None,
                    });
                    counts.oracle_no_comparison += 1;
                    continue;
                }
            },
        };
        match &row.verdict {
            OracleVerdict::BitExactPrefix { .. } => counts.oracle_bit_exact += 1,
            OracleVerdict::TolerancePass => counts.oracle_tolerance_pass += 1,
            OracleVerdict::ToleranceFail => counts.oracle_tolerance_fail += 1,
            OracleVerdict::NotBitExact => counts.oracle_not_bit_exact += 1,
            OracleVerdict::NoComparison => counts.oracle_no_comparison += 1,
            _ => counts.oracle_other += 1,
        }
        oracle_rows.push(row);
    }
    let oracle_source = match &oracle_state {
        oracle::OracleState::Live(_) => format!(
            "live run: the pinned {} (version-asserted before use; invocation record in \
             the config + docs/corpus/vlc-diff-report-20261023.md §Oracle run 2026-11-09)",
            config
                .oracle
                .vlc
                .as_ref()
                .map(|v| v.version_line.clone())
                .unwrap_or_default()
        ),
        oracle::OracleState::Absent(reason) => {
            format!("committed record only ({reason}): docs/corpus/vlc-oracle-run-20261109.json")
        }
    };
    eprintln!(
        "compat-matrix: oracle comparisons: {} bit-exact, {} tolerance-pass, {} tolerance-fail, {} not-bit-exact, {} no-comparison",
        counts.oracle_bit_exact,
        counts.oracle_tolerance_pass,
        counts.oracle_tolerance_fail,
        counts.oracle_not_bit_exact,
        counts.oracle_no_comparison
    );

    // 5. The lanes.
    let mut lanes: Vec<LaneRecord> = Vec::new();
    for lane in &config.lanes {
        let record = match &lane.kind {
            LaneKind::NativeDecode { .. } => LaneRecord {
                id: lane.id.clone(),
                label: lane.label.clone(),
                kind: "native-decode".to_string(),
                steps: Vec::new(),
                blocked: None,
            },
            LaneKind::Steps { steps } => {
                let mut step_records = Vec::new();
                for step in steps {
                    let record = run_step(config, step)?;
                    step_records.push(record);
                }
                LaneRecord {
                    id: lane.id.clone(),
                    label: lane.label.clone(),
                    kind: "steps".to_string(),
                    steps: step_records,
                    blocked: None,
                }
            }
            LaneKind::Blocked {
                reason,
                reason_ref,
                next_step,
                tracking,
            } => LaneRecord {
                id: lane.id.clone(),
                label: lane.label.clone(),
                kind: "blocked".to_string(),
                steps: Vec::new(),
                blocked: Some(BlockedCitation {
                    reason: reason.clone(),
                    reason_ref: reason_ref.clone(),
                    next_step: next_step.clone(),
                    tracking: tracking.clone(),
                }),
            },
        };
        lanes.push(record);
    }

    // 6. The matrix + the gaps.
    let claims: BTreeMap<&str, &MatrixClaim> = config
        .claims
        .iter()
        .map(|claim| (claim.row.as_str(), claim))
        .collect();
    let native_lane = config
        .lanes
        .iter()
        .find(|lane| matches!(lane.kind, LaneKind::NativeDecode { .. }))
        .map(|lane| lane.id.clone())
        .unwrap_or_else(|| "linux-native".to_string());
    // The feature-matrix column this host's sweep is evidence for
    // (config/2: the native-decode lane's REQUIRED `platform` — the
    // gap rules judge this column and the outbox's transitions carry
    // it; never inferred from the host OS).
    let native_platform = config
        .lanes
        .iter()
        .find_map(|lane| match &lane.kind {
            LaneKind::NativeDecode { platform } => Some(platform.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "linux".to_string());
    // The run's own matrix table — the evidence string the gap-rule
    // transitions cite (the run's rendered report, wherever the
    // config points it; for the 2026-11-10 real run this is exactly
    // docs/corpus/compat-matrix-20261110.md).
    let run_evidence = format!(
        "{} (this run's matrix table + raw rows)",
        config.reports.markdown
    );
    let native = NativeCtx {
        lane: &native_lane,
        platform: &native_platform,
        evidence: &run_evidence,
    };
    let mut transitions: Vec<TransitionOut> = Vec::new();

    let mut format_rows: Vec<FormatRowOut> = Vec::new();
    for format in &config.formats {
        // Files the open path named as this family (probe id match),
        // plus files that never got that far: a valid-class corpus
        // file of one of this row's extensions that the probe REFUSED
        // (no signature) is still this row's honest evidence — a
        // claimed-supported family whose files cannot even probe is
        // the sharpest gap there is (e.g. asf/wmv today).
        let valid: Vec<&(CorpusFile, FileRecord)> = records
            .iter()
            .filter(|(file, record)| {
                !matches!(file.class, CorpusClass::Malformed)
                    && record.probe_format.as_deref() == Some(format.probe_format.as_str())
            })
            .collect();
        let refused: Vec<&(CorpusFile, FileRecord)> = records
            .iter()
            .filter(|(file, record)| {
                !matches!(file.class, CorpusClass::Malformed)
                    && record.probe_format.is_none()
                    && matches!(
                        record.outcome,
                        FileOutcome::RefusedProbe { .. }
                            | FileOutcome::RefusedDemux { .. }
                            | FileOutcome::RefusedInput { .. }
                    )
                    && format.extensions.iter().any(|ext| {
                        file.path
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
                    })
            })
            .collect();
        let malformed = records
            .iter()
            .filter(|(file, record)| {
                matches!(file.class, CorpusClass::Malformed)
                    && record.probe_format.as_deref() == Some(format.probe_format.as_str())
            })
            .count();
        let mut cells: Vec<Cell> = Vec::new();
        for lane in &config.lanes {
            let steps = lane_steps(&lanes, &lane.id);
            cells.push(format_cell(lane, steps, &valid, &refused, malformed));
        }
        let verdict = if valid.is_empty() && !refused.is_empty() {
            "refused"
        } else {
            container_verdict(&valid)
        };
        // Gap detection: the container rows are judged by the open
        // path (probe + demux walk), not by codec decode — a
        // NoDecoder track is the CODEC row's gap, not the container's.
        let evidence_rows = FormatEvidence {
            valid: &valid,
            refused: &refused,
            probe_format: &format.probe_format,
        };
        for row_id in &format.matrix_rows {
            let Some(claim) = claims.get(row_id.as_str()) else {
                continue;
            };
            container_gap(
                row_id,
                claim,
                verdict,
                &evidence_rows,
                &native,
                &mut transitions,
            );
        }
        format_rows.push(FormatRowOut {
            probe_format: format.probe_format.clone(),
            vlc_name: format.vlc_name.clone(),
            matrix_rows: format.matrix_rows.clone(),
            claims: format
                .matrix_rows
                .iter()
                .filter_map(|row| claims.get(row.as_str()))
                .map(|claim| claim_out(claim))
                .collect(),
            files: valid
                .iter()
                .chain(refused.iter())
                .map(|(file, _)| file.rel.clone())
                .collect(),
            malformed_files: malformed,
            cells,
        });
    }

    let mut codec_rows: Vec<CodecRowOut> = Vec::new();
    for codec in &config.codecs {
        // The files with at least one track of this family.
        let mut valid: Vec<&(CorpusFile, FileRecord)> = Vec::new();
        for entry in &records {
            if matches!(entry.0.class, CorpusClass::Malformed) {
                continue;
            }
            let has_track = entry.1.tracks.iter().any(|track| match track {
                TrackOutcome::Decoded { codec: c, .. }
                | TrackOutcome::NoDecoder { codec: c, .. } => {
                    if codec.prefix_match {
                        c.starts_with(&codec.codec)
                    } else {
                        c == &codec.codec
                    }
                }
            });
            if has_track {
                valid.push(entry);
            }
        }
        let mut cells: Vec<Cell> = Vec::new();
        for lane in &config.lanes {
            let steps = lane_steps(&lanes, &lane.id);
            cells.push(codec_cell(lane, steps, &valid, codec));
        }
        for row_id in &codec.matrix_rows {
            let Some(claim) = claims.get(row_id.as_str()) else {
                continue;
            };
            codec_gap(row_id, claim, &valid, codec, &native, &mut transitions);
        }
        codec_rows.push(CodecRowOut {
            codec: codec.codec.clone(),
            kind: codec.kind.clone(),
            matrix_rows: codec.matrix_rows.clone(),
            claims: codec
                .matrix_rows
                .iter()
                .filter_map(|row| claims.get(row.as_str()))
                .map(|claim| claim_out(claim))
                .collect(),
            files: valid.iter().map(|(file, _)| file.rel.clone()).collect(),
            cells,
        });
    }

    // Notes: the honest boundary statements.
    let mut notes = Vec::new();
    notes.push(format!(
        "The native-decode lane (`{native_lane}`, platform `{native_platform}`) is the only lane \
         that decoded media in this run. Cells of the lanes that did not run are blocked-lane \
         citations (3-OS runner machines + CI billing are user-owned blockers — \
         docs/plans/phase8-cards.md §1); NO result is inferred for platforms that did not run."
    ));
    notes.push(
        "The web/wasm lane evidence is the recorded build/test steps: the wasm32 module is \
         real (engine core: probe → demux → engine), but it ships DECODER-LESS on wasm32 — \
         the codecs crate does not build for the target (rav1d 1.1.0 asm-gated, measured \
         2026-11-10; apps/web/Cargo.toml target gate; apps/web/src/capabilities.rs reports \
         the split honestly). No decode is claimed or exercised on the web lane."
            .to_string(),
    );
    notes.push(
        "Oracle comparisons cover the lossless-comparable family only (PCM-in-AVI, the \
         documented first vertical): integer PCM via the MD5-prefix method (VLC's afile dump \
         is a bit-exact prefix; the shortfall is the unflushed tail), pcm_f32le via the \
         ratified RMS ≤ 1e-4 / peak ≤ 1e-3 tolerance path (docs/corpus/vlc-f32-tolerance-\
         20261109.json). Lossy codecs are not byte-compared — decoder implementations \
         legitimately differ; their rows carry decode-count evidence only."
            .to_string(),
    );
    notes.push(format!(
        "Transitions are PROPOSALS in the outbox ({}); only `cargo xtask compat-matrix \
         apply-transitions` writes the matrix, after re-checking each `from` status against \
         the live file. Nothing here self-verifies: no transition proposes `verified` (CI \
         billing is blocked; MP §15).",
        transitions.len()
    ));

    let report = RunReport {
        schema: "compat-matrix/report/1".to_string(),
        generated: generated.clone(),
        host: HostInfo {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        },
        git_commit,
        corpus: CorpusSummary {
            committed_files: committed_count,
            malformed_files: malformed_count,
            regen_files: counts
                .files_total
                .saturating_sub(committed_count)
                .saturating_sub(malformed_count),
            regen_provenance: corpus::read_manifest(config)
                .map(|manifest| manifest.provenance)
                .unwrap_or_default(),
            regen_outcomes,
        },
        lanes,
        oracle: OracleSummary {
            source: oracle_source,
            f32_record: config.oracle.f32_record.clone(),
            rows: oracle_rows,
            committed_cross_checks: cross_checks,
        },
        formats: format_rows,
        codecs: codec_rows,
        counts,
        transitions: transitions.clone(),
        notes,
    };

    // Render + write everything.
    let json_path = config.rel(&config.reports.json);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| ToolError::Subprocess(format!("serializing the report: {e}")))?;
    std::fs::write(&json_path, json.as_bytes())
        .map_err(|e| ToolError::io(format!("writing {}", json_path.display()), e))?;

    let markdown = render_markdown(config, &report);
    let md_path = config.rel(&config.reports.markdown);
    std::fs::write(&md_path, markdown.as_bytes())
        .map_err(|e| ToolError::io(format!("writing {}", md_path.display()), e))?;

    let outbox = TransitionsOutbox {
        schema: "compat-matrix/transitions/1".to_string(),
        generated: generated.clone(),
        report: config.reports.json.clone(),
        evidence: config.reports.markdown.clone(),
        transitions,
    };
    let outbox_path = config.rel(&config.reports.transitions);
    let outbox_json = serde_json::to_string_pretty(&outbox)
        .map_err(|e| ToolError::Subprocess(format!("serializing the outbox: {e}")))?;
    std::fs::write(&outbox_path, outbox_json.as_bytes())
        .map_err(|e| ToolError::io(format!("writing {}", outbox_path.display()), e))?;

    eprintln!(
        "compat-matrix: run complete in {:.1}s → {} (+ json, rows, transitions)",
        started.elapsed().as_secs_f64(),
        md_path.display()
    );
    Ok(())
}

/// The outbox file (schema `compat-matrix/transitions/1`).
#[derive(Debug, Serialize)]
struct TransitionsOutbox {
    schema: String,
    generated: String,
    /// The machine report this outbox cites.
    report: String,
    /// The human report (evidence prose).
    evidence: String,
    transitions: Vec<TransitionOut>,
}

/// Run one configured lane step for real.
fn run_step(config: &Config, step: &Step) -> Result<StepRecord, ToolError> {
    let cwd = if step.cwd.is_empty() {
        None
    } else {
        Some(config.rel(&step.cwd))
    };
    let record = proc::run_bounded(&step.argv, cwd.as_deref(), &[], step.timeout_s)?;
    let mut artifacts = Vec::new();
    for artifact in &step.artifacts {
        let path = config.rel(artifact);
        let bytes = std::fs::metadata(&path).map(|meta| meta.len()).ok();
        if let Some(bytes) = bytes {
            artifacts.push(ArtifactRecord {
                path: artifact.clone(),
                bytes,
            });
        }
    }
    eprintln!(
        "compat-matrix: lane step `{}`: {} ({:.1}s)",
        step.purpose, record.outcome, record.wall_s
    );
    Ok(StepRecord {
        purpose: step.purpose.clone(),
        argv: step.argv.clone(),
        outcome: record.outcome,
        wall_s: record.wall_s,
        artifacts,
    })
}

/// Count one file's outcome.
fn count_file(record: &FileRecord, malformed: bool, counts: &mut Counts) {
    if malformed {
        // Every malformed-file outcome the runner observes is by
        // construction a typed result (a crash would have killed the
        // run); the report lists the distribution.
        counts.malformed_typed += 1;
        return;
    }
    match &record.outcome {
        FileOutcome::Decoded => counts.decoded += 1,
        FileOutcome::DecodedPartial => counts.decoded_partial += 1,
        FileOutcome::RefusedProbe { .. } => counts.refused_probe += 1,
        FileOutcome::RefusedDemux { .. } => counts.refused_demux += 1,
        FileOutcome::DemuxFatal { .. } => counts.demux_fatal += 1,
        FileOutcome::RefusedInput { .. } => counts.refused_input += 1,
    }
}

/// The refusal/fatal message of an outcome.
fn outcome_message(outcome: &FileOutcome) -> String {
    match outcome {
        FileOutcome::Decoded => "decoded".to_string(),
        FileOutcome::DecodedPartial => "decoded (partial: a track lacked a decoder)".to_string(),
        FileOutcome::RefusedProbe { message }
        | FileOutcome::RefusedDemux { message }
        | FileOutcome::DemuxFatal { message }
        | FileOutcome::RefusedInput { message } => message.clone(),
    }
}

/// The container-level verdict over one format's valid files.
fn container_verdict(valid: &[&(CorpusFile, FileRecord)]) -> &'static str {
    if valid.is_empty() {
        return "not-covered";
    }
    let mut worst = "decoded";
    for (_, record) in valid {
        let level = match &record.outcome {
            FileOutcome::Decoded | FileOutcome::DecodedPartial => 0,
            FileOutcome::DemuxFatal { .. } => 2,
            FileOutcome::RefusedProbe { .. }
            | FileOutcome::RefusedDemux { .. }
            | FileOutcome::RefusedInput { .. } => 3,
        };
        if level == 3 {
            return "refused";
        }
        if level == 2 && worst != "refused" {
            worst = "demux-fatal";
        }
    }
    worst
}

/// The steps-lane cell, DERIVED from the recorded step outcomes
/// (review finding P2-2): the cell text can never claim a build the
/// lane's own recorded steps contradict. Every step `exit 0` → the
/// engine-core-only claim (the pass-case detail is per row kind);
/// any nonzero/timeout/termination → `steps-failed` quoting the
/// first failing step's purpose, argv and recorded outcome; a steps
/// lane with no recorded steps claims nothing.
fn steps_cell(lane: &crate::config::Lane, steps: &[StepRecord], pass_detail: &str) -> Cell {
    if let Some(worst) = steps.iter().find(|step| step.outcome != "exit 0") {
        return Cell {
            lane: lane.id.clone(),
            status: "steps-failed".to_string(),
            detail: format!(
                "lane step `{}` FAILED: `{}` → {} — the recorded outcome contradicts the build \
                 claim (see the lane's recorded steps); DECODE refused — the wasm32 module ships \
                 decoder-less (apps/web/src/capabilities.rs); no decode claimed",
                worst.purpose,
                worst.argv.join(" "),
                worst.outcome
            ),
        };
    }
    if steps.is_empty() {
        return Cell {
            lane: lane.id.clone(),
            status: "not-covered".to_string(),
            detail: "the lane recorded no steps (nothing claimed)".to_string(),
        };
    }
    Cell {
        lane: lane.id.clone(),
        status: "engine-core-only".to_string(),
        detail: pass_detail.to_string(),
    }
}

/// The recorded steps of one configured lane (steps lanes carry
/// their executed commands' outcomes; the other kinds carry none —
/// the record exists for every configured lane, so the lookup only
/// misses on a config/record mismatch that cannot happen in one
/// run, and the empty answer is the honest no-steps rendering).
fn lane_steps<'a>(lanes: &'a [LaneRecord], lane_id: &str) -> &'a [StepRecord] {
    lanes
        .iter()
        .find(|record| record.id == lane_id)
        .map(|record| record.steps.as_slice())
        .unwrap_or(&[])
}

/// One format-row cell for one lane.
fn format_cell(
    lane: &crate::config::Lane,
    steps: &[StepRecord],
    valid: &[&(CorpusFile, FileRecord)],
    refused: &[&(CorpusFile, FileRecord)],
    malformed: usize,
) -> Cell {
    match &lane.kind {
        LaneKind::NativeDecode { .. } => {
            if valid.is_empty() && refused.is_empty() {
                return Cell {
                    lane: lane.id.clone(),
                    status: "not-covered".to_string(),
                    detail: "no corpus file of this format (honest: the cell claims nothing)"
                        .to_string(),
                };
            }
            if !valid.is_empty() {
                let verdict = container_verdict(valid);
                let decoded = valid
                    .iter()
                    .filter(|(_, r)| matches!(r.outcome, FileOutcome::Decoded))
                    .count();
                let partial = valid
                    .iter()
                    .filter(|(_, r)| matches!(r.outcome, FileOutcome::DecodedPartial))
                    .count();
                // The row's honest denominator is EVERY valid-class
                // corpus file of the family — the probe-matched files
                // PLUS the extension-matched ones the open path refused
                // (the `files` array behind the table's corpus-files
                // column is exactly `valid` + `refused`; the refusals
                // must be visible in the cell, never only in the raw
                // rows — review finding P2-1).
                let mut detail = format!(
                    "{}/{} files open+demux ({} fully decoded, {} with a codec-side NoDecoder track)",
                    valid.len(),
                    valid.len() + refused.len(),
                    decoded,
                    partial
                );
                if let Some((_, record)) = valid.iter().find(|(_, r)| {
                    matches!(
                        r.outcome,
                        FileOutcome::RefusedProbe { .. }
                            | FileOutcome::RefusedDemux { .. }
                            | FileOutcome::DemuxFatal { .. }
                    )
                }) {
                    detail.push_str("; worst: ");
                    detail.push_str(&outcome_message(&record.outcome));
                }
                if !refused.is_empty() {
                    detail.push_str(&format!(
                        "; {} extension-matched corpus file{} REFUSED at the open path — {}",
                        refused.len(),
                        if refused.len() == 1 { "" } else { "s" },
                        refusal_breakdown(refused)
                    ));
                }
                if malformed > 0 {
                    detail.push_str(&format!(
                        "; error-contract: {malformed} malformed variants, all typed (zero crashes)"
                    ));
                }
                Cell {
                    lane: lane.id.clone(),
                    status: verdict.to_string(),
                    detail,
                }
            } else {
                // Valid corpus files of this family that the probe
                // refused outright — the family has no probe
                // signature, so the open path never even names it.
                let worst = refused
                    .iter()
                    .find(|(_, r)| {
                        matches!(
                            r.outcome,
                            FileOutcome::RefusedProbe { .. }
                                | FileOutcome::RefusedDemux { .. }
                                | FileOutcome::RefusedInput { .. }
                        )
                    })
                    .map(|(_, r)| outcome_message(&r.outcome))
                    .unwrap_or_else(|| "refused".to_string());
                Cell {
                    lane: lane.id.clone(),
                    status: "refused".to_string(),
                    detail: format!(
                        "{}/{} corpus files REFUSED at the open path — {worst}",
                        refused.len(),
                        refused.len()
                    ),
                }
            }
        }
        LaneKind::Steps { .. } => steps_cell(
            lane,
            steps,
            "wasm32 module builds (see the lane's recorded steps); engine core (probe + demux \
             of this family) compiled in; DECODE refused — the wasm32 module ships decoder-less \
             (codecs crate does not build for wasm32; apps/web/src/capabilities.rs reports the \
             split); no in-browser run in this matrix",
        ),
        LaneKind::Blocked {
            reason,
            reason_ref,
            next_step,
            tracking,
        } => Cell {
            lane: lane.id.clone(),
            status: "blocked".to_string(),
            detail: format!(
                "blocked-by-external: {reason} [ref: {reason_ref}; next step: {next_step}; \
                 tracking: {tracking}]"
            ),
        },
    }
}

/// Render one claim snapshot entry (all four platform columns).
fn claim_out(claim: &MatrixClaim) -> ClaimOut {
    ClaimOut {
        row: claim.row.clone(),
        windows: claim.windows.clone(),
        linux: claim.linux.clone(),
        macos: claim.macos.clone(),
        web: claim.web.clone(),
    }
}

/// Summarize a row's in-row refusals for the cell detail: each
/// distinct typed message with its count (deterministic order —
/// alphabetical by message). The refusal bucket only ever holds
/// `Refused{Probe,Demux,Input}` records, so every entry is a real
/// typed refusal (review finding P2-1: counted, now surfaced).
fn refusal_breakdown(refused: &[&(CorpusFile, FileRecord)]) -> String {
    let mut by_message: BTreeMap<String, usize> = BTreeMap::new();
    for (_, record) in refused {
        *by_message
            .entry(outcome_message(&record.outcome))
            .or_default() += 1;
    }
    by_message
        .iter()
        .map(|(message, count)| format!("{message} ×{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One codec-row cell for one lane.
fn codec_cell(
    lane: &crate::config::Lane,
    steps: &[StepRecord],
    valid: &[&(CorpusFile, FileRecord)],
    codec: &crate::config::CodecRow,
) -> Cell {
    let matches_family = |descriptor: &str| {
        if codec.prefix_match {
            descriptor.starts_with(&codec.codec)
        } else {
            descriptor == codec.codec
        }
    };
    match &lane.kind {
        LaneKind::NativeDecode { .. } => {
            if valid.is_empty() {
                return Cell {
                    lane: lane.id.clone(),
                    status: "not-covered".to_string(),
                    detail: "no corpus track of this codec family".to_string(),
                };
            }
            let mut decoded = 0usize;
            let mut concealed_total = 0u64;
            let mut no_decoder = 0usize;
            let mut sample = String::new();
            for (_, record) in valid {
                for track in &record.tracks {
                    match track {
                        TrackOutcome::Decoded {
                            codec: c,
                            concealed,
                            sample: s,
                            ..
                        } if matches_family(c) => {
                            decoded += 1;
                            concealed_total += concealed;
                            if sample.is_empty()
                                && let Some(s) = s
                            {
                                sample = s.clone();
                            }
                        }
                        TrackOutcome::NoDecoder { codec: c, .. } if matches_family(c) => {
                            no_decoder += 1;
                        }
                        _ => {}
                    }
                }
            }
            let status = if no_decoder > 0 {
                "no-decoder"
            } else if concealed_total > 0 {
                "decoded-damaged"
            } else {
                "decoded"
            };
            let mut detail = format!(
                "{decoded} tracks decoded across {} files ({} concealed packets)",
                valid.len(),
                concealed_total
            );
            if !sample.is_empty() {
                detail.push_str(&format!("; sample format {sample}"));
            }
            if no_decoder > 0 {
                detail.push_str(&format!(
                    "; {no_decoder} tracks REFUSED: no decoder in this build"
                ));
            }
            Cell {
                lane: lane.id.clone(),
                status: status.to_string(),
                detail,
            }
        }
        LaneKind::Steps { .. } => steps_cell(
            lane,
            steps,
            "wasm32 module builds; DECODE refused for every codec — the module ships decoder-less \
             on wasm32 (apps/web/src/capabilities.rs); no decode claimed",
        ),
        LaneKind::Blocked {
            reason,
            reason_ref,
            next_step,
            tracking,
        } => Cell {
            lane: lane.id.clone(),
            status: "blocked".to_string(),
            detail: format!(
                "blocked-by-external: {reason} [ref: {reason_ref}; next step: {next_step}; \
                 tracking: {tracking}]"
            ),
        },
    }
}

/// One format row's assembled evidence (the gap rule's input bundle).
struct FormatEvidence<'a> {
    /// Files the open path named as this family.
    valid: &'a [&'a (CorpusFile, FileRecord)],
    /// Valid-class files of this family the probe refused outright.
    refused: &'a [&'a (CorpusFile, FileRecord)],
    /// The probe's format id.
    probe_format: &'a str,
}

/// The native-lane context the gap rules act within (config/2): the
/// lane id (prose), the platform column this host's sweep is
/// evidence for, and the run's own report path (the evidence string
/// the emitted transitions cite).
struct NativeCtx<'a> {
    lane: &'a str,
    platform: &'a str,
    evidence: &'a str,
}

/// The container gap rule (8.B2): a claimed-supported container row
/// whose corpus files cannot open/demux is a gap. Judged against the
/// NATIVE lane's platform column — the sweep is evidence for exactly
/// that column, never any other.
fn container_gap(
    row_id: &str,
    claim: &MatrixClaim,
    verdict: &str,
    evidence_rows: &FormatEvidence<'_>,
    native: &NativeCtx<'_>,
    transitions: &mut Vec<TransitionOut>,
) {
    let NativeCtx {
        lane: native_lane,
        platform: native_platform,
        evidence: run_evidence,
    } = native;
    let (valid, refused, probe_format) = (
        evidence_rows.valid,
        evidence_rows.refused,
        evidence_rows.probe_format,
    );
    let claimed = claim.status_for(native_platform);
    match (claimed, verdict) {
        ("implemented" | "verified", "refused") => {
            let worst = valid
                .iter()
                .chain(refused.iter())
                .find(|(_, r)| {
                    matches!(
                        r.outcome,
                        FileOutcome::RefusedProbe { .. }
                            | FileOutcome::RefusedDemux { .. }
                            | FileOutcome::RefusedInput { .. }
                    )
                })
                .map(|(_, r)| outcome_message(&r.outcome))
                .unwrap_or_else(|| "refused".to_string());
            transitions.push(TransitionOut {
                row: row_id.to_string(),
                platform: native_platform.to_string(),
                from: claimed.to_string(),
                to: "partial".to_string(),
                reason: format!(
                    "compat-matrix run ({native_lane} lane): the open path REFUSES the corpus \
                     files of probe format `{probe_format}` — {worst}. The container claim \
                     cannot be exercised end-to-end through the player's open path."
                ),
                next_step: format!(
                    "wire the missing probe signature / container-registry arm for \
                     `{probe_format}` (the ivf/mpegps open-path wiring precedent), then re-run \
                     the matrix"
                ),
                evidence: run_evidence.to_string(),
            });
        }
        ("implemented" | "verified", "demux-fatal") => {
            transitions.push(TransitionOut {
                row: row_id.to_string(),
                platform: native_platform.to_string(),
                from: claimed.to_string(),
                to: "partial".to_string(),
                reason: format!(
                    "compat-matrix run ({native_lane} lane): a mid-walk demux fatal on a \
                     valid corpus file of probe format `{probe_format}` — see the raw rows"
                ),
                next_step: "fix the demuxer walk defect (bounded fix), add the corpus file as \
                            a regression fixture, re-run the matrix"
                    .to_string(),
                evidence: run_evidence.to_string(),
            });
        }
        ("not_started" | "in_progress", "decoded") if !valid.is_empty() => {
            transitions.push(TransitionOut {
                row: row_id.to_string(),
                platform: native_platform.to_string(),
                from: claimed.to_string(),
                to: "implemented".to_string(),
                reason: format!(
                    "compat-matrix run ({native_lane} lane): {} corpus files of probe format \
                     `{probe_format}` open + demux through the real player path (underclaim — \
                     the matrix says {})",
                    valid.len(),
                    claimed
                ),
                next_step: "n/a (upgrade with run evidence; the applier verifies the from-status)"
                    .to_string(),
                evidence: run_evidence.to_string(),
            });
        }
        _ => {}
    }
}

/// The codec gap rule: a claimed-supported codec row whose corpus
/// tracks lack a decoder (or decode damaged) is a gap. Judged against
/// the NATIVE lane's platform column.
fn codec_gap(
    row_id: &str,
    claim: &MatrixClaim,
    valid: &[&(CorpusFile, FileRecord)],
    codec: &crate::config::CodecRow,
    native: &NativeCtx<'_>,
    transitions: &mut Vec<TransitionOut>,
) {
    let NativeCtx {
        lane: native_lane,
        platform: native_platform,
        evidence: run_evidence,
    } = native;
    let matches_family = |descriptor: &str| {
        if codec.prefix_match {
            descriptor.starts_with(&codec.codec)
        } else {
            descriptor == codec.codec
        }
    };
    let mut decoded = 0usize;
    let mut no_decoder = 0usize;
    let mut concealed_total = 0u64;
    for (_, record) in valid {
        for track in &record.tracks {
            match track {
                TrackOutcome::Decoded {
                    codec: c,
                    concealed,
                    ..
                } if matches_family(c) => {
                    decoded += 1;
                    concealed_total += concealed;
                }
                TrackOutcome::NoDecoder { codec: c, .. } if matches_family(c) => {
                    no_decoder += 1;
                }
                _ => {}
            }
        }
    }
    if valid.is_empty() {
        return;
    }
    let claimed = claim.status_for(native_platform);
    match claimed {
        "implemented" | "verified" if no_decoder > 0 => {
            transitions.push(TransitionOut {
                row: row_id.to_string(),
                platform: native_platform.to_string(),
                from: claimed.to_string(),
                to: "partial".to_string(),
                reason: format!(
                    "compat-matrix run ({native_lane} lane): {no_decoder} corpus tracks of \
                     codec `{}` REFUSED (no decoder in this build) vs {decoded} decoded",
                    codec.codec
                ),
                next_step: "land the missing decoder lane (bounded fix) or split the row's \
                            claimed descriptor set; re-run the matrix"
                    .to_string(),
                evidence: run_evidence.to_string(),
            });
        }
        "implemented" | "verified" if concealed_total > 0 => {
            transitions.push(TransitionOut {
                row: row_id.to_string(),
                platform: native_platform.to_string(),
                from: claimed.to_string(),
                to: "partial".to_string(),
                reason: format!(
                    "compat-matrix run ({native_lane} lane): {concealed_total} concealed \
                     (damaged) packets on VALID corpus files of codec `{}` — the decode is \
                     not clean",
                    codec.codec
                ),
                next_step: "triage the concealment (bounded fix, no correctness trade); add \
                            the corpus file as a regression fixture; re-run"
                    .to_string(),
                evidence: run_evidence.to_string(),
            });
        }
        "not_started" | "in_progress" if decoded > 0 && no_decoder == 0 => {
            transitions.push(TransitionOut {
                row: row_id.to_string(),
                platform: native_platform.to_string(),
                from: claimed.to_string(),
                to: "implemented".to_string(),
                reason: format!(
                    "compat-matrix run ({native_lane} lane): {decoded} corpus tracks of codec \
                     `{}` decode through the real player path (underclaim — the matrix says {})",
                    decoded, claimed
                ),
                next_step: "n/a (upgrade with run evidence; the applier verifies the from-status)"
                    .to_string(),
                evidence: run_evidence.to_string(),
            });
        }
        _ => {}
    }
}

/// The audiofile format for a corpus file, when it is an
/// oracle-comparable row (the config's comparable rels).
fn audiofile_format_for(config: &Config, file: &CorpusFile) -> Option<String> {
    if !config
        .oracle
        .comparable_rels
        .iter()
        .any(|rel| rel == &file.rel)
    {
        return None;
    }
    // The regen manifest carries the per-entry VLC afile format.
    comparable_entry(config, file).and_then(|entry| entry.audiofile_format.clone())
}

/// The committed-record key for a corpus file (the regen manifest's
/// explicit mapping).
fn oracle_rel_for(config: &Config, file: &CorpusFile) -> Option<String> {
    comparable_entry(config, file).and_then(|entry| entry.oracle_rel.clone())
}

/// The regen manifest entry behind a comparable corpus file.
fn comparable_entry(config: &Config, file: &CorpusFile) -> Option<corpus::RegenEntry> {
    let suffix = file
        .rel
        .strip_prefix(&format!("{}/", config.corpus.regen.cache_dir))?
        .to_string();
    corpus::read_manifest(config)
        .ok()?
        .entries
        .into_iter()
        .find(|entry| entry.rel == suffix)
}

/// A filesystem-safe name for dump files.
fn safe_name(rel: &str) -> String {
    rel.replace(['/', '\\'], "_")
}

/// The current commit (`git rev-parse HEAD`), when git is available.
fn git_commit() -> Option<String> {
    let argv = vec![
        "git".to_string(),
        "rev-parse".to_string(),
        "HEAD".to_string(),
    ];
    let record = proc::run_bounded(&argv, None, &[], 10).ok()?;
    if record.outcome == "exit 0" {
        let sha = record.stdout_head.trim().to_string();
        if sha.len() >= 7 {
            return Some(sha);
        }
    }
    None
}

/// A UTC ISO-8601 timestamp (std-only: the system clock + the
/// well-known civil-date conversion — no external crates).
fn iso8601_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = now / 86_400;
    let secs = now % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Days-since-epoch → (y, m, d) — Howard Hinnant's civil-from-days.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

// ---------------------------------------------------------------------------
// The markdown rendering
// ---------------------------------------------------------------------------

fn render_markdown(config: &Config, report: &RunReport) -> String {
    let native_platform = native_platform_of(config);
    let mut out = String::with_capacity(48 * 1024);
    out.push_str(
        "# Full-corpus compatibility matrix — 2026-11-10 (issue #53, cards 8.B1–8.B2)\n\n",
    );
    out.push_str("| Field | Value |\n|---|---|\n");
    out.push_str("| Schema | `compat-matrix/report/1` (this file is the human rendering; machine JSON alongside) |\n");
    out.push_str(&format!(
        "| Run | `{}` on {} / {} (arch recorded; the native lane decodes on exactly one host) |\n",
        report.generated, report.host.os, report.host.arch
    ));
    if let Some(commit) = &report.git_commit {
        out.push_str(&format!("| Commit | `{commit}` |\n"));
    }
    out.push_str(&format!(
        "| Corpus | {} committed + {} malformed (error-contract) + {} regenerable (manifest-anchored, `tests/compat/regen-corpus-manifest.json`) |\n",
        report.corpus.committed_files, report.corpus.malformed_files, report.corpus.regen_files
    ));
    out.push_str(&format!("| Oracle | {} |\n", report.oracle.source));
    out.push_str("| Method | real decode through the player's open path (`lumen_cli::demux_host::open_source` → container registry → `HostDecoderProvider` → `codec_api::Decoder`); MD5-prefix + ratified f32 tolerance vs the pinned VLC 3.0.23 (see notes) |\n");
    out.push('\n');

    // Headline counts.
    out.push_str("## 1. Headline counts (the native lane)\n\n");
    out.push_str(&format!(
        "- Corpus files: **{}** ({} decoded, {} decoded-partial, {} refused, {} demux-fatal; \
         {} malformed all-typed / zero crashes)\n",
        report.counts.files_total,
        report.counts.decoded,
        report.counts.decoded_partial,
        report.counts.refused_probe + report.counts.refused_demux + report.counts.refused_input,
        report.counts.demux_fatal,
        report.counts.malformed_typed
    ));
    out.push_str(&format!(
        "- Oracle differential (lossless-comparable family): **{} bit-exact** (MD5-prefix), \
         **{} f32 tolerance-pass**, **{} f32 tolerance-FAIL**, **{} not-bit-exact**, {} \
         no-comparison\n",
        report.counts.oracle_bit_exact,
        report.counts.oracle_tolerance_pass,
        report.counts.oracle_tolerance_fail,
        report.counts.oracle_not_bit_exact,
        report.counts.oracle_no_comparison
    ));
    out.push_str(&format!(
        "- Lanes: {} — native decode on this host; web/wasm via recorded build steps; \
         Windows/macOS blocked-by-external (user-owned)\n",
        report
            .lanes
            .iter()
            .map(|lane| format!("`{}` ({})", lane.id, lane.kind))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push('\n');

    // The lanes.
    out.push_str("## 2. The lanes and their evidence\n\n");
    for lane in &report.lanes {
        out.push_str(&format!(
            "### `{}` — {} ({})\n\n",
            lane.id, lane.label, lane.kind
        ));
        match (&lane.steps, &lane.blocked) {
            (steps, None) if !steps.is_empty() => {
                for step in steps {
                    out.push_str(&format!(
                        "- `{}` — `{}` → **{}** ({:.1}s)\n",
                        step.purpose,
                        step.argv.join(" "),
                        step.outcome,
                        step.wall_s
                    ));
                }
            }
            _ => {}
        }
        if let Some(blocked) = &lane.blocked {
            out.push_str(&format!(
                "- **blocked-by-external**: {}\n  - ref: {}\n  - next step: {}\n  - tracking: {}\n",
                blocked.reason, blocked.reason_ref, blocked.next_step, blocked.tracking
            ));
        }
        out.push('\n');
    }

    // The format matrix.
    out.push_str("## 3. Container formats × lanes\n\n");
    out.push_str(&format!(
        "| probe format | claimed ({native_platform}) | corpus files |"
    ));
    for lane in &report.lanes {
        out.push_str(&format!(" {} |", lane.id));
    }
    out.push('\n');
    out.push_str("|---|---|---|");
    for _ in &report.lanes {
        out.push_str("---|");
    }
    out.push('\n');
    for row in &report.formats {
        let claim = row
            .claims
            .iter()
            .map(|c| {
                format!(
                    "{}=L:{}/W:{}/M:{}/B:{}",
                    short_row(&c.row),
                    c.linux,
                    c.windows,
                    c.macos,
                    c.web
                )
            })
            .collect::<Vec<_>>()
            .join("<br>");
        out.push_str(&format!(
            "| `{}` ({}) | {} | {} |",
            row.probe_format,
            row.vlc_name,
            claim,
            row.files.len()
        ));
        for cell in &row.cells {
            out.push_str(&format!(" {} |", cell.status));
        }
        out.push('\n');
    }
    out.push('\n');
    out.push_str("Cell details (native lane):\n\n");
    for row in &report.formats {
        for cell in &row.cells {
            if cell.lane == native_lane_of(config) {
                out.push_str(&format!(
                    "- `{}`: {} — {}\n",
                    row.probe_format, cell.status, cell.detail
                ));
            }
        }
    }
    out.push('\n');

    // The codec matrix.
    out.push_str("## 4. Codec families × lanes\n\n");
    out.push_str(&format!(
        "| codec | claimed ({native_platform}) | corpus files |"
    ));
    for lane in &report.lanes {
        out.push_str(&format!(" {} |", lane.id));
    }
    out.push('\n');
    out.push_str("|---|---|---|");
    for _ in &report.lanes {
        out.push_str("---|");
    }
    out.push('\n');
    for row in &report.codecs {
        let claim = row
            .claims
            .iter()
            .map(|c| {
                format!(
                    "{}=L:{}/W:{}/M:{}/B:{}",
                    short_row(&c.row),
                    c.linux,
                    c.windows,
                    c.macos,
                    c.web
                )
            })
            .collect::<Vec<_>>()
            .join("<br>");
        out.push_str(&format!(
            "| `{}` ({}) | {} | {} |",
            row.codec,
            row.kind,
            claim,
            row.files.len()
        ));
        for cell in &row.cells {
            out.push_str(&format!(" {} |", cell.status));
        }
        out.push('\n');
    }
    out.push('\n');
    out.push_str("Cell details (native lane):\n\n");
    for row in &report.codecs {
        for cell in &row.cells {
            if cell.lane == native_lane_of(config) {
                out.push_str(&format!(
                    "- `{}`: {} — {}\n",
                    row.codec, cell.status, cell.detail
                ));
            }
        }
    }
    out.push('\n');

    // The oracle table.
    out.push_str("## 5. The VLC 3.0.23 differential (lossless-comparable family)\n\n");
    out.push_str(
        "| corpus file | afile fmt | lumen B | lumen md5 | vlc B | vlc md5 | source | verdict |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for row in &report.oracle.rows {
        let verdict = match &row.verdict {
            OracleVerdict::BitExactPrefix {
                unflushed_tail_bytes,
            } => {
                format!(
                    "**bit-exact prefix** (tail {unflushed_tail_bytes} B unflushed on VLC's side)"
                )
            }
            OracleVerdict::TolerancePass => {
                if let Some(dev) = &row.deviation {
                    format!(
                        "f32 tolerance PASS (rms {:.2e} ≤ {:.0e}, peak {:.2e} ≤ {:.0e}, {} samples)",
                        dev.rms, dev.rms_max, dev.peak, dev.peak_max, dev.samples
                    )
                } else {
                    "f32 tolerance PASS".to_string()
                }
            }
            OracleVerdict::ToleranceFail => {
                if let Some(dev) = &row.deviation {
                    format!(
                        "f32 tolerance **FAIL** (rms {:.2e} > {:.0e} or peak {:.2e} > {:.0e}, {} samples)",
                        dev.rms, dev.rms_max, dev.peak, dev.peak_max, dev.samples
                    )
                } else {
                    "f32 tolerance **FAIL**".to_string()
                }
            }
            OracleVerdict::NotBitExact => "**not bit-exact**".to_string(),
            OracleVerdict::LumenRefused { reason } => format!("lumen refused: {reason}"),
            OracleVerdict::OracleFailed { reason } => format!("oracle failed: {reason}"),
            OracleVerdict::NoComparison => {
                "no comparison (fresh random realization; no live oracle configured)".to_string()
            }
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | `{}` | {} | `{}` | {} | {} |\n",
            row.rel,
            row.audiofile_format,
            row.lumen_bytes,
            truncate(&row.lumen_md5, 12),
            row.vlc_bytes,
            truncate(&row.vlc_md5, 12),
            match row.vlc_source {
                VlcSource::LiveRun => "live run",
                VlcSource::CommittedRecord => "committed record",
                VlcSource::None => "none",
            },
            verdict
        ));
    }
    out.push('\n');
    if !report.oracle.committed_cross_checks.is_empty() {
        out.push_str("Committed-record cross-checks (2026-11-09 run):\n\n");
        for check in &report.oracle.committed_cross_checks {
            out.push_str(&format!("- {check}\n"));
        }
        out.push('\n');
    }

    // The gaps.
    out.push_str("## 6. Gap triage (8.B2) — the transitions outbox\n\n");
    if report.transitions.is_empty() {
        out.push_str(
            "No gaps: every claimed-supported row with corpus coverage agrees with \
                     the run, and every underclaim discovered is listed here when it occurs.\n\n",
        );
    } else {
        out.push_str(&format!(
            "{} transition proposals (applied by `cargo xtask compat-matrix apply-transitions \
             --outbox {}` — the parity tool re-checks each `from` status against the live \
             matrix before writing):\n\n",
            report.transitions.len(),
            config.reports.transitions
        ));
        for transition in &report.transitions {
            out.push_str(&format!(
                "- **{}** ({}): `{}` → `{}` — {}\n  - next step: {}\n  - evidence: {}\n",
                transition.row,
                transition.platform,
                transition.from,
                transition.to,
                transition.reason,
                transition.next_step,
                transition.evidence
            ));
        }
        out.push('\n');
    }

    // The regen outcomes.
    out.push_str("## 7. Regenerable-corpus outcomes\n\n");
    for (rel, outcome) in &report.corpus.regen_outcomes {
        let text = match outcome {
            RegenOutcome::PresentVerified => "present, sha256 verified".to_string(),
            RegenOutcome::PresentRandom { sha256 } => {
                format!("present (prior realization, sha256 {sha256})")
            }
            RegenOutcome::RegeneratedVerified => "regenerated, sha256 verified".to_string(),
            RegenOutcome::RegeneratedRandom { sha256 } => {
                format!("regenerated (fresh realization, sha256 {sha256})")
            }
            RegenOutcome::Failed { error } => format!("**FAILED**: {error}"),
        };
        out.push_str(&format!("- `{rel}`: {text}\n"));
    }
    out.push('\n');

    // The notes.
    out.push_str("## 8. Honest limits of this run\n\n");
    for note in &report.notes {
        out.push_str(&format!("- {note}\n"));
    }
    out.push('\n');
    out
}

/// The native lane id of this config.
fn native_lane_of(config: &Config) -> String {
    config
        .lanes
        .iter()
        .find(|lane| matches!(lane.kind, LaneKind::NativeDecode { .. }))
        .map(|lane| lane.id.clone())
        .unwrap_or_else(|| "linux-native".to_string())
}

/// The native lane's platform column of this config (config/2).
fn native_platform_of(config: &Config) -> String {
    config
        .lanes
        .iter()
        .find_map(|lane| match &lane.kind {
            LaneKind::NativeDecode { platform } => Some(platform.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "linux".to_string())
}

/// Shorten a matrix row id for the table (`container-avi` → `avi`).
fn short_row(row: &str) -> String {
    for prefix in ["container-", "codec-audio-", "codec-video-"] {
        if let Some(short) = row.strip_prefix(prefix) {
            return short.to_string();
        }
    }
    row.to_string()
}

/// Truncate a hex digest for table display.
fn truncate(text: &str, len: usize) -> String {
    if text.len() <= len {
        text.to_string()
    } else {
        format!("{}…", &text[..len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid-class corpus file fixture.
    fn corpus_file(rel: &str) -> CorpusFile {
        CorpusFile {
            rel: rel.to_string(),
            path: std::path::PathBuf::from(format!("/corpus/{rel}")),
            class: CorpusClass::Regen,
            dump_pcm: false,
        }
    }

    /// A FileRecord skeleton (outcome + probe format are what the
    /// cell logic reads).
    fn record(outcome: FileOutcome, probe_format: Option<&str>) -> FileRecord {
        FileRecord {
            rel: String::new(),
            path: String::new(),
            bytes: 0,
            outcome,
            probe_format: probe_format.map(String::from),
            tracks: Vec::new(),
            demux_recovered: 0,
            demux_skip_bytes: 0,
            skipped_non_av_tracks: 0,
            decode_s: 0.0,
        }
    }

    /// Borrow a fixture list the way the cell functions take it.
    fn as_refs(rows: &[(CorpusFile, FileRecord)]) -> Vec<&(CorpusFile, FileRecord)> {
        rows.iter().collect()
    }

    fn native_lane() -> crate::config::Lane {
        crate::config::Lane {
            id: "linux-native".to_string(),
            label: "Linux native decode".to_string(),
            kind: crate::config::LaneKind::NativeDecode {
                platform: "linux".to_string(),
            },
        }
    }

    /// P2-1: a MIXED row (probe-matched files that decode + an
    /// extension-matched file the open path refused) must surface the
    /// refusal in the cell text — the denominator is the row's whole
    /// valid-class file set, and the typed refusal messages are named.
    #[test]
    fn mixed_row_surfaces_its_refusals() {
        let valid_rows = [
            (
                corpus_file("a-avi-video.avi"),
                record(FileOutcome::Decoded, Some("avi")),
            ),
            (
                corpus_file("b-avi-video.avi"),
                record(FileOutcome::DecodedPartial, Some("avi")),
            ),
        ];
        let valid = as_refs(&valid_rows);
        let refused_rows = [(
            corpus_file("c-avi-video.avi"),
            record(
                FileOutcome::RefusedDemux {
                    message: "unsupported in avi (demux.avi.no-supported-streams)".to_string(),
                },
                None,
            ),
        )];
        let refused = as_refs(&refused_rows);
        let cell = format_cell(&native_lane(), &[], &valid, &refused, 0);
        assert_eq!(cell.status, "decoded");
        assert!(
            cell.detail.contains("2/3 files open+demux"),
            "denominator must count the refused file too: {}",
            cell.detail
        );
        assert!(
            cell.detail
                .contains("1 extension-matched corpus file REFUSED at the open path"),
            "the in-row refusal must be surfaced: {}",
            cell.detail
        );
        assert!(
            cell.detail
                .contains("unsupported in avi (demux.avi.no-supported-streams) ×1"),
            "the typed refusal message must be quoted with its count: {}",
            cell.detail
        );
    }

    /// P2-1: distinct refusal messages are each named with their own
    /// count (deterministic order).
    #[test]
    fn distinct_refusal_messages_are_each_counted() {
        let refused_rows = [
            (
                corpus_file("a.wav"),
                record(
                    FileOutcome::RefusedDemux {
                        message: "unsupported in wav (demux.wav.unsupported-format)".to_string(),
                    },
                    None,
                ),
            ),
            (
                corpus_file("b.wav"),
                record(
                    FileOutcome::RefusedProbe {
                        message: "no signature".to_string(),
                    },
                    None,
                ),
            ),
            (
                corpus_file("c.wav"),
                record(
                    FileOutcome::RefusedDemux {
                        message: "unsupported in wav (demux.wav.unsupported-format)".to_string(),
                    },
                    None,
                ),
            ),
        ];
        let refused = as_refs(&refused_rows);
        assert_eq!(
            refusal_breakdown(&refused),
            "no signature ×1, unsupported in wav (demux.wav.unsupported-format) ×2"
        );
    }

    /// P2-1 (control): a pure row renders exactly as before — same
    /// denominator semantics (n/n), no refusal clause.
    #[test]
    fn pure_row_has_no_refusal_clause() {
        let valid_rows = [
            (
                corpus_file("a.wav"),
                record(FileOutcome::Decoded, Some("wav")),
            ),
            (
                corpus_file("b.wav"),
                record(FileOutcome::Decoded, Some("wav")),
            ),
        ];
        let valid = as_refs(&valid_rows);
        let cell = format_cell(&native_lane(), &[], &valid, &[], 0);
        assert!(cell.detail.contains("2/2 files open+demux"));
        assert!(!cell.detail.contains("REFUSED"));
    }

    fn steps_lane() -> crate::config::Lane {
        crate::config::Lane {
            id: "web-wasm".to_string(),
            label: "Web/wasm".to_string(),
            kind: crate::config::LaneKind::Steps { steps: Vec::new() },
        }
    }

    fn step_record(purpose: &str, outcome: &str) -> StepRecord {
        StepRecord {
            purpose: purpose.to_string(),
            argv: vec!["cargo".to_string(), "test".to_string()],
            outcome: outcome.to_string(),
            wall_s: 1.0,
            artifacts: Vec::new(),
        }
    }

    fn codec_row() -> crate::config::CodecRow {
        crate::config::CodecRow {
            codec: "pcm_".to_string(),
            prefix_match: true,
            matrix_rows: vec!["codec-audio-pcm".to_string()],
            kind: "audio".to_string(),
        }
    }

    /// P2-2: a Steps lane whose recorded wasm build step FAILED must
    /// render the failure — never the hard-coded "builds" claim.
    #[test]
    fn failed_wasm_step_renders_steps_failed() {
        let lane = steps_lane();
        let steps = vec![
            step_record("build the wasm32 module", "exit 0"),
            step_record("test the wasm32 module", "exit 101"),
        ];
        let cell = format_cell(&lane, &steps, &[], &[], 0);
        assert_eq!(cell.status, "steps-failed");
        assert!(cell.detail.contains("exit 101"), "{}", cell.detail);
        assert!(
            cell.detail.contains("test the wasm32 module"),
            "{}",
            cell.detail
        );
        assert!(
            !cell.detail.contains("module builds"),
            "a failed step must not still claim the build: {}",
            cell.detail
        );
        // Codec rows derive from the same recorded outcomes.
        let codec = codec_cell(&lane, &steps, &[], &codec_row());
        assert_eq!(codec.status, "steps-failed");
        assert!(codec.detail.contains("exit 101"), "{}", codec.detail);
    }

    /// P2-2: a timeout is a failure too, and the all-pass rendering
    /// keeps the committed engine-core-only text byte-identical (the
    /// 2026-11-10 run's recorded steps were both `exit 0`).
    #[test]
    fn passing_wasm_steps_render_engine_core_only() {
        let lane = steps_lane();
        let steps = vec![
            step_record("build the wasm32 module", "exit 0"),
            step_record("test the wasm32 module", "exit 0"),
        ];
        let cell = format_cell(&lane, &steps, &[], &[], 0);
        assert_eq!(cell.status, "engine-core-only");
        assert_eq!(
            cell.detail,
            "wasm32 module builds (see the lane's recorded steps); engine core (probe + demux of this family) compiled in; DECODE refused — the wasm32 module ships decoder-less (codecs crate does not build for wasm32; apps/web/src/capabilities.rs reports the split); no in-browser run in this matrix"
        );
        let codec = codec_cell(&lane, &steps, &[], &codec_row());
        assert_eq!(codec.status, "engine-core-only");
        assert_eq!(
            codec.detail,
            "wasm32 module builds; DECODE refused for every codec — the module ships decoder-less on wasm32 (apps/web/src/capabilities.rs); no decode claimed"
        );
        let timed_out = vec![step_record("build the wasm32 module", "killed (timeout)")];
        let cell = format_cell(&lane, &timed_out, &[], &[], 0);
        assert_eq!(cell.status, "steps-failed");
        assert!(cell.detail.contains("killed (timeout)"), "{}", cell.detail);
    }

    /// P2-2: a steps lane with no recorded steps claims nothing.
    #[test]
    fn steps_lane_without_records_claims_nothing() {
        let lane = steps_lane();
        let cell = format_cell(&lane, &[], &[], &[], 0);
        assert_eq!(cell.status, "not-covered");
        assert!(cell.detail.contains("recorded no steps"), "{}", cell.detail);
    }

    /// P2-1 (control): the all-refused arm is unchanged — status
    /// `refused`, every file named as refused.
    #[test]
    fn all_refused_row_stays_refused() {
        let refused_rows = [
            (
                corpus_file("a.wmv"),
                record(
                    FileOutcome::RefusedProbe {
                        message: "unsupported".to_string(),
                    },
                    None,
                ),
            ),
            (
                corpus_file("b.wmv"),
                record(
                    FileOutcome::RefusedProbe {
                        message: "unsupported".to_string(),
                    },
                    None,
                ),
            ),
        ];
        let refused = as_refs(&refused_rows);
        let cell = format_cell(&native_lane(), &[], &[], &refused, 0);
        assert_eq!(cell.status, "refused");
        assert!(cell.detail.contains("2/2 corpus files REFUSED"));
    }
}
