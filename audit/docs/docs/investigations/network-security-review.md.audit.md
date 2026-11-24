# CLAIM AUDIT: docs/investigations/network-security-review.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 8 |
| REAL 7 / STALE-RESIDUAL 1 (fixed since, doc un-updated) | |

## 1. Blunt verdict
A code-cited security review whose line references check out against the live tree: the F1 https→http downgrade refusal is genuinely enforced (typed error + two named regression tests exist), the redirect budget is 10 exactly as stated, the 8 MiB response cap is a real constant. One staleness in the safe direction: the review's "Residual" (auth.rs manual-follow loop lacking the scheme gate) has since been FIXED in the tree (auth.rs:625 returns CrossSchemeRedirect, marked AUD-07) but the doc still lists it as an open flagged item. The review's own honesty section lists what it did NOT audit.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "F1 ... RESOLVED (wave 19) ... `NetworkError::CrossSchemeRedirect` ... checked in client.rs before the cross-host gate" | F1 Status | REAL | crates/streaming/src/client.rs:437 `return Err(NetworkError::CrossSchemeRedirect { hops: followed + 1 })` (gate in the follow loop); :67 module doc |
| 2 | "regression tests: `https_to_http_redirect_downgrade_is_refused_not_followed` ... and `scheme_downgrade_gate_pins_transport_safety`" | F1 Status | REAL | client.rs:689 (fn name) + :687 (doc names the second test) — both exist |
| 3 | "default budget 10 (`DEFAULT_MAX_REDIRECTS`, fetch.rs:31)" | verified-good | REAL | fetch.rs:31 `pub const DEFAULT_MAX_REDIRECTS: u32 = 10` |
| 4 | "default 8 MiB cap on whole-body reads (fetch.rs:376)" | verified-good | REAL | fetch.rs:28 `DEFAULT_RESPONSE_LIMIT: u64 = 8 * 1024 * 1024`; budget cited at :380 |
| 5 | "Residual: the auth layer's manual-follow loop (auth.rs:550-554) enforces cross-host but not the scheme gate — flagged to 6.E1's owner" | F1 Residual | STALE (fixed since) | auth.rs:625 now `return Err(NetworkError::CrossSchemeRedirect {..})`, :554 comment names "AUD-07 — the 6.H1 F1 residual: manual following must not become a bypass" — the residual is closed in-tree; the review doc was never amended |
| 6 | "ureq ... exact-pinned sole external dep (ureq 3.5.0 ...)" | F4 | REAL-UNPROVEN | ureq is in-tree as the streaming transport (worklog w30.4); exact 3.5.0 pin not re-checked in Cargo.lock by me |
| 7 | "no is_private/loopback/link-local check exists anywhere in ... {fetch,client,schemes}.rs" (F2) | F2 | REAL-UNPROVEN | plausible from posture description; rg spot-check for is_private in streaming src: none seen in cited contexts |
| 8 | "it did not audit ureq/rustls internals ... Every unaudited external behavior is marked [VERIFY]" | honest limits | REAL | the [VERIFY] markers are present in F4 |

## 3. Contradictions
| This doc says | Tree says | Which is true |
|---|---|---|
| auth.rs lacks the scheme gate (residual, flagged) | auth.rs:625 enforces CrossSchemeRedirect (AUD-07) | The TREE is ahead of the doc — residual closed, review text stale (S4 hygiene: resolved-fix tracking) |

## 4. Fabrication / inflation indicators
None — all four findings are risks disclosed, none fabricated; verified-good properties cite real code lines that exist.
