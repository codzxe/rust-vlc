#!/bin/sh
# gate.sh — the packaging gate (issue #56, cards 8.E1–8.E3, task W24-C;
# scripts/full_gate.sh step 11 calls THIS).
#
# Cheap, sandbox-runnable, HONEST about what it can prove:
#   1. packaging/packaging.toml validates against
#      packaging/packaging.schema.json (packaging-env.py's structural
#      subset validator + semantic pass) AND the product bridge resolves
#      (product.toml -> locale catalog -> cargo metadata bin names) —
#      this is the config/schema validation the card requires.
#   2. Every scripts/packaging/*.sh parses under BOTH bash -n and
#      sh -n (the round-1 heredoc-splice bug was caught exactly here —
#      never again).
#   3. packaging-env.py + the config files py_compile.
#   4. The PowerShell lane is sanity-CHECKED, not executed (no pwsh in
#      this sandbox): balanced braces/parens/quotes per file, no stray
#      non-ASCII smart quotes, and every `param(` block parses as a
#      single balanced expression. Honest label: structural check, not
#      a PowerShell parse; the real parse runs on the Windows runner
#      [VERIFY at first live run].
#   5. The deb/rpm payload stagers run FOR REAL against a throwaway
#      tree (gen-desktop-entry.sh + stage-linux-payload.sh) — the parts
#      of the Linux lane that need no root and no FUSE. (The full
#      package build + launch tests are verify-linux-artifacts.sh, the
#      release-lane proof — too heavy for the per-commit gate.)
#   6. The .UNSIGNED.txt fallback paths of sign-windows.sh / sign-macos.sh
#      run FOR REAL (credentials absent by construction here): they must
#      write explicit sidecars + ::warning:: and exit 0 — the documented
#      honest-unsigned contract, proven, not asserted.
#
# NOT proven here (labels, by design): wix build / signtool / codesign /
# notarytool / rpmbuild / msiexec — each needs its native platform or
# credentials; the release workflow owns those [VERIFY at first live
# run]. This gate never fakes them.
#
# Usage: scripts/packaging/gate.sh   (from the repo root or anywhere)
# POSIX sh; `bash -n` clean. Exit 0 = pass, 1 = fail.
set -u

# cargo on PATH (product-env.py resolves bin names via cargo metadata);
# same export as scripts/full_gate.sh, so the gate works from any shell.
[ -d "$HOME/.cargo/bin" ] && PATH="$HOME/.cargo/bin:$PATH"
export PATH

ROOT=$(cd "$(dirname -- "$0")/../.." && pwd)
cd "$ROOT" || exit 1

fail=0
step() { printf 'gate.sh: %s\n' "$1"; }
bad() { printf 'gate.sh: FAILED: %s\n' "$1" >&2; fail=1; }

# --- 1. config validation (schema + semantics + product bridge) --------------
step "1. packaging.toml <-> packaging.schema.json + product bridge"
if env_out=$(python3 scripts/packaging/packaging-env.py \
        --cli-package lumen-cli --desktop-package desktop --print 2>&1); then
    varcount=$(printf '%s\n' "$env_out" | wc -l | tr -d ' ')
    step "   packaging-env.py: ${varcount} variables resolved (schema + semantic pass green)"
else
    bad "packaging-env.py validation failed:
$env_out"
fi

