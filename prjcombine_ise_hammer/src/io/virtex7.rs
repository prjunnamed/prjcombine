use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{
        xlat_bit, xlat_bitvec, xlat_bool, xlat_enum, xlat_enum_ocd, CollectorCtx, Diff, OcdMode,
    },
    fgen::{ExtraFeature, ExtraFeatureKind, TileBits, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_inv, fuzz_multi_attr_dec, fuzz_one, fuzz_one_extras,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let bel_idelayctrl = BelId::from_idx(8);
    for (tile, bel, bel_ioi, bel_ologic) in [
        (
            "IO_HR_PAIR",
            "ILOGIC0",
            BelId::from_idx(8),
            BelId::from_idx(2),
        ),
        (
            "IO_HR_PAIR",
            "ILOGIC1",
            BelId::from_idx(8),
            BelId::from_idx(3),
        ),
        (
            "IO_HR_BOT",
            "ILOGIC",
            BelId::from_idx(4),
            BelId::from_idx(1),
        ),
        (
            "IO_HR_TOP",
            "ILOGIC",
            BelId::from_idx(4),
            BelId::from_idx(1),
        ),
        (
            "IO_HP_PAIR",
            "ILOGIC0",
            BelId::from_idx(10),
            BelId::from_idx(2),
        ),
        (
            "IO_HP_PAIR",
            "ILOGIC1",
            BelId::from_idx(10),
            BelId::from_idx(3),
        ),
        (
            "IO_HP_BOT",
            "ILOGIC",
            BelId::from_idx(5),
            BelId::from_idx(1),
        ),
        (
            "IO_HP_TOP",
            "ILOGIC",
            BelId::from_idx(5),
            BelId::from_idx(1),
        ),
    ] {
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, bel, TileBits::MainAuto) else {
            continue;
        };

        fuzz_one!(ctx, "PRESENT", "ILOGICE2", [], [(mode "ILOGICE2")]);
        fuzz_one!(ctx, "PRESENT", "ISERDESE2", [], [(mode "ISERDESE2")]);

        fuzz_inv!(ctx, "D", [(mode "ISERDESE2")]);
        fuzz_inv!(ctx, "CLK", [(mode "ISERDESE2")]);
        fuzz_inv!(ctx, "OCLK", [(mode "ISERDESE2"), (attr "DATA_RATE", "SDR")]);
        fuzz_inv!(ctx, "CLKDIV", [(mode "ISERDESE2"), (attr "DYN_CLKDIV_INV_EN", "FALSE")]);
        fuzz_inv!(ctx, "CLKDIVP", [(mode "ISERDESE2"), (attr "DYN_CLKDIVP_INV_EN", "FALSE")]);
        fuzz_enum!(ctx, "DYN_CLK_INV_EN", ["FALSE", "TRUE"], [(mode "ISERDESE2")]);
        fuzz_enum!(ctx, "DYN_CLKDIV_INV_EN", ["FALSE", "TRUE"], [(mode "ISERDESE2")]);
        fuzz_enum!(ctx, "DYN_CLKDIVP_INV_EN", ["FALSE", "TRUE"], [(mode "ISERDESE2")]);

        fuzz_enum!(ctx, "SRUSED", ["0"], [
            (mode "ILOGICE2"),
            (attr "IFFTYPE", "#FF"),
            (pin "SR")
        ]);
        fuzz_enum!(ctx, "SERDES", ["FALSE", "TRUE"], [
            (mode "ISERDESE2"),
            (attr "DATA_WIDTH", "2"),
            (attr "DATA_RATE", "SDR")
        ]);
        fuzz_enum!(ctx, "SERDES_MODE", ["MASTER", "SLAVE"], [(mode "ISERDESE2")]);
        fuzz_enum!(ctx, "DATA_WIDTH", ["2", "3", "4", "5", "6", "7", "8", "10", "14"], [
            (mode "ISERDESE2"),
            (attr "SERDES", "FALSE")
        ]);
        fuzz_enum!(ctx, "NUM_CE", ["1", "2"], [
            (mode "ISERDESE2")
        ]);

        for attr in [
            "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
            "SRVAL_Q4",
        ] {
            fuzz_enum!(ctx, attr, ["0", "1"], [
                (mode "ISERDESE2")
            ]);
        }

        fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
            (mode "ILOGICE2"),
            (attr "IFFTYPE", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
            (mode "ISERDESE2")
        ]);

        fuzz_enum!(ctx, "D_EMU1", ["FALSE", "TRUE"], [(mode "ISERDESE2")]);
        fuzz_enum!(ctx, "D_EMU2", ["FALSE", "TRUE"], [(mode "ISERDESE2")]);
        fuzz_enum!(ctx, "RANK23_DLY", ["FALSE", "TRUE"], [(mode "ISERDESE2")]);

        fuzz_enum!(ctx, "INTERFACE_TYPE", ["NETWORKING", "MEMORY", "MEMORY_DDR3", "MEMORY_QDR", "OVERSAMPLE"], [
            (mode "ISERDESE2")
        ]);
        fuzz_one!(ctx, "INTERFACE_TYPE", "MEMORY_DDR3_V6", [
            (mode "ISERDESE2")
        ], [
            (attr "INTERFACE_TYPE", "MEMORY_DDR3"),
            (attr "DDR3_V6", "TRUE")
        ]);
        fuzz_enum!(ctx, "DATA_RATE", ["SDR", "DDR"], [
            (mode "ISERDESE2")
        ]);
        fuzz_enum!(ctx, "DDR_CLK_EDGE", ["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"], [
            (mode "ISERDESE2")
        ]);
        fuzz_enum!(ctx, "DDR_CLK_EDGE", ["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"], [
            (mode "ILOGICE2"),
            (attr "IFFTYPE", "DDR")
        ]);
        fuzz_enum!(ctx, "IFFTYPE", ["#FF", "#LATCH", "DDR"], [
            (mode "ILOGICE2")
        ]);

        fuzz_enum!(ctx, "OFB_USED", ["FALSE", "TRUE"], [
            (mode "ISERDESE2"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "TFB_USED", ["FALSE", "TRUE"], [
            (mode "ISERDESE2"),
            (pin "TFB")
        ]);
        fuzz_enum!(ctx, "IOBDELAY", ["NONE", "IFD", "IBUF", "BOTH"], [
            (mode "ISERDESE2")
        ]);

        fuzz_enum!(ctx, "D2OBYP_SEL", ["GND", "T"], [
            (mode "ILOGICE2"),
            (attr "IMUX", "0"),
            (attr "IDELMUX", "1"),
            (attr "IFFMUX", "#OFF"),
            (attr "DINV", ""),
            (pin "D"),
            (pin "DDLY"),
            (pin "TFB"),
            (pin "OFB"),
            (pin "O")
        ]);
        fuzz_enum!(ctx, "D2OFFBYP_SEL", ["GND", "T"], [
            (mode "ILOGICE2"),
            (attr "IFFMUX", "0"),
            (attr "IFFTYPE", "#FF"),
            (attr "IFFDELMUX", "1"),
            (attr "IMUX", "#OFF"),
            (attr "DINV", ""),
            (pin "D"),
            (pin "DDLY"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IMUX", ["0", "1"], [
            (mode "ILOGICE2"),
            (attr "IDELMUX", "1"),
            (attr "DINV", ""),
            (pin "D"),
            (pin "DDLY"),
            (pin "O"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IFFMUX", ["0", "1"], [
            (mode "ILOGICE2"),
            (attr "IFFDELMUX", "1"),
            (attr "IFFTYPE", "#FF"),
            (attr "DINV", ""),
            (pin "D"),
            (pin "DDLY"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IDELMUX", ["0", "1"], [
            (mode "ILOGICE2"),
            (attr "IMUX", "1"),
            (attr "IFFMUX", "1"),
            (attr "IFFTYPE", "#FF"),
            (attr "IFFDELMUX", "0"),
            (attr "DINV", ""),
            (pin "D"),
            (pin "DDLY"),
            (pin "O"),
            (pin "Q1"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IFFDELMUX", ["0", "1"], [
            (mode "ILOGICE2"),
            (attr "IMUX", "1"),
            (attr "IFFMUX", "0"),
            (attr "IFFTYPE", "#FF"),
            (attr "IDELMUX", "0"),
            (attr "D2OFFBYP_SEL", "T"),
            (attr "DINV", ""),
            (pin "D"),
            (pin "DDLY"),
            (pin "O"),
            (pin "Q1"),
            (pin "TFB"),
            (pin "OFB")
        ]);

        if tile.contains("HR") {
            fuzz_one!(ctx, "PRESENT", "ILOGICE3", [], [(mode "ILOGICE3")]);
            for val in ["D", "D_B"] {
                fuzz_one!(ctx, "ZHOLD_IFF_INV", val, [
                    (mode "ILOGICE3"),
                    (attr "ZHOLD_IFF", "TRUE"),
                    (attr "IFFTYPE", "#FF"),
                    (pin "Q1")
                ], [
                    (attr "IFFDELMUXE3", "2"),
                    (attr "IFFMUX", "1"),
                    (attr "ZHOLD_IFF_INV", val)
                ]);
            }
            fuzz_enum!(ctx, "ZHOLD_FABRIC_INV", ["D", "D_B"], [
                (mode "ILOGICE3"),
                (attr "ZHOLD_FABRIC", "TRUE"),
                (attr "IDELMUXE3", "2"),
                (attr "IMUX", "1"),
                (pin "O")
            ]);
            fuzz_enum!(ctx, "ZHOLD_FABRIC", ["FALSE", "TRUE"], [
                (mode "ILOGICE3"),
                (attr "ZHOLD_IFF", "")
            ]);
            fuzz_enum!(ctx, "ZHOLD_IFF", ["FALSE", "TRUE"], [
                (mode "ILOGICE3"),
                (attr "ZHOLD_FABRIC", "")
            ]);
            fuzz_multi_attr_dec!(ctx, "IDELAY_VALUE", 5, [(mode "ILOGICE3")]);
            fuzz_multi_attr_dec!(ctx, "IFFDELAY_VALUE", 5, [(mode "ILOGICE3")]);
        }

        for pin in ["CKINT0", "CKINT1", "PHASER_ICLK"] {
            fuzz_one!(ctx, "MUX.CLK", pin, [
                (mutex "MUX.CLK", pin),
                (pip (pin pin), (pin "CLKB"))
            ], [
                (pip (pin pin), (pin "CLK"))
            ]);
            fuzz_one!(ctx, "MUX.CLKB", pin, [
                (mutex "MUX.CLK", pin)
            ], [
                (pip (pin pin), (pin "CLKB"))
            ]);
        }
        fuzz_one!(ctx, "MUX.CLK", "PHASER_OCLK", [
            (mutex "MUX.CLK", "PHASER_OCLK"),
            (pip (bel_pin bel_ologic, "PHASER_OCLK"), (pin "CLKB"))
        ], [
            (pip (bel_pin bel_ologic, "PHASER_OCLK"), (pin "CLK"))
        ]);
        fuzz_one!(ctx, "MUX.CLKB", "PHASER_OCLK", [
            (mutex "MUX.CLK", "PHASER_OCLK")
        ], [
            (pip (bel_pin bel_ologic, "PHASER_OCLK"), (pin "CLKB"))
        ]);
        for (src, num) in [("HCLK", 6), ("RCLK", 4), ("IOCLK", 4)] {
            for i in 0..num {
                fuzz_one!(ctx, "MUX.CLK", format!("{src}{i}"), [
                    (mutex "MUX.CLK", format!("{src}{i}")),
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLKB"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLK"))
                ]);
                fuzz_one!(ctx, "MUX.CLKB", format!("{src}{i}"), [
                    (mutex "MUX.CLK", format!("{src}{i}"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLKB"))
                ]);
            }
        }

        fuzz_one!(ctx, "MUX.CLKDIVP", "CLKDIV", [
            (mutex "MUX.CLKDIVP", "CLKDIV")
        ], [
            (pin_pips "CLKDIVP")
        ]);
        fuzz_one!(ctx, "MUX.CLKDIVP", "PHASER", [
            (mutex "MUX.CLKDIVP", "PHASER")
        ], [
            (pip (pin "PHASER_ICLKDIV"), (pin "CLKDIVP"))
        ]);
    }
    for (tile, bel, bel_ioi) in [
        ("IO_HR_PAIR", "OLOGIC0", BelId::from_idx(8)),
        ("IO_HR_PAIR", "OLOGIC1", BelId::from_idx(8)),
        ("IO_HR_BOT", "OLOGIC", BelId::from_idx(4)),
        ("IO_HR_TOP", "OLOGIC", BelId::from_idx(4)),
        ("IO_HP_PAIR", "OLOGIC0", BelId::from_idx(10)),
        ("IO_HP_PAIR", "OLOGIC1", BelId::from_idx(10)),
        ("IO_HP_BOT", "OLOGIC", BelId::from_idx(5)),
        ("IO_HP_TOP", "OLOGIC", BelId::from_idx(5)),
    ] {
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, bel, TileBits::MainAuto) else {
            continue;
        };

        fuzz_one!(ctx, "PRESENT", "OLOGICE2", [], [(mode "OLOGICE2")]);
        fuzz_one!(ctx, "PRESENT", "OSERDESE2", [], [(mode "OSERDESE2")]);

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "T1", "T2", "T3", "T4", "CLKDIV",
            "CLKDIVF",
        ] {
            fuzz_inv!(ctx, pin, [(mode "OSERDESE2")]);
        }
        fuzz_enum_suffix!(ctx, "CLKINV", "SAME", ["CLK", "CLK_B"], [
            (mode "OSERDESE2"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "DDR_CLK_EDGE", "SAME_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);
        fuzz_enum_suffix!(ctx, "CLKINV", "OPPOSITE", ["CLK", "CLK_B"], [
            (mode "OSERDESE2"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "DDR_CLK_EDGE", "OPPOSITE_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);

        fuzz_enum!(ctx, "SRTYPE_OQ", ["SYNC", "ASYNC"], [
            (mode "OLOGICE2"),
            (attr "OUTFFTYPE", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE_TQ", ["SYNC", "ASYNC"], [
            (mode "OLOGICE2"),
            (attr "TFFTYPE", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
            (mode "OSERDESE2")
        ]);

        fuzz_enum_suffix!(ctx, "INIT_OQ", "OLOGIC", ["0", "1"], [(mode "OLOGICE2")]);
        fuzz_enum_suffix!(ctx, "INIT_TQ", "OLOGIC", ["0", "1"], [(mode "OLOGICE2")]);
        fuzz_enum_suffix!(ctx, "INIT_OQ", "OSERDES", ["0", "1"], [(mode "OSERDESE2")]);
        fuzz_enum_suffix!(ctx, "INIT_TQ", "OSERDES", ["0", "1"], [(mode "OSERDESE2")]);
        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OLOGIC", ["0", "1"], [(mode "OLOGICE2")]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "OLOGIC", ["0", "1"], [(mode "OLOGICE2")]);
        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OSERDES", ["0", "1"], [(mode "OSERDESE2")]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "OSERDES", ["0", "1"], [(mode "OSERDESE2")]);

        for attr in ["OSRUSED", "TSRUSED"] {
            fuzz_enum!(ctx, attr, ["0"], [
                (mode "OLOGICE2"),
                (attr "OUTFFTYPE", "#FF"),
                (attr "TFFTYPE", "#FF"),
                (pin "OCE"),
                (pin "TCE"),
                (pin "REV"),
                (pin "SR")
            ]);
        }

        fuzz_enum!(ctx, "OUTFFTYPE", ["#FF", "#LATCH", "DDR"], [
            (mode "OLOGICE2"),
            (pin "OQ")
        ]);
        fuzz_enum!(ctx, "TFFTYPE", ["#FF", "#LATCH", "DDR"], [
            (mode "OLOGICE2"),
            (pin "TQ")
        ]);
        fuzz_one!(ctx, "OMUX", "D1", [
            (mode "OLOGICE2")
        ], [
            (attr "OQUSED", "0"),
            (attr "O1USED", "0"),
            (attr "D1INV", "D1"),
            (attr "OMUX", "D1"),
            (pin "OQ"),
            (pin "D1")
        ]);

        fuzz_enum!(ctx, "DATA_RATE_OQ", ["SDR", "DDR"], [
            (mode "OSERDESE2")
        ]);
        fuzz_enum!(ctx, "DATA_RATE_TQ", ["BUF", "SDR", "DDR"], [
            (mode "OSERDESE2")
        ]);

        fuzz_enum!(ctx, "MISR_ENABLE", ["FALSE", "TRUE"], [
            (mode "OLOGICE2"),
            (global_opt "ENABLEMISR", "Y")
        ]);
        fuzz_enum!(ctx, "MISR_ENABLE_FDBK", ["FALSE", "TRUE"], [
            (mode "OLOGICE2"),
            (global_opt "ENABLEMISR", "Y")
        ]);
        fuzz_enum!(ctx, "MISR_CLK_SELECT", ["CLK1", "CLK2"], [
            (mode "OLOGICE2"),
            (global_opt "ENABLEMISR", "Y")
        ]);

        fuzz_enum!(ctx, "SERDES", ["FALSE", "TRUE"], [
            (mode "OSERDESE2")
        ]);
        fuzz_enum!(ctx, "SERDES_MODE", ["SLAVE", "MASTER"], [
            (mode "OSERDESE2")
        ]);
        fuzz_enum!(ctx, "SELFHEAL", ["FALSE", "TRUE"], [
            (mode "OSERDESE2")
        ]);
        fuzz_enum!(ctx, "RANK3_USED", ["FALSE", "TRUE"], [
            (mode "OSERDESE2")
        ]);
        fuzz_enum!(ctx, "TBYTE_CTL", ["FALSE", "TRUE"], [
            (mode "OSERDESE2")
        ]);
        fuzz_enum!(ctx, "TBYTE_SRC", ["FALSE", "TRUE"], [
            (mode "OSERDESE2")
        ]);
        fuzz_enum!(ctx, "TRISTATE_WIDTH", ["1", "4"], [
            (mode "OSERDESE2")
        ]);
        fuzz_enum_suffix!(ctx, "DATA_WIDTH", "SDR", ["2", "3", "4", "5", "6", "7", "8"], [
            (mode "OSERDESE2"),
            (attr "DATA_RATE_OQ", "SDR")
        ]);
        fuzz_enum_suffix!(ctx, "DATA_WIDTH", "DDR", ["4", "6", "8", "10", "14"], [
            (mode "OSERDESE2"),
            (attr "DATA_RATE_OQ", "DDR")
        ]);

        fuzz_one!(ctx, "MUX.CLK", "CKINT", [
            (mutex "MUX.CLK", "CKINT"),
            (pip (pin "CLK_CKINT"), (pin "CLKM"))
        ], [
            (pip (pin "CLK_CKINT"), (pin "CLK_MUX"))
        ]);
        fuzz_one!(ctx, "MUX.CLKB", "CKINT", [
            (mutex "MUX.CLK", "CKINT")
        ], [
            (pip (pin "CLK_CKINT"), (pin "CLKM"))
        ]);
        fuzz_one!(ctx, "MUX.CLK", "PHASER_OCLK", [
            (mutex "MUX.CLK", "PHASER_OCLK"),
            (pip (pin "PHASER_OCLK"), (pin "CLKM"))
        ], [
            (pip (pin "PHASER_OCLK"), (pin "CLK_MUX"))
        ]);
        fuzz_one!(ctx, "MUX.CLKB", "PHASER_OCLK", [
            (mutex "MUX.CLK", "PHASER_OCLK")
        ], [
            (pip (pin "PHASER_OCLK"), (pin "CLKM"))
        ]);
        fuzz_one!(ctx, "MUX.CLK", "PHASER_OCLK90", [
            (mutex "MUX.CLK", "PHASER_OCLK90"),
            (pip (pin "PHASER_OCLK"), (pin "CLKM"))
        ], [
            (pip (pin "PHASER_OCLK90"), (pin "CLK_MUX"))
        ]);
        fuzz_one!(ctx, "MUX.CLK", "PHASER_OCLK90.BOTH", [
            (mutex "MUX.CLK", "PHASER_OCLK90.BOTH")
        ], [
            (pip (pin "PHASER_OCLK90"), (pin "CLK_MUX"))
        ]);
        for (src, num) in [("HCLK", 6), ("RCLK", 4), ("IOCLK", 4)] {
            for i in 0..num {
                fuzz_one!(ctx, "MUX.CLK", format!("{src}{i}"), [
                    (mutex "MUX.CLK", format!("{src}{i}")),
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLKM"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLK_MUX"))
                ]);
                fuzz_one!(ctx, "MUX.CLKB", format!("{src}{i}"), [
                    (mutex "MUX.CLK", format!("{src}{i}"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLKM"))
                ]);
            }
        }

        fuzz_one!(ctx, "MUX.CLKDIV", "PHASER_OCLKDIV", [
            (mutex "MUX.CLKDIV", "PHASER_OCLKDIV"),
            (pip (pin "PHASER_OCLKDIV"), (pin "CLKDIVB"))
        ], [
            (pip (pin "PHASER_OCLKDIV"), (pin "CLKDIV"))
        ]);
        fuzz_one!(ctx, "MUX.CLKDIVB", "PHASER_OCLKDIV", [
            (mutex "MUX.CLKDIV", "PHASER_OCLKDIV")
        ], [
            (pip (pin "PHASER_OCLKDIV"), (pin "CLKDIVB"))
        ]);
        fuzz_one!(ctx, "MUX.CLKDIV", "CKINT", [
            (mutex "MUX.CLKDIV", "CKINT"),
            (pip (pin "CLKDIV_CKINT"), (pin "CLKDIVB"))
        ], [
            (pip (pin "CLKDIV_CKINT"), (pin "CLKDIV"))
        ]);
        fuzz_one!(ctx, "MUX.CLKDIVB", "CKINT", [
            (mutex "MUX.CLKDIV", "CKINT")
        ], [
            (pip (pin "CLKDIV_CKINT"), (pin "CLKDIVB"))
        ]);
        for (src, num) in [("HCLK", 6), ("RCLK", 4)] {
            for i in 0..num {
                fuzz_one!(ctx, "MUX.CLKDIV", format!("{src}{i}"), [
                    (mutex "MUX.CLKDIV", format!("{src}{i}")),
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLKDIVB"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLKDIV"))
                ]);
                fuzz_one!(ctx, "MUX.CLKDIVB", format!("{src}{i}"), [
                    (mutex "MUX.CLKDIV", format!("{src}{i}"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{i}")), (pin "CLKDIVB"))
                ]);
            }
        }
        fuzz_one!(ctx, "MUX.CLKDIV", "HCLK0.F", [
            (mutex "MUX.CLKDIV", "HCLK0.F"),
            (pip (bel_pin bel_ioi, "HCLK0"), (pin "CLKDIVFB"))
        ], [
            (pip (bel_pin bel_ioi, "HCLK0"), (pin "CLKDIVF"))
        ]);
        fuzz_one!(ctx, "MUX.CLKDIVB", "HCLK0.F", [
            (mutex "MUX.CLKDIV", "HCLK0.F")
        ], [
            (pip (bel_pin bel_ioi, "HCLK0"), (pin "CLKDIVFB"))
        ]);
    }
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    let mut extras = vec![];
    for (tile, bel) in [
        ("IO_HP_PAIR", "OLOGIC_COMMON"),
        ("IO_HR_PAIR", "OLOGIC_COMMON"),
        ("IO_HP_BOT", "OLOGIC"),
        ("IO_HP_TOP", "OLOGIC"),
        ("IO_HR_BOT", "OLOGIC"),
        ("IO_HR_TOP", "OLOGIC"),
    ] {
        let node = backend.egrid.db.get_node(tile);
        if backend.egrid.node_index[node].is_empty() {
            continue;
        }
        extras.push(ExtraFeature::new(
            ExtraFeatureKind::AllIobs,
            tile,
            bel,
            "MISR_RESET",
            "1",
        ));
    }
    fuzz_one_extras!(ctx, "MISR_RESET", "1", [
        (global_opt "ENABLEMISR", "Y")
    ], [
        (global_opt_diff "MISRRESET", "N", "Y")
    ], extras);
    for (tile, bel, bel_ologic) in [
        ("IO_HR_PAIR", "IDELAY0", BelId::from_idx(2)),
        ("IO_HR_PAIR", "IDELAY1", BelId::from_idx(3)),
        ("IO_HR_BOT", "IDELAY", BelId::from_idx(1)),
        ("IO_HR_TOP", "IDELAY", BelId::from_idx(1)),
        ("IO_HP_PAIR", "IDELAY0", BelId::from_idx(2)),
        ("IO_HP_PAIR", "IDELAY1", BelId::from_idx(3)),
        ("IO_HP_BOT", "IDELAY", BelId::from_idx(1)),
        ("IO_HP_TOP", "IDELAY", BelId::from_idx(1)),
    ] {
        let hclk_ioi = backend.egrid.db.get_node(if tile.contains("HP") {
            "HCLK_IOI_HP"
        } else {
            "HCLK_IOI_HR"
        });
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, bel, TileBits::MainAuto) else {
            continue;
        };
        fuzz_one!(ctx, "ENABLE", "1", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0"))
        ], [(mode "IDELAYE2")]);
        for pin in ["C", "IDATAIN", "DATAIN"] {
            fuzz_inv!(ctx, pin, [
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "IDELAYCTRL", "USE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_mode bel_idelayctrl, "IDELAYCTRL")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
                (mode "IDELAYE2"),
                (attr "CINVCTRL_SEL", "FALSE")
            ]);
        }
        for attr in [
            "HIGH_PERFORMANCE_MODE",
            "CINVCTRL_SEL",
            "DELAYCHAIN_OSC",
            "PIPE_SEL",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "IDELAYCTRL", "USE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_mode bel_idelayctrl, "IDELAYCTRL")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
                (mode "IDELAYE2")
            ]);
        }
        fuzz_enum!(ctx, "IDELAY_TYPE", ["FIXED", "VARIABLE", "VAR_LOAD", "VAR_LOAD_PIPE"], [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
            (mode "IDELAYE2")
        ]);
        fuzz_enum!(ctx, "DELAY_SRC", ["DATAIN", "IDATAIN"], [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
            (mode "IDELAYE2")
        ]);
        fuzz_one!(ctx, "DELAY_SRC", "OFB", [
            (attr "DELAY_SRC", "")
        ], [
            (pip (bel_pin bel_ologic, "OFB"), (pin "IDATAIN"))
        ]);
        fuzz_multi_attr_dec!(ctx, "IDELAY_VALUE", 5, [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
            (mode "IDELAYE2"),
            (attr "DELAY_SRC", "IDATAIN"),
            (attr "IDELAY_TYPE", "FIXED")
        ]);
        if tile.contains("HP") {
            fuzz_enum!(ctx, "FINEDELAY", ["BYPASS", "ADD_DLY"], [
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "IDELAYCTRL", "USE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_mode bel_idelayctrl, "IDELAYCTRL")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
                (mode "IDELAYE2_FINEDELAY")
            ]);
        }
    }
    for (tile, bel) in [
        ("IO_HP_PAIR", "ODELAY0"),
        ("IO_HP_PAIR", "ODELAY1"),
        ("IO_HP_BOT", "ODELAY"),
        ("IO_HP_TOP", "ODELAY"),
    ] {
        let hclk_ioi = backend.egrid.db.get_node("HCLK_IOI_HP");
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, bel, TileBits::MainAuto) else {
            continue;
        };
        fuzz_one!(ctx, "PRESENT", "1", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0"))
        ], [(mode "ODELAYE2")]);
        for pin in ["C", "ODATAIN"] {
            fuzz_inv!(ctx, pin, [
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "IDELAYCTRL", "USE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_mode bel_idelayctrl, "IDELAYCTRL")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
                (mode "ODELAYE2"),
                (attr "CINVCTRL_SEL", "FALSE")
            ]);
        }
        for attr in [
            "HIGH_PERFORMANCE_MODE",
            "CINVCTRL_SEL",
            "DELAYCHAIN_OSC",
            "PIPE_SEL",
        ] {
            fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "IDELAYCTRL", "USE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_mode bel_idelayctrl, "IDELAYCTRL")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
                (mode "ODELAYE2"),
                (attr "DELAY_SRC", "")
            ]);
        }
        fuzz_enum!(ctx, "ODELAY_TYPE", ["FIXED", "VARIABLE", "VAR_LOAD"], [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
            (mode "ODELAYE2"),
            (attr "DELAY_SRC", "ODATAIN"),
            (attr "PIPE_SEL", "FALSE")
        ]);
        fuzz_enum!(ctx, "DELAY_SRC", ["ODATAIN", "CLKIN"], [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
            (mode "ODELAYE2"),
            (attr "DELAYCHAIN_OSC", "")
        ]);
        fuzz_multi_attr_dec!(ctx, "ODELAY_VALUE", 5, [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
            (mode "ODELAYE2"),
            (attr "DELAY_SRC", "ODATAIN"),
            (attr "ODELAY_TYPE", "FIXED")
        ]);
        fuzz_enum!(ctx, "FINEDELAY", ["BYPASS", "ADD_DLY"], [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "IDELAYCTRL_EN", "ENABLE")),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_attr bel_idelayctrl, "BIAS_MODE", "0")),
            (mode "ODELAYE2_FINEDELAY")
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    for (tile, bel) in [
        ("IO_HR_PAIR", "ILOGIC0"),
        ("IO_HR_PAIR", "ILOGIC1"),
        ("IO_HR_BOT", "ILOGIC"),
        ("IO_HR_TOP", "ILOGIC"),
        ("IO_HP_PAIR", "ILOGIC0"),
        ("IO_HP_PAIR", "ILOGIC1"),
        ("IO_HP_BOT", "ILOGIC"),
        ("IO_HP_TOP", "ILOGIC"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }

        ctx.collect_inv(tile, bel, "D");
        ctx.collect_inv(tile, bel, "CLKDIV");
        ctx.collect_inv(tile, bel, "CLKDIVP");
        let item = ctx.extract_enum_bool_wide(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.tiledb.insert(tile, bel, "INV.CLK", item);
        let item = ctx.extract_bit(tile, bel, "OCLKINV", "OCLK");
        ctx.tiledb.insert(tile, bel, "INV.OCLK1", item);
        let item = ctx.extract_bit(tile, bel, "OCLKINV", "OCLK_B");
        ctx.tiledb.insert(tile, bel, "INV.OCLK2", item);
        ctx.collect_enum_bool(tile, bel, "DYN_CLK_INV_EN", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "DYN_CLKDIV_INV_EN", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "DYN_CLKDIVP_INV_EN", "FALSE", "TRUE");

        let iff_sr_used = ctx.extract_bit(tile, bel, "SRUSED", "0");
        ctx.tiledb.insert(tile, bel, "IFF_SR_USED", iff_sr_used);
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["MASTER", "SLAVE"]);
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10", "14"] {
            diffs.push((val, ctx.state.get_diff(tile, bel, "DATA_WIDTH", val)));
        }
        let mut bits = xlat_enum(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.tiledb.insert(
            tile,
            bel,
            "DATA_WIDTH",
            xlat_enum_ocd(diffs, OcdMode::FixedOrder(&bits)),
        );
        ctx.collect_enum(tile, bel, "NUM_CE", &["1", "2"]);
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
            let item = ctx.extract_enum_bool(tile, bel, sattr, "0", "1");
            ctx.tiledb.insert(tile, bel, attr, item);
        }
        ctx.collect_enum(tile, bel, "SRTYPE", &["ASYNC", "SYNC"]);
        ctx.collect_enum(tile, bel, "DATA_RATE", &["SDR", "DDR"]);
        ctx.collect_enum_bool(tile, bel, "D_EMU1", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "D_EMU2", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "RANK23_DLY", "FALSE", "TRUE");
        ctx.collect_enum(
            tile,
            bel,
            "DDR_CLK_EDGE",
            &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
        );

        let diff_mem = ctx.state.get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY");
        let diff_qdr = ctx
            .state
            .get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY_QDR");
        let diff_net = ctx
            .state
            .get_diff(tile, bel, "INTERFACE_TYPE", "NETWORKING");
        let diff_ddr3 = ctx
            .state
            .get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3");
        let diff_ddr3_v6 = ctx
            .state
            .get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3_V6");
        let diff_os = ctx
            .state
            .get_diff(tile, bel, "INTERFACE_TYPE", "OVERSAMPLE");
        let bitslip_en = diff_net.combine(&!&diff_qdr);
        let diff_net = diff_net.combine(&!&bitslip_en);
        let diff_os = diff_os.combine(&!&bitslip_en);
        ctx.tiledb
            .insert(tile, bel, "BITSLIP_ENABLE", xlat_bit(bitslip_en));
        ctx.tiledb.insert(
            tile,
            bel,
            "INTERFACE_TYPE",
            xlat_enum(vec![
                ("MEMORY", diff_mem),
                ("NETWORKING", diff_net),
                ("MEMORY_DDR3", diff_ddr3),
                ("MEMORY_DDR3_V6", diff_ddr3_v6),
                ("OVERSAMPLE", diff_os),
            ]),
        );

        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "#LATCH");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "DDR_CLK_EDGE"),
            "OPPOSITE_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "#FF");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "DDR_CLK_EDGE"),
            "OPPOSITE_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.tiledb.insert(tile, bel, "IFF_LATCH", xlat_bit(!diff));
        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "DDR");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.tiledb.insert(tile, bel, "IFF_LATCH", xlat_bit(!diff));

        let diff_f = ctx.state.get_diff(tile, bel, "SERDES", "FALSE");
        let diff_t = ctx.state.get_diff(tile, bel, "SERDES", "TRUE");
        let (diff_f, diff_t, mut diff_serdes) = Diff::split(diff_f, diff_t);
        ctx.tiledb
            .insert(tile, bel, "SERDES", xlat_bool(diff_f, diff_t));
        diff_serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_SR_USED"), true, false);
        diff_serdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_LATCH"), false, true);
        diff_serdes.assert_empty();

        let item = ctx.extract_enum(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
        ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = ctx.extract_enum(tile, bel, "D2OFFBYP_SEL", &["GND", "T"]);
        ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = xlat_enum(vec![
            ("T", ctx.state.get_diff(tile, bel, "TFB_USED", "TRUE")),
            ("GND", ctx.state.get_diff(tile, bel, "TFB_USED", "FALSE")),
        ]);
        ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);

        let item = ctx.extract_enum_bool(tile, bel, "IDELMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFFDELMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);

        ctx.state
            .get_diff(tile, bel, "IOBDELAY", "NONE")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IOBDELAY", "IBUF");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IOBDELAY", "IFD");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IOBDELAY", "BOTH");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();

        let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "I_TSBYPASS_ENABLE", item);
        // the fuzzer is slightly fucked to work around some ridiculous ISE bug.
        let _ = ctx.state.get_diff(tile, bel, "IFFMUX", "1");
        let item = ctx.extract_bit(tile, bel, "IFFMUX", "0");
        ctx.tiledb.insert(tile, bel, "IFF_TSBYPASS_ENABLE", item);
        ctx.state
            .get_diff(tile, bel, "OFB_USED", "FALSE")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "OFB_USED", "TRUE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "IFF_TSBYPASS_ENABLE"),
            true,
            false,
        );
        diff.assert_empty();

        ctx.state
            .get_diff(tile, bel, "PRESENT", "ILOGICE2")
            .assert_empty();
        let mut present_iserdes = ctx.state.get_diff(tile, bel, "PRESENT", "ISERDESE2");
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF1_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF2_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF3_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF4_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF1_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF2_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF3_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF4_INIT"), false, true);
        present_iserdes.assert_empty();

        if tile.contains("HR") {
            ctx.state
                .get_diff(tile, bel, "PRESENT", "ILOGICE3")
                .assert_empty();

            ctx.collect_bitvec(tile, bel, "IDELAY_VALUE", "");
            ctx.collect_bitvec(tile, bel, "IFFDELAY_VALUE", "");
            let item = ctx.extract_enum_bool(tile, bel, "ZHOLD_FABRIC", "FALSE", "TRUE");
            ctx.tiledb.insert(tile, bel, "ZHOLD_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "ZHOLD_IFF", "FALSE", "TRUE");
            ctx.tiledb.insert(tile, bel, "ZHOLD_ENABLE", item);

            let diff0 = ctx.state.get_diff(tile, bel, "ZHOLD_FABRIC_INV", "D");
            let diff1 = ctx.state.get_diff(tile, bel, "ZHOLD_FABRIC_INV", "D_B");
            let (diff0, diff1, diff_en) = Diff::split(diff0, diff1);
            ctx.tiledb
                .insert(tile, bel, "INV.ZHOLD_FABRIC", xlat_bool(diff0, diff1));
            ctx.tiledb.insert(tile, bel, "I_ZHOLD", xlat_bit(diff_en));

            let diff0 = ctx.state.get_diff(tile, bel, "ZHOLD_IFF_INV", "D");
            let diff1 = ctx.state.get_diff(tile, bel, "ZHOLD_IFF_INV", "D_B");
            let (diff0, diff1, diff_en) = Diff::split(diff0, diff1);
            ctx.tiledb
                .insert(tile, bel, "INV.ZHOLD_IFF", xlat_bool(diff0, diff1));
            ctx.tiledb.insert(tile, bel, "IFF_ZHOLD", xlat_bit(diff_en));
        }

        let mut vals = vec!["PHASER_ICLK".to_string(), "PHASER_OCLK".to_string()];
        for j in 0..6 {
            vals.push(format!("HCLK{j}"));
        }
        for j in 0..4 {
            vals.push(format!("RCLK{j}"));
        }
        for j in 0..4 {
            vals.push(format!("IOCLK{j}"));
        }
        for j in 0..2 {
            vals.push(format!("CKINT{j}"));
        }
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLK", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKB", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default(tile, bel, "MUX.CLKDIVP", &["CLKDIV", "PHASER"], "NONE");
    }
    for (tile, bel) in [
        ("IO_HR_PAIR", "OLOGIC0"),
        ("IO_HR_PAIR", "OLOGIC1"),
        ("IO_HR_BOT", "OLOGIC"),
        ("IO_HR_TOP", "OLOGIC"),
        ("IO_HP_PAIR", "OLOGIC0"),
        ("IO_HP_PAIR", "OLOGIC1"),
        ("IO_HP_BOT", "OLOGIC"),
        ("IO_HP_TOP", "OLOGIC"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "T1", "T2", "T3", "T4", "CLKDIV",
            "CLKDIVF",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }

        ctx.state
            .get_diff(tile, bel, "CLKINV.SAME", "CLK_B")
            .assert_empty();
        let diff_clk1 = ctx.state.get_diff(tile, bel, "CLKINV.OPPOSITE", "CLK");
        let diff_clk2 = ctx.state.get_diff(tile, bel, "CLKINV.OPPOSITE", "CLK_B");
        let diff_clk12 = ctx.state.get_diff(tile, bel, "CLKINV.SAME", "CLK");
        assert_eq!(diff_clk12, diff_clk1.combine(&diff_clk2));
        ctx.tiledb
            .insert(tile, bel, "INV.CLK1", xlat_bit(!diff_clk1));
        ctx.tiledb
            .insert(tile, bel, "INV.CLK2", xlat_bit(!diff_clk2));

        let item_oq = ctx.extract_enum_bool(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_enum_bool(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.state
            .get_diff(tile, bel, "SRTYPE", "ASYNC")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bit_diff(&item_oq, true, false);
        diff.apply_bit_diff(&item_tq, true, false);
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "OFF_SYNC", item_oq);
        ctx.tiledb.insert(tile, bel, "TFF_SYNC", item_tq);

        let item = ctx.extract_enum_bool(tile, bel, "INIT_OQ.OLOGIC", "0", "1");
        ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "INIT_OQ.OSERDES", "0", "1");
        ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "INIT_TQ.OLOGIC", "0", "1");
        ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "INIT_TQ.OSERDES", "0", "1");
        ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_OQ.OLOGIC", "0", "1");
        ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_OQ.OSERDES", "0", "1");
        ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_TQ.OLOGIC", "0", "1");
        ctx.tiledb.insert(tile, bel, "TFF_SRVAL", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_TQ.OSERDES", "0", "1");
        ctx.tiledb.insert(tile, bel, "TFF_SRVAL", item);

        let osrused = ctx.extract_bit(tile, bel, "OSRUSED", "0");
        let tsrused = ctx.extract_bit(tile, bel, "TSRUSED", "0");
        ctx.tiledb.insert(tile, bel, "OFF_SR_USED", osrused);
        ctx.tiledb.insert(tile, bel, "TFF_SR_USED", tsrused);

        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE_FDBK", "FALSE", "TRUE");
        ctx.collect_enum_default(tile, bel, "MISR_CLK_SELECT", &["CLK1", "CLK2"], "NONE");
        if bel == "OLOGIC" {
            ctx.collect_bit(tile, bel, "MISR_RESET", "1");
        }
        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum_bool(tile, bel, "SELFHEAL", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "RANK3_USED", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "TBYTE_CTL", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "TBYTE_SRC", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "TRISTATE_WIDTH", &["1", "4"]);

        let mut diffs = vec![];
        for val in ["2", "3", "4", "5", "6", "7", "8"] {
            diffs.push((
                val,
                val,
                ctx.state.get_diff(tile, bel, "DATA_WIDTH.SDR", val),
            ));
        }
        for (val, ratio) in [("4", "2"), ("6", "3"), ("8", "4"), ("10", "5"), ("14", "7")] {
            diffs.push((
                val,
                ratio,
                ctx.state.get_diff(tile, bel, "DATA_WIDTH.DDR", val),
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
        ctx.tiledb
            .insert(tile, bel, "DATA_WIDTH", xlat_enum(diffs_width));
        ctx.tiledb
            .insert(tile, bel, "CLK_RATIO", xlat_enum(diffs_ratio));

        let mut diff_sdr = ctx.state.get_diff(tile, bel, "DATA_RATE_OQ", "SDR");
        let mut diff_ddr = ctx.state.get_diff(tile, bel, "DATA_RATE_OQ", "DDR");
        diff_sdr.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_SR_USED"), true, false);
        diff_ddr.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_SR_USED"), true, false);
        ctx.tiledb.insert(
            tile,
            bel,
            "OMUX",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                ("D1", ctx.state.get_diff(tile, bel, "OMUX", "D1")),
                ("SERDES_SDR", diff_sdr),
                ("DDR", diff_ddr),
                ("FF", ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#FF")),
                ("DDR", ctx.state.get_diff(tile, bel, "OUTFFTYPE", "DDR")),
                ("LATCH", ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#LATCH")),
            ]),
        );

        let mut diff_sdr = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "SDR");
        let mut diff_ddr = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "DDR");
        diff_sdr.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_SR_USED"), true, false);
        diff_ddr.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_SR_USED"), true, false);
        ctx.tiledb.insert(
            tile,
            bel,
            "TMUX",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                ("T1", ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "BUF")),
                ("SERDES_SDR", diff_sdr),
                ("DDR", diff_ddr),
                ("FF", ctx.state.get_diff(tile, bel, "TFFTYPE", "#FF")),
                ("DDR", ctx.state.get_diff(tile, bel, "TFFTYPE", "DDR")),
                ("LATCH", ctx.state.get_diff(tile, bel, "TFFTYPE", "#LATCH")),
            ]),
        );

        let mut present_ologic = ctx.state.get_diff(tile, bel, "PRESENT", "OLOGICE2");
        present_ologic.apply_bit_diff(ctx.tiledb.item(tile, bel, "RANK3_USED"), false, true);
        present_ologic.apply_enum_diff(ctx.tiledb.item(tile, bel, "TMUX"), "T1", "NONE");
        present_ologic.assert_empty();
        let mut present_oserdes = ctx.state.get_diff(tile, bel, "PRESENT", "OSERDESE2");
        present_oserdes.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "OFF_SRVAL"), 0, 7);
        present_oserdes.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "TFF_SRVAL"), 0, 7);
        present_oserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_INIT"), false, true);
        present_oserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_INIT"), false, true);
        present_oserdes.assert_empty();

        let mut diffs_clk = vec![("NONE".to_string(), Diff::default())];
        let mut diffs_clkb = vec![("NONE".to_string(), Diff::default())];
        for (src, num) in [("HCLK", 6), ("RCLK", 4), ("IOCLK", 4)] {
            for i in 0..num {
                diffs_clk.push((
                    format!("{src}{i}"),
                    ctx.state
                        .get_diff(tile, bel, "MUX.CLK", format!("{src}{i}")),
                ));
                diffs_clkb.push((
                    format!("{src}{i}"),
                    ctx.state
                        .get_diff(tile, bel, "MUX.CLKB", format!("{src}{i}")),
                ));
            }
        }
        for val in ["CKINT", "PHASER_OCLK"] {
            diffs_clk.push((
                val.to_string(),
                ctx.state.get_diff(tile, bel, "MUX.CLK", val),
            ));
            diffs_clkb.push((
                val.to_string(),
                ctx.state.get_diff(tile, bel, "MUX.CLKB", val),
            ));
        }
        let diff_clk = ctx.state.get_diff(tile, bel, "MUX.CLK", "PHASER_OCLK90");
        let diff_clkb = ctx
            .state
            .get_diff(tile, bel, "MUX.CLK", "PHASER_OCLK90.BOTH")
            .combine(&!&diff_clk);
        diffs_clk.push(("PHASER_OCLK90".to_string(), diff_clk));
        diffs_clkb.push(("PHASER_OCLK90".to_string(), diff_clkb));
        ctx.tiledb
            .insert(tile, bel, "MUX.CLK", xlat_enum_ocd(diffs_clk, OcdMode::Mux));
        ctx.tiledb.insert(
            tile,
            bel,
            "MUX.CLKB",
            xlat_enum_ocd(diffs_clkb, OcdMode::Mux),
        );

        for (attr, attrf) in [
            ("MUX.CLKDIV", "MUX.CLKDIVF"),
            ("MUX.CLKDIVB", "MUX.CLKDIVFB"),
        ] {
            let diff_hclk0f = ctx.state.get_diff(tile, bel, attr, "HCLK0.F");
            let diff_f = ctx
                .state
                .peek_diff(tile, bel, attr, "HCLK0")
                .combine(&!diff_hclk0f);
            let mut diffs = vec![("NONE", Diff::default())];
            for val in [
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "RCLK0", "RCLK1", "RCLK2",
                "RCLK3", "CKINT",
            ] {
                diffs.push((
                    val,
                    ctx.state.get_diff(tile, bel, attr, val).combine(&!&diff_f),
                ));
            }
            ctx.tiledb
                .insert(tile, bel, attrf, xlat_enum_ocd(diffs, OcdMode::Mux));
            ctx.tiledb.insert(
                tile,
                bel,
                attr,
                xlat_enum(vec![
                    ("NONE", Diff::default()),
                    (&attrf[4..], diff_f),
                    (
                        "PHASER_OCLKDIV",
                        ctx.state.get_diff(tile, bel, attr, "PHASER_OCLKDIV"),
                    ),
                ]),
            );
        }
    }
    for tile in ["IO_HR_PAIR", "IO_HP_PAIR"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let mut diff = ctx.state.get_diff(tile, "OLOGIC_COMMON", "MISR_RESET", "1");
        let diff1 = diff.split_bits_by(|bit| bit.tile > 0);
        ctx.tiledb
            .insert(tile, "OLOGIC0", "MISR_RESET", xlat_bit(diff));
        ctx.tiledb
            .insert(tile, "OLOGIC1", "MISR_RESET", xlat_bit(diff1));
    }
    for (tile, bel) in [
        ("IO_HR_PAIR", "IDELAY0"),
        ("IO_HR_PAIR", "IDELAY1"),
        ("IO_HR_BOT", "IDELAY"),
        ("IO_HR_TOP", "IDELAY"),
        ("IO_HP_PAIR", "IDELAY0"),
        ("IO_HP_PAIR", "IDELAY1"),
        ("IO_HP_BOT", "IDELAY"),
        ("IO_HP_TOP", "IDELAY"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        ctx.collect_inv(tile, bel, "C");
        ctx.collect_inv(tile, bel, "DATAIN");
        ctx.collect_inv(tile, bel, "IDATAIN");
        ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CINVCTRL_SEL", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "PIPE_SEL", "FALSE", "TRUE");

        ctx.state
            .get_diff(tile, bel, "DELAYCHAIN_OSC", "FALSE")
            .assert_empty();
        ctx.tiledb.insert(
            tile,
            bel,
            "DELAY_SRC",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                (
                    "IDATAIN",
                    ctx.state.get_diff(tile, bel, "DELAY_SRC", "IDATAIN"),
                ),
                (
                    "DATAIN",
                    ctx.state.get_diff(tile, bel, "DELAY_SRC", "DATAIN"),
                ),
                ("OFB", ctx.state.get_diff(tile, bel, "DELAY_SRC", "OFB")),
                (
                    "DELAYCHAIN_OSC",
                    ctx.state.get_diff(tile, bel, "DELAYCHAIN_OSC", "TRUE"),
                ),
            ]),
        );

        ctx.tiledb.insert(
            tile,
            bel,
            "IDELAY_TYPE",
            xlat_enum(vec![
                (
                    "FIXED",
                    ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "FIXED"),
                ),
                (
                    "VARIABLE",
                    ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "VARIABLE"),
                ),
                (
                    "VAR_LOAD",
                    ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "VAR_LOAD"),
                ),
                (
                    "VAR_LOAD",
                    ctx.state
                        .get_diff(tile, bel, "IDELAY_TYPE", "VAR_LOAD_PIPE"),
                ),
            ]),
        );
        let mut diffs_t = vec![];
        let mut diffs_f = vec![];
        for diff in ctx.state.get_diffs(tile, bel, "IDELAY_VALUE", "") {
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
        ctx.tiledb
            .insert(tile, bel, "IDELAY_VALUE_INIT", xlat_bitvec(diffs_t));
        ctx.tiledb
            .insert(tile, bel, "IDELAY_VALUE_CUR", xlat_bitvec(diffs_f));
        if tile.contains("HP") {
            ctx.collect_enum(tile, bel, "FINEDELAY", &["BYPASS", "ADD_DLY"]);
        }
    }
    for (tile, bel) in [
        ("IO_HP_PAIR", "ODELAY0"),
        ("IO_HP_PAIR", "ODELAY1"),
        ("IO_HP_BOT", "ODELAY"),
        ("IO_HP_TOP", "ODELAY"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_inv(tile, bel, "C");
        ctx.collect_inv(tile, bel, "ODATAIN");
        ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CINVCTRL_SEL", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "PIPE_SEL", "FALSE", "TRUE");
        ctx.state
            .get_diff(tile, bel, "DELAYCHAIN_OSC", "FALSE")
            .assert_empty();

        ctx.tiledb.insert(
            tile,
            bel,
            "DELAY_SRC",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                (
                    "ODATAIN",
                    ctx.state.get_diff(tile, bel, "DELAY_SRC", "ODATAIN"),
                ),
                ("CLKIN", ctx.state.get_diff(tile, bel, "DELAY_SRC", "CLKIN")),
                (
                    "DELAYCHAIN_OSC",
                    ctx.state.get_diff(tile, bel, "DELAYCHAIN_OSC", "TRUE"),
                ),
            ]),
        );

        let en = ctx.extract_bit(tile, bel, "ODELAY_TYPE", "FIXED");
        let mut diff_var = ctx.state.get_diff(tile, bel, "ODELAY_TYPE", "VARIABLE");
        diff_var.apply_bit_diff(&en, true, false);
        let mut diff_vl = ctx.state.get_diff(tile, bel, "ODELAY_TYPE", "VAR_LOAD");
        diff_vl.apply_bit_diff(&en, true, false);
        ctx.tiledb.insert(tile, bel, "ENABLE", en);
        ctx.tiledb.insert(
            tile,
            bel,
            "ODELAY_TYPE",
            xlat_enum(vec![
                ("FIXED", Diff::default()),
                ("VARIABLE", diff_var),
                ("VAR_LOAD", diff_vl),
            ]),
        );

        let mut diffs_t = vec![];
        let mut diffs_f = vec![];
        for diff in ctx.state.get_diffs(tile, bel, "ODELAY_VALUE", "") {
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
        ctx.tiledb
            .insert(tile, bel, "ODELAY_VALUE_INIT", xlat_bitvec(diffs_t));
        ctx.tiledb
            .insert(tile, bel, "ODELAY_VALUE_CUR", xlat_bitvec(diffs_f));
        ctx.collect_enum(tile, bel, "FINEDELAY", &["BYPASS", "ADD_DLY"]);
    }
}
