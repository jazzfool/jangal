use super::{
    Episode, EpisodeMetadata, Library, Media, MediaId, Movie, MovieMetadata, Season,
    SeasonMetadata, Series, SeriesMetadata,
};
use async_std::stream::StreamExt;
use chrono::Datelike;
use futures::{AsyncWriteExt, Future};
use regex::Regex;
use std::{path::Path, sync::LazyLock};
use tmdb_api::{self as tmdb, prelude::Command, reqwest};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MediaType {
    Unknown,
    Movie {
        title: String,
        year: u16,
    },
    Episode {
        series_title: String,
        season: u16,
        episode: u16,
    },
}

pub fn detect_media_type(filename: &str) -> MediaType {
    static RE_ALPHANUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-zA-Z\d]+").unwrap());
    static RE_MOVIES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        vec![
            // movie 2000
            Regex::new(r"^([a-z\d\s]+) (\d{4}) ").unwrap(),
        ]
    });
    static RE_EPISODES: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        vec![
            // tv show 2000 s01e01
            Regex::new(r"^([a-z\d\s]+) \d{4} s(\d+)e(\d+) ").unwrap(),
            // tv show s01e01
            Regex::new(r"^([a-z\d\s]+) s(\d+)e(\d+) ").unwrap(),
        ]
    });

    let clean = RE_ALPHANUM.replace_all(filename, " ").to_lowercase();

    for re in &*RE_EPISODES {
        if let Some(capture) = re.captures(&clean) {
            return MediaType::Episode {
                series_title: capture.get(1).unwrap().as_str().into(),
                season: capture.get(2).unwrap().as_str().parse().unwrap(),
                episode: capture.get(3).unwrap().as_str().parse().unwrap(),
            };
        }
    }

    for re in &*RE_MOVIES {
        if let Some(capture) = re.captures(&clean) {
            return MediaType::Movie {
                title: capture.get(1).unwrap().as_str().into(),
                year: capture.get(2).unwrap().as_str().parse().unwrap(),
            };
        }
    }

    MediaType::Unknown
}

pub trait Scraper {
    async fn scrape_movie_metadata(
        &self,
        storage: &Path,
        title: &str,
        year: u16,
    ) -> anyhow::Result<Option<MovieMetadata>>;
    async fn scrape_series_metadata(
        &self,
        storage: &Path,
        title: &str,
    ) -> anyhow::Result<Option<SeriesMetadata>>;
    async fn scrape_season_metadata(
        &self,
        storage: &Path,
        series_id: u64,
        season: u16,
    ) -> anyhow::Result<Option<(SeasonMetadata, Vec<EpisodeMetadata>)>>;
}

async fn download_image(name: &str, dest: &Path) -> anyhow::Result<()> {
    if dest.exists() {
        return Ok(());
    }

    let mut data = reqwest::get(format!("https://image.tmdb.org/t/p/w200/{}", name))
        .await?
        .bytes_stream();

    let mut file = async_std::fs::File::create(dest).await?;
    while let Some(chunk) = data.next().await {
        file.write_all(&chunk?).await?;
    }

    file.flush().await?;

    Ok(())
}

pub struct TmdbScraper {
    client: tmdb::client::ReqwestClient,
}

impl TmdbScraper {
    pub fn new(secret: &str) -> Self {
        let client = tmdb::client::ReqwestClient::new(secret.into());
        TmdbScraper { client }
    }
}

impl Scraper for TmdbScraper {
    async fn scrape_movie_metadata(
        &self,
        storage: &Path,
        title: &str,
        year: u16,
    ) -> anyhow::Result<Option<MovieMetadata>> {
        let search = tmdb::movie::search::MovieSearch::new(title.to_string()).with_year(Some(year));
        let result = search
            .execute(&self.client)
            .await
            .map_err(|_| anyhow::anyhow!("tmdb movie search failed"))?;

        let Some(result) = result.results.first().map(|movie| movie.inner.clone()) else {
            return Ok(None);
        };

        let poster = if let Some(poster_path) = result.poster_path {
            let poster_path = poster_path.replace('/', "");
            let path = storage.join(&poster_path);
            download_image(&poster_path, &path).await?;
            Some(path)
        } else {
            None
        };

        Ok(Some(MovieMetadata {
            tmdb_id: result.id,
            title: result.title,
            year: result
                .release_date
                .map(|date| date.year() as u16)
                .unwrap_or(0),
            poster,
            released: result.release_date,
        }))
    }

    async fn scrape_series_metadata(
        &self,
        storage: &Path,
        title: &str,
    ) -> anyhow::Result<Option<SeriesMetadata>> {
        let search = tmdb::tvshow::search::TVShowSearch::new(title.to_string());
        let result = search
            .execute(&self.client)
            .await
            .map_err(|_| anyhow::anyhow!("tmdb tv show search failed"))?;

        let Some(result) = result.results.first().map(|series| series.inner.clone()) else {
            return Ok(None);
        };

        let poster = if let Some(poster_path) = result.poster_path {
            let poster_path = poster_path.replace('/', "");
            let path = storage.join(&poster_path);
            download_image(&poster_path, &path).await?;
            Some(path)
        } else {
            None
        };

        Ok(Some(SeriesMetadata {
            tmdb_id: result.id,
            title: result.name,
            poster,
            aired: result.first_air_date,
        }))
    }

