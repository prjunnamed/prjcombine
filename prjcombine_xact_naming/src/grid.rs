use std::{
    collections::{hash_map, HashMap},
    ops::Range,
};

use prjcombine_int::{
    db::{BelId, NodeWireId},
    grid::{ColId, DieId, ExpandedGrid, NodeLoc, RowId},
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::db::{IntPipNaming, NamingDb, NodeNamingId, NodeRawTileId};

#[derive(Clone, Debug)]
pub struct ExpandedGridNaming<'a> {
    pub db: &'a NamingDb,
    pub egrid: &'a ExpandedGrid<'a>,
    pub nodes: HashMap<NodeLoc, GridNodeNaming>,
    pub tie_pin_gnd: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GridNodeNaming {
    pub naming: NodeNamingId,
    pub coords: EntityVec<NodeRawTileId, (Range<usize>, Range<usize>)>,
    pub tie_names: Vec<String>,
    pub bels: EntityPartVec<BelId, Vec<String>>,
}

impl GridNodeNaming {
    pub fn add_bel(&mut self, idx: usize, names: Vec<String>) {
        self.bels.insert(BelId::from_idx(idx), names);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PipCoords {
    Pip((usize, usize)),
    BoxPip((usize, usize), (usize, usize)),
}

impl<'a> ExpandedGridNaming<'a> {
    pub fn new(db: &'a NamingDb, egrid: &'a ExpandedGrid<'a>) -> Self {
        ExpandedGridNaming {
            db,
            egrid,
            tie_pin_gnd: None,
            nodes: HashMap::new(),
        }
    }

    pub fn name_node(
        &mut self,
        nloc: NodeLoc,
        naming: &str,
        coords: impl IntoIterator<Item = (Range<usize>, Range<usize>)>,
    ) -> &mut GridNodeNaming {
        let nnode = GridNodeNaming {
            coords: coords.into_iter().collect(),
            naming: self.db.get_node_naming(naming),
            tie_names: vec![],
            bels: EntityPartVec::new(),
        };
        let hash_map::Entry::Vacant(entry) = self.nodes.entry(nloc) else {
            unreachable!()
        };
        entry.insert(nnode)
    }

    pub fn get_bel_name(&self, col: ColId, row: RowId, key: &str) -> Option<&str> {
        let die = DieId::from_idx(0);
        if let Some((layer, _, bel, _)) = self.egrid.find_bel(die, (col, row), key) {
            let nnode = &self.nodes[&(die, col, row, layer)];
            Some(&nnode.bels[bel][0])
        } else {
            None
        }
    }

    pub fn bel_pip(&self, nloc: NodeLoc, bel: BelId, key: &str) -> PipCoords {
        let nnode = &self.nodes[&nloc];
        let naming = &self.db.node_namings[nnode.naming].bel_pips[&(bel, key.to_string())];
        PipCoords::Pip((
            naming.x + nnode.coords[naming.rt].0.start,
            naming.y + nnode.coords[naming.rt].1.start,
        ))
    }

    pub fn int_pip(&self, nloc: NodeLoc, wire_to: NodeWireId, wire_from: NodeWireId) -> PipCoords {
        let nnode = &self.nodes[&nloc];
        let naming = &self.db.node_namings[nnode.naming].int_pips[&(wire_to, wire_from)];
        match naming {
            IntPipNaming::Pip(pip) => PipCoords::Pip((
                pip.x + nnode.coords[pip.rt].0.start,
                pip.y + nnode.coords[pip.rt].1.start,
            )),
            IntPipNaming::Box(pip1, pip2) => PipCoords::BoxPip(
                (
                    pip1.x + nnode.coords[pip1.rt].0.start,
                    pip1.y + nnode.coords[pip1.rt].1.start,
                ),
                (
                    pip2.x + nnode.coords[pip2.rt].0.start,
                    pip2.y + nnode.coords[pip2.rt].1.start,
                ),
            ),
        }
    }
}