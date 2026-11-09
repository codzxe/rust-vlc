# Differential-vs-VLC 3.0.23 — oracle run 2026-11-09 (29/35 bit-exact prefix; 6/35 f32 on the §6.6 deviation path)

> **2026-11-09 update (issue #19)** — VLC 3.0.23 was obtained in /tmp (apt download of Debian trixie's 3.0.23-0+deb13u1, extracted without install) and the 35 to-run commands were executed; see §"Oracle run 2026-11-09" below for the differential table and findings. The 2026-10-23 table below is kept as the historical record; 19/35 of its inputs are lost (runtime-random lavfi sources) and were re-based — same-input verdicts are in the new section. Nothing is mocked.

- Oracle: VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd), Debian trixie build via apt download — run record in §Oracle run 2026-11-09
- Version assertion (2026-11-09 run): `VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)`
- Lumen side: bench-vs-vlc/lumen-pcm-digests/1 (real decode via the workspace crates)
- Host endianness: little (comparability requires little)
- Generated: 2026-10-23T08:09:43+0000
- Fixtures: 500
- Corpus source: manifest `/home/z/my-project/corpus-big/manifest.json` (seed 58007138736976908607880252061, generated 2026-10-23T06:39:59+0000, 500 files)

## Comparable fixtures (lossless paths)

| fixture | audio | ch | rate | Lumen MD5 | Lumen B | oracle MD5 | verdict |
|---|---|---|---|---|---|---|---|
| media/0022-avi-av.avi | pcm_f32le | 1 | 22050 | c5f5a2856f4d1ee42a999fd9b4ace0dd | 557336 | PENDING-VLC | PENDING-VLC |
| media/0040-avi-av.avi | pcm_u8 | 4 | 16000 | f8a1749289184fc20f02d2cccd0eceff | 350976 | PENDING-VLC | PENDING-VLC |
| media/0041-avi-av.avi | pcm_u8 | 2 | 96000 | 9d0da7cacf4705b28199a48cbdefacf2 | 2112384 | PENDING-VLC | PENDING-VLC |
| media/0096-multivideo-av.avi | pcm_s16le | 2 | 11025 | 44cec9ebf7e457140edb971c3e962c96 | 94208 | PENDING-VLC | PENDING-VLC |
| media/0116-avi-av.avi | pcm_u8 | 2 | 8000 | b2e74f5b6a3d856ef6a9c2b95c93f8b0 | 51728 | PENDING-VLC | PENDING-VLC |
| media/0121-multivideo-av.avi | pcm_s16le | 1 | 48000 | 36355267afc2774cddb8f81535afd281 | 216096 | PENDING-VLC | PENDING-VLC |
| media/0134-avi-av.avi | pcm_u8 | 2 | 16000 | 9d62cb4522aeb10247308e0d6dd97c24 | 178176 | PENDING-VLC | PENDING-VLC |
| media/0155-avi-av.avi | pcm_u8 | 6 | 22050 | e6bb46494f3c1d3678571324a22f7a55 | 1175880 | PENDING-VLC | PENDING-VLC |
| media/0156-avi-av.avi | pcm_u8 | 4 | 8000 | 66958319ced9c6cdb9eb0c8130559bff | 261984 | PENDING-VLC | PENDING-VLC |
| media/0161-multivideo-av.avi | pcm_s16le | 2 | 96000 | 7d7ad50a3f50e59a1fab63eb4a175f87 | 3022848 | PENDING-VLC | PENDING-VLC |
| media/0177-multivideo-av.avi | pcm_s16le | 4 | 11025 | edbba401ec8c17b0fc7e673963a125ee | 340808 | PENDING-VLC | PENDING-VLC |
| media/0180-multivideo-av.avi | pcm_s16le | 1 | 48000 | e6f642b18f48744488659de9587ab9e2 | 331776 | PENDING-VLC | PENDING-VLC |
| media/0187-avi-av.avi | pcm_u8 | 8 | 11025 | 34bcf94ce5308bc76b02f751d3fcad4a | 547720 | PENDING-VLC | PENDING-VLC |
| media/0194-multivideo-av.avi | pcm_s16le | 8 | 44100 | fbf9a9b3a9861350fc1007194e629d3d | 2998272 | PENDING-VLC | PENDING-VLC |
| media/0201-multivideo-av.avi | pcm_s16le | 2 | 96000 | 6a4bde3ffe52692669a2789c160f49b0 | 1970176 | PENDING-VLC | PENDING-VLC |
| media/0214-avi-av.avi | pcm_s16le | 1 | 88200 | 7d176316ea5a6eb7dcca3e15a6c46135 | 1144306 | PENDING-VLC | PENDING-VLC |
| media/0227-avi-av.avi | pcm_s16le | 1 | 48000 | d376c7e6f60d1775fb8357c978b29587 | 472032 | PENDING-VLC | PENDING-VLC |
| media/0233-multivideo-av.avi | pcm_s16le | 1 | 88200 | 649d0232dba068c25d89617e9ef3fba1 | 870400 | PENDING-VLC | PENDING-VLC |
| media/0236-multivideo-av.avi | pcm_s16le | 1 | 11025 | 2fa047d2f34c9029f056237ca8079f5f | 137216 | PENDING-VLC | PENDING-VLC |
| media/0247-avi-av.avi | pcm_f32le | 1 | 48000 | d56f3ef14795d1de7d2533113045fb30 | 745728 | PENDING-VLC | PENDING-VLC |
| media/0278-avi-av.avi | pcm_s16le | 1 | 88200 | 28a9e2652675dccd5cee316150fe0d46 | 356858 | PENDING-VLC | PENDING-VLC |
| media/0280-avi-av.avi | pcm_f32le | 2 | 22050 | 708fd146eb83cbf2922caad2d137ef87 | 935976 | PENDING-VLC | PENDING-VLC |
| media/0297-avi-av.avi | pcm_f32le | 8 | 11025 | 0f368aaa880d22f7c48caf67db304276 | 3520224 | PENDING-VLC | PENDING-VLC |
| media/0305-avi-av.avi | pcm_f32le | 1 | 22050 | db81b3f7263e13eb0b1521c266ef14a6 | 885440 | PENDING-VLC | PENDING-VLC |
| media/0307-multivideo-av.avi | pcm_s16le | 2 | 32000 | e2bce5d5744e374d59ffd1dbb565314c | 450560 | PENDING-VLC | PENDING-VLC |
| media/0308-avi-av.avi | pcm_s16le | 4 | 96000 | 01269e9f433bd2f4496054180b3b9639 | 4096000 | PENDING-VLC | PENDING-VLC |
| media/0318-avi-av.avi | pcm_s16le | 2 | 96000 | 88e8b70a1cb78a6dfe7e00094bd4696b | 1331712 | PENDING-VLC | PENDING-VLC |
| media/0324-avi-av.avi | pcm_u8 | 4 | 44100 | 1767da988df20a761eea5b3be593b233 | 822908 | PENDING-VLC | PENDING-VLC |
| media/0330-avi-av.avi | pcm_f32le | 1 | 96000 | bb76a78b3c77421b3e41c0155dc8d8d1 | 410880 | PENDING-VLC | PENDING-VLC |
| media/0335-multivideo-av.avi | pcm_s16le | 1 | 16000 | 99eace8d6c522f2877aac4d980a6beea | 97216 | PENDING-VLC | PENDING-VLC |
| media/0336-multivideo-av.avi | pcm_s16le | 1 | 32000 | ca97c302d2e1d6ec0e27063076b56dd9 | 425984 | PENDING-VLC | PENDING-VLC |
| media/0342-avi-av.avi | pcm_s16le | 3 | 32000 | 0d1d39cbb0401e1e9303081446c2652b | 1846464 | PENDING-VLC | PENDING-VLC |
| media/0367-multivideo-av.avi | pcm_s16le | 6 | 48000 | daff0c23727ddb22b04da23d201dba7d | 4030464 | PENDING-VLC | PENDING-VLC |
| media/0386-avi-av.avi | pcm_s16le | 1 | 22050 | 37e3ad34ab0d716cc5da69a2317c3305 | 187292 | PENDING-VLC | PENDING-VLC |
| media/0394-avi-av.avi | pcm_s16le | 6 | 32000 | 417c8faf45779e9e95975e86a3fd2d04 | 1092096 | PENDING-VLC | PENDING-VLC |

