use std::collections::{BTreeMap, HashMap, HashSet, hash_map};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttributeId, BelSlotId, CellSlotId, TableFieldId, TableRowId, TileClassId},
    dir::Dir,
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, SpecialId, enum_ocd_swap_bits, extract_bitvec_val, xlat_bit,
    xlat_bit_wide, xlat_bitvec_sparse, xlat_enum_attr, xlat_enum_raw,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{
    Bond, Device, ExpandedBond, ExpandedDevice, ExpandedNamedDevice, GeomDb,
};
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{PolTileBit, TileBit},
};
use prjcombine_virtex2::{
    chip::{ChipKind, IoDiffKind},
    defs::{
        self, bcls, bslots, enums,
        spartan3::tcls as tcls_s3,
        tables::{self, IOB_DATA},
        virtex2::tcls as tcls_v2,
    },
    iob::{IobData, IobDiff, IobKind, get_iob_data, get_iob_tiles},
};

use crate::{
    backend::{IseBackend, Key, MultiValue, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzBuilderBelTestManual, FuzzCtx},
        iostd::{DciKind, DiffKind, Iostd},
        props::{
            DynProp,
            bel::FuzzBelMultiAttr,
            pip::PinFar,
            relation::{NoopRelation, Related, TileRelation},
        },
    },
    virtex2::specials,
};

#[derive(Clone, Debug)]
struct NotIbuf;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for NotIbuf {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ntile = &backend.ngrid.tiles[&tcrd];
        if !ntile.names[RawTileId::from_idx(0)].contains("IOIS") {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct IsVref(BelSlotId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for IsVref {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
        if !ebond.bond.vref.contains(&crd) {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct IsVr(BelSlotId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for IsVr {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
        let mut is_vr = false;
        for (bank, vr) in &edev.chip.dci_io {
            if vr.0 == crd || vr.1 == crd {
                if edev.chip.dci_io_alt.contains_key(bank) {
                    let &FuzzerValue::Base(Value::Bool(alt)) = &fuzzer.kv[&Key::AltVr] else {
                        unreachable!()
                    };
                    is_vr = !alt;
                } else {
                    is_vr = true;
                }
            }
        }
        for (bank, vr) in &edev.chip.dci_io_alt {
            if vr.0 == crd || vr.1 == crd {
                if edev.chip.dci_io.contains_key(bank) {
                    let &FuzzerValue::Base(Value::Bool(alt)) = &fuzzer.kv[&Key::AltVr] else {
                        unreachable!()
                    };
                    is_vr = alt;
                } else {
                    is_vr = true;
                }
            }
        }
        if !is_vr {
            return None;
        }
        if !ebond.ios.contains_key(&crd) {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct OtherIobInput(BelSlotId, String);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for OtherIobInput {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let ExpandedNamedDevice::Virtex2(endev) = backend.endev else {
            unreachable!()
        };
        let edev = endev.edev;
        let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
        let orig_io_info = edev.chip.get_io_info(crd);
        for io in edev.chip.get_bonded_ios() {
            let io_info = edev.chip.get_io_info(io);
            if io != crd
                && orig_io_info.bank == io_info.bank
                && io_info.pad_kind != Some(IobKind::Clk)
                && ebond.ios.contains_key(&io)
            {
                let site = endev.get_io_name(io);
                fuzzer = fuzzer.base(
                    Key::SiteMode(site),
                    if edev.chip.kind.is_spartan3ea() {
                        "IBUF"
                    } else {
                        "IOB"
                    },
                );
                fuzzer = fuzzer.base(Key::SiteAttr(site, "IOATTRBOX".into()), self.1.clone());
                fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), "1");
                fuzzer = fuzzer.base(Key::SitePin(site, "I".into()), true);
                if edev.chip.kind.is_spartan3a() {
                    fuzzer = fuzzer.base(Key::SiteAttr(site, "IBUF_DELAY_VALUE".into()), "DLY0");
                    fuzzer = fuzzer.base(Key::SiteAttr(site, "DELAY_ADJ_ATTRBOX".into()), "FIXED");
                    fuzzer = fuzzer.base(Key::SiteAttr(site, "SEL_MUX".into()), "0");
                }
                return Some((fuzzer, false));
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
struct OtherIobDiffOutput(BelSlotId, String);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for OtherIobDiffOutput {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let ExpandedNamedDevice::Virtex2(endev) = backend.endev else {
            unreachable!()
        };
        let edev = endev.edev;

        let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
        let orig_io_info = edev.chip.get_io_info(crd);
        for io in edev.chip.get_bonded_ios() {
            let io_info = edev.chip.get_io_info(io);
            if io != crd
                && orig_io_info.bank == io_info.bank
                && io_info.pad_kind != Some(IobKind::Clk)
                && io_info.diff != IoDiffKind::None
                && ebond.ios.contains_key(&io)
            {
                let site = endev.get_io_name(io);

                fuzzer = fuzzer.base(
                    Key::SiteMode(site),
                    match io_info.diff {
                        IoDiffKind::P(_) => {
                            if edev.chip.kind.is_spartan3a() {
                                "DIFFMI_NDT"
                            } else if edev.chip.kind.is_spartan3ea() {
                                "DIFFMI"
                            } else {
                                "DIFFM"
                            }
                        }
                        IoDiffKind::N(_) => {
                            if edev.chip.kind.is_spartan3a() {
                                "DIFFSI_NDT"
                            } else if edev.chip.kind.is_spartan3ea() {
                                "DIFFSI"
                            } else {
                                "DIFFS"
                            }
                        }
                        IoDiffKind::None => {
                            unreachable!()
                        }
                    },
                );
                fuzzer = fuzzer.base(Key::SiteAttr(site, "IOATTRBOX".into()), self.1.clone());
                fuzzer = fuzzer.base(Key::SiteAttr(site, "OMUX".into()), "O1");
                fuzzer = fuzzer.base(Key::SiteAttr(site, "O1INV".into()), "O1");
                fuzzer = fuzzer.base(Key::SitePin(site, "O1".into()), true);
                return Some((fuzzer, false));
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
struct BankDiffOutput(BelSlotId, String, Option<String>);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for BankDiffOutput {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex2(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let ExpandedNamedDevice::Virtex2(endev) = backend.endev else {
            unreachable!()
        };
        let edev = endev.edev;

        let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
        let stds = if let Some(ref stdb) = self.2 {
            &[&self.1, stdb][..]
        } else {
            &[&self.1][..]
        };
        let bank = edev.chip.get_io_info(crd).bank;
        let mut done = 0;
        let mut ios = edev.chip.get_bonded_ios();
        if edev.chip.kind != ChipKind::Spartan3ADsp {
            ios.reverse();
        }
        for &io in &ios {
            if io == crd {
                if edev.chip.kind.is_spartan3ea() {
                    // too much thinking. just pick a different loc.
                    return None;
                } else {
                    continue;
                }
            }
            let io_info = edev.chip.get_io_info(io);
            if !ebond.ios.contains_key(&io)
                || io_info.bank != bank
                || io_info.pad_kind != Some(IobKind::Iob)
            {
                continue;
            }
            let IoDiffKind::P(other_iob) = io_info.diff else {
                continue;
            };
            // okay, got a pair.
            let other_io = io.with_iob(other_iob);
            let site_p = endev.get_io_name(io);
            let site_n = endev.get_io_name(other_io);
            let std = stds[done];
            fuzzer = fuzzer
                .base(
                    Key::SiteMode(site_p),
                    if edev.chip.kind.is_spartan3a() {
                        "DIFFMTB"
                    } else {
                        "DIFFM"
                    },
                )
                .base(
                    Key::SiteMode(site_n),
                    if edev.chip.kind.is_spartan3a() {
                        "DIFFSTB"
                    } else {
                        "DIFFS"
                    },
                )
                .base(Key::SiteAttr(site_p, "IOATTRBOX".into()), std)
                .base(Key::SiteAttr(site_n, "IOATTRBOX".into()), std)
                .base(Key::SiteAttr(site_p, "OMUX".into()), "O1")
                .base(Key::SiteAttr(site_p, "O1INV".into()), "O1")
                .base(Key::SitePin(site_p, "O1".into()), true)
                .base(Key::SitePin(site_p, "DIFFO_OUT".into()), true)
                .base(Key::SitePin(site_n, "DIFFO_IN".into()), true)
                .base(Key::SiteAttr(site_n, "DIFFO_IN_USED".into()), "0");
            if edev.chip.kind.is_spartan3a() {
                fuzzer = fuzzer
                    .base(Key::SiteAttr(site_p, "SUSPEND".into()), "3STATE")
                    .base(Key::SiteAttr(site_n, "SUSPEND".into()), "3STATE");
            }
            done += 1;
            if done == stds.len() {
                break;
            }
        }
        if done != stds.len() {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
struct IobRelation(CellSlotId);

impl TileRelation for IobRelation {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let cell = backend.edev.tile_cell(tcrd, self.0);
        Some(cell.tile(defs::tslots::BEL))
    }
}

#[derive(Copy, Clone, Debug)]
struct IobBrefclkClkBT;

impl TileRelation for IobBrefclkClkBT {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        if tcrd.col != edev.chip.col_clk && tcrd.col != edev.chip.col_clk - 1 {
            return None;
        }
        Some(tcrd.with_col(edev.chip.col_clk).tile(defs::tslots::CLK))
    }
}

#[derive(Clone, Debug)]
struct Iobify(IobData);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Iobify {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        _backend: &IseBackend<'b>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        match fuzzer.info.features[0].key {
            DiffKey::BelAttrBit(_, ref mut bslot, _, _, _)
            | DiffKey::BelAttrValue(_, ref mut bslot, _, _)
            | DiffKey::BelAttrU32(_, ref mut bslot, _, _)
            | DiffKey::BelAttrBitVec(_, ref mut bslot, _, _)
            | DiffKey::BelSpecial(_, ref mut bslot, _)
            | DiffKey::BelSpecialRow(_, ref mut bslot, _, _)
            | DiffKey::BelSpecialSpecialSpecialRow(_, ref mut bslot, _, _, _, _)
            | DiffKey::BelSpecialBit(_, ref mut bslot, _, _) => {
                assert_eq!(*bslot, self.0.ioi);
                *bslot = self.0.iob;
            }
            _ => panic!("how to xlat {key:?}", key = fuzzer.info.features[0].key),
        }
        Some((fuzzer, false))
    }
}

trait IobBuilderExt {
    fn iob_commit(self, iob: IobData);
}

impl IobBuilderExt for FuzzBuilderBelTestManual<'_, '_> {
    fn iob_commit(mut self, iob: IobData) {
        self.props = std::mem::take(&mut self.props)
            .into_iter()
            .map(|prop| -> Box<DynProp> {
                Box::new(Related::new_boxed(IobRelation(iob.cell), prop))
            })
            .collect();
        self.prop(Iobify(iob)).commit()
    }
}

fn has_any_vref<'a>(
    edev: &prjcombine_virtex2::expanded::ExpandedDevice,
    device: &'a Device,
    db: &GeomDb,
    tcid: TileClassId,
    iob: BelSlotId,
) -> Option<&'a str> {
    let iob_idx = bslots::IOB.index_of(iob).unwrap();
    let iobs = get_iob_data(edev.chip.kind, tcid).iobs;
    let ioi_cell = iobs[iob_idx].cell;
    let ioi_bel = iobs[iob_idx].ioi;
    let mut bonded_ios = HashMap::new();
    for devbond in device.bonds.values() {
        let bond = &db.bonds[devbond.bond];
        let Bond::Virtex2(bond) = bond else {
            unreachable!()
        };
        for &io in &bond.vref {
            bonded_ios.insert(io, &devbond.name[..]);
        }
    }
    for &tcrd in &edev.tile_index[tcid] {
        let cell = edev.tile_cell(tcrd, ioi_cell);
        let crd = edev.chip.get_io_crd(cell.bel(ioi_bel));
        if let Some(&pkg) = bonded_ios.get(&crd) {
            return Some(pkg);
        }
    }
    None
}

fn has_any_vr<'a>(
    edev: &prjcombine_virtex2::expanded::ExpandedDevice,
    device: &'a Device,
    db: &GeomDb,
    tcid: TileClassId,
    iob: BelSlotId,
) -> Option<(&'a str, Option<bool>)> {
    let iob_idx = bslots::IOB.index_of(iob).unwrap();
    let iobs = get_iob_data(edev.chip.kind, tcid).iobs;
    let ioi_cell = iobs[iob_idx].cell;
    let ioi_bel = iobs[iob_idx].ioi;
    let mut bonded_ios = HashMap::new();
    for devbond in device.bonds.values() {
        let bond = &db.bonds[devbond.bond];
        let Bond::Virtex2(bond) = bond else {
            unreachable!()
        };
        for pin in bond.pins.values() {
            if let prjcombine_virtex2::bond::BondPad::Io(io) = pin {
                bonded_ios.insert(io, &devbond.name[..]);
            }
        }
    }
    for &tcrd in &edev.tile_index[tcid] {
        let cell = edev.tile_cell(tcrd, ioi_cell);
        let crd = edev.chip.get_io_crd(cell.bel(ioi_bel));
        if let Some(&pkg) = bonded_ios.get(&crd) {
            for bank in 0..8 {
                if let Some(alt_vr) = edev.chip.dci_io_alt.get(&bank) {
                    if crd == alt_vr.0 || crd == alt_vr.1 {
                        return Some((pkg, Some(true)));
                    }
                    if let Some(vr) = edev.chip.dci_io_alt.get(&bank)
                        && (crd == vr.0 || crd == vr.1)
                    {
                        return Some((pkg, Some(false)));
                    }
                } else if let Some(vr) = edev.chip.dci_io.get(&bank)
                    && (crd == vr.0 || crd == vr.1)
                {
                    return Some((pkg, None));
                }
            }
        }
    }
    None
}

fn has_any_brefclk(
    edev: &prjcombine_virtex2::expanded::ExpandedDevice,
    tcid: TileClassId,
    iob: BelSlotId,
) -> Option<(usize, usize)> {
    let iob_idx = bslots::IOB.index_of(iob).unwrap();
    if edev.chip.kind != ChipKind::Virtex2P {
        return None;
    }
    match (tcid, iob_idx) {
        (tcls_v2::IOB_V2P_SW2, 5) => Some((1, 0)),
        (tcls_v2::IOB_V2P_SE2, 1) => Some((0, 6)),
        (tcls_v2::IOB_V2P_NW2, 1) => Some((1, 2)),
        (tcls_v2::IOB_V2P_NE2, 5) => Some((0, 4)),
        _ => None,
    }
}

#[allow(clippy::nonminimal_bool)]
pub fn get_iostds(edev: &prjcombine_virtex2::expanded::ExpandedDevice, is_we: bool) -> Vec<Iostd> {
    let mut res = vec![];
    // plain push-pull
    if edev.chip.kind.is_virtex2() {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16]),
            Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8, 12, 16]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else if edev.chip.kind == ChipKind::Spartan3 {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16]),
            Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8, 12]),
            Iostd::cmos("LVCMOS12", 1200, &[2, 4, 6]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
        ]);
    } else if edev.chip.kind == ChipKind::Spartan3E {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16]),
            Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16]),
            Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12]),
            Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8]),
            Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6]),
            Iostd::cmos("LVCMOS12", 1200, &[2]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else if is_we {
        // spartan3a lr
        res.extend([
            Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16]),
            Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8, 12]),
            Iostd::cmos("LVCMOS12", 1200, &[2, 4, 6]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else {
        // spartan3a tb
        res.extend([
            Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16, 24]),
            Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16]),
            Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12]),
            Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8]),
            Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6]),
            Iostd::cmos("LVCMOS12", 1200, &[2]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    }
    // DCI output
    if !edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::odci("LVDCI_33", 3300));
        res.push(Iostd::odci("LVDCI_25", 2500));
        res.push(Iostd::odci("LVDCI_18", 1800));
        res.push(Iostd::odci("LVDCI_15", 1500));
        if !edev.chip.kind.is_virtex2p() {
            res.push(Iostd::odci_half("LVDCI_DV2_33", 3300));
        }
        res.push(Iostd::odci_half("LVDCI_DV2_25", 2500));
        res.push(Iostd::odci_half("LVDCI_DV2_18", 1800));
        res.push(Iostd::odci_half("LVDCI_DV2_15", 1500));
        res.push(Iostd::odci_vref("HSLVDCI_33", 3300, 1650));
        res.push(Iostd::odci_vref("HSLVDCI_25", 2500, 1250));
        res.push(Iostd::odci_vref("HSLVDCI_18", 1800, 900));
        res.push(Iostd::odci_vref("HSLVDCI_15", 1500, 750));
    }
    // VREF-based
    if !edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::vref_od("GTL", 800));
        res.push(Iostd::vref_od("GTLP", 1000));
    }
    if edev.chip.kind == ChipKind::Virtex2 {
        res.push(Iostd::vref("AGP", 3300, 1320));
    }
    if edev.chip.kind == ChipKind::Virtex2 || edev.chip.kind.is_spartan3a() {
        res.push(Iostd::vref("SSTL3_I", 3300, 1500));
        res.push(Iostd::vref("SSTL3_II", 3300, 1500));
    }
    res.push(Iostd::vref("SSTL2_I", 2500, 1250));
    res.push(Iostd::vref("SSTL18_I", 1800, 900));
    if edev.chip.kind != ChipKind::Spartan3E && !(edev.chip.kind.is_spartan3a() && !is_we) {
        res.push(Iostd::vref("SSTL2_II", 2500, 1250));
        res.push(Iostd::vref("SSTL18_II", 1800, 900));
    }
    res.push(Iostd::vref("HSTL_I_18", 1800, 900));
    if edev.chip.kind != ChipKind::Spartan3E && !(edev.chip.kind.is_spartan3a() && !is_we) {
        res.push(Iostd::vref("HSTL_II_18", 1800, 900));
    }
    res.push(Iostd::vref("HSTL_III_18", 1800, 1100));
    if edev.chip.kind.is_virtex2() {
        res.push(Iostd::vref("HSTL_IV_18", 1800, 1100));
    }
    if edev.chip.kind != ChipKind::Spartan3E && !(edev.chip.kind.is_spartan3a() && !is_we) {
        res.push(Iostd::vref("HSTL_I", 1500, 750));
        res.push(Iostd::vref("HSTL_III", 1500, 900));
    }
    if edev.chip.kind.is_virtex2() {
        res.push(Iostd::vref("HSTL_II", 1500, 750));
        res.push(Iostd::vref("HSTL_IV", 1500, 900));
    }
    // VREF-based with DCI
    if !edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::vref_dci_od("GTL_DCI", 1200, 800));
        res.push(Iostd::vref_dci_od("GTLP_DCI", 1500, 1000));
        if edev.chip.kind == ChipKind::Virtex2 {
            res.push(Iostd::vref_dci(
                "SSTL3_I_DCI",
                3300,
                1500,
                DciKind::InputSplit,
            ));
            res.push(Iostd::vref_dci(
                "SSTL3_II_DCI",
                3300,
                1500,
                DciKind::BiSplit,
            ));
        }
        res.push(Iostd::vref_dci(
            "SSTL2_I_DCI",
            2500,
            1250,
            DciKind::InputSplit,
        ));
        res.push(Iostd::vref_dci(
            "SSTL2_II_DCI",
            2500,
            1250,
            DciKind::BiSplit,
        ));
        res.push(Iostd::vref_dci(
            "SSTL18_I_DCI",
            1800,
            900,
            DciKind::InputSplit,
        ));
        if edev.chip.kind.is_virtex2() {
            res.push(Iostd::vref_dci(
                "SSTL18_II_DCI",
                1800,
                900,
                DciKind::BiSplit,
            ));
        }

        res.push(Iostd::vref_dci(
            "HSTL_I_DCI_18",
            1800,
            900,
            DciKind::InputSplit,
        ));
        res.push(Iostd::vref_dci(
            "HSTL_II_DCI_18",
            1800,
            900,
            DciKind::BiSplit,
        ));
        res.push(Iostd::vref_dci(
            "HSTL_III_DCI_18",
            1800,
            1100,
            DciKind::InputVcc,
        ));
        if edev.chip.kind.is_virtex2() {
            res.push(Iostd::vref_dci(
                "HSTL_IV_DCI_18",
                1800,
                1100,
                DciKind::BiVcc,
            ));
        }
        res.push(Iostd::vref_dci(
            "HSTL_I_DCI",
            1500,
            750,
            DciKind::InputSplit,
        ));
        res.push(Iostd::vref_dci(
            "HSTL_III_DCI",
            1500,
            900,
            DciKind::InputVcc,
        ));
        if edev.chip.kind.is_virtex2() {
            res.push(Iostd::vref_dci("HSTL_II_DCI", 1500, 750, DciKind::BiSplit));
            res.push(Iostd::vref_dci("HSTL_IV_DCI", 1500, 900, DciKind::BiVcc));
        }
    }
    // pseudo-diff
    if edev.chip.kind.is_spartan3a() {
        res.push(Iostd::pseudo_diff("DIFF_SSTL3_I", 3300));
        res.push(Iostd::pseudo_diff("DIFF_SSTL3_II", 3300));
    }
    if edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::pseudo_diff("DIFF_SSTL2_I", 2500));
    }
    if edev.chip.kind != ChipKind::Spartan3E && !(edev.chip.kind.is_spartan3a() && !is_we) {
        res.push(Iostd::pseudo_diff("DIFF_SSTL2_II", 2500));
    }
    if edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::pseudo_diff("DIFF_SSTL18_I", 1800));
    }
    if edev.chip.kind.is_virtex2() || (edev.chip.kind.is_spartan3a() && is_we) {
        res.push(Iostd::pseudo_diff("DIFF_SSTL18_II", 1800));
    }
    if edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800));
        res.push(Iostd::pseudo_diff("DIFF_HSTL_III_18", 1800));
    }
    if !edev.chip.kind.is_spartan3ea() || (edev.chip.kind.is_spartan3a() && is_we) {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800));
    }
    if edev.chip.kind.is_spartan3a() && is_we {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_I", 1500));
        res.push(Iostd::pseudo_diff("DIFF_HSTL_III", 1500));
    }
    if edev.chip.kind.is_virtex2() {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_II", 1500));
    }
    res.push(Iostd {
        name: if edev.chip.kind == ChipKind::Virtex2 {
            "LVPECL_33"
        } else {
            "LVPECL_25"
        },
        vcco: Some(2500),
        vref: None,
        diff: DiffKind::Pseudo,
        dci: DciKind::None,
        drive: &[],
        input_only: edev.chip.kind.is_spartan3ea(),
    });
    res.push(Iostd::pseudo_diff("BLVDS_25", 2500));
    // pseudo-diff with DCI
    if !edev.chip.kind.is_spartan3ea() {
        if edev.chip.kind.is_virtex2() {
            res.push(Iostd::pseudo_diff_dci(
                "DIFF_HSTL_II_DCI",
                1500,
                DciKind::BiSplit,
            ));
            res.push(Iostd::pseudo_diff_dci(
                "DIFF_SSTL18_II_DCI",
                1800,
                DciKind::BiSplit,
            ));
        }
        res.push(Iostd::pseudo_diff_dci(
            "DIFF_HSTL_II_DCI_18",
            1800,
            DciKind::BiSplit,
        ));
        res.push(Iostd::pseudo_diff_dci(
            "DIFF_SSTL2_II_DCI",
            2500,
            DciKind::BiSplit,
        ));
    }
    // true diff
    res.push(Iostd::true_diff("LVDS_25", 2500));
    if edev.chip.kind == ChipKind::Virtex2 || edev.chip.kind.is_spartan3a() {
        res.push(Iostd::true_diff("LVDS_33", 3300));
    }
    if !edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::true_diff("LVDSEXT_25", 2500));
        res.push(Iostd::true_diff("ULVDS_25", 2500));
        res.push(Iostd::true_diff("LDT_25", 2500));
    }
    if edev.chip.kind == ChipKind::Virtex2 {
        res.push(Iostd::true_diff("LVDSEXT_33", 3300));
    }
    if !edev.chip.kind.is_virtex2() {
        res.push(Iostd::true_diff("RSDS_25", 2500));
    }
    if edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::true_diff("MINI_LVDS_25", 2500));
    }
    if edev.chip.kind.is_spartan3a() {
        res.push(Iostd::true_diff("PPDS_25", 2500));
        res.push(Iostd::true_diff("RSDS_33", 3300));
        res.push(Iostd::true_diff("MINI_LVDS_33", 3300));
        res.push(Iostd::true_diff("PPDS_33", 3300));
        res.push(Iostd::true_diff("TMDS_33", 3300));
    }
    if edev.chip.kind.is_virtex2p() {
        res.push(Iostd::true_diff_term("LVDS_25_DT", 2500));
        res.push(Iostd::true_diff_term("LVDSEXT_25_DT", 2500));
        res.push(Iostd::true_diff_term("LDT_25_DT", 2500));
        res.push(Iostd::true_diff_term("ULVDS_25_DT", 2500));
    }
    // true diff with DCI
    if !edev.chip.kind.is_spartan3ea() {
        if edev.chip.kind == ChipKind::Virtex2 {
            res.push(Iostd::true_diff_dci("LVDS_33_DCI", 3300));
            res.push(Iostd::true_diff_dci("LVDSEXT_33_DCI", 3300));
        }
        res.push(Iostd::true_diff_dci("LVDS_25_DCI", 2500));
        res.push(Iostd::true_diff_dci("LVDSEXT_25_DCI", 2500));
    }
    res
}

