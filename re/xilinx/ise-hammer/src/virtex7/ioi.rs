use prjcombine_entity::EntityId;
use prjcombine_interconnect::db::WireSlotIdExt;
use prjcombine_re_collector::{
    diff::{Diff, OcdMode, extract_common_diff, xlat_enum_raw},
    legacy::{
        xlat_bit_bi_legacy, xlat_bit_legacy, xlat_bitvec_legacy, xlat_enum_legacy,
        xlat_enum_legacy_ocd,
    },
};
use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs::{
    bslots,
    virtex7::{tcls, wires},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::{
            DynProp,
            bel::{BaseBelAttr, BaseBelMode},
            mutex::{TileMutex, WireMutexExclusive},
            relation::Related,
        },
    },
    virtex4::specials,
    virtex5::io::HclkIoi,
};

fn add_fuzzers_routing<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for (tcid, num_io) in [
        (tcls::IO_HR_PAIR, 2),
        (tcls::IO_HR_S, 1),
        (tcls::IO_HR_N, 1),
        (tcls::IO_HP_PAIR, 2),
        (tcls::IO_HP_S, 1),
        (tcls::IO_HP_N, 1),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for io in 0..num_io {
            for w in [wires::IMUX_IOI_ICLK, wires::IMUX_IOI_OCLK] {
                let dst0 = w[0].cell(io);
                let dst1 = w[1].cell(io);
                let mux = &backend.edev.db_index[tcid].muxes[&dst0];
                for &src in mux.src.keys() {
                    if src.wire == wires::PHASER_OCLK90 {
                        ctx.build()
                            .prop(WireMutexExclusive::new(dst0))
                            .prop(WireMutexExclusive::new(dst1))
                            .prop(BaseIntPip::new(dst1, wires::PHASER_OCLK.cell(io)))
                            .test_routing(dst0, src)
                            .prop(FuzzIntPip::new(dst0, src.tw))
                            .commit();

                        ctx.build()
                            .prop(WireMutexExclusive::new(dst0))
                            .prop(WireMutexExclusive::new(dst1))
                            .test_routing_pair_special(dst1, src, specials::IOI_OCLK90_BOTH)
                            .prop(FuzzIntPip::new(dst0, src.tw))
                            .commit();
                    } else {
                        ctx.build()
                            .prop(WireMutexExclusive::new(dst0))
                            .prop(WireMutexExclusive::new(dst1))
                            .prop(BaseIntPip::new(dst1, src.tw))
                            .test_routing(dst0, src)
                            .prop(FuzzIntPip::new(dst0, src.tw))
                            .commit();
                        ctx.build()
                            .prop(WireMutexExclusive::new(dst0))
                            .prop(WireMutexExclusive::new(dst1))
                            .test_routing(dst1, src)
                            .prop(FuzzIntPip::new(dst1, src.tw))
                            .commit();
                    }
                }
            }

            let dst0 = wires::IMUX_IOI_OCLKDIV[0].cell(io);
            let dst1 = wires::IMUX_IOI_OCLKDIV[1].cell(io);
            let dst0f = wires::IMUX_IOI_OCLKDIVF[0].cell(io);
            let dst1f = wires::IMUX_IOI_OCLKDIVF[1].cell(io);
            let src_p = wires::PHASER_OCLKDIV.cell(io);
            ctx.build()
                .prop(WireMutexExclusive::new(dst0))
                .prop(WireMutexExclusive::new(dst1))
                .prop(BaseIntPip::new(dst1, src_p))
                .test_routing(dst0, src_p.pos())
                .prop(FuzzIntPip::new(dst0, src_p))
                .commit();
            ctx.build()
                .prop(WireMutexExclusive::new(dst0))
                .prop(WireMutexExclusive::new(dst1))
                .test_routing(dst1, src_p.pos())
                .prop(FuzzIntPip::new(dst1, src_p))
                .commit();
            let mux = &backend.edev.db_index[tcid].muxes[&dst0f];
            for &src in mux.src.keys() {
                ctx.build()
                    .prop(WireMutexExclusive::new(dst0))
                    .prop(WireMutexExclusive::new(dst1))
                    .prop(WireMutexExclusive::new(dst0f))
                    .prop(WireMutexExclusive::new(dst1f))
                    .prop(BaseIntPip::new(dst1, src.tw))
                    .test_routing(dst0, src)
                    .prop(FuzzIntPip::new(dst0, src.tw))
                    .commit();
                ctx.build()
                    .prop(WireMutexExclusive::new(dst0))
                    .prop(WireMutexExclusive::new(dst1))
                    .prop(WireMutexExclusive::new(dst0f))
                    .prop(WireMutexExclusive::new(dst1f))
                    .test_routing(dst1, src)
                    .prop(FuzzIntPip::new(dst1, src.tw))
                    .commit();
            }
        }
    }
}

