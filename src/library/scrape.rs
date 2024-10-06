use super::MovieMetadata;
use async_std::{io::WriteExt, stream::StreamExt};
use chrono::Datelike;
use std::path::Path;
use tmdb_api::{self as tmdb, prelude::Command, reqwest};

pub trait Scraper {
    async fn scrape_movie_metadata(
        &self,
        storage: &Path,
        filename: &str,
    ) -> anyhow::Result<Option<MovieMetadata>>;
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
        filename: &str,
    ) -> anyhow::Result<Option<MovieMetadata>> {
        let mut query = filename
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|part| !part.is_empty());
        let Some(year_i) = query
            .clone()
            .position(|part| part.len() == 4 && part.chars().all(|c| c.is_ascii_digit()))
        else {
            return Ok(None);
        };
        let title = query.clone().take(year_i).collect::<Vec<&str>>().join(" ");
        let year: u16 = query.nth(year_i).unwrap().parse()?;

        let search = tmdb::movie::search::MovieSearch::new(title).with_year(Some(year));
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
            let mut file = async_std::fs::File::create(&path).await?;

            let mut data = reqwest::get(format!("https://image.tmdb.org/t/p/w200/{}", poster_path))
                .await?
                .bytes_stream();
            while let Some(chunk) = data.next().await {
                file.write(&chunk?);
            }

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
        }))
    }
}
