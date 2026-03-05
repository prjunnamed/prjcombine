use prjcombine_entity::EntityId;
use prjcombine_interconnect::db::WireSlotIdExt;
use prjcombine_re_collector::{
    diff::{Diff, OcdMode},
    legacy::{
        extract_bitvec_val_part_legacy, xlat_bit_bi_legacy, xlat_bit_legacy, xlat_bitvec_legacy,
        xlat_enum_legacy, xlat_enum_legacy_ocd,
    },
};
use prjcombine_re_hammer::Session;
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem},
};
use prjcombine_virtex4::defs::{
    bslots,
    virtex6::{tcls, wires},
};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::{BaseIntPip, FuzzIntPip},
        props::mutex::{WireMutexExclusive, WireMutexShared},
    },
    virtex5::io::HclkIoi,
};

fn add_fuzzers_routing<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::IO);
    for c in 0..2 {
        for w in [
            wires::IMUX_IOI_ICLK,
            wires::IMUX_IOI_OCLK,
            wires::IMUX_IOI_OCLKDIV,
        ] {
            let dst_a = w[0].cell(c);
            let dst_b = w[1].cell(c);
            let mux = &backend.edev.db_index.tile_classes[tcls::IO].muxes[&dst_a];
            for &src in mux.src.keys() {
                ctx.build()
                    .prop(WireMutexExclusive::new(dst_a))
                    .prop(WireMutexExclusive::new(dst_b))
                    .prop(WireMutexShared::new(src.tw))
                    .prop(BaseIntPip::new(dst_b, src.tw))
                    .test_routing(dst_a, src)
                    .prop(FuzzIntPip::new(dst_a, src.tw))
                    .commit();
                ctx.build()
                    .prop(WireMutexExclusive::new(dst_b))
                    .prop(WireMutexShared::new(src.tw))
                    .test_routing(dst_b, src)
                    .prop(FuzzIntPip::new(dst_b, src.tw))
                    .commit();
            }
        }
    }
}

