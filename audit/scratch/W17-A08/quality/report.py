#!/usr/bin/env python3
"""report.py - aggregate docs/audit/reports/ into BASELINE.md and FINDINGS counts.

Rule: every number written here is derived from a file that exists in
docs/audit/reports/. Missing evidence is reported as NOT RUN, never guessed.
"""
import json
import os
import re
import subprocess
import sys
from collections import Counter
from datetime import datetime, timezone

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "docs", "audit", "reports")
AUD = os.path.join(ROOT, "docs", "audit")


def read(name):
    p = os.path.join(OUT, name)
    if not os.path.exists(p):
        return None
    return open(p, encoding="utf-8-sig", errors="replace").read()


def count_lines(path, pattern):
    txt = read(path)
    if txt is None:
        return None
    return len(re.findall(pattern, txt, re.M))


def astgrep_counts():
    d = os.path.join(OUT, "astgrep")
    rows = {}
    if not os.path.isdir(d):
        return rows
    for fn in sorted(os.listdir(d)):
        if not fn.endswith(".json"):
            continue
        rid = fn[:-5]
        try:
            hits = json.load(open(os.path.join(d, fn), encoding="utf-8"))
            rows[rid] = len(hits) if isinstance(hits, list) else 0
        except Exception:
            rows[rid] = None
    return rows


def cargo_metadata():
    try:
        raw = subprocess.check_output(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"],
            cwd=ROOT, text=True)
        return json.loads(raw)
    except Exception:
        return None


def crate_loc(meta):
    rows = []
    if not meta:
        return rows
    for p in meta["packages"]:
        base = os.path.dirname(p["manifest_path"])
        loc = tests = files = 0
        for dirpath, dirnames, fns in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in ("target", "node_modules", ".git")]
            for fn in fns:
                if not fn.endswith(".rs"):
                    continue
                try:
                    src = open(os.path.join(dirpath, fn), encoding="utf-8", errors="replace").read()
                except Exception:
                    continue
                loc += src.count("\n") + 1
                files += 1
                tests += len(re.findall(r"#\[(?:tokio::)?test\]", src))
        rel = os.path.relpath(base, ROOT).replace("\\", "/")
        rows.append((rel, p["name"], loc, tests, files))
    return rows


