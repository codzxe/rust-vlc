# CLAIM AUDIT: docs/dependencies/records/symphonia-aac-0.6.1.md
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 0 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An admitted-dependency record whose every repo-side claim verifies. The crate
IS in the tree exactly as recorded: `symphonia-codec-aac = { version = "=0.6.1",
optional = true }` (codecs/Cargo.toml:109), Cargo.lock carries
symphonia-codec-aac/common/metadata 0.6.1 + regex-lite (the record's exact
"4 new registry packages"), the feature wiring exists (`codec-aac =
["dep:symphonia-codec-aac", ...]`, codecs/Cargo.toml:60), the conformance
test it cites exists (crates/codecs/tests/aac_corpus_differential.rs), and the
"MPL-2.0 not yet in deny.toml, Lead owes the edit" claim was a true debt that
has since been paid — `MPL-2.0` is in deny.toml licenses.allow with the
symphonia-family comment naming codec-aac (deny.toml:110). The
naming-reality check is consistent with the tree: no `symphonia-bundle-aac`
anywhere (the AAC dep uses the codec spelling; only mp3 uses the bundle
spelling). Tarball-internal claims (forbid(unsafe_code) lines, 0 unsafe
greps, sparse-index NoSuchKey for the bundle-aac key) are network/tarball
claims — UNVERIFIED from this sandbox.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "pass ... admitted by card 5.C2a ... Pinned `symphonia-codec-aac = \"=0.6.1\"`" | :149-156 | REAL | codecs/Cargo.toml:109 `symphonia-codec-aac = { version = "=0.6.1", optional = true }`; Cargo.lock:1015 `symphonia-codec-aac 0.6.1` |
| 2 | "4 new registry packages (symphonia-codec-aac, symphonia-common, symphonia-metadata, regex-lite)" | :65-66 | REAL | Cargo.lock:1015/1049/1056/733 all present; the other 8 nodes (lazy_static, log, symphonia-core, smallvec) also in lock |
| 3 | "`symphonia-bundle-aac` — that crate does not exist on crates.io ... the actual published crate is `symphonia-codec-aac`" | :9-14 | REAL (as to tree) | rg symphonia-bundle-aac in Cargo.lock/codecs -> 0; the mp3 dep uses bundle-mp3, the aac dep uses codec-aac — the spelling split the record documents exists verbatim |
| 4 | "MPL-2.0 is NOT yet in deny.toml's allow set — the same-PR Lead edit the mp3 record already owes" | :129-133 | REAL (debt since discharged) | deny.toml licenses.allow now contains "MPL-2.0"; deny.toml:110 comment: "MPL-2.0 — symphonia 0.6.1 family: bundle-mp3, codec-aac," |
| 5 | "the corpus differential (crates/codecs/tests/aac_corpus_differential.rs) is the recorded coverage proof" | :123-125 | REAL | file exists |
| 6 | "codec-aac aac 0.6.1 ... `#![forbid(unsafe_code)]` ... 0 unsafe sites; no build.rs in any of the four tarballs" | :30-38, :73-76 | UNVERIFIED | tarball-internal; no network, no ~/.cargo cache in this sandbox |
| 7 | R6 API/behavior claims (ASC/ADTS modes, HE-AAC half-rate degradation honesty, `unsupported_error("aac: aac too complex")` mapping) | :92-122 | REAL-UNPROVEN | our decoder exists (crates/codecs/src/audio/aac/decoder.rs) with typed-refusal docs; the quoted error string is symphonia-internal (not in our tree — correctly, since it is the dependency's text); behavior not re-run here |

## 3. Contradictions
None found. The record's own "reality check" (bundle vs codec spelling) is
confirmed by the dependency wiring; the allowlist note (:18-20 "untouched by
5.C2a") is consistent with allowlist.md still lacking symphonia rows (its own
:55 lists the family as pending — the allowlist doc's gap, not this record's).

## 4. Fabrication / inflation indicators
LOW. The record's claims are falsifiable and the falsifiable ones pass; the
honest weakenings (symphonia-common lacking a crate-level forbid) are recorded
against the recommendation. The HE-AAC degradation is disclosed rather than
hidden ("records that degradation in the module docs + the differential
table"), the posture this audit looks for.

## 5. Note for the matrix owner
feature-matrix aac row + engine-info: the Wave-0 baseline flagged the
"landed: ... aac-lc ..." error-text vs engine-info (mjpeg, pcm_s16le only)
registry contradiction — the admission record itself is not the source of
that mismatch; the aac feature is optional (dep-gated), so a build without
the feature set legitimately lacks the decoder. Not a defect of this record.
