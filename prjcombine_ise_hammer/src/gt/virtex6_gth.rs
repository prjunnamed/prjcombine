use prjcombine_hammer::Session;
use prjcombine_types::TileItem;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{xlat_bitvec, xlat_enum, CollectorCtx},
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_hex, fuzz_one, fuzz_one_extras,
};

const GTH_INVPINS: &[&str] = &[
    "DCLK",
    "SCANCLK",
    "SDSSCANCLK",
    "TPCLK",
    "TSTNOISECLK",
    "RXUSERCLKIN0",
    "RXUSERCLKIN1",
    "RXUSERCLKIN2",
    "RXUSERCLKIN3",
    "TXUSERCLKIN0",
    "TXUSERCLKIN1",
    "TXUSERCLKIN2",
    "TXUSERCLKIN3",
];

const GTH_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("CLKTESTSIG_SEL", &["USER_OPERATION", "CLKTESTSIG"]),
    (
        "RX_FABRIC_WIDTH0",
        &["6466", "8", "10", "16", "20", "32", "40", "64", "80"],
    ),
    (
        "RX_FABRIC_WIDTH1",
        &["6466", "8", "10", "16", "20", "32", "40", "64", "80"],
    ),
    (
        "RX_FABRIC_WIDTH2",
        &["6466", "8", "10", "16", "20", "32", "40", "64", "80"],
    ),
    (
        "RX_FABRIC_WIDTH3",
        &["6466", "8", "10", "16", "20", "32", "40", "64", "80"],
    ),
    (
        "TX_FABRIC_WIDTH0",
        &["6466", "8", "10", "16", "20", "32", "40", "64", "80"],
    ),
    (
        "TX_FABRIC_WIDTH1",
        &["6466", "8", "10", "16", "20", "32", "40", "64", "80"],
    ),
    (
        "TX_FABRIC_WIDTH2",
        &["6466", "8", "10", "16", "20", "32", "40", "64", "80"],
    ),
    (
        "TX_FABRIC_WIDTH3",
        &["6466", "8", "10", "16", "20", "32", "40", "64", "80"],
    ),
];

const GTH_BIN_ATTRS: &[(&str, usize)] = &[
    ("GTH_CFG_PWRUP_LANE0", 1),
    ("GTH_CFG_PWRUP_LANE1", 1),
    ("GTH_CFG_PWRUP_LANE2", 1),
    ("GTH_CFG_PWRUP_LANE3", 1),
    ("TST_PCS_LOOPBACK_LANE0", 1),
    ("TST_PCS_LOOPBACK_LANE1", 1),
    ("TST_PCS_LOOPBACK_LANE2", 1),
    ("TST_PCS_LOOPBACK_LANE3", 1),
];

