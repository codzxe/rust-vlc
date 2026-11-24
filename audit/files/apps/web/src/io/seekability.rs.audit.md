# AUDIT: apps/web/src/io/seekability.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-24 |
| Lines / functions | 448 / 4 prod (2 methods, 2 fns) + 11 tests |
| File verdict | REAL (correct, executed) but UNREACHABLE — the fetch lane it serves is not wired into any shipped entry point |
| Reachable from shipped entry point? | NO — `detect_seekability` has zero production callers: only `io/staged_source.rs:482` (test-wired) and `io/mock_fetch.rs`. The shipped web open path REFUSES http/https (open.rs:182-186, "network streaming is card 7.H4"), so only `blob:` sources exist; `StagedSource` is constructed nowhere outside its own module/tests (rg over crates/ + apps/). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | cargo test -p lumen-web --offline io::seekability → 11 passed, exit 0; rg usage searches (exit 0); sed/od quote verification (exit 0) |

## 1. One-paragraph verdict
The detection logic itself is real and correct: a tight state machine over one probe response (206/200/416/3xx/other), a strict `Content-Range` parser with typed refusals, saturating arithmetic throughout (no overflow panics on hostile `u64::MAX` headers), start-agreement and body-vs-span checks, and an honest wire-end truncation rule. All 11 tests pass under my own targeted run. The problem is that none of it is reachable: the module doc claims the detection "feeds staged_source's mode and — through it — media.get-info's seekable field", but the shipped lumen-web open path answers every `http(s)` URL with a typed `unsupported-for-host` refusal naming http-streaming as future card-7.H4 work, and `StagedSource` (the only caller) is itself never constructed by production code. Secondary defects: `416` maps to `Empty` without checking the request's range start (wrong answer for any non-zero-start probe), and a garbage `Content-Length` is silently swallowed rather than refused.

## 2. What actually works (verified)
— `cargo test -p lumen-web --offline io::seekability` (my run): 11 passed; 0 failed, exit 0. Also baseline `audit/runs/baseline/cargo-test-workspace.txt:3324-3451` (lumen-web 111 passed, exit_code=0) includes all 11 seekability tests by name.
— Saturating-arithmetic audit of the 206 span math (lines 119-138): `bytes 0-u64MAX/u64MAX` with a short body degrades to a typed `Truncated`, no overflow panic (reasoned, not executed — marked LOW-confidence execution below).

