//! Local media library: a directory tree of video files indexed through `aphrody_media`
//! (container list, subtitle parser, evidence index) plus MP4 metadata from `craft-media`.
//! Nothing here touches the network.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result};
use aphrody_media::watch::WatchEvidenceRecord;
use aphrody_media::{SubtitleCue, SubtitleFormat, WatchIndex, capabilities, parse_subtitles};
use craft_media::FileReader;
use craft_media::isobmff::{self, CodecConfig, TrackKind};

const MAX_SCAN_DEPTH: usize = 5;
pub const LOOSE_FILES_CREATOR: &str = "Local files";
const RECENT_WINDOW_SECS: u64 = 30 * 24 * 3600;

#[derive(Debug, Clone)]
pub struct VideoItem {
    pub id: usize,
    pub path: PathBuf,
    pub title: String,
    pub creator: String,
    pub description: String,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub modified_secs: u64,
    /// MP4/MOV with an H.264 track: the only combination the bundled pure-Rust decoder plays.
    pub playable: bool,
    pub subtitles: Vec<SubtitleCue>,
}

impl VideoItem {
    #[must_use]
    pub fn aspect(&self) -> f32 {
        if self.width == 0 || self.height == 0 {
            16.0 / 9.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Portrait clips belong to the short-form viewer instead of the landscape feeds.
    #[must_use]
    pub fn is_short(&self) -> bool {
        self.height > self.width
    }

    fn probe(id: usize, root: &Path, path: &Path) -> Self {
        let metadata = std::fs::metadata(path).ok();
        let modified_secs = metadata
            .as_ref()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |elapsed| elapsed.as_secs());
        let size_bytes = metadata.map_or(0, |meta| meta.len());
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("untitled");
        let creator = path
            .strip_prefix(root)
            .ok()
            .and_then(|relative| relative.parent())
            .and_then(|parent| parent.components().next())
            .map_or_else(
                || LOOSE_FILES_CREATOR.to_owned(),
                |component| prettify(&component.as_os_str().to_string_lossy()),
            );
        let mut item = Self {
            id,
            path: path.to_owned(),
            title: prettify(stem),
            creator,
            description: String::new(),
            duration_ms: 0,
            width: 0,
            height: 0,
            size_bytes,
            modified_secs,
            playable: false,
            subtitles: Vec::new(),
        };
        match probe_mp4(path) {
            Ok(probe) => {
                item.duration_ms = probe.duration_ms;
                item.width = probe.width;
                item.height = probe.height;
                item.playable = probe.h264;
                if let Some(title) = probe.title.filter(|title| !title.trim().is_empty()) {
                    item.title = title;
                }
            }
            Err(error) => log::debug!("not demuxed as MP4 {}: {error:#}", path.display()),
        }
        item.subtitles = load_sidecar_subtitles(path);
        item.description = std::fs::read_to_string(path.with_extension("txt"))
            .map(|text| text.trim().to_owned())
            .unwrap_or_default();
        item
    }
}

struct Mp4Probe {
    duration_ms: u64,
    width: u32,
    height: u32,
    h264: bool,
    title: Option<String>,
}

fn probe_mp4(path: &Path) -> Result<Mp4Probe> {
    let source = FileReader::open(path).context("open")?;
    let file = isobmff::open(source).context("parse mp4")?;
    let track_index = file
        .track_of_kind(TrackKind::Video)
        .context("no video track")?;
    let track = &file.tracks[track_index];
    let duration_ms = if file.timescale > 0 {
        file.duration.saturating_mul(1000) / u64::from(file.timescale)
    } else {
        0
    };
    let (mut width, mut height) = (track.width, track.height);
    if (width == 0 || height == 0)
        && let Some(params) = track.video()
    {
        width = u32::from(params.width);
        height = u32::from(params.height);
    }
    Ok(Mp4Probe {
        duration_ms,
        width,
        height,
        h264: matches!(track.codec(), Some(CodecConfig::Avc(_))),
        title: file.metadata.title().map(ToOwned::to_owned),
    })
}

fn load_sidecar_subtitles(path: &Path) -> Vec<SubtitleCue> {
    for (extension, format) in [
        ("srt", SubtitleFormat::Srt),
        ("vtt", SubtitleFormat::WebVtt),
    ] {
        let sidecar = path.with_extension(extension);
        let Ok(text) = std::fs::read_to_string(&sidecar) else {
            continue;
        };
        match parse_subtitles(format, &text) {
            Ok(cues) => return cues,
            Err(error) => log::warn!("bad subtitles {}: {error:#}", sidecar.display()),
        }
    }
    Vec::new()
}

/// Turns a file or folder name into a display name.
#[must_use]
pub fn prettify(raw: &str) -> String {
    let spaced: String = raw
        .chars()
        .map(|character| {
            if matches!(character, '_' | '-' | '.') {
                ' '
            } else {
                character
            }
        })
        .collect();
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Creator {
    pub name: String,
    pub video_count: usize,
    pub short_count: usize,
    pub newest_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchFilter {
    #[default]
    All,
    Videos,
    Shorts,
    Unwatched,
    Recent,
}

impl SearchFilter {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Shorts,
        Self::Unwatched,
        Self::Videos,
        Self::Recent,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Videos => "Videos",
            Self::Shorts => "Short-form",
            Self::Unwatched => "Unwatched",
            Self::Recent => "Recently added",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchSnippet {
    pub start_ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub id: usize,
    pub score: f32,
    pub snippet: Option<SearchSnippet>,
}

#[derive(Debug, Default, Clone)]
pub struct Library {
    root: PathBuf,
    items: Vec<VideoItem>,
    index: WatchIndex,
}

impl Library {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Indexes every file under `root` whose extension is a container `aphrody_media` lists.
    pub fn scan(root: &Path) -> Result<Self> {
        let extensions = capabilities().supported_containers;
        let mut paths = Vec::new();
        collect_media_files(root, &extensions, 0, &mut paths)
            .with_context(|| format!("scan {}", root.display()))?;
        paths.sort();
        let items: Vec<VideoItem> = paths
            .iter()
            .enumerate()
            .map(|(id, path)| VideoItem::probe(id, root, path))
            .collect();
        let mut index = WatchIndex::new();
        for item in &items {
            let media_id = item.id.to_string();
            index.insert(WatchEvidenceRecord {
                media_id: media_id.clone(),
                timestamp_ms: 0,
                end_ms: item.duration_ms,
                source_kind: "description".to_owned(),
                text: item.description.clone(),
            });
            for cue in &item.subtitles {
                index.insert(WatchEvidenceRecord {
                    media_id: media_id.clone(),
                    timestamp_ms: cue.start_ms,
                    end_ms: cue.end_ms,
                    source_kind: "subtitle".to_owned(),
                    text: cue.text.clone(),
                });
            }
        }
        Ok(Self {
            root: root.to_owned(),
            items,
            index,
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn items(&self) -> &[VideoItem] {
        &self.items
    }

    #[must_use]
    pub fn get(&self, id: usize) -> Option<&VideoItem> {
        self.items.get(id)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Landscape items, newest first.
    #[must_use]
    pub fn feed(&self) -> Vec<usize> {
        self.sorted_ids(|item| !item.is_short())
    }

    #[must_use]
    pub fn shorts(&self) -> Vec<usize> {
        self.sorted_ids(VideoItem::is_short)
    }

    #[must_use]
    pub fn videos_by(&self, creator: &str) -> Vec<usize> {
        self.sorted_ids(|item| item.creator == creator && !item.is_short())
    }

    #[must_use]
    pub fn shorts_by(&self, creator: &str) -> Vec<usize> {
        self.sorted_ids(|item| item.creator == creator && item.is_short())
    }

    fn sorted_ids(&self, keep: impl Fn(&VideoItem) -> bool) -> Vec<usize> {
        let mut ids: Vec<usize> = self
            .items
            .iter()
            .filter(|item| keep(item))
            .map(|item| item.id)
            .collect();
        ids.sort_by(|left, right| {
            self.items[*right]
                .modified_secs
                .cmp(&self.items[*left].modified_secs)
                .then(left.cmp(right))
        });
        ids
    }

    #[must_use]
    pub fn creators(&self) -> Vec<Creator> {
        let mut by_name: BTreeMap<&str, Creator> = BTreeMap::new();
        for item in &self.items {
            let creator = by_name.entry(&item.creator).or_insert_with(|| Creator {
                name: item.creator.clone(),
                video_count: 0,
                short_count: 0,
                newest_secs: 0,
            });
            if item.is_short() {
                creator.short_count += 1;
            } else {
                creator.video_count += 1;
            }
            creator.newest_secs = creator.newest_secs.max(item.modified_secs);
        }
        by_name.into_values().collect()
    }

    #[must_use]
    pub fn creator(&self, name: &str) -> Option<Creator> {
        self.creators()
            .into_iter()
            .find(|creator| creator.name == name)
    }

    /// Related list: same creator first, then everything else, newest first, never the item itself.
    #[must_use]
    pub fn related(&self, id: usize, limit: usize) -> Vec<usize> {
        let Some(current) = self.get(id) else {
            return Vec::new();
        };
        let mut ids =
            self.sorted_ids(|item| item.id != id && item.is_short() == current.is_short());
        ids.sort_by_key(|other| usize::from(self.items[*other].creator != current.creator));
        ids.truncate(limit);
        ids
    }

    /// The item that follows `id` in the related order, used by autoplay-next.
    #[must_use]
    pub fn next_after(&self, id: usize) -> Option<usize> {
        self.related(id, 1).first().copied()
    }

    #[must_use]
    pub fn search(
        &self,
        query: &str,
        filter: SearchFilter,
        watched: &HashSet<usize>,
        now_secs: u64,
    ) -> Vec<SearchHit> {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(str::to_lowercase)
            .filter(|term| !term.is_empty())
            .collect();
        let mut snippets: BTreeMap<usize, (f32, SearchSnippet)> = BTreeMap::new();
        for hit in self.index.query(query, 10_000) {
            let Ok(id) = hit.record.media_id.parse::<usize>() else {
                continue;
            };
            let better = snippets
                .get(&id)
                .is_none_or(|(existing_score, _)| hit.score > *existing_score);
            if better && !hit.record.text.is_empty() {
                snippets.insert(
                    id,
                    (
                        hit.score,
                        SearchSnippet {
                            start_ms: hit.record.timestamp_ms,
                            text: hit.record.text,
                        },
                    ),
                );
            }
        }

        let mut hits: Vec<SearchHit> = self
            .items
            .iter()
            .filter(|item| match filter {
                SearchFilter::All => true,
                SearchFilter::Videos => !item.is_short(),
                SearchFilter::Shorts => item.is_short(),
                SearchFilter::Unwatched => !watched.contains(&item.id),
                SearchFilter::Recent => {
                    now_secs.saturating_sub(item.modified_secs) <= RECENT_WINDOW_SECS
                }
            })
            .filter_map(|item| {
                if terms.is_empty() {
                    return Some(SearchHit {
                        id: item.id,
                        score: 0.0,
                        snippet: None,
                    });
                }
                let title = item.title.to_lowercase();
                let creator = item.creator.to_lowercase();
                let description = item.description.to_lowercase();
                let mut score = 0.0_f32;
                for term in &terms {
                    if title.contains(term.as_str()) {
                        score += 3.0;
                    }
                    if creator.contains(term.as_str()) {
                        score += 2.0;
                    }
                    if description.contains(term.as_str()) {
                        score += 1.0;
                    }
                }
                let snippet = snippets.get(&item.id).map(|(subtitle_score, snippet)| {
                    score += 0.5 + subtitle_score;
                    snippet.clone()
                });
                (score > 0.0).then_some(SearchHit {
                    id: item.id,
                    score: score / terms.len() as f32,
                    snippet,
                })
            })
            .collect();
        hits.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    self.items[right.id]
                        .modified_secs
                        .cmp(&self.items[left.id].modified_secs),
                )
                .then(left.id.cmp(&right.id))
        });
        hits
    }

    /// Spell correction for a query that matches nothing: swaps each unknown word for the closest
    /// title or creator word within two edits.
    #[must_use]
    pub fn suggest(&self, query: &str) -> Option<String> {
        let vocabulary: HashSet<String> = self
            .items
            .iter()
            .flat_map(|item| [item.title.as_str(), item.creator.as_str()])
            .flat_map(|text| text.split(|character: char| !character.is_alphanumeric()))
            .filter(|word| word.chars().count() >= 3)
            .map(str::to_lowercase)
            .collect();
        let mut changed = false;
        let corrected: Vec<String> = query
            .split_whitespace()
            .map(str::to_lowercase)
            .map(|word| {
                if vocabulary.iter().any(|known| known.contains(&word)) || word.chars().count() < 4
                {
                    return word;
                }
                let best = vocabulary
                    .iter()
                    .map(|known| (edit_distance(&word, known), known))
                    .filter(|(distance, _)| *distance <= 2)
                    .min_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(right.1)));
                match best {
                    Some((_, known)) => {
                        changed = true;
                        known.clone()
                    }
                    None => word,
                }
            })
            .collect();
        changed.then(|| corrected.join(" "))
    }
}

fn collect_media_files(
    directory: &Path,
    extensions: &[String],
    depth: usize,
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if depth < MAX_SCAN_DEPTH {
                collect_media_files(&path, extensions, depth + 1, out)?;
            }
        } else if file_type.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    extensions
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(extension))
                })
        {
            out.push(path);
        }
    }
    Ok(())
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (row, left_char) in left.chars().enumerate() {
        let mut current = vec![row + 1];
        for (column, right_char) in right.iter().enumerate() {
            let substitution = previous[column] + usize::from(left_char != *right_char);
            current.push(
                substitution
                    .min(previous[column + 1] + 1)
                    .min(current[column] + 1),
            );
        }
        previous = current;
    }
    previous[right.len()]
}

