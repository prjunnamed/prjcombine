use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bond {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond {
        ExpandedBond { bond: self }
    }
}
