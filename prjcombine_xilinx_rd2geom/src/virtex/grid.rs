use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::virtex::{Grid, GridKind};
use prjcombine_xilinx_geom::{
    CfgPin, ColId, DisabledPart,
};
use prjcombine_xilinx_rawdump::{Coord, Part};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::grid::{extract_int, find_columns, IntGrid};

fn get_kind(rd: &Part) -> GridKind {
    match &rd.family[..] {
        "virtex" | "spartan2" => GridKind::Virtex,
        "virtexe" | "spartan2e" => {
            if find_columns(rd, &["MBRAM"]).contains(&6) {
                GridKind::VirtexEM
            } else {
                GridKind::VirtexE
            }
        }
        _ => panic!("unknown family {}", rd.family),
    }
}

fn get_cols_bram(rd: &Part, int: &IntGrid) -> BTreeSet<ColId> {
    find_columns(rd, &["LBRAM", "RBRAM", "MBRAM", "MBRAMS2E"])
        .into_iter()
        .map(|r| int.lookup_column(r))
        .collect()
}

fn get_cols_clkv(rd: &Part, int: &IntGrid) -> Vec<(ColId, ColId, ColId)> {
    let mut cols_clkv: BTreeSet<_> = find_columns(rd, &["GCLKV", "CLKV"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .collect();
    cols_clkv.insert(int.cols.first_id().unwrap() + 1);
    cols_clkv.insert(int.cols.last_id().unwrap() - 1);
    let mut cols_brk: BTreeSet<_> = find_columns(rd, &["GBRKV"])
        .into_iter()
        .map(|r| int.lookup_column_inter(r))
        .collect();
    let mut cols_brk_l = cols_brk.clone();
    cols_brk_l.insert(int.cols.first_id().unwrap());
    cols_brk.insert(int.cols.next_id());
    assert_eq!(cols_clkv.len(), cols_brk.len());
    assert_eq!(cols_clkv.len(), cols_brk_l.len());
    cols_clkv.into_iter().zip(cols_brk_l.into_iter()).zip(cols_brk.into_iter()).map(|((a, b), c)| (a, b, c)).collect()
}

fn add_disabled_dlls(disabled: &mut BTreeSet<DisabledPart>, rd: &Part) {
    let c = Coord {
        x: rd.width / 2,
        y: 0,
    };
    let t = &rd.tiles[&c];
    if rd.tile_kinds.key(t.kind) == "CLKB_2DLL" {
        disabled.insert(DisabledPart::VirtexPrimaryDlls);
    }
}

fn add_disabled_brams(disabled: &mut BTreeSet<DisabledPart>, rd: &Part, int: &IntGrid) {
    for c in find_columns(rd, &["MBRAMS2E"]) {
        disabled.insert(DisabledPart::VirtexBram(int.lookup_column(c)));
    }
}

fn handle_spec_io(rd: &Part, grid: &mut Grid) {
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
                            assert_eq!(coord.bel.to_idx(), 3);
                            assert_eq!(coord.row, grid.row_mid());
                            continue;
                        }
                        "IO_TRDY" => {
                            assert_eq!(coord.bel.to_idx(), 1);
                            assert_eq!(coord.row, grid.row_mid() - 1);
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

pub fn make_grid(rd: &Part) -> (Grid, BTreeSet<DisabledPart>) {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(
        rd,
        &[
            "CENTER", "LBRAM", "RBRAM", "MBRAM", "MBRAMS2E", "LL", "LR", "UL", "UR",
        ],
        &[],
    );
    let kind = get_kind(rd);
    let mut disabled = BTreeSet::new();
    add_disabled_dlls(&mut disabled, rd);
    add_disabled_brams(&mut disabled, rd, &int);
    let mut grid = Grid {
        kind,
        columns: int.cols.len(),
        cols_bram: get_cols_bram(rd, &int),
        cols_clkv: get_cols_clkv(rd, &int),
        rows: int.rows.len(),
        vref: BTreeSet::new(),
        cfg_io: BTreeMap::new(),
    };
    handle_spec_io(rd, &mut grid);
    (grid, disabled)
}
