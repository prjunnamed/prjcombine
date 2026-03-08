use std::collections::{HashMap, HashSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelSlotId, TableRowId, TileClassId},
    dir::Dir,
    grid::{DieId, DieIdExt, TileCoord},
};
use prjcombine_re_collector::diff::{
    Diff, OcdMode, xlat_bit, xlat_bit_wide_bi, xlat_enum_attr, xlat_enum_raw,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{
    Bond, Device, ExpandedBond, ExpandedDevice, ExpandedNamedDevice, GeomDb,
};
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{PolTileBit, TileBit},
};
use prjcombine_virtex::defs::{
    self,
    bcls::{IOB, IOFB, IOI},
    bslots, enums,
    tables::{IOB_DATA_V, IOB_DATA_VE},
    tcls,
};

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{DynProp, relation::Delta},
    },
    virtex::specials,
};

#[derive(Clone, Debug)]
struct VirtexIsDllIob(BelSlotId, bool);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexIsDllIob {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex(edev) = backend.edev else {
            unreachable!()
        };
        let is_dll = edev.chip.kind.is_virtexe()
            && ((tcrd.col == edev.chip.col_clk() - 1 && self.0 == bslots::IOI[1])
                || (tcrd.col == edev.chip.col_clk() && self.0 == bslots::IOI[2]));
        if self.1 != is_dll {
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

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let ExpandedNamedDevice::Virtex(endev) = backend.endev else {
            unreachable!()
        };
        let crd = endev.grid.get_io_crd(tcrd.bel(self.0));
        if !ebond.bond.vref.contains(&crd) {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct IsDiff(BelSlotId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for IsDiff {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };

        let ExpandedNamedDevice::Virtex(endev) = backend.endev else {
            unreachable!()
        };
        let crd = endev.grid.get_io_crd(tcrd.bel(self.0));
        if !ebond.bond.diffp.contains(&crd) && !ebond.bond.diffn.contains(&crd) {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct VirtexOtherIobInput(pub BelSlotId, pub String);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VirtexOtherIobInput {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex(edev) = backend.edev else {
            unreachable!()
        };
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let ExpandedNamedDevice::Virtex(endev) = backend.endev else {
            unreachable!()
        };
        let (crd, orig_bank) = if bslots::IOI.contains(self.0) {
            let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
            (Some(crd), edev.chip.get_io_bank(crd))
        } else {
            (
                None,
                if tcrd.row == edev.chip.row_s() {
                    if self.0 == bslots::GCLK_IOB[0] { 4 } else { 5 }
                } else {
                    if self.0 == bslots::GCLK_IOB[0] { 1 } else { 0 }
                },
            )
        };
        for io in edev.chip.get_bonded_ios() {
            let bank = edev.chip.get_io_bank(io);
            if Some(io) != crd && bank == orig_bank && ebond.ios.contains_key(&io) {
                let site = endev.get_io_name(io);
                fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
                fuzzer = fuzzer.base(Key::SiteAttr(site, "IOATTRBOX".into()), &self.1);
                fuzzer = fuzzer.base(Key::SiteAttr(site, "IMUX".into()), "1");
                fuzzer = fuzzer.base(Key::SiteAttr(site, "OUTMUX".into()), None);
                fuzzer = fuzzer.base(Key::SiteAttr(site, "TSEL".into()), None);
                fuzzer = fuzzer.base(Key::SitePin(site, "I".into()), true);
                return Some((fuzzer, false));
            }
        }
        None
    }
}

fn has_any_vref<'a>(
    edev: &prjcombine_virtex::expanded::ExpandedDevice,
    device: &'a Device,
    db: &GeomDb,
    tcid: TileClassId,
    slot: BelSlotId,
) -> Option<&'a str> {
    let mut bonded_ios = HashMap::new();
    for devbond in device.bonds.values() {
        let bond = &db.bonds[devbond.bond];
        let Bond::Virtex(bond) = bond else {
            unreachable!()
        };
        for &io in &bond.vref {
            bonded_ios.insert(io, &devbond.name[..]);
        }
    }
    for &tcrd in &edev.tile_index[tcid] {
        let crd = edev.chip.get_io_crd(tcrd.bel(slot));
        if let Some(&pkg) = bonded_ios.get(&crd) {
            return Some(pkg);
        }
    }
    None
}

const IOSTDS_CMOS_V: &[&str] = &["LVTTL", "LVCMOS2", "PCI33_3", "PCI33_5", "PCI66_3"];
const IOSTDS_CMOS_VE: &[&str] = &[
    "LVTTL", "LVCMOS2", "LVCMOS18", "PCI33_3", "PCI66_3", "PCIX66_3",
];
const IOSTDS_VREF_LV: &[&str] = &["GTL", "HSTL_I", "HSTL_III", "HSTL_IV"];
const IOSTDS_VREF_HV: &[&str] = &[
    "GTLP", "SSTL3_I", "SSTL3_II", "SSTL2_I", "SSTL2_II", "AGP", "CTT",
];
const IOSTDS_DIFF: &[&str] = &["LVDS", "LVPECL"];

fn get_istd_row(edev: &prjcombine_virtex::expanded::ExpandedDevice<'_>, iostd: &str) -> TableRowId {
    let iostd = if iostd == "LVTTL" { "LVTTL_2" } else { iostd };
    if edev.chip.kind.is_virtexe() {
        edev.db[IOB_DATA_VE].rows.get(iostd).unwrap().0
    } else {
        edev.db[IOB_DATA_V].rows.get(iostd).unwrap().0
    }
}

fn get_ostd_row(
    edev: &prjcombine_virtex::expanded::ExpandedDevice<'_>,
    iostd: &str,
    drive: u8,
) -> TableRowId {
    let iostd = if iostd == "LVTTL" {
        format!("LVTTL_{drive}")
    } else {
        iostd.to_string()
    };
    if edev.chip.kind.is_virtexe() {
        edev.db[IOB_DATA_VE].rows.get(&iostd).unwrap().0
    } else {
        edev.db[IOB_DATA_V].rows.get(&iostd).unwrap().0
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let package = backend.ebonds.keys().next().unwrap();
    let ExpandedDevice::Virtex(edev) = backend.edev else {
        unreachable!()
    };
    for (_side, tcid, tcid_iob_v, tcid_iob_ve) in [
        (Dir::W, tcls::IO_W, tcls::IOB_W_V, tcls::IOB_W_VE),
        (Dir::E, tcls::IO_E, tcls::IOB_E_V, tcls::IOB_E_VE),
        (Dir::S, tcls::IO_S, tcls::IOB_S_V, tcls::IOB_S_VE),
        (Dir::N, tcls::IO_N, tcls::IOB_N_V, tcls::IOB_N_VE),
    ] {
        let tcid_iob = if edev.chip.kind.is_virtexe() {
            tcid_iob_ve
        } else {
            tcid_iob_v
        };
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(tcid, tcls::IO_S | tcls::IO_N)) {
                continue;
            }
            let mut bctx = ctx.bel(bslots::IOI[i]);
            let mode = "IOB";
            bctx.build()
                .global_mutex("VREF", "NO")
                .global("SHORTENJTAGCHAIN", "NO")
                .global("UNUSEDPIN", "PULLNONE")
                .prop(VirtexIsDllIob(bslots::IOI[i], false))
                .test_bel_special(specials::PRESENT)
                .mode(mode)
                .attr("TFFATTRBOX", "HIGH")
                .attr("OFFATTRBOX", "HIGH")
                .commit();
            if let Some(pkg) = has_any_vref(edev, backend.device, backend.db, tcid, bslots::IOI[i])
            {
                bctx.build()
                    .raw(Key::Package, pkg)
                    .global_mutex("VREF", "YES")
                    .prop(VirtexOtherIobInput(bslots::IOI[i], "GTL".to_string()))
                    .global("SHORTENJTAGCHAIN", "NO")
                    .global("UNUSEDPIN", "PULLNONE")
                    .prop(VirtexIsDllIob(bslots::IOI[i], false))
                    .prop(IsVref(bslots::IOI[i]))
                    .test_bel_special(specials::PRESENT_NOT_VREF)
                    .mode(mode)
                    .attr("TFFATTRBOX", "HIGH")
                    .attr("OFFATTRBOX", "HIGH")
                    .commit();
            }
            bctx.build()
                .global_mutex("VREF", "NO")
                .global("SHORTENJTAGCHAIN", "YES")
                .global("UNUSEDPIN", "PULLNONE")
                .prop(VirtexIsDllIob(bslots::IOI[i], false))
                .test_bel_attr_bits(IOI::SHORTEN_JTAG_CHAIN)
                .mode(mode)
                .attr("TFFATTRBOX", "HIGH")
                .attr("OFFATTRBOX", "HIGH")
                .commit();
            for (val, vname) in [(false, "1"), (false, "SR"), (true, "0"), (true, "SR_B")] {
                bctx.mode(mode)
                    .attr("IFF", "#FF")
                    .attr("IINITMUX", "0")
                    .pin("SR")
                    .test_bel_input_inv(IOI::SR, val)
                    .attr("SRMUX", vname)
                    .commit();
            }
            for (val, vname) in [(false, "1"), (false, "ICE"), (true, "0"), (true, "ICE_B")] {
                bctx.mode(mode)
                    .attr("IFF", "#FF")
                    .pin("ICE")
                    .test_bel_input_inv(IOI::ICE, val)
                    .attr("ICEMUX", vname)
                    .commit();
            }
            for (val, vname) in [(false, "1"), (false, "OCE"), (true, "0"), (true, "OCE_B")] {
                bctx.mode(mode)
                    .attr("OFF", "#FF")
                    .pin("OCE")
                    .test_bel_input_inv(IOI::OCE, val)
                    .attr("OCEMUX", vname)
                    .commit();
            }
            for (val, vname) in [(false, "1"), (false, "TCE"), (true, "0"), (true, "TCE_B")] {
                bctx.mode(mode)
                    .attr("TFF", "#FF")
                    .pin("TCE")
                    .test_bel_input_inv(IOI::TCE, val)
                    .attr("TCEMUX", vname)
                    .commit();
            }
            for (val, vname) in [(false, "1"), (false, "T"), (true, "0"), (true, "T_TB")] {
                bctx.mode(mode)
                    .global_mutex("DRIVE", "IOB")
                    .attr("TSEL", "1")
                    .pin("T")
                    .test_bel_input_inv(IOI::T, val)
                    .attr("TRIMUX", vname)
                    .commit();
            }
            for (val, vname) in [(false, "1"), (false, "O"), (true, "0"), (true, "O_B")] {
                bctx.mode(mode)
                    .global_mutex("DRIVE", "IOB")
                    .attr("OUTMUX", "1")
                    .pin("O")
                    .test_bel_input_inv(IOI::O, val)
                    .attr("OMUX", vname)
                    .commit();
            }
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .pin("CLK")
                .test_bel_input_inv_enum("ICKINV", IOI::ICLK, "1", "0");
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .pin("CLK")
                .test_bel_input_inv_enum("OCKINV", IOI::OCLK, "1", "0");
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .pin("CLK")
                .test_bel_input_inv_enum("TCKINV", IOI::TCLK, "1", "0");
            bctx.mode(mode)
                .attr("ICEMUX", "0")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_bel_attr_bool_rename("IFF", IOI::FFI_LATCH, "#FF", "#LATCH");
            bctx.mode(mode)
                .attr("OCEMUX", "0")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_bel_attr_bool_rename("OFF", IOI::FFO_LATCH, "#FF", "#LATCH");
            bctx.mode(mode)
                .attr("TCEMUX", "0")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_bel_attr_bool_rename("TFF", IOI::FFT_LATCH, "#FF", "#LATCH");
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_bel_attr_bits(IOI::FFI_SR_ENABLE)
                .attr("IINITMUX", "0")
                .commit();
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_bel_attr_bits(IOI::FFO_SR_ENABLE)
                .attr("OINITMUX", "0")
                .commit();
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_bel_attr_bits(IOI::FFT_SR_ENABLE)
                .attr("TINITMUX", "0")
                .commit();
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_bel_attr_bool_rename("IFFINITATTR", IOI::FFI_INIT, "LOW", "HIGH");
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_bel_attr_bool_rename("OFFATTRBOX", IOI::FFO_INIT, "LOW", "HIGH");
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_bel_attr_bool_rename("TFFATTRBOX", IOI::FFT_INIT, "LOW", "HIGH");
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .pin("IQ")
                .test_bel_attr_bool_rename("FFATTRBOX", IOI::FFI_SR_SYNC, "ASYNC", "SYNC");
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("IFFMUX", "1")
                .pin("IQ")
                .pin("I")
                .test_bel_attr_bool_rename("IMUX", IOI::I_DELAY_ENABLE, "1", "0");
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("IMUX", "1")
                .pin("IQ")
                .pin("I")
                .test_bel_attr_bool_rename("IFFMUX", IOI::FFI_DELAY_ENABLE, "1", "0");
            for (val, vname) in [(enums::IO_MUX_T::T, "1"), (enums::IO_MUX_T::FFT, "0")] {
                bctx.mode(mode)
                    .global_mutex("DRIVE", "IOB")
                    .attr("TFF", "#FF")
                    .attr("TRIMUX", "T")
                    .pin("T")
                    .test_bel_attr_val(IOI::MUX_T, val)
                    .attr("TSEL", vname)
                    .commit();
            }
            for (val, vname) in [(enums::IO_MUX_O::O, "1"), (enums::IO_MUX_O::FFO, "0")] {
                bctx.mode(mode)
                    .global_mutex("DRIVE", "IOB")
                    .attr("OFF", "#FF")
                    .attr("OMUX", "O")
                    .attr("TRIMUX", "T")
                    .attr("TSEL", "1")
                    .pin("O")
                    .pin("T")
                    .test_bel_attr_val(IOI::MUX_O, val)
                    .attr("OUTMUX", vname)
                    .commit();
            }
            for (val, vname) in [
                (enums::IOB_PULL::PULLDOWN, "PULLDOWN"),
                (enums::IOB_PULL::PULLUP, "PULLUP"),
                (enums::IOB_PULL::KEEPER, "KEEPER"),
            ] {
                bctx.mode(mode)
                    .null_bits()
                    .extra_tile_attr_val(Delta::new(0, 0, tcid_iob), bslots::IOB[i], IOB::PULL, val)
                    .attr("IMUX", "0")
                    .pin("I")
                    .test_bel_special(specials::IOB)
                    .attr("PULL", vname)
                    .commit();
            }
            let iostds_cmos = if !edev.chip.kind.is_virtexe() {
                IOSTDS_CMOS_V
            } else {
                IOSTDS_CMOS_VE
            };
            for &iostd in iostds_cmos {
                bctx.mode(mode)
                    .attr("OUTMUX", "")
                    .pin("I")
                    .prop(VirtexIsDllIob(bslots::IOI[i], false))
                    .test_bel_special_row(specials::IOB_ISTD, get_istd_row(edev, iostd))
                    .attr("IOATTRBOX", iostd)
                    .attr("IMUX", "1")
                    .commit();
                for (spec, slew) in [
                    (specials::IOB_OSTD_FAST, "FAST"),
                    (specials::IOB_OSTD_SLOW, "SLOW"),
                ] {
                    if iostd == "LVTTL" {
                        for drive in [2, 4, 6, 8, 12, 16, 24] {
                            bctx.mode(mode)
                                .global_mutex("DRIVE", "IOB")
                                .attr("IMUX", "")
                                .attr("IFFMUX", "")
                                .pin("O")
                                .pin("T")
                                .prop(VirtexIsDllIob(bslots::IOI[i], false))
                                .test_bel_special_row(spec, get_ostd_row(edev, iostd, drive))
                                .attr("IOATTRBOX", iostd)
                                .attr("DRIVEATTRBOX", drive.to_string())
                                .attr("SLEW", slew)
                                .attr("OMUX", "O_B")
                                .attr("OUTMUX", "1")
                                .attr("TRIMUX", "T")
                                .attr("TSEL", "1")
                                .commit();
                        }
                    } else {
                        bctx.mode(mode)
                            .global_mutex("DRIVE", "IOB")
                            .attr("IMUX", "")
                            .attr("IFFMUX", "")
                            .pin("O")
                            .pin("T")
                            .prop(VirtexIsDllIob(bslots::IOI[i], false))
                            .test_bel_special_row(spec, get_ostd_row(edev, iostd, 0))
                            .attr("IOATTRBOX", iostd)
                            .attr("SLEW", slew)
                            .attr("OMUX", "O_B")
                            .attr("OUTMUX", "1")
                            .attr("TRIMUX", "T")
                            .attr("TSEL", "1")
                            .commit();
                    }
                }
            }
            for &iostd in IOSTDS_VREF_LV.iter().chain(IOSTDS_VREF_HV) {
                bctx.mode(mode)
                    .global_mutex("VREF", "YES")
                    .raw(Key::Package, package)
                    .prop(VirtexOtherIobInput(bslots::IOI[i], iostd.to_string()))
                    .attr("OUTMUX", "")
                    .pin("I")
                    .prop(VirtexIsDllIob(bslots::IOI[i], false))
                    .test_bel_special_row(specials::IOB_ISTD, get_istd_row(edev, iostd))
                    .attr("IOATTRBOX", iostd)
                    .attr("IMUX", "1")
                    .commit();
                for (spec, slew) in [
                    (specials::IOB_OSTD_FAST, "FAST"),
                    (specials::IOB_OSTD_SLOW, "SLOW"),
                ] {
                    bctx.mode(mode)
                        .global_mutex("DRIVE", "IOB")
                        .attr("IMUX", "")
                        .attr("IFFMUX", "")
                        .pin("O")
                        .pin("T")
                        .prop(VirtexIsDllIob(bslots::IOI[i], false))
                        .test_bel_special_row(spec, get_ostd_row(edev, iostd, 0))
                        .attr("IOATTRBOX", iostd)
                        .attr("SLEW", slew)
                        .attr("OMUX", "O_B")
                        .attr("OUTMUX", "1")
                        .attr("TRIMUX", "T")
                        .attr("TSEL", "1")
                        .commit();
                }
            }
            if edev.chip.kind.is_virtexe() {
                for &iostd in IOSTDS_DIFF {
                    bctx.mode(mode)
                        .raw(Key::Package, package)
                        .global("UNUSEDPIN", "PULLNONE")
                        .attr("OUTMUX", "")
                        .pin("I")
                        .prop(VirtexIsDllIob(bslots::IOI[i], false))
                        .prop(IsDiff(bslots::IOI[i]))
                        .test_bel_special_row(specials::IOB_ISTD, get_istd_row(edev, iostd))
                        .attr("IOATTRBOX", iostd)
                        .attr("IMUX", "1")
                        .commit();
                    for (spec, slew) in [
                        (specials::IOB_OSTD_FAST, "FAST"),
                        (specials::IOB_OSTD_SLOW, "SLOW"),
                    ] {
                        bctx.mode(mode)
                            .global_mutex("DRIVE", "IOB")
                            .raw(Key::Package, package)
                            .global("UNUSEDPIN", "PULLNONE")
                            .attr("IMUX", "")
                            .attr("IFFMUX", "")
                            .pin("O")
                            .pin("T")
                            .prop(VirtexIsDllIob(bslots::IOI[i], false))
                            .prop(IsDiff(bslots::IOI[i]))
                            .test_bel_special_row(spec, get_ostd_row(edev, iostd, 0))
                            .attr("IOATTRBOX", iostd)
                            .attr("SLEW", slew)
                            .attr("OMUX", "O_B")
                            .attr("OUTMUX", "1")
                            .attr("TRIMUX", "T")
                            .attr("TSEL", "1")
                            .commit();
                    }
                }
                if tcid == tcls::IO_S || tcid == tcls::IO_N {
                    let row = if tcid == tcls::IO_S {
                        edev.chip.row_s()
                    } else {
                        edev.chip.row_n()
                    };
                    let bel_clk = if i == 1 {
                        bslots::IOFB[1]
                    } else {
                        bslots::IOFB[0]
                    };
                    let clkbt = DieId::from_idx(0)
                        .cell(edev.chip.col_clk(), row)
                        .tile(defs::tslots::CLK);
                    for &iostd in IOSTDS_CMOS_VE {
                        bctx.mode("DLLIOB")
                            .global_mutex("GCLKIOB", "NO")
                            .attr("OUTMUX", "")
                            .pin("DLLFB")
                            .pin("I")
                            .prop(VirtexIsDllIob(bslots::IOI[i], true))
                            .extra_fixed_bel_attr_val(
                                clkbt,
                                bel_clk,
                                IOFB::IBUF_MODE,
                                enums::IOB_IBUF_MODE::CMOS,
                            )
                            .test_bel_special_row(specials::IOB_ISTD, get_istd_row(edev, iostd))
                            .attr("IOATTRBOX", iostd)
                            .attr("DLLFBUSED", "0")
                            .attr("IMUX", "1")
                            .commit();
                    }
                    for &iostd in IOSTDS_VREF_LV.iter().chain(IOSTDS_VREF_HV) {
                        bctx.mode("DLLIOB")
                            .global_mutex("GCLKIOB", "NO")
                            .global_mutex("VREF", "YES")
                            .raw(Key::Package, package)
                            .prop(VirtexOtherIobInput(bslots::IOI[i], iostd.to_string()))
                            .attr("OUTMUX", "")
                            .pin("DLLFB")
                            .pin("I")
                            .prop(VirtexIsDllIob(bslots::IOI[i], true))
                            .extra_fixed_bel_attr_val(
                                clkbt,
                                bel_clk,
                                IOFB::IBUF_MODE,
                                enums::IOB_IBUF_MODE::VREF,
                            )
                            .test_bel_special_row(specials::IOB_ISTD, get_istd_row(edev, iostd))
                            .attr("IOATTRBOX", iostd)
                            .attr("DLLFBUSED", "0")
                            .attr("IMUX", "1")
                            .commit();
                    }
                }
            }
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    for (attr, bit, opt) in [
        (IOB::NDRIVE, 4, "IDNX"),
        (IOB::NDRIVE, 3, "IDNA"),
        (IOB::NDRIVE, 2, "IDNB"),
        (IOB::NDRIVE, 1, "IDNC"),
        (IOB::NDRIVE, 0, "IDND"),
        (IOB::PDRIVE, 3, "IDPA"),
        (IOB::PDRIVE, 2, "IDPB"),
        (IOB::PDRIVE, 1, "IDPC"),
        (IOB::PDRIVE, 0, "IDPD"),
    ] {
        for (val, vname) in [(false, "0"), (true, "1")] {
            ctx.build()
                .global_mutex("DRIVE", "GLOBAL")
                .extra_tiles_by_bel_attr_bits_base_bi(bslots::IOB[1], attr, bit, val)
                .test_global_special(specials::IOB)
                .global(opt, vname)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    for (side, tcid, tcid_iob_v, tcid_iob_ve) in [
        (Dir::W, tcls::IO_W, tcls::IOB_W_V, tcls::IOB_W_VE),
        (Dir::E, tcls::IO_E, tcls::IOB_E_V, tcls::IOB_E_VE),
        (Dir::S, tcls::IO_S, tcls::IOB_S_V, tcls::IOB_S_VE),
        (Dir::N, tcls::IO_N, tcls::IOB_N_V, tcls::IOB_N_VE),
    ] {
        let tcid_iob = if edev.chip.kind.is_virtexe() {
            tcid_iob_ve
        } else {
            tcid_iob_v
        };
        let mut pdrive_all = vec![];
        let mut ndrive_all = vec![];
        for i in 0..4 {
            pdrive_all.push(xlat_bit_wide_bi(
                ctx.get_diff_attr_bit_bi(tcid_iob, bslots::IOB[1], IOB::PDRIVE, i, false),
                ctx.get_diff_attr_bit_bi(tcid_iob, bslots::IOB[1], IOB::PDRIVE, i, true),
            ));
        }
        for i in 0..5 {
            ndrive_all.push(xlat_bit_wide_bi(
                ctx.get_diff_attr_bit_bi(tcid_iob, bslots::IOB[1], IOB::NDRIVE, i, false),
                ctx.get_diff_attr_bit_bi(tcid_iob, bslots::IOB[1], IOB::NDRIVE, i, true),
            ));
        }
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(side, Dir::S | Dir::N)) {
                continue;
            }
            let bslot = bslots::IOI[i];
            let bslot_iob = bslots::IOB[i];

            // IOI

            let present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
            let diff = ctx
                .get_diff_attr_bool(tcid, bslot, IOI::SHORTEN_JTAG_CHAIN)
                .combine(&!&present);
            let item = xlat_bit(!diff);
            ctx.insert_bel_attr_bool(tcid, bslot, IOI::SHORTEN_JTAG_CHAIN, item);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::SR);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::ICE);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::OCE);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::TCE);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::O);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::T);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::ICLK);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::OCLK);
            ctx.collect_bel_input_inv_bi(tcid, bslot, IOI::TCLK);
            ctx.collect_bel_attr(tcid, bslot, IOI::FFI_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, IOI::FFO_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, IOI::FFT_SR_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, IOI::FFI_INIT);
            ctx.collect_bel_attr_bi(tcid, bslot, IOI::FFO_INIT);
            ctx.collect_bel_attr_bi(tcid, bslot, IOI::FFT_INIT);
            ctx.get_diff_attr_bool_bi(tcid, bslot, IOI::FFI_SR_SYNC, false)
                .assert_empty();
            let mut diff = ctx.get_diff_attr_bool_bi(tcid, bslot, IOI::FFI_SR_SYNC, true);
            for (sr_sync, init) in [
                (IOI::FFI_SR_SYNC, IOI::FFI_INIT),
                (IOI::FFO_SR_SYNC, IOI::FFO_INIT),
                (IOI::FFT_SR_SYNC, IOI::FFT_INIT),
            ] {
                let init_bit = ctx.bel_attr_bit(tcid, bslot, init);
                let item = xlat_bit(diff.split_bits_by(|bit| {
                    bit.rect == init_bit.bit.rect
                        && bit.frame.to_idx().abs_diff(init_bit.bit.frame.to_idx()) == 1
                        && bit.bit == init_bit.bit.bit
                }));
                ctx.insert_bel_attr_bool(tcid, bslot, sr_sync, item);
            }
            diff.assert_empty();
            ctx.collect_bel_attr_bi(tcid, bslot, IOI::FFI_LATCH);
            ctx.collect_bel_attr_bi(tcid, bslot, IOI::FFO_LATCH);
            ctx.collect_bel_attr_bi(tcid, bslot, IOI::FFT_LATCH);
            ctx.collect_bel_attr_bi(tcid, bslot, IOI::I_DELAY_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, IOI::FFI_DELAY_ENABLE);

            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                IOI::FFI_READBACK,
                TileBit::new(
                    0,
                    match (side, i) {
                        (Dir::E, 1) => 2,
                        (Dir::E, 2) => 27,
                        (Dir::E, 3) => 32,
                        (_, 1) => 45,
                        (_, 2) => 20,
                        (_, 3) => 15,
                        _ => unreachable!(),
                    },
                    17,
                )
                .pos(),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                IOI::FFO_READBACK,
                TileBit::new(
                    0,
                    match (side, i) {
                        (Dir::E, 1) => 8,
                        (Dir::E, 2) => 21,
                        (Dir::E, 3) => 38,
                        (_, 1) => 39,
                        (_, 2) => 26,
                        (_, 3) => 9,
                        _ => unreachable!(),
                    },
                    17,
                )
                .pos(),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                IOI::FFT_READBACK,
                TileBit::new(
                    0,
                    match (side, i) {
                        (Dir::E, 1) => 12,
                        (Dir::E, 2) => 17,
                        (Dir::E, 3) => 42,
                        (_, 1) => 35,
                        (_, 2) => 30,
                        (_, 3) => 5,
                        _ => unreachable!(),
                    },
                    17,
                )
                .pos(),
            );

            // IOI + IOB

            ctx.get_diff_attr_val(tcid, bslot, IOI::MUX_T, enums::IO_MUX_T::T)
                .assert_empty();
            let mut diff = ctx.get_diff_attr_val(tcid, bslot, IOI::MUX_T, enums::IO_MUX_T::FFT);
            let diff_ioi =
                diff.split_bits_by(|bit| bit.frame.to_idx() < 48 && bit.bit.to_idx() == 16);
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                IOI::MUX_T,
                xlat_enum_attr(vec![
                    (enums::IO_MUX_T::T, Diff::default()),
                    (enums::IO_MUX_T::FFT, diff_ioi),
                ]),
            );
            ctx.insert_bel_attr_enum(
                tcid_iob,
                bslot_iob,
                IOB::MUX_T,
                xlat_enum_attr(vec![
                    (enums::IO_MUX_T::T, Diff::default()),
                    (enums::IO_MUX_T::FFT, diff),
                ]),
            );
            let mut diff = ctx
                .get_diff_attr_val(tcid, bslot, IOI::MUX_O, enums::IO_MUX_O::FFO)
                .combine(&!ctx.get_diff_attr_val(tcid, bslot, IOI::MUX_O, enums::IO_MUX_O::O));
            let diff_ioi =
                diff.split_bits_by(|bit| bit.frame.to_idx() < 48 && bit.bit.to_idx() == 16);
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                IOI::MUX_O,
                xlat_enum_attr(vec![
                    (enums::IO_MUX_O::O, Diff::default()),
                    (enums::IO_MUX_O::FFO, diff_ioi),
                ]),
            );
            ctx.insert_bel_attr_enum(
                tcid_iob,
                bslot_iob,
                IOB::MUX_O,
                xlat_enum_attr(vec![
                    (enums::IO_MUX_O::O, Diff::default()),
                    (enums::IO_MUX_O::FFO, diff),
                ]),
            );

            // IOB

            ctx.insert_bel_attr_bool(
                tcid_iob,
                bslot_iob,
                IOB::READBACK_I,
                match (side, i) {
                    (Dir::W | Dir::E, 1) => TileBit::new(0, 50, 13).pos(),
                    (Dir::W | Dir::E, 2) => TileBit::new(0, 50, 12).pos(),
                    (Dir::W | Dir::E, 3) => TileBit::new(0, 50, 2).pos(),
                    (Dir::S | Dir::N, 1) => TileBit::new(0, 25, 17).pos(),
                    (Dir::S | Dir::N, 2) => TileBit::new(0, 21, 17).pos(),
                    _ => unreachable!(),
                },
            );
            ctx.collect_bel_attr_default(tcid_iob, bslot_iob, IOB::PULL, enums::IOB_PULL::NONE);

            if has_any_vref(edev, ctx.device, ctx.db, tcid, bslots::IOI[i]).is_some() {
                let diff = present.combine(&!&ctx.get_diff_bel_special(
                    tcid,
                    bslot,
                    specials::PRESENT_NOT_VREF,
                ));
                ctx.insert_bel_attr_bool(tcid_iob, bslot_iob, IOB::VREF, xlat_bit(diff));
            }

            let (table, row_off) = if !edev.chip.kind.is_virtexe() {
                (IOB_DATA_V, IOB_DATA_V::OFF)
            } else {
                (IOB_DATA_VE, IOB_DATA_VE::OFF)
            };
            let mut diffs_istd = vec![];
            let mut diffs_iostd_misc = HashMap::new();
            let mut diffs_iostd_misc_vec = vec![(row_off, !&present)];
            let iostds: Vec<_> = if !edev.chip.kind.is_virtexe() {
                IOSTDS_CMOS_V
                    .iter()
                    .map(|&x| (enums::IOB_IBUF_MODE::CMOS, x))
                    .chain(
                        IOSTDS_VREF_LV
                            .iter()
                            .map(|&x| (enums::IOB_IBUF_MODE::VREF_LV, x)),
                    )
                    .chain(
                        IOSTDS_VREF_HV
                            .iter()
                            .map(|&x| (enums::IOB_IBUF_MODE::VREF_HV, x)),
                    )
                    .collect()
            } else {
                IOSTDS_CMOS_VE
                    .iter()
                    .map(|&x| (enums::IOB_IBUF_MODE::CMOS, x))
                    .chain(
                        IOSTDS_VREF_LV
                            .iter()
                            .map(|&x| (enums::IOB_IBUF_MODE::VREF, x)),
                    )
                    .chain(
                        IOSTDS_VREF_HV
                            .iter()
                            .map(|&x| (enums::IOB_IBUF_MODE::VREF, x)),
                    )
                    .chain(IOSTDS_DIFF.iter().map(|&x| (enums::IOB_IBUF_MODE::DIFF, x)))
                    .collect()
            };
            for &(kind, iostd) in &iostds {
                let diff_i = ctx.get_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD,
                    get_istd_row(edev, iostd),
                );
                let diff_o = ctx
                    .peek_diff_bel_special_row(
                        tcid,
                        bslot,
                        specials::IOB_OSTD_SLOW,
                        get_ostd_row(edev, iostd, 12),
                    )
                    .clone();
                let (diff_i, _, diff_c) = Diff::split(diff_i, diff_o);
                diffs_istd.push((kind, diff_i));
                if iostd == "LVTTL" {
                    for drive in [2, 4, 6, 8, 12, 16, 24] {
                        let row = get_ostd_row(edev, iostd, drive);
                        diffs_iostd_misc.insert(row, diff_c.clone());
                        diffs_iostd_misc_vec.push((row, diff_c.clone()));
                    }
                } else {
                    let row = get_ostd_row(edev, iostd, 12);
                    diffs_iostd_misc.insert(row, diff_c.clone());
                    diffs_iostd_misc_vec.push((row, diff_c));
                }
            }
            diffs_istd.push((enums::IOB_IBUF_MODE::NONE, Diff::default()));
            ctx.insert_bel_attr_enum(
                tcid_iob,
                bslot_iob,
                IOB::IBUF_MODE,
                xlat_enum_attr(diffs_istd),
            );

            let mut pdrive = vec![None; 4];
            let mut ndrive = vec![None; 5];
            for drive in [2, 4, 6, 8, 12, 16, 24] {
                let diff = ctx.peek_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_OSTD_SLOW,
                    get_ostd_row(edev, "LVTTL", drive),
                );
                for (i, bits) in pdrive_all.iter().enumerate() {
                    for &bit in bits {
                        if let Some(&pol) = diff.bits.get(&bit.bit) {
                            let bit = PolTileBit {
                                bit: bit.bit,
                                inv: !pol,
                            };
                            if pdrive[i].is_none() {
                                pdrive[i] = Some(bit);
                            }
                            assert_eq!(pdrive[i], Some(bit));
                        }
                    }
                }
                for (i, bits) in ndrive_all.iter().enumerate() {
                    for &bit in bits {
                        if let Some(&pol) = diff.bits.get(&bit.bit) {
                            let bit = PolTileBit {
                                bit: bit.bit,
                                inv: !pol,
                            };
                            if ndrive[i].is_none() {
                                ndrive[i] = Some(bit);
                            }
                            assert_eq!(ndrive[i], Some(bit));
                        }
                    }
                }
            }
            let pdrive: Vec<_> = pdrive.into_iter().map(|x| x.unwrap()).collect();
            let ndrive: Vec<_> = ndrive.into_iter().map(|x| x.unwrap()).collect();

            let slew_bits: HashSet<_> = ctx
                .peek_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_OSTD_FAST,
                    get_ostd_row(edev, "LVTTL", 24),
                )
                .combine(&!ctx.peek_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_OSTD_SLOW,
                    get_ostd_row(edev, "LVTTL", 24),
                ))
                .bits
                .into_keys()
                .collect();

            let mut slews = vec![((row_off, specials::IOB_OSTD_FAST), Diff::default())];
            let mut ostd_misc = vec![(row_off, Diff::default())];
            for (_, iostd) in iostds {
                if iostd == "LVTTL" {
                    for drive in [2, 4, 6, 8, 12, 16, 24] {
                        for spec in [specials::IOB_OSTD_FAST, specials::IOB_OSTD_SLOW] {
                            let row = get_ostd_row(edev, iostd, drive);
                            let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                            let pdrive_val: BitVec = pdrive
                                .iter()
                                .map(|&bit| {
                                    if let Some(val) = diff.bits.remove(&bit.bit) {
                                        assert_eq!(bit.inv, !val);
                                        true
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                            let ndrive_val: BitVec = ndrive
                                .iter()
                                .map(|&bit| {
                                    if let Some(val) = diff.bits.remove(&bit.bit) {
                                        assert_eq!(bit.inv, !val);
                                        true
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                            if !edev.chip.kind.is_virtexe() {
                                ctx.insert_table_bitvec(
                                    IOB_DATA_V,
                                    row,
                                    IOB_DATA_V::PDRIVE,
                                    pdrive_val,
                                );
                                ctx.insert_table_bitvec(
                                    IOB_DATA_V,
                                    row,
                                    IOB_DATA_V::NDRIVE,
                                    ndrive_val,
                                );
                            } else {
                                ctx.insert_table_bitvec(
                                    IOB_DATA_VE,
                                    row,
                                    IOB_DATA_VE::PDRIVE,
                                    pdrive_val,
                                );
                                ctx.insert_table_bitvec(
                                    IOB_DATA_VE,
                                    row,
                                    IOB_DATA_VE::NDRIVE,
                                    ndrive_val,
                                );
                            }
                            slews.push(((row, spec), diff.split_bits(&slew_bits)));
                            ostd_misc.push((row, diff))
                        }
                    }
                } else {
                    for spec in [specials::IOB_OSTD_FAST, specials::IOB_OSTD_SLOW] {
                        let row = get_ostd_row(edev, iostd, 0);
                        let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                        let pdrive_val: BitVec = pdrive
                            .iter()
                            .map(|&bit| {
                                if let Some(val) = diff.bits.remove(&bit.bit) {
                                    assert_eq!(bit.inv, !val);
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();
                        let ndrive_val: BitVec = ndrive
                            .iter()
                            .map(|&bit| {
                                if let Some(val) = diff.bits.remove(&bit.bit) {
                                    assert_eq!(bit.inv, !val);
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();
                        if !edev.chip.kind.is_virtexe() {
                            ctx.insert_table_bitvec(
                                IOB_DATA_V,
                                row,
                                IOB_DATA_V::PDRIVE,
                                pdrive_val,
                            );
                            ctx.insert_table_bitvec(
                                IOB_DATA_V,
                                row,
                                IOB_DATA_V::NDRIVE,
                                ndrive_val,
                            );
                        } else {
                            ctx.insert_table_bitvec(
                                IOB_DATA_VE,
                                row,
                                IOB_DATA_VE::PDRIVE,
                                pdrive_val,
                            );
                            ctx.insert_table_bitvec(
                                IOB_DATA_VE,
                                row,
                                IOB_DATA_VE::NDRIVE,
                                ndrive_val,
                            );
                        }
                        diff = diff.combine(&!&diffs_iostd_misc[&row]);
                        slews.push(((row, spec), diff.split_bits(&slew_bits)));
                        ostd_misc.push((row, diff))
                    }
                }
            }

            if !edev.chip.kind.is_virtexe() {
                ctx.insert_table_bitvec(
                    IOB_DATA_V,
                    IOB_DATA_V::OFF,
                    IOB_DATA_V::PDRIVE,
                    bits![0; 4],
                );
                ctx.insert_table_bitvec(
                    IOB_DATA_V,
                    IOB_DATA_V::OFF,
                    IOB_DATA_V::NDRIVE,
                    bits![0; 5],
                );
            } else {
                ctx.insert_table_bitvec(
                    IOB_DATA_VE,
                    IOB_DATA_VE::OFF,
                    IOB_DATA_VE::PDRIVE,
                    bits![0; 4],
                );
                ctx.insert_table_bitvec(
                    IOB_DATA_VE,
                    IOB_DATA_VE::OFF,
                    IOB_DATA_VE::NDRIVE,
                    bits![0; 5],
                );
            }

            ctx.insert_bel_attr_bitvec(tcid_iob, bslot_iob, IOB::PDRIVE, pdrive);
            ctx.insert_bel_attr_bitvec(tcid_iob, bslot_iob, IOB::NDRIVE, ndrive);

            let attrs = if !edev.chip.kind.is_virtexe() {
                [
                    (
                        IOB::V_IOSTD_MISC,
                        IOB_DATA_V::IOSTD_MISC,
                        diffs_iostd_misc_vec,
                    ),
                    (IOB::V_OUTPUT_MISC, IOB_DATA_V::OUTPUT_MISC, ostd_misc),
                ]
            } else {
                [
                    (
                        IOB::VE_IOSTD_MISC,
                        IOB_DATA_VE::IOSTD_MISC,
                        diffs_iostd_misc_vec,
                    ),
                    (IOB::VE_OUTPUT_MISC, IOB_DATA_VE::OUTPUT_MISC, ostd_misc),
                ]
            };
            for (attr, field, diffs) in attrs {
                let item = xlat_enum_raw(diffs, OcdMode::ValueOrder);
                let val_off = item.values[&row_off].clone();
                ctx.insert_bel_attr_bitvec(
                    tcid_iob,
                    bslot_iob,
                    attr,
                    Vec::from_iter(
                        item.bits
                            .iter()
                            .zip(val_off.iter())
                            .map(|(&bit, vo)| PolTileBit { bit, inv: vo }),
                    ),
                );
                for (row, val) in item.values {
                    ctx.insert_table_bitvec(table, row, field, &val ^ &val_off);
                }
            }

            let (attr, field_fast, field_slow) = if !edev.chip.kind.is_virtexe() {
                (IOB::V_SLEW, IOB_DATA_V::SLEW_FAST, IOB_DATA_V::SLEW_SLOW)
            } else {
                (IOB::VE_SLEW, IOB_DATA_VE::SLEW_FAST, IOB_DATA_VE::SLEW_SLOW)
            };
            let item = xlat_enum_raw(slews, OcdMode::ValueOrder);
            let val_off = item.values[&(row_off, specials::IOB_OSTD_FAST)].clone();
            ctx.insert_bel_attr_bitvec(
                tcid_iob,
                bslot_iob,
                attr,
                Vec::from_iter(
                    item.bits
                        .iter()
                        .zip(val_off.iter())
                        .map(|(&bit, vo)| PolTileBit { bit, inv: vo }),
                ),
            );
            for ((row, spec), val) in item.values {
                let field = if spec == specials::IOB_OSTD_FAST {
                    field_fast
                } else {
                    field_slow
                };
                ctx.insert_table_bitvec(table, row, field, &val ^ &val_off);
            }
        }
    }
    for tcid in [
        tcls::CLK_S_VE_2DLL,
        tcls::CLK_N_VE_2DLL,
        tcls::CLK_S_VE_4DLL,
        tcls::CLK_N_VE_4DLL,
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for bslot in bslots::IOFB {
            ctx.collect_bel_attr_subset_default_ocd(
                tcid,
                bslot,
                IOFB::IBUF_MODE,
                &[enums::IOB_IBUF_MODE::CMOS, enums::IOB_IBUF_MODE::VREF],
                enums::IOB_IBUF_MODE::NONE,
                OcdMode::ValueOrder,
            );
        }
    }
}