# --- 2. shell syntax (both parsers; catches heredoc-splice class) -------------
step "2. scripts/packaging/*.sh parse under bash -n AND sh -n"
for f in scripts/packaging/*.sh; do
    out=$(bash -n "$f" 2>&1) || bad "bash -n $f: $out"
    out=$(sh -n "$f" 2>&1) || bad "sh -n $f: $out"
done
[ $fail -eq 0 ] && step "   all $(ls scripts/packaging/*.sh | wc -l | tr -d ' ') scripts parse clean"

# --- 3. python syntax ----------------------------------------------------------
step "3. packaging python tools py_compile"
for f in scripts/packaging/*.py; do
    out=$(python3 -m py_compile "$f" 2>&1) || bad "py_compile $f: $out"
done
[ $fail -eq 0 ] && step "   python tools compile"

# --- 4. PowerShell structural sanity (honest: NOT a real parse) ---------------
step "4. scripts/packaging/*.ps1 structural check (no pwsh in this sandbox)"
for f in scripts/packaging/*.ps1; do
    out=$(python3 scripts/packaging/ps1-sanity.py "$f" 2>&1) || bad "$f: $out"
done
[ $fail -eq 0 ] && step "   ps1 structural checks pass (real parse: Windows runner [VERIFY])"

# --- 5. payload stagers, executed for real ------------------------------------
step "5. gen-desktop-entry.sh + stage-linux-payload.sh (throwaway tree)"
if [ $fail -eq 0 ]; then
    stage=$(mktemp -d) || { bad "mktemp failed"; exit 1; }
    # shellcheck disable=SC2016 — eval of packaging-env.py --shell output
    if eval "$(python3 scripts/packaging/packaging-env.py \
            --cli-package lumen-cli --desktop-package desktop --shell)"; then
        # 5a. The stager must REFUSE LOUDLY when the release binaries are
        # absent (config-driven hard error, never a silent empty package).
        if out=$(OS_ID=linux ARCH_ID=aarch64 TARGET_DIR=/nonexistent-gate-target \
                sh scripts/packaging/stage-linux-payload.sh "$stage/payload" 2>&1); then
            bad "payload staging unexpectedly succeeded with no built binaries (should have failed loudly):
$out"
        else
            case "$out" in
                *"missing packaging input"*) step "   stager refuses loudly without built binaries (as designed)" ;;
                *) bad "stager failed for an unexpected reason: $out" ;;
            esac
        fi
        # 5b. The desktop-entry writer needs no binaries — prove it end to
        # end and cross-check the generated content against the config
        # (Categories/Terminal are packaging.toml decisions).
        if out=$(sh scripts/packaging/gen-desktop-entry.sh "$stage/entry.desktop" 2>&1); then
            [ -s "$stage/entry.desktop" ] ||
                bad "gen-desktop-entry.sh wrote an empty file"
            grep -q "^Categories=${PACKAGING_DESKTOP_CATEGORIES}$" "$stage/entry.desktop" ||
                bad "generated desktop entry Categories != packaging.toml desktop_entry.categories"
            grep -q "^Terminal=${PACKAGING_DESKTOP_TERMINAL}$" "$stage/entry.desktop" ||
                bad "generated desktop entry Terminal != packaging.toml desktop_entry.terminal"
            grep -q "^Exec=${BIN_DESKTOP}$" "$stage/entry.desktop" ||
                bad "generated desktop entry Exec != BIN_DESKTOP (cargo metadata)"
            step "   gen-desktop-entry.sh content matches config"
        else
            bad "gen-desktop-entry.sh failed: $out"
        fi
    else
        bad "packaging-env.py --shell evaluation failed"
    fi
    rm -rf "$stage"
else
    step "   skipped (config validation already failed)"
fi

# --- 6. honest-unsigned fallback paths, executed for real ---------------------
step "6. sign-windows.sh / sign-macos.sh .UNSIGNED fallback (credentials absent)"
if [ $fail -eq 0 ]; then
    stage=$(mktemp -d) || { bad "mktemp failed"; exit 1; }
    eval "$(python3 scripts/packaging/packaging-env.py \
        --cli-package lumen-cli --desktop-package desktop --shell)" || fail=1
    if [ $fail -eq 0 ]; then
        # Stand-in artifacts -- the sidecar paths only need real files.
        echo "stand-in artifact for the unsigned-sidecar contract" >"$stage/standin.msi"
        echo "stand-in artifact for the unsigned-sidecar contract" >"$stage/standin.app"
        if out=$(sh scripts/packaging/sign-windows.sh "$stage/standin.msi" 2>&1); then
            [ -s "$stage/standin.msi.UNSIGNED.txt" ] ||
                bad "sign-windows.sh exited 0 but wrote no .UNSIGNED.txt sidecar"
            step "   sign-windows.sh: unsigned sidecar contract holds"
        else
            bad "sign-windows.sh failed in the credentials-absent path: $out"
        fi
        if out=$(sh scripts/packaging/sign-macos.sh --app "$stage/standin.app" 2>&1); then
            [ -s "$stage/standin.app.UNSIGNED.txt" ] ||
                bad "sign-macos.sh exited 0 but wrote no .UNSIGNED.txt sidecar"
            step "   sign-macos.sh: unsigned sidecar contract holds"
        else
            bad "sign-macos.sh failed in the credentials-absent path: $out"
        fi
    fi
    rm -rf "$stage"
else
    step "   skipped (config validation already failed)"
fi

if [ $fail -eq 0 ]; then
    echo "gate.sh: PASS (packaging config + scripts + local-provable halves)"
else
    echo "gate.sh: FAIL" >&2
fi
exit $fail
