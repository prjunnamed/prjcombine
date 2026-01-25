use prjcombine_entity::EntityId;
use prjcombine_interconnect::grid::{CellCoord, DieId, TileCoord};
use prjcombine_re_collector::{
    diff::{Diff, DiffKey, FeatureId, OcdMode},
    legacy::{
        extract_bitvec_val_legacy, extract_bitvec_val_part_legacy, xlat_bit_bi_legacy,
        xlat_bit_legacy, xlat_bit_wide_legacy, xlat_bitvec_legacy, xlat_enum_legacy,
        xlat_enum_legacy_ocd,
    },
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::defs;

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        iostd::{DciKind, DiffKind, Iostd},
        props::DynProp,
    },
    virtex4::io::IsBonded,
    virtex5::io::{DiffOut, HclkIoi, VrefInternal},
};

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

fn get_vrefs(backend: &IseBackend, tcrd: TileCoord) -> Vec<TileCoord> {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let chip = edev.chips[tcrd.die];
    let reg = chip.row_to_reg(tcrd.row);
    let bot = chip.row_reg_bot(reg);
    [bot + 10, bot + 30]
        .into_iter()
        .map(|vref_row| tcrd.with_row(vref_row).tile(defs::tslots::BEL))
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Vref;

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
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: "IO".into(),
                    bel: "IOB[0]".into(),
                    attr: "PRESENT".into(),
                    val: "VREF".into(),
                }),
                rects: backend.edev.tile_bits(vref),
            });
        }
        Some((fuzzer, false))
    }
}

