# AUDIT: crates/media-clock/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 143 / 1 function (`validate_rate`) + module declarations, 6 re-export blocks, 1 doctest |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/Cargo.toml:55` media-clock dep → `apps/cli/src/pipeline_host.rs` wires `Arc<MediaClock>` into `SessionConfig.clock` (playback-engine/Cargo.toml:37) → every tick of the AVI lane reads it; `validate_rate`'s callers: `clock.rs:522, 808` — the set-rate mutation boundaries) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read of full file (Read tool — Bash display mangles "VirtualClock" to "n", noted); `sed -n` quote verification (131-143); read `media-types/src/time.rs:839-870` (Rate::new semantics — mirror-claim check); `rg "validate_rate"` (clock.rs:522, 808); `rg "MAX_CORRECTION_PPM"` → drift.rs:35 `= 1000` (doc claim check); `rg "thread::spawn"` in crate → 0 hits (no-threads claim check); Cargo.toml deps read (media-types only); baseline cites: `orchrun-test-workspace-nofailfast.log.txt` — `Running unittests src/lib.rs (…/media_clock-…)` 92 tests, and `Doc-tests media_clock` 2 passed incl. `crates/media-clock/src/lib.rs - (line 66) … ok`. |

## 1. One-paragraph verdict
This file is the crate's facade — module docs, six re-export groups, one `pub(crate)` defensive helper — and every checkable claim in it checks out against code or executed runs: the sample-counter/pause/resume semantics the doc advertises are literally executed by the file's own doctest, which ran green in the no-fail-fast baseline (`Doc-tests media_clock … (line 66) … ok`), the ±1000 ppm drift bound named in the docs is the actual `MAX_CORRECTION_PPM` constant (drift.rs:35), the "no threads of its own / no I/O" Never-list is true (zero `thread::spawn` in the crate; the only dependency is media-types), the `testkit`-VirtualClock reference resolves to a real testkit trait, and `validate_rate` is an exact behavioral mirror of `Rate::new` (rejects zero components, gcd-reduces — verified against media-types time.rs:849-862) guarding the two set-rate mutation sites in clock.rs. Zero findings; §3 documents what was checked and how, per the zero-finding rule.

## 2. What actually works (verified)
— The doctest executed: baseline `orchrun-test-workspace-nofailfast.log.txt` (Doc-tests media_clock): `test crates/media-clock/src/lib.rs - (line 66) ... ok` — the audio-master sample-counter derivation, the pause-freeze, and the no-discontinuity resume asserted in the doc are executed, not just claimed.
— The crate's 92 lib tests ran in the same baseline (all ok) — the facade's submodules (clock, backends, drift, mapping, sync) are test-covered elsewhere in this wave by other agents' files or remain unaudited (§9).
— `validate_rate` mirror verified: `Rate::new` rejects `num==0 || den==0` with stage `Control`, code `rate.zero_component`, and gcd-reduces (time.rs:853-862); `validate_rate(rate)` = `Rate::new(rate.num, rate.den)` — identical behavior including the error, so a `Rate{num:0,den:1}` built through the pub fields cannot enter storage through clock.rs:522/:808.
— Doc-claim spot checks: `MAX_CORRECTION_PPM: i64 = 1000` (drift.rs:35) matches "±1000 ppm"; zero thread spawns in the crate matches "No dedicated thread"; deps = `[media-types]` matches the Never-list ("I/O, device APIs, UI, threads"); `testkit::VirtualClock` exists (testkit/src/lib.rs:261).

