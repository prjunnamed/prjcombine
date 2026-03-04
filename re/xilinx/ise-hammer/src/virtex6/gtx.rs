use prjcombine_interconnect::db::{BelAttributeType, BelInputId, WireSlotIdExt};
use prjcombine_re_collector::diff::{Diff, OcdMode, xlat_bit, xlat_enum_attr};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    bcls::{GTCLK, GTX, HCLK_DRP, HCLK_GTX},
    bslots, enums,
    virtex6::{tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::BaseIntPip,
        props::{mutex::WireMutexExclusive, pip::PinFar, relation::Delta},
    },
    virtex4::specials,
};

const GTX_INVPINS: &[BelInputId] = &[
    GTX::DCLK,
    GTX::RXUSRCLK,
    GTX::RXUSRCLK2,
    GTX::TXUSRCLK,
    GTX::TXUSRCLK2,
    GTX::TSTCLK.index_const(0),
    GTX::TSTCLK.index_const(1),
    GTX::SCANCLK,
    GTX::GREFCLKRX,
    GTX::GREFCLKTX,
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::GTX) else {
        return;
    };
    for i in 0..4 {
        let mut bctx = ctx.bel(bslots::GTX[i]);
        let bel_other = bslots::GTX[i ^ 1];
        let mode = "GTXE1";
        bctx.build()
            .bel_unused(bel_other)
            .extra_tile_attr_bits(
                Delta::new(0, 0, tcls::HCLK),
                bslots::HCLK_DRP[0],
                if i < 2 {
                    HCLK_DRP::DRP_MASK_S
                } else {
                    HCLK_DRP::DRP_MASK_N
                },
            )
            .test_bel_attr_bits(GTX::GTX_CFG_PWRUP)
            .mode(mode)
            .commit();
        for &pin in GTX_INVPINS {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }

        for (aid, aname, attr) in &backend.edev.db[GTX].attributes {
            match aid {
                GTX::DRP
                | GTX::GTX_CFG_PWRUP
                | GTX::PMA_CAS_CLK_EN
                | GTX::RXPLLREFSEL_STATIC_VAL
                | GTX::RXPLLREFSEL_MODE_DYNAMIC
                | GTX::RXPLLREFSEL_TESTCLK
                | GTX::TXPLLREFSEL_STATIC_VAL
                | GTX::TXPLLREFSEL_MODE_DYNAMIC
                | GTX::TXPLLREFSEL_TESTCLK => (),

                GTX::CLK_COR_REPEAT_WAIT
                | GTX::RXBUF_OVFL_THRESH
                | GTX::RXBUF_UDFL_THRESH
                | GTX::RX_SLIDE_AUTO_WAIT
                | GTX::TXOUTCLKPCS_SEL => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                }
                GTX::RX_CLK25_DIVIDER | GTX::TX_CLK25_DIVIDER => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(1));
                }

                GTX::BIAS_CFG
                | GTX::PMA_CDR_SCAN
                | GTX::PMA_CFG
                | GTX::PMA_RXSYNC_CFG
                | GTX::PMA_RX_CFG
                | GTX::PMA_TX_CFG
                | GTX::RXPLL_COM_CFG
                | GTX::RXPLL_CP_CFG
                | GTX::RXUSRCLK_DLY
                | GTX::RX_EYE_OFFSET
                | GTX::TRANS_TIME_FROM_P2
                | GTX::TRANS_TIME_NON_P2
                | GTX::TRANS_TIME_RATE
                | GTX::TRANS_TIME_TO_P2
                | GTX::TST_ATTR
                | GTX::TXPLL_COM_CFG
                | GTX::TXPLL_CP_CFG
                | GTX::TX_BYTECLK_CFG
                | GTX::TX_DETECT_RX_CFG
                | GTX::TX_USRCLK_CFG => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }

                GTX::RX_SLIDE_MODE => {
                    for (val, vname) in [
                        (enums::GTX_RX_SLIDE_MODE::NONE, "#OFF"),
                        (enums::GTX_RX_SLIDE_MODE::AUTO, "AUTO"),
                        (enums::GTX_RX_SLIDE_MODE::PCS, "PCS"),
                        (enums::GTX_RX_SLIDE_MODE::PMA, "PMA"),
                    ] {
                        bctx.mode(mode)
                            .test_bel_attr_val(aid, val)
                            .attr(aname, vname)
                            .commit();
                    }
                }

                GTX::CHAN_BOND_1_MAX_SKEW | GTX::CHAN_BOND_2_MAX_SKEW => {
                    for val in 1..15 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTX::CLK_COR_MAX_LAT | GTX::CLK_COR_MIN_LAT => {
                    for val in 3..49 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTX::SAS_MAX_COMSAS
                | GTX::SAS_MIN_COMSAS
                | GTX::SATA_MAX_BURST
                | GTX::SATA_MAX_INIT
                | GTX::SATA_MAX_WAKE
                | GTX::SATA_MIN_BURST
                | GTX::SATA_MIN_INIT
                | GTX::SATA_MIN_WAKE => {
                    for val in 1..62 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::BitVec(_width) => {
                        bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Bin);
                    }
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode).test_bel_attr_auto(aid);
                    }
                    _ => unreachable!(),
                },
            }
        }
        for val in ["FALSE", "TRUE"] {
            bctx.build()
                .null_bits()
                .mode(mode)
                .test_bel_special(specials::GTX_CFG_PWRUP)
                .attr("GTX_CFG_PWRUP", val)
                .commit();
        }

        for (val, orx, otx, pin) in [
            // ("PERFCLK", "PERFCLKRX", "PERFCLKTX", "PERFCLK"),
            (
                enums::GTX_PLLREFSEL::MGTREFCLK0,
                "MGTREFCLKRX0",
                "MGTREFCLKTX0",
                "MGTREFCLKOUT0",
            ),
            (
                enums::GTX_PLLREFSEL::MGTREFCLK1,
                "MGTREFCLKRX1",
                "MGTREFCLKTX1",
                "MGTREFCLKOUT1",
            ),
            (
                enums::GTX_PLLREFSEL::SOUTHREFCLK0,
                "SOUTHREFCLKRX0",
                "SOUTHREFCLKTX0",
                "SOUTHREFCLKOUT0",
            ),
            (
                enums::GTX_PLLREFSEL::SOUTHREFCLK1,
                "SOUTHREFCLKRX1",
                "SOUTHREFCLKTX1",
                "SOUTHREFCLKOUT1",
            ),
            (
                enums::GTX_PLLREFSEL::NORTHREFCLK0,
                "NORTHREFCLKRX0",
                "NORTHREFCLKTX0",
                "NORTHREFCLKIN0",
            ),
            (
                enums::GTX_PLLREFSEL::NORTHREFCLK1,
                "NORTHREFCLKRX1",
                "NORTHREFCLKTX1",
                "NORTHREFCLKIN1",
            ),
        ] {
            bctx.build()
                .mutex("RXPLLREFSEL", pin)
                .test_bel_attr_val(GTX::RXPLLREFSEL_STATIC_VAL, val)
                .pip(orx, pin)
                .commit();
            bctx.build()
                .mutex("TXPLLREFSEL", pin)
                .test_bel_attr_val(GTX::TXPLLREFSEL_STATIC_VAL, val)
                .pip(otx, pin)
                .commit();
        }
        let wt = wires::IMUX_GTX_PERFCLK.cell(20);
        let wf = wires::PERF_ROW[0].cell(20);
        bctx.build()
            .prop(WireMutexExclusive::new(wt))
            .prop(BaseIntPip::new(wt, wf))
            .mutex("RXPLLREFSEL", "PERFCLK")
            .test_bel_attr_val(
                GTX::RXPLLREFSEL_TESTCLK,
                enums::GTX_PLLREFSEL_TESTCLK::PERFCLK,
            )
            .pip("PERFCLKRX", (PinFar, "PERFCLKRX"))
            .commit();
        bctx.build()
            .prop(WireMutexExclusive::new(wt))
            .prop(BaseIntPip::new(wt, wf))
            .mutex("TXPLLREFSEL", "PERFCLK")
            .test_bel_attr_val(
                GTX::TXPLLREFSEL_TESTCLK,
                enums::GTX_PLLREFSEL_TESTCLK::PERFCLK,
            )
            .pip("PERFCLKTX", (PinFar, "PERFCLKTX"))
            .commit();
        bctx.mode(mode)
            .mutex("RXPLLREFSEL", "CAS_CLK")
            .mutex("TXPLLREFSEL", "CAS_CLK")
            .test_bel_attr_bits(GTX::PMA_CAS_CLK_EN)
            .attr("PMA_CAS_CLK_EN", "TRUE")
            .commit();
        bctx.build()
            .mutex("RXPLLREFSEL", "GREFCLK")
            .test_bel_attr_val(
                GTX::RXPLLREFSEL_TESTCLK,
                enums::GTX_PLLREFSEL_TESTCLK::GREFCLK,
            )
            .pip("GREFCLKRX", (PinFar, "GREFCLKRX"))
            .commit();
        bctx.build()
            .mutex("TXPLLREFSEL", "GREFCLK")
            .test_bel_attr_val(
                GTX::TXPLLREFSEL_TESTCLK,
                enums::GTX_PLLREFSEL_TESTCLK::GREFCLK,
            )
            .pip("GREFCLKTX", (PinFar, "GREFCLKTX"))
            .commit();
        bctx.build()
            .mutex("RXPLLREFSEL", "MODE")
            .pip("GREFCLKRX", (PinFar, "GREFCLKRX"))
            .test_bel_attr_bits(GTX::RXPLLREFSEL_MODE_DYNAMIC)
            .pip("MGTREFCLKRX0", "MGTREFCLKOUT0")
            .commit();
        bctx.build()
            .mutex("TXPLLREFSEL", "MODE")
            .pip("GREFCLKTX", (PinFar, "GREFCLKTX"))
            .test_bel_attr_bits(GTX::TXPLLREFSEL_MODE_DYNAMIC)
            .pip("MGTREFCLKTX0", "MGTREFCLKOUT0")
            .commit();
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::GTCLK[i]);
        let mode = "IBUFDS_GTXE1";
        bctx.mode(mode)
            .test_bel_attr_bool_auto(GTCLK::CLKCM_CFG, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_bool_auto(GTCLK::CLKRCV_TRST, "FALSE", "TRUE");
        bctx.mode(mode)
            .test_bel_attr_multi(GTCLK::REFCLKOUT_DLY, MultiValue::Bin);
        for (val, pin) in [
            (enums::GTCLK_MUX_CLKOUT::O, "O"),
            (enums::GTCLK_MUX_CLKOUT::ODIV2, "ODIV2"),
            (enums::GTCLK_MUX_CLKOUT::CLKTESTSIG, "CLKTESTSIG"),
        ] {
            bctx.mode(mode)
                .mutex("MUX_CLKOUT", pin)
                .test_bel_attr_val(GTCLK::MUX_CLKOUT, val)
                .pip("CLKOUT", (PinFar, pin))
                .commit();
        }
    }
    let mut bctx = ctx.bel(bslots::HCLK_GTX);
    for (i, attr, val_pass, vals_refclkin) in [
        (
            0,
            HCLK_GTX::MUX_SOUTHREFCLKOUT0,
            enums::HCLK_GTX_MUX_SOUTHREFCLKOUT0::SOUTHREFCLKIN0,
            [
                enums::HCLK_GTX_MUX_SOUTHREFCLKOUT0::MGTREFCLKIN0,
                enums::HCLK_GTX_MUX_SOUTHREFCLKOUT0::MGTREFCLKIN1,
            ],
        ),
        (
            1,
            HCLK_GTX::MUX_SOUTHREFCLKOUT1,
            enums::HCLK_GTX_MUX_SOUTHREFCLKOUT1::SOUTHREFCLKIN1,
            [
                enums::HCLK_GTX_MUX_SOUTHREFCLKOUT1::MGTREFCLKIN0,
                enums::HCLK_GTX_MUX_SOUTHREFCLKOUT1::MGTREFCLKIN1,
            ],
        ),
    ] {
        for j in 0..2 {
            bctx.build()
                .mutex(format!("MUX_SOUTHREFCLKOUT{i}"), format!("MGTREFCLKIN{j}"))
                .test_bel_attr_val(attr, vals_refclkin[j])
                .pip(format!("SOUTHREFCLKOUT{i}"), format!("MGTREFCLKIN{j}"))
                .commit();
        }
        bctx.build()
            .mutex(
                format!("MUX_SOUTHREFCLKOUT{i}"),
                format!("SOUTHREFCLKIN{i}"),
            )
            .test_bel_attr_val(attr, val_pass)
            .pip(format!("SOUTHREFCLKOUT{i}"), format!("SOUTHREFCLKIN{i}"))
            .commit();
    }
    for (i, attr, val_pass, vals_refclkin) in [
        (
            0,
            HCLK_GTX::MUX_NORTHREFCLKOUT0,
            enums::HCLK_GTX_MUX_NORTHREFCLKOUT0::NORTHREFCLKIN0,
            [
                enums::HCLK_GTX_MUX_NORTHREFCLKOUT0::MGTREFCLKIN0,
                enums::HCLK_GTX_MUX_NORTHREFCLKOUT0::MGTREFCLKIN1,
            ],
        ),
        (
            1,
            HCLK_GTX::MUX_NORTHREFCLKOUT1,
            enums::HCLK_GTX_MUX_NORTHREFCLKOUT1::NORTHREFCLKIN1,
            [
                enums::HCLK_GTX_MUX_NORTHREFCLKOUT1::MGTREFCLKIN0,
                enums::HCLK_GTX_MUX_NORTHREFCLKOUT1::MGTREFCLKIN1,
            ],
        ),
    ] {
        for j in 0..2 {
            bctx.build()
                .mutex(format!("MUX_NORTHREFCLKOUT{i}"), format!("MGTREFCLKOUT{j}"))
                .test_bel_attr_val(attr, vals_refclkin[j])
                .pip(format!("NORTHREFCLKOUT{i}"), format!("MGTREFCLKOUT{j}"))
                .commit();
        }
        bctx.build()
            .mutex(
                format!("MUX_NORTHREFCLKOUT{i}"),
                format!("NORTHREFCLKIN{i}"),
            )
            .test_bel_attr_val(attr, val_pass)
            .pip(format!("NORTHREFCLKOUT{i}"), format!("NORTHREFCLKIN{i}"))
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::GTX;
    if !ctx.has_tcls(tcid) {
        return;
    }
    for i in 0..4 {
        let bslot = bslots::GTX[i];
        fn drp_bit(which: usize, idx: usize, bit: usize) -> TileBit {
            let tile = which * 10 + (idx >> 3);
            let frame = 28 + (bit & 1);
            let bit = (bit >> 1) | (idx & 7) << 3;
            TileBit::new(tile, frame, bit)
        }
        let mut drp = vec![];
        for addr in 0..0x50 {
            for bit in 0..16 {
                drp.push(drp_bit(i, addr, bit).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, GTX::DRP, drp);

        for &pin in GTX_INVPINS {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        for (aid, _, attr) in &ctx.edev.db[GTX].attributes {
            match aid {
                GTX::DRP
                | GTX::PMA_CAS_CLK_EN
                | GTX::RXPLLREFSEL_STATIC_VAL
                | GTX::RXPLLREFSEL_MODE_DYNAMIC
                | GTX::RXPLLREFSEL_TESTCLK
                | GTX::TXPLLREFSEL_STATIC_VAL
                | GTX::TXPLLREFSEL_MODE_DYNAMIC
                | GTX::TXPLLREFSEL_TESTCLK => (),

                GTX::GTX_CFG_PWRUP => {
                    ctx.collect_bel_attr(tcid, bslot, aid);
                }
                GTX::CHAN_BOND_1_MAX_SKEW | GTX::CHAN_BOND_2_MAX_SKEW => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..15);
                }
                GTX::CLK_COR_MAX_LAT | GTX::CLK_COR_MIN_LAT => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 3..49);
                }
                GTX::SAS_MAX_COMSAS
                | GTX::SAS_MIN_COMSAS
                | GTX::SATA_MAX_BURST
                | GTX::SATA_MAX_INIT
                | GTX::SATA_MAX_WAKE
                | GTX::SATA_MIN_BURST
                | GTX::SATA_MIN_INIT
                | GTX::SATA_MIN_WAKE => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..62);
                }
                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        ctx.collect_bel_attr_bi(tcid, bslot, aid);
                    }
                    BelAttributeType::BitVec(_) => {
                        ctx.collect_bel_attr(tcid, bslot, aid);
                    }
                    BelAttributeType::Enum(_) => {
                        ctx.collect_bel_attr_ocd(tcid, bslot, aid, OcdMode::BitOrderDrpV6);
                    }
                    _ => unreachable!(),
                },
            }
        }

        let mut diff_cas_clk = ctx.get_diff_attr_bool(tcid, bslot, GTX::PMA_CAS_CLK_EN);
        for (attr_static, attr_dynamic, attr_testclk) in [
            (
                GTX::RXPLLREFSEL_STATIC_VAL,
                GTX::RXPLLREFSEL_MODE_DYNAMIC,
                GTX::RXPLLREFSEL_TESTCLK,
            ),
            (
                GTX::TXPLLREFSEL_STATIC_VAL,
                GTX::TXPLLREFSEL_MODE_DYNAMIC,
                GTX::TXPLLREFSEL_TESTCLK,
            ),
        ] {
            let diff_grefclk = ctx.get_diff_attr_val(
                tcid,
                bslot,
                attr_testclk,
                enums::GTX_PLLREFSEL_TESTCLK::GREFCLK,
            );
            let diff_perfclk = ctx
                .get_diff_attr_val(
                    tcid,
                    bslot,
                    attr_testclk,
                    enums::GTX_PLLREFSEL_TESTCLK::PERFCLK,
                )
                .combine(&!&diff_grefclk);
            ctx.insert_bel_attr_enum(
                tcid,
                bslot,
                attr_testclk,
                xlat_enum_attr(vec![
                    (enums::GTX_PLLREFSEL_TESTCLK::GREFCLK, Diff::default()),
                    (enums::GTX_PLLREFSEL_TESTCLK::PERFCLK, diff_perfclk),
                ]),
            );
            let mut diffs = vec![];
            for val in [
                enums::GTX_PLLREFSEL::MGTREFCLK0,
                enums::GTX_PLLREFSEL::MGTREFCLK1,
                enums::GTX_PLLREFSEL::NORTHREFCLK0,
                enums::GTX_PLLREFSEL::NORTHREFCLK1,
                enums::GTX_PLLREFSEL::SOUTHREFCLK0,
                enums::GTX_PLLREFSEL::SOUTHREFCLK1,
            ] {
                diffs.push((val, ctx.get_diff_attr_val(tcid, bslot, attr_static, val)))
            }
            diffs.push((
                enums::GTX_PLLREFSEL::CAS_CLK,
                diff_cas_clk.split_bits(&diff_grefclk.bits.keys().copied().collect()),
            ));
            diffs.push((enums::GTX_PLLREFSEL::TESTCLK, diff_grefclk));
            ctx.insert_bel_attr_enum(tcid, bslot, attr_static, xlat_enum_attr(diffs));
            ctx.collect_bel_attr(tcid, bslot, attr_dynamic);
        }
        ctx.insert_bel_attr_bool(tcid, bslot, GTX::PMA_CAS_CLK_EN, xlat_bit(diff_cas_clk));
    }
    for i in 0..2 {
        let bslot = bslots::GTCLK[i];
        ctx.collect_bel_attr_bi(tcid, bslot, GTCLK::CLKCM_CFG);
        ctx.collect_bel_attr_bi(tcid, bslot, GTCLK::CLKRCV_TRST);
        ctx.collect_bel_attr(tcid, bslot, GTCLK::REFCLKOUT_DLY);
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            GTCLK::MUX_CLKOUT,
            enums::GTCLK_MUX_CLKOUT::NONE,
        );
    }
    let bslot = bslots::HCLK_GTX;
    ctx.collect_bel_attr_default(
        tcid,
        bslot,
        HCLK_GTX::MUX_SOUTHREFCLKOUT0,
        enums::HCLK_GTX_MUX_SOUTHREFCLKOUT0::NONE,
    );
    ctx.collect_bel_attr_default(
        tcid,
        bslot,
        HCLK_GTX::MUX_SOUTHREFCLKOUT1,
        enums::HCLK_GTX_MUX_SOUTHREFCLKOUT1::NONE,
    );
    ctx.collect_bel_attr_default(
        tcid,
        bslot,
        HCLK_GTX::MUX_NORTHREFCLKOUT0,
        enums::HCLK_GTX_MUX_NORTHREFCLKOUT0::NONE,
    );
    ctx.collect_bel_attr_default(
        tcid,
        bslot,
        HCLK_GTX::MUX_NORTHREFCLKOUT1,
        enums::HCLK_GTX_MUX_NORTHREFCLKOUT1::NONE,
    );
    let tcid = tcls::HCLK;
    let bslot = bslots::HCLK_DRP[0];
    ctx.collect_bel_attr(tcid, bslot, HCLK_DRP::DRP_MASK_S);
    ctx.collect_bel_attr(tcid, bslot, HCLK_DRP::DRP_MASK_N);
}
