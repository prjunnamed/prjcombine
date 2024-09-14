use bitvec::prelude::*;
use prjcombine_hammer::Session;
use prjcombine_int::db::BelId;
use prjcombine_types::{TileItem, TileItemKind};
use unnamed_entity::EntityId;

use crate::{
    backend::{FeatureBit, IseBackend},
    diff::{
        extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_wide, xlat_enum,
        CollectorCtx, Diff,
    },
    fgen::{BelKV, ExtraFeature, ExtraFeatureKind, TileBits, TileKV},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_multi_attr_bin, fuzz_one, fuzz_one_extras,
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
    // TODO: ILOGIC
    // TODO: OLOGIC
    // TODO: IODELAY
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

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "IO";
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
                    FeatureBit::new(0, 37, 17),
                    FeatureBit::new(0, 36, 23),
                    FeatureBit::new(0, 37, 23),
                    FeatureBit::new(0, 37, 30),
                    FeatureBit::new(0, 37, 29),
                    FeatureBit::new(0, 37, 27),
                ],
                vec![
                    FeatureBit::new(0, 36, 31),
                    FeatureBit::new(0, 36, 27),
                    FeatureBit::new(0, 37, 31),
                    FeatureBit::new(0, 37, 28),
                    FeatureBit::new(0, 36, 26),
                    FeatureBit::new(0, 37, 20),
                ],
            )
        } else {
            (
                vec![
                    FeatureBit::new(0, 37, 46),
                    FeatureBit::new(0, 36, 40),
                    FeatureBit::new(0, 37, 40),
                    FeatureBit::new(0, 37, 33),
                    FeatureBit::new(0, 37, 34),
                    FeatureBit::new(0, 37, 36),
                ],
                vec![
                    FeatureBit::new(0, 36, 32),
                    FeatureBit::new(0, 36, 36),
                    FeatureBit::new(0, 37, 32),
                    FeatureBit::new(0, 37, 35),
                    FeatureBit::new(0, 36, 37),
                    FeatureBit::new(0, 37, 43),
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
            FeatureBit::new(0, 35, 15),
            FeatureBit::new(0, 34, 15),
            FeatureBit::new(0, 34, 14),
            FeatureBit::new(0, 35, 14),
            FeatureBit::new(0, 35, 13),
            FeatureBit::new(0, 34, 13),
            FeatureBit::new(0, 34, 12),
            FeatureBit::new(0, 35, 12),
            FeatureBit::new(0, 32, 13),
            FeatureBit::new(0, 33, 13),
            FeatureBit::new(0, 33, 12),
            FeatureBit::new(0, 32, 12),
        ],
        false,
    );
    let lvdiv2 = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 52, 12),
            FeatureBit::new(0, 53, 12),
            FeatureBit::new(0, 53, 15),
        ],
        false,
    );
    let pref = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 51, 12),
            FeatureBit::new(0, 50, 12),
            FeatureBit::new(0, 53, 14),
            FeatureBit::new(0, 52, 15),
        ],
        false,
    );
    let nref = TileItem::from_bitvec(
        vec![FeatureBit::new(0, 52, 14), FeatureBit::new(0, 52, 13)],
        false,
    );
    let pmask_term_vcc = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 50, 15),
            FeatureBit::new(0, 50, 14),
            FeatureBit::new(0, 51, 14),
            FeatureBit::new(0, 51, 13),
            FeatureBit::new(0, 50, 13),
        ],
        false,
    );
    let pmask_term_split = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 46, 13),
            FeatureBit::new(0, 46, 12),
            FeatureBit::new(0, 47, 12),
            FeatureBit::new(0, 48, 15),
            FeatureBit::new(0, 49, 15),
        ],
        false,
    );
    let nmask_term_split = TileItem::from_bitvec(
        vec![
            FeatureBit::new(0, 48, 13),
            FeatureBit::new(0, 49, 13),
            FeatureBit::new(0, 49, 12),
            FeatureBit::new(0, 48, 12),
            FeatureBit::new(0, 51, 15),
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
