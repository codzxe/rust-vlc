# AUDIT: crates/media-probe/src/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 228 / 12 (7 constructors + ProbeHints Display/Error + ok + 4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: all 7 constructors have live callers on the lumen-cli open path — `no_demuxer` (signature/mod.rs:207 via demux_host.rs:445), `lookahead_zero`/`lookahead_over_ceiling`/`min_score_out_of_range` (config.rs:54-60), `stream_not_rewindable` (lib.rs:199, the http lane), `hint_codec_invalid`/`format_id_invalid` (report.rs:141,234) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/media_probe-440f45029f776138 --test-threads=1 'error::'` -> 4 passed, exit 0; sed quote verifications; media-types kind_summary read (error.rs:310-328) to confirm Display rendering |

## 1. One-paragraph verdict
The typed-error mint is real, uniform (stage Probe + dotted codes), and fully
wired — every constructor has a reachable caller. The flagship
`no_demuxer` is honest: sniffed bytes clamped to 16 without a panic path,
the best sub-threshold hit's name carried, advisory hints riding the error
`source()` chain (verified end-to-end by probe_io.rs's
`extension_is_a_hint_not_truth`). Defect: two constructors abuse
`FormatInfo.name` — a semantic slot for FORMAT NAMES — to smuggle a
misconfigured minimum-score number and a URI scheme, producing Display
strings that read "unsupported in 101" and "unsupported in http"; plus a
dot-vs-dash code grammar inconsistency and one test that asserts the
report module's logic instead of this file's.

## 2. What actually works (verified)
— verified by live run `error::` -> "test result: ok. 4 passed", exit 0;
caller sweep by rg (all 7 constructors wired, listed in the header);
Display of the ResourceLimit path pinned to the exact string by
`limit_error_reports_the_doc_ceiling` (:203-207). The
`window[..window.len().min(16)]` clamp (:26) cannot panic on empty windows
(slice of 0..0).

## 3. Findings
F-001 [S3]
Where: crates/media-probe/src/error.rs:61-72 and 78-89
Code:
```
pub fn min_score_out_of_range(attempted: u8) -> EngineError {
    ...
            format: Some(FormatInfo {
                name: Some(attempted.to_string()),
                sniffed: Vec::new(),
            }),
```
(and identically `stream_not_rewindable(scheme)` at :82-86 with
`name: Some(scheme.to_owned())`)
What is wrong: `FormatInfo.name` is the format-name vocabulary slot; both
constructors stuff a config value ("101") or a URI scheme ("http") into it.
Via media-types' `kind_summary` (media-types/src/error.rs:315-319: `" in
{name}"`) the rendered errors are "[probe] unsupported in 101
(probe.min-score-out-of-range)" and "[probe] unsupported in http
(probe.stream-not-rewindable)" — nonsense sentences: the score 101 is not a
format that is unsupported, and http is a scheme whose SOURCE cannot rewind,
not an unsupported format. Any consumer keying on `format.name` (UI error
mapping, telemetry) receives garbage vocabulary. Why it matters: MP §8's own
bar is "never present a guess as fact" — "unsupported in 101" presents a
category error as fact. What correct looks like: `format: None` (the
context code already identifies the failure) with the attempted value in a
detail field, or an ErrorKind variant that carries the value.
Confidence: HIGH — Verification: read + Display rendering confirmed in
media-types error.rs:310-328.

F-002 [S4]
Where: crates/media-probe/src/error.rs:105
Code:
```
        ErrorContext::new("probe.format-id.invalid"),
```
What is wrong: code-grammar inconsistency — every sibling uses dashes inside
the final segment ("probe.min-score-out-of-range", "probe.hint-codec-invalid",
"probe.stream-not-rewindable") but this one embeds a second dot
("probe.format-id.invalid"), making "probe.format-id" parse as a segment.
Harmless to matching (codes are opaque strings) but breaks the visual
grammar the other six maintain.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/media-probe/src/error.rs:210-226
Code:
```
    #[test]
    fn confidence_and_hit_construction_stay_in_range() {
        // The error constructors take validated vocabulary; the report
        // module owns range enforcement (pinned there). This pins the
        // constructor plumbing compiles against the real types.
```
What is wrong: the test body exercises `Confidence::from_score(70)` /
`meets(60)` — report.rs logic, not error.rs; no error constructor is called.
Its own comment concedes it "pins the constructor plumbing compiles". A
regression in any error.rs constructor leaves this test green — it
contributes nothing to this file's coverage (the other three tests do cover
the constructors).
Confidence: HIGH — Verification: read + live run.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| no_demuxer | 20-35 | REAL | — | 16-byte sniff clamp safe on empty; hints chain verified e2e |
| lookahead_zero | 38-44 | REAL | — | wired config.rs:54 |
| lookahead_over_ceiling | 48-58 | REAL | — | ResourceLimit, budget = 1 MiB ceiling; Display pinned by test |
| min_score_out_of_range | 61-72 | REAL | S3 (F-001) | number in the format-name slot |
| stream_not_rewindable | 78-89 | REAL | S3 (F-001) | scheme in the format-name slot |
| hint_codec_invalid | 92-98 | REAL | — | wired report.rs:234 |
| format_id_invalid | 101-107 | REAL | S4 (F-002) | dot-in-segment code |
| ProbeHints Display / Error | 112-129 | REAL | — | "probe hints: extension=..., size=..." — matches probe_io assertions |
| tests (4) | 144-227 | REAL | S4 (F-003) | 3 honest + 1 wrong-module |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| no_demuxer_carries_sniffed_bytes_and_hints | YES | 300-byte window → 16-cap; stage/code/kind/source-chain pinned |
| no_demuxer_names_the_best_subthreshold_hit | YES | name + full sniffed bytes pinned |
| limit_error_reports_the_doc_ceiling | YES | budget/attempted + exact Display string |
| confidence_and_hit_construction_stay_in_range | NO (for this file) | tests report.rs; error.rs regression invisible |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every probe failure is minted here with stage Probe and a stable dotted context code" | error.rs:1-5 | REAL | all 7 constructors verified wired; stage pinned by tests |
| "the first bytes that were actually sniffed (capped at 16 by the L0 Display)" | error.rs:17-18 | REAL | :26 clamp + test :158 |

## 7. Dead / duplicated / unreachable code
None — all constructors wired (rg sweep in §header).

## 8. Dependencies & flags
media-types (taxonomy), crate::config (ceiling), crate::report. No unsafe,
no features.

## 9. Open questions for the next agent
- If media-types ever gains a `Config`/`Argument` ErrorKind, migrate F-001's
  two constructors off the Unsupported/format-name hack.
- crates/media-types/src/error.rs `kind_summary` truncates sniffed to 16
  twice (here and at construction) — harmless double clamp, note when that
  file is audited.
