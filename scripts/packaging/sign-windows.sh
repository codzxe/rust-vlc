#!/bin/sh
# sign-windows.sh — Authenticode signing driver for the Windows release
# artifacts (issue #56, card 8.E1, task W24-C).
#
# What this is: a REAL signtool driver, fully config/secrets-driven.
#   * Certificate material comes from the two secret env vars NAMED in
#     packaging/packaging.toml [signing.windows] (exported here as
#     PACKAGING_SIGNING_WINDOWS_PFX_B64_ENV / _PASSWORD_ENV by
#     packaging-env.py — this script carries no secret-name literal).
#   * The RFC 3161 timestamp server URL comes from the same config
#     (PACKAGING_SIGNING_WINDOWS_TIMESTAMP_URL).
#   * signtool is located via $SIGNTOOL, then the Windows SDK bin
#     glob, then PATH.
#
# EXTERNAL BLOCKER (owner = USER, phase8-cards.md §1 row 2): no
# Authenticode certificate is budgeted for this project. Card 8.E1
# says "[VERIFY] — escalate if absent". Therefore the documented,
# honest v1 behavior when the certificate secret is ABSENT:
#   * every artifact gets a <artifact>.UNSIGNED.txt sidecar stating
#     it is unsigned, why, and how to verify integrity instead
#     (checksums from the release manifest);
#   * a GitHub Actions ::warning:: names the blocker and the next
#     step;
#   * the script exits 0 so the release can ship an explicitly-labeled
#     unsigned artifact set (doc 11 §9.3 item 6: "documented unsigned
#     + checksums").
# When the certificate IS present, any signing or verification failure
# is a HARD failure — never ship a half-signed artifact silently.
#
# Usage: sign-windows.sh <artifact> [artifact...]
# POSIX sh (git-bash on the Windows runner); `bash -n` clean.

set -eu

: "${PACKAGING_SIGNING_WINDOWS_PFX_B64_ENV:?sign-windows.sh: PACKAGING_SIGNING_WINDOWS_PFX_B64_ENV not set (run packaging-env.py first)}"
: "${PACKAGING_SIGNING_WINDOWS_PFX_PASSWORD_ENV:?sign-windows.sh: PACKAGING_SIGNING_WINDOWS_PFX_PASSWORD_ENV not set (run packaging-env.py first)}"
: "${PACKAGING_SIGNING_WINDOWS_TIMESTAMP_URL:?sign-windows.sh: PACKAGING_SIGNING_WINDOWS_TIMESTAMP_URL not set (run packaging-env.py first)}"
: "${PRODUCT_NAME:?sign-windows.sh: PRODUCT_NAME not set (run packaging-env.py first)}"
: "${PRODUCT_URLS_HELP:?sign-windows.sh: PRODUCT_URLS_HELP not set (run packaging-env.py first)}"
: "${PACKAGING_CHECKSUM_MANIFEST:?sign-windows.sh: PACKAGING_CHECKSUM_MANIFEST not set (run packaging-env.py first)}"

