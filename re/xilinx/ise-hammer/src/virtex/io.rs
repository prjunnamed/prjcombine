use std::collections::{HashMap, HashSet};

use bitvec::vec::BitVec;
use prjcombine_interconnect::{
    db::BelSlotId,
    grid::{DieId, NodeLoc},
};
use prjcombine_re_fpga_hammer::{Diff, FuzzerProp, xlat_bit, xlat_bitvec, xlat_bool, xlat_enum};
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{
    Bond, Device, ExpandedBond, ExpandedDevice, ExpandedNamedDevice, GeomDb,
};
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_virtex::{bels, chip::ChipKind};
use unnamed_entity::EntityId;

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
        nloc: NodeLoc,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex(edev) = backend.edev else {
            unreachable!()
        };
        let is_dll = edev.chip.kind != prjcombine_virtex::chip::ChipKind::Virtex
            && ((nloc.1 == edev.chip.col_clk() - 1 && self.0 == bels::IO[1])
                || (nloc.1 == edev.chip.col_clk() && self.0 == bels::IO[2]));
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
        nloc: NodeLoc,
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
        let crd = endev.grid.get_io_crd((nloc.0, (nloc.1, nloc.2), self.0));
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
        nloc: NodeLoc,
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
        let crd = endev.grid.get_io_crd((nloc.0, (nloc.1, nloc.2), self.0));
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
        nloc: NodeLoc,
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
        let (crd, orig_bank) = if bels::IO.contains(&self.0) {
            let crd = edev.chip.get_io_crd((nloc.0, (nloc.1, nloc.2), self.0));
            (Some(crd), edev.chip.get_io_bank(crd))
        } else {
            (
                None,
                if nloc.2 == edev.chip.row_s() {
                    if self.0 == bels::GCLK_IO0 { 4 } else { 5 }
                } else {
                    if self.0 == bels::GCLK_IO0 { 1 } else { 0 }
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
    tile: &str,
    slot: BelSlotId,
) -> Option<&'a str> {
    let node_kind = edev.egrid.db.get_node(tile);
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
    for &(die, col, row, _) in &edev.egrid.node_index[node_kind] {
        let crd = edev.chip.get_io_crd((die, (col, row), slot));
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
    for side in ['L', 'R', 'B', 'T'] {
        let tile = format!("IO.{side}");
        let mut ctx = FuzzCtx::new(session, backend, &tile);
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(side, 'B' | 'T')) {
                continue;
            }
            let mut bctx = ctx.bel(bels::IO[i]);
            let mode = "IOB";
            bctx.build()
                .global_mutex("VREF", "NO")
                .global("SHORTENJTAGCHAIN", "NO")
                .global("UNUSEDPIN", "PULLNONE")
                .prop(VirtexIsDllIob(bels::IO[i], false))
                .test_manual("PRESENT", "1")
                .mode(mode)
                .attr("TFFATTRBOX", "HIGH")
                .attr("OFFATTRBOX", "HIGH")
                .commit();
            if let Some(pkg) = has_any_vref(edev, backend.device, backend.db, &tile, bels::IO[i]) {
                bctx.build()
                    .raw(Key::Package, pkg)
                    .global_mutex("VREF", "YES")
                    .prop(VirtexOtherIobInput(bels::IO[i], "GTL".to_string()))
                    .global("SHORTENJTAGCHAIN", "NO")
                    .global("UNUSEDPIN", "PULLNONE")
                    .prop(VirtexIsDllIob(bels::IO[i], false))
                    .prop(IsVref(bels::IO[i]))
                    .test_manual("PRESENT", "NOT_VREF")
                    .mode(mode)
                    .attr("TFFATTRBOX", "HIGH")
                    .attr("OFFATTRBOX", "HIGH")
                    .commit();
            }
            bctx.build()
                .global_mutex("VREF", "NO")
                .global("SHORTENJTAGCHAIN", "YES")
                .global("UNUSEDPIN", "PULLNONE")
                .prop(VirtexIsDllIob(bels::IO[i], false))
                .test_manual("SHORTEN_JTAG_CHAIN", "0")
                .mode(mode)
                .attr("TFFATTRBOX", "HIGH")
                .attr("OFFATTRBOX", "HIGH")
                .commit();
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("IINITMUX", "0")
                .pin("SR")
                .test_enum("SRMUX", &["0", "1", "SR", "SR_B"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .pin("ICE")
                .test_enum("ICEMUX", &["0", "1", "ICE", "ICE_B"]);
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .pin("OCE")
                .test_enum("OCEMUX", &["0", "1", "OCE", "OCE_B"]);
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .pin("TCE")
                .test_enum("TCEMUX", &["0", "1", "TCE", "TCE_B"]);
            bctx.mode(mode)
                .global_mutex("DRIVE", "IOB")
                .attr("TSEL", "1")
                .pin("T")
                .test_enum("TRIMUX", &["0", "1", "T", "T_TB"]);
            bctx.mode(mode)
                .global_mutex("DRIVE", "IOB")
                .attr("OUTMUX", "1")
                .pin("O")
                .test_enum("OMUX", &["0", "1", "O", "O_B"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .pin("CLK")
                .test_enum("ICKINV", &["0", "1"]);
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .pin("CLK")
                .test_enum("OCKINV", &["0", "1"]);
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .pin("CLK")
                .test_enum("TCKINV", &["0", "1"]);
            bctx.mode(mode)
                .attr("ICEMUX", "0")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_enum("IFF", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("OCEMUX", "0")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_enum("OFF", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("TCEMUX", "0")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_enum("TFF", &["#FF", "#LATCH"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_enum("IINITMUX", &["0"]);
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_enum("OINITMUX", &["0"]);
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_enum("TINITMUX", &["0"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("ICKINV", "1")
                .pin("CLK")
                .test_enum("IFFINITATTR", &["LOW", "HIGH"]);
            bctx.mode(mode)
                .attr("OFF", "#FF")
                .attr("OCKINV", "1")
                .pin("CLK")
                .test_enum("OFFATTRBOX", &["LOW", "HIGH"]);
            bctx.mode(mode)
                .attr("TFF", "#FF")
                .attr("TCKINV", "1")
                .pin("CLK")
                .test_enum("TFFATTRBOX", &["LOW", "HIGH"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .pin("IQ")
                .test_enum("FFATTRBOX", &["SYNC", "ASYNC"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("IFFMUX", "1")
                .pin("IQ")
                .pin("I")
                .test_enum("IMUX", &["0", "1"]);
            bctx.mode(mode)
                .attr("IFF", "#FF")
                .attr("IMUX", "1")
                .pin("IQ")
                .pin("I")
                .test_enum("IFFMUX", &["0", "1"]);
            bctx.mode(mode)
                .global_mutex("DRIVE", "IOB")
                .attr("TFF", "#FF")
                .attr("TRIMUX", "T")
                .pin("T")
                .test_enum("TSEL", &["0", "1"]);
            bctx.mode(mode)
                .global_mutex("DRIVE", "IOB")
                .attr("OFF", "#FF")
                .attr("OMUX", "O")
                .attr("TRIMUX", "T")
                .attr("TSEL", "1")
                .pin("O")
                .pin("T")
                .test_enum("OUTMUX", &["0", "1"]);
            bctx.mode(mode)
                .attr("IMUX", "0")
                .pin("I")
                .test_enum("PULL", &["PULLDOWN", "PULLUP", "KEEPER"]);
            let iostds_cmos = if edev.chip.kind == ChipKind::Virtex {
                IOSTDS_CMOS_V
            } else {
                IOSTDS_CMOS_VE
            };
            for &iostd in iostds_cmos {
                bctx.mode(mode)
                    .attr("OUTMUX", "")
                    .pin("I")
                    .prop(VirtexIsDllIob(bels::IO[i], false))
                    .test_manual("ISTD", iostd)
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
                                .prop(VirtexIsDllIob(bels::IO[i], false))
                                .test_manual("OSTD", format!("{iostd}.{drive}.{slew}"))
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
                            .prop(VirtexIsDllIob(bels::IO[i], false))
                            .test_manual("OSTD", format!("{iostd}.{slew}"))
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
                    .prop(VirtexOtherIobInput(bels::IO[i], iostd.to_string()))
                    .attr("OUTMUX", "")
                    .pin("I")
                    .prop(VirtexIsDllIob(bels::IO[i], false))
                    .test_manual("ISTD", iostd)
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
                        .prop(VirtexIsDllIob(bels::IO[i], false))
                        .test_manual("OSTD", format!("{iostd}.{slew}"))
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
                        .prop(VirtexIsDllIob(bels::IO[i], false))
                        .prop(IsDiff(bels::IO[i]))
                        .test_manual("ISTD", iostd)
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
                            .prop(VirtexIsDllIob(bels::IO[i], false))
                            .prop(IsDiff(bels::IO[i]))
                            .test_manual("OSTD", format!("{iostd}.{slew}"))
                            .attr("IOATTRBOX", iostd)
                            .attr("SLEW", slew)
                            .attr("OMUX", "O_B")
                            .attr("OUTMUX", "1")
                            .attr("TRIMUX", "T")
                            .attr("TSEL", "1")
                            .commit();
                    }
                }
                if tile == "IO.B" || tile == "IO.T" {
                    let tile_clk = if backend.device.name.contains("2s") {
                        if tile == "IO.B" {
                            "CLKB_2DLL"
                        } else {
                            "CLKT_2DLL"
                        }
                    } else {
                        if tile == "IO.B" {
                            "CLKB_4DLL"
                        } else {
                            "CLKT_4DLL"
                        }
                    };
                    let row = if tile == "IO.B" {
                        edev.chip.row_s()
                    } else {
                        edev.chip.row_n()
                    };
                    let bel_clk = if i == 1 { "IOFB1" } else { "IOFB0" };
                    let clkbt = edev.egrid.get_node_by_kind(
                        DieId::from_idx(0),
                        (edev.chip.col_clk(), row),
                        |x| x == tile_clk,
                    );
                    for &iostd in IOSTDS_CMOS_VE {
                        bctx.mode("DLLIOB")
                            .global_mutex("GCLKIOB", "NO")
                            .attr("OUTMUX", "")
                            .pin("DLLFB")
                            .pin("I")
                            .prop(VirtexIsDllIob(bels::IO[i], true))
                            .extra_tile_attr_fixed(clkbt, bel_clk, "IBUF", "CMOS")
                            .test_manual("ISTD", iostd)
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
                            .prop(VirtexOtherIobInput(bels::IO[i], iostd.to_string()))
                            .attr("OUTMUX", "")
                            .pin("DLLFB")
                            .pin("I")
                            .prop(VirtexIsDllIob(bels::IO[i], true))
                            .extra_tile_attr_fixed(clkbt, bel_clk, "IBUF", "VREF")
                            .test_manual("ISTD", iostd)
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
                .extra_tiles_by_bel(bels::IO0, "IOB_ALL")
                .test_manual("IOB_ALL", attr, val)
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
    for side in ['L', 'R', 'B', 'T'] {
        let tile = &format!("IO.{side}");
        let tile_iob = &format!("IOB.{side}.{kind}");
        let mut pdrive_all = vec![];
        let mut ndrive_all = vec![];
        for attr in ["IDPD", "IDPC", "IDPB", "IDPA"] {
            pdrive_all.push(
                ctx.extract_enum_bool_wide(tile, "IOB_ALL", attr, "0", "1")
                    .bits,
            );
        }
        for attr in ["IDND", "IDNC", "IDNB", "IDNA", "IDNX"] {
            ndrive_all.push(
                ctx.extract_enum_bool_wide(tile, "IOB_ALL", attr, "0", "1")
                    .bits,
            );
        }
        for i in 0..4 {
            if i == 0 || (i == 3 && matches!(side, 'B' | 'T')) {
                continue;
            }
            let bel = &format!("IO{i}");

            // IOI

            let present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
            let diff = ctx
                .state
                .get_diff(tile, bel, "SHORTEN_JTAG_CHAIN", "0")
                .combine(&!&present);
            let item = xlat_bit(!diff);
            ctx.tiledb.insert(tile, bel, "SHORTEN_JTAG_CHAIN", item);
            for (pin, pin_b, pinmux) in [
                ("SR", "SR_B", "SRMUX"),
                ("ICE", "ICE_B", "ICEMUX"),
                ("OCE", "OCE_B", "OCEMUX"),
                ("TCE", "TCE_B", "TCEMUX"),
                ("T", "T_TB", "TRIMUX"),
                ("O", "O_B", "OMUX"),
            ] {
                let diff0 = ctx.state.get_diff(tile, bel, pinmux, "1");
                assert_eq!(diff0, ctx.state.get_diff(tile, bel, pinmux, pin));
                let diff1 = ctx.state.get_diff(tile, bel, pinmux, "0");
                assert_eq!(diff1, ctx.state.get_diff(tile, bel, pinmux, pin_b));
                let item = xlat_bool(diff0, diff1);
                ctx.insert_int_inv(&[tile], tile, bel, pin, item);
            }
            for iot in ['I', 'O', 'T'] {
                let item = ctx.extract_enum_bool(tile, bel, &format!("{iot}CKINV"), "1", "0");
                ctx.tiledb
                    .insert(tile, bel, format!("INV.{iot}FF.CLK"), item);
                let item = ctx.extract_bit(tile, bel, &format!("{iot}INITMUX"), "0");
                ctx.tiledb
                    .insert(tile, bel, format!("{iot}FF_SR_ENABLE"), item);
            }
            let item = ctx.extract_enum_bool(tile, bel, "IFFINITATTR", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "IFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFFATTRBOX", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFFATTRBOX", "LOW", "HIGH");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            ctx.state
                .get_diff(tile, bel, "FFATTRBOX", "ASYNC")
                .assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "FFATTRBOX", "SYNC");
            for iot in ['I', 'O', 'T'] {
                let init = ctx.tiledb.item(tile, bel, &format!("{iot}FF_INIT"));
                let init_bit = init.bits[0];
                let item = xlat_bitvec(vec![diff.split_bits_by(|bit| {
                    bit.tile == init_bit.tile
                        && bit.frame.abs_diff(init_bit.frame) == 1
                        && bit.bit == init_bit.bit
                })]);
                ctx.tiledb
                    .insert(tile, bel, format!("{iot}FF_SR_SYNC"), item);
            }
            diff.assert_empty();
            let item = ctx.extract_enum_bool(tile, bel, "IFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);

            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_IFF",
                TileItem::from_bit(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 2,
                            ('R', 2) => 27,
                            ('R', 3) => 32,
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
            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_OFF",
                TileItem::from_bit(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 8,
                            ('R', 2) => 21,
                            ('R', 3) => 38,
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
            ctx.tiledb.insert(
                tile,
                bel,
                "READBACK_TFF",
                TileItem::from_bit(
                    TileBit::new(
                        0,
                        match (side, i) {
                            ('R', 1) => 12,
                            ('R', 2) => 17,
                            ('R', 3) => 42,
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

            ctx.state.get_diff(tile, bel, "TSEL", "1").assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "TSEL", "0");
            let diff_ioi = diff.split_bits_by(|bit| bit.frame < 48 && bit.bit == 16);
            ctx.tiledb.insert(
                tile,
                bel,
                "TMUX",
                xlat_enum(vec![("T", Diff::default()), ("TFF", diff_ioi)]),
            );
            ctx.tiledb.insert(
                tile_iob,
                bel,
                "TMUX",
                xlat_enum(vec![("T", Diff::default()), ("TFF", diff)]),
            );
            let mut diff = ctx
                .state
                .get_diff(tile, bel, "OUTMUX", "0")
                .combine(&!ctx.state.get_diff(tile, bel, "OUTMUX", "1"));
            let diff_ioi = diff.split_bits_by(|bit| bit.frame < 48 && bit.bit == 16);
            ctx.tiledb.insert(
                tile,
                bel,
                "OMUX",
                xlat_enum(vec![("O", Diff::default()), ("OFF", diff_ioi)]),
            );
            ctx.tiledb.insert(
                tile_iob,
                bel,
                "OMUX",
                xlat_enum(vec![("O", Diff::default()), ("OFF", diff)]),
            );

            // IOB

            ctx.tiledb.insert(
                tile_iob,
                bel,
                "READBACK_I",
                TileItem::from_bit(
                    match (side, i) {
                        ('L' | 'R', 1) => TileBit::new(0, 50, 13),
                        ('L' | 'R', 2) => TileBit::new(0, 50, 12),
                        ('L' | 'R', 3) => TileBit::new(0, 50, 2),
                        ('B' | 'T', 1) => TileBit::new(0, 25, 17),
                        ('B' | 'T', 2) => TileBit::new(0, 21, 17),
                        _ => unreachable!(),
                    },
                    false,
                ),
            );
            let item = ctx.extract_enum_default(
                tile,
                bel,
                "PULL",
                &["PULLDOWN", "PULLUP", "KEEPER"],
                "NONE",
            );
            ctx.tiledb.insert(tile_iob, bel, "PULL", item);

            if has_any_vref(edev, ctx.device, ctx.db, tile, bels::IO[i]).is_some() {
                let diff = present.combine(&!&ctx.state.get_diff(tile, bel, "PRESENT", "NOT_VREF"));
                ctx.tiledb.insert(tile_iob, bel, "VREF", xlat_bit(diff));
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
                let diff_i = ctx.state.get_diff(tile, bel, "ISTD", iostd);
                let diff_o = if iostd == "LVTTL" {
                    ctx.state
                        .peek_diff(tile, bel, "OSTD", format!("{iostd}.12.SLOW"))
                } else {
                    ctx.state
                        .peek_diff(tile, bel, "OSTD", format!("{iostd}.SLOW"))
                }
                .clone();
                let (diff_i, _, diff_c) = Diff::split(diff_i, diff_o);
                diffs_istd.push((kind, diff_i));
                diffs_iostd_misc.insert(iostd, diff_c.clone());
                diffs_iostd_misc_vec.push((iostd, diff_c));
            }
            diffs_istd.push(("NONE", Diff::default()));
            ctx.tiledb
                .insert(tile_iob, bel, "IBUF", xlat_enum(diffs_istd));

            let mut pdrive = vec![None; 4];
            let mut ndrive = vec![None; 5];
            for drive in ["2", "4", "6", "8", "12", "16", "24"] {
                let diff = ctx
                    .state
                    .peek_diff(tile, bel, "OSTD", format!("LVTTL.{drive}.SLOW"));
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
                .state
                .peek_diff(tile, bel, "OSTD", "LVTTL.24.FAST")
                .combine(&!ctx.state.peek_diff(tile, bel, "OSTD", "LVTTL.24.SLOW"))
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
                            let mut diff = ctx.state.get_diff(
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
                            ctx.tiledb.insert_misc_data(
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
                            ctx.tiledb.insert_misc_data(
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
                            ctx.state
                                .get_diff(tile, bel, "OSTD", format!("{iostd}.{slew}"));
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
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{tag}:PDRIVE:{iostd}"), pdrive_val);
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
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{tag}:NDRIVE:{iostd}"), ndrive_val);
                        slews.push((format!("{iostd}.{slew}"), diff.split_bits(&slew_bits)));
                        ostd_misc.push((iostd, diff))
                    }
                }
            }

            ctx.tiledb.insert(
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
            ctx.tiledb.insert(
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
                ("IOSTD_MISC", xlat_enum(diffs_iostd_misc_vec)),
                ("OUTPUT_MISC", xlat_enum(ostd_misc)),
                ("SLEW", xlat_enum(slews)),
            ] {
                let TileItemKind::Enum { values } = item.kind else {
                    unreachable!()
                };
                for (name, val) in values {
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:{tag}:{attr}:{name}"), val);
                }
                let item = TileItem::from_bitvec(item.bits, false);
                ctx.tiledb.insert(tile_iob, bel, attr, item);
            }
        }
    }
    if edev.chip.kind != ChipKind::Virtex {
        for tile in if ctx.device.name.contains("2s") {
            ["CLKB_2DLL", "CLKT_2DLL"]
        } else {
            ["CLKB_4DLL", "CLKT_4DLL"]
        } {
            for bel in ["IOFB0", "IOFB1"] {
                ctx.collect_enum_default(tile, bel, "IBUF", &["CMOS", "VREF"], "NONE");
            }
        }
    }
}
