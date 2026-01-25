use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::BelSlotId,
    grid::{CellCoord, DieId, RowId, TileCoord, TileIobId},
};
use prjcombine_re_fpga_hammer::{
    backend::{FuzzerFeature, FuzzerProp},
    diff::{
        Diff, DiffKey, FeatureId, OcdMode, extract_bitvec_val, extract_bitvec_val_part, xlat_bit,
        xlat_bit_wide, xlat_bitvec, xlat_bool, xlat_enum, xlat_enum_ocd,
    },
};
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{ExpandedBond, ExpandedDevice};
use prjcombine_types::{
    bits,
    bitvec::BitVec,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::{defs, expanded::IoCoord};

use crate::{
    backend::{IseBackend, Key, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        iostd::{DciKind, DiffKind, Iostd},
        props::DynProp,
    },
};

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

fn get_vrefs(backend: &IseBackend, tcrd: TileCoord) -> Vec<TileCoord> {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let chip = edev.chips[tcrd.die];

    let row_cfg = chip.row_reg_bot(chip.reg_cfg);
    let rows = if Some(tcrd.col) == edev.col_lio || Some(tcrd.col) == edev.col_rio {
        let mut reg = chip.row_to_reg(tcrd.row);
        if reg.to_idx() % 2 == 1 {
            reg -= 1;
        }
        let bot = chip.row_reg_bot(reg);
        vec![bot + 4, bot + 12, bot + 20, bot + 28]
    } else if tcrd.row < edev.row_dcmiob.unwrap() + 8 {
        vec![edev.row_dcmiob.unwrap() + 4]
    } else if tcrd.row < row_cfg {
        let mut res = vec![];
        let mut vref_row = edev.row_dcmiob.unwrap() + 12;
        while vref_row < row_cfg - 8 {
            res.push(vref_row);
            vref_row += 8;
        }
        res
    } else if tcrd.row < edev.row_iobdcm.unwrap() - 8 {
        let mut res = vec![];
        let mut vref_row = row_cfg + 12;
        while vref_row < edev.row_iobdcm.unwrap() - 8 {
            res.push(vref_row);
            vref_row += 8;
        }
        res
    } else {
        vec![edev.row_iobdcm.unwrap() - 4]
    };
    rows.into_iter()
        .map(|vref_row| tcrd.with_row(vref_row).tile(defs::tslots::BEL))
        .collect()
}

// Reused by v5/v6/v7.
#[derive(Clone, Copy, Debug)]
pub struct IsBonded(pub BelSlotId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for IsBonded {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply(
        &self,
        backend: &IseBackend<'b>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'b>>,
    ) -> Option<(Fuzzer<IseBackend<'b>>, bool)> {
        let FuzzerValue::Base(Value::String(pkg)) = &fuzzer.kv[&Key::Package] else {
            unreachable!()
        };
        let ExpandedBond::Virtex4(ref ebond) = backend.ebonds[pkg] else {
            unreachable!()
        };
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };

        let io = edev.get_io_info(IoCoord {
            cell: tcrd.cell,
            iob: TileIobId::from_idx(
                defs::bslots::IOB
                    .into_iter()
                    .position(|x| x == self.0)
                    .unwrap(),
            ),
        });
        if !ebond.ios.contains_key(&(io.bank, io.biob)) {
            return None;
        }
        Some((fuzzer, false))
    }
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
        let vrefs = get_vrefs(backend, tcrd);
        if vrefs.contains(&tcrd) {
            return None;
        }
        for tcrd_vref in vrefs {
            fuzzer = fuzzer.fuzz(Key::TileMutex(tcrd_vref, "VREF".into()), None, "EXCLUSIVE");
            let site = backend
                .ngrid
                .get_bel_name(tcrd_vref.cell.bel(defs::bslots::IOB[0]))
                .unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: "IO".into(),
                    bel: "IOB[0]".into(),
                    attr: "PRESENT".into(),
                    val: "VREF".into(),
                }),
                rects: backend.edev.tile_bits(tcrd_vref),
            });
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
struct Dci(&'static str);

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
        if tcrd.col == edev.col_cfg {
            // Center column is more trouble than it's worth.
            return None;
        }
        if tcrd.row.to_idx() % 32 == 9 {
            // Not in VR tile please.
            return None;
        }
        // Ensure nothing is placed in VR.
        let cell_vr = tcrd
            .cell
            .with_row(RowId::from_idx(tcrd.row.to_idx() / 32 * 32 + 9));
        let tile_vr = cell_vr.tile(defs::tslots::BEL);
        for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
            let site = backend.ngrid.get_bel_name(cell_vr.bel(bel)).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Test VR.
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::Legacy(FeatureId {
                tile: "IO".into(),
                bel: "IOB_COMMON".into(),
                attr: "PRESENT".into(),
                val: "VR".into(),
            }),
            rects: edev.tile_bits(tile_vr),
        });
        // Take exclusive mutex on bank DCI.
        let hclk_iois_dci = cell_vr.delta(0, -1).tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_iois_dci, "BANK_DCI".to_string()),
            None,
            "EXCLUSIVE",
        );
        // Test bank DCI.
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::Legacy(FeatureId {
                tile: "HCLK_IO_DCI".into(),
                bel: "DCI".into(),
                attr: "STD".into(),
                val: self.0.into(),
            }),
            rects: edev.tile_bits(hclk_iois_dci),
        });
        // Take shared mutex on global DCI.
        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
        // Anchor global DCI by putting something in bottom IOB of center column.
        let iob_center = tcrd
            .cell
            .with_cr(edev.col_cfg, edev.row_dcmiob.unwrap())
            .bel(defs::bslots::IOB[0]);
        let site = backend.ngrid.get_bel_name(iob_center).unwrap();
        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
        fuzzer = fuzzer.base(Key::SiteAttr(site, "IOATTRBOX".into()), "LVDCI_33");
        // Ensure anchor VR IOBs are free.
        for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
            let iob_center_vr = tcrd
                .cell
                .with_cr(edev.col_cfg, edev.row_dcmiob.unwrap() + 1)
                .bel(bel);
            let site = backend.ngrid.get_bel_name(iob_center_vr).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
