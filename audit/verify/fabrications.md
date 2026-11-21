# AUDIT-OF-THE-AUDITORS — FABRICATION RECORD

Per §7.3 step 4 and §13. Every fabricated citation found by the wave gate or
Wave-20 A04 sweep is recorded here with the agent, file, and disposition.
An agent with any confirmed fabrication is re-run on all its files.

## Wave 1 gate (2026-11-21)

Sample: 14 audit files × up to 3 citations (citecheck.py, seed 43).
Raw result: PASS 1 · PARTIAL 1 · FAIL 2 · SKIP (no extractable code block) rest.

Adjudication (orchestrator, manual `sed -n` verification):

1. `w01-a08/audit/files/LICENSE.audit.md` — F-001 `Where: LICENSE:1-21
   (cross-evidence: …; deny.toml:28-30)`, code block labeled "Code (the
   admission):" quotes deny.toml:28-30. Verified verbatim vs deny.toml:28-30 →
   **genuine, not a fabrication** (extractor paired the first path of a
   compound citation with the wrong quote).
2. `w01-a07/audit/files/product.schema.json.audit.md` — F-003 `Where:
   product.schema.json:21-25, 120-125; tools/xtask/…vs crates/…`, code blocks
   labeled with their exact sources (config_validator.rs:607-612,
   semantic.rs:128-129). Both verified verbatim; product.schema.json:21-25
   confirmed to lack a `pattern` on `name`. → **genuine, same extractor
   artifact.**

Confirmed fabrications in Wave 1: **0**.

Systemic note for Wave 20 A04: citecheck.py's `Where:` regex takes only the
first `path:a-b` of compound citations; adjudicate FAILs manually before
recording. Agents should keep quoting with explicit per-block source labels
(A07/A08 style is acceptable and precise).

## Wave 2 gate (2026-11-21)

Sample: 10 audit files × up to 3 citations. Raw: PASS 24 · FAIL 1.

Adjudication: `w02-a01/audit/files/crates/commands/src/catalog/command.rs.part2.audit.md`
F-001 `Where: command.rs:491-668 (variants) + apps/cli/src/dispatch.rs:155`,
code block labeled "Code (dispatch fall-through):" quotes dispatch.rs. Verified
verbatim at apps/cli/src/dispatch.rs:154-155 → **genuine; extractor paired the
quote with the first path of a compound citation** (same class as wave 1).

Confirmed fabrications in Wave 2: **0**.

## Wave 3 gate (2026-11-21)

Sample: 12 audit files × up to 3 citations. Raw: PASS 25 · FAIL 0.
Confirmed fabrications in Wave 3: **0**.

Process note: W03-A08 and W03-A09 initial spawns failed at the orchestration
platform level (timeout before any result); both were re-spawned. A08's
interrupted incarnation had already committed 7 file audits in its worktree;
the retry completed the remaining 4. No work was lost or duplicated.

## Wave 4 gate (2026-11-21)

Sample: 12 audit files × up to 3 citations. Raw: PASS 20 · FAIL 1.

Adjudication: `w04-a02/audit/files/crates/demux/src/es/tests.rs.audit.md`
F-002 cites tests.rs:302-303 and quotes the doc comment that actually lives
at :298-299 (verbatim, verified); the anchor line :303 (fn name) is correctly
cited and explicitly called out in the audit text. **Citation imprecision,
not a fabrication** — finding genuine, stands. Recorded as a precision
warning for the agent.

Confirmed fabrications in Wave 4: **0**.

## Wave 5 gate (2026-11-21)

Sample: 10 audit files × up to 3 citations. Raw: PASS 28 · FAIL 2.

Adjudication: both FAILs are the known compound-citation extractor artifact —
1. w05-a02 mkv/cluster.rs.audit.md F-001 `Where: …:254-262 (Fixed), 283-291
   (Xiph), 316-323 (EBML)`, quote labeled "Code (Xiph head…)" — verified
   verbatim at cluster.rs:283-291.
2. w05-a08 mkv/mod.rs.part1.audit.md F-001 `Where: …:249-254 (title), … with
   the admission at 270-274`, quote is that admission — verified verbatim at
   mkv/mod.rs:270-274.
Both genuine. Confirmed fabrications in Wave 5: **0**.
