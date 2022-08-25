use prjcombine_xilinx_geom::xc4k::{self, GridKind};
use prjcombine_xilinx_geom::{self as geom, int, Bond, BondPin, CfgPin};
use prjcombine_xilinx_rawdump::{Part, PkgPin};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::grid::{extract_int, make_device, PreDevice};

fn get_kind(rd: &Part) -> GridKind {
    match &rd.family[..] {
        "xc4000e" => GridKind::Xc4000E,
        "xc4000ex" => GridKind::Xc4000Ex,
        "xc4000xla" => GridKind::Xc4000Xla,
        "xc4000xv" => GridKind::Xc4000Xv,
        "spartanxl" => GridKind::SpartanXl,
        _ => panic!("unknown family {}", rd.family),
    }
}

fn handle_spec_io(rd: &Part, grid: &mut xc4k::Grid) {
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    for pins in rd.packages.values() {
        for pin in pins {
            if let Some(ref pad) = pin.pad {
                if let Some(&io) = io_lookup.get(pad) {
                    let cfg = match &pin.func[..] {
                        "IO" => continue,
                        "IO_TCK" => CfgPin::Tck,
                        "IO_TDI" => CfgPin::Tdi,
                        "IO_TMS" => CfgPin::Tms,
                        _ => {
                            println!("UNK FUNC {}", pin.func);
                            continue;
                        }
                    };
                    let old = grid.cfg_io.insert(cfg, io);
                    assert!(old.is_none() || old == Some(io));
                }
            }
        }
    }
}

fn make_grid(rd: &Part) -> xc4k::Grid {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &["CENTER", "LL", "LR", "UL", "UR"], &[]);
    let kind = get_kind(rd);
    let mut grid = xc4k::Grid {
        kind,
        columns: int.cols.len() as u32,
        rows: int.rows.len() as u32,
        cfg_io: BTreeMap::new(),
    };
    handle_spec_io(rd, &mut grid);
    grid
}

fn make_bond(grid: &xc4k::Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if let Some(&io) = io_lookup.get(pad) {
                BondPin::IoByCoord(io)
            } else {
                match &pad[..] {
                    "TDO" => BondPin::Cfg(CfgPin::Tdo),
                    "MD0" => BondPin::Cfg(CfgPin::M0),
                    "MD1" => BondPin::Cfg(CfgPin::M1),
                    "MD2" => BondPin::Cfg(CfgPin::M2),
                    _ => {
                        println!("UNK PAD {}", pad);
                        continue;
                    }
                }
            }
        } else {
            match &pin.func[..] {
                "NC" | "N.C." => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCC" => BondPin::VccO(0),
                "VCCINT" => BondPin::VccInt,
                "CCLK" => BondPin::Cfg(CfgPin::Cclk),
                "DONE" => BondPin::Cfg(CfgPin::Done),
                "/PROG" | "/PROGRAM" => BondPin::Cfg(CfgPin::ProgB),
                "MODE" | "M0" => BondPin::Cfg(CfgPin::M0),
                "M1" => BondPin::Cfg(CfgPin::M1),
                "M2" => BondPin::Cfg(CfgPin::M2),
                "M2_OPT" => BondPin::Cfg(CfgPin::M2),
                "/PWRDOWN" | "LPWRB" => BondPin::Cfg(CfgPin::PwrdwnB),
                _ => {
                    println!("UNK FUNC {}", pin.func);
                    continue;
                }
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks: BTreeMap::new(),
    }
}

pub fn ingest(rd: &Part) -> (PreDevice, Option<int::IntDb>) {
    let grid = make_grid(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((pkg.clone(), make_bond(&grid, pins)));
    }
    (
        make_device(rd, geom::Grid::Xc4k(grid), bonds, BTreeSet::new()),
        None,
    )
}
