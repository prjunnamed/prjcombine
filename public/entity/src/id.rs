use core::hash::Hash;
use core::marker::PhantomData;

use std::{
    fmt::Debug,
    num::{NonZeroU8, NonZeroU16, NonZeroU32},
};

#[cfg(feature = "bincode")]
use bincode::{BorrowDecode, Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

macro_rules! make_res_type {
    ($t:ident, $b:ty, $i:ty) => {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        #[cfg_attr(
            feature = "serde",
            derive(Serialize, Deserialize),
            serde(into = "usize"),
            serde(try_from = "usize")
        )]
        pub struct $t($b);

        impl $t {
            pub const fn from_usize_const(val: usize) -> Self {
                let tmp = (val + 1) as $i;
                $t(<$b>::new(tmp).unwrap())
            }

            pub const fn to_usize_const(self) -> usize {
                (self.0.get() - 1) as usize
            }
        }

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
            fn fmt(
                &self,
                f: &mut std::fmt::Formatter<'_>,
            ) -> core::result::Result<(), std::fmt::Error> {
                write!(f, "{}", usize::from(*self))
            }
        }

        impl std::fmt::Display for $t {
            fn fmt(
                &self,
                f: &mut std::fmt::Formatter<'_>,
            ) -> core::result::Result<(), std::fmt::Error> {
                write!(f, "{}", usize::from(*self))
            }
        }
    };
}

make_res_type!(__ReservedU8, core::num::NonZeroU8, u8);
make_res_type!(__ReservedU16, core::num::NonZeroU16, u16);
make_res_type!(__ReservedU32, core::num::NonZeroU32, u32);
make_res_type!(__ReservedUsize, core::num::NonZeroUsize, usize);

#[cfg(feature = "serde")]
#[macro_export]
macro_rules! __def_entity_id {
    ($v:vis, $id:ident, $t:ty) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, $crate::__serde::Serialize, $crate::__serde::Deserialize)]
        #[serde(crate="prjcombine_entity::__serde")]
        $v struct $id($t);
    }
}

#[cfg(not(feature = "serde"))]
#[macro_export]
macro_rules! __def_entity_id {
    ($v:vis, $id:ident, $t:ty) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        $v struct $id($t);
    }
}

#[macro_export]
macro_rules! __impl_entity_id {
    ($v:vis, $id:ident, $t:ty) => {
        $crate::__def_entity_id!($v, $id, $t);

        impl $crate::EntityId for $id {
            fn to_idx(self) -> usize {
                self.0.try_into().unwrap()
            }
            fn from_idx(idx: usize) -> Self {
                Self(idx.try_into().unwrap())
            }
        }

        impl std::fmt::Display for $id {
            fn fmt(
                &self,
                f: &mut std::fmt::Formatter<'_>,
            ) -> core::result::Result<(), std::fmt::Error> {
                use $crate::EntityId;
                write!(f, "{}", self.to_idx())
            }
        }
    };
}

#[macro_export]
macro_rules! __impl_entity_id_const {
    ($v:vis, $id:ident, $t:ty) => {
        impl $id {
            pub const fn from_idx_const(val: usize) -> Self {
                Self(val as $t)
            }

            pub const fn to_idx_const(self) -> usize {
                self.0 as usize
            }
        }
    };
}

#[macro_export]
macro_rules! __impl_entity_id_const_reserved {
    ($v:vis, $id:ident, $t:ty) => {
        impl $id {
            pub const fn from_idx_const(val: usize) -> Self {
                Self(<$t>::from_usize_const(val))
            }

            pub const fn to_idx_const(self) -> usize {
                self.0.to_usize_const()
            }
        }
    };
}

