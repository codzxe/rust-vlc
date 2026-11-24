# CLAIM AUDIT: design/figma-renders/Settings_-_global.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A07 — RECOVERED by W19-R2 |
| Type | PNG image data, 900 x 743, 8-bit/color RGBA, non-interlaced (`file`) |
| Size / sha256 | 74,288 B / 0fbb89ad1088aba8a24d9b5aa84365c962c6d7bfbf57bf4a69b44820122accee |
| Verdict | REAL (render of Settings>Global; oracle for the global-area GAPS verdicts) |

## 1. Blunt verdict
Genuine Figma render, standard 900×743 Settings-family geometry, the `global` area's
default-state oracle. It is the frame the w26 fidelity loop graded the global settings
route against — and that grading produced honest GAPS verdicts (e05: "GAPS (F1, F2,
F5, F6)"), i.e. this asset's job is to witness what is MISSING, not to decorate a pass.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | One of the 13-area Settings render set (CONTRACT §3 lists global first) | whole | REAL | docs/waves/w26/CONTRACT.md:52 enumerates `Settings_-_global.png(-1)` + the other 12 areas; 13-area list matches D-002 (my ROADMAP ledger verified the same list via design/README.md:20). |
| 2 | Oracle for `/settings/global` fidelity comparisons | whole | REAL | w26.e05.md:8/:38/:283 — the route is compared against this PNG + `-1` state and the recorded verdict is **GAPS** (F1/F2/F5/F6), not a claimed pass; consumers w26.b01/b13/i18/k01/g10/h08 (rg hits). |

## 3. Contradictions
None. The consumers publish gap findings against it — no consumer claim about the asset
itself is contradicted by its verified type/dimensions.
