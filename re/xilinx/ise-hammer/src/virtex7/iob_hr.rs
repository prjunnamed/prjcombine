use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::TableRowId,
    grid::{TileCoord, TileIobId},
};
use prjcombine_re_collector::diff::{
    Diff, SpecialId, extract_bitvec_val, extract_bitvec_val_part, xlat_bit, xlat_bitvec,
    xlat_enum_attr,
};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bsdata::TileBit};
use prjcombine_virtex4::{
    defs::{
        bcls::{BANK, IOB},
        bslots, enums,
        virtex7::{
            tables::{DRIVERBIAS, IOB_DATA_HR, LVDS_DATA_HR},
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
        iostd::{DiffKind, Iostd},
        props::{
            DynProp,
            bel::{BaseBelAttr, BaseBelMode, BaseBelPin, BaseBelPinPair},
            mutex::TileMutex,
            relation::{Delta, Related},
        },
    },
    virtex4::{io::IsBonded, specials},
    virtex5::io::{DiffOut, HclkIoi, VrefInternal},
    virtex7::iob_hp::Vref,
};

const HR_IOSTDS: &[Iostd] = &[
    Iostd::cmos("LVTTL", 3300, &[4, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS33", 3300, &[4, 8, 12, 16]),
    Iostd::cmos("LVCMOS25", 2500, &[4, 8, 12, 16]),
    Iostd::cmos("LVCMOS18", 1800, &[4, 8, 12, 16, 24]),
    Iostd::cmos("LVCMOS15", 1500, &[4, 8, 12, 16]),
    Iostd::cmos("LVCMOS12", 1200, &[4, 8, 12]),
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

fn get_lvds_row(edev: &prjcombine_virtex4::expanded::ExpandedDevice, iostd: &Iostd) -> TableRowId {
    edev.db[LVDS_DATA_HR].rows.get(iostd.name).unwrap().0
}

fn get_istd_row(
    edev: &prjcombine_virtex4::expanded::ExpandedDevice,
    iostd: &Iostd,
    is_lp: bool,
) -> (SpecialId, TableRowId) {
    if iostd.diff == DiffKind::True {
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
            edev.db[IOB_DATA_HR].rows.get(name).unwrap().0,
        )
    } else if iostd.drive.is_empty() {
        (
            if is_lp {
                specials::IOB_ISTD_LP
            } else {
                specials::IOB_ISTD_HP
            },
            edev.db[IOB_DATA_HR].rows.get(iostd.name).unwrap().0,
        )
    } else {
        (
            if is_lp {
                specials::IOB_ISTD_LP
            } else {
                specials::IOB_ISTD_HP
            },
            edev.db[IOB_DATA_HR]
                .rows
                .get(&format!("{}_4", iostd.name))
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
            edev.db[IOB_DATA_HR].rows.get(name).unwrap().0,
        )
    } else {
        (
            if slew == "SLOW" {
                specials::IOB_OSTD_SLOW
            } else {
                specials::IOB_OSTD_FAST
            },
            if iostd.drive.is_empty() {
                edev.db[IOB_DATA_HR].rows.get(iostd.name).unwrap().0
            } else {
                edev.db[IOB_DATA_HR]
                    .rows
                    .get(&format!("{std}_{drive}", std = iostd.name))
                    .unwrap()
                    .0
            },
        )
    }
}

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

fn add_fuzzers_iob_hr<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
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
        (tcls::IO_HR_PAIR, 2),
        (tcls::IO_HR_S, 1),
        (tcls::IO_HR_N, 1),
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        for i in 0..num_io {
            let bel = bslots::IOB[i];
            let mut bctx = ctx.bel(bel);
            let bel_other = if num_io == 1 {
                None
            } else {
                Some(bslots::IOB[i ^ 1])
            };

            bctx.build()
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .test_bel_special(specials::PRESENT)
                .mode("IOB33")
                .commit();
            if num_io == 2 {
                bctx.build()
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .test_bel_special(specials::IOB_IPAD)
                    .mode("IPAD")
                    .commit();
            }
            bctx.mode("IOB33")
                .raw(Key::Package, &package.name)
                .prop(IsBonded(bel))
                .test_bel_attr_auto_default(IOB::PULL, enums::IOB_PULL::NONE);
            for pin in ["PD_INT_EN", "PU_INT_EN", "KEEPER_INT_EN"] {
                bctx.mode("IOB33")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .mutex("PULL_DYNAMIC", pin)
                    .test_bel_attr_bits(IOB::PULL_DYNAMIC)
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
                .test_bel_special_bits(specials::IOB_OPROGRAMMING)
                .multi_attr("OPROGRAMMING", MultiValue::Bin, 39);
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
                .test_bel_special_bits(specials::IOB_IPROGRAMMING)
                .multi_attr("IPROGRAMMING", MultiValue::Bin, 9);
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
                .test_bel_attr_bool_rename("IBUFDISABLE_SEL", IOB::IBUFDISABLE_EN, "GND", "I");
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
                .test_bel_attr_bool_rename("INTERMDISABLE_SEL", IOB::INTERMDISABLE_EN, "GND", "I");
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
                .test_bel_attr_bool_auto(IOB::DQS_BIAS, "FALSE", "TRUE");
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
                .test_bel_attr_auto(IOB::IN_TERM);

            let anchor_props = |dy, vcco: u16, anchor_std: &'static str| -> [Box<DynProp>; 5] {
                let rel = Delta::new(0, dy, tcls::IO_HR_PAIR);
                [
                    Box::new(Related::new(
                        HclkIoi,
                        TileMutex::new("VCCO".into(), vcco.to_string().into()),
                    )),
                    Box::new(Related::new(
                        rel.clone(),
                        BaseBelMode::new(bslots::IOB[1], 0, "IOB33".into()),
                    )),
                    Box::new(Related::new(
                        rel.clone(),
                        BaseBelPin::new(bslots::IOB[1], 0, "O".into()),
                    )),
                    Box::new(Related::new(
                        rel.clone(),
                        BaseBelAttr::new(bslots::IOB[1], 0, "OUSED".into(), "0".into()),
                    )),
                    Box::new(Related::new(
                        rel.clone(),
                        BaseBelAttr::new(bslots::IOB[1], 0, "OSTANDARD".into(), anchor_std.into()),
                    )),
                ]
            };
            let anchor_dy = match tcid {
                tcls::IO_HR_S => 1,
                tcls::IO_HR_PAIR => 2,
                tcls::IO_HR_N => -2,
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
                        for (is_lp, lp) in [(true, "TRUE"), (false, "FALSE")] {
                            let (spec, row) = get_istd_row(edev, &std, is_lp);
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
                        if std.diff == DiffKind::True
                            && bel == bslots::IOB[0]
                            && std.name != "TMDS_33"
                        {
                            let row = get_lvds_row(edev, &std);
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
                                .test_bel_special_row(specials::IOB_ISTD_LVDS_TERM, row)
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
                        bctx.mode("IOB33")
                            .global("UNCONSTRAINEDPINS", "ALLOW")
                            .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                            .attr("OUSED", "")
                            .pin("I")
                            .raw(Key::Package, &package.name)
                            .prop(IsBonded(bel))
                            .maybe_prop(vref_special)
                            .test_bel_special_row(spec, row)
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
                    let row = get_lvds_row(edev, &std);
                    if bel == bslots::IOB[1] {
                        let bel_other = bel_other.unwrap();
                        bctx.build()
                            .global("UNCONSTRAINEDPINS", "ALLOW")
                            .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                            .attr("IUSED", "")
                            .attr("OPROGRAMMING", "")
                            .raw(Key::Package, &package.name)
                            .prop(IsBonded(bel))
                            .prop(DiffOut(specials::IOB_OSTD_LVDS_GROUP0, row))
                            .bel_attr(bel_other, "IUSED", "")
                            .bel_attr(bel_other, "OPROGRAMMING", "")
                            .bel_attr(bel_other, "OSTANDARD", "")
                            .bel_attr(bel_other, "OUSED", "")
                            .test_bel_special_row(specials::IOB_OSTD_LVDS_GROUP0, row)
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
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelMode::new(bslots::IOB[1], 0, "IOB33M".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelPin::new(bslots::IOB[1], 0, "O".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelAttr::new(bslots::IOB[1], 0, "OUSED".into(), "0".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelAttr::new(
                                        bslots::IOB[1],
                                        0,
                                        "DIFFO_OUTUSED".into(),
                                        "0".into(),
                                    ),
                                ))
                                .prop(Related::new(
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelAttr::new(
                                        bslots::IOB[1],
                                        0,
                                        "OSTANDARD".into(),
                                        alt_std.into(),
                                    ),
                                ))
                                .prop(Related::new(
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelMode::new(bslots::IOB[0], 0, "IOB33S".into()),
                                ))
                                .prop(Related::new(
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelPinPair::new(
                                        bslots::IOB[1],
                                        0,
                                        "DIFFO_OUT".into(),
                                        bslots::IOB[0],
                                        0,
                                        "DIFFO_IN".into(),
                                    ),
                                ))
                                .prop(Related::new(
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelAttr::new(
                                        bslots::IOB[0],
                                        0,
                                        "OUTMUX".into(),
                                        "1".into(),
                                    ),
                                ))
                                .prop(Related::new(
                                    Delta::new(0, 4, tcls::IO_HR_PAIR),
                                    BaseBelAttr::new(
                                        bslots::IOB[0],
                                        0,
                                        "DIFFO_INUSED".into(),
                                        "0".into(),
                                    ),
                                ))
                                .attr("IUSED", "")
                                .attr("OPROGRAMMING", "")
                                .raw(Key::Package, &package.name)
                                .prop(IsBonded(bel))
                                .prop(DiffOut(specials::IOB_OSTD_LVDS_GROUP1, row))
                                .bel_attr(bel_other, "IUSED", "")
                                .bel_attr(bel_other, "OPROGRAMMING", "")
                                .bel_attr(bel_other, "OSTANDARD", "")
                                .bel_attr(bel_other, "OUSED", "")
                                .test_bel_special_row(specials::IOB_OSTD_LVDS_GROUP1, row)
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
                    if bel == bslots::IOB[1] {
                        let bel_other = bel_other.unwrap();
                        let slews = if std.name == "BLVDS_25" {
                            &[""][..]
                        } else {
                            &["SLOW", "FAST"]
                        };
                        for &slew in slews {
                            let (spec, row) = get_ostd_row(edev, &std, 0, slew);
                            bctx.build()
                                .global("UNCONSTRAINEDPINS", "ALLOW")
                                .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                .attr("IUSED", "")
                                .attr("OPROGRAMMING", "")
                                .raw(Key::Package, &package.name)
                                .prop(IsBonded(bel))
                                .bel_attr(bel_other, "IUSED", "")
                                .bel_attr(bel_other, "OPROGRAMMING", "")
                                .bel_mode(bslots::OLOGIC[0], "OLOGICE2")
                                .test_bel_special_row(spec, row)
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
                        &[0][..]
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
                            let (spec, row) = get_ostd_row(edev, &std, drive, slew);
                            bctx.mode("IOB33")
                                .global("UNCONSTRAINEDPINS", "ALLOW")
                                .props(anchor_props(anchor_dy, std.vcco.unwrap(), anchor_std))
                                .pin("O")
                                .attr("IUSED", "")
                                .attr("OPROGRAMMING", "")
                                .raw(Key::Package, &package.name)
                                .prop(IsBonded(bel))
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
                        IOB_DATA_HR::HSUL_12,
                        "HSUL_12",
                        1200,
                        enums::INTERNAL_VREF::_600,
                    ),
                    (
                        IOB_DATA_HR::SSTL135,
                        "SSTL135",
                        1350,
                        enums::INTERNAL_VREF::_675,
                    ),
                    (
                        IOB_DATA_HR::HSTL_I,
                        "HSTL_I",
                        1500,
                        enums::INTERNAL_VREF::_750,
                    ),
                    (
                        IOB_DATA_HR::HSTL_I_18,
                        "HSTL_I_18",
                        1800,
                        enums::INTERNAL_VREF::_900,
                    ),
                ] {
                    bctx.mode("IOB33")
                        .global("UNCONSTRAINEDPINS", "ALLOW")
                        .props(anchor_props(anchor_dy, vcco, std))
                        .attr("OUSED", "")
                        .pin("I")
                        .raw(Key::Package, &package.name)
                        .prop(IsBonded(bel))
                        .prop(VrefInternal(tcls::HCLK_IO_HR, vref))
                        .test_bel_special_row(specials::IOB_ISTD_LP, row)
                        .attr("IUSED", "0")
                        .attr("ISTANDARD", std)
                        .attr("IBUF_LOW_PWR", "TRUE")
                        .commit();
                }
            }

            if tcid == tcls::IO_HR_S {
                let mut builder = bctx
                    .mode("IOB33")
                    .global("UNCONSTRAINEDPINS", "ALLOW")
                    .related_tile_mutex(HclkIoi, "VCCO", "TEST")
                    .pin("O")
                    .attr("IUSED", "")
                    .attr("OPROGRAMMING", "")
                    .raw(Key::Package, &package.name)
                    .prop(IsBonded(bel))
                    .extra_tile_attr_bits(
                        Delta::new(0, 49, tcls::IO_HR_N),
                        bslots::IOB[0],
                        IOB::LOW_VOLTAGE,
                    )
                    .extra_tile_bel_special(
                        Delta::new(0, 25, tcls::HCLK_IO_HR),
                        bslots::BANK,
                        specials::DRIVERBIAS_LV,
                    );
                for i in 0..24 {
                    builder = builder.extra_tile_attr_bits(
                        Delta::new(0, 1 + i * 2, tcls::IO_HR_PAIR),
                        bslots::IOB[0],
                        IOB::LOW_VOLTAGE,
                    );
                }
                builder
                    .test_bel_special(specials::IOB_OSTD_LVCMOS18_4_SLOW_EXCL)
                    .attr("OUSED", "0")
                    .attr("OSTANDARD", "LVCMOS18")
                    .attr("DRIVE", "4")
                    .attr("SLEW", "SLOW")
                    .commit();
            }
        }
    }
}

fn add_fuzzers_hclk_io_hr<'a>(
    session: &mut Session<'a, IseBackend<'a>>,
    backend: &'a IseBackend<'a>,
) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::HCLK_IO_HR) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::BANK);
    for (val, vname) in [
        (enums::VCCOSENSE_MODE::OFF, "OFF"),
        (enums::VCCOSENSE_MODE::FREEZE, "FREEZE"),
        (enums::VCCOSENSE_MODE::ALWAYSACTIVE, "ALWAYSACTIVE"),
    ] {
        bctx.build()
            .test_bel_attr_val(BANK::HR_VCCOSENSE_MODE, val)
            .prop(VccoSenseMode(vname))
            .commit();
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .extra_tiles_by_class_bel_attr_bits(tcls::HCLK_IO_HR, bslots::BANK, BANK::HR_VCCOSENSE_FLAG)
        .test_global_special(specials::VCCOSENSE_FLAG_ENABLE)
        .global("VCCOSENSEFLAG", "ENABLE")
        .commit();
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    add_fuzzers_iob_hr(session, backend);
    add_fuzzers_hclk_io_hr(session, backend);
}

fn collect_fuzzers_iob_hr(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    if !ctx.has_tcls(tcls::IO_HR_PAIR) {
        return;
    }

    for (tcid, idx) in [
        (tcls::IO_HR_PAIR, 0),
        (tcls::IO_HR_PAIR, 1),
        (tcls::IO_HR_S, 0),
        (tcls::IO_HR_N, 0),
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::IOB[idx];

        ctx.collect_bel_attr_default(tcid, bslot, IOB::PULL, enums::IOB_PULL::NONE);
        ctx.collect_bel_attr_bi(tcid, bslot, IOB::INTERMDISABLE_EN);
        ctx.collect_bel_attr_bi(tcid, bslot, IOB::IBUFDISABLE_EN);
        ctx.collect_bel_attr(tcid, bslot, IOB::PULL_DYNAMIC);
        ctx.collect_bel_attr_bi(tcid, bslot, IOB::DQS_BIAS);
        ctx.collect_bel_attr(tcid, bslot, IOB::IN_TERM);

        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);
        if tcid == tcls::IO_HR_PAIR {
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
        present.apply_enum_diff(
            ctx.bel_attr_enum(tcid, bslot, IOB::PULL),
            enums::IOB_PULL::NONE,
            enums::IOB_PULL::PULLDOWN,
        );

        let diff_cmos_lv = ctx.peek_diff_bel_special_row(
            tcid,
            bslot,
            specials::IOB_ISTD_LP,
            IOB_DATA_HR::LVCMOS18_4,
        );
        let diff_cmos_hv = ctx.peek_diff_bel_special_row(
            tcid,
            bslot,
            specials::IOB_ISTD_LP,
            IOB_DATA_HR::LVCMOS33_4,
        );
        let diff_vref_lp =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LP, IOB_DATA_HR::HSTL_I);
        let diff_vref_hp =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_HP, IOB_DATA_HR::HSTL_I);
        let diff_pci =
            ctx.peek_diff_bel_special_row(tcid, bslot, specials::IOB_ISTD_LP, IOB_DATA_HR::PCI33_3);
        let mut diffs = vec![
            (enums::IOB_IBUF_MODE::NONE, Diff::default()),
            (enums::IOB_IBUF_MODE::VREF, diff_vref_lp.clone()),
            (enums::IOB_IBUF_MODE::CMOS, diff_cmos_lv.clone()),
            (enums::IOB_IBUF_MODE::CMOS_HV, diff_cmos_hv.clone()),
        ];
        let vref_hp = xlat_bit(diff_vref_hp.combine(&!diff_vref_lp));
        let pci = xlat_bit(diff_pci.combine(&!diff_cmos_hv));
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::IBUF_VREF_HP, vref_hp);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::IBUF_PCI, pci);
        if tcid == tcls::IO_HR_PAIR {
            let mut diff_diff_lp = ctx
                .peek_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_LVDS_LP,
                    LVDS_DATA_HR::LVDS_25,
                )
                .clone();
            let diff_diff_lp = diff_diff_lp.split_bits_by(|bit| bit.rect.to_idx() == idx);
            let mut diff_diff_hp = ctx
                .peek_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_LVDS_HP,
                    LVDS_DATA_HR::LVDS_25,
                )
                .clone();
            let diff_diff_hp = diff_diff_hp.split_bits_by(|bit| bit.rect.to_idx() == idx);
            let mut diff_tmds_lp = ctx
                .peek_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_LVDS_LP,
                    LVDS_DATA_HR::TMDS_33,
                )
                .clone();
            let diff_tmds_lp = diff_tmds_lp.split_bits_by(|bit| bit.rect.to_idx() == idx);
            let mut diff_tmds_hp = ctx
                .peek_diff_bel_special_row(
                    tcid,
                    bslot,
                    specials::IOB_ISTD_LVDS_HP,
                    LVDS_DATA_HR::TMDS_33,
                )
                .clone();
            let diff_tmds_hp = diff_tmds_hp.split_bits_by(|bit| bit.rect.to_idx() == idx);
            let diff_hp = xlat_bit(diff_diff_hp.combine(&!&diff_diff_lp));
            ctx.insert_bel_attr_bool(tcid, bslot, IOB::IBUF_DIFF_HP, diff_hp);
            let diff_hp = xlat_bit(diff_tmds_hp.combine(&!&diff_tmds_lp));
            ctx.insert_bel_attr_bool(tcid, bslot, IOB::IBUF_DIFF_HP, diff_hp);
            diffs.extend([
                (enums::IOB_IBUF_MODE::DIFF, diff_diff_lp),
                (enums::IOB_IBUF_MODE::TMDS, diff_tmds_lp),
            ]);
        }
        ctx.insert_bel_attr_enum(tcid, bslot, IOB::IBUF_MODE, xlat_enum_attr(diffs));

        let iprog = ctx.get_diffs_bel_special_bits(tcid, bslot, specials::IOB_IPROGRAMMING, 9);
        ctx.insert_bel_attr_bool(tcid, bslot, IOB::INPUT_MISC, xlat_bit(iprog[7].clone()));

        for &std in HR_IOSTDS {
            if tcid != tcls::IO_HR_PAIR
                && !matches!(std.name, "LVCMOS18" | "LVCMOS33" | "PCI33_3" | "HSTL_I")
            {
                continue;
            }
            if std.diff != DiffKind::None {
                continue;
            }
            for is_lp in [false, true] {
                let (spec, row) = get_istd_row(edev, &std, is_lp);
                let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                let mode = if std.vref.is_some() {
                    if !is_lp {
                        diff.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslot, IOB::IBUF_VREF_HP),
                            true,
                            false,
                        );
                    }
                    enums::IOB_IBUF_MODE::VREF
                } else if std.name == "PCI33_3" {
                    diff.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, IOB::IBUF_PCI), true, false);
                    enums::IOB_IBUF_MODE::CMOS_HV
                } else if std.vcco.unwrap() < 2500 {
                    enums::IOB_IBUF_MODE::CMOS
                } else {
                    enums::IOB_IBUF_MODE::CMOS_HV
                };
                diff.apply_enum_diff(
                    ctx.bel_attr_enum(tcid, bslot, IOB::IBUF_MODE),
                    mode,
                    enums::IOB_IBUF_MODE::NONE,
                );
                diff.assert_empty();
            }
        }

        let mut oprog = ctx.get_diffs_bel_special_bits(tcid, bslot, specials::IOB_OPROGRAMMING, 39);
        ctx.insert_bel_attr_bitvec(
            tcid,
            bslot,
            IOB::OUTPUT_ENABLE,
            xlat_bitvec(oprog.split_off(37)),
        );
        if tcid == tcls::IO_HR_PAIR && bslot == bslots::IOB[0] {
            let mut diff = oprog.pop().unwrap();
            let diff_t = diff.split_bits_by(|bit| bit.bit.to_idx() == 59);
            assert_eq!(diff.bits.len(), 1);
            assert_eq!(diff_t.bits.len(), 1);
            ctx.insert_bel_attr_bool(tcid, bslot, IOB::OUTPUT_PSEUDO_DIFF, xlat_bit(diff));
            ctx.insert_bel_attr_bool(tcid, bslot, IOB::OUTPUT_PSEUDO_DIFF_T, xlat_bit(diff_t));
        } else {
            oprog.pop().unwrap().assert_empty();
        }
        oprog.pop().unwrap();
        ctx.insert_bel_attr_bool(
            tcid,
            bslot,
            IOB::LOW_VOLTAGE,
            xlat_bit(oprog.pop().unwrap()),
        );
        oprog.pop().unwrap();
        let mut pslew = xlat_bitvec(oprog.split_off(30));
        oprog.pop().unwrap();
        let mut nslew = xlat_bitvec(oprog.split_off(26));
        let mut output_misc = xlat_bitvec(oprog.split_off(24));
        oprog.pop().unwrap();
        oprog.pop().unwrap();
        let mut ndrive = xlat_bitvec(oprog.split_off(18));
        oprog.pop().unwrap();
        let mut pdrive = xlat_bitvec(oprog.split_off(14));
        oprog.pop().unwrap().assert_empty();
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::HR_LVDS, xlat_bitvec(oprog));
        for bit in pdrive
            .iter_mut()
            .chain(ndrive.iter_mut())
            .chain(output_misc.iter_mut())
        {
            bit.inv = match present.bits.remove(&bit.bit) {
                Some(true) => true,
                None => false,
                _ => unreachable!(),
            };
        }
        pslew[1].inv = true;
        nslew[1].inv = true;
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::HR_OUTPUT_MISC, output_misc);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::HR_PDRIVE, pdrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::HR_NDRIVE, ndrive);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::HR_PSLEW, pslew);
        ctx.insert_bel_attr_bitvec(tcid, bslot, IOB::HR_NSLEW, nslew);
        present.assert_empty();

        ctx.insert_table_bitvec(
            LVDS_DATA_HR,
            LVDS_DATA_HR::OFF,
            LVDS_DATA_HR::OUTPUT_T,
            bits![0; 13],
        );
        ctx.insert_table_bitvec(
            LVDS_DATA_HR,
            LVDS_DATA_HR::OFF,
            LVDS_DATA_HR::OUTPUT_C,
            bits![0; 13],
        );
        ctx.insert_table_bitvec(
            IOB_DATA_HR,
            IOB_DATA_HR::OFF,
            IOB_DATA_HR::PDRIVE,
            bits![0; 3],
        );
        ctx.insert_table_bitvec(
            IOB_DATA_HR,
            IOB_DATA_HR::OFF,
            IOB_DATA_HR::NDRIVE,
            bits![0; 4],
        );
        ctx.insert_table_bitvec(
            IOB_DATA_HR,
            IOB_DATA_HR::OFF,
            IOB_DATA_HR::OUTPUT_MISC,
            bits![0; 2],
        );
        ctx.insert_table_bitvec(
            IOB_DATA_HR,
            IOB_DATA_HR::OFF,
            IOB_DATA_HR::PSLEW_FAST,
            bits![0, 1, 0],
        );
        ctx.insert_table_bitvec(
            IOB_DATA_HR,
            IOB_DATA_HR::OFF,
            IOB_DATA_HR::NSLEW_FAST,
            bits![0, 1, 0],
        );

        if tcid == tcls::IO_HR_PAIR {
            for std in HR_IOSTDS {
                if std.diff != DiffKind::None {
                    continue;
                }
                let drives = if !std.drive.is_empty() {
                    std.drive
                } else {
                    &[0][..]
                };
                let slews = if std.name == "PCI33_3" {
                    &[""][..]
                } else {
                    &["SLOW", "FAST"]
                };
                for &drive in drives {
                    for &slew in slews {
                        let (spec, row) = get_ostd_row(edev, std, drive, slew);
                        let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                        diff.apply_bitvec_diff(
                            ctx.bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE),
                            &bits![1; 2],
                            &bits![0; 2],
                        );
                        let (field_pslew, field_nslew) = if slew == "SLOW" {
                            (IOB_DATA_HR::PSLEW_SLOW, IOB_DATA_HR::NSLEW_SLOW)
                        } else {
                            (IOB_DATA_HR::PSLEW_FAST, IOB_DATA_HR::NSLEW_FAST)
                        };
                        for (field, attr, base) in [
                            (IOB_DATA_HR::PDRIVE, IOB::HR_PDRIVE, bits![0; 3]),
                            (IOB_DATA_HR::NDRIVE, IOB::HR_NDRIVE, bits![0; 4]),
                            (field_pslew, IOB::HR_PSLEW, bits![0, 1, 0]),
                            (field_nslew, IOB::HR_NSLEW, bits![0, 1, 0]),
                            (IOB_DATA_HR::OUTPUT_MISC, IOB::HR_OUTPUT_MISC, bits![0; 2]),
                        ] {
                            let bits = ctx.bel_attr_bitvec(tcid, bslot, attr);
                            let value = extract_bitvec_val_part(bits, &base, &mut diff);
                            ctx.insert_table_bitvec(IOB_DATA_HR, row, field, value);
                        }
                        diff.assert_empty();
                    }
                }
            }
        }
    }
    let tcid = tcls::IO_HR_PAIR;
    for &std in HR_IOSTDS {
        if std.diff == DiffKind::None {
            continue;
        }
        for idx in 0..2 {
            let bslot = bslots::IOB[idx];
            for is_lp in [false, true] {
                let (spec, row) = get_istd_row(edev, &std, is_lp);
                let mut diff = ctx.get_diff_bel_special_row(tcid, bslot, spec, row);
                for idx in 0..2 {
                    if !is_lp {
                        diff.apply_bit_diff(
                            ctx.bel_attr_bit(tcid, bslots::IOB[idx], IOB::IBUF_DIFF_HP),
                            true,
                            false,
                        );
                    }
                    diff.apply_enum_diff(
                        ctx.bel_attr_enum(tcid, bslots::IOB[idx], IOB::IBUF_MODE),
                        if std.name == "TMDS_33" {
                            enums::IOB_IBUF_MODE::TMDS
                        } else {
                            enums::IOB_IBUF_MODE::DIFF
                        },
                        enums::IOB_IBUF_MODE::NONE,
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
                let (spec, row) = get_ostd_row(edev, &std, 0, slew);
                let mut diff = ctx.get_diff_bel_special_row(tcid, bslots::IOB[1], spec, row);
                for idx in 0..2 {
                    let bslot = bslots::IOB[idx];
                    diff.apply_bitvec_diff(
                        ctx.bel_attr_bitvec(tcid, bslot, IOB::OUTPUT_ENABLE),
                        &bits![1; 2],
                        &bits![0; 2],
                    );
                    let (field_pslew, field_nslew) = if slew == "SLOW" {
                        (IOB_DATA_HR::PSLEW_SLOW, IOB_DATA_HR::NSLEW_SLOW)
                    } else {
                        (IOB_DATA_HR::PSLEW_FAST, IOB_DATA_HR::NSLEW_FAST)
                    };
                    for (field, attr, base) in [
                        (IOB_DATA_HR::PDRIVE, IOB::HR_PDRIVE, bits![0; 3]),
                        (IOB_DATA_HR::NDRIVE, IOB::HR_NDRIVE, bits![0; 4]),
                        (field_pslew, IOB::HR_PSLEW, bits![0, 1, 0]),
                        (field_nslew, IOB::HR_NSLEW, bits![0, 1, 0]),
                        (IOB_DATA_HR::OUTPUT_MISC, IOB::HR_OUTPUT_MISC, bits![0; 2]),
                    ] {
                        let bits = ctx.bel_attr_bitvec(tcid, bslot, attr);
                        let value = extract_bitvec_val_part(bits, &base, &mut diff);
                        ctx.insert_table_bitvec(IOB_DATA_HR, row, field, value);
                    }
                }
                diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslots::IOB[0], IOB::OUTPUT_PSEUDO_DIFF),
                    true,
                    false,
                );
                diff.apply_bit_diff(
                    ctx.bel_attr_bit(tcid, bslots::IOB[0], IOB::OUTPUT_PSEUDO_DIFF_T),
                    true,
                    false,
                );
                diff.assert_empty();
            }
        } else {
            let row = get_lvds_row(edev, &std);
            if std.name != "TMDS_33" {
                let mut diff = ctx.get_diff_bel_special_row(
                    tcid,
                    bslots::IOB[0],
                    specials::IOB_ISTD_LVDS_TERM,
                    row,
                );
                let val_c = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslots::IOB[0], IOB::HR_LVDS),
                    &bits![0; 13],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslots::IOB[1], IOB::HR_LVDS),
                    &bits![0; 13],
                    &mut diff,
                );
                ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::TERM_T, val_t);
                ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::TERM_C, val_c);
                diff.assert_empty();

                let mut diff = ctx.get_diff_bel_special_row(
                    tcid,
                    bslots::IOB[0],
                    specials::IOB_ISTD_LVDS_DYN_TERM,
                    row,
                );
                let val_c = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslots::IOB[0], IOB::HR_LVDS),
                    &bits![0; 13],
                    &mut diff,
                );
                let val_t = extract_bitvec_val_part(
                    ctx.bel_attr_bitvec(tcid, bslots::IOB[1], IOB::HR_LVDS),
                    &bits![0; 13],
                    &mut diff,
                );
                ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::TERM_T, val_t);
                ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::TERM_C, val_c);
                diff.assert_empty();
            }

            let mut diff = ctx.get_diff_bel_special_row(
                tcid,
                bslots::IOB[1],
                specials::IOB_OSTD_LVDS_GROUP0,
                row,
            );
            if std.name != "TMDS_33" {
                let mut altdiff = ctx
                    .get_diff_bel_special_row(
                        tcid,
                        bslots::IOB[1],
                        specials::IOB_OSTD_LVDS_GROUP1,
                        row,
                    )
                    .combine(&!&diff);
                let diff1 = altdiff.split_bits_by(|bit| bit.rect.to_idx() == 1);
                ctx.insert_bel_attr_bool(tcid, bslots::IOB[0], IOB::LVDS_GROUP, xlat_bit(altdiff));
                ctx.insert_bel_attr_bool(tcid, bslots::IOB[1], IOB::LVDS_GROUP, xlat_bit(diff1));
            }
            let val_c = extract_bitvec_val_part(
                ctx.bel_attr_bitvec(tcid, bslots::IOB[0], IOB::HR_LVDS),
                &bits![0; 13],
                &mut diff,
            );
            let val_t = extract_bitvec_val_part(
                ctx.bel_attr_bitvec(tcid, bslots::IOB[1], IOB::HR_LVDS),
                &bits![0; 13],
                &mut diff,
            );
            ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::OUTPUT_T, val_t);
            ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::OUTPUT_C, val_c);
            diff.apply_bitvec_diff(
                ctx.bel_attr_bitvec(tcid, bslots::IOB[1], IOB::OUTPUT_ENABLE),
                &bits![1; 2],
                &bits![0; 2],
            );
            diff.assert_empty();
        }
    }
    ctx.collect_bel_attr(tcls::IO_HR_N, bslots::IOB[0], IOB::LOW_VOLTAGE);
    // meh.
    let _ = ctx.get_diff_bel_special(
        tcls::IO_HR_S,
        bslots::IOB[0],
        specials::IOB_OSTD_LVCMOS18_4_SLOW_EXCL,
    );
    let _ = ctx.get_diff_attr_bool(tcls::IO_HR_PAIR, bslots::IOB[0], IOB::LOW_VOLTAGE);
}

