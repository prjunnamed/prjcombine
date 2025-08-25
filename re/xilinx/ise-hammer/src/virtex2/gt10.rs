use prjcombine_interconnect::db::{BelInfo, PinDir};
use prjcombine_re_fpga_hammer::{OcdMode, extract_bitvec_val};
use prjcombine_re_hammer::Session;
use prjcombine_types::bitvec::BitVec;
use prjcombine_virtex2::bels;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let intdb = backend.edev.db;
    for tile in ["GIGABIT10.B", "GIGABIT10.T"] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let bel_data = &intdb.tile_classes[ctx.tile_class.unwrap()].bels[bels::GT10];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let mut bctx = ctx.bel(bels::GT10);
        let mode = "GT10";
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
            &[
                "XAUI",
                "TEST",
                "PCI_EXPRESS",
                "OC192",
                "OC48",
                "INFINIBAND",
                "CUSTOM",
                "AURORA",
                "10GFC",
                "10GE",
            ],
        );
        for attr in [
            "PCOMMA_DETECT",
            "MCOMMA_DETECT",
            "DEC_PCOMMA_DETECT",
            "DEC_MCOMMA_DETECT",
            "DEC_VALID_COMMA_ONLY",
            "RX_BUFFER_USE",
            "TX_BUFFER_USE",
            "CLK_CORRECT_USE",
            "CLK_COR_KEEP_IDLE",
            "CLK_COR_SEQ_DROP",
            "CLK_COR_SEQ_2_USE",
            "CLK_COR_INSERT_IDLE_FLAG",
            "CLK_COR_8B10B_DE",
            "CHAN_BOND_SEQ_2_USE",
            "CHAN_BOND_ONE_SHOT",
            "CHAN_BOND_64B66B_SV",
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
            .test_enum("CLK_COR_SEQ_LEN", &["1", "2", "3", "4", "8"]);
        bctx.mode(mode)
            .test_multi_attr_dec("CLK_COR_REPEAT_WAIT", 5);
        bctx.mode(mode).test_multi_attr_dec("CLK_COR_ADJ_MAX", 5);
        bctx.mode(mode).test_multi_attr_dec("CLK_COR_MIN_LAT", 6);
        bctx.mode(mode).test_multi_attr_dec("CLK_COR_MAX_LAT", 6);
        bctx.mode(mode)
            .test_enum("CHAN_BOND_MODE", &["MASTER", "SLAVE_1_HOP", "SLAVE_2_HOPS"]);
        bctx.mode(mode)
            .test_enum("CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4", "8"]);
        bctx.mode(mode).test_multi_attr_dec("CHAN_BOND_LIMIT", 5);
        bctx.mode(mode)
            .test_enum("ALIGN_COMMA_WORD", &["1", "2", "4"]);
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
        bctx.mode(mode).test_multi_attr_dec("SH_INVALID_CNT_MAX", 8);
        bctx.mode(mode).test_multi_attr_dec("SH_CNT_MAX", 8);
        bctx.mode(mode)
            .test_enum("PMA_SPEED_USE", &["PMA_SPEED", "PMA_SPEED_HEX"]);
        bctx.mode(mode)
            .attr("PMA_SPEED_USE", "PMA_SPEED")
            .test_enum(
                "PMA_SPEED",
                &[
                    "15_64", "15_32", "14_80", "14_40", "13_80", "13_40", "12_80", "12_40",
                    "11_64", "11_32", "10_64", "10_32", "9_64", "9_32", "8_64", "8_32", "7_64",
                    "7_32", "6_64", "6_32", "5_64", "5_32", "4_64", "4_32", "3_64", "3_32", "2_64",
                    "2_32", "1_64", "1_32", "0_64", "0_32", "28_40", "28_20", "28_10", "27_40",
                    "27_20", "27_10", "26_40", "26_20", "26_10", "25_40", "25_20", "25_10",
                    "24_40", "24_20", "24_10", "23_40", "23_20", "23_10", "22_80", "22_40",
                    "21_80", "21_40", "20_80", "20_40", "19_80", "19_40", "18_80", "18_40",
                    "17_64", "17_32", "16_64", "16_32", "31_8", "31_32", "31_16", "30_8", "30_32",
                    "30_16", "29_40", "29_20", "29_10",
                ],
            );
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
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_1_MASK", 4);
        bctx.mode(mode).test_multi_attr_bin("CLK_COR_SEQ_2_MASK", 4);
        bctx.mode(mode)
            .test_multi_attr_bin("CHAN_BOND_SEQ_1_MASK", 4);
        bctx.mode(mode)
            .test_multi_attr_bin("CHAN_BOND_SEQ_2_MASK", 4);
        bctx.mode(mode).test_multi_attr_bin("PMA_PWR_CNTRL", 8);
        bctx.mode(mode)
            .attr("PMA_SPEED_USE", "PMA_SPEED_HEX")
            .test_multi_attr_hex("PMA_SPEED_HEX", 120);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for tile in ["GIGABIT10.B", "GIGABIT10.T"] {
        let tcid = ctx.edev.db.get_tile_class(tile);
        let bel = "GT10";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        let bel_data = &ctx.edev.db.tile_classes[tcid].bels[bels::GT10];
        let BelInfo::Bel(bel_data) = bel_data else {
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
            let int_tiles = &[
                "INT.GT.CLKPAD",
                "INT.PPC",
                "INT.PPC",
                "INT.PPC",
                "INT.PPC",
                "INT.PPC",
                "INT.PPC",
                "INT.PPC",
                "INT.PPC",
            ];
            let flip = ctx.edev.db.wires.key(wire.wire).starts_with("IMUX.SR");
            ctx.collect_int_inv(int_tiles, tile, bel, pin, flip);
        }
        for attr in [
            "PCOMMA_DETECT",
            "MCOMMA_DETECT",
            "DEC_PCOMMA_DETECT",
            "DEC_MCOMMA_DETECT",
            "DEC_VALID_COMMA_ONLY",
            "RX_BUFFER_USE",
            "TX_BUFFER_USE",
            "CLK_CORRECT_USE",
            "CLK_COR_KEEP_IDLE",
            "CLK_COR_SEQ_DROP",
            "CLK_COR_SEQ_2_USE",
            "CLK_COR_INSERT_IDLE_FLAG",
            "CLK_COR_8B10B_DE",
            "CHAN_BOND_SEQ_2_USE",
            "CHAN_BOND_ONE_SHOT",
            "CHAN_BOND_64B66B_SV",
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
        for val in [
            "XAUI",
            "TEST",
            "PCI_EXPRESS",
            "OC192",
            "OC48",
            "INFINIBAND",
            "CUSTOM",
            "AURORA",
            "10GFC",
            "10GE",
        ] {
            ctx.state
                .get_diff(tile, bel, "IOSTANDARD", val)
                .assert_empty();
        }
        ctx.state
            .get_diff(tile, bel, "PMA_SPEED_USE", "PMA_SPEED")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "PMA_SPEED_USE", "PMA_SPEED_HEX")
            .assert_empty();

        ctx.collect_enum(tile, bel, "CLK_COR_SEQ_LEN", &["1", "2", "3", "4", "8"]);
        ctx.collect_enum(tile, bel, "CHAN_BOND_SEQ_LEN", &["1", "2", "3", "4", "8"]);
        ctx.collect_bitvec(tile, bel, "CLK_COR_REPEAT_WAIT", "");
        ctx.collect_enum_default(
            tile,
            bel,
            "CHAN_BOND_MODE",
            &["MASTER", "SLAVE_1_HOP", "SLAVE_2_HOPS"],
            "NONE",
        );
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_LIMIT", "");
        ctx.collect_enum(tile, bel, "ALIGN_COMMA_WORD", &["1", "2", "4"]);
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
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_1_MASK", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_SEQ_2_MASK", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_1_MASK", "");
        ctx.collect_bitvec(tile, bel, "CHAN_BOND_SEQ_2_MASK", "");
        ctx.collect_bitvec(tile, bel, "PMA_PWR_CNTRL", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_MAX_LAT", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_MIN_LAT", "");
        ctx.collect_bitvec(tile, bel, "CLK_COR_ADJ_MAX", "");
        ctx.collect_bitvec(tile, bel, "SH_CNT_MAX", "");
        ctx.collect_bitvec(tile, bel, "SH_INVALID_CNT_MAX", "");
        let item = ctx.extract_bitvec(tile, bel, "PMA_SPEED_HEX", "");
        let base = BitVec::repeat(false, 120);
        for val in [
            "15_64", "15_32", "14_80", "14_40", "13_80", "13_40", "12_80", "12_40", "11_64",
            "11_32", "10_64", "10_32", "9_64", "9_32", "8_64", "8_32", "7_64", "7_32", "6_64",
            "6_32", "5_64", "5_32", "4_64", "4_32", "3_64", "3_32", "2_64", "2_32", "1_64", "1_32",
            "0_64", "0_32", "28_40", "28_20", "28_10", "27_40", "27_20", "27_10", "26_40", "26_20",
            "26_10", "25_40", "25_20", "25_10", "24_40", "24_20", "24_10", "23_40", "23_20",
            "23_10", "22_80", "22_40", "21_80", "21_40", "20_80", "20_40", "19_80", "19_40",
            "18_80", "18_40", "17_64", "17_32", "16_64", "16_32", "31_8", "31_32", "31_16", "30_8",
            "30_32", "30_16", "29_40", "29_20", "29_10",
        ] {
            let diff = ctx.state.get_diff(tile, bel, "PMA_SPEED", val);
            let bits = extract_bitvec_val(&item, &base, diff);
            ctx.tiledb
                .insert_misc_data(format!("GT10:PMA_SPEED:{val}"), bits);
        }
        ctx.tiledb.insert(tile, bel, "PMA_SPEED", item);
    }
}
