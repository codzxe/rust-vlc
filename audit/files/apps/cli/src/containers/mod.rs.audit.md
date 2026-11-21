# AUDIT: apps/cli/src/containers/mod.rs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-21 |
| Lines / functions | 142 / 3 (dispatch fn, refusal fn, 1 test; plus 11 mod declarations) |
| File verdict | REAL — the dispatch is complete and honest; every claim I could falsify (15 ids/15 arms, the sub-threshold `annexb` story, the honest default arm) verified against the probe registry; one doc-order drift only |
| Reachable from shipped entry point? | YES — `lumen` → run/dispatch → `demux_host::open_source` → `containers::open` (this file, :45) on every media open. Baseline: every container smoke/test lane flows through it. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `Read` mod.rs 1-142 (full); `rg -n "with_builtins" -A 45 crates/media-probe/src/signature/mod.rs` (14 builtin plugins: Avi Mp4 Wav Aiff Ogg Mkv Adts Ts Ps Ivf Mp3 Mp2 Annexb Flac — Annexb emits `h264`/`hevc` codec ids → 15 ids, matching the 15 match arms); `sed -n '455,520p' crates/media-probe/src/signature/es.rs` (the `annexb` family id is minted at SCORE_FAMILY_ONLY=50, below the default threshold 60 — "never reaches the registry" verified by the signature's own test); `grep -n "containers::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (`unwired_format_is_refused_with_the_format_named ... ok` at :2834; adts/mp4/mkv/aiff lane suites green around it); `rg -n "probe_stream\|ProbeConfig" apps/cli/src/demux_host.rs` (:444-446 default config, threshold default). |

## 1. One-paragraph verdict
This 142-line registry is the open path's single dispatch and it does exactly what it advertises: 11 container modules, 15 probe-id arms covering every id the builtin probe registry can mint (verified: 14 signature plugins, the AnnexB plugin discriminating `h264` vs `hevc`), and an honest typed default arm that names the format and refuses at the probe stage — pinned by a test that ran green in the baseline. The comment claiming the `annexb` family id "never reaches the registry" is not hand-waving: the signature's own test proves the family rung scores 50 against the default threshold of 60, so ambiguous NAL shapes die at the probe with the family name riding the error, never reaching this match. The only defect is cosmetic: the module doc prescribes a §5 wiring-unit order (`wav → aiff → adts → …`) that matches neither the mod declarations nor the match arms, both of which are in different near-alphabetical orders — a doc that describes a filing convention the file does not follow.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2834` → `containers::tests::unwired_format_is_refused_with_the_format_named ... ok`, and the container lane suites around it (:2774-2790 adts, mp4, mkv, aiff) — the dispatch and its default arm are executed.
— Verified by read + count: 15 probe ids (`adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg mpegps mpegts wav`) ↔ 15 match arms (:52-73) — the "every registered probe id now has an arm" claim in the test's doc (:105-107) is true against `ProbeRegistry::with_builtins()` (signature/mod.rs:130-146).
— Verified by read: the `annexb` sub-threshold claim (:60-61) — es.rs:457-490 proves the family-only hit scores 50 (`SCORE_FAMILY_ONLY`), confidence does not meet the default 60, and the probe errors with `probe.no-demuxer` carrying `annexb` as best-known info; `demux_host.rs:445` uses `ProbeConfig::default()`.

## 3. Findings

### F-001 [S4]
Where: apps/cli/src/containers/mod.rs:13-15 (doc) vs 22-32 (mods) and 52-73 (arms)
Code:
```rust
//! points and are trivial one-line re-merges). Entries keep the §5
//! unit order: wav → aiff → adts → ogg → ivf → mp4 → mkv → ts → ps →
//! es (`avi` is wired here as the moved first vertical).
```
What is wrong: The declared ordering rule is followed by neither list. The mod declarations (:22-32) are alphabetical (`adts aiff avi es ivf mkv mp4 ogg ps ts wav` — wav LAST); the match arms (:52-73) run `adts aiff avi ivf matroska flac hevc h264 mp2 mp3 mp4 ogg mpegps mpegts wav` — neither the §5 unit order (wav first) nor strictly alphabetical (the `es` arms sit after `matroska`).
Why it matters: The doc's stated collision-confinement convention ("entries keep the §5 unit order") is the merge-conflict guidance for exactly this file; a contributor following it produces a third ordering. Zero behavioral impact.
What correct looks like: pick one convention (alphabetical by probe id is what the file almost does) and say that.
Confidence: HIGH — Verification: read of all three sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` (dispatch) | 45-76 | REAL | — | 15 arms + default; `es` module serves 5 ids via three constructor fns (`open_flac`, `open_annexb` ×2 codecs, `open_mpeg_audio` ×2 formats); the format string is passed through where the codec id matters |
| `unwired_refusal` | 81-92 | REAL | — | Stage `probe`, code `Unsupported`, format named in message AND `.with_detail("format", …)`; the D-018-era message shape kept deliberately |
| `unwired_format_is_refused_with_the_format_named` (test) | 114-141 | REAL | — | Executed (baseline :2834); uses a honestly-labeled HYPOTHETICAL id (`mpegps-dv-extended`, no probe emits it) after the last real family was wired; asserts stage/code/message/detail; `Ok(_) => unreachable!` makes an accidentally-matching arm a failure; the "never read" stream claim is weakly enforced (a 10-byte mem stream that would error differently if read) but the arm-catch assertion carries the test |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unwired_format_is_refused_with_the_format_named | YES | Four hard asserts on the typed refusal's shape; any arm accidentally matching the id hits `unreachable!`. Its own doc honestly re-targets to a hypothetical id since all real ids gained arms — the test now pins the DEFAULT arm only, which is the correct remaining scope (the per-family lanes are pinned by their own modules' tests and the integration suites). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the open path's single format dispatch, consulted after the probe names a container" (1-3) | mod.rs | REAL | demux_host.rs:444-461 probe → `containers::open` (my host.rs#part2 trace) |
| "one `mod <fmt>;` declaration and one match arm here — no other shared file is touched" (10-11) | mod.rs | REAL (structure) | 11 mods, 15 arms, no other registry edits visible; per-family opens confirmed in my avi/aiff/mkv audits |
| "Anything not wired falls to `unwired_refusal` — … exit 65" (17-18) | mod.rs | REAL | Test asserts stage/code; exit 65 seen in unsupported_path.rs lane tests (mpegps stub refusal ran `code == 65`) |
| "**all 15 registered signature ids** … now have arms" (105-107) | mod.rs test doc | REAL | Counted against `with_builtins()` (14 plugins, annexb→h264+hevc) — 15/15 |
| "the `annexb` family id never reaches the registry — the probe mints it sub-threshold" (60-61) | mod.rs | REAL | es.rs:457-490: family rung scores 50 < default threshold 60; probe errors with the family name on the error, not a hit |
| "Entries keep the §5 unit order: wav → aiff → adts → …" (13-15) | mod.rs | FALSE | Neither the mod list nor the arms follow it (F-001) |

## 7. Dead / duplicated / unreachable code
None. Every arm dispatches to a module audited or assigned in this wave; the default arm is live and tested.

## 8. Dependencies & flags
Workspace-only (commands, plugin_api, demux test_support in the test). No feature gates. One `lumen-allow-literal` waiver (:116) on a test URI — legitimate.

## 9. Open questions for the next agent
- W13 es.rs / demux_host owners: if a host ever lowers `ProbeConfig`'s threshold below 50, `annexb` becomes a winning id and hits the default arm's refusal — acceptable (honest no), but the message "R3 registered the signature … has not landed" would be misleading for a family the `es` module can actually serve (`open_annexb`). A one-line `"annexb" => es::open_annexb(Hevc, …)` style family arm (or a threshold guard) would close it.
