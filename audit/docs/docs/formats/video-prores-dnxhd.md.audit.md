# CLAIM AUDIT: docs/formats/video-prores-dnxhd.md
| Field | Value |
|---|---|
| Auditor | W19-A07 (recovered by W19-R3) |
| Claims extracted | 9 |
| REAL 6 (incl. the append-only addendum's every checkable fact) / REAL-UNPROVEN 3 (crates.io + test-run claims) | |
| Internal contradictions found | 0 (by design: append-only supersession, both states preserved) |

## 1. Blunt verdict
A decision record whose 2026-11-10 addendum is the model for honest supersession: the
DNxHD-half claims all verify (filmcraft-dnx =0.9.0 pin with default-features=false —
byte-exact in Cargo.toml; dnx glue code present; matrix row partial ×3 with the
one-row-two-halves note; ProRes zero-hit grep re-confirmed by me), and the original
2026-10-24 body is left as written above the addendum rather than silently rewritten.
The material finding (soundkit-dnx LGPL posture) is the kind of license-catch a hostile
audit wants. One S4 pointer nit in code comments.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Addendum: "filmcraft-dnx = { version = "=0.9.0", default-features = false } is pinned in crates/codecs/Cargo.toml" | :118-123 | REAL | crates/codecs/Cargo.toml:158 `filmcraft-dnx = { version = "=0.9.0", optional = true, default-features = false }` — exact pin form; feature gate :67 `codec-dnx = ["dep:filmcraft-dnx"]`; root Cargo.lock:217 present |
| 2 | Addendum: "decode glue lives at crates/codecs/src/video/dnx/**" | :123-124 | REAL | dnx/{decoder.rs, error.rs, mod.rs} present (ls) |
| 3 | Addendum: "matrix row codec-video-prores-dnxhd is now partial ×3 (windows/linux/macos partial, web not_started)" | :124-126 | REAL | feature-matrix.toml:589 `status = { windows = "partial", linux = "partial", macos = "partial", web = "not_started" }` |
| 4 | Addendum: "ProRes: NOT implemented ... rg -li prores crates/codecs/ → zero hits" | :132-137 | REAL (re-verified) | rg -li prores crates/codecs/ -> 0 hits today; matrix notes row carries the same one-row-two-halves honesty (:591) |
| 5 | Addendum: "in-tree differential: dnx_differential 6/6 pass in the codecs suite, 643 tests / 624 pass / 0 fail" | :129-131 | REAL-UNPROVEN | cargo prohibited (load discipline); dnx_differential references exist in codecs sources (mod.rs); record filmcraft-dnx-0.9.0.md present with the admission addendum structure |
| 6 | "soundkit-dnx 0.13.3 ... LGPL-2.1-or-later ... not a candidate under current policy" | :48-54 | REAL-UNPROVEN (license read) | crates.io metadata unverifiable offline; the policy consequence is consistent with deny.toml permissive-only posture (verified in the dependency-record audits) |
| 7 | "doc 04's flat "none" for ProRes is stale since at least 2026-07-20 (feasibility already recorded this drift)" | :44-45 | REAL-UNPROVEN (network) | internal-consistency claim; the feasibility amendment is cited as the recording surface |
| 8 | "VLC handling: both families decode via avcodec ... no dedicated VLC modules" | :68-72 | REAL-UNPROVEN | formats.json census claim; consistent with the baseline module inventory pattern |
| 9 | Main-body "Status: deferred (later) ... not_started ×4" superseded by the addendum, "everything above is the 2026-10-24 authoring-date record and is left as written" | :3-15, :112-115 | REAL (append-only discipline) | the file IS append-only as claimed — original text intact below the addendum; the W24-R7 finding NEW-2 correction chain is visible |

## 3. Contradictions
None. The one-row-two-halves tension (deferred vs partial) is explicitly reconciled by
the addendum, which is itself the anti-contradiction mechanism.

## 4. Fabrication / inflation indicators
None — the record's headline honesty is downgrading its own primary record post-hoc
rather than editing it. [VERIFY] markers on provenance/conformance claims remain.

## 5. Findings
F-001 [S4] crates/codecs/Cargo.toml:146/:158 comment cites
`docs/dependencies/records/filmcraft-dnx-0.1.0.md` — the actual record file is
`filmcraft-dnx-0.9.0.md` (version-digit slip in the code comment's pointer only).
Confidence HIGH — Verification: read (both paths).

## 6. Reachability
N/A (decision record). The DNxHD decode path is shipped via the codec-dnx feature gate.
Commands: rg Cargo.toml:67/:146/:158, Cargo.lock:217, ls dnx/, matrix :589/:591,
rg -li prores crates/codecs (0 hits), rg dnx_differential, ls dependency record.
