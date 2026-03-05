use prjcombine_entity::EntityPartVec;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, BelAttributeType, BelInputId, WireSlotIdExt},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{Diff, OcdMode, xlat_bit, xlat_enum_attr};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bsdata::TileBit};
use prjcombine_virtex4::defs::{
    bcls::{self, GTCLK, GTP_CHANNEL, GTP_COMMON, GTX_CHANNEL, GTX_COMMON},
    bslots, enums, tslots,
    virtex7::{tcls, wires},
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{DynProp, bel::BelMutex, mutex::WireMutexExclusive, pip::PinFar, relation::Delta},
    },
    virtex4::specials,
};

const GTP_COMMON_INVPINS: &[BelInputId] = &[
    GTP_COMMON::DRPCLK,
    GTP_COMMON::GTGREFCLK0,
    GTP_COMMON::GTGREFCLK1,
    GTP_COMMON::PLLCLKSPARE,
    GTP_COMMON::PLL0LOCKDETCLK,
    GTP_COMMON::PLL1LOCKDETCLK,
    GTP_COMMON::PMASCANCLK0,
    GTP_COMMON::PMASCANCLK1,
];

const GTX_COMMON_INVPINS: &[BelInputId] = &[
    GTX_COMMON::DRPCLK,
    GTX_COMMON::GTGREFCLK,
    GTX_COMMON::QPLLCLKSPARE.index_const(0),
    GTX_COMMON::QPLLCLKSPARE.index_const(1),
    GTX_COMMON::QPLLLOCKDETCLK,
    GTX_COMMON::PMASCANCLK.index_const(0),
    GTX_COMMON::PMASCANCLK.index_const(1),
];

const GTP_CHANNEL_INVPINS: &[BelInputId] = &[
    GTP_CHANNEL::CLKRSVD.index_const(0),
    GTP_CHANNEL::CLKRSVD.index_const(1),
    GTP_CHANNEL::DMONITORCLK,
    GTP_CHANNEL::DRPCLK,
    GTP_CHANNEL::PMASCANCLK.index_const(0),
    GTP_CHANNEL::PMASCANCLK.index_const(1),
    GTP_CHANNEL::PMASCANCLK.index_const(2),
    GTP_CHANNEL::PMASCANCLK.index_const(3),
    GTP_CHANNEL::RXUSRCLK,
    GTP_CHANNEL::RXUSRCLK2,
    GTP_CHANNEL::SCANCLK,
    GTP_CHANNEL::SIGVALIDCLK,
    GTP_CHANNEL::TSTCLK.index_const(0),
    GTP_CHANNEL::TSTCLK.index_const(1),
    GTP_CHANNEL::TXPHDLYTSTCLK,
    GTP_CHANNEL::TXUSRCLK,
    GTP_CHANNEL::TXUSRCLK2,
];

const GTX_CHANNEL_INVPINS: &[BelInputId] = &[
    GTX_CHANNEL::CPLLLOCKDETCLK,
    GTX_CHANNEL::DRPCLK,
    GTX_CHANNEL::EDTCLOCK,
    GTX_CHANNEL::GTGREFCLK,
    GTX_CHANNEL::PMASCANCLK.index_const(0),
    GTX_CHANNEL::PMASCANCLK.index_const(1),
    GTX_CHANNEL::PMASCANCLK.index_const(2),
    GTX_CHANNEL::PMASCANCLK.index_const(3),
    GTX_CHANNEL::PMASCANCLK.index_const(4),
    GTX_CHANNEL::RXUSRCLK,
    GTX_CHANNEL::RXUSRCLK2,
    GTX_CHANNEL::SCANCLK,
    GTX_CHANNEL::TSTCLK.index_const(0),
    GTX_CHANNEL::TSTCLK.index_const(1),
    GTX_CHANNEL::TXPHDLYTSTCLK,
    GTX_CHANNEL::TXUSRCLK,
    GTX_CHANNEL::TXUSRCLK2,
];

const GTH_CHANNEL_INVPINS: &[BelInputId] = &[
    GTX_CHANNEL::CLKRSVD.index_const(0),
    GTX_CHANNEL::CLKRSVD.index_const(1),
    GTX_CHANNEL::CPLLLOCKDETCLK,
    GTX_CHANNEL::DMONITORCLK,
    GTX_CHANNEL::DRPCLK,
    GTX_CHANNEL::GTGREFCLK,
    GTX_CHANNEL::PMASCANCLK.index_const(0),
    GTX_CHANNEL::PMASCANCLK.index_const(1),
    GTX_CHANNEL::PMASCANCLK.index_const(2),
    GTX_CHANNEL::PMASCANCLK.index_const(3),
    GTX_CHANNEL::PMASCANCLK.index_const(4),
    GTX_CHANNEL::RXUSRCLK,
    GTX_CHANNEL::RXUSRCLK2,
    GTX_CHANNEL::SCANCLK,
    GTX_CHANNEL::SIGVALIDCLK,
    GTX_CHANNEL::TSTCLK.index_const(0),
    GTX_CHANNEL::TSTCLK.index_const(1),
    GTX_CHANNEL::TXPHDLYTSTCLK,
    GTX_CHANNEL::TXUSRCLK,
    GTX_CHANNEL::TXUSRCLK2,
];