#[macro_export]
macro_rules! __impl_entity_id_delta {
    ($v:vis, $id:ident, $t:ty) => {
        impl core::ops::Add<usize> for $id {
            type Output = $id;
            fn add(self, x: usize) -> Self {
                use $crate::EntityId;
                Self::from_idx(self.to_idx() + x)
            }
        }

        impl core::ops::AddAssign<usize> for $id {
            fn add_assign(&mut self, x: usize) {
                *self = *self + x;
            }
        }

        impl core::ops::Sub<usize> for $id {
            type Output = $id;
            fn sub(self, x: usize) -> Self {
                use $crate::EntityId;
                Self::from_idx(self.to_idx() - x)
            }
        }

        impl core::ops::SubAssign<usize> for $id {
            fn sub_assign(&mut self, x: usize) {
                *self = *self - x;
            }
        }

        impl core::ops::Add<isize> for $id {
            type Output = $id;
            fn add(self, x: isize) -> Self {
                use $crate::EntityId;
                Self::from_idx(self.to_idx().checked_add_signed(x).unwrap())
            }
        }

        impl core::ops::AddAssign<isize> for $id {
            fn add_assign(&mut self, x: isize) {
                *self = *self + x;
            }
        }

        impl core::ops::Sub<isize> for $id {
            type Output = $id;
            fn sub(self, x: isize) -> Self {
                self + (-x)
            }
        }

        impl core::ops::SubAssign<isize> for $id {
            fn sub_assign(&mut self, x: isize) {
                *self = *self - x;
            }
        }

        impl core::ops::Add<i32> for $id {
            type Output = $id;
            fn add(self, x: i32) -> Self {
                self + (x as isize)
            }
        }

        impl core::ops::AddAssign<i32> for $id {
            fn add_assign(&mut self, x: i32) {
                *self = *self + (x as isize);
            }
        }

        impl core::ops::Sub<i32> for $id {
            type Output = $id;
            fn sub(self, x: i32) -> Self {
                self - (x as isize)
            }
        }

        impl core::ops::SubAssign<i32> for $id {
            fn sub_assign(&mut self, x: i32) {
                *self = *self - (x as isize);
            }
        }

        impl core::ops::Sub<$id> for $id {
            type Output = isize;
            fn sub(self, x: Self) -> isize {
                use $crate::EntityId;
                self.to_idx() as isize - x.to_idx() as isize
            }
        }

        impl $id {
            pub fn range(self, other: $id) -> $crate::id::EntityRange<$id> {
                use $crate::EntityId;
                $crate::id::EntityRange::new(self.to_idx(), other.to_idx())
            }
        }
    };
}

#[macro_export]
macro_rules! __reserved_ty {
    (u8) => {
        $crate::id::__ReservedU8
    };
    (u16) => {
        $crate::id::__ReservedU16
    };
    (u32) => {
        $crate::id::__ReservedU32
    };
    (usize) => {
        $crate::id::__ReservedUsize
    };
}

#[macro_export]
macro_rules! entity_id {
    ($v:vis id $id:ident $t:ty; $($rest:tt)*) => {
        $crate::__impl_entity_id!($v, $id, $t);
        $crate::__impl_entity_id_const!($v, $id, $t);
        $crate::entity_id!{ $($rest)* }
    };
    ($v:vis id $id:ident $t:ty, delta; $($rest:tt)*) => {
        $crate::__impl_entity_id!($v, $id, $t);
        $crate::__impl_entity_id_const!($v, $id, $t);
        $crate::__impl_entity_id_delta!($v, $id, $t);
        $crate::entity_id!{ $($rest)* }
    };
    ($v:vis id $id:ident $t:tt, reserve 1; $($rest:tt)*) => {
        $crate::__impl_entity_id!($v, $id, $crate::__reserved_ty!($t));
        $crate::__impl_entity_id_const_reserved!($v, $id, $crate::__reserved_ty!($t));
        $crate::entity_id!{ $($rest)* }
    };
    ($v:vis id $id:ident $t:tt, reserve 1, delta; $($rest:tt)*) => {
        $crate::__impl_entity_id!($v, $id, $crate::__reserved_ty!($t));
        $crate::__impl_entity_id_delta!($v, $id, $t);
        $crate::__impl_entity_id_const_reserved!($v, $id, $crate::__reserved_ty!($t));
        $crate::entity_id!{ $($rest)* }
    };
    () => {};
}

pub trait EntityId:
    Debug + Copy + Clone + Send + Sync + Eq + PartialEq + Ord + PartialOrd + Hash
{
    fn from_idx(idx: usize) -> Self;
    fn to_idx(self) -> usize;
}

// new-style id

