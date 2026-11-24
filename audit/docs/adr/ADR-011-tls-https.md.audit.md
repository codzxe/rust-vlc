# CLAIM AUDIT: docs/adr/ADR-011-tls-https.md (L1-214)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 3 / 0 / 0 / 0 / 0 / 0 / 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An honest deviation-record ADR: it names the exact uncomfortable truth ("there is no production pure-Rust TLS provider… every https-capable option carries a language-policy deviation"), admits ureq 3.5.0 + rustls(ring) as a recorded E2-class exception with the ring C/asm deviation counted (135 files), and the tree matches the ADR's state exactly — ureq 3.5.0 pinned in crates/streaming/Cargo.toml with the admission record's path in a comment, record file present. The claims about what "the audit" measured in-session (cargo tree walks) are dated session facts, unverifiable here.

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "accepted (2026-11-09)… ureq 3.5.0 + rustls(ring 0.17.14) deviation is admitted as the E2/ADR-001-class exception" | :1-6 | REAL | crates/streaming/Cargo.toml:43-50 — ureq pin + "rustls crypto provider is ring 0.17.14 (C + asm compiled by cc)" comment; record docs/dependencies/records/ureq-3.5.0.md exists. |
| 2 | "there is no production pure-Rust TLS provider; rustls routes… to aws-lc-rs… or ring" | Context | UNVERIFIED | Ecosystem claim, dated; consistent with record and dependency records. |
| 3 | "ureq 3.5.0… is in the tree, the adapter is live, and a real HTTPS roundtrip was measured (record R10)" | :13-16 | REAL-UNPROVEN | Dep + client.rs exist; the measured roundtrip is a record-internal claim (record file exists; not re-executed — network). |
| 4 | "The audit (fresh, real cargo data — 2026-11-09, this session)" | §The audit | UNVERIFIED | Session cargo-walk facts; not re-runnable (cargo budget). |
| 5 | Supersedes the 2026-10-19 interim posture "no HTTPS dependency merges" | :18-19 | REAL | ROADMAP D-011 documents the same supersession chain (executed in Phase 6, 6.A1b-RATIFY). |
| 6 | Deviation counting: "the ring provider's 135 C/asm files documented as the recorded E2 deviation" | :6-8 | UNVERIFIED | Counted in the fresh audit section — dated session fact. |

## 3. Contradictions
(none — the README's "URL sources refused" is consistent: the transport is admitted but streaming enablement remains the open wave, exactly as ADR-011's timeline states.)
## 4. Fabrication / inflation indicators
None. The ADR's candor ("the deadline passed with the decision unmade, and the repo resolved the tension the honest way — admit conditionally, record the deviation") matches the repo's own deviation ledger. Commands run: rg ureq/ring/rustls in crates/streaming/Cargo.toml; ls records. Exit 0.
Findings: S0-S4: 0.