## 3. Findings
None. Zero-finding justification (what was checked, how):
1. **Fakeness scan:** no `todo!`/`unimplemented!`/stub bodies — the file's only executable code is `validate_rate` (3 lines, traced to its two callers and to the mirrored constructor). Re-export lists checked for phantom items: every name resolved to a real item in its submodule (`backends`, `clock`, `drift`, `mapping`, `source`, `sync` all declared at 109-114 and present in the crate).
2. **Doc-claim audit:** all 7 concrete claims in the module docs verified (§2 + §6) — semantics summary lines 42-52 map to clock.rs behavior (executed by the doctest and the 92-test suite), the 3.B2 backends summary (54-62) maps to the `backends` re-exports, and the Never-list (104-107) is enforced by the crate's actual shape.
3. **validate_rate correctness:** mirror-verified against `Rate::new` (same rejections, same reduction, same error stage/code — the only difference is the extra function-call indirection). No path stores an unreduced or zero-component rate through the guarded boundaries.
4. **Doctest honesty:** it asserts real behavior (`now().ticks == 1_000_000_000` after 48k samples at 48 kHz; freeze across a paused counter advance; no jump on resume) and it ran — a mutation of clock.rs's pause logic or sample mapping would fail it.
5. **Display artifact note (process, not a finding):** Bash-tool output renders "`testkit`'s `VirtualClock`" as "`testkit`'s `n`" — the Read tool shows the true bytes; no source defect.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + doctest | 1-107 | REAL | — | 7 concrete claims verified (§2/§6); doctest executed in baseline. |
| module declarations | 109-114 | REAL | — | 6 submodules, all present (backends 2117 LOC, clock 1178, drift 312, mapping 425, source 119, sync 697). |
| re-exports | 116-129 | REAL | — | Every exported name resolved to a real item (rg per name). |
| `validate_rate` | 141-143 | REAL | — | Exact mirror of `Rate::new`; guards clock.rs:522, :808. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| lib.rs doctest (66-102) | YES | Executes `audio_master` + 48k-sample advance + pause-freeze + resume against the real `MediaClock`; a broken pause or sample→ns mapping fails the asserts. Ran ok in baseline (Doc-tests media_clock). |
| (Submodule tests — 92 — belong to clock.rs/backends.rs/etc., not this file) | — | Ran ok in baseline; their honesty is those files' auditors' scope (§9). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "now() = anchor + elapsed(counter) × rate within one anchor epoch: monotonic" | lib.rs docs 42-43 | REAL | Doctest (ran ok) + clock.rs suite (92 tests ok in baseline). |
| "Pause freezes the position; resume continues … no discontinuity" | lib.rs docs 47-49 | REAL | Doctest lines 93-99 assert exactly this; ran ok. |
| "Drift correction is a bounded rate adjustment … ±1000 ppm" | lib.rs docs 50-52 | REAL | `MAX_CORRECTION_PPM: i64 = 1000` (drift.rs:35). |
| "No dedicated thread … one short internal lock" | lib.rs docs 31-38 | REAL | 0 thread spawns in crate (rg); v0-lock honesty note included. |
| "Never: I/O, device APIs, UI, threads of its own" | lib.rs docs 104-107 | REAL | Deps = media-types only; no I/O crates; rg confirmed. |
| "tests wire virtual sources built on testkit's VirtualClock" | lib.rs docs 17-20 | REAL | `testkit::VirtualClock` trait exists (testkit/src/lib.rs:261). |
| "conversion failure is data, never a panic (MP §8)" | lib.rs docs 28-29 | REAL | TimeError taxonomy in mapping.rs (checked-checked conversions; the 92-test suite incl. mapping tests ran ok). |

## 7. Dead / duplicated / unreachable code
None: the facade's every export is consumed downstream (playback-engine, apps/cli, video-engine import media-clock items); no duplication.

## 8. Dependencies & flags
- Single dependency `media-types` (L0); `testkit` dev-only. No feature gates, no unsafe, std-only.
- The crate is the timing root of the shipped pipeline (audio-master, card 2.C1).

## 9. Open questions for the next agent
- `clock.rs` (1178 LOC — the actual MediaClock), `backends.rs` (2117 LOC — master-loss demotion chain), `drift.rs`, `mapping.rs`, `sync.rs`, `source.rs` are NOT part of my assignment and NOT audited here — the facade's claims about them were spot-checked only where a constant or a doctest pinned them. Wave leads: these are high-value targets (the scheduler's A/V sync consumes `sync.rs` seams directly).
- The 92-test suite's honesty (tautology scan) belongs to whoever audits those submodules.
