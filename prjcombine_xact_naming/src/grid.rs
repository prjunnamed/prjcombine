use std::{collections::HashMap, ops::Range};

use prjcombine_int::{
    db::BelId,
    grid::{ColId, DieId, ExpandedGrid, NodeLoc, RowId},
};
use unnamed_entity::{EntityId, EntityPartVec, EntityVec};

use crate::db::{NamingDb, NodeRawTileId};

#[derive(Clone, Debug)]
pub struct ExpandedGridNaming<'a> {
    pub db: &'a NamingDb,
    pub egrid: &'a ExpandedGrid<'a>,
    pub nodes: HashMap<NodeLoc, GridNodeNaming>,
    pub tie_pin_gnd: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GridNodeNaming {
    pub coords: EntityVec<NodeRawTileId, (Range<usize>, Range<usize>)>,
    pub tie_names: Vec<String>,
    pub bels: EntityPartVec<BelId, Vec<String>>,
}

impl GridNodeNaming {
    pub fn add_bel(&mut self, idx: usize, names: Vec<String>) {
        self.bels.insert(BelId::from_idx(idx), names);
    }
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

    pub fn get_bel_name(&self, col: ColId, row: RowId, key: &str) -> Option<&str> {
        let die = DieId::from_idx(0);
        if let Some((layer, _, bel, _)) = self.egrid.find_bel(die, (col, row), key) {
            let nnode = &self.nodes[&(die, col, row, layer)];
            Some(&nnode.bels[bel][0])
        } else {
            None
        }
    }
}
