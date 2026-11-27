<!--FIXV1
agent: FX9-R6
title: Report bookkeeping — restore the missing changed-list entries for FX9-A03's registry-truth pin and FX9-A02's central Cargo.lock refresh
verdict: DONE
changed:
- audit/fix/waves/FX9/FX9-A03.md
- audit/fix/waves/FX9/FX9-A02.md
tests:
- (EMPTY — report bookkeeping; the gate is the oracle)
evidence_read:
- audit/fix/RULES.md
-->

Before the FX9-A03.md addition, `rg -n "pgssub" apps/cli/tests/fx2_a01_registry_truth.rs` confirmed the pin really landed at :110, :141-143 (the EXPECTED_FAMILIES entry attributed in-file to the FX9-A03 lane) and :219 (the family string `vp9, dnxhd, h264, pgssub, pcm`), so the file belongs in that report's changed list. Before the FX9-A02.md addition, `rg -n "swash" Cargo.lock` confirmed the central refresh carries the admission's resolution — the `swash` node (:1477) resolving through skrifa (:1387, dep :1482), read-fonts (:1109), yazi (:2107, dep :1483), and zeno (:2113, dep :1484) — and RULES.md (:11) was read to confirm the never-agent-edited framing; the added line is verbatim the mandated form (path first, parenthetical stripped by the gate). Both edits are exactly one line each inside the FIXV1 changed lists; every other line in both reports is byte-identical.