pub trait EntityTag {
    const PREFIX: &'static str = "ID";
}
pub trait EntityTagArith: EntityTag {}

// HORRIBLE HACK ALERT: we want `EntityIdU*` to be `StructuralEq`, so that we can match on constants
// of that type.  However, the only way to get it is deriving `Eq`, which introduces a where-bound
// on `Tag: Eq`, which we do not want.  Therefore, `EntityIdU*` are actually type aliases which
// wrap the parameter in `PhantomData` (which is always `Eq` and `StructuralEq`) and pass it into
// the inner type, which has derived `Eq`.

macro_rules! make_id {
    ($ty:ident, $nzty:ident, $intty:ident, $max:literal) => {
        #[doc(hidden)]
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ty<Tag>($nzty, PhantomData<fn(Tag) -> Tag>);

        impl<Tag> $ty<Tag> {
            pub const fn from_idx_const(idx: usize) -> Self {
                assert!(idx < $max);
                let idx = (idx + 1) as $intty;
                let idx = $nzty::new(idx).unwrap();
                $ty(idx, PhantomData)
            }

            pub const fn to_idx_const(self) -> usize {
                (self.0.get() - 1) as usize
            }
        }

        impl<Tag: EntityTag> EntityId for $ty<PhantomData<Tag>> {
            fn from_idx(idx: usize) -> Self {
                let idx = $intty::try_from(idx.checked_add(1).unwrap()).unwrap();
                let idx = $nzty::new(idx).unwrap();
                $ty(idx, PhantomData)
            }

            fn to_idx(self) -> usize {
                let idx = self.0.get() - 1;
                idx.try_into().unwrap()
            }
        }

        impl<Tag: EntityTag> Debug for $ty<PhantomData<Tag>> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}{}", Tag::PREFIX, self.to_idx())
            }
        }
        impl<Tag: EntityTag> std::fmt::Display for $ty<PhantomData<Tag>> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                if f.alternate() {
                    write!(f, "{}", self.to_idx())
                } else {
                    write!(f, "{}{}", Tag::PREFIX, self.to_idx())
                }
            }
        }
        #[cfg(feature = "serde")]
        impl<Tag> Serialize for $ty<PhantomData<Tag>> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let idx = self.0.get() - 1;
                idx.serialize(serializer)
            }
        }
        #[cfg(feature = "serde")]
        impl<'de, Tag> Deserialize<'de> for $ty<PhantomData<Tag>> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let idx = $intty::deserialize(deserializer)?;
                let idx = idx.checked_add(1).unwrap();
                let idx = $nzty::new(idx).unwrap();
                Ok($ty(idx, PhantomData))
            }
        }
        #[cfg(feature = "bincode")]
        impl<Tag> Encode for $ty<PhantomData<Tag>> {
            fn encode<E: bincode::enc::Encoder>(
                &self,
                encoder: &mut E,
            ) -> Result<(), bincode::error::EncodeError> {
                let idx = self.0.get() - 1;
                idx.encode(encoder)
            }
        }
        #[cfg(feature = "bincode")]
        impl<Tag, Context> Decode<Context> for $ty<PhantomData<Tag>> {
            fn decode<D: bincode::de::Decoder<Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                let idx = $intty::decode(decoder)?;
                let idx = idx.checked_add(1).unwrap();
                let idx = $nzty::new(idx).unwrap();
                Ok($ty(idx, PhantomData))
            }
        }
        #[cfg(feature = "bincode")]
        impl<'de, Tag, Context> BorrowDecode<'de, Context> for $ty<PhantomData<Tag>> {
            fn borrow_decode<D: bincode::de::BorrowDecoder<'de, Context = Context>>(
                decoder: &mut D,
            ) -> Result<Self, bincode::error::DecodeError> {
                let idx = $intty::borrow_decode(decoder)?;
                let idx = idx.checked_add(1).unwrap();
                let idx = $nzty::new(idx).unwrap();
                Ok($ty(idx, PhantomData))
            }
        }

        impl<Tag: EntityTagArith> core::ops::Add<usize> for $ty<PhantomData<Tag>> {
            type Output = $ty<PhantomData<Tag>>;
            fn add(self, x: usize) -> Self {
                use $crate::EntityId;
                Self::from_idx(self.to_idx() + x)
            }
        }

        impl<Tag: EntityTagArith> core::ops::AddAssign<usize> for $ty<PhantomData<Tag>> {
            fn add_assign(&mut self, x: usize) {
                *self = *self + x;
            }
        }

        impl<Tag: EntityTagArith> core::ops::Sub<usize> for $ty<PhantomData<Tag>> {
            type Output = $ty<PhantomData<Tag>>;
            fn sub(self, x: usize) -> Self {
                use $crate::EntityId;
                Self::from_idx(self.to_idx() - x)
            }
        }

        impl<Tag: EntityTagArith> core::ops::SubAssign<usize> for $ty<PhantomData<Tag>> {
            fn sub_assign(&mut self, x: usize) {
                *self = *self - x;
            }
        }

        impl<Tag: EntityTagArith> core::ops::Add<isize> for $ty<PhantomData<Tag>> {
            type Output = $ty<PhantomData<Tag>>;
            fn add(self, x: isize) -> Self {
                use $crate::EntityId;
                Self::from_idx(self.to_idx().checked_add_signed(x).unwrap())
            }
        }

        impl<Tag: EntityTagArith> core::ops::AddAssign<isize> for $ty<PhantomData<Tag>> {
            fn add_assign(&mut self, x: isize) {
                *self = *self + x;
            }
        }

        impl<Tag: EntityTagArith> core::ops::Sub<isize> for $ty<PhantomData<Tag>> {
            type Output = $ty<PhantomData<Tag>>;
            fn sub(self, x: isize) -> Self {
                self + (-x)
            }
        }

        impl<Tag: EntityTagArith> core::ops::SubAssign<isize> for $ty<PhantomData<Tag>> {
            fn sub_assign(&mut self, x: isize) {
                *self = *self - x;
            }
        }

        impl<Tag: EntityTagArith> core::ops::Add<i32> for $ty<PhantomData<Tag>> {
            type Output = $ty<PhantomData<Tag>>;
            fn add(self, x: i32) -> Self {
                self + (x as isize)
            }
        }

        impl<Tag: EntityTagArith> core::ops::AddAssign<i32> for $ty<PhantomData<Tag>> {
            fn add_assign(&mut self, x: i32) {
                *self = *self + (x as isize);
            }
        }

        impl<Tag: EntityTagArith> core::ops::Sub<i32> for $ty<PhantomData<Tag>> {
            type Output = $ty<PhantomData<Tag>>;
            fn sub(self, x: i32) -> Self {
                self - (x as isize)
            }
        }

        impl<Tag: EntityTagArith> core::ops::SubAssign<i32> for $ty<PhantomData<Tag>> {
            fn sub_assign(&mut self, x: i32) {
                *self = *self - (x as isize);
            }
        }

        impl<Tag: EntityTagArith> core::ops::Sub<$ty<PhantomData<Tag>>> for $ty<PhantomData<Tag>> {
            type Output = isize;
            fn sub(self, x: Self) -> isize {
                use $crate::EntityId;
                self.to_idx() as isize - x.to_idx() as isize
            }
        }

        impl<Tag: EntityTagArith> $ty<PhantomData<Tag>> {
            pub fn range(self, other: Self) -> $crate::id::EntityRange<Self> {
                use $crate::EntityId;
                $crate::id::EntityRange::new(self.to_idx(), other.to_idx())
            }
        }
    };
}

