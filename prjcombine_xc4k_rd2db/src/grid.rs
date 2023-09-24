use prjcombine_rawdump::{Part, TkSiteSlot};
use prjcombine_xc4k::grid::{Grid, GridKind, IoCoord, SharedCfgPin, TileIobId};
use std::collections::{BTreeMap, HashMap};
use unnamed_entity::EntityId;

use prjcombine_rdgrid::{extract_int, IntGrid};

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
                            iob: TileIobId::from_idx(idx as usize - 1),
                        },
                    );
                }
            }
        }
    }

    for pins in rd.packages.values() {
        for pin in pins {
            if let Some(ref pad) = pin.pad {
                if let Some(&io) = io_lookup.get(pad) {
                    let cfg = match &pin.func[..] {
                        "IO" => continue,
                        "IO_TCK" => SharedCfgPin::Tck,
                        "IO_TDI" => SharedCfgPin::Tdi,
                        "IO_TMS" => SharedCfgPin::Tms,
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

pub fn make_grid(rd: &Part) -> Grid {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &["CENTER", "LL", "LR", "UL", "UR"], &[]);
    let kind = get_kind(rd);
    let mut grid = Grid {
        kind,
        columns: int.cols.len(),
        rows: int.rows.len(),
        cfg_io: BTreeMap::new(),
        is_buff_large: rd.tile_kinds.contains_key("RHVBRK"),
    };
    handle_spec_io(rd, &mut grid, &int);
    grid
}
