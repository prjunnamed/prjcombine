pub mod id;
pub mod part;
pub mod vec;

pub use id::{EntityId, EntityRange};
pub use part::EntityPartVec;
pub use vec::EntityVec;

#[cfg(feature = "map")]
pub mod bundle_map;
#[cfg(feature = "map")]
pub mod map;
#[cfg(feature = "map")]
pub mod set;

#[cfg(feature = "map")]
pub use {
    bundle_map::EntityBundleIndices, bundle_map::EntityBundleItemIndex, bundle_map::EntityBundleMap,
    map::EntityMap, set::EntitySet,
};

#[cfg(feature = "bitvec")]
pub mod bitvec;

#[cfg(feature = "bitvec")]
pub use crate::bitvec::EntityBitVec;

#[cfg(feature = "serde")]
pub use serde as __serde;
