use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

use std::collections::hash_map::RandomState;
use std::fmt;

use indexmap::Equivalent;
use indexmap::map::{
    self as inner,
    IndexMap,
};

use crate::id::EntityRange;
use crate::{EntityId, EntityVec};

/// An [`IndexMap`] with strongly-typed indices.
///
/// An `EntityMap` assigns sequential IDs to each inserted `(K, V)` pair. Entries may be looked up
/// either by the key `K`, or the index `I` (by means of the [`Index`] trait).
///
/// ## Accessors at a glance
///
/// | **Accessor**                    | *Requires* | *Obtains*             |
/// |---------------------------------|------------|-----------------------|
/// | `&map[id]`                      | `I`        | `&V`                  |
/// | `&mut map[id]`                  | `I`        | `&mut V`              |
/// | [`key`][EntityMap::key]         | `I`        | `&K`                  |
/// | [`get`][EntityMap::get]         | `&K`       | `Option<(I, &V)>`     |
/// | [`get_mut`][EntityMap::get_mut] | `&K`       | `Option<(I, &mut V)>` |
#[derive(Clone)]
pub struct EntityMap<I: EntityId, K: Hash + Eq, V, RS: BuildHasher = RandomState> {
    map: IndexMap<K, V, RS>,
    ids: PhantomData<I>,
}

impl<I, K, V, RS> EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher,
{
    pub fn with_hasher(h: RS) -> Self {
        Self {
            map: IndexMap::with_hasher(h),
            ids: PhantomData,
        }
    }

    pub fn with_capacity_and_hasher(n: usize, h: RS) -> Self {
        Self {
            map: IndexMap::with_capacity_and_hasher(n, h),
            ids: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn next_id(&self) -> I {
        I::from_idx(self.len())
    }

    pub fn clear(&mut self) {
        self.map.clear()
    }

    pub fn insert(&mut self, k: K, v: V) -> (I, Option<V>) {
        let (i, v) = self.map.insert_full(k, v);
        (I::from_idx(i), v)
    }

    #[track_caller]
    pub fn insert_new(&mut self, k: K, v: V) -> I {
        let (i, f) = self.insert(k, v);
        assert!(f.is_none(), "key already present in EntityMap");
        i
    }

    pub fn key(&self, id: I) -> &K {
        self.map.get_index(id.to_idx()).unwrap().0
    }

    pub fn get<Q>(&self, key: &Q) -> Option<(I, &V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let (i, _, v) = self.map.get_full(key)?;
        Some((I::from_idx(i), v))
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<(I, &mut V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let (i, _, v) = self.map.get_full_mut(key)?;
        Some((I::from_idx(i), v))
    }

    pub fn get_full<Q>(&self, key: &Q) -> Option<(I, &K, &V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let (i, k, v) = self.map.get_full(key)?;
        Some((I::from_idx(i), k, v))
    }

    pub fn get_full_mut<Q>(&mut self, key: &Q) -> Option<(I, &K, &mut V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let (i, k, v) = self.map.get_full_mut(key)?;
        Some((I::from_idx(i), k, v))
    }

    pub fn entry(&mut self, key: K) -> Entry<'_, I, K, V> {
        match self.map.entry(key) {
            inner::Entry::Occupied(entry) => {
                Entry::Occupied(OccupiedEntry(entry, PhantomData))
            }
            inner::Entry::Vacant(entry) => {
                Entry::Vacant(VacantEntry(entry, PhantomData))
            }
        }
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.map.contains_key(key)
    }

    pub fn ids(&self) -> EntityRange<I> {
        EntityRange::new(0, self.len())
    }

    pub fn iter(&self) -> Iter<'_, I, K, V> {
        Iter {
            vals: self.map.iter(),
            pos: 0,
            ids: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, I, K, V> {
        IterMut {
            vals: self.map.iter_mut(),
            pos: 0,
            ids: PhantomData,
        }
    }

    pub fn keys(&self) -> indexmap::map::Keys<'_, K, V> {
        self.map.keys()
    }

    pub fn into_keys(self) -> indexmap::map::IntoKeys<K, V> {
        self.map.into_keys()
    }

    pub fn values(&self) -> indexmap::map::Values<'_, K, V> {
        self.map.values()
    }

    pub fn values_mut(&mut self) -> indexmap::map::ValuesMut<'_, K, V> {
        self.map.values_mut()
    }

    pub fn into_values(self) -> indexmap::map::IntoValues<K, V> {
        self.map.into_values()
    }

    pub fn into_vec(self) -> EntityVec<I, (K, V)> {
        self.into_iter().map(|(_, k, v)| (k, v)).collect()
    }
}

impl<I, K, V> EntityMap<I, K, V>
where
    I: EntityId,
    K: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            map: IndexMap::new(),
            ids: PhantomData,
        }
    }

    pub fn with_capacity(n: usize) -> Self {
        Self {
            map: IndexMap::with_capacity(n),
            ids: PhantomData,
        }
    }
}

