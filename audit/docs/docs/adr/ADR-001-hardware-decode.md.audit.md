# CLAIM AUDIT: docs/adr/ADR-001-hardware-decode.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A02) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 6 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
ADR-001 (hw-first H.264/HEVC, accepted 2026-10-19) + a 2026-11-15 addendum recording the
software-track trigger firing. All code-level addendum claims verify in today's tree: the
differential suite, the admission record, the sw decoder dispatch under the software-only
default, and the matrix flip (codec-video-h264-software = implemented x3 platforms; the hw
row honestly partial). The ADR's honest-reporting contract stands in tension with the W13
finding that the four decoder registries disagree (error text lists codecs engine-info
denies) — the ADR mandates truthful reporting, the shipped error string violates it.
No fabrication in this ADR.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "rust_h264 0.6.1... passed the conformance kit's requirements" | :81-90 | REAL-UNPROVEN | docs/dependencies/records/rust_h264-0.6.1.md exists; the differential run itself not re-runnable (cargo budget) |
| 2 | "crates/codecs/tests/h264_sw_differential.rs" exists | :83-84 | REAL | ls crates/codecs/tests/ -> h264_sw_differential.rs |
| 3 | "the codec-video-h264-software matrix row flips blocked → implemented" | :94-96 | REAL | docs/feature-matrix.toml:419 row; status windows/linux/macos = implemented, web not_started |
| 4 | "CLI provider dispatches sw::H264SwDecoder for h264/avc1 under playback.hw-decode-policy = software-only" | :96-99 | REAL | apps/cli/src/pipeline_host.rs:65 import + :346 `"h264" | "avc1" => ... H264SwDecoder::new`; schemas/settings.toml:640 playback.hw-decode-policy |
| 5 | "hw row stays partial pending Phase-7 runtime activation (ADR-013)" | :100-105 | REAL | feature-matrix.toml:406 codec-video-h264 partial; notes :416 "this hw row stays partial pending the Phase-7 runtime activation (ADR-013)" |
| 6 | "truthful active-decoder reporting... a feature-matrix and conformance-tested contract" | :38-41 | REAL | the contract exists (matrix row statuses + hw/mod.rs truth table) — but see Contradictions: shipped error text violates it |
| 7 | "Media Foundation... may be used only after audit" (decision 1 context) | :16-19,37 | REAL | MF never implemented (hw/mod.rs:51-56 retraction, verified in this batch's wave24-ledger audit); ADR never licensed an unaudited MF |
| 8 | "VA-API treated as E2 OS-service binding conditional on distro-shipped drivers" | :51-56 | REAL | ADR-013 (proposed) carries the activation decision; VaApiBridge landed (see R3's ADR-013 audit: gate-violated S3 — adapter code ahead of the ADR) |
| 9 | "the same session... converts I420 BT.601/BT.709... BT.2020 still refuses typed" | :110-114 | UNVERIFIED | color-converter lane not re-run here; consistent with matrix notes (mp4 row:152 mentions the converter) |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| "truthful active-decoder reporting" is the hw-first contract | W13: HEVC open error claims "landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264" while engine-info decoders = mjpeg, pcm_s16le only | The shipped error string violates this ADR's own reporting law — registry disagreement is the defect, the ADR is the standard |

## 4. Fabrication / inflation indicators
None. The ADR records its revisit trigger firing WITH the admission evidence paths, and
flips only the rows it earned (software implemented; hw honestly partial).

## 5. Notes for next agents
- The 2026-11-15 addendum makes ADR-001's "everything else stays software-first" list
  partly stale (rust_h264 admitted; check codec rows for flac/mp3/opus etc. actual states
  against the four-registry W13 finding before citing decision 4).
- Pair with docs/adr/ADR-013 audit (W19-R3): VaApiBridge 3,734 LOC landed before the
  ADR-013 decision — decision 5's conditional was not honored procedurally.