#[derive(Clone, Copy, Debug)]
pub struct TouchHout(pub usize);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for TouchHout {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[tcrd.die];
        let idx = self.0;
        let mut tgt_col_cmt = None;
        let mut tgt_col_gt = None;
        if tcrd.col < edev.col_clk {
            if let Some(col_io) = edev.col_io_w
                && tcrd.col < col_io
            {
                tgt_col_cmt = Some(col_io + 1);
            }
            if let Some((col_gt, _)) = edev.col_gt_m {
                let gtcol = chip.get_col_gt(col_gt).unwrap();
                if tcrd.col > col_gt && gtcol.regs[chip.row_to_reg(tcrd.row)].is_some() {
                    tgt_col_gt = Some(col_gt);
                }
            }
        } else {
            if let Some(col_io) = edev.col_io_e
                && tcrd.col > col_io
            {
                tgt_col_cmt = Some(col_io - 1);
            }
            if let Some((_, col_gt)) = edev.col_gt_m {
                let gtcol = chip.get_col_gt(col_gt).unwrap();
                if tcrd.col > col_gt && gtcol.regs[chip.row_to_reg(tcrd.row)].is_some() {
                    tgt_col_gt = Some(col_gt);
                }
            }
        }
        if let Some(_col) = tgt_col_cmt {
            todo!();
        } else if tgt_col_gt.is_some() {
            // nope.
            return None;
        } else {
            let clk_hrow = tcrd.with_col(edev.col_clk).tile(tslots::BEL);
            let hrow_i = if tcrd.col <= edev.col_clk {
                wires::HROW_I_HROW_W[idx].cell(1)
            } else {
                wires::HROW_I_HROW_E[idx].cell(1)
            };
            let imux_bufg = wires::IMUX_BUFG_O[idx].cell(1);

            (fuzzer, _) = BelMutex::new(bslots::SPEC_INT, "CASCO".into(), "CASCO".into())
                .apply(backend, clk_hrow, fuzzer)?;
            (fuzzer, _) = WireMutexExclusive::new(imux_bufg).apply(backend, clk_hrow, fuzzer)?;
            (fuzzer, _) = WireMutexExclusive::new(hrow_i).apply(backend, clk_hrow, fuzzer)?;
            (fuzzer, _) = BaseIntPip::new(imux_bufg, hrow_i).apply(backend, clk_hrow, fuzzer)?;
        }

        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tcid in [tcls::GTP_COMMON, tcls::GTP_COMMON_MID] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::GTP_COMMON);
        let mode = "GTPE2_COMMON";
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for &pin in GTP_COMMON_INVPINS {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }
        for (aid, aname, attr) in &backend.edev.db[GTP_COMMON].attributes {
            match aid {
                GTP_COMMON::DRP
                | GTP_COMMON::CLKSWING_CFG
                | GTP_COMMON::PLL0REFCLKSEL_STATIC_VAL
                | GTP_COMMON::PLL0REFCLKSEL_MODE_DYNAMIC
                | GTP_COMMON::PLL1REFCLKSEL_STATIC_VAL
                | GTP_COMMON::PLL1REFCLKSEL_MODE_DYNAMIC => (),

                GTP_COMMON::EAST_REFCLK0_SEL
                | GTP_COMMON::EAST_REFCLK1_SEL
                | GTP_COMMON::WEST_REFCLK0_SEL
                | GTP_COMMON::WEST_REFCLK1_SEL => {
                    bctx.mode(mode)
                        .test_bel_attr_bits(aid)
                        .multi_attr(aname, MultiValue::Bin, 2);
                }

                GTP_COMMON::BIAS_CFG
                | GTP_COMMON::COMMON_CFG
                | GTP_COMMON::PLL0_CFG
                | GTP_COMMON::PLL1_CFG
                | GTP_COMMON::PLL0_LOCK_CFG
                | GTP_COMMON::PLL1_LOCK_CFG
                | GTP_COMMON::PLL0_INIT_CFG
                | GTP_COMMON::PLL1_INIT_CFG
                | GTP_COMMON::RSVD_ATTR0
                | GTP_COMMON::RSVD_ATTR1 => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode).test_bel_attr_auto(aid);
                    }
                    BelAttributeType::BitVec(_width) => {
                        bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Bin);
                    }
                    _ => unreachable!(),
                },
            }
        }
    }
    for (tcid, mode) in [
        (tcls::GTX_COMMON, "GTXE2_COMMON"),
        (tcls::GTH_COMMON, "GTHE2_COMMON"),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::GTX_COMMON);

        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for &pin in GTX_COMMON_INVPINS {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }

        for (aid, _aname, attr) in &backend.edev.db[GTX_COMMON].attributes {
            match aid {
                GTX_COMMON::DRP
                | GTX_COMMON::CLKSWING_CFG
                | GTX_COMMON::QPLLREFCLKSEL_STATIC_VAL
                | GTX_COMMON::QPLLREFCLKSEL_MODE_DYNAMIC
                | GTX_COMMON::MUX_SOUTHREFCLKOUT0
                | GTX_COMMON::MUX_SOUTHREFCLKOUT1
                | GTX_COMMON::MUX_NORTHREFCLKOUT0
                | GTX_COMMON::MUX_NORTHREFCLKOUT1 => (),

                GTX_COMMON::QPLL_RP_COMP
                | GTX_COMMON::QPLL_VTRL_RESET
                | GTX_COMMON::RCAL_CFG
                | GTX_COMMON::RSVD_ATTR0
                | GTX_COMMON::RSVD_ATTR1
                    if tcid == tcls::GTX_COMMON =>
                {
                    continue;
                }

                GTX_COMMON::BIAS_CFG
                | GTX_COMMON::COMMON_CFG
                | GTX_COMMON::QPLL_CFG
                | GTX_COMMON::QPLL_INIT_CFG
                | GTX_COMMON::QPLL_LOCK_CFG
                | GTX_COMMON::RSVD_ATTR0
                | GTX_COMMON::RSVD_ATTR1 => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode).test_bel_attr_auto(aid);
                    }
                    BelAttributeType::BitVec(_width) => {
                        bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Bin);
                    }
                    _ => unreachable!(),
                },
            }
        }

        for (val, pin) in [
            (enums::GTX_COMMON_PLLREFCLKSEL::GTREFCLK0, "GTREFCLK0"),
            (enums::GTX_COMMON_PLLREFCLKSEL::GTREFCLK1, "GTREFCLK1"),
            (
                enums::GTX_COMMON_PLLREFCLKSEL::GTNORTHREFCLK0,
                "GTNORTHREFCLK0",
            ),
            (
                enums::GTX_COMMON_PLLREFCLKSEL::GTNORTHREFCLK1,
                "GTNORTHREFCLK1",
            ),
            (
                enums::GTX_COMMON_PLLREFCLKSEL::GTSOUTHREFCLK0,
                "GTSOUTHREFCLK0",
            ),
            (
                enums::GTX_COMMON_PLLREFCLKSEL::GTSOUTHREFCLK1,
                "GTSOUTHREFCLK1",
            ),
            (enums::GTX_COMMON_PLLREFCLKSEL::GTGREFCLK, "GTGREFCLK"),
        ] {
            bctx.build()
                .mutex("QPLLREFCLKSEL_STATIC", pin)
                .test_bel_attr_val(GTX_COMMON::QPLLREFCLKSEL_STATIC_VAL, val)
                .pip(pin, (PinFar, pin))
                .commit();
        }
        bctx.build()
            .mutex("QPLLREFCLKSEL_STATIC", "MODE")
            .pip("GTGREFCLK", (PinFar, "GTGREFCLK"))
            .test_bel_attr_bits(GTX_COMMON::QPLLREFCLKSEL_MODE_DYNAMIC)
            .pip("GTREFCLK0", (PinFar, "GTREFCLK0"))
            .commit();
        if backend.edev.tile_index[tcid].len() > 1 {
            for (i, attr, val_pass, vals_refclkin) in [
                (
                    0,
                    GTX_COMMON::MUX_SOUTHREFCLKOUT0,
                    enums::HCLK_GTX_MUX_SOUTHREFCLKOUT0::SOUTHREFCLKIN0,
                    [
                        enums::HCLK_GTX_MUX_SOUTHREFCLKOUT0::MGTREFCLKIN0,
                        enums::HCLK_GTX_MUX_SOUTHREFCLKOUT0::MGTREFCLKIN1,
                    ],
                ),
                (
                    1,
                    GTX_COMMON::MUX_SOUTHREFCLKOUT1,
                    enums::HCLK_GTX_MUX_SOUTHREFCLKOUT1::SOUTHREFCLKIN1,
                    [
                        enums::HCLK_GTX_MUX_SOUTHREFCLKOUT1::MGTREFCLKIN0,
                        enums::HCLK_GTX_MUX_SOUTHREFCLKOUT1::MGTREFCLKIN1,
                    ],
                ),
            ] {
                bctx.build()
                    .mutex(format!("MUX_NORTHREFCLK{i}_N"), format!("NORTHREFCLK{i}"))
                    .has_related(Delta::new(0, 50, tcid))
                    .test_bel_attr_val(attr, val_pass)
                    .pip(
                        format!("BRKH_N_NORTHREFCLK{i}_N"),
                        format!("BRKH_N_NORTHREFCLK{i}"),
                    )
                    .commit();
                for j in 0..2 {
                    bctx.build()
                        .mutex(format!("MUX_NORTHREFCLK{i}_N"), format!("REFCLK{j}"))
                        .has_related(Delta::new(0, 50, tcid))
                        .test_bel_attr_val(attr, vals_refclkin[j])
                        .pip(
                            format!("BRKH_N_NORTHREFCLK{i}_N"),
                            format!("BRKH_N_REFCLK{j}"),
                        )
                        .commit();
                }
            }
            for (i, attr, val_pass, vals_refclkin) in [
                (
                    0,
                    GTX_COMMON::MUX_NORTHREFCLKOUT0,
                    enums::HCLK_GTX_MUX_NORTHREFCLKOUT0::NORTHREFCLKIN0,
                    [
                        enums::HCLK_GTX_MUX_NORTHREFCLKOUT0::MGTREFCLKIN0,
                        enums::HCLK_GTX_MUX_NORTHREFCLKOUT0::MGTREFCLKIN1,
                    ],
                ),
                (
                    1,
                    GTX_COMMON::MUX_NORTHREFCLKOUT1,
                    enums::HCLK_GTX_MUX_NORTHREFCLKOUT1::NORTHREFCLKIN1,
                    [
                        enums::HCLK_GTX_MUX_NORTHREFCLKOUT1::MGTREFCLKIN0,
                        enums::HCLK_GTX_MUX_NORTHREFCLKOUT1::MGTREFCLKIN1,
                    ],
                ),
            ] {
                bctx.build()
                    .mutex(format!("MUX_SOUTHREFCLK{i}_S"), format!("SOUTHREFCLK{i}"))
                    .has_related(Delta::new(0, -50, tcid))
                    .test_bel_attr_val(attr, val_pass)
                    .pip(
                        format!("BRKH_S_SOUTHREFCLK{i}_S"),
                        format!("BRKH_S_SOUTHREFCLK{i}"),
                    )
                    .commit();
                for j in 0..2 {
                    bctx.build()
                        .mutex(format!("MUX_SOUTHREFCLK{i}_S"), format!("REFCLK{j}"))
                        .has_related(Delta::new(0, -50, tcid))
                        .test_bel_attr_val(attr, vals_refclkin[j])
                        .pip(
                            format!("BRKH_S_SOUTHREFCLK{i}_S"),
                            format!("BRKH_S_REFCLK{j}"),
                        )
                        .commit();
                }
            }
        }
    }
    for tcid in [
        tcls::GTP_COMMON,
        tcls::GTP_COMMON_MID,
        tcls::GTX_COMMON,
        tcls::GTH_COMMON,
    ] {
        for i in 0..2 {
            let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
                continue;
            };
            let mut bctx = ctx.bel(bslots::GTCLK[i]);
            let mode = "IBUFDS_GTE2";
            bctx.build()
                .test_bel_special(specials::PRESENT)
                .mode(mode)
                .commit();
            bctx.mode(mode).test_bel_input_inv_auto(GTCLK::CLKTESTSIG);
            bctx.mode(mode)
                .test_bel_attr_bool_auto(GTCLK::CLKCM_CFG, "FALSE", "TRUE");
            bctx.mode(mode)
                .test_bel_attr_bool_auto(GTCLK::CLKRCV_TRST, "FALSE", "TRUE");
            match tcid {
                tcls::GTP_COMMON | tcls::GTP_COMMON_MID => {
                    bctx.mode(mode)
                        .tile_mutex("CLKSWING_CFG", format!("IBUFDS{i}"))
                        .test_bel(bslots::GTP_COMMON)
                        .test_bel_attr_multi(GTP_COMMON::CLKSWING_CFG, MultiValue::Bin);
                }
                tcls::GTX_COMMON | tcls::GTH_COMMON => {
                    bctx.mode(mode)
                        .tile_mutex("CLKSWING_CFG", format!("IBUFDS{i}"))
                        .test_bel(bslots::GTX_COMMON)
                        .test_bel_attr_multi(GTX_COMMON::CLKSWING_CFG, MultiValue::Bin);
                }
                _ => unreachable!(),
            }
            for (val, pin) in [
                (enums::GTCLK_MUX_CLKOUT::O, "O"),
                (enums::GTCLK_MUX_CLKOUT::ODIV2, "ODIV2"),
            ] {
                bctx.mode(mode)
                    .mutex("MUX.MGTCLKOUT", pin)
                    .test_bel_attr_val(GTCLK::MUX_CLKOUT, val)
                    .pip("CLKOUT", pin)
                    .commit();
            }
            bctx.mode(mode)
                .mutex("MUX.MGTCLKOUT", "CLKTESTSIG")
                .test_bel_attr_val(GTCLK::MUX_CLKOUT, enums::GTCLK_MUX_CLKOUT::CLKTESTSIG)
                .pin_pips("CLKTESTSIG")
                .commit();
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::GTP_COMMON_MID) {
        for o in 0..14 {
            let dst = wires::HROW_O[o].cell(3);
            let odst = wires::HROW_O[o ^ 1].cell(3);
            for i in [o, o ^ 1] {
                let src = wires::HROW_I_GTP[i].cell(3);
                let fsrc = wires::HROW_I[i].cell(3);
                ctx.build()
                    .tile_mutex("HIN", "USE")
                    .prop(TouchHout(o))
                    .prop(TouchHout(o ^ 1))
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(odst))
                    .prop(BaseIntPip::new(odst, src))
                    .test_routing(dst, src.pos())
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
                if o == i {
                    ctx.build()
                        .tile_mutex("HIN", "TEST")
                        .prop(TouchHout(o))
                        .prop(WireMutexExclusive::new(dst))
                        .test_routing(dst, fsrc.pos())
                        .prop(FuzzIntPip::new(dst, src))
                        .commit();
                }
            }
            for (i, pin) in wires::OUT_GT_MGTCLKOUT_HCLK
                .into_iter()
                .chain(wires::OUT_GT_RXOUTCLK_HCLK)
                .chain(wires::OUT_GT_TXOUTCLK_HCLK)
                .enumerate()
            {
                let src = pin.cell(3);
                let fsrc = backend.edev.db_index[tcls::GTP_COMMON_MID].only_bwd(src);
                ctx.build()
                    .tile_mutex("HIN", "USE")
                    .prop(TouchHout(o))
                    .prop(TouchHout(o ^ 1))
                    .prop(WireMutexExclusive::new(dst))
                    .prop(WireMutexExclusive::new(odst))
                    .prop(BaseIntPip::new(odst, src))
                    .test_routing(dst, src.pos())
                    .prop(FuzzIntPip::new(dst, src))
                    .commit();
                if o == i {
                    ctx.build()
                        .tile_mutex("HIN", "TEST")
                        .prop(TouchHout(o))
                        .prop(WireMutexExclusive::new(dst))
                        .test_routing(dst, fsrc)
                        .prop(FuzzIntPip::new(dst, src))
                        .commit();
                }
            }
        }
    }

    for tcid in [tcls::GTP_CHANNEL, tcls::GTP_CHANNEL_MID] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::GTP_CHANNEL);
        let mode = "GTPE2_CHANNEL";
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for &pin in GTP_CHANNEL_INVPINS {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }

        for (aid, aname, attr) in &backend.edev.db[GTP_CHANNEL].attributes {
            match aid {
                GTP_CHANNEL::DRP => (),

                GTP_CHANNEL::RXSLIDE_MODE => {
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

                GTP_CHANNEL::CLK_COR_REPEAT_WAIT
                | GTP_CHANNEL::RXBUF_THRESH_OVFLW
                | GTP_CHANNEL::RXBUF_THRESH_UNDFLW
                | GTP_CHANNEL::RXSLIDE_AUTO_WAIT => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                }

                GTP_CHANNEL::AMONITOR_CFG
                | GTP_CHANNEL::DMONITOR_CFG
                | GTP_CHANNEL::ES_HORZ_OFFSET
                | GTP_CHANNEL::ES_QUALIFIER
                | GTP_CHANNEL::ES_QUAL_MASK
                | GTP_CHANNEL::ES_SDATA_MASK
                | GTP_CHANNEL::PCS_RSVD_ATTR
                | GTP_CHANNEL::PD_TRANS_TIME_FROM_P2
                | GTP_CHANNEL::PD_TRANS_TIME_NONE_P2
                | GTP_CHANNEL::PD_TRANS_TIME_TO_P2
                | GTP_CHANNEL::PMA_RSV
                | GTP_CHANNEL::PMA_RSV2
                | GTP_CHANNEL::RXCDR_CFG
                | GTP_CHANNEL::RXDLY_CFG
                | GTP_CHANNEL::RXDLY_LCFG
                | GTP_CHANNEL::RXDLY_TAP_CFG
                | GTP_CHANNEL::RXPHDLY_CFG
                | GTP_CHANNEL::RXPH_CFG
                | GTP_CHANNEL::TRANS_TIME_RATE
                | GTP_CHANNEL::TST_RSV
                | GTP_CHANNEL::TXDLY_CFG
                | GTP_CHANNEL::TXDLY_LCFG
                | GTP_CHANNEL::TXDLY_TAP_CFG
                | GTP_CHANNEL::TXPHDLY_CFG
                | GTP_CHANNEL::TXPH_CFG
                | GTP_CHANNEL::TX_RXDETECT_CFG => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }

                GTP_CHANNEL::TX_CLK25_DIV
                | GTP_CHANNEL::RX_CLK25_DIV
                | GTP_CHANNEL::RX_SIG_VALID_DLY => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(1));
                }

                GTP_CHANNEL::CHAN_BOND_MAX_SKEW => {
                    for val in 1..15 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTP_CHANNEL::CLK_COR_MAX_LAT | GTP_CHANNEL::CLK_COR_MIN_LAT => {
                    for val in 3..61 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTP_CHANNEL::SAS_MAX_COM => {
                    for val in 1..128 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTP_CHANNEL::SAS_MIN_COM
                | GTP_CHANNEL::SATA_MAX_BURST
                | GTP_CHANNEL::SATA_MAX_INIT
                | GTP_CHANNEL::SATA_MAX_WAKE
                | GTP_CHANNEL::SATA_MIN_INIT
                | GTP_CHANNEL::SATA_MIN_WAKE => {
                    for val in 1..64 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTP_CHANNEL::SATA_MIN_BURST => {
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
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode).test_bel_attr_auto(aid);
                    }
                    BelAttributeType::BitVec(_width) => {
                        bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Bin);
                    }
                    _ => unreachable!(),
                },
            }
        }
    }
    for (tcid, mode, invpins) in [
        (tcls::GTX_CHANNEL, "GTXE2_CHANNEL", GTX_CHANNEL_INVPINS),
        (tcls::GTH_CHANNEL, "GTHE2_CHANNEL", GTH_CHANNEL_INVPINS),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::GTX_CHANNEL);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        for &pin in invpins {
            bctx.mode(mode).test_bel_input_inv_auto(pin);
        }

        for (aid, aname, attr) in &backend.edev.db[GTX_CHANNEL].attributes {
            match aid {
                GTX_CHANNEL::DRP
                | GTX_CHANNEL::CPLLREFCLKSEL_STATIC_VAL
                | GTX_CHANNEL::CPLLREFCLKSEL_MODE_DYNAMIC => (),

                GTX_CHANNEL::RXSLIDE_MODE => {
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

                GTX_CHANNEL::CLK_COR_REPEAT_WAIT
                | GTX_CHANNEL::RXBUF_THRESH_OVFLW
                | GTX_CHANNEL::RXBUF_THRESH_UNDFLW
                | GTX_CHANNEL::RXSLIDE_AUTO_WAIT
                | GTX_CHANNEL::RX_INT_DATAWIDTH
                | GTX_CHANNEL::TXOUTCLKPCS_SEL
                | GTX_CHANNEL::TX_INT_DATAWIDTH => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                }

                GTX_CHANNEL::TX_CLK25_DIV
                | GTX_CHANNEL::RX_CLK25_DIV
                | GTX_CHANNEL::RX_SIG_VALID_DLY => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(1));
                }

                GTX_CHANNEL::CHAN_BOND_MAX_SKEW => {
                    for val in 1..15 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTX_CHANNEL::CLK_COR_MAX_LAT | GTX_CHANNEL::CLK_COR_MIN_LAT => {
                    for val in 3..61 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTX_CHANNEL::SAS_MAX_COM => {
                    for val in 1..128 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTX_CHANNEL::SAS_MIN_COM
                | GTX_CHANNEL::SATA_MAX_BURST
                | GTX_CHANNEL::SATA_MAX_INIT
                | GTX_CHANNEL::SATA_MAX_WAKE
                | GTX_CHANNEL::SATA_MIN_INIT
                | GTX_CHANNEL::SATA_MIN_WAKE => {
                    for val in 1..64 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                GTX_CHANNEL::SATA_MIN_BURST => {
                    for val in 1..62 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }

                GTX_CHANNEL::TXPI_PPMCLK_SEL
                | GTX_CHANNEL::RXOOB_CLK_CFG
                | GTX_CHANNEL::ACJTAG_DEBUG_MODE
                | GTX_CHANNEL::ACJTAG_MODE
                | GTX_CHANNEL::ACJTAG_RESET
                | GTX_CHANNEL::AEN_TXPI_PPM
                | GTX_CHANNEL::A_RXADAPTSELTEST
                | GTX_CHANNEL::A_RXDFETAP6HOLD
                | GTX_CHANNEL::A_RXDFETAP6OVRDEN
                | GTX_CHANNEL::A_RXDFETAP7HOLD
                | GTX_CHANNEL::A_RXDFETAP7OVRDEN
                | GTX_CHANNEL::A_RXDFEAGCTRL
                | GTX_CHANNEL::A_RXDFESLIDETAP
                | GTX_CHANNEL::A_RXDFESLIDETAPADAPTEN
                | GTX_CHANNEL::A_RXDFESLIDETAPHOLD
                | GTX_CHANNEL::A_RXDFESLIDETAPID
                | GTX_CHANNEL::A_RXDFESLIDETAPINITOVRDEN
                | GTX_CHANNEL::A_RXDFESLIDETAPONLYADAPTEN
                | GTX_CHANNEL::A_RXDFESLIDETAPOVRDEN
                | GTX_CHANNEL::A_RXDFESLIDETAPSTROBE
                | GTX_CHANNEL::A_RXOSCALRESET
                | GTX_CHANNEL::A_RXOSINTCFG
                | GTX_CHANNEL::A_RXOSINTEN
                | GTX_CHANNEL::A_RXOSINTHOLD
                | GTX_CHANNEL::A_RXOSINTID0
                | GTX_CHANNEL::A_RXOSINTNTRLEN
                | GTX_CHANNEL::A_RXOSINTOVRDEN
                | GTX_CHANNEL::A_RXOSINTSTROBE
                | GTX_CHANNEL::A_RXOSINTTESTOVRDEN
                | GTX_CHANNEL::A_TXPIPPMOVRDEN
                | GTX_CHANNEL::A_TXPIPPMPD
                | GTX_CHANNEL::A_TXPIPPMSEL
                | GTX_CHANNEL::A_TXQPIBIASEN
                | GTX_CHANNEL::CFOK_CFG2
                | GTX_CHANNEL::CFOK_CFG3
                | GTX_CHANNEL::ES_CLK_PHASE_SEL
                | GTX_CHANNEL::LOOPBACK_CFG
                | GTX_CHANNEL::PMA_RSV5
                | GTX_CHANNEL::RESET_POWERSAVE_DISABLE
                | GTX_CHANNEL::RXOSCALRESET_TIME
                | GTX_CHANNEL::RXOSCALRESET_TIMEOUT
                | GTX_CHANNEL::RXPI_CFG0
                | GTX_CHANNEL::RXPI_CFG1
                | GTX_CHANNEL::RXPI_CFG2
                | GTX_CHANNEL::RXPI_CFG3
                | GTX_CHANNEL::RXPI_CFG4
                | GTX_CHANNEL::RXPI_CFG5
                | GTX_CHANNEL::RXPI_CFG6
                | GTX_CHANNEL::RXSYNC_MULTILANE
                | GTX_CHANNEL::RXSYNC_OVRD
                | GTX_CHANNEL::RXSYNC_SKIP_DA
                | GTX_CHANNEL::RX_DFE_H6_CFG
                | GTX_CHANNEL::RX_DFE_H7_CFG
                | GTX_CHANNEL::RX_DFELPM_CFG0
                | GTX_CHANNEL::RX_DFELPM_CFG1
                | GTX_CHANNEL::RX_DFELPM_KLKH_AGC_STUP_EN
                | GTX_CHANNEL::RX_DFE_AGC_CFG0
                | GTX_CHANNEL::RX_DFE_AGC_CFG1
                | GTX_CHANNEL::RX_DFE_AGC_CFG2
                | GTX_CHANNEL::RX_DFE_AGC_OVRDEN
                | GTX_CHANNEL::RX_DFE_KL_LPM_KH_CFG0
                | GTX_CHANNEL::RX_DFE_KL_LPM_KH_CFG1
                | GTX_CHANNEL::RX_DFE_KL_LPM_KH_CFG2
                | GTX_CHANNEL::RX_DFE_KL_LPM_KH_OVRDEN
                | GTX_CHANNEL::RX_DFE_KL_LPM_KL_CFG0
                | GTX_CHANNEL::RX_DFE_KL_LPM_KL_CFG1
                | GTX_CHANNEL::RX_DFE_KL_LPM_KL_CFG2
                | GTX_CHANNEL::RX_DFE_KL_LPM_KL_OVRDEN
                | GTX_CHANNEL::TXOOB_CFG
                | GTX_CHANNEL::TXPI_CFG0
                | GTX_CHANNEL::TXPI_CFG1
                | GTX_CHANNEL::TXPI_CFG2
                | GTX_CHANNEL::TXPI_CFG3
                | GTX_CHANNEL::TXPI_CFG4
                | GTX_CHANNEL::TXPI_CFG5
                | GTX_CHANNEL::TXPI_GREY_SEL
                | GTX_CHANNEL::TXPI_INVSTROBE_SEL
                | GTX_CHANNEL::TXPI_PPM_CFG
                | GTX_CHANNEL::TXPI_SYNFREQ_PPM
                | GTX_CHANNEL::TXSYNC_MULTILANE
                | GTX_CHANNEL::TXSYNC_OVRD
                | GTX_CHANNEL::TXSYNC_SKIP_DA
                | GTX_CHANNEL::USE_PCS_CLK_PHASE_SEL
                | GTX_CHANNEL::RXLPM_LF_CFG_GTH
                | GTX_CHANNEL::RX_BIAS_CFG_GTH
                | GTX_CHANNEL::RX_CM_TRIM_GTH
                | GTX_CHANNEL::RX_DEBUG_CFG_GTH
                | GTX_CHANNEL::RX_DFE_KL_CFG_GTH
                | GTX_CHANNEL::TERM_RCAL_CFG_GTH
                | GTX_CHANNEL::TERM_RCAL_OVRD_GTH
                | GTX_CHANNEL::TX_DEEMPH0_GTH
                | GTX_CHANNEL::TX_DEEMPH1_GTH
                | GTX_CHANNEL::CPLL_CFG_GTH
                | GTX_CHANNEL::RXCDR_CFG_GTH
                | GTX_CHANNEL::PMA_RSV2_GTH
                | GTX_CHANNEL::PMA_RSV4_GTH
                | GTX_CHANNEL::ADAPT_CFG0
                | GTX_CHANNEL::CFOK_CFG
                | GTX_CHANNEL::RX_DFE_ST_CFG
                | GTX_CHANNEL::TX_RXDETECT_PRECHARGE_TIME
                    if tcid == tcls::GTX_CHANNEL => {}

                GTX_CHANNEL::A_RXDFEXYDHOLD
                | GTX_CHANNEL::A_RXDFEXYDOVRDEN
                | GTX_CHANNEL::CPLL_PCD_1UI_CFG
                | GTX_CHANNEL::RX_DFE_XYD_CFG
                | GTX_CHANNEL::TX_PREDRIVER_MODE
                | GTX_CHANNEL::RXLPM_LF_CFG_GTX
                | GTX_CHANNEL::RX_BIAS_CFG_GTX
                | GTX_CHANNEL::RX_CM_TRIM_GTX
                | GTX_CHANNEL::RX_DEBUG_CFG_GTX
                | GTX_CHANNEL::RX_DFE_KL_CFG_GTX
                | GTX_CHANNEL::TERM_RCAL_CFG_GTX
                | GTX_CHANNEL::TERM_RCAL_OVRD_GTX
                | GTX_CHANNEL::TX_DEEMPH0_GTX
                | GTX_CHANNEL::TX_DEEMPH1_GTX
                | GTX_CHANNEL::CPLL_CFG_GTX
                | GTX_CHANNEL::RXCDR_CFG_GTX
                | GTX_CHANNEL::PMA_RSV2_GTX
                | GTX_CHANNEL::PMA_RSV4_GTX
                | GTX_CHANNEL::RX_DFE_KL_CFG2
                    if tcid == tcls::GTH_CHANNEL => {}

                GTX_CHANNEL::AMONITOR_CFG
                | GTX_CHANNEL::CPLL_INIT_CFG
                | GTX_CHANNEL::CPLL_LOCK_CFG
                | GTX_CHANNEL::DMONITOR_CFG
                | GTX_CHANNEL::ES_HORZ_OFFSET
                | GTX_CHANNEL::ES_QUALIFIER
                | GTX_CHANNEL::ES_QUAL_MASK
                | GTX_CHANNEL::ES_SDATA_MASK
                | GTX_CHANNEL::PCS_RSVD_ATTR
                | GTX_CHANNEL::PD_TRANS_TIME_FROM_P2
                | GTX_CHANNEL::PD_TRANS_TIME_NONE_P2
                | GTX_CHANNEL::PD_TRANS_TIME_TO_P2
                | GTX_CHANNEL::RXDLY_CFG
                | GTX_CHANNEL::RXDLY_LCFG
                | GTX_CHANNEL::RXDLY_TAP_CFG
                | GTX_CHANNEL::RXPHDLY_CFG
                | GTX_CHANNEL::RXPH_CFG
                | GTX_CHANNEL::RX_DFE_GAIN_CFG
                | GTX_CHANNEL::RX_DFE_LPM_CFG
                | GTX_CHANNEL::TRANS_TIME_RATE
                | GTX_CHANNEL::TST_RSV
                | GTX_CHANNEL::TXDLY_CFG
                | GTX_CHANNEL::TXDLY_LCFG
                | GTX_CHANNEL::TXDLY_TAP_CFG
                | GTX_CHANNEL::TXPHDLY_CFG
                | GTX_CHANNEL::TXPH_CFG
                | GTX_CHANNEL::TX_RXDETECT_CFG
                | GTX_CHANNEL::RX_DFE_KL_CFG2
                | GTX_CHANNEL::ADAPT_CFG0
                | GTX_CHANNEL::CFOK_CFG
                | GTX_CHANNEL::RX_DFE_ST_CFG
                | GTX_CHANNEL::TX_RXDETECT_PRECHARGE_TIME => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }

                GTX_CHANNEL::PMA_RSV if tcid == tcls::GTX_CHANNEL => {
                    // hex on GTX only.
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }

                GTX_CHANNEL::RXLPM_LF_CFG_GTX
                | GTX_CHANNEL::RX_BIAS_CFG_GTX
                | GTX_CHANNEL::RX_CM_TRIM_GTX
                | GTX_CHANNEL::RX_DEBUG_CFG_GTX
                | GTX_CHANNEL::RX_DFE_KL_CFG_GTX
                | GTX_CHANNEL::TERM_RCAL_CFG_GTX
                | GTX_CHANNEL::TERM_RCAL_OVRD_GTX
                | GTX_CHANNEL::TX_DEEMPH0_GTX
                | GTX_CHANNEL::TX_DEEMPH1_GTX => {
                    let aname = aname.strip_suffix("_GTX").unwrap();
                    bctx.mode(mode)
                        .test_bel_attr_multi_rename(aname, aid, MultiValue::Bin);
                }

                GTX_CHANNEL::RXLPM_LF_CFG_GTH
                | GTX_CHANNEL::RX_BIAS_CFG_GTH
                | GTX_CHANNEL::RX_CM_TRIM_GTH
                | GTX_CHANNEL::RX_DEBUG_CFG_GTH
                | GTX_CHANNEL::RX_DFE_KL_CFG_GTH
                | GTX_CHANNEL::TERM_RCAL_CFG_GTH
                | GTX_CHANNEL::TERM_RCAL_OVRD_GTH
                | GTX_CHANNEL::TX_DEEMPH0_GTH
                | GTX_CHANNEL::TX_DEEMPH1_GTH
                | GTX_CHANNEL::PMA_RSV2_GTH
                | GTX_CHANNEL::PMA_RSV4_GTH => {
                    let aname = aname.strip_suffix("_GTH").unwrap();
                    bctx.mode(mode)
                        .test_bel_attr_multi_rename(aname, aid, MultiValue::Bin);
                }

                GTX_CHANNEL::CPLL_CFG_GTX
                | GTX_CHANNEL::RXCDR_CFG_GTX
                | GTX_CHANNEL::PMA_RSV2_GTX
                | GTX_CHANNEL::PMA_RSV4_GTX => {
                    let aname = aname.strip_suffix("_GTX").unwrap();
                    bctx.mode(mode)
                        .test_bel_attr_multi_rename(aname, aid, MultiValue::Hex(0));
                }

                GTX_CHANNEL::CPLL_CFG_GTH | GTX_CHANNEL::RXCDR_CFG_GTH => {
                    let aname = aname.strip_suffix("_GTH").unwrap();
                    bctx.mode(mode)
                        .test_bel_attr_multi_rename(aname, aid, MultiValue::Hex(0));
                }

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::Enum(_) => {
                        bctx.mode(mode).test_bel_attr_auto(aid);
                    }
                    BelAttributeType::BitVec(_width) => {
                        bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Bin);
                    }
                    _ => unreachable!(),
                },
            }
        }
    }
    for tcid in [tcls::GTX_CHANNEL, tcls::GTH_CHANNEL] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::GTX_CHANNEL);
        for (val, pin) in [
            (enums::GTX_COMMON_PLLREFCLKSEL::GTREFCLK0, "GTREFCLK0"),
            (enums::GTX_COMMON_PLLREFCLKSEL::GTREFCLK1, "GTREFCLK1"),
            (
                enums::GTX_COMMON_PLLREFCLKSEL::GTNORTHREFCLK0,
                "GTNORTHREFCLK0",
            ),
            (
                enums::GTX_COMMON_PLLREFCLKSEL::GTNORTHREFCLK1,
                "GTNORTHREFCLK1",
            ),
            (
                enums::GTX_COMMON_PLLREFCLKSEL::GTSOUTHREFCLK0,
                "GTSOUTHREFCLK0",
            ),
            (
                enums::GTX_COMMON_PLLREFCLKSEL::GTSOUTHREFCLK1,
                "GTSOUTHREFCLK1",
            ),
            (enums::GTX_COMMON_PLLREFCLKSEL::GTGREFCLK, "GTGREFCLK"),
        ] {
            bctx.build()
                .mutex("CPLLREFCLKSEL_STATIC", pin)
                .test_bel_attr_val(GTX_CHANNEL::CPLLREFCLKSEL_STATIC_VAL, val)
                .pip(pin, (PinFar, pin))
                .commit();
        }
        bctx.build()
            .mutex("CPLLREFCLKSEL_STATIC", "MODE")
            .pip("GTGREFCLK", (PinFar, "GTGREFCLK"))
            .test_bel_attr_bits(GTX_CHANNEL::CPLLREFCLKSEL_MODE_DYNAMIC)
            .pip("GTREFCLK0", (PinFar, "GTREFCLK0"))
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    fn common_drp_bit(is_mid: bool, reg: usize, bit: usize) -> TileBit {
        if reg < 0x30 {
            TileBit::new(
                reg >> 3,
                if is_mid { 0 } else { 28 } + (bit & 1),
                (reg & 7) << 3 | bit >> 1,
            )
        } else {
            TileBit::new(
                (reg - 0x30) >> 3,
                if is_mid { 2 } else { 30 } + (bit & 1),
                (reg & 7) << 3 | bit >> 1,
            )
        }
    }
    fn channel_drp_bit(is_mid: bool, reg: usize, bit: usize) -> TileBit {
        if reg < 0x58 {
            TileBit::new(
                reg >> 3,
                if is_mid { 0 } else { 28 } + (bit & 1),
                (reg & 7) << 3 | bit >> 1,
            )
        } else {
            TileBit::new(
                (reg - 0x58) >> 3,
                if is_mid { 2 } else { 30 } + (bit & 1),
                (reg & 7) << 3 | bit >> 1,
            )
        }
    }

    for tcid in [tcls::GTP_COMMON, tcls::GTP_COMMON_MID] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::GTP_COMMON;
        for &pin in GTP_COMMON_INVPINS {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        let mut drp = vec![];
        for reg in 0..0x60 {
            for bit in 0..16 {
                drp.push(common_drp_bit(tcid == tcls::GTP_COMMON_MID, reg, bit).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, GTP_COMMON::DRP, drp);

        for (aid, _aname, attr) in &ctx.edev.db[GTP_COMMON].attributes {
            match aid {
                GTP_COMMON::DRP
                | GTP_COMMON::PLL0REFCLKSEL_STATIC_VAL
                | GTP_COMMON::PLL0REFCLKSEL_MODE_DYNAMIC
                | GTP_COMMON::PLL1REFCLKSEL_STATIC_VAL
                | GTP_COMMON::PLL1REFCLKSEL_MODE_DYNAMIC => (),

                GTP_COMMON::EAST_REFCLK0_SEL
                | GTP_COMMON::EAST_REFCLK1_SEL
                | GTP_COMMON::WEST_REFCLK0_SEL
                | GTP_COMMON::WEST_REFCLK1_SEL => {
                    let [diff0, diff1] = ctx
                        .get_diffs_attr_bits(tcid, bslot, aid, 2)
                        .try_into()
                        .unwrap();
                    ctx.insert_bel_attr_enum(
                        tcid,
                        bslot,
                        aid,
                        xlat_enum_attr(vec![
                            (enums::GTP_COMMON_WE_REFCLK_SEL::NONE, Diff::default()),
                            (enums::GTP_COMMON_WE_REFCLK_SEL::REFCLK0, diff0),
                            (enums::GTP_COMMON_WE_REFCLK_SEL::REFCLK1, diff1),
                        ]),
                    );
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

        // too annoying to bother fuzzing cleanly, given that east/west clocks are only present on 7a200t
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            GTP_COMMON::PLL0REFCLKSEL_STATIC_VAL,
            BelAttributeEnum {
                bits: vec![
                    common_drp_bit(tcid == tcls::GTP_COMMON_MID, 0x03, 28),
                    common_drp_bit(tcid == tcls::GTP_COMMON_MID, 0x03, 29),
                    common_drp_bit(tcid == tcls::GTP_COMMON_MID, 0x03, 30),
                ],
                values: EntityPartVec::from_iter([
                    (enums::GTP_COMMON_PLLREFCLKSEL::NONE, bits![0, 0, 0]),
                    (enums::GTP_COMMON_PLLREFCLKSEL::GTREFCLK0, bits![1, 0, 0]),
                    (enums::GTP_COMMON_PLLREFCLKSEL::GTREFCLK1, bits![0, 1, 0]),
                    (
                        enums::GTP_COMMON_PLLREFCLKSEL::GTEASTREFCLK0,
                        bits![1, 1, 0],
                    ),
                    (
                        enums::GTP_COMMON_PLLREFCLKSEL::GTEASTREFCLK1,
                        bits![0, 0, 1],
                    ),
                    (
                        enums::GTP_COMMON_PLLREFCLKSEL::GTWESTREFCLK0,
                        bits![1, 0, 1],
                    ),
                    (
                        enums::GTP_COMMON_PLLREFCLKSEL::GTWESTREFCLK1,
                        bits![0, 1, 1],
                    ),
                    (enums::GTP_COMMON_PLLREFCLKSEL::GTGREFCLK, bits![1, 1, 1]),
                ]),
            },
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            GTP_COMMON::PLL0REFCLKSEL_MODE_DYNAMIC,
            common_drp_bit(tcid == tcls::GTP_COMMON_MID, 0x03, 31).pos(),
        );
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            GTP_COMMON::PLL1REFCLKSEL_STATIC_VAL,
            BelAttributeEnum {
                bits: vec![
                    common_drp_bit(tcid == tcls::GTP_COMMON_MID, 0x2d, 28),
                    common_drp_bit(tcid == tcls::GTP_COMMON_MID, 0x2d, 29),
                    common_drp_bit(tcid == tcls::GTP_COMMON_MID, 0x2d, 30),
                ],
                values: EntityPartVec::from_iter([
                    (enums::GTP_COMMON_PLLREFCLKSEL::NONE, bits![0, 0, 0]),
                    (enums::GTP_COMMON_PLLREFCLKSEL::GTREFCLK0, bits![1, 0, 0]),
                    (enums::GTP_COMMON_PLLREFCLKSEL::GTREFCLK1, bits![0, 1, 0]),
                    (
                        enums::GTP_COMMON_PLLREFCLKSEL::GTEASTREFCLK0,
                        bits![1, 1, 0],
                    ),
                    (
                        enums::GTP_COMMON_PLLREFCLKSEL::GTEASTREFCLK1,
                        bits![0, 0, 1],
                    ),
                    (
                        enums::GTP_COMMON_PLLREFCLKSEL::GTWESTREFCLK0,
                        bits![1, 0, 1],
                    ),
                    (
                        enums::GTP_COMMON_PLLREFCLKSEL::GTWESTREFCLK1,
                        bits![0, 1, 1],
                    ),
                    (enums::GTP_COMMON_PLLREFCLKSEL::GTGREFCLK, bits![1, 1, 1]),
                ]),
            },
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            GTP_COMMON::PLL1REFCLKSEL_MODE_DYNAMIC,
            common_drp_bit(tcid == tcls::GTP_COMMON_MID, 0x2d, 31).pos(),
        );
    }
    for tcid in [tcls::GTX_COMMON, tcls::GTH_COMMON] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::GTX_COMMON;
        for &pin in GTX_COMMON_INVPINS {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        let mut drp = vec![];
        for reg in 0..0x60 {
            for bit in 0..16 {
                drp.push(common_drp_bit(false, reg, bit).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, GTX_COMMON::DRP, drp);

        for (aid, _aname, attr) in &ctx.edev.db[GTX_COMMON].attributes {
            match aid {
                GTX_COMMON::DRP
                | GTX_COMMON::QPLLREFCLKSEL_STATIC_VAL
                | GTX_COMMON::QPLLREFCLKSEL_MODE_DYNAMIC
                | GTX_COMMON::MUX_SOUTHREFCLKOUT0
                | GTX_COMMON::MUX_SOUTHREFCLKOUT1
                | GTX_COMMON::MUX_NORTHREFCLKOUT0
                | GTX_COMMON::MUX_NORTHREFCLKOUT1 => (),

                GTX_COMMON::QPLL_RP_COMP
                | GTX_COMMON::QPLL_VTRL_RESET
                | GTX_COMMON::RCAL_CFG
                | GTX_COMMON::RSVD_ATTR0
                | GTX_COMMON::RSVD_ATTR1
                    if tcid == tcls::GTX_COMMON =>
                {
                    continue;
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

        ctx.collect_bel_attr_default_ocd(
            tcid,
            bslot,
            GTX_COMMON::QPLLREFCLKSEL_STATIC_VAL,
            enums::GTX_COMMON_PLLREFCLKSEL::NONE,
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_bel_attr(tcid, bslot, GTX_COMMON::QPLLREFCLKSEL_MODE_DYNAMIC);
        if ctx.edev.tile_index[tcid].len() > 1 {
            ctx.collect_bel_attr_default_ocd(
                tcid,
                bslot,
                GTX_COMMON::MUX_NORTHREFCLKOUT0,
                enums::HCLK_GTX_MUX_NORTHREFCLKOUT0::NONE,
                OcdMode::BitOrderDrpV6,
            );
            ctx.collect_bel_attr_default_ocd(
                tcid,
                bslot,
                GTX_COMMON::MUX_NORTHREFCLKOUT1,
                enums::HCLK_GTX_MUX_NORTHREFCLKOUT1::NONE,
                OcdMode::BitOrderDrpV6,
            );
            ctx.collect_bel_attr_default_ocd(
                tcid,
                bslot,
                GTX_COMMON::MUX_SOUTHREFCLKOUT0,
                enums::HCLK_GTX_MUX_SOUTHREFCLKOUT0::NONE,
                OcdMode::BitOrderDrpV6,
            );
            ctx.collect_bel_attr_default_ocd(
                tcid,
                bslot,
                GTX_COMMON::MUX_SOUTHREFCLKOUT1,
                enums::HCLK_GTX_MUX_SOUTHREFCLKOUT1::NONE,
                OcdMode::BitOrderDrpV6,
            );
        }
    }
    for tcid in [
        tcls::GTP_COMMON,
        tcls::GTP_COMMON_MID,
        tcls::GTX_COMMON,
        tcls::GTH_COMMON,
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        for i in 0..2 {
            let bslot = bslots::GTCLK[i];
            ctx.collect_bel_input_inv_bi(tcid, bslot, GTCLK::CLKTESTSIG);
            ctx.collect_bel_attr_bi(tcid, bslot, GTCLK::CLKCM_CFG);
            ctx.collect_bel_attr_bi(tcid, bslot, GTCLK::CLKRCV_TRST);
            ctx.collect_bel_attr_default(
                tcid,
                bslot,
                GTCLK::MUX_CLKOUT,
                enums::GTCLK_MUX_CLKOUT::NONE,
            );
            let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, GTCLK::MUX_CLKOUT),
                enums::GTCLK_MUX_CLKOUT::NONE,
                enums::GTCLK_MUX_CLKOUT::O,
            );
            diff.assert_empty();
        }
    }
    if ctx.has_tcls(tcls::GTP_COMMON_MID) {
        let tcid = tcls::GTP_COMMON_MID;
        for i in 0..14 {
            let dst = wires::HROW_O[i].cell(3);
            let src = wires::HROW_I_GTP[i].cell(3);
            let fsrc = wires::HROW_I[i].cell(3);
            let diff = ctx
                .get_diff_routing(tcid, dst, fsrc.pos())
                .combine(&!ctx.peek_diff_routing(tcid, dst, src.pos()));
            ctx.insert_progbuf(tcid, src, fsrc.pos(), xlat_bit(diff));
        }
        for (i, pin) in wires::OUT_GT_MGTCLKOUT_HCLK
            .into_iter()
            .chain(wires::OUT_GT_RXOUTCLK_HCLK)
            .chain(wires::OUT_GT_TXOUTCLK_HCLK)
            .enumerate()
        {
            let dst = wires::HROW_O[i].cell(3);
            let src = pin.cell(3);
            let fsrc = ctx.edev.db_index[tcls::GTP_COMMON_MID].only_bwd(src);
            let diff = ctx
                .get_diff_routing(tcid, dst, fsrc.pos())
                .combine(&!ctx.peek_diff_routing(tcid, dst, src.pos()));
            ctx.insert_progbuf(tcid, src, fsrc.pos(), xlat_bit(diff));
        }
        for i in 0..14 {
            ctx.collect_mux(tcid, wires::HROW_O[i].cell(3));
        }

        // ... seem glued together in fuzzing? screw this. manual time.
        ctx.insert_bel_attr_bool(
            tcid,
            bslots::HCLK_DRP_GTP_MID,
            bcls::HCLK_DRP::DRP_MASK_S,
            TileBit::new(6, 0, 13).pos(),
        );
        ctx.insert_bel_attr_bool(
            tcid,
            bslots::HCLK_DRP_GTP_MID,
            bcls::HCLK_DRP::DRP_MASK_N,
            TileBit::new(6, 1, 13).pos(),
        );
    }
    for tcid in [tcls::GTP_CHANNEL, tcls::GTP_CHANNEL_MID] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::GTP_CHANNEL;
        for &pin in GTP_CHANNEL_INVPINS {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        let mut drp = vec![];
        for reg in 0..0xb0 {
            for bit in 0..16 {
                drp.push(channel_drp_bit(tcid == tcls::GTP_CHANNEL_MID, reg, bit).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, GTP_CHANNEL::DRP, drp);

        for (aid, _aname, attr) in &ctx.edev.db[GTP_CHANNEL].attributes {
            match aid {
                GTP_CHANNEL::DRP => (),

                GTP_CHANNEL::CHAN_BOND_MAX_SKEW => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..15);
                }
                GTP_CHANNEL::CLK_COR_MAX_LAT | GTP_CHANNEL::CLK_COR_MIN_LAT => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 3..61);
                }
                GTP_CHANNEL::SAS_MAX_COM => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..128);
                }
                GTP_CHANNEL::SAS_MIN_COM
                | GTP_CHANNEL::SATA_MAX_BURST
                | GTP_CHANNEL::SATA_MAX_INIT
                | GTP_CHANNEL::SATA_MAX_WAKE
                | GTP_CHANNEL::SATA_MIN_INIT
                | GTP_CHANNEL::SATA_MIN_WAKE => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..64);
                }
                GTP_CHANNEL::SATA_MIN_BURST => {
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
    }
    for (tcid, invpins) in [
        (tcls::GTX_CHANNEL, GTX_CHANNEL_INVPINS),
        (tcls::GTH_CHANNEL, GTH_CHANNEL_INVPINS),
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::GTX_CHANNEL;
        let mut drp = vec![];
        for reg in 0..0xb0 {
            for bit in 0..16 {
                drp.push(channel_drp_bit(false, reg, bit).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, GTX_CHANNEL::DRP, drp);

        for &pin in invpins {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        for (aid, _aname, attr) in &ctx.edev.db[GTX_CHANNEL].attributes {
            match aid {
                GTX_CHANNEL::DRP
                | GTX_CHANNEL::CPLLREFCLKSEL_STATIC_VAL
                | GTX_CHANNEL::CPLLREFCLKSEL_MODE_DYNAMIC => (),

                GTX_CHANNEL::TXPI_PPMCLK_SEL
                | GTX_CHANNEL::RXOOB_CLK_CFG
                | GTX_CHANNEL::ACJTAG_DEBUG_MODE
                | GTX_CHANNEL::ACJTAG_MODE
                | GTX_CHANNEL::ACJTAG_RESET
                | GTX_CHANNEL::AEN_TXPI_PPM
                | GTX_CHANNEL::A_RXADAPTSELTEST
                | GTX_CHANNEL::A_RXDFETAP6HOLD
                | GTX_CHANNEL::A_RXDFETAP6OVRDEN
                | GTX_CHANNEL::A_RXDFETAP7HOLD
                | GTX_CHANNEL::A_RXDFETAP7OVRDEN
                | GTX_CHANNEL::A_RXDFEAGCTRL
                | GTX_CHANNEL::A_RXDFESLIDETAP
                | GTX_CHANNEL::A_RXDFESLIDETAPADAPTEN
                | GTX_CHANNEL::A_RXDFESLIDETAPHOLD
                | GTX_CHANNEL::A_RXDFESLIDETAPID
                | GTX_CHANNEL::A_RXDFESLIDETAPINITOVRDEN
                | GTX_CHANNEL::A_RXDFESLIDETAPONLYADAPTEN
                | GTX_CHANNEL::A_RXDFESLIDETAPOVRDEN
                | GTX_CHANNEL::A_RXDFESLIDETAPSTROBE
                | GTX_CHANNEL::A_RXOSCALRESET
                | GTX_CHANNEL::A_RXOSINTCFG
                | GTX_CHANNEL::A_RXOSINTEN
                | GTX_CHANNEL::A_RXOSINTHOLD
                | GTX_CHANNEL::A_RXOSINTID0
                | GTX_CHANNEL::A_RXOSINTNTRLEN
                | GTX_CHANNEL::A_RXOSINTOVRDEN
                | GTX_CHANNEL::A_RXOSINTSTROBE
                | GTX_CHANNEL::A_RXOSINTTESTOVRDEN
                | GTX_CHANNEL::A_TXPIPPMOVRDEN
                | GTX_CHANNEL::A_TXPIPPMPD
                | GTX_CHANNEL::A_TXPIPPMSEL
                | GTX_CHANNEL::A_TXQPIBIASEN
                | GTX_CHANNEL::CFOK_CFG2
                | GTX_CHANNEL::CFOK_CFG3
                | GTX_CHANNEL::ES_CLK_PHASE_SEL
                | GTX_CHANNEL::LOOPBACK_CFG
                | GTX_CHANNEL::PMA_RSV2_GTH
                | GTX_CHANNEL::PMA_RSV4_GTH
                | GTX_CHANNEL::PMA_RSV5
                | GTX_CHANNEL::RESET_POWERSAVE_DISABLE
                | GTX_CHANNEL::RXOSCALRESET_TIME
                | GTX_CHANNEL::RXOSCALRESET_TIMEOUT
                | GTX_CHANNEL::RXPI_CFG0
                | GTX_CHANNEL::RXPI_CFG1
                | GTX_CHANNEL::RXPI_CFG2
                | GTX_CHANNEL::RXPI_CFG3
                | GTX_CHANNEL::RXPI_CFG4
                | GTX_CHANNEL::RXPI_CFG5
                | GTX_CHANNEL::RXPI_CFG6
                | GTX_CHANNEL::RXSYNC_MULTILANE
                | GTX_CHANNEL::RXSYNC_OVRD
                | GTX_CHANNEL::RXSYNC_SKIP_DA
                | GTX_CHANNEL::RX_DFE_H6_CFG
                | GTX_CHANNEL::RX_DFE_H7_CFG
                | GTX_CHANNEL::RX_DFELPM_CFG0
                | GTX_CHANNEL::RX_DFELPM_CFG1
                | GTX_CHANNEL::RX_DFELPM_KLKH_AGC_STUP_EN
                | GTX_CHANNEL::RX_DFE_AGC_CFG0
                | GTX_CHANNEL::RX_DFE_AGC_CFG1
                | GTX_CHANNEL::RX_DFE_AGC_CFG2
                | GTX_CHANNEL::RX_DFE_AGC_OVRDEN
                | GTX_CHANNEL::RX_DFE_KL_LPM_KH_CFG0
                | GTX_CHANNEL::RX_DFE_KL_LPM_KH_CFG1
                | GTX_CHANNEL::RX_DFE_KL_LPM_KH_CFG2
                | GTX_CHANNEL::RX_DFE_KL_LPM_KH_OVRDEN
                | GTX_CHANNEL::RX_DFE_KL_LPM_KL_CFG0
                | GTX_CHANNEL::RX_DFE_KL_LPM_KL_CFG1
                | GTX_CHANNEL::RX_DFE_KL_LPM_KL_CFG2
                | GTX_CHANNEL::RX_DFE_KL_LPM_KL_OVRDEN
                | GTX_CHANNEL::TXOOB_CFG
                | GTX_CHANNEL::TXPI_CFG0
                | GTX_CHANNEL::TXPI_CFG1
                | GTX_CHANNEL::TXPI_CFG2
                | GTX_CHANNEL::TXPI_CFG3
                | GTX_CHANNEL::TXPI_CFG4
                | GTX_CHANNEL::TXPI_CFG5
                | GTX_CHANNEL::TXPI_GREY_SEL
                | GTX_CHANNEL::TXPI_INVSTROBE_SEL
                | GTX_CHANNEL::TXPI_PPM_CFG
                | GTX_CHANNEL::TXPI_SYNFREQ_PPM
                | GTX_CHANNEL::TXSYNC_MULTILANE
                | GTX_CHANNEL::TXSYNC_OVRD
                | GTX_CHANNEL::TXSYNC_SKIP_DA
                | GTX_CHANNEL::USE_PCS_CLK_PHASE_SEL
                | GTX_CHANNEL::RXLPM_LF_CFG_GTH
                | GTX_CHANNEL::RX_BIAS_CFG_GTH
                | GTX_CHANNEL::RX_CM_TRIM_GTH
                | GTX_CHANNEL::RX_DEBUG_CFG_GTH
                | GTX_CHANNEL::RX_DFE_KL_CFG_GTH
                | GTX_CHANNEL::TERM_RCAL_CFG_GTH
                | GTX_CHANNEL::TERM_RCAL_OVRD_GTH
                | GTX_CHANNEL::TX_DEEMPH0_GTH
                | GTX_CHANNEL::TX_DEEMPH1_GTH
                | GTX_CHANNEL::CPLL_CFG_GTH
                | GTX_CHANNEL::RXCDR_CFG_GTH
                | GTX_CHANNEL::ADAPT_CFG0
                | GTX_CHANNEL::CFOK_CFG
                | GTX_CHANNEL::RX_DFE_ST_CFG
                | GTX_CHANNEL::TX_RXDETECT_PRECHARGE_TIME
                    if tcid == tcls::GTX_CHANNEL => {}

                GTX_CHANNEL::A_RXDFEXYDHOLD
                | GTX_CHANNEL::A_RXDFEXYDOVRDEN
                | GTX_CHANNEL::CPLL_PCD_1UI_CFG
                | GTX_CHANNEL::RX_DFE_XYD_CFG
                | GTX_CHANNEL::TX_PREDRIVER_MODE
                | GTX_CHANNEL::RXLPM_LF_CFG_GTX
                | GTX_CHANNEL::RX_BIAS_CFG_GTX
                | GTX_CHANNEL::RX_CM_TRIM_GTX
                | GTX_CHANNEL::RX_DEBUG_CFG_GTX
                | GTX_CHANNEL::RX_DFE_KL_CFG_GTX
                | GTX_CHANNEL::TERM_RCAL_CFG_GTX
                | GTX_CHANNEL::TERM_RCAL_OVRD_GTX
                | GTX_CHANNEL::TX_DEEMPH0_GTX
                | GTX_CHANNEL::TX_DEEMPH1_GTX
                | GTX_CHANNEL::CPLL_CFG_GTX
                | GTX_CHANNEL::RXCDR_CFG_GTX
                | GTX_CHANNEL::PMA_RSV2_GTX
                | GTX_CHANNEL::PMA_RSV4_GTX
                | GTX_CHANNEL::RX_DFE_KL_CFG2
                    if tcid == tcls::GTH_CHANNEL => {}

                GTX_CHANNEL::CHAN_BOND_MAX_SKEW => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..15);
                }
                GTX_CHANNEL::CLK_COR_MAX_LAT | GTX_CHANNEL::CLK_COR_MIN_LAT => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 3..61);
                }
                GTX_CHANNEL::SAS_MAX_COM => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..128);
                }
                GTX_CHANNEL::SAS_MIN_COM
                | GTX_CHANNEL::SATA_MAX_BURST
                | GTX_CHANNEL::SATA_MAX_INIT
                | GTX_CHANNEL::SATA_MAX_WAKE
                | GTX_CHANNEL::SATA_MIN_INIT
                | GTX_CHANNEL::SATA_MIN_WAKE => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..64);
                }
                GTX_CHANNEL::SATA_MIN_BURST => {
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
        ctx.collect_bel_attr_default_ocd(
            tcid,
            bslot,
            GTX_CHANNEL::CPLLREFCLKSEL_STATIC_VAL,
            enums::GTX_COMMON_PLLREFCLKSEL::NONE,
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_bel_attr(tcid, bslot, GTX_CHANNEL::CPLLREFCLKSEL_MODE_DYNAMIC);
    }
}
