use prjcombine_interconnect::db::{BelAttributeType, BelInfo, SwitchBoxItem, TileWireCoord};
use prjcombine_re_collector::diff::{
    OcdMode, extract_bitvec_val_part, extract_common_diff, xlat_bit, xlat_enum_raw,
};
use prjcombine_re_hammer::Session;
use prjcombine_spartan6::defs::{bcls, bslots, tables, tcls, tslots, wires};
use prjcombine_types::{bitvec::BitVec, bsdata::TileBit};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::FuzzIntPip,
        props::{
            mutex::{WireMutexExclusive, WireMutexShared},
            pip::{PinFar, PipWire},
            relation::DeltaSlot,
        },
    },
    spartan6::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let tcid = tcls::CMT_PLL;
    let mut ctx = FuzzCtx::new(session, backend, tcid);
    let tcls = &backend.edev.db[tcid];
    let BelInfo::SwitchBox(ref sb) = tcls.bels[bslots::CMT_INT] else {
        unreachable!()
    };
    let muxes = &backend.edev.db_index.tile_classes[tcid].muxes;
    let mut pairmux = None;
    for item in &sb.items {
        if let SwitchBoxItem::PairMux(mux) = item {
            pairmux = Some(mux);
        }
    }
    let pairmux = pairmux.unwrap();

    let mut bctx = ctx.bel(bslots::PLL);
    let mode = "PLL_ADV";
    bctx.build()
        .global_mutex("CMT", "PRESENT_PLL")
        .global_mutex_here("CMT_PRESENT")
        .extra_tiles_by_bel_special(bslots::CMT_VREG, specials::CMT_PRESENT_ANY_PLL)
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .global_xy("PLLADV_*_USE_CALC", "NO")
        .commit();
    for pin in [
        bcls::PLL::CLKBRST,
        bcls::PLL::CLKINSEL,
        bcls::PLL::ENOUTSYNC,
        bcls::PLL::MANPDLF,
        bcls::PLL::MANPULF,
        bcls::PLL::RST,
        bcls::PLL::SKEWCLKIN1,
        bcls::PLL::SKEWCLKIN2,
        bcls::PLL::SKEWRST,
        bcls::PLL::SKEWSTB,
    ] {
        bctx.mode(mode)
            .global_mutex("CMT", "INV")
            .global_xy("PLLADV_*_USE_CALC", "NO")
            .test_bel_input_inv_auto(pin);
    }
    for (val, vname) in [(false, "REL"), (true, "REL_B")] {
        bctx.mode(mode)
            .global_mutex("CMT", "INV")
            .global_xy("PLLADV_*_USE_CALC", "NO")
            .pip("REL", "TIE_PLL_HARD1")
            .pin("REL")
            .test_bel_attr_bits_bi(bcls::PLL::REL_INV, val)
            .attr("RELINV", vname)
            .commit();
    }

    for (aid, aname, attr) in &backend.edev.db[bcls::PLL].attributes {
        match aid {
            bcls::PLL::DRP
            | bcls::PLL::ENABLE
            | bcls::PLL::CLKINSEL_STATIC_VAL
            | bcls::PLL::CLKINSEL_MODE_DYNAMIC
            | bcls::PLL::REL_INV
            | bcls::PLL::PLL_DIVCLK_EN => {
                // handled elsewhere
            }
            bcls::PLL::PLL_CP_REPL => {
                let BelAttributeType::BitVec(width) = attr.typ else {
                    unreachable!()
                };
                bctx.mode(mode)
                    .global_mutex("CMT", "CP_REPL")
                    .global_xy("PLLADV_*_USE_CALC", "YES")
                    .test_bel_attr_bits(aid)
                    .multi_attr(aname, MultiValue::Dec(0), width);
            }
            bcls::PLL::PLL_EN_CNTRL
            | bcls::PLL::PLL_IN_DLY_MX_SEL
            | bcls::PLL::PLL_IN_DLY_SET
            | bcls::PLL::PLL_CLKFBOUT2_DT
            | bcls::PLL::PLL_CLKFBOUT2_HT
            | bcls::PLL::PLL_CLKFBOUT2_LT
            | bcls::PLL::PLL_CLKFBOUT_DT
            | bcls::PLL::PLL_CLKFBOUT_HT
            | bcls::PLL::PLL_CLKFBOUT_LT
            | bcls::PLL::PLL_CLKFBOUT_PM
            | bcls::PLL::PLL_CLKOUT0_DT
            | bcls::PLL::PLL_CLKOUT0_HT
            | bcls::PLL::PLL_CLKOUT0_LT
            | bcls::PLL::PLL_CLKOUT0_PM
            | bcls::PLL::PLL_CLKOUT1_DT
            | bcls::PLL::PLL_CLKOUT1_HT
            | bcls::PLL::PLL_CLKOUT1_LT
            | bcls::PLL::PLL_CLKOUT1_PM
            | bcls::PLL::PLL_CLKOUT2_DT
            | bcls::PLL::PLL_CLKOUT2_HT
            | bcls::PLL::PLL_CLKOUT2_LT
            | bcls::PLL::PLL_CLKOUT2_PM
            | bcls::PLL::PLL_CLKOUT3_DT
            | bcls::PLL::PLL_CLKOUT3_HT
            | bcls::PLL::PLL_CLKOUT3_LT
            | bcls::PLL::PLL_CLKOUT3_PM
            | bcls::PLL::PLL_CLKOUT4_DT
            | bcls::PLL::PLL_CLKOUT4_HT
            | bcls::PLL::PLL_CLKOUT4_LT
            | bcls::PLL::PLL_CLKOUT4_PM
            | bcls::PLL::PLL_CLKOUT5_DT
            | bcls::PLL::PLL_CLKOUT5_HT
            | bcls::PLL::PLL_CLKOUT5_LT
            | bcls::PLL::PLL_CLKOUT5_PM
            | bcls::PLL::PLL_DIVCLK_HT
            | bcls::PLL::PLL_DIVCLK_LT => {
                let BelAttributeType::BitVec(width) = attr.typ else {
                    unreachable!()
                };

                bctx.mode(mode)
                    .global_mutex("CMT", "TEST")
                    .global_xy("PLLADV_*_USE_CALC", "NO")
                    .test_bel_attr_bits(aid)
                    .multi_attr(aname, MultiValue::Bin, width);
            }

            _ => match attr.typ {
                BelAttributeType::Bool => {
                    bctx.mode(mode)
                        .global_mutex("CMT", "TEST")
                        .global_xy("PLLADV_*_USE_CALC", "NO")
                        .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                }
                BelAttributeType::BitVec(width) => {
                    bctx.mode(mode)
                        .global_mutex("CMT", "TEST")
                        .global_xy("PLLADV_*_USE_CALC", "NO")
                        .test_bel_attr_bits(aid)
                        .multi_attr(aname, MultiValue::Dec(0), width);
                }
                _ => unreachable!(),
            },
        }
    }

    for (spec, attr, multi, width) in [
        (specials::PLL_MISC, "PLL_MISC", MultiValue::Bin, 4),
        (specials::PLL_OPT_INV, "PLL_OPT_INV", MultiValue::Bin, 6),
        (specials::PLL_DIVCLK_DT, "PLL_DIVCLK_DT", MultiValue::Bin, 6),
        (
            specials::PLL_IO_CLKSRC,
            "PLL_IO_CLKSRC",
            MultiValue::Dec(0),
            2,
        ),
        (
            specials::PLL_SKEW_CNTRL,
            "PLL_SKEW_CNTRL",
            MultiValue::Dec(0),
            2,
        ),
    ] {
        bctx.mode(mode)
            .global_mutex("CMT", "TEST")
            .global_xy("PLLADV_*_USE_CALC", "NO")
            .test_bel_special_bits(spec)
            .multi_attr(attr, multi, width);
    }

    for (spec, val) in [
        (
            specials::PLL_COMPENSATION_SOURCE_SYNCHRONOUS,
            "SOURCE_SYNCHRONOUS",
        ),
        (
            specials::PLL_COMPENSATION_SYSTEM_SYNCHRONOUS,
            "SYSTEM_SYNCHRONOUS",
        ),
        (specials::PLL_COMPENSATION_PLL2DCM, "PLL2DCM"),
        (specials::PLL_COMPENSATION_DCM2PLL, "DCM2PLL"),
        (specials::PLL_COMPENSATION_EXTERNAL, "EXTERNAL"),
        (specials::PLL_COMPENSATION_INTERNAL, "INTERNAL"),
    ] {
        bctx.mode(mode)
            .global_mutex("CMT", "CALC")
            .global_xy("PLLADV_*_USE_CALC", "NO")
            .test_bel_special(spec)
            .attr("COMPENSATION", val)
            .commit();
    }

    for (row, rname, _) in &backend.edev.db.tables[tables::PLL_MULT].rows {
        for (bandwidth, spec) in [
            ("LOW", specials::PLL_TABLES_LOW),
            ("HIGH", specials::PLL_TABLES_HIGH),
        ] {
            bctx.mode(mode)
                .global_mutex("CMT", "CALC")
                .global_xy("PLLADV_*_USE_CALC", "NO")
                .test_bel_special_row(spec, row)
                .attr_diff("CLKFBOUT_MULT", "0", rname.strip_prefix('_').unwrap())
                .attr_diff("BANDWIDTH", "LOW", bandwidth)
                .commit();
        }
    }

    let obel_dcm0 = bslots::DCM[0];
    let obel_dcm1 = bslots::DCM[1];
    let relation_dcm = DeltaSlot::new(0, -16, tslots::BEL);

    for (i, dst) in pairmux.dst.into_iter().enumerate() {
        let opin = ["CLKIN1", "CLKIN2"][i];
        for &srcs in pairmux.src.keys() {
            let Some(src) = srcs[i] else {
                continue;
            };
            let mut builder = bctx
                .mode(mode)
                .global_xy("PLLADV_*_USE_CALC", "NO")
                .global_mutex("CMT", format!("TEST_{opin}"))
                .pip((PinFar, "CLKFBIN"), "CLKFBIN_CKINT0")
                .prop(WireMutexShared::new(src.tw))
                .prop(WireMutexExclusive::new(dst));

            if wires::DIVCLK_CMT_W.contains(src.wire)
                | wires::DIVCLK_CMT_E.contains(src.wire)
                | wires::DIVCLK_CMT_V.contains(src.wire)
            {
                builder = builder.global_mutex("BUFIO2_CMT_OUT", "USE").related_pip(
                    relation_dcm.clone(),
                    (obel_dcm0, "CLKIN"),
                    PipWire::AltInt(src.tw),
                )
            }

            builder
                .test_routing(dst, src)
                .prop(FuzzIntPip::new(dst, src.tw))
                .commit();
        }
    }

    bctx.mode(mode)
        .global_xy("PLLADV_*_USE_CALC", "NO")
        .global_mutex("CMT", "TEST_CLKIN1_BOTH")
        .mutex("CLKIN_IN", "CLKIN1_CKINT0")
        .pip((PinFar, "CLKFBIN"), "CLKFBIN_CKINT0")
        .pip((PinFar, "CLKIN2"), "CLKIN2_CKINT0")
        .test_bel_attr_bits(bcls::PLL::CLKINSEL_MODE_DYNAMIC)
        .pip((PinFar, "CLKIN1"), "CLKIN1_CKINT0")
        .commit();

    {
        let mux = &muxes[&TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKFB)];
        for &src in mux.src.keys() {
            let mut builder = bctx
                .mode(mode)
                .global_mutex("CMT", "TEST_CLKFBIN")
                .global_xy("PLLADV_*_USE_CALC", "NO")
                .pip((PinFar, "CLKIN1"), "CLKIN1_CKINT0")
                .prop(WireMutexShared::new(src.tw))
                .prop(WireMutexExclusive::new(mux.dst));

            if wires::IOFBCLK_CMT_W.contains(src.wire)
                | wires::IOFBCLK_CMT_E.contains(src.wire)
                | wires::IOFBCLK_CMT_V.contains(src.wire)
            {
                builder = builder.global_mutex("BUFIO2_CMT_OUT", "USE").related_pip(
                    relation_dcm.clone(),
                    (obel_dcm0, "CLKFB"),
                    PipWire::AltInt(src.tw),
                )
            }

            builder
                .test_routing(mux.dst, src)
                .prop(FuzzIntPip::new(mux.dst, src.tw))
                .commit();
        }
        bctx.mode(mode)
            .global_mutex("CMT", "TEST_CLKFBIN")
            .prop(WireMutexExclusive::new(mux.dst))
            .pip((PinFar, "CLKIN1"), "CLKIN1_CKINT0")
            .test_bel_special(specials::PLL_CLKFB_CLKOUT0)
            .pip((PinFar, "CLKFBIN"), "CLKOUT0")
            .commit();
    }

    for (w, w_buf) in [
        (wires::OMUX_PLL_SKEWCLKIN1, wires::OMUX_PLL_SKEWCLKIN1_BUF),
        (wires::OMUX_PLL_SKEWCLKIN2, wires::OMUX_PLL_SKEWCLKIN2_BUF),
    ] {
        let mux = &muxes[&TileWireCoord::new_idx(1, w)];
        for &src in mux.src.keys() {
            bctx.build()
                .global_mutex("CMT", "MUX_PLL")
                .mode(mode)
                .extra_tile_routing(
                    relation_dcm.clone(),
                    TileWireCoord::new_idx(1, w_buf),
                    TileWireCoord::new_idx(2, w).pos(),
                )
                .prop(WireMutexShared::new(src.tw))
                .prop(WireMutexExclusive::new(mux.dst))
                .test_routing(mux.dst, src)
                .prop(FuzzIntPip::new(mux.dst, src.tw))
                .commit();
        }
    }

    {
        let mux = &muxes[&TileWireCoord::new_idx(1, wires::CMT_TEST_CLK)];
        for &src in mux.src.keys() {
            bctx.build()
                .global_mutex("CMT", "MUX_PLL")
                .related_pip(
                    relation_dcm.clone(),
                    (obel_dcm0, "CLKIN"),
                    (obel_dcm0, "CLKIN_CKINT0"),
                )
                .related_pip(
                    relation_dcm.clone(),
                    (obel_dcm1, "CLKIN"),
                    (obel_dcm1, "CLKIN_CKINT0"),
                )
                .related_pip(
                    relation_dcm.clone(),
                    (obel_dcm0, "CLKFB"),
                    (obel_dcm0, "CLKFB_CKINT0"),
                )
                .related_pip(
                    relation_dcm.clone(),
                    (obel_dcm1, "CLKFB"),
                    (obel_dcm1, "CLKFB_CKINT0"),
                )
                .related_tile_mutex(relation_dcm.clone(), "CLKIN_BEL", "PLL")
                .prop(WireMutexShared::new(src.tw))
                .prop(WireMutexExclusive::new(mux.dst))
                .test_routing(mux.dst, src)
                .prop(FuzzIntPip::new(mux.dst, src.tw))
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, skip_dcm: bool) {
    let tcid = tcls::CMT_PLL;
    let tcls = &ctx.edev.db[tcid];
    let BelInfo::SwitchBox(ref sb) = tcls.bels[bslots::CMT_INT] else {
        unreachable!()
    };
    let muxes = &ctx.edev.db_index.tile_classes[tcid].muxes;
    let mut pairmux = None;
    for item in &sb.items {
        if let SwitchBoxItem::PairMux(mux) = item {
            pairmux = Some(mux);
        }
    }
    let pairmux = pairmux.unwrap();

    let bslot = bslots::PLL;

    fn reg_bit(addr: usize, bit: usize) -> TileBit {
        let slot = match addr {
            0..6 => 22 + addr,
            6..0x1c => 36 + (addr - 6),
            0x1c.. => 59 + (addr - 0x1c),
        };
        TileBit::new(slot / 4, 30, (slot % 4) * 16 + bit)
    }

    let mut drp = vec![];
    for addr in 0..0x20 {
        for bit in 0..16 {
            drp.push(reg_bit(addr, bit).pos());
        }
    }
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::PLL::DRP, drp);

    let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);

    for pin in [
        bcls::PLL::CLKBRST,
        bcls::PLL::CLKINSEL,
        bcls::PLL::ENOUTSYNC,
        bcls::PLL::MANPDLF,
        bcls::PLL::MANPULF,
        bcls::PLL::RST,
        bcls::PLL::SKEWRST,
        bcls::PLL::SKEWSTB,
    ] {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }
    ctx.collect_bel_attr_bi(tcid, bslot, bcls::PLL::REL_INV);

    // hm.
    for pin in [bcls::PLL::SKEWCLKIN1, bcls::PLL::SKEWCLKIN2] {
        ctx.get_diff_bel_input_inv(tcid, bslot, pin, false)
            .assert_empty();
        ctx.get_diff_bel_input_inv(tcid, bslot, pin, true)
            .assert_empty();
    }

    for (aid, _, attr) in &ctx.edev.db[bcls::PLL].attributes {
        match aid {
            bcls::PLL::DRP
            | bcls::PLL::ENABLE
            | bcls::PLL::CLKINSEL_STATIC_VAL
            | bcls::PLL::CLKINSEL_MODE_DYNAMIC
            | bcls::PLL::REL_INV
            | bcls::PLL::PLL_DIVCLK_EN
            | bcls::PLL::PLL_CP
            | bcls::PLL::PLL_EN => {
                // handled elsewhere
            }
            _ => match attr.typ {
                BelAttributeType::Bool => {
                    ctx.collect_bel_attr_bi(tcid, bslot, aid);
                }
                BelAttributeType::BitVec(_) => {
                    ctx.collect_bel_attr(tcid, bslot, aid);
                }
                _ => unreachable!(),
            },
        }
    }

    for (spec, width) in [(specials::PLL_MISC, 4), (specials::PLL_DIVCLK_DT, 6)] {
        for diff in ctx.get_diffs_bel_special_bits(tcid, bslot, spec, width) {
            diff.assert_empty();
        }
    }

    // sigh. bug. again. murder me with a rusty spoon.
    for diff in ctx.get_diffs_attr_bits(tcid, bslot, bcls::PLL::PLL_CP, 4) {
        diff.assert_empty();
    }
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslot,
        bcls::PLL::PLL_CP,
        vec![
            reg_bit(0x18, 13).pos(),
            reg_bit(0x18, 10).pos(),
            reg_bit(0x18, 11).pos(),
            reg_bit(0x18, 9).pos(),
        ],
    );

    ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::PLL::PLL_EN, false)
        .assert_empty();
    ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::PLL::PLL_EN, true)
        .assert_empty();
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::PLL::PLL_EN, reg_bit(0x1a, 8).pos());

    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        bcls::PLL::PLL_DIVCLK_EN,
        reg_bit(0x16, 0).pos(),
    );

    for w in [
        wires::OMUX_PLL_SKEWCLKIN1,
        wires::OMUX_PLL_SKEWCLKIN2,
        wires::CMT_TEST_CLK,
        wires::IMUX_PLL_CLKFB,
    ] {
        let mux = &muxes[&TileWireCoord::new_idx(1, w)];
        let mut diffs = vec![];
        let mut got_empty = false;
        for &src in mux.src.keys() {
            let diff = ctx.get_diff_routing(tcid, mux.dst, src);
            if diff.bits.is_empty() {
                got_empty = true;
            }
            diffs.push((Some(src), diff));
        }
        if !got_empty {
            diffs.push((None, Default::default()));
        }
        ctx.insert_mux(tcid, mux.dst, xlat_enum_raw(diffs, OcdMode::Mux));
    }

    ctx.collect_bel_attr(tcid, bslot, bcls::PLL::CLKINSEL_MODE_DYNAMIC);

    // ????
    ctx.get_diff_bel_special(tcid, bslot, specials::PLL_CLKFB_CLKOUT0)
        .assert_empty();

    let mut diffs_clkin1 = vec![];
    let mut diffs_clkin2 = vec![];
    for &[src0, src1] in pairmux.src.keys() {
        if let Some(src) = src0 {
            diffs_clkin1.push((
                [src0, src1],
                ctx.get_diff_routing(tcid, pairmux.dst[0], src),
            ));
        }
        if let Some(src) = src1 {
            diffs_clkin2.push((
                [src0, src1],
                ctx.get_diff_routing(tcid, pairmux.dst[1], src),
            ));
        }
    }
    let diff = extract_common_diff(&mut diffs_clkin2);
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::PLL::CLKINSEL_STATIC_VAL, xlat_bit(diff));
    let mut diffs = diffs_clkin1;
    diffs.extend(diffs_clkin2);
    ctx.insert_pairmux(tcid, pairmux.dst, xlat_enum_raw(diffs, OcdMode::Mux));

    let mut diffs = ctx.get_diffs_bel_special_bits(tcid, bslot, specials::PLL_IO_CLKSRC, 2);
    diffs[0].apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::CLKINSEL_MODE_DYNAMIC),
        true,
        false,
    );
    diffs[1].apply_bit_diff(
        ctx.bel_input_inv(tcid, bslot, bcls::PLL::CLKINSEL),
        false,
        true,
    );
    for diff in diffs {
        diff.assert_empty();
    }

    let mut diffs = ctx.get_diffs_bel_special_bits(tcid, bslot, specials::PLL_OPT_INV, 6);
    diffs[0].apply_bit_diff(ctx.bel_input_inv(tcid, bslot, bcls::PLL::RST), true, false);
    diffs[1].apply_bit_diff(
        ctx.bel_input_inv(tcid, bslot, bcls::PLL::MANPDLF),
        true,
        false,
    );
    diffs[2].apply_bit_diff(
        ctx.bel_input_inv(tcid, bslot, bcls::PLL::MANPULF),
        true,
        false,
    );
    diffs[3].apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::REL_INV),
        false,
        true,
    );
    diffs[4].apply_bit_diff(
        ctx.bel_input_inv(tcid, bslot, bcls::PLL::CLKBRST),
        true,
        false,
    );
    diffs[5].apply_bit_diff(
        ctx.bel_input_inv(tcid, bslot, bcls::PLL::ENOUTSYNC),
        true,
        false,
    );
    for diff in diffs {
        diff.assert_empty();
    }

    let mut diffs = ctx.get_diffs_bel_special_bits(tcid, bslot, specials::PLL_SKEW_CNTRL, 2);
    diffs[0].apply_bit_diff(
        ctx.bel_input_inv(tcid, bslot, bcls::PLL::SKEWSTB),
        true,
        false,
    );
    diffs[1].apply_bit_diff(
        ctx.bel_input_inv(tcid, bslot, bcls::PLL::SKEWRST),
        true,
        false,
    );
    for diff in diffs {
        diff.assert_empty();
    }

    // um?
    present.apply_enum_diff_raw(
        ctx.sb_mux(tcid, TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKFB)),
        &Some(TileWireCoord::new_idx(1, wires::IOFBCLK_CMT_W[3]).pos()),
        &Some(TileWireCoord::new_idx(1, wires::IOFBCLK_CMT_V[0]).pos()),
    );
    present.apply_enum_diff_raw(
        ctx.sb_pairmux(
            tcid,
            [
                TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKIN1),
                TileWireCoord::new_idx(1, wires::IMUX_PLL_CLKIN2),
            ],
        ),
        &[
            Some(TileWireCoord::new_idx(1, wires::DIVCLK_CMT_E[3]).pos()),
            Some(TileWireCoord::new_idx(1, wires::DIVCLK_CMT_W[3]).pos()),
        ],
        &[
            Some(TileWireCoord::new_idx(1, wires::DIVCLK_CMT_V[0]).pos()),
            Some(TileWireCoord::new_idx(1, wires::DIVCLK_CMT_V[4]).pos()),
        ],
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::CLKINSEL_STATIC_VAL),
        true,
        false,
    );
    present.apply_enum_diff_raw(
        ctx.sb_mux(tcid, TileWireCoord::new_idx(1, wires::OMUX_PLL_SKEWCLKIN2)),
        &None,
        &Some(TileWireCoord::new_idx(1, wires::OUT_PLL_CLKOUTDCM[0]).pos()),
    );
    present.apply_enum_diff_raw(
        ctx.sb_mux(tcid, TileWireCoord::new_idx(1, wires::OMUX_PLL_SKEWCLKIN1)),
        &None,
        &Some(TileWireCoord::new_idx(1, wires::OUT_PLL_CLKOUTDCM[0]).pos()),
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_CLKOUT0_NOCOUNT),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_CLKOUT1_NOCOUNT),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_CLKOUT2_NOCOUNT),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_CLKOUT3_NOCOUNT),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_CLKOUT4_NOCOUNT),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_CLKOUT5_NOCOUNT),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_CLKFBOUT_NOCOUNT),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_DIVCLK_NOCOUNT),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_EN_DLY),
        false,
        true,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_EN),
        true,
        false,
    );
    present.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_DIVCLK_EN),
        true,
        false,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_DIVCLK_LT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_DIVCLK_HT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT0_LT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT0_HT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT1_LT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT1_HT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT2_LT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT2_HT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT3_LT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT3_HT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT4_LT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT4_HT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT5_LT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKOUT5_HT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKFBOUT_LT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CLKFBOUT_HT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_SET),
        0x11,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_MX_SEL),
        0xa,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_LOCK_CNT),
        0x3e8,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_UNLOCK_CNT),
        1,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_LOCK_SAT_HIGH),
        0x3e9,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_LOCK_REF_DLY),
        0x9,
        0,
    );
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_LOCK_FB_DLY),
        0x7,
        0,
    );
    present.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_LFHF), 3, 0);
    present.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_RES), 11, 0);
    present.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CP), 2, 0);
    present.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_CP_REPL),
        2,
        0,
    );

    ctx.insert_bel_attr_bool(tcid, bslot, bcls::PLL::ENABLE, xlat_bit(present));

    ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_SYSTEM_SYNCHRONOUS)
        .assert_empty();
    let mut diff =
        ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_SOURCE_SYNCHRONOUS);
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_SET),
        0,
        0x11,
    );
    diff.assert_empty();
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_EXTERNAL);
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_SET),
        0,
        0x11,
    );
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_MX_SEL),
        0,
        0xa,
    );
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_EN_DLY),
        true,
        false,
    );
    diff.assert_empty();
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_INTERNAL);
    diff.apply_bitvec_diff_int(ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_INTFB), 2, 0);
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_SET),
        0,
        0x11,
    );
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_MX_SEL),
        0,
        0xa,
    );
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_EN_DLY),
        true,
        false,
    );
    diff.assert_empty();
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_DCM2PLL);
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_SET),
        0,
        0x11,
    );
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_MX_SEL),
        0,
        0xa,
    );
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_EN_DLY),
        true,
        false,
    );
    diff.assert_empty();
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PLL_COMPENSATION_PLL2DCM);
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_SET),
        0,
        0x11,
    );
    diff.apply_bitvec_diff_int(
        ctx.bel_attr_bitvec(tcid, bslot, bcls::PLL::PLL_IN_DLY_MX_SEL),
        0,
        0xa,
    );
    diff.apply_bit_diff(
        ctx.bel_attr_bit(tcid, bslot, bcls::PLL::PLL_EN_DLY),
        true,
        false,
    );
    diff.assert_empty();

    for row in ctx.edev.db.tables[tables::PLL_MULT].rows.ids() {
        for (spec, field_cp_repl, field_cp, field_res, field_lfhf) in [
            (
                specials::PLL_TABLES_LOW,
                tables::PLL_MULT::PLL_CP_REPL_LOW,
                tables::PLL_MULT::PLL_CP_LOW,
                tables::PLL_MULT::PLL_RES_LOW,
                tables::PLL_MULT::PLL_LFHF_LOW,
            ),
            (
                specials::PLL_TABLES_HIGH,
                tables::PLL_MULT::PLL_CP_REPL_HIGH,
                tables::PLL_MULT::PLL_CP_HIGH,
                tables::PLL_MULT::PLL_RES_HIGH,
                tables::PLL_MULT::PLL_LFHF_HIGH,
            ),
        ] {
            let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
            for (field, attr, width) in [
                (
                    tables::PLL_MULT::PLL_LOCK_REF_DLY,
                    bcls::PLL::PLL_LOCK_REF_DLY,
                    5,
                ),
                (
                    tables::PLL_MULT::PLL_LOCK_FB_DLY,
                    bcls::PLL::PLL_LOCK_FB_DLY,
                    5,
                ),
                (tables::PLL_MULT::PLL_LOCK_CNT, bcls::PLL::PLL_LOCK_CNT, 10),
                (
                    tables::PLL_MULT::PLL_LOCK_SAT_HIGH,
                    bcls::PLL::PLL_LOCK_SAT_HIGH,
                    10,
                ),
                (
                    tables::PLL_MULT::PLL_UNLOCK_CNT,
                    bcls::PLL::PLL_UNLOCK_CNT,
                    10,
                ),
            ] {
                let val = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslot, attr),
                    &BitVec::repeat(false, width),
                    &mut diff,
                );
                ctx.insert_table_bitvec(tables::PLL_MULT, row, field, val);
            }
            for (field, attr, width) in [
                (field_cp_repl, bcls::PLL::PLL_CP_REPL, 4),
                (field_cp, bcls::PLL::PLL_CP, 4),
                (field_res, bcls::PLL::PLL_RES, 4),
                (field_lfhf, bcls::PLL::PLL_LFHF, 2),
            ] {
                let val = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslot, attr),
                    &BitVec::repeat(false, width),
                    &mut diff,
                );
                ctx.insert_table_bitvec(tables::PLL_MULT, row, field, val);
            }
            for attr in [
                bcls::PLL::PLL_CLKFBOUT_NOCOUNT,
                bcls::PLL::PLL_CLKFBOUT_LT,
                bcls::PLL::PLL_CLKFBOUT_HT,
                bcls::PLL::PLL_CLKFBOUT_EDGE,
            ] {
                diff.discard_polbits(ctx.bel_attr_bitvec(tcid, bslot, attr));
            }
            diff.assert_empty();
        }
    }

    let tcid = tcls::CMT_DCM;
    let bslot = bslots::CMT_VREG;
    let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::CMT_PRESENT_ANY_PLL);
    if !skip_dcm {
        diff.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, bcls::CMT_VREG::REG_BG),
            0x520,
            1,
        );
        diff.assert_empty();
    }
    for (wt, wf) in [
        (wires::OMUX_PLL_SKEWCLKIN2_BUF, wires::OMUX_PLL_SKEWCLKIN2),
        (wires::OMUX_PLL_SKEWCLKIN1_BUF, wires::OMUX_PLL_SKEWCLKIN1),
    ] {
        let dst = TileWireCoord::new_idx(1, wt);
        let src = TileWireCoord::new_idx(2, wf).pos();

        ctx.collect_progbuf(tcid, dst, src);
    }
}
