use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use prjcombine_interconnect::db::{BelSlotId, IntDb, TileWireCoord};
use prjcombine_entity::{
    EntityMap,
    id::{EntityIdU16, EntityTag},
};

pub struct TileRawCellTag;
impl EntityTag for TileRawCellTag {
    const PREFIX: &'static str = "RT";
}
impl EntityTag for TileNaming {
    const PREFIX: &'static str = "TNCLS";
}
pub type TileNamingId = EntityIdU16<TileNaming>;
pub type TileRawCellId = EntityIdU16<TileRawCellTag>;

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct NamingDb {
    pub tile_namings: EntityMap<TileNamingId, String, TileNaming>,
    pub tile_widths: BTreeMap<String, usize>,
    pub tile_heights: BTreeMap<String, usize>,
}

impl NamingDb {
    #[track_caller]
    pub fn get_tile_naming(&self, name: &str) -> TileNamingId {
        self.tile_namings
            .get(name)
            .unwrap_or_else(|| panic!("no tile naming {name}"))
            .0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct TileNaming {
    pub int_pips: BTreeMap<(TileWireCoord, TileWireCoord), IntPipNaming>,
    pub bel_pips: BTreeMap<(BelSlotId, String), PipNaming>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Encode, Decode)]
pub enum IntPipNaming {
    Pip(PipNaming),
    Box(PipNaming, PipNaming),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Encode, Decode)]
pub struct PipNaming {
    pub rt: TileRawCellId,
    pub x: usize,
    pub y: usize,
}

impl NamingDb {
    pub fn print(&self, intdb: &IntDb, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (_, name, naming) in &self.tile_namings {
            writeln!(o, "\tTILE NAMING {name}")?;
            for (&k, &v) in &naming.int_pips {
                let (wt, wf) = k;
                write!(
                    o,
                    "\t\tPIP {wtt}.{wtn:20} <- {wft}.{wfn:20}: ",
                    wtt = wt.cell,
                    wtn = intdb.wires.key(wt.wire),
                    wft = wf.cell,
                    wfn = intdb.wires.key(wf.wire),
                )?;
                match v {
                    IntPipNaming::Pip(p) => {
                        writeln!(o, "{rt}.{x}.{y}", rt = p.rt, x = p.x, y = p.y)?
                    }
                    IntPipNaming::Box(p0, p1) => writeln!(
                        o,
                        "{rt0}.{x0}.{y0} - {rt1}.{x1}.{y1}",
                        rt0 = p0.rt,
                        x0 = p0.x,
                        y0 = p0.y,
                        rt1 = p1.rt,
                        x1 = p1.x,
                        y1 = p1.y
                    )?,
                }
            }
            for (&(slot, ref key), &v) in &naming.bel_pips {
                writeln!(
                    o,
                    "\t\tPIP BEL {slot:20}  {key:20}  : {rt}.{x}.{y}",
                    slot = intdb.bel_slots.key(slot),
                    rt = v.rt,
                    x = v.x,
                    y = v.y
                )?;
            }
        }
        for (k, v) in &self.tile_widths {
            writeln!(o, "\tWIDTH {k:4}: {v:2}",)?;
        }
        for (k, v) in &self.tile_heights {
            writeln!(o, "\tHEIGHT {k:4}: {v:2}",)?;
        }
        Ok(())
    }
}
