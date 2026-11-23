# AUDIT: crates/streaming/tests/net_interrupted.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| Lines / functions | 385 (+239 in included tests/netfault/mod.rs) / 3 tests + 6 helpers (+ netfault: 10) |
| File verdict | REAL — strong deterministic fault-injection e2e over a real socket |
| Reachable from shipped entry point? | NO — exercises `streaming::hls::{open_vod,open_live}` + `reconnect::RetryFetch`, none reachable from apps (only apps/cli depends on streaming: fetch/ranged/schemes only) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `cargo test -p streaming --test net_interrupted` → 3 passed/0 failed (exit 0, 0.02s); read tests/netfault/mod.rs; sed quote checks |

## 1. One-paragraph verdict
Three genuinely adversarial network scenarios run over a REAL loopback socket
(mockhttp), with deterministic byte-offset fault injection (truncate/reset/drop/
delay) and a virtual clock that makes 1.3 s of scripted wire latency and the whole
exponential backoff ladder cost zero real time — all asserted, not assumed:
byte-exact resume against the patterned body, exact fetch/fault counts, typed
error code `streaming.hls.fetch-failed`, per-attempt ceiling bounds on every
sleep, and a byte-exact live-edge join against on-disk ffmpeg fixtures. I ran
the suite: 3/3 pass in 0.02s. The limits: this proves the `streaming` library,
not the shipped product (the CLI's http lane is `RangedResource` progressive,
which does not go through `open_vod`/`open_live`/`RetryFetch`), and the fault
classes are application-layer emulations of wire shapes (the proxy faults a fully
fetched response), which the netfault docs admit inline.

## 2. What actually works (verified)
— verified by `cargo test -p streaming --test net_interrupted` → 3 passed; 0
failed. The virtual-time claims are self-proving: `started.elapsed() < 5s`
assertions in tests (b) and (c) would fail on real sleeping; `clock.sleeps() ==
vec![900, 400]` pins the exact scripted delays.

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/net_interrupted.rs:34-37 (imports) — whole file
Code:
```
use streaming::hls::live::{HlsLiveConfig, open_live};
use streaming::hls::media::{HlsVodConfig, open_vod};
```
What is wrong: The scenarios prove `streaming::hls` + `streaming::reconnect`
library behavior. No shipped entry point reaches those modules (rg over
apps/: `hls::` zero hits; CLI uses fetch/ranged/schemes only), so
"interrupted-network playback resilience" is proven for a lane the product
does not open. Wave-level reachability gap (same root as mpd.rs F-001), not a
defect of this file's logic. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/streaming/tests/netfault/mod.rs:198-237 (FaultProxy::fetch)
Code:
```
        let fault = self.script.take();
        let response = self.inner.fetch(request)?;
```
What is wrong: All offset faults apply AFTER the inner transport returns a
complete successful response — the "TCP RST mid-body"/"black hole" wire shapes
are emulated by discarding the body and returning the typed error, not produced
on the socket. Deterministic (good) but weaker than the doc table's "Wire shape"
column implies at first read; the inline comment (L202-205) is honest about it.
Risk: a bug that only manifests during partial socket reads (e.g. a reader that
blocks forever on a stalled recv) is not covered by this harness. Confidence:
HIGH — Verification: read.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| body / serve | 45-59 | REAL | — | patterned bytes (offset drift visible), Range-honoring server |
| proxied | 62-68 | REAL | — | socket → MockClient → FaultProxy(clock) |
| mid_segment_drop_resumes_byte_exact | 80-148 | REAL | — | DropAfter@300 on ranged 250.. fetch + TruncateAt@300 sibling; counts + stats + byte-exact |
| manifest_reset_fails_typed_with_bounded_backoff | 161-237 | REAL | — | ResetAfter forever; typed code, 4 retries/5 fetches, per-attempt ceilings, ladder sum, <5s real |
| ORACLE_BYTES/fixture/windows/read_exactly | 245-310 | REAL | — | on-disk oracle + 4 sliding windows with a real discontinuity seam |
| live_window_edge_survives_wire_delay | 319-385 | REAL | — | delays 900/400ms virtual; edge join back=2; byte-exact vs cat(s01..t01); segments_lost_window==0 |
| netfault: Fault/FaultScript/FaultProxy | mod.rs:51-239 | REAL | S4 | deterministic, poison-safe, counters; app-layer emulation (F-002) |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mid_segment_drop_resumes_byte_exact | YES | body equality + fired/fetched counts + retry stats all exact |
| manifest_reset_fails_typed_with_bounded_backoff | YES | error code, kind, retries=4, exhausted=1, fetched=5, ceiling math, wall-clock bound |
| live_window_edge_survives_wire_delay | YES | full byte equality vs on-disk fixtures + exact sleep ledger [900,400] + fired=2 |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "re-issues the identical ranged request… byte-exact" | net_interrupted.rs:9-12 | REAL | ran; assert_eq on body + script.fetched()==2 |
| "zero real wall-clock cost" | net_interrupted.rs:15-16 | REAL | ran in 0.02s; elapsed assertions hold |
| "loses no window" | net_interrupted.rs:17-19 | REAL | segments_lost_window==0 asserted |
| netfault "at byte offsets… exactly scripted" | netfault/mod.rs:4-9 | REAL | fired() counts only faults that bit (offset > body passes through) |
## 7. Dead / duplicated / unreachable code
- Library-only coverage gap (F-001). No dead code in the file.
## 8. Dependencies & flags
streaming (clock, fetch, hls, mockhttp, reconnect), plugin_api, testkit,
netfault test module via #[path]. No features.
## 9. Open questions for the next agent
- RetryFetch + FaultProxy under a REAL stalled socket (no deadline): who owns
  the read deadline in the ureq client — is ReadTimeout reachable in production
  or only via this proxy? (crates/streaming/src/client.rs, not my file)
- Is `segments_lost_window` surfaced anywhere a user could see it?
