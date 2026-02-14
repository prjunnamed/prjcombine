use std::collections::{HashMap, HashSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelSlotId, TileClassId},
    grid::{CellCoord, DieId, TileCoord},
};
use prjcombine_re_collector::{
    diff::Diff,
    legacy::{xlat_bit_bi_legacy, xlat_bit_legacy, xlat_bitvec_legacy, xlat_enum_legacy},
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{
    Bond, Device, ExpandedBond, ExpandedDevice, ExpandedNamedDevice, GeomDb,
};
use prjcombine_types::{
    bitvec::BitVec,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex::{
    chip::ChipKind,
    defs::{self, tcls},
};

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
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
        let is_dll = edev.chip.kind != prjcombine_virtex::chip::ChipKind::Virtex
            && ((tcrd.col == edev.chip.col_clk() - 1 && self.0 == defs::bslots::IO[1])
                || (tcrd.col == edev.chip.col_clk() && self.0 == defs::bslots::IO[2]));
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
        let (crd, orig_bank) = if defs::bslots::IO.contains(self.0) {
            let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
            (Some(crd), edev.chip.get_io_bank(crd))
        } else {
            (
                None,
                if tcrd.row == edev.chip.row_s() {
                    if self.0 == defs::bslots::GCLK_IO[0] {
                        4
                    } else {
                        5
                    }
                } else {
                    if self.0 == defs::bslots::GCLK_IO[0] {
                        1
                    } else {
                        0
                    }
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

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let package = backend.ebonds.keys().next().unwrap();
    let ExpandedDevice::Virtex(edev) = backend.edev else {
        unreachable!()
    };
    for tcid in [tcls::IO_W, tcls::IO_E, tcls::IO_S, tcls::IO_N] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(tcid, tcls::IO_S | tcls::IO_N)) {
                continue;
            }
            let mut bctx = ctx.bel(defs::bslots::IO[i]);
            let mode = "IOB";
            bctx.build()
                .global_mutex("VREF", "NO")
                .global("SHORTENJTAGCHAIN", "NO")
                .global("UNUSEDPIN", "PULLNONE")
                .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                .test_manual_legacy("PRESENT", "1")
                .mode(mode)
                .attr("TFFATTRBOX", "HIGH")
                .attr("OFFATTRBOX", "HIGH")
                .commit();
            if let Some(pkg) =
                has_any_vref(edev, backend.device, backend.db, tcid, defs::bslots::IO[i])
            {
                bctx.build()
                    .raw(Key::Package, pkg)
                    .global_mutex("VREF", "YES")
                    .prop(VirtexOtherIobInput(defs::bslots::IO[i], "GTL".to_string()))
                    .global("SHORTENJTAGCHAIN", "NO")
                    .global("UNUSEDPIN", "PULLNONE")
                    .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                    .prop(IsVref(defs::bslots::IO[i]))
                    .test_manual_legacy("PRESENT", "NOT_VREF")
                    .mode(mode)
                    .attr("TFFATTRBOX", "HIGH")
                    .attr("OFFATTRBOX", "HIGH")
                    .commit();
            }
            bctx.build()
                .global_mutex("VREF", "NO")
                .global("SHORTENJTAGCHAIN", "YES")
                .global("UNUSEDPIN", "PULLNONE")
                .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                .test_manual_legacy("SHORTEN_JTAG_CHAIN", "0")
                .mode(mode)
                .attr("TFFATTRBOX", "HIGH")
                .attr("OFFATTRBOX", "HIGH")
                .commit();
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("IINITMUX", "0")
                .pin("SR")
                .test_enum_legacy("SRMUX", &["0", "1", "SR", "SR_B"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .pin("ICE")
                .test_enum_legacy("ICEMUX", &["0", "1", "ICE", "ICE_B"]);
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .pin("OCE")
                .test_enum_legacy("OCEMUX", &["0", "1", "OCE", "OCE_B"]);
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .pin("TCE")
                .test_enum_legacy("TCEMUX", &["0", "1", "TCE", "TCE_B"]);
            bctx.mode(mode)
                .global_mutex("DRIVE", "IOB")
                .attr("TSEL", "1")
                .pin("T")
                .test_enum_legacy("TRIMUX", &["0", "1", "T", "T_TB"]);
            bctx.mode(mode)
                .global_mutex("DRIVE", "IOB")
                .attr("OUTMUX", "1")
                .pin("O")
                .test_enum_legacy("OMUX", &["0", "1", "O", "O_B"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .pin("CLK")
                .test_enum_legacy("ICKINV", &["0", "1"]);
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .pin("CLK")
                .test_enum_legacy("OCKINV", &["0", "1"]);
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .pin("CLK")
                .test_enum_legacy("TCKINV", &["0", "1"]);
            bctx.mode(mode)
                .attr("ICEMUX", "0")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_enum_legacy("IFF", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("OCEMUX", "0")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_enum_legacy("OFF", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("TCEMUX", "0")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_enum_legacy("TFF", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_enum_legacy("IINITMUX", &["0"]);
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_enum_legacy("OINITMUX", &["0"]);
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_enum_legacy("TINITMUX", &["0"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_enum_legacy("IFFINITATTR", &["LOW", "HIGH"]);
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_enum_legacy("OFFATTRBOX", &["LOW", "HIGH"]);
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_enum_legacy("TFFATTRBOX", &["LOW", "HIGH"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .pin("IQ")
                .test_enum_legacy("FFATTRBOX", &["SYNC", "ASYNC"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("IFFMUX", "1")
                .pin("IQ")
                .pin("I")
                .test_enum_legacy("IMUX", &["0", "1"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("IMUX", "1")
                .pin("IQ")
                .pin("I")
                .test_enum_legacy("IFFMUX", &["0", "1"]);
            bctx.mode(mode)
                .global_mutex("DRIVE", "IOB")
                .attr("TFF", "#FF")
                .attr("TRIMUX", "T")
                .pin("T")
                .test_enum_legacy("TSEL", &["0", "1"]);
            bctx.mode(mode)
                .global_mutex("DRIVE", "IOB")
                .attr("OFF", "#FF")
                .attr("OMUX", "O")
                .attr("TRIMUX", "T")
                .attr("TSEL", "1")
                .pin("O")
                .pin("T")
                .test_enum_legacy("OUTMUX", &["0", "1"]);
            bctx.mode(mode)
                .attr("IMUX", "0")
                .pin("I")
                .test_enum_legacy("PULL", &["PULLDOWN", "PULLUP", "KEEPER"]);
            let iostds_cmos = if edev.chip.kind == ChipKind::Virtex {
                IOSTDS_CMOS_V
            } else {
                IOSTDS_CMOS_VE
            };
            for &iostd in iostds_cmos {
                bctx.mode(mode)
                    .attr("OUTMUX", "")
                    .pin("I")
                    .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                    .test_manual_legacy("ISTD", iostd)
                    .attr("IOATTRBOX", iostd)
                    .attr("IMUX", "1")
                    .commit();
                for slew in ["FAST", "SLOW"] {
                    if iostd == "LVTTL" {
                        for drive in ["2", "4", "6", "8", "12", "16", "24"] {
                            bctx.mode(mode)
                                .global_mutex("DRIVE", "IOB")
                                .attr("IMUX", "")
                                .attr("IFFMUX", "")
                                .pin("O")
                                .pin("T")
                                .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                                .test_manual_legacy("OSTD", format!("{iostd}.{drive}.{slew}"))
                                .attr("IOATTRBOX", iostd)
                                .attr("DRIVEATTRBOX", drive)
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
                            .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                            .test_manual_legacy("OSTD", format!("{iostd}.{slew}"))
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
                    .prop(VirtexOtherIobInput(defs::bslots::IO[i], iostd.to_string()))
                    .attr("OUTMUX", "")
                    .pin("I")
                    .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                    .test_manual_legacy("ISTD", iostd)
                    .attr("IOATTRBOX", iostd)
                    .attr("IMUX", "1")
                    .commit();
                for slew in ["FAST", "SLOW"] {
                    bctx.mode(mode)
                        .global_mutex("DRIVE", "IOB")
                        .attr("IMUX", "")
                        .attr("IFFMUX", "")
                        .pin("O")
                        .pin("T")
                        .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                        .test_manual_legacy("OSTD", format!("{iostd}.{slew}"))
                        .attr("IOATTRBOX", iostd)
                        .attr("SLEW", slew)
                        .attr("OMUX", "O_B")
                        .attr("OUTMUX", "1")
                        .attr("TRIMUX", "T")
                        .attr("TSEL", "1")
                        .commit();
                }
            }
            if edev.chip.kind != ChipKind::Virtex {
                for &iostd in IOSTDS_DIFF {
                    bctx.mode(mode)
                        .raw(Key::Package, package)
                        .global("UNUSEDPIN", "PULLNONE")
                        .attr("OUTMUX", "")
                        .pin("I")
                        .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                        .prop(IsDiff(defs::bslots::IO[i]))
                        .test_manual_legacy("ISTD", iostd)
                        .attr("IOATTRBOX", iostd)
                        .attr("IMUX", "1")
                        .commit();
                    for slew in ["FAST", "SLOW"] {
                        bctx.mode(mode)
                            .global_mutex("DRIVE", "IOB")
                            .raw(Key::Package, package)
                            .global("UNUSEDPIN", "PULLNONE")
                            .attr("IMUX", "")
                            .attr("IFFMUX", "")
                            .pin("O")
                            .pin("T")
                            .prop(VirtexIsDllIob(defs::bslots::IO[i], false))
                            .prop(IsDiff(defs::bslots::IO[i]))
                            .test_manual_legacy("OSTD", format!("{iostd}.{slew}"))
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
                    let bel_clk = if i == 1 { "IOFB[1]" } else { "IOFB[0]" };
                    let clkbt = CellCoord::new(DieId::from_idx(0), edev.chip.col_clk(), row)
                        .tile(defs::tslots::CLK_SN);
                    for &iostd in IOSTDS_CMOS_VE {
                        bctx.mode("DLLIOB")
                            .global_mutex("GCLKIOB", "NO")
                            .attr("OUTMUX", "")
                            .pin("DLLFB")
                            .pin("I")
                            .prop(VirtexIsDllIob(defs::bslots::IO[i], true))
                            .extra_tile_attr_fixed_legacy(clkbt, bel_clk, "IBUF", "CMOS")
                            .test_manual_legacy("ISTD", iostd)
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
                            .prop(VirtexOtherIobInput(defs::bslots::IO[i], iostd.to_string()))
                            .attr("OUTMUX", "")
                            .pin("DLLFB")
                            .pin("I")
                            .prop(VirtexIsDllIob(defs::bslots::IO[i], true))
                            .extra_tile_attr_fixed_legacy(clkbt, bel_clk, "IBUF", "VREF")
                            .test_manual_legacy("ISTD", iostd)
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
    for attr in [
        "IDNX", "IDNA", "IDNB", "IDNC", "IDND", "IDPA", "IDPB", "IDPC", "IDPD",
    ] {
        for val in ["0", "1"] {
            ctx.build()
                .global_mutex("DRIVE", "GLOBAL")
                .extra_tiles_by_bel_legacy(defs::bslots::IO[0], "IOB_ALL")
                .test_manual_legacy("IOB_ALL", attr, val)
                .global(attr, val)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    let kind = match edev.chip.kind {
        ChipKind::Virtex => "V",
        ChipKind::VirtexE | ChipKind::VirtexEM => "VE",
    };
    for side in ['W', 'E', 'S', 'N'] {
        let tile = &format!("IO_{side}");
        let tcid = edev.db.get_tile_class(tile);
        let tile_iob = &format!("IOB_{side}_{kind}");
        let mut pdrive_all = vec![];
        let mut ndrive_all = vec![];
        for attr in ["IDPD", "IDPC", "IDPB", "IDPA"] {
            pdrive_all.push(
                ctx.extract_bit_wide_bi_legacy(tile, "IOB_ALL", attr, "0", "1")
                    .bits,
            );
        }
        for attr in ["IDND", "IDNC", "IDNB", "IDNA", "IDNX"] {
            ndrive_all.push(
                ctx.extract_bit_wide_bi_legacy(tile, "IOB_ALL", attr, "0", "1")
                    .bits,
            );
        }
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(side, 'S' | 'N')) {
                continue;
            }
            let bel = &format!("IO[{i}]");

            // IOI

            let present = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");
            let diff = ctx
                .get_diff_legacy(tile, bel, "SHORTEN_JTAG_CHAIN", "0")
                .combine(&!&present);
            let item = xlat_bit_legacy(!diff);
            ctx.insert_legacy(tile, bel, "SHORTEN_JTAG_CHAIN", item);
            for (pin, pin_b, pinmux) in [
                ("SR", "SR_B", "SRMUX"),
                ("ICE", "ICE_B", "ICEMUX"),
                ("OCE", "OCE_B", "OCEMUX"),
                ("TCE", "TCE_B", "TCEMUX"),
                ("T", "T_TB", "TRIMUX"),
                ("O", "O_B", "OMUX"),
            ] {
                let diff0 = ctx.get_diff_legacy(tile, bel, pinmux, "1");
                assert_eq!(diff0, ctx.get_diff_legacy(tile, bel, pinmux, pin));
                let diff1 = ctx.get_diff_legacy(tile, bel, pinmux, "0");
                assert_eq!(diff1, ctx.get_diff_legacy(tile, bel, pinmux, pin_b));
                let item = xlat_bit_bi_legacy(diff0, diff1);
                ctx.insert_legacy(tile, bel, format!("INV.{pin}"), item);
            }
            for iot in ['I', 'O', 'T'] {
                let item = ctx.extract_bit_bi_legacy(tile, bel, &format!("{iot}CKINV"), "1", "0");
                ctx.insert_legacy(tile, bel, format!("INV.{iot}FF.CLK"), item);
                let item = ctx.extract_bit_legacy(tile, bel, &format!("{iot}INITMUX"), "0");
                ctx.insert_legacy(tile, bel, format!("{iot}FF_SR_ENABLE"), item);
            }
            let item = ctx.extract_bit_bi_legacy(tile, bel, "IFFINITATTR", "LOW", "HIGH");
            ctx.insert_legacy(tile, bel, "IFF_INIT", item);
            let item = ctx.extract_bit_bi_legacy(tile, bel, "OFFATTRBOX", "LOW", "HIGH");
            ctx.insert_legacy(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_bit_bi_legacy(tile, bel, "TFFATTRBOX", "LOW", "HIGH");
            ctx.insert_legacy(tile, bel, "TFF_INIT", item);
            ctx.get_diff_legacy(tile, bel, "FFATTRBOX", "ASYNC")
                .assert_empty();
            let mut diff = ctx.get_diff_legacy(tile, bel, "FFATTRBOX", "SYNC");
            for iot in ['I', 'O', 'T'] {
                let init = ctx.item_legacy(tile, bel, &format!("{iot}FF_INIT"));
                let init_bit = init.bits[0];
                let item = xlat_bitvec_legacy(vec![diff.split_bits_by(|bit| {
                    bit.rect == init_bit.rect
                        && bit.frame.to_idx().abs_diff(init_bit.frame.to_idx()) == 1
                        && bit.bit == init_bit.bit
                })]);
                ctx.insert_legacy(tile, bel, format!("{iot}FF_SR_SYNC"), item);
            }
            diff.assert_empty();
            let item = ctx.extract_bit_bi_legacy(tile, bel, "IFF", "#FF", "#LATCH");
            ctx.insert_legacy(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_bit_bi_legacy(tile, bel, "OFF", "#FF", "#LATCH");
            ctx.insert_legacy(tile, bel, "OFF_LATCH", item);
            let item = ctx.extract_bit_bi_legacy(tile, bel, "TFF", "#FF", "#LATCH");
            ctx.insert_legacy(tile, bel, "TFF_LATCH", item);
            let item = ctx.extract_bit_bi_legacy(tile, bel, "IMUX", "1", "0");
            ctx.insert_legacy(tile, bel, "I_DELAY_ENABLE", item);
            let item = ctx.extract_bit_bi_legacy(tile, bel, "IFFMUX", "1", "0");
            ctx.insert_legacy(tile, bel, "IFF_DELAY_ENABLE", item);

            ctx.insert_legacy(
                tile,
                bel,
                "READBACK_IFF",
                TileItem::from_bit_inv(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('E', 1) => 2,
                            ('E', 2) => 27,
                            ('E', 3) => 32,
                            (_, 1) => 45,
                            (_, 2) => 20,
                            (_, 3) => 15,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );
            ctx.insert_legacy(
                tile,
                bel,
                "READBACK_OFF",
                TileItem::from_bit_inv(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('E', 1) => 8,
                            ('E', 2) => 21,
                            ('E', 3) => 38,
                            (_, 1) => 39,
                            (_, 2) => 26,
                            (_, 3) => 9,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );
            ctx.insert_legacy(
                tile,
                bel,
                "READBACK_TFF",
                TileItem::from_bit_inv(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('E', 1) => 12,
                            ('E', 2) => 17,
                            ('E', 3) => 42,
                            (_, 1) => 35,
                            (_, 2) => 30,
                            (_, 3) => 5,
                            _ => unreachable!(),
                        },
                        17,
                    ),
                    false,
                ),
            );

            // IOI + IOB

            ctx.get_diff_legacy(tile, bel, "TSEL", "1").assert_empty();
            let mut diff = ctx.get_diff_legacy(tile, bel, "TSEL", "0");
            let diff_ioi =
                diff.split_bits_by(|bit| bit.frame.to_idx() < 48 && bit.bit.to_idx() == 16);
            ctx.insert_legacy(
                tile,
                bel,
                "TMUX",
                xlat_enum_legacy(vec![("T", Diff::default()), ("TFF", diff_ioi)]),
            );
            ctx.insert_legacy(
                tile_iob,
                bel,
                "TMUX",
                xlat_enum_legacy(vec![("T", Diff::default()), ("TFF", diff)]),
            );
            let mut diff = ctx
                .get_diff_legacy(tile, bel, "OUTMUX", "0")
                .combine(&!ctx.get_diff_legacy(tile, bel, "OUTMUX", "1"));
            let diff_ioi =
                diff.split_bits_by(|bit| bit.frame.to_idx() < 48 && bit.bit.to_idx() == 16);
            ctx.insert_legacy(
                tile,
                bel,
                "OMUX",
                xlat_enum_legacy(vec![("O", Diff::default()), ("OFF", diff_ioi)]),
            );
            ctx.insert_legacy(
                tile_iob,
                bel,
                "OMUX",
                xlat_enum_legacy(vec![("O", Diff::default()), ("OFF", diff)]),
            );

            // IOB

            ctx.insert_legacy(
                tile_iob,
                bel,
                "READBACK_I",
                TileItem::from_bit_inv(
                    match (side, i) {
                        ('W' | 'E', 1) => TileBit::new(0, 50, 13),
                        ('W' | 'E', 2) => TileBit::new(0, 50, 12),
                        ('W' | 'E', 3) => TileBit::new(0, 50, 2),
                        ('S' | 'N', 1) => TileBit::new(0, 25, 17),
                        ('S' | 'N', 2) => TileBit::new(0, 21, 17),
                        _ => unreachable!(),
                    },
                    false,
                ),
            );
            let item = ctx.extract_enum_default_legacy(
                tile,
                bel,
                "PULL",
                &["PULLDOWN", "PULLUP", "KEEPER"],
                "NONE",
            );
            ctx.insert_legacy(tile_iob, bel, "PULL", item);

            if has_any_vref(edev, ctx.device, ctx.db, tcid, defs::bslots::IO[i]).is_some() {
                let diff =
                    present.combine(&!&ctx.get_diff_legacy(tile, bel, "PRESENT", "NOT_VREF"));
                ctx.insert_legacy(tile_iob, bel, "VREF", xlat_bit_legacy(diff));
            }

            let mut diffs_istd = vec![];
            let mut diffs_iostd_misc = HashMap::new();
            let mut diffs_iostd_misc_vec = vec![("NONE", !&present)];
            let iostds: Vec<_> = if edev.chip.kind == ChipKind::Virtex {
                IOSTDS_CMOS_V
                    .iter()
                    .map(|&x| ("CMOS", x))
                    .chain(IOSTDS_VREF_LV.iter().map(|&x| ("VREF_LV", x)))
                    .chain(IOSTDS_VREF_HV.iter().map(|&x| ("VREF_HV", x)))
                    .collect()
            } else {
                IOSTDS_CMOS_VE
                    .iter()
                    .map(|&x| ("CMOS", x))
                    .chain(IOSTDS_VREF_LV.iter().map(|&x| ("VREF", x)))
                    .chain(IOSTDS_VREF_HV.iter().map(|&x| ("VREF", x)))
                    .chain(IOSTDS_DIFF.iter().map(|&x| ("DIFF", x)))
                    .collect()
            };
            for &(kind, iostd) in &iostds {
                let diff_i = ctx.get_diff_legacy(tile, bel, "ISTD", iostd);
                let diff_o = if iostd == "LVTTL" {
                    ctx.peek_diff_legacy(tile, bel, "OSTD", format!("{iostd}.12.SLOW"))
                } else {
                    ctx.peek_diff_legacy(tile, bel, "OSTD", format!("{iostd}.SLOW"))
                }
                .clone();
                let (diff_i, _, diff_c) = Diff::split(diff_i, diff_o);
                diffs_istd.push((kind, diff_i));
                diffs_iostd_misc.insert(iostd, diff_c.clone());
                diffs_iostd_misc_vec.push((iostd, diff_c));
            }
            diffs_istd.push(("NONE", Diff::default()));
            ctx.insert_legacy(tile_iob, bel, "IBUF", xlat_enum_legacy(diffs_istd));

            let mut pdrive = vec![None; 4];
            let mut ndrive = vec![None; 5];
            for drive in ["2", "4", "6", "8", "12", "16", "24"] {
                let diff = ctx.peek_diff_legacy(tile, bel, "OSTD", format!("LVTTL.{drive}.SLOW"));
                for (i, bits) in pdrive_all.iter().enumerate() {
                    for &bit in bits {
                        if let Some(&pol) = diff.bits.get(&bit) {
                            if pdrive[i].is_none() {
                                pdrive[i] = Some((bit, !pol));
                            }
                            assert_eq!(pdrive[i], Some((bit, !pol)));
                        }
                    }
                }
                for (i, bits) in ndrive_all.iter().enumerate() {
                    for &bit in bits {
                        if let Some(&pol) = diff.bits.get(&bit) {
                            if ndrive[i].is_none() {
                                ndrive[i] = Some((bit, !pol));
                            }
                            assert_eq!(ndrive[i], Some((bit, !pol)));
                        }
                    }
                }
            }
            let pdrive: Vec<_> = pdrive.into_iter().map(|x| x.unwrap()).collect();
            let ndrive: Vec<_> = ndrive.into_iter().map(|x| x.unwrap()).collect();

            let slew_bits: HashSet<_> = ctx
                .peek_diff_legacy(tile, bel, "OSTD", "LVTTL.24.FAST")
                .combine(&!ctx.peek_diff_legacy(tile, bel, "OSTD", "LVTTL.24.SLOW"))
                .bits
                .into_keys()
                .collect();

            let tag = if edev.chip.kind == ChipKind::Virtex {
                "V"
            } else {
                "VE"
            };

            let mut slews = vec![("NONE".to_string(), Diff::default())];
            let mut ostd_misc = vec![("NONE", Diff::default())];
            for (_, iostd) in iostds {
                if iostd == "LVTTL" {
                    for drive in ["2", "4", "6", "8", "12", "16", "24"] {
                        for slew in ["SLOW", "FAST"] {
                            let mut diff = ctx.get_diff_legacy(
                                tile,
                                bel,
                                "OSTD",
                                format!("{iostd}.{drive}.{slew}"),
                            );
                            let pdrive_val: BitVec = pdrive
                                .iter()
                                .map(|&(bit, inv)| {
                                    if let Some(val) = diff.bits.remove(&bit) {
                                        assert_eq!(inv, !val);
                                        true
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                            ctx.insert_misc_data_legacy(
                                format!("IOSTD:{tag}:PDRIVE:{iostd}.{drive}"),
                                pdrive_val,
                            );
                            let ndrive_val: BitVec = ndrive
                                .iter()
                                .map(|&(bit, inv)| {
                                    if let Some(val) = diff.bits.remove(&bit) {
                                        assert_eq!(inv, !val);
                                        true
                                    } else {
                                        false
                                    }
                                })
                                .collect();
                            ctx.insert_misc_data_legacy(
                                format!("IOSTD:{tag}:NDRIVE:{iostd}.{drive}"),
                                ndrive_val,
                            );
                            slews.push((
                                format!("{iostd}.{drive}.{slew}"),
                                diff.split_bits(&slew_bits),
                            ));
                            ostd_misc.push((iostd, diff))
                        }
                    }
                } else {
                    for slew in ["SLOW", "FAST"] {
                        let mut diff =
                            ctx.get_diff_legacy(tile, bel, "OSTD", format!("{iostd}.{slew}"));
                        let pdrive_val: BitVec = pdrive
                            .iter()
                            .map(|&(bit, inv)| {
                                if let Some(val) = diff.bits.remove(&bit) {
                                    assert_eq!(inv, !val);
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();
                        ctx.insert_misc_data_legacy(
                            format!("IOSTD:{tag}:PDRIVE:{iostd}"),
                            pdrive_val,
                        );
                        let ndrive_val: BitVec = ndrive
                            .iter()
                            .map(|&(bit, inv)| {
                                if let Some(val) = diff.bits.remove(&bit) {
                                    assert_eq!(inv, !val);
                                    true
                                } else {
                                    false
                                }
                            })
                            .collect();
                        diff = diff.combine(&!&diffs_iostd_misc[iostd]);
                        ctx.insert_misc_data_legacy(
                            format!("IOSTD:{tag}:NDRIVE:{iostd}"),
                            ndrive_val,
                        );
                        slews.push((format!("{iostd}.{slew}"), diff.split_bits(&slew_bits)));
                        ostd_misc.push((iostd, diff))
                    }
                }
            }

            ctx.insert_legacy(
                tile_iob,
                bel,
                "PDRIVE",
                TileItem {
                    bits: pdrive.iter().map(|&(bit, _)| bit).collect(),
                    kind: TileItemKind::BitVec {
                        invert: pdrive.iter().map(|&(_, pol)| pol).collect(),
                    },
                },
            );
            ctx.insert_legacy(
                tile_iob,
                bel,
                "NDRIVE",
                TileItem {
                    bits: ndrive.iter().map(|&(bit, _)| bit).collect(),
                    kind: TileItemKind::BitVec {
                        invert: ndrive.iter().map(|&(_, pol)| pol).collect(),
                    },
                },
            );

            for (attr, item) in [
                ("IOSTD_MISC", xlat_enum_legacy(diffs_iostd_misc_vec)),
                ("OUTPUT_MISC", xlat_enum_legacy(ostd_misc)),
                ("SLEW", xlat_enum_legacy(slews)),
            ] {
                let TileItemKind::Enum { values } = item.kind else {
                    unreachable!()
                };
                for (name, val) in values {
                    ctx.insert_misc_data_legacy(format!("IOSTD:{tag}:{attr}:{name}"), val);
                }
                let item = TileItem::from_bitvec_inv(item.bits, false);
                ctx.insert_legacy(tile_iob, bel, attr, item);
            }
        }
    }
    if edev.chip.kind != ChipKind::Virtex {
        for tile in if ctx.device.name.contains("2s") {
            ["CLK_S_VE_2DLL", "CLK_N_VE_2DLL"]
        } else {
            ["CLK_S_VE_4DLL", "CLK_N_VE_4DLL"]
        } {
            for bel in ["IOFB[0]", "IOFB[1]"] {
                ctx.collect_enum_default_legacy(tile, bel, "IBUF", &["CMOS", "VREF"], "NONE");
            }
        }
    }
}
