#!/usr/bin/env python3
"""packaging-env.py — config-driven packaging-env injection (issue #56,
cards 8.E1–8.E3; task W24-C).

The bridge between the two config files and every packaging script:

  product.toml          (product identity — name, version, publisher,
   └ scripts/release/     per-OS ids, MSI UpgradeCode, accent, bin
      product-env.py      names via cargo metadata)      [wave-22 tool]
  packaging/packaging.toml + packaging.schema.json
                          (packaging plane — install dirs, file lists,
                           checksum algorithm, signing secret names,
                           desktop-entry categories, launch-test verbs)
                           [this lane, validated HERE]

Nothing else: packaging scripts carry ZERO identity/packaging literals;
they read the environment this script emits. A missing or invalid value
is a hard error with an actionable message — never a silent fallback
(same posture as product-env.py and product.schema.json).

Output modes:
  --print                KEY=VALUE lines on stdout
  --shell                KEY='value' shell-sourceable lines on stdout
                         (single-quoted, embedded quotes escaped — safe
                         to `eval`/source even with spaces in values)
  --github-env FILE      append KEY=VALUE lines to FILE (GitHub Actions
                         environment-file format)

Semantic pass (beyond the structural schema subset): shipped docs and
the launch-test fixture exist in the repo; the secret env-var names are
pairwise distinct; the packaging tree carries no product-identity
duplication (identity fields are absent from packaging.schema.json by
construction — this script re-asserts the two files do not overlap on
identity keys).

Usage (from anywhere; the repo root is resolved from this file):
  python3 scripts/packaging/packaging-env.py \
      --cli-package lumen-cli --desktop-package desktop --shell
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
PRODUCT_ENV = REPO_ROOT / "scripts" / "release" / "product-env.py"
PACKAGING_TOML = REPO_ROOT / "packaging" / "packaging.toml"
PACKAGING_SCHEMA = REPO_ROOT / "packaging" / "packaging.schema.json"

# The structural-keyword subset this validator enforces. Anything else
# in the schema is a hard error: extending the schema means extending
# this validator in the same change (no silently-ignored constraints).
SUPPORTED_KEYWORDS = {
    "type", "required", "additionalProperties", "properties", "items",
    "enum", "const", "minimum", "maximum", "minLength", "minItems",
    "maxItems", "uniqueItems", "pattern", "$comment", "x-lumen-hint",
    "$schema", "$id", "title",
}


def die(msg: str) -> None:
    print(f"packaging-env: error: {msg}", file=sys.stderr)
    raise SystemExit(1)


def load_toml(path: Path, what: str) -> dict:
    try:
        with path.open("rb") as fh:
            return tomllib.load(fh)
    except FileNotFoundError:
        die(f"{what} not found at {path}")
    except tomllib.TOMLDecodeError as exc:
        die(f"{what} at {path} is not valid TOML: {exc}")
    raise AssertionError("unreachable")


# ---------------------------------------------------------------------------
# Structural JSON-schema (subset) validation
# ---------------------------------------------------------------------------

def check_type(value, expected: str, where: str) -> None:
    ok = {
        "object": lambda v: isinstance(v, dict),
        "array": lambda v: isinstance(v, list),
        "string": lambda v: isinstance(v, str),
        "integer": lambda v: isinstance(v, int) and not isinstance(v, bool),
        "boolean": lambda v: isinstance(v, bool),
    }[expected]
    if not ok(value):
        die(f"{where}: expected {expected}, got "
            f"{type(value).__name__} ({value!r})")


def validate(value, schema: dict, where: str) -> None:
    for keyword in schema:
        if keyword not in SUPPORTED_KEYWORDS:
            die(f"packaging.schema.json uses unsupported keyword "
                f"{keyword!r} at {where} — extend the validator in the "
                f"same change that extends the schema (no silently "
                f"ignored constraints)")

    if "type" in schema:
        check_type(value, schema["type"], where)

    if "const" in schema and value != schema["const"]:
        die(f"{where}: must be the constant {schema['const']!r}, got {value!r}")

    if "enum" in schema and value not in schema["enum"]:
        die(f"{where}: {value!r} is not one of {schema['enum']}")

    if isinstance(value, str):
        if "pattern" in schema:
            try:
                matched = re.fullmatch(schema["pattern"], value) is not None
            except re.error as exc:
                die(f"{where}: schema pattern {schema['pattern']!r} is not "
                    f"a valid regex: {exc}")
            if not matched:
                hint = schema.get("x-lumen-hint", "")
                die(f"{where}: {value!r} does not match "
                    f"{schema['pattern']!r}{f' ({hint})' if hint else ''}")
        if "minLength" in schema and len(value) < schema["minLength"]:
            die(f"{where}: shorter than minLength {schema['minLength']}")

    if isinstance(value, int) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            die(f"{where}: below minimum {schema['minimum']}")
        if "maximum" in schema and value > schema["maximum"]:
            die(f"{where}: above maximum {schema['maximum']}")

    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            die(f"{where}: fewer than minItems {schema['minItems']}")
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            die(f"{where}: more than maxItems {schema['maxItems']}")
        if schema.get("uniqueItems") and len(value) != len(set(map(str, value))):
            die(f"{where}: items are not unique")
        if "items" in schema:
            for i, item in enumerate(value):
                validate(item, schema["items"], f"{where}[{i}]")

    if isinstance(value, dict):
        for key in schema.get("required", []):
            if key not in value:
                die(f"{where}: required key {key!r} is missing")
        props = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            for key in value:
                if key not in props:
                    die(f"{where}: unknown key {key!r} "
                        f"(additionalProperties: false)")
        for key, sub in props.items():
            if key in value:
                validate(value[key], sub, f"{where}.{key}")


# ---------------------------------------------------------------------------
# Config loading + derivation
# ---------------------------------------------------------------------------

def product_identity_env(cli_package: str | None,
                         desktop_package: str | None) -> dict[str, str]:
    """Run scripts/release/product-env.py and parse its KEY=VALUE lines.

    Identity derivation (including the cargo-metadata bin-name lookup
    and the numeric MSI version) stays owned by that wave-22 tool; this
    lane re-uses it rather than duplicating it. Its absence or failure
    is fatal.
    """
    if not PRODUCT_ENV.is_file():
        die(f"product-env.py not found at {PRODUCT_ENV} "
            f"(wave-22 release tooling is a prerequisite of this lane)")
    cmd = [sys.executable, str(PRODUCT_ENV), "--print"]
    if cli_package:
        cmd += ["--cli-package", cli_package]
    if desktop_package:
        cmd += ["--desktop-package", desktop_package]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        die(f"product-env.py failed:\n{proc.stderr.strip()}")
    env: dict[str, str] = {}
    for line in proc.stdout.splitlines():
        if not line or "=" not in line:
            die(f"product-env.py emitted an unparseable line: {line!r}")
        key, val = line.split("=", 1)
        env[key] = val
    return env


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--cli-package",
                    help="cargo package name of the CLI app (passthrough to product-env.py)")
    ap.add_argument("--desktop-package",
                    help="cargo package name of the desktop app (passthrough to product-env.py)")
    ap.add_argument("--print", dest="mode", action="store_const",
                    const="print", default="print")
    ap.add_argument("--shell", dest="mode", action="store_const", const="shell")
    ap.add_argument("--github-env", dest="mode", action="store_const", const="github-env")
    ap.add_argument("--github-env-file", dest="github_env_file")
    args = ap.parse_args()

    if args.mode == "github-env" and not args.github_env_file:
        die("--github-env mode requires --github-env-file")

    # --- 1. product identity (via the wave-22 tool) -----------------------
    env = product_identity_env(args.cli_package, args.desktop_package)

    # Brand icon paths live in product.toml [brand] but are not exported
    # by product-env.py (the wave-22 scripts generate placeholder icons
    # from the accent instead). This lane exports them so every
    # packaging script can prefer the real asset when it exists and fall
    # back to the documented placeholder otherwise. The help URL joins
    # for the same reason: signtool's /du description-URL field.
    product = load_toml(REPO_ROOT / "product.toml", "product.toml")
    brand = product.get("brand", {})
    for key in ("icon_ico", "icon_icns", "icon_png_512", "logo_svg"):
        val = brand.get(key)
        if not isinstance(val, str) or not val:
            die(f"product.toml brand.{key} is missing or not a string")
        env[f"PRODUCT_{key.upper()}"] = val
    urls = product.get("urls", {})
    help_url = urls.get("help")
    if not isinstance(help_url, str) or not help_url:
        die("product.toml urls.help is missing or not a string")
    env["PRODUCT_URLS_HELP"] = help_url

    # --- 2. packaging plane (schema-validated) ----------------------------
    schema = json.loads(PACKAGING_SCHEMA.read_text(encoding="utf-8"))
    packaging = load_toml(PACKAGING_TOML, "packaging/packaging.toml")
    validate(packaging, schema, "packaging.toml")

    # Semantic pass: referenced repo files exist.
    for doc in packaging["shipped"]["docs"]:
        if not (REPO_ROOT / doc).is_file():
            die(f"packaging.toml shipped.docs lists {doc!r} — not found "
                f"in the repo (semantic pass)")
    fixture = packaging["launch_test"]["smoke_fixture"]
    if not (REPO_ROOT / fixture).is_file():
        die(f"packaging.toml launch_test.smoke_fixture {fixture!r} — not "
            f"found in the repo (semantic pass)")
    if env.get("PRODUCT_NAME", "").lower() in json.dumps(packaging).lower():
        # Guard the single-source rule: identity prose must not be
        # duplicated into the packaging file.
        die("packaging.toml appears to contain the product name — "
            "identity lives in product.toml only (MP §5)")
    secret_names = [
        packaging["signing"]["windows"]["pfx_b64_env"],
        packaging["signing"]["windows"]["pfx_password_env"],
        packaging["signing"]["macos"]["identity_env"],
        packaging["signing"]["macos"]["p12_b64_env"],
        packaging["signing"]["macos"]["p12_password_env"],
        packaging["signing"]["macos"]["notary_key_id_env"],
        packaging["signing"]["macos"]["notary_key_b64_env"],
        packaging["signing"]["macos"]["notary_issuer_env"],
    ]
    if len(secret_names) != len(set(secret_names)):
        die("packaging.toml signing: secret env-var names collide "
            "(must be pairwise distinct)")

    # --- 3. flatten packaging.toml -> PACKAGING_* -------------------------
    def put(key: str, val) -> None:
        env[f"PACKAGING_{key}"] = val

    put("CHECKSUM_ALGORITHM", packaging["checksum"]["algorithm"])
    put("CHECKSUM_MANIFEST", packaging["checksum"]["manifest_name"])
    put("SBOM_SPEC_VERSION", packaging["sbom"]["spec_version"])
    put("SBOM_ARTIFACT_SUFFIX", packaging["sbom"]["artifact_suffix"])
    put("DOCS", " ".join(packaging["shipped"]["docs"]))
    put("DESKTOP_CATEGORIES", packaging["desktop_entry"]["categories"])
    put("DESKTOP_TERMINAL", str(packaging["desktop_entry"]["terminal"]).lower())
    linux = packaging["linux"]
    put("LINUX_BINDIR", linux["install_bindir"])
    put("LINUX_DOCS_DIR", linux["docs_dir"])
    put("LINUX_APPLICATIONS_DIR", linux["applications_dir"])
    put("LINUX_ICONS_DIR", linux["icons_dir"])
    put("DEB_SECTION", linux["deb_section"])
    put("RPM_RELEASE", linux["rpm_release"])
    put("RPM_LICENSE", linux["rpm_license"])
    macos = packaging["macos"]
    put("MACOS_UNIVERSAL", str(macos["universal"]).lower())
    put("MACOS_TARGETS", " ".join(macos["targets"]))
    windows = packaging["windows"]
    put("WINDOWS_BINDIR", windows["install_bindir"])
    put("WINDOWS_UPGRADE_TEST", str(windows["upgrade_test"]).lower())
    launch = packaging["launch_test"]
    put("LAUNCH_VERSION_FLAG", launch["version_flag"])
    put("LAUNCH_SMOKE_VERB", launch["smoke_verb"])
    put("LAUNCH_SMOKE_FIXTURE", launch["smoke_fixture"])
    put("LAUNCH_DESKTOP_PENDING_EXIT", str(launch["desktop_pending_exit"]))
    win_sign = packaging["signing"]["windows"]
    put("SIGNING_WINDOWS_TIMESTAMP_URL", win_sign["timestamp_url"])
    put("SIGNING_WINDOWS_PFX_B64_ENV", win_sign["pfx_b64_env"])
    put("SIGNING_WINDOWS_PFX_PASSWORD_ENV", win_sign["pfx_password_env"])
    mac_sign = packaging["signing"]["macos"]
    put("SIGNING_MACOS_IDENTITY_ENV", mac_sign["identity_env"])
    put("SIGNING_MACOS_P12_B64_ENV", mac_sign["p12_b64_env"])
    put("SIGNING_MACOS_P12_PASSWORD_ENV", mac_sign["p12_password_env"])
    put("SIGNING_MACOS_NOTARY_KEY_ID_ENV", mac_sign["notary_key_id_env"])
    put("SIGNING_MACOS_NOTARY_KEY_B64_ENV", mac_sign["notary_key_b64_env"])
    put("SIGNING_MACOS_NOTARY_ISSUER_ENV", mac_sign["notary_issuer_env"])

    # --- 4. emit -----------------------------------------------------------
    for key, val in env.items():
        if "\n" in val or "\r" in val:
            die(f"derived {key} contains a newline "
                f"(GITHUB_ENV would corrupt)")

    if args.mode == "print":
        for key, val in env.items():
            print(f"{key}={val}")
    elif args.mode == "shell":
        for key, val in env.items():
            # Single-quote every value; escape embedded single quotes
            # with the '"'"' idiom. Values with spaces stay one word.
            escaped = val.replace("'", "'\\''")
            print(f"{key}='{escaped}'")
    else:
        with open(args.github_env_file, "a", encoding="utf-8") as fh:
            for key, val in env.items():
                fh.write(f"{key}={val}\n")
        print(f"packaging-env: appended {len(env)} variables "
              f"to {args.github_env_file}")


if __name__ == "__main__":
    main()