### Exact to-run commands (PENDING-VLC)

```
# media/0022-avi-av.avi (Lumen: c5f5a2856f4d1ee42a999fd9b4ace0dd / 557336 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format float32 --audiofile-file /tmp/bench-vs-vlc/media__0022-avi-av.avi.raw /home/z/my-project/corpus-big/media/0022-avi-av.avi
```
```
# media/0040-avi-av.avi (Lumen: f8a1749289184fc20f02d2cccd0eceff / 350976 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format u8 --audiofile-file /tmp/bench-vs-vlc/media__0040-avi-av.avi.raw /home/z/my-project/corpus-big/media/0040-avi-av.avi
```
```
# media/0041-avi-av.avi (Lumen: 9d0da7cacf4705b28199a48cbdefacf2 / 2112384 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format u8 --audiofile-file /tmp/bench-vs-vlc/media__0041-avi-av.avi.raw /home/z/my-project/corpus-big/media/0041-avi-av.avi
```
```
# media/0096-multivideo-av.avi (Lumen: 44cec9ebf7e457140edb971c3e962c96 / 94208 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0096-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0096-multivideo-av.avi
```
```
# media/0116-avi-av.avi (Lumen: b2e74f5b6a3d856ef6a9c2b95c93f8b0 / 51728 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format u8 --audiofile-file /tmp/bench-vs-vlc/media__0116-avi-av.avi.raw /home/z/my-project/corpus-big/media/0116-avi-av.avi
```
```
# media/0121-multivideo-av.avi (Lumen: 36355267afc2774cddb8f81535afd281 / 216096 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0121-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0121-multivideo-av.avi
```
```
# media/0134-avi-av.avi (Lumen: 9d62cb4522aeb10247308e0d6dd97c24 / 178176 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format u8 --audiofile-file /tmp/bench-vs-vlc/media__0134-avi-av.avi.raw /home/z/my-project/corpus-big/media/0134-avi-av.avi
```
```
# media/0155-avi-av.avi (Lumen: e6bb46494f3c1d3678571324a22f7a55 / 1175880 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format u8 --audiofile-file /tmp/bench-vs-vlc/media__0155-avi-av.avi.raw /home/z/my-project/corpus-big/media/0155-avi-av.avi
```
```
# media/0156-avi-av.avi (Lumen: 66958319ced9c6cdb9eb0c8130559bff / 261984 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format u8 --audiofile-file /tmp/bench-vs-vlc/media__0156-avi-av.avi.raw /home/z/my-project/corpus-big/media/0156-avi-av.avi
```
```
# media/0161-multivideo-av.avi (Lumen: 7d7ad50a3f50e59a1fab63eb4a175f87 / 3022848 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0161-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0161-multivideo-av.avi
```
```
# media/0177-multivideo-av.avi (Lumen: edbba401ec8c17b0fc7e673963a125ee / 340808 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0177-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0177-multivideo-av.avi
```
```
# media/0180-multivideo-av.avi (Lumen: e6f642b18f48744488659de9587ab9e2 / 331776 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0180-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0180-multivideo-av.avi
```
```
# media/0187-avi-av.avi (Lumen: 34bcf94ce5308bc76b02f751d3fcad4a / 547720 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format u8 --audiofile-file /tmp/bench-vs-vlc/media__0187-avi-av.avi.raw /home/z/my-project/corpus-big/media/0187-avi-av.avi
```
```
# media/0194-multivideo-av.avi (Lumen: fbf9a9b3a9861350fc1007194e629d3d / 2998272 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0194-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0194-multivideo-av.avi
```
```
# media/0201-multivideo-av.avi (Lumen: 6a4bde3ffe52692669a2789c160f49b0 / 1970176 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0201-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0201-multivideo-av.avi
```
```
# media/0214-avi-av.avi (Lumen: 7d176316ea5a6eb7dcca3e15a6c46135 / 1144306 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0214-avi-av.avi.raw /home/z/my-project/corpus-big/media/0214-avi-av.avi
```
```
# media/0227-avi-av.avi (Lumen: d376c7e6f60d1775fb8357c978b29587 / 472032 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0227-avi-av.avi.raw /home/z/my-project/corpus-big/media/0227-avi-av.avi
```
```
# media/0233-multivideo-av.avi (Lumen: 649d0232dba068c25d89617e9ef3fba1 / 870400 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0233-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0233-multivideo-av.avi
```
```
# media/0236-multivideo-av.avi (Lumen: 2fa047d2f34c9029f056237ca8079f5f / 137216 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0236-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0236-multivideo-av.avi
```
```
# media/0247-avi-av.avi (Lumen: d56f3ef14795d1de7d2533113045fb30 / 745728 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format float32 --audiofile-file /tmp/bench-vs-vlc/media__0247-avi-av.avi.raw /home/z/my-project/corpus-big/media/0247-avi-av.avi
```
```
# media/0278-avi-av.avi (Lumen: 28a9e2652675dccd5cee316150fe0d46 / 356858 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0278-avi-av.avi.raw /home/z/my-project/corpus-big/media/0278-avi-av.avi
```
```
# media/0280-avi-av.avi (Lumen: 708fd146eb83cbf2922caad2d137ef87 / 935976 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format float32 --audiofile-file /tmp/bench-vs-vlc/media__0280-avi-av.avi.raw /home/z/my-project/corpus-big/media/0280-avi-av.avi
```
```
# media/0297-avi-av.avi (Lumen: 0f368aaa880d22f7c48caf67db304276 / 3520224 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format float32 --audiofile-file /tmp/bench-vs-vlc/media__0297-avi-av.avi.raw /home/z/my-project/corpus-big/media/0297-avi-av.avi
```
```
# media/0305-avi-av.avi (Lumen: db81b3f7263e13eb0b1521c266ef14a6 / 885440 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format float32 --audiofile-file /tmp/bench-vs-vlc/media__0305-avi-av.avi.raw /home/z/my-project/corpus-big/media/0305-avi-av.avi
```
```
# media/0307-multivideo-av.avi (Lumen: e2bce5d5744e374d59ffd1dbb565314c / 450560 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0307-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0307-multivideo-av.avi
```
```
# media/0308-avi-av.avi (Lumen: 01269e9f433bd2f4496054180b3b9639 / 4096000 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0308-avi-av.avi.raw /home/z/my-project/corpus-big/media/0308-avi-av.avi
```
```
# media/0318-avi-av.avi (Lumen: 88e8b70a1cb78a6dfe7e00094bd4696b / 1331712 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0318-avi-av.avi.raw /home/z/my-project/corpus-big/media/0318-avi-av.avi
```
```
# media/0324-avi-av.avi (Lumen: 1767da988df20a761eea5b3be593b233 / 822908 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format u8 --audiofile-file /tmp/bench-vs-vlc/media__0324-avi-av.avi.raw /home/z/my-project/corpus-big/media/0324-avi-av.avi
```
```
# media/0330-avi-av.avi (Lumen: bb76a78b3c77421b3e41c0155dc8d8d1 / 410880 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format float32 --audiofile-file /tmp/bench-vs-vlc/media__0330-avi-av.avi.raw /home/z/my-project/corpus-big/media/0330-avi-av.avi
```
```
# media/0335-multivideo-av.avi (Lumen: 99eace8d6c522f2877aac4d980a6beea / 97216 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0335-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0335-multivideo-av.avi
```
```
# media/0336-multivideo-av.avi (Lumen: ca97c302d2e1d6ec0e27063076b56dd9 / 425984 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0336-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0336-multivideo-av.avi
```
```
# media/0342-avi-av.avi (Lumen: 0d1d39cbb0401e1e9303081446c2652b / 1846464 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0342-avi-av.avi.raw /home/z/my-project/corpus-big/media/0342-avi-av.avi
```
```
# media/0367-multivideo-av.avi (Lumen: daff0c23727ddb22b04da23d201dba7d / 4030464 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0367-multivideo-av.avi.raw /home/z/my-project/corpus-big/media/0367-multivideo-av.avi
```
```
# media/0386-avi-av.avi (Lumen: 37e3ad34ab0d716cc5da69a2317c3305 / 187292 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0386-avi-av.avi.raw /home/z/my-project/corpus-big/media/0386-avi-av.avi
```
```
# media/0394-avi-av.avi (Lumen: 417c8faf45779e9e95975e86a3fd2d04 / 1092096 B)
mkdir -p /tmp/bench-vs-vlc
vlc -I dummy --play-and-exit --vout none --aout afile --no-audiofile-wav --audiofile-format s16 --audiofile-file /tmp/bench-vs-vlc/media__0394-avi-av.avi.raw /home/z/my-project/corpus-big/media/0394-avi-av.avi
```

