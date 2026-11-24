#!/usr/bin/env python3
# W14-A01: parse apps/desktop-tauri/Cargo.lock (+ workspace Cargo.lock) as data.
import re, sys, json
from collections import defaultdict

def parse(path):
    pkgs = []
    cur = None
    depmode = False
    with open(path, 'r', encoding='utf-8') as f:
        for raw in f:
            line = raw.rstrip('\n')
            if line.startswith('[[package]]'):
                cur = {'name': None, 'version': None, 'source': None,
                       'checksum': None, 'deps': [], 'kind': None}
                pkgs.append(cur); depmode = False
            elif cur is not None:
                if line.startswith('name = '):
                    cur['name'] = line[8:-1]; depmode = False
                elif line.startswith('version = '):
                    cur['version'] = line[10:-1]; depmode = False
                elif line.startswith('source = '):
                    cur['source'] = line[9:-1]; depmode = False
                elif line.startswith('checksum = '):
                    cur['checksum'] = line[11:-1]; depmode = False
                elif line == 'dependencies = [':
                    depmode = True
                elif depmode:
                    m = re.match(r'^ "(.+?)(?: \(([^)]+)\))?,?"$', line)
                    if m:
                        cur['deps'].append((m.group(1), m.group(2)))
                    elif line == ']':
                        depmode = False
    return pkgs

dt = parse('/home/z/my-project/audit-workspace/apps/desktop-tauri/Cargo.lock')
ws = parse('/home/z/my-project/audit-workspace/Cargo.lock')

print(f"desktop-tauri packages: {len(dt)}")
print(f"workspace packages:     {len(ws)}")

def index(pkgs):
    byname = defaultdict(list)
    for p in pkgs:
        byname[p['name']].append(p)
    return byname

dtn = index(dt)
wsn = index(ws)

# roots (no source => path dep)
roots = [p for p in dt if p['source'] is None]
print("\n== path/root packages in desktop-tauri lock (no source field) ==")
for p in roots:
    print(f"  {p['name']} {p['version']}  deps={p['deps']}")

# duplicates
dups = {n: v for n, v in dtn.items() if len(v) > 1}
print(f"\n== duplicate-name crates in desktop-tauri lock: {len(dups)} names, "
      f"{sum(len(v) for v in dups.values())} entries ==")
for n in sorted(dups):
    vs = sorted(p['version'] for p in dups[n])
    print(f"  {n}: {', '.join(vs)}")

# missing checksums among registry crates
noreg = [p for p in dt if p['source'] and not p['checksum']]
print(f"\nregistry crates missing checksum: {len(noreg)}")
# non-crates.io sources
oddsrc = {p['source'] for p in dt if p['source'] and 'crates.io-index' not in p['source']}
print(f"non-crates.io sources: {oddsrc if oddsrc else 'NONE'}")

# build graph and closures
def key(name, ver, byname):
    cands = [p for p in byname.get(name, []) if p['version'] == ver] if ver else byname.get(name, [])
    return cands[0] if cands else None

def closure(pkgs, start_names):
    byname = index(pkgs)
    # map: (name,version) -> pkg
    bykey = {(p['name'], p['version']): p for p in pkgs}
    # name-resolution: prefer matching version, else unique, else latest
    def resolve(name, ver):
        if ver:
            k = (name, ver)
            if k in bykey: return k
        cands = byname.get(name, [])
        if len(cands) == 1: return (cands[0]['name'], cands[0]['version'])
        if not cands: return None
        # ambiguity w/o version pin: pick highest (cargo would have pinned in real graph)
        return None
    seen = set()
    frontier = list(start_names)
    while frontier:
        name, ver = frontier.pop()
        k = (name, ver)
        if k in seen: continue
        p = bykey.get(k)
        if p is None: continue
        seen.add(k)
        for (dn, dv) in p['deps']:
            r = resolve(dn, dv)
            if r: frontier.append(r)
            elif dn not in byname:
                print(f"  !! unresolvable dep {dn} {dv} of {name}")
    return seen

