use prjcombine_interconnect::grid::DieIdExt;
use prjcombine_re_xilinx_rawdump::{Part, TkSiteSlot};
use prjcombine_xc2000::{
    chip::{Chip, ChipKind, SharedCfgPad},
    xc4000::bslots,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

use prjcombine_re_xilinx_rd2db_grid::{IntGrid, extract_int};

fn get_kind(rd: &Part) -> ChipKind {
    match &rd.family[..] {
        "xc4000e" => ChipKind::Xc4000E,
        "xc4000ex" => ChipKind::Xc4000Ex,
        "xc4000xla" => ChipKind::Xc4000Xla,
        "xc4000xv" => ChipKind::Xc4000Xv,
        "spartanxl" => ChipKind::SpartanXl,
        _ => panic!("unknown family {}", rd.family),
    }
}

fn handle_spec_io(rd: &Part, chip: &mut Chip, int: &IntGrid) {
    let mut io_lookup = HashMap::new();
    for (&crd, tile) in &rd.tiles {
        let tk = &rd.tile_kinds[tile.kind];
        for (k, v) in &tile.sites {
            if let &TkSiteSlot::Indexed(sn, idx) = tk.sites.key(k)
                && rd.slot_kinds[sn] == "IOB"
            {
                io_lookup.insert(
                    v.clone(),
                    chip.get_io_crd(
                        Chip::DIE
                            .cell(
                                int.lookup_column(crd.x.into()),
                                int.lookup_row(crd.y.into()),
                            )
                            .bel(bslots::IO[idx as usize - 1]),
                    ),
                );
            }
        }
    }

    for pins in rd.packages.values() {
        for pin in pins {
            if let Some(ref pad) = pin.pad
                && let Some(&io) = io_lookup.get(pad)
            {
                let cfg = match &pin.func[..] {
                    "IO" => continue,
                    "IO_TCK" => SharedCfgPad::Tck,
                    "IO_TDI" => SharedCfgPad::Tdi,
                    "IO_TMS" => SharedCfgPad::Tms,
                    _ => {
                        println!("UNK FUNC {}", pin.func);
                        continue;
                    }
                };
                let old = chip.cfg_io.insert(cfg, io);
                assert!(old.is_none() || old == Some(io));
            }
        }
    }
}

pub fn make_grid(rd: &Part) -> Chip {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &["CENTER", "LL", "LR", "UL", "UR"], &[]);
    let kind = get_kind(rd);
    let mut chip = Chip {
        kind,
        columns: int.cols.len(),
        rows: int.rows.len(),
        cfg_io: BTreeMap::new(),
        is_buff_large: rd.tile_kinds.contains_key("RHVBRK"),
        is_small: false,
        cols_bidi: Default::default(),
        rows_bidi: Default::default(),
        unbonded_io: BTreeSet::new(),
    };
    handle_spec_io(rd, &mut chip, &int);
    chip
}
