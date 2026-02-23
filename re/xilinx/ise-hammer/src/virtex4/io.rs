use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, BelSlotId, TableRowId},
    grid::{DieId, DieIdExt, RowId, TileCoord, TileIobId},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, SpecialId, extract_bitvec_val, extract_bitvec_val_part, xlat_bit,
    xlat_bit_bi, xlat_bit_wide, xlat_bit_wide_bi, xlat_bitvec, xlat_enum_attr, xlat_enum_attr_ocd,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, FuzzerValue, Session};
use prjcombine_re_xilinx_geom::{ExpandedBond, ExpandedDevice};
use prjcombine_types::{bits, bitvec::BitVec, bsdata::TileBit};
use prjcombine_virtex4::{
    defs::{
        bcls::{self, ILOGIC, IOB},
        bslots, enums, tslots,
        virtex4::{
            tables::{IOB_DATA, LVDS_DATA},
            tcls,
        },
    },
    expanded::IoCoord,
};

use crate::{
    backend::{IseBackend, Key, MultiValue, Value},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        iostd::{DciKind, DiffKind, Iostd},
        props::DynProp,
    },
    virtex4::specials,
};

const IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8, 12, 16]),
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

fn get_lvds_row(edev: &prjcombine_virtex4::expanded::ExpandedDevice, iostd: &Iostd) -> TableRowId {
    let name = match iostd.name {
        "LDT_25" => "HT_25",
        "ULVDS_25" => "MINI_LVDS_25",
        _ => iostd.name,
    };
    edev.db[LVDS_DATA].rows.get(name).unwrap().0
}

fn get_istd_row(
    edev: &prjcombine_virtex4::expanded::ExpandedDevice,
    iostd: &Iostd,
) -> (SpecialId, TableRowId) {
    if iostd.diff == DiffKind::True && iostd.dci == DciKind::None {
        (specials::IOB_ISTD_LVDS, get_lvds_row(edev, iostd))
    } else if let Some(name) = iostd.name.strip_prefix("DIFF_") {
        (
            specials::IOB_ISTD_DIFF,
            edev.db[IOB_DATA].rows.get(name).unwrap().0,
        )
    } else if iostd.drive.is_empty() {
        (
            specials::IOB_ISTD,
            edev.db[IOB_DATA].rows.get(iostd.name).unwrap().0,
        )
    } else {
        (
            specials::IOB_ISTD,
            edev.db[IOB_DATA]
                .rows
                .get(&format!("{}_2", iostd.name))
                .unwrap()
                .0,
        )
    }
}

fn get_ostd_row(
    edev: &prjcombine_virtex4::expanded::ExpandedDevice,
    iostd: &Iostd,
    drive: u8,
    slew: &str,
) -> (SpecialId, TableRowId) {
    if let Some(name) = iostd.name.strip_prefix("DIFF_") {
        (
            specials::IOB_OSTD_DIFF,
            edev.db[IOB_DATA].rows.get(name).unwrap().0,
        )
    } else if iostd.drive.is_empty() {
        (
            specials::IOB_OSTD,
            edev.db[IOB_DATA].rows.get(iostd.name).unwrap().0,
        )
    } else if slew == "SLOW" {
        (
            specials::IOB_OSTD_SLOW,
            edev.db[IOB_DATA]
                .rows
                .get(&format!("{std}_{drive}", std = iostd.name))
                .unwrap()
                .0,
        )
    } else {
        (
            specials::IOB_OSTD_FAST,
            edev.db[IOB_DATA]
                .rows
                .get(&format!("{std}_{drive}", std = iostd.name))
                .unwrap()
                .0,
        )
    }
}

