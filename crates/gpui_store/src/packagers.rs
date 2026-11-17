//! Pure-Rust deterministic manifest generators for all 22 supported store, distro,
//! mobile, browser extension, and container targets.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::{StoreEntry, StoreError, StoreTarget};

/// Single generated package manifest or recipe file relative to `<out>/<entry_id>/<target>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedArtifact {
    pub relative_path: String,
    pub content: String,
}

#[must_use]
pub fn slugify(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_dash = true;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "gpui-app".to_owned()
    } else {
        trimmed
    }
}

#[must_use]
pub fn entry_slug(entry: &StoreEntry) -> String {
    if !entry.id.trim().is_empty() {
        slugify(&entry.id)
    } else {
        slugify(&entry.crate_name)
    }
}

#[must_use]
pub fn pascal_identifier(raw: &str) -> String {
    let mut out = String::new();
    for part in raw.split(|c: char| !c.is_ascii_alphanumeric()) {
        if part.is_empty() {
            continue;
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.push(first.to_ascii_uppercase());
            for ch in chars {
                out.push(ch);
            }
        }
    }
    if out.is_empty() {
        return "GpuiApp".to_owned();
    }
    if out.as_bytes()[0].is_ascii_digit() {
        format!("App{out}")
    } else {
        out
    }
}

#[must_use]
pub fn sanitize_semver(raw: &str) -> String {
    let clean: String = raw
        .trim()
        .trim_start_matches('v')
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let parts: Vec<&str> = clean.split('.').filter(|s| !s.is_empty()).collect();
    match parts.as_slice() {
        [a, b, c, ..] => format!("{a}.{b}.{c}"),
        [a, b] => format!("{a}.{b}.0"),
        [a] => format!("{a}.0.0"),
        [] => "0.1.0".to_owned(),
    }
}

#[must_use]
pub fn winget_identifier(entry: &StoreEntry) -> String {
    let publisher = if entry.publisher_id.is_empty() {
        pascal_identifier(&entry.publisher)
    } else {
        entry.publisher_id.clone()
    };
    format!("{publisher}.{}", pascal_identifier(&entry.name))
}

#[must_use]
pub fn android_package_name(app_id: &str) -> String {
    let mut segs: Vec<String> = Vec::new();
    for seg in app_id.split('.') {
        let clean: String = seg
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if clean.is_empty() {
            continue;
        }
        if clean.as_bytes()[0].is_ascii_digit() {
            segs.push(format!("a{clean}"));
        } else {
            segs.push(clean);
        }
    }
    if segs.len() >= 2 {
        segs.join(".")
    } else {
        format!("org.gpui.{}", segs.first().map_or("app", String::as_str))
    }
}

fn xml_escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

fn json_quote(raw: &str) -> String {
    serde_json::to_string(raw).unwrap_or_else(|_| "\"\"".to_owned())
}

fn snap_summary(entry: &StoreEntry) -> String {
    let base = if entry.tagline.trim().is_empty() {
        entry.name.as_str()
    } else {
        entry.tagline.as_str()
    };
    let single_line: String = base.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= 78 {
        single_line
    } else {
        let prefix: String = single_line.chars().take(75).collect();
        format!("{}...", prefix.trim_end())
    }
}

fn extract_github_owner_repo(repo_url: &str) -> Option<(&str, &str)> {
    let trimmed = repo_url
        .strip_prefix("https://github.com/")
        .or_else(|| repo_url.strip_prefix("http://github.com/"))?;
    let mut parts = trimmed.split('/').filter(|s| !s.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?.trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        None
    } else {
        Some((owner, repo))
    }
}

fn artifact(relative_path: impl Into<String>, content: String) -> GeneratedArtifact {
    GeneratedArtifact {
        relative_path: relative_path.into(),
        content,
    }
}

/// Generate all manifests/build recipes for a single `(entry, target)` pair.
#[must_use]
pub fn generate_target(entry: &StoreEntry, target: StoreTarget) -> Vec<GeneratedArtifact> {
    let ver = sanitize_semver(&entry.version);
    let msix_ver = format!("{ver}.0");
    let pkg_slug = entry_slug(entry);
    let crate_name = if entry.crate_name.is_empty() {
        pkg_slug.as_str()
    } else {
        entry.crate_name.as_str()
    };
    let bin = if entry.bin_name.is_empty() {
        pkg_slug.as_str()
    } else {
        entry.bin_name.as_str()
    };
    let wg_id = winget_identifier(entry);
    let android_pkg = android_package_name(&entry.app_id);
    let android_lib = slugify(crate_name).replace('-', "_");
    let homepage = if entry.homepage.is_empty() {
        entry.repo.as_str()
    } else {
        entry.homepage.as_str()
    };
    let release_base = if entry.repo.starts_with("https://github.com/") {
        format!(
            "{}/releases/download/v{ver}",
            entry.repo.trim_end_matches('/')
        )
    } else {
        format!("{}/releases/v{ver}", homepage.trim_end_matches('/'))
    };
    let archive_url = format!(
        "{}/archive/refs/tags/v{ver}.tar.gz",
        entry.repo.trim_end_matches('/')
    );
    let zero_sha = "0".repeat(64);
    let gh_ref = extract_github_owner_repo(&entry.repo);

    match target {
        StoreTarget::WinGet => {
            generate_winget(entry, &ver, &wg_id, bin, homepage, &release_base, &zero_sha)
        }
        StoreTarget::MicrosoftStore => generate_msstore(entry, &msix_ver, bin),
        StoreTarget::Flatpak => generate_flatpak(entry, &ver, crate_name, bin, homepage),
        StoreTarget::PlayStore => {
            generate_playstore(entry, &ver, &android_pkg, &android_lib, crate_name)
        }
        StoreTarget::AppStore => generate_appstore(entry, &ver, bin),
        StoreTarget::UbuntuSnap => generate_snap(entry, &ver, &pkg_slug, bin),
        StoreTarget::ArchPacman => generate_arch(
            entry,
            &ver,
            &pkg_slug,
            crate_name,
            bin,
            homepage,
            &archive_url,
        ),
        StoreTarget::NixFlake => {
            generate_nix(entry, &ver, &pkg_slug, crate_name, bin, homepage, gh_ref)
        }
        StoreTarget::CargoCrate => {
            generate_cargo(entry, &ver, &pkg_slug, crate_name, bin, homepage)
        }
        StoreTarget::NpmPackage => generate_npm(entry, &ver, crate_name, bin, homepage),
        StoreTarget::ChromeWebStore => generate_chrome(entry, &ver, homepage),
        StoreTarget::AlpineApk => generate_apk(
            entry,
            &ver,
            &pkg_slug,
            crate_name,
            bin,
            homepage,
            &archive_url,
        ),
        StoreTarget::FedoraRpm => generate_rpm(
            entry,
            &ver,
            &pkg_slug,
            crate_name,
            bin,
            homepage,
            &archive_url,
        ),
        StoreTarget::DebianDeb => generate_deb(entry, &ver, &pkg_slug, crate_name, bin, homepage),
        StoreTarget::VoidXbps => generate_xbps(
            entry,
            &ver,
            &pkg_slug,
            crate_name,
            bin,
            homepage,
            &archive_url,
            &zero_sha,
        ),
        StoreTarget::GentooEmerge => generate_gentoo(
            entry,
            &ver,
            &pkg_slug,
            crate_name,
            bin,
            homepage,
            &archive_url,
        ),
        StoreTarget::FreeBsdPkg => {
            generate_freebsd(entry, &ver, &pkg_slug, crate_name, bin, homepage, gh_ref)
        }
        StoreTarget::Homebrew => generate_brew(
            entry,
            &ver,
            &pkg_slug,
            crate_name,
            bin,
            homepage,
            &archive_url,
        ),
        StoreTarget::Scoop => generate_scoop(
            entry,
            &ver,
            &pkg_slug,
            bin,
            homepage,
            &release_base,
            &zero_sha,
            gh_ref.is_some(),
        ),
        StoreTarget::Chocolatey => generate_choco(
            entry,
            &ver,
            &pkg_slug,
            bin,
            homepage,
            &release_base,
            &zero_sha,
        ),
        StoreTarget::AppImage => generate_appimage(entry, &ver, &pkg_slug, bin),
        StoreTarget::DockerOci => generate_oci(entry, &ver, crate_name, bin, homepage),
    }
}

