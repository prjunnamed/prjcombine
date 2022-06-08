use std::collections::{BTreeMap, BTreeSet, HashMap};
use prjcombine_xilinx_rawdump::{Part, Coord, PkgPin};
use prjcombine_xilinx_geom::{self as geom, DisabledPart, CfgPin, Bond, BondPin};
use prjcombine_xilinx_geom::virtex::{self, GridKind};

use itertools::Itertools;

use crate::grid::{extract_int, find_columns, IntGrid, PreDevice, make_device};

fn get_kind(rd: &Part) -> GridKind {
    match &rd.family[..] {
        "virtex" | "spartan2" => GridKind::Virtex,
        "virtexe" | "spartan2e" => if find_columns(rd, &["MBRAM"]).contains(&6) {
            GridKind::VirtexEM
        } else {
            GridKind::VirtexE
        },
        _ => panic!("unknown family {}", rd.family),
    }
}

fn get_cols_bram(rd: &Part, int: &IntGrid) -> Vec<u32> {
    find_columns(rd, &["LBRAM", "RBRAM", "MBRAM", "MBRAMS2E"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .sorted()
        .collect()
}

fn get_cols_clkv(rd: &Part, int: &IntGrid) -> Vec<(u32, u32)> {
    let cols_clkv: Vec<_> = find_columns(rd, &["LBRAM", "RBRAM", "GCLKV", "CLKV"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .sorted()
        .collect();
    let mut cols_brk: Vec<_> = find_columns(rd, &["GBRKV"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .sorted()
        .collect();
    cols_brk.push(int.cols.len() as u32);
    assert_eq!(cols_clkv.len(), cols_brk.len());
    cols_clkv.into_iter().zip(cols_brk.into_iter()).collect()
}

fn add_disabled_dlls(disabled: &mut BTreeSet<DisabledPart>, rd: &Part) {
    let c = Coord {
        x: rd.width / 2,
        y: 0,
    };
    let t = &rd.tiles[&c];
    if t.kind == "CLKB_2DLL" {
        disabled.insert(DisabledPart::VirtexPrimaryDlls);
    }
}

fn add_disabled_brams(disabled: &mut BTreeSet<DisabledPart>, rd: &Part, int: &IntGrid) {
    for c in find_columns(rd, &["MBRAMS2E"]) {
        disabled.insert(DisabledPart::VirtexBram(int.lookup_column_inter(c)));
    }
}

fn handle_spec_io(rd: &Part, grid: &mut virtex::Grid) {
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, io.coord))
        .collect();
    let mut novref = BTreeSet::new();
    for pins in rd.packages.values() {
        for pin in pins {
            if let Some(ref pad) = pin.pad {
                if pad.starts_with("GCLK") {
                    continue;
                }
                let coord = io_lookup[pad];
                let mut func = &pin.func[..];
                if let Some(pos) = func.find("_L") {
                    func = &func[..pos];
                }
                if func.starts_with("IO_VREF_") {
                    grid.vref.insert(coord);
                } else {
                    novref.insert(coord);
                    let cfg = match func {
                        "IO" => continue,
                        "IO_DIN_D0" => CfgPin::Data(0),
                        "IO_D1" => CfgPin::Data(1),
                        "IO_D2" => CfgPin::Data(2),
                        "IO_D3" => CfgPin::Data(3),
                        "IO_D4" => CfgPin::Data(4),
                        "IO_D5" => CfgPin::Data(5),
                        "IO_D6" => CfgPin::Data(6),
                        "IO_D7" => CfgPin::Data(7),
                        "IO_CS" => CfgPin::CsiB,
                        "IO_INIT" => CfgPin::InitB,
                        "IO_WRITE" => CfgPin::RdWrB,
                        "IO_DOUT_BUSY" => CfgPin::Dout,
                        "IO_IRDY" => {
                            assert_eq!(coord.bel, 3);
                            assert_eq!(coord.row, grid.rows / 2);
                            continue;
                        }
                        "IO_TRDY" => {
                            assert_eq!(coord.bel, 1);
                            assert_eq!(coord.row, grid.rows / 2 - 1);
                            continue;
                        }
                        _ => panic!("UNK FUNC {func} {coord:?}"),
                    };
                    let old = grid.cfg_io.insert(cfg, coord);
                    assert!(old.is_none() || old == Some(coord));
                }
            }
        }
    }
    for c in novref {
        assert!(!grid.vref.contains(&c));
    }
}

fn make_grid(rd: &Part) -> (virtex::Grid, BTreeSet<DisabledPart>) {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &[
        "CENTER",
        "LL",
        "LR",
        "UL",
        "UR",
    ], &[]);
    let kind = get_kind(rd);
    let mut disabled = BTreeSet::new();
    add_disabled_dlls(&mut disabled, rd);
    add_disabled_brams(&mut disabled, rd, &int);
    let mut grid = virtex::Grid {
        kind,
        columns: int.cols.len() as u32,
        cols_bram: get_cols_bram(&rd, &int),
        cols_clkv: get_cols_clkv(&rd, &int),
        rows: int.rows.len() as u32,
        vref: BTreeSet::new(),
        cfg_io: BTreeMap::new(),
    };
    handle_spec_io(rd, &mut grid);
    (grid, disabled)
}

fn make_bond(grid: &virtex::Grid, pins: &[PkgPin]) -> Bond {
    let mut bond_pins = BTreeMap::new();
    let mut io_banks = BTreeMap::new();
    let io_lookup: HashMap<_, _> = grid
        .get_io()
        .into_iter()
        .map(|io| (io.name, (io.coord, io.bank)))
        .collect();
    for pin in pins {
        let bpin = if let Some(ref pad) = pin.pad {
            if pad.starts_with("GCLKPAD") {
                let bank = match &pad[..] {
                    "GCLKPAD0" => 4,
                    "GCLKPAD1" => 5,
                    "GCLKPAD2" => 1,
                    "GCLKPAD3" => 0,
                    _ => panic!("unknown pad {}", pad),
                };
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByBank(bank, 0)
            } else {
                let (coord, bank) = io_lookup[pad];
                assert_eq!(pin.vref_bank, Some(bank));
                let old = io_banks.insert(bank, pin.vcco_bank.unwrap());
                assert!(old.is_none() || old == Some(pin.vcco_bank.unwrap()));
                BondPin::IoByCoord(coord)
            }
        } else if pin.func.starts_with("VCCO_") {
            let bank = pin.func[5..].parse().unwrap();
            BondPin::VccO(bank)
        } else {
            match &pin.func[..] {
                "NC" => BondPin::Nc,
                "GND" => BondPin::Gnd,
                "VCCINT" => BondPin::VccInt,
                "VCCAUX" => BondPin::VccAux,
                "VCCO" => BondPin::VccO(0),
                "TCK" => BondPin::Cfg(CfgPin::Tck),
                "TDI" => BondPin::Cfg(CfgPin::Tdi),
                "TDO" => BondPin::Cfg(CfgPin::Tdo),
                "TMS" => BondPin::Cfg(CfgPin::Tms),
                "CCLK" => BondPin::Cfg(CfgPin::Cclk),
                "DONE" => BondPin::Cfg(CfgPin::Done),
                "PROGRAM" => BondPin::Cfg(CfgPin::ProgB),
                "M0" => BondPin::Cfg(CfgPin::M0),
                "M1" => BondPin::Cfg(CfgPin::M1),
                "M2" => BondPin::Cfg(CfgPin::M2),
                "DXN" => BondPin::Dxn,
                "DXP" => BondPin::Dxp,
                _ => panic!("UNK FUNC {}", pin.func),
            }
        };
        bond_pins.insert(pin.pin.clone(), bpin);
    }
    Bond {
        pins: bond_pins,
        io_banks,
    }
}

pub fn ingest(rd: &Part) -> PreDevice {
    let (grid, disabled) = make_grid(rd);
    let mut bonds = Vec::new();
    for (pkg, pins) in rd.packages.iter() {
        bonds.push((
            pkg.clone(),
            make_bond(&grid, pins),
        ));
    }
    make_device(rd, geom::Grid::Virtex(grid), bonds, disabled)
}
