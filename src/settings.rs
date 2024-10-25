use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserSettings {
    pub tmdb_secret: String,
    pub directories: Vec<PathBuf>,
    pub show_subtitles: bool,

    pub watch_threshold_movies: u32,
    pub watch_threshold_episodes: u32,
}

impl UserSettings {
    pub fn new() -> Self {
        UserSettings {
            tmdb_secret: String::new(),
            directories: vec![],
            show_subtitles: false,

            watch_threshold_movies: 15,
            watch_threshold_episodes: 2,
        }
    }

    pub fn load(storage: &Path) -> Self {
        std::fs::File::open(storage.join("user.json"))
            .ok()
            .and_then(|file| serde_json::from_reader(file).ok())
            .unwrap_or_else(Self::new)
    }

    pub fn save(&self, storage: &Path) -> anyhow::Result<()> {
        let file = std::fs::File::create(storage.join("user.json"))?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }
}
