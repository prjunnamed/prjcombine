use std::collections::{BTreeMap, HashMap};

use prjcombine_interconnect::{
    db::{BelInfo, SwitchBoxItem, TileWireCoord},
    grid::TileCoord,
};
use prjcombine_re_collector::{
    diff::{Diff, DiffKey, OcdMode, SpecialId, xlat_bit, xlat_bitvec, xlat_enum_raw},
    legacy::{xlat_bit_legacy, xlat_bitvec_legacy, xlat_enum_default_legacy},
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_spartan6::defs::{self, bcls, bslots, tcls, tslots, wires};
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{TileItem, TileItemKind},
};

use crate::{
    backend::{IseBackend, MultiValue, PinFromKind},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::FuzzIntPip,
        props::{
            DynProp,
            mutex::{WireMutexExclusive, WireMutexShared},
            pip::PipWire,
            relation::DeltaSlot,
        },
    },
    spartan6::specials,
};

#[derive(Copy, Clone, Debug)]
struct AllOtherDcms(SpecialId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for AllOtherDcms {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tcid = tcls::CMT_DCM;
        for &ntcrd in &backend.edev.tile_index[tcid] {
            if tcrd == ntcrd {
                continue;
            }
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelSpecial(tcls::CMT_DCM, bslots::CMT_VREG, self.0),
                rects: backend.edev.tile_bits(ntcrd),
            });
        }

        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let tcid = tcls::CMT_DCM;
    let mut ctx = FuzzCtx::new(session, backend, tcid);
    let tcls = &backend.edev.db[tcid];
    let BelInfo::SwitchBox(ref sb) = tcls.bels[bslots::CMT_INT] else {
        unreachable!()
    };
    let mut muxes = HashMap::new();
    for item in &sb.items {
        let SwitchBoxItem::Mux(mux) = item else {
            continue;
        };
        muxes.insert(mux.dst, mux);
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::DCM[i]);
        bctx.build()
            .global_mutex("CMT", "PRESENT")
            .global_mutex_here("CMT_PRESENT")
            .prop(AllOtherDcms(specials::CMT_PRESENT_ANY_DCM))
            .test_bel_special(specials::PRESENT)
            .mode("DCM")
            .commit();
        bctx.build()
            .global_mutex("CMT", "PRESENT")
            .global_mutex_here("CMT_PRESENT")
            .prop(AllOtherDcms(specials::CMT_PRESENT_ANY_DCM))
            .test_bel_special(specials::DCM_CLKGEN)
            .mode("DCM_CLKGEN")
            .commit();

        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("DLL_C", "")
            .multi_global_xy("CFG_DLL_C_*", MultiValue::Bin, 32);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("DLL_S", "")
            .multi_global_xy("CFG_DLL_S_*", MultiValue::Bin, 32);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("DFS_C", "")
            .multi_global_xy("CFG_DFS_C_*", MultiValue::Bin, 3);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("DFS_S", "")
            .multi_global_xy("CFG_DFS_S_*", MultiValue::Bin, 87);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("INTERFACE", "")
            .multi_global_xy("CFG_INTERFACE_*", MultiValue::Bin, 40);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_manual_legacy("OPT_INV", "")
            .multi_global_xy("CFG_OPT_INV_*", MultiValue::Bin, 20);
        bctx.mode("DCM")
            .global_mutex("CMT", format!("CFG_DCM{i}"))
            .test_bel_special_bits(specials::DCM_REG)
            .multi_global_xy("CFG_REG_*", MultiValue::Bin, 9);
        bctx.mode("DCM")
            .global_mutex("CMT", format!("CFG_DCM{i}"))
            .test_bel_special_bits(specials::DCM_BG)
            .multi_global_xy("CFG_BG_*", MultiValue::Bin, 11);

        let obel_dcm = defs::bslots::DCM[i ^ 1];
        for opin in ["CLKIN", "CLKIN_TEST"] {
            let mux = muxes[&TileWireCoord::new_idx(1, wires::IMUX_DCM_CLKIN[i])];
            let related_pll = DeltaSlot::new(0, 16, tslots::BEL);
            let bel_pll = defs::bslots::PLL;
            for &src in mux.src.keys() {
                let mut builder = bctx
                    .mode("DCM")
                    .global_mutex("CMT", "TEST")
                    .mutex("CLKIN_OUT", opin)
                    .tile_mutex("CLKIN_BEL", format!("DCM{i}"))
                    .prop(WireMutexShared::new(src.tw))
                    .prop(WireMutexExclusive::new(mux.dst))
                    .related_pip(
                        related_pll.clone(),
                        TileWireCoord::new_idx(
                            1,
                            [wires::OMUX_PLL_SKEWCLKIN2, wires::OMUX_PLL_SKEWCLKIN1][i],
                        ),
                        (bel_pll, "CLKOUTDCM0"),
                    );
                if wires::DIVCLK_CMT_W.contains(src.wire)
                    | wires::DIVCLK_CMT_E.contains(src.wire)
                    | wires::DIVCLK_CMT_V.contains(src.wire)
                {
                    builder = builder
                        .global_mutex("BUFIO2_CMT_OUT", "USE")
                        .pip((obel_dcm, opin), PipWire::AltInt(src.tw));
                }
                builder
                    .test_raw(DiffKey::Routing(tcid, mux.dst, src))
                    .pip(opin, PipWire::AltInt(src.tw))
                    .commit();
            }
        }
        for opin in ["CLKFB", "CLKFB_TEST"] {
            let mux = muxes[&TileWireCoord::new_idx(1, wires::IMUX_DCM_CLKFB[i])];
            for &src in mux.src.keys() {
                let mut builder = bctx
                    .mode("DCM")
                    .global_mutex("CMT", "TEST")
                    .mutex("CLKIN_OUT", opin)
                    .tile_mutex("CLKIN_BEL", format!("DCM{i}"))
                    .prop(WireMutexShared::new(src.tw))
                    .prop(WireMutexExclusive::new(mux.dst));
                if wires::IOFBCLK_CMT_W.contains(src.wire)
                    | wires::IOFBCLK_CMT_E.contains(src.wire)
                    | wires::IOFBCLK_CMT_V.contains(src.wire)
                {
                    builder = builder
                        .global_mutex("BUFIO2_CMT_OUT", "USE")
                        .pip((obel_dcm, opin), PipWire::AltInt(src.tw));
                }
                builder
                    .test_raw(DiffKey::Routing(tcid, mux.dst, src))
                    .pip(opin, PipWire::AltInt(src.tw))
                    .commit();
            }
        }

        for w in [wires::OMUX_DCM_SKEWCLKIN1[i], wires::OMUX_DCM_SKEWCLKIN2[i]] {
            let mux = muxes[&TileWireCoord::new_idx(1, w)];
            for &src in mux.src.keys() {
                bctx.mode("DCM")
                    .global_mutex("CMT", "TEST")
                    .pin("CLKDV")
                    .prop(WireMutexShared::new(src.tw))
                    .prop(WireMutexExclusive::new(mux.dst))
                    .test_raw(DiffKey::Routing(tcid, mux.dst, src))
                    .prop(FuzzIntPip::new(mux.dst, src.tw))
                    .commit();
            }
        }

        for pin in [
            "PSCLK", "PSEN", "PSINCDEC", "RST", "SKEWIN", "CTLGO", "CTLSEL0", "CTLSEL1", "CTLSEL2",
            "SKEWRST",
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "TEST")
                .test_inv_legacy(pin);
        }
        for pin in ["PROGCLK", "PROGEN", "PROGDATA", "RST"] {
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "TEST")
                .pin(pin)
                .test_manual_legacy(format!("{pin}INV.DCM_CLKGEN"), "0")
                .attr(format!("{pin}INV"), pin)
                .commit();
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "TEST")
                .pin(pin)
                .test_manual_legacy(format!("{pin}INV.DCM_CLKGEN"), "1")
                .attr(format!("{pin}INV"), format!("{pin}_B"))
                .commit();
        }
        for pin in [
            "FREEZEDLL",
            "FREEZEDFS",
            "CTLMODE",
            "CTLOSC1",
            "CTLOSC2",
            "STSADRS0",
            "STSADRS1",
            "STSADRS2",
            "STSADRS3",
            "STSADRS4",
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "TEST")
                .test_manual_legacy(format!("PIN.{pin}"), "1")
                .pin(pin)
                .pin_pips(pin)
                .commit();
        }

        for pin in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV", "CLKFX", "CLKFX180",
            "CONCUR",
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "PINS")
                .mutex("PIN", pin)
                .no_pin("CLKFB")
                .test_manual_legacy(pin, "1")
                .pin(pin)
                .commit();
            bctx.mode("DCM")
                .global_mutex("CMT", "PINS")
                .mutex("PIN", pin)
                .pin("CLKFB")
                .test_manual_legacy(pin, "1.CLKFB")
                .pin(pin)
                .commit();
            if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
                bctx.mode("DCM")
                    .global_mutex("CMT", "PINS")
                    .mutex("PIN", format!("{pin}.CLKFX"))
                    .pin("CLKFX")
                    .pin("CLKFB")
                    .test_manual_legacy(pin, "1.CLKFX")
                    .pin(pin)
                    .commit();
            }
        }
        bctx.mode("DCM")
            .global_mutex("CMT", "PINS")
            .test_manual_legacy("CLKFB", "1")
            .pin("CLKFB")
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GLUTMASK", "NO")
            .pin("CLKIN")
            .pin("CLKFB")
            .pin_from("CLKFB", PinFromKind::Bufg)
            .test_manual_legacy("CLKIN_IOB", "1")
            .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GLUTMASK", "NO")
            .pin("CLKIN")
            .pin("CLKFB")
            .pin_from("CLKIN", PinFromKind::Bufg)
            .test_manual_legacy("CLKFB_IOB", "1")
            .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
            .commit();

        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGEN")
            .no_pin("PROGDATA")
            .test_manual_legacy("PIN.PROGCLK", "1")
            .pin("PROGCLK")
            .commit();
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGCLK")
            .no_pin("PROGDATA")
            .test_manual_legacy("PIN.PROGEN", "1")
            .pin("PROGEN")
            .commit();
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGCLK")
            .no_pin("PROGEN")
            .test_manual_legacy("PIN.PROGDATA", "1")
            .pin("PROGDATA")
            .commit();

        bctx.mode("DCM")
            .global_mutex("CMT", "TEST")
            .test_enum_legacy(
                "DSS_MODE",
                &["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"],
            );
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("DLL_FREQUENCY_MODE", &["LOW", "HIGH"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GTS_CYCLE", "1")
            .global("DONE_CYCLE", "1")
            .global("LCK_CYCLE", "NOWAIT")
            .test_enum_legacy("STARTUP_WAIT", &["FALSE", "TRUE"]);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .global("GTS_CYCLE", "1")
            .global("DONE_CYCLE", "1")
            .global("LCK_CYCLE", "NOWAIT")
            .test_enum_suffix("STARTUP_WAIT", "CLKGEN", &["FALSE", "TRUE"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("DUTY_CYCLE_CORRECTION", &["FALSE", "TRUE"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_multi_attr_dec_legacy("DESKEW_ADJUST", 4);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("CLKIN_DIVIDE_BY_2", &["FALSE", "TRUE"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("CLK_FEEDBACK", &["NONE", "1X", "2X"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("CLKFX_MULTIPLY", "")
            .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 8);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("CLKFX_DIVIDE", "")
            .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 8);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("CLKFX_MULTIPLY.CLKGEN", "")
            .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 8);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("CLKFX_DIVIDE.CLKGEN", "")
            .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 8);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .pin("CLK0")
            .no_pin("CLKFB")
            .test_enum_legacy("VERY_HIGH_FREQUENCY", &["FALSE", "TRUE"]);

        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .pin("CLK0")
            .test_enum_legacy("CLKOUT_PHASE_SHIFT", &["NONE", "FIXED", "VARIABLE"]);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_multi_attr_dec_legacy("PHASE_SHIFT", 7);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("PHASE_SHIFT", "-1")
            .attr("PHASE_SHIFT", "-1")
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_manual_legacy("PHASE_SHIFT", "-255")
            .attr("PHASE_SHIFT", "-255")
            .commit();

        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_enum_legacy(
                "CLKDV_DIVIDE",
                &[
                    "2.0", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0", "10.0", "11.0", "12.0",
                    "13.0", "14.0", "15.0", "16.0",
                ],
            );
        for dll_mode in ["LOW", "HIGH"] {
            for val in ["1.5", "2.5", "3.5", "4.5", "5.5", "6.5", "7.5"] {
                bctx.mode("DCM")
                    .global_mutex("CMT", "USE")
                    .attr("DLL_FREQUENCY_MODE", dll_mode)
                    .attr("CLKIN_PERIOD", "")
                    .test_manual_legacy("CLKDV_DIVIDE", format!("{val}.{dll_mode}"))
                    .attr("CLKDV_DIVIDE", val)
                    .commit();
            }
        }

        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("CLKFXDV_DIVIDE", &["2", "4", "8", "16", "32"]);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("DFS_BANDWIDTH", &["LOW", "HIGH", "OPTIMIZED"]);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_enum_legacy("PROG_MD_BANDWIDTH", &["LOW", "HIGH", "OPTIMIZED"]);

        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .no_pin("PROGCLK")
            .no_pin("PROGEN")
            .no_pin("PROGDATA")
            .test_enum_legacy(
                "SPREAD_SPECTRUM",
                &[
                    "NONE",
                    "CENTER_LOW_SPREAD",
                    "CENTER_HIGH_SPREAD",
                    "VIDEO_LINK_M0",
                    "VIDEO_LINK_M1",
                    "VIDEO_LINK_M2",
                ],
            );

        // TODO: CLKIN_PERIOD?
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CMT_DCM;
    let tcls = &ctx.edev.db[tcid];
    let BelInfo::SwitchBox(ref sb) = tcls.bels[bslots::CMT_INT] else {
        unreachable!()
    };
    let mut muxes = HashMap::new();
    for item in &sb.items {
        let SwitchBoxItem::Mux(mux) = item else {
            continue;
        };
        muxes.insert(mux.dst, mux);
    }

    let tile = "CMT_DCM";
    for i in 0..2 {
        let bslot = bslots::DCM[i];
        let bel = ["DCM[0]", "DCM[1]"][i];
        let mut present_dcm = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        let mut present_dcm_clkgen = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKGEN);

        let mut cfg_interface = ctx.get_diffs_legacy(tile, bel, "INTERFACE", "");
        cfg_interface.reverse();
        let mut cfg_dll_c = ctx.get_diffs_legacy(tile, bel, "DLL_C", "");
        cfg_dll_c.reverse();
        let cfg_dll_c = xlat_bitvec_legacy(cfg_dll_c);
        for attr in ["DLL_S", "DFS_C", "DFS_S"] {
            let mut diffs = ctx.get_diffs_legacy(tile, bel, attr, "");
            diffs.reverse();
            ctx.insert_legacy(tile, bel, attr, xlat_bitvec_legacy(diffs));
        }
        for (attr, spec, width) in [
            (bcls::CMT_VREG::REG_REG, specials::DCM_REG, 9),
            (bcls::CMT_VREG::REG_BG, specials::DCM_BG, 11),
        ] {
            let mut diffs = ctx.get_diffs_bel_special_bits(tcid, bslot, spec, width);
            diffs.reverse();
            ctx.insert_bel_attr_bitvec(tcid, bslots::CMT_VREG, attr, xlat_bitvec(diffs));
        }
        let mut cfg_opt_inv = ctx.get_diffs_legacy(tile, bel, "OPT_INV", "");
        cfg_opt_inv.reverse();
        ctx.insert_legacy(
            tile,
            bel,
            "OPT_INV",
            xlat_bitvec_legacy(cfg_opt_inv[..3].to_vec()),
        );
        for pin in [
            "PSEN", "PSINCDEC", "RST", "SKEWIN", "CTLGO", "CTLSEL0", "CTLSEL1", "CTLSEL2",
            "SKEWRST",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }
        for (hwpin, pin) in [("PSEN", "PROGEN"), ("PSINCDEC", "PROGDATA"), ("RST", "RST")] {
            let item =
                ctx.extract_bit_bi_legacy(tile, bel, &format!("{pin}INV.DCM_CLKGEN"), "0", "1");
            ctx.insert_legacy(tile, bel, format!("INV.{hwpin}"), item);
        }
        for pin in [
            "FREEZEDLL",
            "FREEZEDFS",
            "CTLMODE",
            "CTLOSC1",
            "CTLOSC2",
            "STSADRS0",
            "STSADRS1",
            "STSADRS2",
            "STSADRS3",
            "STSADRS4",
        ] {
            let diff = ctx.get_diff_legacy(tile, bel, format!("PIN.{pin}"), "1");
            present_dcm = present_dcm.combine(&diff);
            present_dcm_clkgen = present_dcm.combine(&diff);
            ctx.insert_legacy(tile, bel, format!("INV.{pin}"), xlat_bit_legacy(!diff));
        }

        // hrm. concerning.
        ctx.get_diff_legacy(tile, bel, "PSCLKINV", "PSCLK")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "PSCLKINV", "PSCLK_B")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "PROGCLKINV.DCM_CLKGEN", "0")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "PROGCLKINV.DCM_CLKGEN", "1")
            .assert_empty();

        let mut diffs_clkin = vec![];
        let mux_clkin = muxes[&TileWireCoord::new_idx(1, wires::IMUX_DCM_CLKIN[i])];
        for &src in mux_clkin.src.keys() {
            diffs_clkin.push((Some(src), ctx.get_diff_routing(tcid, mux_clkin.dst, src)));
        }
        let mut diffs_clkfb = vec![];
        let mux_clkfb = muxes[&TileWireCoord::new_idx(1, wires::IMUX_DCM_CLKFB[i])];
        for &src in mux_clkfb.src.keys() {
            diffs_clkfb.push((Some(src), ctx.get_diff_routing(tcid, mux_clkfb.dst, src)));
        }

        let (_, _, clkin_clkfb_enable) =
            Diff::split(diffs_clkin[0].1.clone(), diffs_clkfb[0].1.clone());
        for (_, diff) in &mut diffs_clkin {
            *diff = diff.combine(&!&clkin_clkfb_enable);
        }
        for (_, diff) in &mut diffs_clkfb {
            *diff = diff.combine(&!&clkin_clkfb_enable);
        }
        diffs_clkin.push((None, Default::default()));
        diffs_clkfb.push((None, Default::default()));
        ctx.insert_mux(
            tcid,
            mux_clkin.dst,
            xlat_enum_raw(diffs_clkin, OcdMode::Mux),
        );
        ctx.insert_mux(
            tcid,
            mux_clkfb.dst,
            xlat_enum_raw(diffs_clkfb, OcdMode::Mux),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            bcls::DCM::CLKIN_CLKFB_ENABLE,
            xlat_bit(clkin_clkfb_enable),
        );

        for w in [wires::OMUX_DCM_SKEWCLKIN1[i], wires::OMUX_DCM_SKEWCLKIN2[i]] {
            let mux = muxes[&TileWireCoord::new_idx(1, w)];
            let mut diffs = vec![];
            for &src in mux.src.keys() {
                diffs.push((Some(src), ctx.get_diff_routing(tcid, mux.dst, src)));
            }
            ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::ValueOrder));
        }

        ctx.collect_bit_bi_legacy(tile, bel, "DUTY_CYCLE_CORRECTION", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "CLKIN_DIVIDE_BY_2", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "CLK_FEEDBACK", &["1X", "2X"]);
        ctx.collect_enum_legacy(tile, bel, "DLL_FREQUENCY_MODE", &["LOW", "HIGH"]);
        ctx.collect_enum_legacy(tile, bel, "DFS_FREQUENCY_MODE", &["LOW", "HIGH"]);
        ctx.collect_bitvec_legacy(tile, bel, "DESKEW_ADJUST", "");
        ctx.collect_bitvec_legacy(tile, bel, "CLKFX_MULTIPLY", "");
        ctx.collect_bitvec_legacy(tile, bel, "CLKFX_DIVIDE", "");
        let item = ctx.extract_bitvec_legacy(tile, bel, "CLKFX_MULTIPLY.CLKGEN", "");
        ctx.insert_legacy(tile, bel, "CLKFX_MULTIPLY", item);
        let item = ctx.extract_bitvec_legacy(tile, bel, "CLKFX_DIVIDE.CLKGEN", "");
        ctx.insert_legacy(tile, bel, "CLKFX_DIVIDE", item);
        ctx.collect_bit_legacy(tile, bel, "CLKIN_IOB", "1");
        ctx.collect_bit_legacy(tile, bel, "CLKFB_IOB", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "STARTUP_WAIT", "FALSE", "TRUE");
        let item = ctx.extract_bit_bi_legacy(tile, bel, "STARTUP_WAIT.CLKGEN", "FALSE", "TRUE");
        ctx.insert_legacy(tile, bel, "STARTUP_WAIT", item);
        let item = ctx.extract_bit_legacy(tile, bel, "CLK_FEEDBACK", "NONE");
        ctx.insert_legacy(tile, bel, "NO_FEEDBACK", item);

        ctx.get_diff_legacy(tile, bel, "CLKFB", "1").assert_empty();

        let (_, _, dll_en) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "CLK0", "1").clone(),
            ctx.peek_diff_legacy(tile, bel, "CLK180", "1").clone(),
        );

        for pin in [
            "CLK0", "CLK90", "CLK180", "CLK270", "CLK2X", "CLK2X180", "CLKDV",
        ] {
            let diff = ctx.get_diff_legacy(tile, bel, pin, "1");
            let diff_fb = ctx.get_diff_legacy(tile, bel, pin, "1.CLKFB");
            assert_eq!(diff, diff_fb);
            let diff_fx = ctx.get_diff_legacy(tile, bel, pin, "1.CLKFX");
            let diff_fx = diff_fx.combine(&!&diff);
            let mut diff = diff.combine(&!&dll_en);
            // hrm.
            if ctx.device.name.ends_with('l') && pin == "CLKDV" {
                diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "DLL_S"), 0, 0x40);
            }
            ctx.insert_legacy(tile, bel, format!("ENABLE.{pin}"), xlat_bit_legacy(diff));
            ctx.insert_legacy(tile, bel, "DFS_FEEDBACK", xlat_bit_legacy(diff_fx));
        }
        ctx.insert_legacy(tile, bel, "DLL_ENABLE", xlat_bit_legacy(dll_en));

        ctx.get_diff_legacy(tile, bel, "VERY_HIGH_FREQUENCY", "FALSE")
            .assert_empty();
        let diff = ctx.get_diff_legacy(tile, bel, "VERY_HIGH_FREQUENCY", "TRUE");
        ctx.insert_legacy(tile, bel, "DLL_ENABLE", xlat_bit_legacy(!diff));

        for attr in ["PIN.PROGCLK", "PIN.PROGEN", "PIN.PROGDATA"] {
            let item = ctx.extract_bit_legacy(tile, bel, attr, "1");
            ctx.insert_legacy(tile, bel, "PROG_ENABLE", item);
        }

        let (_, _, dfs_en) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "CLKFX", "1").clone(),
            ctx.peek_diff_legacy(tile, bel, "CONCUR", "1").clone(),
        );
        for pin in ["CLKFX", "CLKFX180", "CONCUR"] {
            let diff = ctx.get_diff_legacy(tile, bel, pin, "1");
            let diff_fb = ctx.get_diff_legacy(tile, bel, pin, "1.CLKFB");
            assert_eq!(diff, diff_fb);
            let diff = diff.combine(&!&dfs_en);
            let pin = if pin == "CONCUR" { pin } else { "CLKFX" };
            ctx.insert_legacy(tile, bel, format!("ENABLE.{pin}"), xlat_bit_legacy(diff));
        }
        ctx.insert_legacy(tile, bel, "DFS_ENABLE", xlat_bit_legacy(dfs_en));

        let mut diffs = vec![ctx.get_diff_legacy(tile, bel, "PHASE_SHIFT", "-255")];
        diffs.extend(ctx.get_diffs_legacy(tile, bel, "PHASE_SHIFT", ""));
        let item = xlat_bitvec_legacy(diffs);
        let mut diff = ctx.get_diff_legacy(tile, bel, "PHASE_SHIFT", "-1");
        diff.apply_bitvec_diff_int_legacy(&item, 2, 0);
        ctx.insert_legacy(tile, bel, "PHASE_SHIFT", item);
        ctx.insert_legacy(tile, bel, "PHASE_SHIFT_NEGATIVE", xlat_bit_legacy(diff));

        ctx.collect_enum_legacy(
            tile,
            bel,
            "CLKOUT_PHASE_SHIFT",
            &["NONE", "FIXED", "VARIABLE"],
        );

        let mut diffs = vec![];
        for val in [
            "NONE",
            "CENTER_LOW_SPREAD",
            "CENTER_HIGH_SPREAD",
            "VIDEO_LINK_M0",
            "VIDEO_LINK_M1",
            "VIDEO_LINK_M2",
        ] {
            let mut diff = ctx.get_diff_legacy(tile, bel, "SPREAD_SPECTRUM", val);
            if val.starts_with("VIDEO_LINK") {
                diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "PROG_ENABLE"), true, false);
            }
            diffs.push((val.to_string(), diff));
        }
        ctx.insert_legacy(
            tile,
            bel,
            "SPREAD_SPECTRUM",
            xlat_enum_default_legacy(diffs, "DCM"),
        );

        for (attr, bits) in [
            ("CLKDV_COUNT_MAX", &cfg_dll_c.bits[1..5]),
            ("CLKDV_COUNT_FALL", &cfg_dll_c.bits[5..9]),
            ("CLKDV_COUNT_FALL_2", &cfg_dll_c.bits[9..13]),
            ("CLKDV_PHASE_RISE", &cfg_dll_c.bits[13..15]),
            ("CLKDV_PHASE_FALL", &cfg_dll_c.bits[15..17]),
        ] {
            ctx.insert_legacy(
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
        ctx.insert_legacy(
            tile,
            bel,
            "CLKDV_MODE",
            TileItem {
                bits: cfg_dll_c.bits[17..18].to_vec(),
                kind: TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("HALF".to_string(), bits![0]),
                        ("INT".to_string(), bits![1]),
                    ]),
                },
            },
        );

        let clkdv_count_max = ctx.item_legacy(tile, bel, "CLKDV_COUNT_MAX").clone();
        let clkdv_count_fall = ctx.item_legacy(tile, bel, "CLKDV_COUNT_FALL").clone();
        let clkdv_count_fall_2 = ctx.item_legacy(tile, bel, "CLKDV_COUNT_FALL_2").clone();
        let clkdv_phase_fall = ctx.item_legacy(tile, bel, "CLKDV_PHASE_FALL").clone();
        let clkdv_mode = ctx.item_legacy(tile, bel, "CLKDV_MODE").clone();
        for i in 2..=16 {
            let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.0"));
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, i - 1, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        for i in 1..=7 {
            let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.LOW"));
            diff.apply_enum_diff_legacy(&clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, i / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
            diff.assert_empty();
            let mut diff = ctx.get_diff_legacy(tile, bel, "CLKDV_DIVIDE", format!("{i}.5.HIGH"));
            diff.apply_enum_diff_legacy(&clkdv_mode, "HALF", "INT");
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
            diff.apply_bitvec_diff_int_legacy(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }

        for val in ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"] {
            ctx.get_diff_legacy(tile, bel, "DSS_MODE", val)
                .assert_empty();
        }
        for val in ["LOW", "HIGH", "OPTIMIZED"] {
            ctx.get_diff_legacy(tile, bel, "DFS_BANDWIDTH", val)
                .assert_empty();
            ctx.get_diff_legacy(tile, bel, "PROG_MD_BANDWIDTH", val)
                .assert_empty();
        }
        let mut item =
            ctx.extract_enum_legacy(tile, bel, "CLKFXDV_DIVIDE", &["32", "16", "8", "4", "2"]);
        assert_eq!(item.bits.len(), 3);
        let TileItemKind::Enum { ref mut values } = item.kind else {
            unreachable!()
        };
        values.insert("NONE".into(), BitVec::repeat(false, 3));
        ctx.insert_legacy(tile, bel, "CLKFXDV_DIVIDE", item);

        ctx.insert_legacy(tile, bel, "DLL_C", cfg_dll_c);

        present_dcm.apply_bitvec_diff_int_legacy(
            ctx.item_legacy(tile, bel, "CLKDV_COUNT_MAX"),
            1,
            0,
        );
        present_dcm.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "CLKDV_MODE"), "INT", "HALF");
        present_dcm_clkgen.apply_bitvec_diff_int_legacy(
            ctx.item_legacy(tile, bel, "CLKDV_COUNT_MAX"),
            1,
            0,
        );
        present_dcm_clkgen.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "CLKDV_MODE"),
            "INT",
            "HALF",
        );
        present_dcm.apply_bitvec_diff_int_legacy(
            ctx.item_legacy(tile, bel, "DESKEW_ADJUST"),
            11,
            0,
        );
        present_dcm_clkgen.apply_bitvec_diff_int_legacy(
            ctx.item_legacy(tile, bel, "DESKEW_ADJUST"),
            11,
            0,
        );
        present_dcm.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "CLKFXDV_DIVIDE"),
            "2",
            "NONE",
        );
        present_dcm_clkgen.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "CLKFXDV_DIVIDE"),
            "2",
            "NONE",
        );
        present_dcm.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "DUTY_CYCLE_CORRECTION"),
            true,
            false,
        );
        present_dcm_clkgen.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "DUTY_CYCLE_CORRECTION"),
            true,
            false,
        );
        present_dcm.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslots::CMT_VREG, bcls::CMT_VREG::REG_REG),
            &bits![1, 1, 0, 0, 0, 0, 1, 0, 1],
            &bits![0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm_clkgen.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslots::CMT_VREG, bcls::CMT_VREG::REG_REG),
            &bits![1, 1, 0, 0, 0, 0, 1, 0, 1],
            &bits![0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslots::CMT_VREG, bcls::CMT_VREG::REG_BG),
            &bits![0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1],
            &bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        present_dcm_clkgen.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslots::CMT_VREG, bcls::CMT_VREG::REG_BG),
            &bits![0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1],
            &bits![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );

        // ???
        present_dcm_clkgen.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "INV.STSADRS4"),
            false,
            true,
        );

        let mut base_dfs_s = BitVec::repeat(false, 87);
        base_dfs_s.set(17, true);
        base_dfs_s.set(21, true);
        base_dfs_s.set(32, true);
        base_dfs_s.set(33, true);
        base_dfs_s.set(37, true);
        base_dfs_s.set(41, true);
        base_dfs_s.set(43, true);
        base_dfs_s.set(45, true);
        base_dfs_s.set(52, true);
        base_dfs_s.set(64, true);
        base_dfs_s.set(68, true);
        base_dfs_s.set(76, true);
        base_dfs_s.set(77, true);
        present_dcm.apply_bitvec_diff_legacy(
            ctx.item_legacy(tile, bel, "DFS_S"),
            &base_dfs_s,
            &BitVec::repeat(false, 87),
        );
        present_dcm_clkgen.apply_bitvec_diff_legacy(
            ctx.item_legacy(tile, bel, "DFS_S"),
            &base_dfs_s,
            &BitVec::repeat(false, 87),
        );

        let mut base_dll_s = BitVec::repeat(false, 32);
        base_dll_s.set(0, true);
        base_dll_s.set(6, true);
        base_dll_s.set(13, true); // period not hf
        present_dcm.apply_bitvec_diff_legacy(
            ctx.item_legacy(tile, bel, "DLL_S"),
            &base_dll_s,
            &BitVec::repeat(false, 32),
        );
        present_dcm_clkgen.apply_bitvec_diff_legacy(
            ctx.item_legacy(tile, bel, "DLL_S"),
            &base_dll_s,
            &BitVec::repeat(false, 32),
        );

        present_dcm = present_dcm.combine(&!&cfg_interface[9]);
        present_dcm = present_dcm.combine(&!&cfg_interface[10]);
        present_dcm = present_dcm.combine(&!&cfg_interface[12]);
        present_dcm = present_dcm.combine(&!&cfg_interface[13]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[9]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[10]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[12]);
        present_dcm_clkgen = present_dcm_clkgen.combine(&!&cfg_interface[13]);

        assert_eq!(present_dcm.bits.len(), 1);
        assert_eq!(present_dcm, present_dcm_clkgen);
        cfg_interface[18].assert_empty();
        cfg_interface[18] = present_dcm;
        ctx.insert_legacy(tile, bel, "INTERFACE", xlat_bitvec_legacy(cfg_interface));
    }

    let bslot = bslots::CMT_VREG;
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::CMT_PRESENT_ANY_DCM);
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::CMT_VREG::REG_BG),
        0,
        1,
    );
    diff.assert_empty();
}