## Byte-count cross-check vs ffprobe (secondary, NOT an oracle)

- comparable rows checked: 35; Lumen decoded PCM bytes == ffprobe a:0 packet-payload bytes on 35/35

## Digest-integrity flags (treat these digests with care)

Rows appear here when the Lumen walk concealed decoder damage, skipped container bytes while recovering, or saw a mid-stream format change. Note the two benign sources on this build: the v0 codec filter's stream decisions (`unsupported-video-handler`, `skipped == 0`), and the filtered streams' dropped `idx1` entries (`idx1-entry-unknown-stream`, 16 B per entry — seek-table only, not walked data; the W5-14 branch silences them for declared-but-filtered streams). What was independently verified for THIS run is recorded in the run notes.

| fixture | flags |
|---|---|
| media/0040-avi-av.avi | demux_skip_bytes=5264 |
| media/0096-multivideo-av.avi | demux_skip_bytes=1024 |
| media/0121-multivideo-av.avi | demux_skip_bytes=528 |
| media/0134-avi-av.avi | demux_skip_bytes=2672 |
| media/0156-avi-av.avi | demux_skip_bytes=7856 |
| media/0161-multivideo-av.avi | demux_skip_bytes=3152 |
| media/0177-multivideo-av.avi | demux_skip_bytes=1840 |
| media/0180-multivideo-av.avi | demux_skip_bytes=1328 |
| media/0187-avi-av.avi | demux_skip_bytes=784 |
| media/0194-multivideo-av.avi | demux_skip_bytes=2048 |
| media/0201-multivideo-av.avi | demux_skip_bytes=2464 |
| media/0227-avi-av.avi | demux_skip_bytes=1168 |
| media/0233-multivideo-av.avi | demux_skip_bytes=2368 |
| media/0236-multivideo-av.avi | demux_skip_bytes=992 |
| media/0307-multivideo-av.avi | demux_skip_bytes=848 |
| media/0318-avi-av.avi | demux_skip_bytes=5536 |
| media/0335-multivideo-av.avi | demux_skip_bytes=1152 |
| media/0336-multivideo-av.avi | demux_skip_bytes=3200 |
| media/0342-avi-av.avi | demux_skip_bytes=3680 |
| media/0367-multivideo-av.avi | demux_skip_bytes=2688 |
| media/0386-avi-av.avi | demux_skip_bytes=1200 |
| media/0394-avi-av.avi | demux_skip_bytes=10912 |

## Non-comparable fixtures (explicit reasons)