fn add_fuzzers_ilogic<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for (tcid, is_hp, num_io) in [
        (tcls::IO_HR_PAIR, false, 2),
        (tcls::IO_HR_S, false, 1),
        (tcls::IO_HR_N, false, 1),
        (tcls::IO_HP_PAIR, true, 2),
        (tcls::IO_HP_S, true, 1),
        (tcls::IO_HP_N, true, 1),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for i in 0..num_io {
            let mut bctx = ctx.bel(bslots::ILOGIC[i]);

            bctx.test_manual_legacy("PRESENT", "ILOGICE2")
                .mode("ILOGICE2")
                .commit();
            bctx.test_manual_legacy("PRESENT", "ISERDESE2")
                .mode("ISERDESE2")
                .commit();

            bctx.mode("ISERDESE2").test_inv_legacy("D");
            bctx.mode("ISERDESE2").test_inv_legacy("CLK");
            bctx.mode("ISERDESE2")
                .attr("DATA_RATE", "SDR")
                .test_inv_legacy("OCLK");
            bctx.mode("ISERDESE2")
                .attr("DYN_CLKDIV_INV_EN", "FALSE")
                .test_inv_legacy("CLKDIV");
            bctx.mode("ISERDESE2")
                .attr("DYN_CLKDIVP_INV_EN", "FALSE")
                .test_inv_legacy("CLKDIVP");
            bctx.mode("ISERDESE2")
                .test_enum_legacy("DYN_CLK_INV_EN", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum_legacy("DYN_CLKDIV_INV_EN", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum_legacy("DYN_CLKDIVP_INV_EN", &["FALSE", "TRUE"]);

            bctx.mode("ILOGICE2")
                .attr("IFFTYPE", "#FF")
                .pin("SR")
                .test_enum_legacy("SRUSED", &["0"]);
            bctx.mode("ISERDESE2")
                .attr("DATA_WIDTH", "2")
                .attr("DATA_RATE", "SDR")
                .test_enum_legacy("SERDES", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum_legacy("SERDES_MODE", &["MASTER", "SLAVE"]);
            bctx.mode("ISERDESE2")
                .attr("SERDES", "FALSE")
                .test_enum_legacy(
                    "DATA_WIDTH",
                    &["2", "3", "4", "5", "6", "7", "8", "10", "14"],
                );
            bctx.mode("ISERDESE2")
                .test_enum_legacy("NUM_CE", &["1", "2"]);

            for attr in [
                "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
                "SRVAL_Q4",
            ] {
                bctx.mode("ISERDESE2").test_enum_legacy(attr, &["0", "1"]);
            }

            bctx.mode("ILOGICE2")
                .attr("IFFTYPE", "#FF")
                .test_enum_legacy("SRTYPE", &["SYNC", "ASYNC"]);
            bctx.mode("ISERDESE2")
                .test_enum_legacy("SRTYPE", &["SYNC", "ASYNC"]);

            bctx.mode("ISERDESE2")
                .test_enum_legacy("D_EMU1", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum_legacy("D_EMU2", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum_legacy("RANK23_DLY", &["FALSE", "TRUE"]);

            bctx.mode("ISERDESE2").test_enum_legacy(
                "INTERFACE_TYPE",
                &[
                    "NETWORKING",
                    "MEMORY",
                    "MEMORY_DDR3",
                    "MEMORY_QDR",
                    "OVERSAMPLE",
                ],
            );
            bctx.mode("ISERDESE2")
                .test_manual_legacy("INTERFACE_TYPE", "MEMORY_DDR3_V6")
                .attr("INTERFACE_TYPE", "MEMORY_DDR3")
                .attr("DDR3_V6", "TRUE")
                .commit();
            bctx.mode("ISERDESE2")
                .test_enum_legacy("DATA_RATE", &["SDR", "DDR"]);
            bctx.mode("ISERDESE2").test_enum_legacy(
                "DDR_CLK_EDGE",
                &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
            );
            bctx.mode("ILOGICE2")
                .attr("IFFTYPE", "DDR")
                .test_enum_legacy(
                    "DDR_CLK_EDGE",
                    &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
                );
            bctx.mode("ILOGICE2")
                .test_enum_legacy("IFFTYPE", &["#FF", "#LATCH", "DDR"]);

            bctx.mode("ISERDESE2")
                .pin("OFB")
                .test_enum_legacy("OFB_USED", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .pin("TFB")
                .test_enum_legacy("TFB_USED", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum_legacy("IOBDELAY", &["NONE", "IFD", "IBUF", "BOTH"]);

            bctx.mode("ILOGICE2")
                .attr("IMUX", "0")
                .attr("IDELMUX", "1")
                .attr("IFFMUX", "#OFF")
                .attr("DINV", "")
                .pin("D")
                .pin("DDLY")
                .pin("TFB")
                .pin("OFB")
                .pin("O")
                .test_enum_legacy("D2OBYP_SEL", &["GND", "T"]);
            bctx.mode("ILOGICE2")
                .attr("IFFMUX", "0")
                .attr("IFFTYPE", "#FF")
                .attr("IFFDELMUX", "1")
                .attr("IMUX", "#OFF")
                .attr("DINV", "")
                .pin("D")
                .pin("DDLY")
                .pin("TFB")
                .pin("OFB")
                .test_enum_legacy("D2OFFBYP_SEL", &["GND", "T"]);
            bctx.mode("ILOGICE2")
                .attr("IDELMUX", "1")
                .attr("DINV", "")
                .pin("D")
                .pin("DDLY")
                .pin("O")
                .pin("TFB")
                .pin("OFB")
                .test_enum_legacy("IMUX", &["0", "1"]);
            bctx.mode("ILOGICE2")
                .attr("IFFDELMUX", "1")
                .attr("IFFTYPE", "#FF")
                .attr("DINV", "")
                .pin("D")
                .pin("DDLY")
                .pin("TFB")
                .pin("OFB")
                .test_enum_legacy("IFFMUX", &["0", "1"]);
            bctx.mode("ILOGICE2")
                .attr("IMUX", "1")
                .attr("IFFMUX", "1")
                .attr("IFFTYPE", "#FF")
                .attr("IFFDELMUX", "0")
                .attr("DINV", "")
                .pin("D")
                .pin("DDLY")
                .pin("O")
                .pin("Q1")
                .pin("TFB")
                .pin("OFB")
                .test_enum_legacy("IDELMUX", &["0", "1"]);
            bctx.mode("ILOGICE2")
                .attr("IMUX", "1")
                .attr("IFFMUX", "0")
                .attr("IFFTYPE", "#FF")
                .attr("IDELMUX", "0")
                .attr("D2OFFBYP_SEL", "T")
                .attr("DINV", "")
                .pin("D")
                .pin("DDLY")
                .pin("O")
                .pin("Q1")
                .pin("TFB")
                .pin("OFB")
                .test_enum_legacy("IFFDELMUX", &["0", "1"]);

            if !is_hp {
                bctx.test_manual_legacy("PRESENT", "ILOGICE3")
                    .mode("ILOGICE3")
                    .commit();
                for val in ["D", "D_B"] {
                    bctx.mode("ILOGICE3")
                        .attr("ZHOLD_IFF", "TRUE")
                        .attr("IFFTYPE", "#FF")
                        .pin("Q1")
                        .test_manual_legacy("ZHOLD_IFF_INV", val)
                        .attr("IFFDELMUXE3", "2")
                        .attr("IFFMUX", "1")
                        .attr("ZHOLD_IFF_INV", val)
                        .commit();
                }
                bctx.mode("ILOGICE3")
                    .attr("ZHOLD_FABRIC", "TRUE")
                    .attr("IDELMUXE3", "2")
                    .attr("IMUX", "1")
                    .pin("O")
                    .test_enum_legacy("ZHOLD_FABRIC_INV", &["D", "D_B"]);
                bctx.mode("ILOGICE3")
                    .attr("ZHOLD_IFF", "")
                    .test_enum_legacy("ZHOLD_FABRIC", &["FALSE", "TRUE"]);
                bctx.mode("ILOGICE3")
                    .attr("ZHOLD_FABRIC", "")
                    .test_enum_legacy("ZHOLD_IFF", &["FALSE", "TRUE"]);
                bctx.mode("ILOGICE3")
                    .test_multi_attr_dec_legacy("IDELAY_VALUE", 5);
                bctx.mode("ILOGICE3")
                    .test_multi_attr_dec_legacy("IFFDELAY_VALUE", 5);
            }
        }
    }
}

fn add_fuzzers_ologic<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for (tcid, num_io) in [
        (tcls::IO_HR_PAIR, 2),
        (tcls::IO_HR_S, 1),
        (tcls::IO_HR_N, 1),
        (tcls::IO_HP_PAIR, 2),
        (tcls::IO_HP_S, 1),
        (tcls::IO_HP_N, 1),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for i in 0..num_io {
            let mut bctx = ctx.bel(bslots::OLOGIC[i]);

            bctx.test_manual_legacy("PRESENT", "OLOGICE2")
                .mode("OLOGICE2")
                .commit();
            bctx.test_manual_legacy("PRESENT", "OSERDESE2")
                .mode("OSERDESE2")
                .commit();

            for pin in [
                "D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "T1", "T2", "T3", "T4", "CLKDIV",
                "CLKDIVF",
            ] {
                bctx.mode("OSERDESE2").test_inv_legacy(pin);
            }
            bctx.mode("OSERDESE2")
                .attr("DATA_RATE_OQ", "DDR")
                .attr("DDR_CLK_EDGE", "SAME_EDGE")
                .pin("OCE")
                .pin("CLK")
                .test_enum_suffix_legacy("CLKINV", "SAME", &["CLK", "CLK_B"]);
            bctx.mode("OSERDESE2")
                .attr("DATA_RATE_OQ", "DDR")
                .attr("DDR_CLK_EDGE", "OPPOSITE_EDGE")
                .pin("OCE")
                .pin("CLK")
                .test_enum_suffix_legacy("CLKINV", "OPPOSITE", &["CLK", "CLK_B"]);

            bctx.mode("OLOGICE2")
                .attr("OUTFFTYPE", "#FF")
                .test_enum_legacy("SRTYPE_OQ", &["SYNC", "ASYNC"]);
            bctx.mode("OLOGICE2")
                .attr("TFFTYPE", "#FF")
                .test_enum_legacy("SRTYPE_TQ", &["SYNC", "ASYNC"]);
            bctx.mode("OSERDESE2")
                .test_enum_legacy("SRTYPE", &["SYNC", "ASYNC"]);

            bctx.mode("OLOGICE2")
                .test_enum_suffix_legacy("INIT_OQ", "OLOGIC", &["0", "1"]);
            bctx.mode("OLOGICE2")
                .test_enum_suffix_legacy("INIT_TQ", "OLOGIC", &["0", "1"]);
            bctx.mode("OSERDESE2")
                .test_enum_suffix_legacy("INIT_OQ", "OSERDES", &["0", "1"]);
            bctx.mode("OSERDESE2")
                .test_enum_suffix_legacy("INIT_TQ", "OSERDES", &["0", "1"]);
            bctx.mode("OLOGICE2")
                .test_enum_suffix_legacy("SRVAL_OQ", "OLOGIC", &["0", "1"]);
            bctx.mode("OLOGICE2")
                .test_enum_suffix_legacy("SRVAL_TQ", "OLOGIC", &["0", "1"]);
            bctx.mode("OSERDESE2")
                .test_enum_suffix_legacy("SRVAL_OQ", "OSERDES", &["0", "1"]);
            bctx.mode("OSERDESE2")
                .test_enum_suffix_legacy("SRVAL_TQ", "OSERDES", &["0", "1"]);

            for attr in ["OSRUSED", "TSRUSED"] {
                bctx.mode("OLOGICE2")
                    .attr("OUTFFTYPE", "#FF")
                    .attr("TFFTYPE", "#FF")
                    .pin("OCE")
                    .pin("TCE")
                    .pin("REV")
                    .pin("SR")
                    .test_enum_legacy(attr, &["0"]);
            }

            bctx.mode("OLOGICE2")
                .pin("OQ")
                .test_enum_legacy("OUTFFTYPE", &["#FF", "#LATCH", "DDR"]);
            bctx.mode("OLOGICE2")
                .pin("TQ")
                .test_enum_legacy("TFFTYPE", &["#FF", "#LATCH", "DDR"]);
            bctx.mode("OLOGICE2")
                .test_manual_legacy("OMUX", "D1")
                .attr("OQUSED", "0")
                .attr("O1USED", "0")
                .attr("D1INV", "D1")
                .attr("OMUX", "D1")
                .pin("OQ")
                .pin("D1")
                .commit();

            bctx.mode("OSERDESE2")
                .test_enum_legacy("DATA_RATE_OQ", &["SDR", "DDR"]);
            bctx.mode("OSERDESE2")
                .test_enum_legacy("DATA_RATE_TQ", &["BUF", "SDR", "DDR"]);

            bctx.mode("OLOGICE2")
                .global("ENABLEMISR", "Y")
                .test_enum_legacy("MISR_ENABLE", &["FALSE", "TRUE"]);
            bctx.mode("OLOGICE2")
                .global("ENABLEMISR", "Y")
                .test_enum_legacy("MISR_ENABLE_FDBK", &["FALSE", "TRUE"]);
            bctx.mode("OLOGICE2")
                .global("ENABLEMISR", "Y")
                .test_enum_legacy("MISR_CLK_SELECT", &["CLK1", "CLK2"]);

            bctx.mode("OSERDESE2")
                .test_enum_legacy("SERDES", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum_legacy("SERDES_MODE", &["SLAVE", "MASTER"]);
            bctx.mode("OSERDESE2")
                .test_enum_legacy("SELFHEAL", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum_legacy("RANK3_USED", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum_legacy("TBYTE_CTL", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum_legacy("TBYTE_SRC", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum_legacy("TRISTATE_WIDTH", &["1", "4"]);
            bctx.mode("OSERDESE2")
                .attr("DATA_RATE_OQ", "SDR")
                .test_enum_suffix_legacy("DATA_WIDTH", "SDR", &["2", "3", "4", "5", "6", "7", "8"]);
            bctx.mode("OSERDESE2")
                .attr("DATA_RATE_OQ", "DDR")
                .test_enum_suffix_legacy("DATA_WIDTH", "DDR", &["4", "6", "8", "10", "14"]);
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .global("ENABLEMISR", "Y")
        .extra_tiles_by_kind_legacy("IO_HP_PAIR", "OLOGIC_COMMON")
        .extra_tiles_by_kind_legacy("IO_HR_PAIR", "OLOGIC_COMMON")
        .extra_tiles_by_kind_legacy("IO_HP_S", "OLOGIC[0]")
        .extra_tiles_by_kind_legacy("IO_HP_N", "OLOGIC[0]")
        .extra_tiles_by_kind_legacy("IO_HR_S", "OLOGIC[0]")
        .extra_tiles_by_kind_legacy("IO_HR_N", "OLOGIC[0]")
        .test_manual_legacy("NULL", "MISR_RESET", "1")
        .global_diff("MISRRESET", "N", "Y")
        .commit();
}

fn add_fuzzers_iodelay<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    for (tcid, is_hp, num_io) in [
        (tcls::IO_HR_PAIR, false, 2),
        (tcls::IO_HR_S, false, 1),
        (tcls::IO_HR_N, false, 1),
        (tcls::IO_HP_PAIR, true, 2),
        (tcls::IO_HP_S, true, 1),
        (tcls::IO_HP_N, true, 1),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let setup_idelayctrl: [Box<DynProp>; 4] = [
            Box::new(Related::new(
                HclkIoi,
                TileMutex::new("IDELAYCTRL".into(), "USE".into()),
            )),
            Box::new(Related::new(
                HclkIoi,
                BaseBelMode::new(bslots::IDELAYCTRL, 0, "IDELAYCTRL".into()),
            )),
            Box::new(Related::new(
                HclkIoi,
                BaseBelAttr::new(
                    bslots::IDELAYCTRL,
                    0,
                    "IDELAYCTRL_EN".into(),
                    "ENABLE".into(),
                ),
            )),
            Box::new(Related::new(
                HclkIoi,
                BaseBelAttr::new(bslots::IDELAYCTRL, 0, "BIAS_MODE".into(), "0".into()),
            )),
        ];
        for i in 0..num_io {
            let mut bctx = ctx.bel(bslots::IDELAY[i]);
            let bel_ologic = bslots::OLOGIC[i];
            bctx.build()
                .props(setup_idelayctrl.clone())
                .test_manual_legacy("ENABLE", "1")
                .mode("IDELAYE2")
                .commit();
            for pin in ["C", "IDATAIN", "DATAIN"] {
                bctx.mode("IDELAYE2")
                    .props(setup_idelayctrl.clone())
                    .attr("CINVCTRL_SEL", "FALSE")
                    .test_inv_legacy(pin);
            }
            for attr in [
                "HIGH_PERFORMANCE_MODE",
                "CINVCTRL_SEL",
                "DELAYCHAIN_OSC",
                "PIPE_SEL",
            ] {
                bctx.mode("IDELAYE2")
                    .props(setup_idelayctrl.clone())
                    .test_enum_legacy(attr, &["FALSE", "TRUE"]);
            }
            bctx.mode("IDELAYE2")
                .props(setup_idelayctrl.clone())
                .test_enum_legacy(
                    "IDELAY_TYPE",
                    &["FIXED", "VARIABLE", "VAR_LOAD", "VAR_LOAD_PIPE"],
                );
            bctx.mode("IDELAYE2")
                .props(setup_idelayctrl.clone())
                .test_enum_legacy("DELAY_SRC", &["DATAIN", "IDATAIN"]);
            bctx.build()
                .attr("DELAY_SRC", "")
                .test_manual_legacy("DELAY_SRC", "OFB")
                .pip("IDATAIN", (bel_ologic, "OFB"))
                .commit();
            bctx.mode("IDELAYE2")
                .props(setup_idelayctrl.clone())
                .attr("DELAY_SRC", "IDATAIN")
                .attr("IDELAY_TYPE", "FIXED")
                .test_multi_attr_dec_legacy("IDELAY_VALUE", 5);
            if is_hp {
                bctx.mode("IDELAYE2_FINEDELAY")
                    .props(setup_idelayctrl.clone())
                    .test_enum_legacy("FINEDELAY", &["BYPASS", "ADD_DLY"]);
            }
        }
        if is_hp {
            for i in 0..num_io {
                let mut bctx = ctx.bel(bslots::ODELAY[i]);
                bctx.build()
                    .props(setup_idelayctrl.clone())
                    .test_manual_legacy("PRESENT", "1")
                    .mode("ODELAYE2")
                    .commit();
                for pin in ["C", "ODATAIN"] {
                    bctx.mode("ODELAYE2")
                        .props(setup_idelayctrl.clone())
                        .attr("CINVCTRL_SEL", "FALSE")
                        .test_inv_legacy(pin);
                }
                for attr in [
                    "HIGH_PERFORMANCE_MODE",
                    "CINVCTRL_SEL",
                    "DELAYCHAIN_OSC",
                    "PIPE_SEL",
                ] {
                    bctx.mode("ODELAYE2")
                        .props(setup_idelayctrl.clone())
                        .attr("DELAY_SRC", "")
                        .test_enum_legacy(attr, &["FALSE", "TRUE"]);
                }
                bctx.mode("ODELAYE2")
                    .props(setup_idelayctrl.clone())
                    .attr("DELAY_SRC", "ODATAIN")
                    .attr("PIPE_SEL", "FALSE")
                    .test_enum_legacy("ODELAY_TYPE", &["FIXED", "VARIABLE", "VAR_LOAD"]);
                bctx.mode("ODELAYE2")
                    .props(setup_idelayctrl.clone())
                    .attr("DELAYCHAIN_OSC", "")
                    .test_enum_legacy("DELAY_SRC", &["ODATAIN", "CLKIN"]);
                bctx.mode("ODELAYE2")
                    .props(setup_idelayctrl.clone())
                    .attr("DELAY_SRC", "ODATAIN")
                    .attr("ODELAY_TYPE", "FIXED")
                    .test_multi_attr_dec_legacy("ODELAY_VALUE", 5);
                bctx.mode("ODELAYE2_FINEDELAY")
                    .props(setup_idelayctrl.clone())
                    .test_enum_legacy("FINEDELAY", &["BYPASS", "ADD_DLY"]);
            }
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    add_fuzzers_routing(session, backend);
    add_fuzzers_ilogic(session, backend);
    add_fuzzers_ologic(session, backend);
    add_fuzzers_iodelay(session, backend);
}
fn collect_fuzzers_routing(ctx: &mut CollectorCtx) {
    for (tcid, num_io) in [
        (tcls::IO_HR_PAIR, 2),
        (tcls::IO_HR_S, 1),
        (tcls::IO_HR_N, 1),
        (tcls::IO_HP_PAIR, 2),
        (tcls::IO_HP_S, 1),
        (tcls::IO_HP_N, 1),
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        };
        for io in 0..num_io {
            ctx.collect_mux(tcid, wires::IMUX_IOI_ICLK[0].cell(io));
            ctx.collect_mux(tcid, wires::IMUX_IOI_ICLK[1].cell(io));

            let dst0 = wires::IMUX_IOI_OCLK[0].cell(io);
            let dst1 = wires::IMUX_IOI_OCLK[1].cell(io);
            ctx.collect_mux(tcid, dst0);
            let mux = &ctx.edev.db_index[tcid].muxes[&dst0];
            let mut diffs = vec![(None, Diff::default())];
            for &src in mux.src.keys() {
                if src.wire == wires::PHASER_OCLK90 {
                    let mut diff = ctx.get_diff_routing_pair_special(
                        tcid,
                        dst1,
                        src,
                        specials::IOI_OCLK90_BOTH,
                    );
                    diff.apply_enum_diff_raw(ctx.sb_mux(tcid, dst0), &Some(src), &None);
                    diffs.push((Some(src), diff));
                } else {
                    diffs.push((Some(src), ctx.get_diff_routing(tcid, dst1, src)));
                }
            }
            ctx.insert_mux(tcid, dst1, xlat_enum_raw(diffs, OcdMode::Mux));

            for i in 0..2 {
                let dst = wires::IMUX_IOI_OCLKDIV[i].cell(io);
                let dst_f = wires::IMUX_IOI_OCLKDIVF[i].cell(io);

                let mut diffs_f = vec![];
                let mux = &ctx.edev.db_index[tcid].muxes[&dst_f];
                for &src in mux.src.keys() {
                    diffs_f.push((Some(src), ctx.get_diff_routing(tcid, dst, src)));
                }
                let diff_f = extract_common_diff(&mut diffs_f);
                diffs_f.push((None, Diff::default()));
                ctx.insert_mux(tcid, dst_f, xlat_enum_raw(diffs_f, OcdMode::Mux));

                let src_p = wires::PHASER_OCLKDIV.cell(io);
                let diffs = vec![
                    (None, Diff::default()),
                    (
                        Some(src_p.pos()),
                        ctx.get_diff_routing(tcid, dst, src_p.pos()),
                    ),
                    (Some(dst_f.pos()), diff_f),
                ];
                ctx.insert_mux(tcid, dst, xlat_enum_raw(diffs, OcdMode::Mux));
            }
        }
    }
}

fn collect_fuzzers_ilogic(ctx: &mut CollectorCtx) {
    for (tile, bel) in [
        ("IO_HR_PAIR", "ILOGIC[0]"),
        ("IO_HR_PAIR", "ILOGIC[1]"),
        ("IO_HR_S", "ILOGIC[0]"),
        ("IO_HR_N", "ILOGIC[0]"),
        ("IO_HP_PAIR", "ILOGIC[0]"),
        ("IO_HP_PAIR", "ILOGIC[1]"),
        ("IO_HP_S", "ILOGIC[0]"),
        ("IO_HP_N", "ILOGIC[0]"),
    ] {
        if !ctx.has_tile_legacy(tile) {
            continue;
        }

        ctx.collect_inv_legacy(tile, bel, "D");
        ctx.collect_inv_legacy(tile, bel, "CLKDIV");
        ctx.collect_inv_legacy(tile, bel, "CLKDIVP");
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.insert_legacy(tile, bel, "INV.CLK", item);
        let item = ctx.extract_bit_legacy(tile, bel, "OCLKINV", "OCLK");
        ctx.insert_legacy(tile, bel, "INV.OCLK1", item);
        let item = ctx.extract_bit_legacy(tile, bel, "OCLKINV", "OCLK_B");
        ctx.insert_legacy(tile, bel, "INV.OCLK2", item);
        ctx.collect_bit_bi_legacy(tile, bel, "DYN_CLK_INV_EN", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "DYN_CLKDIV_INV_EN", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "DYN_CLKDIVP_INV_EN", "FALSE", "TRUE");

        let iff_sr_used = ctx.extract_bit_legacy(tile, bel, "SRUSED", "0");
        ctx.insert_legacy(tile, bel, "IFF_SR_USED", iff_sr_used);
        ctx.collect_enum_legacy(tile, bel, "SERDES_MODE", &["MASTER", "SLAVE"]);
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10", "14"] {
            diffs.push((val, ctx.get_diff_legacy(tile, bel, "DATA_WIDTH", val)));
        }
        let mut bits = xlat_enum_legacy(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.insert_legacy(
            tile,
            bel,
            "DATA_WIDTH",
            xlat_enum_legacy_ocd(diffs, OcdMode::FixedOrder(&bits)),
        );
        ctx.collect_enum_legacy(tile, bel, "NUM_CE", &["1", "2"]);
        for (sattr, attr) in [
            ("INIT_Q1", "IFF1_INIT"),
            ("INIT_Q2", "IFF2_INIT"),
            ("INIT_Q3", "IFF3_INIT"),
            ("INIT_Q4", "IFF4_INIT"),
            ("SRVAL_Q1", "IFF1_SRVAL"),
            ("SRVAL_Q2", "IFF2_SRVAL"),
            ("SRVAL_Q3", "IFF3_SRVAL"),
            ("SRVAL_Q4", "IFF4_SRVAL"),
        ] {
            let item = ctx.extract_bit_bi_legacy(tile, bel, sattr, "0", "1");
            ctx.insert_legacy(tile, bel, attr, item);
        }
        ctx.collect_enum_legacy(tile, bel, "SRTYPE", &["ASYNC", "SYNC"]);
        ctx.collect_enum_legacy(tile, bel, "DATA_RATE", &["SDR", "DDR"]);
        ctx.collect_bit_bi_legacy(tile, bel, "D_EMU1", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "D_EMU2", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "RANK23_DLY", "FALSE", "TRUE");
        ctx.collect_enum_legacy(
            tile,
            bel,
            "DDR_CLK_EDGE",
            &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
        );

        let diff_mem = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "MEMORY");
        let diff_qdr = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "MEMORY_QDR");
        let diff_net = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "NETWORKING");
        let diff_ddr3 = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3");
        let diff_ddr3_v6 = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3_V6");
        let diff_os = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "OVERSAMPLE");
        let bitslip_en = diff_net.combine(&!&diff_qdr);
        let diff_net = diff_net.combine(&!&bitslip_en);
        let diff_os = diff_os.combine(&!&bitslip_en);
        ctx.insert_legacy(tile, bel, "BITSLIP_ENABLE", xlat_bit_legacy(bitslip_en));
        ctx.insert_legacy(
            tile,
            bel,
            "INTERFACE_TYPE",
            xlat_enum_legacy(vec![
                ("MEMORY", diff_mem),
                ("NETWORKING", diff_net),
                ("MEMORY_DDR3", diff_ddr3),
                ("MEMORY_DDR3_V6", diff_ddr3_v6),
                ("OVERSAMPLE", diff_os),
            ]),
        );

        let mut diff = ctx.get_diff_legacy(tile, bel, "IFFTYPE", "#LATCH");
        diff.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "DDR_CLK_EDGE"),
            "OPPOSITE_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "IFFTYPE", "#FF");
        diff.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "DDR_CLK_EDGE"),
            "OPPOSITE_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.insert_legacy(tile, bel, "IFF_LATCH", xlat_bit_legacy(!diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "IFFTYPE", "DDR");
        diff.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.insert_legacy(tile, bel, "IFF_LATCH", xlat_bit_legacy(!diff));

        let diff_f = ctx.get_diff_legacy(tile, bel, "SERDES", "FALSE");
        let diff_t = ctx.get_diff_legacy(tile, bel, "SERDES", "TRUE");
        let (diff_f, diff_t, mut diff_serdes) = Diff::split(diff_f, diff_t);
        ctx.insert_legacy(tile, bel, "SERDES", xlat_bit_bi_legacy(diff_f, diff_t));
        diff_serdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF_SR_USED"), true, false);
        diff_serdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF_LATCH"), false, true);
        diff_serdes.assert_empty();

        let item = ctx.extract_enum_legacy(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
        ctx.insert_legacy(tile, bel, "TSBYPASS_MUX", item);
        let item = ctx.extract_enum_legacy(tile, bel, "D2OFFBYP_SEL", &["GND", "T"]);
        ctx.insert_legacy(tile, bel, "TSBYPASS_MUX", item);
        let item = xlat_enum_legacy(vec![
            ("T", ctx.get_diff_legacy(tile, bel, "TFB_USED", "TRUE")),
            ("GND", ctx.get_diff_legacy(tile, bel, "TFB_USED", "FALSE")),
        ]);
        ctx.insert_legacy(tile, bel, "TSBYPASS_MUX", item);

        let item = ctx.extract_bit_bi_legacy(tile, bel, "IDELMUX", "1", "0");
        ctx.insert_legacy(tile, bel, "I_DELAY_ENABLE", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "IFFDELMUX", "1", "0");
        ctx.insert_legacy(tile, bel, "IFF_DELAY_ENABLE", item);

        ctx.get_diff_legacy(tile, bel, "IOBDELAY", "NONE")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "IOBDELAY", "IBUF");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "IOBDELAY", "IFD");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "IOBDELAY", "BOTH");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();

        let item = ctx.extract_bit_bi_legacy(tile, bel, "IMUX", "1", "0");
        ctx.insert_legacy(tile, bel, "I_TSBYPASS_ENABLE", item);
        // the fuzzer is slightly fucked to work around some ridiculous ISE bug.
        let _ = ctx.get_diff_legacy(tile, bel, "IFFMUX", "1");
        let item = ctx.extract_bit_legacy(tile, bel, "IFFMUX", "0");
        ctx.insert_legacy(tile, bel, "IFF_TSBYPASS_ENABLE", item);
        ctx.get_diff_legacy(tile, bel, "OFB_USED", "FALSE")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "OFB_USED", "TRUE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "IFF_TSBYPASS_ENABLE"),
            true,
            false,
        );
        diff.assert_empty();

        ctx.get_diff_legacy(tile, bel, "PRESENT", "ILOGICE2")
            .assert_empty();
        let mut present_iserdes = ctx.get_diff_legacy(tile, bel, "PRESENT", "ISERDESE2");
        present_iserdes.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "IFF1_SRVAL"),
            false,
            true,
        );
        present_iserdes.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "IFF2_SRVAL"),
            false,
            true,
        );
        present_iserdes.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "IFF3_SRVAL"),
            false,
            true,
        );
        present_iserdes.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "IFF4_SRVAL"),
            false,
            true,
        );
        present_iserdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF1_INIT"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF2_INIT"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF3_INIT"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF4_INIT"), false, true);
        present_iserdes.assert_empty();

        if tile.contains("HR") {
            ctx.get_diff_legacy(tile, bel, "PRESENT", "ILOGICE3")
                .assert_empty();

            ctx.collect_bitvec_legacy(tile, bel, "IDELAY_VALUE", "");
            ctx.collect_bitvec_legacy(tile, bel, "IFFDELAY_VALUE", "");
            let item = ctx.extract_bit_bi_legacy(tile, bel, "ZHOLD_FABRIC", "FALSE", "TRUE");
            ctx.insert_legacy(tile, bel, "ZHOLD_ENABLE", item);
            let item = ctx.extract_bit_bi_legacy(tile, bel, "ZHOLD_IFF", "FALSE", "TRUE");
            ctx.insert_legacy(tile, bel, "ZHOLD_ENABLE", item);

            let diff0 = ctx.get_diff_legacy(tile, bel, "ZHOLD_FABRIC_INV", "D");
            let diff1 = ctx.get_diff_legacy(tile, bel, "ZHOLD_FABRIC_INV", "D_B");
            let (diff0, diff1, diff_en) = Diff::split(diff0, diff1);
            ctx.insert_legacy(
                tile,
                bel,
                "INV.ZHOLD_FABRIC",
                xlat_bit_bi_legacy(diff0, diff1),
            );
            ctx.insert_legacy(tile, bel, "I_ZHOLD", xlat_bit_legacy(diff_en));

            let diff0 = ctx.get_diff_legacy(tile, bel, "ZHOLD_IFF_INV", "D");
            let diff1 = ctx.get_diff_legacy(tile, bel, "ZHOLD_IFF_INV", "D_B");
            let (diff0, diff1, diff_en) = Diff::split(diff0, diff1);
            ctx.insert_legacy(tile, bel, "INV.ZHOLD_IFF", xlat_bit_bi_legacy(diff0, diff1));
            ctx.insert_legacy(tile, bel, "IFF_ZHOLD", xlat_bit_legacy(diff_en));
        }
    }
}

