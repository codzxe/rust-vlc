# tests/fixtures — the doc 07 §3 validation fixture set

One fixture per rule row of `docs/07-config-driven-system.md` §3.3 (rule
inventory) plus the §3.4 stage-1 syntax case and the §3.5 golden example.
Each fixture is the repo-root `product.toml` (doc 07 §2 draft) with exactly
one documented mutation — regenerate a fixture by re-applying its mutation
to a fresh copy of `product.toml` (the generation script was a one-off;
mutations are listed below).

The test suites that consume these files:

- `tests/fixtures_matrix.rs` — structural + parse fixtures: each must be
  rejected with the documented key path, rule and line.
- `tests/golden_s35.rs` — `range-probe-max-bytes.toml` must render the doc
  07 §3.5 example diagnostic.
- `tests/semantic.rs` — cross-field, injected-enum, catalog, F1/F2 and
  asset fixtures (run with explicit `SemanticInputs`).

## Fixture ↔ rule map (doc 07 §3.3)

| Fixture | Rule class | Mutation |
|---|---|---|
| `invalid-syntax.toml` | §3.4 stage 1 | `width = ` with no value (TOML syntax error) |
| `missing-section-identity.toml` | Required fields | whole `[identity]` subtree removed |
| `missing-identity-field.toml` | Required fields | `identity.description_key` removed |
| `missing-window-fields.toml` | Required fields | `window.title_template` + `window.title_idle_key` removed (two diagnostics: every violation is reported) |
| `missing-brand-asset.toml` | Required fields | `brand.icon_png_512` removed |
| `missing-defaults-key.toml` | Required fields | `defaults.locale` removed |
| `missing-limits-key.toml` | Required fields | `limits.max_open_sessions` removed |
| `enum-subtitle-mode.toml` | Enums | `subtitle_mode = "always"` |
| `enum-hw-decode-policy.toml` | Enums (post-ADR-001 vocabulary) | `hw_decode_policy = "hw-first"` (ADR-001's feature-matrix term, not a config value; legal set is software-only/prefer-hardware/hardware-only per ADR-001, accepted 2026-10-19) |
| `enum-crash-reports.toml` | Enums | `crash_reports = "always"` |
| `range-window-width.toml` | Ranges (geometry) | `width = 100` (below 480) |
| `range-default-rate.toml` | Ranges | `default_rate = 10.0` (above 4.0) |
| `range-probe-max-bytes.toml` | Ranges + §3.5 golden | `probe_max_bytes = 1048576` (below 16 MiB) |
| `range-limits-multi.toml` | Ranges (limits) | four `[limits]` keys out of range at once |
| `type-window-width.toml` | Types | `width = "wide"` |
| `type-features-value.toml` | Types | `features.core-playback = "yes"` |
| `pattern-short-name.toml` | Patterns | `short_name = "Lumen Player"` (spaces) |
| `pattern-display-version.toml` | Patterns | `display_version = "version one"` |
| `pattern-accent-color.toml` | Patterns | `accent_color = "4f8cff"` (no `#`) |
| `pattern-reverse-dns.toml` | Patterns | `reverse_dns = "Dev.Lumen"` (uppercase) |
| `pattern-guid.toml` | Patterns | `msi_upgrade_code = "not-a-guid"` |
| `pattern-title-template.toml` | Patterns | `title_template = "{unknown_var} — {product_name}"` |
| `pattern-msgid.toml` | Patterns | `description_key = "Product Description"` |
| `pattern-urls.toml` | Patterns | `urls.help` downgraded to `http://` |
| `pattern-bcp47.toml` | Patterns | `preferred_audio_languages = ["english"]` |
| `reserved-name.toml` | Unique-identity (a) | `name = "VLC"` (reserved list) |
| `reserved-short-name.toml` | Unique-identity (a) | `short_name = "vlc"` (semantic pass) |
| `unknown-key-typo.toml` | §3.5 (4) | `cache_demux_mb` typo → did-you-mean `cache_demux_mib` |
| `unknown-top-level.toml` | Unknown keys | appended `[extra]` section |
| `cross-field-min-width.toml` | Cross-field | `min_width = 1600` ≥ `width` |
| `cross-field-schema-version.toml` | Cross-field | `schema_version = 2` with no migrations |
| `cross-field-telemetry-privacy.toml` | Cross-field | `telemetry.enabled = true`, `urls.privacy` removed |
| `id-consistency.toml` | Unique-identity (b) | `app_user_model_id = "wrong.id.Here"` |
| `migrations-inverted.toml` | Cross-field (migrations) | entry `from = 3, to = 2` |
| `assets-wrong-magic.toml` + `assets/` | Path existence | `icon.ico` has wrong magic bytes |
| `assets-missing.toml` | Path existence | `logo_svg` points at a missing file |
| `injected-enum-theme.toml` | Enums (injected) | `theme = "oled-dark"` |
| `injected-enum-locale.toml` | Enums (injected) | `locale = "xx-XX"` |
| `injected-enum-profile.toml` | Enums (injected) | `profile = "cinema"` |
| `catalog-missing-key.toml` | Cross-field (catalog) | `title_idle_key = "window.title.gone"` |
| `feature-unknown-f1.toml` | F1 | `telepathy = true` (not in the registry) |
| `feature-unimplemented-f2.toml` | F2 | `streaming-http = true` (known, not implemented) |

`assets/` is a miniature repository root: `assets/assets/brand/*` are read
with `SemanticInputs::base_dir = tests/fixtures/assets`; `icon.ico`
deliberately contains wrong magic bytes, the other three assets are valid.
