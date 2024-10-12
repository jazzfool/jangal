use chrono::{Datelike, NaiveDate};
use futures::StreamExt;
use normpath::PathExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
};

const SUPPORTED_EXTENSIONS: &[&str] = &["mp4", "mkv"];

#[repr(transparent)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaId(pub usize);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Library {
    media: HashMap<MediaId, Media>,
    next_id: MediaId,
}

impl Library {
    pub fn new() -> Self {
        Library {
            media: HashMap::new(),
            next_id: MediaId(1),
        }
    }

    pub fn load(storage: &Path) -> Self {
        std::fs::File::open(storage.join("library.json"))
            .ok()
            .and_then(|file| serde_json::from_reader(file).ok())
            .unwrap_or_else(Self::new)
    }

    pub fn save(&self, storage: &Path) -> anyhow::Result<()> {
        let file = std::fs::File::create(storage.join("library.json"))?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    fn generate_id(&mut self) -> MediaId {
        let id = self.next_id;
        self.next_id = MediaId(self.next_id.0 + 1);
        id
    }

    pub fn insert(&mut self, media: Media) -> MediaId {
        let id = self.generate_id();
        self.media.insert(id, media);
        id
    }

    pub fn extend(&mut self, media: impl IntoIterator<Item = Media>) {
        for media in media {
            if self.iter().any(|(_, other)| other.path() == media.path()) {
                continue;
            }
            let id = self.generate_id();
            self.media.insert(id, media);
        }
    }

    pub fn remove(&mut self, id: MediaId) -> Option<Media> {
        self.media.remove(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&MediaId, &Media)> {
        self.media.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&MediaId, &mut Media)> {
        self.media.iter_mut()
    }

    pub fn get(&self, id: MediaId) -> Option<&Media> {
        self.media.get(&id)
    }

    pub fn get_mut(&mut self, id: MediaId) -> Option<&mut Media> {
        self.media.get_mut(&id)
    }
}

async fn scan_file(path: &Path) -> anyhow::Result<Media> {
    let path = path.normalize()?.into_path_buf();

    let extension = path
        .extension()
        .map(|ext| ext.to_str().unwrap().to_string())
        .ok_or_else(|| anyhow::anyhow!("failed to read file extension"))?;

    if !SUPPORTED_EXTENSIONS.contains(&extension.to_lowercase().as_str()) {
        return Err(anyhow::anyhow!("not a video file"));
    }

    Ok(Media::Uncategorised(Uncategorised {
        path,
        dont_scrape: false,
        watched: Watched::No,
    }))
}

pub async fn scan_directories(paths: impl Iterator<Item = &Path>) -> anyhow::Result<Vec<Media>> {
    let mut out: Vec<Media> = vec![];

    let mut queue = VecDeque::new();
    queue.extend(paths.map(|path| path.to_path_buf()));
    while let Some(dir) = queue.pop_front() {
        for entry in std::fs::read_dir(dir)? {
            let Ok(entry) = entry else {
                continue;
            };

            let path = entry.path();

            if entry.file_type()?.is_dir() {
                queue.push_back(path);
            } else {
                if out.iter().any(|media| media.path() == Some(&path)) {
                    continue;
                }

                match scan_file(&path).await {
                    Ok(media) => out.push(media),
                    Err(err) => {
                        log::error!("{:#?}", err)
                    }
                }
            }
        }
    }

    Ok(out)
}

pub async fn purge_media(media: impl Iterator<Item = (MediaId, PathBuf)>) -> Vec<MediaId> {
    futures::stream::iter(media)
        .filter_map(|(id, path)| async move {
            (!async_std::path::Path::new(&path).exists().await).then_some(id)
        })
        .collect()
        .await
}

pub fn find_episodes(
    season: MediaId,
    library: &Library,
) -> impl Iterator<Item = (&MediaId, &Episode)> {
    library.iter().filter_map(move |(id, media)| match media {
        Media::Episode(episode) if episode.season == season => Some((id, episode)),
        _ => None,
    })
}

pub fn find_seasons(
    series: MediaId,
    library: &Library,
) -> impl Iterator<Item = (&MediaId, &Season)> {
    library.iter().filter_map(move |(id, media)| match media {
        Media::Season(season) if season.series == series => Some((id, season)),
        _ => None,
    })
}

pub fn calculate_season_watched(season: MediaId, library: &Library) -> Watched {
    let (count, percent_sum) = find_episodes(season, library)
        .fold((0, 0.0), |(count, percent_sum), (_, episode)| {
            (count + 1, percent_sum + episode.watched.percent())
        });
    let total = percent_sum / count as f32;
    if total < f32::EPSILON {
        Watched::No
    } else if (total - 1.0).abs() < f32::EPSILON {
        Watched::Yes
    } else {
        Watched::Partial {
            seconds: 0.0,
            percent: total,
        }
    }
}

pub fn calculate_series_watched(series: MediaId, library: &Library) -> Watched {
    let (count, percent_sum) =
        find_seasons(series, library).fold((0, 0.0), |(count, percent_sum), (season, _)| {
            (
                count + 1,
                percent_sum + calculate_season_watched(*season, library).percent(),
            )
        });
    let total = percent_sum / count as f32;
    if total < f32::EPSILON {
        Watched::No
    } else if (total - 1.0).abs() < f32::EPSILON {
        Watched::Yes
    } else {
        Watched::Partial {
            seconds: 0.0,
            percent: total,
        }
    }
}

pub fn calculate_watched(id: MediaId, library: &Library) -> Option<Watched> {
    library.get(id).map(|media| {
        media.watched().unwrap_or_else(|| match media {
            Media::Series(_) => calculate_series_watched(id, library),
            Media::Season(_) => calculate_season_watched(id, library),
            _ => unreachable!(),
        })
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Media {
    Uncategorised(Uncategorised),
    Movie(Movie),
    Series(Series),
    Season(Season),
    Episode(Episode),
}

impl Media {
    pub fn full_title(&self) -> Option<String> {
        match self {
            Media::Uncategorised(uncategorised) => {
                Some(uncategorised.path.file_name()?.to_str()?.to_string())
            }
            Media::Movie(movie) => Some(format!(
                "{} ({})",
                movie.metadata.title, movie.metadata.year
            )),
            _ => self.title().map(|s| s.to_string()),
        }
    }

    pub fn title(&self) -> Option<String> {
        match self {
            Media::Uncategorised(uncategorised) => {
                Some(uncategorised.path.file_name()?.to_str()?.to_string())
            }
            Media::Movie(movie) => Some(movie.metadata.title.clone()),
            Media::Series(series) => Some(series.metadata.title.clone()),
            Media::Season(season) => Some(format!("Season {}", season.metadata.season)),
            Media::Episode(episode) => Some(episode.metadata.title.clone()),
        }
    }

    pub fn date(&self) -> Option<NaiveDate> {
        match self {
            Media::Uncategorised(_) => None,
            Media::Movie(movie) => movie.metadata.released,
            Media::Series(series) => series.metadata.aired,
            Media::Season(season) => season.metadata.aired,
            Media::Episode(episode) => Some(episode.metadata.aired),
        }
    }

    pub fn year(&self) -> Option<u16> {
        match self {
            Media::Movie(movie) => Some(movie.metadata.year),
            _ => self.date().map(|date| date.year() as u16),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Media::Uncategorised(uncategorised) => Some(&uncategorised.path),
            Media::Movie(movie) => Some(&movie.path),
            Media::Episode(episode) => Some(&episode.path),
            _ => None,
        }
    }

    pub fn watched(&self) -> Option<Watched> {
        match self {
            Media::Uncategorised(Uncategorised { watched, .. })
            | Media::Movie(Movie { watched, .. })
            | Media::Episode(Episode { watched, .. }) => Some(*watched),
            _ => None,
        }
    }

    pub fn watched_mut(&mut self) -> Option<&mut Watched> {
        match self {
            Media::Uncategorised(Uncategorised { watched, .. })
            | Media::Movie(Movie { watched, .. })
            | Media::Episode(Episode { watched, .. }) => Some(watched),
            _ => None,
        }
    }

    pub fn poster(&self) -> Option<&Path> {
        match self {
            Media::Movie(movie) => Some(&movie.metadata.poster.as_ref()?),
            Media::Series(series) => Some(&series.metadata.poster.as_ref()?),
            Media::Season(season) => Some(&season.metadata.poster.as_ref()?),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Watched {
    No,
    Partial { seconds: f32, percent: f32 },
    Yes,
}

impl Watched {
    pub fn percent(&self) -> f32 {
        match self {
            Watched::No => 0.0,
            Watched::Partial { percent, .. } => *percent,
            Watched::Yes => 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Uncategorised {
    pub path: PathBuf,
    pub dont_scrape: bool,
    pub watched: Watched,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Movie {
    pub path: PathBuf,
    pub watched: Watched,
    pub metadata: MovieMetadata,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Series {
    pub metadata: SeriesMetadata,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Season {
    pub metadata: SeasonMetadata,
    pub series: MediaId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Episode {
    pub path: PathBuf,
    pub series: MediaId,
    pub season: MediaId,
    pub watched: Watched,
    pub metadata: EpisodeMetadata,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Metadata {
    Movie(MovieMetadata),
    Series(SeriesMetadata),
    Season(SeasonMetadata),
    Episode(EpisodeMetadata),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MovieMetadata {
    pub tmdb_id: u64,
    pub title: String,
    pub year: u16,
    pub poster: Option<PathBuf>,
    pub released: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeriesMetadata {
    pub tmdb_id: u64,
    pub title: String,
    pub poster: Option<PathBuf>,
    pub aired: Option<NaiveDate>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeasonMetadata {
    pub series_tmdb_id: u64,
    pub title: String,
    pub season: u16,
    pub poster: Option<PathBuf>,
    pub aired: Option<NaiveDate>,
    pub overview: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EpisodeMetadata {
    pub series_tmdb_id: u64,
    pub title: String,
    pub season: u16,
    pub episode: u16,
    pub aired: NaiveDate,
}
