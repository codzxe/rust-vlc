# CLAIM AUDIT: docs/dependencies/records/symphonia-vorbis-0.6.1.md
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Claims extracted | 15 |
| REAL / REAL-UNPROVEN / FALSE / UNVERIFIED | 9 REAL / 4 REAL-UNPROVEN / 0 FALSE / 2 UNVERIFIED |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The record's admission claims are all repo-verifiable and all check out: exact pin `=0.6.1` in crates/codecs/Cargo.toml, all 4 "new" packages present in Cargo.lock with checksums byte-matching the record's pasted `ck` values, the deny.toml MPL-2.0 allowlist row it called pending has landed, and the vorbis seam (crates/codecs/src/audio/vorbis/) genuinely wires `symphonia_codec_vorbis::VorbisDecoder` behind the default-on `codec-vorbis` feature. The tarball-internal purity scans (F1–F7, `#![forbid(unsafe_code)]`) are session data I cannot re-inspect (no registry src in sandbox). One dangling edge: the record's standing wasm32 P1-verify is effectively still open because apps/web tree-shakes the external codecs out of the wasm build, so no wasm gate ever compiles this crate.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "the published Vorbis engine crate ... is `symphonia-codec-vorbis` 0.6.1" | L12-13 | REAL | Cargo.lock:1038 `name = "symphonia-codec-vorbis"`, version 0.6.1 |
| 2 | "`symphonia-bundle-vorbis` does not exist on crates.io" | L10-11 | UNVERIFIED | needs network; lock contains only bundle-mp3 (consistent) |
| 3 | R2 graph: 4 new packages, tree shape | L59-74 | REAL | Cargo.lock: symphonia-common→{log,core,metadata}; metadata→{lazy_static,log,regex-lite,smallvec,core}; codec-vorbis→{log,common,core}; regex-lite present (L733) |
| 4 | checksum `ck 73d90b4fcf796137cc…` | L61 | REAL | Cargo.lock codec-vorbis checksum `73d90b4fcf796137cc…` (prefix match, byte-for-byte through 18 hex) |
| 5 | checksum `ck 2acc3fcc18ec9b8cd…` / `ck 83713a97705d77bde…` / `ck 5153198d…` | L63,66,69 | REAL | lock entries `2acc3fcc18ec9b8cdd486…`, `83713a97705d77bdef7cd…` match |
| 6 | "No build.rs ... F2 NO" | L89-90 | REAL-UNPROVEN | tarball manifest not inspectable in sandbox |
| 7 | "`#![forbid(unsafe_code)]` in the codec + metadata crates" | L40-42, 162 | REAL-UNPROVEN | registry src absent; no vendored copy |
| 8 | "Pinned `symphonia-codec-vorbis = \"=0.6.1\"`" | L167 | REAL | crates/codecs/Cargo.toml:103 `symphonia-codec-vorbis = { version = "=0.6.1", optional = true }` |
| 9 | "deny.toml MPL-2.0 ... addition if 5.A3's merge has not already carried it" | L138-139 | REAL (landed) | deny.toml:117 `"MPL-2.0"` in allow list; L110 comment names the symphonia family |
| 10 | "implemented by `crates/codecs/src/audio/vorbis/**` ourselves" | L124-125 | REAL | dir exists: comments/decoder/error/headers/mod/seek.rs |
| 11 | "the corpus differential (`crates/codecs/tests/vorbis_bitexact.rs`) are the recorded coverage proof" | L127-129 | REAL | file exists (ls) |
| 12 | engine wired and shipped | implied by admission | REAL | decoder.rs:107 `use symphonia_codec_vorbis::VorbisDecoder as Engine;`; codecs default features include `codec-vorbis` |
| 13 | "wasm32 left as the standing P1 verify" | L104-105, 169-170 | OPEN (and effectively un-gated) | apps/web/Cargo.toml:13-26 target-splits codecs out of wasm (builtin-codecs only) — no wasm lane compiles symphonia-codec-vorbis |
| 14 | "OSV/RustSec ... no advisories as of 2026-10-26" | L144-146 | UNVERIFIED | needs network; date-stamped honest |
| 15 | "sha256 recomputed in-sandbox matches the index cksum" | L155-156 | REAL (via lock) | lock checksum equals the record's paste; recompute itself not re-run |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| none found among repo-verifiable claims | — | — |

## 4. Fabrication / inflation indicators
None. The record even documents a naming correction against the Lead's preferred spelling (`symphonia-bundle-vorbis` → `-codec-`, L9-15) — the opposite of yes-man inflation.

## 5. Open questions / next agent
- Claim #13: is anyone ever going to compile symphonia-codec-vorbis on wasm32, or should the P1 verify be closed as N/A given the tree-shake? Record leaves it "standing".