/// Generate manifests for all 22 targets of a [`StoreEntry`].
#[must_use]
pub fn generate_all_targets(entry: &StoreEntry) -> Vec<(StoreTarget, Vec<GeneratedArtifact>)> {
    StoreTarget::ALL
        .into_iter()
        .map(|target| (target, generate_target(entry, target)))
        .collect()
}

/// Export generated manifests to `<out_dir>/<entry.id>/<target>/<relative_path>`.
pub fn export_entry_to_dir(
    entry: &StoreEntry,
    targets: &[StoreTarget],
    out_dir: &Path,
) -> Result<Vec<PathBuf>, StoreError> {
    let mut written = Vec::new();
    for &target in targets {
        for art in generate_target(entry, target) {
            let dest = out_dir
                .join(&entry.id)
                .join(target.slug())
                .join(&art.relative_path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest, &art.content)?;
            written.push(dest);
        }
    }
    Ok(written)
}

fn generate_winget(
    entry: &StoreEntry,
    ver: &str,
    wg_id: &str,
    bin: &str,
    homepage: &str,
    release_base: &str,
    zero_sha: &str,
) -> Vec<GeneratedArtifact> {
    let version_yaml = format!(
        "PackageIdentifier: {wg_id}\nPackageVersion: {}\nDefaultLocale: en-US\nManifestType: version\nManifestVersion: 1.9.0\n",
        json_quote(ver)
    );
    let installer_yaml = format!(
        "PackageIdentifier: {wg_id}\nPackageVersion: {ver_q}\nInstallerType: zip\nPlatform:\n  - Windows.Desktop\nMinimumOSVersion: 10.0.19041.0\nInstallers:\n  - Architecture: x64\n    InstallerUrl: {x64_url}\n    InstallerSha256: {zero_sha}\n    NestedInstallerType: portable\n    NestedInstallerFiles:\n      - RelativeFilePath: {exe_q}\n        PortableCommandAlias: {bin_q}\n  - Architecture: arm64\n    InstallerUrl: {arm_url}\n    InstallerSha256: {zero_sha}\n    NestedInstallerType: portable\n    NestedInstallerFiles:\n      - RelativeFilePath: {exe_q}\n        PortableCommandAlias: {bin_q}\nManifestType: installer\nManifestVersion: 1.9.0\n",
        ver_q = json_quote(ver),
        x64_url = json_quote(&format!("{release_base}/{bin}-windows-x64.zip")),
        arm_url = json_quote(&format!("{release_base}/{bin}-windows-arm64.zip")),
        exe_q = json_quote(&format!("{bin}.exe")),
        bin_q = json_quote(bin),
    );
    let mut tags_yaml = String::new();
    for tag in entry.tags.iter().take(8) {
        let _ = writeln!(tags_yaml, "  - {}", json_quote(tag));
    }
    let locale_yaml = format!(
        "PackageIdentifier: {wg_id}\nPackageVersion: {ver_q}\nPackageLocale: en-US\nPublisher: {pub_q}\nPublisherUrl: {home_q}\nPackageName: {name_q}\nPackageUrl: {repo_q}\nLicense: {lic_q}\nShortDescription: {short_q}\nDescription: {desc_q}\nTags:\n{tags_yaml}ManifestType: defaultLocale\nManifestVersion: 1.9.0\n",
        ver_q = json_quote(ver),
        pub_q = json_quote(&entry.publisher),
        home_q = json_quote(homepage),
        name_q = json_quote(&entry.name),
        repo_q = json_quote(&entry.repo),
        lic_q = json_quote(&entry.license),
        short_q = json_quote(&entry.tagline),
        desc_q = json_quote(&entry.description),
    );
    vec![
        artifact(format!("{wg_id}.yaml"), version_yaml),
        artifact(format!("{wg_id}.installer.yaml"), installer_yaml),
        artifact(format!("{wg_id}.locale.en-US.yaml"), locale_yaml),
    ]
}