pub fn iostd_to_row(
    edev: &prjcombine_virtex2::expanded::ExpandedDevice,
    std: &Iostd,
) -> TableRowId {
    let mut name = if let Some(name) = std.name.strip_suffix("_DT") {
        name
    } else if let Some(name) = std.name.strip_prefix("DIFF_") {
        name
    } else {
        std.name
    };
    if name == "ULVDS_25" {
        name = "MINI_LVDS_25";
    }
    if name == "LDT_25" {
        name = "HT_25";
    }
    edev.db.tables[IOB_DATA].rows.get(name).unwrap().0
}

fn iostd_is_dt(iostd: &Iostd) -> bool {
    iostd.name.ends_with("_DT")
}

fn drive_to_spec(drive: u8) -> SpecialId {
    match drive {
        0 => specials::IOB_DRIVE_NONE,
        2 => specials::IOB_DRIVE_2,
        4 => specials::IOB_DRIVE_4,
        6 => specials::IOB_DRIVE_6,
        8 => specials::IOB_DRIVE_8,
        12 => specials::IOB_DRIVE_12,
        16 => specials::IOB_DRIVE_16,
        24 => specials::IOB_DRIVE_24,
        _ => unreachable!(),
    }
}

fn spec_to_slew(special: SpecialId) -> &'static str {
    match special {
        specials::IOB_SLEW_NONE => "",
        specials::IOB_SLEW_FAST => "FAST",
        specials::IOB_SLEW_SLOW => "SLOW",
        specials::IOB_SLEW_QUIETIO => "QUIETIO",
        _ => unreachable!(),
    }
}

