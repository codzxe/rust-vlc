#!/usr/bin/env python3
"""W20-A04 citation fraud sweep (§8.1).  v2 — classifier refinements documented in report.

Method (mechanical, per task spec):
  1. Walk audit/files/**/*.audit.md + audit/docs/**/*.audit.md (os.walk, NO git).
  2. Extract citations: `Where: <path.ext>:A(-B)` followed within 900 chars by a ``` block.
     Refinements (v2), each documented in the report:
       R1 multi-cite: a Where statement often lists several path:line refs (and a
          `Code (file:lines):` label before the fence); the quote is verified
          against ANY of those refs (kind=alt-cited-file if not the first).
       R2 "quoted above": if the Where says the quote is above, use the PREVIOUS
          code block instead of the next one.
  3. Sample 10% (deterministic seed 202604).
  4. For each sampled citation where the cited file exists and range<=14 lines:
     read the real range (python equivalent of sed) and classify:
       VERBATIM       — whitespace-normalized equality/subset of the real range
                        (kind: exact | subsequence | multiset-subset | loose-internal-ws)
       ANCHOR-OFFSET  — quote text found elsewhere/at the cited file but not
                        line-verbatim: (a) key line found in file, or (b) >=50% of
                        quote lines present in file, or (c) >=70% of quote tokens
                        present in file (paraphrase/reworded quote). [register policy:
                        "quote text real and present in the cited file" = ANCHOR DEFECT]
       WRONG-FILE     — text not in cited file but found in another same-basename file
       NOT-PARSEABLE  — text not found anywhere checked / empty quote
  5. Rates per wave (Auditor field Wxx-Ayy) and per agent; CSV + summary MD.

Read-only on the repo; writes only under audit/.
"""
import os, re, csv, json, random
from collections import Counter, defaultdict

REPO = "/home/z/my-project/audit-workspace"
AUD = os.path.join(REPO, "audit")
OUT_DIR = os.path.join(AUD, "verify")
SCRATCH = os.path.join(AUD, "scratch", "W20-A04")
CSV_PATH = os.path.join(OUT_DIR, "W20-A04-citations.csv")
JSON_PATH = os.path.join(SCRATCH, "sampled.json")
SEED = 202604
SAMPLE_FRAC = 0.10
MAX_RANGE = 14
NEARBY_CHARS = 900
BASENAME_CAP = 6

REF_RE = re.compile(
    r"([\w./-]+\.(?:rs|ts|tsx|toml|md|sh|json|yml|js|mjs|py|wxs|ps1|lock))"
    r":(\d+)(?:-(\d+))?")
CITE_RE = re.compile(r"Where:\s*" + REF_RE.pattern)
# Where-statement scanner: path:line refs, plus bare "A(-B)" numbers that
# inherit the last path seen (handles "X:12-13 (doc) vs 588-592 (code)").
TOKEN_RE = re.compile(
    r"([\w./-]+\.(?:rs|ts|tsx|toml|md|sh|json|yml|js|mjs|py|wxs|ps1|lock))"
    r":(\d+)(?:-(\d+))?"
    r"|\b(\d{1,4})(?:-(\d{1,4}))?\b")
FENCE = "```"
ELLIPSIS_LINES = {"...", "…"}


def read_text(p):
    try:
        with open(p, "r", encoding="utf-8", errors="replace") as f:
            return f.read()
    except OSError:
        return None


def file_lines(p):
    """Line numbering MUST match sed -n 'A,Bp' semantics: split on \n only
    (Python splitlines() also splits on \x0c/\u2028/etc., drifting numbers)."""
    t = read_text(p)
    return [l for l in t.split("\n")] if t is not None else None


def norm_line(s):
    return s.rstrip("\r\n").strip()


def norm_block(lines):
    out = []
    for l in lines:
        t = norm_line(l)
        if not t or t in ELLIPSIS_LINES:
            continue
        out.append(t)
    return out


def collapse(s):
    return " ".join(s.split())


def tokens(lines):
    toks = []
    for t in lines:
        toks.extend(t.split())
    return toks


def token_overlap(qlines, flines):
    """Fraction of quote tokens present in the file's token multiset."""
    from collections import Counter
    qt, ft = Counter(tokens(qlines)), Counter(tokens(flines))
    if not qt:
        return 0.0
    hit = sum(min(v, ft[k]) for k, v in qt.items())
    return hit / sum(qt.values())


def is_subseq(a, b):
    n, m = len(a), len(b)
    if n == 0 or n > m:
        return False
    return any(b[i:i + n] == a for i in range(m - n + 1))


def is_multiset_subset(a, b):
    ca, cb = Counter(a), Counter(b)
    return all(cb[k] >= v for k, v in ca.items())


