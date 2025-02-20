use core::ops::Range;

use prjcombine_hammer::Session;
use prjcombine_interconnect::db::BelId;
use prjcombine_types::tiledb::{TileBit, TileItem};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum, fuzz_inv,
    fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_multi_attr_hex, fuzz_one,
};

const GT_INVPINS: &[&str] = &[
    "DCLK",
    "RXUSRCLK0",
    "RXUSRCLK1",
    "RXUSRCLK20",
    "RXUSRCLK21",
    "TXUSRCLK0",
    "TXUSRCLK1",
    "TXUSRCLK20",
    "TXUSRCLK21",
];

const GTP_BOOL_ATTRS: &[&str] = &[
    "AC_CAP_DIS_0",
    "AC_CAP_DIS_1",
    "CHAN_BOND_SEQ_2_USE_0",
    "CHAN_BOND_SEQ_2_USE_1",
    "CLKINDC_B",
    "CLK_CORRECT_USE_0",
    "CLK_CORRECT_USE_1",
    "CLK_COR_KEEP_IDLE_0",
    "CLK_COR_KEEP_IDLE_1",
    "CLK_COR_INSERT_IDLE_FLAG_0",
    "CLK_COR_INSERT_IDLE_FLAG_1",
    "CLK_COR_PRECEDENCE_0",
    "CLK_COR_PRECEDENCE_1",
    "CLK_COR_SEQ_2_USE_0",
    "CLK_COR_SEQ_2_USE_1",
    "COMMA_DOUBLE_0",
    "COMMA_DOUBLE_1",
    "DEC_MCOMMA_DETECT_0",
    "DEC_MCOMMA_DETECT_1",
    "DEC_PCOMMA_DETECT_0",
    "DEC_PCOMMA_DETECT_1",
    "DEC_VALID_COMMA_ONLY_0",
    "DEC_VALID_COMMA_ONLY_1",
    "MCOMMA_DETECT_0",
    "MCOMMA_DETECT_1",
    "OVERSAMPLE_MODE",
    "PCOMMA_DETECT_0",
    "PCOMMA_DETECT_1",
    "PCI_EXPRESS_MODE_0",
    "PCI_EXPRESS_MODE_1",
    "PLL_SATA_0",
    "PLL_SATA_1",
    "PLL_STARTUP_EN",
    "RCV_TERM_GND_0",
    "RCV_TERM_GND_1",
    "RCV_TERM_MID_0",
    "RCV_TERM_MID_1",
    "RCV_TERM_VTTRX_0",
    "RCV_TERM_VTTRX_1",
    "RX_BUFFER_USE_0",
    "RX_BUFFER_USE_1",
    "RX_CDR_FORCE_ROTATE_0",
    "RX_CDR_FORCE_ROTATE_1",
    "RX_DECODE_SEQ_MATCH_0",
    "RX_DECODE_SEQ_MATCH_1",
    "RX_LOSS_OF_SYNC_FSM_0",
    "RX_LOSS_OF_SYNC_FSM_1",
    "SYS_CLK_EN",
    "TERMINATION_OVRD",
    "TX_BUFFER_USE_0",
    "TX_BUFFER_USE_1",
    "TX_DIFF_BOOST_0",
    "TX_DIFF_BOOST_1",
];