fn get_drive_row(std: TableRowId, drive: u8) -> TableRowId {
    match (std, drive) {
        (_, 0) => std,
        (IOB_DATA::LVCMOS12, 2) => IOB_DATA::LVCMOS12_2,
        (IOB_DATA::LVCMOS12, 4) => IOB_DATA::LVCMOS12_4,
        (IOB_DATA::LVCMOS12, 6) => IOB_DATA::LVCMOS12_6,
        (IOB_DATA::LVCMOS15, 2) => IOB_DATA::LVCMOS15_2,
        (IOB_DATA::LVCMOS15, 4) => IOB_DATA::LVCMOS15_4,
        (IOB_DATA::LVCMOS15, 6) => IOB_DATA::LVCMOS15_6,
        (IOB_DATA::LVCMOS15, 8) => IOB_DATA::LVCMOS15_8,
        (IOB_DATA::LVCMOS15, 12) => IOB_DATA::LVCMOS15_12,
        (IOB_DATA::LVCMOS15, 16) => IOB_DATA::LVCMOS15_16,
        (IOB_DATA::LVCMOS18, 2) => IOB_DATA::LVCMOS18_2,
        (IOB_DATA::LVCMOS18, 4) => IOB_DATA::LVCMOS18_4,
        (IOB_DATA::LVCMOS18, 6) => IOB_DATA::LVCMOS18_6,
        (IOB_DATA::LVCMOS18, 8) => IOB_DATA::LVCMOS18_8,
        (IOB_DATA::LVCMOS18, 12) => IOB_DATA::LVCMOS18_12,
        (IOB_DATA::LVCMOS18, 16) => IOB_DATA::LVCMOS18_16,
        (IOB_DATA::LVCMOS25, 2) => IOB_DATA::LVCMOS25_2,
        (IOB_DATA::LVCMOS25, 4) => IOB_DATA::LVCMOS25_4,
        (IOB_DATA::LVCMOS25, 6) => IOB_DATA::LVCMOS25_6,
        (IOB_DATA::LVCMOS25, 8) => IOB_DATA::LVCMOS25_8,
        (IOB_DATA::LVCMOS25, 12) => IOB_DATA::LVCMOS25_12,
        (IOB_DATA::LVCMOS25, 16) => IOB_DATA::LVCMOS25_16,
        (IOB_DATA::LVCMOS25, 24) => IOB_DATA::LVCMOS25_24,
        (IOB_DATA::LVCMOS33, 2) => IOB_DATA::LVCMOS33_2,
        (IOB_DATA::LVCMOS33, 4) => IOB_DATA::LVCMOS33_4,
        (IOB_DATA::LVCMOS33, 6) => IOB_DATA::LVCMOS33_6,
        (IOB_DATA::LVCMOS33, 8) => IOB_DATA::LVCMOS33_8,
        (IOB_DATA::LVCMOS33, 12) => IOB_DATA::LVCMOS33_12,
        (IOB_DATA::LVCMOS33, 16) => IOB_DATA::LVCMOS33_16,
        (IOB_DATA::LVCMOS33, 24) => IOB_DATA::LVCMOS33_24,
        (IOB_DATA::LVTTL, 2) => IOB_DATA::LVTTL_2,
        (IOB_DATA::LVTTL, 4) => IOB_DATA::LVTTL_4,
        (IOB_DATA::LVTTL, 6) => IOB_DATA::LVTTL_6,
        (IOB_DATA::LVTTL, 8) => IOB_DATA::LVTTL_8,
        (IOB_DATA::LVTTL, 12) => IOB_DATA::LVTTL_12,
        (IOB_DATA::LVTTL, 16) => IOB_DATA::LVTTL_16,
        (IOB_DATA::LVTTL, 24) => IOB_DATA::LVTTL_24,
        _ => unreachable!(),
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };
    let intdb = backend.edev.db;
    let package = backend
        .device
        .bonds
        .values()
        .max_by_key(|bond| {
            let bdata = &backend.db.bonds[bond.bond];
            let prjcombine_re_xilinx_geom::Bond::Virtex2(bdata) = bdata else {
                unreachable!();
            };
            bdata.pins.len()
        })
        .unwrap();

    // IOI
    for (tcid, _, tcls) in &intdb.tile_classes {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for bel in tcls.bels.ids() {
            let Some(idx) = bslots::IOI.index_of(bel) else {
                continue;
            };
            if edev.chip.kind.is_virtex2() && tcid == tcls_v2::IOI_CLK_N && matches!(idx, 0 | 1) {
                continue;
            }
            if edev.chip.kind.is_virtex2() && tcid == tcls_v2::IOI_CLK_S && matches!(idx, 2 | 3) {
                continue;
            }
            let mut bctx = ctx.bel(bel);
            let mode = if edev.chip.kind.is_spartan3ea() {
                "IBUF"
            } else {
                "IOB"
            };

            // clock & SR invs
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .test_bel_input_inv_auto(bcls::IOI::OTCLK1);
            bctx.mode(mode)
                .attr("OFF2", "#FF")
                .test_bel_input_inv_auto(bcls::IOI::OTCLK2);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .test_bel_input_inv_auto(bcls::IOI::ICLK1);
            bctx.mode(mode)
                .attr("IFF2", "#FF")
                .test_bel_input_inv_auto(bcls::IOI::ICLK2);
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OSR_USED", "0")
                .test_bel_input_inv_auto(bcls::IOI::SR);
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OREV_USED", "0")
                .test_bel_input_inv_auto(bcls::IOI::REV);
            // SR & rev enables
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("OFF1", "#FF")
                .attr("OSR_USED", "0")
                .attr("SRINV", "SR_B")
                .pin("SR")
                .test_bel_attr_bits(bcls::IOI::FFI_SR_ENABLE)
                .attr("ISR_USED", "0")
                .commit();
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("OFF1", "#FF")
                .attr("ISR_USED", "0")
                .attr("SRINV", "SR_B")
                .pin("SR")
                .test_bel_attr_bits(bcls::IOI::FFO_SR_ENABLE)
                .attr("OSR_USED", "0")
                .commit();
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("TFF1", "#FF")
                .attr("ISR_USED", "0")
                .attr("SRINV", "SR_B")
                .pin("SR")
                .test_bel_attr_bits(bcls::IOI::FFT_SR_ENABLE)
                .attr("TSR_USED", "0")
                .commit();
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("OFF1", "#FF")
                .attr("OREV_USED", "0")
                .attr("REVINV", "REV_B")
                .pin("REV")
                .test_bel_attr_bits(bcls::IOI::FFI_REV_ENABLE)
                .attr("IREV_USED", "0")
                .commit();
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("OFF1", "#FF")
                .attr("IREV_USED", "0")
                .attr("REVINV", "REV_B")
                .pin("REV")
                .test_bel_attr_bits(bcls::IOI::FFO_REV_ENABLE)
                .attr("OREV_USED", "0")
                .commit();
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("TFF1", "#FF")
                .attr("IREV_USED", "0")
                .attr("REVINV", "REV_B")
                .pin("REV")
                .test_bel_attr_bits(bcls::IOI::FFT_REV_ENABLE)
                .attr("TREV_USED", "0")
                .commit();

            // CE
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .test_bel_input_inv_auto(bcls::IOI::ICE);
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .test_bel_input_inv_auto(bcls::IOI::TCE);
            if edev.chip.kind.is_spartan3ea() {
                bctx.mode(mode)
                    .attr("OFF1", "#FF")
                    .attr("PCICE_MUX", "OCE")
                    .test_bel_input_inv_auto(bcls::IOI::OCE);
                for (val, vname) in [
                    (enums::IOI_MUX_OCE::OCE, "OCE"),
                    (enums::IOI_MUX_OCE::PCI_CE, "PCICE"),
                ] {
                    bctx.mode(mode)
                        .attr("OFF1", "#FF")
                        .attr("OCEINV", "#OFF")
                        .pin("OCE")
                        .pin("PCI_CE")
                        .test_bel_attr_val(bcls::IOI::MUX_OCE, val)
                        .attr("PCICE_MUX", vname)
                        .commit();
                }
            } else {
                bctx.mode(mode)
                    .attr("OFF1", "#FF")
                    .test_bel_input_inv_auto(bcls::IOI::OCE);
            }
            // Output path
            if edev.chip.kind.is_spartan3ea() {
                bctx.mode(mode)
                    .attr("O1_DDRMUX", "1")
                    .attr("OFF1", "#FF")
                    .attr("OMUX", "OFF1")
                    .test_bel_input_inv_auto(bcls::IOI::O1);
                bctx.mode(mode)
                    .attr("O2_DDRMUX", "1")
                    .attr("OFF2", "#FF")
                    .attr("OMUX", "OFF2")
                    .test_bel_input_inv_auto(bcls::IOI::O2);
            } else {
                bctx.mode(mode)
                    .attr("OFF1", "#FF")
                    .attr("OMUX", "OFF1")
                    .test_bel_input_inv_auto(bcls::IOI::O1);
                bctx.mode(mode)
                    .attr("OFF2", "#FF")
                    .attr("OMUX", "OFF2")
                    .test_bel_input_inv_auto(bcls::IOI::O2);
            }
            bctx.mode(mode)
                .attr("T_USED", "0")
                .attr("TFF1", "#FF")
                .attr("TFF2", "#OFF")
                .attr("TMUX", "TFF1")
                .attr("OFF1", "#OFF")
                .attr("OFF2", "#OFF")
                .attr("OMUX", "#OFF")
                .pin("T")
                .test_bel_input_inv_auto(bcls::IOI::T1);
            bctx.mode(mode)
                .attr("T_USED", "0")
                .attr("TFF1", "#OFF")
                .attr("TFF2", "#FF")
                .attr("TMUX", "TFF2")
                .attr("OFF1", "#OFF")
                .attr("OFF2", "#OFF")
                .attr("OMUX", "#OFF")
                .pin("T")
                .test_bel_input_inv_auto(bcls::IOI::T2);
            for (val, vname) in [
                (enums::IOI_MUX_T::T1, "T1"),
                (enums::IOI_MUX_T::T2, "T2"),
                (enums::IOI_MUX_T::FFT1, "TFF1"),
                (enums::IOI_MUX_T::FFT2, "TFF2"),
                (enums::IOI_MUX_T::FFTDDR, "TFFDDR"),
            ] {
                bctx.mode(mode)
                    .attr("T1INV", "T1")
                    .attr("T2INV", "T2")
                    .attr("TFF1", "#FF")
                    .attr("TFF2", "#FF")
                    .attr("T_USED", "0")
                    .attr("OMUX", "#OFF")
                    .attr("IOATTRBOX", "#OFF")
                    .pin("T1")
                    .pin("T2")
                    .pin("T")
                    .test_bel_attr_val(bcls::IOI::MUX_T, val)
                    .attr("TMUX", vname)
                    .commit();
            }
            // hack to avoid dragging IOB into it.
            for (val, vname) in [
                (enums::IOI_MUX_O::O1, "O1"),
                (enums::IOI_MUX_O::O2, "O2"),
                (enums::IOI_MUX_O::FFO1, "OFF1"),
                (enums::IOI_MUX_O::FFO2, "OFF2"),
                (enums::IOI_MUX_O::FFODDR, "OFFDDR"),
            ] {
                if !edev.chip.kind.is_spartan3ea() {
                    bctx.mode(mode)
                        .attr("O1INV", "O1")
                        .attr("O2INV", "O2")
                        .attr("OFF1", "#FF")
                        .attr("OFF2", "#FF")
                        .attr("IMUX", "0")
                        .attr("TSMUX", "1")
                        .attr("TMUX", "T1")
                        .attr("T1INV", "T1")
                        .attr("T_USED", "0")
                        .attr("IFF1", "#FF")
                        .attr("IFFDMUX", "1")
                        .attr("IFFDELMUX", "1")
                        .pin("O1")
                        .pin("O2")
                        .pin("T1")
                        .pin("T")
                        .pin("I")
                        .test_bel_attr_val(bcls::IOI::MUX_O, val)
                        .attr_diff("OMUX", "OFFDDR", vname)
                        .commit();
                } else if edev.chip.kind == ChipKind::Spartan3E {
                    bctx.mode(mode)
                        .attr("O1INV", "O1")
                        .attr("O2INV", "O2")
                        .attr("OFF1", "#FF")
                        .attr("OFF2", "#FF")
                        .attr("IMUX", "0")
                        .attr("TSMUX", "1")
                        .attr("TMUX", "T1")
                        .attr("T1INV", "T1")
                        .attr("T_USED", "0")
                        .attr("IFF1", "#FF")
                        .attr("IFFDMUX", "1")
                        .attr("IFFDELMUX", "1")
                        .attr("O1_DDRMUX", "1")
                        .attr("O2_DDRMUX", "1")
                        .attr("IDDRIN_MUX", "2")
                        .pin("O1")
                        .pin("O2")
                        .pin("T1")
                        .pin("T")
                        .pin("I")
                        .test_bel_attr_val(bcls::IOI::MUX_O, val)
                        .attr_diff("OMUX", "OFFDDR", vname)
                        .commit();
                } else {
                    bctx.mode(mode)
                        .attr("O1INV", "O1")
                        .attr("O2INV", "O2")
                        .attr("OFF1", "#FF")
                        .attr("OFF2", "#FF")
                        .attr("IMUX", "0")
                        .attr("TSMUX", "1")
                        .attr("TMUX", "T1")
                        .attr("T1INV", "T1")
                        .attr("T_USED", "0")
                        .attr("IFF1", "#FF")
                        .attr("IFFDMUX", "1")
                        .attr("O1_DDRMUX", "1")
                        .attr("O2_DDRMUX", "1")
                        .attr("IDDRIN_MUX", "2")
                        .attr("SEL_MUX", "0")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .pin("O1")
                        .pin("O2")
                        .pin("T1")
                        .pin("T")
                        .pin("I")
                        .test_bel_attr_val(bcls::IOI::MUX_O, val)
                        .attr_diff("OMUX", "OFFDDR", vname)
                        .commit();
                }
            }
            if idx != 2 && edev.chip.kind.is_spartan3ea() {
                let obel = bslots::IOI[idx ^ 1];
                for (val, vname) in [
                    (enums::IOI_MUX_FFO1::O1, "1"),
                    (enums::IOI_MUX_FFO1::PAIR_FFO2, "0"),
                ] {
                    if edev.chip.kind == ChipKind::Spartan3E {
                        bctx.mode(mode)
                            .bel_unused(obel)
                            .attr("OFF1", "#FF")
                            .attr("OFF2", "#FF")
                            .attr("OMUX", "OFFDDR")
                            .attr("TSMUX", "1")
                            .attr("TFF1", "#FF")
                            .attr("IFF1", "#FF")
                            .attr("TMUX", "TFF1")
                            .attr("IMUX", "0")
                            .attr("O1INV", "#OFF")
                            .pin("ODDRIN1")
                            .pin("I")
                            .test_bel_attr_val(bcls::IOI::MUX_FFO1, val)
                            .attr("O1_DDRMUX", vname)
                            .commit();
                    } else {
                        bctx.mode(mode)
                            .bel_unused(obel)
                            .attr("OFF1", "#FF")
                            .attr("OFF2", "#FF")
                            .attr("OMUX", "OFFDDR")
                            .attr("TSMUX", "1")
                            .attr("TFF1", "#FF")
                            .attr("IFF1", "#FF")
                            .attr("TMUX", "TFF1")
                            .attr("IMUX", "0")
                            .attr("O1INV", "#OFF")
                            .attr("SEL_MUX", "0")
                            .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                            .pin("ODDRIN1")
                            .pin("I")
                            .test_bel_attr_val(bcls::IOI::MUX_FFO1, val)
                            .attr("O1_DDRMUX", vname)
                            .commit();
                    }
                }
                for (val, vname) in [
                    (enums::IOI_MUX_FFO2::O2, "1"),
                    (enums::IOI_MUX_FFO2::PAIR_FFO1, "0"),
                ] {
                    if edev.chip.kind == ChipKind::Spartan3E {
                        bctx.mode(mode)
                            .bel_unused(obel)
                            .attr("OFF1", "#FF")
                            .attr("OFF2", "#FF")
                            .attr("OMUX", "OFFDDR")
                            .attr("TSMUX", "1")
                            .attr("TFF1", "#FF")
                            .attr("IFF1", "#FF")
                            .attr("TMUX", "TFF1")
                            .attr("IMUX", "0")
                            .attr("O2INV", "#OFF")
                            .pin("ODDRIN2")
                            .pin("I")
                            .test_bel_attr_val(bcls::IOI::MUX_FFO2, val)
                            .attr("O2_DDRMUX", vname)
                            .commit();
                    } else {
                        bctx.mode(mode)
                            .bel_unused(obel)
                            .attr("OFF1", "#FF")
                            .attr("OFF2", "#FF")
                            .attr("OMUX", "OFFDDR")
                            .attr("TSMUX", "1")
                            .attr("TFF1", "#FF")
                            .attr("IFF1", "#FF")
                            .attr("TMUX", "TFF1")
                            .attr("IMUX", "0")
                            .attr("O2INV", "#OFF")
                            .attr("SEL_MUX", "0")
                            .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                            .pin("ODDRIN2")
                            .pin("I")
                            .test_bel_attr_val(bcls::IOI::MUX_FFO2, val)
                            .attr("O2_DDRMUX", vname)
                            .commit();
                    }
                }
            }

            // Output flops
            if !edev.chip.kind.is_spartan3ea() {
                bctx.mode(mode)
                    .attr("OFF2", "#OFF")
                    .attr("OCEINV", "OCE_B")
                    .attr("OFF1_INIT_ATTR", "INIT1")
                    .pin("OCE")
                    .test_bel_attr_bool_rename("OFF1", bcls::IOI::FFO1_LATCH, "#FF", "#LATCH");
                bctx.mode(mode)
                    .attr("OFF1", "#OFF")
                    .attr("OCEINV", "OCE_B")
                    .attr("OFF2_INIT_ATTR", "INIT1")
                    .pin("OCE")
                    .test_bel_attr_bool_rename("OFF2", bcls::IOI::FFO2_LATCH, "#FF", "#LATCH");
            } else {
                bctx.mode(mode)
                    .attr("OFF2", "#OFF")
                    .attr("OCEINV", "OCE_B")
                    .attr("PCICE_MUX", "OCE")
                    .attr("OFF1_INIT_ATTR", "INIT1")
                    .pin("OCE")
                    .test_bel_attr_bool_rename("OFF1", bcls::IOI::FFO1_LATCH, "#FF", "#LATCH");
                bctx.mode(mode)
                    .attr("OFF1", "#OFF")
                    .attr("OCEINV", "OCE_B")
                    .attr("PCICE_MUX", "OCE")
                    .attr("OFF2_INIT_ATTR", "INIT1")
                    .pin("OCE")
                    .test_bel_attr_bool_rename("OFF2", bcls::IOI::FFO2_LATCH, "#FF", "#LATCH");
            }
            bctx.mode(mode)
                .attr("TFF2", "#OFF")
                .attr("TCEINV", "TCE_B")
                .attr("TFF1_INIT_ATTR", "INIT1")
                .pin("TCE")
                .test_bel_attr_bool_rename("TFF1", bcls::IOI::FFT1_LATCH, "#FF", "#LATCH");
            bctx.mode(mode)
                .attr("TFF1", "#OFF")
                .attr("TCEINV", "TCE_B")
                .attr("TFF2_INIT_ATTR", "INIT1")
                .pin("TCE")
                .test_bel_attr_bool_rename("TFF2", bcls::IOI::FFT2_LATCH, "#FF", "#LATCH");
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OFF1_INIT_ATTR", "INIT0")
                .test_bel_attr_bool_rename(
                    "OFF1_SR_ATTR",
                    bcls::IOI::FFO1_SRVAL,
                    "SRLOW",
                    "SRHIGH",
                );
            bctx.mode(mode)
                .attr("OFF2", "#FF")
                .attr("OFF2_INIT_ATTR", "INIT0")
                .test_bel_attr_bool_rename(
                    "OFF2_SR_ATTR",
                    bcls::IOI::FFO2_SRVAL,
                    "SRLOW",
                    "SRHIGH",
                );
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .attr("TFF1_INIT_ATTR", "INIT0")
                .test_bel_attr_bool_rename(
                    "TFF1_SR_ATTR",
                    bcls::IOI::FFT1_SRVAL,
                    "SRLOW",
                    "SRHIGH",
                );
            bctx.mode(mode)
                .attr("TFF2", "#FF")
                .attr("TFF2_INIT_ATTR", "INIT0")
                .test_bel_attr_bool_rename(
                    "TFF2_SR_ATTR",
                    bcls::IOI::FFT2_SRVAL,
                    "SRLOW",
                    "SRHIGH",
                );
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OFF2", "#FF")
                .attr("OFF1_SR_ATTR", "SRHIGH")
                .attr("OFF2_SR_ATTR", "SRHIGH")
                .attr("OFF2_INIT_ATTR", "#OFF")
                .test_bel_attr_bool_rename("OFF1_INIT_ATTR", bcls::IOI::FFO_INIT, "INIT0", "INIT1");
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OFF2", "#FF")
                .attr("OFF1_SR_ATTR", "SRHIGH")
                .attr("OFF2_SR_ATTR", "SRHIGH")
                .attr("OFF1_INIT_ATTR", "#OFF")
                .test_bel_attr_bool_rename("OFF2_INIT_ATTR", bcls::IOI::FFO_INIT, "INIT0", "INIT1");
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .attr("TFF2", "#FF")
                .attr("TFF1_SR_ATTR", "SRHIGH")
                .attr("TFF2_SR_ATTR", "SRHIGH")
                .attr("TFF2_INIT_ATTR", "#OFF")
                .test_bel_attr_bool_rename("TFF1_INIT_ATTR", bcls::IOI::FFT_INIT, "INIT0", "INIT1");
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .attr("TFF2", "#FF")
                .attr("TFF1_SR_ATTR", "SRHIGH")
                .attr("TFF2_SR_ATTR", "SRHIGH")
                .attr("TFF1_INIT_ATTR", "#OFF")
                .test_bel_attr_bool_rename("TFF2_INIT_ATTR", bcls::IOI::FFT_INIT, "INIT0", "INIT1");
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OFF2", "#FF")
                .test_bel_attr_bool_rename("OFFATTRBOX", bcls::IOI::FFO_SR_SYNC, "ASYNC", "SYNC");
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .attr("TFF2", "#FF")
                .test_bel_attr_bool_rename("TFFATTRBOX", bcls::IOI::FFT_SR_SYNC, "ASYNC", "SYNC");

            // Input flops
            bctx.mode(mode)
                .attr("IFF2", "#OFF")
                .attr("ICEINV", "ICE_B")
                .attr("IFF1_INIT_ATTR", "INIT1")
                .pin("ICE")
                .test_bel_attr_bool_rename("IFF1", bcls::IOI::FFI_LATCH, "#FF", "#LATCH");
            bctx.mode(mode)
                .attr("IFF1", "#OFF")
                .attr("ICEINV", "ICE_B")
                .attr("IFF2_INIT_ATTR", "INIT1")
                .pin("ICE")
                .test_bel_attr_bool_rename("IFF2", bcls::IOI::FFI_LATCH, "#FF", "#LATCH");
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("IFF1_INIT_ATTR", "INIT0")
                .test_bel_attr_bool_rename(
                    "IFF1_SR_ATTR",
                    bcls::IOI::FFI1_SRVAL,
                    "SRLOW",
                    "SRHIGH",
                );
            bctx.mode(mode)
                .attr("IFF2", "#FF")
                .attr("IFF2_INIT_ATTR", "INIT0")
                .test_bel_attr_bool_rename(
                    "IFF2_SR_ATTR",
                    bcls::IOI::FFI2_SRVAL,
                    "SRLOW",
                    "SRHIGH",
                );
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("IFF1_SR_ATTR", "SRHIGH")
                .test_bel_attr_bool_rename(
                    "IFF1_INIT_ATTR",
                    bcls::IOI::FFI1_INIT,
                    "INIT0",
                    "INIT1",
                );
            bctx.mode(mode)
                .attr("IFF2", "#FF")
                .attr("IFF2_SR_ATTR", "SRHIGH")
                .test_bel_attr_bool_rename(
                    "IFF2_INIT_ATTR",
                    bcls::IOI::FFI2_INIT,
                    "INIT0",
                    "INIT1",
                );
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("IFF2", "#FF")
                .test_bel_attr_bool_rename("IFFATTRBOX", bcls::IOI::FFI_SR_SYNC, "ASYNC", "SYNC");

            // Input path.
            if edev.chip.kind == ChipKind::Spartan3E {
                for (val, vname) in [
                    (enums::IOI_MUX_FFI::IBUF, "2"),
                    (enums::IOI_MUX_FFI::PAIR_IQ1, "1"),
                    (enums::IOI_MUX_FFI::PAIR_IQ2, "0"),
                ] {
                    bctx.mode(mode)
                        .attr("IFF1", "#FF")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "#OFF")
                        .pin("IDDRIN1")
                        .pin("IDDRIN2")
                        .pin("I")
                        .test_bel_attr_val(bcls::IOI::MUX_FFI, val)
                        .attr("IDDRIN_MUX", vname)
                        .commit();
                }
            } else if edev.chip.kind.is_spartan3a() {
                for (val, vname) in [
                    (enums::IOI_MUX_FFI::PAIR_IQ1, "1"),
                    (enums::IOI_MUX_FFI::PAIR_IQ2, "0"),
                ] {
                    bctx.mode(mode)
                        .attr("IFF1", "#FF")
                        .attr("IMUX", "1")
                        .attr("SEL_MUX", "0")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .pin("IDDRIN1")
                        .pin("IDDRIN2")
                        .pin("I")
                        .test_bel_attr_val(bcls::IOI::MUX_FFI, val)
                        .attr("IDDRIN_MUX", vname)
                        .commit();
                }
                bctx.mode(mode)
                    .attr("IFF1", "#FF")
                    .attr("IMUX", "1")
                    .attr("SEL_MUX", "0")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .pin("IDDRIN1")
                    .pin("IDDRIN2")
                    .pin("I")
                    .test_bel_attr_val(bcls::IOI::MUX_FFI, enums::IOI_MUX_FFI::IBUF)
                    .attr("IDDRIN_MUX", "2")
                    .attr("IFFDMUX", "1")
                    .commit();
            }

            if !edev.chip.kind.is_spartan3a() {
                if edev.chip.kind != ChipKind::Spartan3E {
                    bctx.mode(mode)
                        .attr("IFFDELMUX", "0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .pin("I")
                        .test_bel_attr_bool_rename("IDELMUX", bcls::IOI::I_DELAY_ENABLE, "1", "0");
                    bctx.mode(mode)
                        .attr("IDELMUX", "0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .pin("I")
                        .test_bel_attr_bool_rename(
                            "IFFDELMUX",
                            bcls::IOI::IQ_DELAY_ENABLE,
                            "1",
                            "0",
                        );
                    bctx.mode(mode)
                        .attr("TSMUX", "1")
                        .attr("IDELMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IFFDMUX", "0")
                        .attr("IFFDELMUX", "1")
                        .attr("O1INV", "O1")
                        .attr("OMUX", "O1")
                        .attr("T1INV", "T1")
                        .attr("TMUX", "T1")
                        .attr("T_USED", "0")
                        .pin("O1")
                        .pin("T1")
                        .pin("I")
                        .test_bel_attr_bool_rename("IMUX", bcls::IOI::I_TSBYPASS_ENABLE, "1", "0");
                    bctx.mode(mode)
                        .attr("TSMUX", "1")
                        .attr("IDELMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IMUX", "0")
                        .attr("IFFDELMUX", "1")
                        .attr("O1INV", "O1")
                        .attr("OMUX", "O1")
                        .attr("T1INV", "T1")
                        .attr("TMUX", "T1")
                        .attr("T_USED", "0")
                        .pin("O1")
                        .pin("T1")
                        .pin("I")
                        .test_bel_attr_bool_rename(
                            "IFFDMUX",
                            bcls::IOI::IQ_TSBYPASS_ENABLE,
                            "1",
                            "0",
                        );
                } else {
                    bctx.mode(mode)
                        .attr("IFFDELMUX", "0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("IBUF_DELAY_VALUE", "DLY4")
                        .attr("PRE_DELAY_MUX", "0")
                        .pin("I")
                        .test_bel_attr_bool_rename("IDELMUX", bcls::IOI::I_DELAY_ENABLE, "1", "0");
                    bctx.mode(mode)
                        .attr("IDELMUX", "0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("IBUF_DELAY_VALUE", "DLY4")
                        .attr("PRE_DELAY_MUX", "0")
                        .pin("I")
                        .test_bel_attr_bool_rename(
                            "IFFDELMUX",
                            bcls::IOI::IQ_DELAY_ENABLE,
                            "1",
                            "0",
                        );
                    bctx.mode(mode)
                        .attr("TSMUX", "1")
                        .attr("IDELMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IFFDMUX", "0")
                        .attr("IFFDELMUX", "1")
                        .attr("O1INV", "O1")
                        .attr("OMUX", "O1")
                        .attr("T1INV", "T1")
                        .attr("TMUX", "T1")
                        .attr("T_USED", "0")
                        .attr("IDDRIN_MUX", "2")
                        .pin("O1")
                        .pin("T1")
                        .pin("I")
                        .test_bel_attr_bool_rename("IMUX", bcls::IOI::I_TSBYPASS_ENABLE, "1", "0");
                    bctx.mode(mode)
                        .attr("TSMUX", "1")
                        .attr("IDELMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IMUX", "0")
                        .attr("IFFDELMUX", "1")
                        .attr("O1INV", "O1")
                        .attr("OMUX", "O1")
                        .attr("T1INV", "T1")
                        .attr("TMUX", "T1")
                        .attr("T_USED", "0")
                        .attr("IDDRIN_MUX", "2")
                        .pin("O1")
                        .pin("T1")
                        .pin("I")
                        .test_bel_attr_bool_rename(
                            "IFFDMUX",
                            bcls::IOI::IQ_TSBYPASS_ENABLE,
                            "1",
                            "0",
                        );
                }
                for (val, vname) in [
                    (enums::IOI_MUX_TSBYPASS::GND, "0"),
                    (enums::IOI_MUX_TSBYPASS::T, "1"),
                ] {
                    bctx.mode(mode)
                        .attr("IFFDMUX", "1")
                        .attr("TMUX", "T1")
                        .attr("T1INV", "T1")
                        .attr("OMUX", "O1")
                        .attr("O1INV", "O1")
                        .attr("IFF1", "#FF")
                        .attr("IMUX", "0")
                        .attr("T_USED", "0")
                        .pin("T1")
                        .pin("O1")
                        .pin("I")
                        .pin("T")
                        .test_bel_attr_val(bcls::IOI::MUX_TSBYPASS, val)
                        .attr("TSMUX", vname)
                        .commit();
                }
            } else {
                bctx.mode(mode)
                    .attr("IFD_DELAY_VALUE", "DLY0")
                    .attr("IMUX", "1")
                    .attr("IFFDMUX", "1")
                    .attr("IFF1", "#FF")
                    .attr("IDDRIN_MUX", "2")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .attr("SEL_MUX", "0")
                    .pin("I")
                    .test_bel_attr_bool_rename(
                        "IBUF_DELAY_VALUE",
                        bcls::IOI::I_DELAY_ENABLE,
                        "DLY0",
                        "DLY16",
                    );
                bctx.mode(mode)
                    .attr("IBUF_DELAY_VALUE", "DLY0")
                    .attr("IMUX", "1")
                    .attr("IFFDMUX", "1")
                    .attr("IFF1", "#FF")
                    .attr("IDDRIN_MUX", "2")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .attr("SEL_MUX", "0")
                    .pin("I")
                    .test_bel_attr_bool_rename(
                        "IFD_DELAY_VALUE",
                        bcls::IOI::IQ_DELAY_ENABLE,
                        "DLY0",
                        "DLY8",
                    );
                if tcid == tcls_s3::IOI_S3A_WE {
                    for i in 0..16 {
                        bctx.mode(mode)
                            .attr("IFD_DELAY_VALUE", "DLY0")
                            .attr("IMUX", "1")
                            .attr("IFFDMUX", "1")
                            .attr("IFF1", "#FF")
                            .attr("IDDRIN_MUX", "2")
                            .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                            .attr("SEL_MUX", "0")
                            .pin("I")
                            .test_bel_attr_bitvec_u32_width(bcls::IOI::I_DELAY, i, 4)
                            .attr_diff("IBUF_DELAY_VALUE", "DLY1", format!("DLY{ii}", ii = i + 1))
                            .commit();
                    }
                    for i in 0..8 {
                        bctx.mode(mode)
                            .attr("IBUF_DELAY_VALUE", "DLY0")
                            .attr("IMUX", "1")
                            .attr("IFFDMUX", "1")
                            .attr("IFF1", "#FF")
                            .attr("IDDRIN_MUX", "2")
                            .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                            .attr("SEL_MUX", "0")
                            .pin("I")
                            .test_bel_attr_bitvec_u32_width(bcls::IOI::IQ_DELAY, i, 3)
                            .attr_diff("IFD_DELAY_VALUE", "DLY1", format!("DLY{ii}", ii = i + 1))
                            .commit();
                    }
                    bctx.mode(mode)
                        .attr("IBUF_DELAY_VALUE", "DLY16")
                        .attr("IFD_DELAY_VALUE", "DLY8")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("SEL_MUX", "0")
                        .pin("I")
                        .test_bel_attr_bits(bcls::IOI::DELAY_VARIABLE)
                        .attr_diff("DELAY_ADJ_ATTRBOX", "FIXED", "VARIABLE")
                        .commit();
                }
                bctx.mode(mode)
                    .attr("TSMUX", "1")
                    .attr("IFF1", "#FF")
                    .attr("IFFDMUX", "0")
                    .attr("O1INV", "O1")
                    .attr("OMUX", "O1")
                    .attr("T1INV", "T1")
                    .attr("TMUX", "T1")
                    .attr("T_USED", "0")
                    .attr("IDDRIN_MUX", "2")
                    .attr("SEL_MUX", "0")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .pin("O1")
                    .pin("T1")
                    .pin("I")
                    .test_bel_attr_bool_rename("IMUX", bcls::IOI::I_TSBYPASS_ENABLE, "1", "0");
                bctx.mode(mode)
                    .attr("TSMUX", "1")
                    .attr("IFF1", "#FF")
                    .attr("IMUX", "0")
                    .attr("O1INV", "O1")
                    .attr("OMUX", "O1")
                    .attr("T1INV", "T1")
                    .attr("TMUX", "T1")
                    .attr("T_USED", "0")
                    .attr("IDDRIN_MUX", "2")
                    .attr("SEL_MUX", "0")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .pin("O1")
                    .pin("T1")
                    .pin("I")
                    .test_bel_attr_bool_rename("IFFDMUX", bcls::IOI::IQ_TSBYPASS_ENABLE, "1", "0");
                for (val, vname) in [
                    (enums::IOI_MUX_TSBYPASS::GND, "0"),
                    (enums::IOI_MUX_TSBYPASS::T, "1"),
                ] {
                    bctx.mode(mode)
                        .attr("IFFDMUX", "1")
                        .attr("TMUX", "T1")
                        .attr("T1INV", "T1")
                        .attr("OMUX", "O1")
                        .attr("O1INV", "O1")
                        .attr("IFF1", "#FF")
                        .attr("IMUX", "0")
                        .attr("T_USED", "0")
                        .attr("SEL_MUX", "0")
                        .pin("T1")
                        .pin("O1")
                        .pin("I")
                        .pin("T")
                        .test_bel_attr_val(bcls::IOI::MUX_TSBYPASS, val)
                        .attr("TSMUX", vname)
                        .commit();
                }
            }
            if edev.chip.kind.is_spartan3ea() {
                bctx.mode("IOB")
                    .prop(NotIbuf)
                    .global("ENABLEMISR", "Y")
                    .global("MISRRESET", "N")
                    .no_global("MISRCLOCK")
                    .attr("PULL", "PULLDOWN")
                    .attr("TMUX", "#OFF")
                    .attr("IMUX", "#OFF")
                    .attr("IFFDMUX", "#OFF")
                    .attr("OMUX", "O1")
                    .attr("O1INV", "O1")
                    .attr("IOATTRBOX", "LVCMOS33")
                    .attr("DRIVE_0MA", "DRIVE_0MA")
                    .pin("O1")
                    .test_bel_attr_bits(bcls::IOI::MISR_ENABLE)
                    .attr("MISRATTRBOX", "ENABLE_MISR")
                    .commit();
                if edev.chip.kind.is_spartan3a() {
                    bctx.mode("IOB")
                        .prop(NotIbuf)
                        .global("ENABLEMISR", "Y")
                        .global("MISRRESET", "N")
                        .attr("PULL", "PULLDOWN")
                        .attr("TMUX", "#OFF")
                        .attr("IMUX", "#OFF")
                        .attr("IFFDMUX", "#OFF")
                        .attr("OMUX", "O1")
                        .attr("O1INV", "O1")
                        .attr("IOATTRBOX", "LVCMOS33")
                        .attr("DRIVE_0MA", "DRIVE_0MA")
                        .attr("MISRATTRBOX", "ENABLE_MISR")
                        .pin("O1")
                        .test_bel_attr_val(
                            bcls::IOI::MUX_MISR_CLOCK,
                            enums::IOI_MUX_MISR_CLOCK::OTCLK1,
                        )
                        .attr("MISR_CLK_SELECT", "OTCLK1")
                        .commit();
                    bctx.mode("IOB")
                        .prop(NotIbuf)
                        .global("ENABLEMISR", "Y")
                        .global("MISRRESET", "N")
                        .attr("PULL", "PULLDOWN")
                        .attr("TMUX", "#OFF")
                        .attr("IMUX", "#OFF")
                        .attr("IFFDMUX", "#OFF")
                        .attr("OMUX", "O1")
                        .attr("O1INV", "O1")
                        .attr("IOATTRBOX", "LVCMOS33")
                        .attr("DRIVE_0MA", "DRIVE_0MA")
                        .attr("MISRATTRBOX", "ENABLE_MISR")
                        .pin("O1")
                        .test_bel_attr_val(
                            bcls::IOI::MUX_MISR_CLOCK,
                            enums::IOI_MUX_MISR_CLOCK::OTCLK2,
                        )
                        .attr("MISR_CLK_SELECT", "OTCLK2")
                        .commit();
                } else {
                    bctx.mode("IOB")
                        .prop(NotIbuf)
                        .global("ENABLEMISR", "Y")
                        .global("MISRRESET", "Y")
                        .no_global("MISRCLOCK")
                        .attr("PULL", "PULLDOWN")
                        .attr("TMUX", "#OFF")
                        .attr("IMUX", "#OFF")
                        .attr("IFFDMUX", "#OFF")
                        .attr("OMUX", "O1")
                        .attr("O1INV", "O1")
                        .attr("IOATTRBOX", "LVCMOS33")
                        .attr("DRIVE_0MA", "DRIVE_0MA")
                        .pin("O1")
                        .test_bel_attr_bits(bcls::IOI::MISR_RESET)
                        .attr("MISRATTRBOX", "ENABLE_MISR")
                        .commit();
                    bctx.mode("IOB")
                        .prop(NotIbuf)
                        .global("ENABLEMISR", "Y")
                        .global("MISRRESET", "N")
                        .global("MISRCLOCK", "OTCLK1")
                        .attr("PULL", "PULLDOWN")
                        .attr("TMUX", "#OFF")
                        .attr("IMUX", "#OFF")
                        .attr("IFFDMUX", "#OFF")
                        .attr("OMUX", "O1")
                        .attr("O1INV", "O1")
                        .attr("IOATTRBOX", "LVCMOS33")
                        .attr("DRIVE_0MA", "DRIVE_0MA")
                        .pin("O1")
                        .test_bel_attr_val(
                            bcls::IOI::MUX_MISR_CLOCK,
                            enums::IOI_MUX_MISR_CLOCK::OTCLK1,
                        )
                        .attr("MISRATTRBOX", "ENABLE_MISR")
                        .commit();
                    bctx.mode("IOB")
                        .prop(NotIbuf)
                        .global("ENABLEMISR", "Y")
                        .global("MISRRESET", "N")
                        .global("MISRCLOCK", "OTCLK2")
                        .attr("PULL", "PULLDOWN")
                        .attr("TMUX", "#OFF")
                        .attr("IMUX", "#OFF")
                        .attr("IFFDMUX", "#OFF")
                        .attr("OMUX", "O1")
                        .attr("O1INV", "O1")
                        .attr("IOATTRBOX", "LVCMOS33")
                        .attr("DRIVE_0MA", "DRIVE_0MA")
                        .pin("O1")
                        .test_bel_attr_val(
                            bcls::IOI::MUX_MISR_CLOCK,
                            enums::IOI_MUX_MISR_CLOCK::OTCLK2,
                        )
                        .attr("MISRATTRBOX", "ENABLE_MISR")
                        .commit();
                }
            }
        }
    }

    // IOB
    for iob_data in get_iob_tiles(edev.chip.kind) {
        let tcid = iob_data.tcid;
        let tile = edev.db.tile_classes.key(tcid);
        let is_s3a_we = !edev.chip.kind.is_virtex2()
            && (matches!(tcid, tcls_s3::IOB_S3A_W4 | tcls_s3::IOB_S3A_E4));
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for &iob in &iob_data.iobs {
            let ibuf_mode = if edev.chip.kind.is_spartan3ea() {
                "IBUF"
            } else {
                "IOB"
            };
            let diffi_mode = match iob.diff {
                IobDiff::None => None,
                IobDiff::True(_) => Some(if edev.chip.kind.is_spartan3a() && is_s3a_we {
                    "DIFFMI_NDT"
                } else if edev.chip.kind.is_spartan3ea() {
                    "DIFFMI"
                } else {
                    "DIFFM"
                }),
                IobDiff::Comp(_) => Some(if edev.chip.kind.is_spartan3a() && is_s3a_we {
                    "DIFFSI_NDT"
                } else if edev.chip.kind.is_spartan3ea() {
                    "DIFFSI"
                } else {
                    "DIFFS"
                }),
            };
            if iob.kind == IobKind::Clk {
                continue;
            }
            let iob_mode = if iob.kind == IobKind::Ibuf {
                "IBUF"
            } else {
                "IOB"
            };
            let mut bctx = ctx.bel(iob.ioi);
            if iob.kind != IobKind::Ibuf {
                bctx.build()
                    .global_mutex("VREF", "NO")
                    .global_mutex("DCI", "NO")
                    .test_bel_special(specials::IOB_MODE_IOB)
                    .mode("IOB")
                    .iob_commit(iob);
            }
            if edev.chip.kind.is_spartan3ea() {
                bctx.build()
                    .global_mutex("VREF", "NO")
                    .test_bel_special(specials::IOB_MODE_IBUF)
                    .mode("IBUF")
                    .iob_commit(iob);
            }
            for (val, vname) in [
                (enums::IOB_PULL::PULLUP, "PULLUP"),
                (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
                (enums::IOB_PULL::KEEPER, "KEEPER"),
            ] {
                bctx.mode(ibuf_mode)
                    .attr("IMUX", "1")
                    .pin("I")
                    .test_bel_attr_val(bcls::IOB::PULL, val)
                    .attr("PULL", vname)
                    .iob_commit(iob);
            }
            bctx.mode(ibuf_mode)
                .test_bel_attr_bits(bcls::IOB::DISABLE_GTS)
                .attr("GTSATTRBOX", "DISABLE_GTS")
                .iob_commit(iob);
            if edev.chip.kind.is_spartan3a() && iob.kind != IobKind::Ibuf {
                for (val, vname) in &backend.edev.db[enums::IOB_SUSPEND].values {
                    let vname = vname.strip_prefix('_').unwrap_or(vname);
                    bctx.mode("IOB")
                        .test_bel_attr_val(bcls::IOB::SUSPEND, val)
                        .attr("SUSPEND", vname)
                        .iob_commit(iob);
                }
            }
            if edev.chip.kind == ChipKind::Spartan3E {
                for i in 1..=16 {
                    bctx.mode("IBUF")
                        .attr("IFD_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .pin("I")
                        .test_bel_attr_u32(bcls::IOB::I_DELAY, i)
                        .attr_diff("IBUF_DELAY_VALUE", "DLY0", format!("DLY{i}"))
                        .iob_commit(iob);
                }
                for i in 1..=8 {
                    bctx.mode("IBUF")
                        .attr("IBUF_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .pin("I")
                        .test_bel_attr_u32(bcls::IOB::IQ_DELAY, i)
                        .attr_diff("IFD_DELAY_VALUE", "DLY0", format!("DLY{i}"))
                        .iob_commit(iob);
                }
            }
            if edev.chip.kind.is_spartan3a() && !is_s3a_we {
                for i in 0..16 {
                    bctx.mode("IBUF")
                        .attr("IFD_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .attr("SEL_MUX", "0")
                        .pin("I")
                        .test_bel_attr_bitvec_u32_width(bcls::IOB::I_DELAY, i, 4)
                        .attr_diff("IBUF_DELAY_VALUE", "DLY1", format!("DLY{ii}", ii = i + 1))
                        .iob_commit(iob);
                }
                for i in 0..8 {
                    bctx.mode("IBUF")
                        .attr("IBUF_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .attr("SEL_MUX", "0")
                        .pin("I")
                        .test_bel_attr_bitvec_u32_width(bcls::IOB::IQ_DELAY, i, 3)
                        .attr_diff("IFD_DELAY_VALUE", "DLY1", format!("DLY{ii}", ii = i + 1))
                        .iob_commit(iob);
                }
                bctx.mode("IBUF")
                    .attr("IBUF_DELAY_VALUE", "DLY16")
                    .attr("IFD_DELAY_VALUE", "DLY8")
                    .attr("IMUX", "1")
                    .attr("IFFDMUX", "1")
                    .attr("IFF1", "#FF")
                    .attr("IDDRIN_MUX", "2")
                    .attr("SEL_MUX", "0")
                    .pin("I")
                    .test_bel_attr_bits(bcls::IOB::DELAY_VARIABLE)
                    .attr_diff("DELAY_ADJ_ATTRBOX", "FIXED", "VARIABLE")
                    .iob_commit(iob);
            }

            // Input path.
            for std in get_iostds(edev, is_s3a_we) {
                let rid = iostd_to_row(edev, &std);
                let vccaux_list = if (std.name.starts_with("LVCMOS")
                    || std.name.starts_with("LVTTL"))
                    && edev.chip.kind.is_spartan3a()
                {
                    &["2.5", "3.3"][..]
                } else {
                    &[""][..]
                };
                if std.diff != DiffKind::None && diffi_mode.is_none() {
                    continue;
                }
                let mode = if std.diff == DiffKind::None {
                    ibuf_mode
                } else {
                    diffi_mode.unwrap()
                };
                for &vccaux in vccaux_list {
                    let is_input_dci = matches!(
                        std.dci,
                        DciKind::InputSplit | DciKind::InputVcc | DciKind::BiSplit | DciKind::BiVcc
                    );
                    let special: Option<Box<DynProp>> = if std.diff != DiffKind::None {
                        if is_input_dci {
                            // sigh.
                            continue;
                        } else if edev.chip.kind == ChipKind::Spartan3E {
                            // I hate ISE.
                            Some(Box::new(OtherIobDiffOutput(iob.ioi, std.name.to_string())))
                        } else {
                            None
                        }
                    } else if std.vref.is_some() || is_input_dci {
                        Some(Box::new(OtherIobInput(iob.ioi, std.name.to_string())))
                    } else {
                        None
                    };
                    let attr = if std.name.starts_with("DIFF_") {
                        specials::IOB_ISTD_DIFF
                    } else {
                        match vccaux {
                            "2.5" => specials::IOB_ISTD_2V5,
                            "3.3" => specials::IOB_ISTD_3V3,
                            _ => specials::IOB_ISTD,
                        }
                    };
                    if edev.chip.kind.is_spartan3a() {
                        let mut builder = bctx
                            .build()
                            .global_mutex("DIFF", "INPUT")
                            .raw(Key::VccAux, vccaux)
                            .attr("OMUX", "#OFF")
                            .attr("TMUX", "#OFF")
                            .attr("IFFDMUX", "#OFF")
                            .attr("PULL", "PULLDOWN")
                            .raw(Key::Package, &package.name);
                        if std.vref.is_some() {
                            builder = builder.global_mutex("VREF", "YES");
                        }
                        if let Some(ref special) = special {
                            builder = builder.prop_box(special.clone());
                        }
                        builder
                            .test_bel_special_row(attr, rid)
                            .mode_diff(ibuf_mode, mode)
                            .attr("IOATTRBOX", std.name)
                            .attr("IBUF_DELAY_VALUE", "DLY0")
                            .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                            .attr("SEL_MUX", "0")
                            .attr("IMUX", "1")
                            .pin("I")
                            .iob_commit(iob);
                    } else {
                        let mut builder = bctx
                            .build()
                            .global_mutex("DIFF", "INPUT")
                            .attr("OMUX", "#OFF")
                            .attr("TMUX", "#OFF")
                            .attr("IFFDMUX", "#OFF")
                            .attr("PULL", "PULLDOWN")
                            .raw(Key::Package, &package.name);
                        if std.vref.is_some() {
                            builder = builder.global_mutex("VREF", "YES");
                        }
                        if std.dci != DciKind::None {
                            builder = builder.global_mutex("DCI", std.name);
                        }
                        if let Some(ref special) = special {
                            builder = builder.prop_box(special.clone());
                        }

                        builder
                            .test_bel_special_row(attr, rid)
                            .mode_diff(ibuf_mode, mode)
                            .attr("IOATTRBOX", std.name)
                            .attr("IMUX", "1")
                            .pin("I")
                            .iob_commit(iob);
                    }
                    if std.diff != DiffKind::None {
                        let mut builder = bctx
                            .build()
                            .global_mutex("DIFF", "INPUT")
                            .attr("OMUX", "#OFF")
                            .attr("TMUX", "#OFF")
                            .attr("IMUX", "#OFF")
                            .attr("IFFDMUX", "#OFF")
                            .attr("PULL", "#OFF")
                            .raw(Key::Package, &package.name);
                        if std.dci != DciKind::None {
                            builder = builder.global_mutex("DCI", std.name);
                        }
                        if let Some(ref special) = special {
                            builder = builder.prop_box(special.clone());
                        }
                        builder
                            .test_bel_special_row(
                                if iostd_is_dt(&std) {
                                    specials::IOB_ISTD_COMP_DT
                                } else {
                                    specials::IOB_ISTD_COMP
                                },
                                rid,
                            )
                            .mode_diff(ibuf_mode, mode)
                            .attr("IOATTRBOX", std.name)
                            .attr("PADOUT_USED", "0")
                            .pin("PADOUT")
                            .iob_commit(iob);
                    }
                }
            }
            if edev.chip.kind.is_spartan3a() {
                bctx.mode(iob_mode)
                    .attr("OMUX", "O1")
                    .attr("O1INV", "O1")
                    .attr("O1_DDRMUX", "1")
                    .attr("TMUX", "T1")
                    .attr("T1INV", "T1")
                    .attr("T_USED", "0")
                    .attr("IFFDMUX", "#OFF")
                    .attr("PULL", "PULLDOWN")
                    .attr("IOATTRBOX", "LVCMOS33")
                    .pin("O1")
                    .pin("T1")
                    .pin("T")
                    // noop relation — mind the iob_commit at the end.
                    .extra_tile_bel_special(
                        NoopRelation,
                        iob.ioi,
                        if iob.kind == IobKind::Ibuf {
                            specials::IOI_SEL_MUX_OMUX_IBUF
                        } else {
                            specials::IOI_SEL_MUX_OMUX
                        },
                    )
                    .test_bel_special(specials::IOB_SEL_MUX_OMUX)
                    .attr("IBUF_DELAY_VALUE", "DLY0")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .attr("SEL_MUX", "1")
                    .attr("IMUX", "1")
                    .pin("I")
                    .iob_commit(iob);
                bctx.mode(iob_mode)
                    .attr("OMUX", "O1")
                    .attr("O1INV", "O1")
                    .attr("O1_DDRMUX", "1")
                    .attr("TMUX", "T1")
                    .attr("T1INV", "T1")
                    .attr("T_USED", "0")
                    .attr("IFFDMUX", "#OFF")
                    .attr("PULL", "PULLDOWN")
                    .attr("IOATTRBOX", "LVCMOS33")
                    .pin("O1")
                    .pin("T1")
                    .pin("T")
                    .test_bel_special(specials::IOB_SEL_MUX_TMUX)
                    .attr("IBUF_DELAY_VALUE", "DLY0")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .attr("SEL_MUX", "2")
                    .attr("IMUX", "1")
                    .pin("I")
                    .iob_commit(iob);
            }
            if let Some(pkg) = has_any_vref(edev, backend.device, backend.db, tcid, iob.iob) {
                bctx.build()
                    .raw(Key::Package, pkg)
                    .global_mutex("VREF", "YES")
                    .prop(IsVref(iob.ioi))
                    .prop(OtherIobInput(iob.ioi, "SSTL2_I".to_string()))
                    .test_bel_special(specials::IOB_MODE_NOTVREF)
                    .mode(ibuf_mode)
                    .iob_commit(iob);
            }
            if let Some((pkg, alt)) = has_any_vr(edev, backend.device, backend.db, tcid, iob.iob) {
                let mut builder = bctx
                    .build()
                    .raw(Key::Package, pkg)
                    .global_mutex("DCI", "YES");
                if let Some(alt) = alt {
                    builder = builder.raw(Key::AltVr, alt);
                }
                builder
                    .prop(IsVr(iob.ioi))
                    .prop(OtherIobInput(iob.ioi, "GTL_DCI".to_string()))
                    .test_bel_special(specials::IOB_MODE_NOTVR)
                    .mode(ibuf_mode)
                    .iob_commit(iob);
            }
            if edev.chip.kind.is_spartan3ea()
                && !is_s3a_we
                && iob.diff != IobDiff::None
                && iob.kind != IobKind::Ibuf
            {
                let difft_mode = if edev.chip.kind == ChipKind::Spartan3E {
                    match iob.diff {
                        IobDiff::None => unreachable!(),
                        IobDiff::True(_) => "DIFFM",
                        IobDiff::Comp(_) => "DIFFS",
                    }
                } else {
                    diffi_mode.unwrap()
                };
                if edev.chip.kind.is_spartan3a() {
                    bctx.mode(difft_mode)
                        .global_mutex("DIFF", "TERM")
                        .attr("OMUX", "#OFF")
                        .attr("TMUX", "#OFF")
                        .attr("IFFDMUX", "#OFF")
                        .attr("PULL", "PULLDOWN")
                        .attr("IOATTRBOX", "LVDS_25")
                        .attr("IBUF_DELAY_VALUE", "DLY0")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .attr("SEL_MUX", "0")
                        .attr("IMUX", "1")
                        .pin("I")
                        .test_bel_special(specials::IOB_DIFF_TERM)
                        .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                        .iob_commit(iob);
                } else {
                    bctx.mode(difft_mode)
                        .global_mutex("DIFF", "TERM")
                        .attr("OMUX", "#OFF")
                        .attr("TMUX", "#OFF")
                        .attr("IFFDMUX", "#OFF")
                        .attr("PULL", "PULLDOWN")
                        .attr("IOATTRBOX", "LVDS_25")
                        .attr("IMUX", "1")
                        .pin("I")
                        .test_bel_special(specials::IOB_DIFF_TERM)
                        .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                        .iob_commit(iob);
                }
                bctx.mode(difft_mode)
                    .global_mutex("DIFF", "TERM")
                    .attr("OMUX", "#OFF")
                    .attr("TMUX", "#OFF")
                    .attr("IMUX", "#OFF")
                    .attr("IFFDMUX", "#OFF")
                    .attr("PULL", "#OFF")
                    .attr("IOATTRBOX", "LVDS_25")
                    .attr("PADOUT_USED", "0")
                    .pin("PADOUT")
                    .test_bel_special(specials::IOB_DIFF_TERM_COMP)
                    .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                    .iob_commit(iob);
            }

            if iob.kind != IobKind::Ibuf {
                // Output path.
                bctx.mode("IOB")
                    .attr("PULL", "PULLDOWN")
                    .attr("TMUX", "#OFF")
                    .attr("IMUX", "#OFF")
                    .attr("IFFDMUX", "#OFF")
                    // noop relation — mind the iob_commit at the end.
                    .extra_tile_bel_special(NoopRelation, iob.ioi, specials::IOI_OUTPUT_ENABLE)
                    .test_bel_attr_bits(bcls::IOB::OUTPUT_ENABLE)
                    .attr("IOATTRBOX", "LVCMOS33")
                    .attr("OMUX", "O1")
                    .attr("O1INV", "O1")
                    .attr("DRIVE_0MA", "DRIVE_0MA")
                    .pin("O1")
                    .iob_commit(iob);
                for std in get_iostds(edev, is_s3a_we) {
                    let rid = iostd_to_row(edev, &std);
                    if std.input_only {
                        continue;
                    }
                    if matches!(std.diff, DiffKind::True | DiffKind::TrueTerm) && is_s3a_we {
                        continue;
                    }
                    match std.diff {
                        DiffKind::None => (),
                        DiffKind::Pseudo => {
                            if iob.diff == IobDiff::None {
                                continue;
                            }
                        }
                        DiffKind::True | DiffKind::TrueTerm => continue,
                    }
                    let (drives, slews) = if std.drive.is_empty() {
                        (&[0][..], &[specials::IOB_SLEW_NONE][..])
                    } else {
                        (
                            std.drive,
                            if edev.chip.kind.is_spartan3a() {
                                &[
                                    specials::IOB_SLEW_FAST,
                                    specials::IOB_SLEW_SLOW,
                                    specials::IOB_SLEW_QUIETIO,
                                ][..]
                            } else {
                                &[specials::IOB_SLEW_FAST, specials::IOB_SLEW_SLOW][..]
                            },
                        )
                    };
                    let vccauxs = if edev.chip.kind.is_spartan3a()
                        && matches!(std.diff, DiffKind::None | DiffKind::Pseudo)
                    {
                        &["2.5", "3.3"][..]
                    } else {
                        &[""][..]
                    };
                    for &vccaux in vccauxs {
                        let attr = if std.name.starts_with("DIFF_") {
                            match vccaux {
                                "" => specials::IOB_OSTD_DIFF,
                                "2.5" => specials::IOB_OSTD_DIFF_2V5,
                                "3.3" => specials::IOB_OSTD_DIFF_3V3,
                                _ => unreachable!(),
                            }
                        } else {
                            match vccaux {
                                "" => specials::IOB_OSTD,
                                "2.5" => specials::IOB_OSTD_2V5,
                                "3.3" => specials::IOB_OSTD_3V3,
                                _ => unreachable!(),
                            }
                        };
                        for &drive in drives {
                            for &slew in slews {
                                let dci_spec = if std.dci == DciKind::None {
                                    None
                                } else if matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                                    Some(OtherIobInput(iob.ioi, "SSTL2_I_DCI".into()))
                                } else if std.diff == DiffKind::None {
                                    Some(OtherIobInput(iob.ioi, std.name.to_string()))
                                } else {
                                    // can't be bothered to get it working.
                                    continue;
                                };
                                let mode = if std.diff == DiffKind::None {
                                    if is_s3a_we { "IOBLR" } else { "IOB" }
                                } else {
                                    match iob.diff {
                                        IobDiff::None => continue,
                                        IobDiff::True(_) => {
                                            if is_s3a_we {
                                                "DIFFMLR"
                                            } else if edev.chip.kind.is_spartan3a() {
                                                "DIFFMTB"
                                            } else {
                                                "DIFFM"
                                            }
                                        }
                                        IobDiff::Comp(_) => {
                                            if is_s3a_we {
                                                "DIFFSLR"
                                            } else if edev.chip.kind.is_spartan3a() {
                                                "DIFFSTB"
                                            } else {
                                                "DIFFS"
                                            }
                                        }
                                    }
                                };
                                let mut builder = bctx
                                    .build()
                                    .raw(Key::Package, &package.name)
                                    .global_mutex("DCI", "YES");
                                if !vccaux.is_empty() {
                                    builder = builder.raw(Key::VccAux, vccaux);
                                }
                                if let Some(dci_spec) = dci_spec {
                                    builder = builder.prop(dci_spec);
                                }
                                builder
                                    .attr("PULL", "PULLDOWN")
                                    .attr("TMUX", "#OFF")
                                    .attr("IMUX", "#OFF")
                                    .attr("IFFDMUX", "#OFF")
                                    .attr("OMUX", "O1")
                                    .attr("O1INV", "O1")
                                    .pin("O1")
                                    .test_bel_sss_row(attr, drive_to_spec(drive), slew, rid)
                                    .mode_diff("IOB", mode)
                                    .attr_diff("IOATTRBOX", "LVCMOS33", std.name)
                                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                    .attr(
                                        "DRIVEATTRBOX",
                                        if drive == 0 {
                                            "".to_string()
                                        } else {
                                            drive.to_string()
                                        },
                                    )
                                    .attr("SLEW", spec_to_slew(slew))
                                    .attr(
                                        "SUSPEND",
                                        if edev.chip.kind.is_spartan3a() {
                                            "3STATE"
                                        } else {
                                            ""
                                        },
                                    )
                                    .iob_commit(iob);
                            }
                        }
                    }
                }
                if let IobDiff::True(other_iob) = iob.diff {
                    let other_idx = bslots::IOB.index_of(other_iob).unwrap();
                    let iob_n = iob_data.iobs[other_idx];
                    for std in get_iostds(edev, is_s3a_we) {
                        let rid = iostd_to_row(edev, &std);
                        if is_s3a_we {
                            continue;
                        }
                        if !matches!(std.diff, DiffKind::True) {
                            continue;
                        }
                        let (mode_p, mode_n) = if edev.chip.kind.is_spartan3a() {
                            ("DIFFMTB", "DIFFSTB")
                        } else {
                            ("DIFFM", "DIFFS")
                        };
                        bctx.build()
                            .raw(Key::Package, &package.name)
                            .global_mutex("DCI", "YES")
                            .global_mutex("DIFF", "OUTPUT")
                            .prop(BankDiffOutput(iob.ioi, std.name.to_string(), None))
                            .attr("PULL", "PULLDOWN")
                            .bel_attr(iob_n.ioi, "PULL", "PULLDOWN")
                            .attr("TMUX", "#OFF")
                            .attr("IMUX", "#OFF")
                            .attr("IFFDMUX", "#OFF")
                            .attr("OMUX", "O1")
                            .attr("O1INV", "O1")
                            .bel_attr(iob_n.ioi, "TMUX", "#OFF")
                            .bel_attr(iob_n.ioi, "IMUX", "#OFF")
                            .bel_attr(iob_n.ioi, "IFFDMUX", "#OFF")
                            .bel_attr(iob_n.ioi, "OMUX", "#OFF")
                            .pin("O1")
                            .test_bel_special_row(specials::IOB_DIFFO, rid)
                            .mode_diff("IOB", mode_p)
                            .bel_mode_diff(iob_n.ioi, "IOB", mode_n)
                            .attr_diff("IOATTRBOX", "LVCMOS33", std.name)
                            .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                            .bel_attr(iob_n.ioi, "IOATTRBOX", std.name)
                            .bel_attr(iob_n.ioi, "DIFFO_IN_USED", "0")
                            .pin("DIFFO_OUT")
                            .bel_pin(iob_n.ioi, "DIFFO_IN")
                            .attr(
                                "SUSPEND",
                                if edev.chip.kind.is_spartan3a() {
                                    "3STATE"
                                } else {
                                    ""
                                },
                            )
                            .bel_attr(
                                iob_n.ioi,
                                "SUSPEND",
                                if edev.chip.kind.is_spartan3a() {
                                    "3STATE"
                                } else {
                                    ""
                                },
                            )
                            .iob_commit(iob);
                        if edev.chip.kind.is_spartan3ea() {
                            let altstd = if std.name == "RSDS_25" {
                                "MINI_LVDS_25"
                            } else {
                                "RSDS_25"
                            };
                            bctx.build()
                                .raw(Key::Package, &package.name)
                                .global_mutex("DCI", "YES")
                                .global_mutex("DIFF", "OUTPUT")
                                .prop(BankDiffOutput(
                                    iob.ioi,
                                    altstd.to_string(),
                                    Some(std.name.to_string()),
                                ))
                                .attr("PULL", "PULLDOWN")
                                .bel_attr(iob_n.ioi, "PULL", "PULLDOWN")
                                .attr("TMUX", "#OFF")
                                .attr("IMUX", "#OFF")
                                .attr("IFFDMUX", "#OFF")
                                .attr("OMUX", "O1")
                                .attr("O1INV", "O1")
                                .bel_attr(iob_n.ioi, "TMUX", "#OFF")
                                .bel_attr(iob_n.ioi, "IMUX", "#OFF")
                                .bel_attr(iob_n.ioi, "IFFDMUX", "#OFF")
                                .bel_attr(iob_n.ioi, "OMUX", "#OFF")
                                .pin("O1")
                                .test_bel_special_row(specials::IOB_DIFFO_ALT, rid)
                                .mode_diff("IOB", mode_p)
                                .bel_mode_diff(iob_n.ioi, "IOB", mode_n)
                                .attr_diff("IOATTRBOX", "LVCMOS33", std.name)
                                .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                .bel_attr(iob_n.ioi, "IOATTRBOX", std.name)
                                .bel_attr(iob_n.ioi, "DIFFO_IN_USED", "0")
                                .pin("DIFFO_OUT")
                                .bel_pin(iob_n.ioi, "DIFFO_IN")
                                .attr(
                                    "SUSPEND",
                                    if edev.chip.kind.is_spartan3a() {
                                        "3STATE"
                                    } else {
                                        ""
                                    },
                                )
                                .bel_attr(
                                    iob_n.ioi,
                                    "SUSPEND",
                                    if edev.chip.kind.is_spartan3a() {
                                        "3STATE"
                                    } else {
                                        ""
                                    },
                                )
                                .iob_commit(iob);
                        }
                    }
                }
                if matches!(
                    edev.chip.kind,
                    ChipKind::Virtex2P | ChipKind::Virtex2PX | ChipKind::Spartan3
                ) && !backend.device.name.ends_with("2vp4")
                    && !backend.device.name.ends_with("2vp7")
                {
                    for (spec, val) in [
                        (specials::DCI_ASREQUIRED, "ASREQUIRED"),
                        (specials::DCI_CONTINUOUS, "CONTINUOUS"),
                        (specials::DCI_QUIET, "QUIET"),
                    ] {
                        bctx.mode("IOB")
                            .global("DCIUPDATEMODE", val)
                            .raw(Key::Package, &package.name)
                            .global_mutex("DCI", "UPDATEMODE")
                            .prop(OtherIobInput(iob.ioi, "SSTL2_I_DCI".into()))
                            .attr("PULL", "PULLDOWN")
                            .attr("TMUX", "#OFF")
                            .attr("IMUX", "#OFF")
                            .attr("IFFDMUX", "#OFF")
                            .attr("OMUX", "O1")
                            .attr("O1INV", "O1")
                            .pin("O1")
                            .test_bel_special(spec)
                            .attr_diff("IOATTRBOX", "LVCMOS33", "LVDCI_33")
                            .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                            .iob_commit(iob);
                    }
                }
                if edev.chip.kind.is_spartan3a() {
                    bctx.mode("IOB")
                        .attr("IMUX", "#OFF")
                        .attr("IOATTRBOX", "#OFF")
                        .attr("OMUX", "O1")
                        .attr("O1INV", "O1")
                        .pin("O1")
                        .test_bel_special_bits(specials::IOB_OPROGRAMMING)
                        .prop(FuzzBelMultiAttr::new(
                            iob.ioi,
                            0,
                            "OPROGRAMMING".into(),
                            MultiValue::Bin,
                            16,
                        ))
                        .iob_commit(iob);
                }
            }
            if let Some((brefclk, _bufg)) = has_any_brefclk(edev, tcid, iob.iob) {
                let brefclk_bel_id = bslots::GLOBALSIG_BUFG[brefclk];

                bctx.build()
                    .test_bel_attr_bits(bcls::IOB::BREFCLK)
                    .related_pip(
                        IobBrefclkClkBT,
                        (brefclk_bel_id, "BREFCLK_O"),
                        (brefclk_bel_id, "BREFCLK_I"),
                    )
                    .iob_commit(iob);
            }
        }
        if tile.ends_with("CLK") {
            // Virtex 2 Pro X special!
            let bslot = bslots::IOI[if tcid == tcls_v2::IOB_V2P_SE2_CLK {
                2
            } else {
                0
            }];
            let mut bctx = ctx.bel(bslot);
            bctx.build()
                .test_bel_special(specials::IOB_CLK_ENABLE)
                .related_pip(
                    IobRelation(CellSlotId::from_idx(1)),
                    (bslot, "I"),
                    (PinFar, bslot, "BREFCLK"),
                )
                .commit();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_iob_data(
    ctx: &mut CollectorCtx,
    tcid: TileClassId,
    bslot: BelSlotId,
    attr: BelAttributeId,
    field: TableFieldId,
    diffs: Vec<(TableRowId, Diff)>,
    present: &Diff,
    ocd: OcdMode,
) {
    let mut item = xlat_enum_raw(diffs, ocd);
    if attr == bcls::IOB::S3E_PDRIVE {
        // needs a little push.
        enum_ocd_swap_bits(&mut item, 0, 1);
    }
    let abits = Vec::from_iter(item.bits.iter().map(|&bit| PolTileBit {
        bit,
        inv: present.bits.contains_key(&bit),
    }));
    ctx.insert_bel_attr_bitvec(tcid, bslot, attr, abits);
    for (row, value) in item.values {
        ctx.insert_table_bitvec(tables::IOB_DATA, row, field, value);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let intdb = ctx.edev.db;

    // IOI
    for (tcid, _, tcls) in &intdb.tile_classes {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let int_tiles = &[match edev.chip.kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => match tcid {
                tcls_v2::IOI_CLK_S => tcls_v2::INT_IOI_CLK_S,
                tcls_v2::IOI_CLK_N => tcls_v2::INT_IOI_CLK_N,
                _ => tcls_v2::INT_IOI,
            },
            ChipKind::Spartan3 => tcls_s3::INT_IOI_S3,
            ChipKind::FpgaCore => unreachable!(),
            ChipKind::Spartan3E => tcls_s3::INT_IOI_S3E,
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                if tcid == tcls_s3::IOI_S3A_WE {
                    tcls_s3::INT_IOI_S3A_WE
                } else {
                    tcls_s3::INT_IOI_S3A_SN
                }
            }
        }];

        for (bslot, _) in &tcls.bels {
            let Some(idx) = bslots::IOI.index_of(bslot) else {
                continue;
            };
            if edev.chip.kind.is_virtex2() && tcid == tcls_v2::IOI_CLK_N && matches!(idx, 0 | 1) {
                continue;
            }
            if edev.chip.kind.is_virtex2() && tcid == tcls_v2::IOI_CLK_S && matches!(idx, 2 | 3) {
                continue;
            }
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::OTCLK1);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::OTCLK2);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::ICLK1);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::ICLK2);
            ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::IOI::SR);
            ctx.collect_bel_input_inv_int_bi(int_tiles, tcid, bslot, bcls::IOI::OCE);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::REV);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::ICE);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::TCE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IOI::FFI_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IOI::FFO_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IOI::FFT_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IOI::FFI_REV_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IOI::FFO_REV_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::IOI::FFT_REV_ENABLE);

            if edev.chip.kind.is_spartan3ea() {
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_OCE,
                    enums::IOI_MUX_OCE::NONE,
                );
            }
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::O1);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::O2);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::T1);
            ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::IOI::T2);
            ctx.collect_bel_attr_default(tcid, bslot, bcls::IOI::MUX_T, enums::IOI_MUX_T::NONE);
            {
                // hack to avoid dragging IOB into it.
                let mut diffs = vec![];
                for val in [
                    enums::IOI_MUX_O::O1,
                    enums::IOI_MUX_O::O2,
                    enums::IOI_MUX_O::FFO1,
                    enums::IOI_MUX_O::FFO2,
                    enums::IOI_MUX_O::FFODDR,
                ] {
                    diffs.push((
                        val,
                        ctx.get_diff_attr_val(tcid, bslot, bcls::IOI::MUX_O, val),
                    ));
                }
                let mut item = xlat_enum_attr(diffs);
                item.values.insert(
                    enums::IOI_MUX_O::NONE,
                    BitVec::repeat(false, item.bits.len()),
                );
                ctx.insert_bel_attr_enum(tcid, bslot, bcls::IOI::MUX_O, item);
            }

            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFI_LATCH);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFO1_LATCH);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFO2_LATCH);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFT1_LATCH);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFT2_LATCH);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFI1_SRVAL);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFI2_SRVAL);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFO1_SRVAL);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFO2_SRVAL);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFT1_SRVAL);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFT2_SRVAL);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFI1_INIT);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFI2_INIT);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFO_INIT);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFT_INIT);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFI_SR_SYNC);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFO_SR_SYNC);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::FFT_SR_SYNC);

            // Input path
            ctx.collect_bel_attr(tcid, bslot, bcls::IOI::MUX_TSBYPASS);

            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::I_DELAY_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::IQ_DELAY_ENABLE);
            if edev.chip.kind.is_spartan3a() && tcid == tcls_s3::IOI_S3A_WE {
                let mut diffs = vec![];
                for val in 0..16 {
                    let mut bv = BitVec::repeat(false, 4);
                    for i in 0..4 {
                        bv.set(i, (val & 1 << i) != 0);
                    }
                    diffs.push((
                        bv.clone(),
                        ctx.get_diff_attr_bitvec(tcid, bslot, bcls::IOI::I_DELAY, bv),
                    ));
                }
                let mut bits = xlat_bitvec_sparse(diffs);
                let common = bits.pop().unwrap();
                ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOI::I_DELAY, bits);

                let mut diffs = vec![];
                for val in 0..8 {
                    let mut bv = BitVec::repeat(false, 3);
                    for i in 0..3 {
                        bv.set(i, (val & 1 << i) != 0);
                    }
                    diffs.push((
                        bv.clone(),
                        ctx.get_diff_attr_bitvec(tcid, bslot, bcls::IOI::IQ_DELAY, bv),
                    ));
                }
                let mut bits = xlat_bitvec_sparse(diffs);
                assert_eq!(common, bits.pop().unwrap());
                ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOI::IQ_DELAY, bits);

                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOI::DELAY_COMMON, common);

                ctx.collect_bel_attr(tcid, bslot, bcls::IOI::DELAY_VARIABLE);
            }
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::I_TSBYPASS_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOI::IQ_TSBYPASS_ENABLE);

            if edev.chip.kind.is_spartan3ea() {
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_FFI,
                    enums::IOI_MUX_FFI::NONE,
                );
            }
            if edev.chip.kind == ChipKind::Spartan3E {
                let en = ctx.get_diff_attr_bit(tcid, bslot, bcls::IOI::MISR_ENABLE, 0);
                let en_rst = ctx.get_diff_attr_bit(tcid, bslot, bcls::IOI::MISR_RESET, 0);
                let rst = en_rst.combine(&!&en);
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOI::MISR_RESET, xlat_bit(rst));
                let clk1 = ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_MISR_CLOCK,
                    enums::IOI_MUX_MISR_CLOCK::OTCLK1,
                );
                let clk2 = ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_MISR_CLOCK,
                    enums::IOI_MUX_MISR_CLOCK::OTCLK2,
                );
                assert_eq!(en, clk1);
                let (clk1, clk2, en) = Diff::split(clk1, clk2);
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOI::MISR_ENABLE, xlat_bit(en));
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_MISR_CLOCK,
                    xlat_enum_attr(vec![
                        (enums::IOI_MUX_MISR_CLOCK::OTCLK1, clk1),
                        (enums::IOI_MUX_MISR_CLOCK::OTCLK2, clk2),
                        (enums::IOI_MUX_MISR_CLOCK::NONE, Diff::default()),
                    ]),
                );
            }
            if edev.chip.kind.is_spartan3a() {
                ctx.collect_bel_attr(tcid, bslot, bcls::IOI::MISR_ENABLE);
                let clk1 = ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_MISR_CLOCK,
                    enums::IOI_MUX_MISR_CLOCK::OTCLK1,
                );
                let clk2 = ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_MISR_CLOCK,
                    enums::IOI_MUX_MISR_CLOCK::OTCLK2,
                );
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_MISR_CLOCK,
                    xlat_enum_attr(vec![
                        (enums::IOI_MUX_MISR_CLOCK::OTCLK1, clk1),
                        (enums::IOI_MUX_MISR_CLOCK::OTCLK2, clk2),
                        (enums::IOI_MUX_MISR_CLOCK::NONE, Diff::default()),
                    ]),
                );
                // Spartan 3A also has the MISRRESET global option, but it affects *all*
                // IOIs in the device, whether they're in use or not, so we cannot easily
                // isolate the diff to a single IOI. The bits are the same as Spartan 3E,
                // so just cheat and inject them manually.
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslot,
                    bcls::IOI::MISR_RESET,
                    TileBit::new(0, 0, [7, 32, 47][idx]).pos(),
                )
            }
            // these could be extracted automatically from .ll files but I'm not setting up
            // a whole another kind of fuzzer for a handful of bits.
            let bit = if edev.chip.kind.is_virtex2() {
                [
                    TileBit::new(0, 2, 13),
                    TileBit::new(0, 2, 33),
                    TileBit::new(0, 2, 53),
                    TileBit::new(0, 2, 73),
                ][idx]
            } else {
                [
                    TileBit::new(0, 3, 0),
                    TileBit::new(0, 3, 39),
                    TileBit::new(0, 3, 40),
                ][idx]
            };
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOI::READBACK_I, bit.pos());

            // discard detritus from IOB testing
            if !edev.chip.kind.is_spartan3a() || bslot != bslots::IOI[2] {
                let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OUTPUT_ENABLE);
                diff.apply_bit_diff(ctx.bel_input_inv(tcid, bslot, bcls::IOI::T1), true, false);
                diff.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, bcls::IOI::MUX_O),
                    enums::IOI_MUX_O::O1,
                    enums::IOI_MUX_O::NONE,
                );
                diff.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, bcls::IOI::MUX_T),
                    enums::IOI_MUX_T::T1,
                    enums::IOI_MUX_T::NONE,
                );
                diff.assert_empty();
            }
            if edev.chip.kind.is_spartan3a() {
                if bslot != bslots::IOI[2] {
                    ctx.get_diff_bel_special(tcid, bslot, specials::IOI_SEL_MUX_OMUX)
                        .assert_empty();
                }
                if bslot == bslots::IOI[2] || tcid == tcls_s3::IOI_S3A_WE {
                    let mut diff =
                        ctx.get_diff_bel_special(tcid, bslot, specials::IOI_SEL_MUX_OMUX_IBUF);
                    diff.apply_enum_diff(
                        ctx.bel_attr_enum(tcid, bslot, bcls::IOI::MUX_O),
                        enums::IOI_MUX_O::O1,
                        enums::IOI_MUX_O::NONE,
                    );
                    diff.assert_empty();
                }
            }
        }
        // specials. need cross-bel discard.
        if edev.chip.kind.is_spartan3ea() && tcls.bels.contains_id(bslots::IOI[0]) {
            for idx in 0..2 {
                let bslot = bslots::IOI[idx];
                let obslot = bslots::IOI[idx ^ 1];
                ctx.get_diff_attr_val(tcid, bslot, bcls::IOI::MUX_FFO1, enums::IOI_MUX_FFO1::O1)
                    .assert_empty();
                ctx.get_diff_attr_val(tcid, bslot, bcls::IOI::MUX_FFO2, enums::IOI_MUX_FFO2::O2)
                    .assert_empty();
                let mut diff = ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_FFO1,
                    enums::IOI_MUX_FFO1::PAIR_FFO2,
                );
                diff.discard_bits(&ctx.bel_attr_enum(tcid, obslot, bcls::IOI::MUX_O).bits);
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_FFO1,
                    xlat_enum_attr(vec![
                        (enums::IOI_MUX_FFO1::O1, Diff::default()),
                        (enums::IOI_MUX_FFO1::PAIR_FFO2, diff),
                    ]),
                );
                let mut diff = ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_FFO2,
                    enums::IOI_MUX_FFO2::PAIR_FFO1,
                );
                diff.discard_bits(&ctx.bel_attr_enum(tcid, obslot, bcls::IOI::MUX_O).bits);
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::IOI::MUX_FFO2,
                    xlat_enum_attr(vec![
                        (enums::IOI_MUX_FFO2::O2, Diff::default()),
                        (enums::IOI_MUX_FFO2::PAIR_FFO1, diff),
                    ]),
                );
            }
        }
    }

    // IOB
    let attr_output_diff = match edev.chip.kind {
        ChipKind::Virtex2 => bcls::IOB::V2_OUTPUT_DIFF,
        ChipKind::Virtex2P | ChipKind::Virtex2PX => bcls::IOB::V2P_OUTPUT_DIFF,
        ChipKind::Spartan3 => bcls::IOB::S3_OUTPUT_DIFF,
        ChipKind::FpgaCore => unreachable!(),
        ChipKind::Spartan3E => bcls::IOB::S3E_OUTPUT_DIFF,
        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => bcls::IOB::S3A_OUTPUT_DIFF,
    };
    for iob_data in get_iob_tiles(edev.chip.kind) {
        let tcid = iob_data.tcid;
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let is_s3a_we = !edev.chip.kind.is_virtex2()
            && (matches!(tcid, tcls_s3::IOB_S3A_W4 | tcls_s3::IOB_S3A_E4));
        for &iob in &iob_data.iobs {
            let bslot = iob.iob;
            if iob.kind == IobKind::Clk {
                continue;
            }
            if edev.chip.kind.is_spartan3ea() {
                ctx.get_diff_attr_bit(tcid, bslot, bcls::IOB::DISABLE_GTS, 0)
                    .assert_empty();
            } else {
                ctx.collect_bel_attr(tcid, bslot, bcls::IOB::DISABLE_GTS);
            }
            ctx.collect_bel_attr_default(tcid, bslot, bcls::IOB::PULL, enums::IOB_PULL::NONE);
            if edev.chip.kind.is_spartan3a() && iob.kind != IobKind::Ibuf {
                ctx.collect_bel_attr(tcid, bslot, bcls::IOB::SUSPEND);
            }
            if edev.chip.kind == ChipKind::Spartan3E {
                let is_e = matches!(
                    tcid,
                    tcls_s3::IOB_S3E_E1
                        | tcls_s3::IOB_S3E_E2
                        | tcls_s3::IOB_S3E_E3
                        | tcls_s3::IOB_S3E_E4
                );
                let max_i = if is_e { 12 } else { 13 };
                let max_iq = if is_e { 6 } else { 7 };
                for val in (max_i + 1)..=16 {
                    ctx.get_diff_attr_u32(tcid, bslot, bcls::IOB::I_DELAY, val)
                        .assert_empty();
                }
                for val in (max_iq + 1)..=8 {
                    ctx.get_diff_attr_u32(tcid, bslot, bcls::IOB::IQ_DELAY, val)
                        .assert_empty();
                }
                let mut diffs_i = BTreeMap::new();
                let mut diffs_iq = BTreeMap::new();
                for i in 1..=max_i {
                    let row = ctx.edev.db.tables[tables::IOB_I_DELAY]
                        .rows
                        .get(&format!("DLY{i}"))
                        .unwrap()
                        .0;
                    diffs_i.insert(
                        row,
                        ctx.get_diff_attr_u32(tcid, bslot, bcls::IOB::I_DELAY, i),
                    );
                }
                for i in 1..=max_iq {
                    let row = ctx.edev.db.tables[tables::IOB_IQ_DELAY]
                        .rows
                        .get(&format!("DLY{i}"))
                        .unwrap()
                        .0;
                    diffs_iq.insert(
                        row,
                        ctx.get_diff_attr_u32(tcid, bslot, bcls::IOB::IQ_DELAY, i),
                    );
                }

                let field_i = if is_e {
                    tables::IOB_I_DELAY::DELAY_E
                } else {
                    tables::IOB_I_DELAY::DELAY_WSN
                };
                let field_iq = if is_e {
                    tables::IOB_IQ_DELAY::DELAY_E
                } else {
                    tables::IOB_IQ_DELAY::DELAY_WSN
                };

                let bits_i;
                let bits_iq;
                if is_e {
                    bits_iq = vec![
                        !xlat_bit(diffs_iq[&tables::IOB_IQ_DELAY::DLY5].clone()),
                        !xlat_bit(diffs_iq[&tables::IOB_IQ_DELAY::DLY4].clone()),
                        !xlat_bit(diffs_iq[&tables::IOB_IQ_DELAY::DLY3].clone()),
                    ];
                    bits_i = vec![
                        !xlat_bit(diffs_i[&tables::IOB_I_DELAY::DLY11].clone()),
                        !xlat_bit(diffs_i[&tables::IOB_I_DELAY::DLY10].clone()),
                        !xlat_bit(diffs_i[&tables::IOB_I_DELAY::DLY8].clone()),
                        !xlat_bit(diffs_iq[&tables::IOB_IQ_DELAY::DLY3].clone()),
                    ];
                } else {
                    bits_iq = vec![
                        !xlat_bit(diffs_iq[&tables::IOB_IQ_DELAY::DLY6].clone()),
                        !xlat_bit(diffs_iq[&tables::IOB_IQ_DELAY::DLY5].clone()),
                        !xlat_bit(diffs_iq[&tables::IOB_IQ_DELAY::DLY4].clone()),
                    ];
                    bits_i = vec![
                        !xlat_bit(
                            diffs_i[&tables::IOB_I_DELAY::DLY11]
                                .combine(&!&diffs_i[&tables::IOB_I_DELAY::DLY12]),
                        ),
                        !xlat_bit(diffs_i[&tables::IOB_I_DELAY::DLY12].clone()),
                        !xlat_bit(diffs_i[&tables::IOB_I_DELAY::DLY10].clone()),
                        !xlat_bit(diffs_iq[&tables::IOB_IQ_DELAY::DLY4].clone()),
                    ];
                }
                for (row, diff) in diffs_iq {
                    let val = extract_bitvec_val(&bits_iq, &bits![1; 3], diff);
                    ctx.insert_table_bitvec(tables::IOB_IQ_DELAY, row, field_iq, val);
                }
                for (row, diff) in diffs_i {
                    let val = extract_bitvec_val(&bits_i, &bits![1; 4], diff);
                    ctx.insert_table_bitvec(tables::IOB_I_DELAY, row, field_i, val);
                }

                assert_eq!(bits_iq[2], bits_i[3]);
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::DELAY_COMMON, bits_iq[2]);
                ctx.insert_bel_attr_bitvec(
                    tcid,
                    bslot,
                    bcls::IOB::IQ_DELAY,
                    bits_iq[0..2].to_vec(),
                );
                ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::I_DELAY, bits_i[0..3].to_vec());
            }
            if edev.chip.kind.is_spartan3a() && !is_s3a_we {
                let mut diffs = vec![];
                for val in 0..16 {
                    let mut bv = BitVec::repeat(false, 4);
                    for i in 0..4 {
                        bv.set(i, (val & 1 << i) != 0);
                    }
                    diffs.push((
                        bv.clone(),
                        ctx.get_diff_attr_bitvec(tcid, bslot, bcls::IOB::I_DELAY, bv),
                    ));
                }
                let mut bits = xlat_bitvec_sparse(diffs);
                let common = bits.pop().unwrap();
                ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::I_DELAY, bits);

                let mut diffs = vec![];
                for val in 0..8 {
                    let mut bv = BitVec::repeat(false, 3);
                    for i in 0..3 {
                        bv.set(i, (val & 1 << i) != 0);
                    }
                    diffs.push((
                        bv.clone(),
                        ctx.get_diff_attr_bitvec(tcid, bslot, bcls::IOB::IQ_DELAY, bv),
                    ));
                }
                let mut bits = xlat_bitvec_sparse(diffs);
                assert_eq!(common, bits.pop().unwrap());
                ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::IQ_DELAY, bits);

                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::DELAY_COMMON, common);

                ctx.collect_bel_attr(tcid, bslot, bcls::IOB::DELAY_VARIABLE);
            }
            // Input path.
            if !edev.chip.kind.is_spartan3ea() {
                let mut vals = vec![
                    (enums::IOB_IBUF_MODE::NONE, Diff::default()),
                    (
                        enums::IOB_IBUF_MODE::CMOS,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::LVCMOS33,
                        )
                        .clone(),
                    ),
                    (
                        enums::IOB_IBUF_MODE::VREF,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::SSTL2_I,
                        )
                        .clone(),
                    ),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        enums::IOB_IBUF_MODE::DIFF,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::BLVDS_25,
                        )
                        .clone(),
                    ));
                }
                ctx.insert_bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE, xlat_enum_attr(vals));
            } else if edev.chip.kind == ChipKind::Spartan3E {
                let mut vals = vec![
                    (enums::IOB_IBUF_MODE::NONE, Diff::default()),
                    (
                        enums::IOB_IBUF_MODE::CMOS_LV,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::LVCMOS18,
                        )
                        .clone(),
                    ),
                    (
                        enums::IOB_IBUF_MODE::CMOS_HV,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::LVCMOS33,
                        )
                        .clone(),
                    ),
                    (
                        enums::IOB_IBUF_MODE::VREF,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::SSTL2_I,
                        )
                        .clone(),
                    ),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        enums::IOB_IBUF_MODE::DIFF,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::BLVDS_25,
                        )
                        .clone(),
                    ));
                }
                ctx.insert_bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE, xlat_enum_attr(vals));
            } else {
                let mut vals = vec![
                    (enums::IOB_IBUF_MODE::NONE, Diff::default()),
                    (
                        enums::IOB_IBUF_MODE::CMOS_VCCINT,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD_3V3,
                            IOB_DATA::LVCMOS18,
                        )
                        .clone(),
                    ),
                    (
                        enums::IOB_IBUF_MODE::CMOS_VCCAUX,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD_2V5,
                            IOB_DATA::LVCMOS25,
                        )
                        .clone(),
                    ),
                    (
                        enums::IOB_IBUF_MODE::CMOS_VCCO,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD_3V3,
                            IOB_DATA::LVCMOS25,
                        )
                        .clone(),
                    ),
                    (
                        enums::IOB_IBUF_MODE::VREF,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::SSTL2_I,
                        )
                        .clone(),
                    ),
                    (
                        enums::IOB_IBUF_MODE::LOOPBACK_T,
                        ctx.get_diff_bel_special(tcid, bslot, specials::IOB_SEL_MUX_TMUX),
                    ),
                    (
                        enums::IOB_IBUF_MODE::LOOPBACK_O,
                        ctx.get_diff_bel_special(tcid, bslot, specials::IOB_SEL_MUX_OMUX),
                    ),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        enums::IOB_IBUF_MODE::DIFF,
                        ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::BLVDS_25,
                        )
                        .clone(),
                    ));
                }
                ctx.insert_bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE, xlat_enum_attr(vals));
            }
            if edev.chip.kind.is_spartan3ea()
                && !is_s3a_we
                && iob.diff != IobDiff::None
                && iob.kind != IobKind::Ibuf
            {
                ctx.get_diff_bel_special(tcid, bslot, specials::IOB_DIFF_TERM_COMP)
                    .assert_empty();
                if matches!(iob.diff, IobDiff::Comp(_)) {
                    // ignore
                    ctx.get_diff_bel_special(tcid, bslot, specials::IOB_DIFF_TERM);
                }
            }
            if has_any_vref(edev, ctx.device, ctx.db, tcid, iob.iob).is_some() {
                let present_vref =
                    ctx.get_diff_bel_special(tcid, bslot, specials::IOB_MODE_NOTVREF);
                let present = ctx.peek_diff_bel_special(
                    tcid,
                    bslot,
                    if edev.chip.kind.is_spartan3ea() {
                        specials::IOB_MODE_IBUF
                    } else {
                        specials::IOB_MODE_IOB
                    },
                );
                let mut vref = present.combine(&!present_vref);
                vref.discard_bits(&ctx.bel_attr_enum(tcid, bslot, bcls::IOB::PULL).bits);
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::VREF, xlat_bit(vref));
            }

            // PCI cruft
            if edev.chip.kind.is_spartan3a() {
                let mut ibuf_diff = ctx
                    .peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD, IOB_DATA::PCI33_3)
                    .clone();
                ibuf_diff.discard_bits(&ctx.bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE).bits);
                if iob.kind == IobKind::Ibuf {
                    ctx.insert_bel_attr_bool(
                        tcid,
                        bslot,
                        bcls::IOB::PCI_INPUT,
                        xlat_bit(ibuf_diff),
                    );
                } else {
                    let obuf_diff = ctx
                        .peek_diff_bel_sss_row(
                            tcid,
                            bslot,
                            specials::IOB_OSTD_3V3,
                            specials::IOB_DRIVE_NONE,
                            specials::IOB_SLEW_NONE,
                            IOB_DATA::PCI33_3,
                        )
                        .clone();
                    let (ibuf_diff, _, common) = Diff::split(ibuf_diff, obuf_diff);
                    ctx.insert_bel_attr_bool(
                        tcid,
                        bslot,
                        bcls::IOB::PCI_INPUT,
                        xlat_bit(ibuf_diff),
                    );
                    ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::PCI_CLAMP, xlat_bit(common));
                }
            }

            // Output path.
            if iob.kind != IobKind::Ibuf {
                let diff = ctx.get_diff_attr_bit(tcid, bslot, bcls::IOB::OUTPUT_ENABLE, 0);
                ctx.insert_bel_attr_bitvec(
                    tcid,
                    bslot,
                    bcls::IOB::OUTPUT_ENABLE,
                    xlat_bit_wide(diff),
                );

                // well ...
                let mut slew_bits = HashSet::new();
                let mut drive_bits = HashSet::new();
                for std in get_iostds(edev, is_s3a_we) {
                    let rid = iostd_to_row(edev, &std);
                    if std.drive.is_empty() {
                        continue;
                    }
                    let vccauxs = if edev.chip.kind.is_spartan3a() {
                        &[specials::IOB_OSTD_2V5, specials::IOB_OSTD_3V3][..]
                    } else {
                        &[specials::IOB_OSTD][..]
                    };
                    let slews = if edev.chip.kind.is_spartan3a() {
                        &[
                            specials::IOB_SLEW_FAST,
                            specials::IOB_SLEW_SLOW,
                            specials::IOB_SLEW_QUIETIO,
                        ][..]
                    } else {
                        &[specials::IOB_SLEW_FAST, specials::IOB_SLEW_SLOW][..]
                    };
                    for &vccaux in vccauxs {
                        // grab SLEW bits.
                        for &drive in std.drive {
                            if edev.chip.kind.is_virtex2p() && std.name == "LVCMOS33" && drive == 8
                            {
                                // ISE bug.
                                continue;
                            }
                            let mut base: Option<Diff> = None;
                            for &slew in slews {
                                let diff = ctx.peek_diff_bel_sss_row(
                                    tcid,
                                    bslot,
                                    vccaux,
                                    drive_to_spec(drive),
                                    slew,
                                    rid,
                                );
                                if let Some(ref base) = base {
                                    let ddiff = diff.combine(&!base);
                                    for &bit in ddiff.bits.keys() {
                                        slew_bits.insert(bit);
                                    }
                                } else {
                                    base = Some(diff.clone());
                                }
                            }
                        }
                        // grab DRIVE bits.
                        for &slew in slews {
                            let mut base: Option<Diff> = None;
                            for &drive in std.drive {
                                let diff = ctx.peek_diff_bel_sss_row(
                                    tcid,
                                    bslot,
                                    vccaux,
                                    drive_to_spec(drive),
                                    slew,
                                    rid,
                                );
                                if let Some(ref base) = base {
                                    let ddiff = diff.combine(&!base);
                                    for &bit in ddiff.bits.keys() {
                                        drive_bits.insert(bit);
                                    }
                                } else {
                                    base = Some(diff.clone());
                                }
                            }
                        }
                    }
                }
                if edev.chip.kind.is_virtex2() {
                    // there is an extra PDRIVE bit on V2 that is not used by any LVCMOS/LVTTL
                    // standards, but is used by some other standards that need extra oomph.
                    // it would be easier to extract this from BLVDS_25, but that requires
                    // a differential pin...
                    let gtl = ctx.peek_diff_bel_sss_row(
                        tcid,
                        bslot,
                        specials::IOB_OSTD,
                        specials::IOB_DRIVE_NONE,
                        specials::IOB_SLEW_NONE,
                        IOB_DATA::GTL,
                    );
                    let gtlp = ctx.peek_diff_bel_sss_row(
                        tcid,
                        bslot,
                        specials::IOB_OSTD,
                        specials::IOB_DRIVE_NONE,
                        specials::IOB_SLEW_NONE,
                        IOB_DATA::GTLP,
                    );
                    for &bit in gtl.bits.keys() {
                        if !slew_bits.contains(&bit) && !gtlp.bits.contains_key(&bit) {
                            drive_bits.insert(bit);
                        }
                    }
                }
                let mut pdrive_bits = HashSet::new();
                let mut pslew_bits = vec![];
                let mut nslew_bits = vec![];
                if edev.chip.kind.is_spartan3a() {
                    let oprog =
                        ctx.extract_bel_special_bitvec(tcid, bslot, specials::IOB_OPROGRAMMING, 16);
                    for i in 13..16 {
                        pdrive_bits.insert(oprog[i].bit);
                    }
                    for i in 2..6 {
                        nslew_bits.push(oprog[i].bit);
                    }
                    for i in 6..10 {
                        pslew_bits.push(oprog[i].bit);
                    }
                } else if edev.chip.kind == ChipKind::Spartan3 {
                    pdrive_bits = drive_bits.clone();
                    for &bit in ctx
                        .peek_diff_bel_sss_row(
                            tcid,
                            bslot,
                            specials::IOB_OSTD,
                            specials::IOB_DRIVE_NONE,
                            specials::IOB_SLEW_NONE,
                            IOB_DATA::GTL,
                        )
                        .bits
                        .keys()
                    {
                        if drive_bits.contains(&bit) {
                            pdrive_bits.remove(&bit);
                        }
                    }
                } else {
                    let drives = if edev.chip.kind == ChipKind::Spartan3E {
                        &[2, 4, 6, 8, 12, 16][..]
                    } else {
                        &[2, 4, 6, 8, 12, 16, 24][..]
                    };
                    for &drive in drives {
                        let ttl = ctx.peek_diff_bel_sss_row(
                            tcid,
                            bslot,
                            specials::IOB_OSTD,
                            drive_to_spec(drive),
                            specials::IOB_SLEW_SLOW,
                            IOB_DATA::LVTTL,
                        );
                        let cmos = ctx.peek_diff_bel_sss_row(
                            tcid,
                            bslot,
                            specials::IOB_OSTD,
                            drive_to_spec(drive),
                            specials::IOB_SLEW_SLOW,
                            IOB_DATA::LVCMOS33,
                        );
                        let diff = ttl.combine(&!cmos);
                        for &bit in diff.bits.keys() {
                            pdrive_bits.insert(bit);
                        }
                    }
                }
                let pslew_bit_set = HashSet::from_iter(pslew_bits.iter().copied());
                let nslew_bit_set = HashSet::from_iter(nslew_bits.iter().copied());
                for &bit in &pdrive_bits {
                    assert!(drive_bits.remove(&bit));
                }
                for &bit in &pslew_bits {
                    assert!(slew_bits.remove(&bit));
                }
                for &bit in &nslew_bits {
                    assert!(slew_bits.remove(&bit));
                }
                let ndrive_bits = drive_bits;
                if !edev.chip.kind.is_spartan3ea() {
                    let mut dci_bits = HashSet::new();
                    let item = ctx.bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE);
                    let mut diff_split = ctx
                        .peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::SSTL2_I_DCI,
                        )
                        .clone();
                    diff_split.discard_bits(&item.bits);
                    for &bit in diff_split.bits.keys() {
                        dci_bits.insert(bit);
                    }
                    let mut diff_vcc = ctx
                        .peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_ISTD,
                            IOB_DATA::GTL_DCI,
                        )
                        .clone();
                    diff_vcc.discard_bits(&item.bits);
                    for &bit in diff_vcc.bits.keys() {
                        dci_bits.insert(bit);
                    }
                    let mut diff_output = ctx
                        .peek_diff_bel_sss_row(
                            tcid,
                            bslot,
                            specials::IOB_OSTD,
                            specials::IOB_DRIVE_NONE,
                            specials::IOB_SLEW_NONE,
                            IOB_DATA::LVDCI_25,
                        )
                        .clone();
                    let diff_output = diff_output.split_bits(&dci_bits);
                    let mut diff_output_half = ctx
                        .peek_diff_bel_sss_row(
                            tcid,
                            bslot,
                            specials::IOB_OSTD,
                            specials::IOB_DRIVE_NONE,
                            specials::IOB_SLEW_NONE,
                            IOB_DATA::LVDCI_DV2_25,
                        )
                        .clone();
                    let diff_output_half = diff_output_half.split_bits(&dci_bits);
                    ctx.insert_bel_attr_enum(
                        tcid,
                        bslot,
                        bcls::IOB::DCI_MODE,
                        xlat_enum_attr(vec![
                            (enums::IOB_DCI_MODE::NONE, Diff::default()),
                            (enums::IOB_DCI_MODE::OUTPUT, diff_output),
                            (enums::IOB_DCI_MODE::OUTPUT_HALF, diff_output_half),
                            (enums::IOB_DCI_MODE::TERM_SPLIT, diff_split),
                            (enums::IOB_DCI_MODE::TERM_VCC, diff_vcc),
                        ]),
                    );
                }
                let mut vr_slew = None;
                if has_any_vr(edev, ctx.device, ctx.db, tcid, iob.iob).is_some() {
                    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IOB_MODE_NOTVR);
                    let item = ctx.bel_attr_enum(tcid, bslot, bcls::IOB::DCI_MODE);
                    diff.apply_enum_diff(
                        item,
                        enums::IOB_DCI_MODE::NONE,
                        enums::IOB_DCI_MODE::TERM_SPLIT,
                    );
                    diff = !diff;
                    vr_slew = Some(diff.split_bits(&slew_bits));
                    ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::VR, xlat_bit(diff));
                }
                if matches!(
                    edev.chip.kind,
                    ChipKind::Virtex2P | ChipKind::Virtex2PX | ChipKind::Spartan3
                ) && !ctx.device.name.ends_with("2vp4")
                    && !ctx.device.name.ends_with("2vp7")
                {
                    let diff_a = ctx.get_diff_bel_special(tcid, bslot, specials::DCI_ASREQUIRED);
                    let diff_c = ctx.get_diff_bel_special(tcid, bslot, specials::DCI_CONTINUOUS);
                    let diff_q = ctx.get_diff_bel_special(tcid, bslot, specials::DCI_QUIET);
                    assert_eq!(diff_c, diff_q);
                    let diff = diff_a.combine(&!diff_c);
                    ctx.insert_bel_attr_bool(
                        tcid,
                        bslot,
                        bcls::IOB::DCIUPDATEMODE_ASREQUIRED,
                        xlat_bit(diff),
                    );
                }
                let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::IOB_MODE_IOB);
                if edev.chip.kind.is_spartan3ea() {
                    let ibuf_present =
                        ctx.get_diff_bel_special(tcid, bslot, specials::IOB_MODE_IBUF);
                    assert_eq!(present, ibuf_present);
                }
                present.discard_bits(&ctx.bel_attr_enum(tcid, bslot, bcls::IOB::PULL).bits);
                for &val in present.bits.values() {
                    assert!(val)
                }

                let mut slew_diffs = vec![];
                let mut pslew_diffs = vec![];
                let mut nslew_diffs = vec![];
                let mut pdrive_diffs = vec![];
                let mut ndrive_diffs = vec![];
                let mut misc_diffs = vec![];
                for std in get_iostds(edev, is_s3a_we) {
                    let rid = iostd_to_row(edev, &std);
                    if std.input_only {
                        continue;
                    }
                    if matches!(std.diff, DiffKind::True | DiffKind::TrueTerm) {
                        continue;
                    }
                    if std.diff == DiffKind::Pseudo && iob.diff == IobDiff::None {
                        continue;
                    }
                    let vccauxs = if edev.chip.kind.is_spartan3a() {
                        &["2.5", "3.3"][..]
                    } else {
                        &[""][..]
                    };
                    let (drives, slews) = if std.drive.is_empty() {
                        (&[0][..], &[specials::IOB_SLEW_NONE][..])
                    } else {
                        (
                            std.drive,
                            if edev.chip.kind.is_spartan3a() {
                                &[
                                    specials::IOB_SLEW_FAST,
                                    specials::IOB_SLEW_SLOW,
                                    specials::IOB_SLEW_QUIETIO,
                                ][..]
                            } else {
                                &[specials::IOB_SLEW_FAST, specials::IOB_SLEW_SLOW][..]
                            },
                        )
                    };
                    if std.dci != DciKind::None && std.diff != DiffKind::None {
                        continue;
                    }
                    for &vccaux in vccauxs {
                        let attr = if std.name.starts_with("DIFF_") {
                            match vccaux {
                                "" => specials::IOB_OSTD_DIFF,
                                "2.5" => specials::IOB_OSTD_DIFF_2V5,
                                "3.3" => specials::IOB_OSTD_DIFF_3V3,
                                _ => unreachable!(),
                            }
                        } else {
                            match vccaux {
                                "" => specials::IOB_OSTD,
                                "2.5" => specials::IOB_OSTD_2V5,
                                "3.3" => specials::IOB_OSTD_3V3,
                                _ => unreachable!(),
                            }
                        };
                        for &drive in drives {
                            for &slew in slews {
                                let mut diff = ctx.get_diff_bel_sss_row(
                                    tcid,
                                    bslot,
                                    attr,
                                    drive_to_spec(drive),
                                    slew,
                                    rid,
                                );
                                if edev.chip.kind.is_virtex2p()
                                    && std.name == "LVCMOS33"
                                    && drive == 8
                                    && slew == specials::IOB_SLEW_FAST
                                {
                                    // ISE bug.
                                    continue;
                                }
                                let slew_diff = diff.split_bits(&slew_bits);
                                let pslew_diff = diff.split_bits(&pslew_bit_set);
                                let nslew_diff = diff.split_bits(&nslew_bit_set);
                                let pdrive_diff = diff.split_bits(&pdrive_bits);
                                let ndrive_diff = diff.split_bits(&ndrive_bits);
                                if !edev.chip.kind.is_spartan3ea() {
                                    let item = ctx.bel_attr_enum(tcid, bslot, bcls::IOB::DCI_MODE);
                                    match std.dci {
                                        DciKind::Output => diff.apply_enum_diff(
                                            item,
                                            enums::IOB_DCI_MODE::OUTPUT,
                                            enums::IOB_DCI_MODE::NONE,
                                        ),
                                        DciKind::OutputHalf => diff.apply_enum_diff(
                                            item,
                                            enums::IOB_DCI_MODE::OUTPUT_HALF,
                                            enums::IOB_DCI_MODE::NONE,
                                        ),
                                        DciKind::BiVcc => diff.apply_enum_diff(
                                            item,
                                            enums::IOB_DCI_MODE::TERM_VCC,
                                            enums::IOB_DCI_MODE::NONE,
                                        ),
                                        DciKind::BiSplit => diff.apply_enum_diff(
                                            item,
                                            enums::IOB_DCI_MODE::TERM_SPLIT,
                                            enums::IOB_DCI_MODE::NONE,
                                        ),
                                        _ => (),
                                    }
                                }
                                if edev.chip.kind.is_spartan3a() && std.name.starts_with("PCI") {
                                    diff.apply_bit_diff(
                                        ctx.bel_attr_bit(tcid, bslot, bcls::IOB::PCI_CLAMP),
                                        true,
                                        false,
                                    );
                                }
                                let slew_rid = match slew {
                                    specials::IOB_SLEW_NONE => rid,
                                    specials::IOB_SLEW_FAST => IOB_DATA::SLEW_FAST,
                                    specials::IOB_SLEW_QUIETIO => IOB_DATA::SLEW_QUIETIO,
                                    specials::IOB_SLEW_SLOW => {
                                        if std.vcco == Some(3300) {
                                            IOB_DATA::SLEW_SLOW_3V3
                                        } else {
                                            IOB_DATA::SLEW_SLOW_LV
                                        }
                                    }
                                    _ => unreachable!(),
                                };
                                let drive_rid = get_drive_row(rid, drive);
                                slew_diffs.push((slew_rid, slew_diff));
                                pslew_diffs.push((slew_rid, pslew_diff));
                                if vccaux == "3.3" {
                                    nslew_diffs
                                        .push(((IOB_DATA::S3A_3V3_NSLEW, slew_rid), nslew_diff));
                                } else {
                                    nslew_diffs
                                        .push(((IOB_DATA::S3A_2V5_NSLEW, slew_rid), nslew_diff));
                                }
                                if matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                                    pdrive_diff.assert_empty();
                                    ndrive_diff.assert_empty();
                                } else {
                                    pdrive_diffs.push((drive_rid, pdrive_diff));
                                    ndrive_diffs.push((drive_rid, ndrive_diff));
                                }
                                if edev.chip.kind != ChipKind::Spartan3E
                                    || !std.name.starts_with("DIFF_")
                                {
                                    misc_diffs.push((rid, diff));
                                }
                            }
                        }
                    }
                }
                if let Some(vr_slew) = vr_slew {
                    slew_diffs.push((IOB_DATA::VR, vr_slew));
                }
                for &bit in present.bits.keys() {
                    assert!(pdrive_bits.contains(&bit) || ndrive_bits.contains(&bit));
                }
                pdrive_diffs.push((IOB_DATA::OFF, Diff::default()));
                ndrive_diffs.push((IOB_DATA::OFF, Diff::default()));
                misc_diffs.push((IOB_DATA::OFF, Diff::default()));
                pslew_diffs.push((IOB_DATA::OFF, Diff::default()));
                nslew_diffs.push(((IOB_DATA::S3A_3V3_NSLEW, IOB_DATA::OFF), Diff::default()));
                nslew_diffs.push(((IOB_DATA::S3A_2V5_NSLEW, IOB_DATA::OFF), Diff::default()));
                slew_diffs.push((IOB_DATA::OFF, Diff::default()));
                for (_, diff) in &mut pdrive_diffs {
                    for &bit in present.bits.keys() {
                        if !pdrive_bits.contains(&bit) {
                            continue;
                        }
                        match diff.bits.entry(bit) {
                            hash_map::Entry::Occupied(e) => {
                                assert!(!*e.get());
                                e.remove();
                            }
                            hash_map::Entry::Vacant(e) => {
                                e.insert(false);
                            }
                        }
                    }
                }
                for (_, diff) in &mut ndrive_diffs {
                    for &bit in present.bits.keys() {
                        if !ndrive_bits.contains(&bit) {
                            continue;
                        }
                        match diff.bits.entry(bit) {
                            hash_map::Entry::Occupied(e) => {
                                assert!(!*e.get());
                                e.remove();
                            }
                            hash_map::Entry::Vacant(e) => {
                                e.insert(false);
                            }
                        }
                    }
                }
                match edev.chip.kind {
                    ChipKind::Virtex2 => {
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::V2_PDRIVE,
                            IOB_DATA::V2_PDRIVE,
                            pdrive_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::V2_NDRIVE,
                            IOB_DATA::V2_NDRIVE,
                            ndrive_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::V2_SLEW,
                            IOB_DATA::V2_SLEW,
                            slew_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::V2_OUTPUT_MISC,
                            IOB_DATA::V2_OUTPUT_MISC,
                            misc_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                    }
                    ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::V2P_PDRIVE,
                            IOB_DATA::V2P_PDRIVE,
                            pdrive_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::V2P_NDRIVE,
                            IOB_DATA::V2P_NDRIVE,
                            ndrive_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::V2P_SLEW,
                            IOB_DATA::V2P_SLEW,
                            slew_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::V2P_OUTPUT_MISC,
                            IOB_DATA::V2P_OUTPUT_MISC,
                            misc_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                    }
                    ChipKind::Spartan3 => {
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3_PDRIVE,
                            IOB_DATA::S3_PDRIVE,
                            pdrive_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3_NDRIVE,
                            IOB_DATA::S3_NDRIVE,
                            ndrive_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3_SLEW,
                            IOB_DATA::S3_SLEW,
                            slew_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3_OUTPUT_MISC,
                            IOB_DATA::S3_OUTPUT_MISC,
                            misc_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                    }
                    ChipKind::FpgaCore => unreachable!(),
                    ChipKind::Spartan3E => {
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3E_PDRIVE,
                            IOB_DATA::S3E_PDRIVE,
                            pdrive_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3E_NDRIVE,
                            IOB_DATA::S3E_NDRIVE,
                            ndrive_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3E_SLEW,
                            IOB_DATA::S3E_SLEW,
                            slew_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3E_OUTPUT_MISC,
                            IOB_DATA::S3E_OUTPUT_MISC,
                            misc_diffs,
                            &present,
                            OcdMode::ValueOrder,
                        );
                    }
                    ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                        if is_s3a_we {
                            insert_iob_data(
                                ctx,
                                tcid,
                                bslot,
                                bcls::IOB::S3A_PDRIVE,
                                IOB_DATA::S3A_WE_PDRIVE,
                                pdrive_diffs,
                                &present,
                                OcdMode::ValueOrder,
                            );
                            insert_iob_data(
                                ctx,
                                tcid,
                                bslot,
                                bcls::IOB::S3A_NDRIVE,
                                IOB_DATA::S3A_WE_NDRIVE,
                                ndrive_diffs,
                                &present,
                                OcdMode::ValueOrder,
                            );
                        } else {
                            insert_iob_data(
                                ctx,
                                tcid,
                                bslot,
                                bcls::IOB::S3A_PDRIVE,
                                IOB_DATA::S3A_SN_PDRIVE,
                                pdrive_diffs,
                                &present,
                                OcdMode::ValueOrder,
                            );
                            insert_iob_data(
                                ctx,
                                tcid,
                                bslot,
                                bcls::IOB::S3A_NDRIVE,
                                IOB_DATA::S3A_SN_NDRIVE,
                                ndrive_diffs,
                                &present,
                                OcdMode::ValueOrder,
                            );
                        }
                        insert_iob_data(
                            ctx,
                            tcid,
                            bslot,
                            bcls::IOB::S3A_PSLEW,
                            IOB_DATA::S3A_PSLEW,
                            pslew_diffs,
                            &present,
                            OcdMode::FixedOrder(&pslew_bits),
                        );
                        let item = xlat_enum_raw(nslew_diffs, OcdMode::FixedOrder(&nslew_bits));
                        let abits = Vec::from_iter(item.bits.iter().map(|&bit| PolTileBit {
                            bit,
                            inv: present.bits.contains_key(&bit),
                        }));
                        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::S3A_NSLEW, abits);
                        for ((field, row), value) in item.values {
                            ctx.insert_table_bitvec(tables::IOB_DATA, row, field, value);
                        }
                    }
                }

                // True differential output path.
                if let IobDiff::True(bslot_n) = iob.diff
                    && !is_s3a_we
                {
                    let mut group_diff = None;
                    if edev.chip.kind.is_spartan3ea() {
                        let base = ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_DIFFO,
                            IOB_DATA::RSDS_25,
                        );
                        let alt = ctx.peek_diff_bel_special_row(
                            tcid,
                            bslot,
                            specials::IOB_DIFFO_ALT,
                            IOB_DATA::RSDS_25,
                        );
                        let diff = alt.combine(&!base);
                        group_diff = Some(diff.clone());
                        let mut bits = xlat_bit_wide(diff);
                        match iob_data.edge {
                            Dir::W => {
                                bits.sort_by_key(|bit| (bit.bit.rect, bit.bit.bit));
                            }
                            Dir::E => {
                                bits.sort_by_key(|bit| {
                                    (
                                        core::cmp::Reverse(bit.bit.rect),
                                        core::cmp::Reverse(bit.bit.bit),
                                    )
                                });
                            }
                            Dir::S => {
                                bits.sort_by_key(|bit| {
                                    (core::cmp::Reverse(bit.bit.rect), bit.bit.frame)
                                });
                            }
                            Dir::N => {
                                bits.sort_by_key(|bit| {
                                    (bit.bit.rect, core::cmp::Reverse(bit.bit.frame))
                                });
                            }
                        }
                        ctx.insert_bel_attr_bool(
                            tcid,
                            bslot,
                            bcls::IOB::OUTPUT_DIFF_GROUP,
                            bits[1],
                        );
                        ctx.insert_bel_attr_bool(
                            tcid,
                            bslot_n,
                            bcls::IOB::OUTPUT_DIFF_GROUP,
                            bits[0],
                        );
                    }
                    let mut diffs = vec![(IOB_DATA::OFF, Diff::default())];
                    if edev.chip.kind.is_virtex2p() {
                        diffs.push((
                            IOB_DATA::DIFF_TERM,
                            ctx.peek_diff_bel_special_row(
                                tcid,
                                bslot_n,
                                specials::IOB_ISTD_COMP_DT,
                                IOB_DATA::LVDS_25,
                            )
                            .clone(),
                        ));
                    } else if edev.chip.kind.is_spartan3ea() {
                        diffs.push((
                            IOB_DATA::DIFF_TERM,
                            ctx.get_diff_bel_special(tcid, bslot, specials::IOB_DIFF_TERM),
                        ));
                    }

                    for std in get_iostds(edev, is_s3a_we) {
                        let rid = iostd_to_row(edev, &std);
                        if std.diff != DiffKind::True {
                            continue;
                        }
                        let mut diff =
                            ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_DIFFO, rid);
                        if edev.chip.kind.is_spartan3ea() {
                            let mut altdiff = ctx.get_diff_bel_special_row(
                                tcid,
                                bslot,
                                specials::IOB_DIFFO_ALT,
                                rid,
                            );
                            if edev.chip.kind == ChipKind::Spartan3E && std.name == "LVDS_25" {
                                assert_eq!(diff, altdiff);
                                diff = diff.combine(&!group_diff.as_ref().unwrap());
                            } else {
                                altdiff = altdiff.combine(&!group_diff.as_ref().unwrap());
                                assert_eq!(diff, altdiff);
                            }
                        }
                        diffs.push((rid, diff));
                    }
                    let item = xlat_enum_raw(diffs, OcdMode::ValueOrder);
                    let item_bv = item.bits.iter().map(|&bit| bit.pos()).collect();
                    let (attr, field) = match edev.chip.kind {
                        ChipKind::Virtex2 => (bcls::IOB::V2_OUTPUT_DIFF, IOB_DATA::V2_OUTPUT_DIFF),
                        ChipKind::Virtex2P | ChipKind::Virtex2PX => {
                            (bcls::IOB::V2P_OUTPUT_DIFF, IOB_DATA::V2P_OUTPUT_DIFF)
                        }
                        ChipKind::Spartan3 => (bcls::IOB::S3_OUTPUT_DIFF, IOB_DATA::S3_OUTPUT_DIFF),
                        ChipKind::FpgaCore => unreachable!(),
                        ChipKind::Spartan3E => {
                            (bcls::IOB::S3E_OUTPUT_DIFF, IOB_DATA::S3E_OUTPUT_DIFF)
                        }
                        ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                            (bcls::IOB::S3A_OUTPUT_DIFF, IOB_DATA::S3A_OUTPUT_DIFF)
                        }
                    };
                    ctx.insert_bel_attr_bitvec(tcid, bslot, attr, item_bv);
                    for (row, value) in item.values {
                        ctx.insert_table_bitvec(IOB_DATA, row, field, value);
                    }
                }
            }
            if iob.kind == IobKind::Ibuf {
                let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IOB_MODE_IBUF);
                diff.discard_bits(&ctx.bel_attr_enum(tcid, bslot, bcls::IOB::PULL).bits);
                if edev.chip.kind.is_spartan3a() {
                    diff.assert_empty();
                } else {
                    // ???
                    ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::IBUF_ENABLE, xlat_bit(diff));
                }
            }
            if has_any_brefclk(edev, tcid, iob.iob).is_some() {
                ctx.collect_bel_attr(tcid, bslot, bcls::IOB::BREFCLK);
            }
        }
        // second loop for stuff involving inter-bel dependencies
        for &iob in &iob_data.iobs {
            let bslot = iob.iob;
            if iob.kind == IobKind::Clk {
                continue;
            }
            for std in get_iostds(edev, is_s3a_we) {
                let rid = iostd_to_row(edev, &std);
                if std.diff != DiffKind::None && iob.diff == IobDiff::None {
                    continue;
                }
                if std.diff != DiffKind::None
                    && matches!(
                        std.dci,
                        DciKind::InputSplit | DciKind::InputVcc | DciKind::BiSplit | DciKind::BiVcc
                    )
                {
                    continue;
                }
                let attrs = if (std.name.starts_with("LVCMOS") || std.name.starts_with("LVTTL"))
                    && edev.chip.kind.is_spartan3a()
                {
                    &[specials::IOB_ISTD_2V5, specials::IOB_ISTD_3V3][..]
                } else if std.name.starts_with("DIFF_") {
                    &[specials::IOB_ISTD_DIFF][..]
                } else {
                    &[specials::IOB_ISTD][..]
                };
                for &attr in attrs {
                    if iostd_is_dt(&std) {
                        continue;
                    }
                    let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, attr, rid);
                    if edev.chip.kind.is_spartan3a() && std.name.starts_with("PCI") {
                        diff.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslot, bcls::IOB::PCI_INPUT),
                            true,
                            false,
                        );
                        if iob.kind != IobKind::Ibuf {
                            diff.apply_bit_diff(
                                ctx.bel_attr_bit(tcid, bslot, bcls::IOB::PCI_CLAMP),
                                true,
                                false,
                            );
                        }
                    }
                    let ibuf_mode = if std.diff != DiffKind::None {
                        enums::IOB_IBUF_MODE::DIFF
                    } else if std.vref.is_some() {
                        enums::IOB_IBUF_MODE::VREF
                    } else if edev.chip.kind.is_spartan3a() {
                        let vcco = std.vcco.unwrap();
                        if vcco < 2500 {
                            enums::IOB_IBUF_MODE::CMOS_VCCINT
                        } else if std.name.starts_with("PCI")
                            || (std.name == "LVCMOS25" && attr == specials::IOB_ISTD_3V3)
                        {
                            enums::IOB_IBUF_MODE::CMOS_VCCO
                        } else {
                            enums::IOB_IBUF_MODE::CMOS_VCCAUX
                        }
                    } else if edev.chip.kind == ChipKind::Spartan3E {
                        let vcco = std.vcco.unwrap();
                        if vcco < 2500 {
                            enums::IOB_IBUF_MODE::CMOS_LV
                        } else {
                            enums::IOB_IBUF_MODE::CMOS_HV
                        }
                    } else {
                        enums::IOB_IBUF_MODE::CMOS
                    };
                    diff.apply_enum_diff(
                        ctx.bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE),
                        ibuf_mode,
                        enums::IOB_IBUF_MODE::NONE,
                    );
                    let dci_mode = match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => {
                            enums::IOB_DCI_MODE::NONE
                        }
                        DciKind::InputVcc | DciKind::BiVcc => enums::IOB_DCI_MODE::TERM_VCC,
                        DciKind::InputSplit | DciKind::BiSplit => enums::IOB_DCI_MODE::TERM_SPLIT,
                        _ => unreachable!(),
                    };
                    if dci_mode != enums::IOB_DCI_MODE::NONE {
                        diff.apply_enum_diff(
                            ctx.bel_attr_enum(tcid, bslot, bcls::IOB::DCI_MODE),
                            dci_mode,
                            enums::IOB_DCI_MODE::NONE,
                        );
                    }
                    if edev.chip.kind == ChipKind::Spartan3E
                        && std.name == "LVDS_25"
                        && iob.kind != IobKind::Ibuf
                    {
                        diff.discard_bits(&[ctx
                            .bel_attr_bit(tcid, bslot, bcls::IOB::OUTPUT_DIFF_GROUP)
                            .bit]);
                        let bslot_other = match iob.diff {
                            IobDiff::None => unreachable!(),
                            IobDiff::True(other) => other,
                            IobDiff::Comp(other) => other,
                        };
                        diff.discard_bits(&[ctx
                            .bel_attr_bit(tcid, bslot_other, bcls::IOB::OUTPUT_DIFF_GROUP)
                            .bit]);
                    }
                    diff.assert_empty();
                }
                if std.diff != DiffKind::None {
                    let mut diff = ctx.get_diff_bel_special_row(
                        tcid,
                        bslot,
                        if iostd_is_dt(&std) {
                            specials::IOB_ISTD_COMP_DT
                        } else {
                            specials::IOB_ISTD_COMP
                        },
                        rid,
                    );
                    if std.diff == DiffKind::TrueTerm
                        && let IobDiff::Comp(bslot_p) = iob.diff
                    {
                        diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot_p, attr_output_diff));
                    }
                    if matches!(edev.chip.kind, ChipKind::Spartan3 | ChipKind::Spartan3E) {
                        diff.discard_bits(
                            &ctx.bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE).bits,
                        );
                    }
                    if edev.chip.kind == ChipKind::Spartan3E
                        && std.name == "LVDS_25"
                        && iob.kind != IobKind::Ibuf
                    {
                        diff.discard_bits(&[ctx
                            .bel_attr_bit(tcid, bslot, bcls::IOB::OUTPUT_DIFF_GROUP)
                            .bit]);
                        let bslot_other = match iob.diff {
                            IobDiff::None => unreachable!(),
                            IobDiff::True(other) => other,
                            IobDiff::Comp(other) => other,
                        };
                        diff.discard_bits(&[ctx
                            .bel_attr_bit(tcid, bslot_other, bcls::IOB::OUTPUT_DIFF_GROUP)
                            .bit]);
                    }
                    diff.assert_empty();
                }
            }
        }
    }
    if edev.chip.kind == ChipKind::Virtex2PX {
        for (tcid, tcid_src, ioi, iob) in [
            (
                tcls_v2::IOB_V2P_SE2_CLK,
                tcls_v2::IOB_V2P_SE2,
                bslots::IOI[2],
                bslots::IOB[1],
            ),
            (
                tcls_v2::IOB_V2P_NE2_CLK,
                tcls_v2::IOB_V2P_NE2,
                bslots::IOI[0],
                bslots::IOB[5],
            ),
        ] {
            let mut diff = ctx.get_diff_bel_special(tcid, ioi, specials::IOB_CLK_ENABLE);
            let mut ibuf_mode = ctx
                .bel_attr_enum(tcid_src, iob, bcls::IOB::IBUF_MODE)
                .clone();
            diff.apply_enum_diff(
                &ibuf_mode,
                enums::IOB_IBUF_MODE::DIFF,
                enums::IOB_IBUF_MODE::NONE,
            );
            diff.assert_empty();
            for val in [enums::IOB_IBUF_MODE::VREF, enums::IOB_IBUF_MODE::CMOS] {
                ibuf_mode.values.remove(val);
            }
            ctx.insert_bel_attr_enum(tcid, iob, bcls::IOB::IBUF_MODE, ibuf_mode);
        }
    }
}
