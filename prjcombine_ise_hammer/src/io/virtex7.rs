use bitvec::prelude::*;
use prjcombine_collector::{
    extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_wide, xlat_bitvec, xlat_bool,
    xlat_enum, xlat_enum_ocd, Diff, OcdMode,
};
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelKV, ExtraFeature, ExtraFeatureKind, TileBits, TileFuzzKV, TileKV, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_one,
    fuzz_one_extras,
    io::iostd::DiffKind,
};

use super::iostd::{DciKind, Iostd};

const HP_IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6", "8"]),
    Iostd::odci("LVDCI_18", 1800),
    Iostd::odci("LVDCI_15", 1500),
    Iostd::odci_half("LVDCI_DV2_18", 1800),
    Iostd::odci_half("LVDCI_DV2_15", 1500),
    Iostd::odci_vref("HSLVDCI_18", 1800, 900),
    Iostd::odci_vref("HSLVDCI_15", 1500, 750),
    Iostd::vref("SSTL18_I", 1800, 900),
    Iostd::vref("SSTL18_II", 1800, 900),
    Iostd::vref("SSTL15", 1500, 750),
    Iostd::vref("SSTL135", 1350, 675),
    Iostd::vref("SSTL12", 1200, 600),
    Iostd::vref("HSTL_I_18", 1800, 900),
    Iostd::vref("HSTL_II_18", 1800, 900),
    Iostd::vref("HSTL_I", 1500, 750),
    Iostd::vref("HSTL_II", 1500, 750),
    Iostd::vref("HSTL_I_12", 1200, 600),
    Iostd::vref("HSUL_12", 1200, 600),
    Iostd::vref_dci("SSTL18_I_DCI", 1800, 900, DciKind::InputSplit),
    Iostd::vref_dci("SSTL18_II_DCI", 1800, 900, DciKind::BiSplit),
    Iostd::vref_dci("SSTL18_II_T_DCI", 1800, 900, DciKind::BiSplitT),
    Iostd::vref_dci("SSTL15_DCI", 1500, 750, DciKind::InputSplit),
    Iostd::vref_dci("SSTL15_T_DCI", 1500, 750, DciKind::BiSplitT),
    Iostd::vref_dci("SSTL135_DCI", 1350, 675, DciKind::InputSplit),
    Iostd::vref_dci("SSTL135_T_DCI", 1350, 675, DciKind::BiSplitT),
    Iostd::vref_dci("SSTL12_DCI", 1200, 600, DciKind::InputSplit),
    Iostd::vref_dci("SSTL12_T_DCI", 1200, 600, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_I_DCI_18", 1800, 900, DciKind::InputSplit),
    Iostd::vref_dci("HSTL_II_DCI_18", 1800, 900, DciKind::BiSplit),
    Iostd::vref_dci("HSTL_II_T_DCI_18", 1800, 900, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_I_DCI", 1500, 750, DciKind::InputSplit),
    Iostd::vref_dci("HSTL_II_DCI", 1500, 750, DciKind::BiSplit),
    Iostd::vref_dci("HSTL_II_T_DCI", 1500, 750, DciKind::BiSplitT),
    Iostd::vref_dci("HSUL_12_DCI", 1200, 600, DciKind::Output),
    Iostd::pseudo_diff("DIFF_SSTL18_I", 1800),
    Iostd::pseudo_diff("DIFF_SSTL18_II", 1800),
    Iostd::pseudo_diff("DIFF_SSTL15", 1500),
    Iostd::pseudo_diff("DIFF_SSTL135", 1350),
    Iostd::pseudo_diff("DIFF_SSTL12", 1200),
    Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_I", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_II", 1500),
    Iostd::pseudo_diff("DIFF_HSUL_12", 1200),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_I_DCI", 1800, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_II_DCI", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_II_T_DCI", 1800, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_SSTL15_DCI", 1500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL15_T_DCI", 1500, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_SSTL135_DCI", 1350, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL135_T_DCI", 1350, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_SSTL12_DCI", 1200, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL12_T_DCI", 1200, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_HSTL_I_DCI_18", 1800, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI_18", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_T_DCI_18", 1800, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_HSTL_I_DCI", 1500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI", 1500, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_T_DCI", 1500, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_HSUL_12_DCI", 1200, DciKind::Output),
    Iostd::true_diff("LVDS", 1800),
];

