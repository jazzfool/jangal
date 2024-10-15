use super::*;
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Media {
    Uncategorised(Uncategorised),
    Movie(Movie),
    Series(Series),
    Season(Season),
    Episode(Episode),
}

impl Media {
    pub fn title(&self) -> String {
        match self {
            Media::Uncategorised(uncategorised) => uncategorised
                .video
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            Media::Movie(movie) => movie.metadata.title.clone(),
            Media::Series(series) => series.metadata.title.clone(),
            Media::Season(season) => format!("Season {}", season.metadata.season),
            Media::Episode(episode) => episode.metadata.title.clone(),
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

    pub fn video(&self) -> Option<&Video> {
        match self {
            Media::Uncategorised(Uncategorised { video, .. })
            | Media::Movie(Movie { video, .. })
            | Media::Episode(Episode { video, .. }) => Some(video),
            _ => None,
        }
    }

    pub fn video_mut(&mut self) -> Option<&mut Video> {
        match self {
            Media::Uncategorised(Uncategorised { video, .. })
            | Media::Movie(Movie { video, .. })
            | Media::Episode(Episode { video, .. }) => Some(video),
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
pub struct Video {
    pub path: PathBuf,
    pub watched: Watched,
    pub added: chrono::DateTime<chrono::Local>,
    pub last_watched: Option<chrono::DateTime<chrono::Local>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Uncategorised {
    pub video: Video,
    pub dont_scrape: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Movie {
    pub video: Video,
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
    pub video: Video,
    pub series: MediaId,
    pub season: MediaId,
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
