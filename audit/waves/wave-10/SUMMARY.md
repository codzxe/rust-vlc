# WAVE 10 — SUMMARY (streaming / network / access)

Date: 2026-11-21 (session 2) · Agents W10-A01…A10 (A09 re-spawned per §7.4) ·
Domain: streaming (HTTP/ranged/ureq/TLS), HLS/DASH/MPD, playlists
(M3U/PLS/XSPF/CUE), reconnect, transcode, device-discovery, input/access.
Gate: **PASS — 82/82 assignments audited, 0 missing, 0 thin, citation
spot-checks clean.** (Agents wrote many composite verdict strings; canonical
form below.)

## Verdicts (file level)
REAL/shipped-reachable ~14 (mostly the streaming error/schemes/open_http core)
· UNREACHABLE-from-shipped ~66 · REAL-UNPROVEN (TLS) — see below.

## Findings
**S0:0 S1:6 S2:25 S3:96 S4:138 — total 265**

## The wave's hostile questions
- **What really opens a URL?** Only `open_http` via a bare UreqClient
  (schemes.rs → fetch.rs ranged path) is wired into the shipped CLI. HLS,
  DASH/MPD, playlists (m3u/pls/xspf/cue), the reconnect/RetryFetch layer,
  auth, transcode, and device-discovery are real, tested, baseline-green
  libraries with **zero shipped consumers** — the CLI never routes
  `.m3u8/.pls/.mpd` to them, `playlist.load` is unserved, transcode and
  device-discovery have no Cargo edges into any shipped binary.
- **TLS claim vs evidence: REAL-UNPROVEN.** The rustls config is real
  (client.rs), but **zero TLS handshakes exist in all recorded evidence** —
  every real-network test is `#[ignore]`d and was never executed. REQUEST-
  ORCH-RUN filed by 4 agents (`cargo test -p streaming live_ -- --ignored`).
- **`schemes.rs`'s "everything the player can open arrives here first" is
  false**: the CLI dispatches URLs with its own `split_once("://")` heuristic
  (parse.rs:505); RFC 8089 `file:/path` URIs mis-route to file-not-found; the
  classify half is dead code.

## Worst findings (citations in per-file audits; sampled at gate)
1. [S1] hls/live.rs:406-434 `absorb`/`serve` — a segment body over the
   (legal-config) `max_retained_bytes` is discarded and the next read
   u64-underflows/slice-panics; identical code ships in media.rs VOD, untested
   in both (A02).
2. [S1] hls/media.rs `open_vod` probe_cache holds every no-range server's full
   segment body with no aggregate budget — large VOD allocates in full at
   open, contradicting "Bounded memory (MP §12)" (A08).
3. [S1] DASH resolver division-by-zero panics on parser-accepted `@d=0` +
   negative `@r` / `@duration=0` (resolve.rs:410/735), contradicting the
   fuzz harness's "Never panics" contract (A07).
4. [S1] resolve.rs:609-617 — `MAX_PLAN_URL_BYTES` debits the pre-folding URL
   but stores the base-folded one: a 64 KiB `<BaseURL>` × 65,536-segment
   timeline mints ~4 GiB of strings; the typed refusal never fires (A06).
5. [S2] fetch.rs URL fragments silently ride the request line though the
   port's doc claims typed rejection — reachable today via `lumen open` (A01).
6. [S2] "v0 ships both layers on" (reconnect) is false — executor real but
   never wired (A03).

## Blocked / UNVERIFIED
- TLS handshake execution (no egress; REQUEST-ORCH-RUN pending).
- ISO 23009-1 DASH spec text unavailable (§5.3.2.1 cited from knowledge,
  marked UNVERIFIED).
- streaming/playlists/transcode test suites never ran in the primary baseline
  (aborted before those packages) — greens exist only in the no-fail-fast
  rerun (same INDEX-drift issue as Wave 9).