## 3. Findings
F-001 [S3]
Where: apps/web/src/io/seekability.rs:26-29
Code:
```
//! The detection feeds [`crate::io::staged_source`]'s mode and —
//! through it — `media.get-info`'s `seekable` field and
//! `playback.seek`'s honest answer: unseekable sources report so,
//! and live-style forward-only playback rules apply (doc 09 §6.1).
```
What is wrong: the claimed consequence chain does not exist in the shipped build. `rg` over `crates/` and `apps/` finds `detect_seekability`/`StagedSource` referenced only inside `apps/web/src/io/` (staged_source.rs:482, mock_fetch.rs, self). The worker's open path refuses `http`/`https` at apps/web/src/open.rs:182-186:
```
                "http" | "https" => Err(unsupported_for_host(
                    scheme,
                    "network streaming is card 7.H4 (fetch + Range seekability detection)",
                )
                .with_detail("capability", "http-streaming")),
```
so no media fetch ever happens; `blob:` sources never run detection. Why it matters: the whole lane (fetch_port, staged_source, seekability, mock_fetch — 4 modules) is present, exported (`pub mod io;`), and tested, but dead in the artifact; the doc presents it as wired. What correct looks like: mark the module doc `[PROPOSAL]/unwired`, or wire the lane. Confidence: HIGH — Verification: ran (rg callers + cargo test) + read open.rs.
F-002 [S3]
Where: apps/web/src/io/seekability.rs:163
Code:
```
        416 => Ok(Seekability::Empty),
```
What is wrong: unconditional. A 416 means "range unsatisfiable", which is `Empty` only when the request started at 0; for a probe with `start > 0` a 416 means the resource is shorter than `start` (it exists, non-empty). The function receives the request (with its range) and never checks `range.start == 0`. Why it matters: the function is a public API whose doc (lines 56-58) says "The probe's 416 at offset 0", but nothing enforces offset 0; the current sole caller only feeds it the initial 0-start probe and routes mid-stream 416s elsewhere (staged_source.rs:520-527 catch-all), so the wrong answer is produced but unconsumed today — a landmine for the next caller. What correct looks like: `if range.map_or(true, |r| r.start == 0) { Ok(Empty) } else { Err(ProtocolViolation …) }`. Confidence: HIGH — Verification: read (both call-site paths).
F-003 [S4]
Where: apps/web/src/io/seekability.rs:146-148
Code:
```
            let declared = response
                .header("Content-Length")
                .and_then(|value| value.trim().parse::<u64>().ok());
```
What is wrong: a malformed `Content-Length` ("12x4") parses to `None` and is silently treated as absent — the delivered body length becomes the answer. Outcome is still factually correct, but the module's own posture ("the same typed protocol-violation posture ranged.rs keeps", line 17) is not kept for the malformed-header case, and no test covers it (the test at 382-395 covers a lying-but-parseable length only). Confidence: HIGH — Verification: read.
F-004 [S4]
Where: apps/web/src/io/seekability.rs:174-176, 184-188
Code:
```
/// Parse `Content-Range: bytes <start>-<end>/<total|*>` (the mirror
/// of `ranged.rs`'s parser, restated because streaming's is private
/// — same grammar, same typed refusals).
```
What is wrong: deliberate duplication of `crates/streaming/src/ranged.rs:953` `parse_content_range` (same `(u64, u64, Option<u64>)` signature, verified present). The rationale (private upstream) is documented, but two copies of a wire-format parser can drift. Also: `strip_prefix("bytes")` accepts `bytes0-99/100` (no separating space) and is case-sensitive. Confidence: HIGH — Verification: read + rg of ranged.rs.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Seekability (enum) | 39-60 | REAL | — | 3 outcome variants, total/length as Option (value vs absence typed) |
| Seekability::is_seekable | 63-69 | REAL | — | Ranged ⇒ true (incl. unknown total) — matches doc |
| Seekability::length | 71-79 | REAL | — | Ranged total / Unseekable length / Empty ⇒ Some(0) |
| detect_seekability | 93-172 | REAL | S3(F-001 unreachable lane, F-002 416) | 206: rangeless/missing-header/start-disagreement/oversized/short-body checks all typed; 200: length cross-check; 3xx surfaced ⇒ breach; other ⇒ HttpStatus |
| parse_content_range | 180-215 | REAL | S4(F-004) | strict grammar, end<start refused, `*` total ⇒ None |
| tests (11) | 250-447 | REAL | — | all pass (my run); see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| accept_ranges_206_yields_ranged_seekable | YES | asserts exact variant + total + is_seekable + length |
| unknown_length_total_yields_ranged_without_total | YES | `bytes 0-1023/*` ⇒ Ranged{None}, length None |
| no_range_200_yields_unseekable_with_length | YES | exact variant, seekable false |
| truncated_206_body_is_the_typed_truncation | YES | asserts Truncated{expected:1024, got:512} AND the wire-end exception (0-1499/1500, 1500 bytes) passes |
| surfaced_redirect_is_the_typed_transport_breach | YES | 302 ⇒ ProtocolViolation |
| range_not_satisfiable_is_the_empty_resource | YES | 416 ⇒ Empty, length Some(0) — but only offset-0 shape covered (F-002 uncovered) |
| garbage_206_is_the_protocol_violation | YES | 5 malformed Content-Range shapes |
| lying_content_length_is_the_protocol_violation | YES | 9999 vs 128 ⇒ violation; missing header ⇒ delivered length (garbage-header case NOT covered — F-003) |
| uninterpreted_statuses_are_typed | YES | 404/500/503 ⇒ HttpStatus{status} |
| disagreeing_or_oversized_206_refuses_typed | YES | start=1 mismatch; body 1024 > span 512 |
| rangeless_request_with_206_refuses_typed | YES | request without range + 206 ⇒ violation |
Not covered: non-zero-start 416 (F-002), garbage Content-Length (F-003), duplicate Content-Range headers, case-variant "Bytes" unit.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seekability is detected, not assumed" (doc 09 §6.1) | seekability.rs:1-2 | REAL (logic) / UNREACHABLE (wiring) | detection code + tests pass; no shipped caller (F-001) |
| "The detection feeds staged_source's mode and — through it — media.get-info's seekable field" | seekability.rs:26-29 | FALSE for the shipped build | open.rs:182-186 refuses http/https; StagedSource never constructed in production (rg) |
| "Accept-Ranges is advisory data only … the ranged GET is the proof" | seekability.rs:19-22 | REAL | no Accept-Ranges read anywhere in detect (rg: only the doc mentions it) |
| "Redirects are followed by the transport BEFORE detection runs" | seekability.rs:30-32 | REAL-UNPROVEN | consistent with 3xx⇒breach arm; transport behavior lives in mock_fetch.rs/browser (not this file, unexecuted) |
| mirror of ranged.rs | seekability.rs:2-3, 174-176 | REAL | crates/streaming/src/ranged.rs:953 same parser signature/posture |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (F-001). Duplicated Content-Range parser (F-004, documented). No other dead code.

## 8. Dependencies & flags
Only `crate::io::fetch_port` types + `testkit` (Must/assert_err) in tests. No feature gates. NOTE for quoting discipline: the literal `#[must_use]` renders as `#ust_use]` in this sandbox's Bash output channel (od-verified: real bytes are `#[must_use]`; probe at audit/runs/probes/attr_probe.rs) — quotes in this audit were od/sed cross-checked and contain no munged sequences.

## 9. Open questions for the next agent
- Will card 7.H4 actually wire the fetch lane (open.rs http refusal removal)? Until then the whole io/ module stays dead weight in the wasm artifact (size-budget relevance).
- staged_source.rs and mock_fetch.rs belong to other agents: confirm whether StagedSource::open has any caller outside tests (my rg says no).
