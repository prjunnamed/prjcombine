use prjcombine_interconnect::db::{BelInfo, BelKind};
use prjcombine_re_collector::diff::{Diff, OcdMode, xlat_bit_wide, xlat_enum_raw};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    bcls, bslots,
    virtex4::{tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::mutex::{WireMutexExclusive, WireMutexShared},
    },
    virtex4::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::CCM) else {
        return;
    };
    for idx in 0..2 {
        let mut bctx = ctx.bel(bslots::PMCD[idx]);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("PMCD")
            .commit();
        for (pin, attr) in [
            ("CLKA", bcls::PMCD::CLKA_ENABLE),
            ("CLKB", bcls::PMCD::CLKB_ENABLE),
            ("CLKC", bcls::PMCD::CLKC_ENABLE),
            ("CLKD", bcls::PMCD::CLKD_ENABLE),
        ] {
            bctx.mode("PMCD").test_bel_attr_bits(attr).pin(pin).commit();
        }
        for pin in [bcls::PMCD::REL, bcls::PMCD::RST] {
            bctx.mode("PMCD").test_bel_input_inv_auto(pin);
        }
        bctx.mode("PMCD")
            .test_bel_attr_bool_auto(bcls::PMCD::EN_REL, "FALSE", "TRUE");
        bctx.mode("PMCD")
            .test_bel_attr_auto(bcls::PMCD::RST_DEASSERT_CLK);
        bctx.mode("PMCD")
            .tile_mutex("VREG", format!("PMCD{idx}"))
            .test_bel(bslots::CCM)
            .test_bel_attr_bool_rename("CCM_VREG_ENABLE", bcls::CCM::VREG_ENABLE, "FALSE", "TRUE");
        // ???
        bctx.mode("PMCD")
            .null_bits()
            .tile_mutex("VREG", format!("PMCD{idx}"))
            .test_bel(bslots::CCM)
            .test_bel_attr_bits(bcls::CCM::VBG_SEL)
            .multi_attr("CCM_VBG_SEL", MultiValue::Bin, 4);
        bctx.mode("PMCD")
            .null_bits()
            .tile_mutex("VREG", format!("PMCD{idx}"))
            .test_bel(bslots::CCM)
            .test_bel_attr_bits(bcls::CCM::VBG_PD)
            .multi_attr("CCM_VBG_PD", MultiValue::Bin, 2);
        bctx.mode("PMCD")
            .null_bits()
            .tile_mutex("VREG", format!("PMCD{idx}"))
            .test_bel(bslots::CCM)
            .test_bel_attr_bits(bcls::CCM::VREG_PHASE_MARGIN)
            .multi_attr("CCM_VREG_PHASE_MARGIN", MultiValue::Bin, 3);
    }
    {
        let mut bctx = ctx.bel(bslots::DPM);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode("DPM")
            .commit();
        for pin in [
            bcls::DPM::ENOSC[0],
            bcls::DPM::ENOSC[1],
            bcls::DPM::ENOSC[2],
            bcls::DPM::OUTSEL[0],
            bcls::DPM::OUTSEL[1],
            bcls::DPM::OUTSEL[2],
            bcls::DPM::HFSEL[0],
            bcls::DPM::HFSEL[1],
            bcls::DPM::HFSEL[2],
            bcls::DPM::RST,
            bcls::DPM::SELSKEW,
            bcls::DPM::FREEZE,
        ] {
            bctx.mode("DPM").test_bel_input_inv_auto(pin);
        }
        bctx.mode("DPM")
            .tile_mutex("VREG", "DPM")
            .test_bel(bslots::CCM)
            .test_bel_attr_bool_rename("CCM_VREG_ENABLE", bcls::CCM::VREG_ENABLE, "FALSE", "TRUE");
        // ???
        bctx.mode("DPM")
            .null_bits()
            .tile_mutex("VREG", "DPM")
            .test_bel(bslots::CCM)
            .test_bel_attr_bits(bcls::CCM::VBG_SEL)
            .multi_attr("CCM_VBG_SEL", MultiValue::Bin, 4);
        bctx.mode("DPM")
            .null_bits()
            .tile_mutex("VREG", "DPM")
            .test_bel(bslots::CCM)
            .test_bel_attr_bits(bcls::CCM::VBG_PD)
            .multi_attr("CCM_VBG_PD", MultiValue::Bin, 2);
        bctx.mode("DPM")
            .null_bits()
            .tile_mutex("VREG", "DPM")
            .test_bel(bslots::CCM)
            .test_bel_attr_bits(bcls::CCM::VREG_PHASE_MARGIN)
            .multi_attr("CCM_VREG_PHASE_MARGIN", MultiValue::Bin, 3);
    }

    let muxes = &backend.edev.db_index.tile_classes[tcls::CCM].muxes;
    for (bslot, bel) in &backend.edev.db[tcls::CCM].bels {
        let BelInfo::Bel(bel) = bel else {
            continue;
        };
        let BelKind::Class(bcid) = backend.edev.db.bel_slots[bslot].kind else {
            unreachable!()
        };
        let mut bctx = ctx.bel(bslot);
        let mode = if bslot == bslots::DPM { "DPM" } else { "PMCD" };
        for (pin, inp) in &bel.inputs {
            let wire = inp.wire();
            let (mux, extra_in) = if wires::IMUX_SPEC.contains(wire.wire) {
                (&muxes[&wire], None)
            } else if wires::IMUX_CCM_REL.contains(wire.wire) {
                let mut extra_in = None;
                let mut spec = None;
                for &src in muxes[&wire].src.keys() {
                    if wires::IMUX_SPEC.contains(src.wire) {
                        spec = Some(src.tw);
                    } else {
                        extra_in = Some(src.tw);
                    }
                }
                (&muxes[&spec.unwrap()], extra_in)
            } else {
                continue;
            };
            let pname = backend.edev.db[bcid].inputs.key(pin).0;
            let opin = if bslot == bslots::DPM {
                if pin == bcls::DPM::REFCLK {
                    bcls::DPM::TESTCLK1
                } else {
                    bcls::DPM::REFCLK
                }
            } else {
                if pin == bcls::PMCD::CLKA {
                    bcls::PMCD::CLKB
                } else {
                    bcls::PMCD::CLKA
                }
            };
            let opname = backend.edev.db[bcid].inputs.key(opin).0;
            let odst = bel.inputs[opin].wire();
            for out in [pname, format!("{pname}_TEST").as_str()] {
                for &src in mux.src.keys() {
                    let mut builder = bctx
                        .build()
                        .mode(mode)
                        .pin(pname)
                        .prop(WireMutexExclusive::new(mux.dst));
                    if wires::HCLK_DCM.contains(src.wire)
                        || wires::GIOB_DCM.contains(src.wire)
                        || wires::MGT_DCM.contains(src.wire)
                        || wires::DCM_DCM_I.contains(src.wire)
                    {
                        if !wires::DCM_DCM_I.contains(src.wire) {
                            builder = builder.global_mutex("HCLK_DCM", "USE");
                        }
                        builder = builder
                            .pin(opname)
                            .prop(WireMutexExclusive::new(odst))
                            .pip(opname, src.tw);
                    }
                    if wires::IMUX_CLK_OPTINV.contains(src.wire) {
                        builder = builder.prop(WireMutexExclusive::new(src.tw));
                    } else {
                        builder = builder.prop(WireMutexShared::new(src.tw));
                    }
                    builder.test_routing(mux.dst, src).pip(out, src.tw).commit();
                }
            }
            if let Some(extra) = extra_in {
                bctx.build()
                    .mode(mode)
                    .pin(pname)
                    .prop(WireMutexExclusive::new(mux.dst))
                    .prop(WireMutexShared::new(extra))
                    .test_routing(wire, extra.pos())
                    .pip(pname, extra)
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::CCM;
    if !ctx.has_tcls(tcid) {
        return;
    }
    for bslot in bslots::PMCD {
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 4], tcid, bslot, bcls::PMCD::RST);
        ctx.collect_bel_input_inv_bi(tcid, bslot, bcls::PMCD::REL);
        let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::PMCD::CLKA_ENABLE));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::PMCD::CLKA_ENABLE, bits);
        ctx.collect_bel_attr(tcid, bslot, bcls::PMCD::CLKB_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::PMCD::CLKC_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::PMCD::CLKD_ENABLE);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::PMCD::EN_REL);
        ctx.collect_bel_attr(tcid, bslot, bcls::PMCD::RST_DEASSERT_CLK);
    }
    {
        let bslot = bslots::DPM;
        ctx.collect_bel_input_inv_int_bi(&[tcls::INT; 4], tcid, bslot, bcls::DPM::RST);
        for pin in [
            bcls::DPM::ENOSC[0],
            bcls::DPM::ENOSC[1],
            bcls::DPM::ENOSC[2],
            bcls::DPM::OUTSEL[0],
            bcls::DPM::OUTSEL[1],
            bcls::DPM::OUTSEL[2],
            bcls::DPM::HFSEL[0],
            bcls::DPM::HFSEL[1],
            bcls::DPM::HFSEL[2],
            bcls::DPM::SELSKEW,
            bcls::DPM::FREEZE,
        ] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
    }
    {
        let bslot = bslots::CCM;
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::CCM::VREG_ENABLE);
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::CCM::VBG_SEL,
            (22..26).map(|bit| TileBit::new(3, 20, bit).pos()).collect(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::CCM::VBG_PD,
            (26..28).map(|bit| TileBit::new(3, 20, bit).pos()).collect(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::CCM::VREG_PHASE_MARGIN,
            (29..32).map(|bit| TileBit::new(3, 20, bit).pos()).collect(),
        );
    }

    for mux in ctx.edev.db_index.tile_classes[tcid].muxes.values() {
        if wires::IMUX_SPEC.contains(mux.dst.wire) {
            let mut diffs = vec![];
            for &src in mux.src.keys() {
                let mut diff = ctx.get_diff_routing(tcid, mux.dst, src);
                if wires::IMUX_CLK_OPTINV.contains(src.wire) {
                    let item = ctx.item_int_inv_raw(&[tcls::INT; 4], src.tw);
                    diff.apply_bit_diff(item, false, true);
                }
                diffs.push((Some(src), diff));
            }
            ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::Mux));
        } else if wires::IMUX_CCM_REL.contains(mux.dst.wire) {
            let mut diffs = vec![];
            for &src in mux.src.keys() {
                let diff = if wires::IMUX_SPEC.contains(src.wire) {
                    Diff::default()
                } else {
                    ctx.get_diff_routing(tcid, mux.dst, src)
                };
                diffs.push((Some(src), diff));
            }
            ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::Mux));
        }
    }
}
