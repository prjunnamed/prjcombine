use prjcombine_interconnect::db::{BelInfo, PinDir};
use prjcombine_re_fpga_hammer::OcdMode;
use prjcombine_re_hammer::Session;
use prjcombine_types::{bits, bitvec::BitVec};
use prjcombine_virtex2::bels;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for tile in ["GIGABIT.B", "GIGABIT.T"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let bel_data = &intdb[ctx.tile_class.unwrap()].bels[bels::GT];
        let BelInfo::Legacy(bel_data) = bel_data else {
            unreachable!()
        };
        let mut bctx = ctx.bel(bels::GT);
        let mode = "GT";
        bctx.test_manual("ENABLE", "1").mode(mode).commit();
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if intdb.wires.key(wire.wire).starts_with("IMUX.G") {
                continue;
            }
            bctx.mode(mode).test_inv(pin);
        }
        bctx.mode(mode).test_enum(
            "IOSTANDARD",
            &["FIBRE_CHAN", "ETHERNET", "XAUI", "INFINIBAND", "AURORA"],
        );
        for attr in [
            "ALIGN_COMMA_MSB",
            "PCOMMA_DETECT",
            "MCOMMA_DETECT",
            "DEC_PCOMMA_DETECT",
            "DEC_MCOMMA_DETECT",
            "DEC_VALID_COMMA_ONLY",
            "SERDES_10B",
            "RX_DECODE_USE",
            "RX_BUFFER_USE",
            "TX_BUFFER_USE",
            "CLK_CORRECT_USE",
            "CLK_COR_KEEP_IDLE",
            "CLK_COR_SEQ_2_USE",
            "CLK_COR_INSERT_IDLE_FLAG",
            "CHAN_BOND_SEQ_2_USE",
            "CHAN_BOND_ONE_SHOT",
            "TEST_MODE_1",
            "TEST_MODE_2",
            "TEST_MODE_3",
            "TEST_MODE_4",
            "TEST_MODE_5",
            "TEST_MODE_6",
            "RX_LOSS_OF_SYNC_FSM",
            "TX_CRC_USE",
            "RX_CRC_USE",
        ] {
            bctx.mode(mode).test_enum(attr, &["FALSE", "TRUE"]);
        }
        bctx.mode(mode)
            .test_enum("TX_PREEMPHASIS", &["0", "1", "2", "3"]);
        bctx.mode(mode).test_enum("TERMINATION_IMP", &["50", "75"]);
        bctx.mode(mode)
            .test_enum("CLK_COR_SEQ_LEN", &["1", "2", "3", "4"]);
        bctx.mode(mode)
            .test_multi_attr_dec("CLK_COR_REPEAT_WAIT", 5);
        bctx.mode(mode)
            .test_enum("CHAN_BOND_MODE", &["MASTER", "SLAVE_1_HOP", "SLAVE_2_HOPS"]);
        bctx.mode(mode).test_enum(
            "CHAN_BOND_WAIT",
            &[
                "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
            ],
        );
        bctx.mode(mode).test_enum(
            "CHAN_BOND_LIMIT",
            &[
                "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
                "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29",
                "30", "31",
            ],
        );
        bctx.mode(mode)
            .test_enum("CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4"]);
        bctx.mode(mode).test_multi_attr_dec("CHAN_BOND_OFFSET", 4);
        bctx.mode(mode).test_enum("RX_DATA_WIDTH", &["1", "2", "4"]);
        bctx.mode(mode).test_enum("TX_DATA_WIDTH", &["1", "2", "4"]);
        bctx.mode(mode)
            .attr("CHAN_BOND_MODE", "")
            .test_multi_attr_dec("RX_BUFFER_LIMIT", 4);
        bctx.mode(mode)
            .attr("CHAN_BOND_MODE", "MASTER")
            .test_manual("RX_BUFFER_LIMIT", "15.MASTER")
            .attr("RX_BUFFER_LIMIT", "15")
            .commit();
        bctx.mode(mode)
            .attr("CHAN_BOND_MODE", "SLAVE_1_HOP")
            .test_manual("RX_BUFFER_LIMIT", "15.SLAVE_1_HOP")
            .attr("RX_BUFFER_LIMIT", "15")
            .commit();
        bctx.mode(mode)
            .attr("CHAN_BOND_MODE", "SLAVE_2_HOPS")
            .test_manual("RX_BUFFER_LIMIT", "15.SLAVE_2_HOPS")
            .attr("RX_BUFFER_LIMIT", "15")
            .commit();
        bctx.mode(mode).test_enum(
            "RX_LOS_INVALID_INCR",
            &["1", "2", "4", "8", "16", "32", "64", "128"],
        );
        bctx.mode(mode).test_enum(
            "RX_LOS_THRESHOLD",
            &["4", "8", "16", "32", "64", "128", "256", "512"],
        );
        bctx.mode(mode).test_enum(
            "CRC_FORMAT",
            &["USER_MODE", "ETHERNET", "INFINIBAND", "FIBRE_CHAN"],
        );
        bctx.mode(mode).test_enum(
            "CRC_START_OF_PKT",
            &[
                "K28_0", "K28_1", "K28_2", "K28_3", "K28_4", "K28_5", "K28_6", "K28_7", "K23_7",
                "K27_7", "K29_7", "K30_7",
            ],
        );
        bctx.mode(mode).test_enum(
            "CRC_END_OF_PKT",
            &[
                "K28_0", "K28_1", "K28_2", "K28_3", "K28_4", "K28_5", "K28_6", "K28_7", "K23_7",
                "K27_7", "K29_7", "K30_7",
            ],
        );
        bctx.mode(mode)
            .test_enum("TX_DIFF_CTRL", &["400", "500", "600", "700", "800"]);
        bctx.mode(mode).test_enum("REF_CLK_V_SEL", &["0", "1"]);
        bctx.mode(mode).test_multi_attr_bin("TX_CRC_FORCE_VALUE", 8);
        bctx.mode(mode).test_multi_attr_bin("COMMA_10B_MASK", 10);
        bctx.mode(mode).test_multi_attr_bin("MCOMMA_10B_VALUE", 10);
        bctx.mode(mode).test_multi_attr_bin("PCOMMA_10B_VALUE", 10);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_1_1", 11);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_1_2", 11);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_1_3", 11);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_1_4", 11);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_2_1", 11);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_2_2", 11);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_2_3", 11);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_2_4", 11);
        bctx.mode(mode).test_multi_attr_bin("CHAN_BOND_SEQ_1_1", 11);
        bctx.mode(mode).test_multi_attr_bin("CHAN_BOND_SEQ_1_2", 11);
        bctx.mode(mode).test_multi_attr_bin("CHAN_BOND_SEQ_1_3", 11);
        bctx.mode(mode).test_multi_attr_bin("CHAN_BOND_SEQ_1_4", 11);
        bctx.mode(mode).test_multi_attr_bin("CHAN_BOND_SEQ_2_1", 11);
        bctx.mode(mode).test_multi_attr_bin("CHAN_BOND_SEQ_2_2", 11);
        bctx.mode(mode).test_multi_attr_bin("CHAN_BOND_SEQ_2_3", 11);
        bctx.mode(mode).test_multi_attr_bin("CHAN_BOND_SEQ_2_4", 11);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["GIGABIT.B", "GIGABIT.T"] {
        let tcid = ctx.edev.db.get_tile_class(tile);
        let bel = "GT";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        let bel_data = &ctx.edev.db[tcid].bels[bels::GT];
        let BelInfo::Legacy(bel_data) = bel_data else {
            unreachable!()
        };
        for (pin, pin_data) in &bel_data.pins {
            if pin_data.dir != PinDir::Input {
                continue;
            }
            assert_eq!(pin_data.wires.len(), 1);
            let wire = *pin_data.wires.first().unwrap();
            if ctx.edev.db.wires.key(wire.wire).starts_with("IMUX.G") {
                continue;
            }
            let int_tiles = &["INT.GT.CLKPAD", "INT.PPC", "INT.PPC", "INT.PPC", "INT.PPC"];
            let flip = ctx.edev.db.wires.key(wire.wire).starts_with("IMUX.SR");
            ctx.collect_int_inv(int_tiles, tile, bel, pin, flip);
        }
        for attr in [
            "ALIGN_COMMA_MSB",
            "PCOMMA_DETECT",
            "MCOMMA_DETECT",
            "DEC_PCOMMA_DETECT",
            "DEC_MCOMMA_DETECT",
            "DEC_VALID_COMMA_ONLY",
            "SERDES_10B",
            "RX_DECODE_USE",
            "RX_BUFFER_USE",
            "TX_BUFFER_USE",
            "CLK_CORRECT_USE",
            "CLK_COR_KEEP_IDLE",
            "CLK_COR_SEQ_2_USE",
            "CLK_COR_INSERT_IDLE_FLAG",
            "CHAN_BOND_SEQ_2_USE",
            "CHAN_BOND_ONE_SHOT",
            "TEST_MODE_1",
            "TEST_MODE_2",
            "TEST_MODE_3",
            "TEST_MODE_4",
            "TEST_MODE_5",
            "TEST_MODE_6",
            "RX_LOSS_OF_SYNC_FSM",
            "TX_CRC_USE",
            "RX_CRC_USE",
        ] {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for val in ["ETHERNET", "AURORA", "FIBRE_CHAN", "INFINIBAND", "XAUI"] {
            ctx.state
                .get_diff(tile, bel, "IOSTANDARD", val)
                .assert_empty();
        }
        ctx.collect_enum_int(tile, bel, "TX_PREEMPHASIS", 0..4, 0);
        ctx.collect_enum(tile, bel, "TERMINATION_IMP", &["50", "75"]);
        ctx.collect_enum(tile, bel, "CLK_COR_SEQ_LEN", &["1", "2", "3", "4"]);
        ctx.collect_enum(tile, bel, "CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4"]);
        ctx.collect_bitvec(tile, bel, "CLK_COR_REPEAT_WAIT", "");
        ctx.collect_enum_default(
            tile,
            bel,
            "CHAN_BOND_MODE",
            &["MASTER", "SLAVE_1_HOP", "SLAVE_2_HOPS"],
            "NONE",
        );
        ctx.collect_enum_int(tile, bel, "CHAN_BOND_WAIT", 1..16, 0);
        ctx.collect_enum_int(tile, bel, "CHAN_BOND_LIMIT", 1..32, 0);
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_OFFSET", "");
        ctx.collect_enum(tile, bel, "RX_DATA_WIDTH", &["1", "2", "4"]);
        ctx.collect_enum(tile, bel, "TX_DATA_WIDTH", &["1", "2", "4"]);
        ctx.collect_bitvec(tile, bel, "RX_BUFFER_LIMIT", "");
        let item = ctx.collector.tiledb.item(tile, bel, "RX_BUFFER_LIMIT");
        for (name, val) in [
            ("15.MASTER", bits![0, 0, 1, 1]),
            ("15.SLAVE_1_HOP", bits![0, 0, 1, 0]),
            ("15.SLAVE_2_HOPS", bits![0, 0, 1, 0]),
        ] {
            let mut diff = ctx
                .collector
                .state
                .get_diff(tile, bel, "RX_BUFFER_LIMIT", name);
            diff.apply_bitvec_diff(item, &val, &BitVec::repeat(false, 4));
            diff.assert_empty();
        }
        ctx.collect_enum(
            tile,
            bel,
            "RX_LOS_INVALID_INCR",
            &["1", "2", "4", "8", "16", "32", "64", "128"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "RX_LOS_THRESHOLD",
            &["4", "8", "16", "32", "64", "128", "256", "512"],
        );
        ctx.collect_enum(
            tile,
            bel,
            "CRC_FORMAT",
            &["USER_MODE", "ETHERNET", "INFINIBAND", "FIBRE_CHAN"],
        );
        ctx.collect_enum_ocd(
            tile,
            bel,
            "CRC_START_OF_PKT",
            &[
                "K28_0", "K28_1", "K28_2", "K28_3", "K28_4", "K28_5", "K28_6", "K28_7", "K23_7",
                "K27_7", "K29_7", "K30_7",
            ],
            OcdMode::BitOrder,
        );
        ctx.collect_enum_ocd(
            tile,
            bel,
            "CRC_END_OF_PKT",
            &[
                "K28_0", "K28_1", "K28_2", "K28_3", "K28_4", "K28_5", "K28_6", "K28_7", "K23_7",
                "K27_7", "K29_7", "K30_7",
            ],
            OcdMode::BitOrder,
        );
        ctx.collect_enum(
            tile,
            bel,
            "TX_DIFF_CTRL",
            &["400", "500", "600", "700", "800"],
        );
        ctx.collect_enum_bool(tile, bel, "REF_CLK_V_SEL", "0", "1");
        ctx.collect_bitvec(tile, bel, "TX_CRC_FORCE_VALUE", "");
        ctx.collect_bitvec(tile, bel, "COMMA_10B_MASK", "");
        ctx.collect_bitvec(tile, bel, "MCOMMA_10B_VALUE", "");
        ctx.collect_bitvec(tile, bel, "PCOMMA_10B_VALUE", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_1_1", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_1_2", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_1_3", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_1_4", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_2_1", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_2_2", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_2_3", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_2_4", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_1_1", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_1_2", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_1_3", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_1_4", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_2_1", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_2_2", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_2_3", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_2_4", "");
    }
}