make_id!(EntityIdU8Inner, NonZeroU8, u8, 0xff);
make_id!(EntityIdU16Inner, NonZeroU16, u16, 0xffff);
make_id!(EntityIdU32Inner, NonZeroU32, u32, 0xffffffff);

pub type EntityIdU8<T> = EntityIdU8Inner<PhantomData<T>>;
pub type EntityIdU16<T> = EntityIdU16Inner<PhantomData<T>>;
pub type EntityIdU32<T> = EntityIdU32Inner<PhantomData<T>>;

// range

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct EntityRange<I: EntityId> {
    start: usize,
    end: usize,
    ids: PhantomData<I>,
}

impl<I: EntityId> EntityRange<I> {
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end);
        Self {
            start,
            end,
            ids: PhantomData,
        }
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.end == self.start
    }

    pub fn map<T>(
        self,
        f: impl FnMut(I) -> T,
    ) -> impl DoubleEndedIterator<Item = T> + ExactSizeIterator<Item = T> {
        self.into_iter().map(f)
    }

    pub fn rev(self) -> impl DoubleEndedIterator<Item = I> + ExactSizeIterator<Item = I> {
        self.into_iter().rev()
    }

    pub fn filter(self, f: impl FnMut(&I) -> bool) -> impl DoubleEndedIterator<Item = I> {
        self.into_iter().filter(f)
    }

    pub fn start(self) -> I {
        I::from_idx(self.start)
    }

    pub fn end(self) -> I {
        I::from_idx(self.end)
    }

    pub fn first(self) -> Option<I> {
        if self.start == self.end {
            None
        } else {
            Some(I::from_idx(self.start))
        }
    }

    pub fn last(self) -> Option<I> {
        if self.start == self.end {
            None
        } else {
            Some(I::from_idx(self.end - 1))
        }
    }

    pub fn index(self, idx: usize) -> I {
        assert!(idx < self.len());
        I::from_idx(self.start + idx)
    }

    pub fn index_of(&self, id: I) -> Option<usize> {
        if id.to_idx() < self.start || id.to_idx() >= self.end {
            None
        } else {
            Some(id.to_idx() - self.start)
        }
    }
}

