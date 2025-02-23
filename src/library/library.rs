use super::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[repr(transparent)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaId(pub usize);

#[repr(transparent)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CollectionId(pub usize);

impl Default for CollectionId {
    fn default() -> Self {
        CollectionId(1)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Library {
    media: FxHashMap<MediaId, Media>,
    next_id: MediaId,

    #[serde(default)]
    collections: FxHashMap<CollectionId, Collection>,
    #[serde(default)]
    next_collection_id: CollectionId,
}

impl Library {
    pub fn new() -> Self {
        Library {
            media: FxHashMap::default(),
            next_id: MediaId(1),

            collections: FxHashMap::default(),
            next_collection_id: CollectionId(1),
        }
    }

    pub fn load(storage: &Path) -> Self {
        std::fs::File::open(storage.join("library.json"))
            .ok()
            .and_then(|file| serde_json::from_reader(file).ok())
            .unwrap_or_else(|| {
                if std::fs::exists(storage.join("library.json")).is_ok_and(|x| x) {
                    let _ = std::fs::copy(
                        storage.join("library.json"),
                        storage.join("library.json.bak"),
                    );
                }
                Self::new()
            })
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

    fn generate_collection_id(&mut self) -> CollectionId {
        let id = self.next_collection_id;
        self.next_collection_id = CollectionId(self.next_collection_id.0 + 1);
        id
    }

    pub fn insert_collection(&mut self) -> Option<&mut Collection> {
        let id = self.generate_collection_id();
        self.collections.insert(id, Collection::new());
        self.collection_mut(id)
    }

    pub fn remove_collection(&mut self, id: CollectionId) -> bool {
        self.collections.remove(&id).is_some()
    }

    pub fn iter_collections(&self) -> impl Iterator<Item = (&CollectionId, &Collection)> {
        self.collections.iter()
    }

    pub fn iter_collections_mut(
        &mut self,
    ) -> impl Iterator<Item = (&CollectionId, &mut Collection)> {
        self.collections.iter_mut()
    }

    pub fn purge_collections(&mut self) {
        for collection in self.collections.values_mut() {
            collection.purge_by(|id| self.media.contains_key(&id));
        }
    }

    pub fn collection(&self, id: CollectionId) -> Option<&Collection> {
        self.collections.get(&id)
    }

    pub fn collection_mut(&mut self, id: CollectionId) -> Option<&mut Collection> {
        self.collections.get_mut(&id)
    }

    pub fn collection_iter(
        &self,
        id: &CollectionId,
    ) -> Option<impl Iterator<Item = (&MediaId, &Media)>> {
        self.collections.get(id).map(|collection| {
            collection
                .iter()
                .filter_map(|id| Some((id, self.media.get(id)?)))
        })
    }
}
