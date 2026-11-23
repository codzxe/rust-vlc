# AUDIT: crates/streaming/tests/netfault_proxy_e2e.rs
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2025-06-14 |
| Lines / functions | 225 / 3 helpers + 7 tests (drives `tests/netfault/mod.rs`, the FaultProxy under test) |
| File verdict | REAL as a proxy-behavior test (7/7 executed pass) — but the "network fault" simulation is APPLICATION-LAYER FABRICATION over a healthy socket, not wire faults (F-001) |
| Reachable from shipped entry point? | N/A (test support). `FaultProxy` is `#[path]`-included test-support only (netfault/mod.rs:39 "Test-support only — nothing here is compiled into production paths") — verified: no production reference. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `./target/debug/deps/netfault_proxy_e2e-4e89c0859ab65585` -> "test result: ok. 7 passed", exit 0 |

## 1. One-paragraph verdict
HOSTILE-FOCUS ANSWER: **the socket is real; the faults are fake.** The stack is a genuine loopback TCP pair (MockServer = `TcpListener` at mockhttp.rs:454; MockClient = real HTTP/1.1 over `TcpStream` at mockhttp.rs:808-889) — but `FaultProxy::fetch` (netfault/mod.rs:198-237) first runs the inner fetch **to full successful completion** over that healthy socket, then *discards the successful response and fabricates the error in user space*: no TCP RST is ever sent (`ResetAfter` constructs `std::io::Error::new(ErrorKind::ConnectionReset, ...)` after a clean full read), no bytes are ever withheld (`DropAfter` returns `ReadTimeout` instantly — no read deadline ever fires), no truncation ever reaches the client (`TruncateAt` synthesizes `BodyTruncated{expected, received}` from a complete body), and `Delay` is pure virtual clock time. The e2e tests are honest about the *proxy's* behavior (typed shapes, fired/fetched counters, heal-after-script, past-end-never-fires) and all 7 pass; the misleading part is the "Wire shape" documentation, which describes simulated outcomes in wire language, and the fact that the project's one REAL wire-level fault facility (`MockConfig::truncate_body_at` + socket teardown, mockhttp.rs:369-371/549/700, pinned by `hard_reset_mid_body_is_body_truncated` at :1375) is never used in this lane. Additionally the fabricated `ResetAfter` error class diverges from what the project's own transports actually produce for a real mid-body reset (F-002).

## 2. What actually works (verified)
— `./target/debug/deps/netfault_proxy_e2e-4e89c0859ab65585` -> 7 passed (truncate/reset/drop exactly-once + heal, past-end-never-fires, virtual-clock delay, in-order multi-fault script, persistent-64 script), exit 0.
— Real transport verified: MockServer binds `TcpListener::bind(("127.0.0.1", 0))` (mockhttp.rs:454); MockClient connects via `TcpStream::connect_timeout` and serializes a real HTTP/1.1 request head (mockhttp.rs:827-889).
— Counter semantics verified by the passing asserts (`fired()` counts only faults that bit; `fetched()` counts every fetch).