fn add_fuzzers_ilogic<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::IO);
    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::ILOGIC[i]);

        bctx.test_manual_legacy("PRESENT", "ILOGIC")
            .mode("ILOGICE1")
            .commit();
        bctx.test_manual_legacy("PRESENT", "ISERDES")
            .mode("ISERDESE1")
            .commit();

        bctx.mode("ISERDESE1").test_inv_legacy("D");
        bctx.mode("ISERDESE1").test_inv_legacy("CLK");
        bctx.mode("ISERDESE1")
            .attr("DYN_CLKDIV_INV_EN", "FALSE")
            .test_inv_legacy("CLKDIV");
        bctx.mode("ISERDESE1")
            .test_enum_legacy("DYN_CLK_INV_EN", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum_legacy("DYN_OCLK_INV_EN", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum_legacy("DYN_CLKDIV_INV_EN", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .attr("OVERSAMPLE", "FALSE")
            .attr("DYN_OCLK_INV_EN", "FALSE")
            .attr("INTERFACE_TYPE", "")
            .pin("OCLK")
            .test_enum_suffix_legacy("OCLKINV", "SDR", &["OCLK", "OCLK_B"]);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "DDR")
            .attr("OVERSAMPLE", "FALSE")
            .attr("DYN_OCLK_INV_EN", "FALSE")
            .attr("INTERFACE_TYPE", "")
            .pin("OCLK")
            .test_enum_suffix_legacy("OCLKINV", "DDR", &["OCLK", "OCLK_B"]);

        bctx.mode("ILOGICE1")
            .attr("IFFTYPE", "#FF")
            .pin("SR")
            .test_enum_legacy("SRUSED", &["0"]);
        bctx.mode("ILOGICE1")
            .attr("IFFTYPE", "#FF")
            .pin("REV")
            .test_enum_legacy("REVUSED", &["0"]);
        bctx.mode("ISERDESE1")
            .attr("DATA_WIDTH", "2")
            .attr("DATA_RATE", "SDR")
            .test_enum_legacy("SERDES", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum_legacy("SERDES_MODE", &["MASTER", "SLAVE"]);
        bctx.mode("ISERDESE1")
            .attr("SERDES", "FALSE")
            .test_enum_legacy("DATA_WIDTH", &["2", "3", "4", "5", "6", "7", "8", "10"]);
        bctx.mode("ISERDESE1")
            .test_enum_legacy("NUM_CE", &["1", "2"]);

        for attr in [
            "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
            "SRVAL_Q4",
        ] {
            bctx.mode("ISERDESE1").test_enum_legacy(attr, &["0", "1"]);
        }

        bctx.mode("ILOGICE1")
            .attr("IFFTYPE", "#FF")
            .test_enum_suffix_legacy("SRTYPE", "ILOGIC", &["SYNC", "ASYNC"]);
        bctx.mode("ISERDESE1")
            .test_enum_suffix_legacy("SRTYPE", "ISERDES", &["SYNC", "ASYNC"]);

        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin_legacy("INIT_CE", 2);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin_legacy("INIT_BITSLIPCNT", 4);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin_legacy("INIT_BITSLIP", 6);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin_legacy("INIT_RANK1_PARTIAL", 5);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin_legacy("INIT_RANK2", 6);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin_legacy("INIT_RANK3", 6);

        bctx.mode("ISERDESE1")
            .pin("OFB")
            .test_enum_legacy("OFB_USED", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .pin("TFB")
            .test_enum_legacy("TFB_USED", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum_legacy("IOBDELAY", &["NONE", "IFD", "IBUF", "BOTH"]);

        bctx.mode("ILOGICE1")
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
        bctx.mode("ILOGICE1")
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
        bctx.mode("ILOGICE1")
            .attr("IDELMUX", "1")
            .attr("DINV", "")
            .pin("D")
            .pin("DDLY")
            .pin("O")
            .pin("TFB")
            .pin("OFB")
            .test_enum_legacy("IMUX", &["0", "1"]);
        bctx.mode("ILOGICE1")
            .attr("IFFDELMUX", "1")
            .attr("IFFTYPE", "#FF")
            .attr("DINV", "")
            .pin("D")
            .pin("DDLY")
            .pin("TFB")
            .pin("OFB")
            .test_enum_legacy("IFFMUX", &["0", "1"]);
        bctx.mode("ILOGICE1")
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
        bctx.mode("ILOGICE1")
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

        bctx.mode("ISERDESE1")
            .test_enum_legacy("D_EMU", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1").test_enum_legacy(
            "D_EMU_OPTION",
            &["MATCH_DLY0", "MATCH_DLY2", "DLY0", "DLY1", "DLY2", "DLY3"],
        );
        bctx.mode("ISERDESE1")
            .test_enum_legacy("RANK12_DLY", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum_legacy("RANK23_DLY", &["FALSE", "TRUE"]);

        bctx.mode("ISERDESE1")
            .attr("OVERSAMPLE", "FALSE")
            .test_enum_legacy(
                "INTERFACE_TYPE",
                &[
                    "NETWORKING",
                    "MEMORY",
                    "MEMORY_DDR3",
                    "MEMORY_QDR",
                    "OVERSAMPLE",
                ],
            );
        bctx.mode("ISERDESE1")
            .attr("INIT_BITSLIPCNT", "1111")
            .attr("INIT_RANK1_PARTIAL", "11111")
            .attr("INIT_RANK2", "111111")
            .attr("INIT_RANK3", "111111")
            .attr("INIT_CE", "11")
            .test_enum_legacy("DATA_RATE", &["SDR", "DDR"]);
        bctx.mode("ISERDESE1").test_enum_legacy(
            "DDR_CLK_EDGE",
            &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
        );
        bctx.mode("ILOGICE1")
            .attr("IFFTYPE", "DDR")
            .test_enum_legacy(
                "DDR_CLK_EDGE",
                &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
            );
        bctx.mode("ILOGICE1")
            .test_enum_legacy("IFFTYPE", &["#FF", "#LATCH", "DDR"]);
    }
}

fn add_fuzzers_ologic<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::IO);
    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::OLOGIC[i]);

        bctx.test_manual_legacy("PRESENT", "OLOGIC")
            .mode("OLOGICE1")
            .commit();
        bctx.test_manual_legacy("PRESENT", "OSERDES")
            .mode("OSERDESE1")
            .commit();

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "T2", "T3", "T4", "CLKDIV", "CLKPERF",
        ] {
            bctx.mode("OSERDESE1").test_inv_legacy(pin);
        }
        bctx.mode("OLOGICE1")
            .attr("TMUX", "T1")
            .attr("T1USED", "0")
            .pin("TQ")
            .test_inv_legacy("T1");
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("DDR_CLK_EDGE", "SAME_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix_legacy("CLKINV", "SAME", &["CLK", "CLK_B"]);
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("DDR_CLK_EDGE", "OPPOSITE_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix_legacy("CLKINV", "OPPOSITE", &["CLK", "CLK_B"]);

        bctx.mode("OLOGICE1")
            .attr("OUTFFTYPE", "#FF")
            .test_enum_legacy("SRTYPE_OQ", &["SYNC", "ASYNC"]);
        bctx.mode("OLOGICE1")
            .attr("TFFTYPE", "#FF")
            .test_enum_legacy("SRTYPE_TQ", &["SYNC", "ASYNC"]);
        bctx.mode("OSERDESE1")
            .test_enum_legacy("SRTYPE", &["SYNC", "ASYNC"]);

        bctx.mode("OLOGICE1")
            .test_enum_suffix_legacy("INIT_OQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OLOGICE1")
            .test_enum_suffix_legacy("INIT_TQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_suffix_legacy("INIT_OQ", "OSERDES", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_suffix_legacy("INIT_TQ", "OSERDES", &["0", "1"]);
        bctx.mode("OLOGICE1")
            .test_enum_suffix_legacy("SRVAL_OQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OLOGICE1")
            .test_enum_suffix_legacy("SRVAL_TQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_suffix_legacy("SRVAL_OQ", "OSERDES", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_suffix_legacy("SRVAL_TQ", "OSERDES", &["0", "1"]);

        for attr in [
            "OSRUSED", "TSRUSED", "OREVUSED", "TREVUSED", "OCEUSED", "TCEUSED",
        ] {
            bctx.mode("OLOGICE1")
                .attr("OUTFFTYPE", "#FF")
                .attr("TFFTYPE", "#FF")
                .pin("OCE")
                .pin("TCE")
                .pin("REV")
                .pin("SR")
                .test_enum_legacy(attr, &["0"]);
        }

        bctx.mode("OLOGICE1")
            .attr("TFFTYPE", "")
            .pin("OQ")
            .test_enum_legacy("OUTFFTYPE", &["#FF", "#LATCH", "DDR"]);
        bctx.mode("OLOGICE1")
            .attr("OUTFFTYPE", "")
            .pin("TQ")
            .test_enum_legacy("TFFTYPE", &["#FF", "#LATCH", "DDR"]);

        bctx.mode("OSERDESE1")
            .test_enum_legacy("DATA_RATE_OQ", &["SDR", "DDR"]);
        bctx.mode("OSERDESE1")
            .attr("T1INV", "T1")
            .pin("T1")
            .test_enum_legacy("DATA_RATE_TQ", &["BUF", "SDR", "DDR"]);

        bctx.mode("OLOGICE1")
            .global("ENABLEMISR", "Y")
            .test_enum_legacy("MISR_ENABLE", &["FALSE", "TRUE"]);
        bctx.mode("OLOGICE1")
            .global("ENABLEMISR", "Y")
            .test_enum_legacy("MISR_ENABLE_FDBK", &["FALSE", "TRUE"]);
        bctx.mode("OLOGICE1")
            .global("ENABLEMISR", "Y")
            .test_enum_legacy("MISR_CLK_SELECT", &["CLK1", "CLK2"]);

        bctx.mode("OSERDESE1")
            .test_enum_legacy("SERDES", &["FALSE", "TRUE"]);
        bctx.mode("OSERDESE1")
            .test_enum_legacy("SERDES_MODE", &["SLAVE", "MASTER"]);
        bctx.mode("OSERDESE1")
            .test_enum_legacy("SELFHEAL", &["FALSE", "TRUE"]);
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "SDR")
            .test_enum_legacy("INTERFACE_TYPE", &["DEFAULT", "MEMORY_DDR3"]);
        bctx.mode("OSERDESE1")
            .test_enum_legacy("TRISTATE_WIDTH", &["1", "4"]);
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "SDR")
            .attr("INTERFACE_TYPE", "DEFAULT")
            .test_enum_suffix_legacy("DATA_WIDTH", "SDR", &["2", "3", "4", "5", "6", "7", "8"]);
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("INTERFACE_TYPE", "DEFAULT")
            .test_enum_suffix_legacy("DATA_WIDTH", "DDR", &["4", "6", "8", "10"]);
        bctx.mode("OSERDESE1")
            .test_enum_legacy("WC_DELAY", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_legacy("DDR3_DATA", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_legacy("ODELAY_USED", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_LOADCNT", 4);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_ORANK1", 6);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_ORANK2_PARTIAL", 4);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_TRANK1", 4);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_FIFO_ADDR", 11);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_FIFO_RESET", 13);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_DLY_CNT", 10);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_PIPE_DATA0", 12);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin_legacy("INIT_PIPE_DATA1", 12);
    }
    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        ctx.build()
            .extra_tiles_by_bel_legacy(bslots::OLOGIC[0], "OLOGIC_COMMON")
            .global("ENABLEMISR", "Y")
            .test_manual_legacy("OLOGIC_COMMON", "MISR_RESET", "1")
            .global_diff("MISRRESET", "N", "Y")
            .commit();
    }
}

