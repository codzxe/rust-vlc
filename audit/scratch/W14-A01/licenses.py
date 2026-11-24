#!/usr/bin/env python3
# W14-A01: license spread from local registry cache for desktop-tauri lock crates.
import os, re, tomllib
from collections import defaultdict, Counter

REG = os.path.expanduser('~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f')

def parse(path):
    pkgs, cur, depmode = [], None, False
    for raw in open(path, encoding='utf-8'):
        line = raw.rstrip('\n')
        if line == '[[package]]':
            cur = {'name': None, 'version': None, 'source': None, 'deps': []}
            pkgs.append(cur); depmode = False
        elif cur is not None:
            if line.startswith('name = "'): cur['name'] = line[8:-1]; depmode = False
            elif line.startswith('version = "'): cur['version'] = line[11:-1]; depmode = False
            elif line.startswith('source = "'): cur['source'] = line[10:-1]; depmode = False
            elif line == 'dependencies = [': depmode = True
            elif depmode:
                m = re.match(r'^ "(.*)",?$', line)
                if m:
                    body = m.group(1)
                    cur['deps'].append(tuple(body.split(' ', 1)) if ' ' in body else (body, None))
                elif line == ']': depmode = False
    return pkgs

dt = parse('/home/z/my-project/audit-workspace/apps/desktop-tauri/Cargo.lock')
reg = [p for p in dt if p['source']]
print(f"registry crates in desktop-tauri lock: {len(reg)}")

def dirname(name, ver):
    # crates.io normalizes: name lowercased, '.'/'_' interchangeable in fs? Actually '-' stays '-'
    return f"{name}-{ver}"

found, missing = {}, []
for p in reg:
    d = os.path.join(REG, dirname(p['name'], p['version']))
    if os.path.isdir(d):
        found[(p['name'], p['version'])] = d
    else:
        missing.append((p['name'], p['version']))
print(f"cached in local registry: {len(found)}; missing: {len(missing)}")

lic = Counter()
gpl = []
nolicense = []
for (n, v), d in sorted(found.items()):
    mf = os.path.join(d, 'Cargo.toml')
    try:
        with open(mf, 'rb') as f:
            m = tomllib.load(f)
        l = m.get('package', {}).get('license') or m.get('package', {}).get('license-file') or 'NONE'
    except Exception as e:
        l = f'PARSE-ERROR:{type(e).__name__}'
    lic[l] += 1
    if 'GPL' in l.upper():
        gpl.append((n, v, l))
    if l in ('NONE',) or l.startswith('PARSE'):
        nolicense.append((n, v))

print("\n== license spread (cached crates only) ==")
for l, c in lic.most_common():
    print(f"  {c:4d}  {l}")
print(f"\nGPL-family hits: {gpl if gpl else 'NONE'}")
print(f"no-license-field crates: {nolicense if nolicense else 'NONE'}")

print(f"\n== uncached ({len(missing)}) — license UNVERIFIED locally ==")
for n, v in sorted(missing):
    print(f"  {n} {v}")
