#!/usr/bin/env python3
"""twin_parity.py -- offline twin-parity proof for the MSI .wxs twins
(gen-wxs.sh <-> build-msi.ps1), introduced with the w28.1 1603 hotfix
and extended with every twin change (w30.9: the exit-86 stub left the
MSI, the Desktop gained a shortcut component).

THE DISCIPLINE (why this file exists)
    scripts/release/gen-wxs.sh (bash -- the CI path via build-msi.sh)
    and scripts/packaging/build-msi.ps1 (PowerShell -- the
    Windows-native path and the engine behind test-msi-upgrade.ps1)
    generate the SAME installer XML from the same config env. The
    upgrade test builds its PREDECESSOR MSI through the .ps1 twin and
    the release lane builds the CURRENT MSI through the .sh twin, so
    any divergence between them on a shared component (a different
    keypath, GUID payload, or attribute) is a Windows Installer
    component-rule violation -> upgrade exit 1603 (release run
    43729218652: the bash heredoc swallowed the registry-key
    backslashes while the .ps1 twin interpolated correctly). The rule:
    change both together, then run BOTH self-tests AND this proof.

WHAT THIS PROOF DOES (and deliberately does not)
    There is no pwsh in this sandbox and no Windows Installer here, so
    the .ps1 twin cannot be EXECUTED locally. Instead:
      1. the REAL gen-wxs.sh runs (subprocess, POSIX sh) on two
         fixtures -- the hostile self-test fixture (every XML
         metacharacter) and a realistic one (plain values, a
         repo-relative doc path, three docs);
      2. the .ps1 twin's .wxs here-string TEMPLATE is extracted from
         the real build-msi.ps1 source, and its interpolation is
         SIMULATED exactly: the PowerShell expandable here-string
         rules ($var / ${var}; backtick escapes and $(...)
         subexpressions are refused loudly, never silently
         mis-modeled), the XmlEscape five-replacement chain (same
         order as both twins), and the docs-row loop;
      3. both outputs are compared SEMANTICALLY: XML comments
         stripped (the twins' comments differ in punctuation by
         design -- the .ps1 must stay ASCII), canonicalized, and
         diffed. Every element + attribute + text node must match.
    Plus the stable-GUID derivation check: the Docs / AppMenuShortcut
    / DesktopShortcut components carry uuid5-stable GUIDs derived
    from product.toml's frozen msi_upgrade_code
    (uuid5(upgrade_code, "component:<name>")); this proof re-derives
    them and pins the literals in BOTH twins' sources, so a changed
    or diverged GUID can never pass quietly (a changed component GUID
    breaks upgrades exactly like the w28.1 keypath divergence did).

    The simulator is a THIRD implementation of the .ps1's row logic,
    so it is itself pinned: its output must match the REAL bash
    twin's output byte-for-byte (after comment stripping), and the
    bash twin's output is produced by the real script, never by a
    copy. A change to either twin (or to the simulator) breaks
    exactly one side of that equation -- the failure mode is a diff,
    never a false PASS.

Usage: python3 scripts/twin_parity.py        (exit 0 = PASS)
"""

from __future__ import annotations

import difflib
import os
import re
import subprocess
import sys
import tempfile
import tomllib
import uuid
from pathlib import Path
from xml.etree import ElementTree as ET

REPO = Path(__file__).resolve().parent.parent
GEN_WXS = REPO / "scripts" / "release" / "gen-wxs.sh"
BUILD_MSI_PS1 = REPO / "scripts" / "packaging" / "build-msi.ps1"
PRODUCT_TOML = REPO / "product.toml"

# The stable (non-Guid="*") components and their uuid5 name strings --
# the derivation comments in both twins' templates document the same
# scheme (uuid5 over the product upgrade code namespace).
STABLE_COMPONENTS = {
    "Docs": "component:docs",
    "AppMenuShortcut": "component:appmenu-shortcut",
    "DesktopShortcut": "component:shortcut-desktop-lumen",
}

# Fixtures: hostile = the same values both --self-tests use (every XML
# metacharacter, apostrophe included); realistic = plain identity
# values, a repo-relative doc path (exercises the basename split) and
# three docs (exercises the non-KeyPath rows). Test data only -- never
# real identity values.
HOSTILE = {
    "label": "hostile",
    "name": 'A&B "C" <D>',
    "manufacturer": "P&Q",
    "description": "it's <a> & \"b\" > c",
    "version": "1.2.3",
    "upgrade_code": "11111111-2222-3333-4444-555555555555",
    "cli": "selftest-cli",
    "tauri": "selftest-tauri",
    "docs": ['a&b<c>"d".md', "it's.md"],
}
REALISTIC = {
    "label": "realistic",
    "name": "Realistic Player",
    "manufacturer": "Realistic Publisher",
    "description": "A realistic description without metacharacters",
    "version": "10.11.12",
    "upgrade_code": "99999999-8888-7777-6666-555555555555",
    "cli": "realistic-cli",
    "tauri": "realistic-tauri",
    "docs": ["LICENSE", "README.md", "docs/CHANGELOG.md"],
}


