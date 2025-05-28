use prjcombine_interconnect::grid::{DieId, NodeLoc};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_bitvec, xlat_enum,
    xlat_enum_default, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_virtex4::bels;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            DynProp,
            relation::{Delta, NodeRelation},
        },
    },
};

const GT11_INVPINS: &[&str] = &[
    "DCLK",
    "DEN",
    "DWE",
    "RXCRCCLK",
    "RXCRCDATAVALID",
    "RXCRCINTCLK",
    "RXCRCRESET",
    "RXPMARESET",
    "RXRESET",
    "RXUSRCLK2",
    "RXUSRCLK",
    "SCANEN0",
    "SCANEN1",
    "SCANEN2",
    "SCANMODE0",
    "SCANMODE1",
    "SCANMODE2",
    "TXCRCCLK",
    "TXCRCDATAVALID",
    "TXCRCINTCLK",
    "TXCRCRESET",
    "TXPMARESET",
    "TXRESET",
    "TXUSRCLK2",
    "TXUSRCLK",
];

const GT11_BOOL_ATTRS: &[&str] = &[
    "AUTO_CAL",
    "BYPASS_CAL",
    "BYPASS_FDET",
    "CCCB_ARBITRATOR_DISABLE",
    "CHAN_BOND_ONE_SHOT",
    "CHAN_BOND_SEQ_2_USE",
    "CLK_COR_8B10B_DE",
    "CLK_CORRECT_USE",
    "CLK_COR_SEQ_2_USE",
    "CLK_COR_SEQ_DROP",
    "COMMA32",
    "DEC_MCOMMA_DETECT",
    "DEC_PCOMMA_DETECT",
    "DEC_VALID_COMMA_ONLY",
    "DIGRX_SYNC_MODE",
    "ENABLE_DCDR",
    "MCOMMA_DETECT",
    "OPPOSITE_SELECT",
    "PCOMMA_DETECT",
    "PCS_BIT_SLIP",
    "PMA_BIT_SLIP",
    "PMACLKENABLE",
    "POWER_ENABLE",
    "REPEATER",
    "RESERVED_CB1",
    "RESERVED_CCA",
    "RESERVED_CCB",
    "RESERVED_M2",
    "RXACTST",
    "RXADCADJPD",
    "RXAFEPD",
    "RXAFETST",
    "RXAPD",
    "RXAPTST",
    "RXAUTO_CAL",
    "RXBIASPD",
    "RX_BUFFER_USE",
    "RXBY_32",
    "RXBYPASS_CAL",
    "RXBYPASS_FDET",
    "RXCLK0_FORCE_PMACLK",
    "RXCLK0_INVERT_PMALEAF",
    "RXCMFPD",
    "RXCMFTST",
    "RXCPSEL",
    "RXCPTST",
    "RXCRCCLOCKDOUBLE",
    "RXCRCENABLE",
    "RXCRCINVERTGEN",
    "RXCRCSAMECLOCK",
    "RXDACSEL",
    "RXDACTST",
    "RXDCCOUPLE",
    "RXDIGRESET",
    "RXDIGRX",
    "RXDIVBUFPD",
    "RXDIVBUFTST",
    "RXDIVPD",
    "RXDIVTST",
    "RXFILTTST",
    "RXLB",
    "RXLKAPD",
    "RXPDDTST",
    "RXPD",
    "RXPFDTST",
    "RXPFDTX",
    "RXQPPD",
    "RXRCPPD",
    "RXRECCLK1_USE_SYNC",
    "RXRPDPD",
    "RXRSDPD",
    "RXSLOSEL",
    "RXTADJ",
    "RXVCOBUFPD",
    "RXVCOBUFTST",
    "RXVCO_CTRL_ENABLE",
    "RXVCOPD",
    "RXVCOTST",
    "SAMPLE_8X",
    "TEST_MODE_1",
    "TEST_MODE_2",
    "TEST_MODE_3",
    "TXAREFBIASSEL",
    "TX_BUFFER_USE",
    "TXCFGENABLE",
    "TXCLK0_FORCE_PMACLK",
    "TXCLK0_INVERT_PMALEAF",
    "TXCRCCLOCKDOUBLE",
    "TXCRCENABLE",
    "TXCRCINVERTGEN",
    "TXCRCSAMECLOCK",
    "TXDIGPD",
    "TXHIGHSIGNALEN",
    "TXLVLSHFTPD",
    "TXOUTCLK1_USE_SYNC",
    "TXPD",
    "TXPHASESEL",
    "TXPOST_TAP_PD",
    "TXPRE_TAP_PD",
    "TXSLEWRATE",
    "VCO_CTRL_ENABLE",
];

