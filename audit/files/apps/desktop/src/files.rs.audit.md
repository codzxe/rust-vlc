# AUDIT: apps/desktop/src/files.rs
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-24 |
| Lines / functions | 236 / 5 (3 classifiers + 2 trait impls, 2 traits) + 5 tests |
| File verdict | REAL for drag-drop path logic; the dialog half is UNREACHABLE in every shipped build |
| Reachable from shipped entry point? | PARTIAL: `PathDragDrop`/`is_*_path` YES — `apps/desktop-tauri/src/app.rs:314` (`let mut handler = PathDragDrop;` → `runtime.on_files_dropped`) and `app.rs:60` argv → `open_path`. `NativeDialog`/`pick_media_file` NO — the tauri app explicitly does not wire it (app.rs:78-81), the `lumen-desktop` binary never calls it (main.rs prints pending state, exits 86), the only caller of `pick_and_open` is shell.rs's own test (:399 ScriptedDialog). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `target/debug/lumen open tests/fixtures/generated/valid/aifc_sowt_s16le_44k1_st.aifc` (ok: accepted — engine opens what the drop filter rejects); `lumen open …/mp4_3gp_h263.3gp` (typed no-decoder, demux opens); `--control-stdio` playlist.load probe → `unsupported-for-host`; sed quote verification; rg caller searches. |

## 1. One-paragraph verdict
The drag-drop half is real, tested, and wired into the actual tauri composition root:
paths are classified and funneled into catalog commands exactly as ARCH-5 demands, and
the classifier tests are mutation-sensitive. Three defects: the module doc's central
promise — "The tauri build swaps the test impls for the plugin-backed ones at the
composition root" — is false for the dialog half (the tauri app wires NO dialog and no
plugin exists anywhere in the tree; `pick_and_open` is unreachable from every shipped
entry point); the media-suffix pre-filter silently drops engine-supported formats
(executed: an `.aifc` the CLI opens fine is a no-op when dropped); and the "playlist
drops load the list" flow dead-ends — `playlist.load` is refused `unsupported-for-host`
by the only dispatch there is (executed over the wire), which the file's doc and its
test both omit to mention.

## 2. What actually works (verified)
— Classifier tests are real and pass (baseline cargo-test-workspace.txt; drop→SessionOpen,
no-op, case-insensitivity, all 5 playlist suffixes, dotfile/no-ext negatives).
— `PathDragDrop` is the production drop handler in the real GUI:
`apps/desktop-tauri/src/app.rs:314` (verified by read; the tauri build itself fails at
gdk-sys in this sandbox — baseline cargo-build-desktop-tauri.txt, environment limit).
— The ARCH-5 rule holds: both impls build `WireRequest`s through
`open_path_request` (apps/desktop/src/dispatch.rs:253-261), no shell-side engine calls.

## 3. Findings

