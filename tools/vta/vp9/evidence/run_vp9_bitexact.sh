#!/usr/bin/env bash
# run_vp9_bitexact.sh — re-run the rusty_vp9 0.1.3 admission record §D
# six-lane bit-exactness differential over the committed fixtures (FX8-A06,
# FIX-PLAN §4 F2.4: "the repo's strongest paperwork (VTA vp9 bit-exact
# verdicts) becomes reproducible").
#
# The six lanes below are the record's §D table verbatim
# (docs/dependencies/records/rusty_vp9-0.1.3.md §D): dimensions, coded
# frame counts and the ffmpeg libvpx-vp9 reference stream MD5s. The
# fixtures are the byte-identical staged lanes committed under
# crates/codecs/tests/fixtures/vp9/ (see crates/codecs/tests/
# vp9_differential.rs, which pins the same constants for the wired suite).
#
# Modes:
#   --ref-only  the reference half only: ffprobe facts + ffmpeg
#               libvpx-vp9 rawvideo stream MD5s vs the record's §D table.
#               Needs ffmpeg + ffprobe only — runnable on any box.
#   (default)   the full differential: reference half + the rusty_vp9 0.1.3
#               mine half (cargo run --release of tools/vta/vp9) + per-frame
#               MD5 compare via evidence/compare_frames.py. Needs cargo AND
#               python3 in addition to ffmpeg/ffprobe; when cargo is not on
#               PATH the script exits 3 with a [VERIFY-central] marker —
#               it NEVER pretends the mine half ran.
#
# Exit codes:
#   0  all lanes green (ref-only: 6/6 reference facts + stream MD5s match
#      the record; full: additionally 6/6 mine-vs-ref per-frame bit-exact)
#   1  a lane is red (facts mismatch, reference MD5 mismatch, or a
#      mine-vs-ref frame difference) — the tally names them
#   2  tooling failure (missing ffmpeg/ffprobe/md5sum, missing fixture,
#      usage error)
#   3  mine half not runnable here (cargo or python3 not on PATH) —
#      [VERIFY-central]: re-run on the orchestrator's central cargo box
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "${script_dir}/../../../.." && pwd)"
fixdir="${root}/crates/codecs/tests/fixtures/vp9"

# --- the record §D table (verbatim; do not edit without the record) --------
lane_ids=(A B C D E F)
lane_fix=(
    testsrc2_320x240_125f
    testsrc2_640x480_g30_90f
    testsrc2_odd_126x94_75f
    corpus_reenc_vp8to9_480x360_41f
    webm_remux_320x240_50f
    testsrc2_320x240_10bit_50f
)
lane_frames=(125 90 75 41 50 50)
lane_size=(320x240 640x480 126x94 480x360 320x240 320x240)
lane_fmt=(yuv420p yuv420p yuv420p yuv420p yuv420p yuv420p10le)
lane_bps=(1 1 1 1 1 2)
lane_md5=(
    7b5cbbc70b04132c8c74fcd07576fd48
    356c84a819e60fcfc3bc73048de20c70
    5d879407bc72e7ea105bc214c64c2256
    9621dcf981bfc3b52efa40c66c66a543
    88bb0b3ce56155bf0edb85bce4374ef4
    2391b4241617d81cf05d50efd54781d4
)

usage() {
    cat >&2 <<'USAGE'
usage: run_vp9_bitexact.sh [--ref-only]
  --ref-only  reference half only: ffprobe facts + ffmpeg libvpx-vp9 stream
              MD5s vs the record §D table (ffmpeg/ffprobe only)
  (default)   full differential: + the rusty_vp9 mine half (needs cargo;
              exits 3 [VERIFY-central] when cargo is not on PATH)
USAGE
}

