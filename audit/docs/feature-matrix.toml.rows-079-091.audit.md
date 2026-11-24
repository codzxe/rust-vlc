# CLAIM AUDIT: docs/feature-matrix.toml#rows-079-091 (playlist-format tail + protocol-access block)

| Field | Value |
|---|---|
| Auditor | W18-A09 |
| Date | 2026-11-24 |
| Rows in chunk | 13 (ids plfmt-xspf … protocol-nfs; matrix line refs 1183-1358) |
| Claims extracted | 13 row-claims + 8 embedded evidence/notes claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 3 REAL-lanes / 2 REAL-UNPROVEN / 0 / 0 / 0 / 2 FALSE / 2 UNREACHABLE |
| Internal contradictions found | 3 |

## 1. Blunt verdict

The playlist parsers are real, tested, executed code — and wired to NOTHING: no shipped entry point loads a local playlist (the only in-repo consumer of XSPF/PLS/CUE parsing is streaming's `remote_playlist.rs`, itself called by nobody; W10's "playlists unwired" holds). The description of row 079 says "Parse (and write) XSPF playlists" — there is no XSPF writer anywhere in the tree (the only serializer is `Playlist::to_m3u_string`, types.rs:65). The worst defect in this chunk is row 084 (protocol-file): statuses say `not_started` on all four platforms with empty evidence, while the shipped release binary opens local files through `input::FileAccess` on the `lumen open` path — executed in the Wave-0 baseline (MP4 smoke, exit 0). The row is false in the understating direction and makes the parity metric meaningless for the first-vertical's most basic capability. The protocol blocked rows (087-091) tell the truth about behavior but cite DRIFTED line anchors: every cited schemes.rs range points at unrelated test code.

## 2. Claim ledger

| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | xspf: "Parse (and write) XSPF playlists; implemented on win/linux/macos" | docs/feature-matrix.toml:1187,:1192 | parse REAL (executed); write claim FALSE; product-reach UNREACHABLE | Parser real: crates/playlists/src/xspf.rs (1037 lines; `MAX_XML_DEPTH: usize = 64` at :66; CDATA/entities documented :23-31; pub parse/parse_detailed/parse_bytes :268-479). Executed: baseline cargo-test-workspace.txt xspf_suite → `14 passed`, `2 passed`, `42 passed` (3 binaries, 0 failed). NO writer exists: rg over crates/playlists/src for `to_xspf\|write_xspf` → nothing; only `pub fn to_m3u_string` types.rs:65. Reachability: `open_remote_playlist` (crates/streaming/src/remote_playlist.rs:114) calls `parse_xspf_bytes` (:194) but has zero callers outside streaming (rg over apps/ crates/ → none) — cross-ref audit/waves/wave-10. |
| 2 | xspf evidence: "hand-rolled XML-subset pull parser, zero new deps, CDATA + entities, depth guard 64; 46/0 playlists gates" | :1193 | substance REAL; "46/0" stale | CDATA/entity/depth-64 all verified in xspf.rs (above). But "46/0" matches the playlists crate LIB unit count (baseline :4625 `running 46 tests`), not any XSPF gate; the xspf suite itself is 14+2+42=58 tests today. Zero deps: crates/playlists/Cargo.toml deps = none (stdlib only) ✓. |
| 3 | asx / b4s / playlist-census-tail: "not_started" ×4 each | :1197-1249 | TRUE (accurate) | ls crates/playlists/src/ → m3u, pls, xspf, cue, hls, mpd, probe, types, error, fuzz — no asx/b4s/wpl/itml modules. Honest empty rows. |
| 4 | cue: "implemented on win/linux/macos; 3 handcrafted AC sheets vs hand-computed truth, 4600-track stress" | :1226-1231 | REAL (executed); product-reach UNREACHABLE | crates/playlists/src/cue.rs (954 lines). Executed: baseline cue_suite → `test result: ok. 20 passed; 0 failed` (mapping verified via awk over cargo-test-workspace.txt). No shipped consumer of local CUE files (same unwired playlists crate; rg callers → fuzz + streaming only). |
| 5 | protocol-file: "status = not_started on all four platforms; evidence = \"\"; notes: 2.A1 input crate; first vertical" | :1261-1263 | FALSE (understates shipped reality) | The shipped `lumen open <path>` lane IS the input crate: apps/cli/src/demux_host.rs:418-425 `"file" => { … input::FileAccess::new().open(&uri) … }` (quote below, §3). crates/input/src/ has file.rs/buffered.rs (BufferedReader fixed-capacity bounded reader, lib.rs:17-18). Executed: baseline mp4-smoke.md `lumen open …/sample.mp4` → `source: File { path: … }`, Playing→Stopped, `exit_code=0`; rc-smoke command-layer-play PASS. A row claiming "no work yet" for a working, executed, shipped feature is a false matrix row. |
| 6 | protocol-directory: "not_started; directory enumeration behind the access trait" | :1274-1276 | TRUE (accurate) | rg 'read_dir\|Directory' over crates/input/src → only an error enum comment (error.rs:59 "not a regular file"); no directory:// access anywhere. (A directory WALK exists in crates/playlists/src/probe.rs:287+ — but that is the unwired playlists-probe lane, not an access-protocol implementation, and no shipped entry point calls it.) |
| 7 | protocol-http-https: "implemented on win/linux/macos; range requests, redirects, TLS verification" | :1287 | linux REAL (executed); win/macos REAL-UNPROVEN | Code: apps/cli/src/demux_host.rs:426 `"http" \| "https" => open_http_source(url, fetch)?`; fn open_http_source :474-500 builds `streaming::ranged::RangedResource` (production ureq transport via `SchemeRegistry::new().open_http`, :491-495). Executed through the REAL binary: baseline cargo-test-workspace.txt:3251 `test http_url_opens_over_a_real_loopback_socket ... ok` (tests/unsupported_path.rs:230 — a real loopback socket, exit 0). product.toml:157 `streaming-http = true`. TLS real per ADR-011 (schemes.rs:644-651 https classifies through the same always-verifying rustls transport). win/macos: notes themselves say "Per-OS runtime on real hardware: [VERIFY at first live run]" — honest, unproven. |
| 8 | udp/rtp/rtsp/smb/nfs: "blocked/unsupported; typed refusal at schemes.rs:557-577 (udp :563, rtp :562, rtsp :561, :677-682); classify default arm :294-305" | :1292-1352 | substance TRUE; line anchors FALSE (drifted) | Behavior verified: schemes.rs:323-326 `other => Err(SchemeError::Unsupported { scheme: other.to_string() })` refuses every non-file/http(s) scheme; test :692-708 `unsupported_schemes_are_refused_typed` asserts refusals for ftp/rtsp(:695)/rtp(:696)/udp(:697)/data/gopher/mms/sftp. BUT the cited ranges are wrong: :557-577 contains file-URI normalization tests (verbatim at §3); :294-305 contains the file/http parse arms, not the default refusal; :677-682 contains an https classification test. The anchors drifted when schemes.rs grew. |
| 9 | rtsp evidence: "userinfo-bearing rtsp refuses, no secret in Display (:677-682)" | :1316 | substance TRUE; anchor drifted | The userinfo rtsp test exists but at schemes.rs:829-836 (`scheme: "rtsp".to_string()` … `assert_eq!(info.name.as_deref(), Some("rtsp"))`). |
| 10 | smb/nfs: "no dedicated test row — refused as 'every other scheme' per :20-24" | :1340,1350 | TRUE | schemes.rs:21-23 module doc (verbatim §3): "every other scheme (`ftp:`, `rtsp:`, `data:`, …) → the typed refusal `SchemeError::Unsupported` naming the scheme." The smb/nfs refusal rides that default arm (:323-326) — verified. |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Row 084 protocol-file: `status = { windows = "not_started", linux = "not_started", … }` with `evidence = ""` (matrix :1261-1262) | The shipped binary opens local files: baseline mp4-smoke.md `lumen open sample.mp4` → `source: File { path: "…/sample.mp4" }` … `exit_code=0`; CLI code apps/cli/src/demux_host.rs:418-425 | The CODE and the RUN are true; the matrix row is FALSE (stale). Verbatim, demux_host.rs:418-425: `"file" => {\n    let path = input::resolve_file(&parsed)\n        .map_err(|err| access_refusal(&err, &parsed))?;\n    let uri = file_uri_from_canonical(&path)?;\n    let stream = input::FileAccess::new()\n        .open(&uri)\n        .map_err(|err| access_refusal(&err, &uri))?;\n    (uri, stream)\n}` |
| Row 086 evidence cites "unsupported_path.rs http_url_opens_over_a_real_loopback_socket, exit 0 + session accepted" | Baseline run log | Both agree: cargo-test-workspace.txt:3251 `test http_url_opens_over_a_real_loopback_socket ... ok`. No contradiction — the row is one of the few with end-to-end proof. |
| Rows 087-091 evidence strings cite schemes.rs:557-577 as "typed-refusal table (udp row :563)" | schemes.rs:557-577 actually contains file-URI tests: verbatim :564-566 `fn file_uris_normalize_to_local_paths() {` … `("file:///media/a.wav", "/media/a.wav"),` | The doc's locators are false; the mechanism is real (default arm :323-326 + test :692-708). R4 requires evidence to resolve — these anchors resolve to the WRONG lines. |

## 4. Fabrication / inflation indicators

