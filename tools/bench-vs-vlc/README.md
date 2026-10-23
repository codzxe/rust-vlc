# bench-vs-vlc — differential-vs-VLC 3.0.23 harness v1

Card 3.E3 (doc 13 wave 3.E), per doc 10 §6.6 (the oracle). Owned
entirely by that card (`tools/bench-vs-vlc/**`).

Compares Lumen's **decoded PCM** against the pinned external VLC
3.0.23's decoded PCM, fixture by fixture, over the corpus — the
lossless-comparable paths only. MD5 is the comparison digest (doc 10
§6.6 "Decoded audio").

## Layout

| file | role |
|---|---|
| `run.py` | the orchestrator: manifest/file-list input → classification → oracle invocation → per-fixture report (markdown + JSON evidence) |
| `mock_vlc.py` | the MOCK oracle (clearly-labeled stand-in; plumbing validation only) |
| `test_run.py` | python unit tests (mock mode end-to-end, synthetic-PCM comparison, classification, separation invariants, version-drift abort) |
| `Cargo.toml` | standalone cargo workspace (NOT a root-workspace member — see the file's header comment) |
| `src/lib.rs` | the Lumen-side digest pipeline (real demux+codec crates; first PCM audio track → MD5) |
| `src/md5.rs` | MD5 from RFC 1321 (public spec; zero new dependencies) |
| `tests/lumen_pcm.rs` | the cargo-test Lumen runner: pinned fixture digests, synthetic-PCM digest proofs, the `#[ignore]`-marked batch entry the orchestrator drives |

## The mock/real separation (binding)

* `--oracle vlc` — the REAL oracle. Requires `vlc`/`cvlc` on PATH;
  `--version` is asserted to be exactly **3.0.23** (drift aborts the
  run — doc 10 §6.6). Without a binary the run **refuses** (exit 2);
  it never falls back to the mock.
* `--oracle mock` — the MOCK oracle. Every row/label/artifact carries
  `[MOCK]`; the JSON envelope records `oracle.kind = "mock"`. A mock
  run is plumbing validation, never a measurement, never a conformance
  claim. `run.py`'s report writer *asserts* these invariants.
* `--oracle pending` — no oracle: comparable rows carry the exact
  to-run VLC command and a `PENDING-VLC` oracle cell; the report
  asserts zero oracle digests are present.

## The pinned VLC dump invocation

Transcribed from this repo's own clean-room baselines of the pinned
Linux build (`docs/baselines/raw/linux/14-options-full-help.txt`
L2335-2348; version line shape from `04-vlc-version.txt`):

```sh
vlc -I dummy --play-and-exit --vout none --aout afile \
    --no-audiofile-wav --audiofile-format {u8|s16|float32} \
    --audiofile-file OUT INPUT
```

`afile` = "File audio output": `--audiofile-format {u8,s16,float32,spdif}`,
`--audiofile-channels 0` keeps all incoming channels (passthrough — the
pin), `--audiofile-wav` off ⇒ headerless raw PCM. `s16`/`float32` are
native-endian ⇒ the comparable set is LE-wire PCM on an LE host (the
endianness is recorded in every artifact; a BE host makes rows
non-comparable with that reason).

**[VERIFY]** the flags are transcribed from the pinned build's help
text; the first real-oracle run must confirm dump byte-counts equal
Lumen's `pcm_bytes` on a known-good fixture before any MATCH is
trusted. Doc 10 §6.6's transcode-determinism open question applies to
everything beyond passthrough.

## Comparable set (v1)

Lossless end-to-end only (doc 10 §6.6): **PCM audio (`pcm_u8`,
`pcm_s16le`, `pcm_f32le`) in AVI** — the landed first vertical
(D-018), where Lumen's decoded bytes are the passthrough wire bytes
and VLC's `afile` dump is the same family. The comparison is over the
**first audio track's decoded PCM**, so the video codec does NOT gate
it: the v0 demuxer filters unsupported video streams (one `Recovered`
decision record each) and still walks the audio — corpus-verified
(mpeg4-video AVIs with PCM audio digest fine; when nothing at all is
supported the demuxer refuses at start and the row shows that typed
refusal). Everything else gets an explicit non-comparable row with the
reason (lossy codecs; containers/decoders not landed;
`pcm_s24le`/`pcm_s32le`/`pcm_f64le` have no `afile` output format;
corrupt/garbage inputs are v1-excluded — their comparison is the
error-class rule, a later card).

## Verdicts

| verdict | meaning |
|---|---|
| `MATCH` / `MISMATCH` | oracle and Lumen digests (+ byte counts) agree or differ |
| `LUMEN-REFUSED` | the oracle answered but the Lumen side typed-refused — a real difference, never a MATCH |
| `ORACLE-ERROR` | the oracle invocation failed (message in the cell) |
| `PENDING-VLC` | no oracle in the environment; the cell carries the exact to-run command instead |

Every mock-run verdict and row carries the `[MOCK]` label (mechanically
asserted by the report writer); pending runs assert zero oracle digests.

## Per-row accounting

Each Lumen row records: md5, pcm bytes, frames, packets, codec facts
(channels/rate/sample/layout), the digested track ordinal, and the
digest-integrity counters — `concealed` (decoder damage concealed),
`demux_recovered` (Recovered events; includes the codec filter's
benign decision records), `demux_skip_bytes` (bytes the walk skipped:
0 for pure decisions; on this build the filtered streams' idx1 drops
ride here at 16 B/entry — seek-table only), `format_changes`. Rows
with nonzero skip/conceal/format-change counters are flagged in the
report's Digest-integrity section.

## Usage

```sh
# Real oracle (requires the pinned VLC 3.0.23 on PATH):
python3 tools/bench-vs-vlc/run.py \
    --manifest /path/to/corpus-manifest.json \
    --corpus-root /path/to/corpus \
    --oracle vlc --out docs/corpus/vlc-diff-report-<date>.md

# No VLC present (PENDING-VLC rows + exact commands; zero oracle digests):
python3 tools/bench-vs-vlc/run.py ... --oracle pending --out REPORT.md \
    --note "environment facts, install attempts, verification results"

# Mock (plumbing validation only — every label says MOCK):
python3 tools/bench-vs-vlc/run.py --files tests/fixtures/generated/valid/*.avi \
    --oracle mock --out /tmp/mock-report.md

# The Lumen side alone (what run.py drives internally):
LUMEN_DIFF_FIXTURES=list.tsv LUMEN_DIFF_OUT=out.json \
    cargo test --manifest-path tools/bench-vs-vlc/Cargo.toml \
        --test lumen_pcm -- --ignored --nocapture

# Unit tests:
python3 -m unittest discover -s tools/bench-vs-vlc -v
cargo test --manifest-path tools/bench-vs-vlc/Cargo.toml
```

Input is either `--manifest` (a corpus manifest JSON; classification
from its recorded params) or `--files` (explicit media files,
classified via ffprobe; paths under `--corpus-root` become
corpus-relative ids, and `dirty/`/`garbage/` top-level directories are
excluded by class). `--note TEXT` (repeatable) records run provenance
in the report's Run notes section. The Lumen batch TSV is
`rel<TAB>abs-path` lines (written next to the output as
`*.lumen.fixtures.tsv`); the Lumen JSON envelope is schema
`bench-vs-vlc/lumen-pcm-digests/1` (host endianness + one row per
fixture: digest, byte/frame/packet counts, codec facts, integrity
counters, or the stage-tagged refusal).

## Reports

* `docs/corpus/vlc-diff-report-20261023.md` — the v1 report (PENDING-VLC
  mode: 500-fixture corpus run, real Lumen digests on all 35 comparable
  rows, exact to-run oracle commands, mock/real separation explicit,
  run notes carrying provenance + independent verification) + siblings:
  `.json` (full evidence), `.lumen.json` (raw Lumen digests),
  `.lumen.fixtures.tsv` (the batch input).