const GTP_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("ALIGN_COMMA_WORD_0", &["1", "2"]),
    ("ALIGN_COMMA_WORD_1", &["1", "2"]),
    ("CHAN_BOND_MODE_0", &["SLAVE", "MASTER", "#OFF"]),
    ("CHAN_BOND_MODE_1", &["SLAVE", "MASTER", "#OFF"]),
    ("CHAN_BOND_SEQ_LEN_0", &["1", "2", "3", "4"]),
    ("CHAN_BOND_SEQ_LEN_1", &["1", "2", "3", "4"]),
    ("CLK25_DIVIDER", &["1", "2", "3", "4", "5", "6", "10", "12"]),
    ("CLK_COR_ADJ_LEN_0", &["1", "2", "3", "4"]),
    ("CLK_COR_ADJ_LEN_1", &["1", "2", "3", "4"]),
    ("CLK_COR_DET_LEN_0", &["1", "2", "3", "4"]),
    ("CLK_COR_DET_LEN_1", &["1", "2", "3", "4"]),
    (
        "OOB_CLK_DIVIDER",
        &["1", "2", "4", "6", "8", "10", "12", "14"],
    ),
    ("PLL_DIVSEL_FB", &["1", "2", "3", "4", "5", "8", "10"]),
    (
        "PLL_DIVSEL_REF",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("PLL_RXDIVSEL_OUT_0", &["1", "2", "4"]),
    ("PLL_RXDIVSEL_OUT_1", &["1", "2", "4"]),
    ("PLL_TXDIVSEL_COMM_OUT", &["1", "2", "4"]),
    ("PLL_TXDIVSEL_OUT_0", &["1", "2", "4"]),
    ("PLL_TXDIVSEL_OUT_1", &["1", "2", "4"]),
    (
        "RX_LOS_INVALID_INCR_0",
        &["1", "2", "4", "8", "16", "32", "64", "128"],
    ),
    (
        "RX_LOS_INVALID_INCR_1",
        &["1", "2", "4", "8", "16", "32", "64", "128"],
    ),
    (
        "RX_LOS_THRESHOLD_0",
        &["4", "8", "16", "32", "64", "128", "256", "512"],
    ),
    (
        "RX_LOS_THRESHOLD_1",
        &["4", "8", "16", "32", "64", "128", "256", "512"],
    ),
    ("RX_SLIDE_MODE_0", &["PCS", "PMA"]),
    ("RX_SLIDE_MODE_1", &["PCS", "PMA"]),
    ("RX_STATUS_FMT_0", &["PCIE", "SATA"]),
    ("RX_STATUS_FMT_1", &["PCIE", "SATA"]),
    ("RX_XCLK_SEL_0", &["RXUSR", "RXREC"]),
    ("RX_XCLK_SEL_1", &["RXUSR", "RXREC"]),
    ("TERMINATION_IMP_0", &["50", "75"]),
    ("TERMINATION_IMP_1", &["50", "75"]),
    ("TX_XCLK_SEL_0", &["TXUSR", "TXOUT"]),
    ("TX_XCLK_SEL_1", &["TXUSR", "TXOUT"]),
];

const GTP_ENUM_INT_ATTRS: &[(&str, Range<u32>)] = &[
    ("CHAN_BOND_1_MAX_SKEW_0", 1..15),
    ("CHAN_BOND_1_MAX_SKEW_1", 1..15),
    ("CHAN_BOND_2_MAX_SKEW_0", 1..15),
    ("CHAN_BOND_2_MAX_SKEW_1", 1..15),
    ("CLK_COR_MAX_LAT_0", 3..49),
    ("CLK_COR_MAX_LAT_1", 3..49),
    ("CLK_COR_MIN_LAT_0", 3..49),
    ("CLK_COR_MIN_LAT_1", 3..49),
    ("SATA_MAX_BURST_0", 1..62),
    ("SATA_MAX_BURST_1", 1..62),
    ("SATA_MAX_INIT_0", 1..62),
    ("SATA_MAX_INIT_1", 1..62),
    ("SATA_MAX_WAKE_0", 1..62),
    ("SATA_MAX_WAKE_1", 1..62),
    ("SATA_MIN_BURST_0", 1..62),
    ("SATA_MIN_BURST_1", 1..62),
    ("SATA_MIN_INIT_0", 1..62),
    ("SATA_MIN_INIT_1", 1..62),
    ("SATA_MIN_WAKE_0", 1..62),
    ("SATA_MIN_WAKE_1", 1..62),
];

const GTP_DEC_ATTRS: &[(&str, usize)] = &[
    ("CHAN_BOND_LEVEL_0", 3),
    ("CHAN_BOND_LEVEL_1", 3),
    ("CLK_COR_REPEAT_WAIT_0", 5),
    ("CLK_COR_REPEAT_WAIT_1", 5),
    ("TXOUTCLK_SEL_0", 1),
    ("TXOUTCLK_SEL_1", 1),
    ("TX_SYNC_FILTERB", 1),
];

