# AUDIT: crates/media-probe/src/error.rs (lines 1-227, whole file)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Lines / functions | 227 / 12 items (7 constructors, ProbeHints struct+Display+Error, helper `ok`, 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open <any file>` → `apps/cli` open path → media-probe `probe()` (lib.rs) → these constructors: `no_demuxer` on no-hit, `stream_not_rewindable` lib.rs:199, `lookahead_zero`/`min_score_out_of_range` config.rs:54/60, `format_id_invalid`/`hint_codec_invalid` report.rs:141/234. Baseline: media_probe 80/80 tests ok (`orchrun-test-workspace-nofailfast.log.txt:3309,3325`), incl. this file's 4 tests at :3322-3325) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | full Read; `grep -rn` every constructor → mint sites + external code pins (lib.rs:386, report.rs:384/419, config.rs tests baseline-confirmed); `grep -rn "EngineError::new" media-probe/src` → only contract.rs (cfg(any(test, feature="contract")) — test-fixture faults, not probe failures, so the module-doc "every probe failure is minted here" holds for production); `sed -n` verification of every quote; baseline greps |

## 1. One-paragraph verdict
A small, disciplined error-mint module: all 7 constructors are live (each has a production mint site), all 7 codes are pinned by a test somewhere in the crate (three in config.rs, two in report.rs, one in lib.rs, two here), the sniffed-bytes cap and source-chain hints are asserted, and the whole thing executed green in the baseline. The defects are vocabulary-level: `min_score_out_of_range` smuggles a number through `FormatInfo::name` so the rendered error presents "150" as a format name; `hint_codec_invalid` and `format_id_invalid` discard the offending string entirely (no error carries what was invalid); and one of the four tests is misplaced report-module plumbing that the comment itself admits pins nothing about this file.

## 2. What actually works (verified)
- Every constructor is reachable and its code pinned somewhere: `lookahead_zero`/`min_score_out_of_range`/`lookahead_over_ceiling` by config tests (baseline: `config::tests::zero_lookahead_is_a_typed_error … ok` etc., nofailfast:3311-3314 region), `stream_not_rewindable` (lib.rs:386), `format_id_invalid` (report.rs:384), `hint_codec_invalid` (report.rs:419), `no_demuxer`/`lookahead_over_ceiling` here (:3322-3325).
- `no_demuxer` (20-35): safe slice `window[..window.len().min(16)]`, best-hit name passthrough, `ProbeHints` source chain with extension/size — test pins all three observables (:3325).
- `limit_error_reports_the_doc_ceiling` (189-208) pins the full Display string against `MAX_LOOKAHEAD_CEILING` (1_048_576) — an exact-render regression test.
- `ProbeHints` Display (118-127): "extension={ext}, size={size}" with none/unknown fallbacks — no secrets (doc claim at 109-111 holds: only extension and size ride the chain).

## 3. Findings

