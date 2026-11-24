# AUDIT: fuzz/corpus/fuzz_http_headers/ranged_206.txt
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A08) |
| Size / sha256 | 88 B / a677471110c0bfa08f29e249e14d901396e95a67922bb316c8339f85df9c0814 |
| Type | hand-authored HTTP 206 Partial Content response head, ASCII with CRLF terminators |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed (campaign unwired); CONTENT byte-identical to an inline literal that IS fuzz-executed green on stable |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/repr + byte-compare vs the sweep's inline literal; sibling cmps ×4; rg in-tree readers; baseline greps |

## 1. One-paragraph verdict
A well-formed fake ranged-response head: `HTTP/1.1 206 Partial Content`,
`Content-Range: bytes 250-999/1000`, `Content-Length: 750` — internally
CONSISTENT (250..=999 inclusive = 750 bytes = 1000−250 = the declared
Content-Length), CRLF terminators, exactly the grammar the transport's response-
head parser + Range lane must handle. Not merely duplicated: the corpus file is
BYTE-IDENTICAL to the 2nd authored literal embedded in the stable sweep
(netfuzz_sweep.rs:145), so its content is fuzz-executed green in the baseline —
only the corpus copy itself rides the unwired campaign.

## 2. What actually works (verified)
— python3: 88 B; sha256 a6774711…c0814; repr
`'HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 250-999/1000\r\nContent-Length: 750\r\n\r\n'`
== the inline literal at netfuzz_sweep.rs:145 (byte-compare TRUE).
— Content executed green: baseline cargo-test-workspace.txt:5713
`netfuzz_sweep_http_headers_survives ... ok` (:5708-5716, 2 passed/2 ignored —
the ignored pair are the 60s long-run variants) — the sweep feeds the authored
heads through fuzz_http_headers (streaming::mockhttp::fuzz_response_head +
Range probes, netfuzz/fuzz.rs).
— Sibling corpus files all verified byte-identical to their inline twins:
ok_200.txt, chunked_200.txt, not_found_404.txt, odd_headers.txt (python3 cmp
TRUE ×4) — the corpus dir and the sweep literal set are in exact sync today.
— Registry wiring: fuzz_http_headers row (fuzz/README.md:90) → mockhttp::
fuzz_response_head + netfuzz; corpora-present gate green (full_gate.txt:57-58).

## 3. Findings
F-001 [S3]
Where: fuzz/corpus/fuzz_http_headers/ranged_206.txt vs crates/streaming/tests/netfuzz_sweep.rs:144-149
Code: `b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 250-999/1000\r\nContent-Length: 750\r\n\r\n",`
What is wrong: unguarded dual-maintenance — the seed exists as BOTH a corpus
file (campaign-only; rg finds zero in-tree readers of the path) and an inline
literal the sweep actually executes. Nothing asserts copy==literal: if one
edits the inline literal (or the file), they silently diverge, and the corpus
lane itself stays unwired (fuzz/README.md:165-168). The stable execution proves
the CONTENT, not the file's role as a corpus seed.
What correct looks like: the sweep reads the corpus dir (one fs walk), or a
5-line test asserts byte-equality of the 5 files vs the 5 literals; campaign
wiring closes the rest.
Confidence: HIGH — Verification: read + python3 byte-compare executed (all five).

## 4. Matrix evidence
README:27 "The HTTP-header seeds are hand-authored response heads (CC0, ours)"
+ the cross-format seed; no feature-matrix row cites the file; the mockhttp
row's stable evidence is the sweep (:5713 green).

## 5. Open questions for the next agent
- The 5 inline literals duplicate the corpus exactly today; when the campaign
  lands, remember the sweep already covers the same bytes — dedupe by making
  the sweep corpus-driven.
