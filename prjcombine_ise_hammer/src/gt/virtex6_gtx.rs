use core::ops::Range;

use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::{TileBit, TileItem};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{xlat_bit, xlat_enum, CollectorCtx, Diff},
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_multi_attr_hex, fuzz_one,
    fuzz_one_extras,
};

const GTX_INVPINS: &[&str] = &[
    "DCLK",
    "RXUSRCLK",
    "RXUSRCLK2",
    "TXUSRCLK",
    "TXUSRCLK2",
    "TSTCLK0",
    "TSTCLK1",
    "SCANCLK",
    "GREFCLKRX",
    "GREFCLKTX",
];

const GTX_BOOL_ATTRS: &[&str] = &[
    "AC_CAP_DIS",
    "CHAN_BOND_KEEP_ALIGN",
    "CHAN_BOND_SEQ_2_USE",
    "CLK_COR_INSERT_IDLE_FLAG",
    "CLK_COR_KEEP_IDLE",
    "CLK_COR_PRECEDENCE",
    "CLK_CORRECT_USE",
    "CLK_COR_SEQ_2_USE",
    "COMMA_DOUBLE",
    "DEC_MCOMMA_DETECT",
    "DEC_PCOMMA_DETECT",
    "DEC_VALID_COMMA_ONLY",
    "DFE_DRP_EN",
    "GEN_RXUSRCLK",
    "GEN_TXUSRCLK",
    "GTX_CFG_PWRUP",
    "LOOPBACK_DRP_EN",
    "MASTER_DRP_EN",
    "MCOMMA_DETECT",
    "PCI_EXPRESS_MODE",
    "PCOMMA_DETECT",
    "PDELIDLE_DRP_EN",
    "PHASEALIGN_DRP_EN",
    "PLL_DRP_EN",
    "POLARITY_DRP_EN",
    "PRBS_DRP_EN",
    "RCV_TERM_GND",
    "RCV_TERM_VTTRX",
    "RESET_DRP_EN",
    "RX_BUFFER_USE",
    "RXBUF_OVRD_THRESH",
    "RX_CDR_FORCE_ROTATE",
    "RX_DECODE_SEQ_MATCH",
    "RX_EN_IDLE_HOLD_CDR",
    "RX_EN_IDLE_HOLD_DFE",
    "RX_EN_IDLE_RESET_BUF",
    "RX_EN_IDLE_RESET_FR",
    "RX_EN_IDLE_RESET_PH",
    "RX_EN_MODE_RESET_BUF",
    "RX_EN_RATE_RESET_BUF",
    "RX_EN_REALIGN_RESET_BUF2",
    "RX_EN_REALIGN_RESET_BUF",
    "RXGEARBOX_USE",
    "RX_LOSS_OF_SYNC_FSM",
    "RX_OVERSAMPLE_MODE",
    "RXPLL_STARTUP_EN",
    "SHOW_REALIGN_COMMA",
    "TERMINATION_OVRD",
    "TX_BUFFER_USE",
    "TXDRIVE_DRP_EN",
    "TXDRIVE_LOOPBACK_HIZ",
    "TXDRIVE_LOOPBACK_PD",
    "TX_EN_RATE_RESET_BUF",
    "TXGEARBOX_USE",
    "TX_OVERSAMPLE_MODE",
    "TXPLL_STARTUP_EN",
];