const GT11_ENUM_ATTRS: &[(&str, &[&str])] = &[
    ("ALIGN_COMMA_WORD", &["1", "2", "4"]),
    (
        "CHAN_BOND_MODE",
        &["NONE", "MASTER", "SLAVE_1_HOP", "SLAVE_2_HOPS"],
    ),
    ("CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4", "8"]),
    ("CLK_COR_SEQ_LEN", &["1", "2", "3", "4", "8"]),
    ("GT11_MODE", &["SINGLE", "DONT_CARE", "B", "A"]),
    ("RXFDCAL_CLOCK_DIVIDE", &["TWO", "NONE", "FOUR"]),
    (
        "RX_LOS_INVALID_INCR",
        &["1", "2", "4", "8", "16", "32", "64", "128"],
    ),
    (
        "RX_LOS_THRESHOLD",
        &["4", "8", "16", "32", "64", "128", "256", "512"],
    ),
    ("RXOUTDIV2SEL", &["1", "2", "4", "8", "16", "32"]),
    ("RXPLLNDIVSEL", &["8", "10", "16", "20", "32", "40"]),
    ("RXPMACLKSEL", &["REFCLK1", "REFCLK2", "GREFCLK"]),
    ("RXUSRDIVISOR", &["1", "2", "4", "8", "16"]),
    ("TXFDCAL_CLOCK_DIVIDE", &["TWO", "NONE", "FOUR"]),
    ("TXOUTDIV2SEL", &["1", "2", "4", "8", "16", "32"]),
];

const GT11_DEC_ATTRS: &[(&str, usize)] = &[
    ("CHAN_BOND_LIMIT", 5),
    ("CLK_COR_MIN_LAT", 6),
    ("CLK_COR_MAX_LAT", 6),
    ("SH_INVALID_CNT_MAX", 8),
    ("SH_CNT_MAX", 8),
];

const GT11_BIN_ATTRS: &[(&str, usize)] = &[
    ("CLK_COR_SEQ_1_1", 11),
    ("CLK_COR_SEQ_1_2", 11),
    ("CLK_COR_SEQ_1_3", 11),
    ("CLK_COR_SEQ_1_4", 11),
    ("CLK_COR_SEQ_2_1", 11),
    ("CLK_COR_SEQ_2_2", 11),
    ("CLK_COR_SEQ_2_3", 11),
    ("CLK_COR_SEQ_2_4", 11),
    ("CHAN_BOND_SEQ_1_1", 11),
    ("CHAN_BOND_SEQ_1_2", 11),
    ("CHAN_BOND_SEQ_1_3", 11),
    ("CHAN_BOND_SEQ_1_4", 11),
    ("CHAN_BOND_SEQ_2_1", 11),
    ("CHAN_BOND_SEQ_2_2", 11),
    ("CHAN_BOND_SEQ_2_3", 11),
    ("CHAN_BOND_SEQ_2_4", 11),
    ("CLK_COR_SEQ_1_MASK", 4),
    ("CLK_COR_SEQ_2_MASK", 4),
    ("CHAN_BOND_SEQ_1_MASK", 4),
    ("CHAN_BOND_SEQ_2_MASK", 4),
    ("CHAN_BOND_TUNE", 8),
    ("CYCLE_LIMIT_SEL", 2),
    ("RXCYCLE_LIMIT_SEL", 2),
    ("DCDR_FILTER", 3),
    ("DIGRX_FWDCLK", 2),
    ("FDET_HYS_CAL", 3),
    ("FDET_HYS_SEL", 3),
    ("FDET_LCK_CAL", 3),
    ("FDET_LCK_SEL", 3),
    ("LOOPCAL_WAIT", 2),
    ("RXAFEEQ", 9),
    ("RXASYNCDIVIDE", 2),
    ("RXCDRLOS", 6),
    ("RXCLKMODE", 6),
    ("RXCLMODE", 2),
    ("RXCMADJ", 2),
    ("RXDATA_SEL", 2),
    ("RXFDET_HYS_CAL", 3),
    ("RXFDET_HYS_SEL", 3),
    ("RXFDET_LCK_CAL", 3),
    ("RXFDET_LCK_SEL", 3),
    ("RXFECONTROL1", 2),
    ("RXFECONTROL2", 3),
    ("RXFETUNE", 2),
    ("RXLKADJ", 5),
    ("RXLOOPCAL_WAIT", 2),
    ("RXLOOPFILT", 4),
    ("RXMODE", 6),
    ("RXRCPADJ", 3),
    ("RXRIBADJ", 2),
    ("RXSLOWDOWN_CAL", 2),
    ("RXVCODAC_INIT", 10),
    ("RX_CLOCK_DIVIDER", 2),
    ("SLOWDOWN_CAL", 2),
    ("TXASYNCDIVIDE", 2),
    ("TXCLKMODE", 4),
    ("TXDATA_SEL", 2),
    ("TXDAT_PRDRV_DAC", 3),
    ("TXDAT_TAP_DAC", 5),
    ("TXLNDR_TST1", 4),
    ("TXLNDR_TST2", 2),
    ("TXPOST_PRDRV_DAC", 3),
    ("TXPOST_TAP_DAC", 5),
    ("TXPRE_PRDRV_DAC", 3),
    ("TXPRE_TAP_DAC", 5),
    ("TXTERMTRIM", 4),
    ("TX_CLOCK_DIVIDER", 2),
    ("VCODAC_INIT", 10),
];

