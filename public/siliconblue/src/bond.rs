use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use itertools::Itertools;
use prjcombine_interconnect::{db::IntDb, grid::BelPadCoord};

use crate::defs;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct Bond {
    pub pins: BTreeMap<String, Vec<BelPadCoord>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpandedBond<'a> {
    pub bond: &'a Bond,
    pub ios: BTreeMap<BelPadCoord, String>,
}

impl Bond {
    pub fn expand(&self) -> ExpandedBond<'_> {
        let mut ios = BTreeMap::new();
        for (name, pads) in &self.pins {
            for &pad in pads {
                if pad.slot == defs::bslots::POWER {
                    continue;
                }
                if (pad.slot == defs::bslots::IO_BANK_SPI
                    || defs::bslots::IO_BANK.contains(pad.slot))
                    && pad.pad == defs::bcls::IO_BANK::VCCIO
                {
                    continue;
                }
                if pad.slot == defs::bslots::CONFIG
                    && matches!(
                        pad.pad,
                        defs::bcls::CONFIG::VPP_2V5 | defs::bcls::CONFIG::VPP_FAST
                    )
                {
                    continue;
                }
                ios.insert(pad, name.clone());
            }
        }
        ExpandedBond { bond: self, ios }
    }
}

fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    if let Some(pos) = name.find(|x: char| x.is_ascii_digit()) {
        (pos, &name[..pos], name[pos..].parse().unwrap())
    } else {
        (name.len(), name, 0)
    }
}

impl Bond {
    pub fn dump(&self, o: &mut dyn std::io::Write, db: &IntDb) -> std::io::Result<()> {
        for (pin, pads) in self.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            if pads.is_empty() {
                writeln!(o, "\tpin {pin} = nc;")?;
            } else {
                writeln!(
                    o,
                    "\tpin {pin} = {pads};",
                    pads = pads.iter().map(|x| x.to_string(db)).join(" + ")
                )?;
            }
        }
        Ok(())
    }
}
