# CLAIM AUDIT: docs/formats/audio-ac3.md
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 9 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 REAL / 0 / 0 / 0 / 0 / 0 / 2 (crates.io wave existence; corpus media files absent from repo) |

## 1. Blunt verdict
This decision record is the rare honest doc: "No code changed" is true, every landed-state
claim matches code byte-for-byte, and it names its own stale references. Only external
claims (crates.io survey, corpus lanes) are unverifiable in-tree — one stale lane id
(0001 vs 0034) is the only defect found.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "No code changed" | L3 | REAL | decision record; rg found no ac3 decoder code in crates/codecs (exit 0 no hits) |
| 2 | "matrix … carries `status = blocked` ×4 with empty `evidence`" | L6-8 | REAL | feature-matrix.toml:759-760 `status = { windows = "blocked", … }` / `evidence = ""` (sed-verified) |
| 3 | "PS: private-stream-1 `0x80..=0x87` → `PrivateKind::Ac3` … DTS rides `0x88..=0x8F`" | L104-108 | REAL | crates/demux/src/ps/mod.rs:148-149 `0x80..=0x87 => Some(PrivateKind::Ac3),` / `0x88..=0x8F => Some(PrivateKind::Dts),` (sed-verified) |
| 4 | "TS: stream types `0x81 → "ac3"`, `0x84 | 0x87 → "eac3"`, MediaKind::Audio" | L109-110 | REAL | crates/demux/src/ts/mod.rs:361-381 `0x81 => "ac3",` / `0x84 | 0x87 => "eac3",` … `=> MediaKind::Audio` (rg-verified) |
| 5 | "Raw ES: no `.ac3` reader yet (`crates/demux/src/es/` = aac/annexb/h264/hevc/mp3)" | L111-112 | REAL | ls crates/demux/src/es/ → aac.rs annexb.rs h264.rs hevc.rs mp3.rs (exact match) |
| 6 | "Decode refuses at the registry level (no AC-3 decoder registered)" | L113-114 | REAL | baseline audit/runs/baseline/mp4-smoke.md engine-info: decoders mjpeg, pcm_s16le — no ac3 |
| 7 | "fixtures `ps-vob-ac3.vob` … pinned in `crates/demux/tests/ps_fixtures.rs`" | L106-107 | REAL | ls fixtures/ps/ → ps-vob-ac3.vob; rg ps_fixtures.rs:102 `file: "ps-vob-ac3.vob",` |
| 8 | "no ac3-in-TS lane exists in corpus-v4 (all 5 `.ts` files … h264-only or aac)" | L123-124 | REAL | corpus-v4-smoke-manifest-20261023.json: .ts entries = 0010/0046/0055/0073/0073-ts-video + bit-drift regions; no ac3 lane (python3 count) |
| 9 | "0001-audio-only.ac3 (1.11 s) … exercises the future ES reader" | L120 | FALSE (stale id) | manifest L2828-2829 says `"path": "0034-audio-only.ac3"` — the only .ac3 lane; no 0001 ac3 lane exists |

## 3. Contradictions
| Doc says | Other source says | Which is true |
|---|---|---|
| lane `0001-audio-only.ac3` (L120) | corpus manifest: `0034-audio-only.ac3` | manifest — doc id stale (S4 drift) |
| doc's own "Doc drift for the Lead" (L159-167) flags doc 04 §3.1 staleness | doc 04 not audited here | consistent self-report |

## 4. Fabrication / inflation indicators
None. External crates.io claims (oxideav-ac3 6782 dl etc., L74-100) are UNVERIFIED —
network claims, no artifact in-tree to check; the doc labels them as survey output.
Corpus media itself is UNVERIFIED: manifest paths point outside the repo
(`/home/z/my-project/worktrees/w5-16/corpus-v4/media/…`, manifest L206) — no media files
in this tree (find → 0 .ac3). PS/TS carriage claims are nonetheless code-verified, and
demux PS tests exercise the substream strip (ps/tests.rs:216, 234-240).
