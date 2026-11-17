use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppOrigin {
    XdgDesktop,
    WindowsStartMenu,
    MacOsBundle,
    ManifestCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub comment: String,
    pub categories: Vec<String>,
    pub origin: AppOrigin,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistFormat {
    PopDeb,
    Flatpak,
    AppImage,
    WindowsInstaller,
    CargoBundle,
}

impl DistFormat {
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "deb" | "pop_deb" | "pop-deb" => Some(Self::PopDeb),
            "flatpak" => Some(Self::Flatpak),
            "appimage" | "app_image" => Some(Self::AppImage),
            "windows" | "windows_installer" | "msi" | "nsis" => Some(Self::WindowsInstaller),
            "cargo" | "cargo_bundle" => Some(Self::CargoBundle),
            _ => None,
        }
    }
}

/// Pop!_OS / Debian / Flatpak / cross-platform packaging descriptor generated without hardcoded paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistBundleSpec {
    pub app_name: String,
    pub app_slug: String,
    pub app_id: String,
    pub version: String,
    pub homepage: String,
    pub repository: String,
    pub license: String,
    pub categories: Vec<String>,
    pub formats: Vec<DistFormat>,
}

impl DistBundleSpec {
    #[must_use]
    pub fn from_brand(version: impl Into<String>) -> Self {
        Self {
            app_name: brand::APP_NAME.to_owned(),
            app_slug: brand::APP_SLUG.to_owned(),
            app_id: brand::IDENTIFIER_STABLE.to_owned(),
            version: version.into(),
            homepage: brand::HOMEPAGE_URL.to_owned(),
            repository: brand::REPOSITORY_URL.to_owned(),
            license: "GPL-3.0-or-later".to_owned(),
            categories: vec![
                "Graphics".to_owned(),
                "Development".to_owned(),
                "Utility".to_owned(),
            ],
            formats: vec![
                DistFormat::PopDeb,
                DistFormat::Flatpak,
                DistFormat::CargoBundle,
                DistFormat::WindowsInstaller,
            ],
        }
    }

    #[must_use]
    pub fn render_debian_control(&self) -> String {
        format!(
            "Package: {slug}\nVersion: {version}\nSection: graphics\nPriority: optional\nArchitecture: amd64\nHomepage: {homepage}\nMaintainer: {company}\nDescription: {desc}\n",
            slug = self.app_slug,
            version = self.version,
            homepage = self.homepage,
            company = brand::COMPANY_NAME,
            desc = brand::DESCRIPTION,
        )
    }

    #[must_use]
    pub fn render_desktop_entry(&self, binary_name: &str) -> String {
        format!(
            "[Desktop Entry]\nType=Application\nName={name} OS\nComment={desc}\nExec={bin} %F\nIcon={slug}\nCategories={cats};\nTerminal=false\nStartupNotify=true\n",
            name = self.app_name,
            desc = brand::TAGLINE,
            bin = binary_name,
            slug = self.app_slug,
            cats = self.categories.join(";"),
        )
    }
}

/// Parses an XDG `.desktop` entry file per Freedesktop Desktop Entry Specification.
#[must_use]
pub fn parse_xdg_desktop_entry(path: &Path, content: &str) -> Option<AppEntry> {
    let mut in_desktop_entry = false;
    let mut name: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut comment = String::new();
    let mut categories = Vec::new();
    let mut no_display = false;
    let mut hidden = false;
    let mut entry_type = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_desktop_entry = trimmed == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, val)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let val = val.trim();
        match key {
            "Name" => name = Some(val.to_owned()),
            "Exec" => exec = Some(clean_desktop_exec(val)),
            "Comment" => comment = val.to_owned(),
            "Type" => entry_type = val.to_owned(),
            "NoDisplay" => no_display = val.eq_ignore_ascii_case("true"),
            "Hidden" => hidden = val.eq_ignore_ascii_case("true"),
            "Categories" => {
                categories = val
                    .split(';')
                    .filter(|c| !c.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
            _ => {}
        }
    }

    if no_display || hidden || (!entry_type.is_empty() && entry_type != "Application") {
        return None;
    }

    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("app")
        .to_owned();
    Some(AppEntry {
        id,
        name: name?,
        exec: exec?,
        comment,
        categories,
        origin: AppOrigin::XdgDesktop,
        source_path: path.to_path_buf(),
    })
}

fn clean_desktop_exec(raw: &str) -> String {
    raw.split_whitespace()
        .filter(|tok| !tok.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Discovers installed desktop applications dynamically from OS directories without fixed lists.
#[must_use]
pub fn discover_system_apps(limit: usize) -> Vec<AppEntry> {
    let mut entries = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for dir in platform_application_dirs() {
        if entries.len() >= limit {
            break;
        }
        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };
        for item in read_dir.flatten() {
            if entries.len() >= limit {
                break;
            }
            let path = item.path();
            if let Some(app) = inspect_platform_app_path(&path)
                && seen_ids.insert(app.id.clone())
            {
                entries.push(app);
            }
        }
    }

    entries.sort_by_key(|a| a.name.to_lowercase());
    entries
}

fn inspect_platform_app_path(path: &Path) -> Option<AppEntry> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)?;

    match ext.as_str() {
        "desktop" => {
            let text = fs::read_to_string(path).ok()?;
            parse_xdg_desktop_entry(path, &text)
        }
        "lnk" | "appref-ms" => {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(AppEntry {
                id: name.to_lowercase().replace(' ', "-"),
                name,
                exec: path.display().to_string(),
                comment: "Windows Start Menu application".to_owned(),
                categories: vec!["Application".to_owned()],
                origin: AppOrigin::WindowsStartMenu,
                source_path: path.to_path_buf(),
            })
        }
        "app" if path.is_dir() => {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(AppEntry {
                id: name.to_lowercase().replace(' ', "-"),
                name,
                exec: path.display().to_string(),
                comment: "macOS Application Bundle".to_owned(),
                categories: vec!["Application".to_owned()],
                origin: AppOrigin::MacOsBundle,
                source_path: path.to_path_buf(),
            })
        }
        _ => None,
    }
}

fn platform_application_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(data_local) = dirs::data_local_dir() {
        dirs.push(data_local.join("applications"));
    }

    if let Ok(xdg_dirs) = std::env::var("XDG_DATA_DIRS") {
        for base in xdg_dirs.split(':').filter(|s| !s.is_empty()) {
            dirs.push(PathBuf::from(base).join("applications"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            dirs.push(
                PathBuf::from(appdata)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs"),
            );
        }
        if let Ok(programdata) = std::env::var("ProgramData") {
            dirs.push(
                PathBuf::from(programdata)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/Applications"));
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join("Applications"));
        }
    }

    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_freedesktop_desktop_entry_and_dist_metadata() {
        let sample = "\
[Desktop Entry]
Type=Application
Name=COSMIC Files
Comment=Manage local and remote files
Exec=cosmic-files %U
Categories=Utility;FileManager;
";
        let entry = parse_xdg_desktop_entry(
            Path::new("/usr/share/applications/cosmic-files.desktop"),
            sample,
        )
        .expect("desktop entry parses");
        assert_eq!(entry.id, "cosmic-files");
        assert_eq!(entry.name, "COSMIC Files");
        assert_eq!(entry.exec, "cosmic-files");

        let spec = DistBundleSpec::from_brand("1.30.0");
        assert!(spec.render_debian_control().contains("Package: aphrody"));
        assert!(
            spec.render_desktop_entry("aphrody-os")
                .contains("Exec=aphrody-os %F")
        );
    }
}
