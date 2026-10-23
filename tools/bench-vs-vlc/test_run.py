#!/usr/bin/env python3
"""Unit tests for the bench-vs-vlc harness (card 3.E3 acceptance).

Covers, per the card:
* mock-decoder mode fully tested (end to end through the oracle
  subprocess seam, agreement AND mismatch paths, determinism);
* digest comparison logic tested on synthetic PCM (byte streams with
  known divergences);
* classification of manifest entries (comparable / non-comparable /
  excluded, with the recorded reasons);
* the mock/real separation: VlcOracle refuses to run without a binary
  and never falls back to the mock; version drift aborts; mock reports
  carry the [MOCK] labels; pending reports carry PENDING-VLC and zero
  oracle digests;
* the exact PENDING-VLC command shape.

Run: python3 -m unittest discover -s tools/bench-vs-vlc -v
(or: python3 tools/bench-vs-vlc/test_run.py)
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

TOOL_ROOT = Path(__file__).resolve().parent
RUNPY = TOOL_ROOT / "run.py"


def load_run_module():
    """Import run.py as a module (its directory has a dash)."""
    spec = importlib.util.spec_from_file_location("bench_vs_vlc_run", RUNPY)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class HarnessTestCase(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="bench-vs-vlc-test-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.module = load_run_module()


class TestClassification(HarnessTestCase):
    def entry(self, **params):
        return {"rel": "media/x.avi", "class": params.pop("_class", "avi-av"),
                "params": params}

    def test_comparable_families(self):
        for codec, fmt in (("pcm_s16le", "s16"), ("pcm_u8", "u8"),
                           ("pcm_f32le", "float32")):
            verdict = self.module.classify_entry(
                self.entry(_class="avi-av", container="avi", codec="mjpeg",
                           audio_codec=codec)
            )
            self.assertEqual(verdict["status"], "comparable", codec)
            self.assertEqual(verdict["afile_format"], fmt)

    def test_non_comparable_undumpable_pcm(self):
        for codec in ("pcm_s24le", "pcm_s32le", "pcm_f64le"):
            verdict = self.module.classify_entry(
                self.entry(container="avi", codec="mjpeg", audio_codec=codec)
            )
            self.assertEqual(verdict["status"], "non_comparable", codec)
            self.assertIn("afile cannot dump", verdict["reason"])

    def test_video_codec_does_not_gate_audio_comparison(self):
        # The comparison is over the first AUDIO track's decoded PCM;
        # the demuxer filters unsupported video and still walks the
        # audio (corpus-verified: mpeg4-video AVIs with PCM audio
        # digest fine).
        verdict = self.module.classify_entry(
            self.entry(container="avi", codec="mpeg4", audio_codec="pcm_s16le")
        )
        self.assertEqual(verdict["status"], "comparable")
        self.assertEqual(verdict["afile_format"], "s16")

    def test_non_comparable_video_without_audio(self):
        verdict = self.module.classify_entry(
            self.entry(_class="avi-video", container="avi", codec="mpeg4")
        )
        self.assertEqual(verdict["status"], "non_comparable")
        self.assertIn("no audio track", verdict["reason"])

    def test_non_comparable_no_audio(self):
        verdict = self.module.classify_entry(
            self.entry(_class="avi-video", container="avi", codec="mjpeg")
        )
        self.assertEqual(verdict["status"], "non_comparable")
        self.assertIn("no audio track", verdict["reason"])

    def test_non_comparable_lossy(self):
        verdict = self.module.classify_entry(
            self.entry(container="avi", codec="mjpeg", audio_codec="mp3")
        )
        self.assertEqual(verdict["status"], "non_comparable")
        self.assertIn("legitimately differ", verdict["reason"])

    def test_non_comparable_other_container_lossless_audio(self):
        verdict = self.module.classify_entry(
            self.entry(_class="audio-only", container="wav", codec="pcm_s16le",
                       **{"class": "audio"})
        )
        self.assertEqual(verdict["status"], "non_comparable")
        self.assertIn("no Lumen demuxer", verdict["reason"])
        self.assertIn("lossless-class", verdict["reason"])

    def test_non_comparable_be_wire(self):
        verdict = self.module.classify_entry(
            self.entry(container="avi", codec="mjpeg", audio_codec="pcm_s16be")
        )
        self.assertEqual(verdict["status"], "non_comparable")
        self.assertIn("no pinned afile format", verdict["reason"])

    def test_excluded_corrupt(self):
        for klass in ("dirty", "dirty-lie-ext", "garbage"):
            verdict = self.module.classify_entry(
                self.entry(_class=klass, container="avi", codec="mjpeg")
            )
            self.assertEqual(verdict["status"], "excluded", klass)
            self.assertIn("error-class rule", verdict["reason"])


class TestSyntheticPcmComparison(HarnessTestCase):
    """The digest comparison logic on synthetic PCM byte streams."""

    def test_identical_streams_match(self):
        left = bytes(range(256)) * 40
        result = self.module.compare_pcm_streams(left, left)
        self.assertEqual(result["verdict"], "MATCH")
        self.assertNotIn("first_divergence", result)

    def test_single_byte_difference_is_found(self):
        left = bytearray(bytes(range(256)) * 40)
        right = bytearray(left)
        right[777] ^= 0x01
        result = self.module.compare_pcm_streams(bytes(left), bytes(right))
        self.assertEqual(result["verdict"], "MISMATCH")
        self.assertEqual(result["first_divergence"]["offset"], 777)
        self.assertEqual(result["first_divergence"]["left_byte"], left[777])
        self.assertEqual(result["first_divergence"]["right_byte"], right[777])

    def test_prefix_truncation_reports_offset(self):
        left = b"\x01\x02\x03\x04"
        right = b"\x01\x02\x03"
        result = self.module.compare_pcm_streams(left, right)
        self.assertEqual(result["verdict"], "MISMATCH")
        self.assertEqual(result["first_divergence"]["offset"], 3)
        self.assertIsNone(result["first_divergence"]["left_byte"])
        self.assertIsNone(result["first_divergence"]["right_byte"])

    def test_empty_streams_match(self):
        result = self.module.compare_pcm_streams(b"", b"")
        self.assertEqual(result["verdict"], "MATCH")
        self.assertEqual(result["left_bytes"], 0)

    def test_md5_file_streams_like_hashlib(self):
        payload = bytes((i * 31 + 7) % 256 for i in range(100_000))
        path = self.root / "pcm.bin"
        path.write_bytes(payload)
        self.assertEqual(self.module.md5_file(path), hashlib.md5(payload).hexdigest())


class TestMockOracle(HarnessTestCase):
    """Mock-decoder mode, fully exercised through the subprocess seam."""

    def make_input(self, size=8192, fill=0x5A):
        path = self.root / "input.avi"
        path.write_bytes(bytes([fill]) * size)
        return path

    def test_version_pin_accepts_pinned(self):
        oracle = self.module.MockOracle()
        line = oracle.assert_version_pin()
        self.assertTrue(line.startswith(f"VLC version {self.module.PINNED_VLC_VERSION}"))

    def test_version_drift_aborts_through_the_real_path(self):
        oracle = self.module.MockOracle()
        saved = os.environ.copy()
        os.environ["LUMEN_MOCK_VLC_VERSION"] = "3.0.24"
        try:
            with self.assertRaises(self.module.VersionDrift) as caught:
                oracle.assert_version_pin()
        finally:
            os.environ.clear()
            os.environ.update(saved)
        self.assertIn("3.0.24", str(caught.exception))
        self.assertIn("3.0.23", str(caught.exception))

    def test_dump_is_deterministic_and_synthetic(self):
        oracle = self.module.MockOracle()
        source = self.make_input()
        first = self.root / "dump1.raw"
        second = self.root / "dump2.raw"
        ok, note = oracle.dump_pcm(source, first, "s16", timeout_s=30)
        self.assertTrue(ok, note)
        ok, note = oracle.dump_pcm(source, second, "s16", timeout_s=30)
        self.assertTrue(ok, note)
        self.assertEqual(first.read_bytes(), second.read_bytes(),
                         "the mock is deterministic")
        # It is NOT the input bytes (synthetic, not a copy).
        self.assertNotEqual(first.read_bytes(), source.read_bytes())

    def test_dump_echo_hook_copies_ground_truth(self):
        oracle = self.module.MockOracle()
        source = self.make_input()
        ground_truth = b"\x11\x22\x33\x44" * 512
        truth_file = self.root / "truth.pcm"
        truth_file.write_bytes(ground_truth)
        out = self.root / "dump.raw"
        env = dict(os.environ, LUMEN_MOCK_VLC_ECHO=str(truth_file))
        saved = os.environ.copy()
        os.environ.update(env)
        try:
            ok, note = oracle.dump_pcm(source, out, "s16", timeout_s=30)
        finally:
            os.environ.clear()
            os.environ.update(saved)
        self.assertTrue(ok, note)
        self.assertEqual(out.read_bytes(), ground_truth)
        self.assertEqual(
            self.module.md5_file(out), hashlib.md5(ground_truth).hexdigest()
        )

    def test_dump_command_shape_matches_documented_flags(self):
        command = self.module.vlc_dump_command(
            "vlc", Path("/in/a.avi"), Path("/tmp/out.raw"), "s16"
        )
        expected = [
            "vlc", "-I", "dummy", "--play-and-exit", "--vout", "none",
            "--aout", "afile", "--no-audiofile-wav",
            "--audiofile-format", "s16", "--audiofile-file", "/tmp/out.raw",
            "/in/a.avi",
        ]
        self.assertEqual(command, expected)


class TestRealOracleRefusal(HarnessTestCase):
    """The separation: no VLC binary ⇒ the real oracle refuses, never
    silently degrades to the mock."""

    def test_resolve_raises_when_absent(self):
        saved = os.environ.get("PATH", "")
        os.environ["PATH"] = str(self.root)  # nothing in it
        try:
            with self.assertRaises(self.module.OracleUnavailable):
                self.module.VlcOracle.resolve()
        finally:
            os.environ["PATH"] = saved

    def test_main_vlc_mode_exits_2_when_absent(self):
        # A valid tiny manifest so input parsing succeeds and the run
        # reaches oracle resolution — which must refuse (exit 2).
        manifest = self.root / "manifest.json"
        manifest.write_text(json.dumps({"files": [
            {"rel": "media/x.avi", "class": "avi-av",
             "params": {"container": "avi", "codec": "mjpeg",
                        "audio_codec": "pcm_s16le"}}
        ]}))
        env = dict(os.environ)
        env["PATH"] = str(self.root)  # nothing executable in it
        proc = subprocess.run(
            [sys.executable, str(RUNPY), "--manifest", str(manifest),
             "--corpus-root", str(self.root), "--oracle", "vlc",
             "--out", str(self.root / "r.md")],
            capture_output=True, text=True, check=False, env=env,
        )
        self.assertEqual(proc.returncode, 2, proc.stderr)
        self.assertIn("no vlc/cvlc on PATH", proc.stderr)
        # The refusal never mentions or falls back to the mock.
        self.assertNotIn("mock", proc.stderr.lower())
        self.assertNotIn("MOCK", proc.stderr)


class TestReportLabels(HarnessTestCase):
    """Mock vs pending labeling, end to end through build_report."""

    def lumen_results(self, rel, md5, pcm_bytes):
        return {
            "schema": self.module.LUMEN_SCHEMA,
            "host_endian": "little",
            "results": [{
                "rel": rel, "status": "ok", "md5": md5,
                "pcm_bytes": pcm_bytes, "frames": 1, "packets": 1,
                "codec": "pcm_s16le", "channels": 1, "rate": 8000,
                "sample": "s16", "layout": "interleaved", "track": 1,
                "concealed": 0, "demux_recovered": 0, "format_changes": 0,
            }],
        }

    def entries(self, rel):
        return [{
            "rel": rel, "class": "avi-av", "bytes": 1234,
            "abs_path": self.root / rel,
            "params": {"container": "avi", "codec": "mjpeg",
                       "audio_codec": "pcm_s16le"},
        }]

    def test_pending_report_has_pending_rows_and_no_digests(self):
        entries = self.entries("media/0001-avi-av.avi")
        lumen = self.lumen_results("media/0001-avi-av.avi",
                                   "a" * 32, 4096)
        markdown, evidence = self.module.build_report(
            self.module.PendingOracle(), entries, lumen,
            {"generated_at": "2026-10-23T00:00:00+0000"}, ffprobe_check=False,
        )
        self.assertIn("PENDING-VLC", markdown)
        self.assertIn("no oracle digest of any kind", markdown)  # banner
        # The exact to-run command is present, with the documented flags.
        self.assertIn("--aout afile", markdown)
        self.assertIn("--audiofile-format s16", markdown)
        self.assertIn("--no-audiofile-wav", markdown)
        self.assertIn("media/0001-avi-av.avi", markdown)
        # No oracle digest anywhere.
        self.assertTrue(all(row.get("oracle_md5") is None
                            for row in evidence["rows"]))
        self.assertEqual(evidence["counts"]["comparable"], 1)

    def test_mock_report_labels_every_digest_row(self):
        rel = "media/0001-avi-av.avi"
        entries = self.entries(rel)
        source = self.root / rel
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_bytes(b"\x00" * 8192)
        # The echo hook makes the mock emit the ground-truth bytes, and
        # the (synthetic, test-fixture) lumen digest is set to those
        # same bytes' MD5 — exercising the MATCH path end to end.
        ground = self.root / "ground.pcm"
        ground.write_bytes(b"\x00" * 4096)
        lumen = self.lumen_results(rel, hashlib.md5(b"\x00" * 4096).hexdigest(),
                                   4096)
        saved = os.environ.copy()
        os.environ["LUMEN_MOCK_VLC_ECHO"] = str(ground)
        try:
            markdown, evidence = self.module.build_report(
                self.module.MockOracle(), entries, lumen,
                {"generated_at": "2026-10-23T00:00:00+0000"}, ffprobe_check=False,
            )
        finally:
            os.environ.clear()
            os.environ.update(saved)
        self.assertIn("[MOCK]", markdown)
        self.assertIn("MATCH [MOCK]", markdown)
        self.assertEqual(evidence["oracle"]["kind"], "mock")
        row = evidence["rows"][0]
        self.assertIsNotNone(row["oracle_md5"])
        self.assertEqual(row["oracle_md5"], hashlib.md5(b"\x00" * 4096).hexdigest())

    def test_mock_mismatch_path_labels_too(self):
        rel = "media/0002-avi-av.avi"
        entries = self.entries(rel)
        source = self.root / rel
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_bytes(b"\x11" * 8192)
        lumen = self.lumen_results(rel, "c" * 32, 4096)
        # No echo hook: the mock emits deterministic synthetic bytes —
        # a guaranteed MISMATCH against the fabricated lumen digest.
        markdown, evidence = self.module.build_report(
            self.module.MockOracle(), entries, lumen,
            {"generated_at": "2026-10-23T00:00:00+0000"}, ffprobe_check=False,
        )
        self.assertIn("MISMATCH [MOCK]", markdown)
        self.assertEqual(evidence["oracle"]["kind"], "mock")


class TestVlcVersionParsing(HarnessTestCase):
    def test_drift_detected_on_fake_binary(self):
        """A fake 'vlc' printing the wrong version must abort via
        VersionDrift — the doc 10 §6.6 pin rule."""
        fake = self.root / "vlc"
        fake.write_text(
            "#!/bin/sh\n"
            "echo 'VLC version 3.0.21 Umbrella (fake test binary)'\n"
        )
        fake.chmod(0o755)
        oracle = self.module.VlcOracle(str(fake))
        with self.assertRaises(self.module.VersionDrift):
            oracle.assert_version_pin()

    def test_correct_version_accepted(self):
        fake = self.root / "vlc"
        fake.write_text(
            "#!/bin/sh\n"
            "echo 'VLC version 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)'\n"
        )
        fake.chmod(0o755)
        oracle = self.module.VlcOracle(str(fake))
        line = oracle.assert_version_pin()
        self.assertIn("3.0.23", line)


class TestRefusedRowsProvenanceAndFilesMode(HarnessTestCase):
    """Lumen-refused comparable rows (oracle still runs, verdict says
    LUMEN-REFUSED, never MATCH), run-notes/provenance rendering, and
    the --files CLI mode's rel/class derivation."""

    def test_lumen_refused_row_gets_oracle_and_label(self):
        rel = "media/0003-avi-av.avi"
        entries = [{
            "rel": rel, "class": "avi-av", "bytes": 1234,
            "abs_path": self.root / rel,
            "params": {"container": "avi", "codec": "mjpeg",
                       "audio_codec": "pcm_s16le"},
        }]
        source = self.root / rel
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_bytes(b"\x22" * 8192)
        lumen = {
            "schema": self.module.LUMEN_SCHEMA,
            "host_endian": "little",
            "results": [{
                "rel": rel, "status": "error",
                "error": "[demux] no audio track (nothing to digest)",
            }],
        }
        markdown, evidence = self.module.build_report(
            self.module.MockOracle(), entries, lumen,
            {"generated_at": "2026-10-23T00:00:00+0000",
             "source": {"mode": "files", "n_files": 1},
             "notes": ["note one"]},
            ffprobe_check=False,
        )
        # The oracle answered (mock); the verdict names the refusal and
        # carries the [MOCK] label — never MATCH.
        self.assertIn("LUMEN-REFUSED [MOCK]", markdown)
        self.assertNotIn("MATCH [MOCK]", markdown)
        self.assertIn("## Lumen-side refusals on comparable-classified rows",
                      markdown)
        self.assertIn("[demux] no audio track", markdown)
        # The manifest-declared audio codec stays visible on the row.
        self.assertIn("pcm_s16le", markdown)
        # --note lands in the Run notes section.
        self.assertIn("## Run notes", markdown)
        self.assertIn("- note one", markdown)
        row = evidence["rows"][0]
        self.assertIsNotNone(row["oracle_md5"], "the mock oracle answered")
        self.assertEqual(row["lumen"]["status"], "error")

    def test_manifest_provenance_line(self):
        rel = "media/0001-avi-video.avi"
        entries = [{
            "rel": rel, "class": "avi-video", "bytes": 10,
            "abs_path": self.root / rel,
            "params": {"container": "avi", "codec": "mjpeg"},
        }]
        lumen = {
            "schema": self.module.LUMEN_SCHEMA,
            "host_endian": "little",
            "results": [],
        }
        markdown, _ = self.module.build_report(
            self.module.PendingOracle(), entries, lumen,
            {"generated_at": "2026-10-23T00:00:00+0000",
             "source": {"mode": "manifest", "manifest": "/x/manifest.json",
                        "manifest_seed": 42,
                        "manifest_generated_at": "2026-10-23T06:39:59+0000",
                        "manifest_count": 500},
             "notes": None},
            ffprobe_check=False,
        )
        self.assertIn("- Corpus source: manifest `/x/manifest.json` "
                      "(seed 42, generated 2026-10-23T06:39:59+0000, "
                      "500 files)", markdown)
        # No notes → no Run notes section.
        self.assertNotIn("## Run notes", markdown)

    @unittest.skipUnless(shutil.which("ffprobe"), "--files mode needs ffprobe")
    def test_files_mode_rel_and_class_cli(self):
        """End-to-end main() in --files mode: rel is corpus-relative,
        dirty/ files are excluded by class, --note reaches the report."""
        (self.root / "media").mkdir()
        clean = self.root / "media" / "a.avi"
        clean.write_bytes(b"RIFF0000")  # garbage: ffprobe refuses → facts None
        (self.root / "dirty").mkdir()
        dirty = self.root / "dirty" / "b.avi"
        dirty.write_bytes(b"RIFF0000")
        lumen = {"schema": self.module.LUMEN_SCHEMA,
                 "host_endian": "little", "results": []}
        lumen_path = self.root / "lumen.json"
        lumen_path.write_text(json.dumps(lumen))
        out = self.root / "report.md"
        proc = subprocess.run(
            [sys.executable, str(RUNPY),
             "--files", str(clean), str(dirty),
             "--corpus-root", str(self.root), "--oracle", "pending",
             "--lumen-json", str(lumen_path), "--out", str(out),
             "--note", "cli note", "--no-ffprobe-check"],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(proc.returncode, 0, proc.stderr)
        markdown = out.read_text(encoding="utf-8")
        self.assertIn("media/a.avi", markdown)
        self.assertIn("dirty/b.avi", markdown)
        self.assertIn("- cli note", markdown)
        evidence = json.loads(
            (self.root / "report.json").read_text(encoding="utf-8"))
        by_rel = {r["rel"]: r for r in evidence["rows"]}
        self.assertEqual(by_rel["media/a.avi"]["class"], "explicit")
        self.assertEqual(by_rel["media/a.avi"]["classification"]["status"],
                         "non_comparable")
        self.assertEqual(by_rel["dirty/b.avi"]["class"], "dirty")
        self.assertEqual(by_rel["dirty/b.avi"]["classification"]["status"],
                         "excluded")

    def test_vlc_mode_version_drift_exits_3(self):
        """A present-but-drifted binary aborts cleanly (exit 3), the
        doc 10 §6.6 pin rule through the CLI surface."""
        fake = self.root / "vlc"
        fake.write_text(
            "#!/bin/sh\n"
            "echo 'VLC version 3.0.21 Umbrella (fake test binary)'\n"
        )
        fake.chmod(0o755)
        manifest = self.root / "manifest.json"
        manifest.write_text(json.dumps({"files": [
            {"rel": "media/x.avi", "class": "avi-av",
             "params": {"container": "avi", "codec": "mjpeg",
                        "audio_codec": "pcm_s16le"}}
        ]}))
        env = dict(os.environ)
        env["PATH"] = str(self.root)
        proc = subprocess.run(
            [sys.executable, str(RUNPY), "--manifest", str(manifest),
             "--corpus-root", str(self.root), "--oracle", "vlc",
             "--out", str(self.root / "r.md")],
            capture_output=True, text=True, check=False, env=env,
        )
        self.assertEqual(proc.returncode, 3, proc.stderr)
        self.assertIn("3.0.21", proc.stderr)
        self.assertIn("3.0.23", proc.stderr)


if __name__ == "__main__":
    unittest.main()
