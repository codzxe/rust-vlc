# CLAIM AUDIT: docs/baselines/raw/linux/16-options-vlcrc.txt (chunks #part1..#part6, L1-4726)
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 9 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED-as-verdict | 5 / 3 / 0 / 0 / 0 / 0 (1 UNVERIFIED sub-item) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This is a genuine-looking raw VLC 3.0.23 vlcrc preferences dump: 4707 body
lines between two verbatim markers, 191 module sections, 1439 commented
`#name=value` default lines and ZERO active (uncommented) option lines —
exactly matching every quantitative claim in its own 18-line preamble. The
embedded version strings (`###  vlc 3.0.23`, `VLC Media Player - 3.0.23
Vetinari`) and known VLC defaults (`sout-udp-caching=300`, `postproc-q=6`,
`adaptive-bw=250`) are consistent with VLC 3.0.23. Reproduction is impossible
in this sandbox (no VLC binary), so capture-level claims stay REAL-UNPROVEN;
structure-level claims are REAL (measured). One methodology trap: the Bash
output display layer eats a leading `[m` (CSI-m ANSI artifact), which first
made line 2406 look corrupted (`osaic_bridge]...`); `od -c` proves the file
line is intact (`[   m   o   s   a   i   c ...`). Any auditor quoting lines
that start with `[m` must verify with `od -c`/`sed | od -c`.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "VLC 3.0.23 pin — raw dump 16: preferences file walk (vlcrc, doc 02 §3.3 extraction 2)" | :1 | REAL | doc 02 §3.3 exists and describes exactly this: `docs/02-phase0-audit-plan.md:172-177` (`### 3.3 Options and preferences tree` ... `2. **Preferences file walk.** Run VLC once headless ... Parse and store under raw/.`) — rg for `3.3|vlcrc` on that file. |
| 2 | "Date: 2026-10-19 13:20 UTC" | :2 | UNVERIFIED | Date-only claim, no in-file evidence; sandbox cannot time-travel. Not contradicted. |
| 3 | Generation: `--reset-config ... -> exit=0` | :8-13 | REAL-UNPROVEN | Command text is coherent and explains the all-commented output (see claim 5); cannot re-run VLC here. |
| 4 | "writes /tmp/0b1-home/.config/vlc/vlcrc (4707 lines incl. blanks)" | :11 | REAL | `awk 'NR==19,NR==4725' <file> | wc -l` -> `4707` (body between the two `# ---- verbatim` markers; markers at :18 and :4726, `rg -c '^# ---- verbatim'` -> 2). |
| 5 | "Commented `#name=value` lines = options at DEFAULT (0 lines are active)" | :16-17 | REAL | `rg -c '^[a-zA-Z0-9_-]+='` on body -> 0 matches (ACTIVE_LINES:0); `awk '/^#[a-zA-Z0-9_+-]+=/{n++} END{print n}'` on whole file -> 1439 commented options. |
| 6 | "the on-disk file starts with a UTF-8 BOM (EF BB BF); it is stripped here" | :15-16 | REAL | `head -c 3 <file> \| od -An -tx1` -> `23 20 56` = `# V`, no BOM present in repo copy. |
| 7 | "Everything below the next marker is VERBATIM" | :17-18 | REAL-UNPROVEN | Malformed-line scan over body (python/awk grammar check: only `###`, `# ...`, `#name=value`, `[section] # ...`, blank) -> COUNT:0 anomalous lines. Section count `rg -c '^\['` -> 191. Byte-verbatimity vs upstream vlcrc not provable without the upstream binary. |
| 8 | Version identity of the captured tool | :20-21, :1866 | REAL | `sed -n '20,21p'` -> `###  vlc 3.0.23`; :1866 `#sout-avi-encoder=VLC Media Player - 3.0.23 Vetinari` (verified via rg -n 'Vetinari'). Matches pinned baseline docs/baselines/vlc-3.0.23.md header. |
| 9 | Repo copy is 4726 lines / 85700 bytes | manifest.tsv:1461 | REAL | `wc -l` -> 4726; python3 len(data) -> 85700; manifest.tsv lists `85700  4726  text` for this path. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| none found | — | Header counts (4707 body, 0 active, BOM stripped) all reproduce exactly; no doc contradicts this dump. |

## 4. Fabrication / inflation indicators
None. No "pass" language, no test claims — it is an inert preferences dump.
The only unverifiable items are the capture date and the generation commands
(require a real VLC 3.0.23 binary + Debian container).

## 5. Matrix rows
Not a matrix chunk — no rows. No matrix row cites this file directly
(`rg -l '16-options-vlcrc' docs/` -> no hits; only audit/ + wave manifest
reference it). Supporting doc docs/inventory/options-summary.md (audited by
another wave) cites it as raw evidence for counts 191/1439/4707, which this
audit independently reproduces.

## 6. Methodology note for next agents
Bash display layer strips a leading `[m` from output lines (ANSI CSI-m).
`sed -n '2406p'` displayed `osaic_bridge] # Mosaic bridge stream output`
while `sed -n '2406p' | od -c` shows the true bytes
`[   m   o   s   a   i   c   _   b   r   i   d   g   e   ]    # ...`.
The file is NOT corrupted. Quote `[m...` lines only after od -c verification.
