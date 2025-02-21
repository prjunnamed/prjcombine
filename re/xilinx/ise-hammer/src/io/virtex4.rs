use bitvec::prelude::*;
use prjcombine_interconnect::db::BelId;
use prjcombine_re_collector::{
    Diff, OcdMode, extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_wide,
    xlat_bitvec, xlat_bool, xlat_enum, xlat_enum_ocd,
};
use prjcombine_re_hammer::Session;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelKV, ExtraFeature, ExtraFeatureKind, TileBits, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_enum_suffix, fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_one,
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
    Iostd::pseudo_diff("DIFF_SSTL2_II", 2500),
    Iostd::pseudo_diff("DIFF_SSTL18_II", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800),
    Iostd::pseudo_diff("DIFF_HSTL_II", 1500),
    Iostd::pseudo_diff("BLVDS_25", 2500),
    Iostd::pseudo_diff("LVPECL_25", 2500),
    Iostd::pseudo_diff_dci("DIFF_SSTL2_II_DCI", 2500, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_SSTL18_II_DCI", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI_18", 1800, DciKind::BiSplit),
    Iostd::pseudo_diff_dci("DIFF_HSTL_II_DCI", 1500, DciKind::BiSplit),
    Iostd::true_diff("LVDS_25", 2500),
    Iostd::true_diff("LVDSEXT_25", 2500),
    Iostd::true_diff("RSDS_25", 2500),
    Iostd::true_diff("ULVDS_25", 2500),
    Iostd::true_diff("LDT_25", 2500),
    Iostd::true_diff_dci("LVDS_25_DCI", 2500),
    Iostd::true_diff_dci("LVDSEXT_25_DCI", 2500),
];

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let package = backend
        .device
        .bonds
        .values()
        .max_by_key(|bond| {
            let bdata = &backend.db.bonds[bond.bond];
            let prjcombine_re_xilinx_geom::Bond::Virtex4(bdata) = bdata else {
                unreachable!();
            };
            bdata.pins.len()
        })
        .unwrap();

    let bel_ioclk = BelId::from_idx(6);
    for i in 0..2 {
        let ctx = FuzzCtx::new(
            session,
            backend,
            "IO",
            format!("ILOGIC{i}"),
            TileBits::MainAuto,
        );
        let bel_ologic = BelId::from_idx(2 + i);
        fuzz_one!(ctx, "PRESENT", "ILOGIC", [(bel_unused bel_ologic)], [(mode "ILOGIC")]);
        fuzz_one!(ctx, "PRESENT", "ISERDES", [(bel_unused bel_ologic)], [(mode "ISERDES")]);

        fuzz_enum!(ctx, "CLKDIVINV", ["CLKDIV", "CLKDIV_B"], [
            (mode "ILOGIC"),
            (attr "IMUX", "1"),
            (attr "IDELAYMUX", "1"),
            (attr "IDELMUX", "0"),
            (pin "CLKDIV")
        ]);
        fuzz_enum!(ctx, "CLKDIVINV", ["CLKDIV", "CLKDIV_B"], [
            (mode "ISERDES"),
            (pin "CLKDIV")
        ]);

        fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (pin "CLK")
        ]);
        fuzz_enum!(ctx, "CLKINV", ["CLK", "CLK_B"], [
            (mode "ISERDES"),
            (pin "CLK")
        ]);

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

        fuzz_enum!(ctx, "CE1INV", ["CE1", "CE1_B"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (pin "CE1")
        ]);
        fuzz_enum!(ctx, "CE1INV", ["CE1", "CE1_B"], [
            (mode "ISERDES"),
            (attr "INIT_CE", "11"),
            (pin "CE1")
        ]);
        fuzz_enum!(ctx, "CE2INV", ["CE2", "CE2_B"], [
            (mode "ISERDES"),
            (attr "INIT_CE", "11"),
            (pin "CE2")
        ]);

        fuzz_enum_suffix!(ctx, "SRINV", "OSR", ["SR", "SR_B"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (pin "SR"),
            (bel_mode bel_ologic, "OSERDES"),
            (bel_attr bel_ologic, "SRINV", "SR"),
            (bel_pin bel_ologic, "SR")
        ]);
        fuzz_enum_suffix!(ctx, "SRINV", "OSR_B", ["SR", "SR_B"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (pin "SR"),
            (bel_mode bel_ologic, "OSERDES"),
            (bel_attr bel_ologic, "SRINV", "SR_B"),
            (bel_pin bel_ologic, "SR")
        ]);
        fuzz_enum_suffix!(ctx, "REVINV", "OREV", ["REV", "REV_B"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (pin "REV"),
            (bel_mode bel_ologic, "OSERDES"),
            (bel_attr bel_ologic, "REVINV", "REV"),
            (bel_pin bel_ologic, "REV")
        ]);
        fuzz_enum_suffix!(ctx, "REVINV", "OREV_B", ["REV", "REV_B"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (pin "REV"),
            (bel_mode bel_ologic, "OSERDES"),
            (bel_attr bel_ologic, "REVINV", "REV_B"),
            (bel_pin bel_ologic, "REV")
        ]);
        fuzz_enum_suffix!(ctx, "SRINV", "OSR", ["SR", "SR_B"], [
            (mode "ISERDES"),
            (pin "SR"),
            (bel_mode bel_ologic, "OSERDES"),
            (bel_attr bel_ologic, "SRINV", "SR"),
            (bel_pin bel_ologic, "SR")
        ]);
        fuzz_enum_suffix!(ctx, "SRINV", "OSR_B", ["SR", "SR_B"], [
            (mode "ISERDES"),
            (pin "SR"),
            (bel_mode bel_ologic, "OSERDES"),
            (bel_attr bel_ologic, "SRINV", "SR_B"),
            (bel_pin bel_ologic, "SR")
        ]);
        fuzz_enum_suffix!(ctx, "REVINV", "OREV", ["REV", "REV_B"], [
            (mode "ISERDES"),
            (pin "REV"),
            (bel_mode bel_ologic, "OSERDES"),
            (bel_attr bel_ologic, "REVINV", "REV"),
            (bel_pin bel_ologic, "REV")
        ]);
        fuzz_enum_suffix!(ctx, "REVINV", "OREV_B", ["REV", "REV_B"], [
            (mode "ISERDES"),
            (pin "REV"),
            (bel_mode bel_ologic, "OSERDES"),
            (bel_attr bel_ologic, "REVINV", "REV_B"),
            (bel_pin bel_ologic, "REV")
        ]);

        fuzz_enum!(ctx, "SERDES", ["FALSE", "TRUE"], [
            (mode "ISERDES"),
            (attr "DATA_WIDTH", "2")
        ]);
        fuzz_enum!(ctx, "SERDES_MODE", ["SLAVE", "MASTER"], [
            (mode "ISERDES")
        ]);
        fuzz_enum!(ctx, "INTERFACE_TYPE", ["NETWORKING", "MEMORY"], [
            (mode "ISERDES")
        ]);
        fuzz_enum_suffix!(ctx, "Q1MUX", "IFF2", ["IFF1", "IFF3"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (attr "Q2MUX", "IFF2"),
            (attr "IFFMUX", "1"),
            (attr "IFFDELMUX", "1"),
            (pin "D"),
            (pin "Q1"),
            (pin "Q2")
        ]);
        fuzz_enum_suffix!(ctx, "Q1MUX", "IFF4", ["IFF1", "IFF3"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (attr "Q2MUX", "IFF4"),
            (attr "IFFMUX", "1"),
            (attr "IFFDELMUX", "1"),
            (pin "D"),
            (pin "Q1"),
            (pin "Q2")
        ]);
        fuzz_enum_suffix!(ctx, "Q2MUX", "IFF1", ["IFF2", "IFF4"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (attr "Q1MUX", "IFF1"),
            (attr "IFFMUX", "1"),
            (attr "IFFDELMUX", "1"),
            (pin "D"),
            (pin "Q1"),
            (pin "Q2")
        ]);
        fuzz_enum_suffix!(ctx, "Q2MUX", "IFF3", ["IFF2", "IFF4"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF"),
            (attr "Q1MUX", "IFF3"),
            (attr "IFFMUX", "1"),
            (attr "IFFDELMUX", "1"),
            (pin "D"),
            (pin "Q1"),
            (pin "Q2")
        ]);

        fuzz_enum!(ctx, "DATA_WIDTH", ["2", "3", "4", "5", "6", "7", "8", "10"], [
            (mode "ISERDES"),
            (attr "SERDES", "TRUE")
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
            (attr "INIT_RANK3", "111111")
        ]);
        fuzz_enum!(ctx, "DDR_CLK_EDGE", ["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"], [
            (mode "ISERDES")
        ]);

        fuzz_enum!(ctx, "IFF1", ["#FF", "#LATCH"], [
            (mode "ILOGIC")
        ]);
        for attr in [
            "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
            "SRVAL_Q4",
        ] {
            fuzz_enum!(ctx, attr, ["0", "1"], [
                (mode "ILOGIC"),
                (attr "IFF1", "#FF")
            ]);
            fuzz_enum!(ctx, attr, ["0", "1"], [
                (mode "ISERDES")
            ]);
        }
        fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
            (mode "ILOGIC"),
            (attr "IFF1", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
            (mode "ISERDES")
        ]);

        fuzz_multi_attr_bin!(ctx, "INIT_CE", 2, [
            (mode "ISERDES"),
            (attr "CE1INV", "CE1"),
            (attr "CE2INV", "CE2"),
            (pin "CE1"),
            (pin "CE2")
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

        fuzz_enum!(ctx, "D2OBYP_SEL", ["GND", "T"], [
            (mode "ILOGIC"),
            (attr "IMUX", "0"),
            (attr "IDELMUX", "1"),
            (attr "IFFMUX", "#OFF"),
            (pin "D"),
            (pin "TFB"),
            (pin "OFB"),
            (pin "O")
        ]);
        fuzz_enum!(ctx, "D2OFFBYP_SEL", ["GND", "T"], [
            (mode "ILOGIC"),
            (attr "IFFMUX", "0"),
            (attr "IFF1", "#FF"),
            (attr "IFFDELMUX", "1"),
            (attr "IMUX", "#OFF"),
            (pin "D"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IMUX", ["0", "1"], [
            (mode "ILOGIC"),
            (attr "IDELMUX", "1"),
            (attr "IDELMUX1USED", "0"),
            (pin "D"),
            (pin "O"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IFFMUX", ["0", "1"], [
            (mode "ILOGIC"),
            (attr "IFFDELMUX", "1"),
            (attr "IFF1", "#FF"),
            (pin "D"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IDELAYMUX", ["0", "1"], [
            (mode "ILOGIC"),
            (attr "IDELMUX", "0"),
            (attr "IMUX", "1"),
            (attr "CLKDIVINV", "CLKDIV"),
            (pin "D"),
            (pin "O"),
            (pin "OFB"),
            (pin "CLKDIV")
        ]);
        fuzz_enum!(ctx, "IDELMUX", ["0", "1"], [
            (mode "ILOGIC"),
            (attr "IMUX", "1"),
            (attr "IFFMUX", "1"),
            (attr "IFF1", "#FF"),
            (attr "IDELMUX1USED", "0"),
            (attr "IDELAYMUX", "1"),
            (attr "IFFDELMUX", "0"),
            (attr "Q1MUX", "IFF1"),
            (pin "D"),
            (pin "O"),
            (pin "Q1"),
            (pin "TFB"),
            (pin "OFB")
        ]);
        fuzz_enum!(ctx, "IFFDELMUX", ["0", "1"], [
            (mode "ILOGIC"),
            (attr "IMUX", "1"),
            (attr "IFFMUX", "0"),
            (attr "IFF1", "#FF"),
            (attr "IDELMUX1USED", "0"),
            (attr "IDELAYMUX", "1"),
            (attr "IDELMUX", "0"),
            (attr "Q1MUX", "IFF1"),
            (attr "D2OFFBYP_SEL", "T"),
            (pin "D"),
            (pin "O"),
            (pin "Q1"),
            (pin "TFB"),
            (pin "OFB")
        ]);

        for val in ["NONE", "IFD", "IBUF", "BOTH"] {
            fuzz_enum_suffix!(ctx, "OFB_USED", val, ["FALSE", "TRUE"], [
                (mode "ISERDES"),
                (attr "IOBDELAY", val),
                (pin "OFB")
            ]);
        }
        fuzz_enum!(ctx, "TFB_USED", ["FALSE", "TRUE"], [
            (mode "ISERDES"),
            (pin "TFB")
        ]);
        fuzz_enum!(ctx, "IOBDELAY", ["NONE", "IFD", "IBUF", "BOTH"], [
            (mode "ISERDES"),
            (attr "OFB_USED", "FALSE")
        ]);

        fuzz_enum_suffix!(ctx, "IOBDELAY_TYPE", "ILOGIC.IBUF", ["DEFAULT", "FIXED", "VARIABLE"], [
            (mode "ILOGIC"),
            (attr "IDELMUX", "0"),
            (attr "IMUX", "1"),
            (attr "IDELAYMUX", "1"),
            (attr "CLKDIVINV", "CLKDIV"),
            (attr "IFFDELMUX", "#OFF"),
            (pin "CLKDIV"),
            (pin "D"),
            (pin "O")
        ]);
        fuzz_enum_suffix!(ctx, "IOBDELAY_TYPE", "ILOGIC.IFD", ["DEFAULT", "FIXED", "VARIABLE"], [
            (mode "ILOGIC"),
            (attr "IFFDELMUX", "0"),
            (attr "IFFMUX", "1"),
            (attr "IDELAYMUX", "1"),
            (attr "CLKDIVINV", "CLKDIV"),
            (attr "IDELMUX", "#OFF"),
            (attr "IFF1", "#FF"),
            (attr "Q1MUX", "IFF1"),
            (pin "CLKDIV"),
            (pin "D"),
            (pin "Q1")
        ]);
        fuzz_enum_suffix!(ctx, "IOBDELAY_TYPE", "ISERDES.IBUF", ["DEFAULT", "FIXED", "VARIABLE"], [
            (mode "ISERDES"),
            (attr "IOBDELAY", "IBUF")
        ]);
        fuzz_enum_suffix!(ctx, "IOBDELAY_TYPE", "ISERDES.IFD", ["DEFAULT", "FIXED", "VARIABLE"], [
            (mode "ISERDES"),
            (attr "IOBDELAY", "IFD")
        ]);

        fuzz_multi_attr_dec!(ctx, "IOBDELAY_VALUE", 6, [
            (mode "ILOGIC")
        ]);
        fuzz_multi_attr_dec!(ctx, "IOBDELAY_VALUE", 6, [
            (mode "ISERDES")
        ]);

        fuzz_one!(ctx, "MUX.CLK", "CKINT", [
            (mutex "MUX.CLK", "CKINT")
        ], [
            (pip (pin "CLKMUX_INT"), (pin "CLKMUX"))
        ]);
        for ipin in [
            "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "RCLK0",
            "RCLK1", "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0", "IOCLK_N1",
        ] {
            fuzz_one!(ctx, "MUX.CLK", ipin, [
                (mutex "MUX.CLK", ipin)
            ], [
                (pip (bel_pin bel_ioclk, ipin), (pin "CLKMUX"))
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
        fuzz_enum_suffix!(ctx, "CLK1INV", "OLOGIC", ["C", "C_B"], [
            (mode "OLOGIC"),
            (attr "OFF1", "#FF"),
            (attr "OMUX", "OFFDDRA"),
            (pin "CLK"),
            (pin "OQ")
        ]);
        fuzz_enum_suffix!(ctx, "CLK2INV", "OLOGIC", ["CLK", "CLK_B"], [
            (mode "OLOGIC"),
            (attr "OFF1", "#FF"),
            (attr "OMUX", "OFFDDRA"),
            (pin "CLK"),
            (pin "OQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRINV", "OLOGIC", ["SR", "SR_B"], [
            (mode "OLOGIC"),
            (attr "OFF1", "#FF"),
            (attr "OSRUSED", "0"),
            (attr "OMUX", "OFFDDRA"),
            (pin "SR"),
            (pin "OQ"),
            (bel_unused bel_ilogic)
        ]);
        fuzz_enum_suffix!(ctx, "REVINV", "OLOGIC", ["REV", "REV_B"], [
            (mode "OLOGIC"),
            (attr "OFF1", "#FF"),
            (attr "OREVUSED", "0"),
            (attr "OMUX", "OFFDDRA"),
            (pin "REV"),
            (pin "OQ"),
            (bel_unused bel_ilogic)
        ]);
        for pin in ["D1", "D2", "OCE"] {
            fuzz_enum_suffix!(ctx, format!("{pin}INV"), "OLOGIC", [pin, &format!("{pin}_B")], [
                (mode "OLOGIC"),
                (attr "OFF1", "#FF"),
                (attr "OMUX", "OFFDDRA"),
                (pin pin),
                (pin "OQ")
            ]);
        }
        for pin in ["T2", "TCE"] {
            fuzz_enum_suffix!(ctx, format!("{pin}INV"), "OLOGIC", [pin, &format!("{pin}_B")], [
                (mode "OLOGIC"),
                (attr "TFF1", "#FF"),
                (attr "TMUX", "TFFDDRA"),
                (pin pin),
                (pin "TQ")
            ]);
        }
        fuzz_enum_suffix!(ctx, "T1INV", "OLOGIC", ["T1", "T1_B"], [
            (mode "OLOGIC"),
            (attr "TMUX", "T1"),
            (attr "T1USED", "0"),
            (pin "T1"),
            (pin "TQ")
        ]);

        for pin in [
            "CLKDIV", "SR", "REV", "D1", "D2", "D3", "D4", "D5", "D6", "T2", "T3", "T4",
        ] {
            fuzz_enum_suffix!(ctx, format!("{pin}INV"), "OSERDES", [pin, &format!("{pin}_B")], [
                (mode "OSERDES"),
                (pin pin),
                (bel_unused bel_ilogic)
            ]);
        }
        fuzz_enum_suffix!(ctx, "T1INV", "OSERDES", ["T1", "T1_B"], [
            (mode "OSERDES"),
            (attr "DATA_RATE_TQ", "BUF"),
            (pin "T1")
        ]);
        fuzz_enum_suffix!(ctx, "TCEINV", "OSERDES", ["TCE", "TCE_B"], [
            (mode "OSERDES"),
            (attr "DATA_RATE_TQ", "DDR"),
            (pin "TCE")
        ]);
        fuzz_enum_suffix!(ctx, "OCEINV", "OSERDES", ["OCE", "OCE_B"], [
            (mode "OSERDES"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "CLKINV", "CLK"),
            (attr "DDR_CLK_EDGE", "SAME_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);
        fuzz_enum_suffix!(ctx, "CLKINV", "OSERDES.SAME", ["CLK", "CLK_B"], [
            (mode "OSERDES"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "OCEINV", "OCE"),
            (attr "DDR_CLK_EDGE", "SAME_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);
        fuzz_enum_suffix!(ctx, "CLKINV", "OSERDES.OPPOSITE", ["CLK", "CLK_B"], [
            (mode "OSERDES"),
            (attr "DATA_RATE_OQ", "DDR"),
            (attr "OCEINV", "OCE"),
            (attr "DDR_CLK_EDGE", "OPPOSITE_EDGE"),
            (pin "OCE"),
            (pin "CLK")
        ]);

        fuzz_enum!(ctx, "OFF1", ["#FF", "#LATCH"], [
            (mode "OLOGIC"),
            (attr "OCEINV", "OCE_B"),
            (pin "OCE")
        ]);
        fuzz_enum!(ctx, "TFF1", ["#FF", "#LATCH"], [
            (mode "OLOGIC"),
            (attr "TCEINV", "TCE_B"),
            (pin "TCE")
        ]);
        fuzz_enum!(ctx, "SRTYPE_OQ", ["SYNC", "ASYNC"], [
            (mode "OLOGIC"),
            (attr "OFF1", "#FF")
        ]);
        fuzz_enum!(ctx, "SRTYPE_TQ", ["SYNC", "ASYNC"], [
            (mode "OLOGIC"),
            (attr "TFF1", "#FF")
        ]);
        for (attr, oattr) in [
            ("OSRUSED", "TSRUSED"),
            ("TSRUSED", "OSRUSED"),
            ("OREVUSED", "TREVUSED"),
            ("TREVUSED", "OREVUSED"),
        ] {
            fuzz_enum!(ctx, attr, ["0"], [
                (mode "OLOGIC"),
                (attr "OFF1", "#FF"),
                (attr "TFF1", "#FF"),
                (attr "REVINV", "REV"),
                (attr "SRINV", "SR"),
                (attr oattr, "0"),
                (pin "REV"),
                (pin "SR")
            ]);
        }

        fuzz_enum_suffix!(ctx, "INIT_OQ", "OLOGIC", ["0", "1"], [(mode "OLOGIC")]);
        fuzz_enum_suffix!(ctx, "INIT_TQ", "OLOGIC", ["0", "1"], [(mode "OLOGIC")]);
        fuzz_enum_suffix!(ctx, "INIT_OQ", "OSERDES", ["0", "1"], [(mode "OSERDES")]);
        fuzz_enum_suffix!(ctx, "INIT_TQ", "OSERDES", ["0", "1"], [(mode "OSERDES")]);

        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OFF1", ["0", "1"], [
            (mode "OLOGIC"),
            (attr "OFF1", "#FF"),
            (attr "D2INV", "#OFF"),
            (attr "OMUX", "OFF1"),
            (pin "OQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OFFDDRA", ["0", "1"], [
            (mode "OLOGIC"),
            (attr "OFF1", "#FF"),
            (attr "D2INV", "D2"),
            (attr "OMUX", "OFFDDRA"),
            (pin "D2"),
            (pin "OQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OFFDDRB", ["0", "1"], [
            (mode "OLOGIC"),
            (attr "OFF1", "#FF"),
            (attr "D2INV", "D2"),
            (attr "OMUX", "OFFDDRB"),
            (pin "D2"),
            (pin "OQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "TFF1", ["0", "1"], [
            (mode "OLOGIC"),
            (attr "TFF1", "#FF"),
            (attr "T2INV", "#OFF"),
            (attr "TMUX", "TFF1"),
            (pin "TQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "TFFDDRA", ["0", "1"], [
            (mode "OLOGIC"),
            (attr "TFF1", "#FF"),
            (attr "T2INV", "T2"),
            (attr "TMUX", "TFFDDRA"),
            (pin "T2"),
            (pin "TQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "TFFDDRB", ["0", "1"], [
            (mode "OLOGIC"),
            (attr "TFF1", "#FF"),
            (attr "T2INV", "T2"),
            (attr "TMUX", "TFFDDRB"),
            (pin "T2"),
            (pin "TQ")
        ]);
        fuzz_enum_suffix!(ctx, "SRVAL_OQ", "OSERDES", ["0", "1"], [(mode "OSERDES")]);
        fuzz_enum_suffix!(ctx, "SRVAL_TQ", "OSERDES", ["0", "1"], [(mode "OSERDES")]);

        fuzz_enum!(ctx, "OMUX", ["D1", "OFF1", "OFFDDRA", "OFFDDRB"], [
            (mode "OLOGIC"),
            (attr "SRINV", "#OFF"),
            (attr "REVINV", "#OFF"),
            (attr "OSRUSED", "#OFF"),
            (attr "OREVUSED", "#OFF"),
            (attr "OFF1", "#FF"),
            (attr "O1USED", "0"),
            (attr "D1INV", "D1"),
            (pin "D1"),
            (pin "OQ")
        ]);
        fuzz_enum!(ctx, "TMUX", ["T1", "TFF1", "TFFDDRA", "TFFDDRB"], [
            (mode "OLOGIC"),
            (attr "SRINV", "#OFF"),
            (attr "REVINV", "#OFF"),
            (attr "TSRUSED", "#OFF"),
            (attr "TREVUSED", "#OFF"),
            (attr "TFF1", "#FF"),
            (attr "T1USED", "0"),
            (attr "T1INV", "T1"),
            (pin "T1"),
            (pin "TQ")
        ]);

        fuzz_enum!(ctx, "SERDES", ["FALSE", "TRUE"], [
            (mode "OSERDES"),
            (attr "DATA_WIDTH", "2")
        ]);
        fuzz_enum!(ctx, "SERDES_MODE", ["SLAVE", "MASTER"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "DDR_CLK_EDGE", ["SAME_EDGE", "OPPOSITE_EDGE"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "SRTYPE", ["SYNC", "ASYNC"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "DATA_RATE_OQ", ["SDR", "DDR"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "DATA_RATE_TQ", ["BUF", "SDR", "DDR"], [
            (mode "OSERDES"),
            (attr "TCEINV", "TCE_B"),
            (attr "T1INV", "T1"),
            (pin "TCE"),
            (pin "T1")
        ]);
        fuzz_enum!(ctx, "TRISTATE_WIDTH", ["1", "2", "4"], [
            (mode "OSERDES")
        ]);
        fuzz_enum!(ctx, "DATA_WIDTH", ["2", "3", "4", "5", "6", "7", "8", "10"], [
            (mode "OSERDES"),
            (attr "SERDES", "TRUE")
        ]);
        fuzz_multi_attr_bin!(ctx, "INIT_LOADCNT", 4, [(mode "OSERDES")]);

        fuzz_one!(ctx, "MUX.CLK", "CKINT", [
            (mutex "MUX.CLK", "CKINT")
        ], [
            (pip (pin "CLKMUX_INT"), (pin "CLKMUX"))
        ]);
        for ipin in [
            "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "RCLK0",
            "RCLK1", "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0", "IOCLK_N1",
        ] {
            fuzz_one!(ctx, "MUX.CLK", ipin, [
                (mutex "MUX.CLK", ipin)
            ], [
                (pip (bel_pin bel_ioclk, ipin), (pin "CLKMUX"))
            ]);
        }
    }
    for i in 0..2 {
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
        fuzz_enum!(ctx, "GTSATTRBOX", ["DISABLE_GTS"], [
            (mode "IOB")
        ]);
        fuzz_one!(ctx, "OUSED", "0", [
            (mode "IOB"),
            (pin "O"),
            (attr "IOATTRBOX", "")
        ], [
            (attr "DRIVE_0MA", "DRIVE_0MA"),
            (attr "OUSED", "0")
        ]);
        fuzz_multi_attr_bin!(ctx, "OPROGRAMMING", 22, [
            (mode "IOB"),
            (attr "OUSED", "0"),
            (pin "O")
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
                    ExtraFeatureKind::HclkIoDci("HCLK_IOIS_DCI"),
                    "HCLK_IOIS_DCI",
                    "DCI",
                    "STD",
                    std.name,
                ));
            }
            if std.diff != DiffKind::None {
                fuzz_one_extras!(ctx, "ISTD", std.name, [
                    (mode "IOB"),
                    (attr "OUSED", ""),
                    (pin "I"),
                    (pin "DIFFI_IN"),
                    (package package.name),
                    (bel_special BelKV::IsBonded),
                    (bel_special dci_special.clone())
                ], [
                    (attr "INBUFUSED", "0"),
                    (attr "DIFFI_INUSED", "0"),
                    (attr "IOATTRBOX", std.name)
                ], extras);
                if std.diff == DiffKind::True && std.dci == DciKind::None {
                    fuzz_one!(ctx, "DIFF_TERM", std.name, [
                        (mode "IOB"),
                        (attr "OUSED", ""),
                        (pin "I"),
                        (pin "DIFFI_IN"),
                        (attr "INBUFUSED", "0"),
                        (attr "DIFFI_INUSED", "0"),
                        (attr "IOATTRBOX", std.name),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special dci_special)
                    ], [
                        (attr "DIFF_TERM", "TRUE")
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
                    (attr "INBUFUSED", "0"),
                    (attr "IOATTRBOX", std.name)
                ], extras);
            }
        }
        for &std in IOSTDS {
            if std.diff == DiffKind::True {
                if i == 0 {
                    fuzz_one!(ctx, "OSTD", std.name, [
                        (attr "INBUFUSED", ""),
                        (attr "OPROGRAMMING", ""),
                        (attr "OUSED", ""),
                        (pin "DIFFO_IN"),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special BelKV::PrepDiffOut)
                    ], [
                        (mode_diff "IOB", "IOBS"),
                        (attr "IOATTRBOX", std.name),
                        (attr "OUTMUX", "1"),
                        (attr "DIFFO_INUSED", "0")
                    ]);
                } else {
                    let extras = vec![ExtraFeature::new(
                        ExtraFeatureKind::HclkIoLvds,
                        "HCLK_IOIS_LVDS",
                        "LVDS",
                        "STD",
                        std.name,
                    )];
                    fuzz_one_extras!(ctx, "OSTD", std.name, [
                        (pin "O"),
                        (attr "INBUFUSED", ""),
                        (attr "OPROGRAMMING", ""),
                        (attr "OUSED", "0"),
                        (package package.name),
                        (bel_special BelKV::IsBonded),
                        (bel_special BelKV::PrepDiffOut)
                    ], [
                        (mode_diff "IOB", "IOBM"),
                        (attr "IOATTRBOX", std.name),
                        (attr_diff "DRIVE_0MA", "DRIVE_0MA", "")
                    ], extras);
                }
            } else if matches!(
                std.dci,
                DciKind::Output
                    | DciKind::OutputHalf
                    | DciKind::BiSplit
                    | DciKind::BiSplitT
                    | DciKind::BiVcc
            ) {
                let extras = vec![
                    ExtraFeature::new(ExtraFeatureKind::Vr, "IO", "IOB_COMMON", "PRESENT", "VR"),
                    ExtraFeature::new(
                        ExtraFeatureKind::HclkIoDci("HCLK_IOIS_DCI"),
                        "HCLK_IOIS_DCI",
                        "DCI",
                        "STD",
                        std.name,
                    ),
                ];
                fuzz_one_extras!(ctx, "OSTD", std.name, [
                    (mode "IOB"),
                    (pin "O"),
                    (attr "INBUFUSED", ""),
                    (attr "OPROGRAMMING", ""),
                    (attr "OUSED", "0"),
                    (package package.name),
                    (bel_special BelKV::IsBonded),
                    (bel_special BelKV::PrepDci)
                ], [
                    (attr "IOATTRBOX", std.name),
                    (attr_diff "DRIVE_0MA", "DRIVE_0MA", "")
                ], extras);
            } else if !std.drive.is_empty() {
                for &drive in std.drive {
                    for slew in ["SLOW", "FAST"] {
                        fuzz_one!(ctx, "OSTD", format!("{name}.{drive}.{slew}", name=std.name), [
                            (mode "IOB"),
                            (pin "O"),
                            (attr "INBUFUSED", ""),
                            (attr "OPROGRAMMING", ""),
                            (attr "OUSED", "0")
                        ], [
                            (attr "IOATTRBOX", std.name),
                            (attr_diff "DRIVE_0MA", "DRIVE_0MA", ""),
                            (attr "DRIVEATTRBOX", drive),
                            (attr "SLEW", slew)
                        ]);
                    }
                }
            } else {
                fuzz_one!(ctx, "OSTD", std.name, [
                    (mode "IOB"),
                    (pin "O"),
                    (attr "INBUFUSED", ""),
                    (attr "OPROGRAMMING", ""),
                    (attr "OUSED", "0")
                ], [
                    (attr "IOATTRBOX", std.name),
                    (attr_diff "DRIVE_0MA", "DRIVE_0MA", "")
                ]);
            }
        }
    }
    let ctx = FuzzCtx::new_fake_tile(session, backend, "NULL", "NULL", TileBits::Null);
    let extras = [
        "HCLK_IOIS_DCI",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
        "HCLK_DCMIOB",
        "HCLK_IOBDCM",
    ]
    .into_iter()
    .map(|tile| ExtraFeature::new(ExtraFeatureKind::AllHclkIo(tile), tile, "DCI", "QUIET", "1"))
    .collect();
    fuzz_one_extras!(ctx, "DCIUPDATEMODE", "QUIET", [], [
        (global_opt_diff "DCIUPDATEMODE", "CONTINUOUS", "QUIET")
    ], extras);
    let hclk_center_cnt = backend.egrid.node_index[backend.egrid.db.get_node("HCLK_CENTER")].len();
    for i in [1, 2, 3, 4] {
        let mut extras = vec![
            ExtraFeature::new(ExtraFeatureKind::Cfg, "CFG", "MISC", "DCI_CLK_ENABLE", "1"),
            ExtraFeature::new(
                ExtraFeatureKind::CenterDciIo(i),
                "IO",
                "IOB0",
                "OSTD",
                "LVDCI_33",
            ),
        ];
        if matches!(i, 1 | 2) && hclk_center_cnt > 1 {
            extras.extend([
                ExtraFeature::new(
                    ExtraFeatureKind::CenterDciHclk(i),
                    "HCLK_CENTER",
                    "DCI",
                    "ENABLE",
                    "1",
                ),
                ExtraFeature::new(
                    ExtraFeatureKind::CenterDciVr(i),
                    "IO",
                    "IOB_COMMON",
                    "PRESENT",
                    "VR_CENTER",
                ),
            ]);
            if i == 2 {
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::CenterDciHclkCascade(i, "HCLK_CENTER"),
                    "HCLK_CENTER",
                    "DCI",
                    "CASCADE_FROM_BELOW",
                    "1",
                ));
            } else {
                extras.push(ExtraFeature::new(
                    ExtraFeatureKind::CenterDciHclkCascade(i, "HCLK_CENTER_ABOVE_CFG"),
                    "HCLK_CENTER_ABOVE_CFG",
                    "DCI",
                    "CASCADE_FROM_ABOVE",
                    "1",
                ));
                if hclk_center_cnt > 3 {
                    extras.push(ExtraFeature::new(
                        ExtraFeatureKind::CenterDciHclkCascade(i, "HCLK_CENTER"),
                        "HCLK_CENTER",
                        "DCI",
                        "CASCADE_FROM_ABOVE",
                        "1",
                    ));
                }
            }
        } else if matches!(i, 1 | 2) {
            extras.extend([ExtraFeature::new(
                ExtraFeatureKind::CenterDciHclk(i),
                if i == 1 {
                    "HCLK_CENTER_ABOVE_CFG"
                } else {
                    "HCLK_CENTER"
                },
                "DCI",
                "ENABLE",
                "1",
            )]);
        } else {
            extras.extend([
                ExtraFeature::new(
                    ExtraFeatureKind::CenterDciHclk(i),
                    if i == 3 { "HCLK_IOBDCM" } else { "HCLK_DCMIOB" },
                    "DCI",
                    "ENABLE",
                    "1",
                ),
                ExtraFeature::new(
                    ExtraFeatureKind::CenterDciVr(i),
                    "IO",
                    "IOB_COMMON",
                    "PRESENT",
                    "VR_CENTER",
                ),
            ]);
        }
        fuzz_one_extras!(ctx, format!("CENTER_DCI.{i}"), "1", [
            (package package.name),
            (special TileKV::CenterDci(i))
        ], [], extras);
    }

    for tile in [
        "HCLK_IOIS_DCI",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
        "HCLK_DCMIOB",
        "HCLK_IOBDCM",
    ] {
        let ctx = FuzzCtx::new(session, backend, tile, "DCI", TileBits::Hclk);
        fuzz_one!(ctx, "TEST_ENABLE", "1", [
            (global_mutex "GLOBAL_DCI", "NOPE")
        ], [
            (mode "DCI")
        ]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "IO";
    for i in 0..2 {
        let bel = &format!("ILOGIC{i}");

        let mut present_ilogic = ctx.state.get_diff(tile, bel, "PRESENT", "ILOGIC");
        let mut present_iserdes = ctx.state.get_diff(tile, bel, "PRESENT", "ISERDES");

        ctx.collect_int_inv(&["INT"], tile, bel, "CLKDIV", false);
        ctx.collect_inv(tile, bel, "CE1");
        ctx.collect_inv(tile, bel, "CE2");
        for pin in ["SR", "REV"] {
            let diff0 = ctx
                .state
                .get_diff(tile, bel, format!("{pin}INV.O{pin}"), pin);
            let diff1 =
                ctx.state
                    .get_diff(tile, bel, format!("{pin}INV.O{pin}"), format!("{pin}_B"));
            let item = xlat_bool(diff0, diff1);
            ctx.tiledb.insert(tile, bel, format!("INV.{pin}"), item);
            let diff0 =
                ctx.state
                    .get_diff(tile, bel, format!("{pin}INV.O{pin}_B"), format!("{pin}_B"));
            let diff1 = ctx
                .state
                .get_diff(tile, bel, format!("{pin}INV.O{pin}_B"), pin);
            let item = xlat_bool(diff0, diff1);
            ctx.tiledb.insert(tile, bel, format!("INV.{pin}"), item);
        }

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
        let item = ctx.extract_enum_bool_wide(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.tiledb.insert(tile, bel, "INV.CLK", item);

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        let item = ctx.extract_enum_bool(tile, bel, "IFF1", "#FF", "#LATCH");
        ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
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
            let mut diff = ctx.state.get_diff(tile, bel, "DATA_WIDTH", val);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "SERDES"), true, false);
            diffs.push((val, diff));
        }
        let mut bits = xlat_enum(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.tiledb.insert(
            tile,
            bel,
            "DATA_WIDTH",
            xlat_enum_ocd(diffs, OcdMode::FixedOrder(&bits)),
        );

        let mut diffs = vec![];
        for val in ["SDR", "DDR"] {
            let mut diff = ctx.state.get_diff(tile, bel, "DATA_RATE", val);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_LATCH"), false, true);
            diffs.push((val, diff));
        }
        ctx.tiledb.insert(tile, bel, "DATA_RATE", xlat_enum(diffs));

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

        let mut diffs_a = vec![];
        let mut diffs_b = vec![];
        for diff in ctx.state.get_diffs(tile, bel, "IOBDELAY_VALUE", "") {
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
            .insert(tile, bel, "IOBDELAY_VALUE_INIT", xlat_bitvec(diffs_a));
        ctx.tiledb
            .insert(tile, bel, "IOBDELAY_VALUE_CUR", xlat_bitvec(diffs_b));

        let item = xlat_enum(vec![
            (
                "OPPOSITE_EDGE",
                ctx.state.get_diff(tile, bel, "Q2MUX.IFF3", "IFF2"),
            ),
            (
                "SAME_EDGE",
                ctx.state.get_diff(tile, bel, "Q1MUX.IFF4", "IFF1"),
            ),
            ("SAME_EDGE_PIPELINED", Diff::default()),
        ]);
        // wtf is even going on
        present_iserdes.apply_enum_diff(&item, "SAME_EDGE", "SAME_EDGE_PIPELINED");
        ctx.state
            .get_diff(tile, bel, "DDR_CLK_EDGE", "SAME_EDGE")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "DDR_CLK_EDGE", "SAME_EDGE_PIPELINED")
            .assert_empty();
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "DDR_CLK_EDGE", "OPPOSITE_EDGE");
        diff.apply_enum_diff(&item, "OPPOSITE_EDGE", "SAME_EDGE");
        diff.assert_empty();
        ctx.state
            .get_diff(tile, bel, "Q1MUX.IFF2", "IFF1")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "Q1MUX.IFF4", "IFF3")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "Q2MUX.IFF3", "IFF4")
            .assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "Q1MUX.IFF2", "IFF3");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "Q2MUX.IFF1", "IFF4");
        diff.apply_enum_diff(
            ctx.tiledb.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "Q2MUX.IFF1", "IFF2");
        diff.apply_enum_diff(&item, "OPPOSITE_EDGE", "SAME_EDGE");
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "DDR_CLK_EDGE", item);

        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("D", ctx.state.get_diff(tile, bel, "IDELAYMUX", "1")),
            ("OFB", ctx.state.get_diff(tile, bel, "IDELAYMUX", "0")),
        ]);
        ctx.tiledb.insert(tile, bel, "IDELAYMUX", item);
        let item = ctx.extract_enum(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
        ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = ctx.extract_enum(tile, bel, "D2OFFBYP_SEL", &["GND", "T"]);
        ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);
        // this seems wrong, and also it's opposite on v5 — bug?
        let item = xlat_enum(vec![
            ("GND", ctx.state.get_diff(tile, bel, "TFB_USED", "TRUE")),
            ("T", ctx.state.get_diff(tile, bel, "TFB_USED", "FALSE")),
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
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IDELAYMUX"), "D", "NONE");
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IOBDELAY", "IFD");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IDELAYMUX"), "D", "NONE");
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "IOBDELAY", "BOTH");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IDELAYMUX"), "D", "NONE");
        diff.assert_empty();

        let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
        ctx.tiledb.insert(tile, bel, "I_TSBYPASS_ENABLE", item);
        let diff0 = ctx.state.get_diff(tile, bel, "IFFMUX", "1");
        let diff1 = ctx.state.get_diff(tile, bel, "IFFMUX", "0");
        let (diff0, diff1, diff_common) = Diff::split(diff0, diff1);
        ctx.tiledb
            .insert(tile, bel, "IFF_TSBYPASS_ENABLE", xlat_bool(diff0, diff1));
        present_iserdes = present_iserdes.combine(&!&diff_common);
        ctx.tiledb
            .insert(tile, bel, "IFF_ENABLE", xlat_bit(diff_common));

        ctx.state
            .get_diff(tile, bel, "OFB_USED.NONE", "FALSE")
            .assert_empty();
        for attr in ["OFB_USED.IBUF", "OFB_USED.IFD", "OFB_USED.BOTH"] {
            let mut diff = ctx.state.get_diff(tile, bel, attr, "FALSE");
            diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IDELAYMUX"), "D", "NONE");
            diff.assert_empty();
        }
        let mut diff = ctx.state.get_diff(tile, bel, "OFB_USED.NONE", "TRUE");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IDELAYMUX"), "OFB", "NONE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "IFF_TSBYPASS_ENABLE"),
            true,
            false,
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "OFB_USED.IBUF", "TRUE");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IDELAYMUX"), "OFB", "NONE");
        diff.apply_bit_diff(
            ctx.tiledb.item(tile, bel, "IFF_TSBYPASS_ENABLE"),
            true,
            false,
        );
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "OFB_USED.IFD", "TRUE");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IDELAYMUX"), "OFB", "NONE");
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.state.get_diff(tile, bel, "OFB_USED.BOTH", "TRUE");
        diff.apply_enum_diff(ctx.tiledb.item(tile, bel, "IDELAYMUX"), "OFB", "NONE");
        diff.assert_empty();

        let item = ctx.extract_enum(
            tile,
            bel,
            "IOBDELAY_TYPE.ILOGIC.IFD",
            &["DEFAULT", "FIXED", "VARIABLE"],
        );
        ctx.tiledb.insert(tile, bel, "IOBDELAY_TYPE", item);
        let item = ctx.extract_enum(
            tile,
            bel,
            "IOBDELAY_TYPE.ISERDES.IFD",
            &["DEFAULT", "FIXED", "VARIABLE"],
        );
        ctx.tiledb.insert(tile, bel, "IOBDELAY_TYPE", item);
        let item = ctx.extract_enum_default(
            tile,
            bel,
            "IOBDELAY_TYPE.ILOGIC.IBUF",
            &["FIXED", "VARIABLE"],
            "DEFAULT",
        );
        ctx.tiledb.insert(tile, bel, "IOBDELAY_TYPE", item);
        let item = ctx.extract_enum_default(
            tile,
            bel,
            "IOBDELAY_TYPE.ISERDES.IBUF",
            &["FIXED", "VARIABLE"],
            "DEFAULT",
        );
        ctx.tiledb.insert(tile, bel, "IOBDELAY_TYPE", item);

        // hm. not clear what's going on.
        let item = ctx.extract_bit(tile, bel, "IOBDELAY_TYPE.ILOGIC.IBUF", "DEFAULT");
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "IOBDELAY_TYPE.ISERDES.IBUF", "DEFAULT");
        diff.apply_bit_diff(&item, true, false);
        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "I_DELAY_ENABLE"), false, true);
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "I_DELAY_DEFAULT", item);

        present_ilogic.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.CE1"), false, true);
        present_iserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.CE1"), false, true);
        present_ilogic.apply_bitvec_diff_int(
            ctx.tiledb.item(tile, bel, "IOBDELAY_VALUE_CUR"),
            0,
            0x3f,
        );
        present_iserdes.apply_bitvec_diff_int(
            ctx.tiledb.item(tile, bel, "IOBDELAY_VALUE_CUR"),
            0,
            0x3f,
        );

        present_ilogic.assert_empty();
        present_iserdes.assert_empty();

        ctx.tiledb.insert(
            tile,
            bel,
            "READBACK_I",
            TileItem::from_bit(TileBit::new(0, 21, [47, 32][i]), false),
        );

        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.CLK",
            &[
                "CKINT", "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7",
                "RCLK0", "RCLK1", "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0",
                "IOCLK_N1",
            ],
            "NONE",
            OcdMode::Mux,
        );
    }
    for i in 0..2 {
        let bel = &format!("OLOGIC{i}");
        ctx.collect_enum_default_ocd(
            tile,
            bel,
            "MUX.CLK",
            &[
                "CKINT", "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7",
                "RCLK0", "RCLK1", "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0",
                "IOCLK_N1",
            ],
            "NONE",
            OcdMode::Mux,
        );
        let osrused = ctx.extract_bit(tile, bel, "OSRUSED", "0");
        let tsrused = ctx.extract_bit(tile, bel, "TSRUSED", "0");
        let orevused = ctx.extract_bit(tile, bel, "OREVUSED", "0");
        let trevused = ctx.extract_bit(tile, bel, "TREVUSED", "0");
        for pin in ["D1", "D2", "D3", "D4", "D5", "D6", "T1", "T2", "T3", "T4"] {
            let item = ctx.extract_enum_bool(
                tile,
                bel,
                &format!("{pin}INV.OSERDES"),
                pin,
                &format!("{pin}_B"),
            );
            ctx.tiledb.insert(tile, bel, format!("INV.{pin}"), item);
        }
        for pin in ["D1", "D2", "T1", "T2"] {
            let item = ctx.extract_enum_bool(
                tile,
                bel,
                &format!("{pin}INV.OLOGIC"),
                pin,
                &format!("{pin}_B"),
            );
            ctx.tiledb.insert(tile, bel, format!("INV.{pin}"), item);
        }
        for pin in ["OCE", "TCE", "CLKDIV"] {
            let item = ctx.extract_enum_bool(
                tile,
                bel,
                &format!("{pin}INV.OSERDES"),
                pin,
                &format!("{pin}_B"),
            );
            ctx.insert_int_inv(&["INT"], tile, bel, pin, item);
        }
        for pin in ["OCE", "TCE"] {
            let item = ctx.extract_enum_bool(
                tile,
                bel,
                &format!("{pin}INV.OLOGIC"),
                pin,
                &format!("{pin}_B"),
            );
            ctx.insert_int_inv(&["INT"], tile, bel, pin, item);
        }
        for (pin, oused, tused) in [("SR", &osrused, &tsrused), ("REV", &orevused, &trevused)] {
            let mut diff0 = ctx
                .state
                .get_diff(tile, bel, format!("{pin}INV.OLOGIC"), pin);
            let mut diff1 =
                ctx.state
                    .get_diff(tile, bel, format!("{pin}INV.OLOGIC"), format!("{pin}_B"));
            diff0.apply_bit_diff(oused, true, false);
            diff1.apply_bit_diff(oused, true, false);
            ctx.insert_int_inv(&["INT"], tile, bel, pin, xlat_bool(diff0, diff1));
            let mut diff0 = ctx
                .state
                .get_diff(tile, bel, format!("{pin}INV.OSERDES"), pin);
            let mut diff1 =
                ctx.state
                    .get_diff(tile, bel, format!("{pin}INV.OSERDES"), format!("{pin}_B"));
            diff0.apply_bit_diff(oused, true, false);
            diff1.apply_bit_diff(oused, true, false);
            diff0.apply_bit_diff(tused, true, false);
            diff1.apply_bit_diff(tused, true, false);
            ctx.insert_int_inv(&["INT"], tile, bel, pin, xlat_bool(diff0, diff1));
        }
        let clk1inv = ctx.extract_enum_bool(tile, bel, "CLK1INV.OLOGIC", "C", "C_B");
        let clk2inv = ctx.extract_enum_bool(tile, bel, "CLK2INV.OLOGIC", "CLK", "CLK_B");
        let mut diff = ctx.state.get_diff(tile, bel, "CLKINV.OSERDES.SAME", "CLK");
        diff.apply_bit_diff(&clk1inv, false, true);
        diff.apply_bit_diff(&clk2inv, false, true);
        diff.assert_empty();
        let diff = ctx
            .state
            .get_diff(tile, bel, "CLKINV.OSERDES.SAME", "CLK_B");
        diff.assert_empty();
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "CLKINV.OSERDES.OPPOSITE", "CLK");
        diff.apply_bit_diff(&clk1inv, false, true);
        diff.assert_empty();
        let mut diff = ctx
            .state
            .get_diff(tile, bel, "CLKINV.OSERDES.OPPOSITE", "CLK_B");
        diff.apply_bit_diff(&clk2inv, false, true);
        diff.assert_empty();
        ctx.tiledb.insert(tile, bel, "INV.CLK1", clk1inv);
        ctx.tiledb.insert(tile, bel, "INV.CLK2", clk2inv);
        ctx.state
            .get_diff(tile, bel, "DDR_CLK_EDGE", "SAME_EDGE")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "DDR_CLK_EDGE", "OPPOSITE_EDGE")
            .assert_empty();

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

        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("D1", ctx.state.get_diff(tile, bel, "OMUX", "D1")),
            ("OFF1", ctx.state.get_diff(tile, bel, "OMUX", "OFF1")),
            ("OFFDDR", ctx.state.get_diff(tile, bel, "OMUX", "OFFDDRA")),
            ("OFFDDR", ctx.state.get_diff(tile, bel, "OMUX", "OFFDDRB")),
        ]);
        ctx.tiledb.insert(tile, bel, "OMUX", item);
        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("T1", ctx.state.get_diff(tile, bel, "TMUX", "T1")),
            ("TFF1", ctx.state.get_diff(tile, bel, "TMUX", "TFF1")),
            ("TFFDDR", ctx.state.get_diff(tile, bel, "TMUX", "TFFDDRA")),
            ("TFFDDR", ctx.state.get_diff(tile, bel, "TMUX", "TFFDDRB")),
            ("T1", ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "BUF")),
            ("TFF1", ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "SDR")),
            (
                "TFFDDR",
                ctx.state.get_diff(tile, bel, "DATA_RATE_TQ", "DDR"),
            ),
        ]);
        ctx.tiledb.insert(tile, bel, "TMUX", item);
        let mut diff_sdr = ctx.state.get_diff(tile, bel, "DATA_RATE_OQ", "SDR");
        let mut diff_ddr = ctx.state.get_diff(tile, bel, "DATA_RATE_OQ", "DDR");
        diff_sdr.apply_enum_diff(ctx.tiledb.item(tile, bel, "OMUX"), "OFF1", "D1");
        diff_ddr.apply_enum_diff(ctx.tiledb.item(tile, bel, "OMUX"), "OFFDDR", "D1");
        assert_eq!(diff_sdr, diff_ddr);
        ctx.tiledb
            .insert(tile, bel, "OFF_SERDES", xlat_bit_wide(diff_sdr));

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum(tile, bel, "TRISTATE_WIDTH", &["1", "2", "4"]);
        ctx.collect_bitvec(tile, bel, "INIT_LOADCNT", "");

        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10"] {
            let mut diff = ctx.state.get_diff(tile, bel, "DATA_WIDTH", val);
            diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "SERDES"), true, false);
            diffs.push((val, diff));
        }
        ctx.tiledb.insert(tile, bel, "DATA_WIDTH", xlat_enum(diffs));

        let item = ctx.extract_enum_bool(tile, bel, "OFF1", "#FF", "#LATCH");
        ctx.tiledb.insert(tile, bel, "OFF_LATCH", item);
        let item = ctx.extract_enum_bool(tile, bel, "TFF1", "#FF", "#LATCH");
        ctx.tiledb.insert(tile, bel, "TFF_LATCH", item);

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
        for attr in [
            "SRVAL_OQ.OFF1",
            "SRVAL_OQ.OFFDDRA",
            "SRVAL_OQ.OFFDDRB",
            "SRVAL_OQ.OSERDES",
        ] {
            let item = ctx.extract_enum_bool_wide(tile, bel, attr, "0", "1");
            ctx.tiledb.insert(tile, bel, "OFF_SRVAL", item);
        }

        for attr in [
            "SRVAL_TQ.TFF1",
            "SRVAL_TQ.TFFDDRA",
            "SRVAL_TQ.TFFDDRB",
            "SRVAL_TQ.OSERDES",
        ] {
            ctx.state.get_diff(tile, bel, attr, "1").assert_empty();
        }
        let diff1 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.TFF1", "0");
        let diff2 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.TFFDDRA", "0");
        let diff3 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.TFFDDRB", "0");
        let diff4 = ctx.state.get_diff(tile, bel, "SRVAL_TQ.OSERDES", "0");
        assert_eq!(diff3, diff4);
        let diff3 = diff3.combine(&!&diff2);
        let diff2 = diff2.combine(&!&diff1);
        ctx.tiledb.insert(tile, bel, "TFF1_SRVAL", xlat_bit(!diff1));
        ctx.tiledb.insert(tile, bel, "TFF2_SRVAL", xlat_bit(!diff2));
        ctx.tiledb.insert(tile, bel, "TFF3_SRVAL", xlat_bit(!diff3));

        let mut present_ologic = ctx.state.get_diff(tile, bel, "PRESENT", "OLOGIC");
        let mut present_oserdes = ctx.state.get_diff(tile, bel, "PRESENT", "OSERDES");
        present_ologic.apply_enum_diff(ctx.tiledb.item(tile, bel, "TMUX"), "T1", "NONE");
        present_oserdes.apply_enum_diff(ctx.tiledb.item(tile, bel, "OMUX"), "D1", "NONE");
        present_oserdes.apply_enum_diff(ctx.tiledb.item(tile, bel, "TMUX"), "T1", "NONE");
        present_oserdes.apply_bit_diff(ctx.tiledb.item(tile, bel, "INV.D1"), false, true);
        present_ologic.assert_empty();
        present_oserdes.assert_empty();
    }
    let mut present_vr = ctx.state.get_diff(tile, "IOB_COMMON", "PRESENT", "VR");
    // I don't care.
    ctx.state
        .get_diff(tile, "IOB_COMMON", "PRESENT", "VR_CENTER");
    for i in 0..2 {
        let bel = &format!("IOB{i}");
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "IOB");
        ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
        let item = ctx.extract_bit_wide(tile, bel, "OUSED", "0");
        assert_eq!(item.bits.len(), 2);
        ctx.tiledb.insert(tile, bel, "OUTPUT_ENABLE", item);
        ctx.state
            .get_diff(tile, bel, "GTSATTRBOX", "DISABLE_GTS")
            .assert_empty();
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
        let lvds = TileItem::from_bitvec(oprog.bits[0..4].to_vec(), false);
        let dci_t = TileItem::from_bit(oprog.bits[4], false);
        let dci_mode = TileItem {
            bits: oprog.bits[5..8].to_vec(),
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
        let output_misc = TileItem::from_bitvec(oprog.bits[8..10].to_vec(), false);
        let dci_misc = TileItem::from_bitvec(oprog.bits[10..12].to_vec(), false);
        let pdrive_bits = oprog.bits[12..17].to_vec();
        let ndrive_bits = oprog.bits[17..22].to_vec();
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
                    TileBit::new(0, 26, 0),
                    TileBit::new(0, 26, 6),
                    TileBit::new(0, 26, 12),
                    TileBit::new(0, 26, 18),
                ],
                vec![
                    TileBit::new(0, 26, 1),
                    TileBit::new(0, 26, 7),
                    TileBit::new(0, 26, 13),
                    TileBit::new(0, 25, 19),
                ],
            )
        } else {
            (
                vec![
                    TileBit::new(0, 26, 79),
                    TileBit::new(0, 26, 73),
                    TileBit::new(0, 26, 67),
                    TileBit::new(0, 26, 61),
                ],
                vec![
                    TileBit::new(0, 26, 78),
                    TileBit::new(0, 26, 72),
                    TileBit::new(0, 26, 66),
                    TileBit::new(0, 25, 60),
                ],
            )
        };
        let pslew_invert = bitvec![0, 0, 0, 0];
        let nslew_invert = bitvec![0, 1, 0, 0];

        let mut ibuf_mode = vec![("OFF", Diff::default())];

        for &std in IOSTDS {
            let mut diff = ctx.state.get_diff(tile, bel, "ISTD", std.name);
            match std.dci {
                DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                DciKind::InputVcc | DciKind::BiVcc => {
                    diff.apply_enum_diff(&dci_mode, "TERM_VCC", "NONE");
                    diff.apply_bitvec_diff(&dci_misc, &bitvec![1, 1], &bitvec![0, 0]);
                }
                DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                    diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                }
            }
            let mode = if std.diff != DiffKind::None {
                "DIFF"
            } else if std.vref.is_some() {
                "VREF"
            } else {
                "CMOS"
            };
            ibuf_mode.push((mode, diff));

            if std.diff == DiffKind::True {
                let stdname = std.name;
                let diff = ctx.state.get_diff(tile, bel, "OSTD", std.name);
                let value = extract_bitvec_val(&lvds, &bitvec![0; 4], diff);
                let tc = ['C', 'T'][i];
                ctx.tiledb
                    .insert_misc_data(format!("IOSTD:LVDS_{tc}:OUTPUT_{stdname}"), value);
                if std.dci == DciKind::None {
                    let diff = ctx.state.get_diff(tile, bel, "DIFF_TERM", std.name);
                    let value = extract_bitvec_val(&lvds, &bitvec![0; 4], diff);
                    let tc = ['C', 'T'][i];
                    ctx.tiledb
                        .insert_misc_data(format!("IOSTD:LVDS_{tc}:TERM_{stdname}"), value);
                }
            } else {
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
                        for (attr, bits, invert) in [
                            ("PSLEW", &pslew_bits, &pslew_invert),
                            ("NSLEW", &nslew_bits, &nslew_invert),
                        ] {
                            let value: BitVec = bits
                                .iter()
                                .zip(invert.iter())
                                .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                    Some(true) => !*inv,
                                    None => *inv,
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
        }
        ctx.tiledb
            .insert(tile, bel, "IBUF_MODE", xlat_enum(ibuf_mode));

        for (attr, bits, invert) in [
            ("PDRIVE", &pdrive_bits, &pdrive_invert),
            ("NDRIVE", &ndrive_bits, &ndrive_invert),
            ("PSLEW", &pslew_bits, &pslew_invert),
            ("NSLEW", &nslew_bits, &nslew_invert),
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
                ("PSLEW", &pslew_bits, &pslew_invert),
                ("NSLEW", &nslew_bits, &nslew_invert),
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
                    .insert_misc_data(format!("IOSTD:{attr}:VREF"), value);
            }
            present_vref.assert_empty();
        }

        ctx.tiledb
            .insert_misc_data("IOSTD:OUTPUT_MISC:OFF", bitvec![0; 2]);
        ctx.tiledb
            .insert_misc_data("IOSTD:LVDS_T:OFF", bitvec![0; 4]);
        ctx.tiledb
            .insert_misc_data("IOSTD:LVDS_C:OFF", bitvec![0; 4]);
        ctx.tiledb
            .insert_misc_data("IOSTD:PDRIVE:OFF", bitvec![0; 5]);
        ctx.tiledb
            .insert_misc_data("IOSTD:NDRIVE:OFF", bitvec![0; 5]);
        ctx.tiledb
            .insert_misc_data("IOSTD:PSLEW:OFF", pslew_invert.clone());
        ctx.tiledb
            .insert_misc_data("IOSTD:NSLEW:OFF", nslew_invert.clone());
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
        ctx.tiledb.insert(
            tile,
            bel,
            "PSLEW",
            TileItem {
                bits: pslew_bits,
                kind: TileItemKind::BitVec {
                    invert: pslew_invert,
                },
            },
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "NSLEW",
            TileItem {
                bits: nslew_bits,
                kind: TileItemKind::BitVec {
                    invert: nslew_invert,
                },
            },
        );
        present.assert_empty();
    }
    let diff1 = present_vr.split_bits_by(|bit| bit.bit >= 40);
    ctx.tiledb.insert(tile, "IOB0", "VR", xlat_bit(present_vr));
    ctx.tiledb.insert(tile, "IOB1", "VR", xlat_bit(diff1));

    let tile = "HCLK_IOIS_LVDS";
    let bel = "LVDS";
    let item = TileItem::from_bitvec(
        vec![
            TileBit::new(0, 5, 12),
            TileBit::new(0, 5, 14),
            TileBit::new(0, 3, 15),
            TileBit::new(0, 2, 13),
            TileBit::new(0, 3, 14),
            TileBit::new(0, 3, 13),
            TileBit::new(0, 5, 15),
            TileBit::new(0, 2, 14),
            TileBit::new(0, 11, 13),
            TileBit::new(0, 3, 12),
        ],
        false,
    );
    for std in IOSTDS {
        if std.diff == DiffKind::True {
            let diff = ctx.state.get_diff(tile, bel, "STD", std.name);
            let val = extract_bitvec_val(&item, &bitvec![0; 10], diff);
            ctx.tiledb
                .insert_misc_data(format!("IOSTD:LVDSBIAS:{}", std.name), val);
        }
    }
    ctx.tiledb.insert(tile, bel, "LVDSBIAS", item);
    ctx.tiledb
        .insert_misc_data("IOSTD:LVDSBIAS:OFF", bitvec![0; 10]);

    let hclk_center_cnt =
        ctx.edev.egrid().node_index[ctx.edev.egrid().db.get_node("HCLK_CENTER")].len();
    for tile in [
        "HCLK_IOIS_DCI",
        "HCLK_CENTER",
        "HCLK_CENTER_ABOVE_CFG",
        "HCLK_IOBDCM",
        "HCLK_DCMIOB",
    ] {
        let bel = "DCI";
        ctx.tiledb.insert(
            tile,
            bel,
            "PREF",
            TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 1, 15),
                    TileBit::new(0, 1, 14),
                    TileBit::new(0, 1, 13),
                    TileBit::new(0, 1, 12),
                ],
                false,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "NREF",
            TileItem::from_bitvec(
                vec![TileBit::new(0, 27, 15), TileBit::new(0, 27, 12)],
                false,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "LVDIV2",
            TileItem::from_bitvec(
                vec![TileBit::new(0, 27, 13), TileBit::new(0, 27, 14)],
                false,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "PMASK_TERM_VCC",
            TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 4, 12),
                    TileBit::new(0, 4, 13),
                    TileBit::new(0, 4, 14),
                    TileBit::new(0, 4, 15),
                    TileBit::new(0, 2, 12),
                ],
                false,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "PMASK_TERM_SPLIT",
            TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 10, 13),
                    TileBit::new(0, 10, 14),
                    TileBit::new(0, 11, 14),
                    TileBit::new(0, 10, 15),
                    TileBit::new(0, 11, 15),
                ],
                false,
            ),
        );
        ctx.tiledb.insert(
            tile,
            bel,
            "NMASK_TERM_SPLIT",
            TileItem::from_bitvec(
                vec![
                    TileBit::new(0, 12, 12),
                    TileBit::new(0, 12, 13),
                    TileBit::new(0, 12, 14),
                    TileBit::new(0, 12, 15),
                    TileBit::new(0, 10, 12),
                ],
                false,
            ),
        );
        ctx.collect_bit(tile, bel, "QUIET", "1");

        let enable = if (tile == "HCLK_CENTER_ABOVE_CFG" && hclk_center_cnt != 1)
            || tile == "HCLK_IOIS_DCI"
        {
            TileItem::from_bit(TileBit::new(0, 0, 14), false)
        } else {
            ctx.extract_bit(tile, bel, "ENABLE", "1")
        };
        let mut test_enable = ctx.state.get_diff(tile, bel, "TEST_ENABLE", "1");
        test_enable.apply_bit_diff(&enable, true, false);
        ctx.tiledb.insert(tile, bel, "ENABLE", enable);
        ctx.tiledb
            .insert(tile, bel, "TEST_ENABLE", xlat_bit_wide(test_enable));
        if tile == "HCLK_CENTER" {
            if hclk_center_cnt > 1 {
                ctx.collect_bit(tile, bel, "CASCADE_FROM_BELOW", "1");
            }
            if hclk_center_cnt > 3 {
                ctx.collect_bit(tile, bel, "CASCADE_FROM_ABOVE", "1");
            }
        }
        if tile == "HCLK_CENTER_ABOVE_CFG" && hclk_center_cnt > 1 {
            ctx.collect_bit(tile, bel, "CASCADE_FROM_ABOVE", "1");
        }
    }
    let tile = "HCLK_IOIS_DCI";
    let bel = "DCI";
    for std in IOSTDS {
        if std.dci == DciKind::None {
            continue;
        }
        let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
        let mut diff = ctx.state.get_diff(tile, bel, "STD", std.name);
        match std.dci {
            DciKind::OutputHalf => {
                let val = extract_bitvec_val_part(
                    ctx.tiledb.item(tile, bel, "LVDIV2"),
                    &bitvec![0; 2],
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
    ctx.tiledb
        .insert_misc_data("IOSTD:DCI:LVDIV2:OFF", bitvec![0; 2]);
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