const GTP_BIN_ATTRS: &[(&str, usize)] = &[
    ("CHAN_BOND_SEQ_1_1_0", 10),
    ("CHAN_BOND_SEQ_1_1_1", 10),
    ("CHAN_BOND_SEQ_1_2_0", 10),
    ("CHAN_BOND_SEQ_1_2_1", 10),
    ("CHAN_BOND_SEQ_1_3_0", 10),
    ("CHAN_BOND_SEQ_1_3_1", 10),
    ("CHAN_BOND_SEQ_1_4_0", 10),
    ("CHAN_BOND_SEQ_1_4_1", 10),
    ("CHAN_BOND_SEQ_1_ENABLE_0", 4),
    ("CHAN_BOND_SEQ_1_ENABLE_1", 4),
    ("CHAN_BOND_SEQ_2_1_0", 10),
    ("CHAN_BOND_SEQ_2_1_1", 10),
    ("CHAN_BOND_SEQ_2_2_0", 10),
    ("CHAN_BOND_SEQ_2_2_1", 10),
    ("CHAN_BOND_SEQ_2_3_0", 10),
    ("CHAN_BOND_SEQ_2_3_1", 10),
    ("CHAN_BOND_SEQ_2_4_0", 10),
    ("CHAN_BOND_SEQ_2_4_1", 10),
    ("CHAN_BOND_SEQ_2_ENABLE_0", 4),
    ("CHAN_BOND_SEQ_2_ENABLE_1", 4),
    ("CLK_COR_SEQ_1_1_0", 10),
    ("CLK_COR_SEQ_1_1_1", 10),
    ("CLK_COR_SEQ_1_2_0", 10),
    ("CLK_COR_SEQ_1_2_1", 10),
    ("CLK_COR_SEQ_1_3_0", 10),
    ("CLK_COR_SEQ_1_3_1", 10),
    ("CLK_COR_SEQ_1_4_0", 10),
    ("CLK_COR_SEQ_1_4_1", 10),
    ("CLK_COR_SEQ_1_ENABLE_0", 4),
    ("CLK_COR_SEQ_1_ENABLE_1", 4),
    ("CLK_COR_SEQ_2_1_0", 10),
    ("CLK_COR_SEQ_2_1_1", 10),
    ("CLK_COR_SEQ_2_2_0", 10),
    ("CLK_COR_SEQ_2_2_1", 10),
    ("CLK_COR_SEQ_2_3_0", 10),
    ("CLK_COR_SEQ_2_3_1", 10),
    ("CLK_COR_SEQ_2_4_0", 10),
    ("CLK_COR_SEQ_2_4_1", 10),
    ("CLK_COR_SEQ_2_ENABLE_0", 4),
    ("CLK_COR_SEQ_2_ENABLE_1", 4),
    ("COMMA_10B_ENABLE_0", 10),
    ("COMMA_10B_ENABLE_1", 10),
    ("COM_BURST_VAL_0", 4),
    ("COM_BURST_VAL_1", 4),
    ("MCOMMA_10B_VALUE_0", 10),
    ("MCOMMA_10B_VALUE_1", 10),
    ("OOBDETECT_THRESHOLD_0", 3),
    ("OOBDETECT_THRESHOLD_1", 3),
    ("PCOMMA_10B_VALUE_0", 10),
    ("PCOMMA_10B_VALUE_1", 10),
    ("PLLLKDET_CFG", 3),
    ("SATA_BURST_VAL_0", 3),
    ("SATA_BURST_VAL_1", 3),
    ("SATA_IDLE_VAL_0", 3),
    ("SATA_IDLE_VAL_1", 3),
    ("TERMINATION_CTRL", 5),
    ("TXRX_INVERT_0", 5),
    ("TXRX_INVERT_1", 5),
];

const GTP_HEX_ATTRS: &[(&str, usize)] = &[
    ("PCS_COM_CFG", 28),
    ("PMA_CDR_SCAN_0", 27),
    ("PMA_CDR_SCAN_1", 27),
    ("PMA_COM_CFG", 90),
    ("PMA_RX_CFG_0", 25),
    ("PMA_RX_CFG_1", 25),
    ("PRBS_ERR_THRESHOLD_0", 32),
    ("PRBS_ERR_THRESHOLD_1", 32),
    ("TRANS_TIME_FROM_P2_0", 16),
    ("TRANS_TIME_FROM_P2_1", 16),
    ("TRANS_TIME_NON_P2_0", 16),
    ("TRANS_TIME_NON_P2_1", 16),
    ("TRANS_TIME_TO_P2_0", 16),
    ("TRANS_TIME_TO_P2_1", 16),
    ("TX_DETECT_RX_CFG_0", 14),
    ("TX_DETECT_RX_CFG_1", 14),
];

