//! Provides constructs for the Zed app version and release channel.

#![deny(missing_docs)]

use std::{str::FromStr, sync::LazyLock};

use gpui::{App, Global};
use semver::Version;

const ZED_DOCS_URL: &str = brand::DOCS_URL;

/// stable | dev | nightly | preview
pub static RELEASE_CHANNEL_NAME: LazyLock<String> = LazyLock::new(|| {
    if cfg!(debug_assertions) {
        brand::env_var("RELEASE_CHANNEL").unwrap_or_else(|_| compile_time_release_channel_name())
    } else {
        compile_time_release_channel_name()
    }
});

/// When a crate in zed is used as a dependency that uses the `crane` nix
/// library, it vendors each crate separately and builds it in isolation, which
/// makes the `include_str!` fail.
///
/// The build script checks for `$APHRODY_RELEASE_CHANNEL` / `$ZED_RELEASE_CHANNEL` and emits the `cfg`
#[cfg(__do_not_set_zed_release_channel)]
fn compile_time_release_channel_name() -> String {
    option_env!("APHRODY_RELEASE_CHANNEL")
        .or(option_env!("ZED_RELEASE_CHANNEL"))
        .expect("APHRODY_RELEASE_CHANNEL or ZED_RELEASE_CHANNEL must be set")
        .trim()
        .to_string()
}

#[cfg(not(__do_not_set_zed_release_channel))]
fn compile_time_release_channel_name() -> String {
    include_str!("../../zed/RELEASE_CHANNEL").trim().to_string()
}

#[doc(hidden)]
pub static RELEASE_CHANNEL: LazyLock<ReleaseChannel> =
    LazyLock::new(|| match ReleaseChannel::from_str(&RELEASE_CHANNEL_NAME) {
        Ok(channel) => channel,
        _ => panic!("invalid release channel {}", *RELEASE_CHANNEL_NAME),
    });

/// The app identifier for the current release channel, Windows only.
#[cfg(target_os = "windows")]
pub fn app_identifier() -> &'static str {
    match *RELEASE_CHANNEL {
        ReleaseChannel::Dev => brand::WINDOWS_APP_ID_DEV,
        ReleaseChannel::Nightly => brand::WINDOWS_APP_ID_NIGHTLY,
        ReleaseChannel::Preview => brand::WINDOWS_APP_ID_PREVIEW,
        ReleaseChannel::Stable => brand::WINDOWS_APP_ID_STABLE,
    }
}

/// The Git commit SHA that Zed was built at.
#[derive(Clone, Eq, Debug, PartialEq)]
pub struct AppCommitSha(String);

struct GlobalAppCommitSha(AppCommitSha);

impl Global for GlobalAppCommitSha {}

impl AppCommitSha {
    /// Creates a new [`AppCommitSha`].
    pub fn new(sha: String) -> Self {
        AppCommitSha(sha)
    }

    /// Returns the global [`AppCommitSha`], if one is set.
    pub fn try_global(cx: &App) -> Option<AppCommitSha> {
        cx.try_global::<GlobalAppCommitSha>()
            .map(|sha| sha.0.clone())
    }

    /// Sets the global [`AppCommitSha`].
    pub fn set_global(sha: AppCommitSha, cx: &mut App) {
        cx.set_global(GlobalAppCommitSha(sha))
    }

    /// Returns the full commit SHA.
    pub fn full(&self) -> String {
        self.0.to_string()
    }

    /// Returns the short (7 character) commit SHA.
    pub fn short(&self) -> String {
        self.0.chars().take(7).collect()
    }
}

struct GlobalAppVersion(Version);

impl Global for GlobalAppVersion {}

/// The version of Zed.
pub struct AppVersion;

impl AppVersion {
    /// Load the app version from env.
    pub fn load(
        pkg_version: &str,
        build_id: Option<&str>,
        commit_sha: Option<AppCommitSha>,
    ) -> Version {
        let mut version: Version = if let Ok(from_env) = brand::env_var("APP_VERSION") {
            from_env.parse().expect("invalid APHRODY_APP_VERSION / ZED_APP_VERSION")
        } else {
            pkg_version.parse().expect("invalid version in Cargo.toml")
        };
        let mut pre = String::from(RELEASE_CHANNEL.dev_name());

        if let Some(build_id) = build_id {
            pre.push('.');
            pre.push_str(&build_id);
        }

        if let Some(sha) = commit_sha {
            pre.push('.');
            pre.push_str(&sha.0);
        }
        if let Ok(build) = semver::BuildMetadata::new(&pre) {
            version.build = build;
        }

        version
    }

    /// Returns the global version number.
    pub fn global(cx: &App) -> Version {
        if cx.has_global::<GlobalAppVersion>() {
            cx.global::<GlobalAppVersion>().0.clone()
        } else {
            Version::new(0, 0, 0)
        }
    }
}

