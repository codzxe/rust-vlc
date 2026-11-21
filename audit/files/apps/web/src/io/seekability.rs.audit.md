# AUDIT: apps/web/src/io/seekability.rs
| Field | Value |
|---|---|
| Auditor | W12-A01 |
| Date | 2026-11-21 |
| Lines / functions | 448 / 15 (2 enum impl methods, 2 free fns, 1 enum, 11 tests + 2 helpers) |
| File verdict | REAL (logic verified, 11/11 tests executed in baseline) — but the lane it serves is unshipped: no `lumen`/`lumen-tauri` path reaches `lumen-web`; see Reachability |
| Reachable from shipped entry point? | NO (crate-level): `lumen-web` is a parallel wasm32 composition root; nothing in apps/cli, apps/desktop-tauri, or packaging references it (Grep `lumen-web|apps/web|wasm32` over .github/, scripts/ → zero hits; no dist/, no index.html). WITHIN its own lane: YES — `StagedSource::absorb` calls `detect_seekability` on EVERY absorbed response (staged_source.rs:482), and `StagedSource` is the `SourceStream` the web host opens http(s) URLs through (io/mod.rs docs; source.rs/open.rs lane). The lane itself never ran in a browser in baseline (INDEX.md has no web-matrix run; io/mod.rs:88-93 labels real-fetch behavior `[VERIFY]`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `Read` full file; `Grep detect_seekability` (callers: staged_source.rs:482 production + tests); read of staged_source.rs:455-584 (the caller's contract and its OWN at_end handler); read of crates/streaming/src/ranged.rs:445-610 + 1218-1262 (the claimed mirror: absorb has NO span-vs-body check; mid-read 416 → typed Truncated there); baseline `orchrun-test-workspace-nofailfast.log.txt:3143-3153` (11/11 tests ok); `sed -n` verbatim checks at 82-92, 125-138, 146-148, 163, 203-214. |

## 1. One-paragraph verdict
The probe classifier is mostly the disciplined mirror it claims: 206 requires a range and an agreeing, parseable `Content-Range`; oversized bodies and lying `Content-Length`s refuse typed; unknown-length (`bytes a-b/*`) stays `None`; surfaced 3xx is a typed transport breach; 416-at-offset-0 is honored as the empty resource — and all of it is pinned by 11 baseline-executed tests that would fail on real mutations. Two claims rot under pressure: the "wire-end" acceptance tests the PROMISED span's end instead of the DELIVERED body's end, so a connection that dies mid-body on a last-chunk 206 (the single most common truncation shape on real CDNs) is silently classified `Ranged` — exactly the "silent EOF" the module exists to prevent, and exactly the check its own consumer `staged_source.rs:540` implements correctly 350 lines away. And the blanket `416 → Empty` diverges from the ranged.rs mirror (mid-read 416 = typed Truncated there) once you notice the "one probe response" function is actually fed every window response by its only production caller.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3143-3153` → 11/11 `io::seekability::tests` pass.
— verified by read: the 206 agreement discipline is real — rangeless-206 refused (:99-103), missing header refused (:104-109), start disagreement refused (:111-118), body longer than span refused (:120-124), and the parser rejects missing unit/`/`/`-`, non-numeric fields, and `end < start` (:184-213, all pinned by `garbage_206_is_the_protocol_violation` and `disagreeing_or_oversized_206_refuses_typed`).
— verified by read: `Accept-Ranges` is genuinely advisory-only — the word never appears in the decision code, matching the doc (:19-22) and differing from naive implementations.
— verified by read: surfaced 3xx → `ProtocolViolation` with "the transport owns redirect following" (:164-169) — closes mock_fetch F-005's open question; the mock can script a 3xx and the detector refuses it typed.

## 3. Findings

### F-001 [S2]
Where: apps/web/src/io/seekability.rs:129-138
Code:
```
            let got = response.body.len() as u64;
            if got < span {
                let at_end = total.is_some_and(|total| end.saturating_add(1) == total);
                if !at_end {
                    return Err(FetchError::Truncated {
                        expected: span,
                        got,
                    });
                }
            }
```
What is wrong: the "wire-end" acceptance tests `end + 1 == total` — the PROMISED span's last byte against the resource total. It never consults `got` (the delivered length). The honest wire-end shape never enters this branch at all (an honest server that clamps its `Content-Range` to the resource end delivers the full span, so `got == span`); the ONLY responses that reach the `at_end` acceptance are bodies SHORT of their promised span — and every one of them whose span happens to end at the resource total is silently classified `Ok(Ranged)`. Concrete: resource 1500 B, probe `bytes=0-262143`, dying server answers `206, Content-Range: bytes 0-1499/1500` with 512 B of body → `at_end` = (1499+1 == 1500) = true → `Ok(Ranged { total: Some(1500) })`. No `Truncated`, no counter, no refusal.
Why it matters: this is the exact mid-body connection death the module's doctrine ("never a silent EOF", :127-128) exists to refuse, on the most common real-world shape (a `Content-Range` clamped to the end is what every honest server sends for the last chunk). The doc comment itself says the body must be the thing that "ends at the resource end" (:125-126) — `start + got == total` is that condition. The proof that the codebase knows the correct semantics: `staged_source.rs:539-540` computes `let end = start.saturating_add(got); let at_end = self.len.is_none() || self.len == Some(end);` — the delivered-body check — for the `Truncated` errors this function DOES return. The detector's premature acceptance also bypasses that handler entirely (it only sees `Err(Truncated)`, never `Ok` with a short body).
What correct looks like: `let at_end = total.is_some_and(|total| range.start + got == total);` (delivered end vs total) — or return `Truncated` unconditionally and let the caller (which has the length state) decide, as the current split already half-does.
Confidence: HIGH — Verification: read (arithmetic traced on the 1500/512 case); mutation reasoning: the existing test `truncated_206_body_is_the_typed_truncation` (:297-325) only covers the unknown-total/`*` shape (`bytes 0-1023/5000`, body 512 → `Truncated`) — no test scripts a clamped-to-end span with a short body, so the suite cannot catch this.

### F-002 [S3]
Where: apps/web/src/io/seekability.rs:163 (doc contract at :82-83; caller at staged_source.rs:482)
Code:
```
        416 => Ok(Seekability::Empty),
```
What is wrong: every 416 is classified as the empty resource. True for the probe (offset 0, `ranged.rs` honors the same), false for what the function actually receives: its only production caller runs it on EVERY absorbed response (staged_source.rs:482), including later window fetches. A mid-stream 416 on a window fetch (resource shrank — the exact scenario ranged.rs pins with its `mid_read_416_is_a_typed_truncation` test, ranged.rs:1221-1240, "the file source's contract, mirrored") here yields `Ok(Empty)`, and staged_source's `(_, _)` arm silently keeps the staged state (staged_source.rs:524-528).
Why it matters: the "mirror of ranged.rs" claim (:1-3) is false on this row — the mirror converts a mid-read 416 to a typed `Corrupt::Truncated`; this lane converts it to "empty resource" + caller-side no-op. The demuxer's next read at the unstaged frontier then loops on `window-not-staged` refusals (or re-mints the same 416-ing window fetch forever) instead of failing typed.
What correct looks like: `416 =>` discriminate on `request.range.start == 0` (probe → `Empty`; window → `Err(FetchError::Truncated/ProtocolViolation)`), or move the 416 decision into the caller that knows probe-vs-window.
Confidence: HIGH — Verification: read of both sites; downstream spin behavior inferred from staged_source.rs:524-528 (read), not executed — flagged for the staged_source auditors.

### F-003 [S3]
Where: apps/web/src/io/seekability.rs:203-213
Code:
```
    let total = match total.trim() {
        "*" => None,
        digits => Some(
            digits
                .parse::<u64>()
                .map_err(|_| violation("bad total".to_string()))?,
        ),
    };
    if end < start {
        return Err(violation("end before start".to_string()));
    }
```
What is wrong: no `end >= total` validation. RFC 9110 §14.2 requires `last-byte-pos < complete-length`; `bytes 0-1499/100` (a span claiming 1500 bytes of a 100-byte resource) parses to `(0, 1499, Some(100))`, the start-agrees check passes, a 1500-byte body fits the span, and detection returns `Ok(Ranged { total: Some(100) })` — length 100 with a staged window claiming [0, 1500).
Why it matters: a lying server puts the source into an inconsistent state (window extends past the believed length) that the seek/read bounds then have to survive; the module's posture is typed refusal for lying headers, and this lie walks through. (The claimed mirror `ranged.rs` has the identical hole — it binds `_rend` and ignores it, ranged.rs:572 — so "same grammar, same typed refusals" (:174-176) is accurate; the hole is inherited, not invented.)
What correct looks like: `if let Some(total) = total, end >= total { return Err(violation("end beyond total")); }`.
Confidence: HIGH — Verification: read; no test scripts an end-past-total header (the garbage test's `bytes 99-0/100` covers only end<start).

### F-004 [S4]
Where: apps/web/src/io/seekability.rs:146-148
Code:
```
            let declared = response
                .header("Content-Length")
                .and_then(|value| value.trim().parse::<u64>().ok());
```
What is wrong: a malformed `Content-Length` ("12x", "0x10") is silently swallowed to `None` and reinterpreted as "delivered body is the length" — while a PARSEABLE lie ("9999" vs 128 B) is the typed `ProtocolViolation` (:150-158). The never-silent doctrine applies to lies but not to garbage.
Why it matters: a server sending garbage in the one header this arm reads gets a MORE permissive answer than an honest one; the branch is untestable through the mock (which always writes well-formed headers).
What correct looks like: distinguish absent (`Option<&str>` → `None` → delivered-is-length) from present-but-unparseable (→ `ProtocolViolation`).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Seekability` (enum) | 38-60 | REAL | — | 3 outcomes, `Option<u64>` totals — a value and its absence are different types, as documented. |
| `Seekability::is_seekable` | 62-69 | REAL | — | `Ranged` only; honest (seek-serving is a separate duty, per doc). |
| `Seekability::length` | 71-79 | REAL | — | `Empty → Some(0)`; `Ranged{None} → None`. Correct. |
| `detect_seekability` | 93-172 | REAL | S2/S3/S4 | 206 arm: agreement checks solid; wire-end acceptance wrong quantity (F-001). 200 arm: cross-check good, malformed swallow (F-004). 416 arm: blanket Empty (F-002). 3xx arm: correct typed breach. Fall-through `HttpStatus`: correct. No panics (all arithmetic saturating; `u64::try_from(..).unwrap_or(u64::MAX)` at :120 is panic-free). |
| `parse_content_range` | 180-215 | REAL | S3 | Rejects unit-missing/no-total/no-end/bad-numbers/end<start; accepts end≥total (F-003). `strip_prefix("bytes")` + trim accepts glued forms ("bytes0-99/100") — permissive, harmless. |
| `mod tests` (11 tests, `probe`/`response` helpers) | 217-448 | REAL | — | See §5. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `accept_ranges_206_yields_ranged_seekable` (:250) | YES | Exact `Ranged{Some(5000)}` + seekable + length. |
| `unknown_length_total_yields_ranged_without_total` (:266) | YES | `bytes 0-1023/*` → `Ranged{None}`, `length() == None`. |
| `no_range_200_yields_unseekable_with_length` (:280) | YES | `Unseekable{Some(4096)}`, `!is_seekable`. |
| `truncated_206_body_is_the_typed_truncation` (:296) | PARTIALLY | Pins the unknown-total truncation (`expected:1024, got:512`) and the honest full-span wire end — but never scripts a clamped-to-end span with a short body, so F-001's wrong acceptance is invisible to it (the honest case it does script, `bytes 0-1499/1500` + 1500 B, has `got == span` and skips the branch entirely). |
| `surfaced_redirect_is_the_typed_transport_breach` (:330) | YES | 302 → `ProtocolViolation`. |
| `range_not_satisfiable_is_the_empty_resource` (:343) | YES | 416 → `Empty`, `length()==Some(0)`. (Probe-shaped only — see F-002.) |
| `garbage_206_is_the_protocol_violation` (:358) | YES | 5 malformed headers incl. `end<start`, wrong unit. |
| `lying_content_length_is_the_protocol_violation` (:381) | YES | 9999 vs 128 → violation; missing header → delivered-is-length. |
| `uninterpreted_statuses_are_typed` (:399) | YES | 404/500/503 → exact `HttpStatus`. |
| `disagreeing_or_oversized_206_refuses_typed` (:413) | YES | start mismatch and body>span both pinned. |
| `rangeless_request_with_206_refuses_typed` (:435) | YES | The "answered a question nobody asked" shape. |
All 11 ran and passed in baseline (orchrun-test-workspace-nofailfast.log.txt:3143-3153).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "seekability is *detected, not assumed*" (doc 09 §6.1) | seekability.rs:1-2 | REAL | The ranged probe IS the decision input; `Accept-Ranges` unused in decisions (:19-22 verified). |
| "A body shorter than the promised span is only honest when it ends at the resource end" | seekability.rs:125-126 | FALSE (code) | Code tests the promised span's end (`end+1==total`), not the body's (`start+got==total`). F-001. |
| "`Truncated` for a body shorter than the `Content-Range` span that does not end at the resource end" (doc comment :91-92) | seekability.rs:91-92 | FALSE (code) | Same defect — the acceptance fires for bodies that do NOT end at the resource end. F-001. |
| "the same typed protocol-violation posture `ranged.rs` keeps" | seekability.rs:17, :127-128 | SHALLOW | ranged.rs's absorb does NO span-vs-body check at all (ranged.rs:570-587) — this module is STRICTER than its mirror (good), except where it is silently LOOSER (mid-stream 416, F-002). |
| "mirrored from ranged.rs" (truncation) | seekability.rs:128 | SHALLOW | The mirror enforces truncation at READ time (wire-end vs open-time length, ranged.rs:19-23) and mid-read 416 → typed Truncated (ranged.rs:1221); this lane's 416 → `Ok(Empty)` (:163). F-002. |
| "`416` ⇒ `Seekability::Empty` — Nothing exists at offset 0" (table row) | seekability.rs:15 | REAL (probe) / FALSE (windows) | True for the probe; its only production caller feeds it every response (staged_source.rs:482). F-002. |

## 7. Dead / duplicated / unreachable code
- No dead code. `parse_content_range` is a deliberate re-statement of streaming's private parser (documented, :174-176) — acceptable duplication with the same grammar; the two implementations can drift (and F-003 shows both already share the same missing validation).
- Crate-level unreachable from shipped entry points (see Reachability) — the whole lane awaits a shipping vehicle that does not exist yet (no web dist, no page).

## 8. Dependencies & flags
- Depends only on `crate::io::fetch_port` types. No external crates, no unsafe, no feature gates. Workspace lints (unwrap/expect/panic deny) respected — the file is panic-free by construction (saturating arithmetic throughout; the single `unwrap_or` at :120 is the lint-legal form).

## 9. Open questions for the next agent
- staged_source.rs auditors (A05/A06): (1) does any test pin a mid-stream 416 on a WINDOW fetch (F-002's downstream: the `(_, _)` arm at :524-528 silently no-ops — does the pump then re-mint the same window forever)? (2) F-001's bypass — the `Ok` path inserts `[start, start+got)` with a hole to `total`; does any read cross such a hole in tests?
- host.rs/open.rs auditors (A03/A04/A07): confirm the open lane routes http(s) URLs through `StagedSource` (and thus this detector) — I verified the io/mod.rs wiring only.
- ui/e2e/web matrix owner: the browser lane has never executed end-to-end anywhere in baseline; every `[VERIFY per-browser]` label in this crate is still open.
