use std::collections::{HashMap, HashSet, hash_map};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelSlotId, CellSlotId},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{
    Diff, FuzzerProp, OcdMode, enum_ocd_swap_bits, xlat_bit, xlat_bit_wide, xlat_bitvec, xlat_enum,
    xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{
    Bond, Device, ExpandedBond, ExpandedDevice, ExpandedNamedDevice, GeomDb,
};
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex2::{
    bels,
    chip::{ChipKind, IoDiffKind},
    iob::{IobData, IobDiff, IobKind, get_iob_data},
    tslots,
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
        Some(cell.tile(tslots::BEL))
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
        Some(tcrd.with_col(edev.chip.col_clk).tile(tslots::CLK))
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
        backend: &IseBackend<'b>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let id = &mut fuzzer.info.features[0].id;
        assert_eq!(id.bel, *backend.edev.db.bel_slots.key(self.0.bel));
        id.bel = format!("IOB{}", self.0.index);
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
                Box::new(Related::new_boxed(IobRelation(iob.tile), prop))
            })
            .collect();
        self.prop(Iobify(iob)).commit()
    }
}

fn has_any_vref<'a>(
    edev: &prjcombine_virtex2::expanded::ExpandedDevice,
    device: &'a Device,
    db: &GeomDb,
    tile: &str,
    iob_idx: usize,
) -> Option<&'a str> {
    let tcls = edev.db.get_tile_class(tile);
    let iobs = get_iob_data(tile).iobs;
    let ioi_cell = iobs[iob_idx].tile;
    let ioi_bel = iobs[iob_idx].bel;
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
    for &tcrd in &edev.tile_index[tcls] {
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
    tile: &str,
    iob_idx: usize,
) -> Option<(&'a str, Option<bool>)> {
    let tcls = edev.db.get_tile_class(tile);
    let iobs = get_iob_data(tile).iobs;
    let ioi_cell = iobs[iob_idx].tile;
    let ioi_bel = iobs[iob_idx].bel;
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
    for &tcrd in &edev.tile_index[tcls] {
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
    tile: &str,
    iob_idx: usize,
) -> Option<(usize, usize)> {
    if edev.chip.kind != ChipKind::Virtex2P {
        return None;
    }
    match (tile, iob_idx) {
        ("IOBS.V2P.B.L2", 5) => Some((1, 0)),
        ("IOBS.V2P.B.R2", 1) => Some((0, 6)),
        ("IOBS.V2P.T.L2", 1) => Some((1, 2)),
        ("IOBS.V2P.T.R2", 5) => Some((0, 4)),
        _ => None,
    }
}

#[allow(clippy::nonminimal_bool)]
pub fn get_iostds(edev: &prjcombine_virtex2::expanded::ExpandedDevice, lr: bool) -> Vec<Iostd> {
    let mut res = vec![];
    // plain push-pull
    if edev.chip.kind.is_virtex2() {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else if edev.chip.kind == ChipKind::Spartan3 {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12"]),
            Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
        ]);
    } else if edev.chip.kind == ChipKind::Spartan3E {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6"]),
            Iostd::cmos("LVCMOS12", 1200, &["2"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else if lr {
        // spartan3a lr
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12"]),
            Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else {
        // spartan3a tb
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6"]),
            Iostd::cmos("LVCMOS12", 1200, &["2"]),
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
    if edev.chip.kind != ChipKind::Spartan3E && !(edev.chip.kind.is_spartan3a() && !lr) {
        res.push(Iostd::vref("SSTL2_II", 2500, 1250));
        res.push(Iostd::vref("SSTL18_II", 1800, 900));
    }
    res.push(Iostd::vref("HSTL_I_18", 1800, 900));
    if edev.chip.kind != ChipKind::Spartan3E && !(edev.chip.kind.is_spartan3a() && !lr) {
        res.push(Iostd::vref("HSTL_II_18", 1800, 900));
    }
    res.push(Iostd::vref("HSTL_III_18", 1800, 1100));
    if edev.chip.kind.is_virtex2() {
        res.push(Iostd::vref("HSTL_IV_18", 1800, 1100));
    }
    if edev.chip.kind != ChipKind::Spartan3E && !(edev.chip.kind.is_spartan3a() && !lr) {
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
    if edev.chip.kind != ChipKind::Spartan3E && !(edev.chip.kind.is_spartan3a() && !lr) {
        res.push(Iostd::pseudo_diff("DIFF_SSTL2_II", 2500));
    }
    if edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::pseudo_diff("DIFF_SSTL18_I", 1800));
    }
    if edev.chip.kind.is_virtex2() || (edev.chip.kind.is_spartan3a() && lr) {
        res.push(Iostd::pseudo_diff("DIFF_SSTL18_II", 1800));
    }
    if edev.chip.kind.is_spartan3ea() {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800));
        res.push(Iostd::pseudo_diff("DIFF_HSTL_III_18", 1800));
    }
    if !edev.chip.kind.is_spartan3ea() || (edev.chip.kind.is_spartan3a() && lr) {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800));
    }
    if edev.chip.kind.is_spartan3a() && lr {
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
    for (_, name, tcls) in &intdb.tile_classes {
        if !name.starts_with("IOI") {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, name) else {
            continue;
        };
        for bel in tcls.bels.ids() {
            let Some(idx) = bels::IO.into_iter().position(|x| x == bel) else {
                continue;
            };
            if name == "IOI.CLK_T" && matches!(idx, 0 | 1) {
                continue;
            }
            if name == "IOI.CLK_B" && matches!(idx, 2 | 3) {
                continue;
            }
            let mut bctx = ctx.bel(bel);
            let mode = if edev.chip.kind.is_spartan3ea() {
                "IBUF"
            } else {
                "IOB"
            };

            // clock & SR invs
            bctx.mode(mode).attr("OFF1", "#FF").test_inv("OTCLK1");
            bctx.mode(mode).attr("OFF2", "#FF").test_inv("OTCLK2");
            bctx.mode(mode).attr("IFF1", "#FF").test_inv("ICLK1");
            bctx.mode(mode).attr("IFF2", "#FF").test_inv("ICLK2");
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OSR_USED", "0")
                .test_inv("SR");
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OREV_USED", "0")
                .test_inv("REV");
            // SR & rev enables
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("OFF1", "#FF")
                .attr("OSR_USED", "0")
                .attr("SRINV", "SR_B")
                .pin("SR")
                .test_enum("ISR_USED", &["0"]);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("OFF1", "#FF")
                .attr("ISR_USED", "0")
                .attr("SRINV", "SR_B")
                .pin("SR")
                .test_enum("OSR_USED", &["0"]);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("TFF1", "#FF")
                .attr("ISR_USED", "0")
                .attr("SRINV", "SR_B")
                .pin("SR")
                .test_enum("TSR_USED", &["0"]);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("OFF1", "#FF")
                .attr("OREV_USED", "0")
                .attr("REVINV", "REV_B")
                .pin("REV")
                .test_enum("IREV_USED", &["0"]);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("OFF1", "#FF")
                .attr("IREV_USED", "0")
                .attr("REVINV", "REV_B")
                .pin("REV")
                .test_enum("OREV_USED", &["0"]);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("TFF1", "#FF")
                .attr("IREV_USED", "0")
                .attr("REVINV", "REV_B")
                .pin("REV")
                .test_enum("TREV_USED", &["0"]);

            // CE
            bctx.mode(mode).attr("IFF1", "#FF").test_inv("ICE");
            bctx.mode(mode).attr("TFF1", "#FF").test_inv("TCE");
            if edev.chip.kind.is_spartan3ea() {
                bctx.mode(mode)
                    .attr("OFF1", "#FF")
                    .attr("PCICE_MUX", "OCE")
                    .test_inv("OCE");
                bctx.mode(mode)
                    .attr("OFF1", "#FF")
                    .attr("OCEINV", "#OFF")
                    .pin("OCE")
                    .pin("PCI_CE")
                    .test_enum("PCICE_MUX", &["OCE", "PCICE"]);
            } else {
                bctx.mode(mode).attr("OFF1", "#FF").test_inv("OCE");
            }
            // Output path
            if edev.chip.kind.is_spartan3ea() {
                bctx.mode(mode)
                    .attr("O1_DDRMUX", "1")
                    .attr("OFF1", "#FF")
                    .attr("OMUX", "OFF1")
                    .test_inv("O1");
                bctx.mode(mode)
                    .attr("O2_DDRMUX", "1")
                    .attr("OFF2", "#FF")
                    .attr("OMUX", "OFF2")
                    .test_inv("O2");
            } else {
                bctx.mode(mode)
                    .attr("OFF1", "#FF")
                    .attr("OMUX", "OFF1")
                    .test_inv("O1");
                bctx.mode(mode)
                    .attr("OFF2", "#FF")
                    .attr("OMUX", "OFF2")
                    .test_inv("O2");
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
                .test_inv("T1");
            bctx.mode(mode)
                .attr("T_USED", "0")
                .attr("TFF1", "#OFF")
                .attr("TFF2", "#FF")
                .attr("TMUX", "TFF2")
                .attr("OFF1", "#OFF")
                .attr("OFF2", "#OFF")
                .attr("OMUX", "#OFF")
                .pin("T")
                .test_inv("T2");
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
                .test_enum("TMUX", &["T1", "T2", "TFF1", "TFF2", "TFFDDR"]);
            // hack to avoid dragging IOB into it.
            for val in ["O1", "O2", "OFF1", "OFF2", "OFFDDR"] {
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
                        .test_manual("OMUX", val)
                        .attr_diff("OMUX", "OFFDDR", val)
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
                        .test_manual("OMUX", val)
                        .attr_diff("OMUX", "OFFDDR", val)
                        .commit();
                    if idx != 2 {
                        let obel = bels::IO[idx ^ 1];
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
                            .test_enum("O1_DDRMUX", &["0", "1"]);
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
                            .test_enum("O2_DDRMUX", &["0", "1"]);
                    }
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
                        .test_manual("OMUX", val)
                        .attr_diff("OMUX", "OFFDDR", val)
                        .commit();
                    if idx != 2 {
                        let obel = bels::IO[idx ^ 1];
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
                            .test_enum("O1_DDRMUX", &["0", "1"]);
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
                            .test_enum("O2_DDRMUX", &["0", "1"]);
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
                    .test_enum("OFF1", &["#FF", "#LATCH"]);
                bctx.mode(mode)
                    .attr("OFF1", "#OFF")
                    .attr("OCEINV", "OCE_B")
                    .attr("OFF2_INIT_ATTR", "INIT1")
                    .pin("OCE")
                    .test_enum("OFF2", &["#FF", "#LATCH"]);
            } else {
                bctx.mode(mode)
                    .attr("OFF2", "#OFF")
                    .attr("OCEINV", "OCE_B")
                    .attr("PCICE_MUX", "OCE")
                    .attr("OFF1_INIT_ATTR", "INIT1")
                    .pin("OCE")
                    .test_enum("OFF1", &["#FF", "#LATCH"]);
                bctx.mode(mode)
                    .attr("OFF1", "#OFF")
                    .attr("OCEINV", "OCE_B")
                    .attr("PCICE_MUX", "OCE")
                    .attr("OFF2_INIT_ATTR", "INIT1")
                    .pin("OCE")
                    .test_enum("OFF2", &["#FF", "#LATCH"]);
            }
            bctx.mode(mode)
                .attr("TFF2", "#OFF")
                .attr("TCEINV", "TCE_B")
                .attr("TFF1_INIT_ATTR", "INIT1")
                .pin("TCE")
                .test_enum("TFF1", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("TFF1", "#OFF")
                .attr("TCEINV", "TCE_B")
                .attr("TFF2_INIT_ATTR", "INIT1")
                .pin("TCE")
                .test_enum("TFF2", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OFF1_INIT_ATTR", "INIT0")
                .test_enum("OFF1_SR_ATTR", &["SRLOW", "SRHIGH"]);
            bctx.mode(mode)
                .attr("OFF2", "#FF")
                .attr("OFF2_INIT_ATTR", "INIT0")
                .test_enum("OFF2_SR_ATTR", &["SRLOW", "SRHIGH"]);
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .attr("TFF1_INIT_ATTR", "INIT0")
                .test_enum("TFF1_SR_ATTR", &["SRLOW", "SRHIGH"]);
            bctx.mode(mode)
                .attr("TFF2", "#FF")
                .attr("TFF2_INIT_ATTR", "INIT0")
                .test_enum("TFF2_SR_ATTR", &["SRLOW", "SRHIGH"]);
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OFF2", "#FF")
                .attr("OFF1_SR_ATTR", "SRHIGH")
                .attr("OFF2_SR_ATTR", "SRHIGH")
                .attr("OFF2_INIT_ATTR", "#OFF")
                .test_enum("OFF1_INIT_ATTR", &["INIT0", "INIT1"]);
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OFF2", "#FF")
                .attr("OFF1_SR_ATTR", "SRHIGH")
                .attr("OFF2_SR_ATTR", "SRHIGH")
                .attr("OFF1_INIT_ATTR", "#OFF")
                .test_enum("OFF2_INIT_ATTR", &["INIT0", "INIT1"]);
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .attr("TFF2", "#FF")
                .attr("TFF1_SR_ATTR", "SRHIGH")
                .attr("TFF2_SR_ATTR", "SRHIGH")
                .attr("TFF2_INIT_ATTR", "#OFF")
                .test_enum("TFF1_INIT_ATTR", &["INIT0", "INIT1"]);
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .attr("TFF2", "#FF")
                .attr("TFF1_SR_ATTR", "SRHIGH")
                .attr("TFF2_SR_ATTR", "SRHIGH")
                .attr("TFF1_INIT_ATTR", "#OFF")
                .test_enum("TFF2_INIT_ATTR", &["INIT0", "INIT1"]);
            bctx.mode(mode)
                .attr("OFF1", "#FF")
                .attr("OFF2", "#FF")
                .test_enum("OFFATTRBOX", &["SYNC", "ASYNC"]);
            bctx.mode(mode)
                .attr("TFF1", "#FF")
                .attr("TFF2", "#FF")
                .test_enum("TFFATTRBOX", &["SYNC", "ASYNC"]);

            // Input flops
            bctx.mode(mode)
                .attr("IFF2", "#OFF")
                .attr("ICEINV", "ICE_B")
                .attr("IFF1_INIT_ATTR", "INIT1")
                .pin("ICE")
                .test_enum("IFF1", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("IFF1", "#OFF")
                .attr("ICEINV", "ICE_B")
                .attr("IFF2_INIT_ATTR", "INIT1")
                .pin("ICE")
                .test_enum("IFF2", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("IFF1_INIT_ATTR", "INIT0")
                .test_enum("IFF1_SR_ATTR", &["SRLOW", "SRHIGH"]);
            bctx.mode(mode)
                .attr("IFF2", "#FF")
                .attr("IFF2_INIT_ATTR", "INIT0")
                .test_enum("IFF2_SR_ATTR", &["SRLOW", "SRHIGH"]);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("IFF1_SR_ATTR", "SRHIGH")
                .test_enum("IFF1_INIT_ATTR", &["INIT0", "INIT1"]);
            bctx.mode(mode)
                .attr("IFF2", "#FF")
                .attr("IFF2_SR_ATTR", "SRHIGH")
                .test_enum("IFF2_INIT_ATTR", &["INIT0", "INIT1"]);
            bctx.mode(mode)
                .attr("IFF1", "#FF")
                .attr("IFF2", "#FF")
                .test_enum("IFFATTRBOX", &["SYNC", "ASYNC"]);

            // Input path.
            if edev.chip.kind == ChipKind::Spartan3E {
                bctx.mode(mode)
                    .attr("IFF1", "#FF")
                    .attr("IMUX", "1")
                    .attr("IFFDMUX", "#OFF")
                    .pin("IDDRIN1")
                    .pin("IDDRIN2")
                    .pin("I")
                    .test_enum("IDDRIN_MUX", &["0", "1", "2"]);
            } else if edev.chip.kind.is_spartan3a() {
                bctx.mode(mode)
                    .attr("IFF1", "#FF")
                    .attr("IMUX", "1")
                    .attr("SEL_MUX", "0")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .pin("IDDRIN1")
                    .pin("IDDRIN2")
                    .pin("I")
                    .test_enum("IDDRIN_MUX", &["0", "1"]);
                bctx.mode(mode)
                    .attr("IFF1", "#FF")
                    .attr("IMUX", "1")
                    .attr("SEL_MUX", "0")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .pin("IDDRIN1")
                    .pin("IDDRIN2")
                    .pin("I")
                    .test_manual("IDDRIN_MUX", "2")
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
                        .test_enum("IDELMUX", &["0", "1"]);
                    bctx.mode(mode)
                        .attr("IDELMUX", "0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .pin("I")
                        .test_enum("IFFDELMUX", &["0", "1"]);
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
                        .test_enum("IMUX", &["0", "1"]);
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
                        .test_enum("IFFDMUX", &["0", "1"]);
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
                        .test_enum("IDELMUX", &["0", "1"]);
                    bctx.mode(mode)
                        .attr("IDELMUX", "0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("IBUF_DELAY_VALUE", "DLY4")
                        .attr("PRE_DELAY_MUX", "0")
                        .pin("I")
                        .test_enum("IFFDELMUX", &["0", "1"]);
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
                        .test_enum("IMUX", &["0", "1"]);
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
                        .test_enum("IFFDMUX", &["0", "1"]);
                }
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
                    .test_enum("TSMUX", &["0", "1"]);
            } else {
                if name.ends_with("T") || name.ends_with("B") {
                    bctx.mode(mode)
                        .attr("IFD_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .attr("SEL_MUX", "0")
                        .pin("I")
                        .test_enum("IBUF_DELAY_VALUE", &["DLY0", "DLY16"]);
                    bctx.mode(mode)
                        .attr("IBUF_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .attr("SEL_MUX", "0")
                        .pin("I")
                        .test_enum("IFD_DELAY_VALUE", &["DLY0", "DLY8"]);
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
                        .test_enum(
                            "IBUF_DELAY_VALUE",
                            &[
                                "DLY0", "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7",
                                "DLY8", "DLY9", "DLY10", "DLY11", "DLY12", "DLY13", "DLY14",
                                "DLY15", "DLY16",
                            ],
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
                        .test_enum(
                            "IFD_DELAY_VALUE",
                            &[
                                "DLY0", "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7",
                                "DLY8",
                            ],
                        );
                    bctx.mode(mode)
                        .attr("IBUF_DELAY_VALUE", "DLY16")
                        .attr("IFD_DELAY_VALUE", "DLY8")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("SEL_MUX", "0")
                        .pin("I")
                        .test_manual("DELAY_ADJ_ATTRBOX", "VARIABLE")
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
                    .test_enum("IMUX", &["0", "1"]);
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
                    .test_enum("IFFDMUX", &["0", "1"]);
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
                    .test_enum("TSMUX", &["0", "1"]);
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
                    .test_manual("MISR_ENABLE", "1")
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
                        .test_manual("MISR_ENABLE_OTCLK1", "1")
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
                        .test_manual("MISR_ENABLE_OTCLK2", "1")
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
                        .test_manual("MISR_ENABLE_RESET", "1")
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
                        .test_manual("MISR_ENABLE_OTCLK1", "1")
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
                        .test_manual("MISR_ENABLE_OTCLK2", "1")
                        .attr("MISRATTRBOX", "ENABLE_MISR")
                        .commit();
                }
            }
        }
    }

    // IOB
    for tile in intdb.tile_classes.keys() {
        let is_s3a_lr = tile.starts_with("IOBS.S3A.L") || tile.starts_with("IOBS.S3A.R");
        if !tile.starts_with("IOB") {
            continue;
        }
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let iob_data = get_iob_data(tile);
        for &iob in &iob_data.iobs {
            let ioi = bels::IO.into_iter().position(|x| x == iob.bel).unwrap();
            let ibuf_mode = if edev.chip.kind.is_spartan3ea() {
                "IBUF"
            } else {
                "IOB"
            };
            let diffi_mode = match iob.diff {
                IobDiff::None => None,
                IobDiff::True(_) => Some(if edev.chip.kind.is_spartan3a() && is_s3a_lr {
                    "DIFFMI_NDT"
                } else if edev.chip.kind.is_spartan3ea() {
                    "DIFFMI"
                } else {
                    "DIFFM"
                }),
                IobDiff::Comp(_) => Some(if edev.chip.kind.is_spartan3a() && is_s3a_lr {
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
            let mut bctx = ctx.bel(iob.bel);
            if iob.kind != IobKind::Ibuf {
                bctx.build()
                    .global_mutex("VREF", "NO")
                    .global_mutex("DCI", "NO")
                    .test_manual("PRESENT", "IOB")
                    .mode("IOB")
                    .iob_commit(iob);
            }
            if edev.chip.kind.is_spartan3ea() {
                bctx.build()
                    .global_mutex("VREF", "NO")
                    .test_manual("PRESENT", "IBUF")
                    .mode("IBUF")
                    .iob_commit(iob);
            }
            for val in ["PULLUP", "PULLDOWN", "KEEPER"] {
                bctx.mode(ibuf_mode)
                    .attr("IMUX", "1")
                    .pin("I")
                    .test_manual("PULL", val)
                    .attr("PULL", val)
                    .iob_commit(iob);
            }
            bctx.mode(ibuf_mode)
                .test_manual("GTSATTRBOX", "DISABLE_GTS")
                .attr("GTSATTRBOX", "DISABLE_GTS")
                .iob_commit(iob);
            if edev.chip.kind.is_spartan3a() && iob.kind != IobKind::Ibuf {
                for val in [
                    "DRIVE_LAST_VALUE",
                    "3STATE",
                    "3STATE_PULLUP",
                    "3STATE_PULLDOWN",
                    "3STATE_KEEPER",
                ] {
                    bctx.mode("IOB")
                        .test_manual("SUSPEND", val)
                        .attr("SUSPEND", val)
                        .iob_commit(iob);
                }
            }
            if edev.chip.kind == ChipKind::Spartan3E {
                for val in [
                    "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7", "DLY8", "DLY9",
                    "DLY10", "DLY11", "DLY12", "DLY13", "DLY14", "DLY15", "DLY16",
                ] {
                    bctx.mode("IBUF")
                        .attr("IFD_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .pin("I")
                        .test_manual("IBUF_DELAY_VALUE", val)
                        .attr_diff("IBUF_DELAY_VALUE", "DLY0", val)
                        .iob_commit(iob);
                }
                for val in [
                    "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7", "DLY8",
                ] {
                    bctx.mode("IBUF")
                        .attr("IBUF_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .pin("I")
                        .test_manual("IFD_DELAY_VALUE", val)
                        .attr_diff("IFD_DELAY_VALUE", "DLY0", val)
                        .iob_commit(iob);
                }
            }
            if edev.chip.kind.is_spartan3a() && !is_s3a_lr {
                for val in [
                    "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7", "DLY8", "DLY9",
                    "DLY10", "DLY11", "DLY12", "DLY13", "DLY14", "DLY15", "DLY16",
                ] {
                    bctx.mode("IBUF")
                        .attr("IFD_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .attr("SEL_MUX", "0")
                        .pin("I")
                        .test_manual("IBUF_DELAY_VALUE", val)
                        .attr_diff("IBUF_DELAY_VALUE", "DLY16", val)
                        .iob_commit(iob);
                }
                for val in [
                    "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7", "DLY8",
                ] {
                    bctx.mode("IBUF")
                        .attr("IBUF_DELAY_VALUE", "DLY0")
                        .attr("IMUX", "1")
                        .attr("IFFDMUX", "1")
                        .attr("IFF1", "#FF")
                        .attr("IDDRIN_MUX", "2")
                        .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                        .attr("SEL_MUX", "0")
                        .pin("I")
                        .test_manual("IFD_DELAY_VALUE", val)
                        .attr_diff("IFD_DELAY_VALUE", "DLY8", val)
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
                    .test_manual("DELAY_ADJ_ATTRBOX", "VARIABLE")
                    .attr_diff("DELAY_ADJ_ATTRBOX", "FIXED", "VARIABLE")
                    .iob_commit(iob);
            }

            // Input path.
            for std in get_iostds(edev, is_s3a_lr) {
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
                            Some(Box::new(OtherIobDiffOutput(iob.bel, std.name.to_string())))
                        } else {
                            None
                        }
                    } else if std.vref.is_some() || is_input_dci {
                        Some(Box::new(OtherIobInput(iob.bel, std.name.to_string())))
                    } else {
                        None
                    };
                    let attr = match vccaux {
                        "2.5" => "ISTD.2.5",
                        "3.3" => "ISTD.3.3",
                        _ => "ISTD",
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
                            .test_manual(attr, std.name)
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
                            .test_manual(attr, std.name)
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
                            .test_manual("ISTD.COMP", std.name)
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
                    .extra_tile_attr(
                        NoopRelation,
                        format!("IO{ioi}"),
                        "SEL_MUX",
                        if iob.kind == IobKind::Ibuf {
                            "OMUX_IBUF"
                        } else {
                            "OMUX"
                        },
                    )
                    .test_manual("SEL_MUX", "OMUX")
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
                    .test_manual("SEL_MUX", "TMUX")
                    .attr("IBUF_DELAY_VALUE", "DLY0")
                    .attr("DELAY_ADJ_ATTRBOX", "FIXED")
                    .attr("SEL_MUX", "2")
                    .attr("IMUX", "1")
                    .pin("I")
                    .iob_commit(iob);
            }
            if let Some(pkg) = has_any_vref(edev, backend.device, backend.db, tile, iob.index) {
                bctx.build()
                    .raw(Key::Package, pkg)
                    .global_mutex("VREF", "YES")
                    .prop(IsVref(iob.bel))
                    .prop(OtherIobInput(iob.bel, "SSTL2_I".to_string()))
                    .test_manual("PRESENT", "NOTVREF")
                    .mode(ibuf_mode)
                    .iob_commit(iob);
            }
            if let Some((pkg, alt)) = has_any_vr(edev, backend.device, backend.db, tile, iob.index)
            {
                let mut builder = bctx
                    .build()
                    .raw(Key::Package, pkg)
                    .global_mutex("DCI", "YES");
                if let Some(alt) = alt {
                    builder = builder.raw(Key::AltVr, alt);
                }
                builder
                    .prop(IsVr(iob.bel))
                    .prop(OtherIobInput(iob.bel, "GTL_DCI".to_string()))
                    .test_manual("PRESENT", "NOTVR")
                    .mode(ibuf_mode)
                    .iob_commit(iob);
            }
            if edev.chip.kind.is_spartan3ea()
                && !is_s3a_lr
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
                        .test_manual("DIFF_TERM", "1")
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
                        .test_manual("DIFF_TERM", "1")
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
                    .test_manual("DIFF_TERM.COMP", "1")
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
                    .extra_tile_attr(NoopRelation, format!("IO{ioi}"), "OUTPUT_ENABLE", "1")
                    .test_manual("OUTPUT_ENABLE", "1")
                    .attr("IOATTRBOX", "LVCMOS33")
                    .attr("OMUX", "O1")
                    .attr("O1INV", "O1")
                    .attr("DRIVE_0MA", "DRIVE_0MA")
                    .pin("O1")
                    .iob_commit(iob);
                for std in get_iostds(edev, is_s3a_lr) {
                    if std.input_only {
                        continue;
                    }
                    if matches!(std.diff, DiffKind::True | DiffKind::TrueTerm) && is_s3a_lr {
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
                        (&[""][..], &[""][..])
                    } else {
                        (
                            std.drive,
                            if edev.chip.kind.is_spartan3a() {
                                &["FAST", "SLOW", "QUIETIO"][..]
                            } else {
                                &["FAST", "SLOW"][..]
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
                        for &drive in drives {
                            for &slew in slews {
                                let dci_spec = if std.dci == DciKind::None {
                                    None
                                } else if matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                                    Some(OtherIobInput(iob.bel, "SSTL2_I_DCI".into()))
                                } else if std.diff == DiffKind::None {
                                    Some(OtherIobInput(iob.bel, std.name.to_string()))
                                } else {
                                    // can't be bothered to get it working.
                                    continue;
                                };
                                let name = if vccaux.is_empty() {
                                    if drive.is_empty() {
                                        std.name.to_string()
                                    } else {
                                        format!("{s}.{drive}.{slew}", s = std.name)
                                    }
                                } else {
                                    if drive.is_empty() {
                                        format!("{s}.{vccaux}", s = std.name)
                                    } else {
                                        format!("{s}.{drive}.{slew}.{vccaux}", s = std.name)
                                    }
                                };
                                let mode = if std.diff == DiffKind::None {
                                    if is_s3a_lr { "IOBLR" } else { "IOB" }
                                } else {
                                    match iob.diff {
                                        IobDiff::None => continue,
                                        IobDiff::True(_) => {
                                            if is_s3a_lr {
                                                "DIFFMLR"
                                            } else if edev.chip.kind.is_spartan3a() {
                                                "DIFFMTB"
                                            } else {
                                                "DIFFM"
                                            }
                                        }
                                        IobDiff::Comp(_) => {
                                            if is_s3a_lr {
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
                                    .test_manual("OSTD", name)
                                    .mode_diff("IOB", mode)
                                    .attr_diff("IOATTRBOX", "LVCMOS33", std.name)
                                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                    .attr("DRIVEATTRBOX", drive)
                                    .attr("SLEW", slew)
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
                    let iob_n = iob_data.iobs[other_iob];
                    for std in get_iostds(edev, is_s3a_lr) {
                        if is_s3a_lr {
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
                            .prop(BankDiffOutput(iob.bel, std.name.to_string(), None))
                            .attr("PULL", "PULLDOWN")
                            .bel_attr(iob_n.bel, "PULL", "PULLDOWN")
                            .attr("TMUX", "#OFF")
                            .attr("IMUX", "#OFF")
                            .attr("IFFDMUX", "#OFF")
                            .attr("OMUX", "O1")
                            .attr("O1INV", "O1")
                            .bel_attr(iob_n.bel, "TMUX", "#OFF")
                            .bel_attr(iob_n.bel, "IMUX", "#OFF")
                            .bel_attr(iob_n.bel, "IFFDMUX", "#OFF")
                            .bel_attr(iob_n.bel, "OMUX", "#OFF")
                            .pin("O1")
                            .test_manual("DIFFO", std.name)
                            .mode_diff("IOB", mode_p)
                            .bel_mode_diff(iob_n.bel, "IOB", mode_n)
                            .attr_diff("IOATTRBOX", "LVCMOS33", std.name)
                            .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                            .bel_attr(iob_n.bel, "IOATTRBOX", std.name)
                            .bel_attr(iob_n.bel, "DIFFO_IN_USED", "0")
                            .pin("DIFFO_OUT")
                            .bel_pin(iob_n.bel, "DIFFO_IN")
                            .attr(
                                "SUSPEND",
                                if edev.chip.kind.is_spartan3a() {
                                    "3STATE"
                                } else {
                                    ""
                                },
                            )
                            .bel_attr(
                                iob_n.bel,
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
                                    iob.bel,
                                    altstd.to_string(),
                                    Some(std.name.to_string()),
                                ))
                                .attr("PULL", "PULLDOWN")
                                .bel_attr(iob_n.bel, "PULL", "PULLDOWN")
                                .attr("TMUX", "#OFF")
                                .attr("IMUX", "#OFF")
                                .attr("IFFDMUX", "#OFF")
                                .attr("OMUX", "O1")
                                .attr("O1INV", "O1")
                                .bel_attr(iob_n.bel, "TMUX", "#OFF")
                                .bel_attr(iob_n.bel, "IMUX", "#OFF")
                                .bel_attr(iob_n.bel, "IFFDMUX", "#OFF")
                                .bel_attr(iob_n.bel, "OMUX", "#OFF")
                                .pin("O1")
                                .test_manual("DIFFO.ALT", std.name)
                                .mode_diff("IOB", mode_p)
                                .bel_mode_diff(iob_n.bel, "IOB", mode_n)
                                .attr_diff("IOATTRBOX", "LVCMOS33", std.name)
                                .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                .bel_attr(iob_n.bel, "IOATTRBOX", std.name)
                                .bel_attr(iob_n.bel, "DIFFO_IN_USED", "0")
                                .pin("DIFFO_OUT")
                                .bel_pin(iob_n.bel, "DIFFO_IN")
                                .attr(
                                    "SUSPEND",
                                    if edev.chip.kind.is_spartan3a() {
                                        "3STATE"
                                    } else {
                                        ""
                                    },
                                )
                                .bel_attr(
                                    iob_n.bel,
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
                    for val in ["ASREQUIRED", "CONTINUOUS", "QUIET"] {
                        bctx.mode("IOB")
                            .global("DCIUPDATEMODE", val)
                            .raw(Key::Package, &package.name)
                            .global_mutex("DCI", "UPDATEMODE")
                            .prop(OtherIobInput(iob.bel, "SSTL2_I_DCI".into()))
                            .attr("PULL", "PULLDOWN")
                            .attr("TMUX", "#OFF")
                            .attr("IMUX", "#OFF")
                            .attr("IFFDMUX", "#OFF")
                            .attr("OMUX", "O1")
                            .attr("O1INV", "O1")
                            .pin("O1")
                            .test_manual("DCIUPDATEMODE", val)
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
                        .test_manual("OPROGRAMMING", "")
                        .prop(FuzzBelMultiAttr::new(
                            iob.bel,
                            "OPROGRAMMING".into(),
                            MultiValue::Bin,
                            16,
                        ))
                        .iob_commit(iob);
                }
            }
            if let Some((brefclk, bufg)) = has_any_brefclk(edev, tile, iob.index) {
                let bufg_bel_id = bels::BUFGMUX[bufg];
                let brefclk_bel_id = bels::BREFCLK;
                let brefclk_pin = ["BREFCLK", "BREFCLK2"][brefclk];

                bctx.test_manual("BREFCLK_ENABLE", "1")
                    .related_pip(
                        IobBrefclkClkBT,
                        (brefclk_bel_id, brefclk_pin),
                        (PinFar, bufg_bel_id, "CKI"),
                    )
                    .iob_commit(iob);
            }
        }
        if tile.ends_with("CLK") {
            // Virtex 2 Pro X special!
            let bel_id = bels::BREFCLK_INT;
            let clk_bel_id = bels::IO[if tile == "IOBS.V2P.B.R2.CLK" { 2 } else { 0 }];
            let mut bctx = ctx.bel(bels::BREFCLK_INT);
            bctx.test_manual("ENABLE", "1")
                .related_pip(
                    IobRelation(CellSlotId::from_idx(1)),
                    (bel_id, "BREFCLK"),
                    (PinFar, clk_bel_id, "I"),
                )
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let intdb = ctx.edev.db;

    // IOI
    for (tcid, tile, tcls) in &intdb.tile_classes {
        if !tile.starts_with("IOI") {
            continue;
        }
        if ctx.edev.tile_index[tcid].is_empty() {
            continue;
        }
        let int_tiles = &[match edev.chip.kind {
            ChipKind::Virtex2 | ChipKind::Virtex2P | ChipKind::Virtex2PX => match &tile[..] {
                "IOI.CLK_B" => "INT.IOI.CLK_B",
                "IOI.CLK_T" => "INT.IOI.CLK_T",
                _ => "INT.IOI",
            },
            ChipKind::Spartan3 => "INT.IOI.S3",
            ChipKind::FpgaCore => unreachable!(),
            ChipKind::Spartan3E => "INT.IOI.S3E",
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                if tile == "IOI.S3A.LR" {
                    "INT.IOI.S3A.LR"
                } else {
                    "INT.IOI.S3A.TB"
                }
            }
        }];

        for (slot, _) in &tcls.bels {
            let Some(idx) = bels::IO.into_iter().position(|x| x == slot) else {
                continue;
            };
            if tile == "IOI.CLK_T" && matches!(idx, 0 | 1) {
                continue;
            }
            if tile == "IOI.CLK_B" && matches!(idx, 2 | 3) {
                continue;
            }
            let bel = intdb.bel_slots.key(slot);
            ctx.collect_inv(tile, bel, "OTCLK1");
            ctx.collect_inv(tile, bel, "OTCLK2");
            ctx.collect_inv(tile, bel, "ICLK1");
            ctx.collect_inv(tile, bel, "ICLK2");
            ctx.collect_int_inv(int_tiles, tile, bel, "SR", edev.chip.kind.is_virtex2());
            ctx.collect_int_inv(int_tiles, tile, bel, "OCE", edev.chip.kind.is_virtex2());
            ctx.collect_inv(tile, bel, "REV");
            ctx.collect_inv(tile, bel, "ICE");
            ctx.collect_inv(tile, bel, "TCE");
            let item = ctx.extract_bit(tile, bel, "ISR_USED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_SR_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "OSR_USED", "0");
            ctx.tiledb.insert(tile, bel, "OFF_SR_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "TSR_USED", "0");
            ctx.tiledb.insert(tile, bel, "TFF_SR_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "IREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_REV_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "OREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "OFF_REV_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "TREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "TFF_REV_ENABLE", item);

            if edev.chip.kind.is_spartan3ea() {
                ctx.collect_enum_default(tile, bel, "PCICE_MUX", &["OCE", "PCICE"], "NONE");
            }
            ctx.collect_inv(tile, bel, "O1");
            ctx.collect_inv(tile, bel, "O2");
            ctx.collect_inv(tile, bel, "T1");
            ctx.collect_inv(tile, bel, "T2");
            ctx.collect_enum_default(
                tile,
                bel,
                "TMUX",
                &["T1", "T2", "TFF1", "TFF2", "TFFDDR"],
                "NONE",
            );
            // hack to avoid dragging IOB into it.
            let mut item = xlat_enum(vec![
                ("O1", ctx.state.get_diff(tile, bel, "OMUX", "O1")),
                ("O2", ctx.state.get_diff(tile, bel, "OMUX", "O2")),
                ("OFF1", ctx.state.get_diff(tile, bel, "OMUX", "OFF1")),
                ("OFF2", ctx.state.get_diff(tile, bel, "OMUX", "OFF2")),
                ("OFFDDR", ctx.state.get_diff(tile, bel, "OMUX", "OFFDDR")),
            ]);
            let TileItemKind::Enum { ref mut values } = item.kind else {
                unreachable!()
            };
            values.insert("NONE".into(), BitVec::repeat(false, item.bits.len()));
            ctx.tiledb.insert(tile, bel, "OMUX", item);

            let item = ctx.extract_enum_bool(tile, bel, "IFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF1_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF2_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF1_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF2_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "IFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "IFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "OFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "OFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "TFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "TFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "IFF1_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "IFF2_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "IFF_SR_SYNC", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "OFF_SR_SYNC", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "TFF_SR_SYNC", item);

            // Input path
            let item = xlat_enum(vec![
                ("GND", ctx.state.get_diff(tile, bel, "TSMUX", "0")),
                ("TMUX", ctx.state.get_diff(tile, bel, "TSMUX", "1")),
            ]);
            ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);

            if !edev.chip.kind.is_spartan3a() {
                let item = ctx.extract_enum_bool(tile, bel, "IDELMUX", "1", "0");
                ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
                let item = ctx.extract_enum_bool(tile, bel, "IFFDELMUX", "1", "0");
                ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);
            } else {
                let item_i = ctx.extract_enum_bool(tile, bel, "IBUF_DELAY_VALUE", "DLY0", "DLY16");
                let item_iff = ctx.extract_enum_bool(tile, bel, "IFD_DELAY_VALUE", "DLY0", "DLY8");
                if tile.ends_with("L") || tile.ends_with("R") {
                    let en_i = Diff::from_bool_item(&item_i);
                    let en_iff = Diff::from_bool_item(&item_iff);
                    let common = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8")
                        .combine(&!&en_i);
                    assert_eq!(
                        common,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4")
                            .combine(&!&en_iff)
                    );
                    // I
                    let b0_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY15")
                        .combine(&!&en_i);
                    let b1_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY14")
                        .combine(&!&en_i);
                    let b2_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12")
                        .combine(&!&en_i);
                    for (val, diffs) in [
                        ("DLY13", &[&b0_i, &b1_i][..]),
                        ("DLY11", &[&b0_i, &b2_i][..]),
                        ("DLY10", &[&b1_i, &b2_i][..]),
                        ("DLY9", &[&b0_i, &b1_i, &b2_i][..]),
                        ("DLY7", &[&b0_i, &common][..]),
                        ("DLY6", &[&b1_i, &common][..]),
                        ("DLY5", &[&b0_i, &b1_i, &common][..]),
                        ("DLY4", &[&b2_i, &common][..]),
                        ("DLY3", &[&b0_i, &b2_i, &common][..]),
                        ("DLY2", &[&b1_i, &b2_i, &common][..]),
                        ("DLY1", &[&b0_i, &b1_i, &b2_i, &common][..]),
                    ] {
                        let mut diff = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", val);
                        for &d in diffs {
                            diff = diff.combine(&!d);
                        }
                        diff = diff.combine(&!&en_i);
                        diff.assert_empty();
                    }
                    ctx.tiledb
                        .insert(tile, bel, "I_DELAY", xlat_bitvec(vec![!b0_i, !b1_i, !b2_i]));

                    // IFF
                    let b0_iff = ctx
                        .state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY7")
                        .combine(&!&en_iff);
                    let b1_iff = ctx
                        .state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6")
                        .combine(&!&en_iff);
                    for (val, diffs) in [
                        ("DLY5", &[&b0_iff, &b1_iff][..]),
                        ("DLY3", &[&b0_iff, &common][..]),
                        ("DLY2", &[&b1_iff, &common][..]),
                        ("DLY1", &[&b0_iff, &b1_iff, &common][..]),
                    ] {
                        let mut diff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", val);
                        for &d in diffs {
                            diff = diff.combine(&!d);
                        }
                        diff = diff.combine(&!&en_iff);
                        diff.assert_empty();
                    }
                    ctx.tiledb
                        .insert(tile, bel, "IFF_DELAY", xlat_bitvec(vec![!b0_iff, !b1_iff]));
                    ctx.tiledb
                        .insert(tile, bel, "DELAY_COMMON", xlat_bit(!common));
                    let item = ctx.extract_bit(tile, bel, "DELAY_ADJ_ATTRBOX", "VARIABLE");
                    ctx.tiledb.insert(tile, bel, "DELAY_VARIABLE", item);
                }
                ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item_i);
                ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item_iff);
            }
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "I_TSBYPASS_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFDMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "IFF_TSBYPASS_ENABLE", item);

            if edev.chip.kind.is_spartan3ea() {
                let item = xlat_enum(vec![
                    ("IFFDMUX", ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "2")),
                    ("IDDRIN1", ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "1")),
                    ("IDDRIN2", ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "0")),
                    ("NONE", Diff::default()),
                ]);
                ctx.tiledb.insert(tile, bel, "IDDRIN_MUX", item);
            }
            if edev.chip.kind == ChipKind::Spartan3E {
                let en = ctx.state.get_diff(tile, bel, "MISR_ENABLE", "1");
                let en_rst = ctx.state.get_diff(tile, bel, "MISR_ENABLE_RESET", "1");
                let rst = en_rst.combine(&!&en);
                ctx.tiledb.insert(tile, bel, "MISR_RESET", xlat_bit(rst));
                let clk1 = ctx.state.get_diff(tile, bel, "MISR_ENABLE_OTCLK1", "1");
                let clk2 = ctx.state.get_diff(tile, bel, "MISR_ENABLE_OTCLK2", "1");
                assert_eq!(en, clk1);
                let (clk1, clk2, en) = Diff::split(clk1, clk2);
                ctx.tiledb.insert(tile, bel, "MISR_ENABLE", xlat_bit(en));
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "MISR_CLOCK",
                    xlat_enum(vec![
                        ("OTCLK1", clk1),
                        ("OTCLK2", clk2),
                        ("NONE", Diff::default()),
                    ]),
                );
            }
            if edev.chip.kind.is_spartan3a() {
                ctx.collect_bit(tile, bel, "MISR_ENABLE", "1");
                let clk1 = ctx.state.get_diff(tile, bel, "MISR_ENABLE_OTCLK1", "1");
                let clk2 = ctx.state.get_diff(tile, bel, "MISR_ENABLE_OTCLK2", "1");
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "MISR_CLOCK",
                    xlat_enum(vec![
                        ("OTCLK1", clk1),
                        ("OTCLK2", clk2),
                        ("NONE", Diff::default()),
                    ]),
                );
                // Spartan 3A also has the MISRRESET global option, but it affects *all*
                // IOIs in the device, whether they're in use or not, so we cannot easily
                // isolate the diff to a single IOI. The bits are the same as Spartan 3E,
                // so just cheat and inject them manually.
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "MISR_RESET",
                    TileItem {
                        bits: vec![TileBit::new(0, 0, [7, 32, 47][idx])],
                        kind: TileItemKind::BitVec {
                            invert: BitVec::from_iter([false]),
                        },
                    },
                )
            }
            // these could be extracted automatically from .ll files but I'm not setting up
            // a while another kind of fuzzer for a handful of bits.
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
            ctx.tiledb
                .insert(tile, bel, "READBACK_I", TileItem::from_bit(bit, false));

            // discard detritus from IOB testing
            if !edev.chip.kind.is_spartan3a() || slot != bels::IO2 {
                let mut diff = ctx.state.get_diff(tile, bel, "OUTPUT_ENABLE", "1");
                diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.T1"), true, false);
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "OMUX"), "O1", "NONE");
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "TMUX"), "T1", "NONE");
                diff.assert_empty();
            }
            if edev.chip.kind.is_spartan3a() {
                if slot != bels::IO2 {
                    ctx.state
                        .get_diff(tile, bel, "SEL_MUX", "OMUX")
                        .assert_empty();
                }
                if slot == bels::IO2 || tile == "IOI.S3A.LR" {
                    let mut diff = ctx.state.get_diff(tile, bel, "SEL_MUX", "OMUX_IBUF");
                    diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "OMUX"), "O1", "NONE");
                    diff.assert_empty();
                }
            }
        }
        // specials. need cross-bel discard.
        if edev.chip.kind.is_spartan3ea() {
            for bel in ["IO0", "IO1"] {
                let obel = if bel == "IO0" { "IO1" } else { "IO0" };
                ctx.state
                    .get_diff(tile, bel, "O1_DDRMUX", "1")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, bel, "O2_DDRMUX", "1")
                    .assert_empty();
                let mut diff = ctx.state.get_diff(tile, bel, "O1_DDRMUX", "0");
                diff.discard_bits(ctx.tiledb.item(tile, obel, "OMUX"));
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "O1_DDRMUX",
                    xlat_enum(vec![("O1", Diff::default()), ("ODDRIN1", diff)]),
                );
                let mut diff = ctx.state.get_diff(tile, bel, "O2_DDRMUX", "0");
                diff.discard_bits(ctx.tiledb.item(tile, obel, "OMUX"));
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "O2_DDRMUX",
                    xlat_enum(vec![("O2", Diff::default()), ("ODDRIN2", diff)]),
                );
            }
        }
    }

    // IOB
    for (tcls, tile, _) in &intdb.tile_classes {
        if !tile.starts_with("IOB") {
            continue;
        }
        if ctx.edev.tile_index[tcls].is_empty() {
            continue;
        }
        let iob_data = get_iob_data(tile);
        let is_s3a_lr = matches!(&tile[..], "IOBS.S3A.L4" | "IOBS.S3A.R4");
        for &iob in &iob_data.iobs {
            let bel = &format!("IOB{}", iob.index);
            if iob.kind == IobKind::Clk {
                continue;
            }
            if edev.chip.kind.is_spartan3ea() {
                ctx.state
                    .get_diff(tile, bel, "GTSATTRBOX", "DISABLE_GTS")
                    .assert_empty();
            } else {
                let item = ctx.extract_bit(tile, bel, "GTSATTRBOX", "DISABLE_GTS");
                ctx.tiledb.insert(tile, bel, "DISABLE_GTS", item);
            }
            ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
            if edev.chip.kind.is_spartan3a() && iob.kind != IobKind::Ibuf {
                ctx.collect_enum(
                    tile,
                    bel,
                    "SUSPEND",
                    &[
                        "DRIVE_LAST_VALUE",
                        "3STATE",
                        "3STATE_PULLUP",
                        "3STATE_PULLDOWN",
                        "3STATE_KEEPER",
                    ],
                );
            }
            if edev.chip.kind == ChipKind::Spartan3E {
                if tile.starts_with("IOBS.S3E.R") {
                    for val in ["DLY13", "DLY14", "DLY15", "DLY16"] {
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", val)
                            .assert_empty();
                    }
                    for val in ["DLY7", "DLY8"] {
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", val)
                            .assert_empty();
                    }
                    let common = !ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY3");

                    let iff3_6 = Diff::default();
                    assert_eq!(
                        iff3_6,
                        ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6")
                    );
                    let iff2_5 = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY5");
                    assert_eq!(
                        iff2_5,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY2")
                            .combine(&common)
                    );
                    let iff1_4 = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4");
                    assert_eq!(
                        iff1_4,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY1")
                            .combine(&common)
                    );
                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "IFF_DELAY",
                        xlat_enum(vec![
                            ("SDLY3_LDLY6", iff3_6),
                            ("SDLY2_LDLY5", iff2_5),
                            ("SDLY1_LDLY4", iff1_4),
                        ]),
                    );

                    let i12 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12");
                    let i5_11 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY11");
                    assert_eq!(
                        i5_11,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY5")
                            .combine(&common)
                    );
                    let i4_10 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY10");
                    assert_eq!(
                        i4_10,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY4")
                            .combine(&common)
                    );
                    let i3_9 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY9");
                    assert_eq!(
                        i3_9,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY3")
                            .combine(&common)
                    );
                    let i2_8 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8");
                    assert_eq!(
                        i2_8,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY2")
                            .combine(&common)
                    );
                    let i1_7 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY7");
                    assert_eq!(
                        i1_7,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY1")
                            .combine(&common)
                    );
                    let i6 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY6");

                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "I_DELAY",
                        xlat_enum(vec![
                            ("LDLY12", i12),
                            ("SDLY5_LDLY11", i5_11),
                            ("SDLY4_LDLY10", i4_10),
                            ("SDLY3_LDLY9", i3_9),
                            ("SDLY2_LDLY8", i2_8),
                            ("SDLY1_LDLY7", i1_7),
                            ("LDLY6", i6),
                        ]),
                    );

                    ctx.tiledb
                        .insert(tile, bel, "DELAY_COMMON", xlat_bit(!common));
                } else {
                    for val in ["DLY14", "DLY15", "DLY16"] {
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", val)
                            .assert_empty();
                    }
                    ctx.state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY8")
                        .assert_empty();
                    let common = !ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4");

                    let iff4_7 = Diff::default();
                    assert_eq!(
                        iff4_7,
                        ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY7")
                    );
                    let iff3_6 = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6");
                    assert_eq!(
                        iff3_6,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY3")
                            .combine(&common)
                    );
                    let iff2_5 = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY5");
                    assert_eq!(
                        iff2_5,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY2")
                            .combine(&common)
                    );
                    let iff1 = ctx
                        .state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY1")
                        .combine(&common);
                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "IFF_DELAY",
                        xlat_enum(vec![
                            ("SDLY4_LDLY7", iff4_7),
                            ("SDLY3_LDLY6", iff3_6),
                            ("SDLY2_LDLY5", iff2_5),
                            ("SDLY1", iff1),
                        ]),
                    );

                    let i13 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY13");
                    let i7 = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY7")
                        .combine(&common);
                    let i6_12 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12");
                    assert_eq!(
                        i6_12,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY6")
                            .combine(&common)
                    );
                    let i5_11 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY11");
                    assert_eq!(
                        i5_11,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY5")
                            .combine(&common)
                    );
                    let i4_10 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY10");
                    assert_eq!(
                        i4_10,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY4")
                            .combine(&common)
                    );
                    let i3_9 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY9");
                    assert_eq!(
                        i3_9,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY3")
                            .combine(&common)
                    );
                    let i2_8 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8");
                    assert_eq!(
                        i2_8,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY2")
                            .combine(&common)
                    );
                    let i1 = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY1")
                        .combine(&common);

                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "I_DELAY",
                        xlat_enum(vec![
                            ("LDLY13", i13),
                            ("SDLY7", i7),
                            ("SDLY6_LDLY12", i6_12),
                            ("SDLY5_LDLY11", i5_11),
                            ("SDLY4_LDLY10", i4_10),
                            ("SDLY3_LDLY9", i3_9),
                            ("SDLY2_LDLY8", i2_8),
                            ("SDLY1", i1),
                        ]),
                    );

                    ctx.tiledb
                        .insert(tile, bel, "DELAY_COMMON", xlat_bit(!common));
                }
            }
            if edev.chip.kind.is_spartan3a()
                && (tile.starts_with("IOBS.S3A.B") || tile.starts_with("IOBS.S3A.T"))
            {
                let common = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8");
                assert_eq!(
                    common,
                    ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4")
                );
                // I
                let b0_i = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY15");
                let b1_i = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY14");
                let b2_i = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12");
                for (val, diffs) in [
                    ("DLY16", &[][..]),
                    ("DLY13", &[&b0_i, &b1_i][..]),
                    ("DLY11", &[&b0_i, &b2_i][..]),
                    ("DLY10", &[&b1_i, &b2_i][..]),
                    ("DLY9", &[&b0_i, &b1_i, &b2_i][..]),
                    ("DLY7", &[&b0_i, &common][..]),
                    ("DLY6", &[&b1_i, &common][..]),
                    ("DLY5", &[&b0_i, &b1_i, &common][..]),
                    ("DLY4", &[&b2_i, &common][..]),
                    ("DLY3", &[&b0_i, &b2_i, &common][..]),
                    ("DLY2", &[&b1_i, &b2_i, &common][..]),
                    ("DLY1", &[&b0_i, &b1_i, &b2_i, &common][..]),
                ] {
                    let mut diff = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", val);
                    for &d in diffs {
                        diff = diff.combine(&!d);
                    }
                    diff.assert_empty();
                }
                ctx.tiledb
                    .insert(tile, bel, "I_DELAY", xlat_bitvec(vec![!b0_i, !b1_i, !b2_i]));

                // IFF
                let b0_iff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY7");
                let b1_iff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6");
                for (val, diffs) in [
                    ("DLY8", &[][..]),
                    ("DLY5", &[&b0_iff, &b1_iff][..]),
                    ("DLY3", &[&b0_iff, &common][..]),
                    ("DLY2", &[&b1_iff, &common][..]),
                    ("DLY1", &[&b0_iff, &b1_iff, &common][..]),
                ] {
                    let mut diff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", val);
                    for &d in diffs {
                        diff = diff.combine(&!d);
                    }
                    diff.assert_empty();
                }
                ctx.tiledb
                    .insert(tile, bel, "IFF_DELAY", xlat_bitvec(vec![!b0_iff, !b1_iff]));
                ctx.tiledb
                    .insert(tile, bel, "DELAY_COMMON", xlat_bit(!common));
                let item = ctx.extract_bit(tile, bel, "DELAY_ADJ_ATTRBOX", "VARIABLE");
                ctx.tiledb.insert(tile, bel, "DELAY_VARIABLE", item);
            }
            // Input path.
            if !edev.chip.kind.is_spartan3ea() {
                let mut vals = vec![
                    ("NONE", Diff::default()),
                    (
                        "CMOS",
                        ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS33").clone(),
                    ),
                    (
                        "VREF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "SSTL2_I").clone(),
                    ),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        "DIFF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "BLVDS_25").clone(),
                    ));
                }
                ctx.tiledb.insert(tile, bel, "IBUF_MODE", xlat_enum(vals));
            } else if edev.chip.kind == ChipKind::Spartan3E {
                let mut vals = vec![
                    ("NONE", Diff::default()),
                    (
                        "CMOS_LV",
                        ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS18").clone(),
                    ),
                    (
                        "CMOS_HV",
                        ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS33").clone(),
                    ),
                    (
                        "VREF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "SSTL2_I").clone(),
                    ),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        "DIFF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "BLVDS_25").clone(),
                    ));
                }
                ctx.tiledb.insert(tile, bel, "IBUF_MODE", xlat_enum(vals));
            } else {
                let mut vals = vec![
                    ("NONE", Diff::default()),
                    (
                        "CMOS_VCCINT",
                        ctx.state
                            .peek_diff(tile, bel, "ISTD.3.3", "LVCMOS18")
                            .clone(),
                    ),
                    (
                        "CMOS_VCCAUX",
                        ctx.state
                            .peek_diff(tile, bel, "ISTD.2.5", "LVCMOS25")
                            .clone(),
                    ),
                    (
                        "CMOS_VCCO",
                        ctx.state
                            .peek_diff(tile, bel, "ISTD.3.3", "LVCMOS25")
                            .clone(),
                    ),
                    (
                        "VREF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "SSTL2_I").clone(),
                    ),
                    (
                        "TMUX",
                        ctx.state.get_diff(tile, bel, "SEL_MUX", "TMUX").clone(),
                    ),
                    (
                        "OMUX",
                        ctx.state.get_diff(tile, bel, "SEL_MUX", "OMUX").clone(),
                    ),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        "DIFF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "BLVDS_25").clone(),
                    ));
                }
                ctx.tiledb.insert(tile, bel, "IBUF_MODE", xlat_enum(vals));
            }
            if edev.chip.kind.is_spartan3ea()
                && !is_s3a_lr
                && iob.diff != IobDiff::None
                && iob.kind != IobKind::Ibuf
            {
                ctx.state
                    .get_diff(tile, bel, "DIFF_TERM.COMP", "1")
                    .assert_empty();
                if matches!(iob.diff, IobDiff::Comp(_)) {
                    // ignore
                    ctx.state.get_diff(tile, bel, "DIFF_TERM", "1");
                }
            }
            if has_any_vref(edev, ctx.device, ctx.db, tile, iob.index).is_some() {
                let present_vref = ctx.state.get_diff(tile, bel, "PRESENT", "NOTVREF");
                let present = ctx.state.peek_diff(
                    tile,
                    bel,
                    "PRESENT",
                    if edev.chip.kind.is_spartan3ea() {
                        "IBUF"
                    } else {
                        "IOB"
                    },
                );
                let mut vref = present.combine(&!present_vref);
                vref.discard_bits(ctx.tiledb.item(tile, bel, "PULL"));
                ctx.tiledb.insert(tile, bel, "VREF", xlat_bit(vref));
            }

            // PCI cruft
            if edev.chip.kind.is_spartan3a() {
                let mut ibuf_diff = ctx.state.peek_diff(tile, bel, "ISTD", "PCI33_3").clone();
                ibuf_diff.discard_bits(ctx.tiledb.item(tile, bel, "IBUF_MODE"));
                if iob.kind == IobKind::Ibuf {
                    ctx.tiledb
                        .insert(tile, bel, "PCI_INPUT", xlat_bit(ibuf_diff));
                } else {
                    let obuf_diff = ctx
                        .state
                        .peek_diff(tile, bel, "OSTD", "PCI33_3.3.3")
                        .clone();
                    let (ibuf_diff, _, common) = Diff::split(ibuf_diff, obuf_diff);
                    ctx.tiledb
                        .insert(tile, bel, "PCI_INPUT", xlat_bit(ibuf_diff));
                    ctx.tiledb.insert(tile, bel, "PCI_CLAMP", xlat_bit(common));
                }
            }

            // Output path.
            if iob.kind != IobKind::Ibuf {
                ctx.collect_bit_wide(tile, bel, "OUTPUT_ENABLE", "1");

                // well ...
                let mut slew_bits = HashSet::new();
                let mut drive_bits = HashSet::new();
                for std in get_iostds(edev, is_s3a_lr) {
                    if std.drive.is_empty() {
                        continue;
                    }
                    let vccauxs = if edev.chip.kind.is_spartan3a() {
                        &["2.5", "3.3"][..]
                    } else {
                        &[""]
                    };
                    let slews = if edev.chip.kind.is_spartan3a() {
                        &["FAST", "SLOW", "QUIETIO"][..]
                    } else {
                        &["FAST", "SLOW"]
                    };
                    for vccaux in vccauxs {
                        // grab SLEW bits.
                        for &drive in std.drive {
                            if edev.chip.kind.is_virtex2p()
                                && std.name == "LVCMOS33"
                                && drive == "8"
                            {
                                // ISE bug.
                                continue;
                            }
                            let mut base: Option<Diff> = None;
                            for &slew in slews {
                                let name = if edev.chip.kind.is_spartan3a() {
                                    format!("{s}.{drive}.{slew}.{vccaux}", s = std.name)
                                } else {
                                    format!("{s}.{drive}.{slew}", s = std.name)
                                };
                                let diff = ctx.state.peek_diff(tile, bel, "OSTD", name);
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
                                let name = if edev.chip.kind.is_spartan3a() {
                                    format!("{s}.{drive}.{slew}.{vccaux}", s = std.name)
                                } else {
                                    format!("{s}.{drive}.{slew}", s = std.name)
                                };
                                let diff = ctx.state.peek_diff(tile, bel, "OSTD", &name);
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
                    let gtl = ctx.state.peek_diff(tile, bel, "OSTD", "GTL");
                    let gtlp = ctx.state.peek_diff(tile, bel, "OSTD", "GTLP");
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
                    let oprog = xlat_bitvec(ctx.state.get_diffs(tile, bel, "OPROGRAMMING", ""));
                    for i in 13..16 {
                        pdrive_bits.insert(oprog.bits[i]);
                    }
                    for i in 2..6 {
                        nslew_bits.push(oprog.bits[i]);
                    }
                    for i in 6..10 {
                        pslew_bits.push(oprog.bits[i]);
                    }
                } else if edev.chip.kind == ChipKind::Spartan3 {
                    pdrive_bits = drive_bits.clone();
                    for &bit in ctx.state.peek_diff(tile, bel, "OSTD", "GTL").bits.keys() {
                        if drive_bits.contains(&bit) {
                            pdrive_bits.remove(&bit);
                        }
                    }
                } else {
                    let drives = if edev.chip.kind == ChipKind::Spartan3E {
                        &["2", "4", "6", "8", "12", "16"][..]
                    } else {
                        &["2", "4", "6", "8", "12", "16", "24"][..]
                    };
                    for drive in drives {
                        let ttl =
                            ctx.state
                                .peek_diff(tile, bel, "OSTD", format!("LVTTL.{drive}.SLOW"));
                        let cmos = ctx.state.peek_diff(
                            tile,
                            bel,
                            "OSTD",
                            format!("LVCMOS33.{drive}.SLOW"),
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
                    let item = ctx.tiledb.item(tile, bel, "IBUF_MODE");
                    let mut diff_split = ctx
                        .state
                        .peek_diff(tile, bel, "ISTD", "SSTL2_I_DCI")
                        .clone();
                    diff_split.discard_bits(item);
                    for &bit in diff_split.bits.keys() {
                        dci_bits.insert(bit);
                    }
                    let mut diff_vcc = ctx.state.peek_diff(tile, bel, "ISTD", "GTL_DCI").clone();
                    diff_vcc.discard_bits(item);
                    for &bit in diff_vcc.bits.keys() {
                        dci_bits.insert(bit);
                    }
                    let mut diff_output =
                        ctx.state.peek_diff(tile, bel, "OSTD", "LVDCI_25").clone();
                    let diff_output = diff_output.split_bits(&dci_bits);
                    let mut diff_output_half = ctx
                        .state
                        .peek_diff(tile, bel, "OSTD", "LVDCI_DV2_25")
                        .clone();
                    let diff_output_half = diff_output_half.split_bits(&dci_bits);
                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "DCI_MODE",
                        xlat_enum(vec![
                            ("NONE", Diff::default()),
                            ("OUTPUT", diff_output),
                            ("OUTPUT_HALF", diff_output_half),
                            ("TERM_SPLIT", diff_split),
                            ("TERM_VCC", diff_vcc),
                        ]),
                    );
                }
                let mut vr_slew = None;
                if has_any_vr(edev, ctx.device, ctx.db, tile, iob.index).is_some() {
                    let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "NOTVR");
                    let item = ctx.tiledb.item(tile, bel, "DCI_MODE");
                    diff.apply_enum_diff(item, "NONE", "TERM_SPLIT");
                    diff = !diff;
                    vr_slew = Some(diff.split_bits(&slew_bits));
                    ctx.tiledb.insert(tile, bel, "VR", xlat_bit(diff));
                }
                if matches!(
                    edev.chip.kind,
                    ChipKind::Virtex2P | ChipKind::Virtex2PX | ChipKind::Spartan3
                ) && !ctx.device.name.ends_with("2vp4")
                    && !ctx.device.name.ends_with("2vp7")
                {
                    let diff_a = ctx.state.get_diff(tile, bel, "DCIUPDATEMODE", "ASREQUIRED");
                    let diff_c = ctx.state.get_diff(tile, bel, "DCIUPDATEMODE", "CONTINUOUS");
                    let diff_q = ctx.state.get_diff(tile, bel, "DCIUPDATEMODE", "QUIET");
                    assert_eq!(diff_c, diff_q);
                    let diff = diff_a.combine(&!diff_c);
                    ctx.tiledb
                        .insert(tile, bel, "DCIUPDATEMODE_ASREQUIRED", xlat_bit(diff));
                }
                let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "IOB");
                if edev.chip.kind.is_spartan3ea() {
                    let ibuf_present = ctx.state.get_diff(tile, bel, "PRESENT", "IBUF");
                    assert_eq!(present, ibuf_present);
                }
                present.discard_bits(ctx.tiledb.item(tile, bel, "PULL"));
                for &val in present.bits.values() {
                    assert!(val)
                }

                let mut slew_diffs = vec![];
                let mut pslew_diffs = vec![];
                let mut nslew_diffs = vec![];
                let mut pdrive_diffs = vec![];
                let mut ndrive_diffs = vec![];
                let mut misc_diffs = vec![];
                for std in get_iostds(edev, is_s3a_lr) {
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
                        (&[""][..], &[""][..])
                    } else {
                        (
                            std.drive,
                            if edev.chip.kind.is_spartan3a() {
                                &["FAST", "SLOW", "QUIETIO"][..]
                            } else {
                                &["FAST", "SLOW"][..]
                            },
                        )
                    };
                    if std.dci != DciKind::None && std.diff != DiffKind::None {
                        continue;
                    }
                    for &vccaux in vccauxs {
                        for &drive in drives {
                            for &slew in slews {
                                let name = if vccaux.is_empty() {
                                    if drive.is_empty() {
                                        std.name.to_string()
                                    } else {
                                        format!("{s}.{drive}.{slew}", s = std.name)
                                    }
                                } else {
                                    if drive.is_empty() {
                                        format!("{s}.{vccaux}", s = std.name)
                                    } else {
                                        format!("{s}.{drive}.{slew}.{vccaux}", s = std.name)
                                    }
                                };
                                let mut diff = ctx.state.get_diff(tile, bel, "OSTD", &name);
                                if edev.chip.kind.is_virtex2p()
                                    && std.name == "LVCMOS33"
                                    && drive == "8"
                                    && slew == "FAST"
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
                                    let item = ctx.tiledb.item(tile, bel, "DCI_MODE");
                                    match std.dci {
                                        DciKind::Output => {
                                            diff.apply_enum_diff(item, "OUTPUT", "NONE")
                                        }
                                        DciKind::OutputHalf => {
                                            diff.apply_enum_diff(item, "OUTPUT_HALF", "NONE")
                                        }
                                        DciKind::BiVcc => {
                                            diff.apply_enum_diff(item, "TERM_VCC", "NONE")
                                        }
                                        DciKind::BiSplit => {
                                            diff.apply_enum_diff(item, "TERM_SPLIT", "NONE")
                                        }
                                        _ => (),
                                    }
                                }
                                if edev.chip.kind.is_spartan3a() && std.name.starts_with("PCI") {
                                    diff.apply_bit_diff(
                                        ctx.tiledb.item(tile, bel, "PCI_CLAMP"),
                                        true,
                                        false,
                                    );
                                }
                                let stdn = if let Some(x) = std.name.strip_prefix("DIFF_") {
                                    x
                                } else {
                                    std.name
                                };
                                let slew_name = if vccaux.is_empty() {
                                    if slew.is_empty() {
                                        stdn.to_string()
                                    } else {
                                        format!("{stdn}.{slew}")
                                    }
                                } else {
                                    if slew.is_empty() {
                                        format!("{stdn}.{vccaux}")
                                    } else {
                                        format!("{stdn}.{slew}.{vccaux}")
                                    }
                                };
                                let drive_name = if slew.is_empty() {
                                    stdn.to_string()
                                } else {
                                    format!("{stdn}.{drive}")
                                };
                                slew_diffs.push((slew_name.clone(), slew_diff));
                                pslew_diffs.push((slew_name.clone(), pslew_diff));
                                nslew_diffs.push((slew_name, nslew_diff));
                                if matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                                    pdrive_diff.assert_empty();
                                    ndrive_diff.assert_empty();
                                } else {
                                    pdrive_diffs.push((drive_name.clone(), pdrive_diff));
                                    ndrive_diffs.push((drive_name, ndrive_diff));
                                }
                                if edev.chip.kind != ChipKind::Spartan3E
                                    || !std.name.starts_with("DIFF_")
                                {
                                    misc_diffs.push((stdn.to_string(), diff));
                                }
                            }
                        }
                    }
                }
                if let Some(vr_slew) = vr_slew {
                    slew_diffs.push(("VR".to_string(), vr_slew));
                }
                for &bit in present.bits.keys() {
                    assert!(pdrive_bits.contains(&bit) || ndrive_bits.contains(&bit));
                }
                pdrive_diffs.push(("OFF".to_string(), Diff::default()));
                ndrive_diffs.push(("OFF".to_string(), Diff::default()));
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
                let prefix = match edev.chip.kind {
                    ChipKind::Virtex2 => "V2",
                    ChipKind::Virtex2P | ChipKind::Virtex2PX => "V2P",
                    ChipKind::Spartan3 => "S3",
                    ChipKind::FpgaCore => unreachable!(),
                    ChipKind::Spartan3E => "S3E",
                    ChipKind::Spartan3A | ChipKind::Spartan3ADsp => {
                        if is_s3a_lr {
                            "S3A.LR"
                        } else {
                            "S3A.TB"
                        }
                    }
                };
                for (set_name, diffs) in [
                    ("SLEW", slew_diffs),
                    ("PSLEW", pslew_diffs),
                    ("NSLEW", nslew_diffs),
                    ("PDRIVE", pdrive_diffs),
                    ("NDRIVE", ndrive_diffs),
                    ("OUTPUT_MISC", misc_diffs),
                ] {
                    let ocd = if set_name == "PSLEW" {
                        OcdMode::FixedOrder(&pslew_bits)
                    } else if set_name == "NSLEW" {
                        OcdMode::FixedOrder(&nslew_bits)
                    } else {
                        OcdMode::ValueOrder
                    };
                    let mut item_enum = xlat_enum_ocd(diffs, ocd);
                    if set_name == "PDRIVE" && edev.chip.kind == ChipKind::Spartan3E {
                        // needs a little push.
                        enum_ocd_swap_bits(&mut item_enum, 0, 1);
                    }
                    if item_enum.bits.is_empty() {
                        continue;
                    }
                    let invert = BitVec::from_iter(
                        item_enum
                            .bits
                            .iter()
                            .map(|bit| present.bits.contains_key(bit)),
                    );
                    let item_pdrive = TileItem {
                        bits: item_enum.bits,
                        kind: TileItemKind::BitVec { invert },
                    };
                    ctx.tiledb.insert(tile, bel, set_name, item_pdrive);
                    let TileItemKind::Enum { values } = item_enum.kind else {
                        unreachable!()
                    };
                    for (name, value) in values {
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{prefix}:{set_name}:{name}"), value);
                    }
                }

                // True differential output path.
                if let IobDiff::True(other) = iob.diff {
                    let bel_n = [
                        "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
                    ][other];
                    if !is_s3a_lr {
                        let mut group_diff = None;
                        if edev.chip.kind.is_spartan3ea() {
                            let base = ctx.state.peek_diff(tile, bel, "DIFFO", "RSDS_25");
                            let alt = ctx.state.peek_diff(tile, bel, "DIFFO.ALT", "RSDS_25");
                            let diff = alt.combine(&!base);
                            group_diff = Some(diff.clone());
                            let item = xlat_bit_wide(diff);
                            ctx.tiledb.insert(tile, bel, "OUTPUT_DIFF_GROUP", item);
                        }
                        let mut diffs = vec![("OFF", Diff::default())];
                        if edev.chip.kind.is_virtex2p() {
                            diffs.push((
                                "TERM",
                                ctx.state
                                    .peek_diff(tile, bel_n, "ISTD.COMP", "LVDS_25_DT")
                                    .clone(),
                            ));
                        } else if edev.chip.kind.is_spartan3ea() {
                            diffs.push(("TERM", ctx.state.get_diff(tile, bel, "DIFF_TERM", "1")));
                        }

                        for std in get_iostds(edev, is_s3a_lr) {
                            if std.diff != DiffKind::True {
                                continue;
                            }
                            let mut diff = ctx.state.get_diff(tile, bel, "DIFFO", std.name);
                            if edev.chip.kind.is_spartan3ea() {
                                let mut altdiff =
                                    ctx.state.get_diff(tile, bel, "DIFFO.ALT", std.name);
                                if edev.chip.kind == ChipKind::Spartan3E && std.name == "LVDS_25" {
                                    assert_eq!(diff, altdiff);
                                    diff = diff.combine(&!group_diff.as_ref().unwrap());
                                } else {
                                    altdiff = altdiff.combine(&!group_diff.as_ref().unwrap());
                                    assert_eq!(diff, altdiff);
                                }
                            }
                            diffs.push((std.name, diff));
                        }
                        let item_enum = xlat_enum(diffs);
                        let l = item_enum.bits.len();
                        let item = TileItem {
                            bits: item_enum.bits,
                            kind: TileItemKind::BitVec {
                                invert: BitVec::repeat(false, l),
                            },
                        };
                        ctx.tiledb.insert(tile, bel, "OUTPUT_DIFF", item);
                        let TileItemKind::Enum { values } = item_enum.kind else {
                            unreachable!()
                        };
                        for (name, value) in values {
                            ctx.tiledb.insert_misc_data(
                                format!("IOSTD:{prefix}:OUTPUT_DIFF:{name}"),
                                value,
                            );
                        }
                    }
                }
            }
            if iob.kind == IobKind::Ibuf {
                let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "IBUF");
                diff.discard_bits(ctx.tiledb.item(tile, bel, "PULL"));
                if edev.chip.kind.is_spartan3a() {
                    diff.assert_empty();
                } else {
                    // ???
                    ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff));
                }
            }
            if has_any_brefclk(edev, tile, iob.index).is_some() {
                ctx.collect_bit(tile, bel, "BREFCLK_ENABLE", "1");
            }
        }
        // second loop for stuff involving inter-bel dependencies
        for &iob in &iob_data.iobs {
            let bel = &format!("IOB{}", iob.index);
            if iob.kind == IobKind::Clk {
                continue;
            }
            for std in get_iostds(edev, is_s3a_lr) {
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
                    &["ISTD.2.5", "ISTD.3.3"][..]
                } else {
                    &["ISTD"][..]
                };
                for &attr in attrs {
                    let mut diff = ctx.state.get_diff(tile, bel, attr, std.name);
                    if edev.chip.kind.is_spartan3a() && std.name.starts_with("PCI") {
                        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PCI_INPUT"), true, false);
                        if iob.kind != IobKind::Ibuf {
                            diff.apply_bit_diff(
                                ctx.tiledb.item(tile, bel, "PCI_CLAMP"),
                                true,
                                false,
                            );
                        }
                    }
                    let ibuf_mode = if std.diff != DiffKind::None {
                        "DIFF"
                    } else if std.vref.is_some() {
                        "VREF"
                    } else if edev.chip.kind.is_spartan3a() {
                        let vcco = std.vcco.unwrap();
                        if vcco < 2500 {
                            "CMOS_VCCINT"
                        } else if std.name.starts_with("PCI")
                            || (std.name == "LVCMOS25" && attr == "ISTD.3.3")
                        {
                            "CMOS_VCCO"
                        } else {
                            "CMOS_VCCAUX"
                        }
                    } else if edev.chip.kind == ChipKind::Spartan3E {
                        let vcco = std.vcco.unwrap();
                        if vcco < 2500 { "CMOS_LV" } else { "CMOS_HV" }
                    } else {
                        "CMOS"
                    };
                    diff.apply_enum_diff(
                        ctx.tiledb.item(tile, bel, "IBUF_MODE"),
                        ibuf_mode,
                        "NONE",
                    );
                    let dci_mode = match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => "NONE",
                        DciKind::InputVcc | DciKind::BiVcc => "TERM_VCC",
                        DciKind::InputSplit | DciKind::BiSplit => "TERM_SPLIT",
                        _ => unreachable!(),
                    };
                    if dci_mode != "NONE" {
                        diff.apply_enum_diff(
                            ctx.tiledb.item(tile, bel, "DCI_MODE"),
                            dci_mode,
                            "NONE",
                        );
                    }
                    if edev.chip.kind == ChipKind::Spartan3E
                        && std.name == "LVDS_25"
                        && iob.kind != IobKind::Ibuf
                    {
                        let bel_p = if let IobDiff::Comp(other) = iob.diff {
                            [
                                "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
                            ][other]
                        } else {
                            bel
                        };
                        diff.discard_bits(ctx.tiledb.item(tile, bel_p, "OUTPUT_DIFF_GROUP"));
                    }
                    diff.assert_empty();
                }
                if std.diff != DiffKind::None {
                    let mut diff = ctx.state.get_diff(tile, bel, "ISTD.COMP", std.name);
                    if std.diff == DiffKind::TrueTerm
                        && let IobDiff::Comp(other) = iob.diff
                    {
                        let bel_p = [
                            "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
                        ][other];
                        diff.discard_bits(ctx.tiledb.item(tile, bel_p, "OUTPUT_DIFF"));
                    }
                    if matches!(edev.chip.kind, ChipKind::Spartan3 | ChipKind::Spartan3E) {
                        diff.discard_bits(ctx.tiledb.item(tile, bel, "IBUF_MODE"));
                    }
                    if edev.chip.kind == ChipKind::Spartan3E
                        && std.name == "LVDS_25"
                        && iob.kind != IobKind::Ibuf
                    {
                        let bel_p = if let IobDiff::Comp(other) = iob.diff {
                            [
                                "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
                            ][other]
                        } else {
                            bel
                        };
                        diff.discard_bits(ctx.tiledb.item(tile, bel_p, "OUTPUT_DIFF_GROUP"));
                    }
                    diff.assert_empty();
                }
            }
        }
        if tile.ends_with("CLK") {
            let bel = "BREFCLK_INT";
            ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
        }
    }
}