const GTX_BOOL_ATTRS: &[&str] = &[
    "AC_CAP_DIS_0",
    "AC_CAP_DIS_1",
    "CHAN_BOND_KEEP_ALIGN_0",
    "CHAN_BOND_KEEP_ALIGN_1",
    "CHAN_BOND_SEQ_2_USE_0",
    "CHAN_BOND_SEQ_2_USE_1",
    "CLK_COR_INSERT_IDLE_FLAG_0",
    "CLK_COR_INSERT_IDLE_FLAG_1",
    "CLK_COR_KEEP_IDLE_0",
    "CLK_COR_KEEP_IDLE_1",
    "CLK_COR_PRECEDENCE_0",
    "CLK_COR_PRECEDENCE_1",
    "CLK_CORRECT_USE_0",
    "CLK_CORRECT_USE_1",
    "CLK_COR_SEQ_2_USE_0",
    "CLK_COR_SEQ_2_USE_1",
    "CLKINDC_B",
    "CLKRCV_TRST",
    "COMMA_DOUBLE_0",
    "COMMA_DOUBLE_1",
    "DEC_MCOMMA_DETECT_0",
    "DEC_MCOMMA_DETECT_1",
    "DEC_PCOMMA_DETECT_0",
    "DEC_PCOMMA_DETECT_1",
    "DEC_VALID_COMMA_ONLY_0",
    "DEC_VALID_COMMA_ONLY_1",
    "MCOMMA_DETECT_0",
    "MCOMMA_DETECT_1",
    "OVERSAMPLE_MODE",
    "PCI_EXPRESS_MODE_0",
    "PCI_EXPRESS_MODE_1",
    "PCOMMA_DETECT_0",
    "PCOMMA_DETECT_1",
    "PLL_FB_DCCEN",
    "PLL_SATA_0",
    "PLL_SATA_1",
    "PLL_STARTUP_EN",
    "RCV_TERM_GND_0",
    "RCV_TERM_GND_1",
    "RCV_TERM_VTTRX_0",
    "RCV_TERM_VTTRX_1",
    "RX_BUFFER_USE_0",
    "RX_BUFFER_USE_1",
    "RX_CDR_FORCE_ROTATE_0",
    "RX_CDR_FORCE_ROTATE_1",
    "RX_DECODE_SEQ_MATCH_0",
    "RX_DECODE_SEQ_MATCH_1",
    "RX_EN_IDLE_HOLD_CDR",
    "RX_EN_IDLE_HOLD_DFE_0",
    "RX_EN_IDLE_HOLD_DFE_1",
    "RX_EN_IDLE_RESET_BUF_0",
    "RX_EN_IDLE_RESET_BUF_1",
    "RX_EN_IDLE_RESET_FR",
    "RX_EN_IDLE_RESET_PH",
    "RXGEARBOX_USE_0",
    "RXGEARBOX_USE_1",
    "RX_LOSS_OF_SYNC_FSM_0",
    "RX_LOSS_OF_SYNC_FSM_1",
    "TERMINATION_OVRD",
    "TX_BUFFER_USE_0",
    "TX_BUFFER_USE_1",
    "TXGEARBOX_USE_0",
    "TXGEARBOX_USE_1",
];

const GTX_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("ALIGN_COMMA_WORD_0", &["1", "2"]),
    ("ALIGN_COMMA_WORD_1", &["1", "2"]),
    ("CHAN_BOND_MODE_0", &["SLAVE", "MASTER", "#OFF"]),
    ("CHAN_BOND_MODE_1", &["SLAVE", "MASTER", "#OFF"]),
    ("CHAN_BOND_SEQ_LEN_0", &["1", "2", "3", "4"]),
    ("CHAN_BOND_SEQ_LEN_1", &["1", "2", "3", "4"]),
    ("CLK25_DIVIDER", &["1", "2", "3", "4", "5", "6", "10", "12"]),
    ("CLK_COR_ADJ_LEN_0", &["1", "2", "3", "4"]),
    ("CLK_COR_ADJ_LEN_1", &["1", "2", "3", "4"]),
    ("CLK_COR_DET_LEN_0", &["1", "2", "3", "4"]),
    ("CLK_COR_DET_LEN_1", &["1", "2", "3", "4"]),
    (
        "OOB_CLK_DIVIDER",
        &["1", "2", "4", "6", "8", "10", "12", "14"],
    ),
    ("PLL_DIVSEL_FB", &["1", "2", "3", "4", "5", "8", "10"]),
    (
        "PLL_DIVSEL_REF",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("PLL_RXDIVSEL_OUT_0", &["1", "2", "4"]),
    ("PLL_RXDIVSEL_OUT_1", &["1", "2", "4"]),
    ("PLL_TXDIVSEL_OUT_0", &["1", "2", "4"]),
    ("PLL_TXDIVSEL_OUT_1", &["1", "2", "4"]),
    (
        "RX_LOS_INVALID_INCR_0",
        &["1", "2", "4", "8", "16", "32", "64", "128"],
    ),
    (
        "RX_LOS_INVALID_INCR_1",
        &["1", "2", "4", "8", "16", "32", "64", "128"],
    ),
    (
        "RX_LOS_THRESHOLD_0",
        &["4", "8", "16", "32", "64", "128", "256", "512"],
    ),
    (
        "RX_LOS_THRESHOLD_1",
        &["4", "8", "16", "32", "64", "128", "256", "512"],
    ),
    ("RX_SLIDE_MODE_0", &["PCS", "PMA"]),
    ("RX_SLIDE_MODE_1", &["PCS", "PMA"]),
    ("RX_STATUS_FMT_0", &["PCIE", "SATA"]),
    ("RX_STATUS_FMT_1", &["PCIE", "SATA"]),
    ("RX_XCLK_SEL_0", &["RXUSR", "RXREC"]),
    ("RX_XCLK_SEL_1", &["RXUSR", "RXREC"]),
    ("TERMINATION_IMP_0", &["50", "75"]),
    ("TERMINATION_IMP_1", &["50", "75"]),
    ("TX_XCLK_SEL_0", &["TXUSR", "TXOUT"]),
    ("TX_XCLK_SEL_1", &["TXUSR", "TXOUT"]),
];

