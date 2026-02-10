use std::collections::HashSet;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, BelSlotId, TableRowId, WireSlotIdExt},
    dir::DirV,
    grid::{CellCoord, DieId, TileCoord},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, SpecialId, extract_bitvec_val, extract_bitvec_val_part, xlat_bit,
    xlat_bit_wide, xlat_bitvec, xlat_enum_attr, xlat_enum_raw,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{ExpandedBond, ExpandedDevice};
use prjcombine_spartan6::defs::{self, bcls, bslots, enums, tables, tcls, tslots, wires};
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{RectBitId, TileBit},
};

use crate::{
    backend::{IseBackend, Key, MultiValue, Value},
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
    spartan6::specials,
};

const IOSTDS_WE: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS12", 1200, &[2, 4, 6, 8, 12]),
    Iostd::cmos("LVCMOS18_JEDEC", 1800, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS15_JEDEC", 1500, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS12_JEDEC", 1200, &[2, 4, 6, 8, 12]),
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

const IOSTDS_SN: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8]),
    Iostd::cmos("LVCMOS12", 1200, &[2, 4, 6]),
    Iostd::cmos("LVCMOS18_JEDEC", 1800, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS15_JEDEC", 1500, &[2, 4, 6, 8]),
    Iostd::cmos("LVCMOS12_JEDEC", 1200, &[2, 4, 6]),
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
struct AllMcbIoi(SpecialId);

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
            let ntcrd = tcrd.with_row(row).tile(tslots::BEL);
            if let Some(tile) = backend.edev.get_tile(ntcrd)
                && tile.class == tcls::IOI_WE
            {
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::BelSpecial(tcls::IOI_WE, bslots::MISC_IOI, self.0),
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

fn iostd_to_row(
    edev: &prjcombine_spartan6::expanded::ExpandedDevice,
    std: &Iostd,
    drive: u8,
) -> (TableRowId, SpecialId) {
    let mut name = std.name;
    let mut spec = specials::IOB_STD_PLAIN;
    if let Some(n) = name.strip_suffix("_JEDEC") {
        name = n;
        spec = specials::IOB_STD_JEDEC;
    }
    if let Some(n) = name.strip_prefix("DIFF_")
        && name != "DIFF_MOBILE_DDR"
    {
        name = n;
        spec = specials::IOB_STD_PSEUDO_DIFF;
    }

    let row = if std.diff == DiffKind::True {
        spec = specials::IOB_STD_TRUE_DIFF;
        edev.db[tables::LVDSBIAS].rows.get(std.name).unwrap().0
    } else if name == "LVPECL_25" {
        spec = specials::IOB_STD_LVPECL;
        tables::LVDSBIAS::LVDS_25
    } else if name == "LVPECL_33" {
        spec = specials::IOB_STD_LVPECL;
        tables::LVDSBIAS::LVDS_33
    } else if name.starts_with("LVCMOS") || name == "LVTTL" {
        edev.db[tables::IOB_DATA]
            .rows
            .get(&format!("{name}_{drive}"))
            .unwrap()
            .0
    } else {
        edev.db[tables::IOB_DATA].rows.get(name).unwrap().0
    };
    (row, spec)
}

fn iostd_slew_to_row(
    edev: &prjcombine_spartan6::expanded::ExpandedDevice,
    std: &Iostd,
    slew: SpecialId,
) -> TableRowId {
    let mut name = std.name;
    if let Some(n) = name.strip_prefix("DIFF_")
        && name != "DIFF_MOBILE_DDR"
    {
        name = n;
    }
    if name.starts_with("LVCMOS") || name == "LVTTL" {
        match slew {
            specials::IOB_SLEW_SLOW => tables::IOB_DATA::SLEW_SLOW,
            specials::IOB_SLEW_FAST => tables::IOB_DATA::SLEW_FAST,
            specials::IOB_SLEW_QUIETIO => tables::IOB_DATA::SLEW_QUIETIO,
            _ => unreachable!(),
        }
    } else {
        edev.db[tables::IOB_DATA].rows.get(name).unwrap().0
    }
}

fn iostd_to_lvdsbias_row(
    edev: &prjcombine_spartan6::expanded::ExpandedDevice,
    std: &Iostd,
) -> TableRowId {
    edev.db[tables::LVDSBIAS].rows.get(std.name).unwrap().0
}

fn term_to_row(
    edev: &prjcombine_spartan6::expanded::ExpandedDevice,
    name: &str,
    vcco: u16,
) -> TableRowId {
    let a = vcco / 1000;
    let b = vcco / 100 % 10;
    edev.db[tables::IOB_TERM]
        .rows
        .get(&format!("{name}_{a}V{b}"))
        .unwrap()
        .0
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
    for tcid in [tcls::IOI_WE, tcls::IOI_SN] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        for i in 0..2 {
            let mut bctx = ctx.bel(defs::bslots::ILOGIC[i]);
            let bel_other = defs::bslots::ILOGIC[i ^ 1];
            let bel_ologic = defs::bslots::OLOGIC[i];
            let bel_ioiclk = defs::bslots::IOI_DDR[i];
            for (spec, mode) in [
                (specials::IOI_ILOGIC_ILOGIC2, "ILOGIC2"),
                (specials::IOI_ILOGIC_ISERDES2, "ISERDES2"),
            ] {
                bctx.build()
                    .tile_mutex("CLK", "TEST_LOGIC")
                    .global("GLUTMASK", "NO")
                    .bel_unused(bel_other)
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .test_bel_special(spec)
                    .mode(mode)
                    .commit();
            }
            for (spec, val) in [
                (specials::IOI_ILOGIC_IFFTYPE_LATCH, "#LATCH"),
                (specials::IOI_ILOGIC_IFFTYPE_FF, "#FF"),
                (specials::IOI_ILOGIC_IFFTYPE_DDR, "DDR"),
            ] {
                bctx.mode("ILOGIC2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "NOPE")
                    .test_bel_special(spec)
                    .attr("IFFTYPE", val)
                    .commit();
            }
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .attr("FABRICOUTUSED", "0")
                .pin("TFB")
                .pin("FABRICOUT")
                .test_bel_attr_rename("D2OBYP_SEL", bcls::ILOGIC::MUX_TSBYPASS);
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .bel_unused(bel_other)
                .tile_mutex("CLK", "NOPE")
                .attr("FABRICOUTUSED", "0")
                .attr("IFFTYPE", "#FF")
                .attr("D2OBYP_SEL", "GND")
                .pin("OFB")
                .pin("D")
                .pin("DDLY")
                .test_bel_attr_bool_rename("IMUX", bcls::ILOGIC::I_DELAY_ENABLE, "1", "0");
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .bel_unused(bel_other)
                .tile_mutex("CLK", "NOPE")
                .attr("FABRICOUTUSED", "0")
                .attr("IFFTYPE", "#FF")
                .attr("D2OBYP_SEL", "GND")
                .pin("OFB")
                .pin("D")
                .pin("DDLY")
                .test_bel_attr_bool_rename("IFFMUX", bcls::ILOGIC::FFI_DELAY_ENABLE, "1", "0");
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bool_rename("SRINIT_Q", bcls::ILOGIC::FFI_INIT, "0", "1");
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bool_rename("SRTYPE_Q", bcls::ILOGIC::FFI_SR_SYNC, "ASYNC", "SYNC");
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .pin("SR")
                .attr("IFFTYPE", "#FF")
                .test_bel_attr_bits(bcls::ILOGIC::FFI_SR_ENABLE)
                .attr("SRUSED", "0")
                .commit();
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .pin("REV")
                .attr("IFFTYPE", "#FF")
                .test_bel_attr_bits(bcls::ILOGIC::FFI_REV_ENABLE)
                .attr("REVUSED", "0")
                .commit();
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .pin("CE0")
                .attr("IFFTYPE", "#FF")
                .test_bel_attr_bits(bcls::ILOGIC::FFI_CE_ENABLE)
                .pin_pips("CE0")
                .commit();

            bctx.mode("ISERDES2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_rename("DATA_WIDTH", bcls::ILOGIC::DATA_WIDTH_START);
            bctx.mode("ISERDES2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bool_auto(bcls::ILOGIC::BITSLIP_ENABLE, "FALSE", "TRUE");
            bctx.mode("ISERDES2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_default_rename(
                    "INTERFACE_TYPE",
                    bcls::ILOGIC::MUX_Q1,
                    enums::ILOGIC_MUX_Q::SHIFT_REGISTER,
                );
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.SR", "INT")
                .test_bel_attr_val(bcls::ILOGIC::MUX_SR, enums::ILOGIC_MUX_SR::INT)
                .pip("SR_MUXED", "SR")
                .commit();
            bctx.mode("ILOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.SR", "OLOGIC_SR")
                .test_bel_attr_val(bcls::ILOGIC::MUX_SR, enums::ILOGIC_MUX_SR::OLOGIC_SR)
                .pip("SR_MUXED", (PinFar, bel_ologic, "SR"))
                .commit();

            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_LOGIC")
                .test_routing(
                    wires::IMUX_ILOGIC_CLK[i].cell(0),
                    wires::IOI_ICLK[i].cell(0).pos(),
                )
                .pip("CLK0", (bel_ioiclk, "CLK0_ILOGIC"))
                .commit();
            bctx.mode("ISERDES2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_LOGIC")
                .bel_mode(bel_other, "ISERDES2")
                .pin("D")
                .bel_pin(bel_other, "D")
                .test_bel_attr_bits(bcls::ILOGIC::IOCE_ENABLE)
                .pip("IOCE", (bel_ioiclk, "IOCE0"))
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_LOGIC")
                .unused()
                .bel_unused(bel_other)
                .test_bel_attr_bits(bcls::ILOGIC::ENABLE)
                .pip("IOCE", (bel_ioiclk, "IOCE0"))
                .commit();
            if i == 0 {
                bctx.build()
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .test_bel_attr_val(bcls::ILOGIC::MUX_D, enums::ILOGIC_MUX_D::OTHER_IOB_I)
                    .pip("D_MUX", (bel_other, "IOB_I"))
                    .commit();
            }
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(defs::bslots::OLOGIC[i]);
            let bel_iodelay = defs::bslots::IODELAY[i];
            let bel_ioiclk = defs::bslots::IOI_DDR[i];
            let bel_ioi = defs::bslots::MISC_IOI;
            for (spec, mode) in [
                (specials::IOI_OLOGIC_OLOGIC2, "OLOGIC2"),
                (specials::IOI_OLOGIC_OSERDES2, "OSERDES2"),
            ] {
                bctx.build()
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .global("ENABLEMISR", "N")
                    .tile_mutex("CLK", "TEST_LOGIC")
                    .global("GLUTMASK", "NO")
                    .test_bel_special(spec)
                    .mode(mode)
                    .commit();
            }
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global("ENABLEMISR", "Y")
                .global("MISRRESET", "Y")
                .tile_mutex("CLK", "TEST_LOGIC")
                .global("GLUTMASK", "NO")
                .test_bel_attr_bits(bcls::OLOGIC::MISR_RESET)
                .mode("OLOGIC2")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bool_rename("SRINIT_OQ", bcls::OLOGIC::FFO_INIT, "0", "1");
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bool_rename("SRINIT_TQ", bcls::OLOGIC::FFT_INIT, "0", "1");
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .pin("SR")
                .test_bel_attr_bool_rename("SRTYPE_OQ", bcls::OLOGIC::FFO_SR_SYNC, "ASYNC", "SYNC");
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .pin("SR")
                .test_bel_attr_bool_rename("SRTYPE_TQ", bcls::OLOGIC::FFT_SR_SYNC, "ASYNC", "SYNC");
            for (val, vname) in [
                (false, "1"),
                (false, "2"),
                (false, "3"),
                (false, "4"),
                (true, "5"),
                (true, "6"),
                (true, "7"),
                (true, "8"),
            ] {
                bctx.mode("OSERDES2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .test_bel_attr_bits_bi(bcls::OLOGIC::CASCADE_ENABLE, val)
                    .attr("DATA_WIDTH", vname)
                    .commit();
            }
            for (spec, val) in [
                (specials::IOI_OLOGIC_BYPASS_GCLK_FF_FALSE, "FALSE"),
                (specials::IOI_OLOGIC_BYPASS_GCLK_FF_TRUE, "TRUE"),
            ] {
                bctx.mode("OSERDES2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .test_bel_special(spec)
                    .attr("BYPASS_GCLK_FF", val)
                    .commit();
            }
            {
                let mut builder = bctx
                    .mode("OSERDES2")
                    .has_related(Delta::new(0, 0, tcls::IOB));
                if i == 1 {
                    builder = builder.null_bits();
                }
                builder.test_bel_attr_auto(bcls::OLOGIC::OUTPUT_MODE);
            }
            for (attr, aname) in [
                (bcls::OLOGIC::FFO_SR_ENABLE, "OSRUSED"),
                (bcls::OLOGIC::FFT_SR_ENABLE, "TSRUSED"),
                (bcls::OLOGIC::FFO_REV_ENABLE, "OREVUSED"),
                (bcls::OLOGIC::FFT_REV_ENABLE, "TREVUSED"),
            ] {
                bctx.mode("OLOGIC2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .attr("OUTFFTYPE", "#FF")
                    .attr("TFFTYPE", "#FF")
                    .pin("SR")
                    .pin("REV")
                    .test_bel_attr_bits(attr)
                    .attr(aname, "0")
                    .commit();
            }
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.SR", "INT")
                .test_bel_attr_val(bcls::OLOGIC::MUX_SR, enums::OLOGIC_MUX_SR::INT)
                .pin_pips("SR")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.REV", "INT")
                .test_bel_attr_val(bcls::OLOGIC::MUX_REV, enums::OLOGIC_MUX_REV::INT)
                .pin_pips("REV")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.OCE", "INT")
                .attr("OUTFFTYPE", "#FF")
                .test_bel_attr_val(bcls::OLOGIC::MUX_OCE, enums::OLOGIC_MUX_OCE::INT)
                .pin_pips("OCE")
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.OCE", "PCI_CE")
                .attr("OUTFFTYPE", "#FF")
                .test_bel_attr_val(bcls::OLOGIC::MUX_OCE, enums::OLOGIC_MUX_OCE::PCI_CE)
                .pip("OCE", (bel_ioi, "PCI_CE"))
                .commit();
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.TCE", "INT")
                .attr("TFFTYPE", "#FF")
                .test_bel_special(specials::IOI_OLOGIC_TCE)
                .pin_pips("TCE")
                .commit();
            bctx.mode("OSERDES2")
                .global_mutex("DRPSDO", "NOPE")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.TRAIN", "MCB")
                .test_bel_attr_val(bcls::OLOGIC::MUX_TRAIN, enums::OLOGIC_MUX_TRAIN::MCB)
                .pip("TRAIN", (bel_ioi, "MCB_DRPTRAIN"))
                .commit();
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .mutex("MUX.TRAIN", "INT")
                .test_bel_attr_val(bcls::OLOGIC::MUX_TRAIN, enums::OLOGIC_MUX_TRAIN::INT)
                .pin_pips("TRAIN")
                .commit();
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_multi(bcls::OLOGIC::TRAIN_PATTERN, MultiValue::Dec(0));
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global_mutex("DRPSDO", "USE")
                .pip((bel_iodelay, "CE"), (bel_ioi, "MCB_DRPSDO"))
                .test_bel_attr_val(bcls::OLOGIC::MUX_IN_O, enums::OLOGIC_MUX_IN::MCB)
                .pip("D1", "MCB_D1")
                .commit();
            for (spec, val) in [
                (specials::IOI_OLOGIC_OUTFFTYPE_LATCH, "#LATCH"),
                (specials::IOI_OLOGIC_OUTFFTYPE_FF, "#FF"),
                (specials::IOI_OLOGIC_OUTFFTYPE_DDR, "DDR"),
            ] {
                bctx.mode("OLOGIC2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "NOPE")
                    .attr("TFFTYPE", "")
                    .test_bel_special(spec)
                    .attr("OUTFFTYPE", val)
                    .commit();
            }
            for (spec, val) in [
                (specials::IOI_OLOGIC_TFFTYPE_LATCH, "#LATCH"),
                (specials::IOI_OLOGIC_TFFTYPE_FF, "#FF"),
                (specials::IOI_OLOGIC_TFFTYPE_DDR, "DDR"),
            ] {
                bctx.mode("OLOGIC2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "NOPE")
                    .attr("OUTFFTYPE", "")
                    .test_bel_special(spec)
                    .attr("TFFTYPE", val)
                    .commit();
            }
            for (val, vname) in [
                (enums::OLOGIC_MUX_O::D1, "D1"),
                (enums::OLOGIC_MUX_O::FFO, "OUTFF"),
            ] {
                bctx.mode("OLOGIC2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "NOPE")
                    .attr("OUTFFTYPE", "#FF")
                    .attr("D1USED", "0")
                    .attr("O1USED", "0")
                    .pin("D1")
                    .pin("OQ")
                    .test_bel_attr_val(bcls::OLOGIC::MUX_O, val)
                    .attr("OMUX", vname)
                    .commit();
            }
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "NOPE")
                .attr("OUTFFTYPE", "")
                .attr("TFFTYPE", "")
                .attr("T1USED", "0")
                .pin("T1")
                .pin("TQ")
                .test_bel_attr_val(bcls::OLOGIC::MUX_T, enums::OLOGIC_MUX_T::T1)
                .attr("OT1USED", "0")
                .commit();
            for (spec, val) in [
                (specials::IOI_OLOGIC_DDR_ALIGNMENT_NONE, "NONE"),
                (specials::IOI_OLOGIC_DDR_ALIGNMENT_C0, "C0"),
            ] {
                bctx.mode("OLOGIC2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "NOPE")
                    .attr("OUTFFTYPE", "DDR")
                    .attr("TDDR_ALIGNMENT", "")
                    .test_bel_special(spec)
                    .attr("DDR_ALIGNMENT", val)
                    .commit();
            }
            for (spec, val) in [
                (specials::IOI_OLOGIC_TDDR_ALIGNMENT_NONE, "NONE"),
                (specials::IOI_OLOGIC_TDDR_ALIGNMENT_C0, "C0"),
            ] {
                bctx.mode("OLOGIC2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "NOPE")
                    .attr("TFFTYPE", "DDR")
                    .attr("DDR_ALIGNMENT", "")
                    .test_bel_special(spec)
                    .attr("TDDR_ALIGNMENT", val)
                    .commit();
            }
            bctx.mode("OLOGIC2")
                .has_related(Delta::new(0, 0, tcls::IOB))
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
                .test_bel_attr_bits(bcls::OLOGIC::MISR_ENABLE_DATA)
                .attr("MISRATTRBOX", "MISR_ENABLE_DATA")
                .commit();

            for val in ["CLK0", "CLK1"] {
                bctx.mode("OLOGIC2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .global("ENABLEMISR", "Y")
                    .test_bel_attr_bits(bcls::OLOGIC::MISR_ENABLE_CLK)
                    .attr("MISR_ENABLE_CLK", val)
                    .commit();
            }

            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_LOGIC")
                .test_routing(
                    wires::IMUX_OLOGIC_CLK[i].cell(0),
                    wires::IOI_OCLK[i].cell(0).pos(),
                )
                .pip("CLK0", (bel_ioiclk, "CLK0_OLOGIC"))
                .commit();
            bctx.mode("OSERDES2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_LOGIC")
                .test_bel_attr_bits(bcls::OLOGIC::IOCE_ENABLE)
                .pip("IOCE", (bel_ioiclk, "IOCE1"))
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_LOGIC")
                .unused()
                .test_bel_attr_bits(bcls::OLOGIC::ENABLE)
                .pip("IOCE", (bel_ioiclk, "IOCE1"))
                .commit();
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(defs::bslots::IODELAY[i]);
            let bel_other = defs::bslots::IODELAY[i ^ 1];
            let bel_ilogic = defs::bslots::ILOGIC[i];
            let bel_ologic = defs::bslots::OLOGIC[i];
            let bel_ioiclk = defs::bslots::IOI_DDR[i];
            for (spec, mode) in [
                (specials::IOI_IODELAY_IODELAY2, "IODELAY2"),
                (specials::IOI_IODELAY_IODRP2, "IODRP2"),
                (specials::IOI_IODELAY_IODRP2_MCB, "IODRP2_MCB"),
            ] {
                bctx.build()
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .global_mutex("DRPSDO", "NOPE")
                    .global("GLUTMASK", "NO")
                    .global("IOI_TESTPCOUNTER", "NO")
                    .global("IOI_TESTNCOUNTER", "NO")
                    .global("IOIENFFSCAN_DRP", "NO")
                    .bel_unused(bel_other)
                    .test_bel_special(spec)
                    .mode(mode)
                    .commit();
            }
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global("GLUTMASK", "NO")
                .global("IOI_TESTPCOUNTER", "YES")
                .global("IOI_TESTNCOUNTER", "NO")
                .global("IOIENFFSCAN_DRP", "NO")
                .bel_unused(bel_other)
                .test_bel_special(specials::IOI_IODELAY_IODELAY2_TEST_PCOUNTER)
                .mode("IODELAY2")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global("GLUTMASK", "NO")
                .global("IOI_TESTPCOUNTER", "NO")
                .global("IOI_TESTNCOUNTER", "YES")
                .global("IOIENFFSCAN_DRP", "NO")
                .bel_unused(bel_other)
                .test_bel_special(specials::IOI_IODELAY_IODELAY2_TEST_NCOUNTER)
                .mode("IODELAY2")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global("GLUTMASK", "NO")
                .global("IOI_TESTPCOUNTER", "NO")
                .global("IOI_TESTNCOUNTER", "NO")
                .global("IOIENFFSCAN_DRP", "YES")
                .bel_unused(bel_other)
                .test_bel_special(specials::IOI_IODELAY_IODRP2_IOIENFFSCAN_DRP)
                .mode("IODRP2")
                .commit();

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bits(bcls::IODELAY::ODELAY_VALUE_P)
                .multi_attr("ODELAY_VALUE", MultiValue::Dec(0), 8);
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .attr("IDELAY_TYPE", "FIXED")
                .attr("IDELAY_MODE", "PCI")
                .test_bel_attr_bits(bcls::IODELAY::IDELAY_VALUE_P)
                .multi_attr("IDELAY_VALUE", MultiValue::Dec(0), 8);
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .attr("IDELAY_TYPE", "FIXED")
                .attr("IDELAY_MODE", "PCI")
                .test_bel_attr_bits(bcls::IODELAY::IDELAY_VALUE_N)
                .multi_attr("IDELAY2_VALUE", MultiValue::Dec(0), 8);
            bctx.mode("IODRP2_MCB")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global_mutex("DRPSDO", "NOPE")
                .test_bel_special_bits(specials::IOI_IODELAY_MCB_ADDRESS)
                .multi_attr("MCB_ADDRESS", MultiValue::Dec(0), 4);
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .pin("CIN")
                .test_bel_attr_bits(bcls::IODELAY::CIN_ENABLE)
                .pin_pips("CIN")
                .commit();

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bool_auto(bcls::IODELAY::TEST_GLITCH_FILTER, "FALSE", "TRUE");

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_auto(bcls::IODELAY::COUNTER_WRAPAROUND);

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_auto(bcls::IODELAY::IODELAY_CHANGE);

            for (val, spec, spec_dpd) in [
                (
                    "FIXED",
                    specials::IOI_IODELAY_FIXED,
                    specials::IOI_IODELAY_DPD_FIXED,
                ),
                (
                    "DEFAULT",
                    specials::IOI_IODELAY_DEFAULT,
                    specials::IOI_IODELAY_DPD_DEFAULT,
                ),
                (
                    "VARIABLE_FROM_ZERO",
                    specials::IOI_IODELAY_VARIABLE_FROM_ZERO,
                    specials::IOI_IODELAY_DPD_VARIABLE_FROM_ZERO,
                ),
                (
                    "VARIABLE_FROM_HALF_MAX",
                    specials::IOI_IODELAY_VARIABLE_FROM_HALF_MAX,
                    specials::IOI_IODELAY_DPD_VARIABLE_FROM_HALF_MAX,
                ),
                (
                    "DIFF_PHASE_DETECTOR",
                    specials::IOI_IODELAY_DIFF_PHASE_DETECTOR,
                    specials::IOI_IODELAY_DPD_DIFF_PHASE_DETECTOR,
                ),
            ] {
                bctx.mode("IODELAY2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .bel_unused(bel_other)
                    .test_bel_special(spec)
                    .attr("IDELAY_TYPE", val)
                    .commit();
                bctx.mode("IODELAY2")
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .bel_mode(bel_other, "IODELAY2")
                    .bel_attr(bel_other, "IDELAY_TYPE", "DIFF_PHASE_DETECTOR")
                    .test_bel_special(spec_dpd)
                    .attr("IDELAY_TYPE", val)
                    .commit();
            }

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bits(bcls::IODELAY::ODATAIN_ENABLE)
                .pip("ODATAIN", (bel_ologic, "OQ"))
                .commit();

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "IODELAY")
                .mutex("MUX.IOCLK", "ILOGIC_CLK")
                .pip((bel_ilogic, "CLK0"), (bel_ioiclk, "CLK0_ILOGIC"))
                .test_routing(
                    wires::IMUX_IODELAY_IOCLK[i].cell(0),
                    wires::IMUX_ILOGIC_CLK[i].cell(0).pos(),
                )
                .pip("IOCLK0", (bel_ioiclk, "CLK0_ILOGIC"))
                .commit();
            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "IODELAY")
                .mutex("MUX.IOCLK", "OLOGIC_CLK")
                .pip((bel_ologic, "CLK0"), (bel_ioiclk, "CLK0_OLOGIC"))
                .test_routing(
                    wires::IMUX_IODELAY_IOCLK[i].cell(0),
                    wires::IMUX_OLOGIC_CLK[i].cell(0).pos(),
                )
                .pip("IOCLK0", (bel_ioiclk, "CLK0_OLOGIC"))
                .commit();

            bctx.mode("IODRP2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .attr("IDELAY_MODE", "NORMAL")
                .test_bel_attr_auto(bcls::IODELAY::DELAY_SRC);

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_auto(bcls::IODELAY::IDELAY_MODE);

            bctx.mode("IODELAY2")
                .has_related(Delta::new(0, 0, tcls::IOB))
                .test_bel_attr_bool_auto(bcls::IODELAY::DELAYCHAIN_OSC, "FALSE", "TRUE");
        }
        for i in 0..2 {
            let mut bctx = ctx.bel(defs::bslots::IOI_DDR[i]);
            let bel_ilogic = defs::bslots::ILOGIC[i];
            let bel_ologic = defs::bslots::OLOGIC[i];
            let bel_ioi = defs::bslots::MISC_IOI;
            for (j, pin, wire) in [
                (0, "CKINT0", wires::IMUX_CLK[i ^ 1]),
                (0, "CKINT1", wires::IMUX_GFAN[i ^ 1]),
                (1, "CKINT0", wires::IMUX_CLK[i ^ 1]),
                (1, "CKINT1", wires::IMUX_GFAN[i ^ 1]),
            ] {
                bctx.build()
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .mutex(format!("MUX.CLK{j}"), pin)
                    .tile_mutex("CLK", "TEST_INTER")
                    .test_routing(wires::IOI_IOCLK[i * 3 + j].cell(0), wire.cell(0).pos())
                    .pip(format!("CLK{j}INTER"), pin)
                    .commit();
            }
            for (j, pin, wire) in [
                (0, "IOCLK0", wires::IOCLK[0]),
                (0, "IOCLK2", wires::IOCLK[2]),
                (0, "PLLCLK0", wires::PLLCLK[0]),
                (1, "IOCLK1", wires::IOCLK[1]),
                (1, "IOCLK3", wires::IOCLK[3]),
                (1, "PLLCLK1", wires::PLLCLK[1]),
                (2, "PLLCLK0", wires::PLLCLK[0]),
                (2, "PLLCLK1", wires::PLLCLK[1]),
            ] {
                bctx.build()
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .mutex(format!("MUX.CLK{j}"), pin)
                    .tile_mutex("CLK", "TEST_INTER")
                    .test_routing(wires::IOI_IOCLK[i * 3 + j].cell(0), wire.cell(0).pos())
                    .pip(format!("CLK{j}INTER"), (bel_ioi, pin))
                    .commit();
            }
            for j in 0..3 {
                bctx.build()
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "TEST_INV")
                    .pip("CLK0_ILOGIC", format!("CLK{j}INTER"))
                    .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                    .bel_mode(bel_ilogic, "ISERDES2")
                    .bel_attr(bel_ilogic, "DATA_RATE", "SDR")
                    .bel_pin(bel_ilogic, "CLK0")
                    .test_raw(DiffKey::RoutingInv(
                        tcid,
                        wires::IOI_IOCLK_OPTINV[i * 3 + j].cell(0),
                        true,
                    ))
                    .bel_attr(bel_ilogic, "CLK0INV", "CLK0_B")
                    .commit();
            }
            for j in 0..3 {
                bctx.build()
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "TEST_CLK")
                    .mutex("MUX.ICLK", format!("CLK{j}"))
                    .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                    .bel_mode(bel_ilogic, "ISERDES2")
                    .bel_attr(bel_ilogic, "DATA_RATE", "SDR")
                    .bel_pin(bel_ilogic, "CLK0")
                    .test_routing(
                        wires::IOI_ICLK[i].cell(0),
                        wires::IOI_IOCLK[i * 3 + j].cell(0).pos(),
                    )
                    .pip("CLK0_ILOGIC", format!("CLK{j}INTER"))
                    .commit();
                bctx.build()
                    .has_related(Delta::new(0, 0, tcls::IOB))
                    .tile_mutex("CLK", "TEST_CLK")
                    .mutex("MUX.OCLK", format!("CLK{j}"))
                    .pip((bel_ologic, "CLK0"), "CLK0_OLOGIC")
                    .bel_mode(bel_ologic, "OSERDES2")
                    .bel_attr(bel_ologic, "DATA_RATE_OQ", "SDR")
                    .bel_pin(bel_ologic, "CLK0")
                    .test_routing(
                        wires::IOI_OCLK[i].cell(0),
                        wires::IOI_IOCLK[i * 3 + j].cell(0).pos(),
                    )
                    .pip("CLK0_OLOGIC", format!("CLK{j}INTER"))
                    .commit();
            }
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_ICLK_DDR")
                .mutex("MUX.ICLK", "DDR")
                .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                .bel_mode(bel_ilogic, "ISERDES2")
                .bel_attr(bel_ilogic, "DATA_RATE", "DDR")
                .bel_pin(bel_ilogic, "CLK0")
                .test_routing(
                    wires::IOI_ICLK[i].cell(0),
                    wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                )
                .pip("CLK0_ILOGIC", "CLK0INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_OCLK_DDR")
                .mutex("MUX.OCLK", "DDR")
                .pip((bel_ologic, "CLK0"), "CLK0_OLOGIC")
                .bel_mode(bel_ologic, "OSERDES2")
                .bel_attr(bel_ologic, "DATA_RATE_OQ", "DDR")
                .bel_pin(bel_ologic, "CLK0")
                .test_routing(
                    wires::IOI_OCLK[i].cell(0),
                    wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                )
                .pip("CLK0_OLOGIC", "CLK0INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_ICLK_DDR")
                .mutex("MUX.ICLK", "DDR")
                .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                .bel_mode(bel_ilogic, "ILOGIC2")
                .bel_attr(bel_ilogic, "IFFTYPE", "DDR")
                .bel_attr(bel_ilogic, "DDR_ALIGNMENT", "")
                .bel_pin(bel_ilogic, "CLK0")
                .test_routing_pair_special(
                    wires::IOI_ICLK[i].cell(0),
                    wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                    specials::IOI_ILOGIC_DDR,
                )
                .pip("CLK0_ILOGIC", "CLK0INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_ICLK_DDR_C0")
                .mutex("MUX.ICLK", "DDR")
                .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                .bel_mode(bel_ilogic, "ILOGIC2")
                .bel_attr(bel_ilogic, "IFFTYPE", "DDR")
                .bel_attr(bel_ilogic, "DDR_ALIGNMENT", "C0")
                .bel_pin(bel_ilogic, "CLK0")
                .test_routing_pair_special(
                    wires::IOI_ICLK[i].cell(0),
                    wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                    specials::IOI_ILOGIC_DDR_C0,
                )
                .pip("CLK0_ILOGIC", "CLK0INTER")
                .pip("CLK1", "CLK1INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_ICLK_DDR_C1")
                .mutex("MUX.ICLK", "DDR")
                .pip((bel_ilogic, "CLK0"), "CLK0_ILOGIC")
                .bel_mode(bel_ilogic, "ILOGIC2")
                .bel_attr(bel_ilogic, "IFFTYPE", "DDR")
                .bel_attr(bel_ilogic, "DDR_ALIGNMENT", "C0")
                .bel_pin(bel_ilogic, "CLK0")
                .test_routing_pair_special(
                    wires::IOI_ICLK[i].cell(0),
                    wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                    specials::IOI_ILOGIC_DDR_C1,
                )
                .pip("CLK0_ILOGIC", "CLK1INTER")
                .pip("CLK1", "CLK0INTER")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .tile_mutex("CLK", "TEST_OCLK_DDR")
                .mutex("MUX.OCLK", "DDR")
                .pip((bel_ologic, "CLK0"), "CLK0_OLOGIC")
                .bel_mode(bel_ologic, "OLOGIC2")
                .bel_attr(bel_ologic, "OUTFFTYPE", "DDR")
                .bel_attr(bel_ologic, "TFFTYPE", "DDR")
                .bel_attr(bel_ologic, "ODDR_ALIGNMENT", "")
                .bel_attr(bel_ologic, "TDDR_ALIGNMENT", "")
                .bel_pin(bel_ologic, "CLK0")
                .test_routing_pair_special(
                    wires::IOI_OCLK[i].cell(0),
                    wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                    specials::IOI_OLOGIC_DDR,
                )
                .pip("CLK0_OLOGIC", "CLK0INTER")
                .commit();
            for j in 0..2 {
                for (pin, wire) in [
                    ("IOCE0", wires::IOCE[0]),
                    ("IOCE1", wires::IOCE[1]),
                    ("IOCE2", wires::IOCE[2]),
                    ("IOCE3", wires::IOCE[3]),
                    ("PLLCE0", wires::PLLCE[0]),
                    ("PLLCE1", wires::PLLCE[1]),
                ] {
                    bctx.build()
                        .has_related(Delta::new(0, 0, tcls::IOB))
                        .tile_mutex("CLK", ["TEST_ICE", "TEST_OCE"][j])
                        .mutex(["MUX.ICE", "MUX.OCE"][j], pin)
                        .test_routing(
                            [wires::IMUX_ILOGIC_IOCE, wires::IMUX_OLOGIC_IOCE][j][i].cell(0),
                            wire.cell(0).pos(),
                        )
                        .pip(format!("IOCE{j}"), (bel_ioi, pin))
                        .commit();
                }
            }
        }
        let mut bctx = ctx.bel(defs::bslots::MISC_IOI);
        if tcid == tcls::IOI_SN {
            let bel_iodelay = defs::bslots::IODELAY[0];
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global_mutex("MCB", "NONE")
                .global_mutex("DRPSDO", "TEST")
                .global("MEM_PLL_POL_SEL", "INVERTED")
                .global("MEM_PLL_DIV_EN", "DISABLED")
                .test_bel_special(specials::IOI_DRPSDO)
                .pip((bel_iodelay, "CE"), "MCB_DRPSDO")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global_mutex("MCB", "NONE")
                .global_mutex("DRPSDO", "TEST")
                .global("MEM_PLL_POL_SEL", "INVERTED")
                .global("MEM_PLL_DIV_EN", "ENABLED")
                .test_bel_special(specials::IOI_DRPSDO_DIV_EN)
                .pip((bel_iodelay, "CE"), "MCB_DRPSDO")
                .commit();
            bctx.build()
                .has_related(Delta::new(0, 0, tcls::IOB))
                .global_mutex("MCB", "NONE")
                .global_mutex("DRPSDO", "TEST")
                .global("MEM_PLL_POL_SEL", "NOTINVERTED")
                .global("MEM_PLL_DIV_EN", "DISABLED")
                .test_bel_special(specials::IOI_DRPSDO_NOTINV)
                .pip((bel_iodelay, "CE"), "MCB_DRPSDO")
                .commit();
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::MCB) {
        let mut bctx = ctx.bel(defs::bslots::MCB);
        bctx.build()
            .null_bits()
            .prop(AllMcbIoi(specials::IOI_DRPSDO))
            .global_mutex("MCB", "NONE")
            .global_mutex("DRPSDO", "TEST")
            .global("MEM_PLL_POL_SEL", "INVERTED")
            .global("MEM_PLL_DIV_EN", "DISABLED")
            .test_bel_special(specials::MCB_DRPSDO)
            .pip((PinFar, "IOIDRPSDO"), "IOIDRPSDO")
            .commit();
        bctx.build()
            .null_bits()
            .prop(AllMcbIoi(specials::IOI_DRPSDO_DIV_EN))
            .global_mutex("MCB", "NONE")
            .global_mutex("DRPSDO", "TEST")
            .global("MEM_PLL_POL_SEL", "INVERTED")
            .global("MEM_PLL_DIV_EN", "ENABLED")
            .test_bel_special(specials::MCB_DRPSDO)
            .pip((PinFar, "IOIDRPSDO"), "IOIDRPSDO")
            .commit();
        bctx.build()
            .null_bits()
            .prop(AllMcbIoi(specials::IOI_DRPSDO_NOTINV))
            .global_mutex("MCB", "NONE")
            .global_mutex("DRPSDO", "TEST")
            .global("MEM_PLL_POL_SEL", "NOTINVERTED")
            .global("MEM_PLL_DIV_EN", "DISABLED")
            .test_bel_special(specials::MCB_DRPSDO)
            .pip((PinFar, "IOIDRPSDO"), "IOIDRPSDO")
            .commit();
    }
    let mut ctx = FuzzCtx::new(session, backend, tcls::IOB);
    for i in 0..2 {
        let bel = defs::bslots::IOB[i];
        let mut bctx = ctx.bel(bel);
        let bel_other = defs::bslots::IOB[i ^ 1];
        bctx.build()
            .global_mutex("IOB", "SHARED")
            .global_mutex("VREF", "NO")
            .bel_mode(bel_other, "IOB")
            .test_bel_special(specials::PRESENT)
            .mode("IOB")
            .commit();
        if i == 0 {
            bctx.build()
                .global_mutex("IOB", "SHARED")
                .global_mutex("VREF", "YES")
                .global_mutex("VCCO.WE", "1800")
                .global_mutex("VREF.WE", "1800")
                .global_mutex("VCCO.SN", "1800")
                .global_mutex("VREF.SN", "1800")
                .raw(Key::Package, package.name.clone())
                .prop(IsVref(bel))
                .bel_mode(bel_other, "IOB")
                .bel_pin(bel_other, "I")
                .bel_attr(bel_other, "TUSED", "")
                .bel_attr(bel_other, "IMUX", "I")
                .bel_attr(bel_other, "BYPASS_MUX", "I")
                .bel_attr(bel_other, "ISTANDARD", "HSTL_I_18")
                .test_bel_special(specials::IOB_NOTVREF)
                .mode("IOB")
                .commit();
        }

        for (val, vname) in &edev.db[enums::IOB_PULL].values {
            if val == enums::IOB_PULL::NONE {
                continue;
            }
            bctx.mode("IOB")
                .global_mutex("IOB", "SHARED")
                .attr("TUSED", "0")
                .pin("T")
                .test_bel_attr_val(bcls::IOB::PULL, val)
                .attr("PULLTYPE", vname)
                .commit();
        }
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .attr("TUSED", "0")
            .pin("T")
            .test_bel_attr_auto(bcls::IOB::SUSPEND);
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .attr("TUSED", "0")
            .pin("T")
            .test_bel_attr_bits(bcls::IOB::PRE_EMPHASIS)
            .attr("PRE_EMPHASIS", "ON")
            .commit();
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .attr("TUSED", "0")
            .attr("BYPASS_MUX", "I")
            .pin("T")
            .pin("I")
            .test_bel_attr_bool_rename("IMUX", bcls::IOB::I_INV, "I", "I_B");
        for (val, vname) in [
            (enums::IOB_IBUF_MODE::NONE, "I"),
            (enums::IOB_IBUF_MODE::LOOPBACK_O, "O"),
            (enums::IOB_IBUF_MODE::LOOPBACK_T, "T"),
        ] {
            bctx.mode("IOB")
                .global_mutex("IOB", "SHARED")
                .mutex("MODE", "BYPASS")
                .attr("TUSED", "0")
                .attr("OUSED", "0")
                .attr("IMUX", "I")
                .pin("T")
                .pin("O")
                .pin("I")
                .test_bel_attr_val(bcls::IOB::IBUF_MODE, val)
                .attr("BYPASS_MUX", vname)
                .commit();
        }
        bctx.mode("IOB")
            .global_mutex("IOB", "SHARED")
            .mutex("MODE", "OUSED")
            .test_bel_attr_bits(bcls::IOB::OUTPUT_ENABLE)
            .attr("TUSED", "0")
            .attr("OUSED", "0")
            .attr("DRIVE_0MA", "DRIVE_0MA")
            .pin("T")
            .pin("O")
            .commit();

        let cnr_sw = CellCoord::new(DieId::from_idx(0), edev.chip.col_w(), edev.chip.row_s())
            .tile(defs::tslots::BEL);
        let cnr_nw = CellCoord::new(DieId::from_idx(0), edev.chip.col_w(), edev.chip.row_n())
            .tile(defs::tslots::BEL);
        let cnr_se = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_s())
            .tile(defs::tslots::BEL);
        let cnr_ne = CellCoord::new(
            DieId::from_idx(0),
            edev.chip.col_e(),
            edev.chip.row_n_inner(),
        )
        .tile(defs::tslots::BEL);

        bctx.build()
            .global("GLUTMASK", "YES")
            .global_mutex_here("IOB")
            .extra_fixed_bel_attr_bits(cnr_se, bslots::MISC_SE, bcls::MISC_SE::GLUTMASK_IOB)
            .test_bel_special(specials::PRESENT)
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
            .extra_fixed_bel_attr_bits(cnr_nw, bslots::MISC_NW, bcls::MISC_NW::VREF_LV)
            .test_bel_attr_bits_bi(bcls::IOB::VREF_HV, false)
            .attr_diff("ISTANDARD", "SSTL3_I", "SSTL18_I")
            .commit();

        let banks = [cnr_nw, cnr_se, cnr_sw, cnr_sw, cnr_nw, cnr_ne];
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
                .extra_fixed_bel_attr_val(
                    banks[bank],
                    bslots::OCT_CAL[bank],
                    bcls::OCT_CAL::VREF_VALUE,
                    enums::OCT_CAL_VREF_VALUE::_0P5,
                )
                .test_bel_sss_row(
                    specials::IOB_ISTD_3V3,
                    specials::IOB_STD_PLAIN,
                    specials::IOB_STD_PLAIN,
                    tables::IOB_DATA::SSTL2_I,
                )
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

        for (kind, ioi, iostds) in [
            ("WE", tcls::IOI_WE, IOSTDS_WE),
            ("SN", tcls::IOI_SN, IOSTDS_SN),
        ] {
            let bel_ologic = defs::bslots::OLOGIC[i];
            for vccaux in ["2.5", "3.3"] {
                let (istd_spec, ostd_spec, term_spec) = match vccaux {
                    "2.5" => (
                        specials::IOB_ISTD_2V5,
                        specials::IOB_OSTD_2V5,
                        specials::IOB_IN_TERM_2V5,
                    ),
                    "3.3" => (
                        specials::IOB_ISTD_3V3,
                        specials::IOB_OSTD_3V3,
                        specials::IOB_IN_TERM_3V3,
                    ),
                    _ => unreachable!(),
                };
                for std in iostds {
                    if matches!(std.name, "PCI33_3" | "PCI66_3" | "TMDS_33" | "LVPECL_33")
                        && vccaux == "2.5"
                    {
                        continue;
                    }
                    if std.name == "TML_33" {
                        continue;
                    }
                    let (row, spec) = iostd_to_row(edev, std, 2);
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
                                BelUnused::new(bel_ologic, 0),
                            ))
                            .test_bel_sss_row(istd_spec, spec, specials::IOB_STD_PLAIN, row)
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
                                    BelUnused::new(bel_ologic, 0),
                                ))
                                .test_bel_attr_bits(bcls::IOB::DIFF_TERM)
                                .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                                .commit();
                        }
                        if std.name.starts_with("DIFF_") {
                            for term in ["UNTUNED_SPLIT_25", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_75"]
                            {
                                let row_term = term_to_row(edev, term, std.vcco.unwrap());
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
                                        BelUnused::new(bel_ologic, 0),
                                    ))
                                    .extra_tile_bel_special(
                                        Delta::new(0, 0, ioi),
                                        bslots::OLOGIC[i],
                                        specials::IOI_IN_TERM,
                                    )
                                    .test_bel_special_row(term_spec, row_term)
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
                                BelUnused::new(bel_ologic, 0),
                            ))
                            .test_bel_sss_row(istd_spec, spec, specials::IOB_STD_PLAIN, row)
                            .attr("ISTANDARD", std.name)
                            .commit();
                        for term in ["UNTUNED_SPLIT_25", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_75"] {
                            let row_term = term_to_row(edev, term, std.vcco.unwrap());
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
                                    BelUnused::new(bel_ologic, 0),
                                ))
                                .extra_tile_bel_special(
                                    Delta::new(0, 0, ioi),
                                    bslots::OLOGIC[i],
                                    specials::IOI_IN_TERM,
                                )
                                .test_bel_special_row(term_spec, row_term)
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
                                BelUnused::new(bel_ologic, 0),
                            ))
                            .test_bel_sss_row(istd_spec, spec, specials::IOB_STD_PLAIN, row)
                            .attr("ISTANDARD", std.name)
                            .commit();
                        if std.name.starts_with("LVCMOS")
                            || std.name == "LVTTL"
                            || std.name == "MOBILE_DDR"
                        {
                            for term in ["UNTUNED_SPLIT_25", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_75"]
                            {
                                let row_term = term_to_row(edev, term, std.vcco.unwrap());
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
                                        BelUnused::new(bel_ologic, 0),
                                    ))
                                    .extra_tile_bel_special(
                                        Delta::new(0, 0, ioi),
                                        bslots::OLOGIC[i],
                                        specials::IOI_IN_TERM,
                                    )
                                    .test_bel_special_row(term_spec, row_term)
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
                        let (row, spec) = iostd_to_row(edev, std, 2);
                        let row_lvds = iostd_to_lvdsbias_row(edev, std);
                        for (dir, corner, corner_name, bank_bslot, dx) in [
                            (DirV::S, cnr_sw, "CNR_SW", bslots::BANK[2], 1),
                            (DirV::N, cnr_nw, "CNR_NW", bslots::BANK[0], -1),
                        ] {
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
                                    BelUnused::new(bel_ologic, 0),
                                ))
                                .attr("TUSED", "0")
                                .attr("OUSED", "0")
                                .attr("BYPASS_MUX", "")
                                .attr("SUSPEND", "")
                                .attr("PULLTYPE", "")
                                .pin("T")
                                .pin("O")
                                .extra_fixed_bel_special_row(
                                    corner,
                                    bank_bslot,
                                    specials::BANK_LVDSBIAS_0,
                                    row_lvds,
                                )
                                .test_bel_sss_row(ostd_spec, spec, specials::IOB_STD_GROUP0, row)
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
                                    BelUnused::new(bel_ologic, 0),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, ioi),
                                    BelUnused::new(bel_ologic, 0),
                                ))
                                .prop(Related::new(Delta::new(dx, 0, tcls::IOB), IsBonded(bel)))
                                .prop(Related::new(
                                    Delta::new(dx, 0, tcls::IOB),
                                    BaseBelMode::new(bel, 0, ["IOBS", "IOBM"][i].into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, tcls::IOB),
                                    BaseBelAttr::new(bel, 0, "TUSED".into(), "0".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, tcls::IOB),
                                    BaseBelAttr::new(bel, 0, "OUSED".into(), "0".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, tcls::IOB),
                                    BaseBelPin::new(bel, 0, "T".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, tcls::IOB),
                                    BaseBelPin::new(bel, 0, "O".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, tcls::IOB),
                                    BaseBelAttr::new(bel, 0, "OUTMUX".into(), ["0", ""][i].into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(dx, 0, tcls::IOB),
                                    BaseBelAttr::new(bel, 0, "OSTANDARD".into(), other_std.into()),
                                ))
                                .attr("TUSED", "0")
                                .attr("OUSED", "0")
                                .attr("BYPASS_MUX", "")
                                .attr("SUSPEND", "")
                                .attr("PULLTYPE", "")
                                .pin("T")
                                .pin("O")
                                .extra_fixed_bel_special_row(
                                    corner,
                                    bank_bslot,
                                    specials::BANK_LVDSBIAS_1,
                                    row_lvds,
                                )
                                .test_bel_sss_row(ostd_spec, spec, specials::IOB_STD_GROUP1, row)
                                .mode_diff("IOB", ["IOBS", "IOBM"][i])
                                .attr("OUTMUX", ["0", ""][i])
                                .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                .attr("OSTANDARD", std.name)
                                .commit();
                        }
                    } else {
                        let (drives, slews) = if std.drive.is_empty() {
                            (&[0][..], &[specials::IOB_SLEW_STD][..])
                        } else {
                            (
                                std.drive,
                                &[
                                    specials::IOB_SLEW_SLOW,
                                    specials::IOB_SLEW_FAST,
                                    specials::IOB_SLEW_QUIETIO,
                                ][..],
                            )
                        };
                        for &drive in drives {
                            let (row, spec) = iostd_to_row(edev, std, drive);
                            for &slew in slews {
                                bctx.build()
                                    .global_mutex("IOB", "SHARED")
                                    .global_mutex(format!("VCCO.{kind}"), vcco)
                                    .raw(Key::VccAux, vccaux)
                                    .raw(Key::Package, package.name.clone())
                                    .prop(IsBonded(bel))
                                    .mode("IOB")
                                    .prop(Related::new(
                                        Delta::new(0, 0, ioi),
                                        BelUnused::new(bel_ologic, 0),
                                    ))
                                    .attr("TUSED", "0")
                                    .attr("OUSED", "0")
                                    .attr("BYPASS_MUX", "")
                                    .attr("SUSPEND", "")
                                    .attr("PULLTYPE", "")
                                    .pin("T")
                                    .pin("O")
                                    .test_bel_sss_row(ostd_spec, spec, slew, row)
                                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                    .attr("OSTANDARD", std.name)
                                    .attr(
                                        "DRIVEATTRBOX",
                                        if drive == 0 {
                                            "".to_string()
                                        } else {
                                            drive.to_string()
                                        },
                                    )
                                    .attr(
                                        "SLEW",
                                        match slew {
                                            specials::IOB_SLEW_SLOW => "SLOW",
                                            specials::IOB_SLEW_FAST => "FAST",
                                            specials::IOB_SLEW_QUIETIO => "QUIETIO",
                                            _ => "",
                                        },
                                    )
                                    .commit();
                            }
                        }
                        if std.name == "LVTTL"
                            || std.name.starts_with("LVCMOS")
                            || std.name.contains("HSTL")
                            || std.name.contains("SSTL")
                            || std.name.contains("MOBILE_DDR")
                        {
                            for (term_spec, term) in [
                                (specials::IOB_TERM_UNTUNED_25, "UNTUNED_25"),
                                (specials::IOB_TERM_UNTUNED_50, "UNTUNED_50"),
                                (specials::IOB_TERM_UNTUNED_75, "UNTUNED_75"),
                            ] {
                                let (row, spec) = iostd_to_row(edev, std, 2);
                                bctx.build()
                                    .global_mutex("IOB", "SHARED")
                                    .global_mutex(format!("VCCO.{kind}"), vcco)
                                    .raw(Key::VccAux, vccaux)
                                    .raw(Key::Package, package.name.clone())
                                    .prop(IsBonded(bel))
                                    .mode("IOB")
                                    .prop(Related::new(
                                        Delta::new(0, 0, ioi),
                                        BelUnused::new(bel_ologic, 0),
                                    ))
                                    .attr("TUSED", "0")
                                    .attr("OUSED", "0")
                                    .attr("BYPASS_MUX", "")
                                    .attr("SUSPEND", "")
                                    .attr("PULLTYPE", "")
                                    .pin("T")
                                    .pin("O")
                                    .test_bel_sss_row(ostd_spec, spec, term_spec, row)
                                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                                    .attr("OSTANDARD", std.name)
                                    .attr("OUT_TERM", term)
                                    .attr(
                                        "SLEW",
                                        match slews[0] {
                                            specials::IOB_SLEW_SLOW => "SLOW",
                                            specials::IOB_SLEW_FAST => "FAST",
                                            specials::IOB_SLEW_QUIETIO => "QUIETIO",
                                            _ => "",
                                        },
                                    )
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
    for tcid in [tcls::IOI_WE, tcls::IOI_SN] {
        for i in 0..2 {
            let bslot = bslots::ILOGIC[i];
            ctx.get_diff_bel_special(tcid, bslot, specials::IOI_ILOGIC_ILOGIC2)
                .assert_empty();
            // TODO: wtf is this bit really? could be MUX.IOCE...
            ctx.collect_bel_attr(tcid, bslot, bcls::ILOGIC::ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::ILOGIC::IOCE_ENABLE);
            let diff = ctx.get_diff_routing(
                tcid,
                wires::IMUX_ILOGIC_CLK[i].cell(0),
                wires::IOI_ICLK[i].cell(0).pos(),
            );
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
            ctx.insert_mux(
                tcid,
                wires::IMUX_ILOGIC_CLK[i].cell(0),
                xlat_enum_raw(
                    vec![
                        (None, Diff::default()),
                        (Some(wires::IOI_ICLK[i].cell(0).pos()), diff),
                        (Some(wires::IOI_ICLK[i ^ 1].cell(0).pos()), diff2),
                    ],
                    OcdMode::BitOrder,
                ),
            );

            ctx.collect_bel_attr_bi(tcid, bslot, bcls::ILOGIC::BITSLIP_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::ILOGIC::FFI_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::ILOGIC::FFI_REV_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::ILOGIC::FFI_SR_SYNC);
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::ILOGIC::FFI_INIT, false)
                .assert_empty();
            let mut diff = ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::ILOGIC::FFI_INIT, true);
            let diff_init = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 38 | 41));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::ILOGIC::FFI_SRVAL, xlat_bit(diff));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::ILOGIC::FFI_INIT, xlat_bit(diff_init));
            ctx.collect_bel_attr(tcid, bslot, bcls::ILOGIC::FFI_CE_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::ILOGIC::MUX_TSBYPASS);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::ILOGIC::I_DELAY_ENABLE);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::ILOGIC::FFI_DELAY_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::ILOGIC::MUX_SR);

            if i == 0 {
                ctx.collect_bel_attr_default(
                    tcid,
                    bslot,
                    bcls::ILOGIC::MUX_D,
                    enums::ILOGIC_MUX_D::IOB_I,
                );
            }

            let mut serdes = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_ILOGIC_ISERDES2);
            let mut diff_ff =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_ILOGIC_IFFTYPE_FF);
            let diff_latch = ctx
                .get_diff_bel_special(tcid, bslot, specials::IOI_ILOGIC_IFFTYPE_LATCH)
                .combine(&!&diff_ff);
            let mut diff_ddr =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_ILOGIC_IFFTYPE_DDR);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::ILOGIC::FFI_LATCH, xlat_bit(diff_latch));

            diff_ff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::ILOGIC::FFI_CE_ENABLE),
                false,
                true,
            );
            diff_ff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::ILOGIC::ENABLE),
                true,
                false,
            );
            diff_ff.assert_empty();
            diff_ddr.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::ILOGIC::FFI_CE_ENABLE),
                false,
                true,
            );

            let mut diff_n = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::MUX_Q1,
                enums::ILOGIC_MUX_Q::NETWORKING,
            );
            let mut diff_np = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::MUX_Q1,
                enums::ILOGIC_MUX_Q::NETWORKING_PIPELINED,
            );
            let mut diff_r = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::MUX_Q1,
                enums::ILOGIC_MUX_Q::RETIMED,
            );
            for (attr, range) in [
                (bcls::ILOGIC::MUX_Q1, 46..50),
                (bcls::ILOGIC::MUX_Q2, 44..52),
                (bcls::ILOGIC::MUX_Q3, 42..54),
                (bcls::ILOGIC::MUX_Q4, 40..56),
            ] {
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    attr,
                    xlat_enum_attr(vec![
                        (enums::ILOGIC_MUX_Q::SHIFT_REGISTER, Diff::default()),
                        (
                            enums::ILOGIC_MUX_Q::NETWORKING,
                            diff_n.split_bits_by(|bit| range.contains(&bit.bit.to_idx())),
                        ),
                        (
                            enums::ILOGIC_MUX_Q::NETWORKING_PIPELINED,
                            diff_np.split_bits_by(|bit| range.contains(&bit.bit.to_idx())),
                        ),
                        (
                            enums::ILOGIC_MUX_Q::RETIMED,
                            diff_r.split_bits_by(|bit| range.contains(&bit.bit.to_idx())),
                        ),
                    ]),
                );
            }
            diff_n.assert_empty();
            diff_np.assert_empty();
            diff_r.assert_empty();

            let mut diff_1 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::DATA_WIDTH_START,
                enums::ILOGIC_DATA_WIDTH::_1,
            );
            let mut diff_2 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::DATA_WIDTH_START,
                enums::ILOGIC_DATA_WIDTH::_2,
            );
            let mut diff_3 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::DATA_WIDTH_START,
                enums::ILOGIC_DATA_WIDTH::_3,
            );
            let mut diff_4 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::DATA_WIDTH_START,
                enums::ILOGIC_DATA_WIDTH::_4,
            );
            let mut diff_5 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::DATA_WIDTH_START,
                enums::ILOGIC_DATA_WIDTH::_5,
            );
            let mut diff_6 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::DATA_WIDTH_START,
                enums::ILOGIC_DATA_WIDTH::_6,
            );
            let mut diff_7 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::DATA_WIDTH_START,
                enums::ILOGIC_DATA_WIDTH::_7,
            );
            let mut diff_8 = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::ILOGIC::DATA_WIDTH_START,
                enums::ILOGIC_DATA_WIDTH::_8,
            );
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
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslot,
                    bcls::ILOGIC::CASCADE_ENABLE,
                    xlat_bit(!diff_4_f),
                );
            } else {
                diff_4_f.assert_empty();
            }

            serdes = serdes
                .combine(&diff_1_f)
                .combine(&diff_2_f)
                .combine(&diff_3_f);
            diff_ddr = diff_ddr.combine(&diff_1_f);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::ILOGIC::ROW2_CLK_ENABLE,
                xlat_bit(!diff_1_f),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::ILOGIC::ROW3_CLK_ENABLE,
                xlat_bit(!diff_2_f),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::ILOGIC::ROW4_CLK_ENABLE,
                xlat_bit(!diff_3_f),
            );

            let (serdes, mut diff_ddr, diff_row1) = Diff::split(serdes, diff_ddr);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::ILOGIC::ROW1_CLK_ENABLE,
                xlat_bit(diff_row1),
            );

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
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::ILOGIC::DATA_WIDTH_RELOAD,
                    xlat_enum_attr(vec![
                        (enums::ILOGIC_DATA_WIDTH::_8, diff_8_a),
                        (enums::ILOGIC_DATA_WIDTH::_7, diff_7_a),
                        (enums::ILOGIC_DATA_WIDTH::_6, diff_6_a),
                        (enums::ILOGIC_DATA_WIDTH::_5, diff_5_a),
                        (enums::ILOGIC_DATA_WIDTH::_4, diff_4_a),
                        (enums::ILOGIC_DATA_WIDTH::_3, diff_3_a),
                        (enums::ILOGIC_DATA_WIDTH::_2, diff_2_a),
                        (enums::ILOGIC_DATA_WIDTH::_1, diff_1_a),
                    ]),
                );
                let (diff_5, diff_6, diff_casc) = Diff::split(diff_5, diff_6);
                let diff_7 = diff_7.combine(&!&diff_casc);
                let diff_8 = diff_8.combine(&!&diff_casc);
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::ILOGIC::DATA_WIDTH_START,
                    xlat_enum_attr(vec![
                        (enums::ILOGIC_DATA_WIDTH::_2, diff_2),
                        (enums::ILOGIC_DATA_WIDTH::_3, diff_3),
                        (enums::ILOGIC_DATA_WIDTH::_4, diff_4),
                        (enums::ILOGIC_DATA_WIDTH::_5, diff_5),
                        (enums::ILOGIC_DATA_WIDTH::_6, diff_6),
                        (enums::ILOGIC_DATA_WIDTH::_7, diff_7),
                        (enums::ILOGIC_DATA_WIDTH::_8, diff_8),
                    ]),
                );
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslot,
                    bcls::ILOGIC::CASCADE_ENABLE,
                    xlat_bit(diff_casc),
                );
                diff_ddr.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, bcls::ILOGIC::DATA_WIDTH_RELOAD),
                    enums::ILOGIC_DATA_WIDTH::_2,
                    enums::ILOGIC_DATA_WIDTH::_8,
                );
            } else {
                assert_eq!(diff_3_a, diff_5_a);
                assert_eq!(diff_3_a, diff_6_a);
                assert_eq!(diff_3_a, diff_7_a);
                assert_eq!(diff_3_a, diff_8_a);
                diff_5.assert_empty();
                diff_6.assert_empty();
                diff_7.assert_empty();
                diff_8.assert_empty();
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::ILOGIC::DATA_WIDTH_RELOAD,
                    xlat_enum_attr(vec![
                        (enums::ILOGIC_DATA_WIDTH::_4, diff_4_a),
                        (enums::ILOGIC_DATA_WIDTH::_3, diff_3_a),
                        (enums::ILOGIC_DATA_WIDTH::_2, diff_2_a),
                        (enums::ILOGIC_DATA_WIDTH::_1, diff_1_a),
                    ]),
                );
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    bcls::ILOGIC::DATA_WIDTH_START,
                    xlat_enum_attr(vec![
                        (enums::ILOGIC_DATA_WIDTH::_2, diff_2),
                        (enums::ILOGIC_DATA_WIDTH::_3, diff_3),
                        (enums::ILOGIC_DATA_WIDTH::_4, diff_4),
                    ]),
                );
                diff_ddr.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, bcls::ILOGIC::DATA_WIDTH_RELOAD),
                    enums::ILOGIC_DATA_WIDTH::_2,
                    enums::ILOGIC_DATA_WIDTH::_4,
                );
            }
            diff_ddr.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, bcls::ILOGIC::DATA_WIDTH_START),
                enums::ILOGIC_DATA_WIDTH::_3,
                enums::ILOGIC_DATA_WIDTH::_2,
            );

            ctx.insert_bel_attr_bool(tcid, bslot, bcls::ILOGIC::DDR, xlat_bit(diff_ddr));
        }
        for i in 0..2 {
            let bslot = bslots::OLOGIC[i];
            ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_OLOGIC2)
                .assert_empty();
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::IOCE_ENABLE);

            let diff = ctx.get_diff_routing(
                tcid,
                wires::IMUX_OLOGIC_CLK[i].cell(0),
                wires::IOI_OCLK[i].cell(0).pos(),
            );
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
            ctx.insert_mux(
                tcid,
                wires::IMUX_OLOGIC_CLK[i].cell(0),
                xlat_enum_raw(
                    vec![
                        (None, Diff::default()),
                        (Some(wires::IOI_OCLK[i].cell(0).pos()), diff),
                        (Some(wires::IOI_OCLK[i ^ 1].cell(0).pos()), diff2),
                    ],
                    OcdMode::BitOrder,
                ),
            );

            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::FFO_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::FFT_SR_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::FFO_REV_ENABLE);
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::FFT_REV_ENABLE);
            ctx.collect_bel_attr_default(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_SR,
                enums::OLOGIC_MUX_SR::GND,
            );
            ctx.collect_bel_attr_default(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_REV,
                enums::OLOGIC_MUX_REV::GND,
            );
            ctx.collect_bel_attr_default(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_TRAIN,
                enums::OLOGIC_MUX_TRAIN::GND,
            );
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::OLOGIC::FFO_SR_SYNC);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::OLOGIC::FFT_SR_SYNC);

            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::TRAIN_PATTERN);
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::MISR_ENABLE_DATA);
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::MISR_ENABLE_CLK);
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::MISR_RESET);
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::OLOGIC::CASCADE_ENABLE);
            if i == 0 {
                ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::OUTPUT_MODE);
            }

            let mut serdes = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_OSERDES2);
            serdes.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::ENABLE),
                true,
                false,
            );

            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFO_INIT, false)
                .assert_empty();
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFT_INIT, false)
                .assert_empty();
            let diff = ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFT_INIT, true);
            let (mut serdes, diff_init, diff_srval) = Diff::split(serdes, diff);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFT_INIT, xlat_bit(diff_init));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFT_SRVAL, xlat_bit(diff_srval));
            let mut diff = ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFO_INIT, true);
            let diff_srval = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 8 | 24));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFO_INIT, xlat_bit(diff));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFO_SRVAL, xlat_bit(diff_srval));

            let mut diff = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_IN_O,
                enums::OLOGIC_MUX_IN::MCB,
            );
            let diff_t = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 2 | 28));
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_IN_T,
                xlat_enum_attr(vec![
                    (enums::OLOGIC_MUX_IN::INT, Diff::default()),
                    (enums::OLOGIC_MUX_IN::MCB, diff_t),
                ]),
            );
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_IN_O,
                xlat_enum_attr(vec![
                    (enums::OLOGIC_MUX_IN::INT, Diff::default()),
                    (enums::OLOGIC_MUX_IN::MCB, diff),
                ]),
            );
            ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::MUX_O);
            ctx.collect_bel_attr_default(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_T,
                enums::OLOGIC_MUX_T::FFT,
            );

            let diff =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_DDR_ALIGNMENT_NONE);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::DDR_OPPOSITE_EDGE, xlat_bit(diff));
            let diff =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_TDDR_ALIGNMENT_NONE);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::DDR_OPPOSITE_EDGE, xlat_bit(diff));

            let diff = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_DDR_ALIGNMENT_C0);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::OLOGIC::FFO_RANK2_CLK_ENABLE,
                xlat_bit(diff),
            );
            let diff =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_TDDR_ALIGNMENT_C0);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::OLOGIC::FFT_RANK2_CLK_ENABLE,
                xlat_bit(diff),
            );

            let mut diff =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_BYPASS_GCLK_FF_FALSE);
            let diff_t = diff.split_bits_by(|bit| matches!(bit.bit.to_idx(), 6 | 22));
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::OLOGIC::FFO_RANK1_CLK_ENABLE,
                xlat_bit(diff),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::OLOGIC::FFT_RANK1_CLK_ENABLE,
                xlat_bit(diff_t),
            );

            let diff_bypass =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_BYPASS_GCLK_FF_TRUE);
            let diff_olatch =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_OUTFFTYPE_LATCH);
            let diff_off = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_OUTFFTYPE_FF);
            let diff_oddr =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_OUTFFTYPE_DDR);
            let diff_tlatch =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_TFFTYPE_LATCH);
            let diff_tff = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_TFFTYPE_FF);
            let diff_tddr = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_TFFTYPE_DDR);
            let diff_oce = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_OCE,
                enums::OLOGIC_MUX_OCE::INT,
            );
            let diff_oce_pci = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_OCE,
                enums::OLOGIC_MUX_OCE::PCI_CE,
            );
            let diff_tce = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_OLOGIC_TCE);

            let diff_oce_pci = diff_oce_pci.combine(&!&diff_oce);
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::OLOGIC::MUX_OCE,
                xlat_enum_attr(vec![
                    (enums::OLOGIC_MUX_OCE::INT, Diff::default()),
                    (enums::OLOGIC_MUX_OCE::PCI_CE, diff_oce_pci),
                ]),
            );

            let diff_tlatch = diff_tlatch.combine(&!&diff_tff);
            let diff_olatch = diff_olatch.combine(&!&diff_tlatch).combine(&!&diff_off);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFT_LATCH, xlat_bit(diff_tlatch));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFO_LATCH, xlat_bit(diff_olatch));

            let (diff_tff, diff_obypass, diff_tbypass) = Diff::split(diff_tff, diff_bypass);
            let diff_tddr = diff_tddr.combine(&!&diff_tbypass);
            let diff_off = diff_off.combine(&!&diff_obypass);
            let diff_oddr = diff_oddr.combine(&!&diff_obypass);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::OLOGIC::FFO_RANK1_BYPASS,
                xlat_bit(diff_obypass),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::OLOGIC::FFT_RANK1_BYPASS,
                xlat_bit(diff_tbypass),
            );

            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::ENABLE, xlat_bit(diff_off));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::ENABLE, xlat_bit(diff_tff));

            let diff_oce = diff_oce.combine(&!&diff_oddr);
            let diff_tce = diff_tce.combine(&!&diff_tddr);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::OLOGIC::FFO_CE_OR_DDR,
                xlat_bit(diff_oddr),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::OLOGIC::FFT_CE_OR_DDR,
                xlat_bit(diff_tddr),
            );

            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFO_CE_ENABLE, xlat_bit(diff_oce));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFT_CE_ENABLE, xlat_bit(diff_tce));

            serdes.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::FFO_RANK2_CLK_ENABLE),
                true,
                false,
            );
            serdes.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::FFT_RANK2_CLK_ENABLE),
                true,
                false,
            );
            serdes.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::FFO_CE_ENABLE),
                true,
                false,
            );
            serdes.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::FFT_CE_ENABLE),
                true,
                false,
            );
            serdes.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::FFO_CE_OR_DDR),
                true,
                false,
            );
            serdes.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::FFT_CE_OR_DDR),
                true,
                false,
            );

            serdes.assert_empty();

            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IN_TERM);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::FFT_INIT),
                true,
                false,
            );
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::FFT_CE_ENABLE),
                true,
                false,
            );
            diff.assert_empty();
        }
        let (_, _, diff) = Diff::split(
            ctx.peek_diff_bel_special(tcid, bslots::IODELAY[0], specials::IOI_IODELAY_IODRP2)
                .clone(),
            ctx.peek_diff_bel_special(tcid, bslots::IODELAY[1], specials::IOI_IODELAY_IODRP2)
                .clone(),
        );
        let (_, _, diff_mcb) = Diff::split(
            ctx.peek_diff_bel_special(tcid, bslots::IODELAY[0], specials::IOI_IODELAY_IODRP2_MCB)
                .clone(),
            ctx.peek_diff_bel_special(tcid, bslots::IODELAY[1], specials::IOI_IODELAY_IODRP2_MCB)
                .clone(),
        );
        let diff_mcb = diff_mcb.combine(&!&diff);
        ctx.insert_bel_attr_bool(
            tcid,
            bslots::MISC_IOI,
            bcls::MISC_IOI::DRP_ENABLE,
            xlat_bit(diff),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslots::MISC_IOI,
            bcls::MISC_IOI::DRP_FROM_MCB,
            xlat_bit(diff_mcb),
        );

        for i in 0..2 {
            let bslot = bslots::IODELAY[i];
            let diffs = ctx.get_diffs_attr_bits(tcid, bslot, bcls::IODELAY::ODELAY_VALUE_P, 8);
            let mut diffs_p = vec![];
            let mut diffs_n = vec![];
            for mut diff in diffs {
                let diff_p = diff.split_bits_by(|bit| (16..48).contains(&bit.bit.to_idx()));
                diffs_p.push(diff_p);
                diffs_n.push(diff);
            }
            ctx.insert_bel_attr_bitvec(
                tcid,
                bslot,
                bcls::IODELAY::ODELAY_VALUE_P,
                xlat_bitvec(diffs_p),
            );
            ctx.insert_bel_attr_bitvec(
                tcid,
                bslot,
                bcls::IODELAY::ODELAY_VALUE_N,
                xlat_bitvec(diffs_n),
            );
            ctx.collect_bel_attr(tcid, bslot, bcls::IODELAY::IDELAY_VALUE_P);
            ctx.collect_bel_attr(tcid, bslot, bcls::IODELAY::IDELAY_VALUE_N);
            let diffs =
                ctx.get_diffs_bel_special_bits(tcid, bslot, specials::IOI_IODELAY_MCB_ADDRESS, 4);
            if i == 1 {
                ctx.insert_bel_attr_bitvec(
                    tcid,
                    bslots::MISC_IOI,
                    bcls::MISC_IOI::DRP_MCB_ADDRESS,
                    xlat_bitvec(diffs),
                );
            } else {
                for diff in diffs {
                    diff.assert_empty();
                }
            }
            let diff = ctx.get_diff_attr_bool(tcid, bslot, bcls::IODELAY::CIN_ENABLE);
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IODELAY::CIN_ENABLE, xlat_bit_wide(diff));
            ctx.collect_bel_attr_bi(tcid, bslot, bcls::IODELAY::TEST_GLITCH_FILTER);
            ctx.collect_bel_attr(tcid, bslot, bcls::IODELAY::COUNTER_WRAPAROUND);
            ctx.collect_bel_attr(tcid, bslot, bcls::IODELAY::IODELAY_CHANGE);
            let diff = ctx
                .get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODELAY2_TEST_NCOUNTER)
                .combine(&!ctx.peek_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODELAY2));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::TEST_NCOUNTER, xlat_bit(diff));
            let diff = ctx
                .get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODELAY2_TEST_PCOUNTER)
                .combine(&!ctx.peek_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODELAY2));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::TEST_PCOUNTER, xlat_bit(diff));
            let diff = ctx
                .get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODRP2_IOIENFFSCAN_DRP)
                .combine(&!ctx.peek_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODRP2));
            ctx.insert_bel_attr_bitvec(
                tcid,
                bslots::MISC_IOI,
                bcls::MISC_IOI::ENFFSCAN_DRP,
                xlat_bit_wide(diff),
            );

            ctx.collect_bel_attr(tcid, bslot, bcls::IODELAY::ODATAIN_ENABLE);

            ctx.collect_mux(tcid, wires::IMUX_IODELAY_IOCLK[i].cell(0));

            let item = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_DEFAULT);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::IDELAY_FIXED, xlat_bit(item));
            let item = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_FIXED);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::IDELAY_FIXED, xlat_bit(item));
            ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_VARIABLE_FROM_ZERO)
                .assert_empty();
            let item =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_VARIABLE_FROM_HALF_MAX);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::IODELAY::IDELAY_FROM_HALF_MAX,
                xlat_bit(item),
            );
            let item = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_DPD_DEFAULT);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::IDELAY_FIXED, xlat_bit(item));
            let item = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_DPD_FIXED);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::IDELAY_FIXED, xlat_bit(item));
            ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_DPD_VARIABLE_FROM_ZERO)
                .assert_empty();
            let item = ctx.get_diff_bel_special(
                tcid,
                bslot,
                specials::IOI_IODELAY_DPD_VARIABLE_FROM_HALF_MAX,
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::IODELAY::IDELAY_FROM_HALF_MAX,
                xlat_bit(item),
            );
            let item =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_DIFF_PHASE_DETECTOR);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::IODELAY::DIFF_PHASE_DETECTOR,
                xlat_bit(item),
            );

            ctx.insert_bel_attr_bitvec(
                tcid,
                bslot,
                bcls::IODELAY::CAL_DELAY_MAX,
                vec![
                    TileBit::new(0, 28, [63, 0][i]).pos(),
                    TileBit::new(0, 28, [62, 1][i]).pos(),
                    TileBit::new(0, 28, [61, 2][i]).pos(),
                    TileBit::new(0, 28, [60, 3][i]).pos(),
                    TileBit::new(0, 28, [59, 4][i]).pos(),
                    TileBit::new(0, 28, [58, 5][i]).pos(),
                    TileBit::new(0, 28, [57, 6][i]).pos(),
                    TileBit::new(0, 28, [56, 7][i]).pos(),
                ],
            );
            ctx.insert_bel_attr_bitvec(
                tcid,
                bslot,
                bcls::IODELAY::DRP_ADDR,
                vec![
                    TileBit::new(0, 28, [39, 24][i]).pos(),
                    TileBit::new(0, 28, [38, 25][i]).pos(),
                    TileBit::new(0, 28, [37, 26][i]).pos(),
                    TileBit::new(0, 28, [36, 27][i]).pos(),
                    TileBit::new(0, 28, [32, 31][i]).pos(),
                ],
            );
            let drp06 = vec![
                TileBit::new(0, 28, [45, 18][i]).pos(),
                TileBit::new(0, 28, [47, 16][i]).pos(),
                TileBit::new(0, 28, [50, 13][i]).pos(),
                TileBit::new(0, 28, [53, 10][i]).pos(),
                TileBit::new(0, 28, [55, 8][i]).pos(),
                TileBit::new(0, 28, [49, 14][i]).pos(),
                TileBit::new(0, 28, [41, 22][i]).pos(),
                TileBit::new(0, 28, [43, 20][i]).pos(),
            ];
            let drp07 = vec![
                TileBit::new(0, 28, [44, 19][i]).pos(),
                TileBit::new(0, 28, [46, 17][i]).pos(),
                TileBit::new(0, 28, [51, 12][i]).pos(),
                TileBit::new(0, 28, [52, 11][i]).pos(),
                TileBit::new(0, 28, [54, 9][i]).pos(),
                TileBit::new(0, 28, [48, 15][i]).pos(),
                TileBit::new(0, 28, [40, 23][i]).pos(),
                TileBit::new(0, 28, [42, 21][i]).pos(),
            ];
            if i == 1 {
                ctx.insert_bel_attr_bitvec(
                    tcid,
                    bslot,
                    bcls::IODELAY::EVENT_SEL,
                    drp06[0..2].to_vec(),
                );
            } else {
                ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::PLUS1, drp06[0]);
            }
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::LUMPED_DELAY, drp07[3]);
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IODELAY::LUMPED_DELAY_SELECT, drp07[4]);
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IODELAY::DRP06, drp06);
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IODELAY::DRP07, drp07);

            ctx.collect_bel_attr(tcid, bslot, bcls::IODELAY::DELAY_SRC);
            ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::IODELAY::IDELAY_MODE,
                enums::IODELAY_IDELAY_MODE::NORMAL,
            )
            .assert_empty();
            let mut diff = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::IODELAY::IDELAY_MODE,
                enums::IODELAY_IDELAY_MODE::PCI,
            );
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, bcls::IODELAY::DELAY_SRC),
                enums::IODELAY_DELAY_SRC::ODATAIN,
                enums::IODELAY_DELAY_SRC::IO,
            );
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::IODELAY::IDELAY_MODE,
                xlat_enum_attr(vec![
                    (enums::IODELAY_IDELAY_MODE::NORMAL, Diff::default()),
                    (enums::IODELAY_IDELAY_MODE::PCI, diff),
                ]),
            );

            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::IODELAY::DELAYCHAIN_OSC, false)
                .assert_empty();
            let mut diff_iodelay2 =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODELAY2);
            let mut diff_iodrp2 =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODRP2);
            let mut diff_iodrp2_mcb =
                ctx.get_diff_bel_special(tcid, bslot, specials::IOI_IODELAY_IODRP2_MCB);
            let diff_delaychain_osc =
                ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::IODELAY::DELAYCHAIN_OSC, true);
            diff_iodrp2.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslots::MISC_IOI, bcls::MISC_IOI::DRP_ENABLE),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslots::MISC_IOI, bcls::MISC_IOI::DRP_ENABLE),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslots::MISC_IOI, bcls::MISC_IOI::DRP_FROM_MCB),
                true,
                false,
            );
            diff_iodrp2_mcb.apply_enum_diff_raw(
                ctx.sb_mux(tcid, wires::IMUX_IODELAY_IOCLK[i].cell(0)),
                &Some(wires::IMUX_OLOGIC_CLK[i].cell(0).pos()),
                &Some(wires::IMUX_ILOGIC_CLK[i].cell(0).pos()),
            );
            if i == 1 {
                diff_iodelay2.apply_bitvec_diff_int(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::IODELAY::EVENT_SEL),
                    3,
                    0,
                );
                diff_iodrp2.apply_bitvec_diff_int(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::IODELAY::EVENT_SEL),
                    3,
                    0,
                );
                diff_iodrp2_mcb.apply_bitvec_diff_int(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::IODELAY::EVENT_SEL),
                    3,
                    0,
                );
            }
            let (diff_iodrp2_mcb, diff_delaychain_osc, diff_common) =
                Diff::split(diff_iodrp2_mcb, diff_delaychain_osc);
            ctx.insert_bel_attr_bitvec(
                tcid,
                bslot,
                bcls::IODELAY::DELAYCHAIN_OSC_OR_ODATAIN_LP_OR_IDRP2_MCB,
                xlat_bit_wide(diff_common),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::IODELAY::DELAYCHAIN_OSC,
                xlat_bit(diff_delaychain_osc),
            );
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::IODELAY::MODE,
                xlat_enum_attr(vec![
                    (enums::IODELAY_MODE::IODELAY2, diff_iodelay2),
                    (enums::IODELAY_MODE::IODRP2, diff_iodrp2),
                    (enums::IODELAY_MODE::IODRP2_MCB, diff_iodrp2_mcb),
                ]),
            );
        }
        {
            let mut diff0 = ctx.get_diff_bel_special(
                tcid,
                bslots::IODELAY[0],
                specials::IOI_IODELAY_DPD_DIFF_PHASE_DETECTOR,
            );
            let mut diff1 = ctx.get_diff_bel_special(
                tcid,
                bslots::IODELAY[1],
                specials::IOI_IODELAY_DPD_DIFF_PHASE_DETECTOR,
            );
            diff0.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslots::IODELAY[0], bcls::IODELAY::DIFF_PHASE_DETECTOR),
                true,
                false,
            );
            diff1.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslots::IODELAY[1], bcls::IODELAY::DIFF_PHASE_DETECTOR),
                true,
                false,
            );
            diff0.apply_bit_diff(
                ctx.bel_attr_bit(
                    tcid,
                    bslots::IODELAY[1],
                    bcls::IODELAY::IDELAY_FROM_HALF_MAX,
                ),
                true,
                false,
            );
            diff1.apply_bit_diff(
                ctx.bel_attr_bit(
                    tcid,
                    bslots::IODELAY[1],
                    bcls::IODELAY::IDELAY_FROM_HALF_MAX,
                ),
                true,
                false,
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MISC_IOI,
                bcls::MISC_IOI::DIFF_PHASE_DETECTOR,
                xlat_bit(diff0),
            );
            ctx.insert_bel_attr_bool(
                tcid,
                bslots::MISC_IOI,
                bcls::MISC_IOI::DIFF_PHASE_DETECTOR,
                xlat_bit(diff1),
            );
        }
        for i in 0..2 {
            let bslot = bslots::IOI_DDR[i];
            ctx.collect_inv(tcid, wires::IOI_IOCLK_OPTINV[i * 3].cell(0));
            ctx.collect_inv(tcid, wires::IOI_IOCLK_OPTINV[i * 3 + 1].cell(0));
            ctx.collect_inv(tcid, wires::IOI_IOCLK_OPTINV[i * 3 + 2].cell(0));
            ctx.collect_mux(tcid, wires::IOI_IOCLK[i * 3].cell(0));
            ctx.collect_mux(tcid, wires::IOI_IOCLK[i * 3 + 1].cell(0));
            ctx.collect_mux(tcid, wires::IOI_IOCLK[i * 3 + 2].cell(0));

            let diff_iddr = ctx.get_diff_routing(
                tcid,
                wires::IOI_ICLK[i].cell(0),
                wires::OUT_DDR_IOCLK[i].cell(0).pos(),
            );
            let diff_iddr_ce = ctx.get_diff_routing_pair_special(
                tcid,
                wires::IOI_ICLK[i].cell(0),
                wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                specials::IOI_ILOGIC_DDR,
            );
            let diff_iddr_ce_c0 = ctx.get_diff_routing_pair_special(
                tcid,
                wires::IOI_ICLK[i].cell(0),
                wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                specials::IOI_ILOGIC_DDR_C0,
            );
            let diff_iddr_ce_c1 = ctx.get_diff_routing_pair_special(
                tcid,
                wires::IOI_ICLK[i].cell(0),
                wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                specials::IOI_ILOGIC_DDR_C1,
            );
            let diff_oddr = ctx.get_diff_routing(
                tcid,
                wires::IOI_OCLK[i].cell(0),
                wires::OUT_DDR_IOCLK[i].cell(0).pos(),
            );
            let diff_oddr_ce = ctx.get_diff_routing_pair_special(
                tcid,
                wires::IOI_OCLK[i].cell(0),
                wires::OUT_DDR_IOCLK[i].cell(0).pos(),
                specials::IOI_OLOGIC_DDR,
            );
            let diff_c0 = diff_iddr_ce_c0.combine(&!&diff_iddr_ce);
            let diff_c1 = diff_iddr_ce_c1.combine(&!&diff_iddr_ce);
            let diff_iddr_ce = diff_iddr_ce.combine(&!&diff_iddr);
            let diff_oddr_ce = diff_oddr_ce.combine(&!&diff_oddr);
            let (diff_iddr, diff_oddr, diff_ddr) = Diff::split(diff_iddr, diff_oddr);
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::IOI_DDR::ALIGNMENT,
                xlat_enum_attr(vec![
                    (enums::IOI_DDR_ALIGNMENT::NONE, Diff::default()),
                    (enums::IOI_DDR_ALIGNMENT::CLK0, diff_c0),
                    (enums::IOI_DDR_ALIGNMENT::CLK1, diff_c1),
                ]),
            );
            let item = xlat_enum_raw(
                vec![
                    (None, Diff::default()),
                    (
                        Some(wires::IOI_IOCLK[i * 3].cell(0).pos()),
                        ctx.get_diff_routing(
                            tcid,
                            wires::IOI_ICLK[i].cell(0),
                            wires::IOI_IOCLK[i * 3].cell(0).pos(),
                        ),
                    ),
                    (
                        Some(wires::IOI_IOCLK[i * 3 + 1].cell(0).pos()),
                        ctx.get_diff_routing(
                            tcid,
                            wires::IOI_ICLK[i].cell(0),
                            wires::IOI_IOCLK[i * 3 + 1].cell(0).pos(),
                        ),
                    ),
                    (
                        Some(wires::IOI_IOCLK[i * 3 + 2].cell(0).pos()),
                        ctx.get_diff_routing(
                            tcid,
                            wires::IOI_ICLK[i].cell(0),
                            wires::IOI_IOCLK[i * 3 + 2].cell(0).pos(),
                        ),
                    ),
                    (Some(wires::OUT_DDR_IOCLK[i].cell(0).pos()), diff_iddr),
                ],
                OcdMode::Mux,
            );
            ctx.insert_mux(tcid, wires::IOI_ICLK[i].cell(0), item);
            let item = xlat_enum_raw(
                vec![
                    (None, Diff::default()),
                    (
                        Some(wires::IOI_IOCLK[i * 3].cell(0).pos()),
                        ctx.get_diff_routing(
                            tcid,
                            wires::IOI_OCLK[i].cell(0),
                            wires::IOI_IOCLK[i * 3].cell(0).pos(),
                        ),
                    ),
                    (
                        Some(wires::IOI_IOCLK[i * 3 + 1].cell(0).pos()),
                        ctx.get_diff_routing(
                            tcid,
                            wires::IOI_OCLK[i].cell(0),
                            wires::IOI_IOCLK[i * 3 + 1].cell(0).pos(),
                        ),
                    ),
                    (
                        Some(wires::IOI_IOCLK[i * 3 + 2].cell(0).pos()),
                        ctx.get_diff_routing(
                            tcid,
                            wires::IOI_OCLK[i].cell(0),
                            wires::IOI_IOCLK[i * 3 + 2].cell(0).pos(),
                        ),
                    ),
                    (Some(wires::OUT_DDR_IOCLK[i].cell(0).pos()), diff_oddr),
                ],
                OcdMode::Mux,
            );
            ctx.insert_mux(tcid, wires::IOI_OCLK[i].cell(0), item);
            ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOI_DDR::ENABLE, xlat_bit_wide(diff_ddr));

            let diff_ice_ioce0 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_ILOGIC_IOCE[i].cell(0),
                wires::IOCE[0].cell(0).pos(),
            );
            let diff_ice_ioce1 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_ILOGIC_IOCE[i].cell(0),
                wires::IOCE[1].cell(0).pos(),
            );
            let diff_ice_ioce2 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_ILOGIC_IOCE[i].cell(0),
                wires::IOCE[2].cell(0).pos(),
            );
            let diff_ice_ioce3 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_ILOGIC_IOCE[i].cell(0),
                wires::IOCE[3].cell(0).pos(),
            );
            let diff_ice_pllce0 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_ILOGIC_IOCE[i].cell(0),
                wires::PLLCE[0].cell(0).pos(),
            );
            let diff_ice_pllce1 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_ILOGIC_IOCE[i].cell(0),
                wires::PLLCE[1].cell(0).pos(),
            );
            let diff_oce_ioce0 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_OLOGIC_IOCE[i].cell(0),
                wires::IOCE[0].cell(0).pos(),
            );
            let diff_oce_ioce1 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_OLOGIC_IOCE[i].cell(0),
                wires::IOCE[1].cell(0).pos(),
            );
            let diff_oce_ioce2 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_OLOGIC_IOCE[i].cell(0),
                wires::IOCE[2].cell(0).pos(),
            );
            let diff_oce_ioce3 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_OLOGIC_IOCE[i].cell(0),
                wires::IOCE[3].cell(0).pos(),
            );
            let diff_oce_pllce0 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_OLOGIC_IOCE[i].cell(0),
                wires::PLLCE[0].cell(0).pos(),
            );
            let diff_oce_pllce1 = ctx.get_diff_routing(
                tcid,
                wires::IMUX_OLOGIC_IOCE[i].cell(0),
                wires::PLLCE[1].cell(0).pos(),
            );
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
            let ce0 = wires::IOI_IOCE[2 * i].cell(0);
            let ce1 = wires::IOI_IOCE[2 * i + 1].cell(0);
            ctx.insert_mux(
                tcid,
                wires::IMUX_ILOGIC_IOCE[i].cell(0),
                xlat_enum_raw(
                    vec![
                        (None, Diff::default()),
                        (Some(ce0.pos()), diff_ice_ioce0),
                        (Some(ce0.pos()), diff_ice_ioce2),
                        (Some(ce0.pos()), diff_ice_pllce0),
                        (Some(ce1.pos()), diff_ice_ioce1),
                        (Some(ce1.pos()), diff_ice_ioce3),
                        (Some(ce1.pos()), diff_ice_pllce1),
                        (Some(wires::OUT_DDR_IOCE[i].cell(0).pos()), diff_iddr_ce),
                    ],
                    OcdMode::Mux,
                ),
            );
            ctx.insert_mux(
                tcid,
                wires::IMUX_OLOGIC_IOCE[i].cell(0),
                xlat_enum_raw(
                    vec![
                        (None, Diff::default()),
                        (Some(ce0.pos()), diff_oce_ioce0),
                        (Some(ce0.pos()), diff_oce_ioce2),
                        (Some(ce0.pos()), diff_oce_pllce0),
                        (Some(ce1.pos()), diff_oce_ioce1),
                        (Some(ce1.pos()), diff_oce_ioce3),
                        (Some(ce1.pos()), diff_oce_pllce1),
                        (Some(wires::OUT_DDR_IOCE[i].cell(0).pos()), diff_oddr_ce),
                    ],
                    OcdMode::Mux,
                ),
            );
            ctx.insert_mux(
                tcid,
                ce0,
                xlat_enum_raw(
                    vec![
                        (None, Diff::default()),
                        (Some(wires::IOCE[0].cell(0).pos()), diff_ioce0),
                        (Some(wires::IOCE[2].cell(0).pos()), diff_ioce2),
                        (Some(wires::PLLCE[0].cell(0).pos()), diff_pllce0),
                    ],
                    OcdMode::Mux,
                ),
            );
            ctx.insert_mux(
                tcid,
                ce1,
                xlat_enum_raw(
                    vec![
                        (None, Diff::default()),
                        (Some(wires::IOCE[1].cell(0).pos()), diff_ioce1),
                        (Some(wires::IOCE[3].cell(0).pos()), diff_ioce3),
                        (Some(wires::PLLCE[1].cell(0).pos()), diff_pllce1),
                    ],
                    OcdMode::Mux,
                ),
            );
        }
        if tcid == tcls::IOI_SN || ctx.has_tcls(tcls::MCB) {
            let bslot = bslots::MISC_IOI;
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IOI_DRPSDO);
            let diff_de = ctx
                .get_diff_bel_special(tcid, bslot, specials::IOI_DRPSDO_DIV_EN)
                .combine(&!&diff);
            let diff_ni = ctx
                .get_diff_bel_special(tcid, bslot, specials::IOI_DRPSDO_NOTINV)
                .combine(&!&diff);
            ctx.insert_bel_attr_bool(
                tcid,
                bslot,
                bcls::MISC_IOI::MEM_PLL_DIV_EN,
                xlat_bit(diff_de),
            );
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::MISC_IOI::MEM_PLL_POL_SEL,
                xlat_enum_attr(vec![
                    (enums::MCB_MEM_PLL_POL_SEL::INVERTED, Diff::default()),
                    (enums::MCB_MEM_PLL_POL_SEL::NOTINVERTED, diff_ni),
                ]),
            );
            diff.apply_bitvec_diff_int(
                ctx.bel_attr_bitvec(tcid, bslots::MISC_IOI, bcls::MISC_IOI::DRP_MCB_ADDRESS),
                0xa,
                0,
            );
            diff.assert_empty();
        }
    }
    for i in 0..2 {
        let tcid = tcls::IOB;
        let bslot = bslots::IOB[i];
        ctx.collect_bel_attr(tcid, bslot, bcls::IOB::OUTPUT_ENABLE);
        ctx.collect_bel_attr_default(tcid, bslot, bcls::IOB::PULL, enums::IOB_PULL::NONE);
        ctx.collect_bel_attr(tcid, bslot, bcls::IOB::SUSPEND);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::IOB::I_INV);
        ctx.collect_bel_attr(tcid, bslot, bcls::IOB::PRE_EMPHASIS);
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        present.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::IOB::PULL),
            enums::IOB_PULL::NONE,
            enums::IOB_PULL::PULLDOWN,
        );

        let pdrive: Vec<_> = (0..6)
            .map(|j| {
                let bit = TileBit::new(0, 0, i * 64 + j);
                match present.bits.remove(&bit) {
                    Some(true) => bit.neg(),
                    None => bit.pos(),
                    _ => unreachable!(),
                }
            })
            .collect();
        let ndrive: Vec<_> = (0..7)
            .map(|j| {
                let bit = TileBit::new(0, 0, i * 64 + 16 + j);
                match present.bits.remove(&bit) {
                    Some(true) => bit.neg(),
                    None => bit.pos(),
                    _ => unreachable!(),
                }
            })
            .collect();
        let pterm: Vec<_> = (0..6)
            .map(|j| TileBit::new(0, 0, i * 64 + 8 + j).pos())
            .collect();
        let nterm: Vec<_> = (0..7)
            .map(|j| TileBit::new(0, 0, i * 64 + 24 + j).pos())
            .collect();

        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::PDRIVE, pdrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::NDRIVE, ndrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::PTERM, pterm);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::NTERM, nterm);
        present.assert_empty();
        let pslew: Vec<_> = (0..4)
            .map(|j| {
                let bit = TileBit::new(0, 0, i * 64 + 32 + j);
                if j == 2 { bit.neg() } else { bit.pos() }
            })
            .collect();
        let nslew: Vec<_> = (0..4)
            .map(|j| {
                let bit = TileBit::new(0, 0, i * 64 + 36 + j);
                if j == 2 { bit.neg() } else { bit.pos() }
            })
            .collect();
        let pslew_invert = BitVec::from_iter(pslew.iter().map(|bit| bit.inv));
        let nslew_invert = BitVec::from_iter(nslew.iter().map(|bit| bit.inv));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::PSLEW, pslew);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::IOB::NSLEW, nslew);

        ctx.insert_table_bitvec(
            tables::IOB_DATA,
            tables::IOB_DATA::OFF,
            tables::IOB_DATA::PSLEW,
            BitVec::repeat(false, 4),
        );
        ctx.insert_table_bitvec(
            tables::IOB_DATA,
            tables::IOB_DATA::OFF,
            tables::IOB_DATA::NSLEW,
            BitVec::repeat(false, 4),
        );
        ctx.insert_table_bitvec(
            tables::IOB_DATA,
            tables::IOB_DATA::IN_TERM,
            tables::IOB_DATA::PSLEW,
            pslew_invert.clone(),
        );
        ctx.insert_table_bitvec(
            tables::IOB_DATA,
            tables::IOB_DATA::IN_TERM,
            tables::IOB_DATA::NSLEW,
            nslew_invert.clone(),
        );
        ctx.insert_table_bitvec(
            tables::IOB_DATA,
            tables::IOB_DATA::OFF,
            tables::IOB_DATA::PDRIVE,
            BitVec::repeat(false, 6),
        );
        ctx.insert_table_bitvec(
            tables::IOB_DATA,
            tables::IOB_DATA::OFF,
            tables::IOB_DATA::NDRIVE_2V5,
            BitVec::repeat(false, 7),
        );
        ctx.insert_table_bitvec(
            tables::IOB_DATA,
            tables::IOB_DATA::OFF,
            tables::IOB_DATA::NDRIVE_3V3,
            BitVec::repeat(false, 7),
        );
        ctx.insert_table_bitvec(
            tables::IOB_TERM,
            tables::IOB_TERM::OFF,
            tables::IOB_TERM::PTERM_2V5,
            bits![0; 6],
        );
        ctx.insert_table_bitvec(
            tables::IOB_TERM,
            tables::IOB_TERM::OFF,
            tables::IOB_TERM::PTERM_3V3,
            bits![0; 6],
        );
        ctx.insert_table_bitvec(
            tables::IOB_TERM,
            tables::IOB_TERM::OFF,
            tables::IOB_TERM::NTERM_2V5,
            bits![0; 7],
        );
        ctx.insert_table_bitvec(
            tables::IOB_TERM,
            tables::IOB_TERM::OFF,
            tables::IOB_TERM::NTERM_3V3,
            bits![0; 7],
        );

        if i == 0 {
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::IOB_NOTVREF);
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::PSLEW),
                &pslew_invert,
                &BitVec::repeat(false, 4),
            );
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::NSLEW),
                &nslew_invert,
                &BitVec::repeat(false, 4),
            );
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::VREF, xlat_bit(!diff));
        }

        let (_, _, diff) = Diff::split(
            ctx.peek_diff_bel_sss_row(
                tcid,
                bslot,
                specials::IOB_ISTD_3V3,
                specials::IOB_STD_PLAIN,
                specials::IOB_STD_PLAIN,
                tables::IOB_DATA::PCI33_3,
            )
            .clone(),
            ctx.peek_diff_bel_sss_row(
                tcid,
                bslot,
                specials::IOB_OSTD_3V3,
                specials::IOB_STD_PLAIN,
                specials::IOB_SLEW_STD,
                tables::IOB_DATA::PCI33_3,
            )
            .clone(),
        );
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::PCI_CLAMP, xlat_bit(diff));

        let mut diff = ctx
            .peek_diff_bel_sss_row(
                tcid,
                bslot,
                specials::IOB_ISTD_3V3,
                specials::IOB_STD_PLAIN,
                specials::IOB_STD_PLAIN,
                tables::IOB_DATA::PCI33_3,
            )
            .combine(&!ctx.peek_diff_bel_sss_row(
                tcid,
                bslot,
                specials::IOB_ISTD_3V3,
                specials::IOB_STD_PLAIN,
                specials::IOB_STD_PLAIN,
                tables::IOB_DATA::MOBILE_DDR,
            ));
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::IOB::PCI_CLAMP),
            true,
            false,
        );
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::PCI_INPUT, xlat_bit(diff));

        let diff = ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::IOB::VREF_HV, false);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::VREF_HV, xlat_bit(!diff));

        let item = xlat_enum_attr(vec![
            (
                enums::IOB_IBUF_MODE::NONE,
                ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOB::IBUF_MODE,
                    enums::IOB_IBUF_MODE::NONE,
                ),
            ),
            (
                enums::IOB_IBUF_MODE::LOOPBACK_T,
                ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOB::IBUF_MODE,
                    enums::IOB_IBUF_MODE::LOOPBACK_T,
                ),
            ),
            (
                enums::IOB_IBUF_MODE::LOOPBACK_O,
                ctx.get_diff_attr_val(
                    tcid,
                    bslot,
                    bcls::IOB::IBUF_MODE,
                    enums::IOB_IBUF_MODE::LOOPBACK_O,
                ),
            ),
            (
                enums::IOB_IBUF_MODE::CMOS_VCCINT,
                ctx.peek_diff_bel_sss_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_3V3,
                    specials::IOB_STD_PLAIN,
                    specials::IOB_STD_PLAIN,
                    tables::IOB_DATA::LVCMOS12_2,
                )
                .clone(),
            ),
            (
                enums::IOB_IBUF_MODE::CMOS_VCCO,
                ctx.peek_diff_bel_sss_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_3V3,
                    specials::IOB_STD_JEDEC,
                    specials::IOB_STD_PLAIN,
                    tables::IOB_DATA::LVCMOS12_2,
                )
                .clone(),
            ),
            (
                enums::IOB_IBUF_MODE::VREF,
                ctx.peek_diff_bel_sss_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_3V3,
                    specials::IOB_STD_PLAIN,
                    specials::IOB_STD_PLAIN,
                    tables::IOB_DATA::SSTL18_I,
                )
                .clone(),
            ),
            (
                enums::IOB_IBUF_MODE::DIFF,
                ctx.peek_diff_bel_sss_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_3V3,
                    specials::IOB_STD_PSEUDO_DIFF,
                    specials::IOB_STD_PLAIN,
                    tables::IOB_DATA::SSTL18_I,
                )
                .clone(),
            ),
            (
                enums::IOB_IBUF_MODE::CMOS_VCCAUX,
                ctx.peek_diff_bel_sss_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_3V3,
                    specials::IOB_STD_PLAIN,
                    specials::IOB_STD_PLAIN,
                    tables::IOB_DATA::LVTTL_2,
                )
                .clone(),
            ),
        ]);
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE, item);
        if i == 1 {
            let diff_lvds = ctx
                .peek_diff_bel_sss_row(
                    tcid,
                    bslot,
                    specials::IOB_OSTD_3V3,
                    specials::IOB_STD_TRUE_DIFF,
                    specials::IOB_STD_GROUP0,
                    tables::LVDSBIAS::LVDS_25,
                )
                .clone();
            let diff_tmds = ctx
                .peek_diff_bel_sss_row(
                    tcid,
                    bslot,
                    specials::IOB_OSTD_3V3,
                    specials::IOB_STD_TRUE_DIFF,
                    specials::IOB_STD_GROUP0,
                    tables::LVDSBIAS::TMDS_33,
                )
                .clone();
            let (diff_lvds, diff_tmds, mut diff) = Diff::split(diff_lvds, diff_tmds);
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::PSLEW),
                &BitVec::repeat(false, 4),
                &pslew_invert,
            );
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::NSLEW),
                &BitVec::repeat(false, 4),
                &nslew_invert,
            );
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::DIFF_OUTPUT_ENABLE, xlat_bit(diff));
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                bcls::IOB::DIFF_MODE,
                xlat_enum_attr(vec![
                    (enums::IOB_DIFF_MODE::NONE, Diff::default()),
                    (enums::IOB_DIFF_MODE::LVDS, diff_lvds),
                    (enums::IOB_DIFF_MODE::TMDS, diff_tmds),
                ]),
            );
            let mut diff = ctx.get_diff_attr_bool(tcid, bslot, bcls::IOB::DIFF_TERM);
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::PSLEW),
                &BitVec::repeat(false, 4),
                &pslew_invert,
            );
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::NSLEW),
                &BitVec::repeat(false, 4),
                &nslew_invert,
            );
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, bcls::IOB::DIFF_MODE),
                enums::IOB_DIFF_MODE::LVDS,
                enums::IOB_DIFF_MODE::NONE,
            );
            diff.assert_empty();
        } else {
            let mut diff = ctx.get_diff_attr_bool(tcid, bslot, bcls::IOB::DIFF_TERM);
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::PSLEW),
                &BitVec::repeat(false, 4),
                &pslew_invert,
            );
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::NSLEW),
                &BitVec::repeat(false, 4),
                &nslew_invert,
            );
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::DIFF_TERM, xlat_bit(diff));
        }

        let mut handled = HashSet::new();
        for iostds in [IOSTDS_WE, IOSTDS_SN] {
            for vccaux in ["2.5", "3.3"] {
                let istd_spec = match vccaux {
                    "2.5" => specials::IOB_ISTD_2V5,
                    "3.3" => specials::IOB_ISTD_3V3,
                    _ => unreachable!(),
                };
                for std in iostds {
                    if matches!(std.name, "PCI33_3" | "PCI66_3" | "TMDS_33" | "LVPECL_33")
                        && vccaux == "2.5"
                    {
                        continue;
                    }
                    if std.name == "TML_33" {
                        continue;
                    }
                    let (row, spec) = iostd_to_row(edev, std, 2);
                    let key = (istd_spec, spec, row);
                    if !handled.insert(key) {
                        continue;
                    }
                    let mut diff = ctx.get_diff_bel_sss_row(
                        tcid,
                        bslot,
                        istd_spec,
                        spec,
                        specials::IOB_STD_PLAIN,
                        row,
                    );
                    let val = if std.diff != DiffKind::None {
                        enums::IOB_IBUF_MODE::DIFF
                    } else if let Some(vref) = std.vref {
                        if vref >= 1250 {
                            diff.apply_bit_diff(
                                ctx.bel_attr_bit(tcid, bslot, bcls::IOB::VREF_HV),
                                true,
                                false,
                            );
                        }
                        enums::IOB_IBUF_MODE::VREF
                    } else if std.name.starts_with("PCI") {
                        diff.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslot, bcls::IOB::PCI_INPUT),
                            true,
                            false,
                        );
                        diff.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslot, bcls::IOB::PCI_CLAMP),
                            true,
                            false,
                        );
                        enums::IOB_IBUF_MODE::CMOS_VCCO
                    } else if matches!(std.name, "LVCMOS12" | "LVCMOS15" | "LVCMOS18") {
                        enums::IOB_IBUF_MODE::CMOS_VCCINT
                    } else if matches!(
                        std.name,
                        "LVCMOS12_JEDEC" | "LVCMOS15_JEDEC" | "LVCMOS18_JEDEC" | "MOBILE_DDR"
                    ) || (vccaux == "3.3" && std.name == "LVCMOS25")
                    {
                        enums::IOB_IBUF_MODE::CMOS_VCCO
                    } else {
                        enums::IOB_IBUF_MODE::CMOS_VCCAUX
                    };
                    diff.apply_enum_diff(
                        ctx.bel_attr_enum(tcid, bslot, bcls::IOB::IBUF_MODE),
                        val,
                        enums::IOB_IBUF_MODE::NONE,
                    );
                    diff.assert_empty();
                }
            }
        }
        for (term_spec, field_pterm, field_nterm) in [
            (
                specials::IOB_IN_TERM_2V5,
                tables::IOB_TERM::PTERM_2V5,
                tables::IOB_TERM::NTERM_2V5,
            ),
            (
                specials::IOB_IN_TERM_3V3,
                tables::IOB_TERM::PTERM_3V3,
                tables::IOB_TERM::NTERM_3V3,
            ),
        ] {
            for term in ["UNTUNED_SPLIT_25", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_75"] {
                for vcco in [1200, 1500, 1800, 2500, 3300] {
                    let row_term = term_to_row(edev, term, vcco);
                    let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, term_spec, row_term);
                    diff.apply_bit_diff(
                        ctx.bel_attr_bit(tcid, bslot, bcls::IOB::OUTPUT_ENABLE),
                        true,
                        false,
                    );
                    let val = extract_bitvec_val_part(
                        ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::PTERM),
                        &BitVec::repeat(false, 6),
                        &mut diff,
                    );
                    ctx.insert_table_bitvec(tables::IOB_TERM, row_term, field_pterm, val);
                    let val = extract_bitvec_val_part(
                        ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::NTERM),
                        &BitVec::repeat(false, 7),
                        &mut diff,
                    );
                    ctx.insert_table_bitvec(tables::IOB_TERM, row_term, field_nterm, val);

                    if vcco >= 2500 {
                        diff.assert_empty()
                    } else {
                        ctx.insert_bel_attr_bool(
                            tcid,
                            bslot,
                            bcls::IOB::OUTPUT_LOW_VOLTAGE,
                            xlat_bit(diff),
                        );
                    }
                }
            }
        }
        let mut handled_ostd = HashSet::new();
        for iostds in [IOSTDS_WE, IOSTDS_SN] {
            for vccaux in ["2.5", "3.3"] {
                let (ostd_spec, ndrive_field) = match vccaux {
                    "2.5" => (specials::IOB_OSTD_2V5, tables::IOB_DATA::NDRIVE_2V5),
                    "3.3" => (specials::IOB_OSTD_3V3, tables::IOB_DATA::NDRIVE_3V3),
                    _ => unreachable!(),
                };
                for std in iostds {
                    if std.input_only {
                        continue;
                    }
                    if matches!(std.name, "PCI33_3" | "PCI66_3" | "TMDS_33" | "TML_33")
                        && vccaux == "2.5"
                    {
                        continue;
                    }
                    if std.diff == DiffKind::True {
                        let (row, spec) = iostd_to_row(edev, std, 2);
                        let mut diff0 = ctx.get_diff_bel_sss_row(
                            tcid,
                            bslot,
                            ostd_spec,
                            spec,
                            specials::IOB_STD_GROUP0,
                            row,
                        );
                        let diff1 = ctx
                            .get_diff_bel_sss_row(
                                tcid,
                                bslot,
                                ostd_spec,
                                spec,
                                specials::IOB_STD_GROUP1,
                                row,
                            )
                            .combine(&!&diff0);
                        if i == 1 {
                            ctx.insert_bel_attr_bool(
                                tcid,
                                bslot,
                                bcls::IOB::LVDS_GROUP,
                                xlat_bit(diff1),
                            );
                        } else {
                            diff1.assert_empty();
                        }
                        if i == 1 {
                            diff0.apply_bit_diff(
                                ctx.bel_attr_bit(tcid, bslot, bcls::IOB::DIFF_OUTPUT_ENABLE),
                                true,
                                false,
                            );
                            diff0.apply_enum_diff(
                                ctx.bel_attr_enum(tcid, bslot, bcls::IOB::DIFF_MODE),
                                if matches!(std.name, "TMDS_33" | "TML_33") {
                                    enums::IOB_DIFF_MODE::TMDS
                                } else {
                                    enums::IOB_DIFF_MODE::LVDS
                                },
                                enums::IOB_DIFF_MODE::NONE,
                            );
                        }
                        if std.name == "TML_33" {
                            for (attr, field, base) in [
                                (
                                    bcls::IOB::PTERM,
                                    tables::IOB_TERM::PTERM_3V3,
                                    &BitVec::repeat(false, 6),
                                ),
                                (
                                    bcls::IOB::NTERM,
                                    tables::IOB_TERM::NTERM_3V3,
                                    &BitVec::repeat(false, 7),
                                ),
                            ] {
                                let val = extract_bitvec_val_part(
                                    ctx.bel_attr_bitvec(tcid, bslot, attr),
                                    base,
                                    &mut diff0,
                                );
                                ctx.insert_table_bitvec(
                                    tables::IOB_TERM,
                                    tables::IOB_TERM::TML_33,
                                    field,
                                    val,
                                );
                            }
                            for (attr, field, base) in [
                                (
                                    bcls::IOB::PDRIVE,
                                    tables::IOB_DATA::PDRIVE,
                                    &BitVec::repeat(false, 6),
                                ),
                                (
                                    bcls::IOB::NDRIVE,
                                    tables::IOB_DATA::NDRIVE_3V3,
                                    &BitVec::repeat(false, 7),
                                ),
                                (bcls::IOB::PSLEW, tables::IOB_DATA::PSLEW, &pslew_invert),
                                (bcls::IOB::NSLEW, tables::IOB_DATA::NSLEW, &nslew_invert),
                            ] {
                                let val = extract_bitvec_val_part(
                                    ctx.bel_attr_bitvec(tcid, bslot, attr),
                                    base,
                                    &mut diff0,
                                );
                                ctx.insert_table_bitvec(
                                    tables::IOB_DATA,
                                    tables::IOB_DATA::TML_33,
                                    field,
                                    val,
                                );
                            }
                            ctx.insert_bel_attr_bool(tcid, bslot, bcls::IOB::TML, xlat_bit(diff0));
                        } else {
                            diff0.apply_bitvec_diff(
                                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::PSLEW),
                                &BitVec::repeat(false, 4),
                                &pslew_invert,
                            );
                            diff0.apply_bitvec_diff(
                                ctx.bel_attr_bitvec(tcid, bslot, bcls::IOB::NSLEW),
                                &BitVec::repeat(false, 4),
                                &nslew_invert,
                            );
                            diff0.assert_empty();
                        }
                    } else {
                        let (drives, slews) = if std.drive.is_empty() {
                            (&[0][..], &[specials::IOB_SLEW_STD][..])
                        } else {
                            (
                                std.drive,
                                &[
                                    specials::IOB_SLEW_SLOW,
                                    specials::IOB_SLEW_FAST,
                                    specials::IOB_SLEW_QUIETIO,
                                ][..],
                            )
                        };
                        for &drive in drives {
                            for &slew in slews {
                                let (row, spec) = iostd_to_row(edev, std, drive);
                                let slew_row = iostd_slew_to_row(edev, std, slew);
                                let key = (ostd_spec, row, spec, slew);
                                if !handled_ostd.insert(key) {
                                    continue;
                                }
                                let mut diff = ctx
                                    .get_diff_bel_sss_row(tcid, bslot, ostd_spec, spec, slew, row);
                                if let Some(vcco) = std.vcco
                                    && vcco < 2500
                                {
                                    diff.apply_bit_diff(
                                        ctx.bel_attr_bit(
                                            tcid,
                                            bslot,
                                            bcls::IOB::OUTPUT_LOW_VOLTAGE,
                                        ),
                                        true,
                                        false,
                                    );
                                }
                                if std.name.starts_with("PCI") {
                                    diff.apply_bit_diff(
                                        ctx.bel_attr_bit(tcid, bslot, bcls::IOB::PCI_CLAMP),
                                        true,
                                        false,
                                    );
                                }
                                for (attr, field, base) in [
                                    (
                                        bcls::IOB::PDRIVE,
                                        tables::IOB_DATA::PDRIVE,
                                        BitVec::repeat(false, 6),
                                    ),
                                    (bcls::IOB::NDRIVE, ndrive_field, BitVec::repeat(false, 7)),
                                ] {
                                    let val = extract_bitvec_val_part(
                                        ctx.bel_attr_bitvec(tcid, bslot, attr),
                                        &base,
                                        &mut diff,
                                    );
                                    ctx.insert_table_bitvec(tables::IOB_DATA, row, field, val);
                                }
                                for (attr, field, base) in [
                                    (bcls::IOB::PSLEW, tables::IOB_DATA::PSLEW, &pslew_invert),
                                    (bcls::IOB::NSLEW, tables::IOB_DATA::NSLEW, &nslew_invert),
                                ] {
                                    let val = extract_bitvec_val_part(
                                        ctx.bel_attr_bitvec(tcid, bslot, attr),
                                        base,
                                        &mut diff,
                                    );
                                    ctx.insert_table_bitvec(tables::IOB_DATA, slew_row, field, val);
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
                            for (term_spec, term) in [
                                (specials::IOB_TERM_UNTUNED_25, "UNTUNED_25"),
                                (specials::IOB_TERM_UNTUNED_50, "UNTUNED_50"),
                                (specials::IOB_TERM_UNTUNED_75, "UNTUNED_75"),
                            ] {
                                let (row, spec) = iostd_to_row(edev, std, 2);
                                let key = (ostd_spec, row, spec, term_spec);
                                if !handled_ostd.insert(key) {
                                    continue;
                                }
                                let mut diff = ctx.get_diff_bel_sss_row(
                                    tcid, bslot, ostd_spec, spec, term_spec, row,
                                );
                                let vcco = std.vcco.unwrap();
                                if vcco < 2500 {
                                    diff.apply_bit_diff(
                                        ctx.bel_attr_bit(
                                            tcid,
                                            bslot,
                                            bcls::IOB::OUTPUT_LOW_VOLTAGE,
                                        ),
                                        true,
                                        false,
                                    );
                                }
                                let term_row = edev.db[tables::IOB_DATA]
                                    .rows
                                    .get(&format!(
                                        "{term}_{a}V{b}",
                                        a = vcco / 1000,
                                        b = vcco / 100 % 10
                                    ))
                                    .unwrap()
                                    .0;
                                for (attr, field, base) in [
                                    (
                                        bcls::IOB::PDRIVE,
                                        tables::IOB_DATA::PDRIVE,
                                        BitVec::repeat(false, 6),
                                    ),
                                    (bcls::IOB::NDRIVE, ndrive_field, BitVec::repeat(false, 7)),
                                ] {
                                    let val = extract_bitvec_val_part(
                                        ctx.bel_attr_bitvec(tcid, bslot, attr),
                                        &base,
                                        &mut diff,
                                    );
                                    ctx.insert_table_bitvec(tables::IOB_DATA, term_row, field, val);
                                }
                                let slew_row = iostd_slew_to_row(edev, std, slews[0]);
                                for (attr, field, base) in [
                                    (bcls::IOB::PSLEW, tables::IOB_DATA::PSLEW, &pslew_invert),
                                    (bcls::IOB::NSLEW, tables::IOB_DATA::NSLEW, &nslew_invert),
                                ] {
                                    let val = extract_bitvec_val_part(
                                        ctx.bel_attr_bitvec(tcid, bslot, attr),
                                        base,
                                        &mut diff,
                                    );
                                    ctx.insert_table_bitvec(tables::IOB_DATA, slew_row, field, val);
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
        let tcid = tcls::CNR_SW;
        let bslot = bslots::BANK[2];
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::BANK::LVDSBIAS,
            (0..(2 * 12))
                .map(|i| TileBit::new(0, 23, 29 + i).pos())
                .collect(),
        );
    }
    {
        let tcid = tcls::CNR_SE;
        let bslot = bslots::MISC_SE;
        ctx.collect_bel_attr(tcid, bslot, bcls::MISC_SE::GLUTMASK_IOB);
    }
    {
        let tcid = tcls::CNR_NW;
        let bslot = bslots::MISC_NW;
        let item = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::MISC_NW::VREF_LV));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::MISC_NW::VREF_LV, item);
        let bslot = bslots::BANK[0];
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::BANK::LVDSBIAS,
            vec![
                TileBit::new(0, 22, 9).pos(),
                TileBit::new(0, 22, 21).pos(),
                TileBit::new(0, 22, 20).pos(),
                TileBit::new(0, 22, 19).pos(),
                TileBit::new(0, 22, 18).pos(),
                TileBit::new(0, 22, 17).pos(),
                TileBit::new(0, 22, 16).pos(),
                TileBit::new(0, 22, 15).pos(),
                TileBit::new(0, 22, 14).pos(),
                TileBit::new(0, 22, 13).pos(),
                TileBit::new(0, 22, 12).pos(),
                TileBit::new(0, 22, 11).pos(),
                //
                TileBit::new(0, 22, 10).pos(),
                TileBit::new(0, 22, 27).pos(),
                TileBit::new(0, 22, 26).pos(),
                TileBit::new(0, 22, 25).pos(),
                TileBit::new(0, 22, 24).pos(),
                TileBit::new(0, 22, 23).pos(),
                TileBit::new(0, 22, 22).pos(),
                TileBit::new(0, 22, 32).pos(),
                TileBit::new(0, 22, 31).pos(),
                TileBit::new(0, 22, 30).pos(),
                TileBit::new(0, 22, 29).pos(),
                TileBit::new(0, 22, 28).pos(),
            ],
        );
    }
    ctx.insert_table_bitvec(
        tables::LVDSBIAS,
        tables::LVDSBIAS::OFF,
        tables::LVDSBIAS::LVDSBIAS,
        bits![0; 12],
    );
    for (tcid, bslot) in [
        (tcls::CNR_SW, bslots::BANK[2]),
        (tcls::CNR_NW, bslots::BANK[0]),
    ] {
        for std in IOSTDS_SN {
            if std.diff != DiffKind::True {
                continue;
            }
            let row = iostd_to_lvdsbias_row(edev, std);
            for i in 0..2 {
                let diff = ctx.get_diff_bel_special_row(
                    tcid,
                    bslot,
                    [specials::BANK_LVDSBIAS_0, specials::BANK_LVDSBIAS_1][i],
                    row,
                );
                let item =
                    &ctx.bel_attr_bitvec(tcid, bslot, bcls::BANK::LVDSBIAS)[i * 12..(i + 1) * 12];
                let val = extract_bitvec_val(item, &BitVec::repeat(false, 12), diff);
                ctx.insert_table_bitvec(tables::LVDSBIAS, row, tables::LVDSBIAS::LVDSBIAS, val);
            }
        }
    }
    for (tcid, bank, bit_25, bit_75) in [
        (
            tcls::CNR_SW,
            2,
            TileBit::new(0, 23, 27),
            TileBit::new(0, 23, 28),
        ),
        (
            tcls::CNR_SW,
            3,
            TileBit::new(0, 23, 24),
            TileBit::new(0, 23, 25),
        ),
        (
            tcls::CNR_NW,
            0,
            TileBit::new(0, 22, 43),
            TileBit::new(0, 22, 42),
        ),
        (
            tcls::CNR_NW,
            4,
            TileBit::new(0, 22, 46),
            TileBit::new(0, 22, 45),
        ),
        (
            tcls::CNR_SE,
            1,
            TileBit::new(0, 22, 52),
            TileBit::new(0, 22, 53),
        ),
        (
            tcls::CNR_NE,
            5,
            TileBit::new(1, 22, 51),
            TileBit::new(1, 22, 52),
        ),
    ] {
        let bslot = bslots::OCT_CAL[bank];
        let item = BelAttributeEnum {
            bits: vec![bit_25, bit_75],
            values: [
                (enums::OCT_CAL_VREF_VALUE::NONE, bits![0, 0]),
                (enums::OCT_CAL_VREF_VALUE::_0P25, bits![1, 0]),
                (enums::OCT_CAL_VREF_VALUE::_0P75, bits![0, 1]),
                (enums::OCT_CAL_VREF_VALUE::_0P5, bits![1, 1]),
            ]
            .into_iter()
            .collect(),
        };
        if bank < 4 || edev.chip.row_mcb_split.is_some() {
            let mut diff = ctx.get_diff_attr_val(
                tcid,
                bslot,
                bcls::OCT_CAL::VREF_VALUE,
                enums::OCT_CAL_VREF_VALUE::_0P5,
            );
            diff.apply_enum_diff(
                &item,
                enums::OCT_CAL_VREF_VALUE::_0P5,
                enums::OCT_CAL_VREF_VALUE::NONE,
            );
            diff.assert_empty();
        }
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::OCT_CAL::VREF_VALUE, item);
    }
}
