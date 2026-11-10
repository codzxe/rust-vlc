#!/usr/bin/env python3
"""Derive the per-OS CI run config for the compat-matrix runner (issue #53).

Usage (from the repository root):

    python3 tests/compat/ci/derive_config.py <platform> <out-config> <reports-dir>

    <platform>    windows | linux | macos — the feature-matrix column this
                  CI job's native sweep is evidence for (config/2's
                  native-decode `platform`; `web` is the steps lane, never
                  a native sweep — refused here like the runner refuses it)
    <out-config>  where the derived config is written (the runner's
                  `--config` argument)
    <reports-dir> directory the derived config points the runner's reports
                  at (the runner creates it; the committed docs/corpus
                  evidence files are NEVER touched — CI reports are
                  ephemeral artifacts, uploaded by the workflow, never
                  commits)

What it derives from the committed `tests/compat/compat-matrix.json`
(schema `compat-matrix/config/2`):

  * claims are REFRESHED from the LIVE `docs/feature-matrix.toml` (the
    parity ledger, read with stdlib tomllib) — so the rendered
    `claimed (<platform>)` columns and the gap rules judge the CURRENT
    matrix, never a stale snapshot (the committed config's claims are the
    2026-11-10 pre-triage snapshot; a stale claim would re-emit
    already-applied transitions that the parity tool would then reject
    as stale — pointless noise, and a misleading report);
  * the native-decode lane is re-attributed to `<platform>` (id
    `<platform>-native`, kind.platform `<platform>`) — the whole point of
    config/2: attribution is explicit, never inferred from the host;
  * the blocked lane whose id equals `<platform>` is DROPPED (that OS's
    sweep IS this job); the remaining blocked lanes keep their ids but
    carry CI-honest citations (a sibling matrix job of the same dispatch
    owns that lane — no result is inferred here);
  * report paths move under `<reports-dir>/compat-matrix-<platform>.*`.

The oracle block is kept verbatim: on CI hosts the pinned Debian VLC
tree does not exist, so the runner records the absence and cites the
committed 2026-11-09 record (labeled `committed-record`, never inferred
— the 3d90424 semantics this job was built for).

Exit codes: 0 derived | 1 usage or derivation failure (loud; nothing is
written when a check fails).
"""

import json
import os
import sys

try:
    import tomllib
except ImportError:  # CI runners ship 3.11+; the fail is loud otherwise
    print("derive_config: python3 >= 3.11 (stdlib tomllib) is required", file=sys.stderr)
    sys.exit(1)

PLATFORMS = ("windows", "linux", "macos")
PLATFORM_KEYS = ("windows", "linux", "macos", "web")
SOURCE_CONFIG = "tests/compat/compat-matrix.json"
LIVE_MATRIX = "docs/feature-matrix.toml"
SCHEMA = "compat-matrix/config/2"


def fail(message: str) -> None:
    print(f"derive_config: {message}", file=sys.stderr)
    sys.exit(1)


def load_committed_config() -> dict:
    try:
        with open(SOURCE_CONFIG, encoding="utf-8") as handle:
            config = json.load(handle)
    except OSError as error:
        fail(f"cannot read {SOURCE_CONFIG}: {error}")
    except json.JSONDecodeError as error:
        fail(f"cannot parse {SOURCE_CONFIG}: {error}")
    if config.get("schema") != SCHEMA:
        fail(f"unsupported schema {config.get('schema')!r} (want {SCHEMA!r})")
    return config


def live_claims(config: dict) -> list:
    """Refresh the config's claims snapshot from the live parity ledger."""
    try:
        with open(LIVE_MATRIX, "rb") as handle:
            matrix = tomllib.load(handle)
    except OSError as error:
        fail(f"cannot read {LIVE_MATRIX}: {error}")
    except tomllib.TOMLDecodeError as error:
        fail(f"cannot parse {LIVE_MATRIX}: {error}")
    live = {row.get("id", ""): row.get("status", {}) for row in matrix.get("feature", [])}
    claims = config.get("claims", [])
    missing = [claim["row"] for claim in claims if claim["row"] not in live]
    if missing:
        fail(
            f"claim rows absent from the live {LIVE_MATRIX} (config drift — fix the "
            f"config's claims list): {', '.join(missing)}"
        )
    refreshed = []
    for claim in claims:
        status = live[claim["row"]]
        bad_keys = [key for key in PLATFORM_KEYS if key not in status]
        if bad_keys:
            fail(f"matrix row {claim['row']!r} lacks platform column(s) {bad_keys}")
        refreshed.append({"row": claim["row"], **{key: status[key] for key in PLATFORM_KEYS}})
    return refreshed


def derive_lanes(config: dict, platform: str) -> list:
    """Re-attribute the native lane; drop this platform's blocked lane."""
    native_seen = 0
    kept = []
    for lane in config.get("lanes", []):
        kind = lane.get("kind", {})
        if kind.get("type") == "native-decode":
            native_seen += 1
            kind["platform"] = platform
            lane["id"] = f"{platform}-native"
            lane["label"] = (
                f"{platform.capitalize()} native decode "
                "(CI runner lane — the lane that actually ran)"
            )
        elif kind.get("type") == "blocked":
            if lane.get("id") == platform:
                continue  # this OS's sweep IS this job — its blocked lane is dropped
            other = lane["id"]
            kind["reason"] = (
                f"this job runs the native sweep on {platform} only; the {other} sweep is a "
                "sibling matrix job of the same workflow dispatch — no result is inferred here"
            )
            kind["reason_ref"] = (
                ".github/workflows/compat-matrix.yml (the 3-OS matrix lanes of this dispatch)"
            )
            kind["next_step"] = (
                f"the {other} sibling job's uploaded artifact carries that lane's real run; "
                "its outbox proposals flow through cargo xtask compat-matrix "
                "apply-transitions like any other (the parity tool re-checks every from-status)"
            )
        kept.append(lane)
    if native_seen != 1:
        fail(f"expected exactly one native-decode lane in {SOURCE_CONFIG}, found {native_seen}")
    return kept


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__, file=sys.stderr)
        return 1
    platform, out_config, reports_dir = sys.argv[1], sys.argv[2], sys.argv[3]
    if platform not in PLATFORMS:
        fail(
            f"platform must be one of {'/'.join(PLATFORMS)} (got {platform!r} — "
            "`web` is the steps lane, never a native sweep)"
        )

    config = load_committed_config()
    claims = live_claims(config)
    lanes = derive_lanes(config, platform)
    config["claims"] = claims
    config["lanes"] = lanes
    config["reports"] = {
        "markdown": f"{reports_dir}/compat-matrix-{platform}.md",
        "json": f"{reports_dir}/compat-matrix-{platform}.json",
        "raw_rows": f"{reports_dir}/compat-matrix-{platform}.raw-rows.jsonl",
        "transitions": f"{reports_dir}/compat-matrix-{platform}.transitions.json",
    }

    parent = os.path.dirname(out_config)
    if parent:
        os.makedirs(parent, exist_ok=True)
    with open(out_config, "w", encoding="utf-8") as handle:
        json.dump(config, handle, indent=1)
        handle.write("\n")

    # A short derivation summary for the job log (the operator's first
    # stop when a lane looks wrong).
    print(f"derive_config: platform={platform} -> {out_config}")
    print(f"derive_config: claims refreshed from the live {LIVE_MATRIX} ({len(claims)} rows)")
    print(f"derive_config: lanes = {', '.join(lane['id'] for lane in lanes)}")
    print(f"derive_config: reports under {reports_dir}/ (ephemeral; committed evidence untouched)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
