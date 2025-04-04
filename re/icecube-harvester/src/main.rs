use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, btree_map},
    mem::ManuallyDrop,
    path::{Path, PathBuf},
    sync::Mutex,
};

use clap::Parser;
use collect::{collect, collect_iob};
use generate::{GeneratorConfig, generate};
use intdb::{MiscNodeBuilder, make_intdb};
use jzon::JsonValue;
use parts::Part;
use pkg::get_pkg_pins;
use prims::{Primitive, get_prims};
use prjcombine_interconnect::{
    db::{
        BelInfo, BelPin, IntDb, MuxInfo, MuxKind, NodeKind, NodeKindId, NodeTileId, NodeWireId,
        PinDir,
    },
    dir::{DirH, DirV},
    grid::{ColId, DieId, EdgeIoCoord, IntWire, RowId, TileIobId},
};
use prjcombine_re_harvester::Harvester;
use prjcombine_siliconblue::{
    bels,
    bond::{Bond, BondPin, CfgPin},
    chip::{Chip, ChipKind, ExtraNode, ExtraNodeIo, ExtraNodeLoc, SharedCfgPin},
    db::Database,
    expanded::BitOwner,
};
use prjcombine_types::tiledb::{TileBit, TileItemKind};
use rayon::prelude::*;
use run::{Design, InstPin, RawLoc, RunResult, run};
use sample::{get_golden_mux_stats, make_sample, wanted_keys_global, wanted_keys_tiled};
use sites::{
    BelPins, SiteInfo, find_bel_pins, find_io_latch_locs, find_sites_iox3, find_sites_misc,
    find_sites_plb,
};
use unnamed_entity::{EntityId, EntityVec};

mod collect;
mod generate;
mod intdb;
mod parts;
mod pkg;
mod prims;
mod run;
mod sample;
mod sites;
mod xlat;

#[derive(Parser)]
struct Args {
    sbt: PathBuf,
    parts: Vec<String>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

#[allow(clippy::type_complexity)]
struct PartContext<'a> {
    part: &'static Part,
    chip: Chip,
    intdb: IntDb,
    sbt: &'a Path,
    prims: BTreeMap<&'static str, Primitive>,
    empty_runs: BTreeMap<&'static str, RunResult>,
    plb_info: Vec<SiteInfo>,
    bel_info: BTreeMap<&'static str, Vec<SiteInfo>>,
    pkg_bel_info: BTreeMap<(&'static str, &'static str), Vec<SiteInfo>>,
    xlat_col: BTreeMap<&'static str, Vec<ColId>>,
    xlat_row: BTreeMap<&'static str, Vec<RowId>>,
    xlat_io: BTreeMap<&'static str, BTreeMap<(u32, u32, u32), EdgeIoCoord>>,
    bonds: BTreeMap<&'static str, Bond>,
    extra_wire_names: BTreeMap<(u32, u32, String), IntWire>,
    bel_pins: BTreeMap<(&'static str, RawLoc), BelPins>,
    extra_node_locs: BTreeMap<ExtraNodeLoc, Vec<RawLoc>>,
    debug: u8,
}

impl PartContext<'_> {
    fn fill_sites(&mut self) {
        let sbt = self.sbt;
        let part = self.part;
        let mut plb_info = None;
        let plb_info_ref = &mut plb_info;
        let bel_info = Mutex::new(BTreeMap::new());
        let bel_info_ref = &bel_info;
        let pkg_bel_info = Mutex::new(BTreeMap::new());
        let pkg_bel_info_ref = &pkg_bel_info;
        let prims_ref = &self.prims;
        let empty_runs = Mutex::new(BTreeMap::new());
        let empty_runs_ref = &empty_runs;
        rayon::scope(|s| {
            s.spawn(move |_| {
                let locs = find_sites_plb(sbt, part);
                *plb_info_ref = Some(locs);
            });
            for kind in [
                "SB_GB",
                "SB_RAM4K",
                "SB_RAM40_4K",
                "SB_RAM40_16K",
                "SB_MAC16",
                "SB_SPRAM256KA",
                "SB_WARMBOOT",
                "SB_SPI",
                "SB_I2C",
                "SB_FILTER_50NS",
                "SB_HSOSC",
                "SB_LSOSC",
                "SB_HFOSC",
                "SB_LFOSC",
                "SB_LEDD_IP",
                "SB_LEDDA_IP",
                "SB_IR_IP",
                "SB_I2C_FIFO",
            ] {
                if !self.prims.contains_key(kind) {
                    continue;
                }
                s.spawn(move |_| {
                    let locs = find_sites_misc(sbt, prims_ref, part, part.packages[0], kind);
                    let mut binfo = bel_info_ref.lock().unwrap();
                    binfo.insert(kind, locs);
                });
            }
            for &pkg in part.packages {
                s.spawn(move |_| {
                    let design = Design {
                        kind: part.kind,
                        device: part.name,
                        package: pkg,
                        speed: part.speeds[0],
                        temp: part.temps[0],
                        insts: Default::default(),
                        keep_tmp: false,
                        opts: vec![],
                        props: Default::default(),
                    };
                    empty_runs_ref
                        .lock()
                        .unwrap()
                        .insert(pkg, run(sbt, &design).unwrap());
                });

                for kind in [
                    "SB_IO",
                    "SB_IO_DS",
                    "SB_IO_DLY",
                    "SB_IO_I3C",
                    "SB_IO_OD",
                    "SB_GB_IO",
                    "SB_IR_DRV",
                    "SB_BARCODE_DRV",
                    "SB_IR400_DRV",
                    "SB_IR500_DRV",
                    "SB_RGB_DRV",
                    "SB_RGBA_DRV",
                    "SB_PLL_CORE",
                    "SB_PLL_PAD",
                    "SB_PLL_2_PAD",
                    "SB_PLL40_CORE",
                    "SB_PLL40_PAD",
                    "SB_PLL40_PAD_DS",
                    "SB_PLL40_2_PAD",
                    "SB_PLL40_2F_CORE",
                    "SB_PLL40_2F_PAD",
                    "SB_PLL40_2F_PAD_DS",
                    "SB_MIPI_RX_2LANE",
                    "SB_MIPI_TX_4LANE",
                    "SB_TMDS_deserializer",
                ] {
                    if !self.prims.contains_key(kind) {
                        continue;
                    }
                    if kind.starts_with("SB_PLL") && part.kind == ChipKind::Ice40MX {
                        continue;
                    }
                    s.spawn(move |_| {
                        let locs = find_sites_misc(sbt, prims_ref, part, pkg, kind);
                        let mut binfo = pkg_bel_info_ref.lock().unwrap();
                        binfo.insert((pkg, kind), locs);
                    });
                }
                s.spawn(move |_| {
                    let locs = find_sites_iox3(sbt, part, pkg);
                    let mut binfo = pkg_bel_info_ref.lock().unwrap();
                    binfo.insert((pkg, "IOx3"), locs);
                });
            }
        });

        self.empty_runs = empty_runs.into_inner().unwrap();
        self.plb_info = plb_info.unwrap();
        self.bel_info = bel_info.into_inner().unwrap();
        self.pkg_bel_info = pkg_bel_info.into_inner().unwrap();
        self.chip.row_mid = RowId::from_idx(
            self.empty_runs[self.part.packages[0]].bitstream.cram[0]
                .frame_present
                .len()
                / 16,
        )
    }

