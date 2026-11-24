# CLAIM AUDIT: docs/11-dependency-security-compliance.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 9 / 1 / 0 / 0 / 0 / 1 / 1 |
| Internal contradictions found | 1 (§8.3 H.264 row vs tree reality) |

## 1. Blunt verdict
A 757-line policy doc whose status header is honest ("planning document ...
Nothing here has been executed") — but which also carries live in-tree facts,
and almost all of them verify: the ADR-011 ureq/rustls admission (versions in
Cargo.lock, record file with matching counts, seam actually deleted, TLS
config fn present), deny.toml landed with a deviation log, cargo-deny pinned
in CI, SBOM tooling present, the codec patent register's audio/VP9/AV1/DNx
exposure rows matching the code, and the blocked crates genuinely absent
from Cargo.lock. One material lie of staleness: the H.264 patent row claims
"software decoder still absent" while `rust_h264 = "=0.6.1"` was admitted
2026-11-15 as a DEFAULT feature (`codec-h264-sw`) with a 640-line software
decoder and a bit-exact differential suite. A compliance register that
misstates its highest patent exposure is a real defect, not pedantry.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "ureq 3.5.0 + rustls; record `deps/records/ureq-3.5.0.md` R10" | :73-76 | REAL | Record exists as `docs/dependencies/records/ureq-3.5.0.md`; `rg 'ureq' crates/streaming/Cargo.toml` -> :52 `ureq = { version = "=3.5.0", default-features = false, features = ["rustls", "rustls-webpki-roots"] }`. |
| 2 | "rustls 0.23.46 ... ring 0.17.14 ... 135 C/asm files, cc-only build, 230 unsafe tokens" | :82-86 | REAL | `Cargo.lock`: `name="rustls" version="0.23.46"`, `name="ring" version="0.17.14"`, `name="ureq" version="3.5.0"`; record :55,:135,:253-254 carries the same 135/230 numbers. |
| 3 | "The acceptance deleted the seam (`HTTPS_ADR011_ADMITTED` const + `SchemeError::HttpsNotAdmitted`)" from `crates/streaming/src/schemes.rs` | :95-97 | REAL | `rg 'HTTPS_ADR011_ADMITTED\|HttpsNotAdmitted' crates/` -> only comments (schemes.rs:313,:651) + test doc comment (tests/https_activation.rs:9); no const/variant lives. https dispatch == http dispatch at schemes.rs:310-320 (verified read). |
| 4 | "verification never off — `crates/streaming/src/client.rs::tls_config`" (SEC-07) | :528 | REAL | `rg -n 'fn tls_config' crates/streaming/src/client.rs` -> :91 `pub(crate) fn tls_config() -> TlsConfig {`. |
| 5 | "`deny.toml` [PROPOSAL — exact keys verified when the file lands]" sketch | :222-246 | REAL | deny.toml exists at repo root with an explicit deviations-from-sketch header (deny.toml:1-33: minimal license set, bare-name bans, version-key omission) — the proposal landed and self-documents drift. |
| 6 | cargo-deny CI job "G3/G4 — licenses/sources/bans/advisories" | :299-305 | REAL | .github/workflows/ci.yml:491 job name; :510-516 installs cargo-deny 0.20.2 with SHA-256 check `9f12ed4c...  cargo-deny.tar.gz \| sha256sum -c -`. |
| 7 | SBOMs "per release ... CycloneDX ... `sbom-rust.json`" | :262-268, §9.3:702 | REAL-UNPROVEN | Tooling present: tools/xtask/src/cmd/sbom.rs and release.yml:396-399 (`gen-sboms.sh` per-artifact CycloneDX); actual SBOM artifact correctness not re-executed here (no build). |
| 8 | Codec exposure: AAC-LC, MP3/MP2, Opus, VP9, AV1(rav1d), FLAC/Vorbis, DNxHD implemented; AC-3/truehd BLOCKED | §8.3:624-644 | REAL | `ls crates/codecs/src/audio` -> aac,adpcm,alac,flac,mp2,mp3,opus,pcm,vorbis; `video` -> av1,dnx,h264,hevc,mjpeg,mpeg12,vp9; Cargo.lock has filmcraft-dnx, rav1d, ruopus, rusty_vp9, symphonia-codec-aac; NO oxideav/truehd in Cargo.lock; records: oxideav-ac3-0.0.11.md:147 `## Verdict: **BLOCKED**`, truehd-0.7.2.md:15 `PASS, admission BLOCKED`. |
| 9 | HEVC "parser + hw seam, no software decode" with typed `hevc.hw.required` | :631 | REAL | crates/codecs/src/video/hevc/decode.rs:434 `return Err(unsupported("hevc.hw.required"));` — no sw decoder in hevc/. |
| 10 | **H.264/AVC exposure: "software decoder still absent (parser + hw seam only — crates/codecs/src/video/h264/**)"** — "Exposure column refreshed 2026-11-09 to the implementation reality" | :630, :613 | FALSE | crates/codecs/src/video/h264/sw.rs (640 LOC, "The H.264 software decode track — the Lead-admitted rust_h264 engine (0.6.1, exact pin)"); record rust_h264-0.6.1.md:4-6 "admitted on the Lead's explicit 2026-11-15 instruction"; crates/codecs/Cargo.toml:53 `"codec-h264-sw"` inside `default = [...]`, :73 `codec-h264-sw = ["dep:rust_h264"]`; Cargo.lock:769 `name="rust_h264"`. The register's cited authority (ADR-010 §1 :104 "no software decode") is equally stale. |
| 11 | "ADR-006 ... DECIDED 2026-10-19 — MIT" | :570-572 | REAL | docs/adr/ADR-006-license-mit.md exists in docs/adr/ listing. |
| 12 | "Dev-only override via `LUMEN_DEV_INSECURE_TLS=1`" (SEC-07) | :528 | UNVERIFIED | `rg 'LUMEN_DEV_INSECURE_TLS' crates/` -> 0 hits; marker is `[PROPOSAL]` so absence is honest, but no implementation exists today. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| §8.3 H.264: "software decoder still absent (parser + hw seam only)" (:630) | rust_h264-0.6.1.md:4-6: admitted 2026-11-15; h264/mod.rs:15-23,38-39 documents the software track + differential suite; Cargo.toml default feature | The CODE is true — the doc row is stale/false. Doc 11 even asserts the column was "refreshed 2026-11-09 to the implementation reality (ADR-010 §1 is the file-evidence authority)" (:613) — but ADR-010 §1:104 carries the same stale "no software decode" line. The file-evidence authority is itself 6 days behind the admission it should track (ADR-010's own "Revisit triggers": "Any codec admission ... re-checks its tier before it joins a distributed build"). |

## 4. Fabrication / inflation indicators
One drift-class fabrication: the "refreshed to the implementation reality"
sentence (:613) is false for the H.264 row as of the rust_h264 admission
(2026-11-15). Everything else deflates honestly ([PROPOSAL]/[VERIFY]
markers survive; blocked crates stay blocked; the "nothing has been
executed" status header is contradicted only by the sections that
explicitly say they landed, and those verify).

## 5. Matrix rows
Not a matrix chunk. Note for the matrix ledger: the stale H.264 row here is
the same family of drift that parity-report flags (0/252 verified) — doc
layers asserting implementation reality without a mechanical re-check.

## 6. Findings
F-001 [S2]
Where: docs/11-dependency-security-compliance.md:630 (§8.3 patent register, H.264/AVC row)
Code: `| H.264/AVC | multiple pools ... | software decoder still absent (parser + hw seam only — crates/codecs/src/video/h264/**, doc 04 §4.2); using OS decoders ... |`
What is wrong: the register's core compliance fact for the most
patent-encumbered codec family is false — a pure-Rust H.264 software decoder
(rust_h264 =0.6.1) is admitted, default-enabled (`codec-h264-sw` in
crates/codecs/Cargo.toml:47-53), with differential evidence
(tests/h264_sw_differential.rs). Why it matters: anyone consuming §8.3 (or
ADR-010 §1, equally stale at :104) to decide "hw-only builds" or shipping
posture would be deciding on a false inventory. What correct looks like:
the row must state the software decoder landed 2026-11-15 behind
`codec-h264-sw` (default-on) and re-open ADR-010 Q3 exposure.
Confidence: HIGH — Verification: read (Cargo.toml:53, sw.rs:1-6, record
:4-6, Cargo.lock:769).

F-002 [S4]
Where: docs/11-dependency-security-compliance.md:75, :121, :207
Code: `record = "deps/records/mp4parse-0.17.0.md"` / `Records live in `deps/records/<crate>-<version>.md` `[PROPOSAL: layout]``
What is wrong: the records actually live at `docs/dependencies/records/`
(22 files, `ls` verified); the `deps/` path is used in prose at :75 as fact
("record `deps/records/ureq-3.5.0.md` R10") despite the layout still being
a proposal. Why it matters: path drift in the compliance chain of custody.
What correct looks like: one sed pass replacing `deps/records/` with
`docs/dependencies/records/` (3 spots).
Confidence: HIGH — Verification: read (ls docs/dependencies/records/).

F-003 [S4]
Where: docs/11-dependency-security-compliance.md:528 (SEC-07)
Code: `Dev-only override via `LUMEN_DEV_INSECURE_TLS=1`, refused in release builds ... `[PROPOSAL]``
What is wrong: no trace of the env var anywhere in crates/ (rg -> 0 hits).
Marked [PROPOSAL], so not a false claim, but the security catalog's only
TLS escape-hatch design is unimplemented and untested. Confidence: HIGH —
Verification: ran (rg).
