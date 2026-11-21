# AUDIT: crates/streaming/src/hls/live.rs#part3 (lines 983-1473)
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2026-11-21 |
| Lines / functions | 491 (983-1473) / 13 (12 complete `#[test]` fns + the tail of `endlist_terminates_and_fixes_the_total` which straddles the boundary). Part boundaries: part1 (W10-A02) = 1-491 (module docs, `HlsLiveConfig`/stats, `HlsLiveStream` methods incl. `reload_wait_ms` head); part2 (W10-A07) = 492-982 (`reload` body tail, `SourceStream` impl, `open_live`, test harness `LiveRouter`/`drain`/`read_exactly`/`fast_config`, first three tests); **part3 (MINE) = 983-1473 — the behavioral suite from `stalled_reloads_fail_typed_live_timeout` to the end** (the `endlist` test starts at 971 in part2's range; its assertion tail 983-1001 is mine, covered for coherence). |
| File verdict | UNREACHABLE (tests REAL and executed green incl. a real-socket e2e; the live session they cover has no shipped caller) |
| Reachable from shipped entry point? | NO. `rg -n "open_live\|HlsLiveStream" crates apps` → `open_live` is invoked only by this file's own tests (live.rs:901…1466) and `tests/hls_live_e2e.rs`; `HlsLiveStream` is referenced nowhere else. The shipped CLI's HTTP lane (`apps/cli/src/demux_host.rs:474-492`) routes every http(s) URL through `RangedResource`/`SchemeRegistry` — no m3u8 sniffing, no HLS session construction. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "open_live\|HlsLiveStream" crates/streaming/src apps` → test-only callers; `rg -ln "open_live" tests crates/streaming/tests` → hls_live_e2e.rs, hls_conformance_e2e.rs, net_interrupted.rs; `sed -n '476,606p'` → reload/pacing logic cross-check for every cadence assertion; `sed -n '100,214p'` → config default (pacing_clock None) + validate; `rg -n "test hls::live::tests::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 15/15 ok at 5029-5043; `sed -n '5195,5202p'` same log → e2e 2/2 over a real socket; streaming lib suite `212 passed; 2 ignored` |

## 1. One-paragraph verdict
The live-window test suite is precise, hostile, and pinned to typed codes: stall-budget exhaustion asserts the exact `ResourceLimit{budget:2, attempted:3}` shape, the RFC 8216 pacing tests assert the exact sleep vector `[2000, 1000, 1000]` and the advanced virtual time, gap/regression windows refuse with exact codes and honest stats, the refusal table pins six open-time codes plus the discontinuity-tolerated counter-case, and the X-TIMESTAMP-MAP test proves parse → live-edge derivation (LOCAL + window EXTINF total) → per-reload supersession across an encoder-restart anchor jump. It ran 15/15 green plus 2/2 over a real loopback socket in the baseline's no-fail-fast run. The defects are placement and residue: the entire live session is a library island no shipped host ever opens (the CLI never routes an m3u8 URL to it, so the matrix's `network-hls: partial` is code truth, not user truth), one test carries a debug-monologue comment instead of documentation, one assertion is a `>=` where every sibling pins exact counts, and the reload-pacing fallback for playlists without TARGETDURATION — a real dirty-server branch — is executed by no test in the file.

## 2. What actually works (verified)
— All 15 `hls::live::tests` green: `orchrun-test-workspace-nofailfast.log.txt:5029-5043` (every test in this part plus the 3 part2-range ones); streaming lib suite `212 passed; 0 failed; 2 ignored` (log, streaming section).
— Real-socket e2e green: same log 5195-5202 — `live_slides_endlist_and_discontinuity_over_a_real_socket … ok`, `stalled_live_source_fails_typed_live_timeout … ok` (mockhttp loopback server, not a mock transport object).
— Pacing math cross-verified against production code: `reload_wait_ms` (live.rs:476-494: full target-duration, half after a stall) × `reload`'s wait-then-fetch-then-reanchor (506-526) reproduces exactly the asserted `[2000, 1000, 1000]` / now_ms=4000 / paced_reloads=3 under FakeClock-from-zero.

## 3. Findings

### F-001 [S3] The live-session suite (and its e2e lane) guards a session no shipped binary can open
Where: crates/streaming/src/hls/live.rs:983-1473 (suite) ↔ crates/streaming/src/hls/live.rs:682-686
Code:
```rust
pub fn open_live(
    fetch: Arc<dyn HttpFetch>,
    url: &str,
    config: HlsLiveConfig,
) -> Result<HlsLiveStream, EngineError> {
```
What is wrong: `open_live` has zero production callers. The CLI's `open_http_source` (demux_host.rs:474-492) passes every http(s) URL to `RangedResource` as a generic byte source; nothing sniffs `#EXTM3U` or an `.m3u8` suffix to construct a live (or VOD/master) session. The matrix row `network-hls` (feature-matrix.toml:1431-1443) claims `partial` on all desktop platforms citing this code and its e2e runs.
Why it matters: a user running `lumen open http://server/live.m3u8` gets a demux-probe failure on playlist text, not live playback — while the repo carries a fully-tested live session with pacing, gap refusal, and timestamp mapping. The row's evidence sentence is true; its `partial` status reads as user capability that does not exist (the wave-10 hostile question answered for the HLS lane).
What correct looks like: a shipped route (URL/content sniff in the open path → `open_live`/VOD/master session) or a matrix status/notes line saying "library-complete, no shipped host routes it".
Confidence: HIGH — Verification: rg census (header) + read the CLI open path.

