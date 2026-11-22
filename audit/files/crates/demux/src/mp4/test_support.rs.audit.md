# AUDIT: crates/demux/src/mp4/test_support.rs

| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-22 |
| Lines / functions | 570 / 24 (builders + models + delegate) |
| File verdict | REAL (as test-support) — UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO. Ungated in the lib (`mp4/mod.rs:101 pub mod test_support;`), so it **compiles into the production `demux` rlib**, but every caller is test-gated or dev tooling: `mp4/tests.rs` (`#[cfg(test)]`, mod.rs:103-104), `mp4/fuzz.rs:46` (called only by the separate cargo-fuzz crate `fuzz/fuzz_targets/fuzz_demux_mp4.rs:19`), and `apps/cli/src/containers/mp4.rs:637` (inside `#[cfg(test)] mod tests`, mp4.rs:430). Zero calls from any non-test production path — the module doc's claim "nothing in the demuxer itself uses it" (line 5-6) verified TRUE by exhaustive grep. |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:5 (+1 cross-file S2 registered against mp4/mod.rs) |
| Commands run | Read (full); Grep for all callers of every `pub fn`; sed -n verification of every quote; cross-checks into mp4/mod.rs, mp4/tests.rs, apps/cli/src/containers/mp4.rs, fuzz/fuzz_targets/. No cargo (adaptation). |

## 1. One-paragraph verdict

This file is what it says it is: deterministic MP4 byte builders (progressive, cover-art, chapters, fragmented-with-trun) that emit spec-shaped boxes, plus a thin `mem_stream` delegate to the AVI seam. The two-pass stco offset patching and the moof/trun data-offset math are correct (verified by hand-decoding the box layouts below). It is honest about its purpose and the demuxer never touches it. The defects: (1) it is compiled unconditionally into the production library — 570 LOC of fixture-builder API shipped in the `demux` rlib because integration tests and cargo-fuzz cannot see `#[cfg(test)]` items, a real Rust constraint the doc cites, solved elsewhere in the ecosystem with a feature gate this crate does not have; (2) the builder silently inverts intent for all-non-sync tracks (omits `stss`, which per ISO 14496-12 means *all* samples are sync — and the demuxer-side test `absent_stss_means_all_sync` proves the reader takes it that way); (3) `fragmented()` silently fabricates 4-byte `0xAA` samples when `frags × per_frag` exceeds the spec's sample list. Bonus: this audit solved the W00 baseline mystery — the "voe" track language in the MP4 smoke is NOT produced here (this file correctly writes `0x55C4 // "und"`); it is an off-by-one alphabet base in `mp4/mod.rs:536` — see cross-file note CX-1.

## 2. What actually works (verified)

