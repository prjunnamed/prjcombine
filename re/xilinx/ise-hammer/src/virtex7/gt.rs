use std::{collections::BTreeMap, ops::Range};

use prjcombine_interconnect::grid::TileCoord;
use prjcombine_re_fpga_hammer::{
    backend::FuzzerProp,
    diff::{Diff, OcdMode, xlat_bit, xlat_enum},
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::defs;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            DynProp,
            pip::{PinFar, PipWire},
            relation::Delta,
        },
    },
};

const GTP_COMMON_INVPINS: &[&str] = &[
    "DRPCLK",
    "GTGREFCLK0",
    "GTGREFCLK1",
    "PLLCLKSPARE",
    "PLL0LOCKDETCLK",
    "PLL1LOCKDETCLK",
    "PMASCANCLK0",
    "PMASCANCLK1",
];

const GTP_COMMON_ENUM_ATTRS: &[(&str, &[&str])] = &[
    (
        "PLL0_FBDIV",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    (
        "PLL1_FBDIV",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("PLL0_FBDIV_45", &["4", "5"]),
    ("PLL1_FBDIV_45", &["4", "5"]),
    (
        "PLL0_REFCLK_DIV",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    (
        "PLL1_REFCLK_DIV",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
];

const GTP_COMMON_BIN_ATTRS: &[(&str, usize)] = &[
    ("AEN_BGBS", 1),
    ("AEN_MASTER", 1),
    ("AEN_PD", 1),
    ("AEN_PLL", 1),
    ("AEN_REFCLK", 1),
    ("AEN_RESET", 1),
    ("A_BGMONITOREN", 1),
    ("A_BGPD", 1),
    ("A_GTREFCLKPD0", 1),
    ("A_GTREFCLKPD1", 1),
    ("A_PLL0LOCKEN", 1),
    ("A_PLL1LOCKEN", 1),
    ("A_PLL0PD", 1),
    ("A_PLL1PD", 1),
    ("A_PLL0RESET", 1),
    ("A_PLL1RESET", 1),
    ("AQDMUXSEL", 3),
    ("COMMON_AMUX_SEL", 2),
    ("COMMON_INSTANTIATED", 1),
    ("PLL_CLKOUT_CFG", 8),
    ("PLL0_DMON_CFG", 1),
    ("PLL1_DMON_CFG", 1),
    ("EAST_REFCLK0_SEL", 2),
    ("EAST_REFCLK1_SEL", 2),
    ("WEST_REFCLK0_SEL", 2),
    ("WEST_REFCLK1_SEL", 2),
];

const GTP_COMMON_HEX_ATTRS: &[(&str, usize)] = &[
    ("BIAS_CFG", 64),
    ("COMMON_CFG", 32),
    ("PLL0_CFG", 27),
    ("PLL1_CFG", 27),
    ("PLL0_LOCK_CFG", 9),
    ("PLL1_LOCK_CFG", 9),
    ("PLL0_INIT_CFG", 24),
    ("PLL1_INIT_CFG", 24),
    ("RSVD_ATTR0", 16),
    ("RSVD_ATTR1", 16),
];

const GTXH_COMMON_INVPINS: &[&str] = &[
    "DRPCLK",
    "GTGREFCLK",
    "QPLLCLKSPARE0",
    "QPLLCLKSPARE1",
    "QPLLLOCKDETCLK",
    "PMASCANCLK0",
    "PMASCANCLK1",
];

const GTXH_COMMON_ENUM_ATTRS: &[(&str, &[&str])] = &[(
    "QPLL_REFCLK_DIV",
    &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
)];

const GTX_COMMON_BIN_ATTRS: &[(&str, usize)] = &[
    ("AEN_BGBS", 1),
    ("AEN_MASTER", 1),
    ("AEN_PD", 1),
    ("AEN_QPLL", 1),
    ("AEN_REFCLK", 1),
    ("AEN_RESET", 1),
    ("AQDMUXSEL", 3),
    ("A_BGMONITOREN", 1),
    ("A_BGPD", 1),
    ("A_GTREFCLKPD0", 1),
    ("A_GTREFCLKPD1", 1),
    ("A_QPLLLOCKEN", 1),
    ("A_QPLLOUTRESET", 1),
    ("A_QPLLPD", 1),
    ("A_QPLLRESET", 1),
    ("COMMON_AMUX_SEL", 2),
    ("COMMON_INSTANTIATED", 1),
    ("QPLL_AMONITOR_SEL", 2),
    ("QPLL_CLKOUT_CFG", 4),
    ("QPLL_COARSE_FREQ_OVRD", 6),
    ("QPLL_COARSE_FREQ_OVRD_EN", 1),
    ("QPLL_CP", 10),
    ("QPLL_CP_MONITOR_EN", 1),
    ("QPLL_DMONITOR_SEL", 1),
    ("QPLL_FBDIV", 10),
    ("QPLL_FBDIV_MONITOR_EN", 1),
    ("QPLL_FBDIV_RATIO", 1),
    ("QPLL_LPF", 4),
    ("QPLL_VCTRL_MONITOR_EN", 1),
    ("QPLL_VREG_MONITOR_EN", 1),
];

const GTX_COMMON_HEX_ATTRS: &[(&str, usize)] = &[
    ("BIAS_CFG", 64),
    ("COMMON_CFG", 32),
    ("QPLL_CFG", 27),
    ("QPLL_INIT_CFG", 24),
    ("QPLL_LOCK_CFG", 16),
];

const GTH_COMMON_BIN_ATTRS: &[(&str, usize)] = &[
    ("AEN_BGBS", 1),
    ("AEN_MASTER", 1),
    ("AEN_PD", 1),
    ("AEN_QPLL", 1),
    ("AEN_REFCLK", 1),
    ("AEN_RESET", 1),
    ("AQDMUXSEL", 3),
    ("A_BGMONITOREN", 1),
    ("A_BGPD", 1),
    ("A_GTREFCLKPD0", 1),
    ("A_GTREFCLKPD1", 1),
    ("A_QPLLLOCKEN", 1),
    ("A_QPLLOUTRESET", 1),
    ("A_QPLLPD", 1),
    ("A_QPLLRESET", 1),
    ("COMMON_AMUX_SEL", 2),
    ("COMMON_INSTANTIATED", 1),
    ("QPLL_AMONITOR_SEL", 2),
    ("QPLL_CLKOUT_CFG", 4),
    ("QPLL_COARSE_FREQ_OVRD", 6),
    ("QPLL_COARSE_FREQ_OVRD_EN", 1),
    ("QPLL_CP", 10),
    ("QPLL_CP_MONITOR_EN", 1),
    ("QPLL_DMONITOR_SEL", 1),
    ("QPLL_FBDIV", 10),
    ("QPLL_FBDIV_MONITOR_EN", 1),
    ("QPLL_FBDIV_RATIO", 1),
    ("QPLL_LPF", 4),
    ("QPLL_RP_COMP", 1),
    ("QPLL_VCTRL_MONITOR_EN", 1),
    ("QPLL_VREG_MONITOR_EN", 1),
    ("QPLL_VTRL_RESET", 2),
    ("RCAL_CFG", 2),
];

const GTH_COMMON_HEX_ATTRS: &[(&str, usize)] = &[
    ("BIAS_CFG", 64),
    ("COMMON_CFG", 32),
    ("QPLL_CFG", 27),
    ("QPLL_INIT_CFG", 24),
    ("QPLL_LOCK_CFG", 16),
    ("RSVD_ATTR0", 16),
    ("RSVD_ATTR1", 16),
];

const GTP_CHANNEL_INVPINS: &[&str] = &[
    "CLKRSVD0",
    "CLKRSVD1",
    "DMONITORCLK",
    "DRPCLK",
    "PMASCANCLK0",
    "PMASCANCLK1",
    "PMASCANCLK2",
    "PMASCANCLK3",
    "RXUSRCLK",
    "RXUSRCLK2",
    "SCANCLK",
    "SIGVALIDCLK",
    "TSTCLK0",
    "TSTCLK1",
    "TXPHDLYTSTCLK",
    "TXUSRCLK",
    "TXUSRCLK2",
];

const GTP_CHANNEL_BOOL_ATTRS: &[&str] = &[
    "ALIGN_COMMA_DOUBLE",
    "ALIGN_MCOMMA_DET",
    "ALIGN_PCOMMA_DET",
    "CHAN_BOND_KEEP_ALIGN",
    "CHAN_BOND_SEQ_2_USE",
    "CLK_COR_INSERT_IDLE_FLAG",
    "CLK_COR_KEEP_IDLE",
    "CLK_COR_PRECEDENCE",
    "CLK_CORRECT_USE",
    "CLK_COR_SEQ_2_USE",
    "DEC_MCOMMA_DETECT",
    "DEC_PCOMMA_DETECT",
    "DEC_VALID_COMMA_ONLY",
    "ES_ERRDET_EN",
    "ES_EYE_SCAN_EN",
    "FTS_LANE_DESKEW_EN",
    "GEN_RXUSRCLK",
    "GEN_TXUSRCLK",
    "PCS_PCIE_EN",
    "RXBUF_EN",
    "RXBUF_RESET_ON_CB_CHANGE",
    "RXBUF_RESET_ON_COMMAALIGN",
    "RXBUF_RESET_ON_EIDLE",
    "RXBUF_RESET_ON_RATE_CHANGE",
    "RXBUF_THRESH_OVRD",
    "RX_DEFER_RESET_BUF_EN",
    "RX_DISPERR_SEQ_MATCH",
    "RXGEARBOX_EN",
    "SHOW_REALIGN_COMMA",
    "TXBUF_EN",
    "TXBUF_RESET_ON_RATE_CHANGE",
    "TXGEARBOX_EN",
    "TX_LOOPBACK_DRIVE_HIZ",
];

const GTP_CHANNEL_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("ALIGN_COMMA_WORD", &["1", "2"]),
    ("CBCC_DATA_SOURCE_SEL", &["DECODED", "ENCODED"]),
    ("CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4"]),
    ("CLK_COR_SEQ_LEN", &["1", "2", "3", "4"]),
    ("RXBUF_ADDR_MODE", &["FULL", "FAST"]),
    ("RX_DATA_WIDTH", &["20", "16", "32", "40"]),
    ("RXOOB_CLK_CFG", &["PMA", "FABRIC"]),
    ("RXOUT_DIV", &["2", "1", "4", "8", "16"]),
    ("RXSIPO_DIV_45", &["4", "5"]),
    ("RXSLIDE_MODE", &["#OFF", "AUTO", "PCS", "PMA"]),
    ("RX_XCLK_SEL", &["RXREC", "RXUSR"]),
    (
        "SATA_PLL_CFG",
        &["VCO_3000MHZ", "VCO_750MHZ", "VCO_1500MHZ"],
    ),
    ("TX_DATA_WIDTH", &["20", "16", "32", "40"]),
    ("TX_DRIVE_MODE", &["DIRECT", "PIPE", "PIPEGEN3"]),
    ("TXOUT_DIV", &["2", "1", "4", "8", "16"]),
    ("TXPI_PPMCLK_SEL", &["TXUSRCLK2", "TXUSRCLK"]),
    ("TXPISO_DIV_45", &["5", "4"]),
    ("TX_XCLK_SEL", &["TXUSR", "TXOUT"]),
];

const GTP_CHANNEL_ENUM_INT_ATTRS: &[(&str, Range<u32>, u32)] = &[
    ("CHAN_BOND_MAX_SKEW", 1..15, 0),
    ("CLK_COR_MAX_LAT", 3..61, 0),
    ("CLK_COR_MIN_LAT", 3..61, 0),
    ("RX_CLK25_DIV", 0..32, 1),
    ("RX_SIG_VALID_DLY", 0..32, 1),
    ("SAS_MAX_COM", 1..128, 0),
    ("SAS_MIN_COM", 1..64, 0),
    ("SATA_MAX_BURST", 1..64, 0),
    ("SATA_MAX_INIT", 1..64, 0),
    ("SATA_MAX_WAKE", 1..64, 0),
    ("SATA_MIN_BURST", 1..62, 0),
    ("SATA_MIN_INIT", 1..64, 0),
    ("SATA_MIN_WAKE", 1..64, 0),
    ("TX_CLK25_DIV", 0..32, 1),
];

const GTP_CHANNEL_DEC_ATTRS: &[(&str, usize)] = &[
    ("CLK_COR_REPEAT_WAIT", 5),
    ("RXBUF_THRESH_OVFLW", 6),
    ("RXBUF_THRESH_UNDFLW", 6),
    ("RXSLIDE_AUTO_WAIT", 4),
    ("TXOUTCLKPCS_SEL", 1),
];

const GTP_CHANNEL_BIN_ATTRS: &[(&str, usize)] = &[
    ("ACJTAG_DEBUG_MODE", 1),
    ("ACJTAG_MODE", 1),
    ("ACJTAG_RESET", 1),
    ("ADAPT_CFG0", 20),
    ("AEN_LOOPBACK", 1),
    ("AEN_MASTER", 1),
    ("AEN_PD_AND_EIDLE", 1),
    ("AEN_PLL", 1),
    ("AEN_POLARITY", 1),
    ("AEN_PRBS", 1),
    ("AEN_RESET", 1),
    ("AEN_RSV", 1),
    ("AEN_RXCDR", 1),
    ("AEN_RXDFE", 1),
    ("AEN_RXLPM", 1),
    ("AEN_RXOUTCLK_SEL", 1),
    ("AEN_RXPHDLY", 1),
    ("AEN_RXSYSCLK_SEL", 1),
    ("AEN_TXOUTCLK_SEL", 1),
    ("AEN_TXPHDLY", 1),
    ("AEN_TXPI_PPM", 1),
    ("AEN_TXSYSCLK_SEL", 1),
    ("AEN_TX_DRIVE_MODE", 1),
    ("ALIGN_COMMA_ENABLE", 10),
    ("ALIGN_MCOMMA_VALUE", 10),
    ("ALIGN_PCOMMA_VALUE", 10),
    ("A_CFGRESET", 1),
    ("A_EYESCANMODE", 1),
    ("A_EYESCANRESET", 1),
    ("A_GTRESETSEL", 1),
    ("A_GTRXRESET", 1),
    ("A_GTTXRESET", 1),
    ("A_LOOPBACK", 3),
    ("A_PMARSVDIN3", 1),
    ("A_PMARSVDIN4", 1),
    ("A_RXADAPTSELTEST", 14),
    ("A_RXBUFRESET", 1),
    ("A_RXCDRFREQRESET", 1),
    ("A_RXCDRHOLD", 1),
    ("A_RXCDROVRDEN", 1),
    ("A_RXCDRRESET", 1),
    ("A_RXCDRRESETRSV", 1),
    ("A_RXDFEXYDEN", 1),
    ("A_RXDLYBYPASS", 1),
    ("A_RXDLYEN", 1),
    ("A_RXDLYOVRDEN", 1),
    ("A_RXDLYSRESET", 1),
    ("A_RXLPMHFHOLD", 1),
    ("A_RXLPMHFOVRDEN", 1),
    ("A_RXLPMLFHOLD", 1),
    ("A_RXLPMLFOVRDEN", 1),
    ("A_RXLPMOSINTNTRLEN", 1),
    ("A_RXLPMRESET", 1),
    ("A_RXOOBRESET", 1),
    ("A_RXOSCALRESET", 1),
    ("A_RXOSHOLD", 1),
    ("A_RXOSINTCFG", 4),
    ("A_RXOSINTEN", 1),
    ("A_RXOSINTHOLD", 1),
    ("A_RXOSINTID0", 4),
    ("A_RXOSINTNTRLEN", 1),
    ("A_RXOSINTOVRDEN", 1),
    ("A_RXOSINTSTROBE", 1),
    ("A_RXOSINTTESTOVRDEN", 1),
    ("A_RXOSOVRDEN", 1),
    ("A_RXOUTCLKSEL", 3),
    ("A_RXPCSRESET", 1),
    ("A_RXPD", 2),
    ("A_RXPHALIGN", 1),
    ("A_RXPHALIGNEN", 1),
    ("A_RXPHDLYPD", 1),
    ("A_RXPHDLYRESET", 1),
    ("A_RXPHOVRDEN", 1),
    ("A_RXPMARESET", 1),
    ("A_RXPOLARITY", 1),
    ("A_RXPRBSCNTRESET", 1),
    ("A_RXPRBSSEL", 3),
    ("A_RXSYSCLKSEL", 2),
    ("A_SPARE", 1),
    ("A_TXBUFDIFFCTRL", 3),
    ("A_TXDEEMPH", 1),
    ("A_TXDIFFCTRL", 4),
    ("A_TXDLYBYPASS", 1),
    ("A_TXDLYEN", 1),
    ("A_TXDLYOVRDEN", 1),
    ("A_TXDLYSRESET", 1),
    ("A_TXELECIDLE", 1),
    ("A_TXINHIBIT", 1),
    ("A_TXMAINCURSOR", 7),
    ("A_TXMARGIN", 3),
    ("A_TXOUTCLKSEL", 3),
    ("A_TXPCSRESET", 1),
    ("A_TXPD", 2),
    ("A_TXPHALIGN", 1),
    ("A_TXPHALIGNEN", 1),
    ("A_TXPHDLYPD", 1),
    ("A_TXPHDLYRESET", 1),
    ("A_TXPHINIT", 1),
    ("A_TXPHOVRDEN", 1),
    ("A_TXPIPPMOVRDEN", 1),
    ("A_TXPIPPMPD", 1),
    ("A_TXPIPPMSEL", 1),
    ("A_TXPMARESET", 1),
    ("A_TXPOLARITY", 1),
    ("A_TXPOSTCURSOR", 5),
    ("A_TXPOSTCURSORINV", 1),
    ("A_TXPRBSFORCEERR", 1),
    ("A_TXPRBSSEL", 3),
    ("A_TXPRECURSOR", 5),
    ("A_TXPRECURSORINV", 1),
    ("A_TXSWING", 1),
    ("A_TXSYSCLKSEL", 2),
    ("CFOK_CFG", 43),
    ("CFOK_CFG2", 7),
    ("CFOK_CFG3", 7),
    ("CFOK_CFG4", 1),
    ("CFOK_CFG5", 2),
    ("CFOK_CFG6", 4),
    ("CHAN_BOND_SEQ_1_1", 10),
    ("CHAN_BOND_SEQ_1_2", 10),
    ("CHAN_BOND_SEQ_1_3", 10),
    ("CHAN_BOND_SEQ_1_4", 10),
    ("CHAN_BOND_SEQ_1_ENABLE", 4),
    ("CHAN_BOND_SEQ_2_1", 10),
    ("CHAN_BOND_SEQ_2_2", 10),
    ("CHAN_BOND_SEQ_2_3", 10),
    ("CHAN_BOND_SEQ_2_4", 10),
    ("CHAN_BOND_SEQ_2_ENABLE", 4),
    ("CLK_COMMON_SWING", 1),
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
    ("ES_CLK_PHASE_SEL", 1),
    ("ES_CONTROL", 6),
    ("ES_PMA_CFG", 10),
    ("ES_PRESCALE", 5),
    ("ES_VERT_OFFSET", 9),
    ("FTS_DESKEW_SEQ_ENABLE", 4),
    ("FTS_LANE_DESKEW_CFG", 4),
    ("GEARBOX_MODE", 3),
    ("GT_INSTANTIATED", 1),
    ("LOOPBACK_CFG", 1),
    ("OUTREFCLK_SEL_INV", 2),
    ("PCD_2UI_CFG", 1),
    ("PMA_LOOPBACK_CFG", 1),
    ("PMA_POWER_SAVE", 10),
    ("PMA_RSV3", 2),
    ("PMA_RSV4", 4),
    ("PMA_RSV5", 1),
    ("PMA_RSV6", 1),
    ("PMA_RSV7", 1),
    ("RXBUFRESET_TIME", 5),
    ("RXBUF_EIDLE_HI_CNT", 4),
    ("RXBUF_EIDLE_LO_CNT", 4),
    ("RXCDRFREQRESET_TIME", 5),
    ("RXCDRPHRESET_TIME", 5),
    ("RXCDRRESET_TIME", 7),
    ("RXCDR_FR_RESET_ON_EIDLE", 1),
    ("RXCDR_HOLD_DURING_EIDLE", 1),
    ("RXCDR_LOCK_CFG", 6),
    ("RXCDR_PCIERESET_WAIT_TIME", 5),
    ("RXCDR_PH_RESET_ON_EIDLE", 1),
    ("RXISCANRESET_TIME", 5),
    ("RXLPMRESET_TIME", 7),
    ("RXLPM_BIAS_STARTUP_DISABLE", 1),
    ("RXLPM_CFG", 4),
    ("RXLPM_CFG1", 1),
    ("RXLPM_CM_CFG", 1),
    ("RXLPM_GC_CFG", 9),
    ("RXLPM_GC_CFG2", 3),
    ("RXLPM_HF_CFG", 14),
    ("RXLPM_HF_CFG2", 5),
    ("RXLPM_HF_CFG3", 4),
    ("RXLPM_HOLD_DURING_EIDLE", 1),
    ("RXLPM_INCM_CFG", 1),
    ("RXLPM_IPCM_CFG", 1),
    ("RXLPM_LF_CFG", 18),
    ("RXLPM_LF_CFG2", 5),
    ("RXLPM_OSINT_CFG", 3),
    ("RXOOB_CFG", 7),
    ("RXOSCALRESET_TIME", 5),
    ("RXOSCALRESET_TIMEOUT", 5),
    ("RXPCSRESET_TIME", 5),
    ("RXPH_MONITOR_SEL", 5),
    ("RXPI_CFG0", 3),
    ("RXPI_CFG1", 1),
    ("RXPI_CFG2", 1),
    ("RXPLL_SEL", 1),
    ("RXPMARESET_TIME", 5),
    ("RXPRBS_ERR_LOOPBACK", 1),
    ("RXSYNC_MULTILANE", 1),
    ("RXSYNC_OVRD", 1),
    ("RXSYNC_SKIP_DA", 1),
    ("RX_BIAS_CFG", 16),
    ("RX_BUFFER_CFG", 6),
    ("RX_CLKMUX_EN", 1),
    ("RX_CM_SEL", 2),
    ("RX_CM_TRIM", 4),
    ("RX_DDI_SEL", 6),
    ("RX_DEBUG_CFG", 14),
    ("RX_OS_CFG", 13),
    ("SATA_BURST_SEQ_LEN", 4),
    ("SATA_BURST_VAL", 3),
    ("SATA_EIDLE_VAL", 3),
    ("SP_REFCLK_CFG", 3),
    ("TERM_RCAL_CFG", 15),
    ("TERM_RCAL_OVRD", 3),
    ("TXOOB_CFG", 1),
    ("TXPCSRESET_TIME", 5),
    ("TXPH_MONITOR_SEL", 5),
    ("TXPI_CFG0", 2),
    ("TXPI_CFG1", 2),
    ("TXPI_CFG2", 2),
    ("TXPI_CFG3", 1),
    ("TXPI_CFG4", 1),
    ("TXPI_CFG5", 3),
    ("TXPI_GREY_SEL", 1),
    ("TXPI_INVSTROBE_SEL", 1),
    ("TXPI_PPM_CFG", 8),
    ("TXPI_SYNFREQ_PPM", 3),
    ("TXPLL_SEL", 1),
    ("TXPMARESET_TIME", 5),
    ("TXSYNC_MULTILANE", 1),
    ("TXSYNC_OVRD", 1),
    ("TXSYNC_SKIP_DA", 1),
    ("TX_CLKMUX_EN", 1),
    ("TX_DEEMPH0", 6),
    ("TX_DEEMPH1", 6),
    ("TX_EIDLE_ASSERT_DELAY", 3),
    ("TX_EIDLE_DEASSERT_DELAY", 3),
    ("TX_MAINCURSOR_SEL", 1),
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
    ("TX_PREDRIVER_MODE", 1),
    ("TX_RXDETECT_REF", 3),
    ("UCODEER_CLR", 1),
    ("USE_PCS_CLK_PHASE_SEL", 1),
];

const GTP_CHANNEL_HEX_ATTRS: &[(&str, usize)] = &[
    ("AMONITOR_CFG", 16),
    ("DMONITOR_CFG", 24),
    ("ES_HORZ_OFFSET", 12),
    ("ES_QUALIFIER", 80),
    ("ES_QUAL_MASK", 80),
    ("ES_SDATA_MASK", 80),
    ("PCS_RSVD_ATTR", 48),
    ("PD_TRANS_TIME_FROM_P2", 12),
    ("PD_TRANS_TIME_NONE_P2", 8),
    ("PD_TRANS_TIME_TO_P2", 8),
    ("PMA_RSV", 32),
    ("PMA_RSV2", 32),
    ("RXCDR_CFG", 83),
    ("RXDLY_CFG", 16),
    ("RXDLY_LCFG", 9),
    ("RXDLY_TAP_CFG", 16),
    ("RXPHDLY_CFG", 24),
    ("RXPH_CFG", 24),
    ("TRANS_TIME_RATE", 8),
    ("TST_RSV", 32),
    ("TXDLY_CFG", 16),
    ("TXDLY_LCFG", 9),
    ("TXDLY_TAP_CFG", 16),
    ("TXPHDLY_CFG", 24),
    ("TXPH_CFG", 16),
    ("TX_RXDETECT_CFG", 14),
];

const GTX_CHANNEL_INVPINS: &[&str] = &[
    "CPLLLOCKDETCLK",
    "DRPCLK",
    "EDTCLOCK",
    "GTGREFCLK",
    "PMASCANCLK0",
    "PMASCANCLK1",
    "PMASCANCLK2",
    "PMASCANCLK3",
    "PMASCANCLK4",
    "RXUSRCLK",
    "RXUSRCLK2",
    "SCANCLK",
    "TSTCLK0",
    "TSTCLK1",
    "TXPHDLYTSTCLK",
    "TXUSRCLK",
    "TXUSRCLK2",
];

const GTX_CHANNEL_BOOL_ATTRS: &[&str] = &[
    "ALIGN_COMMA_DOUBLE",
    "ALIGN_MCOMMA_DET",
    "ALIGN_PCOMMA_DET",
    "CHAN_BOND_KEEP_ALIGN",
    "CHAN_BOND_SEQ_2_USE",
    "CLK_COR_INSERT_IDLE_FLAG",
    "CLK_COR_KEEP_IDLE",
    "CLK_COR_PRECEDENCE",
    "CLK_COR_SEQ_2_USE",
    "CLK_CORRECT_USE",
    "DEC_MCOMMA_DETECT",
    "DEC_PCOMMA_DETECT",
    "DEC_VALID_COMMA_ONLY",
    "ES_ERRDET_EN",
    "ES_EYE_SCAN_EN",
    "FTS_LANE_DESKEW_EN",
    "GEN_RXUSRCLK",
    "GEN_TXUSRCLK",
    "PCS_PCIE_EN",
    "RX_DEFER_RESET_BUF_EN",
    "RX_DISPERR_SEQ_MATCH",
    "RXBUF_EN",
    "RXBUF_RESET_ON_CB_CHANGE",
    "RXBUF_RESET_ON_COMMAALIGN",
    "RXBUF_RESET_ON_EIDLE",
    "RXBUF_RESET_ON_RATE_CHANGE",
    "RXBUF_THRESH_OVRD",
    "RXGEARBOX_EN",
    "SHOW_REALIGN_COMMA",
    "TX_LOOPBACK_DRIVE_HIZ",
    "TXBUF_EN",
    "TXBUF_RESET_ON_RATE_CHANGE",
    "TXGEARBOX_EN",
];

const GTX_CHANNEL_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("ALIGN_COMMA_WORD", &["1", "2", "4"]),
    ("CBCC_DATA_SOURCE_SEL", &["DECODED", "ENCODED"]),
    ("CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4"]),
    ("CLK_COR_SEQ_LEN", &["1", "2", "3", "4"]),
    (
        "CPLL_FBDIV",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("CPLL_FBDIV_45", &["5", "4"]),
    (
        "CPLL_REFCLK_DIV",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("RXOUT_DIV", &["1", "2", "4", "8", "16"]),
    ("TXOUT_DIV", &["1", "2", "4", "8", "16"]),
    ("RX_DATA_WIDTH", &["16", "20", "32", "40", "64", "80"]),
    ("RX_XCLK_SEL", &["RXREC", "RXUSR"]),
    ("RXBUF_ADDR_MODE", &["FULL", "FAST"]),
    ("RXPLL_SEL", &["CPLL", "QPLL"]),
    ("RXSIPO_DIV_45", &["4", "5"]),
    ("RXSLIDE_MODE", &["#OFF", "AUTO", "PCS", "PMA"]),
    (
        "SATA_CPLL_CFG",
        &["VCO_3000MHZ", "VCO_750MHZ", "VCO_1500MHZ"],
    ),
    ("TX_DATA_WIDTH", &["16", "20", "32", "40", "64", "80"]),
    ("TX_DRIVE_MODE", &["DIRECT", "PIPE", "PIPEGEN3"]),
    ("TX_XCLK_SEL", &["TXUSR", "TXOUT"]),
    ("TXPISO_DIV_45", &["5", "4"]),
    ("TXPLL_SEL", &["CPLL", "QPLL"]),
];

const GTX_CHANNEL_ENUM_INT_ATTRS: &[(&str, Range<u32>, u32)] = &[
    ("CHAN_BOND_MAX_SKEW", 1..15, 0),
    ("CLK_COR_MAX_LAT", 3..61, 0),
    ("CLK_COR_MIN_LAT", 3..61, 0),
    ("RX_CLK25_DIV", 0..32, 1),
    ("RX_SIG_VALID_DLY", 0..32, 1),
    ("SAS_MAX_COM", 1..128, 0),
    ("SAS_MIN_COM", 1..64, 0),
    ("SATA_MAX_BURST", 1..64, 0),
    ("SATA_MAX_INIT", 1..64, 0),
    ("SATA_MAX_WAKE", 1..64, 0),
    ("SATA_MIN_BURST", 1..62, 0),
    ("SATA_MIN_INIT", 1..64, 0),
    ("SATA_MIN_WAKE", 1..64, 0),
    ("TX_CLK25_DIV", 0..32, 1),
];

const GTX_CHANNEL_DEC_ATTRS: &[(&str, usize)] = &[
    ("CLK_COR_REPEAT_WAIT", 5),
    ("RXBUF_THRESH_OVFLW", 6),
    ("RXBUF_THRESH_UNDFLW", 6),
    ("RXSLIDE_AUTO_WAIT", 4),
    ("RX_INT_DATAWIDTH", 1),
    ("TXOUTCLKPCS_SEL", 1),
    ("TX_INT_DATAWIDTH", 1),
];

const GTX_CHANNEL_BIN_ATTRS: &[(&str, usize)] = &[
    ("AEN_CPLL", 1),
    ("AEN_LOOPBACK", 1),
    ("AEN_MASTER", 1),
    ("AEN_PD_AND_EIDLE", 1),
    ("AEN_POLARITY", 1),
    ("AEN_PRBS", 1),
    ("AEN_QPI", 1),
    ("AEN_RESET", 1),
    ("AEN_RXCDR", 1),
    ("AEN_RXDFE", 1),
    ("AEN_RXDFELPM", 1),
    ("AEN_RXOUTCLK_SEL", 1),
    ("AEN_RXPHDLY", 1),
    ("AEN_RXSYSCLK_SEL", 1),
    ("AEN_TXOUTCLK_SEL", 1),
    ("AEN_TXPHDLY", 1),
    ("AEN_TXSYSCLK_SEL", 1),
    ("AEN_TX_DRIVE_MODE", 1),
    ("ALIGN_COMMA_ENABLE", 10),
    ("ALIGN_MCOMMA_VALUE", 10),
    ("ALIGN_PCOMMA_VALUE", 10),
    ("A_CFGRESET", 1),
    ("A_CPLLLOCKEN", 1),
    ("A_CPLLPD", 1),
    ("A_CPLLRESET", 1),
    ("A_EYESCANMODE", 1),
    ("A_EYESCANRESET", 1),
    ("A_GTRESETSEL", 1),
    ("A_GTRXRESET", 1),
    ("A_GTTXRESET", 1),
    ("A_LOOPBACK", 3),
    ("A_RXBUFRESET", 1),
    ("A_RXCDRFREQRESET", 1),
    ("A_RXCDRHOLD", 1),
    ("A_RXCDROVRDEN", 1),
    ("A_RXCDRRESET", 1),
    ("A_RXCDRRESETRSV", 1),
    ("A_RXDFEAGCHOLD", 1),
    ("A_RXDFEAGCOVRDEN", 1),
    ("A_RXDFECM1EN", 1),
    ("A_RXDFELFHOLD", 1),
    ("A_RXDFELFOVRDEN", 1),
    ("A_RXDFELPMRESET", 1),
    ("A_RXDFETAP2HOLD", 1),
    ("A_RXDFETAP2OVRDEN", 1),
    ("A_RXDFETAP3HOLD", 1),
    ("A_RXDFETAP3OVRDEN", 1),
    ("A_RXDFETAP4HOLD", 1),
    ("A_RXDFETAP4OVRDEN", 1),
    ("A_RXDFETAP5HOLD", 1),
    ("A_RXDFETAP5OVRDEN", 1),
    ("A_RXDFEUTHOLD", 1),
    ("A_RXDFEUTOVRDEN", 1),
    ("A_RXDFEVPHOLD", 1),
    ("A_RXDFEVPOVRDEN", 1),
    ("A_RXDFEVSEN", 1),
    ("A_RXDFEXYDEN", 1),
    ("A_RXDFEXYDHOLD", 1),
    ("A_RXDFEXYDOVRDEN", 1),
    ("A_RXDLYBYPASS", 1),
    ("A_RXDLYEN", 1),
    ("A_RXDLYOVRDEN", 1),
    ("A_RXDLYSRESET", 1),
    ("A_RXLPMEN", 1),
    ("A_RXLPMHFHOLD", 1),
    ("A_RXLPMHFOVRDEN", 1),
    ("A_RXLPMLFHOLD", 1),
    ("A_RXLPMLFKLOVRDEN", 1),
    ("A_RXMONITORSEL", 2),
    ("A_RXOOBRESET", 1),
    ("A_RXOSHOLD", 1),
    ("A_RXOSOVRDEN", 1),
    ("A_RXOUTCLKSEL", 3),
    ("A_RXPCSRESET", 1),
    ("A_RXPD", 2),
    ("A_RXPHALIGN", 1),
    ("A_RXPHALIGNEN", 1),
    ("A_RXPHDLYPD", 1),
    ("A_RXPHDLYRESET", 1),
    ("A_RXPHOVRDEN", 1),
    ("A_RXPMARESET", 1),
    ("A_RXPOLARITY", 1),
    ("A_RXPRBSCNTRESET", 1),
    ("A_RXPRBSSEL", 3),
    ("A_RXSYSCLKSEL", 2),
    ("A_SPARE", 1),
    ("A_TXBUFDIFFCTRL", 3),
    ("A_TXDEEMPH", 1),
    ("A_TXDIFFCTRL", 4),
    ("A_TXDLYBYPASS", 1),
    ("A_TXDLYEN", 1),
    ("A_TXDLYOVRDEN", 1),
    ("A_TXDLYSRESET", 1),
    ("A_TXELECIDLE", 1),
    ("A_TXINHIBIT", 1),
    ("A_TXMAINCURSOR", 7),
    ("A_TXMARGIN", 3),
    ("A_TXOUTCLKSEL", 3),
    ("A_TXPCSRESET", 1),
    ("A_TXPD", 2),
    ("A_TXPHALIGN", 1),
    ("A_TXPHALIGNEN", 1),
    ("A_TXPHDLYPD", 1),
    ("A_TXPHDLYRESET", 1),
    ("A_TXPHINIT", 1),
    ("A_TXPHOVRDEN", 1),
    ("A_TXPMARESET", 1),
    ("A_TXPOLARITY", 1),
    ("A_TXPOSTCURSOR", 5),
    ("A_TXPOSTCURSORINV", 1),
    ("A_TXPRBSFORCEERR", 1),
    ("A_TXPRBSSEL", 3),
    ("A_TXPRECURSOR", 5),
    ("A_TXPRECURSORINV", 1),
    ("A_TXSWING", 1),
    ("A_TXSYSCLKSEL", 2),
    ("CHAN_BOND_SEQ_1_1", 10),
    ("CHAN_BOND_SEQ_1_2", 10),
    ("CHAN_BOND_SEQ_1_3", 10),
    ("CHAN_BOND_SEQ_1_4", 10),
    ("CHAN_BOND_SEQ_1_ENABLE", 4),
    ("CHAN_BOND_SEQ_2_1", 10),
    ("CHAN_BOND_SEQ_2_2", 10),
    ("CHAN_BOND_SEQ_2_3", 10),
    ("CHAN_BOND_SEQ_2_4", 10),
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
    ("CPLL_PCD_1UI_CFG", 1),
    ("CPLL_PCD_2UI_CFG", 1),
    ("ES_CONTROL", 6),
    ("ES_PMA_CFG", 10),
    ("ES_PRESCALE", 5),
    ("ES_VERT_OFFSET", 9),
    ("FTS_DESKEW_SEQ_ENABLE", 4),
    ("FTS_LANE_DESKEW_CFG", 4),
    ("GEARBOX_MODE", 3),
    ("GT_INSTANTIATED", 1),
    ("OUTREFCLK_SEL_INV", 2),
    ("PMA_POWER_SAVE", 10),
    ("PMA_RSV3", 2),
    ("RXBUFRESET_TIME", 5),
    ("RXBUF_EIDLE_HI_CNT", 4),
    ("RXBUF_EIDLE_LO_CNT", 4),
    ("RXCDRFREQRESET_TIME", 5),
    ("RXCDRPHRESET_TIME", 5),
    ("RXCDRRESET_TIME", 7),
    ("RXCDR_FR_RESET_ON_EIDLE", 1),
    ("RXCDR_HOLD_DURING_EIDLE", 1),
    ("RXCDR_LOCK_CFG", 6),
    ("RXCDR_PCIERESET_WAIT_TIME", 5),
    ("RXCDR_PH_RESET_ON_EIDLE", 1),
    ("RXDFELPMRESET_TIME", 7),
    ("RXISCANRESET_TIME", 5),
    ("RXLPM_HF_CFG", 14),
    ("RXLPM_LF_CFG", 14),
    ("RXOOB_CFG", 7),
    ("RXPCSRESET_TIME", 5),
    ("RXPH_MONITOR_SEL", 5),
    ("RXPMARESET_TIME", 5),
    ("RXPRBS_ERR_LOOPBACK", 1),
    ("RX_BIAS_CFG", 12),
    ("RX_BUFFER_CFG", 6),
    ("RX_CLKMUX_PD", 1),
    ("RX_CM_SEL", 2),
    ("RX_CM_TRIM", 3),
    ("RX_DDI_SEL", 6),
    ("RX_DEBUG_CFG", 12),
    ("RX_DFE_H2_CFG", 12),
    ("RX_DFE_H3_CFG", 12),
    ("RX_DFE_H4_CFG", 11),
    ("RX_DFE_H5_CFG", 11),
    ("RX_DFE_KL_CFG", 13),
    ("RX_DFE_LPM_HOLD_DURING_EIDLE", 1),
    ("RX_DFE_UT_CFG", 17),
    ("RX_DFE_VP_CFG", 17),
    ("RX_DFE_VS_CFG", 9),
    ("RX_DFE_XYD_CFG", 13),
    ("RX_OS_CFG", 13),
    ("SATA_BURST_SEQ_LEN", 4),
    ("SATA_BURST_VAL", 3),
    ("SATA_EIDLE_VAL", 3),
    ("SP_REFCLK_CFG", 3),
    ("TERM_RCAL_CFG", 5),
    ("TERM_RCAL_OVRD", 1),
    ("TXPCSRESET_TIME", 5),
    ("TXPH_MONITOR_SEL", 5),
    ("TXPMARESET_TIME", 5),
    ("TX_CLKMUX_PD", 1),
    ("TX_DEEMPH0", 5),
    ("TX_DEEMPH1", 5),
    ("TX_EIDLE_ASSERT_DELAY", 3),
    ("TX_EIDLE_DEASSERT_DELAY", 3),
    ("TX_MAINCURSOR_SEL", 1),
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
    ("TX_PREDRIVER_MODE", 1),
    ("TX_QPI_STATUS_EN", 1),
    ("TX_RXDETECT_REF", 3),
    ("UCODEER_CLR", 1),
];

const GTX_CHANNEL_HEX_ATTRS: &[(&str, usize)] = &[
    ("AMONITOR_CFG", 16),
    ("CPLL_CFG", 24),
    ("CPLL_INIT_CFG", 24),
    ("CPLL_LOCK_CFG", 16),
    ("DMONITOR_CFG", 24),
    ("ES_HORZ_OFFSET", 12),
    ("ES_QUALIFIER", 80),
    ("ES_QUAL_MASK", 80),
    ("ES_SDATA_MASK", 80),
    ("PCS_RSVD_ATTR", 48),
    ("PD_TRANS_TIME_FROM_P2", 12),
    ("PD_TRANS_TIME_NONE_P2", 8),
    ("PD_TRANS_TIME_TO_P2", 8),
    ("PMA_RSV", 32),
    ("PMA_RSV2", 16),
    ("PMA_RSV4", 32),
    ("RXCDR_CFG", 72),
    ("RXDLY_CFG", 16),
    ("RXDLY_LCFG", 9),
    ("RXDLY_TAP_CFG", 16),
    ("RXPHDLY_CFG", 24),
    ("RXPH_CFG", 24),
    ("RX_DFE_GAIN_CFG", 23),
    ("RX_DFE_KL_CFG2", 32),
    ("RX_DFE_LPM_CFG", 16),
    ("TRANS_TIME_RATE", 8),
    ("TST_RSV", 32),
    ("TXDLY_CFG", 16),
    ("TXDLY_LCFG", 9),
    ("TXDLY_TAP_CFG", 16),
    ("TXPHDLY_CFG", 24),
    ("TXPH_CFG", 16),
    ("TX_RXDETECT_CFG", 14),
];

const GTH_CHANNEL_INVPINS: &[&str] = &[
    "CLKRSVD0",
    "CLKRSVD1",
    "CPLLLOCKDETCLK",
    "DMONITORCLK",
    "DRPCLK",
    "GTGREFCLK",
    "PMASCANCLK0",
    "PMASCANCLK1",
    "PMASCANCLK2",
    "PMASCANCLK3",
    "PMASCANCLK4",
    "RXUSRCLK",
    "RXUSRCLK2",
    "SCANCLK",
    "SIGVALIDCLK",
    "TSTCLK0",
    "TSTCLK1",
    "TXPHDLYTSTCLK",
    "TXUSRCLK",
    "TXUSRCLK2",
];

const GTH_CHANNEL_BOOL_ATTRS: &[&str] = &[
    "ALIGN_COMMA_DOUBLE",
    "ALIGN_MCOMMA_DET",
    "ALIGN_PCOMMA_DET",
    "CHAN_BOND_KEEP_ALIGN",
    "CHAN_BOND_SEQ_2_USE",
    "CLK_COR_INSERT_IDLE_FLAG",
    "CLK_COR_KEEP_IDLE",
    "CLK_COR_PRECEDENCE",
    "CLK_CORRECT_USE",
    "CLK_COR_SEQ_2_USE",
    "DEC_MCOMMA_DETECT",
    "DEC_PCOMMA_DETECT",
    "DEC_VALID_COMMA_ONLY",
    "ES_ERRDET_EN",
    "ES_EYE_SCAN_EN",
    "FTS_LANE_DESKEW_EN",
    "GEN_RXUSRCLK",
    "GEN_TXUSRCLK",
    "PCS_PCIE_EN",
    "RXBUF_EN",
    "RXBUF_RESET_ON_CB_CHANGE",
    "RXBUF_RESET_ON_COMMAALIGN",
    "RXBUF_RESET_ON_EIDLE",
    "RXBUF_RESET_ON_RATE_CHANGE",
    "RXBUF_THRESH_OVRD",
    "RX_DEFER_RESET_BUF_EN",
    "RX_DISPERR_SEQ_MATCH",
    "RXGEARBOX_EN",
    "SHOW_REALIGN_COMMA",
    "TXBUF_EN",
    "TXBUF_RESET_ON_RATE_CHANGE",
    "TXGEARBOX_EN",
    "TX_LOOPBACK_DRIVE_HIZ",
];

const GTH_CHANNEL_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("ALIGN_COMMA_WORD", &["1", "2", "4"]),
    ("CBCC_DATA_SOURCE_SEL", &["DECODED", "ENCODED"]),
    ("CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4"]),
    ("CLK_COR_SEQ_LEN", &["1", "2", "3", "4"]),
    (
        "CPLL_FBDIV",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("CPLL_FBDIV_45", &["4", "5"]),
    (
        "CPLL_REFCLK_DIV",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("RXBUF_ADDR_MODE", &["FULL", "FAST"]),
    ("RX_DATA_WIDTH", &["16", "20", "32", "40", "64", "80"]),
    ("RXOOB_CLK_CFG", &["PMA", "FABRIC"]),
    ("RXOUT_DIV", &["1", "2", "4", "8", "16"]),
    ("RXPLL_SEL", &["CPLL", "QPLL"]),
    ("RXSIPO_DIV_45", &["4", "5"]),
    ("RXSLIDE_MODE", &["#OFF", "AUTO", "PCS", "PMA"]),
    ("RX_XCLK_SEL", &["RXREC", "RXUSR"]),
    (
        "SATA_CPLL_CFG",
        &["VCO_3000MHZ", "VCO_750MHZ", "VCO_1500MHZ"],
    ),
    ("TX_DATA_WIDTH", &["16", "20", "32", "40", "64", "80"]),
    ("TX_DRIVE_MODE", &["DIRECT", "PIPE", "PIPEGEN3"]),
    ("TXOUT_DIV", &["1", "2", "4", "8", "16"]),
    ("TXPI_PPMCLK_SEL", &["TXUSRCLK2", "TXUSRCLK"]),
    ("TXPISO_DIV_45", &["4", "5"]),
    ("TXPLL_SEL", &["CPLL", "QPLL"]),
    ("TX_XCLK_SEL", &["TXUSR", "TXOUT"]),
];

const GTH_CHANNEL_ENUM_INT_ATTRS: &[(&str, Range<u32>, u32)] = &[
    ("CHAN_BOND_MAX_SKEW", 1..15, 0),
    ("CLK_COR_MAX_LAT", 3..61, 0),
    ("CLK_COR_MIN_LAT", 3..61, 0),
    ("RX_CLK25_DIV", 0..32, 1),
    ("RX_SIG_VALID_DLY", 0..32, 1),
    ("SAS_MAX_COM", 1..128, 0),
    ("SAS_MIN_COM", 1..64, 0),
    ("SATA_MAX_BURST", 1..64, 0),
    ("SATA_MAX_INIT", 1..64, 0),
    ("SATA_MAX_WAKE", 1..64, 0),
    ("SATA_MIN_BURST", 1..62, 0),
    ("SATA_MIN_INIT", 1..64, 0),
    ("SATA_MIN_WAKE", 1..64, 0),
    ("TX_CLK25_DIV", 0..32, 1),
];

const GTH_CHANNEL_DEC_ATTRS: &[(&str, usize)] = &[
    ("CLK_COR_REPEAT_WAIT", 5),
    ("RXBUF_THRESH_OVFLW", 6),
    ("RXBUF_THRESH_UNDFLW", 6),
    ("RX_INT_DATAWIDTH", 1),
    ("RXSLIDE_AUTO_WAIT", 4),
    ("TX_INT_DATAWIDTH", 1),
    ("TXOUTCLKPCS_SEL", 1),
];

const GTH_CHANNEL_BIN_ATTRS: &[(&str, usize)] = &[
    ("ACJTAG_DEBUG_MODE", 1),
    ("ACJTAG_MODE", 1),
    ("ACJTAG_RESET", 1),
    ("AEN_CPLL", 1),
    ("AEN_LOOPBACK", 1),
    ("AEN_MASTER", 1),
    ("AEN_PD_AND_EIDLE", 1),
    ("AEN_POLARITY", 1),
    ("AEN_PRBS", 1),
    ("AEN_QPI", 1),
    ("AEN_RESET", 1),
    ("AEN_RXCDR", 1),
    ("AEN_RXDFE", 1),
    ("AEN_RXDFELPM", 1),
    ("AEN_RXOUTCLK_SEL", 1),
    ("AEN_RXPHDLY", 1),
    ("AEN_RXSYSCLK_SEL", 1),
    ("AEN_TXOUTCLK_SEL", 1),
    ("AEN_TXPHDLY", 1),
    ("AEN_TXPI_PPM", 1),
    ("AEN_TXSYSCLK_SEL", 1),
    ("AEN_TX_DRIVE_MODE", 1),
    ("ALIGN_COMMA_ENABLE", 10),
    ("ALIGN_MCOMMA_VALUE", 10),
    ("ALIGN_PCOMMA_VALUE", 10),
    ("A_CFGRESET", 1),
    ("A_CPLLLOCKEN", 1),
    ("A_CPLLPD", 1),
    ("A_CPLLRESET", 1),
    ("A_EYESCANMODE", 1),
    ("A_EYESCANRESET", 1),
    ("A_GTRESETSEL", 1),
    ("A_GTRXRESET", 1),
    ("A_GTTXRESET", 1),
    ("A_LOOPBACK", 3),
    ("A_RXADAPTSELTEST", 14),
    ("A_RXBUFRESET", 1),
    ("A_RXCDRFREQRESET", 1),
    ("A_RXCDRHOLD", 1),
    ("A_RXCDROVRDEN", 1),
    ("A_RXCDRRESET", 1),
    ("A_RXCDRRESETRSV", 1),
    ("A_RXDFEAGCHOLD", 1),
    ("A_RXDFEAGCOVRDEN", 1),
    ("A_RXDFEAGCTRL", 5),
    ("A_RXDFECM1EN", 1),
    ("A_RXDFELFHOLD", 1),
    ("A_RXDFELFOVRDEN", 1),
    ("A_RXDFELPMRESET", 1),
    ("A_RXDFESLIDETAP", 5),
    ("A_RXDFESLIDETAPADAPTEN", 1),
    ("A_RXDFESLIDETAPHOLD", 1),
    ("A_RXDFESLIDETAPID", 6),
    ("A_RXDFESLIDETAPINITOVRDEN", 1),
    ("A_RXDFESLIDETAPONLYADAPTEN", 1),
    ("A_RXDFESLIDETAPOVRDEN", 1),
    ("A_RXDFESLIDETAPSTROBE", 1),
    ("A_RXDFETAP2HOLD", 1),
    ("A_RXDFETAP2OVRDEN", 1),
    ("A_RXDFETAP3HOLD", 1),
    ("A_RXDFETAP3OVRDEN", 1),
    ("A_RXDFETAP4HOLD", 1),
    ("A_RXDFETAP4OVRDEN", 1),
    ("A_RXDFETAP5HOLD", 1),
    ("A_RXDFETAP5OVRDEN", 1),
    ("A_RXDFETAP6HOLD", 1),
    ("A_RXDFETAP6OVRDEN", 1),
    ("A_RXDFETAP7HOLD", 1),
    ("A_RXDFETAP7OVRDEN", 1),
    ("A_RXDFEUTHOLD", 1),
    ("A_RXDFEUTOVRDEN", 1),
    ("A_RXDFEVPHOLD", 1),
    ("A_RXDFEVPOVRDEN", 1),
    ("A_RXDFEVSEN", 1),
    ("A_RXDFEXYDEN", 1),
    ("A_RXDLYBYPASS", 1),
    ("A_RXDLYEN", 1),
    ("A_RXDLYOVRDEN", 1),
    ("A_RXDLYSRESET", 1),
    ("A_RXLPMEN", 1),
    ("A_RXLPMHFHOLD", 1),
    ("A_RXLPMHFOVRDEN", 1),
    ("A_RXLPMLFHOLD", 1),
    ("A_RXLPMLFKLOVRDEN", 1),
    ("A_RXMONITORSEL", 2),
    ("A_RXOOBRESET", 1),
    ("A_RXOSCALRESET", 1),
    ("A_RXOSHOLD", 1),
    ("A_RXOSINTCFG", 4),
    ("A_RXOSINTEN", 1),
    ("A_RXOSINTHOLD", 1),
    ("A_RXOSINTID0", 4),
    ("A_RXOSINTNTRLEN", 1),
    ("A_RXOSINTOVRDEN", 1),
    ("A_RXOSINTSTROBE", 1),
    ("A_RXOSINTTESTOVRDEN", 1),
    ("A_RXOSOVRDEN", 1),
    ("A_RXOUTCLKSEL", 3),
    ("A_RXPCSRESET", 1),
    ("A_RXPD", 2),
    ("A_RXPHALIGN", 1),
    ("A_RXPHALIGNEN", 1),
    ("A_RXPHDLYPD", 1),
    ("A_RXPHDLYRESET", 1),
    ("A_RXPHOVRDEN", 1),
    ("A_RXPMARESET", 1),
    ("A_RXPOLARITY", 1),
    ("A_RXPRBSCNTRESET", 1),
    ("A_RXPRBSSEL", 3),
    ("A_RXSYSCLKSEL", 2),
    ("A_SPARE", 1),
    ("A_TXBUFDIFFCTRL", 3),
    ("A_TXDEEMPH", 1),
    ("A_TXDIFFCTRL", 4),
    ("A_TXDLYBYPASS", 1),
    ("A_TXDLYEN", 1),
    ("A_TXDLYOVRDEN", 1),
    ("A_TXDLYSRESET", 1),
    ("A_TXELECIDLE", 1),
    ("A_TXINHIBIT", 1),
    ("A_TXMAINCURSOR", 7),
    ("A_TXMARGIN", 3),
    ("A_TXOUTCLKSEL", 3),
    ("A_TXPCSRESET", 1),
    ("A_TXPD", 2),
    ("A_TXPHALIGN", 1),
    ("A_TXPHALIGNEN", 1),
    ("A_TXPHDLYPD", 1),
    ("A_TXPHDLYRESET", 1),
    ("A_TXPHINIT", 1),
    ("A_TXPHOVRDEN", 1),
    ("A_TXPIPPMOVRDEN", 1),
    ("A_TXPIPPMPD", 1),
    ("A_TXPIPPMSEL", 1),
    ("A_TXPMARESET", 1),
    ("A_TXPOLARITY", 1),
    ("A_TXPOSTCURSOR", 5),
    ("A_TXPOSTCURSORINV", 1),
    ("A_TXPRBSFORCEERR", 1),
    ("A_TXPRBSSEL", 3),
    ("A_TXPRECURSOR", 5),
    ("A_TXPRECURSORINV", 1),
    ("A_TXQPIBIASEN", 1),
    ("A_TXSWING", 1),
    ("A_TXSYSCLKSEL", 2),
    ("CFOK_CFG2", 6),
    ("CFOK_CFG3", 6),
    ("CHAN_BOND_SEQ_1_1", 10),
    ("CHAN_BOND_SEQ_1_2", 10),
    ("CHAN_BOND_SEQ_1_3", 10),
    ("CHAN_BOND_SEQ_1_4", 10),
    ("CHAN_BOND_SEQ_1_ENABLE", 4),
    ("CHAN_BOND_SEQ_2_1", 10),
    ("CHAN_BOND_SEQ_2_2", 10),
    ("CHAN_BOND_SEQ_2_3", 10),
    ("CHAN_BOND_SEQ_2_4", 10),
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
    ("CPLL_PCD_2UI_CFG", 1),
    ("ES_CLK_PHASE_SEL", 1),
    ("ES_CONTROL", 6),
    ("ES_PMA_CFG", 10),
    ("ES_PRESCALE", 5),
    ("ES_VERT_OFFSET", 9),
    ("FTS_DESKEW_SEQ_ENABLE", 4),
    ("FTS_LANE_DESKEW_CFG", 4),
    ("GEARBOX_MODE", 3),
    ("GT_INSTANTIATED", 1),
    ("LOOPBACK_CFG", 1),
    ("OUTREFCLK_SEL_INV", 2),
    ("PMA_POWER_SAVE", 10),
    ("PMA_RSV", 32),
    ("PMA_RSV2", 32),
    ("PMA_RSV3", 2),
    ("PMA_RSV4", 15),
    ("PMA_RSV5", 4),
    ("RESET_POWERSAVE_DISABLE", 1),
    ("RXBUFRESET_TIME", 5),
    ("RXBUF_EIDLE_HI_CNT", 4),
    ("RXBUF_EIDLE_LO_CNT", 4),
    ("RXCDRFREQRESET_TIME", 5),
    ("RXCDRPHRESET_TIME", 5),
    ("RXCDRRESET_TIME", 7),
    ("RXCDR_FR_RESET_ON_EIDLE", 1),
    ("RXCDR_HOLD_DURING_EIDLE", 1),
    ("RXCDR_LOCK_CFG", 6),
    ("RXCDR_PCIERESET_WAIT_TIME", 5),
    ("RXCDR_PH_RESET_ON_EIDLE", 1),
    ("RXDFELPMRESET_TIME", 7),
    ("RXISCANRESET_TIME", 5),
    ("RXLPM_HF_CFG", 14),
    ("RXLPM_LF_CFG", 18),
    ("RXOOB_CFG", 7),
    ("RXOSCALRESET_TIME", 5),
    ("RXOSCALRESET_TIMEOUT", 5),
    ("RXPCSRESET_TIME", 5),
    ("RXPH_MONITOR_SEL", 5),
    ("RXPI_CFG0", 2),
    ("RXPI_CFG1", 2),
    ("RXPI_CFG2", 2),
    ("RXPI_CFG3", 2),
    ("RXPI_CFG4", 1),
    ("RXPI_CFG5", 1),
    ("RXPI_CFG6", 3),
    ("RXPMARESET_TIME", 5),
    ("RXPRBS_ERR_LOOPBACK", 1),
    ("RXSYNC_MULTILANE", 1),
    ("RXSYNC_OVRD", 1),
    ("RXSYNC_SKIP_DA", 1),
    ("RX_BIAS_CFG", 24),
    ("RX_BUFFER_CFG", 6),
    ("RX_CLKMUX_PD", 1),
    ("RX_CM_SEL", 2),
    ("RX_CM_TRIM", 4),
    ("RX_DDI_SEL", 6),
    ("RX_DEBUG_CFG", 14),
    ("RX_DFELPM_CFG0", 4),
    ("RX_DFELPM_CFG1", 1),
    ("RX_DFELPM_KLKH_AGC_STUP_EN", 1),
    ("RX_DFE_AGC_CFG0", 2),
    ("RX_DFE_AGC_CFG1", 3),
    ("RX_DFE_AGC_CFG2", 4),
    ("RX_DFE_AGC_OVRDEN", 1),
    ("RX_DFE_H2_CFG", 12),
    ("RX_DFE_H3_CFG", 12),
    ("RX_DFE_H4_CFG", 11),
    ("RX_DFE_H5_CFG", 11),
    ("RX_DFE_H6_CFG", 11),
    ("RX_DFE_H7_CFG", 11),
    ("RX_DFE_KL_CFG", 33),
    ("RX_DFE_KL_LPM_KH_CFG0", 2),
    ("RX_DFE_KL_LPM_KH_CFG1", 3),
    ("RX_DFE_KL_LPM_KH_CFG2", 4),
    ("RX_DFE_KL_LPM_KH_OVRDEN", 1),
    ("RX_DFE_KL_LPM_KL_CFG0", 2),
    ("RX_DFE_KL_LPM_KL_CFG1", 3),
    ("RX_DFE_KL_LPM_KL_CFG2", 4),
    ("RX_DFE_KL_LPM_KL_OVRDEN", 1),
    ("RX_DFE_LPM_HOLD_DURING_EIDLE", 1),
    ("RX_DFE_UT_CFG", 17),
    ("RX_DFE_VP_CFG", 17),
    ("RX_DFE_VS_CFG", 9),
    ("RX_OS_CFG", 13),
    ("SATA_BURST_SEQ_LEN", 4),
    ("SATA_BURST_VAL", 3),
    ("SATA_EIDLE_VAL", 3),
    ("SP_REFCLK_CFG", 3),
    ("TERM_RCAL_CFG", 15),
    ("TERM_RCAL_OVRD", 3),
    ("TXOOB_CFG", 1),
    ("TXPCSRESET_TIME", 5),
    ("TXPH_MONITOR_SEL", 5),
    ("TXPI_CFG0", 2),
    ("TXPI_CFG1", 2),
    ("TXPI_CFG2", 2),
    ("TXPI_CFG3", 1),
    ("TXPI_CFG4", 1),
    ("TXPI_CFG5", 3),
    ("TXPI_GREY_SEL", 1),
    ("TXPI_INVSTROBE_SEL", 1),
    ("TXPI_PPM_CFG", 8),
    ("TXPI_SYNFREQ_PPM", 3),
    ("TXPMARESET_TIME", 5),
    ("TXSYNC_MULTILANE", 1),
    ("TXSYNC_OVRD", 1),
    ("TXSYNC_SKIP_DA", 1),
    ("TX_CLKMUX_PD", 1),
    ("TX_DEEMPH0", 6),
    ("TX_DEEMPH1", 6),
    ("TX_EIDLE_ASSERT_DELAY", 3),
    ("TX_EIDLE_DEASSERT_DELAY", 3),
    ("TX_MAINCURSOR_SEL", 1),
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
    ("TX_QPI_STATUS_EN", 1),
    ("TX_RXDETECT_REF", 3),
    ("UCODEER_CLR", 1),
    ("USE_PCS_CLK_PHASE_SEL", 1),
];

const GTH_CHANNEL_HEX_ATTRS: &[(&str, usize)] = &[
    ("ADAPT_CFG0", 20),
    ("AMONITOR_CFG", 16),
    ("CFOK_CFG", 42),
    ("CPLL_CFG", 29),
    ("CPLL_INIT_CFG", 24),
    ("CPLL_LOCK_CFG", 16),
    ("DMONITOR_CFG", 24),
    ("ES_HORZ_OFFSET", 12),
    ("ES_QUALIFIER", 80),
    ("ES_QUAL_MASK", 80),
    ("ES_SDATA_MASK", 80),
    ("PCS_RSVD_ATTR", 48),
    ("PD_TRANS_TIME_FROM_P2", 12),
    ("PD_TRANS_TIME_NONE_P2", 8),
    ("PD_TRANS_TIME_TO_P2", 8),
    ("RXCDR_CFG", 83),
    ("RXDLY_CFG", 16),
    ("RXDLY_LCFG", 9),
    ("RXDLY_TAP_CFG", 16),
    ("RXPHDLY_CFG", 24),
    ("RXPH_CFG", 24),
    ("RX_DFE_GAIN_CFG", 23),
    ("RX_DFE_LPM_CFG", 16),
    ("RX_DFE_ST_CFG", 54),
    ("TRANS_TIME_RATE", 8),
    ("TST_RSV", 32),
    ("TXDLY_CFG", 16),
    ("TXDLY_LCFG", 9),
    ("TXDLY_TAP_CFG", 16),
    ("TXPHDLY_CFG", 24),
    ("TXPH_CFG", 16),
    ("TX_RXDETECT_CFG", 14),
    ("TX_RXDETECT_PRECHARGE_TIME", 17),
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
            if let Some(col_io) = edev.col_lio
                && tcrd.col < col_io
            {
                tgt_col_cmt = Some(col_io + 1);
            }
            if let Some((col_gt, _)) = edev.col_mgt {
                let gtcol = chip.get_col_gt(col_gt).unwrap();
                if tcrd.col > col_gt && gtcol.regs[chip.row_to_reg(tcrd.row)].is_some() {
                    tgt_col_gt = Some(col_gt);
                }
            }
        } else {
            if let Some(col_io) = edev.col_rio
                && tcrd.col > col_io
            {
                tgt_col_cmt = Some(col_io - 1);
            }
            if let Some((_, col_gt)) = edev.col_mgt {
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
            let lr = if tcrd.col < edev.col_clk { 'L' } else { 'R' };
            let clk_hrow = tcrd.with_col(edev.col_clk).tile(defs::tslots::BEL);
            let clk_hrow_bel = clk_hrow.cell.bel(defs::bslots::CLK_HROW_V7);
            let (ta, wa) = PipWire::BelPinNear(defs::bslots::CLK_HROW_V7, format!("HIN{idx}_{lr}"))
                .resolve(backend, clk_hrow)?;
            let (tb, wb) = PipWire::BelPinNear(defs::bslots::CLK_HROW_V7, format!("CASCO{idx}"))
                .resolve(backend, clk_hrow)?;
            assert_eq!(ta, tb);

            fuzzer = fuzzer
                .base(Key::TileMutex(clk_hrow, format!("HIN{idx}_{lr}")), "USE")
                .base(
                    Key::BelMutex(clk_hrow_bel, format!("MUX.CASCO{idx}")),
                    format!("HIN{idx}_{lr}"),
                )
                .base(Key::BelMutex(clk_hrow_bel, "CASCO".into()), "CASCO")
                .base(Key::Pip(ta, wa, wb), true);
        }

        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for tile in ["GTP_COMMON", "GTP_COMMON_MID"] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let mut bctx = ctx.bel(defs::bslots::GTP_COMMON);
        let mode = "GTPE2_COMMON";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for &pin in GTP_COMMON_INVPINS {
            bctx.mode(mode).test_inv(pin);
        }
        for &(attr, vals) in GTP_COMMON_ENUM_ATTRS {
            bctx.mode(mode).test_enum(attr, vals);
        }
        for &(attr, width) in GTP_COMMON_BIN_ATTRS {
            bctx.mode(mode).test_multi_attr_bin(attr, width);
        }
        for &(attr, width) in GTP_COMMON_HEX_ATTRS {
            bctx.mode(mode).test_multi_attr_hex(attr, width);
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "GTX_COMMON") {
        let mut bctx = ctx.bel(defs::bslots::GTX_COMMON);
        let mode = "GTXE2_COMMON";

        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for &pin in GTXH_COMMON_INVPINS {
            bctx.mode(mode).test_inv(pin);
        }
        for &(attr, vals) in GTXH_COMMON_ENUM_ATTRS {
            bctx.mode(mode).test_enum(attr, vals);
        }
        for &(attr, width) in GTX_COMMON_BIN_ATTRS {
            bctx.mode(mode).test_multi_attr_bin(attr, width);
        }
        for &(attr, width) in GTX_COMMON_HEX_ATTRS {
            bctx.mode(mode).test_multi_attr_hex(attr, width);
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "GTH_COMMON") {
        let mut bctx = ctx.bel(defs::bslots::GTH_COMMON);
        let mode = "GTHE2_COMMON";

        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for &pin in GTXH_COMMON_INVPINS {
            bctx.mode(mode).test_inv(pin);
        }
        for &(attr, vals) in GTXH_COMMON_ENUM_ATTRS {
            bctx.mode(mode).test_enum(attr, vals);
        }
        for &(attr, width) in GTH_COMMON_BIN_ATTRS {
            bctx.mode(mode).test_multi_attr_bin(attr, width);
        }
        for &(attr, width) in GTH_COMMON_HEX_ATTRS {
            bctx.mode(mode).test_multi_attr_hex(attr, width);
        }
    }
    for (tile, bel) in [
        ("GTX_COMMON", defs::bslots::GTX_COMMON),
        ("GTH_COMMON", defs::bslots::GTH_COMMON),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let mut bctx = ctx.bel(bel);
        for pin in [
            "GTREFCLK0",
            "GTREFCLK1",
            "GTNORTHREFCLK0",
            "GTNORTHREFCLK1",
            "GTSOUTHREFCLK0",
            "GTSOUTHREFCLK1",
            "GTGREFCLK",
        ] {
            bctx.build()
                .mutex("QPLLREFCLKSEL_STATIC", pin)
                .test_manual("QPLLREFCLKSEL_STATIC", pin)
                .pip(pin, (PinFar, pin))
                .commit();
        }
        bctx.build()
            .mutex("QPLLREFCLKSEL_STATIC", "MODE")
            .pip("GTGREFCLK", (PinFar, "GTGREFCLK"))
            .test_manual("QPLLREFCLKSEL_MODE", "DYNAMIC")
            .pip("GTREFCLK0", (PinFar, "GTREFCLK0"))
            .commit();
        let tcid = backend.edev.db.get_tile_class(tile);
        if backend.edev.tile_index[tcid].len() > 1 {
            for i in 0..2 {
                bctx.build()
                    .mutex(format!("MUX.NORTHREFCLK{i}_N"), format!("NORTHREFCLK{i}"))
                    .has_related(Delta::new(0, 50, tile))
                    .test_manual(format!("MUX.NORTHREFCLK{i}_N"), format!("NORTHREFCLK{i}"))
                    .related_pip(
                        Delta::new(0, 25, "BRKH_GTX"),
                        (defs::bslots::BRKH_GTX, format!("NORTHREFCLK{i}_U")),
                        (defs::bslots::BRKH_GTX, format!("NORTHREFCLK{i}_D")),
                    )
                    .commit();
                for j in 0..2 {
                    bctx.build()
                        .mutex(format!("MUX.NORTHREFCLK{i}_N"), format!("REFCLK{j}"))
                        .has_related(Delta::new(0, 50, tile))
                        .test_manual(format!("MUX.NORTHREFCLK{i}_N"), format!("REFCLK{j}"))
                        .related_pip(
                            Delta::new(0, 25, "BRKH_GTX"),
                            (defs::bslots::BRKH_GTX, format!("NORTHREFCLK{i}_U")),
                            (defs::bslots::BRKH_GTX, format!("REFCLK{j}_D")),
                        )
                        .commit();
                }
                bctx.build()
                    .mutex(format!("MUX.SOUTHREFCLK{i}_S"), format!("SOUTHREFCLK{i}"))
                    .has_related(Delta::new(0, -50, tile))
                    .test_manual(format!("MUX.SOUTHREFCLK{i}_S"), format!("SOUTHREFCLK{i}"))
                    .related_pip(
                        Delta::new(0, -25, "BRKH_GTX"),
                        (defs::bslots::BRKH_GTX, format!("SOUTHREFCLK{i}_D")),
                        (defs::bslots::BRKH_GTX, format!("SOUTHREFCLK{i}_U")),
                    )
                    .commit();
                for j in 0..2 {
                    bctx.build()
                        .mutex(format!("MUX.SOUTHREFCLK{i}_S"), format!("REFCLK{j}"))
                        .has_related(Delta::new(0, -50, tile))
                        .test_manual(format!("MUX.SOUTHREFCLK{i}_S"), format!("REFCLK{j}"))
                        .related_pip(
                            Delta::new(0, -25, "BRKH_GTX"),
                            (defs::bslots::BRKH_GTX, format!("SOUTHREFCLK{i}_D")),
                            (defs::bslots::BRKH_GTX, format!("REFCLK{j}_U")),
                        )
                        .commit();
                }
            }
        }
    }
    for tile in ["GTP_COMMON", "GTP_COMMON_MID", "GTX_COMMON", "GTH_COMMON"] {
        for i in 0..2 {
            let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
                continue;
            };
            let mut bctx = ctx.bel(defs::bslots::BUFDS[i]);
            let mode = "IBUFDS_GTE2";
            bctx.test_manual("PRESENT", "1").mode(mode).commit();
            bctx.mode(mode).test_inv("CLKTESTSIG");
            bctx.mode(mode).test_enum("CLKCM_CFG", &["FALSE", "TRUE"]);
            bctx.mode(mode).test_enum("CLKRCV_TRST", &["FALSE", "TRUE"]);
            bctx.mode(mode)
                .tile_mutex("CLKSWING_CFG", format!("IBUFDS{i}"))
                .test_multi_attr_bin("CLKSWING_CFG", 2);
            for pin in ["O", "ODIV2"] {
                bctx.mode(mode)
                    .mutex("MUX.MGTCLKOUT", pin)
                    .test_manual("MUX.MGTCLKOUT", pin)
                    .pip("MGTCLKOUT", pin)
                    .commit();
            }
            bctx.mode(mode)
                .mutex("MUX.MGTCLKOUT", "CLKTESTSIG")
                .test_manual("MUX.MGTCLKOUT", "CLKTESTSIG")
                .pin_pips("CLKTESTSIG")
                .commit();
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "GTP_COMMON_MID") {
        let mut bctx = ctx.bel(defs::bslots::GTP_COMMON);
        for i in 0..14 {
            let oi = i ^ 1;
            for j in [i, oi] {
                bctx.build()
                    .tile_mutex("HIN", "USE")
                    .prop(TouchHout(i))
                    .prop(TouchHout(oi))
                    .mutex(format!("MUX.HOUT{i}"), format!("HIN{j}"))
                    .mutex(format!("MUX.HOUT{oi}"), format!("HIN{j}"))
                    .pip(format!("HOUT{oi}"), format!("HIN{j}"))
                    .test_manual(format!("MUX.HOUT{i}"), format!("HIN{j}"))
                    .pip(format!("HOUT{i}"), format!("HIN{j}"))
                    .commit();
                if i == j {
                    bctx.build()
                        .tile_mutex("HIN", "TEST")
                        .prop(TouchHout(i))
                        .mutex(format!("MUX.HOUT{i}"), format!("HIN{j}"))
                        .test_manual(format!("MUX.HOUT{i}"), format!("HIN{j}.EXCL"))
                        .pip(format!("HOUT{i}"), format!("HIN{j}"))
                        .commit();
                }
            }
            for pin in [
                "RXOUTCLK0",
                "RXOUTCLK1",
                "RXOUTCLK2",
                "RXOUTCLK3",
                "TXOUTCLK0",
                "TXOUTCLK1",
                "TXOUTCLK2",
                "TXOUTCLK3",
                "MGTCLKOUT0",
                "MGTCLKOUT1",
            ] {
                bctx.build()
                    .tile_mutex("HIN", "USE")
                    .prop(TouchHout(i))
                    .prop(TouchHout(oi))
                    .mutex(format!("MUX.HOUT{i}"), pin)
                    .mutex(format!("MUX.HOUT{oi}"), pin)
                    .pip(format!("HOUT{oi}"), format!("{pin}_BUF"))
                    .test_manual(format!("MUX.HOUT{i}"), pin)
                    .pip(format!("HOUT{i}"), format!("{pin}_BUF"))
                    .commit();
                if i == 0 {
                    bctx.build()
                        .tile_mutex("HIN", "TEST")
                        .prop(TouchHout(i))
                        .mutex(format!("MUX.HOUT{i}"), pin)
                        .test_manual(format!("MUX.HOUT{i}"), format!("{pin}.EXCL"))
                        .pip(format!("HOUT{i}"), format!("{pin}_BUF"))
                        .commit();
                }
            }
        }
    }

    for tile in ["GTP_CHANNEL", "GTP_CHANNEL_MID"] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let mut bctx = ctx.bel(defs::bslots::GTP_CHANNEL);
        let mode = "GTPE2_CHANNEL";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for &pin in GTP_CHANNEL_INVPINS {
            bctx.mode(mode).test_inv(pin);
        }
        for &attr in GTP_CHANNEL_BOOL_ATTRS {
            bctx.mode(mode).test_enum(attr, &["FALSE", "TRUE"]);
        }
        for &(attr, vals) in GTP_CHANNEL_ENUM_ATTRS {
            bctx.mode(mode).test_enum(attr, vals);
        }
        for &(attr, ref vals, delta) in GTP_CHANNEL_ENUM_INT_ATTRS {
            let vals = Vec::from_iter(vals.clone().map(|i| (i + delta).to_string()));
            bctx.mode(mode).test_enum(attr, &vals);
        }
        for &(attr, width) in GTP_CHANNEL_DEC_ATTRS {
            bctx.mode(mode).test_multi_attr_dec(attr, width);
        }
        for &(attr, width) in GTP_CHANNEL_BIN_ATTRS {
            bctx.mode(mode).test_multi_attr_bin(attr, width);
        }
        for &(attr, width) in GTP_CHANNEL_HEX_ATTRS {
            bctx.mode(mode).test_multi_attr_hex(attr, width);
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "GTX_CHANNEL") {
        let mut bctx = ctx.bel(defs::bslots::GTX_CHANNEL);
        let mode = "GTXE2_CHANNEL";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for &pin in GTX_CHANNEL_INVPINS {
            bctx.mode(mode).test_inv(pin);
        }
        for &attr in GTX_CHANNEL_BOOL_ATTRS {
            bctx.mode(mode).test_enum(attr, &["FALSE", "TRUE"]);
        }
        for &(attr, vals) in GTX_CHANNEL_ENUM_ATTRS {
            bctx.mode(mode).test_enum(attr, vals);
        }
        for &(attr, ref vals, delta) in GTX_CHANNEL_ENUM_INT_ATTRS {
            let vals = Vec::from_iter(vals.clone().map(|i| (i + delta).to_string()));
            bctx.mode(mode).test_enum(attr, &vals);
        }
        for &(attr, width) in GTX_CHANNEL_DEC_ATTRS {
            bctx.mode(mode).test_multi_attr_dec(attr, width);
        }
        for &(attr, width) in GTX_CHANNEL_BIN_ATTRS {
            bctx.mode(mode).test_multi_attr_bin(attr, width);
        }
        for &(attr, width) in GTX_CHANNEL_HEX_ATTRS {
            bctx.mode(mode).test_multi_attr_hex(attr, width);
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "GTH_CHANNEL") {
        let mut bctx = ctx.bel(defs::bslots::GTH_CHANNEL);
        let mode = "GTHE2_CHANNEL";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for &pin in GTH_CHANNEL_INVPINS {
            bctx.mode(mode).test_inv(pin);
        }
        for &attr in GTH_CHANNEL_BOOL_ATTRS {
            bctx.mode(mode).test_enum(attr, &["FALSE", "TRUE"]);
        }
        for &(attr, vals) in GTH_CHANNEL_ENUM_ATTRS {
            bctx.mode(mode).test_enum(attr, vals);
        }
        for &(attr, ref vals, delta) in GTH_CHANNEL_ENUM_INT_ATTRS {
            let vals = Vec::from_iter(vals.clone().map(|i| (i + delta).to_string()));
            bctx.mode(mode).test_enum(attr, &vals);
        }
        for &(attr, width) in GTH_CHANNEL_DEC_ATTRS {
            bctx.mode(mode).test_multi_attr_dec(attr, width);
        }
        for &(attr, width) in GTH_CHANNEL_BIN_ATTRS {
            bctx.mode(mode).test_multi_attr_bin(attr, width);
        }
        for &(attr, width) in GTH_CHANNEL_HEX_ATTRS {
            bctx.mode(mode).test_multi_attr_hex(attr, width);
        }
    }
    for (tile, bel) in [
        ("GTX_CHANNEL", defs::bslots::GTX_CHANNEL),
        ("GTH_CHANNEL", defs::bslots::GTH_CHANNEL),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        let mut bctx = ctx.bel(bel);
        for pin in [
            "GTREFCLK0",
            "GTREFCLK1",
            "GTNORTHREFCLK0",
            "GTNORTHREFCLK1",
            "GTSOUTHREFCLK0",
            "GTSOUTHREFCLK1",
            "GTGREFCLK",
        ] {
            bctx.build()
                .mutex("CPLLREFCLKSEL_STATIC", pin)
                .test_manual("CPLLREFCLKSEL_STATIC", pin)
                .pip(pin, (PinFar, pin))
                .commit();
        }
        bctx.build()
            .mutex("CPLLREFCLKSEL_STATIC", "MODE")
            .pip("GTGREFCLK", (PinFar, "GTGREFCLK"))
            .test_manual("CPLLREFCLKSEL_MODE", "DYNAMIC")
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

    for tile in ["GTP_COMMON", "GTP_COMMON_MID"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "GTP_COMMON";
        ctx.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for &pin in GTP_COMMON_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for &(attr, vals) in GTP_COMMON_ENUM_ATTRS {
            ctx.collect_enum_ocd(tile, bel, attr, vals, OcdMode::BitOrderDrpV6);
        }
        for &(attr, _) in GTP_COMMON_BIN_ATTRS {
            if matches!(
                attr,
                "EAST_REFCLK0_SEL" | "EAST_REFCLK1_SEL" | "WEST_REFCLK0_SEL" | "WEST_REFCLK1_SEL"
            ) {
                let [diff0, diff1] = ctx.get_diffs(tile, bel, attr, "").try_into().unwrap();
                ctx.insert(
                    tile,
                    bel,
                    attr,
                    xlat_enum(vec![
                        ("NONE", Diff::default()),
                        ("REFCLK0", diff0),
                        ("REFCLK1", diff1),
                    ]),
                );
            } else {
                ctx.collect_bitvec(tile, bel, attr, "");
            }
        }
        for &(attr, _) in GTP_COMMON_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        // too annoying to bother fuzzing cleanly, given that east/west clocks are only present on 7a200t
        ctx.insert(
            tile,
            bel,
            "PLL0REFCLKSEL_STATIC",
            TileItem {
                bits: vec![
                    common_drp_bit(tile == "GTP_COMMON_MID", 0x03, 28),
                    common_drp_bit(tile == "GTP_COMMON_MID", 0x03, 29),
                    common_drp_bit(tile == "GTP_COMMON_MID", 0x03, 30),
                ],
                kind: TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("NONE".to_string(), bits![0, 0, 0]),
                        ("GTREFCLK0".to_string(), bits![1, 0, 0]),
                        ("GTREFCLK1".to_string(), bits![0, 1, 0]),
                        ("GTEASTREFCLK0".to_string(), bits![1, 1, 0]),
                        ("GTEASTREFCLK1".to_string(), bits![0, 0, 1]),
                        ("GTWESTREFCLK0".to_string(), bits![1, 0, 1]),
                        ("GTWESTREFCLK1".to_string(), bits![0, 1, 1]),
                        ("GTGREFCLK0".to_string(), bits![1, 1, 1]),
                    ]),
                },
            },
        );
        ctx.insert(
            tile,
            bel,
            "PLL0REFCLKSEL_MODE",
            TileItem {
                bits: vec![common_drp_bit(tile == "GTP_COMMON_MID", 0x03, 31)],
                kind: TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("STATIC".to_string(), bits![0]),
                        ("DYNAMIC".to_string(), bits![1]),
                    ]),
                },
            },
        );
        ctx.insert(
            tile,
            bel,
            "PLL1REFCLKSEL_STATIC",
            TileItem {
                bits: vec![
                    common_drp_bit(tile == "GTP_COMMON_MID", 0x2d, 28),
                    common_drp_bit(tile == "GTP_COMMON_MID", 0x2d, 29),
                    common_drp_bit(tile == "GTP_COMMON_MID", 0x2d, 30),
                ],
                kind: TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("NONE".to_string(), bits![0, 0, 0]),
                        ("GTREFCLK0".to_string(), bits![1, 0, 0]),
                        ("GTREFCLK1".to_string(), bits![0, 1, 0]),
                        ("GTEASTREFCLK0".to_string(), bits![1, 1, 0]),
                        ("GTEASTREFCLK1".to_string(), bits![0, 0, 1]),
                        ("GTWESTREFCLK0".to_string(), bits![1, 0, 1]),
                        ("GTWESTREFCLK1".to_string(), bits![0, 1, 1]),
                        ("GTGREFCLK1".to_string(), bits![1, 1, 1]),
                    ]),
                },
            },
        );
        ctx.insert(
            tile,
            bel,
            "PLL1REFCLKSEL_MODE",
            TileItem {
                bits: vec![common_drp_bit(tile == "GTP_COMMON_MID", 0x2d, 31)],
                kind: TileItemKind::Enum {
                    values: BTreeMap::from_iter([
                        ("STATIC".to_string(), bits![0]),
                        ("DYNAMIC".to_string(), bits![1]),
                    ]),
                },
            },
        );
    }
    if ctx.has_tile("GTX_COMMON") {
        let tile = "GTX_COMMON";
        let bel = "GTX_COMMON";
        ctx.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for &pin in GTXH_COMMON_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for &(attr, vals) in GTXH_COMMON_ENUM_ATTRS {
            ctx.collect_enum_ocd(tile, bel, attr, vals, OcdMode::BitOrderDrpV6);
        }
        for &(attr, _) in GTX_COMMON_BIN_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTX_COMMON_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    if ctx.has_tile("GTH_COMMON") {
        let tile = "GTH_COMMON";
        let bel = "GTH_COMMON";
        ctx.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for &pin in GTXH_COMMON_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for &(attr, vals) in GTXH_COMMON_ENUM_ATTRS {
            ctx.collect_enum_ocd(tile, bel, attr, vals, OcdMode::BitOrderDrpV6);
        }
        for &(attr, _) in GTH_COMMON_BIN_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTH_COMMON_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    for tile in ["GTX_COMMON", "GTH_COMMON"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = tile;
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "QPLLREFCLKSEL_STATIC",
            &[
                "GTREFCLK0",
                "GTREFCLK1",
                "GTNORTHREFCLK0",
                "GTNORTHREFCLK1",
                "GTSOUTHREFCLK0",
                "GTSOUTHREFCLK1",
                "GTGREFCLK",
            ],
            "NONE",
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum_default(tile, bel, "QPLLREFCLKSEL_MODE", &["DYNAMIC"], "STATIC");
        let tcid = ctx.edev.db.get_tile_class(tile);
        if ctx.edev.tile_index[tcid].len() > 1 {
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                "MUX.NORTHREFCLK0_N",
                &["REFCLK0", "REFCLK1", "NORTHREFCLK0"],
                "NONE",
                OcdMode::BitOrderDrpV6,
            );
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                "MUX.NORTHREFCLK1_N",
                &["REFCLK0", "REFCLK1", "NORTHREFCLK1"],
                "NONE",
                OcdMode::BitOrderDrpV6,
            );
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                "MUX.SOUTHREFCLK0_S",
                &["REFCLK0", "REFCLK1", "SOUTHREFCLK0"],
                "NONE",
                OcdMode::BitOrderDrpV6,
            );
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                "MUX.SOUTHREFCLK1_S",
                &["REFCLK0", "REFCLK1", "SOUTHREFCLK1"],
                "NONE",
                OcdMode::BitOrderDrpV6,
            );
        }
    }
    for (tile, bel_common) in [
        ("GTP_COMMON", "GTP_COMMON"),
        ("GTP_COMMON_MID", "GTP_COMMON"),
        ("GTX_COMMON", "GTX_COMMON"),
        ("GTH_COMMON", "GTH_COMMON"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        for reg in 0..0x60 {
            ctx.insert(
                tile,
                bel_common,
                format!("DRP{reg:02X}"),
                TileItem::from_bitvec(
                    (0..16)
                        .map(|bit| common_drp_bit(tile == "GTP_COMMON_MID", reg, bit))
                        .collect(),
                    false,
                ),
            );
        }
        for bel in ["BUFDS[0]", "BUFDS[1]"] {
            ctx.collect_inv(tile, bel, "CLKTESTSIG");
            ctx.collect_enum_bool(tile, bel, "CLKCM_CFG", "FALSE", "TRUE");
            ctx.collect_enum_bool(tile, bel, "CLKRCV_TRST", "FALSE", "TRUE");
            let item = ctx.extract_bitvec(tile, bel, "CLKSWING_CFG", "");
            ctx.insert(tile, bel_common, "CLKSWING_CFG", item);
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                "MUX.MGTCLKOUT",
                &["O", "ODIV2", "CLKTESTSIG"],
                "NONE",
                OcdMode::BitOrderDrpV6,
            );
            let mut diff = ctx.get_diff(tile, bel, "PRESENT", "1");
            diff.apply_enum_diff(ctx.item(tile, bel, "MUX.MGTCLKOUT"), "NONE", "O");
            diff.assert_empty();
        }
    }
    if ctx.has_tile("GTP_COMMON_MID") {
        let tile = "GTP_COMMON_MID";
        let bel = "GTP_COMMON";
        for i in 0..14 {
            let diff = ctx
                .get_diff(tile, bel, format!("MUX.HOUT{i}"), format!("HIN{i}.EXCL"))
                .combine(&!ctx.peek_diff(tile, bel, format!("MUX.HOUT{i}"), format!("HIN{i}")));
            ctx.insert(
                tile,
                "HCLK_GTP_MID",
                format!("ENABLE.HIN{i}"),
                xlat_bit(diff),
            );
        }
        for pin in [
            "RXOUTCLK0",
            "RXOUTCLK1",
            "RXOUTCLK2",
            "RXOUTCLK3",
            "TXOUTCLK0",
            "TXOUTCLK1",
            "TXOUTCLK2",
            "TXOUTCLK3",
            "MGTCLKOUT0",
            "MGTCLKOUT1",
        ] {
            let diff = ctx
                .get_diff(tile, bel, "MUX.HOUT0", format!("{pin}.EXCL"))
                .combine(&!ctx.peek_diff(tile, bel, "MUX.HOUT0", pin));
            ctx.insert(
                tile,
                "HCLK_GTP_MID",
                format!("ENABLE.{pin}"),
                xlat_bit(diff),
            );
        }
        for i in 0..14 {
            let item = ctx.extract_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.HOUT{i}"),
                &[
                    "RXOUTCLK0".to_string(),
                    "RXOUTCLK1".to_string(),
                    "RXOUTCLK2".to_string(),
                    "RXOUTCLK3".to_string(),
                    "TXOUTCLK0".to_string(),
                    "TXOUTCLK1".to_string(),
                    "TXOUTCLK2".to_string(),
                    "TXOUTCLK3".to_string(),
                    "MGTCLKOUT0".to_string(),
                    "MGTCLKOUT1".to_string(),
                    format!("HIN{i}"),
                    format!("HIN{ii}", ii = i ^ 1),
                ],
                "NONE",
                OcdMode::Mux,
            );
            ctx.insert(tile, "HCLK_GTP_MID", format!("MUX.HOUT{i}"), item);
        }
        // ... seem glued together in fuzzing? screw this. manual time.
        ctx.insert(
            tile,
            "HCLK_GTP_MID",
            "DRP_MASK_BELOW",
            TileItem::from_bit(TileBit::new(6, 0, 13), false),
        );
        ctx.insert(
            tile,
            "HCLK_GTP_MID",
            "DRP_MASK_ABOVE",
            TileItem::from_bit(TileBit::new(6, 1, 13), false),
        );
    }
    for (tile, bel) in [
        ("GTP_CHANNEL", "GTP_CHANNEL"),
        ("GTP_CHANNEL_MID", "GTP_CHANNEL"),
        ("GTX_CHANNEL", "GTX_CHANNEL"),
        ("GTH_CHANNEL", "GTH_CHANNEL"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        for reg in 0..0xb0 {
            ctx.insert(
                tile,
                bel,
                format!("DRP{reg:02X}"),
                TileItem::from_bitvec(
                    (0..16)
                        .map(|bit| channel_drp_bit(tile == "GTP_CHANNEL_MID", reg, bit))
                        .collect(),
                    false,
                ),
            );
        }
    }

    for tile in ["GTP_CHANNEL", "GTP_CHANNEL_MID"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "GTP_CHANNEL";
        ctx.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for &pin in GTP_CHANNEL_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for &attr in GTP_CHANNEL_BOOL_ATTRS {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for &(attr, vals) in GTP_CHANNEL_ENUM_ATTRS {
            ctx.collect_enum_ocd(tile, bel, attr, vals, OcdMode::BitOrderDrpV6);
        }
        for &(attr, ref vals, delta) in GTP_CHANNEL_ENUM_INT_ATTRS {
            ctx.collect_enum_int(tile, bel, attr, vals.clone(), delta);
        }
        for &(attr, _) in GTP_CHANNEL_DEC_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTP_CHANNEL_BIN_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTP_CHANNEL_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    if ctx.has_tile("GTX_CHANNEL") {
        let tile = "GTX_CHANNEL";
        let bel = "GTX_CHANNEL";
        ctx.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for &pin in GTX_CHANNEL_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for &attr in GTX_CHANNEL_BOOL_ATTRS {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for &(attr, vals) in GTX_CHANNEL_ENUM_ATTRS {
            ctx.collect_enum_ocd(tile, bel, attr, vals, OcdMode::BitOrderDrpV6);
        }
        for &(attr, ref vals, delta) in GTX_CHANNEL_ENUM_INT_ATTRS {
            ctx.collect_enum_int(tile, bel, attr, vals.clone(), delta);
        }
        for &(attr, _) in GTX_CHANNEL_DEC_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTX_CHANNEL_BIN_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTX_CHANNEL_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    if ctx.has_tile("GTH_CHANNEL") {
        let tile = "GTH_CHANNEL";
        let bel = "GTH_CHANNEL";
        ctx.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for &pin in GTH_CHANNEL_INVPINS {
            ctx.collect_inv(tile, bel, pin);
        }
        for &attr in GTH_CHANNEL_BOOL_ATTRS {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for &(attr, vals) in GTH_CHANNEL_ENUM_ATTRS {
            ctx.collect_enum_ocd(tile, bel, attr, vals, OcdMode::BitOrderDrpV6);
        }
        for &(attr, ref vals, delta) in GTH_CHANNEL_ENUM_INT_ATTRS {
            ctx.collect_enum_int(tile, bel, attr, vals.clone(), delta);
        }
        for &(attr, _) in GTH_CHANNEL_DEC_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTH_CHANNEL_BIN_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GTH_CHANNEL_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
    }
    for tile in ["GTX_CHANNEL", "GTH_CHANNEL"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = tile;
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "CPLLREFCLKSEL_STATIC",
            &[
                "GTREFCLK0",
                "GTREFCLK1",
                "GTNORTHREFCLK0",
                "GTNORTHREFCLK1",
                "GTSOUTHREFCLK0",
                "GTSOUTHREFCLK1",
                "GTGREFCLK",
            ],
            "NONE",
            OcdMode::BitOrderDrpV6,
        );
        ctx.collect_enum_default(tile, bel, "CPLLREFCLKSEL_MODE", &["DYNAMIC"], "STATIC");
    }
}