class ParityError(Exception):
    pass


# --- the .ps1 twin's logic, mirrored (each piece cross-pinned below) ----------

def xml_escape(text: str) -> str:
    """XmlEscape: the five-replacement chain, order-identical to both
    twins (build-msi.ps1's .Replace chain / gen-wxs.sh's sed chain)."""
    return (
        text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&apos;")
    )


def ps_get_filename(path: str) -> str:
    """[IO.Path]::GetFileName: splits on BOTH separators (the bash twin
    uses POSIX basename, '/'-only -- a backslash in a doc path is a
    legitimate divergence this proof would surface, never hide)."""
    return path.replace("\\", "/").rsplit("/", 1)[-1]


def docs_rows_ps1(docs: list[str]) -> str:
    """The New-MsiWxs docs loop: positional Doc_N_<sanitized> ids, the
    FIRST row the component KeyPath, basenames XML-escaped, rows joined
    with a single newline (the .sh twin appends the newline per row --
    equivalent output, pinned by the canonical diff below)."""
    rows: list[str] = []
    seen: set[str] = set()
    for doc in docs:
        base = ps_get_filename(doc)
        if base in seen:
            raise ParityError(f"duplicate doc basename in fixture: {base!r}")
        seen.add(base)
        ident = "Doc_{}_{}".format(
            len(rows) + 1, re.sub(r"[^0-9A-Za-z_]", "_", base)
        )
        esc_base = xml_escape(base)
        if len(rows) == 0:
            rows.append(
                f'          <File Id="{ident}" Source="{esc_base}" KeyPath="yes" />'
            )
        else:
            rows.append(f'          <File Id="{ident}" Source="{esc_base}" />')
    if not rows:
        raise ParityError("fixture has no docs")
    return "\n".join(rows)


def extract_ps1_template(source: str) -> str:
    """Pull the $wxs = @" ... "@ here-string out of the REAL
    build-msi.ps1 source -- the template text is never duplicated
    here, so template drift in the .ps1 twin reaches this proof."""
    lines = source.splitlines()
    start = None
    for idx, line in enumerate(lines):
        if line.strip() == '$wxs = @"':
            start = idx + 1
            break
    if start is None:
        raise ParityError(
            "cannot find the '$wxs = @\"' here-string opener in build-msi.ps1"
        )
    body: list[str] = []
    for line in lines[start:]:
        if line.startswith('"@'):
            return "\n".join(body) + "\n"
        body.append(line)
    raise ParityError("the $wxs here-string in build-msi.ps1 never terminates")


PS_VAR = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)")


def ps_interpolate(template: str, variables: dict[str, str]) -> str:
    """PowerShell expandable here-string semantics for the subset the
    template actually uses: $var / ${var} expansion; a literal $ only
    before a non-identifier character. Backtick escapes and $(...)
    subexpressions are NOT modeled -- their presence is refused loudly
    (extending the simulator must be a conscious act, never a silent
    mis-interpolation that could fake a PASS)."""

    def repl(m: re.Match[str]) -> str:
        name = m.group(1) or m.group(2)
        if name not in variables:
            raise ParityError(
                f"the .ps1 template references ${name}, which the simulator "
                f"does not compute -- New-MsiWxs gained a variable; extend "
                f"the simulator and both fixtures consciously"
            )
        return variables[name]

    if "`" in template:
        raise ParityError(
            "the .ps1 here-string contains a backtick escape -- not modeled"
        )
    if "$(" in template:
        raise ParityError(
            "the .ps1 here-string contains a $( subexpression -- not modeled"
        )
    return PS_VAR.sub(repl, template)


def simulate_ps1(fixture: dict[str, str], ps1_source: str) -> str:
    """New-MsiWxs, simulated: the template from the real source plus
    the mirrored escape/docs logic. BIN_DESKTOP never reaches this
    simulation (w30.9: the stub left the MSI -- New-MsiWxs takes no
    DesktopExe parameter)."""
    template = extract_ps1_template(ps1_source)
    variables = {
        "escName": xml_escape(fixture["name"]),
        "escManufacturer": xml_escape(fixture["manufacturer"]),
        "escDescription": xml_escape(fixture["description"]),
        "Version": fixture["version"],
        "UpgradeCode": fixture["upgrade_code"],
        "CliExe": fixture["cli"] + ".exe",
        "TauriExe": fixture["tauri"] + ".exe",
        "docsXml": docs_rows_ps1(fixture["docs"]),
    }
    return ps_interpolate(template, variables)


# --- the .sh twin: executed FOR REAL (never simulated) -----------------------

