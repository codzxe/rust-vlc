# AUDIT: apps/desktop/src/error.rs
| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 7 items (module docs; enums ×2; Display impls ×2; Error impls ×2; 11 error variants total) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lib.rs:96 pub use error::{HostError, WireError}`; `WireError` constructed at wire.rs:{114,152,211-227,407,574-586,619,629,636,642} and bridge.rs:{204,216,220} (the frame-streaming decoder + presenter, exercised by the desktop lib test suite, green in orchrun-test-workspace-nofailfast.log.txt:2294-2320); `HostError::WebviewPending` constructed at host.rs:223 and files.rs (NativeDialog), consumed by shell.rs `pick_and_open`/`refusal_response`. Shipped-binary reachability rides the same build-gated tauri wrapper as shell.rs (baseline #2: UNVERIFIED-BUILD — infra, not repo defect). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full Read; `rg "WireError::\|HostError::" apps` → every construction site listed above; `rg "HostError::MailboxMisconfigured" apps crates` → ONLY its own Display arm (error.rs:146) — never constructed; `rg "MailboxMisconfigured" apps/desktop/src/{mailbox,transport}.rs` → the real failure is `TransportError::MailboxMisconfigured` (transport.rs:59,73,87); `rg "reason: \"" wire.rs bridge.rs` → exactly 4 distinct BadGeometry reasons ("zero dimension", "dimension exceeds 65535", "payload exceeds the 1 GiB cap", "payload exceeds the u32 length field"); `rg "bad magic 0x\|Display\|to_string" apps/desktop/{src,tests}` → no Display-string test anywhere; `rg "error::" apps/desktop/src/lib.rs` → re-export; sed -n verification of quotes. |

## 1. One-paragraph verdict
A clean, honest typed-error vocabulary for the desktop skeleton: 9 `WireError` variants (all constructed by the real decoder in wire.rs / presenter in bridge.rs — no aspirational variants except one, below), structured payloads only (no `String` fields, all `&'static str` or numbers), exhaustive `Display` impls without wildcard arms (a new variant forces a compile error in `Display`), and the engine-vocabulary mapping correctly deferred to bridge.rs where it lives (verified: bridge.rs:252-261 maps every variant to a dotted `render.stream.*` code). The defects are small and precise: `HostError::MailboxMisconfigured` is dead vocabulary — never constructed anywhere; the real capacity-zero failure raises the *different* type `TransportError::MailboxMisconfigured`, so the variant's doc comment describes a flow that does not exist through it; the `BadGeometry` "closed set — five outcomes, all named" claim is false as written (four distinct reason strings, convention-enforced by `&'static str` with no static guarantee, one string hard-coding the 65535 cap value); and the `Display` impls — including the "hex-stable" BadMagic rendering — have zero test coverage.

## 2. What actually works (verified)
- All 9 `WireError` variants have live construction sites in production code (wire.rs:114 ReservedNotZero, :152 UnknownFormatCode, :211/216/222 BadGeometry, :227+407 PayloadLengthMismatch, :574/581/619 Truncated, :586 TrailingBytes, :629 BadMagic, :636 BadHeaderLen, :642 UnsupportedVersion; bridge.rs:204/216/220) — no paper-only variants in `WireError`. — verified by grep.
- Both `Display` impls are exhaustive per-variant matches with no `_ =>` arm — adding a variant breaks compilation until the message is written. — verified by read.
- The doc's claim "the engine-vocabulary mapping … lives at the presenter boundary in bridge.rs" — verified: bridge.rs:252-261 maps `UnknownFormatCode → render.stream.format-unsupported`, `BadGeometry|PayloadLengthMismatch → render.stream.bad-geometry`, all others `→ render.stream.malformed`. — verified by read.
- "malformed records are *data* errors — a hostile or buggy webview feed must never crash the host": wire.rs's decoder returns these typed errors from every parse path (construction sites above); no unwrap/panic in error.rs itself. — verified by read + grep.
- `HostError::WebviewPending` is the honest ADR-004 pending state, constructed (host.rs:223, files.rs NativeDialog) and consumed through shell.rs's `DialogOutcome::Pending` — exercised green by `shell::tests::dialog_picks_dispatch_cancels_noop_and_pending` (nofailfast:2289). — test-cited.

## 3. Findings

### F-001 [S3]
Where: apps/desktop/src/error.rs:131-136
Code:
```
    /// The frame mailbox rejected a frame structurally (capacity
    /// zero — a configuration bug, not stream data).
    MailboxMisconfigured {
        /// Human-readable reason.
        reason: &'static str,
    },
```
What is wrong: `HostError::MailboxMisconfigured` is constructed NOWHERE in the repository — the only reference outside its own `Display` arm is nothing (grep across apps/ and crates/ returns error.rs:146 only). The real capacity-zero failure path raises a different type: `TransportError::MailboxMisconfigured` (transport.rs:59-87, produced by `MailboxTransport::new`/`FrameMailbox::new`), which is what `FrameTap::new()` propagates. The variant's doc comment describes a flow ("the frame mailbox rejected a frame structurally") that never routes through `HostError`.
Why it matters: Dead error vocabulary with a misleading doc comment: a maintainer handling a mailbox misconfiguration will look for/pattern-match `HostError::MailboxMisconfigured` and find it never fires; the duplicate name across two types (`HostError` vs `TransportError`) invites exactly the wrong match arm. This is the checklist's "naming that lies" / dead-code class.
What correct looks like: Delete the variant (and its Display arm), or convert `HostError` into the single mailbox-failure type by mapping `TransportError::MailboxMisconfigured` into it at the shell/frame_tap seam.
Confidence: HIGH — Verification: read + exhaustive grep.

