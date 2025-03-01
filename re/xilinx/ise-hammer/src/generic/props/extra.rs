use prjcombine_interconnect::{
    db::{BelSlotId, NodeKindId},
    dir::Dir,
    grid::NodeLoc,
};
use prjcombine_re_fpga_hammer::{FeatureId, FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::Fuzzer;
use prjcombine_xilinx_bitstream::{BitTile, Reg};

use crate::backend::IseBackend;

use super::{DynProp, relation::NodeRelation};

#[derive(Clone, Debug)]
pub struct ExtraTile<R> {
    pub relation: R,
    pub bel: Option<String>,
    pub attr: Option<String>,
    pub val: Option<String>,
}

impl<R> ExtraTile<R> {
    pub fn new(
        relation: R,
        bel: Option<String>,
        attr: Option<String>,
        val: Option<String>,
    ) -> Self {
        Self {
            relation,
            bel,
            attr,
            val,
        }
    }
}

impl<'b, R: NodeRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTile<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let nloc = self.relation.resolve(backend, nloc)?;
        let node = backend.egrid.node(nloc);
        let tile = backend.egrid.db.nodes.key(node.kind);
        let main_id = &fuzzer.info.features[0].id;
        let id = FeatureId {
            tile: tile.into(),
            bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
            attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
            val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
        };
        fuzzer.info.features.push(FuzzerFeature {
            id,
            tiles: backend.edev.node_bits(nloc),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTileMaybe<R> {
    pub relation: R,
    pub bel: Option<String>,
    pub attr: Option<String>,
    pub val: Option<String>,
}

impl<R> ExtraTileMaybe<R> {
    pub fn new(
        relation: R,
        bel: Option<String>,
        attr: Option<String>,
        val: Option<String>,
    ) -> Self {
        Self {
            relation,
            bel,
            attr,
            val,
        }
    }
}

impl<'b, R: NodeRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTileMaybe<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let Some(nloc) = self.relation.resolve(backend, nloc) else {
            return Some((fuzzer, true));
        };
        let node = backend.egrid.node(nloc);
        let tile = backend.egrid.db.nodes.key(node.kind);
        let main_id = &fuzzer.info.features[0].id;
        let id = FeatureId {
            tile: tile.into(),
            bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
            attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
            val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
        };
        fuzzer.info.features.push(FuzzerFeature {
            id,
            tiles: backend.edev.node_bits(nloc),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTilesByKind {
    pub kind: NodeKindId,
    pub bel: Option<String>,
    pub attr: Option<String>,
    pub val: Option<String>,
}

impl ExtraTilesByKind {
    pub fn new(
        kind: NodeKindId,
        bel: Option<String>,
        attr: Option<String>,
        val: Option<String>,
    ) -> Self {
        Self {
            kind,
            bel,
            attr,
            val,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTilesByKind {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        if let Some(locs) = backend.egrid.node_index.get(self.kind) {
            for &nloc in locs {
                let node = backend.egrid.node(nloc);
                let tile = backend.egrid.db.nodes.key(node.kind);
                let main_id = &fuzzer.info.features[0].id;
                let id = FeatureId {
                    tile: tile.into(),
                    bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
                    attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
                    val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
                };
                fuzzer.info.features.push(FuzzerFeature {
                    id,
                    tiles: backend.edev.node_bits(nloc),
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTilesByBel {
    pub slot: BelSlotId,
    pub bel: Option<String>,
    pub attr: Option<String>,
    pub val: Option<String>,
}

impl ExtraTilesByBel {
    pub fn new(
        slot: BelSlotId,
        bel: Option<String>,
        attr: Option<String>,
        val: Option<String>,
    ) -> Self {
        Self {
            slot,
            bel,
            attr,
            val,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTilesByBel {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        for (node_kind, locs) in &backend.egrid.node_index {
            let node_kind = &backend.egrid.db.nodes[node_kind];
            if !node_kind.bels.contains_id(self.slot) {
                continue;
            }
            for &nloc in locs {
                let node = backend.egrid.node(nloc);
                let tile = backend.egrid.db.nodes.key(node.kind);
                let main_id = &fuzzer.info.features[0].id;
                let id = FeatureId {
                    tile: tile.into(),
                    bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
                    attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
                    val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
                };
                fuzzer.info.features.push(FuzzerFeature {
                    id,
                    tiles: backend.edev.node_bits(nloc),
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraReg {
    pub regs: Vec<Reg>,
    pub present: bool,
    pub tile: String,
    pub bel: Option<String>,
    pub attr: Option<String>,
    pub val: Option<String>,
}

impl ExtraReg {
    pub fn new(
        regs: Vec<Reg>,
        present: bool,
        tile: String,
        bel: Option<String>,
        attr: Option<String>,
        val: Option<String>,
    ) -> Self {
        Self {
            regs,
            present,
            tile,
            bel,
            attr,
            val,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraReg {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        for die in backend.egrid.die.ids() {
            let main_id = &fuzzer.info.features[0].id;
            let id = FeatureId {
                tile: self.tile.clone(),
                bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
                attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
                val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
            };
            let mut tiles = Vec::from_iter(self.regs.iter().map(|&reg| BitTile::Reg(die, reg)));
            if self.present {
                tiles.extend(self.regs.iter().map(|&reg| BitTile::RegPresent(die, reg)));
            }
            fuzzer.info.features.push(FuzzerFeature { id, tiles });
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraGtz(pub Dir);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraGtz {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        _nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let main_id = &fuzzer.info.features[0].id;
        let id = FeatureId {
            tile: "GTZ".into(),
            bel: "GTZ".into(),
            attr: main_id.attr.clone(),
            val: main_id.val.clone(),
        };
        fuzzer.info.features.push(FuzzerFeature {
            id,
            tiles: vec![BitTile::Gtz(self.0)],
        });
        Some((fuzzer, false))
    }
}
