use core::ops::Range;

use prjcombine_interconnect::{dir::DirH, grid::TileCoord};
use prjcombine_re_fpga_hammer::{FuzzerProp, OcdMode, xlat_bit, xlat_bitvec, xlat_enum};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_spartan6::{bels, chip::Gts};
use prjcombine_types::bsdata::{TileBit, TileItem};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
};

const GTP_INVPINS: &[&str] = &[
    "DCLK",
    "RXUSRCLK0",
    "RXUSRCLK1",
    "RXUSRCLK20",
    "RXUSRCLK21",
    "TXUSRCLK0",
    "TXUSRCLK1",
    "TXUSRCLK20",
    "TXUSRCLK21",
    "TSTCLK0",
    "TSTCLK1",
];

const GTP_BOOL_ATTRS: &[&str] = &[
    "AC_CAP_DIS",
    "CHAN_BOND_KEEP_ALIGN",
    "CHAN_BOND_SEQ_2_USE",
    "CLK_COR_INSERT_IDLE_FLAG",
    "CLK_COR_KEEP_IDLE",
    "CLK_COR_PRECEDENCE",
    "CLK_CORRECT_USE",
    "CLK_COR_SEQ_2_USE",
    "CLKINDC_B",
    "CLKRCV_TRST",
    "DEC_MCOMMA_DETECT",
    "DEC_PCOMMA_DETECT",
    "DEC_VALID_COMMA_ONLY",
    "GTP_CFG_PWRUP",
    "LOOPBACK_DRP_EN",
    "MASTER_DRP_EN",
    "MCOMMA_DETECT",
    "PCI_EXPRESS_MODE",
    "PCOMMA_DETECT",
    "PDELIDLE_DRP_EN",
    "PHASEALIGN_DRP_EN",
    "PLL_DRP_EN",
    "PLL_SATA",
    "PLL_STARTUP_EN",
    "POLARITY_DRP_EN",
    "PRBS_DRP_EN",
    "RCV_TERM_GND",
    "RCV_TERM_VTTRX",
    "RESET_DRP_EN",
    "RX_BUFFER_USE",
    "RX_CDR_FORCE_ROTATE",
    "RX_DECODE_SEQ_MATCH",
    "RX_EN_IDLE_HOLD_CDR",
    "RX_EN_IDLE_RESET_BUF",
    "RX_EN_IDLE_RESET_FR",
    "RX_EN_IDLE_RESET_PH",
    "RX_EN_MODE_RESET_BUF",
    "RXEQ_DRP_EN",
    "RX_LOSS_OF_SYNC_FSM",
    "TERMINATION_OVRD",
    "TX_BUFFER_USE",
    "TXDRIVE_DRP_EN",
];

const GTP_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("ALIGN_COMMA_WORD", &["1", "2"]),
    ("CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4"]),
    ("CLK_COR_ADJ_LEN", &["1", "2", "3", "4"]),
    ("CLK_COR_DET_LEN", &["1", "2", "3", "4"]),
    ("CLK25_DIVIDER", &["1", "2", "3", "4", "5", "6", "10", "12"]),
    (
        "OOB_CLK_DIVIDER",
        &["1", "2", "4", "6", "8", "10", "12", "14"],
    ),
    ("PLL_DIVSEL_FB", &["1", "2", "3", "4", "5", "8", "10"]),
    (
        "PLL_DIVSEL_REF",
        &["1", "2", "3", "4", "5", "6", "8", "10", "12", "16", "20"],
    ),
    ("PLL_RXDIVSEL_OUT", &["1", "2", "4"]),
    ("PLL_TXDIVSEL_OUT", &["1", "2", "4"]),
    ("PLL_SOURCE", &["PLL0", "PLL1"]),
    (
        "RX_LOS_INVALID_INCR",
        &["1", "2", "4", "8", "16", "32", "64", "128"],
    ),
    (
        "RX_LOS_THRESHOLD",
        &["4", "8", "16", "32", "64", "128", "256", "512"],
    ),
    ("RX_SLIDE_MODE", &["PCS", "PMA"]),
    ("RX_STATUS_FMT", &["PCIE", "SATA"]),
    ("RX_XCLK_SEL", &["RXREC", "RXUSR"]),
    ("TX_XCLK_SEL", &["TXUSR", "TXOUT"]),
];

