use crate::{EntityId, EntityRange, EntityVec};
use indexmap::IndexMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EntityBundleIndex<I: EntityId> {
    Single(I),
    Array(EntityRange<I>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EntityBundleItemIndex {
    Single,
    Array { index: usize, total: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityBundleMap<I: EntityId, T> {
    ids: EntityVec<I, usize>,
    bundles: IndexMap<String, (EntityBundleIndex<I>, T)>,
}

impl<I: EntityId, T> EntityBundleMap<I, T> {
    pub fn new() -> Self {
        Self {
            ids: Default::default(),
            bundles: Default::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn ids(&self) -> EntityRange<I> {
        self.ids.ids()
    }

    pub fn get(&self, key: &str) -> Option<(EntityBundleIndex<I>, &T)> {
        let &(idx, ref val) = self.bundles.get(key)?;
        Some((idx, val))
    }

    pub fn get_mut(&mut self, key: &str) -> Option<(EntityBundleIndex<I>, &mut T)> {
        let &mut (idx, ref mut val) = self.bundles.get_mut(key)?;
        Some((idx, val))
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.bundles.contains_key(key)
    }

    pub fn key(&self, id: I) -> (&str, EntityBundleItemIndex) {
        let (key, &(bidx, _)) = self.bundles.get_index(self.ids[id]).unwrap();
        match bidx {
            EntityBundleIndex::Single(sid) => {
                assert_eq!(id, sid);
                (key, EntityBundleItemIndex::Single)
            }
            EntityBundleIndex::Array(range) => (
                key,
                EntityBundleItemIndex::Array {
                    index: range.index_of(id).unwrap(),
                    total: range.len(),
                },
            ),
        }
    }

    pub fn insert(&mut self, name: String, value: T) -> Option<I> {
        match self.bundles.entry(name) {
            indexmap::map::Entry::Occupied(_) => None,
            indexmap::map::Entry::Vacant(e) => {
                let id = self.ids.push(e.index());
                e.insert((EntityBundleIndex::Single(id), value));
                Some(id)
            }
        }
    }

    pub fn insert_array(&mut self, name: String, num: usize, value: T) -> Option<EntityRange<I>> {
        match self.bundles.entry(name) {
            indexmap::map::Entry::Occupied(_) => None,
            indexmap::map::Entry::Vacant(e) => {
                let id = self.ids.next_id();
                let range = EntityRange::new(id.to_idx(), id.to_idx() + num);
                for _ in 0..num {
                    self.ids.push(e.index());
                }
                e.insert((EntityBundleIndex::Array(range), value));
                Some(range)
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (I, &str, EntityBundleItemIndex, &T)> {
        self.ids.ids().map(|id| {
            let (idx, name) = self.key(id);
            (id, idx, name, &self[id])
        })
    }

    pub fn bundles(&self) -> impl Iterator<Item = (EntityBundleIndex<I>, &str, &T)> {
        self.bundles
            .iter()
            .map(|(k, &(i, ref v))| (i, k.as_str(), v))
    }

    pub fn bundles_mut(&mut self) -> impl Iterator<Item = (EntityBundleIndex<I>, &str, &mut T)> {
        self.bundles
            .iter_mut()
            .map(|(k, &mut (i, ref mut v))| (i, k.as_str(), v))
    }

    pub fn into_bundles(self) -> impl Iterator<Item = (EntityBundleIndex<I>, String, T)> {
        self.bundles.into_iter().map(|(k, (i, v))| (i, k, v))
    }
}

impl<I: EntityId, T> Default for EntityBundleMap<I, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: EntityId, T> core::ops::Index<I> for EntityBundleMap<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        let idx = self.ids[index];
        &self.bundles.get_index(idx).unwrap().1.1
    }
}

impl<I: EntityId, T> core::ops::IndexMut<I> for EntityBundleMap<I, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let idx = self.ids[index];
        &mut self.bundles.get_index_mut(idx).unwrap().1.1
    }
}

#[cfg(feature = "bincode")]
mod bincode;
