# AUDIT: crates/platform-linux/src/error.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 115 / 3 (enum, Display, Error impl) |
| File verdict | UNREACHABLE (real, tested, orphan crate) |
| Reachable from shipped entry point? | NO — same crate-level proof as the pulse_ffi audit (F-001 there): platform-linux has zero workspace dependents. Exercised only by the crate's own tests (`platform_errors_render_their_details ... ok`, baseline orchrun :3672) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `diff` of the three platform error.rs files → byte-identical except the card number in line 1 (7.A1/7.A2/7.A3); `wc -l` → 114/114/114; Display test read (seam_behaviour.rs — all 8 variants, exact rendered strings); baseline check :3672 |

## 1. One-paragraph verdict
A clean eight-variant typed error vocabulary with payload-carrying fields
(names/symbol/api/code/deadline/program), exact human `Display` forms pinned
by a test that ran ok in the baseline, and honest doc comments. Its one
structural defect is that it exists three times: the Linux, Windows, and
macOS adapters each carry a byte-identical 114-line copy (verified by diff —
only the card number in the first line differs), a documented-but-live
tripllication that forces every fix to land three times. The crate-level
unreachability (orphan crate) is filed and proven in the pulse_ffi audit and
applies here unchanged.

## 2. What actually works (verified)
— `platform_errors_render_their_details ... ok` (baseline
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3672`) —
the test constructs all 8 variants and asserts exact rendered substrings
(seam_behaviour.rs read: NotCompiledForThisOs through SpawnFailed).
— All 8 variants carry structured payloads — no bare-string error anywhere
in the vocabulary (the MP §8 discipline the doc claims, :3-5).
— `CallFailed.code: Option<i64>` honestly models numeric-vs-non-numeric OS
  statuses; Display branches on it (:94-97).
— Mutation check: changing any Display arm's wording fails the exact-
substring test.

## 3. Findings
### F-001 [S3]
Where: crates/platform-linux/src/error.rs:1-115 (with crates/platform-windows/src/error.rs, crates/platform-macos/src/error.rs)
Code:
```rust
//! The typed platform-error vocabulary (card 7.A2).
//!
//! Every failure a platform seam can produce is one of these variants
//! — never a bare string, never a fake success (MP §8: "errors are
//! data, and the error path is a first-class outcome"). The same
//! shape is mirrored in `platform-windows`/`platform-macos` so the
//! Lead can later unify them into a shared L5 vocabulary crate
//! `[PROPOSAL]`.
```
What is wrong: The "mirror" is a byte-for-byte triplication: diff shows the
three 114-line files identical except `7.A2` vs `7.A1` vs `7.A3` in line 1.
228 redundant lines. The unification is a `[PROPOSAL]` with no issue number
and no tracking row I could find.
Why it matters: Any vocabulary change (a new variant, a field rename, a
Display fix) must land in three places with no test or CI check pinning
their equality — the kind of drift that surfaces exactly when one
platform's error story silently diverges. My own capability.rs finding
(from_error's lying catch-all) already exists in triplicate.
What correct looks like: Either the shared L5 vocabulary crate the doc
proposes (one `platform-common` with the enum + Display), or a CI diff-
check asserting the three files stay identical (the crude-but-honest
interim).
Confidence: HIGH — Verification: `diff` × 2 (byte-identical but line 1);
`wc -l` × 3.
Severity S3: duplication is the named S3 class; the drift risk is real but
bounded to the (currently orphan) adapter trio.

### F-002 [S4]
Where: crates/platform-linux/src/error.rs:114
Code:
```rust
impl std::error::Error for PlatformError {}
```
What is wrong: The Error impl carries no `source()` and the vocabulary has
no `From<std::io::Error>` (or similar) — every underlying OS failure is
stringified into a `detail` field at construction and the causal chain ends
there. `SpawnFailed { program, detail }` is the clearest case: the io::Error
kind (NotFound vs PermissionDenied) survives only as prose.
Why it matters: Downstream handlers cannot branch on the OS-level cause
without parsing English; the "errors are data" discipline stops one level
short of the OS.
What correct looks like: `source()` returning the wrapped error where one
exists (SpawnFailed carrying `#[source] io::Error`), or at least a typed
cause enum instead of the detail string for the spawn case.
Confidence: HIGH — Verification: read; construction sites in dialogs.rs use
detail strings (cross-checked during the capability audit).
Severity S4: the detail string preserves the information for humans; only
the machine branch loses.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PlatformError` (enum) | 15-74 | REAL | — | 8 variants, all payload-carrying; WrongOs/Service use &'static str names (no alloc on the common paths) |
| `Display` | 76-112 | REAL | — | All 8 forms exact-tested; code-Option branch honest |
| `Error` impl | 114 | REAL | S4 (F-002) | Leaf error, no source chain |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| platform_errors_render_their_details (seam_behaviour.rs) | YES | All 8 variants constructed with realistic payloads; exact rendered substrings asserted (full case list read) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every failure a platform seam can produce is one of these variants" | module doc :3-4 | REAL within the crate's seams | pulse/dl/dialogs/gpu construct only these variants (imports verified during pulse_ffi/capability audits) |
| "The same shape is mirrored in platform-windows/platform-macos" | :6-7 | REAL — and it is a literal byte-copy | diff proof (F-001) |
| "so the Lead can later unify them [PROPOSAL]" | :7-8 | PAPER-ONLY | No tracked issue/row for the unification found (rg over docs/ + matrix) |

## 7. Dead / duplicated / unreachable code
- Duplicated: the entire file, ×3 (F-001).
- Unreachable from shipped entry points (crate-level, proof in the
  pulse_ffi audit — not re-counted as a new finding here to avoid
  triple-filing one defect).

## 8. Dependencies & flags
`std::fmt` only. No unsafe, no I/O, no feature gates. Derives
Debug/Clone/PartialEq/Eq — PartialEq on String payloads makes the
capability tests' exact-variant assertions possible.

## 9. Open questions for the next agent
- platform-windows/src/error.rs and platform-macos/src/error.rs are
  assigned to other waves' files? If unassigned, this audit's F-001 diff
  proof covers them; their auditors can cite it and skip re-deriving.
- If the shared L5 vocabulary crate lands, capability.rs's `from_error`
  (my finding there) should move into it — one mapping, three adapters.
