use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_interconnect::grid::{ColId, DieId, EdgeIoCoord};
use prjcombine_re_xilinx_rawdump::{Coord, Part, TkSiteSlot};
use prjcombine_virtex::{
    bels,
    chip::{Chip, ChipKind, DisabledPart, SharedCfgPad},
};
use unnamed_entity::EntityId;

use prjcombine_re_xilinx_rd2db_grid::{IntGrid, extract_int, find_columns};

fn get_kind(rd: &Part) -> ChipKind {
    match &rd.family[..] {
        "virtex" | "spartan2" => ChipKind::Virtex,
        "virtexe" | "spartan2e" => {
            if find_columns(rd, &["MBRAM"]).contains(&6) {
                ChipKind::VirtexEM
            } else {
                ChipKind::VirtexE
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
        .zip(cols_brk_l)
        .zip(cols_brk)
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

fn handle_spec_io(rd: &Part, chip: &mut Chip, int: &IntGrid) {
    let mut io_lookup = HashMap::new();
    for (&crd, tile) in &rd.tiles {
        let tk = &rd.tile_kinds[tile.kind];
        for (k, v) in &tile.sites {
            if let &TkSiteSlot::Indexed(sn, idx) = tk.sites.key(k) {
                if rd.slot_kinds[sn] == "IOB" {
                    let col = int.lookup_column(crd.x.into());
                    let row = int.lookup_row(crd.y.into());
                    let io =
                        chip.get_io_crd((DieId::from_idx(0), (col, row), bels::IO[idx as usize]));
                    io_lookup.insert(v.clone(), io);
                }
            }
        }
    }
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
                    // pass
                } else {
                    let cfg = match func {
                        "IO" => continue,
                        "IO_DIN_D0" => SharedCfgPad::Data(0),
                        "IO_D1" => SharedCfgPad::Data(1),
                        "IO_D2" => SharedCfgPad::Data(2),
                        "IO_D3" => SharedCfgPad::Data(3),
                        "IO_D4" => SharedCfgPad::Data(4),
                        "IO_D5" => SharedCfgPad::Data(5),
                        "IO_D6" => SharedCfgPad::Data(6),
                        "IO_D7" => SharedCfgPad::Data(7),
                        "IO_CS" => SharedCfgPad::CsB,
                        "IO_INIT" => SharedCfgPad::InitB,
                        "IO_WRITE" => SharedCfgPad::RdWrB,
                        "IO_DOUT_BUSY" => SharedCfgPad::Dout,
                        "IO_IRDY" => {
                            match coord {
                                EdgeIoCoord::W(row, iob) | EdgeIoCoord::E(row, iob) => {
                                    assert_eq!(iob.to_idx(), 3);
                                    assert_eq!(row, chip.row_mid());
                                }
                                _ => unreachable!(),
                            }
                            continue;
                        }
                        "IO_TRDY" => {
                            match coord {
                                EdgeIoCoord::W(row, iob) | EdgeIoCoord::E(row, iob) => {
                                    assert_eq!(iob.to_idx(), 1);
                                    assert_eq!(row, chip.row_mid() - 1);
                                }
                                _ => unreachable!(),
                            }
                            continue;
                        }
                        _ => panic!("UNK FUNC {func} {coord:?}"),
                    };
                    let old = chip.cfg_io.insert(cfg, coord);
                    assert!(old.is_none() || old == Some(coord));
                }
            }
        }
    }
}

pub fn make_grid(rd: &Part) -> (Chip, BTreeSet<DisabledPart>) {
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
    let mut grid = Chip {
        kind,
        columns: int.cols.len(),
        cols_bram: get_cols_bram(rd, &int),
        cols_clkv: get_cols_clkv(rd, &int),
        rows: int.rows.len(),
        cfg_io: BTreeMap::new(),
    };
    handle_spec_io(rd, &mut grid, &int);
    (grid, disabled)
}
