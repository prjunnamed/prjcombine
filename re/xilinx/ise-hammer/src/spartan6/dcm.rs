use std::collections::HashMap;

use prjcombine_entity::EntityPartVec;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, BelAttributeType, BelInfo, SwitchBoxItem, TileWireCoord},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, SpecialId, xlat_bit, xlat_bitvec, xlat_enum_attr, xlat_enum_raw,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_spartan6::defs::{self, bcls, bslots, enums, tcls, tslots, wires};
use prjcombine_types::{bits, bitvec::BitVec};

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
            .test_bel_attr_bits(bcls::DCM::REG_DLL_C)
            .multi_global_xy("CFG_DLL_C_*", MultiValue::Bin, 32);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_bel_attr_bits(bcls::DCM::REG_DLL_S)
            .multi_global_xy("CFG_DLL_S_*", MultiValue::Bin, 32);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_bel_attr_bits(bcls::DCM::REG_DFS_C)
            .multi_global_xy("CFG_DFS_C_*", MultiValue::Bin, 3);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_bel_attr_bits(bcls::DCM::REG_DFS_S)
            .multi_global_xy("CFG_DFS_S_*", MultiValue::Bin, 87);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_bel_attr_bits(bcls::DCM::REG_INTERFACE)
            .multi_global_xy("CFG_INTERFACE_*", MultiValue::Bin, 40);
        bctx.mode("DCM")
            .global_mutex("CMT", "CFG")
            .test_bel_attr_bits(bcls::DCM::REG_OPT_INV)
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
                    .test_routing(mux.dst, src)
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
                    .test_routing(mux.dst, src)
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
                    .test_routing(mux.dst, src)
                    .prop(FuzzIntPip::new(mux.dst, src.tw))
                    .commit();
            }
        }

        for pin in [
            bcls::DCM::PSCLK,
            bcls::DCM::PSEN,
            bcls::DCM::PSINCDEC,
            bcls::DCM::RST,
            bcls::DCM::SKEWIN,
            bcls::DCM::CTLGO,
            bcls::DCM::CTLSEL[0],
            bcls::DCM::CTLSEL[1],
            bcls::DCM::CTLSEL[2],
            bcls::DCM::SKEWRST,
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "TEST")
                .test_bel_input_inv_auto(pin);
        }

        for (pid, pin) in [
            (bcls::DCM::PSCLK, "PROGCLK"),
            (bcls::DCM::PSEN, "PROGEN"),
            (bcls::DCM::PSINCDEC, "PROGDATA"),
            (bcls::DCM::RST, "RST"),
        ] {
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "TEST")
                .pin(pin)
                .test_bel_input_inv(pid, false)
                .attr(format!("{pin}INV"), pin)
                .commit();
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "TEST")
                .pin(pin)
                .test_bel_input_inv(pid, true)
                .attr(format!("{pin}INV"), format!("{pin}_B"))
                .commit();
        }

        for (pid, pin) in [
            (bcls::DCM::FREEZEDLL, "FREEZEDLL"),
            (bcls::DCM::FREEZEDFS, "FREEZEDFS"),
            (bcls::DCM::CTLMODE, "CTLMODE"),
            (bcls::DCM::CTLOSC1, "CTLOSC1"),
            (bcls::DCM::CTLOSC2, "CTLOSC2"),
            (bcls::DCM::STSADRS[0], "STSADRS0"),
            (bcls::DCM::STSADRS[1], "STSADRS1"),
            (bcls::DCM::STSADRS[2], "STSADRS2"),
            (bcls::DCM::STSADRS[3], "STSADRS3"),
            (bcls::DCM::STSADRS[4], "STSADRS4"),
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "TEST")
                .test_bel_input_inv(pid, false)
                .pin(pin)
                .pin_pips(pin)
                .commit();
        }

        for (attr, pin) in [
            (bcls::DCM::OUT_CLK0_ENABLE, "CLK0"),
            (bcls::DCM::OUT_CLK90_ENABLE, "CLK90"),
            (bcls::DCM::OUT_CLK180_ENABLE, "CLK180"),
            (bcls::DCM::OUT_CLK270_ENABLE, "CLK270"),
            (bcls::DCM::OUT_CLK2X_ENABLE, "CLK2X"),
            (bcls::DCM::OUT_CLK2X180_ENABLE, "CLK2X180"),
            (bcls::DCM::OUT_CLKDV_ENABLE, "CLKDV"),
            (bcls::DCM::OUT_CLKFX_ENABLE, "CLKFX"),
            (bcls::DCM::OUT_CLKFX_ENABLE, "CLKFX180"),
            (bcls::DCM::OUT_CONCUR_ENABLE, "CONCUR"),
        ] {
            bctx.mode("DCM")
                .global_mutex("CMT", "PINS")
                .mutex("PIN", pin)
                .no_pin("CLKFB")
                .test_bel_attr_bits(attr)
                .pin(pin)
                .commit();
            bctx.mode("DCM")
                .global_mutex("CMT", "PINS")
                .mutex("PIN", pin)
                .pin("CLKFB")
                .test_bel_attr_special(attr, specials::DCM_PIN_CLKFB)
                .pin(pin)
                .commit();
            if pin != "CLKFX" && pin != "CLKFX180" && pin != "CONCUR" {
                bctx.mode("DCM")
                    .global_mutex("CMT", "PINS")
                    .mutex("PIN", format!("{pin}.CLKFX"))
                    .pin("CLKFX")
                    .pin("CLKFB")
                    .test_bel_attr_special(attr, specials::DCM_PIN_CLKFX)
                    .pin(pin)
                    .commit();
            }
        }
        bctx.mode("DCM")
            .null_bits()
            .global_mutex("CMT", "PINS")
            .test_bel_special(specials::DCM_PIN_CLKFB)
            .pin("CLKFB")
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GLUTMASK", "NO")
            .pin("CLKIN")
            .pin("CLKFB")
            .pin_from("CLKFB", PinFromKind::Bufg)
            .test_bel_attr_bits(bcls::DCM::CLKIN_IOB)
            .pin_from("CLKIN", PinFromKind::Bufg, PinFromKind::Iob)
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GLUTMASK", "NO")
            .pin("CLKIN")
            .pin("CLKFB")
            .pin_from("CLKIN", PinFromKind::Bufg)
            .test_bel_attr_bits(bcls::DCM::CLKFB_IOB)
            .pin_from("CLKFB", PinFromKind::Bufg, PinFromKind::Iob)
            .commit();

        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGEN")
            .no_pin("PROGDATA")
            .test_bel_attr_bits(bcls::DCM::PROG_ENABLE)
            .pin("PROGCLK")
            .commit();
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGCLK")
            .no_pin("PROGDATA")
            .test_bel_attr_bits(bcls::DCM::PROG_ENABLE)
            .pin("PROGEN")
            .commit();
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "PINS")
            .no_pin("PROGCLK")
            .no_pin("PROGEN")
            .test_bel_attr_bits(bcls::DCM::PROG_ENABLE)
            .pin("PROGDATA")
            .commit();

        for val in ["NONE", "SPREAD_2", "SPREAD_4", "SPREAD_6", "SPREAD_8"] {
            bctx.mode("DCM")
                .null_bits()
                .global_mutex("CMT", "TEST")
                .test_bel_special(specials::DCM_DSS_MODE)
                .attr("DSS_MODE", val)
                .commit();
        }
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_auto(bcls::DCM::DLL_FREQUENCY_MODE);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_auto(bcls::DCM::DFS_FREQUENCY_MODE);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .global("GTS_CYCLE", "1")
            .global("DONE_CYCLE", "1")
            .global("LCK_CYCLE", "NOWAIT")
            .test_bel_attr_bool_auto(bcls::DCM::STARTUP_WAIT, "FALSE", "TRUE");
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .global("GTS_CYCLE", "1")
            .global("DONE_CYCLE", "1")
            .global("LCK_CYCLE", "NOWAIT")
            .test_bel_attr_bool_auto(bcls::DCM::STARTUP_WAIT, "FALSE", "TRUE");
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bool_auto(bcls::DCM::DUTY_CYCLE_CORRECTION, "FALSE", "TRUE");
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_multi(bcls::DCM::DESKEW_ADJUST, MultiValue::Dec(0));
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bool_auto(bcls::DCM::CLKIN_DIVIDE_BY_2, "FALSE", "TRUE");
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bool_rename("CLK_FEEDBACK", bcls::DCM::CLK_FEEDBACK_2X, "1X", "2X");
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bits(bcls::DCM::CLK_FEEDBACK_DISABLE)
            .attr("CLK_FEEDBACK", "NONE")
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bits(bcls::DCM::CLKFX_MULTIPLY)
            .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 8);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bits(bcls::DCM::CLKFX_DIVIDE)
            .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 8);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bits(bcls::DCM::CLKFX_MULTIPLY)
            .multi_attr("CLKFX_MULTIPLY", MultiValue::Dec(1), 8);
        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bits(bcls::DCM::CLKFX_DIVIDE)
            .multi_attr("CLKFX_DIVIDE", MultiValue::Dec(1), 8);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .pin("CLK0")
            .no_pin("CLKFB")
            .test_bel_special(specials::DCM_VERY_HIGH_FREQUENCY)
            .attr("VERY_HIGH_FREQUENCY", "TRUE")
            .commit();

        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .pin("CLK0")
            .test_bel_attr_auto_default(
                bcls::DCM::CLKOUT_PHASE_SHIFT,
                enums::DCM_CLKOUT_PHASE_SHIFT::MISSING,
            );
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_attr_bits(bcls::DCM::PHASE_SHIFT)
            .multi_attr("PHASE_SHIFT", MultiValue::Dec(0), 7);
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_special(specials::DCM_PHASE_SHIFT_N1)
            .attr("PHASE_SHIFT", "-1")
            .commit();
        bctx.mode("DCM")
            .global_mutex("CMT", "USE")
            .test_bel_special(specials::DCM_PHASE_SHIFT_N255)
            .attr("PHASE_SHIFT", "-255")
            .commit();

        for val in 2..=16 {
            bctx.mode("DCM")
                .global_mutex("CMT", "USE")
                .test_bel_special_u32(specials::DCM_CLKDV_DIVIDE_INT, val)
                .attr("CLKDV_DIVIDE", format!("{val}.0"))
                .commit();
        }
        for (spec, dll_mode) in [
            (specials::DCM_CLKDV_DIVIDE_HALF_LOW, "LOW"),
            (specials::DCM_CLKDV_DIVIDE_HALF_HIGH, "HIGH"),
        ] {
            for val in 1..=7 {
                bctx.mode("DCM")
                    .global_mutex("CMT", "USE")
                    .attr("DLL_FREQUENCY_MODE", dll_mode)
                    .attr("CLKIN_PERIOD", "")
                    .test_bel_special_u32(spec, val)
                    .attr("CLKDV_DIVIDE", format!("{val}.5"))
                    .commit();
            }
        }

        bctx.mode("DCM_CLKGEN")
            .global_mutex("CMT", "USE")
            .test_bel_attr_auto_default(bcls::DCM::CLKFXDV_DIVIDE, enums::DCM_CLKFXDV_DIVIDE::NONE);
        for val in ["LOW", "HIGH", "OPTIMIZED"] {
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "USE")
                .null_bits()
                .test_bel_special(specials::DCM_DFS_BANDWIDTH)
                .attr("DFS_BANDWIDTH", val)
                .commit();
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "USE")
                .null_bits()
                .test_bel_special(specials::DCM_PROG_MD_BANDWIDTH)
                .attr("PROG_MD_BANDWIDTH", val)
                .commit();
        }

        for (val, vname) in &backend.edev.db[enums::DCM_SPREAD_SPECTRUM].values {
            if matches!(
                val,
                enums::DCM_SPREAD_SPECTRUM::MISSING | enums::DCM_SPREAD_SPECTRUM::DCM
            ) {
                continue;
            }
            bctx.mode("DCM_CLKGEN")
                .global_mutex("CMT", "USE")
                .no_pin("PROGCLK")
                .no_pin("PROGEN")
                .no_pin("PROGDATA")
                .test_bel_attr_val(bcls::DCM::SPREAD_SPECTRUM, val)
                .attr("SPREAD_SPECTRUM", vname)
                .commit();
        }

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

    for i in 0..2 {
        let bslot = bslots::DCM[i];
        let mut present_dcm = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        let mut present_dcm_clkgen = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_CLKGEN);

        let mut cfg_interface = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::REG_INTERFACE, 40);
        cfg_interface.reverse();
        for attr in [
            bcls::DCM::REG_DLL_C,
            bcls::DCM::REG_DLL_S,
            bcls::DCM::REG_DFS_C,
            bcls::DCM::REG_DFS_S,
        ] {
            let BelAttributeType::BitVec(width) = ctx.edev.db[bcls::DCM].attributes[attr].typ
            else {
                unreachable!()
            };
            let mut diffs = ctx.get_diffs_attr_bits(tcid, bslot, attr, width);
            diffs.reverse();
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr, xlat_bitvec(diffs));
        }
        let cfg_dll_c = ctx
            .bel_attr_bitvec(tcid, bslot, bcls::DCM::REG_DLL_C)
            .to_vec();
        for (attr, spec, width) in [
            (bcls::CMT_VREG::REG_REG, specials::DCM_REG, 9),
            (bcls::CMT_VREG::REG_BG, specials::DCM_BG, 11),
        ] {
            let mut diffs = ctx.get_diffs_bel_special_bits(tcid, bslot, spec, width);
            diffs.reverse();
            ctx.insert_bel_attr_bitvec(tcid, bslots::CMT_VREG, attr, xlat_bitvec(diffs));
        }
        let mut cfg_opt_inv = ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::REG_OPT_INV, 20);
        cfg_opt_inv.reverse();
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCM::REG_OPT_INV,
            xlat_bitvec(cfg_opt_inv[..3].to_vec()),
        );
        for pin in [
            bcls::DCM::PSEN,
            bcls::DCM::PSINCDEC,
            bcls::DCM::RST,
            bcls::DCM::SKEWIN,
            bcls::DCM::CTLGO,
            bcls::DCM::CTLSEL[0],
            bcls::DCM::CTLSEL[1],
            bcls::DCM::CTLSEL[2],
            bcls::DCM::SKEWRST,
        ] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        for pin in [
            bcls::DCM::FREEZEDLL,
            bcls::DCM::FREEZEDFS,
            bcls::DCM::CTLMODE,
            bcls::DCM::CTLOSC1,
            bcls::DCM::CTLOSC2,
            bcls::DCM::STSADRS[0],
            bcls::DCM::STSADRS[1],
            bcls::DCM::STSADRS[2],
            bcls::DCM::STSADRS[3],
            bcls::DCM::STSADRS[4],
        ] {
            let diff = ctx.get_diff_bel_input_inv(tcid, bslot, pin, false);
            present_dcm = present_dcm.combine(&diff);
            present_dcm_clkgen = present_dcm_clkgen.combine(&diff);
            ctx.insert_bel_input_inv(tcid, bslot, pin, xlat_bit(!diff));
        }

        // hrm. concerning.
        ctx.get_diff_bel_input_inv(tcid, bslot, bcls::DCM::PSCLK, false)
            .assert_empty();
        ctx.get_diff_bel_input_inv(tcid, bslot, bcls::DCM::PSCLK, true)
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

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::DUTY_CYCLE_CORRECTION);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::CLKIN_DIVIDE_BY_2);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::CLK_FEEDBACK_2X);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::DLL_FREQUENCY_MODE);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::DFS_FREQUENCY_MODE);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::DESKEW_ADJUST);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKFX_MULTIPLY);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKFX_DIVIDE);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKIN_IOB);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLKFB_IOB);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::DCM::STARTUP_WAIT);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::CLK_FEEDBACK_DISABLE);

        let (_, _, dll_en) = Diff::split(
            ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CLK0_ENABLE, 0)
                .clone(),
            ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CLK180_ENABLE, 0)
                .clone(),
        );

        for attr in [
            bcls::DCM::OUT_CLK0_ENABLE,
            bcls::DCM::OUT_CLK90_ENABLE,
            bcls::DCM::OUT_CLK180_ENABLE,
            bcls::DCM::OUT_CLK270_ENABLE,
            bcls::DCM::OUT_CLK2X_ENABLE,
            bcls::DCM::OUT_CLK2X180_ENABLE,
            bcls::DCM::OUT_CLKDV_ENABLE,
        ] {
            let diff = ctx.get_diff_attr_bit(tcid, bslot, attr, 0);
            let diff_fb = ctx.get_diff_attr_special(tcid, bslot, attr, specials::DCM_PIN_CLKFB);
            let diff_fx = ctx.get_diff_attr_special(tcid, bslot, attr, specials::DCM_PIN_CLKFX);
            assert_eq!(diff, diff_fb);
            let diff_fx = diff_fx.combine(&!&diff);
            let mut diff = diff.combine(&!&dll_en);
            // hrm.
            if ctx.device.name.ends_with('l') && attr == bcls::DCM::OUT_CLKDV_ENABLE {
                diff.apply_bitvec_diff_int(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::REG_DLL_S),
                    0,
                    0x40,
                );
            }
            ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DFS_FEEDBACK, xlat_bit(diff_fx));
        }
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DLL_ENABLE, xlat_bit(dll_en));

        let diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_VERY_HIGH_FREQUENCY);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DLL_ENABLE, xlat_bit(!diff));

        ctx.collect_bel_attr(tcid, bslot, bcls::DCM::PROG_ENABLE);

        let (_, _, dfs_en) = Diff::split(
            ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CLKFX_ENABLE, 0)
                .clone(),
            ctx.peek_diff_attr_bit(tcid, bslot, bcls::DCM::OUT_CONCUR_ENABLE, 0)
                .clone(),
        );
        for attr in [bcls::DCM::OUT_CLKFX_ENABLE, bcls::DCM::OUT_CONCUR_ENABLE] {
            let diff = ctx.get_diff_attr_bit(tcid, bslot, attr, 0);
            let diff_fb = ctx.get_diff_attr_special(tcid, bslot, attr, specials::DCM_PIN_CLKFB);
            assert_eq!(diff, diff_fb);
            let diff = diff.combine(&!&dfs_en);
            ctx.insert_bel_attr_bool(tcid, bslot, attr, xlat_bit(diff));
        }
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::DFS_ENABLE, xlat_bit(dfs_en));

        let mut diffs = vec![ctx.get_diff_bel_special(tcid, bslot, specials::DCM_PHASE_SHIFT_N255)];
        diffs.extend(ctx.get_diffs_attr_bits(tcid, bslot, bcls::DCM::PHASE_SHIFT, 7));
        let item = xlat_bitvec(diffs);
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::DCM_PHASE_SHIFT_N1);
        diff.apply_bitvec_diff_int(&item, 2, 0);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCM::PHASE_SHIFT, item);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCM::PHASE_SHIFT_NEGATIVE, xlat_bit(diff));

        let mut diffs = vec![];
        for val in ctx.edev.db[enums::DCM_CLKOUT_PHASE_SHIFT].values.ids() {
            if val == enums::DCM_CLKOUT_PHASE_SHIFT::MISSING {
                continue;
            }
            diffs.push((
                val,
                ctx.get_diff_attr_val(tcid, bslot, bcls::DCM::CLKOUT_PHASE_SHIFT, val),
            ));
        }
        let mut item = xlat_enum_attr(diffs);
        item.values.insert(
            enums::DCM_CLKOUT_PHASE_SHIFT::MISSING,
            bits![0; item.bits.len()],
        );
        present_dcm_clkgen.apply_enum_diff(
            &item,
            enums::DCM_CLKOUT_PHASE_SHIFT::VARIABLE,
            enums::DCM_CLKOUT_PHASE_SHIFT::MISSING,
        );
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::DCM::CLKOUT_PHASE_SHIFT, item);

        let mut diffs = vec![];
        for val in ctx.edev.db[enums::DCM_SPREAD_SPECTRUM].values.ids() {
            if matches!(
                val,
                enums::DCM_SPREAD_SPECTRUM::MISSING | enums::DCM_SPREAD_SPECTRUM::DCM
            ) {
                continue;
            }
            let mut diff = ctx.get_diff_attr_val(tcid, bslot, bcls::DCM::SPREAD_SPECTRUM, val);
            if matches!(
                val,
                enums::DCM_SPREAD_SPECTRUM::VIDEO_LINK_M0
                    | enums::DCM_SPREAD_SPECTRUM::VIDEO_LINK_M1
                    | enums::DCM_SPREAD_SPECTRUM::VIDEO_LINK_M2
            ) {
                diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslot, bcls::DCM::PROG_ENABLE),
                    true,
                    false,
                );
            }
            if val == enums::DCM_SPREAD_SPECTRUM::NONE {
                let dfs_s = ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::REG_DFS_S);
                let mut diff_dcm = diff.clone();
                diff_dcm.apply_bit_diff(dfs_s[76], false, true);
                diff_dcm.apply_bit_diff(dfs_s[77], false, true);
                diffs.push((enums::DCM_SPREAD_SPECTRUM::DCM, diff_dcm));
            }

            diffs.push((val, diff));
        }
        let mut item = xlat_enum_attr(diffs);
        item.values.insert(
            enums::DCM_SPREAD_SPECTRUM::MISSING,
            bits![0; item.bits.len()],
        );
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::DCM::SPREAD_SPECTRUM, item);

        for (attr, bits) in [
            (bcls::DCM::CLKDV_COUNT_MAX, &cfg_dll_c[1..5]),
            (bcls::DCM::CLKDV_COUNT_FALL, &cfg_dll_c[5..9]),
            (bcls::DCM::CLKDV_COUNT_FALL_2, &cfg_dll_c[9..13]),
            (bcls::DCM::CLKDV_PHASE_RISE, &cfg_dll_c[13..15]),
            (bcls::DCM::CLKDV_PHASE_FALL, &cfg_dll_c[15..17]),
        ] {
            ctx.insert_bel_attr_bitvec(tcid, bslot, attr, bits.to_vec());
        }
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::DCM::CLKDV_MODE,
            BelAttributeEnum {
                bits: vec![cfg_dll_c[17].bit],
                values: EntityPartVec::from_iter([
                    (enums::DCM_CLKDV_MODE::HALF, bits![0]),
                    (enums::DCM_CLKDV_MODE::INT, bits![1]),
                ]),
            },
        );

        let clkdv_count_max = ctx
            .bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_MAX)
            .to_vec();
        let clkdv_count_fall = ctx
            .bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_FALL)
            .to_vec();
        let clkdv_count_fall_2 = ctx
            .bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_FALL_2)
            .to_vec();
        let clkdv_phase_fall = ctx
            .bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_PHASE_FALL)
            .to_vec();
        let clkdv_mode = ctx
            .bel_attr_enum(tcid, bslot, bcls::DCM::CLKDV_MODE)
            .clone();
        for i in 2..=16 {
            let mut diff =
                ctx.get_diff_bel_special_u32(tcid, bslot, specials::DCM_CLKDV_DIVIDE_INT, i as u32);
            diff.apply_bitvec_diff_int(&clkdv_count_max, i - 1, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        for i in 1..=7 {
            let mut diff = ctx.get_diff_bel_special_u32(
                tcid,
                bslot,
                specials::DCM_CLKDV_DIVIDE_HALF_LOW,
                i as u32,
            );
            diff.apply_enum_diff(
                &clkdv_mode,
                enums::DCM_CLKDV_MODE::HALF,
                enums::DCM_CLKDV_MODE::INT,
            );
            diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, i / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
            diff.assert_empty();
            let mut diff = ctx.get_diff_bel_special_u32(
                tcid,
                bslot,
                specials::DCM_CLKDV_DIVIDE_HALF_HIGH,
                i as u32,
            );
            diff.apply_enum_diff(
                &clkdv_mode,
                enums::DCM_CLKDV_MODE::HALF,
                enums::DCM_CLKDV_MODE::INT,
            );
            diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }

        let mut diffs = vec![];
        for val in [
            enums::DCM_CLKFXDV_DIVIDE::_2,
            enums::DCM_CLKFXDV_DIVIDE::_4,
            enums::DCM_CLKFXDV_DIVIDE::_8,
            enums::DCM_CLKFXDV_DIVIDE::_16,
            enums::DCM_CLKFXDV_DIVIDE::_32,
        ] {
            diffs.push((
                val,
                ctx.get_diff_attr_val(tcid, bslot, bcls::DCM::CLKFXDV_DIVIDE, val),
            ));
        }
        let mut item = xlat_enum_attr(diffs);
        assert_eq!(item.bits.len(), 3);
        item.values
            .insert(enums::DCM_CLKFXDV_DIVIDE::NONE, BitVec::repeat(false, 3));
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::DCM::CLKFXDV_DIVIDE, item);

        present_dcm.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_MAX),
            1,
            0,
        );
        present_dcm.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::DCM::CLKDV_MODE),
            enums::DCM_CLKDV_MODE::INT,
            enums::DCM_CLKDV_MODE::HALF,
        );
        present_dcm_clkgen.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::CLKDV_COUNT_MAX),
            1,
            0,
        );
        present_dcm_clkgen.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::DCM::CLKDV_MODE),
            enums::DCM_CLKDV_MODE::INT,
            enums::DCM_CLKDV_MODE::HALF,
        );
        present_dcm.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::DESKEW_ADJUST),
            11,
            0,
        );
        present_dcm.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::DCM::CLKFXDV_DIVIDE),
            enums::DCM_CLKFXDV_DIVIDE::_2,
            enums::DCM_CLKFXDV_DIVIDE::NONE,
        );
        present_dcm_clkgen.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::DCM::CLKFXDV_DIVIDE),
            enums::DCM_CLKFXDV_DIVIDE::_2,
            enums::DCM_CLKFXDV_DIVIDE::NONE,
        );
        present_dcm.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, bcls::DCM::DUTY_CYCLE_CORRECTION),
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

        let item = ctx.bel_attr_enum(tcid, bslot, bcls::DCM::SPREAD_SPECTRUM);
        present_dcm.apply_enum_diff(
            item,
            enums::DCM_SPREAD_SPECTRUM::DCM,
            enums::DCM_SPREAD_SPECTRUM::MISSING,
        );
        present_dcm_clkgen.discard_bits_enum(item);

        let mut base_dfs_s = BitVec::repeat(false, 87);
        base_dfs_s.set(17, true);
        base_dfs_s.set(21, true);
        base_dfs_s.set(37, true);
        base_dfs_s.set(43, true);
        base_dfs_s.set(52, true);
        base_dfs_s.set(64, true);
        present_dcm.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::REG_DFS_S),
            &base_dfs_s,
            &BitVec::repeat(false, 87),
        );
        present_dcm_clkgen.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::REG_DFS_S),
            &base_dfs_s,
            &BitVec::repeat(false, 87),
        );

        let mut base_dll_s = BitVec::repeat(false, 32);
        base_dll_s.set(0, true);
        base_dll_s.set(6, true);
        base_dll_s.set(13, true); // period not hf
        present_dcm.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::REG_DLL_S),
            &base_dll_s,
            &BitVec::repeat(false, 32),
        );
        present_dcm_clkgen.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::DCM::REG_DLL_S),
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

        present_dcm_clkgen = present_dcm_clkgen.combine(&!&present_dcm);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            bcls::DCM::MODE,
            xlat_enum_attr(vec![(enums::DCM_MODE::DCM_CLKGEN, present_dcm_clkgen)]),
        );

        assert_eq!(present_dcm.bits.len(), 1);
        cfg_interface[18].assert_empty();
        cfg_interface[18] = present_dcm;
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCM::REG_INTERFACE,
            xlat_bitvec(cfg_interface),
        );
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
