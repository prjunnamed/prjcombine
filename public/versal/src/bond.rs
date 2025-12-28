use bincode::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub struct Bond {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond<'_> {
        ExpandedBond { bond: self }
    }
}

impl Bond {
    pub fn dump(&self, _o: &mut dyn std::io::Write) -> std::io::Result<()> {
        // well.
        Ok(())
    }
}