    fn fill_xlat_rc(&mut self) {
        let mut xlat_col = BTreeMap::new();
        let mut xlat_row = BTreeMap::new();
        let mut info_sets = vec![
            (&self.plb_info, InstPin::Simple("Q".into()), true, false),
            (
                &self.bel_info["SB_GB"],
                InstPin::Simple("USER_SIGNAL_TO_GLOBAL_BUFFER".into()),
                true,
                false,
            ),
            (
                &self.bel_info[if self.part.kind.is_ice65() {
                    "SB_RAM4K"
                } else {
                    "SB_RAM40_4K"
                }],
                InstPin::Indexed("RDATA".into(), 0),
                false,
                true,
            ),
        ];
        if self.bel_info.contains_key("SB_MAC16") {
            info_sets.push((
                &self.bel_info["SB_MAC16"],
                InstPin::Indexed("O".into(), 0),
                false,
                false,
            ));
        }
        if self.bel_info.contains_key("SB_LSOSC") {
            info_sets.push((
                &self.bel_info["SB_LSOSC"],
                InstPin::Simple("CLKK".into()),
                false,
                false,
            ));
            info_sets.push((
                &self.bel_info["SB_HSOSC"],
                InstPin::Simple("CLKM".into()),
                false,
                false,
            ));
        }

        for (infos, pin, do_y, is_ram) in info_sets {
            for info in infos {
                let (col, row, _) = *info
                    .out_wires
                    .get(&pin)
                    .unwrap_or_else(|| &info.in_wires[&pin]);
                let col = ColId::from_idx(col.try_into().unwrap());
                match xlat_col.entry(info.loc.x) {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(col);
                    }
                    btree_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), col);
                    }
                }
                if is_ram {
                    self.chip.cols_bram.insert(col);
                }
                if do_y {
                    let row = RowId::from_idx(row.try_into().unwrap());
                    match xlat_row.entry(info.loc.y) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(row);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), row);
                        }
                    }
                }
            }
        }
        self.chip.columns = xlat_col.values().max().unwrap().to_idx() + 1;
        self.chip.rows = xlat_row.values().max().unwrap().to_idx() + 1;

        // iCE5LP1K fixup.
        if !self.chip.kind.has_io_we() {
            for &pkg in self.part.packages {
                for site in &self.pkg_bel_info[&(pkg, "SB_IO")] {
                    let (col, _, _) = site.in_wires[&InstPin::Simple("D_OUT_0".into())];
                    self.chip.columns = self.chip.columns.max(col as usize + 2);
                }
            }
        }

        for (i, (&j, _)) in xlat_col.iter().enumerate() {
            assert_eq!(i, usize::try_from(j).unwrap());
        }
        for (i, (&j, _)) in xlat_row.iter().enumerate() {
            assert_eq!(i, usize::try_from(j).unwrap());
        }
        let xlat_col: Vec<_> = xlat_col.into_values().collect();
        let xlat_row: Vec<_> = xlat_row.into_values().collect();

        for &pkg in self.part.packages {
            if self.part.name == "iCE40LP640" && pkg == "SWG16TR" {
                self.xlat_col
                    .insert(pkg, Vec::from_iter((0..14).map(ColId::from_idx)));
                self.xlat_row
                    .insert(pkg, Vec::from_iter((0..18).map(RowId::from_idx)));
            } else {
                self.xlat_col.insert(pkg, xlat_col.clone());
                self.xlat_row.insert(pkg, xlat_row.clone());
            }
        }
    }

    fn fill_bonds(&mut self) {
        let col_w = ColId::from_idx(0);
        let col_e = ColId::from_idx(self.chip.columns - 1);
        let row_s = RowId::from_idx(0);
        let row_n = RowId::from_idx(self.chip.rows - 1);
        for &pkg in self.part.packages {
            let mut xlat_io = BTreeMap::new();

            let mut bond = Bond {
                pins: Default::default(),
            };
            for info in &self.pkg_bel_info[&(pkg, "SB_IO")] {
                let (col, row, ref wn) = info.in_wires[&InstPin::Simple("D_OUT_0".into())];
                let col = ColId::from_idx(col.try_into().unwrap());
                let row = RowId::from_idx(row.try_into().unwrap());
                let slot = bels::IO[if wn == "wire_io_cluster/io_0/D_OUT_0" {
                    0
                } else if wn == "wire_io_cluster/io_1/D_OUT_0" {
                    1
                } else {
                    panic!("ummm {wn}?")
                }];
                let (loc, ref pin) = info.pads["PACKAGE_PIN"];
                let xy = (loc.x, loc.y, loc.bel);
                assert_eq!(loc, info.loc);
                let io = self.chip.get_io_crd((DieId::from_idx(0), (col, row), slot));
                // will be fixed up later.
                self.chip.io_iob.insert(io, io);
                assert_eq!(bond.pins.insert(pin.clone(), BondPin::Io(io)), None);
                match xlat_io.entry(xy) {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(io);
                    }
                    btree_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), io);
                    }
                }
            }
            if self.part.kind.is_ice65() {
                for info in &self.pkg_bel_info[&(pkg, "SB_IO_DS")] {
                    for pin in ["PACKAGE_PIN", "PACKAGE_PIN_B"] {
                        let (loc, ref pin) = info.pads[pin];
                        let col = ColId::from_idx(loc.x.try_into().unwrap());
                        let row = RowId::from_idx(loc.y.try_into().unwrap());
                        let iob = TileIobId::from_idx(loc.bel.try_into().unwrap());
                        let io = if row == row_s {
                            EdgeIoCoord::S(col, iob)
                        } else if row == row_n {
                            EdgeIoCoord::N(col, iob)
                        } else if col == col_w {
                            EdgeIoCoord::W(row, iob)
                        } else if col == col_e {
                            EdgeIoCoord::E(row, iob)
                        } else {
                            unreachable!()
                        };
                        self.chip.io_iob.insert(io, io);
                        assert_eq!(bond.pins.insert(pin.clone(), BondPin::Io(io)), None);
                    }
                }
            }
            if let Some(infos) = self.pkg_bel_info.get(&(pkg, "SB_IO_OD")) {
                for info in infos {
                    let (col, row, ref wn) = info.in_wires[&InstPin::Simple("DOUT0".into())];
                    let col = ColId::from_idx(col.try_into().unwrap());
                    let row = RowId::from_idx(row.try_into().unwrap());
                    let iob = TileIobId::from_idx(if wn == "wire_io_cluster/io_0/D_OUT_0" {
                        0
                    } else if wn == "wire_io_cluster/io_1/D_OUT_0" {
                        1
                    } else {
                        panic!("ummm {wn}?")
                    });
                    let (loc, ref pin) = info.pads["PACKAGEPIN"];
                    let xy = (loc.x, loc.y, loc.bel);
                    assert_eq!(loc, info.loc);
                    let io = if row == row_s {
                        EdgeIoCoord::S(col, iob)
                    } else if row == row_n {
                        EdgeIoCoord::N(col, iob)
                    } else if col == col_w {
                        EdgeIoCoord::W(row, iob)
                    } else if col == col_e {
                        EdgeIoCoord::E(row, iob)
                    } else {
                        unreachable!()
                    };
                    self.chip.io_iob.insert(io, io);
                    self.chip.io_od.insert(io);
                    assert_eq!(bond.pins.insert(pin.clone(), BondPin::Io(io)), None);
                    match xlat_io.entry(xy) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(io);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), io);
                        }
                    }
                }
            }
            if matches!(self.part.name, "iCE65L04" | "iCE65P04") && pkg == "CB132" {
                // AAAAAAAAAAAAAAAAAAAaaaaaaaaaaaa
                let io = EdgeIoCoord::W(RowId::from_idx(11), TileIobId::from_idx(0));
                self.chip.io_iob.insert(io, io);
                bond.pins.insert("G1".into(), BondPin::Io(io));
                let io = EdgeIoCoord::W(RowId::from_idx(10), TileIobId::from_idx(1));
                self.chip.io_iob.insert(io, io);
                bond.pins.insert("H1".into(), BondPin::Io(io));
            }
            if self.part.kind.is_ice65() {
                for &io in self.chip.io_iob.keys() {
                    let (col, row, iob) = match io {
                        EdgeIoCoord::N(col, iob) => (col, row_n, iob),
                        EdgeIoCoord::E(row, iob) => (col_e, row, iob),
                        EdgeIoCoord::S(col, iob) => (col, row_s, iob),
                        EdgeIoCoord::W(row, iob) => (col_w, row, iob),
                    };
                    let xy = (
                        col.to_idx().try_into().unwrap(),
                        row.to_idx().try_into().unwrap(),
                        iob.to_idx().try_into().unwrap(),
                    );
                    match xlat_io.entry(xy) {
                        btree_map::Entry::Vacant(e) => {
                            e.insert(io);
                        }
                        btree_map::Entry::Occupied(e) => {
                            assert_eq!(*e.get(), io);
                        }
                    }
                }
            }
            for (pin, info) in &self.empty_runs[pkg].pin_table {
                let typ = &info.typ[..];
                let pad = match typ {
                    "GND" => BondPin::Gnd,
                    "VCC" => BondPin::VccInt,
                    "VCCIO_0" => BondPin::VccIo(0),
                    "VCCIO_1" => BondPin::VccIo(1),
                    "VCCIO_2" => BondPin::VccIo(2),
                    "VCCIO_3" => BondPin::VccIo(3),
                    "VDDIO_SPI" => BondPin::VccIoSpi,
                    "VPP_DIRECT" | "VPP" => BondPin::VppDirect,
                    "VPP_PUMP" | "VDDP" => BondPin::VppPump,
                    "VREF" => BondPin::Vref,
                    "VSSIO_LED" => BondPin::GndLed,
                    "AGND" | "AGND_BOT" => BondPin::GndPll(DirV::S),
                    "AVDD" | "AVDD_BOT" => BondPin::VccPll(DirV::S),
                    "AGND_TOP" => BondPin::GndPll(DirV::N),
                    "AVDD_TOP" => BondPin::VccPll(DirV::N),
                    "CRESET_B" => BondPin::Cfg(CfgPin::CResetB),
                    "CDONE" => BondPin::Cfg(CfgPin::CDone),
                    "TCK" => BondPin::Cfg(CfgPin::Tck),
                    "TMS" => BondPin::Cfg(CfgPin::Tms),
                    "TDI" => BondPin::Cfg(CfgPin::Tdi),
                    "TDO" => BondPin::Cfg(CfgPin::Tdo),
                    "TRST_B" => BondPin::Cfg(CfgPin::TrstB),
                    "POR_test" => BondPin::PorTest,
                    "NC" => BondPin::Nc,
                    "PIO" | "PIO_GBIN" | "PIO_GBIN_CDONE" | "PIO_LED" | "PIO_RGB"
                    | "PIO_BARCODE" | "PIO_I3C" => {
                        let BondPin::Io(crd) = bond.pins[pin] else {
                            panic!("umm {pin} not really IO?");
                        };
                        if typ == "PIO_GBIN_CDONE" {
                            bond.pins.insert(pin.clone(), BondPin::IoCDone(crd));
                        }
                        continue;
                    }
                    "SPI_SCK" | "SPI_SI" | "SPI_SO" | "SPI_SS_B" => {
                        let BondPin::Io(crd) = bond.pins[pin] else {
                            panic!("umm {pin} not really IO?");
                        };
                        let cpin = match typ {
                            "SPI_SCK" => SharedCfgPin::SpiSck,
                            "SPI_SI" => SharedCfgPin::SpiSi,
                            "SPI_SO" => SharedCfgPin::SpiSo,
                            "SPI_SS_B" => SharedCfgPin::SpiCsB,
                            _ => unreachable!(),
                        };
                        match self.chip.cfg_io.entry(cpin) {
                            btree_map::Entry::Vacant(e) => {
                                e.insert(crd);
                            }
                            btree_map::Entry::Occupied(e) => {
                                assert_eq!(*e.get(), crd);
                            }
                        }
                        continue;
                    }
                    _ => panic!("ummm {}", info.typ),
                };
                assert_eq!(bond.pins.insert(pin.clone(), pad), None);
            }
            let mut x3 = BTreeMap::new();
            for info in &self.pkg_bel_info[&(pkg, "IOx3")] {
                let xy = (info.loc.x, info.loc.y, info.loc.bel);
                let io = xlat_io[&xy];
                let r0 = info.dedio["REP0"];
                let ior0 = self.chip.get_io_crd((
                    DieId::from_idx(0),
                    (
                        ColId::from_idx(r0.x as usize),
                        RowId::from_idx(r0.y as usize),
                    ),
                    bels::IO[r0.bel as usize],
                ));
                let r1 = info.dedio["REP1"];
                let ior1 = self.chip.get_io_crd((
                    DieId::from_idx(0),
                    (
                        ColId::from_idx(r1.x as usize),
                        RowId::from_idx(r1.y as usize),
                    ),
                    bels::IO[r1.bel as usize],
                ));
                x3.insert(io, (ior0, ior1));
            }
            for bpin in bond.pins.values_mut() {
                if let BondPin::Io(io) = *bpin {
                    if let Some(&(ior0, ior1)) = x3.get(&io) {
                        let mut ior = [ior0, ior1];
                        ior.sort();
                        *bpin = BondPin::IoTriple([io, ior[0], ior[1]]);
                    }
                }
            }
            if pkg != "DI" {
                let all_pins = get_pkg_pins(pkg);
                for pin in &all_pins {
                    if let btree_map::Entry::Vacant(e) = bond.pins.entry(pin.to_string()) {
                        e.insert(BondPin::Nc);
                    }
                }
                assert_eq!(bond.pins.len(), all_pins.len());
            }
            self.bonds.insert(pkg, bond);
            self.xlat_io.insert(pkg, xlat_io);
        }
        self.chip.col_bio_split = match self.part.kind {
            ChipKind::Ice40T04 | ChipKind::Ice40T05 => ColId::from_idx(12),
            _ => {
                let EdgeIoCoord::S(col, _) = self.chip.cfg_io[&SharedCfgPin::SpiSo] else {
                    unreachable!()
                };
                col
            }
        };
    }

    fn fill_cbsel(&mut self) {
        if !self.part.kind.has_actual_io_we() {
            // not sure if the later devices really don't have CBSEL or just don't advertise it,
            // but the below pin mappings definitely aren't stable anymore
            return;
        }
        for &pkg in self.part.packages {
            let bond = &self.bonds[pkg];
            let balls = match pkg {
                "CB132" => [(SharedCfgPin::CbSel0, "L9"), (SharedCfgPin::CbSel1, "P10")],
                "CM36" | "CM36A" => [(SharedCfgPin::CbSel0, "E3"), (SharedCfgPin::CbSel1, "F3")],
                "CM49" => [(SharedCfgPin::CbSel0, "F4"), (SharedCfgPin::CbSel1, "G4")],
                "CB81" | "CM81" => [(SharedCfgPin::CbSel0, "G5"), (SharedCfgPin::CbSel1, "H5")],
                "CB121" => [(SharedCfgPin::CbSel0, "H6"), (SharedCfgPin::CbSel1, "J6")],
                "VQ100" => [(SharedCfgPin::CbSel0, "41"), (SharedCfgPin::CbSel1, "42")],
                _ => continue,
            };
            for (cpin, ball) in balls {
                let BondPin::Io(io) = bond.pins[ball] else {
                    unreachable!()
                };
                match self.chip.cfg_io.entry(cpin) {
                    btree_map::Entry::Vacant(e) => {
                        e.insert(io);
                    }
                    btree_map::Entry::Occupied(e) => {
                        assert_eq!(*e.get(), io);
                    }
                }
            }
        }
    }

    fn fill_bel_pins(&mut self) {
        let mut worklist = BTreeMap::new();
        let defpkg = self.part.packages[0];
        for (&kind, sites) in &self.bel_info {
            if kind == "SB_GB" || kind.starts_with("SB_RAM") {
                continue;
            }
            for site in sites {
                worklist.insert((kind, site.loc), (defpkg, site));
            }
        }
        for (&(pkg, kind), sites) in &self.pkg_bel_info {
            if matches!(
                kind,
                "SB_IO" | "SB_IO_DS" | "SB_IO_OD" | "SB_GB_IO" | "IOx3"
            ) {
                continue;
            }
            for site in sites {
                worklist.insert((kind, site.loc), (pkg, site));
            }
        }
        let edev = self.chip.expand_grid(&self.intdb);
        let db = if edev.chip.kind.has_actual_io_we() {
            None
        } else {
            Some(Database::from_file("databases/iCE40LP1K.zstd").unwrap())
        };
        let tiledb = db.as_ref().map(|x| &x.tiles);
        let extra_wire_names = Mutex::new(BTreeMap::new());
        let bel_pins = Mutex::new(BTreeMap::new());
        worklist
            .into_par_iter()
            .for_each(|((kind, _), (pkg, site))| {
                let mut pins = find_bel_pins(
                    self.sbt,
                    &self.prims,
                    self.part,
                    &edev,
                    tiledb,
                    pkg,
                    kind,
                    site,
                );
                let mut extra_wire_names = extra_wire_names.lock().unwrap();
                for (wn, iw) in std::mem::take(&mut pins.wire_names) {
                    match extra_wire_names.entry(wn) {
                        btree_map::Entry::Vacant(entry) => {
                            entry.insert(iw);
                        }
                        btree_map::Entry::Occupied(entry) => {
                            assert_eq!(*entry.get(), iw);
                        }
                    }
                }
                std::mem::drop(extra_wire_names);
                let mut bel_pins = bel_pins.lock().unwrap();
                bel_pins.insert((kind, site.loc), pins);
            });
        self.extra_wire_names = extra_wire_names.into_inner().unwrap();
        if self.chip.kind == ChipKind::Ice40T01 {
            let wire = self.extra_wire_names[&(13, 15, "wire_ir400_drv/IRLEDEN".into())];
            self.extra_wire_names
                .insert((13, 15, "wire_ir500_drv/IRLEDEN".into()), wire);
            let wire = self.extra_wire_names[&(13, 15, "wire_ir400_drv/IRPWM".into())];
            self.extra_wire_names
                .insert((13, 15, "wire_ir500_drv/IRPWM".into()), wire);
            let wire = self.extra_wire_names[&(13, 15, "wire_bc_drv/BARCODEEN".into())];
            self.extra_wire_names
                .insert((13, 15, "wire_ir500_drv/IRLEDEN2".into()), wire);
            let wire = self.extra_wire_names[&(13, 15, "wire_bc_drv/BARCODEPWM".into())];
            self.extra_wire_names
                .insert((13, 15, "wire_ir500_drv/IRPWM2".into()), wire);
        }
        self.bel_pins = bel_pins.into_inner().unwrap();
    }

    fn fill_io_latch(&mut self) {
        let package = self
            .part
            .packages
            .iter()
            .copied()
            .max_by_key(|pkg| {
                self.bonds[pkg]
                    .pins
                    .values()
                    .filter(|&pin| matches!(pin, BondPin::Io(_) | BondPin::IoCDone(_)))
                    .count()
            })
            .unwrap();
        let mut pkg_pins = HashMap::new();
        for (pkg_pin, &pin) in &self.bonds[package].pins {
            let (BondPin::Io(crd) | BondPin::IoCDone(crd)) = pin else {
                continue;
            };
            if self.chip.io_od.contains(&crd) {
                continue;
            }
            let edge = crd.edge();
            pkg_pins.entry(edge).or_insert(pkg_pin.as_str());
        }
        let expected = if self.chip.kind.has_io_we() && self.chip.kind != ChipKind::Ice40R04 {
            4
        } else {
            2
        };
        assert_eq!(pkg_pins.len(), expected);
        for (edge, (x, y)) in find_io_latch_locs(self.sbt, self.part, package, &pkg_pins) {
            self.chip.extra_nodes.insert(
                ExtraNodeLoc::LatchIo(edge),
                ExtraNode {
                    io: Default::default(),
                    tiles: EntityVec::from_iter([(
                        ColId::from_idx(x as usize),
                        RowId::from_idx(y as usize),
                    )]),
                },
            );
        }
    }

    fn fill_gbin_fabric(&mut self) {
        let sb_gb = &self.bel_info["SB_GB"];
        let mut found = HashSet::new();
        for site in sb_gb {
            let (x, y) = site.fabout_wires[&InstPin::Simple("USER_SIGNAL_TO_GLOBAL_BUFFER".into())];
            let crd = (ColId::from_idx(x as usize), RowId::from_idx(y as usize));
            let index = site.global_nets[&InstPin::Simple("GLOBAL_BUFFER_OUTPUT".into())];
            assert!(found.insert(index));
            self.chip.extra_nodes.insert(
                ExtraNodeLoc::GbFabric(index as usize),
                ExtraNode {
                    io: Default::default(),
                    tiles: EntityVec::from_iter([crd]),
                },
            );
        }
        assert_eq!(found.len(), 8);
    }

    fn fill_gbin_io(&mut self) {
        let mut gb_io = BTreeMap::new();
        for &package in self.part.packages {
            let xlat_io = &self.xlat_io[package];
            let sb_gb_io = &self.pkg_bel_info[&(package, "SB_GB_IO")];
            for site in sb_gb_io {
                let index =
                    site.global_nets[&InstPin::Simple("GLOBAL_BUFFER_OUTPUT".into())] as usize;
                let xy = (site.loc.x, site.loc.y, site.loc.bel);
                let io = xlat_io[&xy];
                match gb_io.entry(index) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(io);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), io);
                    }
                }
            }
        }

        if self.part.kind.is_ice65() {
            // sigh.
            if !gb_io.contains_key(&1) {
                let Some(&EdgeIoCoord::E(row, iob)) = gb_io.get(&0) else {
                    unreachable!()
                };
                gb_io.insert(1, EdgeIoCoord::W(row, iob));
            }
            if !gb_io.contains_key(&4) {
                let Some(&EdgeIoCoord::E(row, iob)) = gb_io.get(&5) else {
                    unreachable!()
                };
                gb_io.insert(4, EdgeIoCoord::W(row, iob));
            }
        }

        if self.chip.kind.has_actual_io_we() {
            for i in 0..8 {
                assert!(gb_io.contains_key(&i));
            }
        } else {
            for i in [0, 1, 2, 3, 6, 7] {
                assert!(gb_io.contains_key(&i));
            }
            for i in [4, 5] {
                assert!(!gb_io.contains_key(&i));
            }
        }

        for (index, io) in gb_io {
            let (_, (col, row), _) = self.chip.get_io_loc(io);
            let node = ExtraNode {
                io: BTreeMap::from_iter([(ExtraNodeIo::GbIn, io)]),
                tiles: EntityVec::from_iter([(col, row)]),
            };
            let loc = ExtraNodeLoc::GbIo(index);
            self.chip.extra_nodes.insert(loc, node);
            self.intdb
                .nodes
                .insert(loc.node_kind(), MiscNodeBuilder::new(&[(col, row)]).node);
        }
    }

    fn fill_trim(&mut self) {
        let loc = ExtraNodeLoc::Trim;
        let crd = match self.chip.kind {
            ChipKind::Ice40T04 | ChipKind::Ice40T05 => (self.chip.col_w(), RowId::from_idx(16)),
            ChipKind::Ice40T01 => (self.chip.col_w(), RowId::from_idx(13)),
            _ => return,
        };
        let nb = MiscNodeBuilder::new(&[crd]);
        let (int_node, extra_node) = nb.finish();
        self.intdb.nodes.insert(loc.node_kind(), int_node);
        self.chip.extra_nodes.insert(loc, extra_node);
    }

    fn fill_extra_misc(&mut self) {
        for kind in [
            "SB_MAC16",
            "SB_WARMBOOT",
            "SB_SPI",
            "SB_I2C",
            "SB_I2C_FIFO",
            "SB_HSOSC",
            "SB_LSOSC",
            "SB_HFOSC",
            "SB_LFOSC",
            "SB_LEDD_IP",
            "SB_LEDDA_IP",
            "SB_IR_IP",
        ] {
            let Some(sites) = self.bel_info.get(kind) else {
                continue;
            };
            for site in sites {
                let (loc, slot, fixed_crd, extra_crd, dedio) = match kind {
                    "SB_MAC16" => {
                        let col = self.xlat_col[self.part.packages[0]][site.loc.x as usize];
                        let row = self.xlat_row[self.part.packages[0]][site.loc.y as usize];
                        (
                            if self.chip.kind == ChipKind::Ice40T05
                                && col.to_idx() == 0
                                && row.to_idx() == 15
                            {
                                ExtraNodeLoc::Mac16Trim(col, row)
                            } else {
                                ExtraNodeLoc::Mac16(col, row)
                            },
                            bels::MAC16,
                            (col, row),
                            vec![],
                            [].as_slice(),
                        )
                    }
                    "SB_WARMBOOT" => (
                        ExtraNodeLoc::Warmboot,
                        bels::WARMBOOT,
                        (self.chip.col_e(), self.chip.row_s()),
                        vec![],
                        [].as_slice(),
                    ),
                    "SB_SPI" => {
                        let (edge, col) = if site.loc.x == 0 {
                            (DirH::W, self.chip.col_w())
                        } else {
                            (DirH::E, self.chip.col_e())
                        };
                        (
                            ExtraNodeLoc::Spi(edge),
                            bels::SPI,
                            (col, self.chip.row_s()),
                            vec![],
                            [
                                (ExtraNodeIo::SpiCopi, "MOSI"),
                                (ExtraNodeIo::SpiCipo, "MISO"),
                                (ExtraNodeIo::SpiSck, "SCK"),
                                (ExtraNodeIo::SpiCsB0, "CSN0"),
                                (ExtraNodeIo::SpiCsB1, "CSN1"),
                            ]
                            .as_slice(),
                        )
                    }
                    "SB_I2C" => {
                        let (edge, col) = if site.loc.x == 0 {
                            (DirH::W, self.chip.col_w())
                        } else {
                            (DirH::E, self.chip.col_e())
                        };
                        (
                            ExtraNodeLoc::I2c(edge),
                            bels::I2C,
                            (col, self.chip.row_n()),
                            vec![],
                            [(ExtraNodeIo::I2cScl, "SCL"), (ExtraNodeIo::I2cSda, "SDA")].as_slice(),
                        )
                    }
                    "SB_I2C_FIFO" => {
                        let (edge, col) = if site.loc.x == 0 {
                            (DirH::W, self.chip.col_w())
                        } else {
                            (DirH::E, self.chip.col_e())
                        };
                        (
                            ExtraNodeLoc::I2cFifo(edge),
                            bels::I2C_FIFO,
                            (col, self.chip.row_s()),
                            vec![],
                            [(ExtraNodeIo::I2cScl, "SCL"), (ExtraNodeIo::I2cSda, "SDA")].as_slice(),
                        )
                    }
                    "SB_HSOSC" => (
                        ExtraNodeLoc::HsOsc,
                        bels::HSOSC,
                        (self.chip.col_w(), self.chip.row_n()),
                        vec![],
                        [].as_slice(),
                    ),
                    "SB_LSOSC" => (
                        ExtraNodeLoc::LsOsc,
                        bels::LSOSC,
                        (self.chip.col_e(), self.chip.row_n()),
                        vec![],
                        [].as_slice(),
                    ),
                    "SB_HFOSC" => (
                        ExtraNodeLoc::HfOsc,
                        bels::HFOSC,
                        (
                            self.chip.col_w(),
                            if self.chip.kind == ChipKind::Ice40T01 {
                                self.chip.row_s()
                            } else {
                                self.chip.row_n()
                            },
                        ),
                        vec![],
                        [].as_slice(),
                    ),
                    "SB_LFOSC" => (
                        ExtraNodeLoc::LfOsc,
                        bels::LFOSC,
                        (
                            self.chip.col_e(),
                            if self.chip.kind == ChipKind::Ice40T01 {
                                self.chip.row_s()
                            } else {
                                self.chip.row_n()
                            },
                        ),
                        vec![],
                        [].as_slice(),
                    ),
                    "SB_LEDD_IP" => (
                        ExtraNodeLoc::LeddIp,
                        bels::LEDD_IP,
                        (self.chip.col_w(), self.chip.row_n()),
                        vec![],
                        [].as_slice(),
                    ),
                    "SB_LEDDA_IP" => (
                        ExtraNodeLoc::LeddaIp,
                        bels::LEDDA_IP,
                        (self.chip.col_w(), self.chip.row_n()),
                        vec![],
                        [].as_slice(),
                    ),
                    "SB_IR_IP" => (
                        ExtraNodeLoc::IrIp,
                        bels::IR_IP,
                        (self.chip.col_e(), self.chip.row_n()),
                        vec![],
                        [].as_slice(),
                    ),
                    _ => unreachable!(),
                };
                let bel_pins = &self.bel_pins[&(kind, site.loc)];
                let mut nb = MiscNodeBuilder::new(&[fixed_crd]);
                nb.add_bel(slot, bel_pins);
                for crd in extra_crd {
                    nb.get_tile(crd);
                }
                let (int_node, mut extra_node) = nb.finish();
                for &(slot, pin) in dedio {
                    let loc = site.dedio[pin];
                    let xy = (loc.x, loc.y, loc.bel);
                    let io = self.xlat_io[self.part.packages[0]][&xy];
                    extra_node.io.insert(slot, io);
                }
                self.intdb.nodes.insert(loc.node_kind(), int_node);
                self.chip.extra_nodes.insert(loc, extra_node);
                self.extra_node_locs.insert(loc, vec![site.loc]);
            }
        }
    }

    fn fill_pll(&mut self) {
        let kind = if self.chip.kind.is_ice40() {
            "SB_PLL40_CORE"
        } else {
            "SB_PLL_CORE"
        };
        for &package in self.part.packages {
            let Some(sites) = self.pkg_bel_info.get(&(package, kind)) else {
                continue;
            };
            for site in sites {
                let xy = (site.loc.x, site.loc.y, site.loc.bel);
                let io = self.xlat_io[package][&xy];
                let io2 = match io {
                    EdgeIoCoord::S(col, _) => EdgeIoCoord::S(col + 1, TileIobId::from_idx(0)),
                    EdgeIoCoord::N(col, _) => EdgeIoCoord::N(col + 1, TileIobId::from_idx(0)),
                    _ => unreachable!(),
                };
                let (_, (col, row), _) = self.chip.get_io_loc(io);
                let mut bel_pins = self.bel_pins[&(kind, site.loc)].clone();
                bel_pins.ins.remove("LATCHINPUTVALUE");
                bel_pins.outs.retain(|k, _| !k.starts_with("PLLOUT"));
                let mut nb = MiscNodeBuilder::new(&[(col, row), (col + 1, row)]);
                if self.chip.kind.is_ice40() {
                    if self.chip.kind == ChipKind::Ice40P01 {
                        for i in 1..=5 {
                            nb.get_tile((self.chip.col_w(), RowId::from_idx(i)));
                        }
                    } else {
                        for i in 0..5 {
                            nb.get_tile((col - 2 + i, row));
                        }
                    }
                }
                nb.io.insert(ExtraNodeIo::PllA, io);
                nb.io.insert(ExtraNodeIo::PllB, io2);
                nb.add_bel(bels::PLL, &bel_pins);
                let (int_node, extra_node) = nb.finish();
                let loc = ExtraNodeLoc::Pll(if row == self.chip.row_s() {
                    DirV::S
                } else {
                    DirV::N
                });
                match self.chip.extra_nodes.entry(loc) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(extra_node);
                        self.intdb.nodes.insert(loc.node_kind(), int_node);
                        self.extra_node_locs.insert(loc, vec![site.loc]);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), extra_node);
                    }
                }
            }
        }
        if matches!(self.chip.kind, ChipKind::Ice40T04 | ChipKind::Ice40T05) {
            let xnode = ExtraNode {
                io: BTreeMap::from_iter([
                    (
                        ExtraNodeIo::PllA,
                        self.chip.extra_nodes[&ExtraNodeLoc::GbIo(6)].io[&ExtraNodeIo::GbIn],
                    ),
                    (
                        ExtraNodeIo::PllB,
                        self.chip.extra_nodes[&ExtraNodeLoc::GbIo(3)].io[&ExtraNodeIo::GbIn],
                    ),
                ]),
                tiles: EntityVec::from_iter([(self.chip.col_mid() + 1, self.chip.row_s())]),
            };
            let xloc = ExtraNodeLoc::PllStub(DirV::S);
            self.chip.extra_nodes.insert(xloc, xnode);
            let node = NodeKind {
                tiles: EntityVec::from_iter([()]),
                muxes: Default::default(),
                iris: Default::default(),
                intfs: Default::default(),
                bels: Default::default(),
            };
            self.intdb.nodes.insert(xloc.node_kind(), node);
        }
    }

    fn fill_io_i3c(&mut self) {
        for &package in self.part.packages {
            let Some(sites) = self.pkg_bel_info.get(&(package, "SB_IO_I3C")) else {
                continue;
            };
            for site in sites {
                let xy = (site.loc.x, site.loc.y, site.loc.bel);
                let crd = self.xlat_io[package][&xy];
                let (_, (col, row), _) = self.chip.get_io_loc(crd);
                let mut bel_pins = self.bel_pins[&("SB_IO_I3C", site.loc)].clone();
                bel_pins.outs.clear();
                let mut nb = MiscNodeBuilder::new(&[(col, row)]);
                nb.add_bel(bels::IO_I3C[crd.iob().to_idx()], &bel_pins);
                let (int_node, extra_node) = nb.finish();
                let loc = ExtraNodeLoc::IoI3c(crd);
                match self.chip.extra_nodes.entry(loc) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(extra_node);
                        self.intdb.nodes.insert(loc.node_kind(), int_node);
                        self.extra_node_locs.insert(loc, vec![site.loc]);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), extra_node);
                    }
                }
            }
        }
    }

    fn fill_drv(&mut self) {
        for &package in self.part.packages {
            for (loc, node_bels, fixed_crd, extra_crd) in [
                (
                    ExtraNodeLoc::RgbDrv,
                    [(
                        "SB_RGB_DRV",
                        bels::RGB_DRV,
                        [
                            (ExtraNodeIo::RgbLed0, "RGB0"),
                            (ExtraNodeIo::RgbLed1, "RGB1"),
                            (ExtraNodeIo::RgbLed2, "RGB2"),
                        ]
                        .as_slice(),
                    )]
                    .as_slice(),
                    (self.chip.col_w(), self.chip.row_n()),
                    vec![
                        (ColId::from_idx(0), RowId::from_idx(18)),
                        (ColId::from_idx(0), RowId::from_idx(19)),
                        (ColId::from_idx(0), RowId::from_idx(20)),
                    ],
                ),
                (
                    ExtraNodeLoc::IrDrv,
                    [(
                        "SB_IR_DRV",
                        bels::IR_DRV,
                        [(ExtraNodeIo::IrLed, "IRLED")].as_slice(),
                    )]
                    .as_slice(),
                    (self.chip.col_e(), self.chip.row_n()),
                    vec![
                        (ColId::from_idx(25), RowId::from_idx(19)),
                        (ColId::from_idx(25), RowId::from_idx(20)),
                    ],
                ),
                (
                    ExtraNodeLoc::RgbaDrv,
                    [(
                        "SB_RGBA_DRV",
                        bels::RGBA_DRV,
                        [
                            (ExtraNodeIo::RgbLed0, "RGB0"),
                            (ExtraNodeIo::RgbLed1, "RGB1"),
                            (ExtraNodeIo::RgbLed2, "RGB2"),
                        ]
                        .as_slice(),
                    )]
                    .as_slice(),
                    (self.chip.col_w(), self.chip.row_n()),
                    match self.chip.kind {
                        ChipKind::Ice40T05 => vec![
                            (ColId::from_idx(0), RowId::from_idx(28)),
                            (ColId::from_idx(0), RowId::from_idx(29)),
                            (ColId::from_idx(0), RowId::from_idx(30)),
                        ],
                        ChipKind::Ice40T01 => vec![
                            (ColId::from_idx(0), RowId::from_idx(1)),
                            (ColId::from_idx(0), RowId::from_idx(2)),
                            (ColId::from_idx(0), RowId::from_idx(3)),
                        ],
                        _ => vec![],
                    },
                ),
                (
                    ExtraNodeLoc::Ir500Drv,
                    [
                        (
                            "SB_IR400_DRV",
                            bels::IR400_DRV,
                            [(ExtraNodeIo::IrLed, "IRLED")].as_slice(),
                        ),
                        (
                            "SB_BARCODE_DRV",
                            bels::BARCODE_DRV,
                            [(ExtraNodeIo::BarcodeLed, "BARCODE")].as_slice(),
                        ),
                    ]
                    .as_slice(),
                    (self.chip.col_e(), self.chip.row_n()),
                    vec![
                        (ColId::from_idx(13), RowId::from_idx(1)),
                        (ColId::from_idx(13), RowId::from_idx(2)),
                        (ColId::from_idx(13), RowId::from_idx(3)),
                    ],
                ),
            ] {
                let Some(sites) = self.pkg_bel_info.get(&(package, node_bels[0].0)) else {
                    continue;
                };
                if sites.is_empty() {
                    continue;
                }
                let mut nb = MiscNodeBuilder::new(&[fixed_crd]);
                for crd in extra_crd {
                    nb.get_tile(crd);
                }

                let mut site_locs = vec![];
                for &(kind, slot, io_pins) in node_bels {
                    let sites = &self.pkg_bel_info[&(package, kind)];
                    assert_eq!(sites.len(), 1);
                    let site = &sites[0];
                    site_locs.push(site.loc);
                    let mut bel_pins = self.bel_pins[&(kind, site.loc)].clone();
                    let mut bel_pins_drv = BelPins::default();
                    bel_pins.ins.retain(|pin, &mut iw| {
                        if let Some(pin) = pin.strip_prefix("LED_DRV_CUR__") {
                            bel_pins_drv.ins.insert(pin.to_string(), iw);
                            false
                        } else if pin == "CURREN" {
                            bel_pins_drv.ins.insert("EN".to_string(), iw);
                            false
                        } else if pin.starts_with("TRIM") {
                            bel_pins_drv.ins.insert(pin.clone(), iw);
                            false
                        } else {
                            true
                        }
                    });
                    for &(slot, pin) in io_pins {
                        let io = site.pads[pin].0;
                        let xy = (io.x, io.y, io.bel);
                        let crd = self.xlat_io[package][&xy];
                        nb.io.insert(slot, crd);
                    }
                    nb.add_bel(slot, &bel_pins);

                    let fixed_crd = if self.chip.kind == ChipKind::Ice40T01 {
                        (self.chip.col_e(), self.chip.row_s())
                    } else {
                        (self.chip.col_e(), self.chip.row_n())
                    };
                    let mut nb_drv_cur = MiscNodeBuilder::new(&[fixed_crd]);
                    nb_drv_cur.add_bel(bels::LED_DRV_CUR, &bel_pins_drv);
                    let (int_node, extra_node) = nb_drv_cur.finish();
                    let loc = ExtraNodeLoc::LedDrvCur;
                    match self.chip.extra_nodes.entry(loc) {
                        btree_map::Entry::Vacant(entry) => {
                            entry.insert(extra_node);
                            self.intdb.nodes.insert(loc.node_kind(), int_node);
                        }
                        btree_map::Entry::Occupied(entry) => {
                            assert_eq!(*entry.get(), extra_node);
                        }
                    }
                }
                let (int_node, extra_node) = nb.finish();
                match self.chip.extra_nodes.entry(loc) {
                    btree_map::Entry::Vacant(entry) => {
                        entry.insert(extra_node);
                        self.intdb.nodes.insert(loc.node_kind(), int_node);
                        self.extra_node_locs.insert(loc, site_locs);
                    }
                    btree_map::Entry::Occupied(entry) => {
                        assert_eq!(*entry.get(), extra_node);
                    }
                }
            }
        }
    }

    fn fill_spram(&mut self) {
        let Some(sites) = self.bel_info.get("SB_SPRAM256KA") else {
            return;
        };
        let mut sites = sites.clone();
        sites.sort_by_key(|site| site.loc);
        assert_eq!(sites.len(), 4);
        for edge_sites in sites.chunks_exact(2) {
            assert_eq!(edge_sites[0].loc.x, edge_sites[1].loc.x);
            let edge = if edge_sites[0].loc.x == 0 {
                DirH::W
            } else {
                DirH::E
            };
            let loc = ExtraNodeLoc::SpramPair(edge);
            let mut nb = MiscNodeBuilder::new(&[]);
            for (i, site) in edge_sites.iter().enumerate() {
                let bel_pins = &self.bel_pins[&("SB_SPRAM256KA", site.loc)];
                nb.add_bel(bels::SPRAM[i], bel_pins);
            }
            let (int_node, extra_node) = nb.finish();
            self.intdb.nodes.insert(loc.node_kind(), int_node);
            self.chip.extra_nodes.insert(loc, extra_node);
            self.extra_node_locs
                .insert(loc, vec![edge_sites[0].loc, edge_sites[1].loc]);
        }
    }

    fn fill_filter(&mut self) {
        let Some(sites) = self.bel_info.get("SB_FILTER_50NS") else {
            return;
        };
        let mut sites = sites.clone();
        sites.sort_by_key(|site| site.loc);
        assert_eq!(sites.len(), 2);
        let loc = ExtraNodeLoc::FilterPair;
        let mut nb = MiscNodeBuilder::new(&[]);
        for (i, site) in sites.iter().enumerate() {
            let bel_pins = &self.bel_pins[&("SB_FILTER_50NS", site.loc)];
            nb.add_bel(bels::FILTER[i], bel_pins);
        }
        nb.get_tile((ColId::from_idx(25), RowId::from_idx(30)));
        let (int_node, extra_node) = nb.finish();
        self.intdb.nodes.insert(loc.node_kind(), int_node);
        self.chip.extra_nodes.insert(loc, extra_node);
        self.extra_node_locs
            .insert(loc, vec![sites[0].loc, sites[1].loc]);
    }

    fn fill_smcclk(&mut self) {
        let (col, row, wire) = match self.chip.kind {
            ChipKind::Ice40T04 => (ColId::from_idx(25), RowId::from_idx(3), "OUT.LC5"),
            ChipKind::Ice40T05 => (ColId::from_idx(25), RowId::from_idx(9), "OUT.LC1"),
            _ => return,
        };
        let wire = self.intdb.get_wire(wire);
        let mut node = NodeKind {
            tiles: EntityVec::from_iter([()]),
            muxes: Default::default(),
            iris: Default::default(),
            intfs: Default::default(),
            bels: Default::default(),
        };
        let mut bel = BelInfo::default();
        bel.pins.insert(
            "CLK".into(),
            BelPin {
                wires: BTreeSet::from_iter([(NodeTileId::from_idx(0), wire)]),
                dir: PinDir::Output,
                is_intf_in: false,
            },
        );
        node.bels.insert(bels::SMCCLK, bel);
        self.intdb.nodes.insert("SMCCLK".into(), node);
        self.chip.extra_nodes.insert(
            ExtraNodeLoc::SmcClk,
            ExtraNode {
                io: Default::default(),
                tiles: EntityVec::from_iter([(col, row)]),
            },
        );
    }

    fn compute_rows_colbuf(
        &self,
        colbuf_map: BTreeMap<RowId, RowId>,
    ) -> Option<Vec<(RowId, RowId, RowId)>> {
        let mut row_c = *colbuf_map.get(&RowId::from_idx(1))?;
        let mut row_b = RowId::from_idx(0);
        let mut row_prev = RowId::from_idx(0);
        let mut in_top = false;
        let mut result = vec![];
        for (row, trow) in colbuf_map {
            if trow != row_c {
                if row != row_prev + 1 {
                    return None;
                }
                if !in_top {
                    assert_eq!(trow, row_c + 1);
                    assert_eq!(trow, row);
                    in_top = true;
                } else {
                    result.push((row_c, row_b, row));
                    row_b = row;
                    in_top = false;
                }
                row_c = trow;
            }
            row_prev = row;
        }
        if row_prev != self.chip.row_n() - 1 {
            return None;
        }
        assert!(in_top);
        result.push((row_c, row_b, row_prev + 2));
        Some(result)
    }

    fn inject_lut0_cascade(&mut self, harvester: &mut Harvester<BitOwner>) {
        harvester.force_tiled(
            "PLB:LC0:MUX.I2:LTIN",
            BTreeMap::from_iter([(
                TileBit {
                    tile: 0,
                    frame: 0,
                    bit: 50,
                },
                true,
            )]),
        );
    }

    fn inject_io_inv_clk(&mut self, harvester: &mut Harvester<BitOwner>) {
        for (tile, key, bit) in [
            (
                "IO.W",
                "INT:INV.IMUX.IO.ICLK:BIT0",
                TileBit {
                    tile: 0,
                    frame: 9,
                    bit: 4,
                },
            ),
            (
                "IO.W",
                "INT:INV.IMUX.IO.OCLK:BIT0",
                TileBit {
                    tile: 0,
                    frame: 15,
                    bit: 4,
                },
            ),
            (
                "IO.E",
                "INT:INV.IMUX.IO.ICLK:BIT0",
                TileBit {
                    tile: 0,
                    frame: 9,
                    bit: 13,
                },
            ),
            (
                "IO.E",
                "INT:INV.IMUX.IO.OCLK:BIT0",
                TileBit {
                    tile: 0,
                    frame: 15,
                    bit: 13,
                },
            ),
            (
                "IO.S",
                "INT:INV.IMUX.IO.ICLK:BIT0",
                TileBit {
                    tile: 0,
                    frame: 6,
                    bit: 35,
                },
            ),
            (
                "IO.S",
                "INT:INV.IMUX.IO.OCLK:BIT0",
                TileBit {
                    tile: 0,
                    frame: 1,
                    bit: 35,
                },
            ),
            (
                "IO.N",
                "INT:INV.IMUX.IO.ICLK:BIT0",
                TileBit {
                    tile: 0,
                    frame: 9,
                    bit: 35,
                },
            ),
            (
                "IO.N",
                "INT:INV.IMUX.IO.OCLK:BIT0",
                TileBit {
                    tile: 0,
                    frame: 14,
                    bit: 35,
                },
            ),
        ] {
            if self.intdb.nodes.contains_key(tile) {
                harvester.force_tiled(format!("{tile}:{key}"), BTreeMap::from_iter([(bit, true)]));
            }
        }
    }

    fn transplant_r04(
        &mut self,
        harvester: &mut Harvester<BitOwner>,
        muxes: &mut BTreeMap<NodeKindId, BTreeMap<NodeWireId, MuxInfo>>,
    ) {
        let db = Database::from_file("databases/iCE40LP1K.zstd").unwrap();
        for tile in ["IO.W", "IO.E"] {
            let node = db.int.nodes.get(tile).unwrap().1;
            let node_dst = self.intdb.nodes.get(tile).unwrap().0;
            muxes.insert(node_dst, node.muxes.clone());
            let tile_data = &db.tiles.tiles[tile];
            for (name, item) in &tile_data.items {
                if name.starts_with("COLBUF:")
                    || name.ends_with(":PIN_TYPE")
                    || name.starts_with("INT:INV")
                {
                    let TileItemKind::BitVec { ref invert } = item.kind else {
                        unreachable!()
                    };
                    for (idx, (&bit, inv)) in item.bits.iter().zip(invert.iter()).enumerate() {
                        harvester.force_tiled(
                            format!("{tile}:{name}:BIT{idx}"),
                            BTreeMap::from_iter([(bit, !*inv)]),
                        );
                    }
                } else if name.starts_with("INT:MUX") {
                    let TileItemKind::Enum { ref values } = item.kind else {
                        unreachable!()
                    };
                    for (vname, val) in values {
                        if vname == "NONE" {
                            continue;
                        }
                        harvester.force_tiled(
                            format!("{tile}:{name}:{vname}"),
                            BTreeMap::from_iter(
                                item.bits
                                    .iter()
                                    .zip(val.iter())
                                    .filter(|(_, bval)| **bval)
                                    .map(|(&bit, _)| (bit, true)),
                            ),
                        );
                    }
                } else if let Some(suf) = name.strip_prefix("INT:BUF.") {
                    let (wt, b) = suf.split_once(".OUT.").unwrap();
                    let TileItemKind::BitVec { ref invert } = item.kind else {
                        unreachable!()
                    };
                    assert_eq!(item.bits.len(), 1);
                    harvester.force_tiled(
                        format!("{tile}:INT:MUX.{wt}:OUT.{b}"),
                        BTreeMap::from_iter(
                            item.bits
                                .iter()
                                .zip(invert.iter())
                                .map(|(&bit, inv)| (bit, !*inv)),
                        ),
                    );
                }
            }
        }
    }

    fn harvest(&mut self) {
        let mut harvester = Harvester::new();
        let mut muxes = BTreeMap::new();
        if self.chip.kind.is_ice40() {
            self.inject_lut0_cascade(&mut harvester);
        }
        self.inject_io_inv_clk(&mut harvester);
        if self.chip.kind == ChipKind::Ice40R04 {
            self.transplant_r04(&mut harvester, &mut muxes);
        }

        let edev = self.chip.expand_grid(&self.intdb);
        let mut gencfg = GeneratorConfig {
            part: self.part,
            prims: &self.prims,
            edev: &edev,
            bonds: &self.bonds,
            plb_info: &self.plb_info,
            bel_info: &self.bel_info,
            pkg_bel_info: &self.pkg_bel_info,
            allow_global: false,
            rows_colbuf: vec![],
            extra_node_locs: &self.extra_node_locs,
        };
        let muxes = Mutex::new(muxes);
        harvester.debug = self.debug;
        for key in wanted_keys_tiled(&edev) {
            harvester.want_tiled(key);
        }
        for key in wanted_keys_global(&edev) {
            harvester.want_global(key);
        }
        let mut harvester = Mutex::new(harvester);
        for round in 0.. {
            if round >= 5 {
                gencfg.allow_global = true;
            }
            (0..40).into_par_iter().for_each(|_| {
                let design = generate(&gencfg);
                let mut result = match run(self.sbt, &design) {
                    Ok(res) => res,
                    Err(err) => {
                        if self.debug >= 2 {
                            println!("OOPS {err:?}");
                            if let Some(dir) = err.dir {
                                println!(
                                    "fucked directory left at {}",
                                    dir.path().to_string_lossy()
                                );
                                std::mem::forget(dir);
                            }
                        }
                        return;
                    }
                };
                let dir = ManuallyDrop::new(result.dir.take());
                if let Some(ref dir) = *dir {
                    println!("YAY {}", dir.path().to_string_lossy());
                }
                let (sample, cur_pips) = make_sample(
                    &design,
                    &edev,
                    &result,
                    &self.empty_runs[&design.package],
                    &self.xlat_col[&design.package],
                    &self.xlat_row[&design.package],
                    &self.xlat_io[&design.package],
                    &gencfg.rows_colbuf,
                    &self.extra_wire_names,
                    &self.extra_node_locs,
                );
                let mut harvester = harvester.lock().unwrap();
                let mut muxes = muxes.lock().unwrap();
                let mut ctr = 0;
                for pip in cur_pips {
                    let mux = muxes
                        .entry(pip.0)
                        .or_default()
                        .entry((NodeTileId::from_idx(0), pip.1))
                        .or_insert_with(|| MuxInfo {
                            kind: MuxKind::Plain,
                            ins: BTreeSet::new(),
                        });

                    if mux.ins.insert((NodeTileId::from_idx(0), pip.2)) {
                        ctr += 1;
                    }
                }
                if self.debug >= 2 {
                    println!("TOTAL NEW PIPS: {ctr} / {tot}", tot = muxes.len());
                }
                drop(muxes);
                if let Some(sample_id) = harvester.add_sample(sample) {
                    if let Some(ref dir) = *dir {
                        println!("SAMPLE {sample_id} {}", dir.path().to_string_lossy());
                    }
                }
            });
            let harvester = harvester.get_mut().unwrap();
            if self.debug >= 1 {
                println!("ROUND {round}:")
            }
            harvester.process();
            if self.chip.kind.has_colbuf() && gencfg.rows_colbuf.is_empty() {
                let mut plb_bits = [const { None }; 8];
                let mut colbuf_map = BTreeMap::new();
                for (key, bits) in &harvester.known_global {
                    let Some(crd) = key.strip_prefix("COLBUF:") else {
                        continue;
                    };
                    let (_, crd) = crd.split_once('.').unwrap();
                    let (srow, idx) = crd.split_once('.').unwrap();
                    let srow = RowId::from_idx(srow.parse().unwrap());
                    let idx: usize = idx.parse().unwrap();
                    assert_eq!(bits.len(), 1);
                    let (&bit, &val) = bits.iter().next().unwrap();
                    plb_bits[idx] = Some(BTreeMap::from_iter([(
                        TileBit {
                            tile: 0,
                            frame: bit.1,
                            bit: bit.2,
                        },
                        val,
                    )]));
                    let BitOwner::Main(_, row) = bit.0 else {
                        unreachable!()
                    };
                    colbuf_map.insert(srow, row);
                }
                if self.debug >= 3 {
                    println!("COLBUF ROWS: {colbuf_map:?}");
                }
                let got_all_idx = plb_bits.iter().all(|x| x.is_some());
                if got_all_idx {
                    if let Some(rows_colbuf) = self.compute_rows_colbuf(colbuf_map) {
                        if self.debug >= 1 {
                            println!("HEEEEEEY WE GOT COLBUFS!");
                        }
                        gencfg.rows_colbuf = rows_colbuf;
                        for col in self.chip.columns() {
                            if self.chip.kind.has_io_we()
                                && (col == self.chip.col_w() || col == self.chip.col_e())
                            {
                                continue;
                            }
                            if self.chip.cols_bram.contains(&col) {
                                continue;
                            }
                            for row in self.chip.rows() {
                                if row == self.chip.row_s() || row == self.chip.row_n() {
                                    continue;
                                }
                                let (row_colbuf, _, _) = gencfg
                                    .rows_colbuf
                                    .iter()
                                    .copied()
                                    .find(|&(_, row_b, row_t)| row >= row_b && row < row_t)
                                    .unwrap();
                                let trow = if row < row_colbuf {
                                    if edev.chip.cols_bram.contains(&col)
                                        && !edev.chip.kind.has_ice40_bramv2()
                                    {
                                        row_colbuf - 2
                                    } else {
                                        row_colbuf - 1
                                    }
                                } else {
                                    row_colbuf
                                };

                                for (idx, bits) in plb_bits.iter().enumerate() {
                                    let bits = bits.as_ref().unwrap();
                                    let key = format!("COLBUF:{col}.{row}.{idx}");
                                    let bits = bits
                                        .iter()
                                        .map(|(&bit, &val)| {
                                            ((BitOwner::Main(col, trow), bit.frame, bit.bit), val)
                                        })
                                        .collect();
                                    harvester.force_global(key.clone(), bits);
                                    harvester.known_global.remove(&key);
                                }
                            }
                        }

                        for (idx, bits) in plb_bits.into_iter().enumerate() {
                            harvester.force_tiled(
                                format!("PLB:COLBUF:GLOBAL.{idx}:BIT0"),
                                bits.unwrap(),
                            );
                        }
                        harvester.process();
                    }
                }
            }
            let muxes = muxes.lock().unwrap();
            let mut nodes_complete = 0;
            for (&nk, muxes) in &*muxes {
                let mut stats: BTreeMap<String, usize> = BTreeMap::new();
                let nkn = edev.egrid.db.nodes.key(nk);
                for (&(_, wt), mux) in muxes {
                    let wtn = edev.egrid.db.wires.key(wt);
                    for &(_, wf) in &mux.ins {
                        let wfn = edev.egrid.db.wires.key(wf);
                        let bucket = if wtn.starts_with("QUAD.V") && wfn.starts_with("QUAD") {
                            "QUAD-QUAD.V"
                        } else if wtn.starts_with("QUAD.H") && wfn.starts_with("QUAD") {
                            "QUAD-QUAD.H"
                        } else if wtn.starts_with("QUAD.V") && wfn.starts_with("LONG") {
                            "LONG-QUAD.V"
                        } else if wtn.starts_with("QUAD.H") && wfn.starts_with("LONG") {
                            "LONG-QUAD.H"
                        } else if wtn.starts_with("QUAD.V") && wfn.starts_with("OUT") {
                            "OUT-QUAD.V"
                        } else if wtn.starts_with("QUAD.H") && wfn.starts_with("OUT") {
                            "OUT-QUAD.H"
                        } else if wtn.starts_with("LONG.V") && wfn.starts_with("LONG") {
                            "LONG-LONG.V"
                        } else if wtn.starts_with("LONG.H") && wfn.starts_with("LONG") {
                            "LONG-LONG.H"
                        } else if wtn.starts_with("LONG.V") && wfn.starts_with("OUT") {
                            "OUT-LONG.V"
                        } else if wtn.starts_with("LONG.H") && wfn.starts_with("OUT") {
                            "OUT-LONG.H"
                        } else {
                            wtn
                        };
                        *stats.entry(bucket.to_string()).or_default() += 1;
                    }
                }
                let golden_stats = get_golden_mux_stats(edev.chip.kind, nkn);
                if stats == golden_stats {
                    nodes_complete += 1;
                } else {
                    for (k, &v) in &stats {
                        let gv = golden_stats.get(k).copied().unwrap_or(0);
                        if v > gv {
                            println!(
                                "UMMMM GOT MORE MUXES THAN BARGAINED FOR AT {nkn} {k} {v}/{gv}"
                            );
                        }
                    }
                    let mut missing = BTreeMap::new();
                    for (k, &gv) in &golden_stats {
                        let v = stats.get(k).copied().unwrap_or(0);
                        if v < gv {
                            missing.insert(k, gv - v);
                        }
                    }
                    if self.debug >= 1 && !missing.is_empty() {
                        print!("missing muxes in {nkn}:");
                        for (k, v) in missing {
                            print!(" {v}×{k}");
                        }
                        println!();
                    }
                }
            }
            let golden_nodes_complete = if edev.chip.kind == ChipKind::Ice40P03 {
                5 // PLB, 4×IO
            } else if edev.chip.kind.has_io_we() {
                6 // PLB, INT.BRAM, 4×IO
            } else {
                4 // PLB, INT.BRAM, 2×IO
            };
            if golden_nodes_complete == nodes_complete && !harvester.has_unresolved() {
                break;
            }
        }
        println!("DONE with {}!", self.part.name);
        let mut muxes = muxes.into_inner().unwrap();
        let mut harvester = harvester.into_inner().unwrap();
        let io_iob = collect_iob(&edev, &mut harvester);
        let tiledb = collect(&edev, &muxes, &harvester);
        self.chip.rows_colbuf = gencfg.rows_colbuf;
        self.chip.io_iob = io_iob;

        for tile_muxes in muxes.values_mut() {
            let mut new_muxes = BTreeMap::new();
            for (&(_, wt), mux) in &mut *tile_muxes {
                let wtn = self.intdb.wires.key(wt);
                if let Some(idx) = wtn.strip_prefix("LOCAL.") {
                    let (a, b) = idx.split_once('.').unwrap();
                    let a: usize = a.parse().unwrap();
                    let b: usize = b.parse().unwrap();
                    if a == 0 && b >= 4 {
                        let g2l_wire = (
                            NodeTileId::from_idx(0),
                            self.intdb.get_wire(&format!("GOUT.{}", b - 4)),
                        );
                        let mut g2l_ins = BTreeSet::new();
                        mux.ins.retain(|&wf| {
                            let wfn = self.intdb.wires.key(wf.1);
                            if wfn.starts_with("GLOBAL") {
                                g2l_ins.insert(wf);
                                false
                            } else {
                                true
                            }
                        });
                        if !g2l_ins.is_empty() {
                            mux.ins.insert(g2l_wire);
                            new_muxes.insert(
                                g2l_wire,
                                MuxInfo {
                                    kind: MuxKind::Plain,
                                    ins: g2l_ins,
                                },
                            );
                        }
                    }
                }
            }
            for (wt, mux) in new_muxes {
                tile_muxes.insert(wt, mux);
            }
        }

        for (nk, node_muxes) in muxes {
            self.intdb.nodes[nk].muxes = node_muxes;
        }

        // TODO: move to some proper place.
        let mut db = Database {
            chips: EntityVec::new(),
            bonds: EntityVec::new(),
            parts: vec![],
            int: self.intdb.clone(),
            tiles: tiledb,
        };
        let chip = db.chips.push(self.chip.clone());
        let bonds = self
            .bonds
            .iter()
            .map(|(name, bond)| {
                let bid = db.bonds.push(bond.clone());
                (name.to_string(), bid)
            })
            .collect();
        let part = prjcombine_siliconblue::db::Part {
            name: self.part.name.to_string(),
            chip,
            bonds,
            speeds: self.part.speeds.iter().map(|x| x.to_string()).collect(),
            temps: self.part.temps.iter().map(|x| x.to_string()).collect(),
        };
        db.parts.push(part);
        db.to_file(format!("databases/{}.zstd", self.part.name))
            .unwrap();
        std::fs::write(
            format!("databases/{}.json", self.part.name),
            JsonValue::from(&db).to_string(),
        )
        .unwrap();
    }
}