const GT11_HEX_ATTRS: &[(&str, usize)] = &[
    ("COMMA_10B_MASK", 10),
    ("RESERVED_CM", 24),
    ("RESERVED_CM2", 22),
    ("RXCRCINITVAL", 32),
    ("RXCTRL1", 10),
    ("RXEQ", 64),
    ("RXTUNE", 13),
    ("TXCRCINITVAL", 32),
    ("TXLNDR_TST3", 15),
];

const GT11_SHARED_BOOL_ATTRS: &[&str] = &[
    "TXADCADJPD",
    "TXAPTST",
    "TXAPD",
    "TXBIASPD",
    "TXCMFPD",
    "TXCMFTST",
    "TXCPSEL",
    "TXDIVPD",
    "TXDIVTST",
    "TXDIVBUFPD",
    "TXDIVBUFTST",
    "TXDIGRX",
    "TXDACTST",
    "TXDACSEL",
    "TXFILTTST",
    "TXPFDTST",
    "TXPFDTX",
    "TXQPPD",
    "TXSLOSEL",
    "TXVCOBUFPD",
    "TXVCOBUFTST",
    "TXVCOPD",
    "TXVCOTST",
    "NATBENABLE",
    "ATBENABLE",
    "ATBBUMPEN",
    "BIASRESSEL",
    "PMATUNE",
    "PMABIASPD",
    "PMACOREPWRENABLE",
    "PMACTRL",
    "VREFSELECT",
    "BANDGAPSEL",
];

const GT11_SHARED_BIN_ATTRS: &[(&str, usize)] = &[
    ("IREFBIASMODE", 2),
    ("PMAIREFTRIM", 4),
    ("PMAVBGCTRL", 5),
    ("PMAVREFTRIM", 4),
    ("RXAREGCTRL", 5),
    ("TXCLMODE", 2),
    ("TXLOOPFILT", 4),
    ("TXREGCTRL", 5),
    ("VREFBIASMODE", 2),
];

const GT11_SHARED_HEX_ATTRS: &[(&str, usize)] = &[
    ("ATBSEL", 18),
    ("PMACFG2SPARE", 46),
    ("TXCTRL1", 10),
    ("TXTUNE", 13),
];

#[derive(Clone, Debug)]
struct MgtRepeaterMgt(i32, String, String);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for MgtRepeaterMgt {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        nloc: NodeLoc,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let row = nloc.2 + self.0;
        let is_w = nloc.1 < edev.col_cfg;
        for &col in &edev.chips[nloc.0].cols_vbrk {
            if (col < edev.col_cfg) == is_w {
                let rcol = if is_w { col } else { col - 1 };
                let nnloc = edev
                    .egrid
                    .get_tile_by_class(nloc.0, (rcol, row), |kind| kind == "HCLK_MGT_REPEATER");
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: "HCLK_MGT_REPEATER".into(),
                        bel: "HCLK_MGT_REPEATER".into(),
                        attr: self.1.clone(),
                        val: self.2.clone(),
                    },
                    tiles: edev.node_bits(nnloc),
                });
            }
        }

        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct ClkHrow(i32);