fn generate_msstore(entry: &StoreEntry, msix_ver: &str, bin: &str) -> Vec<GeneratedArtifact> {
    let app_id_attr = pascal_identifier(bin);
    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap rescap">
  <Identity
    Name="{name}"
    Publisher="CN={publisher}"
    Version="{msix_ver}"
    ProcessorArchitecture="x64" />
  <Properties>
    <DisplayName>{display_name}</DisplayName>
    <PublisherDisplayName>{publisher}</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
    <Description>{description}</Description>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0" />
  </Dependencies>
  <Resources>
    <Resource Language="en-us" />
  </Resources>
  <Applications>
    <Application
      Id="{app_id_attr}"
      Executable="{bin_exe}"
      EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements
        DisplayName="{display_name}"
        Description="{tagline}"
        BackgroundColor="transparent"
        Square150x150Logo="Assets\Square150x150Logo.png"
        Square44x44Logo="Assets\Square44x44Logo.png">
        <uap:DefaultTile Wide310x150Logo="Assets\Wide310x150Logo.png" />
      </uap:VisualElements>
    </Application>
  </Applications>
  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
  </Capabilities>
</Package>
"#,
        name = xml_escape(&entry.app_id),
        publisher = xml_escape(&entry.publisher),
        display_name = xml_escape(&entry.name),
        description = xml_escape(&entry.description),
        bin_exe = xml_escape(&format!("{bin}.exe")),
        tagline = xml_escape(&entry.tagline),
    );
    vec![artifact("AppxManifest.xml", xml)]
}

fn generate_flatpak(
    entry: &StoreEntry,
    ver: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
) -> Vec<GeneratedArtifact> {
    let manifest = format!(
        "id: {id_q}
runtime: org.freedesktop.Platform
runtime-version: \"24.08\"
sdk: org.freedesktop.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
  - org.freedesktop.Sdk.Extension.llvm19
command: {bin_q}
finish-args:
  - --share=ipc
  - --socket=fallback-x11
  - --socket=wayland
  - --device=dri
  - --filesystem=home
  - --share=network
modules:
  - name: {bin_q}
    buildsystem: simple
    build-options:
      append-path: /usr/lib/sdk/rust-stable/bin:/usr/lib/sdk/llvm19/bin
      env:
        CARGO_HOME: /run/build/{bin}/cargo
    build-commands:
      - cargo build --release --locked --package {crate_name}
      - install -Dm755 target/release/{bin} /app/bin/{bin}
      - install -Dm644 {app_id}.metainfo.xml /app/share/metainfo/{app_id}.metainfo.xml
    sources:
      - type: git
        url: {git_q}
        tag: {tag_q}
",
        id_q = json_quote(&entry.app_id),
        bin_q = json_quote(bin),
        app_id = entry.app_id,
        git_q = json_quote(&format!("{}.git", entry.repo)),
        tag_q = json_quote(&format!("v{ver}")),
    );
    let metainfo = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>{app_id}</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>{license}</project_license>
  <name>{name}</name>
  <summary>{summary}</summary>
  <description>
    <p>{description}</p>
  </description>
  <launchable type="desktop-id">{desktop_id}</launchable>
  <url type="homepage">{homepage}</url>
  <url type="vcs-browser">{repo}</url>
  <developer id="{app_id}">
    <name>{publisher}</name>
  </developer>
  <releases>
    <release version="{ver}" date="2026-11-16" />
  </releases>
  <content_rating type="oars-1.1" />
</component>
"#,
        app_id = xml_escape(&entry.app_id),
        license = xml_escape(&entry.license),
        name = xml_escape(&entry.name),
        summary = xml_escape(&entry.tagline),
        description = xml_escape(&entry.description),
        desktop_id = xml_escape(&format!("{}.desktop", entry.app_id)),
        homepage = xml_escape(homepage),
        repo = xml_escape(&entry.repo),
        publisher = xml_escape(&entry.publisher),
    );
    vec![
        artifact(format!("{}.yml", entry.app_id), manifest),
        artifact(format!("{}.metainfo.xml", entry.app_id), metainfo),
    ]
}

fn generate_playstore(
    entry: &StoreEntry,
    ver: &str,
    android_pkg: &str,
    android_lib: &str,
    crate_name: &str,
) -> Vec<GeneratedArtifact> {
    let manifest = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{pkg}"
    android:versionCode="1"
    android:versionName="{ver}">
    <uses-sdk android:minSdkVersion="26" android:targetSdkVersion="35" />
    <uses-feature android:glEsVersion="0x00030002" android:required="true" />
    <uses-feature android:name="android.hardware.vulkan.version" android:version="0x00401000" android:required="false" />
    <uses-permission android:name="android.permission.INTERNET" />
    <application
        android:label="{label}"
        android:hasCode="true"
        android:hardwareAccelerated="true">
        <activity
            android:name="android.app.NativeActivity"
            android:exported="true"
            android:configChanges="orientation|keyboardHidden|screenSize|screenLayout|smallestScreenSize|uiMode">
            <meta-data android:name="android.app.lib_name" android:value="{lib_name}" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#,
        pkg = xml_escape(android_pkg),
        ver = xml_escape(ver),
        label = xml_escape(&entry.name),
        lib_name = xml_escape(android_lib),
    );
    let gradle = format!(
        r#"plugins {{
    id("com.android.application")
}}

android {{
    namespace = "{android_pkg}"
    compileSdk = 35
    ndkVersion = "27.2.12479018"

    defaultConfig {{
        applicationId = "{android_pkg}"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "{ver}"
        ndk {{
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }}
    }}

    buildTypes {{
        release {{
            isMinifyEnabled = true
            isShrinkResources = true
        }}
    }}
}}

val buildCargoNdk by tasks.registering(Exec::class) {{
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "armeabi-v7a",
        "-t", "x86_64",
        "-o", "src/main/jniLibs",
        "build", "--release", "--package", "{crate_name}"
    )
}}

tasks.named("preBuild") {{
    dependsOn(buildCargoNdk)
}}
"#
    );
    vec![
        artifact("AndroidManifest.xml", manifest),
        artifact("build.gradle.kts", gradle),
    ]
}

fn generate_appstore(entry: &StoreEntry, ver: &str, bin: &str) -> Vec<GeneratedArtifact> {
    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>{name}</string>
  <key>CFBundleExecutable</key>
  <string>{bin_esc}</string>
  <key>CFBundleIdentifier</key>
  <string>{app_id}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>{name}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>{ver}</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>14.0</string>
  <key>MinimumOSVersion</key>
  <string>17.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>UIRequiredDeviceCapabilities</key>
  <array>
    <string>arm64</string>
    <string>metal</string>
  </array>
</dict>
</plist>
"#,
        name = xml_escape(&entry.name),
        bin_esc = xml_escape(bin),
        app_id = xml_escape(&entry.app_id),
        ver = xml_escape(ver),
    );
    let entitlements = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key>
  <true/>
  <key>com.apple.security.network.client</key>
  <true/>
  <key>com.apple.security.files.user-selected.read-write</key>
  <true/>
  <key>com.apple.security.cs.allow-jit</key>
  <false/>
