use std::collections::BTreeMap;

use prjcombine_interconnect::db::{BelId, IntDb, NodeIriId, NodeWireId, WireId};
use serde::{Deserialize, Serialize};
use unnamed_entity::{EntityId, EntityMap, EntityPartVec, EntityVec, entity_id};

entity_id! {
    pub id NodeNamingId u16, reserve 1;
    pub id TermNamingId u16, reserve 1;
    pub id NodeRawTileId u16, reserve 1;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamingDb {
    pub node_namings: EntityMap<NodeNamingId, String, NodeNaming>,
    pub term_namings: EntityMap<TermNamingId, String, TermNaming>,
}

impl NamingDb {
    #[track_caller]
    pub fn get_node_naming(&self, name: &str) -> NodeNamingId {
        self.node_namings
            .get(name)
            .unwrap_or_else(|| panic!("no node naming {name}"))
            .0
    }
    #[track_caller]
    pub fn get_term_naming(&self, name: &str) -> TermNamingId {
        self.term_namings
            .get(name)
            .unwrap_or_else(|| panic!("no term naming {name}"))
            .0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct NodeNaming {
    pub wires: BTreeMap<NodeWireId, String>,
    pub wire_bufs: BTreeMap<NodeWireId, NodeExtPipNaming>,
    pub ext_pips: BTreeMap<(NodeWireId, NodeWireId), NodeExtPipNaming>,
    pub bels: EntityVec<BelId, BelNaming>,
    pub iris: EntityVec<NodeIriId, IriNaming>,
    pub intf_wires_out: BTreeMap<NodeWireId, IntfWireOutNaming>,
    pub intf_wires_in: BTreeMap<NodeWireId, IntfWireInNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeExtPipNaming {
    pub tile: NodeRawTileId,
    pub wire_to: String,
    pub wire_from: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelNaming {
    pub tile: NodeRawTileId,
    pub pins: BTreeMap<String, BelPinNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct BelPinNaming {
    pub name: String,
    pub name_far: String,
    pub pips: Vec<NodeExtPipNaming>,
    pub int_pips: BTreeMap<NodeWireId, NodeExtPipNaming>,
    pub is_intf_out: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IriNaming {
    pub tile: NodeRawTileId,
    pub kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfWireOutNaming {
    Simple { name: String },
    Buf { name_out: String, name_in: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntfWireInNaming {
    Simple {
        name: String,
    },
    Buf {
        name_out: String,
        name_in: String,
    },
    TestBuf {
        name_out: String,
        name_in: String,
    },
    Delay {
        name_out: String,
        name_in: String,
        name_delay: String,
    },
    Iri {
        name_out: String,
        name_pin_out: String,
        name_pin_in: String,
        name_in: String,
    },
    IriDelay {
        name_out: String,
        name_delay: String,
        name_pre_delay: String,
        name_pin_out: String,
        name_pin_in: String,
        name_in: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct TermNaming {
    pub wires_out: EntityPartVec<WireId, TermWireOutNaming>,
    pub wires_in_near: EntityPartVec<WireId, String>,
    pub wires_in_far: EntityPartVec<WireId, TermWireInFarNaming>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TermWireOutNaming {
    Simple { name: String },
    Buf { name_out: String, name_in: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TermWireInFarNaming {
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
        for (_, name, naming) in &self.node_namings {
            writeln!(o, "\tNODE NAMING {name}")?;
            for (k, v) in &naming.wires {
                writeln!(
                    o,
                    "\t\tWIRE {wt:3}.{wn:20} {v}",
                    wt = k.0.to_idx(),
                    wn = intdb.wires.key(k.1)
                )?;
            }
            for (k, v) in &naming.wire_bufs {
                writeln!(
                    o,
                    "\t\tWIRE BUF {wt:3}.{wn:20}: RT.{vrt} {vt} <- {vf}",
                    wt = k.0.to_idx(),
                    wn = intdb.wires.key(k.1),
                    vrt = v.tile.to_idx(),
                    vt = v.wire_to,
                    vf = v.wire_from,
                )?;
            }
            for (k, v) in &naming.ext_pips {
                writeln!(
                    o,
                    "\t\tEXT PIP {wtt:3}.{wtn:20} <- {wft:3}.{wfn:20}: RT.{vrt} {vt} <- {vf}",
                    wtt = k.0.0.to_idx(),
                    wtn = intdb.wires.key(k.0.1),
                    wft = k.1.0.to_idx(),
                    wfn = intdb.wires.key(k.1.1),
                    vrt = v.tile.to_idx(),
                    vt = v.wire_to,
                    vf = v.wire_from,
                )?;
            }
            for (bid, bn) in &naming.bels {
                writeln!(
                    o,
                    "\t\tBEL {bid} RT.{rt}:",
                    bid = bid.to_idx(),
                    rt = bn.tile.to_idx()
                )?;
                for (k, v) in &bn.pins {
                    write!(o, "\t\t\tPIN {k}: ")?;
                    if v.name == v.name_far {
                        write!(o, "{n}", n = v.name)?;
                    } else {
                        write!(o, "NEAR {nn} FAR {nf}", nn = v.name, nf = v.name_far)?;
                    }
                    if v.is_intf_out {
                        write!(o, " INTF.OUT")?;
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
                            wt = w.0.to_idx(),
                            wn = intdb.wires.key(w.1),
                            rt = pip.tile.to_idx(),
                            pt = pip.wire_to,
                            pf = pip.wire_from
                        )?;
                    }
                }
            }
            for (i, iri) in &naming.iris {
                writeln!(
                    o,
                    "\t\tIRI.{i}: RT.{rt} {kind}",
                    i = i.to_idx(),
                    rt = iri.tile.to_idx(),
                    kind = iri.kind
                )?;
            }
            for (w, wn) in &naming.intf_wires_out {
                write!(
                    o,
                    "\t\tINTF.OUT {wt:3}.{wn:20}: ",
                    wt = w.0.to_idx(),
                    wn = intdb.wires.key(w.1)
                )?;
                match wn {
                    IntfWireOutNaming::Simple { name } => writeln!(o, "SIMPLE {name}")?,
                    IntfWireOutNaming::Buf { name_out, name_in } => {
                        writeln!(o, "BUF {name_out} <- {name_in}")?
                    }
                }
            }
            for (w, wn) in &naming.intf_wires_in {
                write!(
                    o,
                    "\t\tINTF.IN {wt:3}.{wn:20}: ",
                    wt = w.0.to_idx(),
                    wn = intdb.wires.key(w.1)
                )?;
                match wn {
                    IntfWireInNaming::Simple { name } => writeln!(o, "SIMPLE {name}")?,
                    IntfWireInNaming::Buf { name_out, name_in } => {
                        writeln!(o, "BUF {name_out} <- {name_in}")?
                    }
                    IntfWireInNaming::TestBuf { name_out, name_in } => {
                        writeln!(o, "TESTBUF {name_out} <- {name_in}")?
                    }
                    IntfWireInNaming::Delay {
                        name_out,
                        name_delay,
                        name_in,
                    } => writeln!(o, "DELAY {name_out} <- {name_delay} <- {name_in}")?,
                    IntfWireInNaming::Iri {
                        name_out,
                        name_pin_out,
                        name_pin_in,
                        name_in,
                    } => writeln!(
                        o,
                        "IRI {name_out} <- {name_pin_out} <-IRI- {name_pin_in} <- {name_in}"
                    )?,
                    IntfWireInNaming::IriDelay {
                        name_out,
                        name_delay,
                        name_pre_delay,
                        name_pin_out,
                        name_pin_in,
                        name_in,
                    } => writeln!(
                        o,
                        "IRI.DELAY {name_out} <- {name_delay} <- {name_pre_delay} <- {name_pin_out} <-IRI- {name_pin_in} <- {name_in}"
                    )?,
                }
            }
        }
        for (_, name, naming) in &self.term_namings {
            writeln!(o, "\tTERM NAMING {name}")?;
            for (w, wn) in &naming.wires_out {
                write!(o, "\t\tWIRE OUT {w}: ", w = intdb.wires.key(w))?;
                match wn {
                    TermWireOutNaming::Simple { name } => writeln!(o, "{name}")?,
                    TermWireOutNaming::Buf { name_out, name_in } => {
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
                    TermWireInFarNaming::Simple { name } => writeln!(o, "{name}")?,
                    TermWireInFarNaming::Buf { name_out, name_in } => {
                        writeln!(o, "{name_out} <- {name_in}")?
                    }
                    TermWireInFarNaming::BufFar {
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