fn collect_fuzzers_hclk_io_hr(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };

    let tcid = tcls::HCLK_IO_HR;
    if !ctx.has_tcls(tcid) {
        return;
    }
    let bslot = bslots::BANK;

    ctx.collect_bel_attr(tcid, bslot, BANK::HR_VCCOSENSE_FLAG);
    ctx.collect_bel_attr(tcid, bslot, BANK::HR_VCCOSENSE_MODE);

    {
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
                bits: [TileBit::new(0, 38, 26), TileBit::new(0, 38, 29)]
                    .into_iter()
                    .map(|x| (x, true))
                    .collect(),
            },
        ));
        diffs.push((
            enums::INTERNAL_VREF::_1250,
            Diff {
                bits: [TileBit::new(0, 38, 26), TileBit::new(0, 38, 30)]
                    .into_iter()
                    .map(|x| (x, true))
                    .collect(),
            },
        ));
        ctx.insert_bel_attr_enum(tcid, bslot, BANK::INTERNAL_VREF, xlat_enum_attr(diffs));
    }
    {
        let item = vec![
            TileBit::new(0, 39, 16).pos(),
            TileBit::new(0, 39, 17).pos(),
            TileBit::new(0, 39, 18).pos(),
            TileBit::new(0, 38, 14).pos(),
            TileBit::new(0, 38, 15).pos(),
            TileBit::new(0, 39, 19).pos(),
            TileBit::new(0, 39, 20).pos(),
            TileBit::new(0, 39, 21).pos(),
            TileBit::new(0, 41, 26).pos(),
            TileBit::new(0, 41, 25).pos(),
            TileBit::new(0, 41, 24).pos(),
            TileBit::new(0, 41, 23).pos(),
            TileBit::new(0, 41, 22).pos(),
            TileBit::new(0, 41, 21).pos(),
            TileBit::new(0, 39, 14).pos(),
            TileBit::new(0, 39, 15).pos(),
        ];
        for row in [DRIVERBIAS::OFF, DRIVERBIAS::_3V3, DRIVERBIAS::_2V5] {
            ctx.insert_table_bitvec(DRIVERBIAS, row, DRIVERBIAS::DRIVERBIAS, bits![0; 16]);
        }
        let diff = ctx.get_diff_bel_special(tcid, bslot, specials::DRIVERBIAS_LV);
        let lv = extract_bitvec_val(&item, &bits![0; 16], diff);
        for row in [
            DRIVERBIAS::_1V8,
            DRIVERBIAS::_1V5,
            DRIVERBIAS::_1V35,
            DRIVERBIAS::_1V2,
        ] {
            ctx.insert_table_bitvec(DRIVERBIAS, row, DRIVERBIAS::DRIVERBIAS, lv.clone());
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, BANK::HR_DRIVERBIAS, item);
    }
    {
        let common = vec![
            TileBit::new(0, 40, 30).pos(),
            TileBit::new(0, 40, 28).pos(),
            TileBit::new(0, 40, 27).pos(),
            TileBit::new(0, 40, 26).pos(),
            TileBit::new(0, 40, 25).pos(),
            TileBit::new(0, 40, 31).pos(),
            TileBit::new(0, 39, 23).pos(),
            TileBit::new(0, 41, 31).pos(),
            TileBit::new(0, 41, 30).pos(),
        ];
        let group0 = vec![
            TileBit::new(0, 38, 23).pos(),
            TileBit::new(0, 38, 24).pos(),
            TileBit::new(0, 38, 25).pos(),
            TileBit::new(0, 41, 29).pos(),
            TileBit::new(0, 41, 28).pos(),
            TileBit::new(0, 41, 27).pos(),
            TileBit::new(0, 41, 14).pos(),
            TileBit::new(0, 41, 20).pos(),
            TileBit::new(0, 41, 19).pos(),
            TileBit::new(0, 41, 18).pos(),
            TileBit::new(0, 41, 17).pos(),
            TileBit::new(0, 41, 16).pos(),
            TileBit::new(0, 41, 15).pos(),
            TileBit::new(0, 38, 28).pos(),
            TileBit::new(0, 38, 27).pos(),
            TileBit::new(0, 40, 29).pos(),
        ];
        let group1 = vec![
            TileBit::new(0, 38, 18).pos(),
            TileBit::new(0, 38, 19).pos(),
            TileBit::new(0, 38, 20).pos(),
            TileBit::new(0, 40, 24).pos(),
            TileBit::new(0, 40, 23).pos(),
            TileBit::new(0, 40, 22).pos(),
            TileBit::new(0, 40, 21).pos(),
            TileBit::new(0, 40, 20).pos(),
            TileBit::new(0, 40, 19).pos(),
            TileBit::new(0, 40, 18).pos(),
            TileBit::new(0, 40, 17).pos(),
            TileBit::new(0, 40, 16).pos(),
            TileBit::new(0, 40, 15).pos(),
            TileBit::new(0, 40, 14).pos(),
            TileBit::new(0, 39, 31).pos(),
            TileBit::new(0, 38, 31).pos(),
        ];
        for std in HR_IOSTDS {
            if std.diff != DiffKind::True {
                continue;
            }
            let row = get_lvds_row(edev, std);
            let mut diff =
                ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_LVDS_GROUP0, row);
            let vc = extract_bitvec_val_part(&common, &bits![0; 9], &mut diff);
            let val = extract_bitvec_val(&group0, &bits![0; 16], diff);
            ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::LVDSBIAS_COMMON, vc);
            ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::LVDSBIAS_GROUP, val);
            if std.name != "TMDS_33" {
                let diff =
                    ctx.get_diff_bel_special_row(tcid, bslot, specials::IOB_OSTD_LVDS_GROUP1, row);
                let val = extract_bitvec_val(&group1, &bits![0; 16], diff);
                ctx.insert_table_bitvec(LVDS_DATA_HR, row, LVDS_DATA_HR::LVDSBIAS_GROUP, val);
            }
        }
        ctx.insert_table_bitvec(
            LVDS_DATA_HR,
            LVDS_DATA_HR::OFF,
            LVDS_DATA_HR::LVDSBIAS_COMMON,
            bits![0; 9],
        );
        ctx.insert_table_bitvec(
            LVDS_DATA_HR,
            LVDS_DATA_HR::OFF,
            LVDS_DATA_HR::LVDSBIAS_GROUP,
            bits![0; 16],
        );

        let group = Vec::from_iter([group0, group1].into_iter().flatten());
        ctx.insert_bel_attr_bitvec(tcid, bslot, BANK::HR_LVDS_COMMON, common);
        ctx.insert_bel_attr_bitvec(tcid, bslot, BANK::HR_LVDS_GROUP, group);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    collect_fuzzers_iob_hr(ctx);
    collect_fuzzers_hclk_io_hr(ctx);
}
