use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{BelSlotId, TileClassId},
    dir::DirV,
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{DiffKey, FeatureId, FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::Fuzzer;
use prjcombine_xilinx_bitstream::{BitRect, Reg};

use crate::backend::IseBackend;

use super::{DynProp, relation::TileRelation};

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

impl<'b, R: TileRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTile<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tcrd = self.relation.resolve(backend, tcrd)?;
        let tile = backend.edev.db.tile_classes.key(backend.edev[tcrd].class);
        let DiffKey::Legacy(ref main_id) = fuzzer.info.features[0].key else {
            unreachable!()
        };
        let id = FeatureId {
            tile: tile.into(),
            bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
            attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
            val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
        };
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::Legacy(id),
            rects: backend.edev.tile_bits(tcrd),
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

impl<'b, R: TileRelation + 'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTileMaybe<R> {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let Some(tcrd) = self.relation.resolve(backend, tcrd) else {
            return Some((fuzzer, true));
        };
        let tile = backend.edev.db.tile_classes.key(backend.edev[tcrd].class);
        let DiffKey::Legacy(ref main_id) = fuzzer.info.features[0].key else {
            unreachable!()
        };
        let key = DiffKey::Legacy(FeatureId {
            tile: tile.into(),
            bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
            attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
            val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
        });
        fuzzer.info.features.push(FuzzerFeature {
            key,
            rects: backend.edev.tile_bits(tcrd),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTilesByKind {
    pub kind: TileClassId,
    pub bel: Option<String>,
    pub attr: Option<String>,
    pub val: Option<String>,
}

impl ExtraTilesByKind {
    pub fn new(
        kind: TileClassId,
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
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        if let Some(locs) = backend.edev.tile_index.get(self.kind) {
            for &tcrd in locs {
                let tile = backend.edev.db.tile_classes.key(backend.edev[tcrd].class);
                let DiffKey::Legacy(ref main_id) = fuzzer.info.features[0].key else {
                    unreachable!()
                };
                let id = FeatureId {
                    tile: tile.into(),
                    bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
                    attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
                    val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
                };
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::Legacy(id),
                    rects: backend.edev.tile_bits(tcrd),
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
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        for (tcls, locs) in &backend.edev.tile_index {
            let tcls = &backend.edev.db[tcls];
            if !tcls.bels.contains_id(self.slot) {
                continue;
            }
            for &tcrd in locs {
                let tile = backend.edev.db.tile_classes.key(backend.edev[tcrd].class);
                let DiffKey::Legacy(ref main_id) = fuzzer.info.features[0].key else {
                    unreachable!()
                };
                let id = FeatureId {
                    tile: tile.into(),
                    bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
                    attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
                    val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
                };
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::Legacy(id),
                    rects: backend.edev.tile_bits(tcrd),
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
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        for die in backend.edev.die.ids() {
            let DiffKey::Legacy(ref main_id) = fuzzer.info.features[0].key else {
                unreachable!()
            };
            let id = FeatureId {
                tile: self.tile.clone(),
                bel: self.bel.as_ref().unwrap_or(&main_id.bel).clone(),
                attr: self.attr.as_ref().unwrap_or(&main_id.attr).clone(),
                val: self.val.as_ref().unwrap_or(&main_id.val).clone(),
            };
            let mut rects =
                EntityVec::from_iter(self.regs.iter().map(|&reg| BitRect::Reg(die, reg)));
            if self.present {
                rects.extend(self.regs.iter().map(|&reg| BitRect::RegPresent(die, reg)));
            }
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(id),
                rects,
            });
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraGtz(pub DirV);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraGtz {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let DiffKey::Legacy(ref main_id) = fuzzer.info.features[0].key else {
            unreachable!()
        };
        let id = FeatureId {
            tile: "GTZ".into(),
            bel: "GTZ".into(),
            attr: main_id.attr.clone(),
            val: main_id.val.clone(),
        };
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::Legacy(id),
            rects: EntityVec::from_iter([BitRect::Gtz(self.0)]),
        });
        Some((fuzzer, false))
    }
}
