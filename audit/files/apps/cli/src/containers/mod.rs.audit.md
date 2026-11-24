# AUDIT: apps/cli/src/containers/mod.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2025-06-27 |
| Lines / functions | 142 / 2 production fns + 1 test |
| File verdict | REAL — the container-host dispatch registry; every arm verified against the builtin probe's minted format ids (15/15 wired, cross-checked by me against crates/media-probe sources) |
| Reachable from shipped entry point? | YES: `lumen open` → `apps/cli/src/run.rs` → `lumen_dispatch` (SessionOpen) → `apps/cli/src/demux_host.rs:446-458` (`probe_stream` → `report.format`) → `crate::containers::open(&container, stream, &opts)` (demux_host.rs:458) → this file's `open` (mod.rs:45-76). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg callers (demux_host.rs:458); rg/sed over crates/media-probe/src/signature/mod.rs:130-144 (with_builtins: 14 signatures), report.rs FormatId constants, es.rs/phase5.rs/avi.rs/mp4.rs minted-id assertions — the 15-arm completeness claim verified against source; sed quote verification |

## 1. One-paragraph verdict
A 30-line match statement doing exactly what its docs claim: the open path's single format dispatch. I independently verified the central claim — "all 15 registered signature ids now have arms" — by enumerating the builtin probe registry (media-probe/src/signature/mod.rs:130-144 registers 14 signatures; the Annex-B signature mints two ids, h264/hevc, per its own tests at es.rs:436,451) and the FormatId constants (report.rs:49-84: wav, aiff, matroska, adts, mpegts, mpegps, ivf; plus mp4/avi/ogg/mp3/mp2/flac asserted in signature tests): 15 ids, 15 arms, zero mismatch. The default arm (`unwired_refusal`) is production-unreachable today — the CLI's probe registry is hardcoded to builtins (demux_host.rs:446 `ProbeRegistry::with_builtins()`), so no real id can fall through; it is kept as a deliberate, documented guard and its test exercises it through a self-declared hypothetical id. Nothing here is fake.

## 2. What actually works (verified)
— verified by static cross-check: rg `with_builtins` → mod.rs:130-144; the 15 minted ids (`adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg mpegps mpegts wav`) each have an arm at mod.rs:52-73. Baseline proof of the dispatch working end-to-end: mp4-smoke.md (AGENT-SPEC §13 — `lumen open` H.264+AAC MP4 walks the real state machine through this very arm) and rc-smoke headless-decode AVI/MJPEG/PCM (the `avi` arm). Cited, not re-run.

## 3. Findings
F-001 [S4]
Where: apps/cli/src/containers/mod.rs:74, 81-92
Code:
```
        other => Err(unwired_refusal(other)),
    }
}
```
and
```
        format!(
            "format `{format}` probes but has no open-path wiring yet \
             (R3 registered the signature; the container-host registry wires \
             one family per unit — this one has not landed)"
        ),
```
What is wrong: the default arm is dead in production — the only caller builds a builtin-only registry (demux_host.rs:446 `ProbeRegistry::with_builtins()`), every builtin id is wired (verified above), and no plugin lane feeds this path, so no real input can reach `unwired_refusal`. The doc frames it as "kept forever" — a deliberate guard — and the test (mod.rs:105-113) admits it must use "a clearly HYPOTHETICAL one" id to exercise it. This is honest dead code, recorded as hygiene, not fakeness. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open | 45-76 | REAL | — | 15 arms, one per minted probe id (verified); es family shares openers correctly (mp2/mp3 pass `format` through; h264/hevc pass the codec) |
| unwired_refusal | 81-92 | REAL (guard) | S4 | typed Probe/Unsupported with format detail; production-unreachable today (F-001) |
| tests::unwired_format_is_refused_with_the_format_named | 114-141 | REAL | — | pins stage/code/message/detail via hypothetical id; honest about why |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unwired_format_is_refused_with_the_format_named | YES | asserts Stage::Probe + ErrorCode::Unsupported + both message fragments + the `format` detail; deleting or re-shaping the default arm fails it. It cannot catch a MISSING arm for a future real id (by construction — hypothetical id). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "all 15 registered signature ids ... now have arms" | containers/mod.rs:105-107 | REAL | verified: media-probe with_builtins (mod.rs:130-144) + FormatId constants (report.rs:49-84) + minted-id test assertions (es.rs:339-451, phase5.rs:164-187) = 15 ids ↔ 15 arms |
| "Anything not wired falls to unwired_refusal — the honest typed refusal ... kept forever" | containers/mod.rs:17-20 | REAL (design) | default arm present and test-pinned; currently unreachable in production (F-001) |
| "the `annexb` family id never reaches the registry — the probe mints it sub-threshold" | containers/mod.rs:59-61 | CONSISTENT | es.rs:516 asserts minted ids are only `h264|hevc|annexb` and arms exist for h264/hevc; "annexb" itself has no arm — matches the comment |

## 7. Dead / duplicated / unreachable code
- `unwired_refusal` + the `other` arm: production-dead, deliberate guard (F-001).

## 8. Dependencies & flags
- Deps: commands (CommandError), plugin_api (SourceStream), crate demux_host types, demux (es::annexb::AnnexbCodec). No features, no unsafe. 11 `mod` declarations, one per family file — the append-point collision rule described in the docs is structurally visible.

## 9. Open questions for the next agent
- The CLI probe registry is `with_builtins()` only (demux_host.rs:446) while the event catalog has PluginsLoaded/PluginsFailed — is a plugin-registered signature EVER consulted on the open path? If not, the default arm's "future R3 row" story is the only remaining reachability, and plugin signatures are decorative on this lane (demux_host.rs owner should answer).
