# AUDIT: fuzz/corpus/fuzz_playlist_xspf/seed_namespaced.xspf
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (hand-authored hostile XSPF) |
| Size / sha256-prefix | 1024 bytes / 1338d9731a9c8a5b (verified locally) |
| File verdict | REAL — handmade, genuinely adversarial |
| Reachable from shipped entry point? | NO — fuzz seed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256, full text dump (quoted below) |

## 1. Verdict
Real hand-authored seed (the "hand-authored text/cookie/skeleton seeds"
arm, fuzz/README.md:106). It is genuinely hostile: XSPF namespaced elements
(`xspf:` prefix), an `xspf:extension` block with foreign-namespace children
("ignored"), a track with TWO `<xspf:location>` children (first-wins rule),
CDATA with raw `<title>` and `&` inside, a multi-line annotation, a
`<xspf:track/>` empty element. Presence gated (baseline full_gate.txt:57-58).

## 2. What actually works (verified)
- Verbatim (fuzz/corpus/fuzz_playlist_xspf/seed_namespaced.xspf, selected lines):
```
<xspf:track>
      <xspf:location>first-location.mp3</xspf:location>
      <xspf:location>second-location.mp3</xspf:location>
      <xspf:title>First wins</xspf:title>
```
and
```
<xspf:title><![CDATA[CDATA <title> & raw & stuff]]></xspf:title>
```
- The seed's edge cases (duplicate location, CDATA-injection, foreign
  extensions) map to real parser obligations in crates/playlists.

## 3. Findings
None.