const GTX_ENUM_INT_ATTRS: &[(&str, Range<u32>)] = &[
    ("CHAN_BOND_1_MAX_SKEW_0", 1..15),
    ("CHAN_BOND_1_MAX_SKEW_1", 1..15),
    ("CHAN_BOND_2_MAX_SKEW_0", 1..15),
    ("CHAN_BOND_2_MAX_SKEW_1", 1..15),
    ("CLK_COR_MAX_LAT_0", 3..49),
    ("CLK_COR_MAX_LAT_1", 3..49),
    ("CLK_COR_MIN_LAT_0", 3..49),
    ("CLK_COR_MIN_LAT_1", 3..49),
    ("SATA_MAX_BURST_0", 1..62),
    ("SATA_MAX_BURST_1", 1..62),
    ("SATA_MAX_INIT_0", 1..62),
    ("SATA_MAX_INIT_1", 1..62),
    ("SATA_MAX_WAKE_0", 1..62),
    ("SATA_MAX_WAKE_1", 1..62),
    ("SATA_MIN_BURST_0", 1..62),
    ("SATA_MIN_BURST_1", 1..62),
    ("SATA_MIN_INIT_0", 1..62),
    ("SATA_MIN_INIT_1", 1..62),
    ("SATA_MIN_WAKE_0", 1..62),
    ("SATA_MIN_WAKE_1", 1..62),
];

const GTX_DEC_ATTRS: &[(&str, usize)] = &[
    ("CHAN_BOND_LEVEL_0", 3),
    ("CHAN_BOND_LEVEL_1", 3),
    ("CB2_INH_CC_PERIOD_0", 4),
    ("CB2_INH_CC_PERIOD_1", 4),
    ("CLK_COR_REPEAT_WAIT_0", 5),
    ("CLK_COR_REPEAT_WAIT_1", 5),
    ("TXOUTCLK_SEL_0", 1),
    ("TXOUTCLK_SEL_1", 1),
];

