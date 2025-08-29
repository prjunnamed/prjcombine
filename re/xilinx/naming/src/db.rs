use std::collections::BTreeMap;

use bincode::{Decode, Encode};
use prjcombine_interconnect::db::{BelSlotId, IntDb, TileWireCoord, WireSlotId};
use unnamed_entity::{
    EntityId, EntityMap, EntityPartVec,
    id::{EntityIdU16, EntityTag},
};

pub struct RawTileTag;
impl EntityTag for RawTileTag {
    const PREFIX: &'static str = "RT";
}
impl EntityTag for TileClassNaming {
    const PREFIX: &'static str = "TNCLS";
}
impl EntityTag for ConnectorClassNaming {
    const PREFIX: &'static str = "CNCLS";
}
pub type TileClassNamingId = EntityIdU16<TileClassNaming>;
pub type ConnectorClassNamingId = EntityIdU16<ConnectorClassNaming>;
pub type RawTileId = EntityIdU16<RawTileTag>;

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct NamingDb {
    pub tile_class_namings: EntityMap<TileClassNamingId, String, TileClassNaming>,
    pub conn_class_namings: EntityMap<ConnectorClassNamingId, String, ConnectorClassNaming>,
}

impl NamingDb {
    #[track_caller]
    pub fn get_tile_class_naming(&self, name: &str) -> TileClassNamingId {
        self.tile_class_namings
            .get(name)
            .unwrap_or_else(|| panic!("no tile class naming {name}"))
            .0
    }
    #[track_caller]
    pub fn get_conn_class_naming(&self, name: &str) -> ConnectorClassNamingId {
        self.conn_class_namings
            .get(name)
            .unwrap_or_else(|| panic!("no conn class naming {name}"))
            .0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct TileClassNaming {
    pub wires: BTreeMap<TileWireCoord, String>,
    pub wire_bufs: BTreeMap<TileWireCoord, PipNaming>,
    pub ext_pips: BTreeMap<(TileWireCoord, TileWireCoord), PipNaming>,
    pub delay_wires: BTreeMap<TileWireCoord, String>,
    pub bels: EntityPartVec<BelSlotId, BelNaming>,
    pub intf_wires_in: BTreeMap<TileWireCoord, IntfWireInNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct PipNaming {
    pub tile: RawTileId,
    pub wire_to: String,
    pub wire_from: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
#[non_exhaustive]
pub enum BelNaming {
    Bel(ProperBelNaming),
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub struct ProperBelNaming {
    pub tile: RawTileId,
    pub pins: BTreeMap<String, BelPinNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct BelPinNaming {
    pub name: String,
    pub name_far: String,
    pub pips: Vec<PipNaming>,
    pub int_pips: BTreeMap<TileWireCoord, PipNaming>,
    pub is_intf: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum IntfWireInNaming {
    Simple { name: String },
    Buf { name_out: String, name_in: String },
    TestBuf { name_out: String, name_in: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Encode, Decode)]
pub struct ConnectorClassNaming {
    pub wires_out: EntityPartVec<WireSlotId, ConnectorWireOutNaming>,
    pub wires_in_near: EntityPartVec<WireSlotId, String>,
    pub wires_in_far: EntityPartVec<WireSlotId, ConnectorWireInFarNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum ConnectorWireOutNaming {
    Simple { name: String },
    Buf { name_out: String, name_in: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode)]
pub enum ConnectorWireInFarNaming {
    Simple {
        name: String,
    },
    Buf {
        name_out: String,
        name_in: String,
    },
    BufFar {
        name: String,
        name_far_out: String,
        name_far_in: String,
    },
}

impl NamingDb {
    pub fn print(&self, intdb: &IntDb, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (_, name, naming) in &self.tile_class_namings {
            writeln!(o, "\tTILE NAMING {name}")?;
            for (k, v) in &naming.wires {
                writeln!(
                    o,
                    "\t\tWIRE {wt:3}.{wn:20} {v}",
                    wt = k.cell.to_idx(),
                    wn = intdb.wires.key(k.wire)
                )?;
            }
            for (k, v) in &naming.wire_bufs {
                writeln!(
                    o,
                    "\t\tWIRE BUF {wt:3}.{wn:20}: RT.{vrt} {vt} <- {vf}",
                    wt = k.cell.to_idx(),
                    wn = intdb.wires.key(k.wire),
                    vrt = v.tile.to_idx(),
                    vt = v.wire_to,
                    vf = v.wire_from,
                )?;
            }
            for (k, v) in &naming.ext_pips {
                writeln!(
                    o,
                    "\t\tEXT PIP {wtt:3}.{wtn:20} <- {wft:3}.{wfn:20}: RT.{vrt} {vt} <- {vf}",
                    wtt = k.0.cell.to_idx(),
                    wtn = intdb.wires.key(k.0.wire),
                    wft = k.1.cell.to_idx(),
                    wfn = intdb.wires.key(k.1.wire),
                    vrt = v.tile.to_idx(),
                    vt = v.wire_to,
                    vf = v.wire_from,
                )?;
            }
            for (k, v) in &naming.delay_wires {
                writeln!(
                    o,
                    "\t\tDELAY WIRE {wt:3}.{wn:20} {v}",
                    wt = k.cell.to_idx(),
                    wn = intdb.wires.key(k.wire)
                )?;
            }
            for (slot, bn) in &naming.bels {
                match bn {
                    BelNaming::Bel(bn) => {
                        writeln!(
                            o,
                            "\t\tBEL {slot} RT.{rt}:",
                            slot = intdb.bel_slots.key(slot),
                            rt = bn.tile,
                        )?;
                        for (k, v) in &bn.pins {
                            write!(o, "\t\t\tPIN {k}: ")?;
                            if v.name == v.name_far {
                                write!(o, "{n}", n = v.name)?;
                            } else {
                                write!(o, "NEAR {nn} FAR {nf}", nn = v.name, nf = v.name_far)?;
                            }
                            if v.is_intf {
                                write!(o, " INTF")?;
                            }
                            writeln!(o)?;
                            for pip in &v.pips {
                                writeln!(
                                    o,
                                    "\t\t\t\tPIP RT.{rt} {wt} <- {wf}",
                                    rt = pip.tile.to_idx(),
                                    wt = pip.wire_to,
                                    wf = pip.wire_from
                                )?;
                            }
                            for (w, pip) in &v.int_pips {
                                writeln!(
                                    o,
                                    "\t\t\t\tINT PIP {wt:3}.{wn:20}: RT.{rt} {pt} <- {pf}",
                                    wt = w.cell.to_idx(),
                                    wn = intdb.wires.key(w.wire),
                                    rt = pip.tile.to_idx(),
                                    pt = pip.wire_to,
                                    pf = pip.wire_from
                                )?;
                            }
                        }
                    }
                }
            }
            for (w, wn) in &naming.intf_wires_in {
                write!(
                    o,
                    "\t\tINTF.IN {wt:3}.{wn:20}: ",
                    wt = w.cell.to_idx(),
                    wn = intdb.wires.key(w.wire)
                )?;
                match wn {
                    IntfWireInNaming::Simple { name } => writeln!(o, "SIMPLE {name}")?,
                    IntfWireInNaming::Buf { name_out, name_in } => {
                        writeln!(o, "BUF {name_out} <- {name_in}")?
                    }
                    IntfWireInNaming::TestBuf { name_out, name_in } => {
                        writeln!(o, "TESTBUF {name_out} <- {name_in}")?
                    }
                }
            }
        }
        for (_, name, naming) in &self.conn_class_namings {
            writeln!(o, "\tCONN NAMING {name}")?;
            for (w, wn) in &naming.wires_out {
                write!(o, "\t\tWIRE OUT {w}: ", w = intdb.wires.key(w))?;
                match wn {
                    ConnectorWireOutNaming::Simple { name } => writeln!(o, "{name}")?,
                    ConnectorWireOutNaming::Buf { name_out, name_in } => {
                        writeln!(o, "{name_out} <- {name_in}")?
                    }
                }
            }
            for (w, wn) in &naming.wires_in_near {
                writeln!(o, "\t\tWIRE IN NEAR {w}: {wn}", w = intdb.wires.key(w))?;
            }
            for (w, wn) in &naming.wires_in_far {
                write!(o, "\t\tWIRE IN FAR {w}: ", w = intdb.wires.key(w))?;
                match wn {
                    ConnectorWireInFarNaming::Simple { name } => writeln!(o, "{name}")?,
                    ConnectorWireInFarNaming::Buf { name_out, name_in } => {
                        writeln!(o, "{name_out} <- {name_in}")?
                    }
                    ConnectorWireInFarNaming::BufFar {
                        name,
                        name_far_out,
                        name_far_in,
                    } => writeln!(o, "{name} <- {name_far_out} <- {name_far_in}")?,
                }
            }
        }
        Ok(())
    }
}