impl<I: EntityId> IntoIterator for EntityRange<I> {
    type Item = I;

    type IntoIter = EntityRangeIter<I>;

    fn into_iter(self) -> Self::IntoIter {
        EntityRangeIter {
            start: self.start,
            end: self.end,
            ids: PhantomData,
        }
    }
}

// iterator

#[derive(Clone, Debug)]
pub struct EntityRangeIter<I: EntityId> {
    start: usize,
    end: usize,
    ids: PhantomData<I>,
}

impl<I: EntityId> Iterator for EntityRangeIter<I> {
    type Item = I;
    fn next(&mut self) -> Option<I> {
        if self.start == self.end {
            None
        } else {
            let res = I::from_idx(self.start);
            self.start += 1;
            Some(res)
        }
    }
}

impl<I: EntityId> DoubleEndedIterator for EntityRangeIter<I> {
    fn next_back(&mut self) -> Option<I> {
        if self.start == self.end {
            None
        } else {
            self.end -= 1;
            Some(I::from_idx(self.end))
        }
    }
}

impl<I: EntityId> ExactSizeIterator for EntityRangeIter<I> {
    fn len(&self) -> usize {
        self.end - self.start
    }
}

// static range

#[derive(Copy, Clone, Debug)]
pub struct EntityStaticRange<I: EntityId, const N: usize> {
    data: [I; N],
}

macro_rules! impl_static_range {
    ($ty:ident) => {
        impl<T: EntityTag, const N: usize> EntityStaticRange<$ty<T>, N> {
            pub const fn new_const(base: usize) -> Self {
                let mut data = [$ty::from_idx_const(0); N];
                let mut i = 0;
                while i < N {
                    data[i] = $ty::from_idx_const(base + i);
                    i += 1;
                }
                Self { data }
            }
        }
    };
}

impl_static_range!(EntityIdU8);
impl_static_range!(EntityIdU16);
impl_static_range!(EntityIdU32);

impl<I: EntityId, const N: usize> EntityStaticRange<I, N> {
    pub const fn index_const(&self, idx: usize) -> I {
        self.data[idx]
    }

    pub fn index_of(&self, id: I) -> Option<usize> {
        let base = *self.data.first()?;
        if id.to_idx() < base.to_idx() {
            None
        } else {
            let res = id.to_idx() - base.to_idx();
            if res >= N { None } else { Some(res) }
        }
    }

    pub fn contains(&self, id: I) -> bool {
        self.index_of(id).is_some()
    }
}

impl<I: EntityId, const N: usize> core::ops::Deref for EntityStaticRange<I, N> {
    type Target = [I; N];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<I: EntityId, const N: usize> IntoIterator for EntityStaticRange<I, N> {
    type Item = I;

    type IntoIter = core::array::IntoIter<I, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
