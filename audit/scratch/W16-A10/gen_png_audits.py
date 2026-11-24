#!/usr/bin/env python3
"""W16-A10: generate the 18 ui/e2e/visual/baselines/linux/*.png audits from batchmeta."""
import re, pathlib

ROOT = pathlib.Path("/home/z/my-project/audit-workspace")
META = (ROOT/"audit/scratch/W16-A10/batchmeta.txt").read_text()

PNGS = [
 "player-music-1920x1080-1x-dark.png","settings-playback-1920x1080-1x-light.png",
 "settings-privacy-1280x720-2x-dark.png","settings-privacy-1280x720-1x-dark.png",
 "settings-playback-1280x720-1x-light.png","settings-library-1920x1080-2x-light.png",
 "settings-audio-1280x720-1x-light.png","settings-video-1920x1080-2x-light.png",
 "settings-network-1280x720-2x-dark.png","settings-performance-1280x720-2x-light.png",
 "discovery-1280x720-1x-light.png","settings-extensions-1280x720-2x-dark.png",
 "settings-extensions-1280x720-1x-dark.png","open-network-stream-1920x1080-1x-light.png",
 "onboarding-1280x720-1x-dark.png","playlist-1920x1080-1x-dark.png",
 "not-found-1280x720-2x-light.png","miniplayer-1920x1080-2x-light.png",
]

# parse batchmeta blocks
blocks = {}
raw = {}
cur = None
for line in META.splitlines():
    if line.startswith("## "):
        cur = line[3:].strip()
        blocks[cur] = {}
        raw[cur] = []
    elif cur and line.startswith("  ") and "=" in line:
        k, v = line.strip().split("=", 1)
        blocks[cur][k.strip()] = v.strip()
        raw[cur].append(line)
SIZE = {}
SHA = {}
for k, lines in raw.items():
    txt = "\n".join(lines)
    m = re.search(r"size=(\d+) sha256=([0-9a-f]{64})", txt)
    if m:
        SIZE[k], SHA[k] = m.group(1), m.group(2)

TEMPLATE = """# AUDIT: ui/e2e/visual/baselines/linux/{name}
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | {size} B / {sha} |
| Dimensions | {w}x{h} (bit depth {bd}, color type {ct} RGB8, non-interlaced) |
| File verdict | REAL (pinned screenshot baseline; content is a real rendered screen) |
| Reachable from shipped entry point? | NO — test asset; consumed by ui/e2e/visual/visual.spec.ts via toHaveScreenshot + snapshotPathTemplate |
| Findings | {findings} |

## 1. Verdict (binary-asset audit)
Real PNG baseline (not a placeholder): {uniq} unique pixel rows among the first 2000 decoded rows ({rows} rows total, {raw} raw bytes) — a solid-color or 1x1 placeholder would show 1-2. Dimensions {w}x{h} {dimnote}. Consumed exactly: `await expect(page).toHaveScreenshot(`${{id}}.png`, {{ clip: box }})` (ui/e2e/visual/visual.spec.ts:103) with `snapshotPathTemplate: "e2e/visual/baselines/{{platform}}/{{arg}}{{ext}}"` (ui/playwright.visual.config.ts:25) — the case-id `{stem}` maps to this file on Linux. Baseline set completeness verified: 216 files in baselines/linux == the manifest's claimed 216 nightly cases.

## 2. What was checked and how
- PNG magic/IHDR/dimensions/IDAT decode: audit/scratch/W16-A10/batch.py (single batch script; see batchmeta.txt).
- Consumer wiring: visual.spec.ts:103 + playwright.visual.config.ts:25 (Read).
- Gate wiring: rg "visual" in audit/runs/baseline/full_gate.txt and audit/runs/baseline/rc-smoke.txt -> NO visual section — the screenshot-diff harness is not run by any committed gate; runs are recorded claims in ui/e2e/visual/README.md (pasted output: "npm run visual:nightly / 32 passed (54.2s)"). Execution here: UNVERIFIED (playwright not run in this sandbox; no baseline artifact).
- Matrix evidence: no docs/feature-matrix.toml row cites baseline PNGs (rg: no hits); parity-report verified-only remains 0/252.
{extra}
## 3. Findings
{findings_body}
"""

for name in PNGS:
    key = f"ui/e2e/visual/baselines/linux/{name}"
    b = blocks.get(key)
    if b is None:
        print("MISSING META", key); continue
    m = re.search(r"'width': (\d+), 'height': (\d+), 'bitdepth': (\d+), 'colortype': (\d+)", b.get("png",""))
    w, h, bd, ct = map(int, m.groups())
    m2 = re.search(r"'unique_rows\(first2000\)': (\d+)", b["png"])
    uniq = m2.group(1)
    rows = re.search(r"'rows_decoded': (\d+)", b["png"]).group(1)
    raw = re.search(r"'raw_bytes': (\d+)", b["png"]).group(1)
    stem = name[:-4]
    # filename dims
    fm = re.search(r"(\d+)x(\d+)", name)
    fw, fh = int(fm.group(1)), int(fm.group(2))
    findings = "S0: 0 S1: 0 S2: 0 S3: 0 S4: 0"
    extra = ""
    fbody = "None beyond the harness-level gap recorded in every baseline audit: the screenshot-diff lane is not wired into any committed gate (full_gate/rc-smoke have no visual section). Harness-level, not file-level."
    if (w, h) != (fw, fh):
        findings = "S0: 0 S1: 0 S2: 0 S3: 0 S4: 1"
        extra = f"\n### Filename/dimension mismatch\nThe case id encodes viewport {fw}x{fh} but the pinned image is {w}x{h}. Cause: visual.spec.ts clips to the `#root` bounding box (visual.spec.ts:97-102), and the onboarding route's #root box is 16 px shorter than the viewport — the baseline bakes that layout state in. Comparison still works (it diffs against the pinned clip), but the name promises a dimension the pixels do not have.\n"
        fbody = f"""F-001 [S4]
Where: ui/e2e/visual/baselines/linux/{name} (content) vs the case id encoded in the filename
What is wrong: filename encodes {fw}x{fh}; IHDR says {w}x{h}. Why it matters: misleading asset naming; also evidence the onboarding route renders a #root box 16 px short of the viewport (layout quirk baked into the pinned baseline). What correct looks like: either fix the route layout or name/record the clip size honestly.
Confidence: HIGH — Verification: ran (batch.py IHDR parse)."""
    out = TEMPLATE.format(name=name, size=SIZE.get(key,"?"), sha=SHA.get(key,"?"),
        w=w, h=h, bd=bd, ct=ct, uniq=uniq, rows=rows, raw=raw, stem=stem,
        findings=findings, extra=extra, findings_body=fbody,
        dimnote="(matches the viewport encoded in the filename)" if (w,h)==(fw,fh) else f"(filename encodes {fw}x{fh} — MISMATCH)")
    dest = ROOT/f"audit/files/ui/e2e/visual/baselines/linux/{name}.audit.md"
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(out)
    print("wrote", dest.name)
