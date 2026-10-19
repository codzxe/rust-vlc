#!/bin/bash
# Probe runner — executes the pinned VLC 3.0.23 oracle headless on synthetic fixtures.
# Clean-room note: fixtures are OUR OWN generated files (python struct-writers + ffmpeg
# lavfi synthesis). Oracle behavior observed via -vv logs. No VLC source consulted.
EXTRACT=/home/z/my-project/tmp/pinning/extract
export LD_LIBRARY_PATH=$EXTRACT/usr/lib/x86_64-linux-gnu
P=/home/z/my-project/tmp/probe
VLC="$EXTRACT/usr/bin/vlc -I dummy --play-and-exit --aout dummy --vout dummy -vv"
rm -f $P/logs/status.txt
run() { # run <name> <args...>
  local name=$1; shift
  timeout 25 $VLC "$@" > $P/logs/$name.log 2>&1
  echo "$name exit=$?" >> $P/logs/status.txt
}
# --- containers + codec payloads ---
for f in wav_pcm_py.wav avi_mjpeg.avi mp4_h264_aac.mp4 mkv_h264_flac.mkv \
         webm_vp9_opus.webm ogg_vorbis.ogg ogg_opus.opus flac_native.flac \
         mp3_raw.mp3 adts_aac.aac mpegts_h264.ts mpegps_mpeg2.mpg \
         asf_wmv2_wmav2.wmv aiff_pcm.aiff h264_annexb.264 hevc_annexb.265 \
         mp4_hevc.mp4 mkv_av1_opus.mkv ogg_theora.ogg avi_mpeg4.avi \
         m4a_alac.m4a ac3_raw.ac3 eac3_raw.eac3 dts_raw.dts ogg_speex.spx \
         mpeg2_es.m2v wav_adpcm_ima.wav au_pcm.au midi_minimal.mid \
         tone_zipped.zip; do
  run "${f%%.*}" "$P/media/$f"
done
# --- subtitles (standalone open) ---
for f in srt_sample.srt ssa_sample.ssa ass_sample.ass vtt_sample.vtt \
         microdvd_sample.sub subviewer_sample.sub sami_sample.smi ttml_sample.ttml; do
  run "sub_${f%%.*}" "$P/subs/$f"
done
# --- playlists ---
for f in m3u_sample.m3u pls_sample.pls xspf_sample.xspf asx_sample.asx b4s_sample.b4s cue_sample.cue; do
  run "pl_${f%%.*}" "$P/pls/$f"
done
# --- directory access ---
run dir_access "$P/dir"
cat $P/logs/status.txt
