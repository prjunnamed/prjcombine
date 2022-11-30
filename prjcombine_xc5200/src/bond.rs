use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::grid::IoCoord;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BondPin {
    Io(IoCoord),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    pub pins: BTreeMap<String, BondPin>,
}
