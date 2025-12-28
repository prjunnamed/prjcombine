#![recursion_limit = "1024"]

use std::{
    collections::{BTreeMap, BTreeSet, btree_map},
    error::Error,
    path::PathBuf,
};

use clap::Parser;
use itertools::Itertools;
use prjcombine_ecp::{
    bond::{BondPad, CfgPad, MipiPad, SerdesPad},
    chip::{Chip, ChipKind, MachXo2Kind, SpecialIoKey},
    db::Device,
    expanded::ExpandedDevice,
};
use prjcombine_entity::{EntityId, EntityVec};
use prjcombine_interconnect::{
    db::{BelInfo, BelSlotId, IntDb, LegacyBel, TileClassId},
    grid::{BelCoord, CellCoord, ColId, WireCoord},
};
use prjcombine_re_lattice_naming::{BelNaming, ChipNaming, Database, WireName};
use prjcombine_re_lattice_rawdump::{Grid, GridId, NodeId};
use prjcombine_types::db::DeviceCombo;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    chip::{ChipExt, make_chip},
    intdb::init_intdb,
    pkg::process_bond,
};

mod archive;
mod chip;
mod clk;
mod config;
mod dsp;
mod ebr;
mod int;
mod intdb;
mod io;
mod maco;
mod pkg;
mod plc;
mod pll;
mod serdes;

struct ChipContext<'a> {
    name: &'a str,
    grid: &'a Grid,
    chip: &'a Chip,
    intdb: &'a IntDb,
    edev: &'a ExpandedDevice<'a>,
    naming: ChipNaming,
    nodes: EntityVec<NodeId, WireName>,
    has_v01b: BTreeSet<CellCoord>,
    int_wires: BTreeMap<WireName, Vec<WireCoord>>,
    io_int_wires: BTreeMap<WireName, Vec<WireCoord>>,
    io_int_names: BTreeMap<WireCoord, WireName>,
    sorted_wires: BTreeMap<(CellCoord, &'static str), BTreeMap<String, WireName>>,
    pclk_cols: EntityVec<ColId, (ColId, ColId)>,
    hsdclk_locs: BTreeMap<WireName, CellCoord>,
    keep_nodes: BTreeSet<WireName>,
    discard_nodes: BTreeSet<WireName>,
    unclaimed_nodes: BTreeSet<WireName>,
    unclaimed_pips: BTreeSet<(WireName, WireName)>,
    pips_fwd: BTreeMap<WireName, BTreeSet<WireName>>,
    pips_bwd: BTreeMap<WireName, BTreeSet<WireName>>,
    bels: BTreeMap<(TileClassId, BelSlotId), BelInfo>,
    dummy_sites: BTreeSet<String>,
    skip_serdes: bool,
    ebr_wires: BTreeMap<WireCoord, WireName>,
}

