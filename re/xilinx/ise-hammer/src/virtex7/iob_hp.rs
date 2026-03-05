use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::{BelAttributeEnum, TableRowId},
    grid::{CellCoord, DieId, DieIdExt, TileCoord},
};
use prjcombine_re_collector::diff::{
    Diff, DiffKey, SpecialId, extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bit_wide,
    xlat_bitvec, xlat_enum_attr,
};
use prjcombine_re_fpga_hammer::{FuzzerFeature, FuzzerProp};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bitvec::BitVec, bsdata::TileBit};
use prjcombine_virtex4::{
    chip::RegId,
    defs::{
        bcls::{self, BANK, DCI, IOB},
        bslots, enums, tslots,
        virtex7::{
            tables::{IOB_DATA, LVDS_DATA},
            tcls,
        },
    },
    expanded::IoCoord,
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

const HP_IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVCMOS18", 1800, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS15", 1500, &[2, 4, 6, 8, 12, 16]),
    Iostd::cmos("LVCMOS12", 1200, &[2, 4, 6, 8]),
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
            if slew == "SLOW" {
                specials::IOB_OSTD_DIFF_SLOW
            } else {
                specials::IOB_OSTD_DIFF_FAST
            },
            edev.db[IOB_DATA].rows.get(name).unwrap().0,
        )
    } else {
        (
            if slew == "SLOW" {
                specials::IOB_OSTD_SLOW
            } else {
                specials::IOB_OSTD_FAST
            },
            if iostd.drive.is_empty() {
                edev.db[IOB_DATA].rows.get(iostd.name).unwrap().0
            } else {
                edev.db[IOB_DATA]
                    .rows
                    .get(&format!("{std}_{drive}", std = iostd.name))
                    .unwrap()
                    .0
            },
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
    [bot + 11, bot + 37]
        .into_iter()
        .map(|vref_row| tcrd.with_row(vref_row).tile(tslots::BEL))
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub struct Vref(pub bool);

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
            if self.0 {
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::BelSpecial(edev[vref].class, bslots::IOB[0], specials::IOB_VREF),
                    rects: backend.edev.tile_bits(vref),
                });
            }
        }
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

        // Avoid anchor bank.
        let anchor_reg = if chip.has_ps {
            RegId::from_idx(chip.regs - 1)
        } else {
            RegId::from_idx(0)
        };
        if tcrd.col == edev.col_io_e.unwrap() && chip.row_to_reg(tcrd.row) == anchor_reg {
            return None;
        }

        // Ensure nothing is placed in VR.
        for row in [chip.row_hclk(tcrd.row) - 25, chip.row_hclk(tcrd.row) + 24] {
            let vr_tile = tcrd.with_row(row).tile(tslots::BEL);
            let vr_bel = vr_tile.cell.bel(bslots::IOB[0]);
            let site = backend.ngrid.get_bel_name(vr_bel).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
            // Test VR.
            if self.0.is_some() {
                fuzzer.info.features.push(FuzzerFeature {
                    key: DiffKey::BelSpecial(edev[vr_tile].class, bslots::IOB[0], specials::IOB_VR),
                    rects: edev.tile_bits(vr_tile),
                });
            }
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
                key: DiffKey::BelSpecialRow(tcls::HCLK_IO_HP, bslots::DCI, spec, row),
                rects: edev.tile_bits(hclk_ioi),
            });
        }

        // Take shared mutex on global DCI.
        fuzzer = fuzzer.base(Key::GlobalMutex("GLOBAL_DCI".into()), "SHARED");

        // Anchor global DCI by putting something in arbitrary bank.
        let iob_anchor = tcrd
            .cell
            .with_cr(edev.col_io_e.unwrap(), chip.row_reg_bot(anchor_reg) + 1)
            .bel(bslots::IOB[0]);
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
                .with_cr(edev.col_io_e.unwrap(), row)
                .bel(bslots::IOB[0]);
            let site = backend.ngrid.get_bel_name(iob_anchor_vr).unwrap();
            fuzzer = fuzzer.base(Key::SiteMode(site), None);
        }
        // Make note of anchor VCCO.
        let hclk_ioi_anchor = tcrd
            .cell
            .with_cr(edev.col_io_e.unwrap(), chip.row_reg_hclk(anchor_reg))
            .tile(tslots::HCLK_BEL);
        fuzzer = fuzzer.base(Key::TileMutex(hclk_ioi_anchor, "VCCO".to_string()), "1800");

        Some((fuzzer, false))
    }
}