- Row 084 (protocol-file) is stale-false in the CONSERVATIVE direction — it hides a working feature, the inverse of inflation, but equally destroys the parity ledger's purpose: 0/252 "verified" is partly an artifact of rows never updated after the first vertical landed.
- "46/0 playlists gates" reused as evidence across multiple playlist rows (xspf here, pls at :1178) is a copy-paste gate figure that no longer matches any single suite (46 = lib tests only; xspf is 58 across its own suites).
- The five protocol blocked rows copy-paste the same drifted line-anchor pattern (":294-305", ":557-577", ":677-682") — evidence strings written once against an older schemes.rs and never re-resolved.

## 5. Matrix rows

| Row id | Statuses claimed (w/l/m/web) | Evidence string (verbatim key) | Code actually found | Executed? | Verdict |
|---|---|---|---|---|---|
| plfmt-xspf | implemented ×3, web not_started | "crates/playlists/src/xspf.rs (card 5.O1b: hand-rolled XML-subset pull parser, zero new deps, CDATA + entities, depth guard 64; 46/0 playlists gates)" | xspf.rs real (1037 lines, depth-64, CDATA, entities); NO writer for the "(and write)" half of the description | YES — baseline xspf_suite 14+2+42 passed | parse REAL; write FALSE; UNREACHABLE from shipped entry points (unwired crate; W10) |
| plfmt-asx | not_started ×4 | "" | none | n/a | TRUE |
| plfmt-b4s | not_started ×4 | "" | none | n/a | TRUE |
| plfmt-cue | implemented ×3, web not_started | "crates/playlists/src/cue.rs (card 5.O2: 3 handcrafted AC sheets … 4600-track stress; playlists gates green at card)" | cue.rs real (954 lines) | YES — baseline cue_suite 20 passed/0 failed | REAL code+tests; UNREACHABLE from shipped binary (no playlist loading wired) |
| plfmt-census-tail | not_started ×4 | "" | none | n/a | TRUE |
| protocol-file | not_started ×4 | "" | input::FileAccess + BufferedReader — WIRED in the shipped open path | YES — baseline mp4-smoke (File open, exit 0) + rc-smoke | FALSE row (feature works; statuses say no work) |
| protocol-directory | not_started ×4 | "" | none (playlists::probe::walk exists but is a different, unwired lane) | n/a | TRUE |
| protocol-http-https | implemented ×3, web not_started | "w31.3: apps/cli/src/demux_host.rs open_http_source … loopback e2e through the REAL binary … exit 0; product.toml streaming-http = true" | open_http_source + RangedResource + ureq; product.toml:157 | YES — baseline :3251 loopback test ok through real binary | linux REAL; win/macos REAL-UNPROVEN (row's own [VERIFY] honest) |
| protocol-udp | blocked ×3, unsupported web | "schemes.rs:557-577 typed-refusal table (udp row :563) + classify default arm :294-305; investigation rtsp-rtp-feasibility.md" | refusal mechanism real (arm :323-326, test :697) | unit-test refusal asserted in baseline (schemes tests in streaming suite) | TRUE behavior; FALSE line anchors |
| protocol-rtp | blocked ×3, unsupported web | "…rtp row :562…" | same mechanism (test :696) | ditto | TRUE behavior; FALSE anchors |
| protocol-rtsp | blocked ×3, unsupported web | "schemes.rs:561 + :677-682 (userinfo-bearing rtsp refuses…)" | rtsp refusal + userinfo-redaction test real (at :695, :829-836 — not at cited lines) | ditto | TRUE behavior; FALSE anchors |
| protocol-smb | blocked ×4 | "classify default arm :294-305 … refused as 'every other scheme' per :20-24" | default arm real at :323-326; doc bullet real at :21-23 | ditto | TRUE behavior; partially-stale anchors |
| protocol-nfs | blocked ×4 | same pattern as smb | same | ditto | TRUE behavior; partially-stale anchors |

## 6. Findings routed to repo-level

- F-E (S2, doc): matrix row protocol-file (:1261-1262) says not_started/evidence-empty while the shipped binary opens local files (executed baseline) — the parity ledger is stale against its own first vertical; the parity metric cannot be trusted per-row.
- F-F (S3, code): no XSPF writer exists, but row 079's description and vlc_modules claim "(and write)" / ["playlist", "export"] — PAPER-ONLY half-claim inside an otherwise real row.
- F-G (S3): playlist format rows 079/082 claim "implemented" with no disclosure that the parsers are unreachable from any shipped entry point (unwired crate) — the matrix has no reachability dimension and hides this.
- F-H (S4): 5 protocol rows carry drifted evidence line-anchors (":557-577", ":294-305", ":677-682") that point at unrelated test code — evidence strings written against an older schemes.rs revision, violating the file's own R4 resolution rule.
- F-I (S4): "46/0 playlists gates" figure copy-pasted across playlist rows no longer describes any existing test suite (46 = crate lib count; xspf = 58 across suites).