impl<I, K, V> Default for EntityMap<I, K, V>
where
    I: EntityId,
    K: Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<I, K, V, RS> fmt::Debug for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq + fmt::Debug,
    V: fmt::Debug,
    RS: BuildHasher,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_map()
            .entries(self.iter().map(|(i, k, v)| (i, (k, v))))
            .finish()
    }
}

impl<I, K, V, RS, RS2> PartialEq<EntityMap<I, K, V, RS2>> for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    V: PartialEq,
    RS: BuildHasher,
    RS2: BuildHasher,
{
    fn eq(&self, other: &EntityMap<I, K, V, RS2>) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<I, K, V, RS> Eq for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    V: Eq,
    RS: BuildHasher,
{
}

impl<I, K, V, RS> IntoIterator for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher,
{
    type Item = (I, K, V);
    type IntoIter = IntoIter<I, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            vals: self.map.into_iter(),
            pos: 0,
            ids: PhantomData,
        }
    }
}

impl<'a, I, K, V, RS> IntoIterator for &'a EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher,
{
    type Item = (I, &'a K, &'a V);
    type IntoIter = Iter<'a, I, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, I, K, V, RS> IntoIterator for &'a mut EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher,
{
    type Item = (I, &'a K, &'a mut V);
    type IntoIter = IterMut<'a, I, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<I, K, V, RS> Index<I> for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher,
{
    type Output = V;
    fn index(&self, index: I) -> &V {
        self.map.index(index.to_idx())
    }
}

impl<I, K, V, RS> IndexMut<I> for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher,
{
    fn index_mut(&mut self, index: I) -> &mut V {
        self.map.index_mut(index.to_idx())
    }
}

#[derive(Clone, Debug)]
pub struct Iter<'a, I, K: Hash, V> {
    vals: indexmap::map::Iter<'a, K, V>,
    pos: usize,
    ids: PhantomData<I>,
}

impl<'a, I, K, V> Iterator for Iter<'a, I, K, V>
where
    I: EntityId,
    K: Hash,
{
    type Item = (I, &'a K, &'a V);
    fn next(&mut self) -> Option<(I, &'a K, &'a V)> {
        let (key, val) = self.vals.next()?;
        let id = I::from_idx(self.pos);
        self.pos += 1;
        Some((id, key, val))
    }
}

impl<'a, I, K, V> DoubleEndedIterator for Iter<'a, I, K, V>
where
    I: EntityId,
    K: Hash,
{
    fn next_back(&mut self) -> Option<(I, &'a K, &'a V)> {
        let (key, val) = self.vals.next_back()?;
        Some((I::from_idx(self.pos + self.vals.len()), key, val))
    }
}

impl<'a, I, K, V> ExactSizeIterator for Iter<'a, I, K, V>
where
    I: EntityId,
    K: Hash,
{
    fn len(&self) -> usize {
        self.vals.len()
    }
}

pub struct IterMut<'a, I, K: Hash, V> {
    vals: indexmap::map::IterMut<'a, K, V>,
    pos: usize,
    ids: PhantomData<I>,
}

impl<'a, I, K, V> Iterator for IterMut<'a, I, K, V>
where
    I: EntityId,
    K: Hash,
{
    type Item = (I, &'a K, &'a mut V);
    fn next(&mut self) -> Option<(I, &'a K, &'a mut V)> {
        let (key, val) = self.vals.next()?;
        let id = I::from_idx(self.pos);
        self.pos += 1;
        Some((id, key, val))
    }
}

impl<'a, I, K, V> DoubleEndedIterator for IterMut<'a, I, K, V>
where
    I: EntityId,
    K: Hash,
{
    fn next_back(&mut self) -> Option<(I, &'a K, &'a mut V)> {
        let (key, val) = self.vals.next_back()?;
        Some((I::from_idx(self.pos + self.vals.len()), key, val))
    }
}

impl<'a, I, K, V> ExactSizeIterator for IterMut<'a, I, K, V>
where
    I: EntityId,
    K: Hash,
{
    fn len(&self) -> usize {
        self.vals.len()
    }
}

#[derive(Debug)]
pub struct IntoIter<I, K: Hash, V> {
    vals: indexmap::map::IntoIter<K, V>,
    pos: usize,
    ids: PhantomData<I>,
}

impl<I, K, V> Iterator for IntoIter<I, K, V>
where
    I: EntityId,
    K: Hash,
{
    type Item = (I, K, V);
    fn next(&mut self) -> Option<(I, K, V)> {
        let (key, val) = self.vals.next()?;
        let id = I::from_idx(self.pos);
        self.pos += 1;
        Some((id, key, val))
    }
}

