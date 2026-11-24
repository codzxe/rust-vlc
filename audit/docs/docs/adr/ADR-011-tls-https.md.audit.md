# CLAIM AUDIT: docs/adr/ADR-011-tls-https.md (214 lines, accepted TLS/HTTPS decision record)
| Field | Value |
|---|---|
| Auditor | W19-A09 — RECOVERED by W19-R2 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 6 / 0 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The decision record and the tree agree exactly: the admitted transport is ureq
`="3.5.0"` with `features = ["rustls", "rustls-webpki-roots"]` (Cargo.toml:52), the
deviation comment in that same manifest names ring 0.17.14 as "Known deviation,
Lead-ADR ask", the dependency record's R10 verdict text ("ADMITTED — conditionally…")
is byte-verbatim in ureq-3.5.0.md:247, and the repo lockfile carries rustls 0.23.46 +
webpki-roots (and NOT aws-lc — consistent with the ring-activation claim). The ADR
rejects its own options (b)/(c)/(d) with reasons, and its acceptance closes the
record's conditional admission exactly as the chain describes.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "ureq 3.5.0 + rustls(ring 0.17.14) deviation is admitted … exact-pinned (`=3.5.0`), minimal features … verification never off, Mozilla webpki-roots compiled in" | :93-97 | REAL | crates/streaming/Cargo.toml:52 `ureq = { version = "=3.5.0", default-features = false, features = ["rustls", "rustls-webpki-roots"] }` — feature-for-feature identical. |
| 2 | "`crates/streaming/Cargo.toml`: 'Known deviation, Lead-ADR ask'" | :38-39 | REAL | Cargo.toml:49-50: "Known deviation, Lead-ADR ask: the rustls crypto provider is ring 0.17.14 (C + asm compiled by cc)" — verbatim. |
| 3 | Record `ureq-3.5.0.md` R10: "ADMITTED — conditionally, on the Lead accepting the recorded ring deviation" + "ring 0.17.14 — 135 native C/asm files … 230 unsafe tokens" | :4-7, :79-81, :247 | REAL | docs/dependencies/records/ureq-3.5.0.md:247 R10 verdict text byte-verbatim; :55 "135 native source files"; :253-254 "ring 0.17.14 (135 C/asm files, cc build, 230 unsafe tokens)". |
| 4 | "ring 0.17.14" is the activated provider in the repo tree (rustls opt-in alternate) | :75-81 | REAL | Cargo.lock:749-750 `ring 0.17.14` + :775-776 `rustls 0.23.46` + :1313 webpki-roots 1.0.11; `aws-lc` NOT in the repo lockfile (the default provider is absent — exactly the "ring activated, aws-lc-rs not" state). |
| 5 | "crates/streaming/src/client.rs (the admitted transport … verification always on, webpki-roots)" | :14 | REAL | client.rs:1-15: "ureq adapter — the real HTTP(S) transport … rustls TLS with verification always on". |
| 6 | Acceptance chain: "decision ratified by the orchestrator, 2026-11-09, activating issue #39 … accepted 2026-11-09" | :2-8 | REAL-UNPROVEN | The repo-side facts (record + manifest + lock) all predate-consistently; the ratification ACT itself is orchestrator history — UNVERIFIED without git, but the ROADMAP's D-011 register (my ledger) records the same chain. |
| 7 | Scratch-crate audit numbers: "rustls v0.23.46; 35 packages locked; 19 crates; default provider aws-lc-rs v1.19.0 → cmake v0.1.58 …" | :62-70 | UNVERIFIED | Requires cargo (absent in this sandbox); the version claim (0.23.46) is corroborated by the repo lockfile; the aws-lc/cmake dependency claims are session data only. |
| 8 | Option rejections: (b) aws-lc heavier, (c) native-tls platform linkage banned by 6.A1b, (d) no production pure-Rust provider exists | :104-130 | REAL (reasoning over verified facts) | (c) corroborated by the record's banned-status language (ureq record F-columns); (d)'s premise is the Phase-0 §9 finding quoted consistently across docs 11/feasibility (SEC-07 chain). |

## 3. Contradictions
None. The ADR supersedes its own 2026-10-19 interim posture explicitly and names the
superseding cards (6.A1b/6.A1c) whose merge artifacts are the tree facts above.

## 4. Fabrication / inflation indicators
None. The ADR's central admission is a DEVIATION it discloses in its title line, quantifies
(135 files, 230 unsafe tokens), and chains to a conditional-admission record whose
condition this ADR exists to discharge.