fn add_fuzzers_iodelay<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::IO);
    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::IODELAY[i]);
        let bel_other = bslots::IODELAY[i ^ 1];

        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .test_manual_legacy("PRESENT", "1")
            .mode("IODELAYE1")
            .commit();
        for pin in ["C", "DATAIN", "IDATAIN"] {
            bctx.mode("IODELAYE1")
                .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
                .test_inv_legacy(pin);
        }
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .test_enum_legacy("CINVCTRL_SEL", &["FALSE", "TRUE"]);
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .test_enum_legacy("HIGH_PERFORMANCE_MODE", &["FALSE", "TRUE"]);
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .test_enum_legacy("DELAY_SRC", &["I", "O", "IO", "DATAIN", "CLKIN"]);
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .test_manual_legacy("DELAY_SRC", "DELAYCHAIN_OSC")
            .attr("DELAY_SRC", "I")
            .attr("DELAYCHAIN_OSC", "TRUE")
            .commit();
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .attr("DELAY_SRC", "IO")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .test_multi_attr_dec_legacy("IDELAY_VALUE", 5);
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .attr("DELAY_SRC", "IO")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .test_multi_attr_dec_legacy("ODELAY_VALUE", 5);
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "DEFAULT")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "I_DEFAULT")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "DEFAULT")
            .attr("DELAY_SRC", "I")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "I_FIXED")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("DELAY_SRC", "I")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "VARIABLE")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "I_VARIABLE")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "VARIABLE")
            .attr("DELAY_SRC", "I")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "VAR_LOADABLE")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "I_VAR_LOADABLE")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "VAR_LOADABLE")
            .attr("DELAY_SRC", "I")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "O_FIXED")
            .mode("IODELAYE1")
            .attr("ODELAY_TYPE", "FIXED")
            .attr("DELAY_SRC", "O")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "O_VARIABLE")
            .mode("IODELAYE1")
            .attr("ODELAY_TYPE", "VARIABLE")
            .attr("DELAY_SRC", "O")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "O_VAR_LOADABLE")
            .mode("IODELAYE1")
            .attr("ODELAY_TYPE", "VAR_LOADABLE")
            .attr("DELAY_SRC", "O")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "IO_FIXED")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .attr("DELAY_SRC", "IO")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "I_VARIABLE_O_FIXED")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "VARIABLE")
            .attr("ODELAY_TYPE", "FIXED")
            .attr("DELAY_SRC", "IO")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "I_FIXED_O_VARIABLE")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "VARIABLE")
            .attr("DELAY_SRC", "IO")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual_legacy("MODE", "IO_VAR_LOADABLE")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "VAR_LOADABLE")
            .attr("ODELAY_TYPE", "VAR_LOADABLE")
            .attr("DELAY_SRC", "IO")
            .commit();
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let mut ctx = FuzzCtx::new(session, backend, tcls::IO);
    if devdata_only {
        for i in 0..2 {
            let mut bctx = ctx.bel(bslots::IODELAY[i]);
            let bel_other = bslots::IODELAY[i ^ 1];
            bctx.build()
                .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
                .bel_mode(bel_other, "IODELAYE1")
                .bel_attr(bel_other, "IDELAY_TYPE", "DEFAULT")
                .bel_attr(bel_other, "DELAY_SRC", "I")
                .test_manual_legacy("MODE", "I_DEFAULT")
                .mode("IODELAYE1")
                .attr("IDELAY_TYPE", "DEFAULT")
                .attr("DELAY_SRC", "I")
                .commit();
        }
        return;
    }

    add_fuzzers_routing(session, backend);
    add_fuzzers_ilogic(session, backend);
    add_fuzzers_ologic(session, backend);
    add_fuzzers_iodelay(session, backend);
}