mode="full"
if [ $# -gt 1 ]; then
    usage
    exit 2
elif [ $# -eq 1 ]; then
    if [ "$1" != "--ref-only" ]; then
        usage
        exit 2
    fi
    mode="ref-only"
fi

for tool in ffmpeg ffprobe md5sum; do
    if ! command -v "$tool" > /dev/null 2>&1; then
        echo "run_vp9_bitexact: FAIL — $tool not on PATH (the differential's oracle tooling)" >&2
        exit 2
    fi
done

if [ "$mode" = "full" ]; then
    if ! command -v cargo > /dev/null 2>&1; then
        echo "[VERIFY-central] run_vp9_bitexact: mine half NOT RUN — cargo is not on this PATH." >&2
        echo "[VERIFY-central] the reference half below was skipped too: re-run this script on the" >&2
        echo "[VERIFY-central] orchestrator's central cargo box for the full §D differential:" >&2
        echo "[VERIFY-central]   bash tools/vta/vp9/evidence/run_vp9_bitexact.sh" >&2
        exit 3
    fi
    if ! command -v python3 > /dev/null 2>&1; then
        echo "[VERIFY-central] run_vp9_bitexact: mine half NOT RUN — python3 (evidence/compare_frames.py) is not on this PATH." >&2
        echo "[VERIFY-central] re-run on the orchestrator's central box: bash tools/vta/vp9/evidence/run_vp9_bitexact.sh" >&2
        exit 3
    fi
fi

ffmpeg_version="$(ffmpeg -version | head -1)"
work="$(mktemp -d "${TMPDIR:-/tmp}/vta-vp9-bitexact.XXXXXX")"
trap "rm -rf '${work}'" EXIT

echo "run_vp9_bitexact: mode=${mode} — record §D six-lane differential over committed fixtures (${fixdir})"
echo "run_vp9_bitexact: oracle: ${ffmpeg_version}"

ref_green=0
mine_green=0

# --- the reference half (always runs) --------------------------------------
for i in "${!lane_ids[@]}"; do
    id="${lane_ids[$i]}"
    name="${lane_fix[$i]}"
    want_frames="${lane_frames[$i]}"
    size="${lane_size[$i]}"
    fmt="${lane_fmt[$i]}"
    fixture="${fixdir}/${name}.ivf"
    echo "lane ${id} ${name}: ${size} ${fmt}, record: ${want_frames} frames, md5 ${lane_md5[$i]}"
    if [ ! -f "${fixture}" ]; then
        echo "lane ${id}: RED — committed fixture missing: ${fixture}" >&2
        continue
    fi

    if ! facts="$(ffprobe -v error -count_frames -select_streams v:0 \
        -show_entries stream=codec_name,width,height,nb_read_frames \
        -of csv=p=0 "${fixture}")"; then
        echo "lane ${id}: RED — ffprobe failed on ${fixture}" >&2
        continue
    fi
    codec="${facts%%,*}"
    rest="${facts#*,}"
    fw="${rest%%,*}"
    rest="${rest#*,}"
    fh="${rest%%,*}"
    nb="${rest##*,}"
    want_w="${size%x*}"
    want_h="${size#*x}"
    if [ "${codec}" != "vp9" ] || [ "${fw}" != "${want_w}" ] || \
       [ "${fh}" != "${want_h}" ] || [ "${nb}" != "${want_frames}" ]; then
        echo "lane ${id}: RED — ffprobe says ${facts}, record says vp9,${want_w},${want_h},${want_frames}" >&2
        continue
    fi

    ref_raw="${work}/${name}.ref.raw"
    if ! ffmpeg -hide_banner -loglevel error -y -c:v libvpx-vp9 -i "${fixture}" \
        -f rawvideo -pix_fmt "${fmt}" "${ref_raw}"; then
        echo "lane ${id}: RED — ffmpeg libvpx-vp9 reference decode failed" >&2
        continue
    fi
    got="$(md5sum "${ref_raw}" | cut -d' ' -f1)"
    if [ "${got}" != "${lane_md5[$i]}" ]; then
        echo "lane ${id}: RED — reference stream MD5 ${got} != record ${lane_md5[$i]} (ffmpeg ${fmt} decode drifted)" >&2
        continue
    fi
    echo "lane ${id}: reference PASS — ffprobe ${facts} == record; stream MD5 ${got} == record"
    ref_green=$((ref_green + 1))
done

echo "reference half: ${ref_green}/6 lanes match the record §D table (oracle: ${ffmpeg_version})"
if [ "${mode}" = "ref-only" ]; then
    if [ "${ref_green}" -ne 6 ]; then
        echo "run_vp9_bitexact: REF-ONLY RED — ${ref_green}/6 reference lanes green" >&2
        exit 1
    fi
    echo "run_vp9_bitexact: REF-ONLY GREEN — 6/6 lanes; the reference half of the record's verdict is reproduced"
    exit 0
fi

# --- the mine half (the rusty_vp9 0.1.3 decode through our ivf demuxer) ---
for i in "${!lane_ids[@]}"; do
    id="${lane_ids[$i]}"
    name="${lane_fix[$i]}"
    fixture="${fixdir}/${name}.ivf"
    mine_raw="${work}/${name}.mine.raw"
    ref_raw="${work}/${name}.ref.raw"
    if [ ! -f "${ref_raw}" ]; then
        continue
    fi
    if ! cargo run --release --manifest-path "${root}/tools/vta/vp9/Cargo.toml" \
        -- "${fixture}" --out "${mine_raw}"; then
        echo "lane ${id}: RED — cargo run of the vta-vp9 harness failed on ${name}" >&2
        continue
    fi
    if python3 "${script_dir}/compare_frames.py" "${mine_raw}" "${ref_raw}" \
        --frames "${lane_frames[$i]}" --size "${lane_size[$i]}" --bps "${lane_bps[$i]}"; then
        mine_green=$((mine_green + 1))
    else
        echo "lane ${id}: RED — mine-vs-ref compare failed (${name})" >&2
    fi
done

echo "full differential: reference ${ref_green}/6, mine-vs-ref bit-exact ${mine_green}/6 lanes"
if [ "${ref_green}" -ne 6 ] || [ "${mine_green}" -ne 6 ]; then
    echo "run_vp9_bitexact: RED — ${mine_green}/6 bit-exact lanes (reference ${ref_green}/6)" >&2
    exit 1
fi
echo "run_vp9_bitexact: GREEN — 6/6 lanes bit-exact (mine == ref == record §D)"
exit 0
