//! Headless agent & human CLI commands for Google OAuth/`gcloud`, Google Cloud AI (Gemini/Vertex),
//! Google Workspace Discovery (Drive/Gmail/Calendar/Docs/Sheets), YouTube, YouTube Music, and Media.

use anyhow::{Context as _, Result};
use aphrody_google::{
    GeminiGenerateRequest, GoogleAiBackend, GoogleAiClient, GoogleAuthManager,
    GoogleWorkspaceClient, InnerTubeClient, InnerTubeClientProfile, WorkspaceDiscoveryRequest,
    YouTubeDataClient, YouTubeMusicClient, YtMusicSearchFilter, sdk_parity_matrix,
    workspace::WORKSPACE_SERVICES,
};
use aphrody_media::{
    SubtitleFormat, capabilities, parse_dash_mpd, parse_hls_m3u8, parse_subtitles,
    validate_all_media_shaders,
};
use clap::{Subcommand, ValueEnum};

#[derive(Debug, Subcommand)]
pub enum GoogleSubcommand {
    /// Inspect Google OAuth 2.0 / gcloud / ADC authentication readiness (JSON)
    AuthStatus,
    /// Generate a Google OAuth 2.0 PKCE S256 login URL
    PkceUrl {
        #[arg(long, default_value = "http://127.0.0.1:49152/callback")]
        redirect_uri: String,
    },
    /// Print the pure-Rust SDK parity matrix covering Google Kotlin, Go, Python, TS, and Rust SDKs
    SdkParity,
    /// Run a Gemini Developer API or Vertex AI generation request via gcloud / OAuth / API key
    Ai {
        /// Prompt text
        prompt: String,
        #[arg(long, default_value = "gemini-2.5-flash")]
        model: String,
        /// Use Vertex AI backend instead of Gemini Developer API
        #[arg(long)]
        vertex: bool,
        /// Optional YouTube URL attached as multimodal video input
        #[arg(long)]
        youtube_url: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceSubcommand {
    /// List supported Google Workspace Discovery services (Drive, Gmail, Calendar, Docs, Sheets, Slides, Tasks, Chat)
    Services,
    /// List Google Drive files via OAuth / gcloud
    Drive {
        #[arg(long, default_value_t = 15)]
        limit: u32,
        #[arg(long)]
        query: Option<String>,
    },
    /// List Gmail messages via OAuth / gcloud
    Gmail {
        #[arg(long, default_value_t = 15)]
        limit: u32,
        #[arg(long)]
        query: Option<String>,
    },
    /// List upcoming Google Calendar events via OAuth / gcloud
    Calendar {
        #[arg(long, default_value_t = 15)]
        limit: u32,
    },
    /// Execute a generic Google Workspace Discovery REST call
    Call {
        service: String,
        path: String,
        #[arg(long, default_value = "GET")]
        method: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum YouTubeSubcommand {
    /// Search YouTube via pure-Rust InnerTube (WEB) without API key
    Search { query: String },
    /// Search YouTube via official YouTube Data API v3 (OAuth / gcloud)
    V3Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Inspect YouTube player metadata, adaptive streams, and captions via InnerTube
    Info { video_id: String },
    /// Fetch SponsorBlock segments for a YouTube video ID
    Sponsorblock { video_id: String },
    /// List authenticated user playlists via YouTube Data API v3
    Playlists {
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliMusicFilter {
    Songs,
    Videos,
    Albums,
    Artists,
    Playlists,
}

impl From<CliMusicFilter> for YtMusicSearchFilter {
    fn from(value: CliMusicFilter) -> Self {
        match value {
            CliMusicFilter::Songs => Self::Songs,
            CliMusicFilter::Videos => Self::Videos,
            CliMusicFilter::Albums => Self::Albums,
            CliMusicFilter::Artists => Self::Artists,
            CliMusicFilter::Playlists => Self::Playlists,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum YouTubeMusicSubcommand {
    /// Search YouTube Music (InnerTube WEB_REMIX)
    Search {
        query: String,
        #[arg(long, value_enum)]
        filter: Option<CliMusicFilter>,
    },
    /// Fetch YouTube Music home shelf tracks
    Home,
    /// Fetch lyrics for a YouTube Music video ID
    Lyrics { video_id: String },
}

#[derive(Debug, Subcommand)]
pub enum MediaSubcommand {
    /// Report pure-Rust zero-FFmpeg multimedia capabilities (JSON)
    Capabilities,
    /// Parse and validate all built-in media WGSL shaders with Naga v30
    ValidateShaders,
    /// Parse an HLS .m3u8 manifest file and report segments & AES-128 keys
    InspectHls { path: std::path::PathBuf },
    /// Parse a DASH .mpd manifest file and report representations
    InspectDash { path: std::path::PathBuf },
    /// Parse an SRT or WebVTT subtitle file with Unicode Bidi metadata
    InspectSubtitles {
        path: std::path::PathBuf,
        #[arg(long)]
        vtt: bool,
    },
}

pub fn run_google(cmd: GoogleSubcommand) -> Result<()> {
    let auth = GoogleAuthManager::default();
    match cmd {
        GoogleSubcommand::AuthStatus => {
            println!("{}", serde_json::to_string_pretty(&auth.status())?);
        }
        GoogleSubcommand::PkceUrl { redirect_uri } => {
            let req = auth.prepare_pkce_login(&redirect_uri)?;
            println!("{}", serde_json::to_string_pretty(&req)?);
        }
        GoogleSubcommand::SdkParity => {
            println!("{}", serde_json::to_string_pretty(&sdk_parity_matrix())?);
        }
        GoogleSubcommand::Ai {
            prompt,
            model,
            vertex,
            youtube_url,
        } => {
            let backend = if vertex {
                let status = auth.status();
                let project_id = status
                    .project_id
                    .context("unable to resolve GCP project_id for Vertex AI")?;
                GoogleAiBackend::VertexAi {
                    project_id,
                    region: status.region,
                }
            } else {
                GoogleAiBackend::GeminiDeveloper
            };
            let client = GoogleAiClient::new(auth, backend);
            let mut request = GeminiGenerateRequest::text(model, prompt);
            if let Some(url) = youtube_url {
                request = request.with_youtube_url(url);
            }
            let response = client.generate_content(&request)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }
    Ok(())
}

pub fn run_workspace(cmd: WorkspaceSubcommand) -> Result<()> {
    let client = GoogleWorkspaceClient::new(GoogleAuthManager::default());
    match cmd {
        WorkspaceSubcommand::Services => {
            println!("{}", serde_json::to_string_pretty(WORKSPACE_SERVICES)?);
        }
        WorkspaceSubcommand::Drive { limit, query } => {
            let out = client.list_drive_files(limit, query.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        WorkspaceSubcommand::Gmail { limit, query } => {
            let out = client.list_gmail_messages(limit, query.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        WorkspaceSubcommand::Calendar { limit } => {
            let out = client.list_calendar_events(limit)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        WorkspaceSubcommand::Call {
            service,
            path,
            method,
        } => {
            let out = client.call(&WorkspaceDiscoveryRequest {
                service,
                method,
                path,
                query: Vec::new(),
                body: None,
            })?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }
    Ok(())
}

pub fn run_youtube(cmd: YouTubeSubcommand) -> Result<()> {
    match cmd {
        YouTubeSubcommand::Search { query } => {
            let client = InnerTubeClient::discover(InnerTubeClientProfile::Web)?;
            let items = client.search(&query, None)?;
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
        YouTubeSubcommand::V3Search { query, limit } => {
            let client = YouTubeDataClient::new(GoogleAuthManager::default());
            let items = client.search_videos(&query, limit)?;
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
        YouTubeSubcommand::Info { video_id } => {
            let client = InnerTubeClient::discover(InnerTubeClientProfile::Web)?;
            let info = client.player(&video_id)?;
            println!("{}", serde_json::to_string_pretty(&info)?);
        }
        YouTubeSubcommand::Sponsorblock { video_id } => {
            let segments = InnerTubeClient::sponsorblock_segments(&video_id)?;
            println!("{}", serde_json::to_string_pretty(&segments)?);
        }
        YouTubeSubcommand::Playlists { limit } => {
            let client = YouTubeDataClient::new(GoogleAuthManager::default());
            let items = client.list_my_playlists(limit)?;
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
    }
    Ok(())
}

pub fn run_ytmusic(cmd: YouTubeMusicSubcommand) -> Result<()> {
    let client = YouTubeMusicClient::discover()?;
    match cmd {
        YouTubeMusicSubcommand::Search { query, filter } => {
            let items = client.search(&query, filter.map(Into::into))?;
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
        YouTubeMusicSubcommand::Home => {
            let items = client.home_tracks()?;
            println!("{}", serde_json::to_string_pretty(&items)?);
        }
        YouTubeMusicSubcommand::Lyrics { video_id } => {
            let lyrics = client.lyrics_for_video(&video_id)?;
            println!("{}", serde_json::to_string_pretty(&lyrics)?);
        }
    }
    Ok(())
}

pub fn run_media(cmd: MediaSubcommand) -> Result<()> {
    match cmd {
        MediaSubcommand::Capabilities => {
            println!("{}", serde_json::to_string_pretty(&capabilities())?);
        }
        MediaSubcommand::ValidateShaders => {
            let report = validate_all_media_shaders()?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        MediaSubcommand::InspectHls { path } => {
            let raw = std::fs::read_to_string(&path)?;
            let parsed = parse_hls_m3u8(&raw)?;
            println!("{}", serde_json::to_string_pretty(&parsed)?);
        }
        MediaSubcommand::InspectDash { path } => {
            let raw = std::fs::read_to_string(&path)?;
            let parsed = parse_dash_mpd(&raw)?;
            println!("{}", serde_json::to_string_pretty(&parsed)?);
        }
        MediaSubcommand::InspectSubtitles { path, vtt } => {
            let raw = std::fs::read_to_string(&path)?;
            let fmt = if vtt {
                SubtitleFormat::WebVtt
            } else {
                SubtitleFormat::Srt
            };
            let parsed = parse_subtitles(fmt, &raw)?;
            println!("{}", serde_json::to_string_pretty(&parsed)?);
        }
    }
    Ok(())
}
