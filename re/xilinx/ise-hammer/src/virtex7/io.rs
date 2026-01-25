use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{CellCoord, DieId, TileCoord, TileIobId};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, FeatureId, OcdMode, extract_bitvec_val, extract_bitvec_val_part, xlat_bit,
    xlat_bit_wide, xlat_bitvec, xlat_bool, xlat_enum, xlat_enum_ocd,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::{chip::RegId, defs, expanded::IoCoord};

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        iostd::{DciKind, DiffKind, Iostd},
        props::{
            DynProp,
            bel::{BaseBelAttr, BaseBelMode, BaseBelPin, BaseBelPinPair},
            mutex::TileMutex,
            relation::{Delta, Related},
        },
    },
    virtex4::io::IsBonded,
    virtex5::io::{DiffOut, HclkIoi, VrefInternal},
};

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

#[derive(Clone, Copy, Debug)]
struct VccoSenseMode(&'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VccoSenseMode {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let bank = edev
            .get_io_info(IoCoord {
                cell: tcrd.cell,
                iob: TileIobId::from_idx(0),
            })
            .bank;
        Some((fuzzer.fuzz(Key::VccoSenseMode(bank), None, self.0), false))
    }
}

fn get_vrefs(backend: &IseBackend, tcrd: TileCoord) -> Vec<TileCoord> {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let chip = edev.chips[tcrd.die];

    let reg = chip.row_to_reg(tcrd.row);
    let bot = chip.row_reg_bot(reg);
    [bot + 11, bot + 37]
        .into_iter()
        .map(|vref_row| tcrd.with_row(vref_row).tile(defs::tslots::BEL))
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Vref(bool);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Vref {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };

        let vrefs = get_vrefs(backend, tcrd);
        if vrefs.contains(&tcrd) {
            return None;
        }
        let chip = edev.chips[tcrd.die];

        let hclk_row = chip.row_hclk(tcrd.row);
        // Take exclusive mutex on VREF.
        let hclk_ioi = tcrd.with_row(hclk_row).tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "VREF".to_string()),
            None,
            "EXCLUSIVE",
        );
        for vref in vrefs {
            let site = backend
                .ngrid
                .get_bel_name(vref.cell.bel(defs::bslots::IOB[0]))
                .unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
            if self.0 {
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::Legacy(FeatureId {
                        tile: edev.db.tile_classes.key(edev[vref].class).clone(),
                        bel: "IOB[0]".into(),
                        attr: "PRESENT".into(),
                        val: "VREF".into(),
                    }),
                    rects: backend.edev.tile_bits(vref),
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
struct Dci(Option<&'static str>);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for Dci {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[tcrd.die];

        // Avoid anchor bank.
        let anchor_reg = if chip.has_ps {
            RegId::from_idx(chip.regs - 1)
        } else {
            RegId::from_idx(0)
        };
        if tcrd.col == edev.col_rio.unwrap() && chip.row_to_reg(tcrd.row) == anchor_reg {
            return None;
        }

        // Ensure nothing is placed in VR.
        for row in [chip.row_hclk(tcrd.row) - 25, chip.row_hclk(tcrd.row) + 24] {
            let vr_tile = tcrd.with_row(row).tile(defs::tslots::BEL);
            let vr_bel = vr_tile.cell.bel(defs::bslots::IOB[0]);
            let site = backend.ngrid.get_bel_name(vr_bel).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
            // Test VR.
            if self.0.is_some() {
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::Legacy(FeatureId {
                        tile: edev.db.tile_classes.key(edev[vr_tile].class).clone(),
                        bel: "IOB[0]".into(),
                        attr: "PRESENT".into(),
                        val: "VR".into(),
                    }),
                    rects: edev.tile_bits(vr_tile),
                });
            }
        }

        // Take exclusive mutex on bank DCI.
        let hclk_ioi = tcrd
            .cell
            .with_row(chip.row_hclk(tcrd.row))
            .tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "BANK_DCI".to_string()),
            None,
            "EXCLUSIVE",
        );
        // Test bank DCI.
        if let Some(std) = self.0 {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: "HCLK_IO_HP".into(),
                    bel: "DCI".into(),
                    attr: "STD".into(),
                    val: std.into(),
                }),
                rects: edev.tile_bits(hclk_ioi),
            });
        }

        // Take shared mutex on global DCI.
        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");

        // Anchor global DCI by putting something in arbitrary bank.
        let iob_anchor = tcrd
            .cell
            .with_cr(edev.col_rio.unwrap(), chip.row_reg_bot(anchor_reg) + 1)
            .bel(defs::bslots::IOB[0]);
        let site = backend.ngrid.get_bel_name(iob_anchor).unwrap();
        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_18");
        // Ensure anchor VR IOBs are free.
        for row in [
            chip.row_reg_hclk(anchor_reg) - 25,
            chip.row_reg_hclk(anchor_reg) + 24,
        ] {
            let iob_anchor_vr = tcrd
                .cell
                .with_cr(edev.col_rio.unwrap(), row)
                .bel(defs::bslots::IOB[0]);
            let site = backend.ngrid.get_bel_name(iob_anchor_vr).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Make note of anchor VCCO.
        let hclk_ioi_anchor = tcrd
            .cell
            .with_cr(edev.col_rio.unwrap(), chip.row_reg_hclk(anchor_reg))
            .tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.base(Key::TileMutex(hclk_ioi_anchor, "VCCO".to_string()), "1800");

        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

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
    for (tile, num_io) in [
        ("IO_HR_PAIR", 2),
        ("IO_HR_S", 1),
        ("IO_HR_N", 1),
        ("IO_HP_PAIR", 2),
        ("IO_HP_S", 1),
        ("IO_HP_N", 1),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tile) else {
            continue;
        };
        for i in 0..num_io {
            let mut bctx = ctx.bel(defs::bslots::ILOGIC[i]);
            let bel_ologic = defs::bslots::OLOGIC[i];

            bctx.test_manual("PRESENT", "ILOGICE2")
                .mode("ILOGICE2")
                .commit();
            bctx.test_manual("PRESENT", "ISERDESE2")
                .mode("ISERDESE2")
                .commit();

            bctx.mode("ISERDESE2").test_inv("D");
            bctx.mode("ISERDESE2").test_inv("CLK");
            bctx.mode("ISERDESE2")
                .attr("DATA_RATE", "SDR")
                .test_inv("OCLK");
            bctx.mode("ISERDESE2")
                .attr("DYN_CLKDIV_INV_EN", "FALSE")
                .test_inv("CLKDIV");
            bctx.mode("ISERDESE2")
                .attr("DYN_CLKDIVP_INV_EN", "FALSE")
                .test_inv("CLKDIVP");
            bctx.mode("ISERDESE2")
                .test_enum("DYN_CLK_INV_EN", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum("DYN_CLKDIV_INV_EN", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum("DYN_CLKDIVP_INV_EN", &["FALSE", "TRUE"]);

            bctx.mode("ILOGICE2")
                .attr("IFFTYPE", "#FF")
                .pin("SR")
                .test_enum("SRUSED", &["0"]);
            bctx.mode("ISERDESE2")
                .attr("DATA_WIDTH", "2")
                .attr("DATA_RATE", "SDR")
                .test_enum("SERDES", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum("SERDES_MODE", &["MASTER", "SLAVE"]);
            bctx.mode("ISERDESE2").attr("SERDES", "FALSE").test_enum(
                "DATA_WIDTH",
                &["2", "3", "4", "5", "6", "7", "8", "10", "14"],
            );
            bctx.mode("ISERDESE2").test_enum("NUM_CE", &["1", "2"]);

            for attr in [
                "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
                "SRVAL_Q4",
            ] {
                bctx.mode("ISERDESE2").test_enum(attr, &["0", "1"]);
            }

            bctx.mode("ILOGICE2")
                .attr("IFFTYPE", "#FF")
                .test_enum("SRTYPE", &["SYNC", "ASYNC"]);
            bctx.mode("ISERDESE2")
                .test_enum("SRTYPE", &["SYNC", "ASYNC"]);

            bctx.mode("ISERDESE2")
                .test_enum("D_EMU1", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum("D_EMU2", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum("RANK23_DLY", &["FALSE", "TRUE"]);

            bctx.mode("ISERDESE2").test_enum(
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
                .test_manual("INTERFACE_TYPE", "MEMORY_DDR3_V6")
                .attr("INTERFACE_TYPE", "MEMORY_DDR3")
                .attr("DDR3_V6", "TRUE")
                .commit();
            bctx.mode("ISERDESE2")
                .test_enum("DATA_RATE", &["SDR", "DDR"]);
            bctx.mode("ISERDESE2").test_enum(
                "DDR_CLK_EDGE",
                &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
            );
            bctx.mode("ILOGICE2").attr("IFFTYPE", "DDR").test_enum(
                "DDR_CLK_EDGE",
                &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
            );
            bctx.mode("ILOGICE2")
                .test_enum("IFFTYPE", &["#FF", "#LATCH", "DDR"]);

            bctx.mode("ISERDESE2")
                .pin("OFB")
                .test_enum("OFB_USED", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .pin("TFB")
                .test_enum("TFB_USED", &["FALSE", "TRUE"]);
            bctx.mode("ISERDESE2")
                .test_enum("IOBDELAY", &["NONE", "IFD", "IBUF", "BOTH"]);

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
                .test_enum("D2OBYP_SEL", &["GND", "T"]);
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
                .test_enum("D2OFFBYP_SEL", &["GND", "T"]);
            bctx.mode("ILOGICE2")
                .attr("IDELMUX", "1")
                .attr("DINV", "")
                .pin("D")
                .pin("DDLY")
                .pin("O")
                .pin("TFB")
                .pin("OFB")
                .test_enum("IMUX", &["0", "1"]);
            bctx.mode("ILOGICE2")
                .attr("IFFDELMUX", "1")
                .attr("IFFTYPE", "#FF")
                .attr("DINV", "")
                .pin("D")
                .pin("DDLY")
                .pin("TFB")
                .pin("OFB")
                .test_enum("IFFMUX", &["0", "1"]);
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
                .test_enum("IDELMUX", &["0", "1"]);
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
                .test_enum("IFFDELMUX", &["0", "1"]);

            if tile.contains("HR") {
                bctx.test_manual("PRESENT", "ILOGICE3")
                    .mode("ILOGICE3")
                    .commit();
                for val in ["D", "D_B"] {
                    bctx.mode("ILOGICE3")
                        .attr("ZHOLD_IFF", "TRUE")
                        .attr("IFFTYPE", "#FF")
                        .pin("Q1")
                        .test_manual("ZHOLD_IFF_INV", val)
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
                    .test_enum("ZHOLD_FABRIC_INV", &["D", "D_B"]);
                bctx.mode("ILOGICE3")
                    .attr("ZHOLD_IFF", "")
                    .test_enum("ZHOLD_FABRIC", &["FALSE", "TRUE"]);
                bctx.mode("ILOGICE3")
                    .attr("ZHOLD_FABRIC", "")
                    .test_enum("ZHOLD_IFF", &["FALSE", "TRUE"]);
                bctx.mode("ILOGICE3").test_multi_attr_dec("IDELAY_VALUE", 5);
                bctx.mode("ILOGICE3")
                    .test_multi_attr_dec("IFFDELAY_VALUE", 5);
            }

            for pin in ["CKINT0", "CKINT1", "PHASER_ICLK"] {
                bctx.build()
                    .mutex("MUX.CLK", pin)
                    .pip("CLKB", pin)
                    .test_manual("MUX.CLK", pin)
                    .pip("CLK", pin)
                    .commit();
                bctx.build()
                    .mutex("MUX.CLK", pin)
                    .test_manual("MUX.CLKB", pin)
                    .pip("CLKB", pin)
                    .commit();
            }
            bctx.build()
                .mutex("MUX.CLK", "PHASER_OCLK")
                .pip("CLKB", (bel_ologic, "PHASER_OCLK"))
                .test_manual("MUX.CLK", "PHASER_OCLK")
                .pip("CLK", (bel_ologic, "PHASER_OCLK"))
                .commit();
            bctx.build()
                .mutex("MUX.CLK", "PHASER_OCLK")
                .test_manual("MUX.CLKB", "PHASER_OCLK")
                .pip("CLKB", (bel_ologic, "PHASER_OCLK"))
                .commit();
            for (src, num) in [("HCLK", 6), ("RCLK", 4), ("IOCLK", 4)] {
                for i in 0..num {
                    bctx.build()
                        .mutex("MUX.CLK", format!("{src}{i}"))
                        .pip("CLKB", (defs::bslots::IOI, format!("{src}{i}")))
                        .test_manual("MUX.CLK", format!("{src}{i}"))
                        .pip("CLK", (defs::bslots::IOI, format!("{src}{i}")))
                        .commit();
                    bctx.build()
                        .mutex("MUX.CLK", format!("{src}{i}"))
                        .test_manual("MUX.CLKB", format!("{src}{i}"))
                        .pip("CLKB", (defs::bslots::IOI, format!("{src}{i}")))
                        .commit();
                }
            }

            bctx.build()
                .mutex("MUX.CLKDIVP", "CLKDIV")
                .test_manual("MUX.CLKDIVP", "CLKDIV")
                .pin_pips("CLKDIVP")
                .commit();
            bctx.build()
                .mutex("MUX.CLKDIVP", "PHASER")
                .test_manual("MUX.CLKDIVP", "PHASER")
                .pip("CLKDIVP", "PHASER_ICLKDIV")
                .commit();
        }
        for i in 0..num_io {
            let mut bctx = ctx.bel(defs::bslots::OLOGIC[i]);

            bctx.test_manual("PRESENT", "OLOGICE2")
                .mode("OLOGICE2")
                .commit();
            bctx.test_manual("PRESENT", "OSERDESE2")
                .mode("OSERDESE2")
                .commit();

            for pin in [
                "D1", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "T1", "T2", "T3", "T4", "CLKDIV",
                "CLKDIVF",
            ] {
                bctx.mode("OSERDESE2").test_inv(pin);
            }
            bctx.mode("OSERDESE2")
                .attr("DATA_RATE_OQ", "DDR")
                .attr("DDR_CLK_EDGE", "SAME_EDGE")
                .pin("OCE")
                .pin("CLK")
                .test_enum_suffix("CLKINV", "SAME", &["CLK", "CLK_B"]);
            bctx.mode("OSERDESE2")
                .attr("DATA_RATE_OQ", "DDR")
                .attr("DDR_CLK_EDGE", "OPPOSITE_EDGE")
                .pin("OCE")
                .pin("CLK")
                .test_enum_suffix("CLKINV", "OPPOSITE", &["CLK", "CLK_B"]);

            bctx.mode("OLOGICE2")
                .attr("OUTFFTYPE", "#FF")
                .test_enum("SRTYPE_OQ", &["SYNC", "ASYNC"]);
            bctx.mode("OLOGICE2")
                .attr("TFFTYPE", "#FF")
                .test_enum("SRTYPE_TQ", &["SYNC", "ASYNC"]);
            bctx.mode("OSERDESE2")
                .test_enum("SRTYPE", &["SYNC", "ASYNC"]);

            bctx.mode("OLOGICE2")
                .test_enum_suffix("INIT_OQ", "OLOGIC", &["0", "1"]);
            bctx.mode("OLOGICE2")
                .test_enum_suffix("INIT_TQ", "OLOGIC", &["0", "1"]);
            bctx.mode("OSERDESE2")
                .test_enum_suffix("INIT_OQ", "OSERDES", &["0", "1"]);
            bctx.mode("OSERDESE2")
                .test_enum_suffix("INIT_TQ", "OSERDES", &["0", "1"]);
            bctx.mode("OLOGICE2")
                .test_enum_suffix("SRVAL_OQ", "OLOGIC", &["0", "1"]);
            bctx.mode("OLOGICE2")
                .test_enum_suffix("SRVAL_TQ", "OLOGIC", &["0", "1"]);
            bctx.mode("OSERDESE2")
                .test_enum_suffix("SRVAL_OQ", "OSERDES", &["0", "1"]);
            bctx.mode("OSERDESE2")
                .test_enum_suffix("SRVAL_TQ", "OSERDES", &["0", "1"]);

            for attr in ["OSRUSED", "TSRUSED"] {
                bctx.mode("OLOGICE2")
                    .attr("OUTFFTYPE", "#FF")
                    .attr("TFFTYPE", "#FF")
                    .pin("OCE")
                    .pin("TCE")
                    .pin("REV")
                    .pin("SR")
                    .test_enum(attr, &["0"]);
            }

            bctx.mode("OLOGICE2")
                .pin("OQ")
                .test_enum("OUTFFTYPE", &["#FF", "#LATCH", "DDR"]);
            bctx.mode("OLOGICE2")
                .pin("TQ")
                .test_enum("TFFTYPE", &["#FF", "#LATCH", "DDR"]);
            bctx.mode("OLOGICE2")
                .test_manual("OMUX", "D1")
                .attr("OQUSED", "0")
                .attr("O1USED", "0")
                .attr("D1INV", "D1")
                .attr("OMUX", "D1")
                .pin("OQ")
                .pin("D1")
                .commit();

            bctx.mode("OSERDESE2")
                .test_enum("DATA_RATE_OQ", &["SDR", "DDR"]);
            bctx.mode("OSERDESE2")
                .test_enum("DATA_RATE_TQ", &["BUF", "SDR", "DDR"]);

            bctx.mode("OLOGICE2")
                .global("ENABLEMISR", "Y")
                .test_enum("MISR_ENABLE", &["FALSE", "TRUE"]);
            bctx.mode("OLOGICE2")
                .global("ENABLEMISR", "Y")
                .test_enum("MISR_ENABLE_FDBK", &["FALSE", "TRUE"]);
            bctx.mode("OLOGICE2")
                .global("ENABLEMISR", "Y")
                .test_enum("MISR_CLK_SELECT", &["CLK1", "CLK2"]);

            bctx.mode("OSERDESE2")
                .test_enum("SERDES", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum("SERDES_MODE", &["SLAVE", "MASTER"]);
            bctx.mode("OSERDESE2")
                .test_enum("SELFHEAL", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum("RANK3_USED", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum("TBYTE_CTL", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum("TBYTE_SRC", &["FALSE", "TRUE"]);
            bctx.mode("OSERDESE2")
                .test_enum("TRISTATE_WIDTH", &["1", "4"]);
            bctx.mode("OSERDESE2")
                .attr("DATA_RATE_OQ", "SDR")
                .test_enum_suffix("DATA_WIDTH", "SDR", &["2", "3", "4", "5", "6", "7", "8"]);
            bctx.mode("OSERDESE2")
                .attr("DATA_RATE_OQ", "DDR")
                .test_enum_suffix("DATA_WIDTH", "DDR", &["4", "6", "8", "10", "14"]);

            bctx.build()
                .mutex("MUX.CLK", "CKINT")
                .pip("CLKM", "CLK_CKINT")
                .test_manual("MUX.CLK", "CKINT")
                .pip("CLK_MUX", "CLK_CKINT")
                .commit();
            bctx.build()
                .mutex("MUX.CLK", "CKINT")
                .test_manual("MUX.CLKB", "CKINT")
                .pip("CLKM", "CLK_CKINT")
                .commit();
            bctx.build()
                .mutex("MUX.CLK", "PHASER_OCLK")
                .pip("CLKM", "PHASER_OCLK")
                .test_manual("MUX.CLK", "PHASER_OCLK")
                .pip("CLK_MUX", "PHASER_OCLK")
                .commit();
            bctx.build()
                .mutex("MUX.CLK", "PHASER_OCLK")
                .test_manual("MUX.CLKB", "PHASER_OCLK")
                .pip("CLKM", "PHASER_OCLK")
                .commit();
            bctx.build()
                .mutex("MUX.CLK", "PHASER_OCLK90")
                .pip("CLKM", "PHASER_OCLK")
                .test_manual("MUX.CLK", "PHASER_OCLK90")
                .pip("CLK_MUX", "PHASER_OCLK90")
                .commit();
            bctx.build()
                .mutex("MUX.CLK", "PHASER_OCLK90.BOTH")
                .test_manual("MUX.CLK", "PHASER_OCLK90.BOTH")
                .pip("CLK_MUX", "PHASER_OCLK90")
                .commit();
            for (src, num) in [("HCLK", 6), ("RCLK", 4), ("IOCLK", 4)] {
                for i in 0..num {
                    bctx.build()
                        .mutex("MUX.CLK", format!("{src}{i}"))
                        .pip("CLKM", (defs::bslots::IOI, format!("{src}{i}")))
                        .test_manual("MUX.CLK", format!("{src}{i}"))
                        .pip("CLK_MUX", (defs::bslots::IOI, format!("{src}{i}")))
                        .commit();
                    bctx.build()
                        .mutex("MUX.CLK", format!("{src}{i}"))
                        .test_manual("MUX.CLKB", format!("{src}{i}"))
                        .pip("CLKM", (defs::bslots::IOI, format!("{src}{i}")))
                        .commit();
                }
            }

            bctx.build()
                .mutex("MUX.CLKDIV", "PHASER_OCLKDIV")
                .pip("CLKDIVB", "PHASER_OCLKDIV")
                .test_manual("MUX.CLKDIV", "PHASER_OCLKDIV")
                .pip("CLKDIV", "PHASER_OCLKDIV")
                .commit();
            bctx.build()
                .mutex("MUX.CLKDIV", "PHASER_OCLKDIV")
                .test_manual("MUX.CLKDIVB", "PHASER_OCLKDIV")
                .pip("CLKDIVB", "PHASER_OCLKDIV")
                .commit();
            bctx.build()
                .mutex("MUX.CLKDIV", "CKINT")
                .pip("CLKDIVB", "CLKDIV_CKINT")
                .test_manual("MUX.CLKDIV", "CKINT")
                .pip("CLKDIV", "CLKDIV_CKINT")
                .commit();
            bctx.build()
                .mutex("MUX.CLKDIV", "CKINT")
                .test_manual("MUX.CLKDIVB", "CKINT")
                .pip("CLKDIVB", "CLKDIV_CKINT")
                .commit();
            for (src, num) in [("HCLK", 6), ("RCLK", 4)] {
                for i in 0..num {
                    bctx.build()
                        .mutex("MUX.CLKDIV", format!("{src}{i}"))
                        .pip("CLKDIVB", (defs::bslots::IOI, format!("{src}{i}")))
                        .test_manual("MUX.CLKDIV", format!("{src}{i}"))
                        .pip("CLKDIV", (defs::bslots::IOI, format!("{src}{i}")))
                        .commit();
                    bctx.build()
                        .mutex("MUX.CLKDIV", format!("{src}{i}"))
                        .test_manual("MUX.CLKDIVB", format!("{src}{i}"))
                        .pip("CLKDIVB", (defs::bslots::IOI, format!("{src}{i}")))
                        .commit();
                }
            }
            bctx.build()
                .mutex("MUX.CLKDIV", "HCLK0.F")
                .pip("CLKDIVFB", (defs::bslots::IOI, "HCLK0"))
                .test_manual("MUX.CLKDIV", "HCLK0.F")
                .pip("CLKDIVF", (defs::bslots::IOI, "HCLK0"))
                .commit();
            bctx.build()
                .mutex("MUX.CLKDIV", "HCLK0.F")
                .test_manual("MUX.CLKDIVB", "HCLK0.F")
                .pip("CLKDIVFB", (defs::bslots::IOI, "HCLK0"))
                .commit();
        }
        let setup_idelayctrl: [Box<DynProp>; 4] = [
            Box::new(Related::new(
                HclkIoi,
                TileMutex::new("IDELAYCTRL".into(), "USE".into()),
            )),
            Box::new(Related::new(
                HclkIoi,
                BaseBelMode::new(defs::bslots::IDELAYCTRL, "IDELAYCTRL".into()),
            )),
            Box::new(Related::new(
                HclkIoi,
                BaseBelAttr::new(
                    defs::bslots::IDELAYCTRL,
                    "IDELAYCTRL_EN".into(),
                    "ENABLE".into(),
                ),
            )),
            Box::new(Related::new(
                HclkIoi,
                BaseBelAttr::new(defs::bslots::IDELAYCTRL, "BIAS_MODE".into(), "0".into()),
            )),
        ];
        for i in 0..num_io {
            let mut bctx = ctx.bel(defs::bslots::IDELAY[i]);
            let bel_ologic = defs::bslots::OLOGIC[i];
            bctx.build()
                .props(setup_idelayctrl.clone())
                .test_manual("ENABLE", "1")
                .mode("IDELAYE2")
                .commit();
            for pin in ["C", "IDATAIN", "DATAIN"] {
                bctx.mode("IDELAYE2")
                    .props(setup_idelayctrl.clone())
                    .attr("CINVCTRL_SEL", "FALSE")
                    .test_inv(pin);
            }
            for attr in [
                "HIGH_PERFORMANCE_MODE",
                "CINVCTRL_SEL",
                "DELAYCHAIN_OSC",
                "PIPE_SEL",
            ] {
                bctx.mode("IDELAYE2")
                    .props(setup_idelayctrl.clone())
                    .test_enum(attr, &["FALSE", "TRUE"]);
            }
            bctx.mode("IDELAYE2")
                .props(setup_idelayctrl.clone())
                .test_enum(
                    "IDELAY_TYPE",
                    &["FIXED", "VARIABLE", "VAR_LOAD", "VAR_LOAD_PIPE"],
                );
            bctx.mode("IDELAYE2")
                .props(setup_idelayctrl.clone())
                .test_enum("DELAY_SRC", &["DATAIN", "IDATAIN"]);
            bctx.build()
                .attr("DELAY_SRC", "")
                .test_manual("DELAY_SRC", "OFB")
                .pip("IDATAIN", (bel_ologic, "OFB"))
                .commit();
            bctx.mode("IDELAYE2")
                .props(setup_idelayctrl.clone())
                .attr("DELAY_SRC", "IDATAIN")
                .attr("IDELAY_TYPE", "FIXED")
                .test_multi_attr_dec("IDELAY_VALUE", 5);
            if tile.contains("HP") {
                bctx.mode("IDELAYE2_FINEDELAY")
                    .props(setup_idelayctrl.clone())
                    .test_enum("FINEDELAY", &["BYPASS", "ADD_DLY"]);
            }
        }
        if tile.contains("HP") {
            for i in 0..num_io {
                let mut bctx = ctx.bel(defs::bslots::ODELAY[i]);
                bctx.build()
                    .props(setup_idelayctrl.clone())
                    .test_manual("PRESENT", "1")
                    .mode("ODELAYE2")
                    .commit();
                for pin in ["C", "ODATAIN"] {
                    bctx.mode("ODELAYE2")
                        .props(setup_idelayctrl.clone())
                        .attr("CINVCTRL_SEL", "FALSE")
                        .test_inv(pin);
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
                        .test_enum(attr, &["FALSE", "TRUE"]);
                }
                bctx.mode("ODELAYE2")
                    .props(setup_idelayctrl.clone())
                    .attr("DELAY_SRC", "ODATAIN")
                    .attr("PIPE_SEL", "FALSE")
                    .test_enum("ODELAY_TYPE", &["FIXED", "VARIABLE", "VAR_LOAD"]);
                bctx.mode("ODELAYE2")
                    .props(setup_idelayctrl.clone())
                    .attr("DELAYCHAIN_OSC", "")
                    .test_enum("DELAY_SRC", &["ODATAIN", "CLKIN"]);
                bctx.mode("ODELAYE2")
                    .props(setup_idelayctrl.clone())
                    .attr("DELAY_SRC", "ODATAIN")
                    .attr("ODELAY_TYPE", "FIXED")
                    .test_multi_attr_dec("ODELAY_VALUE", 5);
                bctx.mode("ODELAYE2_FINEDELAY")
                    .props(setup_idelayctrl.clone())
                    .test_enum("FINEDELAY", &["BYPASS", "ADD_DLY"]);
            }
            for i in 0..num_io {
                let bel = defs::bslots::IOB[i];
                let mut bctx = ctx.bel(bel);
                let bel_ologic = defs::bslots::OLOGIC[i];
                let bel_odelay = defs::bslots::ODELAY[i];
                let bel_other = if num_io == 1 {
                    None
                } else {
                    Some(defs::bslots::IOB[i ^ 1])
                };
                bctx.build()
                    .global("DCIUPDATEMODE", "ASREQUIRED")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .test_manual("PRESENT", "IOB")
                    .mode("IOB18")
                    .commit();
                bctx.build()
                    .global("DCIUPDATEMODE", "QUIET")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .test_manual("PRESENT", "IOB.QUIET")
                    .mode("IOB18")
                    .commit();
                if num_io == 2 {
                    bctx.build()
                        .global("DCIUPDATEMODE", "ASREQUIRED")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .test_manual("PRESENT", "IPAD")
                        .mode("IPAD")
                        .commit();
                }
                bctx.mode("IOB18")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .test_enum("PULL", &["KEEPER", "PULLDOWN", "PULLUP"]);
                for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
                    bctx.mode("IOB18")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .mutex("PULL_DYNAMIC", pin)
                        .test_manual("PULL_DYNAMIC", "1")
                        .pin_pips(pin)
                        .commit();
                }
                bctx.mode("IOB18")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "1800")
                    .pin("I")
                    .pin("O")
                    .attr("OPROGRAMMING", "0000000000000000000000000000000000")
                    .attr("IUSED", "0")
                    .attr("OUSED", "0")
                    .attr("ISTANDARD", "LVCMOS18")
                    .attr("OSTANDARD", "LVCMOS18")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_multi_attr_bin("IPROGRAMMING", 24);
                bctx.mode("IOB18")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "1800")
                    .pin("O")
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", "LVCMOS18")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_multi_attr_bin("OPROGRAMMING", 34);
                for &std in HP_IOSTDS {
                    if num_io == 1 && !matches!(std.name, "LVCMOS18" | "HSTL_I") {
                        continue;
                    }
                    let mut vref_special = None;
                    let mut dci_special = None;
                    let mut dci_special_lite = None;
                    if std.vref.is_some() {
                        vref_special = Some(Vref(true));
                    }
                    if std.dci == DciKind::BiSplitT {
                        continue;
                    } else if matches!(
                        std.dci,
                        DciKind::BiSplit | DciKind::BiVcc | DciKind::InputSplit | DciKind::InputVcc
                    ) {
                        dci_special = Some(Dci(Some(std.name)));
                        dci_special_lite = Some(Dci(None));
                    }
                    if std.diff != DiffKind::None {
                        if let Some(bel_other) = bel_other {
                            for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                                bctx.mode("IOB18")
                                    .global("DCIUPDATEMODE", "ASREQUIRED")
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .related_tile_mutex(
                                        HclkIoi,
                                        "VCCO",
                                        std.vcco.unwrap().to_string(),
                                    )
                                    .attr("OUSED", "")
                                    .pin("I")
                                    .pin("DIFFI_IN")
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .maybe_prop(dci_special)
                                    .bel_mode(bel_other, "IOB")
                                    .bel_pin(bel_other, "PADOUT")
                                    .bel_attr(bel_other, "OUSED", "")
                                    .test_manual("ISTD", format!("{sn}.{suffix}", sn = std.name))
                                    .attr("IUSED", "0")
                                    .attr("DIFFI_INUSED", "0")
                                    .attr("ISTANDARD", std.name)
                                    .attr(
                                        "DIFF_TERM",
                                        if std.diff == DiffKind::True {
                                            "FALSE"
                                        } else {
                                            ""
                                        },
                                    )
                                    .attr("IBUF_LOW_PWR", lp)
                                    .bel_attr(bel_other, "PADOUTUSED", "0")
                                    .bel_attr(bel_other, "ISTANDARD", std.name)
                                    .bel_attr(
                                        bel_other,
                                        "DIFF_TERM",
                                        if std.diff == DiffKind::True {
                                            "FALSE"
                                        } else {
                                            ""
                                        },
                                    )
                                    .bel_attr(bel_other, "IBUF_LOW_PWR", lp)
                                    .commit();
                            }
                            if std.diff == DiffKind::True && bel == defs::bslots::IOB[0] {
                                bctx.mode("IOB18")
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .related_tile_mutex(
                                        HclkIoi,
                                        "VCCO",
                                        std.vcco.unwrap().to_string(),
                                    )
                                    .attr("OUSED", "")
                                    .pin("I")
                                    .pin("DIFFI_IN")
                                    .attr("IUSED", "0")
                                    .attr("DIFFI_INUSED", "0")
                                    .attr("ISTANDARD", std.name)
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .maybe_prop(dci_special_lite)
                                    .bel_mode(bel_other, "IOB")
                                    .bel_pin(bel_other, "PADOUT")
                                    .bel_attr(bel_other, "OUSED", "")
                                    .bel_attr(bel_other, "PADOUTUSED", "0")
                                    .bel_attr(bel_other, "ISTANDARD", std.name)
                                    .test_manual("DIFF_TERM", std.name)
                                    .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                                    .bel_attr_diff(bel_other, "DIFF_TERM", "FALSE", "TRUE")
                                    .commit();
                                bctx.mode("IOB18")
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .related_tile_mutex(
                                        HclkIoi,
                                        "VCCO",
                                        std.vcco.unwrap().to_string(),
                                    )
                                    .attr("OUSED", "")
                                    .pin("I")
                                    .pin("DIFFI_IN")
                                    .attr("IUSED", "0")
                                    .attr("DIFFI_INUSED", "0")
                                    .attr("ISTANDARD", std.name)
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .maybe_prop(dci_special_lite)
                                    .bel_mode(bel_other, "IOB")
                                    .bel_pin(bel_other, "PADOUT")
                                    .bel_attr(bel_other, "OUSED", "")
                                    .bel_attr(bel_other, "PADOUTUSED", "0")
                                    .bel_attr(bel_other, "ISTANDARD", std.name)
                                    .test_manual("DIFF_TERM_DYNAMIC", std.name)
                                    .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                                    .bel_attr_diff(bel_other, "DIFF_TERM", "FALSE", "TRUE")
                                    .pin_pips("DIFF_TERM_INT_EN")
                                    .commit();
                            }
                        }
                    } else {
                        for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                            bctx.mode("IOB18")
                                .global("DCIUPDATEMODE", "ASREQUIRED")
                                .global("UNCONSTRAINEDPINS", "ALLOW")
                                .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                                .attr("OUSED", "")
                                .pin("I")
                                .raw(Key::Package, &package.name)
                                .prop(IsBonded(bel))
                                .maybe_prop(vref_special)
                                .maybe_prop(dci_special)
                                .test_manual("ISTD", format!("{sn}.{suffix}", sn = std.name))
                                .attr("IUSED", "0")
                                .attr("ISTANDARD", std.name)
                                .attr("IBUF_LOW_PWR", lp)
                                .commit();
                        }
                    }
                }
                bctx.mode("IOB18")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "1800")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .pin("I")
                    .pin("O")
                    .pin("IBUFDISABLE")
                    .attr("IUSED", "0")
                    .attr("OUSED", "0")
                    .attr("ISTANDARD", "LVCMOS18")
                    .attr("OSTANDARD", "LVCMOS18")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_enum("IBUFDISABLE_SEL", &["GND", "I"]);
                bctx.mode("IOB18")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "1800")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .pin("I")
                    .pin("O")
                    .pin("DCITERMDISABLE")
                    .attr("IUSED", "0")
                    .attr("OUSED", "0")
                    .attr("ISTANDARD", "LVCMOS18")
                    .attr("OSTANDARD", "LVCMOS18")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_enum("DCITERMDISABLE_SEL", &["GND", "I"]);
                for &std in HP_IOSTDS {
                    if num_io == 1 && std.name != "HSTL_I" {
                        continue;
                    }
                    let mut dci_special = None;
                    if matches!(
                        std.dci,
                        DciKind::Output
                            | DciKind::OutputHalf
                            | DciKind::BiSplit
                            | DciKind::BiVcc
                            | DciKind::BiSplitT
                    ) {
                        dci_special = Some(Dci(Some(std.name)));
                    }
                    if std.diff == DiffKind::True {
                        if bel == defs::bslots::IOB[1] {
                            let bel_other = bel_other.unwrap();
                            bctx.build()
                                .global("UNCONSTRAINEDPINS", "ALLOW")
                                .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                                .attr("IUSED", "")
                                .attr("OPROGRAMMING", "")
                                .raw(Key::Package, &package.name)
                                .prop(IsBonded(bel))
                                .prop(DiffOut("STD", std.name))
                                .bel_attr(bel_other, "IUSED", "")
                                .bel_attr(bel_other, "OPROGRAMMING", "")
                                .bel_attr(bel_other, "OSTANDARD", "")
                                .bel_attr(bel_other, "OUSED", "")
                                .test_manual("OSTD", std.name)
                                .mode_diff("IOB18", "IOB18M")
                                .pin("O")
                                .attr("OUSED", "0")
                                .attr("DIFFO_OUTUSED", "0")
                                .attr("OSTANDARD", std.name)
                                .bel_mode_diff(bel_other, "IOB18", "IOB18S")
                                .bel_attr(bel_other, "OUTMUX", "1")
                                .bel_attr(bel_other, "DIFFO_INUSED", "0")
                                .pin_pair("DIFFO_OUT", bel_other, "DIFFO_IN")
                                .commit();
                        }
                    } else if std.diff != DiffKind::None {
                        if bel == defs::bslots::IOB[1] {
                            let bel_other = bel_other.unwrap();
                            for slew in ["SLOW", "FAST"] {
                                if std.dci == DciKind::BiSplitT {
                                    bctx.build()
                                        .global("DCIUPDATEMODE", "ASREQUIRED")
                                        .global("UNCONSTRAINEDPINS", "ALLOW")
                                        .related_tile_mutex(
                                            HclkIoi,
                                            "VCCO",
                                            std.vcco.unwrap().to_string(),
                                        )
                                        .attr("OPROGRAMMING", "")
                                        .raw(Key::Package, &package.name)
                                        .prop(IsBonded(bel))
                                        .maybe_prop(dci_special)
                                        .bel_attr(bel_other, "OPROGRAMMING", "")
                                        .bel_mode(defs::bslots::OLOGIC[0], "OLOGICE2")
                                        .test_manual(
                                            "OSTD",
                                            format!("{name}.{slew}", name = std.name),
                                        )
                                        .mode_diff("IOB18", "IOB18M")
                                        .pin("O")
                                        .pin("I")
                                        .attr("OUSED", "0")
                                        .attr("IUSED", "0")
                                        .attr("O_OUTUSED", "0")
                                        .attr("OSTANDARD", std.name)
                                        .attr("ISTANDARD", std.name)
                                        .attr("SLEW", slew)
                                        .bel_mode_diff(bel_other, "IOB18", "IOB18S")
                                        .bel_pin(bel_other, "I")
                                        .bel_attr(bel_other, "IUSED", "0")
                                        .bel_attr(bel_other, "OUTMUX", "0")
                                        .bel_attr(bel_other, "OINMUX", "1")
                                        .bel_attr(bel_other, "OSTANDARD", std.name)
                                        .bel_attr(bel_other, "ISTANDARD", std.name)
                                        .bel_attr(bel_other, "SLEW", slew)
                                        .pin_pair("O_OUT", bel_other, "O_IN")
                                        .commit();
                                } else {
                                    bctx.build()
                                        .global("DCIUPDATEMODE", "ASREQUIRED")
                                        .global("UNCONSTRAINEDPINS", "ALLOW")
                                        .related_tile_mutex(
                                            HclkIoi,
                                            "VCCO",
                                            std.vcco.unwrap().to_string(),
                                        )
                                        .attr("IUSED", "")
                                        .attr("OPROGRAMMING", "")
                                        .raw(Key::Package, &package.name)
                                        .prop(IsBonded(bel))
                                        .maybe_prop(dci_special)
                                        .bel_attr(bel_other, "IUSED", "")
                                        .bel_attr(bel_other, "OPROGRAMMING", "")
                                        .bel_mode(defs::bslots::OLOGIC[0], "OLOGICE2")
                                        .test_manual(
                                            "OSTD",
                                            format!("{name}.{slew}", name = std.name),
                                        )
                                        .mode_diff("IOB18", "IOB18M")
                                        .pin("O")
                                        .attr("OUSED", "0")
                                        .attr("O_OUTUSED", "0")
                                        .attr("OSTANDARD", std.name)
                                        .attr("SLEW", slew)
                                        .bel_mode_diff(bel_other, "IOB18", "IOB18S")
                                        .bel_attr(bel_other, "OUTMUX", "0")
                                        .bel_attr(bel_other, "OINMUX", "1")
                                        .bel_attr(bel_other, "OSTANDARD", std.name)
                                        .bel_attr(bel_other, "SLEW", slew)
                                        .pin_pair("O_OUT", bel_other, "O_IN")
                                        .commit();
                                }
                            }
                        }
                    } else if std.dci == DciKind::BiSplitT {
                        for slew in ["SLOW", "FAST"] {
                            bctx.mode("IOB18")
                                .global("DCIUPDATEMODE", "ASREQUIRED")
                                .global("UNCONSTRAINEDPINS", "ALLOW")
                                .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                                .pin("O")
                                .pin("I")
                                .attr("OPROGRAMMING", "")
                                .raw(Key::Package, &package.name)
                                .prop(IsBonded(bel))
                                .prop(Vref(true))
                                .maybe_prop(dci_special)
                                .test_manual("OSTD", format!("{name}.{slew}", name = std.name))
                                .attr("OUSED", "0")
                                .attr("IUSED", "0")
                                .attr("OSTANDARD", std.name)
                                .attr("ISTANDARD", std.name)
                                .attr("SLEW", slew)
                                .commit();
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
                            for &slew in slews {
                                let val = if slew.is_empty() {
                                    std.name.to_string()
                                } else if drive.is_empty() {
                                    format!("{name}.{slew}", name = std.name)
                                } else {
                                    format!("{name}.{drive}.{slew}", name = std.name)
                                };
                                bctx.mode("IOB18")
                                    .global("DCIUPDATEMODE", "ASREQUIRED")
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .related_tile_mutex(
                                        HclkIoi,
                                        "VCCO",
                                        std.vcco.unwrap().to_string(),
                                    )
                                    .pin("O")
                                    .attr("IUSED", "")
                                    .attr("OPROGRAMMING", "")
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .maybe_prop(dci_special)
                                    .test_manual("OSTD", val)
                                    .attr("OUSED", "0")
                                    .attr("OSTANDARD", std.name)
                                    .attr("DRIVE", drive)
                                    .attr("SLEW", slew)
                                    .commit();
                            }
                        }
                    }
                }

                if num_io == 2 {
                    for (std, vcco, vref) in [
                        ("HSTL_I_12", 1200, 600),
                        ("SSTL135", 1350, 675),
                        ("HSTL_I", 1500, 750),
                        ("HSTL_I_18", 1800, 900),
                        // ("HSTL_III_18", 1800, 1100),
                        // ("SSTL2_I", 2500, 1250),
                    ] {
                        bctx.build()
                            .global("UNCONSTRAINEDPINS", "ALLOW")
                            .related_tile_mutex(HclkIoi, "VCCO", vcco.to_string())
                            .mode("IOB18")
                            .attr("OUSED", "")
                            .pin("I")
                            .raw(Key::Package, &package.name)
                            .prop(IsBonded(bel))
                            .prop(VrefInternal("HCLK_IO_HP", vref))
                            .test_manual("ISTD", format!("{std}.LP"))
                            .attr("IUSED", "0")
                            .attr("ISTANDARD", std)
                            .attr("IBUF_LOW_PWR", "TRUE")
                            .commit();
                    }
                }

                bctx.build()
                    .mutex("OUTPUT_DELAY", "0")
                    .bel_mode(bel_odelay, "ODELAYE2")
                    .bel_mode(bel_ologic, "OLOGICE2")
                    .test_manual("OUTPUT_DELAY", "0")
                    .pip((bel_ologic, "IOB_O"), (bel_ologic, "OQ"))
                    .commit();
                bctx.build()
                    .mutex("OUTPUT_DELAY", "1")
                    .bel_mode(bel_odelay, "ODELAYE2")
                    .bel_mode(bel_ologic, "OLOGICE2")
                    .test_manual("OUTPUT_DELAY", "1")
                    .pip((bel_ologic, "IOB_O"), (bel_odelay, "DATAOUT"))
                    .commit();
            }
        } else {
            for i in 0..num_io {
                let bel = defs::bslots::IOB[i];
                let mut bctx = ctx.bel(bel);
                let bel_other = if num_io == 1 {
                    None
                } else {
                    Some(defs::bslots::IOB[i ^ 1])
                };

                bctx.build()
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .test_manual("PRESENT", "IOB")
                    .mode("IOB33")
                    .commit();
                if num_io == 2 {
                    bctx.build()
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .test_manual("PRESENT", "IPAD")
                        .mode("IPAD")
                        .commit();
                }
                bctx.mode("IOB33")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .test_enum("PULL", &["KEEPER", "PULLDOWN", "PULLUP"]);
                for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
                    bctx.mode("IOB33")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .mutex("PULL_DYNAMIC", pin)
                        .test_manual("PULL_DYNAMIC", "1")
                        .pin_pips(pin)
                        .commit();
                }
                bctx.mode("IOB33")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "3300")
                    .pin("O")
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", "LVCMOS33")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_multi_attr_bin("OPROGRAMMING", 39);
                bctx.mode("IOB33")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "3300")
                    .pin("I")
                    .pin("O")
                    .attr("OPROGRAMMING", "000000000000000000000000000000000000000")
                    .attr("IUSED", "0")
                    .attr("OUSED", "0")
                    .attr("ISTANDARD", "LVCMOS33")
                    .attr("OSTANDARD", "LVCMOS33")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_multi_attr_bin("IPROGRAMMING", 9);
                bctx.mode("IOB33")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "1800")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .pin("I")
                    .pin("O")
                    .pin("IBUFDISABLE")
                    .attr("IUSED", "0")
                    .attr("OUSED", "0")
                    .attr("ISTANDARD", "LVCMOS18")
                    .attr("OSTANDARD", "LVCMOS18")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_enum("IBUFDISABLE_SEL", &["GND", "I"]);
                bctx.mode("IOB33")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "1800")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .pin("I")
                    .pin("O")
                    .pin("INTERMDISABLE")
                    .attr("IUSED", "0")
                    .attr("OUSED", "0")
                    .attr("ISTANDARD", "LVCMOS18")
                    .attr("OSTANDARD", "LVCMOS18")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_enum("INTERMDISABLE_SEL", &["GND", "I"]);
                bctx.mode("IOB33")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "1800")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .pin("I")
                    .pin("O")
                    .attr("IUSED", "0")
                    .attr("OUSED", "0")
                    .attr("ISTANDARD", "LVCMOS18")
                    .attr("OSTANDARD", "LVCMOS18")
                    .attr("DRIVE", "12")
                    .attr("SLEW", "SLOW")
                    .test_enum("DQS_BIAS", &["FALSE", "TRUE"]);
                bctx.mode("IOB33")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "1800")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .prop(Vref(false))
                    .pin("I")
                    .pin("O")
                    .attr("IUSED", "0")
                    .attr("OUSED", "0")
                    .attr("ISTANDARD", "SSTL18_II")
                    .attr("OSTANDARD", "SSTL18_II")
                    .attr("SLEW", "SLOW")
                    .test_enum(
                        "IN_TERM",
                        &[
                            "NONE",
                            "UNTUNED_SPLIT_40",
                            "UNTUNED_SPLIT_50",
                            "UNTUNED_SPLIT_60",
                        ],
                    );

                let anchor_props = |dy, vcco: u16, anchor_std: &'static str| -> [Box<DynProp>; 5] {
                    let rel = Delta::new(0, dy, "IO_HR_PAIR");
                    [
                        Box::new(Related::new(
                            HclkIoi,
                            TileMutex::new("VCCO".into(), vcco.to_string()),
                        )),
                        Box::new(Related::new(
                            rel.clone(),
                            BaseBelMode::new(defs::bslots::IOB[1], "IOB33".into()),
                        )),
                        Box::new(Related::new(
                            rel.clone(),
                            BaseBelPin::new(defs::bslots::IOB[1], "O".into()),
                        )),
                        Box::new(Related::new(
                            rel.clone(),
                            BaseBelAttr::new(defs::bslots::IOB[1], "OUSED".into(), "0".into()),
                        )),
                        Box::new(Related::new(
                            rel.clone(),
                            BaseBelAttr::new(
                                defs::bslots::IOB[1],
                                "OSTANDARD".into(),
                                anchor_std.into(),
                            ),
                        )),
                    ]
                };
                let anchor_dy = match tile {
                    "IO_HR_S" => 1,
                    "IO_HR_PAIR" => 2,
                    "IO_HR_N" => -2,
                    _ => unreachable!(),
                };
                for &std in HR_IOSTDS {
                    if num_io == 1
                        && !matches!(std.name, "PCI33_3" | "LVCMOS18" | "LVCMOS33" | "HSTL_I")
                    {
                        continue;
                    }
                    let mut vref_special = None;
                    if std.vref.is_some() {
                        vref_special = Some(Vref(true));
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
                                bctx.mode("IOB33")
                                    .global("DCIUPDATEMODE", "ASREQUIRED")
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                    .attr("OUSED", "")
                                    .pin("I")
                                    .pin("DIFFI_IN")
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .bel_mode(bel_other, "IOB")
                                    .bel_pin(bel_other, "PADOUT")
                                    .bel_attr(bel_other, "OUSED", "")
                                    .test_manual("ISTD", format!("{sn}.{suffix}", sn = std.name))
                                    .attr("IUSED", "0")
                                    .attr("DIFFI_INUSED", "0")
                                    .attr("ISTANDARD", std.name)
                                    .attr(
                                        "DIFF_TERM",
                                        if std.diff == DiffKind::True {
                                            "FALSE"
                                        } else {
                                            ""
                                        },
                                    )
                                    .attr("IBUF_LOW_PWR", lp)
                                    .bel_attr(bel_other, "PADOUTUSED", "0")
                                    .bel_attr(bel_other, "ISTANDARD", std.name)
                                    .bel_attr(
                                        bel_other,
                                        "DIFF_TERM",
                                        if std.diff == DiffKind::True {
                                            "FALSE"
                                        } else {
                                            ""
                                        },
                                    )
                                    .bel_attr(bel_other, "IBUF_LOW_PWR", lp)
                                    .commit();
                            }
                            if std.diff == DiffKind::True
                                && bel == defs::bslots::IOB[0]
                                && std.name != "TMDS_33"
                            {
                                bctx.mode("IOB33")
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                    .attr("OUSED", "")
                                    .pin("I")
                                    .pin("DIFFI_IN")
                                    .attr("IUSED", "0")
                                    .attr("DIFFI_INUSED", "0")
                                    .attr("ISTANDARD", std.name)
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .bel_mode(bel_other, "IOB")
                                    .bel_pin(bel_other, "PADOUT")
                                    .bel_attr(bel_other, "OUSED", "")
                                    .bel_attr(bel_other, "PADOUTUSED", "0")
                                    .bel_attr(bel_other, "ISTANDARD", std.name)
                                    .test_manual("DIFF_TERM", std.name)
                                    .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                                    .bel_attr_diff(bel_other, "DIFF_TERM", "FALSE", "TRUE")
                                    .commit();
                                bctx.mode("IOB33")
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                    .attr("OUSED", "")
                                    .pin("I")
                                    .pin("DIFFI_IN")
                                    .attr("IUSED", "0")
                                    .attr("DIFFI_INUSED", "0")
                                    .attr("ISTANDARD", std.name)
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .bel_mode(bel_other, "IOB")
                                    .bel_pin(bel_other, "PADOUT")
                                    .bel_attr(bel_other, "OUSED", "")
                                    .bel_attr(bel_other, "PADOUTUSED", "0")
                                    .bel_attr(bel_other, "ISTANDARD", std.name)
                                    .test_manual("DIFF_TERM_DYNAMIC", std.name)
                                    .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                                    .bel_attr_diff(bel_other, "DIFF_TERM", "FALSE", "TRUE")
                                    .pin_pips("DIFF_TERM_INT_EN")
                                    .commit();
                            }
                        }
                    } else {
                        for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                            bctx.mode("IOB33")
                                .global("UNCONSTRAINEDPINS", "ALLOW")
                                .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                .attr("OUSED", "")
                                .pin("I")
                                .raw(Key::Package, &package.name)
                                .prop(IsBonded(bel))
                                .maybe_prop(vref_special)
                                .test_manual("ISTD", format!("{sn}.{suffix}", sn = std.name))
                                .attr("IUSED", "0")
                                .attr("ISTANDARD", std.name)
                                .attr("IBUF_LOW_PWR", lp)
                                .commit();
                        }
                    }
                }

                for &std in HR_IOSTDS {
                    if num_io == 1 {
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
                        if bel == defs::bslots::IOB[1] {
                            let bel_other = bel_other.unwrap();
                            bctx.build()
                                .global("UNCONSTRAINEDPINS", "ALLOW")
                                .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                .attr("IUSED", "")
                                .attr("OPROGRAMMING", "")
                                .raw(Key::Package, &package.name)
                                .prop(IsBonded(bel))
                                .prop(DiffOut("STD0", std.name))
                                .bel_attr(bel_other, "IUSED", "")
                                .bel_attr(bel_other, "OPROGRAMMING", "")
                                .bel_attr(bel_other, "OSTANDARD", "")
                                .bel_attr(bel_other, "OUSED", "")
                                .test_manual("OSTD", std.name)
                                .mode_diff("IOB33", "IOB33M")
                                .pin("O")
                                .attr("OUSED", "0")
                                .attr("DIFFO_OUTUSED", "0")
                                .attr("OSTANDARD", std.name)
                                .bel_mode_diff(bel_other, "IOB33", "IOB33S")
                                .bel_attr(bel_other, "OUTMUX", "1")
                                .bel_attr(bel_other, "DIFFO_INUSED", "0")
                                .pin_pair("DIFFO_OUT", bel_other, "DIFFO_IN")
                                .commit();
                            let alt_std = if std.name == "LVDS_25" {
                                "RSDS_25"
                            } else {
                                "LVDS_25"
                            };
                            if std.name != "TMDS_33" {
                                bctx.build()
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelMode::new(defs::bslots::IOB[1], "IOB33M".into()),
                                    ))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelPin::new(defs::bslots::IOB[1], "O".into()),
                                    ))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelAttr::new(
                                            defs::bslots::IOB[1],
                                            "OUSED".into(),
                                            "0".into(),
                                        ),
                                    ))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelAttr::new(
                                            defs::bslots::IOB[1],
                                            "DIFFO_OUTUSED".into(),
                                            "0".into(),
                                        ),
                                    ))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelAttr::new(
                                            defs::bslots::IOB[1],
                                            "OSTANDARD".into(),
                                            alt_std.into(),
                                        ),
                                    ))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelMode::new(defs::bslots::IOB[0], "IOB33S".into()),
                                    ))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelPinPair::new(
                                            defs::bslots::IOB[1],
                                            "DIFFO_OUT".into(),
                                            defs::bslots::IOB[0],
                                            "DIFFO_IN".into(),
                                        ),
                                    ))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelAttr::new(
                                            defs::bslots::IOB[0],
                                            "OUTMUX".into(),
                                            "1".into(),
                                        ),
                                    ))
                                    .prop(Related::new(
                                        Delta::new(0, 4, "IO_HR_PAIR"),
                                        BaseBelAttr::new(
                                            defs::bslots::IOB[0],
                                            "DIFFO_INUSED".into(),
                                            "0".into(),
                                        ),
                                    ))
                                    .attr("IUSED", "")
                                    .attr("OPROGRAMMING", "")
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .prop(DiffOut("STD1", std.name))
                                    .bel_attr(bel_other, "IUSED", "")
                                    .bel_attr(bel_other, "OPROGRAMMING", "")
                                    .bel_attr(bel_other, "OSTANDARD", "")
                                    .bel_attr(bel_other, "OUSED", "")
                                    .test_manual("OSTD", format!("{}.ALT", std.name))
                                    .mode_diff("IOB33", "IOB33M")
                                    .pin("O")
                                    .attr("OUSED", "0")
                                    .attr("DIFFO_OUTUSED", "0")
                                    .attr("OSTANDARD", std.name)
                                    .bel_mode_diff(bel_other, "IOB33", "IOB33S")
                                    .bel_attr(bel_other, "OUTMUX", "1")
                                    .bel_attr(bel_other, "DIFFO_INUSED", "0")
                                    .pin_pair("DIFFO_OUT", bel_other, "DIFFO_IN")
                                    .commit();
                            }
                        }
                    } else if std.diff != DiffKind::None {
                        if bel == defs::bslots::IOB[1] {
                            let bel_other = bel_other.unwrap();
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
                                bctx.build()
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                    .attr("IUSED", "")
                                    .attr("OPROGRAMMING", "")
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .bel_attr(bel_other, "IUSED", "")
                                    .bel_attr(bel_other, "OPROGRAMMING", "")
                                    .bel_mode(defs::bslots::OLOGIC[0], "OLOGICE2")
                                    .test_manual("OSTD", val)
                                    .mode_diff("IOB33", "IOB33M")
                                    .pin("O")
                                    .attr("OUSED", "0")
                                    .attr("O_OUTUSED", "0")
                                    .attr("OSTANDARD", std.name)
                                    .attr("SLEW", slew)
                                    .bel_mode_diff(bel_other, "IOB33", "IOB33S")
                                    .bel_attr(bel_other, "OUTMUX", "0")
                                    .bel_attr(bel_other, "OINMUX", "1")
                                    .bel_attr(bel_other, "OSTANDARD", std.name)
                                    .bel_attr(bel_other, "SLEW", slew)
                                    .pin_pair("O_OUT", bel_other, "O_IN")
                                    .commit();
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
                            for &slew in slews {
                                let val = if slew.is_empty() {
                                    std.name.to_string()
                                } else if drive.is_empty() {
                                    format!("{name}.{slew}", name = std.name)
                                } else {
                                    format!("{name}.{drive}.{slew}", name = std.name)
                                };
                                bctx.mode("IOB33")
                                    .global("UNCONSTRAINEDPINS", "ALLOW")
                                    .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                    .pin("O")
                                    .attr("IUSED", "")
                                    .attr("OPROGRAMMING", "")
                                    .raw(Key::Package, &package.name)
                                    .prop(IsBonded(bel))
                                    .test_manual("OSTD", val)
                                    .attr("OUSED", "0")
                                    .attr("OSTANDARD", std.name)
                                    .attr("DRIVE", drive)
                                    .attr("SLEW", slew)
                                    .commit();
                            }
                        }
                    }
                }

                if num_io == 2 {
                    for (std, vcco, vref) in [
                        ("HSUL_12", 1200, 600),
                        ("SSTL135", 1350, 675),
                        ("HSTL_I", 1500, 750),
                        ("HSTL_I_18", 1800, 900),
                    ] {
                        bctx.mode("IOB33")
                            .global("UNCONSTRAINEDPINS", "ALLOW")
                            .props(anchor_props(anchor_dy, vcco, std))
                            .attr("OUSED", "")
                            .pin("I")
                            .raw(Key::Package, &package.name)
                            .prop(IsBonded(bel))
                            .prop(VrefInternal("HCLK_IO_HR", vref))
                            .test_manual("ISTD", format!("{std}.LP"))
                            .attr("IUSED", "0")
                            .attr("ISTANDARD", std)
                            .attr("IBUF_LOW_PWR", "TRUE")
                            .commit();
                    }
                }

                if tile == "IO_HR_S" {
                    let mut builder = bctx
                        .mode("IOB33")
                        .global("UNCONSTRAINEDPINS", "ALLOW")
                        .related_tile_mutex(HclkIoi, "VCCO", "TEST")
                        .pin("O")
                        .attr("IUSED", "")
                        .attr("OPROGRAMMING", "")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .extra_tile_attr(Delta::new(0, 49, "IO_HR_N"), "IOB[0]", "LOW_VOLTAGE", "1")
                        .extra_tile_attr(
                            Delta::new(0, 25, "HCLK_IO_HR"),
                            "DRIVERBIAS",
                            "DRIVERBIAS",
                            "LV",
                        );
                    for i in 0..24 {
                        builder = builder.extra_tile_attr(
                            Delta::new(0, 1 + i * 2, "IO_HR_PAIR"),
                            "IOB_COMMON",
                            "LOW_VOLTAGE",
                            "1",
                        );
                    }
                    builder
                        .test_manual("OSTD", "LVCMOS18.4.SLOW.EXCL")
                        .attr("OUSED", "0")
                        .attr("OSTANDARD", "LVCMOS18")
                        .attr("DRIVE", "4")
                        .attr("SLEW", "SLOW")
                        .commit();
                }
            }
        }
    }
    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        ctx.build()
            .global("ENABLEMISR", "Y")
            .extra_tiles_by_kind("IO_HP_PAIR", "OLOGIC_COMMON")
            .extra_tiles_by_kind("IO_HR_PAIR", "OLOGIC_COMMON")
            .extra_tiles_by_kind("IO_HP_S", "OLOGIC[0]")
            .extra_tiles_by_kind("IO_HP_N", "OLOGIC[0]")
            .extra_tiles_by_kind("IO_HR_S", "OLOGIC[0]")
            .extra_tiles_by_kind("IO_HR_N", "OLOGIC[0]")
            .test_manual("NULL", "MISR_RESET", "1")
            .global_diff("MISRRESET", "N", "Y")
            .commit();
    }

    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "HCLK_IO_HP") {
        let mut bctx = ctx.bel(defs::bslots::DCI);
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_manual("TEST_ENABLE", "1")
            .mode("DCI")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "QUIET")
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_manual("TEST_ENABLE", "QUIET")
            .mode("DCI")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_manual("DYNAMIC_ENABLE", "1")
            .mode("DCI")
            .pin_pips("INT_DCI_EN")
            .commit();
        {
            let mut ctx = FuzzCtx::new_null(session, backend);
            let die = DieId::from_idx(0);
            let chip = edev.chips[die];
            let mut builder = ctx
                .build()
                .raw(Key::Package, &package.name)
                .global("DCIUPDATEMODE", "ASREQUIRED")
                .global("UNCONSTRAINEDPINS", "ALLOW")
                .extra_tile_attr_fixed(edev.tile_cfg(die), "MISC", "DCI_CLK_ENABLE", "1");

            let anchor_reg = if chip.has_ps {
                RegId::from_idx(chip.regs - 2)
            } else {
                RegId::from_idx(0)
            };
            let io_row = chip.row_reg_hclk(anchor_reg) - 24;
            let io_tile =
                CellCoord::new(die, edev.col_rio.unwrap(), io_row).tile(defs::tslots::BEL);
            let io_bel = io_tile.cell.bel(defs::bslots::IOB[0]);
            let hclk_row = chip.row_hclk(io_tile.cell.row);
            let hclk_tile =
                CellCoord::new(die, edev.col_rio.unwrap(), hclk_row).tile(defs::tslots::HCLK_BEL);

            // Ensure nothing is placed in VR.
            for row in [
                chip.row_reg_hclk(anchor_reg) - 25,
                chip.row_reg_hclk(anchor_reg) + 24,
            ] {
                let vr_tile =
                    CellCoord::new(die, edev.col_rio.unwrap(), row).tile(defs::tslots::BEL);
                let vr_bel = vr_tile.cell.bel(defs::bslots::IOB[0]);
                let site = backend.ngrid.get_bel_name(vr_bel).unwrap();
                builder = builder
                    .raw(Key::SiteMode(site), None)
                    .extra_tile_attr_fixed(vr_tile, "IOB[0]", "PRESENT", "VR");
            }

            // Set up hclk.
            builder = builder.extra_tile_attr_fixed(hclk_tile, "DCI", "ENABLE", "1");

            // Set up the IO and fire.
            let site = backend.ngrid.get_bel_name(io_bel).unwrap();
            builder
                .raw(Key::SiteMode(site), "IOB")
                .raw(Key::SitePin(site, "O".into()), true)
                .raw(Key::SiteAttr(site, "IUSED".into()), None)
                .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
                .raw_diff(Key::SiteAttr(site, "OUSED".into()), None, "0")
                .raw_diff(Key::SiteAttr(site, "OSTANDARD".into()), None, "HSLVDCI_18")
                // Make note of anchor VCCO.
                .raw(Key::TileMutex(hclk_tile, "VCCO".to_string()), "1800")
                // Take exclusive mutex on global DCI.
                .raw_diff(Key::GlobalMutex("GLOBAL_DCI".into()), None, "EXCLUSIVE")
                // Avoid interference.
                .global("MATCH_CYCLE", "NOWAIT")
                .extra_tile_attr_fixed(io_tile, "IOB[0]", "OSTD", "HSLVDCI_18")
                .test_manual("NULL", "CENTER_DCI", "1")
                .commit();
        }
        for (bank_from, bank_to) in [(0, 1), (1, 0)] {
            let mut ctx = FuzzCtx::new_null(session, backend);
            let die = DieId::from_idx(0);
            let chip = edev.chips[die];
            let mut builder = ctx
                .build()
                .raw(Key::Package, &package.name)
                .global("DCIUPDATEMODE", "ASREQUIRED")
                .global("UNCONSTRAINEDPINS", "ALLOW");

            let (anchor_reg_from, anchor_reg_to) = if chip.has_ps {
                (
                    RegId::from_idx(chip.regs - 2 + bank_from),
                    RegId::from_idx(chip.regs - 2 + bank_to),
                )
            } else {
                (RegId::from_idx(bank_from), RegId::from_idx(bank_to))
            };
            let col = edev.col_rio.unwrap();
            let hclk_row_from = chip.row_reg_hclk(anchor_reg_from);
            let hclk_row_to = chip.row_reg_hclk(anchor_reg_to);
            let hclk_tile_to = CellCoord::new(die, col, hclk_row_to).tile(defs::tslots::HCLK_BEL);
            let io_row_from = hclk_row_from - 24;
            let io_bel_from = CellCoord::new(die, col, io_row_from).bel(defs::bslots::IOB[0]);
            let io_row_to = hclk_row_to - 24;
            let io_tile_to = CellCoord::new(die, col, io_row_to).tile(defs::tslots::BEL);
            let io_bel_to = io_tile_to.cell.bel(defs::bslots::IOB[0]);
            let actual_bank_from = edev
                .get_io_info(IoCoord {
                    cell: CellCoord {
                        die,
                        col,
                        row: hclk_row_from - 24,
                    },
                    iob: EntityId::from_idx(0),
                })
                .bank;
            let actual_bank_to = edev
                .get_io_info(IoCoord {
                    cell: CellCoord {
                        die,
                        col,
                        row: hclk_row_to - 24,
                    },
                    iob: EntityId::from_idx(0),
                })
                .bank;

            // Ensure nothing else in the bank.
            for i in 0..50 {
                let row = hclk_row_from - 25 + i;
                for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
                    if row == io_row_from && bel == defs::bslots::IOB[0] {
                        continue;
                    }
                    if let Some(site) = backend
                        .ngrid
                        .get_bel_name(CellCoord::new(die, col, row).bel(bel))
                    {
                        builder = builder.raw(Key::SiteMode(site), None);
                    }
                }
            }
            let site = backend.ngrid.get_bel_name(io_bel_from).unwrap();
            builder = builder
                .raw(Key::SiteMode(site), "IOB")
                .raw(Key::SitePin(site, "O".into()), true)
                .raw(Key::SiteAttr(site, "IMUX".into()), None)
                .raw(Key::SiteAttr(site, "IUSED".into()), None)
                .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
                .raw(Key::SiteAttr(site, "OUSED".into()), "0")
                .raw(Key::SiteAttr(site, "OSTANDARD".into()), "HSLVDCI_18")
                // Take shared mutex on global DCI.
                .raw(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");

            // Ensure nothing else in the bank.
            for i in 0..50 {
                let row = hclk_row_to - 25 + i;
                for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
                    if row == io_row_to && bel == defs::bslots::IOB[0] {
                        continue;
                    }
                    if let Some(site) = backend
                        .ngrid
                        .get_bel_name(CellCoord::new(die, col, row).bel(bel))
                    {
                        builder = builder.raw(Key::SiteMode(site), None);
                    }
                }
            }
            let site = backend.ngrid.get_bel_name(io_bel_to).unwrap();
            builder
                .raw(Key::SiteMode(site), "IOB")
                .raw(Key::SitePin(site, "O".into()), true)
                .raw(Key::SiteAttr(site, "IMUX".into()), None)
                .raw(Key::SiteAttr(site, "IUSED".into()), None)
                .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
                .raw_diff(Key::SiteAttr(site, "OUSED".into()), None, "0")
                .raw_diff(Key::SiteAttr(site, "OSTANDARD".into()), None, "HSLVDCI_18")
                .raw_diff(Key::DciCascade(actual_bank_to), None, actual_bank_from)
                .extra_tile_attr_fixed(io_tile_to, "IOB[0]", "OSTD", "HSLVDCI_18")
                .extra_tile_attr_fixed(
                    hclk_tile_to,
                    "DCI",
                    if bank_to == 0 {
                        "CASCADE_FROM_ABOVE"
                    } else {
                        "CASCADE_FROM_BELOW"
                    },
                    "1",
                )
                .test_manual("NULL", format!("CASCADE_DCI.{bank_to}"), "1")
                .commit();
        }
    }

    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "HCLK_IO_HR") {
        for val in ["OFF", "FREEZE", "ALWAYSACTIVE"] {
            ctx.test_manual("VCCOSENSE", "MODE", val)
                .prop(VccoSenseMode(val))
                .commit();
        }
        let mut ctx = FuzzCtx::new_null(session, backend);
        ctx.build()
            .extra_tiles_by_kind("HCLK_IO_HR", "VCCOSENSE")
            .test_manual("VCCOSENSE", "FLAG", "ENABLE")
            .global("VCCOSENSEFLAG", "ENABLE")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
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
        if !ctx.has_tile(tile) {
            continue;
        }

        ctx.collect_inv(tile, bel, "D");
        ctx.collect_inv(tile, bel, "CLKDIV");
        ctx.collect_inv(tile, bel, "CLKDIVP");
        let item = ctx.extract_enum_bool_wide(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.insert(tile, bel, "INV.CLK", item);
        let item = ctx.extract_bit(tile, bel, "OCLKINV", "OCLK");
        ctx.insert(tile, bel, "INV.OCLK1", item);
        let item = ctx.extract_bit(tile, bel, "OCLKINV", "OCLK_B");
        ctx.insert(tile, bel, "INV.OCLK2", item);
        ctx.collect_enum_bool(tile, bel, "DYN_CLK_INV_EN", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "DYN_CLKDIV_INV_EN", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "DYN_CLKDIVP_INV_EN", "FALSE", "TRUE");

        let iff_sr_used = ctx.extract_bit(tile, bel, "SRUSED", "0");
        ctx.insert(tile, bel, "IFF_SR_USED", iff_sr_used);
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["MASTER", "SLAVE"]);
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10", "14"] {
            diffs.push((val, ctx.get_diff(tile, bel, "DATA_WIDTH", val)));
        }
        let mut bits = xlat_enum(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.insert(
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
            ctx.insert(tile, bel, attr, item);
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

        let diff_mem = ctx.get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY");
        let diff_qdr = ctx.get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY_QDR");
        let diff_net = ctx.get_diff(tile, bel, "INTERFACE_TYPE", "NETWORKING");
        let diff_ddr3 = ctx.get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3");
        let diff_ddr3_v6 = ctx.get_diff(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3_V6");
        let diff_os = ctx.get_diff(tile, bel, "INTERFACE_TYPE", "OVERSAMPLE");
        let bitslip_en = diff_net.combine(&!&diff_qdr);
        let diff_net = diff_net.combine(&!&bitslip_en);
        let diff_os = diff_os.combine(&!&bitslip_en);
        ctx.insert(tile, bel, "BITSLIP_ENABLE", xlat_bit(bitslip_en));
        ctx.insert(
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

        let mut diff = ctx.get_diff(tile, bel, "IFFTYPE", "#LATCH");
        diff.apply_enum_diff(
            ctx.item(tile, bel, "DDR_CLK_EDGE"),
            "OPPOSITE_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "IFFTYPE", "#FF");
        diff.apply_enum_diff(
            ctx.item(tile, bel, "DDR_CLK_EDGE"),
            "OPPOSITE_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.apply_enum_diff(
            ctx.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.insert(tile, bel, "IFF_LATCH", xlat_bit(!diff));
        let mut diff = ctx.get_diff(tile, bel, "IFFTYPE", "DDR");
        diff.apply_enum_diff(
            ctx.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.insert(tile, bel, "IFF_LATCH", xlat_bit(!diff));

        let diff_f = ctx.get_diff(tile, bel, "SERDES", "FALSE");
        let diff_t = ctx.get_diff(tile, bel, "SERDES", "TRUE");
        let (diff_f, diff_t, mut diff_serdes) = Diff::split(diff_f, diff_t);
        ctx.insert(tile, bel, "SERDES", xlat_bool(diff_f, diff_t));
        diff_serdes.apply_bit_diff(ctx.item(tile, bel, "IFF_SR_USED"), true, false);
        diff_serdes.apply_bit_diff(ctx.item(tile, bel, "IFF_LATCH"), false, true);
        diff_serdes.assert_empty();

        let item = ctx.extract_enum(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = ctx.extract_enum(tile, bel, "D2OFFBYP_SEL", &["GND", "T"]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = xlat_enum(vec![
            ("T", ctx.get_diff(tile, bel, "TFB_USED", "TRUE")),
            ("GND", ctx.get_diff(tile, bel, "TFB_USED", "FALSE")),
        ]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);

        let item = ctx.extract_enum_bool(tile, bel, "IDELMUX", "1", "0");
        ctx.insert(tile, bel, "I_DELAY_ENABLE", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFFDELMUX", "1", "0");
        ctx.insert(tile, bel, "IFF_DELAY_ENABLE", item);

        ctx.get_diff(tile, bel, "IOBDELAY", "NONE").assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "IOBDELAY", "IBUF");
        diff.apply_bit_diff(ctx.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "IOBDELAY", "IFD");
        diff.apply_bit_diff(ctx.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "IOBDELAY", "BOTH");
        diff.apply_bit_diff(ctx.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.apply_bit_diff(ctx.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();

        let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
        ctx.insert(tile, bel, "I_TSBYPASS_ENABLE", item);
        // the fuzzer is slightly fucked to work around some ridiculous ISE bug.
        let _ = ctx.get_diff(tile, bel, "IFFMUX", "1");
        let item = ctx.extract_bit(tile, bel, "IFFMUX", "0");
        ctx.insert(tile, bel, "IFF_TSBYPASS_ENABLE", item);
        ctx.get_diff(tile, bel, "OFB_USED", "FALSE").assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "OFB_USED", "TRUE");
        diff.apply_bit_diff(ctx.item(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.apply_bit_diff(ctx.item(tile, bel, "IFF_TSBYPASS_ENABLE"), true, false);
        diff.assert_empty();

        ctx.get_diff(tile, bel, "PRESENT", "ILOGICE2")
            .assert_empty();
        let mut present_iserdes = ctx.get_diff(tile, bel, "PRESENT", "ISERDESE2");
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "IFF1_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "IFF2_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "IFF3_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "IFF4_SRVAL"), false, true);
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "IFF1_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "IFF2_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "IFF3_INIT"), false, true);
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "IFF4_INIT"), false, true);
        present_iserdes.assert_empty();

        if tile.contains("HR") {
            ctx.get_diff(tile, bel, "PRESENT", "ILOGICE3")
                .assert_empty();

            ctx.collect_bitvec(tile, bel, "IDELAY_VALUE", "");
            ctx.collect_bitvec(tile, bel, "IFFDELAY_VALUE", "");
            let item = ctx.extract_enum_bool(tile, bel, "ZHOLD_FABRIC", "FALSE", "TRUE");
            ctx.insert(tile, bel, "ZHOLD_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "ZHOLD_IFF", "FALSE", "TRUE");
            ctx.insert(tile, bel, "ZHOLD_ENABLE", item);

            let diff0 = ctx.get_diff(tile, bel, "ZHOLD_FABRIC_INV", "D");
            let diff1 = ctx.get_diff(tile, bel, "ZHOLD_FABRIC_INV", "D_B");
            let (diff0, diff1, diff_en) = Diff::split(diff0, diff1);
            ctx.insert(tile, bel, "INV.ZHOLD_FABRIC", xlat_bool(diff0, diff1));
            ctx.insert(tile, bel, "I_ZHOLD", xlat_bit(diff_en));

            let diff0 = ctx.get_diff(tile, bel, "ZHOLD_IFF_INV", "D");
            let diff1 = ctx.get_diff(tile, bel, "ZHOLD_IFF_INV", "D_B");
            let (diff0, diff1, diff_en) = Diff::split(diff0, diff1);
            ctx.insert(tile, bel, "INV.ZHOLD_IFF", xlat_bool(diff0, diff1));
            ctx.insert(tile, bel, "IFF_ZHOLD", xlat_bit(diff_en));
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
        ("IO_HR_PAIR", "OLOGIC[0]"),
        ("IO_HR_PAIR", "OLOGIC[1]"),
        ("IO_HR_S", "OLOGIC[0]"),
        ("IO_HR_N", "OLOGIC[0]"),
        ("IO_HP_PAIR", "OLOGIC[0]"),
        ("IO_HP_PAIR", "OLOGIC[1]"),
        ("IO_HP_S", "OLOGIC[0]"),
        ("IO_HP_N", "OLOGIC[0]"),
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

        ctx.get_diff(tile, bel, "CLKINV.SAME", "CLK_B")
            .assert_empty();
        let diff_clk1 = ctx.get_diff(tile, bel, "CLKINV.OPPOSITE", "CLK");
        let diff_clk2 = ctx.get_diff(tile, bel, "CLKINV.OPPOSITE", "CLK_B");
        let diff_clk12 = ctx.get_diff(tile, bel, "CLKINV.SAME", "CLK");
        assert_eq!(diff_clk12, diff_clk1.combine(&diff_clk2));
        ctx.insert(tile, bel, "INV.CLK1", xlat_bit(!diff_clk1));
        ctx.insert(tile, bel, "INV.CLK2", xlat_bit(!diff_clk2));

        let item_oq = ctx.extract_enum_bool(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_enum_bool(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.get_diff(tile, bel, "SRTYPE", "ASYNC").assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bit_diff(&item_oq, true, false);
        diff.apply_bit_diff(&item_tq, true, false);
        diff.assert_empty();
        ctx.insert(tile, bel, "OFF_SR_SYNC", item_oq);
        ctx.insert(tile, bel, "TFF_SR_SYNC", item_tq);

        let item = ctx.extract_enum_bool(tile, bel, "INIT_OQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "OFF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "INIT_OQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "OFF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "INIT_TQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_enum_bool(tile, bel, "INIT_TQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_OQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_OQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_TQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "TFF_SRVAL", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "SRVAL_TQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "TFF_SRVAL", item);

        let osrused = ctx.extract_bit(tile, bel, "OSRUSED", "0");
        let tsrused = ctx.extract_bit(tile, bel, "TSRUSED", "0");
        ctx.insert(tile, bel, "OFF_SR_USED", osrused);
        ctx.insert(tile, bel, "TFF_SR_USED", tsrused);

        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "MISR_ENABLE_FDBK", "FALSE", "TRUE");
        ctx.collect_enum_default(tile, bel, "MISR_CLK_SELECT", &["CLK1", "CLK2"], "NONE");
        if !tile.ends_with("PAIR") {
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
            diffs.push((val, val, ctx.get_diff(tile, bel, "DATA_WIDTH.SDR", val)));
        }
        for (val, ratio) in [("4", "2"), ("6", "3"), ("8", "4"), ("10", "5"), ("14", "7")] {
            diffs.push((val, ratio, ctx.get_diff(tile, bel, "DATA_WIDTH.DDR", val)));
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
        ctx.insert(tile, bel, "DATA_WIDTH", xlat_enum(diffs_width));
        ctx.insert(tile, bel, "CLK_RATIO", xlat_enum(diffs_ratio));

        let mut diff_sdr = ctx.get_diff(tile, bel, "DATA_RATE_OQ", "SDR");
        let mut diff_ddr = ctx.get_diff(tile, bel, "DATA_RATE_OQ", "DDR");
        diff_sdr.apply_bit_diff(ctx.item(tile, bel, "OFF_SR_USED"), true, false);
        diff_ddr.apply_bit_diff(ctx.item(tile, bel, "OFF_SR_USED"), true, false);
        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("D1", ctx.get_diff(tile, bel, "OMUX", "D1")),
            ("SERDES_SDR", diff_sdr),
            ("DDR", diff_ddr),
            ("FF", ctx.get_diff(tile, bel, "OUTFFTYPE", "#FF")),
            ("DDR", ctx.get_diff(tile, bel, "OUTFFTYPE", "DDR")),
            ("LATCH", ctx.get_diff(tile, bel, "OUTFFTYPE", "#LATCH")),
        ]);
        ctx.insert(tile, bel, "OMUX", item);

        let mut diff_sdr = ctx.get_diff(tile, bel, "DATA_RATE_TQ", "SDR");
        let mut diff_ddr = ctx.get_diff(tile, bel, "DATA_RATE_TQ", "DDR");
        diff_sdr.apply_bit_diff(ctx.item(tile, bel, "TFF_SR_USED"), true, false);
        diff_ddr.apply_bit_diff(ctx.item(tile, bel, "TFF_SR_USED"), true, false);
        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("T1", ctx.get_diff(tile, bel, "DATA_RATE_TQ", "BUF")),
            ("SERDES_SDR", diff_sdr),
            ("DDR", diff_ddr),
            ("FF", ctx.get_diff(tile, bel, "TFFTYPE", "#FF")),
            ("DDR", ctx.get_diff(tile, bel, "TFFTYPE", "DDR")),
            ("LATCH", ctx.get_diff(tile, bel, "TFFTYPE", "#LATCH")),
        ]);
        ctx.insert(tile, bel, "TMUX", item);

        let mut present_ologic = ctx.get_diff(tile, bel, "PRESENT", "OLOGICE2");
        present_ologic.apply_bit_diff(ctx.item(tile, bel, "RANK3_USED"), false, true);
        present_ologic.apply_enum_diff(ctx.item(tile, bel, "TMUX"), "T1", "NONE");
        present_ologic.assert_empty();
        let mut present_oserdes = ctx.get_diff(tile, bel, "PRESENT", "OSERDESE2");
        present_oserdes.apply_bitvec_diff_int(ctx.item(tile, bel, "OFF_SRVAL"), 0, 7);
        present_oserdes.apply_bitvec_diff_int(ctx.item(tile, bel, "TFF_SRVAL"), 0, 7);
        present_oserdes.apply_bit_diff(ctx.item(tile, bel, "OFF_INIT"), false, true);
        present_oserdes.apply_bit_diff(ctx.item(tile, bel, "TFF_INIT"), false, true);
        present_oserdes.assert_empty();

        let mut diffs_clk = vec![("NONE".to_string(), Diff::default())];
        let mut diffs_clkb = vec![("NONE".to_string(), Diff::default())];
        for (src, num) in [("HCLK", 6), ("RCLK", 4), ("IOCLK", 4)] {
            for i in 0..num {
                diffs_clk.push((
                    format!("{src}{i}"),
                    ctx.get_diff(tile, bel, "MUX.CLK", format!("{src}{i}")),
                ));
                diffs_clkb.push((
                    format!("{src}{i}"),
                    ctx.get_diff(tile, bel, "MUX.CLKB", format!("{src}{i}")),
                ));
            }
        }
        for val in ["CKINT", "PHASER_OCLK"] {
            diffs_clk.push((val.to_string(), ctx.get_diff(tile, bel, "MUX.CLK", val)));
            diffs_clkb.push((val.to_string(), ctx.get_diff(tile, bel, "MUX.CLKB", val)));
        }
        let diff_clk = ctx.get_diff(tile, bel, "MUX.CLK", "PHASER_OCLK90");
        let diff_clkb = ctx
            .get_diff(tile, bel, "MUX.CLK", "PHASER_OCLK90.BOTH")
            .combine(&!&diff_clk);
        diffs_clk.push(("PHASER_OCLK90".to_string(), diff_clk));
        diffs_clkb.push(("PHASER_OCLK90".to_string(), diff_clkb));
        ctx.insert(tile, bel, "MUX.CLK", xlat_enum_ocd(diffs_clk, OcdMode::Mux));
        ctx.insert(
            tile,
            bel,
            "MUX.CLKB",
            xlat_enum_ocd(diffs_clkb, OcdMode::Mux),
        );

        for (attr, attrf) in [
            ("MUX.CLKDIV", "MUX.CLKDIVF"),
            ("MUX.CLKDIVB", "MUX.CLKDIVFB"),
        ] {
            let diff_hclk0f = ctx.get_diff(tile, bel, attr, "HCLK0.F");
            let diff_f = ctx
                .peek_diff(tile, bel, attr, "HCLK0")
                .combine(&!diff_hclk0f);
            let mut diffs = vec![("NONE", Diff::default())];
            for val in [
                "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "RCLK0", "RCLK1", "RCLK2",
                "RCLK3", "CKINT",
            ] {
                diffs.push((val, ctx.get_diff(tile, bel, attr, val).combine(&!&diff_f)));
            }
            ctx.insert(tile, bel, attrf, xlat_enum_ocd(diffs, OcdMode::Mux));
            let item = xlat_enum(vec![
                ("NONE", Diff::default()),
                (&attrf[4..], diff_f),
                (
                    "PHASER_OCLKDIV",
                    ctx.get_diff(tile, bel, attr, "PHASER_OCLKDIV"),
                ),
            ]);
            ctx.insert(tile, bel, attr, item);
        }
    }
    for tile in ["IO_HR_PAIR", "IO_HP_PAIR"] {
        if !ctx.has_tile(tile) {
            continue;
        }
        let mut diff = ctx.get_diff(tile, "OLOGIC_COMMON", "MISR_RESET", "1");
        let diff1 = diff.split_bits_by(|bit| bit.rect.to_idx() > 0);
        ctx.insert(tile, "OLOGIC[0]", "MISR_RESET", xlat_bit(diff));
        ctx.insert(tile, "OLOGIC[1]", "MISR_RESET", xlat_bit(diff1));
    }
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

        ctx.get_diff(tile, bel, "DELAYCHAIN_OSC", "FALSE")
            .assert_empty();
        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("IDATAIN", ctx.get_diff(tile, bel, "DELAY_SRC", "IDATAIN")),
            ("DATAIN", ctx.get_diff(tile, bel, "DELAY_SRC", "DATAIN")),
            ("OFB", ctx.get_diff(tile, bel, "DELAY_SRC", "OFB")),
            (
                "DELAYCHAIN_OSC",
                ctx.get_diff(tile, bel, "DELAYCHAIN_OSC", "TRUE"),
            ),
        ]);
        ctx.insert(tile, bel, "DELAY_SRC", item);

        let item = xlat_enum(vec![
            ("FIXED", ctx.get_diff(tile, bel, "IDELAY_TYPE", "FIXED")),
            (
                "VARIABLE",
                ctx.get_diff(tile, bel, "IDELAY_TYPE", "VARIABLE"),
            ),
            (
                "VAR_LOAD",
                ctx.get_diff(tile, bel, "IDELAY_TYPE", "VAR_LOAD"),
            ),
            (
                "VAR_LOAD",
                ctx.get_diff(tile, bel, "IDELAY_TYPE", "VAR_LOAD_PIPE"),
            ),
        ]);
        ctx.insert(tile, bel, "IDELAY_TYPE", item);
        let mut diffs_t = vec![];
        let mut diffs_f = vec![];
        for diff in ctx.get_diffs(tile, bel, "IDELAY_VALUE", "") {
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
        ctx.insert(tile, bel, "IDELAY_VALUE_INIT", xlat_bitvec(diffs_t));
        ctx.insert(tile, bel, "IDELAY_VALUE_CUR", xlat_bitvec(diffs_f));
        if tile.contains("HP") {
            ctx.collect_enum(tile, bel, "FINEDELAY", &["BYPASS", "ADD_DLY"]);
        }
    }
    for (tile, bel) in [
        ("IO_HP_PAIR", "ODELAY[0]"),
        ("IO_HP_PAIR", "ODELAY[1]"),
        ("IO_HP_S", "ODELAY[0]"),
        ("IO_HP_N", "ODELAY[0]"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        ctx.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.collect_inv(tile, bel, "C");
        ctx.collect_inv(tile, bel, "ODATAIN");
        ctx.collect_enum_bool(tile, bel, "HIGH_PERFORMANCE_MODE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CINVCTRL_SEL", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "PIPE_SEL", "FALSE", "TRUE");
        ctx.get_diff(tile, bel, "DELAYCHAIN_OSC", "FALSE")
            .assert_empty();

        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("ODATAIN", ctx.get_diff(tile, bel, "DELAY_SRC", "ODATAIN")),
            ("CLKIN", ctx.get_diff(tile, bel, "DELAY_SRC", "CLKIN")),
            (
                "DELAYCHAIN_OSC",
                ctx.get_diff(tile, bel, "DELAYCHAIN_OSC", "TRUE"),
            ),
        ]);
        ctx.insert(tile, bel, "DELAY_SRC", item);

        let en = ctx.extract_bit(tile, bel, "ODELAY_TYPE", "FIXED");
        let mut diff_var = ctx.get_diff(tile, bel, "ODELAY_TYPE", "VARIABLE");
        diff_var.apply_bit_diff(&en, true, false);
        let mut diff_vl = ctx.get_diff(tile, bel, "ODELAY_TYPE", "VAR_LOAD");
        diff_vl.apply_bit_diff(&en, true, false);
        ctx.insert(tile, bel, "ENABLE", en);
        ctx.insert(
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
        for diff in ctx.get_diffs(tile, bel, "ODELAY_VALUE", "") {
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
        ctx.insert(tile, bel, "ODELAY_VALUE_INIT", xlat_bitvec(diffs_t));
        ctx.insert(tile, bel, "ODELAY_VALUE_CUR", xlat_bitvec(diffs_f));
        ctx.collect_enum(tile, bel, "FINEDELAY", &["BYPASS", "ADD_DLY"]);
    }
    for (tile, bel) in [
        ("IO_HP_PAIR", "IOB[0]"),
        ("IO_HP_PAIR", "IOB[1]"),
        ("IO_HP_S", "IOB[0]"),
        ("IO_HP_N", "IOB[0]"),
    ] {
        if !ctx.has_tile(tile) {
            continue;
        }
        ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
        ctx.collect_enum(tile, bel, "DCITERMDISABLE_SEL", &["I", "GND"]);
        ctx.collect_enum(tile, bel, "IBUFDISABLE_SEL", &["I", "GND"]);
        ctx.collect_bit(tile, bel, "PULL_DYNAMIC", "1");
        ctx.collect_enum_bool(tile, bel, "OUTPUT_DELAY", "0", "1");
        let mut present = ctx.get_diff(tile, bel, "PRESENT", "IOB");
        if tile == "IO_HP_PAIR" {
            let diff = ctx
                .get_diff(tile, bel, "PRESENT", "IPAD")
                .combine(&!&present);
            ctx.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
            if bel == "IOB[0]" {
                let diff = ctx
                    .get_diff(tile, bel, "PRESENT", "VREF")
                    .combine(&!&present);
                ctx.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
            }
        }
        let diff = ctx
            .get_diff(tile, bel, "PRESENT", "IOB.QUIET")
            .combine(&!&present);
        ctx.insert(tile, bel, "DCIUPDATEMODE_QUIET", xlat_bit(diff));
        present.apply_enum_diff(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

        let iprog = ctx.get_diffs(tile, bel, "IPROGRAMMING", "");
        ctx.insert(tile, bel, "INPUT_MISC", xlat_bit(iprog[19].clone()));

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
                    ("NONE".into(), bits![0, 0]),
                    ("OUTPUT".into(), bits![1, 0]),
                    ("OUTPUT_HALF".into(), bits![0, 1]),
                    ("TERM_SPLIT".into(), bits![1, 1]),
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
        let tidx = if bel == "IOB[1]" { 1 } else { 0 };

        let (pslew_bits, nslew_bits) =
            if (tile == "IO_HP_PAIR" && bel == "IOB[0]") || tile == "IO_HP_N" {
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
            .peek_diff(tile, bel, "OSTD", "HSTL_I.FAST")
            .combine(&present);
        for &bit in &pdrive_bits {
            diff.bits.remove(&bit);
        }
        for &bit in &ndrive_bits {
            diff.bits.remove(&bit);
        }
        extract_bitvec_val_part(&pslew, &bits![0; 5], &mut diff);
        extract_bitvec_val_part(&nslew, &bits![0; 5], &mut diff);
        ctx.insert(tile, bel, "OUTPUT_ENABLE", xlat_bit_wide(diff));

        let diff_cmos = ctx.peek_diff(tile, bel, "ISTD", "LVCMOS18.LP");
        let diff_vref_lp = ctx.peek_diff(tile, bel, "ISTD", "HSTL_I.LP");
        let diff_vref_hp = ctx.peek_diff(tile, bel, "ISTD", "HSTL_I.HP");
        let mut diffs = vec![
            ("OFF", Diff::default()),
            ("CMOS", diff_cmos.clone()),
            ("VREF_LP", diff_vref_lp.clone()),
            ("VREF_HP", diff_vref_hp.clone()),
        ];
        if tile == "IO_HP_PAIR" {
            let mut diff_diff_lp = ctx.peek_diff(tile, bel, "ISTD", "LVDS.LP").clone();
            let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.rect.to_idx() == tidx);
            let mut diff_diff_hp = ctx.peek_diff(tile, bel, "ISTD", "LVDS.HP").clone();
            let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.rect.to_idx() == tidx);
            diffs.extend([("DIFF_LP", diff_diff_lp), ("DIFF_HP", diff_diff_hp)]);
        }
        ctx.insert(tile, bel, "IBUF_MODE", xlat_enum(diffs));

        for &std in HP_IOSTDS {
            if tile != "IO_HP_PAIR" && std.name != "HSTL_I" {
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
                    let mut diff = ctx.get_diff(tile, bel, "OSTD", &val);
                    diff.apply_bitvec_diff(
                        ctx.item(tile, bel, "OUTPUT_ENABLE"),
                        &bits![1; 2],
                        &bits![0; 2],
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
                                        assert_eq!(val, !inv);
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
                            ctx.insert_misc_data(format!("HP_IOSTD:{attr}:{name}"), value);
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
                        ctx.insert_misc_data(format!("HP_IOSTD:{attr}:{name}"), value);
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
                                ctx.item(tile, bel, "IBUF_MODE"),
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
            if tile != "IO_HP_PAIR" && !matches!(std.name, "LVCMOS18" | "HSTL_I") {
                continue;
            }
            if std.diff != DiffKind::None {
                continue;
            }
            if std.dci == DciKind::BiSplitT {
                continue;
            }
            for lp in ["HP", "LP"] {
                let mut diff = ctx.get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                match std.dci {
                    DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                    DciKind::InputSplit | DciKind::BiSplit => {
                        diff.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
                    }
                    _ => unreachable!(),
                }
                let mode = if std.vref.is_some() {
                    if lp == "LP" { "VREF_LP" } else { "VREF_HP" }
                } else {
                    "CMOS"
                };
                diff.apply_enum_diff(ctx.item(tile, bel, "IBUF_MODE"), mode, "OFF");
                diff.assert_empty();
            }
        }

        if tile != "IO_HP_PAIR" {
            let mut present_vr = ctx.get_diff(tile, bel, "PRESENT", "VR");
            present_vr.apply_enum_diff(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");
            present_vr.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
            for (attr, bits, invert) in [
                ("PDRIVE", &pdrive_bits, &pdrive_invert),
                ("NDRIVE", &ndrive_bits, &ndrive_invert),
                ("PSLEW", &pslew.bits, &bits![0; 5]),
                ("NSLEW", &nslew.bits, &bits![0; 5]),
            ] {
                let value: BitVec = bits
                    .iter()
                    .zip(invert.iter())
                    .map(|(&bit, inv)| match present_vr.bits.remove(&bit) {
                        Some(true) => !inv,
                        None => inv,
                        _ => unreachable!(),
                    })
                    .collect();
                if attr.contains("DRIVE") {
                    assert!(!value.any());
                } else {
                    ctx.insert_misc_data(format!("HP_IOSTD:{attr}:VR"), value);
                }
            }
            ctx.insert(tile, bel, "VR", xlat_bit(present_vr));
        }

        ctx.insert_misc_data("HP_IOSTD:LVDS_T:OFF", bits![0; 9]);
        ctx.insert_misc_data("HP_IOSTD:LVDS_C:OFF", bits![0; 9]);
        ctx.insert_misc_data("HP_IOSTD:PDRIVE:OFF", bits![0; 7]);
        ctx.insert_misc_data("HP_IOSTD:NDRIVE:OFF", bits![0; 7]);
        ctx.insert_misc_data("HP_IOSTD:PSLEW:OFF", bits![0; 5]);
        ctx.insert_misc_data("HP_IOSTD:NSLEW:OFF", bits![0; 5]);
        ctx.insert(tile, bel, "LVDS", lvds);
        ctx.insert(tile, bel, "DCI_T", dci_t);
        ctx.insert(tile, bel, "DQS_BIAS_N", dqsbias_n);
        ctx.insert(tile, bel, "DQS_BIAS_P", dqsbias_p);
        ctx.insert(tile, bel, "DCI_MODE", dci_mode);
        ctx.insert(tile, bel, "OUTPUT_MISC", output_misc);
        ctx.insert(
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
        ctx.insert(
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
        ctx.insert(tile, bel, "PSLEW", pslew);
        ctx.insert(tile, bel, "NSLEW", nslew);

        present.assert_empty();
    }

    if ctx.has_tile("IO_HP_PAIR") {
        let tile = "IO_HP_PAIR";
        for &std in HP_IOSTDS {
            if std.diff == DiffKind::None {
                continue;
            }
            for bel in ["IOB[0]", "IOB[1]"] {
                for lp in ["HP", "LP"] {
                    if std.dci == DciKind::BiSplitT {
                        continue;
                    }
                    let mut diff =
                        ctx.get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                    for cbel in ["IOB[0]", "IOB[1]"] {
                        match std.dci {
                            DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                            DciKind::InputSplit | DciKind::BiSplit => {
                                diff.apply_enum_diff(
                                    ctx.item(tile, cbel, "DCI_MODE"),
                                    "TERM_SPLIT",
                                    "NONE",
                                );
                            }
                            _ => unreachable!(),
                        }
                        diff.apply_enum_diff(
                            ctx.item(tile, cbel, "IBUF_MODE"),
                            if lp == "LP" { "DIFF_LP" } else { "DIFF_HP" },
                            "OFF",
                        );
                    }
                    diff.assert_empty();
                }
            }
            if std.diff == DiffKind::True {
                let mut diff = ctx.get_diff(tile, "IOB[0]", "DIFF_TERM", std.name);
                let val_c = extract_bitvec_val_part(
                    ctx.item(tile, "IOB[0]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.item(tile, "IOB[1]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("HP_IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                ctx.insert_misc_data(format!("HP_IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                diff.assert_empty();

                let mut diff = ctx.get_diff(tile, "IOB[0]", "DIFF_TERM_DYNAMIC", std.name);
                let val_c = extract_bitvec_val_part(
                    ctx.item(tile, "IOB[0]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.item(tile, "IOB[1]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("HP_IOSTD:LVDS_T:TERM_DYNAMIC_{}", std.name), val_t);
                ctx.insert_misc_data(format!("HP_IOSTD:LVDS_C:TERM_DYNAMIC_{}", std.name), val_c);
                diff.assert_empty();

                let mut diff = ctx.get_diff(tile, "IOB[1]", "OSTD", std.name);
                let val_c = extract_bitvec_val_part(
                    ctx.item(tile, "IOB[0]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.item(tile, "IOB[1]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("HP_IOSTD:LVDS_T:OUTPUT_{}", std.name), val_t);
                ctx.insert_misc_data(format!("HP_IOSTD:LVDS_C:OUTPUT_{}", std.name), val_c);
                diff.apply_bitvec_diff(
                    ctx.item(tile, "IOB[1]", "OUTPUT_ENABLE"),
                    &bits![1; 2],
                    &bits![0; 2],
                );
                diff.assert_empty();
            }
            if std.diff == DiffKind::Pseudo {
                for slew in ["SLOW", "FAST"] {
                    let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                    let mut diff = ctx.get_diff(
                        tile,
                        "IOB[1]",
                        "OSTD",
                        format!("{sn}.{slew}", sn = std.name),
                    );
                    for bel in ["IOB[0]", "IOB[1]"] {
                        diff.apply_bitvec_diff(
                            ctx.item(tile, bel, "OUTPUT_ENABLE"),
                            &bits![1; 2],
                            &bits![0; 2],
                        );
                        if !matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                            for attr in ["PDRIVE", "NDRIVE"] {
                                let item = ctx.item(tile, bel, attr);
                                let value = extract_bitvec_val_part(
                                    item,
                                    &BitVec::repeat(false, item.bits.len()),
                                    &mut diff,
                                );
                                ctx.insert_misc_data(format!("HP_IOSTD:{attr}:{stdname}"), value);
                            }
                        }
                        for attr in ["PSLEW", "NSLEW"] {
                            let item = ctx.item(tile, bel, attr);
                            let value = extract_bitvec_val_part(
                                item,
                                &BitVec::repeat(false, item.bits.len()),
                                &mut diff,
                            );
                            ctx.insert_misc_data(
                                format!("HP_IOSTD:{attr}:{stdname}.{slew}"),
                                value,
                            );
                        }
                        let dci_mode = ctx.item(tile, bel, "DCI_MODE");
                        let dci_t = ctx.item(tile, bel, "DCI_T");
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
                    let diff_t = diff.split_bits_by(|bit| bit.bit.to_idx() == 17);
                    assert_eq!(diff.bits.len(), 1);
                    assert_eq!(diff_t.bits.len(), 1);
                    ctx.insert(
                        tile,
                        "IOB[0]",
                        "OMUX",
                        xlat_enum(vec![("O", Diff::default()), ("OTHER_O_INV", diff)]),
                    );
                    ctx.insert(
                        tile,
                        "IOB[0]",
                        "TMUX",
                        xlat_enum(vec![("T", Diff::default()), ("OTHER_T", diff_t)]),
                    );
                }
            }
        }
    }

    if ctx.has_tile("HCLK_IO_HP") {
        let tile = "HCLK_IO_HP";
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
                let diff = ctx.get_diff(tile, bel, "STD", std.name);
                let val = extract_bitvec_val(&lvdsbias, &bits![0; 18], diff);
                ctx.insert_misc_data(format!("HP_IOSTD:LVDSBIAS:{}", std.name), val);
            }
            if std.dci != DciKind::None {
                let bel = "DCI";
                let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                let mut diff = ctx.get_diff(tile, bel, "STD", std.name);
                match std.dci {
                    DciKind::Output => {
                        let val = extract_bitvec_val_part(&nref_output, &bits![0; 2], &mut diff);
                        ctx.insert_misc_data(format!("HP_IOSTD:DCI:NREF_OUTPUT:{stdname}"), val);
                        let val = extract_bitvec_val_part(&pref_output, &bits![0; 2], &mut diff);
                        ctx.insert_misc_data(format!("HP_IOSTD:DCI:PREF_OUTPUT:{stdname}"), val);
                    }
                    DciKind::OutputHalf => {
                        let val =
                            extract_bitvec_val_part(&nref_output_half, &bits![0; 3], &mut diff);
                        ctx.insert_misc_data(
                            format!("HP_IOSTD:DCI:NREF_OUTPUT_HALF:{stdname}"),
                            val,
                        );
                        let val =
                            extract_bitvec_val_part(&pref_output_half, &bits![0; 3], &mut diff);
                        ctx.insert_misc_data(
                            format!("HP_IOSTD:DCI:PREF_OUTPUT_HALF:{stdname}"),
                            val,
                        );
                    }
                    DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                        let val =
                            extract_bitvec_val_part(&nref_term_split, &bits![0; 3], &mut diff);
                        ctx.insert_misc_data(
                            format!("HP_IOSTD:DCI:NREF_TERM_SPLIT:{stdname}"),
                            val,
                        );
                    }
                    _ => {}
                }
                ctx.insert(tile, bel, "ENABLE", xlat_bit(diff));
            }
        }
        let bel = "LVDS";
        ctx.insert(tile, bel, "LVDSBIAS", lvdsbias);
        ctx.insert_misc_data("HP_IOSTD:LVDSBIAS:OFF", bits![0; 18]);
        let bel = "DCI";
        ctx.insert(tile, bel, "PREF_OUTPUT", pref_output);
        ctx.insert(tile, bel, "NREF_OUTPUT", nref_output);
        ctx.insert(tile, bel, "PREF_OUTPUT_HALF", pref_output_half);
        ctx.insert(tile, bel, "NREF_OUTPUT_HALF", nref_output_half);
        ctx.insert(tile, bel, "NREF_TERM_SPLIT", nref_term_split);
        ctx.insert_misc_data("HP_IOSTD:DCI:PREF_OUTPUT:OFF", bits![0; 2]);
        ctx.insert_misc_data("HP_IOSTD:DCI:NREF_OUTPUT:OFF", bits![0; 2]);
        ctx.insert_misc_data("HP_IOSTD:DCI:PREF_OUTPUT_HALF:OFF", bits![0; 3]);
        ctx.insert_misc_data("HP_IOSTD:DCI:NREF_OUTPUT_HALF:OFF", bits![0; 3]);
        ctx.insert_misc_data("HP_IOSTD:DCI:NREF_TERM_SPLIT:OFF", bits![0; 3]);

        let dci_en = ctx.get_diff(tile, bel, "ENABLE", "1");
        let test_en = ctx.get_diff(tile, bel, "TEST_ENABLE", "1");
        let quiet = ctx
            .get_diff(tile, bel, "TEST_ENABLE", "QUIET")
            .combine(&!&test_en);
        ctx.insert(tile, bel, "QUIET", xlat_bit(quiet));
        let test_en = test_en.combine(&!&dci_en);
        let dyn_en = ctx
            .get_diff(tile, bel, "DYNAMIC_ENABLE", "1")
            .combine(&!&dci_en);
        ctx.insert(tile, bel, "TEST_ENABLE", xlat_bit_wide(test_en));
        ctx.insert(tile, bel, "DYNAMIC_ENABLE", xlat_bit(dyn_en));
        let casc_from_above = ctx
            .get_diff(tile, bel, "CASCADE_FROM_ABOVE", "1")
            .combine(&!&dci_en);
        ctx.insert(
            tile,
            bel,
            "CASCADE_FROM_ABOVE",
            xlat_bit_wide(casc_from_above),
        );
        let casc_from_below = ctx
            .get_diff(tile, bel, "CASCADE_FROM_BELOW", "1")
            .combine(&!&dci_en);
        ctx.insert(
            tile,
            bel,
            "CASCADE_FROM_BELOW",
            xlat_bit_wide(casc_from_below),
        );
        ctx.insert(tile, bel, "ENABLE", xlat_bit(dci_en));

        let mut diffs = vec![("OFF", Diff::default())];
        for val in ["600", "675", "750", "900"] {
            diffs.push((val, ctx.get_diff(tile, "INTERNAL_VREF", "VREF", val)));
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
        ctx.insert(tile, "INTERNAL_VREF", "VREF", xlat_enum(diffs));

        ctx.collect_bit_wide("CFG", "MISC", "DCI_CLK_ENABLE", "1");
    }
    for (tile, bel) in [
        ("IO_HR_PAIR", "IOB[0]"),
        ("IO_HR_PAIR", "IOB[1]"),
        ("IO_HR_S", "IOB[0]"),
        ("IO_HR_N", "IOB[0]"),
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

        let mut present = ctx.get_diff(tile, bel, "PRESENT", "IOB");
        if tile == "IO_HR_PAIR" {
            let diff = ctx
                .get_diff(tile, bel, "PRESENT", "IPAD")
                .combine(&!&present);
            ctx.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
            if bel == "IOB[0]" {
                let diff = ctx
                    .get_diff(tile, bel, "PRESENT", "VREF")
                    .combine(&!&present);
                ctx.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
            }
        }
        present.apply_enum_diff(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

        let tidx = if bel == "IOB[1]" { 1 } else { 0 };
        let diff_cmos_lv = ctx.peek_diff(tile, bel, "ISTD", "LVCMOS18.LP");
        let diff_cmos_hv = ctx.peek_diff(tile, bel, "ISTD", "LVCMOS33.LP");
        let diff_vref_lp = ctx.peek_diff(tile, bel, "ISTD", "HSTL_I.LP");
        let diff_vref_hp = ctx.peek_diff(tile, bel, "ISTD", "HSTL_I.HP");
        let diff_pci = ctx.peek_diff(tile, bel, "ISTD", "PCI33_3.LP");
        let mut diffs = vec![
            ("OFF", Diff::default()),
            ("VREF_LP", diff_vref_lp.clone()),
            ("CMOS_LV", diff_cmos_lv.clone()),
            ("CMOS_HV", diff_cmos_hv.clone()),
            ("PCI", diff_pci.clone()),
            ("VREF_HP", diff_vref_hp.clone()),
        ];
        if tile == "IO_HR_PAIR" {
            let mut diff_diff_lp = ctx.peek_diff(tile, bel, "ISTD", "LVDS_25.LP").clone();
            let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.rect.to_idx() == tidx);
            let mut diff_diff_hp = ctx.peek_diff(tile, bel, "ISTD", "LVDS_25.HP").clone();
            let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.rect.to_idx() == tidx);
            let mut diff_tmds_lp = ctx.peek_diff(tile, bel, "ISTD", "TMDS_33.LP").clone();
            let diff_tmds_lp = diff_tmds_lp.split_bits_by(|bit| bit.rect.to_idx() == tidx);
            let mut diff_tmds_hp = ctx.peek_diff(tile, bel, "ISTD", "TMDS_33.HP").clone();
            let diff_tmds_hp = diff_tmds_hp.split_bits_by(|bit| bit.rect.to_idx() == tidx);
            diffs.extend([
                ("DIFF_LP", diff_diff_lp),
                ("DIFF_HP", diff_diff_hp),
                ("TMDS_LP", diff_tmds_lp),
                ("TMDS_HP", diff_tmds_hp),
            ]);
        }
        ctx.insert(tile, bel, "IBUF_MODE", xlat_enum(diffs));

        let iprog = ctx.get_diffs(tile, bel, "IPROGRAMMING", "");
        ctx.insert(tile, bel, "INPUT_MISC", xlat_bit(iprog[7].clone()));

        for &std in HR_IOSTDS {
            if tile != "IO_HR_PAIR"
                && !matches!(std.name, "LVCMOS18" | "LVCMOS33" | "PCI33_3" | "HSTL_I")
            {
                continue;
            }
            if std.diff != DiffKind::None {
                continue;
            }
            for lp in ["HP", "LP"] {
                let mut diff = ctx.get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                let mode = if std.vref.is_some() {
                    if lp == "LP" { "VREF_LP" } else { "VREF_HP" }
                } else if std.name == "PCI33_3" {
                    "PCI"
                } else if std.vcco.unwrap() < 2500 {
                    "CMOS_LV"
                } else {
                    "CMOS_HV"
                };
                diff.apply_enum_diff(ctx.item(tile, bel, "IBUF_MODE"), mode, "OFF");
                diff.assert_empty();
            }
        }

        let mut oprog = ctx.get_diffs(tile, bel, "OPROGRAMMING", "");
        ctx.insert(tile, bel, "OUTPUT_ENABLE", xlat_bitvec(oprog.split_off(37)));
        if tile == "IO_HR_PAIR" && bel == "IOB[0]" {
            ctx.insert(
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
        ctx.insert(tile, bel, "OUTPUT_MISC_B", xlat_bit(oprog.pop().unwrap()));
        ctx.insert(tile, bel, "LOW_VOLTAGE", xlat_bit(oprog.pop().unwrap()));
        let slew_bits = xlat_bitvec(oprog.split_off(24)).bits;
        ctx.insert(tile, bel, "OUTPUT_MISC", xlat_bitvec(oprog.split_off(21)));
        let drive_bits = xlat_bitvec(oprog.split_off(14)).bits;
        oprog.pop().unwrap().assert_empty();
        ctx.insert(tile, bel, "LVDS", xlat_bitvec(oprog));
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
        ctx.insert(
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
        ctx.insert(
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

        ctx.insert_misc_data("HR_IOSTD:LVDS_T:OFF", bits![0; 13]);
        ctx.insert_misc_data("HR_IOSTD:LVDS_C:OFF", bits![0; 13]);
        ctx.insert_misc_data("HR_IOSTD:DRIVE:OFF", bits![0; 7]);
        ctx.insert_misc_data("HR_IOSTD:OUTPUT_MISC:OFF", bits![0; 3]);
        ctx.insert_misc_data("HR_IOSTD:SLEW:OFF", bits![0; 10]);

        if tile == "IO_HR_PAIR" {
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
                        let mut diff = ctx.get_diff(tile, bel, "OSTD", &val);
                        diff.apply_bitvec_diff(
                            ctx.item(tile, bel, "OUTPUT_ENABLE"),
                            &bits![1; 2],
                            &bits![0; 2],
                        );
                        let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                        let drive_item = ctx.item(tile, bel, "DRIVE");
                        let TileItemKind::BitVec { ref invert } = drive_item.kind else {
                            unreachable!()
                        };
                        let value: BitVec = drive_item
                            .bits
                            .iter()
                            .zip(invert.iter())
                            .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                Some(val) => {
                                    assert_eq!(val, !inv);
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
                        ctx.insert_misc_data(format!("HR_IOSTD:DRIVE:{name}"), value);
                        let slew_item = ctx.item(tile, bel, "SLEW");
                        let TileItemKind::BitVec { ref invert } = slew_item.kind else {
                            unreachable!()
                        };
                        let value: BitVec = slew_item
                            .bits
                            .iter()
                            .zip(invert.iter())
                            .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                Some(val) => {
                                    assert_eq!(val, !inv);
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
                        ctx.insert_misc_data(format!("HR_IOSTD:SLEW:{name}"), value);
                        let val = extract_bitvec_val(
                            ctx.item(tile, bel, "OUTPUT_MISC"),
                            &bits![0; 3],
                            diff,
                        );
                        ctx.insert_misc_data(format!("HR_IOSTD:OUTPUT_MISC:{stdname}"), val);
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
            for bel in ["IOB[0]", "IOB[1]"] {
                for lp in ["HP", "LP"] {
                    let mut diff =
                        ctx.get_diff(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                    for cbel in ["IOB[0]", "IOB[1]"] {
                        diff.apply_enum_diff(
                            ctx.item(tile, cbel, "IBUF_MODE"),
                            if std.name == "TMDS_33" {
                                if lp == "LP" { "TMDS_LP" } else { "TMDS_HP" }
                            } else {
                                if lp == "LP" { "DIFF_LP" } else { "DIFF_HP" }
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
                    let mut diff = ctx.get_diff(tile, "IOB[1]", "OSTD", &val);
                    for bel in ["IOB[0]", "IOB[1]"] {
                        diff.apply_bitvec_diff(
                            ctx.item(tile, bel, "OUTPUT_ENABLE"),
                            &bits![1; 2],
                            &bits![0; 2],
                        );
                        let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                        let drive_item = ctx.item(tile, bel, "DRIVE");
                        let TileItemKind::BitVec { ref invert } = drive_item.kind else {
                            unreachable!()
                        };
                        let value: BitVec = drive_item
                            .bits
                            .iter()
                            .zip(invert.iter())
                            .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                Some(val) => {
                                    assert_eq!(val, !inv);
                                    true
                                }
                                None => false,
                            })
                            .collect();
                        ctx.insert_misc_data(format!("HR_IOSTD:DRIVE:{stdname}"), value);
                        let slew_item = ctx.item(tile, bel, "SLEW");
                        let TileItemKind::BitVec { ref invert } = slew_item.kind else {
                            unreachable!()
                        };
                        let value: BitVec = slew_item
                            .bits
                            .iter()
                            .zip(invert.iter())
                            .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                Some(val) => {
                                    assert_eq!(val, !inv);
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
                        ctx.insert_misc_data(format!("HR_IOSTD:SLEW:{name}"), value);
                        let val = extract_bitvec_val_part(
                            ctx.item(tile, bel, "OUTPUT_MISC"),
                            &bits![0; 3],
                            &mut diff,
                        );
                        ctx.insert_misc_data(format!("HR_IOSTD:OUTPUT_MISC:{stdname}"), val);
                    }
                    diff.apply_enum_diff(ctx.item(tile, "IOB[0]", "OMUX"), "OTHER_O_INV", "O");
                    diff.assert_empty();
                }
            } else {
                if std.name != "TMDS_33" {
                    let mut diff = ctx.get_diff(tile, "IOB[0]", "DIFF_TERM", std.name);
                    let val_c = extract_bitvec_val_part(
                        ctx.item(tile, "IOB[0]", "LVDS"),
                        &bits![0; 13],
                        &mut diff,
                    );
                    let val_t = extract_bitvec_val_part(
                        ctx.item(tile, "IOB[1]", "LVDS"),
                        &bits![0; 13],
                        &mut diff,
                    );
                    ctx.insert_misc_data(format!("HR_IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                    ctx.insert_misc_data(format!("HR_IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                    diff.assert_empty();

                    let mut diff = ctx.get_diff(tile, "IOB[0]", "DIFF_TERM_DYNAMIC", std.name);
                    let val_c = extract_bitvec_val_part(
                        ctx.item(tile, "IOB[0]", "LVDS"),
                        &bits![0; 13],
                        &mut diff,
                    );
                    let val_t = extract_bitvec_val_part(
                        ctx.item(tile, "IOB[1]", "LVDS"),
                        &bits![0; 13],
                        &mut diff,
                    );
                    ctx.insert_misc_data(format!("HR_IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                    ctx.insert_misc_data(format!("HR_IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                    diff.assert_empty();
                }

                let mut diff = ctx.get_diff(tile, "IOB[1]", "OSTD", std.name);
                if std.name != "TMDS_33" {
                    let mut altdiff = ctx
                        .get_diff(tile, "IOB[1]", "OSTD", format!("{}.ALT", std.name))
                        .combine(&!&diff);
                    let diff1 = altdiff.split_bits_by(|bit| bit.rect.to_idx() == 1);
                    ctx.insert(tile, "IOB[0]", "LVDS_GROUP", xlat_bit(altdiff));
                    ctx.insert(tile, "IOB[1]", "LVDS_GROUP", xlat_bit(diff1));
                }
                let val_c = extract_bitvec_val_part(
                    ctx.item(tile, "IOB[0]", "LVDS"),
                    &bits![0; 13],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.item(tile, "IOB[1]", "LVDS"),
                    &bits![0; 13],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("HR_IOSTD:LVDS_T:OUTPUT_{}", std.name), val_t);
                ctx.insert_misc_data(format!("HR_IOSTD:LVDS_C:OUTPUT_{}", std.name), val_c);
                diff.apply_bitvec_diff(
                    ctx.item(tile, "IOB[1]", "OUTPUT_ENABLE"),
                    &bits![1; 2],
                    &bits![0; 2],
                );
                diff.assert_empty();
            }
        }
        ctx.collect_bit("IO_HR_N", "IOB[0]", "LOW_VOLTAGE", "1");
        // meh.
        let _ = ctx.get_diff("IO_HR_S", "IOB[0]", "OSTD", "LVCMOS18.4.SLOW.EXCL");
        let _ = ctx.get_diff("IO_HR_PAIR", "IOB_COMMON", "LOW_VOLTAGE", "1");
    }

    if ctx.has_tile("HCLK_IO_HR") {
        let tile = "HCLK_IO_HR";
        {
            let bel = "VCCOSENSE";
            ctx.collect_bit(tile, bel, "FLAG", "ENABLE");
            ctx.collect_enum(tile, bel, "MODE", &["OFF", "ALWAYSACTIVE", "FREEZE"]);
        }
        {
            let bel = "INTERNAL_VREF";
            let mut diffs = vec![("OFF", Diff::default())];
            for val in ["600", "675", "750", "900"] {
                diffs.push((val, ctx.get_diff(tile, bel, "VREF", val)));
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
            ctx.insert(tile, bel, "VREF", xlat_enum(diffs));
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
                ctx.insert_misc_data(format!("HR_IOSTD:DRIVERBIAS:{val}"), bits![0; 16]);
            }
            let diff = ctx.get_diff(tile, bel, "DRIVERBIAS", "LV");
            let lv = extract_bitvec_val(&item, &bits![0; 16], diff);
            for val in ["1800", "1500", "1350", "1200"] {
                ctx.insert_misc_data(format!("HR_IOSTD:DRIVERBIAS:{val}"), lv.clone());
            }
            ctx.insert(tile, bel, "DRIVERBIAS", item);
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
                let mut diff = ctx.get_diff(tile, bel, "STD0", std.name);
                let vc = extract_bitvec_val_part(&common, &bits![0; 9], &mut diff);
                let val = extract_bitvec_val(&group0, &bits![0; 16], diff);
                ctx.insert_misc_data(format!("HR_IOSTD:LVDSBIAS:COMMON:{}", std.name), vc);
                ctx.insert_misc_data(format!("HR_IOSTD:LVDSBIAS:GROUP:{}", std.name), val);
                if std.name != "TMDS_33" {
                    let diff = ctx.get_diff(tile, bel, "STD1", std.name);
                    let val = extract_bitvec_val(&group1, &bits![0; 16], diff);
                    ctx.insert_misc_data(format!("HR_IOSTD:LVDSBIAS:GROUP:{}", std.name), val);
                }
            }
            ctx.insert_misc_data("HR_IOSTD:LVDSBIAS:COMMON:OFF", bits![0; 9]);
            ctx.insert_misc_data("HR_IOSTD:LVDSBIAS:GROUP:OFF", bits![0; 16]);

            ctx.insert(tile, bel, "COMMON", common);
            ctx.insert(tile, bel, "GROUP0", group0);
            ctx.insert(tile, bel, "GROUP1", group1);
        }
    }
}
