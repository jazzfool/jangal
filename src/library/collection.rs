use super::MediaId;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Collection {
    name: String,
    media: FxHashSet<MediaId>,
}

impl Collection {
    pub fn new() -> Self {
        Collection {
            name: String::from("Untitled Collection"),
            media: FxHashSet::default(),
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn purge_by(&mut self, mut f: impl FnMut(MediaId) -> bool) {
        self.media = FxHashSet::from_iter(self.media.iter().copied().filter(|&id| f(id)));
    }

    pub fn insert(&mut self, id: MediaId) -> bool {
        self.media.insert(id)
    }

    pub fn remove(&mut self, id: MediaId) -> bool {
        self.media.remove(&id)
    }

    pub fn contains(&self, id: MediaId) -> bool {
        self.media.contains(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &MediaId> {
        self.media.iter()
    }
}
