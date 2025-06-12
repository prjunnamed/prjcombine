use prjcombine_interconnect::grid::{CellCoord, ColId, DieId, RowId};
use prjcombine_re_xilinx_xact_naming::{db::NamingDb, grid::ExpandedGridNaming};
use prjcombine_xc2000::{bels::xc2000 as bels, chip::Chip, expanded::ExpandedDevice};
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
        x += if col == grid.col_w() {
            ndb.tile_widths["L"]
        } else if col == grid.col_e() {
            ndb.tile_widths["R"]
        } else {
            ndb.tile_widths["C"]
        };
        col_x.push(ox..x);
    }
    let mut y = 0;
    for row in egrid.die(DieId::from_idx(0)).rows() {
        let oy = y;
        y += if row == grid.row_s() {
            ndb.tile_heights["B"]
        } else if row == grid.row_n() {
            ndb.tile_heights["T"]
        } else {
            ndb.tile_heights["C"]
        };
        row_y.push(oy..y);
    }
    for die in egrid.dies() {
        for col in die.cols() {
            for row in die.rows() {
                let cell = CellCoord::new(die.die, col, row);
                for (tslot, tile) in &die[(col, row)].tiles {
                    let tcrd = cell.tile(tslot);
                    let kind = egrid.db.tile_classes.key(tile.class);
                    let mut naming = kind.to_string();
                    if col == grid.col_w() + 1 {
                        naming += ".L1";
                    }
                    if row == grid.row_s() + 1 {
                        naming += ".B1";
                    }
                    if kind.starts_with("CLB") {
                        let nnode = ngrid.name_node(
                            tcrd,
                            &naming,
                            [(col_x[col].clone(), row_y[row].clone())],
                        );

                        if col != grid.col_w() {
                            nnode
                                .coords
                                .push((col_x[col - 1].clone(), row_y[row].clone()));
                        }
                        if col != grid.col_e() {
                            nnode
                                .coords
                                .push((col_x[col + 1].clone(), row_y[row].clone()));
                        }
                        if row != grid.row_s() {
                            nnode
                                .coords
                                .push((col_x[col].clone(), row_y[row - 1].clone()));
                        }
                        if row != grid.row_n() {
                            nnode
                                .coords
                                .push((col_x[col].clone(), row_y[row + 1].clone()));
                        }

                        nnode.add_bel(bels::CLB, vec![name_a(grid, "", "", col, row, 0, 0)]);

                        if kind.starts_with("CLB.B") {
                            let p0 = 1
                                + grid.columns * 2
                                + grid.rows * 2
                                + (grid.col_e().to_idx() - col.to_idx()) * 2;
                            let p1 = p0 + 1;
                            nnode.add_bel(bels::IO_S0, vec![format!("PAD{p1}")]);
                            nnode.add_bel(bels::IO_S1, vec![format!("PAD{p0}")]);
                            if kind.starts_with("CLB.BL") {
                                let p2 = p0 + 2;
                                let p3 = p0 + 3;
                                nnode.add_bel(bels::IO_W0, vec![format!("PAD{p3}")]);
                                nnode.add_bel(bels::IO_W1, vec![format!("PAD{p2}")]);
                            } else if kind.starts_with("CLB.BR") {
                                let p2 = p0 - 2;
                                let p3 = p0 - 1;
                                nnode.add_bel(bels::IO_E0, vec![format!("PAD{p2}")]);
                                nnode.add_bel(bels::IO_E1, vec![format!("PAD{p3}")]);
                            }
                        } else if kind.starts_with("CLB.T") {
                            let p0 = 1 + col.to_idx() * 2;
                            let p1 = p0 + 1;
                            nnode.add_bel(bels::IO_N0, vec![format!("PAD{p0}")]);
                            nnode.add_bel(bels::IO_N1, vec![format!("PAD{p1}")]);
                            if kind.starts_with("CLB.TL") {
                                let p0 = grid.columns * 4 + grid.rows * 4 - 1;
                                let p1 = p0 + 1;
                                nnode.add_bel(bels::IO_W0, vec![format!("PAD{p1}")]);
                                nnode.add_bel(bels::IO_W1, vec![format!("PAD{p0}")]);
                            } else if kind.starts_with("CLB.TR") {
                                let p2 = p0 + 2;
                                let p3 = p0 + 3;
                                nnode.add_bel(bels::IO_E0, vec![format!("PAD{p2}")]);
                                nnode.add_bel(bels::IO_E1, vec![format!("PAD{p3}")]);
                            }
                        } else if kind.starts_with("CLB.L") {
                            let p0 = 1 + grid.columns * 4 + grid.rows * 2 + row.to_idx() * 2;
                            let p1 = p0 + 1;
                            nnode.add_bel(bels::IO_W0, vec![format!("PAD{p1}")]);
                            nnode.add_bel(bels::IO_W1, vec![format!("PAD{p0}")]);
                        } else if kind.starts_with("CLB.R") {
                            let p0 =
                                1 + grid.columns * 2 + (grid.row_n().to_idx() - row.to_idx()) * 2;
                            let p1 = p0 + 1;
                            nnode.add_bel(bels::IO_E0, vec![format!("PAD{p0}")]);
                            nnode.add_bel(bels::IO_E1, vec![format!("PAD{p1}")]);
                        }

                        let suf2 = if row == grid.row_n() { ".1" } else { ".2" };
                        nnode.add_bel(
                            bels::TBUF0,
                            vec![name_a(grid, "TBUF.", ".1", col, row, 0, 1)],
                        );
                        nnode.add_bel(
                            bels::TBUF1,
                            vec![name_a(grid, "TBUF.", suf2, col, row, 0, 0)],
                        );
                        if col == grid.col_e() {
                            nnode.add_bel(
                                bels::TBUF0_E,
                                vec![name_a(grid, "TBUF.", ".1", col, row, 1, 1)],
                            );
                            nnode.add_bel(
                                bels::TBUF1_E,
                                vec![name_a(grid, "TBUF.", suf2, col, row, 1, 0)],
                            );
                            nnode.add_bel(
                                bels::PULLUP_TBUF0,
                                vec![name_a(grid, "PU.", ".1", col, row, 1, 1)],
                            );
                            nnode.add_bel(
                                bels::PULLUP_TBUF1,
                                vec![name_a(grid, "PU.", suf2, col, row, 1, 0)],
                            );
                        } else if col == grid.col_w() {
                            nnode.add_bel(
                                bels::PULLUP_TBUF0,
                                vec![name_a(grid, "PU.", ".1", col, row, 0, 1)],
                            );
                            nnode.add_bel(
                                bels::PULLUP_TBUF1,
                                vec![name_a(grid, "PU.", suf2, col, row, 0, 0)],
                            );
                        }

                        if kind.starts_with("CLB.TL") {
                            nnode.add_bel(bels::CLKIOB, vec!["TCLKIN".into()]);
                            nnode.add_bel(bels::BUFG, vec!["GCLK".into()]);
                        }
                        if kind.starts_with("CLB.BR") {
                            nnode.add_bel(bels::CLKIOB, vec!["BCLKIN".into()]);
                            nnode.add_bel(bels::BUFG, vec!["ACLK".into()]);
                            nnode.add_bel(bels::OSC, vec!["OSC".into()]);
                        }
                    } else if kind.starts_with("LLH") {
                        ngrid.name_node(tcrd, kind, [(col_x[col].clone(), row_y[row].clone())]);
                    } else if kind.starts_with("LLV") {
                        ngrid.name_node(tcrd, kind, [(col_x[col].clone(), row_y[row - 1].clone())]);
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
        chip: grid,
        col_x,
        row_y,
        clk_x: None,
        clk_y: None,
    }
}