def main():
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    meta = cargo_metadata()
    ag = astgrep_counts()
    notrun = read("not-run.txt") or ""

    L = []
    L.append("# BASELINE - measured state of the Lumen Rust workspace")
    L.append("")
    L.append("Generated: %s by `scripts/quality/report.py`." % now)
    L.append("Every number below traces to a file in `reports/`. `n/a` means the tool")
    L.append("did not run - never that it found zero.")
    L.append("")

    if meta:
        pkgs = meta["packages"]
        L.append("## Workspace")
        L.append("")
        L.append("| metric | value |")
        L.append("|---|---|")
        L.append("| workspace members | %d |" % len(pkgs))
        nodes = (meta.get("resolve") or {}).get("nodes", [])
        L.append("| resolved package nodes | %s |" % (len(nodes) if nodes else "n/a (--no-deps metadata)"))
        L.append("| targets | %d |" % sum(len(p.get("targets", [])) for p in pkgs))
        L.append("| binaries | %d |" % len([t for p in pkgs for t in p.get("targets", []) if "bin" in t.get("kind", [])]))
        L.append("")

    rows = crate_loc(meta)
    if rows:
        total_loc = sum(r[2] for r in rows)
        total_tests = sum(r[3] for r in rows)
        L.append("## Size and test density (by workspace member)")
        L.append("")
        L.append("Total: **%s lines**, **%s `#[test]` items** across %d members." % (
            f"{total_loc:,}", f"{total_tests:,}", len(rows)))
        L.append("")
        L.append("| path | crate | LOC | rs files | tests | lines/test |")
        L.append("|---|---|---|---|---|---|")
        for path, name, loc, tests, files in sorted(rows, key=lambda r: -r[2]):
            ratio = ("%.0f" % (loc / tests)) if tests else "**no tests**"
            L.append("| %s | %s | %s | %d | %d | %s |" % (path, name, f"{loc:,}", files, tests, ratio))
        L.append("")

    L.append("## Static analysis")
    L.append("")
    L.append("| check | result | evidence |")
    L.append("|---|---|---|")
    clippy_warn = count_lines("clippy-L0.log", r"^warning: ")
    clippy_err = count_lines("clippy-L0.log", r"^error")
    L.append("| clippy L0 warnings | %s | `reports/clippy-L0.log` |" % (clippy_warn if clippy_warn is not None else "n/a"))
    L.append("| clippy L0 errors | %s | `reports/clippy-L0.log` |" % (clippy_err if clippy_err is not None else "n/a"))
    if ag:
        L.append("")
        L.append("### ast-grep rules (total / shipped code / tests)")
        L.append("")
        L.append("| rule | total | shipped | tests |")
        L.append("|---|---|---|---|")
        summ = None
        try:
            summ = json.loads(read("astgrep-summary.json") or "null")
        except Exception:
            summ = None
        if summ and "counts" in summ:
            for k, v in sorted(summ["counts"].items(), key=lambda kv: -kv[1]["production"]):
                L.append("| %s | %d | **%d** | %d |" % (k, v["total"], v["production"], v["test"]))
        else:
            for k, v in sorted(ag.items(), key=lambda kv: -(kv[1] or 0)):
                L.append("| %s | %s | n/a | n/a |" % (k, v if v is not None else "n/a"))
    L.append("")

    # test quality
    try:
        th = json.loads(read("test-hygiene.json") or "null")
    except Exception:
        th = None
    if th and "counts" in th:
        c = th["counts"]
        L.append("## Test quality (structural census)")
        L.append("")
        L.append("| metric | value |")
        L.append("|---|---|")
        L.append("| `#[test]` functions parsed | %d |" % c.get("tests", 0))
        L.append("| assertion-free (candidate fake) | %d |" % c.get("assertion_free", 0))
        L.append("| tests containing a sleep | %d |" % c.get("sleeps", 0))
        L.append("| ignored tests | %d |" % c.get("ignore", 0))
        L.append("| ignored without a reason | %d |" % c.get("ignore_no_reason", 0))
        L.append("| should_panic without expected= | %d |" % c.get("should_panic_no_expected", 0))
        L.append("")

    # mutation scores
    mut_rows = []
    mroot = os.path.join(OUT, "mutants")
    if os.path.isdir(mroot):
        for crate in sorted(os.listdir(mroot)):
            p = os.path.join(mroot, crate, "mutants.out", "outcomes.json")
            if not os.path.exists(p):
                continue
            try:
                d = json.load(open(p, encoding="utf-8"))
            except Exception:
                continue
            caught = d.get("caught", 0)
            missed = d.get("missed", 0)
            timeout = d.get("timeout", 0)
            unviable = d.get("unviable", 0)
            denom = caught + missed + timeout
            score = (100.0 * caught / denom) if denom else 0.0
            mut_rows.append((crate, d.get("total_mutants", 0), caught, missed, unviable, score))
    if mut_rows:
        L.append("## Mutation testing (cargo-mutants)")
        L.append("")
        L.append("`score = caught / (caught + missed + timeout)`; unviable mutants")
        L.append("(do not compile) are excluded. >=80% trustworthy, 60-79% spotty,")
        L.append("<60% decorative.")
        L.append("")
        L.append("| crate | mutants | caught | **missed** | unviable | score |")
        L.append("|---|---|---|---|---|---|")
        for crate, total, caught, missed, unviable, score in mut_rows:
            L.append("| %s | %d | %d | **%d** | %d | **%.1f%%** |" % (
                crate, total, caught, missed, unviable, score))
        L.append("")

    # duplication
    try:
        jr = json.loads(read("jscpd/jscpd-report.json") or "null")
    except Exception:
        jr = None
    if jr:
        t = jr.get("statistics", {}).get("total", {})
        L.append("## Duplication (jscpd)")
        L.append("")
        L.append("| metric | value |")
        L.append("|---|---|")
        L.append("| files scanned | %s |" % t.get("sources"))
        L.append("| lines scanned | %s |" % t.get("lines"))
        L.append("| clone pairs | %s |" % t.get("clones"))
        L.append("| duplicated lines | %s |" % t.get("duplicatedLines"))
        L.append("| **duplication percent** | **%s** |" % t.get("percentage"))
        L.append("")

    for label, path, pat in (
        ("typos", "typos.txt", r"^"),
        ("duplication clones", "jscpd/jscpd-report.json", r'"clones"'),
        ("unused deps (machete)", "machete.txt", r"^"),
        ("orphan modules", "orphans.txt", r"^"),
        ("duplicate crate versions", "duplicates.txt", r"^[a-zA-Z]"),
        ("#[allow] suppressions", "astgrep/allow-attr.json", r"\"ruleId\""),
    ):
        c = count_lines(path, pat)
        if c is not None:
            L.append("- **%s**: %d (evidence: `reports/%s`)" % (label, c, path))
    L.append("")

    ti = read("test-infra.txt")
    if ti:
        L.append("## Test infrastructure")
        L.append("")
        L.append("```")
        L.append(ti.strip())
        L.append("```")
        L.append("")

    th = read("test-hygiene.txt")
    if th:
        L.append("## Test hygiene")
        L.append("")
        L.append("```")
        L.append(th.strip()[:4000])
        L.append("```")
        L.append("")

    L.append("## NOT RUN")
    L.append("")
    if notrun.strip():
        L.append("```")
        L.append(notrun.strip())
        L.append("```")
    else:
        L.append("(nothing recorded)")
    L.append("")

    open(os.path.join(AUD, "BASELINE.md"), "w", encoding="utf-8").write("\n".join(L) + "\n")

    # ---------------- FINDINGS summary ----------------
    sev = Counter()
    lines = []
    lines.append("## Auto-generated counts (%s)" % now)
    lines.append("")
    lines.append("| category | count | source |")
    lines.append("|---|---|---|")
    if ag:
        for k, v in sorted(ag.items(), key=lambda kv: -(kv[1] or 0)):
            lines.append("| ast-grep: %s | %s | `reports/astgrep/%s.json` |" % (k, v, k))
    th_json = read("test-hygiene.json")
    if th_json:
        try:
            tj = json.loads(th_json)
            for k in ("assertion_free_suspect", "ignore_without_reason", "should_panic_without_expected"):
                if k in tj:
                    lines.append("| tests: %s | %d | `reports/test-hygiene.json` |" % (k, len(tj[k])))
        except Exception:
            pass
    lines.append("")

    p = os.path.join(AUD, "FINDINGS.md")
    if not os.path.exists(p):
        open(p, "w", encoding="utf-8").write("# FINDINGS\n\n")
    body = open(p, encoding="utf-8").read()
    marker = "<!-- AUTO-COUNTS -->"
    if marker in body:
        body = body.split(marker)[0] + marker + "\n" + "\n".join(lines) + "\n"
    else:
        body = body + "\n" + marker + "\n" + "\n".join(lines) + "\n"
    open(p, "w", encoding="utf-8").write(body)

    print("wrote %s" % os.path.join(AUD, "BASELINE.md"))
    print("updated %s (auto counts)" % p)


if __name__ == "__main__":
    main()