def key_line(qlines):
    for t in qlines:
        if len(t) >= 8 and any(ch.isalnum() for ch in t):
            return t
    return max(qlines, key=len) if qlines else None


def build_index():
    index = {}
    for root, dirs, files in os.walk(REPO):
        dirs[:] = [d for d in dirs if d not in (".git", "node_modules", "target", "audit")]
        for f in files:
            index.setdefault(f, []).append(os.path.join(root, f))
    for v in index.values():
        v.sort()
    return index


def src_of_audit_file(audit_rel):
    p = audit_rel
    for pre in ("files/", "docs/"):
        if p.startswith("audit/" + pre):
            p = p[len("audit/" + pre):]
            break
    if p.endswith(".audit.md"):
        p = p[: -len(".audit.md")]
    return p


def resolve_path(cited, audit_src, index):
    """Resolution order for a cited path:
    1. as-is relative to repo root
    2. audit subject itself (basename match, or the subject path ends with the cited path)
    3. suffix match: any repo file whose path ends with the cited path — prefer the
       one sharing the longest directory prefix with the audit subject
    4. sibling of the audit subject (same directory, same basename)
    5. global basename index (unique, else first sorted — flagged 'first'/'ambiguous')
    """
    base = os.path.basename(cited)
    subj = os.path.join(REPO, audit_src) if audit_src else None
    if subj and os.path.isfile(subj):
        if base == os.path.basename(subj):
            return subj, "audit-subject"
    c = os.path.join(REPO, cited.lstrip("./"))
    if os.path.isfile(c):
        return c, "as-is"
    if subj and os.path.isfile(subj) and subj.endswith(cited):
        return subj, "audit-subject-suffix"
    # suffix candidates
    suf = []
    for paths in index.values():
        for p in paths:
            if p.endswith("/" + cited) or p == os.path.join(REPO, cited):
                suf.append(p)
    if len(suf) == 1:
        return suf[0], "suffix-unique"
    if suf and subj:
        d = os.path.dirname(subj)
        best = max(suf, key=lambda p: len(os.path.commonpath([p, d])))
        return best, "suffix-nearest-subject" if len(suf) > 1 else "suffix-unique"
    if suf:
        return sorted(suf)[0], "suffix-first"
    if subj:
        d = os.path.dirname(subj)
        same_dir = [x for x in index.get(base, []) if os.path.dirname(x) == d]
        if same_dir:
            return same_dir[0], "sibling-dir"
    if base in index:
        return index[base][0], "basename-unique" if len(index[base]) == 1 else "basename-first"
    return None, "unresolved"


def fence_positions(text):
    pos, out = 0, []
    while True:
        i = text.find(FENCE, pos)
        if i == -1:
            break
        if i == 0 or text[i - 1] == "\n":
            out.append(i)
        pos = i + 3
    return out


def block_between(text, a, b):
    """Content of the code block whose opening fence is at a; b is next fence."""
    nl = text.find("\n", a)
    if nl == -1 or nl >= b:
        return ""
    return text[nl + 1: b]


def classify_pair(qlines, rng_norm):
    """VERBATIM sub-checks (strict first, then loose)."""
    if qlines == rng_norm:
        return "exact"
    if is_subseq(qlines, rng_norm):
        return "subsequence"
    if is_multiset_subset(qlines, rng_norm):
        return "multiset-subset"
    q_c = [collapse(x) for x in qlines]
    r_c = [collapse(x) for x in rng_norm]
    if q_c == r_c or is_subseq(q_c, r_c):
        return "loose-internal-ws"
    return None


