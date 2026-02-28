use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, EnumValueId, TableRowId, TileClassId, WireSlotIdExt},
    grid::{DieId, DieIdExt, RowId, TileCoord, TileIobId},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, FeatureId, OcdMode, SpecialId, extract_bitvec_val, extract_bitvec_val_part,
    xlat_bit, xlat_bit_wide, xlat_bit_wide_bi, xlat_bitvec, xlat_enum_attr, xlat_enum_attr_ocd,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bitvec::BitVec, bsdata::TileBit};
use prjcombine_virtex4::{
    chip::ChipKind,
    defs::{
        self,
        bcls::{self, ILOGIC, IOB, IODELAY_V5 as IODELAY, OLOGIC},
        bslots, devdata, enums, tslots,
        virtex5::{
            tables::{IOB_DATA, LVDS_DATA},
            tcls, wires,
        },
    },
    expanded::IoCoord,
};

use crate::{
    backend::{IseBackend, Key, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        int::BaseIntPip,
        iostd::{DciKind, DiffKind, Iostd},
        props::{
            DynProp,
            bel::BaseBelMode,
            mutex::WireMutexExclusive,
            relation::{FixedRelation, Related, TileRelation},
        },
    },
    virtex4::{io::IsBonded, specials},
};

const IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS33", 3300, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS12", 1200, &[2, 4, 6, 8]),
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

#[derive(Copy, Clone, Debug)]
pub struct HclkIoi;

impl TileRelation for HclkIoi {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[tcrd.die];
        let row = chip.row_hclk(tcrd.row);
        Some(tcrd.with_row(row).tile(defs::tslots::HCLK_BEL))
    }
}

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

    let reg = chip.row_to_reg(tcrd.row);
    let bot = chip.row_reg_bot(reg);
    let rows = if tcrd.col == edev.col_cfg && (reg == chip.reg_cfg || reg == chip.reg_cfg - 2) {
        vec![bot + 15]
    } else if tcrd.col == edev.col_cfg && (reg == chip.reg_cfg - 1 || reg == chip.reg_cfg + 1) {
        vec![bot + 5]
    } else {
        vec![bot + 5, bot + 15]
    };
    rows.into_iter()
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
                .get_bel_name(vref.cell.bel(bslots::IOB[0]))
                .unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelSpecial(tcls::IO, bslots::IOB[0], specials::IOB_VREF),
                rects: backend.edev.tile_bits(vref),
            });
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VrefInternal(pub TileClassId, pub EnumValueId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for VrefInternal {
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
        let hclk_row = chip.row_hclk(tcrd.row);
        // Take exclusive mutex on VREF.
        let hclk_ioi = tcrd.with_row(hclk_row).tile(tslots::HCLK_BEL);
        if edev[hclk_ioi].class != self.0 {
            return None;
        }
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "VREF".to_string()),
            None,
            "EXCLUSIVE",
        );
        let io = edev.get_io_info(IoCoord {
            cell: tcrd.cell,
            iob: TileIobId::from_idx(0),
        });
        let val = match self.1 {
            enums::INTERNAL_VREF::_600 => 600,
            enums::INTERNAL_VREF::_675 => 675,
            enums::INTERNAL_VREF::_750 => 750,
            enums::INTERNAL_VREF::_900 => 900,
            enums::INTERNAL_VREF::_1080 => 1080,
            enums::INTERNAL_VREF::_1100 => 1100,
            enums::INTERNAL_VREF::_1250 => 1250,
            _ => unreachable!(),
        };
        fuzzer = fuzzer.fuzz(Key::InternalVref(io.bank), None, val);
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::BelAttrValue(self.0, bslots::BANK, bcls::BANK::INTERNAL_VREF, self.1),
            rects: edev.tile_bits(hclk_ioi),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
