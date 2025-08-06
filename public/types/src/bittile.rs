use std::fmt::Debug;
use std::hash::Hash;

pub trait BitTile: Copy + Debug + Hash + Eq + Sync + Send {
    type BitPos: Copy + Debug + Hash + Eq + Sync + Send;

    fn xlat_pos_rev(&self, bit: Self::BitPos) -> Option<(usize, usize)>;
    fn xlat_pos_fwd(&self, bit: (usize, usize)) -> Self::BitPos;
}
