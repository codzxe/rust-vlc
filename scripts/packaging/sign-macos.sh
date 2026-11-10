#!/bin/sh
# sign-macos.sh — codesign + notarytool driver for the macOS release
# artifacts (issue #56, card 8.E2, task W24-C).
#
# What this is: a REAL Driver for Apple's signing/notarization CLIs,
# fully config/secrets-driven:
#   * codesign identity: either the env var NAMED in
#     packaging/packaging.toml [signing.macos] identity_env (a
#     preinstalled keychain identity), or a base64 PKCS#12
#     (p12_b64_env) imported into a SCRATCH keychain on the runner —
#     the standard CI dance, cleaned up on exit.
#   * notarization: `xcrun notarytool submit ... --wait` in App Store
#     Connect API-key mode (key id / issuer / base64 .p8 — the three
#     env vars named in the same config), then `xcrun stapler staple`.
#
# EXTERNAL BLOCKER — R-35 (risk register, OPEN, User-decision): no
# Apple Developer account exists for this project ("P8 gate; trigger:
# first signed-build attempt"). Therefore the documented, honest v1
# behavior when credentials are ABSENT:
#   * no signing identity  -> <artifact>.UNSIGNED.txt sidecar + a
#     ::warning:: naming R-35, owner=user, next step (provision the
#     account + Developer ID Application certificate + App Store
#     Connect API key, then set the repository secrets);
#   * identity but no notary credentials -> the artifact IS signed and
#     gets a <artifact>.NOT-NOTARIZED.txt sidecar + warning (signed
#     but Gatekeeper will still require a right-click > Open);
#   * any credential present but a signing/notarization STEP FAILS ->
#     HARD exit. Never silently pretend, never ship half-done.
#
# Usage:
#   sign-macos.sh --app <path-to-.app>   codesign the bundle
#   sign-macos.sh --dmg <path-to-.dmg>   codesign + notarize + staple
#                                        the disk image (the .app inside
#                                        must already be signed — see
#                                        build-dmg.sh for the order)
# macOS only when credentials exist (codesign/notarytool are macOS
# CLIs); the unsigned-label path runs anywhere. POSIX sh; `bash -n`
# clean.
#
# HONEST LOCAL-PROOF BOUNDARY: authored on Linux; the signed path is
# macOS/CI-only [VERIFY at first live run]. The unsigned path
# (sidecar + warning) is exercised locally in gate.sh.

set -eu

: "${PACKAGING_SIGNING_MACOS_IDENTITY_ENV:?sign-macos.sh: PACKAGING_SIGNING_MACOS_IDENTITY_ENV not set (run packaging-env.py first)}"
: "${PACKAGING_SIGNING_MACOS_P12_B64_ENV:?sign-macos.sh: PACKAGING_SIGNING_MACOS_P12_B64_ENV not set}"
: "${PACKAGING_SIGNING_MACOS_P12_PASSWORD_ENV:?sign-macos.sh: PACKAGING_SIGNING_MACOS_P12_PASSWORD_ENV not set}"
: "${PACKAGING_SIGNING_MACOS_NOTARY_KEY_ID_ENV:?sign-macos.sh: PACKAGING_SIGNING_MACOS_NOTARY_KEY_ID_ENV not set}"
: "${PACKAGING_SIGNING_MACOS_NOTARY_KEY_B64_ENV:?sign-macos.sh: PACKAGING_SIGNING_MACOS_NOTARY_KEY_B64_ENV not set}"
: "${PACKAGING_SIGNING_MACOS_NOTARY_ISSUER_ENV:?sign-macos.sh: PACKAGING_SIGNING_MACOS_NOTARY_ISSUER_ENV not set}"
: "${PRODUCT_NAME:?sign-macos.sh: PRODUCT_NAME not set (run packaging-env.py first)}"
: "${PACKAGING_CHECKSUM_MANIFEST:?sign-macos.sh: PACKAGING_CHECKSUM_MANIFEST not set (run packaging-env.py first)}"

die() { echo "sign-macos.sh: error: $*" >&2; exit 1; }

mode=""
target=""
case "${1-}" in
    --app) mode=app; target=${2-} ;;
    --dmg) mode=dmg; target=${2-} ;;
    *) die "usage: $0 --app <path> | --dmg <path>" ;;
esac
[ -n "$target" ] || die "usage: $0 --app <path> | --dmg <path>"
[ -e "$target" ] || die "target not found: $target"

# Read a variable whose NAME comes from config (POSIX indirect read).
config_get() { # $1 = env var name
    eval "printf '%s' \"\${$1-}\""
}

