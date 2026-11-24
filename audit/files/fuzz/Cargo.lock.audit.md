# AUDIT: fuzz/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W16-A02 | Date | 2026-11-24 |
| What | Cargo lockfile v4 (`version = 4`), 24,202 B ASCII, sha256 3d4f1ef332015e49e6b350835a0a02248d2cf9a70c5fdfc249d9861edc6ba27b (batch manifest), 110 `[[package]]` entries |
| Generated vs handmade | Machine-generated (cargo), committed for the standalone fuzz workspace — correct practice (fuzz README + CI `cargo fuzz run --locked` semantics; audit.yml L154-adjacent tier installs cargo-fuzz) |
| Verification | `sed -n '247,252p'`: `libfuzzer-sys` version 0.4.13 with crates.io checksum — MATCHES fuzz/Cargo.toml's caret pin "0.4.13" (>=0.4.13,<0.5). Path deps all present: codecs (L105), demux (L123), playlists (L357), streaming (L568), subtitles, lumen-fuzz root (L272). |
| Consumers | cargo/cargo-fuzz builds of the 35 targets; the nightly audit.yml fuzz tier runs `cargo fuzz run` (which consults this lock) |
| Verdict | REAL, no findings. Note (context, not a defect of this file): the lock being committed does not mean the workspace ever built — zero committed campaign evidence exists (see fuzz_demux_avi.rs audit F-002); lock freshness vs the path-dep crates' current manifests is UNVERIFIED (cargo absent).
