use futures::{future, StreamExt};
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

#[derive(Serialize, Deserialize, Debug)]
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

    Ok(Media::Movie(Movie {
        path,
        metadata: None,
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
            async_std::path::Path::new(&path)
                .exists()
                .await
                .then_some(id)
        })
        .collect()
        .await
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Media {
    Movie(Movie),
    Series(Series),
    // Season(Season),
    // Episode(Episode),
}

impl Media {
    pub fn full_title(&self) -> Option<String> {
        match self {
            Media::Movie(_) => Some(format!("{} ({})", self.title()?, self.year()?)),
            _ => todo!(),
        }
    }

    pub fn title(&self) -> Option<&str> {
        match self {
            Media::Movie(movie) => movie
                .metadata
                .as_ref()
                .map(|metadata| metadata.title.as_str()),
            _ => todo!(),
        }
    }

    pub fn year(&self) -> Option<u16> {
        match self {
            Media::Movie(movie) => movie.metadata.as_ref().map(|metadata| metadata.year),
            _ => todo!(),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            Media::Movie(movie) => Some(&movie.path),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MovieMetadata {
    pub tmdb_id: u64,
    pub title: String,
    pub year: u16,
    pub poster: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Movie {
    pub path: PathBuf,
    pub metadata: Option<MovieMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Series {}
