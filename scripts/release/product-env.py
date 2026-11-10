#!/usr/bin/env python3
"""product-env.py — config-driven release-env injection (card 7.w22g).

The release workflow and every scripts/release/*.sh packaging script read
product identity (name, version, publisher, per-OS ids, description,
accent color) and the two shipped bin names from HERE — never from
hard-coded literals. Sources, in order of authority:

  * product.toml          — identity, ids, brand accent, default locale
                            (MP §5: product.toml is the single source of
                            truth for product decisions)
  * locales/<locale>.toml — the description prose, via
                            identity.description_key (doc 07 §6.1: prose
                            lives in the locale catalog, never in code)
  * cargo metadata        — bin names of the two shipped packages
                            (apps/cli, apps/desktop), so a bin rename
                            needs no packaging edit

Anything missing is a hard error with an actionable message — never a
silent fallback (same posture as product.schema.json validation).

Output modes:
  --print              KEY=VALUE lines on stdout (shell `eval`-safe only
                       if values are quoted by the caller; prefer the
                       GITHUB_ENV mode)
  --github-env FILE    append KEY=VALUE lines to FILE (GitHub Actions
                       environment-file format: one literal KEY=VALUE per
                       line; values here are single-line by construction)

Also derives:
  MSI_VERSION    numeric-only x.y.z from identity.display_version (strips
                 a pre-release suffix such as "-dev"; MSI ProductVersion
                 tolerates neither suffixes nor >3 numeric fields)
  PRODUCT_LOWER  identity.name, lowercased, spaces->dashes (artifact and
                 package naming slug)

Usage (from the repo root):
  python3 scripts/release/product-env.py \
      --cli-package lumen-cli --desktop-package desktop \
      [--print | --github-env-file "$GITHUB_ENV"]
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


def die(msg: str) -> None:
    print(f"product-env: error: {msg}", file=sys.stderr)
    raise SystemExit(1)


def load_toml(path: Path, what: str) -> dict:
    try:
        with path.open("rb") as fh:
            return tomllib.load(fh)
    except FileNotFoundError:
        die(f"{what} not found at {path} (run from the repo root)")
    except tomllib.TOMLDecodeError as exc:
        die(f"{what} at {path} is not valid TOML: {exc}")
    raise AssertionError("unreachable")


def cargo_bin_name(package: str, metadata: dict) -> str:
    for pkg in metadata["packages"]:
        if pkg["name"] == package:
            bins = [t["name"] for t in pkg.get("targets", []) if t["kind"] and "bin" in t["kind"]]
            if len(bins) != 1:
                die(f"package {package!r} must declare exactly one [[bin]] target "
                    f"(found {bins})")
            return bins[0]
    die(f"package {package!r} not found in cargo metadata")


def msi_version(display_version: str) -> str:
    # MSI ProductVersion: numeric fields only, max three (WiX v5 rejects
    # pre-release suffixes such as "-dev"). Take the numeric prefix, keep
    # at most 3 fields, left-pad missing fields with 0 up to x.y.z.
    numeric = display_version.split("-", 1)[0].split("+", 1)[0]
    fields = numeric.split(".")
    if not all(re.fullmatch(r"[0-9]+", f) for f in fields) or not fields:
        die(
            f"cannot derive a numeric MSI version from display_version "
            f"{display_version!r} (identity.display_version in product.toml)"
        )
    fields = fields[:3]
    while len(fields) < 3:
        fields.append("0")
    return ".".join(fields)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--cli-package",
                    help="cargo package name of the CLI app (its [[bin]] name -> BIN_CLI)")
    ap.add_argument("--desktop-package",
                    help="cargo package name of the desktop app (its [[bin]] name -> BIN_DESKTOP)")
    ap.add_argument("--print", dest="mode", action="store_const", const="print", default="print")
    ap.add_argument("--github-env", dest="mode", action="store_const", const="github-env")
    ap.add_argument("--github-env-file", dest="github_env_file")
    args = ap.parse_args()

    mode = args.mode
    out_file = args.github_env_file
    if mode == "github-env":
        if not out_file:
            die("--github-env mode requires --github-env-file")
        if not (args.cli_package and args.desktop_package):
            die("--github-env mode needs --cli-package and --desktop-package")

    product = load_toml(REPO_ROOT / "product.toml", "product.toml")

    ident = product.get("identity")
    if not isinstance(ident, dict):
        die("product.toml has no [identity] table")
    ids = ident.get("ids", {})
    brand = product.get("brand", {})
    defaults = product.get("defaults", {})

    def need(table: dict, key: str, where: str) -> str:
        val = table.get(key)
        if not isinstance(val, str) or not val:
            die(f"product.toml {where}.{key} is missing or not a string")
        return val

    name = need(ident, "name", "identity")
    short_name = need(ident, "short_name", "identity")
    publisher = need(ident, "publisher", "identity")
    display_version = need(ident, "display_version", "identity")
    version_code = ident.get("version_code")
    if not isinstance(version_code, int):
        die("product.toml identity.version_code is missing or not an integer")
    description_key = need(ident, "description_key", "identity")
    bundle_id = need(ids.get("macos", {}), "bundle_id", "identity.ids.macos")
    appstream_id = need(ids.get("linux", {}), "appstream_id", "identity.ids.linux")
    msi_upgrade_code = need(ids.get("windows", {}), "msi_upgrade_code", "identity.ids.windows")
    aumid = need(ids.get("windows", {}), "app_user_model_id", "identity.ids.windows")
    accent = need(brand, "accent_color", "brand")
    locale = need(defaults, "locale", "defaults")

    # Description prose: the localization catalog, via the key product.toml
    # points at (doc 07 §6.1). The catalog uses quoted top-level keys such
    # as "product.description".
    locale_path = REPO_ROOT / "locales" / f"{locale}.toml"
    catalog = load_toml(locale_path, f"locales/{locale}.toml")
    if description_key not in catalog:
        die(f"locales/{locale}.toml has no key {description_key!r} "
            f"(identity.description_key in product.toml)")
    description = catalog[description_key]
    if not isinstance(description, str) or not description:
        die(f"locales/{locale}.toml {description_key!r} is empty")

    env: dict[str, str] = {
        "PRODUCT_NAME": name,
        "PRODUCT_SHORT_NAME": short_name,
        "PRODUCT_PUBLISHER": publisher,
        "PRODUCT_VERSION": display_version,
        "PRODUCT_VERSION_CODE": str(version_code),
        "PRODUCT_DESCRIPTION": description,
        "PRODUCT_LOWER": re.sub(r"\s+", "-", name.strip()).lower(),
        "PRODUCT_BUNDLE_ID": bundle_id,
        "PRODUCT_APPSTREAM_ID": appstream_id,
        "PRODUCT_MSI_UPGRADE_CODE": msi_upgrade_code,
        "PRODUCT_AUMID": aumid,
        "PRODUCT_ACCENT_COLOR": accent,
        "MSI_VERSION": msi_version(display_version),
    }

    if args.cli_package or args.desktop_package:
        try:
            meta_raw = subprocess.run(
                ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
                cwd=REPO_ROOT, capture_output=True, text=True, check=True,
            ).stdout
        except FileNotFoundError:
            die("cargo not found on PATH (needed to resolve bin names)")
        except subprocess.CalledProcessError as exc:
            die(f"cargo metadata failed: {exc.stderr.strip()}")
        metadata = json.loads(meta_raw)
        if args.cli_package:
            env["BIN_CLI"] = cargo_bin_name(args.cli_package, metadata)
        if args.desktop_package:
            env["BIN_DESKTOP"] = cargo_bin_name(args.desktop_package, metadata)

    for key, val in env.items():
        if "\n" in val or "\r" in val:
            die(f"derived {key} contains a newline (GITHUB_ENV would corrupt)")

    lines = [f"{k}={v}" for k, v in env.items()]
    if mode == "print":
        for line in lines:
            print(line)
    else:
        with open(out_file, "a", encoding="utf-8") as fh:
            for line in lines:
                fh.write(line + "\n")
        print(f"product-env: appended {len(lines)} variables to {out_file}")


if __name__ == "__main__":
    main()