### F-002 [S4]
Where: apps/desktop/src/error.rs:42-47 (with wire.rs:211-223, bridge.rs:220-221)
Code:
```
    /// A dimension or payload-length field that cannot describe a
    /// real frame (zero dims, cap violations).
    BadGeometry {
        /// Human-readable reason (a closed set — geometry validation
        /// has five outcomes, all named).
        reason: &'static str,
    },
```
What is wrong: The "closed set … five outcomes, all named" claim miscounts and is unenforced: exactly FOUR distinct reason strings exist repo-wide — `"zero dimension"`, `"dimension exceeds 65535"` (wire.rs:211-217), `"payload exceeds the 1 GiB cap"` (wire.rs:222), `"payload exceeds the u32 length field"` (bridge.rs:221) — and because `reason` is a free `&'static str`, nothing (type, const table, test) prevents a fifth free-text reason or a typo'd duplicate. Additionally `"dimension exceeds 65535"` hard-codes the numeric value of `MAX_DIMENSION` in prose while the code compares against the constant — if the cap ever changes, the message lies.
Why it matters: A "closed set" contract that is actually an honor system; the count is already wrong today (4, not 5 — or 5 only if one counts the adjacent `PayloadLengthMismatch` outcome, which is a different variant).
What correct looks like: Make the reason an enum (`enum GeometryReason { ZeroDimension, DimensionOverCap, PayloadOverCap, PayloadOverU32 }`) with `Display` — a real closed set; derive the cap text from the constant.
Confidence: HIGH — Verification: read + grep of every `reason: "` literal.

### F-003 [S4]
Where: apps/desktop/src/error.rs:77-117
Code:
```
            WireError::BadMagic { found, offset } => write!(
                f,
                "frame stream: bad magic 0x{:02x}{:02x}{:02x}{:02x} at offset {offset}",
                found[0], found[1], found[2], found[3]
            ),
```
What is wrong: Neither `Display` impl has a single test: no test in apps/desktop (src or tests/) asserts any rendered string (grep for the rendered text / `to_string` finds nothing). The doc's "hex-stable in `Display`" claim (error.rs:16) is unpinned — a formatter regression (`{:02x}` → `{:x}`, a dropped offset, a changed prefix) passes the entire suite, and the strings ARE a stability surface: they are what a user sees in logs and what the `zero_alloc_transport`-style diagnostics would print.
Why it matters: Error text is a de-facto UI/log contract; zero mutation protection.
What correct looks like: One test iterating representative variants asserting the exact rendered strings (the wire.rs test module already constructs most variants — extend with `to_string()` assertions).
Confidence: HIGH — Verification: read + grep (no Display assertions anywhere in the crate).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-9 | REAL | — | accurate two-family split; mapping-deferral claim verified true |
| `enum WireError` (9 variants) | 12-75 | REAL | — | every variant constructed in production (wire.rs/bridge.rs); structured payloads only — no String fields |
| `Display for WireError` | 77-117 | REAL | S4 | exhaustive, no wildcard; zero test coverage (F-003) |
| `Error for WireError` | 119 | REAL | — | empty impl — correct |
| `enum HostError` (2 variants) | 122-137 | REAL | S3 | F-001: MailboxMisconfigured never constructed (dead + duplicated name with TransportError's variant) |
| `Display for HostError` | 139-151 | REAL | S4 | exhaustive; untested (F-003) |
| `Error for HostError` | 153 | REAL | — | empty impl — correct |

## 5. Test honesty
No tests in this file. Variant *construction* is well-tested downstream (wire.rs's decoder tests assert structural equality on most variants, e.g. wire.rs:811-837, mailbox.rs:361, shell.rs:820-826); variant *rendering* (Display) is tested nowhere — F-003.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "malformed records are *data* errors — a hostile or buggy webview feed must never crash the host" | error.rs:4-6 | REAL | every parse failure returns a typed variant (wire.rs construction sites); no panic in the decode path — desktop tests green (nofailfast:2294-2320) |
| "Both carry structured payloads only" | error.rs:7 | REAL | all fields are numeric or `&'static str` — verified by read |
| "the engine-vocabulary mapping (stage `Present`, stable dotted codes) lives at the presenter boundary in bridge.rs" | error.rs:8-9 | REAL | bridge.rs:252-261 — verified by read |
| "hex-stable in `Display`" (BadMagic) | error.rs:16 | REAL-UNPROVEN | the formatter is correct today but untested (F-003) |
| "a closed set — geometry validation has five outcomes, all named" | error.rs:44-45 | FALSE | four distinct reasons exist (wire.rs ×3, bridge.rs ×1); set unenforced (F-002) |
| HostError::MailboxMisconfigured "The frame mailbox rejected a frame structurally" | error.rs:131-133 | FALSE | never constructed; the real failure is TransportError::MailboxMisconfigured (transport.rs:59-87) — F-001 |

## 7. Dead / duplicated / unreachable code
- `HostError::MailboxMisconfigured` — dead (F-001), name-duplicated with the live `TransportError::MailboxMisconfigured`.
- Everything else in the file is live (construction sites listed in §2).

## 8. Dependencies & flags
No external dependencies (std-only `fmt`/`error` impls). No feature gates, no unsafe, no alloc. Cross-file contract: `MIN_HEADER_LEN`/`MAX_HEADER_LEN` doc-links to wire.rs (both exist — verified by the doc-link targets in wire.rs's consts).

## 9. Open questions for the next agent
- bridge.rs / wire.rs owners: if F-002's reason-enum is adopted, bridge.rs:220-221 and wire.rs:211-223 are the four call sites to migrate; the `render.stream.*` mapping (bridge.rs:252-261) already groups them correctly.
- Whoever tests Display strings (F-003): the wire.rs test module's existing fixtures (811-837) are the cheapest place to add `to_string()` pins.