fn main() {
    let args = Args::parse();
    for part in parts::PARTS {
        if !args.parts.is_empty() && !args.parts.iter().any(|p| p == part.name) {
            continue;
        }
        let mut ctx = PartContext {
            part,
            chip: Chip {
                kind: part.kind,
                columns: 0,
                cols_bram: BTreeSet::new(),
                col_bio_split: ColId::from_idx(0),
                rows: 0,
                row_mid: RowId::from_idx(0),
                rows_colbuf: vec![],
                cfg_io: BTreeMap::new(),
                io_iob: BTreeMap::new(),
                io_od: BTreeSet::new(),
                extra_nodes: BTreeMap::new(),
            },
            intdb: make_intdb(part.kind),
            sbt: &args.sbt,
            prims: get_prims(part.kind),
            plb_info: vec![],
            empty_runs: BTreeMap::new(),
            bel_info: BTreeMap::new(),
            pkg_bel_info: BTreeMap::new(),
            xlat_col: BTreeMap::new(),
            xlat_row: BTreeMap::new(),
            xlat_io: BTreeMap::new(),
            bonds: BTreeMap::new(),
            extra_wire_names: BTreeMap::new(),
            bel_pins: BTreeMap::new(),
            extra_node_locs: BTreeMap::new(),
            debug: args.debug,
        };

        println!("{}: initializing", ctx.part.name);

        // ctx.intdb.print(&mut std::io::stdout()).unwrap();

        ctx.fill_sites();
        ctx.fill_xlat_rc();
        ctx.fill_bonds();
        ctx.fill_cbsel();
        ctx.fill_bel_pins();
        ctx.fill_io_latch();
        ctx.fill_gbin_fabric();
        ctx.fill_gbin_io();
        ctx.fill_trim();
        ctx.fill_extra_misc();
        ctx.fill_pll();
        ctx.fill_io_i3c();
        ctx.fill_drv();
        ctx.fill_spram();
        ctx.fill_filter();
        ctx.fill_smcclk();

        println!("{}: initial geometry done; starting harvest", ctx.part.name);

        // println!("---- {}", part.name);
        // println!("PLB: {num}", num = ctx.plb_info.len());
        // for info in &ctx.plb_info {
        //     println!("\t{:?}", info.loc);
        // }
        // for (&kind, locs) in &ctx.bel_info {
        //     println!("{kind}: {num}", num = locs.len());
        //     for info in locs {
        //         println!("\t{:?}", info.loc);
        //     }
        // }
        // for (&(pkg, kind), locs) in &ctx.pkg_bel_info {
        //     println!("{pkg} {kind}: {num}", num = locs.len());
        //     for info in locs {
        //         println!("\t{:?}", info.loc);
        //     }
        // }

        // let bs = &ctx.empty_runs[part.packages[0]].bitstream;
        // for (i, bank) in bs.cram.iter().enumerate() {
        //     println!(
        //         "CRAM {i}: {w}×{h}",
        //         w = bank.frame_len,
        //         h = bank.frame_present.len()
        //     );
        // }
        // for (i, bank) in bs.bram.iter().enumerate() {
        //     println!(
        //         "BRAM {i}: {w}×{h}",
        //         w = bank.frame_len,
        //         h = bank.frame_present.len()
        //     );
        // }

        // for (&pkg, bond) in &ctx.bonds {
        //     println!("BOND {pkg}:");
        //     print!("{bond}");
        // }
        // for (&cpin, &io) in &ctx.chip.cfg_io {
        //     println!("CFG {io}: {cpin:?}");
        // }

        ctx.harvest();
    }
}
