use prjcombine_interconnect::{
    db::{BelInfo, BelSlotId},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::Fuzzer;
use prjcombine_re_xilinx_naming::db::BelNaming;

use crate::backend::{IseBackend, Key, MultiValue, PinFromKind, Value};

use super::DynProp;

#[derive(Clone, Debug)]
pub struct BaseBelMode {
    pub bel: BelSlotId,
    pub val: String,
}

impl BaseBelMode {
    pub fn new(bel: BelSlotId, val: String) -> Self {
        Self { bel, val }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseBelMode {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.base(Key::SiteMode(&ntile.bels[self.bel]), self.val.clone()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BelUnused {
    pub bel: BelSlotId,
}

impl BelUnused {
    pub fn new(bel: BelSlotId) -> Self {
        Self { bel }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BelUnused {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.base(Key::SiteMode(&ntile.bels[self.bel]), None),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelMode {
    pub bel: BelSlotId,
    pub val0: String,
    pub val1: String,
}

impl FuzzBelMode {
    pub fn new(bel: BelSlotId, val0: String, val1: String) -> Self {
        Self { bel, val0, val1 }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzBelMode {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.fuzz(
                Key::SiteMode(&ntile.bels[self.bel]),
                self.val0.clone(),
                self.val1.clone(),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelPin {
    pub bel: BelSlotId,
    pub pin: String,
}

impl BaseBelPin {
    pub fn new(bel: BelSlotId, pin: String) -> Self {
        Self { bel, pin }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseBelPin {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.base(Key::SitePin(&ntile.bels[self.bel], self.pin.clone()), true),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelNoPin {
    pub bel: BelSlotId,
    pub pin: String,
}

impl BaseBelNoPin {
    pub fn new(bel: BelSlotId, pin: String) -> Self {
        Self { bel, pin }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseBelNoPin {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.base(Key::SitePin(&ntile.bels[self.bel], self.pin.clone()), false),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelPin {
    pub bel: BelSlotId,
    pub pin: String,
}

impl FuzzBelPin {
    pub fn new(bel: BelSlotId, pin: String) -> Self {
        Self { bel, pin }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzBelPin {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.fuzz(
                Key::SitePin(&ntile.bels[self.bel], self.pin.clone()),
                None,
                true,
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelPinPips {
    pub bel: BelSlotId,
    pub pin: String,
}

impl BaseBelPinPips {
    pub fn new(bel: BelSlotId, pin: String) -> Self {
        Self { bel, pin }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseBelPinPips {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        let tile_naming = &backend.ngrid.db.tile_class_namings[ntile.naming];
        let BelNaming::Bel(bel_naming) = &tile_naming.bels[self.bel] else {
            unreachable!()
        };
        let pin_naming = &bel_naming.pins[&self.pin];
        for pip in &pin_naming.pips {
            fuzzer = fuzzer.base(
                Key::Pip(&ntile.names[pip.tile], &pip.wire_from, &pip.wire_to),
                true,
            );
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelPinPips {
    pub bel: BelSlotId,
    pub pin: String,
}

impl FuzzBelPinPips {
    pub fn new(bel: BelSlotId, pin: String) -> Self {
        Self { bel, pin }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzBelPinPips {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        let tile_naming = &backend.ngrid.db.tile_class_namings[ntile.naming];
        let BelNaming::Bel(bel_naming) = &tile_naming.bels[self.bel] else {
            unreachable!()
        };
        let pin_naming = &bel_naming.pins[&self.pin];
        for pip in &pin_naming.pips {
            fuzzer = fuzzer.fuzz(
                Key::Pip(&ntile.names[pip.tile], &pip.wire_from, &pip.wire_to),
                None,
                true,
            );
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelPinIntPips {
    pub bel: BelSlotId,
    pub pin: String,
}

impl FuzzBelPinIntPips {
    pub fn new(bel: BelSlotId, pin: String) -> Self {
        Self { bel, pin }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzBelPinIntPips {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tile = &backend.egrid[tcrd];
        let tcls = &backend.egrid.db.tile_classes[tile.class];
        let ntile = &backend.ngrid.tiles[&tcrd];
        let tile_naming = &backend.ngrid.db.tile_class_namings[ntile.naming];
        let bel_data = &tcls.bels[self.bel];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let pin_data = &bel_data.pins[&self.pin];
        let BelNaming::Bel(bel_naming) = &tile_naming.bels[self.bel] else {
            unreachable!()
        };
        let pin_naming = &bel_naming.pins[&self.pin];
        assert_eq!(pin_data.wires.len(), 1);
        let wire = *pin_data.wires.first().unwrap();
        if let Some(pip) = pin_naming.int_pips.get(&wire) {
            fuzzer = fuzzer.fuzz(
                Key::Pip(&ntile.names[pip.tile], &pip.wire_from, &pip.wire_to),
                false,
                true,
            );
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelPinFrom {
    pub bel: BelSlotId,
    pub pin: String,
    pub from: PinFromKind,
}

impl BaseBelPinFrom {
    pub fn new(bel: BelSlotId, pin: String, from: PinFromKind) -> Self {
        Self { bel, pin, from }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseBelPinFrom {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.base(
                Key::SitePinFrom(&ntile.bels[self.bel], self.pin.clone()),
                self.from,
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelPinFrom {
    pub bel: BelSlotId,
    pub pin: String,
    pub from0: PinFromKind,
    pub from1: PinFromKind,
}

impl FuzzBelPinFrom {
    pub fn new(bel: BelSlotId, pin: String, from0: PinFromKind, from1: PinFromKind) -> Self {
        Self {
            bel,
            pin,
            from0,
            from1,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzBelPinFrom {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.fuzz(
                Key::SitePinFrom(&ntile.bels[self.bel], self.pin.clone()),
                self.from0,
                self.from1,
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelPinPair {
    pub bel_to: BelSlotId,
    pub pin_to: String,
    pub bel_from: BelSlotId,
    pub pin_from: String,
}

impl BaseBelPinPair {
    pub fn new(bel_to: BelSlotId, pin_to: String, bel_from: BelSlotId, pin_from: String) -> Self {
        Self {
            bel_to,
            pin_to,
            bel_from,
            pin_from,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseBelPinPair {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        let site_to = &ntile.bels[self.bel_to];
        let site_from = &ntile.bels[self.bel_from];

        Some((
            fuzzer.base(
                Key::SitePin(site_to, self.pin_to.clone()),
                Value::FromPin(site_from, self.pin_from.clone()),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelPinPair {
    pub bel_to: BelSlotId,
    pub pin_to: String,
    pub bel_from: BelSlotId,
    pub pin_from: String,
}

impl FuzzBelPinPair {
    pub fn new(bel_to: BelSlotId, pin_to: String, bel_from: BelSlotId, pin_from: String) -> Self {
        Self {
            bel_to,
            pin_to,
            bel_from,
            pin_from,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzBelPinPair {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        let site_to = &ntile.bels[self.bel_to];
        let site_from = &ntile.bels[self.bel_from];

        Some((
            fuzzer.fuzz(
                Key::SitePin(site_to, self.pin_to.clone()),
                false,
                Value::FromPin(site_from, self.pin_from.clone()),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BaseBelAttr {
    pub bel: BelSlotId,
    pub attr: String,
    pub val: String,
}

impl BaseBelAttr {
    pub fn new(bel: BelSlotId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseBelAttr {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.base(
                Key::SiteAttr(&ntile.bels[self.bel], self.attr.clone()),
                self.val.clone(),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelAttr {
    pub bel: BelSlotId,
    pub attr: String,
    pub val_a: String,
    pub val_b: String,
}

impl FuzzBelAttr {
    pub fn new(bel: BelSlotId, attr: String, val_a: String, val_b: String) -> Self {
        Self {
            bel,
            attr,
            val_a,
            val_b,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzBelAttr {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        Some((
            fuzzer.fuzz(
                Key::SiteAttr(&ntile.bels[self.bel], self.attr.clone()),
                self.val_a.clone(),
                self.val_b.clone(),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzBelMultiAttr {
    pub bel: BelSlotId,
    pub attr: String,
    pub val: MultiValue,
    pub width: usize,
}

impl FuzzBelMultiAttr {
    pub fn new(bel: BelSlotId, attr: String, val: MultiValue, width: usize) -> Self {
        Self {
            bel,
            attr,
            val,
            width,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzBelMultiAttr {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        fuzzer.bits = self.width;
        Some((
            fuzzer.fuzz_multi(
                Key::SiteAttr(&ntile.bels[self.bel], self.attr.clone()),
                self.val,
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct BelMutex {
    pub bel: BelSlotId,
    pub attr: String,
    pub val: String,
}

impl BelMutex {
    pub fn new(bel: BelSlotId, attr: String, val: String) -> Self {
        Self { bel, attr, val }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BelMutex {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let bel = tcrd.bel(self.bel);
        Some((
            fuzzer.base(Key::BelMutex(bel, self.attr.clone()), self.val.clone()),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct RowMutexHere {
    pub bel: BelSlotId,
    pub key: String,
}

impl RowMutexHere {
    pub fn new(bel: BelSlotId, key: String) -> Self {
        Self { bel, key }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for RowMutexHere {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        Some((
            fuzzer.base(
                Key::RowMutex(self.key.clone(), tcrd.row),
                Value::Bel(tcrd.bel(self.bel)),
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct GlobalMutexHere {
    pub bel: BelSlotId,
    pub key: String,
}

impl GlobalMutexHere {
    pub fn new(bel: BelSlotId, key: String) -> Self {
        Self { bel, key }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for GlobalMutexHere {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        Some((
            fuzzer.base(
                Key::GlobalMutex(self.key.clone()),
                Value::Bel(tcrd.bel(self.bel)),
            ),
            false,
        ))
    }
}

fn resolve_global_xy(backend: &IseBackend, tcrd: TileCoord, slot: BelSlotId, opt: &str) -> String {
    let site = &backend.ngrid.tiles[&tcrd].bels[slot];
    opt.replace('*', &site[site.rfind('X').unwrap()..])
}

#[derive(Clone, Debug)]
pub struct BaseGlobalXy {
    pub bel: BelSlotId,
    pub opt: String,
    pub val: String,
}

impl BaseGlobalXy {
    pub fn new(bel: BelSlotId, opt: String, val: String) -> Self {
        Self { bel, opt, val }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BaseGlobalXy {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let opt = resolve_global_xy(backend, tcrd, self.bel, &self.opt);
        Some((fuzzer.base(Key::GlobalOpt(opt), self.val.clone()), false))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzGlobalXy {
    pub bel: BelSlotId,
    pub opt: String,
    pub val0: Option<String>,
    pub val1: Option<String>,
}

impl FuzzGlobalXy {
    pub fn new(bel: BelSlotId, opt: String, val0: Option<String>, val1: Option<String>) -> Self {
        Self {
            bel,
            opt,
            val0,
            val1,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzGlobalXy {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let opt = resolve_global_xy(backend, tcrd, self.bel, &self.opt);
        Some((
            fuzzer.fuzz(
                Key::GlobalOpt(opt),
                match self.val0 {
                    Some(ref s) => Value::String(s.clone()),
                    None => Value::None,
                },
                match self.val1 {
                    Some(ref s) => Value::String(s.clone()),
                    None => Value::None,
                },
            ),
            false,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct FuzzMultiGlobalXy {
    pub bel: BelSlotId,
    pub opt: String,
    pub val: MultiValue,
    pub width: usize,
}

impl FuzzMultiGlobalXy {
    pub fn new(bel: BelSlotId, opt: String, val: MultiValue, width: usize) -> Self {
        Self {
            bel,
            opt,
            val,
            width,
        }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzMultiGlobalXy {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let opt = resolve_global_xy(backend, tcrd, self.bel, &self.opt);
        fuzzer.bits = self.width;
        Some((fuzzer.fuzz_multi(Key::GlobalOpt(opt), self.val), false))
    }
}

#[derive(Clone, Debug)]
pub struct ForceBelName(pub String);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ForceBelName {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        _backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        fuzzer.info.features[0].id.bel = self.0.clone();
        Some((fuzzer, false))
    }
}
