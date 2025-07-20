use std::collections::BTreeMap;

use prjcombine_interconnect::{
    dir::{DirH, DirHV, DirV},
    grid::TileCoord,
};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, extract_bitvec_val, extract_bitvec_val_part,
    xlat_bit, xlat_bit_wide, xlat_bitvec, xlat_bool, xlat_enum,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex2::{
    bels,
    chip::{ChipKind, ColumnKind},
    tslots,
};

use crate::{
    backend::{IseBackend, MultiValue, PinFromKind},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
};

#[derive(Copy, Clone, Debug)]
struct DcmCornerEnable(DirHV, bool);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DcmCornerEnable {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex2(edev) = backend.edev else {
            unreachable!()
        };
        let required = self.1;
        let we_match = match self.0.h {
            DirH::W => tcrd.col < edev.chip.col_clk,
            DirH::E => tcrd.col >= edev.chip.col_clk,
        };
        let sn_match = match self.0.v {
            DirV::S => tcrd.row < edev.chip.row_mid(),
            DirV::N => tcrd.row >= edev.chip.row_mid(),
        };
        if we_match && sn_match {
            let col = edev.chip.col_edge(self.0.h);
            let tcrd = tcrd.with_col(col).tile(tslots::BEL);
            fuzzer.info.features.push(FuzzerFeature {
                id: FeatureId {
                    tile: edev
                        .egrid
                        .db
                        .tile_classes
                        .key(edev.egrid[tcrd].class)
                        .clone(),
                    bel: "MISC".into(),
                    attr: "DCM_ENABLE".into(),
                    val: "1".into(),
                },
                tiles: edev.tile_bits(tcrd),
            });
            Some((fuzzer, false))
        } else if required {
            None
        } else {
            Some((fuzzer, true))
        }
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };
    let tile = match edev.chip.kind {
        ChipKind::Virtex2 => "DCM.V2",
        ChipKind::Virtex2P | ChipKind::Virtex2PX => "DCM.V2P",
        ChipKind::Spartan3 => "DCM.S3",
        _ => unreachable!(),
    };

    if devdata_only {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(bels::DCM);
        let mode = "DCM";
        let mut builder = bctx.build().global_mutex("DCM_OPT", "NO");
        if edev.chip.kind == ChipKind::Spartan3 {
            builder = builder.prop(DcmCornerEnable(DirHV::SW, true));
        }
        builder.test_manual("ENABLE", "1").mode(mode).commit();
        return;
    }

    let mut ctx = FuzzCtx::new_null(session, backend);
    for val in ["90", "180", "270", "360"] {
        ctx.build()
            .extra_tiles_by_bel(bels::DCM, "DCM")
            .test_manual("DCM", "TEST_OSC", val)
            .global("TESTOSC", val)
            .commit();
    }

    let mut ctx = FuzzCtx::new(session, backend, tile);
    let mut bctx = ctx.bel(bels::DCM);
    let mode = "DCM";
    let mut props = vec![];
    if edev.chip.kind == ChipKind::Spartan3 {
        props.extend([
            DcmCornerEnable(DirHV::SW, false),
            DcmCornerEnable(DirHV::NW, false),
        ]);
        if edev.chip.columns[edev.chip.columns.last_id().unwrap() - 3].kind == ColumnKind::Bram {
            props.extend([
                DcmCornerEnable(DirHV::SE, false),
                DcmCornerEnable(DirHV::NE, false),
            ]);
        }
    }
    let mut builder = bctx.build().global_mutex("DCM_OPT", "NO");
    for &prop in &props {
        builder = builder.prop(prop);
    }
    builder.test_manual("ENABLE", "1").mode(mode).commit();
    let mut builder = bctx
        .build()
        .global_mutex("DCM_OPT", "YES")
        .global("VBG_SEL0", "0")
        .global("VBG_SEL1", "0")
        .global("VBG_SEL2", "0")
        .global("VBG_PD0", "0")
        .global("VBG_PD1", "0");
    for &prop in &props {
        builder = builder.prop(prop);
    }
    builder
        .test_manual("ENABLE", "OPT_BASE")
        .mode(mode)
        .commit();

    for opt in ["VBG_SEL0", "VBG_SEL1", "VBG_SEL2", "VBG_PD0", "VBG_PD1"] {
        let mut builder = bctx
            .build()
            .global_mutex("DCM_OPT", "YES")
            .global("VBG_SEL0", if opt == "VBG_SEL0" { "1" } else { "0" })
            .global("VBG_SEL1", if opt == "VBG_SEL1" { "1" } else { "0" })
            .global("VBG_SEL2", if opt == "VBG_SEL2" { "1" } else { "0" })
            .global("VBG_PD0", if opt == "VBG_PD0" { "1" } else { "0" })
            .global("VBG_PD1", if opt == "VBG_PD1" { "1" } else { "0" });
        for &prop in &props {
            builder = builder.prop(prop);
        }
        builder.test_manual("ENABLE", opt).mode(mode).commit();
    }

    for pin in ["RST", "PSCLK", "PSEN", "PSINCDEC", "DSSEN"] {
        bctx.mode(mode)
            .global_mutex("PSCLK", "DCM")
            .mutex("MODE", "SIMPLE")
            .test_inv(pin);
    }
    for pin in [
        "CTLMODE",
        "CTLSEL0",
        "CTLSEL1",
        "CTLSEL2",
        "CTLOSC1",
        "CTLOSC2",
        "CTLGO",
        "STSADRS0",
        "STSADRS1",
        "STSADRS2",
        "STSADRS3",
        "STSADRS4",
        "FREEZEDFS",
        "FREEZEDLL",
    ] {
        if pin == "STSADRS4" && edev.chip.kind == ChipKind::Virtex2 {
            continue;
        }
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .mutex("INV", pin)
            .test_inv(pin);
    }

    for pin in [
        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
        "CONCUR",
    ] {
        bctx.mode(mode)
            .mutex("MODE", "PINS")
            .mutex("PIN", pin)
            .no_pin("CLKFB")
            .test_manual(pin, "1")
            .pin(pin)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "PINS")
            .mutex("PIN", pin)
            .pin("CLKFB")
            .test_manual(pin, "1.CLKFB")
            .pin(pin)
            .commit();
        if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
            bctx.mode(mode)
                .mutex("MODE", "PINS")
                .mutex("PIN", format!("{pin}.CLKFX"))
                .pin("CLKFX")
                .pin("CLKFB")
                .test_manual(pin, "1.CLKFX")
                .pin(pin)
                .commit();
        }
    }
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_manual("CLKFB", "1")
        .pin("CLKFB")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .pin("CLKIN")
        .pin("CLKFB")
        .pin_from("CLKFB", PinFromKind::Bufg)
        .test_manual("CLKIN_IOB", "1")
        .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .pin("CLKIN")
        .pin("CLKFB")
        .pin_from("CLKIN", PinFromKind::Bufg)
        .test_manual("CLKFB_IOB", "1")
        .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
        .commit();
    for pin in [
        "STATUS0", "STATUS1", "STATUS2", "STATUS3", "STATUS4", "STATUS5", "STATUS6", "STATUS7",
    ] {
        bctx.mode(mode)
            .mutex("MODE", "SIMPLE")
            .test_manual(pin, "1")
            .pin(pin)
            .commit();
    }

    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_enum("DLL_FREQUENCY_MODE", &["LOW", "HIGH"]);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_enum("DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .global("GTS_CYCLE", "1")
        .global("DONE_CYCLE", "1")
        .global("LCK_CYCLE", "NOWAIT")
        .test_enum("STARTUP_WAIT", &["STARTUP_WAIT"]);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_enum("DUTY_CYCLE_CORRECTION", &["FALSE", "TRUE"]);
    bctx.mode(mode).mutex("MODE", "SIMPLE").test_enum(
        "FACTORY_JF1",
        &[
            "0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF",
        ],
    );
    bctx.mode(mode).mutex("MODE", "SIMPLE").test_enum(
        "FACTORY_JF2",
        &[
            "0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF",
        ],
    );
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_manual("DESKEW_ADJUST", "")
        .multi_attr("DESKEW_ADJUST", MultiValue::Dec(0), 4);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_enum("CLKIN_DIVIDE_BY_2", &["CLKIN_DIVIDE_BY_2"]);
    bctx.mode(mode)
        .attr("DUTY_CYCLE_CORRECTION", "#OFF")
        .mutex("MODE", "SIMPLE")
        .pin("CLK0")
        .test_enum("VERY_HIGH_FREQUENCY", &["VERY_HIGH_FREQUENCY"]);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "NONE")
        .test_enum(
            "DSS_MODE",
            &["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"],
        );
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_enum("CLK_FEEDBACK", &["1X", "2X"]);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("PHASE_SHIFT", "1")
        .pin("CLK0")
        .test_enum("CLKOUT_PHASE_SHIFT", &["NONE", "FIXED", "VARIABLE"]);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("PHASE_SHIFT", "-1")
        .pin("CLK0")
        .test_manual("CLKOUT_PHASE_SHIFT", "FIXED.NEG")
        .attr("CLKOUT_PHASE_SHIFT", "FIXED")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("PHASE_SHIFT", "-1")
        .pin("CLK0")
        .test_manual("CLKOUT_PHASE_SHIFT", "VARIABLE.NEG")
        .attr("CLKOUT_PHASE_SHIFT", "VARIABLE")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_manual("CLKFX_MULTIPLY", "")
        .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 12);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .test_manual("CLKFX_DIVIDE", "")
        .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 12);

    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "FIXED")
        .test_manual("PHASE_SHIFT", "")
        .multi_attr("PHASE_SHIFT", MultiValue::Dec(0), 8);
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "FIXED")
        .test_manual("PHASE_SHIFT", "-255.FIXED")
        .attr("PHASE_SHIFT", "-255")
        .commit();
    bctx.mode(mode)
        .mutex("MODE", "SIMPLE")
        .attr("CLKOUT_PHASE_SHIFT", "VARIABLE")
        .test_manual("PHASE_SHIFT", "-255.VARIABLE")
        .attr("PHASE_SHIFT", "-255")
        .commit();

    bctx.mode(mode).mutex("MODE", "SIMPLE").test_enum(
        "CLKDV_DIVIDE",
        &[
            "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        ],
    );
    for dll_mode in ["LOW", "HIGH"] {
        for val in ["1_5", "2_5", "3_5", "4_5", "5_5", "6_5", "7_5"] {
            bctx.mode(mode)
                .mutex("MODE", "SIMPLE")
                .attr("DLL_FREQUENCY_MODE", dll_mode)
                .test_manual("CLKDV_DIVIDE", format!("{val}.{dll_mode}"))
                .attr("CLKDV_DIVIDE", val)
                .commit();
        }
    }

    bctx.mode(mode)
        .mutex("MODE", "LL_DLLC")
        .no_global("TESTOSC")
        .pin("STATUS1")
        .pin("STATUS7")
        .test_manual("DLLC", "")
        .multi_attr("LL_HEX_DLLC", MultiValue::Hex(0), 32);
    bctx.mode(mode)
        .mutex("MODE", "LL_DLLS")
        .test_manual("DLLS", "")
        .multi_attr("LL_HEX_DLLS", MultiValue::Hex(0), 32);
    bctx.mode(mode)
        .mutex("MODE", "LL_DFS")
        .test_manual("DFS", "")
        .multi_attr("LL_HEX_DFS", MultiValue::Hex(0), 32);
    bctx.mode(mode)
        .mutex("MODE", "LL_COM")
        .test_manual("COM", "")
        .multi_attr("LL_HEX_COM", MultiValue::Hex(0), 32);
    bctx.mode(mode)
        .mutex("MODE", "LL_MISC")
        .test_manual("MISC", "")
        .multi_attr("LL_HEX_MISC", MultiValue::Hex(0), 32);
    for val in ["0", "1", "2", "3"] {
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .test_manual("COIN_WINDOW", val)
            .global_xy("COINWINDOW_*", val)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .test_manual("SEL_PL_DLY", val)
            .global_xy("SELPLDLY_*", val)
            .commit();
    }
    for val in ["0", "1"] {
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .test_manual("EN_OSC_COARSE", val)
            .global_xy("ENOSCCOARSE_*", val)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .global_xy("NONSTOP_*", "0")
            .test_manual("EN_DUMMY_OSC", val)
            .global_xy("ENDUMMYOSC_*", val)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .test_manual("PL_CENTERED", val)
            .global_xy("PLCENTERED_*", val)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .global_xy("ENDUMMYOSC_*", "0")
            .test_manual("NON_STOP", val)
            .global_xy("NONSTOP_*", val)
            .commit();
        bctx.mode(mode)
            .mutex("MODE", "GLOBALS")
            .mutex("ZD2", "PLAIN")
            .test_manual("ZD2_BY1", val)
            .global_xy("ZD2_BY1_*", val)
            .commit();
        if edev.chip.kind.is_virtex2() {
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("PS_CENTERED", val)
                .global_xy("CENTERED_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .mutex("ZD2", "HF")
                .test_manual("ZD2_HF_BY1", val)
                .global_xy("ZD2_HF_BY1_*", val)
                .commit();
        }
        if edev.chip.kind != ChipKind::Virtex2 {
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("ZD1_BY1", val)
                .global_xy("ZD1_BY1_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("RESET_PS_SEL", val)
                .global_xy("RESETPS_SEL_*", val)
                .commit();
        }
        if edev.chip.kind == ChipKind::Spartan3 {
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("SPLY_IDC0", val)
                .global_xy("SPLY_IDC0_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("SPLY_IDC1", val)
                .global_xy("SPLY_IDC1_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("EXTENDED_FLUSH_TIME", val)
                .global_xy("EXTENDEDFLUSHTIME_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("EXTENDED_HALT_TIME", val)
                .global_xy("EXTENDEDHALTTIME_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("EXTENDED_RUN_TIME", val)
                .global_xy("EXTENDEDRUNTIME_*", val)
                .commit();
            for i in 0..=8 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_manual(format!("CFG_DLL_PS{i}"), val)
                    .global_xy(format!("CFG_DLL_PS{i}_*"), val)
                    .commit();
            }
            for i in 0..=2 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_manual(format!("CFG_DLL_LP{i}"), val)
                    .global_xy(format!("CFG_DLL_LP{i}_*"), val)
                    .commit();
            }
            for i in 0..=1 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_manual(format!("SEL_HSYNC_B{i}"), val)
                    .global_xy(format!("SELHSYNC_B{i}_*"), val)
                    .commit();
            }
            for i in 0..=1 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_manual(format!("LPON_B_DFS{i}"), val)
                    .global_xy(format!("LPON_B_DFS{i}_*"), val)
                    .commit();
            }
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("EN_PWCTL", val)
                .global_xy("ENPWCTL_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("M1D1", val)
                .global_xy("M1D1_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("MIS1", val)
                .global_xy("MIS1_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("EN_RELRST_B", val)
                .global_xy("ENRELRST_B_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("EN_OLD_OSCCTL", val)
                .global_xy("ENOLDOSCCTL_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("TRIM_LP_B", val)
                .global_xy("TRIM_LP_B_*", val)
                .commit();
            bctx.mode(mode)
                .mutex("MODE", "GLOBALS")
                .test_manual("INVERT_ZD1_CUSTOM", val)
                .global_xy("INVERT_ZD1_CUSTOM_*", val)
                .commit();
            for i in 0..=4 {
                bctx.mode(mode)
                    .mutex("MODE", "GLOBALS")
                    .test_manual(format!("VREG_PROBE{i}"), val)
                    .global_xy(format!("VREG_PROBE{i}_*"), val)
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let tile = match edev.chip.kind {
        ChipKind::Virtex2 => "DCM.V2",
        ChipKind::Virtex2P | ChipKind::Virtex2PX => "DCM.V2P",
        ChipKind::Spartan3 => "DCM.S3",
        _ => unreachable!(),
    };
    let bel = "DCM";

    if devdata_only {
        let mut present = ctx.state.get_diff(tile, bel, "ENABLE", "1");
        let item = ctx.tiledb.item(tile, bel, "DESKEW_ADJUST");
        let val = extract_bitvec_val(
            item,
            &BitVec::repeat(false, 4),
            present.split_bits(&item.bits.iter().copied().collect()),
        );
        ctx.insert_device_data("DCM:DESKEW_ADJUST", val);
        let vbg_sel = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "VBG_SEL"),
            &BitVec::repeat(false, 3),
            &mut present,
        );
        let vbg_pd = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "VBG_PD"),
            &BitVec::repeat(false, 2),
            &mut present,
        );
        ctx.insert_device_data("DCM:VBG_SEL", vbg_sel);
        ctx.insert_device_data("DCM:VBG_PD", vbg_pd);
        if edev.chip.kind == ChipKind::Spartan3 {
            ctx.collect_bit("LL.S3", "MISC", "DCM_ENABLE", "1");
        }
        return;
    }

    let mut present = ctx.state.get_diff(tile, bel, "ENABLE", "1");
    let dllc = ctx.state.get_diffs(tile, bel, "DLLC", "");
    let dlls = ctx.state.get_diffs(tile, bel, "DLLS", "");
    let dfs = ctx.state.get_diffs(tile, bel, "DFS", "");
    let mut com = ctx.state.get_diffs(tile, bel, "COM", "");
    let mut misc = ctx.state.get_diffs(tile, bel, "MISC", "");

    // sigh. fixups.
    assert!(com[11].bits.is_empty());
    let com9 = *com[9].bits.keys().next().unwrap();
    let com11 = TileBit {
        bit: com9.bit + 2,
        ..com9
    };
    assert_eq!(com[10].bits.remove(&com11), Some(true));
    com[11].bits.insert(com11, true);

    if edev.chip.kind == ChipKind::Spartan3 {
        for diff in &misc[12..31] {
            assert!(diff.bits.is_empty());
        }
        misc.truncate(12);
    }

    let dllc = xlat_bitvec(dllc);
    let dlls = xlat_bitvec(dlls);
    let dfs = xlat_bitvec(dfs);
    let com = xlat_bitvec(com);
    let misc = xlat_bitvec(misc);

    let base = ctx.state.get_diff(tile, bel, "ENABLE", "OPT_BASE");
    for (attr, len) in [("VBG_SEL", 3), ("VBG_PD", 2)] {
        let mut diffs = vec![];
        for bit in 0..len {
            diffs.push(
                ctx.state
                    .get_diff(tile, bel, "ENABLE", format!("{attr}{bit}"))
                    .combine(&!&base),
            );
        }
        ctx.tiledb.insert(tile, bel, attr, xlat_bitvec(diffs));
    }
    ctx.collect_enum(tile, bel, "TEST_OSC", &["90", "180", "270", "360"]);

    ctx.collect_enum(tile, bel, "COIN_WINDOW", &["0", "1", "2", "3"]);
    ctx.collect_enum(tile, bel, "SEL_PL_DLY", &["0", "1", "2", "3"]);
    ctx.collect_enum_bool(tile, bel, "EN_OSC_COARSE", "0", "1");
    ctx.collect_enum_bool(tile, bel, "PL_CENTERED", "0", "1");
    if edev.chip.kind.is_virtex2() {
        ctx.state
            .get_diff(tile, bel, "NON_STOP", "0")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "EN_DUMMY_OSC", "1")
            .assert_empty();
        let en_dummy_osc = !ctx.state.get_diff(tile, bel, "EN_DUMMY_OSC", "0");
        let non_stop = ctx.state.get_diff(tile, bel, "NON_STOP", "1");
        let (en_dummy_osc, non_stop, common) = Diff::split(en_dummy_osc, non_stop);
        ctx.tiledb.insert(tile, bel, "NON_STOP", xlat_bit(non_stop));
        ctx.tiledb
            .insert(tile, bel, "EN_DUMMY_OSC", xlat_bit_wide(en_dummy_osc));
        ctx.tiledb
            .insert(tile, bel, "EN_DUMMY_OSC_OR_NON_STOP", xlat_bit(common));
    } else {
        ctx.collect_enum_bool(tile, bel, "EN_DUMMY_OSC", "0", "1");
        ctx.collect_enum_bool(tile, bel, "NON_STOP", "0", "1");
    }
    ctx.collect_enum_bool(tile, bel, "ZD2_BY1", "0", "1");
    if edev.chip.kind.is_virtex2() {
        ctx.collect_enum_bool(tile, bel, "PS_CENTERED", "0", "1");
        let item = ctx.extract_enum_bool(tile, bel, "ZD2_HF_BY1", "0", "1");
        assert_eq!(item, *ctx.tiledb.item(tile, bel, "ZD2_BY1"));
    }
    if edev.chip.kind != ChipKind::Virtex2 {
        ctx.collect_enum_bool(tile, bel, "ZD1_BY1", "0", "1");
        ctx.collect_enum_bool(tile, bel, "RESET_PS_SEL", "0", "1");
    }
    if edev.chip.kind == ChipKind::Spartan3 {
        ctx.collect_enum_bool(tile, bel, "EXTENDED_FLUSH_TIME", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EXTENDED_HALT_TIME", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EXTENDED_RUN_TIME", "0", "1");
        ctx.collect_enum_bool(tile, bel, "M1D1", "0", "1");
        ctx.collect_enum_bool(tile, bel, "MIS1", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EN_OLD_OSCCTL", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EN_PWCTL", "0", "1");
        ctx.collect_enum_bool(tile, bel, "EN_RELRST_B", "0", "1");
        ctx.collect_enum_bool(tile, bel, "INVERT_ZD1_CUSTOM", "0", "1");
        ctx.collect_enum_bool(tile, bel, "TRIM_LP_B", "0", "1");

        for (attr, len) in [
            ("SPLY_IDC", 2),
            ("VREG_PROBE", 5),
            ("CFG_DLL_PS", 9),
            ("CFG_DLL_LP", 3),
            ("SEL_HSYNC_B", 2),
            ("LPON_B_DFS", 2),
        ] {
            let mut diffs = vec![];
            for i in 0..len {
                let d0 = ctx.state.get_diff(tile, bel, format!("{attr}{i}"), "0");
                let d1 = ctx.state.get_diff(tile, bel, format!("{attr}{i}"), "1");
                if d0.bits.is_empty() {
                    diffs.push(d1);
                } else {
                    diffs.push(!d0);
                    d1.assert_empty();
                }
            }
            ctx.tiledb.insert(tile, bel, attr, xlat_bitvec(diffs));
        }
    }

    let int_tiles = &[match edev.chip.kind {
        ChipKind::Virtex2 => "INT.DCM.V2",
        ChipKind::Virtex2P | ChipKind::Virtex2PX => "INT.DCM.V2P",
        ChipKind::Spartan3 => "INT.DCM",
        _ => unreachable!(),
    }];
    ctx.collect_int_inv(int_tiles, tile, bel, "PSCLK", false);
    for pin in ["RST", "PSEN", "PSINCDEC"] {
        ctx.collect_inv(tile, bel, pin);
    }
    if edev.chip.kind == ChipKind::Spartan3 {
        ctx.state
            .get_diff(tile, bel, "DSSENINV", "DSSEN")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "DSSENINV", "DSSEN_B")
            .assert_empty();
    } else {
        ctx.collect_inv(tile, bel, "DSSEN");
    }
    for pin in [
        "CTLMODE",
        "CTLSEL0",
        "CTLSEL1",
        "CTLSEL2",
        "CTLOSC1",
        "CTLOSC2",
        "CTLGO",
        "STSADRS0",
        "STSADRS1",
        "STSADRS2",
        "STSADRS3",
        "STSADRS4",
        "FREEZEDFS",
        "FREEZEDLL",
    ] {
        if pin == "STSADRS4" && edev.chip.kind == ChipKind::Virtex2 {
            continue;
        }
        let d0 = ctx.state.get_diff(tile, bel, format!("{pin}INV"), pin);
        let d1 = ctx
            .state
            .get_diff(tile, bel, format!("{pin}INV"), format!("{pin}_B"));
        let (d0, d1, dc) = Diff::split(d0, d1);
        ctx.tiledb
            .insert(tile, bel, format!("INV.{pin}"), xlat_bool(d0, d1));
        if edev.chip.kind.is_virtex2() {
            ctx.tiledb.insert(tile, bel, "TEST_ENABLE", xlat_bit(dc));
        } else {
            dc.assert_empty();
        }
    }
    for pin in [
        "STATUS0", "STATUS2", "STATUS3", "STATUS4", "STATUS5", "STATUS6",
    ] {
        ctx.state.get_diff(tile, bel, pin, "1").assert_empty();
    }
    for pin in ["STATUS1", "STATUS7"] {
        ctx.collect_bit(tile, bel, pin, "1");
    }
    let (_, _, en_dll) = Diff::split(
        ctx.state.peek_diff(tile, bel, "CLK0", "1").clone(),
        ctx.state.peek_diff(tile, bel, "CLK90", "1").clone(),
    );
    let (_, _, en_dfs) = Diff::split(
        ctx.state.peek_diff(tile, bel, "CLKFX", "1").clone(),
        ctx.state.peek_diff(tile, bel, "CLKFX180", "1").clone(),
    );
    let vhf = ctx
        .state
        .get_diff(tile, bel, "VERY_HIGH_FREQUENCY", "VERY_HIGH_FREQUENCY");
    assert_eq!(en_dll, !vhf);
    for pin in [
        "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
    ] {
        let diff = ctx.state.get_diff(tile, bel, pin, "1");
        let diff_fb = ctx.state.get_diff(tile, bel, pin, "1.CLKFB");
        let diff_fx = ctx.state.get_diff(tile, bel, pin, "1.CLKFX");
        assert_eq!(diff, diff_fb);
        assert_eq!(diff, diff_fx);
        let diff = diff.combine(&!&en_dll);
        ctx.tiledb
            .insert(tile, bel, format!("ENABLE.{pin}"), xlat_bit(diff));
    }
    for pin in ["CLKFX", "CLKFX180", "CONCUR"] {
        let diff = ctx.state.get_diff(tile, bel, pin, "1");
        let diff_fb = ctx.state.get_diff(tile, bel, pin, "1.CLKFB");
        let diff_fb = diff_fb.combine(&!&diff);
        let diff = diff.combine(&!&en_dfs);
        ctx.tiledb
            .insert(tile, bel, format!("ENABLE.{pin}"), xlat_bit(diff));
        ctx.tiledb
            .insert(tile, bel, "DFS_FEEDBACK", xlat_bit(diff_fb));
    }
    ctx.tiledb.insert(tile, bel, "DLL_ENABLE", xlat_bit(en_dll));
    ctx.tiledb.insert(tile, bel, "DFS_ENABLE", xlat_bit(en_dfs));
    let item = ctx.extract_bit(tile, bel, "CLKFB", "1");
    ctx.tiledb.insert(tile, bel, "ENABLE.CLKFB", item);

    ctx.collect_bit(tile, bel, "CLKIN_IOB", "1");
    ctx.collect_bit(tile, bel, "CLKFB_IOB", "1");

    ctx.collect_bitvec(tile, bel, "CLKFX_MULTIPLY", "");
    ctx.collect_bitvec(tile, bel, "CLKFX_DIVIDE", "");
    ctx.collect_bitvec(tile, bel, "DESKEW_ADJUST", "");
    ctx.collect_bitvec(tile, bel, "PHASE_SHIFT", "");
    let mut diff = ctx
        .state
        .get_diff(tile, bel, "PHASE_SHIFT", "-255.VARIABLE");
    diff.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "PHASE_SHIFT"),
        &BitVec::repeat(true, 8),
        &BitVec::repeat(false, 8),
    );
    ctx.tiledb
        .insert(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bit(diff));

    ctx.state
        .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "NONE")
        .assert_empty();
    let fixed = ctx.state.get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "FIXED");
    let fixed_n = ctx
        .state
        .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "FIXED.NEG");
    let variable = ctx
        .state
        .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "VARIABLE");
    let variable_n = ctx
        .state
        .get_diff(tile, bel, "CLKOUT_PHASE_SHIFT", "VARIABLE.NEG");
    assert_eq!(variable, variable_n);
    let fixed_n = fixed_n.combine(&!&fixed);
    let (fixed, variable, en_ps) = Diff::split(fixed, variable);
    ctx.tiledb.insert(tile, bel, "PS_ENABLE", xlat_bit(en_ps));
    ctx.tiledb
        .insert(tile, bel, "PS_CENTERED", xlat_bool(fixed, variable));

    let mut diff = ctx.state.get_diff(tile, bel, "PHASE_SHIFT", "-255.FIXED");
    diff.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "PHASE_SHIFT"),
        &BitVec::repeat(true, 8),
        &BitVec::repeat(false, 8),
    );
    diff.apply_bit_diff(
        ctx.tiledb.item(tile, bel, "PHASE_SHIFT_NEGATIVE"),
        true,
        false,
    );
    assert_eq!(diff, fixed_n);
    if edev.chip.kind != ChipKind::Virtex2 {
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "RESET_PS_SEL"), true, false);
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "PS_MODE",
        xlat_enum(vec![("CLKFB", diff), ("CLKIN", Diff::default())]),
    );

    ctx.collect_enum_bool_wide(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE", "TRUE");
    ctx.collect_bit(tile, bel, "STARTUP_WAIT", "STARTUP_WAIT");
    ctx.collect_bit(tile, bel, "CLKIN_DIVIDE_BY_2", "CLKIN_DIVIDE_BY_2");
    ctx.collect_enum(tile, bel, "CLK_FEEDBACK", &["1X", "2X"]);
    ctx.collect_enum(tile, bel, "DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
    let low = ctx.state.get_diff(tile, bel, "DLL_FREQUENCY_MODE", "LOW");
    let mut high = ctx.state.get_diff(tile, bel, "DLL_FREQUENCY_MODE", "HIGH");
    if edev.chip.kind.is_virtex2p() {
        high.apply_bit_diff(ctx.tiledb.item(tile, bel, "ZD2_BY1"), true, false);
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "DLL_FREQUENCY_MODE",
        xlat_enum(vec![("LOW", low), ("HIGH", high)]),
    );

    let mut jf1 = ctx.extract_enum(
        tile,
        bel,
        "FACTORY_JF1",
        &[
            "0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF",
        ],
    );
    jf1.bits.reverse();
    assert_eq!(jf1.bits, dlls.bits[8..15]);
    let mut jf2 = ctx.extract_enum(
        tile,
        bel,
        "FACTORY_JF2",
        &[
            "0X80", "0XC0", "0XE0", "0XF0", "0XF8", "0XFC", "0XFE", "0XFF",
        ],
    );
    jf2.bits.reverse();
    assert_eq!(jf2.bits, dlls.bits[0..7]);
    assert_eq!(jf2.kind, jf1.kind);
    let TileItemKind::Enum { values } = jf2.kind else {
        unreachable!()
    };
    assert_eq!(values["0X80"], bits![0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(values["0XC0"], bits![1, 0, 0, 0, 0, 0, 0]);
    assert_eq!(values["0XE0"], bits![1, 1, 0, 0, 0, 0, 0]);
    assert_eq!(values["0XF0"], bits![1, 1, 1, 0, 0, 0, 0]);
    assert_eq!(values["0XF8"], bits![1, 1, 1, 1, 0, 0, 0]);
    assert_eq!(values["0XFC"], bits![1, 1, 1, 1, 1, 0, 0]);
    assert_eq!(values["0XFE"], bits![1, 1, 1, 1, 1, 1, 0]);
    assert_eq!(values["0XFF"], bits![1, 1, 1, 1, 1, 1, 1]);
    jf1.bits.push(dlls.bits[15]);
    jf2.bits.push(dlls.bits[7]);
    jf1.kind = TileItemKind::BitVec {
        invert: BitVec::repeat(false, 8),
    };
    jf2.kind = TileItemKind::BitVec {
        invert: BitVec::repeat(false, 8),
    };
    ctx.tiledb.insert(tile, bel, "FACTORY_JF1", jf1);
    ctx.tiledb.insert(tile, bel, "FACTORY_JF2", jf2);

    for (attr, bits) in [
        ("CLKDV_COUNT_MAX", &dllc.bits[4..8]),
        ("CLKDV_COUNT_FALL", &dllc.bits[8..12]),
        ("CLKDV_COUNT_FALL_2", &dllc.bits[12..16]),
        ("CLKDV_PHASE_RISE", &dllc.bits[16..18]),
        ("CLKDV_PHASE_FALL", &dllc.bits[18..20]),
    ] {
        ctx.tiledb.insert(
            tile,
            bel,
            attr,
            TileItem {
                bits: bits.to_vec(),
                kind: TileItemKind::BitVec {
                    invert: BitVec::repeat(false, bits.len()),
                },
            },
        );
    }
    ctx.tiledb.insert(
        tile,
        bel,
        "CLKDV_MODE",
        TileItem {
            bits: dllc.bits[20..21].to_vec(),
            kind: TileItemKind::Enum {
                values: BTreeMap::from_iter([
                    ("HALF".to_string(), bits![0]),
                    ("INT".to_string(), bits![1]),
                ]),
            },
        },
    );

    let clkdv_count_max = ctx.collector.tiledb.item(tile, bel, "CLKDV_COUNT_MAX");
    let clkdv_count_fall = ctx.collector.tiledb.item(tile, bel, "CLKDV_COUNT_FALL");
    let clkdv_count_fall_2 = ctx.collector.tiledb.item(tile, bel, "CLKDV_COUNT_FALL_2");
    let clkdv_phase_fall = ctx.collector.tiledb.item(tile, bel, "CLKDV_PHASE_FALL");
    let clkdv_mode = ctx.collector.tiledb.item(tile, bel, "CLKDV_MODE");
    for i in 2..=16 {
        let mut diff = ctx
            .collector
            .state
            .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}"));
        diff.apply_bitvec_diff_int(clkdv_count_max, i - 1, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }
    for i in 1..=7 {
        let mut diff =
            ctx.collector
                .state
                .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}_5.LOW"));
        diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
        diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, i / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_count_fall_2, 3 * i / 2 + 1, 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2 + 1, 0);
        diff.assert_empty();
        let mut diff =
            ctx.collector
                .state
                .get_diff(tile, bel, "CLKDV_DIVIDE", format!("{i}_5.HIGH"));
        diff.apply_enum_diff(clkdv_mode, "HALF", "INT");
        diff.apply_bitvec_diff_int(clkdv_count_max, 2 * i, 1);
        diff.apply_bitvec_diff_int(clkdv_count_fall, (i - 1) / 2, 0);
        diff.apply_bitvec_diff_int(clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
        diff.apply_bitvec_diff_int(clkdv_phase_fall, (i % 2) * 2, 0);
        diff.assert_empty();
    }

    if edev.chip.kind.is_virtex2() {
        ctx.state
            .get_diff(tile, bel, "DSS_MODE", "NONE")
            .assert_empty();
        let mut dss_base = ctx
            .state
            .peek_diff(tile, bel, "DSS_MODE", "SPREAD_2")
            .clone();
        let mut diffs = vec![];
        for val in ["SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"] {
            diffs.push((
                val,
                ctx.state
                    .get_diff(tile, bel, "DSS_MODE", val)
                    .combine(&!&dss_base),
            ));
        }
        ctx.tiledb.insert(tile, bel, "DSS_MODE", xlat_enum(diffs));
        dss_base.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_ENABLE"), true, false);
        dss_base.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_CENTERED"), true, false);
        ctx.tiledb
            .insert(tile, bel, "DSS_ENABLE", xlat_bit(dss_base));
    } else {
        for val in ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"] {
            ctx.state
                .get_diff(tile, bel, "DSS_MODE", val)
                .assert_empty();
        }
    }

    ctx.tiledb.insert(tile, bel, "DLLC", dllc);
    ctx.tiledb.insert(tile, bel, "DLLS", dlls);
    ctx.tiledb.insert(tile, bel, "DFS", dfs);
    ctx.tiledb.insert(tile, bel, "COM", com);
    ctx.tiledb.insert(tile, bel, "MISC", misc);

    present.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "FACTORY_JF2"),
        &bits![0, 0, 0, 0, 0, 0, 0, 1],
        &bits![0, 0, 0, 0, 0, 0, 0, 0],
    );
    present.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "FACTORY_JF1"),
        &bits![0, 0, 0, 0, 0, 0, 1, 1],
        &bits![0, 0, 0, 0, 0, 0, 0, 0],
    );
    let vbg_sel = extract_bitvec_val_part(
        ctx.tiledb.item(tile, bel, "VBG_SEL"),
        &BitVec::repeat(false, 3),
        &mut present,
    );
    let vbg_pd = extract_bitvec_val_part(
        ctx.tiledb.item(tile, bel, "VBG_PD"),
        &BitVec::repeat(false, 2),
        &mut present,
    );
    ctx.insert_device_data("DCM:VBG_SEL", vbg_sel);
    ctx.insert_device_data("DCM:VBG_PD", vbg_pd);
    for attr in ["CLKFX_MULTIPLY", "CLKFX_DIVIDE"] {
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, attr),
            &BitVec::repeat(true, 12),
            &BitVec::repeat(false, 12),
        );
    }

    let item = ctx.tiledb.item(tile, bel, "DESKEW_ADJUST");
    let val = extract_bitvec_val(
        item,
        &BitVec::repeat(false, 4),
        present.split_bits(&item.bits.iter().copied().collect()),
    );
    ctx.insert_device_data("DCM:DESKEW_ADJUST", val);

    present.apply_bitvec_diff(
        ctx.tiledb.item(tile, bel, "DUTY_CYCLE_CORRECTION"),
        &BitVec::repeat(true, 4),
        &BitVec::repeat(false, 4),
    );

    if edev.chip.kind.is_virtex2() {
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.DSSEN"), false, true);
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "EN_OSC_COARSE"), true, false);
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "EN_DUMMY_OSC"),
            &bits![1, 1, 1],
            &bits![0, 0, 0],
        );
        present.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "EN_DUMMY_OSC_OR_NON_STOP"),
            true,
            false,
        );
        if !edev.chip.kind.is_virtex2p() {
            present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ZD2_BY1"), true, false);
        }
    } else {
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "EN_PWCTL"), true, false);
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "SEL_HSYNC_B"),
            &bits![0, 1],
            &bits![0, 0],
        );
        present.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "CFG_DLL_PS"),
            &bits![0, 1, 1, 0, 1, 0, 0, 1, 0],
            &BitVec::repeat(false, 9),
        );
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ZD1_BY1"), true, false);
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "ZD2_BY1"), true, false);
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "EN_DUMMY_OSC"), true, false);
    }
    present.discard_bits(ctx.tiledb.item(tile, bel, "PS_MODE"));
    if edev.chip.kind == ChipKind::Spartan3 {
        present.apply_bit_diff(ctx.tiledb.item(tile, bel, "PS_CENTERED"), true, false);
    }
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "CLKDV_COUNT_MAX"), 1, 0);
    present.apply_enum_diff(ctx.tiledb.item(tile, bel, "CLKDV_MODE"), "INT", "HALF");

    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "MISC"), 1, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "DFS"), 1 << 26, 0);
    present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "COM"), 0x800a0a, 0);

    present.assert_empty();

    if edev.chip.kind == ChipKind::Spartan3 {
        ctx.collect_bit("LL.S3", "MISC", "DCM_ENABLE", "1");
        ctx.collect_bit("UL.S3", "MISC", "DCM_ENABLE", "1");
        if edev.chip.columns[edev.chip.columns.last_id().unwrap() - 3].kind == ColumnKind::Bram {
            ctx.collect_bit("LR.S3", "MISC", "DCM_ENABLE", "1");
            ctx.collect_bit("UR.S3", "MISC", "DCM_ENABLE", "1");
        }
    }
}
