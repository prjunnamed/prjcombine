use std::collections::{BTreeMap, btree_map};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BiMap<L: Ord + Clone, R: Ord + Clone> {
    ltr: BTreeMap<L, R>,
    rtl: BTreeMap<R, L>,
}

impl<L: Ord + Clone, R: Ord + Clone> BiMap<L, R> {
    pub fn new() -> Self {
        Self {
            ltr: Default::default(),
            rtl: Default::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.ltr.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ltr.is_empty()
    }

    pub fn get_left(&self, key: &L) -> Option<&R> {
        self.ltr.get(key)
    }

    pub fn get_right(&self, key: &R) -> Option<&L> {
        self.rtl.get(key)
    }

    pub fn insert(&mut self, l: L, r: R) {
        match self.ltr.entry(l.clone()) {
            btree_map::Entry::Vacant(e) => {
                assert!(!self.rtl.contains_key(&r));
                e.insert(r.clone());
                self.rtl.insert(r, l);
            }
            btree_map::Entry::Occupied(e) => {
                assert!(*e.get() == r);
            }
        }
    }

    pub fn iter(&self) -> btree_map::Iter<'_, L, R> {
        self.ltr.iter()
    }

    pub fn keys_left(&self) -> btree_map::Keys<'_, L, R> {
        self.ltr.keys()
    }

    pub fn keys_right(&self) -> btree_map::Values<'_, L, R> {
        self.ltr.values()
    }
}

impl<L: Ord + Clone, R: Ord + Clone> Default for BiMap<L, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, L: Ord + Clone, R: Ord + Clone> IntoIterator for &'a BiMap<L, R> {
    type Item = (&'a L, &'a R);

    type IntoIter = btree_map::Iter<'a, L, R>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<L: Ord + Clone, R: Ord + Clone> IntoIterator for BiMap<L, R> {
    type Item = (L, R);

    type IntoIter = btree_map::IntoIter<L, R>;

    fn into_iter(self) -> Self::IntoIter {
        self.ltr.into_iter()
    }
}

impl<L: Ord + Clone, R: Ord + Clone> FromIterator<(L, R)> for BiMap<L, R> {
    fn from_iter<T: IntoIterator<Item = (L, R)>>(iter: T) -> Self {
        let mut res = Self::new();
        for (l, r) in iter {
            res.insert(l, r);
        }
        res
    }
}

mod bincode {
    use bincode::{BorrowDecode, Decode, Encode};

    use super::BiMap;

    impl<L: Ord + Clone + Encode, R: Ord + Clone + Encode> Encode for BiMap<L, R> {
        fn encode<E: bincode::enc::Encoder>(
            &self,
            encoder: &mut E,
        ) -> Result<(), bincode::error::EncodeError> {
            self.len().encode(encoder)?;
            for (l, r) in self {
                l.encode(encoder)?;
                r.encode(encoder)?;
            }
            Ok(())
        }
    }

    impl<Context, L: Ord + Clone + Decode<Context>, R: Ord + Clone + Decode<Context>>
        Decode<Context> for BiMap<L, R>
    {
        fn decode<D: bincode::de::Decoder<Context = Context>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            let len: usize = Decode::decode(decoder)?;
            let mut res = BiMap::new();
            for _ in 0..len {
                let l = Decode::decode(decoder)?;
                let r = Decode::decode(decoder)?;
                res.insert(l, r);
            }
            Ok(res)
        }
    }

    impl<
        'de,
        Context,
        L: Ord + Clone + BorrowDecode<'de, Context>,
        R: Ord + Clone + BorrowDecode<'de, Context>,
    > BorrowDecode<'de, Context> for BiMap<L, R>
    {
        fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
            decoder: &mut D,
        ) -> Result<Self, bincode::error::DecodeError> {
            let len: usize = BorrowDecode::borrow_decode(decoder)?;
            let mut res = BiMap::new();
            for _ in 0..len {
                let l = BorrowDecode::borrow_decode(decoder)?;
                let r = BorrowDecode::borrow_decode(decoder)?;
                res.insert(l, r);
            }
            Ok(res)
        }
    }
}
