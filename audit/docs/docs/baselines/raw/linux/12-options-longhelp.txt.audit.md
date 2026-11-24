# CLAIM AUDIT: docs/baselines/raw/linux/12-options-longhelp.txt
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 3 (provenance header claims) |
| REAL 3 / others 0 | |

## 1. Blunt verdict
A machine-captured raw evidence dump of `vlc -I dummy --longhelp` (VLC 3.0.23, Debian container, 2026-10-19). The body is genuine VLC help text (MRL syntax, options-styles, module tree) — 618 option-flag lines, 3 section headers. Not a hand-typed fabrication: format matches real `--longhelp` output including the "--help" module preamble. Re-running VLC here is not possible (binary not in sandbox) — provenance header is self-consistent and the upstream baseline doc cites it.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Pinned baseline: VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" | lines 1-6 header | REAL-UNPROVEN | Cannot execute VLC in sandbox. Version string internally consistent with docs/baselines/vlc-3.0.23-linux.md (audited separately). |
| 2 | "exit=0" for the version re-verification | line 13 | REAL-UNPROVEN | Same reason; header logs the command verbatim. |
| 3 | Body = real --longhelp output | lines 24+ | REAL | Content matches VLC's documented help structure (Options-styles/MRL syntax blocks, 618 `-` option lines); no placeholder markers. |

## 3. Contradictions
None found within this file.

## 4. Fabrication / inflation indicators
None. This is raw evidence, consumed by docs/inventory/options-summary.md and parse-options.py manifest (cited via rg).