F-001 [S2]
Where: apps/desktop/src/files.rs:18-24 (module doc) + :120-124 (NativeDialog)
Code:
```
//! [`HostError::WebviewPending`] state and the **test impls** carry
//! the logic proof. The tauri build swaps the test impls for the
//! plugin-backed ones at the composition root — the seam, not the
//! rule, is what moves.
```
```
impl NativeFileDialog for NativeDialog {
    fn pick_media_file(&mut self) -> Result<Option<PathBuf>, HostError> {
        Err(HostError::WebviewPending { adr: "ADR-004" })
    }
}
```
What is wrong: the promised swap does not exist. The real composition root
(`apps/desktop-tauri/src/app.rs:78-81`) states verbatim:
```
//! `pick_and_open` (native dialog) is NOT wired here: it needs a dialog
//! plugin admission of its own (capabilities stay `core:default` only,
```
`rg tauri-plugin-dialog` over the tree matches only this file's doc comment — no
dependency, no plugin-backed impl, no caller. `ShellRuntime::pick_and_open`
(shell.rs:329) is invoked only by its own test (ScriptedDialog :399). The typed
`WebviewPending` refusal is honest, but the doc describes a swap site that was never
built, and the whole dialog seam is dead code in every shipped lane.
Why it matters: a reader (or the 4.H1 card's AC) believes the dialog joins at the
wrapper; it does not — the open-dialog feature exists nowhere.
What correct looks like: doc says "no dialog impl exists in any build; the seam
awaits plugin admission" and the dead impl is marked as such (or removed until wired).
Confidence: HIGH — Verification: read (caller search + app.rs + Cargo.toml grep);
tauri build UNVERIFIED beyond baseline's gdk-sys failure (environment limit).

F-002 [S3]
Where: apps/desktop/src/files.rs:39-42 (in `is_media_path`)
Code:
```
    const MEDIA_SUFFIXES: &[&str] = &[
        "avi", "mkv", "mp4", "mov", "webm", "m4v", "mpg", "mpeg", "ts", "flv", "wmv", "wav", "mp3",
        "flac", "ogg", "opus", "aac", "m4a", "aiff",
    ];
```
What is wrong: the pre-filter denies engine-supported formats. The demux layer
supports the AIFF family (`.aifc`, `.aif`), `.adts`, `.mp2`, `.ps`, `.es`, `.ivf`,
`.3gp` (crates/demux/src/{aiff,adts,ps,es,ivf,mp4}). Executed:
`lumen open tests/fixtures/generated/valid/aifc_sowt_s16le_44k1_st.aifc` →
`ok: accepted session=1 gen=0` (full playback works), yet `is_media_path("…aifc")` is
false, so dropping that same file on the desktop window is a silent `Ok(None)` no-op —
no session, no error, no feedback. Why it matters: "the open path owns format truth"
(:34-35) is inverted here — the pre-filter overrides the open path's truth for a
decodable file. What correct looks like: accept-by-default with a small deny-list
(matching open-path.ts's "Anything else is media"), or regenerate the list from the
demux registry. Confidence: HIGH — Verification: ran + read.

F-003 [S3]
Where: apps/desktop/src/files.rs:49-51 (doc on `is_playlist_path`) + test :177-208
Code:
```
/// surface: M3U/M3U8, PLS, XSPF, CUE). A playlist file does not open
/// a session — it loads the list (`playlist.load`; 4.D1's handoff,
```
What is wrong: the "loads the list" flow cannot complete. The desktop's only dispatch
delegates to the CLI's (`apps/desktop/src/dispatch.rs:211-213`
`lumen_cli::dispatch::lumen_dispatch(&mut shell.host, request)`), whose SERVED table
(dispatch.rs:38-54) does not include `playlist.load`. Executed over the wire:
`{"cmd":"playlist.load",…}` → `{"error":{"code":"unsupported-for-host","message":
"`playlist.load` is not served by this build's command-line host (the playlist model
is a Phase-4 card)"}}`. So dropping an `.m3u` on the real GUI answers a typed refusal;
the file's doc narrates a working handoff and the test
(`playlist_drops_load_the_list_not_a_session`) asserts only that the REQUEST is built
— it never dispatches it. Honest refusal, dishonest framing.
What correct looks like: doc/test state the terminal reality (drop → typed refusal
until Phase-4), or the classifier demotes playlist files to no-op until the command
is served. Confidence: HIGH — Verification: ran (wire probe) + read.

F-004 [S4]
Where: apps/desktop/src/files.rs:51 ("Mirrors ui/src/client/open-path.ts exactly") + :134
Code:
```
/// parse engine-side). Mirrors ui/src/client/open-path.ts exactly.
```
What is wrong: the PLAYLIST rule mirrors exactly (verified: open-path.ts:19-25 same
5 suffixes; dotfile semantics match, test :218), but the media side does not:
open-path.ts has no media filter — `classifyOpenSource` (open-path.ts:45-50) sends
EVERY non-playlist file to `session.open` ("Anything else is media", :17) — while
files.rs pre-filters. Same path, two acquisition sides, two behaviors (drop README.md
= silent no-op; open it from the UI = typed session.open error). The comment's
"exactly" overclaims; also the test name `drops_become_the_open_command_first_media_wins`
(:134) misnomers the impl, which is first-OPENABLE (playlist or media, :108).
Confidence: HIGH — Verification: read (both files diffed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| is_media_path | 38-46 | REAL | S3 | F-002 suffix list denies engine formats |
| is_playlist_path | 53-58 | REAL | — | mirrors TS exactly; dotfile semantics correct |
| is_openable_path | 63-65 | REAL | — | union |
| trait DragDropHandler | 68-82 | REAL | — | documented Ok(None) no-op contract |
| trait NativeFileDialog | 85-92 | REAL seam | — | doc'd pending state |
| PathDragDrop::on_files_dropped | 102-113 | REAL | — | first-openable; wired in tauri app :314 |
| NativeDialog::pick_media_file | 120-124 | REAL (typed refusal) | S2 | F-001: unreachable in every shipped build; only impl in tree |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| drops_become_the_open_command_first_media_wins :134 | YES | asserts full WireRequest shape incl. request_id echo and Path source |
| drops_without_media_are_noops_not_errors :160 | YES | Ok(None) + case-insensitive .MP3 |
| playlist_drops_load_the_list_not_a_session :177 | PARTIAL | proves request construction only; the dispatch dead-end (F-003) is outside its net — the test PASSES while the user-visible flow fails |
| path_classification_is_exact :213 | YES | suffix/dotfile/no-ext negatives |
| native_dialog_is_the_typed_pending_state :227 | YES | pins the refusal — but nothing tests that any real dialog ever opens (nothing can; F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The tauri build swaps the test impls for the plugin-backed ones at the composition root" | files.rs:22-24 | FALSE | app.rs:78-81 not wired; no tauri-plugin-dialog anywhere (rg); F-001 |
| "A playlist file … loads the list (`playlist.load`)" | files.rs:49-50 | FALSE (end-to-end) | wire probe → unsupported-for-host; F-003 |
| "Mirrors ui/src/client/open-path.ts exactly" | files.rs:51 | PARTIAL | playlist rule yes; media filter has no TS twin; F-004 |
| "the shell's pre-filter so a dropped README never becomes a session.open error" | files.rs:34-36 | REAL | Ok(None) verified in tests — but it also swallows decodable media (F-002) |
| "A file open is a command, never a shell-side engine call (ARCH-5)" | files.rs:5-8 | REAL | open_path_request funnel; shell.rs:308-325 dispatches |

## 7. Dead / duplicated / unreachable code
- `NativeDialog` + `ShellRuntime::pick_and_open`: unreachable from every shipped entry
point (F-001). `NativeFileDialog` trait has exactly one impl (the refusal).
- No duplication; the classifier is deliberately single-sourced (TS twin noted).

## 8. Dependencies & flags
Std-only (Path/PathBuf) + commands crate. No feature gates; ships in the desktop
crate's default lane. The `// lumen-allow-literal` markers on test path vectors are
present and correctly scoped (contrast: host.rs:1799's missing marker, Wave 1).

## 9. Open questions for the next agent
- Who owns the media-suffix list when the demux registry grows? Suggest generating
from crates/demux (F-002's fix).
- apps/desktop-tauri (W-someone's assignment?): confirm the drag-drop path's receipt
handling — app.rs:315-318 observes the receipt but the UI truth is the event stream;
the playlist refusal (F-003) surfaces to the user only if the UI renders the error
envelope. UNVERIFIED (no webview build possible here).