root_names = [(p['name'], p['version']) for p in roots if p['name'] == 'lumen-desktop-tauri']
clos_root = closure(dt, root_names)
print(f"\nclosure from lumen-desktop-tauri: {len(clos_root)} of {len(dt)} locked "
      f"({len(dt)-len(clos_root)} locked-but-unreachable-from-root)")

# closure from tauri and wry specifically
for target in ['tauri', 'wry', 'tao', 'muda', 'webkit2gtk', 'gtk', 'glib']:
    vs = [p['version'] for p in dtn.get(target, [])]
    if not vs:
        print(f"{target}: NOT IN LOCK")
        continue
    c = closure(dt, [(target, v) for v in vs])
    print(f"closure from {target} ({', '.join(vs)}): {len(c)}")

# locked-but-unreachable list
unreach = [(p['name'], p['version']) for p in dt if (p['name'], p['version']) not in clos_root]
print(f"\n== locked but NOT in root closure ({len(unreach)}) ==")
for n, v in sorted(unreach):
    print(f"  {n} {v}")

# exact versions of interest (deny.toml cross-ref)
print("\n== deny.toml cross-ref exact versions ==")
for n in ['tauri', 'wry', 'tao', 'foldhash', 'dom_query', 'tauri-utils', 'target-lexicon',
          'proc-macro-error', 'glib-macros', 'libloading', 'libappindicator', 'libappindicator-sys',
          'tray-icon', 'muda', 'webkit2gtk', 'webkit2gtk-sys', 'gtk', 'gtk-sys', 'glib',
          'soup', 'soup3', 'javascriptcore', 'serde', 'serde_json', 'cargo_metadata',
          'tauri-build', 'tauri-codegen', 'tauri-macros', 'tauri-runtime', 'tauri-runtime-wry',
          'tauri-plugin-', 'getrandom', 'zbus', 'nix', 'dirs', 'png', 'ico', 'embed-plist',
          'http', 'url', 'reqwest', 'ureq', 'raw-window-handle', 'windows', 'windows-sys',
          'windows-strings', 'windows-core', 'objc2', 'icrate', 'dpi', 'miniblink', 'gdk',
          'gdk-sys', 'gdkwayland-sys', 'gdkx11', 'gdkx11-sys', 'pango', 'pangocairo', 'cairo',
          'cairo-sys-rs', 'harfbuzz', 'freetype', 'fontconfig', 'webview2-com', 'windows_im'] :
    if n in dtn:
        print(f"  {n}: " + ", ".join(sorted(p['version'] for p in dtn[n])))
    else:
        print(f"  {n}: NOT IN desktop-tauri LOCK")

# who depends on libloading (wrapper check)
print("\n== reverse deps of libloading (deny.toml wrapper claim) ==")
for p in dt:
    for (dn, dv) in p['deps']:
        if dn == 'libloading':
            print(f"  {p['name']} {p['version']} -> libloading {dv}")

# tauri's own deps
for nm in ['tauri', 'wry']:
    for p in dtn.get(nm, []):
        print(f"\n== deps of {nm} {p['version']} ==")
        print("  " + ", ".join(f"{d}@{v or '?'}" for d, v in p['deps']))

# workspace overlap / drift
print("\n== workspace lockfile overlap ==")
shared = set(dtn) & set(wsn)
print(f"names in both: {len(shared)}; desktop-only: {len(set(dtn)-set(wsn))}; "
      f"workspace-only: {len(set(wsn)-set(dtn))}")
drift = []
for n in sorted(shared):
    dv = sorted(p['version'] for p in dtn[n])
    wv = sorted(p['version'] for p in wsn[n])
    if dv != wv:
        drift.append((n, wv, dv))
print(f"shared names with differing version sets: {len(drift)}")
for n, wv, dv in drift:
    print(f"  {n}: workspace={','.join(wv)}  desktop-tauri={','.join(dv)}")
same = [n for n in shared if sorted(p['version'] for p in dtn[n]) == sorted(p['version'] for p in wsn[n])]
print(f"shared names with identical version sets: {len(same)}")

# engine crates present in desktop-tauri lock (path deps pulling engine?)
print("\n== path/workspace crates in desktop lock (source is None) — full dep list ==")
for p in roots:
    print(f"  {p['name']} {p['version']}: {[d for d,_ in p['deps']]}")
