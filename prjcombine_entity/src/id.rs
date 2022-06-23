use core::hash::Hash;
use core::marker::PhantomData;

use std::fmt::Debug;

use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Error};

macro_rules! make_res_type {
    ($t:ident, $b:ty, $i:ty) => {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $t ($b);

        impl From<$t> for usize {
            fn from(x: $t) -> usize {
                (x.0.get() - 1).try_into().unwrap()
            }
        }

        impl TryFrom<usize> for $t {
            type Error = std::num::TryFromIntError;
            fn try_from(x: usize) -> Result<Self, Self::Error> {
                let tmp: $i = (x + 1).try_into()?;
                Ok($t(tmp.try_into()?))
            }
        }

        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::result::Result<(), std::fmt::Error> {
                write!(f, "{}", usize::from(*self))
            }
        }

        impl<'de> Deserialize<'de> for $t
        {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                usize::deserialize(deserializer).and_then(|x| x.try_into().map_err (|_|
                    D::Error::custom("entity id too large")
                ))
            }
        }

        impl Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: Serializer {
                usize::from(*self).serialize(serializer)
            }
        }
    }
}

make_res_type!(__ReservedU16, core::num::NonZeroU16, u16);
make_res_type!(__ReservedU32, core::num::NonZeroU32, u32);
make_res_type!(__ReservedUsize, core::num::NonZeroUsize, usize);

#[macro_export]
macro_rules! __impl_entity_id {
    ($v:vis, $id:ident, $t:ty) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, serde::Serialize, serde::Deserialize)]
        $v struct $id($t);

        impl<'de> $crate::EntityId for $id {
            fn to_idx(self) -> usize {
                self.0.into()
            }
            fn from_idx(idx: usize) -> Self {
                Self(idx.try_into().unwrap())
            }
        }
    }
}

#[macro_export]
macro_rules! entity_id {
    ($v:vis id $id:ident $t:ty; $($rest:tt)*) => {
        $crate::__impl_entity_id!($v, $id, $t);
        $crate::entity_id!{ $($rest)* }
    };
    ($v:vis id $id:ident u16, reserve 1; $($rest:tt)*) => {
        $crate::__impl_entity_id!($v, $id, $crate::id::__ReservedU16);
        $crate::entity_id!{ $($rest)* }
    };
    ($v:vis id $id:ident u32, reserve 1; $($rest:tt)*) => {
        $crate::__impl_entity_id!($v, $id, $crate::id::__ReservedU32);
        $crate::entity_id!{ $($rest)* }
    };
    ($v:vis id $id:ident usize, reserve 1; $($rest:tt)*) => {
        $crate::__impl_entity_id!($v, $id, $crate::id::__ReservedUsize);
        $crate::entity_id!{ $($rest)* }
    };
    () => {};
}

pub trait EntityId: Debug + Copy + Clone + Send + Sync + Eq + PartialEq + Ord + PartialOrd + Hash + Serialize + for<'de> Deserialize<'de> {
    fn from_idx(idx: usize) -> Self;
    fn to_idx(self) -> usize;
}

// iterator

#[derive(Clone, Debug)]
pub struct EntityIds<I> {
    cur: usize,
    end: usize,
    ids: PhantomData<I>,
}

impl<I: EntityId> EntityIds<I> {
    pub fn new(num: usize) -> Self {
        Self {
            cur: 0,
            end: num,
            ids: PhantomData,
        }
    }
}

impl<I: EntityId> Iterator for EntityIds<I> {
    type Item = I;
    fn next(&mut self) -> Option<I> {
        if self.cur == self.end {
            None
        } else {
            let res = I::from_idx(self.cur);
            self.cur += 1;
            Some(res)
        }
    }
}

impl<I: EntityId> DoubleEndedIterator for EntityIds<I> {
    fn next_back(&mut self) -> Option<I> {
        if self.cur == self.end {
            None
        } else {
            self.end -= 1;
            Some(I::from_idx(self.end))
        }
    }
}

impl<I: EntityId> ExactSizeIterator for EntityIds<I> {
    fn len(&self) -> usize {
        self.end - self.cur
    }
}