/// A Zed release channel.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ReleaseChannel {
    /// The development release channel.
    ///
    /// Used for local debug builds of Zed.
    #[default]
    Dev,

    /// The Nightly release channel.
    Nightly,

    /// The Preview release channel.
    Preview,

    /// The Stable release channel.
    Stable,
}

struct GlobalReleaseChannel(ReleaseChannel);

impl Global for GlobalReleaseChannel {}

/// Initializes the release channel.
pub fn init(app_version: Version, cx: &mut App) {
    cx.set_global(GlobalAppVersion(app_version));
    cx.set_global(GlobalReleaseChannel(*RELEASE_CHANNEL))
}

/// Initializes the release channel for tests that rely on fake release channel.
pub fn init_test(app_version: Version, release_channel: ReleaseChannel, cx: &mut App) {
    cx.set_global(GlobalAppVersion(app_version));
    cx.set_global(GlobalReleaseChannel(release_channel))
}

/// Returns the Zed docs URL for the current release channel for the given
/// `slug`.
pub fn docs_url(slug: &str, cx: &App) -> String {
    ReleaseChannel::try_global(cx)
        .unwrap_or(*RELEASE_CHANNEL)
        .docs_url(slug)
}

impl ReleaseChannel {
    /// All release channels.
    pub const ALL: [ReleaseChannel; 4] = [
        ReleaseChannel::Dev,
        ReleaseChannel::Nightly,
        ReleaseChannel::Preview,
        ReleaseChannel::Stable,
    ];

    /// Returns the global [`ReleaseChannel`].
    pub fn global(cx: &App) -> Self {
        cx.global::<GlobalReleaseChannel>().0
    }

    /// Returns the global [`ReleaseChannel`], if one is set.
    pub fn try_global(cx: &App) -> Option<Self> {
        cx.try_global::<GlobalReleaseChannel>()
            .map(|channel| channel.0)
    }

    /// Returns whether we want to poll for updates for this [`ReleaseChannel`]
    pub fn poll_for_updates(&self) -> bool {
        !matches!(self, ReleaseChannel::Dev)
    }

    /// Returns the display name for this [`ReleaseChannel`].
    pub fn display_name(&self) -> &'static str {
        match self {
            ReleaseChannel::Dev => brand::APP_NAME_DEV,
            ReleaseChannel::Nightly => brand::APP_NAME_NIGHTLY,
            ReleaseChannel::Preview => brand::APP_NAME_PREVIEW,
            ReleaseChannel::Stable => brand::APP_NAME,
        }
    }

    /// Returns the programmatic name for this [`ReleaseChannel`].
    pub fn dev_name(&self) -> &'static str {
        match self {
            ReleaseChannel::Dev => "dev",
            ReleaseChannel::Nightly => "nightly",
            ReleaseChannel::Preview => "preview",
            ReleaseChannel::Stable => "stable",
        }
    }

    /// Returns the application ID that's used by Wayland as application ID
    /// and WM_CLASS on X11.
    /// This also has to match the bundle identifier on macOS.
    pub fn app_id(&self) -> &'static str {
        match self {
            ReleaseChannel::Dev => brand::IDENTIFIER_DEV,
            ReleaseChannel::Nightly => brand::IDENTIFIER_NIGHTLY,
            ReleaseChannel::Preview => brand::IDENTIFIER_PREVIEW,
            ReleaseChannel::Stable => brand::IDENTIFIER_STABLE,
        }
    }

    /// Returns the query parameter for this [`ReleaseChannel`].
    pub fn release_query_param(&self) -> Option<&'static str> {
        match self {
            Self::Dev => None,
            Self::Nightly => Some("nightly=1"),
            Self::Preview => Some("preview=1"),
            Self::Stable => None,
        }
    }

    /// Returns the Zed docs URL for this [`ReleaseChannel`] for the given
    /// `slug`.
    pub fn docs_url(&self, slug: &str) -> String {
        let channel_path_segment = match self {
            Self::Dev | Self::Nightly => Some("nightly"),
            Self::Preview => Some("preview"),
            Self::Stable => None,
        };

        match channel_path_segment {
            Some(channel) if slug.is_empty() => format!("{ZED_DOCS_URL}/{channel}"),
            Some(channel) => format!("{ZED_DOCS_URL}/{channel}/{slug}"),
            None if slug.is_empty() => ZED_DOCS_URL.to_string(),
            None => format!("{ZED_DOCS_URL}/{slug}"),
        }
    }
}

/// Error indicating that release channel string does not match any known release channel names.
#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub struct InvalidReleaseChannel;

impl FromStr for ReleaseChannel {
    type Err = InvalidReleaseChannel;