impl ChipContext<'_> {
    fn rc(&self, cell: CellCoord) -> (u8, u8) {
        let c = (cell.col.to_idx() + 1).try_into().unwrap();
        let r = (self.chip.rows.len() - cell.row.to_idx())
            .try_into()
            .unwrap();
        (r, c)
    }

    fn rc_io(&self, cell: CellCoord) -> (u8, u8) {
        let mut c = (cell.col.to_idx() + 1).try_into().unwrap();
        let mut r = (self.chip.rows.len() - cell.row.to_idx())
            .try_into()
            .unwrap();
        if cell.col == self.chip.col_w() && self.chip.kind != ChipKind::Crosslink {
            c -= 1;
        } else if cell.col == self.chip.col_e() && self.chip.kind != ChipKind::Crosslink {
            c += 1;
        } else if cell.row == self.chip.row_s() {
            r += 1;
        } else if cell.row == self.chip.row_n() {
            r -= 1;
        } else {
            unreachable!()
        }
        (r, c)
    }

    fn rc_io_sn(&self, cell: CellCoord) -> (u8, u8) {
        let c = (cell.col.to_idx() + 1).try_into().unwrap();
        let mut r = (self.chip.rows.len() - cell.row.to_idx())
            .try_into()
            .unwrap();
        if cell.row == self.chip.row_s() {
            r += 1;
        } else if cell.row == self.chip.row_n() {
            r -= 1;
        } else {
            unreachable!()
        }
        (r, c)
    }

    fn rc_corner(&self, cell: CellCoord) -> (u8, u8) {
        let mut c = (cell.col.to_idx() + 1).try_into().unwrap();
        let mut r = (self.chip.rows.len() - cell.row.to_idx())
            .try_into()
            .unwrap();
        if cell.col == self.chip.col_w() {
            c -= 1;
        } else if cell.col == self.chip.col_e() {
            c += 1;
        } else {
            unreachable!()
        }
        if cell.row == self.chip.row_s() {
            r += 1;
        } else if cell.row == self.chip.row_n() {
            r -= 1;
        } else {
            unreachable!()
        }
        (r, c)
    }

    fn rc_wire(&self, cell: CellCoord, suffix: &str) -> WireName {
        let Some(suffix) = self.naming.strings.get(suffix) else {
            panic!("{name}: no suffix {suffix}", name = self.name)
        };
        let (r, c) = self.rc(cell);
        WireName { r, c, suffix }
    }

    fn rc_io_wire(&self, cell: CellCoord, suffix: &str) -> WireName {
        let Some(suffix) = self.naming.strings.get(suffix) else {
            panic!("{name}: no suffix {suffix}", name = self.name)
        };
        let (r, c) = self.rc_io(cell);
        WireName { r, c, suffix }
    }

    fn rc_io_sn_wire(&self, cell: CellCoord, suffix: &str) -> WireName {
        let Some(suffix) = self.naming.strings.get(suffix) else {
            panic!("{name}: no suffix {suffix}", name = self.name)
        };
        let (r, c) = self.rc_io_sn(cell);
        WireName { r, c, suffix }
    }

    fn rc_corner_wire(&self, cell: CellCoord, suffix: &str) -> WireName {
        let Some(suffix) = self.naming.strings.get(suffix) else {
            panic!("{name}: no suffix {suffix}", name = self.name)
        };
        let (r, c) = self.rc_corner(cell);
        WireName { r, c, suffix }
    }

    fn find_single_in(&self, w: WireName) -> WireName {
        let Some(ins) = self.pips_bwd.get(&w) else {
            panic!(
                "{name}: wire {w} has no ins",
                name = self.name,
                w = w.to_string(&self.naming)
            );
        };
        if ins.len() != 1 {
            let ins = ins.iter().map(|&wi| wi.to_string(&self.naming)).join(", ");
            panic!(
                "{name}: wire {w} has many ins: {ins}",
                name = self.name,
                w = w.to_string(&self.naming)
            );
        }
        ins.iter().copied().next().unwrap()
    }

    fn claim_single_in(&mut self, w: WireName) -> WireName {
        let inp = self.find_single_in(w);
        self.claim_pip(w, inp);
        inp
    }

    fn find_single_out(&self, w: WireName) -> WireName {
        let Some(outs) = self.pips_fwd.get(&w) else {
            panic!(
                "{name}: wire {w} has no outs",
                name = self.name,
                w = w.to_string(&self.naming)
            );
        };
        if outs.len() != 1 {
            let outs = outs.iter().map(|&wi| wi.to_string(&self.naming)).join(", ");
            panic!(
                "{name}: wire {w} has many outs: {outs}",
                name = self.name,
                w = w.to_string(&self.naming)
            );
        }
        outs.iter().copied().next().unwrap()
    }

    fn claim_single_out(&mut self, w: WireName) -> WireName {
        let out = self.find_single_out(w);
        self.claim_pip(out, w);
        out
    }

    fn claim_node(&mut self, w: WireName) {
        if !self.unclaimed_nodes.remove(&w) {
            println!(
                "{name}: DOUBLE CLAIMED: {w}",
                name = self.name,
                w = w.to_string(&self.naming)
            );
        }
    }

    fn claim_pip(&mut self, wt: WireName, wf: WireName) {
        if !self.unclaimed_pips.remove(&(wt, wf)) {
            println!(
                "{name}: DOUBLE CLAIMED: {wt} <- {wf}",
                name = self.name,
                wt = wt.to_string(&self.naming),
                wf = wf.to_string(&self.naming)
            );
        }
    }

    fn claim_pip_bi(&mut self, wt: WireName, wf: WireName) {
        if !self.unclaimed_pips.remove(&(wt, wf)) && !self.unclaimed_pips.remove(&(wf, wt)) {
            println!(
                "{name}: DOUBLE CLAIMED: {wt} <-> {wf}",
                name = self.name,
                wt = wt.to_string(&self.naming),
                wf = wf.to_string(&self.naming)
            );
        }
    }

    #[track_caller]
    fn claim_pip_int_out(&mut self, wt: WireCoord, wf: WireName) {
        let Some(&wt) = self.naming.interconnect.get(&wt) else {
            println!(
                "no name for int wire {wt} <- {wf}",
                wt = wt.to_string(self.intdb),
                wf = wf.to_string(&self.naming),
            );
            return;
        };
        self.claim_pip(wt, wf);
    }

    fn claim_pip_int_in(&mut self, wt: WireName, wf: WireCoord) {
        let Some(&wf) = self.naming.interconnect.get(&wf) else {
            println!(
                "no name for int wire {wt} <- {wf}",
                wt = wt.to_string(&self.naming),
                wf = wf.to_string(self.intdb)
            );
            return;
        };
        self.claim_pip(wt, wf);
    }

    fn name_bel<T: AsRef<str>>(&mut self, bel: BelCoord, names: impl IntoIterator<Item = T>) {
        let names = Vec::from_iter(
            names
                .into_iter()
                .map(|x| self.naming.strings.get_or_insert(x.as_ref())),
        );
        assert!(
            self.naming
                .bels
                .insert(
                    bel,
                    BelNaming {
                        names,
                        wires: Default::default(),
                    },
                )
                .is_none()
        );
    }

    fn name_bel_null(&mut self, bel: BelCoord) {
        self.name_bel::<&str>(bel, []);
    }

    fn add_bel_wire(&mut self, bel: BelCoord, pin: impl AsRef<str>, wire: WireName) {
        self.add_bel_wire_no_claim(bel, pin, wire);
        self.claim_node(wire);
    }

    fn add_bel_wire_no_claim(&mut self, bel: BelCoord, pin: impl AsRef<str>, wire: WireName) {
        let pin = self.naming.strings.get_or_insert(pin.as_ref());
        self.naming
            .bels
            .get_mut(&bel)
            .unwrap()
            .wires
            .insert(pin, wire);
    }

    fn sort_bel_wires(&mut self) {
        for &wn in &self.unclaimed_nodes {
            let name = self.naming.strings[wn.suffix].as_str();
            for suffix in [
                "EBR",
                "IOLOGIC",
                "PIO",
                "PICTEST",
                "DQS",
                "SDQS",
                "DQSTEST",
                "DQSDLL",
                "DQSDLLTEST",
                "DLLDEL",
                "DDRDLL",
                "SYSBUS",
                "JTAG",
                "RDBK",
                "OSC",
                "GSR",
                "START",
                "SPIM",
                "SED",
                "TSALL",
                "WAKEUP",
                "SSPICIB",
                "STF",
                "AMBOOT",
                "PERREG",
                "PCNTR",
                "EFB",
                "ESB",
                "PMU",
                "PMUTEST",
                "CFGTEST",
                "TESTCK",
                "RSTN",
                "CCLK",
                "M0",
                "M1",
                "M2",
                "M3",
                "TESTIN",
                "TESTOUT",
                "CENTEST",
                "DTS",
                "PLL",
                "PLL3",
                "DLL",
                "DLLDEL",
                "CLKDIV",
                "SPLL",
                "PCS",
                "ASB",
                "DCU",
                "MIPIDPHY",
                "I2C",
                "NVCMTEST",
                "BCPG",
                "BCINRD",
                "BCLVDSO",
                "BCSLEWRATE",
                "PVTTEST",
                "PVTCAL",
                "DTR",
                "RNET",
                "PROMON1V",
                "PROMON2V",
            ] {
                if let Some(n) = name.strip_suffix(suffix)
                    && let Some(n) = n.strip_suffix('_')
                    && let Some(prefix) = n.strip_prefix('J')
                {
                    let cell = self.chip.xlat_rc_wire(wn);
                    self.sorted_wires
                        .entry((cell, suffix))
                        .or_default()
                        .insert(prefix.to_string(), wn);
                }
            }
        }
    }

    fn insert_bel_generic(&mut self, bcrd: BelCoord, bel: BelInfo) {
        let tcrd = self.edev.get_tile_by_bel(bcrd);
        match self.bels.entry((self.edev[tcrd].class, bcrd.slot)) {
            btree_map::Entry::Vacant(e) => {
                e.insert(bel);
            }
            btree_map::Entry::Occupied(e) => {
                assert_eq!(*e.get(), bel);
            }
        }
    }

    fn insert_bel(&mut self, bcrd: BelCoord, bel: LegacyBel) {
        self.insert_bel_generic(bcrd, BelInfo::Legacy(bel));
    }

    fn extract_simple_bel(
        &mut self,
        bcrd: BelCoord,
        cell: CellCoord,
        suffix: &'static str,
    ) -> LegacyBel {
        let wires = self.sorted_wires[&(cell, suffix)].clone();
        let mut bel = LegacyBel::default();
        for (pin, wire) in wires {
            let Some(bpin) = self.try_xlat_int_wire(bcrd, wire) else {
                continue;
            };
            self.add_bel_wire(bcrd, &pin, wire);
            bel.pins.insert(pin, bpin);
        }
        bel
    }

    fn insert_simple_bel(&mut self, bcrd: BelCoord, cell: CellCoord, suffix: &'static str) {
        let bel = self.extract_simple_bel(bcrd, cell, suffix);
        self.insert_bel(bcrd, bel);
    }

    fn print_unclaimed(&self) {
        for &wn in &self.unclaimed_nodes {
            println!(
                "{name}: UNCLAIMED: {w}",
                name = self.name,
                w = wn.to_string(&self.naming)
            );
        }
        for &(wt, wf) in &self.unclaimed_pips {
            println!(
                "{name}: UNCLAIMED: {wt} <- {wf}",
                name = self.name,
                wt = wt.to_string(&self.naming),
                wf = wf.to_string(&self.naming)
            );
        }
    }
}

