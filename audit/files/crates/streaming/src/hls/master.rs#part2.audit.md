# AUDIT: crates/streaming/src/hls/master.rs#part2 (lines 596–1189)
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-21 |
| Lines / functions | 594 audited (chunk part2 of 1189; part1 = 1-595 is W10-A07's — constants, AbrConfig/Controller, HlsMasterConfig, stats structs, HlsAdaptiveStream head + load_variant_plan + switch_variant head; `fetch_next_segment` (555-599) crosses the 596 boundary — audited whole here with this note) / 10 production fns (fetch_next_segment, absorb, serve, 6 SourceStream methods, open_master) + 6 test helpers + 8 tests + RouterFetch test double |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. Caller census: `open_master`/`HlsAdaptiveStream` appear ONLY in master.rs's own tests and `crates/streaming/tests/hls_master_e2e.rs` (rg across repo). The shipped CLI's http lane is raw-bytes only: apps/cli/src/demux_host.rs:427 `"http" \| "https" => open_http_source` → `RangedResource` (no m3u8 dispatch); media-probe has no m3u8 signature (rg in crates/media-probe/src → 0 hits); apps/cli/src/dispatch.rs:191-192 refuses `playlist.*` commands ("the playlist model is a Phase-4 card"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | `grep -n` structure map (all fn pins); `sed -n` verbatim verification of every quote; `rg "open_master\|HlsAdaptiveStream\|AbrController\|AbrConfig"` (reachability census — tests only); `rg "hls\|m3u8"` in apps/ + crates/media-probe/src (no HLS lane shipped); baseline: `orchrun-test-workspace-nofailfast.log.txt` :5044-5051 (8/8 master tests ok), :5203-5210 (hls_master_e2e 3/3 ok); `sed` of feature-matrix.toml network-hls row :1431-1441 |

## Chunk boundary note
Part2 = 596–1189: tail of `fetch_next_segment` (whole fn audited; its head 555-595 is part1's), `absorb`, `serve`, `impl SourceStream` (639-690), `open_master` (704-770), and the entire `mod tests` (772-1189).

## 1. One-paragraph verdict
This chunk is the adaptive session's serving half plus its whole test suite. The serving logic is sound on every path I traced: the retained-prefix invariant (`pos ∈ [retained_from, fetched_upto]`, retained either empty-with-`retained_from == fetched_upto` or covering through `fetched_upto`) holds across absorb's drop rule, seeks, and EOF, so `serve`'s index math cannot underflow; timeline alignment by absolute media sequence is correctly refused typed when a variant window lacks the expected sequence. The 8 tests are honest — byte-pattern probes per variant, seam-continuity windows, exact request counts — and all executed green in the baseline. Three real defects: a failed switch permanently desyncs the ABR controller from the session (decide() commits before switch_variant validates, so a refused switch leaves a phantom current-variant that a retry then "switches" back, recording a phantom discontinuity seam); `HlsMasterStats.bytes_fetched`'s documented "sum of all response body bytes" silently excludes every variant-playlist body (`_bytes` discarded at :492); and `read()` silently clamps to 64 MiB where the crate's own `RangedResource` refuses the same input typed. None of it is reachable from a shipped binary — the CLI has no HLS lane at all, so the feature-matrix `network-hls` "partial" status rests entirely on library tests, not product behavior.

## 2. What actually works (verified)
— 8/8 in-file tests + 3/3 `hls_master_e2e` executed ok: baseline `orchrun-test-workspace-nofailfast.log.txt:5044-5051` and `:5203-5210`.
— Read-traced `serve` (620-636) against `absorb` (603-617): the drop rule sets `retained_from = start` (610) with the new body retained, or `retained_from = fetched_upto` with retained empty (614); `seek` (661-685) admits only `[retained_from, fetched_upto]`; therefore `at = pos - retained_from` and `have = retained.len() - at` can never underflow. No panic path on valid input in this chunk.
— Read-traced `adaptive_session_switches_on_scripted_bandwidth` (988-1064): EWMA arithmetic (observe_throughput 199-211: 3/4+1/4 shift merge) reproduces each asserted transition — hold at 1 Mbps on hi, hi→mid when EWMA (1.525 Mbps after 250→190.6 B/ms) crosses hi's 80% line (1.6 Mbps), starve to lo, recover to mid after dwell — and the seam assertions (media_sequence +1 across every `windows(2)`, 12 boundaries, ≥2 switches, size()==Some(total)) pin the timeline end to end.
— `switch_to_misaligned_variant_refuses` (1096-1129) proves the alignment refusal is real: lo's window (MEDIA-SEQUENCE 106) lacks mid's next expected sequence → `streaming.hls.variant-sequence-missing` (minted 535-542).

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/hls/master.rs:560-564 (with 303-312 commit and 543-551)
Code:
```
        if self.next > 0
            && let Some(target) = self.abr.decide(&self.master.variants)
        {
            self.switch_variant(target)?;
        }
```
What is wrong: `AbrController::decide` (part1, 263-300) commits the switch internally (`commit` 303-312: `current = target`, dwell reset, `switch_count += 1`, history push) BEFORE `switch_variant` runs; `switch_variant` can then fail — `load_variant_plan` transport error, or the typed `streaming.hls.variant-sequence-missing` at 535-542 — after which the error returns BEFORE `self.plan = plan` / `self.stats.variant_switches += 1` / `current_bandwidth_bps` update (547-550). Post-error state: `abr.current == target` (a variant never served), `abr.snapshot().switch_count` incremented while `stats.variant_switches` is not — two counters that mirror each other everywhere else now disagree. On a retry of the read (a normal `SourceStream` consumer posture after a transient error), `decide` runs from the phantom rung: starvation/up-step math is keyed to the wrong variant's bandwidth, and a recovered throughput produces an up-"switch" back to the variant already being served — a real playlist re-fetch plus a **phantom discontinuity seam** (`plan[0].discontinuity = true`, 546) recorded into `segment_boundaries` for a seam where the encoding never changed.
Why it matters: the seam record is the B1b contract downstream decoders key on ("nothing silently decoded across a seam" — module docs :52-55); a phantom seam mislabels continuous media, and the stats/snapshot disagreement corrupts any metrics consumer. The error path is exactly the realistic case: flaky network mid-session.
What correct looks like: validate the switch (fetch + align) before committing ABR state — e.g. `decide` returns a proposal; the session commits via an explicit `abr.commit(target)` only after `switch_variant` succeeds; or `switch_variant` rolls the controller back on failure.
Confidence: HIGH — Verification: read (state traced on both the success and failure paths); retry scenario reasoned, not executed.

### F-002 [S3]
Where: crates/streaming/src/hls/master.rs:491-495 (with 396-397, 581-582, 757)
Code:
```
    fn load_variant_plan(&mut self, url: &str) -> Result<Vec<PlannedSegment>, EngineError> {
        let (model, _timestamp_map, _bytes) =
            fetch_playlist_model(&self.fetch, url, self.config.playlist_response_limit)?;
        self.stats.requests += 1;
        self.stats.playlists_fetched += 1;
```
What is wrong: `HlsMasterStats::bytes_fetched` is documented (396-397) as "Sum of all response body bytes received", but the variant-playlist body length — returned by `fetch_playlist_model` and bound to `_bytes` — is discarded. Only the master body (open_master, 757) and segment bodies (fetch_next_segment, 582) are counted. Every variant playlist fetched at open and at every switch (which the ABR can do repeatedly) is missing from the sum.
Why it matters: the field is the crate's own throughput/accounting surface ("tests, logging, metrics", 387); any consumer computing fetch efficiency (bytes_fetched vs served) gets a systematic undercount that grows with switch count. `requests` counts the playlist fetch (494) while `bytes_fetched` ignores its body — internally inconsistent accounting in the same function.
What correct looks like: `self.stats.bytes_fetched += body_len;` in `load_variant_plan` (the helper already returns the byte count), or re-document the field as "master + segment body bytes".
Confidence: HIGH — Verification: read; all three increment sites located (491-495, 581-582, 755-757).

### F-003 [S3]
Where: crates/streaming/src/hls/master.rs:657-659 (with crates/streaming/src/ranged.rs:861-870, crates/streaming/src/hls/media.rs:422-424)
Code:
```
    fn read(&mut self, len: usize) -> Result<Vec<u8>, EngineError> {
        self.serve(len.min(MAX_READ_BYTES))
    }
```
What is wrong: A read above the 64 MiB payload ceiling is silently clamped to a short read. The same crate's `RangedResource::read` (ranged.rs:861-870) refuses the identical input with the typed `streaming.ranged.read-over-budget` (ResourceLimit), mirroring the input crate's ceiling. `HlsVodSession` (media.rs:422-424) clamps too — so the crate ships two `SourceStream` impls that answer the same oversized read with different semantics (typed error vs silent short read), and neither site documents the choice.
Why it matters: a demuxer or bench that legitimately asks for >64 MiB gets a short-but-nonzero read here and a hard error there — behavior drift across impls of one trait in one crate, invisible until a consumer depends on either answer. Silent clamps are exactly what this codebase's own "typed refusal, never a clamp (MP §5)" posture forbids for budgets.
What correct looks like: one policy crate-wide — either both refuse typed (`streaming.hls.read-over-budget` minted here) or both clamp, with the shared rule stated in the `SourceStream` implementor docs.
Confidence: HIGH — Verification: read of all three impls (media.rs part1 is this agent's next file; quote re-verified there).

### F-004 [S4]
Where: crates/streaming/src/hls/master.rs:923-924
Code:
```
            // Exactly-at-margin does not switch (>= is required; the
            // EWMA sits just under from the earlier low samples).
```
What is wrong: The first clause contradicts the code it sits on. Headroom is `throughput * 100 >= next_bw * (100 + up_margin_percent)` (291) — `>=` means a throughput EXACTLY at the 1 Mbps margin (800k × 1.25) DOES satisfy headroom and would switch. The reason this loop holds is only the second clause: the EWMA (≈95 B/ms after the low samples) is still below the 125 B/ms threshold. The comment teaches the wrong rule.
Why it matters: A reader tuning `up_margin_percent` or writing the next ABR test will believe at-margin is a hold; the arithmetic says it is a switch.
What correct looks like: "At-margin would switch (>=); the EWMA sits just under the 125 B/ms threshold from the earlier low samples, so this holds."
Confidence: HIGH — Verification: read + arithmetic check of 291 (1e6·100 ≥ 8e5·125 → 1e8 ≥ 1e8, true).

### F-005 [S4]
Where: crates/streaming/src/hls/master.rs:697-703 (doc) vs 730-739 (code)
Code:
```
/// [`EngineError`] (stage `Open`): transport failures mapped from
/// [`NetworkError`], master parse refusals
/// (`streaming.hls.not-hls` / `.media-playlist` / `.parse-failed`),
```
What is wrong: The `#Errors` list omits `streaming.hls.bad-tag-value`, which `open_master` mints (734: `BadTagValue { .. } => "streaming.hls.bad-tag-value"`), and `streaming.hls.no-variant` is listed but `playlist-not-utf8` appears only in the following line — the enumeration is incomplete relative to the match arms two screens below.
Why it matters: The error-contract doc is what callers pin tests against; a missing code means a caller can't pre-handle it.
What correct looks like: List all five parse-path codes (`not-hls`, `media-playlist`, `bad-tag-value`, `parse-failed`, `playlist-not-utf8`) plus `no-variant`.
Confidence: HIGH — Verification: read; match arms at 730-739 enumerate exactly the codes.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HlsAdaptiveStream::fetch_next_segment (BOUNDARY, audited whole) | 555-599 | REAL (test-verified) | S3 (F-001) | Boundary ABR consult → switch → fetch → absorb → bookkeeping. decide-before-validate desync (F-001). HttpStatus remap (571-577) honest. No panic path (plan[next] guarded by serve). |
| HlsAdaptiveStream::absorb | 603-617 | REAL | — | Budget drop rule; empty-retained edge sets retained_from = fetched_upto. Invariant traced sound (see §2). |
| HlsAdaptiveStream::serve | 620-636 | REAL | — | Loop fetches until len or EOF; index math safe under the traced invariant; clean sticky EOF via `ended`. |
| SourceStream::uri / position | 640-642 / 687-689 | REAL | — | Trivial accessors. |
| SourceStream::size | 644-649 | REAL | — | None until drained (honest unknown-length); `ended && next >= plan.len()` second clause redundant (ended implies it) — harmless. |
| SourceStream::supports_seek | 651-655 | REAL | — | `true` — within retained prefix; honest given seek's typed refusals. |
| SourceStream::read | 657-659 | REAL | S3 (F-003) | Silent clamp vs ranged's typed refusal. |
| SourceStream::seek | 661-685 | REAL | — | Backward below retained_from and forward past fetched_upto both refuse typed; range math via shared resolve_seek_origin (media.rs:375). |
| open_master | 704-770 | REAL | S4 (F-005) | URL parse → master fetch → UTF-8 → typed parse-code mapping → selection → variant plan load. Stats seeded honestly (requests=1, bytes=master body). UNREACHABLE from shipped binaries. |
| tests::RouterFetch + router | 783-814 | REAL | — | In-test URL-path router over the real port; 404 for unknown routes. No range/limit emulation — fine for a unit double (mockhttp covers transport semantics). |
| tests::master_text / variant_playlist / routes_for / owned_routes | 816-868 | REAL | — | Fixture builders; `owned_routes` `Box::leak`s per test (bounded, test-only). |
| tests::selection_is_highest_under_cap | 873-888 | REAL | — | Pins inclusive cap, tie order, all-over-cap → None. Mutation-sensitive. |
| tests::abr_hysteresis_holds_inside_the_band | 895-935 | REAL | S4 (F-004) | EWMA traced by hand; holds/down/exact-margin hold/up all asserted. Comment defect only. |
| tests::abr_stall_prediction_forces_down | 940-955 | REAL | — | Stall bypasses dwell (:281), cleared + dwell-0 holds. Correct per decide(). |
| tests::abr_history_is_bounded | 961-980 | REAL | — | 48 iterations, switch_count ≥ CAP, `history.len() ≤ CAP` (private-field access, same module). Bounded-ledger proof. |
| tests::adaptive_session_switches_on_scripted_bandwidth | 988-1064 | REAL | — | The strongest test: full ladder cycle with seam continuity, counts, size. Traced (§2). Would fail on alignment, EWMA, or accounting regressions. |
| tests::adaptive_refusals_are_typed | 1069-1091 | REAL | — | no-variant + media-playlist codes pinned. |
| tests::switch_to_misaligned_variant_refuses | 1096-1129 | REAL | — | Alignment refusal end-to-end; narrative comment's sequence arithmetic slightly off (says mid serves 102..104 then expects 105; trace: after hi 100/101 mid serves 102, expects 103) — assertion itself correct. |
| tests::in_playlist_discontinuity_is_recorded_without_switching | 1138-1189 | REAL | — | In-playlist EXT-X-DISCONTINUITY recorded without a switch; discontinuities==1, switches==0, size()==Some(200). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 8 | YES (each) | Assertions are against external truth: per-variant byte patterns ('H'/'M'/'L'), boundary-window continuity, exact counters. Mutation reasoning: gut `switch_variant`'s alignment search (always start 0) → seam continuity asserts fire (sequences jump); drop dwell check in decide → adaptive test flaps and `current` assertions fail; remove discontinuity recording → both discontinuity tests fail. |
| abr_history_is_bounded | YES | Direct private-field bound check `abr.history.len() <= SWITCH_HISTORY_CAP`. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bytes_fetched: Sum of all response body bytes received" | master.rs:396-397 | FALSE | F-002: variant-playlist bodies excluded (`_bytes` discarded :492). |
| "6.B2b boundary variant switching + absolute-sequence alignment + ABR hysteresis band/dwell" (evidence for network-hls status `partial` win/linux/mac) | docs/feature-matrix.toml:1437-1440 | REAL-UNPROVEN / UNREACHABLE from product | Code exists and 8+3 tests ran green (baseline :5044-5051, :5203-5210) — but no shipped binary reaches it (caller census §Reachable). "partial" status reflects library maturity, not product capability; `lumen open x.m3u8` lands in the raw RangedResource lane. |
| "VOD variants only, honestly: every variant playlist must pass the B1a playability table" | master.rs module docs :59-61 | REAL | load_variant_plan refusals 496-510 (live-playlist, empty, encrypted, byterange, init-segment); pinned by tests in part1's suite + adaptive_refusals. |
| "#Errors … not-hls / media-playlist / parse-failed" | master.rs:697-703 | INCOMPLETE | F-005: omits bad-tag-value. |

## 7. Dead / duplicated / unreachable code
- The ENTIRE production half of this chunk (`HlsAdaptiveStream` serving, `open_master`) is unreachable from shipped entry points (see header). Not dead — it is the intended Phase-6 lane — but as shipped it is library-plus-tests only.
- `AbrController::current()` (part1, 233-236) has no caller anywhere (snapshot() serves the same data) — noted for part1's owner.
- No duplication within the chunk; `absorb`'s budget rule duplicates media.rs's session logic by stated design ("the budget rule shared with the VOD/live sessions").

## 8. Dependencies & flags
Uses `super::media` helpers (refuse/mint/map_fetch_err/resolve_seek_origin/fetch_playlist_model/MAX_READ_BYTES/retained constants) and `crate::buffer::BufferSignal`, `crate::fetch::{DEFAULT_RESPONSE_LIMIT, HttpFetch, HttpRequest}`, `playlists::{MasterPlaylist, Variant}`. No feature gates. No unsafe. Tests use `testkit::Must` and `Box::leak` fixtures (test-only).

## 9. Open questions for the next agent
- W10-A07 (master.rs part1): does `AbrController::decide`'s commit-inside-decide design have any caller that relies on it (i.e., is F-001's fix breaking any contract)? Also `current()` dead-accessor check.
- Matrix-rows owner for `network-hls`: adjudicate whether "partial" is honest given zero product reachability of the whole hls module (this file proves it for the master/ABR half; media/live/timestamp_map owners should confirm theirs).
- Whoever owns the engine retry guard: confirm reads are re-issued after a typed SourceStream error (F-001's precondition).
