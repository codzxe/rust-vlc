#!/usr/bin/env python3
"""0.B1 parser: VLC 3.0.23 help dumps -> docs/inventory/options-linux.json.
Run from anywhere: python3 docs/inventory/raw/options/parse-options.py

Input (all captured from the pinned sandbox binary, docs/baselines/raw/linux/):
  12-options-longhelp.txt          (basic options, --longhelp)
  13-options-advanced-longhelp.txt (--advanced --longhelp)
  14-options-full-help.txt         (--full-help / -H, superset incl. advanced)
  16-options-vlcrc.txt             (preferences file walk, --reset-config)

Clean-room: parses observed runtime help output only. No VLC source consulted.
"""
import json
import os
import re
from collections import Counter, OrderedDict

RAW = "docs/baselines/raw/linux"

MODULE_RE = re.compile(r"^ (\S.*?) \(([a-zA-Z0-9_.-]+)\)\s*$")
SUBSECTION_RE = re.compile(r"^   (\S.*):$")
CATEGORIES = {  # one-space headers observed inside module/core sections
    "Audio", "Video", "Subpictures", "Decoders", "Input", "Stream output",
    "Miscellaneous", "Playlist", "Hot keys", "Hacks",
}
FLAGS_RE = re.compile(
    r"^(--(?P<name>[a-z0-9_-]+))"                        # primary long flag
    r"(?:,\s*(?P<neg>--no-(?P<negname>[a-z0-9_-]+)))?"   # optional --no- form
    r"(?:\s+(?P<arg>\{.*\}|<[^>]*>))?"                   # optional {choices} or <type>
    r"\s*$"
)


