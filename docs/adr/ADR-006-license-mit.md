# ADR-006: Repository License — MIT

- **Status:** accepted (user decision, 2026-10-19)
- **Deciders:** project owner (user), Lead (recording)
- **Supersedes:** open question in `docs/11-dependency-security-compliance.md` §8.2; risk R-05 (`docs/14-risk-register.md`)
- **Related:** D-009 (`docs/ROADMAP.md` §3), D-010 (design-package clearance), ADR-010 (patent matrix, pending)

## Context

This repository is a clean-room, from-scratch reimplementation of VLC media
player. The license choice constrains: which dependencies may enter the
allowlist (doc 11 §3), how the plugin/extension ecosystem can grow (doc 06 —
WASM extensions may be third-party and proprietary), and how the project can
be redistributed and forked. Options considered during planning (doc 11 §8.2):
GPL-family (VLC-symmetry), Apache-2.0 (explicit patent grant + notice
machinery), MIT (maximal permissiveness, minimal friction), dual-licensing.

## Decision

The repository is licensed under the **MIT License** (see `LICENSE`).
Copyright line: "Lumen Media Player Contributors" (working name per D-006;
rebranding via `product.toml` does not change the license).

## Consequences

**Positive**

- Maximally permissive: no friction for adoption, embedding, proprietary
  forks, or a mixed-ecosystem of extension plugins (doc 06 §5 extensions are
  not forced to any license by the host).
- GPL-compatible: derived works may still be GPL; coexistence with the wider
  VLC-adjacent ecosystem is unimpeded.
- Aligns with the interim dependency rule already enforced since planning
  (only permissive-licensed dependencies merge — doc 14 R-05 mitigation),
  which now becomes permanent baseline policy.

**Negative / accepted trade-offs**

- No explicit patent grant (unlike Apache-2.0). Accepted at current scale;
  revisit via a superseding ADR if patent exposure grows (see ADR-010 patent
  shipping matrix, doc 11 §8.4).
- MIT's notice requirements are minimal; contributor processes (CLA/DCO) are
  out of scope for now and can be added later without relicensing.

**Unaffected**

- Clean-room discipline (MASTER-PROMPT §2): MIT covers our code only. VLC's
  GPL-licensed source remains strictly forbidden — no vendoring, no copying
  (doc 11 §8.1). Running pinned VLC binaries as an external test oracle
  remains allowed.
- The design package under `design/figma/` is cleared per separate user
  attestation (D-010 / R-07 closure log in doc 14).

## Notes

Recorded from the user's decision delivered 2026-10-19 ("mit licence"). This
ADR closes risk R-05 before Phase 0's license-policy work (doc 02 §7),
unblocking the dependency-allowlist license-compatibility column from day one.

*Location note (2026-10-22, card 3.DOC-1): this file moved from
`docs/decisions/` to `docs/adr/` for register consistency (doc 01 §7 — all
ADRs live in `docs/adr/`); all doc references updated in the same change.
The worklog's historical entries retain the original path by design
(append-only log).*