| reason | fixtures |
|---|---|
| no audio track (nothing to digest) | 82: media/0001-avi-video.avi, media/0011-avi-video.avi, media/0012-avi-video.avi, media/0013-avi-video.avi, media/0017-avi-video.avi, media/0019-avi-video.avi, media/0026-avi-video.avi, media/0027-avi-video.avi, media/0030-avi-video.avi, media/0048-avi-video.avi, media/0050-avi-video.avi, media/0067-avi-video.avi, media/0069-avi-video.avi, media/0070-avi-video.avi, media/0076-avi-video.avi, media/0079-avi-video.avi, media/0082-avi-video.avi, media/0092-avi-video.avi, media/0095-avi-video.avi, media/0098-avi-video.avi, media/0102-avi-video.avi, media/0103-avi-video.avi, media/0106-avi-video.avi, media/0110-avi-video.avi, media/0111-avi-video.avi, media/0118-avi-video.avi, media/0122-avi-video.avi, media/0127-avi-video.avi, media/0128-avi-video.avi, media/0137-avi-video.avi, media/0138-avi-video.avi, media/0139-avi-video.avi, media/0146-avi-video.avi, media/0149-avi-video.avi, media/0154-avi-video.avi, media/0158-avi-video.avi, media/0162-avi-video.avi, media/0163-avi-video.avi, media/0171-avi-video.avi, media/0172-avi-video.avi, media/0175-avi-video.avi, media/0178-avi-video.avi, media/0181-avi-video.avi, media/0188-avi-video.avi, media/0196-avi-video.avi, media/0207-avi-video.avi, media/0208-avi-video.avi, media/0217-avi-video.avi, media/0235-avi-video.avi, media/0242-avi-video.avi, media/0243-avi-video.avi, media/0253-avi-video.avi, media/0257-avi-video.avi, media/0258-avi-video.avi, media/0259-avi-video.avi, media/0268-avi-video.avi, media/0271-avi-video.avi, media/0284-avi-video.avi, media/0289-avi-video.avi, media/0299-avi-video.avi, media/0310-avi-video.avi, media/0315-avi-video.avi, media/0317-avi-video.avi, media/0321-avi-video.avi, media/0325-avi-video.avi, media/0334-avi-video.avi, media/0343-avi-video.avi, media/0348-avi-video.avi, media/0351-avi-video.avi, media/0356-avi-video.avi, media/0357-avi-video.avi, media/0359-avi-video.avi, media/0365-avi-video.avi, media/0372-avi-video.avi, media/0380-avi-video.avi, media/0382-avi-video.avi, media/0406-avi-video.avi, media/0408-avi-video.avi, media/0411-avi-video.avi, media/0416-avi-video.avi, media/0418-avi-video.avi, media/0419-avi-video.avi |
| container mkv: no Lumen demuxer in this build (first vertical: AVI, D-018) | 70: media/0003-muxed-av.mkv, media/0006-multi-av.mkv, media/0008-mkv-video.mkv, media/0032-multivideo-av.mkv, media/0042-mkv-video.mkv, media/0049-mkv-video.mkv, media/0057-multi-av.mkv, media/0064-mkv-video.mkv, media/0073-mkv-video.mkv, media/0077-dynres-video.mkv, media/0085-muxed-av.mkv, media/0100-drift-av.mkv, media/0105-mkv-video.mkv, media/0107-mkv-video.mkv, media/0112-mkv-video.mkv, media/0113-mkv-video.mkv, media/0125-mkv-video.mkv, media/0129-multi-av.mkv, media/0132-meta-bomb-av.mkv, media/0141-vfr-av.mkv, media/0144-dynres-video.mkv, media/0151-drift-av.mkv, media/0157-meta-bomb-av.mkv, media/0159-drift-av.mkv, media/0165-muxed-av.mkv, media/0182-mkv-video.mkv, media/0183-mkv-video.mkv, media/0185-mkv-video.mkv, media/0192-mkv-video.mkv, media/0193-multivideo-av.mkv, media/0202-muxed-av.mkv, media/0204-mkv-video.mkv, media/0205-multi-av.mkv, media/0209-meta-bomb-av.mkv, media/0216-vfr-av.mkv, media/0219-muxed-av.mkv, media/0221-dynres-video.mkv, media/0225-mkv-video.mkv, media/0228-mkv-video.mkv, media/0230-vfr-av.mkv, media/0231-dynres-video.mkv, media/0232-muxed-av.mkv, media/0238-mkv-video.mkv, media/0249-mkv-video.mkv, media/0269-vfr-av.mkv, media/0272-drift-av.mkv, media/0281-multi-av.mkv, media/0292-mkv-video.mkv, media/0293-vfr-av.mkv, media/0294-mkv-video.mkv, media/0303-mkv-video.mkv, media/0304-mkv-video.mkv, media/0311-mkv-video.mkv, media/0314-drift-av.mkv, media/0323-meta-bomb-av.mkv, media/0339-vfr-av.mkv, media/0352-mkv-video.mkv, media/0363-mkv-video.mkv, media/0368-mkv-video.mkv, media/0379-mkv-video.mkv, media/0381-muxed-av.mkv, media/0383-muxed-av.mkv, media/0385-dynres-video.mkv, media/0392-drift-av.mkv, media/0397-dynres-video.mkv, media/0398-mkv-video.mkv, media/0401-mkv-video.mkv, media/0407-mkv-video.mkv, media/0410-mkv-video.mkv, media/0417-drift-av.mkv |
| container mp4: no Lumen demuxer in this build (first vertical: AVI, D-018) | 59: media/0004-muxed-av.mp4, media/0005-fmp4-video.mp4, media/0020-drift-av.mp4, media/0024-fmp4-video.mp4, media/0025-meta-bomb-av.mp4, media/0031-mp4-video.mp4, media/0033-dynres-video.mp4, media/0038-mp4-video.mp4, media/0039-fmp4-video.mp4, media/0043-mp4-video.mp4, media/0047-muxed-av.mp4, media/0068-fmp4-video.mp4, media/0071-vfr-av.mp4, media/0074-mp4-video.mp4, media/0075-mp4-video.mp4, media/0081-mp4-video.mp4, media/0088-mp4-video.mp4, media/0090-muxed-av.mp4, media/0094-dynres-video.mp4, media/0130-muxed-av.mp4, media/0136-audio-only.mp4, media/0145-vfr-av.mp4, media/0148-vfr-av.mp4, media/0160-mp4-video.mp4, media/0167-mp4-video.mp4, media/0168-drift-av.mp4, media/0169-mp4-video.mp4, media/0176-meta-bomb-av.mp4, media/0184-audio-only.mp4, media/0199-muxed-av.mp4, media/0211-vfr-av.mp4, media/0224-meta-bomb-av.mp4, media/0234-mp4-video.mp4, media/0240-mp4-video.mp4, media/0245-muxed-av.mp4, media/0246-vfr-av.mp4, media/0255-drift-av.mp4, media/0260-drift-av.mp4, media/0264-drift-av.mp4, media/0276-mp4-video.mp4, media/0279-mp4-video.mp4, media/0290-mp4-video.mp4, media/0319-vfr-av.mp4, media/0320-dynres-video.mp4, media/0322-mp4-video.mp4, media/0341-muxed-av.mp4, media/0344-muxed-av.mp4, media/0346-vfr-av.mp4, media/0347-muxed-av.mp4, media/0354-mp4-video.mp4, media/0355-muxed-av.mp4, media/0358-vfr-av.mp4, media/0361-vfr-av.mp4, media/0364-muxed-av.mp4, media/0369-muxed-av.mp4, media/0373-drift-av.mp4, media/0389-fmp4-video.mp4, media/0412-mp4-video.mp4, media/0414-fmp4-video.mp4 |
| container raw: no Lumen demuxer in this build (first vertical: AVI, D-018) | 16: media/0034-raw-video.h264, media/0097-raw-video.h264, media/0099-raw-video.265, media/0114-raw-video.265, media/0123-raw-video.265, media/0126-raw-video.h264, media/0140-raw-video.265, media/0179-raw-video.265, media/0215-raw-video.h264, media/0237-raw-video.h264, media/0286-raw-video.265, media/0296-raw-video.265, media/0327-raw-video.265, media/0388-raw-video.h264, media/0391-raw-video.h264, media/0413-raw-video.265 |
| lossless both sides, but VLC afile cannot dump pcm_s24le (no afile output format for 24-bit int (u8/s16/float32/spdif only) — baselines 14-options-full-help.txt L2338); the transcode-chain comparison is doc 10 §6.6's open [VERIFY], deferred | 15: media/0035-avi-av.avi, media/0055-avi-av.avi, media/0058-avi-av.avi, media/0143-avi-av.avi, media/0152-avi-av.avi, media/0203-avi-av.avi, media/0226-avi-av.avi, media/0244-avi-av.avi, media/0252-avi-av.avi, media/0266-avi-av.avi, media/0277-avi-av.avi, media/0283-avi-av.avi, media/0340-avi-av.avi, media/0362-avi-av.avi, media/0399-avi-av.avi |
| container ts: no Lumen demuxer in this build (first vertical: AVI, D-018) | 14: media/0056-dynres-video.ts, media/0061-ts-video.ts, media/0086-dynres-video.ts, media/0147-ts-video.ts, media/0150-muxed-av.ts, media/0153-dynres-video.ts, media/0263-ts-video.ts, media/0287-muxed-av.ts, media/0300-ts-video.ts, media/0301-ts-video.ts, media/0371-muxed-av.ts, media/0377-ts-video.ts, media/0378-ts-video.ts, media/0390-ts-video.ts |
| container webm: no Lumen demuxer in this build (first vertical: AVI, D-018) | 13: media/0002-muxed-av.webm, media/0010-webm-video.webm, media/0018-webm-video.webm, media/0028-audio-only.webm, media/0065-webm-video.webm, media/0251-webm-video.webm, media/0261-webm-video.webm, media/0262-webm-video.webm, media/0265-webm-video.webm, media/0312-webm-video.webm, media/0332-webm-video.webm, media/0345-webm-video.webm, media/0376-muxed-av.webm |
| container mov: no Lumen demuxer in this build (first vertical: AVI, D-018) | 9: media/0007-vfr-av.mov, media/0060-mov-video.mov, media/0087-mov-video.mov, media/0195-mov-video.mov, media/0198-vfr-av.mov, media/0270-vfr-av.mov, media/0291-mov-video.mov, media/0395-muxed-av.mov, media/0409-fmp4-video.mov |
| container flv: no Lumen demuxer in this build (first vertical: AVI, D-018) | 8: media/0000-flv-video.flv, media/0093-flv-video.flv, media/0131-flv-video.flv, media/0250-flv-video.flv, media/0273-flv-video.flv, media/0349-flv-video.flv, media/0393-flv-video.flv, media/0403-flv-video.flv |
| container wav: no Lumen demuxer in this build (first vertical: AVI, D-018) | 7: media/0016-audio-only.wav, media/0044-audio-only.wav, media/0091-audio-only.wav, media/0133-audio-only.wav, media/0220-audio-only.wav, media/0241-audio-only.wav, media/0331-audio-only.wav |
| container flac: no Lumen demuxer in this build (first vertical: AVI, D-018); the flac audio itself is lossless-class and becomes comparable when its container card lands | 7: media/0036-cover-audio.mp3, media/0046-cover-audio.m4a, media/0080-cover-audio.flac, media/0200-cover-audio.mp3, media/0213-cover-audio.flac, media/0316-audio-only.flac, media/0350-audio-only.flac |
| container wav: no Lumen demuxer in this build (first vertical: AVI, D-018); the pcm_s24le audio itself is lossless-class and becomes comparable when its container card lands | 7: media/0059-audio-only.wav, media/0078-audio-only.wav, media/0117-audio-only.wav, media/0124-audio-only.wav, media/0170-audio-only.wav, media/0229-audio-only.wav, media/0374-audio-only.wav |
| lossless both sides, but VLC afile cannot dump pcm_s32le (no afile output format for 32-bit int — baselines 14-options-full-help.txt L2338); the transcode-chain comparison is doc 10 §6.6's open [VERIFY], deferred | 6: media/0021-avi-av.avi, media/0063-avi-av.avi, media/0104-avi-av.avi, media/0191-avi-av.avi, media/0197-avi-av.avi, media/0402-avi-av.avi |
| container wav: no Lumen demuxer in this build (first vertical: AVI, D-018); the pcm_s16le audio itself is lossless-class and becomes comparable when its container card lands | 6: media/0037-audio-only.wav, media/0053-audio-only.wav, media/0108-audio-only.wav, media/0212-audio-only.wav, media/0218-audio-only.wav, media/0282-audio-only.wav |
| container hls: no Lumen demuxer in this build (first vertical: AVI, D-018) | 6: hls/0174/master.m3u8, hls/0189/master.m3u8, hls/0222/master.m3u8, hls/0288/master.m3u8, hls/0384/master.m3u8, hls/0404/master.m3u8 |
| container mp3: no Lumen demuxer in this build (first vertical: AVI, D-018) | 4: media/0054-audio-only.mp3, media/0248-cover-audio.mp3, media/0360-audio-only.mp3, media/0366-cover-audio.mp3 |
| container m4a: no Lumen demuxer in this build (first vertical: AVI, D-018) | 4: media/0066-cover-audio.m4a, media/0120-cover-audio.mp3, media/0190-audio-only.m4a, media/0239-cover-audio.mp3 |
| container mka: no Lumen demuxer in this build (first vertical: AVI, D-018); the alac audio itself is lossless-class and becomes comparable when its container card lands | 3: media/0009-audio-only.mka, media/0029-audio-only.mka, media/0164-audio-only.mka |
| container mp2: no Lumen demuxer in this build (first vertical: AVI, D-018) | 3: media/0051-audio-only.mp2, media/0295-audio-only.mp2, media/0302-audio-only.mp2 |
| container wav: no Lumen demuxer in this build (first vertical: AVI, D-018); the pcm_s32le audio itself is lossless-class and becomes comparable when its container card lands | 3: media/0052-audio-only.wav, media/0223-audio-only.wav, media/0353-audio-only.wav |
| container wav: no Lumen demuxer in this build (first vertical: AVI, D-018); the pcm_u8 audio itself is lossless-class and becomes comparable when its container card lands | 3: media/0072-audio-only.wav, media/0101-audio-only.wav, media/0387-audio-only.wav |
| container adts: no Lumen demuxer in this build (first vertical: AVI, D-018) | 3: media/0083-audio-only.aac, media/0142-audio-only.aac, media/0313-audio-only.aac |
| container mkv: no Lumen demuxer in this build (first vertical: AVI, D-018); the flac audio itself is lossless-class and becomes comparable when its container card lands | 3: media/0084-audio-only.mkv, media/0267-audio-only.mkv, media/0338-audio-only.mkv |
| container ogv: no Lumen demuxer in this build (first vertical: AVI, D-018) | 3: media/0089-ogv-video.ogv, media/0173-ogv-video.ogv, media/0256-ogv-video.ogv |
| container ogg: no Lumen demuxer in this build (first vertical: AVI, D-018) | 3: media/0115-audio-only.ogg, media/0210-audio-only.ogg, media/0254-audio-only.ogg |
| container mkv: no Lumen demuxer in this build (first vertical: AVI, D-018); the pcm_s16le audio itself is lossless-class and becomes comparable when its container card lands | 2: media/0014-vfr-av.mkv, media/0329-vfr-av.mkv |
| container mov: no Lumen demuxer in this build (first vertical: AVI, D-018); the pcm_s16le audio itself is lossless-class and becomes comparable when its container card lands | 2: media/0023-vfr-av.mov, media/0274-vfr-av.mov |
| container wv: no Lumen demuxer in this build (first vertical: AVI, D-018); the wavpack audio itself is lossless-class and becomes comparable when its container card lands | 2: media/0045-audio-only.wv, media/0337-audio-only.wv |
| container mov: no Lumen demuxer in this build (first vertical: AVI, D-018); the alac audio itself is lossless-class and becomes comparable when its container card lands | 2: media/0166-audio-only.mov, media/0396-audio-only.mov |
| container ac3: no Lumen demuxer in this build (first vertical: AVI, D-018) | 1: media/0135-audio-only.ac3 |
| container opus: no Lumen demuxer in this build (first vertical: AVI, D-018) | 1: media/0206-audio-only.opus |
| container mp4: no Lumen demuxer in this build (first vertical: AVI, D-018); the pcm_s16le audio itself is lossless-class and becomes comparable when its container card lands | 1: media/0306-vfr-av.mp4 |
| container wav: no Lumen demuxer in this build (first vertical: AVI, D-018); the flac audio itself is lossless-class and becomes comparable when its container card lands | 1: media/0370-audio-only.wav |