const HR_IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &["4", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS33", 3300, &["4", "8", "12", "16"]),
    Iostd::cmos("LVCMOS25", 2500, &["4", "8", "12", "16"]),
    Iostd::cmos("LVCMOS18", 1800, &["4", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS15", 1500, &["4", "8", "12", "16"]),
    Iostd::cmos("LVCMOS12", 1200, &["4", "8", "12"]),
    Iostd::cmos("PCI33_3", 3300, &[]),
    Iostd::cmos("MOBILE_DDR", 1800, &[]),
    Iostd::vref("SSTL18_I", 1800, 900),
    Iostd::vref("SSTL18_II", 1800, 900),
    Iostd::vref("SSTL15", 1500, 750),
    Iostd::vref("SSTL15_R", 1500, 750),
    Iostd::vref("SSTL135", 1350, 675),
    Iostd::vref("SSTL135_R", 1350, 675),
    Iostd::vref("HSTL_I_18", 1800, 900),
    Iostd::vref("HSTL_II_18", 1800, 900),
    Iostd::vref("HSTL_I", 1500, 750),
    Iostd::vref("HSTL_II", 1500, 750),
    Iostd::vref("HSUL_12", 1200, 600),
    Iostd::pseudo_diff("DIFF_MOBILE_DDR", 1800),
    Iostd::pseudo_diff("DIFF_SSTL18_I", 1800),
    Iostd::pseudo_diff("DIFF_SSTL18_II", 1800),
    Iostd::pseudo_diff("DIFF_SSTL15", 1500),
    Iostd::pseudo_diff("DIFF_SSTL15_R", 1500),
    Iostd::pseudo_diff("DIFF_SSTL135", 1350),
    Iostd::pseudo_diff("DIFF_SSTL135_R", 1350),
    Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_I", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_II", 1500),
    Iostd::pseudo_diff("DIFF_HSUL_12", 1200),
    Iostd::pseudo_diff("BLVDS_25", 2500),
    Iostd::true_diff("LVDS_25", 2500),
    Iostd::true_diff("MINI_LVDS_25", 2500),
    Iostd::true_diff("RSDS_25", 2500),
    Iostd::true_diff("PPDS_25", 2500),
    Iostd::true_diff("TMDS_33", 3300),
];

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let package = backend
        .device
        .bonds
        .values()
        .max_by_key(|bond| {
            let bdata = &backend.db.bonds[bond.bond];
            let prjcombine_xilinx_geom::Bond::Virtex4(bdata) = bdata else {
                unreachable!();
            };
            bdata.pins.len()
        })
        .unwrap();
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
    for (tile, bel, bel_ologic, bel_odelay, bel_other) in [
        (
            "IO_HP_PAIR",
            "IOB0",
            BelId::from_idx(2),
            BelId::from_idx(6),
            Some(BelId::from_idx(9)),
        ),
        (
            "IO_HP_PAIR",
            "IOB1",
            BelId::from_idx(3),
            BelId::from_idx(7),
            Some(BelId::from_idx(8)),
        ),
        (
            "IO_HP_BOT",
            "IOB",
            BelId::from_idx(1),
            BelId::from_idx(3),
            None,
        ),
        (
            "IO_HP_TOP",
            "IOB",
            BelId::from_idx(1),
            BelId::from_idx(3),
            None,
        ),
    ] {
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, bel, TileBits::MainAuto) else {
            continue;
        };
        let hclk_ioi = backend.egrid.db.get_node("HCLK_IOI_HP");
        fuzz_one!(ctx, "PRESENT", "IOB", [
            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
            (package package.name),
            (bel_special BelKV::IsBonded)
        ], [
            (mode "IOB18")
        ]);
        fuzz_one!(ctx, "PRESENT", "IOB.QUIET", [
            (global_opt "DCIUPDATEMODE", "QUIET"),
            (package package.name),
            (bel_special BelKV::IsBonded)
        ], [
            (mode "IOB18")
        ]);
        if bel != "IOB" {
            fuzz_one!(ctx, "PRESENT", "IPAD", [
                (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                (package package.name),
                (bel_special BelKV::IsBonded)
            ], [
                (mode "IPAD")
            ]);
        }
        fuzz_enum!(ctx, "PULL", ["KEEPER", "PULLDOWN", "PULLUP"], [
            (package package.name),
            (bel_special BelKV::IsBonded),
            (mode "IOB18")
        ]);
        for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
            fuzz_one!(ctx, "PULL_DYNAMIC", "1", [
                (package package.name),
                (bel_special BelKV::IsBonded),
                (mutex "PULL_DYNAMIC", pin),
                (mode "IOB18")
            ], [
                (pin_pips pin)
            ]);
        }
        fuzz_multi_attr_bin!(ctx, "IPROGRAMMING", 24, [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (mode "IOB18"),
            (pin "I"),
            (pin "O"),
            (attr "OPROGRAMMING", "0000000000000000000000000000000000"),
            (attr "IUSED", "0"),
            (attr "OUSED", "0"),
            (attr "ISTANDARD", "LVCMOS18"),
            (attr "OSTANDARD", "LVCMOS18"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        fuzz_multi_attr_bin!(ctx, "OPROGRAMMING", 34, [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (mode "IOB18"),
            (pin "O"),
            (attr "OUSED", "0"),
            (attr "OSTANDARD", "LVCMOS18"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        for &std in HP_IOSTDS {
            if bel == "IOB" && !matches!(std.name, "LVCMOS18" | "HSTL_I") {
                continue;
            }
            let mut extras = vec![];
            let mut vref_special = BelKV::Nop;
            let mut dci_special = BelKV::Nop;
            if std.vref.is_some() {
                vref_special = BelKV::PrepVref;
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::Vref,
                    "IO_HP_PAIR",
                    "IOB0",
                    "PRESENT",
                    "VREF",
                ));
            }
            if std.dci == DciKind::BiSplitT {
                continue;
            } else if matches!(
                std.dci,
                DciKind::BiSplit | DciKind::BiVcc | DciKind::InputSplit | DciKind::InputVcc
            ) {
                dci_special = BelKV::PrepDci;
                extras.extend([
                    ExtraFeature::new(ExtraFeatureKind::VrBot, "IO_HP_BOT", "IOB", "PRESENT", "VR"),
                    ExtraFeature::new(ExtraFeatureKind::VrTop, "IO_HP_TOP", "IOB", "PRESENT", "VR"),
                ]);
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::Hclk(0, 0),
                    "HCLK_IOI_HP",
                    "DCI",
                    "STD",
                    std.name,
                ));
            }
            if std.diff != DiffKind::None {
                if let Some(bel_other) = bel_other {
                    for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                        fuzz_one_extras!(ctx, "ISTD", format!("{sn}.{suffix}", sn=std.name), [
                            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (mode "IOB18"),
                            (attr "OUSED", ""),
                            (pin "I"),
                            (pin "DIFFI_IN"),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_special dci_special.clone()),
                            (bel_mode bel_other, "IOB"),
                            (bel_pin bel_other, "PADOUT"),
                            (bel_attr bel_other, "OUSED", "")
                        ], [
                            (attr "IUSED", "0"),
                            (attr "DIFFI_INUSED", "0"),
                            (attr "ISTANDARD", std.name),
                            (attr "DIFF_TERM", if std.diff == DiffKind::True {"FALSE"} else {""}),
                            (attr "IBUF_LOW_PWR", lp),
                            (bel_attr bel_other, "PADOUTUSED", "0"),
                            (bel_attr bel_other, "ISTANDARD", std.name),
                            (bel_attr bel_other, "DIFF_TERM", if std.diff == DiffKind::True {"FALSE"} else {""}),
                            (bel_attr bel_other, "IBUF_LOW_PWR", lp)
                        ], extras.clone());
                    }
                    if std.diff == DiffKind::True && bel == "IOB0" {
                        fuzz_one!(ctx, "DIFF_TERM", std.name, [
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (mode "IOB18"),
                            (attr "OUSED", ""),
                            (pin "I"),
                            (pin "DIFFI_IN"),
                            (attr "IUSED", "0"),
                            (attr "DIFFI_INUSED", "0"),
                            (attr "ISTANDARD", std.name),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_special dci_special.clone()),
                            (bel_mode bel_other, "IOB"),
                            (bel_pin bel_other, "PADOUT"),
                            (bel_attr bel_other, "OUSED", ""),
                            (bel_attr bel_other, "PADOUTUSED", "0"),
                            (bel_attr bel_other, "ISTANDARD", std.name)
                        ], [
                            (attr_diff "DIFF_TERM", "FALSE", "TRUE"),
                            (bel_attr_diff bel_other, "DIFF_TERM", "FALSE", "TRUE")
                        ]);
                        fuzz_one!(ctx, "DIFF_TERM_DYNAMIC", std.name, [
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (mode "IOB18"),
                            (attr "OUSED", ""),
                            (pin "I"),
                            (pin "DIFFI_IN"),
                            (attr "IUSED", "0"),
                            (attr "DIFFI_INUSED", "0"),
                            (attr "ISTANDARD", std.name),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_special dci_special),
                            (bel_mode bel_other, "IOB"),
                            (bel_pin bel_other, "PADOUT"),
                            (bel_attr bel_other, "OUSED", ""),
                            (bel_attr bel_other, "PADOUTUSED", "0"),
                            (bel_attr bel_other, "ISTANDARD", std.name)
                        ], [
                            (attr_diff "DIFF_TERM", "FALSE", "TRUE"),
                            (bel_attr_diff bel_other, "DIFF_TERM", "FALSE", "TRUE"),
                            (pin_pips "DIFF_TERM_INT_EN")
                        ]);
                    }
                }
            } else {
                for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                    fuzz_one_extras!(ctx, "ISTD", format!("{sn}.{suffix}", sn=std.name), [
                        (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                        (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (mode "IOB18"),
                        (attr "OUSED", ""),
                        (pin "I"),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special vref_special.clone()),
                        (bel_special dci_special.clone())
                    ], [
                        (attr "IUSED", "0"),
                        (attr "ISTANDARD", std.name),
                        (attr "IBUF_LOW_PWR", lp)
                    ], extras.clone());
                }
            }
        }
        fuzz_enum!(ctx, "IBUFDISABLE_SEL", ["GND", "I"], [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (package package.name),
            (bel_special BelKV::IsBonded),
            (mode "IOB18"),
            (pin "I"),
            (pin "O"),
            (pin "IBUFDISABLE"),
            (attr "IUSED", "0"),
            (attr "OUSED", "0"),
            (attr "ISTANDARD", "LVCMOS18"),
            (attr "OSTANDARD", "LVCMOS18"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        fuzz_enum!(ctx, "DCITERMDISABLE_SEL", ["GND", "I"], [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (package package.name),
            (bel_special BelKV::IsBonded),
            (mode "IOB18"),
            (pin "I"),
            (pin "O"),
            (pin "DCITERMDISABLE"),
            (attr "IUSED", "0"),
            (attr "OUSED", "0"),
            (attr "ISTANDARD", "LVCMOS18"),
            (attr "OSTANDARD", "LVCMOS18"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        for &std in HP_IOSTDS {
            if bel == "IOB" && std.name != "HSTL_I" {
                continue;
            }
            let mut extras = vec![];
            let mut dci_special = BelKV::Nop;
            if matches!(
                std.dci,
                DciKind::Output
                    | DciKind::OutputHalf
                    | DciKind::BiSplit
                    | DciKind::BiVcc
                    | DciKind::BiSplitT
            ) {
                extras.extend([
                    ExtraFeature::new(ExtraFeatureKind::VrBot, "IO_HP_BOT", "IOB", "PRESENT", "VR"),
                    ExtraFeature::new(ExtraFeatureKind::VrTop, "IO_HP_TOP", "IOB", "PRESENT", "VR"),
                    ExtraFeature::new(
                        ExtraFeatureKind::Hclk(0, 0),
                        "HCLK_IOI_HP",
                        "DCI",
                        "STD",
                        std.name,
                    ),
                ]);
                dci_special = BelKV::PrepDci;
            }
            if std.diff == DiffKind::True {
                if bel == "IOB1" {
                    let bel_other = bel_other.unwrap();
                    let extras = vec![ExtraFeature::new(
                        ExtraFeatureKind::Hclk(0, 0),
                        "HCLK_IOI_HP",
                        "LVDS",
                        "STD",
                        std.name,
                    )];
                    fuzz_one_extras!(ctx, "OSTD", std.name, [
                        (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (attr "IUSED", ""),
                        (attr "OPROGRAMMING", ""),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special BelKV::PrepDiffOut),
                        (bel_attr bel_other, "IUSED", ""),
                        (bel_attr bel_other, "OPROGRAMMING", ""),
                        (bel_attr bel_other, "OSTANDARD", ""),
                        (bel_attr bel_other, "OUSED", "")
                    ], [
                        (mode_diff "IOB18", "IOB18M"),
                        (pin "O"),
                        (attr "OUSED", "0"),
                        (attr "DIFFO_OUTUSED", "0"),
                        (attr "OSTANDARD", std.name),
                        (bel_mode_diff bel_other, "IOB18", "IOB18S"),
                        (bel_attr bel_other, "OUTMUX", "1"),
                        (bel_attr bel_other, "DIFFO_INUSED", "0"),
                        (pin_pair "DIFFO_OUT", bel_other, "DIFFO_IN")
                    ], extras);
                }
            } else if std.diff != DiffKind::None {
                if bel == "IOB1" {
                    let bel_other = bel_other.unwrap();
                    let bel_other_ologic = BelId::from_idx(2);
                    for slew in ["SLOW", "FAST"] {
                        if std.dci == DciKind::BiSplitT {
                            fuzz_one_extras!(ctx, "OSTD", format!("{name}.{slew}", name=std.name), [
                                (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                                (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                                (related TileRelation::Hclk(hclk_ioi),
                                    (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                                (attr "OPROGRAMMING", ""),
                                (package package.name),
                                (bel_special BelKV::IsBonded),
                                (bel_special dci_special.clone()),
                                (bel_attr bel_other, "OPROGRAMMING", ""),
                                (bel_mode bel_other_ologic, "OLOGICE2")
                            ], [
                                (mode_diff "IOB18", "IOB18M"),
                                (pin "O"),
                                (pin "I"),
                                (attr "OUSED", "0"),
                                (attr "IUSED", "0"),
                                (attr "O_OUTUSED", "0"),
                                (attr "OSTANDARD", std.name),
                                (attr "ISTANDARD", std.name),
                                (attr "SLEW", slew),
                                (bel_mode_diff bel_other, "IOB18", "IOB18S"),
                                (bel_pin bel_other, "I"),
                                (bel_attr bel_other, "IUSED", "0"),
                                (bel_attr bel_other, "OUTMUX", "0"),
                                (bel_attr bel_other, "OINMUX", "1"),
                                (bel_attr bel_other, "OSTANDARD", std.name),
                                (bel_attr bel_other, "ISTANDARD", std.name),
                                (bel_attr bel_other, "SLEW", slew),
                                (pin_pair "O_OUT", bel_other, "O_IN")
                            ], extras.clone());
                        } else {
                            fuzz_one_extras!(ctx, "OSTD", format!("{name}.{slew}", name=std.name), [
                                (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                                (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                                (related TileRelation::Hclk(hclk_ioi),
                                    (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                                (attr "IUSED", ""),
                                (attr "OPROGRAMMING", ""),
                                (package package.name),
                                (bel_special BelKV::IsBonded),
                                (bel_special dci_special.clone()),
                                (bel_attr bel_other, "IUSED", ""),
                                (bel_attr bel_other, "OPROGRAMMING", ""),
                                (bel_mode bel_other_ologic, "OLOGICE2")
                            ], [
                                (mode_diff "IOB18", "IOB18M"),
                                (pin "O"),
                                (attr "OUSED", "0"),
                                (attr "O_OUTUSED", "0"),
                                (attr "OSTANDARD", std.name),
                                (attr "SLEW", slew),
                                (bel_mode_diff bel_other, "IOB18", "IOB18S"),
                                (bel_attr bel_other, "OUTMUX", "0"),
                                (bel_attr bel_other, "OINMUX", "1"),
                                (bel_attr bel_other, "OSTANDARD", std.name),
                                (bel_attr bel_other, "SLEW", slew),
                                (pin_pair "O_OUT", bel_other, "O_IN")
                            ], extras.clone());
                        }
                    }
                }
            } else if std.dci == DciKind::BiSplitT {
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::Vref,
                    "IO_HP_PAIR",
                    "IOB0",
                    "PRESENT",
                    "VREF",
                ));
                for slew in ["SLOW", "FAST"] {
                    fuzz_one_extras!(ctx, "OSTD", format!("{name}.{slew}", name=std.name), [
                        (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                        (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (mode "IOB18"),
                        (pin "O"),
                        (pin "I"),
                        (attr "OPROGRAMMING", ""),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special BelKV::PrepVref),
                        (bel_special dci_special.clone())
                    ], [
                        (attr "OUSED", "0"),
                        (attr "IUSED", "0"),
                        (attr "OSTANDARD", std.name),
                        (attr "ISTANDARD", std.name),
                        (attr "SLEW", slew)
                    ], extras.clone());
                }
            } else {
                let drives = if std.drive.is_empty() {
                    &[""][..]
                } else {
                    std.drive
                };
                let slews = if std.name.contains("LVDCI") {
                    &[""][..]
                } else {
                    &["SLOW", "FAST"][..]
                };
                for &drive in drives {
                    for slew in slews {
                        let val = if slew.is_empty() {
                            std.name.to_string()
                        } else if drive.is_empty() {
                            format!("{name}.{slew}", name = std.name)
                        } else {
                            format!("{name}.{drive}.{slew}", name = std.name)
                        };
                        fuzz_one_extras!(ctx, "OSTD", val, [
                            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (mode "IOB18"),
                            (pin "O"),
                            (attr "IUSED", ""),
                            (attr "OPROGRAMMING", ""),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_special dci_special.clone())
                        ], [
                            (attr "OUSED", "0"),
                            (attr "OSTANDARD", std.name),
                            (attr "DRIVE", drive),
                            (attr "SLEW", slew)
                        ], extras.clone());
                    }
                }
            }
        }

        if bel != "IOB" {
            for (std, vcco, vref) in [
                ("HSTL_I_12", 1200, 600),
                ("SSTL135", 1350, 675),
                ("HSTL_I", 1500, 750),
                ("HSTL_I_18", 1800, 900),
                // ("HSTL_III_18", 1800, 1100),
                // ("SSTL2_I", 2500, 1250),
            ] {
                let extras = vec![ExtraFeature::new(
                    ExtraFeatureKind::Hclk(0, 0),
                    "HCLK_IOI_HP",
                    "INTERNAL_VREF",
                    "VREF",
                    format!("{vref}"),
                )];
                fuzz_one_extras!(ctx, "ISTD", format!("{std}.LP"), [
                    (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                    (related TileRelation::Hclk(hclk_ioi),
                        (tile_mutex "VCCO", vcco.to_string())),
                    (mode "IOB18"),
                    (attr "OUSED", ""),
                    (pin "I"),
                    (package package.name),
                    (bel_special BelKV::IsBonded),
                    (bel_special BelKV::PrepVrefInternal(vref))
                ], [
                    (attr "IUSED", "0"),
                    (attr "ISTANDARD", std),
                    (attr "IBUF_LOW_PWR", "TRUE")
                ], extras);
            }
        }

        fuzz_one!(ctx, "OUTPUT_DELAY", "0", [
            (mutex "OUTPUT_DELAY", "0"),
            (bel_mode bel_odelay, "ODELAYE2"),
            (bel_mode bel_ologic, "OLOGICE2")
        ], [
            (pip (bel_pin bel_ologic, "OQ"), (bel_pin bel_ologic, "IOB_O"))
        ]);
        fuzz_one!(ctx, "OUTPUT_DELAY", "1", [
            (mutex "OUTPUT_DELAY", "1"),
            (bel_mode bel_odelay, "ODELAYE2"),
            (bel_mode bel_ologic, "OLOGICE2")
        ], [
            (pip (bel_pin bel_odelay, "DATAOUT"), (bel_pin bel_ologic, "IOB_O"))
        ]);
    }
    if let Some(ctx) = FuzzCtx::try_new(session, backend, "HCLK_IOI_HP", "DCI", TileBits::Hclk) {
        fuzz_one!(ctx, "TEST_ENABLE", "1", [
            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
            (global_mutex "GLOBAL_DCI", "NOPE")
        ], [
            (mode "DCI")
        ]);
        fuzz_one!(ctx, "TEST_ENABLE", "QUIET", [
            (global_opt "DCIUPDATEMODE", "QUIET"),
            (global_mutex "GLOBAL_DCI", "NOPE")
        ], [
            (mode "DCI")
        ]);
        fuzz_one!(ctx, "DYNAMIC_ENABLE", "1", [
            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
            (global_mutex "GLOBAL_DCI", "NOPE")
        ], [
            (mode "DCI"),
            (pin_pips "INT_DCI_EN")
        ]);
        let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
        let extras = vec![
            ExtraFeature::new(ExtraFeatureKind::Cfg, "CFG", "MISC", "DCI_CLK_ENABLE", "1"),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciIo(0),
                "IO_HP_PAIR",
                "IOB0",
                "OSTD",
                "HSLVDCI_18",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciHclk(0),
                "HCLK_IOI_HP",
                "DCI",
                "ENABLE",
                "1",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciVrBot(0),
                "IO_HP_BOT",
                "IOB",
                "PRESENT",
                "VR",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciVrTop(0),
                "IO_HP_TOP",
                "IOB",
                "PRESENT",
                "VR",
            ),
        ];
        fuzz_one_extras!(ctx, "CENTER_DCI", "1", [
            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
            (package package.name),
            (special TileKV::CenterDci(0))
        ], [], extras);
        for (a, b) in [(0, 1), (1, 0)] {
            let extras = vec![
                ExtraFeature::new(
                    ExtraFeatureKind::CenterDciIo(b),
                    "IO_HP_PAIR",
                    "IOB0",
                    "OSTD",
                    "HSLVDCI_18",
                ),
                ExtraFeature::new(
                    ExtraFeatureKind::CenterDciHclk(b),
                    "HCLK_IOI_HP",
                    "DCI",
                    if b == 0 {
                        "CASCADE_FROM_ABOVE"
                    } else {
                        "CASCADE_FROM_BELOW"
                    },
                    "1",
                ),
            ];
            fuzz_one_extras!(ctx, format!("CASCADE_DCI.{a}.{b}"), "1", [
                (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                (package package.name),
                (special TileKV::CascadeDci(a, b))
            ], [], extras);
        }
    }
    for (tile, bel, dy, bel_other) in [
        ("IO_HR_PAIR", "IOB0", 2, Some(BelId::from_idx(7))),
        ("IO_HR_PAIR", "IOB1", 2, Some(BelId::from_idx(6))),
        ("IO_HR_BOT", "IOB", 1, None),
        ("IO_HR_TOP", "IOB", -2, None),
    ] {
        let hclk_ioi = backend.egrid.db.get_node("HCLK_IOI_HR");
        let Some(ctx) = FuzzCtx::try_new(session, backend, tile, bel, TileBits::MainAuto) else {
            continue;
        };

        fuzz_one!(ctx, "PRESENT", "IOB", [
            (package package.name),
            (bel_special BelKV::IsBonded)
        ], [
            (mode "IOB33")
        ]);
        if bel != "IOB" {
            fuzz_one!(ctx, "PRESENT", "IPAD", [
                (package package.name),
                (bel_special BelKV::IsBonded)
            ], [
                (mode "IPAD")
            ]);
        }
        fuzz_enum!(ctx, "PULL", ["KEEPER", "PULLDOWN", "PULLUP"], [
            (package package.name),
            (bel_special BelKV::IsBonded),
            (mode "IOB33")
        ]);
        for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
            fuzz_one!(ctx, "PULL_DYNAMIC", "1", [
                (package package.name),
                (bel_special BelKV::IsBonded),
                (mutex "PULL_DYNAMIC", pin),
                (mode "IOB33")
            ], [
                (pin_pips pin)
            ]);
        }
        fuzz_multi_attr_bin!(ctx, "OPROGRAMMING", 39, [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "3300")),
            (mode "IOB33"),
            (pin "O"),
            (attr "OUSED", "0"),
            (attr "OSTANDARD", "LVCMOS33"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        fuzz_multi_attr_bin!(ctx, "IPROGRAMMING", 9, [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "3300")),
            (mode "IOB33"),
            (pin "I"),
            (pin "O"),
            (attr "OPROGRAMMING", "000000000000000000000000000000000000000"),
            (attr "IUSED", "0"),
            (attr "OUSED", "0"),
            (attr "ISTANDARD", "LVCMOS33"),
            (attr "OSTANDARD", "LVCMOS33"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        fuzz_enum!(ctx, "IBUFDISABLE_SEL", ["GND", "I"], [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (package package.name),
            (bel_special BelKV::IsBonded),
            (mode "IOB33"),
            (pin "I"),
            (pin "O"),
            (pin "IBUFDISABLE"),
            (attr "IUSED", "0"),
            (attr "OUSED", "0"),
            (attr "ISTANDARD", "LVCMOS18"),
            (attr "OSTANDARD", "LVCMOS18"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        fuzz_enum!(ctx, "INTERMDISABLE_SEL", ["GND", "I"], [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (package package.name),
            (bel_special BelKV::IsBonded),
            (mode "IOB33"),
            (pin "I"),
            (pin "O"),
            (pin "INTERMDISABLE"),
            (attr "IUSED", "0"),
            (attr "OUSED", "0"),
            (attr "ISTANDARD", "LVCMOS18"),
            (attr "OSTANDARD", "LVCMOS18"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        fuzz_enum!(ctx, "DQS_BIAS", ["FALSE", "TRUE"], [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (package package.name),
            (bel_special BelKV::IsBonded),
            (mode "IOB33"),
            (pin "I"),
            (pin "O"),
            (attr "IUSED", "0"),
            (attr "OUSED", "0"),
            (attr "ISTANDARD", "LVCMOS18"),
            (attr "OSTANDARD", "LVCMOS18"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        fuzz_enum!(ctx, "IN_TERM", ["NONE", "UNTUNED_SPLIT_40", "UNTUNED_SPLIT_50", "UNTUNED_SPLIT_60"], [
            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (package package.name),
            (bel_special BelKV::IsBonded),
            (bel_special BelKV::PrepVref),
            (mode "IOB33"),
            (pin "I"),
            (pin "O"),
            (attr "IUSED", "0"),
            (attr "OUSED", "0"),
            (attr "ISTANDARD", "SSTL18_II"),
            (attr "OSTANDARD", "SSTL18_II"),
            (attr "SLEW", "SLOW")
        ]);
        let io_pair = backend.egrid.db.get_node("IO_HR_PAIR");
        let bel_iob0 = BelId::from_idx(6);
        let bel_iob1 = BelId::from_idx(7);
        for &std in HR_IOSTDS {
            if bel == "IOB" && !matches!(std.name, "PCI33_3" | "LVCMOS18" | "LVCMOS33" | "HSTL_I") {
                continue;
            }
            let mut extras = vec![];
            let mut vref_special = BelKV::Nop;
            if std.vref.is_some() {
                vref_special = BelKV::PrepVref;
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::Vref,
                    "IO_HR_PAIR",
                    "IOB0",
                    "PRESENT",
                    "VREF",
                ));
            }
            let anchor_std = match std.vcco {
                Some(3300) => "LVCMOS33",
                Some(2500) => "LVCMOS25",
                Some(1800) => "LVCMOS18",
                Some(1500) => "LVCMOS15",
                Some(1200) => "LVCMOS12",
                Some(1350) => "SSTL135",
                _ => unreachable!(),
            };
            if std.diff != DiffKind::None {
                if let Some(bel_other) = bel_other {
                    for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                        fuzz_one_extras!(ctx, "ISTD", format!("{sn}.{suffix}", sn=std.name), [
                            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_mode bel_iob1, "IOB33")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_pin bel_iob1, "O")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OUSED", "0")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OSTANDARD", anchor_std)),
                            (mode "IOB33"),
                            (attr "OUSED", ""),
                            (pin "I"),
                            (pin "DIFFI_IN"),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_mode bel_other, "IOB"),
                            (bel_pin bel_other, "PADOUT"),
                            (bel_attr bel_other, "OUSED", "")
                        ], [
                            (attr "IUSED", "0"),
                            (attr "DIFFI_INUSED", "0"),
                            (attr "ISTANDARD", std.name),
                            (attr "DIFF_TERM", if std.diff == DiffKind::True {"FALSE"} else {""}),
                            (attr "IBUF_LOW_PWR", lp),
                            (bel_attr bel_other, "PADOUTUSED", "0"),
                            (bel_attr bel_other, "ISTANDARD", std.name),
                            (bel_attr bel_other, "DIFF_TERM", if std.diff == DiffKind::True {"FALSE"} else {""}),
                            (bel_attr bel_other, "IBUF_LOW_PWR", lp)
                        ], extras.clone());
                    }
                    if std.diff == DiffKind::True && bel == "IOB0" && std.name != "TMDS_33" {
                        fuzz_one!(ctx, "DIFF_TERM", std.name, [
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_mode bel_iob1, "IOB33")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_pin bel_iob1, "O")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OUSED", "0")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OSTANDARD", anchor_std)),
                            (mode "IOB33"),
                            (attr "OUSED", ""),
                            (pin "I"),
                            (pin "DIFFI_IN"),
                            (attr "IUSED", "0"),
                            (attr "DIFFI_INUSED", "0"),
                            (attr "ISTANDARD", std.name),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_mode bel_other, "IOB"),
                            (bel_pin bel_other, "PADOUT"),
                            (bel_attr bel_other, "OUSED", ""),
                            (bel_attr bel_other, "PADOUTUSED", "0"),
                            (bel_attr bel_other, "ISTANDARD", std.name)
                        ], [
                            (attr_diff "DIFF_TERM", "FALSE", "TRUE"),
                            (bel_attr_diff bel_other, "DIFF_TERM", "FALSE", "TRUE")
                        ]);
                        fuzz_one!(ctx, "DIFF_TERM_DYNAMIC", std.name, [
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_mode bel_iob1, "IOB33")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_pin bel_iob1, "O")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OUSED", "0")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OSTANDARD", anchor_std)),
                            (mode "IOB33"),
                            (attr "OUSED", ""),
                            (pin "I"),
                            (pin "DIFFI_IN"),
                            (attr "IUSED", "0"),
                            (attr "DIFFI_INUSED", "0"),
                            (attr "ISTANDARD", std.name),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_mode bel_other, "IOB"),
                            (bel_pin bel_other, "PADOUT"),
                            (bel_attr bel_other, "OUSED", ""),
                            (bel_attr bel_other, "PADOUTUSED", "0"),
                            (bel_attr bel_other, "ISTANDARD", std.name)
                        ], [
                            (attr_diff "DIFF_TERM", "FALSE", "TRUE"),
                            (bel_attr_diff bel_other, "DIFF_TERM", "FALSE", "TRUE"),
                            (pin_pips "DIFF_TERM_INT_EN")
                        ]);
                    }
                }
            } else {
                for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                    fuzz_one_extras!(ctx, "ISTD", format!("{sn}.{suffix}", sn=std.name), [
                        (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (related TileRelation::Delta(0, dy, io_pair),
                            (bel_mode bel_iob1, "IOB33")),
                        (related TileRelation::Delta(0, dy, io_pair),
                            (bel_pin bel_iob1, "O")),
                        (related TileRelation::Delta(0, dy, io_pair),
                            (bel_attr bel_iob1, "OUSED", "0")),
                        (related TileRelation::Delta(0, dy, io_pair),
                            (bel_attr bel_iob1, "OSTANDARD", anchor_std)),
                        (mode "IOB33"),
                        (attr "OUSED", ""),
                        (pin "I"),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special vref_special.clone())
                    ], [
                        (attr "IUSED", "0"),
                        (attr "ISTANDARD", std.name),
                        (attr "IBUF_LOW_PWR", lp)
                    ], extras.clone());
                }
            }
        }

        for &std in HR_IOSTDS {
            if bel == "IOB" {
                continue;
            }
            let anchor_std = match std.vcco {
                Some(3300) => "LVCMOS33",
                Some(2500) => "LVCMOS25",
                Some(1800) => "LVCMOS18",
                Some(1500) => "LVCMOS15",
                Some(1200) => "LVCMOS12",
                Some(1350) => "SSTL135",
                _ => unreachable!(),
            };
            if std.diff == DiffKind::True {
                if bel == "IOB1" {
                    let bel_other = bel_other.unwrap();
                    let extras = vec![ExtraFeature::new(
                        ExtraFeatureKind::Hclk(0, 0),
                        "HCLK_IOI_HR",
                        "LVDS",
                        "STD0",
                        std.name,
                    )];
                    fuzz_one_extras!(ctx, "OSTD", std.name, [
                        (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (related TileRelation::Delta(0, dy, io_pair),
                            (bel_mode bel_iob1, "IOB33")),
                        (related TileRelation::Delta(0, dy, io_pair),
                            (bel_pin bel_iob1, "O")),
                        (related TileRelation::Delta(0, dy, io_pair),
                            (bel_attr bel_iob1, "OUSED", "0")),
                        (related TileRelation::Delta(0, dy, io_pair),
                            (bel_attr bel_iob1, "OSTANDARD", anchor_std)),
                        (attr "IUSED", ""),
                        (attr "OPROGRAMMING", ""),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special BelKV::PrepDiffOut),
                        (bel_attr bel_other, "IUSED", ""),
                        (bel_attr bel_other, "OPROGRAMMING", ""),
                        (bel_attr bel_other, "OSTANDARD", ""),
                        (bel_attr bel_other, "OUSED", "")
                    ], [
                        (mode_diff "IOB33", "IOB33M"),
                        (pin "O"),
                        (attr "OUSED", "0"),
                        (attr "DIFFO_OUTUSED", "0"),
                        (attr "OSTANDARD", std.name),
                        (bel_mode_diff bel_other, "IOB33", "IOB33S"),
                        (bel_attr bel_other, "OUTMUX", "1"),
                        (bel_attr bel_other, "DIFFO_INUSED", "0"),
                        (pin_pair "DIFFO_OUT", bel_other, "DIFFO_IN")
                    ], extras);
                    let alt_std = if std.name == "LVDS_25" {
                        "RSDS_25"
                    } else {
                        "LVDS_25"
                    };
                    if std.name != "TMDS_33" {
                        let extras = vec![ExtraFeature::new(
                            ExtraFeatureKind::Hclk(0, 0),
                            "HCLK_IOI_HR",
                            "LVDS",
                            "STD1",
                            std.name,
                        )];
                        fuzz_one_extras!(ctx, "OSTD", format!("{}.ALT", std.name), [
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_mode bel_iob1, "IOB33")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_pin bel_iob1, "O")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OUSED", "0")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OSTANDARD", anchor_std)),

                            (related TileRelation::Delta(0, 4, io_pair),
                                (bel_mode bel_iob1, "IOB33M")),
                            (related TileRelation::Delta(0, 4, io_pair),
                                (bel_pin bel_iob1, "O")),
                            (related TileRelation::Delta(0, 4, io_pair),
                                (bel_attr bel_iob1, "OUSED", "0")),
                            (related TileRelation::Delta(0, 4, io_pair),
                                (bel_attr bel_iob1, "DIFFO_OUTUSED", "0")),
                            (related TileRelation::Delta(0, 4, io_pair),
                                (bel_attr bel_iob1, "OSTANDARD", alt_std)),
                            (related TileRelation::Delta(0, 4, io_pair),
                                (bel_mode bel_iob0, "IOB33S")),
                            (related TileRelation::Delta(0, 4, io_pair),
                                (pin_pair "DIFFO_OUT", bel_iob0, "DIFFO_IN")),
                            (related TileRelation::Delta(0, 4, io_pair),
                                (bel_attr bel_iob0, "OUTMUX", "1")),
                            (related TileRelation::Delta(0, 4, io_pair),
                                (bel_attr bel_iob0, "DIFFO_INUSED", "0")),

                            (attr "IUSED", ""),
                            (attr "OPROGRAMMING", ""),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_special BelKV::PrepDiffOut),
                            (bel_attr bel_other, "IUSED", ""),
                            (bel_attr bel_other, "OPROGRAMMING", ""),
                            (bel_attr bel_other, "OSTANDARD", ""),
                            (bel_attr bel_other, "OUSED", "")
                        ], [
                            (mode_diff "IOB33", "IOB33M"),
                            (pin "O"),
                            (attr "OUSED", "0"),
                            (attr "DIFFO_OUTUSED", "0"),
                            (attr "OSTANDARD", std.name),
                            (bel_mode_diff bel_other, "IOB33", "IOB33S"),
                            (bel_attr bel_other, "OUTMUX", "1"),
                            (bel_attr bel_other, "DIFFO_INUSED", "0"),
                            (pin_pair "DIFFO_OUT", bel_other, "DIFFO_IN")
                        ], extras);
                    }
                }
            } else if std.diff != DiffKind::None {
                if bel == "IOB1" {
                    let bel_other = bel_other.unwrap();
                    let bel_other_ologic = BelId::from_idx(2);
                    let slews = if std.name == "BLVDS_25" {
                        &[""][..]
                    } else {
                        &["SLOW", "FAST"]
                    };
                    for &slew in slews {
                        let val = if slew.is_empty() {
                            std.name.to_string()
                        } else {
                            format!("{name}.{slew}", name = std.name)
                        };
                        fuzz_one!(ctx, "OSTD", val, [
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_mode bel_iob1, "IOB33")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_pin bel_iob1, "O")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OUSED", "0")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OSTANDARD", anchor_std)),
                            (attr "IUSED", ""),
                            (attr "OPROGRAMMING", ""),
                            (package package.name),
                            (bel_special BelKV::IsBonded),
                            (bel_attr bel_other, "IUSED", ""),
                            (bel_attr bel_other, "OPROGRAMMING", ""),
                            (bel_mode bel_other_ologic, "OLOGICE2")
                        ], [
                            (mode_diff "IOB33", "IOB33M"),
                            (pin "O"),
                            (attr "OUSED", "0"),
                            (attr "O_OUTUSED", "0"),
                            (attr "OSTANDARD", std.name),
                            (attr "SLEW", slew),
                            (bel_mode_diff bel_other, "IOB33", "IOB33S"),
                            (bel_attr bel_other, "OUTMUX", "0"),
                            (bel_attr bel_other, "OINMUX", "1"),
                            (bel_attr bel_other, "OSTANDARD", std.name),
                            (bel_attr bel_other, "SLEW", slew),
                            (pin_pair "O_OUT", bel_other, "O_IN")
                        ]);
                    }
                }
            } else {
                let drives = if std.drive.is_empty() {
                    &[""][..]
                } else {
                    std.drive
                };
                let slews = if matches!(std.name, "PCI33_3" | "BLVDS_25") {
                    &[""][..]
                } else {
                    &["SLOW", "FAST"][..]
                };
                for &drive in drives {
                    for slew in slews {
                        let val = if slew.is_empty() {
                            std.name.to_string()
                        } else if drive.is_empty() {
                            format!("{name}.{slew}", name = std.name)
                        } else {
                            format!("{name}.{drive}.{slew}", name = std.name)
                        };
                        fuzz_one!(ctx, "OSTD", val, [
                            (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_mode bel_iob1, "IOB33")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_pin bel_iob1, "O")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OUSED", "0")),
                            (related TileRelation::Delta(0, dy, io_pair),
                                (bel_attr bel_iob1, "OSTANDARD", anchor_std)),
                            (mode "IOB33"),
                            (pin "O"),
                            (attr "IUSED", ""),
                            (attr "OPROGRAMMING", ""),
                            (package package.name),
                            (bel_special BelKV::IsBonded)
                        ], [
                            (attr "OUSED", "0"),
                            (attr "OSTANDARD", std.name),
                            (attr "DRIVE", drive),
                            (attr "SLEW", slew)
                        ]);
                    }
                }
            }
        }

        if bel != "IOB" {
            for (std, vcco, vref) in [
                ("HSUL_12", 1200, 600),
                ("SSTL135", 1350, 675),
                ("HSTL_I", 1500, 750),
                ("HSTL_I_18", 1800, 900),
            ] {
                let extras = vec![ExtraFeature::new(
                    ExtraFeatureKind::Hclk(0, 0),
                    "HCLK_IOI_HR",
                    "INTERNAL_VREF",
                    "VREF",
                    format!("{vref}"),
                )];
                fuzz_one_extras!(ctx, "ISTD", format!("{std}.LP"), [
                    (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                    (related TileRelation::Hclk(hclk_ioi),
                        (tile_mutex "VCCO", vcco.to_string())),
                    (related TileRelation::Delta(0, dy, io_pair),
                        (bel_mode bel_iob1, "IOB33")),
                    (related TileRelation::Delta(0, dy, io_pair),
                        (bel_pin bel_iob1, "O")),
                    (related TileRelation::Delta(0, dy, io_pair),
                        (bel_attr bel_iob1, "OUSED", "0")),
                    (related TileRelation::Delta(0, dy, io_pair),
                        (bel_attr bel_iob1, "OSTANDARD", std)),
                    (mode "IOB33"),
                    (attr "OUSED", ""),
                    (pin "I"),
                    (package package.name),
                    (bel_special BelKV::IsBonded),
                    (bel_special BelKV::PrepVrefInternal(vref))
                ], [
                    (attr "IUSED", "0"),
                    (attr "ISTANDARD", std),
                    (attr "IBUF_LOW_PWR", "TRUE")
                ], extras);
            }
        }

        if tile == "IO_HR_BOT" {
            let extras = vec![
                ExtraFeature::new(
                    ExtraFeatureKind::AllBankIo,
                    "IO_HR_PAIR",
                    "IOB_COMMON",
                    "LOW_VOLTAGE",
                    "1",
                ),
                ExtraFeature::new(
                    ExtraFeatureKind::VrTop,
                    "IO_HR_TOP",
                    "IOB",
                    "LOW_VOLTAGE",
                    "1",
                ),
                ExtraFeature::new(
                    ExtraFeatureKind::Hclk(0, 0),
                    "HCLK_IOI_HR",
                    "DRIVERBIAS",
                    "DRIVERBIAS",
                    "LV",
                ),
            ];
            fuzz_one_extras!(ctx, "OSTD", "LVCMOS18.4.SLOW.EXCL", [
                (global_opt "UNCONSTRAINEDPINS", "ALLOW"),
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "VCCO", "TEST")),
                (mode "IOB33"),
                (pin "O"),
                (attr "IUSED", ""),
                (attr "OPROGRAMMING", ""),
                (package package.name),
                (bel_special BelKV::IsBonded)
            ], [
                (attr "OUSED", "0"),
                (attr "OSTANDARD", "LVCMOS18"),
                (attr "DRIVE", "4"),
                (attr "SLEW", "SLOW")
            ], extras);
        }
    }
    if let Some(mut ctx) = FuzzCtx::try_new(
        session,
        backend,
        "HCLK_IOI_HR",
        "IDELAYCTRL",
        TileBits::Hclk,
    ) {
        ctx.bel_name = "VCCOSENSE".to_string();
        for val in ["OFF", "FREEZE", "ALWAYSACTIVE"] {
            fuzz_one!(ctx, "MODE", val, [], [
                (special TileFuzzKV::VccoSenseMode(val.to_string()))
            ]);
        }
        let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
        let extras = vec![ExtraFeature::new(
            ExtraFeatureKind::AllHclkIo("HCLK_IOI_HR"),
            "HCLK_IOI_HR",
            "VCCOSENSE",
            "FLAG",
            "ENABLE",
        )];
        fuzz_one_extras!(ctx, "VCCOSENSEFLAG", "ENABLE", [], [
            (global_opt "VCCOSENSEFLAG", "ENABLE")
        ], extras);
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
        ctx.tiledb.insert(tile, bel, "OFF_SR_SYNC", item_oq);
        ctx.tiledb.insert(tile, bel, "TFF_SR_SYNC", item_tq);

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
        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("D1", ctx.state.get_diff(tile, bel, "OMUX", "D1")),
            ("SERDES_SDR", diff_sdr),
            ("DDR", diff_ddr),
            ("FF", ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#FF")),
            ("DDR", ctx.state.get_diff(tile, bel, "OUTFFTYPE", "DDR")),
            (
                "LATCH",
                ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#LATCH"),
            ),
        ]);
        ctx.tiledb.insert(tile, bel, "OMUX", item);

        let mut diff_sdr = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "SDR");
        let mut diff_ddr = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "DDR");
        diff_sdr.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_SR_USED"), true, false);
        diff_ddr.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_SR_USED"), true, false);
        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("T1", ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "BUF")),
            ("SERDES_SDR", diff_sdr),
            ("DDR", diff_ddr),
            ("FF", ctx.state.get_diff(tile, bel, "TFFTYPE", "#FF")),
            ("DDR", ctx.state.get_diff(tile, bel, "TFFTYPE", "DDR")),
            ("LATCH", ctx.state.get_diff(tile, bel, "TFFTYPE", "#LATCH")),
        ]);
        ctx.tiledb.insert(tile, bel, "TMUX", item);

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
            let item = xlat_enum(vec![
                ("NONE", Diff::default()),
                (&attrf[4..], diff_f),
                (
                    "PHASER_OCLKDIV",
                    ctx.state.get_diff(tile, bel, attr, "PHASER_OCLKDIV"),
                ),
            ]);
            ctx.tiledb.insert(tile, bel, attr, item);
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
        let item = xlat_enum(vec![
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
        ]);
        ctx.tiledb.insert(tile, bel, "DELAY_SRC", item);

        let item = xlat_enum(vec![
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
        ]);
        ctx.tiledb.insert(tile, bel, "IDELAY_TYPE", item);
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

        let item = xlat_enum(vec![
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
        ]);
        ctx.tiledb.insert(tile, bel, "DELAY_SRC", item);

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
    for (tile, bel) in [
        ("IO_HP_PAIR", "IOB0"),
        ("IO_HP_PAIR", "IOB1"),
        ("IO_HP_BOT", "IOB"),
        ("IO_HP_TOP", "IOB"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
        ctx.collect_enum(tile, bel, "DCITERMDISABLE_SEL", &["I", "GND"]);
        ctx.collect_enum(tile, bel, "IBUFDISABLE_SEL", &["I", "GND"]);
        ctx.collect_bit(tile, bel, "PULL_DYNAMIC", "1");
        ctx.collect_enum_bool(tile, bel, "OUTPUT_DELAY", "0", "1");
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "IOB");
        if bel != "IOB" {
            let diff = ctx
                .state
                .get_diff(tile, bel, "PRESENT", "IPAD")
                .combine(&!&present);
            ctx.tiledb.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
        }
        if bel == "IOB0" {
            let diff = ctx
                .state
                .get_diff(tile, bel, "PRESENT", "VREF")
                .combine(&!&present);
            ctx.tiledb.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
        }
        let diff = ctx
            .state
            .get_diff(tile, bel, "PRESENT", "IOB.QUIET")
            .combine(&!&present);
        ctx.tiledb
            .insert(tile, bel, "DCIUPDATEMODE_QUIET", xlat_bit(diff));
        present.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

        let iprog = ctx.state.get_diffs(tile, bel, "IPROGRAMMING", "");
        ctx.tiledb
            .insert(tile, bel, "INPUT_MISC", xlat_bit(iprog[19].clone()));

        let oprog = ctx.extract_bitvec(tile, bel, "OPROGRAMMING", "");
        let lvds = TileItem::from_bitvec(oprog.bits[0..9].to_vec(), false);
        let mut om_bits = oprog.bits[9..14].to_vec();
        om_bits.push(oprog.bits[19]);
        let output_misc = TileItem::from_bitvec(om_bits, false);
        let dci_t = TileItem::from_bit(oprog.bits[14], false);
        let dqsbias_n = TileItem::from_bit(oprog.bits[17], false);
        let dqsbias_p = TileItem::from_bit(oprog.bits[18], false);
        let dci_mode = TileItem {
            bits: oprog.bits[15..17].to_vec(),
            kind: TileItemKind::Enum {
                values: [
                    ("NONE".into(), bitvec![0, 0]),
                    ("OUTPUT".into(), bitvec![1, 0]),
                    ("OUTPUT_HALF".into(), bitvec![0, 1]),
                    ("TERM_SPLIT".into(), bitvec![1, 1]),
                ]
                .into_iter()
                .collect(),
            },
        };
        let pdrive_bits = oprog.bits[20..27].to_vec();
        let ndrive_bits = oprog.bits[27..34].to_vec();
        let pdrive_invert: BitVec = pdrive_bits
            .iter()
            .map(|&bit| match present.bits.remove(&bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            })
            .collect();
        let ndrive_invert: BitVec = ndrive_bits
            .iter()
            .map(|&bit| match present.bits.remove(&bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            })
            .collect();
        let tidx = if bel == "IOB1" { 1 } else { 0 };

        let (pslew_bits, nslew_bits) = if bel == "IOB0" || tile == "IO_HP_TOP" {
            (
                vec![
                    TileBit::new(tidx, 38, 50),
                    TileBit::new(tidx, 38, 30),
                    TileBit::new(tidx, 38, 26),
                    TileBit::new(tidx, 38, 16),
                    TileBit::new(tidx, 39, 13),
                ],
                vec![
                    TileBit::new(tidx, 38, 46),
                    TileBit::new(tidx, 39, 45),
                    TileBit::new(tidx, 38, 38),
                    TileBit::new(tidx, 38, 22),
                    TileBit::new(tidx, 38, 14),
                ],
            )
        } else {
            (
                vec![
                    TileBit::new(tidx, 39, 13),
                    TileBit::new(tidx, 39, 33),
                    TileBit::new(tidx, 39, 37),
                    TileBit::new(tidx, 39, 47),
                    TileBit::new(tidx, 38, 50),
                ],
                vec![
                    TileBit::new(tidx, 39, 17),
                    TileBit::new(tidx, 38, 18),
                    TileBit::new(tidx, 39, 25),
                    TileBit::new(tidx, 39, 41),
                    TileBit::new(tidx, 39, 49),
                ],
            )
        };
        let pslew = TileItem::from_bitvec(pslew_bits, false);
        let nslew = TileItem::from_bitvec(nslew_bits, false);

        let mut diff = ctx
            .state
            .peek_diff(tile, bel, "OSTD", "HSTL_I.FAST")
            .combine(&present);
        for &bit in &pdrive_bits {
            diff.bits.remove(&bit);
        }
        for &bit in &ndrive_bits {
            diff.bits.remove(&bit);
        }
        extract_bitvec_val_part(&pslew, &bitvec![0; 5], &mut diff);
        extract_bitvec_val_part(&nslew, &bitvec![0; 5], &mut diff);
        ctx.tiledb
            .insert(tile, bel, "OUTPUT_ENABLE", xlat_bit_wide(diff));

        let diff_cmos = ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS18.LP");
        let diff_vref_lp = ctx.state.peek_diff(tile, bel, "ISTD", "HSTL_I.LP");
        let diff_vref_hp = ctx.state.peek_diff(tile, bel, "ISTD", "HSTL_I.HP");
        let mut diffs = vec![
            ("OFF", Diff::default()),
            ("CMOS", diff_cmos.clone()),
            ("VREF_LP", diff_vref_lp.clone()),
            ("VREF_HP", diff_vref_hp.clone()),
        ];
        if bel != "IOB" {
            let mut diff_diff_lp = ctx.state.peek_diff(tile, bel, "ISTD", "LVDS.LP").clone();
            let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.tile == tidx);
            let mut diff_diff_hp = ctx.state.peek_diff(tile, bel, "ISTD", "LVDS.HP").clone();
            let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.tile == tidx);
            diffs.extend([("DIFF_LP", diff_diff_lp), ("DIFF_HP", diff_diff_hp)]);
        }
        ctx.tiledb.insert(tile, bel, "IBUF_MODE", xlat_enum(diffs));

        for &std in HP_IOSTDS {
            if bel == "IOB" && std.name != "HSTL_I" {
                continue;
            }
            if std.diff != DiffKind::None {
                continue;
            }
            let drives = if !std.drive.is_empty() {
                std.drive
            } else {
                &[""][..]
            };
            let slews = if std.name.contains("LVDCI") {
                &[""][..]
            } else {
                &["SLOW", "FAST"]
            };
            for &drive in drives {
                for &slew in slews {
                    let val = if slew.is_empty() {
                        std.name.to_string()
                    } else if drive.is_empty() {
                        format!("{name}.{slew}", name = std.name)
                    } else {
                        format!("{name}.{drive}.{slew}", name = std.name)
                    };
                    let mut diff = ctx.state.get_diff(tile, bel, "OSTD", &val);
                    diff.apply_bitvec_diff(
                        ctx.tiledb.item(tile, bel, "OUTPUT_ENABLE"),
                        &bitvec![1; 2],
                        &bitvec![0; 2],
                    );
                    let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                    if !matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                        for (attr, bits, invert) in [
                            ("PDRIVE", &pdrive_bits, &pdrive_invert),
                            ("NDRIVE", &ndrive_bits, &ndrive_invert),
                        ] {
                            let value: BitVec = bits
                                .iter()
                                .zip(invert.iter())
                                .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                    Some(val) => {
                                        assert_eq!(val, !*inv);
                                        true
                                    }
                                    None => false,
                                })
                                .collect();
                            let name = if drive.is_empty() {
                                stdname.to_string()
                            } else {
                                format!("{stdname}.{drive}")
                            };
                            ctx.tiledb
                                .insert_misc_data(format!("HP_IOSTD:{attr}:{name}"), value);
                        }
                    }
                    for (attr, item) in [("PSLEW", &pslew), ("NSLEW", &nslew)] {
                        let value: BitVec = item
                            .bits
                            .iter()
                            .map(|&bit| match diff.bits.remove(&bit) {
                                Some(true) => true,
                                None => false,
                                _ => unreachable!(),
                            })
                            .collect();
                        let name = if slew.is_empty() {
                            stdname.to_string()
                        } else if drive.is_empty() {
                            format!("{stdname}.{slew}")
                        } else {
                            format!("{stdname}.{drive}.{slew}")
                        };
                        ctx.tiledb
                            .insert_misc_data(format!("HP_IOSTD:{attr}:{name}"), value);
                    }
                    match std.dci {
                        DciKind::None | DciKind::InputSplit => {}
                        DciKind::Output => {
                            diff.apply_enum_diff(&dci_mode, "OUTPUT", "NONE");
                        }
                        DciKind::OutputHalf => {
                            diff.apply_enum_diff(&dci_mode, "OUTPUT_HALF", "NONE");
                        }
                        DciKind::BiSplit => {
                            diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                        }
                        DciKind::BiSplitT => {
                            diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                            diff.apply_bit_diff(&dci_t, true, false);
                            diff.apply_enum_diff(
                                ctx.tiledb.item(tile, bel, "IBUF_MODE"),
                                "VREF_LP",
                                "OFF",
                            );
                        }
                        _ => unreachable!(),
                    }
                    diff.assert_empty();
                }
            }
        }
        for &std in HP_IOSTDS {
            if bel == "IOB" && !matches!(std.name, "LVCMOS18" | "HSTL_I") {
                continue;
            }
            if std.diff != DiffKind::None {
                continue;
            }
            if std.dci == DciKind::BiSplitT {
                continue;
            }
            for lp in ["HP", "LP"] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                match std.dci {
                    DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                    DciKind::InputSplit | DciKind::BiSplit => {
                        diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                    }
                    _ => unreachable!(),
                }
                let mode = if std.vref.is_some() {
                    if lp == "LP" {
                        "VREF_LP"
                    } else {
                        "VREF_HP"
                    }
                } else {
                    "CMOS"
                };
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IBUF_MODE"), mode, "OFF");
                diff.assert_empty();
            }
        }

        if bel == "IOB" {
            let mut present_vr = ctx.state.get_diff(tile, bel, "PRESENT", "VR");
            present_vr.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");
            present_vr.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
            for (attr, bits, invert) in [
                ("PDRIVE", &pdrive_bits, &pdrive_invert),
                ("NDRIVE", &ndrive_bits, &ndrive_invert),
                ("PSLEW", &pslew.bits, &bitvec![0; 5]),
                ("NSLEW", &nslew.bits, &bitvec![0; 5]),
            ] {
                let value: BitVec = bits
                    .iter()
                    .zip(invert.iter())
                    .map(|(&bit, inv)| match present_vr.bits.remove(&bit) {
                        Some(true) => !*inv,
                        None => *inv,
                        _ => unreachable!(),
                    })
                    .collect();
                if attr.contains("DRIVE") {
                    assert!(!value.any());
                } else {
                    ctx.tiledb
                        .insert_misc_data(format!("HP_IOSTD:{attr}:VR"), value);
                }
            }
            ctx.tiledb.insert(tile, bel, "VR", xlat_bit(present_vr));
        }

        ctx.tiledb
            .insert_misc_data("HP_IOSTD:LVDS_T:OFF", bitvec![0; 9]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:LVDS_C:OFF", bitvec![0; 9]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:PDRIVE:OFF", bitvec![0; 7]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:NDRIVE:OFF", bitvec![0; 7]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:PSLEW:OFF", bitvec![0; 5]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:NSLEW:OFF", bitvec![0; 5]);
        ctx.tiledb.insert(tile, bel, "LVDS", lvds);
        ctx.tiledb.insert(tile, bel, "DCI_T", dci_t);
        ctx.tiledb.insert(tile, bel, "DQS_BIAS_N", dqsbias_n);
        ctx.tiledb.insert(tile, bel, "DQS_BIAS_P", dqsbias_p);
        ctx.tiledb.insert(tile, bel, "DCI_MODE", dci_mode);
        ctx.tiledb.insert(tile, bel, "OUTPUT_MISC", output_misc);
        ctx.tiledb.insert(
            tile,
            bel,
            "PDRIVE",
            TileItem {
                bits: pdrive_bits,
                kind: TileItemKind::BitVec {
                    invert: pdrive_invert,
                },
            },
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "NDRIVE",
            TileItem {
                bits: ndrive_bits,
                kind: TileItemKind::BitVec {
                    invert: ndrive_invert,
                },
            },
        );
        ctx.tiledb.insert(tile, bel, "PSLEW", pslew);
        ctx.tiledb.insert(tile, bel, "NSLEW", nslew);

        present.assert_empty();
    }

    if ctx.has_tile("IO_HP_PAIR") {
        let tile = "IO_HP_PAIR";
        for &std in HP_IOSTDS {
            if std.diff == DiffKind::None {
                continue;
            }
            for bel in ["IOB0", "IOB1"] {
                for lp in ["HP", "LP"] {
                    if std.dci == DciKind::BiSplitT {
                        continue;
                    }
                    let mut diff =
                        ctx.state
                            .get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                    for cbel in ["IOB0", "IOB1"] {
                        match std.dci {
                            DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                            DciKind::InputSplit | DciKind::BiSplit => {
                                diff.apply_enum_diff(
                                    ctx.tiledb.item(tile, cbel, "DCI_MODE"),
                                    "TERM_SPLIT",
                                    "NONE",
                                );
                            }
                            _ => unreachable!(),
                        }
                        diff.apply_enum_diff(
                            ctx.tiledb.item(tile, cbel, "IBUF_MODE"),
                            if lp == "LP" { "DIFF_LP" } else { "DIFF_HP" },
                            "OFF",
                        );
                    }
                    diff.assert_empty();
                }
            }
            if std.diff == DiffKind::True {
                let mut diff = ctx.state.get_diff(tile, "IOB0", "DIFF_TERM", std.name);
                let val_c = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB0", "LVDS"),
                    &bitvec![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB1", "LVDS"),
                    &bitvec![0; 9],
                    &mut diff,
                );
                ctx.tiledb
                    .insert_misc_data(format!("HP_IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                ctx.tiledb
                    .insert_misc_data(format!("HP_IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                diff.assert_empty();

                let mut diff = ctx
                    .state
                    .get_diff(tile, "IOB0", "DIFF_TERM_DYNAMIC", std.name);
                let val_c = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB0", "LVDS"),
                    &bitvec![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB1", "LVDS"),
                    &bitvec![0; 9],
                    &mut diff,
                );
                ctx.tiledb
                    .insert_misc_data(format!("HP_IOSTD:LVDS_T:TERM_DYNAMIC_{}", std.name), val_t);
                ctx.tiledb
                    .insert_misc_data(format!("HP_IOSTD:LVDS_C:TERM_DYNAMIC_{}", std.name), val_c);
                diff.assert_empty();

                let mut diff = ctx.state.get_diff(tile, "IOB1", "OSTD", std.name);
                let val_c = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB0", "LVDS"),
                    &bitvec![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB1", "LVDS"),
                    &bitvec![0; 9],
                    &mut diff,
                );
                ctx.tiledb
                    .insert_misc_data(format!("HP_IOSTD:LVDS_T:OUTPUT_{}", std.name), val_t);
                ctx.tiledb
                    .insert_misc_data(format!("HP_IOSTD:LVDS_C:OUTPUT_{}", std.name), val_c);
                diff.apply_bitvec_diff(
                    ctx.tiledb.item(tile, "IOB1", "OUTPUT_ENABLE"),
                    &bitvec![1; 2],
                    &bitvec![0; 2],
                );
                diff.assert_empty();
            }
            if std.diff == DiffKind::Pseudo {
                for slew in ["SLOW", "FAST"] {
                    let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                    let mut diff = ctx.state.get_diff(
                        tile,
                        "IOB1",
                        "OSTD",
                        format!("{sn}.{slew}", sn = std.name),
                    );
                    for bel in ["IOB0", "IOB1"] {
                        diff.apply_bitvec_diff(
                            ctx.tiledb.item(tile, bel, "OUTPUT_ENABLE"),
                            &bitvec![1; 2],
                            &bitvec![0; 2],
                        );
                        if !matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                            for attr in ["PDRIVE", "NDRIVE"] {
                                let item = ctx.tiledb.item(tile, bel, attr);
                                let value = extract_bitvec_val_part(
                                    item,
                                    &BitVec::repeat(false, item.bits.len()),
                                    &mut diff,
                                );
                                ctx.tiledb
                                    .insert_misc_data(format!("HP_IOSTD:{attr}:{stdname}"), value);
                            }
                        }
                        for attr in ["PSLEW", "NSLEW"] {
                            let item = ctx.tiledb.item(tile, bel, attr);
                            let value = extract_bitvec_val_part(
                                item,
                                &BitVec::repeat(false, item.bits.len()),
                                &mut diff,
                            );
                            ctx.tiledb.insert_misc_data(
                                format!("HP_IOSTD:{attr}:{stdname}.{slew}"),
                                value,
                            );
                        }
                        let dci_mode = ctx.tiledb.item(tile, bel, "DCI_MODE");
                        let dci_t = ctx.tiledb.item(tile, bel, "DCI_T");
                        match std.dci {
                            DciKind::None | DciKind::InputSplit => {}
                            DciKind::Output => {
                                diff.apply_enum_diff(dci_mode, "OUTPUT", "NONE");
                            }
                            DciKind::OutputHalf => {
                                diff.apply_enum_diff(dci_mode, "OUTPUT_HALF", "NONE");
                            }
                            DciKind::BiSplit => {
                                diff.apply_enum_diff(dci_mode, "TERM_SPLIT", "NONE");
                            }
                            DciKind::BiSplitT => {
                                diff.apply_enum_diff(dci_mode, "TERM_SPLIT", "NONE");
                                diff.apply_bit_diff(dci_t, true, false);
                            }
                            _ => unreachable!(),
                        }
                    }
                    let diff_t = diff.split_bits_by(|bit| bit.bit == 17);
                    assert_eq!(diff.bits.len(), 1);
                    assert_eq!(diff_t.bits.len(), 1);
                    ctx.tiledb.insert(
                        tile,
                        "IOB0",
                        "OMUX",
                        xlat_enum(vec![("O", Diff::default()), ("OTHER_O_INV", diff)]),
                    );
                    ctx.tiledb.insert(
                        tile,
                        "IOB0",
                        "TMUX",
                        xlat_enum(vec![("T", Diff::default()), ("OTHER_T", diff_t)]),
                    );
                }
            }
        }
    }

    if ctx.has_tile("HCLK_IOI_HP") {
        let tile = "HCLK_IOI_HP";
        let lvdsbias = TileItem::from_bitvec(
            vec![
                TileBit::new(0, 41, 14),
                TileBit::new(0, 41, 15),
                TileBit::new(0, 41, 16),
                TileBit::new(0, 41, 17),
                TileBit::new(0, 41, 18),
                TileBit::new(0, 41, 19),
                TileBit::new(0, 41, 20),
                TileBit::new(0, 41, 21),
                TileBit::new(0, 41, 22),
                TileBit::new(0, 41, 23),
                TileBit::new(0, 41, 24),
                TileBit::new(0, 41, 25),
                TileBit::new(0, 41, 26),
                TileBit::new(0, 41, 27),
                TileBit::new(0, 41, 28),
                TileBit::new(0, 41, 29),
                TileBit::new(0, 41, 30),
                TileBit::new(0, 40, 31),
            ],
            false,
        );
        let nref_output = TileItem::from_bitvec(
            vec![TileBit::new(0, 39, 30), TileBit::new(0, 39, 29)],
            false,
        );
        let pref_output = TileItem::from_bitvec(
            vec![TileBit::new(0, 40, 18), TileBit::new(0, 40, 17)],
            false,
        );
        let nref_output_half = TileItem::from_bitvec(
            vec![
                TileBit::new(0, 39, 28),
                TileBit::new(0, 39, 27),
                TileBit::new(0, 39, 26),
            ],
            false,
        );
        let pref_output_half = TileItem::from_bitvec(
            vec![
                TileBit::new(0, 40, 16),
                TileBit::new(0, 40, 15),
                TileBit::new(0, 40, 14),
            ],
            false,
        );
        let nref_term_split = TileItem::from_bitvec(
            vec![
                TileBit::new(0, 39, 25),
                TileBit::new(0, 39, 24),
                TileBit::new(0, 39, 23),
            ],
            false,
        );

        for std in HP_IOSTDS {
            if std.diff == DiffKind::True {
                let bel = "LVDS";
                let diff = ctx.state.get_diff(tile, bel, "STD", std.name);
                let val = extract_bitvec_val(&lvdsbias, &bitvec![0; 18], diff);
                ctx.tiledb
                    .insert_misc_data(format!("HP_IOSTD:LVDSBIAS:{}", std.name), val);
            }
            if std.dci != DciKind::None {
                let bel = "DCI";
                let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                let mut diff = ctx.state.get_diff(tile, bel, "STD", std.name);
                match std.dci {
                    DciKind::Output => {
                        let val = extract_bitvec_val_part(&nref_output, &bitvec![0; 2], &mut diff);
                        ctx.tiledb
                            .insert_misc_data(format!("HP_IOSTD:DCI:NREF_OUTPUT:{stdname}"), val);
                        let val = extract_bitvec_val_part(&pref_output, &bitvec![0; 2], &mut diff);
                        ctx.tiledb
                            .insert_misc_data(format!("HP_IOSTD:DCI:PREF_OUTPUT:{stdname}"), val);
                    }
                    DciKind::OutputHalf => {
                        let val =
                            extract_bitvec_val_part(&nref_output_half, &bitvec![0; 3], &mut diff);
                        ctx.tiledb.insert_misc_data(
                            format!("HP_IOSTD:DCI:NREF_OUTPUT_HALF:{stdname}"),
                            val,
                        );
                        let val =
                            extract_bitvec_val_part(&pref_output_half, &bitvec![0; 3], &mut diff);
                        ctx.tiledb.insert_misc_data(
                            format!("HP_IOSTD:DCI:PREF_OUTPUT_HALF:{stdname}"),
                            val,
                        );
                    }
                    DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                        let val =
                            extract_bitvec_val_part(&nref_term_split, &bitvec![0; 3], &mut diff);
                        ctx.tiledb.insert_misc_data(
                            format!("HP_IOSTD:DCI:NREF_TERM_SPLIT:{stdname}"),
                            val,
                        );
                    }
                    _ => {}
                }
                ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff));
            }
        }
        let bel = "LVDS";
        ctx.tiledb.insert(tile, bel, "LVDSBIAS", lvdsbias);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:LVDSBIAS:OFF", bitvec![0; 18]);
        let bel = "DCI";
        ctx.tiledb.insert(tile, bel, "PREF_OUTPUT", pref_output);
        ctx.tiledb.insert(tile, bel, "NREF_OUTPUT", nref_output);
        ctx.tiledb
            .insert(tile, bel, "PREF_OUTPUT_HALF", pref_output_half);
        ctx.tiledb
            .insert(tile, bel, "NREF_OUTPUT_HALF", nref_output_half);
        ctx.tiledb
            .insert(tile, bel, "NREF_TERM_SPLIT", nref_term_split);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:DCI:PREF_OUTPUT:OFF", bitvec![0; 2]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:DCI:NREF_OUTPUT:OFF", bitvec![0; 2]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:DCI:PREF_OUTPUT_HALF:OFF", bitvec![0; 3]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:DCI:NREF_OUTPUT_HALF:OFF", bitvec![0; 3]);
        ctx.tiledb
            .insert_misc_data("HP_IOSTD:DCI:NREF_TERM_SPLIT:OFF", bitvec![0; 3]);

        let dci_en = ctx.state.get_diff(tile, bel, "ENABLE", "1");
        let test_en = ctx.state.get_diff(tile, bel, "TEST_ENABLE", "1");
        let quiet = ctx
            .state
            .get_diff(tile, bel, "TEST_ENABLE", "QUIET")
            .combine(&!&test_en);
        ctx.tiledb.insert(tile, bel, "QUIET", xlat_bit(quiet));
        let test_en = test_en.combine(&!&dci_en);
        let dyn_en = ctx
            .state
            .get_diff(tile, bel, "DYNAMIC_ENABLE", "1")
            .combine(&!&dci_en);
        ctx.tiledb
            .insert(tile, bel, "TEST_ENABLE", xlat_bit_wide(test_en));
        ctx.tiledb
            .insert(tile, bel, "DYNAMIC_ENABLE", xlat_bit(dyn_en));
        let casc_from_above = ctx
            .state
            .get_diff(tile, bel, "CASCADE_FROM_ABOVE", "1")
            .combine(&!&dci_en);
        ctx.tiledb.insert(
            tile,
            bel,
            "CASCADE_FROM_ABOVE",
            xlat_bit_wide(casc_from_above),
        );
        let casc_from_below = ctx
            .state
            .get_diff(tile, bel, "CASCADE_FROM_BELOW", "1")
            .combine(&!&dci_en);
        ctx.tiledb.insert(
            tile,
            bel,
            "CASCADE_FROM_BELOW",
            xlat_bit_wide(casc_from_below),
        );
        ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(dci_en));

        let mut diffs = vec![("OFF", Diff::default())];
        for val in ["600", "675", "750", "900"] {
            diffs.push((val, ctx.state.get_diff(tile, "INTERNAL_VREF", "VREF", val)));
        }
        // cannot be dealt with normally as there are no standards with such VREF.
        diffs.push((
            "1100",
            Diff {
                bits: [TileBit::new(0, 40, 19), TileBit::new(0, 40, 24)]
                    .into_iter()
                    .map(|x| (x, true))
                    .collect(),
            },
        ));
        diffs.push((
            "1250",
            Diff {
                bits: [TileBit::new(0, 40, 19), TileBit::new(0, 40, 23)]
                    .into_iter()
                    .map(|x| (x, true))
                    .collect(),
            },
        ));
        ctx.tiledb
            .insert(tile, "INTERNAL_VREF", "VREF", xlat_enum(diffs));

        ctx.collect_bit_wide("CFG", "MISC", "DCI_CLK_ENABLE", "1");
    }
    for (tile, bel) in [
        ("IO_HR_PAIR", "IOB0"),
        ("IO_HR_PAIR", "IOB1"),
        ("IO_HR_BOT", "IOB"),
        ("IO_HR_TOP", "IOB"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }

        ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
        ctx.collect_enum(tile, bel, "INTERMDISABLE_SEL", &["I", "GND"]);
        ctx.collect_enum(tile, bel, "IBUFDISABLE_SEL", &["I", "GND"]);
        ctx.collect_bit(tile, bel, "PULL_DYNAMIC", "1");
        ctx.collect_enum_bool(tile, bel, "DQS_BIAS", "FALSE", "TRUE");
        ctx.collect_enum(
            tile,
            bel,
            "IN_TERM",
            &[
                "NONE",
                "UNTUNED_SPLIT_40",
                "UNTUNED_SPLIT_50",
                "UNTUNED_SPLIT_60",
            ],
        );

        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "IOB");
        if bel != "IOB" {
            let diff = ctx
                .state
                .get_diff(tile, bel, "PRESENT", "IPAD")
                .combine(&!&present);
            ctx.tiledb.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
        }
        if bel == "IOB0" {
            let diff = ctx
                .state
                .get_diff(tile, bel, "PRESENT", "VREF")
                .combine(&!&present);
            ctx.tiledb.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
        }
        present.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

        let tidx = if bel == "IOB1" { 1 } else { 0 };
        let diff_cmos_lv = ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS18.LP");
        let diff_cmos_hv = ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS33.LP");
        let diff_vref_lp = ctx.state.peek_diff(tile, bel, "ISTD", "HSTL_I.LP");
        let diff_vref_hp = ctx.state.peek_diff(tile, bel, "ISTD", "HSTL_I.HP");
        let diff_pci = ctx.state.peek_diff(tile, bel, "ISTD", "PCI33_3.LP");
        let mut diffs = vec![
            ("OFF", Diff::default()),
            ("VREF_LP", diff_vref_lp.clone()),
            ("CMOS_LV", diff_cmos_lv.clone()),
            ("CMOS_HV", diff_cmos_hv.clone()),
            ("PCI", diff_pci.clone()),
            ("VREF_HP", diff_vref_hp.clone()),
        ];
        if bel != "IOB" {
            let mut diff_diff_lp = ctx.state.peek_diff(tile, bel, "ISTD", "LVDS_25.LP").clone();
            let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.tile == tidx);
            let mut diff_diff_hp = ctx.state.peek_diff(tile, bel, "ISTD", "LVDS_25.HP").clone();
            let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.tile == tidx);
            let mut diff_tmds_lp = ctx.state.peek_diff(tile, bel, "ISTD", "TMDS_33.LP").clone();
            let diff_tmds_lp = diff_tmds_lp.split_bits_by(|bit| bit.tile == tidx);
            let mut diff_tmds_hp = ctx.state.peek_diff(tile, bel, "ISTD", "TMDS_33.HP").clone();
            let diff_tmds_hp = diff_tmds_hp.split_bits_by(|bit| bit.tile == tidx);
            diffs.extend([
                ("DIFF_LP", diff_diff_lp),
                ("DIFF_HP", diff_diff_hp),
                ("TMDS_LP", diff_tmds_lp),
                ("TMDS_HP", diff_tmds_hp),
            ]);
        }
        ctx.tiledb.insert(tile, bel, "IBUF_MODE", xlat_enum(diffs));

        let iprog = ctx.state.get_diffs(tile, bel, "IPROGRAMMING", "");
        ctx.tiledb
            .insert(tile, bel, "INPUT_MISC", xlat_bit(iprog[7].clone()));

        for &std in HR_IOSTDS {
            if bel == "IOB" && !matches!(std.name, "LVCMOS18" | "LVCMOS33" | "PCI33_3" | "HSTL_I") {
                continue;
            }
            if std.diff != DiffKind::None {
                continue;
            }
            for lp in ["HP", "LP"] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                let mode = if std.vref.is_some() {
                    if lp == "LP" {
                        "VREF_LP"
                    } else {
                        "VREF_HP"
                    }
                } else if std.name == "PCI33_3" {
                    "PCI"
                } else if std.vcco.unwrap() < 2500 {
                    "CMOS_LV"
                } else {
                    "CMOS_HV"
                };
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IBUF_MODE"), mode, "OFF");
                diff.assert_empty();
            }
        }

        let mut oprog = ctx.state.get_diffs(tile, bel, "OPROGRAMMING", "");
        ctx.tiledb
            .insert(tile, bel, "OUTPUT_ENABLE", xlat_bitvec(oprog.split_off(37)));
        if bel == "IOB0" {
            ctx.tiledb.insert(
                tile,
                bel,
                "OMUX",
                xlat_enum(vec![
                    ("O", Diff::default()),
                    ("OTHER_O_INV", oprog.pop().unwrap()),
                ]),
            );
        } else {
            oprog.pop().unwrap().assert_empty();
        }
        ctx.tiledb
            .insert(tile, bel, "OUTPUT_MISC_B", xlat_bit(oprog.pop().unwrap()));
        ctx.tiledb
            .insert(tile, bel, "LOW_VOLTAGE", xlat_bit(oprog.pop().unwrap()));
        let slew_bits = xlat_bitvec(oprog.split_off(24)).bits;
        ctx.tiledb
            .insert(tile, bel, "OUTPUT_MISC", xlat_bitvec(oprog.split_off(21)));
        let drive_bits = xlat_bitvec(oprog.split_off(14)).bits;
        oprog.pop().unwrap().assert_empty();
        ctx.tiledb.insert(tile, bel, "LVDS", xlat_bitvec(oprog));
        let drive_invert: BitVec = drive_bits
            .iter()
            .map(|&bit| match present.bits.remove(&bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            })
            .collect();
        let slew_invert: BitVec = slew_bits
            .iter()
            .map(|&bit| match present.bits.remove(&bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            })
            .collect();
        ctx.tiledb.insert(
            tile,
            bel,
            "DRIVE",
            TileItem {
                bits: drive_bits,
                kind: TileItemKind::BitVec {
                    invert: drive_invert,
                },
            },
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "SLEW",
            TileItem {
                bits: slew_bits,
                kind: TileItemKind::BitVec {
                    invert: slew_invert,
                },
            },
        );
        present.assert_empty();

        ctx.tiledb
            .insert_misc_data("HR_IOSTD:LVDS_T:OFF", bitvec![0; 13]);
        ctx.tiledb
            .insert_misc_data("HR_IOSTD:LVDS_C:OFF", bitvec![0; 13]);
        ctx.tiledb
            .insert_misc_data("HR_IOSTD:DRIVE:OFF", bitvec![0; 7]);
        ctx.tiledb
            .insert_misc_data("HR_IOSTD:OUTPUT_MISC:OFF", bitvec![0; 3]);
        ctx.tiledb
            .insert_misc_data("HR_IOSTD:SLEW:OFF", bitvec![0; 10]);

        if bel != "IOB" {
            for std in HR_IOSTDS {
                if std.diff != DiffKind::None {
                    continue;
                }
                let drives = if !std.drive.is_empty() {
                    std.drive
                } else {
                    &[""][..]
                };
                let slews = if std.name == "PCI33_3" {
                    &[""][..]
                } else {
                    &["SLOW", "FAST"]
                };
                for &drive in drives {
                    for &slew in slews {
                        let val = if slew.is_empty() {
                            std.name.to_string()
                        } else if drive.is_empty() {
                            format!("{name}.{slew}", name = std.name)
                        } else {
                            format!("{name}.{drive}.{slew}", name = std.name)
                        };
                        let mut diff = ctx.state.get_diff(tile, bel, "OSTD", &val);
                        diff.apply_bitvec_diff(
                            ctx.tiledb.item(tile, bel, "OUTPUT_ENABLE"),
                            &bitvec![1; 2],
                            &bitvec![0; 2],
                        );
                        let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                        let drive_item = ctx.tiledb.item(tile, bel, "DRIVE");
                        let TileItemKind::BitVec { ref invert } = drive_item.kind else {
                            unreachable!()
                        };
                        let value: BitVec = drive_item
                            .bits
                            .iter()
                            .zip(invert.iter())
                            .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                Some(val) => {
                                    assert_eq!(val, !*inv);
                                    true
                                }
                                None => false,
                            })
                            .collect();
                        let name = if drive.is_empty() {
                            stdname.to_string()
                        } else {
                            format!("{stdname}.{drive}")
                        };
                        ctx.tiledb
                            .insert_misc_data(format!("HR_IOSTD:DRIVE:{name}"), value);
                        let slew_item = ctx.tiledb.item(tile, bel, "SLEW");
                        let TileItemKind::BitVec { ref invert } = slew_item.kind else {
                            unreachable!()
                        };
                        let value: BitVec = slew_item
                            .bits
                            .iter()
                            .zip(invert.iter())
                            .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                Some(val) => {
                                    assert_eq!(val, !*inv);
                                    true
                                }
                                None => false,
                            })
                            .collect();
                        let name = if slew.is_empty() {
                            stdname.to_string()
                        } else if drive.is_empty() {
                            format!("{stdname}.{slew}")
                        } else {
                            format!("{stdname}.{drive}.{slew}")
                        };
                        ctx.tiledb
                            .insert_misc_data(format!("HR_IOSTD:SLEW:{name}"), value);
                        let val = extract_bitvec_val(
                            ctx.tiledb.item(tile, bel, "OUTPUT_MISC"),
                            &bitvec![0; 3],
                            diff,
                        );
                        ctx.tiledb
                            .insert_misc_data(format!("HR_IOSTD:OUTPUT_MISC:{stdname}"), val);
                    }
                }
            }
        }
    }

    if ctx.has_tile("IO_HR_PAIR") {
        let tile = "IO_HR_PAIR";
        for &std in HR_IOSTDS {
            if std.diff == DiffKind::None {
                continue;
            }
            for bel in ["IOB0", "IOB1"] {
                for lp in ["HP", "LP"] {
                    let mut diff =
                        ctx.state
                            .get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                    for cbel in ["IOB0", "IOB1"] {
                        diff.apply_enum_diff(
                            ctx.tiledb.item(tile, cbel, "IBUF_MODE"),
                            if std.name == "TMDS_33" {
                                if lp == "LP" {
                                    "TMDS_LP"
                                } else {
                                    "TMDS_HP"
                                }
                            } else {
                                if lp == "LP" {
                                    "DIFF_LP"
                                } else {
                                    "DIFF_HP"
                                }
                            },
                            "OFF",
                        );
                    }
                    diff.assert_empty();
                }
            }
            if std.diff == DiffKind::Pseudo {
                let slews = if std.name == "BLVDS_25" {
                    &[""][..]
                } else {
                    &["SLOW", "FAST"]
                };
                for &slew in slews {
                    let val = if slew.is_empty() {
                        std.name.to_string()
                    } else {
                        format!("{name}.{slew}", name = std.name)
                    };
                    let mut diff = ctx.state.get_diff(tile, "IOB1", "OSTD", &val);
                    for bel in ["IOB0", "IOB1"] {
                        diff.apply_bitvec_diff(
                            ctx.tiledb.item(tile, bel, "OUTPUT_ENABLE"),
                            &bitvec![1; 2],
                            &bitvec![0; 2],
                        );
                        let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                        let drive_item = ctx.tiledb.item(tile, bel, "DRIVE");
                        let TileItemKind::BitVec { ref invert } = drive_item.kind else {
                            unreachable!()
                        };
                        let value: BitVec = drive_item
                            .bits
                            .iter()
                            .zip(invert.iter())
                            .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                Some(val) => {
                                    assert_eq!(val, !*inv);
                                    true
                                }
                                None => false,
                            })
                            .collect();
                        ctx.tiledb
                            .insert_misc_data(format!("HR_IOSTD:DRIVE:{stdname}"), value);
                        let slew_item = ctx.tiledb.item(tile, bel, "SLEW");
                        let TileItemKind::BitVec { ref invert } = slew_item.kind else {
                            unreachable!()
                        };
                        let value: BitVec = slew_item
                            .bits
                            .iter()
                            .zip(invert.iter())
                            .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                Some(val) => {
                                    assert_eq!(val, !*inv);
                                    true
                                }
                                None => false,
                            })
                            .collect();
                        let name = if slew.is_empty() {
                            stdname.to_string()
                        } else {
                            format!("{stdname}.{slew}")
                        };
                        ctx.tiledb
                            .insert_misc_data(format!("HR_IOSTD:SLEW:{name}"), value);
                        let val = extract_bitvec_val_part(
                            ctx.tiledb.item(tile, bel, "OUTPUT_MISC"),
                            &bitvec![0; 3],
                            &mut diff,
                        );
                        ctx.tiledb
                            .insert_misc_data(format!("HR_IOSTD:OUTPUT_MISC:{stdname}"), val);
                    }
                    diff.apply_enum_diff(ctx.tiledb.item(tile, "IOB0", "OMUX"), "OTHER_O_INV", "O");
                    diff.assert_empty();
                }
            } else {
                if std.name != "TMDS_33" {
                    let mut diff = ctx.state.get_diff(tile, "IOB0", "DIFF_TERM", std.name);
                    let val_c = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, "IOB0", "LVDS"),
                        &bitvec![0; 13],
                        &mut diff,
                    );
                    let val_t = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, "IOB1", "LVDS"),
                        &bitvec![0; 13],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("HR_IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                    ctx.tiledb
                        .insert_misc_data(format!("HR_IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                    diff.assert_empty();

                    let mut diff = ctx
                        .state
                        .get_diff(tile, "IOB0", "DIFF_TERM_DYNAMIC", std.name);
                    let val_c = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, "IOB0", "LVDS"),
                        &bitvec![0; 13],
                        &mut diff,
                    );
                    let val_t = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, "IOB1", "LVDS"),
                        &bitvec![0; 13],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("HR_IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                    ctx.tiledb
                        .insert_misc_data(format!("HR_IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                    diff.assert_empty();
                }

                let mut diff = ctx.state.get_diff(tile, "IOB1", "OSTD", std.name);
                if std.name != "TMDS_33" {
                    let mut altdiff = ctx
                        .state
                        .get_diff(tile, "IOB1", "OSTD", format!("{}.ALT", std.name))
                        .combine(&!&diff);
                    let diff1 = altdiff.split_bits_by(|bit| bit.tile == 1);
                    ctx.tiledb
                        .insert(tile, "IOB0", "LVDS_GROUP", xlat_bit(altdiff));
                    ctx.tiledb
                        .insert(tile, "IOB1", "LVDS_GROUP", xlat_bit(diff1));
                }
                let val_c = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB0", "LVDS"),
                    &bitvec![0; 13],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, "IOB1", "LVDS"),
                    &bitvec![0; 13],
                    &mut diff,
                );
                ctx.tiledb
                    .insert_misc_data(format!("HR_IOSTD:LVDS_T:OUTPUT_{}", std.name), val_t);
                ctx.tiledb
                    .insert_misc_data(format!("HR_IOSTD:LVDS_C:OUTPUT_{}", std.name), val_c);
                diff.apply_bitvec_diff(
                    ctx.tiledb.item(tile, "IOB1", "OUTPUT_ENABLE"),
                    &bitvec![1; 2],
                    &bitvec![0; 2],
                );
                diff.assert_empty();
            }
        }
        ctx.collect_bit("IO_HR_TOP", "IOB", "LOW_VOLTAGE", "1");
        // meh.
        let _ = ctx
            .state
            .get_diff("IO_HR_BOT", "IOB", "OSTD", "LVCMOS18.4.SLOW.EXCL");
        let _ = ctx
            .state
            .get_diff("IO_HR_PAIR", "IOB_COMMON", "LOW_VOLTAGE", "1");
    }

    if ctx.has_tile("HCLK_IOI_HR") {
        let tile = "HCLK_IOI_HR";
        {
            let bel = "VCCOSENSE";
            ctx.collect_bit(tile, bel, "FLAG", "ENABLE");
            ctx.collect_enum(tile, bel, "MODE", &["OFF", "ALWAYSACTIVE", "FREEZE"]);
        }
        {
            let bel = "INTERNAL_VREF";
            let mut diffs = vec![("OFF", Diff::default())];
            for val in ["600", "675", "750", "900"] {
                diffs.push((val, ctx.state.get_diff(tile, bel, "VREF", val)));
            }
            // cannot be dealt with normally as there are no standards with such VREF.
            diffs.push((
                "1100",
                Diff {
                    bits: [TileBit::new(0, 38, 26), TileBit::new(0, 38, 29)]
                        .into_iter()
                        .map(|x| (x, true))
                        .collect(),
                },
            ));
            diffs.push((
                "1250",
                Diff {
                    bits: [TileBit::new(0, 38, 26), TileBit::new(0, 38, 30)]
                        .into_iter()
                        .map(|x| (x, true))
                        .collect(),
                },
            ));
            ctx.tiledb.insert(tile, bel, "VREF", xlat_enum(diffs));
        }
        {
            let bel = "DRIVERBIAS";
            let item = TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 39, 16),
                    TileBit::new(0, 39, 17),
                    TileBit::new(0, 39, 18),
                    TileBit::new(0, 38, 14),
                    TileBit::new(0, 38, 15),
                    TileBit::new(0, 39, 19),
                    TileBit::new(0, 39, 20),
                    TileBit::new(0, 39, 21),
                    TileBit::new(0, 41, 26),
                    TileBit::new(0, 41, 25),
                    TileBit::new(0, 41, 24),
                    TileBit::new(0, 41, 23),
                    TileBit::new(0, 41, 22),
                    TileBit::new(0, 41, 21),
                    TileBit::new(0, 39, 14),
                    TileBit::new(0, 39, 15),
                ],
                false,
            );
            for val in ["OFF", "3300", "2500"] {
                ctx.tiledb
                    .insert_misc_data(format!("HR_IOSTD:DRIVERBIAS:{val}"), bitvec![0; 16]);
            }
            let diff = ctx.state.get_diff(tile, bel, "DRIVERBIAS", "LV");
            let lv = extract_bitvec_val(&item, &bitvec![0; 16], diff);
            for val in ["1800", "1500", "1350", "1200"] {
                ctx.tiledb
                    .insert_misc_data(format!("HR_IOSTD:DRIVERBIAS:{val}"), lv.clone());
            }
            ctx.tiledb.insert(tile, bel, "DRIVERBIAS", item);
        }
        {
            let bel = "LVDS";
            let common = TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 40, 30),
                    TileBit::new(0, 40, 28),
                    TileBit::new(0, 40, 27),
                    TileBit::new(0, 40, 26),
                    TileBit::new(0, 40, 25),
                    TileBit::new(0, 40, 31),
                    TileBit::new(0, 39, 23),
                    TileBit::new(0, 41, 31),
                    TileBit::new(0, 41, 30),
                ],
                false,
            );
            let group0 = TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 38, 23),
                    TileBit::new(0, 38, 24),
                    TileBit::new(0, 38, 25),
                    TileBit::new(0, 41, 29),
                    TileBit::new(0, 41, 28),
                    TileBit::new(0, 41, 27),
                    TileBit::new(0, 41, 14),
                    TileBit::new(0, 41, 20),
                    TileBit::new(0, 41, 19),
                    TileBit::new(0, 41, 18),
                    TileBit::new(0, 41, 17),
                    TileBit::new(0, 41, 16),
                    TileBit::new(0, 41, 15),
                    TileBit::new(0, 38, 28),
                    TileBit::new(0, 38, 27),
                    TileBit::new(0, 40, 29),
                ],
                false,
            );
            let group1 = TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 38, 18),
                    TileBit::new(0, 38, 19),
                    TileBit::new(0, 38, 20),
                    TileBit::new(0, 40, 24),
                    TileBit::new(0, 40, 23),
                    TileBit::new(0, 40, 22),
                    TileBit::new(0, 40, 21),
                    TileBit::new(0, 40, 20),
                    TileBit::new(0, 40, 19),
                    TileBit::new(0, 40, 18),
                    TileBit::new(0, 40, 17),
                    TileBit::new(0, 40, 16),
                    TileBit::new(0, 40, 15),
                    TileBit::new(0, 40, 14),
                    TileBit::new(0, 39, 31),
                    TileBit::new(0, 38, 31),
                ],
                false,
            );
            for std in HR_IOSTDS {
                if std.diff != DiffKind::True {
                    continue;
                }
                let mut diff = ctx.state.get_diff(tile, bel, "STD0", std.name);
                let vc = extract_bitvec_val_part(&common, &bitvec![0; 9], &mut diff);
                let val = extract_bitvec_val(&group0, &bitvec![0; 16], diff);
                ctx.tiledb
                    .insert_misc_data(format!("HR_IOSTD:LVDSBIAS:COMMON:{}", std.name), vc);
                ctx.tiledb
                    .insert_misc_data(format!("HR_IOSTD:LVDSBIAS:GROUP:{}", std.name), val);
                if std.name != "TMDS_33" {
                    let diff = ctx.state.get_diff(tile, bel, "STD1", std.name);
                    let val = extract_bitvec_val(&group1, &bitvec![0; 16], diff);
                    ctx.tiledb
                        .insert_misc_data(format!("HR_IOSTD:LVDSBIAS:GROUP:{}", std.name), val);
                }
            }
            ctx.tiledb
                .insert_misc_data("HR_IOSTD:LVDSBIAS:COMMON:OFF", bitvec![0; 9]);
            ctx.tiledb
                .insert_misc_data("HR_IOSTD:LVDSBIAS:GROUP:OFF", bitvec![0; 16]);

            ctx.tiledb.insert(tile, bel, "COMMON", common);
            ctx.tiledb.insert(tile, bel, "GROUP0", group0);
            ctx.tiledb.insert(tile, bel, "GROUP1", group1);
        }
    }
}
