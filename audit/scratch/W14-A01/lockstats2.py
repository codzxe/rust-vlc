#!/usr/bin/env python3
# W14-A01 v2: parse lockfiles correctly (Cargo.lock v4 format).
import re
from collections import defaultdict

def parse2(path):
    pkgs, cur, depmode = [], None, False
    with open(path, 'r', encoding='utf-8') as f:
        for raw in f:
            line = raw.rstrip('\n')
            if line == '[[package]]':
                cur = {'name': None, 'version': None, 'source': None,
                       'checksum': None, 'deps': []}
                pkgs.append(cur); depmode = False
            elif cur is not None:
                if line.startswith('name = "'):
                    cur['name'] = line[8:-1]; depmode = False
                elif line.startswith('version = "'):
                    cur['version'] = line[11:-1]; depmode = False
                elif line.startswith('source = "'):
                    cur['source'] = line[10:-1]; depmode = False
                elif line.startswith('checksum = "'):
                    cur['checksum'] = line[12:-1]; depmode = False
                elif line == 'dependencies = [':
                    depmode = True
                elif depmode:
                    m = re.match(r'^ "(.+?)(?: \(([^)]+)\))?,?$', line)
                    if m:
                        cur['deps'].append((m.group(1), m.group(2)))
                    elif line == ']':
                        depmode = False
    return pkgs

dt = parse2('/home/z/my-project/audit-workspace/apps/desktop-tauri/Cargo.lock')
ws = parse2('/home/z/my-project/audit-workspace/Cargo.lock')
print(f"desktop-tauri packages: {len(dt)}; workspace packages: {len(ws)}")

def index(pkgs):
    d = defaultdict(list)
    for p in pkgs: d[p['name']].append(p)
    return d
dtn, wsn = index(dt), index(ws)

roots = [p for p in dt if p['source'] is None]
print("\n== root (path) packages in desktop-tauri lock ==")
for p in roots:
    print(f"  {p['name']} {p['version']}: deps={[f'{d}@{v}' for d,v in p['deps']]}")

bykey = {(p['name'], p['version']): p for p in dt}
def closure(starts):
    seen, frontier, missing = set(), list(starts), set()
    while frontier:
        k = frontier.pop()
        if k in seen: continue
        p = bykey.get(k)
        if p is None:
            missing.add(k); continue
        seen.add(k)
        for dn, dv in p['deps']:
            if dv and (dn, dv) in bykey:
                frontier.append((dn, dv))
            else:
                cands = dtn.get(dn, [])
                if len(cands) == 1:
                    frontier.append((dn, cands[0]['version']))
                elif len(cands) > 1 and dv is None:
                    # ambiguous: all versions (upper bound); flag separately
                    for c in cands: frontier.append((dn, c['version']))
                else:
                    missing.add((dn, dv))
    return seen, missing

croot, missroot = closure([('lumen-desktop-tauri', '0.1.3')])
print(f"\nclosure from lumen-desktop-tauri: {len(croot)} packages "
      f"(of {len(dt)} locked; {len(dt)-len(croot)} outside)")
if missroot: print(f"  unresolved edges: {sorted(missroot)[:20]}")

for t in ['tauri', 'wry', 'tauri-runtime-wry']:
    vs = [p['version'] for p in dtn.get(t, [])]
    c, m = closure([(t, v) for v in vs])
    print(f"closure from {t} {vs}: {len(c)}")

# unreachable-from-root list (locked but outside root closure)
unreach = sorted((p['name'], p['version']) for p in dt if (p['name'], p['version']) not in croot)
print(f"\n== locked, NOT reachable from lumen-desktop-tauri ({len(unreach)}) ==")
for n, v in unreach: print(f"  {n} {v}")

# duplicates (recompute with fixed versions)
dups = {n: sorted(p['version'] for p in v) for n, v in dtn.items() if len(v) > 1}
print(f"\n== duplicate names: {len(dups)} names / {sum(len(v) for v in dups.values())} entries ==")
for n, vs in sorted(dups.items()): print(f"  {n}: {', '.join(vs)}")

# checksum sanity
noreg = [p for p in dt if p['source'] and not p['checksum']]
badchk = [p for p in dt if p['checksum'] and not re.fullmatch(r'[0-9a-f]{64}', p['checksum'])]
print(f"\nregistry crates missing checksum: {len(noreg)}; malformed checksums: {len(badchk)}")
oddsrc = {p['source'] for p in dt if p['source'] and 'crates.io-index' not in p['source']}
print(f"non-crates.io sources: {oddsrc if oddsrc else 'NONE'}")

# deny.toml cross-ref
print("\n== deny.toml / docs cross-ref versions ==")
for n in ['tauri','wry','tao','foldhash','dom_query','tauri-utils','target-lexicon',
          'proc-macro-error','glib-macros','libloading','libappindicator','libappindicator-sys',
          'tray-icon','muda','gtk','glib','soup3','webkit2gtk','tauri-build','tauri-codegen',
          'tauri-macros','tauri-runtime','tauri-runtime-wry','getrandom','schemars','web_atoms',
          'rav1d','paste','symphonia-core','ureq','ring','serde','serde_json','toml']:
    vs = sorted(p['version'] for p in dtn.get(n, []))
    print(f"  {n}: {', '.join(vs) if vs else 'NOT IN LOCK'}")

print("\n== reverse deps of libloading ==")
for p in dt:
    for dn, dv in p['deps']:
        if dn == 'libloading': print(f"  {p['name']} {p['version']} -> libloading {dv}")

for nm in ['tauri','wry','tauri-build','lumen-desktop-tauri','desktop','commands','tauri-runtime-wry']:
    for p in dtn.get(nm, []):
        print(f"\n== deps of {nm} {p['version']} ==")
        print("  " + ", ".join(f"{d}@{v or '*'}" for d, v in p['deps']))

# overlap with workspace
shared = set(dtn) & set(wsn)
drift = []
for n in sorted(shared):
    dv = sorted(p['version'] for p in dtn[n]); wv = sorted(p['version'] for p in wsn[n])
    if dv != wv: drift.append((n, wv, dv))
print(f"\n== workspace overlap: shared {len(shared)}, desktop-only {len(set(dtn)-set(wsn))}, "
      f"ws-only {len(set(wsn)-set(dtn))}; version drift on {len(drift)} shared names, "
      f"identical on {len(shared)-len(drift)} ==")
for n, wv, dv in drift:
    print(f"  {n}: ws={','.join(wv)} dt={','.join(dv)}")
print("workspace-only names: " + ", ".join(sorted(set(wsn)-set(dtn))))
print("desktop-only names (count %d): " % len(set(dtn)-set(wsn)) + ", ".join(sorted(set(dtn)-set(wsn))[:40]) + " ...")