const GTX_BIN_ATTRS: &[(&str, usize)] = &[
    ("CDR_PH_ADJ_TIME", 5),
    ("CHAN_BOND_SEQ_1_1_0", 10),
    ("CHAN_BOND_SEQ_1_1_1", 10),
    ("CHAN_BOND_SEQ_1_2_0", 10),
    ("CHAN_BOND_SEQ_1_2_1", 10),
    ("CHAN_BOND_SEQ_1_3_0", 10),
    ("CHAN_BOND_SEQ_1_3_1", 10),
    ("CHAN_BOND_SEQ_1_4_0", 10),
    ("CHAN_BOND_SEQ_1_4_1", 10),
    ("CHAN_BOND_SEQ_1_ENABLE_0", 4),
    ("CHAN_BOND_SEQ_1_ENABLE_1", 4),
    ("CHAN_BOND_SEQ_2_1_0", 10),
    ("CHAN_BOND_SEQ_2_1_1", 10),
    ("CHAN_BOND_SEQ_2_2_0", 10),
    ("CHAN_BOND_SEQ_2_2_1", 10),
    ("CHAN_BOND_SEQ_2_3_0", 10),
    ("CHAN_BOND_SEQ_2_3_1", 10),
    ("CHAN_BOND_SEQ_2_4_0", 10),
    ("CHAN_BOND_SEQ_2_4_1", 10),
    ("CHAN_BOND_SEQ_2_ENABLE_0", 4),
    ("CHAN_BOND_SEQ_2_ENABLE_1", 4),
    ("CLK_COR_SEQ_1_1_0", 10),
    ("CLK_COR_SEQ_1_1_1", 10),
    ("CLK_COR_SEQ_1_2_0", 10),
    ("CLK_COR_SEQ_1_2_1", 10),
    ("CLK_COR_SEQ_1_3_0", 10),
    ("CLK_COR_SEQ_1_3_1", 10),
    ("CLK_COR_SEQ_1_4_0", 10),
    ("CLK_COR_SEQ_1_4_1", 10),
    ("CLK_COR_SEQ_1_ENABLE_0", 4),
    ("CLK_COR_SEQ_1_ENABLE_1", 4),
    ("CLK_COR_SEQ_2_1_0", 10),
    ("CLK_COR_SEQ_2_1_1", 10),
    ("CLK_COR_SEQ_2_2_0", 10),
    ("CLK_COR_SEQ_2_2_1", 10),
    ("CLK_COR_SEQ_2_3_0", 10),
    ("CLK_COR_SEQ_2_3_1", 10),
    ("CLK_COR_SEQ_2_4_0", 10),
    ("CLK_COR_SEQ_2_4_1", 10),
    ("CLK_COR_SEQ_2_ENABLE_0", 4),
    ("CLK_COR_SEQ_2_ENABLE_1", 4),
    ("CM_TRIM_0", 2),
    ("CM_TRIM_1", 2),
    ("COMMA_10B_ENABLE_0", 10),
    ("COMMA_10B_ENABLE_1", 10),
    ("COM_BURST_VAL_0", 4),
    ("COM_BURST_VAL_1", 4),
    ("DFE_CAL_TIME", 5),
    ("DFE_CFG_0", 10),
    ("DFE_CFG_1", 10),
    ("GEARBOX_ENDEC_0", 3),
    ("GEARBOX_ENDEC_1", 3),
    ("MCOMMA_10B_VALUE_0", 10),
    ("MCOMMA_10B_VALUE_1", 10),
    ("OOBDETECT_THRESHOLD_0", 3),
    ("OOBDETECT_THRESHOLD_1", 3),
    ("PCOMMA_10B_VALUE_0", 10),
    ("PCOMMA_10B_VALUE_1", 10),
    ("PLL_LKDET_CFG", 3),
    ("RX_IDLE_HI_CNT_0", 4),
    ("RX_IDLE_HI_CNT_1", 4),
    ("RX_IDLE_LO_CNT_0", 4),
    ("RX_IDLE_LO_CNT_1", 4),
    ("SATA_BURST_VAL_0", 3),
    ("SATA_BURST_VAL_1", 3),
    ("SATA_IDLE_VAL_0", 3),
    ("SATA_IDLE_VAL_1", 3),
    ("TERMINATION_CTRL", 5),
    ("TXRX_INVERT_0", 3),
    ("TXRX_INVERT_1", 3),
    ("TX_IDLE_DELAY_0", 3),
    ("TX_IDLE_DELAY_1", 3),
];

