use std::collections::BTreeSet;

use prjcombine_re_xilinx_rawdump::Part;
use prjcombine_xc2000::chip::{Chip, ChipKind};

use prjcombine_re_xilinx_rd2db_grid::extract_int;

pub fn make_grid(rd: &Part) -> Chip {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &["CENTER", "LL", "LR", "UL", "UR"], &[]);
    Chip {
        kind: ChipKind::Xc5200,
        columns: int.cols.len(),
        rows: int.rows.len(),
        cfg_io: Default::default(),
        is_small: false,
        is_buff_large: false,
        cols_bidi: Default::default(),
        rows_bidi: Default::default(),
        unbonded_io: BTreeSet::new(),
    }
}
