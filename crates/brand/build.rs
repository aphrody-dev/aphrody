#![allow(
    clippy::disallowed_methods,
    reason = "build scripts run outside the GPUI async runtime"
)]

use std::env;
use std::fs;
use std::path::PathBuf;

fn get_str<'a>(table: &'a toml::Value, section: &str, key: &str) -> &'a str {
    table
        .get(section)
        .and_then(|sec| sec.get(key))
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("missing `{section}.{key}` string in product.toml"))
}

fn get_str_list(table: &toml::Value, section: &str, key: &str) -> Vec<String> {
    table
        .get(section)
        .and_then(|sec| sec.get(key))
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("missing `{section}.{key}` array in product.toml"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("non-string item in `{section}.{key}`"))
                .to_owned()
        })
        .collect()
}

fn rust_str_literal(value: &str) -> String {
    format!("{value:?}")
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let product_toml_path = manifest_dir.join("../../product.toml");
    println!("cargo:rerun-if-changed={}", product_toml_path.display());

    let raw = fs::read_to_string(&product_toml_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", product_toml_path.display()));
    let parsed: toml::Value = toml::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", product_toml_path.display()));

    let name = get_str(&parsed, "product", "name");
    let slug = get_str(&parsed, "product", "slug");
    let tagline = get_str(&parsed, "product", "tagline");
    let description = get_str(&parsed, "product", "description");
    let company_name = get_str(&parsed, "product", "company_name");
    let copyright = get_str(&parsed, "product", "copyright");
    let id_prefix = get_str(&parsed, "product", "identifier_prefix");
    let win_id_prefix = get_str(&parsed, "product", "windows_app_id_prefix");
    let url_scheme = get_str(&parsed, "product", "url_scheme");
    let cli_url_scheme = get_str(&parsed, "product", "cli_url_scheme");
    let dock_action_scheme = get_str(&parsed, "product", "dock_action_scheme");
    let legacy_url_schemes = get_str_list(&parsed, "product", "legacy_url_schemes");
    let legacy_cli_schemes = get_str_list(&parsed, "product", "legacy_cli_url_schemes");
    let legacy_dock_schemes = get_str_list(&parsed, "product", "legacy_dock_action_schemes");

    let local_settings_folder = get_str(&parsed, "paths", "local_settings_folder");
    let legacy_local_settings_folder = get_str(&parsed, "paths", "legacy_local_settings_folder");
    let remote_server_folder = get_str(&parsed, "paths", "remote_server_folder");
    let legacy_remote_server_folder = get_str(&parsed, "paths", "legacy_remote_server_folder");
    let remote_wsl_server_folder = get_str(&parsed, "paths", "remote_wsl_server_folder");
    let legacy_remote_wsl_server_folder =
        get_str(&parsed, "paths", "legacy_remote_wsl_server_folder");

    let homepage = get_str(&parsed, "urls", "homepage");
    let repository = get_str(&parsed, "urls", "repository");
    let docs = get_str(&parsed, "urls", "docs");
    let status = get_str(&parsed, "urls", "status");
    let store = get_str(&parsed, "urls", "store");
    let server = get_str(&parsed, "urls", "server");
    let extension_registry = get_str(&parsed, "urls", "extension_registry");

    let env_prefix = get_str(&parsed, "env", "prefix");
    let legacy_env_prefix = get_str(&parsed, "env", "legacy_prefix");

    let sans_alias = get_str(&parsed, "fonts", "sans_alias");
    let mono_alias = get_str(&parsed, "fonts", "mono_alias");
    let legacy_sans_alias = get_str(&parsed, "fonts", "legacy_sans_alias");
    let legacy_mono_alias = get_str(&parsed, "fonts", "legacy_mono_alias");

    let logo_svg = get_str(&parsed, "assets", "logo_svg");

    let plugin_name = get_str(&parsed, "plugin", "name");
    let plugin_version = get_str(&parsed, "plugin", "version");
    let plugin_scope = get_str(&parsed, "plugin", "scope");

    let git_default_branch = get_str(&parsed, "git", "default_branch");
    let upstream_zed = get_str(&parsed, "git", "upstream_zed");
    let upstream_zed_branch = get_str(&parsed, "git", "upstream_zed_branch");
    let upstream_nsh = get_str(&parsed, "git", "upstream_nsh");
    let upstream_gpui_kit = get_str(&parsed, "git", "upstream_gpui_kit");
    let upstream_wgpu = get_str(&parsed, "git", "upstream_wgpu");
    let upstream_awesome_gpui = get_str(&parsed, "git", "upstream_awesome_gpui");
    let upstream_extensions = get_str(&parsed, "git", "upstream_extensions");
    let upstream_winget_pkgs = get_str(&parsed, "git", "upstream_winget_pkgs");
    let upstream_modern_web_guidance = get_str(&parsed, "git", "upstream_modern_web_guidance");
    let upstream_design_md = get_str(&parsed, "git", "upstream_design_md");
    let upstream_stitch_sdk = get_str(&parsed, "git", "upstream_stitch_sdk");
    let upstream_stitch_skills = get_str(&parsed, "git", "upstream_stitch_skills");
    let submodule_bun = get_str(&parsed, "git", "submodule_bun");

    let wgpu_cli_bin = get_str(&parsed, "features", "wgpu_cli_bin");

    let design_source_of_truth = get_str(&parsed, "design", "source_of_truth");
    let design_system = get_str(&parsed, "design", "system");
    let design_spec_url = get_str(&parsed, "design", "spec_url");
    let design_fonts_url = get_str(&parsed, "design", "fonts_url");
    let design_stitch_url = get_str(&parsed, "design", "stitch_url");
    let design_sans_flex_family = get_str(&parsed, "design", "sans_flex_family");
    let design_md = get_str(&parsed, "design", "design_md");
    let design_tokens_manifest = get_str(&parsed, "design", "tokens_manifest");
    let design_themes_manifest = get_str(&parsed, "design", "themes_manifest");
    let design_seed_hex = get_str(&parsed, "design", "seed_hex");
    let design_variant = get_str(&parsed, "design", "variant");

    let legacy_schemes_slice = legacy_url_schemes
        .iter()
        .map(|s| rust_str_literal(s))
        .collect::<Vec<_>>()
        .join(", ");
    let legacy_cli_slice = legacy_cli_schemes
        .iter()
        .map(|s| rust_str_literal(s))
        .collect::<Vec<_>>()
        .join(", ");
    let legacy_dock_slice = legacy_dock_schemes
        .iter()
        .map(|s| rust_str_literal(s))
        .collect::<Vec<_>>()
        .join(", ");

    let generated = format!(
        r#"pub const APP_NAME: &str = {name};
pub const APP_SLUG: &str = {slug};
pub const APP_NAME_DEV: &str = "{raw_name} Dev";
pub const APP_NAME_NIGHTLY: &str = "{raw_name} Nightly";
pub const APP_NAME_PREVIEW: &str = "{raw_name} Preview";
pub const TAGLINE: &str = {tagline};
pub const DESCRIPTION: &str = {description};
pub const COMPANY_NAME: &str = {company_name};
pub const COPYRIGHT: &str = {copyright};

pub const IDENTIFIER_STABLE: &str = "{raw_id_prefix}";
pub const IDENTIFIER_DEV: &str = "{raw_id_prefix}-Dev";
pub const IDENTIFIER_NIGHTLY: &str = "{raw_id_prefix}-Nightly";
pub const IDENTIFIER_PREVIEW: &str = "{raw_id_prefix}-Preview";

pub const WINDOWS_APP_ID_STABLE: &str = "{raw_win_prefix}-Stable";
pub const WINDOWS_APP_ID_DEV: &str = "{raw_win_prefix}-Dev";
pub const WINDOWS_APP_ID_NIGHTLY: &str = "{raw_win_prefix}-Nightly";
pub const WINDOWS_APP_ID_PREVIEW: &str = "{raw_win_prefix}-Preview";

pub const URL_SCHEME: &str = {url_scheme};
pub const URL_SCHEME_PREFIX: &str = "{raw_url_scheme}://";
pub const CLI_URL_SCHEME: &str = {cli_url_scheme};
pub const CLI_URL_SCHEME_PREFIX: &str = "{raw_cli_scheme}://";
pub const DOCK_ACTION_SCHEME: &str = {dock_action_scheme};
pub const DOCK_ACTION_SCHEME_PREFIX: &str = "{raw_dock_scheme}://";
pub const LEGACY_URL_SCHEMES: &[&str] = &[{legacy_schemes_slice}];
pub const LEGACY_CLI_URL_SCHEMES: &[&str] = &[{legacy_cli_slice}];
pub const LEGACY_DOCK_ACTION_SCHEMES: &[&str] = &[{legacy_dock_slice}];

pub const LOCAL_SETTINGS_FOLDER: &str = {local_settings_folder};
pub const LOCAL_SETTINGS_FILE: &str = "{raw_local_folder}/settings.json";
pub const LOCAL_TASKS_FILE: &str = "{raw_local_folder}/tasks.json";
pub const LOCAL_DEBUG_FILE: &str = "{raw_local_folder}/debug.json";

pub const LEGACY_LOCAL_SETTINGS_FOLDER: &str = {legacy_local_settings_folder};
pub const LEGACY_LOCAL_SETTINGS_FILE: &str = "{raw_legacy_folder}/settings.json";
pub const LEGACY_LOCAL_TASKS_FILE: &str = "{raw_legacy_folder}/tasks.json";
pub const LEGACY_LOCAL_DEBUG_FILE: &str = "{raw_legacy_folder}/debug.json";

pub const REMOTE_SERVER_FOLDER: &str = {remote_server_folder};
pub const LEGACY_REMOTE_SERVER_FOLDER: &str = {legacy_remote_server_folder};
pub const REMOTE_WSL_SERVER_FOLDER: &str = {remote_wsl_server_folder};
pub const LEGACY_REMOTE_WSL_SERVER_FOLDER: &str = {legacy_remote_wsl_server_folder};

pub const HOMEPAGE_URL: &str = {homepage};
pub const REPOSITORY_URL: &str = {repository};
pub const DOCS_URL: &str = {docs};
pub const STATUS_URL: &str = {status};
pub const STORE_URL: &str = {store};
pub const DEFAULT_SERVER_URL: &str = {server};
pub const EXTENSION_REGISTRY_URL: &str = {extension_registry};

pub const ENV_PREFIX: &str = {env_prefix};
pub const LEGACY_ENV_PREFIX: &str = {legacy_env_prefix};

pub const FONT_SANS_ALIAS: &str = {sans_alias};
pub const FONT_MONO_ALIAS: &str = {mono_alias};
pub const LEGACY_FONT_SANS_ALIAS: &str = {legacy_sans_alias};
pub const LEGACY_FONT_MONO_ALIAS: &str = {legacy_mono_alias};
pub const DEFAULT_ICON_THEME_NAME: &str = "{raw_name} (Default)";
pub const LEGACY_DEFAULT_ICON_THEME_NAME: &str = "Zed (Default)";

pub const LOGO_SVG_PATH: &str = {logo_svg};

pub const PLUGIN_NAME: &str = {plugin_name};
pub const PLUGIN_VERSION: &str = {plugin_version};
pub const PLUGIN_SCOPE: &str = {plugin_scope};

pub const GIT_DEFAULT_BRANCH: &str = {git_default_branch};
pub const UPSTREAM_ZED_REPO: &str = {upstream_zed};
pub const UPSTREAM_ZED_BRANCH: &str = {upstream_zed_branch};
pub const UPSTREAM_NSH_REPO: &str = {upstream_nsh};
pub const UPSTREAM_GPUI_KIT_REPO: &str = {upstream_gpui_kit};
pub const UPSTREAM_WGPU_REPO: &str = {upstream_wgpu};
pub const UPSTREAM_AWESOME_GPUI_REPO: &str = {upstream_awesome_gpui};
pub const UPSTREAM_EXTENSIONS_REPO: &str = {upstream_extensions};
pub const UPSTREAM_WINGET_PKGS_REPO: &str = {upstream_winget_pkgs};
pub const UPSTREAM_MODERN_WEB_GUIDANCE_REPO: &str = {upstream_modern_web_guidance};
pub const UPSTREAM_DESIGN_MD_REPO: &str = {upstream_design_md};
pub const UPSTREAM_STITCH_SDK_REPO: &str = {upstream_stitch_sdk};
pub const UPSTREAM_STITCH_SKILLS_REPO: &str = {upstream_stitch_skills};
pub const SUBMODULE_BUN_REPO: &str = {submodule_bun};

pub const WGPU_CLI_BIN: &str = {wgpu_cli_bin};

pub const DESIGN_SOURCE_OF_TRUTH: &str = {design_source_of_truth};
pub const DESIGN_SYSTEM: &str = {design_system};
pub const DESIGN_SPEC_URL: &str = {design_spec_url};
pub const DESIGN_FONTS_URL: &str = {design_fonts_url};
pub const DESIGN_STITCH_URL: &str = {design_stitch_url};
pub const DESIGN_SANS_FLEX_FAMILY: &str = {design_sans_flex_family};
pub const DESIGN_MD_PATH: &str = {design_md};
pub const DESIGN_TOKENS_MANIFEST: &str = {design_tokens_manifest};
pub const DESIGN_THEMES_MANIFEST: &str = {design_themes_manifest};
pub const DESIGN_SEED_HEX: &str = {design_seed_hex};
pub const DESIGN_VARIANT: &str = {design_variant};
"#,
        name = rust_str_literal(name),
        slug = rust_str_literal(slug),
        raw_name = name,
        tagline = rust_str_literal(tagline),
        description = rust_str_literal(description),
        company_name = rust_str_literal(company_name),
        copyright = rust_str_literal(copyright),
        raw_id_prefix = id_prefix,
        raw_win_prefix = win_id_prefix,
        url_scheme = rust_str_literal(url_scheme),
        raw_url_scheme = url_scheme,
        cli_url_scheme = rust_str_literal(cli_url_scheme),
        raw_cli_scheme = cli_url_scheme,
        dock_action_scheme = rust_str_literal(dock_action_scheme),
        raw_dock_scheme = dock_action_scheme,
        local_settings_folder = rust_str_literal(local_settings_folder),
        raw_local_folder = local_settings_folder,
        legacy_local_settings_folder = rust_str_literal(legacy_local_settings_folder),
        raw_legacy_folder = legacy_local_settings_folder,
        remote_server_folder = rust_str_literal(remote_server_folder),
        legacy_remote_server_folder = rust_str_literal(legacy_remote_server_folder),
        remote_wsl_server_folder = rust_str_literal(remote_wsl_server_folder),
        legacy_remote_wsl_server_folder = rust_str_literal(legacy_remote_wsl_server_folder),
        homepage = rust_str_literal(homepage),
        repository = rust_str_literal(repository),
        docs = rust_str_literal(docs),
        status = rust_str_literal(status),
        store = rust_str_literal(store),
        server = rust_str_literal(server),
        extension_registry = rust_str_literal(extension_registry),
        env_prefix = rust_str_literal(env_prefix),
        legacy_env_prefix = rust_str_literal(legacy_env_prefix),
        sans_alias = rust_str_literal(sans_alias),
        mono_alias = rust_str_literal(mono_alias),
        legacy_sans_alias = rust_str_literal(legacy_sans_alias),
        legacy_mono_alias = rust_str_literal(legacy_mono_alias),
        logo_svg = rust_str_literal(logo_svg),
        plugin_name = rust_str_literal(plugin_name),
        plugin_version = rust_str_literal(plugin_version),
        plugin_scope = rust_str_literal(plugin_scope),
        git_default_branch = rust_str_literal(git_default_branch),
        upstream_zed = rust_str_literal(upstream_zed),
        upstream_zed_branch = rust_str_literal(upstream_zed_branch),
        upstream_nsh = rust_str_literal(upstream_nsh),
        upstream_gpui_kit = rust_str_literal(upstream_gpui_kit),
        upstream_wgpu = rust_str_literal(upstream_wgpu),
        upstream_awesome_gpui = rust_str_literal(upstream_awesome_gpui),
        upstream_extensions = rust_str_literal(upstream_extensions),
        upstream_winget_pkgs = rust_str_literal(upstream_winget_pkgs),
        upstream_modern_web_guidance = rust_str_literal(upstream_modern_web_guidance),
        upstream_design_md = rust_str_literal(upstream_design_md),
        upstream_stitch_sdk = rust_str_literal(upstream_stitch_sdk),
        upstream_stitch_skills = rust_str_literal(upstream_stitch_skills),
        submodule_bun = rust_str_literal(submodule_bun),
        wgpu_cli_bin = rust_str_literal(wgpu_cli_bin),
        design_source_of_truth = rust_str_literal(design_source_of_truth),
        design_system = rust_str_literal(design_system),
        design_spec_url = rust_str_literal(design_spec_url),
        design_fonts_url = rust_str_literal(design_fonts_url),
        design_stitch_url = rust_str_literal(design_stitch_url),
        design_sans_flex_family = rust_str_literal(design_sans_flex_family),
        design_md = rust_str_literal(design_md),
        design_tokens_manifest = rust_str_literal(design_tokens_manifest),
        design_themes_manifest = rust_str_literal(design_themes_manifest),
        design_seed_hex = rust_str_literal(design_seed_hex),
        design_variant = rust_str_literal(design_variant),
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("product_constants.rs"), generated)
        .expect("failed to write product_constants.rs");
    generate_design_tokens(&out_dir);
}

fn generate_design_tokens(out_dir: &std::path::Path) {
    use std::collections::BTreeMap;
    use std::fmt::Write as _;

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let tokens_path = PathBuf::from(&manifest_dir).join("../../assets/os/google_design_tokens.toml");
    println!("cargo:rerun-if-changed={}", tokens_path.display());

    let raw = fs::read_to_string(&tokens_path).expect("failed to read assets/os/google_design_tokens.toml");
    let root: toml::Value = toml::from_str(&raw).expect("failed to parse google_design_tokens.toml");

    fn req_table<'a>(
        val: &'a toml::Value,
        key: &str,
    ) -> &'a toml::map::Map<String, toml::Value> {
        val.get(key)
            .and_then(|v| v.as_table())
            .unwrap_or_else(|| panic!("google_design_tokens.toml missing table `{key}`"))
    }
    fn req_str<'a>(tbl: &'a toml::map::Map<String, toml::Value>, key: &str) -> &'a str {
        tbl.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("google_design_tokens.toml missing string `{key}`"))
    }
    fn req_i64(tbl: &toml::map::Map<String, toml::Value>, key: &str) -> i64 {
        tbl.get(key)
            .and_then(|v| v.as_integer())
            .unwrap_or_else(|| panic!("google_design_tokens.toml missing integer `{key}`"))
    }
    let table = req_table;
    let fmt_f32 = |tbl: &toml::map::Map<String, toml::Value>, key: &str| -> String {
        match tbl.get(key) {
            Some(toml::Value::Float(f)) => {
                let s = f.to_string();
                if s.contains('.') {
                    format!("{s}_f32")
                } else {
                    format!("{s}.0_f32")
                }
            }
            Some(toml::Value::Integer(i)) => format!("{i}.0_f32"),
            other => panic!("google_design_tokens.toml expected number `{key}`, got {other:?}"),
        }
    };
    let hex_to_rgb_literal = |hex: &str| -> String {
        let clean = hex
            .trim()
            .strip_prefix('#')
            .unwrap_or_else(|| panic!("invalid hex color `{hex}` in google_design_tokens.toml"));
        assert!(
            clean.len() >= 6 && clean[..6].chars().all(|c| c.is_ascii_hexdigit()),
            "invalid hex color `{hex}`"
        );
        format!("0x{}_u32", clean[..6].to_ascii_uppercase())
    };
    let rust_str = |s: &str| -> String { format!("{s:?}") };

    let root_tbl = root
        .as_table()
        .expect("google_design_tokens.toml root must be a table");
    let schema_version = req_i64(root_tbl, "schema_version") as u32;
    let source_of_truth = req_str(root_tbl, "source_of_truth");
    let system_name = req_str(root_tbl, "system");
    let spec_url = req_str(root_tbl, "spec_url");
    let fonts_url = req_str(root_tbl, "fonts_url");
    let stitch_url = req_str(root_tbl, "stitch_url");
    let sans_flex_family = req_str(root_tbl, "sans_flex_family");
    let synced_at = req_str(root_tbl, "synced_at");
    let seed_hex = req_str(root_tbl, "seed_hex");
    let variant = req_str(root_tbl, "variant");
    let principles: Vec<String> = root_tbl
        .get("principles")
        .and_then(|v| v.as_array())
        .expect("missing principles array")
        .iter()
        .map(|v| rust_str(v.as_str().expect("principle entry must be string")))
        .collect();

    let ref_tbl = table(&root, "ref");
    let typeface_tbl = ref_tbl
        .get("typeface")
        .and_then(|v| v.as_table())
        .expect("missing [ref.typeface]");
    let ref_palette_tbl = ref_tbl
        .get("palette")
        .and_then(|v| v.as_table())
        .expect("missing [ref.palette]");
    let mut ref_palette_sorted: BTreeMap<&str, &str> = BTreeMap::new();
    for (k, v) in ref_palette_tbl {
        ref_palette_sorted.insert(k.as_str(), v.as_str().expect("ref.palette value must be string"));
    }

    let axes_tbl = ref_tbl
        .get("variable_axes")
        .and_then(|v| v.as_table())
        .expect("missing [ref.variable_axes]");
    let sys_tbl = table(&root, "sys");
    let shape_tbl = sys_tbl
        .get("shape")
        .and_then(|v| v.as_table())
        .expect("missing [sys.shape]");
    let state_tbl = sys_tbl
        .get("state")
        .and_then(|v| v.as_table())
        .expect("missing [sys.state]");
    let density_tbl = sys_tbl
        .get("density")
        .and_then(|v| v.as_table())
        .expect("missing [sys.density]");
    let window_tbl = sys_tbl
        .get("window_size_classes")
        .and_then(|v| v.as_table())
        .expect("missing [sys.window_size_classes]");
    let motion_tbl = sys_tbl
        .get("motion")
        .and_then(|v| v.as_table())
        .expect("missing [sys.motion]");
    let springs_tbl = motion_tbl
        .get("springs")
        .and_then(|v| v.as_table())
        .expect("missing [sys.motion.springs]");
    let typescale_tbl = sys_tbl
        .get("typescale")
        .and_then(|v| v.as_table())
        .expect("missing [sys.typescale]");
    let color_tbl = sys_tbl
        .get("color")
        .and_then(|v| v.as_table())
        .expect("missing [sys.color]");

    let mut out = String::with_capacity(16_384);
    let _ = writeln!(out, "pub const SCHEMA_VERSION: u32 = {schema_version};");
    let _ = writeln!(out, "pub const SOURCE_OF_TRUTH: &str = {};", rust_str(source_of_truth));
    let _ = writeln!(out, "pub const SYSTEM_NAME: &str = {};", rust_str(system_name));
    let _ = writeln!(out, "pub const SPEC_URL: &str = {};", rust_str(spec_url));
    let _ = writeln!(out, "pub const FONTS_URL: &str = {};", rust_str(fonts_url));
    let _ = writeln!(out, "pub const STITCH_URL: &str = {};", rust_str(stitch_url));
    let _ = writeln!(out, "pub const SANS_FLEX_FAMILY: &str = {};", rust_str(sans_flex_family));
    let _ = writeln!(out, "pub const SYNCED_AT: &str = {};", rust_str(synced_at));
    let _ = writeln!(out, "pub const SEED_HEX: &str = {};", rust_str(seed_hex));
    let _ = writeln!(out, "pub const VARIANT: &str = {};", rust_str(variant));
    let _ = writeln!(out, "pub const PRINCIPLES: &[&str] = &[{}];", principles.join(", "));
    let _ = writeln!(out, "pub const TYPEFACE_BRAND: &str = {};", rust_str(req_str(typeface_tbl, "brand")));
    let _ = writeln!(out, "pub const TYPEFACE_PLAIN: &str = {};", rust_str(req_str(typeface_tbl, "plain")));
    let _ = writeln!(out, "pub const TYPEFACE_MONO: &str = {};", rust_str(req_str(typeface_tbl, "mono")));
    let _ = writeln!(out, "pub const TYPEFACE_FALLBACK: &str = {};", rust_str(req_str(typeface_tbl, "fallback")));

    let _ = writeln!(out, "pub const REFERENCE_PALETTE: &[(&str, &str, u32)] = &[");
    for (k, hex) in &ref_palette_sorted {
        let _ = writeln!(out, "    ({}, {}, {}),", rust_str(k), rust_str(hex), hex_to_rgb_literal(hex));
    }
    let _ = writeln!(out, "];");

    let axis_order = ["opsz", "wght", "wdth", "GRAD", "slnt", "ROND"];
    let _ = writeln!(out, "pub const VARIABLE_AXES: &[AxisSpec; 6] = &[");
    for key in axis_order {
        let ax = axes_tbl
            .get(key)
            .and_then(|v| v.as_table())
            .unwrap_or_else(|| panic!("missing variable axis `{key}`"));
        let _ = writeln!(
            out,
            "    AxisSpec {{ tag: {}, name: {}, min: {}, default_value: {}, max: {} }},",
            rust_str(req_str(ax, "tag")),
            rust_str(req_str(ax, "name")),
            fmt_f32(ax, "min"),
            fmt_f32(ax, "default_value"),
            fmt_f32(ax, "max"),
        );
    }
    let _ = writeln!(out, "];");

    let _ = writeln!(
        out,
        "pub const SHAPES: ShapeScale = ShapeScale {{ none_px: {}, extra_small_px: {}, small_px: {}, medium_px: {}, large_px: {}, large_increased_px: {}, extra_large_px: {}, full_px: {} }};",
        fmt_f32(shape_tbl, "none_px"),
        fmt_f32(shape_tbl, "extra_small_px"),
        fmt_f32(shape_tbl, "small_px"),
        fmt_f32(shape_tbl, "medium_px"),
        fmt_f32(shape_tbl, "large_px"),
        fmt_f32(shape_tbl, "large_increased_px"),
        fmt_f32(shape_tbl, "extra_large_px"),
        fmt_f32(shape_tbl, "full_px"),
    );

    let _ = writeln!(
        out,
        "pub const STATES: StateLayers = StateLayers {{ hover_opacity: {}, focus_opacity: {}, pressed_opacity: {}, dragged_opacity: {}, disabled_container_opacity: {}, disabled_content_opacity: {}, focus_ring_px: {} }};",
        fmt_f32(state_tbl, "hover_opacity"),
        fmt_f32(state_tbl, "focus_opacity"),
        fmt_f32(state_tbl, "pressed_opacity"),
        fmt_f32(state_tbl, "dragged_opacity"),
        fmt_f32(state_tbl, "disabled_container_opacity"),
        fmt_f32(state_tbl, "disabled_content_opacity"),
        fmt_f32(state_tbl, "focus_ring_px"),
    );

    let _ = writeln!(
        out,
        "pub const DENSITY: DensityScale = DensityScale {{ default: {}_i32, compact_1: {}_i32, compact_2: {}_i32, compact_3: {}_i32, touch_target_min_px: {}, control_height_min_px: {} }};",
        req_i64(density_tbl, "default"),
        req_i64(density_tbl, "compact_1"),
        req_i64(density_tbl, "compact_2"),
        req_i64(density_tbl, "compact_3"),
        fmt_f32(density_tbl, "touch_target_min_px"),
        fmt_f32(density_tbl, "control_height_min_px"),
    );

    let _ = writeln!(
        out,
        "pub const WINDOW_SIZE_CLASSES: WindowSizeThresholds = WindowSizeThresholds {{ compact_max_px: {}, medium_min_px: {}, expanded_min_px: {}, large_min_px: {}, extra_large_min_px: {} }};",
        fmt_f32(window_tbl, "compact_max_px"),
        fmt_f32(window_tbl, "medium_min_px"),
        fmt_f32(window_tbl, "expanded_min_px"),
        fmt_f32(window_tbl, "large_min_px"),
        fmt_f32(window_tbl, "extra_large_min_px"),
    );

    let spring_order = [
        "spatial_fast",
        "spatial_default",
        "spatial_slow",
        "effects_fast",
        "effects_default",
        "effects_slow",
        "quire_unfold",
        "quire_settle",
        "quire_expand",
        "quire_fold",
        "quire_tick",
    ];
    let _ = writeln!(out, "pub const SPRINGS: &[SpringTokenSpec; 11] = &[");
    for key in spring_order {
        let sp = springs_tbl
            .get(key)
            .and_then(|v| v.as_table())
            .unwrap_or_else(|| panic!("missing spring `{key}`"));
        let _ = writeln!(
            out,
            "    SpringTokenSpec {{ key: {}, stiffness: {}, damping_ratio: {} }},",
            rust_str(key),
            fmt_f32(sp, "stiffness"),
            fmt_f32(sp, "damping_ratio"),
        );
    }
    let _ = writeln!(out, "];");

    let typescale_order = [
        "display_large",
        "display_medium",
        "display_small",
        "headline_large",
        "headline_medium",
        "headline_small",
        "title_large",
        "title_medium",
        "title_small",
        "body_large",
        "body_medium",
        "body_small",
        "label_large",
        "label_medium",
        "label_small",
    ];
    let fmt_ts = |key: &str| -> String {
        let ts = typescale_tbl
            .get(key)
            .and_then(|v| v.as_table())
            .unwrap_or_else(|| panic!("missing typescale `{key}`"));
        format!(
            "TypeScaleEntry {{ key: {}, font: {}, size_sp: {}, line_height_sp: {}, weight: {}, tracking_sp: {} }}",
            rust_str(key),
            rust_str(req_str(ts, "font")),
            fmt_f32(ts, "size_sp"),
            fmt_f32(ts, "line_height_sp"),
            fmt_f32(ts, "weight"),
            fmt_f32(ts, "tracking_sp"),
        )
    };
    let _ = writeln!(out, "pub const TYPESCALES: &[TypeScaleEntry; 15] = &[");
    for key in typescale_order {
        let _ = writeln!(out, "    {},", fmt_ts(key));
    }
    let _ = writeln!(out, "];");
    let _ = writeln!(out, "pub const TYPESCALE: TypeScaleSet = TypeScaleSet {{");
    for key in typescale_order {
        let _ = writeln!(out, "    {key}: {},", fmt_ts(key));
    }
    let _ = writeln!(out, "}};");

    let role_order = [
        "background",
        "on_background",
        "surface",
        "surface_dim",
        "surface_bright",
        "surface_container_lowest",
        "surface_container_low",
        "surface_container",
        "surface_container_high",
        "surface_container_highest",
        "on_surface",
        "on_surface_variant",
        "outline",
        "outline_variant",
        "primary",
        "on_primary",
        "primary_container",
        "on_primary_container",
        "secondary",
        "on_secondary",
        "secondary_container",
        "on_secondary_container",
        "tertiary",
        "on_tertiary",
        "tertiary_container",
        "on_tertiary_container",
        "error",
        "on_error",
        "error_container",
        "on_error_container",
    ];
    for (mode_key, const_suffix) in [
        ("light", "LIGHT"),
        ("dark", "DARK"),
        ("neural_obsidian", "NEURAL_OBSIDIAN"),
        ("oled_pitch", "OLED_PITCH"),
    ] {
        let mode_tbl = color_tbl
            .get(mode_key)
            .and_then(|v| v.as_table())
            .unwrap_or_else(|| panic!("missing [sys.color.{mode_key}]"));
        let _ = writeln!(out, "pub const PALETTE_{const_suffix}_HEX: ColorPaletteHex = ColorPaletteHex {{");
        for role in role_order {
            let _ = writeln!(out, "    {role}: {},", rust_str(req_str(mode_tbl, role)));
        }
        let _ = writeln!(out, "}};");
        let _ = writeln!(out, "pub const PALETTE_{const_suffix}_RGB: ColorPaletteRgb = ColorPaletteRgb {{");
        for role in role_order {
            let _ = writeln!(out, "    {role}: {},", hex_to_rgb_literal(req_str(mode_tbl, role)));
        }
        let _ = writeln!(out, "}};");
    }

    let comp_tbl = table(&root, "comp");
    let mut comp_entries = Vec::new();
    for (comp_name, comp_val) in comp_tbl {
        if let Some(fields) = comp_val.as_table() {
            for (field_name, field_val) in fields {
                let num_lit = match field_val {
                    toml::Value::Float(f) => {
                        let s = f.to_string();
                        if s.contains('.') { format!("{s}_f32") } else { format!("{s}.0_f32") }
                    }
                    toml::Value::Integer(i) => format!("{i}.0_f32"),
                    _ => continue,
                };
                comp_entries.push((comp_name.clone(), field_name.clone(), num_lit));
            }
        }
    }
    comp_entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let _ = writeln!(out, "pub const COMP_NUMBERS: &[(&str, &str, f32)] = &[");
    for (c, k, num_lit) in comp_entries {
        let _ = writeln!(out, "    ({}, {}, {num_lit}),", rust_str(&c), rust_str(&k));
    }
    let _ = writeln!(out, "];");

    fs::write(out_dir.join("design_tokens_constants.rs"), out)
        .expect("failed to write design_tokens_constants.rs");
}
