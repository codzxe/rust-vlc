# AUDIT: apps/cli/src/containers/mod.rs

| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-22 |
| Lines / functions | 143 / 3 (2 production + 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — this is THE registry: `main.rs` → `run::run` → `lumen_dispatch` → `CliHost::open_session` → `open_source` (demux_host.rs:458) → `containers::open` (this file, line 45). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |

Commands run:
- Full Read of the file; `sed -n '78,92p' apps/cli/src/containers/mod.rs` (quote verification, exit 0)
- `rg -i "asf|wmv|wma" crates/media-probe/src` → **no matches** (no ASF probe signature exists)
- `rg -i "asf" apps/` → one comment only (pipeline_host.rs:322, "ASF futures own that")
- `ls crates/demux/src/asf/fixtures/` → 11 committed fixtures (wma/wmv/asf)
- `rg -n "FormatId::" crates/media-probe/src/signature/` → the registered ids enumerated (wav aiff ogg matroska adts mpegts mpegps ivf avi mp4 mp3 mp2 h264 hevc flac annexb-subthreshold) — exactly the 15 the test names, all armed in this file (verified arm-by-arm)

## 1. One-paragraph verdict

A clean, tiny registry: 15 probe ids → 11 container modules (the es module serves 5 ids; flac is a deliberate typed refusal), one default arm that honestly names any format that probes but has no wiring. The arm count claim in the test ("all 15 registered signature ids now have arms") was verified id-by-id against both the probe registry's FormatId constructors and this file's match arms — it is true. The one real hole is the inverse direction: the tree contains a full ASF/WMV/WMA demuxer (crates/demux/src/asf, with 11 committed fixtures) that has NO probe signature and NO arm — an `.asf`/`.wmv`/`.wma` file answers "probe.no-demuxer" in every shipped app while a complete demuxer for it sits unreachable in the tree. Second tier: nothing mechanically couples the probe registry's id set to this file's arm set (the only default-arm test uses a hypothetical id), so the next signature that lands without an arm will only be caught at runtime — honestly refused, but by accident of the message, not by a tested contract.

## 2. What actually works (verified)

- All 15 registered ids have arms (verified: `adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg mpegps mpegts wav` — mod.rs:52-73 — against the probe registry's `FormatId` constructors in signature/{phase5,es,avi,mp4}.rs).
- The per-family coverage is independently pinned by each container host's own tests (`facts.container` equality), so a mistyped id in an arm fails elsewhere in the suite.
- The default-arm test is well-constructed: it feeds a HYPOTHETICAL id (`mpegps-dv-extended`) and asserts stage, code, message content, and the `format` detail row — and documents why the hypothetical retarget happened when the last real family (mpegps) was wired.

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/containers/mod.rs:50-75 (the arm set) vs crates/demux/src/asf/ + crates/media-probe/src/
Code:
```rust
        "adts" => adts::open(stream, opts),
        "aiff" => aiff::open(stream, opts),
        "avi" => avi::open(stream, opts),
        ...
        other => Err(unwired_refusal(other)),
```
What is wrong: the registry arms 15 probe ids, but the demux crate contains a 12th family — ASF (`crates/demux/src/asf/` with `header.rs`, `guid.rs`, `data.rs`, fuzz + tests + **11 committed fixtures**: `asf_mjpeg.asf`, `wmv_wmv1_64x48.wmv`, `wma_wmav1_st44k.wma`, …) — that has no probe signature (`rg -i "asf|wmv|wma" crates/media-probe/src` → zero matches) and therefore can never reach this registry. An `.asf`/`.wmv`/`.wma` file answers the probe-stage miss ("no demuxer") in every shipped app.
Why it matters: a complete, fixture-backed demuxer is dead code from every shipped entry point — the exact "UNREACHABLE real code" class §5 defines. The registry's docs are honest about what they claim ("every REGISTERED probe id now has an arm") but the practical coverage claim a user would assume — "the containers the tree implements can be opened" — is false for ASF. (Cross-ref: pipeline_host.rs:322's comment shows the pipeline side knows ASF is unwired.)
What correct looks like: either an ASF probe signature + `mod asf;` + arm, or an explicit registry-doc line naming ASF as intentionally unwired.
Confidence: HIGH — Verification: greps above + fixture ls.
Severity: S3 (a whole family unreachable; no doc in this file claims otherwise, and no matrix row was checked here — Wave 18 owns the claim ledger).

### F-002 [S4]
Where: apps/cli/src/containers/mod.rs:94-141 (test module)
Code:
```rust
        let stream = demux::avi::test_support::mem_stream(*b"never read").must();
        match open("mpegps-dv-extended", stream, &opts) {
```
What is wrong: nothing mechanically couples the probe registry's builtin id set to this file's arm set. The only default-arm test uses a hypothetical id; there is no test that iterates `media_probe::ProbeRegistry::with_builtins()` and asserts every emittable id either lands on a wired arm or an INTENTIONAL refusal (flac-style). The next signature that lands in the probe crate without an arm here produces a runtime-only "probes but has no wiring" for a real format.
What correct looks like: a registry-coverage test enumerating the probe builtins against the arm set (the same shape as dispatch.rs's `served_table_matches_the_help_synopsis`).
Confidence: HIGH — Verification: read + absence of such a test confirmed by grep.
Severity: S4 (the failure mode is an honest refusal, not a wrong open).

### F-003 [S4]
Where: apps/cli/src/containers/mod.rs:50-74
Code:
```rust
    match format {
        // The entry shape: `"<probe-id>" => <mod>::open(stream, opts),`
        "adts" => adts::open(stream, opts),
```
What is wrong: stringly-typed dispatch on probe ids — no shared `FormatId`-typed enum between the probe crate and this registry. Mitigated in practice: each container test pins its `facts.container` string, and the ids are short and stable. Noted as the structural reason F-002 needs a test at all.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 45-76 | REAL | S3/S4 | 15 arms verified against probe ids; ASF hole (F-001); stringly dispatch (F-003) |
| `unwired_refusal` | 81-92 | REAL | — | stage `probe`, code `unsupported`, format named + detail row; the D-018-era shape kept deliberately |
| test `unwired_format_is_refused_with_the_format_named` | 114-141 | REAL | S4 | hypothetical-id default-arm pin (F-002); documents the mpegps retarget history honestly |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| unwired_format_is_refused… | YES for the default arm's shape | full error shape asserted; but it cannot catch a MISSING arm for a real id (F-002) — no test can, today |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Wiring ONE container family = … one mod declaration and one match arm — no other shared file is touched" | module docs 5-13 | REAL | the wiring pattern of every audited sibling (ps/es/adts/wav/aiff) matches |
| "Anything not wired falls to `unwired_refusal` … never a silent fallback" | module docs 17-20 | REAL | line 74 + the test |
| "every registered probe id now has an arm" (6.OPW-ps = FINAL unit) | test docs 105-107 | REAL | verified arm-by-arm against the probe registry |
| (inverse, implicit) "the tree's demuxers are openable" | — | FALSE for ASF | F-001 |

## 7. Dead / duplicated / unreachable code

- The default arm is live-but-dormant for real ids (all 15 registered ids are armed) — a deliberate guard, correctly documented.
- The unreachable code this file exposes is elsewhere: crates/demux/src/asf/** (F-001).

## 8. Dependencies & flags

No feature gates. The registry links every container module (avi, ivf, mkv, mp4, ogg, ts — other agents' files — plus the five audited here) and `demux::es::annexb::AnnexbCodec` for the h264/hevc arms.

## 9. Open questions for the next agent

- Does any feature-matrix row claim ASF/WMV/WMA support? If yes it is FALSE end-to-end (no probe signature → cannot even reach the honest refusal). Wave 18.
- avi.rs, ivf.rs, mkv.rs, mp4.rs, ogg.rs, ts.rs (other agents): do any of them contain a second MpegAudioAssembler-style stranded-EOS-queue bug (the ps.rs F-001 class)? ts.rs:373/382 uses the same queue pattern.
- The `annexb` probe id "never reaches the registry — the probe mints it sub-threshold" (mod.rs:60-61): verify the threshold math in media-probe (Wave 1/4 boundary).