struct DiffOut(Option<&'static str>);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DiffOut {
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
        // Skip non-NC pads.
        if tcrd.col == edev.col_cfg {
            return None;
        }
        if matches!(tcrd.row.to_idx() % 16, 7 | 8) {
            return None;
        }
        let hclk_iois_lvds = tcrd
            .cell
            .with_row(RowId::from_idx(tcrd.row.to_idx() / 32 * 32 + 24))
            .tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_iois_lvds, "BANK_LVDS".to_string()),
            None,
            "EXCLUSIVE",
        );
        if let Some(std) = self.0 {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::Legacy(FeatureId {
                    tile: "HCLK_IO_LVDS".into(),
                    bel: "LVDS".into(),
                    attr: "STD".into(),
                    val: std.into(),
                }),
                rects: edev.tile_bits(hclk_iois_lvds),
            });
        }
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
    let mut ctx = FuzzCtx::new(session, backend, "IO");

    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::ILOGIC[i]);
        let bel_ologic = defs::bslots::OLOGIC[i];
        bctx.build()
            .bel_unused(bel_ologic)
            .test_manual("PRESENT", "ILOGIC")
            .mode("ILOGIC")
            .commit();
        bctx.build()
            .bel_unused(bel_ologic)
            .test_manual("PRESENT", "ISERDES")
            .mode("ISERDES")
            .commit();

        bctx.mode("ILOGIC")
            .attr("IMUX", "1")
            .attr("IDELAYMUX", "1")
            .attr("IDELMUX", "0")
            .pin("CLKDIV")
            .test_enum("CLKDIVINV", &["CLKDIV", "CLKDIV_B"]);
        bctx.mode("ISERDES")
            .pin("CLKDIV")
            .test_enum("CLKDIVINV", &["CLKDIV", "CLKDIV_B"]);

        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .pin("CLK")
            .test_enum("CLKINV", &["CLK", "CLK_B"]);
        bctx.mode("ISERDES")
            .pin("CLK")
            .test_enum("CLKINV", &["CLK", "CLK_B"]);

        bctx.mode("ISERDES")
            .attr("INTERFACE_TYPE", "MEMORY")
            .attr("DATA_RATE", "SDR")
            .pin("OCLK")
            .test_enum_suffix("OCLKINV", "SDR", &["OCLK", "OCLK_B"]);
        bctx.mode("ISERDES")
            .attr("INTERFACE_TYPE", "MEMORY")
            .attr("DATA_RATE", "DDR")
            .pin("OCLK")
            .test_enum_suffix("OCLKINV", "DDR", &["OCLK", "OCLK_B"]);

        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .pin("CE1")
            .test_enum("CE1INV", &["CE1", "CE1_B"]);
        bctx.mode("ISERDES")
            .attr("INIT_CE", "11")
            .pin("CE1")
            .test_enum("CE1INV", &["CE1", "CE1_B"]);
        bctx.mode("ISERDES")
            .attr("INIT_CE", "11")
            .pin("CE2")
            .test_enum("CE2INV", &["CE2", "CE2_B"]);

        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .pin("SR")
            .bel_mode(bel_ologic, "OSERDES")
            .bel_attr(bel_ologic, "SRINV", "SR")
            .bel_pin(bel_ologic, "SR")
            .test_enum_suffix("SRINV", "OSR", &["SR", "SR_B"]);
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .pin("SR")
            .bel_mode(bel_ologic, "OSERDES")
            .bel_attr(bel_ologic, "SRINV", "SR_B")
            .bel_pin(bel_ologic, "SR")
            .test_enum_suffix("SRINV", "OSR_B", &["SR", "SR_B"]);
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .pin("REV")
            .bel_mode(bel_ologic, "OSERDES")
            .bel_attr(bel_ologic, "REVINV", "REV")
            .bel_pin(bel_ologic, "REV")
            .test_enum_suffix("REVINV", "OREV", &["REV", "REV_B"]);
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .pin("REV")
            .bel_mode(bel_ologic, "OSERDES")
            .bel_attr(bel_ologic, "REVINV", "REV_B")
            .bel_pin(bel_ologic, "REV")
            .test_enum_suffix("REVINV", "OREV_B", &["REV", "REV_B"]);
        bctx.mode("ISERDES")
            .pin("SR")
            .bel_mode(bel_ologic, "OSERDES")
            .bel_attr(bel_ologic, "SRINV", "SR")
            .bel_pin(bel_ologic, "SR")
            .test_enum_suffix("SRINV", "OSR", &["SR", "SR_B"]);
        bctx.mode("ISERDES")
            .pin("SR")
            .bel_mode(bel_ologic, "OSERDES")
            .bel_attr(bel_ologic, "SRINV", "SR_B")
            .bel_pin(bel_ologic, "SR")
            .test_enum_suffix("SRINV", "OSR_B", &["SR", "SR_B"]);
        bctx.mode("ISERDES")
            .pin("REV")
            .bel_mode(bel_ologic, "OSERDES")
            .bel_attr(bel_ologic, "REVINV", "REV")
            .bel_pin(bel_ologic, "REV")
            .test_enum_suffix("REVINV", "OREV", &["REV", "REV_B"]);
        bctx.mode("ISERDES")
            .pin("REV")
            .bel_mode(bel_ologic, "OSERDES")
            .bel_attr(bel_ologic, "REVINV", "REV_B")
            .bel_pin(bel_ologic, "REV")
            .test_enum_suffix("REVINV", "OREV_B", &["REV", "REV_B"]);

        bctx.mode("ISERDES")
            .attr("DATA_WIDTH", "2")
            .test_enum("SERDES", &["FALSE", "TRUE"]);
        bctx.mode("ISERDES")
            .test_enum("SERDES_MODE", &["SLAVE", "MASTER"]);
        bctx.mode("ISERDES")
            .test_enum("INTERFACE_TYPE", &["NETWORKING", "MEMORY"]);
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .attr("Q2MUX", "IFF2")
            .attr("IFFMUX", "1")
            .attr("IFFDELMUX", "1")
            .pin("D")
            .pin("Q1")
            .pin("Q2")
            .test_enum_suffix("Q1MUX", "IFF2", &["IFF1", "IFF3"]);
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .attr("Q2MUX", "IFF4")
            .attr("IFFMUX", "1")
            .attr("IFFDELMUX", "1")
            .pin("D")
            .pin("Q1")
            .pin("Q2")
            .test_enum_suffix("Q1MUX", "IFF4", &["IFF1", "IFF3"]);
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .attr("Q1MUX", "IFF1")
            .attr("IFFMUX", "1")
            .attr("IFFDELMUX", "1")
            .pin("D")
            .pin("Q1")
            .pin("Q2")
            .test_enum_suffix("Q2MUX", "IFF1", &["IFF2", "IFF4"]);
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .attr("Q1MUX", "IFF3")
            .attr("IFFMUX", "1")
            .attr("IFFDELMUX", "1")
            .pin("D")
            .pin("Q1")
            .pin("Q2")
            .test_enum_suffix("Q2MUX", "IFF3", &["IFF2", "IFF4"]);

        bctx.mode("ISERDES")
            .attr("SERDES", "TRUE")
            .test_enum("DATA_WIDTH", &["2", "3", "4", "5", "6", "7", "8", "10"]);
        bctx.mode("ISERDES")
            .attr("SRTYPE", "SYNC")
            .test_enum_suffix("BITSLIP_ENABLE", "SYNC", &["FALSE", "TRUE"]);
        bctx.mode("ISERDES")
            .attr("SRTYPE", "ASYNC")
            .test_enum_suffix("BITSLIP_ENABLE", "ASYNC", &["FALSE", "TRUE"]);
        bctx.mode("ISERDES").test_enum("NUM_CE", &["1", "2"]);
        bctx.mode("ISERDES")
            .attr("INIT_BITSLIPCNT", "1111")
            .attr("INIT_RANK1_PARTIAL", "11111")
            .attr("INIT_RANK2", "111111")
            .attr("INIT_RANK3", "111111")
            .test_enum("DATA_RATE", &["SDR", "DDR"]);
        bctx.mode("ISERDES").test_enum(
            "DDR_CLK_EDGE",
            &["OPPOSITE_EDGE", "SAME_EDGE", "SAME_EDGE_PIPELINED"],
        );

        bctx.mode("ILOGIC").test_enum("IFF1", &["#FF", "#LATCH"]);
        for attr in [
            "INIT_Q1", "INIT_Q2", "INIT_Q3", "INIT_Q4", "SRVAL_Q1", "SRVAL_Q2", "SRVAL_Q3",
            "SRVAL_Q4",
        ] {
            bctx.mode("ILOGIC")
                .attr("IFF1", "#FF")
                .test_enum(attr, &["0", "1"]);
            bctx.mode("ISERDES").test_enum(attr, &["0", "1"]);
        }
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .test_enum("SRTYPE", &["SYNC", "ASYNC"]);
        bctx.mode("ISERDES").test_enum("SRTYPE", &["SYNC", "ASYNC"]);

        bctx.mode("ISERDES")
            .attr("CE1INV", "CE1")
            .attr("CE2INV", "CE2")
            .pin("CE1")
            .pin("CE2")
            .test_multi_attr_bin("INIT_CE", 2);
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_BITSLIPCNT", 4);
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_RANK1_PARTIAL", 5);
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_RANK2", 6);
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
            .test_multi_attr_bin("INIT_RANK3", 6);

        bctx.mode("ILOGIC")
            .attr("IMUX", "0")
            .attr("IDELMUX", "1")
            .attr("IFFMUX", "#OFF")
            .pin("D")
            .pin("TFB")
            .pin("OFB")
            .pin("O")
            .test_enum("D2OBYP_SEL", &["GND", "T"]);
        bctx.mode("ILOGIC")
            .attr("IFFMUX", "0")
            .attr("IFF1", "#FF")
            .attr("IFFDELMUX", "1")
            .attr("IMUX", "#OFF")
            .pin("D")
            .pin("TFB")
            .pin("OFB")
            .test_enum("D2OFFBYP_SEL", &["GND", "T"]);
        bctx.mode("ILOGIC")
            .attr("IDELMUX", "1")
            .attr("IDELMUX1USED", "0")
            .pin("D")
            .pin("O")
            .pin("TFB")
            .pin("OFB")
            .test_enum("IMUX", &["0", "1"]);
        bctx.mode("ILOGIC")
            .attr("IFFDELMUX", "1")
            .attr("IFF1", "#FF")
            .pin("D")
            .pin("TFB")
            .pin("OFB")
            .test_enum("IFFMUX", &["0", "1"]);
        bctx.mode("ILOGIC")
            .attr("IDELMUX", "0")
            .attr("IMUX", "1")
            .attr("CLKDIVINV", "CLKDIV")
            .pin("D")
            .pin("O")
            .pin("OFB")
            .pin("CLKDIV")
            .test_enum("IDELAYMUX", &["0", "1"]);
        bctx.mode("ILOGIC")
            .attr("IMUX", "1")
            .attr("IFFMUX", "1")
            .attr("IFF1", "#FF")
            .attr("IDELMUX1USED", "0")
            .attr("IDELAYMUX", "1")
            .attr("IFFDELMUX", "0")
            .attr("Q1MUX", "IFF1")
            .pin("D")
            .pin("O")
            .pin("Q1")
            .pin("TFB")
            .pin("OFB")
            .test_enum("IDELMUX", &["0", "1"]);
        bctx.mode("ILOGIC")
            .attr("IMUX", "1")
            .attr("IFFMUX", "0")
            .attr("IFF1", "#FF")
            .attr("IDELMUX1USED", "0")
            .attr("IDELAYMUX", "1")
            .attr("IDELMUX", "0")
            .attr("Q1MUX", "IFF1")
            .attr("D2OFFBYP_SEL", "T")
            .pin("D")
            .pin("O")
            .pin("Q1")
            .pin("TFB")
            .pin("OFB")
            .test_enum("IFFDELMUX", &["0", "1"]);

        for val in ["NONE", "IFD", "IBUF", "BOTH"] {
            bctx.mode("ISERDES")
                .attr("IOBDELAY", val)
                .pin("OFB")
                .test_enum_suffix("OFB_USED", val, &["FALSE", "TRUE"]);
        }
        bctx.mode("ISERDES")
            .pin("TFB")
            .test_enum("TFB_USED", &["FALSE", "TRUE"]);
        bctx.mode("ISERDES")
            .attr("OFB_USED", "FALSE")
            .test_enum("IOBDELAY", &["NONE", "IFD", "IBUF", "BOTH"]);

        bctx.mode("ILOGIC")
            .attr("IDELMUX", "0")
            .attr("IMUX", "1")
            .attr("IDELAYMUX", "1")
            .attr("CLKDIVINV", "CLKDIV")
            .attr("IFFDELMUX", "#OFF")
            .pin("CLKDIV")
            .pin("D")
            .pin("O")
            .test_enum_suffix(
                "IOBDELAY_TYPE",
                "ILOGIC.IBUF",
                &["DEFAULT", "FIXED", "VARIABLE"],
            );
        bctx.mode("ILOGIC")
            .attr("IFFDELMUX", "0")
            .attr("IFFMUX", "1")
            .attr("IDELAYMUX", "1")
            .attr("CLKDIVINV", "CLKDIV")
            .attr("IDELMUX", "#OFF")
            .attr("IFF1", "#FF")
            .attr("Q1MUX", "IFF1")
            .pin("CLKDIV")
            .pin("D")
            .pin("Q1")
            .test_enum_suffix(
                "IOBDELAY_TYPE",
                "ILOGIC.IFD",
                &["DEFAULT", "FIXED", "VARIABLE"],
            );
        bctx.mode("ISERDES")
            .attr("IOBDELAY", "IBUF")
            .test_enum_suffix(
                "IOBDELAY_TYPE",
                "ISERDES.IBUF",
                &["DEFAULT", "FIXED", "VARIABLE"],
            );
        bctx.mode("ISERDES")
            .attr("IOBDELAY", "IFD")
            .test_enum_suffix(
                "IOBDELAY_TYPE",
                "ISERDES.IFD",
                &["DEFAULT", "FIXED", "VARIABLE"],
            );

        bctx.mode("ILOGIC").test_multi_attr_dec("IOBDELAY_VALUE", 6);
        bctx.mode("ISERDES")
            .test_multi_attr_dec("IOBDELAY_VALUE", 6);

        bctx.build()
            .mutex("MUX.CLK", "CKINT")
            .test_manual("MUX.CLK", "CKINT")
            .pip("CLKMUX", "CLKMUX_INT")
            .commit();
        for ipin in [
            "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "RCLK0",
            "RCLK1", "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0", "IOCLK_N1",
        ] {
            bctx.build()
                .mutex("MUX.CLK", ipin)
                .test_manual("MUX.CLK", ipin)
                .pip("CLKMUX", (defs::bslots::IOI, ipin))
                .commit();
        }
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(defs::bslots::OLOGIC[i]);
        let bel_ilogic = defs::bslots::ILOGIC[i];
        bctx.build()
            .bel_unused(bel_ilogic)
            .test_manual("PRESENT", "OLOGIC")
            .mode("OLOGIC")
            .commit();
        bctx.build()
            .bel_unused(bel_ilogic)
            .test_manual("PRESENT", "OSERDES")
            .mode("OSERDES")
            .commit();
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("OMUX", "OFFDDRA")
            .pin("CLK")
            .pin("OQ")
            .test_enum_suffix("CLK1INV", "OLOGIC", &["C", "C_B"]);
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("OMUX", "OFFDDRA")
            .pin("CLK")
            .pin("OQ")
            .test_enum_suffix("CLK2INV", "OLOGIC", &["CLK", "CLK_B"]);
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("OSRUSED", "0")
            .attr("OMUX", "OFFDDRA")
            .pin("SR")
            .pin("OQ")
            .bel_unused(bel_ilogic)
            .test_enum_suffix("SRINV", "OLOGIC", &["SR", "SR_B"]);
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("OREVUSED", "0")
            .attr("OMUX", "OFFDDRA")
            .pin("REV")
            .pin("OQ")
            .bel_unused(bel_ilogic)
            .test_enum_suffix("REVINV", "OLOGIC", &["REV", "REV_B"]);
        for pin in ["D1", "D2", "OCE"] {
            bctx.mode("OLOGIC")
                .attr("OFF1", "#FF")
                .attr("OMUX", "OFFDDRA")
                .pin(pin)
                .pin("OQ")
                .test_enum_suffix(format!("{pin}INV"), "OLOGIC", &[pin, &format!("{pin}_B")]);
        }
        for pin in ["T2", "TCE"] {
            bctx.mode("OLOGIC")
                .attr("TFF1", "#FF")
                .attr("TMUX", "TFFDDRA")
                .pin(pin)
                .pin("TQ")
                .test_enum_suffix(format!("{pin}INV"), "OLOGIC", &[pin, &format!("{pin}_B")]);
        }
        bctx.mode("OLOGIC")
            .attr("TMUX", "T1")
            .attr("T1USED", "0")
            .pin("T1")
            .pin("TQ")
            .test_enum_suffix("T1INV", "OLOGIC", &["T1", "T1_B"]);

        for pin in [
            "CLKDIV", "SR", "REV", "D1", "D2", "D3", "D4", "D5", "D6", "T2", "T3", "T4",
        ] {
            bctx.mode("OSERDES")
                .pin(pin)
                .bel_unused(bel_ilogic)
                .test_enum_suffix(format!("{pin}INV"), "OSERDES", &[pin, &format!("{pin}_B")]);
        }
        bctx.mode("OSERDES")
            .attr("DATA_RATE_TQ", "BUF")
            .pin("T1")
            .test_enum_suffix("T1INV", "OSERDES", &["T1", "T1_B"]);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_TQ", "DDR")
            .pin("TCE")
            .test_enum_suffix("TCEINV", "OSERDES", &["TCE", "TCE_B"]);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("CLKINV", "CLK")
            .attr("DDR_CLK_EDGE", "SAME_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix("OCEINV", "OSERDES", &["OCE", "OCE_B"]);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("OCEINV", "OCE")
            .attr("DDR_CLK_EDGE", "SAME_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix("CLKINV", "OSERDES.SAME", &["CLK", "CLK_B"]);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("OCEINV", "OCE")
            .attr("DDR_CLK_EDGE", "OPPOSITE_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_enum_suffix("CLKINV", "OSERDES.OPPOSITE", &["CLK", "CLK_B"]);

        bctx.mode("OLOGIC")
            .attr("OCEINV", "OCE_B")
            .pin("OCE")
            .test_enum("OFF1", &["#FF", "#LATCH"]);
        bctx.mode("OLOGIC")
            .attr("TCEINV", "TCE_B")
            .pin("TCE")
            .test_enum("TFF1", &["#FF", "#LATCH"]);
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .test_enum("SRTYPE_OQ", &["SYNC", "ASYNC"]);
        bctx.mode("OLOGIC")
            .attr("TFF1", "#FF")
            .test_enum("SRTYPE_TQ", &["SYNC", "ASYNC"]);
        for (attr, oattr) in [
            ("OSRUSED", "TSRUSED"),
            ("TSRUSED", "OSRUSED"),
            ("OREVUSED", "TREVUSED"),
            ("TREVUSED", "OREVUSED"),
        ] {
            bctx.mode("OLOGIC")
                .attr("OFF1", "#FF")
                .attr("TFF1", "#FF")
                .attr("REVINV", "REV")
                .attr("SRINV", "SR")
                .attr(oattr, "0")
                .pin("REV")
                .pin("SR")
                .test_enum(attr, &["0"]);
        }

        bctx.mode("OLOGIC")
            .test_enum_suffix("INIT_OQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OLOGIC")
            .test_enum_suffix("INIT_TQ", "OLOGIC", &["0", "1"]);
        bctx.mode("OSERDES")
            .test_enum_suffix("INIT_OQ", "OSERDES", &["0", "1"]);
        bctx.mode("OSERDES")
            .test_enum_suffix("INIT_TQ", "OSERDES", &["0", "1"]);

        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("D2INV", "#OFF")
            .attr("OMUX", "OFF1")
            .pin("OQ")
            .test_enum_suffix("SRVAL_OQ", "OFF1", &["0", "1"]);
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("D2INV", "D2")
            .attr("OMUX", "OFFDDRA")
            .pin("D2")
            .pin("OQ")
            .test_enum_suffix("SRVAL_OQ", "OFFDDRA", &["0", "1"]);
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("D2INV", "D2")
            .attr("OMUX", "OFFDDRB")
            .pin("D2")
            .pin("OQ")
            .test_enum_suffix("SRVAL_OQ", "OFFDDRB", &["0", "1"]);
        bctx.mode("OLOGIC")
            .attr("TFF1", "#FF")
            .attr("T2INV", "#OFF")
            .attr("TMUX", "TFF1")
            .pin("TQ")
            .test_enum_suffix("SRVAL_TQ", "TFF1", &["0", "1"]);
        bctx.mode("OLOGIC")
            .attr("TFF1", "#FF")
            .attr("T2INV", "T2")
            .attr("TMUX", "TFFDDRA")
            .pin("T2")
            .pin("TQ")
            .test_enum_suffix("SRVAL_TQ", "TFFDDRA", &["0", "1"]);
        bctx.mode("OLOGIC")
            .attr("TFF1", "#FF")
            .attr("T2INV", "T2")
            .attr("TMUX", "TFFDDRB")
            .pin("T2")
            .pin("TQ")
            .test_enum_suffix("SRVAL_TQ", "TFFDDRB", &["0", "1"]);
        bctx.mode("OSERDES")
            .test_enum_suffix("SRVAL_OQ", "OSERDES", &["0", "1"]);
        bctx.mode("OSERDES")
            .test_enum_suffix("SRVAL_TQ", "OSERDES", &["0", "1"]);

        bctx.mode("OLOGIC")
            .attr("SRINV", "#OFF")
            .attr("REVINV", "#OFF")
            .attr("OSRUSED", "#OFF")
            .attr("OREVUSED", "#OFF")
            .attr("OFF1", "#FF")
            .attr("O1USED", "0")
            .attr("D1INV", "D1")
            .pin("D1")
            .pin("OQ")
            .test_enum("OMUX", &["D1", "OFF1", "OFFDDRA", "OFFDDRB"]);
        bctx.mode("OLOGIC")
            .attr("SRINV", "#OFF")
            .attr("REVINV", "#OFF")
            .attr("TSRUSED", "#OFF")
            .attr("TREVUSED", "#OFF")
            .attr("TFF1", "#FF")
            .attr("T1USED", "0")
            .attr("T1INV", "T1")
            .pin("T1")
            .pin("TQ")
            .test_enum("TMUX", &["T1", "TFF1", "TFFDDRA", "TFFDDRB"]);

        bctx.mode("OSERDES")
            .attr("DATA_WIDTH", "2")
            .test_enum("SERDES", &["FALSE", "TRUE"]);
        bctx.mode("OSERDES")
            .test_enum("SERDES_MODE", &["SLAVE", "MASTER"]);
        bctx.mode("OSERDES")
            .test_enum("DDR_CLK_EDGE", &["SAME_EDGE", "OPPOSITE_EDGE"]);
        bctx.mode("OSERDES").test_enum("SRTYPE", &["SYNC", "ASYNC"]);
        bctx.mode("OSERDES")
            .test_enum("DATA_RATE_OQ", &["SDR", "DDR"]);
        bctx.mode("OSERDES")
            .attr("TCEINV", "TCE_B")
            .attr("T1INV", "T1")
            .pin("TCE")
            .pin("T1")
            .test_enum("DATA_RATE_TQ", &["BUF", "SDR", "DDR"]);
        bctx.mode("OSERDES")
            .test_enum("TRISTATE_WIDTH", &["1", "2", "4"]);
        bctx.mode("OSERDES")
            .attr("SERDES", "TRUE")
            .test_enum("DATA_WIDTH", &["2", "3", "4", "5", "6", "7", "8", "10"]);
        bctx.mode("OSERDES").test_multi_attr_bin("INIT_LOADCNT", 4);

        bctx.build()
            .mutex("MUX.CLK", "CKINT")
            .test_manual("MUX.CLK", "CKINT")
            .pip("CLKMUX", "CLKMUX_INT")
            .commit();
        for ipin in [
            "HCLK0", "HCLK1", "HCLK2", "HCLK3", "HCLK4", "HCLK5", "HCLK6", "HCLK7", "RCLK0",
            "RCLK1", "IOCLK0", "IOCLK1", "IOCLK_S0", "IOCLK_S1", "IOCLK_N0", "IOCLK_N1",
        ] {
            bctx.build()
                .mutex("MUX.CLK", ipin)
                .test_manual("MUX.CLK", ipin)
                .pip("CLKMUX", (defs::bslots::IOI, ipin))
                .commit();
        }
    }
    for i in 0..2 {
        let bel = defs::bslots::IOB[i];
        let mut bctx = ctx.bel(bel);
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
        bctx.mode("IOB").test_enum("GTSATTRBOX", &["DISABLE_GTS"]);
        bctx.build()
            .mode("IOB")
            .pin("O")
            .attr("IOATTRBOX", "")
            .test_manual("OUSED", "0")
            .attr("DRIVE_0MA", "DRIVE_0MA")
            .attr("OUSED", "0")
            .commit();
        bctx.mode("IOB")
            .attr("OUSED", "0")
            .pin("O")
            .test_multi_attr_bin("OPROGRAMMING", 22);
        for &std in IOSTDS {
            let mut vref_special = None;
            let mut dci_special = None;
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
                dci_special = Some(Dci(std.name));
            }
            if std.diff != DiffKind::None {
                bctx.mode("IOB")
                    .attr("OUSED", "")
                    .pin("I")
                    .pin("DIFFI_IN")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .maybe_prop(dci_special)
                    .test_manual("ISTD", std.name)
                    .attr("INBUFUSED", "0")
                    .attr("DIFFI_INUSED", "0")
                    .attr("IOATTRBOX", std.name)
                    .commit();
                if std.diff == DiffKind::True && std.dci == DciKind::None {
                    bctx.mode("IOB")
                        .attr("OUSED", "")
                        .pin("I")
                        .pin("DIFFI_IN")
                        .attr("INBUFUSED", "0")
                        .attr("DIFFI_INUSED", "0")
                        .attr("IOATTRBOX", std.name)
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .maybe_prop(dci_special)
                        .test_manual("DIFF_TERM", std.name)
                        .attr("DIFF_TERM", "TRUE")
                        .commit();
                }
            } else {
                bctx.mode("IOB")
                    .attr("OUSED", "")
                    .pin("I")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .maybe_prop(vref_special)
                    .maybe_prop(dci_special)
                    .test_manual("ISTD", std.name)
                    .attr("INBUFUSED", "0")
                    .attr("IOATTRBOX", std.name)
                    .commit();
            }
        }
        for &std in IOSTDS {
            if std.diff == DiffKind::True {
                if i == 0 {
                    bctx.build()
                        .attr("INBUFUSED", "")
                        .attr("OPROGRAMMING", "")
                        .attr("OUSED", "")
                        .pin("DIFFO_IN")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .prop(DiffOut(None))
                        .test_manual("OSTD", std.name)
                        .mode_diff("IOB", "IOBS")
                        .attr("IOATTRBOX", std.name)
                        .attr("OUTMUX", "1")
                        .attr("DIFFO_INUSED", "0")
                        .commit();
                } else {
                    bctx.build()
                        .pin("O")
                        .attr("INBUFUSED", "")
                        .attr("OPROGRAMMING", "")
                        .attr("OUSED", "0")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .prop(DiffOut(Some(std.name)))
                        .test_manual("OSTD", std.name)
                        .mode_diff("IOB", "IOBM")
                        .attr("IOATTRBOX", std.name)
                        .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                        .commit();
                }
            } else if matches!(
                std.dci,
                DciKind::Output
                    | DciKind::OutputHalf
                    | DciKind::BiSplit
                    | DciKind::BiSplitT
                    | DciKind::BiVcc
            ) {
                bctx.mode("IOB")
                    .pin("O")
                    .attr("INBUFUSED", "")
                    .attr("OPROGRAMMING", "")
                    .attr("OUSED", "0")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .prop(Dci(std.name))
                    .test_manual("OSTD", std.name)
                    .attr("IOATTRBOX", std.name)
                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                    .commit();
            } else if !std.drive.is_empty() {
                for &drive in std.drive {
                    for slew in ["SLOW", "FAST"] {
                        bctx.mode("IOB")
                            .pin("O")
                            .attr("INBUFUSED", "")
                            .attr("OPROGRAMMING", "")
                            .attr("OUSED", "0")
                            .test_manual("OSTD", format!("{name}.{drive}.{slew}", name = std.name))
                            .attr("IOATTRBOX", std.name)
                            .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                            .attr("DRIVEATTRBOX", drive)
                            .attr("SLEW", slew)
                            .commit();
                    }
                }
            } else {
                bctx.mode("IOB")
                    .pin("O")
                    .attr("INBUFUSED", "")
                    .attr("OPROGRAMMING", "")
                    .attr("OUSED", "0")
                    .test_manual("OSTD", std.name)
                    .attr("IOATTRBOX", std.name)
                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                    .commit();
            }
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .extra_tiles_by_bel(defs::bslots::DCI, "DCI")
        .test_manual("DCI", "QUIET", "1")
        .global_diff("DCIUPDATEMODE", "CONTINUOUS", "QUIET")
        .commit();
    for bank in [1, 2, 3, 4] {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        let cfg = edev.tile_cfg(die);
        let mut builder = ctx
            .build()
            .raw(Key::Package, &package.name)
            .extra_tile_attr_fixed(cfg, "MISC", "DCI_CLK_ENABLE", "1");

        // Find VR and IO rows, set up HCLKs.
        let (vr_row, io_row) = match bank {
            1 => {
                let mut row = chip.row_bufg() + 8;
                while row != edev.row_iobdcm.unwrap() - 16 {
                    let hclk_center =
                        CellCoord::new(die, edev.col_cfg, row).tile(defs::tslots::HCLK_BEL);
                    builder = builder.extra_tile_attr_fixed(
                        hclk_center,
                        "DCI",
                        "CASCADE_FROM_ABOVE",
                        "1",
                    );
                    row += 16;
                }
                let hclk_center =
                    CellCoord::new(die, edev.col_cfg, row).tile(defs::tslots::HCLK_BEL);
                builder = builder.extra_tile_attr_fixed(hclk_center, "DCI", "ENABLE", "1");
                (
                    if chip.row_bufg() == edev.row_iobdcm.unwrap() - 24 {
                        None
                    } else {
                        Some(edev.row_iobdcm.unwrap() - 16 - 2)
                    },
                    chip.row_bufg() + 8,
                )
            }
            2 => {
                let mut row = chip.row_bufg() - 8;
                while row != edev.row_dcmiob.unwrap() + 16 {
                    let hclk_center =
                        CellCoord::new(die, edev.col_cfg, row).tile(defs::tslots::HCLK_BEL);
                    builder = builder.extra_tile_attr_fixed(
                        hclk_center,
                        "DCI",
                        "CASCADE_FROM_BELOW",
                        "1",
                    );
                    row -= 16;
                }
                let hclk_center =
                    CellCoord::new(die, edev.col_cfg, row).tile(defs::tslots::HCLK_BEL);
                builder = builder.extra_tile_attr_fixed(hclk_center, "DCI", "ENABLE", "1");
                (
                    if chip.row_bufg() == edev.row_dcmiob.unwrap() + 24 {
                        None
                    } else {
                        Some(edev.row_dcmiob.unwrap() + 16 + 1)
                    },
                    chip.row_bufg() - 9,
                )
            }
            3 => {
                let hclk_iobdcm = CellCoord::new(die, edev.col_cfg, edev.row_iobdcm.unwrap())
                    .tile(defs::tslots::HCLK_BEL);
                builder = builder.extra_tile_attr_fixed(hclk_iobdcm, "DCI", "ENABLE", "1");
                (
                    Some(edev.row_iobdcm.unwrap() - 2),
                    edev.row_iobdcm.unwrap() - 1,
                )
            }
            4 => {
                let hclk_dcmiob = CellCoord::new(die, edev.col_cfg, edev.row_dcmiob.unwrap())
                    .tile(defs::tslots::HCLK_BEL);
                builder = builder.extra_tile_attr_fixed(hclk_dcmiob, "DCI", "ENABLE", "1");
                (Some(edev.row_dcmiob.unwrap() + 1), edev.row_dcmiob.unwrap())
            }
            _ => unreachable!(),
        };
        let vr_tile =
            vr_row.map(|row| CellCoord::new(die, edev.col_cfg, row).tile(defs::tslots::BEL));
        let io_tile = CellCoord::new(die, edev.col_cfg, io_row).tile(defs::tslots::BEL);

        // Ensure nothing is placed in VR.  Set up VR diff.
        if let Some(vr_tile) = vr_tile {
            for bel in [defs::bslots::IOB[0], defs::bslots::IOB[1]] {
                let site = backend.ngrid.get_bel_name(vr_tile.cell.bel(bel)).unwrap();
                builder = builder.raw(Key::SiteMode(site), None);
            }
            builder = builder.extra_tile_attr_fixed(vr_tile, "IOB_COMMON", "PRESENT", "VR_CENTER");
        }

        // Set up the IO and fire.
        let site = backend
            .ngrid
            .get_bel_name(io_tile.cell.bel(defs::bslots::IOB[0]))
            .unwrap();
        builder
            .raw(Key::SiteMode(site), "IOB")
            .raw(Key::SitePin(site, "O".into()), true)
            .raw(Key::SiteAttr(site, "INBUFUSED".into()), None)
            .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
            .raw(Key::SiteAttr(site, "OUSED".into()), "0")
            .raw_diff(Key::SiteAttr(site, "IOATTRBOX".into()), None, "LVDCI_33")
            .raw_diff(Key::SiteAttr(site, "DRIVE_0MA".into()), "DRIVE_0MA", None)
            // Take exclusive mutex on global DCI.
            .raw_diff(Key::GlobalMutex("GLOBAL_DCI".into()), None, "EXCLUSIVE")
            // Avoid interference.
            .raw(Key::GlobalOpt("MATCH_CYCLE".into()), "NOWAIT")
            .extra_tile_attr_fixed(io_tile, "IOB[0]", "OSTD", "LVDCI_33")
            .test_manual("MISC", format!("CENTER_DCI.{bank}"), "1")
            .commit();
    }

    for tile in [
        "HCLK_IO_DCI",
        "HCLK_IO_CENTER",
        "HCLK_IO_CFG_N",
        "HCLK_IO_DCM_S",
        "HCLK_IO_DCM_N",
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tile);
        let mut bctx = ctx.bel(defs::bslots::DCI);
        bctx.build()
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_manual("TEST_ENABLE", "1")
            .mode("DCI")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tile = "IO";
    for i in 0..2 {
        let bel = &format!("ILOGIC[{i}]");

        let mut present_ilogic = ctx.get_diff(tile, bel, "PRESENT", "ILOGIC");
        let mut present_iserdes = ctx.get_diff(tile, bel, "PRESENT", "ISERDES");

        ctx.collect_int_inv(&["INT"], tile, bel, "CLKDIV", false);
        ctx.collect_inv(tile, bel, "CE1");
        ctx.collect_inv(tile, bel, "CE2");
        for pin in ["SR", "REV"] {
            let diff0 = ctx.get_diff(tile, bel, format!("{pin}INV.O{pin}"), pin);
            let diff1 = ctx.get_diff(tile, bel, format!("{pin}INV.O{pin}"), format!("{pin}_B"));
            let item = xlat_bool(diff0, diff1);
            ctx.insert(tile, bel, format!("INV.{pin}"), item);
            let diff0 = ctx.get_diff(tile, bel, format!("{pin}INV.O{pin}_B"), format!("{pin}_B"));
            let diff1 = ctx.get_diff(tile, bel, format!("{pin}INV.O{pin}_B"), pin);
            let item = xlat_bool(diff0, diff1);
            ctx.insert(tile, bel, format!("INV.{pin}"), item);
        }

        let diff1 = ctx.get_diff(tile, bel, "OCLKINV.DDR", "OCLK_B");
        let diff2 = ctx.get_diff(tile, bel, "OCLKINV.DDR", "OCLK");
        ctx.get_diff(tile, bel, "OCLKINV.SDR", "OCLK")
            .assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "OCLKINV.SDR", "OCLK_B");
        diff = diff.combine(&!&diff1);
        diff = diff.combine(&!&diff2);
        diff.assert_empty();
        ctx.insert(tile, bel, "INV.OCLK1", xlat_bit(diff1));
        ctx.insert(tile, bel, "INV.OCLK2", xlat_bit(diff2));
        let item = ctx.extract_enum_bool_wide(tile, bel, "CLKINV", "CLK", "CLK_B");
        ctx.insert(tile, bel, "INV.CLK", item);

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        let item = ctx.extract_enum_bool(tile, bel, "IFF1", "#FF", "#LATCH");
        ctx.insert(tile, bel, "IFF_LATCH", item);
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum(tile, bel, "INTERFACE_TYPE", &["MEMORY", "NETWORKING"]);
        ctx.collect_enum(tile, bel, "NUM_CE", &["1", "2"]);
        ctx.collect_bitvec(tile, bel, "INIT_BITSLIPCNT", "");
        ctx.collect_bitvec(tile, bel, "INIT_CE", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK1_PARTIAL", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK2", "");
        ctx.collect_bitvec(tile, bel, "INIT_RANK3", "");
        let item = ctx.extract_enum_bool(tile, bel, "SRTYPE", "ASYNC", "SYNC");
        ctx.insert(tile, bel, "IFF_SR_SYNC", item);
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

        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10"] {
            let mut diff = ctx.get_diff(tile, bel, "DATA_WIDTH", val);
            diff.apply_bit_diff(ctx.item(tile, bel, "SERDES"), true, false);
            diffs.push((val, diff));
        }
        let mut bits = xlat_enum(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.insert(
            tile,
            bel,
            "DATA_WIDTH",
            xlat_enum_ocd(diffs, OcdMode::FixedOrder(&bits)),
        );

        let mut diffs = vec![];
        for val in ["SDR", "DDR"] {
            let mut diff = ctx.get_diff(tile, bel, "DATA_RATE", val);
            diff.apply_bit_diff(ctx.item(tile, bel, "IFF_LATCH"), false, true);
            diffs.push((val, diff));
        }
        ctx.insert(tile, bel, "DATA_RATE", xlat_enum(diffs));

        ctx.get_diff(tile, bel, "BITSLIP_ENABLE.ASYNC", "FALSE")
            .assert_empty();
        ctx.get_diff(tile, bel, "BITSLIP_ENABLE.SYNC", "FALSE")
            .assert_empty();
        let diff_async = ctx.get_diff(tile, bel, "BITSLIP_ENABLE.ASYNC", "TRUE");
        let diff_sync = ctx.get_diff(tile, bel, "BITSLIP_ENABLE.SYNC", "TRUE");
        let diff_sync = diff_sync.combine(&!&diff_async);
        ctx.insert(tile, bel, "BITSLIP_ENABLE", xlat_bit_wide(diff_async));
        ctx.insert(tile, bel, "BITSLIP_SYNC", xlat_bit(diff_sync));

        let mut diffs_a = vec![];
        let mut diffs_b = vec![];
        for diff in ctx.get_diffs(tile, bel, "IOBDELAY_VALUE", "") {
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
        ctx.insert(tile, bel, "IOBDELAY_VALUE_INIT", xlat_bitvec(diffs_a));
        ctx.insert(tile, bel, "IOBDELAY_VALUE_CUR", xlat_bitvec(diffs_b));

        let item = xlat_enum(vec![
            (
                "OPPOSITE_EDGE",
                ctx.get_diff(tile, bel, "Q2MUX.IFF3", "IFF2"),
            ),
            ("SAME_EDGE", ctx.get_diff(tile, bel, "Q1MUX.IFF4", "IFF1")),
            ("SAME_EDGE_PIPELINED", Diff::default()),
        ]);
        // wtf is even going on
        present_iserdes.apply_enum_diff(&item, "SAME_EDGE", "SAME_EDGE_PIPELINED");
        ctx.get_diff(tile, bel, "DDR_CLK_EDGE", "SAME_EDGE")
            .assert_empty();
        ctx.get_diff(tile, bel, "DDR_CLK_EDGE", "SAME_EDGE_PIPELINED")
            .assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "DDR_CLK_EDGE", "OPPOSITE_EDGE");
        diff.apply_enum_diff(&item, "OPPOSITE_EDGE", "SAME_EDGE");
        diff.assert_empty();
        ctx.get_diff(tile, bel, "Q1MUX.IFF2", "IFF1").assert_empty();
        ctx.get_diff(tile, bel, "Q1MUX.IFF4", "IFF3").assert_empty();
        ctx.get_diff(tile, bel, "Q2MUX.IFF3", "IFF4").assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "Q1MUX.IFF2", "IFF3");
        diff.apply_enum_diff(
            ctx.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "Q2MUX.IFF1", "IFF4");
        diff.apply_enum_diff(
            ctx.item(tile, bel, "INTERFACE_TYPE"),
            "NETWORKING",
            "MEMORY",
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "Q2MUX.IFF1", "IFF2");
        diff.apply_enum_diff(&item, "OPPOSITE_EDGE", "SAME_EDGE");
        diff.assert_empty();
        ctx.insert(tile, bel, "DDR_CLK_EDGE", item);

        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("D", ctx.get_diff(tile, bel, "IDELAYMUX", "1")),
            ("OFB", ctx.get_diff(tile, bel, "IDELAYMUX", "0")),
        ]);
        ctx.insert(tile, bel, "IDELAYMUX", item);
        let item = ctx.extract_enum(tile, bel, "D2OBYP_SEL", &["GND", "T"]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);
        let item = ctx.extract_enum(tile, bel, "D2OFFBYP_SEL", &["GND", "T"]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);
        // this seems wrong, and also it's opposite on v5 — bug?
        let item = xlat_enum(vec![
            ("GND", ctx.get_diff(tile, bel, "TFB_USED", "TRUE")),
            ("T", ctx.get_diff(tile, bel, "TFB_USED", "FALSE")),
        ]);
        ctx.insert(tile, bel, "TSBYPASS_MUX", item);

        let item = ctx.extract_enum_bool(tile, bel, "IDELMUX", "1", "0");
        ctx.insert(tile, bel, "I_DELAY_ENABLE", item);
        let item = ctx.extract_enum_bool(tile, bel, "IFFDELMUX", "1", "0");
        ctx.insert(tile, bel, "IFF_DELAY_ENABLE", item);

        ctx.get_diff(tile, bel, "IOBDELAY", "NONE").assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "IOBDELAY", "IBUF");
        diff.apply_bit_diff(ctx.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.apply_enum_diff(ctx.item(tile, bel, "IDELAYMUX"), "D", "NONE");
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "IOBDELAY", "IFD");
        diff.apply_bit_diff(ctx.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.apply_enum_diff(ctx.item(tile, bel, "IDELAYMUX"), "D", "NONE");
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "IOBDELAY", "BOTH");
        diff.apply_bit_diff(ctx.item(tile, bel, "I_DELAY_ENABLE"), true, false);
        diff.apply_bit_diff(ctx.item(tile, bel, "IFF_DELAY_ENABLE"), true, false);
        diff.apply_enum_diff(ctx.item(tile, bel, "IDELAYMUX"), "D", "NONE");
        diff.assert_empty();

        let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
        ctx.insert(tile, bel, "I_TSBYPASS_ENABLE", item);
        let diff0 = ctx.get_diff(tile, bel, "IFFMUX", "1");
        let diff1 = ctx.get_diff(tile, bel, "IFFMUX", "0");
        let (diff0, diff1, diff_common) = Diff::split(diff0, diff1);
        ctx.insert(tile, bel, "IFF_TSBYPASS_ENABLE", xlat_bool(diff0, diff1));
        present_iserdes = present_iserdes.combine(&!&diff_common);
        ctx.insert(tile, bel, "IFF_ENABLE", xlat_bit(diff_common));

        ctx.get_diff(tile, bel, "OFB_USED.NONE", "FALSE")
            .assert_empty();
        for attr in ["OFB_USED.IBUF", "OFB_USED.IFD", "OFB_USED.BOTH"] {
            let mut diff = ctx.get_diff(tile, bel, attr, "FALSE");
            diff.apply_enum_diff(ctx.item(tile, bel, "IDELAYMUX"), "D", "NONE");
            diff.assert_empty();
        }
        let mut diff = ctx.get_diff(tile, bel, "OFB_USED.NONE", "TRUE");
        diff.apply_enum_diff(ctx.item(tile, bel, "IDELAYMUX"), "OFB", "NONE");
        diff.apply_bit_diff(ctx.item(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.apply_bit_diff(ctx.item(tile, bel, "IFF_TSBYPASS_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "OFB_USED.IBUF", "TRUE");
        diff.apply_enum_diff(ctx.item(tile, bel, "IDELAYMUX"), "OFB", "NONE");
        diff.apply_bit_diff(ctx.item(tile, bel, "IFF_TSBYPASS_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "OFB_USED.IFD", "TRUE");
        diff.apply_enum_diff(ctx.item(tile, bel, "IDELAYMUX"), "OFB", "NONE");
        diff.apply_bit_diff(ctx.item(tile, bel, "I_TSBYPASS_ENABLE"), true, false);
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "OFB_USED.BOTH", "TRUE");
        diff.apply_enum_diff(ctx.item(tile, bel, "IDELAYMUX"), "OFB", "NONE");
        diff.assert_empty();

        let item = ctx.extract_enum(
            tile,
            bel,
            "IOBDELAY_TYPE.ILOGIC.IFD",
            &["DEFAULT", "FIXED", "VARIABLE"],
        );
        ctx.insert(tile, bel, "IOBDELAY_TYPE", item);
        let item = ctx.extract_enum(
            tile,
            bel,
            "IOBDELAY_TYPE.ISERDES.IFD",
            &["DEFAULT", "FIXED", "VARIABLE"],
        );
        ctx.insert(tile, bel, "IOBDELAY_TYPE", item);
        let item = ctx.extract_enum_default(
            tile,
            bel,
            "IOBDELAY_TYPE.ILOGIC.IBUF",
            &["FIXED", "VARIABLE"],
            "DEFAULT",
        );
        ctx.insert(tile, bel, "IOBDELAY_TYPE", item);
        let item = ctx.extract_enum_default(
            tile,
            bel,
            "IOBDELAY_TYPE.ISERDES.IBUF",
            &["FIXED", "VARIABLE"],
            "DEFAULT",
        );
        ctx.insert(tile, bel, "IOBDELAY_TYPE", item);

        // hm. not clear what's going on.
        let item = ctx.extract_bit(tile, bel, "IOBDELAY_TYPE.ILOGIC.IBUF", "DEFAULT");
        let mut diff = ctx.get_diff(tile, bel, "IOBDELAY_TYPE.ISERDES.IBUF", "DEFAULT");
        diff.apply_bit_diff(&item, true, false);
        diff.apply_bit_diff(ctx.item(tile, bel, "I_DELAY_ENABLE"), false, true);
        diff.assert_empty();
        ctx.insert(tile, bel, "I_DELAY_DEFAULT", item);

        present_ilogic.apply_bit_diff(ctx.item(tile, bel, "INV.CE1"), false, true);
        present_iserdes.apply_bit_diff(ctx.item(tile, bel, "INV.CE1"), false, true);
        present_ilogic.apply_bitvec_diff_int(ctx.item(tile, bel, "IOBDELAY_VALUE_CUR"), 0, 0x3f);
        present_iserdes.apply_bitvec_diff_int(ctx.item(tile, bel, "IOBDELAY_VALUE_CUR"), 0, 0x3f);

        present_ilogic.assert_empty();
        present_iserdes.assert_empty();

        ctx.insert(
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
        let bel = &format!("OLOGIC[{i}]");
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
            ctx.insert(tile, bel, format!("INV.{pin}"), item);
        }
        for pin in ["D1", "D2", "T1", "T2"] {
            let item = ctx.extract_enum_bool(
                tile,
                bel,
                &format!("{pin}INV.OLOGIC"),
                pin,
                &format!("{pin}_B"),
            );
            ctx.insert(tile, bel, format!("INV.{pin}"), item);
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
            let mut diff0 = ctx.get_diff(tile, bel, format!("{pin}INV.OLOGIC"), pin);
            let mut diff1 = ctx.get_diff(tile, bel, format!("{pin}INV.OLOGIC"), format!("{pin}_B"));
            diff0.apply_bit_diff(oused, true, false);
            diff1.apply_bit_diff(oused, true, false);
            ctx.insert_int_inv(&["INT"], tile, bel, pin, xlat_bool(diff0, diff1));
            let mut diff0 = ctx.get_diff(tile, bel, format!("{pin}INV.OSERDES"), pin);
            let mut diff1 =
                ctx.get_diff(tile, bel, format!("{pin}INV.OSERDES"), format!("{pin}_B"));
            diff0.apply_bit_diff(oused, true, false);
            diff1.apply_bit_diff(oused, true, false);
            diff0.apply_bit_diff(tused, true, false);
            diff1.apply_bit_diff(tused, true, false);
            ctx.insert_int_inv(&["INT"], tile, bel, pin, xlat_bool(diff0, diff1));
        }
        let clk1inv = ctx.extract_enum_bool(tile, bel, "CLK1INV.OLOGIC", "C", "C_B");
        let clk2inv = ctx.extract_enum_bool(tile, bel, "CLK2INV.OLOGIC", "CLK", "CLK_B");
        let mut diff = ctx.get_diff(tile, bel, "CLKINV.OSERDES.SAME", "CLK");
        diff.apply_bit_diff(&clk1inv, false, true);
        diff.apply_bit_diff(&clk2inv, false, true);
        diff.assert_empty();
        let diff = ctx.get_diff(tile, bel, "CLKINV.OSERDES.SAME", "CLK_B");
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "CLKINV.OSERDES.OPPOSITE", "CLK");
        diff.apply_bit_diff(&clk1inv, false, true);
        diff.assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "CLKINV.OSERDES.OPPOSITE", "CLK_B");
        diff.apply_bit_diff(&clk2inv, false, true);
        diff.assert_empty();
        ctx.insert(tile, bel, "INV.CLK1", clk1inv);
        ctx.insert(tile, bel, "INV.CLK2", clk2inv);
        ctx.get_diff(tile, bel, "DDR_CLK_EDGE", "SAME_EDGE")
            .assert_empty();
        ctx.get_diff(tile, bel, "DDR_CLK_EDGE", "OPPOSITE_EDGE")
            .assert_empty();

        ctx.insert(tile, bel, "OFF_SR_USED", osrused);
        ctx.insert(tile, bel, "TFF_SR_USED", tsrused);
        ctx.insert(tile, bel, "OFF_REV_USED", orevused);
        ctx.insert(tile, bel, "TFF_REV_USED", trevused);

        let item_oq = ctx.extract_enum_bool_wide(tile, bel, "SRTYPE_OQ", "ASYNC", "SYNC");
        let item_tq = ctx.extract_enum_bool_wide(tile, bel, "SRTYPE_TQ", "ASYNC", "SYNC");
        ctx.get_diff(tile, bel, "SRTYPE", "ASYNC").assert_empty();
        let mut diff = ctx.get_diff(tile, bel, "SRTYPE", "SYNC");
        diff.apply_bitvec_diff(&item_oq, &bits![1; 4], &bits![0; 4]);
        diff.apply_bitvec_diff(&item_tq, &bits![1; 2], &bits![0; 2]);
        diff.assert_empty();
        ctx.insert(tile, bel, "OFF_SR_SYNC", item_oq);
        ctx.insert(tile, bel, "TFF_SR_SYNC", item_tq);

        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("D1", ctx.get_diff(tile, bel, "OMUX", "D1")),
            ("OFF1", ctx.get_diff(tile, bel, "OMUX", "OFF1")),
            ("OFFDDR", ctx.get_diff(tile, bel, "OMUX", "OFFDDRA")),
            ("OFFDDR", ctx.get_diff(tile, bel, "OMUX", "OFFDDRB")),
        ]);
        ctx.insert(tile, bel, "OMUX", item);
        let item = xlat_enum(vec![
            ("NONE", Diff::default()),
            ("T1", ctx.get_diff(tile, bel, "TMUX", "T1")),
            ("TFF1", ctx.get_diff(tile, bel, "TMUX", "TFF1")),
            ("TFFDDR", ctx.get_diff(tile, bel, "TMUX", "TFFDDRA")),
            ("TFFDDR", ctx.get_diff(tile, bel, "TMUX", "TFFDDRB")),
            ("T1", ctx.get_diff(tile, bel, "DATA_RATE_TQ", "BUF")),
            ("TFF1", ctx.get_diff(tile, bel, "DATA_RATE_TQ", "SDR")),
            ("TFFDDR", ctx.get_diff(tile, bel, "DATA_RATE_TQ", "DDR")),
        ]);
        ctx.insert(tile, bel, "TMUX", item);
        let mut diff_sdr = ctx.get_diff(tile, bel, "DATA_RATE_OQ", "SDR");
        let mut diff_ddr = ctx.get_diff(tile, bel, "DATA_RATE_OQ", "DDR");
        diff_sdr.apply_enum_diff(ctx.item(tile, bel, "OMUX"), "OFF1", "D1");
        diff_ddr.apply_enum_diff(ctx.item(tile, bel, "OMUX"), "OFFDDR", "D1");
        assert_eq!(diff_sdr, diff_ddr);
        ctx.insert(tile, bel, "OFF_SERDES", xlat_bit_wide(diff_sdr));

        ctx.collect_enum_bool(tile, bel, "SERDES", "FALSE", "TRUE");
        ctx.collect_enum(tile, bel, "SERDES_MODE", &["SLAVE", "MASTER"]);
        ctx.collect_enum(tile, bel, "TRISTATE_WIDTH", &["1", "2", "4"]);
        ctx.collect_bitvec(tile, bel, "INIT_LOADCNT", "");

        let mut diffs = vec![("NONE", Diff::default())];
        for val in ["2", "3", "4", "5", "6", "7", "8", "10"] {
            let mut diff = ctx.get_diff(tile, bel, "DATA_WIDTH", val);
            diff.apply_bit_diff(ctx.item(tile, bel, "SERDES"), true, false);
            diffs.push((val, diff));
        }
        ctx.insert(tile, bel, "DATA_WIDTH", xlat_enum(diffs));

        let item = ctx.extract_enum_bool(tile, bel, "OFF1", "#FF", "#LATCH");
        ctx.insert(tile, bel, "OFF_LATCH", item);
        let item = ctx.extract_enum_bool(tile, bel, "TFF1", "#FF", "#LATCH");
        ctx.insert(tile, bel, "TFF_LATCH", item);

        let diff_ologic = ctx.get_diff(tile, bel, "INIT_OQ.OLOGIC", "0");
        let diff_oserdes = ctx
            .get_diff(tile, bel, "INIT_OQ.OSERDES", "0")
            .combine(&!&diff_ologic);
        ctx.insert(tile, bel, "OFF_INIT", xlat_bit_wide(!diff_ologic));
        ctx.insert(tile, bel, "OFF_INIT_SERDES", xlat_bit_wide(!diff_oserdes));
        ctx.get_diff(tile, bel, "INIT_OQ.OLOGIC", "1")
            .assert_empty();
        ctx.get_diff(tile, bel, "INIT_OQ.OSERDES", "1")
            .assert_empty();
        let item = ctx.extract_enum_bool_wide(tile, bel, "INIT_TQ.OLOGIC", "0", "1");
        ctx.insert(tile, bel, "TFF_INIT", item);
        let item = ctx.extract_enum_bool_wide(tile, bel, "INIT_TQ.OSERDES", "0", "1");
        ctx.insert(tile, bel, "TFF_INIT", item);
        for attr in [
            "SRVAL_OQ.OFF1",
            "SRVAL_OQ.OFFDDRA",
            "SRVAL_OQ.OFFDDRB",
            "SRVAL_OQ.OSERDES",
        ] {
            let item = ctx.extract_enum_bool_wide(tile, bel, attr, "0", "1");
            ctx.insert(tile, bel, "OFF_SRVAL", item);
        }

        for attr in [
            "SRVAL_TQ.TFF1",
            "SRVAL_TQ.TFFDDRA",
            "SRVAL_TQ.TFFDDRB",
            "SRVAL_TQ.OSERDES",
        ] {
            ctx.get_diff(tile, bel, attr, "1").assert_empty();
        }
        let diff1 = ctx.get_diff(tile, bel, "SRVAL_TQ.TFF1", "0");
        let diff2 = ctx.get_diff(tile, bel, "SRVAL_TQ.TFFDDRA", "0");
        let diff3 = ctx.get_diff(tile, bel, "SRVAL_TQ.TFFDDRB", "0");
        let diff4 = ctx.get_diff(tile, bel, "SRVAL_TQ.OSERDES", "0");
        assert_eq!(diff3, diff4);
        let diff3 = diff3.combine(&!&diff2);
        let diff2 = diff2.combine(&!&diff1);
        ctx.insert(tile, bel, "TFF1_SRVAL", xlat_bit(!diff1));
        ctx.insert(tile, bel, "TFF2_SRVAL", xlat_bit(!diff2));
        ctx.insert(tile, bel, "TFF3_SRVAL", xlat_bit(!diff3));

        let mut present_ologic = ctx.get_diff(tile, bel, "PRESENT", "OLOGIC");
        let mut present_oserdes = ctx.get_diff(tile, bel, "PRESENT", "OSERDES");
        present_ologic.apply_enum_diff(ctx.item(tile, bel, "TMUX"), "T1", "NONE");
        present_oserdes.apply_enum_diff(ctx.item(tile, bel, "OMUX"), "D1", "NONE");
        present_oserdes.apply_enum_diff(ctx.item(tile, bel, "TMUX"), "T1", "NONE");
        present_oserdes.apply_bit_diff(ctx.item(tile, bel, "INV.D1"), false, true);
        present_ologic.assert_empty();
        present_oserdes.assert_empty();
    }
    let mut present_vr = ctx.get_diff(tile, "IOB_COMMON", "PRESENT", "VR");
    // I don't care.
    ctx.get_diff(tile, "IOB_COMMON", "PRESENT", "VR_CENTER");
    for i in 0..2 {
        let bel = &format!("IOB[{i}]");
        let mut present = ctx.get_diff(tile, bel, "PRESENT", "IOB");
        ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
        let item = ctx.extract_bit_wide(tile, bel, "OUSED", "0");
        assert_eq!(item.bits.len(), 2);
        ctx.insert(tile, bel, "OUTPUT_ENABLE", item);
        ctx.get_diff(tile, bel, "GTSATTRBOX", "DISABLE_GTS")
            .assert_empty();
        let diff = ctx
            .get_diff(tile, bel, "PRESENT", "IPAD")
            .combine(&!&present);
        ctx.insert(tile, bel, "VREF_SYSMON", xlat_bit(diff));
        let diff = ctx
            .get_diff(tile, bel, "PRESENT", "IOB.CONTINUOUS")
            .combine(&!&present);
        ctx.insert(tile, bel, "DCIUPDATEMODE_ASREQUIRED", xlat_bit(!diff));
        present.apply_enum_diff(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

        let oprog = ctx.extract_bitvec(tile, bel, "OPROGRAMMING", "");
        let lvds = TileItem::from_bitvec(oprog.bits[0..4].to_vec(), false);
        let dci_t = TileItem::from_bit(oprog.bits[4], false);
        let dci_mode = TileItem {
            bits: oprog.bits[5..8].to_vec(),
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
        let pslew_invert = bits![0, 0, 0, 0];
        let nslew_invert = bits![0, 1, 0, 0];

        let mut ibuf_mode = vec![("OFF", Diff::default())];

        for &std in IOSTDS {
            let mut diff = ctx.get_diff(tile, bel, "ISTD", std.name);
            match std.dci {
                DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                DciKind::InputVcc | DciKind::BiVcc => {
                    diff.apply_enum_diff(&dci_mode, "TERM_VCC", "NONE");
                    diff.apply_bitvec_diff(&dci_misc, &bits![1, 1], &bits![0, 0]);
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
                let diff = ctx.get_diff(tile, bel, "OSTD", std.name);
                let value = extract_bitvec_val(&lvds, &bits![0; 4], diff);
                let tc = ['C', 'T'][i];
                ctx.insert_misc_data(format!("IOSTD:LVDS_{tc}:OUTPUT_{stdname}"), value);
                if std.dci == DciKind::None {
                    let diff = ctx.get_diff(tile, bel, "DIFF_TERM", std.name);
                    let value = extract_bitvec_val(&lvds, &bits![0; 4], diff);
                    let tc = ['C', 'T'][i];
                    ctx.insert_misc_data(format!("IOSTD:LVDS_{tc}:TERM_{stdname}"), value);
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
                        let mut diff = ctx.get_diff(tile, bel, "OSTD", val);
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
                        for (attr, bits, invert) in [
                            ("PSLEW", &pslew_bits, &pslew_invert),
                            ("NSLEW", &nslew_bits, &nslew_invert),
                        ] {
                            let value: BitVec = bits
                                .iter()
                                .zip(invert.iter())
                                .map(|(&bit, inv)| match diff.bits.remove(&bit) {
                                    Some(true) => !inv,
                                    None => inv,
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
                                diff.apply_enum_diff(&dci_mode, "OUTPUT", "NONE");
                            }
                            DciKind::OutputHalf => {
                                diff.apply_enum_diff(&dci_mode, "OUTPUT_HALF", "NONE");
                            }
                            DciKind::BiVcc => {
                                diff.apply_enum_diff(&dci_mode, "TERM_VCC", "NONE");
                                diff.apply_bitvec_diff(&dci_misc, &bits![1, 1], &bits![0, 0]);
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
        ctx.insert(tile, bel, "IBUF_MODE", xlat_enum(ibuf_mode));

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
                    Some(true) => !inv,
                    None => inv,
                    _ => unreachable!(),
                })
                .collect();
            ctx.insert_misc_data(format!("IOSTD:{attr}:VR"), value);
        }
        present_vr.apply_enum_diff(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");
        present_vr.apply_enum_diff(&dci_mode, "TERM_SPLIT", "NONE");
        if i == 0 {
            let mut present_vref = ctx.get_diff(tile, bel, "PRESENT", "VREF");
            present_vref.apply_bit_diff(ctx.item(tile, bel, "VREF_SYSMON"), true, false);
            present_vref.apply_enum_diff(ctx.item(tile, bel, "PULL"), "NONE", "PULLDOWN");

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
                        Some(true) => !inv,
                        None => inv,
                        _ => unreachable!(),
                    })
                    .collect();
                ctx.insert_misc_data(format!("IOSTD:{attr}:VREF"), value);
            }
            present_vref.assert_empty();
        }

        ctx.insert_misc_data("IOSTD:OUTPUT_MISC:OFF", bits![0; 2]);
        ctx.insert_misc_data("IOSTD:LVDS_T:OFF", bits![0; 4]);
        ctx.insert_misc_data("IOSTD:LVDS_C:OFF", bits![0; 4]);
        ctx.insert_misc_data("IOSTD:PDRIVE:OFF", bits![0; 5]);
        ctx.insert_misc_data("IOSTD:NDRIVE:OFF", bits![0; 5]);
        ctx.insert_misc_data("IOSTD:PSLEW:OFF", pslew_invert.clone());
        ctx.insert_misc_data("IOSTD:NSLEW:OFF", nslew_invert.clone());
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
        ctx.insert(
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
        ctx.insert(
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
    let diff1 = present_vr.split_bits_by(|bit| bit.bit.to_idx() >= 40);
    ctx.insert(tile, "IOB[0]", "VR", xlat_bit(present_vr));
    ctx.insert(tile, "IOB[1]", "VR", xlat_bit(diff1));

    let tile = "HCLK_IO_LVDS";
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
            let diff = ctx.get_diff(tile, bel, "STD", std.name);
            let val = extract_bitvec_val(&item, &bits![0; 10], diff);
            ctx.insert_misc_data(format!("IOSTD:LVDSBIAS:{}", std.name), val);
        }
    }
    ctx.insert(tile, bel, "LVDSBIAS", item);
    ctx.insert_misc_data("IOSTD:LVDSBIAS:OFF", bits![0; 10]);

    let hclk_center_cnt = ctx.edev.tile_index[ctx.edev.db.get_tile_class("HCLK_IO_CENTER")].len();
    for tile in [
        "HCLK_IO_DCI",
        "HCLK_IO_CENTER",
        "HCLK_IO_CFG_N",
        "HCLK_IO_DCM_S",
        "HCLK_IO_DCM_N",
    ] {
        let bel = "DCI";
        ctx.insert(
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
        ctx.insert(
            tile,
            bel,
            "NREF",
            TileItem::from_bitvec(
                vec![TileBit::new(0, 27, 15), TileBit::new(0, 27, 12)],
                false,
            ),
        );
        ctx.insert(
            tile,
            bel,
            "LVDIV2",
            TileItem::from_bitvec(
                vec![TileBit::new(0, 27, 13), TileBit::new(0, 27, 14)],
                false,
            ),
        );
        ctx.insert(
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
        ctx.insert(
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
        ctx.insert(
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

        let enable = if (tile == "HCLK_IO_CFG_N" && hclk_center_cnt != 1) || tile == "HCLK_IO_DCI" {
            TileItem::from_bit(TileBit::new(0, 0, 14), false)
        } else {
            ctx.extract_bit(tile, bel, "ENABLE", "1")
        };
        let mut test_enable = ctx.get_diff(tile, bel, "TEST_ENABLE", "1");
        test_enable.apply_bit_diff(&enable, true, false);
        ctx.insert(tile, bel, "ENABLE", enable);
        ctx.insert(tile, bel, "TEST_ENABLE", xlat_bit_wide(test_enable));
        if tile == "HCLK_IO_CENTER" {
            if hclk_center_cnt > 1 {
                ctx.collect_bit(tile, bel, "CASCADE_FROM_BELOW", "1");
            }
            if hclk_center_cnt > 3 {
                ctx.collect_bit(tile, bel, "CASCADE_FROM_ABOVE", "1");
            }
        }
        if tile == "HCLK_IO_CFG_N" && hclk_center_cnt > 1 {
            ctx.collect_bit(tile, bel, "CASCADE_FROM_ABOVE", "1");
        }
    }
    let tile = "HCLK_IO_DCI";
    let bel = "DCI";
    for std in IOSTDS {
        if std.dci == DciKind::None {
            continue;
        }
        let stdname = std.name.strip_prefix("DIFF_").unwrap_or(std.name);
        let mut diff = ctx.get_diff(tile, bel, "STD", std.name);
        match std.dci {
            DciKind::OutputHalf => {
                let val =
                    extract_bitvec_val_part(ctx.item(tile, bel, "LVDIV2"), &bits![0; 2], &mut diff);
                ctx.insert_misc_data(format!("IOSTD:DCI:LVDIV2:{stdname}"), val);
            }
            DciKind::InputVcc | DciKind::BiVcc => {
                let val = extract_bitvec_val_part(
                    ctx.item(tile, bel, "PMASK_TERM_VCC"),
                    &bits![0; 5],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_VCC:{stdname}"), val);
            }
            DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                let val = extract_bitvec_val_part(
                    ctx.item(tile, bel, "PMASK_TERM_SPLIT"),
                    &bits![0; 5],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("IOSTD:DCI:PMASK_TERM_SPLIT:{stdname}"), val);
                let val = extract_bitvec_val_part(
                    ctx.item(tile, bel, "NMASK_TERM_SPLIT"),
                    &bits![0; 5],
                    &mut diff,
                );
                ctx.insert_misc_data(format!("IOSTD:DCI:NMASK_TERM_SPLIT:{stdname}"), val);
            }
            _ => {}
        }
        ctx.insert(tile, bel, "ENABLE", xlat_bit(diff));
    }
    ctx.insert_misc_data("IOSTD:DCI:LVDIV2:OFF", bits![0; 2]);
    ctx.insert_misc_data("IOSTD:DCI:PMASK_TERM_VCC:OFF", bits![0; 5]);
    ctx.insert_misc_data("IOSTD:DCI:PMASK_TERM_SPLIT:OFF", bits![0; 5]);
    ctx.insert_misc_data("IOSTD:DCI:NMASK_TERM_SPLIT:OFF", bits![0; 5]);
    let tile = "CFG";
    let bel = "MISC";
    ctx.collect_bit_wide(tile, bel, "DCI_CLK_ENABLE", "1");
}