fn get_vrefs(backend: &IseBackend, tcrd: TileCoord) -> Vec<TileCoord> {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let chip = edev.chips[tcrd.die];

    let row_cfg = chip.row_reg_bot(chip.reg_cfg);
    let rows = if Some(tcrd.col) == edev.col_io_w || Some(tcrd.col) == edev.col_io_e {
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
        .map(|vref_row| tcrd.with_row(vref_row).tile(tslots::BEL))
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
            iob: TileIobId::from_idx(bslots::IOB.into_iter().position(|x| x == self.0).unwrap()),
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
                .get_bel_name(tcrd_vref.cell.bel(bslots::IOB[0]))
                .unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelSpecial(tcls::IO, bslots::IOB[0], specials::IOB_VREF),
                rects: backend.edev.tile_bits(tcrd_vref),
            });
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
struct Dci(SpecialId, TableRowId);

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
        let tile_vr = cell_vr.tile(tslots::BEL);
        for bel in [bslots::IOB[0], bslots::IOB[1]] {
            let site = backend.ngrid.get_bel_name(cell_vr.bel(bel)).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Test VR.
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::BelSpecial(tcls::IO, bslots::IOB[0], specials::IOB_VR),
            rects: edev.tile_bits(tile_vr),
        });
        // Take exclusive mutex on bank DCI.
        let hclk_iois_dci = cell_vr.delta(0, -1).tile(tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_iois_dci, "BANK_DCI".to_string()),
            None,
            "EXCLUSIVE",
        );
        // Test bank DCI.
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::BelSpecialRow(tcls::HCLK_IO_DCI, bslots::DCI, self.0, self.1),
            rects: edev.tile_bits(hclk_iois_dci),
        });
        // Take shared mutex on global DCI.
        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
        // Anchor global DCI by putting something in bottom IOB of center column.
        let iob_center = tcrd
            .cell
            .with_cr(edev.col_cfg, edev.row_dcmiob.unwrap())
            .bel(bslots::IOB[0]);
        let site = backend.ngrid.get_bel_name(iob_center).unwrap();
        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
        fuzzer = fuzzer.base(Key::SiteAttr(site, "IOATTRBOX".into()), "LVDCI_33");
        // Ensure anchor VR IOBs are free.
        for bel in [bslots::IOB[0], bslots::IOB[1]] {
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
struct DiffOut(Option<TableRowId>);

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
            .tile(tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_iois_lvds, "BANK_LVDS".to_string()),
            None,
            "EXCLUSIVE",
        );
        if let Some(row) = self.0 {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelSpecialRow(
                    tcls::HCLK_IO_LVDS,
                    bslots::LVDS,
                    specials::IOB_OSTD_LVDS,
                    row,
                ),
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
    let mut ctx = FuzzCtx::new(session, backend, tcls::IO);

    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::ILOGIC[i]);
        let bel_ologic = bslots::OLOGIC[i];
        bctx.build()
            .bel_unused(bel_ologic)
            .test_bel_special(specials::ILOGIC)
            .mode("ILOGIC")
            .commit();
        bctx.build()
            .bel_unused(bel_ologic)
            .test_bel_special(specials::ISERDES)
            .mode("ISERDES")
            .commit();

        bctx.mode("ILOGIC")
            .attr("IMUX", "1")
            .attr("IDELAYMUX", "1")
            .attr("IDELMUX", "0")
            .test_bel_input_inv_auto(ILOGIC::CLKDIV);
        bctx.mode("ISERDES").test_bel_input_inv_auto(ILOGIC::CLKDIV);

        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .test_bel_input_inv_auto(ILOGIC::CLK);
        bctx.mode("ISERDES").test_bel_input_inv_auto(ILOGIC::CLK);

        for (dr, val, spec) in [
            ("SDR", "OCLK", specials::ISERDES_OCLK_SDR),
            ("SDR", "OCLK_B", specials::ISERDES_OCLK_B_SDR),
            ("DDR", "OCLK", specials::ISERDES_OCLK_DDR),
            ("DDR", "OCLK_B", specials::ISERDES_OCLK_B_DDR),
        ] {
            bctx.mode("ISERDES")
                .attr("INTERFACE_TYPE", "MEMORY")
                .attr("DATA_RATE", dr)
                .pin("OCLK")
                .test_bel_special(spec)
                .attr("OCLKINV", val)
                .commit();
        }

        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .test_bel_input_inv_auto(ILOGIC::CE1);
        bctx.mode("ISERDES")
            .attr("INIT_CE", "11")
            .test_bel_input_inv_auto(ILOGIC::CE1);
        bctx.mode("ISERDES")
            .attr("INIT_CE", "11")
            .test_bel_input_inv_auto(ILOGIC::CE2);

        for (val, vname) in [(false, "SR"), (true, "SR_B")] {
            bctx.mode("ILOGIC")
                .attr("IFF1", "#FF")
                .pin("SR")
                .bel_mode(bel_ologic, "OSERDES")
                .bel_attr(bel_ologic, "SRINV", "SR")
                .bel_pin(bel_ologic, "SR")
                .test_bel_input_inv_special(ILOGIC::SR, specials::ILOGIC_OSR, val)
                .attr("SRINV", vname)
                .commit();
            bctx.mode("ILOGIC")
                .attr("IFF1", "#FF")
                .pin("SR")
                .bel_mode(bel_ologic, "OSERDES")
                .bel_attr(bel_ologic, "SRINV", "SR_B")
                .bel_pin(bel_ologic, "SR")
                .test_bel_input_inv_special(ILOGIC::SR, specials::ILOGIC_OSR_B, val)
                .attr("SRINV", vname)
                .commit();
            bctx.mode("ISERDES")
                .pin("SR")
                .bel_mode(bel_ologic, "OSERDES")
                .bel_attr(bel_ologic, "SRINV", "SR")
                .bel_pin(bel_ologic, "SR")
                .test_bel_input_inv_special(ILOGIC::SR, specials::ILOGIC_OSR, val)
                .attr("SRINV", vname)
                .commit();
            bctx.mode("ISERDES")
                .pin("SR")
                .bel_mode(bel_ologic, "OSERDES")
                .bel_attr(bel_ologic, "SRINV", "SR_B")
                .bel_pin(bel_ologic, "SR")
                .test_bel_input_inv_special(ILOGIC::SR, specials::ILOGIC_OSR_B, val)
                .attr("SRINV", vname)
                .commit();
        }

        for (val, vname) in [(false, "REV"), (true, "REV_B")] {
            bctx.mode("ILOGIC")
                .attr("IFF1", "#FF")
                .pin("REV")
                .bel_mode(bel_ologic, "OSERDES")
                .bel_attr(bel_ologic, "REVINV", "REV")
                .bel_pin(bel_ologic, "REV")
                .test_bel_input_inv_special(ILOGIC::REV, specials::ILOGIC_OSR, val)
                .attr("REVINV", vname)
                .commit();
            bctx.mode("ILOGIC")
                .attr("IFF1", "#FF")
                .pin("REV")
                .bel_mode(bel_ologic, "OSERDES")
                .bel_attr(bel_ologic, "REVINV", "REV_B")
                .bel_pin(bel_ologic, "REV")
                .test_bel_input_inv_special(ILOGIC::REV, specials::ILOGIC_OSR_B, val)
                .attr("REVINV", vname)
                .commit();
            bctx.mode("ISERDES")
                .pin("REV")
                .bel_mode(bel_ologic, "OSERDES")
                .bel_attr(bel_ologic, "REVINV", "REV")
                .bel_pin(bel_ologic, "REV")
                .test_bel_input_inv_special(ILOGIC::REV, specials::ILOGIC_OSR, val)
                .attr("REVINV", vname)
                .commit();
            bctx.mode("ISERDES")
                .pin("REV")
                .bel_mode(bel_ologic, "OSERDES")
                .bel_attr(bel_ologic, "REVINV", "REV_B")
                .bel_pin(bel_ologic, "REV")
                .test_bel_input_inv_special(ILOGIC::REV, specials::ILOGIC_OSR_B, val)
                .attr("REVINV", vname)
                .commit();
        }

        bctx.mode("ISERDES")
            .attr("DATA_WIDTH", "2")
            .test_bel_attr_bool_auto(ILOGIC::SERDES, "FALSE", "TRUE");
        bctx.mode("ISERDES").test_bel_attr_auto(ILOGIC::SERDES_MODE);
        bctx.mode("ISERDES").test_bel_attr_subset_auto(
            ILOGIC::INTERFACE_TYPE,
            &[
                enums::ILOGIC_INTERFACE_TYPE::MEMORY,
                enums::ILOGIC_INTERFACE_TYPE::NETWORKING,
            ],
        );
        for (q2, q1, spec) in [
            ("IFF2", "IFF1", specials::ILOGIC_Q1MUX_IFF2_IFF1),
            ("IFF2", "IFF3", specials::ILOGIC_Q1MUX_IFF2_IFF3),
            ("IFF4", "IFF1", specials::ILOGIC_Q1MUX_IFF4_IFF1),
            ("IFF4", "IFF3", specials::ILOGIC_Q1MUX_IFF4_IFF3),
        ] {
            bctx.mode("ILOGIC")
                .attr("IFF1", "#FF")
                .attr("Q2MUX", q2)
                .attr("IFFMUX", "1")
                .attr("IFFDELMUX", "1")
                .pin("D")
                .pin("Q1")
                .pin("Q2")
                .test_bel_special(spec)
                .attr("Q1MUX", q1)
                .commit();
        }
        for (q1, q2, spec) in [
            ("IFF1", "IFF2", specials::ILOGIC_Q2MUX_IFF1_IFF2),
            ("IFF3", "IFF2", specials::ILOGIC_Q2MUX_IFF3_IFF2),
            ("IFF1", "IFF4", specials::ILOGIC_Q2MUX_IFF1_IFF4),
            ("IFF3", "IFF4", specials::ILOGIC_Q2MUX_IFF3_IFF4),
        ] {
            bctx.mode("ILOGIC")
                .attr("IFF1", "#FF")
                .attr("Q1MUX", q1)
                .attr("IFFMUX", "1")
                .attr("IFFDELMUX", "1")
                .pin("D")
                .pin("Q1")
                .pin("Q2")
                .test_bel_special(spec)
                .attr("Q2MUX", q2)
                .commit();
        }
        bctx.mode("ISERDES")
            .attr("SERDES", "TRUE")
            .test_bel_attr_subset_rename(
                "DATA_WIDTH",
                ILOGIC::DATA_WIDTH,
                &[
                    enums::IO_DATA_WIDTH::_2,
                    enums::IO_DATA_WIDTH::_3,
                    enums::IO_DATA_WIDTH::_4,
                    enums::IO_DATA_WIDTH::_5,
                    enums::IO_DATA_WIDTH::_6,
                    enums::IO_DATA_WIDTH::_7,
                    enums::IO_DATA_WIDTH::_8,
                    enums::IO_DATA_WIDTH::_10,
                ],
            );
        bctx.mode("ISERDES")
            .attr("SRTYPE", "SYNC")
            .test_bel_attr_bool_special_auto(
                ILOGIC::BITSLIP_ENABLE,
                specials::ILOGIC_SYNC,
                "FALSE",
                "TRUE",
            );
        bctx.mode("ISERDES")
            .attr("SRTYPE", "ASYNC")
            .test_bel_attr_bool_special_auto(
                ILOGIC::BITSLIP_ENABLE,
                specials::ILOGIC_ASYNC,
                "FALSE",
                "TRUE",
            );
        bctx.mode("ISERDES").test_bel_attr_auto(ILOGIC::NUM_CE);
        bctx.mode("ISERDES")
            .attr("INIT_BITSLIPCNT", "1111")
            .attr("INIT_RANK1_PARTIAL", "11111")
            .attr("INIT_RANK2", "111111")
            .attr("INIT_RANK3", "111111")
            .test_bel_attr_auto(ILOGIC::DATA_RATE);
        bctx.mode("ISERDES")
            .test_bel_attr_auto(ILOGIC::DDR_CLK_EDGE);

        bctx.mode("ILOGIC")
            .test_bel_attr_bool_rename("IFF1", ILOGIC::FFI_LATCH, "#FF", "#LATCH");
        for (aname, attr) in [
            ("INIT_Q1", ILOGIC::FFI1_INIT),
            ("INIT_Q2", ILOGIC::FFI2_INIT),
            ("INIT_Q3", ILOGIC::FFI3_INIT),
            ("INIT_Q4", ILOGIC::FFI4_INIT),
            ("SRVAL_Q1", ILOGIC::FFI1_SRVAL),
            ("SRVAL_Q2", ILOGIC::FFI2_SRVAL),
            ("SRVAL_Q3", ILOGIC::FFI3_SRVAL),
            ("SRVAL_Q4", ILOGIC::FFI4_SRVAL),
        ] {
            bctx.mode("ILOGIC")
                .attr("IFF1", "#FF")
                .test_bel_attr_bool_rename(aname, attr, "0", "1");
            bctx.mode("ISERDES")
                .test_bel_attr_bool_rename(aname, attr, "0", "1");
        }
        bctx.mode("ILOGIC")
            .attr("IFF1", "#FF")
            .test_bel_attr_bool_rename("SRTYPE", ILOGIC::FFI_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode("ISERDES").test_bel_attr_bool_rename(
            "SRTYPE",
            ILOGIC::FFI_SR_SYNC,
            "ASYNC",
            "SYNC",
        );

        bctx.mode("ISERDES")
            .attr("CE1INV", "CE1")
            .attr("CE2INV", "CE2")
            .pin("CE1")
            .pin("CE2")
            .test_bel_attr_multi(ILOGIC::INIT_CE, MultiValue::Bin);
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
            .test_bel_attr_multi(ILOGIC::INIT_BITSLIPCNT, MultiValue::Bin);
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
            .test_bel_attr_multi(ILOGIC::INIT_RANK1_PARTIAL, MultiValue::Bin);
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
            .test_bel_attr_multi(ILOGIC::INIT_RANK2, MultiValue::Bin);
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
            .test_bel_attr_multi(ILOGIC::INIT_RANK3, MultiValue::Bin);

        bctx.mode("ILOGIC")
            .attr("IMUX", "0")
            .attr("IDELMUX", "1")
            .attr("IFFMUX", "#OFF")
            .pin("D")
            .pin("TFB")
            .pin("OFB")
            .pin("O")
            .test_bel_attr_rename("D2OBYP_SEL", ILOGIC::MUX_TSBYPASS);
        bctx.mode("ILOGIC")
            .attr("IFFMUX", "0")
            .attr("IFF1", "#FF")
            .attr("IFFDELMUX", "1")
            .attr("IMUX", "#OFF")
            .pin("D")
            .pin("TFB")
            .pin("OFB")
            .test_bel_attr_rename("D2OFFBYP_SEL", ILOGIC::MUX_TSBYPASS);
        bctx.mode("ILOGIC")
            .attr("IDELMUX", "1")
            .attr("IDELMUX1USED", "0")
            .pin("D")
            .pin("O")
            .pin("TFB")
            .pin("OFB")
            .test_bel_attr_bool_rename("IMUX", ILOGIC::I_TSBYPASS_ENABLE, "1", "0");
        bctx.mode("ILOGIC")
            .attr("IFFDELMUX", "1")
            .attr("IFF1", "#FF")
            .pin("D")
            .pin("TFB")
            .pin("OFB")
            .test_bel_attr_bool_rename("IFFMUX", ILOGIC::FFI_TSBYPASS_ENABLE, "1", "0");
        for (val, vname) in [
            (enums::ILOGIC_IDELAYMUX::D, "1"),
            (enums::ILOGIC_IDELAYMUX::OFB, "0"),
        ] {
            bctx.mode("ILOGIC")
                .attr("IDELMUX", "0")
                .attr("IMUX", "1")
                .attr("CLKDIVINV", "CLKDIV")
                .pin("D")
                .pin("O")
                .pin("OFB")
                .pin("CLKDIV")
                .test_bel_attr_val(ILOGIC::IDELAYMUX, val)
                .attr("IDELAYMUX", vname)
                .commit();
        }
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
            .test_bel_attr_bool_rename("IDELMUX", ILOGIC::I_DELAY_ENABLE, "1", "0");
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
            .test_bel_attr_bool_rename("IFFDELMUX", ILOGIC::FFI_DELAY_ENABLE, "1", "0");

        for (spec, val) in [
            (specials::ILOGIC_IOBDELAY_NONE, "NONE"),
            (specials::ILOGIC_IOBDELAY_IFD, "IFD"),
            (specials::ILOGIC_IOBDELAY_IBUF, "IBUF"),
            (specials::ILOGIC_IOBDELAY_BOTH, "BOTH"),
        ] {
            bctx.mode("ISERDES")
                .attr("IOBDELAY", val)
                .pin("OFB")
                .test_bel_attr_bool_special_rename(
                    "OFB_USED",
                    ILOGIC::I_TSBYPASS_ENABLE,
                    spec,
                    "FALSE",
                    "TRUE",
                );
            bctx.mode("ISERDES")
                .attr("OFB_USED", "FALSE")
                .test_bel_special(spec)
                .attr("IOBDELAY", val)
                .commit();
        }
        // this seems wrong, and also it's opposite on v5 — bug?
        for (val, vname) in [
            (enums::ILOGIC_MUX_TSBYPASS::GND, "TRUE"),
            (enums::ILOGIC_MUX_TSBYPASS::T, "FALSE"),
        ] {
            bctx.mode("ISERDES")
                .pin("TFB")
                .test_bel_attr_special_val(ILOGIC::MUX_TSBYPASS, specials::ISERDES, val)
                .attr("TFB_USED", vname)
                .commit();
        }

        bctx.mode("ILOGIC")
            .attr("IDELMUX", "0")
            .attr("IMUX", "1")
            .attr("IDELAYMUX", "1")
            .attr("CLKDIVINV", "CLKDIV")
            .attr("IFFDELMUX", "#OFF")
            .pin("CLKDIV")
            .pin("D")
            .pin("O")
            .test_bel_attr_special_auto(ILOGIC::IOBDELAY_TYPE, specials::ILOGIC_IBUF);
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
            .test_bel_attr_special_auto(ILOGIC::IOBDELAY_TYPE, specials::ILOGIC_IFD);
        bctx.mode("ISERDES")
            .attr("IOBDELAY", "IBUF")
            .test_bel_attr_special_auto(ILOGIC::IOBDELAY_TYPE, specials::ISERDES_IBUF);
        bctx.mode("ISERDES")
            .attr("IOBDELAY", "IFD")
            .test_bel_attr_special_auto(ILOGIC::IOBDELAY_TYPE, specials::ISERDES_IFD);

        bctx.mode("ILOGIC")
            .test_bel_attr_bits(ILOGIC::IOBDELAY_VALUE_INIT)
            .multi_attr("IOBDELAY_VALUE", MultiValue::Dec(0), 6);
        bctx.mode("ISERDES")
            .test_bel_attr_bits(ILOGIC::IOBDELAY_VALUE_INIT)
            .multi_attr("IOBDELAY_VALUE", MultiValue::Dec(0), 6);
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::OLOGIC[i]);
        let bel_ilogic = bslots::ILOGIC[i];
        bctx.build()
            .bel_unused(bel_ilogic)
            .test_bel_special(specials::OLOGIC)
            .mode("OLOGIC")
            .commit();
        bctx.build()
            .bel_unused(bel_ilogic)
            .test_bel_special(specials::OSERDES)
            .mode("OSERDES")
            .commit();
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("OMUX", "OFFDDRA")
            .pin("CLK")
            .pin("OQ")
            .test_bel_attr_bool_rename("CLK1INV", bcls::OLOGIC::CLK1_INV, "C", "C_B");
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("OMUX", "OFFDDRA")
            .pin("CLK")
            .pin("OQ")
            .test_bel_attr_bool_rename("CLK2INV", bcls::OLOGIC::CLK2_INV, "CLK", "CLK_B");
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("OSRUSED", "0")
            .attr("OMUX", "OFFDDRA")
            .pin("OQ")
            .bel_unused(bel_ilogic)
            .test_bel_input_inv_special_auto(bcls::OLOGIC::SR, specials::OLOGIC);
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("OREVUSED", "0")
            .attr("OMUX", "OFFDDRA")
            .pin("OQ")
            .bel_unused(bel_ilogic)
            .test_bel_input_inv_special_auto(bcls::OLOGIC::REV, specials::OLOGIC);
        for pin in [bcls::OLOGIC::D1, bcls::OLOGIC::D2, bcls::OLOGIC::OCE] {
            bctx.mode("OLOGIC")
                .attr("OFF1", "#FF")
                .attr("OMUX", "OFFDDRA")
                .pin("OQ")
                .test_bel_input_inv_special_auto(pin, specials::OLOGIC);
        }
        for pin in [bcls::OLOGIC::T2, bcls::OLOGIC::TCE] {
            bctx.mode("OLOGIC")
                .attr("TFF1", "#FF")
                .attr("TMUX", "TFFDDRA")
                .pin("TQ")
                .test_bel_input_inv_special_auto(pin, specials::OLOGIC);
        }
        bctx.mode("OLOGIC")
            .attr("TMUX", "T1")
            .attr("T1USED", "0")
            .pin("TQ")
            .test_bel_input_inv_special_auto(bcls::OLOGIC::T1, specials::OLOGIC);

        for pin in [
            bcls::OLOGIC::CLKDIV,
            bcls::OLOGIC::SR,
            bcls::OLOGIC::REV,
            bcls::OLOGIC::D1,
            bcls::OLOGIC::D2,
            bcls::OLOGIC::D3,
            bcls::OLOGIC::D4,
            bcls::OLOGIC::D5,
            bcls::OLOGIC::D6,
            bcls::OLOGIC::T2,
            bcls::OLOGIC::T3,
            bcls::OLOGIC::T4,
        ] {
            bctx.mode("OSERDES")
                .bel_unused(bel_ilogic)
                .test_bel_input_inv_special_auto(pin, specials::OSERDES);
        }
        bctx.mode("OSERDES")
            .attr("DATA_RATE_TQ", "BUF")
            .test_bel_input_inv_special_auto(bcls::OLOGIC::T1, specials::OSERDES);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_TQ", "DDR")
            .test_bel_input_inv_special_auto(bcls::OLOGIC::TCE, specials::OSERDES);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("CLKINV", "CLK")
            .attr("DDR_CLK_EDGE", "SAME_EDGE")
            .pin("CLK")
            .test_bel_input_inv_special_auto(bcls::OLOGIC::OCE, specials::OSERDES);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("OCEINV", "OCE")
            .attr("DDR_CLK_EDGE", "SAME_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_bel_input_inv_special_auto(bcls::OLOGIC::CLK, specials::OSERDES_SAME_EDGE);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("OCEINV", "OCE")
            .attr("DDR_CLK_EDGE", "OPPOSITE_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_bel_input_inv_special_auto(bcls::OLOGIC::CLK, specials::OSERDES_OPPOSITE_EDGE);

        bctx.mode("OLOGIC")
            .attr("OCEINV", "OCE_B")
            .pin("OCE")
            .test_bel_attr_bool_rename("OFF1", bcls::OLOGIC::FFO_LATCH, "#FF", "#LATCH");
        bctx.mode("OLOGIC")
            .attr("TCEINV", "TCE_B")
            .pin("TCE")
            .test_bel_attr_bool_rename("TFF1", bcls::OLOGIC::FFT_LATCH, "#FF", "#LATCH");
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .test_bel_attr_bool_rename("SRTYPE_OQ", bcls::OLOGIC::FFO_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode("OLOGIC")
            .attr("TFF1", "#FF")
            .test_bel_attr_bool_rename("SRTYPE_TQ", bcls::OLOGIC::FFT_SR_SYNC, "ASYNC", "SYNC");
        for (attr, aname, oaname) in [
            (bcls::OLOGIC::FFO_SR_ENABLE, "OSRUSED", "TSRUSED"),
            (bcls::OLOGIC::FFT_SR_ENABLE, "TSRUSED", "OSRUSED"),
            (bcls::OLOGIC::FFO_REV_ENABLE, "OREVUSED", "TREVUSED"),
            (bcls::OLOGIC::FFT_REV_ENABLE, "TREVUSED", "OREVUSED"),
        ] {
            bctx.mode("OLOGIC")
                .attr("OFF1", "#FF")
                .attr("TFF1", "#FF")
                .attr("REVINV", "REV")
                .attr("SRINV", "SR")
                .attr(oaname, "0")
                .pin("REV")
                .pin("SR")
                .test_bel_attr_bits(attr)
                .attr(aname, "0")
                .commit();
        }

        bctx.mode("OLOGIC").test_bel_attr_bool_special_rename(
            "INIT_OQ",
            bcls::OLOGIC::FFO_INIT,
            specials::OLOGIC,
            "0",
            "1",
        );
        bctx.mode("OLOGIC")
            .test_bel_attr_bool_rename("INIT_TQ", bcls::OLOGIC::FFT_INIT, "0", "1");
        bctx.mode("OSERDES").test_bel_attr_bool_special_rename(
            "INIT_OQ",
            bcls::OLOGIC::FFO_INIT,
            specials::OSERDES,
            "0",
            "1",
        );
        bctx.mode("OSERDES")
            .test_bel_attr_bool_rename("INIT_TQ", bcls::OLOGIC::FFT_INIT, "0", "1");

        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("D2INV", "#OFF")
            .attr("OMUX", "OFF1")
            .pin("OQ")
            .test_bel_attr_bool_rename("SRVAL_OQ", bcls::OLOGIC::FFO_SRVAL, "0", "1");
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("D2INV", "D2")
            .attr("OMUX", "OFFDDRA")
            .pin("D2")
            .pin("OQ")
            .test_bel_attr_bool_rename("SRVAL_OQ", bcls::OLOGIC::FFO_SRVAL, "0", "1");
        bctx.mode("OLOGIC")
            .attr("OFF1", "#FF")
            .attr("D2INV", "D2")
            .attr("OMUX", "OFFDDRB")
            .pin("D2")
            .pin("OQ")
            .test_bel_attr_bool_rename("SRVAL_OQ", bcls::OLOGIC::FFO_SRVAL, "0", "1");
        bctx.mode("OLOGIC")
            .attr("TFF1", "#FF")
            .attr("T2INV", "#OFF")
            .attr("TMUX", "TFF1")
            .pin("TQ")
            .test_bel_attr_bool_special_rename(
                "SRVAL_TQ",
                bcls::OLOGIC::FFT1_SRVAL,
                specials::OLOGIC_TFF1,
                "0",
                "1",
            );
        bctx.mode("OLOGIC")
            .attr("TFF1", "#FF")
            .attr("T2INV", "T2")
            .attr("TMUX", "TFFDDRA")
            .pin("T2")
            .pin("TQ")
            .test_bel_attr_bool_special_rename(
                "SRVAL_TQ",
                bcls::OLOGIC::FFT1_SRVAL,
                specials::OLOGIC_TFFDDRA,
                "0",
                "1",
            );
        bctx.mode("OLOGIC")
            .attr("TFF1", "#FF")
            .attr("T2INV", "T2")
            .attr("TMUX", "TFFDDRB")
            .pin("T2")
            .pin("TQ")
            .test_bel_attr_bool_special_rename(
                "SRVAL_TQ",
                bcls::OLOGIC::FFT1_SRVAL,
                specials::OLOGIC_TFFDDRB,
                "0",
                "1",
            );
        bctx.mode("OSERDES").test_bel_attr_bool_rename(
            "SRVAL_OQ",
            bcls::OLOGIC::FFO_SRVAL,
            "0",
            "1",
        );
        bctx.mode("OSERDES").test_bel_attr_bool_special_rename(
            "SRVAL_TQ",
            bcls::OLOGIC::FFT1_SRVAL,
            specials::OSERDES,
            "0",
            "1",
        );

        for (val, vname) in [
            (enums::OLOGIC_V4_MUX_O::D1, "D1"),
            (enums::OLOGIC_V4_MUX_O::FFO1, "OFF1"),
            (enums::OLOGIC_V4_MUX_O::FFODDR, "OFFDDRA"),
            (enums::OLOGIC_V4_MUX_O::FFODDR, "OFFDDRB"),
        ] {
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
                .test_bel_attr_val(bcls::OLOGIC::V4_MUX_O, val)
                .attr("OMUX", vname)
                .commit();
        }
        for (val, vname) in [
            (enums::OLOGIC_V4_MUX_T::T1, "T1"),
            (enums::OLOGIC_V4_MUX_T::FFT1, "TFF1"),
            (enums::OLOGIC_V4_MUX_T::FFTDDR, "TFFDDRA"),
            (enums::OLOGIC_V4_MUX_T::FFTDDR, "TFFDDRB"),
        ] {
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
                .test_bel_attr_val(bcls::OLOGIC::V4_MUX_T, val)
                .attr("TMUX", vname)
                .commit();
        }

        bctx.mode("OSERDES")
            .attr("DATA_WIDTH", "2")
            .test_bel_attr_bool_auto(bcls::OLOGIC::SERDES, "FALSE", "TRUE");
        bctx.mode("OSERDES")
            .test_bel_attr_auto(bcls::OLOGIC::SERDES_MODE);
        for (val, spec) in [
            ("SAME_EDGE", specials::OSERDES_SAME_EDGE),
            ("OPPOSITE_EDGE", specials::OSERDES_OPPOSITE_EDGE),
        ] {
            bctx.mode("OSERDES")
                .null_bits()
                .test_bel_special(spec)
                .attr("DDR_CLK_EDGE", val)
                .commit();
        }
        for (val, spec) in [
            ("ASYNC", specials::OSERDES_SRTYPE_ASYNC),
            ("SYNC", specials::OSERDES_SRTYPE_SYNC),
        ] {
            bctx.mode("OSERDES")
                .test_bel_special(spec)
                .attr("SRTYPE", val)
                .commit();
        }
        for (val, spec) in [
            ("SDR", specials::OSERDES_SDR),
            ("DDR", specials::OSERDES_DDR),
        ] {
            bctx.mode("OSERDES")
                .test_bel_special(spec)
                .attr("DATA_RATE_OQ", val)
                .commit();
        }
        for (val, vname) in [
            (enums::OLOGIC_V4_MUX_T::T1, "BUF"),
            (enums::OLOGIC_V4_MUX_T::FFT1, "SDR"),
            (enums::OLOGIC_V4_MUX_T::FFTDDR, "DDR"),
        ] {
            bctx.mode("OSERDES")
                .attr("TCEINV", "TCE_B")
                .attr("T1INV", "T1")
                .pin("TCE")
                .pin("T1")
                .test_bel_attr_val(bcls::OLOGIC::V4_MUX_T, val)
                .attr("DATA_RATE_TQ", vname)
                .commit();
        }
        bctx.mode("OSERDES")
            .test_bel_attr_auto(bcls::OLOGIC::TRISTATE_WIDTH);
        bctx.mode("OSERDES")
            .attr("SERDES", "TRUE")
            .test_bel_attr_subset_rename(
                "DATA_WIDTH",
                bcls::OLOGIC::DATA_WIDTH,
                &[
                    enums::IO_DATA_WIDTH::_2,
                    enums::IO_DATA_WIDTH::_3,
                    enums::IO_DATA_WIDTH::_4,
                    enums::IO_DATA_WIDTH::_5,
                    enums::IO_DATA_WIDTH::_6,
                    enums::IO_DATA_WIDTH::_7,
                    enums::IO_DATA_WIDTH::_8,
                    enums::IO_DATA_WIDTH::_10,
                ],
            );
        bctx.mode("OSERDES")
            .test_bel_attr_multi(bcls::OLOGIC::INIT_LOADCNT, MultiValue::Bin);
    }
    for i in 0..2 {
        let bel = bslots::IOB[i];
        let mut bctx = ctx.bel(bel);
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_bel_special(specials::PRESENT)
            .mode("IOB")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "CONTINUOUS")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_bel_special(specials::IOB_CONTINUOUS)
            .mode("IOB")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_bel_special(specials::IOB_IPAD)
            .mode("IPAD")
            .commit();
        bctx.mode("IOB")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .test_bel_attr_auto_default(IOB::PULL, enums::IOB_PULL::NONE);
        bctx.mode("IOB")
            .null_bits()
            .test_bel_special(specials::IOB_DISABLE_GTS)
            .attr("GTSATTRBOX", "DISABLE_GTS")
            .commit();
        bctx.build()
            .mode("IOB")
            .pin("O")
            .attr("IOATTRBOX", "")
            .test_bel_attr_bits(IOB::OUTPUT_ENABLE)
            .attr("DRIVE_0MA", "DRIVE_0MA")
            .attr("OUSED", "0")
            .commit();
        bctx.mode("IOB")
            .attr("OUSED", "0")
            .pin("O")
            .test_bel_special_bits(specials::IOB_OPROGRAMMING)
            .multi_attr("OPROGRAMMING", MultiValue::Bin, 22);
        for &std in IOSTDS {
            let (spec, row) = get_istd_row(edev, &std);
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
                dci_special = Some(Dci(spec, row));
            }
            if std.diff != DiffKind::None {
                bctx.mode("IOB")
                    .attr("OUSED", "")
                    .pin("I")
                    .pin("DIFFI_IN")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .maybe_prop(dci_special)
                    .test_bel_special_row(spec, row)
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
                        .test_bel_special_row(specials::IOB_ISTD_LVDS_TERM, row)
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
                    .test_bel_special_row(spec, row)
                    .attr("INBUFUSED", "0")
                    .attr("IOATTRBOX", std.name)
                    .commit();
            }
        }
        for &std in IOSTDS {
            if std.diff == DiffKind::True {
                let row = get_lvds_row(edev, &std);
                if i == 0 {
                    bctx.build()
                        .attr("INBUFUSED", "")
                        .attr("OPROGRAMMING", "")
                        .attr("OUSED", "")
                        .pin("DIFFO_IN")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .prop(DiffOut(None))
                        .test_bel_special_row(specials::IOB_OSTD_LVDS, row)
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
                        .prop(DiffOut(Some(row)))
                        .test_bel_special_row(specials::IOB_OSTD_LVDS, row)
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
                let (spec, row) = get_ostd_row(edev, &std, 0, "");
                let (dspec, drow) = get_istd_row(edev, &std);
                bctx.mode("IOB")
                    .pin("O")
                    .attr("INBUFUSED", "")
                    .attr("OPROGRAMMING", "")
                    .attr("OUSED", "0")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .prop(Dci(dspec, drow))
                    .test_bel_special_row(spec, row)
                    .attr("IOATTRBOX", std.name)
                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                    .commit();
            } else if !std.drive.is_empty() {
                for &drive in std.drive {
                    for slew in ["SLOW", "FAST"] {
                        let (spec, row) = get_ostd_row(edev, &std, drive, slew);
                        bctx.mode("IOB")
                            .pin("O")
                            .attr("INBUFUSED", "")
                            .attr("OPROGRAMMING", "")
                            .attr("OUSED", "0")
                            .test_bel_special_row(spec, row)
                            .attr("IOATTRBOX", std.name)
                            .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                            .attr(
                                "DRIVEATTRBOX",
                                if drive == 0 {
                                    "".to_string()
                                } else {
                                    drive.to_string()
                                },
                            )
                            .attr("SLEW", slew)
                            .commit();
                    }
                }
            } else {
                let (spec, row) = get_ostd_row(edev, &std, 0, "");
                bctx.mode("IOB")
                    .pin("O")
                    .attr("INBUFUSED", "")
                    .attr("OPROGRAMMING", "")
                    .attr("OUSED", "0")
                    .test_bel_special_row(spec, row)
                    .attr("IOATTRBOX", std.name)
                    .attr_diff("DRIVE_0MA", "DRIVE_0MA", "")
                    .commit();
            }
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .extra_tiles_by_bel_attr_bits(bslots::DCI, bcls::DCI::QUIET)
        .test_global_special(specials::DCI_QUIET)
        .global_diff("DCIUPDATEMODE", "CONTINUOUS", "QUIET")
        .commit();
    for (spec, bank) in [
        (specials::CENTER_DCI_BANK1, 1),
        (specials::CENTER_DCI_BANK2, 2),
        (specials::CENTER_DCI_BANK3, 3),
        (specials::CENTER_DCI_BANK4, 4),
    ] {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        let cfg = edev.tile_cfg(die);
        let mut builder = ctx
            .build()
            .raw(Key::Package, &package.name)
            .extra_fixed_bel_attr_bits(cfg, bslots::MISC_CFG, bcls::MISC_CFG::DCI_CLK_ENABLE);

        // Find VR and IO rows, set up HCLKs.
        let (vr_row, io_row) = match bank {
            1 => {
                let mut row = chip.row_bufg() + 8;
                while row != edev.row_iobdcm.unwrap() - 16 {
                    let hclk_center = die.cell(edev.col_cfg, row).tile(tslots::HCLK_BEL);
                    builder = builder.extra_fixed_bel_attr_bits(
                        hclk_center,
                        bslots::DCI,
                        bcls::DCI::CASCADE_FROM_ABOVE,
                    );
                    row += 16;
                }
                let hclk_center = die.cell(edev.col_cfg, row).tile(tslots::HCLK_BEL);
                builder =
                    builder.extra_fixed_bel_attr_bits(hclk_center, bslots::DCI, bcls::DCI::ENABLE);
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
                    let hclk_center = die.cell(edev.col_cfg, row).tile(tslots::HCLK_BEL);
                    builder = builder.extra_fixed_bel_attr_bits(
                        hclk_center,
                        bslots::DCI,
                        bcls::DCI::CASCADE_FROM_BELOW,
                    );
                    row -= 16;
                }
                let hclk_center = die.cell(edev.col_cfg, row).tile(tslots::HCLK_BEL);
                builder =
                    builder.extra_fixed_bel_attr_bits(hclk_center, bslots::DCI, bcls::DCI::ENABLE);
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
                let hclk_iobdcm = die
                    .cell(edev.col_cfg, edev.row_iobdcm.unwrap())
                    .tile(tslots::HCLK_BEL);
                builder =
                    builder.extra_fixed_bel_attr_bits(hclk_iobdcm, bslots::DCI, bcls::DCI::ENABLE);
                (
                    Some(edev.row_iobdcm.unwrap() - 2),
                    edev.row_iobdcm.unwrap() - 1,
                )
            }
            4 => {
                let hclk_dcmiob = die
                    .cell(edev.col_cfg, edev.row_dcmiob.unwrap())
                    .tile(tslots::HCLK_BEL);
                builder =
                    builder.extra_fixed_bel_attr_bits(hclk_dcmiob, bslots::DCI, bcls::DCI::ENABLE);
                (Some(edev.row_dcmiob.unwrap() + 1), edev.row_dcmiob.unwrap())
            }
            _ => unreachable!(),
        };
        let vr_tile = vr_row.map(|row| die.cell(edev.col_cfg, row).tile(tslots::BEL));
        let io_tile = die.cell(edev.col_cfg, io_row).tile(tslots::BEL);

        // Ensure nothing is placed in VR.  Set up VR diff.
        if let Some(vr_tile) = vr_tile {
            for bel in [bslots::IOB[0], bslots::IOB[1]] {
                let site = backend.ngrid.get_bel_name(vr_tile.cell.bel(bel)).unwrap();
                builder = builder.raw(Key::SiteMode(site), None);
            }
            builder =
                builder.extra_fixed_bel_special(vr_tile, bslots::IOB[0], specials::IOB_VR_CENTER);
        }

        // Set up the IO and fire.
        let site = backend
            .ngrid
            .get_bel_name(io_tile.cell.bel(bslots::IOB[0]))
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
            .extra_fixed_bel_special_row(
                io_tile,
                bslots::IOB[0],
                specials::IOB_OSTD,
                IOB_DATA::LVDCI_33,
            )
            .test_global_special(spec)
            .commit();
    }

    for tcid in [
        tcls::HCLK_IO_DCI,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_N,
        tcls::HCLK_IO_DCM_S,
        tcls::HCLK_IO_DCM_N,
    ] {
        let mut ctx = FuzzCtx::new(session, backend, tcid);
        let mut bctx = ctx.bel(bslots::DCI);
        bctx.build()
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_bel_attr_bits(bcls::DCI::TEST_ENABLE)
            .mode("DCI")
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    let tcid = tcls::IO;
    for i in 0..2 {
        let bslot = bslots::ILOGIC[i];

        let mut present_ilogic = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC);
        let mut present_iserdes = ctx.get_diff_bel_special(tcid, bslot, specials::ISERDES);

        ctx.collect_bel_input_inv_int_bi(&[tcls::INT], tcid, bslot, ILOGIC::CLKDIV);
        ctx.collect_bel_input_inv_bi(tcid, bslot, ILOGIC::CE1);
        ctx.collect_bel_input_inv_bi(tcid, bslot, ILOGIC::CE2);
        for pin in [ILOGIC::SR, ILOGIC::REV] {
            let diff0 =
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::ILOGIC_OSR, false);
            let diff1 =
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::ILOGIC_OSR, true);
            let item = xlat_bit_bi(diff0, diff1);
            ctx.insert_bel_input_inv(tcid, bslot, pin, item);
            let diff0 =
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::ILOGIC_OSR_B, true);
            let diff1 =
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::ILOGIC_OSR_B, false);
            let item = xlat_bit_bi(diff0, diff1);
            ctx.insert_bel_input_inv(tcid, bslot, pin, item);
        }

        let diff1 = ctx.get_diff_bel_special(tcid, bslot, specials::ISERDES_OCLK_B_DDR);
        let diff2 = ctx.get_diff_bel_special(tcid, bslot, specials::ISERDES_OCLK_DDR);
        ctx.get_diff_bel_special(tcid, bslot, specials::ISERDES_OCLK_SDR)
            .assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ISERDES_OCLK_B_SDR);
        diff = diff.combine(&!&diff1);
        diff = diff.combine(&!&diff2);
        diff.assert_empty();
        ctx.insert_bel_attr_bool(tcid, bslot, ILOGIC::OCLK1_INV, xlat_bit(diff1));
        ctx.insert_bel_attr_bool(tcid, bslot, ILOGIC::OCLK2_INV, xlat_bit(diff2));
        let bits = xlat_bit_wide_bi(
            ctx.get_diff_bel_input_inv(tcid, bslot, ILOGIC::CLK, false),
            ctx.get_diff_bel_input_inv(tcid, bslot, ILOGIC::CLK, true),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, ILOGIC::CLK_INV, bits);

        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::SERDES);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI_LATCH);
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::SERDES_MODE);
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            ILOGIC::INTERFACE_TYPE,
            &[
                enums::ILOGIC_INTERFACE_TYPE::MEMORY,
                enums::ILOGIC_INTERFACE_TYPE::NETWORKING,
            ],
        );
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::NUM_CE);
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::INIT_BITSLIPCNT);
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::INIT_CE);
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::INIT_RANK1_PARTIAL);
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::INIT_RANK2);
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::INIT_RANK3);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI_SR_SYNC);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI1_INIT);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI2_INIT);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI3_INIT);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI4_INIT);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI1_SRVAL);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI2_SRVAL);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI3_SRVAL);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI4_SRVAL);

        let mut diffs = vec![(enums::IO_DATA_WIDTH::NONE, Diff::default())];
        for val in [
            enums::IO_DATA_WIDTH::_2,
            enums::IO_DATA_WIDTH::_3,
            enums::IO_DATA_WIDTH::_4,
            enums::IO_DATA_WIDTH::_5,
            enums::IO_DATA_WIDTH::_6,
            enums::IO_DATA_WIDTH::_7,
            enums::IO_DATA_WIDTH::_8,
            enums::IO_DATA_WIDTH::_10,
        ] {
            let mut diff = ctx.get_diff_attr_val(tcid, bslot, ILOGIC::DATA_WIDTH, val);
            diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, ILOGIC::SERDES), true, false);
            diffs.push((val, diff));
        }
        let mut bits = xlat_enum_attr(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            ILOGIC::DATA_WIDTH,
            xlat_enum_attr_ocd(diffs, OcdMode::FixedOrder(&bits)),
        );

        let mut diffs = vec![];
        for val in [enums::IO_DATA_RATE::SDR, enums::IO_DATA_RATE::DDR] {
            let mut diff = ctx.get_diff_attr_val(tcid, bslot, ILOGIC::DATA_RATE, val);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, ILOGIC::FFI_LATCH),
                false,
                true,
            );
            diffs.push((val, diff));
        }
        ctx.insert_bel_attr_enum(tcid, bslot, ILOGIC::DATA_RATE, xlat_enum_attr(diffs));

        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::BITSLIP_ENABLE,
            specials::ILOGIC_ASYNC,
            0,
            false,
        )
        .assert_empty();
        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::BITSLIP_ENABLE,
            specials::ILOGIC_SYNC,
            0,
            false,
        )
        .assert_empty();
        let diff_async = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::BITSLIP_ENABLE,
            specials::ILOGIC_ASYNC,
            0,
            true,
        );
        let diff_sync = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::BITSLIP_ENABLE,
            specials::ILOGIC_SYNC,
            0,
            true,
        );
        let diff_sync = diff_sync.combine(&!&diff_async);
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            ILOGIC::BITSLIP_ENABLE,
            xlat_bit_wide(diff_async),
        );
        ctx.insert_bel_attr_bool(tcid, bslot, ILOGIC::BITSLIP_SYNC, xlat_bit(diff_sync));

        let mut diffs_a = vec![];
        let mut diffs_b = vec![];
        for diff in ctx.get_diffs_attr_bits(tcid, bslot, ILOGIC::IOBDELAY_VALUE_INIT, 6) {
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
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            ILOGIC::IOBDELAY_VALUE_INIT,
            xlat_bitvec(diffs_a),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            ILOGIC::IOBDELAY_VALUE_CUR,
            xlat_bitvec(diffs_b),
        );

        let item = xlat_enum_attr(vec![
            (
                enums::ILOGIC_DDR_CLK_EDGE::OPPOSITE_EDGE,
                ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_Q2MUX_IFF3_IFF2),
            ),
            (
                enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE,
                ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_Q1MUX_IFF4_IFF1),
            ),
            (
                enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE_PIPELINED,
                Diff::default(),
            ),
        ]);
        // wtf is even going on
        present_iserdes.apply_enum_diff(
            &item,
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE,
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE_PIPELINED,
        );
        ctx.get_diff_attr_val(
            tcid,
            bslot,
            ILOGIC::DDR_CLK_EDGE,
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE,
        )
        .assert_empty();
        ctx.get_diff_attr_val(
            tcid,
            bslot,
            ILOGIC::DDR_CLK_EDGE,
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE_PIPELINED,
        )
        .assert_empty();
        let mut diff = ctx.get_diff_attr_val(
            tcid,
            bslot,
            ILOGIC::DDR_CLK_EDGE,
            enums::ILOGIC_DDR_CLK_EDGE::OPPOSITE_EDGE,
        );
        diff.apply_enum_diff(
            &item,
            enums::ILOGIC_DDR_CLK_EDGE::OPPOSITE_EDGE,
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE,
        );
        diff.assert_empty();
        ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_Q1MUX_IFF2_IFF1)
            .assert_empty();
        ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_Q1MUX_IFF4_IFF3)
            .assert_empty();
        ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_Q2MUX_IFF3_IFF4)
            .assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_Q1MUX_IFF2_IFF3);
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::INTERFACE_TYPE),
            enums::ILOGIC_INTERFACE_TYPE::NETWORKING,
            enums::ILOGIC_INTERFACE_TYPE::MEMORY,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_Q2MUX_IFF1_IFF4);
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::INTERFACE_TYPE),
            enums::ILOGIC_INTERFACE_TYPE::NETWORKING,
            enums::ILOGIC_INTERFACE_TYPE::MEMORY,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_Q2MUX_IFF1_IFF2);
        diff.apply_enum_diff(
            &item,
            enums::ILOGIC_DDR_CLK_EDGE::OPPOSITE_EDGE,
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE,
        );
        diff.assert_empty();
        ctx.insert_bel_attr_enum(tcid, bslot, ILOGIC::DDR_CLK_EDGE, item);

        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            ILOGIC::IDELAYMUX,
            enums::ILOGIC_IDELAYMUX::NONE,
        );
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::MUX_TSBYPASS);
        let item = xlat_enum_attr(vec![
            (
                enums::ILOGIC_MUX_TSBYPASS::GND,
                ctx.get_diff_attr_special_val(
                    tcid,
                    bslot,
                    bcls::ILOGIC::MUX_TSBYPASS,
                    specials::ISERDES,
                    enums::ILOGIC_MUX_TSBYPASS::GND,
                ),
            ),
            (
                enums::ILOGIC_MUX_TSBYPASS::T,
                ctx.get_diff_attr_special_val(
                    tcid,
                    bslot,
                    bcls::ILOGIC::MUX_TSBYPASS,
                    specials::ISERDES,
                    enums::ILOGIC_MUX_TSBYPASS::T,
                ),
            ),
        ]);
        ctx.insert_bel_attr_enum(tcid, bslot, ILOGIC::MUX_TSBYPASS, item);

        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::I_DELAY_ENABLE);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI_DELAY_ENABLE);

        ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_IOBDELAY_NONE)
            .assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_IOBDELAY_IBUF);
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::I_DELAY_ENABLE),
            true,
            false,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::IDELAYMUX),
            enums::ILOGIC_IDELAYMUX::D,
            enums::ILOGIC_IDELAYMUX::NONE,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_IOBDELAY_IFD);
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::FFI_DELAY_ENABLE),
            true,
            false,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::IDELAYMUX),
            enums::ILOGIC_IDELAYMUX::D,
            enums::ILOGIC_IDELAYMUX::NONE,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_IOBDELAY_BOTH);
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::I_DELAY_ENABLE),
            true,
            false,
        );
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::FFI_DELAY_ENABLE),
            true,
            false,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::IDELAYMUX),
            enums::ILOGIC_IDELAYMUX::D,
            enums::ILOGIC_IDELAYMUX::NONE,
        );
        diff.assert_empty();

        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::I_TSBYPASS_ENABLE);
        let diff0 = ctx.get_diff_attr_bool_bi(tcid, bslot, ILOGIC::FFI_TSBYPASS_ENABLE, false);
        let diff1 = ctx.get_diff_attr_bool_bi(tcid, bslot, ILOGIC::FFI_TSBYPASS_ENABLE, true);
        let (diff0, diff1, diff_common) = Diff::split(diff0, diff1);
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            ILOGIC::FFI_TSBYPASS_ENABLE,
            xlat_bit_bi(diff0, diff1),
        );
        present_iserdes = present_iserdes.combine(&!&diff_common);
        ctx.insert_bel_attr_bool(tcid, bslot, ILOGIC::FFI_ENABLE, xlat_bit(diff_common));

        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::I_TSBYPASS_ENABLE,
            specials::ILOGIC_IOBDELAY_NONE,
            0,
            false,
        )
        .assert_empty();
        for spec in [
            specials::ILOGIC_IOBDELAY_IBUF,
            specials::ILOGIC_IOBDELAY_IFD,
            specials::ILOGIC_IOBDELAY_BOTH,
        ] {
            let mut diff = ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                ILOGIC::I_TSBYPASS_ENABLE,
                spec,
                0,
                false,
            );
            diff.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, ILOGIC::IDELAYMUX),
                enums::ILOGIC_IDELAYMUX::D,
                enums::ILOGIC_IDELAYMUX::NONE,
            );
            diff.assert_empty();
        }
        let mut diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::I_TSBYPASS_ENABLE,
            specials::ILOGIC_IOBDELAY_NONE,
            0,
            true,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::IDELAYMUX),
            enums::ILOGIC_IDELAYMUX::OFB,
            enums::ILOGIC_IDELAYMUX::NONE,
        );
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::I_TSBYPASS_ENABLE),
            true,
            false,
        );
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::FFI_TSBYPASS_ENABLE),
            true,
            false,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::I_TSBYPASS_ENABLE,
            specials::ILOGIC_IOBDELAY_IBUF,
            0,
            true,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::IDELAYMUX),
            enums::ILOGIC_IDELAYMUX::OFB,
            enums::ILOGIC_IDELAYMUX::NONE,
        );
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::FFI_TSBYPASS_ENABLE),
            true,
            false,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::I_TSBYPASS_ENABLE,
            specials::ILOGIC_IOBDELAY_IFD,
            0,
            true,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::IDELAYMUX),
            enums::ILOGIC_IDELAYMUX::OFB,
            enums::ILOGIC_IDELAYMUX::NONE,
        );
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::I_TSBYPASS_ENABLE),
            true,
            false,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::I_TSBYPASS_ENABLE,
            specials::ILOGIC_IOBDELAY_BOTH,
            0,
            true,
        );
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::IDELAYMUX),
            enums::ILOGIC_IDELAYMUX::OFB,
            enums::ILOGIC_IDELAYMUX::NONE,
        );
        diff.assert_empty();

        for spec in [
            specials::ILOGIC_IFD,
            specials::ILOGIC_IBUF,
            specials::ISERDES_IFD,
            specials::ISERDES_IBUF,
        ] {
            let mut diffs = vec![];
            for val in [
                enums::ILOGIC_IOBDELAY_TYPE::DEFAULT,
                enums::ILOGIC_IOBDELAY_TYPE::FIXED,
                enums::ILOGIC_IOBDELAY_TYPE::VARIABLE,
            ] {
                if val == enums::ILOGIC_IOBDELAY_TYPE::DEFAULT
                    && matches!(spec, specials::ILOGIC_IBUF | specials::ISERDES_IBUF)
                {
                    diffs.push((val, Diff::default()));
                } else {
                    diffs.push((
                        val,
                        ctx.get_diff_attr_special_val(
                            tcid,
                            bslot,
                            ILOGIC::IOBDELAY_TYPE,
                            spec,
                            val,
                        ),
                    ));
                }
            }
            ctx.insert_bel_attr_enum(tcid, bslot, ILOGIC::IOBDELAY_TYPE, xlat_enum_attr(diffs));
        }

        // hm. not clear what's going on.
        let bit = xlat_bit(ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            ILOGIC::IOBDELAY_TYPE,
            specials::ILOGIC_IBUF,
            enums::ILOGIC_IOBDELAY_TYPE::DEFAULT,
        ));
        let mut diff = ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            ILOGIC::IOBDELAY_TYPE,
            specials::ISERDES_IBUF,
            enums::ILOGIC_IOBDELAY_TYPE::DEFAULT,
        );
        diff.apply_bit_diff(bit, true, false);
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::I_DELAY_ENABLE),
            false,
            true,
        );
        diff.assert_empty();
        ctx.insert_bel_attr_bool(tcid, bslot, ILOGIC::I_DELAY_DEFAULT, bit);

        present_ilogic.apply_bit_diff(ctx.bel_input_inv(tcid, bslot, ILOGIC::CE1), false, true);
        present_iserdes.apply_bit_diff(ctx.bel_input_inv(tcid, bslot, ILOGIC::CE1), false, true);
        present_ilogic.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, ILOGIC::IOBDELAY_VALUE_CUR),
            0,
            0x3f,
        );
        present_iserdes.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, ILOGIC::IOBDELAY_VALUE_CUR),
            0,
            0x3f,
        );

        present_ilogic.assert_empty();
        present_iserdes.assert_empty();

        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            ILOGIC::READBACK_I,
            TileBit::new(0, 21, [47, 32][i]).pos(),
        );
    }
    for i in 0..2 {
        let bslot = bslots::OLOGIC[i];
        ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::FFO_SR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::FFT_SR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::FFO_REV_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::FFT_REV_ENABLE);
        for pin in [
            bcls::OLOGIC::D1,
            bcls::OLOGIC::D2,
            bcls::OLOGIC::D3,
            bcls::OLOGIC::D4,
            bcls::OLOGIC::D5,
            bcls::OLOGIC::D6,
            bcls::OLOGIC::T1,
            bcls::OLOGIC::T2,
            bcls::OLOGIC::T3,
            bcls::OLOGIC::T4,
        ] {
            let bit = xlat_bit_bi(
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OSERDES, false),
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OSERDES, true),
            );
            ctx.insert_bel_input_inv(tcid, bslot, pin, bit);
        }
        for pin in [
            bcls::OLOGIC::D1,
            bcls::OLOGIC::D2,
            bcls::OLOGIC::T1,
            bcls::OLOGIC::T2,
        ] {
            let bit = xlat_bit_bi(
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OLOGIC, false),
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OLOGIC, true),
            );
            ctx.insert_bel_input_inv(tcid, bslot, pin, bit);
        }
        for pin in [bcls::OLOGIC::OCE, bcls::OLOGIC::TCE, bcls::OLOGIC::CLKDIV] {
            let bit = xlat_bit_bi(
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OSERDES, false),
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OSERDES, true),
            );
            ctx.insert_bel_input_inv_int(&[tcls::INT], tcid, bslot, pin, bit);
        }
        for pin in [bcls::OLOGIC::OCE, bcls::OLOGIC::TCE] {
            let bit = xlat_bit_bi(
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OLOGIC, false),
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OLOGIC, true),
            );
            ctx.insert_bel_input_inv_int(&[tcls::INT], tcid, bslot, pin, bit);
        }
        for (pin, oused, tused) in [
            (
                bcls::OLOGIC::SR,
                bcls::OLOGIC::FFO_SR_ENABLE,
                bcls::OLOGIC::FFT_SR_ENABLE,
            ),
            (
                bcls::OLOGIC::REV,
                bcls::OLOGIC::FFO_REV_ENABLE,
                bcls::OLOGIC::FFT_REV_ENABLE,
            ),
        ] {
            let oused = ctx.bel_attr_bit(tcid, bslot, oused);
            let tused = ctx.bel_attr_bit(tcid, bslot, tused);
            let mut diff0 =
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OLOGIC, false);
            let mut diff1 =
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OLOGIC, true);
            diff0.apply_bit_diff(oused, true, false);
            diff1.apply_bit_diff(oused, true, false);
            ctx.insert_bel_input_inv_int(&[tcls::INT], tcid, bslot, pin, xlat_bit_bi(diff0, diff1));
            let mut diff0 =
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OSERDES, false);
            let mut diff1 =
                ctx.get_diff_bel_input_inv_special(tcid, bslot, pin, specials::OSERDES, true);
            diff0.apply_bit_diff(oused, true, false);
            diff1.apply_bit_diff(oused, true, false);
            diff0.apply_bit_diff(tused, true, false);
            diff1.apply_bit_diff(tused, true, false);
            ctx.insert_bel_input_inv_int(&[tcls::INT], tcid, bslot, pin, xlat_bit_bi(diff0, diff1));
        }
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OLOGIC::CLK1_INV);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OLOGIC::CLK2_INV);

        let clk1inv = ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::CLK1_INV);
        let clk2inv = ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::CLK2_INV);
        let mut diff = ctx.get_diff_bel_input_inv_special(
            tcid,
            bslot,
            bcls::OLOGIC::CLK,
            specials::OSERDES_SAME_EDGE,
            false,
        );
        diff.apply_bit_diff(clk1inv, false, true);
        diff.apply_bit_diff(clk2inv, false, true);
        diff.assert_empty();
        let diff = ctx.get_diff_bel_input_inv_special(
            tcid,
            bslot,
            bcls::OLOGIC::CLK,
            specials::OSERDES_SAME_EDGE,
            true,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_input_inv_special(
            tcid,
            bslot,
            bcls::OLOGIC::CLK,
            specials::OSERDES_OPPOSITE_EDGE,
            false,
        );
        diff.apply_bit_diff(clk1inv, false, true);
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_input_inv_special(
            tcid,
            bslot,
            bcls::OLOGIC::CLK,
            specials::OSERDES_OPPOSITE_EDGE,
            true,
        );
        diff.apply_bit_diff(clk2inv, false, true);
        diff.assert_empty();

        let item_oq = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFO_SR_SYNC, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFO_SR_SYNC, true),
        );
        let item_tq = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFT_SR_SYNC, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFT_SR_SYNC, true),
        );
        ctx.get_diff_bel_special(tcid, bslot, specials::OSERDES_SRTYPE_ASYNC)
            .assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::OSERDES_SRTYPE_SYNC);
        diff.apply_bitvec_diff(&item_oq, &bits![1; 4], &bits![0; 4]);
        diff.apply_bitvec_diff(&item_tq, &bits![1; 2], &bits![0; 2]);
        diff.assert_empty();
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::OLOGIC::FFO_SR_SYNC, item_oq);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::OLOGIC::FFT_SR_SYNC, item_tq);

        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            bcls::OLOGIC::V4_MUX_O,
            enums::OLOGIC_V4_MUX_O::NONE,
        );
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            bcls::OLOGIC::V4_MUX_T,
            enums::OLOGIC_V4_MUX_T::NONE,
        );

        let mut diff_sdr = ctx.get_diff_bel_special(tcid, bslot, specials::OSERDES_SDR);
        let mut diff_ddr = ctx.get_diff_bel_special(tcid, bslot, specials::OSERDES_DDR);
        diff_sdr.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::OLOGIC::V4_MUX_O),
            enums::OLOGIC_V4_MUX_O::FFO1,
            enums::OLOGIC_V4_MUX_O::D1,
        );
        diff_ddr.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::OLOGIC::V4_MUX_O),
            enums::OLOGIC_V4_MUX_O::FFODDR,
            enums::OLOGIC_V4_MUX_O::D1,
        );
        assert_eq!(diff_sdr, diff_ddr);
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::OLOGIC::FFO_SERDES,
            xlat_bit_wide(diff_sdr),
        );

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OLOGIC::SERDES);
        ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::SERDES_MODE);
        ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::TRISTATE_WIDTH);
        ctx.collect_bel_attr(tcid, bslot, bcls::OLOGIC::INIT_LOADCNT);

        let mut diffs = vec![(enums::IO_DATA_WIDTH::NONE, Diff::default())];
        for val in [
            enums::IO_DATA_WIDTH::_2,
            enums::IO_DATA_WIDTH::_3,
            enums::IO_DATA_WIDTH::_4,
            enums::IO_DATA_WIDTH::_5,
            enums::IO_DATA_WIDTH::_6,
            enums::IO_DATA_WIDTH::_7,
            enums::IO_DATA_WIDTH::_8,
            enums::IO_DATA_WIDTH::_10,
        ] {
            let mut diff = ctx.get_diff_attr_val(tcid, bslot, bcls::OLOGIC::DATA_WIDTH, val);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, bcls::OLOGIC::SERDES),
                true,
                false,
            );
            diffs.push((val, diff));
        }
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::OLOGIC::DATA_WIDTH, xlat_enum_attr(diffs));

        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OLOGIC::FFO_LATCH);
        ctx.collect_bel_attr_bi(tcid, bslot, bcls::OLOGIC::FFT_LATCH);

        let diff_ologic = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            bcls::OLOGIC::FFO_INIT,
            specials::OLOGIC,
            0,
            false,
        );
        let diff_oserdes = ctx
            .get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                bcls::OLOGIC::FFO_INIT,
                specials::OSERDES,
                0,
                false,
            )
            .combine(&!&diff_ologic);
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::OLOGIC::FFO_INIT,
            xlat_bit_wide(!diff_ologic),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::OLOGIC::FFO_INIT_SERDES,
            xlat_bit_wide(!diff_oserdes),
        );
        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            bcls::OLOGIC::FFO_INIT,
            specials::OLOGIC,
            0,
            true,
        )
        .assert_empty();
        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            bcls::OLOGIC::FFO_INIT,
            specials::OSERDES,
            0,
            true,
        )
        .assert_empty();
        let bit = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFT_INIT, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFT_INIT, true),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::OLOGIC::FFT_INIT, bit);
        let bit = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFO_SRVAL, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, bcls::OLOGIC::FFO_SRVAL, true),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::OLOGIC::FFO_SRVAL, bit);

        for spec in [
            specials::OLOGIC_TFF1,
            specials::OLOGIC_TFFDDRA,
            specials::OLOGIC_TFFDDRB,
            specials::OSERDES,
        ] {
            ctx.get_diff_attr_special_bit_bi(tcid, bslot, bcls::OLOGIC::FFT1_SRVAL, spec, 0, true)
                .assert_empty();
        }
        let diff1 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            bcls::OLOGIC::FFT1_SRVAL,
            specials::OLOGIC_TFF1,
            0,
            false,
        );
        let diff2 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            bcls::OLOGIC::FFT1_SRVAL,
            specials::OLOGIC_TFFDDRA,
            0,
            false,
        );
        let diff3 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            bcls::OLOGIC::FFT1_SRVAL,
            specials::OLOGIC_TFFDDRB,
            0,
            false,
        );
        let diff4 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            bcls::OLOGIC::FFT1_SRVAL,
            specials::OSERDES,
            0,
            false,
        );
        assert_eq!(diff3, diff4);
        let diff3 = diff3.combine(&!&diff2);
        let diff2 = diff2.combine(&!&diff1);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFT1_SRVAL, xlat_bit(!diff1));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFT2_SRVAL, xlat_bit(!diff2));
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::OLOGIC::FFT3_SRVAL, xlat_bit(!diff3));

        let mut present_ologic = ctx.get_diff_bel_special(tcid, bslot, specials::OLOGIC);
        let mut present_oserdes = ctx.get_diff_bel_special(tcid, bslot, specials::OSERDES);
        present_ologic.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::OLOGIC::V4_MUX_T),
            enums::OLOGIC_V4_MUX_T::T1,
            enums::OLOGIC_V4_MUX_T::NONE,
        );
        present_oserdes.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::OLOGIC::V4_MUX_O),
            enums::OLOGIC_V4_MUX_O::D1,
            enums::OLOGIC_V4_MUX_O::NONE,
        );
        present_oserdes.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, bcls::OLOGIC::V4_MUX_T),
            enums::OLOGIC_V4_MUX_T::T1,
            enums::OLOGIC_V4_MUX_T::NONE,
        );
        present_oserdes.apply_bit_diff(
            ctx.bel_input_inv(tcid, bslot, bcls::OLOGIC::D1),
            false,
            true,
        );
        present_ologic.assert_empty();
        present_oserdes.assert_empty();
    }
    let mut present_vr = ctx.get_diff_bel_special(tcid, bslots::IOB[0], specials::IOB_VR);
    // I don't care.
    ctx.get_diff_bel_special(tcid, bslots::IOB[0], specials::IOB_VR_CENTER);
    for i in 0..2 {
        let bslot = bslots::IOB[i];
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        ctx.collect_bel_attr_default(tcid, bslot, IOB::PULL, enums::IOB_PULL::NONE);
        let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, IOB::OUTPUT_ENABLE));
        assert_eq!(bits.len(), 2);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE, bits);
        let diff = ctx
            .get_diff_bel_special(tcid, bslot, specials::IOB_IPAD)
            .combine(&!&present);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::VREF_SYSMON, xlat_bit(diff));
        let diff = ctx
            .get_diff_bel_special(tcid, bslot, specials::IOB_CONTINUOUS)
            .combine(&!&present);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::DCIUPDATEMODE_ASREQUIRED, xlat_bit(!diff));
        present.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, IOB::PULL),
            enums::IOB_PULL::NONE,
            enums::IOB_PULL::PULLDOWN,
        );

        let oprog = xlat_bitvec(ctx.get_diffs_bel_special_bits(
            tcid,
            bslot,
            specials::IOB_OPROGRAMMING,
            22,
        ));
        let lvds = oprog[0..4].to_vec();
        let dci_t = oprog[4];
        let dci_mode = BelAttributeEnum {
            bits: oprog[5..8].iter().map(|bit| bit.bit).collect(),
            values: [
                (enums::IOB_DCI_MODE::NONE, bits![0, 0, 0]),
                (enums::IOB_DCI_MODE::OUTPUT, bits![1, 0, 0]),
                (enums::IOB_DCI_MODE::OUTPUT_HALF, bits![0, 1, 0]),
                (enums::IOB_DCI_MODE::TERM_VCC, bits![1, 1, 0]),
                (enums::IOB_DCI_MODE::TERM_SPLIT, bits![0, 0, 1]),
            ]
            .into_iter()
            .collect(),
        };
        let output_misc = oprog[8..10].to_vec();
        let dci_misc = oprog[10..12].to_vec();
        let mut pdrive = oprog[12..17].to_vec();
        let mut ndrive = oprog[17..22].to_vec();
        for bit in &mut pdrive {
            bit.inv = match present.bits.remove(&bit.bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            };
        }
        for bit in &mut ndrive {
            bit.inv = match present.bits.remove(&bit.bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            };
        }
        let (pslew, nslew) = if i == 0 {
            (
                vec![
                    TileBit::new(0, 26, 0).pos(),
                    TileBit::new(0, 26, 6).pos(),
                    TileBit::new(0, 26, 12).pos(),
                    TileBit::new(0, 26, 18).pos(),
                ],
                vec![
                    TileBit::new(0, 26, 1).pos(),
                    TileBit::new(0, 26, 7).neg(),
                    TileBit::new(0, 26, 13).pos(),
                    TileBit::new(0, 25, 19).pos(),
                ],
            )
        } else {
            (
                vec![
                    TileBit::new(0, 26, 79).pos(),
                    TileBit::new(0, 26, 73).pos(),
                    TileBit::new(0, 26, 67).pos(),
                    TileBit::new(0, 26, 61).pos(),
                ],
                vec![
                    TileBit::new(0, 26, 78).pos(),
                    TileBit::new(0, 26, 72).neg(),
                    TileBit::new(0, 26, 66).pos(),
                    TileBit::new(0, 25, 60).pos(),
                ],
            )
        };

        let mut ibuf_mode = vec![(enums::IOB_IBUF_MODE::NONE, Diff::default())];

        for &std in IOSTDS {
            let (spec, row) = get_istd_row(edev, &std);
            let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
            match std.dci {
                DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                DciKind::InputVcc | DciKind::BiVcc => {
                    diff.apply_enum_diff(
                        &dci_mode,
                        enums::IOB_DCI_MODE::TERM_VCC,
                        enums::IOB_DCI_MODE::NONE,
                    );
                    diff.apply_bitvec_diff(&dci_misc, &bits![1, 1], &bits![0, 0]);
                }
                DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                    diff.apply_enum_diff(
                        &dci_mode,
                        enums::IOB_DCI_MODE::TERM_SPLIT,
                        enums::IOB_DCI_MODE::NONE,
                    );
                }
            }
            let mode = if std.diff != DiffKind::None {
                enums::IOB_IBUF_MODE::DIFF
            } else if std.vref.is_some() {
                enums::IOB_IBUF_MODE::VREF
            } else {
                enums::IOB_IBUF_MODE::CMOS
            };
            ibuf_mode.push((mode, diff));

            if std.diff == DiffKind::True {
                let row = get_lvds_row(edev, &std);
                let diff = ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_LVDS, row);
                let value = extract_bitvec_val(&lvds, &bits![0; 4], diff);
                let field = [LVDS_DATA::OUTPUT_C, LVDS_DATA::OUTPUT_T][i];
                ctx.insert_table_bitvec(LVDS_DATA, row, field, value);
                if std.dci == DciKind::None {
                    let diff = ctx.get_diff_bel_special_row(
                        tcid,
                        bslot,
                        specials::IOB_ISTD_LVDS_TERM,
                        row,
                    );
                    let value = extract_bitvec_val(&lvds, &bits![0; 4], diff);
                    let field = [LVDS_DATA::TERM_C, LVDS_DATA::TERM_T][i];
                    ctx.insert_table_bitvec(LVDS_DATA, row, field, value);
                }
            } else {
                let (drives, slews) = if !std.drive.is_empty() {
                    (std.drive, &["SLOW", "FAST"][..])
                } else {
                    (&[0][..], &[""][..])
                };
                for &drive in drives {
                    for &slew in slews {
                        let (spec, row) = get_ostd_row(edev, &std, drive, slew);
                        let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                        if !matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                            for (field, bits) in
                                [(IOB_DATA::PDRIVE, &pdrive), (IOB_DATA::NDRIVE, &ndrive)]
                            {
                                let value: BitVec = bits
                                    .iter()
                                    .map(|&bit| match diff.bits.remove(&bit.bit) {
                                        Some(val) => {
                                            assert_eq!(val, !bit.inv);
                                            true
                                        }
                                        None => false,
                                    })
                                    .collect();
                                ctx.insert_table_bitvec(IOB_DATA, row, field, value);
                            }
                        }
                        let (field_pslew, field_nslew) = if slew == "SLOW" {
                            (IOB_DATA::PSLEW_SLOW, IOB_DATA::NSLEW_SLOW)
                        } else {
                            (IOB_DATA::PSLEW_FAST, IOB_DATA::NSLEW_FAST)
                        };
                        for (field, bits) in [(field_pslew, &pslew), (field_nslew, &nslew)] {
                            let value: BitVec = bits
                                .iter()
                                .map(|&bit| match diff.bits.remove(&bit.bit) {
                                    Some(true) => !bit.inv,
                                    None => bit.inv,
                                    _ => unreachable!(),
                                })
                                .collect();
                            ctx.insert_table_bitvec(IOB_DATA, row, field, value);
                        }
                        let value: BitVec = output_misc
                            .iter()
                            .map(|&bit| match diff.bits.remove(&bit.bit) {
                                Some(true) => true,
                                None => false,
                                _ => unreachable!(),
                            })
                            .collect();
                        ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::OUTPUT_MISC, value);
                        match std.dci {
                            DciKind::None | DciKind::InputVcc | DciKind::InputSplit => {}
                            DciKind::Output => {
                                diff.apply_enum_diff(
                                    &dci_mode,
                                    enums::IOB_DCI_MODE::OUTPUT,
                                    enums::IOB_DCI_MODE::NONE,
                                );
                            }
                            DciKind::OutputHalf => {
                                diff.apply_enum_diff(
                                    &dci_mode,
                                    enums::IOB_DCI_MODE::OUTPUT_HALF,
                                    enums::IOB_DCI_MODE::NONE,
                                );
                            }
                            DciKind::BiVcc => {
                                diff.apply_enum_diff(
                                    &dci_mode,
                                    enums::IOB_DCI_MODE::TERM_VCC,
                                    enums::IOB_DCI_MODE::NONE,
                                );
                                diff.apply_bitvec_diff(&dci_misc, &bits![1, 1], &bits![0, 0]);
                            }
                            DciKind::BiSplit => {
                                diff.apply_enum_diff(
                                    &dci_mode,
                                    enums::IOB_DCI_MODE::TERM_SPLIT,
                                    enums::IOB_DCI_MODE::NONE,
                                );
                            }
                            DciKind::BiSplitT => {
                                diff.apply_enum_diff(
                                    &dci_mode,
                                    enums::IOB_DCI_MODE::TERM_SPLIT,
                                    enums::IOB_DCI_MODE::NONE,
                                );
                                diff.apply_bit_diff(dci_t, true, false);
                            }
                        }
                        diff.assert_empty();
                    }
                }
            }
        }
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::IBUF_MODE, xlat_enum_attr(ibuf_mode));

        for (field, bits) in [
            (IOB_DATA::PDRIVE, &pdrive),
            (IOB_DATA::NDRIVE, &ndrive),
            (IOB_DATA::PSLEW_FAST, &pslew),
            (IOB_DATA::NSLEW_FAST, &nslew),
        ] {
            let value: BitVec = bits
                .iter()
                .map(|&bit| match present_vr.bits.remove(&bit.bit) {
                    Some(true) => !bit.inv,
                    None => bit.inv,
                    _ => unreachable!(),
                })
                .collect();
            ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::VR, field, value);
        }
        present_vr.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, IOB::PULL),
            enums::IOB_PULL::NONE,
            enums::IOB_PULL::PULLDOWN,
        );
        present_vr.apply_enum_diff(
            &dci_mode,
            enums::IOB_DCI_MODE::TERM_SPLIT,
            enums::IOB_DCI_MODE::NONE,
        );
        if i == 0 {
            let mut present_vref = ctx.get_diff_bel_special(tcid, bslot, specials::IOB_VREF);
            present_vref.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, IOB::VREF_SYSMON),
                true,
                false,
            );
            present_vref.apply_enum_diff(
                ctx.bel_attr_enum(tcid, bslot, IOB::PULL),
                enums::IOB_PULL::NONE,
                enums::IOB_PULL::PULLDOWN,
            );

            for (field, bits) in [
                (IOB_DATA::PDRIVE, &pdrive),
                (IOB_DATA::NDRIVE, &ndrive),
                (IOB_DATA::PSLEW_FAST, &pslew),
                (IOB_DATA::NSLEW_FAST, &nslew),
            ] {
                let value: BitVec = bits
                    .iter()
                    .map(|&bit| match present_vref.bits.remove(&bit.bit) {
                        Some(true) => !bit.inv,
                        None => bit.inv,
                        _ => unreachable!(),
                    })
                    .collect();
                ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::VREF, field, value);
            }
            present_vref.assert_empty();
        }

        ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::OUTPUT_T, bits![0; 4]);
        ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::OUTPUT_C, bits![0; 4]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::OUTPUT_MISC, bits![0; 2]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::PDRIVE, bits![0; 5]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::NDRIVE, bits![0; 5]);
        ctx.insert_table_bitvec(
            IOB_DATA,
            IOB_DATA::OFF,
            IOB_DATA::PSLEW_FAST,
            BitVec::from_iter(pslew.iter().map(|bit| bit.inv)),
        );
        ctx.insert_table_bitvec(
            IOB_DATA,
            IOB_DATA::OFF,
            IOB_DATA::NSLEW_FAST,
            BitVec::from_iter(nslew.iter().map(|bit| bit.inv)),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V4_LVDS, lvds);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::DCI_T, dci_t);
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::DCI_MODE, dci_mode);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V4_OUTPUT_MISC, output_misc);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::DCI_MISC, dci_misc);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V4_PDRIVE, pdrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V4_NDRIVE, ndrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V4_PSLEW, pslew);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V4_NSLEW, nslew);
        present.assert_empty();
    }
    let diff1 = present_vr.split_bits_by(|bit| bit.bit.to_idx() >= 40);
    ctx.insert_bel_attr_bool(tcid, bslots::IOB[0], IOB::VR, xlat_bit(present_vr));
    ctx.insert_bel_attr_bool(tcid, bslots::IOB[1], IOB::VR, xlat_bit(diff1));

    let tcid = tcls::HCLK_IO_LVDS;
    let bslot = bslots::LVDS;
    let item = vec![
        TileBit::new(0, 5, 12).pos(),
        TileBit::new(0, 5, 14).pos(),
        TileBit::new(0, 3, 15).pos(),
        TileBit::new(0, 2, 13).pos(),
        TileBit::new(0, 3, 14).pos(),
        TileBit::new(0, 3, 13).pos(),
        TileBit::new(0, 5, 15).pos(),
        TileBit::new(0, 2, 14).pos(),
        TileBit::new(0, 11, 13).pos(),
        TileBit::new(0, 3, 12).pos(),
    ];
    for std in IOSTDS {
        if std.diff == DiffKind::True {
            let row = get_lvds_row(edev, std);
            let diff = ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_LVDS, row);
            let val = extract_bitvec_val(&item, &bits![0; 10], diff);
            ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::LVDSBIAS, val);
        }
    }
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::LVDS_V4::LVDSBIAS, item);
    ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::LVDSBIAS, bits![0; 10]);

    let hclk_center_cnt = ctx.edev.tile_index[tcls::HCLK_IO_CENTER].len();
    for tcid in [
        tcls::HCLK_IO_DCI,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_N,
        tcls::HCLK_IO_DCM_S,
        tcls::HCLK_IO_DCM_N,
    ] {
        let bslot = bslots::DCI;
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::PREF,
            vec![
                TileBit::new(0, 1, 15).pos(),
                TileBit::new(0, 1, 14).pos(),
                TileBit::new(0, 1, 13).pos(),
                TileBit::new(0, 1, 12).pos(),
            ],
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::NREF,
            vec![TileBit::new(0, 27, 15).pos(), TileBit::new(0, 27, 12).pos()],
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::V4_LVDIV2,
            vec![TileBit::new(0, 27, 13).pos(), TileBit::new(0, 27, 14).pos()],
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::V4_PMASK_TERM_VCC,
            vec![
                TileBit::new(0, 4, 12).pos(),
                TileBit::new(0, 4, 13).pos(),
                TileBit::new(0, 4, 14).pos(),
                TileBit::new(0, 4, 15).pos(),
                TileBit::new(0, 2, 12).pos(),
            ],
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::V4_PMASK_TERM_SPLIT,
            vec![
                TileBit::new(0, 10, 13).pos(),
                TileBit::new(0, 10, 14).pos(),
                TileBit::new(0, 11, 14).pos(),
                TileBit::new(0, 10, 15).pos(),
                TileBit::new(0, 11, 15).pos(),
            ],
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::V4_NMASK_TERM_SPLIT,
            vec![
                TileBit::new(0, 12, 12).pos(),
                TileBit::new(0, 12, 13).pos(),
                TileBit::new(0, 12, 14).pos(),
                TileBit::new(0, 12, 15).pos(),
                TileBit::new(0, 10, 12).pos(),
            ],
        );
        ctx.collect_bel_attr(tcid, bslot, bcls::DCI::QUIET);

        let enable =
            if (tcid == tcls::HCLK_IO_CFG_N && hclk_center_cnt != 1) || tcid == tcls::HCLK_IO_DCI {
                TileBit::new(0, 0, 14).pos()
            } else {
                xlat_bit(ctx.get_diff_attr_bool(tcid, bslot, bcls::DCI::ENABLE))
            };
        let mut test_enable = ctx.get_diff_attr_bool(tcid, bslot, bcls::DCI::TEST_ENABLE);
        test_enable.apply_bit_diff(enable, true, false);
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::ENABLE, enable);
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::TEST_ENABLE,
            xlat_bit_wide(test_enable),
        );
        if tcid == tcls::HCLK_IO_CENTER {
            if hclk_center_cnt > 1 {
                ctx.collect_bel_attr(tcid, bslot, bcls::DCI::CASCADE_FROM_BELOW);
            }
            if hclk_center_cnt > 3 {
                ctx.collect_bel_attr(tcid, bslot, bcls::DCI::CASCADE_FROM_ABOVE);
            }
        }
        if tcid == tcls::HCLK_IO_CFG_N && hclk_center_cnt > 1 {
            ctx.collect_bel_attr(tcid, bslot, bcls::DCI::CASCADE_FROM_ABOVE);
        }
    }
    let tcid = tcls::HCLK_IO_DCI;
    let bslot = bslots::DCI;
    for std in IOSTDS {
        if std.dci == DciKind::None {
            continue;
        }
        let (spec, row) = get_istd_row(edev, std);
        let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
        match std.dci {
            DciKind::OutputHalf => {
                let val = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::DCI::V4_LVDIV2),
                    &bits![0; 2],
                    &mut diff,
                );
                ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::LVDIV2, val);
            }
            DciKind::InputVcc | DciKind::BiVcc => {
                let val = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::DCI::V4_PMASK_TERM_VCC),
                    &bits![0; 5],
                    &mut diff,
                );
                ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::PMASK_TERM_VCC, val);
            }
            DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                let val = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::DCI::V4_PMASK_TERM_SPLIT),
                    &bits![0; 5],
                    &mut diff,
                );
                ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::PMASK_TERM_SPLIT, val);
                let val = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslot, bcls::DCI::V4_NMASK_TERM_SPLIT),
                    &bits![0; 5],
                    &mut diff,
                );
                ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::NMASK_TERM_SPLIT, val);
            }
            _ => {}
        }
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::ENABLE, xlat_bit(diff));
    }
    ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::LVDIV2, bits![0; 2]);
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::PMASK_TERM_VCC,
        bits![0; 5],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::PMASK_TERM_SPLIT,
        bits![0; 5],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::NMASK_TERM_SPLIT,
        bits![0; 5],
    );

    let tcid = tcls::CFG;
    let bslot = bslots::MISC_CFG;
    let bits = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::MISC_CFG::DCI_CLK_ENABLE));
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::MISC_CFG::DCI_CLK_ENABLE, bits);
}
