# CLAIM AUDIT: docs/formats/audio-truehd.md (88 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 REAL / 1 REAL-UNPROVEN (scratch differential) / 0 / 0 / 0 / 0 / 1 UNVERIFIED (OOM root-cause in this sandbox class) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Honest blocked-record: nothing shipped (crates=[] VERIFIED — no truehd/wavpack nodes in
Cargo.toml or Cargo.lock; matrix row blocked x4 with a blocked_reason that quotes this
record verbatim, including the 4,065,640-sample scratch differential and the
no-silent-downmix policy). The blocker narrative (rustc 1.99.1 OOM-killed by bitstream-io
4.10.0 ToBits macro expansion) matches the environment's known 3-4 GB constraint class;
the dependency record truehd-0.7.2.md exists. The record's discipline claim — "Per the
mandate: NO Cargo.toml edit, no module, no tests shipped" — is exactly what the tree shows.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "crates = [] — none admitted (truehd 0.7.2 ... NOT in the workspace graph)" | :13-14 | REAL — REPRODUCED | rg 'truehd|wavpack' Cargo.toml Cargo.lock -> 0 hits. |
| 2 | Matrix row "carries not_started + the 5.K1 hardening note — consistent: nothing shipped" | :7-8 | REAL (row now blocked x4 with reason) | matrix :779-790 codec-audio-truehd-mlp blocked x4; blocked_reason quotes "record docs/formats/audio-truehd.md ... 4/4 bit-exact (4,065,640 samples)". |
| 3 | "Purity: PASS — 28 .rs, build=false, zero native/blobs, Apache-2.0" | blocker § | REAL-UNPROVEN (record exists) | docs/dependencies/records/truehd-0.7.2.md exists. |
| 4 | "rustc 1.99.1 is OOM-killed compiling it ... bitstream-io 4.10.0's ToBits/write-tree macro expansion" | blocker § | UNVERIFIED (not recompiled here; matches the 2-core/3GB-class sandbox constraint) | Consistent with AGENT-SPEC §13 environment facts (sandbox disk/cpu limits recorded elsewhere). |
| 5 | "4 ffmpeg 7.1.5-authored streams through the /tmp-patched tarball ... 4 065 640 i32 samples compared, 0 mismatches" | proof § | REAL-UNPROVEN (scratch, not re-runnable — /tmp gone) | The numbers are consistent across record + matrix blocked_reason. |
| 6 | "fuzz_codec_truehd (planned name — nothing to fuzz until admission)" | :11 | REAL (absent) | fuzz/fuzz_targets/ has no truehd target (35-target census). |

## 3. Contradictions
None.
## 4. Fabrication / inflation indicators
None — a blocked row kept blocked with a root-caused unblock path; the record ships
negative status honestly.