</dict>
</plist>
"#
    .to_owned();
    let export_options = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>method</key>
  <string>app-store-connect</string>
  <key>signingStyle</key>
  <string>automatic</string>
  <key>destination</key>
  <string>export</string>
  <key>stripSwiftSymbols</key>
  <true/>
  <key>uploadSymbols</key>
  <true/>
</dict>
</plist>
"#
    .to_owned();
    vec![
        artifact("Info.plist", info_plist),
        artifact(
            format!("{}.entitlements", pascal_identifier(bin)),
            entitlements,
        ),
        artifact("ExportOptions.plist", export_options),
    ]
}

fn generate_snap(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    bin: &str,
) -> Vec<GeneratedArtifact> {
    let desc = entry.description.replace(['\r', '\n'], " ");
    let yaml = format!(
        "name: {pkg_slug}\nbase: core24\nversion: {ver_q}\nsummary: {sum_q}\ndescription: |\n  {desc}\ngrade: stable\nconfinement: strict\nlicense: {lic_q}\nplatforms:\n  amd64:\n  arm64:\napps:\n  {pkg_slug}:\n    command: bin/{bin}\n    plugs:\n      - desktop\n      - desktop-legacy\n      - wayland\n      - x11\n      - opengl\n      - network\n      - home\nparts:\n  {pkg_slug}:\n    plugin: rust\n    source: {src_q}\n    rust-channel: stable\n",
        ver_q = json_quote(ver),
        sum_q = json_quote(&snap_summary(entry)),
        lic_q = json_quote(&entry.license),
        src_q = json_quote(&format!("{}.git", entry.repo)),
    );
    vec![artifact("snap/snapcraft.yaml", yaml)]
}

fn generate_arch(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
    archive_url: &str,
) -> Vec<GeneratedArtifact> {
    let pkgbuild = format!(
        "# Maintainer: {pub_name}\npkgname={pkg_slug}\npkgver={ver}\npkgrel=1\npkgdesc={desc_q}\narch=('x86_64' 'aarch64')\nurl={url_q}\nlicense=({lic_q})\ndepends=('gcc-libs' 'vulkan-icd-loader' 'wayland' 'libxkbcommon')\nmakedepends=('cargo' 'git' 'clang' 'lld')\nsource=(\"$pkgname-$pkgver.tar.gz::{archive_url}\")\nsha256sums=('SKIP')\n\nprepare() {{\n  cd \"$srcdir/$pkgname-$pkgver\"\n  export RUSTUP_TOOLCHAIN=stable\n  cargo fetch --locked --target \"$(rustc -vV | sed -n 's/host: //p')\"\n}}\n\nbuild() {{\n  cd \"$srcdir/$pkgname-$pkgver\"\n  export RUSTUP_TOOLCHAIN=stable\n  export CARGO_TARGET_DIR=target\n  cargo build --frozen --release --package {crate_name}\n}}\n\npackage() {{\n  cd \"$srcdir/$pkgname-$pkgver\"\n  install -Dm755 \"target/release/{bin}\" \"$pkgdir/usr/bin/{bin}\"\n}}\n",
        pub_name = entry.publisher,
        desc_q = json_quote(&entry.tagline),
        url_q = json_quote(homepage),
        lic_q = json_quote(&entry.license),
    );
    let srcinfo = format!(
        "pkgbase = {pkg_slug}\n\tpkgdesc = {tagline}\n\tpkgver = {ver}\n\tpkgrel = 1\n\turl = {homepage}\n\tarch = x86_64\n\tarch = aarch64\n\tlicense = {lic}\n\tmakedepends = cargo\n\tmakedepends = git\n\tmakedepends = clang\n\tmakedepends = lld\n\tdepends = gcc-libs\n\tdepends = vulkan-icd-loader\n\tdepends = wayland\n\tdepends = libxkbcommon\n\tsource = {pkg_slug}-{ver}.tar.gz::{archive_url}\n\tsha256sums = SKIP\n\npkgname = {pkg_slug}\n",
        tagline = entry.tagline,
        lic = entry.license,
    );
    vec![
        artifact("PKGBUILD", pkgbuild),
        artifact(".SRCINFO", srcinfo),
    ]
}

fn generate_nix(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
    gh_ref: Option<(&str, &str)>,
) -> Vec<GeneratedArtifact> {
    let src_expr = match gh_ref {
        Some((owner, repo)) => format!(
            "pkgs.fetchFromGitHub {{\n            owner = {owner_q};\n            repo = {repo_q};\n            rev = {rev_q};\n            hash = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\";\n          }}",
            owner_q = json_quote(owner),
            repo_q = json_quote(repo),
            rev_q = json_quote(&format!("v{ver}")),
        ),
        None => "./.".to_owned(),
    };
    let flake = format!(
        "{{\n  description = {desc_q};\n\n  inputs = {{\n    nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";\n    flake-utils.url = \"github:numtide/flake-utils\";\n  }};\n\n  outputs = {{ self, nixpkgs, flake-utils }}:\n    flake-utils.lib.eachDefaultSystem (system:\n      let\n        pkgs = import nixpkgs {{ inherit system; }};\n      in\n      {{\n        packages.default = pkgs.rustPlatform.buildRustPackage {{\n          pname = {pname_q};\n          version = {ver_q};\n          src = {src_expr};\n          cargoLock.lockFile = ./Cargo.lock;\n          cargoBuildFlags = [ \"--package\" {crate_q} ];\n          nativeBuildInputs = with pkgs; [ pkg-config cmake ];\n          buildInputs = with pkgs; [\n            vulkan-loader\n            wayland\n            libxkbcommon\n            libGL\n            fontconfig\n          ];\n          meta = with pkgs.lib; {{\n            description = {tagline_q};\n            homepage = {home_q};\n            mainProgram = {bin_q};\n            platforms = platforms.linux ++ platforms.darwin;\n          }};\n        }};\n\n        apps.default = flake-utils.lib.mkApp {{\n          drv = self.packages.${{system}}.default;\n        }};\n\n        devShells.default = pkgs.mkShell {{\n          inputsFrom = [ self.packages.${{system}}.default ];\n          packages = with pkgs; [ rustc cargo rustfmt clippy ];\n        }};\n      }});\n}}\n",
        desc_q = json_quote(&format!("{} — {}", entry.name, entry.tagline)),
        pname_q = json_quote(pkg_slug),
        ver_q = json_quote(ver),
        crate_q = json_quote(crate_name),
        tagline_q = json_quote(&entry.tagline),
        home_q = json_quote(homepage),
        bin_q = json_quote(bin),
    );
    vec![artifact("flake.nix", flake)]
}

