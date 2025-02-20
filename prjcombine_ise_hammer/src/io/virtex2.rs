use std::collections::{hash_map, HashMap, HashSet};

use bitvec::prelude::*;
use prjcombine_collector::{
    enum_ocd_swap_bits, xlat_bit, xlat_bit_wide, xlat_bitvec, xlat_enum, xlat_enum_ocd,
    xlat_item_tile_fwd, Diff, OcdMode,
};
use prjcombine_hammer::Session;
use prjcombine_interconnect::db::BelId;
use prjcombine_types::tiledb::{TileBit, TileItem, TileItemKind};
use prjcombine_virtex2::grid::GridKind;
use prjcombine_xilinx_geom::{Bond, Device, ExpandedDevice, GeomDb};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    diff::CollectorCtx,
    fgen::{BelKV, TileBits, TileKV, TileRelation},
    fuzz::FuzzCtx,
    fuzz_enum, fuzz_inv, fuzz_multi, fuzz_one,
    io::iostd::Iostd,
};

use super::iostd::{DciKind, DiffKind};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum IobDiff {
    None,
    True(usize),
    Comp(usize),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct IobData {
    tile: usize,
    bel: BelId,
    diff: IobDiff,
    is_ibuf: bool,
}

fn iobs_data(iobs: &str) -> Vec<IobData> {
    fn iob(tile: usize, bel: usize) -> IobData {
        IobData {
            is_ibuf: false,
            diff: IobDiff::None,
            tile,
            bel: BelId::from_idx(bel),
        }
    }
    fn iobt(tile: usize, bel: usize, other: usize) -> IobData {
        IobData {
            is_ibuf: false,
            diff: IobDiff::True(other),
            tile,
            bel: BelId::from_idx(bel),
        }
    }
    fn iobc(tile: usize, bel: usize, other: usize) -> IobData {
        IobData {
            is_ibuf: false,
            diff: IobDiff::Comp(other),
            tile,
            bel: BelId::from_idx(bel),
        }
    }
    fn ibuf(tile: usize, bel: usize) -> IobData {
        IobData {
            is_ibuf: true,
            diff: IobDiff::None,
            tile,
            bel: BelId::from_idx(bel),
        }
    }
    fn ibuft(tile: usize, bel: usize, other: usize) -> IobData {
        IobData {
            is_ibuf: true,
            diff: IobDiff::True(other),
            tile,
            bel: BelId::from_idx(bel),
        }
    }
    fn ibufc(tile: usize, bel: usize, other: usize) -> IobData {
        IobData {
            is_ibuf: true,
            diff: IobDiff::Comp(other),
            tile,
            bel: BelId::from_idx(bel),
        }
    }

    match iobs {
        // Virtex 2
        "IOBS.V2P.T.L1" => vec![iob(0, 2), iobc(0, 1, 2), iobt(0, 0, 1)],
        "IOBS.V2P.T.L1.ALT" => vec![iobc(0, 2, 1), iobt(0, 1, 0), iob(0, 0)],
        "IOBS.V2P.T.R1" => vec![iobc(0, 3, 1), iobt(0, 2, 0), iob(0, 1)],
        "IOBS.V2P.T.R1.ALT" => vec![iob(0, 3), iobc(0, 2, 2), iobt(0, 1, 1)],
        "IOBS.V2.T.L2" | "IOBS.V2P.T.L2" => vec![
            iobc(0, 3, 1),
            iobt(0, 2, 0),
            iobc(0, 1, 3),
            iobt(0, 0, 2),
            iobc(1, 1, 5),
            iobt(1, 0, 4),
        ],
        "IOBS.V2.T.R2" | "IOBS.V2P.T.R2" => vec![
            iobc(0, 3, 1),
            iobt(0, 2, 0),
            iobc(1, 3, 3),
            iobt(1, 2, 2),
            iobc(1, 1, 5),
            iobt(1, 0, 4),
        ],
        "IOBS.V2P.T.R2.CLK" => vec![iobc(0, 3, 1), iobt(0, 2, 0), iobc(1, 3, 3), iobt(1, 2, 2)],
        "IOBS.V2.R.T2" | "IOBS.V2P.R.T2" => vec![
            iobc(1, 3, 1),
            iobt(1, 2, 0),
            iobc(1, 1, 3),
            iobt(1, 0, 2),
            iobc(0, 1, 5),
            iobt(0, 0, 4),
        ],
        "IOBS.V2.R.B2" | "IOBS.V2P.R.B2" => vec![
            iobc(1, 3, 1),
            iobt(1, 2, 0),
            iobc(0, 3, 3),
            iobt(0, 2, 2),
            iobc(0, 1, 5),
            iobt(0, 0, 4),
        ],
        "IOBS.V2P.B.R1" => vec![iob(0, 2), iobc(0, 1, 2), iobt(0, 0, 1)],
        "IOBS.V2P.B.R1.ALT" => vec![iobc(0, 2, 1), iobt(0, 1, 0), iob(0, 0)],
        "IOBS.V2P.B.L1" => vec![iobc(0, 3, 1), iobt(0, 2, 0), iob(0, 1)],
        "IOBS.V2P.B.L1.ALT" => vec![iob(0, 3), iobc(0, 2, 2), iobt(0, 1, 1)],
        "IOBS.V2.B.R2" | "IOBS.V2P.B.R2" => vec![
            iobc(1, 3, 1),
            iobt(1, 2, 0),
            iobc(1, 1, 3),
            iobt(1, 0, 2),
            iobc(0, 1, 5),
            iobt(0, 0, 4),
        ],
        "IOBS.V2.B.L2" | "IOBS.V2P.B.L2" => vec![
            iobc(1, 3, 1),
            iobt(1, 2, 0),
            iobc(0, 3, 3),
            iobt(0, 2, 2),
            iobc(0, 1, 5),
            iobt(0, 0, 4),
        ],
        "IOBS.V2P.B.R2.CLK" => vec![iobc(1, 1, 1), iobt(1, 0, 0), iobc(0, 1, 3), iobt(0, 0, 2)],
        "IOBS.V2.L.B2" | "IOBS.V2P.L.B2" => vec![
            iobt(0, 0, 1),
            iobc(0, 1, 0),
            iobt(0, 2, 3),
            iobc(0, 3, 2),
            iobt(1, 2, 5),
            iobc(1, 3, 4),
        ],
        "IOBS.V2.L.T2" | "IOBS.V2P.L.T2" => vec![
            iobt(0, 0, 1),
            iobc(0, 1, 0),
            iobt(1, 0, 3),
            iobc(1, 1, 2),
            iobt(1, 2, 5),
            iobc(1, 3, 4),
        ],

        // Spartan 3
        "IOBS.S3.T2" => vec![
            iob(0, 2),
            iobc(0, 1, 2),
            iobt(0, 0, 1),
            iobc(1, 1, 4),
            iobt(1, 0, 3),
        ],
        "IOBS.S3.R1" => vec![iobc(0, 1, 1), iobt(0, 0, 0)],
        "IOBS.S3.B2" => vec![
            iob(1, 2),
            iobc(1, 1, 2),
            iobt(1, 0, 1),
            iobc(0, 1, 4),
            iobt(0, 0, 3),
        ],
        "IOBS.S3.L1" => vec![iobc(0, 0, 1), iobt(0, 1, 0)],

        // Spartan 3E
        "IOBS.S3E.T1" => vec![iob(0, 2)],
        "IOBS.S3E.T2" => vec![iobc(0, 1, 1), iobt(0, 0, 0), ibuf(1, 2)],
        "IOBS.S3E.T3" => vec![
            iobc(0, 1, 1),
            iobt(0, 0, 0),
            ibuf(1, 2),
            iobc(2, 1, 4),
            iobt(2, 0, 3),
        ],
        "IOBS.S3E.T4" => vec![
            iobc(0, 1, 1),
            iobt(0, 0, 0),
            iob(1, 2),
            iobc(2, 1, 4),
            iobt(2, 0, 3),
            ibufc(3, 1, 6),
            ibuft(3, 0, 5),
        ],
        "IOBS.S3E.R1" => vec![iob(0, 2)],
        "IOBS.S3E.R2" => vec![iobc(0, 1, 1), iobt(0, 0, 0)],
        "IOBS.S3E.R3" => vec![ibuf(2, 2), iob(1, 2), iobc(0, 1, 3), iobt(0, 0, 2)],
        "IOBS.S3E.R4" => vec![
            ibuf(3, 2),
            iobc(2, 1, 2),
            iobt(2, 0, 1),
            iobc(0, 1, 4),
            iobt(0, 0, 3),
        ],
        "IOBS.S3E.B1" => vec![iob(0, 2)],
        "IOBS.S3E.B2" => vec![iobc(1, 1, 1), iobt(1, 0, 0), ibuf(0, 2)],
        "IOBS.S3E.B3" => vec![
            iobc(2, 1, 1),
            iobt(2, 0, 0),
            ibuf(1, 2),
            iobc(0, 1, 4),
            iobt(0, 0, 3),
        ],
        "IOBS.S3E.B4" => vec![
            iobc(3, 1, 1),
            iobt(3, 0, 0),
            iob(2, 2),
            iobc(1, 1, 4),
            iobt(1, 0, 3),
            ibufc(0, 1, 6),
            ibuft(0, 0, 5),
        ],
        "IOBS.S3E.L1" => vec![iob(0, 2)],
        "IOBS.S3E.L2" => vec![iobc(1, 1, 1), iobt(1, 0, 0)],
        "IOBS.S3E.L3" => vec![ibuf(0, 2), iob(1, 2), iobc(2, 1, 3), iobt(2, 0, 2)],
        "IOBS.S3E.L4" => vec![
            ibuf(0, 2),
            iobc(1, 1, 2),
            iobt(1, 0, 1),
            iobc(3, 1, 4),
            iobt(3, 0, 3),
        ],

        // Spartan 3A
        "IOBS.S3A.T2" => vec![
            iobc(0, 0, 1),
            iobt(0, 1, 0),
            ibuf(0, 2),
            iobc(1, 0, 4),
            iobt(1, 1, 3),
        ],
        "IOBS.S3A.R4" => vec![
            ibufc(3, 1, 1),
            ibuft(3, 0, 0),
            iobc(2, 1, 3),
            iobt(2, 0, 2),
            iobc(1, 1, 5),
            iobt(1, 0, 4),
            iobc(0, 1, 7),
            iobt(0, 0, 6),
        ],
        "IOBS.S3A.B2" => vec![
            iobc(1, 1, 1),
            iobt(1, 0, 0),
            ibuf(0, 2),
            iobc(0, 1, 4),
            iobt(0, 0, 3),
        ],
        "IOBS.S3A.L4" => vec![
            ibufc(0, 0, 1),
            ibuft(0, 1, 0),
            iobc(1, 0, 3),
            iobt(1, 1, 2),
            iobc(2, 0, 5),
            iobt(2, 1, 4),
            iobc(3, 0, 7),
            iobt(3, 1, 6),
        ],

        _ => unreachable!(),
    }
}

fn has_any_vref<'a>(
    edev: &prjcombine_virtex2::expanded::ExpandedDevice,
    device: &'a Device,
    db: &GeomDb,
    tile: &str,
    iob_idx: usize,
) -> Option<&'a str> {
    let node_kind = edev.egrid.db.get_node(tile);
    let iobs = iobs_data(tile);
    let ioi_tile = iobs[iob_idx].tile;
    let ioi_bel = iobs[iob_idx].bel;
    let mut bonded_ios = HashMap::new();
    for devbond in device.bonds.values() {
        let bond = &db.bonds[devbond.bond];
        let Bond::Virtex2(bond) = bond else {
            unreachable!()
        };
        for &io in &bond.vref {
            bonded_ios.insert(io, &devbond.name[..]);
        }
    }
    for &(_, mut col, mut row, _) in &edev.egrid.node_index[node_kind] {
        if col == edev.grid.col_left() || col == edev.grid.col_right() {
            row += ioi_tile;
        } else {
            col += ioi_tile
        }
        let crd = edev.grid.get_io_crd(col, row, ioi_bel);
        if let Some(&pkg) = bonded_ios.get(&crd) {
            return Some(pkg);
        }
    }
    None
}

fn has_any_vr<'a>(
    edev: &prjcombine_virtex2::expanded::ExpandedDevice,
    device: &'a Device,
    db: &GeomDb,
    tile: &str,
    iob_idx: usize,
) -> Option<(&'a str, Option<bool>)> {
    let node_kind = edev.egrid.db.get_node(tile);
    let iobs = iobs_data(tile);
    let ioi_tile = iobs[iob_idx].tile;
    let ioi_bel = iobs[iob_idx].bel;
    let mut bonded_ios = HashMap::new();
    for devbond in device.bonds.values() {
        let bond = &db.bonds[devbond.bond];
        let Bond::Virtex2(bond) = bond else {
            unreachable!()
        };
        for pin in bond.pins.values() {
            if let prjcombine_virtex2::bond::BondPin::Io(io) = pin {
                bonded_ios.insert(io, &devbond.name[..]);
            }
        }
    }
    for &(_, mut col, mut row, _) in &edev.egrid.node_index[node_kind] {
        if col == edev.grid.col_left() || col == edev.grid.col_right() {
            row += ioi_tile;
        } else {
            col += ioi_tile
        }
        let crd = edev.grid.get_io_crd(col, row, ioi_bel);
        if let Some(&pkg) = bonded_ios.get(&crd) {
            for bank in 0..8 {
                if let Some(alt_vr) = edev.grid.dci_io_alt.get(&bank) {
                    if crd == alt_vr.0 || crd == alt_vr.1 {
                        return Some((pkg, Some(true)));
                    }
                    if let Some(vr) = edev.grid.dci_io_alt.get(&bank) {
                        if crd == vr.0 || crd == vr.1 {
                            return Some((pkg, Some(false)));
                        }
                    }
                } else if let Some(vr) = edev.grid.dci_io.get(&bank) {
                    if crd == vr.0 || crd == vr.1 {
                        return Some((pkg, None));
                    }
                }
            }
        }
    }
    None
}

fn has_any_brefclk(
    edev: &prjcombine_virtex2::expanded::ExpandedDevice,
    tile: &str,
    iob_idx: usize,
) -> Option<(usize, usize)> {
    if edev.grid.kind != GridKind::Virtex2P {
        return None;
    }
    match (tile, iob_idx) {
        ("IOBS.V2P.B.L2", 5) => Some((1, 0)),
        ("IOBS.V2P.B.R2", 1) => Some((0, 6)),
        ("IOBS.V2P.T.L2", 1) => Some((1, 2)),
        ("IOBS.V2P.T.R2", 5) => Some((0, 4)),
        _ => None,
    }
}

