# AUDIT: fuzz/corpus/fuzz_http_headers/not_found_404.txt
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 45 B / c8d9b907227fc110… (full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (raw HTTP response seed, hand-authored) |
| Reachable from shipped entry point? | N/A (fuzz corpus by convention) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

What it is: a raw wire-format HTTP/1.1 404 response with a single
Content-Length: 0 header and no body (45 B, CRLF line endings, content verified
by full read above) — the not-found branch seed for the HTTP header fuzz target.
Consumer: corpus-dir convention (fuzz/corpus/fuzz_http_headers/ + Cargo.toml
target fuzz_http_headers); no in-repo test reads it by name. Sibling seeds
ok_200/chunked_200/ranged_206 cover the other status branches.
