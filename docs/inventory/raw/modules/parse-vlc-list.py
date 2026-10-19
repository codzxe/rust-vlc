#!/usr/bin/env python3
"""0.B2 census parser: VLC 3.0.x `--list-verbose` dump -> modules-linux.json.

Clean-room: parses ONLY observed runtime output (docs/baselines/raw/linux/06-...).
Input format per module:
    <2 spaces><module name><whitespace><description>
    [ <3 spaces>s <shortcut> ]...
      <3 spaces>c <capability> (<score>)
Four base/parent entries are printed without a 'c' line (export, bridge,
playlist, core); they are recorded with capability "(none)".
Run: python3 parse-vlc-list.py <verbose-dump> <terse-dump> <out.json>
"""
import re, sys, json
from collections import OrderedDict, defaultdict

verbose, terse, out = sys.argv[1], sys.argv[2], sys.argv[3]

entries = []
cur = None
for ln, raw in enumerate(open(verbose, encoding='utf-8'), 1):
    line = raw.rstrip('\n')
    if not line.strip() or line.startswith('#'):
        continue
    if line.startswith('   s '):
        cur['shortcuts'].append(line[4:].strip())
    elif line.startswith('   c '):
        m = re.match(r'\s*c\s+(.*?)\s*\((-?\d+)\)\s*$', line)
        assert m, f'line {ln}: unparsable c-line {line!r}'
        cur['capability'], cur['score'] = m.group(1).strip(), int(m.group(2))
    elif line.startswith('  ') and not line.startswith('   '):
        m = re.match(r'\s{2}(\S+)\s+(.*)$', line)
        assert m, f'line {ln}: unparsable header {line!r}'
        cur = OrderedDict([('line', ln), ('name', m.group(1)),
                           ('description', m.group(2).strip()),
                           ('shortcuts', []), ('capability', None), ('score', None)])
        entries.append(cur)
    else:
        raise SystemExit(f'line {ln}: unexpected shape {line!r}')
for e in entries:
    if e['capability'] is None:
        e['capability'] = '(none)'

# cross-check against the terse dump (must be the same 502 header lines)
terse_headers = []
for raw in open(terse, encoding='utf-8'):
    line = raw.rstrip('\n')
    if not line.strip() or line.startswith('#'):
        continue
    m = re.match(r'\s{2}(\S+)\s+(.*)$', line)
    assert m, f'terse unparsable: {line!r}'
    terse_headers.append((m.group(1), m.group(2).strip()))
assert len(terse_headers) == len(entries), \
    f'count mismatch: terse {len(terse_headers)} vs verbose {len(entries)}'
for i, ((tn, td), e) in enumerate(zip(terse_headers, entries)):
    assert (tn, td) == (e['name'], e['description']), f'entry {i} differs: {tn}/{e["name"]}'

# ---- G3 rollup rules (documented in modules-summary.md §2) ----
def bucket(e):
    cap, name, desc = e['capability'], e['name'], e['description']
    if cap in ('demux', 'demux_filter'):
        return 'demuxers'
    if cap in ('audio decoder', 'video decoder', 'hw decoder'):
        return 'decoders'
    if cap in ('access', 'access_demux', 'stream_extractor', 'stream_directory'):
        return 'access'
    if cap == 'stream_filter':
        if name == 'playlist' or (name == 'lua' and 'Playlist' in desc):
            return 'playlist'
        return 'access'
    if cap in ('audio filter', 'audio converter', 'audio resampler',
               'audio renderer', 'audio volume', 'video filter',
               'video converter', 'video blending', 'glconv'):
        return 'filters'
    if cap in ('spu decoder', 'sub source', 'sub filter', 'text renderer'):
        return 'subtitle'
    if cap == 'playlist export':
        return 'playlist'
    if cap in ('services_discovery', 'services probe',
               'renderer probe', 'renderer_discovery'):
        return 'services-discovery'
    if cap == '(none)':
        if name in ('playlist', 'export'):
            return 'playlist'
        return 'other'  # bridge (sout stream base), core (host core)
    return 'other'

rollup = defaultdict(list)
for e in entries:
    rollup[bucket(e)].append(e)

by_cap = defaultdict(list)
for e in entries:
    by_cap[e['capability']].append(e)

def mod_json(e):
    d = OrderedDict([('name', e['name'])])
    if e['shortcuts']:
        d['shortcuts'] = e['shortcuts']
    d['description'] = e['description']
    d['score'] = e['score']
    d['dump_line'] = e['line']
    return d