fn collect_fuzzers_ologic(ctx: &mut CollectorCtx) {
    for (tile, bel) in [
        ("IO_HR_PAIR", "OLOGIC[0]"),
        ("IO_HR_PAIR", "OLOGIC[1]"),
        ("IO_HR_S", "OLOGIC[0]"),
        ("IO_HR_N", "OLOGIC[0]"),
        ("IO_HP_PAIR", "OLOGIC[0]"),
        ("IO_HP_PAIR", "OLOGIC[1]"),
        ("IO_HP_S", "OLOGIC[0]"),
        ("IO_HP_N", "OLOGIC[0]"),
    ] {
        if !ctx.has_tile_legacy(tile) {
            continue;
        }

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "T1", "T2", "T3", "T4", "CLKDIV",
            "CLKDIVF",
        ] {
            ctx.collect_inv_legacy(tile, bel, pin);
        }

        ctx.get_diff_legacy(tile, bel, "CLKINV.SAME", "CLK_B")
            .assert_empty();
        let diff_clk1 = ctx.get_diff_legacy(tile, bel, "CLKINV.OPPOSITE", "CLK");
        let diff_clk2 = ctx.get_diff_legacy(tile, bel, "CLKINV.OPPOSITE", "CLK_B");
        let diff_clk12 = ctx.get_diff_legacy(tile, bel, "CLKINV.SAME", "CLK");
        assert_eq!(diff_clk12, diff_clk1.combine(&diff_clk2));
        ctx.insert_legacy(tile, bel, "INV.CLK1", xlat_bit_legacy(!diff_clk1));
        ctx.insert_legacy(tile, bel, "INV.CLK2", xlat_bit_legacy(!diff_clk2));

        let item_oq = ctx.extract_bit_bi_legacy(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_bit_bi_legacy(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.get_diff_legacy(tile, bel, "SRTYPE", "ASYNC")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bit_diff_legacy(&item_oq, true, false);
        diff.apply_bit_diff_legacy(&item_tq, true, false);
        diff.assert_empty();
        ctx.insert_legacy(tile, bel, "OFF_SR_SYNC", item_oq);
        ctx.insert_legacy(tile, bel, "TFF_SR_SYNC", item_tq);

        let item = ctx.extract_bit_bi_legacy(tile, bel, "INIT_OQ.OLOGIC", "0", "1");
        ctx.insert_legacy(tile, bel, "OFF_INIT", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "INIT_OQ.OSERDES", "0", "1");
        ctx.insert_legacy(tile, bel, "OFF_INIT", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "INIT_TQ.OLOGIC", "0", "1");
        ctx.insert_legacy(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "INIT_TQ.OSERDES", "0", "1");
        ctx.insert_legacy(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRVAL_OQ.OLOGIC", "0", "1");
        ctx.insert_legacy(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRVAL_OQ.OSERDES", "0", "1");
        ctx.insert_legacy(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRVAL_TQ.OLOGIC", "0", "1");
        ctx.insert_legacy(tile, bel, "TFF_SRVAL", item);
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRVAL_TQ.OSERDES", "0", "1");
        ctx.insert_legacy(tile, bel, "TFF_SRVAL", item);

        let osrused = ctx.extract_bit_legacy(tile, bel, "OSRUSED", "0");
        let tsrused = ctx.extract_bit_legacy(tile, bel, "TSRUSED", "0");
        ctx.insert_legacy(tile, bel, "OFF_SR_USED", osrused);
        ctx.insert_legacy(tile, bel, "TFF_SR_USED", tsrused);

        ctx.collect_bit_bi_legacy(tile, bel, "MISR_ENABLE", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "MISR_ENABLE_FDBK", "FALSE", "TRUE");
        ctx.collect_enum_default_legacy(tile, bel, "MISR_CLK_SELECT", &["CLK1", "CLK2"], "NONE");
        if !tile.ends_with("PAIR") {
            ctx.collect_bit_legacy(tile, bel, "MISR_RESET", "1");
        }
        ctx.collect_bit_bi_legacy(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_bit_bi_legacy(tile, bel, "SELFHEAL", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "RANK3_USED", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "TBYTE_CTL", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "TBYTE_SRC", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "TRISTATE_WIDTH", &["1", "4"]);

        let mut diffs = vec![];
        for val in ["2", "3", "4", "5", "6", "7", "8"] {
            diffs.push((
                val,
                val,
                ctx.get_diff_legacy(tile, bel, "DATA_WIDTH.SDR", val),
            ));
        }
        for (val, ratio) in [("4", "2"), ("6", "3"), ("8", "4"), ("10", "5"), ("14", "7")] {
            diffs.push((
                val,
                ratio,
                ctx.get_diff_legacy(tile, bel, "DATA_WIDTH.DDR", val),
            ));
        }
        let mut diffs_width = vec![("NONE", Diff::default())];
        let mut diffs_ratio = vec![("NONE", Diff::default())];
        for &(width, ratio, ref diff) in &diffs {
            let mut diff_ratio = Diff::default();
            let mut diff_width = Diff::default();
            for (&bit, &val) in &diff.bits {
                if diffs
                    .iter()
                    .any(|&(owidth, _, ref odiff)| width != owidth && odiff.bits.contains_key(&bit))
                {
                    diff_ratio.bits.insert(bit, val);
                } else {
                    diff_width.bits.insert(bit, val);
                }
            }
            diffs_width.push((width, diff_width));
            let ratio = if matches!(ratio, "7" | "8") {
                "7_8"
            } else {
                ratio
            };
            diffs_ratio.push((ratio, diff_ratio));
        }
        ctx.insert_legacy(tile, bel, "DATA_WIDTH", xlat_enum_legacy(diffs_width));
        ctx.insert_legacy(tile, bel, "CLK_RATIO", xlat_enum_legacy(diffs_ratio));

        let mut diff_sdr = ctx.get_diff_legacy(tile, bel, "DATA_RATE_OQ", "SDR");
        let mut diff_ddr = ctx.get_diff_legacy(tile, bel, "DATA_RATE_OQ", "DDR");
        diff_sdr.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "OFF_SR_USED"), true, false);
        diff_ddr.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "OFF_SR_USED"), true, false);
        let item = xlat_enum_legacy(vec![
            ("NONE", Diff::default()),
            ("D1", ctx.get_diff_legacy(tile, bel, "OMUX", "D1")),
            ("SERDES_SDR", diff_sdr),
            ("DDR", diff_ddr),
            ("FF", ctx.get_diff_legacy(tile, bel, "OUTFFTYPE", "#FF")),
            ("DDR", ctx.get_diff_legacy(tile, bel, "OUTFFTYPE", "DDR")),
            (
                "LATCH",
                ctx.get_diff_legacy(tile, bel, "OUTFFTYPE", "#LATCH"),
            ),
        ]);
        ctx.insert_legacy(tile, bel, "OMUX", item);

        let mut diff_sdr = ctx.get_diff_legacy(tile, bel, "DATA_RATE_TQ", "SDR");
        let mut diff_ddr = ctx.get_diff_legacy(tile, bel, "DATA_RATE_TQ", "DDR");
        diff_sdr.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "TFF_SR_USED"), true, false);
        diff_ddr.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "TFF_SR_USED"), true, false);
        let item = xlat_enum_legacy(vec![
            ("NONE", Diff::default()),
            ("T1", ctx.get_diff_legacy(tile, bel, "DATA_RATE_TQ", "BUF")),
            ("SERDES_SDR", diff_sdr),
            ("DDR", diff_ddr),
            ("FF", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "#FF")),
            ("DDR", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "DDR")),
            ("LATCH", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "#LATCH")),
        ]);
        ctx.insert_legacy(tile, bel, "TMUX", item);

        let mut present_ologic = ctx.get_diff_legacy(tile, bel, "PRESENT", "OLOGICE2");
        present_ologic.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "RANK3_USED"), false, true);
        present_ologic.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "TMUX"), "T1", "NONE");
        present_ologic.assert_empty();
        let mut present_oserdes = ctx.get_diff_legacy(tile, bel, "PRESENT", "OSERDESE2");
        present_oserdes.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "OFF_SRVAL"), 0, 7);
        present_oserdes.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "TFF_SRVAL"), 0, 7);
        present_oserdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "OFF_INIT"), false, true);
        present_oserdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "TFF_INIT"), false, true);
        present_oserdes.assert_empty();
    }
    for tile in ["IO_HR_PAIR", "IO_HP_PAIR"] {
        if !ctx.has_tile_legacy(tile) {
            continue;
        }
        let mut diff = ctx.get_diff_legacy(tile, "OLOGIC_COMMON", "MISR_RESET", "1");
        let diff1 = diff.split_bits_by(|bit| bit.rect.to_idx() > 0);
        ctx.insert_legacy(tile, "OLOGIC[0]", "MISR_RESET", xlat_bit_legacy(diff));
        ctx.insert_legacy(tile, "OLOGIC[1]", "MISR_RESET", xlat_bit_legacy(diff1));
    }
}