fn generate_cargo(
    entry: &StoreEntry,
    ver: &str,
    _pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
) -> Vec<GeneratedArtifact> {
    let cargo_toml = format!(
        "[package]\nname = {name_q}\nversion = {ver_q}\nedition = \"2024\"\nlicense = {lic_q}\ndescription = {desc_q}\nhomepage = {home_q}\nrepository = {repo_q}\nkeywords = [\"gpui\", \"nsh\", \"webgpu\", \"ui\"]\ncategories = [\"gui\"]\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n\n[[bin]]\nname = {bin_q}\npath = \"src/main.rs\"\n\n[dependencies]\ngpui = \"0.2\"\nnsh-core = \"0.5\"\nserde = {{ version = \"1.0\", features = [\"derive\"] }}\n\n[package.metadata.deb]\nmaintainer = {pub_q}\nsection = \"utils\"\ndepends = \"$auto\"\nassets = [\n  [\"target/release/{bin}\", \"usr/bin/{bin}\", \"755\"],\n]\n\n[package.metadata.generate-rpm]\nassets = [\n  {{ source = \"target/release/{bin}\", dest = \"/usr/bin/{bin}\", mode = \"755\" }},\n]\n\n[profile.web]\ninherits = \"release\"\nopt-level = \"z\"\nlto = \"thin\"\ncodegen-units = 1\npanic = \"abort\"\n",
        name_q = json_quote(crate_name),
        ver_q = json_quote(ver),
        lic_q = json_quote(&entry.license),
        desc_q = json_quote(&entry.tagline),
        home_q = json_quote(homepage),
        repo_q = json_quote(&entry.repo),
        bin_q = json_quote(bin),
        pub_q = json_quote(&entry.publisher),
    );
    vec![artifact("Cargo.toml", cargo_toml)]
}

fn generate_npm(
    entry: &StoreEntry,
    ver: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
) -> Vec<GeneratedArtifact> {
    let pkg_name = if entry.npm_package.is_empty() {
        format!("@gpui-store/{}", entry_slug(entry))
    } else {
        entry.npm_package.clone()
    };
    let manifest = json!({
        "name": pkg_name,
        "version": ver,
        "description": entry.tagline,
        "license": entry.license,
        "homepage": homepage,
        "repository": {
            "type": "git",
            "url": format!("git+{}.git", entry.repo),
        },
        "type": "module",
        "bin": {
            bin: format!("./bin/{bin}.js"),
        },
        "exports": {
            ".": "./dist/index.js",
            "./wasm": "./pkg/index.js",
            "./nsh": "./src/App.tsx",
        },
        "files": ["dist/", "pkg/", "bin/", "nsh.json"],
        "scripts": {
            "build:wasm": format!("wasm-pack build --target web --out-dir pkg -- --package {crate_name}"),
            "dev:nsh": "nsh dev src/App.tsx",
        },
        "keywords": entry.tags,
        "engines": {
            "node": ">=20",
            "bun": ">=1.2",
        },
        "nsh": {
            "appId": entry.app_id,
            "entry": "src/App.tsx",
            "targets": ["web", "desktop", "mobile"],
        },
    });
    let mut rendered = serde_json::to_string_pretty(&manifest).unwrap_or_default();
    rendered.push('\n');
    vec![artifact("package.json", rendered)]
}

fn generate_chrome(entry: &StoreEntry, ver: &str, homepage: &str) -> Vec<GeneratedArtifact> {
    let short_name: String = entry.name.chars().take(45).collect();
    let short_desc: String = entry.tagline.chars().take(132).collect();
    let manifest = json!({
        "manifest_version": 3,
        "name": short_name,
        "version": ver,
        "description": short_desc,
        "homepage_url": homepage,
        "action": {
            "default_title": entry.name,
            "default_popup": "popup.html",
        },
        "background": {
            "service_worker": "service_worker.js",
            "type": "module",
        },
        "permissions": ["storage", "activeTab"],
        "content_security_policy": {
            "extension_pages": "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'",
        },
        "web_accessible_resources": [
            {
                "resources": ["pkg/*.wasm", "pkg/*.js"],
                "matches": ["<all_urls>"],
            }
        ],
    });
    let mut manifest_str = serde_json::to_string_pretty(&manifest).unwrap_or_default();
    manifest_str.push('\n');

    let popup_html = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\" />\n  <title>{name}</title>\n  <style>body{{margin:0;width:420px;height:540px;background:#0b1020;color:#e2e8f0;font-family:system-ui,sans-serif;}}canvas{{width:100%;height:100%;display:block;}}</style>\n</head>\n<body>\n  <canvas id=\"gpui-canvas\" aria-label=\"{name} WebGPU Surface\"></canvas>\n  <script type=\"module\" src=\"./host.js\"></script>\n</body>\n</html>\n",
        name = xml_escape(&entry.name)
    );
    let host_js = format!(
        "export async function mountWebGpuExtension(canvasId = \"gpui-canvas\") {{\n  const canvas = document.getElementById(canvasId);\n  const hasWebGpu = typeof navigator !== \"undefined\" && \"gpu\" in navigator;\n  if (!canvas || !hasWebGpu) return {{ mounted: false, webgpu: hasWebGpu }};\n  const adapter = await navigator.gpu.requestAdapter();\n  return {{ mounted: Boolean(adapter), appId: {}, wasmTarget: \"wasm32-unknown-unknown\" }};\n}}\nmountWebGpuExtension();\n",
        json_quote(&entry.app_id)
    );
    let sw_js = format!(
        "chrome.runtime.onInstalled.addListener(() => {{\n  console.info(\"Mounted GPUI WebGPU extension\", {});\n}});\n",
        json_quote(&entry.app_id)
    );
    vec![
        artifact("manifest.json", manifest_str),
        artifact("popup.html", popup_html),
        artifact("host.js", host_js),
        artifact("service_worker.js", sw_js),
    ]
}