fn add_fuzzers_iob_hp<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
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

    for (tcid, num_io) in [
        (tcls::IO_HP_PAIR, 2),
        (tcls::IO_HP_S, 1),
        (tcls::IO_HP_N, 1),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for i in 0..num_io {
            let bel = bslots::IOB[i];
            let mut bctx = ctx.bel(bel);
            let bel_ologic = bslots::OLOGIC[i];
            let bel_odelay = bslots::ODELAY[i];
            let bel_other = if num_io == 1 {
                None
            } else {
                Some(bslots::IOB[i ^ 1])
            };
            bctx.build()
                .global("DCIUPDATEMODE", "ASREQUIRED")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .test_bel_special(specials::PRESENT)
                .mode("IOB18")
                .commit();
            bctx.build()
                .global("DCIUPDATEMODE", "QUIET")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .test_bel_special(specials::IOB_QUIET)
                .mode("IOB18")
                .commit();
            if num_io == 2 {
                bctx.build()
                    .global("DCIUPDATEMODE", "ASREQUIRED")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .test_bel_special(specials::IOB_IPAD)
                    .mode("IPAD")
                    .commit();
            }
            bctx.mode("IOB18")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .test_bel_attr_auto_default(IOB::PULL, enums::IOB_PULL::NONE);
            for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
                bctx.mode("IOB18")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .mutex("PULL_DYNAMIC", pin)
                    .test_bel_attr_bits(IOB::PULL_DYNAMIC)
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
                .test_bel_special_bits(specials::IOB_IPROGRAMMING)
                .multi_attr("IPROGRAMMING", MultiValue::Bin, 24);
            bctx.mode("IOB18")
                .global("UNCONSTRAINEDPINS", "ALLOW")
                .related_tile_mutex(HclkIoi, "VCCO", "1800")
                .pin("O")
                .attr("OUSED", "0")
                .attr("OSTANDARD", "LVCMOS18")
                .attr("DRIVE", "12")
                .attr("SLEW", "SLOW")
                .test_bel_special_bits(specials::IOB_OPROGRAMMING)
                .multi_attr("OPROGRAMMING", MultiValue::Bin, 34);
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
                    let (spec_dci, row_dci) = get_istd_row(edev, &std, true);
                    dci_special = Some(Dci(Some((spec_dci, row_dci))));
                    dci_special_lite = Some(Dci(None));
                }
                if std.diff != DiffKind::None {
                    if let Some(bel_other) = bel_other {
                        for (is_lp, lp) in [(true, "TRUE"), (false, "FALSE")] {
                            let (spec, row) = get_istd_row(edev, &std, is_lp);
                            bctx.mode("IOB18")
                                .global("DCIUPDATEMODE", "ASREQUIRED")
                                .global("UNCONSTRAINEDPINS", "ALLOW")
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
                        if std.diff == DiffKind::True && bel == bslots::IOB[0] {
                            let row = get_lvds_row(edev, &std);
                            bctx.mode("IOB18")
                                .global("UNCONSTRAINEDPINS", "ALLOW")
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
                            bctx.mode("IOB18")
                                .global("UNCONSTRAINEDPINS", "ALLOW")
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
                    }
                } else {
                    for (is_lp, lp) in [(true, "TRUE"), (false, "FALSE")] {
                        let (spec, row) = get_istd_row(edev, &std, is_lp);
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
                            .test_bel_special_row(spec, row)
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
                .test_bel_attr_bool_rename("IBUFDISABLE_SEL", IOB::IBUFDISABLE_EN, "GND", "I");
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
                .test_bel_attr_bool_rename(
                    "DCITERMDISABLE_SEL",
                    IOB::DCITERMDISABLE_EN,
                    "GND",
                    "I",
                );
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
                    let (spec_dci, row_dci) = get_istd_row(edev, &std, true);
                    dci_special = Some(Dci(Some((spec_dci, row_dci))));
                }
                if std.diff == DiffKind::True {
                    let row = get_lvds_row(edev, &std);
                    if bel == bslots::IOB[1] {
                        let bel_other = bel_other.unwrap();
                        bctx.build()
                            .global("UNCONSTRAINEDPINS", "ALLOW")
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
                    if bel == bslots::IOB[1] {
                        let bel_other = bel_other.unwrap();
                        for slew in ["SLOW", "FAST"] {
                            let (spec, row) = get_ostd_row(edev, &std, 0, slew);
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
                                    .bel_mode(bslots::OLOGIC[0], "OLOGICE2")
                                    .test_bel_special_row(spec, row)
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
                                    .bel_mode(bslots::OLOGIC[0], "OLOGICE2")
                                    .test_bel_special_row(spec, row)
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
                        let (spec, row) = get_ostd_row(edev, &std, 0, slew);
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
                            .test_bel_special_row(spec, row)
                            .attr("OUSED", "0")
                            .attr("IUSED", "0")
                            .attr("OSTANDARD", std.name)
                            .attr("ISTANDARD", std.name)
                            .attr("SLEW", slew)
                            .commit();
                    }
                } else {
                    let drives = if std.drive.is_empty() {
                        &[0][..]
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
                            let (spec, row) = get_ostd_row(edev, &std, drive, slew);
                            bctx.mode("IOB18")
                                .global("DCIUPDATEMODE", "ASREQUIRED")
                                .global("UNCONSTRAINEDPINS", "ALLOW")
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
                }
            }

            if num_io == 2 {
                for (row, std, vcco, vref) in [
                    (
                        IOB_DATA::HSTL_I_12,
                        "HSTL_I_12",
                        1200,
                        enums::INTERNAL_VREF::_600,
                    ),
                    (
                        IOB_DATA::SSTL135,
                        "SSTL135",
                        1350,
                        enums::INTERNAL_VREF::_675,
                    ),
                    (IOB_DATA::HSTL_I, "HSTL_I", 1500, enums::INTERNAL_VREF::_750),
                    (
                        IOB_DATA::HSTL_I_18,
                        "HSTL_I_18",
                        1800,
                        enums::INTERNAL_VREF::_900,
                    ),
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
                        .prop(VrefInternal(tcls::HCLK_IO_HP, vref))
                        .test_bel_special_row(specials::IOB_ISTD_LP, row)
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
                .test_bel_attr_bits_bi(IOB::OUTPUT_DELAY, false)
                .pip((bel_ologic, "IOB_O"), (bel_ologic, "OQ"))
                .commit();
            bctx.build()
                .mutex("OUTPUT_DELAY", "1")
                .bel_mode(bel_odelay, "ODELAYE2")
                .bel_mode(bel_ologic, "OLOGICE2")
                .test_bel_attr_bits_bi(IOB::OUTPUT_DELAY, true)
                .pip((bel_ologic, "IOB_O"), (bel_odelay, "DATAOUT"))
                .commit();
        }
    }
}

fn add_fuzzers_hclk_io_hp<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
) {
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

    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::HCLK_IO_HP) {
        let mut bctx = ctx.bel(bslots::DCI);
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_bel_attr_bits(DCI::TEST_ENABLE)
            .mode("DCI")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "QUIET")
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_bel_attr_bits(DCI::QUIET)
            .mode("DCI")
            .commit();
        bctx.build()
            .global("DCIUPDATEMODE", "ASREQUIRED")
            .global_mutex("GLOBAL_DCI", "NOPE")
            .test_bel_attr_bits(DCI::DYNAMIC_ENABLE)
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
                .extra_fixed_bel_attr_bits(
                    edev.tile_cfg(die),
                    bslots::MISC_CFG,
                    bcls::MISC_CFG::DCI_CLK_ENABLE_TR,
                );

            let anchor_reg = if chip.has_ps {
                RegId::from_idx(chip.regs - 2)
            } else {
                RegId::from_idx(0)
            };
            let io_row = chip.row_reg_hclk(anchor_reg) - 24;
            let io_tile = die.cell(edev.col_io_e.unwrap(), io_row).tile(tslots::BEL);
            let io_bel = io_tile.cell.bel(bslots::IOB[0]);
            let hclk_row = chip.row_hclk(io_tile.cell.row);
            let hclk_tile = die
                .cell(edev.col_io_e.unwrap(), hclk_row)
                .tile(tslots::HCLK_BEL);

            // Ensure nothing is placed in VR.
            for row in [
                chip.row_reg_hclk(anchor_reg) - 25,
                chip.row_reg_hclk(anchor_reg) + 24,
            ] {
                let vr_tile = die.cell(edev.col_io_e.unwrap(), row).tile(tslots::BEL);
                let vr_bel = vr_tile.cell.bel(bslots::IOB[0]);
                let site = backend.ngrid.get_bel_name(vr_bel).unwrap();
                builder = builder
                    .raw(Key::SiteMode(site), None)
                    .extra_fixed_bel_special(vr_tile, bslots::IOB[0], specials::IOB_VR);
            }

            // Set up hclk.
            builder = builder.extra_fixed_bel_attr_bits(hclk_tile, bslots::DCI, DCI::ENABLE);

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
                .extra_fixed_bel_special_row(
                    io_tile,
                    bslots::IOB[0],
                    specials::IOB_OSTD_FAST,
                    IOB_DATA::HSLVDCI_18,
                )
                .test_global_special(specials::CENTER_DCI)
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
            let col = edev.col_io_e.unwrap();
            let hclk_row_from = chip.row_reg_hclk(anchor_reg_from);
            let hclk_row_to = chip.row_reg_hclk(anchor_reg_to);
            let hclk_tile_to = die.cell(col, hclk_row_to).tile(tslots::HCLK_BEL);
            let io_row_from = hclk_row_from - 24;
            let io_bel_from = die.cell(col, io_row_from).bel(bslots::IOB[0]);
            let io_row_to = hclk_row_to - 24;
            let io_tile_to = die.cell(col, io_row_to).tile(tslots::BEL);
            let io_bel_to = io_tile_to.cell.bel(bslots::IOB[0]);
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
                for bel in [bslots::IOB[0], bslots::IOB[1]] {
                    if row == io_row_from && bel == bslots::IOB[0] {
                        continue;
                    }
                    if let Some(site) = backend.ngrid.get_bel_name(die.cell(col, row).bel(bel)) {
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
                for bel in [bslots::IOB[0], bslots::IOB[1]] {
                    if row == io_row_to && bel == bslots::IOB[0] {
                        continue;
                    }
                    if let Some(site) = backend.ngrid.get_bel_name(die.cell(col, row).bel(bel)) {
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
                .extra_fixed_bel_special_row(
                    io_tile_to,
                    bslots::IOB[0],
                    specials::IOB_OSTD_FAST,
                    IOB_DATA::HSLVDCI_18,
                )
                .extra_fixed_bel_attr_bits(
                    hclk_tile_to,
                    bslots::DCI,
                    if bank_to == 0 {
                        DCI::CASCADE_FROM_ABOVE
                    } else {
                        DCI::CASCADE_FROM_BELOW
                    },
                )
                .test_global_special(specials::CASCADE_DCI)
                .commit();
        }
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    add_fuzzers_iob_hp(session, backend);
    add_fuzzers_hclk_io_hp(session, backend);
}

fn collect_fuzzers_iob_hp(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    if !ctx.has_tcls(tcls::IO_HP_PAIR) {
        return;
    }

    for (tcid, idx) in [
        (tcls::IO_HP_PAIR, 0),
        (tcls::IO_HP_PAIR, 1),
        (tcls::IO_HP_S, 0),
        (tcls::IO_HP_N, 0),
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::IOB[idx];
        ctx.collect_bel_attr_default(tcid, bslot, IOB::PULL, enums::IOB_PULL::NONE);
        ctx.collect_bel_attr_bi(tcid, bslot, IOB::DCITERMDISABLE_EN);
        ctx.collect_bel_attr_bi(tcid, bslot, IOB::IBUFDISABLE_EN);
        ctx.collect_bel_attr(tcid, bslot, IOB::PULL_DYNAMIC);
        ctx.collect_bel_attr_bi(tcid, bslot, IOB::OUTPUT_DELAY);
        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        if tcid == tcls::IO_HP_PAIR {
            let diff = ctx
                .get_diff_bel_special(tcid, bslot, specials::IOB_IPAD)
                .combine(&!&present);
            ctx.insert_bel_attr_bool(tcid, bslot, IOB::VREF_SYSMON, xlat_bit(diff));
            if bslot == bslots::IOB[0] {
                let diff = ctx
                    .get_diff_bel_special(tcid, bslot, specials::IOB_VREF)
                    .combine(&!&present);
                ctx.insert_bel_attr_bool(tcid, bslot, IOB::VREF_SYSMON, xlat_bit(diff));
            }
        }
        let diff = ctx
            .get_diff_bel_special(tcid, bslot, specials::IOB_QUIET)
            .combine(&!&present);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::DCIUPDATEMODE_ASREQUIRED, xlat_bit(!diff));
        present.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, IOB::PULL),
            enums::IOB_PULL::NONE,
            enums::IOB_PULL::PULLDOWN,
        );

        let iprog = ctx.get_diffs_bel_special_bits(tcid, bslot, specials::IOB_IPROGRAMMING, 24);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::INPUT_MISC, xlat_bit(iprog[19].clone()));

        let oprog = xlat_bitvec(ctx.get_diffs_bel_special_bits(
            tcid,
            bslot,
            specials::IOB_OPROGRAMMING,
            34,
        ));
        let lvds = oprog[0..9].to_vec();
        let mut output_misc = oprog[9..14].to_vec();
        output_misc.push(oprog[19]);
        let dci_t = oprog[14];
        let dqsbias_n = oprog[17];
        let dqsbias_p = oprog[18];
        let dci_mode = BelAttributeEnum {
            bits: oprog[15..17].iter().map(|bit| bit.bit).collect(),
            values: [
                (enums::IOB_DCI_MODE::NONE, bits![0, 0]),
                (enums::IOB_DCI_MODE::OUTPUT, bits![1, 0]),
                (enums::IOB_DCI_MODE::OUTPUT_HALF, bits![0, 1]),
                (enums::IOB_DCI_MODE::TERM_SPLIT, bits![1, 1]),
            ]
            .into_iter()
            .collect(),
        };
        let mut pdrive = oprog[20..27].to_vec();
        let mut ndrive = oprog[27..34].to_vec();
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

        let (pslew, nslew) =
            if (tcid == tcls::IO_HP_PAIR && bslot == bslots::IOB[0]) || tcid == tcls::IO_HP_N {
                (
                    vec![
                        TileBit::new(idx, 38, 50).pos(),
                        TileBit::new(idx, 38, 30).pos(),
                        TileBit::new(idx, 38, 26).pos(),
                        TileBit::new(idx, 38, 16).pos(),
                        TileBit::new(idx, 39, 13).pos(),
                    ],
                    vec![
                        TileBit::new(idx, 38, 46).pos(),
                        TileBit::new(idx, 39, 45).pos(),
                        TileBit::new(idx, 38, 38).pos(),
                        TileBit::new(idx, 38, 22).pos(),
                        TileBit::new(idx, 38, 14).pos(),
                    ],
                )
            } else {
                (
                    vec![
                        TileBit::new(idx, 39, 13).pos(),
                        TileBit::new(idx, 39, 33).pos(),
                        TileBit::new(idx, 39, 37).pos(),
                        TileBit::new(idx, 39, 47).pos(),
                        TileBit::new(idx, 38, 50).pos(),
                    ],
                    vec![
                        TileBit::new(idx, 39, 17).pos(),
                        TileBit::new(idx, 38, 18).pos(),
                        TileBit::new(idx, 39, 25).pos(),
                        TileBit::new(idx, 39, 41).pos(),
                        TileBit::new(idx, 39, 49).pos(),
                    ],
                )
            };

        let mut diff = ctx
            .peek_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_FAST, IOB_DATA::HSTL_I)
            .combine(&present);
        diff.discard_polbits(&pdrive);
        diff.discard_polbits(&ndrive);
        diff.discard_polbits(&pslew);
        diff.discard_polbits(&nslew);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE, xlat_bit_wide(diff));

        let diff_cmos =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LP, IOB_DATA::LVCMOS18_2);
        let diff_vref_lp =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LP, IOB_DATA::HSTL_I);
        let diff_vref_hp =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_HP, IOB_DATA::HSTL_I);
        let mut diffs = vec![
            (enums::IOB_IBUF_MODE::NONE, Diff::default()),
            (enums::IOB_IBUF_MODE::CMOS, diff_cmos.clone()),
            (enums::IOB_IBUF_MODE::VREF, diff_vref_lp.clone()),
        ];
        let vref_hp = xlat_bit(diff_vref_hp.combine(&!diff_vref_lp));
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::IBUF_VREF_HP, vref_hp);
        if tcid == tcls::IO_HP_PAIR {
            let mut diff_diff_lp = ctx
                .peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LVDS_LP, LVDS_DATA::LVDS)
                .clone();
            let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.rect.to_idx() == idx);
            let mut diff_diff_hp = ctx
                .peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LVDS_HP, LVDS_DATA::LVDS)
                .clone();
            let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.rect.to_idx() == idx);
            let diff_hp = xlat_bit(diff_diff_hp.combine(&!&diff_diff_lp));
            ctx.insert_bel_attr_bool(tcid, bslot, IOB::IBUF_DIFF_HP, diff_hp);
            diffs.extend([(enums::IOB_IBUF_MODE::DIFF, diff_diff_lp)]);
        }
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::IBUF_MODE, xlat_enum_attr(diffs));

        for &std in HP_IOSTDS {
            if tcid != tcls::IO_HP_PAIR && std.name != "HSTL_I" {
                continue;
            }
            if std.diff != DiffKind::None {
                continue;
            }
            let drives = if !std.drive.is_empty() {
                std.drive
            } else {
                &[0][..]
            };
            let slews = if std.name.contains("LVDCI") {
                &[""][..]
            } else {
                &["SLOW", "FAST"]
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
                                Some(true) => true,
                                None => false,
                                _ => unreachable!(),
                            })
                            .collect();
                        ctx.insert_table_bitvec(IOB_DATA, row, field, value);
                    }
                    match std.dci {
                        DciKind::None | DciKind::InputSplit => {}
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
                            diff.apply_enum_diff(
                                ctx.bel_attr_enum(tcid, bslot, IOB::IBUF_MODE),
                                enums::IOB_IBUF_MODE::VREF,
                                enums::IOB_IBUF_MODE::NONE,
                            );
                        }
                        _ => unreachable!(),
                    }
                    diff.assert_empty();
                }
            }
        }
        for &std in HP_IOSTDS {
            if tcid != tcls::IO_HP_PAIR && !matches!(std.name, "LVCMOS18" | "HSTL_I") {
                continue;
            }
            if std.diff != DiffKind::None {
                continue;
            }
            if std.dci == DciKind::BiSplitT {
                continue;
            }
            for is_lp in [false, true] {
                let (spec, row) = get_istd_row(edev, &std, is_lp);
                let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                match std.dci {
                    DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                    DciKind::InputSplit | DciKind::BiSplit => {
                        diff.apply_enum_diff(
                            &dci_mode,
                            enums::IOB_DCI_MODE::TERM_SPLIT,
                            enums::IOB_DCI_MODE::NONE,
                        );
                    }
                    _ => unreachable!(),
                }
                let mode = if std.vref.is_some() {
                    if !is_lp {
                        diff.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslot, IOB::IBUF_VREF_HP),
                            true,
                            false,
                        );
                    }
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
        }

        if tcid != tcls::IO_HP_PAIR {
            let mut present_vr = ctx.get_diff_bel_special(tcid, bslot, specials::IOB_VR);
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
            ctx.insert_bel_attr_bool(tcid, bslot, IOB::VR, xlat_bit(present_vr));
        }

        ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::OUTPUT_T, bits![0; 9]);
        ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::OUTPUT_C, bits![0; 9]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::OUTPUT_MISC, bits![0; 6]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::PDRIVE, bits![0; 7]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::NDRIVE, bits![0; 7]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::PSLEW_FAST, bits![0; 5]);
        ctx.insert_table_bitvec(IOB_DATA, IOB_DATA::OFF, IOB_DATA::NSLEW_FAST, bits![0; 5]);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V5_LVDS, lvds);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::DCI_T, dci_t);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::DQS_BIAS_N, dqsbias_n);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::DQS_BIAS_P, dqsbias_p);
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::DCI_MODE, dci_mode);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V7_OUTPUT_MISC, output_misc);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V7_PDRIVE, pdrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V7_NDRIVE, ndrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V6_PSLEW, pslew);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::V6_NSLEW, nslew);

        present.assert_empty();
    }
    let tcid = tcls::IO_HP_PAIR;
    for &std in HP_IOSTDS {
        if std.diff == DiffKind::None {
            continue;
        }
        for idx in 0..2 {
            let bslot = bslots::IOB[idx];
            for is_lp in [false, true] {
                if std.dci == DciKind::BiSplitT {
                    continue;
                }
                let (spec, row) = get_istd_row(edev, &std, is_lp);
                let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                for bslot in [bslots::IOB[0], bslots::IOB[1]] {
                    match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => {}
                        DciKind::InputSplit | DciKind::BiSplit => {
                            diff.apply_enum_diff(
                                ctx.bel_attr_enum(tcid, bslot, IOB::DCI_MODE),
                                enums::IOB_DCI_MODE::TERM_SPLIT,
                                enums::IOB_DCI_MODE::NONE,
                            );
                        }
                        _ => unreachable!(),
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
            }
        }
        if std.diff == DiffKind::True {
            let row = get_lvds_row(edev, &std);
            let mut diff = ctx.get_diff_bel_special_row(
                tcid,
                bslots::IOB[0],
                specials::IOB_ISTD_LVDS_TERM,
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
            ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::TERM_T, val_t);
            ctx.insert_table_bitvec(LVDS_DATA, row, LVDS_DATA::TERM_C, val_c);
            diff.assert_empty();

            let mut diff = ctx.get_diff_bel_special_row(
                tcid,
                bslots::IOB[0],
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

            let mut diff =
                ctx.get_diff_bel_special_row(tcid, bslots::IOB[1], specials::IOB_OSTD_LVDS, row);
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
        if std.diff == DiffKind::Pseudo {
            for slew in ["SLOW", "FAST"] {
                let (spec, row) = get_ostd_row(edev, &std, 0, slew);
                let mut diff = ctx.get_diff_bel_special_row(tcid, bslots::IOB[1], spec, row);
                for idx in 0..2 {
                    let bslot = bslots::IOB[idx];
                    diff.apply_bitvec_diff(
                        ctx.bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE),
                        &bits![1; 2],
                        &bits![0; 2],
                    );
                    if !matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                        for (field, attr) in [
                            (IOB_DATA::PDRIVE, IOB::V7_PDRIVE),
                            (IOB_DATA::NDRIVE, IOB::V7_NDRIVE),
                        ] {
                            let item = ctx.bel_attr_bitvec(tcid, bslot, attr);
                            let value = extract_bitvec_val_part(
                                item,
                                &BitVec::repeat(false, item.len()),
                                &mut diff,
                            );
                            ctx.insert_table_bitvec(IOB_DATA, row, field, value);
                        }
                    }
                    let (field_pslew, field_nslew) = if slew == "SLOW" {
                        (IOB_DATA::PSLEW_SLOW, IOB_DATA::NSLEW_SLOW)
                    } else {
                        (IOB_DATA::PSLEW_FAST, IOB_DATA::NSLEW_FAST)
                    };
                    for (field, attr) in
                        [(field_pslew, IOB::V6_PSLEW), (field_nslew, IOB::V6_NSLEW)]
                    {
                        let item = ctx.bel_attr_bitvec(tcid, bslot, attr);
                        let value = extract_bitvec_val_part(
                            item,
                            &BitVec::repeat(false, item.len()),
                            &mut diff,
                        );
                        ctx.insert_table_bitvec(IOB_DATA, row, field, value);
                    }
                    let dci_mode = ctx.bel_attr_enum(tcid, bslot, IOB::DCI_MODE);
                    let dci_t = ctx.bel_attr_bit(tcid, bslot, IOB::DCI_T);
                    match std.dci {
                        DciKind::None | DciKind::InputSplit => {}
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
                        _ => unreachable!(),
                    }
                }
                let diff_t = diff.split_bits_by(|bit| bit.bit.to_idx() == 17);
                assert_eq!(diff.bits.len(), 1);
                assert_eq!(diff_t.bits.len(), 1);
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslots::IOB[0],
                    IOB::OUTPUT_PSEUDO_DIFF,
                    xlat_bit(diff),
                );
                ctx.insert_bel_attr_bool(
                    tcid,
                    bslots::IOB[0],
                    IOB::OUTPUT_PSEUDO_DIFF_T,
                    xlat_bit(diff_t),
                );
            }
        }
    }
}