#[allow(clippy::nonminimal_bool)]
pub fn get_iostds(edev: &prjcombine_virtex2::expanded::ExpandedDevice, lr: bool) -> Vec<Iostd> {
    let mut res = vec![];
    // plain push-pull
    if edev.grid.kind.is_virtex2() {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else if edev.grid.kind == GridKind::Spartan3 {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12"]),
            Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
        ]);
    } else if edev.grid.kind == GridKind::Spartan3E {
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6"]),
            Iostd::cmos("LVCMOS12", 1200, &["2"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else if lr {
        // spartan3a lr
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6", "8", "12"]),
            Iostd::cmos("LVCMOS12", 1200, &["2", "4", "6"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    } else {
        // spartan3a tb
        res.extend([
            Iostd::cmos("LVTTL", 3300, &["2", "4", "6", "8", "12", "16", "24"]),
            Iostd::cmos("LVCMOS33", 3300, &["2", "4", "6", "8", "12", "16"]),
            Iostd::cmos("LVCMOS25", 2500, &["2", "4", "6", "8", "12"]),
            Iostd::cmos("LVCMOS18", 1800, &["2", "4", "6", "8"]),
            Iostd::cmos("LVCMOS15", 1500, &["2", "4", "6"]),
            Iostd::cmos("LVCMOS12", 1200, &["2"]),
            Iostd::cmos("PCI33_3", 3300, &[]),
            Iostd::cmos("PCI66_3", 3300, &[]),
            Iostd::cmos("PCIX", 3300, &[]),
        ]);
    }
    // DCI output
    if !edev.grid.kind.is_spartan3ea() {
        res.push(Iostd::odci("LVDCI_33", 3300));
        res.push(Iostd::odci("LVDCI_25", 2500));
        res.push(Iostd::odci("LVDCI_18", 1800));
        res.push(Iostd::odci("LVDCI_15", 1500));
        if !edev.grid.kind.is_virtex2p() {
            res.push(Iostd::odci_half("LVDCI_DV2_33", 3300));
        }
        res.push(Iostd::odci_half("LVDCI_DV2_25", 2500));
        res.push(Iostd::odci_half("LVDCI_DV2_18", 1800));
        res.push(Iostd::odci_half("LVDCI_DV2_15", 1500));
        res.push(Iostd::odci_vref("HSLVDCI_33", 3300, 1650));
        res.push(Iostd::odci_vref("HSLVDCI_25", 2500, 1250));
        res.push(Iostd::odci_vref("HSLVDCI_18", 1800, 900));
        res.push(Iostd::odci_vref("HSLVDCI_15", 1500, 750));
    }
    // VREF-based
    if !edev.grid.kind.is_spartan3ea() {
        res.push(Iostd::vref_od("GTL", 800));
        res.push(Iostd::vref_od("GTLP", 1000));
    }
    if edev.grid.kind == GridKind::Virtex2 {
        res.push(Iostd::vref("AGP", 3300, 1320));
    }
    if edev.grid.kind == GridKind::Virtex2 || edev.grid.kind.is_spartan3a() {
        res.push(Iostd::vref("SSTL3_I", 3300, 1500));
        res.push(Iostd::vref("SSTL3_II", 3300, 1500));
    }
    res.push(Iostd::vref("SSTL2_I", 2500, 1250));
    res.push(Iostd::vref("SSTL18_I", 1800, 900));
    if edev.grid.kind != GridKind::Spartan3E && !(edev.grid.kind.is_spartan3a() && !lr) {
        res.push(Iostd::vref("SSTL2_II", 2500, 1250));
        res.push(Iostd::vref("SSTL18_II", 1800, 900));
    }
    res.push(Iostd::vref("HSTL_I_18", 1800, 900));
    if edev.grid.kind != GridKind::Spartan3E && !(edev.grid.kind.is_spartan3a() && !lr) {
        res.push(Iostd::vref("HSTL_II_18", 1800, 900));
    }
    res.push(Iostd::vref("HSTL_III_18", 1800, 1100));
    if edev.grid.kind.is_virtex2() {
        res.push(Iostd::vref("HSTL_IV_18", 1800, 1100));
    }
    if edev.grid.kind != GridKind::Spartan3E && !(edev.grid.kind.is_spartan3a() && !lr) {
        res.push(Iostd::vref("HSTL_I", 1500, 750));
        res.push(Iostd::vref("HSTL_III", 1500, 900));
    }
    if edev.grid.kind.is_virtex2() {
        res.push(Iostd::vref("HSTL_II", 1500, 750));
        res.push(Iostd::vref("HSTL_IV", 1500, 900));
    }
    // VREF-based with DCI
    if !edev.grid.kind.is_spartan3ea() {
        res.push(Iostd::vref_dci_od("GTL_DCI", 1200, 800));
        res.push(Iostd::vref_dci_od("GTLP_DCI", 1500, 1000));
        if edev.grid.kind == GridKind::Virtex2 {
            res.push(Iostd::vref_dci(
                "SSTL3_I_DCI",
                3300,
                1500,
                DciKind::InputSplit,
            ));
            res.push(Iostd::vref_dci(
                "SSTL3_II_DCI",
                3300,
                1500,
                DciKind::BiSplit,
            ));
        }
        res.push(Iostd::vref_dci(
            "SSTL2_I_DCI",
            2500,
            1250,
            DciKind::InputSplit,
        ));
        res.push(Iostd::vref_dci(
            "SSTL2_II_DCI",
            2500,
            1250,
            DciKind::BiSplit,
        ));
        res.push(Iostd::vref_dci(
            "SSTL18_I_DCI",
            1800,
            900,
            DciKind::InputSplit,
        ));
        if edev.grid.kind.is_virtex2() {
            res.push(Iostd::vref_dci(
                "SSTL18_II_DCI",
                1800,
                900,
                DciKind::BiSplit,
            ));
        }

        res.push(Iostd::vref_dci(
            "HSTL_I_DCI_18",
            1800,
            900,
            DciKind::InputSplit,
        ));
        res.push(Iostd::vref_dci(
            "HSTL_II_DCI_18",
            1800,
            900,
            DciKind::BiSplit,
        ));
        res.push(Iostd::vref_dci(
            "HSTL_III_DCI_18",
            1800,
            1100,
            DciKind::InputVcc,
        ));
        if edev.grid.kind.is_virtex2() {
            res.push(Iostd::vref_dci(
                "HSTL_IV_DCI_18",
                1800,
                1100,
                DciKind::BiVcc,
            ));
        }
        res.push(Iostd::vref_dci(
            "HSTL_I_DCI",
            1500,
            750,
            DciKind::InputSplit,
        ));
        res.push(Iostd::vref_dci(
            "HSTL_III_DCI",
            1500,
            900,
            DciKind::InputVcc,
        ));
        if edev.grid.kind.is_virtex2() {
            res.push(Iostd::vref_dci("HSTL_II_DCI", 1500, 750, DciKind::BiSplit));
            res.push(Iostd::vref_dci("HSTL_IV_DCI", 1500, 900, DciKind::BiVcc));
        }
    }
    // pseudo-diff
    if edev.grid.kind.is_spartan3a() {
        res.push(Iostd::pseudo_diff("DIFF_SSTL3_I", 3300));
        res.push(Iostd::pseudo_diff("DIFF_SSTL3_II", 3300));
    }
    if edev.grid.kind.is_spartan3ea() {
        res.push(Iostd::pseudo_diff("DIFF_SSTL2_I", 2500));
    }
    if edev.grid.kind != GridKind::Spartan3E && !(edev.grid.kind.is_spartan3a() && !lr) {
        res.push(Iostd::pseudo_diff("DIFF_SSTL2_II", 2500));
    }
    if edev.grid.kind.is_spartan3ea() {
        res.push(Iostd::pseudo_diff("DIFF_SSTL18_I", 1800));
    }
    if edev.grid.kind.is_virtex2() || (edev.grid.kind.is_spartan3a() && lr) {
        res.push(Iostd::pseudo_diff("DIFF_SSTL18_II", 1800));
    }
    if edev.grid.kind.is_spartan3ea() {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_I_18", 1800));
        res.push(Iostd::pseudo_diff("DIFF_HSTL_III_18", 1800));
    }
    if !edev.grid.kind.is_spartan3ea() || (edev.grid.kind.is_spartan3a() && lr) {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_II_18", 1800));
    }
    if edev.grid.kind.is_spartan3a() && lr {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_I", 1500));
        res.push(Iostd::pseudo_diff("DIFF_HSTL_III", 1500));
    }
    if edev.grid.kind.is_virtex2() {
        res.push(Iostd::pseudo_diff("DIFF_HSTL_II", 1500));
    }
    res.push(Iostd {
        name: if edev.grid.kind == GridKind::Virtex2 {
            "LVPECL_33"
        } else {
            "LVPECL_25"
        },
        vcco: Some(2500),
        vref: None,
        diff: DiffKind::Pseudo,
        dci: DciKind::None,
        drive: &[],
        input_only: edev.grid.kind.is_spartan3ea(),
    });
    res.push(Iostd::pseudo_diff("BLVDS_25", 2500));
    // pseudo-diff with DCI
    if !edev.grid.kind.is_spartan3ea() {
        if edev.grid.kind.is_virtex2() {
            res.push(Iostd::pseudo_diff_dci(
                "DIFF_HSTL_II_DCI",
                1500,
                DciKind::BiSplit,
            ));
            res.push(Iostd::pseudo_diff_dci(
                "DIFF_SSTL18_II_DCI",
                1800,
                DciKind::BiSplit,
            ));
        }
        res.push(Iostd::pseudo_diff_dci(
            "DIFF_HSTL_II_DCI_18",
            1800,
            DciKind::BiSplit,
        ));
        res.push(Iostd::pseudo_diff_dci(
            "DIFF_SSTL2_II_DCI",
            2500,
            DciKind::BiSplit,
        ));
    }
    // true diff
    res.push(Iostd::true_diff("LVDS_25", 2500));
    if edev.grid.kind == GridKind::Virtex2 || edev.grid.kind.is_spartan3a() {
        res.push(Iostd::true_diff("LVDS_33", 3300));
    }
    if !edev.grid.kind.is_spartan3ea() {
        res.push(Iostd::true_diff("LVDSEXT_25", 2500));
        res.push(Iostd::true_diff("ULVDS_25", 2500));
        res.push(Iostd::true_diff("LDT_25", 2500));
    }
    if edev.grid.kind == GridKind::Virtex2 {
        res.push(Iostd::true_diff("LVDSEXT_33", 3300));
    }
    if !edev.grid.kind.is_virtex2() {
        res.push(Iostd::true_diff("RSDS_25", 2500));
    }
    if edev.grid.kind.is_spartan3ea() {
        res.push(Iostd::true_diff("MINI_LVDS_25", 2500));
    }
    if edev.grid.kind.is_spartan3a() {
        res.push(Iostd::true_diff("PPDS_25", 2500));
        res.push(Iostd::true_diff("RSDS_33", 3300));
        res.push(Iostd::true_diff("MINI_LVDS_33", 3300));
        res.push(Iostd::true_diff("PPDS_33", 3300));
        res.push(Iostd::true_diff("TMDS_33", 3300));
    }
    if edev.grid.kind.is_virtex2p() {
        res.push(Iostd::true_diff_term("LVDS_25_DT", 2500));
        res.push(Iostd::true_diff_term("LVDSEXT_25_DT", 2500));
        res.push(Iostd::true_diff_term("LDT_25_DT", 2500));
        res.push(Iostd::true_diff_term("ULVDS_25_DT", 2500));
    }
    // true diff with DCI
    if !edev.grid.kind.is_spartan3ea() {
        if edev.grid.kind == GridKind::Virtex2 {
            res.push(Iostd::true_diff_dci("LVDS_33_DCI", 3300));
            res.push(Iostd::true_diff_dci("LVDSEXT_33_DCI", 3300));
        }
        res.push(Iostd::true_diff_dci("LVDS_25_DCI", 2500));
        res.push(Iostd::true_diff_dci("LVDSEXT_25_DCI", 2500));
    }
    res
}

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let ExpandedDevice::Virtex2(edev) = backend.edev else {
        unreachable!()
    };
    let intdb = backend.egrid.db;
    let package = backend
        .device
        .bonds
        .values()
        .max_by_key(|bond| {
            let bdata = &backend.db.bonds[bond.bond];
            let prjcombine_xilinx_geom::Bond::Virtex2(bdata) = bdata else {
                unreachable!();
            };
            bdata.pins.len()
        })
        .unwrap();

    // IOI
    for (node_kind, name, node) in &intdb.nodes {
        if !name.starts_with("IOI") {
            continue;
        }
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        for bel in node.bels.keys() {
            if !bel.starts_with("IOI") {
                continue;
            }
            let ctx = FuzzCtx::new(session, backend, name, bel, TileBits::MainAuto);
            let mode = if edev.grid.kind.is_spartan3ea() {
                "IBUF"
            } else {
                "IOB"
            };

            // clock & SR invs
            fuzz_inv!(ctx, "OTCLK1", [
                (mode mode),
                (attr "OFF1", "#FF")
            ]);
            fuzz_inv!(ctx, "OTCLK2", [
                (mode mode),
                (attr "OFF2", "#FF")
            ]);
            fuzz_inv!(ctx, "ICLK1", [
                (mode mode),
                (attr "IFF1", "#FF")
            ]);
            fuzz_inv!(ctx, "ICLK2", [
                (mode mode),
                (attr "IFF2", "#FF")
            ]);
            fuzz_inv!(ctx, "SR", [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OSR_USED", "0")
            ]);
            fuzz_inv!(ctx, "REV", [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OREV_USED", "0")
            ]);
            // SR & rev enables
            fuzz_enum!(ctx, "ISR_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "OFF1", "#FF"),
                (attr "OSR_USED", "0"),
                (attr "SRINV", "SR_B"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "OSR_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "OFF1", "#FF"),
                (attr "ISR_USED", "0"),
                (attr "SRINV", "SR_B"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "TSR_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "TFF1", "#FF"),
                (attr "ISR_USED", "0"),
                (attr "SRINV", "SR_B"),
                (pin "SR")
            ]);
            fuzz_enum!(ctx, "IREV_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "OFF1", "#FF"),
                (attr "OREV_USED", "0"),
                (attr "REVINV", "REV_B"),
                (pin "REV")
            ]);
            fuzz_enum!(ctx, "OREV_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "OFF1", "#FF"),
                (attr "IREV_USED", "0"),
                (attr "REVINV", "REV_B"),
                (pin "REV")
            ]);
            fuzz_enum!(ctx, "TREV_USED", ["0"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "TFF1", "#FF"),
                (attr "IREV_USED", "0"),
                (attr "REVINV", "REV_B"),
                (pin "REV")
            ]);

            // CE
            fuzz_inv!(ctx, "ICE", [
                (mode mode),
                (attr "IFF1", "#FF")
            ]);
            fuzz_inv!(ctx, "TCE", [
                (mode mode),
                (attr "TFF1", "#FF")
            ]);
            if edev.grid.kind.is_spartan3ea() {
                fuzz_inv!(ctx, "OCE", [
                    (mode mode),
                    (attr "OFF1", "#FF"),
                    (attr "PCICE_MUX", "OCE")
                ]);
                fuzz_enum!(ctx, "PCICE_MUX", ["OCE", "PCICE"], [
                    (mode mode),
                    (attr "OFF1", "#FF"),
                    (attr "OCEINV", "#OFF"),
                    (pin "OCE"),
                    (pin "PCI_CE")
                ]);
            } else {
                fuzz_inv!(ctx, "OCE", [
                    (mode mode),
                    (attr "OFF1", "#FF")
                ]);
            }
            // Output path
            if edev.grid.kind.is_spartan3ea() {
                fuzz_inv!(ctx, "O1", [
                    (mode mode),
                    (attr "O1_DDRMUX", "1"),
                    (attr "OFF1", "#FF"),
                    (attr "OMUX", "OFF1")
                ]);
                fuzz_inv!(ctx, "O2", [
                    (mode mode),
                    (attr "O2_DDRMUX", "1"),
                    (attr "OFF2", "#FF"),
                    (attr "OMUX", "OFF2")
                ]);
            } else {
                fuzz_inv!(ctx, "O1", [
                    (mode mode),
                    (attr "OFF1", "#FF"),
                    (attr "OMUX", "OFF1")
                ]);
                fuzz_inv!(ctx, "O2", [
                    (mode mode),
                    (attr "OFF2", "#FF"),
                    (attr "OMUX", "OFF2")
                ]);
            }
            fuzz_inv!(ctx, "T1", [
                (mode mode),
                (attr "T_USED", "0"),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#OFF"),
                (attr "TMUX", "TFF1"),
                (attr "OFF1", "#OFF"),
                (attr "OFF2", "#OFF"),
                (attr "OMUX", "#OFF"),
                (pin "T")
            ]);
            fuzz_inv!(ctx, "T2", [
                (mode mode),
                (attr "T_USED", "0"),
                (attr "TFF1", "#OFF"),
                (attr "TFF2", "#FF"),
                (attr "TMUX", "TFF2"),
                (attr "OFF1", "#OFF"),
                (attr "OFF2", "#OFF"),
                (attr "OMUX", "#OFF"),
                (pin "T")
            ]);
            fuzz_enum!(ctx, "TMUX", ["T1", "T2", "TFF1", "TFF2", "TFFDDR"], [
                (mode mode),
                (attr "T1INV", "T1"),
                (attr "T2INV", "T2"),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#FF"),
                (attr "T_USED", "0"),
                (attr "OMUX", "#OFF"),
                (attr "IOATTRBOX", "#OFF"),
                (pin "T1"),
                (pin "T2"),
                (pin "T")
            ]);
            // hack to avoid dragging IOB into it.
            for val in ["O1", "O2", "OFF1", "OFF2", "OFFDDR"] {
                if !edev.grid.kind.is_spartan3ea() {
                    fuzz_one!(ctx, "OMUX", val, [
                        (mode mode),
                        (attr "O1INV", "O1"),
                        (attr "O2INV", "O2"),
                        (attr "OFF1", "#FF"),
                        (attr "OFF2", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "TSMUX", "1"),
                        (attr "TMUX", "T1"),
                        (attr "T1INV", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFFDELMUX", "1"),
                        (pin "O1"),
                        (pin "O2"),
                        (pin "T1"),
                        (pin "T"),
                        (pin "I")
                    ], [
                        (attr_diff "OMUX", "OFFDDR", val)
                    ]);
                } else if edev.grid.kind == GridKind::Spartan3E {
                    fuzz_one!(ctx, "OMUX", val, [
                        (mode mode),
                        (attr "O1INV", "O1"),
                        (attr "O2INV", "O2"),
                        (attr "OFF1", "#FF"),
                        (attr "OFF2", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "TSMUX", "1"),
                        (attr "TMUX", "T1"),
                        (attr "T1INV", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1_DDRMUX", "1"),
                        (attr "O2_DDRMUX", "1"),
                        (attr "IDDRIN_MUX", "2"),
                        (pin "O1"),
                        (pin "O2"),
                        (pin "T1"),
                        (pin "T"),
                        (pin "I")
                    ], [
                        (attr_diff "OMUX", "OFFDDR", val)
                    ]);
                    if ctx.bel.to_idx() != 2 {
                        let obel = BelId::from_idx(ctx.bel.to_idx() ^ 1);
                        fuzz_enum!(ctx, "O1_DDRMUX", ["0", "1"], [
                            (mode mode),
                            (bel_unused obel),
                            (attr "OFF1", "#FF"),
                            (attr "OFF2", "#FF"),
                            (attr "OMUX", "OFFDDR"),
                            (attr "TSMUX", "1"),
                            (attr "TFF1", "#FF"),
                            (attr "IFF1", "#FF"),
                            (attr "TMUX", "TFF1"),
                            (attr "IMUX", "0"),
                            (attr "O1INV", "#OFF"),
                            (pin "ODDRIN1"),
                            (pin "I")
                        ]);
                        fuzz_enum!(ctx, "O2_DDRMUX", ["0", "1"], [
                            (mode mode),
                            (bel_unused obel),
                            (attr "OFF1", "#FF"),
                            (attr "OFF2", "#FF"),
                            (attr "OMUX", "OFFDDR"),
                            (attr "TSMUX", "1"),
                            (attr "TFF1", "#FF"),
                            (attr "IFF1", "#FF"),
                            (attr "TMUX", "TFF1"),
                            (attr "IMUX", "0"),
                            (attr "O2INV", "#OFF"),
                            (pin "ODDRIN2"),
                            (pin "I")
                        ]);
                    }
                } else {
                    fuzz_one!(ctx, "OMUX", val, [
                        (mode mode),
                        (attr "O1INV", "O1"),
                        (attr "O2INV", "O2"),
                        (attr "OFF1", "#FF"),
                        (attr "OFF2", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "TSMUX", "1"),
                        (attr "TMUX", "T1"),
                        (attr "T1INV", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "1"),
                        (attr "O1_DDRMUX", "1"),
                        (attr "O2_DDRMUX", "1"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "SEL_MUX", "0"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (pin "O1"),
                        (pin "O2"),
                        (pin "T1"),
                        (pin "T"),
                        (pin "I")
                    ], [
                        (attr_diff "OMUX", "OFFDDR", val)
                    ]);
                    if ctx.bel.to_idx() != 2 {
                        let obel = BelId::from_idx(ctx.bel.to_idx() ^ 1);
                        fuzz_enum!(ctx, "O1_DDRMUX", ["0", "1"], [
                            (mode mode),
                            (bel_unused obel),
                            (attr "OFF1", "#FF"),
                            (attr "OFF2", "#FF"),
                            (attr "OMUX", "OFFDDR"),
                            (attr "TSMUX", "1"),
                            (attr "TFF1", "#FF"),
                            (attr "IFF1", "#FF"),
                            (attr "TMUX", "TFF1"),
                            (attr "IMUX", "0"),
                            (attr "O1INV", "#OFF"),
                            (attr "SEL_MUX", "0"),
                            (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                            (pin "ODDRIN1"),
                            (pin "I")
                        ]);
                        fuzz_enum!(ctx, "O2_DDRMUX", ["0", "1"], [
                            (mode mode),
                            (bel_unused obel),
                            (attr "OFF1", "#FF"),
                            (attr "OFF2", "#FF"),
                            (attr "OMUX", "OFFDDR"),
                            (attr "TSMUX", "1"),
                            (attr "TFF1", "#FF"),
                            (attr "IFF1", "#FF"),
                            (attr "TMUX", "TFF1"),
                            (attr "IMUX", "0"),
                            (attr "O2INV", "#OFF"),
                            (attr "SEL_MUX", "0"),
                            (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                            (pin "ODDRIN2"),
                            (pin "I")
                        ]);
                    }
                }
            }

            // Output flops
            if !edev.grid.kind.is_spartan3ea() {
                fuzz_enum!(ctx, "OFF1", ["#FF", "#LATCH"], [
                    (mode mode),
                    (attr "OFF2", "#OFF"),
                    (attr "OCEINV", "OCE_B"),
                    (attr "OFF1_INIT_ATTR", "INIT1"),
                    (pin "OCE")
                ]);
                fuzz_enum!(ctx, "OFF2", ["#FF", "#LATCH"], [
                    (mode mode),
                    (attr "OFF1", "#OFF"),
                    (attr "OCEINV", "OCE_B"),
                    (attr "OFF2_INIT_ATTR", "INIT1"),
                    (pin "OCE")
                ]);
            } else {
                fuzz_enum!(ctx, "OFF1", ["#FF", "#LATCH"], [
                    (mode mode),
                    (attr "OFF2", "#OFF"),
                    (attr "OCEINV", "OCE_B"),
                    (attr "PCICE_MUX", "OCE"),
                    (attr "OFF1_INIT_ATTR", "INIT1"),
                    (pin "OCE")
                ]);
                fuzz_enum!(ctx, "OFF2", ["#FF", "#LATCH"], [
                    (mode mode),
                    (attr "OFF1", "#OFF"),
                    (attr "OCEINV", "OCE_B"),
                    (attr "PCICE_MUX", "OCE"),
                    (attr "OFF2_INIT_ATTR", "INIT1"),
                    (pin "OCE")
                ]);
            }
            fuzz_enum!(ctx, "TFF1", ["#FF", "#LATCH"], [
                (mode mode),
                (attr "TFF2", "#OFF"),
                (attr "TCEINV", "TCE_B"),
                (attr "TFF1_INIT_ATTR", "INIT1"),
                (pin "TCE")
            ]);
            fuzz_enum!(ctx, "TFF2", ["#FF", "#LATCH"], [
                (mode mode),
                (attr "TFF1", "#OFF"),
                (attr "TCEINV", "TCE_B"),
                (attr "TFF2_INIT_ATTR", "INIT1"),
                (pin "TCE")
            ]);
            fuzz_enum!(ctx, "OFF1_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OFF1_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "OFF2_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "OFF2", "#FF"),
                (attr "OFF2_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "TFF1_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (attr "TFF1_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "TFF2_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "TFF2", "#FF"),
                (attr "TFF2_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "OFF1_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OFF2", "#FF"),
                (attr "OFF1_SR_ATTR", "SRHIGH"),
                (attr "OFF2_SR_ATTR", "SRHIGH"),
                (attr "OFF2_INIT_ATTR", "#OFF")
            ]);
            fuzz_enum!(ctx, "OFF2_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OFF2", "#FF"),
                (attr "OFF1_SR_ATTR", "SRHIGH"),
                (attr "OFF2_SR_ATTR", "SRHIGH"),
                (attr "OFF1_INIT_ATTR", "#OFF")
            ]);
            fuzz_enum!(ctx, "TFF1_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#FF"),
                (attr "TFF1_SR_ATTR", "SRHIGH"),
                (attr "TFF2_SR_ATTR", "SRHIGH"),
                (attr "TFF2_INIT_ATTR", "#OFF")
            ]);
            fuzz_enum!(ctx, "TFF2_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#FF"),
                (attr "TFF1_SR_ATTR", "SRHIGH"),
                (attr "TFF2_SR_ATTR", "SRHIGH"),
                (attr "TFF1_INIT_ATTR", "#OFF")
            ]);
            fuzz_enum!(ctx, "OFFATTRBOX", ["SYNC", "ASYNC"], [
                (mode mode),
                (attr "OFF1", "#FF"),
                (attr "OFF2", "#FF")
            ]);
            fuzz_enum!(ctx, "TFFATTRBOX", ["SYNC", "ASYNC"], [
                (mode mode),
                (attr "TFF1", "#FF"),
                (attr "TFF2", "#FF")
            ]);

            // Input flops
            fuzz_enum!(ctx, "IFF1", ["#FF", "#LATCH"], [
                (mode mode),
                (attr "IFF2", "#OFF"),
                (attr "ICEINV", "ICE_B"),
                (attr "IFF1_INIT_ATTR", "INIT1"),
                (pin "ICE")
            ]);
            fuzz_enum!(ctx, "IFF2", ["#FF", "#LATCH"], [
                (mode mode),
                (attr "IFF1", "#OFF"),
                (attr "ICEINV", "ICE_B"),
                (attr "IFF2_INIT_ATTR", "INIT1"),
                (pin "ICE")
            ]);
            fuzz_enum!(ctx, "IFF1_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "IFF1_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "IFF2_SR_ATTR", ["SRLOW", "SRHIGH"], [
                (mode mode),
                (attr "IFF2", "#FF"),
                (attr "IFF2_INIT_ATTR", "INIT0")
            ]);
            fuzz_enum!(ctx, "IFF1_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "IFF1_SR_ATTR", "SRHIGH")
            ]);
            fuzz_enum!(ctx, "IFF2_INIT_ATTR", ["INIT0", "INIT1"], [
                (mode mode),
                (attr "IFF2", "#FF"),
                (attr "IFF2_SR_ATTR", "SRHIGH")
            ]);
            fuzz_enum!(ctx, "IFFATTRBOX", ["SYNC", "ASYNC"], [
                (mode mode),
                (attr "IFF1", "#FF"),
                (attr "IFF2", "#FF")
            ]);

            // Input path.
            if edev.grid.kind == GridKind::Spartan3E {
                fuzz_enum!(ctx, "IDDRIN_MUX", ["0", "1", "2"], [
                    (mode mode),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "1"),
                    (attr "IFFDMUX", "#OFF"),
                    (pin "IDDRIN1"),
                    (pin "IDDRIN2"),
                    (pin "I")
                ]);
            } else if edev.grid.kind.is_spartan3a() {
                fuzz_enum!(ctx, "IDDRIN_MUX", ["0", "1"], [
                    (mode mode),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "1"),
                    (attr "SEL_MUX", "0"),
                    (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (pin "IDDRIN1"),
                    (pin "IDDRIN2"),
                    (pin "I")
                ]);
                fuzz_one!(ctx, "IDDRIN_MUX", "2", [
                    (mode mode),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "1"),
                    (attr "SEL_MUX", "0"),
                    (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (pin "IDDRIN1"),
                    (pin "IDDRIN2"),
                    (pin "I")
                ], [
                    (attr "IDDRIN_MUX", "2"),
                    (attr "IFFDMUX", "1")
                ]);
            }

            if !edev.grid.kind.is_spartan3a() {
                if edev.grid.kind != GridKind::Spartan3E {
                    fuzz_enum!(ctx, "IDELMUX", ["0", "1"], [
                        (mode mode),
                        (attr "IFFDELMUX", "0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFFDELMUX", ["0", "1"], [
                        (mode mode),
                        (attr "IDELMUX", "0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                        (mode mode),
                        (attr "TSMUX", "1"),
                        (attr "IDELMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "0"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1INV", "O1"),
                        (attr "OMUX", "O1"),
                        (attr "T1INV", "T1"),
                        (attr "TMUX", "T1"),
                        (attr "T_USED", "0"),
                        (pin "O1"),
                        (pin "T1"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFFDMUX", ["0", "1"], [
                        (mode mode),
                        (attr "TSMUX", "1"),
                        (attr "IDELMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1INV", "O1"),
                        (attr "OMUX", "O1"),
                        (attr "T1INV", "T1"),
                        (attr "TMUX", "T1"),
                        (attr "T_USED", "0"),
                        (pin "O1"),
                        (pin "T1"),
                        (pin "I")
                    ]);
                } else {
                    fuzz_enum!(ctx, "IDELMUX", ["0", "1"], [
                        (mode mode),
                        (attr "IFFDELMUX", "0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "IBUF_DELAY_VALUE", "DLY4"),
                        (attr "PRE_DELAY_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFFDELMUX", ["0", "1"], [
                        (mode mode),
                        (attr "IDELMUX", "0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "IBUF_DELAY_VALUE", "DLY4"),
                        (attr "PRE_DELAY_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                        (mode mode),
                        (attr "TSMUX", "1"),
                        (attr "IDELMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IFFDMUX", "0"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1INV", "O1"),
                        (attr "OMUX", "O1"),
                        (attr "T1INV", "T1"),
                        (attr "TMUX", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IDDRIN_MUX", "2"),
                        (pin "O1"),
                        (pin "T1"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFFDMUX", ["0", "1"], [
                        (mode mode),
                        (attr "TSMUX", "1"),
                        (attr "IDELMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IMUX", "0"),
                        (attr "IFFDELMUX", "1"),
                        (attr "O1INV", "O1"),
                        (attr "OMUX", "O1"),
                        (attr "T1INV", "T1"),
                        (attr "TMUX", "T1"),
                        (attr "T_USED", "0"),
                        (attr "IDDRIN_MUX", "2"),
                        (pin "O1"),
                        (pin "T1"),
                        (pin "I")
                    ]);
                }
                fuzz_enum!(ctx, "TSMUX", ["0", "1"], [
                    (mode mode),
                    (attr "IFFDMUX", "1"),
                    (attr "TMUX", "T1"),
                    (attr "T1INV", "T1"),
                    (attr "OMUX", "O1"),
                    (attr "O1INV", "O1"),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "0"),
                    (attr "T_USED", "0"),
                    (pin "T1"),
                    (pin "O1"),
                    (pin "I"),
                    (pin "T")
                ]);
            } else {
                if name.ends_with("T") || name.ends_with("B") {
                    fuzz_enum!(ctx, "IBUF_DELAY_VALUE", ["DLY0", "DLY16"], [
                        (mode mode),
                        (attr "IFD_DELAY_VALUE", "DLY0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFD_DELAY_VALUE", ["DLY0", "DLY8"], [
                        (mode mode),
                        (attr "IBUF_DELAY_VALUE", "DLY0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ]);
                } else {
                    fuzz_enum!(ctx, "IBUF_DELAY_VALUE", [
                        "DLY0",
                        "DLY1",
                        "DLY2",
                        "DLY3",
                        "DLY4",
                        "DLY5",
                        "DLY6",
                        "DLY7",
                        "DLY8",
                        "DLY9",
                        "DLY10",
                        "DLY11",
                        "DLY12",
                        "DLY13",
                        "DLY14",
                        "DLY15",
                        "DLY16",
                    ], [
                        (mode mode),
                        (attr "IFD_DELAY_VALUE", "DLY0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_enum!(ctx, "IFD_DELAY_VALUE", [
                        "DLY0",
                        "DLY1",
                        "DLY2",
                        "DLY3",
                        "DLY4",
                        "DLY5",
                        "DLY6",
                        "DLY7",
                        "DLY8",
                    ], [
                        (mode mode),
                        (attr "IBUF_DELAY_VALUE", "DLY0"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ]);
                    fuzz_one!(ctx, "DELAY_ADJ_ATTRBOX", "VARIABLE", [
                        (mode mode),
                        (attr "IBUF_DELAY_VALUE", "DLY16"),
                        (attr "IFD_DELAY_VALUE", "DLY8"),
                        (attr "IMUX", "1"),
                        (attr "IFFDMUX", "1"),
                        (attr "IFF1", "#FF"),
                        (attr "IDDRIN_MUX", "2"),
                        (attr "SEL_MUX", "0"),
                        (pin "I")
                    ], [
                        (attr_diff "DELAY_ADJ_ATTRBOX", "FIXED", "VARIABLE")
                    ]);
                }
                fuzz_enum!(ctx, "IMUX", ["0", "1"], [
                    (mode mode),
                    (attr "TSMUX", "1"),
                    (attr "IFF1", "#FF"),
                    (attr "IFFDMUX", "0"),
                    (attr "O1INV", "O1"),
                    (attr "OMUX", "O1"),
                    (attr "T1INV", "T1"),
                    (attr "TMUX", "T1"),
                    (attr "T_USED", "0"),
                    (attr "IDDRIN_MUX", "2"),
                    (attr "SEL_MUX", "0"),
                    (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (pin "O1"),
                    (pin "T1"),
                    (pin "I")
                ]);
                fuzz_enum!(ctx, "IFFDMUX", ["0", "1"], [
                    (mode mode),
                    (attr "TSMUX", "1"),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "0"),
                    (attr "O1INV", "O1"),
                    (attr "OMUX", "O1"),
                    (attr "T1INV", "T1"),
                    (attr "TMUX", "T1"),
                    (attr "T_USED", "0"),
                    (attr "IDDRIN_MUX", "2"),
                    (attr "SEL_MUX", "0"),
                    (attr "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (pin "O1"),
                    (pin "T1"),
                    (pin "I")
                ]);
                fuzz_enum!(ctx, "TSMUX", ["0", "1"], [
                    (mode mode),
                    (attr "IFFDMUX", "1"),
                    (attr "TMUX", "T1"),
                    (attr "T1INV", "T1"),
                    (attr "OMUX", "O1"),
                    (attr "O1INV", "O1"),
                    (attr "IFF1", "#FF"),
                    (attr "IMUX", "0"),
                    (attr "T_USED", "0"),
                    (attr "SEL_MUX", "0"),
                    (pin "T1"),
                    (pin "O1"),
                    (pin "I"),
                    (pin "T")
                ]);
            }
            if edev.grid.kind.is_spartan3ea() {
                fuzz_one!(ctx, "MISR_ENABLE", "1", [
                    (bel_special BelKV::NotIbuf),
                    (global_opt "ENABLEMISR", "Y"),
                    (global_opt "MISRRESET", "N"),
                    (no_global_opt "MISRCLOCK"),
                    (mode "IOB"),
                    (attr "PULL", "PULLDOWN"),
                    (attr "TMUX", "#OFF"),
                    (attr "IMUX", "#OFF"),
                    (attr "IFFDMUX", "#OFF"),
                    (attr "OMUX", "O1"),
                    (attr "O1INV", "O1"),
                    (attr "IOATTRBOX", "LVCMOS33"),
                    (attr "DRIVE_0MA", "DRIVE_0MA"),
                    (pin "O1")
                ], [
                    (attr "MISRATTRBOX", "ENABLE_MISR")
                ]);
                if edev.grid.kind.is_spartan3a() {
                    fuzz_one!(ctx, "MISR_ENABLE_OTCLK1", "1", [
                        (bel_special BelKV::NotIbuf),
                        (global_opt "ENABLEMISR", "Y"),
                        (global_opt "MISRRESET", "N"),
                        (mode "IOB"),
                        (attr "PULL", "PULLDOWN"),
                        (attr "TMUX", "#OFF"),
                        (attr "IMUX", "#OFF"),
                        (attr "IFFDMUX", "#OFF"),
                        (attr "OMUX", "O1"),
                        (attr "O1INV", "O1"),
                        (attr "IOATTRBOX", "LVCMOS33"),
                        (attr "DRIVE_0MA", "DRIVE_0MA"),
                        (attr "MISRATTRBOX", "ENABLE_MISR"),
                        (pin "O1")
                    ], [
                        (attr "MISR_CLK_SELECT", "OTCLK1")
                    ]);
                    fuzz_one!(ctx, "MISR_ENABLE_OTCLK2", "1", [
                        (bel_special BelKV::NotIbuf),
                        (global_opt "ENABLEMISR", "Y"),
                        (global_opt "MISRRESET", "N"),
                        (mode "IOB"),
                        (attr "PULL", "PULLDOWN"),
                        (attr "TMUX", "#OFF"),
                        (attr "IMUX", "#OFF"),
                        (attr "IFFDMUX", "#OFF"),
                        (attr "OMUX", "O1"),
                        (attr "O1INV", "O1"),
                        (attr "IOATTRBOX", "LVCMOS33"),
                        (attr "DRIVE_0MA", "DRIVE_0MA"),
                        (attr "MISRATTRBOX", "ENABLE_MISR"),
                        (pin "O1")
                    ], [
                        (attr "MISR_CLK_SELECT", "OTCLK2")
                    ]);
                } else {
                    fuzz_one!(ctx, "MISR_ENABLE_RESET", "1", [
                        (bel_special BelKV::NotIbuf),
                        (global_opt "ENABLEMISR", "Y"),
                        (global_opt "MISRRESET", "Y"),
                        (no_global_opt "MISRCLOCK"),
                        (mode "IOB"),
                        (attr "PULL", "PULLDOWN"),
                        (attr "TMUX", "#OFF"),
                        (attr "IMUX", "#OFF"),
                        (attr "IFFDMUX", "#OFF"),
                        (attr "OMUX", "O1"),
                        (attr "O1INV", "O1"),
                        (attr "IOATTRBOX", "LVCMOS33"),
                        (attr "DRIVE_0MA", "DRIVE_0MA"),
                        (pin "O1")
                    ], [
                        (attr "MISRATTRBOX", "ENABLE_MISR")
                    ]);
                    fuzz_one!(ctx, "MISR_ENABLE_OTCLK1", "1", [
                        (bel_special BelKV::NotIbuf),
                        (global_opt "ENABLEMISR", "Y"),
                        (global_opt "MISRRESET", "N"),
                        (global_opt "MISRCLOCK", "OTCLK1"),
                        (mode "IOB"),
                        (attr "PULL", "PULLDOWN"),
                        (attr "TMUX", "#OFF"),
                        (attr "IMUX", "#OFF"),
                        (attr "IFFDMUX", "#OFF"),
                        (attr "OMUX", "O1"),
                        (attr "O1INV", "O1"),
                        (attr "IOATTRBOX", "LVCMOS33"),
                        (attr "DRIVE_0MA", "DRIVE_0MA"),
                        (pin "O1")
                    ], [
                        (attr "MISRATTRBOX", "ENABLE_MISR")
                    ]);
                    fuzz_one!(ctx, "MISR_ENABLE_OTCLK2", "1", [
                        (bel_special BelKV::NotIbuf),
                        (global_opt "ENABLEMISR", "Y"),
                        (global_opt "MISRRESET", "N"),
                        (global_opt "MISRCLOCK", "OTCLK2"),
                        (mode "IOB"),
                        (attr "PULL", "PULLDOWN"),
                        (attr "TMUX", "#OFF"),
                        (attr "IMUX", "#OFF"),
                        (attr "IFFDMUX", "#OFF"),
                        (attr "OMUX", "O1"),
                        (attr "O1INV", "O1"),
                        (attr "IOATTRBOX", "LVCMOS33"),
                        (attr "DRIVE_0MA", "DRIVE_0MA"),
                        (pin "O1")
                    ], [
                        (attr "MISRATTRBOX", "ENABLE_MISR")
                    ]);
                }
            }
        }
    }

    // IOB
    for (node_kind, name, node_data) in &intdb.nodes {
        let is_s3a_lr = name.starts_with("IOBS.S3A.L") || name.starts_with("IOBS.S3A.R");
        if !name.starts_with("IOB") {
            continue;
        }
        if backend.egrid.node_index[node_kind].is_empty() {
            continue;
        }
        let iobs = iobs_data(name);
        for (i, &iob) in iobs.iter().enumerate() {
            let ctx = FuzzCtx::new_fake_bel(
                session,
                backend,
                name,
                format!("IOB{i}"),
                TileBits::Iob(node_data.tiles.len()),
            );
            let ibuf_mode = if edev.grid.kind.is_spartan3ea() {
                "IBUF"
            } else {
                "IOB"
            };
            let diffi_mode = match iob.diff {
                IobDiff::None => None,
                IobDiff::True(_) => Some(if edev.grid.kind.is_spartan3a() && is_s3a_lr {
                    "DIFFMI_NDT"
                } else if edev.grid.kind.is_spartan3ea() {
                    "DIFFMI"
                } else {
                    "DIFFM"
                }),
                IobDiff::Comp(_) => Some(if edev.grid.kind.is_spartan3a() && is_s3a_lr {
                    "DIFFSI_NDT"
                } else if edev.grid.kind.is_spartan3ea() {
                    "DIFFSI"
                } else {
                    "DIFFS"
                }),
            };
            let iob_mode = if iob.is_ibuf { "IBUF" } else { "IOB" };
            if !iob.is_ibuf {
                fuzz_one!(ctx, "PRESENT", "IOB", [
                    (global_mutex "VREF", "NO"),
                    (global_mutex "DCI", "NO")
                ], [
                    (iob_mode iob, "IOB")
                ]);
            }
            if edev.grid.kind.is_spartan3ea() {
                fuzz_one!(ctx, "PRESENT", "IBUF", [
                    (global_mutex "VREF", "NO")
                ], [
                    (iob_mode iob, "IBUF")
                ]);
            }
            for val in ["PULLUP", "PULLDOWN", "KEEPER"] {
                fuzz_one!(ctx, "PULL", val, [
                    (iob_mode iob, ibuf_mode),
                    (iob_attr iob, "IMUX", "1"),
                    (iob_pin iob, "I")
                ], [
                    (iob_attr iob, "PULL", val)
                ]);
            }
            fuzz_one!(ctx, "GTSATTRBOX", "DISABLE_GTS", [
                (iob_mode iob, ibuf_mode)
            ], [
                (iob_attr iob, "GTSATTRBOX", "DISABLE_GTS")
            ]);
            if edev.grid.kind.is_spartan3a() && !iob.is_ibuf {
                for val in [
                    "DRIVE_LAST_VALUE",
                    "3STATE",
                    "3STATE_PULLUP",
                    "3STATE_PULLDOWN",
                    "3STATE_KEEPER",
                ] {
                    fuzz_one!(ctx, "SUSPEND", val, [
                        (iob_mode iob, "IOB")
                    ], [
                        (iob_attr iob, "SUSPEND", val)
                    ]);
                }
            }
            if edev.grid.kind == GridKind::Spartan3E {
                for val in [
                    "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7", "DLY8", "DLY9",
                    "DLY10", "DLY11", "DLY12", "DLY13", "DLY14", "DLY15", "DLY16",
                ] {
                    fuzz_one!(ctx, "IBUF_DELAY_VALUE", val, [
                        (iob_mode iob, "IBUF"),
                        (iob_attr iob, "IFD_DELAY_VALUE", "DLY0"),
                        (iob_attr iob, "IMUX", "1"),
                        (iob_attr iob, "IFFDMUX", "1"),
                        (iob_attr iob, "IFF1", "#FF"),
                        (iob_attr iob, "IDDRIN_MUX", "2"),
                        (iob_pin iob, "I")
                    ], [
                        (iob_attr_diff iob, "IBUF_DELAY_VALUE", "DLY0", val)
                    ]);
                }
                for val in [
                    "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7", "DLY8",
                ] {
                    fuzz_one!(ctx, "IFD_DELAY_VALUE", val, [
                        (iob_mode iob, "IBUF"),
                        (iob_attr iob, "IBUF_DELAY_VALUE", "DLY0"),
                        (iob_attr iob, "IMUX", "1"),
                        (iob_attr iob, "IFFDMUX", "1"),
                        (iob_attr iob, "IFF1", "#FF"),
                        (iob_attr iob, "IDDRIN_MUX", "2"),
                        (iob_pin iob, "I")
                    ], [
                        (iob_attr_diff iob, "IFD_DELAY_VALUE", "DLY0", val)
                    ]);
                }
            }
            if edev.grid.kind.is_spartan3a() && !is_s3a_lr {
                for val in [
                    "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7", "DLY8", "DLY9",
                    "DLY10", "DLY11", "DLY12", "DLY13", "DLY14", "DLY15", "DLY16",
                ] {
                    fuzz_one!(ctx, "IBUF_DELAY_VALUE", val, [
                        (iob_mode iob, "IBUF"),
                        (iob_attr iob, "IFD_DELAY_VALUE", "DLY0"),
                        (iob_attr iob, "IMUX", "1"),
                        (iob_attr iob, "IFFDMUX", "1"),
                        (iob_attr iob, "IFF1", "#FF"),
                        (iob_attr iob, "IDDRIN_MUX", "2"),
                        (iob_attr iob, "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (iob_attr iob, "SEL_MUX", "0"),
                        (iob_pin iob, "I")
                    ], [
                        (iob_attr_diff iob, "IBUF_DELAY_VALUE", "DLY16", val)
                    ]);
                }
                for val in [
                    "DLY1", "DLY2", "DLY3", "DLY4", "DLY5", "DLY6", "DLY7", "DLY8",
                ] {
                    fuzz_one!(ctx, "IFD_DELAY_VALUE", val, [
                        (iob_mode iob, "IBUF"),
                        (iob_attr iob, "IBUF_DELAY_VALUE", "DLY0"),
                        (iob_attr iob, "IMUX", "1"),
                        (iob_attr iob, "IFFDMUX", "1"),
                        (iob_attr iob, "IFF1", "#FF"),
                        (iob_attr iob, "IDDRIN_MUX", "2"),
                        (iob_attr iob, "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (iob_attr iob, "SEL_MUX", "0"),
                        (iob_pin iob, "I")
                    ], [
                        (iob_attr_diff iob, "IFD_DELAY_VALUE", "DLY8", val)
                    ]);
                }
                fuzz_one!(ctx, "DELAY_ADJ_ATTRBOX", "VARIABLE", [
                    (iob_mode iob, "IBUF"),
                    (iob_attr iob, "IBUF_DELAY_VALUE", "DLY16"),
                    (iob_attr iob, "IFD_DELAY_VALUE", "DLY8"),
                    (iob_attr iob, "IMUX", "1"),
                    (iob_attr iob, "IFFDMUX", "1"),
                    (iob_attr iob, "IFF1", "#FF"),
                    (iob_attr iob, "IDDRIN_MUX", "2"),
                    (iob_attr iob, "SEL_MUX", "0"),
                    (iob_pin iob, "I")
                ], [
                    (iob_attr_diff iob, "DELAY_ADJ_ATTRBOX", "FIXED", "VARIABLE")
                ]);
            }

            // Input path.
            for std in get_iostds(edev, is_s3a_lr) {
                let vccaux_list = if (std.name.starts_with("LVCMOS")
                    || std.name.starts_with("LVTTL"))
                    && edev.grid.kind.is_spartan3a()
                {
                    &["2.5", "3.3"][..]
                } else {
                    &[""][..]
                };
                if std.diff != DiffKind::None && diffi_mode.is_none() {
                    continue;
                }
                let mode = if std.diff == DiffKind::None {
                    ibuf_mode
                } else {
                    diffi_mode.unwrap()
                };
                for &vccaux in vccaux_list {
                    let is_input_dci = matches!(
                        std.dci,
                        DciKind::InputSplit | DciKind::InputVcc | DciKind::BiSplit | DciKind::BiVcc
                    );
                    let special = if std.diff != DiffKind::None {
                        if is_input_dci {
                            // sigh.
                            continue;
                        } else if edev.grid.kind == GridKind::Spartan3E {
                            // I hate ISE.
                            BelKV::OtherIobDiffOutput(std.name.to_string())
                        } else {
                            BelKV::Nop
                        }
                    } else if std.vref.is_some() || is_input_dci {
                        BelKV::OtherIobInput(std.name.to_string())
                    } else {
                        BelKV::Nop
                    };
                    let vref_mutex = if std.vref.is_some() {
                        TileKV::GlobalMutex("VREF".to_string(), "YES".to_string())
                    } else {
                        TileKV::Nop
                    };
                    let dci_mutex = if std.dci == DciKind::None {
                        TileKV::Nop
                    } else {
                        TileKV::GlobalMutex("DCI".to_string(), std.name.to_string())
                    };
                    let attr = match vccaux {
                        "2.5" => "ISTD.2.5",
                        "3.3" => "ISTD.3.3",
                        _ => "ISTD",
                    };
                    if edev.grid.kind.is_spartan3a() {
                        fuzz_one!(ctx, attr, std.name, [
                            (global_mutex "DIFF", "INPUT"),
                            (vccaux vccaux),
                            (iob_attr iob, "OMUX", "#OFF"),
                            (iob_attr iob, "TMUX", "#OFF"),
                            (iob_attr iob, "IFFDMUX", "#OFF"),
                            (iob_attr iob, "PULL", "PULLDOWN"),
                            (package &package.name),
                            (special vref_mutex),
                            (iob_special iob, special.clone())
                        ], [
                            (iob_mode_diff iob, ibuf_mode, mode),
                            (iob_attr iob, "IOATTRBOX", std.name),
                            (iob_attr iob, "IBUF_DELAY_VALUE", "DLY0"),
                            (iob_attr iob, "DELAY_ADJ_ATTRBOX", "FIXED"),
                            (iob_attr iob, "SEL_MUX", "0"),
                            (iob_attr iob, "IMUX", "1"),
                            (iob_pin iob, "I")
                        ]);
                    } else {
                        fuzz_one!(ctx, attr, std.name, [
                            (global_mutex "DIFF", "INPUT"),
                            (iob_attr iob, "OMUX", "#OFF"),
                            (iob_attr iob, "TMUX", "#OFF"),
                            (iob_attr iob, "IFFDMUX", "#OFF"),
                            (iob_attr iob, "PULL", "PULLDOWN"),
                            (package &package.name),
                            (special vref_mutex),
                            (special dci_mutex.clone()),
                            (iob_special iob, special.clone())
                        ], [
                            (iob_mode_diff iob, ibuf_mode, mode),
                            (iob_attr iob, "IOATTRBOX", std.name),
                            (iob_attr iob, "IMUX", "1"),
                            (iob_pin iob, "I")
                        ]);
                    }
                    if std.diff != DiffKind::None {
                        fuzz_one!(ctx, "ISTD.COMP", std.name, [
                            (global_mutex "DIFF", "INPUT"),
                            (iob_attr iob, "OMUX", "#OFF"),
                            (iob_attr iob, "TMUX", "#OFF"),
                            (iob_attr iob, "IMUX", "#OFF"),
                            (iob_attr iob, "IFFDMUX", "#OFF"),
                            (iob_attr iob, "PULL", "#OFF"),
                            (package &package.name),
                            (special dci_mutex),
                            (iob_special iob, special)
                        ], [
                            (iob_mode_diff iob, ibuf_mode, mode),
                            (iob_attr iob, "IOATTRBOX", std.name),
                            (iob_attr iob, "PADOUT_USED", "0"),
                            (iob_pin iob, "PADOUT")
                        ]);
                    }
                }
            }
            if edev.grid.kind.is_spartan3a() {
                fuzz_one!(ctx, "SEL_MUX", "OMUX", [
                    (iob_mode iob, iob_mode),
                    (iob_attr iob, "OMUX", "O1"),
                    (iob_attr iob, "O1INV", "O1"),
                    (iob_attr iob, "O1_DDRMUX", "1"),
                    (iob_attr iob, "TMUX", "T1"),
                    (iob_attr iob, "T1INV", "T1"),
                    (iob_attr iob, "T_USED", "0"),
                    (iob_attr iob, "IFFDMUX", "#OFF"),
                    (iob_attr iob, "PULL", "PULLDOWN"),
                    (iob_attr iob, "IOATTRBOX", "LVCMOS33"),
                    (iob_pin iob, "O1"),
                    (iob_pin iob, "T1"),
                    (iob_pin iob, "T")
                ], [
                    (iob_attr iob, "IBUF_DELAY_VALUE", "DLY0"),
                    (iob_attr iob, "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (iob_attr iob, "SEL_MUX", "1"),
                    (iob_attr iob, "IMUX", "1"),
                    (iob_pin iob, "I")
                ]);
                fuzz_one!(ctx, "SEL_MUX", "TMUX", [
                    (iob_mode iob, iob_mode),
                    (iob_attr iob, "OMUX", "O1"),
                    (iob_attr iob, "O1INV", "O1"),
                    (iob_attr iob, "O1_DDRMUX", "1"),
                    (iob_attr iob, "TMUX", "T1"),
                    (iob_attr iob, "T1INV", "T1"),
                    (iob_attr iob, "T_USED", "0"),
                    (iob_attr iob, "IFFDMUX", "#OFF"),
                    (iob_attr iob, "PULL", "PULLDOWN"),
                    (iob_attr iob, "IOATTRBOX", "LVCMOS33"),
                    (iob_pin iob, "O1"),
                    (iob_pin iob, "T1"),
                    (iob_pin iob, "T")
                ], [
                    (iob_attr iob, "IBUF_DELAY_VALUE", "DLY0"),
                    (iob_attr iob, "DELAY_ADJ_ATTRBOX", "FIXED"),
                    (iob_attr iob, "SEL_MUX", "2"),
                    (iob_attr iob, "IMUX", "1"),
                    (iob_pin iob, "I")
                ]);
            }
            if let Some(pkg) = has_any_vref(edev, backend.device, backend.db, name, i) {
                fuzz_one!(ctx, "PRESENT", "NOTVREF", [
                    (package pkg),
                    (global_mutex "VREF", "YES"),
                    (iob_special iob, BelKV::IsVref),
                    (iob_special iob, BelKV::OtherIobInput("SSTL2_I".to_string()))
                ], [
                    (iob_mode iob, ibuf_mode)
                ]);
            }
            if let Some((pkg, alt)) = has_any_vr(edev, backend.device, backend.db, name, i) {
                let spec = if let Some(alt) = alt {
                    TileKV::AltVr(alt)
                } else {
                    TileKV::Nop
                };
                fuzz_one!(ctx, "PRESENT", "NOTVR", [
                    (package pkg),
                    (global_mutex "DCI", "YES"),
                    (special spec),
                    (iob_special iob, BelKV::IsVr),
                    (iob_special iob, BelKV::OtherIobInput("GTL_DCI".to_string()))
                ], [
                    (iob_mode iob, ibuf_mode)
                ]);
            }
            if edev.grid.kind.is_spartan3ea()
                && !is_s3a_lr
                && iob.diff != IobDiff::None
                && !iob.is_ibuf
            {
                let difft_mode = if edev.grid.kind == GridKind::Spartan3E {
                    match iob.diff {
                        IobDiff::None => unreachable!(),
                        IobDiff::True(_) => "DIFFM",
                        IobDiff::Comp(_) => "DIFFS",
                    }
                } else {
                    diffi_mode.unwrap()
                };
                if edev.grid.kind.is_spartan3a() {
                    fuzz_one!(ctx, "DIFF_TERM", "1", [
                        (global_mutex "DIFF", "TERM"),
                        (iob_mode iob, difft_mode),
                        (iob_attr iob, "OMUX", "#OFF"),
                        (iob_attr iob, "TMUX", "#OFF"),
                        (iob_attr iob, "IFFDMUX", "#OFF"),
                        (iob_attr iob, "PULL", "PULLDOWN"),
                        (iob_attr iob, "IOATTRBOX", "LVDS_25"),
                        (iob_attr iob, "IBUF_DELAY_VALUE", "DLY0"),
                        (iob_attr iob, "DELAY_ADJ_ATTRBOX", "FIXED"),
                        (iob_attr iob, "SEL_MUX", "0"),
                        (iob_attr iob, "IMUX", "1"),
                        (iob_pin iob, "I")
                    ], [
                        (iob_attr_diff iob, "DIFF_TERM", "FALSE", "TRUE")
                    ]);
                } else {
                    fuzz_one!(ctx, "DIFF_TERM", "1", [
                        (global_mutex "DIFF", "TERM"),
                        (iob_mode iob, difft_mode),
                        (iob_attr iob, "OMUX", "#OFF"),
                        (iob_attr iob, "TMUX", "#OFF"),
                        (iob_attr iob, "IFFDMUX", "#OFF"),
                        (iob_attr iob, "PULL", "PULLDOWN"),
                        (iob_attr iob, "IOATTRBOX", "LVDS_25"),
                        (iob_attr iob, "IMUX", "1"),
                        (iob_pin iob, "I")
                    ], [
                        (iob_attr_diff iob, "DIFF_TERM", "FALSE", "TRUE")
                    ]);
                }
                fuzz_one!(ctx, "DIFF_TERM.COMP", "1", [
                    (global_mutex "DIFF", "TERM"),
                    (iob_mode iob, difft_mode),
                    (iob_attr iob, "OMUX", "#OFF"),
                    (iob_attr iob, "TMUX", "#OFF"),
                    (iob_attr iob, "IMUX", "#OFF"),
                    (iob_attr iob, "IFFDMUX", "#OFF"),
                    (iob_attr iob, "PULL", "#OFF"),
                    (iob_attr iob, "IOATTRBOX", "LVDS_25"),
                    (iob_attr iob, "PADOUT_USED", "0"),
                    (iob_pin iob, "PADOUT")
                ], [
                    (iob_attr_diff iob, "DIFF_TERM", "FALSE", "TRUE")
                ]);
            }

            if !iob.is_ibuf {
                // Output path.
                fuzz_one!(ctx, "OUTPUT_ENABLE", "1", [
                    (iob_mode iob, "IOB"),
                    (iob_attr iob, "PULL", "PULLDOWN"),
                    (iob_attr iob, "TMUX", "#OFF"),
                    (iob_attr iob, "IMUX", "#OFF"),
                    (iob_attr iob, "IFFDMUX", "#OFF")
                ], [
                    (iob_attr iob, "IOATTRBOX", "LVCMOS33"),
                    (iob_attr iob, "OMUX", "O1"),
                    (iob_attr iob, "O1INV", "O1"),
                    (iob_attr iob, "DRIVE_0MA", "DRIVE_0MA"),
                    (iob_pin iob, "O1")
                ]);
                for std in get_iostds(edev, is_s3a_lr) {
                    if std.input_only {
                        continue;
                    }
                    if matches!(std.diff, DiffKind::True | DiffKind::TrueTerm) && is_s3a_lr {
                        continue;
                    }
                    match std.diff {
                        DiffKind::None => (),
                        DiffKind::Pseudo => {
                            if iob.diff == IobDiff::None {
                                continue;
                            }
                        }
                        DiffKind::True | DiffKind::TrueTerm => continue,
                    }
                    let (drives, slews) = if std.drive.is_empty() {
                        (&[""][..], &[""][..])
                    } else {
                        (
                            std.drive,
                            if edev.grid.kind.is_spartan3a() {
                                &["FAST", "SLOW", "QUIETIO"][..]
                            } else {
                                &["FAST", "SLOW"][..]
                            },
                        )
                    };
                    let vccauxs = if edev.grid.kind.is_spartan3a()
                        && matches!(std.diff, DiffKind::None | DiffKind::Pseudo)
                    {
                        &["2.5", "3.3"][..]
                    } else {
                        &[""][..]
                    };
                    for &vccaux in vccauxs {
                        for &drive in drives {
                            for &slew in slews {
                                let vccaux_spec = if vccaux.is_empty() {
                                    TileKV::Nop
                                } else {
                                    TileKV::VccAux(vccaux.to_string())
                                };
                                let dci_spec = if std.dci == DciKind::None {
                                    BelKV::Nop
                                } else if matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                                    BelKV::OtherIobInput("SSTL2_I_DCI".into())
                                } else if std.diff == DiffKind::None {
                                    BelKV::OtherIobInput(std.name.to_string())
                                } else {
                                    // can't be bothered to get it working.
                                    continue;
                                };
                                let name = if vccaux.is_empty() {
                                    if drive.is_empty() {
                                        std.name.to_string()
                                    } else {
                                        format!("{s}.{drive}.{slew}", s = std.name)
                                    }
                                } else {
                                    if drive.is_empty() {
                                        format!("{s}.{vccaux}", s = std.name)
                                    } else {
                                        format!("{s}.{drive}.{slew}.{vccaux}", s = std.name)
                                    }
                                };
                                let mode = if std.diff == DiffKind::None {
                                    if is_s3a_lr {
                                        "IOBLR"
                                    } else {
                                        "IOB"
                                    }
                                } else {
                                    match iob.diff {
                                        IobDiff::None => continue,
                                        IobDiff::True(_) => {
                                            if is_s3a_lr {
                                                "DIFFMLR"
                                            } else if edev.grid.kind.is_spartan3a() {
                                                "DIFFMTB"
                                            } else {
                                                "DIFFM"
                                            }
                                        }
                                        IobDiff::Comp(_) => {
                                            if is_s3a_lr {
                                                "DIFFSLR"
                                            } else if edev.grid.kind.is_spartan3a() {
                                                "DIFFSTB"
                                            } else {
                                                "DIFFS"
                                            }
                                        }
                                    }
                                };
                                fuzz_one!(ctx, "OSTD", name, [
                                    (package &package.name),
                                    (special vccaux_spec),
                                    (global_mutex "DCI", "YES"),
                                    (iob_special iob, dci_spec),
                                    (iob_attr iob, "PULL", "PULLDOWN"),
                                    (iob_attr iob, "TMUX", "#OFF"),
                                    (iob_attr iob, "IMUX", "#OFF"),
                                    (iob_attr iob, "IFFDMUX", "#OFF"),
                                    (iob_attr iob, "OMUX", "O1"),
                                    (iob_attr iob, "O1INV", "O1"),
                                    (iob_pin iob, "O1")
                                ], [
                                    (iob_mode_diff iob, "IOB", mode),
                                    (iob_attr_diff iob, "IOATTRBOX", "LVCMOS33", std.name),
                                    (iob_attr_diff iob, "DRIVE_0MA", "DRIVE_0MA", ""),
                                    (iob_attr iob, "DRIVEATTRBOX", drive),
                                    (iob_attr iob, "SLEW", slew),
                                    (iob_attr iob, "SUSPEND", if edev.grid.kind.is_spartan3a() {
                                        "3STATE"
                                    } else {
                                        ""
                                    })
                                ]);
                            }
                        }
                    }
                }
                if let IobDiff::True(other_iob) = iob.diff {
                    let iob_n = iobs[other_iob];
                    for std in get_iostds(edev, is_s3a_lr) {
                        if is_s3a_lr {
                            continue;
                        }
                        if !matches!(std.diff, DiffKind::True) {
                            continue;
                        }
                        let (mode_p, mode_n) = if edev.grid.kind.is_spartan3a() {
                            ("DIFFMTB", "DIFFSTB")
                        } else {
                            ("DIFFM", "DIFFS")
                        };
                        fuzz_one!(ctx, "DIFFO", std.name, [
                            (package &package.name),
                            (global_mutex "DCI", "YES"),
                            (global_mutex "DIFF", "OUTPUT"),
                            (iob_special iob, BelKV::BankDiffOutput(std.name.to_string(), None)),
                            (iob_attr iob, "PULL", "PULLDOWN"),
                            (iob_attr iob_n, "PULL", "PULLDOWN"),
                            (iob_attr iob, "TMUX", "#OFF"),
                            (iob_attr iob, "IMUX", "#OFF"),
                            (iob_attr iob, "IFFDMUX", "#OFF"),
                            (iob_attr iob, "OMUX", "O1"),
                            (iob_attr iob, "O1INV", "O1"),
                            (iob_attr iob_n, "TMUX", "#OFF"),
                            (iob_attr iob_n, "IMUX", "#OFF"),
                            (iob_attr iob_n, "IFFDMUX", "#OFF"),
                            (iob_attr iob_n, "OMUX", "#OFF"),
                            (iob_pin iob, "O1")
                        ], [
                            (iob_mode_diff iob, "IOB", mode_p),
                            (iob_mode_diff iob_n, "IOB", mode_n),
                            (iob_attr_diff iob, "IOATTRBOX", "LVCMOS33", std.name),
                            (iob_attr_diff iob, "DRIVE_0MA", "DRIVE_0MA", ""),
                            (iob_attr iob_n, "IOATTRBOX", std.name),
                            (iob_attr iob_n, "DIFFO_IN_USED", "0"),
                            (iob_pin iob, "DIFFO_OUT"),
                            (iob_pin iob_n, "DIFFO_IN"),
                            (iob_attr iob, "SUSPEND", if edev.grid.kind.is_spartan3a() {
                                "3STATE"
                            } else {
                                ""
                            }),
                            (iob_attr iob_n, "SUSPEND", if edev.grid.kind.is_spartan3a() {
                                "3STATE"
                            } else {
                                ""
                            })
                        ]);
                        if edev.grid.kind.is_spartan3ea() {
                            let altstd = if std.name == "RSDS_25" {
                                "MINI_LVDS_25"
                            } else {
                                "RSDS_25"
                            };
                            fuzz_one!(ctx, "DIFFO.ALT", std.name, [
                                (package &package.name),
                                (global_mutex "DCI", "YES"),
                                (global_mutex "DIFF", "OUTPUT"),
                                (iob_special iob, BelKV::BankDiffOutput(altstd.to_string(), Some(std.name.to_string()))),
                                (iob_attr iob, "PULL", "PULLDOWN"),
                                (iob_attr iob_n, "PULL", "PULLDOWN"),
                                (iob_attr iob, "TMUX", "#OFF"),
                                (iob_attr iob, "IMUX", "#OFF"),
                                (iob_attr iob, "IFFDMUX", "#OFF"),
                                (iob_attr iob, "OMUX", "O1"),
                                (iob_attr iob, "O1INV", "O1"),
                                (iob_attr iob_n, "TMUX", "#OFF"),
                                (iob_attr iob_n, "IMUX", "#OFF"),
                                (iob_attr iob_n, "IFFDMUX", "#OFF"),
                                (iob_attr iob_n, "OMUX", "#OFF"),
                                (iob_pin iob, "O1")
                            ], [
                                (iob_mode_diff iob, "IOB", mode_p),
                                (iob_mode_diff iob_n, "IOB", mode_n),
                                (iob_attr_diff iob, "IOATTRBOX", "LVCMOS33", std.name),
                                (iob_attr_diff iob, "DRIVE_0MA", "DRIVE_0MA", ""),
                                (iob_attr iob_n, "IOATTRBOX", std.name),
                                (iob_attr iob_n, "DIFFO_IN_USED", "0"),
                                (iob_pin iob, "DIFFO_OUT"),
                                (iob_pin iob_n, "DIFFO_IN"),
                                (iob_attr iob, "SUSPEND", if edev.grid.kind.is_spartan3a() {
                                    "3STATE"
                                } else {
                                    ""
                                }),
                                (iob_attr iob_n, "SUSPEND", if edev.grid.kind.is_spartan3a() {
                                    "3STATE"
                                } else {
                                    ""
                                })
                            ]);
                        }
                    }
                }
                if matches!(
                    edev.grid.kind,
                    GridKind::Virtex2P | GridKind::Virtex2PX | GridKind::Spartan3
                ) && !backend.device.name.ends_with("2vp4")
                    && !backend.device.name.ends_with("2vp7")
                {
                    for val in ["ASREQUIRED", "CONTINUOUS", "QUIET"] {
                        fuzz_one!(ctx, "DCIUPDATEMODE", val, [
                            (iob_mode iob, "IOB"),
                            (global_opt "DCIUPDATEMODE", val),
                            (package &package.name),
                            (global_mutex "DCI", "UPDATEMODE"),
                            (iob_special iob, BelKV::OtherIobInput("SSTL2_I_DCI".into())),
                            (iob_attr iob, "PULL", "PULLDOWN"),
                            (iob_attr iob, "TMUX", "#OFF"),
                            (iob_attr iob, "IMUX", "#OFF"),
                            (iob_attr iob, "IFFDMUX", "#OFF"),
                            (iob_attr iob, "OMUX", "O1"),
                            (iob_attr iob, "O1INV", "O1"),
                            (iob_pin iob, "O1")
                        ], [
                            (iob_attr_diff iob, "IOATTRBOX", "LVCMOS33", "LVDCI_33"),
                            (iob_attr_diff iob, "DRIVE_0MA", "DRIVE_0MA", "")
                        ]);
                    }
                }
                if edev.grid.kind.is_spartan3a() {
                    fuzz_multi!(ctx, "OPROGRAMMING", "", 16, [
                        (iob_mode iob, "IOB"),
                        (iob_attr iob, "IMUX", "#OFF"),
                        (iob_attr iob, "IOATTRBOX", "#OFF"),
                        (iob_attr iob, "OMUX", "O1"),
                        (iob_attr iob, "O1INV", "O1"),
                        (iob_pin iob, "O1")
                    ], (iob_attr_bin iob, "OPROGRAMMING"));
                }
            }
            if let Some((brefclk, bufg)) = has_any_brefclk(edev, name, i) {
                let bufg_bel_id = BelId::from_idx(bufg);
                let brefclk_bel_id = BelId::from_idx(10);
                let brefclk_pin = ["BREFCLK", "BREFCLK2"][brefclk];
                fuzz_one!(ctx, "BREFCLK_ENABLE", "1", [], [
                    (related TileRelation::IobBrefclkClkBT,
                        (pip (bel_pin_far bufg_bel_id, "CKI"),
                            (bel_pin brefclk_bel_id, brefclk_pin)))
                ]);
            }
        }
        if name.ends_with("CLK") {
            // Virtex 2 Pro X special!
            let bel_id = BelId::from_idx(4);
            let clk_bel_id = BelId::from_idx(if name == "IOBS.V2P.B.R2.CLK" { 2 } else { 0 });
            let ctx = FuzzCtx::new_fake_bel(
                session,
                backend,
                name,
                "BREFCLK_INT",
                TileBits::Iob(node_data.tiles.len()),
            );
            fuzz_one!(ctx, "ENABLE", "1", [], [
                (related TileRelation::IoiBrefclk, (pip (bel_pin_far clk_bel_id, "I"), (bel_pin bel_id, "BREFCLK")))
            ]);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex2(edev) = ctx.edev else {
        unreachable!()
    };
    let intdb = ctx.edev.egrid().db;

    // IOI
    for (node_kind, tile, node) in &intdb.nodes {
        if !tile.starts_with("IOI") {
            continue;
        }
        if ctx.edev.egrid().node_index[node_kind].is_empty() {
            continue;
        }
        let int_tiles = &[match edev.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => match &tile[..] {
                "IOI.CLK_B" => "INT.IOI.CLK_B",
                "IOI.CLK_T" => "INT.IOI.CLK_T",
                _ => "INT.IOI",
            },
            GridKind::Spartan3 => "INT.IOI.S3",
            GridKind::FpgaCore => unreachable!(),
            GridKind::Spartan3E => "INT.IOI.S3E",
            GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                if tile == "IOI.S3A.LR" {
                    "INT.IOI.S3A.LR"
                } else {
                    "INT.IOI.S3A.TB"
                }
            }
        }];

        for (bel_id, bel, _) in &node.bels {
            if !bel.starts_with("IOI") {
                continue;
            }
            ctx.collect_inv(tile, bel, "OTCLK1");
            ctx.collect_inv(tile, bel, "OTCLK2");
            ctx.collect_inv(tile, bel, "ICLK1");
            ctx.collect_inv(tile, bel, "ICLK2");
            ctx.collect_int_inv(int_tiles, tile, bel, "SR", edev.grid.kind.is_virtex2());
            ctx.collect_int_inv(int_tiles, tile, bel, "OCE", edev.grid.kind.is_virtex2());
            ctx.collect_inv(tile, bel, "REV");
            ctx.collect_inv(tile, bel, "ICE");
            ctx.collect_inv(tile, bel, "TCE");
            let item = ctx.extract_bit(tile, bel, "ISR_USED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_SR_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "OSR_USED", "0");
            ctx.tiledb.insert(tile, bel, "OFF_SR_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "TSR_USED", "0");
            ctx.tiledb.insert(tile, bel, "TFF_SR_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "IREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "IFF_REV_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "OREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "OFF_REV_ENABLE", item);
            let item = ctx.extract_bit(tile, bel, "TREV_USED", "0");
            ctx.tiledb.insert(tile, bel, "TFF_REV_ENABLE", item);

            if edev.grid.kind.is_spartan3ea() {
                ctx.collect_enum_default(tile, bel, "PCICE_MUX", &["OCE", "PCICE"], "NONE");
            }
            ctx.collect_inv(tile, bel, "O1");
            ctx.collect_inv(tile, bel, "O2");
            ctx.collect_inv(tile, bel, "T1");
            ctx.collect_inv(tile, bel, "T2");
            ctx.collect_enum_default(
                tile,
                bel,
                "TMUX",
                &["T1", "T2", "TFF1", "TFF2", "TFFDDR"],
                "NONE",
            );
            // hack to avoid dragging IOB into it.
            let mut item = xlat_enum(vec![
                ("O1", ctx.state.get_diff(tile, bel, "OMUX", "O1")),
                ("O2", ctx.state.get_diff(tile, bel, "OMUX", "O2")),
                ("OFF1", ctx.state.get_diff(tile, bel, "OMUX", "OFF1")),
                ("OFF2", ctx.state.get_diff(tile, bel, "OMUX", "OFF2")),
                ("OFFDDR", ctx.state.get_diff(tile, bel, "OMUX", "OFFDDR")),
            ]);
            let TileItemKind::Enum { ref mut values } = item.kind else {
                unreachable!()
            };
            values.insert("NONE".into(), BitVec::repeat(false, item.bits.len()));
            ctx.tiledb.insert(tile, bel, "OMUX", item);

            let item = ctx.extract_enum_bool(tile, bel, "IFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "IFF_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF1_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "OFF2_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF1_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2", "#FF", "#LATCH");
            ctx.tiledb.insert(tile, bel, "TFF2_LATCH", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "IFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "IFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "OFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "OFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "TFF1_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2_SR_ATTR", "SRLOW", "SRHIGH");
            ctx.tiledb.insert(tile, bel, "TFF2_SRVAL", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "IFF1_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "IFF2_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "OFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF1_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFF2_INIT_ATTR", "INIT0", "INIT1");
            ctx.tiledb.insert(tile, bel, "TFF_INIT", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "IFF_SR_SYNC", item);
            let item = ctx.extract_enum_bool(tile, bel, "OFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "OFF_SR_SYNC", item);
            let item = ctx.extract_enum_bool(tile, bel, "TFFATTRBOX", "ASYNC", "SYNC");
            ctx.tiledb.insert(tile, bel, "TFF_SR_SYNC", item);

            // Input path
            let item = xlat_enum(vec![
                ("GND", ctx.state.get_diff(tile, bel, "TSMUX", "0")),
                ("TMUX", ctx.state.get_diff(tile, bel, "TSMUX", "1")),
            ]);
            ctx.tiledb.insert(tile, bel, "TSBYPASS_MUX", item);

            if !edev.grid.kind.is_spartan3a() {
                let item = ctx.extract_enum_bool(tile, bel, "IDELMUX", "1", "0");
                ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item);
                let item = ctx.extract_enum_bool(tile, bel, "IFFDELMUX", "1", "0");
                ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item);
            } else {
                let item_i = ctx.extract_enum_bool(tile, bel, "IBUF_DELAY_VALUE", "DLY0", "DLY16");
                let item_iff = ctx.extract_enum_bool(tile, bel, "IFD_DELAY_VALUE", "DLY0", "DLY8");
                if tile.ends_with("L") || tile.ends_with("R") {
                    let en_i = Diff::from_bool_item(&item_i);
                    let en_iff = Diff::from_bool_item(&item_iff);
                    let common = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8")
                        .combine(&!&en_i);
                    assert_eq!(
                        common,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4")
                            .combine(&!&en_iff)
                    );
                    // I
                    let b0_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY15")
                        .combine(&!&en_i);
                    let b1_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY14")
                        .combine(&!&en_i);
                    let b2_i = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12")
                        .combine(&!&en_i);
                    for (val, diffs) in [
                        ("DLY13", &[&b0_i, &b1_i][..]),
                        ("DLY11", &[&b0_i, &b2_i][..]),
                        ("DLY10", &[&b1_i, &b2_i][..]),
                        ("DLY9", &[&b0_i, &b1_i, &b2_i][..]),
                        ("DLY7", &[&b0_i, &common][..]),
                        ("DLY6", &[&b1_i, &common][..]),
                        ("DLY5", &[&b0_i, &b1_i, &common][..]),
                        ("DLY4", &[&b2_i, &common][..]),
                        ("DLY3", &[&b0_i, &b2_i, &common][..]),
                        ("DLY2", &[&b1_i, &b2_i, &common][..]),
                        ("DLY1", &[&b0_i, &b1_i, &b2_i, &common][..]),
                    ] {
                        let mut diff = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", val);
                        for &d in diffs {
                            diff = diff.combine(&!d);
                        }
                        diff = diff.combine(&!&en_i);
                        diff.assert_empty();
                    }
                    ctx.tiledb
                        .insert(tile, bel, "I_DELAY", xlat_bitvec(vec![!b0_i, !b1_i, !b2_i]));

                    // IFF
                    let b0_iff = ctx
                        .state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY7")
                        .combine(&!&en_iff);
                    let b1_iff = ctx
                        .state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6")
                        .combine(&!&en_iff);
                    for (val, diffs) in [
                        ("DLY5", &[&b0_iff, &b1_iff][..]),
                        ("DLY3", &[&b0_iff, &common][..]),
                        ("DLY2", &[&b1_iff, &common][..]),
                        ("DLY1", &[&b0_iff, &b1_iff, &common][..]),
                    ] {
                        let mut diff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", val);
                        for &d in diffs {
                            diff = diff.combine(&!d);
                        }
                        diff = diff.combine(&!&en_iff);
                        diff.assert_empty();
                    }
                    ctx.tiledb
                        .insert(tile, bel, "IFF_DELAY", xlat_bitvec(vec![!b0_iff, !b1_iff]));
                    ctx.tiledb
                        .insert(tile, bel, "DELAY_COMMON", xlat_bit(!common));
                    let item = ctx.extract_bit(tile, bel, "DELAY_ADJ_ATTRBOX", "VARIABLE");
                    ctx.tiledb.insert(tile, bel, "DELAY_VARIABLE", item);
                }
                ctx.tiledb.insert(tile, bel, "I_DELAY_ENABLE", item_i);
                ctx.tiledb.insert(tile, bel, "IFF_DELAY_ENABLE", item_iff);
            }
            let item = ctx.extract_enum_bool(tile, bel, "IMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "I_TSBYPASS_ENABLE", item);
            let item = ctx.extract_enum_bool(tile, bel, "IFFDMUX", "1", "0");
            ctx.tiledb.insert(tile, bel, "IFF_TSBYPASS_ENABLE", item);

            if edev.grid.kind.is_spartan3ea() {
                let item = xlat_enum(vec![
                    ("IFFDMUX", ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "2")),
                    ("IDDRIN1", ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "1")),
                    ("IDDRIN2", ctx.state.get_diff(tile, bel, "IDDRIN_MUX", "0")),
                    ("NONE", Diff::default()),
                ]);
                ctx.tiledb.insert(tile, bel, "IDDRIN_MUX", item);
            }
            if edev.grid.kind == GridKind::Spartan3E {
                let en = ctx.state.get_diff(tile, bel, "MISR_ENABLE", "1");
                let en_rst = ctx.state.get_diff(tile, bel, "MISR_ENABLE_RESET", "1");
                let rst = en_rst.combine(&!&en);
                ctx.tiledb.insert(tile, bel, "MISR_RESET", xlat_bit(rst));
                let clk1 = ctx.state.get_diff(tile, bel, "MISR_ENABLE_OTCLK1", "1");
                let clk2 = ctx.state.get_diff(tile, bel, "MISR_ENABLE_OTCLK2", "1");
                assert_eq!(en, clk1);
                let (clk1, clk2, en) = Diff::split(clk1, clk2);
                ctx.tiledb.insert(tile, bel, "MISR_ENABLE", xlat_bit(en));
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "MISR_CLOCK",
                    xlat_enum(vec![
                        ("OTCLK1", clk1),
                        ("OTCLK2", clk2),
                        ("NONE", Diff::default()),
                    ]),
                );
            }
            if edev.grid.kind.is_spartan3a() {
                ctx.collect_bit(tile, bel, "MISR_ENABLE", "1");
                let clk1 = ctx.state.get_diff(tile, bel, "MISR_ENABLE_OTCLK1", "1");
                let clk2 = ctx.state.get_diff(tile, bel, "MISR_ENABLE_OTCLK2", "1");
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "MISR_CLOCK",
                    xlat_enum(vec![
                        ("OTCLK1", clk1),
                        ("OTCLK2", clk2),
                        ("NONE", Diff::default()),
                    ]),
                );
                // Spartan 3A also has the MISRRESET global option, but it affects *all*
                // IOIs in the device, whether they're in use or not, so we cannot easily
                // isolate the diff to a single IOI. The bits are the same as Spartan 3E,
                // so just cheat and inject them manually.
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "MISR_RESET",
                    TileItem {
                        bits: vec![TileBit {
                            tile: 0,
                            frame: 0,
                            bit: [7, 32, 47][bel_id.to_idx()],
                        }],
                        kind: TileItemKind::BitVec {
                            invert: BitVec::from_iter([false]),
                        },
                    },
                )
            }
            // these could be extracted automatically from .ll files but I'm not setting up
            // a while another kind of fuzzer for a handful of bits.
            let bit = if edev.grid.kind.is_virtex2() {
                [
                    TileBit::new(0, 2, 13),
                    TileBit::new(0, 2, 33),
                    TileBit::new(0, 2, 53),
                    TileBit::new(0, 2, 73),
                ][bel_id.to_idx()]
            } else {
                [
                    TileBit::new(0, 3, 0),
                    TileBit::new(0, 3, 39),
                    TileBit::new(0, 3, 40),
                ][bel_id.to_idx()]
            };
            ctx.tiledb
                .insert(tile, bel, "READBACK_I", TileItem::from_bit(bit, false));
        }
        // specials. need cross-bel discard.
        if edev.grid.kind.is_spartan3ea() {
            for bel in ["IOI0", "IOI1"] {
                let obel = if bel == "IOI0" { "IOI1" } else { "IOI0" };
                ctx.state
                    .get_diff(tile, bel, "O1_DDRMUX", "1")
                    .assert_empty();
                ctx.state
                    .get_diff(tile, bel, "O2_DDRMUX", "1")
                    .assert_empty();
                let mut diff = ctx.state.get_diff(tile, bel, "O1_DDRMUX", "0");
                diff.discard_bits(ctx.tiledb.item(tile, obel, "OMUX"));
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "O1_DDRMUX",
                    xlat_enum(vec![("O1", Diff::default()), ("ODDRIN1", diff)]),
                );
                let mut diff = ctx.state.get_diff(tile, bel, "O2_DDRMUX", "0");
                diff.discard_bits(ctx.tiledb.item(tile, obel, "OMUX"));
                ctx.tiledb.insert(
                    tile,
                    bel,
                    "O2_DDRMUX",
                    xlat_enum(vec![("O2", Diff::default()), ("ODDRIN2", diff)]),
                );
            }
        }
    }

    // IOB
    for (node_kind, tile, node_data) in &intdb.nodes {
        if !tile.starts_with("IOB") {
            continue;
        }
        if ctx.edev.egrid().node_index[node_kind].is_empty() {
            continue;
        }
        let iobs = iobs_data(tile);
        let is_s3a_lr = matches!(&tile[..], "IOBS.S3A.L4" | "IOBS.S3A.R4");
        let ioi_tile = match edev.grid.kind {
            GridKind::Virtex2 | GridKind::Virtex2P | GridKind::Virtex2PX => "IOI",
            GridKind::Spartan3 => "IOI.S3",
            GridKind::FpgaCore => unreachable!(),
            GridKind::Spartan3E => "IOI.S3E",
            GridKind::Spartan3A | GridKind::Spartan3ADsp => match &tile[..] {
                "IOBS.S3A.B2" => "IOI.S3A.B",
                "IOBS.S3A.T2" => "IOI.S3A.T",
                "IOBS.S3A.L4" | "IOBS.S3A.R4" => "IOI.S3A.LR",
                _ => unreachable!(),
            },
        };
        for (i, &iob) in iobs.iter().enumerate() {
            let bel = [
                "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
            ][i];
            let ioi_bel = ["IOI0", "IOI1", "IOI2", "IOI3"][iob.bel.to_idx()];
            if edev.grid.kind.is_spartan3ea() {
                ctx.state
                    .get_diff(tile, bel, "GTSATTRBOX", "DISABLE_GTS")
                    .assert_empty();
            } else {
                let item = ctx.extract_bit(tile, bel, "GTSATTRBOX", "DISABLE_GTS");
                ctx.tiledb.insert(tile, bel, "DISABLE_GTS", item);
            }
            ctx.collect_enum_default(tile, bel, "PULL", &["PULLDOWN", "PULLUP", "KEEPER"], "NONE");
            if edev.grid.kind.is_spartan3a() && !iob.is_ibuf {
                ctx.collect_enum(
                    tile,
                    bel,
                    "SUSPEND",
                    &[
                        "DRIVE_LAST_VALUE",
                        "3STATE",
                        "3STATE_PULLUP",
                        "3STATE_PULLDOWN",
                        "3STATE_KEEPER",
                    ],
                );
            }
            if edev.grid.kind == GridKind::Spartan3E {
                if tile.starts_with("IOBS.S3E.R") {
                    for val in ["DLY13", "DLY14", "DLY15", "DLY16"] {
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", val)
                            .assert_empty();
                    }
                    for val in ["DLY7", "DLY8"] {
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", val)
                            .assert_empty();
                    }
                    let common = !ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY3");

                    let iff3_6 = Diff::default();
                    assert_eq!(
                        iff3_6,
                        ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6")
                    );
                    let iff2_5 = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY5");
                    assert_eq!(
                        iff2_5,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY2")
                            .combine(&common)
                    );
                    let iff1_4 = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4");
                    assert_eq!(
                        iff1_4,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY1")
                            .combine(&common)
                    );
                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "IFF_DELAY",
                        xlat_enum(vec![
                            ("SDLY3_LDLY6", iff3_6),
                            ("SDLY2_LDLY5", iff2_5),
                            ("SDLY1_LDLY4", iff1_4),
                        ]),
                    );

                    let i12 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12");
                    let i5_11 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY11");
                    assert_eq!(
                        i5_11,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY5")
                            .combine(&common)
                    );
                    let i4_10 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY10");
                    assert_eq!(
                        i4_10,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY4")
                            .combine(&common)
                    );
                    let i3_9 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY9");
                    assert_eq!(
                        i3_9,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY3")
                            .combine(&common)
                    );
                    let i2_8 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8");
                    assert_eq!(
                        i2_8,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY2")
                            .combine(&common)
                    );
                    let i1_7 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY7");
                    assert_eq!(
                        i1_7,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY1")
                            .combine(&common)
                    );
                    let i6 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY6");

                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "I_DELAY",
                        xlat_enum(vec![
                            ("LDLY12", i12),
                            ("SDLY5_LDLY11", i5_11),
                            ("SDLY4_LDLY10", i4_10),
                            ("SDLY3_LDLY9", i3_9),
                            ("SDLY2_LDLY8", i2_8),
                            ("SDLY1_LDLY7", i1_7),
                            ("LDLY6", i6),
                        ]),
                    );

                    ctx.tiledb
                        .insert(tile, bel, "DELAY_COMMON", xlat_bit(!common));
                } else {
                    for val in ["DLY14", "DLY15", "DLY16"] {
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", val)
                            .assert_empty();
                    }
                    ctx.state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY8")
                        .assert_empty();
                    let common = !ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4");

                    let iff4_7 = Diff::default();
                    assert_eq!(
                        iff4_7,
                        ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY7")
                    );
                    let iff3_6 = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6");
                    assert_eq!(
                        iff3_6,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY3")
                            .combine(&common)
                    );
                    let iff2_5 = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY5");
                    assert_eq!(
                        iff2_5,
                        ctx.state
                            .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY2")
                            .combine(&common)
                    );
                    let iff1 = ctx
                        .state
                        .get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY1")
                        .combine(&common);
                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "IFF_DELAY",
                        xlat_enum(vec![
                            ("SDLY4_LDLY7", iff4_7),
                            ("SDLY3_LDLY6", iff3_6),
                            ("SDLY2_LDLY5", iff2_5),
                            ("SDLY1", iff1),
                        ]),
                    );

                    let i13 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY13");
                    let i7 = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY7")
                        .combine(&common);
                    let i6_12 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12");
                    assert_eq!(
                        i6_12,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY6")
                            .combine(&common)
                    );
                    let i5_11 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY11");
                    assert_eq!(
                        i5_11,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY5")
                            .combine(&common)
                    );
                    let i4_10 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY10");
                    assert_eq!(
                        i4_10,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY4")
                            .combine(&common)
                    );
                    let i3_9 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY9");
                    assert_eq!(
                        i3_9,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY3")
                            .combine(&common)
                    );
                    let i2_8 = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8");
                    assert_eq!(
                        i2_8,
                        ctx.state
                            .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY2")
                            .combine(&common)
                    );
                    let i1 = ctx
                        .state
                        .get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY1")
                        .combine(&common);

                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "I_DELAY",
                        xlat_enum(vec![
                            ("LDLY13", i13),
                            ("SDLY7", i7),
                            ("SDLY6_LDLY12", i6_12),
                            ("SDLY5_LDLY11", i5_11),
                            ("SDLY4_LDLY10", i4_10),
                            ("SDLY3_LDLY9", i3_9),
                            ("SDLY2_LDLY8", i2_8),
                            ("SDLY1", i1),
                        ]),
                    );

                    ctx.tiledb
                        .insert(tile, bel, "DELAY_COMMON", xlat_bit(!common));
                }
            }
            if edev.grid.kind.is_spartan3a()
                && (tile.starts_with("IOBS.S3A.B") || tile.starts_with("IOBS.S3A.T"))
            {
                let common = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY8");
                assert_eq!(
                    common,
                    ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY4")
                );
                // I
                let b0_i = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY15");
                let b1_i = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY14");
                let b2_i = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", "DLY12");
                for (val, diffs) in [
                    ("DLY16", &[][..]),
                    ("DLY13", &[&b0_i, &b1_i][..]),
                    ("DLY11", &[&b0_i, &b2_i][..]),
                    ("DLY10", &[&b1_i, &b2_i][..]),
                    ("DLY9", &[&b0_i, &b1_i, &b2_i][..]),
                    ("DLY7", &[&b0_i, &common][..]),
                    ("DLY6", &[&b1_i, &common][..]),
                    ("DLY5", &[&b0_i, &b1_i, &common][..]),
                    ("DLY4", &[&b2_i, &common][..]),
                    ("DLY3", &[&b0_i, &b2_i, &common][..]),
                    ("DLY2", &[&b1_i, &b2_i, &common][..]),
                    ("DLY1", &[&b0_i, &b1_i, &b2_i, &common][..]),
                ] {
                    let mut diff = ctx.state.get_diff(tile, bel, "IBUF_DELAY_VALUE", val);
                    for &d in diffs {
                        diff = diff.combine(&!d);
                    }
                    diff.assert_empty();
                }
                ctx.tiledb
                    .insert(tile, bel, "I_DELAY", xlat_bitvec(vec![!b0_i, !b1_i, !b2_i]));

                // IFF
                let b0_iff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY7");
                let b1_iff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", "DLY6");
                for (val, diffs) in [
                    ("DLY8", &[][..]),
                    ("DLY5", &[&b0_iff, &b1_iff][..]),
                    ("DLY3", &[&b0_iff, &common][..]),
                    ("DLY2", &[&b1_iff, &common][..]),
                    ("DLY1", &[&b0_iff, &b1_iff, &common][..]),
                ] {
                    let mut diff = ctx.state.get_diff(tile, bel, "IFD_DELAY_VALUE", val);
                    for &d in diffs {
                        diff = diff.combine(&!d);
                    }
                    diff.assert_empty();
                }
                ctx.tiledb
                    .insert(tile, bel, "IFF_DELAY", xlat_bitvec(vec![!b0_iff, !b1_iff]));
                ctx.tiledb
                    .insert(tile, bel, "DELAY_COMMON", xlat_bit(!common));
                let item = ctx.extract_bit(tile, bel, "DELAY_ADJ_ATTRBOX", "VARIABLE");
                ctx.tiledb.insert(tile, bel, "DELAY_VARIABLE", item);
            }
            // Input path.
            if !edev.grid.kind.is_spartan3ea() {
                let mut vals = vec![
                    ("NONE", Diff::default()),
                    (
                        "CMOS",
                        ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS33").clone(),
                    ),
                    (
                        "VREF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "SSTL2_I").clone(),
                    ),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        "DIFF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "BLVDS_25").clone(),
                    ));
                }
                ctx.tiledb.insert(tile, bel, "IBUF_MODE", xlat_enum(vals));
            } else if edev.grid.kind == GridKind::Spartan3E {
                let mut vals = vec![
                    ("NONE", Diff::default()),
                    (
                        "CMOS_LV",
                        ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS18").clone(),
                    ),
                    (
                        "CMOS_HV",
                        ctx.state.peek_diff(tile, bel, "ISTD", "LVCMOS33").clone(),
                    ),
                    (
                        "VREF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "SSTL2_I").clone(),
                    ),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        "DIFF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "BLVDS_25").clone(),
                    ));
                }
                ctx.tiledb.insert(tile, bel, "IBUF_MODE", xlat_enum(vals));
            } else {
                let item = ctx.tiledb.item(ioi_tile, ioi_bel, "OMUX");
                let item = xlat_item_tile_fwd(item.clone(), &[node_data.tiles.len() + iob.tile]);
                let mut omux = ctx.state.get_diff(tile, bel, "SEL_MUX", "OMUX").clone();
                omux.discard_bits(&item);
                let mut vals = vec![
                    ("NONE", Diff::default()),
                    (
                        "CMOS_VCCINT",
                        ctx.state
                            .peek_diff(tile, bel, "ISTD.3.3", "LVCMOS18")
                            .clone(),
                    ),
                    (
                        "CMOS_VCCAUX",
                        ctx.state
                            .peek_diff(tile, bel, "ISTD.2.5", "LVCMOS25")
                            .clone(),
                    ),
                    (
                        "CMOS_VCCO",
                        ctx.state
                            .peek_diff(tile, bel, "ISTD.3.3", "LVCMOS25")
                            .clone(),
                    ),
                    (
                        "VREF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "SSTL2_I").clone(),
                    ),
                    (
                        "TMUX",
                        ctx.state.get_diff(tile, bel, "SEL_MUX", "TMUX").clone(),
                    ),
                    ("OMUX", omux),
                ];
                if iob.diff != IobDiff::None {
                    vals.push((
                        "DIFF",
                        ctx.state.peek_diff(tile, bel, "ISTD", "BLVDS_25").clone(),
                    ));
                }
                ctx.tiledb.insert(tile, bel, "IBUF_MODE", xlat_enum(vals));
            }
            if edev.grid.kind.is_spartan3ea()
                && !is_s3a_lr
                && iob.diff != IobDiff::None
                && !iob.is_ibuf
            {
                ctx.state
                    .get_diff(tile, bel, "DIFF_TERM.COMP", "1")
                    .assert_empty();
                if matches!(iob.diff, IobDiff::Comp(_)) {
                    // ignore
                    ctx.state.get_diff(tile, bel, "DIFF_TERM", "1");
                }
            }
            if has_any_vref(edev, ctx.device, ctx.db, tile, i).is_some() {
                let present_vref = ctx.state.get_diff(tile, bel, "PRESENT", "NOTVREF");
                let present = ctx.state.peek_diff(
                    tile,
                    bel,
                    "PRESENT",
                    if edev.grid.kind.is_spartan3ea() {
                        "IBUF"
                    } else {
                        "IOB"
                    },
                );
                let mut vref = present.combine(&!present_vref);
                vref.discard_bits(ctx.tiledb.item(tile, bel, "PULL"));
                ctx.tiledb.insert(tile, bel, "VREF", xlat_bit(vref));
            }

            // PCI cruft
            if edev.grid.kind.is_spartan3a() {
                let mut ibuf_diff = ctx.state.peek_diff(tile, bel, "ISTD", "PCI33_3").clone();
                ibuf_diff.discard_bits(ctx.tiledb.item(tile, bel, "IBUF_MODE"));
                if iob.is_ibuf {
                    ctx.tiledb
                        .insert(tile, bel, "PCI_INPUT", xlat_bit(ibuf_diff));
                } else {
                    let obuf_diff = ctx
                        .state
                        .peek_diff(tile, bel, "OSTD", "PCI33_3.3.3")
                        .clone();
                    let (ibuf_diff, _, common) = Diff::split(ibuf_diff, obuf_diff);
                    ctx.tiledb
                        .insert(tile, bel, "PCI_INPUT", xlat_bit(ibuf_diff));
                    ctx.tiledb.insert(tile, bel, "PCI_CLAMP", xlat_bit(common));
                }
            }

            // Output path.
            if !iob.is_ibuf {
                let mut diff = ctx.state.get_diff(tile, bel, "OUTPUT_ENABLE", "1");
                for attr in ["OMUX", "TMUX", "INV.T1"] {
                    let item = ctx.tiledb.item(ioi_tile, ioi_bel, attr);
                    let item =
                        xlat_item_tile_fwd(item.clone(), &[node_data.tiles.len() + iob.tile]);
                    diff.discard_bits(&item);
                }
                let mut bits = vec![];
                for (bit, val) in diff.bits {
                    bits.push(bit);
                    assert!(val);
                }
                bits.sort();
                assert_eq!(bits.len(), 2);
                let item = TileItem {
                    bits,
                    kind: TileItemKind::BitVec {
                        invert: BitVec::from_iter([false, false]),
                    },
                };
                ctx.tiledb.insert(tile, bel, "OUTPUT_ENABLE", item);

                // well ...
                let mut slew_bits = HashSet::new();
                let mut drive_bits = HashSet::new();
                for std in get_iostds(edev, is_s3a_lr) {
                    if std.drive.is_empty() {
                        continue;
                    }
                    let vccauxs = if edev.grid.kind.is_spartan3a() {
                        &["2.5", "3.3"][..]
                    } else {
                        &[""]
                    };
                    let slews = if edev.grid.kind.is_spartan3a() {
                        &["FAST", "SLOW", "QUIETIO"][..]
                    } else {
                        &["FAST", "SLOW"]
                    };
                    for vccaux in vccauxs {
                        // grab SLEW bits.
                        for &drive in std.drive {
                            if edev.grid.kind.is_virtex2p()
                                && std.name == "LVCMOS33"
                                && drive == "8"
                            {
                                // ISE bug.
                                continue;
                            }
                            let mut base: Option<Diff> = None;
                            for &slew in slews {
                                let name = if edev.grid.kind.is_spartan3a() {
                                    format!("{s}.{drive}.{slew}.{vccaux}", s = std.name)
                                } else {
                                    format!("{s}.{drive}.{slew}", s = std.name)
                                };
                                let diff = ctx.state.peek_diff(tile, bel, "OSTD", name);
                                if let Some(ref base) = base {
                                    let ddiff = diff.combine(&!base);
                                    for &bit in ddiff.bits.keys() {
                                        slew_bits.insert(bit);
                                    }
                                } else {
                                    base = Some(diff.clone());
                                }
                            }
                        }
                        // grab DRIVE bits.
                        for &slew in slews {
                            let mut base: Option<Diff> = None;
                            for &drive in std.drive {
                                let name = if edev.grid.kind.is_spartan3a() {
                                    format!("{s}.{drive}.{slew}.{vccaux}", s = std.name)
                                } else {
                                    format!("{s}.{drive}.{slew}", s = std.name)
                                };
                                let diff = ctx.state.peek_diff(tile, bel, "OSTD", &name);
                                if let Some(ref base) = base {
                                    let ddiff = diff.combine(&!base);
                                    for &bit in ddiff.bits.keys() {
                                        drive_bits.insert(bit);
                                    }
                                } else {
                                    base = Some(diff.clone());
                                }
                            }
                        }
                    }
                }
                if edev.grid.kind.is_virtex2() {
                    // there is an extra PDRIVE bit on V2 that is not used by any LVCMOS/LVTTL
                    // standards, but is used by some other standards that need extra oomph.
                    // it would be easier to extract this from BLVDS_25, but that requires
                    // a differential pin...
                    let gtl = ctx.state.peek_diff(tile, bel, "OSTD", "GTL");
                    let gtlp = ctx.state.peek_diff(tile, bel, "OSTD", "GTLP");
                    for &bit in gtl.bits.keys() {
                        if !slew_bits.contains(&bit) && !gtlp.bits.contains_key(&bit) {
                            drive_bits.insert(bit);
                        }
                    }
                }
                let mut pdrive_bits = HashSet::new();
                let mut pslew_bits = vec![];
                let mut nslew_bits = vec![];
                if edev.grid.kind.is_spartan3a() {
                    let oprog = xlat_bitvec(ctx.state.get_diffs(tile, bel, "OPROGRAMMING", ""));
                    for i in 13..16 {
                        pdrive_bits.insert(oprog.bits[i]);
                    }
                    for i in 2..6 {
                        nslew_bits.push(oprog.bits[i]);
                    }
                    for i in 6..10 {
                        pslew_bits.push(oprog.bits[i]);
                    }
                } else if edev.grid.kind == GridKind::Spartan3 {
                    pdrive_bits = drive_bits.clone();
                    for &bit in ctx.state.peek_diff(tile, bel, "OSTD", "GTL").bits.keys() {
                        if drive_bits.contains(&bit) {
                            pdrive_bits.remove(&bit);
                        }
                    }
                } else {
                    let drives = if edev.grid.kind == GridKind::Spartan3E {
                        &["2", "4", "6", "8", "12", "16"][..]
                    } else {
                        &["2", "4", "6", "8", "12", "16", "24"][..]
                    };
                    for drive in drives {
                        let ttl =
                            ctx.state
                                .peek_diff(tile, bel, "OSTD", format!("LVTTL.{drive}.SLOW"));
                        let cmos = ctx.state.peek_diff(
                            tile,
                            bel,
                            "OSTD",
                            format!("LVCMOS33.{drive}.SLOW"),
                        );
                        let diff = ttl.combine(&!cmos);
                        for &bit in diff.bits.keys() {
                            pdrive_bits.insert(bit);
                        }
                    }
                }
                let pslew_bit_set = HashSet::from_iter(pslew_bits.iter().copied());
                let nslew_bit_set = HashSet::from_iter(nslew_bits.iter().copied());
                for &bit in &pdrive_bits {
                    assert!(drive_bits.remove(&bit));
                }
                for &bit in &pslew_bits {
                    assert!(slew_bits.remove(&bit));
                }
                for &bit in &nslew_bits {
                    assert!(slew_bits.remove(&bit));
                }
                let ndrive_bits = drive_bits;
                if !edev.grid.kind.is_spartan3ea() {
                    let mut dci_bits = HashSet::new();
                    let item = ctx.tiledb.item(tile, bel, "IBUF_MODE");
                    let mut diff_split = ctx
                        .state
                        .peek_diff(tile, bel, "ISTD", "SSTL2_I_DCI")
                        .clone();
                    diff_split.discard_bits(item);
                    for &bit in diff_split.bits.keys() {
                        dci_bits.insert(bit);
                    }
                    let mut diff_vcc = ctx.state.peek_diff(tile, bel, "ISTD", "GTL_DCI").clone();
                    diff_vcc.discard_bits(item);
                    for &bit in diff_vcc.bits.keys() {
                        dci_bits.insert(bit);
                    }
                    let mut diff_output =
                        ctx.state.peek_diff(tile, bel, "OSTD", "LVDCI_25").clone();
                    let diff_output = diff_output.split_bits(&dci_bits);
                    let mut diff_output_half = ctx
                        .state
                        .peek_diff(tile, bel, "OSTD", "LVDCI_DV2_25")
                        .clone();
                    let diff_output_half = diff_output_half.split_bits(&dci_bits);
                    ctx.tiledb.insert(
                        tile,
                        bel,
                        "DCI_MODE",
                        xlat_enum(vec![
                            ("NONE", Diff::default()),
                            ("OUTPUT", diff_output),
                            ("OUTPUT_HALF", diff_output_half),
                            ("TERM_SPLIT", diff_split),
                            ("TERM_VCC", diff_vcc),
                        ]),
                    );
                }
                let mut vr_slew = None;
                if has_any_vr(edev, ctx.device, ctx.db, tile, i).is_some() {
                    let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "NOTVR");
                    let item = ctx.tiledb.item(tile, bel, "DCI_MODE");
                    diff.apply_enum_diff(item, "NONE", "TERM_SPLIT");
                    diff = !diff;
                    vr_slew = Some(diff.split_bits(&slew_bits));
                    ctx.tiledb.insert(tile, bel, "VR", xlat_bit(diff));
                }
                if matches!(
                    edev.grid.kind,
                    GridKind::Virtex2P | GridKind::Virtex2PX | GridKind::Spartan3
                ) && !ctx.device.name.ends_with("2vp4")
                    && !ctx.device.name.ends_with("2vp7")
                {
                    let diff_a = ctx.state.get_diff(tile, bel, "DCIUPDATEMODE", "ASREQUIRED");
                    let diff_c = ctx.state.get_diff(tile, bel, "DCIUPDATEMODE", "CONTINUOUS");
                    let diff_q = ctx.state.get_diff(tile, bel, "DCIUPDATEMODE", "QUIET");
                    assert_eq!(diff_c, diff_q);
                    let diff = diff_a.combine(&!diff_c);
                    ctx.tiledb
                        .insert(tile, bel, "DCIUPDATEMODE_ASREQUIRED", xlat_bit(diff));
                }
                let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "IOB");
                if edev.grid.kind.is_spartan3ea() {
                    let ibuf_present = ctx.state.get_diff(tile, bel, "PRESENT", "IBUF");
                    assert_eq!(present, ibuf_present);
                }
                present.discard_bits(ctx.tiledb.item(tile, bel, "PULL"));
                for &val in present.bits.values() {
                    assert!(val)
                }

                let mut slew_diffs = vec![];
                let mut pslew_diffs = vec![];
                let mut nslew_diffs = vec![];
                let mut pdrive_diffs = vec![];
                let mut ndrive_diffs = vec![];
                let mut misc_diffs = vec![];
                for std in get_iostds(edev, is_s3a_lr) {
                    if std.input_only {
                        continue;
                    }
                    if matches!(std.diff, DiffKind::True | DiffKind::TrueTerm) {
                        continue;
                    }
                    if std.diff == DiffKind::Pseudo && iob.diff == IobDiff::None {
                        continue;
                    }
                    let vccauxs = if edev.grid.kind.is_spartan3a() {
                        &["2.5", "3.3"][..]
                    } else {
                        &[""][..]
                    };
                    let (drives, slews) = if std.drive.is_empty() {
                        (&[""][..], &[""][..])
                    } else {
                        (
                            std.drive,
                            if edev.grid.kind.is_spartan3a() {
                                &["FAST", "SLOW", "QUIETIO"][..]
                            } else {
                                &["FAST", "SLOW"][..]
                            },
                        )
                    };
                    if std.dci != DciKind::None && std.diff != DiffKind::None {
                        continue;
                    }
                    for &vccaux in vccauxs {
                        for &drive in drives {
                            for &slew in slews {
                                let name = if vccaux.is_empty() {
                                    if drive.is_empty() {
                                        std.name.to_string()
                                    } else {
                                        format!("{s}.{drive}.{slew}", s = std.name)
                                    }
                                } else {
                                    if drive.is_empty() {
                                        format!("{s}.{vccaux}", s = std.name)
                                    } else {
                                        format!("{s}.{drive}.{slew}.{vccaux}", s = std.name)
                                    }
                                };
                                let mut diff = ctx.state.get_diff(tile, bel, "OSTD", &name);
                                if edev.grid.kind.is_virtex2p()
                                    && std.name == "LVCMOS33"
                                    && drive == "8"
                                    && slew == "FAST"
                                {
                                    // ISE bug.
                                    continue;
                                }
                                let slew_diff = diff.split_bits(&slew_bits);
                                let pslew_diff = diff.split_bits(&pslew_bit_set);
                                let nslew_diff = diff.split_bits(&nslew_bit_set);
                                let pdrive_diff = diff.split_bits(&pdrive_bits);
                                let ndrive_diff = diff.split_bits(&ndrive_bits);
                                if !edev.grid.kind.is_spartan3ea() {
                                    let item = ctx.tiledb.item(tile, bel, "DCI_MODE");
                                    match std.dci {
                                        DciKind::Output => {
                                            diff.apply_enum_diff(item, "OUTPUT", "NONE")
                                        }
                                        DciKind::OutputHalf => {
                                            diff.apply_enum_diff(item, "OUTPUT_HALF", "NONE")
                                        }
                                        DciKind::BiVcc => {
                                            diff.apply_enum_diff(item, "TERM_VCC", "NONE")
                                        }
                                        DciKind::BiSplit => {
                                            diff.apply_enum_diff(item, "TERM_SPLIT", "NONE")
                                        }
                                        _ => (),
                                    }
                                }
                                if edev.grid.kind.is_spartan3a() && std.name.starts_with("PCI") {
                                    diff.apply_bit_diff(
                                        ctx.tiledb.item(tile, bel, "PCI_CLAMP"),
                                        true,
                                        false,
                                    );
                                }
                                let stdn = if let Some(x) = std.name.strip_prefix("DIFF_") {
                                    x
                                } else {
                                    std.name
                                };
                                let slew_name = if vccaux.is_empty() {
                                    if slew.is_empty() {
                                        stdn.to_string()
                                    } else {
                                        format!("{stdn}.{slew}")
                                    }
                                } else {
                                    if slew.is_empty() {
                                        format!("{stdn}.{vccaux}")
                                    } else {
                                        format!("{stdn}.{slew}.{vccaux}")
                                    }
                                };
                                let drive_name = if slew.is_empty() {
                                    stdn.to_string()
                                } else {
                                    format!("{stdn}.{drive}")
                                };
                                slew_diffs.push((slew_name.clone(), slew_diff));
                                pslew_diffs.push((slew_name.clone(), pslew_diff));
                                nslew_diffs.push((slew_name, nslew_diff));
                                if matches!(std.dci, DciKind::Output | DciKind::OutputHalf) {
                                    pdrive_diff.assert_empty();
                                    ndrive_diff.assert_empty();
                                } else {
                                    pdrive_diffs.push((drive_name.clone(), pdrive_diff));
                                    ndrive_diffs.push((drive_name, ndrive_diff));
                                }
                                if edev.grid.kind != GridKind::Spartan3E
                                    || !std.name.starts_with("DIFF_")
                                {
                                    misc_diffs.push((stdn.to_string(), diff));
                                }
                            }
                        }
                    }
                }
                if let Some(vr_slew) = vr_slew {
                    slew_diffs.push(("VR".to_string(), vr_slew));
                }
                for &bit in present.bits.keys() {
                    assert!(pdrive_bits.contains(&bit) || ndrive_bits.contains(&bit));
                }
                pdrive_diffs.push(("OFF".to_string(), Diff::default()));
                ndrive_diffs.push(("OFF".to_string(), Diff::default()));
                for (_, diff) in &mut pdrive_diffs {
                    for &bit in present.bits.keys() {
                        if !pdrive_bits.contains(&bit) {
                            continue;
                        }
                        match diff.bits.entry(bit) {
                            hash_map::Entry::Occupied(e) => {
                                assert!(!*e.get());
                                e.remove();
                            }
                            hash_map::Entry::Vacant(e) => {
                                e.insert(false);
                            }
                        }
                    }
                }
                for (_, diff) in &mut ndrive_diffs {
                    for &bit in present.bits.keys() {
                        if !ndrive_bits.contains(&bit) {
                            continue;
                        }
                        match diff.bits.entry(bit) {
                            hash_map::Entry::Occupied(e) => {
                                assert!(!*e.get());
                                e.remove();
                            }
                            hash_map::Entry::Vacant(e) => {
                                e.insert(false);
                            }
                        }
                    }
                }
                let prefix = match edev.grid.kind {
                    GridKind::Virtex2 => "V2",
                    GridKind::Virtex2P | GridKind::Virtex2PX => "V2P",
                    GridKind::Spartan3 => "S3",
                    GridKind::FpgaCore => unreachable!(),
                    GridKind::Spartan3E => "S3E",
                    GridKind::Spartan3A | GridKind::Spartan3ADsp => {
                        if is_s3a_lr {
                            "S3A.LR"
                        } else {
                            "S3A.TB"
                        }
                    }
                };
                for (set_name, diffs) in [
                    ("SLEW", slew_diffs),
                    ("PSLEW", pslew_diffs),
                    ("NSLEW", nslew_diffs),
                    ("PDRIVE", pdrive_diffs),
                    ("NDRIVE", ndrive_diffs),
                    ("OUTPUT_MISC", misc_diffs),
                ] {
                    let ocd = if set_name == "PSLEW" {
                        OcdMode::FixedOrder(&pslew_bits)
                    } else if set_name == "NSLEW" {
                        OcdMode::FixedOrder(&nslew_bits)
                    } else {
                        OcdMode::ValueOrder
                    };
                    let mut item_enum = xlat_enum_ocd(diffs, ocd);
                    if set_name == "PDRIVE" && edev.grid.kind == GridKind::Spartan3E {
                        // needs a little push.
                        enum_ocd_swap_bits(&mut item_enum, 0, 1);
                    }
                    if item_enum.bits.is_empty() {
                        continue;
                    }
                    let invert = BitVec::from_iter(
                        item_enum
                            .bits
                            .iter()
                            .map(|bit| present.bits.contains_key(bit)),
                    );
                    let item_pdrive = TileItem {
                        bits: item_enum.bits,
                        kind: TileItemKind::BitVec { invert },
                    };
                    ctx.tiledb.insert(tile, bel, set_name, item_pdrive);
                    let TileItemKind::Enum { values } = item_enum.kind else {
                        unreachable!()
                    };
                    for (name, value) in values {
                        ctx.tiledb
                            .insert_misc_data(format!("IOSTD:{prefix}:{set_name}:{name}"), value);
                    }
                }

                // True differential output path.
                if let IobDiff::True(other) = iob.diff {
                    let bel_n = [
                        "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
                    ][other];
                    if !is_s3a_lr {
                        let mut group_diff = None;
                        if edev.grid.kind.is_spartan3ea() {
                            let base = ctx.state.peek_diff(tile, bel, "DIFFO", "RSDS_25");
                            let alt = ctx.state.peek_diff(tile, bel, "DIFFO.ALT", "RSDS_25");
                            let diff = alt.combine(&!base);
                            group_diff = Some(diff.clone());
                            let item = xlat_bit_wide(diff);
                            ctx.tiledb.insert(tile, bel, "OUTPUT_DIFF_GROUP", item);
                        }
                        let mut diffs = vec![("OFF", Diff::default())];
                        if edev.grid.kind.is_virtex2p() {
                            diffs.push((
                                "TERM",
                                ctx.state
                                    .peek_diff(tile, bel_n, "ISTD.COMP", "LVDS_25_DT")
                                    .clone(),
                            ));
                        } else if edev.grid.kind.is_spartan3ea() {
                            diffs.push(("TERM", ctx.state.get_diff(tile, bel, "DIFF_TERM", "1")));
                        }

                        for std in get_iostds(edev, is_s3a_lr) {
                            if std.diff != DiffKind::True {
                                continue;
                            }
                            let mut diff = ctx.state.get_diff(tile, bel, "DIFFO", std.name);
                            if edev.grid.kind.is_spartan3ea() {
                                let mut altdiff =
                                    ctx.state.get_diff(tile, bel, "DIFFO.ALT", std.name);
                                if edev.grid.kind == GridKind::Spartan3E && std.name == "LVDS_25" {
                                    assert_eq!(diff, altdiff);
                                    diff = diff.combine(&!group_diff.as_ref().unwrap());
                                } else {
                                    altdiff = altdiff.combine(&!group_diff.as_ref().unwrap());
                                    assert_eq!(diff, altdiff);
                                }
                            }
                            diffs.push((std.name, diff));
                        }
                        let item_enum = xlat_enum(diffs);
                        let l = item_enum.bits.len();
                        let item = TileItem {
                            bits: item_enum.bits,
                            kind: TileItemKind::BitVec {
                                invert: BitVec::repeat(false, l),
                            },
                        };
                        ctx.tiledb.insert(tile, bel, "OUTPUT_DIFF", item);
                        let TileItemKind::Enum { values } = item_enum.kind else {
                            unreachable!()
                        };
                        for (name, value) in values {
                            ctx.tiledb.insert_misc_data(
                                format!("IOSTD:{prefix}:OUTPUT_DIFF:{name}"),
                                value,
                            );
                        }
                    }
                }
            }
            if iob.is_ibuf {
                let mut diff = ctx.state.get_diff(tile, bel, "PRESENT", "IBUF");
                diff.discard_bits(ctx.tiledb.item(tile, bel, "PULL"));
                if edev.grid.kind.is_spartan3a() {
                    diff.assert_empty();
                } else {
                    // ???
                    ctx.tiledb.insert(tile, bel, "ENABLE", xlat_bit(diff));
                }
            }
            if has_any_brefclk(edev, tile, i).is_some() {
                ctx.collect_bit(tile, bel, "BREFCLK_ENABLE", "1");
            }
        }
        // second loop for stuff involving inter-bel dependencies
        for (i, iob) in iobs.into_iter().enumerate() {
            let bel = [
                "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
            ][i];
            for std in get_iostds(edev, is_s3a_lr) {
                if std.diff != DiffKind::None && iob.diff == IobDiff::None {
                    continue;
                }
                if std.diff != DiffKind::None
                    && matches!(
                        std.dci,
                        DciKind::InputSplit | DciKind::InputVcc | DciKind::BiSplit | DciKind::BiVcc
                    )
                {
                    continue;
                }
                let attrs = if (std.name.starts_with("LVCMOS") || std.name.starts_with("LVTTL"))
                    && edev.grid.kind.is_spartan3a()
                {
                    &["ISTD.2.5", "ISTD.3.3"][..]
                } else {
                    &["ISTD"][..]
                };
                for &attr in attrs {
                    let mut diff = ctx.state.get_diff(tile, bel, attr, std.name);
                    if edev.grid.kind.is_spartan3a() && std.name.starts_with("PCI") {
                        diff.apply_bit_diff(ctx.tiledb.item(tile, bel, "PCI_INPUT"), true, false);
                        if !iob.is_ibuf {
                            diff.apply_bit_diff(
                                ctx.tiledb.item(tile, bel, "PCI_CLAMP"),
                                true,
                                false,
                            );
                        }
                    }
                    let ibuf_mode = if std.diff != DiffKind::None {
                        "DIFF"
                    } else if std.vref.is_some() {
                        "VREF"
                    } else if edev.grid.kind.is_spartan3a() {
                        let vcco = std.vcco.unwrap();
                        if vcco < 2500 {
                            "CMOS_VCCINT"
                        } else if std.name.starts_with("PCI")
                            || (std.name == "LVCMOS25" && attr == "ISTD.3.3")
                        {
                            "CMOS_VCCO"
                        } else {
                            "CMOS_VCCAUX"
                        }
                    } else if edev.grid.kind == GridKind::Spartan3E {
                        let vcco = std.vcco.unwrap();
                        if vcco < 2500 {
                            "CMOS_LV"
                        } else {
                            "CMOS_HV"
                        }
                    } else {
                        "CMOS"
                    };
                    diff.apply_enum_diff(
                        ctx.tiledb.item(tile, bel, "IBUF_MODE"),
                        ibuf_mode,
                        "NONE",
                    );
                    let dci_mode = match std.dci {
                        DciKind::None | DciKind::Output | DciKind::OutputHalf => "NONE",
                        DciKind::InputVcc | DciKind::BiVcc => "TERM_VCC",
                        DciKind::InputSplit | DciKind::BiSplit => "TERM_SPLIT",
                        _ => unreachable!(),
                    };
                    if dci_mode != "NONE" {
                        diff.apply_enum_diff(
                            ctx.tiledb.item(tile, bel, "DCI_MODE"),
                            dci_mode,
                            "NONE",
                        );
                    }
                    if edev.grid.kind == GridKind::Spartan3E
                        && std.name == "LVDS_25"
                        && !iob.is_ibuf
                    {
                        let bel_p = if let IobDiff::Comp(other) = iob.diff {
                            [
                                "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
                            ][other]
                        } else {
                            bel
                        };
                        diff.discard_bits(ctx.tiledb.item(tile, bel_p, "OUTPUT_DIFF_GROUP"));
                    }
                    diff.assert_empty();
                }
                if std.diff != DiffKind::None {
                    let mut diff = ctx.state.get_diff(tile, bel, "ISTD.COMP", std.name);
                    if std.diff == DiffKind::TrueTerm {
                        if let IobDiff::Comp(other) = iob.diff {
                            let bel_p = [
                                "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
                            ][other];
                            diff.discard_bits(ctx.tiledb.item(tile, bel_p, "OUTPUT_DIFF"));
                        }
                    }
                    if matches!(edev.grid.kind, GridKind::Spartan3 | GridKind::Spartan3E) {
                        diff.discard_bits(ctx.tiledb.item(tile, bel, "IBUF_MODE"));
                    }
                    if edev.grid.kind == GridKind::Spartan3E
                        && std.name == "LVDS_25"
                        && !iob.is_ibuf
                    {
                        let bel_p = if let IobDiff::Comp(other) = iob.diff {
                            [
                                "IOB0", "IOB1", "IOB2", "IOB3", "IOB4", "IOB5", "IOB6", "IOB7",
                            ][other]
                        } else {
                            bel
                        };
                        diff.discard_bits(ctx.tiledb.item(tile, bel_p, "OUTPUT_DIFF_GROUP"));
                    }
                    diff.assert_empty();
                }
            }
        }
        if tile.ends_with("CLK") {
            let bel = "BREFCLK_INT";
            ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
        }
    }
}