fn generate_apk(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
    archive_url: &str,
) -> Vec<GeneratedArtifact> {
    let apkbuild = format!(
        "# Maintainer: {pub_name}\npkgname={pkg_slug}\npkgver={ver}\npkgrel=0\npkgdesc={desc_q}\nurl={home_q}\narch=\"x86_64 aarch64\"\nlicense={lic_q}\ndepends=\"vulkan-loader wayland-libs-client libxkbcommon fontconfig\"\nmakedepends=\"cargo clang lld wayland-dev libxkbcommon-dev vulkan-loader-dev fontconfig-dev\"\nsource=\"$pkgname-$pkgver.tar.gz::{archive_url}\"\nbuilddir=\"$srcdir/$pkgname-$pkgver\"\n\nprepare() {{\n\tdefault_prepare\n\tcargo fetch --locked\n}}\n\nbuild() {{\n\tcargo build --release --frozen --package {crate_name}\n}}\n\ncheck() {{\n\tcargo test --release --frozen --package {crate_name} --lib\n}}\n\npackage() {{\n\tinstall -Dm755 \"target/release/{bin}\" \"$pkgdir\"/usr/bin/{bin}\n}}\n",
        pub_name = entry.publisher,
        desc_q = json_quote(&entry.tagline),
        home_q = json_quote(homepage),
        lic_q = json_quote(&entry.license),
    );
    vec![artifact("APKBUILD", apkbuild)]
}

fn generate_rpm(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
    archive_url: &str,
) -> Vec<GeneratedArtifact> {
    let spec = format!(
        "Name:           {pkg_slug}\nVersion:        {ver}\nRelease:        1%{{?dist}}\nSummary:        {tagline}\nLicense:        {lic}\nURL:            {homepage}\nSource0:        {archive_url}\n\nBuildRequires:  cargo\nBuildRequires:  rust\nBuildRequires:  clang\nBuildRequires:  lld\nBuildRequires:  wayland-devel\nBuildRequires:  libxkbcommon-devel\nBuildRequires:  vulkan-loader-devel\nRequires:       vulkan-loader\nRequires:       wayland\nRequires:       libxkbcommon\n\n%description\n{desc}\n\n%prep\n%autosetup -n %{{name}}-%{{version}}\n\n%build\ncargo build --release --locked --package {crate_name}\n\n%install\ninstall -Dm0755 target/release/{bin} %{{buildroot}}%{{_bindir}}/{bin}\n\n%files\n%{{_bindir}}/{bin}\n\n%changelog\n* Mon Nov 16 2026 {pub_name} - {ver}-1\n- Initial GPUI Store RPM package for {name}\n",
        tagline = entry.tagline,
        lic = entry.license,
        desc = entry.description,
        pub_name = entry.publisher,
        name = entry.name,
    );
    let rpm_meta = format!(
        "[package]\nname = {crate_q}\nversion = {ver_q}\nlicense = {lic_q}\ndescription = {desc_q}\n\n[package.metadata.generate-rpm]\nassets = [\n  {{ source = \"target/release/{bin}\", dest = \"/usr/bin/{bin}\", mode = \"755\" }},\n]\n",
        crate_q = json_quote(crate_name),
        ver_q = json_quote(ver),
        lic_q = json_quote(&entry.license),
        desc_q = json_quote(&entry.tagline),
    );
    vec![
        artifact(format!("{pkg_slug}.spec"), spec),
        artifact("rpm-metadata.toml", rpm_meta),
    ]
}

fn generate_deb(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
) -> Vec<GeneratedArtifact> {
    let desc_line = entry.description.replace(['\r', '\n'], " ");
    let control = format!(
        "Package: {pkg_slug}\nVersion: {ver}-1\nSection: utils\nPriority: optional\nArchitecture: amd64\nMaintainer: {pub_name}\nHomepage: {homepage}\nDepends: libc6 (>= 2.34), libwayland-client0, libxkbcommon0, libvulkan1\nDescription: {tagline}\n {desc_line}\n",
        pub_name = entry.publisher,
        tagline = entry.tagline,
    );
    let deb_meta = format!(
        "[package]\nname = {crate_q}\nversion = {ver_q}\nlicense = {lic_q}\ndescription = {desc_q}\n\n[package.metadata.deb]\nmaintainer = {pub_q}\nsection = \"utils\"\ndepends = \"$auto\"\nassets = [\n  [\"target/release/{bin}\", \"usr/bin/{bin}\", \"755\"],\n]\n",
        crate_q = json_quote(crate_name),
        ver_q = json_quote(ver),
        lic_q = json_quote(&entry.license),
        desc_q = json_quote(&entry.tagline),
        pub_q = json_quote(&entry.publisher),
    );
    vec![
        artifact("DEBIAN/control", control),
        artifact("deb-metadata.toml", deb_meta),
    ]
}

fn generate_xbps(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
    archive_url: &str,
    zero_sha: &str,
) -> Vec<GeneratedArtifact> {
    let template = format!(
        "# Template file for '{pkg_slug}'\npkgname={pkg_slug}\nversion={ver}\nrevision=1\nbuild_style=cargo\nconfigure_args=\"--package {crate_name}\"\nhostmakedepends=\"pkg-config clang lld\"\nmakedepends=\"wayland-devel libxkbcommon-devel vulkan-loader-devel fontconfig-devel\"\nshort_desc={desc_q}\nmaintainer={pub_q}\nlicense={lic_q}\nhomepage={home_q}\ndistfiles=\"{archive_url}\"\nchecksum=\"{zero_sha}\"\n\npost_install() {{\n\tvbin \"target/${{RUST_TARGET}}/release/{bin}\" \"{bin}\"\n}}\n",
        desc_q = json_quote(&entry.tagline),
        pub_q = json_quote(&entry.publisher),
        lic_q = json_quote(&entry.license),
        home_q = json_quote(homepage),
    );
    vec![artifact(format!("srcpkgs/{pkg_slug}/template"), template)]
}