### F-002 [S4] Debug monologue left in the gap test's fixture comment
Where: crates/streaming/src/hls/live.rs:1126-1128
Code:
```rust
        // The window jumps to base 3: seg1 (fetched), seg2 (fetched),
        // and seq 2.5 never existed — but base 3 > next cursor 2? No:
        // cursor is 3 after seg2 — jump to base 4 instead.
```
What is wrong: the comment narrates the author's abandoned first attempt ("base 3? No: … jump to base 4 instead") — including a nonexistent "seq 2.5" — instead of documenting the fixture: window base jumps from 0 to 4 while the fetch cursor is at 3, so sequence 3 slides out unfetched.
Why it matters: the reader must re-derive the arithmetic to trust the test; the confusion text actively misdirects (the fixture never uses base 3).
What correct looks like: one sentence: "w2 jumps MEDIA-SEQUENCE to 4 while the cursor sits at 3 — seq 3 slid out unfetched; the next dry read must refuse at the gap."
Confidence: HIGH — Verification: read the whole test (1123-1165) and hand-traced the sequences.

### F-003 [S4] `timestamp_map` test uses a `>=` fetch count where every sibling pins exact counts
Where: crates/streaming/src/hls/live.rs:1445
Code:
```rust
        assert_eq!(more, b"DDD", "seg3 arrives through the W2 window");
        assert!(fetch.playlist_fetches() >= 2, "the reload happened");
```
What is wrong: every other test in the module asserts the exact playlist-fetch count (`assert_eq!(fetch.playlist_fetches(), 4, "open + 3 reloads")` at 1032, exact reload counts at 953-958, 996, 1117); this one accepts any count ≥ 2. A regression that introduced a spurious extra reload (e.g. an off-by-one in the dry-window detection causing a double reload before seg3 arrives) passes unnoticed here.
Why it matters: the fetch count is the demand-driven reload model's core observable; one unpinned site is one blind spot in an otherwise exact suite.
What correct looks like: `assert_eq!(fetch.playlist_fetches(), 2, "open + exactly one reload");` (the fixture w1→w2 slide yields exactly one).
Confidence: HIGH — Verification: traced the fixture (2 playlist bodies, one dry-point) — the exact count is 2.

