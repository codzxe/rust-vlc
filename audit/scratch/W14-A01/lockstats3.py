#!/usr/bin/env python3
# W14-A01 v3: parse Cargo.lock v4 dep lines correctly: ` "name",` / ` "name ver",`
import re
from collections import defaultdict

def parse(path):
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
                    m = re.match(r'^ "(.*)",?$', line)
                    if m:
                        body = m.group(1)
                        if ' ' in body:
                            n, v = body.split(' ', 1)
                        else:
                            n, v = body, None
                        cur['deps'].append((n, v))
                    elif line == ']':
                        depmode = False
    return pkgs

dt = parse('/home/z/my-project/audit-workspace/apps/desktop-tauri/Cargo.lock')
ws = parse('/home/z/my-project/audit-workspace/Cargo.lock')
print(f"desktop-tauri packages: {len(dt)}; workspace packages: {len(ws)}")

def index(pkgs):
    d = defaultdict(list)
    for p in pkgs: d[p['name']].append(p)
    return d
dtn, wsn = index(dt), index(ws)
bykey = {(p['name'], p['version']): p for p in dt}

roots = [p for p in dt if p['source'] is None]
print("\n== root (path) packages in desktop-tauri lock ==")
for p in roots:
    print(f"  {p['name']} {p['version']}: {[f'{d}@{v}' if v else d for d,v in p['deps']]}")

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
                elif not cands:
                    missing.add((dn, dv))
                else:
                    # ambiguous unversioned edge (shouldn't happen in cargo output)
                    missing.add(('AMBIG:' + dn, dv))
    return seen, missing

croot, missroot = closure([('lumen-desktop-tauri', '0.1.3')])
print(f"\nclosure from lumen-desktop-tauri: {len(croot)} packages "
      f"(of {len(dt)} locked; {len(dt)-len(croot)} outside)")
if missroot: print(f"  unresolved: {sorted(missroot)}")

for t in ['tauri', 'wry', 'tauri-runtime-wry', 'desktop', 'commands', 'tauri-build']:
    vs = [p['version'] for p in dtn.get(t, [])]
    c, m = closure([(t, v) for v in vs])
    extra = f" (unresolved {m})" if m else ""
    print(f"closure from {t} {vs}: {len(c)}{extra}")

# engine-side closure (path crates reachable without tauri): from desktop+commands
ceng, meng = closure([('desktop', '0.1.3'), ('commands', '0.1.0')])
print(f"closure from desktop+commands (engine half, no tauri): {len(ceng)}")

# tauri-only half: closure(tauri,tauri-build) minus engine half
ctauri, _ = closure([('tauri', '2.12.5'), ('tauri-build', '2.7.4')])
only_tauri = ctauri - ceng
print(f"closure from tauri+tauri-build: {len(ctauri)}; of which NOT in engine half: {len(only_tauri)}")
overlap = ctauri & ceng
print(f"overlap between tauri half and engine half: {len(overlap)}")
print("  overlap crates: " + ", ".join(sorted(f"{n} {v}" for n, v in overlap)))

unreach = sorted((p['name'], p['version']) for p in dt if (p['name'], p['version']) not in croot)
print(f"\n== locked, NOT reachable from lumen-desktop-tauri ({len(unreach)}) ==")
for n, v in unreach: print(f"  {n} {v}")

# libloading reverse deps (deny.toml wrapper claim)
print("\n== reverse deps of libloading ==")
for p in dt:
    for dn, dv in p['deps']:
        if dn == 'libloading': print(f"  {p['name']} {p['version']} -> libloading {dv}")

# proc-macro-error reverse deps (RUSTSEC-2024-0370 claim: glib-macros -> ...)
print("== reverse deps of proc-macro-error ==")
for p in dt:
    for dn, dv in p['deps']:
        if dn == 'proc-macro-error': print(f"  {p['name']} {p['version']} -> proc-macro-error {dv}")

# foldhash reverse deps (deny.toml: dom_query -> tauri-utils -> tauri)
print("== reverse deps of foldhash 0.2.0 ==")
for p in dt:
    for dn, dv in p['deps']:
        if dn == 'foldhash': print(f"  {p['name']} {p['version']} -> foldhash {dv}")

# target-lexicon reverse deps (deny.toml: the wry/tauri tree)
print("== reverse deps of target-lexicon ==")
for p in dt:
    for dn, dv in p['deps']:
        if dn == 'target-lexicon': print(f"  {p['name']} {p['version']} -> target-lexicon {dv}")

# GPL-family candidates by name (license data NOT in lockfile; name-based only)
print("\n== crates matching banned deny.toml names (should be NONE) ==")
banned = set()
import tomllib
with open('/home/z/my-project/audit-workspace/deny.toml','rb') as f:
    deny = tomllib.load(f)
for b in deny['bans']['deny']:
    banned.add(b['name'] if isinstance(b, dict) else b)
hits = sorted(n for n in dtn if n in banned)
print(f"  banned names present in desktop-tauri lock: {hits if hits else 'NONE'}")

# dev-dependency detection: crates in lock but not in root closure => includes dev deps?
# lumen-desktop-tauri has no dev-deps in Cargo.toml; tests/single_command.rs exists.
# List which path crates are NOT reachable: e.g. lumen-cli is reachable via desktop.
print("\n== tauri ecosystem version pins (for the audit table) ==")
for n in ['tauri','tauri-build','tauri-codegen','tauri-macros','tauri-runtime',
          'tauri-runtime-wry','tauri-utils','tauri-winres','wry','tao','tao-macros',
          'muda','tray-icon','webkit2gtk','webkit2gtk-sys','gtk','gtk-sys','gtk3-macros',
          'glib','glib-sys','glib-macros','gobject-sys','gio','soup3','soup3-sys',
          'javascriptcore-rs','javascriptcore-rs-sys','libappindicator','libappindicator-sys',
          'libloading','window-vibrancy','softbuffer','x11','x11-dl','objc2','webview2-com']:
    vs = sorted(p['version'] for p in dtn.get(n, []))
    print(f"  {n}: {', '.join(vs) if vs else '-'}")
