# AUDIT: tests/integration/first-vertical/src/json.rs
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A04) |
| Date | 2026-06-14 |
| Lines / functions | 192 / Json enum + obj + str + render + write_pretty + write_json_string + 4 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-harness crate module (tests/integration/first-vertical), consumed by harness/evidence writers for the transcript artifacts; not in the lumen-cli/desktop release lane |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | baseline cargo-test-workspace.txt:2735-2740 (4 json tests ok, lib 10/10); baseline :2775-2776 (duplicate-key serialization, F-001 evidence); read-through of the full file |

## 1. One-paragraph verdict
A clean std-only deterministic JSON writer: insertion-ordered objects, 2-space pretty form with trailing newline, RFC 8259 escaping (`"` `\` C0 controls incl. \b \f and \u00xx), no floats by design. Its four unit tests are honest goldens (exact byte strings, escape matrix, i64::MIN/u64::MAX extremes, order preservation) and pass in the baseline. Two defects: the object model silently accepts duplicate keys (and the evidence harness demonstrably emits them — the baseline failure text itself shows doubled "normalized" keys), and `render()` discards the writer Result.

## 2. What actually works (verified)
— verified by baseline cargo-test-workspace.txt:2735-2738 (`json::tests::escapes_strings ... ok`, `deterministic_and_order_preserving ... ok`, `renders_the_pretty_shape ... ok`, `scalars_render ... ok`; lib total :2740 "10 passed; 0 failed"). Mutation reasoning: a broken escaper fails :169 (`\u0001` case); a reordered emitter fails :180 (zebra before alpha); a wrong indent fails the :154-162 exact golden.

## 3. Findings
F-001 [S3]
Where: tests/integration/first-vertical/src/json.rs:31-32
Code:
```
    /// An object — insertion-ordered (deterministic per code version).
    Obj(Vec<(String, Json)>),
}
```
What is wrong: the object representation permits duplicate keys and the emitter renders them all without dedup or debug assertion. This is not hypothetical: the baseline failure output shows the harness-produced normalized transcript containing the key `\"normalized\": \"per-run\"` twice (cargo-test-workspace.txt:2775-2776 — both sides of the assert), i.e. the caller inserted two values under one name and this writer emitted ambiguous JSON. Why it matters: duplicate keys are last-wins in serde and a hard parse error in strict consumers — evidence artifacts whose meaning depends on key uniqueness are at the mercy of caller discipline with no backstop. What correct looks like: `Obj` as an order-preserving map with an insert that panics/debug-asserts on duplicate, or an emit-time duplicate check in the harness. Confidence: HIGH — Verification: ran (baseline output shows the duplicate bytes) + read.
F-002 [S4]
Where: tests/integration/first-vertical/src/json.rs:57-61
Code:
```
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = self.write_pretty(&mut out, 0);
        out.push('\n');
        out
    }
```
What is wrong: `let _ =` discards the `std::fmt::Result`. It is provably infallible for a `String` sink (fmt::Write for String cannot error) and the doc comment says so — but the pattern is a swallowed-error trap if the sink ever changes. What correct looks like: `self.write_pretty(&mut out, 0).expect("String fmt is infallible")`. Confidence: HIGH — Verification: read (clippy-clean per baseline cargo-clippy.txt, so no lint objects today).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Json enum | 17-33 | REAL | — | Int/UInt split keeps u64 counters exact |
| Json::obj | 37-44 | REAL | S3 | order-preserving sugar; dup keys possible (F-001) |
| Json::str | 47-49 | REAL | — | |
| render | 57-62 | REAL | S4 | F-002 |
| write_pretty | 64-112 | REAL | — | empty []/{} special cases, comma discipline |
| write_json_string | 120-139 | REAL | — | full RFC 8259 escape set incl. \b \f \u00xx |
| tests | 141-192 | REAL | — | 4 honest tests incl. i64::MIN/u64::MAX |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| renders_the_pretty_shape :146-163 | YES | exact byte golden |
| escapes_strings :166-170 | YES | covers quote/backslash/\n/\t/\u0001 |
| deterministic_and_order_preserving :174-181 | YES | insertion order asserted by position |
| scalars_render :184-191 | YES | extremes pinned |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Determinism is by construction" | module doc :8-9 | REAL | order-preserving Obj + no float formatting; test :174-181 |
| "Never (string formatting is infallible)" | render doc :54-56 | REAL | fmt::Write for String is infallible |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
std-only (fmt::Write) — honors the crate's C5 external-dependency-free constraint; testkit::Must used only in tests.

## 9. Open questions for the next agent
Who inserts the duplicate "normalized" key — evidence.rs `normalize_run_specific` or the harness; fix there (and consider the backstop here).