### F-004 [S4] The no-TARGETDURATION pacing fallback is executed by no test
Where: crates/streaming/src/hls/live.rs:480-486 (production, part1's range — coverage gap proven from my part3 range)
Code:
```rust
            .unwrap_or_else(|| {
                self.pending
                    .iter()
                    .filter_map(|segment| segment.duration)
                    .fold(0u64, |acc, duration| acc.max(duration.ceil() as u64))
                    .max(1)
            })
```
What is wrong: every pacing-test playlist carries `#EXT-X-TARGETDURATION` (1003-1004, 1045-1046, 1090-1093), and the only TARGETDURATION-less playlists in the module (the refusal table, 1248-1272) refuse at open or never read — so the fallback branch (max pending EXTINF, floored at 1 s) never runs under test. A mutation deleting `.max(1)` (zero wait → busy reload) or inverting the fold survives the suite.
Why it matters: dirty servers omitting the required TARGETDURATION tag are exactly the tolerance class this crate courts; the branch that protects them is unproven.
What correct looks like: one paced test with a TARGETDURATION-less window asserting the fallback cadence (max EXTINF seconds, floor 1 s).
Confidence: HIGH — Verification: enumerated every playlist literal in the test module (767-1473) and their open/read outcomes.

## 4. Per-function table (part3 only — all tests)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| test `endlist_terminates_and_fixes_the_total` (tail; starts 971 in part2) | 983-1001 | REAL | — | Exact byte drain, post-ENDLIST empty read, `size()==Some(10)` exact total, reload count 1. |
| test `stalled_reloads_fail_typed_live_timeout` | 1002-1034 | REAL | — | Pins `streaming.hls.live-timeout` + `ResourceLimit{RecoveryAttempts, budget:2, attempted:3}`; stall/reload/fetch counts exact; paced_reloads=0 without clock (documents the unpaced default). |
| test `paced_reloads_wait_the_rfc8216_cadence` | 1043-1081 | REAL | — | Sleep vector `[2000,1000,1000]`, virtual time 4000, paced 3, budget unchanged — the strongest cadence proof in the crate. |
| test `paced_productive_reload_waits_the_full_cadence` | 1088-1118 | REAL | — | Productive full-cadence wait `[1000]`, stall reset. |
| test `slid_out_segments_refuse_typed_at_the_gap` | 1123-1165 | REAL | S4 | Behavior exact (gap code + `segments_lost_window==1` + `size()==None`); F-002 comment residue. |
| test `window_regression_refuses_typed` | 1170-1200 | REAL | — | Backwards base → `streaming.hls.window-regression` + Corrupt kind. |
| test `discontinuity_is_recorded_across_the_seam` | 1205-1233 | REAL | — | Byte-exact concat + boundary flags (offset 6, duration 2.0, discontinuity) + stats. |
| test `live_refusal_table_is_typed` | 1239-1273 | REAL | — | Six exact open-time codes + the tolerated discontinuity counter-case. |
| test `encryption_arriving_on_reload_refuses` | 1279-1304 | REAL | — | Mid-stream encryption: prefix plays, reload refuses typed. |
| test `live_seek_is_honest` | 1310-1367 | REAL | — | Free in-prefix rewind (requests unchanged — pinned), unfetched-forward and dropped-head refusals with exact codes. |
| test `live_config_bounds_refuse_typed` | 1371-1392 | REAL | — | Zero response limit and over-ceiling retention both `streaming.hls.config`. |
| test `timestamp_map_parses_applies_and_supersedes_per_reload` | 1398-1456 | REAL | S4 | Full 6.B3 surface incl. anchor supersession and unaffected byte stream; F-003 weak count. |
| test `timestamp_map_absent_is_none_not_a_guess` | 1461-1472 | REAL | — | None on both surfaces. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| stalled/paced/productive trio | YES | Exact sleep vectors, virtual time, budget kinds, and request counts — any pacing or budget mutation fails an equality. |
| slid_out / window_regression / encryption_reload / refusal_table / config_bounds | YES | Exact codes + kinds + stats; the refusal table also pins the negative (discontinuity opens). |
| live_seek_is_honest | YES | Seek policy's three edges each pinned, including the no-extra-requests rewind proof. |
| timestamp_map pair | YES for mapping math (exact ms derivations ×2 anchors); NO for reload count (F-003). |
| (suite-wide) | NO for TARGETDURATION-less pacing | F-004 branch never executed. |
All green: log 5029-5043; e2e 5195-5202.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "HLS playback … live window … status partial; e2e hls_live_e2e … over the mockhttp real socket; streaming 173/0 lib tests" | docs/feature-matrix.toml `network-hls` (1431-1443) | REAL-in-library / UNREACHABLE-in-product | e2e 2/2 green (log 5195-5202), lib tests green — but no shipped route opens an m3u8 (F-001). "173/0" is also stale: the suite is now 212 passed / 2 ignored. |
| "None (the default) keeps v0's unpaced demand-driven reloads: the budget stays requests" | live.rs:144-145 (config doc) | REAL | stalled test asserts paced_reloads=0 without a clock; budget bounds the sequence either way. The unpaced default is a documented v0 tradeoff — would matter only if a host ever wired this lane. |
| "a full target duration after a productive window, half after a stalled one" (RFC 8216 §6.3.4) | live.rs:136-139 + paced tests | REAL | Sleep-vector assertions prove both halves of the cadence rule. |
| "the live edge = LOCAL + the window's EXTINF total" | live.rs module docs (part1) / test 1425-1432 | REAL | Exact ms arithmetic asserted for both anchors (1_577_836_803_000 → 1_577_840_403_000). |

## 7. Dead / duplicated / unreachable code
The tested session lane is unreachable from shipped hosts (F-001). Within my range, no dead helpers; `drain()` is documented as ENDED-session-only and used only where the fixture ends (verified each use).

## 8. Dependencies & flags
Tests use in-crate `clock::FakeClock`, `fetch::{HttpRequest, HttpResponse, Url}`, `testkit::Must`, std sync — consistent with the crate's std-only posture. `lumen-allow-literal` annotations on every unit.test URL (test vectors, verified convention). No feature gates on this module; no `#[ignore]` in range.

## 9. Open questions for the next agent
- part1 (W10-A02): `reload_wait_ms` fallback ownership — F-004's missing test would live naturally in your part's range if you add one; the branch is yours to defend.
- part2 (W10-A07): `open_live`'s open-time fetch sets `last_playlist_ms`? (Paced test 1 waits the FULL cadence from open, implying the anchor is the open fetch — confirm in your range.)
- Whoever owns apps/cli demux_host: the missing m3u8 route is the single highest-leverage wiring gap in this wave (F-001, same class as reconnect F-001 and mpd F-001).
