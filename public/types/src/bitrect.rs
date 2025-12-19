use std::fmt::Debug;
use std::hash::Hash;

use crate::bsdata::{RectBitId, RectFrameId};

pub trait BitRect: Copy + Debug + Hash + Eq + Sync + Send {
    type BitPos: Copy + Debug + Hash + Eq + Sync + Send;

    fn xlat_pos_rev(&self, bit: Self::BitPos) -> Option<(RectFrameId, RectBitId)>;
    fn xlat_pos_fwd(&self, bit: (RectFrameId, RectBitId)) -> Self::BitPos;
}