fn get_vr(backend: &IseBackend, tcrd: TileCoord) -> TileCoord {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let chip = edev.chips[tcrd.die];

    let reg = chip.row_to_reg(tcrd.row);
    let row = if reg == chip.reg_cfg {
        chip.row_reg_bot(reg) + 6
    } else if reg == chip.reg_cfg - 1 && Some(tcrd.col) == edev.col_lcio {
        chip.row_reg_bot(reg) + 4
    } else if reg == chip.reg_cfg - 1 && Some(tcrd.col) == edev.col_rcio {
        chip.row_reg_bot(reg) + 0
    } else {
        chip.row_reg_bot(reg) + 14
    };
    tcrd.with_row(row).tile(defs::tslots::BEL)
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

        // Avoid bank 25, which is our (arbitrary) anchor.
        if tcrd.col == edev.col_lcio.unwrap() && chip.row_to_reg(tcrd.row) == chip.reg_cfg {
            return None;
        }

        let vr_tile = get_vr(backend, tcrd);
        if tcrd == vr_tile {
            // Not in VR tile please.
            return None;
        }
        // Ensure nothing is placed in VR.
        for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
            let site = backend.ngrid.get_bel_name(vr_tile.cell.bel(bel)).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Test VR.
        if self.0.is_some() {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: "IO".into(),
                    bel: "IOB_COMMON".into(),
                    attr: "PRESENT".into(),
                    val: "VR".into(),
                }),
                rects: edev.tile_bits(vr_tile),
            });
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
                    tile: "HCLK_IO".into(),
                    bel: "DCI".into(),
                    attr: "STD".into(),
                    val: std.into(),
                }),
                rects: edev.tile_bits(hclk_ioi),
            });
        }
        // Take shared mutex on global DCI.
        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");

        // Anchor global DCI by putting something in bottom IOB of bank 25.
        let iob_center = tcrd
            .cell
            .with_cr(edev.col_lcio.unwrap(), chip.row_bufg())
            .bel(defs::bslots::IOB[0]);
        let site = backend.ngrid.get_bel_name(iob_center).unwrap();
        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_25");
        // Ensure anchor VR IOBs are free.
        for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
            let iob_center_vr = tcrd
                .cell
                .with_cr(edev.col_lcio.unwrap(), chip.row_bufg() + 6)
                .bel(bel);
            let site = backend.ngrid.get_bel_name(iob_center_vr).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Make note of anchor VCCO.
        let hclk_ioi_center = tcrd
            .cell
            .with_cr(edev.col_lcio.unwrap(), chip.row_bufg() + 20)
            .tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.base(Key::TileMutex(hclk_ioi_center, "VCCO".to_string()), "2500");

        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
    devdata_only: bool,
) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    let mut ctx = FuzzCtx::new(session, backend, "IO");
    if devdata_only {
        for i in 0..2 {
            let mut bctx = ctx.bel(defs::bslots::IODELAY[i]);
            let bel_other = defs::bslots::IODELAY[i ^ 1];
            bctx.build()
                .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
                .bel_mode(bel_other, "IODELAYE1")
                .bel_attr(bel_other, "IDELAY_TYPE", "DEFAULT")
                .bel_attr(bel_other, "DELAY_SRC", "I")
                .test_manual("MODE", "I_DEFAULT")
                .mode("IODELAYE1")
                .attr("IDELAY_TYPE", "DEFAULT")
                .attr("DELAY_SRC", "I")
                .commit();
        }
        return;
    }
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
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::ILOGIC[i]);

        bctx.test_manual("PRESENT", "ILOGIC")
            .mode("ILOGICE1")
            .commit();
        bctx.test_manual("PRESENT", "ISERDES")
            .mode("ISERDESE1")
            .commit();

        bctx.mode("ISERDESE1").test_inv("D");
        bctx.mode("ISERDESE1").test_inv("CLK");
        bctx.mode("ISERDESE1")
            .attr("DYN_CLKDIV_INV_EN", "FALSE")
            .test_inv("CLKDIV");
        bctx.mode("ISERDESE1")
            .test_enum("DYN_CLK_INV_EN", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum("DYN_OCLK_INV_EN", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum("DYN_CLKDIV_INV_EN", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .attr("OVERSAMPLE", "FALSE")
            .attr("DYN_OCLK_INV_EN", "FALSE")
            .attr("INTERFACE_TYPE", "")
            .pin("OCLK")
            .test_enum_suffix("OCLKINV", "SDR", &["OCLK", "OCLK_B"]);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "DDR")
            .attr("OVERSAMPLE", "FALSE")
            .attr("DYN_OCLK_INV_EN", "FALSE")
            .attr("INTERFACE_TYPE", "")
            .pin("OCLK")
            .test_enum_suffix("OCLKINV", "DDR", &["OCLK", "OCLK_B"]);

        bctx.mode("ILOGICE1")
            .attr("IFFTYPE", "#FF")
            .pin("SR")
            .test_enum("SRUSED", &["0"]);
        bctx.mode("ILOGICE1")
            .attr("IFFTYPE", "#FF")
            .pin("REV")
            .test_enum("REVUSED", &["0"]);
        bctx.mode("ISERDESE1")
            .attr("DATA_WIDTH", "2")
            .attr("DATA_RATE", "SDR")
            .test_enum("SERDES", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum("SERDES_MODE", &["MASTER", "SLAVE"]);
        bctx.mode("ISERDESE1")
            .attr("SERDES", "FALSE")
            .test_enum("DATA_WIDTH", &["2", "3", "4", "5", "6", "7", "8", "10"]);
        bctx.mode("ISERDESE1").test_enum("NUM_CE", &["1", "2"]);

        for attr in [
            "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
            "SRVAL_Q4",
        ] {
            bctx.mode("ISERDESE1").test_enum(attr, &["0", "1"]);
        }

        bctx.mode("ILOGICE1")
            .attr("IFFTYPE", "#FF")
            .test_enum_suffix("SRTYPE", "ILOGIC", &["SYNC", "ASYNC"]);
        bctx.mode("ISERDESE1")
            .test_enum_suffix("SRTYPE", "ISERDES", &["SYNC", "ASYNC"]);

        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_CE", 2);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_BITSLIPCNT", 4);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_BITSLIP", 6);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_RANK1_PARTIAL", 5);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_RANK2", 6);
        bctx.mode("ISERDESE1")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_RANK3", 6);

        bctx.mode("ISERDESE1")
            .pin("OFB")
            .test_enum("OFB_USED", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .pin("TFB")
            .test_enum("TFB_USED", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum("IOBDELAY", &["NONE", "IFD", "IBUF", "BOTH"]);

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
            .test_enum("D2OBYP_SEL", &["GND", "T"]);
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
            .test_enum("D2OFFBYP_SEL", &["GND", "T"]);
        bctx.mode("ILOGICE1")
            .attr("IDELMUX", "1")
            .attr("DINV", "")
            .pin("D")
            .pin("DDLY")
            .pin("O")
            .pin("TFB")
            .pin("OFB")
            .test_enum("IMUX", &["0", "1"]);
        bctx.mode("ILOGICE1")
            .attr("IFFDELMUX", "1")
            .attr("IFFTYPE", "#FF")
            .attr("DINV", "")
            .pin("D")
            .pin("DDLY")
            .pin("TFB")
            .pin("OFB")
            .test_enum("IFFMUX", &["0", "1"]);
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
            .test_enum("IDELMUX", &["0", "1"]);
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
            .test_enum("IFFDELMUX", &["0", "1"]);

        bctx.mode("ISERDESE1")
            .test_enum("D_EMU", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1").test_enum(
            "D_EMU_OPTION",
            &["MATCH_DLY0", "MATCH_DLY2", "DLY0", "DLY1", "DLY2", "DLY3"],
        );
        bctx.mode("ISERDESE1")
            .test_enum("RANK12_DLY", &["FALSE", "TRUE"]);
        bctx.mode("ISERDESE1")
            .test_enum("RANK23_DLY", &["FALSE", "TRUE"]);

        bctx.mode("ISERDESE1")
            .attr("OVERSAMPLE", "FALSE")
            .test_enum(
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
            .test_enum("DATA_RATE", &["SDR", "DDR"]);
        bctx.mode("ISERDESE1").test_enum(
            "DDR_CLK_EDGE",
            &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
        );
        bctx.mode("ILOGICE1").attr("IFFTYPE", "DDR").test_enum(
            "DDR_CLK_EDGE",
            &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
        );
        bctx.mode("ILOGICE1")
            .test_enum("IFFTYPE", &["#FF", "#LATCH", "DDR"]);

        for (src, num) in [("HCLK", 12), ("RCLK", 6), ("IOCLK", 8)] {
            for j in 0..num {
                bctx.build()
                    .mutex("MUX.CLK", format!("{src}{j}"))
                    .pip("CLKB", (defs::bslots::IOI, format!("{src}{j}")))
                    .test_manual("MUX.CLK", format!("{src}{j}"))
                    .pip("CLK", (defs::bslots::IOI, format!("{src}{j}")))
                    .commit();
                bctx.build()
                    .mutex("MUX.CLK", format!("{src}{j}"))
                    .test_manual("MUX.CLKB", format!("{src}{j}"))
                    .pip("CLKB", (defs::bslots::IOI, format!("{src}{j}")))
                    .commit();
            }
        }
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::OLOGIC[i]);

        bctx.test_manual("PRESENT", "OLOGIC")
            .mode("OLOGICE1")
            .commit();
        bctx.test_manual("PRESENT", "OSERDES")
            .mode("OSERDESE1")
            .commit();

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "T2", "T3", "T4", "CLKDIV", "CLKPERF",
        ] {
            bctx.mode("OSERDESE1").test_inv(pin);
        }
        bctx.mode("OLOGICE1")
            .attr("TMUX", "T1")
            .attr("T1USED", "0")
            .pin("TQ")
            .test_inv("T1");
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("DDR_CLK_EDGE", "SAME_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix("CLKINV", "SAME", &["CLK", "CLK_B"]);
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("DDR_CLK_EDGE", "OPPOSITE_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix("CLKINV", "OPPOSITE", &["CLK", "CLK_B"]);

        bctx.mode("OLOGICE1")
            .attr("OUTFFTYPE", "#FF")
            .test_enum("SRTYPE_OQ", &["SYNC", "ASYNC"]);
        bctx.mode("OLOGICE1")
            .attr("TFFTYPE", "#FF")
            .test_enum("SRTYPE_TQ", &["SYNC", "ASYNC"]);
        bctx.mode("OSERDESE1")
            .test_enum("SRTYPE", &["SYNC", "ASYNC"]);

        bctx.mode("OLOGICE1")
            .test_enum_suffix("INIT_OQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OLOGICE1")
            .test_enum_suffix("INIT_TQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_suffix("INIT_OQ", "OSERDES", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_suffix("INIT_TQ", "OSERDES", &["0", "1"]);
        bctx.mode("OLOGICE1")
            .test_enum_suffix("SRVAL_OQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OLOGICE1")
            .test_enum_suffix("SRVAL_TQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_suffix("SRVAL_OQ", "OSERDES", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_enum_suffix("SRVAL_TQ", "OSERDES", &["0", "1"]);

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
                .test_enum(attr, &["0"]);
        }

        bctx.mode("OLOGICE1")
            .attr("TFFTYPE", "")
            .pin("OQ")
            .test_enum("OUTFFTYPE", &["#FF", "#LATCH", "DDR"]);
        bctx.mode("OLOGICE1")
            .attr("OUTFFTYPE", "")
            .pin("TQ")
            .test_enum("TFFTYPE", &["#FF", "#LATCH", "DDR"]);

        bctx.mode("OSERDESE1")
            .test_enum("DATA_RATE_OQ", &["SDR", "DDR"]);
        bctx.mode("OSERDESE1")
            .attr("T1INV", "T1")
            .pin("T1")
            .test_enum("DATA_RATE_TQ", &["BUF", "SDR", "DDR"]);

        bctx.mode("OLOGICE1")
            .global("ENABLEMISR", "Y")
            .test_enum("MISR_ENABLE", &["FALSE", "TRUE"]);
        bctx.mode("OLOGICE1")
            .global("ENABLEMISR", "Y")
            .test_enum("MISR_ENABLE_FDBK", &["FALSE", "TRUE"]);
        bctx.mode("OLOGICE1")
            .global("ENABLEMISR", "Y")
            .test_enum("MISR_CLK_SELECT", &["CLK1", "CLK2"]);

        bctx.mode("OSERDESE1")
            .test_enum("SERDES", &["FALSE", "TRUE"]);
        bctx.mode("OSERDESE1")
            .test_enum("SERDES_MODE", &["SLAVE", "MASTER"]);
        bctx.mode("OSERDESE1")
            .test_enum("SELFHEAL", &["FALSE", "TRUE"]);
        bctx.mode("OSERDESE1")
            .test_enum("INTERFACE_TYPE", &["DEFAULT", "MEMORY_DDR3"]);
        bctx.mode("OSERDESE1")
            .test_enum("TRISTATE_WIDTH", &["1", "4"]);
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "SDR")
            .attr("INTERFACE_TYPE", "DEFAULT")
            .test_enum_suffix("DATA_WIDTH", "SDR", &["2", "3", "4", "5", "6", "7", "8"]);
        bctx.mode("OSERDESE1")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("INTERFACE_TYPE", "DEFAULT")
            .test_enum_suffix("DATA_WIDTH", "DDR", &["4", "6", "8", "10"]);
        bctx.mode("OSERDESE1").test_enum("WC_DELAY", &["0", "1"]);
        bctx.mode("OSERDESE1").test_enum("DDR3_DATA", &["0", "1"]);
        bctx.mode("OSERDESE1").test_enum("ODELAY_USED", &["0", "1"]);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin("INIT_LOADCNT", 4);
        bctx.mode("OSERDESE1").test_multi_attr_bin("INIT_ORANK1", 6);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin("INIT_ORANK2_PARTIAL", 4);
        bctx.mode("OSERDESE1").test_multi_attr_bin("INIT_TRANK1", 4);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin("INIT_FIFO_ADDR", 11);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin("INIT_FIFO_RESET", 13);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin("INIT_DLY_CNT", 10);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin("INIT_PIPE_DATA0", 12);
        bctx.mode("OSERDESE1")
            .test_multi_attr_bin("INIT_PIPE_DATA1", 12);

        for (src, num) in [("HCLK", 12), ("RCLK", 6)] {
            for j in 0..num {
                bctx.build()
                    .mutex("MUX.CLKDIV", format!("{src}{j}"))
                    .pip("CLKDIVB", (defs::bslots::IOI, format!("{src}{j}")))
                    .test_manual("MUX.CLKDIV", format!("{src}{j}"))
                    .pip("CLKDIV", (defs::bslots::IOI, format!("{src}{j}")))
                    .commit();
                bctx.build()
                    .mutex("MUX.CLKDIV", format!("{src}{j}"))
                    .test_manual("MUX.CLKDIVB", format!("{src}{j}"))
                    .pip("CLKDIVB", (defs::bslots::IOI, format!("{src}{j}")))
                    .commit();
            }
        }
        for (src, num) in [("HCLK", 12), ("RCLK", 6), ("IOCLK", 8)] {
            for j in 0..num {
                bctx.build()
                    .mutex("MUX.CLK", format!("{src}{j}"))
                    .pip("CLKM", (defs::bslots::IOI, format!("{src}{j}")))
                    .test_manual("MUX.CLK", format!("{src}{j}"))
                    .pip("CLK_MUX", (defs::bslots::IOI, format!("{src}{j}")))
                    .commit();
                bctx.build()
                    .mutex("MUX.CLK", format!("{src}{j}"))
                    .test_manual("MUX.CLKB", format!("{src}{j}"))
                    .pip("CLKM", (defs::bslots::IOI, format!("{src}{j}")))
                    .commit();
            }
        }
        for j in 0..2 {
            bctx.build()
                .mutex("MUX.CLKPERF", format!("OCLK{j}"))
                .test_manual("MUX.CLKPERF", format!("OCLK{j}"))
                .pip("CLKPERF", (defs::bslots::IOI, format!("OCLK{j}")))
                .commit();
        }
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::IODELAY[i]);
        let bel_other = defs::bslots::IODELAY[i ^ 1];

        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .test_manual("PRESENT", "1")
            .mode("IODELAYE1")
            .commit();
        for pin in ["C", "DATAIN", "IDATAIN"] {
            bctx.mode("IODELAYE1")
                .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
                .test_inv(pin);
        }
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .test_enum("CINVCTRL_SEL", &["FALSE", "TRUE"]);
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .test_enum("HIGH_PERFORMANCE_MODE", &["FALSE", "TRUE"]);
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .test_enum("DELAY_SRC", &["I", "O", "IO", "DATAIN", "CLKIN"]);
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .test_manual("DELAY_SRC", "DELAYCHAIN_OSC")
            .attr("DELAY_SRC", "I")
            .attr("DELAYCHAIN_OSC", "TRUE")
            .commit();
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .attr("DELAY_SRC", "IO")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .test_multi_attr_dec("IDELAY_VALUE", 5);
        bctx.mode("IODELAYE1")
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .attr("DELAY_SRC", "IO")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("ODELAY_TYPE", "FIXED")
            .test_multi_attr_dec("ODELAY_VALUE", 5);
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "DEFAULT")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual("MODE", "I_DEFAULT")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "DEFAULT")
            .attr("DELAY_SRC", "I")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual("MODE", "I_FIXED")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "FIXED")
            .attr("DELAY_SRC", "I")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "VARIABLE")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual("MODE", "I_VARIABLE")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "VARIABLE")
            .attr("DELAY_SRC", "I")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "VAR_LOADABLE")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual("MODE", "I_VAR_LOADABLE")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "VAR_LOADABLE")
            .attr("DELAY_SRC", "I")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual("MODE", "O_FIXED")
            .mode("IODELAYE1")
            .attr("ODELAY_TYPE", "FIXED")
            .attr("DELAY_SRC", "O")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual("MODE", "O_VARIABLE")
            .mode("IODELAYE1")
            .attr("ODELAY_TYPE", "VARIABLE")
            .attr("DELAY_SRC", "O")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual("MODE", "O_VAR_LOADABLE")
            .mode("IODELAYE1")
            .attr("ODELAY_TYPE", "VAR_LOADABLE")
            .attr("DELAY_SRC", "O")
            .commit();
        bctx.build()
            .related_tile_mutex(HclkIoi, "IDELAYCTRL", "USE")
            .bel_mode(bel_other, "IODELAYE1")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .bel_attr(bel_other, "DELAY_SRC", "I")
            .test_manual("MODE", "IO_FIXED")
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
            .test_manual("MODE", "I_VARIABLE_O_FIXED")
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
            .test_manual("MODE", "I_FIXED_O_VARIABLE")
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
            .test_manual("MODE", "IO_VAR_LOADABLE")
            .mode("IODELAYE1")
            .attr("IDELAY_TYPE", "VAR_LOADABLE")
            .attr("ODELAY_TYPE", "VAR_LOADABLE")
            .attr("DELAY_SRC", "IO")
            .commit();
    }
    for i in 0..2 {
        let bel = defs::bslots::IOB[i];
        let mut bctx = ctx.bel(bel);
        let bel_ologic = defs::bslots::OLOGIC[i];
        let bel_other_ologic = defs::bslots::OLOGIC[i ^ 1];
        let bel_iodelay = defs::bslots::IODELAY[i];
        let bel_other = defs::bslots::IOB[i ^ 1];
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_manual("PRESENT", "IOB")
            .mode("IOB")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "CONTINUOUS")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_manual("PRESENT", "IOB.CONTINUOUS")
            .mode("IOB")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_manual("PRESENT", "IPAD")
            .mode("IPAD")
            .commit();
        bctx.mode("IOB")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_enum("PULL", &["KEEPER", "PULLDOWN", "PULLUP"]);
        for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
            bctx.mode("IOB")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .mutex("PULL_DYNAMIC", pin)
                .test_manual("PULL_DYNAMIC", "1")
                .pin_pips(pin)
                .commit();
        }
        bctx.mode("IOB")
            .related_tile_mutex(HclkIoi, "VCCO", "1800")
            .pin("O")
            .attr("OUSED", "0")
            .attr("OSTANDARD", "LVCMOS18")
            .attr("DRIVE", "12")
            .attr("SLEW", "SLOW")
            .test_multi_attr_bin("OPROGRAMMING", 31);
        for &std in IOSTDS {
            let mut vref_special = None;
            let mut dci_special = None;
            let mut dci_special_lite = None;
            if std.vref.is_some() {
                vref_special = Some(Vref);
            }
            if matches!(
                std.dci,
                DciKind::BiSplit
                    | DciKind::BiSplitT
                    | DciKind::BiVcc
                    | DciKind::InputSplit
                    | DciKind::InputVcc
            ) {
                dci_special = Some(Dci(Some(std.name)));
                dci_special_lite = Some(Dci(None));
            }
            if std.diff != DiffKind::None {
                for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                    bctx.mode("IOB")
                        .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
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
                if std.diff == DiffKind::True && i == 0 {
                    bctx.mode("IOB")
                        .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
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
                    bctx.mode("IOB")
                        .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
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
            } else {
                for (suffix, lp) in [("LP", "TRUE"), ("HP", "FALSE")] {
                    bctx.mode("IOB")
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
        for &std in IOSTDS {
            let mut dci_special = None;
            if matches!(
                std.dci,
                DciKind::Output | DciKind::OutputHalf | DciKind::BiSplit | DciKind::BiVcc
            ) {
                dci_special = Some(Dci(Some(std.name)));
            }
            if std.diff == DiffKind::True {
                if i == 1 {
                    bctx.build()
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
                        .mode_diff("IOB", "IOBM")
                        .pin("O")
                        .attr("OUSED", "0")
                        .attr("DIFFO_OUTUSED", "0")
                        .attr("OSTANDARD", std.name)
                        .bel_mode_diff(bel_other, "IOB", "IOBS")
                        .bel_attr(bel_other, "OUTMUX", "1")
                        .bel_attr(bel_other, "DIFFO_INUSED", "0")
                        .pin_pair("DIFFO_OUT", bel_other, "DIFFO_IN")
                        .commit();
                }
            } else if std.diff != DiffKind::None {
                if i == 1 {
                    bctx.build()
                        .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                        .attr("IUSED", "")
                        .attr("OPROGRAMMING", "")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .maybe_prop(dci_special)
                        .bel_attr(bel_other, "IUSED", "")
                        .bel_attr(bel_other, "OPROGRAMMING", "")
                        .bel_mode(bel_other_ologic, "OLOGICE1")
                        .test_manual("OSTD", std.name)
                        .mode_diff("IOB", "IOBM")
                        .pin("O")
                        .attr("OUSED", "0")
                        .attr("O_OUTUSED", "0")
                        .attr("OSTANDARD", std.name)
                        .bel_mode_diff(bel_other, "IOB", "IOBS")
                        .bel_attr(bel_other, "OUTMUX", "0")
                        .bel_attr(bel_other, "OINMUX", "1")
                        .bel_attr(bel_other, "OSTANDARD", std.name)
                        .pin_pair("O_OUT", bel_other, "O_IN")
                        .commit();
                }
            } else if !std.drive.is_empty() {
                for &drive in std.drive {
                    for slew in ["SLOW", "FAST"] {
                        bctx.mode("IOB")
                            .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                            .pin("O")
                            .attr("IUSED", "")
                            .attr("OPROGRAMMING", "")
                            .test_manual("OSTD", format!("{name}.{drive}.{slew}", name = std.name))
                            .attr("OUSED", "0")
                            .attr("OSTANDARD", std.name)
                            .attr("DRIVE", drive)
                            .attr("SLEW", slew)
                            .commit();
                    }
                }
            } else {
                bctx.mode("IOB")
                    .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                    .pin("O")
                    .attr("IUSED", "")
                    .attr("OPROGRAMMING", "")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .maybe_prop(dci_special)
                    .test_manual("OSTD", std.name)
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", std.name)
                    .commit();
            }
        }

        for (std, vcco, vref) in [
            ("HSTL_I_12", 1200, 600),
            ("HSTL_I", 1500, 750),
            ("HSTL_III", 1500, 900),
            ("HSTL_III_18", 1800, 1100),
            ("SSTL2_I", 2500, 1250),
        ] {
            bctx.mode("IOB")
                .related_tile_mutex(HclkIoi, "VCCO", vcco.to_string())
                .attr("OUSED", "")
                .pin("I")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .prop(VrefInternal("HCLK_IO", vref))
                .test_manual("ISTD", format!("{std}.LP"))
                .attr("IUSED", "0")
                .attr("ISTANDARD", std)
                .attr("IBUF_LOW_PWR", "TRUE")
                .commit();
        }

        bctx.build()
            .mutex("OUTPUT_DELAY", "0")
            .bel_mode(bel_ologic, "OLOGICE1")
            .test_manual("OUTPUT_DELAY", "0")
            .pip((bel_ologic, "IOB_O"), (bel_ologic, "OQ"))
            .commit();
        bctx.build()
            .mutex("OUTPUT_DELAY", "1")
            .bel_mode(bel_ologic, "OLOGICE1")
            .test_manual("OUTPUT_DELAY", "1")
            .pip((bel_ologic, "IOB_O"), (bel_iodelay, "DATAOUT"))
            .commit();
    }
    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        ctx.build()
            .extra_tiles_by_bel(defs::bslots::OLOGIC[0], "OLOGIC_COMMON")
            .global("ENABLEMISR", "Y")
            .test_manual("OLOGIC_COMMON", "MISR_RESET", "1")
            .global_diff("MISRRESET", "N", "Y")
            .commit();
    }
    {
        let mut ctx = FuzzCtx::new(session, backend, "HCLK_IO");
        let mut bctx = ctx.bel(defs::bslots::DCI);
        bctx.build()
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_manual("TEST_ENABLE", "1")
            .mode("DCI")
            .commit();
        bctx.build()
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_manual("DYNAMIC_ENABLE", "1")
            .mode("DCI")
            .pin_pips("INT_DCI_EN")
            .commit();
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .extra_tiles_by_bel(defs::bslots::DCI, "DCI")
        .test_manual("DCI", "QUIET", "1")
        .global_diff("DCIUPDATEMODE", "CONTINUOUS", "QUIET")
        .commit();
    {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        let mut builder = ctx
            .build()
            .raw(Key::Package, &package.name)
            .extra_tile_attr_fixed(edev.tile_cfg(die), "MISC", "DCI_CLK_ENABLE", "1");

        // Find VR and IO rows.
        let vr_tile = CellCoord::new(die, edev.col_lcio.unwrap(), chip.row_bufg() + 6)
            .tile(defs::tslots::BEL);
        let io_tile =
            CellCoord::new(die, edev.col_lcio.unwrap(), chip.row_bufg()).tile(defs::tslots::BEL);
        let io_bel = io_tile.cell.bel(defs::bslots::IOB[0]);
        let hclk_row = chip.row_hclk(io_tile.cell.row);
        let hclk_tcrd =
            CellCoord::new(die, edev.col_lcio.unwrap(), hclk_row).tile(defs::tslots::HCLK_BEL);

        // Ensure nothing is placed in VR.
        for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
            let site = backend.ngrid.get_bel_name(vr_tile.cell.bel(bel)).unwrap();
            builder = builder.raw(Key::SiteMode(site), None);
        }
        builder = builder.extra_tile_attr_fixed(vr_tile, "IOB_COMMON", "PRESENT", "VR");

        // Set up hclk.
        builder = builder.extra_tile_attr_fixed(hclk_tcrd, "DCI", "ENABLE", "1");

        // Set up the IO and fire.
        let site = backend.ngrid.get_bel_name(io_bel).unwrap();
        builder
            .raw(Key::SiteMode(site), "IOB")
            .raw(Key::SitePin(site, "O".into()), true)
            .raw(Key::SiteAttr(site, "IUSED".into()), None)
            .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
            .raw_diff(Key::SiteAttr(site, "OUSED".into()), None, "0")
            .raw_diff(Key::SiteAttr(site, "OSTANDARD".into()), None, "LVDCI_25")
            // Make note of anchor VCCO.
            .raw(Key::TileMutex(hclk_tcrd, "VCCO".to_string()), "2500")
            // Take exclusive mutex on global DCI.
            .raw_diff(Key::GlobalMutex("GLOBAL_DCI".into()), None, "EXCLUSIVE")
            // Avoid interference.
            .raw(Key::GlobalOpt("MATCH_CYCLE".into()), "NOWAIT")
            .extra_tile_attr_fixed(io_tile, "IOB[0]", "OSTD", "LVDCI_25")
            .test_manual("NULL", "CENTER_DCI", "1")
            .commit();
    }
    for bank_to in [24, 26] {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        let mut builder = ctx.build().raw(Key::Package, &package.name);

        let io_tile_from =
            CellCoord::new(die, edev.col_lcio.unwrap(), chip.row_bufg()).tile(defs::tslots::BEL);
        let io_bel_from = io_tile_from.cell.bel(defs::bslots::IOB[0]);
        let io_row_to = match bank_to {
            24 => edev.chips[die].row_bufg() - 40,
            26 => edev.chips[die].row_bufg() + 40,
            _ => unreachable!(),
        };
        let io_tile_to =
            CellCoord::new(die, edev.col_lcio.unwrap(), io_row_to).tile(defs::tslots::BEL);
        let io_bel_to = io_tile_to.cell.bel(defs::bslots::IOB[0]);
        let hclk_row_to = chip.row_hclk(io_row_to);
        let hclk_tile_to =
            CellCoord::new(die, edev.col_lcio.unwrap(), hclk_row_to).tile(defs::tslots::HCLK_BEL);

        // Ensure nothing else in the bank.
        let bot = chip.row_reg_bot(chip.row_to_reg(io_tile_from.cell.row));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
                if row == io_tile_from.cell.row && bel == defs::bslots::IOB[0] {
                    continue;
                }
                if let Some(site) = backend
                    .ngrid
                    .get_bel_name(io_tile_from.cell.with_row(row).bel(bel))
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
            .raw(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_25")
            // Take shared mutex on global DCI.
            .raw(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");

        // Ensure nothing else in the bank.
        let bot = chip.row_reg_bot(chip.row_to_reg(io_tile_to.cell.row));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
                if row == io_tile_to.cell.row && bel == defs::bslots::IOB[0] {
                    continue;
                }
                if let Some(site) = backend
                    .ngrid
                    .get_bel_name(io_tile_to.cell.with_row(row).bel(bel))
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
            .raw_diff(
                Key::SiteAttr(site, "OSTANDARD".into()),
                None,
                if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex6 {
                    "LVDCI_25"
                } else {
                    "LVDCI_33"
                },
            )
            .raw_diff(Key::DciCascade(bank_to), None, 25)
            .extra_tile_attr_fixed(io_tile_to, "IOB[0]", "OSTD", "LVDCI_25")
            .extra_tile_attr_fixed(
                hclk_tile_to,
                "DCI",
                if bank_to == 24 {
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

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let tile = "IO";
    if devdata_only {
        for i in 0..2 {
            let bel = &format!("IODELAY[{i}]");
            let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_DEFAULT");
            let val = extract_bitvec_val_part_legacy(
                ctx.item(tile, bel, "IDELAY_VALUE_CUR"),
                &bits![1; 5],
                &mut diff,
            );
            ctx.insert_device_data("IODELAY:DEFAULT_IDELAY_VALUE", val);
            let val = extract_bitvec_val_part_legacy(
                ctx.item(tile, bel, "IDELAY_VALUE_INIT"),
                &bits![0; 5],
                &mut diff,
            );
            ctx.insert_device_data("IODELAY:DEFAULT_IDELAY_VALUE", val);
        }
        return;
    }
    for i in 0..2 {
        let bel = &format!("ILOGIC[{i}]");

        ctx.collect_inv(tile, bel, "D");
        ctx.collect_inv(tile, bel, "CLKDIV");
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.insert(tile, bel, "INV.CLK", item);

        let diff1 = ctx.get_diff_legacy(tile, bel, "OCLKINV.DDR", "OCLK");
        let diff2 = ctx.get_diff_legacy(tile, bel, "OCLKINV.DDR", "OCLK_B");
        ctx.get_diff_legacy(tile, bel, "OCLKINV.SDR", "OCLK_B")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "OCLKINV.SDR", "OCLK");
        diff = diff.combine(&!&diff1);
        diff = diff.combine(&!&diff2);
        diff.assert_empty();
        ctx.insert(tile, bel, "INV.OCLK1", xlat_bit_legacy(!diff1));
        ctx.insert(tile, bel, "INV.OCLK2", xlat_bit_legacy(!diff2));

        ctx.collect_bit_bi_legacy(tile, bel, "DYN_CLK_INV_EN", "FALSE", "TRUE");
        ctx.collect_bit_bi_legacy(tile, bel, "DYN_CLKDIV_INV_EN", "FALSE", "TRUE");
        ctx.collect_bit_wide_bi_legacy(tile, bel, "DYN_OCLK_INV_EN", "FALSE", "TRUE");

        let iff_rev_used = ctx.extract_bit_legacy(tile, bel, "REVUSED", "0");
        ctx.insert(tile, bel, "IFF_REV_USED", iff_rev_used);
        let iff_sr_used = ctx.extract_bit_legacy(tile, bel, "SRUSED", "0");
        ctx.insert(tile, bel, "IFF_SR_USED", iff_sr_used);
        ctx.collect_bit_bi_legacy(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum_legacy(tile, bel, "SERDES_MODE", &["MASTER", "SLAVE"]);
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10"] {
            diffs.push((val, ctx.get_diff_legacy(tile, bel, "DATA_WIDTH", val)));
        }
        let mut bits = xlat_enum_legacy(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.insert(
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
        ctx.insert(tile, bel, "IFF_SR_SYNC", item);
        ctx.get_diff_legacy(tile, bel, "SRTYPE.ISERDES", "ASYNC")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "SRTYPE.ISERDES", "SYNC");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF_SR_SYNC"), true, false);
        ctx.insert(tile, bel, "BITSLIP_SYNC", xlat_bit_legacy(diff));
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
            ctx.insert(tile, bel, attr, item);
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
        ctx.insert(tile, bel, "BITSLIP_ENABLE", xlat_bit_legacy(bitslip_en));
        ctx.insert(
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
            ctx.item(tile, bel, "DDR_CLK_EDGE"),
            "OPPOSITE_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "IFFTYPE", "#FF");
        diff.apply_enum_diff_legacy(
            ctx.item(tile, bel, "DDR_CLK_EDGE"),
            "OPPOSITE_EDGE",
            "SAME_EDGE_PIPELINED",
        );
        ctx.insert(tile, bel, "IFF_LATCH", xlat_bit_legacy(!diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "IFFTYPE", "DDR");
        diff.apply_enum_diff_legacy(
            ctx.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        ctx.insert(tile, bel, "IFF_LATCH", xlat_bit_legacy(!diff));

        let mut diffs = vec![];
        for val in ["SDR", "DDR"] {
            let mut diff = ctx.get_diff_legacy(tile, bel, "DATA_RATE", val);
            diff.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF_SR_USED"), true, false);
            diff.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF_LATCH"), false, true);
            diffs.push((val, diff));
        }
        ctx.insert(tile, bel, "DATA_RATE", xlat_enum_legacy(diffs));

        let item = ctx.extract_enum_legacy(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = ctx.extract_enum_legacy(tile, bel, "D2OFFBYP_SEL", &["GND", "T"]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = xlat_enum_legacy(vec![
            ("T", ctx.get_diff_legacy(tile, bel, "TFB_USED", "TRUE")),
            ("GND", ctx.get_diff_legacy(tile, bel, "TFB_USED", "FALSE")),
        ]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);

        let item = ctx.extract_bit_bi_legacy(tile, bel, "IDELMUX", "1", "0");
        ctx.insert(tile, bel, "I_DELAY_ENABLE", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "IFFDELMUX", "1", "0");
        ctx.insert(tile, bel, "IFF_DELAY_ENABLE", item);

        ctx.get_diff_legacy(tile, bel, "IOBDELAY", "NONE")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "IOBDELAY", "IBUF");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "IOBDELAY", "IFD");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "IOBDELAY", "BOTH");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.assert_empty();

        let item = ctx.extract_bit_bi_legacy(tile, bel, "IMUX", "1", "0");
        ctx.insert(tile, bel, "I_TSBYPASS_ENABLE", item);
        // the fuzzer is slightly fucked to work around some ridiculous ISE bug.
        let _ = ctx.get_diff_legacy(tile, bel, "IFFMUX", "1");
        let item = ctx.extract_bit_legacy(tile, bel, "IFFMUX", "0");
        ctx.insert(tile, bel, "IFF_TSBYPASS_ENABLE", item);
        ctx.get_diff_legacy(tile, bel, "OFB_USED", "FALSE")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "OFB_USED", "TRUE");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF_TSBYPASS_ENABLE"), true, false);
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
        present_iserdes.apply_enum_diff_legacy(ctx.item(tile, bel, "TSBYPASS_MUX"), "GND", "T");
        present_iserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF1_SRVAL"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF2_SRVAL"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF3_SRVAL"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF4_SRVAL"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF1_INIT"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF2_INIT"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF3_INIT"), false, true);
        present_iserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "IFF4_INIT"), false, true);
        present_iserdes.assert_empty();

        ctx.insert(
            tile,
            bel,
            "READBACK_I",
            TileItem::from_bit_inv([TileBit::new(0, 26, 61), TileBit::new(1, 27, 2)][i], false),
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
        ctx.collect_enum_default_legacy_ocd(tile, bel, "MUX.CLK", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_legacy_ocd(tile, bel, "MUX.CLKB", &vals, "NONE", OcdMode::Mux);
    }
    for i in 0..2 {
        let bel = &format!("OLOGIC[{i}]");

        for pin in [
            "D1", "D2", "D3", "D4", "D5", "D6", "T2", "T3", "T4", "CLKPERF", "CLKDIV",
        ] {
            ctx.collect_inv(tile, bel, pin);
        }

        let diff0 = ctx.get_diff_legacy(tile, bel, "T1INV", "T1");
        let diff1 = ctx.get_diff_legacy(tile, bel, "T1INV", "T1_B");
        let (diff0, diff1, _) = Diff::split(diff0, diff1);
        ctx.insert(tile, bel, "INV.T1", xlat_bit_bi_legacy(diff0, diff1));

        ctx.get_diff_legacy(tile, bel, "CLKINV.SAME", "CLK_B")
            .assert_empty();
        let diff_clk1 = ctx.get_diff_legacy(tile, bel, "CLKINV.OPPOSITE", "CLK");
        let diff_clk2 = ctx.get_diff_legacy(tile, bel, "CLKINV.OPPOSITE", "CLK_B");
        let diff_clk12 = ctx.get_diff_legacy(tile, bel, "CLKINV.SAME", "CLK");
        assert_eq!(diff_clk12, diff_clk1.combine(&diff_clk2));
        ctx.insert(tile, bel, "INV.CLK1", xlat_bit_legacy(!diff_clk1));
        ctx.insert(tile, bel, "INV.CLK2", xlat_bit_legacy(!diff_clk2));

        let item_oq = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.get_diff_legacy(tile, bel, "SRTYPE", "ASYNC")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bitvec_diff_legacy(&item_oq, &bits![1; 4], &bits![0; 4]);
        diff.apply_bitvec_diff_legacy(&item_tq, &bits![1; 2], &bits![0; 2]);
        diff.assert_empty();
        ctx.insert(tile, bel, "OFF_SR_SYNC", item_oq);
        ctx.insert(tile, bel, "TFF_SR_SYNC", item_tq);

        let item = ctx.extract_bit_bi_legacy(tile, bel, "INIT_OQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "OFF_INIT", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "INIT_OQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "OFF_INIT", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "INIT_TQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_bit_bi_legacy(tile, bel, "INIT_TQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRVAL_OQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRVAL_OQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "OFF_SRVAL", item);
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRVAL_TQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "TFF_SRVAL", item);
        let item = ctx.extract_bit_wide_bi_legacy(tile, bel, "SRVAL_TQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "TFF_SRVAL", item);

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
        ctx.insert(tile, bel, "OFF_SR_USED", osrused);
        ctx.insert(tile, bel, "TFF_SR_USED", tsrused);

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
            diff.apply_bit_diff_legacy(ctx.item(tile, bel, "OFF_SR_USED"), true, false);
        }
        let mut ddr3_byp = diffs[0].2.clone();
        for (_, _, diff) in &diffs {
            ddr3_byp.bits.retain(|k, _| diff.bits.contains_key(k));
        }
        let ddr3_byp = xlat_bit_legacy(ddr3_byp);
        for (_, _, diff) in &mut diffs {
            diff.apply_bit_diff_legacy(&ddr3_byp, true, false);
        }
        ctx.insert(tile, bel, "DDR3_BYPASS", ddr3_byp);
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
        ctx.insert(tile, bel, "DATA_WIDTH", xlat_enum_legacy(diffs_width));
        ctx.insert(tile, bel, "CLK_RATIO", xlat_enum_legacy(diffs_ratio));

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
        ctx.insert(tile, bel, "OMUX", item);

        let mut diff_sdr = ctx.get_diff_legacy(tile, bel, "DATA_RATE_TQ", "SDR");
        let mut diff_ddr = ctx.get_diff_legacy(tile, bel, "DATA_RATE_TQ", "DDR");
        diff_sdr.apply_bit_diff_legacy(ctx.item(tile, bel, "TFF_SR_USED"), true, false);
        diff_ddr.apply_bit_diff_legacy(ctx.item(tile, bel, "TFF_SR_USED"), true, false);
        let item = xlat_enum_legacy(vec![
            ("NONE", Diff::default()),
            ("T1", ctx.get_diff_legacy(tile, bel, "DATA_RATE_TQ", "BUF")),
            ("SERDES_SDR", diff_sdr),
            ("SERDES_DDR", diff_ddr),
            ("FF", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "#FF")),
            ("DDR", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "DDR")),
            ("LATCH", ctx.get_diff_legacy(tile, bel, "TFFTYPE", "#LATCH")),
        ]);
        ctx.insert(tile, bel, "TMUX", item);

        ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "DEFAULT")
            .assert_empty();
        let mut diff = ctx.get_diff_legacy(tile, bel, "INTERFACE_TYPE", "MEMORY_DDR3");

        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "OMUX"), "SERDES_DDR", "NONE");
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DATA_WIDTH"), "4", "NONE");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "OFF_SR_USED"), true, false);
        assert_eq!(diff.bits.len(), 1);
        ctx.insert(
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
        present_ologic.apply_bit_diff_legacy(ctx.item(tile, bel, "DDR3_BYPASS"), true, false);
        present_ologic.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "TFF_SRVAL"), 0, 7);
        present_ologic.apply_enum_diff_legacy(ctx.item(tile, bel, "TMUX"), "T1", "NONE");
        present_ologic.assert_empty();

        let mut present_oserdes = ctx.get_diff_legacy(tile, bel, "PRESENT", "OSERDES");
        present_oserdes.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "OFF_SRVAL"), 0, 7);
        present_oserdes.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "TFF_SRVAL"), 0, 7);
        present_oserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "OFF_INIT"), false, true);
        present_oserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "TFF_INIT"), false, true);
        present_oserdes.apply_bit_diff_legacy(ctx.item(tile, bel, "INV.CLKPERF"), false, true);
        present_oserdes.apply_enum_diff_legacy(ctx.item(tile, bel, "OMUX"), "D1", "NONE");
        present_oserdes.apply_enum_diff_legacy(ctx.item(tile, bel, "TMUX"), "T1", "NONE");
        present_oserdes.assert_empty();

        let mut vals = vec![];
        for j in 0..12 {
            vals.push(format!("HCLK{j}"));
        }
        for j in 0..6 {
            vals.push(format!("RCLK{j}"));
        }
        ctx.collect_enum_default_legacy_ocd(tile, bel, "MUX.CLKDIV", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_legacy_ocd(tile, bel, "MUX.CLKDIVB", &vals, "NONE", OcdMode::Mux);
        for j in 0..8 {
            vals.push(format!("IOCLK{j}"));
        }
        ctx.collect_enum_default_legacy_ocd(tile, bel, "MUX.CLK", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_default_legacy_ocd(tile, bel, "MUX.CLKB", &vals, "NONE", OcdMode::Mux);
        ctx.collect_enum_legacy(tile, bel, "MUX.CLKPERF", &["OCLK0", "OCLK1"]);
    }
    let mut diff = ctx.get_diff_legacy(tile, "OLOGIC_COMMON", "MISR_RESET", "1");
    let diff1 = diff.split_bits_by(|bit| bit.rect.to_idx() > 0);
    ctx.insert(tile, "OLOGIC[0]", "MISR_RESET", xlat_bit_legacy(diff));
    ctx.insert(tile, "OLOGIC[1]", "MISR_RESET", xlat_bit_legacy(diff1));
    for i in 0..2 {
        let bel = &format!("IODELAY[{i}]");
        ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
            .assert_empty();
        ctx.collect_inv(tile, bel, "C");
        ctx.collect_inv(tile, bel, "DATAIN");
        ctx.collect_inv(tile, bel, "IDATAIN");
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
        ctx.insert(tile, bel, "IDELAY_VALUE_INIT", xlat_bitvec_legacy(diffs_t));
        ctx.insert(tile, bel, "IDELAY_VALUE_CUR", xlat_bitvec_legacy(diffs_f));
        let item = ctx.extract_bitvec_legacy(tile, bel, "ODELAY_VALUE", "");
        ctx.insert(tile, bel, "ALT_DELAY_VALUE", item);
        let (_, _, mut diff) = Diff::split(
            ctx.peek_diff_legacy(tile, bel, "DELAY_SRC", "I").clone(),
            ctx.peek_diff_legacy(tile, bel, "DELAY_SRC", "O").clone(),
        );
        diff.discard_bits_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"));
        ctx.insert(tile, bel, "ENABLE", xlat_bit_legacy(diff));
        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["I", "IO", "O", "DATAIN", "CLKIN", "DELAYCHAIN_OSC"] {
            let mut diff = ctx.get_diff_legacy(tile, bel, "DELAY_SRC", val);
            diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
            diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
            diffs.push((val, diff));
        }
        ctx.insert(tile, bel, "DELAY_SRC", xlat_enum_legacy(diffs));

        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_DEFAULT");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "I", "NONE");
        let val = extract_bitvec_val_part_legacy(
            ctx.item(tile, bel, "IDELAY_VALUE_CUR"),
            &bits![1; 5],
            &mut diff,
        );
        ctx.insert_device_data("IODELAY:DEFAULT_IDELAY_VALUE", val);
        let val = extract_bitvec_val_part_legacy(
            ctx.item(tile, bel, "IDELAY_VALUE_INIT"),
            &bits![0; 5],
            &mut diff,
        );
        ctx.insert_device_data("IODELAY:DEFAULT_IDELAY_VALUE", val);
        ctx.insert(tile, bel, "EXTRA_DELAY", xlat_bit_legacy(diff));

        let mut diffs = vec![];
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_FIXED");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_VARIABLE");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_VAR_LOADABLE");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "I", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VAR_LOADABLE", diff));

        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "O_FIXED");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "O_VARIABLE");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "O_VAR_LOADABLE");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "O", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VAR_LOADABLE", diff));

        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "IO_FIXED");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("FIXED", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_FIXED_O_VARIABLE");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE_SWAPPED", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "I_VARIABLE_O_FIXED");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("VARIABLE", diff));
        let mut diff = ctx.get_diff_legacy(tile, bel, "MODE", "IO_VAR_LOADABLE");
        diff.apply_bit_diff_legacy(ctx.item(tile, bel, "ENABLE"), true, false);
        diff.apply_enum_diff_legacy(ctx.item(tile, bel, "DELAY_SRC"), "IO", "NONE");
        diff.apply_bitvec_diff_int_legacy(ctx.item(tile, bel, "IDELAY_VALUE_CUR"), 0, 0x1f);
        diffs.push(("IO_VAR_LOADABLE", diff));
        ctx.insert(tile, bel, "DELAY_TYPE", xlat_enum_legacy(diffs));
    }
    let mut present_vr = ctx.get_diff_legacy(tile, "IOB_COMMON", "PRESENT", "VR");
    for i in 0..2 {
        let bel = &format!("IOB[{i}]");
        ctx.collect_enum_default_legacy(
            tile,
            bel,
            "PULL",
            &["PULLDOWN", "PULLUP", "KEEPER"],
            "NONE",
        );
        ctx.collect_bit_bi_legacy(tile, bel, "OUTPUT_DELAY", "0", "1");
        let mut present = ctx.get_diff_legacy(tile, bel, "PRESENT", "IOB");
        let diff = ctx
            .get_diff_legacy(tile, bel, "PRESENT", "IPAD")
            .combine(&!&present);
        ctx.insert(tile, bel, "VREF_SYSMON", xlat_bit_legacy(diff));
        let diff = ctx
            .get_diff_legacy(tile, bel, "PRESENT", "IOB.CONTINUOUS")
            .combine(&!&present);
        ctx.insert(
            tile,
            bel,
            "DCIUPDATEMODE_ASREQUIRED",
            xlat_bit_legacy(!diff),
        );
        present.apply_enum_diff_legacy(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

        let oprog = ctx.extract_bitvec_legacy(tile, bel, "OPROGRAMMING", "");
        let lvds = TileItem::from_bitvec_inv(oprog.bits[0..9].to_vec(), false);
        let dci_t = TileItem::from_bit_inv(oprog.bits[11], false);
        let dci_mode = TileItem {
            bits: oprog.bits[12..15].to_vec(),
            kind: TileItemKind::Enum {
                values: [
                    ("NONE".into(), bits![0, 0, 0]),
                    ("OUTPUT".into(), bits![1, 0, 0]),
                    ("OUTPUT_HALF".into(), bits![0, 1, 0]),
                    ("TERM_VCC".into(), bits![1, 1, 0]),
                    ("TERM_SPLIT".into(), bits![0, 0, 1]),
                ]
                .into_iter()
                .collect(),
            },
        };
        let output_misc = TileItem::from_bitvec_inv(oprog.bits[15..19].to_vec(), false);
        let dci_misc = TileItem::from_bitvec_inv(oprog.bits[9..11].to_vec(), false);
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
                    TileBit::new(0, 41, 39),
                    TileBit::new(0, 41, 31),
                    TileBit::new(0, 41, 27),
                    TileBit::new(0, 40, 20),
                    TileBit::new(0, 40, 10),
                ],
                vec![
                    TileBit::new(0, 40, 44),
                    TileBit::new(0, 40, 30),
                    TileBit::new(0, 40, 32),
                    TileBit::new(0, 41, 17),
                    TileBit::new(0, 41, 43),
                ],
            )
        } else {
            (
                vec![
                    TileBit::new(1, 40, 24),
                    TileBit::new(1, 40, 32),
                    TileBit::new(1, 40, 36),
                    TileBit::new(1, 41, 43),
                    TileBit::new(1, 41, 53),
                ],
                vec![
                    TileBit::new(1, 41, 19),
                    TileBit::new(1, 41, 33),
                    TileBit::new(1, 41, 31),
                    TileBit::new(1, 40, 46),
                    TileBit::new(1, 40, 20),
                ],
            )
        };
        let pslew = TileItem::from_bitvec_inv(pslew_bits, false);
        let nslew = TileItem::from_bitvec_inv(nslew_bits, false);

        let mut diff = ctx
            .peek_diff_legacy(tile, bel, "OSTD", "LVCMOS25.12.SLOW")
            .combine(&present);
        for &bit in &pdrive_bits {
            diff.bits.remove(&bit);
        }
        for &bit in &ndrive_bits {
            diff.bits.remove(&bit);
        }
        extract_bitvec_val_part_legacy(&pslew, &bits![0; 5], &mut diff);
        extract_bitvec_val_part_legacy(&nslew, &bits![0; 5], &mut diff);
        ctx.insert(tile, bel, "OUTPUT_ENABLE", xlat_bit_wide_legacy(diff));

        let diff_cmos = ctx.peek_diff_legacy(tile, bel, "ISTD", "LVCMOS18.LP");
        let diff_cmos12 = ctx.peek_diff_legacy(tile, bel, "ISTD", "LVCMOS12.LP");
        let diff_vref_lp = ctx.peek_diff_legacy(tile, bel, "ISTD", "HSTL_I.LP");
        let diff_vref_hp = ctx.peek_diff_legacy(tile, bel, "ISTD", "HSTL_I.HP");
        let mut diff_diff_lp = ctx
            .peek_diff_legacy(tile, bel, "ISTD", "LVDS_25.LP")
            .clone();
        let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.rect.to_idx() == i);
        let mut diff_diff_hp = ctx
            .peek_diff_legacy(tile, bel, "ISTD", "LVDS_25.HP")
            .clone();
        let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.rect.to_idx() == i);
        let item = xlat_enum_legacy(vec![
            ("OFF", Diff::default()),
            ("CMOS", diff_cmos.clone()),
            ("CMOS12", diff_cmos12.clone()),
            ("VREF_LP", diff_vref_lp.clone()),
            ("VREF_HP", diff_vref_hp.clone()),
            ("DIFF_LP", diff_diff_lp),
            ("DIFF_HP", diff_diff_hp),
        ]);
        ctx.insert(tile, bel, "IBUF_MODE", item);

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
                    let mut diff = ctx.get_diff_legacy(tile, bel, "OSTD", val);
                    diff.apply_bitvec_diff_legacy(
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
                            ctx.insert_misc_data(format!("IOSTD:{attr}:{name}"), value);
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
                        ctx.insert_misc_data(format!("IOSTD:{attr}:{name}"), value);
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
                    ctx.insert_misc_data(format!("IOSTD:OUTPUT_MISC:{stdname}"), value);
                    match std.dci {
                        DciKind::None | DciKind::InputVcc | DciKind::InputSplit => {}
                        DciKind::Output => {
                            diff.apply_enum_diff_legacy(&dci_mode, "OUTPUT", "NONE");
                        }
                        DciKind::OutputHalf => {
                            diff.apply_enum_diff_legacy(&dci_mode, "OUTPUT_HALF", "NONE");
                        }
                        DciKind::BiVcc => {
                            diff.apply_enum_diff_legacy(&dci_mode, "TERM_VCC", "NONE");
                            diff.apply_bitvec_diff_legacy(&dci_misc, &bits![1, 1], &bits![0, 0]);
                        }
                        DciKind::BiSplit => {
                            diff.apply_enum_diff_legacy(&dci_mode, "TERM_SPLIT", "NONE");
                        }
                        DciKind::BiSplitT => {
                            diff.apply_enum_diff_legacy(&dci_mode, "TERM_SPLIT", "NONE");
                            diff.apply_bit_diff_legacy(&dci_t, true, false);
                        }
                    }
                    diff.assert_empty();
                }
            }
        }

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
            ctx.insert_misc_data(format!("IOSTD:{attr}:VR"), value);
        }
        present_vr.apply_enum_diff_legacy(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");
        present_vr.apply_enum_diff_legacy(&dci_mode, "TERM_SPLIT", "NONE");

        if i == 0 {
            let mut present_vref = ctx.get_diff_legacy(tile, bel, "PRESENT", "VREF");
            present_vref.apply_bit_diff_legacy(ctx.item(tile, bel, "VREF_SYSMON"), true, false);
            present_vref.apply_enum_diff_legacy(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

            for (attr, bits, invert) in [
                ("PDRIVE", &pdrive_bits, &pdrive_invert),
                ("NDRIVE", &ndrive_bits, &ndrive_invert),
                ("PSLEW", &pslew.bits, &bits![0; 5]),
                ("NSLEW", &nslew.bits, &bits![0; 5]),
            ] {
                let value: BitVec = bits
                    .iter()
                    .zip(invert.iter())
                    .map(|(&bit, inv)| match present_vref.bits.remove(&bit) {
                        Some(true) => !inv,
                        None => inv,
                        _ => unreachable!(),
                    })
                    .collect();
                ctx.insert_misc_data(format!("IOSTD:{attr}:OFF"), value);
            }
            present_vref.assert_empty();
        }

        ctx.insert_misc_data("IOSTD:OUTPUT_MISC:OFF", bits![0; 4]);
        ctx.insert_misc_data("IOSTD:LVDS_T:OFF", bits![0; 9]);
        ctx.insert_misc_data("IOSTD:LVDS_C:OFF", bits![0; 9]);
        ctx.insert_misc_data("IOSTD:PDRIVE:OFF", bits![0; 6]);
        ctx.insert_misc_data("IOSTD:NDRIVE:OFF", bits![0; 6]);
        ctx.insert_misc_data("IOSTD:PSLEW:OFF", bits![0; 5]);
        ctx.insert_misc_data("IOSTD:NSLEW:OFF", bits![0; 5]);
        ctx.insert(tile, bel, "LVDS", lvds);
        ctx.insert(tile, bel, "DCI_T", dci_t);
        ctx.insert(tile, bel, "DCI_MODE", dci_mode);
        ctx.insert(tile, bel, "OUTPUT_MISC", output_misc);
        ctx.insert(tile, bel, "DCI_MISC", dci_misc);
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
    let diff1 = present_vr.split_bits_by(|bit| bit.rect.to_idx() == 1);
    ctx.insert(tile, "IOB[0]", "VR", xlat_bit_legacy(present_vr));
    ctx.insert(tile, "IOB[1]", "VR", xlat_bit_legacy(diff1));
    // ISE bug.
    let mut diff = ctx.get_diff_legacy(tile, "IOB[0]", "PULL_DYNAMIC", "1");
    let diff1 = diff.split_bits_by(|bit| bit.rect.to_idx() == 1);
    ctx.insert(tile, "IOB[0]", "PULL_DYNAMIC", xlat_bit_legacy(diff));
    ctx.insert(tile, "IOB[1]", "PULL_DYNAMIC", xlat_bit_legacy(diff1));
    ctx.get_diff_legacy(tile, "IOB[1]", "PULL_DYNAMIC", "1")
        .assert_empty();

    for i in 0..2 {
        let bel = &format!("IOB[{i}]");
        for &std in IOSTDS {
            for lp in ["HP", "LP"] {
                let mut diff =
                    ctx.get_diff_legacy(tile, bel, "ISTD", format!("{sn}.{lp}", sn = std.name));
                if std.diff != DiffKind::None {
                    for bel in ["IOB[0]", "IOB[1]"] {
                        match std.dci {
                            DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                            DciKind::InputVcc | DciKind::BiVcc => {
                                diff.apply_enum_diff_legacy(
                                    ctx.item(tile, bel, "DCI_MODE"),
                                    "TERM_VCC",
                                    "NONE",
                                );
                                diff.apply_bitvec_diff_legacy(
                                    ctx.item(tile, bel, "DCI_MISC"),
                                    &bits![1, 1],
                                    &bits![0, 0],
                                );
                            }
                            DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                                diff.apply_enum_diff_legacy(
                                    ctx.item(tile, bel, "DCI_MODE"),
                                    "TERM_SPLIT",
                                    "NONE",
                                );
                            }
                        }
                        diff.apply_enum_diff_legacy(
                            ctx.item(tile, bel, "IBUF_MODE"),
                            if lp == "LP" { "DIFF_LP" } else { "DIFF_HP" },
                            "OFF",
                        );
                    }
                    diff.assert_empty();
                } else {
                    match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                        DciKind::InputVcc | DciKind::BiVcc => {
                            diff.apply_enum_diff_legacy(
                                ctx.item(tile, bel, "DCI_MODE"),
                                "TERM_VCC",
                                "NONE",
                            );
                            diff.apply_bitvec_diff_legacy(
                                ctx.item(tile, bel, "DCI_MISC"),
                                &bits![1, 1],
                                &bits![0, 0],
                            );
                        }
                        DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                            diff.apply_enum_diff_legacy(
                                ctx.item(tile, bel, "DCI_MODE"),
                                "TERM_SPLIT",
                                "NONE",
                            );
                        }
                    }
                    let mode = if std.vref.is_some() {
                        if lp == "LP" { "VREF_LP" } else { "VREF_HP" }
                    } else if std.vcco == Some(1200) {
                        "CMOS12"
                    } else {
                        "CMOS"
                    };
                    diff.apply_enum_diff_legacy(ctx.item(tile, bel, "IBUF_MODE"), mode, "OFF");
                    diff.assert_empty();
                }
            }
            if std.diff == DiffKind::True && i == 0 {
                let mut diff = ctx.get_diff_legacy(tile, bel, "DIFF_TERM", std.name);
                let val_c = extract_bitvec_val_part_legacy(
                    ctx.item(tile, "IOB[0]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part_legacy(
                    ctx.item(tile, "IOB[1]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("IOSTD:LVDS_T:TERM_{}", std.name), val_t);
                ctx.insert_misc_data(format!("IOSTD:LVDS_C:TERM_{}", std.name), val_c);
                diff.assert_empty();
                let mut diff = ctx.get_diff_legacy(tile, bel, "DIFF_TERM_DYNAMIC", std.name);
                let val_c = extract_bitvec_val_part_legacy(
                    ctx.item(tile, "IOB[0]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part_legacy(
                    ctx.item(tile, "IOB[1]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("IOSTD:LVDS_T:TERM_DYNAMIC_{}", std.name), val_t);
                ctx.insert_misc_data(format!("IOSTD:LVDS_C:TERM_DYNAMIC_{}", std.name), val_c);
                diff.assert_empty();
            }
            if std.diff == DiffKind::True && i == 1 {
                let mut diff = ctx.get_diff_legacy(tile, bel, "OSTD", std.name);
                let val_c = extract_bitvec_val_part_legacy(
                    ctx.item(tile, "IOB[0]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part_legacy(
                    ctx.item(tile, "IOB[1]", "LVDS"),
                    &bits![0; 9],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("IOSTD:LVDS_T:OUTPUT_{}", std.name), val_t);
                ctx.insert_misc_data(format!("IOSTD:LVDS_C:OUTPUT_{}", std.name), val_c);
                diff.apply_bitvec_diff_legacy(
                    ctx.item(tile, "IOB[1]", "OUTPUT_ENABLE"),
                    &bits![1; 2],
                    &bits![0; 2],
                );
                diff.assert_empty();
            }
            if std.diff == DiffKind::Pseudo && i == 1 {
                let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
                let mut diff = ctx.get_diff_legacy(tile, bel, "OSTD", std.name);
                for bel in ["IOB[0]", "IOB[1]"] {
                    diff.apply_bitvec_diff_legacy(
                        ctx.item(tile, bel, "OUTPUT_ENABLE"),
                        &bits![1; 2],
                        &bits![0; 2],
                    );
                    for attr in ["PDRIVE", "NDRIVE", "PSLEW", "NSLEW", "OUTPUT_MISC"] {
                        let item = ctx.item(tile, bel, attr);
                        let value = extract_bitvec_val_part_legacy(
                            item,
                            &BitVec::repeat(false, item.bits.len()),
                            &mut diff,
                        );
                        ctx.insert_misc_data(format!("IOSTD:{attr}:{stdname}"), value);
                    }
                    let dci_mode = ctx.item(tile, bel, "DCI_MODE");
                    let dci_misc = ctx.item(tile, bel, "DCI_MISC");
                    let dci_t = ctx.item(tile, bel, "DCI_T");
                    match std.dci {
                        DciKind::None | DciKind::InputVcc | DciKind::InputSplit => {}
                        DciKind::Output => {
                            diff.apply_enum_diff_legacy(dci_mode, "OUTPUT", "NONE");
                        }
                        DciKind::OutputHalf => {
                            diff.apply_enum_diff_legacy(dci_mode, "OUTPUT_HALF", "NONE");
                        }
                        DciKind::BiVcc => {
                            diff.apply_enum_diff_legacy(dci_mode, "TERM_VCC", "NONE");
                            diff.apply_bitvec_diff_legacy(dci_misc, &bits![1, 1], &bits![0, 0]);
                        }
                        DciKind::BiSplit => {
                            diff.apply_enum_diff_legacy(dci_mode, "TERM_SPLIT", "NONE");
                        }
                        DciKind::BiSplitT => {
                            diff.apply_enum_diff_legacy(dci_mode, "TERM_SPLIT", "NONE");
                            diff.apply_bit_diff_legacy(dci_t, true, false);
                        }
                    }
                }
                ctx.insert(
                    tile,
                    "IOB[0]",
                    "OMUX",
                    xlat_enum_legacy(vec![("O", Diff::default()), ("OTHER_O_INV", diff)]),
                );
            }
        }
    }

    let tile = "HCLK_IO";
    let lvdsbias = TileItem::from_bitvec_inv(
        vec![
            TileBit::new(0, 42, 30),
            TileBit::new(0, 42, 28),
            TileBit::new(0, 42, 27),
            TileBit::new(0, 42, 26),
            TileBit::new(0, 42, 25),
            TileBit::new(0, 42, 24),
            TileBit::new(0, 42, 23),
            TileBit::new(0, 42, 22),
            TileBit::new(0, 42, 21),
            TileBit::new(0, 42, 20),
            TileBit::new(0, 42, 19),
            TileBit::new(0, 42, 18),
            TileBit::new(0, 42, 17),
            TileBit::new(0, 42, 16),
            TileBit::new(0, 42, 15),
            TileBit::new(0, 42, 14),
            TileBit::new(0, 41, 28),
        ],
        false,
    );
    let bel = "DCI";
    let dci_en = ctx.get_diff_legacy(tile, bel, "ENABLE", "1");
    let test_en = ctx
        .get_diff_legacy(tile, bel, "TEST_ENABLE", "1")
        .combine(&!&dci_en);
    let dyn_en = ctx
        .get_diff_legacy(tile, bel, "DYNAMIC_ENABLE", "1")
        .combine(&!&dci_en);
    ctx.insert(tile, bel, "TEST_ENABLE", xlat_bit_wide_legacy(test_en));
    ctx.insert(tile, bel, "DYNAMIC_ENABLE", xlat_bit_legacy(dyn_en));
    let casc_from_above = ctx
        .get_diff_legacy(tile, bel, "CASCADE_FROM_ABOVE", "1")
        .combine(&!&dci_en);
    ctx.insert(
        tile,
        bel,
        "CASCADE_FROM_ABOVE",
        xlat_bit_wide_legacy(casc_from_above),
    );
    let casc_from_below = ctx
        .get_diff_legacy(tile, bel, "CASCADE_FROM_BELOW", "1")
        .combine(&!&dci_en);
    ctx.insert(
        tile,
        bel,
        "CASCADE_FROM_BELOW",
        xlat_bit_wide_legacy(casc_from_below),
    );

    let dci_en = xlat_bit_legacy(dci_en);
    let nref_output = TileItem::from_bitvec_inv(
        vec![TileBit::new(0, 40, 16), TileBit::new(0, 40, 17)],
        false,
    );
    let pref_output = TileItem::from_bitvec_inv(
        vec![TileBit::new(0, 41, 14), TileBit::new(0, 41, 15)],
        false,
    );
    let nref_output_half = TileItem::from_bitvec_inv(
        vec![
            TileBit::new(0, 40, 18),
            TileBit::new(0, 40, 19),
            TileBit::new(0, 40, 20),
        ],
        false,
    );
    let pref_output_half = TileItem::from_bitvec_inv(
        vec![
            TileBit::new(0, 41, 16),
            TileBit::new(0, 41, 17),
            TileBit::new(0, 41, 18),
        ],
        false,
    );
    let pref_term_vcc = TileItem::from_bitvec_inv(
        vec![TileBit::new(0, 40, 14), TileBit::new(0, 40, 15)],
        false,
    );
    let pmask_term_vcc = TileItem::from_bitvec_inv(
        vec![
            TileBit::new(0, 43, 14),
            TileBit::new(0, 43, 27),
            TileBit::new(0, 43, 28),
            TileBit::new(0, 43, 29),
            TileBit::new(0, 43, 30),
            TileBit::new(0, 43, 31),
        ],
        false,
    );
    let nref_term_split = TileItem::from_bitvec_inv(
        vec![
            TileBit::new(0, 40, 23),
            TileBit::new(0, 40, 24),
            TileBit::new(0, 40, 25),
        ],
        false,
    );
    let pref_term_split = TileItem::from_bitvec_inv(
        vec![
            TileBit::new(0, 41, 19),
            TileBit::new(0, 41, 20),
            TileBit::new(0, 41, 21),
        ],
        false,
    );
    let pmask_term_split = TileItem::from_bitvec_inv(
        vec![
            TileBit::new(0, 43, 21),
            TileBit::new(0, 43, 22),
            TileBit::new(0, 43, 23),
            TileBit::new(0, 43, 24),
            TileBit::new(0, 43, 25),
            TileBit::new(0, 43, 26),
        ],
        false,
    );
    let nmask_term_split = TileItem::from_bitvec_inv(
        vec![
            TileBit::new(0, 43, 15),
            TileBit::new(0, 43, 16),
            TileBit::new(0, 43, 17),
            TileBit::new(0, 43, 18),
            TileBit::new(0, 43, 19),
            TileBit::new(0, 43, 20),
        ],
        false,
    );
    ctx.collect_enum_default_legacy(
        tile,
        "INTERNAL_VREF",
        "VREF",
        &["600", "750", "900", "1100", "1250"],
        "OFF",
    );
    for std in IOSTDS {
        if std.diff == DiffKind::True {
            let bel = "LVDS";
            let diff = ctx.get_diff_legacy(tile, bel, "STD", std.name);
            let val = extract_bitvec_val_legacy(&lvdsbias, &bits![0; 17], diff);
            ctx.insert_misc_data(format!("IOSTD:LVDSBIAS:{}", std.name), val);
        }
        if std.dci != DciKind::None {
            let bel = "DCI";
            let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
            let mut diff = ctx.get_diff_legacy(tile, bel, "STD", std.name);
            match std.dci {
                DciKind::Output => {
                    let val = extract_bitvec_val_part_legacy(&nref_output, &bits![0; 2], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:NREF_OUTPUT:{stdname}"), val);
                    let val = extract_bitvec_val_part_legacy(&pref_output, &bits![0; 2], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:PREF_OUTPUT:{stdname}"), val);
                }
                DciKind::OutputHalf => {
                    let val =
                        extract_bitvec_val_part_legacy(&nref_output_half, &bits![0; 3], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:NREF_OUTPUT_HALF:{stdname}"), val);
                    let val =
                        extract_bitvec_val_part_legacy(&pref_output_half, &bits![0; 3], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:PREF_OUTPUT_HALF:{stdname}"), val);
                }
                DciKind::InputVcc | DciKind::BiVcc => {
                    let val =
                        extract_bitvec_val_part_legacy(&pref_term_vcc, &bits![0; 2], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:PREF_TERM_VCC:{stdname}"), val);
                    let val =
                        extract_bitvec_val_part_legacy(&pmask_term_vcc, &bits![0; 6], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_VCC:{stdname}"), val);
                }
                DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                    let val =
                        extract_bitvec_val_part_legacy(&nref_term_split, &bits![0; 3], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:NREF_TERM_SPLIT:{stdname}"), val);
                    let val =
                        extract_bitvec_val_part_legacy(&pref_term_split, &bits![0; 3], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:PREF_TERM_SPLIT:{stdname}"), val);
                    let val =
                        extract_bitvec_val_part_legacy(&pmask_term_split, &bits![0; 6], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_SPLIT:{stdname}"), val);
                    let val =
                        extract_bitvec_val_part_legacy(&nmask_term_split, &bits![0; 6], &mut diff);
                    ctx.insert_misc_data(format!("IOSTD:DCI:NMASK_TERM_SPLIT:{stdname}"), val);
                }
                _ => {}
            }
            diff.apply_bit_diff_legacy(&dci_en, true, false);
            diff.assert_empty();
        }
    }
    let bel = "LVDS";
    ctx.insert(tile, bel, "LVDSBIAS", lvdsbias);
    ctx.insert_misc_data("IOSTD:LVDSBIAS:OFF", bits![0; 17]);
    let bel = "DCI";
    ctx.insert(tile, bel, "ENABLE", dci_en);
    ctx.insert(tile, bel, "PREF_OUTPUT", pref_output);
    ctx.insert(tile, bel, "NREF_OUTPUT", nref_output);
    ctx.insert(tile, bel, "PREF_OUTPUT_HALF", pref_output_half);
    ctx.insert(tile, bel, "NREF_OUTPUT_HALF", nref_output_half);
    ctx.insert(tile, bel, "PREF_TERM_VCC", pref_term_vcc);
    ctx.insert(tile, bel, "PREF_TERM_SPLIT", pref_term_split);
    ctx.insert(tile, bel, "NREF_TERM_SPLIT", nref_term_split);

    ctx.insert(tile, bel, "PMASK_TERM_VCC", pmask_term_vcc);
    ctx.insert(tile, bel, "PMASK_TERM_SPLIT", pmask_term_split);
    ctx.insert(tile, bel, "NMASK_TERM_SPLIT", nmask_term_split);
    ctx.collect_bit_legacy(tile, bel, "QUIET", "1");

    ctx.insert_misc_data("IOSTD:DCI:PREF_OUTPUT:OFF", bits![0; 2]);
    ctx.insert_misc_data("IOSTD:DCI:NREF_OUTPUT:OFF", bits![0; 2]);
    ctx.insert_misc_data("IOSTD:DCI:PREF_OUTPUT_HALF:OFF", bits![0; 3]);
    ctx.insert_misc_data("IOSTD:DCI:NREF_OUTPUT_HALF:OFF", bits![0; 3]);
    ctx.insert_misc_data("IOSTD:DCI:PREF_TERM_VCC:OFF", bits![0; 2]);
    ctx.insert_misc_data("IOSTD:DCI:PMASK_TERM_VCC:OFF", bits![0; 6]);
    ctx.insert_misc_data("IOSTD:DCI:PREF_TERM_SPLIT:OFF", bits![0; 3]);
    ctx.insert_misc_data("IOSTD:DCI:NREF_TERM_SPLIT:OFF", bits![0; 3]);
    ctx.insert_misc_data("IOSTD:DCI:PMASK_TERM_SPLIT:OFF", bits![0; 6]);
    ctx.insert_misc_data("IOSTD:DCI:NMASK_TERM_SPLIT:OFF", bits![0; 6]);
    let tile = "CFG";
    let bel = "MISC";
    ctx.collect_bit_wide_legacy(tile, bel, "DCI_CLK_ENABLE", "1");
}