impl NodeRelation for ClkHrow {
    fn resolve(&self, backend: &IseBackend, nloc: NodeLoc) -> Option<NodeLoc> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(
            backend
                .egrid
                .get_tile_by_class(nloc.0, (edev.col_clk, nloc.2 + self.0), |kind| {
                    kind == "CLK_HROW"
                }),
        )
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, "MGT") else {
        return;
    };
    for i in 0..2 {
        let bel = format!("GT11_{i}");
        let mut bctx = ctx.bel(bels::GT11[i]);
        let mode = "GT11";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for &pin in GT11_INVPINS {
            bctx.mode(mode).test_inv(pin);
        }
        for &attr in GT11_BOOL_ATTRS {
            bctx.mode(mode).test_enum(attr, &["FALSE", "TRUE"]);
        }
        for &(attr, vals) in GT11_ENUM_ATTRS {
            bctx.mode(mode).test_enum(attr, vals);
        }
        for &(attr, width) in GT11_DEC_ATTRS {
            bctx.mode(mode).test_multi_attr_dec(attr, width);
        }
        for &(attr, width) in GT11_BIN_ATTRS {
            bctx.mode(mode).test_multi_attr_bin(attr, width);
        }
        for &(attr, width) in GT11_HEX_ATTRS {
            bctx.mode(mode).test_multi_attr_hex(attr, width);
        }
        for &attr in GT11_SHARED_BOOL_ATTRS {
            bctx.mode(mode)
                .tile_mutex(attr, &bel)
                .test_enum(attr, &["FALSE", "TRUE"]);
        }
        bctx.mode(mode)
            .tile_mutex("TXABPMACLKSEL", &bel)
            .test_enum("TXABPMACLKSEL", &["REFCLK1", "REFCLK2", "GREFCLK"]);
        bctx.mode(mode)
            .tile_mutex("TXPLLNDIVSEL", &bel)
            .test_enum("TXPLLNDIVSEL", &["8", "10", "16", "20", "32", "40"]);
        for &(attr, width) in GT11_SHARED_BIN_ATTRS {
            bctx.mode(mode)
                .tile_mutex(attr, &bel)
                .test_multi_attr_bin(attr, width);
        }
        for &(attr, width) in GT11_SHARED_HEX_ATTRS {
            bctx.mode(mode)
                .tile_mutex(attr, &bel)
                .test_multi_attr_hex(attr, width);
        }

        bctx.mode(mode)
            .attr("MCOMMA_32B_VALUE", "")
            .test_multi_attr_hex("MCOMMA_10B_VALUE", 10);
        bctx.mode(mode)
            .attr("MCOMMA_10B_VALUE", "")
            .test_multi_attr_hex("MCOMMA_32B_VALUE", 32);
        bctx.mode(mode)
            .attr("PCOMMA_32B_VALUE", "")
            .test_multi_attr_hex("PCOMMA_10B_VALUE", 10);
        bctx.mode(mode)
            .attr("PCOMMA_10B_VALUE", "")
            .test_multi_attr_hex("PCOMMA_32B_VALUE", 32);

        let hclk_delta = match i {
            0 => 8,
            1 => 24,
            _ => unreachable!(),
        };
        for pin in ["REFCLK", "PMACLK"] {
            for i in 0..8 {
                let obel = bels::CLK_HROW;
                bctx.build()
                    .mutex("HCLK_IN", format!("HCLK{i}"))
                    .mutex("HCLK_OUT", pin)
                    .global_mutex("BUFGCTRL_OUT", "USE")
                    .related_tile_mutex(ClkHrow(hclk_delta), "MODE", "USE")
                    .related_pip(
                        ClkHrow(hclk_delta),
                        (obel, format!("HCLK_L{i}")),
                        (obel, "GCLK0"),
                    )
                    .related_pip(
                        ClkHrow(hclk_delta),
                        (obel, format!("HCLK_R{i}")),
                        (obel, "GCLK0"),
                    )
                    .extra_tile_attr(
                        Delta::new(0, hclk_delta, "HCLK_MGT"),
                        "HCLK_MGT",
                        format!("BUF.HCLK{i}"),
                        "1",
                    )
                    .test_manual(format!("MUX.{pin}"), format!("HCLK{i}"))
                    .pip(pin, format!("HCLK{i}"))
                    .commit();
            }
        }
        for i in 0..2 {
            for inp in ["SYNCLK_OUT", "FWDCLK0_OUT", "FWDCLK1_OUT"] {
                bctx.build()
                    .global_mutex("MGT_OUT", "TEST")
                    .mutex(format!("MUX.MGT{i}"), inp)
                    .tile_mutex("SYNCLK", "USE")
                    .mutex("SYNCLK_OUT", "USE")
                    .pip("SYNCLK_OUT", "SYNCLK1_OUT")
                    .extra_tile_attr(
                        Delta::new(0, hclk_delta, "HCLK_MGT"),
                        "HCLK_MGT",
                        format!("BUF.MGT{i}"),
                        "1",
                    )
                    .prop(MgtRepeaterMgt(
                        hclk_delta,
                        format!("BUF.MGT{i}.MGT"),
                        "1".into(),
                    ))
                    .test_manual(format!("MUX.MGT{i}"), inp)
                    .pip(format!("MGT{i}"), inp)
                    .commit();
            }
        }
        for i in [1, 2] {
            bctx.build()
                .tile_mutex("SYNCLK", &bel)
                .mutex("SYNCLK_OUT", format!("SYNCLK{i}"))
                .test_manual("MUX.SYNCLK_OUT", format!("SYNCLK{i}"))
                .pip("SYNCLK_OUT", format!("SYNCLK{i}_OUT"))
                .commit();
        }
        let obel_clk = bels::GT11CLK;
        let (ab, ba, ns, sn) = match i {
            0 => ('B', 'A', 'S', 'N'),
            1 => ('A', 'B', 'N', 'S'),
            _ => unreachable!(),
        };
        for i in 0..2 {
            for j in 1..=4 {
                bctx.build()
                    .tile_mutex("FWDCLK_MUX_BEL", &bel)
                    .tile_mutex("FWDCLK_MUX", format!("MUX.FWDCLK{i}_OUT"))
                    .mutex(format!("MUX.FWDCLK{i}_OUT"), format!("FWDCLK{j}"))
                    .test_manual(format!("MUX.FWDCLK{i}_OUT"), format!("FWDCLK{j}"))
                    .pip(
                        (obel_clk, format!("FWDCLK{i}{ab}_OUT")),
                        (obel_clk, format!("{ns}FWDCLK{j}")),
                    )
                    .commit();
            }
        }
        for i in 1..=4 {
            for pin in [
                "RXPCSHCLKOUTA",
                "RXPCSHCLKOUTB",
                "TXPCSHCLKOUTA",
                "TXPCSHCLKOUTB",
            ] {
                bctx.build()
                    .global_mutex("MGT_FWDCLK_BUF", "DRIVE")
                    .tile_mutex(format!("MUX.{ab}.FWDCLK{i}"), pin)
                    .test_manual(format!("MUX.FWDCLK{i}"), pin)
                    .pip((obel_clk, format!("{ns}FWDCLK{i}")), (obel_clk, pin))
                    .commit();
            }
            bctx.build()
                .global_mutex("MGT_FWDCLK_BUF", "DRIVE")
                .tile_mutex(format!("MUX.{ab}.FWDCLK{i}"), "FWDCLK")
                .tile_mutex(format!("MUX.{ba}.FWDCLK{i}"), "FORCE")
                .pip(
                    (obel_clk, format!("{sn}FWDCLK{i}")),
                    (obel_clk, "RXPCSHCLKOUTA"),
                )
                .test_manual(format!("MUX.FWDCLK{i}"), format!("{ba}_FWDCLK{i}"))
                .pip(
                    (obel_clk, format!("{ns}FWDCLK{i}")),
                    (obel_clk, format!("{sn}FWDCLK{i}")),
                )
                .commit();
        }
    }
    let mut bctx = ctx.bel(bels::GT11CLK);
    let mode = "GT11CLK";
    bctx.test_manual("PRESENT", "1").mode(mode).commit();
    bctx.mode(mode).test_enum(
        "REFCLKSEL",
        &["REFCLK", "RXBCLK", "MGTCLK", "SYNCLK1IN", "SYNCLK2IN"],
    );
    for inp in ["REFCLKA", "REFCLKB"] {
        bctx.build()
            .mutex("REFCLK", inp)
            .test_manual("MUX.REFCLK", inp)
            .pip("REFCLK", inp)
            .commit();
    }
    for inp in ["PMACLKA", "PMACLKB"] {
        bctx.build()
            .mutex("PMACLK", inp)
            .test_manual("MUX.PMACLK", inp)
            .pip("PMACLK", inp)
            .commit();
    }

    for i in [1, 2] {
        bctx.build()
            .global_mutex("SYNCLK_BUF_DIR", "UP")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_BUF_UP"))
            .related_pip(
                Delta::new(0, -32, "MGT"),
                format!("SYNCLK{i}_N"),
                format!("SYNCLK{i}OUT"),
            )
            .related_tile_mutex(Delta::new(0, -32, "MGT"), "SYNCLK", "USE")
            .test_manual(format!("SYNCLK{i}"), "BUF_UP")
            .pip(format!("SYNCLK{i}_N"), format!("SYNCLK{i}_S"))
            .commit();
        bctx.build()
            .global_mutex("SYNCLK_BUF_DIR", "DOWN")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_BUF_DOWN"))
            .related_pip(
                Delta::new(0, 32, "MGT"),
                format!("SYNCLK{i}_S"),
                format!("SYNCLK{i}OUT"),
            )
            .related_tile_mutex(Delta::new(0, 32, "MGT"), "SYNCLK", "USE")
            .test_manual(format!("SYNCLK{i}"), "BUF_DOWN")
            .pip(format!("SYNCLK{i}_S"), format!("SYNCLK{i}_N"))
            .commit();
        bctx.mode(mode)
            .global_mutex("SYNCLK_BUF_DIR", "UP")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_DRIVE_UP"))
            .test_manual(format!("SYNCLK{i}"), "DRIVE_UP")
            .attr(format!("SYNCLK{i}OUTEN"), "ENABLE")
            .pin(format!("SYNCLK{i}OUT"))
            .pip(format!("SYNCLK{i}_N"), format!("SYNCLK{i}OUT"))
            .commit();
        bctx.mode(mode)
            .global_mutex("SYNCLK_BUF_DIR", "DOWN")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_DRIVE_DOWN"))
            .test_manual(format!("SYNCLK{i}"), "DRIVE_DOWN")
            .attr(format!("SYNCLK{i}OUTEN"), "ENABLE")
            .pin(format!("SYNCLK{i}OUT"))
            .pip(format!("SYNCLK{i}_S"), format!("SYNCLK{i}OUT"))
            .commit();
        bctx.mode(mode)
            .global_mutex_here("SYNCLK_BUF_DIR")
            .tile_mutex("SYNCLK", format!("SYNCLK{i}_DRIVE_BOTH"))
            .test_manual(format!("SYNCLK{i}"), "DRIVE_BOTH")
            .attr(format!("SYNCLK{i}OUTEN"), "ENABLE")
            .pin(format!("SYNCLK{i}OUT"))
            .pip(format!("SYNCLK{i}_N"), format!("SYNCLK{i}OUT"))
            .pip(format!("SYNCLK{i}_S"), format!("SYNCLK{i}OUT"))
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    if !ctx.has_tile("MGT") {
        return;
    }
    let tile = "MGT";
    fn drp_bit(bel: usize, idx: usize, bit: usize) -> TileBit {
        let tile = bel << 4 | (idx & 7) << 1 | (idx & 0x20) >> 5;
        let bit = bit + 1 + 20 * (idx >> 3 & 3);
        TileBit::new(tile, 19, bit)
    }
    let (_, _, synclk_enable) = Diff::split(
        ctx.state
            .peek_diff(tile, "GT11_1", "MUX.SYNCLK_OUT", "SYNCLK1")
            .clone(),
        ctx.state
            .peek_diff(tile, "GT11_0", "MUX.SYNCLK_OUT", "SYNCLK1")
            .clone(),
    );
    for idx in 0..2 {
        let bel = &format!("GT11_{idx}");
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        for i in 0x40..0x80 {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("DRP{i:02X}"),
                TileItem::from_bitvec((0..16).map(|j| drp_bit(idx, i, j)).collect(), false),
            );
            let item = TileItem::from_bit(drp_bit(idx, i, 17), false);
            present.apply_bit_diff(&item, true, false);
            ctx.tiledb
                .insert(tile, bel, format!("DRP{i:02X}_MASK"), item);
        }
        for &pin in GT11_INVPINS {
            ctx.collect_int_inv(&["INT"; 32], tile, bel, pin, false);
        }
        for pin in [
            "RXRESET",
            "RXCRCRESET",
            "RXPMARESET",
            "TXRESET",
            "TXCRCRESET",
            "TXPMARESET",
            "RXCRCINTCLK",
            "TXCRCINTCLK",
            "RXCRCCLK",
            "TXCRCCLK",
            "RXCRCDATAVALID",
            "TXCRCDATAVALID",
            "DCLK",
            "DEN",
            "DWE",
        ] {
            present.apply_bit_diff(&ctx.item_int_inv(&["INT"; 32], tile, bel, pin), false, true);
        }
        present.assert_empty();
        for &attr in GT11_BOOL_ATTRS {
            if attr == "PMACLKENABLE" {
                ctx.state.get_diff(tile, bel, attr, "FALSE").assert_empty();
                ctx.state.get_diff(tile, bel, attr, "TRUE").assert_empty();
            } else {
                ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            }
        }
        for &(attr, vals) in GT11_ENUM_ATTRS {
            // TODO: RXOUTDIV2SEL split
            // TODO: intify RXUSRDIVISOR, RX_LOS_INVALID_INCR, RX_LOS_THRESHOLD (div4!)
            if attr == "GT11_MODE" {
                for &val in vals {
                    ctx.state.get_diff(tile, bel, attr, val).assert_empty();
                }
            } else {
                ctx.collect_enum(tile, bel, attr, vals);
            }
        }
        for &(attr, _) in GT11_DEC_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GT11_BIN_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in GT11_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }

        let diffs_10 = ctx.state.get_diffs(tile, bel, "MCOMMA_10B_VALUE", "");
        let diffs_32 = ctx.state.get_diffs(tile, bel, "MCOMMA_32B_VALUE", "");
        assert!(diffs_32.starts_with(&diffs_10));
        ctx.tiledb
            .insert(tile, bel, "MCOMMA_VALUE", xlat_bitvec(diffs_32));
        let diffs_10 = ctx.state.get_diffs(tile, bel, "PCOMMA_10B_VALUE", "");
        let diffs_32 = ctx.state.get_diffs(tile, bel, "PCOMMA_32B_VALUE", "");
        assert!(diffs_32.starts_with(&diffs_10));
        ctx.tiledb
            .insert(tile, bel, "PCOMMA_VALUE", xlat_bitvec(diffs_32));

        for &attr in GT11_SHARED_BOOL_ATTRS {
            let item = ctx.extract_enum_bool(tile, bel, attr, "FALSE", "TRUE");
            ctx.tiledb.insert(tile, "GT11_COMMON", attr, item);
        }
        let item = ctx.extract_enum(
            tile,
            bel,
            "TXABPMACLKSEL",
            &["REFCLK1", "REFCLK2", "GREFCLK"],
        );
        ctx.tiledb
            .insert(tile, "GT11_COMMON", "TXABPMACLKSEL", item);
        let item = ctx.extract_enum(
            tile,
            bel,
            "TXPLLNDIVSEL",
            &["8", "10", "16", "20", "32", "40"],
        );
        ctx.tiledb.insert(tile, "GT11_COMMON", "TXPLLNDIVSEL", item);
        for &(attr, _) in GT11_SHARED_BIN_ATTRS {
            let item = ctx.extract_bitvec(tile, bel, attr, "");
            ctx.tiledb.insert(tile, "GT11_COMMON", attr, item);
        }
        for &(attr, _) in GT11_SHARED_HEX_ATTRS {
            let item = ctx.extract_bitvec(tile, bel, attr, "");
            ctx.tiledb.insert(tile, "GT11_COMMON", attr, item);
        }

        for attr in ["MUX.PMACLK", "MUX.REFCLK"] {
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                attr,
                &[
                    "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7",
                ],
                "NONE",
                OcdMode::BitOrder,
            );
        }

        let ba = match idx {
            1 => 'B',
            0 => 'A',
            _ => unreachable!(),
        };
        for i in 1..=4 {
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.FWDCLK{i}"),
                &[
                    "RXPCSHCLKOUTA",
                    "RXPCSHCLKOUTB",
                    "TXPCSHCLKOUTA",
                    "TXPCSHCLKOUTB",
                    &format!("{ba}_FWDCLK{i}")[..],
                ],
                "NONE",
                OcdMode::BitOrder,
            );
        }

        let (_, _, fwdclk_out_enable) = Diff::split(
            ctx.state
                .peek_diff(tile, bel, "MUX.FWDCLK0_OUT", "FWDCLK1")
                .clone(),
            ctx.state
                .peek_diff(tile, bel, "MUX.FWDCLK1_OUT", "FWDCLK1")
                .clone(),
        );
        for i in 0..2 {
            let mut diffs = vec![];
            for j in 1..=4 {
                let mut diff = ctx.state.get_diff(
                    tile,
                    bel,
                    format!("MUX.FWDCLK{i}_OUT"),
                    format!("FWDCLK{j}"),
                );
                diff = diff.combine(&!&fwdclk_out_enable);
                diffs.push((format!("FWDCLK{j}"), diff));
            }
            ctx.tiledb.insert(
                tile,
                bel,
                format!("MUX.FWDCLK{i}_OUT"),
                xlat_enum_ocd(diffs, OcdMode::BitOrder),
            );
        }
        ctx.tiledb.insert(
            tile,
            "GT11_COMMON",
            "FWDCLK_OUT_ENABLE",
            xlat_bit(fwdclk_out_enable),
        );

        for i in 0..2 {
            ctx.collect_enum_default_ocd(
                tile,
                bel,
                &format!("MUX.MGT{i}"),
                &["SYNCLK_OUT", "FWDCLK0_OUT", "FWDCLK1_OUT"],
                "NONE",
                OcdMode::BitOrder,
            );
        }
        let mut diffs = vec![];
        for inp in ["SYNCLK1", "SYNCLK2"] {
            let mut diff = ctx.state.get_diff(tile, bel, "MUX.SYNCLK_OUT", inp);
            diff = diff.combine(&!&synclk_enable);
            diffs.push((inp.to_string(), diff));
        }
        ctx.tiledb.insert(
            tile,
            bel,
            "MUX.SYNCLK_OUT",
            xlat_enum_default(diffs, "NONE"),
        );
    }
    ctx.state
        .get_diff(tile, "GT11CLK", "PRESENT", "1")
        .assert_empty();

    let (_, _, mut synclk_drive_enable) = Diff::split(
        ctx.state
            .peek_diff(tile, "GT11CLK", "SYNCLK1", "BUF_DOWN")
            .clone(),
        ctx.state
            .peek_diff(tile, "GT11CLK", "SYNCLK2", "BUF_DOWN")
            .clone(),
    );
    for attr in ["SYNCLK1", "SYNCLK2"] {
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["BUF_UP", "BUF_DOWN", "DRIVE_UP", "DRIVE_DOWN", "DRIVE_BOTH"] {
            let mut diff = ctx.state.get_diff(tile, "GT11CLK", attr, val);
            diff = diff.combine(&!&synclk_drive_enable);
            diffs.push((val, diff));
        }
        ctx.tiledb
            .insert(tile, "GT11_COMMON", attr, xlat_enum(diffs));
    }
    synclk_drive_enable = synclk_drive_enable.combine(&!&synclk_enable);
    ctx.tiledb.insert(
        tile,
        "GT11_COMMON",
        "SYNCLK_DRIVE_ENABLE",
        xlat_bit(synclk_drive_enable),
    );
    ctx.tiledb.insert(
        tile,
        "GT11_COMMON",
        "SYNCLK_ENABLE",
        xlat_bit(synclk_enable),
    );

    let item = ctx.extract_enum(
        tile,
        "GT11CLK",
        "REFCLKSEL",
        &["SYNCLK1IN", "SYNCLK2IN", "RXBCLK", "REFCLK", "MGTCLK"],
    );
    ctx.tiledb.insert(tile, "GT11_COMMON", "REFCLKSEL", item);

    let item = ctx.extract_enum_default_ocd(
        tile,
        "GT11CLK",
        "MUX.REFCLK",
        &["REFCLKA", "REFCLKB"],
        "NONE",
        OcdMode::BitOrder,
    );
    ctx.tiledb.insert(tile, "GT11_COMMON", "MUX.REFCLK", item);
    let item = ctx.extract_enum_default_ocd(
        tile,
        "GT11CLK",
        "MUX.PMACLK",
        &["PMACLKA", "PMACLKB"],
        "NONE",
        OcdMode::BitOrder,
    );
    ctx.tiledb.insert(tile, "GT11_COMMON", "MUX.PMACLK", item);

    let tile = "HCLK_MGT";
    let bel = "HCLK_MGT";
    for i in 0..8 {
        ctx.collect_bit(tile, bel, &format!("BUF.HCLK{i}"), "1");
    }
    for i in 0..2 {
        ctx.collect_bit(tile, bel, &format!("BUF.MGT{i}"), "1");
    }

    if !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
        let tile = "HCLK_MGT_REPEATER";
        let bel = "HCLK_MGT_REPEATER";
        let item = ctx.extract_bit(tile, bel, "BUF.MGT0.MGT", "1");
        ctx.tiledb.insert(tile, bel, "BUF.MGT0", item);
        let item = ctx.extract_bit(tile, bel, "BUF.MGT1.MGT", "1");
        ctx.tiledb.insert(tile, bel, "BUF.MGT1", item);
    }
}