    async fn scrape_season_metadata(
        &self,
        storage: &Path,
        series_id: u64,
        season: u16,
    ) -> anyhow::Result<Option<(SeasonMetadata, Vec<EpisodeMetadata>)>> {
        let details =
            tmdb::tvshow::season::details::TVShowSeasonDetails::new(series_id, season as _)
                .execute(&self.client)
                .await
                .map_err(|err| anyhow::anyhow!("tmdb tv show season details failed: {}", err))?;

        let poster = if let Some(poster_path) = details.inner.poster_path {
            let poster_path = poster_path.replace('/', "");
            let path = storage.join(&poster_path);
            download_image(&poster_path, &path).await?;
            Some(path)
        } else {
            None
        };

        let episodes = details
            .episodes
            .into_iter()
            .map(|episode| EpisodeMetadata {
                series_tmdb_id: series_id,
                title: episode.inner.name,
                season,
                episode: episode.inner.episode_number as _,
                aired: episode.inner.air_date,
            })
            .collect();

        Ok(Some((
            SeasonMetadata {
                series_tmdb_id: series_id,
                title: details.inner.name,
                season,
                poster,
                aired: details.inner.air_date,
                overview: details.inner.overview,
            },
            episodes,
        )))
    }
}

#[derive(Debug, Clone)]
struct SeasonScrapeResult {
    metadata: SeasonMetadata,
    episodes: Vec<(MediaId, EpisodeMetadata)>,
    unmatched: Vec<EpisodeMetadata>,
}

#[derive(Debug, Clone)]
struct SeriesScrapeResult {
    metadata: SeriesMetadata,
    seasons: Vec<SeasonScrapeResult>,
}

#[derive(Debug, Clone)]
pub struct ScrapeResult {
    movies: Vec<(MediaId, MovieMetadata)>,
    series: Vec<SeriesScrapeResult>,
}

impl ScrapeResult {
    pub fn insert(self, library: &mut Library) {
        let Self { movies, series } = self;

        for (id, metadata) in movies {
            let Some(media) = library.get_mut(id) else {
                continue;
            };

            let Media::Uncategorised(uncategorised) = media else {
                continue;
            };

            *media = Media::Movie(Movie {
                video: uncategorised.video.clone(),
                metadata,
            });
        }

        for series in series {
            let series_id = library.iter().find_map(|(id, media)| match media {
                Media::Series(other) if other.metadata.tmdb_id == series.metadata.tmdb_id => {
                    Some(*id)
                }
                _ => None,
            });
            let series_id = series_id.unwrap_or_else(|| {
                library.insert(Media::Series(Series {
                    metadata: series.metadata,
                }))
            });

            for season in series.seasons {
                let season_id = library.iter().find_map(|(id, media)| match media {
                    Media::Season(other)
                        if other.series == series_id
                            && other.metadata.season == season.metadata.season =>
                    {
                        Some(*id)
                    }
                    _ => None,
                });
                let season_id = season_id.unwrap_or_else(|| {
                    library.insert(Media::Season(Season {
                        metadata: season.metadata,
                        series: series_id,
                    }))
                });

                for (id, metadata) in season.episodes {
                    let Some(media) = library.get_mut(id) else {
                        continue;
                    };

                    let Media::Uncategorised(uncategorised) = media else {
                        continue;
                    };

                    *media = Media::Episode(Episode {
                        video: uncategorised.video.clone(),
                        series: series_id,
                        season: season_id,
                        metadata,
                    });
                }
            }
        }
    }
}

async fn find_or_insert<T, F>(
    v: &mut Vec<T>,
    pred: impl FnMut(&T) -> bool,
    insert: impl FnOnce() -> F,
) -> Option<&mut T>
where
    F: Future<Output = Option<T>>,
{
    if let Some(i) = v.iter().position(pred) {
        Some(&mut v[i])
    } else {
        if let Some(insert) = insert().await {
            v.push(insert);
            v.last_mut()
        } else {
            None
        }
    }
}

pub async fn scrape_all(
    scraper: &impl Scraper,
    storage: &Path,
    media: impl Iterator<Item = (MediaId, String)>,
) -> ScrapeResult {
    let mut result = ScrapeResult {
        movies: vec![],
        series: vec![],
    };

    for (id, filename) in media {
        let media_type = detect_media_type(&filename);
        match media_type {
            MediaType::Unknown => {}
            MediaType::Movie { title, year } => {
                if let Ok(Some(metadata)) =
                    scraper.scrape_movie_metadata(storage, &title, year).await
                {
                    result.movies.push((id, metadata));
                }
            }
            MediaType::Episode {
                series_title,
                season,
                episode,
            } => {
                let Some(series) = find_or_insert(
                    &mut result.series,
                    |series| series.metadata.title == series_title,
                    || async {
                        Some(SeriesScrapeResult {
                            metadata: scraper
                                .scrape_series_metadata(storage, &series_title)
                                .await
                                .ok()??,
                            seasons: vec![],
                        })
                    },
                )
                .await
                else {
                    continue;
                };

                if let Some(season) = find_or_insert(
                    &mut series.seasons,
                    |s| s.metadata.season == season,
                    || async {
                        let (metadata, episodes) = scraper
                            .scrape_season_metadata(storage, series.metadata.tmdb_id, season)
                            .await
                            .ok()??;
                        Some(SeasonScrapeResult {
                            metadata,
                            episodes: vec![],
                            unmatched: episodes,
                        })
                    },
                )
                .await
                {
                    let Some(i) = season.unmatched.iter().position(|e| e.episode == episode) else {
                        continue;
                    };
                    let metadata = season.unmatched.remove(i);
                    season.episodes.push((id, metadata));
                }
            }
        }
    }

    result
}
