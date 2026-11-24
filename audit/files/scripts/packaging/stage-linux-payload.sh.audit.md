# AUDIT: scripts/packaging/stage-linux-payload.sh
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 99 / 2 (main + die()) |
| File verdict | REAL (staging logic verified; one placeholder branch is the live path today) |
| Reachable from shipped entry point? | YES (packaging lane): build-deb.sh:66 and build-rpm.sh:103 call it directly; gate.sh:139 exercises it in the packaging self-test (baseline: gate.sh PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `bash -n` -> OK (exit 0); `sh scripts/packaging/stage-linux-payload.sh` (no env) -> "PRODUCT_LOWER: ... not set", exit 2 (guard works); `ls assets` -> No such file or directory; sed quote verification |

## 1. One-paragraph verdict
This is a real stager, not a fake one: `set -eu` is on, fourteen required env
vars are guarded with `:?"..."`, missing CLI *and* tauri binaries are hard
`die` errors (no placeholder binary ever staged — the old exit-86 skeleton
stub was deliberately dropped, lines 19-22), and modes are explicit (0755
bins, 0644 docs/desktop/icon). The one hostile-focus item is real: the icon
branch ships a generated placeholder PNG whenever `assets/brand/icon-512.png`
is absent — and that directory does not exist anywhere in this repo, so the
placeholder is the *live* path for every deb/rpm/AppImage staged today. It is
disclosed in comments and clean-room, but it is still a placeholder artifact
in the shipped payload.

## 2. What actually works (verified)
- `bash -n scripts/packaging/stage-linux-payload.sh` -> exit 0.
- Ran it with no environment: died immediately on the first `:?` guard
  (`PRODUCT_LOWER: ... not set`), exit 2 — the cannot-run-silently claim holds.
- `install -m` used for every copied file; `mkdir -p` before; `chmod 0644` on icon.
- Callers confirmed by grep: build-deb.sh:66, build-rpm.sh:103, gate.sh:139.

## 3. Findings
F-001 [S3] — The placeholder-icon fallback is the live shipping path today
Where: scripts/packaging/stage-linux-payload.sh:86-98; product.toml:96-99
Code:
```
icon_dest="$dest/$PACKAGING_LINUX_ICONS_DIR/$PRODUCT_LOWER.png"
if [ -f "$PRODUCT_ICON_PNG_512" ]; then
    install -m 0644 "$PRODUCT_ICON_PNG_512" "$icon_dest"
else
    PY=$(command -v python3 || command -v python) ||
        die "neither python3 nor python found on PATH (icon generation needs the stdlib)"
    "$PY" "$ROOT/scripts/release/gen-icon.py" "$icon_dest" "$PRODUCT_ACCENT_COLOR"
fi
```
What is wrong: `PRODUCT_ICON_PNG_512` resolves to `assets/brand/icon-512.png`
(product.toml:99 `icon_png_512 = "assets/brand/icon-512.png"`), and `ls assets`
fails — the `assets/` directory does not exist in the repo at all. The `-f`
test can never be true on a fresh checkout, so every Linux package icon is
the flat #4f8cff 512x512 square from gen-icon.py. Corroborated by the Wave 0
baseline ("config-validator: FAILURE — 4 missing brand assets") and by
audit/files/apps/desktop-tauri/icons/icon.png.audit.md (W14): every shipped
surface brands as a flat blue square. Why it matters: this is exactly
"packaging ships placeholder artifacts"; it is disclosed (comment lines
24-29; docs/RELEASE.md known limits) and clean-room, so S3 not S2 — but a
release built today ships a placeholder as its only icon. What correct looks
like: land `assets/brand/icon-512.png` (plus ico/icns) or make the missing
brand asset a hard `die` in the release lane instead of a fallback.
Confidence: HIGH — Verification: ran (`ls assets` -> No such file or
directory; branch logic read; gen-icon.py head-verified: stdlib zlib/struct
flat-color PNG writer, deterministic, real code).

F-002 [S4] — gen-icon.py is non-executable by policy accident
Where: scripts/packaging/stage-linux-payload.sh:87-90
Code:
```
# gen-icon.py is invoked THROUGH the interpreter, not executed
# directly: the wave-22 commit carries it as 100644 (no exec bit), so a
# direct call dies with EACCES on a fresh checkout (reproduced here;
# flagged to the Lead for a mode-only fix in scripts/release/).
```
What is wrong: the workaround (invoke via `$PY`) is correct and honest, but
the underlying defect (mode 100644 on a script meant to be run) is admitted
and left in place ("flagged to the Lead"). `ls -l scripts/release/gen-icon.py`
-> `-rw-rw-r--`. Why it matters: any other caller that assumes exec bits
breaks. Limited blast radius: interpreter invocation works.
Confidence: HIGH — Verification: ran (`ls -la`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main (top level) | 34-99 | REAL | — | set -eu; 14 `:?` guards; hard die on missing bins/docs; icon fallback F-001 |
| die() | 54 | REAL | — | stderr + exit 1; used by all failure paths |

## 5. Test honesty
Not a test file. gate.sh:125-139 ("gen-desktop-entry.sh + stage-linux-payload.sh
(throwaway tree)") executes this script in a scratch tree; baseline full_gate
shows `gate.sh: PASS`. That is a real execution of this file in the baseline
(cited, not re-run — creating throwaway trees outside audit/ is out of scope).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "EXECUTED for real in the local 8.E3 proof" (L32-33) | script header | REAL-UNPROVEN | docs/packaging/evidence/w24c-r3.log:133 shows gate.sh step 5 ran it; baseline gate.sh PASS |
| "no directory literal in this script" (L5-6) | header | REAL | read: all dirs from PACKAGING_LINUX_* vars |
| "never a silently-one-binary package" (L18-19) | header | REAL | L62-63 both bins `[ -f ] \|\| die` (quote above) |
| icon "falls back ... until assets/brand exists" (L27-29) | header | REAL (fallback live) | assets/ absent -> placeholder always ships (F-001) |

## 7. Dead / duplicated / unreachable code
None. `OS_ID`/`ARCH_ID` guards (L42-43) are asserted but never read after
L56's OS_ID check — ARCH_ID is validated-for-presence only (dead-ish but
defensive; not worth a finding).

## 8. Dependencies & flags
Depends on packaging-env.py exports (PRODUCT_*, PACKAGING_*), gen-desktop-entry.sh (the single desktop-entry writer), scripts/release/gen-icon.py (stdlib python). No cargo, no network. POSIX sh with `bash -n` clean — verified.

## 9. Open questions for the next agent
- build-targz.sh (my next file) shares this pattern — check whether IT has a placeholder fallback too.
- Who is supposed to create assets/brand/? config-validator failure (4 missing assets) is a known-red gate; the release lane does not block on it.
