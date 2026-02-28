use prjcombine_interconnect::{db::BelAttributeType, dir::DirH, grid::TileCoord};
use prjcombine_re_collector::diff::xlat_bit;
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    bcls::{self, PCIE_V7 as PCIE, PCIE3},
    bslots, tslots,
    virtex7::tcls,
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::{Delta, TileRelation},
    },
    virtex4::specials,
};

#[derive(Copy, Clone, Debug)]
struct PcieHclkPair;

impl TileRelation for PcieHclkPair {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = match edev.col_side(tcrd.col) {
            DirH::W => tcrd.col - 4,
            DirH::E => tcrd.col - 1,
        };
        let row = tcrd.row + edev.chips[tcrd.die].rows_per_reg() / 2;
        Some(tcrd.with_cr(col, row).tile(tslots::HCLK))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::PCIE) {
        let mut bctx = ctx.bel(bslots::PCIE);
        let mode = "PCIE_2_1";
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        // always appears in left column even when DRP is in right column — bug or intentional?
        bctx.mode(mode)
            .null_bits()
            .extra_tile_bel_special(PcieHclkPair, bslots::HCLK_DRP[1], specials::DRP_MASK_PCIE)
            .test_bel_special(specials::DRP_MASK_CMT)
            .pin("DRPWE")
            .commit();

        for (aid, _, attr) in &backend.edev.db[PCIE].attributes {
            if aid == PCIE::DRP {
                continue;
            }
            match attr.typ {
                BelAttributeType::Bool => {
                    bctx.mode(mode)
                        .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                }
                BelAttributeType::BitVec(_width) => {
                    let multi = if matches!(
                        aid,
                        PCIE::CFG_ECRC_ERR_CPLSTAT
                            | PCIE::DEV_CAP_ENDPOINT_L0S_LATENCY
                            | PCIE::DEV_CAP_ENDPOINT_L1_LATENCY
                            | PCIE::DEV_CAP_MAX_PAYLOAD_SUPPORTED
                            | PCIE::DEV_CAP_PHANTOM_FUNCTIONS_SUPPORT
                            | PCIE::DEV_CAP_RSVD_14_12
                            | PCIE::DEV_CAP_RSVD_17_16
                            | PCIE::DEV_CAP_RSVD_31_29
                            | PCIE::LINK_CAP_ASPM_SUPPORT
                            | PCIE::LINK_CAP_L0S_EXIT_LATENCY_COMCLK_GEN1
                            | PCIE::LINK_CAP_L0S_EXIT_LATENCY_COMCLK_GEN2
                            | PCIE::LINK_CAP_L0S_EXIT_LATENCY_GEN1
                            | PCIE::LINK_CAP_L0S_EXIT_LATENCY_GEN2
                            | PCIE::LINK_CAP_L1_EXIT_LATENCY_COMCLK_GEN1
                            | PCIE::LINK_CAP_L1_EXIT_LATENCY_COMCLK_GEN2
                            | PCIE::LINK_CAP_L1_EXIT_LATENCY_GEN1
                            | PCIE::LINK_CAP_L1_EXIT_LATENCY_GEN2
                            | PCIE::LINK_CAP_RSVD_23
                            | PCIE::LINK_CONTROL_RCB
                            | PCIE::LL_ACK_TIMEOUT_FUNC
                            | PCIE::LL_REPLAY_TIMEOUT_FUNC
                            | PCIE::MSI_CAP_MULTIMSG_EXTENSION
                            | PCIE::MSI_CAP_MULTIMSGCAP
                            | PCIE::MSIX_CAP_PBA_BIR
                            | PCIE::MSIX_CAP_TABLE_BIR
                            | PCIE::PCIE_CAP_RSVD_15_14
                            | PCIE::PL_AUTO_CONFIG
                            | PCIE::PM_ASPML0S_TIMEOUT_FUNC
                            | PCIE::PM_CAP_AUXCURRENT
                            | PCIE::PM_CAP_RSVD_04
                            | PCIE::PM_CAP_VERSION
                            | PCIE::RECRC_CHK
                            | PCIE::SLOT_CAP_SLOT_POWER_LIMIT_SCALE
                            | PCIE::TL_RX_RAM_RADDR_LATENCY
                            | PCIE::TL_RX_RAM_RDATA_LATENCY
                            | PCIE::TL_RX_RAM_WRITE_LATENCY
                            | PCIE::TL_TX_RAM_RADDR_LATENCY
                            | PCIE::TL_TX_RAM_RDATA_LATENCY
                            | PCIE::TL_TX_RAM_WRITE_LATENCY
                            | PCIE::USER_CLK_FREQ
                            | PCIE::N_FTS_COMCLK_GEN1
                            | PCIE::N_FTS_COMCLK_GEN2
                            | PCIE::N_FTS_GEN1
                            | PCIE::N_FTS_GEN2
                            | PCIE::PCIE_REVISION
                            | PCIE::VC0_TOTAL_CREDITS_CD
                            | PCIE::VC0_TOTAL_CREDITS_CH
                            | PCIE::VC0_TOTAL_CREDITS_NPD
                            | PCIE::VC0_TOTAL_CREDITS_NPH
                            | PCIE::VC0_TOTAL_CREDITS_PD
                            | PCIE::VC0_TOTAL_CREDITS_PH
                            | PCIE::VC0_TX_LASTPACKET
                    ) {
                        MultiValue::Dec(0)
                    } else {
                        MultiValue::Hex(0)
                    };
                    bctx.mode(mode).test_bel_attr_multi(aid, multi);
                }
                _ => unreachable!(),
            }
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::PCIE3) {
        let mut bctx = ctx.bel(bslots::PCIE3);
        let mode = "PCIE_3_0";
        // always turns on the "bottom" bit even in the lower region — bug or intentional?
        bctx.build()
            .null_bits()
            .test_bel_special(specials::PRESENT)
            .mode(mode)
            .commit();
        bctx.mode(mode)
            .null_bits()
            .extra_tile_bel_special(
                Delta::new(3, 0, tcls::HCLK),
                bslots::HCLK_DRP[1],
                specials::DRP_MASK_PCIE,
            )
            .extra_tile_bel_special(
                Delta::new(3, 50, tcls::HCLK),
                bslots::HCLK_DRP[1],
                specials::DRP_MASK_PCIE,
            )
            .test_bel_special(specials::DRP_MASK_CMT)
            .pin("DRPWE")
            .commit();

        for (aid, _, attr) in &backend.edev.db[PCIE3].attributes {
            match attr.typ {
                BelAttributeType::Bool => {
                    bctx.mode(mode)
                        .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                }
                BelAttributeType::BitVec(_width) => {
                    let multi = if matches!(
                        aid,
                        PCIE3::LL_ACK_TIMEOUT_FUNC
                            | PCIE3::LL_REPLAY_TIMEOUT_FUNC
                            | PCIE3::PF0_DEV_CAP_ENDPOINT_L0S_LATENCY
                            | PCIE3::PF0_DEV_CAP_ENDPOINT_L1_LATENCY
                            | PCIE3::PL_N_FTS_COMCLK_GEN1
                            | PCIE3::PL_N_FTS_COMCLK_GEN2
                            | PCIE3::PL_N_FTS_COMCLK_GEN3
                            | PCIE3::PL_N_FTS_GEN1
                            | PCIE3::PL_N_FTS_GEN2
                            | PCIE3::PL_N_FTS_GEN3
                    ) {
                        MultiValue::Dec(0)
                    } else {
                        MultiValue::Hex(0)
                    };
                    bctx.mode(mode).test_bel_attr_multi(aid, multi);
                }
                _ => unreachable!(),
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let mut got_pcie = false;
    if ctx.has_tcls(tcls::PCIE) {
        let tcid = tcls::PCIE;
        let bslot = bslots::PCIE;

        fn pcie_drp_bit(reg: usize, bit: usize) -> TileBit {
            let tile = reg / 6;
            let frame = 28 + (bit & 1);
            let bit = (bit >> 1) | (reg % 6) << 3;
            TileBit::new(tile, frame, bit)
        }
        let mut drp = vec![];
        for reg in 0..0x96 {
            for bit in 0..16 {
                drp.push(pcie_drp_bit(reg, bit).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, PCIE::DRP, drp);

        for (aid, _, attr) in &ctx.edev.db[PCIE].attributes {
            if aid == PCIE::DRP {
                continue;
            }
            match attr.typ {
                BelAttributeType::Bool => {
                    ctx.collect_bel_attr_bi(tcid, bslot, aid);
                }
                BelAttributeType::BitVec(_width) => {
                    ctx.collect_bel_attr(tcid, bslot, aid);
                }
                _ => unreachable!(),
            }
        }

        got_pcie = true;
    }
    if ctx.has_tcls(tcls::PCIE3) {
        let tcid = tcls::PCIE3;
        let bslot = bslots::PCIE3;
        for (aid, _, attr) in &ctx.edev.db[PCIE3].attributes {
            match attr.typ {
                BelAttributeType::Bool => {
                    ctx.collect_bel_attr_bi(tcid, bslot, aid);
                }
                BelAttributeType::BitVec(_width) => {
                    ctx.collect_bel_attr(tcid, bslot, aid);
                }
                _ => unreachable!(),
            }
        }

        got_pcie = true;
    }
    if got_pcie {
        let tcid = tcls::HCLK;
        let bslot = bslots::HCLK_DRP[1];
        let bit = xlat_bit(ctx.get_diff_bel_special(tcid, bslot, specials::DRP_MASK_PCIE));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::HCLK_DRP::DRP_MASK_S, bit);
    }
}