### F-001 [S4]
Where: crates/media-probe/src/error.rs:61-72
Code:
```rust
pub fn min_score_out_of_range(attempted: u8) -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some(attempted.to_string()),
```
What is wrong: the attempted score is smuggled into `FormatInfo::name` — a field whose contract is a format name. The user-facing `Display` renders the number as the unsupported "format" (the same shape as `not_adts`'s `name: Some("adts")`), so the message reads as an unsupported format literally named "150".
Why it matters: error vocabulary that lies about its own fields; any consumer keying on `FormatInfo::name` (UI hints, telemetry grouping) gets a numeric string where a format name is documented to live.
What correct looks like: a taxonomy slot that carries an out-of-range value (or the attempted number in the context code's detail), leaving `FormatInfo::name` a name.
Confidence: HIGH — Verification: read (kind/field definitions in media-types read via use-site; the same abuse pattern is *not* used by sibling constructors except `stream_not_rewindable`'s scheme, which at least is a name-ish string).

### F-002 [S4]
Where: crates/media-probe/src/error.rs:92-107
Code:
```rust
pub fn hint_codec_invalid() -> EngineError {
    EngineError::new(
        PipelineStage::Probe,
        ErrorKind::Unsupported { format: None },
        ErrorContext::new("probe.hint-codec-invalid"),
    )
}

/// A format id string outside the grammar (the extension path).
pub fn format_id_invalid() -> EngineError {
```
What is wrong: both constructors drop the offending value. At the mint sites (report.rs:141 — a format-id string failed the grammar; report.rs:234 — a track-hint codec string failed the descriptor grammar) the invalid string is in hand, but neither constructor accepts it, so the error cannot say *what* was invalid.
Why it matters: "missing error context" is the checklist's own item — a user configuring a bad track hint gets `probe.hint-codec-invalid` with no hint of which string failed; debugging requires re-running with a debugger.
What correct looks like: `hint_codec_invalid(offending: &str)` carrying the string (capped) in `FormatInfo::sniffed` or a detail field, as `no_demuxer` already models.
Confidence: HIGH — Verification: read (mint sites grepped and read; both take no argument).

### F-003 [S4]
Where: crates/media-probe/src/error.rs:211-226
Code:
```rust
    fn confidence_and_hit_construction_stay_in_range() {
        // The error constructors take validated vocabulary; the report
        // module owns range enforcement (pinned there). This pins the
        // constructor plumbing compiles against the real types.
```
What is wrong: this test exercises zero error-constructor logic — it builds a `ProbeHit`, converts score 70 through `Confidence::from_score`, and asserts `meets(60)`. That is report-module plumbing (whose real range tests live in report.rs, as the comment itself concedes), parked in the error module's test block.
Why it matters: a reader scanning error.rs's tests believes constructor behavior is covered by four tests when one of the four covers nothing here; the name ("stay in range") promises range enforcement that this file does not own or perform.
What correct looks like: delete it (report.rs owns the range pins) or move it next to `Confidence`.
Confidence: HIGH — Verification: read (whole test body; no error constructor called).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `no_demuxer` | 20-35 | REAL | — | safe cap slice; hints source chain; pinned twice (tests + baseline) |
| `lookahead_zero` | 38-44 | REAL | — | minted config.rs:54; pinned by config tests (baseline ok) |
| `lookahead_over_ceiling` | 48-58 | REAL | — | budget from `MAX_LOOKAHEAD_CEILING`; Display pinned exactly (test :189-208) |
| `min_score_out_of_range` | 61-72 | REAL | S4 | F-001: number in FormatInfo::name |
| `stream_not_rewindable` | 78-89 | REAL | — | scheme in name slot — name-ish, acceptable; pinned lib.rs:386 |
| `hint_codec_invalid` | 92-98 | REAL | S4 | F-002: offending string dropped |
| `format_id_invalid` | 101-107 | REAL | S4 | F-002: offending string dropped |
| `ProbeHints` + `Display` + `Error` | 112-129 | REAL | — | extension/size only; no secrets (doc claim holds) |
| test `no_demuxer_carries_sniffed_bytes_and_hints` | 144-164 | REAL | — | pins cap-16 + hints chain |
| test `no_demuxer_names_the_best_subthreshold_hit` | 167-186 | REAL | — | pins best-hit name passthrough |
| test `limit_error_reports_the_doc_ceiling` | 188-208 | REAL | — | pins exact Display + budget/attempted |
| test `confidence_and_hit_construction_stay_in_range` | 210-226 | REAL | S4 | F-003: misplaced plumbing test, pins nothing in this file |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| no_demuxer_carries_sniffed_bytes_and_hints | YES | stage/code/kind/name-absence/cap-16/source-chain all pinned |
| no_demuxer_names_the_best_subthreshold_hit | YES | name + sniffed bytes pinned |
| limit_error_reports_the_doc_ceiling | YES | exact Display string pinned — any wording/budget drift fails |
| confidence_and_hit_construction_stay_in_range | NO (for this file) | exercises report-module plumbing only (F-003) |
| Baseline execution | — | all 4 ok (nofailfast:3322-3325) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every probe failure is minted here" | error.rs:1-5 (module header) | REAL | repo grep: production mint sites all route through these constructors; the only raw `EngineError::new` sites are contract.rs's scripted test faults (cfg(any(test, feature="contract")), codes `probe-test.*`) — not probe failures |
| "never present a guess as fact … the first bytes that were actually sniffed (capped at 16 …)" | error.rs:15-19 | REAL | cap asserted at :158; sniffed passthrough asserted at :182 |

## 7. Dead / duplicated / unreachable code
- None: all 7 constructors have live mint sites (§2); `ProbeHints` is used only by `no_demuxer` and that is its purpose.

## 8. Dependencies & flags
- Depends on media-types taxonomy + `crate::config::MAX_LOOKAHEAD_CEILING` and `crate::report` types — intra-crate, no layering issue. No feature gates in this file; the `contract` feature (test-only) is documented in contract.rs and correctly absent from release binaries per its header.

## 9. Open questions for the next agent
- media-types owner: is there a taxonomy slot for "configuration value out of range" that `min_score_out_of_range` should use instead of `FormatInfo::name` (F-001)? If Wave-2's media-types audit already flagged the taxonomy gap, this is the same finding from the consumer side.
- Whoever audits media-probe/src/report.rs: confirm the range enforcement tests the F-003 comment points to actually exist there (codes `probe.format-id.invalid`/`probe.hint-codec-invalid` pinned at report.rs:384/419 suggest they do).
