use super::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[repr(transparent)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaId(pub usize);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Library {
    media: FxHashMap<MediaId, Media>,
    next_id: MediaId,
}

impl Library {
    pub fn new() -> Self {
        Library {
            media: FxHashMap::default(),
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
            if self.iter().any(|(_, other)| {
                other.video().map(|video| &video.path) == media.video().map(|video| &video.path)
            }) {
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
