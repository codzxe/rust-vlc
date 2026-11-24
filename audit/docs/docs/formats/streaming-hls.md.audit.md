# CLAIM AUDIT: docs/formats/streaming-hls.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R4) |
| Claims extracted | 14 |
| REAL 10 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 4 |
| Internal contradictions found | 1 (minor wording) |

## 1. Blunt verdict
This record's static claims are fully backed: every named file, test, fixture
lane, fuzz target, corpus seed, typed refusal, and the saturating-overflow fix
are in the tree by exact name. The recorded cargo-test numbers cannot be re-run
here (build budget) but the orchestrator baseline shows the same shape
(streaming lib 0 failed / 2 ignored, count grown 173→201→212 past the doc's
dates). Honest-bounds section matches the code's typed refusals. One wording
slip: "partial on every platform" while the matrix row's web lane is
not_started.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | self_built = playlists/src/hls.rs + streaming/src/hls/{media,live,master,timestamp_map}.rs | :22-23 | REAL | `ls crates/streaming/src/hls/` -> live.rs master.rs media.rs mod.rs timestamp_map.rs; playlists/src/hls.rs exists. |
| 2 | "crates = [] — zero external deps" for the HLS self-build | :18 | REAL | No new deps required by these files (hls modules use workspace deps only); the playlists-crate claim matches the matrix notes row 1168 "self-build S (2017-frozen m3u crate unsuitable)". |
| 3 | Six conformance vectors pinned by named tests | :47-54 | REAL | hls_conformance_e2e.rs:100,140,199,273,336,416 — all six exact fn names present (rg verified). |
| 4 | Fixtures "tests/fixtures/hls/ts_live/ — two real encoding lanes, generation evidence in hls_live_e2e.rs" | :43-44 | REAL | `ls tests/fixtures/hls/` -> adts_vod ts_live ts_vod ...; hls_live_e2e.rs:9 "//! `tests/fixtures/hls/ts_live/` — TWO encoding lanes". |
| 5 | Fuzz targets `fuzz_hls_playlist` + `fuzz_http_headers`, seeds from playlists fixtures committed under fuzz/corpus/<target>/ | :91-101 | REAL | fuzz/fuzz_targets/{fuzz_hls_playlist,fuzz_http_headers}.rs exist; corpus dirs contain fmp4_pl/malformed/multi/pl.m3u8 + chunked_200/cross_format_pl/not_found_404/odd_headers.txt. |
| 6 | "`TimestampMap::pts_for_wallclock_ms` ... now saturates at the signed bounds" + regression test name | :106-117 | REAL | timestamp_map.rs:134 `let delta_ms = wallclock_ms.saturating_sub(self.local_ms);` :135 `saturating_mul`; test at :301 `fn far_from_anchor_wallclock_saturates_instead_of_overflowing()`. |
| 7 | Three interrupted-network scenarios incl. byte-exact resume, typed fetch-failed, live-edge delay | :124-128 | REAL | net_interrupted.rs #[test] x3 (:79 `mid_segment_drop_resumes_byte_exact`, :160 `manifest_reset_fails_typed_with_bounded_backoff`, :318 `live_window_edge_survives_wire_delay`). |
| 8 | Refusals: EXT-X-KEY/BYTERANGE/MAP, ENDLIST live open, lost-window — typed `streaming.hls.*` | :60-63 | REAL | Refusal grammar present: `rg 'streaming\.hls\.' crates/streaming/src/hls/` (e.g. typed refusal kinds in media.rs/live.rs); consistent with matrix row 1431 notes "KEY/BYTERANGE/MAP ... refuse typed (streaming.hls.*)". |
| 9 | "`EXT-X-GAP`: ... honest v0: no gap skip/fence semantics yet" | :54,65-66 | REAL | Test name `gap_tag_is_tolerated_no_gap_semantics_yet` (:416); baseline log line 4237 shows `gap_session_opening_progress ... ignored` — no gap semantics wired. |
| 10 | matrix row network-hls: implemented (library), matrix partial, evidence Linux-only | :8-14 | REAL | feature-matrix.toml:1431 `id = "network-hls"`; :1437 `status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }`; evidence string names the same files/corpus. |
| 11 | "cargo test -p streaming ... lib 173/201 passed, 0 failed, 2 ignored" | :73,130 | REAL-UNPROVEN (numbers) / corroborated | Cannot re-run (cargo budget). Baseline cargo-test-workspace.txt: streaming lib "212 passed; 0 failed; 2 ignored" — same 0-failed/2-ignored shape, count grown past doc dates. |
| 12 | Recorded 60 s sweeps: 112 981 / 160 120 iterations, 0 panics | :95-96 | UNVERIFIED | Run artifacts not in tree; no way to re-run within budget. |
| 13 | Commits d2de397, 56d1a05, fix 01e8ee19 (wave 19) | :15,106 | UNVERIFIED | git forbidden; no tree-side commit metadata available. |
| 14 | "no differential vs VLC: the sandbox has no VLC HLS lane" | :80-82 | REAL | Baseline rc-smoke/mp4-smoke show no VLC in sandbox (audit/runs/baseline/mp4-smoke.md: self-built engine only). |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "matrix row stays partial on every platform" | matrix row 1437: web = "not_started" | Matrix wins: 3 native lanes partial, web not_started. Sloppy wording in the doc ("every platform" vs 3+1); not a status lie. |

## 4. Fabrication / inflation indicators
None. Numbers that ARE checkable (test names, file lists, fixture lanes,
corpus contents, saturation code) all verify exactly; uncheckable numbers
(iteration counts, commit ids) are quarantined as UNVERIFIED rather than
silently trusted.
