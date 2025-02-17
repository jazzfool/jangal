use super::*;
use futures::StreamExt;
use itertools::Itertools;
use normpath::PathExt;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

const SUPPORTED_EXTENSIONS: &[&str] = &["mp4", "mkv"];

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
        video: Video {
            path,
            watched: Watched::No,
            added: chrono::Local::now(),
            last_watched: None,
        },
        dont_scrape: false,
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
                if out
                    .iter()
                    .any(|media| media.video().map(|video| &video.path) == Some(&path))
                {
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

pub fn full_title(id: MediaId, library: &Library) -> String {
    library
        .get(id)
        .map(|media| match media {
            Media::Movie(movie) => format!("{} ({})", movie.metadata.title, movie.metadata.year),
            Media::Episode(episode) => format!(
                "{} S{:02}E{:02} - {}",
                library.get(episode.series).unwrap().title(),
                episode.metadata.season,
                episode.metadata.episode,
                episode.metadata.title,
            ),
            Media::Season(season) => format!(
                "{} S{:02} - {}",
                library.get(season.series).unwrap().title(),
                season.metadata.season,
                season.metadata.title,
            ),
            _ => media.title(),
        })
        .unwrap_or_else(|| String::from("Unknown Media"))
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

pub fn find_all_episodes(
    series: MediaId,
    library: &Library,
) -> impl Iterator<Item = (&MediaId, &Episode)> {
    find_seasons(series, library).flat_map(|(season, _)| find_episodes(*season, library))
}

pub fn calculate_season_watched(season: MediaId, library: &Library) -> Watched {
    let (count, percent_sum) =
        find_episodes(season, library).fold((0, 0.0), |(count, percent_sum), (_, episode)| {
            (count + 1, percent_sum + episode.video.watched.percent())
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
        media
            .video()
            .map(|video| video.watched)
            .unwrap_or_else(|| match media {
                Media::Series(_) => calculate_series_watched(id, library),
                Media::Season(_) => calculate_season_watched(id, library),
                _ => unreachable!(),
            })
    })
}

pub fn previous_in_list(id: MediaId, library: &Library) -> Option<MediaId> {
    library.get(id).and_then(|media| match media {
        Media::Episode(episode) => {
            let se = (episode.metadata.season, episode.metadata.episode);
            let mut episodes = find_all_episodes(episode.series, library)
                .filter(|(_, e)| (e.metadata.season, e.metadata.episode) < se)
                .collect_vec();
            episodes.sort_unstable_by_key(|(_, episode)| {
                (episode.metadata.season, episode.metadata.episode)
            });
            episodes.last().map(|(id, _)| **id)
        }
        _ => None,
    })
}

pub fn next_in_list(id: MediaId, library: &Library) -> Option<MediaId> {
    library.get(id).and_then(|media| match media {
        Media::Episode(episode) => {
            let se = (episode.metadata.season, episode.metadata.episode);
            let mut episodes = find_all_episodes(episode.series, library)
                .filter(|(_, e)| (e.metadata.season, e.metadata.episode) > se)
                .collect_vec();
            episodes.sort_unstable_by_key(|(_, episode)| {
                (episode.metadata.season, episode.metadata.episode)
            });
            episodes.first().map(|(id, _)| **id)
        }
        _ => None,
    })
}

pub fn season_last_watched(season: MediaId, library: &Library) -> Option<(&MediaId, &Episode)> {
    find_episodes(season, library).max_by_key(|(_, episode)| episode.video.last_watched)
}

pub fn series_last_watched(season: MediaId, library: &Library) -> Option<(&MediaId, &Episode)> {
    find_all_episodes(season, library).max_by_key(|(_, episode)| episode.video.last_watched)
}

pub fn last_watched(id: MediaId, library: &Library) -> Option<chrono::DateTime<chrono::Local>> {
    let media = library.get(id)?;
    if let Some(video) = media.video() {
        Some(video.last_watched?)
    } else {
        match media {
            Media::Series(_) => series_last_watched(id, library),
            Media::Season(_) => season_last_watched(id, library),
            _ => unreachable!(),
        }?
        .1
        .video
        .last_watched
    }
}

pub fn season_date_added(season: MediaId, library: &Library) -> Option<(&MediaId, &Episode)> {
    find_episodes(season, library).max_by_key(|(_, episode)| episode.video.added)
}

pub fn series_date_added(season: MediaId, library: &Library) -> Option<(&MediaId, &Episode)> {
    find_all_episodes(season, library).max_by_key(|(_, episode)| episode.video.added)
}

pub fn date_added(id: MediaId, library: &Library) -> Option<chrono::DateTime<chrono::Local>> {
    let media = library.get(id)?;
    if let Some(video) = media.video() {
        Some(video.added)
    } else {
        Some(
            match media {
                Media::Series(_) => series_last_watched(id, library),
                Media::Season(_) => season_last_watched(id, library),
                _ => unreachable!(),
            }?
            .1
            .video
            .added,
        )
    }
}

pub fn set_watched(id: MediaId, value: Watched, library: &mut Library) {
    // borrow library immutably first
    let targets = match library.get(id) {
        Some(Media::Series(_)) => find_all_episodes(id, library).map(|(id, _)| *id).collect(),
        Some(Media::Season(_)) => find_episodes(id, library).map(|(id, _)| *id).collect(),
        Some(_) => vec![id],
        _ => return,
    };
    for id in targets {
        if let Some(watched) = library
            .get_mut(id)
            .and_then(Media::video_mut)
            .map(|video| &mut video.watched)
        {
            *watched = value;
        }
    }
}
