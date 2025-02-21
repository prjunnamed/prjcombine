use prjcombine_interconnect::grid::{ColId, DieId, RowId};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{chip::Chip, expanded::ExpandedDevice};
use unnamed_entity::{EntityId, EntityVec};

use crate::ExpandedNamedDevice;

fn name_a(
    grid: &Chip,
    prefix: &str,
    suffix: &str,
    col: ColId,
    row: RowId,
    dx: i32,
    dy: i32,
) -> String {
    let cidx = col.to_idx();
    let ridx = grid.rows - row.to_idx() - 1;
    let cidx = u32::try_from(cidx).unwrap().checked_add_signed(dx).unwrap();
    let ridx = u32::try_from(ridx).unwrap().checked_add_signed(dy).unwrap();
    let r = char::from_u32(u32::from('A') + ridx).unwrap();
    let c = char::from_u32(u32::from('A') + cidx).unwrap();
    format!("{prefix}{r}{c}{suffix}")
}

pub fn name_device<'a>(edev: &'a ExpandedDevice<'a>, ndb: &'a NamingDb) -> ExpandedNamedDevice<'a> {
    let egrid = &edev.egrid;
    let grid = edev.chip;
    let mut ngrid = ExpandedGridNaming::new(ndb, egrid);

    let mut col_x = EntityVec::new();
    let mut row_y = EntityVec::new();
    let mut x = 0;
    for col in egrid.die(DieId::from_idx(0)).cols() {
        let ox = x;
        x += if col == grid.col_lio() {
            ndb.tile_widths["L"]
        } else if col == grid.col_rio() {
            ndb.tile_widths["R"]
        } else {
            ndb.tile_widths["C"]
        };
        col_x.push(ox..x);
    }
    let mut y = 0;
    for row in egrid.die(DieId::from_idx(0)).rows() {
        let oy = y;
        y += if row == grid.row_bio() {
            ndb.tile_heights["B"]
        } else if row == grid.row_tio() {
            ndb.tile_heights["T"]
        } else {
            ndb.tile_heights["C"]
        };
        row_y.push(oy..y);
    }
    for die in egrid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                for (layer, node) in &die[(col, row)].nodes {
                    let nloc = (die.die, col, row, layer);
                    let kind = egrid.db.nodes.key(node.kind);
                    let mut naming = kind.to_string();
                    if col == grid.col_lio() + 1 {
                        naming += ".L1";
                    }
                    if row == grid.row_bio() + 1 {
                        naming += ".B1";
                    }
                    if kind.starts_with("CLB") {
                        let nnode = ngrid.name_node(
                            nloc,
                            &naming,
                            [(col_x[col].clone(), row_y[row].clone())],
                        );

                        if col != grid.col_lio() {
                            nnode
                                .coords
                                .push((col_x[col - 1].clone(), row_y[row].clone()));
                        }
                        if col != grid.col_rio() {
                            nnode
                                .coords
                                .push((col_x[col + 1].clone(), row_y[row].clone()));
                        }
                        if row != grid.row_bio() {
                            nnode
                                .coords
                                .push((col_x[col].clone(), row_y[row - 1].clone()));
                        }
                        if row != grid.row_tio() {
                            nnode
                                .coords
                                .push((col_x[col].clone(), row_y[row + 1].clone()));
                        }

                        nnode.add_bel(0, vec![name_a(grid, "", "", col, row, 0, 0)]);

                        let tidx = if kind.starts_with("CLB.B") {
                            let p0 = 1
                                + grid.columns * 2
                                + grid.rows * 2
                                + (grid.col_rio().to_idx() - col.to_idx()) * 2;
                            let p1 = p0 + 1;
                            nnode.add_bel(1, vec![format!("PAD{p1}")]);
                            nnode.add_bel(2, vec![format!("PAD{p0}")]);
                            if kind.starts_with("CLB.BL") {
                                let p2 = p0 + 2;
                                let p3 = p0 + 3;
                                nnode.add_bel(3, vec![format!("PAD{p3}")]);
                                nnode.add_bel(4, vec![format!("PAD{p2}")]);
                                5
                            } else if kind.starts_with("CLB.BR") {
                                let p2 = p0 - 2;
                                let p3 = p0 - 1;
                                nnode.add_bel(3, vec![format!("PAD{p2}")]);
                                nnode.add_bel(4, vec![format!("PAD{p3}")]);
                                5
                            } else {
                                3
                            }
                        } else if kind.starts_with("CLB.T") {
                            let p0 = 1 + col.to_idx() * 2;
                            let p1 = p0 + 1;
                            nnode.add_bel(1, vec![format!("PAD{p0}")]);
                            nnode.add_bel(2, vec![format!("PAD{p1}")]);
                            if kind.starts_with("CLB.TL") {
                                let p0 = grid.columns * 4 + grid.rows * 4 - 1;
                                let p1 = p0 + 1;
                                nnode.add_bel(3, vec![format!("PAD{p1}")]);
                                nnode.add_bel(4, vec![format!("PAD{p0}")]);
                                5
                            } else if kind.starts_with("CLB.TR") {
                                let p2 = p0 + 2;
                                let p3 = p0 + 3;
                                nnode.add_bel(3, vec![format!("PAD{p2}")]);
                                nnode.add_bel(4, vec![format!("PAD{p3}")]);
                                5
                            } else {
                                3
                            }
                        } else if kind.starts_with("CLB.L") {
                            let p0 = 1 + grid.columns * 4 + grid.rows * 2 + row.to_idx() * 2;
                            let p1 = p0 + 1;
                            nnode.add_bel(1, vec![format!("PAD{p1}")]);
                            nnode.add_bel(2, vec![format!("PAD{p0}")]);
                            3
                        } else if kind.starts_with("CLB.R") {
                            let p0 =
                                1 + grid.columns * 2 + (grid.row_tio().to_idx() - row.to_idx()) * 2;
                            let p1 = p0 + 1;
                            nnode.add_bel(1, vec![format!("PAD{p0}")]);
                            nnode.add_bel(2, vec![format!("PAD{p1}")]);
                            3
                        } else {
                            1
                        };

                        let suf2 = if row == grid.row_tio() { ".1" } else { ".2" };
                        nnode.add_bel(tidx, vec![name_a(grid, "TBUF.", ".1", col, row, 0, 1)]);
                        nnode.add_bel(tidx + 1, vec![name_a(grid, "TBUF.", suf2, col, row, 0, 0)]);
                        if col == grid.col_rio() {
                            nnode.add_bel(
                                tidx + 2,
                                vec![name_a(grid, "TBUF.", ".1", col, row, 1, 1)],
                            );
                            nnode.add_bel(
                                tidx + 3,
                                vec![name_a(grid, "TBUF.", suf2, col, row, 1, 0)],
                            );
                            nnode
                                .add_bel(tidx + 4, vec![name_a(grid, "PU.", ".1", col, row, 1, 1)]);
                            nnode
                                .add_bel(tidx + 5, vec![name_a(grid, "PU.", suf2, col, row, 1, 0)]);
                        } else if col == grid.col_lio() {
                            nnode
                                .add_bel(tidx + 2, vec![name_a(grid, "PU.", ".1", col, row, 0, 1)]);
                            nnode
                                .add_bel(tidx + 3, vec![name_a(grid, "PU.", suf2, col, row, 0, 0)]);
                        }

                        if kind.starts_with("CLB.TL") {
                            nnode.add_bel(9, vec!["TCLKIN".into()]);
                            nnode.add_bel(10, vec!["GCLK".into()]);
                        }
                        if kind.starts_with("CLB.BR") {
                            nnode.add_bel(11, vec!["BCLKIN".into()]);
                            nnode.add_bel(12, vec!["ACLK".into()]);
                            nnode.add_bel(13, vec!["OSC".into()]);
                        }
                    } else if kind.starts_with("LLH") {
                        ngrid.name_node(nloc, kind, [(col_x[col].clone(), row_y[row].clone())]);
                    } else if kind.starts_with("LLV") {
                        ngrid.name_node(nloc, kind, [(col_x[col].clone(), row_y[row - 1].clone())]);
                    } else {
                        panic!("ummmm {kind}?");
                    }
                }
            }
        }
    }

    ExpandedNamedDevice {
        edev,
        ngrid,
        grid,
        col_x,
        row_y,
        clk_x: None,
        clk_y: None,
    }
}
