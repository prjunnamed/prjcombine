use prjcombine_rawdump::Part;
use prjcombine_xc4k::{Grid, GridKind, SharedCfgPin};
use std::collections::{BTreeMap, HashMap};

use prjcombine_rdgrid::extract_int;

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

fn handle_spec_io(rd: &Part, grid: &mut Grid) {
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
        columns: int.cols.len() as u32,
        rows: int.rows.len() as u32,
        cfg_io: BTreeMap::new(),
    };
    handle_spec_io(rd, &mut grid);
    grid
}