def run_gen_wxs(fixture: dict[str, str], out_path: Path) -> str:
    env = {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "PRODUCT_NAME": fixture["name"],
        "PRODUCT_PUBLISHER": fixture["manufacturer"],
        "PRODUCT_DESCRIPTION": fixture["description"],
        "MSI_VERSION": fixture["version"],
        "PRODUCT_MSI_UPGRADE_CODE": fixture["upgrade_code"],
        "BIN_CLI": fixture["cli"],
        "BIN_TAURI": fixture["tauri"],
        "PACKAGING_DOCS": " ".join(fixture["docs"]),
        # BIN_DESKTOP is deliberately ABSENT (w30.9: the generator
        # stopped consuming it when the exit-86 stub left the MSI).
        # The env is built from scratch, so an inherited BIN_DESKTOP
        # cannot leak in either -- if gen-wxs.sh ever re-requires it,
        # this proof fails loudly right here.
    }
    proc = subprocess.run(
        ["sh", str(GEN_WXS), str(out_path)],
        env=env,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise ParityError(
            f"gen-wxs.sh failed on the {fixture['label']} fixture "
            f"(stdout/stderr follow):\n{proc.stdout}{proc.stderr}"
        )
    return out_path.read_text(encoding="utf-8")


# --- comparison ---------------------------------------------------------------

def strip_xml_comments(text: str) -> str:
    return re.sub(r"<!--.*?-->", "", text, flags=re.DOTALL)


def canonical(text: str) -> str:
    """C14N with comments dropped: every element, attribute and text
    node compared; inter-element whitespace is preserved, so even a
    whitespace-only divergence between the twins fails."""
    return ET.canonicalize(xml_data=text, with_comments=False)


def compare(fixture: dict[str, str], sh_xml: str, ps1_xml: str) -> None:
    if canonical(sh_xml) == canonical(ps1_xml):
        return
    diff = "\n".join(
        difflib.unified_diff(
            strip_xml_comments(sh_xml).splitlines(),
            strip_xml_comments(ps1_xml).splitlines(),
            fromfile=f"gen-wxs.sh output ({fixture['label']})",
            tofile=f"build-msi.ps1 simulation ({fixture['label']})",
            lineterm="",
        )
    )
    raise ParityError(
        f"TWIN DIVERGENCE on the {fixture['label']} fixture -- the twins' "
        f"generated .wxs differ (comments stripped; this is the w28.1 "
        f"failure class: divergent twins = component-rule violation = "
        f"upgrade exit 1603):\n{diff}"
    )


# --- stable-GUID derivation check ---------------------------------------------

def check_guid_derivation(sh_source: str, ps1_source: str) -> dict[str, str]:
    with PRODUCT_TOML.open("rb") as fh:
        product = tomllib.load(fh)
    upgrade_code = product["identity"]["ids"]["windows"]["msi_upgrade_code"]
    namespace = uuid.UUID(upgrade_code)
    derived: dict[str, str] = {}
    for comp, name in STABLE_COMPONENTS.items():
        derived[comp] = str(uuid.uuid5(namespace, name))
    for comp, want in derived.items():
        pattern = re.compile(r'<Component Id="' + comp + r'" Guid="([^"]+)"')
        for twin, source in (("gen-wxs.sh", sh_source), ("build-msi.ps1", ps1_source)):
            found = pattern.search(source)
            if not found:
                raise ParityError(
                    f"{twin}: no stable GUID row for component {comp!r} "
                    f"(expected uuid5({upgrade_code!r}, {name!r}) = {want})"
                )
            if found.group(1) != want:
                raise ParityError(
                    f"{twin}: component {comp} Guid {found.group(1)!r} != the "
                    f"uuid5-derived {want!r} (uuid5 over the product upgrade "
                    f"code namespace, {name!r}) -- a changed or diverged "
                    f"component GUID breaks upgrades (component rule)"
                )
    return derived


# --- entry point ---------------------------------------------------------------

def main() -> None:
    for path in (GEN_WXS, BUILD_MSI_PS1, PRODUCT_TOML):
        if not path.is_file():
            raise ParityError(f"missing file: {path}")
    sh_source = GEN_WXS.read_text(encoding="utf-8")
    ps1_source = BUILD_MSI_PS1.read_text(encoding="utf-8")

    derived = check_guid_derivation(sh_source, ps1_source)

    with tempfile.TemporaryDirectory(prefix="twin-parity-") as tmp:
        for fixture in (HOSTILE, REALISTIC):
            sh_xml = run_gen_wxs(fixture, Path(tmp) / f"{fixture['label']}.wxs")
            ps1_xml = simulate_ps1(fixture, ps1_source)
            compare(fixture, sh_xml, ps1_xml)

    print(
        f"twin_parity.py: PASS -- gen-wxs.sh (executed for real) and the "
        f"build-msi.ps1 here-string simulation produce SEMANTICALLY "
        f"IDENTICAL .wxs XML on both fixtures (hostile + realistic; every "
        f"element, attribute and text node, comments stripped); "
        f"{len(derived)} stable component GUIDs re-derived from "
        f"product.toml's frozen msi_upgrade_code and pinned in both twins: "
        + ", ".join(f"{c}={g}" for c, g in sorted(derived.items()))
    )


if __name__ == "__main__":
    try:
        main()
    except ParityError as exc:
        print(f"twin_parity.py: FAIL\n{exc}", file=sys.stderr)
        raise SystemExit(1)
