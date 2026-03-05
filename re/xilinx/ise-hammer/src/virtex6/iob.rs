use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, TableRowId},
    grid::{DieId, DieIdExt, TileCoord},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, OcdMode, SpecialId, extract_bitvec_val, extract_bitvec_val_part, xlat_bit,
    xlat_bit_wide, xlat_bitvec, xlat_enum_attr,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bitvec::BitVec, bsdata::TileBit};
use prjcombine_virtex4::defs::{
    bcls::{self, IOB},
    bslots, enums, tslots,
    virtex6::{
        tables::{IOB_DATA, LVDS_DATA},
        tcls,
    },
};

use crate::{
    backend::{IseBackend, Key, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        iostd::{DciKind, DiffKind, Iostd},
        props::DynProp,
    },
    virtex4::{io::IsBonded, specials},
    virtex5::io::{DiffOut, HclkIoi, VrefInternal},
};

const IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVCMOS25", 2500, &[2, 4, 6, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS12", 1200, &[2, 4, 6, 8]),
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

fn get_lvds_row(edev: &prjcombine_virtex4::expanded::ExpandedDevice, iostd: &Iostd) -> TableRowId {
    edev.db[LVDS_DATA].rows.get(iostd.name).unwrap().0
}

fn get_istd_row(
    edev: &prjcombine_virtex4::expanded::ExpandedDevice,
    iostd: &Iostd,
    is_lp: bool,
) -> (SpecialId, TableRowId) {
    if iostd.diff == DiffKind::True && iostd.dci == DciKind::None {
        (
            if is_lp {
                specials::IOB_ISTD_LVDS_LP
            } else {
                specials::IOB_ISTD_LVDS_HP
            },
            get_lvds_row(edev, iostd),
        )
    } else if let Some(name) = iostd.name.strip_prefix("DIFF_") {
        (
            if is_lp {
                specials::IOB_ISTD_DIFF_LP
            } else {
                specials::IOB_ISTD_DIFF_HP
            },
            edev.db[IOB_DATA].rows.get(name).unwrap().0,
        )
    } else if iostd.drive.is_empty() {
        (
            if is_lp {
                specials::IOB_ISTD_LP
            } else {
                specials::IOB_ISTD_HP
            },
            edev.db[IOB_DATA].rows.get(iostd.name).unwrap().0,
        )
    } else {
        (
            if is_lp {
                specials::IOB_ISTD_LP
            } else {
                specials::IOB_ISTD_HP
            },
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
    [bot + 10, bot + 30]
        .into_iter()
        .map(|vref_row| tcrd.with_row(vref_row).tile(tslots::BEL))
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
        let hclk_ioi = tcrd.with_row(hclk_row).tile(tslots::HCLK_BEL);
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

fn get_vr(backend: &IseBackend, tcrd: TileCoord) -> TileCoord {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    let chip = edev.chips[tcrd.die];

    let reg = chip.row_to_reg(tcrd.row);
    let row = if reg == chip.reg_cfg {
        chip.row_reg_bot(reg) + 6
    } else if reg == chip.reg_cfg - 1 && Some(tcrd.col) == edev.col_io_iw {
        chip.row_reg_bot(reg) + 4
    } else if reg == chip.reg_cfg - 1 && Some(tcrd.col) == edev.col_io_ie {
        chip.row_reg_bot(reg) + 0
    } else {
        chip.row_reg_bot(reg) + 14
    };
    tcrd.with_row(row).tile(tslots::BEL)
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

        // Avoid bank 25, which is our (arbitrary) anchor.
        if tcrd.col == edev.col_io_iw.unwrap() && chip.row_to_reg(tcrd.row) == chip.reg_cfg {
            return None;
        }

        let vr_tile = get_vr(backend, tcrd);
        if tcrd == vr_tile {
            // Not in VR tile please.
            return None;
        }
        // Ensure nothing is placed in VR.
        for bel in [bslots::IOB[0], bslots::IOB[1]] {
            let site = backend.ngrid.get_bel_name(vr_tile.cell.bel(bel)).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Test VR.
        if self.0.is_some() {
            fuzzer.info.features.push(FuzzerFeature {
                key: DiffKey::BelSpecial(tcls::IO, bslots::IOB[0], specials::IOB_VR),
                rects: edev.tile_bits(vr_tile),
            });
        }

        // Take exclusive mutex on bank DCI.
        let hclk_ioi = tcrd
            .cell
            .with_row(chip.row_hclk(tcrd.row))
            .tile(tslots::HCLK_BEL);
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

        // Anchor global DCI by putting something in bottom IOB of bank 25.
        let iob_center = tcrd
            .cell
            .with_cr(edev.col_io_iw.unwrap(), chip.row_bufg())
            .bel(bslots::IOB[0]);
        let site = backend.ngrid.get_bel_name(iob_center).unwrap();
        fuzzer = fuzzer.base(Key::SiteMode(site), "IOB");
        fuzzer = fuzzer.base(Key::SitePin(site, "O".into()), true);
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OUSED".into()), "0");
        fuzzer = fuzzer.base(Key::SiteAttr(site, "OSTANDARD".into()), "LVDCI_25");
        // Ensure anchor VR IOBs are free.
        for bel in [bslots::IOB[0], bslots::IOB[1]] {
            let iob_center_vr = tcrd
                .cell
                .with_cr(edev.col_io_iw.unwrap(), chip.row_bufg() + 6)
                .bel(bel);
            let site = backend.ngrid.get_bel_name(iob_center_vr).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Make note of anchor VCCO.
        let hclk_ioi_center = tcrd
            .cell
            .with_cr(edev.col_io_iw.unwrap(), chip.row_bufg() + 20)
            .tile(tslots::HCLK_BEL);
        fuzzer = fuzzer.base(Key::TileMutex(hclk_ioi_center, "VCCO".to_string()), "2500");

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
        let bel = bslots::IOB[i];
        let mut bctx = ctx.bel(bel);
        let bel_ologic = bslots::OLOGIC[i];
        let bel_other_ologic = bslots::OLOGIC[i ^ 1];
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
        for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
            bctx.mode("IOB")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .mutex("PULL_DYNAMIC", pin)
                .test_bel_attr_bits(IOB::PULL_DYNAMIC)
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
            .test_bel_special_bits(specials::IOB_OPROGRAMMING)
            .multi_attr("OPROGRAMMING", MultiValue::Bin, 31);
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
                let (spec_dci, row_dci) = get_istd_row(edev, &std, true);
                dci_special = Some(Dci(Some((spec_dci, row_dci))));
                dci_special_lite = Some(Dci(None));
            }
            if std.diff != DiffKind::None {
                for (is_lp, lp) in [(true, "TRUE"), (false, "FALSE")] {
                    let (spec, row) = get_istd_row(edev, &std, is_lp);
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
                        .test_bel_special_row(spec, row)
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
                    let row = get_lvds_row(edev, &std);
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
                        .test_bel_special_row(specials::IOB_ISTD_LVDS_TERM, row)
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
                        .test_bel_special_row(specials::IOB_ISTD_LVDS_DYN_TERM, row)
                        .attr_diff("DIFF_TERM", "FALSE", "TRUE")
                        .bel_attr_diff(bel_other, "DIFF_TERM", "FALSE", "TRUE")
                        .pin_pips("DIFF_TERM_INT_EN")
                        .commit();
                }
            } else {
                for (is_lp, lp) in [(true, "TRUE"), (false, "FALSE")] {
                    let (spec, row) = get_istd_row(edev, &std, is_lp);
                    bctx.mode("IOB")
                        .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                        .attr("OUSED", "")
                        .pin("I")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .maybe_prop(vref_special)
                        .maybe_prop(dci_special)
                        .test_bel_special_row(spec, row)
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
                let (spec_dci, row_dci) = get_istd_row(edev, &std, true);
                dci_special = Some(Dci(Some((spec_dci, row_dci))));
            }
            if std.diff == DiffKind::True {
                let row = get_lvds_row(edev, &std);
                if i == 1 {
                    bctx.build()
                        .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                        .attr("IUSED", "")
                        .attr("OPROGRAMMING", "")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .prop(DiffOut(specials::IOB_OSTD_LVDS, row))
                        .bel_attr(bel_other, "IUSED", "")
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
            } else if std.diff != DiffKind::None {
                let (spec, row) = get_ostd_row(edev, &std, 0, "");
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
                        .test_bel_special_row(spec, row)
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
                        let (spec, row) = get_ostd_row(edev, &std, drive, slew);
                        bctx.mode("IOB")
                            .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                            .pin("O")
                            .attr("IUSED", "")
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
                    .related_tile_mutex(HclkIoi, "VCCO", std.vcco.unwrap().to_string())
                    .pin("O")
                    .attr("IUSED", "")
                    .attr("OPROGRAMMING", "")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .maybe_prop(dci_special)
                    .test_bel_special_row(spec, row)
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", std.name)
                    .commit();
            }
        }

        for (row, std, vcco, vref) in [
            (
                IOB_DATA::HSTL_I_12,
                "HSTL_I_12",
                1200,
                enums::INTERNAL_VREF::_600,
            ),
            (IOB_DATA::HSTL_I, "HSTL_I", 1500, enums::INTERNAL_VREF::_750),
            (
                IOB_DATA::HSTL_III,
                "HSTL_III",
                1500,
                enums::INTERNAL_VREF::_900,
            ),
            (
                IOB_DATA::HSTL_III_18,
                "HSTL_III_18",
                1800,
                enums::INTERNAL_VREF::_1100,
            ),
            (
                IOB_DATA::SSTL2_I,
                "SSTL2_I",
                2500,
                enums::INTERNAL_VREF::_1250,
            ),
        ] {
            bctx.mode("IOB")
                .related_tile_mutex(HclkIoi, "VCCO", vcco.to_string())
                .attr("OUSED", "")
                .pin("I")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .prop(VrefInternal(tcls::HCLK_IO, vref))
                .test_bel_special_row(specials::IOB_ISTD_LP, row)
                .attr("IUSED", "0")
                .attr("ISTANDARD", std)
                .attr("IBUF_LOW_PWR", "TRUE")
                .commit();
        }

        bctx.build()
            .mutex("OUTPUT_DELAY", "0")
            .bel_mode(bel_ologic, "OLOGICE1")
            .test_bel_attr_bits_bi(IOB::OUTPUT_DELAY, false)
            .pip((bel_ologic, "IOB_O"), (bel_ologic, "OQ"))
            .commit();
        bctx.build()
            .mutex("OUTPUT_DELAY", "1")
            .bel_mode(bel_ologic, "OLOGICE1")
            .test_bel_attr_bits_bi(IOB::OUTPUT_DELAY, true)
            .pip((bel_ologic, "IOB_O"), (bel_iodelay, "DATAOUT"))
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, tcls::HCLK_IO);
        let mut bctx = ctx.bel(bslots::DCI);
        bctx.build()
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_bel_attr_bits(bcls::DCI::TEST_ENABLE)
            .mode("DCI")
            .commit();
        bctx.build()
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_bel_attr_bits(bcls::DCI::DYNAMIC_ENABLE)
            .mode("DCI")
            .pin_pips("INT_DCI_EN")
            .commit();
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .extra_tiles_by_bel_attr_bits(bslots::DCI, bcls::DCI::QUIET)
        .test_global_special(specials::DCI_QUIET)
        .global_diff("DCIUPDATEMODE", "CONTINUOUS", "QUIET")
        .commit();
    {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        let mut builder = ctx
            .build()
            .raw(Key::Package, &package.name)
            .extra_fixed_bel_attr_bits(
                edev.tile_cfg(die),
                bslots::MISC_CFG,
                bcls::MISC_CFG::DCI_CLK_ENABLE_TR,
            );

        // Find VR and IO rows.
        let vr_tile = die
            .cell(edev.col_io_iw.unwrap(), chip.row_bufg() + 6)
            .tile(tslots::BEL);
        let io_tile = die
            .cell(edev.col_io_iw.unwrap(), chip.row_bufg())
            .tile(tslots::BEL);
        let io_bel = io_tile.cell.bel(bslots::IOB[0]);
        let hclk_row = chip.row_hclk(io_tile.cell.row);
        let hclk_tcrd = die
            .cell(edev.col_io_iw.unwrap(), hclk_row)
            .tile(tslots::HCLK_BEL);

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
            .extra_fixed_bel_special_row(
                io_tile,
                bslots::IOB[0],
                specials::IOB_OSTD,
                IOB_DATA::LVDCI_25,
            )
            .test_global_special(specials::CENTER_DCI_BANK1)
            .commit();
    }
    for bank_to in [24, 26] {
        let die = DieId::from_idx(0);
        let chip = edev.chips[die];
        let mut builder = ctx.build().raw(Key::Package, &package.name);

        let io_tile_from = die
            .cell(edev.col_io_iw.unwrap(), chip.row_bufg())
            .tile(tslots::BEL);
        let io_bel_from = io_tile_from.cell.bel(bslots::IOB[0]);
        let io_row_to = match bank_to {
            24 => edev.chips[die].row_bufg() - 40,
            26 => edev.chips[die].row_bufg() + 40,
            _ => unreachable!(),
        };
        let io_tile_to = die
            .cell(edev.col_io_iw.unwrap(), io_row_to)
            .tile(tslots::BEL);
        let io_bel_to = io_tile_to.cell.bel(bslots::IOB[0]);
        let hclk_row_to = chip.row_hclk(io_row_to);
        let hclk_tile_to = die
            .cell(edev.col_io_iw.unwrap(), hclk_row_to)
            .tile(tslots::HCLK_BEL);

        // Ensure nothing else in the bank.
        let bot = chip.row_reg_bot(chip.row_to_reg(io_tile_from.cell.row));
        for i in 0..chip.rows_per_reg() {
            let row = bot + i;
            for bel in [bslots::IOB[0], bslots::IOB[1]] {
                if row == io_tile_from.cell.row && bel == bslots::IOB[0] {
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
            for bel in [bslots::IOB[0], bslots::IOB[1]] {
                if row == io_tile_to.cell.row && bel == bslots::IOB[0] {
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
            .extra_fixed_bel_special_row(
                io_tile_to,
                bslots::IOB[0],
                specials::IOB_OSTD,
                IOB_DATA::LVDCI_25,
            )
            .extra_fixed_bel_attr_bits(
                hclk_tile_to,
                bslots::DCI,
                if bank_to == 24 {
                    bcls::DCI::CASCADE_FROM_ABOVE
                } else {
                    bcls::DCI::CASCADE_FROM_BELOW
                },
            )
            .test_global_special(specials::CASCADE_DCI)
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    let tcid = tcls::IO;

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

        let oprog = xlat_bitvec(ctx.get_diffs_bel_special_bits(
            tcid,
            bslot,
            specials::IOB_OPROGRAMMING,
            31,
        ));
        let lvds = oprog[0..9].to_vec();
        let dci_t = oprog[11];
        let dci_mode = BelAttributeEnum {
            bits: oprog[12..15].iter().map(|bit| bit.bit).collect(),
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
        let output_misc = oprog[15..19].to_vec();
        let dci_misc = oprog[9..11].to_vec();
        let mut pdrive = oprog[19..25].to_vec();
        let mut ndrive = oprog[25..31].to_vec();
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
                    TileBit::new(0, 41, 39).pos(),
                    TileBit::new(0, 41, 31).pos(),
                    TileBit::new(0, 41, 27).pos(),
                    TileBit::new(0, 40, 20).pos(),
                    TileBit::new(0, 40, 10).pos(),
                ],
                vec![
                    TileBit::new(0, 40, 44).pos(),
                    TileBit::new(0, 40, 30).pos(),
                    TileBit::new(0, 40, 32).pos(),
                    TileBit::new(0, 41, 17).pos(),
                    TileBit::new(0, 41, 43).pos(),
                ],
            )
        } else {
            (
                vec![
                    TileBit::new(1, 40, 24).pos(),
                    TileBit::new(1, 40, 32).pos(),
                    TileBit::new(1, 40, 36).pos(),
                    TileBit::new(1, 41, 43).pos(),
                    TileBit::new(1, 41, 53).pos(),
                ],
                vec![
                    TileBit::new(1, 41, 19).pos(),
                    TileBit::new(1, 41, 33).pos(),
                    TileBit::new(1, 41, 31).pos(),
                    TileBit::new(1, 40, 46).pos(),
                    TileBit::new(1, 40, 20).pos(),
                ],
            )
        };

        let mut diff = ctx
            .peek_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_SLOW, IOB_DATA::LVCMOS25_12)
            .combine(&present);
        diff.discard_polbits(&pdrive);
        diff.discard_polbits(&ndrive);
        extract_bitvec_val_part(&pslew, &bits![0; 5], &mut diff);
        extract_bitvec_val_part(&nslew, &bits![0; 5], &mut diff);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE, xlat_bit_wide(diff));

        let diff_cmos =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LP, IOB_DATA::LVCMOS18_2);
        let diff_cmos12 =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LP, IOB_DATA::LVCMOS12_2);
        let diff_vref_lp =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LP, IOB_DATA::HSTL_I);
        let diff_vref_hp =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_HP, IOB_DATA::HSTL_I);
        let mut diff_diff_lp = ctx
            .peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LVDS_LP, LVDS_DATA::LVDS_25)
            .clone();
        let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.rect.to_idx() == i);
        let mut diff_diff_hp = ctx
            .peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LVDS_HP, LVDS_DATA::LVDS_25)
            .clone();
        let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.rect.to_idx() == i);
        let vref_hp = xlat_bit(diff_vref_hp.combine(&!diff_vref_lp));
        let diff_hp = xlat_bit(diff_diff_hp.combine(&!&diff_diff_lp));
        let item = xlat_enum_attr(vec![
            (enums::IOB_IBUF_MODE::NONE, Diff::default()),
            (enums::IOB_IBUF_MODE::CMOS, diff_cmos.clone()),
            (enums::IOB_IBUF_MODE::CMOS12, diff_cmos12.clone()),
            (enums::IOB_IBUF_MODE::VREF, diff_vref_lp.clone()),
            (enums::IOB_IBUF_MODE::DIFF, diff_diff_lp),
        ]);
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::IBUF_MODE, item);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::IBUF_DIFF_HP, diff_hp);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::IBUF_VREF_HP, vref_hp);

        for &std in IOSTDS {
            if std.diff != DiffKind::None {
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
                ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, field, value);
            }
            present_vref.assert_empty();
        }

        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::OUTPUT_MISC, bits![0; 4]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::PDRIVE, bits![0; 6]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::NDRIVE, bits![0; 6]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::PSLEW_FAST, bits![0; 5]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::NSLEW_FAST, bits![0; 5]);
        ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::OUTPUT_T, bits![0; 9]);
        ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::OUTPUT_C, bits![0; 9]);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V5_LVDS, lvds);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::DCI_T, dci_t);
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::DCI_MODE, dci_mode);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V6_OUTPUT_MISC, output_misc);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::DCI_MISC, dci_misc);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V6_PDRIVE, pdrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V6_NDRIVE, ndrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V6_PSLEW, pslew);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V6_NSLEW, nslew);

        present.assert_empty();
    }

    let diff1 = present_vr.split_bits_by(|bit| bit.rect.to_idx() == 1);
    ctx.insert_bel_attr_bool(tcid, bslots::IOB[0], IOB::VR, xlat_bit(present_vr));
    ctx.insert_bel_attr_bool(tcid, bslots::IOB[1], IOB::VR, xlat_bit(diff1));
    // ISE bug.
    let mut diff = ctx.get_diff_attr_bool(tcid, bslots::IOB[0], IOB::PULL_DYNAMIC);
    let diff1 = diff.split_bits_by(|bit| bit.rect.to_idx() == 1);
    ctx.insert_bel_attr_bool(tcid, bslots::IOB[0], IOB::PULL_DYNAMIC, xlat_bit(diff));
    ctx.insert_bel_attr_bool(tcid, bslots::IOB[1], IOB::PULL_DYNAMIC, xlat_bit(diff1));
    ctx.get_diff_attr_bool(tcid, bslots::IOB[1], IOB::PULL_DYNAMIC)
        .assert_empty();

    for i in 0..2 {
        let bslot = bslots::IOB[i];
        for &std in IOSTDS {
            for is_lp in [false, true] {
                let (spec, row) = get_istd_row(edev, &std, is_lp);
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
                        if !is_lp {
                            diff.apply_bit_diff(
                                ctx.bel_attr_bit(tcid, bslot, IOB::IBUF_DIFF_HP),
                                true,
                                false,
                            );
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
                    if std.vref.is_some() && !is_lp {
                        diff.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslot, IOB::IBUF_VREF_HP),
                            true,
                            false,
                        );
                    }
                    let mode = if std.vref.is_some() {
                        enums::IOB_IBUF_MODE::VREF
                    } else if std.vcco == Some(1200) {
                        enums::IOB_IBUF_MODE::CMOS12
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
            }
            if std.diff == DiffKind::True && i == 0 {
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
                let mut diff = ctx.get_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_LVDS_DYN_TERM,
                    row,
                );
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
                ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::DYN_TERM_T, val_t);
                ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::DYN_TERM_C, val_c);
                diff.assert_empty();
            }
            if std.diff == DiffKind::True && i == 1 {
                let row = get_lvds_row(edev, &std);
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
            if std.diff == DiffKind::Pseudo && i == 1 {
                let (spec, row) = get_ostd_row(edev, &std, 0, "");
                let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                for bslot in [bslots::IOB[0], bslots::IOB[1]] {
                    diff.apply_bitvec_diff(
                        ctx.bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE),
                        &bits![1; 2],
                        &bits![0; 2],
                    );
                    for (field, attr) in [
                        (IOB_DATA::PDRIVE, IOB::V6_PDRIVE),
                        (IOB_DATA::NDRIVE, IOB::V6_NDRIVE),
                        (IOB_DATA::PSLEW_FAST, IOB::V6_PSLEW),
                        (IOB_DATA::NSLEW_FAST, IOB::V6_NSLEW),
                        (IOB_DATA::OUTPUT_MISC, IOB::V6_OUTPUT_MISC),
                    ] {
                        let item = ctx.bel_attr_bitvec(tcid, bslot, attr);
                        let value = extract_bitvec_val_part(
                            item,
                            &BitVec::repeat(false, item.len()),
                            &mut diff,
                        );
                        ctx.insert_table_bitvec(IOB_DATA, row, field, value);
                    }
                    let dci_mode = ctx.bel_attr_enum(tcid, bslot, IOB::DCI_MODE);
                    let dci_misc = ctx.bel_attr_bitvec(tcid, bslot, IOB::DCI_MISC);
                    let dci_t = ctx.bel_attr_bit(tcid, bslot, IOB::DCI_T);
                    match std.dci {
                        DciKind::None | DciKind::InputVcc | DciKind::InputSplit => {}
                        DciKind::Output => {
                            diff.apply_enum_diff(
                                dci_mode,
                                enums::IOB_DCI_MODE::OUTPUT,
                                enums::IOB_DCI_MODE::NONE,
                            );
                        }
                        DciKind::OutputHalf => {
                            diff.apply_enum_diff(
                                dci_mode,
                                enums::IOB_DCI_MODE::OUTPUT_HALF,
                                enums::IOB_DCI_MODE::NONE,
                            );
                        }
                        DciKind::BiVcc => {
                            diff.apply_enum_diff(
                                dci_mode,
                                enums::IOB_DCI_MODE::TERM_VCC,
                                enums::IOB_DCI_MODE::NONE,
                            );
                            diff.apply_bitvec_diff(dci_misc, &bits![1, 1], &bits![0, 0]);
                        }
                        DciKind::BiSplit => {
                            diff.apply_enum_diff(
                                dci_mode,
                                enums::IOB_DCI_MODE::TERM_SPLIT,
                                enums::IOB_DCI_MODE::NONE,
                            );
                        }
                        DciKind::BiSplitT => {
                            diff.apply_enum_diff(
                                dci_mode,
                                enums::IOB_DCI_MODE::TERM_SPLIT,
                                enums::IOB_DCI_MODE::NONE,
                            );
                            diff.apply_bit_diff(dci_t, true, false);
                        }
                    }
                }
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslots::IOB[0],
                    IOB::OUTPUT_PSEUDO_DIFF,
                    xlat_bit(diff),
                );
            }
        }
    }

    let tcid = tcls::HCLK_IO;
    let lvdsbias = vec![
        TileBit::new(0, 42, 30).pos(),
        TileBit::new(0, 42, 28).pos(),
        TileBit::new(0, 42, 27).pos(),
        TileBit::new(0, 42, 26).pos(),
        TileBit::new(0, 42, 25).pos(),
        TileBit::new(0, 42, 24).pos(),
        TileBit::new(0, 42, 23).pos(),
        TileBit::new(0, 42, 22).pos(),
        TileBit::new(0, 42, 21).pos(),
        TileBit::new(0, 42, 20).pos(),
        TileBit::new(0, 42, 19).pos(),
        TileBit::new(0, 42, 18).pos(),
        TileBit::new(0, 42, 17).pos(),
        TileBit::new(0, 42, 16).pos(),
        TileBit::new(0, 42, 15).pos(),
        TileBit::new(0, 42, 14).pos(),
        TileBit::new(0, 41, 28).pos(),
    ];
    let bslot = bslots::DCI;
    let dci_en = ctx.get_diff_attr_bool(tcid, bslot, bcls::DCI::ENABLE);
    let test_en = ctx
        .get_diff_attr_bool(tcid, bslot, bcls::DCI::TEST_ENABLE)
        .combine(&!&dci_en);
    let dyn_en = ctx
        .get_diff_attr_bool(tcid, bslot, bcls::DCI::DYNAMIC_ENABLE)
        .combine(&!&dci_en);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::TEST_ENABLE, xlat_bit_wide(test_en));
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::DYNAMIC_ENABLE, xlat_bit(dyn_en));
    let casc_from_above = ctx
        .get_diff_attr_bool(tcid, bslot, bcls::DCI::CASCADE_FROM_ABOVE)
        .combine(&!&dci_en);
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        bcls::DCI::CASCADE_FROM_ABOVE,
        xlat_bit(casc_from_above),
    );
    let casc_from_below = ctx
        .get_diff_attr_bool(tcid, bslot, bcls::DCI::CASCADE_FROM_BELOW)
        .combine(&!&dci_en);
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        bcls::DCI::CASCADE_FROM_BELOW,
        xlat_bit(casc_from_below),
    );

    let dci_en = xlat_bit(dci_en);
    let nref_output = vec![TileBit::new(0, 40, 16).pos(), TileBit::new(0, 40, 17).pos()];
    let pref_output = vec![TileBit::new(0, 41, 14).pos(), TileBit::new(0, 41, 15).pos()];
    let nref_output_half = vec![
        TileBit::new(0, 40, 18).pos(),
        TileBit::new(0, 40, 19).pos(),
        TileBit::new(0, 40, 20).pos(),
    ];
    let pref_output_half = vec![
        TileBit::new(0, 41, 16).pos(),
        TileBit::new(0, 41, 17).pos(),
        TileBit::new(0, 41, 18).pos(),
    ];
    let pref_term_vcc = vec![TileBit::new(0, 40, 14).pos(), TileBit::new(0, 40, 15).pos()];
    let pmask_term_vcc = vec![
        TileBit::new(0, 43, 14).pos(),
        TileBit::new(0, 43, 27).pos(),
        TileBit::new(0, 43, 28).pos(),
        TileBit::new(0, 43, 29).pos(),
        TileBit::new(0, 43, 30).pos(),
        TileBit::new(0, 43, 31).pos(),
    ];
    let nref_term_split = vec![
        TileBit::new(0, 40, 23).pos(),
        TileBit::new(0, 40, 24).pos(),
        TileBit::new(0, 40, 25).pos(),
    ];
    let pref_term_split = vec![
        TileBit::new(0, 41, 19).pos(),
        TileBit::new(0, 41, 20).pos(),
        TileBit::new(0, 41, 21).pos(),
    ];
    let pmask_term_split = vec![
        TileBit::new(0, 43, 21).pos(),
        TileBit::new(0, 43, 22).pos(),
        TileBit::new(0, 43, 23).pos(),
        TileBit::new(0, 43, 24).pos(),
        TileBit::new(0, 43, 25).pos(),
        TileBit::new(0, 43, 26).pos(),
    ];
    let nmask_term_split = vec![
        TileBit::new(0, 43, 15).pos(),
        TileBit::new(0, 43, 16).pos(),
        TileBit::new(0, 43, 17).pos(),
        TileBit::new(0, 43, 18).pos(),
        TileBit::new(0, 43, 19).pos(),
        TileBit::new(0, 43, 20).pos(),
    ];
    ctx.collect_bel_attr_subset_default_ocd(
        tcid,
        bslots::BANK,
        bcls::BANK::INTERNAL_VREF,
        &[
            enums::INTERNAL_VREF::_600,
            enums::INTERNAL_VREF::_750,
            enums::INTERNAL_VREF::_900,
            enums::INTERNAL_VREF::_1100,
            enums::INTERNAL_VREF::_1250,
        ],
        enums::INTERNAL_VREF::OFF,
        OcdMode::ValueOrder,
    );
    for std in IOSTDS {
        if std.diff == DiffKind::True {
            let row = get_lvds_row(edev, std);
            let bslot = bslots::BANK;
            let diff = ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_LVDS, row);
            let val = extract_bitvec_val(&lvdsbias, &bits![0; 17], diff);
            ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::LVDSBIAS, val);
        }
        if std.dci != DciKind::None {
            let (spec, row) = get_istd_row(edev, std, true);
            let bslot = bslots::DCI;
            let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
            match std.dci {
                DciKind::Output => {
                    let val = extract_bitvec_val_part(&nref_output, &bits![0; 2], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::NREF_OUTPUT, val);
                    let val = extract_bitvec_val_part(&pref_output, &bits![0; 2], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::PREF_OUTPUT, val);
                }
                DciKind::OutputHalf => {
                    let val = extract_bitvec_val_part(&nref_output_half, &bits![0; 3], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::NREF_OUTPUT_HALF, val);
                    let val = extract_bitvec_val_part(&pref_output_half, &bits![0; 3], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::PREF_OUTPUT_HALF, val);
                }
                DciKind::InputVcc | DciKind::BiVcc => {
                    let val = extract_bitvec_val_part(&pref_term_vcc, &bits![0; 2], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::PREF_TERM_VCC, val);
                    let val = extract_bitvec_val_part(&pmask_term_vcc, &bits![0; 6], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::PMASK_TERM_VCC, val);
                }
                DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                    let val = extract_bitvec_val_part(&nref_term_split, &bits![0; 3], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::NREF_TERM_SPLIT, val);
                    let val = extract_bitvec_val_part(&pref_term_split, &bits![0; 3], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::PREF_TERM_SPLIT, val);
                    let val = extract_bitvec_val_part(&pmask_term_split, &bits![0; 6], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::PMASK_TERM_SPLIT, val);
                    let val = extract_bitvec_val_part(&nmask_term_split, &bits![0; 6], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::NMASK_TERM_SPLIT, val);
                }
                _ => {}
            }
            diff.apply_bit_diff(dci_en, true, false);
            diff.assert_empty();
        }
    }
    ctx.insert_bel_attr_bitvec(tcid, bslots::BANK, bcls::BANK::V6_LVDSBIAS, lvdsbias);
    ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::LVDSBIAS, bits![0; 17]);
    let bslot = bslots::DCI;
    ctx.insert_bel_attr_bool(tcid, bslot, bcls::DCI::ENABLE, dci_en);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::PREF_OUTPUT, pref_output);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::NREF_OUTPUT, nref_output);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::PREF_OUTPUT_HALF, pref_output_half);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::NREF_OUTPUT_HALF, nref_output_half);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::PREF_TERM_VCC, pref_term_vcc);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::PREF_TERM_SPLIT, pref_term_split);
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::NREF_TERM_SPLIT, nref_term_split);

    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::DCI::V6_PMASK_TERM_VCC, pmask_term_vcc);
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslot,
        bcls::DCI::V6_PMASK_TERM_SPLIT,
        pmask_term_split,
    );
    ctx.insert_bel_attr_bitvec(
        tcid,
        bslot,
        bcls::DCI::V6_NMASK_TERM_SPLIT,
        nmask_term_split,
    );
    ctx.collect_bel_attr(tcid, bslot, bcls::DCI::QUIET);

    ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::PREF_OUTPUT, bits![0; 2]);
    ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::NREF_OUTPUT, bits![0; 2]);
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::PREF_OUTPUT_HALF,
        bits![0; 3],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::NREF_OUTPUT_HALF,
        bits![0; 3],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::PREF_TERM_VCC,
        bits![0; 2],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::PMASK_TERM_VCC,
        bits![0; 6],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::PREF_TERM_SPLIT,
        bits![0; 3],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::NREF_TERM_SPLIT,
        bits![0; 3],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::PMASK_TERM_SPLIT,
        bits![0; 6],
    );
    ctx.insert_table_bitvec(
        IOB_DATA,
        IOB_DATA::OFF,
        IOB_DATA::NMASK_TERM_SPLIT,
        bits![0; 6],
    );

    let tcid = tcls::CFG;
    let bslot = bslots::MISC_CFG;
    let bits =
        xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::MISC_CFG::DCI_CLK_ENABLE_TR));
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::MISC_CFG::DCI_CLK_ENABLE_TR, bits);
}