const GTP_ENUM_INT_ATTRS: &[(&str, Range<u32>)] = &[
    ("CHAN_BOND_1_MAX_SKEW", 1..15),
    ("CHAN_BOND_2_MAX_SKEW", 1..15),
    ("CLK_COR_MAX_LAT", 3..49),
    ("CLK_COR_MIN_LAT", 3..49),
    ("SATA_MAX_BURST", 1..62),
    ("SATA_MAX_INIT", 1..62),
    ("SATA_MAX_WAKE", 1..62),
    ("SATA_MIN_BURST", 1..62),
    ("SATA_MIN_INIT", 1..62),
    ("SATA_MIN_WAKE", 1..62),
];

const GTP_DEC_ATTRS: &[(&str, usize)] = &[("CB2_INH_CC_PERIOD", 4), ("CLK_COR_REPEAT_WAIT", 5)];

const GTP_BIN_ATTRS: &[(&str, usize)] = &[
    ("A_GTPRESET", 1),
    ("A_LOOPBACK", 3),
    ("A_PLLLKDETEN", 1),
    ("A_PLLPOWERDOWN", 1),
    ("A_PRBSCNTRESET", 1),
    ("A_RXBUFRESET", 1),
    ("A_RXCDRFREQRESET", 1),
    ("A_RXCDRHOLD", 1),
    ("A_RXCDRPHASERESET", 1),
    ("A_RXCDRRESET", 1),
    ("A_RXENPMAPHASEALIGN", 1),
    ("A_RXENPRBSTST", 3),
    ("A_RXEQMIX", 2),
    ("A_RXPMASETPHASE", 1),
    ("A_RXPOLARITY", 1),
    ("A_RXPOWERDOWN", 2),
    ("A_RXRESET", 1),
    ("A_TXBUFDIFFCTRL", 3),
    ("A_TXDIFFCTRL", 4),
    ("A_TXELECIDLE", 1),
    ("A_TXENPMAPHASEALIGN", 1),
    ("A_TXENPRBSTST", 3),
    ("A_TXPMASETPHASE", 1),
    ("A_TXPOLARITY", 1),
    ("A_TXPOWERDOWN", 2),
    ("A_TXPRBSFORCEERR", 1),
    ("A_TXPREEMPHASIS", 3),
    ("A_TXRESET", 1),
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
    ("MCOMMA_10B_VALUE", 10),
    ("OOBDETECT_THRESHOLD", 3),
    ("PCOMMA_10B_VALUE", 10),
    ("PLLLKDET_CFG", 3),
    ("RXEQ_CFG", 8),
    ("RXPRBSERR_LOOPBACK", 1),
    ("RX_IDLE_HI_CNT", 4),
    ("RX_IDLE_LO_CNT", 4),
    ("SATA_BURST_VAL", 3),
    ("SATA_IDLE_VAL", 3),
    ("TERMINATION_CTRL", 5),
    ("TEST_CLK_OUT_GTP", 2),
    ("TXRX_INVERT", 3),
    ("TX_IDLE_DELAY", 3),
    ("TX_TDCC_CFG", 2),
    ("USR_CODE_ERR_CLR", 1),
];

