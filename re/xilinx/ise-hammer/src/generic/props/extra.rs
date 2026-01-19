use prjcombine_entity::EntityVec;
use prjcombine_interconnect::{
    db::{BelAttributeId, BelSlotId, EnumValueId, TileClassId},
    dir::DirV,
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{DiffKey, FeatureId, FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::Fuzzer;
use prjcombine_xilinx_bitstream::{BitRect, Reg};

use crate::backend::IseBackend;

use super::{DynProp, relation::TileRelation};

pub trait KeyMaker: Clone + core::fmt::Debug {
    fn make_key(&self, backend: &IseBackend, main_key: &DiffKey, tcid: TileClassId) -> DiffKey;
}

#[derive(Clone, Debug)]
pub struct ExtraKeyLegacy {
    pub bel: String,
}

impl ExtraKeyLegacy {
    pub fn new(bel: String) -> Self {
        Self { bel }
    }
}

impl KeyMaker for ExtraKeyLegacy {
    fn make_key(&self, backend: &IseBackend, main_key: &DiffKey, tcid: TileClassId) -> DiffKey {
        let DiffKey::Legacy(main_id) = main_key else {
            unreachable!()
        };
        DiffKey::Legacy(FeatureId {
            tile: backend.edev.db.tile_classes.key(tcid).to_string(),
            bel: self.bel.clone(),
            attr: main_id.attr.clone(),
            val: main_id.val.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ExtraKeyBelAttrValue {
    pub bel: BelSlotId,
    pub attr: BelAttributeId,
    pub val: EnumValueId,
}

impl ExtraKeyBelAttrValue {
    pub fn new(bel: BelSlotId, attr: BelAttributeId, val: EnumValueId) -> Self {
        Self { bel, attr, val }
    }
}

impl KeyMaker for ExtraKeyBelAttrValue {
    fn make_key(&self, _backend: &IseBackend, _main_key: &DiffKey, tcid: TileClassId) -> DiffKey {
        DiffKey::BelAttrValue(tcid, self.bel, self.attr, self.val)
    }
}

#[derive(Clone, Debug)]
pub struct ExtraKeyLegacyAttr {
    pub bel: String,
    pub attr: String,
    pub val: String,
}

impl ExtraKeyLegacyAttr {
    pub fn new(bel: String, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl KeyMaker for ExtraKeyLegacyAttr {
    fn make_key(&self, backend: &IseBackend, _main_key: &DiffKey, tcid: TileClassId) -> DiffKey {
        DiffKey::Legacy(FeatureId {
            tile: backend.edev.db.tile_classes.key(tcid).to_string(),
            bel: self.bel.clone(),
            attr: self.attr.clone(),
            val: self.val.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTile<R, K> {
    pub relation: R,
    pub keymaker: K,
}

impl<R, K> ExtraTile<R, K> {
    pub fn new(relation: R, keymaker: K) -> Self {
        Self { relation, keymaker }
    }
}

impl<'b, R: TileRelation + 'b, K: KeyMaker + 'b> FuzzerProp<'b, IseBackend<'b>>
    for ExtraTile<R, K>
{
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
        let key = self.keymaker.make_key(
            backend,
            &fuzzer.info.features[0].key,
            backend.edev[tcrd].class,
        );
        fuzzer.info.features.push(FuzzerFeature {
            key,
            rects: backend.edev.tile_bits(tcrd),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTileMaybe<R, K> {
    pub relation: R,
    pub keymaker: K,
}

impl<R, K> ExtraTileMaybe<R, K> {
    pub fn new(relation: R, keymaker: K) -> Self {
        Self { relation, keymaker }
    }
}

impl<'b, R: TileRelation + 'b, K: KeyMaker + 'b> FuzzerProp<'b, IseBackend<'b>>
    for ExtraTileMaybe<R, K>
{
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
        let key = self.keymaker.make_key(
            backend,
            &fuzzer.info.features[0].key,
            backend.edev[tcrd].class,
        );
        fuzzer.info.features.push(FuzzerFeature {
            key,
            rects: backend.edev.tile_bits(tcrd),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTilesByKind<K> {
    pub kind: TileClassId,
    pub keymaker: K,
}

impl<K> ExtraTilesByKind<K> {
    pub fn new(kind: TileClassId, keymaker: K) -> Self {
        Self { kind, keymaker }
    }
}

impl<'b, K: KeyMaker + 'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTilesByKind<K> {
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
                let key = self.keymaker.make_key(
                    backend,
                    &fuzzer.info.features[0].key,
                    backend.edev[tcrd].class,
                );
                fuzzer.info.features.push(FuzzerFeature {
                    key,
                    rects: backend.edev.tile_bits(tcrd),
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTilesByBel<K> {
    pub slot: BelSlotId,
    pub keymaker: K,
}

impl<K> ExtraTilesByBel<K> {
    pub fn new(slot: BelSlotId, keymaker: K) -> Self {
        Self { slot, keymaker }
    }
}

impl<'b, K: KeyMaker + 'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTilesByBel<K> {
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
                let key = self.keymaker.make_key(
                    backend,
                    &fuzzer.info.features[0].key,
                    backend.edev[tcrd].class,
                );
                fuzzer.info.features.push(FuzzerFeature {
                    key,
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