[ $# -ge 1 ] || { echo "usage: $0 <artifact> [artifact...]" >&2; exit 2; }

die() { echo "sign-windows.sh: error: $*" >&2; exit 1; }

# Read a variable whose NAME comes from config (POSIX indirect read).
config_get() { # $1 = env var name
    eval "printf '%s' \"\${$1-}\""
}

pfx_b64=$(config_get "$PACKAGING_SIGNING_WINDOWS_PFX_B64_ENV")
pfx_password=$(config_get "$PACKAGING_SIGNING_WINDOWS_PFX_PASSWORD_ENV")

find_signtool() {
    if [ -n "${SIGNTOOL:-}" ] && [ -x "$SIGNTOOL" ]; then
        printf '%s' "$SIGNTOOL"
        return 0
    fi
    if command -v signtool >/dev/null 2>&1; then
        command -v signtool
        return 0
    fi
    # Windows SDK layout: <kit>\10\bin\<version>\<arch>\signtool.exe —
    # pick the lexically newest version (monotonic 10.0.* numbering).
    for base in "/c/Program Files (x86)/Windows Kits/10/bin" "/c/Program Files/Windows Kits/10/bin"; do
        if [ -d "$base" ]; then
            newest=$(ls "$base" 2>/dev/null | sort -V | tail -1)
            for arch in x64 x86; do
                candidate="$base/$newest/$arch/signtool.exe"
                if [ -f "$candidate" ]; then
                    printf '%s' "$candidate"
                    return 0
                fi
            done
        fi
    done
    return 1
}

write_unsigned_sidecar() { # $1 = artifact path
    sidecar="$1.UNSIGNED.txt"
    {
        echo "This artifact is NOT Authenticode-signed."
        echo ""
        echo "Reason: no Windows code-signing certificate is provisioned"
        echo "(phase8-cards.md external-dependency table, card 8.E1; owner: user)."
        echo "Next step: provision an OV/EV code-signing certificate, then set the"
        echo "repository secrets this pipeline reads (see docs/packaging/README.md)."
        echo ""
        echo "Integrity: verify the artifact against the release checksum manifest"
        echo "(${PACKAGING_CHECKSUM_MANIFEST} in the same release)."
    } >"$sidecar"
    echo "sign-windows.sh: wrote unsigned sidecar $sidecar"
}

# --- Unsigned fallback path (secrets absent) -------------------------------
if [ -z "$pfx_b64" ]; then
    echo "::warning::Windows artifacts are UNSIGNED — no code-signing certificate provisioned (card 8.E1 external blocker, owner: user; next step: provision a certificate and set the repository secrets). Each artifact ships with an .UNSIGNED.txt sidecar; verify via the release checksum manifest."
    for artifact in "$@"; do
        [ -f "$artifact" ] || die "artifact not found: $artifact"
        write_unsigned_sidecar "$artifact"
    done
    exit 0
fi

# --- Signed path (secrets present) -----------------------------------------
[ -n "$pfx_password" ] || die "certificate secret is set but the passphrase secret is empty — refusing to half-sign"

SIGNTOOL_BIN=$(find_signtool) || die "signtool not found (set SIGNTOOL, install the Windows SDK, or put signtool on PATH)"

PY=$(command -v python3 || command -v python) ||
    die "neither python3 nor python found on PATH (base64 decode needs the stdlib)"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM

# Decode the base64 PKCS#12 (python stdlib — no base64-flag roulette
# across git-bash / macOS / Linux).
"$PY" - "$pfx_b64" >"$work/cert.pfx" <<'PYEOF'
import base64, sys
sys.stdout.buffer.write(base64.b64decode(sys.argv[1], validate=True))
PYEOF
[ -s "$work/cert.pfx" ] || die "base64 certificate decoded to an empty file"

for artifact in "$@"; do
    [ -f "$artifact" ] || die "artifact not found: $artifact"
    echo "sign-windows.sh: signing $artifact"
    # /fd SHA256 file digest; /tr /td RFC 3161 timestamp (SHA-256);
    # /d /du descriptive names come from the product config env.
    "$SIGNTOOL_BIN" sign \
        /f "$work/cert.pfx" \
        /p "$pfx_password" \
        /fd SHA256 \
        /tr "$PACKAGING_SIGNING_WINDOWS_TIMESTAMP_URL" \
        /td SHA256 \
        /d "$PRODUCT_NAME" \
        /du "$PRODUCT_URLS_HELP" \
        "$artifact" || die "signtool sign failed for $artifact"
    "$SIGNTOOL_BIN" verify /pa /all "$artifact" ||
        die "signtool verify failed for $artifact — refusing to ship a half-signed artifact"
    echo "sign-windows.sh: verified signature on $artifact"
done

echo "sign-windows.sh: signed and verified $# artifact(s)"
