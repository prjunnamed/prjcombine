use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::{TileBit, TileItem, TileItemKind};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::{
        extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_wide, xlat_bitvec,
        xlat_enum, xlat_enum_ocd, CollectorCtx, Diff, OcdMode,
    },
    fgen::{BelKV, ExtraFeature, ExtraFeatureKind, TileBits, TileKV, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_inv, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_one,
    fuzz_one_extras,
    io::iostd::DiffKind,
};

use super::iostd::{DciKind, Iostd};

const IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
    Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12", "16"]),
    Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6", "8"]),
    Iostd::cmos("PCI33_3", 3300, &[]),
    Iostd::cmos("PCI66_3", 3300, &[]),
    Iostd::cmos("PCIX", 3300, &[]),
    Iostd::odci("LVDCI_33", 3300),
    Iostd::odci("LVDCI_25", 2500),
    Iostd::odci("LVDCI_18", 1800),
    Iostd::odci("LVDCI_15", 1500),
    Iostd::odci_half("LVDCI_DV2_25", 2500),
    Iostd::odci_half("LVDCI_DV2_18", 1800),
    Iostd::odci_half("LVDCI_DV2_15", 1500),
    Iostd::odci_vref("HSLVDCI_33", 3300, 1650),
    Iostd::odci_vref("HSLVDCI_25", 2500, 1250),
    Iostd::odci_vref("HSLVDCI_18", 1800, 900),
    Iostd::odci_vref("HSLVDCI_15", 1500, 750),
    Iostd::vref_od("GTL", 800),
    Iostd::vref_od("GTLP", 1000),
    Iostd::vref("SSTL2_I", 2500, 1250),
    Iostd::vref("SSTL2_II", 2500, 1250),
    Iostd::vref("SSTL18_I", 1800, 900),
    Iostd::vref("SSTL18_II", 1800, 900),
    Iostd::vref("HSTL_I_18", 1800, 900),
    Iostd::vref("HSTL_II_18", 1800, 900),
    Iostd::vref("HSTL_III_18", 1800, 1080),
    Iostd::vref("HSTL_IV_18", 1800, 1080),
    Iostd::vref("HSTL_I", 1500, 750),
    Iostd::vref("HSTL_II", 1500, 750),
    Iostd::vref("HSTL_III", 1500, 900),
    Iostd::vref("HSTL_IV", 1500, 900),
    Iostd::vref("HSTL_I_12", 1200, 600),
    Iostd::vref_dci_od("GTL_DCI", 1200, 800),
    Iostd::vref_dci_od("GTLP_DCI", 1500, 1000),
    Iostd::vref_dci("SSTL2_I_DCI", 2500, 1250, DciKind::InputSplit),
    Iostd::vref_dci("SSTL2_II_DCI", 2500, 1250, DciKind::BiSplit),
    Iostd::vref_dci("SSTL2_II_T_DCI", 2500, 1250, DciKind::BiSplitT),
    Iostd::vref_dci("SSTL18_I_DCI", 1800, 900, DciKind::InputSplit),
    Iostd::vref_dci("SSTL18_II_DCI", 1800, 900, DciKind::BiSplit),
    Iostd::vref_dci("SSTL18_II_T_DCI", 1800, 900, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_I_DCI_18", 1800, 900, DciKind::InputSplit),
    Iostd::vref_dci("HSTL_II_DCI_18", 1800, 900, DciKind::BiSplit),
    Iostd::vref_dci("HSTL_II_T_DCI_18", 1800, 900, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_III_DCI_18", 1800, 1080, DciKind::InputVcc),
    Iostd::vref_dci("HSTL_IV_DCI_18", 1800, 1080, DciKind::BiVcc),
    Iostd::vref_dci("HSTL_I_DCI", 1500, 750, DciKind::InputSplit),
    Iostd::vref_dci("HSTL_II_DCI", 1500, 750, DciKind::BiSplit),
    Iostd::vref_dci("HSTL_II_T_DCI", 1500, 750, DciKind::BiSplitT),
    Iostd::vref_dci("HSTL_III_DCI", 1500, 900, DciKind::InputVcc),
    Iostd::vref_dci("HSTL_IV_DCI", 1500, 900, DciKind::BiVcc),
    Iostd::pseudo_diff("DIFF_SSTL2_I", 2500),
    Iostd::pseudo_diff("DIFF_SSTL2_II", 2500),
    Iostd::pseudo_diff("DIFF_SSTL18_I", 1800),
    Iostd::pseudo_diff("DIFF_SSTL18_II", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_I", 1500),
    Iostd::pseudo_diff("DIFF_HSTL_II", 1500),
    Iostd::pseudo_diff("BLVDS_25", 2500),
    Iostd::pseudo_diff("LVPECL_25", 2500),
    Iostd::pseudo_diff_dci("DIFF_SSTL2_I_DCI", 2500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL2_II_DCI", 2500, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_I_DCI", 1800, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_II_DCI", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_I_DCI_18", 1800, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI_18", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_I_DCI", 1500, DciKind::InputSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI", 1500, DciKind::BiSplit),
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
    let bel_idelayctrl = backend.egrid.db.nodes[hclk_ioi]
        .bels
        .get("IDELAYCTRL")
        .unwrap()
        .0;
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
            fuzz_enum!(ctx, "IDELAY_TYPE", ["DEFAULT"], [
                (mode "IODELAY"),
                (global_opt "LEGIDELAY", "ENABLE"),
                (bel_mode bel_other, "IODELAY"),
                (bel_attr bel_other, "IDELAY_VALUE", ""),
                (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
                (related TileRelation::Hclk(hclk_ioi),
                    (bel_mode bel_idelayctrl, "IDELAYCTRL"))
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
    let bel_ioi_clk = BelId::from_idx(8);

    {
        let ctx = FuzzCtx::new(session, backend, "IO", "IOI_CLK", TileBits::MainAuto);
        for i in 0..2 {
            for j in 0..2 {
                fuzz_one!(ctx, format!("MUX.ICLK{i}"), format!("CKINT{j}"), [
                    (mutex format!("MUX.ICLK{i}"), format!("CKINT{j}"))
                ], [
                    (pip (pin format!("CKINT{j}")), (pin format!("ICLK{i}")))
                ]);
            }
            for j in 0..4 {
                fuzz_one!(ctx, format!("MUX.ICLK{i}"), format!("IOCLK{j}"), [
                    (mutex format!("MUX.ICLK{i}"), format!("IOCLK{j}"))
                ], [
                    (pip (pin format!("IOCLK{j}")), (pin format!("ICLK{i}")))
                ]);
                fuzz_one!(ctx, format!("MUX.ICLK{i}"), format!("RCLK{j}"), [
                    (mutex format!("MUX.ICLK{i}"), format!("RCLK{j}"))
                ], [
                    (pip (pin format!("RCLK{j}")), (pin format!("ICLK{i}")))
                ]);
            }
            for j in 0..10 {
                fuzz_one!(ctx, format!("MUX.ICLK{i}"), format!("HCLK{j}"), [
                    (mutex format!("MUX.ICLK{i}"), format!("HCLK{j}"))
                ], [
                    (pip (pin format!("HCLK{j}")), (pin format!("ICLK{i}")))
                ]);
            }
        }
    }

    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "IO",
            format!("ILOGIC{i}"),
            TileBits::MainAuto,
        );
        let bel_ologic = BelId::from_idx(2 + i);
        let bel_iodelay = BelId::from_idx(4 + i);
        fuzz_one!(ctx, "PRESENT", "ILOGIC", [(bel_unused bel_ologic)], [(mode "ILOGIC")]);
        fuzz_one!(ctx, "PRESENT", "ISERDES", [(bel_unused bel_ologic)], [(mode "ISERDES")]);

        for (pin, pin_t, pin_c) in [("CLK", "CLK", "CLK_B"), ("CLKB", "CLKB_B", "CLKB")] {
            for j in 0..2 {
                fuzz_one!(ctx, format!("MUX.{pin}"), format!("ICLK{j}"), [
                    (tile_mutex "ICLK", "MUX"),
                    (mutex format!("MUX.{pin}"), format!("ICLK{j}"))
                ], [
                    (pip (bel_pin bel_ioi_clk, format!("ICLK{j}")), (pin pin))
                ]);
                fuzz_one!(ctx, format!("INV.ICLK{j}"), "0", [
                    (mode "ISERDES"),
                    (tile_mutex "ICLK", format!("INV.{pin}.{i}.{j}")),
                    (pip (bel_pin bel_ioi_clk, format!("ICLK{j}")), (pin pin)),
                    (pin pin)
                ], [
                    (attr format!("{pin}INV"), pin_t)
                ]);
                fuzz_one!(ctx, format!("INV.ICLK{j}"), "1", [
                    (mode "ISERDES"),
                    (tile_mutex "ICLK", format!("INV.{pin}.{i}.{j}")),
                    (pip (bel_pin bel_ioi_clk, format!("ICLK{j}")), (pin pin)),
                    (pin pin)
                ], [
                    (attr format!("{pin}INV"), pin_c)
                ]);
            }

            fuzz_inv!(ctx, "CLKDIV", [(mode "ISERDES"), (bel_unused bel_iodelay)]);

            fuzz_enum_suffix!(ctx, "OCLKINV", "SDR", ["OCLK", "OCLK_B"], [
                (mode "ISERDES"),
                (attr "INTERFACE_TYPE", "MEMORY"),
                (attr "DATA_RATE", "SDR"),
                (pin "OCLK")
            ]);
            fuzz_enum_suffix!(ctx, "OCLKINV", "DDR", ["OCLK", "OCLK_B"], [
                (mode "ISERDES"),
                (attr "INTERFACE_TYPE", "MEMORY"),
                (attr "DATA_RATE", "DDR"),
                (pin "OCLK")
            ]);

            fuzz_enum!(ctx, "SRUSED", ["0"], [
                (mode "ILOGIC"),
                (attr "IFFTYPE", "#FF"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "REVUSED", ["0"], [
                (mode "ILOGIC"),
                (attr "IFFTYPE", "#FF"),
                (pin "REV")
            ]);

            fuzz_enum!(ctx, "SERDES", ["FALSE", "TRUE"], [
                (mode "ISERDES"),
                (attr "DATA_WIDTH", "2")
            ]);
            fuzz_enum!(ctx, "SERDES_MODE", ["MASTER", "SLAVE"], [(mode "ISERDES")]);
            fuzz_enum!(ctx, "INTERFACE_TYPE", ["NETWORKING", "MEMORY"], [
                (mode "ISERDES")
            ]);
            fuzz_enum!(ctx, "DATA_WIDTH", ["2", "3", "4", "5", "6", "7", "8", "10"], [
                (mode "ISERDES"),
                (attr "SERDES", "FALSE")
            ]);
            fuzz_enum_suffix!(ctx, "BITSLIP_ENABLE", "SYNC", ["FALSE", "TRUE"], [
                (mode "ISERDES"),
                (attr "SRTYPE", "SYNC")
            ]);
            fuzz_enum_suffix!(ctx, "BITSLIP_ENABLE", "ASYNC", ["FALSE", "TRUE"], [
                (mode "ISERDES"),
                (attr "SRTYPE", "ASYNC")
            ]);
            fuzz_enum!(ctx, "NUM_CE", ["1", "2"], [
                (mode "ISERDES")
            ]);
            fuzz_enum!(ctx, "DATA_RATE", ["SDR", "DDR"], [
                (mode "ISERDES"),
                (attr "INIT_BITSLIPCNT", "1111"),
                (attr "INIT_RANK1_PARTIAL", "11111"),
                (attr "INIT_RANK2", "111111"),
                (attr "INIT_RANK3", "111111"),
                (attr "INIT_CE", "11")
            ]);
            fuzz_enum!(ctx, "DDR_CLK_EDGE", ["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"], [
                (mode "ISERDES")
            ]);

            fuzz_enum!(ctx, "DDR_CLK_EDGE", ["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"], [
                (mode "ILOGIC"),
                (attr "IFFTYPE", "DDR")
            ]);
            fuzz_enum!(ctx, "IFFTYPE", ["#FF", "#LATCH", "DDR"], [
                (mode "ILOGIC")
            ]);
            for attr in [
                "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
                "SRVAL_Q4",
            ] {
                fuzz_enum!(ctx, attr, ["0", "1"], [
                    (mode "ISERDES")
                ]);
            }

            fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
                (mode "ILOGIC"),
                (attr "IFFTYPE", "#FF")
            ]);
            fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
                (mode "ISERDES")
            ]);
            fuzz_multi_attr_bin!(ctx, "INIT_CE", 2, [
                (mode "ISERDES"),
                (attr "DATA_RATE", "SDR")
            ]);
            fuzz_multi_attr_bin!(ctx, "INIT_BITSLIPCNT", 4, [
                (mode "ISERDES"),
                (attr "DATA_RATE", "SDR")
            ]);
            fuzz_multi_attr_bin!(ctx, "INIT_RANK1_PARTIAL", 5, [
                (mode "ISERDES"),
                (attr "DATA_RATE", "SDR")
            ]);
            fuzz_multi_attr_bin!(ctx, "INIT_RANK2", 6, [
                (mode "ISERDES"),
                (attr "DATA_RATE", "SDR")
            ]);
            fuzz_multi_attr_bin!(ctx, "INIT_RANK3", 6, [
                (mode "ISERDES"),
                (attr "DATA_RATE", "SDR")
            ]);

            fuzz_enum!(ctx, "OFB_USED", ["FALSE", "TRUE"], [
                (mode "ISERDES"),
                (pin "OFB")
            ]);
            fuzz_enum!(ctx, "TFB_USED", ["FALSE", "TRUE"], [
                (mode "ISERDES"),
                (pin "TFB")
            ]);
            fuzz_enum!(ctx, "IOBDELAY", ["NONE", "IFD", "IBUF", "BOTH"], [
                (mode "ISERDES")
            ]);

            fuzz_enum!(ctx, "D2OBYP_SEL", ["GND", "T"], [
                (mode "ILOGIC"),
                (attr "IMUX", "0"),
                (attr "IDELMUX", "1"),
                (attr "IFFMUX", "#OFF"),
                (pin "D"),
                (pin "DDLY"),
                (pin "TFB"),
                (pin "OFB"),
                (pin "O")
            ]);
            fuzz_enum!(ctx, "D2OFFBYP_SEL", ["GND", "T"], [
                (mode "ILOGIC"),
                (attr "IFFMUX", "0"),
                (attr "IFFTYPE", "#FF"),
                (attr "IFFDELMUX", "1"),
                (attr "IMUX", "#OFF"),
                (pin "D"),
                (pin "DDLY"),
                (pin "TFB"),
                (pin "OFB")
            ]);
            fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                (mode "ILOGIC"),
                (attr "IDELMUX", "1"),
                (pin "D"),
                (pin "DDLY"),
                (pin "O"),
                (pin "TFB"),
                (pin "OFB")
            ]);
            fuzz_enum!(ctx, "IFFMUX", ["0", "1"], [
                (mode "ILOGIC"),
                (attr "IFFDELMUX", "1"),
                (attr "IFFTYPE", "#FF"),
                (pin "D"),
                (pin "DDLY"),
                (pin "TFB"),
                (pin "OFB")
            ]);
            fuzz_enum!(ctx, "IDELMUX", ["0", "1"], [
                (mode "ILOGIC"),
                (attr "IMUX", "1"),
                (attr "IFFMUX", "1"),
                (attr "IFFTYPE", "#FF"),
                (attr "IFFDELMUX", "0"),
                (pin "D"),
                (pin "DDLY"),
                (pin "O"),
                (pin "Q1"),
                (pin "TFB"),
                (pin "OFB")
            ]);
            fuzz_enum!(ctx, "IFFDELMUX", ["0", "1"], [
                (mode "ILOGIC"),
                (attr "IMUX", "1"),
                (attr "IFFMUX", "0"),
                (attr "IFFTYPE", "#FF"),
                (attr "IDELMUX", "0"),
                (attr "D2OFFBYP_SEL", "T"),
                (pin "D"),
                (pin "DDLY"),
                (pin "O"),
                (pin "Q1"),
                (pin "TFB"),
                (pin "OFB")
            ]);
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
        let bel_ilogic = BelId::from_idx(i);
        fuzz_one!(ctx, "PRESENT", "OLOGIC", [(bel_unused bel_ilogic)], [(mode "OLOGIC")]);
        fuzz_one!(ctx, "PRESENT", "OSERDES", [(bel_unused bel_ilogic)], [(mode "OSERDES")]);

        fuzz_enum_suffix!(ctx, "CLKINV", "SAME", ["CLK", "CLK_B"], [
            (mode "OLOGIC"),
            (attr "ODDR_CLK_EDGE", "SAME_EDGE"),
            (attr "OUTFFTYPE", "#FF"),
            (attr "OMUX", "OUTFF"),
            (pin "CLK"),
            (pin "OQ")
        ]);
        fuzz_enum_suffix!(ctx, "CLKINV", "OPPOSITE", ["CLK", "CLK_B"], [
            (mode "OLOGIC"),
            (attr "ODDR_CLK_EDGE", "OPPOSITE_EDGE"),
            (attr "OUTFFTYPE", "#FF"),
            (attr "OMUX", "OUTFF"),
            (pin "CLK"),
            (pin "OQ")
        ]);
        fuzz_enum!(ctx, "ODDR_CLK_EDGE", ["SAME_EDGE", "OPPOSITE_EDGE"], [
            (mode "OLOGIC")
        ]);
        fuzz_enum!(ctx, "TDDR_CLK_EDGE", ["SAME_EDGE", "OPPOSITE_EDGE"], [
            (mode "OLOGIC")
        ]);
        fuzz_enum_suffix!(ctx, "CLKINV", "SAME", ["CLK", "CLK_B"], [
            (mode "OSERDES"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "DDR_CLK_EDGE", "SAME_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);
        fuzz_enum_suffix!(ctx, "CLKINV", "OPPOSITE", ["CLK", "CLK_B"], [
            (mode "OSERDES"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "DDR_CLK_EDGE", "OPPOSITE_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);
        fuzz_enum!(ctx, "DDR_CLK_EDGE", ["SAME_EDGE", "OPPOSITE_EDGE"], [
            (mode "OSERDES")
        ]);

        fuzz_inv!(ctx, "CLKDIV", [(mode "OSERDES")]);

        for pin in ["D1", "D2", "D3", "D4", "D5", "D6"] {
            fuzz_inv!(ctx, pin, [(mode "OSERDES")]);
        }
        for pin in ["D1", "D2"] {
            fuzz_inv!(ctx, pin, [
                (mode "OLOGIC"),
                (attr "OUTFFTYPE", "DDR"),
                (attr "OMUX", "OUTFF"),
                (pin "OQ")
            ]);
        }

        fuzz_inv!(ctx, "T1", [
            (mode "OLOGIC"),
            (attr "TMUX", "T1"),
            (attr "T1USED", "0"),
            (pin "TQ")
        ]);
        fuzz_inv!(ctx, "T2", [
            (mode "OLOGIC"),
            (attr "TFFTYPE", "DDR"),
            (attr "TMUX", "TFF"),
            (pin "TQ")
        ]);
        fuzz_inv!(ctx, "T1", [
            (mode "OSERDES"),
            (attr "DATA_RATE_TQ", "BUF")
        ]);
        for pin in ["T2", "T3", "T4"] {
            fuzz_inv!(ctx, pin, [
                (mode "OSERDES")
            ]);
        }

        fuzz_enum!(ctx, "SRTYPE_OQ", ["SYNC", "ASYNC"], [
            (mode "OLOGIC"),
            (attr "OUTFFTYPE", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE_TQ", ["SYNC", "ASYNC"], [
            (mode "OLOGIC"),
            (attr "TFFTYPE", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
            (mode "OSERDES")
        ]);

        fuzz_enum_suffix!(ctx, "INIT_OQ", "OLOGIC", ["0", "1"], [(mode "OLOGIC")]);
        fuzz_enum_suffix!(ctx, "INIT_TQ", "OLOGIC", ["0", "1"], [(mode "OLOGIC")]);
        fuzz_enum_suffix!(ctx, "INIT_OQ", "OSERDES", ["0", "1"], [(mode "OSERDES")]);
        fuzz_enum_suffix!(ctx, "INIT_TQ", "OSERDES", ["0", "1"], [(mode "OSERDES")]);

        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OLOGIC", ["0", "1"], [(mode "OLOGIC")]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "FF", ["0", "1"], [
            (mode "OLOGIC"),
            (attr "TFFTYPE", "#FF"),
            (attr "TMUX", "TFF"),
            (pin "TQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "DDR", ["0", "1"], [
            (mode "OLOGIC"),
            (attr "TFFTYPE", "DDR"),
            (attr "TMUX", "TFF"),
            (pin "TQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OSERDES", ["0", "1"], [(mode "OSERDES")]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "OSERDES", ["0", "1"], [(mode "OSERDES")]);

        for attr in [
            "OSRUSED", "TSRUSED", "OREVUSED", "TREVUSED", "OCEUSED", "TCEUSED",
        ] {
            fuzz_enum!(ctx, attr, ["0"], [
                (mode "OLOGIC"),
                (attr "OUTFFTYPE", "#FF"),
                (attr "TFFTYPE", "#FF"),
                (pin "OCE"),
                (pin "TCE"),
                (pin "REV"),
                (pin "SR")
            ]);
        }

        fuzz_enum!(ctx, "OUTFFTYPE", ["#FF", "#LATCH", "DDR"], [
            (mode "OLOGIC"),
            (pin "OQ")
        ]);
        fuzz_enum!(ctx, "TFFTYPE", ["#FF", "#LATCH", "DDR"], [
            (mode "OLOGIC"),
            (pin "TQ")
        ]);

        fuzz_enum!(ctx, "DATA_RATE_OQ", ["SDR", "DDR"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "DATA_RATE_TQ", ["BUF", "SDR", "DDR"], [
            (mode "OSERDES"),
            (attr "T1INV", "T1"),
            (pin "T1")
        ]);

        fuzz_enum!(ctx, "OMUX", ["D1", "OUTFF"], [
            (mode "OLOGIC"),
            (attr "OSRUSED", "#OFF"),
            (attr "OREVUSED", "#OFF"),
            (attr "OUTFFTYPE", "#FF"),
            (attr "O1USED", "0"),
            (attr "D1INV", "D1"),
            (pin "D1"),
            (pin "OQ")
        ]);
        fuzz_enum!(ctx, "TMUX", ["T1", "TFF"], [
            (mode "OLOGIC"),
            (attr "TSRUSED", "#OFF"),
            (attr "TREVUSED", "#OFF"),
            (attr "TFFTYPE", "#FF"),
            (attr "T1USED", "0"),
            (attr "T1INV", "T1"),
            (pin "T1"),
            (pin "TQ")
        ]);

        fuzz_enum!(ctx, "MISR_ENABLE", ["FALSE", "TRUE"], [
            (mode "OLOGIC"),
            (global_opt "ENABLEMISR", "Y")
        ]);
        fuzz_enum!(ctx, "MISR_ENABLE_FDBK", ["FALSE", "TRUE"], [
            (mode "OLOGIC"),
            (global_opt "ENABLEMISR", "Y")
        ]);
        fuzz_enum!(ctx, "MISR_CLK_SELECT", ["CLK1", "CLK2"], [
            (mode "OLOGIC"),
            (global_opt "ENABLEMISR", "Y")
        ]);

        fuzz_enum!(ctx, "SERDES", ["FALSE", "TRUE"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "SERDES_MODE", ["SLAVE", "MASTER"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "TRISTATE_WIDTH", ["1", "4"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "DATA_WIDTH", ["2", "3", "4", "5", "6", "7", "8", "10"], [
            (mode "OSERDES")
        ]);
        fuzz_multi_attr_bin!(ctx, "INIT_LOADCNT", 4, [(mode "OSERDES")]);

        fuzz_one!(ctx, "MUX.CLK", "CKINT", [
            (mutex "MUX.CLK", "CKINT")
        ], [
            (pip (pin "CKINT"), (pin "CLKMUX"))
        ]);
        fuzz_one!(ctx, "MUX.CLKDIV", "CKINT", [
            (mutex "MUX.CLKDIV", "CKINT")
        ], [
            (pip (pin "CKINT_DIV"), (pin "CLKDIVMUX"))
        ]);
        for i in 0..4 {
            fuzz_one!(ctx, "MUX.CLK", format!("IOCLK{i}"), [
                (mutex "MUX.CLK", format!("IOCLK{i}"))
            ], [
                (pip (bel_pin bel_ioi_clk, format!("IOCLK{i}")), (pin "CLKMUX"))
            ]);
            fuzz_one!(ctx, "MUX.CLK", format!("RCLK{i}"), [
                (mutex "MUX.CLK", format!("RCLK{i}"))
            ], [
                (pip (bel_pin bel_ioi_clk, format!("RCLK{i}")), (pin "CLKMUX"))
            ]);
            fuzz_one!(ctx, "MUX.CLKDIV", format!("RCLK{i}"), [
                (mutex "MUX.CLKDIV", format!("RCLK{i}"))
            ], [
                (pip (bel_pin bel_ioi_clk, format!("RCLK{i}")), (pin "CLKDIVMUX"))
            ]);
        }
        for i in 0..10 {
            fuzz_one!(ctx, "MUX.CLK", format!("HCLK{i}"), [
                (mutex "MUX.CLK", format!("HCLK{i}"))
            ], [
                (pip (bel_pin bel_ioi_clk, format!("HCLK{i}")), (pin "CLKMUX"))
            ]);
            fuzz_one!(ctx, "MUX.CLKDIV", format!("HCLK{i}"), [
                (mutex "MUX.CLKDIV", format!("HCLK{i}"))
            ], [
                (pip (bel_pin bel_ioi_clk, format!("HCLK{i}")), (pin "CLKDIVMUX"))
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
        let bel_ilogic = BelId::from_idx(i);
        let bel_other = BelId::from_idx(4 + (1 - i));
        let ctx = FuzzCtx::new(
            session,
            backend,
            "IO",
            format!("IODELAY{i}"),
            TileBits::MainAuto,
        );

        fuzz_one!(ctx, "PRESENT", "1", [
            (bel_mode bel_other, "IODELAY")
        ], [
            (mode "IODELAY")
        ]);

        fuzz_inv!(ctx, "C", [(mode "IODELAY"), (bel_unused bel_ilogic)]);
        fuzz_inv!(ctx, "DATAIN", [(mode "IODELAY")]);
        fuzz_enum!(ctx, "HIGH_PERFORMANCE_MODE", ["FALSE", "TRUE"], [(mode "IODELAY")]);
        fuzz_enum!(ctx, "DELAYCHAIN_OSC", ["FALSE", "TRUE"], [(mode "IODELAY")]);
        fuzz_enum!(ctx, "DELAY_SRC", ["I", "O", "IO", "DATAIN"], [(mode "IODELAY")]);
        fuzz_multi_attr_dec!(ctx, "IDELAY_VALUE", 6, [(mode "IODELAY")]);
        fuzz_multi_attr_dec!(ctx, "ODELAY_VALUE", 6, [(mode "IODELAY")]);

        fuzz_enum!(ctx, "IDELAY_TYPE", ["FIXED", "DEFAULT", "VARIABLE"], [
            (mode "IODELAY"),
            (global_opt "LEGIDELAY", "ENABLE"),
            (bel_mode bel_other, "IODELAY"),
            (bel_attr bel_other, "IDELAY_VALUE", ""),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL"))
        ]);
        fuzz_one!(ctx, "LEGIDELAY", "DISABLE", [
            (mode "IODELAY"),
            (global_opt "LEGIDELAY", "DISABLE"),
            (bel_mode bel_other, "IODELAY"),
            (bel_attr bel_other, "IDELAY_VALUE", ""),
            (bel_attr bel_other, "IDELAY_TYPE", "FIXED"),
            (related TileRelation::Hclk(hclk_ioi),
                (bel_mode bel_idelayctrl, "IDELAYCTRL"))
        ], [
            (attr "IDELAY_TYPE", "FIXED")
        ]);
    }

    for i in 0..2 {
        let bel_ologic = BelId::from_idx(2 + i);
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
        fuzz_multi_attr_bin!(ctx, "OPROGRAMMING", 31, [
            (mode "IOB"),
            (pin "O"),
            (attr "OUSED", "0"),
            (attr "OSTANDARD", "LVCMOS18")
        ]);
        fuzz_one!(ctx, "IMUX", "I", [
            (mode "IOB"),
            (attr "OUSED", ""),
            (pin "I"),
            (package package.name),
            (bel_special BelKV::IsBonded),
            (attr "ISTANDARD", "LVCMOS18")
        ], [
            (attr_diff "IMUX", "I_B", "I")
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
                fuzz_one_extras!(ctx, "ISTD", std.name, [
                    (mode ["IOBS", "IOBM"][i]),
                    (attr "OUSED", ""),
                    (pin "I"),
                    (pin "DIFFI_IN"),
                    (package package.name),
                    (bel_special BelKV::IsBonded),
                    (bel_special dci_special.clone()),
                    (bel_mode bel_other, ["IOBM", "IOBS"][i]),
                    (bel_pin bel_other, "PADOUT"),
                    (bel_attr bel_other, "OUSED", "")
                ], [
                    (attr "IMUX", "I_B"),
                    (attr "DIFFI_INUSED", "0"),
                    (attr "ISTANDARD", std.name),
                    (attr "DIFF_TERM", "FALSE"),
                    (bel_attr bel_other, "PADOUTUSED", "0"),
                    (bel_attr bel_other, "ISTANDARD", std.name),
                    (bel_attr bel_other, "DIFF_TERM", "FALSE")
                ], extras);
                if std.diff == DiffKind::True {
                    fuzz_one!(ctx, "DIFF_TERM", std.name, [
                        (mode ["IOBS", "IOBM"][i]),
                        (attr "OUSED", ""),
                        (pin "I"),
                        (pin "DIFFI_IN"),
                        (attr "IMUX", "I_B"),
                        (attr "DIFFI_INUSED", "0"),
                        (attr "ISTANDARD", std.name),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special dci_special),
                        (bel_mode bel_other, ["IOBM", "IOBS"][i]),
                        (bel_pin bel_other, "PADOUT"),
                        (bel_attr bel_other, "OUSED", ""),
                        (bel_attr bel_other, "PADOUTUSED", "0"),
                        (bel_attr bel_other, "ISTANDARD", std.name)
                    ], [
                        (attr_diff "DIFF_TERM", "FALSE", "TRUE"),
                        (bel_attr_diff bel_other, "DIFF_TERM", "FALSE", "TRUE")
                    ]);
                }
            } else {
                fuzz_one_extras!(ctx, "ISTD", std.name, [
                    (mode "IOB"),
                    (attr "OUSED", ""),
                    (pin "I"),
                    (package package.name),
                    (bel_special BelKV::IsBonded),
                    (bel_special vref_special),
                    (bel_special dci_special)
                ], [
                    (attr "IMUX", "I_B"),
                    (attr "ISTANDARD", std.name)
                ], extras);
            }
        }
        for &std in IOSTDS {
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
                        (attr "IMUX", ""),
                        (attr "OPROGRAMMING", ""),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special BelKV::PrepDiffOut),
                        (bel_attr bel_other, "IMUX", ""),
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
            } else if matches!(
                std.dci,
                DciKind::Output | DciKind::OutputHalf | DciKind::BiSplit | DciKind::BiVcc
            ) {
                let extras = vec![
                    ExtraFeature::new(ExtraFeatureKind::Vr, "IO", "IOB_COMMON", "PRESENT", "VR"),
                    ExtraFeature::new(
                        ExtraFeatureKind::HclkIoDci("HCLK_IOI"),
                        "HCLK_IOI",
                        "DCI",
                        "STD",
                        std.name,
                    ),
                ];
                fuzz_one_extras!(ctx, "OSTD", std.name, [
                    (mode "IOB"),
                    (pin "O"),
                    (attr "IMUX", ""),
                    (attr "OPROGRAMMING", ""),
                    (package package.name),
                    (bel_special BelKV::IsBonded),
                    (bel_special BelKV::PrepDci)
                ], [
                    (attr "OUSED", "0"),
                    (attr "OSTANDARD", std.name)
                ], extras);
            } else if !std.drive.is_empty() {
                for &drive in std.drive {
                    for slew in ["SLOW", "FAST"] {
                        fuzz_one!(ctx, "OSTD", format!("{name}.{drive}.{slew}", name=std.name), [
                            (mode "IOB"),
                            (pin "O"),
                            (attr "IMUX", ""),
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
                fuzz_one!(ctx, "OSTD", std.name, [
                    (mode "IOB"),
                    (pin "O"),
                    (attr "IMUX", ""),
                    (attr "OPROGRAMMING", "")
                ], [
                    (attr "OUSED", "0"),
                    (attr "OSTANDARD", std.name)
                ]);
            }
        }

        for (std, vref) in [
            ("HSTL_I", 750),
            ("HSTL_III", 900),
            ("HSTL_III_18", 1080),
            ("SSTL2_I", 1250),
        ] {
            let extras = vec![ExtraFeature::new(
                ExtraFeatureKind::Hclk(0, 0),
                "HCLK_IOI",
                "INTERNAL_VREF",
                "VREF",
                format!("{vref}"),
            )];
            fuzz_one_extras!(ctx, "ISTD", std, [
                (mode "IOB"),
                (attr "OUSED", ""),
                (pin "I"),
                (package package.name),
                (bel_special BelKV::IsBonded),
                (bel_special BelKV::PrepVrefInternal(vref))
            ], [
                (attr "IMUX", "I_B"),
                (attr "ISTANDARD", std)
            ], extras);
        }

        fuzz_one!(ctx, "OUTPUT_DELAY", "0", [], [
            (pip (bel_pin bel_ologic, "OQ"), (bel_pin bel_ologic, "O_IOB"))
        ]);
        fuzz_one!(ctx, "OUTPUT_DELAY", "1", [], [
            (pip (bel_pin bel_iodelay, "DATAOUT"), (bel_pin bel_ologic, "O_IOB"))
        ]);
    }
    let mut quiet_extras = vec![];
    for tile in [
        "HCLK_IOI",
        "HCLK_IOI_CENTER",
        "HCLK_IOI_TOPCEN",
        "HCLK_IOI_BOTCEN",
        "HCLK_CMT_IOI",
        "HCLK_IOI_CMT",
    ] {
        if let Some(ctx) = FuzzCtx::try_new(session, backend, tile, "DCI", TileBits::Hclk) {
            fuzz_one!(ctx, "TEST_ENABLE", "1", [
                (global_mutex "GLOBAL_DCI", "NOPE")
            ], [
                (mode "DCI")
            ]);
            quiet_extras.push(ExtraFeature::new(
                ExtraFeatureKind::AllHclkIo(tile),
                tile,
                "DCI",
                "QUIET",
                "1",
            ));
        }
    }
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    fuzz_one_extras!(ctx, "DCIUPDATEMODE", "QUIET", [], [
        (global_opt_diff "DCIUPDATEMODE", "CONTINUOUS", "QUIET")
    ], quiet_extras);
    for i in [3, 4] {
        let extras = vec![
            ExtraFeature::new(ExtraFeatureKind::Cfg, "CFG", "MISC", "DCI_CLK_ENABLE", "1"),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciIo(i),
                "IO",
                "IOB0",
                "OSTD",
                "LVDCI_33",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciHclk(i),
                "HCLK_IOI",
                "DCI",
                "ENABLE",
                "1",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciVr(i),
                "IO",
                "IOB_COMMON",
                "PRESENT",
                "VR",
            ),
        ];
        fuzz_one_extras!(ctx, format!("CENTER_DCI.{i}"), "1", [
            (package package.name),
            (special TileKV::CenterDci(i))
        ], [], extras);
    }
    for (a, b) in [(3, 1), (4, 2)] {
        let extras = vec![
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciIo(b),
                "IO",
                "IOB0",
                "OSTD",
                "LVDCI_33",
            ),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciHclk(b),
                "HCLK_IOI",
                "DCI",
                if b == 1 {
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

            let mut diff_default = ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "DEFAULT");
            let val = extract_bitvec_val_part(
                ctx.tiledb.item(tile, bel, "IDELAY_VALUE_INIT"),
                &bitvec![0; 6],
                &mut diff_default,
            );
            ctx.tiledb
                .insert_device_data(&ctx.device.name, "IODELAY:DEFAULT_IDELAY_VALUE", val);
            let val = extract_bitvec_val_part(
                ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"),
                &bitvec![0; 6],
                &mut diff_default,
            );
            ctx.tiledb
                .insert_device_data(&ctx.device.name, "IODELAY:DEFAULT_IDELAY_VALUE", val);
        }
        return;
    }

    {
        let bel = "IOI_CLK";
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.ICLK0",
            &[
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "HCLK8",
                "HCLK9", "IOCLK0", "IOCLK1", "IOCLK2", "IOCLK3", "RCLK0", "RCLK1", "RCLK2",
                "RCLK3", "CKINT0", "CKINT1",
            ],
            "NONE",
            OcdMode::Mux,
        );
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.ICLK1",
            &[
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "HCLK8",
                "HCLK9", "IOCLK0", "IOCLK1", "IOCLK2", "IOCLK3", "RCLK0", "RCLK1", "RCLK2",
                "RCLK3", "CKINT0", "CKINT1",
            ],
            "NONE",
            OcdMode::Mux,
        );
        for ibel in ["ILOGIC0", "ILOGIC1"] {
            for attr in ["INV.ICLK0", "INV.ICLK1"] {
                let item = ctx.extract_enum_bool_wide(tile, ibel, attr, "0", "1");
                ctx.tiledb.insert(tile, bel, attr, item);
            }
        }
    }
    for i in 0..2 {
        let bel = &format!("ILOGIC{i}");
        ctx.collect_inv(tile, bel, "CLKDIV");
        ctx.collect_enum(tile, bel, "MUX.CLK", &["ICLK0", "ICLK1"]);
        ctx.collect_enum(tile, bel, "MUX.CLKB", &["ICLK0", "ICLK1"]);

        let diff1 = ctx.state.get_diff(tile, bel, "OCLKINV.DDR", "OCLK_B");
        let diff2 = ctx.state.get_diff(tile, bel, "OCLKINV.DDR", "OCLK");
        ctx.state
            .get_diff(tile, bel, "OCLKINV.SDR", "OCLK")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "OCLKINV.SDR", "OCLK_B");
        diff = diff.combine(&!&diff1);
        diff = diff.combine(&!&diff2);
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "INV.OCLK1", xlat_bit(diff1));
        ctx.tiledb.insert(tile, bel, "INV.OCLK2", xlat_bit(diff2));

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum(tile, bel, "INTERFACE_TYPE", &["MEMORY", "NETWORKING"]);
        ctx.collect_enum(tile, bel, "NUM_CE", &["1", "2"]);
        ctx.collect_bitvec(tile, bel, "INIT_BITSLIPCNT", "");
        ctx.collect_bitvec(tile, bel, "INIT_CE", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK1_PARTIAL", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK2", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK3", "");
        let item = ctx.extract_enum_bool(tile, bel, "SRTYPE", "ASYNC", "SYNC");
        ctx.tiledb.insert(tile, bel, "IFF_SR_SYNC", item);
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

        ctx.state
            .get_diff(tile, bel, "BITSLIP_ENABLE.ASYNC", "FALSE")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "BITSLIP_ENABLE.SYNC", "FALSE")
            .assert_empty();
        let diff_async = ctx
            .state
            .get_diff(tile, bel, "BITSLIP_ENABLE.ASYNC", "TRUE");
        let diff_sync = ctx.state.get_diff(tile, bel, "BITSLIP_ENABLE.SYNC", "TRUE");
        let diff_sync = diff_sync.combine(&!&diff_async);
        ctx.tiledb
            .insert(tile, bel, "BITSLIP_ENABLE", xlat_bit_wide(diff_async));
        ctx.tiledb
            .insert(tile, bel, "BITSLIP_SYNC", xlat_bit(diff_sync));

        ctx.collect_enum(
            tile,
            bel,
            "DDR_CLK_EDGE",
            &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
        );

        let iff_rev_used = ctx.extract_bit(tile, bel, "REVUSED", "0");
        ctx.tiledb.insert(tile, bel, "IFF_REV_USED", iff_rev_used);
        let iff_sr_used = ctx.extract_bit(tile, bel, "SRUSED", "0");
        ctx.tiledb.insert(tile, bel, "IFF_SR_USED", iff_sr_used);

        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "#LATCH");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "DDR_CLK_EDGE"),
            "SAME_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IFFTYPE", "#FF");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "DDR_CLK_EDGE"),
            "SAME_EDGE",
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
        let item = ctx.extract_enum_bool(tile, bel, "IFFMUX", "1", "0");
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
            .get_diff(tile, bel, "PRESENT", "ILOGIC")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "ISERDES");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "TSBYPASS_MUX"), "GND", "T");
        diff.assert_empty();

        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK_I",
            TileItem::from_bit(TileBit::new(0, 29, [13, 50][i]), false),
        );
    }

    for i in 0..2 {
        let bel = &format!("OLOGIC{i}");
        let mut present_ologic = ctx.state.get_diff(tile, bel, "PRESENT", "OLOGIC");
        let mut present_oserdes = ctx.state.get_diff(tile, bel, "PRESENT", "OSERDES");

        for attr in ["DDR_CLK_EDGE", "ODDR_CLK_EDGE", "TDDR_CLK_EDGE"] {
            for val in ["SAME_EDGE", "OPPOSITE_EDGE"] {
                ctx.state.get_diff(tile, bel, attr, val).assert_empty();
            }
        }
        ctx.state
            .get_diff(tile, bel, "CLKINV.SAME", "CLK_B")
            .assert_empty();
        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "T1", "T2", "T3", "T4", "CLKDIV",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }
        let diff_clk1 = ctx.state.get_diff(tile, bel, "CLKINV.OPPOSITE", "CLK");
        let diff_clk2 = ctx.state.get_diff(tile, bel, "CLKINV.OPPOSITE", "CLK_B");
        let diff_clk12 = ctx.state.get_diff(tile, bel, "CLKINV.SAME", "CLK");
        assert_eq!(diff_clk12, diff_clk1.combine(&diff_clk2));
        ctx.tiledb
            .insert(tile, bel, "INV.CLK1", xlat_bit(!diff_clk1));
        ctx.tiledb
            .insert(tile, bel, "INV.CLK2", xlat_bit(!diff_clk2));

        let osrused = ctx.extract_bit(tile, bel, "OSRUSED", "0");
        let tsrused = ctx.extract_bit(tile, bel, "TSRUSED", "0");
        let orevused = ctx.extract_bit(tile, bel, "OREVUSED", "0");
        let trevused = ctx.extract_bit(tile, bel, "TREVUSED", "0");
        ctx.state.get_diff(tile, bel, "OCEUSED", "0").assert_empty();
        ctx.state.get_diff(tile, bel, "TCEUSED", "0").assert_empty();

        let diff_d1 = ctx.state.get_diff(tile, bel, "OMUX", "D1");
        let diff_serdes_sdr = ctx
            .state
            .get_diff(tile, bel, "DATA_RATE_OQ", "SDR")
            .combine(&diff_d1);
        let diff_serdes_ddr = ctx
            .state
            .get_diff(tile, bel, "DATA_RATE_OQ", "DDR")
            .combine(&diff_d1);
        let (diff_serdes_sdr, diff_serdes_ddr, mut diff_off_serdes) =
            Diff::split(diff_serdes_sdr, diff_serdes_ddr);
        diff_off_serdes.apply_bit_diff(&osrused, true, false);
        diff_off_serdes.apply_bit_diff(&orevused, true, false);
        let diff_latch = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#LATCH");
        let diff_ff = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "#FF");
        let diff_ddr = ctx.state.get_diff(tile, bel, "OUTFFTYPE", "DDR");
        ctx.state
            .get_diff(tile, bel, "OMUX", "OUTFF")
            .assert_empty();
        present_oserdes = present_oserdes.combine(&!&diff_d1);
        ctx.tiledb.insert(
            tile,
            bel,
            "OMUX",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                ("D1", diff_d1),
                ("SERDES_SDR", diff_serdes_sdr),
                ("SERDES_DDR", diff_serdes_ddr),
                ("FF", diff_ff),
                ("DDR", diff_ddr),
                ("LATCH", diff_latch),
            ]),
        );
        ctx.tiledb
            .insert(tile, bel, "OFF_SERDES", xlat_bit_wide(diff_off_serdes));

        let diff_t1 = ctx.state.get_diff(tile, bel, "TMUX", "T1");
        let diff_serdes_buf = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "BUF");
        let mut diff_serdes_sdr = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "SDR");
        let mut diff_serdes_ddr = ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "DDR");
        diff_serdes_sdr.apply_bit_diff(&tsrused, true, false);
        diff_serdes_sdr.apply_bit_diff(&trevused, true, false);
        diff_serdes_ddr.apply_bit_diff(&tsrused, true, false);
        diff_serdes_ddr.apply_bit_diff(&trevused, true, false);
        let diff_latch = ctx.state.get_diff(tile, bel, "TFFTYPE", "#LATCH");
        let diff_ff = ctx.state.get_diff(tile, bel, "TFFTYPE", "#FF");
        let diff_ddr = ctx.state.get_diff(tile, bel, "TFFTYPE", "DDR");
        ctx.state.get_diff(tile, bel, "TMUX", "TFF").assert_empty();
        present_oserdes = present_oserdes.combine(&!&diff_t1);
        present_ologic = present_ologic.combine(&!&diff_t1);
        ctx.tiledb.insert(
            tile,
            bel,
            "TMUX",
            xlat_enum(vec![
                ("NONE", Diff::default()),
                ("T1", diff_t1),
                ("T1", diff_serdes_buf),
                ("SERDES_DDR", diff_serdes_ddr),
                ("FF", diff_serdes_sdr),
                ("FF", diff_ff),
                ("DDR", diff_ddr),
                ("LATCH", diff_latch),
            ]),
        );

        ctx.collect_bitvec(tile, bel, "INIT_LOADCNT", "");
        present_oserdes.apply_bitvec_diff(
            ctx.tiledb.item(tile, bel, "INIT_LOADCNT"),
            &bitvec![0; 4],
            &bitvec![1; 4],
        );

        present_ologic.assert_empty();
        present_oserdes.assert_empty();

        ctx.tiledb.insert(tile, bel, "OFF_SR_USED", osrused);
        ctx.tiledb.insert(tile, bel, "TFF_SR_USED", tsrused);
        ctx.tiledb.insert(tile, bel, "OFF_REV_USED", orevused);
        ctx.tiledb.insert(tile, bel, "TFF_REV_USED", trevused);

        let item_oq = ctx.extract_enum_bool_wide(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_enum_bool_wide(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.state
            .get_diff(tile, bel, "SRTYPE", "ASYNC")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bitvec_diff(&item_oq, &bitvec![1; 4], &bitvec![0; 4]);
        diff.apply_bitvec_diff(&item_tq, &bitvec![1; 2], &bitvec![0; 2]);
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "OFF_SR_SYNC", item_oq);
        ctx.tiledb.insert(tile, bel, "TFF_SR_SYNC", item_tq);

        let diff_ologic = ctx.state.get_diff(tile, bel, "INIT_OQ.OLOGIC", "0");
        let diff_oserdes = ctx
            .state
            .get_diff(tile, bel, "INIT_OQ.OSERDES", "0")
            .combine(&!&diff_ologic);
        ctx.tiledb
            .insert(tile, bel, "OFF_INIT", xlat_bit_wide(!diff_ologic));
        ctx.tiledb
            .insert(tile, bel, "OFF_INIT_SERDES", xlat_bit_wide(!diff_oserdes));
        ctx.state
            .get_diff(tile, bel, "INIT_OQ.OLOGIC", "1")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "INIT_OQ.OSERDES", "1")
            .assert_empty();
        let item = ctx.extract_enum_bool_wide(tile, bel, "INIT_TQ.OLOGIC", "0", "1");
        ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "INIT_TQ.OSERDES", "0", "1");
        ctx.tiledb.insert(tile, bel, "TFF_INIT", item);

        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_OQ.OLOGIC", "0", "1");
        ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_OQ.OSERDES", "0", "1");
        ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);

        for attr in ["SRVAL_TQ.FF", "SRVAL_TQ.DDR", "SRVAL_TQ.OSERDES"] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
        let diff1 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.FF", "0");
        let diff2 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.DDR", "0");
        let diff3 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.OSERDES", "0");
        assert_eq!(diff2, diff3);
        let diff2 = diff2.combine(&!&diff1);
        ctx.tiledb.insert(tile, bel, "TFF1_SRVAL", xlat_bit(!diff1));
        ctx.tiledb
            .insert(tile, bel, "TFF23_SRVAL", xlat_bit_wide(!diff2));

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum(tile, bel, "TRISTATE_WIDTH", &["1", "4"]);
        ctx.collect_enum(
            tile,
            bel,
            "DATA_WIDTH",
            &["2", "3", "4", "5", "6", "7", "8", "10"],
        );

        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE_FDBK", "FALSE", "TRUE");
        ctx.collect_enum_default(tile, bel, "MISR_CLK_SELECT", &["CLK1", "CLK2"], "NONE");

        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.CLK",
            &[
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "HCLK8",
                "HCLK9", "IOCLK0", "IOCLK1", "IOCLK2", "IOCLK3", "RCLK0", "RCLK1", "RCLK2",
                "RCLK3", "CKINT",
            ],
            "NONE",
            OcdMode::Mux,
        );
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.CLKDIV",
            &[
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "HCLK8",
                "HCLK9", "RCLK0", "RCLK1", "RCLK2", "RCLK3", "CKINT",
            ],
            "NONE",
            OcdMode::Mux,
        );
    }
    let mut diff = ctx.state.get_diff(tile, "OLOGIC_COMMON", "MISR_RESET", "1");
    let diff1 = diff.split_bits_by(|bit| bit.bit >= 32);
    ctx.tiledb
        .insert(tile, "OLOGIC0", "MISR_RESET", xlat_bit(diff));
    ctx.tiledb
        .insert(tile, "OLOGIC1", "MISR_RESET", xlat_bit(diff1));

    for i in 0..2 {
        let bel = &format!("IODELAY{i}");
        let item = ctx.extract_inv(tile, bel, "C");
        ctx.tiledb
            .insert(tile, format!("ILOGIC{i}"), "INV.CLKDIV", item);
        ctx.collect_inv(tile, bel, "DATAIN");
        ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "DELAYCHAIN_OSC", "FALSE", "TRUE");
        ctx.collect_enum_default(tile, bel, "DELAY_SRC", &["I", "O", "IO", "DATAIN"], "NONE");
        ctx.collect_bitvec(tile, bel, "ODELAY_VALUE", "");

        let mut diffs_a = vec![];
        let mut diffs_b = vec![];
        for diff in ctx.state.get_diffs(tile, bel, "IDELAY_VALUE", "") {
            let mut diff_a = Diff::default();
            let mut diff_b = Diff::default();
            for (k, v) in diff.bits {
                if v {
                    diff_a.bits.insert(k, v);
                } else {
                    diff_b.bits.insert(k, v);
                }
            }
            diffs_a.push(diff_a);
            diffs_b.push(diff_b);
        }
        ctx.tiledb
            .insert(tile, bel, "IDELAY_VALUE_INIT", xlat_bitvec(diffs_a));
        ctx.tiledb
            .insert(tile, bel, "IDELAY_VALUE_CUR", xlat_bitvec(diffs_b));

        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x3f);
        present.apply_enum_diff(ctx.tiledb.item(tile, bel, "DELAY_SRC"), "NONE", "DATAIN");
        ctx.tiledb
            .insert(tile, bel, "ENABLE", xlat_bit_wide(present));

        let diff = ctx.state.get_diff(tile, bel, "LEGIDELAY", "DISABLE");
        ctx.tiledb.insert(tile, bel, "LEGIDELAY", xlat_bit(!diff));

        ctx.state
            .get_diff(tile, bel, "IDELAY_TYPE", "FIXED")
            .assert_empty();
        let diff_variable = ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "VARIABLE");
        let mut diff_default = ctx.state.get_diff(tile, bel, "IDELAY_TYPE", "DEFAULT");
        let val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "IDELAY_VALUE_INIT"),
            &bitvec![0; 6],
            &mut diff_default,
        );
        ctx.tiledb
            .insert_device_data(&ctx.device.name, "IODELAY:DEFAULT_IDELAY_VALUE", val);
        let val = extract_bitvec_val_part(
            ctx.tiledb.item(tile, bel, "IDELAY_VALUE_CUR"),
            &bitvec![0; 6],
            &mut diff_default,
        );
        ctx.tiledb
            .insert_device_data(&ctx.device.name, "IODELAY:DEFAULT_IDELAY_VALUE", val);
        ctx.tiledb.insert(
            tile,
            bel,
            "IDELAY_TYPE",
            xlat_enum(vec![
                ("VARIABLE", diff_variable),
                ("FIXED", Diff::default()),
                ("DEFAULT", diff_default),
            ]),
        );
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
        let diff = ctx
            .state
            .peek_diff(tile, bel, "OSTD", "LVCMOS25.12.SLOW")
            .combine(&present);
        ctx.tiledb
            .insert(tile, bel, "OUTPUT_ENABLE", xlat_bit_wide(diff));

        let oprog = ctx.extract_bitvec(tile, bel, "OPROGRAMMING", "");
        let lvds = TileItem::from_bitvec(oprog.bits[0..9].to_vec(), false);
        let dci_t = TileItem::from_bit(oprog.bits[9], false);
        let dci_mode = TileItem {
            bits: oprog.bits[10..13].to_vec(),
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
        let output_misc = TileItem::from_bitvec(oprog.bits[13..19].to_vec(), false);
        let dci_misc = TileItem::from_bitvec(oprog.bits[19..21].to_vec(), false);
        let pdrive_bits = oprog.bits[21..26].to_vec();
        let ndrive_bits = oprog.bits[26..31].to_vec();
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
                    TileBit::new(0, 37, 17),
                    TileBit::new(0, 36, 23),
                    TileBit::new(0, 37, 23),
                    TileBit::new(0, 37, 30),
                    TileBit::new(0, 37, 29),
                    TileBit::new(0, 37, 27),
                ],
                vec![
                    TileBit::new(0, 36, 31),
                    TileBit::new(0, 36, 27),
                    TileBit::new(0, 37, 31),
                    TileBit::new(0, 37, 28),
                    TileBit::new(0, 36, 26),
                    TileBit::new(0, 37, 20),
                ],
            )
        } else {
            (
                vec![
                    TileBit::new(0, 37, 46),
                    TileBit::new(0, 36, 40),
                    TileBit::new(0, 37, 40),
                    TileBit::new(0, 37, 33),
                    TileBit::new(0, 37, 34),
                    TileBit::new(0, 37, 36),
                ],
                vec![
                    TileBit::new(0, 36, 32),
                    TileBit::new(0, 36, 36),
                    TileBit::new(0, 37, 32),
                    TileBit::new(0, 37, 35),
                    TileBit::new(0, 36, 37),
                    TileBit::new(0, 37, 43),
                ],
            )
        };
        let pslew = TileItem::from_bitvec(pslew_bits, false);
        let nslew = TileItem::from_bitvec(nslew_bits, false);

        let diff_cmos = ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS18");
        let diff_vref = ctx.state.peek_diff(tile, bel, "ISTD", "HSTL_I");
        let diff_diff = ctx.state.peek_diff(tile, bel, "ISTD", "LVDS_25");
        let (_, _, diff_diff) = Diff::split(diff_cmos.clone(), diff_diff.clone());
        ctx.tiledb.insert(
            tile,
            bel,
            "IBUF_MODE",
            xlat_enum(vec![
                ("OFF", Diff::default()),
                ("CMOS", diff_cmos.clone()),
                ("VREF", diff_vref.clone()),
                ("DIFF", diff_diff),
            ]),
        );

        for &std in IOSTDS {
            if std.diff == DiffKind::True {
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
            ("PSLEW", &pslew.bits, &bitvec![0; 6]),
            ("NSLEW", &nslew.bits, &bitvec![0; 6]),
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
                ("PSLEW", &pslew.bits, &bitvec![0; 6]),
                ("NSLEW", &nslew.bits, &bitvec![0; 6]),
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
            .insert_misc_data("IOSTD:OUTPUT_MISC:OFF", bitvec![0; 6]);
        ctx.tiledb
            .insert_misc_data("IOSTD:LVDS_T:OFF", bitvec![0; 9]);
        ctx.tiledb
            .insert_misc_data("IOSTD:LVDS_C:OFF", bitvec![0; 9]);
        ctx.tiledb
            .insert_misc_data("IOSTD:PDRIVE:OFF", bitvec![0; 5]);
        ctx.tiledb
            .insert_misc_data("IOSTD:NDRIVE:OFF", bitvec![0; 5]);
        ctx.tiledb
            .insert_misc_data("IOSTD:PSLEW:OFF", bitvec![0; 6]);
        ctx.tiledb
            .insert_misc_data("IOSTD:NSLEW:OFF", bitvec![0; 6]);
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

        let diff = ctx.state.get_diff(tile, bel, "IMUX", "I");
        ctx.tiledb.insert(tile, bel, "INV.I", xlat_bit(!diff));

        present.assert_empty();
    }
    let diff1 = present_vr.split_bits_by(|bit| bit.bit >= 32);
    ctx.tiledb.insert(tile, "IOB0", "VR", xlat_bit(present_vr));
    ctx.tiledb.insert(tile, "IOB1", "VR", xlat_bit(diff1));
    for i in 0..2 {
        let bel = &format!("IOB{i}");
        for &std in IOSTDS {
            let mut diff = ctx.state.get_diff(tile, bel, "ISTD", std.name);
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
                    diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IBUF_MODE"), "DIFF", "OFF");
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
                let mode = if std.vref.is_some() { "VREF" } else { "CMOS" };
                diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IBUF_MODE"), mode, "OFF");
                diff.assert_empty();
            }
            if std.diff == DiffKind::True {
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
                if i == 1 {
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
            }
        }
    }

    let lvdsbias = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 35, 15),
            TileBit::new(0, 34, 15),
            TileBit::new(0, 34, 14),
            TileBit::new(0, 35, 14),
            TileBit::new(0, 35, 13),
            TileBit::new(0, 34, 13),
            TileBit::new(0, 34, 12),
            TileBit::new(0, 35, 12),
            TileBit::new(0, 32, 13),
            TileBit::new(0, 33, 13),
            TileBit::new(0, 33, 12),
            TileBit::new(0, 32, 12),
        ],
        false,
    );
    let lvdiv2 = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 52, 12),
            TileBit::new(0, 53, 12),
            TileBit::new(0, 53, 15),
        ],
        false,
    );
    let pref = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 51, 12),
            TileBit::new(0, 50, 12),
            TileBit::new(0, 53, 14),
            TileBit::new(0, 52, 15),
        ],
        false,
    );
    let nref = TileItem::from_bitvec(
        vec![TileBit::new(0, 52, 14), TileBit::new(0, 52, 13)],
        false,
    );
    let pmask_term_vcc = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 50, 15),
            TileBit::new(0, 50, 14),
            TileBit::new(0, 51, 14),
            TileBit::new(0, 51, 13),
            TileBit::new(0, 50, 13),
        ],
        false,
    );
    let pmask_term_split = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 46, 13),
            TileBit::new(0, 46, 12),
            TileBit::new(0, 47, 12),
            TileBit::new(0, 48, 15),
            TileBit::new(0, 49, 15),
        ],
        false,
    );
    let nmask_term_split = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 48, 13),
            TileBit::new(0, 49, 13),
            TileBit::new(0, 49, 12),
            TileBit::new(0, 48, 12),
            TileBit::new(0, 51, 15),
        ],
        false,
    );
    let vref = ctx.extract_enum_default(
        "HCLK_IOI",
        "INTERNAL_VREF",
        "VREF",
        &["750", "900", "1080", "1250"],
        "OFF",
    );
    let dci_en = ctx.extract_bit("HCLK_IOI", "DCI", "ENABLE", "1");
    let dci_casc_above = ctx.extract_bit("HCLK_IOI", "DCI", "CASCADE_FROM_ABOVE", "1");
    let dci_casc_below = ctx.extract_bit("HCLK_IOI", "DCI", "CASCADE_FROM_BELOW", "1");
    for tile in [
        "HCLK_IOI",
        "HCLK_IOI_CENTER",
        "HCLK_IOI_BOTCEN",
        "HCLK_IOI_TOPCEN",
        "HCLK_IOI_CMT",
        "HCLK_CMT_IOI",
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let bel = "LVDS";
        ctx.tiledb.insert(tile, bel, "LVDSBIAS", lvdsbias.clone());
        let bel = "INTERNAL_VREF";
        ctx.tiledb.insert(tile, bel, "VREF", vref.clone());
        let bel = "DCI";
        ctx.tiledb.insert(tile, bel, "ENABLE", dci_en.clone());
        ctx.tiledb
            .insert(tile, bel, "CASCADE_FROM_ABOVE", dci_casc_above.clone());
        ctx.tiledb
            .insert(tile, bel, "CASCADE_FROM_BELOW", dci_casc_below.clone());
        ctx.tiledb.insert(tile, bel, "LVDIV2", lvdiv2.clone());
        ctx.tiledb.insert(tile, bel, "PREF", pref.clone());
        ctx.tiledb.insert(tile, bel, "NREF", nref.clone());
        ctx.tiledb
            .insert(tile, bel, "PMASK_TERM_VCC", pmask_term_vcc.clone());
        ctx.tiledb
            .insert(tile, bel, "PMASK_TERM_SPLIT", pmask_term_split.clone());
        ctx.tiledb
            .insert(tile, bel, "NMASK_TERM_SPLIT", nmask_term_split.clone());
        ctx.collect_bit_wide(tile, bel, "TEST_ENABLE", "1");
        ctx.collect_bit(tile, bel, "QUIET", "1");
    }
    let tile = "HCLK_IOI";
    for std in IOSTDS {
        if std.diff == DiffKind::True {
            let bel = "LVDS";
            let diff = ctx.state.get_diff(tile, bel, "STD", std.name);
            let val = extract_bitvec_val(&lvdsbias, &bitvec![0; 12], diff);
            ctx.tiledb
                .insert_misc_data(format!("IOSTD:LVDSBIAS:{}", std.name), val);
        }
        if std.dci != DciKind::None {
            let bel = "DCI";
            let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
            let mut diff = ctx.state.get_diff(tile, bel, "STD", std.name);
            match std.dci {
                DciKind::OutputHalf => {
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, "LVDIV2"),
                        &bitvec![0; 3],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:LVDIV2:{stdname}"), val);
                }
                DciKind::InputVcc | DciKind::BiVcc => {
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, "PMASK_TERM_VCC"),
                        &bitvec![0; 5],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_VCC:{stdname}"), val);
                }
                DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, "PMASK_TERM_SPLIT"),
                        &bitvec![0; 5],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_SPLIT:{stdname}"), val);
                    let val = extract_bitvec_val_part(
                        ctx.tiledb.item(tile, bel, "NMASK_TERM_SPLIT"),
                        &bitvec![0; 5],
                        &mut diff,
                    );
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:DCI:NMASK_TERM_SPLIT:{stdname}"), val);
                }
                _ => {}
            }
            ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff));
        }
    }
    ctx.tiledb
        .insert_misc_data("IOSTD:LVDSBIAS:OFF", bitvec![0; 12]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:LVDIV2:OFF", bitvec![0; 3]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PMASK_TERM_VCC:OFF", bitvec![0; 5]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:PMASK_TERM_SPLIT:OFF", bitvec![0; 5]);
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:NMASK_TERM_SPLIT:OFF", bitvec![0; 5]);
    let tile = "CFG";
    let bel = "MISC";
    ctx.collect_bit_wide(tile, bel, "DCI_CLK_ENABLE", "1");
}