fn collect_fuzzers_routing(ctx: &mut CollectorCtx) {
    let tcid = tcls::IO;
    for c in 0..2 {
        for w in [
            wires::IMUX_IOI_ICLK,
            wires::IMUX_IOI_OCLK,
            wires::IMUX_IOI_OCLKDIV,
        ] {
            ctx.collect_mux(tcid, w[0].cell(c));
            ctx.collect_mux(tcid, w[1].cell(c));
        }
    }
}

fn collect_fuzzers_ilogic(ctx: &mut CollectorCtx) {
    let tile = "IO";

    for i in 0..2 {
        let bel = &format!("ILOGIC[{i}]");

        ctx.collect_inv_legacy(tile, bel, "D");
        ctx.collect_inv_legacy(tile, bel, "CLKDIV");
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.insert_legacy(tile, bel, "INV.CLK", item);

        let diff1 = ctx.get_diff_legacy(tile, bel, "OCLKINV.DDR", "OCLK");
        let diff2 = ctx.get_diff_legacy(tile, bel, "OCLKINV.DDR", "OCLK_B");
        ctx.get_diff_legacy(tile, bel, "OCLKINV.SDR", "OCLK_B")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "OCLKINV.SDR", "OCLK");
        diff = diff.combine(&!&diff1);
        diff = diff.combine(&!&diff2);
        diff.assert_empty();
        ctx.insert_legacy(tile, bel, "INV.OCLK1", xlat_bit_legacy(!diff1));
        ctx.insert_legacy(tile, bel, "INV.OCLK2", xlat_bit_legacy(!diff2));

        ctx.collect_bit_bi_legacy(tile, bel, "DYN_CLK_INV_EN", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "DYN_CLKDIV_INV_EN", "FALSE", "TRUE");
        ctx.collect_bit_wide_bi_legacy(tile, bel, "DYN_OCLK_INV_EN", "FALSE", "TRUE");

        let iff_rev_used = ctx.extract_bit_legacy(tile, bel, "REVUSED", "0");
        ctx.insert_legacy(tile, bel, "IFF_REV_USED", iff_rev_used);
        let iff_sr_used = ctx.extract_bit_legacy(tile, bel, "SRUSED", "0");
        ctx.insert_legacy(tile, bel, "IFF_SR_USED", iff_sr_used);
        ctx.collect_bit_bi_legacy(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "SERDES_MODE", &["MASTER", "SLAVE"]);
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10"] {
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
        ctx.collect_bitvec_legacy(tile, bel, "INIT_RANK1_PARTIAL", "");
        ctx.collect_bitvec_legacy(tile, bel, "INIT_RANK2", "");
        ctx.collect_bitvec_legacy(tile, bel, "INIT_RANK3", "");
        ctx.collect_bitvec_legacy(tile, bel, "INIT_BITSLIP", "");
        ctx.collect_bitvec_legacy(tile, bel, "INIT_BITSLIPCNT", "");
        ctx.collect_bitvec_legacy(tile, bel, "INIT_CE", "");
        let item = ctx.extract_bit_bi_legacy(tile, bel, "SRTYPE.ILOGIC", "ASYNC", "SYNC");
        ctx.insert_legacy(tile, bel, "IFF_SR_SYNC", item);
        ctx.get_diff_legacy(tile, bel, "SRTYPE.ISERDES", "ASYNC")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "SRTYPE.ISERDES", "SYNC");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF_SR_SYNC"), true, false);
        ctx.insert_legacy(tile, bel, "BITSLIP_SYNC", xlat_bit_legacy(diff));
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
        let diff_os = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "OVERSAMPLE");
        let bitslip_en = diff_net.combine(&!&diff_qdr);
        let diff_ddr3 = diff_ddr3.combine(&!&bitslip_en);
        let diff_os = diff_os.combine(&!&bitslip_en);
        ctx.insert_legacy(tile, bel, "BITSLIP_ENABLE", xlat_bit_legacy(bitslip_en));
        ctx.insert_legacy(
            tile,
            bel,
            "INTERFACE_TYPE",
            xlat_enum_legacy(vec![
                ("MEMORY", diff_mem),
                ("NETWORKING", diff_qdr),
                ("MEMORY_DDR3", diff_ddr3),
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
        ctx.insert_legacy(tile, bel, "IFF_LATCH", xlat_bit_legacy(!diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "IFFTYPE", "DDR");
        diff.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.insert_legacy(tile, bel, "IFF_LATCH", xlat_bit_legacy(!diff));

        let mut diffs = vec![];
        for val in ["SDR", "DDR"] {
            let mut diff = ctx.get_diff_legacy(tile, bel, "DATA_RATE", val);
            diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF_SR_USED"), true, false);
            diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "IFF_LATCH"), false, true);
            diffs.push((val, diff));
        }
        ctx.insert_legacy(tile, bel, "DATA_RATE", xlat_enum_legacy(diffs));

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

        ctx.collect_bit_bi_legacy(tile, bel, "D_EMU", "FALSE", "TRUE");
        ctx.collect_enum_legacy(
            tile,
            bel,
            "D_EMU_OPTION",
            &["DLY0", "DLY1", "DLY2", "DLY3", "MATCH_DLY0", "MATCH_DLY2"],
        );
        ctx.collect_bit_bi_legacy(tile, bel, "RANK12_DLY", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "RANK23_DLY", "FALSE", "TRUE");

        ctx.get_diff_legacy(tile, bel, "PRESENT", "ILOGIC")
            .assert_empty();
        let mut present_iserdes = ctx.get_diff_legacy(tile, bel, "PRESENT", "ISERDES");
        present_iserdes.apply_enum_diff_legacy(
            ctx.item_legacy(tile, bel, "TSBYPASS_MUX"),
            "GND",
            "T",
        );
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

        ctx.insert_legacy(
            tile,
            bel,
            "READBACK_I",
            TileItem::from_bit_inv([TileBit::new(0, 26, 61), TileBit::new(1, 27, 2)][i], false),
        );
    }
}

