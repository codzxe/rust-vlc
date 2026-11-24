# AUDIT: crates/theme-tokens/tests/fixtures/semantic_lint/violating.css
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines | 28 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/semantic_lint.rs:34-91 (`violating_fixture_fails_the_check`), run in baseline (cargo-test-workspace.txt:6320 ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | cat fixture; rg token values in themes/tokens.json (#9A9A9A -> gray.300 :111; alias.stroke-width.icon :968-972; semantic radius.control :1324, color.content.primary :1119) |

## 1. Verdict
Live-accurate violation fixture. Every violation it plants matches the
CURRENT committed token source: `#9A9A9A` is exactly primitive
`color.gray.300`'s `$value` (tokens.json:110-111 — the "W25 repin" comment in
the fixture is true today), `--color-gray-900` is a real primitive CSS name,
`--stroke-width-icon` is the alias `stroke-width.icon` (tokens.json:967-972),
and the two dotted paths (`primitive.color.orange.500`,
`alias.color.surface`) are real layer-prefixed names carried in comments
(the lint is text-based, so comment carriers are fair input). The clean
first rule (`--color-content-primary`, `--color-surface`, `--radius-control`)
references genuine semantic names (tokens.json:1119, :1324), so the fixture
produces EXACTLY the five asserted violations and nothing else — I verified
no unintended violation hides in the `.any()`-style assertions by checking
every name in the file against the three token layers.

## 2. What actually works (verified)
— The consuming test asserts all 5 violation shapes (kind + layer + token +
snippet), that every message cites "doc 08 §3.2", and output sortedness; it
passed in baseline (semantic_lint test binary, ok).
— Value coupling is self-updating: if gray.300 is repinned again, the
RawValue assertion fails until the fixture follows — drift cannot hide.

## 3. Findings
None. Zero findings — checked: all 6 distinct token references in the file
resolve to the intended layer (verified against themes/tokens.json), the
planted hex matches the live primitive value, and the companion clean.css
positive control consumes semantic-only names (verified: radius.control and
color.content.primary are semantic tokens).

## 4. Open questions for the next agent
None.
