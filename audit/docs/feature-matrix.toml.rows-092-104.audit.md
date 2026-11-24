# CLAIM AUDIT: docs/feature-matrix.toml (rows 092-104)

| Field | Value |
|---|---|
| Auditor | W18-A10 |
| Date | 2026-11-24 |
| Rows audited | 13 (protocol-ftp-sftp … playback-rate; file lines 1359-1531) |
| Claims extracted | 41 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 16 / 6 / 3 / 0 / 0 / 11 / 5 (claim-level) |
| Internal contradictions found | 6 |

## 1. Blunt verdict

This chunk is where the matrix stops describing the repo. Four rows
(095 http-client, 099 reconnect+buffering, 100 auth, 101 remote
playlists) say "not_started — no work yet" while ~4,900 lines of real,
tested code for exactly those features sit in `crates/streaming/src/`
(`client.rs` 998, `reconnect.rs` 1560, `auth.rs` 1562, `buffer.rs` 827,
`remote_playlist.rs` 500), with prior-wave executed test runs (61 tests
pass across the five modules). Row 095 is directly contradicted INSIDE
this same document by row 096, whose evidence string describes the landed
ureq HTTP client that row 095 claims does not exist. The HLS/DASH rows
(097/098) claim "partial = a documented subset works" on three platforms
— but every module audit verdict for that lane is UNREACHABLE: no
shipped entry point can open an m3u8/MPD at all; the "subset that works"
works only under test harnesses. Row 103 (playback-seek "implemented")
hides an executed S0 from Wave 5: in-session MP4 seek ALWAYS fails
(demuxer never implements `reanchor`) while MediaInfo reports
`seekable: true`. The blocked protocol rows (092/093) are honest in
substance but every cited line number has drifted by 100+ lines. The
playback rows 102/104 are the honest end of the chunk.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | ftp-sftp: blocked ×4 | :1368 | REAL (honest blocked) | Default arm refuses typed: `schemes.rs:322-324` `other => Err(SchemeError::Unsupported { scheme: other.to_string() })`; test `:692-711` covers ftp/sftp rows. |
| 2 | "schemes.rs:557-577 typed-refusal table (ftp row :560, sftp row :567) + :636-665 ftp error-display tests" | :1369 | STALE-cites (content real, lines wrong) | Actual: refusal table test at :692-711, credential-free Display tests at :790-805, default arm at :322. Cited :557-577/:560/:567/:636-665/:294-305 contain a file-URI test, http classify and other content — all drifted. |
| 3 | mmsh-mms: blocked ×4 | :1381 | REAL (honest) | `mms` in the typed-refusal test vector (:700); mmsh falls to the same default arm. |
| 4 | "classify default arm :294-305 (mmsh variant refused by the same default)" | :1382 | STALE-cite | Default arm is at :322-324, not :294-305 (:294-305 is the classify entry + bare-path/local arm). |
| 5 | census-tail: blocked ×4 rollup, "per-scheme rows to be appended by 6.G1-I" | :1394/:1397 | REAL (honest placeholder) | tcp/sdp/rist/… refused typed by the same default arm; no per-scheme rows exist yet — as stated. |
| 6 | network-http-client: "not_started" ×4 | :1413 | FALSE | `crates/streaming/src/client.rs:1-9` "ureq adapter — the real HTTP(S) transport … (card 6.A1b admission; hardened by 6.A1c)" (998 lines, bounded redirects/loop detection/response budget); ureq in Cargo.lock:1214; shipped reachability: `lumen open http://…` → apps/cli/src/parse.rs:498-505 URL spec → demux_host.rs:491 `SchemeRegistry::new().open_http(` → UreqClient (prior fetch.rs audit: "YES — lumen open http://… → … → UreqClient"). |
| 7 | "6.A1: ureq or reqwest per ADR-011; mock-server tests (redirect loops, timeouts, 416 range)" | :1415 | STALE notes | Cards 6.A1a/b/c LANDED (client.rs doc header); notes describe the pre-card decision state. Doc 13 card 6.A3 (:636) lists http/https access plugins as landed surface. |
| 8 | tls-policy: implemented ×3, web not_started | :1426 | REAL (code) / REAL-UNPROVEN (execution) | `client.rs` REAL per prior audit ("TLS posture verified: rustls always-on, no env override exists"); `agent_config_pins_transport_safety` test at client.rs:923. |
| 9 | "https classifies through the same fetch-port parse as http (ADR-011 accepted 2026-11-09 …)" | :1427 | REAL | Test `https_classifies_exactly_like_http_post_adr011` exists (schemes.rs:655+); `crates/streaming/tests/https_activation.rs` exists (9 test files incl. it). |
| 10 | "NOT yet proven in-tree: a real TLS handshake round-trip … sandbox has no egress [VERIFY]" | :1428 | REAL (honest gap) | Notes admit it; the "implemented" cell is code-complete-only per schema; no TLS handshake ever executed anywhere in-tree. |
| 11 | hls: partial ×3, web not_started | :1439 | SHALLOW (product) | Code real: `streaming/src/hls/` (5 files, 4,215 lines) + `playlists/src/hls.rs` (997); 4 e2e test files exist (hls_vod/live/master/conformance_e2e.rs) — but ALL five module audits verdict UNREACHABLE: "no shipped entry point opens it" (`lumen open http://…/x.m3u8` has no demux/playlist route — zero m3u8 references in apps/cli/src/demux_host.rs/parse.rs). |
| 12 | "streaming 173/0 lib tests" | :1440 | STALE | Baseline cargo-test-workspace.txt:5617 "test result: ok. 212 passed; 0 failed; 2 ignored" — count drifted from 173 to 212. |
| 13 | "Partial per evidence-Linux-only: alternate renditions … not selected; KEY/BYTERANGE/MAP + fMP4 lanes refuse typed" | :1441 | REAL (honest partial scope) | Typed refusals in hls modules confirmed by prior audits; the partial scoping is honest at the library level. |
| 14 | dash: partial ×3 | :1452 | SHALLOW (product) | `streaming/src/dash/` (5 files, 2,764 lines) + `playlists/src/mpd.rs` (2,854) real + tested, but dash/mod.rs audit: "UNREACHABLE … the lane has no session/opener at all". |
| 15 | "streaming 191 lib tests (2 ignored) incl. 27 dash" | :1453 | STALE | Same baseline: 212 passed / 2 ignored. |
| 16 | "MPD reload loop …, multi-Period stitching, CENC decrypt, mockhttp socket e2e + DASH-IF vectors pending" | :1454 | REAL (honest gap list) | Pending items verifiable as absent (no reload loop in dash/sched.rs per prior audit). |
| 17 | reconnect-buffering: "not_started" ×4 | :1464 | FALSE | `reconnect.rs` (1,560 lines) prior audit: "UNREACHABLE (real, tested code — no shipped entry point composes it)" with `cargo test -p streaming --lib reconnect::` → "20 passed; 0 failed"; `buffer.rs` (827 lines) → "10 passed; 0 failed". "No work yet" is false; work exists, is tested, and is unwired. |
| 18 | "6.D1/6.D2 [PROPOSAL parameters]" | :1466 | STALE notes | Notes describe proposal state; implementation exists. |
| 19 | auth: "not_started" ×4 | :1476 | FALSE | `auth.rs` (1,562 lines) prior audit: real Basic/Digest over real 127.0.0.1 sockets — `auth::` 11 passed, `auth_e2e` 8 passed, `auth_scheme_gate` 2 passed; `crates/streaming/tests/auth_e2e.rs` + `auth_scheme_gate.rs` exist. |
| 20 | "6.E1; keychain policy + crate per OS [VERIFY]" | :1478 | STALE (notes) | HTTP auth half landed+tested (auth.rs); only the OS-keychain half is genuinely [VERIFY]/absent. |
| 21 | remote-playlists: "not_started" ×4 | :1488 | FALSE | `remote_playlist.rs` (500 lines) prior audit: "REAL (genuine, well-tested network opener) but UNREACHABLE from shipped hosts"; 9/9 `remote_playlist::tests` ok in baseline (cargo-test-workspace.txt:5588-5604). |
| 22 | "6.E2; HTML listing parse = [PROPOSAL decision]" | :1490 | STALE (notes) | Remote playlist loading exists in code; the HTML-listing half is the honest remaining [PROPOSAL]. |
| 23 | play-pause-stop: implemented ×3, web not_started | :1505 | REAL (linux) / REAL-UNPROVEN (win,mac) | state.rs + `session.rs:328-349` `pub fn play/pause/stop` (sed-verified); UI consumption real: PlayerScreen.tsx:850-853 routes `playback.play/pause/stop` via the player context transport; desktop dispatch delegates to the CLI command layer (apps/desktop/src/dispatch.rs:211-213); baseline command-layer-play PASS (rc-smoke.txt). Prior session.rs audit: REAL, executed. |
| 24 | "UI consumption ui/src/screens/player/PlayerScreen.tsx control-bar actions" | :1506 | REAL (with caveat) | Verified: context.tsx:91+ dispatches `playback.seek`/`play`/`pause` over the client transport with typed pending states — real command surface; in the web/demo lane the responder is the demo transport (demo-transport.ts:423), in the desktop lane the real engine via tauri IPC. |
| 25 | "core hotkeys Space/S + media keys" | :1507 | REAL (data-level) | keymaps/default.toml:61-64 `media-key-play-pause … command = "playback.toggle-play-pause"` / `media-key-stop … "playback.stop"`. (Keymap-to-engine wiring caveats live in W11's findings, not this row's claims.) |
| 26 | seek: implemented ×3, web not_started | :1517 | SHALLOW | Engine seek real (seek.rs; `session.rs` pub seek); reanchor implemented by 9 demuxers (adts/mkv/wav/avi/ivf/aiff/ogg/asf/ps — grep `fn reanchor`) but NOT mp4 and NOT ts: W5-A01 [S0, EXECUTED] "In-session MP4 seek ALWAYS fails: Mp4Demuxer never implements `reanchor` … Shipped `lumen … seek 1s` -> `TimingSeekFailed` … while MediaInfo reports `seekable: true`". Row carries no caveat. |
| 27 | "jump scalars seeded in keymaps/default.toml (3/10/60/300 s)" | :1518 | REAL | keymaps/default.toml:101-108 delta_ms 10000/3000/60000/300000 (short/extrashort/medium/long). |
| 28 | "Ctrl+T dialog ui/src/screens/player/dialogs/GotoTimeDialog.tsx" | :1518 | REAL | File exists (+ GotoTimeDialog.test.tsx). |
| 29 | rate: partial ×3, web not_started | :1529 | REAL (honest partial) | `ids.rs:107` `PlaybackSetRate => "playback.set-rate"`; OpenOptions.rate typed (types.rs:55 doc + parse.rs construction); engine deliberately lacks rate — `playback-engine/src/lib.rs:49-51` "What is deliberately NOT here (Phase 3+): A/V sync, rate, …"; desktop dispatch test proves the typed refusal (dispatch.rs:297-313 `PlaybackSetRate` → `UnsupportedForHost`, "set-rate is not served by this build"). |
| 30 | "demo wiring ui/src/client/demo-transport.ts" | :1530 | REAL | demo-transport.ts:203/:423 handle `playback.play`/set-rate family — the only lane that currently answers rate. |
| 31 | "engine clock-rate deliberately NOT in crates/playback-engine/src/lib.rs yet (Phase 3+)" | :1531 | REAL (honest) | lib.rs:49-51 verbatim match. |
| 32 | (rows 092/093) blocked_reason doc refs "doc 13 card 6.A3 (ftp decision)" / "formats.json rationale" | :1370/:1383 | REAL | doc 13:636 card 6.A3 "ftp decision `[PROPOSAL: blocked v1]`" ✓; formats.json#ftp-sftp anchor exists. |
| 33 | (schema R4) "any verified ⇒ evidence resolves" — chunk has zero verified cells; all evidence paths resolve | all rows | REAL | Every path token in rows 092-104 exists on disk (streaming/src/hls, dash, client.rs, session.rs, keymaps, dialogs — checked). |