fn collect_fuzzers_hclk_io_hp(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    let tcid = tcls::HCLK_IO_HP;
    if !ctx.has_tcls(tcid) {
        return;
    }
    let lvdsbias = vec![
        TileBit::new(0, 41, 14).pos(),
        TileBit::new(0, 41, 15).pos(),
        TileBit::new(0, 41, 16).pos(),
        TileBit::new(0, 41, 17).pos(),
        TileBit::new(0, 41, 18).pos(),
        TileBit::new(0, 41, 19).pos(),
        TileBit::new(0, 41, 20).pos(),
        TileBit::new(0, 41, 21).pos(),
        TileBit::new(0, 41, 22).pos(),
        TileBit::new(0, 41, 23).pos(),
        TileBit::new(0, 41, 24).pos(),
        TileBit::new(0, 41, 25).pos(),
        TileBit::new(0, 41, 26).pos(),
        TileBit::new(0, 41, 27).pos(),
        TileBit::new(0, 41, 28).pos(),
        TileBit::new(0, 41, 29).pos(),
        TileBit::new(0, 41, 30).pos(),
        TileBit::new(0, 40, 31).pos(),
    ];
    let nref_output = vec![TileBit::new(0, 39, 30).pos(), TileBit::new(0, 39, 29).pos()];
    let pref_output = vec![TileBit::new(0, 40, 18).pos(), TileBit::new(0, 40, 17).pos()];
    let nref_output_half = vec![
        TileBit::new(0, 39, 28).pos(),
        TileBit::new(0, 39, 27).pos(),
        TileBit::new(0, 39, 26).pos(),
    ];
    let pref_output_half = vec![
        TileBit::new(0, 40, 16).pos(),
        TileBit::new(0, 40, 15).pos(),
        TileBit::new(0, 40, 14).pos(),
    ];
    let nref_term_split = vec![
        TileBit::new(0, 39, 25).pos(),
        TileBit::new(0, 39, 24).pos(),
        TileBit::new(0, 39, 23).pos(),
    ];
    for std in HP_IOSTDS {
        if std.diff == DiffKind::True {
            let row = get_lvds_row(edev, std);
            let bslot = bslots::BANK;
            let diff = ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_LVDS, row);
            let val = extract_bitvec_val(&lvdsbias, &bits![0; 18], diff);
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
                DciKind::InputSplit | DciKind::BiSplit | DciKind::BiSplitT => {
                    let val = extract_bitvec_val_part(&nref_term_split, &bits![0; 3], &mut diff);
                    ctx.insert_table_bitvec(IOB_DATA, row, IOB_DATA::NREF_TERM_SPLIT, val);
                }
                _ => {}
            }
            ctx.insert_bel_attr_bool(tcid, bslot, DCI::ENABLE, xlat_bit(diff));
        }
    }
    let bslot = bslots::BANK;
    ctx.insert_bel_attr_bitvec(tcid, bslot, BANK::V7_LVDSBIAS, lvdsbias);
    ctx.insert_table_bitvec(LVDS_DATA, LVDS_DATA::OFF, LVDS_DATA::LVDSBIAS, bits![0; 18]);
    let bslot = bslots::DCI;
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCI::PREF_OUTPUT, pref_output);
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCI::NREF_OUTPUT, nref_output);
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCI::PREF_OUTPUT_HALF, pref_output_half);
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCI::NREF_OUTPUT_HALF, nref_output_half);
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCI::NREF_TERM_SPLIT, nref_term_split);
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
        IOB_DATA::NREF_TERM_SPLIT,
        bits![0; 3],
    );

    let dci_en = ctx.get_diff_attr_bool(tcid, bslot, DCI::ENABLE);
    let test_en = ctx.get_diff_attr_bool(tcid, bslot, DCI::TEST_ENABLE);
    let quiet = ctx
        .get_diff_attr_bool(tcid, bslot, DCI::QUIET)
        .combine(&!&test_en);
    ctx.insert_bel_attr_bool(tcid, bslot, DCI::QUIET, xlat_bit(quiet));
    let test_en = test_en.combine(&!&dci_en);
    let dyn_en = ctx
        .get_diff_attr_bool(tcid, bslot, DCI::DYNAMIC_ENABLE)
        .combine(&!&dci_en);
    ctx.insert_bel_attr_bitvec(tcid, bslot, DCI::TEST_ENABLE, xlat_bit_wide(test_en));
    ctx.insert_bel_attr_bool(tcid, bslot, DCI::DYNAMIC_ENABLE, xlat_bit(dyn_en));
    let casc_from_above = ctx
        .get_diff_attr_bool(tcid, bslot, DCI::CASCADE_FROM_ABOVE)
        .combine(&!&dci_en);
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCI::CASCADE_FROM_ABOVE,
        xlat_bit(casc_from_above),
    );
    let casc_from_below = ctx
        .get_diff_attr_bool(tcid, bslot, DCI::CASCADE_FROM_BELOW)
        .combine(&!&dci_en);
    ctx.insert_bel_attr_bool(
        tcid,
        bslot,
        DCI::CASCADE_FROM_BELOW,
        xlat_bit(casc_from_below),
    );
    ctx.insert_bel_attr_bool(tcid, bslot, DCI::ENABLE, xlat_bit(dci_en));

    let bslot = bslots::BANK;
    let mut diffs = vec![(enums::INTERNAL_VREF::OFF, Diff::default())];
    for val in [
        enums::INTERNAL_VREF::_600,
        enums::INTERNAL_VREF::_675,
        enums::INTERNAL_VREF::_750,
        enums::INTERNAL_VREF::_900,
    ] {
        diffs.push((
            val,
            ctx.get_diff_attr_val(tcid, bslot, BANK::INTERNAL_VREF, val),
        ));
    }
    // cannot be dealt with normally as there are no standards with such VREF.
    diffs.push((
        enums::INTERNAL_VREF::_1100,
        Diff {
            bits: [TileBit::new(0, 40, 19), TileBit::new(0, 40, 24)]
                .into_iter()
                .map(|x| (x, true))
                .collect(),
        },
    ));
    diffs.push((
        enums::INTERNAL_VREF::_1250,
        Diff {
            bits: [TileBit::new(0, 40, 19), TileBit::new(0, 40, 23)]
                .into_iter()
                .map(|x| (x, true))
                .collect(),
        },
    ));
    ctx.insert_bel_attr_enum(tcid, bslot, BANK::INTERNAL_VREF, xlat_enum_attr(diffs));

    let tcid = tcls::CFG;
    let bslot = bslots::MISC_CFG;
    let bits =
        xlat_bit_wide(ctx.get_diff_attr_bool(tcid, bslot, bcls::MISC_CFG::DCI_CLK_ENABLE_TR));
    ctx.insert_bel_attr_bitvec(tcid, bslot, bcls::MISC_CFG::DCI_CLK_ENABLE_TR, bits);
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    collect_fuzzers_iob_hp(ctx);
    collect_fuzzers_hclk_io_hp(ctx);
}