identity=$(config_get "$PACKAGING_SIGNING_MACOS_IDENTITY_ENV")
p12_b64=$(config_get "$PACKAGING_SIGNING_MACOS_P12_B64_ENV")
p12_password=$(config_get "$PACKAGING_SIGNING_MACOS_P12_PASSWORD_ENV")
notary_key_id=$(config_get "$PACKAGING_SIGNING_MACOS_NOTARY_KEY_ID_ENV")
notary_key_b64=$(config_get "$PACKAGING_SIGNING_MACOS_NOTARY_KEY_B64_ENV")
notary_issuer=$(config_get "$PACKAGING_SIGNING_MACOS_NOTARY_ISSUER_ENV")

write_sidecar() { # $1 = artifact, $2 = suffix (.UNSIGNED / .NOT-NOTARIZED), $3.. prose
    sidecar="$1$2.txt"
    {
        echo "This artifact is $3."
        echo ""
        shift 3
        for line in "$@"; do echo "$line"; done
    } >"$sidecar"
    echo "sign-macos.sh: wrote sidecar $sidecar"
}

unsigned_blocker_prose() {
    echo "Reason: no Apple Developer account is provisioned — risk register"
    echo "R-35 (OPEN, User-decision; 'P8 gate; trigger: first signed-build"
    echo "attempt'). Owner: user."
    echo "Next step: enroll in the Apple Developer Program, create a Developer"
    echo "ID Application certificate and an App Store Connect API key, then set"
    echo "the repository secrets this pipeline reads (see"
    echo "docs/packaging/README.md)."
    echo ""
    echo "Integrity: verify the artifact against the release checksum manifest"
    echo "(${PACKAGING_CHECKSUM_MANIFEST} in the same release). Gatekeeper"
    echo "will quarantine a downloaded, unsigned bundle; the documented"
    echo "user workaround is right-click > Open."
}

PY=$(command -v python3 || command -v python) ||
    die "neither python3 nor python found on PATH (p12/key decode needs the stdlib)"

work=""
scratch_keychain=""
cleanup() {
    # `if` guards, not `[ ... ] && cmd` lists: under `set -e` an EXIT
    # trap whose last command returns non-zero CLOBBERS the script's
    # exit status (reproduced: the unsigned path exited 1 after a
    # successful sidecar write because `[ -n "$work" ] && rm` was
    # false with work=""). Never again.
    if [ -n "$scratch_keychain" ] && command -v security >/dev/null 2>&1; then
        security delete-keychain "$scratch_keychain" >/dev/null 2>&1 || true
    fi
    if [ -n "$work" ]; then
        rm -rf "$work"
    fi
}
trap cleanup EXIT INT TERM

