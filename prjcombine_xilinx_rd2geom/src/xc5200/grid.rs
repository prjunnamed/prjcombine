use prjcombine_rawdump::Part;
use prjcombine_xilinx_geom::xc5200::Grid;

use crate::grid::extract_int;

pub fn make_grid(rd: &Part) -> Grid {
    // This list of int tiles is incomplete, but suffices for the purpose of grid determination
    let int = extract_int(rd, &["CENTER", "LL", "LR", "UL", "UR"], &[]);
    Grid {
        columns: int.cols.len(),
        rows: int.rows.len(),
    }
}