fn generate_gentoo(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
    archive_url: &str,
) -> Vec<GeneratedArtifact> {
    let ebuild = format!(
        "# Copyright 2026 {pub_name}\n# Distributed under the terms of the {lic} license\n\nEAPI=8\n\ninherit cargo\n\nDESCRIPTION={desc_q}\nHOMEPAGE={home_q}\nSRC_URI=\"{archive_url} -> ${{P}}.tar.gz\"\n\nLICENSE={lic_q}\nSLOT=\"0\"\nKEYWORDS=\"~amd64 ~arm64\"\n\nRDEPEND=\"\n\tdev-libs/wayland\n\tx11-libs/libxkbcommon\n\tmedia-libs/vulkan-loader\n\tmedia-libs/fontconfig\n\"\nDEPEND=\"${{RDEPEND}}\"\nBDEPEND=\"\n\tvirtual/rust\n\tsys-devel/clang\n\"\n\nsrc_compile() {{\n\tcargo_src_compile --package {crate_name}\n}}\n\nsrc_install() {{\n\tdobin \"target/release/{bin}\"\n}}\n",
        pub_name = entry.publisher,
        lic = entry.license,
        desc_q = json_quote(&entry.tagline),
        home_q = json_quote(homepage),
        lic_q = json_quote(&entry.license),
    );
    vec![artifact(format!("{pkg_slug}-{ver}.ebuild"), ebuild)]
}

fn generate_freebsd(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
    gh_ref: Option<(&str, &str)>,
) -> Vec<GeneratedArtifact> {
    let (gh_owner, gh_repo) = gh_ref.unwrap_or(("aphrody-dev", pkg_slug));
    let makefile = format!(
        "PORTNAME=\t{pkg_slug}\nDISTVERSIONPREFIX=\tv\nDISTVERSION=\t{ver}\nCATEGORIES=\tx11 devel\nMAINTAINER=\tports@aphrody.com\nCOMMENT=\t{tagline}\nWWW=\t\t{homepage}\n\nLICENSE=\t{lic}\n\nLIB_DEPENDS=\tlibvulkan.so:graphics/vulkan-loader \\\n\t\tlibwayland-client.so:graphics/wayland \\\n\t\tlibxkbcommon.so:x11/libxkbcommon \\\n\t\tlibfontconfig.so:x11-fonts/fontconfig\n\nUSES=\t\tcargo pkgconfig\nUSE_GITHUB=\tyes\nGH_ACCOUNT=\t{gh_owner}\nGH_PROJECT=\t{gh_repo}\nCARGO_BUILD_ARGS=\t--package {crate_name}\n\n.include <bsd.port.mk>\n",
        tagline = entry.tagline,
        lic = entry.license,
    );
    let plist = format!("bin/{bin}\nshare/applications/{}.desktop\n", entry.app_id);
    vec![artifact("Makefile", makefile), artifact("pkg-plist", plist)]
}

fn generate_brew(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
    archive_url: &str,
) -> Vec<GeneratedArtifact> {
    let formula = format!(
        "class {class_name} < Formula\n  desc {desc_q}\n  homepage {home_q}\n  url {url_q}\n  version {ver_q}\n  license {lic_q}\n\n  depends_on \"rust\" => :build\n  depends_on \"pkg-config\" => :build\n\n  def install\n    system \"cargo\", \"install\", *std_cargo_args(path: \".\"), \"--package\", {crate_q}\n  end\n\n  test do\n    assert_match {ver_q}, shell_output(\"#{{bin}}/{bin} --version\")\n  end\nend\n",
        class_name = pascal_identifier(pkg_slug),
        desc_q = json_quote(&entry.tagline),
        home_q = json_quote(homepage),
        url_q = json_quote(archive_url),
        ver_q = json_quote(ver),
        lic_q = json_quote(&entry.license),
        crate_q = json_quote(crate_name),
    );
    vec![artifact(format!("Formula/{pkg_slug}.rb"), formula)]
}

fn generate_scoop(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    bin: &str,
    homepage: &str,
    release_base: &str,
    zero_sha: &str,
    has_github: bool,
) -> Vec<GeneratedArtifact> {
    let mut doc = json!({
        "version": ver,
        "description": entry.tagline,
        "homepage": homepage,
        "license": entry.license,
        "architecture": {
            "64bit": {
                "url": format!("{release_base}/{bin}-windows-x64.zip"),
                "hash": zero_sha,
            },
            "arm64": {
                "url": format!("{release_base}/{bin}-windows-arm64.zip"),
                "hash": zero_sha,
            },
        },
        "bin": format!("{bin}.exe"),
        "autoupdate": {
            "architecture": {
                "64bit": {
                    "url": format!("{}/releases/download/v$version/{bin}-windows-x64.zip", entry.repo),
                },
                "arm64": {
                    "url": format!("{}/releases/download/v$version/{bin}-windows-arm64.zip", entry.repo),
                },
            }
        }
    });
    if has_github && let Some(obj) = doc.as_object_mut() {
        obj.insert("checkver".to_owned(), json!({ "github": entry.repo }));
    }
    let mut rendered = serde_json::to_string_pretty(&doc).unwrap_or_default();
    rendered.push('\n');
    vec![artifact(format!("bucket/{pkg_slug}.json"), rendered)]
}

fn generate_choco(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    bin: &str,
    homepage: &str,
    release_base: &str,
    zero_sha: &str,
) -> Vec<GeneratedArtifact> {
    let nuspec = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2015/06/nuspec.xsd">
  <metadata>
    <id>{id}</id>
    <version>{ver}</version>
    <title>{title}</title>
    <authors>{authors}</authors>
    <projectUrl>{homepage}</projectUrl>
    <licenseUrl>{license_url}</licenseUrl>
    <tags>{tags}</tags>
    <summary>{summary}</summary>
    <description>{description}</description>
  </metadata>
  <files>
    <file src="tools\**" target="tools" />
  </files>
</package>
"#,
        id = xml_escape(pkg_slug),
        ver = xml_escape(ver),
        title = xml_escape(&entry.name),
        authors = xml_escape(&entry.publisher),
        homepage = xml_escape(homepage),
        license_url = xml_escape(&format!("{}/blob/main/LICENSE", entry.repo)),
        tags = xml_escape(&entry.tags.join(" ")),
        summary = xml_escape(&entry.tagline),
        description = xml_escape(&entry.description),
    );
    let ps1 = format!(
        "$ErrorActionPreference = 'Stop'\n$packageName = '{pkg_slug}'\n$toolsDir = \"$(Split-Path -Parent $MyInvocation.MyCommand.Definition)\"\n$url64 = '{release_base}/{bin}-windows-x64.zip'\n\nInstall-ChocolateyZipPackage `\n  -PackageName $packageName `\n  -Url64bit $url64 `\n  -UnzipLocation $toolsDir `\n  -Checksum64 '{zero_sha}' `\n  -ChecksumType64 'sha256'\n"
    );
    vec![
        artifact(format!("{pkg_slug}.nuspec"), nuspec),
        artifact("tools/chocolateyinstall.ps1", ps1),
    ]
}