    fn from_str(channel: &str) -> Result<Self, Self::Err> {
        Ok(match channel {
            "dev" => ReleaseChannel::Dev,
            "nightly" => ReleaseChannel::Nightly,
            "preview" => ReleaseChannel::Preview,
            "stable" => ReleaseChannel::Stable,
            _ => return Err(InvalidReleaseChannel),
        })
    }
}

/// Six-tier distribution track extending the core 4-variant [`ReleaseChannel`]
/// enum for automated LTS, Canary, and nightly release pipelines without
/// breaking exhaustive matches in downstream workspace crates.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReleaseTrack {
    /// Long-Term Support track pinned for stability and security fixes.
    Lts,
    /// General-availability stable releases.
    Stable,
    /// Release-candidate preview builds ahead of stable promotion.
    Preview,
    /// Automated daily canary snapshots built from `main`.
    Canary,
    /// Unfrozen bleeding-edge nightly builds.
    Nightly,
    /// Local developer workspace builds.
    Dev,
}

impl ReleaseTrack {
    /// All six release tracks ordered from longest-lived to fastest-moving.
    pub const ALL: [Self; 6] = [
        Self::Lts,
        Self::Stable,
        Self::Preview,
        Self::Canary,
        Self::Nightly,
        Self::Dev,
    ];

    /// Parses a case-insensitive track name (`lts`, `stable`, `preview`, `canary`, `nightly`, `dev`).
    pub fn parse(track: &str) -> Result<Self, InvalidReleaseChannel> {
        match track.trim().to_ascii_lowercase().as_str() {
            "lts" => Ok(Self::Lts),
            "stable" => Ok(Self::Stable),
            "preview" => Ok(Self::Preview),
            "canary" => Ok(Self::Canary),
            "nightly" => Ok(Self::Nightly),
            "dev" => Ok(Self::Dev),
            _ => Err(InvalidReleaseChannel),
        }
    }

    /// Returns the lowercase canonical identifier used in release URLs and manifests.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lts => "lts",
            Self::Stable => "stable",
            Self::Preview => "preview",
            Self::Canary => "canary",
            Self::Nightly => "nightly",
            Self::Dev => "dev",
        }
    }

    /// Returns `true` when this track ships pre-release artifacts.
    pub const fn is_prerelease(self) -> bool {
        matches!(self, Self::Preview | Self::Canary | Self::Nightly | Self::Dev)
    }

    /// Maps a core 4-variant [`ReleaseChannel`] onto its corresponding [`ReleaseTrack`].
    pub const fn from_channel(channel: ReleaseChannel) -> Self {
        match channel {
            ReleaseChannel::Stable => Self::Stable,
            ReleaseChannel::Preview => Self::Preview,
            ReleaseChannel::Nightly => Self::Nightly,
            ReleaseChannel::Dev => Self::Dev,
        }
    }

    /// Maps a [`ReleaseTrack`] onto the closest binary-compatible [`ReleaseChannel`].
    pub const fn to_channel(self) -> ReleaseChannel {
        match self {
            Self::Lts | Self::Stable => ReleaseChannel::Stable,
            Self::Preview => ReleaseChannel::Preview,
            Self::Canary | Self::Nightly => ReleaseChannel::Nightly,
            Self::Dev => ReleaseChannel::Dev,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ReleaseChannel, ReleaseTrack};

    #[test]
    fn test_docs_url_for_release_channel() {
        let base = brand::DOCS_URL;
        assert_eq!(
            ReleaseChannel::Dev.docs_url("settings"),
            format!("{base}/nightly/settings")
        );
        assert_eq!(
            ReleaseChannel::Nightly.docs_url("settings"),
            format!("{base}/nightly/settings")
        );
        assert_eq!(
            ReleaseChannel::Preview.docs_url("settings"),
            format!("{base}/preview/settings")
        );
        assert_eq!(
            ReleaseChannel::Stable.docs_url("settings"),
            format!("{base}/settings")
        );
    }

    #[test]
    fn test_six_tier_release_track_mapping() {
        assert_eq!(ReleaseTrack::ALL.len(), 6);
        assert_eq!(ReleaseTrack::parse("LTS"), Ok(ReleaseTrack::Lts));
        assert_eq!(ReleaseTrack::parse("canary"), Ok(ReleaseTrack::Canary));
        assert!(ReleaseTrack::parse("unknown").is_err());

        assert_eq!(ReleaseTrack::Lts.to_channel(), ReleaseChannel::Stable);
        assert_eq!(ReleaseTrack::Canary.to_channel(), ReleaseChannel::Nightly);
        assert!(!ReleaseTrack::Lts.is_prerelease());
        assert!(!ReleaseTrack::Stable.is_prerelease());
        assert!(ReleaseTrack::Canary.is_prerelease());
        assert_eq!(
            ReleaseTrack::from_channel(ReleaseChannel::Preview),
            ReleaseTrack::Preview
        );
    }
}
