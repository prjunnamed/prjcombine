use prjcombine_interconnect::db::{BelAttributeType, BelInputId, BelKind};
use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::TileBit;
use prjcombine_virtex4::defs::{
    bcls::{CRC32, GTP_DUAL, GTX_DUAL},
    bslots,
    enums::{self, GTP_CHAN_BOND_MODE},
    virtex5::tcls,
};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    virtex4::specials,
};

const GTP_INVPINS: &[BelInputId] = &[
    GTP_DUAL::DCLK,
    GTP_DUAL::RXUSRCLK0,
    GTP_DUAL::RXUSRCLK1,
    GTP_DUAL::RXUSRCLK20,
    GTP_DUAL::RXUSRCLK21,
    GTP_DUAL::TXUSRCLK0,
    GTP_DUAL::TXUSRCLK1,
    GTP_DUAL::TXUSRCLK20,
    GTP_DUAL::TXUSRCLK21,
];

const GTX_INVPINS: &[BelInputId] = &[
    GTX_DUAL::DCLK,
    GTX_DUAL::RXUSRCLK0,
    GTX_DUAL::RXUSRCLK1,
    GTX_DUAL::RXUSRCLK20,
    GTX_DUAL::RXUSRCLK21,
    GTX_DUAL::TXUSRCLK0,
    GTX_DUAL::TXUSRCLK1,
    GTX_DUAL::TXUSRCLK20,
    GTX_DUAL::TXUSRCLK21,
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for (tcid, bslot, invpins) in [
        (tcls::GTP, bslots::GTP_DUAL, GTP_INVPINS),
        (tcls::GTX, bslots::GTX_DUAL, GTX_INVPINS),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslot);
        let mode = backend.edev.db.bel_slots.key(bslot).as_str();
        for &pin in invpins {
            bctx.mode(mode)
                .mutex("USRCLK", "INV")
                .test_bel_input_inv_auto(pin);
        }
        let BelKind::Class(bcid) = backend.edev.db.bel_slots[bslot].kind else {
            unreachable!()
        };
        for (aid, aname, attr) in &backend.edev.db[bcid].attributes {
            match (bcid, aid) {
                (GTP_DUAL, GTP_DUAL::DRP) | (GTX_DUAL, GTX_DUAL::DRP) => (),
                (GTP_DUAL, GTP_DUAL::DRP_MASK) | (GTX_DUAL, GTX_DUAL::DRP_MASK) => {
                    bctx.build().test_bel_attr_bits(aid).mode(mode).commit();
                }
                (GTP_DUAL, GTP_DUAL::USRCLK_ENABLE_0) | (GTX_DUAL, GTX_DUAL::USRCLK_ENABLE_0) => {
                    for pin in ["RXUSRCLK0", "TXUSRCLK0"] {
                        bctx.mode(mode)
                            .mutex("USRCLK", pin)
                            .test_bel_attr_bits(aid)
                            .pin(pin)
                            .commit();
                    }
                }
                (GTP_DUAL, GTP_DUAL::USRCLK_ENABLE_1) | (GTX_DUAL, GTX_DUAL::USRCLK_ENABLE_1) => {
                    for pin in ["RXUSRCLK1", "TXUSRCLK1"] {
                        bctx.mode(mode)
                            .mutex("USRCLK", pin)
                            .test_bel_attr_bits(aid)
                            .pin(pin)
                            .commit();
                    }
                }
                (GTP_DUAL, GTP_DUAL::MUX_CLKIN) | (GTX_DUAL, GTX_DUAL::MUX_CLKIN) => {
                    bctx.build()
                        .mutex("MUX.CLKIN", "GREFCLK")
                        .test_bel_attr_val(aid, enums::GTP_MUX_CLKIN::GREFCLK)
                        .pip("CLKIN", "GREFCLK")
                        .commit();
                    bctx.build()
                        .mutex("MUX.CLKIN", "CLKPN")
                        .test_bel_attr_val(aid, enums::GTP_MUX_CLKIN::CLKPN)
                        .pip("CLKIN", "BUFDS_O")
                        .commit();
                    bctx.build()
                        .mutex("MUX.CLKIN", "CLKOUT_NORTH_S")
                        .test_bel_attr_val(aid, enums::GTP_MUX_CLKIN::CLKOUT_NORTH_S)
                        .pip("CLKIN", "CLKOUT_NORTH_S")
                        .commit();
                    bctx.build()
                        .mutex("MUX.CLKIN", "CLKOUT_SOUTH_N")
                        .test_bel_attr_val(aid, enums::GTP_MUX_CLKIN::CLKOUT_SOUTH_N)
                        .pip("CLKIN", "CLKOUT_SOUTH_N")
                        .commit();
                }
                (GTP_DUAL, GTP_DUAL::MUX_CLKOUT_NORTH) | (GTX_DUAL, GTX_DUAL::MUX_CLKOUT_NORTH) => {
                    bctx.build()
                        .mutex("MUX.CLKOUT_NORTH", "CLKPN")
                        .test_bel_attr_val(aid, enums::GTP_MUX_CLKOUT_NORTH::CLKPN)
                        .pip("CLKOUT_NORTH", "BUFDS_O")
                        .commit();
                    bctx.build()
                        .mutex("MUX.CLKOUT_NORTH", "CLKOUT_NORTH_S")
                        .test_bel_attr_val(aid, enums::GTP_MUX_CLKOUT_NORTH::CLKOUT_NORTH_S)
                        .pip("CLKOUT_NORTH", "CLKOUT_NORTH_S")
                        .commit();
                }
                (GTP_DUAL, GTP_DUAL::MUX_CLKOUT_SOUTH) | (GTX_DUAL, GTX_DUAL::MUX_CLKOUT_SOUTH) => {
                    bctx.build()
                        .mutex("MUX.CLKOUT_SOUTH", "CLKPN")
                        .test_bel_attr_val(aid, enums::GTP_MUX_CLKOUT_SOUTH::CLKPN)
                        .pip("CLKOUT_SOUTH", "BUFDS_O")
                        .commit();
                    bctx.build()
                        .mutex("MUX.CLKOUT_SOUTH", "CLKOUT_SOUTH_N")
                        .test_bel_attr_val(aid, enums::GTP_MUX_CLKOUT_SOUTH::CLKOUT_SOUTH_N)
                        .pip("CLKOUT_SOUTH", "CLKOUT_SOUTH_N")
                        .commit();
                }

                (GTP_DUAL, GTP_DUAL::CLKINDC_B) => {
                    bctx.mode(mode)
                        .pip("BUFDS_IP", "IPAD_BUFDS_IP_O")
                        .pip("BUFDS_IN", "IPAD_BUFDS_IN_O")
                        .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                }

                (
                    GTP_DUAL,
                    GTP_DUAL::CHAN_BOND_1_MAX_SKEW_0
                    | GTP_DUAL::CHAN_BOND_1_MAX_SKEW_1
                    | GTP_DUAL::CHAN_BOND_2_MAX_SKEW_0
                    | GTP_DUAL::CHAN_BOND_2_MAX_SKEW_1,
                )
                | (
                    GTX_DUAL,
                    GTX_DUAL::CHAN_BOND_1_MAX_SKEW_0
                    | GTX_DUAL::CHAN_BOND_1_MAX_SKEW_1
                    | GTX_DUAL::CHAN_BOND_2_MAX_SKEW_0
                    | GTX_DUAL::CHAN_BOND_2_MAX_SKEW_1,
                ) => {
                    for val in 1..15 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                (
                    GTP_DUAL,
                    GTP_DUAL::CLK_COR_MAX_LAT_0
                    | GTP_DUAL::CLK_COR_MAX_LAT_1
                    | GTP_DUAL::CLK_COR_MIN_LAT_0
                    | GTP_DUAL::CLK_COR_MIN_LAT_1,
                )
                | (
                    GTX_DUAL,
                    GTX_DUAL::CLK_COR_MAX_LAT_0
                    | GTX_DUAL::CLK_COR_MAX_LAT_1
                    | GTX_DUAL::CLK_COR_MIN_LAT_0
                    | GTX_DUAL::CLK_COR_MIN_LAT_1,
                ) => {
                    for val in 3..49 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }
                (
                    GTP_DUAL,
                    GTP_DUAL::SATA_MAX_BURST_0
                    | GTP_DUAL::SATA_MAX_BURST_1
                    | GTP_DUAL::SATA_MAX_INIT_0
                    | GTP_DUAL::SATA_MAX_INIT_1
                    | GTP_DUAL::SATA_MAX_WAKE_0
                    | GTP_DUAL::SATA_MAX_WAKE_1
                    | GTP_DUAL::SATA_MIN_BURST_0
                    | GTP_DUAL::SATA_MIN_BURST_1
                    | GTP_DUAL::SATA_MIN_INIT_0
                    | GTP_DUAL::SATA_MIN_INIT_1
                    | GTP_DUAL::SATA_MIN_WAKE_0
                    | GTP_DUAL::SATA_MIN_WAKE_1,
                )
                | (
                    GTX_DUAL,
                    GTX_DUAL::SATA_MAX_BURST_0
                    | GTX_DUAL::SATA_MAX_BURST_1
                    | GTX_DUAL::SATA_MAX_INIT_0
                    | GTX_DUAL::SATA_MAX_INIT_1
                    | GTX_DUAL::SATA_MAX_WAKE_0
                    | GTX_DUAL::SATA_MAX_WAKE_1
                    | GTX_DUAL::SATA_MIN_BURST_0
                    | GTX_DUAL::SATA_MIN_BURST_1
                    | GTX_DUAL::SATA_MIN_INIT_0
                    | GTX_DUAL::SATA_MIN_INIT_1
                    | GTX_DUAL::SATA_MIN_WAKE_0
                    | GTX_DUAL::SATA_MIN_WAKE_1,
                ) => {
                    for val in 1..62 {
                        bctx.mode(mode)
                            .test_bel_attr_bitvec_u32(aid, val)
                            .attr(aname, val.to_string())
                            .commit();
                    }
                }

                (
                    GTP_DUAL,
                    GTP_DUAL::CHAN_BOND_LEVEL_0
                    | GTP_DUAL::CHAN_BOND_LEVEL_1
                    | GTP_DUAL::CLK_COR_REPEAT_WAIT_0
                    | GTP_DUAL::CLK_COR_REPEAT_WAIT_1
                    | GTP_DUAL::TXOUTCLK_SEL_0
                    | GTP_DUAL::TXOUTCLK_SEL_1
                    | GTP_DUAL::TX_SYNC_FILTERB,
                )
                | (
                    GTX_DUAL,
                    GTX_DUAL::CHAN_BOND_LEVEL_0
                    | GTX_DUAL::CHAN_BOND_LEVEL_1
                    | GTX_DUAL::CB2_INH_CC_PERIOD_0
                    | GTX_DUAL::CB2_INH_CC_PERIOD_1
                    | GTX_DUAL::CLK_COR_REPEAT_WAIT_0
                    | GTX_DUAL::CLK_COR_REPEAT_WAIT_1
                    | GTX_DUAL::TXOUTCLK_SEL_0
                    | GTX_DUAL::TXOUTCLK_SEL_1,
                ) => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                }

                (
                    GTP_DUAL,
                    GTP_DUAL::PCS_COM_CFG
                    | GTP_DUAL::PMA_CDR_SCAN_0
                    | GTP_DUAL::PMA_CDR_SCAN_1
                    | GTP_DUAL::PMA_COM_CFG
                    | GTP_DUAL::PMA_RX_CFG_0
                    | GTP_DUAL::PMA_RX_CFG_1
                    | GTP_DUAL::PRBS_ERR_THRESHOLD_0
                    | GTP_DUAL::PRBS_ERR_THRESHOLD_1
                    | GTP_DUAL::TRANS_TIME_FROM_P2_0
                    | GTP_DUAL::TRANS_TIME_FROM_P2_1
                    | GTP_DUAL::TRANS_TIME_NON_P2_0
                    | GTP_DUAL::TRANS_TIME_NON_P2_1
                    | GTP_DUAL::TRANS_TIME_TO_P2_0
                    | GTP_DUAL::TRANS_TIME_TO_P2_1
                    | GTP_DUAL::TX_DETECT_RX_CFG_0
                    | GTP_DUAL::TX_DETECT_RX_CFG_1,
                )
                | (
                    GTX_DUAL,
                    GTX_DUAL::PLL_COM_CFG
                    | GTX_DUAL::PLL_CP_CFG
                    | GTX_DUAL::PLL_TDCC_CFG
                    | GTX_DUAL::PMA_CDR_SCAN_0
                    | GTX_DUAL::PMA_CDR_SCAN_1
                    | GTX_DUAL::PMA_COM_CFG
                    | GTX_DUAL::PMA_RXSYNC_CFG_0
                    | GTX_DUAL::PMA_RXSYNC_CFG_1
                    | GTX_DUAL::PMA_RX_CFG_0
                    | GTX_DUAL::PMA_RX_CFG_1
                    | GTX_DUAL::PMA_TX_CFG_0
                    | GTX_DUAL::PMA_TX_CFG_1
                    | GTX_DUAL::PRBS_ERR_THRESHOLD_0
                    | GTX_DUAL::PRBS_ERR_THRESHOLD_1
                    | GTX_DUAL::TRANS_TIME_FROM_P2_0
                    | GTX_DUAL::TRANS_TIME_FROM_P2_1
                    | GTX_DUAL::TRANS_TIME_NON_P2_0
                    | GTX_DUAL::TRANS_TIME_NON_P2_1
                    | GTX_DUAL::TRANS_TIME_TO_P2_0
                    | GTX_DUAL::TRANS_TIME_TO_P2_1
                    | GTX_DUAL::TX_DETECT_RX_CFG_0
                    | GTX_DUAL::TX_DETECT_RX_CFG_1,
                ) => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        bctx.mode(mode)
                            .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
                    }
                    BelAttributeType::Enum(GTP_CHAN_BOND_MODE) => {
                        bctx.mode(mode)
                            .test_bel_attr_auto_default(aid, enums::GTP_CHAN_BOND_MODE::NONE);
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

        let mut bctx = bctx.sub(1);
        bctx.build()
            .null_bits()
            .test_bel_special(specials::BUFDS)
            .mode("BUFDS")
            .commit();

        for i in 0..4 {
            let mut bctx = ctx.bel(bslots::CRC32[i]);
            bctx.build()
                .null_bits()
                .tile_mutex("CRC_MODE", "32")
                .test_bel_special(specials::PRESENT)
                .mode("CRC32")
                .commit();
            bctx.mode("CRC32")
                .tile_mutex("CRC_MODE", "32")
                .test_bel_input_inv_auto(CRC32::CRCCLK);
            bctx.mode("CRC32")
                .tile_mutex("CRC_MODE", "32")
                .test_bel_attr_multi(CRC32::CRC_INIT, MultiValue::Hex(0));
        }

        for i in [0, 2] {
            let mut bctx = ctx.bel(bslots::CRC32[i]).sub(1);
            bctx.build()
                .tile_mutex("CRC_MODE", "64")
                .test_bel_attr_bits(CRC32::ENABLE64)
                .mode("CRC64")
                .commit();
            bctx.mode("CRC64")
                .tile_mutex("CRC_MODE", "64")
                .test_bel_input_inv_auto(CRC32::CRCCLK);
            bctx.mode("CRC64")
                .tile_mutex("CRC_MODE", "64")
                .test_bel_attr_multi(CRC32::CRC_INIT, MultiValue::Hex(0));
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tcid, bslot, invpins, drp) in [
        (tcls::GTP, bslots::GTP_DUAL, GTP_INVPINS, GTP_DUAL::DRP),
        (tcls::GTX, bslots::GTX_DUAL, GTX_INVPINS, GTX_DUAL::DRP),
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }

        fn drp_bit(idx: usize, bit: usize) -> TileBit {
            let tile = 5 + (idx >> 3);
            let frame = match bit & 3 {
                0 | 3 => 31,
                1 | 2 => 30,
                _ => unreachable!(),
            };
            let bit = (bit >> 1) | (idx & 7) << 3;
            TileBit::new(tile, frame, bit)
        }
        let mut bits = vec![];
        for i in 0..0x50 {
            for j in 0..16 {
                bits.push(drp_bit(i, j).pos());
            }
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, drp, bits);

        for &pin in invpins {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }

        let BelKind::Class(bcid) = ctx.edev.db.bel_slots[bslot].kind else {
            unreachable!()
        };
        for (aid, _aname, attr) in &ctx.edev.db[bcid].attributes {
            match (bcid, aid) {
                (GTP_DUAL, GTP_DUAL::DRP) | (GTX_DUAL, GTX_DUAL::DRP) => (),
                (GTP_DUAL, GTP_DUAL::DRP_MASK)
                | (GTX_DUAL, GTX_DUAL::DRP_MASK)
                | (GTP_DUAL, GTP_DUAL::USRCLK_ENABLE_0)
                | (GTX_DUAL, GTX_DUAL::USRCLK_ENABLE_0)
                | (GTP_DUAL, GTP_DUAL::USRCLK_ENABLE_1)
                | (GTX_DUAL, GTX_DUAL::USRCLK_ENABLE_1) => {
                    ctx.collect_bel_attr(tcid, bslot, aid);
                }

                (
                    GTP_DUAL,
                    GTP_DUAL::CHAN_BOND_1_MAX_SKEW_0
                    | GTP_DUAL::CHAN_BOND_1_MAX_SKEW_1
                    | GTP_DUAL::CHAN_BOND_2_MAX_SKEW_0
                    | GTP_DUAL::CHAN_BOND_2_MAX_SKEW_1,
                )
                | (
                    GTX_DUAL,
                    GTX_DUAL::CHAN_BOND_1_MAX_SKEW_0
                    | GTX_DUAL::CHAN_BOND_1_MAX_SKEW_1
                    | GTX_DUAL::CHAN_BOND_2_MAX_SKEW_0
                    | GTX_DUAL::CHAN_BOND_2_MAX_SKEW_1,
                ) => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..15);
                }
                (
                    GTP_DUAL,
                    GTP_DUAL::CLK_COR_MAX_LAT_0
                    | GTP_DUAL::CLK_COR_MAX_LAT_1
                    | GTP_DUAL::CLK_COR_MIN_LAT_0
                    | GTP_DUAL::CLK_COR_MIN_LAT_1,
                )
                | (
                    GTX_DUAL,
                    GTX_DUAL::CLK_COR_MAX_LAT_0
                    | GTX_DUAL::CLK_COR_MAX_LAT_1
                    | GTX_DUAL::CLK_COR_MIN_LAT_0
                    | GTX_DUAL::CLK_COR_MIN_LAT_1,
                ) => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 3..49);
                }
                (
                    GTP_DUAL,
                    GTP_DUAL::SATA_MAX_BURST_0
                    | GTP_DUAL::SATA_MAX_BURST_1
                    | GTP_DUAL::SATA_MAX_INIT_0
                    | GTP_DUAL::SATA_MAX_INIT_1
                    | GTP_DUAL::SATA_MAX_WAKE_0
                    | GTP_DUAL::SATA_MAX_WAKE_1
                    | GTP_DUAL::SATA_MIN_BURST_0
                    | GTP_DUAL::SATA_MIN_BURST_1
                    | GTP_DUAL::SATA_MIN_INIT_0
                    | GTP_DUAL::SATA_MIN_INIT_1
                    | GTP_DUAL::SATA_MIN_WAKE_0
                    | GTP_DUAL::SATA_MIN_WAKE_1,
                )
                | (
                    GTX_DUAL,
                    GTX_DUAL::SATA_MAX_BURST_0
                    | GTX_DUAL::SATA_MAX_BURST_1
                    | GTX_DUAL::SATA_MAX_INIT_0
                    | GTX_DUAL::SATA_MAX_INIT_1
                    | GTX_DUAL::SATA_MAX_WAKE_0
                    | GTX_DUAL::SATA_MAX_WAKE_1
                    | GTX_DUAL::SATA_MIN_BURST_0
                    | GTX_DUAL::SATA_MIN_BURST_1
                    | GTX_DUAL::SATA_MIN_INIT_0
                    | GTX_DUAL::SATA_MIN_INIT_1
                    | GTX_DUAL::SATA_MIN_WAKE_0
                    | GTX_DUAL::SATA_MIN_WAKE_1,
                ) => {
                    ctx.collect_bel_attr_sparse(tcid, bslot, aid, 1..62);
                }

                _ => match attr.typ {
                    BelAttributeType::Bool => {
                        ctx.collect_bel_attr_bi(tcid, bslot, aid);
                    }
                    BelAttributeType::Enum(GTP_CHAN_BOND_MODE) => {
                        ctx.collect_bel_attr_default(
                            tcid,
                            bslot,
                            aid,
                            enums::GTP_CHAN_BOND_MODE::NONE,
                        );
                    }
                    _ => {
                        ctx.collect_bel_attr(tcid, bslot, aid);
                    }
                },
            }
        }

        for (idx, bslot) in bslots::CRC32.into_iter().enumerate() {
            ctx.collect_bel_input_inv_bi(tcid, bslot, CRC32::CRCCLK);
            ctx.collect_bel_attr(tcid, bslot, CRC32::CRC_INIT);
            if idx.is_multiple_of(2) {
                ctx.collect_bel_attr(tcid, bslot, CRC32::ENABLE64);
            }
        }
    }
}