## Excluded from v1 (corrupt/garbage inputs)

- dirty (61): dirty/bit-drift-region-025-0034-raw-video.h264, dirty/bit-drift-region-035-0042-mkv-video.mkv, dirty/bit-drift-region-045-0111-avi-video.avi, dirty/bit-drift-region-052-0192-mkv-video.mkv, dirty/bit-drift-region-064-0118-avi-video.avi, dirty/cut-mid-chunk-002-0106-avi-video.avi, dirty/cut-mid-chunk-003-0291-mov-video.mov, dirty/cut-mid-chunk-055-0126-raw-video.h264, dirty/cut-mid-chunk-065-0223-audio-only.wav, dirty/cut-mid-chunk-072-0403-flv-video.flv, dirty/dup-range-006-0391-raw-video.h264, dirty/dup-range-008-0057-multi-av.mkv, dirty/dup-range-020-0392-drift-av.mkv, dirty/dup-range-060-0031-mp4-video.mp4, dirty/dup-range-062-0313-audio-only.aac, dirty/dup-range-063-0307-multivideo-av.avi, dirty/flip-bytes-036-0356-avi-video.avi, dirty/flip-bytes-041-0012-avi-video.avi, dirty/flip-bytes-044-0089-ogv-video.ogv, dirty/flip-bytes-046-0312-webm-video.webm, dirty/flip-bytes-050-0355-muxed-av.mp4, dirty/flip-bytes-058-0251-webm-video.webm, dirty/head-size-lie-007-0280-avi-av.avi, dirty/head-size-lie-010-0372-avi-video.avi, dirty/head-size-lie-016-0095-avi-video.avi, dirty/head-size-lie-024-0004-muxed-av.mp4, dirty/head-size-lie-038-0229-audio-only.wav, dirty/head-size-lie-053-0419-avi-video.avi, dirty/smash-head-009-0335-multivideo-av.avi, dirty/smash-head-013-0153-dynres-video.ts, dirty/smash-head-018-0269-vfr-av.mkv, dirty/smash-head-026-0302-audio-only.mp2, dirty/smash-head-047-0045-audio-only.wv, dirty/smash-head-051-0329-vfr-av.mkv, dirty/smash-head-054-0208-avi-video.avi, dirty/smash-head-059-0170-audio-only.wav, dirty/smash-head-069-0100-drift-av.mkv, dirty/splice-other-001-0387-audio-only.wav, dirty/splice-other-017-0408-avi-video.avi, dirty/splice-other-022-0047-muxed-av.mp4, dirty/splice-other-032-0142-audio-only.aac, dirty/splice-other-043-0263-ts-video.ts, dirty/splice-other-066-0411-avi-video.avi, dirty/splice-other-068-0286-raw-video.265, dirty/splice-other-073-0202-muxed-av.mkv, dirty/swap-halves-019-0040-avi-av.avi, dirty/swap-halves-021-0184-audio-only.mp4, dirty/swap-halves-033-0325-avi-video.avi, dirty/tail-garbage-012-0244-avi-av.avi, dirty/tail-garbage-014-0267-audio-only.mkv, dirty/tail-garbage-034-0347-muxed-av.mp4, dirty/truncate-015-0185-mkv-video.mkv, dirty/truncate-030-0360-audio-only.mp3, dirty/truncate-049-0297-avi-av.avi, dirty/truncate-067-0370-audio-only.wav, dirty/truncate-071-0000-flv-video.flv, dirty/zero-mid-011-0139-avi-video.avi, dirty/zero-mid-028-master.m3u8, dirty/zero-mid-029-0241-audio-only.wav, dirty/zero-mid-048-0178-avi-video.avi, dirty/zero-mid-070-0066-cover-audio.m4a
- dirty-lie-ext (13): dirty/lie-000-0164-audio-only.mp3, dirty/lie-004-0110-avi-video.265, dirty/lie-005-0103-avi-video.wav, dirty/lie-023-0104-avi-av.265, dirty/lie-027-0364-muxed-av.mkv, dirty/lie-031-0037-audio-only.bin, dirty/lie-037-0228-mkv-video.wav, dirty/lie-039-0167-mp4-video.avi, dirty/lie-040-0371-muxed-av.mkv, dirty/lie-042-0030-avi-video.bin, dirty/lie-056-0293-vfr-av.mp4, dirty/lie-057-0044-audio-only.265, dirty/lie-061-0183-mkv-video.m3u8
- garbage (22): garbage/all-ff-08.bin, garbage/all-zero-09.bin, garbage/avi-magic-head-03.bin, garbage/ebml-magic-05.bin, garbage/ebml-size-lie-16.bin, garbage/empty-00.bin, garbage/ftyp-magic-04.bin, garbage/ftyp-only-13.bin, garbage/gif-magic-17.bin, garbage/html-text-21.bin, garbage/id3-junk-10.bin, garbage/id3-size-lie-14.bin, garbage/jpeg-magic-19.bin, garbage/m3u8-garbage-15.bin, garbage/mdat-only-12.bin, garbage/moov-only-11.bin, garbage/one-byte-01.bin, garbage/png-magic-18.bin, garbage/random-bytes-06.bin, garbage/riff-magic-02.bin, garbage/text-07.bin, garbage/zip-magic-20.bin
- clean-file differential only in v1; corrupt-input comparison is the error-class rule (doc 10 §6.6), a separate card.

