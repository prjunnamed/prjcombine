use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::BelSlotId,
    dir::DirV,
    grid::{CellCoord, DieId, TileCoord},
};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, extract_bitvec_val,
    extract_bitvec_val_part, xlat_bit, xlat_bit_wide, xlat_bitvec, xlat_enum, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{ExpandedBond, ExpandedDevice};
use prjcombine_spartan6::{bels, tslots};
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{RectBitId, TileBit, TileItem, TileItemKind},
};

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        iostd::{DiffKind, Iostd},
        props::{
            DynProp,
            bel::{BaseBelAttr, BaseBelMode, BaseBelPin, BelUnused},
            pip::PinFar,
            relation::{Delta, Related},
        },
    },
};

const IOSTDS_LR: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6", "8", "12"]),
    Iostd::cmos(
        "LVCMOS18_JEDEC",
        1800,
        &["2", "4", "6", "8", "12", "16", "24"],
    ),
    Iostd::cmos("LVCMOS15_JEDEC", 1500, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS12_JEDEC", 1200, &["2", "4", "6", "8", "12"]),
    Iostd::cmos("PCI33_3", 3300, &[]),
    Iostd::cmos("PCI66_3", 3300, &[]),
    Iostd::cmos("SDIO", 3300, &[]),
    Iostd::cmos("MOBILE_DDR", 1800, &[]),
    Iostd::cmos_od("I2C"),
    Iostd::cmos_od("SMBUS"),
    Iostd::vref("SSTL3_I", 3300, 1650),
    Iostd::vref("SSTL3_II", 3300, 1650),
    Iostd::vref("SSTL2_I", 2500, 1250),
    Iostd::vref("SSTL2_II", 2500, 1250),
    Iostd::vref("SSTL18_I", 1800, 900),
    Iostd::vref("SSTL18_II", 1800, 900),
    Iostd::vref("SSTL15_II", 1500, 750),
    Iostd::vref("HSTL_I", 1500, 750),
    Iostd::vref("HSTL_II", 1500, 750),
    Iostd::vref("HSTL_III", 1500, 900),
    Iostd::vref("HSTL_I_18", 1800, 900),
    Iostd::vref("HSTL_II_18", 1800, 900),
    Iostd::vref("HSTL_III_18", 1800, 1080),
    Iostd::pseudo_diff("DIFF_SSTL3_I", 3300),
    Iostd::pseudo_diff("DIFF_SSTL3_II", 3300),
    Iostd::pseudo_diff("DIFF_SSTL2_I", 2500),
    Iostd::pseudo_diff("DIFF_SSTL2_II", 2500),
    Iostd::pseudo_diff("DIFF_SSTL18_I", 1800),
    Iostd::pseudo_diff("DIFF_SSTL18_II", 1800),
    Iostd::pseudo_diff("DIFF_SSTL15_II", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_I", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_II", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_III", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_III_18", 1800),
    Iostd::pseudo_diff("DIFF_MOBILE_DDR", 1800),
    Iostd::pseudo_diff("BLVDS_25", 2500),
    Iostd::pseudo_diff("DISPLAY_PORT", 2500),
    Iostd::diff_input("LVPECL_25", 2500),
    Iostd::diff_input("LVPECL_33", 3300),
    Iostd::true_diff_input("LVDS_25", 2500),
    Iostd::true_diff_input("LVDS_33", 3300),
    Iostd::true_diff_input("MINI_LVDS_25", 2500),
    Iostd::true_diff_input("MINI_LVDS_33", 3300),
    Iostd::true_diff_input("RSDS_25", 2500),
    Iostd::true_diff_input("RSDS_33", 3300),
    Iostd::true_diff_input("PPDS_25", 2500),
    Iostd::true_diff_input("PPDS_33", 3300),
    Iostd::true_diff_input("TMDS_33", 3300),
];