## 3. Findings
F-001 [S3]
Where: crates/streaming/tests/netfault/mod.rs:198-205 (the proxy under test; doc table :11-16)
Code: `fn fetch(&self, request: HttpRequest) -> Result<HttpResponse, NetworkError> {\n        let fault = self.script.take();\n        let response = self.inner.fetch(request)?;\n        // The fault applies to the successful response the healthy\n        // network would have delivered (the proxy is deterministic:\n        // no socket-timing races to reconstruct what "half a body"\n        // means — the offset decides).`
What is wrong: every "network fault" is fabricated AFTER a fully successful fetch — the wire never misbehaves. The doc table markets the classes as "Wire shape" (`ResetAfter` = "a TCP RST mid-body", `DropAfter` = "the black hole: bytes stop, the read deadline hit") — no RST, no stopped bytes, no deadline. The e2e file's own header ("fires exactly as scripted, over the real-socket mockhttp transport") is letter-true but invites the reading that faults occur on the socket. Why it matters: (a) the real mid-body failure handling of the transports themselves (`read_bounded_body` short-read detection, mockhttp.rs:1128-1141; ureq short-body detection, client.rs:318-326) is exercised only by mockhttp's own unit tests, never in the scenario lane; (b) evidence built on this lane (net_interrupted.rs scenarios, 6.F2/6.F3) proves consumer reaction to fabricated error VALUES, not to fault-shaped wire behavior. The design rationale (determinism, no socket-timing races — stated at :202-205) is legitimate; the "Wire shape" framing is not. What correct looks like: rename the table column "Simulated outcome", state plainly that faults are synthesized above the transport, and route at least the truncation class through the REAL `truncate_body_at` server facility that already exists. Confidence: HIGH — Verification: read (proxy impl + mockhttp facilities) + ran.
F-002 [S3]
Where: crates/streaming/tests/netfault/mod.rs:220-228 vs crates/streaming/src/mockhttp.rs:1181-1183 and crates/streaming/src/client.rs:318-326
Code: `Some(Fault::ResetAfter { at }) if len > at => {\n                self.script.record_fired();\n                Err(NetworkError::Io {\n                    source: Error::new(\n                        ErrorKind::ConnectionReset,\n                        "fault-proxy: scripted connection reset mid-body",\n                    ),\n                })\n            }` (vs) `ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => {\n            NetworkError::BodyTruncated { expected, received }\n        }`
What is wrong: the fabricated error class for "reset mid-body" (`Io{ConnectionReset}`) is NOT what the project's own transports produce for a real mid-body reset: the mock client maps a real reset to `BodyTruncated` (mockhttp.rs:1181-1183) and the real ureq client maps the short-read shape to `BodyTruncated` (client.rs:318-326; only a read that *errors* — rather than ends — reaches `Io` via map_io_error's catch-all, client.rs:123-133). The scenario lane therefore validates retry/resume consumers against an error shape the stack's own transports would essentially never emit for that physical fault. Blast radius today: none on retryability — `is_retryable()` returns true for BOTH `Io{..}` and `BodyTruncated` (error.rs:227-233), so the divergence cannot change behavior until the retry table differentiates them; then it silently will. What correct looks like: `ResetAfter` fabricating `BodyTruncated` (the dominant real shape) or the lane covering both. Confidence: HIGH — Verification: read all three mappings + the retryable table.
F-003 [S4]
Where: crates/streaming/tests/netfault_proxy_e2e.rs:218-224
Code: `// ...but the script is finite (64 entries), so a runaway caller\n    // eventually sees the healed pass-through, never a spin.\n    for _ in 0..64 {\n        let _ = proxy.fetch(get(&server));\n    }\n    assert!(proxy.fetch(get(&server)).is_ok(), "finite script heals");`
What is wrong: the drain loop discards 64 fetch results without asserting they all faulted — only the final `fired()==64` count proves it. A proxy that healed early on fetch 40 of the `forever` script would still pass `fired()==64` only if it kept firing — actually it would fail (fired would stay < 64) — the count does pin it. Weaker than asserting each, but functionally sound; hygiene note only (the loop's discarded `Err`s are the entire point and go unchecked for class). Confidence: HIGH — Verification: read.

## 4. Per-function table (test file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| body | 38-40 | REAL | — | recognizable pattern, drift visible |
| serve | 43-51 | REAL | — | real socket server, patterned body |
| proxied | 54-60 | REAL | — | real MockClient wrapped by the synthetic proxy |
| get | 62-64 | REAL | — | |
| truncate_at_offset_fires_exactly_once | 66-86 | REAL | S3 | proves the proxy's fabricated shape + heal — not wire truncation (F-001) |
| reset_after_offset_fires_exactly_once | 88-109 | REAL | S3 | fabricated Io{ConnectionReset} diverges from real mapping (F-002) |
| drop_after_offset_fires_exactly_once | 111-127 | REAL | S3 | instant ReadTimeout — no deadline ever fires (F-001) |
| offset_past_body_end_never_fires | 129-142 | REAL | — | the exactly-scripted rule, pinned |
| delay_paced_by_injectable_clock | 144-163 | REAL | — | virtual-time only (documented honestly) |
| fault_script_sequence_fires_in_order_then_heals | 165-198 | REAL | — | order + heal + sleep accounting |
| persistent_fault_script_never_heals_within_its_budget | 200-225 | REAL | S4 | drain loop discards results (F-003); counters do pin behavior |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 7 | YES (for the proxy's own semantics) | exact typed shapes (expected/received pair, error kind), exact fired/fetched counters, heal-after-exhaustion, past-end passthrough, sleep accounting |
Caveat: they prove the SYNTHETIC proxy's contract, not network behavior — the lane's evidence value ends at "consumers react to these fabricated error values" (F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every Fault mode fires exactly as scripted, over the real-socket mockhttp transport" | netfault_proxy_e2e.rs:1-4 | PARTIAL | transport is genuinely real-socket (verified); the FAULTS are user-space fabrications after a healthy fetch (F-001) |
| "ResetAfter — Wire shape: a TCP RST mid-body" | netfault/mod.rs:14 | FALSE-in-mechanism | no RST is ever sent; io::Error constructed in user space (mod.rs:222-227) |
| "DropAfter — the black hole: bytes stop, the read deadline hit" | netfault/mod.rs:15 | FALSE-in-mechanism | bytes never stop; ReadTimeout returned instantly (mod.rs:229-232) |
| "Delay goes through the injectable clock … not a real sleep" | netfault/mod.rs:27-32 | REAL | FakeClock.sleeps() == [900] asserted (ran) |
| "Test-support only — nothing here is compiled into production paths" | netfault/mod.rs:38-39 | REAL | no production reference to FaultProxy (grep) |

## 7. Dead / duplicated / unreachable code
None in-file. `body()`/`serve()` duplicated verbatim in net_interrupted.rs (test-local duplication, accepted style per its header).

## 8. Dependencies & flags
streaming (mockhttp, clock, error, fetch), testkit, std. No unsafe, no feature gates, no production surface.

## 9. Open questions for the next agent
- net_interrupted.rs (not my file): its header says "the black hole bites mid-segment … byte-exact" — same fabrication caveat applies (F-001); its `mid_segment_drop_resumes_byte_exact` proves RetryFetch's ranged re-issue against a fabricated ReadTimeout, not a real black hole. Assess under the same lens.
- mockhttp.rs audit owner: the REAL wire-fault facility (`truncate_body_at`, forced status, honor_range) deserves a scenario-lane consumer; today only its own unit tests use it.
- streaming-hls.md "network-scenarios" record: does it describe this lane in wire terms? UNVERIFIED (doc not in my assignment).