const GTX_HEX_ATTRS: &[(&str, usize)] = &[
    ("PLL_COM_CFG", 24),
    ("PLL_CP_CFG", 8),
    ("PLL_TDCC_CFG", 3),
    ("PMA_CDR_SCAN_0", 27),
    ("PMA_CDR_SCAN_1", 27),
    ("PMA_COM_CFG", 69),
    ("PMA_RXSYNC_CFG_0", 7),
    ("PMA_RXSYNC_CFG_1", 7),
    ("PMA_RX_CFG_0", 25),
    ("PMA_RX_CFG_1", 25),
    ("PMA_TX_CFG_0", 20),
    ("PMA_TX_CFG_1", 20),
    ("PRBS_ERR_THRESHOLD_0", 32),
    ("PRBS_ERR_THRESHOLD_1", 32),
    ("TRANS_TIME_FROM_P2_0", 12),
    ("TRANS_TIME_FROM_P2_1", 12),
    ("TRANS_TIME_NON_P2_0", 8),
    ("TRANS_TIME_NON_P2_1", 8),
    ("TRANS_TIME_TO_P2_0", 10),
    ("TRANS_TIME_TO_P2_1", 10),
    ("TX_DETECT_RX_CFG_0", 14),
    ("TX_DETECT_RX_CFG_1", 14),
];

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    for (tile, bel) in [("GTP", "GTP_DUAL"), ("GTX", "GTX_DUAL")] {
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, bel, TileBits::Mgt) else {
            continue;
        };
        fuzz_one!(ctx, "ENABLE", "1", [], [
            (mode bel)
        ]);
        for pin in ["RXUSRCLK0", "RXUSRCLK1", "TXUSRCLK0", "TXUSRCLK1"] {
            fuzz_one!(ctx, pin, "1", [
                (mode bel),
                (mutex "USRCLK", pin)
            ], [
                (pin pin)
            ]);
        }
        for &pin in GT_INVPINS {
            fuzz_inv!(ctx, pin, [(mode bel), (mutex "USRCLK", "INV")]);
        }
        if tile == "GTP" {
            for &attr in GTP_BOOL_ATTRS {
                if attr == "CLKINDC_B" {
                    fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                        (mode bel),
                        (pip (bel_pin BelId::from_idx(12), "O"), (bel_pin BelId::from_idx(1), "IP")),
                        (pip (bel_pin BelId::from_idx(13), "O"), (bel_pin BelId::from_idx(1), "IN"))
                    ]);
                } else {
                    fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [(mode bel)]);
                }
            }
            for &(attr, vals) in GTP_ENUM_ATTRS {
                fuzz_enum!(ctx, attr, vals.iter().copied(), [(mode bel)]);
            }
            for &(attr, ref vals) in GTP_ENUM_INT_ATTRS {
                let vals = Vec::from_iter(vals.clone().map(|i| i.to_string()));
                fuzz_enum!(ctx, attr, vals.iter(), [(mode bel)]);
            }
            for &(attr, width) in GTP_DEC_ATTRS {
                fuzz_multi_attr_dec!(ctx, attr, width, [(mode bel)]);
            }
            for &(attr, width) in GTP_BIN_ATTRS {
                fuzz_multi_attr_bin!(ctx, attr, width, [(mode bel)]);
            }
            for &(attr, width) in GTP_HEX_ATTRS {
                fuzz_multi_attr_hex!(ctx, attr, width, [(mode bel)]);
            }
        } else {
            for &attr in GTX_BOOL_ATTRS {
                fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [(mode bel)]);
            }
            for &(attr, vals) in GTX_ENUM_ATTRS {
                fuzz_enum!(ctx, attr, vals.iter().copied(), [(mode bel)]);
            }
            for &(attr, ref vals) in GTX_ENUM_INT_ATTRS {
                let vals = Vec::from_iter(vals.clone().map(|i| i.to_string()));
                fuzz_enum!(ctx, attr, vals.iter(), [(mode bel)]);
            }
            for &(attr, width) in GTX_DEC_ATTRS {
                fuzz_multi_attr_dec!(ctx, attr, width, [(mode bel)]);
            }
            for &(attr, width) in GTX_BIN_ATTRS {
                fuzz_multi_attr_bin!(ctx, attr, width, [(mode bel)]);
            }
            for &(attr, width) in GTX_HEX_ATTRS {
                fuzz_multi_attr_hex!(ctx, attr, width, [(mode bel)]);
            }
        }

        fuzz_one!(ctx, "MUX.CLKIN", "GREFCLK", [
            (mutex "MUX.CLKIN", "GREFCLK")
        ], [
            (pip (pin "GREFCLK"), (pin "CLKIN"))
        ]);
        fuzz_one!(ctx, "MUX.CLKIN", "CLKPN", [
            (mutex "MUX.CLKIN", "CLKPN")
        ], [
            (pip (bel_pin BelId::from_idx(1), "O"), (pin "CLKIN"))
        ]);
        fuzz_one!(ctx, "MUX.CLKIN", "CLKOUT_NORTH_S", [
            (mutex "MUX.CLKIN", "CLKOUT_NORTH_S")
        ], [
            (pip (pin "CLKOUT_NORTH_S"), (pin "CLKIN"))
        ]);
        fuzz_one!(ctx, "MUX.CLKIN", "CLKOUT_SOUTH_N", [
            (mutex "MUX.CLKIN", "CLKOUT_SOUTH_N")
        ], [
            (pip (pin "CLKOUT_SOUTH_N"), (pin "CLKIN"))
        ]);

        fuzz_one!(ctx, "MUX.CLKOUT_SOUTH", "CLKPN", [
            (mutex "MUX.CLKOUT_SOUTH", "CLKPN")
        ], [
            (pip (bel_pin BelId::from_idx(1), "O"), (pin "CLKOUT_SOUTH"))
        ]);
        fuzz_one!(ctx, "MUX.CLKOUT_SOUTH", "CLKOUT_SOUTH_N", [
            (mutex "MUX.CLKOUT_SOUTH", "CLKOUT_SOUTH_N")
        ], [
            (pip (pin "CLKOUT_SOUTH_N"), (pin "CLKOUT_SOUTH"))
        ]);

        fuzz_one!(ctx, "MUX.CLKOUT_NORTH", "CLKPN", [
            (mutex "MUX.CLKOUT_NORTH", "CLKPN")
        ], [
            (pip (bel_pin BelId::from_idx(1), "O"), (pin "CLKOUT_NORTH"))
        ]);
        fuzz_one!(ctx, "MUX.CLKOUT_NORTH", "CLKOUT_NORTH_S", [
            (mutex "MUX.CLKOUT_NORTH", "CLKOUT_NORTH_S")
        ], [
            (pip (pin "CLKOUT_NORTH_S"), (pin "CLKOUT_NORTH"))
        ]);

        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, "BUFDS", TileBits::Null) else {
            unreachable!()
        };
        fuzz_one!(ctx, "BUFDS", "1", [], [
            (mode "BUFDS")
        ]);

        for i in 0..2 {
            let Some(ctx) =
                FuzzCtx::try_new(session, backend, tile, format!("CRC64_{i}"), TileBits::Mgt)
            else {
                unreachable!()
            };
            fuzz_one!(ctx, "PRESENT", "1", [
                (tile_mutex "CRC_MODE", "64")
            ], [
                (mode "CRC64")
            ]);
            fuzz_inv!(ctx, "CRCCLK", [
                (mode "CRC64"),
                (tile_mutex "CRC_MODE", "64")
            ]);
            fuzz_multi_attr_hex!(ctx, "CRC_INIT", 32, [
                (mode "CRC64"),
                (tile_mutex "CRC_MODE", "64")
            ]);
        }

        for i in 0..4 {
            let Some(ctx) =
                FuzzCtx::try_new(session, backend, tile, format!("CRC32_{i}"), TileBits::Mgt)
            else {
                unreachable!()
            };
            fuzz_one!(ctx, "PRESENT", "1", [
                (tile_mutex "CRC_MODE", "32")
            ], [
                (mode "CRC32")
            ]);
            fuzz_inv!(ctx, "CRCCLK", [
                (mode "CRC32"),
                (tile_mutex "CRC_MODE", "32")
            ]);
            fuzz_multi_attr_hex!(ctx, "CRC_INIT", 32, [
                (mode "CRC32"),
                (tile_mutex "CRC_MODE", "32")
            ]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tile, bel) in [("GTP", "GTP_DUAL"), ("GTX", "GTX_DUAL")] {
        if !ctx.has_tile(tile) {
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
        for i in 0..0x50 {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("DRP{i:02X}"),
                TileItem::from_bitvec((0..16).map(|j| drp_bit(i, j)).collect(), false),
            );
        }
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        for &pin in GT_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        if tile == "GTP" {
            for &attr in GTP_BOOL_ATTRS {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            }
            for &(attr, vals) in GTP_ENUM_ATTRS {
                ctx.collect_enum(tile, bel, attr, vals);
            }
            for &(attr, ref vals) in GTP_ENUM_INT_ATTRS {
                ctx.collect_enum_int(tile, bel, attr, vals.clone(), 0);
            }
            for &(attr, _) in GTP_DEC_ATTRS {
                ctx.collect_bitvec(tile, bel, attr, "");
            }
            for &(attr, _) in GTP_BIN_ATTRS {
                ctx.collect_bitvec(tile, bel, attr, "");
            }
            for &(attr, _) in GTP_HEX_ATTRS {
                ctx.collect_bitvec(tile, bel, attr, "");
            }
        } else {
            for &attr in GTX_BOOL_ATTRS {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
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
        }

        ctx.collect_enum(
            tile,
            bel,
            "MUX.CLKIN",
            &["CLKPN", "GREFCLK", "CLKOUT_NORTH_S", "CLKOUT_SOUTH_N"],
        );
        ctx.collect_enum(tile, bel, "MUX.CLKOUT_SOUTH", &["CLKPN", "CLKOUT_SOUTH_N"]);
        ctx.collect_enum(tile, bel, "MUX.CLKOUT_NORTH", &["CLKPN", "CLKOUT_NORTH_S"]);

        let item_rx = ctx.extract_bit(tile, bel, "RXUSRCLK0", "1");
        let item_tx = ctx.extract_bit(tile, bel, "TXUSRCLK0", "1");
        assert_eq!(item_rx, item_tx);
        ctx.tiledb.insert(tile, bel, "USRCLK0", item_rx);
        let item_rx = ctx.extract_bit(tile, bel, "RXUSRCLK1", "1");
        let item_tx = ctx.extract_bit(tile, bel, "TXUSRCLK1", "1");
        assert_eq!(item_rx, item_tx);
        ctx.tiledb.insert(tile, bel, "USRCLK1", item_rx);

        for i in 0..4 {
            let bel = &format!("CRC32_{i}");
            ctx.collect_inv(tile, bel, "CRCCLK");
            ctx.collect_bitvec(tile, bel, "CRC_INIT", "");
            ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        }
        for i in 0..2 {
            let bel = &format!("CRC64_{i}");
            let bel32 = &format!("CRC32_{ii}", ii = i * 3);
            let item = ctx.extract_inv(tile, bel, "CRCCLK");
            ctx.tiledb.insert(tile, bel32, "INV.CRCCLK", item);
            let item = ctx.extract_bitvec(tile, bel, "CRC_INIT", "");
            ctx.tiledb.insert(tile, bel32, "CRC_INIT", item);
            let item = ctx.extract_bit(tile, bel, "PRESENT", "1");
            ctx.tiledb.insert(tile, bel32, "ENABLE64", item);
        }
    }
}