impl<I, K, V> DoubleEndedIterator for IntoIter<I, K, V>
where
    I: EntityId,
    K: Hash,
{
    fn next_back(&mut self) -> Option<(I, K, V)> {
        let (key, val) = self.vals.next_back()?;
        Some((I::from_idx(self.pos + self.vals.len()), key, val))
    }
}

impl<I, K, V> ExactSizeIterator for IntoIter<I, K, V>
where
    I: EntityId,
    K: Hash,
{
    fn len(&self) -> usize {
        self.vals.len()
    }
}

impl<I, K, V, RS> FromIterator<(K, V)> for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher + Default,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        Self {
            map: IndexMap::from_iter(iter),
            ids: PhantomData,
        }
    }
}

impl<I, K, V, RS> Extend<(K, V)> for EntityMap<I, K, V, RS>
where
    I: EntityId,
    K: Hash + Eq,
    RS: BuildHasher + Default,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        self.map.extend(iter);
    }
}

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "bincode")]
mod bincode;

pub enum Entry<'a, I, K, V> {
    Occupied(OccupiedEntry<'a, I, K, V>),
    Vacant(VacantEntry<'a, I, K, V>),
}

pub struct OccupiedEntry<'a, I, K, V>(inner::OccupiedEntry<'a, K, V>, PhantomData<I>);
pub struct VacantEntry<'a, I, K, V>(inner::VacantEntry<'a, K, V>, PhantomData<I>);

impl<'a, I: EntityId, K, V> OccupiedEntry<'a, I, K, V> {
    /// Return the index of the key-value pair.
    #[inline]
    pub fn index(&self) -> I {
        I::from_idx(self.0.index())
    }

    #[inline]
    pub fn key(&self) -> &K {
        self.0.key()
    }

    #[inline]
    pub fn get(&self) -> &V {
        self.0.get()
    }

    #[inline]
    pub fn get_mut(&mut self) -> &mut V {
        self.0.get_mut()
    }

    #[inline]
    pub fn into_mut(self) -> &'a mut V {
        self.0.into_mut()
    }

    /// Sets the value of the entry to `value`, and returns the entry's old value.
    #[inline]
    pub fn insert(&mut self, value: V) -> V {
        self.0.insert(value)
    }
}

impl<'a, I: EntityId, K, V> VacantEntry<'a, I, K, V> {
    /// Return the index where the key-value pair may be inserted.
    #[inline]
    pub fn index(&self) -> I {
        I::from_idx(self.0.index())
    }

    #[inline]
    pub fn key(&self) -> &K {
        self.0.key()
    }

    #[inline]
    pub fn into_key(self) -> K {
        self.0.into_key()
    }

    #[inline]
    pub fn insert(self, value: V) -> &'a mut V {
        self.0.insert(value)
    }

    #[inline]
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, I, K, V> {
        OccupiedEntry(self.0.insert_entry(value), PhantomData)
    }
}

impl<'a, I: EntityId, K, V> Entry<'a, I, K, V> {
    #[inline]
    pub fn index(&self) -> I {
        match self {
            Entry::Occupied(entry) => entry.index(),
            Entry::Vacant(entry) => entry.index(),
        }
    }

    /// Sets the value of the entry (after inserting if vacant), and returns an `OccupiedEntry`.
    ///
    /// If the entry is already occupied, the old value will be discarded.
    #[inline]
    pub fn insert_entry(self, value: V) -> OccupiedEntry<'a, I, K, V> {
        match self {
            Entry::Occupied(mut entry) => {
                entry.insert(value);
                entry
            }
            Entry::Vacant(entry) => entry.insert_entry(value),
        }
    }

    /// Inserts the given default value in the entry if it is vacant and returns a mutable
    /// reference to it. Otherwise a mutable reference to an already existent value is returned.
    #[inline]
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Like [`or_insert`][Self::or_insert], but allows constructing the default value lazily.
    #[inline]
    pub fn or_insert_with<F>(self, f: F) -> &'a mut V
    where
        F: FnOnce() -> V,
    {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(f()),
        }
    }

    /// Like [`or_insert_with`][Self::or_insert_with], but also provides a reference to the key.
    #[inline]
    pub fn or_insert_with_key<F>(self, f: F) -> &'a mut V
    where
        F: FnOnce(&K) -> V,
    {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let value = f(entry.key());
                entry.insert(value)
            }
        }
    }

    #[inline]
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(entry) => entry.key(),
            Entry::Vacant(entry) => entry.key(),
        }
    }

    /// Modifies the entry if it is occupied.
    #[inline]
    pub fn and_modify<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        if let Entry::Occupied(entry) = &mut self {
            f(entry.get_mut());
        }
        self
    }

    /// Inserts a default-constructed value in the entry if it is vacant and returns a mutable
    /// reference to it. Otherwise a mutable reference to an already existent value is returned.
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(V::default()),
        }
    }
}