fn collect_fuzzers_ologic(ctx: &mut CollectorCtx) {
    let tile = "IO";

    for i in 0..2 {
        let bel = &format!("OLOGIC[{i}]");

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "T2", "T3", "T4", "CLKPERF", "CLKDIV",
        ] {
            ctx.collect_inv_legacy(tile, bel, pin);
        }

        let diff0 = ctx.get_diff_legacy(tile, bel, "T1INV", "T1");
        let diff1 = ctx.get_diff_legacy(tile, bel, "T1INV", "T1_B");
        let (diff0, diff1, _) = Diff::split(diff0, diff1);
        ctx.insert_legacy(tile, bel, "INV.T1", xlat_bit_bi_legacy(diff0, diff1));

        ctx.get_diff_legacy(tile, bel, "CLKINV.SAME", "CLK_B")
            .assert_empty();
        let diff_clk1 = ctx.get_diff_legacy(tile, bel, "CLKINV.OPPOSITE", "CLK");
        let diff_clk2 = ctx.get_diff_legacy(tile, bel, "CLKINV.OPPOSITE", "CLK_B");
        let diff_clk12 = ctx.get_diff_legacy(tile, bel, "CLKINV.SAME", "CLK");
        assert_eq!(diff_clk12, diff_clk1.combine(&diff_clk2));
        ctx.insert_legacy(tile, bel, "INV.CLK1", xlat_bit_legacy(!diff_clk1));
        ctx.insert_legacy(tile, bel, "INV.CLK2", xlat_bit_legacy(!diff_clk2));

        let item_oq = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.get_diff_legacy(tile, bel, "SRTYPE", "ASYNC")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bitvec_diff_legacy(&item_oq, &bits![1; 4], &bits![0; 4]);
        diff.apply_bitvec_diff_legacy(&item_tq, &bits![1; 2], &bits![0; 2]);
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

        ctx.get_diff_legacy(tile, bel, "OREVUSED", "0")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "TREVUSED", "0")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "OCEUSED", "0")
            .assert_empty();
        ctx.get_diff_legacy(tile, bel, "TCEUSED", "0")
            .assert_empty();
        let osrused = ctx.extract_bit_legacy(tile, bel, "OSRUSED", "0");
        let tsrused = ctx.extract_bit_legacy(tile, bel, "TSRUSED", "0");
        ctx.insert_legacy(tile, bel, "OFF_SR_USED", osrused);
        ctx.insert_legacy(tile, bel, "TFF_SR_USED", tsrused);

        let mut diffs = vec![];
        for val in ["2", "3", "4", "5", "6", "7", "8"] {
            diffs.push((
                val,
                val,
                ctx.get_diff_legacy(tile, bel, "DATA_WIDTH.SDR", val),
            ));
        }
        for (val, ratio) in [("4", "2"), ("6", "3"), ("8", "4"), ("10", "5")] {
            diffs.push((
                val,
                ratio,
                ctx.get_diff_legacy(tile, bel, "DATA_WIDTH.DDR", val),
            ));
        }
        for (_, _, diff) in &mut diffs {
            diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "OFF_SR_USED"), true, false);
        }
        let mut ddr3_byp = diffs[0].2.clone();
        for (_, _, diff) in &diffs {
            ddr3_byp.bits.retain(|k, _| diff.bits.contains_key(k));
        }
        let ddr3_byp = xlat_bit_legacy(ddr3_byp);
        for (_, _, diff) in &mut diffs {
            diff.apply_bit_diff_legacy(&ddr3_byp, true, false);
        }
        ctx.insert_legacy(tile, bel, "DDR3_BYPASS", ddr3_byp);
        let mut diff_sdr = diffs[0].2.clone();
        for (width, ratio, diff) in &diffs {
            if width == ratio {
                diff_sdr.bits.retain(|k, _| diff.bits.contains_key(k));
            }
        }
        for (width, ratio, diff) in &mut diffs {
            if width == ratio {
                *diff = diff.combine(&!&diff_sdr);
            }
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

        let diff_buf = !ctx.get_diff_legacy(tile, bel, "DATA_RATE_OQ", "SDR");
        let diff_ddr = ctx
            .get_diff_legacy(tile, bel, "DATA_RATE_OQ", "DDR")
            .combine(&diff_buf);
        let item = xlat_enum_legacy(vec![
            ("NONE", Diff::default()),
            ("D1", diff_buf),
            ("SERDES_SDR", diff_sdr),
            ("SERDES_DDR", diff_ddr),
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
            ("SERDES_DDR", diff_ddr),
            ("FF", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "#FF")),
            ("DDR", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "DDR")),
            ("LATCH", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "#LATCH")),
        ]);
        ctx.insert_legacy(tile, bel, "TMUX", item);

        ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "DEFAULT")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3");

        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "OMUX"), "SERDES_DDR", "NONE");
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DATA_WIDTH"), "4", "NONE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "OFF_SR_USED"), true, false);
        assert_eq!(diff.bits.len(), 1);
        ctx.insert_legacy(
            tile,
            bel,
            "INTERFACE_TYPE",
            xlat_enum_legacy(vec![("DEFAULT", Diff::default()), ("MEMORY_DDR3", diff)]),
        );

        ctx.collect_bit_bi_legacy(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_bit_bi_legacy(tile, bel, "SELFHEAL", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "TRISTATE_WIDTH", &["1", "4"]);
        ctx.collect_bit_bi_legacy(tile, bel, "WC_DELAY", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "DDR3_DATA", "0", "1");
        ctx.collect_bit_bi_legacy(tile, bel, "ODELAY_USED", "0", "1");
        for attr in [
            "INIT_LOADCNT",
            "INIT_ORANK1",
            "INIT_ORANK2_PARTIAL",
            "INIT_TRANK1",
            "INIT_FIFO_ADDR",
            "INIT_FIFO_RESET",
            "INIT_DLY_CNT",
            "INIT_PIPE_DATA0",
            "INIT_PIPE_DATA1",
        ] {
            ctx.collect_bitvec_legacy(tile, bel, attr, "");
        }

        ctx.collect_bit_bi_legacy(tile, bel, "MISR_ENABLE", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "MISR_ENABLE_FDBK", "FALSE", "TRUE");
        ctx.collect_enum_default_legacy(tile, bel, "MISR_CLK_SELECT", &["CLK1", "CLK2"], "NONE");

        let mut present_ologic = ctx.get_diff_legacy(tile, bel, "PRESENT", "OLOGIC");
        present_ologic.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "DDR3_BYPASS"),
            true,
            false,
        );
        present_ologic.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "TFF_SRVAL"), 0, 7);
        present_ologic.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "TMUX"), "T1", "NONE");
        present_ologic.assert_empty();

        let mut present_oserdes = ctx.get_diff_legacy(tile, bel, "PRESENT", "OSERDES");
        present_oserdes.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "OFF_SRVAL"), 0, 7);
        present_oserdes.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "TFF_SRVAL"), 0, 7);
        present_oserdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "OFF_INIT"), false, true);
        present_oserdes.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "TFF_INIT"), false, true);
        present_oserdes.apply_bit_diff_legacy(
            ctx.item_legacy(tile, bel, "INV.CLKPERF"),
            false,
            true,
        );
        present_oserdes.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "OMUX"), "D1", "NONE");
        present_oserdes.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "TMUX"), "T1", "NONE");
        present_oserdes.assert_empty();
    }

    let mut diff = ctx.get_diff_legacy(tile, "OLOGIC_COMMON", "MISR_RESET", "1");
    let diff1 = diff.split_bits_by(|bit| bit.rect.to_idx() > 0);
    ctx.insert_legacy(tile, "OLOGIC[0]", "MISR_RESET", xlat_bit_legacy(diff));
    ctx.insert_legacy(tile, "OLOGIC[1]", "MISR_RESET", xlat_bit_legacy(diff1));
}