def main():
    index = build_index()
    audit_files = []
    for sub in ("files", "docs"):
        base = os.path.join(AUD, sub)
        for root, dirs, files in os.walk(base):
            for f in files:
                if f.endswith(".audit.md"):
                    audit_files.append(os.path.join(root, f))
    audit_files.sort()

    records = []
    total_where = 0
    for af in audit_files:
        text = read_text(af) or ""
        arel = os.path.relpath(af, REPO)
        auditors = [a.strip() for a in re.findall(r"\|\s*Auditor\s*\|\s*([^|\n]+)", text) if a.strip()]
        auditor = auditors[0] if auditors else "UNKNOWN"
        mo = re.match(r"W(\d+)-([A-Z]{1,3}\d+)", auditor)
        wave = int(mo.group(1)) if mo else None
        agent = mo.group(0) if mo else auditor
        fences = fence_positions(text)
        audit_src = src_of_audit_file(arel)
        for m in CITE_RE.finditer(text):
            total_where += 1
            cited, a, b = m.group(1), int(m.group(2)), m.group(3)
            end = int(b) if b else a
            if end < a:
                a, end = end, a
            # forward fence within 900 chars
            fwd = [f for f in fences if f >= m.end()]
            fi = None
            if fwd and fwd[0] - m.end() <= NEARBY_CHARS:
                fi = fwd[0]
            if fi is None:
                continue  # no code block nearby -> not a checkable citation
            nxt = None
            for f in fences:
                if f > fi:
                    nxt = f
                    break
            if nxt is None:
                nxt = min(len(text), fi + 4000)
            quote = block_between(text, fi, nxt)
            region = text[m.start(): fi]
            quoted_above = re.search(r"quoted (above|earlier|at)", region, re.I)
            if quoted_above:
                prev = [f for f in fences if f < m.start()]
                if len(prev) >= 2:
                    quote = block_between(text, prev[-2], prev[-1])
            # R1 multi-cite candidates: all path:line refs in the Where region;
            # bare "A(-B)" numbers (e.g. ": ... vs 588-592") inherit the last path seen.
            cands = []
            last_path = None
            for rm in TOKEN_RE.finditer(region):
                if rm.group(1):  # path:line(-line)
                    p, x, y = rm.group(1), int(rm.group(2)), rm.group(3)
                    e = int(y) if y else x
                    last_path = p
                else:  # bare number(s) inheriting last_path
                    if not last_path:
                        continue
                    p, x, y = last_path, int(rm.group(4)), rm.group(5)
                    e = int(y) if y else x
                    if x > 9999 or e - x > 60:
                        continue  # not a line range (dates, run-ids, etc.)
                if e < x:
                    x, e = e, x
                cands.append((p, x, e))
            if not cands or cands[0][0] != cited or cands[0][1] != a:
                cands.insert(0, (cited, a, end))
            records.append({
                "audit_file": arel, "auditor": auditor, "wave": wave, "agent": agent,
                "audit_src": audit_src, "pos": m.start(), "cited": cited,
                "start": a, "end": end, "quote": quote, "cands": cands,
                "quoted_above": bool(quoted_above),
            })

    with_block = records  # all have a forward block by construction
    n = len(with_block)
    k = max(1, round(n * SAMPLE_FRAC))
    rng = random.Random(SEED)
    sample_idx = sorted(rng.sample(range(n), k))
    sampled = [dict(with_block[i]) for i in sample_idx]
    print(f"audit files walked: {len(audit_files)}")
    print(f"Where: citations with code block: {n}; sampled {k} (seed {SEED})")

    results = []
    for s in sampled:
        rng_len = s["end"] - s["start"] + 1
        qlines = norm_block((s["quote"] or "").splitlines())
        s.update({"quote_nlines": len(qlines), "classification": None,
                  "excluded_reason": None, "verbatim_kind": "", "anchor_line": "",
                  "wrong_file": "", "detail": "", "cited_resolved": "",
                  "cited_resolved_how": "", "quote_head": qlines[0][:80] if qlines else ""})
        primary, how = resolve_path(s["cited"], s["audit_src"], index)
        s["cited_resolved"], s["cited_resolved_how"] = (os.path.relpath(primary, REPO) if primary else ""), how
        if primary is None:
            s["excluded_reason"] = "MISSING-SOURCE"
            results.append(s)
            continue
        if rng_len > MAX_RANGE:
            s["excluded_reason"] = "RANGE>14"
            results.append(s)
            continue
        if not qlines:
            s["classification"] = "NOT-PARSEABLE"
            s["detail"] = "empty/unusable quote block"
            results.append(s)
            continue

        # candidate files in Where-statement order
        resolved_cands, seen = [], set()
        for (p, x, e) in s["cands"]:
            key = (p, x, e)
            if key in seen:
                continue
            r, h = resolve_path(p, s["audit_src"], index)
            if r:
                seen.add(key)
                resolved_cands.append((p, x, e, r))

        flines = file_lines(primary)
        full_norm = norm_block(flines)
        rng_norm = norm_block(flines[s["start"] - 1: s["end"]])
        if not rng_norm:
            s["classification"] = "NOT-PARSEABLE"
            s["detail"] = "cited range beyond EOF (%d lines)" % len(flines)
            results.append(s)
            continue

        # 1) VERBATIM vs primary range
        kind = classify_pair(qlines, rng_norm)
        if kind:
            s["classification"] = "VERBATIM"
            s["verbatim_kind"] = kind
            results.append(s)
            continue
        # 1b) VERBATIM vs other cited refs (alt file / alt range)
        for (p, x, e, r) in resolved_cands[1:]:
            rl = norm_block((file_lines(r) or [])[x - 1: e])  # slice RAW lines first
            k2 = classify_pair(qlines, rl)
            if k2:
                s["classification"] = "VERBATIM"
                s["verbatim_kind"] = "alt-cited-file:" + k2
                s["detail"] = "quote matches %s:%d-%d listed in same Where" % (
                    os.path.relpath(r, REPO), x, e)
                break
        if s["classification"]:
            results.append(s)
            continue
        # 2) ANCHOR-OFFSET — text present in the cited file, not line-verbatim
        kk = key_line(qlines)
        # anchor search on RAW line numbers (sed semantics), line-normalized compare
        hits = [i for i, raw in enumerate(flines) if norm_line(raw) == kk] if kk else []
        present = sum(1 for t in qlines if t in set(full_norm))
        tok = token_overlap(qlines, full_norm)
        if hits and present * 2 >= len(qlines):
            s["classification"] = "ANCHOR-OFFSET"
            s["anchor_line"] = hits[0] + 1
            s["detail"] = "key line at file line %d; %d/%d quote lines present" % (
                hits[0] + 1, present, len(qlines))
        elif present * 2 >= len(qlines):
            s["classification"] = "ANCHOR-OFFSET"
            s["detail"] = "%d/%d quote lines present verbatim in file (line numbers off)" % (
                present, len(qlines))
        elif tok >= 0.70:
            s["classification"] = "ANCHOR-OFFSET"
            s["detail"] = "reworded/reflowed quote: %.0f%% of tokens present in cited file" % (tok * 100)
        else:
            # 3) WRONG-FILE — same-basename search
            base = os.path.basename(primary)
            found_in = None
            for o in [c for c in index.get(base, []) if c != primary][:BASENAME_CAP]:
                ol = norm_block(file_lines(o) or [])
                ok = token_overlap(qlines, ol)
                if ok >= 0.70:
                    found_in = o
                    break
            if found_in:
                s["classification"] = "WRONG-FILE"
                s["wrong_file"] = os.path.relpath(found_in, REPO)
                s["detail"] = "text found in same-basename file (misattribution)"
            else:
                s["classification"] = "NOT-PARSEABLE"
                s["detail"] = ("quote text not found in cited file nor same-basename "
                               "files (token overlap %.0f%%)" % (tok * 100))
        results.append(s)

    os.makedirs(OUT_DIR, exist_ok=True)
    cols = ["audit_file", "auditor", "wave", "agent", "cited", "cited_resolved",
            "cited_resolved_how", "start", "end", "range_len", "quote_nlines",
            "classification", "excluded_reason", "verbatim_kind", "anchor_line",
            "wrong_file", "detail", "quote_head", "quoted_above"]
    with open(CSV_PATH, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=cols, extrasaction="ignore")
        w.writeheader()
        for s in results:
            row = dict(s)
            row["range_len"] = s["end"] - s["start"] + 1
            w.writerow(row)
    with open(JSON_PATH, "w", encoding="utf-8") as f:
        json.dump([{k: s.get(k) for k in cols} for s in results], f, indent=1)

    checked = [s for s in results if s["classification"]]
    excl = Counter(s["excluded_reason"] for s in results if s["excluded_reason"])
    cc = Counter(s["classification"] for s in checked)
    print("checked:", len(checked), dict(cc), "excluded:", dict(excl))
    kinds = Counter(s["verbatim_kind"].split(":")[0] for s in checked
                    if s["classification"] == "VERBATIM")
    print("verbatim kinds:", dict(kinds))
    by_wave = defaultdict(Counter)
    for s in checked:
        by_wave[s["wave"] if s["wave"] is not None else -1][s["classification"]] += 1
    for w in sorted(by_wave):
        print("W%02d" % w if w >= 0 else "? ", dict(by_wave[w]))
    by_agent = defaultdict(Counter)
    for s in checked:
        by_agent[s["agent"]][s["classification"]] += 1
    worst = sorted(by_agent.items(),
                   key=lambda kv: -(kv[1]["ANCHOR-OFFSET"] + kv[1]["WRONG-FILE"]
                                    + kv[1]["NOT-PARSEABLE"]))[:10]
    print("worst agents:")
    for a, c in worst:
        tot = sum(c.values())
        print(f"  {a}: n={tot} nonverb={tot - c['VERBATIM']}", dict(c))
    np_ = [s for s in checked if s["classification"] == "NOT-PARSEABLE"]
    wf_ = [s for s in checked if s["classification"] == "WRONG-FILE"]
    print("\nNOT-PARSEABLE rows:")
    for s in np_:
        print("  ", s["audit_file"], "|", s["auditor"], "|", s["cited"] + ":" +
              str(s["start"]) + "-" + str(s["end"]), "|", s["detail"])
    print("\nWRONG-FILE rows:")
    for s in wf_:
        print("  ", s["audit_file"], "|", s["auditor"], "|", s["cited"], "->",
              s["wrong_file"])


if __name__ == "__main__":
    main()