— Box encodings hand-checked against ISO/IEC 14496-12: `ftyp` (major+minor+compat brands), `mvhd` v0 field order, `tkhd` flags=3 (enabled|in_movie), `mdhd` with `0x55C4` ("und", correctly packed: 21/14/4 with ISO's 1-based letter mapping), `stts` run-length, `stsc` (1,1,1) one-sample-per-chunk, `stsz`, `stco`, `stss` 1-based sample numbers, `ctts` v0, `elst` v0, `tref/chap`, `udta/meta/hdlr(mdir)/ilst/covr/data` ladder, `mvex/trex`, `moof/mfhd/traf/tfhd(0x020000 default-base-is-moof)/tfdt/trun(0x0F01 = data-offset+duration+size+flags+cts)`.
— Two-pass offset patching in `progressive*` is length-stable (u32 zeros → u32 reals) and `mdat_payload_start = file.len() + 8` is correct.
— `fragmented`'s `data_offset = moof_box.len() + 8 + track_start[ti]` (line 484) is correct relative to the moof base under default-base-is-moof.
— Grep proof of no production consumer (see reachability field) — the §3.1 question "is test-only code compiled into production paths?" answers: compiled yes, called no.

## 3. Findings

### F-001 [S3] — Test-support surface compiled unconditionally into the production library
Where: crates/demux/src/mp4/mod.rs:99-104 + crates/demux/src/mp4/test_support.rs:3-6
Code (verbatim):
```
pub mod error;
pub mod fuzz;
pub mod test_support;

#[cfg(test)]
mod tests;
```
Code (verbatim, the module's own defense):
```
//! Public like [`crate::wav::test_support`] on purpose: the
//! integration suites + fuzz harness need deterministic shapes. This
//! is test-support surface, not player surface — nothing in the
//! demuxer itself uses it.
```
What is wrong: `pub mod test_support` has no cfg/feature gate, so all 570 LOC of fixture builders compile into the shipped `demux` library. `crates/demux/Cargo.toml` has **no `[features]` section at all** — there is no mechanism by which this could have been gated. The stated reason (integration tests/examples/cargo-fuzz cannot see `#[cfg(test)]` items) is a genuine Rust constraint, but the standard fix (a `test-fixture` feature enabled by dev-dependents and the fuzz crate) was not taken; the same ungated pattern is repeated in avi/wav/ps/asf/ogg/aiff/ivf/ts.
Why it matters: the shipped artifact's public API advertises fixture builders (`demux::mp4::test_support::progressive`), and every release build pays compile time for code that can never execute in it. Binary size impact is probably zero (linker section GC — UNVERIFIED, no toolchain in sandbox).
What correct looks like: `#[cfg(any(test, feature = "test-fixture"))] pub mod test_support;` + `[features] test-fixture = []` + `dev-dependencies`/fuzz crate opting in.
Confidence: HIGH — Verification: read + Grep (all callers enumerated)

### F-002 [S3] — Builder silently inverts "no sync samples" into "all sync samples"
Where: crates/demux/src/mp4/test_support.rs:231-232 (condition) + 224-238
Code (verbatim):
```
        if !syncs.is_empty() && syncs.len() < spec.samples.len() {
            let mut stss = Vec::new();
```
What is wrong: a `TrackSpec` whose samples are ALL `sync: false` produces an empty `syncs` list → the `stss` box is omitted → per ISO/IEC 14496-12, an absent stss means **every sample is a sync sample**. The fixture then asserts the opposite of the spec's intent, and any test written against it passes against the wrong file shape. The reader side proves the semantics are live: `mp4/tests.rs:214 fn absent_stss_means_all_sync()` pins "absent stss ⇒ all keyframes".
Why it matters: this is a fixture-correctness trap in the one module whose entire job is fixture correctness; a future seek/boundary test using `syncs=&[false; n]` would test the all-sync path while believing it tests the no-sync path.
What correct looks like: write `stss` with `entry_count = 0` when `syncs.is_empty() && !spec.samples.is_empty()`, or assert/panic at build time on the unrepresentable spec.
Confidence: MEDIUM-HIGH — Verification: read (ISO semantics + the reader-side test cited; no negative case currently in mp4/tests.rs — grep `sync: false` = 0 matches)

### F-003 [S4] — `fragmented` silently fabricates samples past the spec
Where: crates/demux/src/mp4/test_support.rs:434-437 (and the matching `map_or(1024, …)` at 460/496/510)
Code (verbatim):
```
                let bytes = t
                    .samples
                    .get(idx)
                    .map_or(vec![0xAA; 4], |s| s.bytes.clone());
```
What is wrong: when `frags * per_frag > t.samples.len()`, the builder invents 4-byte `0xAA` payloads with duration 1024 instead of failing. A test author who miscounts gets packets that exist in the file but not in their spec.
Confidence: HIGH — Verification: read

### F-004 [S4] — Dead write in the stts builder
Where: crates/demux/src/mp4/test_support.rs:188-198
Code (verbatim):
```
        // stts: one run per distinct delta.
        let mut stts = Vec::new();
        stts.extend_from_slice(&u32b(1)); // patched below per-run
```
…then line 198: `        stts.clear();`
What is wrong: the `u32b(1)` entry-count write is immediately discarded by `clear()`; the comment says "patched below" but the code rebuilds from scratch. Correct output, dead code on the way there.
Confidence: HIGH — Verification: read

### F-005 [S4] — Full duplicate moof build to measure a length
Where: crates/demux/src/mp4/test_support.rs:446-469 (moof2, discarded) vs 481-504 (moof3, the real one)
What is wrong: `moof2` is built in full only so `moof_box.len()` (line 469/484) can size the data offsets; `moof3` then rebuilds the identical bytes with the offsets filled. Wasteful and a divergence hazard (two copies of the trun field order that must stay in sync — today they do).
Confidence: HIGH — Verification: read

### F-006 [S4] — Silent u32 truncations throughout the builders
Where: crates/demux/src/mp4/test_support.rs:44 (boxb size `as u32`), 87/103/123 (mvhd/tkhd/mdhd duration `as u32`), 172 (elst media_time `[4..8]`), 221 (stco `*o as u32`), 244 (ctts `s.cts as u32` — a negative cts in a v0 ctts wraps to a huge positive), 453/489 (tfdt `dts as u32`)
What is wrong: every field that a real muxer widens to 64-bit (v1 boxes, co64) is silently narrowed. At current fixture scales (KB-sized files, small dts) nothing truncates; a test that grows past 4 GiB or sets `cts: -1024` gets a corrupt fixture with no error. `chapter_track`'s 16-bit length prefix (line 530) has the same shape.
Confidence: HIGH — Verification: read

### F-007 [S4] — Dead public API: `chap_ref()` has zero callers
Where: crates/demux/src/mp4/test_support.rs:543-548
Code (verbatim):
```
/// The `tref/chap` reference bytes for a chapter track id (kept
/// public for tests that splice their own shapes).
#[must_use]
pub fn chap_ref(track_id: u32) -> Vec<u8> {
    boxb(b"tref", &boxb(b"chap", &u32b(track_id)))
}
```
What is wrong: grep across the repo finds no caller — "kept public for tests" describes tests that do not exist. Dead code in an already-ungated public module.
Confidence: HIGH — Verification: Grep (exhaustive)

### CX-1 [S2, cross-file — registered against crates/demux/src/mp4/mod.rs, not this file] — W00's "voe" language mystery solved
Where: crates/demux/src/mp4/mod.rs:534-539
Code (verbatim):
```
/// Unpack the mdhd 3×5-bit language code; `None` for "und".
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}
```
What is wrong: ISO/IEC 14496-12 packs each letter as its 1-based alphabet index (1='a'); this decoder uses a 0-based base (`b'a' + n`), so every letter decodes one too high. The "und" this file's own builder writes (`test_support.rs:124 b.extend_from_slice(&u32b(0x55C4)); // "und"`, packed 21/14/4) decodes as 21+1→'v', 14+1→'o', 4+1→'e' = **"voe"** — exactly the odd track language W00's MP4 smoke observed. The `tag == "und"` sentinel can therefore never fire for a genuine "und" file. No test pins language decoding (grep for language assertions in mp4/tests.rs: none).
Why it matters: every MP4/MOV track with a declared mdhd language reports a wrong language to the user; the correct mapping is `b'a' + (n - 1)` (or reject n=0). This is an S2 in the mp4/mod.rs file (wrong behavior in realistic cases); filed here because the orchestrator asked whether test_support/chapters explain the odd metadata — it does not cause it, it merely proves it.
Confidence: HIGH — Verification: read + arithmetic derivation of "voe" from 0x55C4

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| struct SampleSpec | 17-23 | REAL | — | plain model |
| struct TrackSpec | 26-39 | REAL | — | carries edit + chap_ref hooks; all fields honored in trak() |
| boxb | 42-48 | REAL | S4 | `as u32` size cast (F-006) |
| fullbox / fullbox_f | 51-65 | REAL | — | correct 24-bit flags serialization |
| u32b | 67-69 | REAL | — | trivial |
| ftyp | 71-80 | REAL | — | major+minor+5 compat brands; duplicates major — harmless |
| mvhd | 82-95 | REAL | S4 | v0 only; duration `as u32` (F-006); next_track_id 0xFFFFFFFF fine |
| tkhd | 97-116 | REAL | — | flags=3 correct; duration cast noted |
| mdhd | 118-127 | REAL | — | 0x55C4 "und" packed CORRECTLY (contrast CX-1) |
| hdlr | 129-136 | REAL | — | predefined=0, handler, empty name |
| stsd_entry_with | 143-150 | REAL | — | audio-shaped 28-byte v0 base for all handlers — admitted in comment; parser only needs size/kind |
| trak | 155-261 | REAL | S3/S4 | stss inversion (F-002), stts dead write (F-004), ctts/stco casts (F-006); tkhd duration = Σdur correct |
| udta_cover | 266-283 | REAL | — | meta as FullBox (version/flags zero) — the QuickTime variant; hdlr 'mdir' correct |
| moov_body | 288-301 | REAL | — | mvhd(1000, 0) — movie duration 0, simplification OK for tests |
| moov_body_with_udta | 304-308 | REAL | — | appends udta |
| progressive | 313-315 | REAL | — | delegates |
| progressive_with_entry_children | 323-356 | REAL | — | two-pass offsets correct; zero-offset pass length-stable |
| progressive_with_cover | 361-402 | REAL | — | duplicate of the two-pass logic in progressive_with_entry_children (copy-paste shape, S4-adjacent) |
| fragmented | 408-515 | REAL | S4 | data_offset math correct; fabricates samples (F-003); double moof build (F-005) |
| chapter_track | 519-541 | REAL | — | QuickTime text sample (u16 len + text); handler 'text' |
| chap_ref | 546-548 | DEAD | S4 | zero callers (F-007) |
| progressive_with_chapters | 554-560 | REAL | — | hard-codes chapter track id 2 — matches builder's i+1 numbering; documented |
| mem_stream | 566-570 | REAL | — | delegate to avi seam; used by fuzz harness + tests |

## 5. Test honesty

This file contains no tests; it IS the test machinery. The honesty questions and answers:
- Do the builders produce what consuming tests claim? Verified for box grammar (see §2). The mp4/tests.rs suite (70+ calls to these builders) runs in the W00 baseline (1751 pass), so the fixtures do parse — UNVERIFIED per-test, no cargo here.
- Circular-proof risk: none of the builders is generated by the code under test; they are independent hand-written encoders — good (no golden files).
- The F-002 stss inversion is the one spot where a builder can encode the opposite of the spec's meaning — today no test trips it (grep `sync: false` in mp4/tests.rs: 0 hits).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This is test-support surface, not player surface — nothing in the demuxer itself uses it" | test_support.rs:5-6 | REAL | exhaustive grep: demuxer modules never reference test_support; only tests.rs / fuzz.rs / CLI cfg(test) / examples |
| "The builders emit the layouts ffmpeg's muxers produce (two-pass offset patching, one-chunk-per-sample stsc, mvex/trex defaults)" | test_support.rs:8-10 | REAL-UNPROVEN | stsc (1,1,1) and two-pass stco verified by read; "same as ffmpeg" is a claim about ffmpeg I cannot execute against (UNVERIFIED, no ffmpeg in sandbox) |
| "Public like crate::wav::test_support on purpose" | test_support.rs:3-4 | REAL | same ungated pattern confirmed in avi/wav/ps/asf/ogg/aiff/ivf (grep) — a crate-wide decision, not an accident |

## 7. Dead / duplicated / unreachable code

- `chap_ref` (F-007): dead.
- `progressive_with_cover` duplicates the two-pass offset machinery of `progressive_with_entry_children` instead of parameterizing it — duplication, must-be-kept-in-sync.
- The whole module is unreachable from the shipped CLI/tauri binary (see header) — by design, but ungated (F-001).

## 8. Dependencies & flags

- No feature flags exist in `crates/demux/Cargo.toml` (no `[features]` section) — nothing gates this code, nothing could. Dependencies: `plugin_api::SourceStream`, `std::sync::Arc`, `media_types::EngineError` (via the avi delegate). All in-crate/workspace.
- The cargo-fuzz crate `fuzz/Cargo.toml:111-112` builds `fuzz_demux_mp4` against this module — dev tooling only, not in the release workflow's shipped artifacts (release workflow audited by wave 17).

## 9. Open questions for the next agent

- **mp4/mod.rs agent: adopt CX-1** (`unpack_language` off-by-one → every mdhd language one letter high; "und"→"voe"; the `tag == "und"` sentinel dead for real und files). W00 smoke observed it; this audit derived the exact mechanism.
- mp4/tests.rs agent: does any test construct a track with all `sync: false` (F-002 trap)? Today none does.
- Wave 17: does the release build strip dead pub fns (linker GC), i.e., does F-001 cost only compile time? UNVERIFIED here (no toolchain).
- Why does `demux` have NO `[features]` section at all — is the ungated-test-support pattern a workspace-wide policy recorded anywhere (wav/test_support.rs:4 and avi/test_support.rs:6 carry the same rationale comments)?