const IOSTDS_BT: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8"]),
    Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6"]),
    Iostd::cmos("LVCMOS18_JEDEC", 1800, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS15_JEDEC", 1500, &["2", "4", "6", "8"]),
    Iostd::cmos("LVCMOS12_JEDEC", 1200, &["2", "4", "6"]),
    Iostd::cmos("PCI33_3", 3300, &[]),
    Iostd::cmos("PCI66_3", 3300, &[]),
    Iostd::cmos("SDIO", 3300, &[]),
    Iostd::cmos("MOBILE_DDR", 1800, &[]),
    Iostd::cmos_od("I2C"),
    Iostd::cmos_od("SMBUS"),
    Iostd::vref("SSTL3_I", 3300, 1650),
    Iostd::vref("SSTL3_II", 3300, 1650),
    Iostd::vref("SSTL2_I", 2500, 1250),
    Iostd::vref("SSTL2_II", 2500, 1250),
    Iostd::vref("SSTL18_I", 1800, 900),
    Iostd::vref_input("SSTL18_II", 1800, 900),
    Iostd::vref_input("SSTL15_II", 1500, 750),
    Iostd::vref("HSTL_I", 1500, 750),
    Iostd::vref_input("HSTL_II", 1500, 750),
    Iostd::vref("HSTL_III", 1500, 900),
    Iostd::vref("HSTL_I_18", 1800, 900),
    Iostd::vref_input("HSTL_II_18", 1800, 900),
    Iostd::vref("HSTL_III_18", 1800, 1080),
    Iostd::pseudo_diff("DIFF_SSTL3_I", 3300),
    Iostd::pseudo_diff("DIFF_SSTL3_II", 3300),
    Iostd::pseudo_diff("DIFF_SSTL2_I", 2500),
    Iostd::pseudo_diff("DIFF_SSTL2_II", 2500),
    Iostd::pseudo_diff("DIFF_SSTL18_I", 1800),
    Iostd::diff_input("DIFF_SSTL18_II", 1800),
    Iostd::diff_input("DIFF_SSTL15_II", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_I", 1500),
    Iostd::diff_input("DIFF_HSTL_II", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_III", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800),
    Iostd::diff_input("DIFF_HSTL_II_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_III_18", 1800),
    Iostd::pseudo_diff("DIFF_MOBILE_DDR", 1800),
    Iostd::pseudo_diff("BLVDS_25", 2500),
    Iostd::pseudo_diff("DISPLAY_PORT", 2500),
    Iostd::diff_input("LVPECL_25", 2500),
    Iostd::diff_input("LVPECL_33", 3300),
    Iostd::true_diff("LVDS_25", 2500),
    Iostd::true_diff("LVDS_33", 3300),
    Iostd::true_diff("MINI_LVDS_25", 2500),
    Iostd::true_diff("MINI_LVDS_33", 3300),
    Iostd::true_diff("RSDS_25", 2500),
    Iostd::true_diff("RSDS_33", 3300),
    Iostd::true_diff("PPDS_25", 2500),
    Iostd::true_diff("PPDS_33", 3300),
    Iostd::true_diff("TMDS_33", 3300),
    Iostd::true_diff("TML_33", 3300),
];

#[derive(Copy, Clone, Debug)]
struct AllMcbIoi(&'static str, &'static str, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for AllMcbIoi {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Spartan6(edev) = backend.edev else {
            unreachable!()
        };

        for row in backend.edev.rows(tcrd.die) {
            if let Some(split) = edev.chip.row_mcb_split {
                if tcrd.row < split && row >= split {
                    continue;
                }
                if tcrd.row >= split && row < split {
                    continue;
                }
            }
            if let Some(ntcrd) = backend
                .edev
                .find_tile_by_class(tcrd.with_row(row), |kind| kind == "IOI.LR")
            {
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: "IOI.LR".to_string(),
                        bel: self.0.into(),
                        attr: self.1.into(),
                        val: self.2.into(),
                    },
                    rects: edev.tile_bits(ntcrd),
                })
            }
        }

        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
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
        let ExpandedDevice::Spartan6(edev) = backend.edev else {
            unreachable!()
        };
        let ExpandedBond::Spartan6(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
        if !ebond.bond.vref.contains(&crd) {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
struct IsBonded(BelSlotId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for IsBonded {
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
        let ExpandedDevice::Spartan6(edev) = backend.edev else {
            unreachable!()
        };
        let ExpandedBond::Spartan6(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
        if !ebond.ios.contains_key(&crd) {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
struct IsBank(BelSlotId, u32);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for IsBank {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Spartan6(edev) = backend.edev else {
            unreachable!()
        };
        let crd = edev.chip.get_io_crd(tcrd.bel(self.0));
        if edev.chip.get_io_bank(crd) != self.1 {
            return None;
        }
        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
struct DeviceSide(DirV);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DeviceSide {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Spartan6(edev) = backend.edev else {
            unreachable!()
        };
        let dir_match = match self.0 {
            DirV::S => tcrd.row < edev.chip.row_clk(),
            DirV::N => tcrd.row >= edev.chip.row_clk(),
        };
        if dir_match {
            Some((fuzzer, false))
        } else {
            None
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Spartan6(edev) = backend.edev else {
        unreachable!()
    };
    let package = backend
        .device
        .bonds
        .values()
        .max_by_key(|bond| {
            let bdata = &backend.db.bonds[bond.bond];
            let prjcombine_re_xilinx_geom::Bond::Spartan6(bdata) = bdata else {
                unreachable!();
            };
            bdata.pins.len()
        })
        .unwrap();
    for tile in ["IOI.LR", "IOI.BT"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for i in 0..2 {
            let mut bctx = ctx.bel(bels::ILOGIC[i]);
            let bel_other = bels::ILOGIC[i ^ 1];
            let bel_ologic = bels::OLOGIC[i];
            let bel_ioiclk = bels::IOICLK[i];
            for mode in ["ILOGIC2", "ISERDES2"] {
                bctx.build()
                    .tile_mutex("CLK", "TEST_LOGIC")
                    .global("GLUTMASK", "NO")
                    .bel_unused(bel_other)
                    .has_related(Delta::new(0, 0, "IOB"))
                    .test_manual("MODE", mode)
                    .mode(mode)
                    .commit();
            }
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "NOPE")
                .test_enum("IFFTYPE", &["#LATCH", "#FF", "DDR"]);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .attr("FABRICOUTUSED", "0")
                .pin("TFB")
                .pin("FABRICOUT")
                .test_enum("D2OBYP_SEL", &["GND", "T"]);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .bel_unused(bel_other)
                .tile_mutex("CLK", "NOPE")
                .attr("FABRICOUTUSED", "0")
                .attr("IFFTYPE", "#FF")
                .attr("D2OBYP_SEL", "GND")
                .pin("OFB")
                .pin("D")
                .pin("DDLY")
                .test_enum("IMUX", &["0", "1"]);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .bel_unused(bel_other)
                .tile_mutex("CLK", "NOPE")
                .attr("FABRICOUTUSED", "0")
                .attr("IFFTYPE", "#FF")
                .attr("D2OBYP_SEL", "GND")
                .pin("OFB")
                .pin("D")
                .pin("DDLY")
                .test_enum("IFFMUX", &["0", "1"]);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("SRINIT_Q", &["0", "1"]);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("SRTYPE_Q", &["ASYNC", "SYNC"]);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .pin("SR")
                .attr("IFFTYPE", "#FF")
                .test_enum("SRUSED", &["0"]);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .pin("REV")
                .attr("IFFTYPE", "#FF")
                .test_enum("REVUSED", &["0"]);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .pin("CE0")
                .attr("IFFTYPE", "#FF")
                .test_manual("IFF_CE_ENABLE", "0")
                .pin_pips("CE0")
                .commit();

            bctx.mode("ISERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("DATA_WIDTH", &["1", "2", "3", "4", "5", "6", "7", "8"]);
            bctx.mode("ISERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("BITSLIP_ENABLE", &["FALSE", "TRUE"]);
            bctx.mode("ISERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum(
                    "INTERFACE_TYPE",
                    &["NETWORKING", "NETWORKING_PIPELINED", "RETIMED"],
                );
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.SR", "INT")
                .test_manual("MUX.SR", "INT")
                .pip("SR", "SR_INT")
                .commit();
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.SR", "OLOGIC_SR")
                .test_manual("MUX.SR", "OLOGIC_SR")
                .pip("SR", (PinFar, bel_ologic, "SR"))
                .commit();

            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_LOGIC")
                .test_manual("MUX.CLK", format!("ICLK{i}"))
                .pip("CLK0", (bel_ioiclk, "CLK0_ILOGIC"))
                .commit();
            bctx.mode("ISERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_LOGIC")
                .bel_mode(bel_other, "ISERDES2")
                .pin("D")
                .bel_pin(bel_other, "D")
                .test_manual("ENABLE.IOCE", "1")
                .pip("IOCE", (bel_ioiclk, "IOCE0"))
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_LOGIC")
                .unused()
                .bel_unused(bel_other)
                .test_manual("ENABLE", "1")
                .pip("IOCE", (bel_ioiclk, "IOCE0"))
                .commit();
            if i == 0 {
                bctx.build()
                    .has_related(Delta::new(0, 0, "IOB"))
                    .test_manual("MUX.D", "OTHER_IOB_I")
                    .pip("D_MUX", (bel_other, "IOB_I"))
                    .commit();
            }
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(bels::OLOGIC[i]);
            let bel_iodelay = bels::IODELAY[i];
            let bel_ioiclk = bels::IOICLK[i];
            let bel_ioi = bels::IOI;
            for mode in ["OLOGIC2", "OSERDES2"] {
                bctx.build()
                    .has_related(Delta::new(0, 0, "IOB"))
                    .global("ENABLEMISR", "N")
                    .tile_mutex("CLK", "TEST_LOGIC")
                    .global("GLUTMASK", "NO")
                    .test_manual("MODE", mode)
                    .mode(mode)
                    .commit();
            }
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .global("ENABLEMISR", "Y")
                .global("MISRRESET", "Y")
                .tile_mutex("CLK", "TEST_LOGIC")
                .global("GLUTMASK", "NO")
                .test_manual("MODE", "OLOGIC2.MISR_RESET")
                .mode("OLOGIC2")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("SRINIT_OQ", &["0", "1"]);
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("SRINIT_TQ", &["0", "1"]);
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .pin("SR")
                .test_enum("SRTYPE_OQ", &["SYNC", "ASYNC"]);
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .pin("SR")
                .test_enum("SRTYPE_TQ", &["SYNC", "ASYNC"]);
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("DATA_WIDTH", &["1", "2", "3", "4", "5", "6", "7", "8"]);
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("BYPASS_GCLK_FF", &["FALSE", "TRUE"]);
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("OUTPUT_MODE", &["DIFFERENTIAL", "SINGLE_ENDED"]);
            for attr in ["OSRUSED", "TSRUSED", "OREVUSED", "TREVUSED"] {
                bctx.mode("OLOGIC2")
                    .has_related(Delta::new(0, 0, "IOB"))
                    .attr("OUTFFTYPE", "#FF")
                    .attr("TFFTYPE", "#FF")
                    .pin("SR")
                    .pin("REV")
                    .test_enum(attr, &["0"]);
            }
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.SR", "INT")
                .test_manual("MUX.SR", "INT")
                .pin_pips("SR")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.REV", "INT")
                .test_manual("MUX.REV", "INT")
                .pin_pips("REV")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.OCE", "INT")
                .attr("OUTFFTYPE", "#FF")
                .test_manual("MUX.OCE", "INT")
                .pin_pips("OCE")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.OCE", "PCI_CE")
                .attr("OUTFFTYPE", "#FF")
                .test_manual("MUX.OCE", "PCI_CE")
                .pip("OCE", (bel_ioi, "PCI_CE"))
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.TCE", "INT")
                .attr("TFFTYPE", "#FF")
                .test_manual("MUX.TCE", "INT")
                .pin_pips("TCE")
                .commit();
            bctx.mode("OSERDES2")
                .global_mutex("DRPSDO", "NOPE")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.TRAIN", "MCB")
                .test_manual("MUX.TRAIN", "MCB")
                .pip("TRAIN", (bel_ioi, "MCB_DRPTRAIN"))
                .commit();
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .mutex("MUX.TRAIN", "INT")
                .test_manual("MUX.TRAIN", "INT")
                .pin_pips("TRAIN")
                .commit();
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_multi_attr_dec("TRAIN_PATTERN", 4);
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .global_mutex("DRPSDO", "USE")
                .pip((bel_iodelay, "CE"), (bel_ioi, "MCB_DRPSDO"))
                .test_manual("MUX.D", "MCB")
                .pip("D1", "MCB_D1")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "NOPE")
                .attr("TFFTYPE", "")
                .test_enum("OUTFFTYPE", &["#LATCH", "#FF", "DDR"]);
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "NOPE")
                .attr("OUTFFTYPE", "")
                .test_enum("TFFTYPE", &["#LATCH", "#FF", "DDR"]);
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "NOPE")
                .attr("OUTFFTYPE", "#FF")
                .attr("D1USED", "0")
                .attr("O1USED", "0")
                .pin("D1")
                .pin("OQ")
                .test_enum("OMUX", &["D1", "OUTFF"]);
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "NOPE")
                .attr("OUTFFTYPE", "")
                .attr("TFFTYPE", "")
                .attr("T1USED", "0")
                .pin("T1")
                .pin("TQ")
                .test_enum("OT1USED", &["0"]);
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "NOPE")
                .attr("OUTFFTYPE", "DDR")
                .attr("TDDR_ALIGNMENT", "")
                .test_enum("DDR_ALIGNMENT", &["NONE", "C0"]);
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "NOPE")
                .attr("TFFTYPE", "DDR")
                .attr("DDR_ALIGNMENT", "")
                .test_enum("TDDR_ALIGNMENT", &["NONE", "C0"]);

            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .global("ENABLEMISR", "Y")
                .global("MISR_BLV_EN", "Y")
                .global("MISR_BLH_EN", "Y")
                .global("MISR_BRV_EN", "Y")
                .global("MISR_BRH_EN", "Y")
                .global("MISR_TLV_EN", "Y")
                .global("MISR_TLH_EN", "Y")
                .global("MISR_TRV_EN", "Y")
                .global("MISR_TRH_EN", "Y")
                .global("MISR_BM_EN", "Y")
                .global("MISR_TM_EN", "Y")
                .test_enum("MISRATTRBOX", &["MISR_ENABLE_DATA"]);

            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, "IOB"))
                .global("ENABLEMISR", "Y")
                .test_enum("MISR_ENABLE_CLK", &["CLK0", "CLK1"]);

            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_LOGIC")
                .test_manual("MUX.CLK", format!("OCLK{i}"))
                .pip("CLK0", (bel_ioiclk, "CLK0_OLOGIC"))
                .commit();
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_LOGIC")
                .test_manual("ENABLE.IOCE", "1")
                .pip("IOCE", (bel_ioiclk, "IOCE1"))
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_LOGIC")
                .unused()
                .test_manual("ENABLE", "1")
                .pip("IOCE", (bel_ioiclk, "IOCE1"))
                .commit();
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(bels::IODELAY[i]);
            let bel_other = bels::IODELAY[i ^ 1];
            let bel_ilogic = bels::ILOGIC[i];
            let bel_ologic = bels::OLOGIC[i];
            let bel_ioiclk = bels::IOICLK[i];
            for mode in ["IODELAY2", "IODRP2", "IODRP2_MCB"] {
                bctx.build()
                    .has_related(Delta::new(0, 0, "IOB"))
                    .global_mutex("DRPSDO", "NOPE")
                    .global("GLUTMASK", "NO")
                    .global("IOI_TESTPCOUNTER", "NO")
                    .global("IOI_TESTNCOUNTER", "NO")
                    .global("IOIENFFSCAN_DRP", "NO")
                    .bel_unused(bel_other)
                    .test_manual("MODE", mode)
                    .mode(mode)
                    .commit();
            }
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .global("GLUTMASK", "NO")
                .global("IOI_TESTPCOUNTER", "YES")
                .global("IOI_TESTNCOUNTER", "NO")
                .global("IOIENFFSCAN_DRP", "NO")
                .bel_unused(bel_other)
                .test_manual("MODE", "IODELAY2.TEST_PCOUNTER")
                .mode("IODELAY2")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .global("GLUTMASK", "NO")
                .global("IOI_TESTPCOUNTER", "NO")
                .global("IOI_TESTNCOUNTER", "YES")
                .global("IOIENFFSCAN_DRP", "NO")
                .bel_unused(bel_other)
                .test_manual("MODE", "IODELAY2.TEST_NCOUNTER")
                .mode("IODELAY2")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .global("GLUTMASK", "NO")
                .global("IOI_TESTPCOUNTER", "NO")
                .global("IOI_TESTNCOUNTER", "NO")
                .global("IOIENFFSCAN_DRP", "YES")
                .bel_unused(bel_other)
                .test_manual("MODE", "IODRP2.IOIENFFSCAN_DRP")
                .mode("IODRP2")
                .commit();

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_multi_attr_dec("ODELAY_VALUE", 8);
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .attr("IDELAY_TYPE", "FIXED")
                .attr("IDELAY_MODE", "PCI")
                .test_multi_attr_dec("IDELAY_VALUE", 8);
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .attr("IDELAY_TYPE", "FIXED")
                .attr("IDELAY_MODE", "PCI")
                .test_multi_attr_dec("IDELAY2_VALUE", 8);
            bctx.mode("IODRP2_MCB")
                .has_related(Delta::new(0, 0, "IOB"))
                .global_mutex("DRPSDO", "NOPE")
                .test_multi_attr_dec("MCB_ADDRESS", 4);
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .pin("CIN")
                .test_manual("ENABLE.CIN", "1")
                .pin_pips("CIN")
                .commit();

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("TEST_GLITCH_FILTER", &["FALSE", "TRUE"]);

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("COUNTER_WRAPAROUND", &["WRAPAROUND", "STAY_AT_LIMIT"]);

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("IODELAY_CHANGE", &["CHANGE_ON_CLOCK", "CHANGE_ON_DATA"]);

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .bel_unused(bel_other)
                .test_enum(
                    "IDELAY_TYPE",
                    &[
                        "FIXED",
                        "DEFAULT",
                        "VARIABLE_FROM_ZERO",
                        "VARIABLE_FROM_HALF_MAX",
                        "DIFF_PHASE_DETECTOR",
                    ],
                );
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .bel_mode(bel_other, "IODELAY2")
                .bel_attr(bel_other, "IDELAY_TYPE", "DIFF_PHASE_DETECTOR")
                .test_enum_suffix(
                    "IDELAY_TYPE",
                    "DPD",
                    &[
                        "FIXED",
                        "DEFAULT",
                        "VARIABLE_FROM_ZERO",
                        "VARIABLE_FROM_HALF_MAX",
                        "DIFF_PHASE_DETECTOR",
                    ],
                );

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_manual("ENABLE.ODATAIN", "1")
                .pip("ODATAIN", (bel_ologic, "OQ"))
                .commit();

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "IODELAY")
                .mutex("MUX.IOCLK", "ILOGIC_CLK")
                .pip((bel_ilogic, "CLK0"), (bel_ioiclk, "CLK0_ILOGIC"))
                .test_manual("MUX.IOCLK", "ILOGIC_CLK")
                .pip("IOCLK0", (bel_ioiclk, "CLK0_ILOGIC"))
                .commit();
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "IODELAY")
                .mutex("MUX.IOCLK", "OLOGIC_CLK")
                .pip((bel_ologic, "CLK0"), (bel_ioiclk, "CLK0_OLOGIC"))
                .test_manual("MUX.IOCLK", "OLOGIC_CLK")
                .pip("IOCLK0", (bel_ioiclk, "CLK0_OLOGIC"))
                .commit();

            bctx.mode("IODRP2")
                .has_related(Delta::new(0, 0, "IOB"))
                .attr("IDELAY_MODE", "NORMAL")
                .test_enum("DELAY_SRC", &["IDATAIN", "ODATAIN", "IO"]);

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("IDELAY_MODE", &["PCI", "NORMAL"]);

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, "IOB"))
                .test_enum("DELAYCHAIN_OSC", &["FALSE", "TRUE"]);
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(bels::IOICLK[i]);
            let bel_ilogic = bels::ILOGIC[i];
            let bel_ologic = bels::OLOGIC[i];
            let bel_ioi = bels::IOI;
            for (j, pin) in [(0, "CKINT0"), (0, "CKINT1"), (1, "CKINT0"), (1, "CKINT1")] {
                bctx.build()
                    .has_related(Delta::new(0, 0, "IOB"))
                    .mutex(format!("MUX.CLK{j}"), pin)
                    .tile_mutex("CLK", "TEST_INTER")
                    .test_manual(format!("MUX.CLK{j}"), pin)
                    .pip(format!("CLK{j}INTER"), pin)
                    .commit();
            }
            for (j, pin) in [
                (0, "IOCLK0"),
                (0, "IOCLK2"),
                (0, "PLLCLK0"),
                (1, "IOCLK1"),
                (1, "IOCLK3"),
                (1, "PLLCLK1"),
                (2, "PLLCLK0"),
                (2, "PLLCLK1"),
            ] {
                bctx.build()
                    .has_related(Delta::new(0, 0, "IOB"))
                    .mutex(format!("MUX.CLK{j}"), pin)
                    .tile_mutex("CLK", "TEST_INTER")
                    .test_manual(format!("MUX.CLK{j}"), pin)
                    .pip(format!("CLK{j}INTER"), (bel_ioi, pin))
                    .commit();
            }
            for j in 0..3 {
                bctx.build()
                    .has_related(Delta::new(0, 0, "IOB"))
                    .tile_mutex("CLK", "TEST_INV")
                    .pip("CLK0_ILOGIC", format!("CLK{j}INTER"))
                    .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                    .bel_mode(bel_ilogic, "ISERDES2")
                    .bel_attr(bel_ilogic, "DATA_RATE", "SDR")
                    .bel_pin(bel_ilogic, "CLK0")
                    .test_manual(format!("INV.CLK{j}"), "1")
                    .bel_attr(bel_ilogic, "CLK0INV", "CLK0_B")
                    .commit();
            }
            for j in 0..3 {
                bctx.build()
                    .has_related(Delta::new(0, 0, "IOB"))
                    .tile_mutex("CLK", "TEST_CLK")
                    .mutex("MUX.ICLK", format!("CLK{j}"))
                    .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                    .bel_mode(bel_ilogic, "ISERDES2")
                    .bel_attr(bel_ilogic, "DATA_RATE", "SDR")
                    .bel_pin(bel_ilogic, "CLK0")
                    .test_manual("MUX.ICLK", format!("CLK{j}"))
                    .pip("CLK0_ILOGIC", format!("CLK{j}INTER"))
                    .commit();
                bctx.build()
                    .has_related(Delta::new(0, 0, "IOB"))
                    .tile_mutex("CLK", "TEST_CLK")
                    .mutex("MUX.OCLK", format!("CLK{j}"))
                    .pip((bel_ologic, "CLK0"), "CLK0_OLOGIC")
                    .bel_mode(bel_ologic, "OSERDES2")
                    .bel_attr(bel_ologic, "DATA_RATE_OQ", "SDR")
                    .bel_pin(bel_ologic, "CLK0")
                    .test_manual("MUX.OCLK", format!("CLK{j}"))
                    .pip("CLK0_OLOGIC", format!("CLK{j}INTER"))
                    .commit();
            }
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_ICLK_DDR")
                .mutex("MUX.ICLK", "DDR")
                .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                .bel_mode(bel_ilogic, "ISERDES2")
                .bel_attr(bel_ilogic, "DATA_RATE", "DDR")
                .bel_pin(bel_ilogic, "CLK0")
                .test_manual("MUX.ICLK", "DDR")
                .pip("CLK0_ILOGIC", "CLK0INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_OCLK_DDR")
                .mutex("MUX.OCLK", "DDR")
                .pip((bel_ologic, "CLK0"), "CLK0_OLOGIC")
                .bel_mode(bel_ologic, "OSERDES2")
                .bel_attr(bel_ologic, "DATA_RATE_OQ", "DDR")
                .bel_pin(bel_ologic, "CLK0")
                .test_manual("MUX.OCLK", "DDR")
                .pip("CLK0_OLOGIC", "CLK0INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_ICLK_DDR")
                .mutex("MUX.ICLK", "DDR")
                .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                .bel_mode(bel_ilogic, "ILOGIC2")
                .bel_attr(bel_ilogic, "IFFTYPE", "DDR")
                .bel_attr(bel_ilogic, "DDR_ALIGNMENT", "")
                .bel_pin(bel_ilogic, "CLK0")
                .test_manual("MUX.ICLK", "DDR.ILOGIC")
                .pip("CLK0_ILOGIC", "CLK0INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_ICLK_DDR_C0")
                .mutex("MUX.ICLK", "DDR")
                .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                .bel_mode(bel_ilogic, "ILOGIC2")
                .bel_attr(bel_ilogic, "IFFTYPE", "DDR")
                .bel_attr(bel_ilogic, "DDR_ALIGNMENT", "C0")
                .bel_pin(bel_ilogic, "CLK0")
                .test_manual("MUX.ICLK", "DDR.ILOGIC.C0")
                .pip("CLK0_ILOGIC", "CLK0INTER")
                .pip("CLK1", "CLK1INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_ICLK_DDR_C1")
                .mutex("MUX.ICLK", "DDR")
                .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                .bel_mode(bel_ilogic, "ILOGIC2")
                .bel_attr(bel_ilogic, "IFFTYPE", "DDR")
                .bel_attr(bel_ilogic, "DDR_ALIGNMENT", "C0")
                .bel_pin(bel_ilogic, "CLK0")
                .test_manual("MUX.ICLK", "DDR.ILOGIC.C1")
                .pip("CLK0_ILOGIC", "CLK1INTER")
                .pip("CLK1", "CLK0INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .tile_mutex("CLK", "TEST_OCLK_DDR")
                .mutex("MUX.OCLK", "DDR")
                .pip((bel_ologic, "CLK0"), "CLK0_OLOGIC")
                .bel_mode(bel_ologic, "OLOGIC2")
                .bel_attr(bel_ologic, "OUTFFTYPE", "DDR")
                .bel_attr(bel_ologic, "TFFTYPE", "DDR")
                .bel_attr(bel_ologic, "ODDR_ALIGNMENT", "")
                .bel_attr(bel_ologic, "TDDR_ALIGNMENT", "")
                .bel_pin(bel_ologic, "CLK0")
                .test_manual("MUX.OCLK", "DDR.OLOGIC")
                .pip("CLK0_OLOGIC", "CLK0INTER")
                .commit();
            for j in 0..2 {
                for pin in ["IOCE0", "IOCE1", "IOCE2", "IOCE3", "PLLCE0", "PLLCE1"] {
                    bctx.build()
                        .has_related(Delta::new(0, 0, "IOB"))
                        .tile_mutex("CLK", ["TEST_ICE", "TEST_OCE"][j])
                        .mutex(["MUX.ICE", "MUX.OCE"][j], pin)
                        .test_manual(["MUX.ICE", "MUX.OCE"][j], pin)
                        .pip(format!("IOCE{j}"), (bel_ioi, pin))
                        .commit();
                }
            }
        }
        let mut bctx = ctx.bel(bels::IOI);
        if tile == "IOI.BT" {
            let bel_iodelay = bels::IODELAY[0];
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .global_mutex("MCB", "NONE")
                .global_mutex("DRPSDO", "TEST")
                .global("MEM_PLL_POL_SEL", "INVERTED")
                .global("MEM_PLL_DIV_EN", "DISABLED")
                .test_manual("DRPSDO", "1")
                .pip((bel_iodelay, "CE"), "MCB_DRPSDO")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .global_mutex("MCB", "NONE")
                .global_mutex("DRPSDO", "TEST")
                .global("MEM_PLL_POL_SEL", "INVERTED")
                .global("MEM_PLL_DIV_EN", "ENABLED")
                .test_manual("DRPSDO", "1.DIV_EN")
                .pip((bel_iodelay, "CE"), "MCB_DRPSDO")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, "IOB"))
                .global_mutex("MCB", "NONE")
                .global_mutex("DRPSDO", "TEST")
                .global("MEM_PLL_POL_SEL", "NOTINVERTED")
                .global("MEM_PLL_DIV_EN", "DISABLED")
                .test_manual("DRPSDO", "1.NOTINV")
                .pip((bel_iodelay, "CE"), "MCB_DRPSDO")
                .commit();
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "MCB") {
        let mut bctx = ctx.bel(bels::MCB);
        bctx.build()
            .null_bits()
            .prop(AllMcbIoi("IOI", "DRPSDO", "1"))
            .global_mutex("MCB", "NONE")
            .global_mutex("DRPSDO", "TEST")
            .global("MEM_PLL_POL_SEL", "INVERTED")
            .global("MEM_PLL_DIV_EN", "DISABLED")
            .test_manual("DRPSDO", "1")
            .pip((PinFar, "IOIDRPSDO"), "IOIDRPSDO")
            .commit();
        bctx.build()
            .null_bits()
            .prop(AllMcbIoi("IOI", "DRPSDO", "1.DIV_EN"))
            .global_mutex("MCB", "NONE")
            .global_mutex("DRPSDO", "TEST")
            .global("MEM_PLL_POL_SEL", "INVERTED")
            .global("MEM_PLL_DIV_EN", "ENABLED")
            .test_manual("DRPSDO", "1")
            .pip((PinFar, "IOIDRPSDO"), "IOIDRPSDO")
            .commit();
        bctx.build()
            .null_bits()
            .prop(AllMcbIoi("IOI", "DRPSDO", "1.NOTINV"))
            .global_mutex("MCB", "NONE")
            .global_mutex("DRPSDO", "TEST")
            .global("MEM_PLL_POL_SEL", "NOTINVERTED")
            .global("MEM_PLL_DIV_EN", "DISABLED")
            .test_manual("DRPSDO", "1")
            .pip((PinFar, "IOIDRPSDO"), "IOIDRPSDO")
            .commit();
    }
    let mut ctx = FuzzCtx::new(session, backend, "IOB");
    for i in 0..2 {
        let bel = bels::IOB[i];
        let mut bctx = ctx.bel(bel);
        let bel_other = bels::IOB[i ^ 1];
        bctx.build()
            .global_mutex("IOB", "SHARED")
            .global_mutex("VREF", "NO")
            .bel_mode(bel_other, "IOB")
            .test_manual("PRESENT", "1")
            .mode("IOB")
            .commit();
        if i == 0 {
            bctx.build()
                .global_mutex("IOB", "SHARED")
                .global_mutex("VREF", "YES")
                .global_mutex("VCCO.LR", "1800")
                .global_mutex("VREF.LR", "1800")
                .global_mutex("VCCO.BT", "1800")
                .global_mutex("VREF.BT", "1800")
                .raw(Key::Package, package.name.clone())
                .prop(IsVref(bel))
                .bel_mode(bel_other, "IOB")
                .bel_pin(bel_other, "I")
                .bel_attr(bel_other, "TUSED", "")
                .bel_attr(bel_other, "IMUX", "I")
                .bel_attr(bel_other, "BYPASS_MUX", "I")
                .bel_attr(bel_other, "ISTANDARD", "HSTL_I_18")
                .test_manual("PRESENT", "NOTVREF")
                .mode("IOB")
                .commit();
        }

        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .attr("TUSED", "0")
            .pin("T")
            .test_enum("PULLTYPE", &["KEEPER", "PULLDOWN", "PULLUP"]);
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .attr("TUSED", "0")
            .pin("T")
            .test_enum(
                "SUSPEND",
                &[
                    "3STATE",
                    "3STATE_KEEPER",
                    "3STATE_PULLDOWN",
                    "3STATE_PULLUP",
                    "3STATE_OCT_ON",
                    "DRIVE_LAST_VALUE",
                ],
            );
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .attr("TUSED", "0")
            .pin("T")
            .test_enum("PRE_EMPHASIS", &["ON"]);
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .attr("TUSED", "0")
            .attr("BYPASS_MUX", "I")
            .pin("T")
            .pin("I")
            .test_enum("IMUX", &["I", "I_B"]);
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .mutex("MODE", "BYPASS")
            .attr("TUSED", "0")
            .attr("OUSED", "0")
            .attr("IMUX", "I")
            .pin("T")
            .pin("O")
            .pin("I")
            .test_enum("BYPASS_MUX", &["I", "O", "T"]);
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .mutex("MODE", "OUSED")
            .test_manual("OUTPUT_ENABLE", "0")
            .attr("TUSED", "0")
            .attr("OUSED", "0")
            .attr("DRIVE_0MA", "DRIVE_0MA")
            .pin("T")
            .pin("O")
            .commit();

        let cnr_ll = CellCoord::new(
            DieId::from_idx(0),
            edev.chip.col_w(),
            edev.chip.row_bio_outer(),
        )
        .tile(tslots::BEL);
        let cnr_ul = CellCoord::new(
            DieId::from_idx(0),
            edev.chip.col_w(),
            edev.chip.row_tio_outer(),
        )
        .tile(tslots::BEL);
        let cnr_lr = CellCoord::new(
            DieId::from_idx(0),
            edev.chip.col_e(),
            edev.chip.row_bio_outer(),
        )
        .tile(tslots::BEL);
        let cnr_ur = CellCoord::new(
            DieId::from_idx(0),
            edev.chip.col_e(),
            edev.chip.row_tio_inner(),
        )
        .tile(tslots::BEL);

        bctx.build()
            .global("GLUTMASK", "YES")
            .global_mutex_here("IOB")
            .extra_tile_attr_fixed(cnr_lr, "MISC", "GLUTMASK_IOB", "1")
            .test_manual("PRESENT", "1")
            .mode("IOB")
            .commit();

        bctx.build()
            .global_mutex_here("IOB")
            .raw(Key::VccAux, "3.3")
            .raw(Key::Package, package.name.clone())
            .prop(IsBonded(bel))
            .mode("IOB")
            .pin("I")
            .attr("TUSED", "")
            .attr("IMUX", "I")
            .attr("BYPASS_MUX", "I")
            .extra_tile_attr_fixed(cnr_ul, "MISC", "VREF_LV", "1")
            .test_manual("VREF_LV", "1")
            .attr_diff("ISTANDARD", "SSTL3_I", "SSTL18_I")
            .commit();

        let banks = [cnr_ul, cnr_lr, cnr_ll, cnr_ll, cnr_ul, cnr_ur];
        for bank in 0..6 {
            if bank >= 4 && edev.chip.row_mcb_split.is_none() {
                continue;
            }
            bctx.build()
                .global_mutex_here("IOB")
                .raw(Key::VccAux, "3.3")
                .raw(Key::Package, package.name.clone())
                .prop(IsBonded(bel))
                .prop(IsBank(bel, bank as u32))
                .mode("IOB")
                .pin("O")
                .pin("T")
                .attr("TUSED", "0")
                .attr("OUSED", "0")
                .attr("OSTANDARD", "SSTL2_I")
                .extra_tile_attr_fixed(banks[bank], format!("OCT_CAL{bank}"), "INTERNAL_VREF", "1")
                .test_manual("ISTD", "SSTL2_I:3.3:LR")
                .pin("I")
                .attr("IMUX", "I")
                .attr("BYPASS_MUX", "I")
                .attr("ISTANDARD", "SSTL2_I")
                .raw_diff(
                    Key::InternalVref(bank as u32),
                    Value::None,
                    Value::U32(1250),
                )
                .commit();
        }

        for (kind, ioi, iostds) in [("LR", "IOI.LR", IOSTDS_LR), ("BT", "IOI.BT", IOSTDS_BT)] {
            let bel_ologic = bels::OLOGIC[i];
            for vccaux in ["2.5", "3.3"] {
                for std in iostds {
                    if matches!(std.name, "PCI33_3" | "PCI66_3" | "TMDS_33" | "LVPECL_33")
                        && vccaux == "2.5"
                    {
                        continue;
                    }
                    if std.name == "TML_33" {
                        continue;
                    }
                    let vcco = &match std.vcco {
                        Some(vcco) => vcco.to_string(),
                        None => "3300".to_string(),
                    };
                    if std.diff != DiffKind::None {
                        let has_diff_term = std.diff == DiffKind::True && std.name != "TMDS_33";
                        bctx.build()
                            .global_mutex("IOB", "SHARED")
                            .global_mutex(format!("VCCO.{kind}"), vcco)
                            .raw(Key::VccAux, vccaux)
                            .raw(Key::Package, package.name.clone())
                            .prop(IsBonded(bel))
                            .mode("IOB")
                            .pin("I")
                            .pin("DIFFI_IN")
                            .attr("DIFFI_INUSED", "0")
                            .attr("TUSED", "")
                            .attr("DIFF_TERM", if has_diff_term { "FALSE" } else { "" })
                            .attr("IMUX", "I")
                            .attr("BYPASS_MUX", "I")
                            .prop(Related::new(
                                Delta::new(0, 0, ioi),
                                BelUnused::new(bel_ologic),
                            ))
                            .test_manual("ISTD", format!("{sn}:{vccaux}:{kind}", sn = std.name))
                            .attr("ISTANDARD", std.name)
                            .commit();
                        if has_diff_term {
                            bctx.build()
                                .global_mutex("IOB", "SHARED")
                                .global_mutex(format!("VCCO.{kind}"), vcco)
                                .raw(Key::VccAux, vccaux)
                                .raw(Key::Package, package.name.clone())
                                .prop(IsBonded(bel))
                                .mode("IOB")
                                .pin("I")
                                .pin("DIFFI_IN")
                                .attr("DIFFI_INUSED", "0")
                                .attr("TUSED", "")
                                .attr("IMUX", "I")
                                .attr("BYPASS_MUX", "I")
                                .attr("ISTANDARD", std.name)
                                .prop(Related::new(
                                    Delta::new(0, 0, ioi),
                                    BelUnused::new(bel_ologic),
                                ))
                                .test_manual("DIFF_TERM", "1")
                                .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                                .commit();
                        }
                        if std.name.starts_with("DIFF_") {
                            for term in ["UNTUNED_SPLIT_25", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_75"]
                            {
                                bctx.build()
                                    .global_mutex("IOB", "SHARED")
                                    .global_mutex(format!("VCCO.{kind}"), vcco)
                                    .raw(Key::VccAux, vccaux)
                                    .raw(Key::Package, package.name.clone())
                                    .prop(IsBonded(bel))
                                    .mode("IOB")
                                    .pin("I")
                                    .pin("DIFFI_IN")
                                    .attr("DIFFI_INUSED", "0")
                                    .attr("TUSED", "")
                                    .attr("IMUX", "I")
                                    .attr("BYPASS_MUX", "I")
                                    .attr("ISTANDARD", std.name)
                                    .prop(Related::new(
                                        Delta::new(0, 0, ioi),
                                        BelUnused::new(bel_ologic),
                                    ))
                                    .extra_tile_attr(
                                        Delta::new(0, 0, ioi),
                                        format!("OLOGIC{i}"),
                                        "IN_TERM",
                                        "1",
                                    )
                                    .test_manual(
                                        "IN_TERM",
                                        format!("{sn}:{vccaux}:{kind}:{term}", sn = std.name),
                                    )
                                    .attr("IN_TERM", term)
                                    .commit();
                            }
                        }
                    } else if let Some(vref) = std.vref {
                        bctx.build()
                            .global_mutex("IOB", "SHARED")
                            .global_mutex("VREF", "YES")
                            .global_mutex(format!("VCCO.{kind}"), vcco)
                            .global_mutex(format!("VREF.{kind}"), vref.to_string())
                            .raw(Key::VccAux, vccaux)
                            .raw(Key::Package, package.name.clone())
                            .prop(IsBonded(bel))
                            .mode("IOB")
                            .pin("I")
                            .attr("TUSED", "")
                            .attr("IMUX", "I")
                            .attr("BYPASS_MUX", "I")
                            .bel_mode(bel_other, "IOB")
                            .bel_pin(bel_other, "I")
                            .bel_attr(bel_other, "TUSED", "")
                            .bel_attr(bel_other, "IMUX", "I")
                            .bel_attr(bel_other, "BYPASS_MUX", "I")
                            .bel_attr(bel_other, "ISTANDARD", std.name)
                            .prop(Related::new(
                                Delta::new(0, 0, ioi),
                                BelUnused::new(bel_ologic),
                            ))
                            .test_manual("ISTD", format!("{sn}:{vccaux}:{kind}", sn = std.name))
                            .attr("ISTANDARD", std.name)
                            .commit();
                        for term in ["UNTUNED_SPLIT_25", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_75"] {
                            bctx.build()
                                .global_mutex("IOB", "SHARED")
                                .global_mutex("VREF", "YES")
                                .global_mutex(format!("VCCO.{kind}"), vcco)
                                .global_mutex(format!("VREF.{kind}"), vref.to_string())
                                .raw(Key::VccAux, vccaux)
                                .raw(Key::Package, package.name.clone())
                                .prop(IsBonded(bel))
                                .mode("IOB")
                                .pin("I")
                                .attr("TUSED", "")
                                .attr("IMUX", "I")
                                .attr("BYPASS_MUX", "I")
                                .attr("ISTANDARD", std.name)
                                .prop(Related::new(
                                    Delta::new(0, 0, ioi),
                                    BelUnused::new(bel_ologic),
                                ))
                                .extra_tile_attr(
                                    Delta::new(0, 0, ioi),
                                    format!("OLOGIC{i}"),
                                    "IN_TERM",
                                    "1",
                                )
                                .test_manual(
                                    "IN_TERM",
                                    format!("{sn}:{vccaux}:{kind}:{term}", sn = std.name),
                                )
                                .attr("IN_TERM", term)
                                .commit();
                        }
                    } else {
                        bctx.build()
                            .global_mutex("IOB", "SHARED")
                            .global_mutex(format!("VCCO.{kind}"), vcco)
                            .raw(Key::VccAux, vccaux)
                            .raw(Key::Package, package.name.clone())
                            .prop(IsBonded(bel))
                            .mode("IOB")
                            .pin("I")
                            .attr("TUSED", "")
                            .attr("IMUX", "I")
                            .attr("BYPASS_MUX", "I")
                            .prop(Related::new(
                                Delta::new(0, 0, ioi),
                                BelUnused::new(bel_ologic),
                            ))
                            .test_manual("ISTD", format!("{sn}:{vccaux}:{kind}", sn = std.name))
                            .attr("ISTANDARD", std.name)
                            .commit();
                        if std.name.starts_with("LVCMOS")
                            || std.name == "LVTTL"
                            || std.name == "MOBILE_DDR"
                        {
                            for term in ["UNTUNED_SPLIT_25", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_75"]
                            {
                                bctx.build()
                                    .global_mutex("IOB", "SHARED")
                                    .global_mutex(format!("VCCO.{kind}"), vcco)
                                    .raw(Key::VccAux, vccaux)
                                    .raw(Key::Package, package.name.clone())
                                    .prop(IsBonded(bel))
                                    .mode("IOB")
                                    .pin("I")
                                    .attr("TUSED", "")
                                    .attr("IMUX", "I")
                                    .attr("BYPASS_MUX", "I")
                                    .attr("ISTANDARD", std.name)
                                    .prop(Related::new(
                                        Delta::new(0, 0, ioi),
                                        BelUnused::new(bel_ologic),
                                    ))
                                    .extra_tile_attr(
                                        Delta::new(0, 0, ioi),
                                        format!("OLOGIC{i}"),
                                        "IN_TERM",
                                        "1",
                                    )
                                    .test_manual(
                                        "IN_TERM",
                                        format!("{sn}:{vccaux}:{kind}:{term}", sn = std.name),
                                    )
                                    .attr("IN_TERM", term)
                                    .commit();
                            }
                        }
                    }
                }
                for std in iostds {
                    if std.input_only {
                        continue;
                    }
                    if matches!(std.name, "PCI33_3" | "PCI66_3" | "TMDS_33" | "TML_33")
                        && vccaux == "2.5"
                    {
                        continue;
                    }
                    let vcco = &match std.vcco {
                        Some(vcco) => vcco.to_string(),
                        None => "3300".to_string(),
                    };
                    if std.diff == DiffKind::True {
                        for (dir, corner, corner_name, dx) in
                            [(DirV::S, cnr_ll, "LL", 1), (DirV::N, cnr_ul, "UL", -1)]
                        {
                            bctx.build()
                                .global_mutex("IOB", "SHARED")
                                .global_mutex_here(format!("IOB_DIFF_{corner_name}"))
                                .global_mutex(format!("VCCO.{kind}"), vcco)
                                .raw(Key::VccAux, vccaux)
                                .raw(Key::Package, package.name.clone())
                                .prop(IsBonded(bel))
                                .prop(DeviceSide(dir))
                                .prop(Related::new(
                                    Delta::new(0, 0, ioi),
                                    BelUnused::new(bel_ologic),
                                ))
                                .attr("TUSED", "0")
                                .attr("OUSED", "0")
                                .attr("BYPASS_MUX", "")
                                .attr("SUSPEND", "")
                                .attr("PULLTYPE", "")
                                .pin("T")
                                .pin("O")
                                .extra_tile_attr_fixed(corner, "BANK", "LVDSBIAS_0", std.name)
                                .test_manual("OSTD", format!("{sn}:{vccaux}:GROUP0", sn = std.name))
                                .mode_diff("IOB", ["IOBS", "IOBM"][i])
                                .attr("OUTMUX", ["0", ""][i])
                                .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                .attr("OSTANDARD", std.name)
                                .commit();

                            let other_std = match (std.vcco, std.name) {
                                (Some(2500), "LVDS_25") => "MINI_LVDS_25",
                                (Some(2500), _) => "LVDS_25",
                                (Some(3300), "LVDS_33") => "MINI_LVDS_33",
                                (Some(3300), _) => "LVDS_33",
                                _ => unreachable!(),
                            };
                            bctx.build()
                                .global_mutex("IOB", "SHARED")
                                .global_mutex_here(format!("IOB_DIFF_{corner_name}"))
                                .global_mutex(format!("VCCO.{kind}"), vcco)
                                .raw(Key::VccAux, vccaux)
                                .raw(Key::Package, package.name.clone())
                                .prop(IsBonded(bel))
                                .prop(DeviceSide(dir))
                                .prop(Related::new(
                                    Delta::new(0, 0, ioi),
                                    BelUnused::new(bel_ologic),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, ioi),
                                    BelUnused::new(bel_ologic),
                                ))
                                .prop(Related::new(Delta::new(dx, 0, "IOB"), IsBonded(bel)))
                                .prop(Related::new(
                                    Delta::new(dx, 0, "IOB"),
                                    BaseBelMode::new(bel, ["IOBS", "IOBM"][i].into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, "IOB"),
                                    BaseBelAttr::new(bel, "TUSED".into(), "0".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, "IOB"),
                                    BaseBelAttr::new(bel, "OUSED".into(), "0".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, "IOB"),
                                    BaseBelPin::new(bel, "T".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, "IOB"),
                                    BaseBelPin::new(bel, "O".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, "IOB"),
                                    BaseBelAttr::new(bel, "OUTMUX".into(), ["0", ""][i].into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, "IOB"),
                                    BaseBelAttr::new(bel, "OSTANDARD".into(), other_std.into()),
                                ))
                                .attr("TUSED", "0")
                                .attr("OUSED", "0")
                                .attr("BYPASS_MUX", "")
                                .attr("SUSPEND", "")
                                .attr("PULLTYPE", "")
                                .pin("T")
                                .pin("O")
                                .extra_tile_attr_fixed(corner, "BANK", "LVDSBIAS_1", std.name)
                                .test_manual("OSTD", format!("{sn}:{vccaux}:GROUP1", sn = std.name))
                                .mode_diff("IOB", ["IOBS", "IOBM"][i])
                                .attr("OUTMUX", ["0", ""][i])
                                .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                .attr("OSTANDARD", std.name)
                                .commit();
                        }
                    } else {
                        let (drives, slews) = if std.drive.is_empty() {
                            (&[""][..], &[""][..])
                        } else {
                            (std.drive, &["SLOW", "FAST", "QUIETIO"][..])
                        };
                        for &drive in drives {
                            for &slew in slews {
                                let val = if drive.is_empty() {
                                    format!("{sn}:{vccaux}:{kind}", sn = std.name)
                                } else {
                                    format!("{sn}:{drive}:{slew}:{vccaux}:{kind}", sn = std.name)
                                };
                                bctx.build()
                                    .global_mutex("IOB", "SHARED")
                                    .global_mutex(format!("VCCO.{kind}"), vcco)
                                    .raw(Key::VccAux, vccaux)
                                    .raw(Key::Package, package.name.clone())
                                    .prop(IsBonded(bel))
                                    .mode("IOB")
                                    .prop(Related::new(
                                        Delta::new(0, 0, ioi),
                                        BelUnused::new(bel_ologic),
                                    ))
                                    .attr("TUSED", "0")
                                    .attr("OUSED", "0")
                                    .attr("BYPASS_MUX", "")
                                    .attr("SUSPEND", "")
                                    .attr("PULLTYPE", "")
                                    .pin("T")
                                    .pin("O")
                                    .test_manual("OSTD", val)
                                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                    .attr("OSTANDARD", std.name)
                                    .attr("DRIVEATTRBOX", drive)
                                    .attr("SLEW", slew)
                                    .commit();
                            }
                        }
                        if std.name == "LVTTL"
                            || std.name.starts_with("LVCMOS")
                            || std.name.contains("HSTL")
                            || std.name.contains("SSTL")
                            || std.name.contains("MOBILE_DDR")
                        {
                            for term in ["UNTUNED_25", "UNTUNED_50", "UNTUNED_75"] {
                                let val = if std.drive.is_empty() {
                                    format!("{sn}:{term}:{vccaux}:{kind}", sn = std.name)
                                } else {
                                    format!(
                                        "{sn}:{term}:{slew}:{vccaux}:{kind}",
                                        sn = std.name,
                                        slew = slews[0]
                                    )
                                };
                                bctx.build()
                                    .global_mutex("IOB", "SHARED")
                                    .global_mutex(format!("VCCO.{kind}"), vcco)
                                    .raw(Key::VccAux, vccaux)
                                    .raw(Key::Package, package.name.clone())
                                    .prop(IsBonded(bel))
                                    .mode("IOB")
                                    .prop(Related::new(
                                        Delta::new(0, 0, ioi),
                                        BelUnused::new(bel_ologic),
                                    ))
                                    .attr("TUSED", "0")
                                    .attr("OUSED", "0")
                                    .attr("BYPASS_MUX", "")
                                    .attr("SUSPEND", "")
                                    .attr("PULLTYPE", "")
                                    .pin("T")
                                    .pin("O")
                                    .test_manual("OSTD", val)
                                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                    .attr("OSTANDARD", std.name)
                                    .attr("OUT_TERM", term)
                                    .attr("SLEW", slews[0])
                                    .commit();
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Spartan6(edev) = ctx.edev else {
        unreachable!()
    };
    for tile in ["IOI.LR", "IOI.BT"] {
        for i in 0..2 {
            let bel = &format!("ILOGIC{i}");
            ctx.state
                .get_diff(tile, bel, "MODE", "ILOGIC2")
                .assert_empty();
            // TODO: wtf is this bit really? could be MUX.IOCE...
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE.IOCE", "1");
            let diff = ctx.state.get_diff(tile, bel, "MUX.CLK", format!("ICLK{i}"));
            assert_eq!(diff.bits.len(), 1);
            let mut diff2 = Diff::default();
            for (&k, &v) in &diff.bits {
                diff2.bits.insert(
                    TileBit {
                        bit: RectBitId::from_idx(k.bit.to_idx() ^ 1),
                        ..k
                    },
                    v,
                );
            }
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.CLK",
                xlat_enum_ocd(
                    vec![
                        ("NONE".to_string(), Diff::default()),
                        (format!("ICLK{i}"), diff),
                        (format!("ICLK{}", i ^ 1), diff2),
                    ],
                    OcdMode::BitOrder,
                ),
            );

            ctx.collect_enum_bool(tile, bel, "BITSLIP_ENABLE", "FALSE", "TRUE");
            let item = ctx.extract_bit(tile, bel, "SRUSED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_SR_USED", item);
            let item = ctx.extract_bit(tile, bel, "REVUSED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_REV_USED", item);
            let item = ctx.extract_enum_bool(tile, bel, "SRTYPE_Q", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "IFF_SR_SYNC", item);
            ctx.state
                .get_diff(tile, bel, "SRINIT_Q", "0")
                .assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "SRINIT_Q", "1");
            let diff_init = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 38 | 41));
            ctx.tiledb.insert(tile, bel, "IFF_SRVAL", xlat_bit(diff));
            ctx.tiledb
                .insert(tile, bel, "IFF_INIT", xlat_bit(diff_init));
            ctx.collect_bit(tile, bel, "IFF_CE_ENABLE", "0");
            let item = ctx.extract_enum(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
            ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);

            ctx.collect_enum(tile, bel, "MUX.SR", &["INT", "OLOGIC_SR"]);

            if i == 0 {
                ctx.collect_enum_default(tile, bel, "MUX.D", &["OTHER_IOB_I"], "IOB_I");
            }

            let mut serdes = ctx.state.get_diff(tile, bel, "MODE", "ISERDES2");
            let mut diff_ff = ctx.state.get_diff(tile, bel, "IFFTYPE", "#FF");
            let diff_latch = ctx
                .state
                .get_diff(tile, bel, "IFFTYPE", "#LATCH")
                .combine(&!&diff_ff);
            let mut diff_ddr = ctx.state.get_diff(tile, bel, "IFFTYPE", "DDR");
            ctx.tiledb
                .insert(tile, bel, "IFF_LATCH", xlat_bit(diff_latch));

            diff_ff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_CE_ENABLE"), false, true);
            diff_ff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
            diff_ff.assert_empty();
            diff_ddr.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_CE_ENABLE"), false, true);

            let mut diff_n = ctx
                .state
                .get_diff(tile, bel, "INTERFACE_TYPE", "NETWORKING");
            let mut diff_np =
                ctx.state
                    .get_diff(tile, bel, "INTERFACE_TYPE", "NETWORKING_PIPELINED");
            let mut diff_r = ctx.state.get_diff(tile, bel, "INTERFACE_TYPE", "RETIMED");
            for (attr, range) in [
                ("MUX.Q1", 46..50),
                ("MUX.Q2", 44..52),
                ("MUX.Q3", 42..54),
                ("MUX.Q4", 40..56),
            ] {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    attr,
                    xlat_enum(vec![
                        ("SHIFT_REGISTER", Diff::default()),
                        (
                            "NETWORKING",
                            diff_n.split_bits_by(|bit| range.contains(&bit.bit.to_idx())),
                        ),
                        (
                            "NETWORKING_PIPELINED",
                            diff_np.split_bits_by(|bit| range.contains(&bit.bit.to_idx())),
                        ),
                        (
                            "RETIMED",
                            diff_r.split_bits_by(|bit| range.contains(&bit.bit.to_idx())),
                        ),
                    ]),
                );
            }
            diff_n.assert_empty();
            diff_np.assert_empty();
            diff_r.assert_empty();

            let mut diff_1 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "1");
            let mut diff_2 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "2");
            let mut diff_3 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "3");
            let mut diff_4 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "4");
            let mut diff_5 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "5");
            let mut diff_6 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "6");
            let mut diff_7 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "7");
            let mut diff_8 = ctx.state.get_diff(tile, bel, "DATA_WIDTH", "8");
            let mut diff_1_f = Diff::default();
            let mut diff_2_f = Diff::default();
            let mut diff_3_f = Diff::default();
            let mut diff_4_f = Diff::default();
            for (diff, diff_f) in [
                (&mut diff_1, &mut diff_1_f),
                (&mut diff_2, &mut diff_2_f),
                (&mut diff_3, &mut diff_3_f),
                (&mut diff_4, &mut diff_4_f),
            ] {
                diff.bits.retain(|k, v| {
                    if !*v {
                        diff_f.bits.insert(*k, *v);
                    }
                    *v
                });
            }
            diff_1_f = diff_1_f.combine(&!&diff_2_f);
            diff_2_f = diff_2_f.combine(&!&diff_3_f);
            diff_3_f = diff_3_f.combine(&!&diff_4_f);

            if i == 0 {
                serdes = serdes.combine(&diff_4_f);
                ctx.tiledb
                    .insert(tile, bel, "CASCADE_ENABLE", xlat_bit(!diff_4_f));
            } else {
                diff_4_f.assert_empty();
            }

            serdes = serdes
                .combine(&diff_1_f)
                .combine(&diff_2_f)
                .combine(&diff_3_f);
            diff_ddr = diff_ddr.combine(&diff_1_f);
            ctx.tiledb
                .insert(tile, bel, "ROW2_CLK_ENABLE", xlat_bit(!diff_1_f));
            ctx.tiledb
                .insert(tile, bel, "ROW3_CLK_ENABLE", xlat_bit(!diff_2_f));
            ctx.tiledb
                .insert(tile, bel, "ROW4_CLK_ENABLE", xlat_bit(!diff_3_f));

            let (serdes, mut diff_ddr, diff_row1) = Diff::split(serdes, diff_ddr);
            ctx.tiledb
                .insert(tile, bel, "ROW1_CLK_ENABLE", xlat_bit(diff_row1));

            serdes.assert_empty();

            let diff_1_a = diff_1.split_bits_by(|bit| bit.frame.to_idx() == 27);
            let diff_2_a = diff_2.split_bits_by(|bit| bit.frame.to_idx() == 27);
            let diff_3_a = diff_3.split_bits_by(|bit| bit.frame.to_idx() == 27);
            let diff_4_a = diff_4.split_bits_by(|bit| bit.frame.to_idx() == 27);
            let diff_5_a = diff_5.split_bits_by(|bit| bit.frame.to_idx() == 27);
            let diff_6_a = diff_6.split_bits_by(|bit| bit.frame.to_idx() == 27);
            let diff_7_a = diff_7.split_bits_by(|bit| bit.frame.to_idx() == 27);
            let diff_8_a = diff_8.split_bits_by(|bit| bit.frame.to_idx() == 27);

            assert_eq!(diff_1, diff_2);
            if i == 1 {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "DATA_WIDTH_RELOAD",
                    xlat_enum(vec![
                        ("8", diff_8_a),
                        ("7", diff_7_a),
                        ("6", diff_6_a),
                        ("5", diff_5_a),
                        ("4", diff_4_a),
                        ("3", diff_3_a),
                        ("2", diff_2_a),
                        ("1", diff_1_a),
                    ]),
                );
                let (diff_5, diff_6, diff_casc) = Diff::split(diff_5, diff_6);
                let diff_7 = diff_7.combine(&!&diff_casc);
                let diff_8 = diff_8.combine(&!&diff_casc);
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "DATA_WIDTH_START",
                    xlat_enum(vec![
                        ("2", diff_2),
                        ("3", diff_3),
                        ("4", diff_4),
                        ("5", diff_5),
                        ("6", diff_6),
                        ("7", diff_7),
                        ("8", diff_8),
                    ]),
                );
                ctx.tiledb
                    .insert(tile, bel, "CASCADE_ENABLE", xlat_bit(diff_casc));
                diff_ddr.apply_enum_diff(ctx.tiledb.item(tile, bel, "DATA_WIDTH_RELOAD"), "2", "8");
            } else {
                assert_eq!(diff_3_a, diff_5_a);
                assert_eq!(diff_3_a, diff_6_a);
                assert_eq!(diff_3_a, diff_7_a);
                assert_eq!(diff_3_a, diff_8_a);
                diff_5.assert_empty();
                diff_6.assert_empty();
                diff_7.assert_empty();
                diff_8.assert_empty();
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "DATA_WIDTH_RELOAD",
                    xlat_enum(vec![
                        ("4", diff_4_a),
                        ("3", diff_3_a),
                        ("2", diff_2_a),
                        ("1", diff_1_a),
                    ]),
                );
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "DATA_WIDTH_START",
                    xlat_enum(vec![("2", diff_2), ("3", diff_3), ("4", diff_4)]),
                );
                diff_ddr.apply_enum_diff(ctx.tiledb.item(tile, bel, "DATA_WIDTH_RELOAD"), "2", "4");
            }
            diff_ddr.apply_enum_diff(ctx.tiledb.item(tile, bel, "DATA_WIDTH_START"), "3", "2");

            ctx.tiledb.insert(tile, bel, "DDR", xlat_bit(diff_ddr));
        }
        for i in 0..2 {
            let bel = &format!("OLOGIC{i}");
            ctx.state
                .get_diff(tile, bel, "MODE", "OLOGIC2")
                .assert_empty();
            ctx.collect_bit(tile, bel, "ENABLE", "1");
            ctx.collect_bit(tile, bel, "ENABLE.IOCE", "1");
            let diff = ctx.state.get_diff(tile, bel, "MUX.CLK", format!("OCLK{i}"));
            assert_eq!(diff.bits.len(), 1);
            let mut diff2 = Diff::default();
            for (&k, &v) in &diff.bits {
                diff2.bits.insert(
                    TileBit {
                        bit: RectBitId::from_idx(k.bit.to_idx() ^ 1),
                        ..k
                    },
                    v,
                );
            }
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.CLK",
                xlat_enum_ocd(
                    vec![
                        ("NONE".to_string(), Diff::default()),
                        (format!("OCLK{i}"), diff),
                        (format!("OCLK{}", i ^ 1), diff2),
                    ],
                    OcdMode::BitOrder,
                ),
            );

            for (attr, sattr) in [
                ("OFF_SR_ENABLE", "OSRUSED"),
                ("TFF_SR_ENABLE", "TSRUSED"),
                ("OFF_REV_ENABLE", "OREVUSED"),
                ("TFF_REV_ENABLE", "TREVUSED"),
            ] {
                let item = ctx.extract_bit(tile, bel, sattr, "0");
                ctx.tiledb.insert(tile, bel, attr, item);
            }
            for attr in ["MUX.REV", "MUX.SR"] {
                ctx.collect_enum_default(tile, bel, attr, &["INT"], "GND");
            }
            let item = ctx.extract_enum_bool(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "OFF_SR_SYNC", item);
            let item = ctx.extract_enum_bool(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "TFF_SR_SYNC", item);

            ctx.collect_bitvec(tile, bel, "TRAIN_PATTERN", "");
            ctx.collect_enum_default(tile, bel, "MUX.TRAIN", &["INT", "MCB"], "GND");
            let item = ctx.extract_bit(tile, bel, "MISRATTRBOX", "MISR_ENABLE_DATA");
            ctx.tiledb.insert(tile, bel, "MISR_ENABLE_DATA", item);
            let item = ctx.extract_bit(tile, bel, "MODE", "OLOGIC2.MISR_RESET");
            ctx.tiledb.insert(tile, bel, "MISR_RESET", item);
            for val in ["CLK0", "CLK1"] {
                let item = ctx.extract_bit(tile, bel, "MISR_ENABLE_CLK", val);
                ctx.tiledb.insert(tile, bel, "MISR_ENABLE_CLK", item);
            }
            for val in ["1", "2", "3", "4"] {
                ctx.state
                    .get_diff(tile, bel, "DATA_WIDTH", val)
                    .assert_empty();
            }
            for val in ["5", "6", "7", "8"] {
                let item = ctx.extract_bit(tile, bel, "DATA_WIDTH", val);
                ctx.tiledb.insert(tile, bel, "CASCADE_ENABLE", item);
            }
            if i == 1 {
                ctx.state
                    .get_diff(tile, bel, "OUTPUT_MODE", "SINGLE_ENDED")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, bel, "OUTPUT_MODE", "DIFFERENTIAL")
                    .assert_empty();
            } else {
                ctx.collect_enum(tile, bel, "OUTPUT_MODE", &["SINGLE_ENDED", "DIFFERENTIAL"]);
            }

            let mut serdes = ctx.state.get_diff(tile, bel, "MODE", "OSERDES2");
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);

            ctx.state
                .get_diff(tile, bel, "SRINIT_OQ", "0")
                .assert_empty();
            ctx.state
                .get_diff(tile, bel, "SRINIT_TQ", "0")
                .assert_empty();
            let diff = ctx.state.get_diff(tile, bel, "SRINIT_TQ", "1");
            let (mut serdes, diff_init, diff_srval) = Diff::split(serdes, diff);
            ctx.tiledb
                .insert(tile, bel, "TFF_INIT", xlat_bit(diff_init));
            ctx.tiledb
                .insert(tile, bel, "TFF_SRVAL", xlat_bit(diff_srval));
            let mut diff = ctx.state.get_diff(tile, bel, "SRINIT_OQ", "1");
            let diff_srval = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 8 | 24));
            ctx.tiledb.insert(tile, bel, "OFF_INIT", xlat_bit(diff));
            ctx.tiledb
                .insert(tile, bel, "OFF_SRVAL", xlat_bit(diff_srval));

            let mut diff = ctx.state.get_diff(tile, bel, "MUX.D", "MCB");
            let diff_t = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 2 | 28));
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.T",
                xlat_enum(vec![("INT", Diff::default()), ("MCB", diff_t)]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.D",
                xlat_enum(vec![("INT", Diff::default()), ("MCB", diff)]),
            );

            ctx.collect_enum(tile, bel, "OMUX", &["D1", "OUTFF"]);
            let diff = ctx.state.get_diff(tile, bel, "OT1USED", "0");
            ctx.tiledb.insert(
                tile,
                bel,
                "TMUX",
                xlat_enum(vec![("TFF", Diff::default()), ("T1", diff)]),
            );

            let item = ctx.extract_bit(tile, bel, "DDR_ALIGNMENT", "NONE");
            ctx.tiledb.insert(tile, bel, "DDR_OPPOSITE_EDGE", item);
            let item = ctx.extract_bit(tile, bel, "TDDR_ALIGNMENT", "NONE");
            ctx.tiledb.insert(tile, bel, "DDR_OPPOSITE_EDGE", item);

            let item = ctx.extract_bit(tile, bel, "DDR_ALIGNMENT", "C0");
            ctx.tiledb.insert(tile, bel, "OFF_RANK2_CLK_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "TDDR_ALIGNMENT", "C0");
            ctx.tiledb.insert(tile, bel, "TFF_RANK2_CLK_ENABLE", item);

            let mut diff = ctx.state.get_diff(tile, bel, "BYPASS_GCLK_FF", "FALSE");
            let diff_t = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 6 | 22));
            ctx.tiledb
                .insert(tile, bel, "OFF_RANK1_CLK_ENABLE", xlat_bit(diff));
            ctx.tiledb
                .insert(tile, bel, "TFF_RANK1_CLK_ENABLE", xlat_bit(diff_t));

            let diff_bypass = ctx.state.get_diff(tile, bel, "BYPASS_GCLK_FF", "TRUE");
            let diff_olatch = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#LATCH");
            let diff_off = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#FF");
            let diff_oddr = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "DDR");
            let diff_tlatch = ctx.state.get_diff(tile, bel, "TFFTYPE", "#LATCH");
            let diff_tff = ctx.state.get_diff(tile, bel, "TFFTYPE", "#FF");
            let diff_tddr = ctx.state.get_diff(tile, bel, "TFFTYPE", "DDR");
            let diff_oce = ctx.state.get_diff(tile, bel, "MUX.OCE", "INT");
            let diff_oce_pci = ctx.state.get_diff(tile, bel, "MUX.OCE", "PCI_CE");
            let diff_tce = ctx.state.get_diff(tile, bel, "MUX.TCE", "INT");

            let diff_oce_pci = diff_oce_pci.combine(&!&diff_oce);
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.OCE",
                xlat_enum(vec![("INT", Diff::default()), ("PCI_CE", diff_oce_pci)]),
            );

            let diff_tlatch = diff_tlatch.combine(&!&diff_tff);
            let diff_olatch = diff_olatch.combine(&!&diff_tlatch).combine(&!&diff_off);
            ctx.tiledb
                .insert(tile, bel, "TFF_LATCH", xlat_bit(diff_tlatch));
            ctx.tiledb
                .insert(tile, bel, "OFF_LATCH", xlat_bit(diff_olatch));

            let (diff_tff, diff_obypass, diff_tbypass) = Diff::split(diff_tff, diff_bypass);
            let diff_tddr = diff_tddr.combine(&!&diff_tbypass);
            let diff_off = diff_off.combine(&!&diff_obypass);
            let diff_oddr = diff_oddr.combine(&!&diff_obypass);
            ctx.tiledb
                .insert(tile, bel, "OFF_RANK1_BYPASS", xlat_bit(diff_obypass));
            ctx.tiledb
                .insert(tile, bel, "TFF_RANK1_BYPASS", xlat_bit(diff_tbypass));

            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff_off));
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff_tff));

            let diff_oce = diff_oce.combine(&!&diff_oddr);
            let diff_tce = diff_tce.combine(&!&diff_tddr);
            ctx.tiledb
                .insert(tile, bel, "OFF_CE_OR_DDR", xlat_bit(diff_oddr));
            ctx.tiledb
                .insert(tile, bel, "TFF_CE_OR_DDR", xlat_bit(diff_tddr));

            ctx.tiledb
                .insert(tile, bel, "OFF_CE_ENABLE", xlat_bit(diff_oce));
            ctx.tiledb
                .insert(tile, bel, "TFF_CE_ENABLE", xlat_bit(diff_tce));

            serdes.apply_bit_diff(
                ctx.tiledb.item(tile, bel, "OFF_RANK2_CLK_ENABLE"),
                true,
                false,
            );
            serdes.apply_bit_diff(
                ctx.tiledb.item(tile, bel, "TFF_RANK2_CLK_ENABLE"),
                true,
                false,
            );
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_CE_ENABLE"), true, false);
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_CE_ENABLE"), true, false);
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_CE_OR_DDR"), true, false);
            serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_CE_OR_DDR"), true, false);

            serdes.assert_empty();

            let mut diff = ctx.state.get_diff(tile, bel, "IN_TERM", "1");
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_INIT"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_CE_ENABLE"), true, false);
            diff.assert_empty();
        }
        let (_, _, diff) = Diff::split(
            ctx.state
                .peek_diff(tile, "IODELAY0", "MODE", "IODRP2")
                .clone(),
            ctx.state
                .peek_diff(tile, "IODELAY1", "MODE", "IODRP2")
                .clone(),
        );
        let (_, _, diff_mcb) = Diff::split(
            ctx.state
                .peek_diff(tile, "IODELAY0", "MODE", "IODRP2_MCB")
                .clone(),
            ctx.state
                .peek_diff(tile, "IODELAY1", "MODE", "IODRP2_MCB")
                .clone(),
        );
        let diff_mcb = diff_mcb.combine(&!&diff);
        ctx.tiledb
            .insert(tile, "IODELAY_COMMON", "DRP_ENABLE", xlat_bit(diff));
        ctx.tiledb
            .insert(tile, "IODELAY_COMMON", "DRP_FROM_MCB", xlat_bit(diff_mcb));

        for i in 0..2 {
            let bel = &format!("IODELAY{i}");
            let diffs = ctx.state.get_diffs(tile, bel, "ODELAY_VALUE", "");
            let mut diffs_p = vec![];
            let mut diffs_n = vec![];
            for mut diff in diffs {
                let diff_p = diff.split_bits_by(|bit| (16..48).contains(&bit.bit.to_idx()));
                diffs_p.push(diff_p);
                diffs_n.push(diff);
            }
            ctx.tiledb
                .insert(tile, bel, "ODELAY_VALUE_P", xlat_bitvec(diffs_p));
            ctx.tiledb
                .insert(tile, bel, "ODELAY_VALUE_N", xlat_bitvec(diffs_n));
            let item = ctx.extract_bitvec(tile, bel, "IDELAY_VALUE", "");
            ctx.tiledb.insert(tile, bel, "IDELAY_VALUE_P", item);
            let item = ctx.extract_bitvec(tile, bel, "IDELAY2_VALUE", "");
            ctx.tiledb.insert(tile, bel, "IDELAY_VALUE_N", item);
            if i == 1 {
                let item = ctx.extract_bitvec(tile, bel, "MCB_ADDRESS", "");
                ctx.tiledb
                    .insert(tile, "IODELAY_COMMON", "MCB_ADDRESS", item);
            } else {
                let diffs = ctx.state.get_diffs(tile, bel, "MCB_ADDRESS", "");
                for diff in diffs {
                    diff.assert_empty();
                }
            }
            ctx.collect_bit_wide(tile, bel, "ENABLE.CIN", "1");
            ctx.collect_enum_bool(tile, bel, "TEST_GLITCH_FILTER", "FALSE", "TRUE");
            ctx.collect_enum(
                tile,
                bel,
                "COUNTER_WRAPAROUND",
                &["WRAPAROUND", "STAY_AT_LIMIT"],
            );
            ctx.collect_enum(
                tile,
                bel,
                "IODELAY_CHANGE",
                &["CHANGE_ON_CLOCK", "CHANGE_ON_DATA"],
            );
            let diff = ctx
                .state
                .get_diff(tile, bel, "MODE", "IODELAY2.TEST_NCOUNTER")
                .combine(&!ctx.state.peek_diff(tile, bel, "MODE", "IODELAY2"));
            ctx.tiledb
                .insert(tile, bel, "TEST_NCOUNTER", xlat_bit(diff));
            let diff = ctx
                .state
                .get_diff(tile, bel, "MODE", "IODELAY2.TEST_PCOUNTER")
                .combine(&!ctx.state.peek_diff(tile, bel, "MODE", "IODELAY2"));
            ctx.tiledb
                .insert(tile, bel, "TEST_PCOUNTER", xlat_bit(diff));
            let diff = ctx
                .state
                .get_diff(tile, bel, "MODE", "IODRP2.IOIENFFSCAN_DRP")
                .combine(&!ctx.state.peek_diff(tile, bel, "MODE", "IODRP2"));
            ctx.tiledb
                .insert(tile, "IODELAY_COMMON", "ENFFSCAN_DRP", xlat_bit_wide(diff));

            ctx.collect_bit(tile, bel, "ENABLE.ODATAIN", "1");
            ctx.collect_enum(tile, bel, "MUX.IOCLK", &["ILOGIC_CLK", "OLOGIC_CLK"]);

            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE", "DEFAULT");
            ctx.tiledb.insert(tile, bel, "IDELAY_FIXED", item);
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE", "FIXED");
            ctx.tiledb.insert(tile, bel, "IDELAY_FIXED", item);
            ctx.state
                .get_diff(tile, bel, "IDELAY_TYPE", "VARIABLE_FROM_ZERO")
                .assert_empty();
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE", "VARIABLE_FROM_HALF_MAX");
            ctx.tiledb.insert(tile, bel, "IDELAY_FROM_HALF_MAX", item);
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE.DPD", "DEFAULT");
            ctx.tiledb.insert(tile, bel, "IDELAY_FIXED", item);
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE.DPD", "FIXED");
            ctx.tiledb.insert(tile, bel, "IDELAY_FIXED", item);
            ctx.state
                .get_diff(tile, bel, "IDELAY_TYPE.DPD", "VARIABLE_FROM_ZERO")
                .assert_empty();
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE.DPD", "VARIABLE_FROM_HALF_MAX");
            ctx.tiledb.insert(tile, bel, "IDELAY_FROM_HALF_MAX", item);
            let item = ctx.extract_bit(tile, bel, "IDELAY_TYPE", "DIFF_PHASE_DETECTOR");
            ctx.tiledb.insert(tile, bel, "DIFF_PHASE_DETECTOR", item);

            ctx.tiledb.insert(
                tile,
                bel,
                "CAL_DELAY_MAX",
                TileItem::from_bitvec(
                    vec![
                        TileBit::new(0, 28, [63, 0][i]),
                        TileBit::new(0, 28, [62, 1][i]),
                        TileBit::new(0, 28, [61, 2][i]),
                        TileBit::new(0, 28, [60, 3][i]),
                        TileBit::new(0, 28, [59, 4][i]),
                        TileBit::new(0, 28, [58, 5][i]),
                        TileBit::new(0, 28, [57, 6][i]),
                        TileBit::new(0, 28, [56, 7][i]),
                    ],
                    false,
                ),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "DRP_ADDR",
                TileItem::from_bitvec(
                    vec![
                        TileBit::new(0, 28, [39, 24][i]),
                        TileBit::new(0, 28, [38, 25][i]),
                        TileBit::new(0, 28, [37, 26][i]),
                        TileBit::new(0, 28, [36, 27][i]),
                        TileBit::new(0, 28, [32, 31][i]),
                    ],
                    false,
                ),
            );
            let drp06 = TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 28, [45, 18][i]),
                    TileBit::new(0, 28, [47, 16][i]),
                    TileBit::new(0, 28, [50, 13][i]),
                    TileBit::new(0, 28, [53, 10][i]),
                    TileBit::new(0, 28, [55, 8][i]),
                    TileBit::new(0, 28, [49, 14][i]),
                    TileBit::new(0, 28, [41, 22][i]),
                    TileBit::new(0, 28, [43, 20][i]),
                ],
                false,
            );
            let drp07 = TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 28, [44, 19][i]),
                    TileBit::new(0, 28, [46, 17][i]),
                    TileBit::new(0, 28, [51, 12][i]),
                    TileBit::new(0, 28, [52, 11][i]),
                    TileBit::new(0, 28, [54, 9][i]),
                    TileBit::new(0, 28, [48, 15][i]),
                    TileBit::new(0, 28, [40, 23][i]),
                    TileBit::new(0, 28, [42, 21][i]),
                ],
                false,
            );
            if i == 1 {
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "EVENT_SEL",
                    TileItem::from_bitvec(drp06.bits[0..2].to_vec(), false),
                );
            } else {
                ctx.tiledb
                    .insert(tile, bel, "PLUS1", TileItem::from_bit(drp06.bits[0], false));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                "LUMPED_DELAY",
                TileItem::from_bit(drp07.bits[3], false),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "LUMPED_DELAY_SELECT",
                TileItem::from_bit(drp07.bits[4], false),
            );
            ctx.tiledb.insert(tile, bel, "DRP06", drp06);
            ctx.tiledb.insert(tile, bel, "DRP07", drp07);

            ctx.collect_enum(tile, bel, "DELAY_SRC", &["IDATAIN", "ODATAIN", "IO"]);
            ctx.state
                .get_diff(tile, bel, "IDELAY_MODE", "NORMAL")
                .assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "IDELAY_MODE", "PCI");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "ODATAIN", "IO");
            ctx.tiledb.insert(
                tile,
                bel,
                "IDELAY_MODE",
                xlat_enum(vec![("NORMAL", Diff::default()), ("PCI", diff)]),
            );

            ctx.state
                .get_diff(tile, bel, "DELAYCHAIN_OSC", "FALSE")
                .assert_empty();
            let mut diff_iodelay2 = ctx.state.get_diff(tile, bel, "MODE", "IODELAY2");
            let mut diff_iodrp2 = ctx.state.get_diff(tile, bel, "MODE", "IODRP2");
            let mut diff_iodrp2_mcb = ctx.state.get_diff(tile, bel, "MODE", "IODRP2_MCB");
            let diff_delaychain_osc = ctx.state.get_diff(tile, bel, "DELAYCHAIN_OSC", "TRUE");
            diff_iodrp2.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY_COMMON", "DRP_ENABLE"),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY_COMMON", "DRP_ENABLE"),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY_COMMON", "DRP_FROM_MCB"),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_enum_diff(
                ctx.tiledb.item(tile, bel, "MUX.IOCLK"),
                "OLOGIC_CLK",
                "ILOGIC_CLK",
            );
            if i == 1 {
                diff_iodelay2.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "EVENT_SEL"), 3, 0);
                diff_iodrp2.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "EVENT_SEL"), 3, 0);
                diff_iodrp2_mcb.apply_bitvec_diff_int(
                    ctx.tiledb.item(tile, bel, "EVENT_SEL"),
                    3,
                    0,
                );
            }
            let (diff_iodrp2_mcb, diff_delaychain_osc, diff_common) =
                Diff::split(diff_iodrp2_mcb, diff_delaychain_osc);
            ctx.tiledb.insert(
                tile,
                bel,
                "DELAYCHAIN_OSC_OR_ODATAIN_LP_OR_IDRP2_MCB",
                xlat_bit_wide(diff_common),
            );
            ctx.tiledb
                .insert(tile, bel, "DELAYCHAIN_OSC", xlat_bit(diff_delaychain_osc));
            ctx.tiledb.insert(
                tile,
                bel,
                "MODE",
                xlat_enum(vec![
                    ("IODELAY2", diff_iodelay2),
                    ("IODRP2", diff_iodrp2),
                    ("IODRP2_MCB", diff_iodrp2_mcb),
                ]),
            );
        }
        {
            let mut diff0 =
                ctx.state
                    .get_diff(tile, "IODELAY0", "IDELAY_TYPE.DPD", "DIFF_PHASE_DETECTOR");
            let mut diff1 =
                ctx.state
                    .get_diff(tile, "IODELAY1", "IDELAY_TYPE.DPD", "DIFF_PHASE_DETECTOR");
            diff0.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY0", "DIFF_PHASE_DETECTOR"),
                true,
                false,
            );
            diff1.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY1", "DIFF_PHASE_DETECTOR"),
                true,
                false,
            );
            diff0.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY1", "IDELAY_FROM_HALF_MAX"),
                true,
                false,
            );
            diff1.apply_bit_diff(
                ctx.tiledb.item(tile, "IODELAY1", "IDELAY_FROM_HALF_MAX"),
                true,
                false,
            );
            ctx.tiledb.insert(
                tile,
                "IODELAY_COMMON",
                "DIFF_PHASE_DETECTOR",
                xlat_bit(diff0),
            );
            ctx.tiledb.insert(
                tile,
                "IODELAY_COMMON",
                "DIFF_PHASE_DETECTOR",
                xlat_bit(diff1),
            );
        }
        for i in 0..2 {
            let bel = &format!("IOICLK{i}");
            ctx.collect_bit(tile, bel, "INV.CLK0", "1");
            ctx.collect_bit(tile, bel, "INV.CLK1", "1");
            ctx.collect_bit(tile, bel, "INV.CLK2", "1");
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.CLK0",
                &["IOCLK0", "IOCLK2", "PLLCLK0", "CKINT0", "CKINT1"],
                "NONE",
            );
            ctx.collect_enum_default(
                tile,
                bel,
                "MUX.CLK1",
                &["IOCLK1", "IOCLK3", "PLLCLK1", "CKINT0", "CKINT1"],
                "NONE",
            );
            ctx.collect_enum_default(tile, bel, "MUX.CLK2", &["PLLCLK0", "PLLCLK1"], "NONE");

            let diff_iddr = ctx.state.get_diff(tile, bel, "MUX.ICLK", "DDR");
            let diff_iddr_ce = ctx.state.get_diff(tile, bel, "MUX.ICLK", "DDR.ILOGIC");
            let diff_iddr_ce_c0 = ctx.state.get_diff(tile, bel, "MUX.ICLK", "DDR.ILOGIC.C0");
            let diff_iddr_ce_c1 = ctx.state.get_diff(tile, bel, "MUX.ICLK", "DDR.ILOGIC.C1");
            let diff_oddr = ctx.state.get_diff(tile, bel, "MUX.OCLK", "DDR");
            let diff_oddr_ce = ctx.state.get_diff(tile, bel, "MUX.OCLK", "DDR.OLOGIC");
            let diff_c0 = diff_iddr_ce_c0.combine(&!&diff_iddr_ce);
            let diff_c1 = diff_iddr_ce_c1.combine(&!&diff_iddr_ce);
            let diff_iddr_ce = diff_iddr_ce.combine(&!&diff_iddr);
            let diff_oddr_ce = diff_oddr_ce.combine(&!&diff_oddr);
            let (diff_iddr, diff_oddr, diff_ddr) = Diff::split(diff_iddr, diff_oddr);
            ctx.tiledb.insert(
                tile,
                bel,
                "DDR_ALIGNMENT",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("CLK0", diff_c0),
                    ("CLK1", diff_c1),
                ]),
            );
            let item = xlat_enum(vec![
                ("NONE", Diff::default()),
                ("CLK0", ctx.state.get_diff(tile, bel, "MUX.ICLK", "CLK0")),
                ("CLK1", ctx.state.get_diff(tile, bel, "MUX.ICLK", "CLK1")),
                ("CLK2", ctx.state.get_diff(tile, bel, "MUX.ICLK", "CLK2")),
                ("DDR", diff_iddr),
            ]);
            ctx.tiledb.insert(tile, bel, "MUX.ICLK", item);
            let item = xlat_enum(vec![
                ("NONE", Diff::default()),
                ("CLK0", ctx.state.get_diff(tile, bel, "MUX.OCLK", "CLK0")),
                ("CLK1", ctx.state.get_diff(tile, bel, "MUX.OCLK", "CLK1")),
                ("CLK2", ctx.state.get_diff(tile, bel, "MUX.OCLK", "CLK2")),
                ("DDR", diff_oddr),
            ]);
            ctx.tiledb.insert(tile, bel, "MUX.OCLK", item);
            ctx.tiledb
                .insert(tile, bel, "DDR_ENABLE", xlat_bit_wide(diff_ddr));

            let diff_ice_ioce0 = ctx.state.get_diff(tile, bel, "MUX.ICE", "IOCE0");
            let diff_ice_ioce1 = ctx.state.get_diff(tile, bel, "MUX.ICE", "IOCE1");
            let diff_ice_ioce2 = ctx.state.get_diff(tile, bel, "MUX.ICE", "IOCE2");
            let diff_ice_ioce3 = ctx.state.get_diff(tile, bel, "MUX.ICE", "IOCE3");
            let diff_ice_pllce0 = ctx.state.get_diff(tile, bel, "MUX.ICE", "PLLCE0");
            let diff_ice_pllce1 = ctx.state.get_diff(tile, bel, "MUX.ICE", "PLLCE1");
            let diff_oce_ioce0 = ctx.state.get_diff(tile, bel, "MUX.OCE", "IOCE0");
            let diff_oce_ioce1 = ctx.state.get_diff(tile, bel, "MUX.OCE", "IOCE1");
            let diff_oce_ioce2 = ctx.state.get_diff(tile, bel, "MUX.OCE", "IOCE2");
            let diff_oce_ioce3 = ctx.state.get_diff(tile, bel, "MUX.OCE", "IOCE3");
            let diff_oce_pllce0 = ctx.state.get_diff(tile, bel, "MUX.OCE", "PLLCE0");
            let diff_oce_pllce1 = ctx.state.get_diff(tile, bel, "MUX.OCE", "PLLCE1");
            let (diff_ice_ioce0, diff_oce_ioce0, diff_ioce0) =
                Diff::split(diff_ice_ioce0, diff_oce_ioce0);
            let (diff_ice_ioce1, diff_oce_ioce1, diff_ioce1) =
                Diff::split(diff_ice_ioce1, diff_oce_ioce1);
            let (diff_ice_ioce2, diff_oce_ioce2, diff_ioce2) =
                Diff::split(diff_ice_ioce2, diff_oce_ioce2);
            let (diff_ice_ioce3, diff_oce_ioce3, diff_ioce3) =
                Diff::split(diff_ice_ioce3, diff_oce_ioce3);
            let (diff_ice_pllce0, diff_oce_pllce0, diff_pllce0) =
                Diff::split(diff_ice_pllce0, diff_oce_pllce0);
            let (diff_ice_pllce1, diff_oce_pllce1, diff_pllce1) =
                Diff::split(diff_ice_pllce1, diff_oce_pllce1);
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.ICE",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("CE0", diff_ice_ioce0),
                    ("CE0", diff_ice_ioce2),
                    ("CE0", diff_ice_pllce0),
                    ("CE1", diff_ice_ioce1),
                    ("CE1", diff_ice_ioce3),
                    ("CE1", diff_ice_pllce1),
                    ("DDR", diff_iddr_ce),
                ]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.OCE",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("CE0", diff_oce_ioce0),
                    ("CE0", diff_oce_ioce2),
                    ("CE0", diff_oce_pllce0),
                    ("CE1", diff_oce_ioce1),
                    ("CE1", diff_oce_ioce3),
                    ("CE1", diff_oce_pllce1),
                    ("DDR", diff_oddr_ce),
                ]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.CE0",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("IOCE0", diff_ioce0),
                    ("IOCE2", diff_ioce2),
                    ("PLLCE0", diff_pllce0),
                ]),
            );
            ctx.tiledb.insert(
                tile,
                bel,
                "MUX.CE1",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("IOCE1", diff_ioce1),
                    ("IOCE3", diff_ioce3),
                    ("PLLCE1", diff_pllce1),
                ]),
            );
        }
        let bel = "IOI";
        if tile == "IOI.BT" || ctx.has_tile("MCB") {
            let mut diff = ctx.state.get_diff(tile, bel, "DRPSDO", "1");
            let diff_de = ctx
                .state
                .get_diff(tile, bel, "DRPSDO", "1.DIV_EN")
                .combine(&!&diff);
            let diff_ni = ctx
                .state
                .get_diff(tile, bel, "DRPSDO", "1.NOTINV")
                .combine(&!&diff);
            ctx.tiledb
                .insert(tile, bel, "MEM_PLL_DIV_EN", xlat_bit(diff_de));
            ctx.tiledb.insert(
                tile,
                bel,
                "MEM_PLL_POL_SEL",
                xlat_enum(vec![
                    ("INVERTED", Diff::default()),
                    ("NOTINVERTED", diff_ni),
                ]),
            );
            diff.apply_bitvec_diff_int(
                ctx.tiledb.item(tile, "IODELAY_COMMON", "MCB_ADDRESS"),
                0xa,
                0,
            );
            diff.assert_empty();
        }
    }
    for i in 0..2 {
        let tile = "IOB";
        let bel = &format!("IOB{i}");
        ctx.collect_bit(tile, bel, "OUTPUT_ENABLE", "0");
        ctx.collect_enum_default(
            tile,
            bel,
            "PULLTYPE",
            &["PULLDOWN", "PULLUP", "KEEPER"],
            "NONE",
        );
        ctx.collect_enum(
            tile,
            bel,
            "SUSPEND",
            &[
                "3STATE",
                "DRIVE_LAST_VALUE",
                "3STATE_PULLDOWN",
                "3STATE_PULLUP",
                "3STATE_KEEPER",
                "3STATE_OCT_ON",
            ],
        );
        let item = ctx.extract_enum_bool(tile, bel, "IMUX", "I", "I_B");
        ctx.tiledb.insert(tile, bel, "INV.I", item);
        ctx.collect_bit(tile, bel, "PRE_EMPHASIS", "ON");
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        present.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULLTYPE"), "NONE", "PULLDOWN");

        ctx.state
            .get_diff(tile, bel, "BYPASS_MUX", "I")
            .assert_empty();

        let pdrive_bits: Vec<_> = (0..6).map(|j| TileBit::new(0, 0, i * 64 + j)).collect();
        let pterm_bits: Vec<_> = (0..6).map(|j| TileBit::new(0, 0, i * 64 + 8 + j)).collect();
        let ndrive_bits: Vec<_> = (0..7)
            .map(|j| TileBit::new(0, 0, i * 64 + 16 + j))
            .collect();
        let nterm_bits: Vec<_> = (0..7)
            .map(|j| TileBit::new(0, 0, i * 64 + 24 + j))
            .collect();
        let pdrive_invert: BitVec = pdrive_bits
            .iter()
            .map(|&bit| match present.bits.remove(&bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            })
            .collect();
        let ndrive_invert: BitVec = ndrive_bits
            .iter()
            .map(|&bit| match present.bits.remove(&bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            })
            .collect();
        let pdrive = TileItem {
            bits: pdrive_bits,
            kind: TileItemKind::BitVec {
                invert: pdrive_invert,
            },
        };
        let ndrive = TileItem {
            bits: ndrive_bits,
            kind: TileItemKind::BitVec {
                invert: ndrive_invert,
            },
        };
        let pterm = TileItem::from_bitvec(pterm_bits, false);
        let nterm = TileItem::from_bitvec(nterm_bits, false);
        ctx.tiledb.insert(tile, bel, "PDRIVE", pdrive);
        ctx.tiledb.insert(tile, bel, "NDRIVE", ndrive);
        ctx.tiledb.insert(tile, bel, "PTERM", pterm);
        ctx.tiledb.insert(tile, bel, "NTERM", nterm);
        present.assert_empty();
        let pslew_bits: Vec<_> = (0..4)
            .map(|j| TileBit::new(0, 0, i * 64 + 32 + j))
            .collect();
        let nslew_bits: Vec<_> = (0..4)
            .map(|j| TileBit::new(0, 0, i * 64 + 36 + j))
            .collect();
        let pslew_invert = bits![0, 0, 1, 0];
        let nslew_invert = bits![0, 0, 1, 0];
        let pslew = TileItem {
            bits: pslew_bits,
            kind: TileItemKind::BitVec {
                invert: pslew_invert.clone(),
            },
        };
        let nslew = TileItem {
            bits: nslew_bits,
            kind: TileItemKind::BitVec {
                invert: nslew_invert.clone(),
            },
        };
        ctx.tiledb.insert(tile, bel, "PSLEW", pslew);
        ctx.tiledb.insert(tile, bel, "NSLEW", nslew);

        ctx.tiledb
            .insert_misc_data("IOSTD:PSLEW:OFF", BitVec::repeat(false, 4));
        ctx.tiledb
            .insert_misc_data("IOSTD:NSLEW:OFF", BitVec::repeat(false, 4));
        ctx.tiledb
            .insert_misc_data("IOSTD:PSLEW:IN_TERM", pslew_invert.clone());
        ctx.tiledb
            .insert_misc_data("IOSTD:NSLEW:IN_TERM", nslew_invert.clone());
        ctx.tiledb
            .insert_misc_data("IOSTD:PDRIVE:OFF", BitVec::repeat(false, 6));
        ctx.tiledb
            .insert_misc_data("IOSTD:NDRIVE:OFF", BitVec::repeat(false, 7));
        ctx.tiledb
            .insert_misc_data("IOSTD:PTERM:OFF", BitVec::repeat(false, 6));
        ctx.tiledb
            .insert_misc_data("IOSTD:NTERM:OFF", BitVec::repeat(false, 7));

        if i == 0 {
            let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "NOTVREF");
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, "PSLEW"),
                &pslew_invert,
                &BitVec::repeat(false, 4),
            );
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, "NSLEW"),
                &nslew_invert,
                &BitVec::repeat(false, 4),
            );
            ctx.tiledb.insert(tile, bel, "VREF", xlat_bit(!diff));
        }

        let (_, _, diff) = Diff::split(
            ctx.state
                .peek_diff(tile, bel, "ISTD", "PCI33_3:3.3:BT")
                .clone(),
            ctx.state
                .peek_diff(tile, bel, "OSTD", "PCI33_3:3.3:BT")
                .clone(),
        );
        ctx.tiledb.insert(tile, bel, "PCI_CLAMP", xlat_bit(diff));

        let mut diff = ctx
            .state
            .peek_diff(tile, bel, "ISTD", "PCI33_3:3.3:BT")
            .combine(&!ctx.state.peek_diff(tile, bel, "ISTD", "MOBILE_DDR:3.3:BT"));
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PCI_CLAMP"), true, false);
        ctx.tiledb.insert(tile, bel, "PCI_INPUT", xlat_bit(diff));

        let diff = ctx.state.get_diff(tile, bel, "VREF_LV", "1");
        ctx.tiledb.insert(tile, bel, "VREF_HV", xlat_bit(!diff));

        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("BYPASS_T", ctx.state.get_diff(tile, bel, "BYPASS_MUX", "T")),
            ("BYPASS_O", ctx.state.get_diff(tile, bel, "BYPASS_MUX", "O")),
            (
                "CMOS_VCCINT",
                ctx.state
                    .peek_diff(tile, bel, "ISTD", "LVCMOS12:3.3:BT")
                    .clone(),
            ),
            (
                "CMOS_VCCO",
                ctx.state
                    .peek_diff(tile, bel, "ISTD", "LVCMOS12_JEDEC:3.3:BT")
                    .clone(),
            ),
            (
                "VREF",
                ctx.state
                    .peek_diff(tile, bel, "ISTD", "SSTL18_I:3.3:BT")
                    .clone(),
            ),
            (
                "DIFF",
                ctx.state
                    .peek_diff(tile, bel, "ISTD", "DIFF_SSTL18_I:3.3:BT")
                    .clone(),
            ),
            (
                "CMOS_VCCAUX",
                ctx.state
                    .peek_diff(tile, bel, "ISTD", "LVTTL:3.3:BT")
                    .clone(),
            ),
        ]);
        ctx.tiledb.insert(tile, bel, "IBUF_MODE", item);
        if i == 1 {
            let diff_lvds = ctx
                .state
                .peek_diff(tile, bel, "OSTD", "LVDS_25:3.3:GROUP0")
                .clone();
            let diff_tmds = ctx
                .state
                .peek_diff(tile, bel, "OSTD", "TMDS_33:3.3:GROUP0")
                .clone();
            let (diff_lvds, diff_tmds, mut diff) = Diff::split(diff_lvds, diff_tmds);
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, "PSLEW"),
                &BitVec::repeat(false, 4),
                &pslew_invert,
            );
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, "NSLEW"),
                &BitVec::repeat(false, 4),
                &nslew_invert,
            );
            ctx.tiledb
                .insert(tile, bel, "DIFF_OUTPUT_ENABLE", xlat_bit(diff));
            ctx.tiledb.insert(
                tile,
                bel,
                "DIFF_MODE",
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    ("LVDS", diff_lvds),
                    ("TMDS", diff_tmds),
                ]),
            );
            let mut diff = ctx.state.get_diff(tile, bel, "DIFF_TERM", "1");
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, "PSLEW"),
                &BitVec::repeat(false, 4),
                &pslew_invert,
            );
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, "NSLEW"),
                &BitVec::repeat(false, 4),
                &nslew_invert,
            );
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DIFF_MODE"), "LVDS", "NONE");
            diff.assert_empty();
        } else {
            let mut diff = ctx.state.get_diff(tile, bel, "DIFF_TERM", "1");
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, "PSLEW"),
                &BitVec::repeat(false, 4),
                &pslew_invert,
            );
            diff.apply_bitvec_diff(
                ctx.tiledb.item(tile, bel, "NSLEW"),
                &BitVec::repeat(false, 4),
                &nslew_invert,
            );
            ctx.tiledb.insert(tile, bel, "DIFF_TERM", xlat_bit(diff));
        }

        for (kind, iostds) in [("LR", IOSTDS_LR), ("BT", IOSTDS_BT)] {
            for vccaux in ["2.5", "3.3"] {
                for std in iostds {
                    if matches!(std.name, "PCI33_3" | "PCI66_3" | "TMDS_33" | "LVPECL_33")
                        && vccaux == "2.5"
                    {
                        continue;
                    }
                    if std.name == "TML_33" {
                        continue;
                    }
                    let mut diff = ctx.state.get_diff(
                        tile,
                        bel,
                        "ISTD",
                        format!("{sn}:{vccaux}:{kind}", sn = std.name),
                    );
                    let val = if std.diff != DiffKind::None {
                        "DIFF"
                    } else if let Some(vref) = std.vref {
                        if vref >= 1250 {
                            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "VREF_HV"), true, false);
                        }
                        "VREF"
                    } else if std.name.starts_with("PCI") {
                        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PCI_INPUT"), true, false);
                        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PCI_CLAMP"), true, false);
                        "CMOS_VCCO"
                    } else if matches!(std.name, "LVCMOS12" | "LVCMOS15" | "LVCMOS18") {
                        "CMOS_VCCINT"
                    } else if matches!(
                        std.name,
                        "LVCMOS12_JEDEC" | "LVCMOS15_JEDEC" | "LVCMOS18_JEDEC" | "MOBILE_DDR"
                    ) || (vccaux == "3.3" && std.name == "LVCMOS25")
                    {
                        "CMOS_VCCO"
                    } else {
                        "CMOS_VCCAUX"
                    };
                    diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IBUF_MODE"), val, "NONE");
                    diff.assert_empty();

                    if std.name == "LVTTL"
                        || std.name.starts_with("LVCMOS")
                        || std.name.contains("HSTL")
                        || std.name.contains("SSTL")
                        || std.name.contains("MOBILE_DDR")
                    {
                        for term in ["UNTUNED_SPLIT_25", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_75"] {
                            let mut diff = ctx.state.get_diff(
                                tile,
                                bel,
                                "IN_TERM",
                                format!("{sn}:{vccaux}:{kind}:{term}", sn = std.name),
                            );
                            diff.apply_bit_diff(
                                ctx.tiledb.item(tile, bel, "OUTPUT_ENABLE"),
                                true,
                                false,
                            );
                            let vcco = std.vcco.unwrap().to_string();
                            let name = format!("{term}.{vcco}.{vccaux}");
                            let val = extract_bitvec_val_part(
                                ctx.tiledb.item(tile, bel, "PTERM"),
                                &BitVec::repeat(false, 6),
                                &mut diff,
                            );
                            ctx.tiledb
                                .insert_misc_data(format!("IOSTD:PTERM:{name}"), val);
                            let val = extract_bitvec_val_part(
                                ctx.tiledb.item(tile, bel, "NTERM"),
                                &BitVec::repeat(false, 7),
                                &mut diff,
                            );
                            ctx.tiledb
                                .insert_misc_data(format!("IOSTD:NTERM:{name}"), val);

                            if std.vcco.unwrap() >= 2500 {
                                diff.assert_empty()
                            } else {
                                ctx.tiledb
                                    .insert(tile, bel, "OUTPUT_LOW_VOLTAGE", xlat_bit(diff));
                            }
                        }
                    }
                }
            }
        }
        for (kind, iostds) in [("LR", IOSTDS_LR), ("BT", IOSTDS_BT)] {
            for vccaux in ["2.5", "3.3"] {
                for std in iostds {
                    let stdname = if std.name == "DIFF_MOBILE_DDR" {
                        std.name
                    } else {
                        std.name.strip_prefix("DIFF_").unwrap_or(std.name)
                    };
                    if std.input_only {
                        continue;
                    }
                    if matches!(std.name, "PCI33_3" | "PCI66_3" | "TMDS_33" | "TML_33")
                        && vccaux == "2.5"
                    {
                        continue;
                    }
                    if std.diff == DiffKind::True {
                        let mut diff0 = ctx.state.get_diff(
                            tile,
                            bel,
                            "OSTD",
                            format!("{sn}:{vccaux}:GROUP0", sn = std.name),
                        );
                        let diff1 = ctx
                            .state
                            .get_diff(
                                tile,
                                bel,
                                "OSTD",
                                format!("{sn}:{vccaux}:GROUP1", sn = std.name),
                            )
                            .combine(&!&diff0);
                        if i == 1 {
                            ctx.tiledb.insert(tile, bel, "LVDS_GROUP", xlat_bit(diff1));
                        } else {
                            diff1.assert_empty();
                        }
                        if i == 1 {
                            diff0.apply_bit_diff(
                                ctx.tiledb.item(tile, bel, "DIFF_OUTPUT_ENABLE"),
                                true,
                                false,
                            );
                            diff0.apply_enum_diff(
                                ctx.tiledb.item(tile, bel, "DIFF_MODE"),
                                if matches!(std.name, "TMDS_33" | "TML_33") {
                                    "TMDS"
                                } else {
                                    "LVDS"
                                },
                                "NONE",
                            );
                        }
                        if std.name == "TML_33" {
                            for (attr, base) in [
                                ("PDRIVE", &BitVec::repeat(false, 6)),
                                ("NDRIVE", &BitVec::repeat(false, 7)),
                                ("PTERM", &BitVec::repeat(false, 6)),
                                ("NTERM", &BitVec::repeat(false, 7)),
                                ("PSLEW", &pslew_invert),
                                ("NSLEW", &nslew_invert),
                            ] {
                                let val = extract_bitvec_val_part(
                                    ctx.tiledb.item(tile, bel, attr),
                                    base,
                                    &mut diff0,
                                );
                                if attr.ends_with("SLEW") {
                                    ctx.tiledb
                                        .insert_misc_data(format!("IOSTD:{attr}:{stdname}"), val);
                                } else {
                                    ctx.tiledb.insert_misc_data(
                                        format!("IOSTD:{attr}:{stdname}.{vccaux}"),
                                        val,
                                    );
                                }
                            }
                            ctx.tiledb.insert(tile, bel, "TML", xlat_bit(diff0));
                        } else {
                            diff0.apply_bitvec_diff(
                                ctx.tiledb.item(tile, bel, "PSLEW"),
                                &BitVec::repeat(false, 4),
                                &pslew_invert,
                            );
                            diff0.apply_bitvec_diff(
                                ctx.tiledb.item(tile, bel, "NSLEW"),
                                &BitVec::repeat(false, 4),
                                &nslew_invert,
                            );
                            diff0.assert_empty();
                        }
                    } else {
                        let (drives, slews) = if std.drive.is_empty() {
                            (&[""][..], &[""][..])
                        } else {
                            (std.drive, &["SLOW", "FAST", "QUIETIO"][..])
                        };
                        for drive in drives {
                            for slew in slews {
                                let val = if drive.is_empty() {
                                    format!("{sn}:{vccaux}:{kind}", sn = std.name)
                                } else {
                                    format!("{sn}:{drive}:{slew}:{vccaux}:{kind}", sn = std.name)
                                };
                                let mut diff = ctx.state.get_diff(tile, bel, "OSTD", val);
                                if let Some(vcco) = std.vcco
                                    && vcco < 2500
                                {
                                    diff.apply_bit_diff(
                                        ctx.tiledb.item(tile, bel, "OUTPUT_LOW_VOLTAGE"),
                                        true,
                                        false,
                                    );
                                }
                                if std.name.starts_with("PCI") {
                                    diff.apply_bit_diff(
                                        ctx.tiledb.item(tile, bel, "PCI_CLAMP"),
                                        true,
                                        false,
                                    );
                                }
                                for (attr, base) in [
                                    ("PDRIVE", BitVec::repeat(false, 6)),
                                    ("NDRIVE", BitVec::repeat(false, 7)),
                                ] {
                                    let val = extract_bitvec_val_part(
                                        ctx.tiledb.item(tile, bel, attr),
                                        &base,
                                        &mut diff,
                                    );
                                    let name = if drive.is_empty() {
                                        format!("{stdname}.{vccaux}")
                                    } else {
                                        format!("{stdname}.{drive}.{vccaux}")
                                    };
                                    ctx.tiledb
                                        .insert_misc_data(format!("IOSTD:{attr}:{name}"), val);
                                }
                                for (attr, base) in
                                    [("PSLEW", &pslew_invert), ("NSLEW", &nslew_invert)]
                                {
                                    let val = extract_bitvec_val_part(
                                        ctx.tiledb.item(tile, bel, attr),
                                        base,
                                        &mut diff,
                                    );
                                    let name = if drive.is_empty() {
                                        stdname.to_string()
                                    } else {
                                        format!("{stdname}.{slew}")
                                    };
                                    ctx.tiledb
                                        .insert_misc_data(format!("IOSTD:{attr}:{name}"), val);
                                }
                                diff.assert_empty();
                            }
                        }
                        if std.name == "LVTTL"
                            || std.name.starts_with("LVCMOS")
                            || std.name.contains("HSTL")
                            || std.name.contains("SSTL")
                            || std.name.contains("MOBILE_DDR")
                        {
                            for term in ["UNTUNED_25", "UNTUNED_50", "UNTUNED_75"] {
                                let val = if std.drive.is_empty() {
                                    format!("{sn}:{term}:{vccaux}:{kind}", sn = std.name)
                                } else {
                                    format!(
                                        "{sn}:{term}:{slew}:{vccaux}:{kind}",
                                        sn = std.name,
                                        slew = slews[0]
                                    )
                                };
                                let mut diff = ctx.state.get_diff(tile, bel, "OSTD", val);
                                if let Some(vcco) = std.vcco
                                    && vcco < 2500
                                {
                                    diff.apply_bit_diff(
                                        ctx.tiledb.item(tile, bel, "OUTPUT_LOW_VOLTAGE"),
                                        true,
                                        false,
                                    );
                                }
                                for (attr, base) in [
                                    ("PDRIVE", BitVec::repeat(false, 6)),
                                    ("NDRIVE", BitVec::repeat(false, 7)),
                                ] {
                                    let val = extract_bitvec_val_part(
                                        ctx.tiledb.item(tile, bel, attr),
                                        &base,
                                        &mut diff,
                                    );
                                    let vcco = std.vcco.unwrap();
                                    let name = format!("{term}.{vcco}.{vccaux}");
                                    ctx.tiledb
                                        .insert_misc_data(format!("IOSTD:{attr}:{name}"), val);
                                }
                                for (attr, base) in
                                    [("PSLEW", &pslew_invert), ("NSLEW", &nslew_invert)]
                                {
                                    let val = extract_bitvec_val_part(
                                        ctx.tiledb.item(tile, bel, attr),
                                        base,
                                        &mut diff,
                                    );
                                    let name = if std.drive.is_empty() {
                                        stdname.to_string()
                                    } else {
                                        format!("{stdname}.SLOW")
                                    };
                                    ctx.tiledb
                                        .insert_misc_data(format!("IOSTD:{attr}:{name}"), val);
                                }
                                diff.assert_empty();
                            }
                        }
                    }
                }
            }
        }
    }
    {
        let tile = "LL";
        let bel = "BANK";
        ctx.tiledb.insert(
            tile,
            bel,
            "LVDSBIAS_0",
            TileItem::from_bitvec(
                (0..12).map(|i| TileBit::new(0, 23, 29 + i)).collect(),
                false,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "LVDSBIAS_1",
            TileItem::from_bitvec(
                (0..12).map(|i| TileBit::new(0, 23, 41 + i)).collect(),
                false,
            ),
        );
    }
    {
        let tile = "LR";
        let bel = "MISC";
        ctx.collect_bit(tile, bel, "GLUTMASK_IOB", "1");
    }
    {
        let tile = "UL";
        let bel = "MISC";
        ctx.collect_bit_wide(tile, bel, "VREF_LV", "1");
        let bel = "BANK";
        ctx.tiledb.insert(
            tile,
            bel,
            "LVDSBIAS_0",
            TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 22, 9),
                    TileBit::new(0, 22, 21),
                    TileBit::new(0, 22, 20),
                    TileBit::new(0, 22, 19),
                    TileBit::new(0, 22, 18),
                    TileBit::new(0, 22, 17),
                    TileBit::new(0, 22, 16),
                    TileBit::new(0, 22, 15),
                    TileBit::new(0, 22, 14),
                    TileBit::new(0, 22, 13),
                    TileBit::new(0, 22, 12),
                    TileBit::new(0, 22, 11),
                ],
                false,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "LVDSBIAS_1",
            TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 22, 10),
                    TileBit::new(0, 22, 27),
                    TileBit::new(0, 22, 26),
                    TileBit::new(0, 22, 25),
                    TileBit::new(0, 22, 24),
                    TileBit::new(0, 22, 23),
                    TileBit::new(0, 22, 22),
                    TileBit::new(0, 22, 32),
                    TileBit::new(0, 22, 31),
                    TileBit::new(0, 22, 30),
                    TileBit::new(0, 22, 29),
                    TileBit::new(0, 22, 28),
                ],
                false,
            ),
        );
    }
    for tile in ["LL", "UL"] {
        let bel = "BANK";
        for std in IOSTDS_BT {
            if std.diff != DiffKind::True {
                continue;
            }
            for attr in ["LVDSBIAS_0", "LVDSBIAS_1"] {
                let diff = ctx.state.get_diff(tile, bel, attr, std.name);
                let val = extract_bitvec_val(
                    ctx.tiledb.item(tile, bel, attr),
                    &BitVec::repeat(false, 12),
                    diff,
                );
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:LVDSBIAS:{}", std.name), val);
            }
        }
    }
    for (tile, bank, bit_25, bit_75) in [
        ("LL", 2, TileBit::new(0, 23, 27), TileBit::new(0, 23, 28)),
        ("LL", 3, TileBit::new(0, 23, 24), TileBit::new(0, 23, 25)),
        ("UL", 0, TileBit::new(0, 22, 43), TileBit::new(0, 22, 42)),
        ("UL", 4, TileBit::new(0, 22, 46), TileBit::new(0, 22, 45)),
        ("LR", 1, TileBit::new(0, 22, 52), TileBit::new(0, 22, 53)),
        ("UR", 5, TileBit::new(1, 22, 51), TileBit::new(1, 22, 52)),
    ] {
        let bel = &format!("OCT_CAL{bank}");
        let item = TileItem {
            bits: vec![bit_25, bit_75],
            kind: TileItemKind::Enum {
                values: [
                    ("NONE".to_string(), bits![0, 0]),
                    ("0.25".to_string(), bits![1, 0]),
                    ("0.75".to_string(), bits![0, 1]),
                    ("0.5".to_string(), bits![1, 1]),
                ]
                .into_iter()
                .collect(),
            },
        };
        if bank < 4 || edev.chip.row_mcb_split.is_some() {
            let mut diff = ctx.state.get_diff(tile, bel, "INTERNAL_VREF", "1");
            diff.apply_enum_diff(&item, "0.5", "NONE");
            diff.assert_empty();
        }
        ctx.tiledb.insert(tile, bel, "VREF_VALUE", item);
    }
}