def parse_dump(path):
    """Parse one longhelp/full-help dump. Returns (options, sections, leftovers, lines)."""
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    options = []
    sections = []
    leftovers = []
    cur = None
    module_id = module_name = category = subsection = None
    module_desc = []
    cat_desc = []
    in_module_desc = in_cat_desc = False
    started = False

    def close():
        nonlocal cur
        if cur is not None:
            cur["help_text"] = " ".join(x for x in cur["_help"] if x).strip()
            cur["long_description"] = " ".join(cur["_long"]).strip()
            del cur["_help"], cur["_long"]
            options.append(cur)
            cur = None

    prev_blank = True
    for lineno, line in enumerate(lines, 1):
        m = MODULE_RE.match(line)
        if m and prev_blank:
            close()
            started = True
            module_name, module_id = m.group(1), m.group(2)
            category = subsection = None
            module_desc, cat_desc = [], []
            in_module_desc, in_cat_desc = True, False
            sections.append((module_id, module_name, category, subsection))
            prev_blank = False
            continue
        if not started:
            prev_blank = (line.strip() == "")
            continue
        stripped = line.strip()
        if line == "":  # true separator (empty line)
            close()
            prev_blank = True
            continue
        if line.strip() == "":
            # whitespace-only line: a column-33 help slot with empty text
            if cur is not None:
                cur["_help"].append("")
                continue
            prev_blank = False
            continue
        prev_blank = False

        if line.startswith("# exit="):
            break  # our own dump footer marker, not part of the verbatim output

        # option start: 6-space --flag, or 2-space "-x, --flag"
        if (line.startswith("      --") or
                re.match(r"^  -[a-zA-Z], --", line)):
            close()
            in_module_desc = in_cat_desc = False
            short = None
            if line.startswith("      --"):
                body = line[6:]
            else:
                body = line[2:]
                sm = re.match(r"^-([a-zA-Z]), (.*)$", body)
                short = sm.group(1)
                body = sm.group(2)
            # split "flags [arg] [inline help]" robustly
            bm = re.match(r"^(--[a-zA-Z0-9_+-]+(?:,\s*--no-[a-zA-Z0-9_+-]+)?)", body)
            if not bm:
                leftovers.append({"line": lineno, "text": line})
                continue
            flags_core = bm.group(1)
            rest = body[bm.end():].strip()
            arg = None
            inline_help = ""
            if rest.startswith("{"):
                # choices token: up to the closing brace
                end = rest.find("}")
                if end == -1:
                    leftovers.append({"line": lineno, "text": line})
                    continue
                arg, rest2 = rest[:end + 1], rest[end + 1:].strip()
                inline_help = rest2
            elif rest.startswith("<"):
                end = rest.find(">")
                if end == -1:
                    leftovers.append({"line": lineno, "text": line})
                    continue
                arg, rest2 = rest[:end + 1], rest[end + 1:].strip()
                inline_help = rest2
            else:
                inline_help = rest
            fm = re.match(r"^--([a-zA-Z0-9_+-]+)$", flags_core.split(",")[0])
            neg = None
            nm = re.search(r",\s*(--no-[a-zA-Z0-9_+-]+)$", flags_core)
            if nm:
                neg = nm.group(1)
            if not bm or not fm:
                leftovers.append({"line": lineno, "text": line})
                continue
            name = fm.group(1)
            typ = choices = None
            vmin = vmax = None
            if arg:
                if arg.startswith("{"):
                    typ = "enum"
                    choices = [c.strip() for c in arg[1:-1].split(", ")]
                else:
                    inner = arg[1:-1].strip()
                    rm = re.match(
                        r"^(integer|float|string|longinteger|boolean)"
                        r"(?:\s*\[\s*([^\s]+)\s*\.\.\s*([^\s\]]+)\s*\])?$", inner)
                    if rm:
                        typ = rm.group(1)
                        vmin, vmax = rm.group(2), rm.group(3)
                    else:
                        typ = inner
            else:
                typ = "bool"
            cur = OrderedDict()
            cur["option_name"] = name
            cur["short_form"] = short
            cur["negation_form"] = neg
            cur["type"] = typ
            if choices is not None:
                cur["choices"] = choices
            if vmin is not None:
                cur["min"] = vmin
                cur["max"] = vmax
            cur["help_text"] = None
            cur["default"] = None
            cur["long_description"] = None
            cur["module"] = module_id
            cur["module_name"] = module_name
            cur["category"] = category
            cur["subsection"] = subsection
            cur["_help"] = [inline_help]
            cur["_long"] = []
            continue

        if cur is not None:
            if line.startswith(" " * 32):  # help column (col 33)
                txt = line[32:].strip()
                dm = re.match(r"^\(default (.*)\)$", txt)
                if dm and cur["default"] is None:
                    cur["default"] = dm.group(1)
                else:
                    cur["_help"].append(txt)
                continue
            if line.startswith(" " * 10):  # long description (col 11)
                cur["_long"].append(line[10:].strip())
                continue
            close()

        # not inside an option: headers / descriptions
        if line.startswith("   ") and not line.startswith("    ") \
                and not re.match(r"^   [-:]", line):
            # subsection header (usually ends with ':'; v4l2 has one ending in ')')
            subsection = line.strip().rstrip(":")
            in_module_desc = in_cat_desc = False
            continue
        if (line.startswith(" ") and not line.startswith("  ")
                and stripped.startswith("[")):
            # MRL syntax illustration line inside a module section (e.g. cdda)
            module_desc.append(stripped)
            continue
        if (line.startswith(" ") and not line.startswith("  ")
                and "(" not in line and stripped in CATEGORIES):
            category = stripped
            subsection = None
            in_module_desc, in_cat_desc = False, True
            cat_desc = []
            continue
        if line.startswith(" ") and not line.startswith("  "):
            if in_module_desc:
                module_desc.append(stripped)
            elif in_cat_desc:
                cat_desc.append(stripped)
            else:
                leftovers.append({"line": lineno, "text": line})
            continue
        if in_module_desc or in_cat_desc:
            # flush-left continuation of a module/category description
            (module_desc if in_module_desc else cat_desc).append(stripped)
            continue
        leftovers.append({"line": lineno, "text": line})
    close()
    return options, sections, leftovers, lines