Claims 34-41: per-row vlc_modules/inventory_ref anchors (access_mms, adaptive, access, gnutls…) — seed-inventory provenance, not falsifiable here (inventory audits own them).

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 095: http-client "not_started — no work yet" (schema :49-50) | Row 096 evidence: "crates/streaming/src/client.rs tls_config"; client.rs:1-9 "the real HTTP(S) transport"; Cargo.lock ureq 3.5.0; doc 13 card 6.A3 landed | The code is true; the row is false. Same-document self-contradiction. |
| Rows 097/098: HLS/DASH "partial = a documented subset works" on win/linux/mac | Prior audits: hls/* + dash/* + playlists/hls.rs + mpd.rs all "UNREACHABLE from shipped entry points"; zero m3u8/mpd routing in apps/cli | The subset "works" only under test harnesses; no shipped product path. Platform cells overstate availability. |
| Rows 099/100/101: "not_started", "no work yet" | reconnect.rs/buffer.rs/auth.rs/remote_playlist.rs: 4,849 lines, 61 tests executed green (prior audits + baseline) | Work exists and is tested; the rows are false. The honest issue is UNWIRED (W8's orphan pattern), which the matrix never states. |
| Row 103: playback-seek "implemented" ×3 | W5-A01 executed: MP4 in-session seek ALWAYS fails (TimingSeekFailed), MediaInfo lies seekable:true; W5 handoff: "Wave 18: matrix rows claiming MP4/MKV/TS seek" | Engine-real, flagship-container-broken. Implemented overstated for MP4/TS. |
| Row 092/093 evidence line-cites (:557-577, :560, :566, :567, :636-665, :294-305) | Current schemes.rs: refusal table test at :692-711, Display tests at :790-805, default arm at :322-324 | Cites drifted ~130+ lines (ADR-011 surgery reshuffled the file); content still real. |
| Rows 097/098 test counts ("173/0", "191 lib tests (2 ignored)") | Baseline streaming: 212 passed / 0 failed / 2 ignored | Both stale undercounts. |

## 4. Fabrication / inflation indicators

1. The understatement cluster (rows 095, 099, 100, 101): the board hides
   ~4,900 lines of landed, tested feature code as "no work yet" — the
   opposite lie of inflation, equally a false status. Whatever process
   owns the matrix (the 8.B2 transitions writer only flips cells from
   outboxes; nothing flips these rows), it has not run since the 6.x
   network cards landed.
2. Rows 097/098 "partial on 3 platforms" import library-level honesty
   into product-level cells: no shipped build can reach the lane at all,
   yet three platform cells claim a working subset.
3. Row 103 "implemented" with zero caveat while the P0 container's seek
   is an executed failure — the exact row Wave 5 handed off to Wave 18
   as "the seek story is fiction (S0)".
4. Evidence strings cite line numbers that no longer exist where claimed
   (rows 092/093) — R4 path-resolution passes (the paths exist) while the
   line-level citations are fiction vs the current file.
5. Stale numeric evidence (173/191 vs 212) — counts recorded at write
   time, never refreshed.

## 5. Matrix rows (13)

| Row | id | Statuses claimed (w/l/m/web) | Evidence string claims | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|---|
| 092 | protocol-ftp-sftp | blocked ×4 | schemes.rs typed-refusal table + ftp Display tests (line-cites) | Default typed refusal arm :322; ftp/sftp in refusal test :694-701; credential-free Display test :790+ | YES: prior schemes.rs audit ran the suite; refusal test in baseline 212-pass lib run | REAL status; STALE line-cites |
| 093 | protocol-mmsh-mms | blocked ×4 | refusal table mms row + default arm | Same arm/test (mms :700) | YES (same suite) | REAL status; STALE line-cites |
| 094 | protocol-census-tail | blocked ×4 | (none — rollup row) | tcp/sdp/rist/… refused by same default arm | n/a | REAL (honest placeholder) |
| 095 | network-http-client | not_started ×4 | (none — evidence empty) | client.rs 998 + fetch.rs 730 + ranged.rs 1886, ureq in lock, SHIPPED via `lumen open http://…` | YES: prior fetch.rs/client.rs audits; https_activation tests; W-13 logs show http open path | FALSE — status contradicts code |
| 096 | network-tls-policy | implemented ×3, web ns | client.rs tls_config, ADR-011 tests, ureq record R10 | client.rs REAL (rustls always-on, verified by prior audit); test at :923 | REAL-TLS handshake NO (no egress; notes admit); rest YES | REAL code / REAL-UNPROVEN execution |
| 097 | network-hls | partial ×3, web ns | hls/** + playlists/hls.rs + 4 e2e suites + 173 tests | All files exist (4,215 + 997 lines); e2e tests exist over real sockets | YES (prior audits executed) — but from NO shipped entry point | SHALLOW (library-only; unreachable lane) |
| 098 | network-dash | partial ×3, web ns | dash/** + mpd.rs + 191 tests | All files exist (2,764 + 2,854 lines) | YES (prior audits) — unreachable from shipped | SHALLOW (library-only) |
| 099 | network-reconnect-buffering | not_started ×4 | (none) | reconnect.rs 1,560 + buffer.rs 827, tested | YES: 20 + 10 tests pass (prior audits + baseline) | FALSE — status contradicts code |
| 100 | network-auth | not_started ×4 | (none) | auth.rs 1,562 + auth_e2e/auth_scheme_gate tests | YES: 11+8+2 tests pass over real sockets | FALSE — status contradicts code |
| 101 | network-remote-playlists | not_started ×4 | (none) | remote_playlist.rs 500 | YES: 9/9 tests pass (baseline) | FALSE — status contradicts code |
| 102 | playback-play-pause-stop | implemented ×3, web ns | state.rs + session.rs :333-349 + UI consumption | All verified (session.rs:328-349 sed-verified; PlayerScreen context dispatch) | YES: baseline command-layer-play PASS; prior session audit ran open→full state walk | REAL (linux); win/mac REAL-UNPROVEN |
| 103 | playback-seek | implemented ×3, web ns | seek.rs + session seek + keymap scalars + GotoTime dialog | All cited artifacts exist; reanchor in 9 demuxers but NOT mp4/ts | YES: W5-A01 executed MP4 seek → TimingSeekFailed (S0) | SHALLOW — engine real, MP4/TS seek broken in-session, no caveat |
| 104 | playback-rate | partial ×3, web ns | set-rate command + OpenOptions.rate + keymap + demo wiring | ids.rs:107 + types.rs + keymap :113-114 + demo-transport; engine rate absent (lib.rs:49-51) | YES: desktop dispatch test proves typed UnsupportedForHost for set-rate | REAL (honest partial) |

Row-level summary: 4 rows with statuses contradicted by the code (095,
099, 100, 101 — all in the understatement direction); 2 rows whose
"partial works" is unreachable from any shipped entry point (097, 098);
1 row hiding an executed S0 on the P0 container (103); 3 honest blocked
rows with drifted evidence line-cites (092, 093, 094); 3 honest rows
(096, 102, 104).

## Commands run

sed/grep verification of every quoted line (exit 0); `grep 'fn reanchor'`
over crates/demux/src (9 implementors; mp4/ts/es absent); Cargo.lock
dep check (ureq present at :1214); baseline cites: streaming 212/0/2
(cargo-test-workspace.txt:5617), codecs 506/0 (:880), rc-smoke
command-layer-play PASS. No fresh cargo runs (no target/, cargo budget) —
execution evidence is the orchestrator baseline + prior-wave audit runs.

## Open questions for the next agent

- Who re-runs `cargo xtask compat-matrix` after the 6.D/6.E cards so
  rows 099-101 stop lying "not_started"? The 8.B2 writer exists; no
  outbox has ever proposed these flips.
- Row 095 (http-client) vs row 096 (tls): same lane, opposite cells —
  resolve at Lead level; the matrix self-contradicts.
- Row 103 needs the W5-A01 caveat in notes (MP4/TS seek executed-fail)
  or a status flip; W5's handoff line names this row explicitly.
- engine_info() (apps/cli/src/host.rs:1643-1657) still hard-codes
  `decoders: mjpeg, pcm_s16le` — any consumer trusting engine-info
  instead of the pipeline registry inherits the lie (feeds the parity
  tooling story).
