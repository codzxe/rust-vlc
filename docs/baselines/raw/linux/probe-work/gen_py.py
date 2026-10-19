#!/usr/bin/env python3
"""Generate pure-python synthetic probe fixtures (no external tools).
Each file is minimal-but-valid per its public specification."""
import math, struct, os

M = "/home/z/my-project/tmp/probe"

# --- 1. WAV: canonical RIFF WAVE, PCM s16le, 0.5 s @ 8 kHz mono ---
sr, n = 8000, 4000
frames = b"".join(struct.pack("<h", int(12000 * math.sin(2 * math.pi * 440 * i / sr))) for i in range(n))
data = b"WAVE" + b"fmt " + struct.pack("<IHHIIHH", 16, 1, 1, sr, sr * 2, 2, 16) \
     + b"data" + struct.pack("<I", len(frames)) + frames
with open(f"{M}/media/wav_pcm_py.wav", "wb") as f:
    f.write(b"RIFF" + struct.pack("<I", len(data)) + data)

# --- 2. M3U playlist pointing at the python WAV (relative path, same run cwd resolves) ---
with open(f"{M}/pls/m3u_sample.m3u", "w") as f:
    f.write("#EXTM3U\n#EXTINF:1,Lumen probe tone\n../media/wav_pcm_py.wav\n")

# --- 3. PLS ---
with open(f"{M}/pls/pls_sample.pls", "w") as f:
    f.write("[playlist]\nFile1=../media/wav_pcm_py.wav\nTitle1=Lumen probe tone\nLength1=1\nNumberOfEntries=1\nVersion=2\n")

# --- 4. XSPF ---
with open(f"{M}/pls/xspf_sample.xspf", "w") as f:
    f.write('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<playlist version="1" xmlns="http://xspf.org/ns/0/">\n'
            '  <title>Lumen probe</title>\n  <trackList>\n    <track>\n'
            '      <location>' + f'file://{M}/media/wav_pcm_py.wav' + '</location>\n'
            '      <title>probe tone</title>\n    </track>\n  </trackList>\n</playlist>\n')

# --- 5. ASX ---
with open(f"{M}/pls/asx_sample.asx", "w") as f:
    f.write('<asx version="3.0">\n  <title>Lumen probe</title>\n  <entry>\n'
            f'    <ref href="{M}/media/wav_pcm_py.wav" />\n'
            '    <title>probe tone</title>\n  </entry>\n</asx>\n')

# --- 6. B4S (Winamp SHOUTcast XML playlist) ---
with open(f"{M}/pls/b4s_sample.b4s", "w") as f:
    f.write('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<WinampXML>\n  <playlist>\n    <entry Playstring="' + f'{M}/media/wav_pcm_py.wav' + '">\n'
            '      <Name>probe tone</Name>\n      <Length>1000</Length>\n    </entry>\n  </playlist>\n</WinampXML>\n')

# --- 7. CUE sheet (negative probe: no cue module in pinned census) ---
with open(f"{M}/pls/cue_sample.cue", "w") as f:
    f.write('FILE "wav_pcm_py.wav" WAVE\n  TRACK 01 AUDIO\n    TITLE "Probe Tone"\n    INDEX 01 00:00:00\n')

# --- 8. SRT ---
with open(f"{M}/subs/srt_sample.srt", "w") as f:
    f.write("1\n00:00:00,000 --> 00:00:02,000\nLumen probe subtitle\n\n"
            "2\n00:00:02,000 --> 00:00:04,000\nSecond cue, accents: ä ö ü\n")

# --- 9. SSA (SubStation Alpha v4) ---
with open(f"{M}/subs/ssa_sample.ssa", "w") as f:
    f.write("[Script Info]\nScriptType: v4.00\nPlayResX: 384\nPlayResY: 288\n\n"
            "[V4 Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, "
            "TertiaryColour, BackColour, Bold, Italic, BorderStyle, Outline, Shadow, Alignment, "
            "MarginL, MarginR, MarginV, AlphaLevel, Encoding\n"
            "Style: Default,Arial,16,0,0,0,0,0,0,1,1,0,2,10,10,10,0,1\n\n"
            "[Events]\nFormat: Marked, Start, End, Style, Text\n"
            "Dialogue: Marked=0,0:00:00.00,0:00:02.00,Default,Lumen SSA probe,,0,0,0\n")

# --- 10. ASS (v4+) with an override tag ---
with open(f"{M}/subs/ass_sample.ass", "w") as f:
    f.write("[Script Info]\nScriptType: v4.00+\nPlayResX: 384\nPlayResY: 288\n\n"
            "[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, "
            "OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, "
            "Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n"
            "Style: Default,Arial,16,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,1,0,2,10,10,10,1\n\n"
            "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n"
            "Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,{\\b1}Lumen{\\b0} ASS probe\n")

# --- 11. WebVTT ---
with open(f"{M}/subs/vtt_sample.vtt", "w") as f:
    f.write("WEBVTT\n\n00:00:00.000 --> 00:00:02.000\nLumen WebVTT probe\n\n"
            "00:00:02.000 --> 00:00:04.000\nSecond cue\n")

# --- 12. MicroDVD (frame-based) ---
with open(f"{M}/subs/microdvd_sample.sub", "w") as f:
    f.write("{0}{25}Lumen MicroDVD probe\n{50}{100}Second cue\n")

# --- 13. SubViewer 2.0 ---
with open(f"{M}/subs/subviewer_sample.sub", "w") as f:
    f.write("[INFORMATION]\n[TITLE]Lumen probe\n[END INFORMATION]\n\n"
            "00:00:00.00,00:00:02.00\nLumen SubViewer probe\n\n00:00:02.00,00:00:04.00\nSecond cue\n")

# --- 14. SAMI ---
with open(f"{M}/subs/sami_sample.smi", "w") as f:
    f.write("<SAMI>\n<HEAD><TITLE>Lumen probe</TITLE></HEAD>\n<BODY>\n"
            "<SYNC Start=0><P Class=CC>Lumen SAMI probe\n"
            "<SYNC Start=2000><P Class=CC>Second cue\n"
            "</BODY>\n</SAMI>\n")

# --- 15. TTML ---
with open(f"{M}/subs/ttml_sample.ttml", "w") as f:
    f.write('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<tt xmlns="http://www.w3.org/ns/ttml" xml:lang="en">\n'
            '  <body>\n    <div>\n'
            '      <p begin="0s" end="2s">Lumen TTML probe</p>\n'
            '      <p begin="2s" end="4s">Second cue</p>\n'
            '    </div>\n  </body>\n</tt>\n')

# --- 16. dir-probe: directory access ---
os.makedirs(f"{M}/dir", exist_ok=True)
with open(f"{M}/dir/tone.wav", "wb") as f:
    f.write(open(f"{M}/media/wav_pcm_py.wav", "rb").read())

print("python fixtures written:", sorted(os.listdir(f"{M}/media")) + sorted(os.listdir(f"{M}/subs")) + sorted(os.listdir(f"{M}/pls")))
