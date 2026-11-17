# W3C Internationalization (`https://www.w3.org/International/`) Architecture & Implementation

> End-to-end W3C Internationalization compliance across `aphrody` (Rust OS, GPUI, Craft, NSH, WebGPU/Wasm, Bun tooling), `aphrody-dev`, `aphrody-labs` repositories, and Windows developer workstations.

## Live W3C Standards Referenced

| Standard / Resource | URL | Implemented Surface |
| --- | --- | --- |
| W3C Internationalization Activity | <https://www.w3.org/International/> | Workspace policy, CI checks, multi-script catalogs |
| Internationalization Best Practices for Spec Developers | <https://www.w3.org/TR/international-specs/> | Schema, HTTP header negotiation, JSON/TOML protocols |
| Strings on the Web: Language & Direction Metadata (`string-meta`) | <https://www.w3.org/TR/string-meta/> | [`aphrody_session::i18n`](../../crates/aphrody_session/src/i18n.rs) (`TextDir`, `LocalizedString`, `to_json_ld`) |
| Character Model: String Matching (`charmod-norm`) | <https://www.w3.org/TR/charmod-norm/> | UTF-8 only, BOM-free, NFC paths, Bidi isolate/Trojan Source guard |
| Language Tags and Locale Identifiers (`ltli`, BCP 47) | <https://www.w3.org/TR/ltli/> | `Bcp47Tag`, subtag canonicalization, RFC 4647 fallback chain |
| Authoring HTML & SVG Techniques | <https://www.w3.org/International/techniques/authoring-html> | `<meta charset="utf-8">`, `<html lang="..." dir="...">`, CSS logical properties, `<svg xml:lang>` |
| Language Enablement & Script Typography (`clreq`, `jlreq`, `alreq`, `ilreq`, `klreq`) | <https://www.w3.org/TR/typography/> | `cosmic-text` / HarfBuzz / DirectWrite shaping, `ar`/`he`/`fa`/`hi`/`ja`/`zh-Hans`/`ko`/`uk` catalogs |

## Implemented Architecture Across Stack Layers

1. **OS Core & Protocol Primitives ([`aphrody_session::i18n`](../../crates/aphrody_session/src/i18n.rs))**:
   - `Bcp47Tag::parse` converts platform/POSIX forms (`fr_FR.UTF-8`, `zh_hans_cn@collation=pinyin`) to canonical BCP 47 tags (`fr-FR`, `zh-Hans-CN`) with RFC 4647 fallback chains.
   - `TextDir` (`Ltr`, `Rtl`, `Auto`) and `strong_direction` implement W3C First-Strong resolution across Latin, Arabic, Hebrew, Syriac, Thaana, N'Ko, and Adlam ranges.
   - `LocalizedString` encodes W3C `string-meta` (`value`, `lang`, `dir`, JSON-LD `@language`/`@direction`), and `wrap_bidi_isolate` isolates inline spans with `U+2066 LRI` / `U+2067 RLI` / `U+2068 FSI` ... `U+2069 PDI`.
   - `scan_bidi_hazards` & `validate_utf8_no_bom` flag unsafe `U+202A..U+202E` Trojan Source overrides, unclosed isolates, and stray UTF-8 BOMs.
   - `negotiate_locale` parses HTTP `Accept-Language` with RFC 9110 `q` weights, returning `Content-Language` and `Vary: Accept-Language`.
2. **OS UI & Multi-Script Catalog ([`aphrody_ui::i18n`](../../crates/aphrody_ui/src/i18n.rs), [`locales.toml`](../../assets/os/locales.toml))**:
   - English base catalog (`en`) paired with verified translations across `fr`, `ar` (RTL), `he` (RTL), `es`, `de`, `pt`, `ja`, `zh-Hans`, `ko`, `hi` (Indian 2,2,3 grouping), and `uk`.
   - `plural_category` implements CLDR cardinal plural rules (`zero`, `one`, `two`, `few`, `many`, `other`).
   - `format_iso_utc` emits RFC 3339 / ISO 8601 `YYYY-MM-DDTHH:MM:SSZ` interchange timestamps.
   - [`SettingsProgram`](../../crates/aphrody_desktop/src/windows.rs) exposes live BCP 47 language + direction (`dir=ltr|rtl`) selection persisted in `DesktopPrefs`; [`aphrody-os doctor --json`](../../crates/aphrody_desktop/src/main.rs) exports resolved `i18n` telemetry.
3. **NSH Runtime & Intl ([`crates/nsh-i18n`](../../crates/nsh-i18n/src/lib.rs))**:
   - Expanded CLDR compact formatting datasets (`ar.json`, `fa.json`, `he.json` alongside existing 18 languages) plus `direction()` and `is_rtl()`.
4. **HTTP/MCP & WebGPU/Wasm HTML Surfaces**:
   - [`aphrody_craft::mcp`](../../crates/aphrody_craft/src/mcp.rs) negotiates `Accept-Language` and emits `Content-Type: application/json; charset=utf-8`, `Content-Language`, and `Vary: Accept-Language`.
   - [`web-build.ts`](../../tooling/bun/web-build.ts), [`web_examples.rs`](../../tooling/xtask/src/tasks/web_examples.rs), `gpui_web`, `nsh-platform-web`, and `nsh-cli` HTML hosts emit `<meta charset="utf-8">`, `<html lang="..." dir="...">`, and CSS Logical Properties (`inset-inline-*`, `border-inline-end`, `text-align: start`).
5. **Repositories & Developer PC Enforcement ([`w3c-i18n.ts`](../../tooling/bun/w3c-i18n.ts))**:
   - [`.editorconfig`](../../.editorconfig) & [`.gitattributes`](../../.gitattributes) enforce `charset = utf-8` and `* text=auto eol=lf` (`*.bat`/`*.cmd` CRLF).
   - Developer PC configuration: `git config --global core.quotepath=false`, `i18n.commitEncoding=utf-8`, `i18n.logOutputEncoding=utf-8`, user `~/.editorconfig`, `LANG=en_US.UTF-8`, `LC_CTYPE=en_US.UTF-8`, `PYTHONUTF8=1`, `LESSCHARSET=utf-8`, and UTF-8 PowerShell console I/O.

## CLI Usage

```sh
bun run i18n:check    # Verify UTF-8, LF, BCP 47, HTML/SVG attributes, catalogs (exit 1 on error)
bun run i18n:audit    # Emit JSON audit + workstation PC UTF-8 telemetry
bun run i18n:fix      # Strip BOMs and convert CRLF -> LF safely across repo source/docs
```