by_capability = OrderedDict()
for cap in sorted(by_cap, key=lambda c: (-len(by_cap[c]), c)):
    mods = by_cap[cap]
    g = OrderedDict([('count', len(mods))])
    if cap == '(none)':
        g['note'] = 'base/parent plugin entries printed by --list-verbose without a capability line'
    g['modules'] = [mod_json(e) for e in mods]
    by_capability[cap] = g

cap_of = lambda b: sorted({e['capability'] for e in rollup[b]})
seven = OrderedDict()
for b in ('demuxers', 'decoders', 'access', 'filters', 'subtitle',
          'playlist', 'services-discovery', 'other'):
    seven[b] = OrderedDict([('count', len(rollup[b])), ('capabilities', cap_of(b))])

# stats
name_count = defaultdict(int)
for e in entries:
    name_count[e['name']] += 1
top_reg = sorted(name_count.items(), key=lambda kv: (-kv[1], kv[0]))[:12]
top_sc = sorted(((len(e['shortcuts']), e['name'], e['shortcuts'], e['capability'])
                 for e in entries if e['shortcuts']), reverse=True)[:12]

doc = OrderedDict([
    ('census', 'VLC 3.0.23 module/plugin census — Linux (real, observed)'),
    ('task', '0.B2 (doc 02 §3.2, doc 13 §3); clean-room: runtime module listings only'),
    ('generated_from', [
        'docs/baselines/raw/linux/05-vlc-list.txt (vlc --list, 502 lines)',
        'docs/baselines/raw/linux/06-vlc-list-verbose.txt (vlc --list-verbose)',
    ]),
    ('parser', 'docs/inventory/raw/modules/parse-vlc-list.py (this census is regenerable with it)'),
    ('acquisition', 'docs/baselines/raw/linux/02-acquisition-and-hashcheck.txt (LD_LIBRARY_PATH etc.)'),
    ('vlc_version', '3.0.23-2-0-g79128878dd (VLC 3.0.23 Vetinari, Debian 3.0.23-0+deb13u1)'),
    ('platform', 'linux — Debian 13 (trixie) x86_64 headless container'),
    ('plugin_packages', ['vlc-plugin-base', 'vlc-plugin-video-output']),
    ('excluded_debs_headless', ['vlc-plugin-qt', 'vlc-plugin-skins2', 'vlc-plugin-notify',
        'vlc-plugin-samba', 'vlc-plugin-jack', 'vlc-plugin-svg', 'vlc-plugin-fluidsynth',
        'vlc-plugin-visualization', 'vlc-plugin-video-splitter', 'vlc-plugin-access-extra']),
    ('total', len(entries)),
    ('counting_rule', 'One entry per module-capability registration printed by --list-verbose '
        '(the 502 header lines; every header carries at most one capability line). A single plugin '
        'binary can register several entries in different capabilities (avcodec: 5, lua: 8, stats: 6). '
        '4 entries print without a capability line (base/parent modules: export, bridge, playlist, core) '
        'and are recorded as capability "(none)". Sum(by_capability.count) == total == 502, cross-checked '
        '1:1 (name+description, in order) against the 502 module lines of dump 05. Every module object '
        'carries dump_line = line number of its header in dump 06.'),
    ('seven_category_rollup', OrderedDict([
        ('note', 'G3 rollup of card 0.B2 (doc 13 §3). Every registration lands in exactly one bucket. '
                 'The seven categories are the minimum buckets; capabilities that do not fit are under '
                 '"other" and mapped to Lumen plugin kinds or flagged in modules-summary.md §5.'),
        ('buckets', seven),
        ('sum_check', sum(len(v) for v in rollup.values())),
    ])),
    ('stats', OrderedDict([
        ('unique_module_names', len(name_count)),
        ('names_with_most_registrations', [OrderedDict([('name', n), ('registrations', c)]) for n, c in top_reg]),
        ('modules_with_most_shortcuts', [OrderedDict([('name', n), ('capability', cap), ('shortcuts', sc)]) for _, n, sc, cap in top_sc]),
        ('hardware_related_modules', [OrderedDict([('name', e['name']), ('capability', e['capability']),
            ('description', e['description'])]) for e in entries
            if e['name'].startswith(('vaapi', 'vdpau', 'glconv'))]),
    ])),
    ('by_capability', by_capability),
])
json.dump(doc, open(out, 'w', encoding='utf-8'), indent=1, ensure_ascii=False)
print('wrote', out)
print('total', doc['total'], '| rollup:', {b: seven[b]['count'] for b in seven}, '| sum', doc['seven_category_rollup']['sum_check'])
print('unique names', len(name_count), '| top registrations', top_reg[:6])
print('top shortcuts', [(n, len(sc)) for _, n, sc, _ in top_sc][:8])