const GTH_HEX_ATTRS: &[(&str, usize)] = &[
    ("BER_CONST_PTRN0", 16),
    ("BER_CONST_PTRN1", 16),
    ("BUFFER_CONFIG_LANE0", 16),
    ("BUFFER_CONFIG_LANE1", 16),
    ("BUFFER_CONFIG_LANE2", 16),
    ("BUFFER_CONFIG_LANE3", 16),
    ("DFE_TRAIN_CTRL_LANE0", 16),
    ("DFE_TRAIN_CTRL_LANE1", 16),
    ("DFE_TRAIN_CTRL_LANE2", 16),
    ("DFE_TRAIN_CTRL_LANE3", 16),
    ("DLL_CFG0", 16),
    ("DLL_CFG1", 16),
    ("E10GBASEKR_LD_COEFF_UPD_LANE0", 16),
    ("E10GBASEKR_LD_COEFF_UPD_LANE1", 16),
    ("E10GBASEKR_LD_COEFF_UPD_LANE2", 16),
    ("E10GBASEKR_LD_COEFF_UPD_LANE3", 16),
    ("E10GBASEKR_LP_COEFF_UPD_LANE0", 16),
    ("E10GBASEKR_LP_COEFF_UPD_LANE1", 16),
    ("E10GBASEKR_LP_COEFF_UPD_LANE2", 16),
    ("E10GBASEKR_LP_COEFF_UPD_LANE3", 16),
    ("E10GBASEKR_PMA_CTRL_LANE0", 16),
    ("E10GBASEKR_PMA_CTRL_LANE1", 16),
    ("E10GBASEKR_PMA_CTRL_LANE2", 16),
    ("E10GBASEKR_PMA_CTRL_LANE3", 16),
    ("E10GBASEKX_CTRL_LANE0", 16),
    ("E10GBASEKX_CTRL_LANE1", 16),
    ("E10GBASEKX_CTRL_LANE2", 16),
    ("E10GBASEKX_CTRL_LANE3", 16),
    ("E10GBASER_PCS_CFG_LANE0", 16),
    ("E10GBASER_PCS_CFG_LANE1", 16),
    ("E10GBASER_PCS_CFG_LANE2", 16),
    ("E10GBASER_PCS_CFG_LANE3", 16),
    ("E10GBASER_PCS_SEEDA0_LANE0", 16),
    ("E10GBASER_PCS_SEEDA0_LANE1", 16),
    ("E10GBASER_PCS_SEEDA0_LANE2", 16),
    ("E10GBASER_PCS_SEEDA0_LANE3", 16),
    ("E10GBASER_PCS_SEEDA1_LANE0", 16),
    ("E10GBASER_PCS_SEEDA1_LANE1", 16),
    ("E10GBASER_PCS_SEEDA1_LANE2", 16),
    ("E10GBASER_PCS_SEEDA1_LANE3", 16),
    ("E10GBASER_PCS_SEEDA2_LANE0", 16),
    ("E10GBASER_PCS_SEEDA2_LANE1", 16),
    ("E10GBASER_PCS_SEEDA2_LANE2", 16),
    ("E10GBASER_PCS_SEEDA2_LANE3", 16),
    ("E10GBASER_PCS_SEEDA3_LANE0", 16),
    ("E10GBASER_PCS_SEEDA3_LANE1", 16),
    ("E10GBASER_PCS_SEEDA3_LANE2", 16),
    ("E10GBASER_PCS_SEEDA3_LANE3", 16),
    ("E10GBASER_PCS_SEEDB0_LANE0", 16),
    ("E10GBASER_PCS_SEEDB0_LANE1", 16),
    ("E10GBASER_PCS_SEEDB0_LANE2", 16),
    ("E10GBASER_PCS_SEEDB0_LANE3", 16),
    ("E10GBASER_PCS_SEEDB1_LANE0", 16),
    ("E10GBASER_PCS_SEEDB1_LANE1", 16),
    ("E10GBASER_PCS_SEEDB1_LANE2", 16),
    ("E10GBASER_PCS_SEEDB1_LANE3", 16),
    ("E10GBASER_PCS_SEEDB2_LANE0", 16),
    ("E10GBASER_PCS_SEEDB2_LANE1", 16),
    ("E10GBASER_PCS_SEEDB2_LANE2", 16),
    ("E10GBASER_PCS_SEEDB2_LANE3", 16),
    ("E10GBASER_PCS_SEEDB3_LANE0", 16),
    ("E10GBASER_PCS_SEEDB3_LANE1", 16),
    ("E10GBASER_PCS_SEEDB3_LANE2", 16),
    ("E10GBASER_PCS_SEEDB3_LANE3", 16),
    ("E10GBASER_PCS_TEST_CTRL_LANE0", 16),
    ("E10GBASER_PCS_TEST_CTRL_LANE1", 16),
    ("E10GBASER_PCS_TEST_CTRL_LANE2", 16),
    ("E10GBASER_PCS_TEST_CTRL_LANE3", 16),
    ("E10GBASEX_PCS_TSTCTRL_LANE0", 16),
    ("E10GBASEX_PCS_TSTCTRL_LANE1", 16),
    ("E10GBASEX_PCS_TSTCTRL_LANE2", 16),
    ("E10GBASEX_PCS_TSTCTRL_LANE3", 16),
    ("GLBL0_NOISE_CTRL", 16),
    ("GLBL_AMON_SEL", 16),
    ("GLBL_DMON_SEL", 16),
    ("GLBL_PWR_CTRL", 16),
    ("LANE_AMON_SEL", 16),
    ("LANE_DMON_SEL", 16),
    ("LANE_LNK_CFGOVRD", 16),
    ("LANE_PWR_CTRL_LANE0", 16),
    ("LANE_PWR_CTRL_LANE1", 16),
    ("LANE_PWR_CTRL_LANE2", 16),
    ("LANE_PWR_CTRL_LANE3", 16),
    ("LNK_TRN_CFG_LANE0", 16),
    ("LNK_TRN_CFG_LANE1", 16),
    ("LNK_TRN_CFG_LANE2", 16),
    ("LNK_TRN_CFG_LANE3", 16),
    ("LNK_TRN_COEFF_REQ_LANE0", 16),
    ("LNK_TRN_COEFF_REQ_LANE1", 16),
    ("LNK_TRN_COEFF_REQ_LANE2", 16),
    ("LNK_TRN_COEFF_REQ_LANE3", 16),
    ("MISC_CFG", 16),
    ("MODE_CFG1", 16),
    ("MODE_CFG2", 16),
    ("MODE_CFG3", 16),
    ("MODE_CFG4", 16),
    ("MODE_CFG5", 16),
    ("MODE_CFG6", 16),
    ("MODE_CFG7", 16),
    ("PCS_ABILITY_LANE0", 16),
    ("PCS_ABILITY_LANE1", 16),
    ("PCS_ABILITY_LANE2", 16),
    ("PCS_ABILITY_LANE3", 16),
    ("PCS_CTRL1_LANE0", 16),
    ("PCS_CTRL1_LANE1", 16),
    ("PCS_CTRL1_LANE2", 16),
    ("PCS_CTRL1_LANE3", 16),
    ("PCS_CTRL2_LANE0", 16),
    ("PCS_CTRL2_LANE1", 16),
    ("PCS_CTRL2_LANE2", 16),
    ("PCS_CTRL2_LANE3", 16),
    ("PCS_MISC_CFG_0_LANE0", 16),
    ("PCS_MISC_CFG_0_LANE1", 16),
    ("PCS_MISC_CFG_0_LANE2", 16),
    ("PCS_MISC_CFG_0_LANE3", 16),
    ("PCS_MISC_CFG_1_LANE0", 16),
    ("PCS_MISC_CFG_1_LANE1", 16),
    ("PCS_MISC_CFG_1_LANE2", 16),
    ("PCS_MISC_CFG_1_LANE3", 16),
    ("PCS_MODE_LANE0", 16),
    ("PCS_MODE_LANE1", 16),
    ("PCS_MODE_LANE2", 16),
    ("PCS_MODE_LANE3", 16),
    ("PCS_RESET_1_LANE0", 16),
    ("PCS_RESET_1_LANE1", 16),
    ("PCS_RESET_1_LANE2", 16),
    ("PCS_RESET_1_LANE3", 16),
    ("PCS_RESET_LANE0", 16),
    ("PCS_RESET_LANE1", 16),
    ("PCS_RESET_LANE2", 16),
    ("PCS_RESET_LANE3", 16),
    ("PCS_TYPE_LANE0", 16),
    ("PCS_TYPE_LANE1", 16),
    ("PCS_TYPE_LANE2", 16),
    ("PCS_TYPE_LANE3", 16),
    ("PLL_CFG0", 16),
    ("PLL_CFG1", 16),
    ("PLL_CFG2", 16),
    ("PMA_CTRL1_LANE0", 16),
    ("PMA_CTRL1_LANE1", 16),
    ("PMA_CTRL1_LANE2", 16),
    ("PMA_CTRL1_LANE3", 16),
    ("PMA_CTRL2_LANE0", 16),
    ("PMA_CTRL2_LANE1", 16),
    ("PMA_CTRL2_LANE2", 16),
    ("PMA_CTRL2_LANE3", 16),
    ("PMA_LPBK_CTRL_LANE0", 16),
    ("PMA_LPBK_CTRL_LANE1", 16),
    ("PMA_LPBK_CTRL_LANE2", 16),
    ("PMA_LPBK_CTRL_LANE3", 16),
    ("PRBS_BER_CFG0_LANE0", 16),
    ("PRBS_BER_CFG0_LANE1", 16),
    ("PRBS_BER_CFG0_LANE2", 16),
    ("PRBS_BER_CFG0_LANE3", 16),
    ("PRBS_BER_CFG1_LANE0", 16),
    ("PRBS_BER_CFG1_LANE1", 16),
    ("PRBS_BER_CFG1_LANE2", 16),
    ("PRBS_BER_CFG1_LANE3", 16),
    ("PRBS_CFG_LANE0", 16),
    ("PRBS_CFG_LANE1", 16),
    ("PRBS_CFG_LANE2", 16),
    ("PRBS_CFG_LANE3", 16),
    ("PTRN_CFG0_LSB", 16),
    ("PTRN_CFG0_MSB", 16),
    ("PTRN_LEN_CFG", 16),
    ("PWRUP_DLY", 16),
    ("RX_AEQ_VAL0_LANE0", 16),
    ("RX_AEQ_VAL0_LANE1", 16),
    ("RX_AEQ_VAL0_LANE2", 16),
    ("RX_AEQ_VAL0_LANE3", 16),
    ("RX_AEQ_VAL1_LANE0", 16),
    ("RX_AEQ_VAL1_LANE1", 16),
    ("RX_AEQ_VAL1_LANE2", 16),
    ("RX_AEQ_VAL1_LANE3", 16),
    ("RX_AGC_CTRL_LANE0", 16),
    ("RX_AGC_CTRL_LANE1", 16),
    ("RX_AGC_CTRL_LANE2", 16),
    ("RX_AGC_CTRL_LANE3", 16),
    ("RX_CDR_CTRL0_LANE0", 16),
    ("RX_CDR_CTRL0_LANE1", 16),
    ("RX_CDR_CTRL0_LANE2", 16),
    ("RX_CDR_CTRL0_LANE3", 16),
    ("RX_CDR_CTRL1_LANE0", 16),
    ("RX_CDR_CTRL1_LANE1", 16),
    ("RX_CDR_CTRL1_LANE2", 16),
    ("RX_CDR_CTRL1_LANE3", 16),
    ("RX_CDR_CTRL2_LANE0", 16),
    ("RX_CDR_CTRL2_LANE1", 16),
    ("RX_CDR_CTRL2_LANE2", 16),
    ("RX_CDR_CTRL2_LANE3", 16),
    ("RX_CFG0_LANE0", 16),
    ("RX_CFG0_LANE1", 16),
    ("RX_CFG0_LANE2", 16),
    ("RX_CFG0_LANE3", 16),
    ("RX_CFG1_LANE0", 16),
    ("RX_CFG1_LANE1", 16),
    ("RX_CFG1_LANE2", 16),
    ("RX_CFG1_LANE3", 16),
    ("RX_CFG2_LANE0", 16),
    ("RX_CFG2_LANE1", 16),
    ("RX_CFG2_LANE2", 16),
    ("RX_CFG2_LANE3", 16),
    ("RX_CTLE_CTRL_LANE0", 16),
    ("RX_CTLE_CTRL_LANE1", 16),
    ("RX_CTLE_CTRL_LANE2", 16),
    ("RX_CTLE_CTRL_LANE3", 16),
    ("RX_CTRL_OVRD_LANE0", 16),
    ("RX_CTRL_OVRD_LANE1", 16),
    ("RX_CTRL_OVRD_LANE2", 16),
    ("RX_CTRL_OVRD_LANE3", 16),
    ("RX_LOOP_CTRL_LANE0", 16),
    ("RX_LOOP_CTRL_LANE1", 16),
    ("RX_LOOP_CTRL_LANE2", 16),
    ("RX_LOOP_CTRL_LANE3", 16),
    ("RX_MVAL0_LANE0", 16),
    ("RX_MVAL0_LANE1", 16),
    ("RX_MVAL0_LANE2", 16),
    ("RX_MVAL0_LANE3", 16),
    ("RX_MVAL1_LANE0", 16),
    ("RX_MVAL1_LANE1", 16),
    ("RX_MVAL1_LANE2", 16),
    ("RX_MVAL1_LANE3", 16),
    ("RX_P0S_CTRL", 16),
    ("RX_P0_CTRL", 16),
    ("RX_P1_CTRL", 16),
    ("RX_P2_CTRL", 16),
    ("RX_PI_CTRL0", 16),
    ("RX_PI_CTRL1", 16),
    ("SLICE_CFG", 16),
    ("SLICE_NOISE_CTRL_0_LANE01", 16),
    ("SLICE_NOISE_CTRL_0_LANE23", 16),
    ("SLICE_NOISE_CTRL_1_LANE01", 16),
    ("SLICE_NOISE_CTRL_1_LANE23", 16),
    ("SLICE_NOISE_CTRL_2_LANE01", 16),
    ("SLICE_NOISE_CTRL_2_LANE23", 16),
    ("SLICE_TX_RESET_LANE01", 16),
    ("SLICE_TX_RESET_LANE23", 16),
    ("TERM_CTRL_LANE0", 16),
    ("TERM_CTRL_LANE1", 16),
    ("TERM_CTRL_LANE2", 16),
    ("TERM_CTRL_LANE3", 16),
    ("TX_CFG0_LANE0", 16),
    ("TX_CFG0_LANE1", 16),
    ("TX_CFG0_LANE2", 16),
    ("TX_CFG0_LANE3", 16),
    ("TX_CFG1_LANE0", 16),
    ("TX_CFG1_LANE1", 16),
    ("TX_CFG1_LANE2", 16),
    ("TX_CFG1_LANE3", 16),
    ("TX_CFG2_LANE0", 16),
    ("TX_CFG2_LANE1", 16),
    ("TX_CFG2_LANE2", 16),
    ("TX_CFG2_LANE3", 16),
    ("TX_CLK_SEL0_LANE0", 16),
    ("TX_CLK_SEL0_LANE1", 16),
    ("TX_CLK_SEL0_LANE2", 16),
    ("TX_CLK_SEL0_LANE3", 16),
    ("TX_CLK_SEL1_LANE0", 16),
    ("TX_CLK_SEL1_LANE1", 16),
    ("TX_CLK_SEL1_LANE2", 16),
    ("TX_CLK_SEL1_LANE3", 16),
    ("TX_DISABLE_LANE0", 16),
    ("TX_DISABLE_LANE1", 16),
    ("TX_DISABLE_LANE2", 16),
    ("TX_DISABLE_LANE3", 16),
    ("TX_P0P0S_CTRL", 16),
    ("TX_P1P2_CTRL", 16),
    ("TX_PREEMPH_LANE0", 16),
    ("TX_PREEMPH_LANE1", 16),
    ("TX_PREEMPH_LANE2", 16),
    ("TX_PREEMPH_LANE3", 16),
    ("TX_PWR_RATE_OVRD_LANE0", 16),
    ("TX_PWR_RATE_OVRD_LANE1", 16),
    ("TX_PWR_RATE_OVRD_LANE2", 16),
    ("TX_PWR_RATE_OVRD_LANE3", 16),
];

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let Some(ctx) = FuzzCtx::try_new(session, backend, "GTH", "GTH", TileBits::MainAuto) else {
        return;
    };
    let extras = vec![ExtraFeature::new(
        ExtraFeatureKind::Hclk(0, 0),
        "HCLK",
        "HCLK",
        "DRP_MASK_BOTH",
        "GTH",
    )];
    fuzz_one_extras!(ctx, "ENABLE", "1", [
    ], [
        (mode "GTHE1_QUAD")
    ], extras);
    for &pin in GTH_INVPINS {
        fuzz_inv!(ctx, pin, [
            (mode "GTHE1_QUAD")
        ]);
    }
    for &(attr, vals) in GTH_ENUM_ATTRS {
        fuzz_enum!(ctx, attr, vals.iter().copied(), [
            (mode "GTHE1_QUAD")
        ]);
    }
    for &(attr, width) in GTH_BIN_ATTRS {
        fuzz_multi_attr_bin!(ctx, attr, width, [
            (mode "GTHE1_QUAD")
        ]);
    }
    for &(attr, width) in GTH_HEX_ATTRS {
        fuzz_multi_attr_hex!(ctx, attr, width, [
            (mode "GTHE1_QUAD")
        ]);
    }

    for pin in ["GREFCLK", "REFCLK_IN", "REFCLK_SOUTH", "REFCLK_NORTH"] {
        fuzz_one!(ctx, "MUX.REFCLK", pin, [
            (mode "GTHE1_QUAD"),
            (mutex "MUX.REFCLK", pin),
            (attr "PLL_CFG2", "")
        ], [
            (pip (pin pin), (pin_far "REFCLK"))
        ]);
    }

    let ctx = FuzzCtx::new(session, backend, "GTH", "IBUFDS_GTH", TileBits::Null);
    fuzz_one!(ctx, "ENABLE", "1", [
    ], [
        (mode "IBUFDS_GTHE1")
    ]);
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "GTH";
    if !ctx.has_tile(tile) {
        return;
    }
    let bel = "GTH";
    fn drp_bit(idx: usize, bit: usize) -> FeatureBit {
        let tile = idx >> 3;
        let frame = 28 + (bit & 1);
        let bit = (bit >> 1) | (idx & 7) << 3;
        FeatureBit::new(tile, frame, bit)
    }
    for addr in 0..0x140 {
        ctx.tiledb.insert(
            tile,
            bel,
            format!("DRP{addr:03X}"),
            TileItem::from_bitvec((0..16).map(|bit| drp_bit(addr, bit)).collect(), false),
        );
    }
    ctx.collect_bit(tile, bel, "ENABLE", "1");
    for &pin in GTH_INVPINS {
        ctx.collect_inv(tile, bel, pin);
    }
    for &(attr, vals) in GTH_ENUM_ATTRS {
        if attr.contains("X_FABRIC_WIDTH") {
            let mut diffs = vec![];
            for (val, sval) in [
                ("8_10_16_20", "8"),
                ("8_10_16_20", "10"),
                ("8_10_16_20", "16"),
                ("8_10_16_20", "20"),
                ("32", "32"),
                ("40", "40"),
                ("64", "64"),
                ("80", "80"),
                ("6466", "6466"),
            ] {
                diffs.push((val, ctx.state.get_diff(tile, bel, attr, sval)));
            }
            ctx.tiledb.insert(tile, bel, attr, xlat_enum(diffs));
        } else {
            ctx.collect_enum(tile, bel, attr, vals);
        }
    }
    for &(attr, _) in GTH_BIN_ATTRS {
        ctx.collect_bitvec(tile, bel, attr, "");
    }
    for &(attr, _) in GTH_HEX_ATTRS {
        let mut diffs = ctx.state.get_diffs(tile, bel, attr, "");
        if attr == "SLICE_NOISE_CTRL_1_LANE01" {
            let bit = FeatureBit::new(12, 29, 32);
            assert_eq!(diffs[1].bits.len(), 0);
            assert_eq!(diffs[2].bits.len(), 2);
            diffs[1].bits.insert(bit, true);
            assert_eq!(diffs[2].bits.remove(&bit), Some(true));
        }
        ctx.tiledb.insert(tile, bel, attr, xlat_bitvec(diffs));
    }
    ctx.collect_enum(
        tile,
        bel,
        "MUX.REFCLK",
        &["GREFCLK", "REFCLK_IN", "REFCLK_SOUTH", "REFCLK_NORTH"],
    );

    let tile = "HCLK";
    let bel = "HCLK";
    let mut diff = ctx.state.get_diff(tile, bel, "DRP_MASK_BOTH", "GTH");
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DRP_MASK_BELOW"), true, false);
    diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "DRP_MASK_ABOVE"), true, false);
    diff.assert_empty();
}