# --- Resolve a signing identity ---------------------------------------------
resolve_identity() {
    if [ -n "$identity" ]; then
        printf '%s' "$identity"
        return 0
    fi
    if [ -z "$p12_b64" ]; then
        return 1
    fi
    command -v codesign >/dev/null 2>&1 ||
        die "codesign not found — sign-macos.sh needs macOS when a certificate is present"
    command -v security >/dev/null 2>&1 ||
        die "security not found — p12 import needs macOS"
    [ -n "$p12_password" ] ||
        die "p12 secret is set but its passphrase secret is empty — refusing to half-sign"

    work=$(mktemp -d)
    "$PY" - "$p12_b64" >"$work/cert.p12" <<'PYEOF'
import base64, sys
sys.stdout.buffer.write(base64.b64decode(sys.argv[1], validate=True))
PYEOF
    [ -s "$work/cert.p12" ] || die "base64 p12 decoded to an empty file"

    # Scratch keychain (standard CI dance): create, unlock, import the
    # p12 with codesign allowed, partition-list it so codesign can use
    # the key non-interactively, and prepend it to the search list.
    # Removed by the EXIT trap — never touches the user's default.
    keychain_pw="$(uuidgen)"
    scratch_keychain="$work/build.keychain-db"
    security create-keychain -p "$keychain_pw" "$scratch_keychain" ||
        die "security create-keychain failed"
    security unlock-keychain -p "$keychain_pw" "$scratch_keychain" ||
        die "security unlock-keychain failed"
    security import "$work/cert.p12" -k "$scratch_keychain" \
        -P "$p12_password" -T /usr/bin/codesign ||
        die "security import of the p12 failed (wrong passphrase?)"
    security set-key-partition-list -S apple-tool:,apple: \
        -s -k "$keychain_pw" "$scratch_keychain" ||
        die "security set-key-partition-list failed"
    security list-keychain -d user -s "$scratch_keychain" \
        "$(security list-keychain -d user | sed 's/^[[:space:]]*"\([^"]*\)".*$/\1/' | tr -d '\n')" ||
        die "security list-keychain (search-list prepend) failed"

    # The imported identity's canonical name: "Developer ID Application: <team>"
    resolved=$(security find-identity -v -p codesigning "$scratch_keychain" |
        sed -n 's/^ *[0-9][0-9]*) \([^"]*\) ".*$/\1/p' | head -1)
    [ -n "$resolved" ] || die "no codesigning identity found in the imported p12"
    printf '%s' "$resolved"
}

signing_identity=$(resolve_identity || true)

# --- Unsigned path (no identity at all — R-35) ------------------------------
if [ -z "$signing_identity" ]; then
    echo "::warning::macOS artifact is UNSIGNED — no Apple Developer account (R-35, owner: user; next step: enroll + provision a Developer ID Application certificate and App Store Connect API key, then set the repository secrets). The artifact ships with an .UNSIGNED.txt sidecar; verify via the release checksum manifest."
    prose="NOT code-signed and NOT notarized"
    write_sidecar "$target" ".UNSIGNED" "$prose" "$(unsigned_blocker_prose)"
    exit 0
fi

command -v codesign >/dev/null 2>&1 ||
    die "codesign not found — sign-macos.sh needs macOS when signing"

# --- codesign ----------------------------------------------------------------
case "$mode" in
    app)
        # Hardened runtime + timestamp: both required for notarization.
        codesign --force --sign "$signing_identity" \
            --timestamp --options runtime "$target" ||
            die "codesign failed for $target"
        codesign --verify --strict --verbose=2 "$target" ||
            die "codesign verify failed for $target — refusing to ship a half-signed bundle"
        spctl -a -t open --context context:primary-signature -v "$target" ||
            die "spctl assessment failed for $target (Gatekeeper would reject it)"
        echo "sign-macos.sh: signed + verified $target (identity: $signing_identity)"
        ;;
    dmg)
        codesign --force --sign "$signing_identity" --timestamp "$target" ||
            die "codesign failed for $target"
        codesign --verify --strict "$target" ||
            die "codesign verify failed for $target"
        echo "sign-macos.sh: signed + verified $target"
        ;;
esac

# --- Notarization (dmg carries the signed .app inside it) --------------------
if [ "$mode" != "dmg" ]; then
    exit 0
fi

if [ -z "$notary_key_id" ] || [ -z "$notary_key_b64" ] || [ -z "$notary_issuer" ]; then
    echo "::warning::dmg is SIGNED but NOT NOTARIZED — App Store Connect API credentials absent (R-35, owner: user; next step: provision the API key + set the repository secrets). Gatekeeper will require right-click > Open. Sidecar written."
    write_sidecar "$target" ".NOT-NOTARIZED" \
        "code-SIGNED but NOT notarized" \
        "Reason: App Store Connect API-key credentials are absent (R-35," \
        "owner: user)." \
        "Next step: create an App Store Connect API key (key id + issuer +" \
        ".p8), then set the repository secrets (see" \
        "docs/packaging/README.md)." \
        "Until then the bundle is Developer-ID signed; Gatekeeper accepts" \
        "it via right-click > Open."
    exit 0
fi

command -v xcrun >/dev/null 2>&1 ||
    die "xcrun not found — notarytool needs macOS with Xcode CLT"

[ -n "$work" ] || work=$(mktemp -d)
# Apple requires the key FILE to be named AuthKey_<key-id>.p8.
"$PY" - "$notary_key_b64" "$notary_key_id" >"$work/AuthKey_${notary_key_id}.p8" <<'PYEOF'
import base64, sys
sys.stdout.buffer.write(base64.b64decode(sys.argv[1], validate=True))
PYEOF
[ -s "$work/AuthKey_${notary_key_id}.p8" ] || die "base64 notary key decoded to an empty file"

xcrun notarytool submit "$target" \
    --key "$work/AuthKey_${notary_key_id}.p8" \
    --key-id "$notary_key_id" \
    --issuer "$notary_issuer" \
    --wait || die "notarytool submit --wait failed for $target (see Apple's log id above)"

xcrun stapler staple "$target" ||
    die "stapler failed for $target — notarized but unstapled (offline Gatekeeper would fail)"

xcrun notarytool validate "$target" \
    --key "$work/AuthKey_${notary_key_id}.p8" \
    --key-id "$notary_key_id" \
    --issuer "$notary_issuer" ||
    die "notarytool validate failed for $target"

echo "sign-macos.sh: notarized + stapled + validated $target"