fn collect_fuzzers_iodelay(ctx: &mut CollectorCtx) {
    let tile = "IO";

    for i in 0..2 {
        let bel = &format!("IODELAY[{i}]");
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
        ctx.collect_inv_legacy(tile, bel, "C");
        ctx.collect_inv_legacy(tile, bel, "DATAIN");
        ctx.collect_inv_legacy(tile, bel, "IDATAIN");
        ctx.collect_bit_bi_legacy(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "CINVCTRL_SEL", "FALSE", "TRUE");
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
        let item = ctx.extract_bitvec_legacy(tile, bel, "ODELAY_VALUE", "");
        ctx.insert_legacy(tile, bel, "ALT_DELAY_VALUE", item);
        let (_, _, mut diff) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "DELAY_SRC", "I").clone(),
            ctx.peek_diff_legacy(tile, bel, "DELAY_SRC", "O").clone(),
        );
        diff.discard_bits_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"));
        ctx.insert_legacy(tile, bel, "ENABLE", xlat_bit_legacy(diff));
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["I", "IO", "O", "DATAIN", "CLKIN", "DELAYCHAIN_OSC"] {
            let mut diff = ctx.get_diff_legacy(tile, bel, "DELAY_SRC", val);
            diff.apply_bitvec_diff_int_legacy(
                ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"),
                0,
                0x1f,
            );
            diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
            diffs.push((val, diff));
        }
        ctx.insert_legacy(tile, bel, "DELAY_SRC", xlat_enum_legacy(diffs));

        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_DEFAULT");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "I", "NONE");
        let val = extract_bitvec_val_part_legacy(
            ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"),
            &bits![1; 5],
            &mut diff,
        );
        ctx.insert_device_data_legacy("IODELAY:DEFAULT_IDELAY_VALUE", val);
        let val = extract_bitvec_val_part_legacy(
            ctx.item_legacy(tile, bel, "IDELAY_VALUE_INIT"),
            &bits![0; 5],
            &mut diff,
        );
        ctx.insert_device_data_legacy("IODELAY:DEFAULT_IDELAY_VALUE", val);
        ctx.insert_legacy(tile, bel, "EXTRA_DELAY", xlat_bit_legacy(diff));

        let mut diffs = vec![];
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_FIXED");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_VARIABLE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_VAR_LOADABLE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VAR_LOADABLE", diff));

        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "O_FIXED");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "O_VARIABLE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "O_VAR_LOADABLE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VAR_LOADABLE", diff));

        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "IO_FIXED");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_FIXED_O_VARIABLE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE_SWAPPED", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_VARIABLE_O_FIXED");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "IO_VAR_LOADABLE");
        diff.apply_bit_diff_legacy(ctx.item_legacy(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item_legacy(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("IO_VAR_LOADABLE", diff));
        ctx.insert_legacy(tile, bel, "DELAY_TYPE", xlat_enum_legacy(diffs));
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let tile = "IO";
    if devdata_only {
        for i in 0..2 {
            let bel = &format!("IODELAY[{i}]");
            let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_DEFAULT");
            let val = extract_bitvec_val_part_legacy(
                ctx.item_legacy(tile, bel, "IDELAY_VALUE_CUR"),
                &bits![1; 5],
                &mut diff,
            );
            ctx.insert_device_data_legacy("IODELAY:DEFAULT_IDELAY_VALUE", val);
            let val = extract_bitvec_val_part_legacy(
                ctx.item_legacy(tile, bel, "IDELAY_VALUE_INIT"),
                &bits![0; 5],
                &mut diff,
            );
            ctx.insert_device_data_legacy("IODELAY:DEFAULT_IDELAY_VALUE", val);
        }
        return;
    }

    collect_fuzzers_routing(ctx);
    collect_fuzzers_ilogic(ctx);
    collect_fuzzers_ologic(ctx);
    collect_fuzzers_iodelay(ctx);
}
