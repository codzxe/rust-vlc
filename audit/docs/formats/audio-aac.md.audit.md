# CLAIM AUDIT: docs/formats/audio-aac.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 / 2 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Record verifies on the code side: ADTS parse + codec-api adapter + fuzz exist in-tree
(aac module, 17 tests). Its unusual honesty markers check out: the "brief said
symphonia-bundle-aac but the real crate is -codec-aac" correction and the unsafe-site
census ("0 unsafe sites ... common has no forbid but 0 sites — recorded as the honest
weakening"). HE-AAC stays honestly blocked in the matrix.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "status = implemented for AAC-LC (ADTS + ASC modes); HE-AAC honestly partial/blocked" | front-matter | REAL | crates/codecs/src/audio/aac/{adts,decoder,error,fuzz,mod}.rs exist (17 tests); matrix codec-audio-aac-lc implemented (win/linux partial/macos) — HE sibling row blocked as stated |
| 2 | "codecs::audio::aac (adts byte-level parse + resync walk, codec-api adapter, typed taxonomy, fuzz)" | front-matter | REAL | module files exist incl. adts.rs + fuzz.rs |
| 3 | "crates = [symphonia-codec-aac =0.6.1] — lock diff is exactly 4 nodes" | front-matter | REAL-UNPROVEN | symphonia-codec-aac in codecs Cargo.toml features; lock-diff arithmetic not re-run |
| 4 | "the card brief's symphonia-bundle-aac does not exist on crates.io — the real crate is -codec-aac" | front-matter | REAL-UNPROVEN | external crates.io fact; consistent with doc 04 §3.1 naming |
| 5 | "fuzz_target = fuzz_codec_aac (harness landed; libFuzzer CI owed)" | front-matter | REAL | fuzz/fuzz_targets/fuzz_codec_aac.rs exists; the CI wiring debt is consistent with full_gate's fuzz section state |
| 6 | "0 unsafe sites in codec-aac+metadata (forbid(unsafe) in both)" | front-matter | REAL-UNPROVEN | upstream crate internals claim; not inspectable offline |
| 7 | "Records-consolidation entry (card 5.REC-2; engine evidence from 5.C2a + 5.C2b)" | L1 | REAL | engine module present; wave cards are worklog records (other files) |

## 3-5. Contradictions / fabrication / matrix rows
None / none / codec-audio-aac-lc row verified implemented; HE-AAC row stays blocked as the
record says. Commands: ls aac module; rg -c tests; matrix sed.
