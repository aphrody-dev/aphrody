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
    let submodule_bun = get_str(&parsed, "git", "submodule_bun");

    let wgpu_cli_bin = get_str(&parsed, "features", "wgpu_cli_bin");

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
pub const SUBMODULE_BUN_REPO: &str = {submodule_bun};

pub const WGPU_CLI_BIN: &str = {wgpu_cli_bin};
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
        submodule_bun = rust_str_literal(submodule_bun),
        wgpu_cli_bin = rust_str_literal(wgpu_cli_bin),
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("product_constants.rs"), generated)
        .expect("failed to write product_constants.rs");
}