def parse_vlcrc(path):
    """Parse dump 16 -> ({name: rec}, total_name_value_lines, duplicate_names)."""
    with open(path, encoding="utf-8") as f:
        text = f.read()
    marker = "# ---- verbatim vlcrc starts here (BOM stripped) ----"
    body = text.split(marker, 1)[1]
    body = body.split("# ---- verbatim vlcrc ends here ----", 1)[0]
    recs = OrderedDict()
    seen = []
    module = None
    pending = None
    for line in body.splitlines():
        s = line.strip()
        if s.startswith("[") and "]" in s:
            module = s[1:s.index("]")].strip()
            continue
        if s.startswith("#") and "=" not in s and s.endswith(")"):
            m = re.match(
                r"^#\s*(.*?)\s*\((integer|float|string|boolean|longinteger"
                r"|integer list|float list|string list|module list)\)$", s)
            if m:
                pending = (m.group(1), m.group(2))
                continue
        m = re.match(r"^#?([a-zA-Z0-9_+-]+)=(.*)$", s)
        if m:
            d, t = pending or (None, None)
            recs[m.group(1)] = {
                "module": module,
                "default_value": m.group(2),
                "vlcrc_help": d,
                "vlcrc_type": t,
            }
            seen.append(m.group(1))
            pending = None
    from collections import Counter as _C
    dups = {k: v for k, v in _C(seen).items() if v > 1}
    return recs, len(seen), dups