fn generate_appimage(
    entry: &StoreEntry,
    ver: &str,
    pkg_slug: &str,
    bin: &str,
) -> Vec<GeneratedArtifact> {
    let yaml = format!(
        "version: 1\nAppDir:\n  path: ./AppDir\n  app_info:\n    id: {id_q}\n    name: {name_q}\n    icon: {icon_q}\n    version: {ver_q}\n    exec: {exec_q}\n    exec_args: \"$@\"\n  apt:\n    arch: amd64\n    allow_unauthenticated: true\n    sources:\n      - sourceline: \"deb http://archive.ubuntu.com/ubuntu/ noble main universe\"\n    include:\n      - libwayland-client0\n      - libxkbcommon0\n      - libvulkan1\n      - libfontconfig1\nAppImage:\n  arch: x86_64\n  file_name: {file_q}\n",
        id_q = json_quote(&entry.app_id),
        name_q = json_quote(&entry.name),
        icon_q = json_quote(pkg_slug),
        ver_q = json_quote(ver),
        exec_q = json_quote(&format!("usr/bin/{bin}")),
        file_q = json_quote(&format!(
            "{}-{ver}-x86_64.AppImage",
            pascal_identifier(&entry.name)
        )),
    );
    vec![artifact("AppImageBuilder.yml", yaml)]
}

fn generate_oci(
    entry: &StoreEntry,
    ver: &str,
    crate_name: &str,
    bin: &str,
    homepage: &str,
) -> Vec<GeneratedArtifact> {
    let dockerfile = format!(
        "FROM rust:1.86-slim AS builder\nWORKDIR /src\nRUN apt-get update && apt-get install -y --no-install-recommends pkg-config clang lld libwayland-dev libxkbcommon-dev libvulkan-dev libfontconfig1-dev && rm -rf /var/lib/apt/lists/*\nCOPY . .\nRUN cargo build --release --locked --package {crate_name}\n\nFROM gcr.io/distroless/cc-debian12 AS runtime\nLABEL org.opencontainers.image.title={title_q}\nLABEL org.opencontainers.image.description={desc_q}\nLABEL org.opencontainers.image.version={ver_q}\nLABEL org.opencontainers.image.licenses={lic_q}\nLABEL org.opencontainers.image.source={repo_q}\nLABEL org.opencontainers.image.url={home_q}\nCOPY --from=builder /src/target/release/{bin} /usr/local/bin/{bin}\nENTRYPOINT [\"/usr/local/bin/{bin}\"]\n",
        title_q = json_quote(&entry.name),
        desc_q = json_quote(&entry.tagline),
        ver_q = json_quote(ver),
        lic_q = json_quote(&entry.license),
        repo_q = json_quote(&entry.repo),
        home_q = json_quote(homepage),
    );
    vec![artifact("Dockerfile", dockerfile)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AwesomeGpuiCatalog;

    fn assert_xml_valid(xml: &str, label: &str) {
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(err) => panic!("{label} invalid XML: {err}"),
            }
            buf.clear();
        }
    }

    #[test]
    fn generates_and_validates_all_22_targets_for_bundled_entries() {
        let catalog = AwesomeGpuiCatalog::bundled().unwrap();
        let aphrody = catalog.by_id("aphrody").unwrap();
        let generated = generate_all_targets(aphrody);
        assert_eq!(generated.len(), 22);

        for (target, artifacts) in &generated {
            assert!(!artifacts.is_empty(), "{target} produced no artifacts");
            for art in artifacts {
                if art.relative_path.ends_with(".xml")
                    || art.relative_path.ends_with(".plist")
                    || art.relative_path.ends_with(".entitlements")
                    || art.relative_path.ends_with(".nuspec")
                {
                    assert_xml_valid(&art.content, &art.relative_path);
                } else if art.relative_path.ends_with(".yaml")
                    || art.relative_path.ends_with(".yml")
                {
                    let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&art.content)
                        .unwrap_or_else(|e| panic!("{} invalid YAML: {e}", art.relative_path));
                    assert!(
                        parsed.is_mapping(),
                        "{} YAML must be mapping",
                        art.relative_path
                    );
                } else if art.relative_path.ends_with(".json") {
                    let parsed: serde_json::Value = serde_json::from_str(&art.content)
                        .unwrap_or_else(|e| panic!("{} invalid JSON: {e}", art.relative_path));
                    assert!(parsed.is_object());
                } else if art.relative_path.ends_with(".toml") {
                    let parsed: toml::Value = toml::from_str(&art.content)
                        .unwrap_or_else(|e| panic!("{} invalid TOML: {e}", art.relative_path));
                    assert!(parsed.get("package").is_some());
                }
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let written = export_entry_to_dir(aphrody, &StoreTarget::ALL, dir.path()).unwrap();
        assert!(written.len() >= 28);
        assert!(dir.path().join("aphrody/apk/APKBUILD").is_file());
        assert!(dir.path().join("aphrody/rpm/aphrody.spec").is_file());
        assert!(dir.path().join("aphrody/deb/DEBIAN/control").is_file());
        assert!(
            dir.path()
                .join("aphrody/xbps/srcpkgs/aphrody/template")
                .is_file()
        );
        assert!(dir.path().join("aphrody/brew/Formula/aphrody.rb").is_file());
        assert!(
            dir.path()
                .join("aphrody/scoop/bucket/aphrody.json")
                .is_file()
        );
        assert!(dir.path().join("aphrody/choco/aphrody.nuspec").is_file());
        assert!(
            dir.path()
                .join("aphrody/appimage/AppImageBuilder.yml")
                .is_file()
        );
        assert!(dir.path().join("aphrody/oci/Dockerfile").is_file());
    }
}