struct Dci(Option<(SpecialId, TableRowId)>);

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

        if tcrd.col == edev.col_cfg {
            // Center column is more trouble than it's worth.
            return None;
        }
        if tcrd.row.to_idx() % 20 == 7 {
            // Not in VR tile please.
            return None;
        }
        // Ensure nothing is placed in VR.
        let vr_row = RowId::from_idx(tcrd.row.to_idx() / 20 * 20 + 7);
        let tile_vr = tcrd.with_row(vr_row).tile(defs::tslots::BEL);
        for bel in [bslots::IOB[0], bslots::IOB[1]] {
            let site = backend.ngrid.get_bel_name(tile_vr.cell.bel(bel)).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Test VR.
        if self.0.is_some() {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelSpecial(tcls::IO, bslots::IOB[0], specials::IOB_VR),
                rects: edev.tile_bits(tile_vr),
            });
        }
        // Take exclusive mutex on bank DCI.
        let hclk_ioi = tcrd
            .cell
            .with_row(chip.row_hclk(vr_row))
            .tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "BANK_DCI".to_string()),
            None,
            "EXCLUSIVE",
        );
        // Test bank DCI.
        if let Some((spec, row)) = self.0 {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelSpecialRow(tcls::HCLK_IO, bslots::DCI, spec, row),
                rects: edev.tile_bits(hclk_ioi),
            });
        }
        // Take shared mutex on global DCI.
        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");
        // Anchor global DCI by putting something in bottom IOB of center column.
        let iob_center = tcrd
            .cell
            .with_cr(edev.col_cfg, chip.row_bufg() - 30)
            .bel(bslots::IOB[0]);
        let site = backend.ngrid.get_bel_name(iob_center).unwrap();
        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_33");
        // Ensure anchor VR IOBs are free.
        for bel in [bslots::IOB[0], bslots::IOB[1]] {
            let iob_center_vr = tcrd
                .cell
                .with_cr(edev.col_cfg, chip.row_bufg() - 30 + 2)
                .bel(bel);
            let site = backend.ngrid.get_bel_name(iob_center_vr).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
struct DiffOut(TableRowId);

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
        let chip = edev.chips[tcrd.die];
        let lvds_row = chip.row_hclk(tcrd.row);
        // Take exclusive mutex on bank LVDS.
        let hclk_ioi = tcrd.with_row(lvds_row).tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "BANK_LVDS".to_string()),
            None,
            "EXCLUSIVE",
        );

        let hclk_ioi_tile = &edev[hclk_ioi];
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::BelSpecialRow(
                if edev.kind == ChipKind::Virtex5 {
                    tcls::HCLK_IO
                } else {
                    hclk_ioi_tile.class
                },
                bslots::BANK,
                specials::IOB_OSTD_LVDS,
                self.0,
            ),
            rects: edev.tile_bits(hclk_ioi),
        });
        Some((fuzzer, false))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DiffOutLegacy(pub &'static str, pub &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DiffOutLegacy {
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
        let lvds_row = chip.row_hclk(tcrd.row);
        // Take exclusive mutex on bank LVDS.
        let hclk_ioi = tcrd.with_row(lvds_row).tile(defs::tslots::HCLK_BEL);
        fuzzer = fuzzer.fuzz(
            Key::TileMutex(hclk_ioi, "BANK_LVDS".to_string()),
            None,
            "EXCLUSIVE",
        );

        let hclk_ioi_tile = &edev[hclk_ioi];
        fuzzer.info.features.push(FuzzerFeature {
            key: DiffKey::Legacy(FeatureId {
                tile: if edev.kind == ChipKind::Virtex5 {
                    "HCLK_IO".into()
                } else {
                    edev.db.tile_classes.key(hclk_ioi_tile.class).clone()
                },
                bel: "LVDS".into(),
                attr: self.0.into(),
                val: self.1.into(),
            }),
            rects: edev.tile_bits(hclk_ioi),
        });
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

    let mut ctx = FuzzCtx::new(session, backend, tcls::IO);

    if devdata_only {
        for i in 0..2 {
            let bel_other = bslots::IODELAY[i ^ 1];
            let mut bctx = ctx.bel(bslots::IODELAY[i]);
            bctx.mode("IODELAY")
                .global("LEGIDELAY", "ENABLE")
                .bel_mode(bel_other, "IODELAY")
                .bel_attr(bel_other, "IDELAY_VALUE", "")
                .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
                .prop(Related::new(
                    HclkIoi,
                    BaseBelMode::new(bslots::IDELAYCTRL, 0, "IDELAYCTRL".into()),
                ))
                .test_bel_attr_val(IODELAY::IDELAY_TYPE, enums::IODELAY_V5_IDELAY_TYPE::DEFAULT)
                .attr("IDELAY_TYPE", "DEFAULT")
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
        let mut bctx = ctx.bel(bslots::ILOGIC[i]);
        let bel_ologic = bslots::OLOGIC[i];
        let bel_iodelay = bslots::IODELAY[i];
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

        for (wt, pin, pin_t, pin_c) in [
            (wires::IMUX_ILOGIC_CLK[i], "CLK", "CLK", "CLK_B"),
            (wires::IMUX_ILOGIC_CLKB[i], "CLKB", "CLKB_B", "CLKB"),
        ] {
            let wt = wt.cell(0);
            for j in 0..2 {
                let wo = wires::IMUX_IO_ICLK_OPTINV[j].cell(0);
                let wf = wires::IMUX_IO_ICLK[j].cell(0);
                bctx.mode("ISERDES")
                    .prop(WireMutexExclusive::new(wt))
                    .prop(WireMutexExclusive::new(wo))
                    .prop(WireMutexExclusive::new(wf))
                    .prop(BaseIntPip::new(wt, wo))
                    .pin(pin)
                    .test_routing(wo, wf.pos())
                    .attr(format!("{pin}INV"), pin_t)
                    .commit();
                bctx.mode("ISERDES")
                    .prop(WireMutexExclusive::new(wt))
                    .prop(WireMutexExclusive::new(wo))
                    .prop(WireMutexExclusive::new(wf))
                    .prop(BaseIntPip::new(wt, wo))
                    .pin(pin)
                    .test_routing(wo, wf.neg())
                    .attr(format!("{pin}INV"), pin_c)
                    .commit();
            }
        }

        bctx.mode("ISERDES")
            .bel_unused(bel_iodelay)
            .test_bel_input_inv_auto(ILOGIC::CLKDIV);

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
            .attr("IFFTYPE", "#FF")
            .pin("SR")
            .test_bel_attr_bits(ILOGIC::FFI_SR_ENABLE)
            .attr("SRUSED", "0")
            .commit();
        bctx.mode("ILOGIC")
            .attr("IFFTYPE", "#FF")
            .pin("REV")
            .test_bel_attr_bits(ILOGIC::FFI_REV_ENABLE)
            .attr("REVUSED", "0")
            .commit();

        bctx.mode("ISERDES")
            .attr("DATA_WIDTH", "2")
            .test_bel_attr_bool_auto(ILOGIC::SERDES, "FALSE", "TRUE");
        bctx.mode("ISERDES").test_bel_attr_auto(ILOGIC::SERDES_MODE);
        bctx.mode("ISERDES").test_bel_attr_subset_auto(
            bcls::ILOGIC::INTERFACE_TYPE,
            &[
                enums::ILOGIC_INTERFACE_TYPE::MEMORY,
                enums::ILOGIC_INTERFACE_TYPE::NETWORKING,
            ],
        );
        bctx.mode("ISERDES")
            .attr("SERDES", "FALSE")
            .test_bel_attr_subset_auto(
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
            .attr("INIT_CE", "11")
            .test_bel_attr_auto(ILOGIC::DATA_RATE);
        bctx.mode("ISERDES")
            .test_bel_attr_auto(ILOGIC::DDR_CLK_EDGE);

        bctx.mode("ILOGIC")
            .attr("IFFTYPE", "DDR")
            .test_bel_attr_auto(ILOGIC::DDR_CLK_EDGE);
        for (val, spec) in [
            ("#FF", specials::ILOGIC_IFFTYPE_FF),
            ("#LATCH", specials::ILOGIC_IFFTYPE_LATCH),
            ("DDR", specials::ILOGIC_IFFTYPE_DDR),
        ] {
            bctx.mode("ILOGIC")
                .test_bel_special(spec)
                .attr("IFFTYPE", val)
                .commit();
        }
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
            bctx.mode("ISERDES")
                .test_bel_attr_bool_rename(aname, attr, "0", "1");
        }

        bctx.mode("ILOGIC")
            .attr("IFFTYPE", "#FF")
            .test_bel_attr_bool_rename("SRTYPE", ILOGIC::FFI_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode("ISERDES").test_bel_attr_bool_rename(
            "SRTYPE",
            ILOGIC::FFI_SR_SYNC,
            "ASYNC",
            "SYNC",
        );
        bctx.mode("ISERDES")
            .attr("DATA_RATE", "SDR")
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

        bctx.mode("ISERDES")
            .pin("OFB")
            .test_bel_attr_bool_special_rename(
                "OFB_USED",
                ILOGIC::I_TSBYPASS_ENABLE,
                specials::ISERDES,
                "FALSE",
                "TRUE",
            );
        for (vname, val) in [
            ("FALSE", enums::ILOGIC_MUX_TSBYPASS::GND),
            ("TRUE", enums::ILOGIC_MUX_TSBYPASS::T),
        ] {
            bctx.mode("ISERDES")
                .pin("TFB")
                .test_bel_attr_special_val(ILOGIC::MUX_TSBYPASS, specials::ISERDES, val)
                .attr("TFB_USED", vname)
                .commit();
        }

        for (spec, val) in [
            (specials::ILOGIC_IOBDELAY_NONE, "NONE"),
            (specials::ILOGIC_IOBDELAY_IFD, "IFD"),
            (specials::ILOGIC_IOBDELAY_IBUF, "IBUF"),
            (specials::ILOGIC_IOBDELAY_BOTH, "BOTH"),
        ] {
            bctx.mode("ISERDES")
                .test_bel_special(spec)
                .attr("IOBDELAY", val)
                .commit();
        }

        bctx.mode("ILOGIC")
            .attr("IMUX", "0")
            .attr("IDELMUX", "1")
            .attr("IFFMUX", "#OFF")
            .pin("D")
            .pin("DDLY")
            .pin("TFB")
            .pin("OFB")
            .pin("O")
            .test_bel_attr_rename("D2OBYP_SEL", ILOGIC::MUX_TSBYPASS);
        bctx.mode("ILOGIC")
            .attr("IFFMUX", "0")
            .attr("IFFTYPE", "#FF")
            .attr("IFFDELMUX", "1")
            .attr("IMUX", "#OFF")
            .pin("D")
            .pin("DDLY")
            .pin("TFB")
            .pin("OFB")
            .test_bel_attr_rename("D2OFFBYP_SEL", ILOGIC::MUX_TSBYPASS);
        bctx.mode("ILOGIC")
            .attr("IDELMUX", "1")
            .pin("D")
            .pin("DDLY")
            .pin("O")
            .pin("TFB")
            .pin("OFB")
            .test_bel_attr_bool_rename("IMUX", ILOGIC::I_TSBYPASS_ENABLE, "1", "0");
        bctx.mode("ILOGIC")
            .attr("IFFDELMUX", "1")
            .attr("IFFTYPE", "#FF")
            .pin("D")
            .pin("DDLY")
            .pin("TFB")
            .pin("OFB")
            .test_bel_attr_bool_rename("IFFMUX", ILOGIC::FFI_TSBYPASS_ENABLE, "1", "0");
        bctx.mode("ILOGIC")
            .attr("IMUX", "1")
            .attr("IFFMUX", "1")
            .attr("IFFTYPE", "#FF")
            .attr("IFFDELMUX", "0")
            .pin("D")
            .pin("DDLY")
            .pin("O")
            .pin("Q1")
            .pin("TFB")
            .pin("OFB")
            .test_bel_attr_bool_rename("IDELMUX", ILOGIC::I_DELAY_ENABLE, "1", "0");
        bctx.mode("ILOGIC")
            .attr("IMUX", "1")
            .attr("IFFMUX", "0")
            .attr("IFFTYPE", "#FF")
            .attr("IDELMUX", "0")
            .attr("D2OFFBYP_SEL", "T")
            .pin("D")
            .pin("DDLY")
            .pin("O")
            .pin("Q1")
            .pin("TFB")
            .pin("OFB")
            .test_bel_attr_bool_rename("IFFDELMUX", ILOGIC::FFI_DELAY_ENABLE, "1", "0");
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
            .attr("ODDR_CLK_EDGE", "SAME_EDGE")
            .attr("OUTFFTYPE", "#FF")
            .attr("OMUX", "OUTFF")
            .pin("CLK")
            .pin("OQ")
            .test_bel_attr_bool_special_rename(
                "CLKINV",
                OLOGIC::CLK1_INV,
                specials::OSERDES_SAME_EDGE,
                "CLK",
                "CLK_B",
            );
        bctx.mode("OLOGIC")
            .attr("ODDR_CLK_EDGE", "OPPOSITE_EDGE")
            .attr("OUTFFTYPE", "#FF")
            .attr("OMUX", "OUTFF")
            .pin("CLK")
            .pin("OQ")
            .test_bel_attr_bool_special_rename(
                "CLKINV",
                OLOGIC::CLK1_INV,
                specials::OSERDES_OPPOSITE_EDGE,
                "CLK",
                "CLK_B",
            );
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("DDR_CLK_EDGE", "SAME_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_bel_attr_bool_special_rename(
                "CLKINV",
                OLOGIC::CLK1_INV,
                specials::OSERDES_SAME_EDGE,
                "CLK",
                "CLK_B",
            );
        bctx.mode("OSERDES")
            .attr("DATA_RATE_OQ", "DDR")
            .attr("DDR_CLK_EDGE", "OPPOSITE_EDGE")
            .pin("OCE")
            .pin("CLK")
            .test_bel_attr_bool_special_rename(
                "CLKINV",
                OLOGIC::CLK1_INV,
                specials::OSERDES_OPPOSITE_EDGE,
                "CLK",
                "CLK_B",
            );

        for (mode, spec, attr) in [
            ("OSERDES", specials::OSERDES_DDR_CLK_EDGE, "DDR_CLK_EDGE"),
            ("OLOGIC", specials::OSERDES_ODDR_CLK_EDGE, "ODDR_CLK_EDGE"),
            ("OLOGIC", specials::OSERDES_TDDR_CLK_EDGE, "TDDR_CLK_EDGE"),
        ] {
            for val in ["SAME_EDGE", "OPPOSITE_EDGE"] {
                bctx.mode(mode)
                    .null_bits()
                    .test_bel_special(spec)
                    .attr(attr, val)
                    .commit();
            }
        }

        for pin in [
            OLOGIC::CLKDIV,
            OLOGIC::D1,
            OLOGIC::D2,
            OLOGIC::D3,
            OLOGIC::D4,
            OLOGIC::D5,
            OLOGIC::D6,
        ] {
            bctx.mode("OSERDES").test_bel_input_inv_auto(pin);
        }

        for pin in [OLOGIC::D1, OLOGIC::D2] {
            bctx.mode("OLOGIC")
                .attr("OUTFFTYPE", "DDR")
                .attr("OMUX", "OUTFF")
                .pin("OQ")
                .test_bel_input_inv_auto(pin);
        }

        bctx.mode("OLOGIC")
            .attr("TMUX", "T1")
            .attr("T1USED", "0")
            .pin("TQ")
            .test_bel_input_inv_auto(OLOGIC::T1);
        bctx.mode("OLOGIC")
            .attr("TFFTYPE", "DDR")
            .attr("TMUX", "TFF")
            .pin("TQ")
            .test_bel_input_inv_auto(OLOGIC::T2);
        bctx.mode("OSERDES")
            .attr("DATA_RATE_TQ", "BUF")
            .test_bel_input_inv_auto(OLOGIC::T1);
        for pin in [OLOGIC::T2, OLOGIC::T3, OLOGIC::T4] {
            bctx.mode("OSERDES").test_bel_input_inv_auto(pin);
        }

        bctx.mode("OLOGIC")
            .attr("OUTFFTYPE", "#FF")
            .test_bel_attr_bool_rename("SRTYPE_OQ", OLOGIC::FFO_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode("OLOGIC")
            .attr("TFFTYPE", "#FF")
            .test_bel_attr_bool_rename("SRTYPE_TQ", OLOGIC::FFT_SR_SYNC, "ASYNC", "SYNC");
        bctx.mode("OSERDES").test_bel_attr_bool_special_rename(
            "SRTYPE",
            OLOGIC::FFO_SR_SYNC,
            specials::OSERDES,
            "ASYNC",
            "SYNC",
        );

        bctx.mode("OLOGIC").test_bel_attr_bool_special_rename(
            "INIT_OQ",
            OLOGIC::FFO_INIT,
            specials::OLOGIC,
            "0",
            "1",
        );
        bctx.mode("OLOGIC").test_bel_attr_bool_special_rename(
            "INIT_TQ",
            OLOGIC::FFT_INIT,
            specials::OLOGIC,
            "0",
            "1",
        );
        bctx.mode("OSERDES").test_bel_attr_bool_special_rename(
            "INIT_OQ",
            OLOGIC::FFO_INIT,
            specials::OSERDES,
            "0",
            "1",
        );
        bctx.mode("OSERDES").test_bel_attr_bool_special_rename(
            "INIT_TQ",
            OLOGIC::FFT_INIT,
            specials::OSERDES,
            "0",
            "1",
        );

        bctx.mode("OLOGIC").test_bel_attr_bool_special_rename(
            "SRVAL_OQ",
            OLOGIC::FFO_SRVAL,
            specials::OLOGIC,
            "0",
            "1",
        );
        bctx.mode("OLOGIC")
            .attr("TFFTYPE", "#FF")
            .attr("TMUX", "TFF")
            .pin("TQ")
            .test_bel_attr_bool_special_rename(
                "SRVAL_TQ",
                OLOGIC::FFT1_SRVAL,
                specials::OLOGIC_FF,
                "0",
                "1",
            );
        bctx.mode("OLOGIC")
            .attr("TFFTYPE", "DDR")
            .attr("TMUX", "TFF")
            .pin("TQ")
            .test_bel_attr_bool_special_rename(
                "SRVAL_TQ",
                OLOGIC::FFT1_SRVAL,
                specials::OLOGIC_DDR,
                "0",
                "1",
            );
        bctx.mode("OSERDES").test_bel_attr_bool_special_rename(
            "SRVAL_OQ",
            OLOGIC::FFO_SRVAL,
            specials::OSERDES,
            "0",
            "1",
        );
        bctx.mode("OSERDES").test_bel_attr_bool_special_rename(
            "SRVAL_TQ",
            OLOGIC::FFT1_SRVAL,
            specials::OSERDES,
            "0",
            "1",
        );

        for (attr, aname) in [
            (OLOGIC::FFO_SR_ENABLE, "OSRUSED"),
            (OLOGIC::FFT_SR_ENABLE, "TSRUSED"),
            (OLOGIC::FFO_REV_ENABLE, "OREVUSED"),
            (OLOGIC::FFT_REV_ENABLE, "TREVUSED"),
        ] {
            bctx.mode("OLOGIC")
                .attr("OUTFFTYPE", "#FF")
                .attr("TFFTYPE", "#FF")
                .pin("OCE")
                .pin("TCE")
                .pin("REV")
                .pin("SR")
                .test_bel_attr_bits(attr)
                .attr(aname, "0")
                .commit();
        }
        for (spec, aname) in [
            (specials::OSERDES_OCEUSED, "OCEUSED"),
            (specials::OSERDES_TCEUSED, "TCEUSED"),
        ] {
            bctx.mode("OLOGIC")
                .null_bits()
                .attr("OUTFFTYPE", "#FF")
                .attr("TFFTYPE", "#FF")
                .pin("OCE")
                .pin("TCE")
                .pin("REV")
                .pin("SR")
                .test_bel_special(spec)
                .attr(aname, "0")
                .commit();
        }

        for (val, vname) in [
            (enums::OLOGIC_V5_MUX_O::LATCH, "#LATCH"),
            (enums::OLOGIC_V5_MUX_O::FF, "#FF"),
            (enums::OLOGIC_V5_MUX_O::DDR, "DDR"),
        ] {
            bctx.mode("OLOGIC")
                .pin("OQ")
                .test_bel_attr_val(OLOGIC::V5_MUX_O, val)
                .attr("OUTFFTYPE", vname)
                .commit();
        }
        for (val, vname) in [
            (enums::OLOGIC_V5_MUX_T::LATCH, "#LATCH"),
            (enums::OLOGIC_V5_MUX_T::FF, "#FF"),
            (enums::OLOGIC_V5_MUX_T::DDR, "DDR"),
        ] {
            bctx.mode("OLOGIC")
                .pin("TQ")
                .test_bel_attr_val(OLOGIC::V5_MUX_T, val)
                .attr("TFFTYPE", vname)
                .commit();
        }

        for (val, vname) in [
            (enums::OLOGIC_V5_MUX_O::SERDES_SDR, "SDR"),
            (enums::OLOGIC_V5_MUX_O::SERDES_DDR, "DDR"),
        ] {
            bctx.mode("OSERDES")
                .test_bel_attr_val(OLOGIC::V5_MUX_O, val)
                .attr("DATA_RATE_OQ", vname)
                .commit();
        }
        for (val, vname) in [
            (enums::OLOGIC_V5_MUX_T::T1, "BUF"),
            (enums::OLOGIC_V5_MUX_T::SERDES_SDR, "SDR"),
            (enums::OLOGIC_V5_MUX_T::SERDES_DDR, "DDR"),
        ] {
            bctx.mode("OSERDES")
                .attr("T1INV", "T1")
                .pin("T1")
                .test_bel_attr_special_val(OLOGIC::V5_MUX_T, specials::OSERDES, val)
                .attr("DATA_RATE_TQ", vname)
                .commit();
        }
        for (val, vname) in [
            (enums::OLOGIC_V5_MUX_O::NONE, "OUTFF"),
            (enums::OLOGIC_V5_MUX_O::D1, "D1"),
        ] {
            bctx.mode("OLOGIC")
                .attr("OSRUSED", "#OFF")
                .attr("OREVUSED", "#OFF")
                .attr("OUTFFTYPE", "#FF")
                .attr("O1USED", "0")
                .attr("D1INV", "D1")
                .pin("D1")
                .pin("OQ")
                .test_bel_attr_val(OLOGIC::V5_MUX_O, val)
                .attr("OMUX", vname)
                .commit();
        }
        for (val, vname) in [
            (enums::OLOGIC_V5_MUX_T::NONE, "TFF"),
            (enums::OLOGIC_V5_MUX_T::T1, "T1"),
        ] {
            bctx.mode("OLOGIC")
                .attr("TSRUSED", "#OFF")
                .attr("TREVUSED", "#OFF")
                .attr("TFFTYPE", "#FF")
                .attr("T1USED", "0")
                .attr("T1INV", "T1")
                .pin("T1")
                .pin("TQ")
                .test_bel_attr_val(OLOGIC::V5_MUX_T, val)
                .attr("TMUX", vname)
                .commit();
        }

        bctx.mode("OLOGIC")
            .global("ENABLEMISR", "Y")
            .test_bel_attr_bool_auto(OLOGIC::MISR_ENABLE, "FALSE", "TRUE");
        bctx.mode("OLOGIC")
            .global("ENABLEMISR", "Y")
            .test_bel_attr_bool_auto(OLOGIC::MISR_ENABLE_FDBK, "FALSE", "TRUE");
        bctx.mode("OLOGIC")
            .global("ENABLEMISR", "Y")
            .test_bel_attr_auto_default(
                OLOGIC::MISR_CLK_SELECT,
                enums::OLOGIC_MISR_CLK_SELECT::NONE,
            );

        bctx.mode("OSERDES")
            .test_bel_attr_bool_auto(OLOGIC::SERDES, "FALSE", "TRUE");
        bctx.mode("OSERDES").test_bel_attr_auto(OLOGIC::SERDES_MODE);
        bctx.mode("OSERDES").test_bel_attr_subset_auto(
            OLOGIC::TRISTATE_WIDTH,
            &[
                enums::OLOGIC_TRISTATE_WIDTH::_1,
                enums::OLOGIC_TRISTATE_WIDTH::_4,
            ],
        );
        bctx.mode("OSERDES").test_bel_attr_subset_auto(
            OLOGIC::DATA_WIDTH,
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
            .test_bel_attr_multi(OLOGIC::INIT_LOADCNT, MultiValue::Bin);
    }

    for i in 0..2 {
        let mut bctx = ctx.bel(bslots::IODELAY[i]);
        let bel_ilogic = bslots::ILOGIC[i];
        let bel_other = bslots::IODELAY[i ^ 1];

        bctx.build()
            .bel_mode(bel_other, "IODELAY")
            .test_bel_special(specials::PRESENT)
            .mode("IODELAY")
            .commit();

        bctx.mode("IODELAY")
            .bel_unused(bel_ilogic)
            .test_bel(bslots::ILOGIC[i])
            .pin("C")
            .test_bel_input_inv_enum("CINV", ILOGIC::CLKDIV, "C", "C_B");
        bctx.mode("IODELAY")
            .test_bel_input_inv_auto(IODELAY::DATAIN);
        bctx.mode("IODELAY").test_bel_attr_bool_auto(
            IODELAY::HIGH_PERFORMANCE_MODE,
            "FALSE",
            "TRUE",
        );
        bctx.mode("IODELAY")
            .test_bel_attr_bool_auto(IODELAY::DELAYCHAIN_OSC, "FALSE", "TRUE");
        bctx.mode("IODELAY")
            .test_bel_attr_auto_default(IODELAY::DELAY_SRC, enums::IODELAY_V5_DELAY_SRC::NONE);
        bctx.mode("IODELAY")
            .test_bel_attr_bits(IODELAY::IDELAY_VALUE_INIT)
            .multi_attr("IDELAY_VALUE", MultiValue::Dec(0), 6);
        bctx.mode("IODELAY")
            .test_bel_attr_multi(IODELAY::ODELAY_VALUE, MultiValue::Dec(0));

        bctx.mode("IODELAY")
            .global("LEGIDELAY", "ENABLE")
            .bel_mode(bel_other, "IODELAY")
            .bel_attr(bel_other, "IDELAY_VALUE", "")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .prop(Related::new(
                HclkIoi,
                BaseBelMode::new(bslots::IDELAYCTRL, 0, "IDELAYCTRL".into()),
            ))
            .test_bel_attr_auto(IODELAY::IDELAY_TYPE);
        bctx.mode("IODELAY")
            .global("LEGIDELAY", "DISABLE")
            .bel_mode(bel_other, "IODELAY")
            .bel_attr(bel_other, "IDELAY_VALUE", "")
            .bel_attr(bel_other, "IDELAY_TYPE", "FIXED")
            .prop(Related::new(
                HclkIoi,
                BaseBelMode::new(bslots::IDELAYCTRL, 0, "IDELAYCTRL".into()),
            ))
            .test_bel_attr_bits_bi(IODELAY::LEGIDELAY, false)
            .attr("IDELAY_TYPE", "FIXED")
            .commit();
    }

    for i in 0..2 {
        let bel = bslots::IOB[i];
        let mut bctx = ctx.bel(bel);
        let bel_ologic = bslots::OLOGIC[i];
        let bel_iodelay = bslots::IODELAY[i];
        let bel_other = bslots::IOB[i ^ 1];
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
            .pin("O")
            .attr("OUSED", "0")
            .attr("OSTANDARD", "LVCMOS18")
            .test_bel_special_bits(specials::IOB_OPROGRAMMING)
            .multi_attr("OPROGRAMMING", MultiValue::Bin, 31);
        bctx.mode("IOB")
            .attr("OUSED", "")
            .pin("I")
            .raw(Key::Package, &package.name)
            .prop(IsBonded(bel))
            .attr("ISTANDARD", "LVCMOS18")
            .test_bel_attr_bits(IOB::I_INV)
            .attr_diff("IMUX", "I", "I_B")
            .commit();
        for &std in IOSTDS {
            let (spec, row) = get_istd_row(edev, &std);
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
                dci_special = Some(Dci(Some((spec, row))));
                dci_special_lite = Some(Dci(None));
            }
            if std.diff != DiffKind::None {
                bctx.mode(["IOBS", "IOBM"][i])
                    .attr("OUSED", "")
                    .pin("I")
                    .pin("DIFFI_IN")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .maybe_prop(dci_special)
                    .bel_mode(bel_other, ["IOBM", "IOBS"][i])
                    .bel_pin(bel_other, "PADOUT")
                    .bel_attr(bel_other, "OUSED", "")
                    .test_bel_special_row(spec, row)
                    .attr("IMUX", "I_B")
                    .attr("DIFFI_INUSED", "0")
                    .attr("ISTANDARD", std.name)
                    .attr("DIFF_TERM", "FALSE")
                    .bel_attr(bel_other, "PADOUTUSED", "0")
                    .bel_attr(bel_other, "ISTANDARD", std.name)
                    .bel_attr(bel_other, "DIFF_TERM", "FALSE")
                    .commit();
                if std.diff == DiffKind::True {
                    bctx.mode(["IOBS", "IOBM"][i])
                        .attr("OUSED", "")
                        .pin("I")
                        .pin("DIFFI_IN")
                        .attr("IMUX", "I_B")
                        .attr("DIFFI_INUSED", "0")
                        .attr("ISTANDARD", std.name)
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .maybe_prop(dci_special_lite)
                        .bel_mode(bel_other, ["IOBM", "IOBS"][i])
                        .bel_pin(bel_other, "PADOUT")
                        .bel_attr(bel_other, "OUSED", "")
                        .bel_attr(bel_other, "PADOUTUSED", "0")
                        .bel_attr(bel_other, "ISTANDARD", std.name)
                        .test_bel_special_row(specials::IOB_ISTD_LVDS_TERM, row)
                        .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                        .bel_attr_diff(bel_other, "DIFF_TERM", "FALSE", "TRUE")
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
                    .attr("IMUX", "I_B")
                    .attr("ISTANDARD", std.name)
                    .commit();
            }
        }
        for &std in IOSTDS {
            if std.diff == DiffKind::True {
                let row = get_lvds_row(edev, &std);
                if i == 1 {
                    bctx.build()
                        .attr("IMUX", "")
                        .attr("OPROGRAMMING", "")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .prop(DiffOut(row))
                        .bel_attr(bel_other, "IMUX", "")
                        .bel_attr(bel_other, "OPROGRAMMING", "")
                        .bel_attr(bel_other, "OSTANDARD", "")
                        .bel_attr(bel_other, "OUSED", "")
                        .test_bel_special_row(specials::IOB_OSTD_LVDS, row)
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
            } else if matches!(
                std.dci,
                DciKind::Output | DciKind::OutputHalf | DciKind::BiSplit | DciKind::BiVcc
            ) {
                let (spec, row) = get_ostd_row(edev, &std, 0, "");
                let (dspec, drow) = get_istd_row(edev, &std);
                bctx.mode("IOB")
                    .pin("O")
                    .attr("IMUX", "")
                    .attr("OPROGRAMMING", "")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .prop(Dci(Some((dspec, drow))))
                    .test_bel_special_row(spec, row)
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", std.name)
                    .commit();
            } else if !std.drive.is_empty() {
                for &drive in std.drive {
                    for slew in ["SLOW", "FAST"] {
                        let (spec, row) = get_ostd_row(edev, &std, drive, slew);
                        bctx.mode("IOB")
                            .pin("O")
                            .attr("IMUX", "")
                            .attr("OPROGRAMMING", "")
                            .test_bel_special_row(spec, row)
                            .attr("OUSED", "0")
                            .attr("OSTANDARD", std.name)
                            .attr(
                                "DRIVE",
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
                    .attr("IMUX", "")
                    .attr("OPROGRAMMING", "")
                    .test_bel_special_row(spec, row)
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", std.name)
                    .commit();
            }
        }

        for (row, std, vref) in [
            (IOB_DATA::HSTL_I, "HSTL_I", enums::INTERNAL_VREF::_750),
            (IOB_DATA::HSTL_III, "HSTL_III", enums::INTERNAL_VREF::_900),
            (
                IOB_DATA::HSTL_III_18,
                "HSTL_III_18",
                enums::INTERNAL_VREF::_1080,
            ),
            (IOB_DATA::SSTL2_I, "SSTL2_I", enums::INTERNAL_VREF::_1250),
        ] {
            bctx.mode("IOB")
                .attr("OUSED", "")
                .pin("I")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .prop(VrefInternal(tcls::HCLK_IO, vref))
                .test_bel_special_row(specials::IOB_ISTD, row)
                .attr("IMUX", "I_B")
                .attr("ISTANDARD", std)
                .commit();
        }

        bctx.build()
            .mutex("O_IOB", "OQ")
            .test_bel_attr_bits_bi(IOB::OUTPUT_DELAY, false)
            .pip((bel_ologic, "O_IOB"), (bel_ologic, "OQ"))
            .commit();
        bctx.build()
            .mutex("O_IOB", "DATAOUT")
            .test_bel_attr_bits_bi(IOB::OUTPUT_DELAY, true)
            .pip((bel_ologic, "O_IOB"), (bel_iodelay, "DATAOUT"))
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        ctx.build()
            .global("ENABLEMISR", "Y")
            .extra_tiles_by_bel_attr_bits(bslots::OLOGIC[0], OLOGIC::MISR_RESET)
            .test_global_special(specials::MISR_RESET)
            .global_diff("MISRRESET", "N", "Y")
            .commit();
    }

    for tcid in [
        tcls::HCLK_IO,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_S,
        tcls::HCLK_IO_CFG_N,
        tcls::HCLK_IO_CMT_S,
        tcls::HCLK_IO_CMT_N,
    ] {
        if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) {
            let mut bctx = ctx.bel(bslots::DCI);
            bctx.build()
                .global_mutex("GLOBAL_DCI", "NOPE")
                .test_bel_attr_bits(bcls::DCI::TEST_ENABLE)
                .mode("DCI")
                .commit();
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .extra_tiles_by_bel_attr_bits(bslots::DCI, bcls::DCI::QUIET)
        .test_global_special(specials::DCI_QUIET)
        .global_diff("DCIUPDATEMODE", "CONTINUOUS", "QUIET")
        .commit();
    for (spec, bank) in [
        (specials::CENTER_DCI_BANK3, 3),
        (specials::CENTER_DCI_BANK4, 4),
    ] {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        if bank == 3 && chip.row_bufg() + 30 > chip.rows().last().unwrap() {
            continue;
        }
        let mut builder = ctx
            .build()
            .raw(Key::Package, &package.name)
            .extra_tile_attr_bits(
                FixedRelation(edev.tile_cfg(die)),
                bslots::MISC_CFG,
                bcls::MISC_CFG::DCI_CLK_ENABLE,
            );

        // Find VR and IO rows.
        let (vr_row, io_row) = match bank {
            3 => (chip.row_bufg() + 30 - 3, chip.row_bufg() + 30 - 1),
            4 => (chip.row_bufg() - 30 + 2, chip.row_bufg() - 30),
            _ => unreachable!(),
        };
        let vr_tile = die.cell(edev.col_cfg, vr_row).tile(defs::tslots::BEL);
        let io_tile = die.cell(edev.col_cfg, io_row).tile(defs::tslots::BEL);
        let io_bel = io_tile.cell.bel(bslots::IOB[0]);
        let hclk_row = chip.row_hclk(io_row);
        let hclk_tcrd = die
            .cell(edev.col_cfg, hclk_row)
            .tile(defs::tslots::HCLK_BEL);

        // Ensure nothing is placed in VR.
        for bel in [bslots::IOB[0], bslots::IOB[1]] {
            let site = backend.ngrid.get_bel_name(vr_tile.cell.bel(bel)).unwrap();
            builder = builder.raw(Key::SiteMode(site), None);
        }
        builder = builder.extra_fixed_bel_special(vr_tile, bslots::IOB[0], specials::IOB_VR);

        // Set up hclk.
        builder = builder.extra_fixed_bel_attr_bits(hclk_tcrd, bslots::DCI, bcls::DCI::ENABLE);

        // Set up the IO and fire.
        let site = backend.ngrid.get_bel_name(io_bel).unwrap();
        builder
            .raw(Key::SiteMode(site), "IOB")
            .raw(Key::SitePin(site, "O".into()), true)
            .raw(Key::SiteAttr(site, "IMUX".into()), None)
            .raw(Key::SiteAttr(site, "OPROGRAMMING".into()), None)
            .raw_diff(Key::SiteAttr(site, "OUSED".into()), None, "0")
            .raw_diff(Key::SiteAttr(site, "OSTANDARD".into()), None, "LVDCI_33")
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
    for (spec, bank_from, bank_to) in [
        (specials::CENTER_DCI_BANK1, 3, 1),
        (specials::CENTER_DCI_BANK2, 4, 2),
    ] {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        if bank_from == 3 && chip.row_bufg() + 30 > chip.rows().last().unwrap() {
            continue;
        }
        let mut builder = ctx.build().raw(Key::Package, &package.name);

        let io_row_from = match bank_from {
            3 => chip.row_bufg() + 30 - 1,
            4 => chip.row_bufg() - 30,
            _ => unreachable!(),
        };
        let io_row_to = match bank_to {
            1 => chip.row_bufg() + 10,
            2 => chip.row_bufg() - 11,
            _ => unreachable!(),
        };
        let io_tile_from = die.cell(edev.col_cfg, io_row_from).tile(defs::tslots::BEL);
        let io_bel_from = io_tile_from.cell.bel(bslots::IOB[0]);
        let io_tile_to = die.cell(edev.col_cfg, io_row_to).tile(defs::tslots::BEL);
        let io_bel_to = io_tile_to.cell.bel(bslots::IOB[0]);
        let hclk_row_to = chip.row_hclk(io_row_to);
        let hclk_tcrd_to = die
            .cell(edev.col_cfg, hclk_row_to)
            .tile(defs::tslots::HCLK_BEL);

        // Ensure nothing else in the bank.
        let bot = chip.row_reg_bot(chip.row_to_reg(io_row_from));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            for bel in [bslots::IOB[0], bslots::IOB[1]] {
                if row == io_row_from && bel == bslots::IOB[0] {
                    continue;
                }
                if let Some(site) = backend
                    .ngrid
                    .get_bel_name(io_bel_from.cell.with_row(row).bel(bel))
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
            .raw(
                Key::SiteAttr(site, "OSTANDARD".into()),
                if edev.kind == prjcombine_virtex4::chip::ChipKind::Virtex6 {
                    "LVDCI_25"
                } else {
                    "LVDCI_33"
                },
            )
            // Take shared mutex on global DCI.
            .raw(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");

        // Ensure nothing else in the bank.
        let bot = chip.row_reg_bot(chip.row_to_reg(io_row_to));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            for bel in [bslots::IOB[0], bslots::IOB[1]] {
                if row == io_row_to && bel == bslots::IOB[0] {
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
            .raw_diff(Key::DciCascade(bank_to), None, bank_from)
            .extra_fixed_bel_special_row(
                io_tile_to,
                bslots::IOB[0],
                specials::IOB_OSTD,
                IOB_DATA::LVDCI_33,
            )
            .extra_fixed_bel_attr_bits(
                hclk_tcrd_to,
                bslots::DCI,
                if bank_to == 1 {
                    bcls::DCI::CASCADE_FROM_ABOVE
                } else {
                    bcls::DCI::CASCADE_FROM_BELOW
                },
            )
            .test_global_special(spec)
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx, devdata_only: bool) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    let tcid = tcls::IO;

    if devdata_only {
        for i in 0..2 {
            let bslot = bslots::IODELAY[i];

            let mut diff_default = ctx.get_diff_attr_val(
                tcid,
                bslot,
                IODELAY::IDELAY_TYPE,
                enums::IODELAY_V5_IDELAY_TYPE::DEFAULT,
            );
            let val = extract_bitvec_val_part(
                ctx.bel_attr_bitvec(tcid, bslot, IODELAY::IDELAY_VALUE_INIT),
                &bits![0; 6],
                &mut diff_default,
            );
            ctx.insert_devdata_bitvec(devdata::IODELAY_V5_IDELAY_DEFAULT, val);
            let val = extract_bitvec_val_part(
                ctx.bel_attr_bitvec(tcid, bslot, IODELAY::IDELAY_VALUE_CUR),
                &bits![0; 6],
                &mut diff_default,
            );
            ctx.insert_devdata_bitvec(devdata::IODELAY_V5_IDELAY_DEFAULT, val);
        }
        return;
    }

    ctx.collect_mux(tcid, wires::IMUX_IO_ICLK_OPTINV[0].cell(0));
    ctx.collect_mux(tcid, wires::IMUX_IO_ICLK_OPTINV[1].cell(0));

    for i in 0..2 {
        let bslot = bslots::ILOGIC[i];
        ctx.collect_bel_input_inv_bi(tcid, bslot, ILOGIC::CLKDIV);

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

        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::SERDES);
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
            diffs.push((
                val,
                ctx.get_diff_attr_val(tcid, bslot, ILOGIC::DATA_WIDTH, val),
            ));
        }
        let mut bits = xlat_enum_attr(diffs.clone()).bits;
        bits.swap(0, 1);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            ILOGIC::DATA_WIDTH,
            xlat_enum_attr_ocd(diffs, OcdMode::FixedOrder(&bits)),
        );

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

        ctx.collect_bel_attr(tcid, bslot, ILOGIC::DDR_CLK_EDGE);
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::FFI_REV_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, ILOGIC::FFI_SR_ENABLE);

        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_IFFTYPE_LATCH);
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::DDR_CLK_EDGE),
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE,
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE_PIPELINED,
        );
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_IFFTYPE_FF);
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::DDR_CLK_EDGE),
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE,
            enums::ILOGIC_DDR_CLK_EDGE::SAME_EDGE_PIPELINED,
        );
        ctx.insert_bel_attr_bool(tcid, bslot, ILOGIC::FFI_LATCH, xlat_bit(!diff));
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_IFFTYPE_DDR);
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::INTERFACE_TYPE),
            enums::ILOGIC_INTERFACE_TYPE::NETWORKING,
            enums::ILOGIC_INTERFACE_TYPE::MEMORY,
        );
        ctx.insert_bel_attr_bool(tcid, bslot, ILOGIC::FFI_LATCH, xlat_bit(!diff));

        let mut diffs = vec![];
        for val in [enums::IO_DATA_RATE::SDR, enums::IO_DATA_RATE::DDR] {
            let mut diff = ctx.get_diff_attr_val(tcid, bslot, ILOGIC::DATA_RATE, val);
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, ILOGIC::FFI_SR_ENABLE),
                true,
                false,
            );
            diff.apply_bit_diff(
                ctx.bel_attr_bit(tcid, bslot, ILOGIC::FFI_LATCH),
                false,
                true,
            );
            diffs.push((val, diff));
        }
        ctx.insert_bel_attr_enum(tcid, bslot, ILOGIC::DATA_RATE, xlat_enum_attr(diffs));

        ctx.collect_bel_attr(tcid, bslot, ILOGIC::MUX_TSBYPASS);
        let item = xlat_enum_attr(vec![
            (
                enums::ILOGIC_MUX_TSBYPASS::T,
                ctx.get_diff_attr_special_val(
                    tcid,
                    bslot,
                    ILOGIC::MUX_TSBYPASS,
                    specials::ISERDES,
                    enums::ILOGIC_MUX_TSBYPASS::T,
                ),
            ),
            (
                enums::ILOGIC_MUX_TSBYPASS::GND,
                ctx.get_diff_attr_special_val(
                    tcid,
                    bslot,
                    ILOGIC::MUX_TSBYPASS,
                    specials::ISERDES,
                    enums::ILOGIC_MUX_TSBYPASS::GND,
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
        diff.assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC_IOBDELAY_IFD);
        diff.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, ILOGIC::FFI_DELAY_ENABLE),
            true,
            false,
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
        diff.assert_empty();

        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::I_TSBYPASS_ENABLE);
        ctx.collect_bel_attr_bi(tcid, bslot, ILOGIC::FFI_TSBYPASS_ENABLE);

        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::I_TSBYPASS_ENABLE,
            specials::ISERDES,
            0,
            false,
        )
        .assert_empty();
        let mut diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            ILOGIC::I_TSBYPASS_ENABLE,
            specials::ISERDES,
            0,
            true,
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

        ctx.get_diff_bel_special(tcid, bslot, specials::ILOGIC)
            .assert_empty();
        let mut diff = ctx.get_diff_bel_special(tcid, bslot, specials::ISERDES);
        diff.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, ILOGIC::MUX_TSBYPASS),
            enums::ILOGIC_MUX_TSBYPASS::GND,
            enums::ILOGIC_MUX_TSBYPASS::T,
        );
        diff.assert_empty();

        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            ILOGIC::READBACK_I,
            TileBit::new(0, 29, [13, 50][i]).pos(),
        );
    }

    for i in 0..2 {
        let bslot = bslots::OLOGIC[i];
        let mut present_ologic = ctx.get_diff_bel_special(tcid, bslot, specials::OLOGIC);
        let mut present_oserdes = ctx.get_diff_bel_special(tcid, bslot, specials::OSERDES);

        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::CLK1_INV,
            specials::OSERDES_SAME_EDGE,
            0,
            true,
        )
        .assert_empty();
        for pin in [
            OLOGIC::D1,
            OLOGIC::D2,
            OLOGIC::D3,
            OLOGIC::D4,
            OLOGIC::D5,
            OLOGIC::D6,
            OLOGIC::T1,
            OLOGIC::T2,
            OLOGIC::T3,
            OLOGIC::T4,
            OLOGIC::CLKDIV,
        ] {
            ctx.collect_bel_input_inv_bi(tcid, bslot, pin);
        }
        let diff_clk1 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::CLK1_INV,
            specials::OSERDES_OPPOSITE_EDGE,
            0,
            false,
        );
        let diff_clk2 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::CLK1_INV,
            specials::OSERDES_OPPOSITE_EDGE,
            0,
            true,
        );
        let diff_clk12 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::CLK1_INV,
            specials::OSERDES_SAME_EDGE,
            0,
            false,
        );
        assert_eq!(diff_clk12, diff_clk1.combine(&diff_clk2));
        ctx.insert_bel_attr_bool(tcid, bslot, OLOGIC::CLK1_INV, xlat_bit(!diff_clk1));
        ctx.insert_bel_attr_bool(tcid, bslot, OLOGIC::CLK2_INV, xlat_bit(!diff_clk2));

        ctx.collect_bel_attr(tcid, bslot, OLOGIC::FFO_SR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, OLOGIC::FFT_SR_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, OLOGIC::FFO_REV_ENABLE);
        ctx.collect_bel_attr(tcid, bslot, OLOGIC::FFT_REV_ENABLE);

        let diff_d1 =
            ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_O, enums::OLOGIC_V5_MUX_O::D1);
        let diff_serdes_sdr = ctx
            .get_diff_attr_val(
                tcid,
                bslot,
                OLOGIC::V5_MUX_O,
                enums::OLOGIC_V5_MUX_O::SERDES_SDR,
            )
            .combine(&diff_d1);
        let diff_serdes_ddr = ctx
            .get_diff_attr_val(
                tcid,
                bslot,
                OLOGIC::V5_MUX_O,
                enums::OLOGIC_V5_MUX_O::SERDES_DDR,
            )
            .combine(&diff_d1);
        let (diff_serdes_sdr, diff_serdes_ddr, mut diff_off_serdes) =
            Diff::split(diff_serdes_sdr, diff_serdes_ddr);
        diff_off_serdes.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, OLOGIC::FFO_SR_ENABLE),
            true,
            false,
        );
        diff_off_serdes.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, OLOGIC::FFO_REV_ENABLE),
            true,
            false,
        );
        let diff_latch =
            ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_O, enums::OLOGIC_V5_MUX_O::LATCH);
        let diff_ff =
            ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_O, enums::OLOGIC_V5_MUX_O::FF);
        let diff_ddr =
            ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_O, enums::OLOGIC_V5_MUX_O::DDR);
        ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_O, enums::OLOGIC_V5_MUX_O::NONE)
            .assert_empty();
        present_oserdes = present_oserdes.combine(&!&diff_d1);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            OLOGIC::V5_MUX_O,
            xlat_enum_attr(vec![
                (enums::OLOGIC_V5_MUX_O::NONE, Diff::default()),
                (enums::OLOGIC_V5_MUX_O::D1, diff_d1),
                (enums::OLOGIC_V5_MUX_O::SERDES_SDR, diff_serdes_sdr),
                (enums::OLOGIC_V5_MUX_O::SERDES_DDR, diff_serdes_ddr),
                (enums::OLOGIC_V5_MUX_O::FF, diff_ff),
                (enums::OLOGIC_V5_MUX_O::DDR, diff_ddr),
                (enums::OLOGIC_V5_MUX_O::LATCH, diff_latch),
            ]),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            OLOGIC::FFO_SERDES,
            xlat_bit_wide(diff_off_serdes),
        );

        let diff_t1 =
            ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_T, enums::OLOGIC_V5_MUX_T::T1);
        let diff_serdes_buf = ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            OLOGIC::V5_MUX_T,
            specials::OSERDES,
            enums::OLOGIC_V5_MUX_T::T1,
        );
        let mut diff_serdes_sdr = ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            OLOGIC::V5_MUX_T,
            specials::OSERDES,
            enums::OLOGIC_V5_MUX_T::SERDES_SDR,
        );
        let mut diff_serdes_ddr = ctx.get_diff_attr_special_val(
            tcid,
            bslot,
            OLOGIC::V5_MUX_T,
            specials::OSERDES,
            enums::OLOGIC_V5_MUX_T::SERDES_DDR,
        );
        diff_serdes_sdr.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, OLOGIC::FFT_SR_ENABLE),
            true,
            false,
        );
        diff_serdes_sdr.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, OLOGIC::FFT_REV_ENABLE),
            true,
            false,
        );
        diff_serdes_ddr.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, OLOGIC::FFT_SR_ENABLE),
            true,
            false,
        );
        diff_serdes_ddr.apply_bit_diff(
            ctx.bel_attr_bit(tcid, bslot, OLOGIC::FFT_REV_ENABLE),
            true,
            false,
        );
        let diff_latch =
            ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_T, enums::OLOGIC_V5_MUX_T::LATCH);
        let diff_ff =
            ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_T, enums::OLOGIC_V5_MUX_T::FF);
        let diff_ddr =
            ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_T, enums::OLOGIC_V5_MUX_T::DDR);
        ctx.get_diff_attr_val(tcid, bslot, OLOGIC::V5_MUX_T, enums::OLOGIC_V5_MUX_T::NONE)
            .assert_empty();
        present_oserdes = present_oserdes.combine(&!&diff_t1);
        present_ologic = present_ologic.combine(&!&diff_t1);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            OLOGIC::V5_MUX_T,
            xlat_enum_attr(vec![
                (enums::OLOGIC_V5_MUX_T::NONE, Diff::default()),
                (enums::OLOGIC_V5_MUX_T::T1, diff_t1),
                (enums::OLOGIC_V5_MUX_T::T1, diff_serdes_buf),
                (enums::OLOGIC_V5_MUX_T::SERDES_DDR, diff_serdes_ddr),
                (enums::OLOGIC_V5_MUX_T::FF, diff_serdes_sdr),
                (enums::OLOGIC_V5_MUX_T::FF, diff_ff),
                (enums::OLOGIC_V5_MUX_T::DDR, diff_ddr),
                (enums::OLOGIC_V5_MUX_T::LATCH, diff_latch),
            ]),
        );

        ctx.collect_bel_attr(tcid, bslot, OLOGIC::INIT_LOADCNT);
        present_oserdes.apply_bitvec_diff(
            ctx.bel_attr_bitvec(tcid, bslot, OLOGIC::INIT_LOADCNT),
            &bits![0; 4],
            &bits![1; 4],
        );

        present_ologic.assert_empty();
        present_oserdes.assert_empty();

        let item_oq = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, OLOGIC::FFO_SR_SYNC, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, OLOGIC::FFO_SR_SYNC, true),
        );
        let item_tq = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, OLOGIC::FFT_SR_SYNC, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, OLOGIC::FFT_SR_SYNC, true),
        );
        ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::FFO_SR_SYNC,
            specials::OSERDES,
            0,
            false,
        )
        .assert_empty();
        let mut diff = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::FFO_SR_SYNC,
            specials::OSERDES,
            0,
            true,
        );
        diff.apply_bitvec_diff(&item_oq, &bits![1; 4], &bits![0; 4]);
        diff.apply_bitvec_diff(&item_tq, &bits![1; 2], &bits![0; 2]);
        diff.assert_empty();
        ctx.insert_bel_attr_bitvec(tcid, bslot, OLOGIC::FFO_SR_SYNC, item_oq);
        ctx.insert_bel_attr_bitvec(tcid, bslot, OLOGIC::FFT_SR_SYNC, item_tq);

        let diff_ologic = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::FFO_INIT,
            specials::OLOGIC,
            0,
            false,
        );
        let diff_oserdes = ctx
            .get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFO_INIT,
                specials::OSERDES,
                0,
                false,
            )
            .combine(&!&diff_ologic);
        ctx.insert_bel_attr_bitvec(tcid, bslot, OLOGIC::FFO_INIT, xlat_bit_wide(!diff_ologic));
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            OLOGIC::FFO_INIT_SERDES,
            xlat_bit_wide(!diff_oserdes),
        );
        ctx.get_diff_attr_special_bit_bi(tcid, bslot, OLOGIC::FFO_INIT, specials::OLOGIC, 0, true)
            .assert_empty();
        ctx.get_diff_attr_special_bit_bi(tcid, bslot, OLOGIC::FFO_INIT, specials::OSERDES, 0, true)
            .assert_empty();
        let item = xlat_bit_wide_bi(
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFT_INIT,
                specials::OLOGIC,
                0,
                false,
            ),
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFT_INIT,
                specials::OLOGIC,
                0,
                true,
            ),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, OLOGIC::FFT_INIT, item);
        let item = xlat_bit_wide_bi(
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFT_INIT,
                specials::OSERDES,
                0,
                false,
            ),
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFT_INIT,
                specials::OSERDES,
                0,
                true,
            ),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, OLOGIC::FFT_INIT, item);

        let item = xlat_bit_wide_bi(
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFO_SRVAL,
                specials::OLOGIC,
                0,
                false,
            ),
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFO_SRVAL,
                specials::OLOGIC,
                0,
                true,
            ),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, OLOGIC::FFO_SRVAL, item);
        let item = xlat_bit_wide_bi(
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFO_SRVAL,
                specials::OSERDES,
                0,
                false,
            ),
            ctx.get_diff_attr_special_bit_bi(
                tcid,
                bslot,
                OLOGIC::FFO_SRVAL,
                specials::OSERDES,
                0,
                true,
            ),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, OLOGIC::FFO_SRVAL, item);

        for spec in [specials::OLOGIC_FF, specials::OLOGIC_DDR, specials::OSERDES] {
            ctx.get_diff_attr_special_bit_bi(tcid, bslot, OLOGIC::FFT1_SRVAL, spec, 0, true)
                .assert_empty();
        }
        let diff1 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::FFT1_SRVAL,
            specials::OLOGIC_FF,
            0,
            false,
        );
        let diff2 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::FFT1_SRVAL,
            specials::OLOGIC_DDR,
            0,
            false,
        );
        let diff3 = ctx.get_diff_attr_special_bit_bi(
            tcid,
            bslot,
            OLOGIC::FFT1_SRVAL,
            specials::OSERDES,
            0,
            false,
        );
        assert_eq!(diff2, diff3);
        let diff2 = diff2.combine(&!&diff1);
        ctx.insert_bel_attr_bool(tcid, bslot, OLOGIC::FFT1_SRVAL, xlat_bit(!diff1));
        let bits = xlat_bit_wide(!diff2);
        assert_eq!(bits.len(), 2);
        ctx.insert_bel_attr_bool(tcid, bslot, OLOGIC::FFT2_SRVAL, bits[1]);
        ctx.insert_bel_attr_bool(tcid, bslot, OLOGIC::FFT3_SRVAL, bits[0]);

        ctx.collect_bel_attr_bi(tcid, bslot, OLOGIC::SERDES);
        ctx.collect_bel_attr(tcid, bslot, OLOGIC::SERDES_MODE);
        ctx.collect_bel_attr_subset(
            tcid,
            bslot,
            OLOGIC::TRISTATE_WIDTH,
            &[
                enums::OLOGIC_TRISTATE_WIDTH::_1,
                enums::OLOGIC_TRISTATE_WIDTH::_4,
            ],
        );
        ctx.collect_bel_attr_subset_default_ocd(
            tcid,
            bslot,
            OLOGIC::DATA_WIDTH,
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
            enums::IO_DATA_WIDTH::NONE,
            OcdMode::ValueOrder,
        );

        ctx.collect_bel_attr_bi(tcid, bslot, OLOGIC::MISR_ENABLE);
        ctx.collect_bel_attr_bi(tcid, bslot, OLOGIC::MISR_ENABLE_FDBK);
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            OLOGIC::MISR_CLK_SELECT,
            enums::OLOGIC_MISR_CLK_SELECT::NONE,
        );
    }
    let mut diff = ctx.get_diff_attr_bool(tcid, bslots::OLOGIC[0], OLOGIC::MISR_RESET);
    let diff1 = diff.split_bits_by(|bit| bit.bit.to_idx() >= 32);
    ctx.insert_bel_attr_bool(tcid, bslots::OLOGIC[0], OLOGIC::MISR_RESET, xlat_bit(diff));
    ctx.insert_bel_attr_bool(tcid, bslots::OLOGIC[1], OLOGIC::MISR_RESET, xlat_bit(diff1));

    for i in 0..2 {
        let bslot = bslots::IODELAY[i];
        ctx.collect_bel_input_inv_bi(tcid, bslot, IODELAY::DATAIN);
        ctx.collect_bel_attr_bi(tcid, bslot, IODELAY::HIGH_PERFORMANCE_MODE);
        ctx.collect_bel_attr_bi(tcid, bslot, IODELAY::DELAYCHAIN_OSC);
        ctx.collect_bel_attr_default(
            tcid,
            bslot,
            IODELAY::DELAY_SRC,
            enums::IODELAY_V5_DELAY_SRC::NONE,
        );
        ctx.collect_bel_attr(tcid, bslot, IODELAY::ODELAY_VALUE);

        let mut diffs_a = vec![];
        let mut diffs_b = vec![];
        for diff in ctx.get_diffs_attr_bits(tcid, bslot, IODELAY::IDELAY_VALUE_INIT, 6) {
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
            IODELAY::IDELAY_VALUE_INIT,
            xlat_bitvec(diffs_a),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, IODELAY::IDELAY_VALUE_CUR, xlat_bitvec(diffs_b));

        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        present.apply_bitvec_diff_int(
            ctx.bel_attr_bitvec(tcid, bslot, IODELAY::IDELAY_VALUE_CUR),
            0,
            0x3f,
        );
        present.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, IODELAY::DELAY_SRC),
            enums::IODELAY_V5_DELAY_SRC::NONE,
            enums::IODELAY_V5_DELAY_SRC::DATAIN,
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, IODELAY::ENABLE, xlat_bit_wide(present));

        let diff = ctx.get_diff_attr_bool_bi(tcid, bslot, IODELAY::LEGIDELAY, false);
        ctx.insert_bel_attr_bool(tcid, bslot, IODELAY::LEGIDELAY, xlat_bit(!diff));

        ctx.get_diff_attr_val(
            tcid,
            bslot,
            IODELAY::IDELAY_TYPE,
            enums::IODELAY_V5_IDELAY_TYPE::FIXED,
        )
        .assert_empty();
        let diff_variable = ctx.get_diff_attr_val(
            tcid,
            bslot,
            IODELAY::IDELAY_TYPE,
            enums::IODELAY_V5_IDELAY_TYPE::VARIABLE,
        );
        let mut diff_default = ctx.get_diff_attr_val(
            tcid,
            bslot,
            IODELAY::IDELAY_TYPE,
            enums::IODELAY_V5_IDELAY_TYPE::DEFAULT,
        );
        let val = extract_bitvec_val_part(
            ctx.bel_attr_bitvec(tcid, bslot, IODELAY::IDELAY_VALUE_INIT),
            &bits![0; 6],
            &mut diff_default,
        );
        ctx.insert_devdata_bitvec(devdata::IODELAY_V5_IDELAY_DEFAULT, val);
        let val = extract_bitvec_val_part(
            ctx.bel_attr_bitvec(tcid, bslot, IODELAY::IDELAY_VALUE_CUR),
            &bits![0; 6],
            &mut diff_default,
        );
        ctx.insert_devdata_bitvec(devdata::IODELAY_V5_IDELAY_DEFAULT, val);
        ctx.insert_bel_attr_enum(
            tcid,
            bslot,
            IODELAY::IDELAY_TYPE,
            xlat_enum_attr(vec![
                (enums::IODELAY_V5_IDELAY_TYPE::VARIABLE, diff_variable),
                (enums::IODELAY_V5_IDELAY_TYPE::FIXED, Diff::default()),
                (enums::IODELAY_V5_IDELAY_TYPE::DEFAULT, diff_default),
            ]),
        );
    }

    let mut present_vr = ctx.get_diff_bel_special(tcid, bslots::IOB[0], specials::IOB_VR);
    for i in 0..2 {
        let bslot = bslots::IOB[i];
        ctx.collect_bel_attr_default(tcid, bslot, IOB::PULL, enums::IOB_PULL::NONE);
        ctx.collect_bel_attr_bi(tcid, bslot, IOB::OUTPUT_DELAY);
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
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
        let diff = ctx
            .peek_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_SLOW, IOB_DATA::LVCMOS25_12)
            .combine(&present);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE, xlat_bit_wide(diff));

        let oprog = xlat_bitvec(ctx.get_diffs_bel_special_bits(
            tcid,
            bslot,
            specials::IOB_OPROGRAMMING,
            31,
        ));
        let lvds = oprog[0..9].to_vec();
        let dci_t = oprog[9];
        let dci_mode = BelAttributeEnum {
            bits: oprog[10..13].iter().map(|bit| bit.bit).collect(),
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
        let output_misc = oprog[13..19].to_vec();
        let dci_misc = oprog[19..21].to_vec();
        let mut pdrive = oprog[21..26].to_vec();
        let mut ndrive = oprog[26..31].to_vec();
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
                    TileBit::new(0, 37, 17).pos(),
                    TileBit::new(0, 36, 23).pos(),
                    TileBit::new(0, 37, 23).pos(),
                    TileBit::new(0, 37, 30).pos(),
                    TileBit::new(0, 37, 29).pos(),
                    TileBit::new(0, 37, 27).pos(),
                ],
                vec![
                    TileBit::new(0, 36, 31).pos(),
                    TileBit::new(0, 36, 27).pos(),
                    TileBit::new(0, 37, 31).pos(),
                    TileBit::new(0, 37, 28).pos(),
                    TileBit::new(0, 36, 26).pos(),
                    TileBit::new(0, 37, 20).pos(),
                ],
            )
        } else {
            (
                vec![
                    TileBit::new(0, 37, 46).pos(),
                    TileBit::new(0, 36, 40).pos(),
                    TileBit::new(0, 37, 40).pos(),
                    TileBit::new(0, 37, 33).pos(),
                    TileBit::new(0, 37, 34).pos(),
                    TileBit::new(0, 37, 36).pos(),
                ],
                vec![
                    TileBit::new(0, 36, 32).pos(),
                    TileBit::new(0, 36, 36).pos(),
                    TileBit::new(0, 37, 32).pos(),
                    TileBit::new(0, 37, 35).pos(),
                    TileBit::new(0, 36, 37).pos(),
                    TileBit::new(0, 37, 43).pos(),
                ],
            )
        };

        let diff_cmos =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD, IOB_DATA::LVCMOS18_2);
        let diff_vref =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD, IOB_DATA::HSTL_I);
        let diff_diff =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LVDS, LVDS_DATA::LVDS_25);
        let (_, _, diff_diff) = Diff::split(diff_cmos.clone(), diff_diff.clone());
        let item = xlat_enum_attr(vec![
            (enums::IOB_IBUF_MODE::NONE, Diff::default()),
            (enums::IOB_IBUF_MODE::CMOS, diff_cmos.clone()),
            (enums::IOB_IBUF_MODE::VREF, diff_vref.clone()),
            (enums::IOB_IBUF_MODE::DIFF, diff_diff),
        ]);
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::IBUF_MODE, item);

        for &std in IOSTDS {
            if std.diff == DiffKind::True {
                continue;
            }
            let (drives, slews) = if !std.drive.is_empty() {
                (std.drive, &["SLOW", "FAST"][..])
            } else {
                (&[0][..], &[""][..])
            };
            for &drive in drives {
                for &slew in slews {
                    let (spec, row) = get_ostd_row(edev, &std, drive, slew);
                    let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                    diff.apply_bitvec_diff(
                        ctx.bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE),
                        &bits![1; 2],
                        &bits![0; 2],
                    );
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

        ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::OUTPUT_T, bits![0; 9]);
        ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::OUTPUT_C, bits![0; 9]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::PDRIVE, bits![0; 5]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::OUTPUT_MISC, bits![0; 6]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::NDRIVE, bits![0; 5]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::PSLEW_FAST, bits![0; 6]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::NSLEW_FAST, bits![0; 6]);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V5_LVDS, lvds);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::DCI_T, dci_t);
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::DCI_MODE, dci_mode);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V5_OUTPUT_MISC, output_misc);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::DCI_MISC, dci_misc);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V4_PDRIVE, pdrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V4_NDRIVE, ndrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V5_PSLEW, pslew);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V5_NSLEW, nslew);

        ctx.collect_bel_attr(tcid, bslot, IOB::I_INV);

        present.assert_empty();
    }
    let diff1 = present_vr.split_bits_by(|bit| bit.bit.to_idx() >= 32);
    ctx.insert_bel_attr_bool(tcid, bslots::IOB[0], IOB::VR, xlat_bit(present_vr));
    ctx.insert_bel_attr_bool(tcid, bslots::IOB[1], IOB::VR, xlat_bit(diff1));
    for i in 0..2 {
        let bslot = bslots::IOB[i];
        for &std in IOSTDS {
            let (spec, row) = get_istd_row(edev, &std);
            let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
            if std.diff != DiffKind::None {
                for bslot in [bslots::IOB[0], bslots::IOB[1]] {
                    match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                        DciKind::InputVcc | DciKind::BiVcc => {
                            diff.apply_enum_diff(
                                ctx.bel_attr_enum(tcid, bslot, IOB::DCI_MODE),
                                enums::IOB_DCI_MODE::TERM_VCC,
                                enums::IOB_DCI_MODE::NONE,
                            );
                            diff.apply_bitvec_diff(
                                ctx.bel_attr_bitvec(tcid, bslot, IOB::DCI_MISC),
                                &bits![1, 1],
                                &bits![0, 0],
                            );
                        }
                        DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                            diff.apply_enum_diff(
                                ctx.bel_attr_enum(tcid, bslot, IOB::DCI_MODE),
                                enums::IOB_DCI_MODE::TERM_SPLIT,
                                enums::IOB_DCI_MODE::NONE,
                            );
                        }
                    }
                    diff.apply_enum_diff(
                        ctx.bel_attr_enum(tcid, bslot, IOB::IBUF_MODE),
                        enums::IOB_IBUF_MODE::DIFF,
                        enums::IOB_IBUF_MODE::NONE,
                    );
                }
                diff.assert_empty();
            } else {
                match std.dci {
                    DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                    DciKind::InputVcc | DciKind::BiVcc => {
                        diff.apply_enum_diff(
                            ctx.bel_attr_enum(tcid, bslot, IOB::DCI_MODE),
                            enums::IOB_DCI_MODE::TERM_VCC,
                            enums::IOB_DCI_MODE::NONE,
                        );
                        diff.apply_bitvec_diff(
                            ctx.bel_attr_bitvec(tcid, bslot, IOB::DCI_MISC),
                            &bits![1, 1],
                            &bits![0, 0],
                        );
                    }
                    DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                        diff.apply_enum_diff(
                            ctx.bel_attr_enum(tcid, bslot, IOB::DCI_MODE),
                            enums::IOB_DCI_MODE::TERM_SPLIT,
                            enums::IOB_DCI_MODE::NONE,
                        );
                    }
                }
                let mode = if std.vref.is_some() {
                    enums::IOB_IBUF_MODE::VREF
                } else {
                    enums::IOB_IBUF_MODE::CMOS
                };
                diff.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, IOB::IBUF_MODE),
                    mode,
                    enums::IOB_IBUF_MODE::NONE,
                );
                diff.assert_empty();
            }
            if std.diff == DiffKind::True {
                let row = get_lvds_row(edev, &std);
                let mut diff =
                    ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LVDS_TERM, row);
                let val_c = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslots::IOB[0], IOB::V5_LVDS),
                    &bits![0; 9],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslots::IOB[1], IOB::V5_LVDS),
                    &bits![0; 9],
                    &mut diff,
                );
                ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::TERM_T, val_t);
                ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::TERM_C, val_c);
                diff.assert_empty();
                if i == 1 {
                    let mut diff =
                        ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_LVDS, row);
                    let val_c = extract_bitvec_val_part(
                        ctx.bel_attr_bitvec(tcid, bslots::IOB[0], IOB::V5_LVDS),
                        &bits![0; 9],
                        &mut diff,
                    );
                    let val_t = extract_bitvec_val_part(
                        ctx.bel_attr_bitvec(tcid, bslots::IOB[1], IOB::V5_LVDS),
                        &bits![0; 9],
                        &mut diff,
                    );
                    ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::OUTPUT_T, val_t);
                    ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::OUTPUT_C, val_c);
                    diff.apply_bitvec_diff(
                        ctx.bel_attr_bitvec(tcid, bslots::IOB[1], IOB::OUTPUT_ENABLE),
                        &bits![1; 2],
                        &bits![0; 2],
                    );
                    diff.assert_empty();
                }
            }
        }
    }

    let lvdsbias = vec![
        TileBit::new(0, 35, 15).pos(),
        TileBit::new(0, 34, 15).pos(),
        TileBit::new(0, 34, 14).pos(),
        TileBit::new(0, 35, 14).pos(),
        TileBit::new(0, 35, 13).pos(),
        TileBit::new(0, 34, 13).pos(),
        TileBit::new(0, 34, 12).pos(),
        TileBit::new(0, 35, 12).pos(),
        TileBit::new(0, 32, 13).pos(),
        TileBit::new(0, 33, 13).pos(),
        TileBit::new(0, 33, 12).pos(),
        TileBit::new(0, 32, 12).pos(),
    ];
    let lvdiv2 = vec![
        TileBit::new(0, 52, 12).pos(),
        TileBit::new(0, 53, 12).pos(),
        TileBit::new(0, 53, 15).pos(),
    ];
    let pref = vec![
        TileBit::new(0, 51, 12).pos(),
        TileBit::new(0, 50, 12).pos(),
        TileBit::new(0, 53, 14).pos(),
        TileBit::new(0, 52, 15).pos(),
    ];
    let nref = vec![TileBit::new(0, 52, 14).pos(), TileBit::new(0, 52, 13).pos()];
    let pmask_term_vcc = vec![
        TileBit::new(0, 50, 15).pos(),
        TileBit::new(0, 50, 14).pos(),
        TileBit::new(0, 51, 14).pos(),
        TileBit::new(0, 51, 13).pos(),
        TileBit::new(0, 50, 13).pos(),
    ];
    let pmask_term_split = vec![
        TileBit::new(0, 46, 13).pos(),
        TileBit::new(0, 46, 12).pos(),
        TileBit::new(0, 47, 12).pos(),
        TileBit::new(0, 48, 15).pos(),
        TileBit::new(0, 49, 15).pos(),
    ];
    let nmask_term_split = vec![
        TileBit::new(0, 48, 13).pos(),
        TileBit::new(0, 49, 13).pos(),
        TileBit::new(0, 49, 12).pos(),
        TileBit::new(0, 48, 12).pos(),
        TileBit::new(0, 51, 15).pos(),
    ];
    let mut diffs = vec![(enums::INTERNAL_VREF::OFF, Default::default())];
    for val in [
        enums::INTERNAL_VREF::_750,
        enums::INTERNAL_VREF::_900,
        enums::INTERNAL_VREF::_1080,
        enums::INTERNAL_VREF::_1250,
    ] {
        diffs.push((
            val,
            ctx.get_diff_attr_val(tcls::HCLK_IO, bslots::BANK, bcls::BANK::INTERNAL_VREF, val),
        ));
    }
    let vref = xlat_enum_attr(diffs);
    let dci_en =
        xlat_bit(ctx.get_diff_attr_bool(tcls::HCLK_IO_CMT_N, bslots::DCI, bcls::DCI::ENABLE));
    let has_bank3 = ctx.has_tcls(tcls::HCLK_IO_CMT_S);
    if has_bank3 {
        let dci_en_too =
            xlat_bit(ctx.get_diff_attr_bool(tcls::HCLK_IO_CMT_S, bslots::DCI, bcls::DCI::ENABLE));
        assert_eq!(dci_en, dci_en_too);
    }
    let dci_casc_above = if has_bank3 {
        Some(xlat_bit(ctx.get_diff_attr_bool(
            tcls::HCLK_IO_CFG_N,
            bslots::DCI,
            bcls::DCI::CASCADE_FROM_ABOVE,
        )))
    } else {
        None
    };
    let dci_casc_below = xlat_bit(ctx.get_diff_attr_bool(
        tcls::HCLK_IO_CFG_S,
        bslots::DCI,
        bcls::DCI::CASCADE_FROM_BELOW,
    ));
    for tcid in [
        tcls::HCLK_IO,
        tcls::HCLK_IO_CENTER,
        tcls::HCLK_IO_CFG_S,
        tcls::HCLK_IO_CFG_N,
        tcls::HCLK_IO_CMT_S,
        tcls::HCLK_IO_CMT_N,
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::BANK;
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::BANK::V5_LVDSBIAS, lvdsbias.clone());
        ctx.insert_bel_attr_enum(tcid, bslot, bcls::BANK::INTERNAL_VREF, vref.clone());
        let bslot = bslots::DCI;
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::ENABLE, dci_en);
        if let Some(dci_casc_above) = dci_casc_above {
            ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::CASCADE_FROM_ABOVE, dci_casc_above);
        }
        ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::CASCADE_FROM_BELOW, dci_casc_below);
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::V5_LVDIV2, lvdiv2.clone());
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::PREF, pref.clone());
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::NREF, nref.clone());
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::V4_PMASK_TERM_VCC,
            pmask_term_vcc.clone(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::V4_PMASK_TERM_SPLIT,
            pmask_term_split.clone(),
        );
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            bcls::DCI::V4_NMASK_TERM_SPLIT,
            nmask_term_split.clone(),
        );
        let te = xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::DCI::TEST_ENABLE));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::TEST_ENABLE, te);
        ctx.collect_bel_attr(tcid, bslot, bcls::DCI::QUIET);
    }
    let tcid = tcls::HCLK_IO;
    for std in IOSTDS {
        if std.diff == DiffKind::True {
            let row = get_lvds_row(edev, std);
            let bslot = bslots::BANK;
            let diff = ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_LVDS, row);
            let val = extract_bitvec_val(&lvdsbias, &bits![0; 12], diff);
            ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::LVDSBIAS, val);
        }
        if std.dci != DciKind::None {
            let (spec, row) = get_istd_row(edev, std);
            let bslot = bslots::DCI;
            let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
            match std.dci {
                DciKind::OutputHalf => {
                    let val = extract_bitvec_val_part(
                        ctx.bel_attr_bitvec(tcid, bslot, bcls::DCI::V5_LVDIV2),
                        &bits![0; 3],
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
    }
    ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::LVDSBIAS, bits![0; 12]);
    ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::LVDIV2, bits![0; 3]);
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
    {
        let tcid = tcls::CFG;
        let bslot = bslots::MISC_CFG;
        let bits =
            xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::MISC_CFG::DCI_CLK_ENABLE));
        ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::MISC_CFG::DCI_CLK_ENABLE, bits);
    }
}
