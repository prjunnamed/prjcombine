use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_entity::EntityId;
use prjcombine_int::grid::ColId;
use prjcombine_rawdump::{Coord, Part, TkSiteSlot};
use prjcombine_virtex::grid::{DisabledPart, Grid, GridKind, IoCoord, SharedCfgPin, TileIobId};

use prjcombine_rdgrid::{extract_int, find_columns, IntGrid};

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
    cols_clkv
        .into_iter()
        .zip(cols_brk_l.into_iter())
        .zip(cols_brk.into_iter())
        .map(|((a, b), c)| (a, b, c))
        .collect()
}

fn add_disabled_dlls(disabled: &mut BTreeSet<DisabledPart>, rd: &Part) {
    let c = Coord {
        x: rd.width / 2,
        y: 0,
    };
    let t = &rd.tiles[&c];
    if rd.tile_kinds.key(t.kind) == "CLKB_2DLL" {
        disabled.insert(DisabledPart::PrimaryDlls);
    }
}

fn add_disabled_brams(disabled: &mut BTreeSet<DisabledPart>, rd: &Part, int: &IntGrid) {
    for c in find_columns(rd, &["MBRAMS2E"]) {
        disabled.insert(DisabledPart::Bram(int.lookup_column(c)));
    }
}

fn handle_spec_io(rd: &Part, grid: &mut Grid, int: &IntGrid) {
    let mut io_lookup = HashMap::new();
    for (&crd, tile) in &rd.tiles {
        let tk = &rd.tile_kinds[tile.kind];
        for (k, v) in &tile.sites {
            if let &TkSiteSlot::Indexed(sn, idx) = tk.sites.key(k) {
                if rd.slot_kinds[sn] == "IOB" {
                    io_lookup.insert(
                        v.clone(),
                        IoCoord {
                            col: int.lookup_column(crd.x.into()),
                            row: int.lookup_row(crd.y.into()),
                            iob: TileIobId::from_idx(idx as usize),
                        },
                    );
                }
            }
        }
    }
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
                        "IO_DIN_D0" => SharedCfgPin::Data(0),
                        "IO_D1" => SharedCfgPin::Data(1),
                        "IO_D2" => SharedCfgPin::Data(2),
                        "IO_D3" => SharedCfgPin::Data(3),
                        "IO_D4" => SharedCfgPin::Data(4),
                        "IO_D5" => SharedCfgPin::Data(5),
                        "IO_D6" => SharedCfgPin::Data(6),
                        "IO_D7" => SharedCfgPin::Data(7),
                        "IO_CS" => SharedCfgPin::CsB,
                        "IO_INIT" => SharedCfgPin::InitB,
                        "IO_WRITE" => SharedCfgPin::RdWrB,
                        "IO_DOUT_BUSY" => SharedCfgPin::Dout,
                        "IO_IRDY" => {
                            assert_eq!(coord.iob.to_idx(), 3);
                            assert_eq!(coord.row, grid.row_mid());
                            continue;
                        }
                        "IO_TRDY" => {
                            assert_eq!(coord.iob.to_idx(), 1);
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
    handle_spec_io(rd, &mut grid, &int);
    (grid, disabled)
}