const GTX_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("ALIGN_COMMA_WORD", &["1", "2"]),
    ("CHAN_BOND_SEQ_LEN", &["4", "1", "2", "3"]),
    ("CLK_COR_ADJ_LEN", &["1", "2", "3", "4"]),
    ("CLK_COR_DET_LEN", &["1", "2", "3", "4"]),
    ("RX_DATA_WIDTH", &["8", "10", "16", "20", "32", "40"]),
    ("RX_FIFO_ADDR_MODE", &["FULL", "FAST"]),
    (
        "RX_LOS_INVALID_INCR",
        &["1", "2", "4", "8", "16", "32", "64", "128"],
    ),
    (
        "RX_LOS_THRESHOLD",
        &["4", "8", "16", "32", "64", "128", "256", "512"],
    ),
    ("RXPLL_DIVSEL45_FB", &["4", "5"]),
    (
        "RXPLL_DIVSEL_FB",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("RXPLL_DIVSEL_OUT", &["1", "2", "4"]),
    (
        "RXPLL_DIVSEL_REF",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    (
        "RXRECCLK_CTRL",
        &[
            "RXRECCLKPCS",
            "CLKTESTSIG1",
            "OFF_HIGH",
            "OFF_LOW",
            "RXPLLREFCLK_DIV1",
            "RXPLLREFCLK_DIV2",
            "RXRECCLKPMA_DIV1",
            "RXRECCLKPMA_DIV2",
        ],
    ),
    ("RX_SLIDE_MODE", &["#OFF", "AUTO", "PCS", "PMA"]),
    ("RX_XCLK_SEL", &["RXREC", "RXUSR"]),
    ("TX_CLK_SOURCE", &["TXPLL", "RXPLL"]),
    ("TX_DATA_WIDTH", &["8", "10", "16", "20", "32", "40"]),
    ("TX_DRIVE_MODE", &["DIRECT", "PIPE"]),
    (
        "TXOUTCLK_CTRL",
        &[
            "TXOUTCLKPCS",
            "CLKTESTSIG0",
            "OFF_HIGH",
            "OFF_LOW",
            "TXOUTCLKPMA_DIV1",
            "TXOUTCLKPMA_DIV2",
            "TXPLLREFCLK_DIV1",
            "TXPLLREFCLK_DIV2",
        ],
    ),
    ("TXPLL_DIVSEL45_FB", &["4", "5"]),
    (
        "TXPLL_DIVSEL_FB",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("TXPLL_DIVSEL_OUT", &["1", "2", "4"]),
    (
        "TXPLL_DIVSEL_REF",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("TX_XCLK_SEL", &["TXUSR", "TXOUT"]),
    (
        "RX_CLK25_DIVIDER",
        &[
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30",
            "31", "32",
        ],
    ),
    (
        "TX_CLK25_DIVIDER",
        &[
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
            "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30",
            "31", "32",
        ],
    ),
];

const GTX_ENUM_INT_ATTRS: &[(&str, Range<u32>)] = &[
    ("CHAN_BOND_1_MAX_SKEW", 1..15),
    ("CHAN_BOND_2_MAX_SKEW", 1..15),
    ("CLK_COR_MAX_LAT", 3..49),
    ("CLK_COR_MIN_LAT", 3..49),
    ("SAS_MAX_COMSAS", 1..62),
    ("SAS_MIN_COMSAS", 1..62),
    ("SATA_MAX_BURST", 1..62),
    ("SATA_MAX_INIT", 1..62),
    ("SATA_MAX_WAKE", 1..62),
    ("SATA_MIN_BURST", 1..62),
    ("SATA_MIN_INIT", 1..62),
    ("SATA_MIN_WAKE", 1..62),
];

const GTX_DEC_ATTRS: &[(&str, usize)] = &[
    ("CLK_COR_REPEAT_WAIT", 5),
    ("RXBUF_OVFL_THRESH", 6),
    ("RXBUF_UDFL_THRESH", 6),
    ("RX_SLIDE_AUTO_WAIT", 4),
    ("TXOUTCLKPCS_SEL", 1),
];

const GTX_BIN_ATTRS: &[(&str, usize)] = &[
    ("A_DFECLKDLYADJ", 6),
    ("A_DFEDLYOVRD", 1),
    ("A_DFETAP1", 5),
    ("A_DFETAP2", 5),
    ("A_DFETAP3", 4),
    ("A_DFETAP4", 4),
    ("A_DFETAPOVRD", 1),
    ("A_GTXRXRESET", 1),
    ("A_GTXTXRESET", 1),
    ("A_LOOPBACK", 3),
    ("A_PLLCLKRXRESET", 1),
    ("A_PLLCLKTXRESET", 1),
    ("A_PLLRXRESET", 1),
    ("A_PLLTXRESET", 1),
    ("A_PRBSCNTRESET", 1),
    ("A_RXBUFRESET", 1),
    ("A_RXCDRFREQRESET", 1),
    ("A_RXCDRHOLD", 1),
    ("A_RXCDRPHASERESET", 1),
    ("A_RXCDRRESET", 1),
    ("A_RXDFERESET", 1),
    ("A_RXENPMAPHASEALIGN", 1),
    ("A_RXENPRBSTST", 3),
    ("A_RXENSAMPLEALIGN", 1),
    ("A_RXEQMIX", 10),
    ("A_RXPLLLKDETEN", 1),
    ("A_RXPLLPOWERDOWN", 1),
    ("A_RXPMASETPHASE", 1),
    ("A_RXPOLARITY", 1),
    ("A_RXPOWERDOWN", 2),
    ("A_RXRESET", 1),
    ("A_TXBUFDIFFCTRL", 3),
    ("A_TXDEEMPH", 1),
    ("A_TXDIFFCTRL", 4),
    ("A_TXELECIDLE", 1),
    ("A_TXENPMAPHASEALIGN", 1),
    ("A_TXENPRBSTST", 3),
    ("A_TXMARGIN", 3),
    ("A_TXPLLLKDETEN", 1),
    ("A_TXPLLPOWERDOWN", 1),
    ("A_TXPMASETPHASE", 1),
    ("A_TXPOLARITY", 1),
    ("A_TXPOSTEMPHASIS", 5),
    ("A_TXPOWERDOWN", 2),
    ("A_TXPRBSFORCEERR", 1),
    ("A_TXPREEMPHASIS", 4),
    ("A_TXRESET", 1),
    ("A_TXSWING", 1),
    ("BGTEST_CFG", 2),
    ("CDR_PH_ADJ_TIME", 5),
    ("CHAN_BOND_SEQ_1_1", 10),
    ("CHAN_BOND_SEQ_1_2", 10),
    ("CHAN_BOND_SEQ_1_3", 10),
    ("CHAN_BOND_SEQ_1_4", 10),
    ("CHAN_BOND_SEQ_1_ENABLE", 4),
    ("CHAN_BOND_SEQ_2_1", 10),
    ("CHAN_BOND_SEQ_2_2", 10),
    ("CHAN_BOND_SEQ_2_3", 10),
    ("CHAN_BOND_SEQ_2_4", 10),
    ("CHAN_BOND_SEQ_2_CFG", 5),
    ("CHAN_BOND_SEQ_2_ENABLE", 4),
    ("CLK_COR_SEQ_1_1", 10),
    ("CLK_COR_SEQ_1_2", 10),
    ("CLK_COR_SEQ_1_3", 10),
    ("CLK_COR_SEQ_1_4", 10),
    ("CLK_COR_SEQ_1_ENABLE", 4),
    ("CLK_COR_SEQ_2_1", 10),
    ("CLK_COR_SEQ_2_2", 10),
    ("CLK_COR_SEQ_2_3", 10),
    ("CLK_COR_SEQ_2_4", 10),
    ("CLK_COR_SEQ_2_ENABLE", 4),
    ("CM_TRIM", 2),
    ("COMMA_10B_ENABLE", 10),
    ("COM_BURST_VAL", 4),
    ("DFE_CAL_TIME", 5),
    ("DFE_CFG", 8),
    ("GEARBOX_ENDEC", 3),
    ("MCOMMA_10B_VALUE", 10),
    ("OOBDETECT_THRESHOLD", 3),
    ("PCOMMA_10B_VALUE", 10),
    ("POWER_SAVE", 10),
    ("RXPLL_LKDET_CFG", 3),
    ("RXPRBSERR_LOOPBACK", 1),
    ("RXRECCLK_DLY", 10),
    ("RX_DLYALIGN_CTRINC", 4),
    ("RX_DLYALIGN_EDGESET", 5),
    ("RX_DLYALIGN_LPFINC", 4),
    ("RX_DLYALIGN_MONSEL", 3),
    ("RX_DLYALIGN_OVRDSETTING", 8),
    ("RX_EYE_SCANMODE", 2),
    ("RX_IDLE_HI_CNT", 4),
    ("RX_IDLE_LO_CNT", 4),
    ("SATA_BURST_VAL", 3),
    ("SATA_IDLE_VAL", 3),
    ("TERMINATION_CTRL", 5),
    ("TXOUTCLK_DLY", 10),
    ("TXPLL_LKDET_CFG", 3),
    ("TXPLL_SATA", 2),
    ("TX_DEEMPH_0", 5),
    ("TX_DEEMPH_1", 5),
    ("TX_DLYALIGN_CTRINC", 4),
    ("TX_DLYALIGN_LPFINC", 4),
    ("TX_DLYALIGN_MONSEL", 3),
    ("TX_DLYALIGN_OVRDSETTING", 8),
    ("TX_IDLE_ASSERT_DELAY", 3),
    ("TX_IDLE_DEASSERT_DELAY", 3),
    ("TX_MARGIN_FULL_0", 7),
    ("TX_MARGIN_FULL_1", 7),
    ("TX_MARGIN_FULL_2", 7),
    ("TX_MARGIN_FULL_3", 7),
    ("TX_MARGIN_FULL_4", 7),
    ("TX_MARGIN_LOW_0", 7),
    ("TX_MARGIN_LOW_1", 7),
    ("TX_MARGIN_LOW_2", 7),
    ("TX_MARGIN_LOW_3", 7),
    ("TX_MARGIN_LOW_4", 7),
    ("TX_PMADATA_OPT", 1),
    ("TX_TDCC_CFG", 2),
    ("USR_CODE_ERR_CLR", 1),
];

const GTX_HEX_ATTRS: &[(&str, usize)] = &[
    ("BIAS_CFG", 17),
    ("PMA_CDR_SCAN", 27),
    ("PMA_CFG", 76),
    ("PMA_RXSYNC_CFG", 7),
    ("PMA_RX_CFG", 25),
    ("PMA_TX_CFG", 20),
    ("RXPLL_COM_CFG", 24),
    ("RXPLL_CP_CFG", 8),
    ("RXUSRCLK_DLY", 16),
    ("RX_EYE_OFFSET", 8),
    ("TRANS_TIME_FROM_P2", 12),
    ("TRANS_TIME_NON_P2", 8),
    ("TRANS_TIME_RATE", 8),
    ("TRANS_TIME_TO_P2", 10),
    ("TST_ATTR", 32),
    ("TXPLL_COM_CFG", 24),
    ("TXPLL_CP_CFG", 8),
    ("TX_BYTECLK_CFG", 6),
    ("TX_DETECT_RX_CFG", 14),
    ("TX_USRCLK_CFG", 6),
];

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for i in 0..4 {
        let Some(ctx) = FuzzCtx::try_new(
            session,
            backend,
            "GTX",
            format!("GTX{i}"),
            TileBits::MainAuto,
        ) else {
            return;
        };
        let bel_other = BelId::from_idx(20 + (i ^ 1));
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::Hclk(0, 0),
            "HCLK",
            "HCLK",
            if i < 2 {
                "DRP_MASK_BELOW"
            } else {
                "DRP_MASK_ABOVE"
            },
            "GTX",
        )];
        fuzz_one_extras!(ctx, "GTX_CFG_PWRUP", "1", [
            (bel_unused bel_other)
        ], [
            (mode "GTXE1")
        ], extras);
        for &pin in GTX_INVPINS {
            fuzz_inv!(ctx, pin, [
                (mode "GTXE1")
            ]);
        }
        for &attr in GTX_BOOL_ATTRS {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (mode "GTXE1")
            ]);
        }
        for &(attr, vals) in GTX_ENUM_ATTRS {
            fuzz_enum!(ctx, attr, vals.iter().copied(), [
                (mode "GTXE1")
            ]);
        }
        for &(attr, ref vals) in GTX_ENUM_INT_ATTRS {
            let vals = Vec::from_iter(vals.clone().map(|i| i.to_string()));
            fuzz_enum!(ctx, attr, vals.iter(), [
                (mode "GTXE1")
            ]);
        }
        for &(attr, width) in GTX_DEC_ATTRS {
            fuzz_multi_attr_dec!(ctx, attr, width, [
                (mode "GTXE1")
            ]);
        }
        for &(attr, width) in GTX_BIN_ATTRS {
            fuzz_multi_attr_bin!(ctx, attr, width, [
                (mode "GTXE1")
            ]);
        }
        for &(attr, width) in GTX_HEX_ATTRS {
            fuzz_multi_attr_hex!(ctx, attr, width, [
                (mode "GTXE1")
            ]);
        }

        let bel_hclk_gtx = BelId::from_idx(26);
        for (val, orx, otx, pin) in [
            ("PERFCLK", "PERFCLKRX", "PERFCLKTX", "PERFCLK"),
            (
                "MGTREFCLK0",
                "MGTREFCLKRX0",
                "MGTREFCLKTX0",
                "MGTREFCLKOUT0",
            ),
            (
                "MGTREFCLK1",
                "MGTREFCLKRX1",
                "MGTREFCLKTX1",
                "MGTREFCLKOUT1",
            ),
            (
                "SOUTHREFCLK0",
                "SOUTHREFCLKRX0",
                "SOUTHREFCLKTX0",
                "SOUTHREFCLKOUT0",
            ),
            (
                "SOUTHREFCLK1",
                "SOUTHREFCLKRX1",
                "SOUTHREFCLKTX1",
                "SOUTHREFCLKOUT1",
            ),
            (
                "NORTHREFCLK0",
                "NORTHREFCLKRX0",
                "NORTHREFCLKTX0",
                "NORTHREFCLKIN0",
            ),
            (
                "NORTHREFCLK1",
                "NORTHREFCLKRX1",
                "NORTHREFCLKTX1",
                "NORTHREFCLKIN1",
            ),
        ] {
            fuzz_one!(ctx, "RXPLLREFSEL_STATIC", val, [
                (tile_mutex "PERFCLK", "USE"),
                (mutex "RXPLLREFSEL", val),
                (pip (bel_pin bel_hclk_gtx, "PERF0"), (bel_pin bel_hclk_gtx, "PERFCLK"))
            ], [
                (pip (pin pin), (pin orx))
            ]);
            fuzz_one!(ctx, "TXPLLREFSEL_STATIC", val, [
                (tile_mutex "PERFCLK", "USE"),
                (mutex "TXPLLREFSEL", val),
                (pip (bel_pin bel_hclk_gtx, "PERF0"), (bel_pin bel_hclk_gtx, "PERFCLK"))
            ], [
                (pip (pin pin), (pin otx))
            ]);
        }
        fuzz_one!(ctx, "PMA_CAS_CLK_EN", "TRUE", [
            (mode "GTXE1"),
            (mutex "RXPLLREFSEL", "CAS_CLK"),
            (mutex "TXPLLREFSEL", "CAS_CLK")
        ], [
            (attr "PMA_CAS_CLK_EN", "TRUE")
        ]);
        fuzz_one!(ctx, "RXPLLREFSEL_STATIC", "GREFCLK", [
            (mutex "RXPLLREFSEL", "GREFCLK")
        ], [
            (pip (pin_far "GREFCLKRX"), (pin "GREFCLKRX"))
        ]);
        fuzz_one!(ctx, "TXPLLREFSEL_STATIC", "GREFCLK", [
            (mutex "TXPLLREFSEL", "GREFCLK")
        ], [
            (pip (pin_far "GREFCLKTX"), (pin "GREFCLKTX"))
        ]);
        fuzz_one!(ctx, "RXPLLREFSEL_MODE", "DYNAMIC", [
            (mutex "RXPLLREFSEL", "MODE"),
            (pip (pin_far "GREFCLKRX"), (pin "GREFCLKRX"))
        ], [
            (pip (pin "MGTREFCLKOUT0"), (pin "MGTREFCLKRX0"))
        ]);
        fuzz_one!(ctx, "TXPLLREFSEL_MODE", "DYNAMIC", [
            (mutex "TXPLLREFSEL", "MODE"),
            (pip (pin_far "GREFCLKTX"), (pin "GREFCLKTX"))
        ], [
            (pip (pin "MGTREFCLKOUT0"), (pin "MGTREFCLKTX0"))
        ]);
    }
    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "GTX",
            format!("IBUFDS_GTX{i}"),
            TileBits::Mgt,
        );
        fuzz_enum!(ctx, "CLKCM_CFG", ["FALSE", "TRUE"], [(mode "IBUFDS_GTXE1")]);
        fuzz_enum!(ctx, "CLKRCV_TRST", ["FALSE", "TRUE"], [(mode "IBUFDS_GTXE1")]);
        fuzz_multi_attr_bin!(ctx, "REFCLKOUT_DLY", 10, [(mode "IBUFDS_GTXE1")]);
        for (val, pin) in [
            ("O", "O"),
            ("ODIV2", "ODIV2"),
            ("CLKTESTSIG", "CLKTESTSIG_INT"),
        ] {
            fuzz_one!(ctx, "MUX.HCLK_OUT", val, [
                (mode "IBUFDS_GTXE1"),
                (mutex "MUX.HCLK_OUT", val)
            ], [
                (pip (pin pin), (pin "HCLK_OUT"))
            ]);
        }
    }
    let ctx = FuzzCtx::new(session, backend, "GTX", "HCLK_GTX", TileBits::Mgt);
    for i in 0..4 {
        fuzz_one!(ctx, "MUX.PERFCLK", format!("PERF{i}"), [
            (tile_mutex "PERFCLK", format!("PERF{i}"))
        ], [
            (pip (pin format!("PERF{i}")), (pin "PERFCLK"))
        ]);
    }
    for i in 0..2 {
        for j in 0..2 {
            fuzz_one!(ctx, format!("MUX.SOUTHREFCLKOUT{i}"), format!("MGTREFCLKIN{j}"), [
                (mutex format!("MUX.SOUTHREFCLKOUT{i}"), format!("MGTREFCLKIN{j}"))
            ], [
                (pip (pin format!("MGTREFCLKIN{j}")), (pin format!("SOUTHREFCLKOUT{i}")))
            ]);
        }
        fuzz_one!(ctx, format!("MUX.SOUTHREFCLKOUT{i}"), format!("SOUTHREFCLKIN{i}"), [
            (mutex format!("MUX.SOUTHREFCLKOUT{i}"), format!("SOUTHREFCLKIN{i}"))
        ], [
            (pip (pin format!("SOUTHREFCLKIN{i}")), (pin format!("SOUTHREFCLKOUT{i}")))
        ]);
        for j in 0..2 {
            fuzz_one!(ctx, format!("MUX.NORTHREFCLKOUT{i}"), format!("MGTREFCLKOUT{j}"), [
                (mutex format!("MUX.NORTHREFCLKOUT{i}"), format!("MGTREFCLKOUT{j}"))
            ], [
                (pip (pin format!("MGTREFCLKOUT{j}")), (pin format!("NORTHREFCLKOUT{i}")))
            ]);
        }
        fuzz_one!(ctx, format!("MUX.NORTHREFCLKOUT{i}"), format!("NORTHREFCLKIN{i}"), [
            (mutex format!("MUX.NORTHREFCLKOUT{i}"), format!("NORTHREFCLKIN{i}"))
        ], [
            (pip (pin format!("NORTHREFCLKIN{i}")), (pin format!("NORTHREFCLKOUT{i}")))
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "GTX";
    if !ctx.has_tile(tile) {
        return;
    }
    for i in 0..4 {
        let bel = &format!("GTX{i}");
        fn drp_bit(which: usize, idx: usize, bit: usize) -> TileBit {
            let tile = which * 10 + (idx >> 3);
            let frame = 28 + (bit & 1);
            let bit = (bit >> 1) | (idx & 7) << 3;
            TileBit::new(tile, frame, bit)
        }
        for addr in 0..0x50 {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("DRP{addr:02X}"),
                TileItem::from_bitvec((0..16).map(|bit| drp_bit(i, addr, bit)).collect(), false),
            );
        }

        ctx.collect_bit(tile, bel, "GTX_CFG_PWRUP", "1");
        for &pin in GTX_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for &attr in GTX_BOOL_ATTRS {
            if attr == "GTX_CFG_PWRUP" {
                ctx.state.get_diff(tile, bel, attr, "FALSE").assert_empty();
                ctx.state.get_diff(tile, bel, attr, "TRUE").assert_empty();
            } else {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            }
        }
        for &(attr, vals) in GTX_ENUM_ATTRS {
            ctx.collect_enum(tile, bel, attr, vals);
        }
        for &(attr, ref vals) in GTX_ENUM_INT_ATTRS {
            ctx.collect_enum_int(tile, bel, attr, vals.clone(), 0);
        }
        for &(attr, _) in GTX_DEC_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTX_BIN_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTX_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }

        let mut diff_cas_clk = ctx.state.get_diff(tile, bel, "PMA_CAS_CLK_EN", "TRUE");
        for rxtx in ["RX", "TX"] {
            let attr_static = &format!("{rxtx}PLLREFSEL_STATIC");
            let diff_grefclk = ctx.state.get_diff(tile, bel, attr_static, "GREFCLK");
            let diff_perfclk = ctx
                .state
                .get_diff(tile, bel, attr_static, "PERFCLK")
                .combine(&!&diff_grefclk);
            ctx.tiledb.insert(
                tile,
                bel,
                format!("{rxtx}PLLREFSEL_TESTCLK"),
                xlat_enum(vec![
                    ("GREFCLK", Diff::default()),
                    ("PERFCLK", diff_perfclk),
                ]),
            );
            let mut diffs = vec![];
            for val in [
                "MGTREFCLK0",
                "MGTREFCLK1",
                "NORTHREFCLK0",
                "NORTHREFCLK1",
                "SOUTHREFCLK0",
                "SOUTHREFCLK1",
            ] {
                diffs.push((val, ctx.state.get_diff(tile, bel, attr_static, val)))
            }
            diffs.push((
                "CAS_CLK",
                diff_cas_clk.split_bits(&diff_grefclk.bits.keys().copied().collect()),
            ));
            diffs.push(("TESTCLK", diff_grefclk));
            ctx.tiledb.insert(tile, bel, attr_static, xlat_enum(diffs));
            ctx.collect_enum_default(
                tile,
                bel,
                &format!("{rxtx}PLLREFSEL_MODE"),
                &["DYNAMIC"],
                "STATIC",
            );
        }
        ctx.tiledb
            .insert(tile, bel, "PMA_CAS_CLK_EN", xlat_bit(diff_cas_clk));
    }
    for i in 0..2 {
        let bel = &format!("IBUFDS_GTX{i}");
        ctx.collect_enum_bool(tile, bel, "CLKCM_CFG", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CLKRCV_TRST", "FALSE", "TRUE");
        ctx.collect_bitvec(tile, bel, "REFCLKOUT_DLY", "");
        ctx.collect_enum_default(
            tile,
            bel,
            "MUX.HCLK_OUT",
            &["O", "ODIV2", "CLKTESTSIG"],
            "NONE",
        );
    }
    let bel = "HCLK_GTX";
    ctx.collect_enum_default(
        tile,
        bel,
        "MUX.PERFCLK",
        &["PERF0", "PERF1", "PERF2", "PERF3"],
        "NONE",
    );
    ctx.collect_enum_default(
        tile,
        bel,
        "MUX.SOUTHREFCLKOUT0",
        &["SOUTHREFCLKIN0", "MGTREFCLKIN0", "MGTREFCLKIN1"],
        "NONE",
    );
    ctx.collect_enum_default(
        tile,
        bel,
        "MUX.SOUTHREFCLKOUT1",
        &["SOUTHREFCLKIN1", "MGTREFCLKIN0", "MGTREFCLKIN1"],
        "NONE",
    );
    ctx.collect_enum_default(
        tile,
        bel,
        "MUX.NORTHREFCLKOUT0",
        &["NORTHREFCLKIN0", "MGTREFCLKOUT0", "MGTREFCLKOUT1"],
        "NONE",
    );
    ctx.collect_enum_default(
        tile,
        bel,
        "MUX.NORTHREFCLKOUT1",
        &["NORTHREFCLKIN1", "MGTREFCLKOUT0", "MGTREFCLKOUT1"],
        "NONE",
    );
    let tile = "HCLK";
    let bel = "HCLK";
    ctx.collect_bit(tile, bel, "DRP_MASK_BELOW", "GTX");
    ctx.collect_bit(tile, bel, "DRP_MASK_ABOVE", "GTX");
}
