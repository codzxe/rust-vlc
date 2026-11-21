# AUDIT: crates/demux/src/asf/fixtures/asf_pcm_s16le.wma (binary fixture)
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Size / kind | 26,106 B · Microsoft ASF container, audio-only PCM |
| SHA-256 (computed) | `29c7a469c3627a6f2d74c24b4921197ed68cd8c6e7cedbbc3e48439f79ccd52a` — first 16 hex match the pin `"29c7a469c3627a6f"` at crates/demux/src/asf/tests.rs:1494 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture only (loaded via `CARGO_MANIFEST_DIR/src/asf/fixtures` at test time, asf/tests.rs:1084-1089; never compiled in, never shipped). The demuxer it exercises is itself unreachable (see my asf/error.rs audit F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` → "Microsoft ASF"; `od -A d -t x1` head (ASF Header GUID `30 26 b2 75 8e 66 cf 11 a6 d9 00 aa 00 62 ce 6c` at 0, File Properties GUID `a1 dc ab 8c …` at 32 — the spec GUIDs); `sha256sum` (pin verified above); `rg -rn "asf_pcm_s16le"` (truth table tests.rs:1038-1050 + plan doc); `git log --diff-filter=A` (commit 8572a8e5, Lead ASF-d); baseline `orchrun-test-workspace-nofailfast.log.txt:1567` (family walk ok), `:1953` (live-ffprobe differential ok), `:1571/:1583` (seek lanes ok) |

## 1. One-paragraph verdict
A genuine, pinned, differential-verified ffmpeg 7.1.5 product: the bytes carry the real ASF Header and File Properties GUIDs (hex-verified), the SHA-256 matches the in-test pin exactly, the truth-table entry (`tests.rs:1038-1050`) asserts precisely what the filename promises — WAVEFORMATEX tag 0x0001 (PCM), mono, 8,000 Hz, block align 2, 16 bits, codec `pcm_s16le`, header 456 B / 5 children, Data Object at 456 with 25,650 B / 8 packets, no Simple Index — and the whole family test plus a **live ffprobe cross-check** (codec names + per-track `-count_packets` counts) ran green in the baseline. Provenance is fully documented: the generation command (`-f lavfi -i sine=440Hz:1s:8kHz -ac 1 -c:a pcm_s16le`) sits in both the truth table and `docs/plans/asf-selfbuild.md`, whose measured table lists this exact file as "n.wma (26,106 B) — decodable audio (PCM)".

## 2. What actually works (verified)
— SHA pin: `sha256sum` output prefix `29c7a469c3627a6f` == `expected_sha("asf_pcm_s16le.wma")` (tests.rs:1494). A byte-flip in this fixture fails `fixture_family_walks_match_the_measured_truth` at its first assertion (tests.rs:1138).
— Executed, passing: `asf::tests::fixture_family_walks_match_the_measured_truth ... ok` (baseline :1567 — walks THIS file's header/data facts), `asf::tests::fixture_family_cross_checks_against_live_ffprobe ... ok` (:1953 — live ffprobe oracle on this file's codecs and packet counts), `indexless_files_seek_by_bounded_linear_scan ... ok` (:1571 — this file has `simple_index: None`, so it exercises the linear-scan seek lane).
— Structure: ASF Header Object GUID at offset 0, declared header size 0x1C8 = 456 (od: `c8 01 00 00 …` at offset 16) — matches the truth table's `header_size: 456` exactly; file size 26,106 == the FPO's `file_size` truth tuple (26_106, 8, 3_100, 3_200).

## 3. Findings
None. Exactly what was checked: existence, size (26,106 B, matching three independent records: disk, FPO truth tuple, plan-doc table), container magic (both GUIDs hex-verified), SHA-256 pin equality, code references (truth table + plan doc), provenance (ffmpeg 7.1.5 command, Lead commit 8572a8e5 after the CI 6.1.x byte-stability failure documented at tests.rs:1070-1078), executed test evidence (4 baseline-cited tests), and name-vs-content honesty (pcm_s16le ↔ tag 0x0001/mono/8k/16-bit). The domain-level unreachability (no probe signature / no container arm) is filed against the asf domain in my error.rs audit — it is not a defect of this fixture.

## 4. Per-function table
Not applicable (binary asset). Consumers: `FIXTURES` truth entry (tests.rs:1038-1050), `generate()` fallback path (tests.rs:1100+), family walk test, live-ffprobe differential, indexless-seek test.

## 5. Test honesty
| Test consuming this fixture | Would it fail if the fixture (or code) were broken? | Reason |
|---|---|---|
| fixture_family_walks_match_the_measured_truth | YES | sha pin + header_size + children + FPO tuple + data tuple + track descriptor + first-pts, per file (tests.rs:1138-1300). |
| fixture_family_cross_checks_against_live_ffprobe | YES | Live ffprobe codec list must equal `["pcm_s16le"]` and per-track `-count_packets` counts must equal our walk's counts on these exact bytes (tests.rs:1325-1420). |
| indexless_files_seek_by_bounded_linear_scan | YES | This file (no Simple Index) drives the bounded linear-seek contract. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ffmpeg 7.1.5 sha256-pinned fixture differential incl. live ffprobe codec + -count_packets cross-check" | feature-matrix row container-asf-wmv evidence (:216) | REAL | sha verified; both tests ran ok in baseline (:1567, :1953). |
| "n.wma (26,106 B): … decodable audio (PCM)" | docs/plans/asf-selfbuild.md | REAL | Disk size + wfx truth + family test. |
| "byte-stable fixtures … committed at src/asf/fixtures/ (ASF-d, 2026-11-13)" | tests.rs:1068-1078 | REAL | In-tree dir exists; commit 8572a8e5. |

## 7. Dead / duplicated / unreachable code
Not applicable. Placement note: fixtures live under `src/` rather than the crate's `tests/fixtures/` convention — deliberate and documented (byte-stability + CARGO_MANIFEST_DIR resolution, tests.rs:1068-1089), not drift.

## 8. Dependencies & flags
None (test data). No license file in the fixtures dir — the corpus license claim ("class-B AVI derivatives (CC0, tests/fixtures/CORPUS.md)" in the avi fuzz target) does NOT cover the ASF family; the asf fixtures are ffmpeg self-builds (lavfi sine), so provenance is clean by construction, but no CORPUS.md row exists for them (noted; not a finding against this file).

## 9. Open questions for the next agent
- If issue #53's ASF wiring lands, this fixture becomes the natural first end-to-end open-path probe target; until then everything it proves is unit-level.
- The ASF fixtures have no CORPUS.md/license manifest entry (unlike the AVI corpus) — the fixtures-wave agent should confirm whether docs/corpus covers them elsewhere.