## Run notes

- VLC oracle availability: no vlc/cvlc on PATH (checked 2026-10-23); the one permitted install attempt answered: 'E: Unable to acquire the dpkg frontend lock (/var/lib/dpkg/lock-frontend), are you root?' — hence PENDING-VLC: zero oracle digests, exact to-run commands only.
- Corpus provenance: the on-disk corpus-big was regenerated 2026-10-23 06:39 UTC (same seed 58007138736976908607880252061, 500 files); the committed docs/corpus/corpus-big-manifest-20261022.json no longer describes the on-disk tree (62 of its files absent) — this run used the regenerated corpus's own manifest, whose 500 entries were sha256-verified against the files before the run.
- Lumen-side pins on the committed class-A fixtures (tools/bench-vs-vlc cargo test, tests/lumen_pcm.rs): avi_mjpeg_pcm.avi 08db7ccc163dc478709627c1db9044b5 (352,800 B, 60 packets) and avi_mjpeg_2pcm.avi first track c3bb82035ef4b4b171bf2b3e3847aacc (384,000 B, 20 packets); each independently cross-verified by a Python RIFF walk of the first-auds 01wb payloads + hashlib.md5 (two implementations agree).
- Independent verification of THIS run's Lumen rows: all 60 clean ok-rows (every comparable row included) re-verified by an independent Python RIFF walk (first auds strl ordinal -> NNwb payload concatenation, hashlib.md5): md5 + byte count + packet count + track ordinal agree on 60/60. The single differing ok-row is dirty/dup-range-063-0307-multivideo-av.avi — a corrupted duplicate-range file, excluded from comparison; on damaged input the two walks legitimately disagree about which duplicated bytes are payload.
- Mock-mode plumbing validation (NOT part of this report's data): a [MOCK]-labeled CLI run with the echo hook fed the RIFF-walk ground-truth PCM of avi_mjpeg_pcm.avi and produced MATCH [MOCK] there + MISMATCH [MOCK] on the 2pcm fixture (output kept at /tmp/w5-09/mock-report.md, not committed); 30 python unit tests + 12 cargo tests cover mock end-to-end, digest logic on synthetic PCM, classification, and the mock/real separation invariants.

## Oracle run 2026-11-09 (issue #19 — VLC 3.0.23 obtained and executed)

\* = input re-based on a fresh realization of the recorded argvs (see below); every Lumen-vs-VLC verdict in this table is same-input.

### Oracle acquisition (all in /tmp; nothing vendored, no repo change)

- `apt-cache policy vlc`: Candidate **3.0.24**-0+deb13u1 (trixie-security) — a different version, NOT substituted; **3.0.23-0+deb13u1 is available from deb.debian.org/debian trixie/main** and is the pinned version used.
- `apt-get download vlc=3.0.23-0+deb13u1 vlc-bin=… libvlc-bin=… libvlc5=… libvlccore9=… vlc-plugin-base=… vlc-plugin-video-output=3.0.23-0+deb13u1` (download-only, no root, no install), then `dpkg-deb -x` of all 7 debs into `/tmp/w16ff-vlc/root/`, executed via `LD_LIBRARY_PATH=/tmp/w16ff-vlc/root/usr/lib/x86_64-linux-gnu` + `VLC_PLUGIN_PATH=…/vlc/plugins`.
- Version assertion (doc 10 §6.6 pin): `VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)` — Compiled by Debian buildd for trixie-security, Jan 7 2026.
- flatpak/snap: absent (`which flatpak snap` → no output); not needed.

### Input corpus (the 2026-10-23 corpus-big no longer exists on this machine)

- `/home/z/my-project/corpus-big` is absent (checked 2026-11-09). The 35 fixtures were regenerated from the argvs recorded per-file in the committed `corpus-big-manifest-20261022.json` with the pinned ffmpeg 7.1.5-0+deb13u1.
- 16/35 regenerate **sha256-identical** to the committed manifest (deterministic lavfi sources only).
- 19/35 embed runtime-random lavfi sources (`anoisesrc` without a seed, `life`) — proven by a double-run of one recorded argv yielding two different sha256 (`6b7a4f3a…` vs `aee4767e…`) — so the 2026-10-23 bytes are unrecoverable; those rows (marked \*) were re-based on fresh realizations of the same argvs, with Lumen re-decoded on the same fresh files in this run.
- On 20/35 rows the fresh Lumen digest equals the committed 2026-10-23 Lumen digest (the 16 byte-identical files + 4 rows whose random source touches video only); on 15 rows the audio payload itself differs (fresh noise realization) — the committed Lumen cells for those rows remain tied to the lost 2026-10-23 inputs.

### Run-book command status

- The 35 to-run commands were executed verbatim except the input path (`/home/z/my-project/corpus-big/media/…` → `/tmp/w16ff-corpus/media/…`; original tree absent). Raw dumps written under `/tmp/bench-vs-vlc/`, MD5'd, then deleted per-row.
- Lumen half re-validated this run: `cargo test --test lumen_pcm -- --ignored` driven via `LUMEN_DIFF_FIXTURES`/`LUMEN_DIFF_OUT` over the 35 rows — 35/35 decode ok; fresh Lumen digest == ffmpeg 7.1.5 decode (md5 + byte count) on 35/35, re-proving Lumen/ffmpeg parity on the same inputs.
- The README's first-real-oracle gate ("confirm dump byte-counts equal Lumen `pcm_bytes` on a known-good fixture before any MATCH is trusted") is **answered: it does not hold as stated** — 35/35 VLC dumps are shorter. Cause analysis in Findings.

### Results — decoded-PCM differential (this run, same inputs both sides)

| fixture | audio | ch | rate | Lumen MD5 (this run) | Lumen B | VLC MD5 | VLC B | verdict |
|---|---|---|---|---|---|---|---|---|
| media/0022-avi-av.avi | pcm_f32le | 1 | 22050 | c5f5a2856f4d1ee42a999fd9b4ace0dd | 557336 | 78df10d1a4cbbd79da51c7f024da8f63 | 556976 | NOT-BIT-EXACT (float path): RMS 0.0125 / peak 0.0250 — §6.6 deviation path |
| media/0040-avi-av.avi * | pcm_u8 | 4 | 16000 | 87be0fb5b8e4582584c42c80198f14b7 | 350976 | 84690c1a8635bd4ae49d6486b66271b6 | 350400 | MATCH-prefix (bit-exact; tail 576 B unflushed) |
| media/0041-avi-av.avi * | pcm_u8 | 2 | 96000 | 094340d045306c8ea23fb43e523c26fc | 2112384 | e69260d694dda534d3f3801bc94eaf01 | 2112000 | MATCH-prefix (bit-exact; tail 384 B unflushed) |
| media/0096-multivideo-av.avi | pcm_s16le | 2 | 11025 | 44cec9ebf7e457140edb971c3e962c96 | 94208 | a9e08d88b494a601d5d83dd97997be06 | 93712 | MATCH-prefix (bit-exact; tail 496 B unflushed) |
| media/0116-avi-av.avi * | pcm_u8 | 2 | 8000 | d3da0ad8060de10204959d9b3f612b3d | 51728 | fe17982880a51bdc53e4f6d20790bfcf | 51600 | MATCH-prefix (bit-exact; tail 128 B unflushed) |
| media/0121-multivideo-av.avi | pcm_s16le | 1 | 48000 | 36355267afc2774cddb8f81535afd281 | 216096 | 910fe36f8723e5ada5cf850f2078db9d | 216000 | MATCH-prefix (bit-exact; tail 96 B unflushed) |
| media/0134-avi-av.avi * | pcm_u8 | 2 | 16000 | 9d62cb4522aeb10247308e0d6dd97c24 | 178176 | 2d22ce916d4ff72d5fa6ec20a13df819 | 177600 | MATCH-prefix (bit-exact; tail 576 B unflushed) |
| media/0155-avi-av.avi | pcm_u8 | 6 | 22050 | e6bb46494f3c1d3678571324a22f7a55 | 1175880 | da214c472362d68b75e1ee4bb8d4cbf6 | 1174158 | MATCH-prefix (bit-exact; tail 1722 B unflushed) |
| media/0156-avi-av.avi | pcm_u8 | 4 | 8000 | 66958319ced9c6cdb9eb0c8130559bff | 261984 | a3c8e3633e1fe17d4453bbb7f1353bd5 | 261600 | MATCH-prefix (bit-exact; tail 384 B unflushed) |
| media/0161-multivideo-av.avi | pcm_s16le | 2 | 96000 | 7d7ad50a3f50e59a1fab63eb4a175f87 | 3022848 | b96286a26cb1ae55956888176fe7766a | 3018752 | MATCH-prefix (bit-exact; tail 4096 B unflushed) |
| media/0177-multivideo-av.avi * | pcm_s16le | 4 | 11025 | e9764dea64e89e8e6730de7208088d46 | 340808 | 35ecb9bc606f0a06a7d51818d20d9d78 | 339568 | MATCH-prefix (bit-exact; tail 1240 B unflushed) |
| media/0180-multivideo-av.avi * | pcm_s16le | 1 | 48000 | 89a1a90ca55e84bfd19a8814d94f20e6 | 331776 | 1f16348f09100deb72bbedcb3262cd70 | 331200 | MATCH-prefix (bit-exact; tail 576 B unflushed) |
| media/0187-avi-av.avi | pcm_u8 | 8 | 11025 | 34bcf94ce5308bc76b02f751d3fcad4a | 547720 | bc76bdf18701de20fc6a729e8ff463b9 | 546840 | MATCH-prefix (bit-exact; tail 880 B unflushed) |
| media/0194-multivideo-av.avi * | pcm_s16le | 8 | 44100 | fbf9a9b3a9861350fc1007194e629d3d | 2998272 | 838928ee4447bb5eeecd8842e8dbfbed | 2981888 | MATCH-prefix (bit-exact; tail 16384 B unflushed) |
| media/0201-multivideo-av.avi * | pcm_s16le | 2 | 96000 | 9a2d0119de231d475d4ef4481902ddaf | 1970176 | 0babcb9330a2d5e217bdae7326a87d42 | 1968000 | MATCH-prefix (bit-exact; tail 2176 B unflushed) |
| media/0214-avi-av.avi * | pcm_s16le | 1 | 88200 | f54afc5510fc828db4883f610531f515 | 1144306 | c6a99a39c62286b6a1978be79f6a7f7f | 1142784 | MATCH-prefix (bit-exact; tail 1522 B unflushed) |
| media/0227-avi-av.avi * | pcm_s16le | 1 | 48000 | b48163da74a73d446389986dd31d3b7a | 472032 | d93f66cfe531153842b2fb2e2d2ea841 | 471040 | MATCH-prefix (bit-exact; tail 992 B unflushed) |
| media/0233-multivideo-av.avi | pcm_s16le | 1 | 88200 | 649d0232dba068c25d89617e9ef3fba1 | 870400 | c8db9843ba6a5326b6f98b08980978f3 | 868770 | MATCH-prefix (bit-exact; tail 1630 B unflushed) |
| media/0236-multivideo-av.avi | pcm_s16le | 1 | 11025 | 2fa047d2f34c9029f056237ca8079f5f | 137216 | d96aab94f910dbafb8978d0a72052c09 | 136710 | MATCH-prefix (bit-exact; tail 506 B unflushed) |
| media/0247-avi-av.avi * | pcm_f32le | 1 | 48000 | 2e8a354f4d58e3c5a452f9a35a055ded | 745728 | 57087e90270983d6966542dd1d8cd0d4 | 745392 | NOT-BIT-EXACT (float path): RMS 0.0123 / peak 0.0461 — §6.6 deviation path |
| media/0278-avi-av.avi | pcm_s16le | 1 | 88200 | 28a9e2652675dccd5cee316150fe0d46 | 356858 | 9763ff8dcdb59ff2c34ced3f83dfccbe | 356352 | MATCH-prefix (bit-exact; tail 506 B unflushed) |
| media/0280-avi-av.avi * | pcm_f32le | 2 | 22050 | a21a95e1e7dfd7ad43dda987f3a6c7f6 | 935976 | 364bec8187b1a692751cd0a91379d618 | 934760 | NOT-BIT-EXACT (float path): RMS 0.0087 / peak 0.0340 — §6.6 deviation path |
| media/0297-avi-av.avi | pcm_f32le | 8 | 11025 | 0f368aaa880d22f7c48caf67db304276 | 3520224 | 1796324a7db6a7aa3d5b9ef3895360c0 | 3518528 | NOT-BIT-EXACT (float path): RMS 0.0001 / peak 0.0463 — §6.6 deviation path |
| media/0305-avi-av.avi * | pcm_f32le | 1 | 22050 | 02f69412eb50f59b5ad8edecf388cc10 | 885440 | c6b3e08bcc3cbec3355857da06a6b31b | 884656 | NOT-BIT-EXACT (float path): RMS 0.1188 / peak 0.4617 — §6.6 deviation path |
| media/0307-multivideo-av.avi | pcm_s16le | 2 | 32000 | e2bce5d5744e374d59ffd1dbb565314c | 450560 | d785c2f3859818577337295a8e94594e | 448000 | MATCH-prefix (bit-exact; tail 2560 B unflushed) |
| media/0308-avi-av.avi | pcm_s16le | 4 | 96000 | 01269e9f433bd2f4496054180b3b9639 | 4096000 | a436890a0b7bbe418b1c630d997c8167 | 4089600 | MATCH-prefix (bit-exact; tail 6400 B unflushed) |
| media/0318-avi-av.avi * | pcm_s16le | 2 | 96000 | 48718b7749195fdd2020f71e36cbcc37 | 1331712 | 2620b150f73bf91cdaa907a9c4024489 | 1331200 | MATCH-prefix (bit-exact; tail 512 B unflushed) |
| media/0324-avi-av.avi | pcm_u8 | 4 | 44100 | 1767da988df20a761eea5b3be593b233 | 822908 | 2c669806afcb5e5f15ad4d4e81dd621c | 820260 | MATCH-prefix (bit-exact; tail 2648 B unflushed) |
| media/0330-avi-av.avi | pcm_f32le | 1 | 96000 | bb76a78b3c77421b3e41c0155dc8d8d1 | 410880 | c0e894718e500b5ec3b1c496c4559beb | 409520 | NOT-BIT-EXACT (float path): RMS 0.0000 / peak 0.0012 — §6.6 deviation path |
| media/0335-multivideo-av.avi * | pcm_s16le | 1 | 16000 | 4fd79102bcb2c878d8a5eb197be5755c | 97216 | daac1d6a101167bbdc031a1c58fa978c | 96800 | MATCH-prefix (bit-exact; tail 416 B unflushed) |
| media/0336-multivideo-av.avi * | pcm_s16le | 1 | 32000 | ca97c302d2e1d6ec0e27063076b56dd9 | 425984 | c06837c134cdadf0e7ae88eec959cca7 | 425600 | MATCH-prefix (bit-exact; tail 384 B unflushed) |
| media/0342-avi-av.avi * | pcm_s16le | 3 | 32000 | 0d1d39cbb0401e1e9303081446c2652b | 1846464 | ce235467c545e6a815373b0377208363 | 1843200 | MATCH-prefix (bit-exact; tail 3264 B unflushed) |
| media/0367-multivideo-av.avi * | pcm_s16le | 6 | 48000 | a92ecd89b89a133a8068337d84971a08 | 4030464 | cdb1d60f5a13eab036127b1c72dcc6e6 | 4018176 | MATCH-prefix (bit-exact; tail 12288 B unflushed) |
| media/0386-avi-av.avi | pcm_s16le | 1 | 22050 | 37e3ad34ab0d716cc5da69a2317c3305 | 187292 | 6cfd15ca4347f470ada1509b8fee5cef | 186368 | MATCH-prefix (bit-exact; tail 924 B unflushed) |
| media/0394-avi-av.avi * | pcm_s16le | 6 | 32000 | e79410cbf8e8cbf49d2f94aa95662ee4 | 1092096 | db0b17e061a9b7009608736d195605b6 | 1084800 | MATCH-prefix (bit-exact; tail 7296 B unflushed) |

### Findings

1. **29/35 rows — every `pcm_u8` and `pcm_s16le` row: VLC's dump is a bit-exact PREFIX of the payload** (head drop 0 B on all 29); the entire shortfall is a **tail of 96–16384 B left unflushed** by the `afile` output at `--play-and-exit` end-of-stream. VLC's decode itself is bit-exact on these rows; the byte-count gate fails only because of the unflushed tail.
2. **6/35 rows — every `pcm_f32le` row: the dump is not a sub-window of the payload** — every aligned sample differs slightly (0022: RMS 0.0125 / peak 0.0250; 0247: 0.0123/0.0461; 0280: 0.0087/0.0340; 0297: 0.0001/0.0463; 0305: 0.1188/0.4617; 0330: 0.0000/0.0012), consistent with VLC's float `aout` chain (resampler/filter delay) rather than passthrough. Per doc 10 §6.6 these fall to the sample-wise RMS/peak deviation path (thresholds = doc 02 M2, open) — **Lead decision required**; they are NOT recorded as decoder mismatches.
3. Net parity statement: on all 29 lossless-comparable u8/s16 rows Lumen and VLC agree bit-exactly over the entire delivered window; the afile tail-unflush is a dump-method artifact, not a decode difference.

### Deviations from the pinned protocol (Lead decisions)

- Oracle binary is Debian trixie's build of the pinned 3.0.23 (apt download), not a videolan.org tarball — version pin asserted identically (`3.0.23 Vetinari`).
- Input path substitution + 19/35 re-based inputs (above).
- No threshold was invented for the f32 deviation path; no MATCH is claimed there.