#[must_use]
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// `65_000` becomes `1:05`, `3_725_000` becomes `1:02:05`.
#[must_use]
pub fn format_duration(milliseconds: u64) -> String {
    let total = milliseconds / 1000;
    let (hours, minutes, seconds) = (total / 3600, (total / 60) % 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[must_use]
pub fn format_count(count: u64, noun: &str) -> String {
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{plural}")
}

/// Relative age such as `3 days ago`.
#[must_use]
pub fn relative_age(modified_secs: u64, now_secs: u64) -> String {
    let elapsed = now_secs.saturating_sub(modified_secs);
    let (value, unit) = match elapsed {
        0..60 => return "just now".to_owned(),
        60..3_600 => (elapsed / 60, "minute"),
        3_600..86_400 => (elapsed / 3_600, "hour"),
        86_400..2_592_000 => (elapsed / 86_400, "day"),
        2_592_000..31_536_000 => (elapsed / 2_592_000, "month"),
        _ => (elapsed / 31_536_000, "year"),
    };
    format!("{value} {unit}{} ago", if value == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations_counts_and_ages() {
        assert_eq!(format_duration(65_000), "1:05");
        assert_eq!(format_duration(3_725_000), "1:02:05");
        assert_eq!(format_count(1, "play"), "1 play");
        assert_eq!(format_count(12, "play"), "12 plays");
        assert_eq!(relative_age(0, 30), "just now");
        assert_eq!(relative_age(0, 3 * 86_400), "3 days ago");
        assert_eq!(relative_age(0, 86_400), "1 day ago");
        assert_eq!(prettify("harbour_walk-02.final"), "harbour walk 02 final");
    }

    #[test]
    fn edit_distance_is_levenshtein() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("harbor", "harbour"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
    }
}
