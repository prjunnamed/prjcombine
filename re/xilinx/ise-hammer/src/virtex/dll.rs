use std::collections::BTreeMap;

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelInfo, BelSlotId},
    dir::DirH,
    grid::{CellCoord, DieId, TileCoord},
};
use prjcombine_re_collector::legacy::{xlat_bit_bi_legacy, xlat_bit_legacy, xlat_enum_legacy};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex::defs;
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
};

#[derive(Copy, Clone, Debug)]
struct DeviceSide(DirH);

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
        let ExpandedDevice::Virtex(edev) = backend.edev else {
            unreachable!()
        };
        match self.0 {
            DirH::W => {
                if tcrd.col >= edev.chip.col_clk() {
                    return None;
                }
            }
            DirH::E => {
                if tcrd.col < edev.chip.col_clk() {
                    return None;
                }
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
struct PinWireMutexShared(BelSlotId, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for PinWireMutexShared {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tile = &backend.edev[tcrd];
        let tcls = &backend.edev.db[tile.class];
        let bel_data = &tcls.bels[self.0];
        let BelInfo::Legacy(bel_data) = bel_data else {
            unreachable!()
        };
        let pin_data = &bel_data.pins[self.1];
        for &wire in &pin_data.wires {
            let wire = backend
                .edev
                .resolve_wire(backend.edev.tile_wire(tcrd, wire))?;
            fuzzer = fuzzer.base(Key::WireMutex(wire), "SHARED");
        }
        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
struct FuzzGlobalDll(BelSlotId, &'static str, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzGlobalDll {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let site = backend.ngrid.get_bel_name(tcrd.bel(self.0)).unwrap();
        let opt = self.1;
        let ExpandedDevice::Virtex(edev) = backend.edev else {
            unreachable!()
        };
        let opt = if opt == "TESTZD2OSC*"
            && site.len() == 4
            && edev.chip.kind != prjcombine_virtex::chip::ChipKind::Virtex
        {
            opt.replace('*', &format!("{}S", &site[3..]))
        } else {
            opt.replace('*', &site[3..])
        };
        fuzzer = fuzzer.fuzz(Key::GlobalOpt(opt), None, self.2);
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex(edev) = backend.edev else {
        unreachable!()
    };
    for tile in ["DLL_S", "DLL_N", "DLLP_S", "DLLP_N", "DLLS_S", "DLLS_N"] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let mut bctx = ctx.bel(defs::bslots::DLL);
        let cnr_tl = CellCoord::new(DieId::from_idx(0), edev.chip.col_w(), edev.chip.row_n())
            .tile(defs::tslots::MAIN);
        bctx.build()
            .extra_tile_attr_fixed(cnr_tl, "MISC", "DLL_ENABLE", "1")
            .global_mutex_here("DLL")
            .test_manual_legacy("PRESENT", "1")
            .mode("DLL")
            .commit();
        bctx.mode("DLL")
            .global_mutex("DLL", "USE")
            .pin("RST")
            .test_enum("RSTMUX", &["0", "1", "RST", "RST_B"]);
        bctx.mode("DLL")
            .global_mutex("DLL", "USE")
            .test_manual_legacy("HIGH_FREQUENCY", "1")
            .attr("HIGH_FREQ_ATTR", "HIGH_FREQUENCY")
            .commit();
        bctx.mode("DLL")
            .global_mutex("DLL", "USE")
            .test_enum("DUTY_ATTR", &["FALSE", "TRUE"]);
        for attr in ["JF_ZD1_ATTR", "JF_ZD2_ATTR"] {
            bctx.mode("DLL").global_mutex("DLL", "USE").test_enum(
                attr,
                &[
                    "0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF",
                ],
            );
        }
        bctx.mode("DLL").global_mutex("DLL", "USE").test_enum(
            "DIVIDE_ATTR",
            &[
                "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            ],
        );
        for i in 1..8 {
            bctx.mode("DLL")
                .global_mutex("DLL", "USE")
                .attr("HIGH_FREQ_ATTR", "")
                .test_manual_legacy("DIVIDE_ATTR", format!("{i}_5.LOW"))
                .attr("DIVIDE_ATTR", format!("{i}_5"))
                .commit();
            bctx.mode("DLL")
                .global_mutex("DLL", "USE")
                .attr("HIGH_FREQ_ATTR", "HIGH_FREQUENCY")
                .test_manual_legacy("DIVIDE_ATTR", format!("{i}_5.HIGH"))
                .attr("DIVIDE_ATTR", format!("{i}_5"))
                .commit();
        }
        for (attr, opt) in [
            ("CLK_FEEDBACK_2X", "IDLL*FB2X"),
            ("CFG_O_14", "IDLL*CFG_O_14"),
            ("LVL1_MUX_20", "IDLL*_ILVL1_MUX_20"),
            ("LVL1_MUX_21", "IDLL*_ILVL1_MUX_21"),
            ("LVL1_MUX_22", "IDLL*_ILVL1_MUX_22"),
            ("LVL1_MUX_23", "IDLL*_ILVL1_MUX_23"),
            ("LVL1_MUX_24", "IDLL*_ILVL1_MUX_24"),
        ] {
            for val in ["0", "1"] {
                // value "0" is apparently buggy and affects other DLLs than the one we're
                // aiming for, sometimes.
                //
                // have I mentioned I hate ISE?
                if attr == "LVL1_MUX_21" && val == "0" {
                    continue;
                }
                bctx.mode("DLL")
                    .global_mutex("DLL", "USE")
                    .prop(PinWireMutexShared(defs::bslots::DLL, "CLKIN"))
                    .prop(PinWireMutexShared(defs::bslots::DLL, "CLKFB"))
                    .test_manual_legacy(attr, val)
                    .prop(FuzzGlobalDll(defs::bslots::DLL, opt, val))
                    .commit();
            }
        }
        for (attr, opt) in [("TESTDLL", "TESTDLL*"), ("TESTZD2OSC", "TESTZD2OSC*")] {
            for val in ["NO", "YES"] {
                bctx.mode("DLL")
                    .global_mutex("DLL", "USE")
                    .test_manual_legacy(attr, val)
                    .prop(FuzzGlobalDll(defs::bslots::DLL, opt, val))
                    .commit();
            }
        }

        if !(tile.starts_with("DLLS") && backend.device.name.contains('v')) {
            if tile.ends_with("_S") {
                bctx.mode("DLL")
                    .global_mutex_here("DLL")
                    .prop(DeviceSide(DirH::W))
                    .extra_tile_reg_attr(Reg::Cor0, "REG.COR", "STARTUP", "DLL_WAIT_BL", "1")
                    .null_bits()
                    .test_manual_legacy("STARTUP_ATTR", "STARTUP_WAIT")
                    .attr("STARTUP_ATTR", "STARTUP_WAIT")
                    .commit();

                bctx.mode("DLL")
                    .global_mutex_here("DLL")
                    .prop(DeviceSide(DirH::E))
                    .extra_tile_reg_attr(Reg::Cor0, "REG.COR", "STARTUP", "DLL_WAIT_BR", "1")
                    .null_bits()
                    .test_manual_legacy("STARTUP_ATTR", "STARTUP_WAIT")
                    .attr("STARTUP_ATTR", "STARTUP_WAIT")
                    .commit();
            } else {
                bctx.mode("DLL")
                    .global_mutex_here("DLL")
                    .prop(DeviceSide(DirH::W))
                    .extra_tile_reg_attr(Reg::Cor0, "REG.COR", "STARTUP", "DLL_WAIT_TL", "1")
                    .null_bits()
                    .test_manual_legacy("STARTUP_ATTR", "STARTUP_WAIT")
                    .attr("STARTUP_ATTR", "STARTUP_WAIT")
                    .commit();
                bctx.mode("DLL")
                    .global_mutex_here("DLL")
                    .prop(DeviceSide(DirH::E))
                    .extra_tile_reg_attr(Reg::Cor0, "REG.COR", "STARTUP", "DLL_WAIT_TR", "1")
                    .null_bits()
                    .test_manual_legacy("STARTUP_ATTR", "STARTUP_WAIT")
                    .attr("STARTUP_ATTR", "STARTUP_WAIT")
                    .commit();
            }
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    for val in ["90", "180", "270", "360"] {
        ctx.build()
            .extra_tiles_by_bel(defs::bslots::DLL, "DLL")
            .test_manual("DLL", "TEST_OSC", val)
            .global("TESTOSC", val)
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["DLL_S", "DLL_N", "DLLP_S", "DLLP_N", "DLLS_S", "DLLS_N"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "DLL";

        let mut present = ctx.get_diff_legacy(tile, bel, "PRESENT", "1");

        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "DUTY_ATTR", "FALSE", "TRUE");
        present.apply_bitvec_diff_legacy(
            &item,
            &BitVec::repeat(true, 4),
            &BitVec::repeat(false, 4),
        );
        ctx.insert(tile, bel, "DUTY_CYCLE_CORRECTION", item);

        ctx.collect_bit_legacy(tile, bel, "HIGH_FREQUENCY", "1");

        let d0 = ctx.get_diff_legacy(tile, bel, "RSTMUX", "RST");
        assert_eq!(d0, ctx.get_diff_legacy(tile, bel, "RSTMUX", "1"));
        let d1 = ctx.get_diff_legacy(tile, bel, "RSTMUX", "RST_B");
        assert_eq!(d1, ctx.get_diff_legacy(tile, bel, "RSTMUX", "0"));
        let item = xlat_bit_bi_legacy(d0, d1);
        ctx.insert(tile, bel, "INV.RST", item);

        let item_jf2 =
            TileItem::from_bitvec_inv((0..8).map(|bit| TileBit::new(0, 17, bit)).collect(), false);
        let item_jf1 =
            TileItem::from_bitvec_inv((8..16).map(|bit| TileBit::new(0, 17, bit)).collect(), false);
        for (attr, item, base) in [
            ("JF_ZD2_ATTR", &item_jf2, 0x80),
            ("JF_ZD1_ATTR", &item_jf1, 0xc0),
        ] {
            for val in [0x80, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc, 0xfe, 0xff] {
                let mut diff = ctx.get_diff_legacy(tile, bel, attr, format!("0X{val:02X}"));
                diff.apply_bitvec_diff_int_legacy(item, val, base);
                diff.assert_empty();
            }
            present.apply_bitvec_diff_int_legacy(item, base, 0xf0);
        }
        ctx.insert(tile, bel, "FACTORY_JF2", item_jf2);
        ctx.insert(tile, bel, "FACTORY_JF1", item_jf1);

        let clkdv_count_max =
            TileItem::from_bitvec_inv((4..8).map(|bit| TileBit::new(0, 18, bit)).collect(), false);
        let clkdv_count_fall =
            TileItem::from_bitvec_inv((8..12).map(|bit| TileBit::new(0, 18, bit)).collect(), false);
        let clkdv_count_fall_2 = TileItem::from_bitvec_inv(
            (12..16).map(|bit| TileBit::new(0, 18, bit)).collect(),
            false,
        );
        let clkdv_phase_rise =
            TileItem::from_bitvec_inv((1..3).map(|bit| TileBit::new(0, 16, bit)).collect(), false);
        let clkdv_phase_fall =
            TileItem::from_bitvec_inv((3..5).map(|bit| TileBit::new(0, 16, bit)).collect(), false);
        let clkdv_mode = TileItem {
            bits: vec![TileBit::new(0, 16, 15)],
            kind: TileItemKind::Enum {
                values: BTreeMap::from_iter([
                    ("HALF".to_string(), bits![0]),
                    ("INT".to_string(), bits![1]),
                ]),
            },
        };
        for i in 2..=16 {
            let mut diff = ctx.get_diff_legacy(tile, bel, "DIVIDE_ATTR", format!("{i}"));
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, i - 1, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        for i in 1..=7 {
            let mut diff = ctx.get_diff_legacy(tile, bel, "DIVIDE_ATTR", format!("{i}_5.LOW"));
            diff.apply_enum_diff_legacy(&clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, i / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
            diff.assert_empty();
            let mut diff = ctx.get_diff_legacy(tile, bel, "DIVIDE_ATTR", format!("{i}_5.HIGH"));
            diff.apply_enum_diff_legacy(&clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        present.apply_bitvec_diff_int_legacy(&clkdv_count_max, 1, 0);
        present.apply_enum_diff_legacy(&clkdv_mode, "INT", "HALF");
        ctx.insert(tile, bel, "CLKDV_COUNT_MAX", clkdv_count_max);
        ctx.insert(tile, bel, "CLKDV_COUNT_FALL", clkdv_count_fall);
        ctx.insert(tile, bel, "CLKDV_COUNT_FALL_2", clkdv_count_fall_2);
        ctx.insert(tile, bel, "CLKDV_PHASE_RISE", clkdv_phase_rise);
        ctx.insert(tile, bel, "CLKDV_PHASE_FALL", clkdv_phase_fall);
        ctx.insert(tile, bel, "CLKDV_MODE", clkdv_mode);

        ctx.collect_bit_bi_legacy(tile, bel, "CFG_O_14", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "LVL1_MUX_20", "0", "1");
        ctx.collect_bit_legacy(tile, bel, "LVL1_MUX_21", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "LVL1_MUX_22", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "LVL1_MUX_23", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "LVL1_MUX_24", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "TESTZD2OSC", "NO", "YES");
        ctx.collect_bit_wide_bi_legacy(tile, bel, "TESTDLL", "NO", "YES");
        let item = xlat_enum_legacy(vec![
            ("1X", ctx.get_diff_legacy(tile, bel, "CLK_FEEDBACK_2X", "0")),
            ("2X", ctx.get_diff_legacy(tile, bel, "CLK_FEEDBACK_2X", "1")),
        ]);
        ctx.insert(tile, bel, "CLK_FEEDBACK", item);

        present.apply_bit_diff_legacy(ctx.item(tile, bel, "CFG_O_14"), true, false);
        if ctx.device.name.ends_with('e') {
            ctx.insert(tile, bel, "ENABLE", xlat_bit_legacy(present));
        } else {
            present.assert_empty();
        }
        ctx.collect_enum_legacy(tile, "DLL", "TEST_OSC", &["90", "180", "270", "360"]);
    }
    ctx.collect_bit_legacy("CNR_NW", "MISC", "DLL_ENABLE", "1");
    let tile = "REG.COR";
    let bel = "STARTUP";
    ctx.collect_bit_legacy(tile, bel, "DLL_WAIT_BL", "1");
    ctx.collect_bit_legacy(tile, bel, "DLL_WAIT_BR", "1");
    ctx.collect_bit_legacy(tile, bel, "DLL_WAIT_TL", "1");
    ctx.collect_bit_legacy(tile, bel, "DLL_WAIT_TR", "1");
}
