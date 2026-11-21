# AUDIT: crates/demux/src/es/hevc.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Lines / functions | 30 / 3 (EXTENSIONS, probe, open) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — the shipped route for raw HEVC is: media-probe annexb signature (`signature/es.rs:281`, extensions `["264","h264","265","h265","hevc"]`) → probe id `hevc` → `apps/cli/src/containers/mod.rs:63` → `containers/es.rs:72,186` constructs `demux::es::annexb::AnnexbVideoDemuxer::new(AnnexbCodec::Hevc, …)` **directly** — this module's `open`/`probe` are bypassed. Sole consumers: `crates/demux/src/es/fuzz.rs:72,81` (in-crate fuzz harness — executed, see §2). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read; `Grep "es::hevc"` workspace (fuzz.rs only); `rg -n "hevc|annexb" apps/cli/src/containers/es.rs` (the real construction path); `sed -n '270,285p' crates/media-probe/src/signature/es.rs` (probe extension list); baseline `rg -n "test es::"` (fuzz + lane tests green) |

## 1. One-paragraph verdict
The HEVC twin of `es/aac.rs`: a 30-line registry-surface alias over the shared
Annex-B engine — `probe`/`open` delegate to `super::annexb` with
`AnnexbCodec::Hevc`, `EXTENSIONS = ["h265","265","hevc"]`. The delegation is honest
and the doc correctly defers the access-unit boundary rule to the annexb module
(VPS/SPS/PPS types 32/33/34, `first_slice_segment_in_pic_flag`) — and unlike the AAC
alias, this one has real executed evidence: the in-crate fuzz harness calls
`hevc::probe`/`hevc::open` and its survival tests are green in the baseline. But the
shipped binary still never touches it (the CLI constructs the Annex-B demuxer
directly), and the extension registry is now declared in three places
(`es::hevc::EXTENSIONS`, `es::h264::EXTENSIONS`, and media-probe's combined
`["264","h264","265","h265","hevc"]` list) with nothing tying them together.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt` lines 1678-1679: `es::fuzz::tests::empty_and_marker_inputs_survive ... ok`, `hostile_wide_inputs_survive ... ok` — these call `super::hevc::probe(head)` and `drain_lane(super::hevc::open(stream))` (es/fuzz.rs:72,81): the alias functions execute under hostile input without panic.
— verified by baseline line 1685: `es::tests::hevc_aus_group_on_first_slice_flag_and_vps ... ok` — the HEVC lane behavior this module fronts is tested at the annexb level.
— verified by read: `open`'s doc Errors section matches `AnnexbVideoDemuxer::new`'s actual refusal set (unknown-length/unseekable/empty/no-start-code — consistent with `SourceReader::new` + the open-scan window).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/es/hevc.rs:11
Code:
```rust
/// File extensions this demuxer claims (the R3 registry row).
pub const EXTENSIONS: &[&str] = &["h265", "265", "hevc"];
```
What is wrong: The HEVC extension registry is declared in three independent places:
here (`["h265","265","hevc"]`), `es::h264::EXTENSIONS` (the H.264 subset), and
media-probe's annexb signature (`["264","h264","265","h265","hevc"]`,
signature/es.rs:281) — the layer that actually routes files. Nothing enforces
consistency; a future extension added to the probe (or a rename here) silently
diverges. The shipped open path (`containers/es.rs:186`) bypasses this module
entirely, so the "R3 registry row" claim is documentation-only for the shipped
binary.
Why it matters: Same dual/triple-registry drift risk recorded for `es/aac.rs`
(this session) — the pattern is per-format and systemic: every `es/*` alias module
re-declares a subset of the probe layer's list.
What correct looks like: One canonical list (the probe layer's), with the es
modules referencing it (`pub const EXTENSIONS: &[&str] = &media_probe…` is a
layering violation — so instead: delete the per-module lists, or generate them from
a single source in an xtask check like `cargo xtask fixtures --check`).
Confidence: HIGH — Verification: read + rg (all three declaration sites verified)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `EXTENSIONS` | 11 | REAL (data) | F-001 | one of three independent declarations |
| `probe` | 16-18 | REAL | — | exact delegate to `annexb::probe(Hevc, ..)`; executed by fuzz (baseline:1678-1679) |
| `open` | 26-30 | REAL | — | exact delegate to `AnnexbVideoDemuxer::new(Hevc, ..)`; executed by fuzz drain_lane |

## 5. Test honesty
No tests in this file. Its surface is exercised by `es::fuzz` (green, real) and the
annexb-level lane test `hevc_aus_group_on_first_slice_flag_and_vps` (green). The
delegate bodies are one line each — nothing to mutate without breaking the annexb
module's own tests.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the `.265` file-access surface over the shared Annex-B engine" | module doc 1-3 | REAL as a description of intent / UNREACHABLE as a shipped path | containers/es.rs:72,186 constructs the engine directly |
| "the R3 registry row" | line 10 | FALSE in effect for the shipped binary | the routing registry is media-probe's signature (es.rs:281) |
| "VPS/SPS/PPS types 32/33/34-aware, first_slice_segment_in_pic_flag probe" (deferred behavior) | doc 4-5 | REAL | annexb-level test baseline:1685 |

## 7. Dead / duplicated / unreachable code
The module is unreachable-from-production aliasing, identical in shape to
`es/aac.rs` (see that audit); kept at S4 for the same reasons (honest delegation,
documented intent, executed by the fuzz harness).

## 8. Dependencies & flags
`super::annexb` + `plugin_api::SourceStream` only. No features, no unsafe, no
tests of its own.

## 9. Open questions for the next agent
- Same systemic question as es/aac.rs §9: is any consumer reading
  `demux::es::*::EXTENSIONS`? If none, the whole per-module extension-list pattern
  is doc-weight — a wave-level sweep (one audit row for all es aliases) would close
  it. `es/h264.rs` and `es/mp3.rs` are the remaining twins (not in my list).