struct ChipResult {
    chip: Chip,
    naming: ChipNaming,
    bels: BTreeMap<(TileClassId, BelSlotId), BelInfo>,
    dummy_sites: BTreeSet<String>,
}

fn init_nodes(grid: &Grid) -> (ChipNaming, EntityVec<NodeId, WireName>) {
    let mut naming = ChipNaming::default();
    let mut nodes = EntityVec::new();
    for (id, name, _) in &grid.nodes {
        let (rc, suffix) = name.split_once('_').unwrap();
        let rc = rc.strip_prefix('R').unwrap();
        let (r, c) = rc.split_once('C').unwrap();
        let r = r.parse().unwrap();
        let c = c.parse().unwrap();
        let suffix = naming.strings.get_or_insert(suffix);
        let nid = nodes.push(WireName { r, c, suffix });
        assert_eq!(nid, id);
    }
    // let mut all_names = Vec::from_iter(naming.strings.values());
    // all_names.sort();
    // for s in all_names {
    //     println!("\t{s}");
    // }
    (naming, nodes)
}

fn process_chip(name: &str, grid: &Grid, kind: ChipKind, intdb: &IntDb) -> ChipResult {
    let (naming, nodes) = init_nodes(grid);
    let chip = make_chip(name, grid, kind, &naming, &nodes);
    let edev = chip.expand_grid(intdb);
    let mut ctx = ChipContext {
        name,
        grid,
        chip: &chip,
        intdb,
        edev: &edev,
        naming,
        nodes,
        has_v01b: Default::default(),
        int_wires: Default::default(),
        io_int_wires: Default::default(),
        io_int_names: Default::default(),
        sorted_wires: Default::default(),
        pclk_cols: Default::default(),
        hsdclk_locs: Default::default(),
        discard_nodes: Default::default(),
        keep_nodes: Default::default(),
        unclaimed_nodes: Default::default(),
        unclaimed_pips: Default::default(),
        pips_fwd: Default::default(),
        pips_bwd: Default::default(),
        bels: Default::default(),
        dummy_sites: Default::default(),
        skip_serdes: false,
        ebr_wires: Default::default(),
    };
    ctx.process_int();
    ctx.sort_bel_wires();
    ctx.process_plc();
    ctx.process_io();
    ctx.process_maco();
    ctx.process_ebr();
    ctx.process_dsp();
    ctx.process_pll();
    ctx.process_serdes();
    ctx.process_config();
    ctx.process_clk();
    ctx.print_unclaimed();
    println!("{name}: DONE");
    let naming = ctx.naming;
    let bels = ctx.bels;
    let dummy_sites = ctx.dummy_sites;
    ChipResult {
        chip,
        naming,
        bels,
        dummy_sites,
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "lrd2geom",
    about = "Extract geometry information from Lattice rawdumps."
)]
struct Args {
    dst: PathBuf,
    rawdump: PathBuf,
    datadir: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let rawdb = prjcombine_re_lattice_rawdump::Db::from_file(args.rawdump)?;
    println!("processing start...");
    let kind = match rawdb.family.as_str() {
        "scm" => ChipKind::Scm,
        "ecp" => ChipKind::Ecp,
        "xp" => ChipKind::Xp,
        "machxo" => ChipKind::MachXo,
        "ecp2" => ChipKind::Ecp2,
        "ecp2m" => ChipKind::Ecp2M,
        "xp2" => ChipKind::Xp2,
        "ecp3" => ChipKind::Ecp3,
        "machxo2" => ChipKind::MachXo2(MachXo2Kind::MachXo2),
        "ecp4" => ChipKind::Ecp4,
        "ecp5" => ChipKind::Ecp5,
        "crosslink" => ChipKind::Crosslink,
        _ => panic!("unknown family {}", rawdb.family),
    };
    let mut int = init_intdb(kind);
    let mut devices = vec![];
    let mut chips = EntityVec::new();
    let mut bonds = EntityVec::new();
    let pgrids: Vec<_> = Vec::from_iter(rawdb.grids.iter())
        .into_par_iter()
        .map(|(gid, grid)| {
            let part = rawdb.parts.iter().find(|part| part.grid == gid).unwrap();
            let name = format!("{}-{}", part.name, part.package);
            process_chip(&name, grid, kind, &int)
        })
        .collect();
    let pgrids = EntityVec::<GridId, _>::from_iter(pgrids);
    let mut missing_bels = BTreeSet::new();
    for (tcid, _, tcls) in &int.tile_classes {
        for (bid, bel) in &tcls.bels {
            if let BelInfo::Legacy(bel) = bel
                && !bel.pins.is_empty()
            {
                continue;
            }
            if let BelInfo::SwitchBox(sb) = bel
                && !sb.items.is_empty()
            {
                continue;
            }
            missing_bels.insert((tcid, bid));
        }
    }
    for (gid, cres) in pgrids {
        let mut chip = cres.chip;
        let naming = cres.naming;
        let mut cur_devs: BTreeMap<String, Device> = BTreeMap::new();
        let chip_id = chips.next_id();
        for ((tcid, bid), bel) in cres.bels {
            if missing_bels.remove(&(tcid, bid)) {
                int.tile_classes[tcid].bels.insert(bid, bel);
            } else {
                assert_eq!(
                    int[tcid].bels[bid],
                    bel,
                    "MERGE FAILED {chip_id} {tc} {bel}",
                    tc = int.tile_classes.key(tcid),
                    bel = int.bel_slots.key(bid),
                );
            }
        }
        for part in &rawdb.parts {
            if part.grid != gid {
                continue;
            }
            let pname = format!("{}-{}", part.name, part.package);
            let bres = process_bond(&args.datadir, part, &chip, &naming);
            let mut expected_sites = BTreeSet::from_iter(
                naming
                    .bels
                    .values()
                    .flat_map(|bnaming| bnaming.names.iter())
                    .map(|&x| naming.strings[x].clone()),
            );
            for site in &cres.dummy_sites {
                expected_sites.insert(site.clone());
            }
            let has_jtag_pin_bels =
                matches!(chip.kind, ChipKind::Scm | ChipKind::Ecp3 | ChipKind::Ecp3A);
            for (pin, &pad) in &bres.bond.pins {
                match pad {
                    BondPad::Io(io)
                    | BondPad::IoAsc(io, _)
                    | BondPad::IoPfr(io, _)
                    | BondPad::IoCdone(io) => {
                        if chip.kind == ChipKind::MachXo
                            && io == chip.special_io[&SpecialIoKey::SleepN]
                        {
                            continue;
                        }
                        let bcrd = chip.get_io_loc(io);
                        let Some(bnaming) = naming.bels.get(&bcrd) else {
                            continue;
                        };
                        let name = &naming.strings[bnaming.names[0]];
                        assert!(expected_sites.remove(name));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Serdes(edge, col, spad) => {
                        if matches!(chip.kind, ChipKind::Scm | ChipKind::Ecp4) {
                            // apparently nothing?
                        } else if chip.kind == ChipKind::Ecp5 {
                            let bcrd = chip.bel_serdes(edge, col);
                            let Some(bnaming) = naming.bels.get(&bcrd) else {
                                continue;
                            };
                            let idx = match spad {
                                SerdesPad::ClkP => 2,
                                SerdesPad::ClkN => 3,
                                SerdesPad::InP(0) => 4,
                                SerdesPad::InN(0) => 5,
                                SerdesPad::InP(1) => 6,
                                SerdesPad::InN(1) => 7,
                                SerdesPad::OutP(0) => 8,
                                SerdesPad::OutN(0) => 9,
                                SerdesPad::OutP(1) => 10,
                                SerdesPad::OutN(1) => 11,
                                _ => continue,
                            };
                            let name = &naming.strings[bnaming.names[idx]];
                            assert!(expected_sites.remove(name));
                            expected_sites.insert(pin.clone());
                        } else {
                            let bcrd = chip.bel_serdes(edge, col);
                            let Some(bnaming) = naming.bels.get(&bcrd) else {
                                continue;
                            };
                            let idx = match spad {
                                SerdesPad::ClkP => 1,
                                SerdesPad::ClkN => 2,
                                SerdesPad::InP(0) => 3,
                                SerdesPad::InN(0) => 4,
                                SerdesPad::InP(1) => 5,
                                SerdesPad::InN(1) => 6,
                                SerdesPad::InP(2) => 7,
                                SerdesPad::InN(2) => 8,
                                SerdesPad::InP(3) => 9,
                                SerdesPad::InN(3) => 10,
                                SerdesPad::OutP(0) => 11,
                                SerdesPad::OutN(0) => 12,
                                SerdesPad::OutP(1) => 13,
                                SerdesPad::OutN(1) => 14,
                                SerdesPad::OutP(2) => 15,
                                SerdesPad::OutN(2) => 16,
                                SerdesPad::OutP(3) => 17,
                                SerdesPad::OutN(3) => 18,
                                _ => continue,
                            };
                            let name = &naming.strings[bnaming.names[idx]];
                            assert!(expected_sites.remove(name));
                            expected_sites.insert(pin.clone());
                        }
                    }
                    BondPad::Mipi(col, mpad) => {
                        let bcrd = chip.bel_mipi(col);
                        let Some(bnaming) = naming.bels.get(&bcrd) else {
                            continue;
                        };
                        let idx = match mpad {
                            MipiPad::ClkP => 1,
                            MipiPad::ClkN => 2,
                            MipiPad::DataP(0) => 3,
                            MipiPad::DataN(0) => 4,
                            MipiPad::DataP(1) => 5,
                            MipiPad::DataN(1) => 6,
                            MipiPad::DataP(2) => 7,
                            MipiPad::DataN(2) => 8,
                            MipiPad::DataP(3) => 9,
                            MipiPad::DataN(3) => 10,
                            _ => continue,
                        };
                        let name = &naming.strings[bnaming.names[idx]];
                        assert!(expected_sites.remove(name));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Cfg(CfgPad::Tck) if has_jtag_pin_bels => {
                        assert!(expected_sites.remove("TCK"));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Cfg(CfgPad::Tms) if has_jtag_pin_bels => {
                        assert!(expected_sites.remove("TMS"));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Cfg(CfgPad::Tdi) if has_jtag_pin_bels => {
                        assert!(expected_sites.remove("TDI"));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Cfg(CfgPad::Tdo) if has_jtag_pin_bels => {
                        assert!(expected_sites.remove("TDO"));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Cfg(CfgPad::Cclk) if chip.kind == ChipKind::Scm => {
                        assert!(expected_sites.remove("CCLK"));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Cfg(CfgPad::RdCfgB) if chip.kind == ChipKind::Scm => {
                        assert!(expected_sites.remove("RDCFGN"));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Cfg(CfgPad::ResetB) if chip.kind == ChipKind::Scm => {
                        assert!(expected_sites.remove("RESETN"));
                        expected_sites.insert(pin.clone());
                    }
                    BondPad::Cfg(CfgPad::MpiIrqB) if chip.kind == ChipKind::Scm => {
                        assert!(expected_sites.remove("MPIIRQN"));
                        expected_sites.insert(pin.clone());
                    }
                    _ => (),
                }
            }
            if part.name == "LFMNX-50" && part.package == "FBG484" {
                expected_sites.remove("NXBOOT_MCSN");
                expected_sites.insert("NXBOOTMCSN".into());
                expected_sites.remove("NX_JTAGEN");
                expected_sites.insert("NXJTAGEN".into());
                expected_sites.remove("NX_PROGRAMN");
                expected_sites.insert("NXPROGRAMN".into());
            }
            for site in &part.sites {
                let name = &site.name;
                if !expected_sites.remove(name) {
                    println!("{pname}: missing site {name}");
                }
            }
            for name in expected_sites {
                println!("{pname}: UHHHH site {name} doesn't exist");
            }
            let bond_id = 'bond: {
                for (bid, bond) in &bonds {
                    if *bond == bres.bond {
                        break 'bond bid;
                    }
                }
                bonds.push(bres.bond)
            };
            for (key, io) in bres.special_io {
                match chip.special_io.entry(key) {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(io);
                    }
                    btree_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), io, "mismatch on {key}");
                    }
                }
            }
            let device = cur_devs.entry(part.name.clone()).or_insert_with(|| Device {
                name: part.name.clone(),
                chip: chip_id,
                bonds: Default::default(),
                speeds: Default::default(),
                combos: Default::default(),
            });
            let (dbid, prev) = device.bonds.insert(part.package.clone(), bond_id);
            assert_eq!(prev, None);
            for speed in &part.speeds {
                let dsid = device.speeds.insert(speed.clone()).0;
                device.combos.push(DeviceCombo {
                    devbond: dbid,
                    speed: dsid,
                });
            }
        }
        assert_eq!(chip_id, chips.push((chip, naming)));
        devices.extend(cur_devs.into_values());
    }
    for (tcid, bid) in missing_bels {
        println!(
            "MISSING BEL {tc} {bel}",
            tc = int.tile_classes.key(tcid),
            bel = int.bel_slots.key(bid),
        );
    }
    let db = Database {
        chips,
        bonds,
        devices,
        int,
    };
    db.to_file(args.dst)?;
    Ok(())
}
