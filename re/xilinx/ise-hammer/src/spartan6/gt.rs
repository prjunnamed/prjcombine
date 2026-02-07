use prjcombine_interconnect::{
    db::{BelAttributeType, BelInputId},
    dir::DirH,
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{
    OcdMode, extract_common_diff, xlat_bit, xlat_bitvec, xlat_enum_attr_ocd,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_spartan6::{
    chip::Gts,
    defs::{bcls, bslots, enums, tcls},
};
use prjcombine_types::bsdata::TileBit;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
    spartan6::specials,
};

const GTP_INVPINS: &[BelInputId] = &[
    bcls::GTP::DCLK,
    bcls::GTP::RXUSRCLK0,
    bcls::GTP::RXUSRCLK1,
    bcls::GTP::RXUSRCLK20,
    bcls::GTP::RXUSRCLK21,
    bcls::GTP::TXUSRCLK0,
    bcls::GTP::TXUSRCLK1,
    bcls::GTP::TXUSRCLK20,
    bcls::GTP::TXUSRCLK21,
    bcls::GTP::TSTCLK0,
    bcls::GTP::TSTCLK1,
];

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
        let ExpandedDevice::Spartan6(edev) = backend.edev else {
            unreachable!()
        };
        match self.0 {
            DirH::W => {
                if tcrd.col >= edev.chip.col_clk {
                    return None;
                }
            }
            DirH::E => {
                if tcrd.col < edev.chip.col_clk {
                    return None;
                }
            }
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Spartan6(edev) = backend.edev else {
        unreachable!()
    };
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::GTP) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::GTP);
    let mode = "GTPA1_DUAL";

    bctx.build()
        .null_bits()
        .global("GLUTMASK", "NO")
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();

    for &pin in GTP_INVPINS {
        bctx.mode(mode).test_bel_input_inv_auto(pin);
    }

    for (aid, aname, attr) in &backend.edev.db[bcls::GTP].attributes {
        match aid {
            bcls::GTP::DRP => (),
            bcls::GTP::CB2_INH_CC_PERIOD_0
            | bcls::GTP::CB2_INH_CC_PERIOD_1
            | bcls::GTP::CLK_COR_REPEAT_WAIT_0
            | bcls::GTP::CLK_COR_REPEAT_WAIT_1 => {
                let BelAttributeType::BitVec(width) = attr.typ else {
                    unreachable!()
                };
                bctx.mode(mode).test_bel_attr_bits(aid).multi_attr(
                    aname,
                    MultiValue::Dec(0),
                    width,
                );
            }
            bcls::GTP::PLL_COM_CFG_0
            | bcls::GTP::PLL_COM_CFG_1
            | bcls::GTP::PLL_CP_CFG_0
            | bcls::GTP::PLL_CP_CFG_1
            | bcls::GTP::PMA_CDR_SCAN_0
            | bcls::GTP::PMA_CDR_SCAN_1
            | bcls::GTP::PMA_RXSYNC_CFG_0
            | bcls::GTP::PMA_RXSYNC_CFG_1
            | bcls::GTP::PMA_RX_CFG_0
            | bcls::GTP::PMA_RX_CFG_1
            | bcls::GTP::PMA_TX_CFG_0
            | bcls::GTP::PMA_TX_CFG_1
            | bcls::GTP::TRANS_TIME_FROM_P2_0
            | bcls::GTP::TRANS_TIME_FROM_P2_1
            | bcls::GTP::TRANS_TIME_NON_P2_0
            | bcls::GTP::TRANS_TIME_NON_P2_1
            | bcls::GTP::TRANS_TIME_TO_P2_0
            | bcls::GTP::TRANS_TIME_TO_P2_1
            | bcls::GTP::TST_ATTR_0
            | bcls::GTP::TST_ATTR_1
            | bcls::GTP::TX_DETECT_RX_CFG_0
            | bcls::GTP::TX_DETECT_RX_CFG_1
            | bcls::GTP::PMA_COM_CFG_EAST
            | bcls::GTP::PMA_COM_CFG_WEST => {
                let BelAttributeType::BitVec(width) = attr.typ else {
                    unreachable!()
                };
                bctx.mode(mode).test_bel_attr_bits(aid).multi_attr(
                    aname,
                    MultiValue::Hex(0),
                    width,
                );
            }
            bcls::GTP::CHAN_BOND_1_MAX_SKEW_0
            | bcls::GTP::CHAN_BOND_1_MAX_SKEW_1
            | bcls::GTP::CHAN_BOND_2_MAX_SKEW_0
            | bcls::GTP::CHAN_BOND_2_MAX_SKEW_1 => {
                for val in 1..15 {
                    bctx.mode(mode)
                        .test_bel_attr_bitvec_u32(aid, val)
                        .attr(aname, val.to_string())
                        .commit();
                }
            }
            bcls::GTP::CLK_COR_MAX_LAT_0
            | bcls::GTP::CLK_COR_MAX_LAT_1
            | bcls::GTP::CLK_COR_MIN_LAT_0
            | bcls::GTP::CLK_COR_MIN_LAT_1 => {
                for val in 3..49 {
                    bctx.mode(mode)
                        .test_bel_attr_bitvec_u32(aid, val)
                        .attr(aname, val.to_string())
                        .commit();
                }
            }
            bcls::GTP::SATA_MAX_BURST_0
            | bcls::GTP::SATA_MAX_BURST_1
            | bcls::GTP::SATA_MAX_INIT_0
            | bcls::GTP::SATA_MAX_INIT_1
            | bcls::GTP::SATA_MAX_WAKE_0
            | bcls::GTP::SATA_MAX_WAKE_1
            | bcls::GTP::SATA_MIN_BURST_0
            | bcls::GTP::SATA_MIN_BURST_1
            | bcls::GTP::SATA_MIN_INIT_0
            | bcls::GTP::SATA_MIN_INIT_1
            | bcls::GTP::SATA_MIN_WAKE_0
            | bcls::GTP::SATA_MIN_WAKE_1 => {
                for val in 1..62 {
                    bctx.mode(mode)
                        .test_bel_attr_bitvec_u32(aid, val)
                        .attr(aname, val.to_string())
                        .commit();
                }
            }

            bcls::GTP::MUX_CLKOUT_EAST | bcls::GTP::MUX_CLKOUT_WEST => {
                if aid == bcls::GTP::MUX_CLKOUT_WEST
                    && !matches!(edev.chip.gts, Gts::Double(..) | Gts::Quad(..))
                {
                    continue;
                }
                let side = if aid == bcls::GTP::MUX_CLKOUT_EAST {
                    DirH::W
                } else {
                    DirH::E
                };
                for (val, pin) in &backend.edev.db[enums::GTP_MUX_CLKOUT].values {
                    bctx.build()
                        .mutex("MUX.CLKOUT_EW", pin)
                        .prop(DeviceSide(side))
                        .test_bel_attr_val(aid, val)
                        .pip("CLKOUT_EW", pin)
                        .commit();
                }
            }
            bcls::GTP::REFSELPLL0_STATIC_ENABLE | bcls::GTP::REFSELPLL1_STATIC_ENABLE => (),
            bcls::GTP::REFSELPLL0_STATIC_VAL | bcls::GTP::REFSELPLL1_STATIC_VAL => {
                let idx = if aid == bcls::GTP::REFSELPLL1_STATIC_VAL {
                    1
                } else {
                    0
                };
                for (val, pin) in [
                    (enums::GTP_MUX_REFSELPLL::CLKINEAST, "CLKINEAST"),
                    (enums::GTP_MUX_REFSELPLL::CLKINWEST, "CLKINWEST"),
                ] {
                    bctx.build()
                        .mutex(format!("REFSELPLL{idx}"), pin)
                        .test_bel_attr_val(aid, val)
                        .pip(format!("{pin}{idx}"), pin)
                        .commit();
                }
                for (val, pin, opin) in [
                    (enums::GTP_MUX_REFSELPLL::CLK0, "CLK0", "BUFDS0_O"),
                    (enums::GTP_MUX_REFSELPLL::CLK1, "CLK1", "BUFDS1_O"),
                ] {
                    bctx.build()
                        .mutex(format!("REFSELPLL{idx}"), pin)
                        .test_bel_attr_val(aid, val)
                        .pip(format!("{pin}{idx}"), opin)
                        .commit();
                }
                for (val, pin) in [
                    (enums::GTP_MUX_REFSELPLL::GCLK0, "GCLK0"),
                    (enums::GTP_MUX_REFSELPLL::GCLK1, "GCLK1"),
                    (enums::GTP_MUX_REFSELPLL::PLLCLK0, "PLLCLK0"),
                    (enums::GTP_MUX_REFSELPLL::PLLCLK1, "PLLCLK1"),
                ] {
                    bctx.build()
                        .mutex(format!("REFSELPLL{idx}"), pin)
                        .test_bel_attr_val(aid, val)
                        .pin_pips(format!("{pin}{idx}"))
                        .commit();
                }
            }

            _ => match attr.typ {
                BelAttributeType::Bool => {
                    bctx.mode(mode)
                        .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                }
                BelAttributeType::BitVec(width) => {
                    bctx.mode(mode).test_bel_attr_bits(aid).multi_attr(
                        aname,
                        MultiValue::Bin,
                        width,
                    );
                }
                BelAttributeType::Enum(_) => {
                    bctx.mode(mode).test_bel_attr(aid);
                }
                _ => unreachable!(),
            },
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Spartan6(edev) = ctx.edev else {
        unreachable!()
    };

    let tcid = tcls::GTP;
    let bslot = bslots::GTP;

    if !ctx.has_tcls(tcid) {
        return;
    }

    fn drp_bit(idx: usize, bit: usize) -> TileBit {
        let tile = 8 + ((idx >> 2) & 7);
        let bit = bit + 16 * (idx & 3);
        let frame = 25 - ((idx >> 5) & 3);
        TileBit::new(tile, frame, bit)
    }

    let mut drp = vec![];
    for i in 0..0x80 {
        for j in 0..16 {
            drp.push(drp_bit(i, j).pos());
        }
    }
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::GTP::DRP, drp);

    for &pin in GTP_INVPINS {
        ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
    }

    for (aid, _, attr) in &ctx.edev.db[bcls::GTP].attributes {
        if aid == bcls::GTP::MUX_CLKOUT_WEST
            && !matches!(edev.chip.gts, Gts::Double(..) | Gts::Quad(..))
        {
            continue;
        }
        match aid {
            bcls::GTP::DRP => (),

            // sigh. bugs.
            bcls::GTP::RXPRBSERR_LOOPBACK_1 => {
                // sigh. bugs.
                ctx.get_diff_attr_bit(tcid, bslot, aid, 0).assert_empty();
                ctx.insert_bel_attr_bool(tcid, bslot, aid, TileBit::new(8, 22, 48).pos());
            }
            bcls::GTP::COMMA_10B_ENABLE_1 => {
                let mut diffs = ctx.get_diffs_attr_bits(tcid, bslot, aid, 10);
                diffs[3].bits.insert(TileBit::new(11, 23, 3), true);
                assert_eq!(diffs[4].bits.remove(&TileBit::new(11, 23, 3)), Some(true));
                ctx.insert_bel_attr_bitvec(tcid, bslot, aid, xlat_bitvec(diffs));
            }

            bcls::GTP::CHAN_BOND_1_MAX_SKEW_0
            | bcls::GTP::CHAN_BOND_1_MAX_SKEW_1
            | bcls::GTP::CHAN_BOND_2_MAX_SKEW_0
            | bcls::GTP::CHAN_BOND_2_MAX_SKEW_1 => {
                ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..15);
            }
            bcls::GTP::CLK_COR_MAX_LAT_0
            | bcls::GTP::CLK_COR_MAX_LAT_1
            | bcls::GTP::CLK_COR_MIN_LAT_0
            | bcls::GTP::CLK_COR_MIN_LAT_1 => {
                ctx.collect_bel_attr_sparse(tcid, bslot, aid, 3..49);
            }
            bcls::GTP::SATA_MAX_BURST_0
            | bcls::GTP::SATA_MAX_BURST_1
            | bcls::GTP::SATA_MAX_INIT_0
            | bcls::GTP::SATA_MAX_INIT_1
            | bcls::GTP::SATA_MAX_WAKE_0
            | bcls::GTP::SATA_MAX_WAKE_1
            | bcls::GTP::SATA_MIN_BURST_0
            | bcls::GTP::SATA_MIN_BURST_1
            | bcls::GTP::SATA_MIN_INIT_0
            | bcls::GTP::SATA_MIN_INIT_1
            | bcls::GTP::SATA_MIN_WAKE_0
            | bcls::GTP::SATA_MIN_WAKE_1 => {
                ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..62);
            }

            bcls::GTP::REFSELPLL0_STATIC_ENABLE | bcls::GTP::REFSELPLL1_STATIC_ENABLE => (),
            bcls::GTP::REFSELPLL0_STATIC_VAL | bcls::GTP::REFSELPLL1_STATIC_VAL => {
                let aen = if aid == bcls::GTP::REFSELPLL1_STATIC_VAL {
                    bcls::GTP::REFSELPLL1_STATIC_ENABLE
                } else {
                    bcls::GTP::REFSELPLL0_STATIC_ENABLE
                };
                let mut diffs = vec![];
                for val in ctx.edev.db[enums::GTP_MUX_REFSELPLL].values.ids() {
                    diffs.push((val, ctx.get_diff_attr_val(tcid, bslot, aid, val)));
                }
                let en = extract_common_diff(&mut diffs);
                ctx.insert_bel_attr_bool(tcid, bslot, aen, xlat_bit(en));
                ctx.insert_bel_attr_enum(
                    tcid,
                    bslot,
                    aid,
                    xlat_enum_attr_ocd(diffs, OcdMode::BitOrder),
                );
            }

            _ => match attr.typ {
                BelAttributeType::Bool => {
                    ctx.collect_bel_attr_bi(tcid, bslot, aid);
                }
                BelAttributeType::Enum(_) => {
                    ctx.collect_bel_attr_ocd(tcid, bslot, aid, OcdMode::BitOrder);
                }
                _ => {
                    ctx.collect_bel_attr(tcid, bslot, aid);
                }
            },
        }
    }
}
