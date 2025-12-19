pub mod id;
pub mod part;
pub mod vec;

pub use id::{EntityId, EntityIds};
pub use part::EntityPartVec;
pub use vec::EntityVec;

#[cfg(feature = "map")]
pub mod map;
#[cfg(feature = "map")]
pub mod set;

#[cfg(feature = "map")]
pub use {map::EntityMap, set::EntitySet};

#[cfg(feature = "bitvec")]
pub mod bitvec;

#[cfg(feature = "bitvec")]
pub use crate::bitvec::EntityBitVec;

#[cfg(feature = "serde")]
pub use serde as __serde;