const GTP_HEX_ATTRS: &[(&str, usize)] = &[
    ("PLL_COM_CFG", 24),
    ("PLL_CP_CFG", 8),
    ("PMA_CDR_SCAN", 27),
    ("PMA_RXSYNC_CFG", 7),
    ("PMA_RX_CFG", 25),
    ("PMA_TX_CFG", 20),
    ("TRANS_TIME_FROM_P2", 12),
    ("TRANS_TIME_NON_P2", 8),
    ("TRANS_TIME_TO_P2", 10),
    ("TST_ATTR", 32),
    ("TX_DETECT_RX_CFG", 14),
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
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, "GTP") else {
        return;
    };
    let mut bctx = ctx.bel(bels::GTP);
    let mode = "GTPA1_DUAL";

    bctx.build()
        .global("GLUTMASK", "NO")
        .test_manual("PRESENT", "1")
        .mode(mode)
        .commit();

    for &pin in GTP_INVPINS {
        bctx.mode(mode).test_inv(pin);
    }
    for &attr in GTP_BOOL_ATTRS {
        bctx.mode(mode)
            .test_enum(format!("{attr}_0"), &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .test_enum(format!("{attr}_1"), &["FALSE", "TRUE"]);
    }
    for &(attr, vals) in GTP_ENUM_ATTRS {
        bctx.mode(mode).test_enum(format!("{attr}_0"), vals);
        bctx.mode(mode).test_enum(format!("{attr}_1"), vals);
    }
    for &(attr, ref vals) in GTP_ENUM_INT_ATTRS {
        let vals = Vec::from_iter(vals.clone().map(|i| i.to_string()));
        bctx.mode(mode).test_enum(format!("{attr}_0"), &vals);
        bctx.mode(mode).test_enum(format!("{attr}_1"), &vals);
    }
    for &(attr, width) in GTP_DEC_ATTRS {
        bctx.mode(mode)
            .test_multi_attr_dec(format!("{attr}_0"), width);
        bctx.mode(mode)
            .test_multi_attr_dec(format!("{attr}_1"), width);
    }
    for &(attr, width) in GTP_BIN_ATTRS {
        bctx.mode(mode)
            .test_multi_attr_bin(format!("{attr}_0"), width);
        bctx.mode(mode)
            .test_multi_attr_bin(format!("{attr}_1"), width);
    }
    for &(attr, width) in GTP_HEX_ATTRS {
        bctx.mode(mode)
            .test_multi_attr_hex(format!("{attr}_0"), width);
        bctx.mode(mode)
            .test_multi_attr_hex(format!("{attr}_1"), width);
    }

    bctx.mode(mode)
        .test_enum("CLK_OUT_GTP_SEL_0", &["TXOUTCLK0", "REFCLKPLL0"]);
    bctx.mode(mode)
        .test_enum("CLK_OUT_GTP_SEL_1", &["TXOUTCLK1", "REFCLKPLL1"]);

    bctx.mode(mode).test_multi_attr_hex("PMA_COM_CFG_EAST", 36);
    bctx.mode(mode).test_multi_attr_hex("PMA_COM_CFG_WEST", 36);

    for i in 0..2 {
        for pin in ["PLLCLK0", "PLLCLK1", "CLKINEAST", "CLKINWEST"] {
            bctx.build()
                .mutex(format!("REFSELPLL{i}"), pin)
                .test_manual(format!("REFSELPLL{i}"), pin)
                .pip(format!("{pin}{i}"), pin)
                .commit();
        }
        for (pin, obel) in [("CLK0", bels::BUFDS0), ("CLK1", bels::BUFDS1)] {
            bctx.build()
                .mutex(format!("REFSELPLL{i}"), pin)
                .test_manual(format!("REFSELPLL{i}"), pin)
                .pip(format!("{pin}{i}"), (obel, "O"))
                .commit();
        }
        for pin in ["GCLK0", "GCLK1"] {
            bctx.build()
                .mutex(format!("REFSELPLL{i}"), pin)
                .test_manual(format!("REFSELPLL{i}"), pin)
                .pin_pips(format!("{pin}{i}"))
                .commit();
        }
    }

    for pin in ["REFCLKPLL0", "REFCLKPLL1"] {
        bctx.build()
            .mutex("MUX.CLKOUT_EW", pin)
            .prop(DeviceSide(DirH::W))
            .test_manual("MUX.CLKOUT_EAST", pin)
            .pip("CLKOUT_EW", pin)
            .commit();
        if matches!(edev.chip.gts, Gts::Double(..) | Gts::Quad(..)) {
            bctx.build()
                .mutex("MUX.CLKOUT_EW", pin)
                .prop(DeviceSide(DirH::E))
                .test_manual("MUX.CLKOUT_WEST", pin)
                .pip("CLKOUT_EW", pin)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Spartan6(edev) = ctx.edev else {
        unreachable!()
    };

    let tile = "GTP";
    let bel = "GTP";

    if !ctx.has_tile(tile) {
        return;
    }

    fn drp_bit(idx: usize, bit: usize) -> TileBit {
        let tile = 8 + ((idx >> 2) & 7);
        let bit = bit + 16 * (idx & 3);
        let frame = 25 - ((idx >> 5) & 3);
        TileBit::new(tile, frame, bit)
    }

    for i in 0..0x80 {
        ctx.tiledb.insert(
            tile,
            bel,
            format!("DRP{i:02X}"),
            TileItem::from_bitvec((0..16).map(|j| drp_bit(i, j)).collect(), false),
        );
    }

    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    for &pin in GTP_INVPINS {
        ctx.collect_inv(tile, bel, pin);
    }
    for &attr in GTP_BOOL_ATTRS {
        ctx.collect_enum_bool(tile, bel, &format!("{attr}_0"), "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, &format!("{attr}_1"), "FALSE", "TRUE");
    }
    for &(attr, vals) in GTP_ENUM_ATTRS {
        ctx.collect_enum_ocd(tile, bel, &format!("{attr}_0"), vals, OcdMode::BitOrder);
        ctx.collect_enum_ocd(tile, bel, &format!("{attr}_1"), vals, OcdMode::BitOrder);
    }
    for &(attr, ref vals) in GTP_ENUM_INT_ATTRS {
        ctx.collect_enum_int(tile, bel, &format!("{attr}_0"), vals.clone(), 0);
        ctx.collect_enum_int(tile, bel, &format!("{attr}_1"), vals.clone(), 0);
    }
    ctx.collect_enum(tile, bel, "CLK_OUT_GTP_SEL_0", &["TXOUTCLK0", "REFCLKPLL0"]);
    ctx.collect_enum(tile, bel, "CLK_OUT_GTP_SEL_1", &["TXOUTCLK1", "REFCLKPLL1"]);
    for &(attr, _) in GTP_DEC_ATTRS {
        ctx.collect_bitvec(tile, bel, &format!("{attr}_0"), "");
        ctx.collect_bitvec(tile, bel, &format!("{attr}_1"), "");
    }
    for &(attr, _) in GTP_BIN_ATTRS {
        if attr == "RXPRBSERR_LOOPBACK" || attr == "COMMA_10B_ENABLE" {
            continue;
        }
        ctx.collect_bitvec(tile, bel, &format!("{attr}_0"), "");
        ctx.collect_bitvec(tile, bel, &format!("{attr}_1"), "");
    }
    // sigh. bugs.
    ctx.collect_bitvec(tile, bel, "COMMA_10B_ENABLE_0", "");
    let mut diffs = ctx.state.get_diffs(tile, bel, "COMMA_10B_ENABLE_1", "");
    diffs[3].bits.insert(TileBit::new(11, 23, 3), true);
    assert_eq!(diffs[4].bits.remove(&TileBit::new(11, 23, 3)), Some(true));
    ctx.tiledb
        .insert(tile, bel, "COMMA_10B_ENABLE_1", xlat_bitvec(diffs));
    ctx.collect_bitvec(tile, bel, "RXPRBSERR_LOOPBACK_0", "");
    ctx.state
        .get_diff(tile, bel, "RXPRBSERR_LOOPBACK_1", "")
        .assert_empty();
    ctx.tiledb.insert(
        tile,
        bel,
        "RXPRBSERR_LOOPBACK_1",
        TileItem::from_bit(TileBit::new(8, 22, 48), false),
    );
    for &(attr, _) in GTP_HEX_ATTRS {
        ctx.collect_bitvec(tile, bel, &format!("{attr}_0"), "");
        ctx.collect_bitvec(tile, bel, &format!("{attr}_1"), "");
    }
    ctx.collect_bitvec(tile, bel, "PMA_COM_CFG_EAST", "");
    ctx.collect_bitvec(tile, bel, "PMA_COM_CFG_WEST", "");

    ctx.collect_enum(tile, bel, "MUX.CLKOUT_EAST", &["REFCLKPLL0", "REFCLKPLL1"]);
    if matches!(edev.chip.gts, Gts::Double(..) | Gts::Quad(..)) {
        ctx.collect_enum(tile, bel, "MUX.CLKOUT_WEST", &["REFCLKPLL0", "REFCLKPLL1"]);
    }

    for i in 0..2 {
        let refselpll_static = ctx
            .state
            .peek_diff(tile, bel, format!("REFSELPLL{i}"), "CLK0")
            .clone();
        let mut diffs = vec![];
        for val in [
            "CLK0",
            "GCLK0",
            "PLLCLK0",
            "CLKINEAST",
            "CLK1",
            "GCLK1",
            "PLLCLK1",
            "CLKINWEST",
        ] {
            let mut diff = ctx.state.get_diff(tile, bel, format!("REFSELPLL{i}"), val);
            diff = diff.combine(&!&refselpll_static);
            diffs.push((val, diff));
        }
        ctx.tiledb
            .insert(tile, bel, format!("REFSELPLL{i}_STATIC"), xlat_enum(diffs));
        ctx.tiledb.insert(
            tile,
            bel,
            format!("REFSELPLL{i}_STATIC_ENABLE"),
            xlat_bit(refselpll_static),
        );
    }
}