def main():
    import pathlib
    repo = pathlib.Path(__file__).resolve().parents[4]
    os.chdir(repo)

    d12, s12, l12, raw12 = parse_dump(f"{RAW}/12-options-longhelp.txt")
    d13, s13, l13, raw13 = parse_dump(f"{RAW}/13-options-advanced-longhelp.txt")
    d14, s14, l14, raw14 = parse_dump(f"{RAW}/14-options-full-help.txt")
    vlcrc, rc_total, rc_dups = parse_vlcrc(f"{RAW}/16-options-vlcrc.txt")

    names12 = {o["option_name"] for o in d12}
    names13 = {o["option_name"] for o in d13}
    names14 = {o["option_name"] for o in d14}
    assert names12 <= names13 <= names14, "option sets not nested"
    mods12 = len(s12)
    mods13 = len(s13)
    mods14 = len(s14)
    uids12 = len({s[0] for s in s12})
    uids13 = len({s[0] for s in s13})
    uids14 = len({s[0] for s in s14})

    out_options = []
    name_collisions = {}
    for o in d14:
        r = dict(o)
        vis = ("longhelp" if o["option_name"] in names12 else
               ("advanced-only" if o["option_name"] in names13 else "full-help-only"))
        r["visibility"] = vis
        r["is_advanced"] = vis != "longhelp"
        in_rc = o["option_name"] in vlcrc
        r["in_vlcrc"] = in_rc
        if in_rc:
            r["vlcrc_default"] = vlcrc[o["option_name"]]["default_value"]
        out_options.append(r)
        name_collisions.setdefault(o["option_name"], []).append(o["module"])

    collisions = {n: m for n, m in name_collisions.items() if len(m) > 1}
    rc_only = sorted(set(vlcrc) - names14)
    fh_only = sorted(names14 - set(vlcrc))

    core_cats = []
    for o in out_options:
        if o["module"] == "core" and o["category"] and o["category"] not in core_cats:
            core_cats.append(o["category"])

    doc = OrderedDict()
    doc["schema"] = "lumen-phase0/options-inventory/1.0"
    doc["source_baseline"] = ("VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd), "
                              "Debian 3.0.23-0+deb13u1 extracted install")
    doc["platform"] = "linux"
    doc["captured"] = "2026-10-19"
    doc["capture_environment"] = (
        "Debian 13 (trixie) container, x86_64, headless; "
        "tmp/pinning/extract/usr/bin/vlc -I dummy; see docs/baselines/raw/linux/12..16")
    doc["clean_room_statement"] = (
        "All data parsed from runtime help output of the pinned binary (observed behavior). "
        "No VLC source code consulted. No binaries committed.")
    doc["dump_files"] = {
        "longhelp": "docs/baselines/raw/linux/12-options-longhelp.txt",
        "advanced_longhelp": "docs/baselines/raw/linux/13-options-advanced-longhelp.txt",
        "full_help": "docs/baselines/raw/linux/14-options-full-help.txt",
        "flag_probes": "docs/baselines/raw/linux/15-options-help-and-probes.txt",
        "vlcrc_walk": "docs/baselines/raw/linux/16-options-vlcrc.txt",
    }
    doc["flag_probes"] = {
        "--help": "works; 407-line basic dump (raw 15)",
        "--longhelp": "works; basic options tree (raw 12)",
        "--advanced --longhelp": "works; adds advanced options (raw 13)",
        "--full-help (-H)": ("works; superset incl. advanced and extra core categories "
                             "(raw 14); byte-identical to --advanced --full-help (diff = 0 lines)"),
        "--help-verbose": ("NOT a dump flag on 3.0.23: accepted, no stdout; without a stream "
                           "it runs until killed (timeout exit 124); with vlc://quit exits 0 silently"),
        "footer_note": ("--full-help still hides 4 modules that only have advanced options "
                        "(footer verbatim in raw 15)"),
    }
    doc["scope_notes"] = {
        "plugin_set": ("vlc-plugin-base + vlc-plugin-video-output "
                       "(Debian 3.0.23-0+deb13u1), 502 modules per dump 05"),
        "excluded_packages": ["vlc-plugin-qt", "vlc-plugin-skins2", "vlc-plugin-notify",
                              "vlc-plugin-samba", "vlc-plugin-jack", "vlc-plugin-svg",
                              "vlc-plugin-fluidsynth", "vlc-plugin-visualization",
                              "vlc-plugin-video-splitter", "vlc-plugin-access-extra"],
        "consequence": ("options belonging to excluded plugin packages (e.g. the large qt "
                        "module family --qt-*, skins2, jack --jack*, fluidsynth, svg, notify) "
                        "are NOT in this inventory; the full desktop-stack census runbook is "
                        "docs/baselines/hardware-template.md §5 [VERIFY-blocked-on-hardware-desk]"),
        "vlcrc_consequence": "same limitation applies to dump 16 (written by the same plugin set)",
    }
    doc["structure_notes"] = {
        "dump_layout": ("preamble; per-module sections ' Module name (module_id)'; "
                        "' core program (core)' section LAST, organized into categories"),
        "core_categories_observed": core_cats,
        "categories_elsewhere": "Hacks (inside mp4 module in dumps 13/14)",
        "subpictures_scope_warning": (
            "the core 'Subpictures' category header's scope runs until the next category "
            "header ('Input'), so its later subsections (Track settings, Playback control, "
            "Default devices, Network settings, Socks proxy, Metadata, Advanced) hold options "
            "the Qt UI groups under Input/Codecs. The 'category' field is verbatim-faithful "
            "to the dump layout; use 'subsection' for finer grouping when mapping."),
        "card_assumption_delta": (
            "doc 13 §3 card expected headers 'Main/Subtitles/Interfaces'; 3.0.23 actually "
            "emits 'core program (core)' (the 'Main' equivalent), 'Subpictures' (not "
            "'Subtitles'), and no 'Interfaces' header (interface module options live in "
            "their own module sections, e.g. rc, lua, hotkeys)"),
    }
    doc["totals"] = {
        "options_parsed_full_help": len(d14),
        "options_parsed_advanced_longhelp": len(d13),
        "options_parsed_longhelp": len(d12),
        "vlcrc_unique_names": len(vlcrc),
        "vlcrc_total_name_value_lines": rc_total,
        "vlcrc_duplicate_names": rc_dups,
        "module_sections_in_longhelp": mods12,
        "module_sections_in_advanced_longhelp": mods13,
        "module_sections_in_full_help": mods14,
        "unique_module_ids_longhelp": uids12,
        "unique_module_ids_advanced_longhelp": uids13,
        "unique_module_ids_full_help": uids14,
        "unique_option_names_full_help": len(names14),
        "option_name_collisions": len(collisions),
    }
    doc["parse_coverage"] = {
        "raw_12_lines": len(raw12), "leftover_12": len(l12),
        "raw_13_lines": len(raw13), "leftover_13": len(l13),
        "raw_14_lines": len(raw14), "leftover_14": len(l14),
        "raw_16_option_lines": len(vlcrc),
        "note": ("leftovers = non-blank in-dump lines not consumed as module header, category, "
                 "subsection, option, help, default, or long-description; quoted verbatim below"),
    }
    doc["reconciliation"] = {
        "advanced_only_names_count": len(names13 - names12),
        "full_help_only_names_count": len(names14 - names13),
        "in_vlcrc_not_full_help_count": len(rc_only),
        "in_full_help_not_vlcrc_count": len(fh_only),
        "in_vlcrc_not_full_help_sample": rc_only[:60],
        "in_full_help_not_vlcrc_full_list": fh_only,
        "in_full_help_not_vlcrc_annotation": (
            "observed categories: (a) CLI/meta-only flags not persisted to vlcrc "
            "(help, longhelp, full-help, help-verbose, version, list, list-verbose, module, "
            "config, quiet, verbose, ignore-config, reset-config, reset-plugins-cache, "
            "started-from-file, color); (b) pointer/callback/handle options not persistable "
            "(imem-*, sout-smem-*, drawable-xid, shm-*); (c) remainder listed for [VERIFY] "
            "review (e.g. cdda-track, http-referrer, sub-text-scale, lua-sd)"),
        "name_collisions": collisions,
        "name_collisions_annotation": (
            "6 flat-namespace collisions; the SAME 6 names also appear on multiple lines in "
            "the vlcrc walk (cross-extraction agreement between --full-help and vlcrc)"),
    }
    # default-value agreement between help '(default X)' markers and vlcrc values
    agree = disagree = help_only_default = vlcrc_only_default = 0
    disagreements = []
    for o in out_options:
        hd = o.get("default")
        rd = o.get("vlcrc_default")
        if hd is None and rd is None:
            continue
        if hd is not None and rd is None:
            help_only_default += 1
            continue
        if hd is None and rd is not None:
            vlcrc_only_default += 1
            continue
        norm_h = {"enabled": "1", "disabled": "0"}.get(hd, hd)
        norm_r = {"true": "1", "false": "0"}.get(rd, rd)
        try:
            match = (abs(float(norm_h) - float(norm_r)) < 1e-9)
        except ValueError:
            match = norm_h == norm_r
        if match:
            agree += 1
        else:
            disagree += 1
            if len(disagreements) < 40:
                disagreements.append({"name": o["option_name"], "help": hd, "vlcrc": rd})
    doc["reconciliation"]["default_agreement"] = {
        "both_declared": agree + disagree,
        "agree": agree,
        "disagree": disagree,
        "help_default_but_absent_from_vlcrc": help_only_default,
        "vlcrc_default_but_no_help_marker": vlcrc_only_default,
        "disagreements_sample": disagreements,
    }
    doc["leftovers"] = {
        "annotation": ("all remaining leftovers are dump FOOTER notes ('Note: N modules were "
                        "not displayed…', 'To get exhaustive help, use -H', 'Note: add "
                        "--advanced…') — no option-bearing line was left unparseable"),
        "longhelp": l12[:25],
        "advanced_longhelp": l13[:25],
        "full_help": l14[:40],
    }
    doc["options"] = out_options

    with open("docs/inventory/options-linux.json", "w", encoding="utf-8") as f:
        json.dump(doc, f, indent=1, ensure_ascii=False)
        f.write("\n")

    print("d12:", len(d12), "d13:", len(d13), "d14:", len(d14))
    print("module sections:", mods12, mods13, mods14, "| unique ids:", uids12, uids13, uids14)
    print("leftovers:", len(l12), len(l13), len(l14))
    print("advanced-only names (13 not 12):", len(names13 - names12))
    print("full-help-only names (14 not 13):", len(names14 - names13))
    print("vlcrc records:", len(vlcrc))
    print("rc_only:", len(rc_only), "fh_only:", len(fh_only))
    print("collisions:", len(collisions))
    for k, v in list(collisions.items())[:12]:
        print("  ", k, v)
    print("core categories:", core_cats)
    tc = Counter(o["type"] for o in out_options)
    print("types:", dict(tc))
    vc = Counter(o["visibility"] for o in out_options)
    print("visibility:", dict(vc))
    cc = Counter(o["category"] for o in out_options if o["module"] == "core")
    print("core per-category:", dict(cc))
    print("with (default ...) marker:", sum(1 for o in out_options if o.get("default")))
    print("leftover samples 14:", l14[:8])


if __name__ == "__main__":
    main()