fn collect_fuzzers_iodelay(ctx: &mut CollectorCtx) {
    for (tile, bel) in [
        ("IO_HR_PAIR", "IDELAY[0]"),
        ("IO_HR_PAIR", "IDELAY[1]"),
        ("IO_HR_S", "IDELAY[0]"),
        ("IO_HR_N", "IDELAY[0]"),
        ("IO_HP_PAIR", "IDELAY[0]"),
        ("IO_HP_PAIR", "IDELAY[1]"),
        ("IO_HP_S", "IDELAY[0]"),
        ("IO_HP_N", "IDELAY[0]"),
    ] {
        if !ctx.has_tile_legacy(tile) {
            continue;
        }
        ctx.collect_bit_legacy(tile, bel, "ENABLE", "1");
        ctx.collect_inv_legacy(tile, bel, "C");
        ctx.collect_inv_legacy(tile, bel, "DATAIN");
        ctx.collect_inv_legacy(tile, bel, "IDATAIN");
        ctx.collect_bit_bi_legacy(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "CINVCTRL_SEL", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "PIPE_SEL", "FALSE", "TRUE");

        ctx.get_diff_legacy(tile, bel, "DELAYCHAIN_OSC", "FALSE")
            .assert_empty();
        let item = xlat_enum_legacy(vec![
            ("NONE", Diff::default()),
            (
                "IDATAIN",
                ctx.get_diff_legacy(tile, bel, "DELAY_SRC", "IDATAIN"),
            ),
            (
                "DATAIN",
                ctx.get_diff_legacy(tile, bel, "DELAY_SRC", "DATAIN"),
            ),
            ("OFB", ctx.get_diff_legacy(tile, bel, "DELAY_SRC", "OFB")),
            (
                "DELAYCHAIN_OSC",
                ctx.get_diff_legacy(tile, bel, "DELAYCHAIN_OSC", "TRUE"),
            ),
        ]);
        ctx.insert_legacy(tile, bel, "DELAY_SRC", item);

        let item = xlat_enum_legacy(vec![
            (
                "FIXED",
                ctx.get_diff_legacy(tile, bel, "IDELAY_TYPE", "FIXED"),
            ),
            (
                "VARIABLE",
                ctx.get_diff_legacy(tile, bel, "IDELAY_TYPE", "VARIABLE"),
            ),
            (
                "VAR_LOAD",
                ctx.get_diff_legacy(tile, bel, "IDELAY_TYPE", "VAR_LOAD"),
            ),
            (
                "VAR_LOAD",
                ctx.get_diff_legacy(tile, bel, "IDELAY_TYPE", "VAR_LOAD_PIPE"),
            ),
        ]);
        ctx.insert_legacy(tile, bel, "IDELAY_TYPE", item);
        let mut diffs_t = vec![];
        let mut diffs_f = vec![];
        for diff in ctx.get_diffs_legacy(tile, bel, "IDELAY_VALUE", "") {
            let mut diff_t = Diff::default();
            let mut diff_f = Diff::default();
            for (k, v) in diff.bits {
                if v {
                    diff_t.bits.insert(k, v);
                } else {
                    diff_f.bits.insert(k, v);
                }
            }
            diffs_t.push(diff_t);
            diffs_f.push(diff_f);
        }
        ctx.insert_legacy(tile, bel, "IDELAY_VALUE_INIT", xlat_bitvec_legacy(diffs_t));
        ctx.insert_legacy(tile, bel, "IDELAY_VALUE_CUR", xlat_bitvec_legacy(diffs_f));
        if tile.contains("HP") {
            ctx.collect_enum_legacy(tile, bel, "FINEDELAY", &["BYPASS", "ADD_DLY"]);
        }
    }
    for (tile, bel) in [
        ("IO_HP_PAIR", "ODELAY[0]"),
        ("IO_HP_PAIR", "ODELAY[1]"),
        ("IO_HP_S", "ODELAY[0]"),
        ("IO_HP_N", "ODELAY[0]"),
    ] {
        if !ctx.has_tile_legacy(tile) {
            continue;
        }
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
        ctx.collect_inv_legacy(tile, bel, "C");
        ctx.collect_inv_legacy(tile, bel, "ODATAIN");
        ctx.collect_bit_bi_legacy(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "CINVCTRL_SEL", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "PIPE_SEL", "FALSE", "TRUE");
        ctx.get_diff_legacy(tile, bel, "DELAYCHAIN_OSC", "FALSE")
            .assert_empty();

        let item = xlat_enum_legacy(vec![
            ("NONE", Diff::default()),
            (
                "ODATAIN",
                ctx.get_diff_legacy(tile, bel, "DELAY_SRC", "ODATAIN"),
            ),
            (
                "CLKIN",
                ctx.get_diff_legacy(tile, bel, "DELAY_SRC", "CLKIN"),
            ),
            (
                "DELAYCHAIN_OSC",
                ctx.get_diff_legacy(tile, bel, "DELAYCHAIN_OSC", "TRUE"),
            ),
        ]);
        ctx.insert_legacy(tile, bel, "DELAY_SRC", item);

        let en = ctx.extract_bit_legacy(tile, bel, "ODELAY_TYPE", "FIXED");
        let mut diff_var = ctx.get_diff_legacy(tile, bel, "ODELAY_TYPE", "VARIABLE");
        diff_var.apply_bit_diff_legacy(&en, true, false);
        let mut diff_vl = ctx.get_diff_legacy(tile, bel, "ODELAY_TYPE", "VAR_LOAD");
        diff_vl.apply_bit_diff_legacy(&en, true, false);
        ctx.insert_legacy(tile, bel, "ENABLE", en);
        ctx.insert_legacy(
            tile,
            bel,
            "ODELAY_TYPE",
            xlat_enum_legacy(vec![
                ("FIXED", Diff::default()),
                ("VARIABLE", diff_var),
                ("VAR_LOAD", diff_vl),
            ]),
        );

        let mut diffs_t = vec![];
        let mut diffs_f = vec![];
        for diff in ctx.get_diffs_legacy(tile, bel, "ODELAY_VALUE", "") {
            let mut diff_t = Diff::default();
            let mut diff_f = Diff::default();
            for (k, v) in diff.bits {
                if v {
                    diff_t.bits.insert(k, v);
                } else {
                    diff_f.bits.insert(k, v);
                }
            }
            diffs_t.push(diff_t);
            diffs_f.push(diff_f);
        }
        ctx.insert_legacy(tile, bel, "ODELAY_VALUE_INIT", xlat_bitvec_legacy(diffs_t));
        ctx.insert_legacy(tile, bel, "ODELAY_VALUE_CUR", xlat_bitvec_legacy(diffs_f));
        ctx.collect_enum_legacy(tile, bel, "FINEDELAY", &["BYPASS", "ADD_DLY"]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    collect_fuzzers_routing(ctx);
    collect_fuzzers_ilogic(ctx);
    collect_fuzzers_ologic(ctx);
    collect_fuzzers_iodelay(ctx);
}
