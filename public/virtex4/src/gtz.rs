use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use prjcombine_entity::{
    EntityMap,
    id::{EntityIdU16, EntityTag, EntityTagArith},
};
use prjcombine_interconnect::{db::PinDir, dir::DirV};

impl EntityTag for GtzBel {
    const PREFIX: &'static str = "GTZ";
}
pub type GtzBelId = EntityIdU16<GtzBel>;

pub struct GtzIntColTag;
pub struct GtzIntRowTag;
impl EntityTag for GtzIntColTag {
    const PREFIX: &'static str = "GTZC";
}
impl EntityTag for GtzIntRowTag {
    const PREFIX: &'static str = "GTZR";
}
impl EntityTagArith for GtzIntColTag {}
impl EntityTagArith for GtzIntRowTag {}

pub type GtzIntColId = EntityIdU16<GtzIntColTag>;
pub type GtzIntRowId = EntityIdU16<GtzIntRowTag>;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct GtzBel {
    pub side: DirV,
    pub pins: BTreeMap<String, GtzIntPin>,
    pub clk_pins: BTreeMap<String, GtzClkPin>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct GtzIntPin {
    pub dir: PinDir,
    pub col: GtzIntColId,
    pub row: GtzIntRowId,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Encode, Decode)]
pub struct GtzClkPin {
    pub dir: PinDir,
    pub idx: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, Default)]
pub struct GtzDb {
    pub gtz: EntityMap<GtzBelId, String, GtzBel>,
}

impl GtzDb {
    pub fn dump(&self, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (_, name, gtz) in &self.gtz {
            writeln!(o, "gtz {name}: {side} {{", side = gtz.side)?;
            for (pname, pin) in &gtz.pins {
                writeln!(
                    o,
                    "\t{dir} {pname} = INT {col} {row};",
                    dir = match pin.dir {
                        PinDir::Input => "input",
                        PinDir::Output => "output",
                        PinDir::Inout => unreachable!(),
                    },
                    col = pin.col,
                    row = pin.row
                )?;
            }
            for (pname, pin) in &gtz.clk_pins {
                writeln!(
                    o,
                    "\t{dir} {pname} = GCLK{idx};",
                    dir = match pin.dir {
                        PinDir::Input => "input",
                        PinDir::Output => "output",
                        PinDir::Inout => unreachable!(),
                    },
                    idx = pin.idx
                )?;
            }
            writeln!(o, "}}")?;
            writeln!(o)?;
        }
        Ok(())
    }
}
