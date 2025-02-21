use std::collections::BTreeMap;

use prjcombine_interconnect::db::{BelId, IntDb, NodeWireId};
use serde::{Deserialize, Serialize};
use unnamed_entity::{entity_id, EntityMap};

entity_id! {
    pub id NodeNamingId u16, reserve 1;
    pub id NodeRawTileId u16, reserve 1;
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct NamingDb {
    pub node_namings: EntityMap<NodeNamingId, String, NodeNaming>,
    pub tile_widths: BTreeMap<String, usize>,
    pub tile_heights: BTreeMap<String, usize>,
}

impl NamingDb {
    #[track_caller]
    pub fn get_node_naming(&self, name: &str) -> NodeNamingId {
        self.node_namings
            .get(name)
            .unwrap_or_else(|| panic!("no node naming {name}"))
            .0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct NodeNaming {
    pub int_pips: BTreeMap<(NodeWireId, NodeWireId), IntPipNaming>,
    pub bel_pips: BTreeMap<(BelId, String), PipNaming>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntPipNaming {
    Pip(PipNaming),
    Box(PipNaming, PipNaming),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipNaming {
    pub rt: NodeRawTileId,
    pub x: usize,
    pub y: usize,
}

impl NamingDb {
    pub fn print(&self, intdb: &IntDb, o: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (_, name, naming) in &self.node_namings {
            writeln!(o, "\tNODE NAMING {name}")?;
            for (&k, &v) in &naming.int_pips {
                let (wt, wf) = k;
                write!(
                    o,
                    "\t\tPIP {wtt}.{wtn:20} <- {wft}.{wfn:20}: ",
                    wtt = wt.0,
                    wtn = intdb.wires.key(wt.1),
                    wft = wf.0,
                    wfn = intdb.wires.key(wf.1),
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
            for (&(bel, ref key), &v) in &naming.bel_pips {
                writeln!(
                    o,
                    "\t\tPIP BEL {bel:3} {key:20}: {rt}.{x}.{y}",
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
