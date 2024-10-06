use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::{TileItem, TileItemKind};
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{
        extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_wide, xlat_bitvec,
        xlat_bool, xlat_enum, xlat_enum_ocd, CollectorCtx, Diff, OcdMode,
    },
    fgen::{BelKV, ExtraFeature, ExtraFeatureKind, TileBits, TileKV, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_one,
    fuzz_one_extras,
    io::iostd::DiffKind,
};

use super::iostd::{DciKind, Iostd};

const IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6", "8"]),
    Iostd::odci("LVDCI_25", 2500),
    Iostd::odci("LVDCI_18", 1800),
    Iostd::odci("LVDCI_15", 1500),
    Iostd::odci_half("LVDCI_DV2_25", 2500),
    Iostd::odci_half("LVDCI_DV2_18", 1800),
    Iostd::odci_half("LVDCI_DV2_15", 1500),
    Iostd::odci_vref("HSLVDCI_25", 2500, 1250),
    Iostd::odci_vref("HSLVDCI_18", 1800, 900),
    Iostd::odci_vref("HSLVDCI_15", 1500, 750),
    Iostd::vref("SSTL2_I", 2500, 1250),
    Iostd::vref("SSTL2_II", 2500, 1250),
    Iostd::vref("SSTL18_I", 1800, 900),
    Iostd::vref("SSTL18_II", 1800, 900),
    Iostd::vref("SSTL15", 1500, 750),
    Iostd::vref("HSTL_I_18", 1800, 900),
    Iostd::vref("HSTL_II_18", 1800, 900),
    Iostd::vref("HSTL_III_18", 1800, 1080),
    Iostd::vref("HSTL_I", 1500, 750),
    Iostd::vref("HSTL_II", 1500, 750),
    Iostd::vref("HSTL_III", 1500, 900),
    Iostd::vref("HSTL_I_12", 1200, 600),
    Iostd::vref_dci("SSTL2_I_DCI", 2500, 1250, DciKind::InputSplit),
    Iostd::vref_dci("SSTL2_II_DCI", 2500, 1250, DciKind::BiSplit),
    Iostd::vref_dci("SSTL2_II_T_DCI", 2500, 1250, DciKind::BiSplitT),
    Iostd::vref_dci("SSTL18_I_DCI", 1800, 900, DciKind::InputSplit),
    Iostd::vref_dci("SSTL18_II_DCI", 1800, 900, DciKind::BiSplit),
    Iostd::vref_dci("SSTL18_II_T_DCI", 1800, 900, DciKind::BiSplitT),
    Iostd::vref_dci("SSTL15_DCI", 1500, 750, DciKind::InputSplit),
    Iostd::vref_dci("SSTL15_T_DCI", 1500, 750, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_I_DCI_18", 1800, 900, DciKind::InputSplit),
    Iostd::vref_dci("HSTL_II_DCI_18", 1800, 900, DciKind::BiSplit),
    Iostd::vref_dci("HSTL_II_T_DCI_18", 1800, 900, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_III_DCI_18", 1800, 1080, DciKind::InputVcc),
    Iostd::vref_dci("HSTL_I_DCI", 1500, 750, DciKind::InputSplit),
    Iostd::vref_dci("HSTL_II_DCI", 1500, 750, DciKind::BiSplit),
    Iostd::vref_dci("HSTL_II_T_DCI", 1500, 750, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_III_DCI", 1500, 900, DciKind::InputVcc),
    Iostd::pseudo_diff("DIFF_SSTL2_I", 2500),
    Iostd::pseudo_diff("DIFF_SSTL2_II", 2500),
    Iostd::pseudo_diff("DIFF_SSTL18_I", 1800),
    Iostd::pseudo_diff("DIFF_SSTL18_II", 1800),
    Iostd::pseudo_diff("DIFF_SSTL15", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_I", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_II", 1500),
    Iostd::pseudo_diff("BLVDS_25", 2500),
    Iostd::pseudo_diff("LVPECL_25", 2500),
    Iostd::pseudo_diff_dci("DIFF_SSTL2_I_DCI", 2500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL2_II_DCI", 2500, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL2_II_T_DCI", 2500, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_I_DCI", 1800, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_II_DCI", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_II_T_DCI", 1800, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_SSTL15_DCI", 1500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL15_T_DCI", 1500, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_HSTL_I_DCI_18", 1800, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI_18", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_T_DCI_18", 1800, DciKind::BiSplitT),
    Iostd::pseudo_diff_dci("DIFF_HSTL_I_DCI", 1500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI", 1500, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_T_DCI", 1500, DciKind::BiSplitT),
    Iostd::true_diff("LVDS_25", 2500),
    Iostd::true_diff("LVDSEXT_25", 2500),
    Iostd::true_diff("RSDS_25", 2500),
    Iostd::true_diff("HT_25", 2500),
];

pub fn add_fuzzers<'a>(
    session: &mut Session<IseBackend<'a>>,
    backend: &IseBackend<'a>,
    devdata_only: bool,
) {
    let hclk_ioi = backend.egrid.db.get_node("HCLK_IOI");
    if devdata_only {
        for i in 0..2 {
            let bel_other = BelId::from_idx(4 + (1 - i));
            let ctx = FuzzCtx::new(
                session,
                backend,
                "IO",
                format!("IODELAY{i}"),
                TileBits::MainAuto,
            );
            fuzz_one!(ctx, "MODE", "I_DEFAULT", [
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "IDELAYCTRL", "USE")),
                (bel_mode bel_other, "IODELAYE1"),
                (bel_attr bel_other, "IDELAY_TYPE", "DEFAULT"),
                (bel_attr bel_other, "DELAY_SRC", "I")
            ], [
                (mode "IODELAYE1"),
                (attr "IDELAY_TYPE", "DEFAULT"),
                (attr "DELAY_SRC", "I")
            ]);
        }
        return;
    }
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
    let bel_ioi = BelId::from_idx(8);
    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "IO",
            format!("ILOGIC{i}"),
            TileBits::MainAuto,
        );

        fuzz_one!(ctx, "PRESENT", "ILOGIC", [], [(mode "ILOGICE1")]);
        fuzz_one!(ctx, "PRESENT", "ISERDES", [], [(mode "ISERDESE1")]);

        fuzz_inv!(ctx, "D", [(mode "ISERDESE1")]);
        fuzz_inv!(ctx, "CLK", [(mode "ISERDESE1")]);
        fuzz_inv!(ctx, "CLKDIV", [(mode "ISERDESE1"), (attr "DYN_CLKDIV_INV_EN", "FALSE")]);
        fuzz_enum!(ctx, "DYN_CLK_INV_EN", ["FALSE", "TRUE"], [(mode "ISERDESE1")]);
        fuzz_enum!(ctx, "DYN_OCLK_INV_EN", ["FALSE", "TRUE"], [(mode "ISERDESE1")]);
        fuzz_enum!(ctx, "DYN_CLKDIV_INV_EN", ["FALSE", "TRUE"], [(mode "ISERDESE1")]);
        fuzz_enum_suffix!(ctx, "OCLKINV", "SDR", ["OCLK", "OCLK_B"], [
            (mode "ISERDESE1"),
            (attr "DATA_RATE", "SDR"),
            (attr "OVERSAMPLE", "FALSE"),
            (attr "DYN_OCLK_INV_EN", "FALSE"),
            (attr "INTERFACE_TYPE", ""),
            (pin "OCLK")
        ]);
        fuzz_enum_suffix!(ctx, "OCLKINV", "DDR", ["OCLK", "OCLK_B"], [
            (mode "ISERDESE1"),
            (attr "DATA_RATE", "DDR"),
            (attr "OVERSAMPLE", "FALSE"),
            (attr "DYN_OCLK_INV_EN", "FALSE"),
            (attr "INTERFACE_TYPE", ""),
            (pin "OCLK")
        ]);

        fuzz_enum!(ctx, "SRUSED", ["0"], [
            (mode "ILOGICE1"),
            (attr "IFFTYPE", "#FF"),
            (pin "SR")
        ]);
        fuzz_enum!(ctx, "REVUSED", ["0"], [
            (mode "ILOGICE1"),
            (attr "IFFTYPE", "#FF"),
            (pin "REV")
        ]);
        fuzz_enum!(ctx, "SERDES", ["FALSE", "TRUE"], [
            (mode "ISERDESE1"),
            (attr "DATA_WIDTH", "2"),
            (attr "DATA_RATE", "SDR")
        ]);
        fuzz_enum!(ctx, "SERDES_MODE", ["MASTER", "SLAVE"], [(mode "ISERDESE1")]);
        fuzz_enum!(ctx, "DATA_WIDTH", ["2", "3", "4", "5", "6", "7", "8", "10"], [
            (mode "ISERDESE1"),
            (attr "SERDES", "FALSE")
        ]);
        fuzz_enum!(ctx, "NUM_CE", ["1", "2"], [
            (mode "ISERDESE1")
        ]);

        for attr in [
            "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
            "SRVAL_Q4",
        ] {
            fuzz_enum!(ctx, attr, ["0", "1"], [
                (mode "ISERDESE1")
            ]);
        }

        fuzz_enum_suffix!(ctx, "SRTYPE", "ILOGIC", ["SYNC", "ASYNC"], [
            (mode "ILOGICE1"),
            (attr "IFFTYPE", "#FF")
        ]);
        fuzz_enum_suffix!(ctx, "SRTYPE", "ISERDES", ["SYNC", "ASYNC"], [
            (mode "ISERDESE1")
        ]);

        fuzz_multi_attr_bin!(ctx, "INIT_CE", 2, [
            (mode "ISERDESE1"),
            (attr "DATA_RATE", "SDR")
        ]);
        fuzz_multi_attr_bin!(ctx, "INIT_BITSLIPCNT", 4, [
            (mode "ISERDESE1"),
            (attr "DATA_RATE", "SDR")
        ]);
        fuzz_multi_attr_bin!(ctx, "INIT_BITSLIP", 6, [
            (mode "ISERDESE1"),
            (attr "DATA_RATE", "SDR")
        ]);
        fuzz_multi_attr_bin!(ctx, "INIT_RANK1_PARTIAL", 5, [
            (mode "ISERDESE1"),
            (attr "DATA_RATE", "SDR")
        ]);
        fuzz_multi_attr_bin!(ctx, "INIT_RANK2", 6, [
            (mode "ISERDESE1"),
            (attr "DATA_RATE", "SDR")
        ]);
        fuzz_multi_attr_bin!(ctx, "INIT_RANK3", 6, [
            (mode "ISERDESE1"),
            (attr "DATA_RATE", "SDR")
        ]);

        fuzz_enum!(ctx, "OFB_USED", ["FALSE", "TRUE"], [
            (mode "ISERDESE1"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "TFB_USED", ["FALSE", "TRUE"], [
            (mode "ISERDESE1"),
            (pin "TFB")
        ]);
        fuzz_enum!(ctx, "IOBDELAY", ["NONE", "IFD", "IBUF", "BOTH"], [
            (mode "ISERDESE1")
        ]);

        fuzz_enum!(ctx, "D2OBYP_SEL", ["GND", "T"], [
            (mode "ILOGICE1"),
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
            (mode "ILOGICE1"),
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
            (mode "ILOGICE1"),
            (attr "IDELMUX", "1"),
            (attr "DINV", ""),
            (pin "D"),
            (pin "DDLY"),
            (pin "O"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IFFMUX", ["0", "1"], [
            (mode "ILOGICE1"),
            (attr "IFFDELMUX", "1"),
            (attr "IFFTYPE", "#FF"),
            (attr "DINV", ""),
            (pin "D"),
            (pin "DDLY"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IDELMUX", ["0", "1"], [
            (mode "ILOGICE1"),
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
            (mode "ILOGICE1"),
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

        fuzz_enum!(ctx, "D_EMU", ["FALSE", "TRUE"], [(mode "ISERDESE1")]);
        fuzz_enum!(ctx, "D_EMU_OPTION", [
            "MATCH_DLY0", "MATCH_DLY2", "DLY0", "DLY1", "DLY2", "DLY3"
        ], [(mode "ISERDESE1")]);
        fuzz_enum!(ctx, "RANK12_DLY", ["FALSE", "TRUE"], [(mode "ISERDESE1")]);
        fuzz_enum!(ctx, "RANK23_DLY", ["FALSE", "TRUE"], [(mode "ISERDESE1")]);

        fuzz_enum!(ctx, "INTERFACE_TYPE", ["NETWORKING", "MEMORY", "MEMORY_DDR3", "MEMORY_QDR", "OVERSAMPLE"], [
            (mode "ISERDESE1"),
            (attr "OVERSAMPLE", "FALSE")
        ]);
        fuzz_enum!(ctx, "DATA_RATE", ["SDR", "DDR"], [
            (mode "ISERDESE1"),
            (attr "INIT_BITSLIPCNT", "1111"),
            (attr "INIT_RANK1_PARTIAL", "11111"),
            (attr "INIT_RANK2", "111111"),
            (attr "INIT_RANK3", "111111"),
            (attr "INIT_CE", "11")
        ]);
        fuzz_enum!(ctx, "DDR_CLK_EDGE", ["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"], [
            (mode "ISERDESE1")
        ]);
        fuzz_enum!(ctx, "DDR_CLK_EDGE", ["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"], [
            (mode "ILOGICE1"),
            (attr "IFFTYPE", "DDR")
        ]);
        fuzz_enum!(ctx, "IFFTYPE", ["#FF", "#LATCH", "DDR"], [
            (mode "ILOGICE1")
        ]);

        for (src, num) in [("HCLK", 12), ("RCLK", 6), ("IOCLK", 8)] {
            for j in 0..num {
                fuzz_one!(ctx, "MUX.CLK", format!("{src}{j}"), [
                    (mutex "MUX.CLK", format!("{src}{j}")),
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLKB"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLK"))
                ]);
                fuzz_one!(ctx, "MUX.CLKB", format!("{src}{j}"), [
                    (mutex "MUX.CLK", format!("{src}{j}"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLKB"))
                ]);
            }
        }
    }
    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "IO",
            format!("OLOGIC{i}"),
            TileBits::MainAuto,
        );

        fuzz_one!(ctx, "PRESENT", "OLOGIC", [], [(mode "OLOGICE1")]);
        fuzz_one!(ctx, "PRESENT", "OSERDES", [], [(mode "OSERDESE1")]);

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "T2", "T3", "T4", "CLKDIV", "CLKPERF",
        ] {
            fuzz_inv!(ctx, pin, [(mode "OSERDESE1")]);
        }
        fuzz_inv!(ctx, "T1", [
            (mode "OLOGICE1"),
            (attr "TMUX", "T1"),
            (attr "T1USED", "0"),
            (pin "TQ")
        ]);
        fuzz_enum_suffix!(ctx, "CLKINV", "SAME", ["CLK", "CLK_B"], [
            (mode "OSERDESE1"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "DDR_CLK_EDGE", "SAME_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);
        fuzz_enum_suffix!(ctx, "CLKINV", "OPPOSITE", ["CLK", "CLK_B"], [
            (mode "OSERDESE1"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "DDR_CLK_EDGE", "OPPOSITE_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);

        fuzz_enum!(ctx, "SRTYPE_OQ", ["SYNC", "ASYNC"], [
            (mode "OLOGICE1"),
            (attr "OUTFFTYPE", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE_TQ", ["SYNC", "ASYNC"], [
            (mode "OLOGICE1"),
            (attr "TFFTYPE", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
            (mode "OSERDESE1")
        ]);

        fuzz_enum_suffix!(ctx, "INIT_OQ", "OLOGIC", ["0", "1"], [(mode "OLOGICE1")]);
        fuzz_enum_suffix!(ctx, "INIT_TQ", "OLOGIC", ["0", "1"], [(mode "OLOGICE1")]);
        fuzz_enum_suffix!(ctx, "INIT_OQ", "OSERDES", ["0", "1"], [(mode "OSERDESE1")]);
        fuzz_enum_suffix!(ctx, "INIT_TQ", "OSERDES", ["0", "1"], [(mode "OSERDESE1")]);
        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OLOGIC", ["0", "1"], [(mode "OLOGICE1")]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "OLOGIC", ["0", "1"], [(mode "OLOGICE1")]);
        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OSERDES", ["0", "1"], [(mode "OSERDESE1")]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "OSERDES", ["0", "1"], [(mode "OSERDESE1")]);

        for attr in [
            "OSRUSED", "TSRUSED", "OREVUSED", "TREVUSED", "OCEUSED", "TCEUSED",
        ] {
            fuzz_enum!(ctx, attr, ["0"], [
                (mode "OLOGICE1"),
                (attr "OUTFFTYPE", "#FF"),
                (attr "TFFTYPE", "#FF"),
                (pin "OCE"),
                (pin "TCE"),
                (pin "REV"),
                (pin "SR")
            ]);
        }

        fuzz_enum!(ctx, "OUTFFTYPE", ["#FF", "#LATCH", "DDR"], [
            (mode "OLOGICE1"),
            (pin "OQ")
        ]);
        fuzz_enum!(ctx, "TFFTYPE", ["#FF", "#LATCH", "DDR"], [
            (mode "OLOGICE1"),
            (pin "TQ")
        ]);

        fuzz_enum!(ctx, "DATA_RATE_OQ", ["SDR", "DDR"], [
            (mode "OSERDESE1")
        ]);
        fuzz_enum!(ctx, "DATA_RATE_TQ", ["BUF", "SDR", "DDR"], [
            (mode "OSERDESE1"),
            (attr "T1INV", "T1"),
            (pin "T1")
        ]);

        fuzz_enum!(ctx, "MISR_ENABLE", ["FALSE", "TRUE"], [
            (mode "OLOGICE1"),
            (global_opt "ENABLEMISR", "Y")
        ]);
        fuzz_enum!(ctx, "MISR_ENABLE_FDBK", ["FALSE", "TRUE"], [
            (mode "OLOGICE1"),
            (global_opt "ENABLEMISR", "Y")
        ]);
        fuzz_enum!(ctx, "MISR_CLK_SELECT", ["CLK1", "CLK2"], [
            (mode "OLOGICE1"),
            (global_opt "ENABLEMISR", "Y")
        ]);

        fuzz_enum!(ctx, "SERDES", ["FALSE", "TRUE"], [
            (mode "OSERDESE1")
        ]);
        fuzz_enum!(ctx, "SERDES_MODE", ["SLAVE", "MASTER"], [
            (mode "OSERDESE1")
        ]);
        fuzz_enum!(ctx, "SELFHEAL", ["FALSE", "TRUE"], [
            (mode "OSERDESE1")
        ]);
        fuzz_enum!(ctx, "INTERFACE_TYPE", ["DEFAULT", "MEMORY_DDR3"], [
            (mode "OSERDESE1")
        ]);
        fuzz_enum!(ctx, "TRISTATE_WIDTH", ["1", "4"], [
            (mode "OSERDESE1")
        ]);
        fuzz_enum_suffix!(ctx, "DATA_WIDTH", "SDR", ["2", "3", "4", "5", "6", "7", "8"], [
            (mode "OSERDESE1"),
            (attr "DATA_RATE_OQ", "SDR"),
            (attr "INTERFACE_TYPE", "DEFAULT")
        ]);
        fuzz_enum_suffix!(ctx, "DATA_WIDTH", "DDR", ["4", "6", "8", "10"], [
            (mode "OSERDESE1"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "INTERFACE_TYPE", "DEFAULT")
        ]);
        fuzz_enum!(ctx, "WC_DELAY", ["0", "1"], [(mode "OSERDESE1")]);
        fuzz_enum!(ctx, "DDR3_DATA", ["0", "1"], [(mode "OSERDESE1")]);
        fuzz_enum!(ctx, "ODELAY_USED", ["0", "1"], [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_LOADCNT", 4, [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_ORANK1", 6, [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_ORANK2_PARTIAL", 4, [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_TRANK1", 4, [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_FIFO_ADDR", 11, [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_FIFO_RESET", 13, [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_DLY_CNT", 10, [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_PIPE_DATA0", 12, [(mode "OSERDESE1")]);
        fuzz_multi_attr_bin!(ctx, "INIT_PIPE_DATA1", 12, [(mode "OSERDESE1")]);

        for (src, num) in [("HCLK", 12), ("RCLK", 6)] {
            for j in 0..num {
                fuzz_one!(ctx, "MUX.CLKDIV", format!("{src}{j}"), [
                    (mutex "MUX.CLKDIV", format!("{src}{j}")),
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLKDIVB"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLKDIV"))
                ]);
                fuzz_one!(ctx, "MUX.CLKDIVB", format!("{src}{j}"), [
                    (mutex "MUX.CLKDIV", format!("{src}{j}"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLKDIVB"))
                ]);
            }
        }
        for (src, num) in [("HCLK", 12), ("RCLK", 6), ("IOCLK", 8)] {
            for j in 0..num {
                fuzz_one!(ctx, "MUX.CLK", format!("{src}{j}"), [
                    (mutex "MUX.CLK", format!("{src}{j}")),
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLKM"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLK_MUX"))
                ]);
                fuzz_one!(ctx, "MUX.CLKB", format!("{src}{j}"), [
                    (mutex "MUX.CLK", format!("{src}{j}"))
                ], [
                    (pip (bel_pin bel_ioi, format!("{src}{j}")), (pin "CLKM"))
                ]);
            }
        }
        for j in 0..2 {
            fuzz_one!(ctx, "MUX.CLKPERF", format!("OCLK{j}"), [
                (mutex "MUX.CLKPERF", format!("OCLK{j}"))
            ], [
                (pip (bel_pin bel_ioi, format!("OCLK{j}")), (pin "CLKPERF"))
            ]);
        }
    }
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    fuzz_one_extras!(ctx, "MISR_RESET", "1", [
        (global_opt "ENABLEMISR", "Y")
    ], [
        (global_opt_diff "MISRRESET", "N", "Y")
    ], vec![
        ExtraFeature::new(
            ExtraFeatureKind::AllIobs,
            "IO",
            "OLOGIC_COMMON",
            "MISR_RESET",
            "1",
        )
    ]);
    for i in 0..2 {
        let bel_other = BelId::from_idx(4 + (1 - i));
        let ctx = FuzzCtx::new(
            session,
            backend,
            "IO",
            format!("IODELAY{i}"),
            TileBits::MainAuto,
        );

        fuzz_one!(ctx, "PRESENT", "1", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1")
        ], [
            (mode "IODELAYE1")
        ]);
        for pin in ["C", "DATAIN", "IDATAIN"] {
            fuzz_inv!(ctx, pin, [
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "IDELAYCTRL", "USE")),
                (mode "IODELAYE1")
            ]);
        }
        fuzz_enum!(ctx, "CINVCTRL_SEL", ["FALSE", "TRUE"], [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (mode "IODELAYE1")
        ]);
        fuzz_enum!(ctx, "HIGH_PERFORMANCE_MODE", ["FALSE", "TRUE"], [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (mode "IODELAYE1")
        ]);
        fuzz_enum!(ctx, "DELAY_SRC", ["I", "O", "IO", "DATAIN", "CLKIN"], [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "FIXED"),
            (attr "ODELAY_TYPE", "FIXED")
        ]);
        fuzz_one!(ctx, "DELAY_SRC", "DELAYCHAIN_OSC", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "FIXED"),
            (attr "ODELAY_TYPE", "FIXED")
        ], [
            (attr "DELAY_SRC", "I"),
            (attr "DELAYCHAIN_OSC", "TRUE")
        ]);
        fuzz_multi_attr_dec!(ctx, "IDELAY_VALUE", 5, [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (mode "IODELAYE1"),
            (attr "DELAY_SRC", "IO"),
            (attr "IDELAY_TYPE", "FIXED"),
            (attr "ODELAY_TYPE", "FIXED")
        ]);
        fuzz_multi_attr_dec!(ctx, "ODELAY_VALUE", 5, [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (mode "IODELAYE1"),
            (attr "DELAY_SRC", "IO"),
            (attr "IDELAY_TYPE", "FIXED"),
            (attr "ODELAY_TYPE", "FIXED")
        ]);
        fuzz_one!(ctx, "MODE", "I_DEFAULT", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "DEFAULT"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "DEFAULT"),
            (attr "DELAY_SRC", "I")
        ]);
        fuzz_one!(ctx, "MODE", "I_FIXED", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "FIXED"),
            (attr "DELAY_SRC", "I")
        ]);
        fuzz_one!(ctx, "MODE", "I_VARIABLE", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "VARIABLE"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "VARIABLE"),
            (attr "DELAY_SRC", "I")
        ]);
        fuzz_one!(ctx, "MODE", "I_VAR_LOADABLE", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "VAR_LOADABLE"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "VAR_LOADABLE"),
            (attr "DELAY_SRC", "I")
        ]);
        fuzz_one!(ctx, "MODE", "O_FIXED", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "ODELAY_TYPE", "FIXED"),
            (attr "DELAY_SRC", "O")
        ]);
        fuzz_one!(ctx, "MODE", "O_VARIABLE", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "ODELAY_TYPE", "VARIABLE"),
            (attr "DELAY_SRC", "O")
        ]);
        fuzz_one!(ctx, "MODE", "O_VAR_LOADABLE", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "ODELAY_TYPE", "VAR_LOADABLE"),
            (attr "DELAY_SRC", "O")
        ]);
        fuzz_one!(ctx, "MODE", "IO_FIXED", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "FIXED"),
            (attr "ODELAY_TYPE", "FIXED"),
            (attr "DELAY_SRC", "IO")
        ]);
        fuzz_one!(ctx, "MODE", "I_VARIABLE_O_FIXED", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "VARIABLE"),
            (attr "ODELAY_TYPE", "FIXED"),
            (attr "DELAY_SRC", "IO")
        ]);
        fuzz_one!(ctx, "MODE", "I_FIXED_O_VARIABLE", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "FIXED"),
            (attr "ODELAY_TYPE", "VARIABLE"),
            (attr "DELAY_SRC", "IO")
        ]);
        fuzz_one!(ctx, "MODE", "IO_VAR_LOADABLE", [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "IDELAYCTRL", "USE")),
            (bel_mode bel_other, "IODELAYE1"),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (bel_attr bel_other, "DELAY_SRC", "I")
        ], [
            (mode "IODELAYE1"),
            (attr "IDELAY_TYPE", "VAR_LOADABLE"),
            (attr "ODELAY_TYPE", "VAR_LOADABLE"),
            (attr "DELAY_SRC", "IO")
        ]);
    }
    for i in 0..2 {
        let bel_ologic = BelId::from_idx(2 + i);
        let bel_other_ologic = BelId::from_idx(2 + (1 - i));
        let bel_iodelay = BelId::from_idx(4 + i);
        let bel_other = BelId::from_idx(6 + (1 - i));
        let ctx = FuzzCtx::new(
            session,
            backend,
            "IO",
            format!("IOB{i}"),
            TileBits::MainAuto,
        );
        fuzz_one!(ctx, "PRESENT", "IOB", [
            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
            (package package.name),
            (bel_special BelKV::IsBonded)
        ], [
            (mode "IOB")
        ]);
        fuzz_one!(ctx, "PRESENT", "IOB.CONTINUOUS", [
            (global_opt "DCIUPDATEMODE", "CONTINUOUS"),
            (package package.name),
            (bel_special BelKV::IsBonded)
        ], [
            (mode "IOB")
        ]);
        fuzz_one!(ctx, "PRESENT", "IPAD", [
            (global_opt "DCIUPDATEMODE", "ASREQUIRED"),
            (package package.name),
            (bel_special BelKV::IsBonded)
        ], [
            (mode "IPAD")
        ]);
        fuzz_enum!(ctx, "PULL", ["KEEPER", "PULLDOWN", "PULLUP"], [
            (package package.name),
            (bel_special BelKV::IsBonded),
            (mode "IOB")
        ]);
        for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
            fuzz_one!(ctx, "PULL_DYNAMIC", "1", [
                (package package.name),
                (bel_special BelKV::IsBonded),
                (mutex "PULL_DYNAMIC", pin),
                (mode "IOB")
            ], [
                (pin_pips pin)
            ]);
        }
        fuzz_multi_attr_bin!(ctx, "OPROGRAMMING", 31, [
            (related TileRelation::Hclk(hclk_ioi),
                (tile_mutex "VCCO", "1800")),
            (mode "IOB"),
            (pin "O"),
            (attr "OUSED", "0"),
            (attr "OSTANDARD", "LVCMOS18"),
            (attr "DRIVE", "12"),
            (attr "SLEW", "SLOW")
        ]);
        for &std in IOSTDS {
            let mut extras = vec![];
            let mut vref_special = BelKV::Nop;
            let mut dci_special = BelKV::Nop;
            if std.vref.is_some() {
                vref_special = BelKV::PrepVref;
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::Vref,
                    "IO",
                    "IOB0",
                    "PRESENT",
                    "VREF",
                ));
            }
            if matches!(
                std.dci,
                DciKind::BiSplit
                    | DciKind::BiSplitT
                    | DciKind::BiVcc
                    | DciKind::InputSplit
                    | DciKind::InputVcc
            ) {
                dci_special = BelKV::PrepDci;
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::Vr,
                    "IO",
                    "IOB_COMMON",
                    "PRESENT",
                    "VR",
                ));
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::HclkIoDci("HCLK_IOI"),
                    "HCLK_IOI",
                    "DCI",
                    "STD",
                    std.name,
                ));
            }
            if std.diff != DiffKind::None {
                for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                    fuzz_one_extras!(ctx, "ISTD", format!("{sn}.{suffix}", sn=std.name), [
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (mode "IOB"),
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
                if std.diff == DiffKind::True && i == 0 {
                    fuzz_one!(ctx, "DIFF_TERM", std.name, [
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (mode "IOB"),
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
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (mode "IOB"),
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
            } else {
                for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                    fuzz_one_extras!(ctx, "ISTD", format!("{sn}.{suffix}", sn=std.name), [
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (mode "IOB"),
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
        for &std in IOSTDS {
            let mut extras = vec![];
            let mut dci_special = BelKV::Nop;
            if matches!(
                std.dci,
                DciKind::Output | DciKind::OutputHalf | DciKind::BiSplit | DciKind::BiVcc
            ) {
                extras.extend([
                    ExtraFeature::new(ExtraFeatureKind::Vr, "IO", "IOB_COMMON", "PRESENT", "VR"),
                    ExtraFeature::new(
                        ExtraFeatureKind::HclkIoDci("HCLK_IOI"),
                        "HCLK_IOI",
                        "DCI",
                        "STD",
                        std.name,
                    ),
                ]);
                dci_special = BelKV::PrepDci;
            }
            if std.diff == DiffKind::True {
                if i == 1 {
                    let extras = vec![ExtraFeature::new(
                        ExtraFeatureKind::Hclk(0, 0),
                        "HCLK_IOI",
                        "LVDS",
                        "STD",
                        std.name,
                    )];
                    fuzz_one_extras!(ctx, "OSTD", std.name, [
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
                        (mode_diff "IOB", "IOBM"),
                        (pin "O"),
                        (attr "OUSED", "0"),
                        (attr "DIFFO_OUTUSED", "0"),
                        (attr "OSTANDARD", std.name),
                        (bel_mode_diff bel_other, "IOB", "IOBS"),
                        (bel_attr bel_other, "OUTMUX", "1"),
                        (bel_attr bel_other, "DIFFO_INUSED", "0"),
                        (pin_pair "DIFFO_OUT", bel_other, "DIFFO_IN")
                    ], extras);
                }
            } else if std.diff != DiffKind::None {
                if i == 1 {
                    fuzz_one_extras!(ctx, "OSTD", std.name, [
                        (related TileRelation::Hclk(hclk_ioi),
                            (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                        (attr "IUSED", ""),
                        (attr "OPROGRAMMING", ""),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special dci_special),
                        (bel_attr bel_other, "IUSED", ""),
                        (bel_attr bel_other, "OPROGRAMMING", ""),
                        (bel_mode bel_other_ologic, "OLOGICE1")
                    ], [
                        (mode_diff "IOB", "IOBM"),
                        (pin "O"),
                        (attr "OUSED", "0"),
                        (attr "O_OUTUSED", "0"),
                        (attr "OSTANDARD", std.name),
                        (bel_mode_diff bel_other, "IOB", "IOBS"),
                        (bel_attr bel_other, "OUTMUX", "0"),
                        (bel_attr bel_other, "OINMUX", "1"),
                        (bel_attr bel_other, "OSTANDARD", std.name),
                        (pin_pair "O_OUT", bel_other, "O_IN")
                    ], extras);
                }
            } else if !std.drive.is_empty() {
                for &drive in std.drive {
                    for slew in ["SLOW", "FAST"] {
                        fuzz_one!(ctx, "OSTD", format!("{name}.{drive}.{slew}", name=std.name), [
                            (related TileRelation::Hclk(hclk_ioi),
                                (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                            (mode "IOB"),
                            (pin "O"),
                            (attr "IUSED", ""),
                            (attr "OPROGRAMMING", "")
                        ], [
                            (attr "OUSED", "0"),
                            (attr "OSTANDARD", std.name),
                            (attr "DRIVE", drive),
                            (attr "SLEW", slew)
                        ]);
                    }
                }
            } else {
                fuzz_one_extras!(ctx, "OSTD", std.name, [
                    (related TileRelation::Hclk(hclk_ioi),
                        (tile_mutex "VCCO", std.vcco.unwrap().to_string())),
                    (mode "IOB"),
                    (pin "O"),
                    (attr "IUSED", ""),
                    (attr "OPROGRAMMING", ""),
                    (package package.name),
                    (bel_special BelKV::IsBonded),
                    (bel_special dci_special)
                ], [
                    (attr "OUSED", "0"),
                    (attr "OSTANDARD", std.name)
                ], extras);
            }
        }

        for (std, vcco, vref) in [
            ("HSTL_I_12", 1200, 600),
            ("HSTL_I", 1500, 750),
            ("HSTL_III", 1500, 900),
            ("HSTL_III_18", 1800, 1100),
            ("SSTL2_I", 2500, 1250),
        ] {
            let extras = vec![ExtraFeature::new(
                ExtraFeatureKind::Hclk(0, 0),
                "HCLK_IOI",
                "INTERNAL_VREF",
                "VREF",
                format!("{vref}"),
            )];
            fuzz_one_extras!(ctx, "ISTD", format!("{std}.LP"), [
                (related TileRelation::Hclk(hclk_ioi),
                    (tile_mutex "VCCO", vcco.to_string())),
                (mode "IOB"),
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

        fuzz_one!(ctx, "OUTPUT_DELAY", "0", [
            (mutex "OUTPUT_DELAY", "0"),
            (bel_mode bel_ologic, "OLOGICE1")
        ], [
            (pip (bel_pin bel_ologic, "OQ"), (bel_pin bel_ologic, "IOB_O"))
        ]);
        fuzz_one!(ctx, "OUTPUT_DELAY", "1", [
            (mutex "OUTPUT_DELAY", "1"),
            (bel_mode bel_ologic, "OLOGICE1")
        ], [
            (pip (bel_pin bel_iodelay, "DATAOUT"), (bel_pin bel_ologic, "IOB_O"))
        ]);
    }
    let ctx = FuzzCtx::new(session, backend, "HCLK_IOI", "DCI", TileBits::Hclk);
    fuzz_one!(ctx, "TEST_ENABLE", "1", [
        (global_mutex "GLOBAL_DCI", "NOPE")
    ], [
        (mode "DCI")
    ]);
    fuzz_one!(ctx, "DYNAMIC_ENABLE", "1", [
        (global_mutex "GLOBAL_DCI", "NOPE")
    ], [
        (mode "DCI"),
        (pin_pips "INT_DCI_EN")
    ]);
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    fuzz_one_extras!(ctx, "DCIUPDATEMODE", "QUIET", [], [
        (global_opt_diff "DCIUPDATEMODE", "CONTINUOUS", "QUIET")
    ], vec![
        ExtraFeature::new(
            ExtraFeatureKind::AllHclkIo("HCLK_IOI"),
            "HCLK_IOI",
            "DCI",
            "QUIET",
            "1",
        )
    ]);
    let extras = vec![
        ExtraFeature::new(ExtraFeatureKind::Cfg, "CFG", "MISC", "DCI_CLK_ENABLE", "1"),
        ExtraFeature::new(
            ExtraFeatureKind::CenterDciIo(25),
            "IO",
            "IOB0",
            "OSTD",
            "LVDCI_25",
        ),
        ExtraFeature::new(
            ExtraFeatureKind::CenterDciHclk(25),
            "HCLK_IOI",
            "DCI",
            "ENABLE",
            "1",
        ),
        ExtraFeature::new(
            ExtraFeatureKind::CenterDciVr(25),
            "IO",
            "IOB_COMMON",
            "PRESENT",
            "VR",
        ),
    ];
    fuzz_one_extras!(ctx, "CENTER_DCI", "1", [
        (package package.name),
        (special TileKV::CenterDci(25))
    ], [], extras);
    for (a, b) in [(25, 24), (25, 26)] {
        let extras = vec![
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciIo(b),
                "IO",
                "IOB0",
                "OSTD",
                "LVDCI_25",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciHclk(b),
                "HCLK_IOI",
                "DCI",
                if b == 24 {
                    "CASCADE_FROM_ABOVE"
                } else {
                    "CASCADE_FROM_BELOW"
                },
                "1",
            ),
        ];
        fuzz_one_extras!(ctx, format!("CASCADE_DCI.{a}.{b}"), "1", [
            (package package.name),
            (special TileKV::CascadeDci(a, b))
        ], [], extras);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let tile = "IO";
    if devdata_only {
        for i in 0..2 {
            let bel = &format!("IODELAY{i}");
            let mut diff = ctx.state.get_diff(tile, bel, "MODE", "I_DEFAULT");
            let val = extract_bitvec_val_part(
                ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"),
                &bitvec![1; 5],
                &mut diff,
            );
            ctx.tiledb
                .insert_device_data(&ctx.device.name, "IODELAY:DEFAULT_IDELAY_VALUE", val);
            let val = extract_bitvec_val_part(
                ctx.tiledb.item(tile, bel, "IDELAY_VALUE_INIT"),
                &bitvec![0; 5],
                &mut diff,
            );
            ctx.tiledb
                .insert_device_data(&ctx.device.name, "IODELAY:DEFAULT_IDELAY_VALUE", val);
        }
        return;
    }
    for i in 0..2 {
        let bel = &format!("ILOGIC{i}");

        ctx.collect_inv(tile, bel, "D");
        ctx.collect_inv(tile, bel, "CLKDIV");
        let item = ctx.extract_enum_bool_wide(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.tiledb.insert(tile, bel, "INV.CLK", item);

        let diff1 = ctx.state.get_diff(tile, bel, "OCLKINV.DDR", "OCLK");
        let diff2 = ctx.state.get_diff(tile, bel, "OCLKINV.DDR", "OCLK_B");
        ctx.state
            .get_diff(tile, bel, "OCLKINV.SDR", "OCLK_B")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "OCLKINV.SDR", "OCLK");
        diff = diff.combine(&!&diff1);
        diff = diff.combine(&!&diff2);
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "INV.OCLK1", xlat_bit(!diff1));
        ctx.tiledb.insert(tile, bel, "INV.OCLK2", xlat_bit(!diff2));

        ctx.collect_enum_bool(tile, bel, "DYN_CLK_INV_EN", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "DYN_CLKDIV_INV_EN", "FALSE", "TRUE");
        ctx.collect_enum_bool_wide(tile, bel, "DYN_OCLK_INV_EN", "FALSE", "TRUE");

        let iff_rev_used = ctx.extract_bit(tile, bel, "REVUSED", "0");
        ctx.tiledb.insert(tile, bel, "IFF_REV_USED", iff_rev_used);
        let iff_sr_used = ctx.extract_bit(tile, bel, "SRUSED", "0");
        ctx.tiledb.insert(tile, bel, "IFF_SR_USED", iff_sr_used);
        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["MASTER", "SLAVE"]);
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10"] {
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
        ctx.collect_bitvec(tile, bel, "INIT_RANK1_PARTIAL", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK2", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK3", "");
        ctx.collect_bitvec(tile, bel, "INIT_BITSLIP", "");
        ctx.collect_bitvec(tile, bel, "INIT_BITSLIPCNT", "");
        ctx.collect_bitvec(tile, bel, "INIT_CE", "");
        let item = ctx.extract_enum_bool(tile, bel, "SRTYPE.ILOGIC", "ASYNC", "SYNC");
        ctx.tiledb.insert(tile, bel, "IFF_SYNC", item);
        ctx.state
            .get_diff(tile, bel, "SRTYPE.ISERDES", "ASYNC")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "SRTYPE.ISERDES", "SYNC");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_SYNC"), true, false);
        ctx.tiledb.insert(tile, bel, "BITSLIP_SYNC", xlat_bit(diff));
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
        let diff_os = ctx
            .state
            .get_diff(tile, bel, "INTERFACE_TYPE", "OVERSAMPLE");
        let bitslip_en = diff_net.combine(&!&diff_qdr);
        let diff_ddr3 = diff_ddr3.combine(&!&bitslip_en);
        let diff_os = diff_os.combine(&!&bitslip_en);
        ctx.tiledb
            .insert(tile, bel, "BITSLIP_ENABLE", xlat_bit(bitslip_en));
        ctx.tiledb.insert(
            tile,
            bel,
            "INTERFACE_TYPE",
            xlat_enum(vec![
                ("MEMORY", diff_mem),
                ("NETWORKING", diff_qdr),
                ("MEMORY_DDR3", diff_ddr3),
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
        ctx.tiledb.insert(tile, bel, "IFF_LATCH", xlat_bit(!diff));
        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "DDR");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.tiledb.insert(tile, bel, "IFF_LATCH", xlat_bit(!diff));

        let mut diffs = vec![];
        for val in ["SDR", "DDR"] {
            let mut diff = ctx.state.get_diff(tile, bel, "DATA_RATE", val);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_SR_USED"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_LATCH"), false, true);
            diffs.push((val, diff));
        }
        ctx.tiledb.insert(tile, bel, "DATA_RATE", xlat_enum(diffs));

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

        ctx.collect_enum_bool(tile, bel, "D_EMU", "FALSE", "TRUE");
        ctx.collect_enum(
            tile,
            bel,
            "D_EMU_OPTION",
            &["DLY0", "DLY1", "DLY2", "DLY3", "MATCH_DLY0", "MATCH_DLY2"],
        );
        ctx.collect_enum_bool(tile, bel, "RANK12_DLY", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "RANK23_DLY", "FALSE", "TRUE");

        ctx.state
            .get_diff(tile, bel, "PRESENT", "ILOGIC")
            .assert_empty();
        let mut present_iserdes = ctx.state.get_diff(tile, bel, "PRESENT", "ISERDES");
        present_iserdes.apply_enum_diff(ctx.tiledb.item(tile, bel, "TSBYPASS_MUX"), "GND", "T");
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF1_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF2_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF3_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF4_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF1_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF2_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF3_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF4_INIT"), false, true);
        present_iserdes.assert_empty();

        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK_I",
            TileItem::from_bit(
                [FeatureBit::new(0, 26, 61), FeatureBit::new(1, 27, 2)][i],
                false,
            ),
        );

        let mut vals = vec![];
        for j in 0..12 {
            vals.push(format!("HCLK{j}"));
        }
        for j in 0..6 {
            vals.push(format!("RCLK{j}"));
        }
        for j in 0..8 {
            vals.push(format!("IOCLK{j}"));
        }
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLK", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKB", &vals, "NONE", OcdMode::Mux);
    }
    for i in 0..2 {
        let bel = &format!("OLOGIC{i}");

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "T2", "T3", "T4", "CLKPERF", "CLKDIV",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }

        let diff0 = ctx.state.get_diff(tile, bel, "T1INV", "T1");
        let diff1 = ctx.state.get_diff(tile, bel, "T1INV", "T1_B");
        let (diff0, diff1, _) = Diff::split(diff0, diff1);
        ctx.tiledb
            .insert(tile, bel, "INV.T1", xlat_bool(diff0, diff1));

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

        let item_oq = ctx.extract_enum_bool_wide(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_enum_bool_wide(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.state
            .get_diff(tile, bel, "SRTYPE", "ASYNC")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bitvec_diff(&item_oq, &bitvec![1; 4], &bitvec![0; 4]);
        diff.apply_bitvec_diff(&item_tq, &bitvec![1; 2], &bitvec![0; 2]);
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

        ctx.state
            .get_diff(tile, bel, "OREVUSED", "0")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "TREVUSED", "0")
            .assert_empty();
        ctx.state.get_diff(tile, bel, "OCEUSED", "0").assert_empty();
        ctx.state.get_diff(tile, bel, "TCEUSED", "0").assert_empty();
        let osrused = ctx.extract_bit(tile, bel, "OSRUSED", "0");
        let tsrused = ctx.extract_bit(tile, bel, "TSRUSED", "0");
        ctx.tiledb.insert(tile, bel, "OFF_SR_USED", osrused);
        ctx.tiledb.insert(tile, bel, "TFF_SR_USED", tsrused);

        let mut diffs = vec![];
        for val in ["2", "3", "4", "5", "6", "7", "8"] {
            diffs.push((
                val,
                val,
                ctx.state.get_diff(tile, bel, "DATA_WIDTH.SDR", val),
            ));
        }
        for (val, ratio) in [("4", "2"), ("6", "3"), ("8", "4"), ("10", "5")] {
            diffs.push((
                val,
                ratio,
                ctx.state.get_diff(tile, bel, "DATA_WIDTH.DDR", val),
            ));
        }
        for (_, _, diff) in &mut diffs {
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_SR_USED"), true, false);
        }
        let mut ddr3_byp = diffs[0].2.clone();
        for (_, _, diff) in &diffs {
            ddr3_byp.bits.retain(|k, _| diff.bits.contains_key(k));
        }
        let ddr3_byp = xlat_bit(ddr3_byp);
        for (_, _, diff) in &mut diffs {
            diff.apply_bit_diff(&ddr3_byp, true, false);
        }
        ctx.tiledb.insert(tile, bel, "DDR3_BYPASS", ddr3_byp);
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
        ctx.tiledb
            .insert(tile, bel, "DATA_WIDTH", xlat_enum(diffs_width));
        ctx.tiledb
            .insert(tile, bel, "CLK_RATIO", xlat_enum(diffs_ratio));

        let diff_buf = !ctx.state.get_diff(tile, bel, "DATA_RATE_OQ", "SDR");
        let diff_ddr = ctx
            .state
            .get_diff(tile, bel, "DATA_RATE_OQ", "DDR")
            .combine(&diff_buf);
        ctx.tiledb.insert(
            tile,
            bel,
            "OMUX",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                ("D1", diff_buf),
                ("SERDES_SDR", diff_sdr),
                ("SERDES_DDR", diff_ddr),
                ("FF", ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#FF")),
                ("DDR", ctx.state.get_diff(tile, bel, "OUTFFTYPE", "DDR")),
                (
                    "LATCH",
                    ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#LATCH"),
                ),
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
                ("SERDES_DDR", diff_ddr),
                ("FF", ctx.state.get_diff(tile, bel, "TFFTYPE", "#FF")),
                ("DDR", ctx.state.get_diff(tile, bel, "TFFTYPE", "DDR")),
                ("LATCH", ctx.state.get_diff(tile, bel, "TFFTYPE", "#LATCH")),
            ]),
        );

        ctx.state
            .get_diff(tile, bel, "INTERFACE_TYPE", "DEFAULT")
            .assert_empty();
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3");

        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "OMUX"), "SERDES_DDR", "NONE");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DATA_WIDTH"), "4", "NONE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_SR_USED"), true, false);
        assert_eq!(diff.bits.len(), 1);
        ctx.tiledb.insert(
            tile,
            bel,
            "INTERFACE_TYPE",
            xlat_enum(vec![("DEFAULT", Diff::default()), ("MEMORY_DDR3", diff)]),
        );

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum_bool(tile, bel, "SELFHEAL", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "TRISTATE_WIDTH", &["1", "4"]);
        ctx.collect_enum_bool(tile, bel, "WC_DELAY", "0", "1");
        ctx.collect_enum_bool(tile, bel, "DDR3_DATA", "0", "1");
        ctx.collect_enum_bool(tile, bel, "ODELAY_USED", "0", "1");
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
            ctx.collect_bitvec(tile, bel, attr, "");
        }

        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE_FDBK", "FALSE", "TRUE");
        ctx.collect_enum_default(tile, bel, "MISR_CLK_SELECT", &["CLK1", "CLK2"], "NONE");

        let mut present_ologic = ctx.state.get_diff(tile, bel, "PRESENT", "OLOGIC");
        present_ologic.apply_bit_diff(ctx.tiledb.item(tile, bel, "DDR3_BYPASS"), true, false);
        present_ologic.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "TFF_SRVAL"), 0, 7);
        present_ologic.apply_enum_diff(ctx.tiledb.item(tile, bel, "TMUX"), "T1", "NONE");
        present_ologic.assert_empty();

        let mut present_oserdes = ctx.state.get_diff(tile, bel, "PRESENT", "OSERDES");
        present_oserdes.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "OFF_SRVAL"), 0, 7);
        present_oserdes.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "TFF_SRVAL"), 0, 7);
        present_oserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "OFF_INIT"), false, true);
        present_oserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "TFF_INIT"), false, true);
        present_oserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.CLKPERF"), false, true);
        present_oserdes.apply_enum_diff(ctx.tiledb.item(tile, bel, "OMUX"), "D1", "NONE");
        present_oserdes.apply_enum_diff(ctx.tiledb.item(tile, bel, "TMUX"), "T1", "NONE");
        present_oserdes.assert_empty();

        let mut vals = vec![];
        for j in 0..12 {
            vals.push(format!("HCLK{j}"));
        }
        for j in 0..6 {
            vals.push(format!("RCLK{j}"));
        }
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKDIV", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKDIVB", &vals, "NONE", OcdMode::Mux);
        for j in 0..8 {
            vals.push(format!("IOCLK{j}"));
        }
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLK", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_ocd(tile, bel, "MUX.CLKB", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum(tile, bel, "MUX.CLKPERF", &["OCLK0", "OCLK1"]);
    }
    let mut diff = ctx.state.get_diff(tile, "OLOGIC_COMMON", "MISR_RESET", "1");
    let diff1 = diff.split_bits_by(|bit| bit.tile > 0);
    ctx.tiledb
        .insert(tile, "OLOGIC0", "MISR_RESET", xlat_bit(diff));
    ctx.tiledb
        .insert(tile, "OLOGIC1", "MISR_RESET", xlat_bit(diff1));
    for i in 0..2 {
        let bel = &format!("IODELAY{i}");
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_inv(tile, bel, "C");
        ctx.collect_inv(tile, bel, "DATAIN");
        ctx.collect_inv(tile, bel, "IDATAIN");
        ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CINVCTRL_SEL", "FALSE", "TRUE");
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
        let item = ctx.extract_bitvec(tile, bel, "ODELAY_VALUE", "");
        ctx.tiledb.insert(tile, bel, "ALT_DELAY_VALUE", item);
        let (_, _, mut diff) = Diff::split(
            ctx.state.peek_diff(tile, bel, "DELAY_SRC", "I").clone(),
            ctx.state.peek_diff(tile, bel, "DELAY_SRC", "O").clone(),
        );
        diff.discard_bits(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"));
        ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff));
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["I", "IO", "O", "DATAIN", "CLKIN", "DELAYCHAIN_OSC"] {
            let mut diff = ctx.state.get_diff(tile, bel, "DELAY_SRC", val);
            diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
            diffs.push((val, diff));
        }
        ctx.tiledb.insert(tile, bel, "DELAY_SRC", xlat_enum(diffs));

        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "I_DEFAULT");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "I", "NONE");
        let val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"),
            &bitvec![1; 5],
            &mut diff,
        );
        ctx.tiledb
            .insert_device_data(&ctx.device.name, "IODELAY:DEFAULT_IDELAY_VALUE", val);
        let val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "IDELAY_VALUE_INIT"),
            &bitvec![0; 5],
            &mut diff,
        );
        ctx.tiledb
            .insert_device_data(&ctx.device.name, "IODELAY:DEFAULT_IDELAY_VALUE", val);
        ctx.tiledb.insert(tile, bel, "EXTRA_DELAY", xlat_bit(diff));

        let mut diffs = vec![];
        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "I_FIXED");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "I_VARIABLE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "I_VAR_LOADABLE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VAR_LOADABLE", diff));

        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "O_FIXED");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "O_VARIABLE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "O_VAR_LOADABLE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VAR_LOADABLE", diff));

        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "IO_FIXED");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "I_FIXED_O_VARIABLE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE_SWAPPED", diff));
        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "I_VARIABLE_O_FIXED");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.state.get_diff(tile, bel, "MODE", "IO_VAR_LOADABLE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("IO_VAR_LOADABLE", diff));
        ctx.tiledb.insert(tile, bel, "DELAY_TYPE", xlat_enum(diffs));
    }
    let mut present_vr = ctx.state.get_diff(tile, "IOB_COMMON", "PRESENT", "VR");
    for i in 0..2 {
        let bel = &format!("IOB{i}");
        ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
        ctx.collect_enum_bool(tile, bel, "OUTPUT_DELAY", "0", "1");
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "IOB");
        let diff = ctx
            .state
            .get_diff(tile, bel, "PRESENT", "IPAD")
            .combine(&!&present);
        ctx.tiledb.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
        let diff = ctx
            .state
            .get_diff(tile, bel, "PRESENT", "IOB.CONTINUOUS")
            .combine(&!&present);
        ctx.tiledb
            .insert(tile, bel, "DCIUPDATEMODE_ASREQUIRED", xlat_bit(!diff));
        present.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

        let oprog = ctx.extract_bitvec(tile, bel, "OPROGRAMMING", "");
        let lvds = TileItem::from_bitvec(oprog.bits[0..9].to_vec(), false);
        let dci_t = TileItem::from_bit(oprog.bits[11], false);
        let dci_mode = TileItem {
            bits: oprog.bits[12..15].to_vec(),
            kind: TileItemKind::Enum {
                values: [
                    ("NONE".into(), bitvec![0, 0, 0]),
                    ("OUTPUT".into(), bitvec![1, 0, 0]),
                    ("OUTPUT_HALF".into(), bitvec![0, 1, 0]),
                    ("TERM_VCC".into(), bitvec![1, 1, 0]),
                    ("TERM_SPLIT".into(), bitvec![0, 0, 1]),
                ]
                .into_iter()
                .collect(),
            },
        };
        let output_misc = TileItem::from_bitvec(oprog.bits[15..19].to_vec(), false);
        let dci_misc = TileItem::from_bitvec(oprog.bits[9..11].to_vec(), false);
        let pdrive_bits = oprog.bits[19..25].to_vec();
        let ndrive_bits = oprog.bits[25..31].to_vec();
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
        let (pslew_bits, nslew_bits) = if i == 0 {
            (
                vec![
                    FeatureBit::new(0, 41, 39),
                    FeatureBit::new(0, 41, 31),
                    FeatureBit::new(0, 41, 27),
                    FeatureBit::new(0, 40, 20),
                    FeatureBit::new(0, 40, 10),
                ],
                vec![
                    FeatureBit::new(0, 40, 44),
                    FeatureBit::new(0, 40, 30),
                    FeatureBit::new(0, 40, 32),
                    FeatureBit::new(0, 41, 17),
                    FeatureBit::new(0, 41, 43),
                ],
            )
        } else {
            (
                vec![
                    FeatureBit::new(1, 40, 24),
                    FeatureBit::new(1, 40, 32),
                    FeatureBit::new(1, 40, 36),
                    FeatureBit::new(1, 41, 43),
                    FeatureBit::new(1, 41, 53),
                ],
                vec![
                    FeatureBit::new(1, 41, 19),
                    FeatureBit::new(1, 41, 33),
                    FeatureBit::new(1, 41, 31),
                    FeatureBit::new(1, 40, 46),
                    FeatureBit::new(1, 40, 20),
                ],
            )
        };
        let pslew = TileItem::from_bitvec(pslew_bits, false);
        let nslew = TileItem::from_bitvec(nslew_bits, false);

        let mut diff = ctx
            .state
            .peek_diff(tile, bel, "OSTD", "LVCMOS25.12.SLOW")
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
        let diff_cmos12 = ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS12.LP");
        let diff_vref_lp = ctx.state.peek_diff(tile, bel, "ISTD", "HSTL_I.LP");
        let diff_vref_hp = ctx.state.peek_diff(tile, bel, "ISTD", "HSTL_I.HP");
        let mut diff_diff_lp = ctx.state.peek_diff(tile, bel, "ISTD", "LVDS_25.LP").clone();
        let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.tile == i);
        let mut diff_diff_hp = ctx.state.peek_diff(tile, bel, "ISTD", "LVDS_25.HP").clone();
        let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.tile == i);
        ctx.tiledb.insert(
            tile,
            bel,
            "IBUF_MODE",
            xlat_enum(vec![
                ("OFF", Diff::default()),
                ("CMOS", diff_cmos.clone()),
                ("CMOS12", diff_cmos12.clone()),
                ("VREF_LP", diff_vref_lp.clone()),
                ("VREF_HP", diff_vref_hp.clone()),
                ("DIFF_LP", diff_diff_lp),
                ("DIFF_HP", diff_diff_hp),
            ]),
        );

        for &std in IOSTDS {
            if std.diff != DiffKind::None {
                continue;
            }
            let (drives, slews) = if !std.drive.is_empty() {
                (std.drive, &["SLOW", "FAST"][..])
            } else {
                (&[""][..], &[""][..])
            };
            for &drive in drives {
                for &slew in slews {
                    let val = if drive.is_empty() {
                        std.name.to_string()
                    } else {
                        format!("{name}.{drive}.{slew}", name = std.name)
                    };
                    let mut diff = ctx.state.get_diff(tile, bel, "OSTD", val);
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
                                .insert_misc_data(format!("IOSTD:{attr}:{name}"), value);
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
                        let name = if drive.is_empty() {
                            stdname.to_string()
                        } else {
                            format!("{stdname}.{drive}.{slew}")
                        };
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{attr}:{name}"), value);
                    }
                    let value: BitVec = output_misc
                        .bits
                        .iter()
                        .map(|&bit| match diff.bits.remove(&bit) {
                            Some(true) => true,
                            None => false,
                            _ => unreachable!(),
                        })
                        .collect();
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:OUTPUT_MISC:{stdname}"), value);
                    match std.dci {
                        DciKind::None | DciKind::InputVcc | DciKind::InputSplit => {}
                        DciKind::Output => {
                            diff.apply_enum_diff(&dci_mode, "OUTPUT", "NONE");
                        }
                        DciKind::OutputHalf => {
                            diff.apply_enum_diff(&dci_mode, "OUTPUT_HALF", "NONE");
                        }
                        DciKind::BiVcc => {
                            diff.apply_enum_diff(&dci_mode, "TERM_VCC", "NONE");
                            diff.apply_bitvec_diff(&dci_misc, &bitvec![1, 1], &bitvec![0, 0]);
                        }
                        DciKind::BiSplit => {
                            diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                        }
                        DciKind::BiSplitT => {
                            diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                            diff.apply_bit_diff(&dci_t, true, false);
                        }
                    }
                    diff.assert_empty();
                }
            }
        }

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
            ctx.tiledb
                .insert_misc_data(format!("IOSTD:{attr}:VR"), value);
        }
        present_vr.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");
        present_vr.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");

        if i == 0 {
            let mut present_vref = ctx.state.get_diff(tile, bel, "PRESENT", "VREF");
            present_vref.apply_bit_diff(ctx.tiledb.item(tile, bel, "VREF_SYSMON"), true, false);
            present_vref.apply_enum_diff(ctx.tiledb.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

            for (attr, bits, invert) in [
                ("PDRIVE", &pdrive_bits, &pdrive_invert),
                ("NDRIVE", &ndrive_bits, &ndrive_invert),
                ("PSLEW", &pslew.bits, &bitvec![0; 5]),
                ("NSLEW", &nslew.bits, &bitvec![0; 5]),
            ] {
                let value: BitVec = bits
                    .iter()
                    .zip(invert.iter())
                    .map(|(&bit, inv)| match present_vref.bits.remove(&bit) {
                        Some(true) => !*inv,
                        None => *inv,
                        _ => unreachable!(),
                    })
                    .collect();
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:{attr}:OFF"), value);
            }
            present_vref.assert_empty();
        }

        ctx.tiledb
            .insert_misc_data("IOSTD:OUTPUT_MISC:OFF", bitvec![0; 4]);
        ctx.tiledb
            .insert_misc_data("IOSTD:LVDS_T:OFF", bitvec![0; 9]);
        ctx.tiledb
            .insert_misc_data("IOSTD:LVDS_C:OFF", bitvec![0; 9]);
        ctx.tiledb
            .insert_misc_data("IOSTD:PDRIVE:OFF", bitvec![0; 6]);
        ctx.tiledb
            .insert_misc_data("IOSTD:NDRIVE:OFF", bitvec![0; 6]);
        ctx.tiledb
            .insert_misc_data("IOSTD:PSLEW:OFF", bitvec![0; 5]);
        ctx.tiledb
            .insert_misc_data("IOSTD:NSLEW:OFF", bitvec![0; 5]);
        ctx.tiledb.insert(tile, bel, "LVDS", lvds);
        ctx.tiledb.insert(tile, bel, "DCI_T", dci_t);
        ctx.tiledb.insert(tile, bel, "DCI_MODE", dci_mode);
        ctx.tiledb.insert(tile, bel, "OUTPUT_MISC", output_misc);
        ctx.tiledb.insert(tile, bel, "DCI_MISC", dci_misc);
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
    let diff1 = present_vr.split_bits_by(|bit| bit.tile == 1);
    ctx.tiledb.insert(tile, "IOB0", "VR", xlat_bit(present_vr));
    ctx.tiledb.insert(tile, "IOB1", "VR", xlat_bit(diff1));
    // ISE bug.
    let mut diff = ctx.state.get_diff(tile, "IOB0", "PULL_DYNAMIC", "1");
    let diff1 = diff.split_bits_by(|bit| bit.tile == 1);
    ctx.tiledb
        .insert(tile, "IOB0", "PULL_DYNAMIC", xlat_bit(diff));
    ctx.tiledb
        .insert(tile, "IOB1", "PULL_DYNAMIC", xlat_bit(diff1));
    ctx.state
        .get_diff(tile, "IOB1", "PULL_DYNAMIC", "1")
        .assert_empty();

    for i in 0..2 {
        let bel = &format!("IOB{i}");
        for &std in IOSTDS {
            for lp in ["HP", "LP"] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                if std.diff != DiffKind::None {
                    for bel in ["IOB0", "IOB1"] {
                        match std.dci {
                            DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                            DciKind::InputVcc | DciKind::BiVcc => {
                                diff.apply_enum_diff(
                                    ctx.tiledb.item(tile, bel, "DCI_MODE"),
                                    "TERM_VCC",
                                    "NONE",
                                );
                                diff.apply_bitvec_diff(
                                    ctx.tiledb.item(tile, bel, "DCI_MISC"),
                                    &bitvec![1, 1],
                                    &bitvec![0, 0],
                                );
                            }
                            DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                                diff.apply_enum_diff(
                                    ctx.tiledb.item(tile, bel, "DCI_MODE"),
                                    "TERM_SPLIT",
                                    "NONE",
                                );
                            }
                        }
                        diff.apply_enum_diff(
                            ctx.tiledb.item(tile, bel, "IBUF_MODE"),
                            if lp == "LP" { "DIFF_LP" } else { "DIFF_HP" },
                            "OFF",
                        );
                    }
                    diff.assert_empty();
                } else {
                    match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                        DciKind::InputVcc | DciKind::BiVcc => {
                            diff.apply_enum_diff(
                                ctx.tiledb.item(tile, bel, "DCI_MODE"),
                                "TERM_VCC",
                                "NONE",
                            );
                            diff.apply_bitvec_diff(
                                ctx.tiledb.item(tile, bel, "DCI_MISC"),
                                &bitvec![1, 1],
                                &bitvec![0, 0],
                            );
                        }
                        DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                            diff.apply_enum_diff(
                                ctx.tiledb.item(tile, bel, "DCI_MODE"),
                                "TERM_SPLIT",
                                "NONE",
                            );
                        }
                    }
                    let mode = if std.vref.is_some() {
                        if lp == "LP" {
                            "VREF_LP"
                        } else {
                            "VREF_HP"
                        }
                    } else if std.vcco == Some(1200) {
                        "CMOS12"
                    } else {
                        "CMOS"
                    };
                    diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IBUF_MODE"), mode, "OFF");
                    diff.assert_empty();
                }
            }
            if std.diff == DiffKind::True && i == 0 {
                let mut diff = ctx.state.get_diff(tile, bel, "DIFF_TERM", std.name);
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
                    .insert_misc_data(format!("IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                diff.assert_empty();
                let mut diff = ctx.state.get_diff(tile, bel, "DIFF_TERM_DYNAMIC", std.name);
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
                    .insert_misc_data(format!("IOSTD:LVDS_T:TERM_DYNAMIC_{}", std.name), val_t);
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:LVDS_C:TERM_DYNAMIC_{}", std.name), val_c);
                diff.assert_empty();
            }
            if std.diff == DiffKind::True && i == 1 {
                let mut diff = ctx.state.get_diff(tile, bel, "OSTD", std.name);
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
                    .insert_misc_data(format!("IOSTD:LVDS_T:OUTPUT_{}", std.name), val_t);
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:LVDS_C:OUTPUT_{}", std.name), val_c);
                diff.apply_bitvec_diff(
                    ctx.tiledb.item(tile, "IOB1", "OUTPUT_ENABLE"),
                    &bitvec![1; 2],
                    &bitvec![0; 2],
                );
                diff.assert_empty();
            }
            if std.diff == DiffKind::Pseudo && i == 1 {
                let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                let mut diff = ctx.state.get_diff(tile, bel, "OSTD", std.name);
                for bel in ["IOB0", "IOB1"] {
                    diff.apply_bitvec_diff(
                        ctx.tiledb.item(tile, bel, "OUTPUT_ENABLE"),
                        &bitvec![1; 2],
                        &bitvec![0; 2],
                    );
                    for attr in ["PDRIVE", "NDRIVE", "PSLEW", "NSLEW", "OUTPUT_MISC"] {
                        let item = ctx.tiledb.item(tile, bel, attr);
                        let value = extract_bitvec_val_part(
                            item,
                            &BitVec::repeat(false, item.bits.len()),
                            &mut diff,
                        );
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{attr}:{stdname}"), value);
                    }
                    let dci_mode = ctx.tiledb.item(tile, bel, "DCI_MODE");
                    let dci_misc = ctx.tiledb.item(tile, bel, "DCI_MISC");
                    let dci_t = ctx.tiledb.item(tile, bel, "DCI_T");
                    match std.dci {
                        DciKind::None | DciKind::InputVcc | DciKind::InputSplit => {}
                        DciKind::Output => {
                            diff.apply_enum_diff(dci_mode, "OUTPUT", "NONE");
                        }
                        DciKind::OutputHalf => {
                            diff.apply_enum_diff(dci_mode, "OUTPUT_HALF", "NONE");
                        }
                        DciKind::BiVcc => {
                            diff.apply_enum_diff(dci_mode, "TERM_VCC", "NONE");
                            diff.apply_bitvec_diff(dci_misc, &bitvec![1, 1], &bitvec![0, 0]);
                        }
                        DciKind::BiSplit => {
                            diff.apply_enum_diff(dci_mode, "TERM_SPLIT", "NONE");
                        }
                        DciKind::BiSplitT => {
                            diff.apply_enum_diff(dci_mode, "TERM_SPLIT", "NONE");
                            diff.apply_bit_diff(dci_t, true, false);
                        }
                    }
                }
                ctx.tiledb.insert(
                    tile,
                    "IOB0",
                    "OMUX",
                    xlat_enum(vec![("O", Diff::default()), ("OTHER_O_INV", diff)]),
                );
            }
        }
    }

    let tile = "HCLK_IOI";
    let lvdsbias = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 42, 30),
            FeatureBit::new(0, 42, 28),
            FeatureBit::new(0, 42, 27),
            FeatureBit::new(0, 42, 26),
            FeatureBit::new(0, 42, 25),
            FeatureBit::new(0, 42, 24),
            FeatureBit::new(0, 42, 23),
            FeatureBit::new(0, 42, 22),
            FeatureBit::new(0, 42, 21),
            FeatureBit::new(0, 42, 20),
            FeatureBit::new(0, 42, 19),
            FeatureBit::new(0, 42, 18),
            FeatureBit::new(0, 42, 17),
            FeatureBit::new(0, 42, 16),
            FeatureBit::new(0, 42, 15),
            FeatureBit::new(0, 42, 14),
            FeatureBit::new(0, 41, 28),
        ],
        false,
    );
    let bel = "DCI";
    let dci_en = ctx.state.get_diff(tile, bel, "ENABLE", "1");
    let test_en = ctx
        .state
        .get_diff(tile, bel, "TEST_ENABLE", "1")
        .combine(&!&dci_en);
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

    let dci_en = xlat_bit(dci_en);
    let nref_output = TileItem::from_bitvec(
        vec![FeatureBit::new(0, 40, 16), FeatureBit::new(0, 40, 17)],
        false,
    );
    let pref_output = TileItem::from_bitvec(
        vec![FeatureBit::new(0, 41, 14), FeatureBit::new(0, 41, 15)],
        false,
    );
    let nref_output_half = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 40, 18),
            FeatureBit::new(0, 40, 19),
            FeatureBit::new(0, 40, 20),
        ],
        false,
    );
    let pref_output_half = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 41, 16),
            FeatureBit::new(0, 41, 17),
            FeatureBit::new(0, 41, 18),
        ],
        false,
    );
    let pref_term_vcc = TileItem::from_bitvec(
        vec![FeatureBit::new(0, 40, 14), FeatureBit::new(0, 40, 15)],
        false,
    );
    let pmask_term_vcc = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 43, 14),
            FeatureBit::new(0, 43, 27),
            FeatureBit::new(0, 43, 28),
            FeatureBit::new(0, 43, 29),
            FeatureBit::new(0, 43, 30),
            FeatureBit::new(0, 43, 31),
        ],
        false,
    );
    let nref_term_split = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 40, 23),
            FeatureBit::new(0, 40, 24),
            FeatureBit::new(0, 40, 25),
        ],
        false,
    );
    let pref_term_split = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 41, 19),
            FeatureBit::new(0, 41, 20),
            FeatureBit::new(0, 41, 21),
        ],
        false,
    );
    let pmask_term_split = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 43, 21),
            FeatureBit::new(0, 43, 22),
            FeatureBit::new(0, 43, 23),
            FeatureBit::new(0, 43, 24),
            FeatureBit::new(0, 43, 25),
            FeatureBit::new(0, 43, 26),
        ],
        false,
    );
    let nmask_term_split = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 43, 15),
            FeatureBit::new(0, 43, 16),
            FeatureBit::new(0, 43, 17),
            FeatureBit::new(0, 43, 18),
            FeatureBit::new(0, 43, 19),
            FeatureBit::new(0, 43, 20),
        ],
        false,
    );
    ctx.collect_enum_default(
        tile,
        "INTERNAL_VREF",
        "VREF",
        &["600", "750", "900", "1100", "1250"],
        "OFF",
    );
    for std in IOSTDS {
        if std.diff == DiffKind::True {
            let bel = "LVDS";
            let diff = ctx.state.get_diff(tile, bel, "STD", std.name);
            let val = extract_bitvec_val(&lvdsbias, &bitvec![0; 17], diff);
            ctx.tiledb
                .insert_misc_data(format!("IOSTD:LVDSBIAS:{}", std.name), val);
        }
        if std.dci != DciKind::None {
            let bel = "DCI";
            let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
            let mut diff = ctx.state.get_diff(tile, bel, "STD", std.name);
            match std.dci {
                DciKind::Output => {
                    let val = extract_bitvec_val_part(&nref_output, &bitvec![0; 2], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:NREF_OUTPUT:{stdname}"), val);
                    let val = extract_bitvec_val_part(&pref_output, &bitvec![0; 2], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PREF_OUTPUT:{stdname}"), val);
                }
                DciKind::OutputHalf => {
                    let val = extract_bitvec_val_part(&nref_output_half, &bitvec![0; 3], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:NREF_OUTPUT_HALF:{stdname}"), val);
                    let val = extract_bitvec_val_part(&pref_output_half, &bitvec![0; 3], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PREF_OUTPUT_HALF:{stdname}"), val);
                }
                DciKind::InputVcc | DciKind::BiVcc => {
                    let val = extract_bitvec_val_part(&pref_term_vcc, &bitvec![0; 2], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PREF_TERM_VCC:{stdname}"), val);
                    let val = extract_bitvec_val_part(&pmask_term_vcc, &bitvec![0; 6], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_VCC:{stdname}"), val);
                }
                DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                    let val = extract_bitvec_val_part(&nref_term_split, &bitvec![0; 3], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:NREF_TERM_SPLIT:{stdname}"), val);
                    let val = extract_bitvec_val_part(&pref_term_split, &bitvec![0; 3], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PREF_TERM_SPLIT:{stdname}"), val);
                    let val = extract_bitvec_val_part(&pmask_term_split, &bitvec![0; 6], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_SPLIT:{stdname}"), val);
                    let val = extract_bitvec_val_part(&nmask_term_split, &bitvec![0; 6], &mut diff);
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:NMASK_TERM_SPLIT:{stdname}"), val);
                }
                _ => {}
            }
            diff.apply_bit_diff(&dci_en, true, false);
            diff.assert_empty();
        }
    }
    let bel = "LVDS";
    ctx.tiledb.insert(tile, bel, "LVDSBIAS", lvdsbias);
    ctx.tiledb
        .insert_misc_data("IOSTD:LVDSBIAS:OFF", bitvec![0; 17]);
    let bel = "DCI";
    ctx.tiledb.insert(tile, bel, "ENABLE", dci_en);
    ctx.tiledb.insert(tile, bel, "PREF_OUTPUT", pref_output);
    ctx.tiledb.insert(tile, bel, "NREF_OUTPUT", nref_output);
    ctx.tiledb
        .insert(tile, bel, "PREF_OUTPUT_HALF", pref_output_half);
    ctx.tiledb
        .insert(tile, bel, "NREF_OUTPUT_HALF", nref_output_half);
    ctx.tiledb.insert(tile, bel, "PREF_TERM_VCC", pref_term_vcc);
    ctx.tiledb
        .insert(tile, bel, "PREF_TERM_SPLIT", pref_term_split);
    ctx.tiledb
        .insert(tile, bel, "NREF_TERM_SPLIT", nref_term_split);

    ctx.tiledb
        .insert(tile, bel, "PMASK_TERM_VCC", pmask_term_vcc);
    ctx.tiledb
        .insert(tile, bel, "PMASK_TERM_SPLIT", pmask_term_split);
    ctx.tiledb
        .insert(tile, bel, "NMASK_TERM_SPLIT", nmask_term_split);
    ctx.collect_bit(tile, bel, "QUIET", "1");

    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PREF_OUTPUT:OFF", bitvec![0; 2]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:NREF_OUTPUT:OFF", bitvec![0; 2]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PREF_OUTPUT_HALF:OFF", bitvec![0; 3]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:NREF_OUTPUT_HALF:OFF", bitvec![0; 3]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PREF_TERM_VCC:OFF", bitvec![0; 2]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PMASK_TERM_VCC:OFF", bitvec![0; 6]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PREF_TERM_SPLIT:OFF", bitvec![0; 3]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:NREF_TERM_SPLIT:OFF", bitvec![0; 3]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PMASK_TERM_SPLIT:OFF", bitvec![0; 6]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:NMASK_TERM_SPLIT:OFF", bitvec![0; 6]);
    let tile = "CFG";
    let bel = "MISC";
    ctx.collect_bit_wide(tile, bel, "DCI_CLK_ENABLE", "1");
}
